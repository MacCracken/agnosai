use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};

use crate::core::error::{AgnosaiError, Result};
use crate::core::task::{Task, TaskDAG, TaskId};

/// Priority-based task scheduler with per-level queues and DAG-aware execution.
///
/// Supports two modes:
/// - **Simple priority queue**: `enqueue` / `dequeue` for independent tasks across
///   five priority tiers (Critical → Background).
/// - **DAG scheduling**: `load_dag` validates acyclicity, then `ready_tasks` returns
///   tasks whose dependencies are fully satisfied.
pub struct Scheduler {
    /// Per-priority-tier FIFO queues (index = priority discriminant).
    queues: [VecDeque<Task>; 5],

    /// DAG tasks indexed by their string key from `TaskDAG.tasks`.
    dag_tasks: HashMap<String, Task>,

    /// Forward edges: key → set of keys that depend on it.
    dependents: HashMap<String, HashSet<String>>,

    /// Reverse edges: key → set of keys it depends on.
    dependencies: HashMap<String, HashSet<String>>,

    /// Mapping from TaskId (UUID) back to the string key for lookup.
    id_to_key: HashMap<TaskId, String>,
}

impl Scheduler {
    /// Create a new empty scheduler.
    pub fn new() -> Self {
        Self {
            queues: Default::default(),
            dag_tasks: HashMap::new(),
            dependents: HashMap::new(),
            dependencies: HashMap::new(),
            id_to_key: HashMap::new(),
        }
    }

    // ── Priority queue operations ──────────────────────────────────────

    /// Push a task into the appropriate priority-tier queue.
    pub fn enqueue(&mut self, task: Task) {
        let tier = task.priority as usize;
        self.queues[tier].push_back(task);
    }

    /// Pop the highest-priority task available (Critical first, then High, …).
    pub fn dequeue(&mut self) -> Option<Task> {
        for tier in (0..5).rev() {
            if let Some(task) = self.queues[tier].pop_front() {
                return Some(task);
            }
        }
        None
    }

    /// Total number of tasks across all priority queues (excludes DAG tasks).
    #[inline]
    pub fn len(&self) -> usize {
        self.queues.iter().map(|q| q.len()).sum()
    }

    /// Whether all priority queues are empty (excludes DAG tasks).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.queues.iter().all(|q| q.is_empty())
    }

    // ── DAG operations ─────────────────────────────────────────────────

    /// Load tasks from a `TaskDAG`, validate that the graph is acyclic,
    /// and populate internal adjacency structures.
    ///
    /// Replaces any previously loaded DAG state.
    pub fn load_dag(&mut self, dag: &TaskDAG) -> Result<()> {
        // Build adjacency from edges.
        let cap = dag.tasks.len();
        let mut dependents: HashMap<String, HashSet<String>> = HashMap::with_capacity(cap);
        let mut dependencies: HashMap<String, HashSet<String>> = HashMap::with_capacity(cap);

        // Initialise every task key so nodes with no edges still appear.
        for key in dag.tasks.keys() {
            dependents.entry(key.clone()).or_default();
            dependencies.entry(key.clone()).or_default();
        }

        for (from, to) in &dag.edges {
            // `from` must complete before `to` can run.
            dependents
                .entry(from.clone())
                .or_default()
                .insert(to.clone());
            dependencies
                .entry(to.clone())
                .or_default()
                .insert(from.clone());
        }

        // Cycle detection via Kahn's algorithm — if we cannot consume all
        // nodes, a cycle exists.
        Self::kahn_sort(&dag.tasks, &dependents, &dependencies)?;

        // Commit state.
        let mut id_to_key = HashMap::new();
        for (key, task) in &dag.tasks {
            id_to_key.insert(task.id, key.clone());
        }

        self.dag_tasks = dag.tasks.clone();
        self.dependents = dependents;
        self.dependencies = dependencies;
        self.id_to_key = id_to_key;

        Ok(())
    }

    /// Return tasks whose dependencies have all been completed.
    ///
    /// `completed` contains the `TaskId`s of tasks that are finished.
    /// Only tasks that are **not** themselves completed are returned,
    /// sorted by priority (highest first).
    pub fn ready_tasks(&self, completed: &HashSet<TaskId>) -> Vec<Task> {
        let completed_keys: HashSet<&String> = completed
            .iter()
            .filter_map(|id| self.id_to_key.get(id))
            .collect();

        let mut ready: Vec<Task> = self
            .dag_tasks
            .iter()
            .filter(|(key, task)| {
                // Not already completed.
                !completed.contains(&task.id)
                    // All dependencies satisfied.
                    && self
                        .dependencies
                        .get(*key)
                        .map(|deps| deps.iter().all(|d| completed_keys.contains(d)))
                        .unwrap_or(true)
            })
            .map(|(_, task)| task.clone())
            .collect();

        // Highest priority first.
        ready.sort_by(|a, b| b.priority.cmp(&a.priority));
        ready
    }

    /// Return a topological ordering of task keys in the DAG.
    ///
    /// Returns `Err(AgnosaiError::CyclicDAG)` if the graph contains a cycle.
    pub fn topological_sort(dag: &TaskDAG) -> Result<Vec<String>> {
        let cap = dag.tasks.len();
        let mut dependents: HashMap<String, HashSet<String>> = HashMap::with_capacity(cap);
        let mut dependencies: HashMap<String, HashSet<String>> = HashMap::with_capacity(cap);

        for key in dag.tasks.keys() {
            dependents.entry(key.clone()).or_default();
            dependencies.entry(key.clone()).or_default();
        }

        for (from, to) in &dag.edges {
            dependents
                .entry(from.clone())
                .or_default()
                .insert(to.clone());
            dependencies
                .entry(to.clone())
                .or_default()
                .insert(from.clone());
        }

        Self::kahn_sort(&dag.tasks, &dependents, &dependencies)
    }

    // ── Internal ───────────────────────────────────────────────────────

    /// Kahn's algorithm: returns topological order or `CyclicDAG` error.
    fn kahn_sort(
        tasks: &HashMap<String, Task>,
        dependents: &HashMap<String, HashSet<String>>,
        dependencies: &HashMap<String, HashSet<String>>,
    ) -> Result<Vec<String>> {
        let mut in_degree: HashMap<String, usize> = tasks
            .keys()
            .map(|k| {
                let deg = dependencies.get(k).map(|s| s.len()).unwrap_or(0);
                (k.clone(), deg)
            })
            .collect();

        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|&(_, &deg)| deg == 0)
            .map(|(k, _)| k.clone())
            .collect();

        // Deterministic ordering within the same tier.
        let mut queue_sorted: Vec<String> = queue.drain(..).collect();
        queue_sorted.sort();
        queue = queue_sorted.into_iter().collect();

        let mut order = Vec::with_capacity(tasks.len());

        while let Some(node) = queue.pop_front() {
            order.push(node.clone());
            if let Some(succs) = dependents.get(&node) {
                let mut succs_sorted: Vec<&String> = succs.iter().collect();
                succs_sorted.sort();
                for succ in succs_sorted {
                    if let Some(deg) = in_degree.get_mut(succ) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(succ.clone());
                        }
                    }
                }
            }
        }

        if order.len() != tasks.len() {
            return Err(AgnosaiError::CyclicDAG);
        }

        Ok(order)
    }
}

/// Topological sort for a slice of [`Task`]s using their `id` and `dependencies` fields.
///
/// Returns task IDs in dependency order, respecting priority (highest first
/// among tasks at the same dependency depth). Returns `CyclicDAG` on cycles.
pub fn topological_sort_tasks(tasks: &[Task]) -> Result<Vec<TaskId>> {
    let ids: HashSet<TaskId> = tasks.iter().map(|t| t.id).collect();
    let mut in_degree: HashMap<TaskId, usize> = tasks.iter().map(|t| (t.id, 0)).collect();
    let mut dependents: HashMap<TaskId, Vec<TaskId>> = HashMap::new();

    for task in tasks {
        for dep in &task.dependencies {
            if ids.contains(dep) {
                *in_degree.entry(task.id).or_default() += 1;
                dependents.entry(*dep).or_default().push(task.id);
            }
        }
    }

    // Pre-build priority lookup.
    let priority_map: HashMap<TaskId, crate::core::task::TaskPriority> =
        tasks.iter().map(|t| (t.id, t.priority)).collect();

    // Seed with zero in-degree nodes in a priority heap.
    // BinaryHeap is a max-heap, so higher TaskPriority values pop first.
    let mut queue: BinaryHeap<(crate::core::task::TaskPriority, TaskId)> = in_degree
        .iter()
        .filter(|&(_, &deg)| deg == 0)
        .map(|(&id, _)| (priority_map.get(&id).copied().unwrap_or_default(), id))
        .collect();

    let mut order = Vec::with_capacity(tasks.len());

    while let Some((_, id)) = queue.pop() {
        order.push(id);
        if let Some(children) = dependents.get(&id) {
            for child in children {
                if let Some(deg) = in_degree.get_mut(child) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push((priority_map.get(child).copied().unwrap_or_default(), *child));
                    }
                }
            }
        }
    }

    if order.len() != tasks.len() {
        return Err(AgnosaiError::CyclicDAG);
    }

    Ok(order)
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::task::{TaskPriority, TaskStatus};
    use std::collections::HashMap;

    /// Helper: build a Task with a fixed UUID based on a numeric seed so
    /// tests can reference stable IDs.
    fn make_task(desc: &str, priority: TaskPriority) -> Task {
        Task {
            id: uuid::Uuid::new_v4(),
            description: desc.to_string(),
            expected_output: None,
            assigned_agent: None,
            priority,
            status: TaskStatus::Pending,
            dependencies: Vec::new(),
            context: HashMap::new(),
            risk: crate::core::task::TaskRisk::default(),
            output_schema: None,
        }
    }

    fn make_dag(keys: &[&str], edges: &[(&str, &str)]) -> (TaskDAG, HashMap<String, TaskId>) {
        let mut tasks = HashMap::new();
        let mut ids = HashMap::new();
        for &k in keys {
            let t = make_task(k, TaskPriority::Normal);
            ids.insert(k.to_string(), t.id);
            tasks.insert(k.to_string(), t);
        }
        let edges = edges
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect();
        (
            TaskDAG {
                tasks,
                edges,
                process: crate::core::task::ProcessMode::Dag,
            },
            ids,
        )
    }

    // ── Priority queue tests ───────────────────────────────────────────

    #[test]
    fn priority_ordering() {
        let mut s = Scheduler::new();
        s.enqueue(make_task("bg", TaskPriority::Background));
        s.enqueue(make_task("crit", TaskPriority::Critical));
        s.enqueue(make_task("norm", TaskPriority::Normal));

        assert_eq!(s.len(), 3);

        let first = s.dequeue().unwrap();
        assert_eq!(first.description, "crit");

        let second = s.dequeue().unwrap();
        assert_eq!(second.description, "norm");

        let third = s.dequeue().unwrap();
        assert_eq!(third.description, "bg");

        assert!(s.is_empty());
    }

    #[test]
    fn fifo_within_same_priority() {
        let mut s = Scheduler::new();
        s.enqueue(make_task("first", TaskPriority::High));
        s.enqueue(make_task("second", TaskPriority::High));

        assert_eq!(s.dequeue().unwrap().description, "first");
        assert_eq!(s.dequeue().unwrap().description, "second");
    }

    // ── DAG topological sort tests ─────────────────────────────────────

    #[test]
    fn topo_sort_chain() {
        // A → B → C
        let (dag, _) = make_dag(&["A", "B", "C"], &[("A", "B"), ("B", "C")]);
        let order = Scheduler::topological_sort(&dag).unwrap();
        assert_eq!(order, vec!["A", "B", "C"]);
    }

    #[test]
    fn topo_sort_diamond() {
        // A → {B, C} → D
        let (dag, _) = make_dag(
            &["A", "B", "C", "D"],
            &[("A", "B"), ("A", "C"), ("B", "D"), ("C", "D")],
        );
        let order = Scheduler::topological_sort(&dag).unwrap();

        // A must come first, D must come last, B and C in between.
        assert_eq!(order[0], "A");
        assert_eq!(order[3], "D");
        assert!(order[1..3].contains(&"B".to_string()));
        assert!(order[1..3].contains(&"C".to_string()));
    }

    #[test]
    fn topo_sort_cycle_detected() {
        let (dag, _) = make_dag(&["A", "B", "C"], &[("A", "B"), ("B", "C"), ("C", "A")]);
        let err = Scheduler::topological_sort(&dag).unwrap_err();
        assert!(
            matches!(err, AgnosaiError::CyclicDAG),
            "expected CyclicDAG, got {err:?}"
        );
    }

    // ── DAG ready-task tests ───────────────────────────────────────────

    #[test]
    fn ready_tasks_chain() {
        // A → B → C
        let (dag, ids) = make_dag(&["A", "B", "C"], &[("A", "B"), ("B", "C")]);
        let mut s = Scheduler::new();
        s.load_dag(&dag).unwrap();

        // Nothing completed → only A is ready.
        let completed = HashSet::new();
        let ready = s.ready_tasks(&completed);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].description, "A");

        // A completed → B is ready.
        let completed: HashSet<TaskId> = [ids["A"]].into_iter().collect();
        let ready = s.ready_tasks(&completed);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].description, "B");

        // A and B completed → C is ready.
        let completed: HashSet<TaskId> = [ids["A"], ids["B"]].into_iter().collect();
        let ready = s.ready_tasks(&completed);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].description, "C");
    }

    #[test]
    fn ready_tasks_diamond() {
        // A → {B, C} → D
        let (dag, ids) = make_dag(
            &["A", "B", "C", "D"],
            &[("A", "B"), ("A", "C"), ("B", "D"), ("C", "D")],
        );
        let mut s = Scheduler::new();
        s.load_dag(&dag).unwrap();

        // A completed → B and C are ready.
        let completed: HashSet<TaskId> = [ids["A"]].into_iter().collect();
        let ready = s.ready_tasks(&completed);
        assert_eq!(ready.len(), 2);
        let descs: HashSet<&str> = ready.iter().map(|t| t.description.as_str()).collect();
        assert!(descs.contains("B"));
        assert!(descs.contains("C"));

        // Only B completed (not C) → D is NOT ready.
        let completed: HashSet<TaskId> = [ids["A"], ids["B"]].into_iter().collect();
        let ready = s.ready_tasks(&completed);
        // C is ready, D is not (needs C too).
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].description, "C");

        // Both B and C completed → D is ready.
        let completed: HashSet<TaskId> = [ids["A"], ids["B"], ids["C"]].into_iter().collect();
        let ready = s.ready_tasks(&completed);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].description, "D");
    }

    #[test]
    fn load_dag_rejects_cycle() {
        let (dag, _) = make_dag(&["X", "Y"], &[("X", "Y"), ("Y", "X")]);
        let mut s = Scheduler::new();
        let err = s.load_dag(&dag).unwrap_err();
        assert!(matches!(err, AgnosaiError::CyclicDAG));
    }

    #[test]
    fn ready_tasks_with_priority() {
        // Two independent tasks with different priorities.
        let mut tasks = HashMap::new();
        let t_hi = Task {
            id: uuid::Uuid::new_v4(),
            description: "high".to_string(),
            expected_output: None,
            assigned_agent: None,
            priority: TaskPriority::High,
            status: TaskStatus::Pending,
            dependencies: Vec::new(),
            context: HashMap::new(),
            risk: crate::core::task::TaskRisk::default(),
            output_schema: None,
        };
        let t_lo = Task {
            id: uuid::Uuid::new_v4(),
            description: "low".to_string(),
            expected_output: None,
            assigned_agent: None,
            priority: TaskPriority::Low,
            status: TaskStatus::Pending,
            dependencies: Vec::new(),
            context: HashMap::new(),
            risk: crate::core::task::TaskRisk::default(),
            output_schema: None,
        };
        tasks.insert("hi".to_string(), t_hi);
        tasks.insert("lo".to_string(), t_lo);

        let dag = TaskDAG {
            tasks,
            edges: vec![],
            process: crate::core::task::ProcessMode::Dag,
        };

        let mut s = Scheduler::new();
        s.load_dag(&dag).unwrap();

        let ready = s.ready_tasks(&HashSet::new());
        assert_eq!(ready.len(), 2);
        // High priority should come first.
        assert_eq!(ready[0].description, "high");
        assert_eq!(ready[1].description, "low");
    }

    // ── topological_sort_tasks tests ──────────────────────────────────

    fn make_tasks_with_deps(count: usize, deps: &[(usize, usize)]) -> Vec<Task> {
        let mut tasks: Vec<Task> = (0..count)
            .map(|i| {
                let mut t = make_task(&format!("task-{i}"), TaskPriority::Normal);
                // Use deterministic UUIDs based on index for stable testing.
                t.id = uuid::Uuid::from_u128(i as u128);
                t
            })
            .collect();
        for &(from, to) in deps {
            let dep_id = tasks[from].id;
            tasks[to].dependencies.push(dep_id);
        }
        tasks
    }

    #[test]
    fn topo_sort_tasks_linear_chain() {
        // 0 → 1 → 2
        let tasks = make_tasks_with_deps(3, &[(0, 1), (1, 2)]);
        let order = topological_sort_tasks(&tasks).unwrap();
        assert_eq!(order[0], tasks[0].id);
        assert_eq!(order[1], tasks[1].id);
        assert_eq!(order[2], tasks[2].id);
    }

    #[test]
    fn topo_sort_tasks_diamond() {
        // 0 → {1, 2} → 3
        let tasks = make_tasks_with_deps(4, &[(0, 1), (0, 2), (1, 3), (2, 3)]);
        let order = topological_sort_tasks(&tasks).unwrap();
        assert_eq!(order[0], tasks[0].id);
        assert_eq!(order[3], tasks[3].id);
        // 1 and 2 are in the middle (order depends on priority, both Normal).
        assert!(order[1..3].contains(&tasks[1].id));
        assert!(order[1..3].contains(&tasks[2].id));
    }

    #[test]
    fn topo_sort_tasks_cycle_detected() {
        // 0 → 1 → 2 → 0
        let tasks = make_tasks_with_deps(3, &[(0, 1), (1, 2), (2, 0)]);
        let err = topological_sort_tasks(&tasks).unwrap_err();
        assert!(matches!(err, AgnosaiError::CyclicDAG));
    }

    #[test]
    fn topo_sort_tasks_independent() {
        // Three independent tasks — all should appear in order.
        let tasks = make_tasks_with_deps(3, &[]);
        let order = topological_sort_tasks(&tasks).unwrap();
        assert_eq!(order.len(), 3);
    }

    #[test]
    fn topo_sort_tasks_respects_priority() {
        let mut tasks = make_tasks_with_deps(3, &[]);
        tasks[0].priority = TaskPriority::Background;
        tasks[1].priority = TaskPriority::Critical;
        tasks[2].priority = TaskPriority::Normal;

        let order = topological_sort_tasks(&tasks).unwrap();
        // Critical first, Normal second, Background last.
        assert_eq!(order[0], tasks[1].id);
        assert_eq!(order[1], tasks[2].id);
        assert_eq!(order[2], tasks[0].id);
    }

    #[test]
    fn topo_sort_tasks_single() {
        let tasks = make_tasks_with_deps(1, &[]);
        let order = topological_sort_tasks(&tasks).unwrap();
        assert_eq!(order.len(), 1);
        assert_eq!(order[0], tasks[0].id);
    }

    #[test]
    fn topo_sort_tasks_empty() {
        let order = topological_sort_tasks(&[]).unwrap();
        assert!(order.is_empty());
    }
}
