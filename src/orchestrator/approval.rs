//! Human-in-the-loop approval gates for task results.
//!
//! When a task has `TaskRisk::High` (or `Medium` with approval required),
//! the crew runner suspends after receiving the LLM response and waits for
//! a human to approve or reject it via an HTTP callback.
//!
//! The `ApprovalGate` manages pending approvals using a DashMap of oneshot
//! channels keyed by task ID.

use std::time::Duration;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use tracing::{debug, info, warn};

use crate::core::task::{TaskId, TaskRisk};

/// Default timeout for waiting on human approval (5 minutes).
const DEFAULT_APPROVAL_TIMEOUT: Duration = Duration::from_secs(300);

/// Maximum number of pending approvals before rejecting new ones.
const MAX_PENDING_APPROVALS: usize = 1_000;

/// Decision made by a human reviewer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ApprovalDecision {
    /// The task result is approved and can proceed.
    Approved,
    /// The task result is rejected.  The crew runner will mark the task as failed.
    Rejected,
}

/// A pending approval request emitted as an SSE event.
#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct ApprovalRequest {
    /// Task awaiting approval.
    pub task_id: TaskId,
    /// The LLM output that needs review.
    pub output: String,
    /// Risk level of the task.
    pub risk: TaskRisk,
    /// Agent that produced the output (if any).
    pub agent: Option<String>,
}

/// Manages pending human approval gates.
///
/// Each pending approval is a oneshot channel: the crew runner holds the
/// receiver (waiting), and the HTTP endpoint sends the decision.
pub struct ApprovalGate {
    pending: DashMap<TaskId, oneshot::Sender<ApprovalDecision>>,
    timeout: Duration,
}

impl ApprovalGate {
    /// Create a new approval gate with the default timeout.
    pub fn new() -> Self {
        Self {
            pending: DashMap::new(),
            timeout: DEFAULT_APPROVAL_TIMEOUT,
        }
    }

    /// Create a new approval gate with a custom timeout.
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            pending: DashMap::new(),
            timeout,
        }
    }

    /// Check whether a task requires human approval.
    #[must_use]
    pub fn requires_approval(risk: TaskRisk, require_medium: bool) -> bool {
        match risk {
            TaskRisk::High => true,
            TaskRisk::Medium => require_medium,
            TaskRisk::Low => false,
        }
    }

    /// Register a pending approval and return a receiver that the crew runner
    /// can await.
    ///
    /// Returns `None` if the pending approval limit has been reached.
    pub fn request_approval(&self, task_id: TaskId) -> Option<oneshot::Receiver<ApprovalDecision>> {
        if self.pending.len() >= MAX_PENDING_APPROVALS {
            warn!(
                task_id = %task_id,
                "approval gate at capacity, auto-rejecting"
            );
            return None;
        }

        let (tx, rx) = oneshot::channel();
        self.pending.insert(task_id, tx);
        debug!(task_id = %task_id, "approval requested");
        Some(rx)
    }

    /// Submit a human decision for a pending approval.
    ///
    /// Returns `true` if the decision was delivered, `false` if no such
    /// pending approval exists (may have timed out or been cancelled).
    pub fn submit_decision(&self, task_id: TaskId, decision: ApprovalDecision) -> bool {
        if let Some((_, tx)) = self.pending.remove(&task_id) {
            info!(task_id = %task_id, decision = ?decision, "approval decision submitted");
            tx.send(decision).is_ok()
        } else {
            warn!(task_id = %task_id, "no pending approval found");
            false
        }
    }

    /// Wait for an approval decision with timeout.
    ///
    /// Returns `Approved` on success, `Rejected` on timeout or channel drop.
    pub async fn wait_for_decision(
        &self,
        rx: oneshot::Receiver<ApprovalDecision>,
        task_id: TaskId,
    ) -> ApprovalDecision {
        match tokio::time::timeout(self.timeout, rx).await {
            Ok(Ok(decision)) => decision,
            Ok(Err(_)) => {
                warn!(task_id = %task_id, "approval channel dropped, rejecting");
                ApprovalDecision::Rejected
            }
            Err(_) => {
                warn!(
                    task_id = %task_id,
                    timeout_secs = self.timeout.as_secs(),
                    "approval timed out, rejecting"
                );
                // Clean up the pending entry.
                self.pending.remove(&task_id);
                ApprovalDecision::Rejected
            }
        }
    }

    /// Number of currently pending approvals.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// List all pending task IDs.
    #[must_use]
    pub fn pending_tasks(&self) -> Vec<TaskId> {
        self.pending.iter().map(|e| *e.key()).collect()
    }

    /// Cancel a pending approval (removes it without sending a decision).
    pub fn cancel(&self, task_id: TaskId) -> bool {
        self.pending.remove(&task_id).is_some()
    }
}

impl Default for ApprovalGate {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn requires_approval_high_always() {
        assert!(ApprovalGate::requires_approval(TaskRisk::High, false));
        assert!(ApprovalGate::requires_approval(TaskRisk::High, true));
    }

    #[test]
    fn requires_approval_medium_configurable() {
        assert!(!ApprovalGate::requires_approval(TaskRisk::Medium, false));
        assert!(ApprovalGate::requires_approval(TaskRisk::Medium, true));
    }

    #[test]
    fn requires_approval_low_never() {
        assert!(!ApprovalGate::requires_approval(TaskRisk::Low, false));
        assert!(!ApprovalGate::requires_approval(TaskRisk::Low, true));
    }

    #[tokio::test]
    async fn approval_flow_approved() {
        let gate = ApprovalGate::new();
        let task_id = Uuid::new_v4();

        let rx = gate.request_approval(task_id).unwrap();
        assert_eq!(gate.pending_count(), 1);

        gate.submit_decision(task_id, ApprovalDecision::Approved);
        let decision = gate.wait_for_decision(rx, task_id).await;
        assert_eq!(decision, ApprovalDecision::Approved);
        assert_eq!(gate.pending_count(), 0);
    }

    #[tokio::test]
    async fn approval_flow_rejected() {
        let gate = ApprovalGate::new();
        let task_id = Uuid::new_v4();

        let rx = gate.request_approval(task_id).unwrap();
        gate.submit_decision(task_id, ApprovalDecision::Rejected);
        let decision = gate.wait_for_decision(rx, task_id).await;
        assert_eq!(decision, ApprovalDecision::Rejected);
    }

    #[tokio::test]
    async fn approval_timeout_rejects() {
        let gate = ApprovalGate::with_timeout(Duration::from_millis(10));
        let task_id = Uuid::new_v4();

        let rx = gate.request_approval(task_id).unwrap();
        // Don't submit a decision — let it time out.
        let decision = gate.wait_for_decision(rx, task_id).await;
        assert_eq!(decision, ApprovalDecision::Rejected);
    }

    #[test]
    fn submit_decision_for_unknown_task_returns_false() {
        let gate = ApprovalGate::new();
        assert!(!gate.submit_decision(Uuid::new_v4(), ApprovalDecision::Approved));
    }

    #[test]
    fn cancel_removes_pending() {
        let gate = ApprovalGate::new();
        let task_id = Uuid::new_v4();
        let _rx = gate.request_approval(task_id).unwrap();
        assert_eq!(gate.pending_count(), 1);
        assert!(gate.cancel(task_id));
        assert_eq!(gate.pending_count(), 0);
    }

    #[test]
    fn pending_tasks_lists_all() {
        let gate = ApprovalGate::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let _rx1 = gate.request_approval(id1).unwrap();
        let _rx2 = gate.request_approval(id2).unwrap();
        let tasks = gate.pending_tasks();
        assert_eq!(tasks.len(), 2);
        assert!(tasks.contains(&id1));
        assert!(tasks.contains(&id2));
    }
}
