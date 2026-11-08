//! Shared, versionable types passed between ROCKSOUL-WX subsystems.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A request expressed as an outcome and explicit operating constraints.
///
/// The contract is intentionally provider-agnostic. It describes what may happen,
/// not which model must be called.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadContract {
    #[serde(default = "new_workload_id")]
    pub id: Uuid,
    pub objective: String,
    #[serde(default)]
    pub data_classification: DataClassification,
    #[serde(default)]
    pub raw_data_egress: EgressPolicy,
    #[serde(default = "default_execution_locations")]
    pub allowed_execution: Vec<ExecutionLocation>,
    #[serde(default)]
    pub max_cost_microusd: Option<u64>,
    #[serde(default)]
    pub deadline_ms: Option<u64>,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    #[serde(default)]
    pub require_human_approval: bool,
}

impl Default for WorkloadContract {
    fn default() -> Self {
        Self {
            id: new_workload_id(),
            objective: String::new(),
            data_classification: DataClassification::default(),
            raw_data_egress: EgressPolicy::default(),
            allowed_execution: default_execution_locations(),
            max_cost_microusd: None,
            deadline_ms: None,
            acceptance_criteria: Vec::new(),
            require_human_approval: false,
        }
    }
}

fn new_workload_id() -> Uuid {
    Uuid::new_v4()
}

fn default_execution_locations() -> Vec<ExecutionLocation> {
    vec![ExecutionLocation::Local]
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DataClassification {
    Public,
    #[default]
    Internal,
    Confidential,
    Restricted,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EgressPolicy {
    Allow,
    #[default]
    Deny,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionLocation {
    Local,
    PrivateCloud,
    PublicCloud,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStrategy {
    HumanApprovalRequired,
    LocalOnly,
    LocalFirst,
    PrivateCloudFirst,
    PrivateCloudOnly,
    PublicCloudEligible,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStepKind {
    PolicyCheck,
    HumanApproval,
    SelectRuntime,
    AcceptanceVerification,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PlanStep {
    pub ordinal: u8,
    pub kind: PlanStepKind,
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExecutionPlan {
    pub workload_id: Uuid,
    pub strategy: ExecutionStrategy,
    pub steps: Vec<PlanStep>,
    pub rationale: Vec<String>,
    pub max_cost_microusd: Option<u64>,
    pub deadline_ms: Option<u64>,
    /// A plan is a proposal only; execution is not implemented in the first milestone.
    pub execution_status: PlanStatus,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    Planned,
}
