use std::num::NonZeroUsize;

use workflow_core::{
    CandidateDigests, CandidateFile, CandidateFileKind, CandidateId, CandidateManifest,
    ContentDigest, ProjectId, RequestRecord, WorkflowCommand, WorkflowId, WorkflowMode,
    WorkflowTimestamp,
};
use workflow_store::{CandidateFilePayload, Store, StoreError};

fn manifest(candidate_id: CandidateId, content: &[u8], exact_diff: &[u8]) -> CandidateManifest {
    let payload_digest = ContentDigest::of(
        &serde_json::to_vec(&[("src/lib.rs", ContentDigest::of(content).to_string(), false)])
            .unwrap(),
    );
    CandidateManifest::new(
        candidate_id,
        Some("base".to_owned()),
        vec![
            CandidateFile::new(
                "src/lib.rs",
                Some(ContentDigest::of(content)),
                CandidateFileKind::Modified,
            )
            .unwrap(),
        ],
        CandidateDigests {
            configuration: ContentDigest::of(b"configuration"),
            dependency_state: ContentDigest::of(b"dependencies"),
            diff: ContentDigest::of(exact_diff),
            environment: ContentDigest::of(b"environment"),
        },
        Vec::new(),
    )
    .unwrap()
    .with_delivery_payload_digest(Some(payload_digest))
}

/// A workflow of this project holding one candidate's bytes.
fn workflow_with_candidate(
    store: &mut Store,
    project_id: ProjectId,
    content: &[u8],
) -> (WorkflowId, CandidateId) {
    let workflow_id = WorkflowId::new();
    store
        .save_request_once(
            workflow_id,
            project_id,
            &RequestRecord::new("retention fixture".to_owned(), vec![]),
            WorkflowTimestamp::now(),
        )
        .unwrap();
    store
        .apply_workflow_command(
            workflow_id,
            &format!("{workflow_id}:intake"),
            WorkflowCommand::CompleteIntake,
            WorkflowTimestamp::now(),
        )
        .unwrap();
    let candidate_id = CandidateId::new();
    store
        .save_candidate_once(
            workflow_id,
            &manifest(candidate_id, content, b"exact diff"),
            b"exact diff",
            &[CandidateFilePayload::new("src/lib.rs", content.to_vec())],
            WorkflowTimestamp::now(),
        )
        .unwrap();
    (workflow_id, candidate_id)
}

fn cancel(store: &mut Store, workflow_id: WorkflowId) {
    store
        .apply_workflow_command(
            workflow_id,
            &format!("{workflow_id}:cancel"),
            WorkflowCommand::Cancel,
            WorkflowTimestamp::now(),
        )
        .unwrap();
}

#[test]
fn pruning_releases_finished_bytes_and_leaves_every_live_one_alone() {
    let directory = tempfile::tempdir().unwrap();
    let mut store = Store::open(
        directory.path().join("workflow.db"),
        NonZeroUsize::new(1).unwrap(),
    )
    .unwrap();
    let project_id = ProjectId::from_stable_key("retention-project");
    let (finished, finished_candidate) =
        workflow_with_candidate(&mut store, project_id, b"finished bytes");
    let (live, live_candidate) = workflow_with_candidate(&mut store, project_id, b"live bytes");
    cancel(&mut store, finished);

    let before = store.storage_usage(project_id).unwrap();
    assert_eq!(before.candidates, 2);
    assert_eq!(before.retained_files, 2);
    assert_eq!(before.prunable_candidates, 1, "only the finished workflow");
    assert_eq!(before.prunable_workflows, 1);
    assert!(before.prunable_bytes > 0 && before.prunable_bytes < before.retained_bytes);

    let pruned = store.prune_candidate_payloads(project_id).unwrap();
    assert_eq!(pruned.candidates, 1);
    assert_eq!(pruned.files, 1);
    assert_eq!(pruned.bytes, before.prunable_bytes);

    // The live candidate is untouched and still deliverable.
    let live_candidate = store.load_candidate(live_candidate).unwrap().unwrap();
    assert_eq!(live_candidate.workflow_id, live);
    assert_eq!(
        live_candidate.require_exact_files().unwrap()[0].content(),
        b"live bytes"
    );

    // The pruned one keeps everything that made it provable, and only its bytes are gone.
    let pruned_candidate = store.load_candidate(finished_candidate).unwrap().unwrap();
    assert_eq!(pruned_candidate.exact_diff, b"exact diff");
    assert_eq!(
        pruned_candidate.manifest.files()[0].digest,
        Some(ContentDigest::of(b"finished bytes")),
        "the digest of what the candidate contained survives its bytes"
    );
    assert_eq!(
        pruned_candidate.manifest.digest(),
        manifest(finished_candidate, b"finished bytes", b"exact diff").digest(),
        "the manifest a prune leaves behind is the manifest it froze"
    );
    assert_eq!(pruned_candidate.exact_files, None);
    assert!(
        matches!(
            pruned_candidate.require_exact_files(),
            Err(StoreError::MissingCandidatePayload)
        ),
        "a pruned candidate refuses delivery by the route a payloadless one already did"
    );

    let after = store.storage_usage(project_id).unwrap();
    assert_eq!(after.candidates, 2, "no row was deleted");
    assert_eq!(after.prunable_bytes, 0);
    assert_eq!(after.retained_bytes, before.retained_bytes - pruned.bytes);
}

#[test]
fn pruning_is_idempotent_and_scoped_to_its_own_project() {
    let directory = tempfile::tempdir().unwrap();
    let mut store = Store::open(
        directory.path().join("workflow.db"),
        NonZeroUsize::new(1).unwrap(),
    )
    .unwrap();
    let mine = ProjectId::from_stable_key("mine");
    let theirs = ProjectId::from_stable_key("theirs");
    let (my_workflow, _) = workflow_with_candidate(&mut store, mine, b"my bytes");
    let (their_workflow, their_candidate) =
        workflow_with_candidate(&mut store, theirs, b"their bytes");
    cancel(&mut store, my_workflow);
    cancel(&mut store, their_workflow);

    let first = store.prune_candidate_payloads(mine).unwrap();
    assert_eq!(first.candidates, 1);
    let second = store.prune_candidate_payloads(mine).unwrap();
    assert_eq!(
        second,
        workflow_store::PrunedPayloads::default(),
        "a second prune has nothing left to free"
    );

    // Another project's finished candidate keeps its bytes: retention answers per project.
    let theirs_still = store.load_candidate(their_candidate).unwrap().unwrap();
    assert_eq!(
        theirs_still.require_exact_files().unwrap()[0].content(),
        b"their bytes"
    );
    assert_eq!(store.storage_usage(theirs).unwrap().prunable_candidates, 1);
}

/// Only a finished workflow is prunable, and "finished" means terminal -- not merely past the
/// point where its bytes look settled. A candidate under verification, review or arbitration is
/// still the one about to be delivered, so this walks a workflow through each of those and asserts
/// its bytes stay put, then cancels it and asserts they become free.
#[test]
fn a_workflow_still_running_keeps_its_bytes_in_every_phase() {
    let directory = tempfile::tempdir().unwrap();
    let mut store = Store::open(
        directory.path().join("workflow.db"),
        NonZeroUsize::new(1).unwrap(),
    )
    .unwrap();
    let project_id = ProjectId::from_stable_key("running-project");
    let (workflow_id, candidate_id) = workflow_with_candidate(&mut store, project_id, b"running");

    for (key, command) in [
        ("route", WorkflowCommand::Route(WorkflowMode::Full)),
        ("architecture", WorkflowCommand::ArchitectureAccepted),
        ("candidate", WorkflowCommand::CandidateReady(candidate_id)),
        ("verified", WorkflowCommand::VerificationPassed),
        ("reviewed", WorkflowCommand::ReviewsReady),
        // Approved but not yet promoted: no delivery reservation exists in this window, so the
        // state is the only thing standing between these bytes and a prune.
        (
            "approved",
            WorkflowCommand::Approve {
                mandatory_gates_passed: true,
            },
        ),
    ] {
        store
            .apply_workflow_command(
                workflow_id,
                &format!("{workflow_id}:{key}"),
                command,
                WorkflowTimestamp::now(),
            )
            .unwrap();
        let state = store.load_workflow(workflow_id).unwrap().unwrap().state();
        assert_eq!(
            store.storage_usage(project_id).unwrap().prunable_candidates,
            0,
            "a workflow in {state:?} is still running and its bytes are still needed"
        );
        assert_eq!(
            store.prune_candidate_payloads(project_id).unwrap(),
            workflow_store::PrunedPayloads::default(),
            "nothing may be freed while the workflow is in {state:?}"
        );
    }

    cancel(&mut store, workflow_id);
    assert_eq!(
        store
            .prune_candidate_payloads(project_id)
            .unwrap()
            .candidates,
        1,
        "once terminal, the same candidate is free"
    );
}

/// Bytes a delivery is about to write are never prunable, and the store already guarantees it from
/// the other side: a workflow whose delivery is reserved cannot reach a terminal state at all, so
/// a mid-flight candidate never enters the prunable set. Retention's own reservation check is
/// defence behind that, not the thing holding the line.
#[test]
fn a_workflow_mid_delivery_cannot_become_prunable() {
    let directory = tempfile::tempdir().unwrap();
    let mut store = Store::open(
        directory.path().join("workflow.db"),
        NonZeroUsize::new(1).unwrap(),
    )
    .unwrap();
    let project_id = ProjectId::from_stable_key("reserved-project");
    let (workflow_id, candidate_id) = workflow_with_candidate(&mut store, project_id, b"reserved");
    let digest = store
        .load_candidate(candidate_id)
        .unwrap()
        .unwrap()
        .manifest
        .digest();
    store
        .reserve_candidate_delivery(workflow_id, candidate_id, digest, WorkflowTimestamp::now())
        .unwrap();

    assert!(
        matches!(
            store.apply_workflow_command(
                workflow_id,
                &format!("{workflow_id}:cancel"),
                WorkflowCommand::Cancel,
                WorkflowTimestamp::now(),
            ),
            Err(StoreError::DeliveryInProgress)
        ),
        "a reserved delivery cannot be cancelled out from under itself"
    );
    assert_eq!(
        store.storage_usage(project_id).unwrap().prunable_candidates,
        0
    );
    assert_eq!(
        store.prune_candidate_payloads(project_id).unwrap(),
        workflow_store::PrunedPayloads::default()
    );
    assert!(
        store
            .load_candidate(candidate_id)
            .unwrap()
            .unwrap()
            .require_exact_files()
            .is_ok()
    );
}
