use axum::extract::State;
use axum::Json;
use agnosai_tools::ToolSchema;

use crate::state::SharedState;

pub async fn list_tools(State(state): State<SharedState>) -> Json<Vec<ToolSchema>> {
    Json(state.tools.list())
}

#[cfg(test)]
mod tests {
    use crate::state::{AppState, SharedState};
    use agnosai_orchestrator::Orchestrator;
    use agnosai_tools::builtin::echo::EchoTool;
    use agnosai_tools::builtin::json_transform::JsonTransformTool;
    use agnosai_tools::ToolRegistry;
    use axum::http::{Request, StatusCode};
    use axum::Router;
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
        });
        crate::router(state)
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
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        let names: Vec<&str> = arr.iter().filter_map(|v| v["name"].as_str()).collect();
        assert!(names.contains(&"echo"));
        assert!(names.contains(&"json_transform"));
    }
}
