//! A2A (Agent-to-Agent) protocol endpoints for cross-system task delegation.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use agnosai_core::{CrewSpec, Task};

use crate::state::SharedState;

/// A2A task delegation request — matches Agnostic v1 format.
#[derive(Debug, Deserialize)]
pub struct A2ARequest {
    pub task_id: String,
    pub description: String,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub size: Option<String>, // "lean", "standard", "large"
    #[serde(default)]
    pub preset: Option<String>,
    #[serde(default)]
    pub callback_url: Option<String>, // webhook to POST results back
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct A2AResponse {
    pub task_id: String,
    pub status: String, // "accepted", "completed", "failed"
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// POST /api/v1/a2a/receive — Accept an A2A task delegation.
///
/// Builds a single-task crew from the request, runs it through the orchestrator,
/// and returns the result. If `callback_url` is set, spawns a background task
/// to POST results back (fire-and-forget).
pub async fn receive(
    State(state): State<SharedState>,
    Json(req): Json<A2ARequest>,
) -> (StatusCode, Json<A2AResponse>) {
    let task_id = req.task_id.clone();

    // Build a simple crew with one task and default (empty) agent list.
    let crew_name = format!(
        "a2a-{}-{}",
        req.domain.as_deref().unwrap_or("general"),
        &task_id
    );
    let mut spec = CrewSpec::new(crew_name);
    let task = Task::new(&req.description);
    spec.tasks = vec![task];

    match state.orchestrator.run_crew(spec).await {
        Ok(crew_state) => {
            let result_data: Vec<serde_json::Value> = crew_state
                .results
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "task_id": r.task_id.to_string(),
                        "output": r.output,
                    })
                })
                .collect();

            let response = A2AResponse {
                task_id: task_id.clone(),
                status: "completed".to_string(),
                result: Some(serde_json::json!({ "tasks": result_data })),
                error: None,
            };

            // Fire-and-forget callback if URL is provided.
            if let Some(url) = req.callback_url {
                let resp_clone = response.clone();
                tokio::spawn(async move {
                    let client = reqwest::Client::new();
                    if let Err(e) = client.post(&url).json(&resp_clone).send().await {
                        tracing::warn!(task_id = %task_id, url = %url, "A2A callback failed: {e}");
                    }
                });
            }

            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            let response = A2AResponse {
                task_id,
                status: "failed".to_string(),
                result: None,
                error: Some(e.to_string()),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// POST /api/v1/a2a/status — Check status of a delegated task (placeholder).
pub async fn status() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "not_implemented"}))
}

#[cfg(test)]
mod tests {
    use crate::state::{AppState, SharedState};
    use agnosai_orchestrator::Orchestrator;
    use agnosai_tools::ToolRegistry;
    use axum::http::{Request, StatusCode};
    use axum::Router;
    use std::sync::Arc;
    use tower::ServiceExt;

    async fn test_app() -> Router {
        let orchestrator = Orchestrator::new(Default::default()).await.unwrap();
        let tools = Arc::new(ToolRegistry::new());
        let state: SharedState = Arc::new(AppState {
            orchestrator,
            tools,
        });
        crate::router(state)
    }

    #[tokio::test]
    async fn a2a_receive_with_valid_request_returns_completed() {
        let app = test_app().await;
        let body = serde_json::json!({
            "task_id": "ext-123",
            "description": "Analyse the login flow",
            "domain": "quality",
            "size": "lean",
            "metadata": {"source": "secureyeoman"}
        });
        let response = app
            .oneshot(
                Request::post("/api/v1/a2a/receive")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["task_id"], "ext-123");
        assert_eq!(json["status"], "completed");
        assert!(json["result"].is_object());
    }

    #[tokio::test]
    async fn a2a_receive_minimal_request() {
        let app = test_app().await;
        let body = serde_json::json!({
            "task_id": "min-1",
            "description": "Hello world"
        });
        let response = app
            .oneshot(
                Request::post("/api/v1/a2a/receive")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["task_id"], "min-1");
        assert_eq!(json["status"], "completed");
    }
}
