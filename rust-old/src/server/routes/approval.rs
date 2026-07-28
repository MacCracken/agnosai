//! HTTP endpoints for human-in-the-loop approval gates.

use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};

use crate::orchestrator::approval::ApprovalDecision;
use crate::server::state::SharedState;

/// Request body for submitting an approval decision.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct ApprovalSubmission {
    /// Task ID to approve or reject.
    pub task_id: uuid::Uuid,
    /// The decision: "approved" or "rejected".
    pub decision: ApprovalDecision,
}

/// Response after submitting a decision.
#[derive(Debug, Serialize)]
#[non_exhaustive]
pub struct ApprovalResponse {
    /// Whether the decision was delivered to a pending approval.
    pub delivered: bool,
    /// Human-readable status message.
    pub message: String,
}

/// POST /api/v1/approvals — Submit an approval decision for a pending task.
#[tracing::instrument(skip(state))]
pub async fn submit_approval(
    State(state): State<SharedState>,
    Json(body): Json<ApprovalSubmission>,
) -> Json<ApprovalResponse> {
    let delivered = state
        .approval_gate
        .submit_decision(body.task_id, body.decision);

    let message = if delivered {
        format!(
            "Decision {:?} delivered for task {}",
            body.decision, body.task_id
        )
    } else {
        format!("No pending approval found for task {}", body.task_id)
    };

    Json(ApprovalResponse { delivered, message })
}

/// GET /api/v1/approvals — List pending approval task IDs.
pub async fn list_pending(State(state): State<SharedState>) -> Json<Vec<uuid::Uuid>> {
    Json(state.approval_gate.pending_tasks())
}

#[cfg(test)]
mod tests {
    use crate::orchestrator::Orchestrator;
    use crate::server::state::{AppState, SharedState};
    use crate::tools::ToolRegistry;
    use axum::Router;
    use axum::http::{Request, StatusCode};
    use serde_json::{Value, json};
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
            audit: Arc::new(crate::llm::AuditChain::new(b"test-key", 100)),
            approval_gate: Default::default(),
            definitions: dashmap::DashMap::new(),
        });
        crate::server::router(state)
    }

    #[tokio::test]
    async fn submit_approval_unknown_task() {
        let app = test_app().await;
        let body = json!({
            "task_id": "00000000-0000-0000-0000-000000000001",
            "decision": "approved"
        });
        let response = app
            .oneshot(
                Request::post("/api/v1/approvals")
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
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["delivered"], false);
    }

    #[tokio::test]
    async fn list_pending_empty() {
        let app = test_app().await;
        let response = app
            .oneshot(
                Request::get("/api/v1/approvals")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        let arr = json.as_array().unwrap();
        assert!(arr.is_empty());
    }
}
