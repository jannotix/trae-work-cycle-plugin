use workflow_ledger::{ChainVerification, LedgerChain, LedgerEntry};

const PROTOCOL_V1_EVENT: &str = r#"{"actor":{"id":"arbiter","model":null,"role":null,"session_id":null},"candidate_id":null,"data":{"type":"workflow","action":"approved"},"event_id":"0190f0a0-0000-7000-8000-000000000001","evidence_ids":[],"files":[],"metadata":{},"project_id":"0190f0a0-0000-7000-8000-000000000002","task_id":null,"timestamp":"2026-08-15T12:00:00Z","workflow_id":null}"#;

#[test]
fn protocol_v1_ledger_entry_hash_has_a_fixed_vector() {
    let entry =
        LedgerEntry::new(0, None, serde_json::from_str(PROTOCOL_V1_EVENT).unwrap()).unwrap();
    assert_eq!(
        entry.hash.to_string(),
        "c3c94c9cf026e0fb6a2d9aea7a1b5e786e4860f509f4be64531578326b1c649d"
    );
}

#[test]
fn protocol_v1_history_round_trips_and_verifies() {
    let entry =
        LedgerEntry::new(0, None, serde_json::from_str(PROTOCOL_V1_EVENT).unwrap()).unwrap();
    let encoded = serde_json::to_string(&entry).unwrap();
    let decoded: LedgerEntry = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, entry);
    let chain = LedgerChain::from_entries(vec![entry]);
    let head = chain.head().unwrap();
    assert!(matches!(
        chain.verify(Some(head)),
        ChainVerification::Valid { entries: 1, .. }
    ));
}
