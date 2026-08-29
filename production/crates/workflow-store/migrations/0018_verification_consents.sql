CREATE TABLE workflow_verification_consents (
    consent_token TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    candidate_id TEXT NOT NULL REFERENCES workflow_candidates(candidate_id) ON DELETE CASCADE,
    plan_id TEXT NOT NULL REFERENCES workflow_verification_plans(plan_id) ON DELETE CASCADE,
    gate_id TEXT NOT NULL,
    candidate_digest TEXT NOT NULL,
    command_digest TEXT NOT NULL,
    working_directory_digest TEXT NOT NULL,
    expires_at_unix_millis INTEGER NOT NULL CHECK(expires_at_unix_millis > 0),
    created_at TEXT NOT NULL,
    audit_entry_hash TEXT,
    consumed_at TEXT
) STRICT;

CREATE INDEX workflow_verification_consents_candidate
ON workflow_verification_consents(candidate_id, plan_id, consumed_at, expires_at_unix_millis);

INSERT INTO schema_history(version) VALUES (18);
