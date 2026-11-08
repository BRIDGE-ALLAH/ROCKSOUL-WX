use axum::{
    Json, Router,
    extract::rejection::JsonRejection,
    http::StatusCode,
    routing::{get, post},
};
use serde::Serialize;
use tracing_subscriber::EnvFilter;
use wx_contract::{ExecutionPlan, WorkloadContract};
use wx_policy::PolicyViolation;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    mode: &'static str,
}

#[derive(Serialize)]
struct ApiError {
    error: &'static str,
    message: String,
    violations: Vec<PolicyViolation>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("wx_gateway=info,tower_http=info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let address =
        std::env::var("WX_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_owned());
    let listener = tokio::net::TcpListener::bind(&address).await?;

    tracing::info!(%address, "ROCKSOUL-WX planning gateway listening");
    axum::serve(listener, app()).await?;
    Ok(())
}

fn app() -> Router {
    Router::new()
        .route("/healthz", get(health))
        .route("/v1/workloads/plan", post(plan_workload))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "wx-gateway",
        mode: "planning-only",
    })
}

async fn plan_workload(
    payload: Result<Json<WorkloadContract>, JsonRejection>,
) -> Result<Json<ExecutionPlan>, (StatusCode, Json<ApiError>)> {
    let Json(contract) = payload.map_err(|error| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                error: "invalid_json",
                message: error.body_text(),
                violations: Vec::new(),
            }),
        )
    })?;

    wx_planner::plan(contract).map(Json).map_err(|violations| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiError {
                error: "workload_policy_rejected",
                message: "The workload contract did not pass deterministic policy checks."
                    .to_owned(),
                violations,
            }),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::{Body, to_bytes}, http::Request};
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_endpoint_is_available() {
        let response = app()
            .oneshot(Request::builder().uri("/healthz").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn planner_returns_a_plan_for_a_local_workload() {
        let payload = r#"{
            "objective": "Extract a date from a local document",
            "data_classification": "public",
            "raw_data_egress": "deny",
            "allowed_execution": ["local"]
        }"#;
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/workloads/plan")
                    .header("content-type", "application/json")
                    .body(Body::from(payload))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let plan: ExecutionPlan = serde_json::from_slice(&body).unwrap();
        assert_eq!(plan.strategy, wx_contract::ExecutionStrategy::LocalOnly);
    }

    #[tokio::test]
    async fn planner_rejects_sensitive_public_cloud_workloads() {
        let payload = r#"{
            "objective": "Review a confidential contract",
            "data_classification": "confidential",
            "raw_data_egress": "deny",
            "allowed_execution": ["local", "public_cloud"]
        }"#;
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/workloads/plan")
                    .header("content-type", "application/json")
                    .body(Body::from(payload))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}
