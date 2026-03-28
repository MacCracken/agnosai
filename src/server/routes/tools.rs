use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::server::state::SharedState;
use crate::tools::ToolSchema;

/// GET /api/v1/tools — List all registered tools.
pub async fn list_tools(State(state): State<SharedState>) -> Json<Vec<ToolSchema>> {
    Json(state.tools.list())
}

/// DELETE /api/v1/tools/{name} — Unregister a tool at runtime.
pub async fn remove_tool(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    if state.tools.remove(&name) {
        tracing::info!(tool = %name, "tool unregistered");
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

#[cfg(test)]
mod tests {
    use crate::orchestrator::Orchestrator;
    use crate::server::state::{AppState, SharedState};
    use crate::tools::ToolRegistry;
    use crate::tools::builtin::echo::EchoTool;
    use crate::tools::builtin::json_transform::JsonTransformTool;
    use axum::Router;
    use axum::http::{Request, StatusCode};
    use std::sync::Arc;
    use tower::ServiceExt;

    async fn test_app() -> Router {
        let orchestrator = Orchestrator::new(Default::default()).await.unwrap();
        let tools = Arc::new(ToolRegistry::new());
        tools.register(Arc::new(EchoTool));
        tools.register(Arc::new(JsonTransformTool));
        let state: SharedState = Arc::new(AppState {
            orchestrator,
            tools,
            auth: Default::default(),
            events: crate::server::sse::EventBus::new(),
            http_client: reqwest::Client::new(),
            audit: std::sync::Arc::new(crate::llm::AuditChain::new(b"test-key", 100)),
            approval_gate: Default::default(),
        });
        crate::server::router(state)
    }

    #[tokio::test]
    async fn get_tools_returns_tool_list() {
        let app = test_app().await;
        let response = app
            .oneshot(
                Request::get("/api/v1/tools")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        let names: Vec<&str> = arr.iter().filter_map(|v| v["name"].as_str()).collect();
        assert!(names.contains(&"echo"));
        assert!(names.contains(&"json_transform"));
    }

    #[tokio::test]
    async fn get_tools_returns_correct_schema_fields() {
        let app = test_app().await;
        let response = app
            .oneshot(
                Request::get("/api/v1/tools")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let arr = json.as_array().unwrap();

        for tool in arr {
            assert!(
                tool.get("name").is_some(),
                "each tool should have a 'name' field"
            );
            assert!(
                tool.get("description").is_some(),
                "each tool should have a 'description' field"
            );
            assert!(
                tool.get("parameters").is_some(),
                "each tool should have a 'parameters' field"
            );
            assert!(
                tool["name"].as_str().is_some_and(|s| !s.is_empty()),
                "tool name should be non-empty"
            );
            assert!(
                tool["description"].as_str().is_some_and(|s| !s.is_empty()),
                "tool description should be non-empty"
            );
        }
    }

    #[tokio::test]
    async fn get_tools_with_no_tools_returns_empty_array() {
        let orchestrator = Orchestrator::new(Default::default()).await.unwrap();
        let tools = Arc::new(ToolRegistry::new()); // no tools registered
        let state: SharedState = Arc::new(AppState {
            orchestrator,
            tools,
            auth: Default::default(),
            events: crate::server::sse::EventBus::new(),
            http_client: reqwest::Client::new(),
            audit: std::sync::Arc::new(crate::llm::AuditChain::new(b"test-key", 100)),
            approval_gate: Default::default(),
        });
        let app = crate::server::router(state);

        let response = app
            .oneshot(
                Request::get("/api/v1/tools")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let arr = json.as_array().expect("response should be a JSON array");
        assert!(arr.is_empty(), "expected empty array, got {arr:?}");
    }
}
