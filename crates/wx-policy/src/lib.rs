//! Deterministic checks for workload contracts.
//!
//! This crate is an enforcement boundary. Model output must never override these checks.

use serde::Serialize;
use wx_contract::{DataClassification, EgressPolicy, ExecutionLocation, WorkloadContract};

#[derive(Clone, Debug, Serialize)]
pub struct PolicyViolation {
    pub code: String,
    pub message: String,
}

impl PolicyViolation {
    fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_owned(),
            message: message.to_owned(),
        }
    }
}

/// Validate a contract before any execution plan is produced.
///
/// The first release intentionally fails closed: confidential/restricted workloads
/// cannot use public-cloud execution or raw-data egress.
pub fn validate(contract: &WorkloadContract) -> Result<(), Vec<PolicyViolation>> {
    let mut violations = Vec::new();

    if contract.objective.trim().is_empty() {
        violations.push(PolicyViolation::new(
            "objective_required",
            "The workload objective must not be empty.",
        ));
    }

    if contract.allowed_execution.is_empty() {
        violations.push(PolicyViolation::new(
            "execution_location_required",
            "At least one execution location must be explicitly allowed.",
        ));
    }

    let allows_public_cloud = contract
        .allowed_execution
        .contains(&ExecutionLocation::PublicCloud);

    if contract.raw_data_egress == EgressPolicy::Deny && allows_public_cloud {
        violations.push(PolicyViolation::new(
            "public_cloud_egress_denied",
            "Public-cloud execution conflicts with raw_data_egress=deny.",
        ));
    }

    if matches!(
        contract.data_classification,
        DataClassification::Confidential | DataClassification::Restricted
    ) {
        if contract.raw_data_egress != EgressPolicy::Deny {
            violations.push(PolicyViolation::new(
                "sensitive_data_egress_forbidden",
                "Confidential and restricted workloads must deny raw-data egress.",
            ));
        }

        if allows_public_cloud {
            violations.push(PolicyViolation::new(
                "sensitive_data_public_cloud_forbidden",
                "Confidential and restricted workloads cannot be sent to public-cloud execution.",
            ));
        }

        let has_trusted_location = contract
            .allowed_execution
            .iter()
            .any(|location| matches!(location, ExecutionLocation::Local | ExecutionLocation::PrivateCloud));

        if !has_trusted_location {
            violations.push(PolicyViolation::new(
                "trusted_execution_required",
                "Sensitive workloads require local or private-cloud execution.",
            ));
        }
    }

    if contract
        .acceptance_criteria
        .iter()
        .any(|criterion| criterion.trim().is_empty())
    {
        violations.push(PolicyViolation::new(
            "empty_acceptance_criterion",
            "Acceptance criteria must not contain empty entries.",
        ));
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wx_contract::{DataClassification, EgressPolicy, ExecutionLocation, WorkloadContract};

    #[test]
    fn default_contract_is_local_and_denies_egress() {
        let contract = WorkloadContract::default();
        assert_eq!(contract.raw_data_egress, EgressPolicy::Deny);
        assert_eq!(contract.allowed_execution, vec![ExecutionLocation::Local]);
        assert!(validate(&contract).is_err(), "empty objectives are invalid");
    }

    #[test]
    fn confidential_workload_cannot_use_public_cloud() {
        let contract = WorkloadContract {
            objective: "Review a confidential contract".to_owned(),
            data_classification: DataClassification::Confidential,
            raw_data_egress: EgressPolicy::Deny,
            allowed_execution: vec![ExecutionLocation::Local, ExecutionLocation::PublicCloud],
            ..WorkloadContract::default()
        };

        let violations = validate(&contract).expect_err("public cloud must be rejected");
        assert!(violations.iter().any(|item| item.code == "public_cloud_egress_denied"));
        assert!(violations
            .iter()
            .any(|item| item.code == "sensitive_data_public_cloud_forbidden"));
    }

    #[test]
    fn confidential_local_workload_is_allowed() {
        let contract = WorkloadContract {
            objective: "Classify a confidential document".to_owned(),
            data_classification: DataClassification::Confidential,
            raw_data_egress: EgressPolicy::Deny,
            allowed_execution: vec![ExecutionLocation::Local],
            ..WorkloadContract::default()
        };

        assert!(validate(&contract).is_ok());
    }
}
