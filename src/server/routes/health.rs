use axum::Json;
use serde_json::{Value, json};

/// GET /health — Liveness probe.
pub async fn health() -> Json<Value> {
    Json(json!({"status": "ok"}))
}

/// GET /ready — Readiness probe.
pub async fn ready() -> Json<Value> {
    Json(json!({"status": "ready", "version": "0.1.0"}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::Orchestrator;
    use crate::server::state::{AppState, SharedState};
    use crate::tools::ToolRegistry;
    use axum::Router;
    use axum::http::{Request, StatusCode};
    use std::sync::Arc;
    use tower::ServiceExt;

    async fn test_app() -> Router {
        let orchestrator = Orchestrator::new(Default::default()).await.unwrap();
        let tools = Arc::new(ToolRegistry::new());
        let state: SharedState = Arc::new(AppState {
            orchestrator,
            tools,
            auth: Default::default(),
            events: crate::server::sse::EventBus::new(),
            http_client: reqwest::Client::new(),
        });
        crate::server::router(state)
    }

    #[tokio::test]
    async fn get_health_returns_200() {
        let app = test_app().await;
        let response = app
            .oneshot(
                Request::get("/health")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    async fn get_ready_returns_200_with_version() {
        let app = test_app().await;
        let response = app
            .oneshot(
                Request::get("/ready")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ready");
        assert_eq!(json["version"], "0.1.0");
    }
}
