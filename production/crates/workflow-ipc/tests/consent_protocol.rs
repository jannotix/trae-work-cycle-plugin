use workflow_core::{CandidateId, ContentDigest, VerificationPlanId, WorkflowId};
use workflow_ipc::{ClientMessage, ServerMessage};

#[test]
fn verification_consent_messages_round_trip_and_reject_unknown_fields() {
    let request = ClientMessage::GrantVerificationConsent {
        candidate_id: CandidateId::new(),
        consent_token: ContentDigest::of(b"consent"),
        plan_id: VerificationPlanId::new(),
        project_key: "project".to_owned(),
        request_id: 41,
        workflow_id: WorkflowId::new(),
    };
    let encoded = serde_json::to_string(&request).unwrap();
    assert_eq!(
        serde_json::from_str::<ClientMessage>(&encoded).unwrap(),
        request
    );

    let response = ServerMessage::VerificationConsentGranted {
        consent_token: ContentDigest::of(b"consent"),
        expires_at_unix_millis: 2_000,
        request_id: 41,
        workflow_id: WorkflowId::new(),
    };
    let encoded = serde_json::to_string(&response).unwrap();
    assert_eq!(
        serde_json::from_str::<ServerMessage>(&encoded).unwrap(),
        response
    );

    let malformed = encoded.replacen("\"data\":{", "\"data\":{\"extra\":true,", 1);
    assert!(serde_json::from_str::<ServerMessage>(&malformed).is_err());
}
