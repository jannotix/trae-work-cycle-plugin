use std::{
    collections::BTreeSet,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    sync::mpsc,
    thread,
    time::Duration,
};

use serde_json::{Value, json};
use workflow_core::{
    ArbiterDecision, ArbiterVerdict, ContentDigest, EvidenceId, RequirementDecision,
    RequirementStatus, ReviewDecision, ReviewVerdict, WorkflowRole,
};
use workflow_roles::{
    CompletionUsage, RoleOperation, RolesClient, RolesFile, UsageLedger, client::ReviewOutput,
    config::READ_ONLY_ROLES,
};

struct Script {
    status: u16,
    body: String,
    delay: Option<Duration>,
}

impl Script {
    fn ok(content: Value, usage: Option<(u64, u64, u64)>) -> Self {
        let mut payload = json!({
            "choices": [{"finish_reason": "stop", "index": 0,
                "message": {"content": content.to_string(), "role": "assistant"}}],
            "id": "chatcmpl-fake",
        });
        if let Some((prompt, completion, total)) = usage {
            payload["usage"] = json!({
                "completion_tokens": completion,
                "prompt_tokens": prompt,
                "total_tokens": total,
            });
        }
        Self {
            status: 200,
            body: payload.to_string(),
            delay: None,
        }
    }

    fn failing(status: u16, body: &str) -> Self {
        Self {
            status,
            body: body.to_owned(),
            delay: None,
        }
    }

    fn delayed(delay: Duration) -> Self {
        Self {
            status: 200,
            body: String::new(),
            delay: Some(delay),
        }
    }
}

struct CapturedRequest {
    request_line: String,
    authorization: Option<String>,
    body: Value,
}

struct FakeEndpoint {
    base_url: String,
    requests: mpsc::Receiver<CapturedRequest>,
}

fn start_fake(scripts: Vec<Script>) -> FakeEndpoint {
    let listener = TcpListener::bind("127.0.0.1:0").expect("fake endpoint binds");
    let address = listener.local_addr().expect("fake endpoint address");
    let (sender, requests) = mpsc::channel();
    thread::spawn(move || {
        for script in scripts {
            let Ok((stream, _)) = listener.accept() else {
                break;
            };
            if let Some(delay) = script.delay {
                thread::sleep(delay);
            }
            let mut writer = match stream.try_clone() {
                Ok(writer) => writer,
                Err(_) => continue,
            };
            let captured = match capture_request(&mut BufReader::new(stream)) {
                Some(captured) => captured,
                None => continue,
            };
            let reason = if script.status == 200 { "OK" } else { "Error" };
            let response = format!(
                "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                script.status,
                reason,
                script.body.len(),
                script.body
            );
            let _ = writer.write_all(response.as_bytes());
            let _ = writer.flush();
            let _ = sender.send(captured);
        }
    });
    FakeEndpoint {
        base_url: format!("http://{address}/v1"),
        requests,
    }
}

fn capture_request(reader: &mut BufReader<TcpStream>) -> Option<CapturedRequest> {
    let mut request_line = String::new();
    reader.read_line(&mut request_line).ok()?;
    let mut authorization = None;
    let mut content_length = 0_usize;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header).ok()?;
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        let lower = header.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("authorization:") {
            let offset = header.len() - rest.len();
            authorization = Some(header[offset..].trim().to_owned());
        }
        if let Some(rest) = lower.strip_prefix("content-length:") {
            content_length = rest.trim().parse().ok()?;
        }
    }
    let mut body = vec![0_u8; content_length];
    reader.read_exact(&mut body).ok()?;
    let body: Value = serde_json::from_slice(&body).ok()?;
    Some(CapturedRequest {
        request_line: request_line.trim().to_owned(),
        authorization,
        body,
    })
}

fn setup_roles(directory: &Path, base_url: &str) -> RolesFile {
    let config = directory.join("config");
    std::fs::create_dir_all(&config).expect("config directory");
    let mut roles = serde_json::Map::new();
    for role in READ_ONLY_ROLES {
        let key_file = format!("{role}.key");
        std::fs::write(directory.join(&key_file), format!("secret-{role}\n")).expect("key file");
        roles.insert(
            (*role).to_owned(),
            json!({
                "api_key_file": key_file,
                "base_url": base_url,
                "model_id": format!("provider/{role}"),
            }),
        );
    }
    let text = json!({"roles": roles, "version": 1}).to_string();
    std::fs::write(config.join("roles.json"), &text).expect("roles file");
    serde_json::from_str(&text).expect("roles file parses")
}

fn approved_review(role: WorkflowRole) -> ReviewVerdict {
    ReviewVerdict {
        candidate_digest: ContentDigest::of(b"fixture-candidate"),
        decision: ReviewDecision::Approved,
        findings: Vec::new(),
        repair_target: None,
        requirements: vec![RequirementDecision {
            requirement_id: "r-1".to_owned(),
            status: RequirementStatus::Satisfied,
            evidence_ids: BTreeSet::from([EvidenceId::new()]),
        }],
        role,
    }
}

#[tokio::test]
async fn advisory_call_returns_structured_output_and_records_usage() {
    let directory = tempfile::tempdir().unwrap();
    let fake = start_fake(vec![Script::ok(
        json!({"open_questions": [], "points": ["isolate writes"], "risks": [],
               "summary": "viable with a bounded scope"}),
        Some((11, 7, 18)),
    )]);
    let config = setup_roles(directory.path(), &fake.base_url);
    let ledger = UsageLedger::new();
    let client = RolesClient::new(Duration::from_millis(500));

    let call = client
        .consult(
            &config,
            directory.path(),
            RoleOperation::ArchitectConsult,
            "design a bounded cache layer",
            &ledger,
        )
        .await
        .expect("advisory call");

    assert_eq!(call.result["summary"], "viable with a bounded scope");
    assert_eq!(
        call.usage,
        Some(CompletionUsage {
            prompt_tokens: 11,
            completion_tokens: 7,
            total_tokens: 18,
        })
    );
    let request = fake
        .requests
        .recv_timeout(Duration::from_secs(5))
        .expect("captured request");
    assert!(
        request
            .request_line
            .starts_with("POST /v1/chat/completions")
    );
    assert_eq!(
        request.authorization.as_deref(),
        Some("Bearer secret-architect")
    );
    assert_eq!(request.body["model"], "provider/architect");
    assert_eq!(request.body["response_format"]["type"], "json_object");
    let snapshot = ledger.snapshot().await;
    assert_eq!(snapshot["calls"], 1);
    assert_eq!(snapshot["entries"][0]["totalTokens"], 18);
}

#[tokio::test]
async fn security_review_returns_a_binding_verdict() {
    let directory = tempfile::tempdir().unwrap();
    let fake = start_fake(vec![Script::ok(
        serde_json::to_value(approved_review(WorkflowRole::SecurityArchitectureReviewer)).unwrap(),
        Some((20, 30, 50)),
    )]);
    let config = setup_roles(directory.path(), &fake.base_url);
    let ledger = UsageLedger::new();
    let client = RolesClient::new(Duration::from_millis(500));

    let call = client
        .review(
            &config,
            directory.path(),
            RoleOperation::SecurityReview,
            "review the frozen candidate",
            &ledger,
        )
        .await
        .expect("review call");

    match call.result {
        ReviewOutput::Binding(verdict) => {
            assert_eq!(verdict.decision, ReviewDecision::Approved);
            assert_eq!(verdict.requirements.len(), 1);
        }
        ReviewOutput::Advisory(_) => panic!("expected a binding verdict"),
    }
    let request = fake
        .requests
        .recv_timeout(Duration::from_secs(5))
        .expect("captured request");
    assert_eq!(
        request.authorization.as_deref(),
        Some("Bearer secret-security_reviewer")
    );
}

#[tokio::test]
async fn review_falls_back_to_advisory_for_plain_objects() {
    let directory = tempfile::tempdir().unwrap();
    let fake = start_fake(vec![Script::ok(
        json!({"note": "plan is not complete enough for a binding verdict"}),
        None,
    )]);
    let config = setup_roles(directory.path(), &fake.base_url);
    let client = RolesClient::new(Duration::from_millis(500));

    let call = client
        .review(
            &config,
            directory.path(),
            RoleOperation::FunctionalReview,
            "review the draft plan",
            &UsageLedger::new(),
        )
        .await
        .expect("review call");

    match call.result {
        ReviewOutput::Advisory(value) => assert_eq!(
            value["note"],
            "plan is not complete enough for a binding verdict"
        ),
        ReviewOutput::Binding(_) => panic!("expected an advisory object"),
    }
}

#[tokio::test]
async fn review_rejects_a_verdict_bound_to_the_wrong_role() {
    let directory = tempfile::tempdir().unwrap();
    let fake = start_fake(vec![Script::ok(
        serde_json::to_value(approved_review(WorkflowRole::FunctionalReviewer)).unwrap(),
        None,
    )]);
    let config = setup_roles(directory.path(), &fake.base_url);
    let client = RolesClient::new(Duration::from_millis(500));

    let error = client
        .review(
            &config,
            directory.path(),
            RoleOperation::SecurityReview,
            "review the frozen candidate",
            &UsageLedger::new(),
        )
        .await
        .expect_err("role mismatch");

    assert!(error.to_string().contains("does not match"));
    assert!(!error.is_transient());
}

#[tokio::test]
async fn arbiter_verdict_is_parsed_strictly() {
    let directory = tempfile::tempdir().unwrap();
    let fixture = ArbiterVerdict {
        decision: ArbiterDecision::Approved,
        candidate_digest: ContentDigest::of(b"fixture-candidate"),
        findings: Vec::new(),
        repair_target: None,
        requirements: Vec::new(),
    };
    let fake = start_fake(vec![Script::ok(
        serde_json::to_value(&fixture).unwrap(),
        None,
    )]);
    let config = setup_roles(directory.path(), &fake.base_url);
    let client = RolesClient::new(Duration::from_millis(500));

    let call = client
        .arbitration(
            &config,
            directory.path(),
            "decide over request, candidate, evidence and reviews",
            &UsageLedger::new(),
        )
        .await
        .expect("arbitration call");

    assert_eq!(call.result.decision, ArbiterDecision::Approved);
    let request = fake
        .requests
        .recv_timeout(Duration::from_secs(5))
        .expect("captured request");
    assert_eq!(
        request.authorization.as_deref(),
        Some("Bearer secret-arbiter")
    );
}

#[tokio::test]
async fn malformed_arbiter_output_is_a_permanent_error() {
    let directory = tempfile::tempdir().unwrap();
    let fake = start_fake(vec![Script::ok(json!({"decision": "approved"}), None)]);
    let config = setup_roles(directory.path(), &fake.base_url);
    let client = RolesClient::new(Duration::from_millis(500));

    let error = client
        .arbitration(&config, directory.path(), "decide", &UsageLedger::new())
        .await
        .expect_err("malformed verdict");

    assert!(error.to_string().contains("arbiter verdict"));
    assert!(!error.is_transient());
}

#[tokio::test]
async fn server_errors_are_transient() {
    let directory = tempfile::tempdir().unwrap();
    let fake = start_fake(vec![Script::failing(500, "{\"error\":\"upstream down\"}")]);
    let config = setup_roles(directory.path(), &fake.base_url);
    let client = RolesClient::new(Duration::from_millis(500));

    let error = client
        .consult(
            &config,
            directory.path(),
            RoleOperation::ArbiterReadiness,
            "is the goal ready",
            &UsageLedger::new(),
        )
        .await
        .expect_err("server error");

    assert!(error.is_transient());
    assert!(error.to_string().contains("500"));
}

#[tokio::test]
async fn endpoint_timeouts_are_transient() {
    let directory = tempfile::tempdir().unwrap();
    let fake = start_fake(vec![Script::delayed(Duration::from_millis(900))]);
    let config = setup_roles(directory.path(), &fake.base_url);
    let client = RolesClient::new(Duration::from_millis(200));

    let error = client
        .consult(
            &config,
            directory.path(),
            RoleOperation::ArbiterReadiness,
            "is the goal ready",
            &UsageLedger::new(),
        )
        .await
        .expect_err("timeout");

    assert!(error.is_transient());
    assert!(error.to_string().contains("unreachable"));
}

#[tokio::test]
async fn unreadable_key_files_fail_closed() {
    let directory = tempfile::tempdir().unwrap();
    let fake = start_fake(vec![Script::ok(
        json!({"blocking": [], "ready": true, "summary": "ok", "next_steps": []}),
        None,
    )]);
    let config = setup_roles(directory.path(), &fake.base_url);
    std::fs::remove_file(directory.path().join("arbiter.key")).unwrap();
    let client = RolesClient::new(Duration::from_millis(500));

    let error = client
        .arbitration(&config, directory.path(), "decide", &UsageLedger::new())
        .await
        .expect_err("missing key");

    assert!(!error.is_transient());
    assert!(error.to_string().contains("api key file"));
}
