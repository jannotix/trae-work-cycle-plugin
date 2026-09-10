use std::fs;

use workflow_core::{
    ArchitecturePlan, ContentDigest, EvidenceKind, PlannedTask, Requirement, TaskId,
};
use workflowd::verification::{
    CommandAuthorization, VerificationExecutor, VerificationPlan, discover,
};

fn architecture(scopes: Vec<String>, commands: Vec<String>) -> ArchitecturePlan {
    ArchitecturePlan::validate(
        ContentDigest::of(b"request"),
        vec![Requirement {
            acceptance_criteria: vec!["The feature works end to end.".to_owned()],
            id: "REQ-1".to_owned(),
            statement: "Implement the complete feature.".to_owned(),
        }],
        vec![PlannedTask {
            acceptance_criteria: vec!["All required checks pass.".to_owned()],
            dependencies: vec![],
            id: TaskId::new(),
            objective: "Implement the bounded feature.".to_owned(),
            requirement_ids: vec!["REQ-1".to_owned()],
            title: "Feature".to_owned(),
            verification_commands: commands,
            write_scopes: scopes,
        }],
        vec![],
        vec![],
        vec!["Run the complete integration flow.".to_owned()],
    )
    .unwrap()
}

#[test]
fn adapters_declare_commands_preconditions_risk_timeout_and_mandatory_status() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("package.json"),
        r#"{"scripts":{"lint":"lint","test":"test","test:e2e":"e2e","test:a11y":"a11y"}}"#,
    )
    .unwrap();
    fs::write(directory.path().join("bun.lock"), "lock").unwrap();
    let plan = discover(
        directory.path(),
        &architecture(
            vec!["frontend/components/Login.tsx".to_owned()],
            vec!["bun test".to_owned()],
        ),
    )
    .unwrap();

    assert!(plan.gates.iter().all(|gate| {
        !gate.name.is_empty()
            && !gate.precondition.is_empty()
            && gate.timeout_seconds > 0
            && gate.mandatory
    }));
    assert!(
        plan.gates
            .iter()
            .any(|gate| gate.name.starts_with("browser:") && gate.kind == EvidenceKind::Browser)
    );
    assert!(
        plan.gates
            .iter()
            .any(|gate| gate.name.starts_with("accessibility:"))
    );
    assert!(
        plan.gates
            .iter()
            .any(|gate| matches!(gate.executor, VerificationExecutor::SecretScan))
    );
    assert_eq!(plan.evidence_ids().len(), plan.gates.len());
}

#[test]
fn missing_mandatory_database_and_browser_capabilities_block_explicitly() {
    let directory = tempfile::tempdir().unwrap();
    let plan = discover(
        directory.path(),
        &architecture(
            vec!["migrations/001.sql".to_owned(), "ui/page.tsx".to_owned()],
            vec!["project-test".to_owned()],
        ),
    )
    .unwrap();
    let unavailable: Vec<_> = plan
        .gates
        .iter()
        .filter(|gate| matches!(gate.executor, VerificationExecutor::Unavailable { .. }))
        .collect();
    assert_eq!(unavailable.len(), 3);
    assert!(unavailable.iter().all(|gate| gate.mandatory));
}

#[test]
fn unsafe_commands_are_rejected_without_a_shell() {
    let directory = tempfile::tempdir().unwrap();
    for command in [
        "rm -rf project",
        "bun test && deploy",
        "git reset --hard",
        "bash -c echo",
        "python -c print(1)",
        "node --eval process.exit(0)",
        "node -p process.version",
        "php -r echo(1)",
        "npm run publish:prod",
        "bun run deploy-prod",
    ] {
        assert!(
            discover(
                directory.path(),
                &architecture(vec!["src".to_owned()], vec![command.to_owned()])
            )
            .is_err()
        );
    }

    let value = serde_json::to_value(
        discover(
            directory.path(),
            &architecture(vec!["src".to_owned()], vec!["bun test".to_owned()]),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(serde_json::from_value::<VerificationPlan>(value).is_ok());
}

#[test]
fn only_bounded_non_project_commands_are_preapproved() {
    let directory = tempfile::tempdir().unwrap();
    let plan = discover(
        directory.path(),
        &architecture(
            vec!["src".to_owned()],
            vec![
                "rustc --version".to_owned(),
                "cargo fmt --all --check".to_owned(),
                "cargo test --all-features".to_owned(),
                "node scripts/verify.mjs".to_owned(),
            ],
        ),
    )
    .unwrap();

    let commands: Vec<_> = plan
        .gates
        .iter()
        .filter_map(|gate| match &gate.executor {
            VerificationExecutor::Command {
                arguments,
                authorization,
                program,
            } => Some((program.as_str(), arguments.as_slice(), *authorization)),
            _ => None,
        })
        .collect();
    assert!(commands.contains(&(
        "rustc",
        &["--version".to_owned()][..],
        CommandAuthorization::Preapproved,
    )));
    assert!(commands.contains(&(
        "cargo",
        &["fmt".to_owned(), "--all".to_owned(), "--check".to_owned()][..],
        CommandAuthorization::Preapproved,
    )));
    assert!(commands.contains(&(
        "cargo",
        &["test".to_owned(), "--all-features".to_owned()][..],
        CommandAuthorization::ExplicitConsent,
    )));
    assert!(commands.contains(&(
        "node",
        &["scripts/verify.mjs".to_owned()][..],
        CommandAuthorization::ExplicitConsent,
    )));
}

#[test]
fn conventional_project_adapters_cover_database_browser_accessibility_security_and_package() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("package.json"),
        r#"{"scripts":{"test:database":"db","test:browser":"browser","test:accessibility":"a11y","audit":"audit","license:check":"licenses","package:verify":"package"}}"#,
    )
    .unwrap();
    fs::write(directory.path().join("bun.lock"), "lock").unwrap();
    let plan = discover(
        directory.path(),
        &architecture(
            vec![
                "migrations/001.sql".to_owned(),
                "ui/page.tsx".to_owned(),
                "package.json".to_owned(),
            ],
            vec!["rustc --version".to_owned()],
        ),
    )
    .unwrap();

    for (prefix, kind) in [
        ("database:", EvidenceKind::Database),
        ("browser:", EvidenceKind::Browser),
        ("accessibility:", EvidenceKind::Browser),
        ("security:", EvidenceKind::Security),
        ("package:", EvidenceKind::Package),
    ] {
        assert!(
            plan.gates
                .iter()
                .any(|gate| gate.name.starts_with(prefix) && gate.kind == kind),
            "missing {prefix} adapter"
        );
    }
    assert!(
        plan.gates
            .iter()
            .all(|gate| !matches!(gate.executor, VerificationExecutor::Unavailable { .. }))
    );
}

#[test]
fn dependency_and_packaging_changes_block_without_required_project_adapters() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("package.json"),
        r#"{"scripts":{"test":"test"}}"#,
    )
    .unwrap();
    let plan = discover(
        directory.path(),
        &architecture(
            vec!["package.json".to_owned()],
            vec!["rustc --version".to_owned()],
        ),
    )
    .unwrap();
    let unavailable: Vec<_> = plan
        .gates
        .iter()
        .filter(|gate| matches!(gate.executor, VerificationExecutor::Unavailable { .. }))
        .map(|gate| gate.name.as_str())
        .collect();

    assert!(unavailable.contains(&"security:dependency-vulnerability"));
    assert!(unavailable.contains(&"security:dependency-license"));
    assert!(unavailable.contains(&"package:production-artifact"));
}

/// The layer that breaks is not always the layer that changed. A plan authorized to touch only a
/// configuration file earns the interface gates when the graph says an interface file consumes it,
/// and earns nothing when the graph was never asked.
#[test]
fn what_a_change_reaches_earns_the_gates_of_the_layer_it_reaches() {
    use std::collections::BTreeSet;
    use workflow_code_intel::graph::{Reach, ReachConfidence};

    let directory = tempfile::tempdir().unwrap();
    let plan = architecture(
        vec!["src/config.rs".to_owned()],
        vec!["cargo test --all-features".to_owned()],
    );
    let interface_gates = |plan: &VerificationPlan| {
        plan.gates
            .iter()
            .filter(|gate| {
                gate.name.starts_with("browser:") || gate.name.starts_with("accessibility:")
            })
            .count()
    };

    // Without a reach, a configuration-only scope is a configuration-only change.
    let unreached = discover(directory.path(), &plan).unwrap();
    assert_eq!(
        interface_gates(&unreached),
        0,
        "nothing in the plan touches an interface"
    );

    let reached = workflowd::verification::discover_for(
        directory.path(),
        &plan,
        workflow_core::VerificationPlanId::new(),
        Some(&Reach {
            confidence: ReachConfidence::Resolved,
            hubs: Vec::new(),
            paths: BTreeSet::from(["src/ui/page.tsx".to_owned()]),
            reason: None,
            truncated: false,
        }),
    )
    .unwrap();
    assert_eq!(
        interface_gates(&reached),
        2,
        "the reached interface file requires the browser flow and the accessibility check"
    );
    assert!(
        !reached
            .gates
            .iter()
            .any(|gate| gate.name == "impact:unresolved"),
        "a resolved reach records no uncertainty"
    );
}

/// "I cannot tell what is affected" and "nothing is affected" are different claims. Only the first
/// is recorded, and it does not fail the candidate: code intelligence is bound to delivery here, so
/// a first workflow verifies before its project has ever been indexed.
#[test]
fn an_unresolved_reach_is_recorded_without_failing_the_candidate() {
    use std::collections::BTreeSet;
    use workflow_code_intel::graph::{Reach, ReachConfidence};

    let directory = tempfile::tempdir().unwrap();
    let plan = architecture(
        vec!["src/config.rs".to_owned()],
        vec!["cargo test --all-features".to_owned()],
    );
    let discovered = workflowd::verification::discover_for(
        directory.path(),
        &plan,
        workflow_core::VerificationPlanId::new(),
        Some(&Reach {
            confidence: ReachConfidence::Unresolved,
            hubs: Vec::new(),
            paths: BTreeSet::new(),
            reason: Some("this project has never been indexed. Run cycle_index.".to_owned()),
            truncated: false,
        }),
    )
    .unwrap();

    let recorded = discovered
        .gates
        .iter()
        .find(|gate| gate.name == "impact:unresolved")
        .expect("the uncertainty is recorded");
    assert!(
        !recorded.mandatory,
        "an absent index must not fail every first cycle"
    );
    assert!(
        recorded.precondition.contains("cycle_index"),
        "the record names what would resolve it"
    );
}
