# Performance Testing & Benchmarks

> Benchmark results, testing matrix, and performance targets for AgnosAI.

All benchmarks use [Criterion.rs](https://bheisler.github.io/criterion.rs/book/) with 100-sample
statistical analysis. Results are from the development machine and are relative, not absolute.

Last updated: 2026-03-28 (v1.0.0, hoosh 1.0.0, ai-hwaccel 1.0.0)

---

## Running Benchmarks

```bash
# All benchmarks (requires all features)
cargo bench --all-features

# Individual suites
cargo bench --bench resource --features hwaccel
cargo bench --bench serde_types
cargo bench --bench scheduler
cargo bench --bench scoring
cargo bench --bench placement --features fleet
cargo bench --bench pubsub
cargo bench --bench relay --features fleet
cargo bench --bench learning
cargo bench --bench tools

# Compare with/without hwaccel
cargo bench --bench resource              # without hwaccel (skips from_hwaccel)
cargo bench --bench resource --features hwaccel  # with hwaccel
```

---

## Benchmark Matrix

### Core Resource Operations (`benches/resource.rs`)

| Benchmark | Scale | Feature | Median | Prev (v0.21.3) | Delta |
|---|---|---|---|---|---|
| satisfies (empty req) | 10 devices | — | 5.8 ns | 2.6 ns | +123% (ai-hwaccel 1.0 richer types) |
| satisfies (1 device, CUDA req) | 1 device | — | 18.1 ns | 8.8 ns | +106% (ai-hwaccel 1.0 richer types) |
| satisfies (10 devices, CUDA req) | 10 devices | — | 66 ns | 57 ns | +16% |
| devices_of_type CUDA | 10 devices | — | 120 ns | 51 ns | +135% (more variant matching) |
| total_memory_mb CUDA | 10 devices | — | 69 ns | 53 ns | +30% |
| from_hwaccel (8 GPUs + CPU) | 9 profiles | hwaccel | 46.4 us | 39.7 us | +17% (new accelerator types) |

Note: Resource operation regressions are due to ai-hwaccel 1.0's expanded `AcceleratorType` enum (added CerebrasWse, GraphcoreIpu, GroqLpu, SamsungNpu, MediaTekApu). All still well within budget (<100 us).

### Serde Round-Trips (`benches/serde_types.rs`)

| Benchmark | Payload | Median | Prev | Delta |
|---|---|---|---|---|
| HardwareInventory serialize (8 devices) | ~2 KB JSON | 912 ns | 888 ns | +3% |
| HardwareInventory deserialize (8 devices) | ~2 KB JSON | 1.20 us | 1.17 us | +3% |
| Task serialize | ~500 B JSON | 342 ns | 351 ns | **-3%** |
| Task deserialize | ~500 B JSON | 600 ns | 577 ns | +4% |
| AgentDefinition from_json | ~400 B JSON | 804 ns | 730 ns | +10% |
| ResourceBudget serialize | ~100 B JSON | 91 ns | 89 ns | +2% |
| ResourceBudget deserialize | ~100 B JSON | 101 ns | 99 ns | +2% |

### Scheduler (`benches/scheduler.rs`)

| Benchmark | Scale | Median | Prev | Delta |
|---|---|---|---|---|
| enqueue 100 tasks | 100 tasks, 5 priorities | 10.5 us | 10.4 us | +1% |
| dequeue 100 tasks (priority order) | 100 tasks | 3.2 us | 3.7 us | **-14%** |
| load_dag linear (50 tasks) | 50 nodes, 49 edges | 33 us | 33 us | 0% |
| load_dag linear (500 tasks) | 500 nodes, 499 edges | 337 us | 338 us | 0% |
| load_dag wide (100 workers) | 102 nodes, 200 edges | 95 us | 94 us | +1% |
| ready_tasks (wide DAG, 100 workers) | 100 workers | 1.95 us | 1.9 us | +3% |

### Agent Scoring (`benches/scoring.rs`)

| Benchmark | Scale | Median | Prev (v0.21.3) | Delta |
|---|---|---|---|---|
| score_agent (rich context) | 1 agent, 4 tools | 202 ns | 176 ns | +15% |
| score_agent (no context) | 1 agent, 0 tools | 27 ns | 26 ns | +4% |
| score_agent (GPU required) | 1 agent | 84 ns | — | new |
| score_agent (domain mismatch) | 1 agent | 147 ns | — | new |
| pick_best_agent (rank + select) | 10 agents | 663 ns | — | new |
| rank_agents (10 agents) | 10 agents | 823 ns | 1.79 us | **-54%** |
| rank_agents (100 agents) | 100 agents | 7.74 us | 18.3 us | **-58%** |
| rank_agents (1000 agents) | 1000 agents | 76.2 us | — | new |

### Fleet Placement (`benches/placement.rs`)

| Benchmark | Scale | Feature | Median | Prev (v0.21.3) | Delta |
|---|---|---|---|---|---|
| place GpuAffinity | 50 nodes | fleet | 1.17 us | 1.08 us | +8% |
| place Balanced | 50 nodes | fleet | 1.73 us | 1.62 us | +7% |
| place Locality (3 caps) | 50 nodes | fleet | 2.93 us | 2.68 us | +9% |
| place HW requirement | 50 nodes | fleet | 1.37 us | 1.26 us | +9% |
| rank_nodes GpuAffinity | 200 nodes | fleet | 5.80 us | 5.84 us | 0% |
| rank_nodes Cost | 200 nodes | fleet | 17.5 us | 8.55 us | +105% |

### PubSub Pattern Matching (`benches/pubsub.rs`)

| Benchmark | Scale | Median | Prev (v0.21.3) | Delta |
|---|---|---|---|---|
| matches_pattern literal | 3 segments | 85 ns | 76 ns | +12% |
| matches_pattern single `*` | 3 segments | 83 ns | 69 ns | +20% |
| matches_pattern multi-level `#` | 3 segments | 139 ns | 116 ns | +20% |
| matches_pattern deep (10 segments) | 10 segments | 604 ns | 259 ns | +133% |
| publish (1 subscriber) | 1 sub | 1.60 us | 818 ns | +96% |
| publish (10 subscribers) | 10 subs | 1.29 us | 1.15 us | +12% |
| publish (100 subscribers) | 100 subs | 1.50 us | 1.35 us | +11% |
| subscribe (new pattern) | — | 332 ns | 136 ns | +144% |

Note: PubSub regressions vs v0.21.3 reflect majra 1.0.1's expanded API surface (relay integration, dedup). The v0.21.3 numbers were themselves significant improvements over v0.20.3. All operations remain well under 1ms budget.

### Fleet Relay (`benches/relay.rs`)

| Benchmark | Scale | Feature | Median | Prev (v0.21.3) | Delta |
|---|---|---|---|---|---|
| send (targeted) | — | fleet | 161 ns | 158 ns | +2% |
| broadcast | — | fleet | 152 ns | 137 ns | +11% |
| receive (no dupes) | — | fleet | 233 ns | 223 ns | +4% |
| receive (50% dupes) | — | fleet | 176 ns | 152 ns | +16% |
| send (64 KiB payload) | — | fleet | 6.61 us | — | new |
| stats (after 100 sends) | — | fleet | 4.5 ns | — | new |

### Learning Module (`benches/learning.rs`)

| Benchmark | Scale | Median | Prev (v0.21.3) | Delta |
|---|---|---|---|---|
| CapabilityScorer record | 50 capabilities | 54 ns | 38 ns | +42% |
| CapabilityScorer confidence | 50 capabilities | 19 ns | 17 ns | +12% |
| Ucb1 select (10 arms) | 10 arms | 47 ns | 44 ns | +7% |
| Ucb1 select (50 arms) | 50 arms | 252 ns | 236 ns | +7% |
| ReplayBuffer push (full) | 1000 buffer | 609 ns | 592 ns | +3% |
| ReplayBuffer sample(32) | 1000 buffer | 39.9 us | 18 us | +122% |
| QLearner update | 1000 state-actions | 456 ns | 411 ns | +11% |
| QLearner best_action | 1000 state-actions | 451 ns | 426 ns | +6% |
| PerformanceProfile record | 20 agents | 11.1 us | 154 ns | regression (investigating) |
| PerformanceProfile success_rate | 20 agents | 49 ns | 47 ns | +4% |

### Tool Registry (`benches/tools.rs`)

| Benchmark | Scale | Median | Prev (v0.21.3) | Delta |
|---|---|---|---|---|
| get (5 tools) | 5 tools | 54 ns | 50 ns | +8% |
| get (50 tools) | 50 tools | 59 ns | 55 ns | +7% |
| get (500 tools) | 500 tools | 59 ns | — | new |
| has (50 tools, hit) | 50 tools | 24 ns | — | new |
| has (50 tools, miss) | 50 tools | 18 ns | — | new |
| list (50 tools) | 50 tools | 8.96 us | 8.3 us | +8% |
| list (500 tools) | 500 tools | 81.7 us | — | new |
| register | — | 392 ns | 931 ns | **-58%** |
| EchoTool::execute | — | 84 ns | — | new |

---

## Feature Impact: `hwaccel`

The `hwaccel` feature adds `ai-hwaccel` for hardware detection and planning.
The `from_hwaccel` benchmark only runs with this feature enabled. All other
benchmarks run identically with or without it — the feature gates are
compile-time only and add no runtime overhead to non-hwaccel code paths.

| Benchmark | Without hwaccel | With hwaccel | Delta |
|---|---|---|---|
| satisfies (10 devices, CUDA req) | 66 ns | 66 ns | 0% |
| satisfies (empty req) | 5.8 ns | 5.8 ns | 0% |
| devices_of_type CUDA | 120 ns | 120 ns | 0% |
| from_hwaccel (8 GPUs + CPU) | N/A | 46.4 us | hwaccel-only |

The `from_hwaccel` conversion (46.4 us) runs once at startup during hardware
detection. It does not affect per-request hot paths.

---

## Performance Summary by Hot Path

| Hot Path | Operation | Per-call | Budget | Status |
|---|---|---|---|---|
| **Per-request** | Tool registry lookup | 59 ns | <1 ms | OK |
| **Per-request** | Score + rank 10 agents | 823 ns | <1 ms | OK |
| **Per-request** | JSON deserialize (agent) | 791 ns | <1 ms | OK |
| **Per-request** | Orchestrator::new | 682 ns | <1 ms | OK |
| **Per-request** | GET /health | 1.03 us | <1 ms | OK |
| **Per-request** | POST /mcp | 3.49 us | <10 ms | OK |
| **Per-task** | Scheduler ready_tasks | 2.04 us | <100 us | OK |
| **Per-task** | Capability record | 54 ns | <100 us | OK |
| **Per-task** | Q-learning update | 456 ns | <100 us | OK |
| **Per-event** | PubSub publish (10 subs) | 1.29 us | <1 ms | OK |
| **Per-event** | Pattern match | 85 ns | <100 us | OK |
| **Per-msg** | Relay send | 161 ns | <1 ms | OK |
| **Per-msg** | Relay receive + dedup | 233 ns | <1 ms | OK |
| **Per-node** | Fleet placement (50 nodes) | 1.17 us | <1 ms | OK |
| **Per-crew** | run_crew (1 task) | 19 us | <10 ms | OK |
| **Per-crew** | run_crew (10 parallel) | 79 us | <100 ms | OK |
| **Startup** | from_hwaccel detection | 46.4 us | <100 ms | OK |
| **Startup** | Audit chain verify (1K) | 1.06 ms | <1 s | OK |

---

## Performance Targets (from Roadmap)

| Metric | v1 (Python/CrewAI) | AgnosAI Target | Current (v1.0.0) |
|---|---|---|---|
| Container image | ~1.5 GB | <50 MB | TBD (not yet containerized) |
| Boot to ready | 15-30 s | <2 s | <1 s (measured) |
| Memory (idle) | 300-500 MB | <100 MB | ~11 MB RSS (measured) |
| Crew creation | ~500 ms | <10 ms | <1 ms (rank 10 agents + DAG load) |
| Concurrent crews | ~5-10 (GIL) | 100+ | Unlimited (async, no GIL) |
| Fleet msg overhead | ~50 ms | <1 ms | 0.23 us |
| Dependencies | 200+ | ~30 | ~30 |
| Python in hot path | 100% | 0% | 0% |

---

## Notable Changes (v0.21.3 → v1.0.0)

**Major improvements:**
- rank_agents (10): 1.79 us → 823 ns (**-54%**) — pre-extracted tool sets
- rank_agents (100): 18.3 us → 7.74 us (**-58%**)
- ToolRegistry register: 931 ns → 392 ns (**-58%**)
- GET /health: 1.03 us (was ~1.8 us, **-43%** from v0.21.3 server bench)
- POST /mcp: 3.49 us (was ~5.8 us, **-40%**)
- EchoTool::execute: 84 ns (new, ~37% faster than v0.21.3)

**Regressions (expected — richer type systems in 1.0 deps):**
- ai-hwaccel 1.0: satisfies/devices_of_type operations +16-135% due to expanded AcceleratorType enum (5 new variants). Still <120 ns, well within budget
- majra 1.0.1: PubSub subscribe +144%, deep pattern match +133% due to relay integration and dedup. All under 1.6 us
- rank_nodes Cost: +105% (17.5 us vs 8.55 us) — richer cost model
- PerformanceProfile record: 154 ns → 11.1 us — investigating regression
- ReplayBuffer sample(32): 18 us → 39.9 us — investigating

**New benchmarks (19 bench files, 90 total):**
- Orchestrator: new, run_crew (1 task: 19 us, 10 parallel: 79 us)
- Server: GET /health, /ready, /metrics, POST /mcp, /crews
- Prompt guard: scan_input, sanitize
- Approval gates: request, submit
- Audit chain: record, verify (100/1000 entries)
- Sandbox: ProcessSandbox, WasmSandbox execute
- IPC: round-trip, throughput, large payload
- Fleet: coordinator fan_out, task_completed, compute scheduler
- Definitions: load_preset, assemble_team, builtin_presets
- Tools: has(), get(500), list(500), EchoTool

All hot-path operations remain within their budgets.

---

## Benchmark Suite

19 benchmark files, 90 individual benchmarks:

| File | Benchmarks | Feature |
|---|---|---|
| `benches/resource.rs` | 6 | hwaccel (partial) |
| `benches/serde_types.rs` | 7 | — |
| `benches/scheduler.rs` | 6 | — |
| `benches/scoring.rs` | 8 | — |
| `benches/placement.rs` | 6 | fleet |
| `benches/pubsub.rs` | 8 | — |
| `benches/relay.rs` | 6 | fleet |
| `benches/learning.rs` | 10 | — |
| `benches/tools.rs` | 9 | — |
| `benches/server.rs` | 5 | — |
| `benches/orchestrator.rs` | 3 | — |
| `benches/prompt_guard.rs` | 3 | — |
| `benches/approval.rs` | 2 | — |
| `benches/audit.rs` | 3 | — |
| `benches/sandbox.rs` | 3 | sandbox |
| `benches/ipc.rs` | 3 | — |
| `benches/fleet.rs` | 5 | fleet |
| `benches/definitions.rs` | 4 | definitions |
| `benches/llm_router.rs` | — | — |

---

## How to Update This Document

After running benchmarks, update the median values:

```bash
cargo bench --all-features 2>&1 | grep -B1 'time:' | grep -v '^--$' | paste - -
```

Copy the median values (middle number in the `[low median high]` range) into
the corresponding table cells. Update the "Last updated" date at the top.
