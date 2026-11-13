# Architecture Overview

## System Context

```
                    ┌─────────────────────────────────────┐
                    │           SecureYeoman               │
                    │   (A2A / MCP — wire-compatible)      │
                    └──────────────┬──────────────────────┘
                                   │
                    ┌──────────────▼──────────────────────┐
                    │            Agnostic                   │
                    │   (downstream — depends on agnosai)   │
                    └──────────────┬──────────────────────┘
                                   │
                    ┌──────────────▼──────────────────────┐
                    │            AgnosAI                    │
                    │   (core engine — Cyrius binary)       │
                    └──┬───────────┬───────────┬──────────┘
                       │           │           │
              ┌────────▼──┐  ┌────▼─────┐  ┌──▼───────┐
              │  Synapse   │  │  Mneme   │  │  Delta   │
              │ LLM infra  │  │ KB/search│  │ code/CI  │
              └────────────┘  └──────────┘  └──────────┘
                    (optional throughput tools)
```

- **Agnostic** depends on `agnosai` (replaces CrewAI)
- **SecureYeoman** talks to Agnostic via A2A/MCP — same wire protocol, faster backend
- **Synapse/Mneme/Delta** are optional HTTP tool backends for agents

## Module Structure

`src/` mirrors `rust-old/src/` file for file. ⚠ Cargo features have no Cyrius
equivalent and are **not** a scope boundary — every module is built
unconditionally:

```
agnosai
  ├── core           (foundation types — agents, tasks, crews, errors)
  ├── orchestrator   (scheduling, scoring, pub/sub, crew runner)
  ├── llm            (hoosh seam, task-complexity routing, model catalog)
  ├── fleet          (registry, placement, GPU, state, relay, discovery,
  │                   coordinator, federation, topology, cost, environment)
  │                                                        [feature: fleet]
  ├── sandbox        (WASM, cx, OCI, and process and Python under kavach's confinement, ADR 031)
  ├── tools          (tool trait, registry, builtins)
  ├── learning       (RL, profiling, capability scoring)
  ├── definitions    (loader, assembler, versioning, packaging) [feature: definitions]
  └── server         (axum HTTP, MCP, A2A, SSE)
```

⚠ The `[feature: …]` markers describe **`rust-old/`'s `Cargo.toml`**, which is
useful when reading the oracle. They are **not** the Cyrius port's scope: the
port builds every module unconditionally, because a cargo feature gate is not a
scope boundary. `fleet` is fully ported as of 2026-08-08 — all twelve of the
oracle's modules — and `src/fleet/mod.cyr` is included from `src/main.cyr` with
no gate. See [`roadmap.md`](../development/roadmap.md) for what remains.

## Key Data Flow

### Crew Execution

```
CrewSpec (JSON/YAML)
    │
    ▼
admit (ADR 033): task cap, tenant known / not spent / under its crews
    │
    ▼
wait for a turn: at most max_concurrent_tasks crews at once, first come first served
    │
    ▼
CrewRunner::new(spec), with a budget tracker under its tenant's
    │
    ├── ProcessMode::Sequential → run tasks in order
    ├── ProcessMode::Parallel   → a sliding window of max_concurrency
    ├── ProcessMode::DAG        → topological sort → each wave a sliding window
    └── ProcessMode::Hierarchical → workers in order, each within its manager's tools → the manager's answer
    │
    ├── For each task:
    │   ├── score_agent() → rank agents by suitability (5-factor weighted)
    │   ├── pick_best_agent() → assign highest-scoring agent
    │   └── execute_task() → LLM inference via hoosh, each call checked against
    │       the task's, crew's and tenant's budgets first, and its https exchange
    │       one of 64 TLS permits (ADR 033)
    │       ├── model (ADR 030): the agent's own, else the operator's for its
    │       │   tier, else the tier's in hoosh's catalog, else the task fails
    │       └── tool loop (ADR 029): while the model asks for the tools its agent
    │           was granted: prepare → approve the set → journal → run → deliver
    │
    ▼
CrewState { crew_id, status, results, profile }
```

### The crew log and resuming (2.5.0, [ADR 036](../adr/036-a-crew-keeps-a-log-and-resumes-from-it.md))

```
register ─▶ log: {"crew": spec}                      synced, through a StateStore backend
run      ─▶ every event: a tap on the crew's sender  numbered; task_completed carries its result
         ─▶ each tool call: the log's journal first  synced before the call runs
         ─▶ crew_completed carries the crew's state  synced
restart  ─▶ agnosai_orchestrator_recover: each log
            ├── ended      → back in the registry as it ended
            └── not ended  → resumed: finished tasks stand, an answer is asked about again,
                             a task with calls whose effect may stand waits for a reviewer,
                             the rest runs, its events numbered on from the log's last seq
```

Every move of a task's status goes with the event that reports it under the bus lock, and a snapshot
(`GET /api/v1/crews/{id}/snapshot`) reads them under it, so a reader that fell behind takes each task as
the snapshot says and goes on from the first event after its `seq`.

### Agent Scoring

Five weighted factors (0.0–1.0 each):

| Factor | Weight | Source |
|--------|--------|--------|
| Tool coverage | 0.35 | Fraction of required tools the agent provides |
| Complexity alignment | 0.25 | How well agent/task complexity levels match |
| Domain match | 0.15 | Domain compatibility |
| Personality fit | 0.15 | Neutral 0.5 for every agent until bhava is ported (reported as `unmeasured`) |
| GPU match | 0.10 | GPU capability when task requires it |

The total is the weighted sum in the order tool, complexity, GPU, domain,
personality (`_agnosai_score_fold`), clamped to 0..1. A perfectly matched agent
scores 0.925, not 1.0, because personality contributes only its neutral half.

#### Explaining a selection

`agnosai_explain_selection_a(a, agents, task)` returns every agent's total and
its five component scores, in the runner's rank order, so entry 0 is exactly the
agent the crew runner assigns. `agnosai_selection_to_value_a` renders that as
JSON (`winner`, `candidate_count`, `candidates[]` with `index`, `agent_key`,
`total` and `scores`), and `agnosai_selection_scorer_to_value_a` renders the
weights and `unmeasured: ["personality"]`. All three allocate in the caller's
allocator only. agnosai renders the JSON so consumers do not hardcode factor names.

It is a library call (roadmap F7), and no route serves it. The explanation is
**recomputed** from the roster and the task, not recorded during the run, and that
is exact only while selection is a pure function of those two. A hierarchical
crew keeps it one (2.4.3, ADR 035): its run ranks the workers
`agnosai_hierarchical_workers_a` returns, so the explanation over those is exact.
Learning-driven selection or a stateful bhava personality would end that, and
would have to record the choice on the run instead. agnostic is to
surface the explanation on its plan route (`?explain=selection`) when it re-pins.

### Task DAG Resolution

- Kahn's algorithm for topological sort with cycle detection
- Priority-aware: within each wave, higher-priority tasks run first
- Wave execution: all tasks with satisfied dependencies run concurrently
- Hand-off (2.4.2): each task is handed its dependencies' answers, as data, never their tool
  transcripts (`src/handoff.cyr`, [ADR 034](../adr/034-a-dependent-task-is-handed-its-dependencies-answers.md))

### Pub/Sub

- Topic-based with wildcard patterns: `*` (one segment), `#` (zero or more)
- One bounded channel per subscription pattern. ⚠ Not `tokio::sync::broadcast` —
  the port has no tokio. majra's relay backs it, and since majra 2.6.5 the
  capacity a caller asks for is the capacity it gets (it was hardcoded to 256
  before that). A full ring **drops** rather than blocking the sender, matching
  `broadcast::send`, which never blocks and reports `Lagged` to a slow receiver.
- `DashMap` for concurrent subscription management

## Configuration

Agent definitions are JSON/YAML — same format as Agnostic v1 presets:

```json
{
  "agent_key": "senior-qa-engineer",
  "name": "Senior QA Engineer",
  "role": "Senior QA Engineer",
  "goal": "Ensure comprehensive test coverage",
  "domain": "quality",
  "tools": ["code_analysis", "test_generation"],
  "complexity": "high"
}
```

`llm_model` is optional, and is an id as hoosh lists it. With none, `complexity` picks a tier and
hoosh's catalog says what that tier means, and, for an agent with `tools`, which of its models take them
([ADR 030](../adr/030-a-model-comes-from-the-gateways-catalog.md) and its 2.3.3 addendum). `capable` is not a model: hoosh
has no tier aliases, and the word would be sent as it stands.

## Binary Distribution

Single static binary via release profile:

```toml
[profile.release]
opt-level = 2
lto = "fat"
strip = true
panic = "abort"
codegen-units = 1
```

| Target | Size (est.) | Boot | Memory |
|--------|-------------|------|--------|
| `agnosai` (full) | ~15-25 MB | <2s | 50-150 MB |
| Compare: Agnostic v1 | ~1.5 GB | 15-30s | 300-500 MB |
