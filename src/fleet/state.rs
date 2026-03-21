//! Distributed crew state with barrier sync and checkpoints.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::registry::NodeId;

/// Unique identifier for a crew run.
pub type CrewRunId = Uuid;

/// Phase of a distributed crew run.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CrewPhase {
    Initializing,
    Running,
    WaitingBarrier(String),
    Checkpointing,
    Completed,
    Failed(String),
    Cancelled,
}

/// Full state of a distributed crew run.
#[derive(Debug, Clone)]
pub struct DistributedCrewState {
    pub run_id: CrewRunId,
    pub phase: CrewPhase,
    pub participating_nodes: HashSet<NodeId>,
    pub node_progress: HashMap<NodeId, NodeProgress>,
    pub checkpoints: Vec<Checkpoint>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Progress report from a single node.
#[derive(Debug, Clone)]
pub struct NodeProgress {
    pub node_id: NodeId,
    pub tasks_completed: usize,
    pub tasks_total: usize,
    pub current_task: Option<String>,
    pub last_update: DateTime<Utc>,
}

/// A snapshot of node states at a point in time.
#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub node_states: HashMap<NodeId, serde_json::Value>,
}

/// Result of a barrier synchronisation attempt.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BarrierResult {
    /// Still waiting; value is the number of nodes that have not yet reached the barrier.
    Waiting(usize),
    /// All participating nodes have reached the barrier.
    AllReached,
    /// The run ID was not found.
    UnknownRun,
    /// The node is not a participant in this run.
    UnknownNode,
}

/// Manages state for distributed crew runs.
pub struct CrewStateManager {
    states: HashMap<CrewRunId, DistributedCrewState>,
    /// Per-run, per-barrier tracking of which nodes have arrived.
    barriers: HashMap<CrewRunId, HashMap<String, HashSet<NodeId>>>,
}

impl CrewStateManager {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
            barriers: HashMap::new(),
        }
    }

    /// Create a new distributed crew run. Each node starts with `tasks_per_node` tasks.
    pub fn create_run(&mut self, nodes: HashSet<NodeId>, tasks_per_node: usize) -> CrewRunId {
        let run_id = Uuid::new_v4();
        let now = Utc::now();

        let node_progress: HashMap<NodeId, NodeProgress> = nodes
            .iter()
            .map(|id| {
                (
                    id.clone(),
                    NodeProgress {
                        node_id: id.clone(),
                        tasks_completed: 0,
                        tasks_total: tasks_per_node,
                        current_task: None,
                        last_update: now,
                    },
                )
            })
            .collect();

        let state = DistributedCrewState {
            run_id,
            phase: CrewPhase::Initializing,
            participating_nodes: nodes,
            node_progress,
            checkpoints: Vec::new(),
            created_at: now,
            updated_at: now,
        };

        self.states.insert(run_id, state);
        run_id
    }

    /// Get the state of a run.
    pub fn get(&self, run_id: CrewRunId) -> Option<&DistributedCrewState> {
        self.states.get(&run_id)
    }

    /// Update progress for a node. Returns `true` if the update was applied.
    pub fn report_progress(
        &mut self,
        run_id: CrewRunId,
        node_id: NodeId,
        tasks_completed: usize,
        current_task: Option<String>,
    ) -> bool {
        let Some(state) = self.states.get_mut(&run_id) else {
            return false;
        };
        let Some(progress) = state.node_progress.get_mut(&node_id) else {
            return false;
        };

        progress.tasks_completed = tasks_completed;
        progress.current_task = current_task;
        progress.last_update = Utc::now();

        // Move from Initializing to Running on first progress report.
        if state.phase == CrewPhase::Initializing {
            state.phase = CrewPhase::Running;
        }
        state.updated_at = Utc::now();
        true
    }

    /// Signal that a node has reached a named barrier point.
    pub fn reach_barrier(
        &mut self,
        run_id: CrewRunId,
        node_id: NodeId,
        barrier_name: &str,
    ) -> BarrierResult {
        let Some(state) = self.states.get_mut(&run_id) else {
            return BarrierResult::UnknownRun;
        };
        if !state.participating_nodes.contains(&node_id) {
            return BarrierResult::UnknownNode;
        }

        let run_barriers = self.barriers.entry(run_id).or_default();
        let arrived = run_barriers.entry(barrier_name.to_string()).or_default();
        arrived.insert(node_id);

        let total = state.participating_nodes.len();
        let reached = arrived.len();

        if reached >= total {
            // All nodes arrived — transition phase back to Running.
            state.phase = CrewPhase::Running;
            state.updated_at = Utc::now();
            BarrierResult::AllReached
        } else {
            state.phase = CrewPhase::WaitingBarrier(barrier_name.to_string());
            state.updated_at = Utc::now();
            BarrierResult::Waiting(total - reached)
        }
    }

    /// Force a barrier to complete, even if not all nodes have arrived.
    ///
    /// Use when a node has been detected as dead and the barrier would otherwise
    /// deadlock. The caller should remove the dead node from `participating_nodes`
    /// first or accept that the barrier completes with fewer nodes.
    pub fn force_barrier(&mut self, run_id: CrewRunId, barrier_name: &str) -> bool {
        let Some(state) = self.states.get_mut(&run_id) else {
            return false;
        };
        // Clean up barrier tracking.
        if let Some(run_barriers) = self.barriers.get_mut(&run_id) {
            run_barriers.remove(barrier_name);
        }
        state.phase = CrewPhase::Running;
        state.updated_at = Utc::now();
        true
    }

    /// Remove a node from a run's participating set (e.g. after detecting it as dead).
    pub fn remove_node(&mut self, run_id: CrewRunId, node_id: &NodeId) -> bool {
        let Some(state) = self.states.get_mut(&run_id) else {
            return false;
        };
        state.participating_nodes.retain(|n| n != node_id);
        state.updated_at = Utc::now();
        true
    }

    /// Create a checkpoint of the current state.
    pub fn checkpoint(
        &mut self,
        run_id: CrewRunId,
        name: &str,
        node_states: HashMap<NodeId, serde_json::Value>,
    ) -> bool {
        let Some(state) = self.states.get_mut(&run_id) else {
            return false;
        };

        let prev_phase = state.phase.clone();
        state.phase = CrewPhase::Checkpointing;

        state.checkpoints.push(Checkpoint {
            name: name.to_string(),
            created_at: Utc::now(),
            node_states,
        });

        // Restore previous phase (unless it was Initializing, in which case go to Running).
        state.phase = match prev_phase {
            CrewPhase::Initializing => CrewPhase::Running,
            other => other,
        };
        state.updated_at = Utc::now();
        true
    }

    /// Mark a run as completed.
    pub fn complete(&mut self, run_id: CrewRunId) -> bool {
        let Some(state) = self.states.get_mut(&run_id) else {
            return false;
        };
        state.phase = CrewPhase::Completed;
        state.updated_at = Utc::now();
        true
    }

    /// Mark a run as failed.
    pub fn fail(&mut self, run_id: CrewRunId, reason: String) -> bool {
        let Some(state) = self.states.get_mut(&run_id) else {
            return false;
        };
        state.phase = CrewPhase::Failed(reason);
        state.updated_at = Utc::now();
        true
    }

    /// Cancel a run.
    pub fn cancel(&mut self, run_id: CrewRunId) -> bool {
        let Some(state) = self.states.get_mut(&run_id) else {
            return false;
        };
        state.phase = CrewPhase::Cancelled;
        state.updated_at = Utc::now();
        true
    }

    /// List all runs that are not in a terminal state.
    pub fn active_runs(&self) -> Vec<CrewRunId> {
        self.states
            .values()
            .filter(|s| {
                !matches!(
                    s.phase,
                    CrewPhase::Completed | CrewPhase::Failed(_) | CrewPhase::Cancelled
                )
            })
            .map(|s| s.run_id)
            .collect()
    }

    /// Overall progress as a fraction (0.0–1.0). Returns `None` for unknown runs.
    pub fn overall_progress(&self, run_id: CrewRunId) -> Option<f64> {
        let state = self.states.get(&run_id)?;
        let mut completed: usize = 0;
        let mut total: usize = 0;
        for p in state.node_progress.values() {
            completed += p.tasks_completed;
            total += p.tasks_total;
        }
        if total == 0 {
            return Some(1.0);
        }
        Some(completed as f64 / total as f64)
    }
}

impl Default for CrewStateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nodes(ids: &[&str]) -> HashSet<NodeId> {
        ids.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn create_run_initial_state() {
        let mut mgr = CrewStateManager::new();
        let run = mgr.create_run(nodes(&["a", "b"]), 5);
        let state = mgr.get(run).unwrap();

        assert_eq!(state.phase, CrewPhase::Initializing);
        assert_eq!(state.participating_nodes.len(), 2);
        assert_eq!(state.node_progress.len(), 2);
        assert_eq!(state.node_progress["a"].tasks_total, 5);
        assert_eq!(state.node_progress["a"].tasks_completed, 0);
        assert!(state.checkpoints.is_empty());
    }

    #[test]
    fn report_progress_updates_node() {
        let mut mgr = CrewStateManager::new();
        let run = mgr.create_run(nodes(&["a"]), 10);

        assert!(mgr.report_progress(run, "a".into(), 3, Some("task-1".into())));

        let state = mgr.get(run).unwrap();
        assert_eq!(state.phase, CrewPhase::Running);
        assert_eq!(state.node_progress["a"].tasks_completed, 3);
        assert_eq!(
            state.node_progress["a"].current_task.as_deref(),
            Some("task-1")
        );
    }

    #[test]
    fn report_progress_unknown_run() {
        let mut mgr = CrewStateManager::new();
        assert!(!mgr.report_progress(Uuid::new_v4(), "a".into(), 1, None));
    }

    #[test]
    fn report_progress_unknown_node() {
        let mut mgr = CrewStateManager::new();
        let run = mgr.create_run(nodes(&["a"]), 5);
        assert!(!mgr.report_progress(run, "z".into(), 1, None));
    }

    #[test]
    fn barrier_first_node_waits() {
        let mut mgr = CrewStateManager::new();
        let run = mgr.create_run(nodes(&["a", "b"]), 5);

        let result = mgr.reach_barrier(run, "a".into(), "sync-1");
        assert_eq!(result, BarrierResult::Waiting(1));
    }

    #[test]
    fn barrier_last_node_triggers_all_reached() {
        let mut mgr = CrewStateManager::new();
        let run = mgr.create_run(nodes(&["a", "b"]), 5);

        mgr.reach_barrier(run, "a".into(), "sync-1");
        let result = mgr.reach_barrier(run, "b".into(), "sync-1");
        assert_eq!(result, BarrierResult::AllReached);
    }

    #[test]
    fn barrier_three_nodes_progressive() {
        let mut mgr = CrewStateManager::new();
        let run = mgr.create_run(nodes(&["a", "b", "c"]), 5);

        assert_eq!(
            mgr.reach_barrier(run, "a".into(), "sync"),
            BarrierResult::Waiting(2)
        );
        assert_eq!(
            mgr.reach_barrier(run, "b".into(), "sync"),
            BarrierResult::Waiting(1)
        );
        assert_eq!(
            mgr.reach_barrier(run, "c".into(), "sync"),
            BarrierResult::AllReached
        );
    }

    #[test]
    fn barrier_unknown_run() {
        let mut mgr = CrewStateManager::new();
        assert_eq!(
            mgr.reach_barrier(Uuid::new_v4(), "a".into(), "x"),
            BarrierResult::UnknownRun
        );
    }

    #[test]
    fn barrier_unknown_node() {
        let mut mgr = CrewStateManager::new();
        let run = mgr.create_run(nodes(&["a"]), 5);
        assert_eq!(
            mgr.reach_barrier(run, "z".into(), "x"),
            BarrierResult::UnknownNode
        );
    }

    #[test]
    fn checkpoint_stores_node_states() {
        let mut mgr = CrewStateManager::new();
        let run = mgr.create_run(nodes(&["a"]), 5);

        let mut ns = HashMap::new();
        ns.insert("a".to_string(), serde_json::json!({"step": 3}));

        assert!(mgr.checkpoint(run, "cp-1", ns));

        let state = mgr.get(run).unwrap();
        assert_eq!(state.checkpoints.len(), 1);
        assert_eq!(state.checkpoints[0].name, "cp-1");
        assert_eq!(state.checkpoints[0].node_states["a"]["step"], 3);
    }

    #[test]
    fn checkpoint_unknown_run() {
        let mut mgr = CrewStateManager::new();
        assert!(!mgr.checkpoint(Uuid::new_v4(), "cp", HashMap::new()));
    }

    #[test]
    fn complete_transition() {
        let mut mgr = CrewStateManager::new();
        let run = mgr.create_run(nodes(&["a"]), 5);
        assert!(mgr.complete(run));
        assert_eq!(mgr.get(run).unwrap().phase, CrewPhase::Completed);
    }

    #[test]
    fn fail_transition() {
        let mut mgr = CrewStateManager::new();
        let run = mgr.create_run(nodes(&["a"]), 5);
        assert!(mgr.fail(run, "oom".into()));
        assert_eq!(mgr.get(run).unwrap().phase, CrewPhase::Failed("oom".into()));
    }

    #[test]
    fn cancel_transition() {
        let mut mgr = CrewStateManager::new();
        let run = mgr.create_run(nodes(&["a"]), 5);
        assert!(mgr.cancel(run));
        assert_eq!(mgr.get(run).unwrap().phase, CrewPhase::Cancelled);
    }

    #[test]
    fn active_runs_excludes_terminal() {
        let mut mgr = CrewStateManager::new();
        let r1 = mgr.create_run(nodes(&["a"]), 5);
        let r2 = mgr.create_run(nodes(&["b"]), 5);
        let r3 = mgr.create_run(nodes(&["c"]), 5);

        mgr.complete(r1);
        mgr.fail(r2, "err".into());

        let active = mgr.active_runs();
        assert_eq!(active.len(), 1);
        assert!(active.contains(&r3));
    }

    #[test]
    fn overall_progress_calculation() {
        let mut mgr = CrewStateManager::new();
        let run = mgr.create_run(nodes(&["a", "b"]), 10);

        // a: 5/10, b: 0/10 => 5/20 = 0.25
        mgr.report_progress(run, "a".into(), 5, None);
        let pct = mgr.overall_progress(run).unwrap();
        assert!((pct - 0.25).abs() < 1e-9);

        // a: 10/10, b: 10/10 => 1.0
        mgr.report_progress(run, "a".into(), 10, None);
        mgr.report_progress(run, "b".into(), 10, None);
        let pct = mgr.overall_progress(run).unwrap();
        assert!((pct - 1.0).abs() < 1e-9);
    }

    #[test]
    fn overall_progress_unknown_run() {
        let mgr = CrewStateManager::new();
        assert!(mgr.overall_progress(Uuid::new_v4()).is_none());
    }
}
