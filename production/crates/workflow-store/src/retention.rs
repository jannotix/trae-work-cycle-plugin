//! What the store holds of a candidate, and what of it can be given back.
//!
//! Nothing here deletes a record. The candidate row, its manifest, its per-file digests and
//! executable modes, the exact diff, the evidence recorded against it and every ledger entry that
//! names it all stay exactly where they are. Only the retained *bytes* of a candidate's files go,
//! and only for a workflow that has finished -- where those bytes are either already in the
//! repository, because the candidate was delivered, or deliberately not, because it was cancelled.
//!
//! That is the whole policy, and it is what makes it safe: the manifest keeps a digest for every
//! path, so what a candidate contained stays provable after its bytes are gone. A retention that
//! dropped rows instead would break the chain this product exists to keep.
//!
//! A pruned candidate is exactly the shape the store already had a name for. `payload_complete`
//! goes to zero, so `require_exact_files` returns `MissingCandidatePayload` and the delivery path
//! refuses it by the route it already refuses a candidate whose payload never arrived.

use rusqlite::params;
use workflow_core::{ProjectId, WorkflowState};

use crate::{Store, StoreError, StoreMode};

/// Terminal states: a workflow that will never deliver these bytes again.
const TERMINAL: [WorkflowState; 2] = [WorkflowState::Cancelled, WorkflowState::Completed];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StorageUsage {
    /// Candidate file bytes the project currently holds, and how many rows carry them.
    pub retained_bytes: u64,
    pub retained_files: u64,
    /// The subset belonging to workflows that have reached a terminal state.
    pub prunable_bytes: u64,
    pub prunable_files: u64,
    pub prunable_candidates: u64,
    pub prunable_workflows: u64,
    /// What the project holds overall, so the prunable share can be read against something.
    pub candidates: u64,
    pub workflows: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PrunedPayloads {
    pub bytes: u64,
    pub candidates: u64,
    pub files: u64,
}

struct CandidateRow {
    bytes: u64,
    candidate_id: String,
    files: u64,
    workflow_id: String,
}

impl Store {
    /// What this project's candidates retain, and how much of it a prune would return.
    pub fn storage_usage(&self, project_id: ProjectId) -> Result<StorageUsage, StoreError> {
        let rows = self.candidate_rows(project_id)?;
        let reserved = self.reserved_candidates()?;
        let mut usage = StorageUsage {
            candidates: u64::try_from(rows.len()).map_err(|_| StoreError::IntegerRange)?,
            ..StorageUsage::default()
        };
        let mut workflows = std::collections::BTreeSet::new();
        let mut prunable_workflows = std::collections::BTreeSet::new();
        for row in &rows {
            usage.retained_bytes = usage.retained_bytes.saturating_add(row.bytes);
            usage.retained_files = usage.retained_files.saturating_add(row.files);
            workflows.insert(row.workflow_id.clone());
            if self.is_prunable(row, &reserved)? {
                usage.prunable_bytes = usage.prunable_bytes.saturating_add(row.bytes);
                usage.prunable_files = usage.prunable_files.saturating_add(row.files);
                usage.prunable_candidates = usage.prunable_candidates.saturating_add(1);
                prunable_workflows.insert(row.workflow_id.clone());
            }
        }
        usage.workflows = u64::try_from(workflows.len()).map_err(|_| StoreError::IntegerRange)?;
        usage.prunable_workflows =
            u64::try_from(prunable_workflows.len()).map_err(|_| StoreError::IntegerRange)?;
        Ok(usage)
    }

    /// Releases the retained bytes of finished workflows' candidates and reports what came back.
    pub fn prune_candidate_payloads(
        &mut self,
        project_id: ProjectId,
    ) -> Result<PrunedPayloads, StoreError> {
        if self.mode != StoreMode::ReadWrite {
            return Err(StoreError::ReadOnly);
        }
        let reserved = self.reserved_candidates()?;
        let mut prunable = Vec::new();
        for row in self.candidate_rows(project_id)? {
            if row.files > 0 && self.is_prunable(&row, &reserved)? {
                prunable.push(row);
            }
        }
        let mut pruned = PrunedPayloads::default();
        let transaction = self.connection.transaction()?;
        for row in &prunable {
            transaction.execute(
                "DELETE FROM workflow_candidate_files WHERE candidate_id = ?1",
                params![row.candidate_id],
            )?;
            // The candidate keeps its manifest and digests; only its bytes are gone, which is the
            // state the loader already understands.
            transaction.execute(
                "UPDATE workflow_candidates SET payload_complete = 0 WHERE candidate_id = ?1",
                params![row.candidate_id],
            )?;
            pruned.bytes = pruned.bytes.saturating_add(row.bytes);
            pruned.files = pruned.files.saturating_add(row.files);
            pruned.candidates = pruned.candidates.saturating_add(1);
        }
        transaction.commit()?;
        Ok(pruned)
    }

    /// Candidates belonging to this project, with the bytes each one retains.
    fn candidate_rows(&self, project_id: ProjectId) -> Result<Vec<CandidateRow>, StoreError> {
        let mut statement = self.connection.prepare(
            "SELECT candidates.candidate_id, candidates.workflow_id,
                    coalesce(sum(length(files.content)), 0), count(files.path)
             FROM workflow_candidates candidates
             JOIN workflow_requests requests ON requests.workflow_id = candidates.workflow_id
             LEFT JOIN workflow_candidate_files files
               ON files.candidate_id = candidates.candidate_id
             WHERE requests.project_id = ?1
             GROUP BY candidates.candidate_id",
        )?;
        let rows = statement
            .query_map([project_id.to_string()], |row| {
                Ok(CandidateRow {
                    candidate_id: row.get(0)?,
                    workflow_id: row.get(1)?,
                    bytes: u64::try_from(row.get::<_, i64>(2)?).unwrap_or(0),
                    files: u64::try_from(row.get::<_, i64>(3)?).unwrap_or(0),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// A candidate whose delivery is reserved is mid-flight whatever its workflow says, and its
    /// bytes are what the delivery is about to write.
    fn reserved_candidates(&self) -> Result<std::collections::BTreeSet<String>, StoreError> {
        let mut statement = self
            .connection
            .prepare("SELECT candidate_id FROM candidate_delivery_reservations")?;
        let reserved = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
        Ok(reserved)
    }

    fn is_prunable(
        &self,
        row: &CandidateRow,
        reserved: &std::collections::BTreeSet<String>,
    ) -> Result<bool, StoreError> {
        if reserved.contains(&row.candidate_id) {
            return Ok(false);
        }
        let workflow_id = row
            .workflow_id
            .parse()
            .map_err(|_| StoreError::AggregateConflict)?;
        Ok(self
            .load_workflow(workflow_id)?
            .is_some_and(|workflow| TERMINAL.contains(&workflow.state())))
    }
}
