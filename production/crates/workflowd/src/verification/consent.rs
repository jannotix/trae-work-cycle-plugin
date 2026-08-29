use std::path::Path;

use sha2::{Digest, Sha256};
use workflow_core::{
    CandidateId, CandidateManifest, ContentDigest, EvidenceId, VerificationPlanId, WorkflowId,
};
use workflow_ipc::VerificationConsentRequest;
use workflow_store::VerificationConsentBinding;

use super::{CommandAuthorization, VerificationExecutor, VerificationPlan};

pub const CONSENT_VALIDITY_SECONDS: u64 = 15 * 60;
const COMMAND_DOMAIN: &[u8] = b"cycle/verification-command/v1";
const CONSENT_DOMAIN: &[u8] = b"cycle/verification-consent/v1";
const WORKING_DIRECTORY_DOMAIN: &[u8] = b"cycle/verification-working-directory/v1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequiredConsent {
    pub binding: VerificationConsentBinding,
    pub request: VerificationConsentRequest,
}

pub fn required_consents(
    repository: &Path,
    plan: &VerificationPlan,
    manifest: &CandidateManifest,
    workflow_id: WorkflowId,
    candidate_id: CandidateId,
) -> Vec<RequiredConsent> {
    let candidate_digest = manifest.digest();
    let working_directory = std::fs::canonicalize(repository).unwrap_or_else(|_| repository.into());
    let normalized_directory = working_directory.to_string_lossy().replace('\\', "/");
    let working_directory_digest =
        digest(WORKING_DIRECTORY_DOMAIN, &[normalized_directory.as_bytes()]);
    plan.gates
        .iter()
        .filter_map(|gate| {
            let VerificationExecutor::Command {
                arguments,
                authorization: CommandAuthorization::ExplicitConsent,
                program,
            } = &gate.executor
            else {
                return None;
            };
            let mut command_fields = Vec::with_capacity(arguments.len() + 1);
            command_fields.push(program.as_bytes());
            command_fields.extend(arguments.iter().map(String::as_bytes));
            let command_digest = digest(COMMAND_DOMAIN, &command_fields);
            let token = consent_token(
                workflow_id,
                candidate_id,
                plan.id,
                gate.id,
                candidate_digest,
                command_digest,
                working_directory_digest,
            );
            let invocation = serde_json::to_string(&serde_json::json!({
                "arguments": arguments,
                "program": program,
            }))
            .expect("verification command vectors are serializable");
            Some(RequiredConsent {
                binding: VerificationConsentBinding {
                    candidate_digest,
                    candidate_id,
                    command_digest,
                    gate_id: gate.id,
                    plan_id: plan.id,
                    token,
                    working_directory_digest,
                    workflow_id,
                },
                request: VerificationConsentRequest {
                    candidate_digest,
                    command_digest,
                    consent_token: token,
                    gate_id: gate.id,
                    invocation,
                    validity_seconds: CONSENT_VALIDITY_SECONDS,
                    working_directory_digest,
                },
            })
        })
        .collect()
}

fn consent_token(
    workflow_id: WorkflowId,
    candidate_id: CandidateId,
    plan_id: VerificationPlanId,
    gate_id: EvidenceId,
    candidate_digest: ContentDigest,
    command_digest: ContentDigest,
    working_directory_digest: ContentDigest,
) -> ContentDigest {
    let workflow_id = workflow_id.to_string();
    let candidate_id = candidate_id.to_string();
    let plan_id = plan_id.to_string();
    let gate_id = gate_id.to_string();
    digest(
        CONSENT_DOMAIN,
        &[
            workflow_id.as_bytes(),
            candidate_id.as_bytes(),
            plan_id.as_bytes(),
            gate_id.as_bytes(),
            candidate_digest.as_bytes(),
            command_digest.as_bytes(),
            working_directory_digest.as_bytes(),
        ],
    )
}

fn digest(domain: &[u8], fields: &[&[u8]]) -> ContentDigest {
    let mut hasher = Sha256::new();
    write_field(&mut hasher, domain);
    for field in fields {
        write_field(&mut hasher, field);
    }
    ContentDigest::from_bytes(hasher.finalize().into())
}

fn write_field(hasher: &mut Sha256, value: &[u8]) {
    hasher.update(
        u64::try_from(value.len())
            .expect("consent fields fit in u64")
            .to_be_bytes(),
    );
    hasher.update(value);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consent_token_is_stable_and_binds_every_security_dimension() {
        let workflow_id = WorkflowId::new();
        let candidate_id = CandidateId::new();
        let plan_id = VerificationPlanId::new();
        let gate_id = EvidenceId::new();
        let candidate_digest = ContentDigest::of(b"candidate");
        let command_digest = ContentDigest::of(b"command");
        let working_directory_digest = ContentDigest::of(b"worktree");
        let baseline = consent_token(
            workflow_id,
            candidate_id,
            plan_id,
            gate_id,
            candidate_digest,
            command_digest,
            working_directory_digest,
        );
        assert_eq!(
            baseline,
            consent_token(
                workflow_id,
                candidate_id,
                plan_id,
                gate_id,
                candidate_digest,
                command_digest,
                working_directory_digest,
            )
        );
        assert_ne!(
            baseline,
            consent_token(
                workflow_id,
                candidate_id,
                plan_id,
                gate_id,
                ContentDigest::of(b"other candidate"),
                command_digest,
                working_directory_digest,
            )
        );
        assert_ne!(
            baseline,
            consent_token(
                workflow_id,
                candidate_id,
                plan_id,
                gate_id,
                candidate_digest,
                ContentDigest::of(b"other command"),
                working_directory_digest,
            )
        );
        assert_ne!(
            baseline,
            consent_token(
                workflow_id,
                candidate_id,
                plan_id,
                gate_id,
                candidate_digest,
                command_digest,
                ContentDigest::of(b"other worktree"),
            )
        );
    }
}
