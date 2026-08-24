use std::str::FromStr;

use workflow_core::WorkflowRole;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RoleOperation {
    ArchitectConsult,
    ExecutorFeasibility,
    FunctionalReview,
    SecurityReview,
    ArbiterReadiness,
    ArbiterVerdict,
}

#[derive(Debug, Eq, PartialEq)]
pub struct UnknownRoleOperation(String);

impl std::fmt::Display for UnknownRoleOperation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "unknown role operation {}", self.0)
    }
}

impl std::error::Error for UnknownRoleOperation {}

impl RoleOperation {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ArchitectConsult => "architect_consult",
            Self::ExecutorFeasibility => "executor_feasibility",
            Self::FunctionalReview => "functional_review",
            Self::SecurityReview => "security_review",
            Self::ArbiterReadiness => "arbiter_readiness",
            Self::ArbiterVerdict => "arbiter_verdict",
        }
    }

    #[must_use]
    pub const fn role(self) -> &'static str {
        match self {
            Self::ArchitectConsult => "architect",
            Self::ExecutorFeasibility => "executor",
            Self::FunctionalReview => "functional_reviewer",
            Self::SecurityReview => "security_reviewer",
            Self::ArbiterReadiness | Self::ArbiterVerdict => "arbiter",
        }
    }

    #[must_use]
    pub const fn workflow_role(self) -> WorkflowRole {
        match self {
            Self::ArchitectConsult => WorkflowRole::Architect,
            Self::ExecutorFeasibility => WorkflowRole::Executor,
            Self::FunctionalReview => WorkflowRole::FunctionalReviewer,
            Self::SecurityReview => WorkflowRole::SecurityArchitectureReviewer,
            Self::ArbiterReadiness | Self::ArbiterVerdict => WorkflowRole::Arbiter,
        }
    }

    #[must_use]
    pub const fn is_in_session(self) -> bool {
        matches!(self, Self::ExecutorFeasibility)
    }
}

impl FromStr for RoleOperation {
    type Err = UnknownRoleOperation;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "architect_consult" => Ok(Self::ArchitectConsult),
            "executor_feasibility" => Ok(Self::ExecutorFeasibility),
            "functional_review" => Ok(Self::FunctionalReview),
            "security_review" => Ok(Self::SecurityReview),
            "arbiter_readiness" => Ok(Self::ArbiterReadiness),
            "arbiter_verdict" => Ok(Self::ArbiterVerdict),
            other => Err(UnknownRoleOperation(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RoleOperation;
    use std::str::FromStr;
    use workflow_core::WorkflowRole;

    #[test]
    fn operations_round_trip_and_map_to_roles() {
        for (text, operation, role) in [
            (
                "architect_consult",
                RoleOperation::ArchitectConsult,
                "architect",
            ),
            (
                "executor_feasibility",
                RoleOperation::ExecutorFeasibility,
                "executor",
            ),
            (
                "functional_review",
                RoleOperation::FunctionalReview,
                "functional_reviewer",
            ),
            (
                "security_review",
                RoleOperation::SecurityReview,
                "security_reviewer",
            ),
            (
                "arbiter_readiness",
                RoleOperation::ArbiterReadiness,
                "arbiter",
            ),
            ("arbiter_verdict", RoleOperation::ArbiterVerdict, "arbiter"),
        ] {
            let parsed = RoleOperation::from_str(text).unwrap();
            assert_eq!(parsed, operation);
            assert_eq!(parsed.as_str(), text);
            assert_eq!(parsed.role(), role);
        }
    }

    #[test]
    fn only_executor_feasibility_runs_in_session() {
        assert!(RoleOperation::ExecutorFeasibility.is_in_session());
        assert!(!RoleOperation::ArbiterVerdict.is_in_session());
        assert_eq!(
            RoleOperation::SecurityReview.workflow_role(),
            WorkflowRole::SecurityArchitectureReviewer
        );
        assert!(RoleOperation::from_str("bogus").is_err());
    }
}
