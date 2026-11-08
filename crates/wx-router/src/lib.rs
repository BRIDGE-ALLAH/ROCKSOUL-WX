//! Deterministic, policy-first provider selection for workload execution.
//!
//! Price data is operator-supplied metadata. It is used for route ranking and
//! post-execution estimates only; it is not a hard spending guarantee.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use wx_contract::{
    DataClassification, EgressPolicy, ExecutionLocation, ExecutionStrategy, WorkloadContract,
};
use wx_planner::plan;
use wx_policy::{PolicyViolation, validate};
use wx_provider::TokenUsage;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProviderProfile {
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub location: ExecutionLocation,
    #[serde(default = "default_quality")]
    pub quality_score: u8,
    #[serde(default = "default_latency")]
    pub p50_latency_ms: u64,
    #[serde(default)]
    pub input_cost_microusd_per_1k_tokens: Option<u64>,
    #[serde(default)]
    pub output_cost_microusd_per_1k_tokens: Option<u64>,
    #[serde(default)]
    pub api_key_env: Option<String>,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_quality() -> u8 {
    50
}
fn default_latency() -> u64 {
    1_000
}
fn default_timeout() -> u64 {
    60
}
fn default_enabled() -> bool {
    true
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteObjective {
    #[default]
    Balanced,
    Quality,
    Cost,
    Latency,
}

impl RouteObjective {
    pub fn parse(value: &str) -> Result<Self, &'static str> {
        match value.trim().to_ascii_lowercase().as_str() {
            "balanced" => Ok(Self::Balanced),
            "quality" => Ok(Self::Quality),
            "cost" => Ok(Self::Cost),
            "latency" => Ok(Self::Latency),
            _ => Err("WX_ROUTE_OBJECTIVE must be balanced, quality, cost, or latency"),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Balanced => "balanced",
            Self::Quality => "quality",
            Self::Cost => "cost",
            Self::Latency => "latency",
        }
    }
}

#[derive(Debug, Error)]
pub enum RouteError {
    #[error("workload policy rejected the route request")]
    PolicyRejected(Vec<PolicyViolation>),
    #[error("no configured provider satisfies the workload constraints")]
    NoEligibleProvider,
    #[error("provider catalog is invalid: {0}")]
    InvalidCatalog(String),
}

pub fn validate_catalog(profiles: &[ProviderProfile]) -> Result<(), RouteError> {
    if profiles.is_empty() {
        return Err(RouteError::InvalidCatalog(
            "at least one provider is required".into(),
        ));
    }

    if !profiles.iter().any(|profile| profile.enabled) {
        return Err(RouteError::InvalidCatalog(
            "at least one provider must be enabled".into(),
        ));
    }

    let mut names = std::collections::HashSet::new();
    for profile in profiles {
        if profile.name.trim().is_empty()
            || profile.base_url.trim().is_empty()
            || profile.model.trim().is_empty()
        {
            return Err(RouteError::InvalidCatalog(
                "provider name, base_url, and model must not be empty".into(),
            ));
        }
        if profile.quality_score > 100 {
            return Err(RouteError::InvalidCatalog(
                "quality_score must be between 0 and 100".into(),
            ));
        }
        if profile.p50_latency_ms == 0 {
            return Err(RouteError::InvalidCatalog(
                "p50_latency_ms must be greater than zero".into(),
            ));
        }
        if !names.insert(profile.name.trim().to_ascii_lowercase()) {
            return Err(RouteError::InvalidCatalog(
                "provider names must be unique (case-insensitive)".into(),
            ));
        }
        if profile.api_key_env.as_ref().is_some_and(|name| {
            name.trim().is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        }) {
            return Err(RouteError::InvalidCatalog(
                "api_key_env must be a valid environment variable name".into(),
            ));
        }
    }
    Ok(())
}

/// Select only among operator-configured providers that satisfy the workload's
/// location and privacy policy. More trusted available locations take precedence;
/// the optimization objective ranks providers within that trust boundary.
pub fn select_provider<'a>(
    contract: &WorkloadContract,
    profiles: &'a [ProviderProfile],
    objective: RouteObjective,
) -> Result<&'a ProviderProfile, RouteError> {
    validate(contract).map_err(RouteError::PolicyRejected)?;
    validate_catalog(profiles)?;

    if contract.require_human_approval {
        return Err(RouteError::NoEligibleProvider);
    }

    let plan = plan(contract.clone()).map_err(RouteError::PolicyRejected)?;
    let mut eligible: Vec<&ProviderProfile> = profiles
        .iter()
        .filter(|profile| {
            profile.enabled
                && contract.allowed_execution.contains(&profile.location)
                && strategy_allows(plan.strategy, profile.location)
                && location_is_permitted(contract, profile.location)
        })
        .collect();

    if eligible.is_empty() {
        return Err(RouteError::NoEligibleProvider);
    }

    let best_location_rank = eligible
        .iter()
        .map(|profile| location_rank(contract, profile.location))
        .min()
        .unwrap_or(u8::MAX);
    eligible.retain(|profile| location_rank(contract, profile.location) == best_location_rank);

    match objective {
        RouteObjective::Quality => eligible.sort_by(|a, b| {
            b.quality_score
                .cmp(&a.quality_score)
                .then_with(|| a.p50_latency_ms.cmp(&b.p50_latency_ms))
                .then_with(|| total_price(a).cmp(&total_price(b)))
                .then_with(|| a.name.cmp(&b.name))
        }),
        RouteObjective::Cost => eligible.sort_by(|a, b| {
            total_price(a)
                .cmp(&total_price(b))
                .then_with(|| b.quality_score.cmp(&a.quality_score))
                .then_with(|| a.p50_latency_ms.cmp(&b.p50_latency_ms))
                .then_with(|| a.name.cmp(&b.name))
        }),
        RouteObjective::Latency => eligible.sort_by(|a, b| {
            a.p50_latency_ms
                .cmp(&b.p50_latency_ms)
                .then_with(|| b.quality_score.cmp(&a.quality_score))
                .then_with(|| total_price(a).cmp(&total_price(b)))
                .then_with(|| a.name.cmp(&b.name))
        }),
        RouteObjective::Balanced => {
            let quality_ranks = rank_by(&eligible, |p| u64::from(100 - p.quality_score));
            let latency_ranks = rank_by(&eligible, |p| p.p50_latency_ms);
            let cost_ranks = rank_by(&eligible, total_price);
            eligible.sort_by(|a, b| {
                let score = |p: &ProviderProfile| {
                    quality_ranks[&p.name] * 2 + latency_ranks[&p.name] + cost_ranks[&p.name]
                };
                score(a)
                    .cmp(&score(b))
                    .then_with(|| b.quality_score.cmp(&a.quality_score))
                    .then_with(|| a.p50_latency_ms.cmp(&b.p50_latency_ms))
                    .then_with(|| a.name.cmp(&b.name))
            });
        }
    }

    eligible
        .first()
        .copied()
        .ok_or(RouteError::NoEligibleProvider)
}

fn strategy_allows(strategy: ExecutionStrategy, location: ExecutionLocation) -> bool {
    match strategy {
        ExecutionStrategy::HumanApprovalRequired => false,
        ExecutionStrategy::LocalOnly => location == ExecutionLocation::Local,
        ExecutionStrategy::LocalFirst => true,
        ExecutionStrategy::PrivateCloudOnly => location == ExecutionLocation::PrivateCloud,
        ExecutionStrategy::PrivateCloudFirst => {
            matches!(
                location,
                ExecutionLocation::PrivateCloud | ExecutionLocation::PublicCloud
            )
        }
        ExecutionStrategy::PublicCloudEligible => location == ExecutionLocation::PublicCloud,
    }
}

fn location_is_permitted(contract: &WorkloadContract, location: ExecutionLocation) -> bool {
    if location == ExecutionLocation::PublicCloud {
        if contract.raw_data_egress != EgressPolicy::Allow {
            return false;
        }
        if matches!(
            contract.data_classification,
            DataClassification::Confidential | DataClassification::Restricted
        ) {
            return false;
        }
    }
    true
}

fn location_rank(contract: &WorkloadContract, location: ExecutionLocation) -> u8 {
    let has_local = contract
        .allowed_execution
        .contains(&ExecutionLocation::Local);
    let has_private = contract
        .allowed_execution
        .contains(&ExecutionLocation::PrivateCloud);
    match location {
        ExecutionLocation::Local => 0,
        ExecutionLocation::PrivateCloud if has_local => 1,
        ExecutionLocation::PrivateCloud => 0,
        ExecutionLocation::PublicCloud if has_local && has_private => 2,
        ExecutionLocation::PublicCloud if has_local || has_private => 1,
        ExecutionLocation::PublicCloud => 0,
    }
}

fn total_price(profile: &ProviderProfile) -> u64 {
    match (
        profile.input_cost_microusd_per_1k_tokens,
        profile.output_cost_microusd_per_1k_tokens,
    ) {
        (Some(input), Some(output)) => input.saturating_add(output),
        _ => u64::MAX,
    }
}

fn rank_by(
    profiles: &[&ProviderProfile],
    metric: impl Fn(&ProviderProfile) -> u64,
) -> HashMap<String, u64> {
    let mut sorted: Vec<(&ProviderProfile, u64)> =
        profiles.iter().map(|p| (*p, metric(p))).collect();
    sorted.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.name.cmp(&b.0.name)));
    let mut ranks = HashMap::new();
    let mut rank = 0_u64;
    let mut previous = None;
    for (profile, value) in sorted {
        if previous.is_some_and(|old| old != value) {
            rank += 1;
        }
        ranks.insert(profile.name.clone(), rank);
        previous = Some(value);
    }
    ranks
}

/// Compute an estimated charge in micro-USD from provider-reported token usage.
/// Price metadata is operator-maintained and may become stale; this is not a
/// pre-execution budget reservation or a provider-billed invoice.
pub fn estimate_cost_microusd(
    profile: &ProviderProfile,
    usage: Option<&TokenUsage>,
) -> Option<u64> {
    let usage = usage?;
    let input_rate = profile.input_cost_microusd_per_1k_tokens?;
    let output_rate = profile.output_cost_microusd_per_1k_tokens?;
    let input_tokens = usage.prompt_tokens?;
    let output_tokens = usage.completion_tokens?;
    let total = u128::from(input_tokens)
        .saturating_mul(u128::from(input_rate))
        .saturating_add(u128::from(output_tokens).saturating_mul(u128::from(output_rate)));
    let rounded_up = total.saturating_add(999) / 1_000;
    Some(rounded_up.min(u128::from(u64::MAX)) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wx_contract::{DataClassification, EgressPolicy, ExecutionLocation};

    fn profile(
        name: &str,
        location: ExecutionLocation,
        quality: u8,
        latency: u64,
        price: u64,
    ) -> ProviderProfile {
        ProviderProfile {
            name: name.into(),
            base_url: format!("https://{name}.example/v1"),
            model: format!("{name}-model"),
            location,
            quality_score: quality,
            p50_latency_ms: latency,
            input_cost_microusd_per_1k_tokens: Some(price),
            output_cost_microusd_per_1k_tokens: Some(price),
            api_key_env: None,
            timeout_seconds: 30,
            enabled: true,
        }
    }

    #[test]
    fn prefers_local_candidates_before_public_cloud() {
        let contract = WorkloadContract {
            objective: "Summarize public release notes".into(),
            data_classification: DataClassification::Public,
            raw_data_egress: EgressPolicy::Allow,
            allowed_execution: vec![ExecutionLocation::Local, ExecutionLocation::PublicCloud],
            ..WorkloadContract::default()
        };
        let providers = vec![
            profile("cloud", ExecutionLocation::PublicCloud, 99, 10, 1),
            profile("local", ExecutionLocation::Local, 60, 900, 50),
        ];
        let selected = select_provider(&contract, &providers, RouteObjective::Quality).unwrap();
        assert_eq!(selected.name, "local");
    }

    #[test]
    fn never_routes_confidential_data_to_public_cloud() {
        let contract = WorkloadContract {
            objective: "Review confidential content".into(),
            data_classification: DataClassification::Confidential,
            raw_data_egress: EgressPolicy::Deny,
            allowed_execution: vec![ExecutionLocation::Local, ExecutionLocation::PublicCloud],
            ..WorkloadContract::default()
        };
        let providers = vec![
            profile("cloud", ExecutionLocation::PublicCloud, 99, 10, 1),
            profile("local", ExecutionLocation::Local, 60, 900, 50),
        ];
        let selected = select_provider(&contract, &providers, RouteObjective::Quality).unwrap();
        assert_eq!(selected.name, "local");
    }

    #[test]
    fn cost_objective_uses_known_operator_supplied_rates() {
        let contract = WorkloadContract {
            objective: "Answer a public question".into(),
            data_classification: DataClassification::Public,
            raw_data_egress: EgressPolicy::Allow,
            allowed_execution: vec![ExecutionLocation::PublicCloud],
            ..WorkloadContract::default()
        };
        let providers = vec![
            profile("expensive", ExecutionLocation::PublicCloud, 95, 500, 900),
            profile("cheap", ExecutionLocation::PublicCloud, 70, 1_000, 100),
        ];
        let selected = select_provider(&contract, &providers, RouteObjective::Cost).unwrap();
        assert_eq!(selected.name, "cheap");
    }

    #[test]
    fn quality_objective_prefers_higher_quality_within_same_trust_boundary() {
        let contract = WorkloadContract {
            objective: "Answer a public question".into(),
            data_classification: DataClassification::Public,
            raw_data_egress: EgressPolicy::Allow,
            allowed_execution: vec![ExecutionLocation::PublicCloud],
            ..WorkloadContract::default()
        };
        let providers = vec![
            profile("fast", ExecutionLocation::PublicCloud, 70, 20, 100),
            profile("quality", ExecutionLocation::PublicCloud, 95, 900, 400),
        ];
        let selected = select_provider(&contract, &providers, RouteObjective::Quality).unwrap();
        assert_eq!(selected.name, "quality");
    }

    #[test]
    fn estimated_charge_uses_usage_and_rounds_up_micro_usd() {
        let provider = profile("priced", ExecutionLocation::PublicCloud, 80, 200, 125);
        let usage = TokenUsage {
            prompt_tokens: Some(1),
            completion_tokens: Some(2),
            total_tokens: Some(3),
        };
        assert_eq!(estimate_cost_microusd(&provider, Some(&usage)), Some(1));
    }

    #[test]
    fn rejects_catalog_without_an_enabled_provider() {
        let mut disabled = profile("disabled", ExecutionLocation::Local, 80, 200, 100);
        disabled.enabled = false;
        assert!(validate_catalog(&[disabled]).is_err());
    }

    #[test]
    fn rejects_duplicate_or_invalid_catalog_entries() {
        let first = profile("same", ExecutionLocation::Local, 80, 200, 100);
        let second = profile("SAME", ExecutionLocation::Local, 80, 200, 100);
        assert!(validate_catalog(&[first, second]).is_err());
    }
}
