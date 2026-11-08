use axum::{
    Json, Router,
    extract::{Path, State, rejection::JsonRejection},
    http::{HeaderMap, HeaderValue, Method, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    collections::hash_map::RandomState,
    hash::{BuildHasher, Hash, Hasher},
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{fs, io::AsyncWriteExt};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    limit::RequestBodyLimitLayer,
};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;
use wx_contract::{
    DataClassification, EgressPolicy, ExecutionLocation, ExecutionPlan, WorkloadContract,
};
use wx_policy::PolicyViolation;
use wx_provider::{ChatMessage, ProviderClient, ProviderError, TokenUsage};
use wx_router::{
    ProviderProfile, RouteObjective, estimate_cost_microusd, select_provider, validate_catalog,
};

#[derive(Clone)]
struct RuntimeProvider {
    profile: ProviderProfile,
    client: ProviderClient,
}

#[derive(Clone)]
struct RateLimitState {
    buckets: Arc<Mutex<HashMap<u64, RateBucket>>>,
    hasher: RandomState,
    requests_per_minute: u32,
}

struct RateBucket {
    window_started: Instant,
    count: u32,
}

impl RateLimitState {
    fn new(requests_per_minute: u32) -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
            hasher: RandomState::new(),
            requests_per_minute,
        }
    }

    fn allow(&self, principal: &str) -> bool {
        let mut hasher = self.hasher.build_hasher();
        principal.hash(&mut hasher);
        let key = hasher.finish();
        let now = Instant::now();
        let Ok(mut buckets) = self.buckets.lock() else {
            return false;
        };
        let bucket = buckets.entry(key).or_insert(RateBucket {
            window_started: now,
            count: 0,
        });
        if now.duration_since(bucket.window_started) >= Duration::from_secs(60) {
            bucket.window_started = now;
            bucket.count = 0;
        }
        if bucket.count >= self.requests_per_minute {
            return false;
        }
        bucket.count += 1;
        if buckets.len() > 4096 {
            buckets.retain(|_, item| {
                now.duration_since(item.window_started) < Duration::from_secs(120)
            });
        }
        true
    }
}

#[derive(Clone)]
struct GatewayState {
    api_key: Option<Arc<str>>,
    providers: Vec<RuntimeProvider>,
    route_objective: RouteObjective,
    rate_limit: RateLimitState,
    receipt_dir: PathBuf,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    mode: &'static str,
    provider_count: usize,
    routing_objective: &'static str,
}

#[derive(Debug, Serialize)]
struct ApiError {
    error: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    violations: Vec<PolicyViolation>,
}

impl ApiError {
    fn new(error: &'static str, message: impl Into<String>) -> Self {
        Self {
            error,
            message: message.into(),
            violations: Vec::new(),
        }
    }

    fn with_violations(violations: Vec<PolicyViolation>) -> Self {
        Self {
            error: "workload_policy_rejected",
            message: "The workload contract did not pass deterministic policy checks.".to_owned(),
            violations,
        }
    }
}

type ApiResult<T> = Result<T, (StatusCode, Json<ApiError>)>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExecuteWorkloadRequest {
    contract: WorkloadContract,
    input: String,
    #[serde(default)]
    model: Option<String>,
}

#[derive(Serialize)]
struct WorkloadExecutionResponse {
    workload_id: Uuid,
    plan: ExecutionPlan,
    provider: String,
    model: String,
    output: String,
    usage: Option<TokenUsage>,
    acceptance: AcceptanceSummary,
    receipt: ExecutionReceipt,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum AcceptanceStatus {
    NotRequested,
    Passed,
    Failed,
    NeedsDefinition,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CriterionResult {
    criterion: String,
    passed: Option<bool>,
    detail: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AcceptanceSummary {
    status: AcceptanceStatus,
    checks: Vec<CriterionResult>,
    note: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ExecutionReceipt {
    id: Uuid,
    workload_id: Uuid,
    created_at_unix_ms: u128,
    provider: String,
    model: String,
    execution_location: ExecutionLocation,
    data_classification: DataClassification,
    raw_data_egress: EgressPolicy,
    usage: Option<TokenUsage>,
    acceptance: AcceptanceSummary,
    route_objective: RouteObjective,
    estimated_cost_microusd: Option<u64>,
    provider_latency_ms: u64,
    note: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OpenAiChatRequest {
    #[serde(default)]
    model: Option<String>,
    messages: Vec<ChatMessage>,
    #[serde(default)]
    stream: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("wx_gateway=info,tower_http=info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let address = std::env::var("WX_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_owned());
    let socket: SocketAddr = address.parse()?;

    let api_key = read_secret("WX_API_KEY")?;
    if let Some(key) = &api_key {
        if key.len() < 32 {
            return Err("WX_API_KEY must contain at least 32 characters".into());
        }
    } else if !socket.ip().is_loopback() {
        return Err("WX_API_KEY is required when WX_BIND_ADDR is not loopback".into());
    }

    let profiles = load_provider_profiles()?;
    validate_catalog(&profiles)?;
    let route_objective = RouteObjective::parse(
        &std::env::var("WX_ROUTE_OBJECTIVE").unwrap_or_else(|_| "balanced".to_owned()),
    )?;
    let mut providers = Vec::with_capacity(profiles.len());
    for profile in profiles {
        if profile.location != ExecutionLocation::Local && !profile.base_url.starts_with("https://")
        {
            return Err(format!("non-local provider '{}' must use HTTPS", profile.name).into());
        }
        let upstream_api_key = match profile.api_key_env.as_deref() {
            Some(name) => read_secret(name)?,
            None => None,
        };
        let timeout = Duration::from_secs(profile.timeout_seconds.clamp(1, 300));
        let client = ProviderClient::new(
            &profile.base_url,
            upstream_api_key,
            profile.model.clone(),
            timeout,
        )?;
        providers.push(RuntimeProvider { profile, client });
    }

    let requests_per_minute = std::env::var("WX_RATE_LIMIT_PER_MINUTE")
        .unwrap_or_else(|_| "120".to_owned())
        .parse::<u32>()?;
    if requests_per_minute == 0 {
        return Err("WX_RATE_LIMIT_PER_MINUTE must be greater than zero".into());
    }

    let receipt_dir = std::env::var("WX_RECEIPT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data/receipts"));

    let state = GatewayState {
        api_key: api_key.map(Arc::from),
        providers,
        route_objective,
        rate_limit: RateLimitState::new(requests_per_minute),
        receipt_dir,
    };

    let listener = tokio::net::TcpListener::bind(socket).await?;
    tracing::info!(
        %socket,
        provider_count = state.providers.len(),
        routing_objective = state.route_objective.as_str(),
        "ROCKSOUL-WX gateway listening"
    );
    axum::serve(
        listener,
        app(state)
            .layer(RequestBodyLimitLayer::new(1024 * 1024))
            .layer(cors_layer()),
    )
    .await?;

    Ok(())
}

fn app(state: GatewayState) -> Router {
    Router::new()
        .route("/healthz", get(health))
        .route("/v1/workloads/plan", post(plan_workload))
        .route("/v1/workloads/execute", post(execute_workload))
        .route("/v1/chat/completions", post(openai_compatible_chat))
        .route("/v1/receipts/{id}", get(get_receipt))
        .with_state(state)
}

fn cors_layer() -> CorsLayer {
    let origins = std::env::var("WX_CORS_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:8081,http://127.0.0.1:8081".to_owned())
        .split(',')
        .filter_map(|origin| HeaderValue::from_str(origin.trim()).ok())
        .collect::<Vec<_>>();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::HeaderName::from_static("x-wx-data-classification"),
            header::HeaderName::from_static("x-wx-allow-egress"),
        ])
}

async fn health(State(state): State<GatewayState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "wx-gateway",
        mode: "workload-execution",
        provider_count: state
            .providers
            .iter()
            .filter(|provider| provider.profile.enabled)
            .count(),
        routing_objective: state.route_objective.as_str(),
    })
}

async fn plan_workload(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    payload: Result<Json<WorkloadContract>, JsonRejection>,
) -> ApiResult<Json<ExecutionPlan>> {
    authorize(&headers, &state)?;
    let Json(contract) = payload
        .map_err(|error| api_error(StatusCode::BAD_REQUEST, "invalid_json", error.body_text()))?;

    wx_planner::plan(contract).map(Json).map_err(|violations| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiError::with_violations(violations)),
        )
    })
}

async fn execute_workload(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    payload: Result<Json<ExecuteWorkloadRequest>, JsonRejection>,
) -> ApiResult<Json<WorkloadExecutionResponse>> {
    authorize(&headers, &state)?;
    let Json(request) = payload
        .map_err(|error| api_error(StatusCode::BAD_REQUEST, "invalid_json", error.body_text()))?;

    if request.input.trim().is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "input_required",
            "Workload input must not be empty.",
        ));
    }
    if request.input.len() > 256 * 1024 {
        return Err(api_error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "input_too_large",
            "Workload input must be at most 256 KiB.",
        ));
    }

    let contract = request.contract;
    if contract.max_cost_microusd.is_some() {
        return Err(api_error(
            StatusCode::NOT_IMPLEMENTED,
            "budget_enforcement_unavailable",
            "Provider cost estimates are recorded after execution, but hard pre-execution budget reservation is not implemented. Remove max_cost_microusd.",
        ));
    }
    let plan = wx_planner::plan(contract.clone()).map_err(|violations| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiError::with_violations(violations)),
        )
    })?;

    if contract.require_human_approval {
        return Err(api_error(
            StatusCode::CONFLICT,
            "human_approval_required",
            "This workload requires an approval workflow; execution is blocked in this release.",
        ));
    }

    let runtime = select_runtime_provider(&state, &contract, request.model.as_deref())?;
    let messages = [ChatMessage {
        role: "user".to_owned(),
        content: request.input,
    }];

    let started = Instant::now();
    let completion_result = if let Some(deadline_ms) = contract.deadline_ms {
        tokio::time::timeout(
            Duration::from_millis(deadline_ms),
            runtime.client.complete(&messages, request.model.as_deref()),
        )
        .await
        .map_err(|_| {
            api_error(
                StatusCode::GATEWAY_TIMEOUT,
                "workload_deadline_exceeded",
                "The configured workload deadline elapsed before the provider completed.",
            )
        })?
    } else {
        runtime
            .client
            .complete(&messages, request.model.as_deref())
            .await
    };
    let provider_latency_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    let completion = completion_result.map_err(provider_error)?;

    let acceptance = evaluate_acceptance(&contract.acceptance_criteria, &completion.content);
    let estimated_cost_microusd =
        estimate_cost_microusd(&runtime.profile, completion.usage.as_ref());
    let receipt = build_receipt(
        &state,
        &runtime.profile,
        contract.id,
        contract.data_classification,
        contract.raw_data_egress,
        completion.model.clone(),
        completion.usage.clone(),
        acceptance.clone(),
        estimated_cost_microusd,
        provider_latency_ms,
    );
    persist_receipt(&state.receipt_dir, &receipt).await.map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "receipt_persistence_failed",
            "The provider completed the workload, but its execution receipt could not be persisted.",
        )
    })?;

    Ok(Json(WorkloadExecutionResponse {
        workload_id: contract.id,
        plan,
        provider: runtime.profile.name.clone(),
        model: completion.model,
        output: completion.content,
        usage: completion.usage,
        acceptance,
        receipt,
    }))
}

async fn openai_compatible_chat(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    payload: Result<Json<OpenAiChatRequest>, JsonRejection>,
) -> ApiResult<Response> {
    authorize(&headers, &state)?;
    let Json(request) = payload
        .map_err(|error| api_error(StatusCode::BAD_REQUEST, "invalid_json", error.body_text()))?;

    if request.stream {
        return Err(api_error(
            StatusCode::NOT_IMPLEMENTED,
            "streaming_not_supported",
            "Streaming is not supported by this first release. Set stream=false.",
        ));
    }
    if request.messages.is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "messages_required",
            "At least one chat message is required.",
        ));
    }
    if request.messages.iter().any(|message| {
        !matches!(
            message.role.as_str(),
            "system" | "developer" | "user" | "assistant" | "tool"
        )
    }) {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "invalid_message_role",
            "Chat message roles must be system, developer, user, assistant, or tool.",
        ));
    }

    let classification = parse_classification_header(&headers)?;
    let egress = parse_egress_header(&headers);
    let mut allowed_execution = Vec::new();
    for runtime in &state.providers {
        let location = runtime.profile.location;
        if location == ExecutionLocation::PublicCloud
            && (egress != EgressPolicy::Allow
                || matches!(
                    classification,
                    DataClassification::Confidential | DataClassification::Restricted
                ))
        {
            continue;
        }
        if !allowed_execution.contains(&location) {
            allowed_execution.push(location);
        }
    }
    let contract = WorkloadContract {
        objective: "OpenAI-compatible chat completion".to_owned(),
        data_classification: classification,
        raw_data_egress: egress,
        allowed_execution,
        ..WorkloadContract::default()
    };
    wx_policy::validate(&contract).map_err(|violations| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiError::with_violations(violations)),
        )
    })?;
    let runtime = select_runtime_provider(&state, &contract, request.model.as_deref())?;

    let started = Instant::now();
    let completion = runtime
        .client
        .complete(&request.messages, request.model.as_deref())
        .await
        .map_err(provider_error)?;
    let provider_latency_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    let estimated_cost_microusd =
        estimate_cost_microusd(&runtime.profile, completion.usage.as_ref());

    let acceptance = AcceptanceSummary {
        status: AcceptanceStatus::NotRequested,
        checks: Vec::new(),
        note: "The compatible chat endpoint does not declare acceptance criteria; output is not verified.".to_owned(),
    };
    let receipt = build_receipt(
        &state,
        &runtime.profile,
        contract.id,
        contract.data_classification,
        contract.raw_data_egress,
        completion.model,
        completion.usage,
        acceptance,
        estimated_cost_microusd,
        provider_latency_ms,
    );
    persist_receipt(&state.receipt_dir, &receipt).await.map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "receipt_persistence_failed",
            "The provider completed the request, but its execution receipt could not be persisted.",
        )
    })?;

    let receipt_id = HeaderValue::from_str(&receipt.id.to_string()).map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "invalid_receipt_id",
            "Could not encode the execution receipt identifier.",
        )
    })?;

    Ok((
        [(
            header::HeaderName::from_static("x-wx-receipt-id"),
            receipt_id,
        )],
        Json(completion.provider_response),
    )
        .into_response())
}

async fn get_receipt(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ExecutionReceipt>> {
    authorize(&headers, &state)?;
    let path = state.receipt_dir.join(format!("{id}.json"));
    let bytes = fs::read(path).await.map_err(|_| {
        api_error(
            StatusCode::NOT_FOUND,
            "receipt_not_found",
            "No receipt exists for this identifier.",
        )
    })?;
    let receipt = serde_json::from_slice::<ExecutionReceipt>(&bytes).map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "receipt_invalid",
            "The stored receipt could not be decoded.",
        )
    })?;
    Ok(Json(receipt))
}

fn authorize(headers: &HeaderMap, state: &GatewayState) -> ApiResult<()> {
    let provided = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default();

    let principal = if let Some(expected) = &state.api_key {
        if !constant_time_eq(provided.as_bytes(), expected.as_bytes()) {
            return Err(api_error(
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "A valid Bearer API key is required.",
            ));
        }
        provided
    } else {
        "loopback-development"
    };

    if state.rate_limit.allow(principal) {
        Ok(())
    } else {
        Err(api_error(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limit_exceeded",
            "The request limit for this gateway credential has been reached; retry after the current 60-second window.",
        ))
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let mut difference = left.len() ^ right.len();
    for index in 0..left.len().max(right.len()) {
        difference |= usize::from(
            left.get(index).copied().unwrap_or_default()
                ^ right.get(index).copied().unwrap_or_default(),
        );
    }
    difference == 0
}

fn parse_location(value: &str) -> Result<ExecutionLocation, &'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "local" => Ok(ExecutionLocation::Local),
        "private_cloud" => Ok(ExecutionLocation::PrivateCloud),
        "public_cloud" => Ok(ExecutionLocation::PublicCloud),
        _ => Err("WX_UPSTREAM_LOCATION must be local, private_cloud, or public_cloud"),
    }
}

fn load_provider_profiles() -> Result<Vec<ProviderProfile>, Box<dyn std::error::Error>> {
    if let Ok(path) = std::env::var("WX_PROVIDER_CATALOG_FILE") {
        let contents = std::fs::read_to_string(path)?;
        return Ok(serde_json::from_str::<Vec<ProviderProfile>>(&contents)?);
    }

    let location = parse_location(
        &std::env::var("WX_UPSTREAM_LOCATION").unwrap_or_else(|_| "local".to_owned()),
    )?;
    let base_url = std::env::var("WX_UPSTREAM_BASE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:11434/v1".to_owned());
    let model = std::env::var("WX_UPSTREAM_MODEL").unwrap_or_else(|_| "llama3.2".to_owned());
    let name = std::env::var("WX_UPSTREAM_NAME").unwrap_or_else(|_| "ollama".to_owned());
    let timeout_seconds = std::env::var("WX_UPSTREAM_TIMEOUT_SECONDS")
        .unwrap_or_else(|_| "60".to_owned())
        .parse::<u64>()?;

    Ok(vec![ProviderProfile {
        name,
        base_url,
        model,
        location,
        quality_score: 50,
        p50_latency_ms: 1_000,
        input_cost_microusd_per_1k_tokens: None,
        output_cost_microusd_per_1k_tokens: None,
        api_key_env: Some("WX_UPSTREAM_API_KEY".to_owned()),
        timeout_seconds,
        enabled: true,
    }])
}

fn read_secret(name: &str) -> Result<Option<String>, std::io::Error> {
    let file_variable = format!("{name}_FILE");
    if let Ok(path) = std::env::var(file_variable) {
        let value = std::fs::read_to_string(path)?;
        let value = value.trim().to_owned();
        return Ok((!value.is_empty()).then_some(value));
    }
    Ok(std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty()))
}

fn select_runtime_provider<'a>(
    state: &'a GatewayState,
    contract: &WorkloadContract,
    requested_model: Option<&str>,
) -> ApiResult<&'a RuntimeProvider> {
    if requested_model.is_some_and(|model| {
        !state
            .providers
            .iter()
            .any(|provider| provider.profile.model == model)
    }) {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "model_not_allowed",
            "The requested model is not present in the operator-configured provider catalog.",
        ));
    }

    let candidates = state
        .providers
        .iter()
        .filter(|provider| requested_model.is_none_or(|model| provider.profile.model == model))
        .map(|provider| provider.profile.clone())
        .collect::<Vec<_>>();
    let selected = select_provider(contract, &candidates, state.route_objective).map_err(|_| {
        api_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "no_eligible_provider",
            "No configured provider satisfies this workload's execution-location and privacy constraints.",
        )
    })?;

    state
        .providers
        .iter()
        .find(|provider| provider.profile.name == selected.name)
        .ok_or_else(|| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "provider_selection_invalid",
                "The routing decision could not be matched to a configured provider runtime.",
            )
        })
}

fn location_name(location: ExecutionLocation) -> &'static str {
    match location {
        ExecutionLocation::Local => "local",
        ExecutionLocation::PrivateCloud => "private_cloud",
        ExecutionLocation::PublicCloud => "public_cloud",
    }
}

fn parse_classification_header(headers: &HeaderMap) -> ApiResult<DataClassification> {
    let value = headers
        .get("x-wx-data-classification")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("internal");

    match value.to_ascii_lowercase().as_str() {
        "public" => Ok(DataClassification::Public),
        "internal" => Ok(DataClassification::Internal),
        "confidential" => Ok(DataClassification::Confidential),
        "restricted" => Ok(DataClassification::Restricted),
        _ => Err(api_error(
            StatusCode::BAD_REQUEST,
            "invalid_data_classification",
            "X-WX-Data-Classification must be public, internal, confidential, or restricted.",
        )),
    }
}

fn parse_egress_header(headers: &HeaderMap) -> EgressPolicy {
    match headers
        .get("x-wx-allow-egress")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("false")
        .to_ascii_lowercase()
        .as_str()
    {
        "true" => EgressPolicy::Allow,
        _ => EgressPolicy::Deny,
    }
}

fn evaluate_acceptance(criteria: &[String], output: &str) -> AcceptanceSummary {
    if criteria.is_empty() {
        return AcceptanceSummary {
            status: AcceptanceStatus::NotRequested,
            checks: Vec::new(),
            note: "No acceptance criteria were supplied; the output is not verified.".to_owned(),
        };
    }

    let mut checks = Vec::with_capacity(criteria.len());
    let mut failed = false;
    let mut unsupported = false;

    for criterion in criteria {
        let normalized = criterion.trim().to_ascii_lowercase();
        let result = if matches!(
            normalized.as_str(),
            "non_empty_output" | "non-empty output" | "output is non-empty"
        ) {
            let passed = !output.trim().is_empty();
            failed |= !passed;
            CriterionResult {
                criterion: criterion.clone(),
                passed: Some(passed),
                detail: "Checks that the generated output contains non-whitespace text.".to_owned(),
            }
        } else if let Some(needle) = normalized.strip_prefix("contains:") {
            let passed = output.to_ascii_lowercase().contains(needle.trim());
            failed |= !passed;
            CriterionResult {
                criterion: criterion.clone(),
                passed: Some(passed),
                detail: "Checks for a case-insensitive substring in the generated output."
                    .to_owned(),
            }
        } else if let Some(minimum) = normalized.strip_prefix("min_length:") {
            match minimum.trim().parse::<usize>() {
                Ok(minimum) => {
                    let passed = output.chars().count() >= minimum;
                    failed |= !passed;
                    CriterionResult {
                        criterion: criterion.clone(),
                        passed: Some(passed),
                        detail: format!(
                            "Checks that output has at least {minimum} Unicode characters."
                        ),
                    }
                }
                Err(_) => {
                    unsupported = true;
                    CriterionResult {
                        criterion: criterion.clone(),
                        passed: None,
                        detail: "Unsupported criterion syntax; use min_length:<integer>."
                            .to_owned(),
                    }
                }
            }
        } else {
            unsupported = true;
            CriterionResult {
                criterion: criterion.clone(),
                passed: None,
                detail: "No deterministic evaluator is registered for this criterion.".to_owned(),
            }
        };
        checks.push(result);
    }

    let status = if failed {
        AcceptanceStatus::Failed
    } else if unsupported {
        AcceptanceStatus::NeedsDefinition
    } else {
        AcceptanceStatus::Passed
    };

    AcceptanceSummary {
        status,
        checks,
        note: "Passed means only the listed supported checks passed; it is not proof of semantic truth.".to_owned(),
    }
}

fn build_receipt(
    state: &GatewayState,
    profile: &ProviderProfile,
    workload_id: Uuid,
    classification: DataClassification,
    egress: EgressPolicy,
    model: String,
    usage: Option<TokenUsage>,
    acceptance: AcceptanceSummary,
    estimated_cost_microusd: Option<u64>,
    provider_latency_ms: u64,
) -> ExecutionReceipt {
    let created_at_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    ExecutionReceipt {
        id: Uuid::new_v4(),
        workload_id,
        created_at_unix_ms,
        provider: profile.name.clone(),
        model,
        execution_location: profile.location,
        data_classification: classification,
        raw_data_egress: egress,
        usage,
        acceptance,
        route_objective: state.route_objective,
        estimated_cost_microusd,
        provider_latency_ms,
        note: "Receipt records execution metadata, operator-supplied price estimates, and named checks; it does not prove semantic correctness or final provider billing.".to_owned(),
    }
}

async fn persist_receipt(
    directory: &std::path::Path,
    receipt: &ExecutionReceipt,
) -> std::io::Result<()> {
    fs::create_dir_all(directory).await?;
    let path = directory.join(format!("{}.json", receipt.id));
    let bytes = serde_json::to_vec_pretty(receipt).map_err(std::io::Error::other)?;

    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .await?;
    file.write_all(&bytes).await?;
    file.sync_all().await
}

fn provider_error(error: ProviderError) -> (StatusCode, Json<ApiError>) {
    let (status, code, message) = match error {
        ProviderError::InvalidBaseUrl => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "provider_configuration_invalid",
            "The configured provider URL is invalid.",
        ),
        ProviderError::Transport(_) => (
            StatusCode::BAD_GATEWAY,
            "provider_unavailable",
            "The configured provider could not be reached or timed out.",
        ),
        ProviderError::ModelNotAllowed => (
            StatusCode::BAD_REQUEST,
            "model_not_allowed",
            "The requested model is not the model configured by the gateway operator.",
        ),
        ProviderError::UpstreamStatus(status) => (
            StatusCode::BAD_GATEWAY,
            "provider_rejected_request",
            if status == 429 {
                "The configured provider rate-limited the request."
            } else {
                "The configured provider returned an unsuccessful status."
            },
        ),
        ProviderError::InvalidResponse => (
            StatusCode::BAD_GATEWAY,
            "provider_response_invalid",
            "The configured provider returned an unsupported response shape.",
        ),
    };
    api_error(status, code, message)
}

fn api_error(
    status: StatusCode,
    code: &'static str,
    message: impl Into<String>,
) -> (StatusCode, Json<ApiError>) {
    (status, Json(ApiError::new(code, message)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use serde_json::json;
    use tower::ServiceExt;

    fn test_state(
        provider: ProviderClient,
        receipt_dir: PathBuf,
        api_key: Option<&str>,
    ) -> GatewayState {
        GatewayState {
            api_key: api_key.map(Arc::from),
            providers: vec![RuntimeProvider {
                profile: ProviderProfile {
                    name: "test-provider".to_owned(),
                    base_url: "http://127.0.0.1:11434/v1".to_owned(),
                    model: "mock-model".to_owned(),
                    location: ExecutionLocation::Local,
                    quality_score: 50,
                    p50_latency_ms: 100,
                    input_cost_microusd_per_1k_tokens: Some(100),
                    output_cost_microusd_per_1k_tokens: Some(100),
                    api_key_env: None,
                    timeout_seconds: 5,
                    enabled: true,
                },
                client: provider,
            }],
            route_objective: RouteObjective::Balanced,
            rate_limit: RateLimitState::new(1_000),
            receipt_dir,
        }
    }

    fn test_provider(base_url: &str) -> ProviderClient {
        ProviderClient::new(base_url, None, "mock-model", Duration::from_secs(5)).unwrap()
    }

    fn test_receipt_dir() -> PathBuf {
        std::env::temp_dir().join(format!("rock-soul-wx-test-{}", Uuid::new_v4()))
    }

    async fn mock_provider() -> (String, tokio::task::JoinHandle<()>) {
        async fn complete() -> Json<Value> {
            Json(json!({
                "id": "mock-completion",
                "model": "mock-model",
                "choices": [{"message": {"role": "assistant", "content": "Hello from mock provider."}}],
                "usage": {"prompt_tokens": 4, "completion_tokens": 5, "total_tokens": 9}
            }))
        }

        let upstream = Router::new().route("/v1/chat/completions", post(complete));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            axum::serve(listener, upstream).await.unwrap();
        });
        (format!("http://{address}/v1"), task)
    }

    #[test]
    fn rate_limiter_enforces_a_per_credential_window() {
        let limiter = RateLimitState::new(2);
        assert!(limiter.allow("test-key"));
        assert!(limiter.allow("test-key"));
        assert!(!limiter.allow("test-key"));
        assert!(limiter.allow("different-key"));
    }

    #[test]
    fn gateway_selects_the_highest_quality_eligible_provider() {
        let mut state = test_state(
            test_provider("http://127.0.0.1:11434/v1"),
            test_receipt_dir(),
            None,
        );
        let mut alternate = state.providers[0].profile.clone();
        alternate.name = "quality-provider".to_owned();
        alternate.model = "quality-model".to_owned();
        alternate.quality_score = 96;
        state.providers.push(RuntimeProvider {
            profile: alternate,
            client: test_provider("http://127.0.0.1:11434/v1"),
        });
        state.route_objective = RouteObjective::Quality;

        let contract = WorkloadContract {
            objective: "Generate a public explanation".to_owned(),
            ..WorkloadContract::default()
        };
        let selected = select_runtime_provider(&state, &contract, None).unwrap();
        assert_eq!(selected.profile.name, "quality-provider");
    }

    #[tokio::test]
    async fn health_endpoint_is_available_without_api_key() {
        let state = test_state(
            test_provider("http://127.0.0.1:11434/v1"),
            test_receipt_dir(),
            None,
        );
        let response = app(state)
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn protected_routes_require_bearer_key_when_configured() {
        let state = test_state(
            test_provider("http://127.0.0.1:11434/v1"),
            test_receipt_dir(),
            Some("test-api-key"),
        );
        let response = app(state)
            .oneshot(
                Request::builder()
                    .uri("/v1/workloads/plan")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"objective":"test"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn executes_workload_and_persists_receipt() {
        let (upstream_url, upstream_task) = mock_provider().await;
        let receipt_dir = test_receipt_dir();
        let state = test_state(test_provider(&upstream_url), receipt_dir.clone(), None);
        let payload = json!({
            "contract": {
                "objective": "Greet the operator",
                "data_classification": "public",
                "raw_data_egress": "deny",
                "allowed_execution": ["local"],
                "acceptance_criteria": ["non_empty_output", "contains:hello"]
            },
            "input": "Say hello."
        });

        let response = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/workloads/execute")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let result: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(result["output"], "Hello from mock provider.");
        assert_eq!(result["acceptance"]["status"], "passed");

        let receipt_id = result["receipt"]["id"].as_str().unwrap();
        assert_eq!(result["receipt"]["provider"], "test-provider");
        assert_eq!(result["receipt"]["route_objective"], "balanced");
        assert_eq!(result["receipt"]["estimated_cost_microusd"], 1);
        assert!(result["receipt"]["provider_latency_ms"].is_number());
        assert!(receipt_dir.join(format!("{receipt_id}.json")).exists());

        upstream_task.abort();
        let _ = fs::remove_dir_all(receipt_dir).await;
    }

    #[tokio::test]
    async fn rejects_confidential_data_for_public_provider() {
        let (upstream_url, upstream_task) = mock_provider().await;
        let receipt_dir = test_receipt_dir();
        let mut state = test_state(test_provider(&upstream_url), receipt_dir.clone(), None);
        state.providers[0].profile.location = ExecutionLocation::PublicCloud;
        let payload = json!({
            "contract": {
                "objective": "Review a confidential file",
                "data_classification": "confidential",
                "raw_data_egress": "deny",
                "allowed_execution": ["local", "public_cloud"]
            },
            "input": "secret"
        });

        let response = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/workloads/execute")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        upstream_task.abort();
        let _ = fs::remove_dir_all(receipt_dir).await;
    }

    #[tokio::test]
    async fn openai_compatible_route_returns_provider_response_and_receipt_header() {
        let (upstream_url, upstream_task) = mock_provider().await;
        let state = test_state(test_provider(&upstream_url), test_receipt_dir(), None);
        let payload = json!({
            "model": "mock-model",
            "messages": [{"role":"user","content":"Hello"}],
            "stream": false
        });

        let response = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().contains_key("x-wx-receipt-id"));
        upstream_task.abort();
    }

    #[test]
    fn acceptance_checks_do_not_claim_semantic_truth() {
        let result = evaluate_acceptance(
            &[
                "non_empty_output".to_owned(),
                "domain_correctness".to_owned(),
            ],
            "Some output",
        );
        assert_eq!(result.status, AcceptanceStatus::NeedsDefinition);
        assert!(result.checks.iter().any(|check| check.passed.is_none()));
    }
}
