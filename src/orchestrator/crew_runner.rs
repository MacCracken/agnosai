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
use std::sync::Arc;
use std::time::Instant;

use crate::core::agent::AgentDefinition;
use crate::core::crew::{CrewProfile, CrewSpec, CrewState, CrewStatus};
use crate::core::task::{ProcessMode, Task, TaskId, TaskResult, TaskStatus};
use crate::llm::{HooshClient, InferenceRequest, Message, ResponseCache, Role, cache_key};
use crate::server::sse::CrewEvent;
use tokio::sync::Semaphore;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

use crate::orchestrator::scoring;

/// Orchestrates the full crew lifecycle.
pub struct CrewRunner {
    spec: CrewSpec,
    event_tx: Option<broadcast::Sender<CrewEvent>>,
    /// LLM client for real inference. When `None`, falls back to placeholder.
    llm: Option<Arc<HooshClient>>,
    /// Shared response cache for inference results.
    cache: Arc<ResponseCache>,
}

impl CrewRunner {
    /// Create a new crew runner for the given specification.
    pub fn new(spec: CrewSpec) -> Self {
        Self {
            spec,
            event_tx: None,
            llm: None,
            cache: Arc::new(ResponseCache::new(Default::default())),
        }
    }

    /// Attach a shared response cache.
    pub fn with_cache(mut self, cache: Arc<ResponseCache>) -> Self {
        self.cache = cache;
        self
    }

    /// Attach an LLM client for real inference.
    pub fn with_llm(mut self, client: Arc<HooshClient>) -> Self {
        self.llm = Some(client);
        self
    }

    /// Attach an event sender for SSE streaming.
    pub fn with_events(mut self, tx: broadcast::Sender<CrewEvent>) -> Self {
        self.event_tx = Some(tx);
        self
    }

    /// Emit a crew event if an event sender is configured.
    fn emit(&self, event_type: &str, data: serde_json::Value) {
        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(CrewEvent {
                crew_id: self.spec.id.to_string(),
                event_type: event_type.to_string(),
                data,
            });
        }
    }

    /// Execute the crew according to its `ProcessMode`.
    ///
    /// Tasks are executed via LLM inference when a client is configured, or
    /// fall back to placeholder output (useful for tests). Orchestration
    /// handles dependency resolution, agent assignment, status tracking,
    /// result aggregation, and profiling.
    pub async fn run(&mut self) -> crate::core::Result<CrewState> {
        let crew_start = Instant::now();
        info!(crew_id = %self.spec.id, name = %self.spec.name, "starting crew run");

        self.emit(
            "crew_started",
            serde_json::json!({
                "name": self.spec.name,
                "task_count": self.spec.tasks.len(),
            }),
        );

        let results = match self.spec.process {
            ProcessMode::Sequential => self.run_sequential().await?,
            ProcessMode::Parallel { max_concurrency } => self.run_parallel(max_concurrency).await?,
            ProcessMode::Dag => self.run_dag().await?,
            ProcessMode::Hierarchical { .. } => {
                // Phase 2: manager delegation. For now, fall back to sequential.
                warn!("hierarchical mode not yet implemented, falling back to sequential");
                self.run_sequential().await?
            }
        };

        let wall_ms = crew_start.elapsed().as_millis() as u64;
        let all_ok = results.iter().all(|r| r.status == TaskStatus::Completed);
        let status = if all_ok {
            CrewStatus::Completed
        } else {
            CrewStatus::Failed
        };

        // Build profile from per-task latency metadata.
        let task_ms: HashMap<TaskId, u64> = results
            .iter()
            .filter_map(|r| {
                r.metadata
                    .get("task_duration_ms")
                    .and_then(|v| v.as_u64())
                    .map(|ms| (r.task_id, ms))
            })
            .collect();
        let profile = CrewProfile {
            wall_ms,
            task_count: results.len(),
            task_ms,
        };

        info!(
            crew_id = %self.spec.id,
            ?status,
            wall_ms,
            "crew run finished"
        );

        self.emit(
            "crew_completed",
            serde_json::json!({
                "status": format!("{status:?}"),
                "task_count": results.len(),
                "wall_ms": wall_ms,
            }),
        );

        Ok(CrewState {
            crew_id: self.spec.id,
            status,
            results,
            profile: Some(profile),
        })
    }

    // ── Sequential ──────────────────────────────────────────────────────

    async fn run_sequential(&mut self) -> crate::core::Result<Vec<TaskResult>> {
        let mut results = Vec::with_capacity(self.spec.tasks.len());

        for i in 0..self.spec.tasks.len() {
            let agent = pick_best_agent(&self.spec.agents, &self.spec.tasks[i]);
            self.spec.tasks[i].status = TaskStatus::Queued;

            let agent_key = agent.as_ref().map(|a| a.agent_key.clone());

            if let Some(ref a) = agent {
                debug!(task_id = %self.spec.tasks[i].id, agent = %a.agent_key, "assigned");
            }

            self.emit(
                "task_started",
                serde_json::json!({
                    "task_id": self.spec.tasks[i].id.to_string(),
                    "description": self.spec.tasks[i].description,
                    "agent": agent_key,
                }),
            );

            self.spec.tasks[i].status = TaskStatus::Running;
            let result = execute_task(&self.spec.tasks[i], agent.as_ref(), self.llm.as_ref(), &self.cache).await;
            self.spec.tasks[i].status = result.status;

            self.emit(
                "task_completed",
                serde_json::json!({
                    "task_id": result.task_id.to_string(),
                    "status": format!("{:?}", result.status),
                }),
            );

            results.push(result);
        }

        Ok(results)
    }

    // ── Parallel ────────────────────────────────────────────────────────

    async fn run_parallel(
        &mut self,
        max_concurrency: usize,
    ) -> crate::core::Result<Vec<TaskResult>> {
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

        // Emit task_started for all tasks.
        for (task, agent) in &task_snapshots {
            self.emit(
                "task_started",
                serde_json::json!({
                    "task_id": task.id.to_string(),
                    "description": task.description,
                    "agent": agent.as_ref().map(|a| &a.agent_key),
                }),
            );
        }

        let mut join_set = tokio::task::JoinSet::new();

        for (task, agent) in task_snapshots {
            let permit = semaphore.clone();
            let llm = self.llm.clone();
            let cache = Arc::clone(&self.cache);
            join_set.spawn(async move {
                let _permit = match permit.acquire().await {
                    Ok(p) => p,
                    Err(_) => {
                        return TaskResult {
                            task_id: task.id,
                            status: TaskStatus::Failed,
                            output: "internal error: concurrency semaphore closed".into(),
                            metadata: Default::default(),
                        };
                    }
                };
                execute_task(&task, agent.as_ref(), llm.as_ref(), &cache).await
            });
        }

        let mut results = Vec::with_capacity(self.spec.tasks.len());
        while let Some(res) = join_set.join_next().await {
            match res {
                Ok(task_result) => {
                    self.emit(
                        "task_completed",
                        serde_json::json!({
                            "task_id": task_result.task_id.to_string(),
                            "status": format!("{:?}", task_result.status),
                        }),
                    );
                    results.push(task_result);
                }
                Err(e) => {
                    warn!(error = %e, "task join error");
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

    async fn run_dag(&mut self) -> crate::core::Result<Vec<TaskResult>> {
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
                return Err(crate::core::AgnosaiError::Scheduling(
                    "DAG deadlock: no ready tasks but remaining exist".into(),
                ));
            }

            // Run the ready wave concurrently.
            let mut join_set = tokio::task::JoinSet::new();

            for id in &ready {
                let idx = task_map[id];
                self.spec.tasks[idx].status = TaskStatus::Queued;
                let agent = pick_best_agent(&self.spec.agents, &self.spec.tasks[idx]);

                self.emit(
                    "task_started",
                    serde_json::json!({
                        "task_id": self.spec.tasks[idx].id.to_string(),
                        "description": self.spec.tasks[idx].description,
                        "agent": agent.as_ref().map(|a| &a.agent_key),
                    }),
                );

                let task_snap = self.spec.tasks[idx].clone();
                let llm = self.llm.clone();
                let cache = Arc::clone(&self.cache);

                join_set.spawn(async move {
                    execute_task(&task_snap, agent.as_ref(), llm.as_ref(), &cache).await
                });
            }

            while let Some(res) = join_set.join_next().await {
                match res {
                    Ok(tr) => {
                        if let Some(&idx) = task_map.get(&tr.task_id) {
                            self.spec.tasks[idx].status = tr.status;
                        }
                        self.emit(
                            "task_completed",
                            serde_json::json!({
                                "task_id": tr.task_id.to_string(),
                                "status": format!("{:?}", tr.status),
                            }),
                        );
                        completed.insert(tr.task_id);
                        results.push(tr);
                    }
                    Err(e) => {
                        warn!(error = %e, "DAG task join error");
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

/// Execute a task via LLM inference, or fall back to placeholder if no client.
async fn execute_task(
    task: &Task,
    agent: Option<&AgentDefinition>,
    llm: Option<&Arc<HooshClient>>,
    cache: &Arc<ResponseCache>,
) -> TaskResult {
    let task_start = Instant::now();
    let agent_label = agent.map(|a| a.agent_key.as_str()).unwrap_or("unassigned");

    // If no LLM client, fall back to placeholder (useful for tests).
    let Some(client) = llm else {
        debug!(task_id = %task.id, agent = agent_label, "executing task (placeholder — no LLM client)");
        tokio::task::yield_now().await;
        let mut metadata = HashMap::new();
        metadata.insert(
            "task_duration_ms".into(),
            serde_json::json!(task_start.elapsed().as_millis() as u64),
        );
        return TaskResult {
            task_id: task.id,
            output: task.description.clone(),
            status: TaskStatus::Completed,
            metadata,
        };
    };

    debug!(task_id = %task.id, agent = agent_label, "executing task via LLM");

    // Build system prompt from agent definition.
    let system_prompt = build_system_prompt(agent);

    // Choose model: agent override → router-based tier selection.
    let model = select_model(agent);

    // Build messages: include any context as a preamble.
    let mut messages = Vec::new();
    if !task.context.is_empty() {
        let ctx_json = serde_json::to_string_pretty(&task.context).unwrap_or_default();
        messages.push(Message {
            role: Role::User,
            content: format!("Context:\n```json\n{ctx_json}\n```"),
        });
        messages.push(Message {
            role: Role::Assistant,
            content: "Understood, I have the context.".into(),
        });
    }

    // The task description is the main user message.
    let mut user_msg = task.description.clone();
    if let Some(ref expected) = task.expected_output {
        user_msg.push_str(&format!("\n\nExpected output format: {expected}"));
    }

    let request = InferenceRequest {
        model: model.to_string(),
        prompt: user_msg,
        system: Some(system_prompt),
        messages,
        max_tokens: Some(4096),
        temperature: Some(0.7),
        ..Default::default()
    };

    // Check cache before calling the LLM.
    let ck = cache_key(&request.model, &request.messages);
    if let Some(cached) = cache.get(&ck) {
        let task_duration_ms = task_start.elapsed().as_millis() as u64;
        let mut metadata = HashMap::new();
        metadata.insert("model".into(), serde_json::Value::String(request.model.clone()));
        metadata.insert("cached".into(), serde_json::json!(true));
        metadata.insert("task_duration_ms".into(), serde_json::json!(task_duration_ms));

        debug!(
            task_id = %task.id,
            agent = agent_label,
            model = %request.model,
            "task completed from cache"
        );

        return TaskResult {
            task_id: task.id,
            output: (*cached).clone(),
            status: TaskStatus::Completed,
            metadata,
        };
    }

    match client.infer(&request).await {
        Ok(response) => {
            // Cache the successful response.
            cache.insert(ck, response.text.clone());

            let mut metadata = HashMap::new();
            metadata.insert(
                "model".into(),
                serde_json::Value::String(response.model.clone()),
            );
            metadata.insert(
                "provider".into(),
                serde_json::Value::String(response.provider.clone()),
            );
            metadata.insert("latency_ms".into(), serde_json::json!(response.latency_ms));
            metadata.insert(
                "tokens".into(),
                serde_json::json!({
                    "prompt": response.usage.prompt_tokens,
                    "completion": response.usage.completion_tokens,
                    "total": response.usage.total_tokens,
                }),
            );
            let task_duration_ms = task_start.elapsed().as_millis() as u64;
            metadata.insert("task_duration_ms".into(), serde_json::json!(task_duration_ms));

            info!(
                task_id = %task.id,
                agent = agent_label,
                model = %response.model,
                latency_ms = response.latency_ms,
                task_duration_ms,
                tokens = response.usage.total_tokens,
                "task completed via LLM"
            );

            TaskResult {
                task_id: task.id,
                output: response.text,
                status: TaskStatus::Completed,
                metadata,
            }
        }
        Err(e) => {
            let task_duration_ms = task_start.elapsed().as_millis() as u64;
            warn!(
                task_id = %task.id,
                agent = agent_label,
                task_duration_ms,
                error = %e,
                "LLM inference failed"
            );
            let mut metadata = HashMap::new();
            metadata.insert("error".into(), serde_json::Value::String(e.to_string()));
            metadata.insert("task_duration_ms".into(), serde_json::json!(task_duration_ms));

            TaskResult {
                task_id: task.id,
                output: format!("LLM error: {e}"),
                status: TaskStatus::Failed,
                metadata,
            }
        }
    }
}

/// Build a system prompt from an agent's definition.
fn build_system_prompt(agent: Option<&AgentDefinition>) -> String {
    let Some(agent) = agent else {
        return "You are a helpful AI assistant executing tasks within a crew.".into();
    };

    let mut prompt = format!(
        "You are {name}, a {role}.\n\nGoal: {goal}",
        name = agent.name,
        role = agent.role,
        goal = agent.goal,
    );

    if let Some(ref backstory) = agent.backstory {
        prompt.push_str(&format!("\n\nBackstory: {backstory}"));
    }

    if let Some(ref domain) = agent.domain {
        prompt.push_str(&format!("\n\nDomain expertise: {domain}"));
    }

    if !agent.tools.is_empty() {
        prompt.push_str(&format!("\n\nAvailable tools: {}", agent.tools.join(", ")));
    }

    prompt
}

/// Strip provider prefixes (e.g. `ollama/`, `openai/`) from a model string.
///
/// LiteLLM-style identifiers use `provider/model` notation but most inference
/// endpoints (including Ollama's OpenAI-compatible API) expect just the model
/// name.  This helper normalises both forms.
fn strip_provider_prefix(model: &str) -> &str {
    // Common litellm prefixes: ollama/, openai/, anthropic/, etc.
    if let Some(idx) = model.find('/') {
        let prefix = &model[..idx];
        // Only strip known provider prefixes, not arbitrary path components.
        match prefix {
            "ollama" | "openai" | "anthropic" | "groq" | "deepseek" | "mistral" | "together"
            | "fireworks" | "anyscale" | "perplexity" | "bedrock" | "azure" => &model[idx + 1..],
            _ => model,
        }
    } else {
        model
    }
}

/// Select a model for a task: agent override, or route by complexity tier.
fn select_model(agent: Option<&AgentDefinition>) -> &str {
    if let Some(agent) = agent {
        // Agent-specific model override takes priority.
        if let Some(ref model) = agent.llm_model {
            return strip_provider_prefix(model.as_str());
        }

        // Route by agent complexity.
        let complexity = crate::llm::parse_complexity(&agent.complexity);
        let profile = crate::llm::TaskProfile {
            task_type: crate::llm::TaskType::Reason,
            complexity,
        };
        let tier = crate::llm::router::route(&profile);
        return crate::llm::default_model(tier);
    }

    // No agent — use capable tier default.
    crate::llm::default_model(crate::llm::ModelTier::Capable)
}

/// Kahn's algorithm for topological sort. Returns an error on cycles.
fn topological_sort(tasks: &[Task]) -> crate::core::Result<Vec<TaskId>> {
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
        return Err(crate::core::AgnosaiError::CyclicDAG);
    }

    Ok(order)
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::AgentDefinition;
    use crate::core::crew::CrewSpec;
    use crate::core::task::{ProcessMode, Task};
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

    // ── System prompt building ──────────────────────────────────────────

    #[test]
    fn test_build_system_prompt_no_agent() {
        let prompt = build_system_prompt(None);
        assert!(prompt.contains("helpful AI assistant"));
    }

    #[test]
    fn test_build_system_prompt_full_agent() {
        let mut agent = test_agent("qa");
        agent.name = "QA Lead".into();
        agent.role = "quality assurance".into();
        agent.goal = "ensure zero defects".into();
        agent.backstory = Some("10 years in QA".into());
        agent.domain = Some("testing".into());
        agent.tools = vec!["selenium".into(), "pytest".into()];

        let prompt = build_system_prompt(Some(&agent));
        assert!(prompt.contains("QA Lead"));
        assert!(prompt.contains("quality assurance"));
        assert!(prompt.contains("ensure zero defects"));
        assert!(prompt.contains("10 years in QA"));
        assert!(prompt.contains("testing"));
        assert!(prompt.contains("selenium, pytest"));
    }

    #[test]
    fn test_build_system_prompt_minimal_agent() {
        let agent = test_agent("min");
        let prompt = build_system_prompt(Some(&agent));
        assert!(prompt.contains("min")); // name
        assert!(prompt.contains("tester")); // role
        assert!(prompt.contains("test things")); // goal
        assert!(!prompt.contains("Backstory"));
        assert!(!prompt.contains("Domain"));
        assert!(!prompt.contains("Available tools"));
    }

    // ── Model selection ─────────────────────────────────────────────────

    #[test]
    fn test_select_model_no_agent() {
        let model = select_model(None);
        // Should use capable tier default.
        assert_eq!(model, "llama3:70b");
    }

    #[test]
    fn test_select_model_agent_override() {
        let mut agent = test_agent("a");
        agent.llm_model = Some("gpt-4o".into());
        assert_eq!(select_model(Some(&agent)), "gpt-4o");
    }

    #[test]
    fn test_select_model_strips_provider_prefix() {
        let mut agent = test_agent("a");
        agent.llm_model = Some("ollama/llama3.2:1b".into());
        assert_eq!(select_model(Some(&agent)), "llama3.2:1b");

        agent.llm_model = Some("openai/gpt-4o".into());
        assert_eq!(select_model(Some(&agent)), "gpt-4o");

        agent.llm_model = Some("anthropic/claude-sonnet-4-20250514".into());
        assert_eq!(select_model(Some(&agent)), "claude-sonnet-4-20250514");
    }

    #[test]
    fn test_strip_provider_prefix_preserves_unknown() {
        // Unknown prefix should be kept as-is.
        assert_eq!(strip_provider_prefix("custom/model"), "custom/model");
        assert_eq!(strip_provider_prefix("llama3:70b"), "llama3:70b");
    }

    #[test]
    fn test_select_model_routes_by_complexity() {
        let mut low = test_agent("low");
        low.complexity = "low".into();
        // Low complexity + Reason → Capable → llama3:70b
        assert_eq!(select_model(Some(&low)), "llama3:70b");

        let mut high = test_agent("high");
        high.complexity = "high".into();
        // High complexity + Reason → Premium → llama3:405b
        assert_eq!(select_model(Some(&high)), "llama3:405b");
    }

    // ── Placeholder fallback (no LLM client) ────────────────────────────

    #[tokio::test]
    async fn test_execute_task_placeholder_when_no_llm() {
        let task = test_task("do something");
        let agent = test_agent("a");
        let cache = Arc::new(ResponseCache::new(Default::default()));
        let result = execute_task(&task, Some(&agent), None, &cache).await;
        assert_eq!(result.status, TaskStatus::Completed);
        assert_eq!(result.output, "do something");
    }

    // ── SSE event emission ──────────────────────────────────────────────

    #[tokio::test]
    async fn test_events_emitted_during_sequential_run() {
        let tasks = vec![test_task("step one"), test_task("step two")];
        let spec = test_spec(tasks, ProcessMode::Sequential);
        let (tx, mut rx) = broadcast::channel::<CrewEvent>(64);
        let mut runner = CrewRunner::new(spec).with_events(tx);

        let state = runner.run().await.unwrap();
        assert_eq!(state.status, CrewStatus::Completed);

        // Collect all events.
        let mut events = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            events.push(ev);
        }

        // Should have: crew_started, task_started x2, task_completed x2, crew_completed
        let types: Vec<&str> = events.iter().map(|e| e.event_type.as_str()).collect();
        assert!(types.contains(&"crew_started"));
        assert!(types.contains(&"crew_completed"));
        assert_eq!(types.iter().filter(|&&t| t == "task_started").count(), 2);
        assert_eq!(types.iter().filter(|&&t| t == "task_completed").count(), 2);
    }
}
