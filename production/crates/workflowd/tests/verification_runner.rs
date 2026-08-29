use std::{collections::BTreeSet, fs, path::Path, process::Command};

use workflow_core::{
    ArchitecturePlan, CandidateId, ContentDigest, EvidenceKind, EvidenceStatus, PlannedTask,
    Requirement, TaskId, WorkflowId,
};
use workflow_ipc::ManagedBrowserAttestation;
use workflowd::{
    candidate::freeze,
    verification::{
        discover, required_consents, run, run_with_attestations, run_with_authorizations,
    },
};

struct Repository {
    _directory: tempfile::TempDir,
    base: String,
    path: std::path::PathBuf,
}

impl Repository {
    fn new(candidate: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("project");
        fs::create_dir(&path).unwrap();
        git(&path, ["init"]);
        git(&path, ["config", "user.email", "test@example.invalid"]);
        git(&path, ["config", "user.name", "Test User"]);
        git(&path, ["config", "core.hooksPath", ".git/hooks"]);
        git(&path, ["config", "core.autocrlf", "false"]);
        fs::write(path.join("base.txt"), "base\n").unwrap();
        git(&path, ["add", "."]);
        git(&path, ["commit", "-m", "base"]);
        let base = output(&path, ["rev-parse", "HEAD"]);
        fs::write(path.join("candidate.txt"), candidate).unwrap();
        git(&path, ["add", "."]);
        git(&path, ["commit", "-m", "candidate"]);
        Self {
            _directory: directory,
            base,
            path,
        }
    }
}

fn architecture(scopes: Vec<String>) -> ArchitecturePlan {
    architecture_with_commands(scopes, vec!["rustc --version".to_owned()])
}

fn architecture_with_commands(
    scopes: Vec<String>,
    verification_commands: Vec<String>,
) -> ArchitecturePlan {
    ArchitecturePlan::validate(
        ContentDigest::of(b"request"),
        vec![Requirement {
            acceptance_criteria: vec!["Verification passes.".to_owned()],
            id: "REQ-1".to_owned(),
            statement: "Verify the candidate.".to_owned(),
        }],
        vec![PlannedTask {
            acceptance_criteria: vec!["The command succeeds.".to_owned()],
            dependencies: vec![],
            id: TaskId::new(),
            objective: "Run deterministic verification.".to_owned(),
            requirement_ids: vec!["REQ-1".to_owned()],
            title: "Verify".to_owned(),
            verification_commands,
            write_scopes: scopes,
        }],
        vec![],
        vec![],
        vec!["Run the verification command.".to_owned()],
    )
    .unwrap()
}

#[tokio::test]
async fn project_commands_require_the_exact_authorized_gate() {
    let repository = Repository::new("consent-bound change\n");
    let plan = discover(
        &repository.path,
        &architecture_with_commands(
            vec!["candidate.txt".to_owned()],
            vec!["rustc --print sysroot".to_owned()],
        ),
    )
    .unwrap();
    let candidate_id = CandidateId::new();
    let frozen = freeze(
        &repository.path,
        &repository.base,
        candidate_id,
        plan.evidence_ids(),
    )
    .unwrap();
    let requirements = required_consents(
        &repository.path,
        &plan,
        &frozen.manifest,
        WorkflowId::new(),
        candidate_id,
    );
    assert_eq!(requirements.len(), 1);

    let denied = run(
        &repository.path,
        &plan,
        &frozen.manifest,
        &frozen.exact_diff,
        &frozen.exact_files,
    )
    .await
    .unwrap();
    assert!(!denied.mandatory_passed);
    assert!(denied.infrastructure_blocked);
    assert!(denied.records.iter().any(|record| {
        record.id == requirements[0].binding.gate_id
            && record.status == EvidenceStatus::Skipped
            && record.tool == "consent-required"
    }));

    let authorized = BTreeSet::from([requirements[0].binding.gate_id]);
    let allowed = run_with_authorizations(
        &repository.path,
        &plan,
        &frozen.manifest,
        &frozen.exact_diff,
        &frozen.exact_files,
        &[],
        &authorized,
    )
    .await
    .unwrap();
    assert!(allowed.mandatory_passed);
    assert!(!allowed.infrastructure_blocked);
}

#[tokio::test]
async fn commands_capture_normalized_evidence_and_candidate_integrity() {
    let repository = Repository::new("safe change\n");
    let plan = discover(
        &repository.path,
        &architecture(vec!["candidate.txt".to_owned()]),
    )
    .unwrap();
    let frozen = freeze(
        &repository.path,
        &repository.base,
        CandidateId::new(),
        plan.evidence_ids(),
    )
    .unwrap();
    let result = run(
        &repository.path,
        &plan,
        &frozen.manifest,
        &frozen.exact_diff,
        &frozen.exact_files,
    )
    .await
    .unwrap();

    assert!(result.mandatory_passed);
    assert_eq!(result.records.len(), plan.gates.len());
    assert!(result.records.iter().all(|record| {
        record.status == EvidenceStatus::Passed
            && record.candidate_digest == frozen.manifest.digest()
            && !record.tool_version.is_empty()
    }));
    assert!(result.records.iter().any(|record| {
        record.tool == "rustc"
            && record.tool_version.starts_with("rustc ")
            && record.tool_version != "direct-exec-version-unavailable"
    }));
    assert!(
        result
            .records
            .iter()
            .any(|record| record.kind == EvidenceKind::Inspection)
    );
}

#[tokio::test]
async fn unavailable_mandatory_gates_and_seeded_secrets_fail_honestly() {
    let repository = Repository::new("api_key = 'sk-this-is-a-seeded-test-secret'\n");
    let plan = discover(
        &repository.path,
        &architecture(vec!["ui/page.tsx".to_owned()]),
    )
    .unwrap();
    let frozen = freeze(
        &repository.path,
        &repository.base,
        CandidateId::new(),
        plan.evidence_ids(),
    )
    .unwrap();
    let result = run(
        &repository.path,
        &plan,
        &frozen.manifest,
        &frozen.exact_diff,
        &frozen.exact_files,
    )
    .await
    .unwrap();

    assert!(!result.mandatory_passed);
    assert!(result.records.iter().any(|record| {
        record.kind == EvidenceKind::Security && record.status == EvidenceStatus::Failed
    }));
    assert!(
        result
            .records
            .iter()
            .any(|record| record.status == EvidenceStatus::Skipped && record.skip_reason.is_some())
    );
}

#[tokio::test]
async fn managed_browser_receipt_satisfies_only_bound_ui_gates() {
    let repository = Repository::new("safe browser change\n");
    let plan = discover(
        &repository.path,
        &architecture(vec!["ui/page.tsx".to_owned()]),
    )
    .unwrap();
    let frozen = freeze(
        &repository.path,
        &repository.base,
        CandidateId::new(),
        plan.evidence_ids(),
    )
    .unwrap();
    let receipt = browser_receipt(&["open", "snapshot", "check", "screenshot", "logs", "close"]);
    let attestation = ManagedBrowserAttestation {
        candidate_digest: frozen.manifest.digest(),
        receipt_digest: ContentDigest::of(receipt.as_bytes()),
        receipt_json: receipt,
        session_id: "executor-session".to_owned(),
    };

    let result = run_with_attestations(
        &repository.path,
        &plan,
        &frozen.manifest,
        &frozen.exact_diff,
        &frozen.exact_files,
        &[attestation],
    )
    .await
    .unwrap();

    assert!(result.mandatory_passed);
    assert!(!result.infrastructure_blocked);
    assert!(
        result
            .records
            .iter()
            .filter(|record| record.kind == EvidenceKind::Browser)
            .all(|record| record.status == EvidenceStatus::Passed
                && record.tool == "trae-cycle-managed-browser"
                && record.candidate_digest == frozen.manifest.digest())
    );
}

#[tokio::test]
async fn incomplete_or_wrong_candidate_browser_receipts_fail_closed() {
    let repository = Repository::new("safe browser change\n");
    let plan = discover(
        &repository.path,
        &architecture(vec!["ui/page.tsx".to_owned()]),
    )
    .unwrap();
    let frozen = freeze(
        &repository.path,
        &repository.base,
        CandidateId::new(),
        plan.evidence_ids(),
    )
    .unwrap();
    let receipt = browser_receipt(&["open", "check"]);
    let attestations = [ManagedBrowserAttestation {
        candidate_digest: frozen.manifest.digest(),
        receipt_digest: ContentDigest::of(receipt.as_bytes()),
        receipt_json: receipt,
        session_id: "executor-session".to_owned(),
    }];

    let result = run_with_attestations(
        &repository.path,
        &plan,
        &frozen.manifest,
        &frozen.exact_diff,
        &frozen.exact_files,
        &attestations,
    )
    .await
    .unwrap();

    assert!(!result.mandatory_passed);
    assert!(result.infrastructure_blocked);
    assert!(result.records.iter().any(|record| {
        record.kind == EvidenceKind::Browser && record.status == EvidenceStatus::Skipped
    }));
}

fn browser_receipt(operations: &[&str]) -> String {
    let actions = operations
        .iter()
        .map(|operation| {
            serde_json::json!({
                "digest": ContentDigest::of(operation.as_bytes()).to_string(),
                "operation": operation,
                "timestamp": "2026-08-15T12:00:00.000Z",
                "url": "http://127.0.0.1:8766/index.html",
            })
        })
        .collect::<Vec<_>>();
    format!(
        "{}\n",
        serde_json::json!({ "actions": actions, "logs": [] })
    )
}

fn git<'a>(repository: &Path, arguments: impl IntoIterator<Item = &'a str>) {
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(repository)
            .args(arguments)
            .status()
            .unwrap()
            .success()
    );
}

fn output<'a>(repository: &Path, arguments: impl IntoIterator<Item = &'a str>) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(arguments)
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}
