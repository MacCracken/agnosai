//! Crew fan-out, result aggregation, and failover.

use std::collections::HashMap;

use uuid::Uuid;

use super::registry::NodeId;
use super::state::{CrewRunId, CrewStateManager};

/// A task managed by the fleet coordinator.
#[derive(Debug, Clone)]
pub struct FleetTask {
    pub task_id: Uuid,
    pub description: String,
    pub assigned_node: Option<NodeId>,
    pub status: FleetTaskStatus,
}

/// Status of a fleet task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FleetTaskStatus {
    Pending,
    Assigned,
    Running,
    Completed,
    Failed,
    /// Failed on one node and eligible for reassignment.
    Reassigned,
}

/// Action to take after a task failure.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FailoverAction {
    /// Task can be retried on another node.
    Retry,
    /// Maximum retries exhausted.
    Exhausted,
    /// Task ID not found.
    UnknownTask,
}

/// Coordinates fan-out, tracking, and failover for fleet tasks.
pub struct FleetCoordinator {
    state_manager: CrewStateManager,
    tasks: HashMap<Uuid, FleetTask>,
    max_retries: usize,
    retry_counts: HashMap<Uuid, usize>,
}

impl FleetCoordinator {
    pub fn new() -> Self {
        Self {
            state_manager: CrewStateManager::new(),
            tasks: HashMap::new(),
            max_retries: 3,
            retry_counts: HashMap::new(),
        }
    }

    pub fn with_max_retries(max_retries: usize) -> Self {
        Self {
            state_manager: CrewStateManager::new(),
            tasks: HashMap::new(),
            max_retries,
            retry_counts: HashMap::new(),
        }
    }

    /// Fan out tasks to nodes based on pre-computed placement assignments.
    ///
    /// Creates a crew run in the state manager and registers all tasks with their
    /// assigned nodes. Returns the run ID.
    pub fn fan_out(
        &mut self,
        tasks: Vec<(Uuid, String)>,
        assignments: Vec<(Uuid, NodeId)>,
    ) -> CrewRunId {
        // Build lookup from task_id -> node_id.
        let assignment_map: HashMap<Uuid, NodeId> = assignments.into_iter().collect();

        // Collect unique nodes and count tasks per node for the state manager.
        let mut nodes = std::collections::HashSet::new();
        let mut tasks_per_node: HashMap<NodeId, usize> = HashMap::new();

        for (task_id, description) in &tasks {
            let node = assignment_map.get(task_id).cloned();
            if let Some(ref n) = node {
                nodes.insert(n.clone());
                *tasks_per_node.entry(n.clone()).or_insert(0) += 1;
            }

            self.tasks.insert(
                *task_id,
                FleetTask {
                    task_id: *task_id,
                    description: description.clone(),
                    assigned_node: node,
                    status: if assignment_map.contains_key(task_id) {
                        FleetTaskStatus::Assigned
                    } else {
                        FleetTaskStatus::Pending
                    },
                },
            );
        }

        // Use the max tasks-per-node value so every node gets a slot in the state
        // manager (the state manager uses a uniform count).
        let max_tasks = tasks_per_node.values().copied().max().unwrap_or(0);
        self.state_manager.create_run(nodes, max_tasks)
    }

    /// Report a task as completed. Returns `true` if the task was found and updated.
    pub fn task_completed(&mut self, task_id: Uuid) -> bool {
        let Some(task) = self.tasks.get_mut(&task_id) else {
            return false;
        };
        task.status = FleetTaskStatus::Completed;
        true
    }

    /// Report a task as failed. If retries remain, marks the task as `Reassigned`
    /// and returns `Retry`. Otherwise returns `Exhausted`.
    pub fn task_failed(&mut self, task_id: Uuid) -> FailoverAction {
        let Some(task) = self.tasks.get_mut(&task_id) else {
            return FailoverAction::UnknownTask;
        };

        let count = self.retry_counts.entry(task_id).or_insert(0);
        *count += 1;

        if *count < self.max_retries {
            task.status = FleetTaskStatus::Reassigned;
            FailoverAction::Retry
        } else {
            task.status = FleetTaskStatus::Failed;
            FailoverAction::Exhausted
        }
    }

    /// Get all tasks assigned to a specific node.
    pub fn tasks_for_node(&self, node_id: NodeId) -> Vec<&FleetTask> {
        self.tasks
            .values()
            .filter(|t| t.assigned_node.as_ref() == Some(&node_id))
            .collect()
    }

    /// Returns `true` when every task is either `Completed` or terminally `Failed`.
    pub fn is_complete(&self) -> bool {
        if self.tasks.is_empty() {
            return true;
        }
        self.tasks.values().all(|t| {
            matches!(
                t.status,
                FleetTaskStatus::Completed | FleetTaskStatus::Failed
            )
        })
    }

    /// Fraction of tasks that are completed (0.0–1.0).
    pub fn completion_pct(&self) -> f64 {
        if self.tasks.is_empty() {
            return 1.0;
        }
        let completed = self
            .tasks
            .values()
            .filter(|t| t.status == FleetTaskStatus::Completed)
            .count();
        completed as f64 / self.tasks.len() as f64
    }

    /// List tasks that have been marked `Reassigned` (failed but retriable).
    pub fn pending_reassignment(&self) -> Vec<&FleetTask> {
        self.tasks
            .values()
            .filter(|t| t.status == FleetTaskStatus::Reassigned)
            .collect()
    }

    /// Reassign a task to a new node. Returns `true` if the task was found
    /// and successfully reassigned.
    pub fn reassign(&mut self, task_id: Uuid, new_node: NodeId) -> bool {
        let Some(task) = self.tasks.get_mut(&task_id) else {
            return false;
        };
        task.assigned_node = Some(new_node);
        task.status = FleetTaskStatus::Assigned;
        true
    }

    /// Read-only access to the underlying state manager.
    pub fn state_manager(&self) -> &CrewStateManager {
        &self.state_manager
    }

    /// Plan how to distribute a model across available devices.
    ///
    /// Uses `ai-hwaccel`'s sharding planner to determine the optimal strategy
    /// (pipeline parallel, tensor parallel, or no sharding) based on model size,
    /// quantization level, and available hardware.
    ///
    /// # Arguments
    /// * `model_params` — approximate parameter count (e.g. 70_000_000_000 for 70B)
    /// * `quant` — quantization level to use
    /// * `registry` — detected hardware
    ///
    /// Returns a `ShardingPlan` describing how to split the model.
    #[cfg(feature = "hwaccel")]
    pub fn plan_sharding(
        model_params: u64,
        quant: &ai_hwaccel::QuantizationLevel,
        registry: &ai_hwaccel::AcceleratorRegistry,
    ) -> ai_hwaccel::ShardingPlan {
        registry.plan_sharding(model_params, quant)
    }
}

impl Default for FleetCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tasks(n: usize) -> Vec<(Uuid, String)> {
        (0..n)
            .map(|i| (Uuid::new_v4(), format!("task-{i}")))
            .collect()
    }

    #[test]
    fn fan_out_creates_tasks_with_assignments() {
        let mut coord = FleetCoordinator::new();
        let tasks = make_tasks(3);
        let assignments: Vec<(Uuid, NodeId)> = vec![
            (tasks[0].0, "node-a".into()),
            (tasks[1].0, "node-b".into()),
            (tasks[2].0, "node-a".into()),
        ];

        let _run_id = coord.fan_out(tasks.clone(), assignments);

        assert_eq!(coord.tasks.len(), 3);
        for (id, _) in &tasks {
            let t = coord.tasks.get(id).unwrap();
            assert_eq!(t.status, FleetTaskStatus::Assigned);
            assert!(t.assigned_node.is_some());
        }
    }

    #[test]
    fn task_completed_transitions() {
        let mut coord = FleetCoordinator::new();
        let tasks = make_tasks(1);
        let assignments = vec![(tasks[0].0, "n".into())];
        coord.fan_out(tasks.clone(), assignments);

        assert!(coord.task_completed(tasks[0].0));
        assert_eq!(coord.tasks[&tasks[0].0].status, FleetTaskStatus::Completed);
    }

    #[test]
    fn task_completed_unknown() {
        let mut coord = FleetCoordinator::new();
        assert!(!coord.task_completed(Uuid::new_v4()));
    }

    #[test]
    fn task_failed_with_retries_returns_retry() {
        let mut coord = FleetCoordinator::with_max_retries(3);
        let tasks = make_tasks(1);
        coord.fan_out(tasks.clone(), vec![(tasks[0].0, "n".into())]);

        assert_eq!(coord.task_failed(tasks[0].0), FailoverAction::Retry);
        assert_eq!(coord.tasks[&tasks[0].0].status, FleetTaskStatus::Reassigned);
    }

    #[test]
    fn task_failed_exhausted_after_max_retries() {
        let mut coord = FleetCoordinator::with_max_retries(2);
        let tasks = make_tasks(1);
        coord.fan_out(tasks.clone(), vec![(tasks[0].0, "n".into())]);

        assert_eq!(coord.task_failed(tasks[0].0), FailoverAction::Retry);
        assert_eq!(coord.task_failed(tasks[0].0), FailoverAction::Exhausted);
        assert_eq!(coord.tasks[&tasks[0].0].status, FleetTaskStatus::Failed);
    }

    #[test]
    fn task_failed_unknown() {
        let mut coord = FleetCoordinator::new();
        assert_eq!(
            coord.task_failed(Uuid::new_v4()),
            FailoverAction::UnknownTask
        );
    }

    #[test]
    fn tasks_for_node_filters() {
        let mut coord = FleetCoordinator::new();
        let tasks = make_tasks(3);
        let assignments = vec![
            (tasks[0].0, "a".into()),
            (tasks[1].0, "b".into()),
            (tasks[2].0, "a".into()),
        ];
        coord.fan_out(tasks.clone(), assignments);

        let a_tasks = coord.tasks_for_node("a".into());
        assert_eq!(a_tasks.len(), 2);

        let b_tasks = coord.tasks_for_node("b".into());
        assert_eq!(b_tasks.len(), 1);

        let c_tasks = coord.tasks_for_node("c".into());
        assert_eq!(c_tasks.len(), 0);
    }

    #[test]
    fn is_complete_all_done() {
        let mut coord = FleetCoordinator::new();
        let tasks = make_tasks(2);
        coord.fan_out(
            tasks.clone(),
            vec![(tasks[0].0, "n".into()), (tasks[1].0, "n".into())],
        );

        assert!(!coord.is_complete());

        coord.task_completed(tasks[0].0);
        assert!(!coord.is_complete());

        coord.task_completed(tasks[1].0);
        assert!(coord.is_complete());
    }

    #[test]
    fn is_complete_with_terminal_failure() {
        let mut coord = FleetCoordinator::with_max_retries(1);
        let tasks = make_tasks(2);
        coord.fan_out(
            tasks.clone(),
            vec![(tasks[0].0, "n".into()), (tasks[1].0, "n".into())],
        );

        coord.task_completed(tasks[0].0);
        coord.task_failed(tasks[1].0); // exhausted (max_retries=1)
        assert!(coord.is_complete());
    }

    #[test]
    fn completion_pct_calculation() {
        let mut coord = FleetCoordinator::new();
        let tasks = make_tasks(4);
        let assignments: Vec<(Uuid, NodeId)> =
            tasks.iter().map(|(id, _)| (*id, "n".into())).collect();
        coord.fan_out(tasks.clone(), assignments);

        assert!((coord.completion_pct() - 0.0).abs() < 1e-9);

        coord.task_completed(tasks[0].0);
        assert!((coord.completion_pct() - 0.25).abs() < 1e-9);

        coord.task_completed(tasks[1].0);
        coord.task_completed(tasks[2].0);
        coord.task_completed(tasks[3].0);
        assert!((coord.completion_pct() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn pending_reassignment_lists_retriable() {
        let mut coord = FleetCoordinator::with_max_retries(3);
        let tasks = make_tasks(3);
        coord.fan_out(
            tasks.clone(),
            vec![
                (tasks[0].0, "n".into()),
                (tasks[1].0, "n".into()),
                (tasks[2].0, "n".into()),
            ],
        );

        coord.task_failed(tasks[0].0); // Reassigned
        coord.task_completed(tasks[1].0);
        // tasks[2] still Assigned

        let pending = coord.pending_reassignment();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].task_id, tasks[0].0);
    }

    #[test]
    fn reassign_moves_task() {
        let mut coord = FleetCoordinator::with_max_retries(3);
        let tasks = make_tasks(1);
        coord.fan_out(tasks.clone(), vec![(tasks[0].0, "old".into())]);

        coord.task_failed(tasks[0].0);
        assert!(coord.reassign(tasks[0].0, "new".into()));

        let t = &coord.tasks[&tasks[0].0];
        assert_eq!(t.assigned_node.as_deref(), Some("new"));
        assert_eq!(t.status, FleetTaskStatus::Assigned);
    }

    #[test]
    fn reassign_unknown_task() {
        let mut coord = FleetCoordinator::new();
        assert!(!coord.reassign(Uuid::new_v4(), "n".into()));
    }

    #[test]
    fn state_manager_accessible() {
        let coord = FleetCoordinator::new();
        // Just verify we can access it without panic.
        let _sm = coord.state_manager();
    }

    #[cfg(feature = "hwaccel")]
    mod hwaccel_tests {
        use super::super::*;
        use ai_hwaccel::{AcceleratorProfile, AcceleratorRegistry, QuantizationLevel};

        #[test]
        fn plan_sharding_single_gpu_no_shard() {
            // 7B model at FP16 (~14GB) on a single 80GB GPU — should not shard.
            let registry = AcceleratorRegistry::from_profiles(vec![
                AcceleratorProfile::cpu(64 * 1024 * 1024 * 1024),
                AcceleratorProfile::cuda(0, 80 * 1024 * 1024 * 1024),
            ]);
            let plan = FleetCoordinator::plan_sharding(
                7_000_000_000,
                &QuantizationLevel::Float16,
                &registry,
            );
            assert!(
                plan.shards.len() <= 1,
                "7B FP16 on 80GB should not need sharding, got {} shards",
                plan.shards.len()
            );
        }

        #[test]
        fn plan_sharding_multi_gpu_large_model() {
            // 70B model at FP16 (~140GB) on 2x 80GB GPUs — should shard.
            let registry = AcceleratorRegistry::from_profiles(vec![
                AcceleratorProfile::cpu(128 * 1024 * 1024 * 1024),
                AcceleratorProfile::cuda(0, 80 * 1024 * 1024 * 1024),
                AcceleratorProfile::cuda(1, 80 * 1024 * 1024 * 1024),
            ]);
            let plan = FleetCoordinator::plan_sharding(
                70_000_000_000,
                &QuantizationLevel::Float16,
                &registry,
            );
            assert!(
                plan.shards.len() >= 2,
                "70B FP16 on 2x80GB should shard across devices, got {} shards",
                plan.shards.len()
            );
            assert!(
                plan.total_memory_bytes > 0,
                "plan should report memory usage"
            );
        }

        #[test]
        fn plan_sharding_quantized_fits_single() {
            // 70B model at INT4 (~35GB) on single 80GB GPU — may fit without sharding.
            let registry = AcceleratorRegistry::from_profiles(vec![
                AcceleratorProfile::cpu(64 * 1024 * 1024 * 1024),
                AcceleratorProfile::cuda(0, 80 * 1024 * 1024 * 1024),
            ]);
            let plan = FleetCoordinator::plan_sharding(
                70_000_000_000,
                &QuantizationLevel::Int4,
                &registry,
            );
            // INT4 70B ≈ 35GB, fits in 80GB.
            assert!(
                plan.shards.len() <= 1,
                "70B INT4 on 80GB should fit without sharding, got {} shards",
                plan.shards.len()
            );
        }
    }
}
