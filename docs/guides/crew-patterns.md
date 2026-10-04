# Crew Execution Patterns

AgnosAI supports four execution patterns. Each controls how the tasks within a
crew are scheduled and executed.

⚠ This guide used to describe a Rust `ProcessMode` enum "in `agnosai-core`".
There is no such crate — the port is Cyrius, and a process mode is built by a
constructor in `src/core/task.cyr`.

## Process modes

```cyr
agnosai_process_sequential()              # the default
agnosai_process_parallel(max_concurrency)
agnosai_process_dag()
agnosai_process_hierarchical(manager)     # manager is an agent id
```

Attach one to a crew with `agnosai_crew_with_process(spec, mode)`; read it back
with `agnosai_process_kind(p)`, `agnosai_process_max_concurrency(p)` and
`agnosai_process_manager(p)`.

⚠ **Every mode observes the execution deadline**, including Parallel. That was
not always true — until 2026-08-13 the cooperative check sat only on the
sequential and DAG loop heads, so a Parallel crew ran past
`budget.max_duration_secs` and reported success. It is checked at each batch head
now.

⚠ **The deadline is cooperative, not an abort.** Cyrius cannot interrupt a thread
mid-syscall, so no task *starts* after the deadline but a task already running
finishes. A single wedged LLM call still holds its worker.

---

## Sequential Execution

Tasks run one at a time, in the order they appear in the crew spec. Simple and predictable -- use this when each task depends on the previous one's output.

```cyr
var orchestrator = agnosai_orchestrator_new(0);

var crew = agnosai_crew_new(str_from("pipeline"));
var tasks = vec_new();
vec_push(tasks, agnosai_task_new(str_from("Gather requirements")));
vec_push(tasks, agnosai_task_new(str_from("Write implementation")));
vec_push(tasks, agnosai_task_new(str_from("Review code")));
agnosai_crew_with_tasks(crew, tasks);
agnosai_crew_with_process(crew, agnosai_process_sequential());

let result = orchestrator.run_crew(crew).await?;
// Results are guaranteed in order: [Gather, Write, Review]
```

**When to use:** Linear pipelines where task N needs the output of task N-1.

---

## Parallel Execution

All tasks run concurrently, bounded by a semaphore. Tasks are independent -- no dependency ordering is enforced.

```cyr
var crew = agnosai_crew_new(str_from("batch-analysis"));
var tasks = vec_new();
vec_push(tasks, agnosai_task_new(str_from("Analyze module A")));
vec_push(tasks, agnosai_task_new(str_from("Analyze module B")));
vec_push(tasks, agnosai_task_new(str_from("Analyze module C")));
vec_push(tasks, agnosai_task_new(str_from("Analyze module D")));
agnosai_crew_with_tasks(crew, tasks);
agnosai_crew_with_process(crew, agnosai_process_parallel(2));

# Each wave spawns real OS threads and joins them before the next.
var state = agnosai_orchestrator_run_crew(orchestrator, crew);
// All 4 tasks complete, but at most 2 run simultaneously.
// Result order may differ from input order.
```

`max_concurrency` controls how many tasks run at once. ⚠ There is no semaphore —
the port batches: `_agnosai_crew_run_wave` spawns at most `max_concurrency` real
OS threads, **joins every one of them** before starting the next batch, and
checks the deadline and the cancel flag at each batch head. So no thread is left
detached and the concurrency ceiling is structural rather than permit-based.

**When to use:** Independent tasks that can safely run at the same time (e.g., analyzing separate modules, running parallel test suites).

---

## DAG Execution

Tasks form a directed acyclic graph via their `dependencies` field. The orchestrator resolves the graph using Kahn's algorithm (topological sort), detects cycles, and executes tasks in waves -- each wave contains all tasks whose dependencies are satisfied.

```cyr
var gather = agnosai_task_new(str_from("Gather data from API"));
agnosai_task_with_priority(gather, AGNOSAI_PRIORITY_HIGH);

var transform = agnosai_task_new(str_from("Transform and normalize"));
agnosai_task_with_dependency(transform, agnosai_task_id(gather));

var validate = agnosai_task_new(str_from("Validate schema compliance"));
agnosai_task_with_dependency(validate, agnosai_task_id(gather));

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

```cyr
var manager_id = agnosai_uuid_v4_str();

var crew = agnosai_crew_new(str_from("managed-team"));
var tasks = vec_new();
vec_push(tasks, agnosai_task_new(str_from("Design API schema")));
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

For every task, the orchestrator scores each available agent and assigns the best match. Scoring uses five weighted factors (the constants in `src/orchestrator/scoring.cyr`):

| Factor | Weight | Description |
|--------|--------|-------------|
| Tool coverage | 0.35 | Fraction of `required_tools` the agent provides |
| Complexity alignment | 0.25 | How well agent complexity matches task complexity (`1 - abs(diff) / 3` over low/medium/high; a task with none counts as `medium`) |
| Domain match | 0.15 | Whether agent and task share the same domain (case-insensitive; either side unset matches) |
| Personality fit | 0.15 | Always the neutral 0.5 until bhava is ported; reported as `unmeasured` |
| GPU match | 0.10 | Whether the agent has GPU capability when the task requires it |

Scores range from 0.0 to 1.0, and because personality is fixed at 0.5 the best possible score is 0.925. The agent with the highest score is assigned; on a tie, the one earlier in the roster.

```cyr
# Task context controls what the scorer looks for.
var task = agnosai_task_new(str_from("Run security scan"));
var tools = bayan_json_v_arr_new();
bayan_json_v_arr_push(tools, bayan_json_v_str_new(str_from("vulnerability_scan")));
bayan_json_v_arr_push(tools, bayan_json_v_str_new(str_from("dependency_audit")));
agnosai_task_with_context(task, str_from("required_tools"), tools);
agnosai_task_with_context(task, str_from("complexity"), bayan_json_v_str_new(str_from("high")));
agnosai_task_with_context(task, str_from("domain"), bayan_json_v_str_new(str_from("security")));
agnosai_task_with_context(task, str_from("gpu_required"), bayan_json_v_bool_new(0));
```

⚠ **Context keys are cloned on insert**, so the map owns them — but the tree you
hand it is stored by reference. Do not build a context value in an arena that is
reset before the crew runs.

You can also rank agents explicitly:

```cyr
# Returns a vec of scored entries, highest first.
var ranked = agnosai_rank_agents(agents, task);
```

To see *why* an agent was chosen, explain the selection. It returns the same ranking
with each agent's five component scores, so entry 0 is the agent the crew runner
assigns, and it allocates only in the allocator you pass:

```cyr
var a = arena_allocator(65536);
var explained = agnosai_explain_selection_a(a, agents, task);   # 0 if `a` runs out
var best = vec_get(explained, 0);   # an empty roster explains as an empty vec
var tool = agnosai_selection_tool_coverage(best);   # f64 bits; also _complexity,
                                                    # _gpu, _domain, _personality

# As JSON: {"winner": {"index", "agent_key"} | null, "candidate_count",
#           "candidates": [{"index", "agent_key", "total", "scores": {...}}]}
# The last argument caps the candidates (0 renders all of them).
var why = agnosai_selection_to_value_a(a, explained, agents, 10);
# {"weights": {...}, "unmeasured": ["personality"]}
var scorer = agnosai_selection_scorer_to_value_a(a);
```

The explanation is recomputed from the roster and the task, not recorded during the
run. It names the agent the runner used because selection depends on nothing else
today.

---

## Priority Levels

Tasks have five priority tiers. Higher-priority tasks are dequeued and scheduled first.

```cyr
AGNOSAI_PRIORITY_BACKGROUND = 0;
AGNOSAI_PRIORITY_LOW        = 1;
AGNOSAI_PRIORITY_NORMAL     = 2;   # default
AGNOSAI_PRIORITY_HIGH       = 3;
AGNOSAI_PRIORITY_CRITICAL   = 4;
```

Priority affects:
- **Priority queue scheduling:** The `Scheduler` maintains per-tier FIFO queues and always dequeues from the highest non-empty tier first.
- **DAG wave ordering:** Within a DAG wave, ready tasks are sorted by priority (highest first).
- **Topological sort seeding:** Zero-dependency nodes enter the topological sort ordered by priority.

```cyr
var urgent = agnosai_task_new(str_from("Fix production outage"));
agnosai_task_with_priority(urgent, AGNOSAI_PRIORITY_CRITICAL);

var routine = agnosai_task_new(str_from("Update documentation"));
routine.priority = TaskPriority::Background;

// In a priority-queue scheduler, urgent always runs before routine,
// regardless of insertion order.
```
