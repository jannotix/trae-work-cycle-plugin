use std::collections::BTreeSet;

use rusqlite::{OptionalExtension, params};
use workflow_core::{
    CandidateId, ContentDigest, EvidenceId, EvidenceRecord, VerificationPlanId, WorkflowId,
    WorkflowTimestamp,
};

use crate::{Store, StoreError, StoreMode};

type StoredConsentIdentity = (String, String, String, String, String, String, String);
type StoredConsentState = (String, String, String, i64, Option<String>, Option<String>);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerificationConsentBinding {
    pub candidate_digest: ContentDigest,
    pub candidate_id: CandidateId,
    pub command_digest: ContentDigest,
    pub gate_id: EvidenceId,
    pub plan_id: VerificationPlanId,
    pub token: ContentDigest,
    pub working_directory_digest: ContentDigest,
    pub workflow_id: WorkflowId,
}

impl Store {
    pub fn grant_verification_consent(
        &mut self,
        binding: &VerificationConsentBinding,
        expires_at_unix_millis: i64,
        timestamp: WorkflowTimestamp,
    ) -> Result<bool, StoreError> {
        if self.mode != StoreMode::ReadWrite {
            return Err(StoreError::ReadOnly);
        }
        if expires_at_unix_millis <= 0 {
            return Err(StoreError::AggregateConflict);
        }
        let transaction = self.connection.transaction()?;
        let plan_owner: String = transaction.query_row(
            "SELECT workflow_id FROM workflow_verification_plans WHERE plan_id = ?1",
            [binding.plan_id.to_string()],
            |row| row.get(0),
        )?;
        let candidate: (String, String) = transaction.query_row(
            "SELECT workflow_id, manifest_digest FROM workflow_candidates WHERE candidate_id = ?1",
            [binding.candidate_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if plan_owner != binding.workflow_id.to_string()
            || candidate.0 != binding.workflow_id.to_string()
            || candidate.1 != binding.candidate_digest.to_string()
        {
            return Err(StoreError::AggregateConflict);
        }
        let current: Option<StoredConsentIdentity> = transaction
            .query_row(
                "SELECT workflow_id, candidate_id, plan_id, gate_id, candidate_digest,
                        command_digest, working_directory_digest
                 FROM workflow_verification_consents WHERE consent_token = ?1",
                [binding.token.to_string()],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .optional()?;
        let identity = (
            binding.workflow_id.to_string(),
            binding.candidate_id.to_string(),
            binding.plan_id.to_string(),
            binding.gate_id.to_string(),
            binding.candidate_digest.to_string(),
            binding.command_digest.to_string(),
            binding.working_directory_digest.to_string(),
        );
        if let Some(current) = current {
            if current != identity {
                return Err(StoreError::AggregateConflict);
            }
            transaction.execute(
                "UPDATE workflow_verification_consents
                 SET expires_at_unix_millis = ?2, created_at = ?3,
                     audit_entry_hash = NULL, consumed_at = NULL
                 WHERE consent_token = ?1",
                params![
                    binding.token.to_string(),
                    expires_at_unix_millis,
                    timestamp.to_string(),
                ],
            )?;
            transaction.commit()?;
            return Ok(true);
        }
        transaction.execute(
            "INSERT INTO workflow_verification_consents
             (consent_token, workflow_id, candidate_id, plan_id, gate_id, candidate_digest,
              command_digest, working_directory_digest, expires_at_unix_millis, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                binding.token.to_string(),
                binding.workflow_id.to_string(),
                binding.candidate_id.to_string(),
                binding.plan_id.to_string(),
                binding.gate_id.to_string(),
                binding.candidate_digest.to_string(),
                binding.command_digest.to_string(),
                binding.working_directory_digest.to_string(),
                expires_at_unix_millis,
                timestamp.to_string(),
            ],
        )?;
        transaction.commit()?;
        Ok(false)
    }

    pub fn activate_verification_consent(
        &mut self,
        token: ContentDigest,
        audit_entry_hash: ContentDigest,
    ) -> Result<bool, StoreError> {
        if self.mode != StoreMode::ReadWrite {
            return Err(StoreError::ReadOnly);
        }
        let updated = self.connection.execute(
            "UPDATE workflow_verification_consents SET audit_entry_hash = ?2
             WHERE consent_token = ?1 AND consumed_at IS NULL",
            params![token.to_string(), audit_entry_hash.to_string()],
        )?;
        Ok(updated == 1)
    }

    pub fn claim_verification_consents(
        &mut self,
        workflow_id: WorkflowId,
        candidate_id: CandidateId,
        plan_id: VerificationPlanId,
        tokens: &[ContentDigest],
        now_unix_millis: i64,
        timestamp: WorkflowTimestamp,
    ) -> Result<bool, StoreError> {
        if self.mode != StoreMode::ReadWrite {
            return Err(StoreError::ReadOnly);
        }
        let tokens: BTreeSet<_> = tokens.iter().copied().collect();
        if tokens.is_empty() {
            return Ok(true);
        }
        let transaction = self.connection.transaction()?;
        for token in &tokens {
            let current: Option<StoredConsentState> = transaction
                .query_row(
                    "SELECT workflow_id, candidate_id, plan_id, expires_at_unix_millis,
                            audit_entry_hash, consumed_at
                     FROM workflow_verification_consents WHERE consent_token = ?1",
                    [token.to_string()],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                        ))
                    },
                )
                .optional()?;
            let Some((owner, candidate, plan, expires_at, audit_entry_hash, consumed_at)) = current
            else {
                return Ok(false);
            };
            if owner != workflow_id.to_string()
                || candidate != candidate_id.to_string()
                || plan != plan_id.to_string()
                || expires_at <= now_unix_millis
                || audit_entry_hash.is_none()
                || consumed_at.is_some()
            {
                return Ok(false);
            }
        }
        for token in tokens {
            transaction.execute(
                "UPDATE workflow_verification_consents SET consumed_at = ?2
                 WHERE consent_token = ?1 AND consumed_at IS NULL",
                params![token.to_string(), timestamp.to_string()],
            )?;
        }
        transaction.commit()?;
        Ok(true)
    }

    pub fn save_verification_plan_once(
        &mut self,
        plan_id: VerificationPlanId,
        workflow_id: WorkflowId,
        plan: &serde_json::Value,
        timestamp: WorkflowTimestamp,
    ) -> Result<bool, StoreError> {
        if self.mode != StoreMode::ReadWrite {
            return Err(StoreError::ReadOnly);
        }
        let plan_json = serde_json::to_string(plan)?;
        let transaction = self.connection.transaction()?;
        let current: Option<(String, String)> = transaction
            .query_row(
                "SELECT workflow_id, plan_json FROM workflow_verification_plans WHERE plan_id = ?1",
                [plan_id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        if let Some((owner, current)) = current {
            if owner != workflow_id.to_string() || current != plan_json {
                return Err(StoreError::AggregateConflict);
            }
            return Ok(true);
        }
        transaction.execute(
            "INSERT INTO workflow_verification_plans(plan_id, workflow_id, plan_json, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                plan_id.to_string(),
                workflow_id.to_string(),
                plan_json,
                timestamp.to_string()
            ],
        )?;
        transaction.commit()?;
        Ok(false)
    }

    pub fn load_verification_plan(
        &self,
        plan_id: VerificationPlanId,
    ) -> Result<Option<(WorkflowId, serde_json::Value)>, StoreError> {
        let value: Option<(String, String)> = self
            .connection
            .query_row(
                "SELECT workflow_id, plan_json FROM workflow_verification_plans WHERE plan_id = ?1",
                [plan_id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        value
            .map(|(workflow_id, plan)| {
                Ok((
                    workflow_id
                        .parse()
                        .map_err(|_| StoreError::AggregateConflict)?,
                    serde_json::from_str(&plan)?,
                ))
            })
            .transpose()
    }

    pub fn load_latest_verification_plan_for_workflow(
        &self,
        workflow_id: WorkflowId,
    ) -> Result<Option<(VerificationPlanId, serde_json::Value)>, StoreError> {
        let value: Option<(String, String)> = self
            .connection
            .query_row(
                "SELECT plan_id, plan_json FROM workflow_verification_plans
                 WHERE workflow_id = ?1 ORDER BY created_at DESC, plan_id DESC LIMIT 1",
                [workflow_id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        value
            .map(|(plan_id, plan)| {
                Ok((
                    plan_id.parse().map_err(|_| StoreError::AggregateConflict)?,
                    serde_json::from_str(&plan)?,
                ))
            })
            .transpose()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn save_evidence_once(
        &mut self,
        plan_id: VerificationPlanId,
        workflow_id: WorkflowId,
        candidate_id: CandidateId,
        record: &EvidenceRecord,
        output_redacted: &str,
        mandatory: bool,
        timestamp: WorkflowTimestamp,
    ) -> Result<bool, StoreError> {
        if self.mode != StoreMode::ReadWrite {
            return Err(StoreError::ReadOnly);
        }
        record.validate().map_err(StoreError::Evidence)?;
        if output_redacted.len() > 2 * 1024 * 1024 {
            return Err(StoreError::AggregateConflict);
        }
        let record_json = serde_json::to_string(record)?;
        let transaction = self.connection.transaction()?;
        let plan_owner: String = transaction.query_row(
            "SELECT workflow_id FROM workflow_verification_plans WHERE plan_id = ?1",
            [plan_id.to_string()],
            |row| row.get(0),
        )?;
        let candidate: (String, String) = transaction.query_row(
            "SELECT workflow_id, manifest_digest FROM workflow_candidates WHERE candidate_id = ?1",
            [candidate_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if plan_owner != workflow_id.to_string()
            || candidate.0 != workflow_id.to_string()
            || candidate.1 != record.candidate_digest.to_string()
        {
            return Err(StoreError::AggregateConflict);
        }
        let current: Option<(String, String, String, bool, String, String)> = transaction
            .query_row(
                "SELECT plan_id, workflow_id, candidate_id, mandatory, record_json, output_redacted
                 FROM workflow_evidence WHERE evidence_id = ?1",
                [record.id.to_string()],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .optional()?;
        if let Some(current) = current {
            let identity = (
                plan_id.to_string(),
                workflow_id.to_string(),
                candidate_id.to_string(),
                mandatory,
            );
            if current.0 != identity.0
                || current.1 != identity.1
                || current.2 != identity.2
                || current.3 != identity.3
            {
                return Err(StoreError::AggregateConflict);
            }
            let latest: Option<(i64, String, String)> = transaction
                .query_row(
                    "SELECT attempt, record_json, output_redacted
                     FROM workflow_evidence_attempts WHERE evidence_id = ?1
                     ORDER BY attempt DESC LIMIT 1",
                    [record.id.to_string()],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()?;
            let (attempt, latest_record, latest_output) =
                latest.unwrap_or((0, current.4.clone(), current.5.clone()));
            if latest_record == record_json && latest_output == output_redacted {
                return Ok(true);
            }
            transaction.execute(
                "INSERT INTO workflow_evidence_attempts
                 (evidence_id, attempt, record_json, output_redacted, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    record.id.to_string(),
                    attempt + 1,
                    record_json,
                    output_redacted,
                    timestamp.to_string(),
                ],
            )?;
            transaction.commit()?;
            return Ok(false);
        }
        transaction.execute(
            "INSERT INTO workflow_evidence
             (evidence_id, plan_id, workflow_id, candidate_id, mandatory, record_json,
              output_redacted, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                record.id.to_string(),
                plan_id.to_string(),
                workflow_id.to_string(),
                candidate_id.to_string(),
                mandatory,
                record_json,
                output_redacted,
                timestamp.to_string()
            ],
        )?;
        transaction.commit()?;
        Ok(false)
    }

    pub fn load_candidate_evidence(
        &self,
        candidate_id: CandidateId,
    ) -> Result<Vec<(EvidenceRecord, String, bool)>, StoreError> {
        let mut statement = self.connection.prepare(
            "SELECT
                 COALESCE(
                     (SELECT attempt.record_json FROM workflow_evidence_attempts AS attempt
                      WHERE attempt.evidence_id = evidence.evidence_id
                      ORDER BY attempt.attempt DESC LIMIT 1),
                     evidence.record_json
                 ),
                 COALESCE(
                     (SELECT attempt.output_redacted FROM workflow_evidence_attempts AS attempt
                      WHERE attempt.evidence_id = evidence.evidence_id
                      ORDER BY attempt.attempt DESC LIMIT 1),
                     evidence.output_redacted
                 ),
                 evidence.mandatory
             FROM workflow_evidence AS evidence
             WHERE evidence.candidate_id = ?1 ORDER BY evidence.evidence_id",
        )?;
        let rows = statement.query_map([candidate_id.to_string()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, bool>(2)?,
            ))
        })?;
        rows.map(|row| {
            let (record, output, mandatory) = row?;
            Ok((serde_json::from_str(&record)?, output, mandatory))
        })
        .collect()
    }

    pub fn evidence_exists(&self, evidence_id: EvidenceId) -> Result<bool, StoreError> {
        self.connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM workflow_evidence WHERE evidence_id = ?1)",
                [evidence_id.to_string()],
                |row| row.get(0),
            )
            .map_err(StoreError::from)
    }
}
