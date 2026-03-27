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
