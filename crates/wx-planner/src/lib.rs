//! Conservative, deterministic workload planning.
//!
//! This module returns deterministic execution strategies. The gateway applies these
//! strategies before invoking a configured provider.

use wx_contract::{
    ExecutionLocation, ExecutionPlan, ExecutionStrategy, PlanStatus, PlanStep, PlanStepKind,
    WorkloadContract,
};
use wx_policy::{PolicyViolation, validate};

pub fn plan(contract: WorkloadContract) -> Result<ExecutionPlan, Vec<PolicyViolation>> {
    validate(&contract)?;

    let strategy = choose_strategy(&contract);
    let mut steps = vec![PlanStep {
        ordinal: 1,
        kind: PlanStepKind::PolicyCheck,
        description: "Validate privacy, execution-location, and workload constraints.".to_owned(),
    }];

    let mut next_ordinal = 2;
    if contract.require_human_approval {
        steps.push(PlanStep {
            ordinal: next_ordinal,
            kind: PlanStepKind::HumanApproval,
            description: "Pause for an authorized human decision before execution.".to_owned(),
        });
        next_ordinal += 1;
    }

    steps.push(PlanStep {
        ordinal: next_ordinal,
        kind: PlanStepKind::SelectRuntime,
        description: format!("Prepare a runtime candidate using the {strategy:?} strategy."),
    });
    steps.push(PlanStep {
        ordinal: next_ordinal + 1,
        kind: PlanStepKind::AcceptanceVerification,
        description: "Evaluate declared acceptance criteria after execution; do not infer correctness from model confidence.".to_owned(),
    });

    let rationale = match strategy {
        ExecutionStrategy::HumanApprovalRequired => vec![
            "The workload explicitly requires human approval.".to_owned(),
            "No execution should begin until the approval gate is satisfied.".to_owned(),
        ],
        ExecutionStrategy::LocalOnly => vec![
            "Only local execution is permitted by the workload contract.".to_owned(),
        ],
        ExecutionStrategy::LocalFirst => vec![
            "A trusted local execution path is available.".to_owned(),
            "The planner prefers the local boundary unless a later, separately authorized plan says otherwise.".to_owned(),
        ],
        ExecutionStrategy::PrivateCloudOnly => vec![
            "The contract permits private-cloud execution but does not permit local execution.".to_owned(),
        ],
        ExecutionStrategy::PrivateCloudFirst => vec![
            "Private-cloud execution is preferred; public-cloud fallback is explicitly permitted by the contract.".to_owned(),
        ],
        ExecutionStrategy::PublicCloudEligible => vec![
            "Public-cloud execution is explicitly allowed and raw-data egress is permitted.".to_owned(),
            "The configured provider router may select public cloud only because this is the sole explicitly allowed location.".to_owned(),
        ],
    };

    Ok(ExecutionPlan {
        workload_id: contract.id,
        strategy,
        steps,
        rationale,
        max_cost_microusd: contract.max_cost_microusd,
        deadline_ms: contract.deadline_ms,
        execution_status: PlanStatus::Planned,
    })
}

fn choose_strategy(contract: &WorkloadContract) -> ExecutionStrategy {
    if contract.require_human_approval {
        return ExecutionStrategy::HumanApprovalRequired;
    }

    let allows_local = contract
        .allowed_execution
        .contains(&ExecutionLocation::Local);
    let allows_private = contract
        .allowed_execution
        .contains(&ExecutionLocation::PrivateCloud);
    let allows_public = contract
        .allowed_execution
        .contains(&ExecutionLocation::PublicCloud);

    if allows_local && (allows_private || allows_public) {
        ExecutionStrategy::LocalFirst
    } else if allows_local {
        ExecutionStrategy::LocalOnly
    } else if allows_private && allows_public {
        ExecutionStrategy::PrivateCloudFirst
    } else if allows_private {
        ExecutionStrategy::PrivateCloudOnly
    } else if allows_public {
        ExecutionStrategy::PublicCloudEligible
    } else {
        // Normally unreachable because policy validation rejects an empty allow-list.
        ExecutionStrategy::LocalOnly
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wx_contract::{EgressPolicy, WorkloadContract};

    #[test]
    fn local_default_contract_plans_local_only() {
        let contract = WorkloadContract {
            objective: "Extract a date from a local document".to_owned(),
            ..WorkloadContract::default()
        };

        let plan = plan(contract).expect("valid local contract");
        assert_eq!(plan.strategy, ExecutionStrategy::LocalOnly);
        assert_eq!(plan.execution_status, PlanStatus::Planned);
    }

    #[test]
    fn approval_requirement_precedes_runtime_selection() {
        let contract = WorkloadContract {
            objective: "Publish a customer-facing announcement".to_owned(),
            require_human_approval: true,
            ..WorkloadContract::default()
        };

        let plan = plan(contract).expect("valid contract");
        assert_eq!(plan.strategy, ExecutionStrategy::HumanApprovalRequired);
        assert_eq!(plan.steps[1].kind, PlanStepKind::HumanApproval);
    }

    #[test]
    fn public_cloud_requires_explicit_egress_permission() {
        let contract = WorkloadContract {
            objective: "Summarize a public changelog".to_owned(),
            raw_data_egress: EgressPolicy::Allow,
            allowed_execution: vec![ExecutionLocation::PublicCloud],
            ..WorkloadContract::default()
        };

        let plan = plan(contract).expect("explicitly allowed public-cloud contract");
        assert_eq!(plan.strategy, ExecutionStrategy::PublicCloudEligible);
    }
}
