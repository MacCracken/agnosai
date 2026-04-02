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
                    │   (core engine — Rust crate)          │
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

AgnosAI is a single crate with feature-gated modules:

```
agnosai
  ├── core           (foundation types — agents, tasks, crews, errors)
  ├── orchestrator   (scheduling, scoring, pub/sub, crew runner)
  ├── llm            (hoosh re-exports, task-complexity routing)
  ├── fleet          (distribution, placement, relay, GPU) [feature: fleet]
  ├── sandbox        (WASM, process isolation) [feature: sandbox]
  ├── tools          (tool trait, registry, builtins)
  ├── learning       (RL, profiling, capability scoring)
  ├── definitions    (loader, assembler, versioning, packaging) [feature: definitions]
  └── server         (axum HTTP, MCP, A2A, SSE)
```

## Key Data Flow

### Crew Execution

```
CrewSpec (JSON/YAML)
    │
    ▼
CrewRunner::new(spec)
    │
    ├── ProcessMode::Sequential → run tasks in order
    ├── ProcessMode::Parallel   → JoinSet + Semaphore(max_concurrency)
    ├── ProcessMode::DAG        → topological sort → wave execution
    └── ProcessMode::Hierarchical → manager delegation (falls back to sequential)
    │
    ├── For each task:
    │   ├── score_agent() → rank agents by suitability (4-factor weighted)
    │   ├── pick_best_agent() → assign highest-scoring agent
    │   └── execute_task() → LLM inference via hoosh + response caching
    │
    ▼
CrewState { crew_id, status, results, profile }
```

### Agent Scoring

Five weighted factors (0.0–1.0 each):

| Factor | Weight | Source |
|--------|--------|--------|
| Tool coverage | 0.35 | Fraction of required tools the agent provides |
| Complexity alignment | 0.25 | How well agent/task complexity levels match |
| Domain match | 0.15 | Domain compatibility |
| Personality fit | 0.15 | Personality trait alignment via bhava |
| GPU match | 0.10 | GPU capability when task requires it |

### Task DAG Resolution

- Kahn's algorithm for topological sort with cycle detection
- Priority-aware: within each wave, higher-priority tasks run first
- Wave execution: all tasks with satisfied dependencies run concurrently

### Pub/Sub

- Topic-based with wildcard patterns: `*` (one segment), `#` (zero or more)
- `tokio::sync::broadcast` channels per subscription pattern (capacity 256)
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
  "complexity": "high",
  "llm_model": "capable"
}
```

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
| `agnosai-server` (full) | ~15-25 MB | <2s | 50-150 MB |
| Compare: Agnostic v1 | ~1.5 GB | 15-30s | 300-500 MB |
