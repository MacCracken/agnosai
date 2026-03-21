# Crew Execution Patterns

AgnosAI supports four execution patterns via the `ProcessMode` enum in `agnosai-core`. Each pattern controls how tasks within a crew are scheduled and executed.

## Process Modes

```rust
// From agnosai::core::task
pub enum ProcessMode {
    Sequential,
    Parallel { max_concurrency: usize },
    Dag,
    Hierarchical { manager: AgentId },
}
```

The default is `Sequential`.

---

## Sequential Execution

Tasks run one at a time, in the order they appear in the crew spec. Simple and predictable -- use this when each task depends on the previous one's output.

```rust
use agnosai::core::{CrewSpec, Task, ProcessMode};
use agnosai::orchestrator::Orchestrator;

let orchestrator = Orchestrator::new(Default::default()).await?;

let mut crew = CrewSpec::new("pipeline");
crew.tasks = vec![
    Task::new("Gather requirements"),
    Task::new("Write implementation"),
    Task::new("Review code"),
];
crew.process = ProcessMode::Sequential;

let result = orchestrator.run_crew(crew).await?;
// Results are guaranteed in order: [Gather, Write, Review]
```

**When to use:** Linear pipelines where task N needs the output of task N-1.

---

## Parallel Execution

All tasks run concurrently, bounded by a semaphore. Tasks are independent -- no dependency ordering is enforced.

```rust
use agnosai::core::{CrewSpec, Task, ProcessMode};

let mut crew = CrewSpec::new("batch-analysis");
crew.tasks = vec![
    Task::new("Analyze module A"),
    Task::new("Analyze module B"),
    Task::new("Analyze module C"),
    Task::new("Analyze module D"),
];
crew.process = ProcessMode::Parallel { max_concurrency: 2 };

let result = orchestrator.run_crew(crew).await?;
// All 4 tasks complete, but at most 2 run simultaneously.
// Result order may differ from input order.
```

The `max_concurrency` field controls how many tasks can execute at once. Internally, this uses a `tokio::sync::Semaphore`.

**When to use:** Independent tasks that can safely run at the same time (e.g., analyzing separate modules, running parallel test suites).

---

## DAG Execution

Tasks form a directed acyclic graph via their `dependencies` field. The orchestrator resolves the graph using Kahn's algorithm (topological sort), detects cycles, and executes tasks in waves -- each wave contains all tasks whose dependencies are satisfied.

```rust
use agnosai::core::{CrewSpec, Task, ProcessMode, TaskPriority};

let mut gather = Task::new("Gather data from API");
gather.priority = TaskPriority::High;

let mut transform = Task::new("Transform and normalize");
transform.dependencies.push(gather.id);

let mut validate = Task::new("Validate schema compliance");
validate.dependencies.push(gather.id);

let mut report = Task::new("Generate final report");
report.dependencies.push(transform.id);
report.dependencies.push(validate.id);

let mut crew = CrewSpec::new("etl-pipeline");
crew.tasks = vec![gather, transform, validate, report];
crew.process = ProcessMode::Dag;

let result = orchestrator.run_crew(crew).await?;
// Execution waves:
//   Wave 1: [Gather]
//   Wave 2: [Transform, Validate]  (run concurrently)
//   Wave 3: [Report]
```

Cyclic dependencies produce an `AgnosaiError::CyclicDAG` error at scheduling time, before any task executes.

**When to use:** Complex workflows with branching and merging dependencies (ETL pipelines, build graphs, multi-stage analysis).

---

## Hierarchical Execution

A designated manager agent delegates tasks to worker agents. Currently falls back to sequential execution while full manager delegation is being implemented.

```rust
use agnosai::core::{CrewSpec, Task, ProcessMode};
use uuid::Uuid;

let manager_id = Uuid::new_v4();

let mut crew = CrewSpec::new("managed-team");
crew.tasks = vec![
    Task::new("Design API schema"),
    Task::new("Implement endpoints"),
    Task::new("Write integration tests"),
];
crew.process = ProcessMode::Hierarchical { manager: manager_id };

let result = orchestrator.run_crew(crew).await?;
// Currently executes sequentially; manager delegation is on the roadmap.
```

**When to use:** Scenarios where a lead agent should decompose and assign work. Currently equivalent to sequential; full manager delegation is on the roadmap.

---

## Agent Scoring and Assignment

For every task, the orchestrator scores each available agent and assigns the best match. Scoring uses four weighted factors:

| Factor | Weight | Description |
|--------|--------|-------------|
| Tool coverage | 0.40 | Fraction of `required_tools` the agent provides |
| Complexity alignment | 0.30 | How well agent complexity matches task complexity |
| GPU match | 0.15 | Whether the agent has GPU capability when the task requires it |
| Domain match | 0.15 | Whether agent and task share the same domain |

Scores range from 0.0 to 1.0. The agent with the highest score is assigned.

```rust
// Task context controls what the scorer looks for:
let mut task = Task::new("Run security scan");
task.context.insert("required_tools".into(),
    serde_json::json!(["vulnerability_scan", "dependency_audit"]));
task.context.insert("complexity".into(), serde_json::json!("high"));
task.context.insert("domain".into(), serde_json::json!("security"));
task.context.insert("gpu_required".into(), serde_json::json!(false));
```

You can also rank agents explicitly:

```rust
use agnosai::orchestrator::scoring::rank_agents;

let ranked = rank_agents(&crew.agents, &task);
// Returns Vec<(agent_index, score)> sorted by score descending
for (idx, score) in &ranked {
    println!("Agent {} scored {:.2}", crew.agents[*idx].agent_key, score);
}
```

---

## Priority Levels

Tasks have five priority tiers. Higher-priority tasks are dequeued and scheduled first.

```rust
pub enum TaskPriority {
    Background = 0,
    Low = 1,
    Normal = 2,   // default
    High = 3,
    Critical = 4,
}
```

Priority affects:
- **Priority queue scheduling:** The `Scheduler` maintains per-tier FIFO queues and always dequeues from the highest non-empty tier first.
- **DAG wave ordering:** Within a DAG wave, ready tasks are sorted by priority (highest first).
- **Topological sort seeding:** Zero-dependency nodes enter the topological sort ordered by priority.

```rust
let mut urgent = Task::new("Fix production outage");
urgent.priority = TaskPriority::Critical;

let mut routine = Task::new("Update documentation");
routine.priority = TaskPriority::Background;

// In a priority-queue scheduler, urgent always runs before routine,
// regardless of insertion order.
```
