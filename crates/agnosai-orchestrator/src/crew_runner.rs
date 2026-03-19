//! Crew lifecycle: assemble → execute → aggregate.
//!
//! Replaces CrewAI's Crew class with a Rust-native implementation.
//!
//! Flow:
//! 1. Load agent definitions from crew spec
//! 2. Build task dependency graph
//! 3. Score agents for each task, pick best match
//! 4. Execute tasks respecting ProcessMode (Sequential / Parallel / DAG / Hierarchical)
//! 5. Track status transitions: Pending → Queued → Running → Completed/Failed
//! 6. Aggregate results into CrewState

use std::collections::{HashMap, HashSet};

use agnosai_core::agent::AgentDefinition;
use agnosai_core::crew::{CrewSpec, CrewState, CrewStatus};
use agnosai_core::task::{ProcessMode, Task, TaskId, TaskResult, TaskStatus};
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};

use crate::scoring;

/// Orchestrates the full crew lifecycle.
pub struct CrewRunner {
    spec: CrewSpec,
}

impl CrewRunner {
    pub fn new(spec: CrewSpec) -> Self {
        Self { spec }
    }

    /// Execute the crew according to its `ProcessMode`.
    ///
    /// Each task "execution" currently produces a placeholder result (the task
    /// description echoed back). Actual LLM calls come in Phase 2. The value
    /// here is the orchestration logic: dependency resolution, agent assignment,
    /// status tracking, and result aggregation.
    pub async fn run(&mut self) -> agnosai_core::Result<CrewState> {
        info!(crew_id = %self.spec.id, name = %self.spec.name, "starting crew run");

        let results = match &self.spec.process.clone() {
            ProcessMode::Sequential => self.run_sequential().await?,
            ProcessMode::Parallel { max_concurrency } => {
                self.run_parallel(*max_concurrency).await?
            }
            ProcessMode::Dag => self.run_dag().await?,
            ProcessMode::Hierarchical { .. } => {
                // Phase 2: manager delegation. For now, fall back to sequential.
                debug!("hierarchical mode not yet implemented, falling back to sequential");
                self.run_sequential().await?
            }
        };

        let all_ok = results.iter().all(|r| r.status == TaskStatus::Completed);
        let status = if all_ok {
            CrewStatus::Completed
        } else {
            CrewStatus::Failed
        };

        info!(crew_id = %self.spec.id, ?status, "crew run finished");

        Ok(CrewState {
            crew_id: self.spec.id,
            status,
            results,
        })
    }

    // ── Sequential ──────────────────────────────────────────────────────

    async fn run_sequential(&mut self) -> agnosai_core::Result<Vec<TaskResult>> {
        let mut results = Vec::with_capacity(self.spec.tasks.len());

        for i in 0..self.spec.tasks.len() {
            let agent = pick_best_agent(&self.spec.agents, &self.spec.tasks[i]);
            self.spec.tasks[i].status = TaskStatus::Queued;

            if let Some(a) = &agent {
                debug!(task_id = %self.spec.tasks[i].id, agent = %a.agent_key, "assigned");
            }

            self.spec.tasks[i].status = TaskStatus::Running;
            let result = execute_task(&self.spec.tasks[i], agent.as_ref()).await;
            self.spec.tasks[i].status = result.status;
            results.push(result);
        }

        Ok(results)
    }

    // ── Parallel ────────────────────────────────────────────────────────

    async fn run_parallel(
        &mut self,
        max_concurrency: usize,
    ) -> agnosai_core::Result<Vec<TaskResult>> {
        let semaphore = std::sync::Arc::new(Semaphore::new(max_concurrency));

        // Mark all tasks as Queued up-front.
        for task in &mut self.spec.tasks {
            task.status = TaskStatus::Queued;
        }

        // Snapshot tasks and agent assignments before spawning.
        let task_snapshots: Vec<(Task, Option<AgentDefinition>)> = self
            .spec
            .tasks
            .iter()
            .map(|t| {
                let agent = pick_best_agent(&self.spec.agents, t);
                (t.clone(), agent)
            })
            .collect();

        let mut join_set = tokio::task::JoinSet::new();

        for (task, agent) in task_snapshots {
            let permit = semaphore.clone();
            join_set.spawn(async move {
                let _permit = permit.acquire().await.expect("semaphore closed");
                execute_task(&task, agent.as_ref()).await
            });
        }

        let mut results = Vec::with_capacity(self.spec.tasks.len());
        while let Some(res) = join_set.join_next().await {
            match res {
                Ok(task_result) => results.push(task_result),
                Err(e) => {
                    warn!("task join error: {e}");
                }
            }
        }

        // Update spec task statuses from results.
        let status_map: HashMap<TaskId, TaskStatus> =
            results.iter().map(|r| (r.task_id, r.status)).collect();
        for task in &mut self.spec.tasks {
            if let Some(&s) = status_map.get(&task.id) {
                task.status = s;
            }
        }

        Ok(results)
    }

    // ── DAG ─────────────────────────────────────────────────────────────

    async fn run_dag(&mut self) -> agnosai_core::Result<Vec<TaskResult>> {
        let order = topological_sort(&self.spec.tasks)?;

        // Build dependency sets for quick lookup.
        let dep_sets: HashMap<TaskId, HashSet<TaskId>> = self
            .spec
            .tasks
            .iter()
            .map(|t| (t.id, t.dependencies.iter().copied().collect()))
            .collect();

        let task_map: HashMap<TaskId, usize> = self
            .spec
            .tasks
            .iter()
            .enumerate()
            .map(|(i, t)| (t.id, i))
            .collect();

        let mut completed: HashSet<TaskId> = HashSet::new();
        let mut results: Vec<TaskResult> = Vec::with_capacity(self.spec.tasks.len());

        // Walk topological layers: tasks whose deps are all completed are "ready".
        let mut remaining: Vec<TaskId> = order;

        while !remaining.is_empty() {
            // Collect the ready front.
            let (ready, not_ready): (Vec<TaskId>, Vec<TaskId>) =
                remaining.into_iter().partition(|id| {
                    dep_sets
                        .get(id)
                        .is_none_or(|deps| deps.is_subset(&completed))
                });

            if ready.is_empty() {
                // Should not happen after successful topo sort, but guard anyway.
                return Err(agnosai_core::AgnosaiError::Scheduling(
                    "DAG deadlock: no ready tasks but remaining exist".into(),
                ));
            }

            // Run the ready wave concurrently.
            let mut join_set = tokio::task::JoinSet::new();

            for id in &ready {
                let idx = task_map[id];
                self.spec.tasks[idx].status = TaskStatus::Queued;
                let agent = pick_best_agent(&self.spec.agents, &self.spec.tasks[idx]);
                let task_snap = self.spec.tasks[idx].clone();

                join_set.spawn(async move { execute_task(&task_snap, agent.as_ref()).await });
            }

            while let Some(res) = join_set.join_next().await {
                match res {
                    Ok(tr) => {
                        if let Some(&idx) = task_map.get(&tr.task_id) {
                            self.spec.tasks[idx].status = tr.status;
                        }
                        completed.insert(tr.task_id);
                        results.push(tr);
                    }
                    Err(e) => {
                        warn!("DAG task join error: {e}");
                    }
                }
            }

            remaining = not_ready;
        }

        Ok(results)
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Pick the agent with the highest score for a task, or `None` if the roster
/// is empty.
fn pick_best_agent(agents: &[AgentDefinition], task: &Task) -> Option<AgentDefinition> {
    if agents.is_empty() {
        return None;
    }
    let mut ranked: Vec<(&AgentDefinition, f64)> = agents
        .iter()
        .map(|a| (a, scoring::score_agent(a, task)))
        .collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    ranked.first().map(|(a, _)| (*a).clone())
}

/// Placeholder task execution — returns the description as output.
///
/// Phase 2 will replace this with an actual LLM call via the agent's model.
async fn execute_task(task: &Task, agent: Option<&AgentDefinition>) -> TaskResult {
    let agent_label = agent.map(|a| a.agent_key.as_str()).unwrap_or("unassigned");

    debug!(
        task_id = %task.id,
        agent = agent_label,
        "executing task (placeholder)"
    );

    // Simulate a tiny bit of async work so tokio can schedule other tasks.
    tokio::task::yield_now().await;

    TaskResult {
        task_id: task.id,
        output: task.description.clone(),
        status: TaskStatus::Completed,
        metadata: HashMap::new(),
    }
}

/// Kahn's algorithm for topological sort. Returns an error on cycles.
fn topological_sort(tasks: &[Task]) -> agnosai_core::Result<Vec<TaskId>> {
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

    // Seed with zero in-degree nodes, ordered by priority (highest first).
    let mut queue: Vec<TaskId> = in_degree
        .iter()
        .filter(|&(_, &deg)| deg == 0)
        .map(|(&id, _)| id)
        .collect();
    queue.sort_by(|a, b| {
        let pa = tasks.iter().find(|t| t.id == *a).map(|t| t.priority);
        let pb = tasks.iter().find(|t| t.id == *b).map(|t| t.priority);
        pb.cmp(&pa) // higher priority first
    });

    let mut order = Vec::with_capacity(tasks.len());

    while let Some(id) = queue.pop() {
        order.push(id);
        if let Some(children) = dependents.get(&id) {
            for child in children {
                if let Some(deg) = in_degree.get_mut(child) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(*child);
                    }
                }
            }
        }
    }

    if order.len() != tasks.len() {
        return Err(agnosai_core::AgnosaiError::CyclicDAG);
    }

    Ok(order)
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use agnosai_core::agent::AgentDefinition;
    use agnosai_core::crew::CrewSpec;
    use agnosai_core::task::{ProcessMode, Task};
    use uuid::Uuid;

    fn test_agent(key: &str) -> AgentDefinition {
        AgentDefinition {
            agent_key: key.into(),
            name: key.into(),
            role: "tester".into(),
            goal: "test things".into(),
            backstory: None,
            domain: None,
            tools: vec![],
            complexity: "medium".into(),
            llm_model: None,
            gpu_required: false,
            gpu_preferred: false,
            gpu_memory_min_mb: None,
            hardware: None,
        }
    }

    fn test_task(desc: &str) -> Task {
        Task::new(desc)
    }

    fn test_spec(tasks: Vec<Task>, process: ProcessMode) -> CrewSpec {
        CrewSpec {
            id: Uuid::new_v4(),
            name: "test-crew".into(),
            agents: vec![test_agent("agent-a"), test_agent("agent-b")],
            tasks,
            process,
            metadata: Default::default(),
        }
    }

    // ── Sequential ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_sequential_execution() {
        let tasks = vec![
            test_task("step one"),
            test_task("step two"),
            test_task("step three"),
        ];
        let spec = test_spec(tasks, ProcessMode::Sequential);
        let mut runner = CrewRunner::new(spec);

        let state = runner.run().await.unwrap();

        assert_eq!(state.status, CrewStatus::Completed);
        assert_eq!(state.results.len(), 3);

        // Sequential preserves order.
        assert_eq!(state.results[0].output, "step one");
        assert_eq!(state.results[1].output, "step two");
        assert_eq!(state.results[2].output, "step three");

        // All tasks should be Completed.
        for r in &state.results {
            assert_eq!(r.status, TaskStatus::Completed);
        }
    }

    #[tokio::test]
    async fn test_sequential_empty() {
        let spec = test_spec(vec![], ProcessMode::Sequential);
        let mut runner = CrewRunner::new(spec);
        let state = runner.run().await.unwrap();
        assert_eq!(state.status, CrewStatus::Completed);
        assert!(state.results.is_empty());
    }

    // ── Parallel ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_parallel_execution() {
        let tasks = vec![
            test_task("par one"),
            test_task("par two"),
            test_task("par three"),
            test_task("par four"),
        ];
        let spec = test_spec(tasks, ProcessMode::Parallel { max_concurrency: 2 });
        let mut runner = CrewRunner::new(spec);

        let state = runner.run().await.unwrap();

        assert_eq!(state.status, CrewStatus::Completed);
        assert_eq!(state.results.len(), 4);

        // All completed (order may vary in parallel).
        let outputs: HashSet<String> = state.results.iter().map(|r| r.output.clone()).collect();
        assert!(outputs.contains("par one"));
        assert!(outputs.contains("par two"));
        assert!(outputs.contains("par three"));
        assert!(outputs.contains("par four"));
    }

    #[tokio::test]
    async fn test_parallel_single_concurrency() {
        let tasks = vec![test_task("a"), test_task("b")];
        let spec = test_spec(tasks, ProcessMode::Parallel { max_concurrency: 1 });
        let mut runner = CrewRunner::new(spec);
        let state = runner.run().await.unwrap();
        assert_eq!(state.status, CrewStatus::Completed);
        assert_eq!(state.results.len(), 2);
    }

    // ── DAG ─────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_dag_execution_with_dependencies() {
        // Graph: A → B → C  (C depends on B, B depends on A)
        let task_a = test_task("task A");
        let mut task_b = test_task("task B");
        let mut task_c = test_task("task C");

        task_b.dependencies.push(task_a.id);
        task_c.dependencies.push(task_b.id);

        let spec = test_spec(vec![task_a, task_b, task_c], ProcessMode::Dag);
        let mut runner = CrewRunner::new(spec);

        let state = runner.run().await.unwrap();

        assert_eq!(state.status, CrewStatus::Completed);
        assert_eq!(state.results.len(), 3);

        // Verify dependency order: A must come before B, B before C.
        let pos = |desc: &str| state.results.iter().position(|r| r.output == desc).unwrap();
        assert!(pos("task A") < pos("task B"));
        assert!(pos("task B") < pos("task C"));
    }

    #[tokio::test]
    async fn test_dag_diamond() {
        // Diamond: A → B, A → C, B → D, C → D
        let a = test_task("A");
        let mut b = test_task("B");
        let mut c = test_task("C");
        let mut d = test_task("D");

        b.dependencies.push(a.id);
        c.dependencies.push(a.id);
        d.dependencies.push(b.id);
        d.dependencies.push(c.id);

        let spec = test_spec(vec![a, b, c, d], ProcessMode::Dag);
        let mut runner = CrewRunner::new(spec);
        let state = runner.run().await.unwrap();

        assert_eq!(state.status, CrewStatus::Completed);
        assert_eq!(state.results.len(), 4);

        let pos = |desc: &str| state.results.iter().position(|r| r.output == desc).unwrap();
        assert!(pos("A") < pos("B"));
        assert!(pos("A") < pos("C"));
        assert!(pos("B") < pos("D"));
        assert!(pos("C") < pos("D"));
    }

    #[tokio::test]
    async fn test_dag_no_deps_runs_all() {
        // All independent tasks — should all run in the first wave.
        let tasks = vec![test_task("x"), test_task("y"), test_task("z")];
        let spec = test_spec(tasks, ProcessMode::Dag);
        let mut runner = CrewRunner::new(spec);
        let state = runner.run().await.unwrap();
        assert_eq!(state.status, CrewStatus::Completed);
        assert_eq!(state.results.len(), 3);
    }

    // ── Topological sort ────────────────────────────────────────────────

    #[test]
    fn test_topo_sort_detects_cycle() {
        let mut a = test_task("a");
        let mut b = test_task("b");
        a.dependencies.push(b.id);
        b.dependencies.push(a.id);

        let err = topological_sort(&[a, b]);
        assert!(err.is_err());
    }

    // ── Agent selection ─────────────────────────────────────────────────

    #[test]
    fn test_pick_best_agent_empty_roster() {
        let task = test_task("something");
        assert!(pick_best_agent(&[], &task).is_none());
    }

    #[test]
    fn test_pick_best_agent_returns_some() {
        let task = test_task("something");
        let agents = vec![test_agent("a1")];
        let picked = pick_best_agent(&agents, &task);
        assert!(picked.is_some());
        assert_eq!(picked.unwrap().agent_key, "a1");
    }

    // ── Hierarchical fallback ───────────────────────────────────────────

    #[tokio::test]
    async fn test_hierarchical_falls_back_to_sequential() {
        let tasks = vec![test_task("h1"), test_task("h2")];
        let spec = test_spec(
            tasks,
            ProcessMode::Hierarchical {
                manager: Uuid::new_v4(),
            },
        );
        let mut runner = CrewRunner::new(spec);
        let state = runner.run().await.unwrap();
        assert_eq!(state.status, CrewStatus::Completed);
        assert_eq!(state.results.len(), 2);
        // Sequential order preserved in fallback.
        assert_eq!(state.results[0].output, "h1");
        assert_eq!(state.results[1].output, "h2");
    }
}
