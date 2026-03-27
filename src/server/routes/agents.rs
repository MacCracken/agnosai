use axum::Json;
use axum::http::StatusCode;
use serde_json::Value;

use crate::core::AgentDefinition;

/// GET /api/v1/agents/definitions — List all agent definitions.
pub async fn list_definitions() -> Json<Vec<Value>> {
    // Placeholder — return empty array.
    Json(vec![])
}

/// POST /api/v1/agents/definitions — Create a new agent definition.
pub async fn create_definition(
    Json(def): Json<AgentDefinition>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let value = serde_json::to_value(&def).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("serialization failed: {e}")})),
        )
    })?;
    Ok((StatusCode::CREATED, Json(value)))
}

#[cfg(test)]
mod tests {
    use crate::orchestrator::Orchestrator;
    use crate::server::sse::EventBus;
    use crate::server::state::{AppState, SharedState};
    use crate::tools::ToolRegistry;
    use axum::Router;
    use axum::body::Body;
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
            events: EventBus::new(),
            http_client: reqwest::Client::new(),
            audit: std::sync::Arc::new(crate::llm::AuditChain::new(b"test-key", 100)),
            approval_gate: Default::default(),
        });
        crate::server::router(state)
    }

    #[tokio::test]
    async fn list_definitions_returns_empty_array() {
        let app = test_app().await;
        let resp = app
            .oneshot(
                Request::get("/api/v1/agents/definitions")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(json.is_array());
    }

    #[tokio::test]
    async fn create_definition_returns_201() {
        let app = test_app().await;
        let body = serde_json::json!({
            "agent_key": "test-agent",
            "name": "Test Agent",
            "role": "tester",
            "goal": "test things"
        });
        let resp = app
            .oneshot(
                Request::post("/api/v1/agents/definitions")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["agent_key"], "test-agent");
    }

    #[tokio::test]
    async fn create_definition_rejects_invalid_body() {
        let app = test_app().await;
        let resp = app
            .oneshot(
                Request::post("/api/v1/agents/definitions")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"not":"valid"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn create_definition_with_malformed_json_returns_error() {
        let app = test_app().await;
        let resp = app
            .oneshot(
                Request::post("/api/v1/agents/definitions")
                    .header("content-type", "application/json")
                    .body(Body::from("this is not json at all"))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = resp.status();
        assert!(
            status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY,
            "expected 400 or 422 for malformed JSON, got {status}"
        );
    }

    #[tokio::test]
    async fn create_definition_missing_required_fields_returns_422() {
        let app = test_app().await;
        // Provide valid JSON but missing required fields (e.g. no "role" or "goal").
        let body = serde_json::json!({
            "agent_key": "incomplete-agent",
            "name": "Incomplete"
        });
        let resp = app
            .oneshot(
                Request::post("/api/v1/agents/definitions")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn list_definitions_returns_empty_array_with_correct_content_type() {
        let app = test_app().await;
        let resp = app
            .oneshot(
                Request::get("/api/v1/agents/definitions")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let content_type = resp
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(
            content_type.contains("application/json"),
            "expected application/json, got {content_type}"
        );
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let arr = json.as_array().expect("should be an array");
        assert!(arr.is_empty());
    }
}
