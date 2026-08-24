mod common;

use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use common::{McpClient, call_tool, initialize, spawn_daemon, write_roles};
use serde_json::{Value, json};
use workflow_core::{ArchitecturePlan, ContentDigest, PlannedTask, Requirement, TaskId};

type Script = Box<dyn Fn(&Value) -> Value + Send>;

struct FakeRoles {
    _requests: mpsc::Receiver<Value>,
    base_url: String,
}

fn start_fake_roles(scripts: Vec<Script>) -> FakeRoles {
    let listener = TcpListener::bind("127.0.0.1:0").expect("fake roles bind");
    let address = listener.local_addr().expect("fake roles address");
    let (sender, requests) = mpsc::channel();
    thread::spawn(move || {
        for script in scripts {
            let Ok((stream, _)) = listener.accept() else {
                break;
            };
            let Ok(mut writer) = stream.try_clone() else {
                continue;
            };
            let Some(body) = capture_request(&mut BufReader::new(stream)) else {
                continue;
            };
            let content = script(&body).to_string();
            let payload = json!({
                "choices": [{"finish_reason": "stop", "index": 0,
                    "message": {"content": content, "role": "assistant"}}],
            })
            .to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                payload.len(),
                payload
            );
            let _ = writer.write_all(response.as_bytes());
            let _ = writer.flush();
            let _ = sender.send(body);
        }
    });
    FakeRoles {
        _requests: requests,
        base_url: format!("http://{address}/v1"),
    }
}

fn capture_request(reader: &mut BufReader<TcpStream>) -> Option<Value> {
    let mut request_line = String::new();
    reader.read_line(&mut request_line).ok()?;
    let mut content_length = 0_usize;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header).ok()?;
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        if let Some(rest) = header.to_ascii_lowercase().strip_prefix("content-length:") {
            content_length = rest.trim().parse().ok()?;
        }
    }
    let mut body = vec![0_u8; content_length];
    reader.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

fn consult_request(body: &Value) -> Value {
    body["messages"][1]["content"]
        .as_str()
        .and_then(|text| serde_json::from_str::<Value>(text).ok())
        .unwrap_or_else(|| json!({}))
}

fn requirement_decisions(request: &Value) -> Vec<Value> {
    request["requirementIds"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|id| {
            json!({
                "evidence_ids": request["evidenceIds"].as_array().cloned().unwrap_or_default(),
                "requirement_id": id,
                "status": "satisfied",
            })
        })
        .collect()
}

fn review_script(role: &str) -> Script {
    let role = role.to_owned();
    Box::new(move |body| {
        let request = consult_request(body);
        json!({
            "candidate_digest": request["candidateDigest"],
            "decision": "approved",
            "findings": [],
            "repair_target": null,
            "requirements": requirement_decisions(&request),
            "role": role,
        })
    })
}

fn arbiter_script(approved: bool) -> Script {
    Box::new(move |body| {
        let request = consult_request(body);
        let findings = if approved {
            Vec::new()
        } else {
            vec![json!({
                "evidence_ids": request["evidenceIds"].as_array().cloned().unwrap_or_default(),
                "severity": "medium",
                "summary": "certification: the candidate needs one repair pass",
            })]
        };
        json!({
            "candidate_digest": request["candidateDigest"],
            "decision": if approved { "approved" } else { "rejected" },
            "findings": findings,
            "repair_target": if approved { Value::Null } else { json!("execution") },
            "requirements": requirement_decisions(&request),
        })
    })
}

fn advisory_script() -> Script {
    Box::new(|_| {
        json!({
            "open_questions": [],
            "points": ["certification advisory point"],
            "risks": [],
            "summary": "certification advisory answer",
        })
    })
}

struct Repository {
    _directory: tempfile::TempDir,
    path: PathBuf,
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .status()
        .expect("git runs");
    assert!(status.success(), "git {:?} failed in {dir:?}", args);
}

fn seed_repository(name: &str) -> Repository {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(name);
    fs::create_dir(&path).unwrap();
    git(&path, &["init"]);
    git(&path, &["config", "user.email", "cert@example.invalid"]);
    git(&path, &["config", "user.name", "Certification"]);
    git(&path, &["config", "core.autocrlf", "false"]);
    fs::write(path.join("README.md"), "certification repository\n").unwrap();
    fs::write(path.join("app.txt"), "base\n").unwrap();
    git(&path, &["add", "."]);
    git(&path, &["commit", "-m", "base"]);
    Repository {
        _directory: directory,
        path,
    }
}

fn plan_value(request_digest: &str, write_scope: &str, requirements: &[&str]) -> Value {
    let digest: ContentDigest = request_digest.parse().expect("request digest parses");
    let requirement_values = requirements
        .iter()
        .map(|id| Requirement {
            acceptance_criteria: vec!["The change works.".to_owned()],
            id: (*id).to_owned(),
            statement: format!("Deliver {id}."),
        })
        .collect::<Vec<_>>();
    let task = PlannedTask {
        acceptance_criteria: vec!["Tests pass.".to_owned()],
        dependencies: Vec::new(),
        id: TaskId::new(),
        objective: "Complete the change.".to_owned(),
        requirement_ids: requirements.iter().map(|id| (*id).to_owned()).collect(),
        title: "Complete".to_owned(),
        verification_commands: vec!["rustc --version".to_owned()],
        write_scopes: vec![write_scope.to_owned()],
    };
    let plan = ArchitecturePlan::validate(
        digest,
        requirement_values,
        vec![task],
        Vec::new(),
        Vec::new(),
        vec!["Run the verification command.".to_owned()],
    )
    .expect("plan validates");
    serde_json::to_value(plan).unwrap()
}

struct VerifiedCandidate {
    candidate_digest: String,
    candidate_id: String,
    evidence_ids: Vec<Value>,
}

fn wait_for_job(client: &mut McpClient, project_key: &str, job_id: &str) -> Value {
    let deadline = Instant::now() + Duration::from_secs(240);
    loop {
        let status = call_tool(client, "cycle_status", json!({"project_key": project_key}));
        if let Some(job) = status["jobs"]
            .as_array()
            .and_then(|jobs| jobs.iter().find(|job| job["jobId"] == job_id))
        {
            match job["state"].as_str() {
                Some("done") => return job["result"].clone(),
                Some("failed") => panic!("job {job_id} failed: {}", job["error"]),
                _ => {}
            }
        }
        assert!(
            Instant::now() < deadline,
            "job {job_id} did not finish in time"
        );
        thread::sleep(Duration::from_millis(250));
    }
}

fn start_tool_job(
    client: &mut McpClient,
    project_key: &str,
    tool: &str,
    arguments: Value,
) -> Value {
    let receipt = call_tool(client, tool, arguments);
    let job_id = receipt["jobId"].as_str().expect("job id").to_owned();
    wait_for_job(client, project_key, &job_id)
}

fn freeze_and_verify(
    client: &mut McpClient,
    project_key: &str,
    workflow_id: &str,
    base_revision: &str,
) -> VerifiedCandidate {
    let frozen = start_tool_job(
        client,
        project_key,
        "cycle_freeze",
        json!({"base_revision": base_revision, "project_key": project_key, "workflow_id": workflow_id}),
    );
    assert!(frozen["candidateDigest"].is_string());
    let candidate_id = frozen["candidateId"].as_str().unwrap().to_owned();
    let candidate_digest = frozen["candidateDigest"].as_str().unwrap().to_owned();
    let plan_id = frozen["planId"].as_str().unwrap().to_owned();
    let verified = start_tool_job(
        client,
        project_key,
        "cycle_verify",
        json!({"candidate_id": candidate_id, "plan_id": plan_id, "project_key": project_key, "workflow_id": workflow_id}),
    );
    assert_eq!(verified["mandatoryPassed"], true, "verification must pass");
    let evidence_ids = verified["evidence"]
        .as_array()
        .expect("verification evidence")
        .iter()
        .map(|item| item["record"]["id"].clone())
        .collect::<Vec<_>>();
    assert!(!evidence_ids.is_empty());
    VerifiedCandidate {
        candidate_digest,
        candidate_id,
        evidence_ids,
    }
}

fn consult_arbiter(
    client: &mut McpClient,
    project_key: &str,
    candidate: &VerifiedCandidate,
    requirement_ids: &[&str],
) -> Value {
    let request = json!({
        "candidateDigest": candidate.candidate_digest,
        "evidenceIds": candidate.evidence_ids,
        "originalRequest": "certification request",
        "requirementIds": requirement_ids,
    })
    .to_string();
    let consulted = start_tool_job(
        client,
        project_key,
        "cycle_role",
        json!({
            "operation": "arbiter_verdict",
            "project_key": project_key,
            "request": request,
            "role": "arbiter",
            "session_id": "cert-session",
        }),
    );
    assert_eq!(consulted["output"]["binding"], true);
    consulted["output"]["verdict"].clone()
}

fn arbitrate_and_promote(
    client: &mut McpClient,
    project_key: &str,
    workflow_id: &str,
    project_directory: &Path,
    candidate: &VerifiedCandidate,
    requirement_ids: &[&str],
) -> Value {
    let verdict = consult_arbiter(client, project_key, candidate, requirement_ids);
    let arbitrated = start_tool_job(
        client,
        project_key,
        "cycle_arbitrate",
        json!({"candidate_id": candidate.candidate_id, "project_key": project_key,
               "verdict": verdict, "workflow_id": workflow_id}),
    );
    assert_eq!(arbitrated["decision"], "approved");
    start_tool_job(
        client,
        project_key,
        "cycle_promote",
        json!({"candidate_id": candidate.candidate_id, "project_directory": project_directory,
               "project_key": project_key, "workflow_id": workflow_id}),
    )
}

#[test]
fn quick_cycle_delivers_tested_software_end_to_end() {
    let data_dir = tempfile::tempdir().unwrap();
    let data_path = data_dir.path().to_path_buf();
    let repository = seed_repository("quick-project");
    let _daemon = spawn_daemon(&data_path);
    let mut client = McpClient::spawn(&data_path);
    initialize(&mut client);

    let denied = call_tool(
        &mut client,
        "cycle_start",
        json!({"mode": "quick", "original_request": "certify the quick delivery path",
               "project_key": "cert-quick"}),
    );
    assert_eq!(
        denied["raw"],
        "config/roles.json is missing; configure the four read-only role models first"
    );

    let fake = start_fake_roles(vec![arbiter_script(true)]);
    write_roles(&data_path, &fake.base_url);

    let started = call_tool(
        &mut client,
        "cycle_start",
        json!({"affected_paths": ["app.txt"], "mode": "quick",
               "original_request": "certify the quick delivery path",
               "project_key": "cert-quick"}),
    );
    assert_eq!(started["mode"], "quick");
    let workflow_id = started["workflowId"].as_str().unwrap().to_owned();
    let request_digest = started["requestDigest"].as_str().unwrap().to_owned();

    let submitted = call_tool(
        &mut client,
        "cycle_submit_architecture",
        json!({"plan": plan_value(&request_digest, "app.txt", &["REQ-1"]),
               "project_key": "cert-quick", "workflow_id": workflow_id}),
    );
    assert_eq!(submitted["accepted"], true);

    let worktree = call_tool(
        &mut client,
        "cycle_worktree",
        json!({"project_directory": repository.path, "project_key": "cert-quick",
               "workflow_id": workflow_id}),
    );
    let worktree_path = PathBuf::from(worktree["path"].as_str().unwrap());
    let base_revision = worktree["baseRevision"].as_str().unwrap().to_owned();
    assert!(worktree_path.is_dir());

    fs::write(worktree_path.join("app.txt"), "certified quick\n").unwrap();
    git(&worktree_path, &["add", "-A"]);
    git(&worktree_path, &["commit", "-m", "quick candidate"]);

    let evidence = call_tool(
        &mut client,
        "cycle_evidence",
        json!({"metadata": {"command": "rustc --version", "kind": "command", "outcome": "passed"},
               "project_key": "cert-quick", "session_id": "cert-session",
               "workflow_id": workflow_id}),
    );
    assert!(evidence["sequence"].as_u64().is_some());

    let candidate = freeze_and_verify(&mut client, "cert-quick", &workflow_id, &base_revision);

    let indexed = start_tool_job(
        &mut client,
        "cert-quick",
        "cycle_index",
        json!({"project_directory": repository.path, "project_key": "cert-quick",
               "workflow_id": workflow_id}),
    );
    assert!(
        indexed["index"]["inventoriedFiles"]
            .as_u64()
            .is_some_and(|files| files >= 2),
        "index result: {indexed}"
    );

    let promoted = arbitrate_and_promote(
        &mut client,
        "cert-quick",
        &workflow_id,
        &repository.path,
        &candidate,
        &["REQ-1"],
    );
    assert_eq!(promoted["workflowState"], "completed");
    assert!(
        promoted["changedPaths"]
            .as_array()
            .is_some_and(|paths| paths.iter().any(|path| path == "app.txt"))
    );
    assert_eq!(
        fs::read_to_string(repository.path.join("app.txt")).unwrap(),
        "certified quick\n"
    );

    let status = call_tool(
        &mut client,
        "cycle_status",
        json!({"project_key": "cert-quick"}),
    );
    assert_eq!(status["workflow"]["state"], "completed");

    let verified = call_tool(
        &mut client,
        "cycle_history_verify",
        json!({"project_key": "cert-quick"}),
    );
    assert_eq!(verified["chain"]["status"], "valid");
}

#[test]
fn full_cycle_requires_two_blind_reviews_and_repairs_once() {
    let data_dir = tempfile::tempdir().unwrap();
    let data_path = data_dir.path().to_path_buf();
    let repository = seed_repository("full-project");
    let _daemon = spawn_daemon(&data_path);
    let mut client = McpClient::spawn(&data_path);
    initialize(&mut client);

    let fake = start_fake_roles(vec![
        advisory_script(),
        review_script("functional_reviewer"),
        review_script("security_architecture_reviewer"),
        arbiter_script(false),
        review_script("functional_reviewer"),
        review_script("security_architecture_reviewer"),
        arbiter_script(true),
    ]);
    write_roles(&data_path, &fake.base_url);

    let started = call_tool(
        &mut client,
        "cycle_start",
        json!({"mode": "full", "original_request": "certify the full governed path",
               "project_key": "cert-full"}),
    );
    assert_eq!(started["mode"], "full");
    let workflow_id = started["workflowId"].as_str().unwrap().to_owned();
    let request_digest = started["requestDigest"].as_str().unwrap().to_owned();

    let consulted = start_tool_job(
        &mut client,
        "cert-full",
        "cycle_role",
        json!({"operation": "architect_consult", "project_key": "cert-full",
               "request": "design the change", "role": "architect",
               "session_id": "cert-session"}),
    );
    assert_eq!(consulted["output"]["binding"], false);
    assert_eq!(
        consulted["output"]["advisory"]["summary"],
        "certification advisory answer"
    );

    let submitted = call_tool(
        &mut client,
        "cycle_submit_architecture",
        json!({"plan": plan_value(&request_digest, "app.txt", &["REQ-1", "REQ-2"]),
               "project_key": "cert-full", "workflow_id": workflow_id}),
    );
    assert_eq!(submitted["accepted"], true);

    let worktree = call_tool(
        &mut client,
        "cycle_worktree",
        json!({"project_directory": repository.path, "project_key": "cert-full",
               "workflow_id": workflow_id}),
    );
    let worktree_path = PathBuf::from(worktree["path"].as_str().unwrap());
    let base_revision = worktree["baseRevision"].as_str().unwrap().to_owned();

    let review_candidate = |client: &mut McpClient, candidate: &VerifiedCandidate| {
        for (operation, role) in [
            ("functional_review", "functional_reviewer"),
            ("security_review", "security_reviewer"),
        ] {
            let request = json!({
                "candidateDigest": candidate.candidate_digest,
                "evidenceIds": candidate.evidence_ids,
                "originalRequest": "certify the full governed path",
                "requirementIds": ["REQ-1", "REQ-2"],
            })
            .to_string();
            let consulted = start_tool_job(
                client,
                "cert-full",
                "cycle_role",
                json!({"operation": operation, "project_key": "cert-full",
                       "request": request, "role": role, "session_id": "cert-session"}),
            );
            assert_eq!(consulted["output"]["binding"], true);
            let recorded = call_tool(
                client,
                "cycle_review",
                json!({"candidate_id": candidate.candidate_id, "project_key": "cert-full",
                       "verdict": consulted["output"]["verdict"], "workflow_id": workflow_id}),
            );
            assert!(recorded["reviewsReady"].is_boolean());
        }
    };

    fs::write(worktree_path.join("app.txt"), "first attempt\n").unwrap();
    git(&worktree_path, &["add", "-A"]);
    git(&worktree_path, &["commit", "-m", "first candidate"]);
    let first = freeze_and_verify(&mut client, "cert-full", &workflow_id, &base_revision);
    review_candidate(&mut client, &first);

    let request = json!({
        "candidateDigest": first.candidate_digest,
        "evidenceIds": first.evidence_ids,
        "originalRequest": "certify the full governed path",
        "requirementIds": ["REQ-1", "REQ-2"],
    })
    .to_string();
    let rejected_verdict = start_tool_job(
        &mut client,
        "cert-full",
        "cycle_role",
        json!({"operation": "arbiter_verdict", "project_key": "cert-full",
               "request": request, "role": "arbiter", "session_id": "cert-session"}),
    )["output"]["verdict"]
        .clone();
    let rejected = start_tool_job(
        &mut client,
        "cert-full",
        "cycle_arbitrate",
        json!({"candidate_id": first.candidate_id, "project_key": "cert-full",
               "verdict": rejected_verdict, "workflow_id": workflow_id}),
    );
    assert_eq!(rejected["decision"], "rejected");
    assert_eq!(rejected["workflowState"], "execution");

    let status = call_tool(
        &mut client,
        "cycle_status",
        json!({"project_key": "cert-full"}),
    );
    assert_eq!(status["workflow"]["repairCycles"], 1);

    let indexed = start_tool_job(
        &mut client,
        "cert-full",
        "cycle_index",
        json!({"project_directory": repository.path, "project_key": "cert-full",
               "workflow_id": workflow_id}),
    );
    assert!(indexed["index"]["inventoriedFiles"].as_u64().is_some());

    fs::write(worktree_path.join("app.txt"), "repaired delivery\n").unwrap();
    git(&worktree_path, &["add", "-A"]);
    git(&worktree_path, &["commit", "-m", "repaired candidate"]);
    let second = freeze_and_verify(&mut client, "cert-full", &workflow_id, &base_revision);
    review_candidate(&mut client, &second);

    let promoted = arbitrate_and_promote(
        &mut client,
        "cert-full",
        &workflow_id,
        &repository.path,
        &second,
        &["REQ-1", "REQ-2"],
    );
    assert_eq!(promoted["workflowState"], "completed");
    assert_eq!(
        fs::read_to_string(repository.path.join("app.txt")).unwrap(),
        "repaired delivery\n"
    );

    let verified = call_tool(
        &mut client,
        "cycle_history_verify",
        json!({"project_key": "cert-full"}),
    );
    assert_eq!(verified["chain"]["status"], "valid");
}

#[test]
fn every_cycle_tool_answers_through_mcp() {
    let data_dir = tempfile::tempdir().unwrap();
    let data_path = data_dir.path().to_path_buf();
    let repository = seed_repository("sweep-project");
    let _daemon = spawn_daemon(&data_path);
    let mut client = McpClient::spawn(&data_path);
    initialize(&mut client);

    let fake = start_fake_roles(vec![advisory_script()]);
    write_roles(&data_path, &fake.base_url);

    let setup = call_tool(&mut client, "cycle_setup", json!({}));
    assert!(
        setup["roles"]["configured"].is_boolean() || setup["roles"]["readOnlyRoles"].is_object()
    );

    let tasks = call_tool(
        &mut client,
        "cycle_tasks",
        json!({"project_key": "cert-sweep"}),
    );
    assert!(tasks.is_object());

    let started = call_tool(
        &mut client,
        "cycle_start",
        json!({"mode": "quick", "original_request": "sweep the command surface",
               "project_key": "cert-sweep"}),
    );
    let workflow_id = started["workflowId"].as_str().unwrap().to_owned();

    let paused = call_tool(
        &mut client,
        "cycle_pause",
        json!({"project_key": "cert-sweep"}),
    );
    assert_eq!(paused["state"], "paused");
    let resumed = call_tool(
        &mut client,
        "cycle_resume",
        json!({"project_key": "cert-sweep"}),
    );
    assert_eq!(resumed["state"], "quick_execution");
    let retry = call_tool(
        &mut client,
        "cycle_retry",
        json!({"project_key": "cert-sweep"}),
    );
    assert!(retry["raw"].as_str().is_some() || retry.is_object());

    let execution = call_tool(
        &mut client,
        "cycle_execution_report",
        json!({"outcome": "blocked", "project_key": "cert-sweep",
               "workflow_id": workflow_id}),
    );
    assert!(execution["state"].is_string() || execution["raw"].is_string());

    let goal = call_tool(
        &mut client,
        "cycle_goal_create",
        json!({"objective": "Certify the tool surface", "project_key": "cert-sweep",
               "session_id": "cert-session",
               "success_criteria": ["Every tool answers."]}),
    );
    assert!(goal["goalId"].is_string());
    let goal_id = goal["goalId"].as_str().unwrap().to_owned();

    let listed = call_tool(
        &mut client,
        "cycle_goal_list",
        json!({"project_key": "cert-sweep"}),
    );
    assert!(listed.as_array().is_some() || listed["goals"].is_array());

    let goal_status = call_tool(
        &mut client,
        "cycle_goal_status",
        json!({"project_key": "cert-sweep", "session_id": "cert-session"}),
    );
    assert!(goal_status.is_object());

    let focused = call_tool(
        &mut client,
        "cycle_goal_focus",
        json!({"goal_id": goal_id, "project_key": "cert-sweep",
               "session_id": "cert-session"}),
    );
    assert!(focused.is_object());

    let saved = call_tool(
        &mut client,
        "cycle_goal_save_plan",
        json!({"content": "# Certification plan\nSweep every tool.",
               "goal_id": goal_id, "project_key": "cert-sweep",
               "source_session_id": "cert-session"}),
    );
    assert!(saved.is_object());

    let amended = call_tool(
        &mut client,
        "cycle_goal_amend",
        json!({"goal_id": goal_id, "project_key": "cert-sweep",
               "text": "Cover memory as well."}),
    );
    assert!(amended.is_object());

    for action in ["start_planning", "mark_ready", "activate"] {
        let controlled = call_tool(
            &mut client,
            "cycle_goal_control",
            json!({"action": action, "goal_id": goal_id, "project_key": "cert-sweep"}),
        );
        assert!(
            controlled.is_object(),
            "goal action {action} must answer structurally: {controlled}"
        );
    }

    let memory = call_tool(
        &mut client,
        "cycle_memory_search",
        json!({"project_key": "cert-sweep", "text": "certification"}),
    );
    assert!(memory["entries"].as_array().is_some());

    let explained = call_tool(
        &mut client,
        "cycle_memory_explain",
        json!({"memory_id": "00000000-0000-7000-8000-000000000000",
               "project_key": "cert-sweep"}),
    );
    assert!(explained.is_object());

    let history = call_tool(
        &mut client,
        "cycle_history",
        json!({"limit": 10, "project_key": "cert-sweep"}),
    );
    assert!(history.is_object());

    let models = call_tool(&mut client, "cycle_models", json!({}));
    assert!(models["readOnlyRoles"].is_object());

    let limits = call_tool(&mut client, "cycle_limits", json!({}));
    assert!(limits["admission"]["maximumActive"].as_u64().is_some());

    let executor = call_tool(
        &mut client,
        "cycle_role",
        json!({"operation": "executor_feasibility", "project_key": "cert-sweep",
               "request": "can we ship", "role": "executor"}),
    );
    assert!(
        executor["raw"]
            .as_str()
            .is_some_and(|text| text.contains("Trae Work session"))
    );

    let archived = call_tool(
        &mut client,
        "cycle_export",
        json!({"confirm": true, "project_key": "cert-sweep"}),
    );
    assert!(archived.is_object());

    let cancelled = call_tool(
        &mut client,
        "cycle_cancel",
        json!({"confirm": true, "project_key": "cert-sweep"}),
    );
    assert_eq!(cancelled["state"], "cancelled");
    let _ = repository;
}

#[test]
fn concurrent_projects_share_one_control_plane() {
    let data_dir = tempfile::tempdir().unwrap();
    let data_path = data_dir.path().to_path_buf();
    let first = seed_repository("project-a");
    let second = seed_repository("project-b");
    let _daemon = spawn_daemon(&data_path);
    let mut client = McpClient::spawn(&data_path);
    initialize(&mut client);

    let fake = start_fake_roles(vec![arbiter_script(true)]);
    write_roles(&data_path, &fake.base_url);

    let started_a = call_tool(
        &mut client,
        "cycle_start",
        json!({"mode": "quick", "original_request": "deliver project a",
               "project_key": "cert-multi-a"}),
    );
    let started_b = call_tool(
        &mut client,
        "cycle_start",
        json!({"mode": "quick", "original_request": "hold project b",
               "project_key": "cert-multi-b"}),
    );
    let workflow_a = started_a["workflowId"].as_str().unwrap().to_owned();
    let workflow_b = started_b["workflowId"].as_str().unwrap().to_owned();

    let status_a = call_tool(
        &mut client,
        "cycle_status",
        json!({"project_key": "cert-multi-a", "workflow_id": workflow_a}),
    );
    let status_b = call_tool(
        &mut client,
        "cycle_status",
        json!({"project_key": "cert-multi-b", "workflow_id": workflow_b}),
    );
    assert_eq!(status_a["workflow"]["state"], "quick_execution");
    assert_eq!(status_b["workflow"]["state"], "quick_execution");

    let submitted = call_tool(
        &mut client,
        "cycle_submit_architecture",
        json!({"plan": plan_value(
                   started_a["requestDigest"].as_str().unwrap(),
                   "app.txt",
                   &["REQ-1"]),
               "project_key": "cert-multi-a", "workflow_id": workflow_a}),
    );
    assert_eq!(submitted["accepted"], true);

    let worktree = call_tool(
        &mut client,
        "cycle_worktree",
        json!({"project_directory": first.path, "project_key": "cert-multi-a",
               "workflow_id": workflow_a}),
    );
    let worktree_path = PathBuf::from(worktree["path"].as_str().unwrap());
    let base_revision = worktree["baseRevision"].as_str().unwrap().to_owned();
    fs::write(worktree_path.join("app.txt"), "project a delivered\n").unwrap();
    git(&worktree_path, &["add", "-A"]);
    git(&worktree_path, &["commit", "-m", "candidate a"]);

    let candidate = freeze_and_verify(&mut client, "cert-multi-a", &workflow_a, &base_revision);
    let indexed = start_tool_job(
        &mut client,
        "cert-multi-a",
        "cycle_index",
        json!({"project_directory": first.path, "project_key": "cert-multi-a",
               "workflow_id": workflow_a}),
    );
    assert!(indexed["index"]["inventoriedFiles"].as_u64().is_some());

    let promoted = arbitrate_and_promote(
        &mut client,
        "cert-multi-a",
        &workflow_a,
        &first.path,
        &candidate,
        &["REQ-1"],
    );
    assert_eq!(promoted["workflowState"], "completed");

    let cancelled = call_tool(
        &mut client,
        "cycle_cancel",
        json!({"confirm": true, "project_key": "cert-multi-b"}),
    );
    assert_eq!(cancelled["state"], "cancelled");

    let final_a = call_tool(
        &mut client,
        "cycle_status",
        json!({"project_key": "cert-multi-a"}),
    );
    let final_b = call_tool(
        &mut client,
        "cycle_status",
        json!({"project_key": "cert-multi-b"}),
    );
    assert_eq!(final_a["workflow"]["state"], "completed");
    assert_eq!(final_b["workflow"]["state"], "cancelled");
    assert_eq!(
        fs::read_to_string(first.path.join("app.txt")).unwrap(),
        "project a delivered\n"
    );
    assert_eq!(
        fs::read_to_string(second.path.join("app.txt")).unwrap(),
        "base\n"
    );
}
