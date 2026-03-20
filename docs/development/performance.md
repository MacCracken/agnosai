# Performance Testing & Benchmarks

> Benchmark results, testing matrix, and performance targets for AgnosAI.

All benchmarks use [Criterion.rs](https://bheisler.github.io/criterion.rs/book/) with 100-sample
statistical analysis. Results are from the development machine and are relative, not absolute.

Last updated: 2026-03-20

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

| Benchmark | Scale | Feature | Median | Throughput |
|---|---|---|---|---|
| satisfies (empty req) | 10 devices | — | 2.6 ns | 385M/s |
| satisfies (1 device, CUDA req) | 1 device | — | 8.8 ns | 114M/s |
| satisfies (10 devices, CUDA req) | 10 devices | — | 57 ns | 17.5M/s |
| devices_of_type CUDA | 10 devices | — | 51 ns | 19.6M/s |
| total_memory_mb CUDA | 10 devices | — | 53 ns | 18.9M/s |
| from_hwaccel (8 GPUs + CPU) | 9 profiles | hwaccel | 39.7 us | 25K/s |

### Serde Round-Trips (`benches/serde_types.rs`)

| Benchmark | Payload | Median |
|---|---|---|
| HardwareInventory serialize (8 devices) | ~2 KB JSON | 888 ns |
| HardwareInventory deserialize (8 devices) | ~2 KB JSON | 1.17 us |
| Task serialize | ~500 B JSON | 351 ns |
| Task deserialize | ~500 B JSON | 577 ns |
| AgentDefinition from_json | ~400 B JSON | 730 ns |
| ResourceBudget serialize | ~100 B JSON | 89 ns |
| ResourceBudget deserialize | ~100 B JSON | 99 ns |

### Scheduler (`benches/scheduler.rs`)

| Benchmark | Scale | Median |
|---|---|---|
| enqueue 100 tasks | 100 tasks, 5 priorities | 10.4 us |
| dequeue 100 tasks (priority order) | 100 tasks | 3.7 us |
| load_dag linear (50 tasks) | 50 nodes, 49 edges | 33 us |
| load_dag linear (500 tasks) | 500 nodes, 499 edges | 338 us |
| load_dag wide (100 workers) | 102 nodes, 200 edges | 94 us |
| ready_tasks (wide DAG, 100 workers) | 100 workers | 1.9 us |

### Agent Scoring (`benches/scoring.rs`)

| Benchmark | Scale | Median |
|---|---|---|
| score_agent (rich context) | 1 agent, 4 tools | 165 ns |
| score_agent (no context) | 1 agent, 0 tools | 25 ns |
| rank_agents (10 agents) | 10 agents | 1.68 us |
| rank_agents (100 agents) | 100 agents | 17.7 us |

### Fleet Placement (`benches/placement.rs`)

| Benchmark | Scale | Feature | Median |
|---|---|---|---|
| place GpuAffinity | 50 nodes | fleet | 1.08 us |
| place Balanced | 50 nodes | fleet | 1.62 us |
| place Locality (3 caps) | 50 nodes | fleet | 2.68 us |
| place HW requirement | 50 nodes | fleet | 1.26 us |
| rank_nodes GpuAffinity | 200 nodes | fleet | 5.84 us |
| rank_nodes Cost | 200 nodes | fleet | 8.55 us |

### PubSub Pattern Matching (`benches/pubsub.rs`)

| Benchmark | Scale | Median |
|---|---|---|
| matches_pattern literal | 3 segments | 106 ns |
| matches_pattern single `*` | 3 segments | 88 ns |
| matches_pattern multi-level `#` | 3 segments | 128 ns |
| matches_pattern deep (10 segments) | 10 segments | 281 ns |
| publish (1 subscriber) | 1 sub | 904 ns |
| publish (10 subscribers) | 10 subs | 1.53 us |
| publish (100 subscribers) | 100 subs | 1.74 us |
| subscribe (new pattern) | — | 122 ns |

### Fleet Relay (`benches/relay.rs`)

| Benchmark | Scale | Feature | Median |
|---|---|---|---|
| send (targeted) | — | fleet | 158 ns |
| broadcast | — | fleet | 137 ns |
| receive (no dupes) | — | fleet | 223 ns |
| receive (50% dupes) | — | fleet | 152 ns |

### Learning Module (`benches/learning.rs`)

| Benchmark | Scale | Median |
|---|---|---|
| CapabilityScorer record | 50 capabilities | 53 ns |
| CapabilityScorer confidence | 50 capabilities | 25 ns |
| Ucb1 select (10 arms) | 10 arms | 47 ns |
| Ucb1 select (50 arms) | 50 arms | 250 ns |
| ReplayBuffer push (full) | 1000 buffer | 750 ns |
| ReplayBuffer sample(32) | 1000 buffer | 17 us |
| QLearner update | 1000 state-actions | 352 ns |
| QLearner best_action | 1000 state-actions | 438 ns |
| PerformanceProfile record | 20 agents | 152 ns |
| PerformanceProfile success_rate | 20 agents | 48 ns |

### Tool Registry (`benches/tools.rs`)

| Benchmark | Scale | Median |
|---|---|---|
| get (5 tools) | 5 tools | 49 ns |
| get (50 tools) | 50 tools | 53 ns |
| list (50 tools) | 50 tools | 8.5 us |
| register | — | 948 ns |

---

## Feature Impact: `hwaccel`

The `hwaccel` feature adds `ai-hwaccel` for hardware detection and planning.
The `from_hwaccel` benchmark only runs with this feature enabled. All other
benchmarks run identically with or without it — the feature gates are
compile-time only and add no runtime overhead to non-hwaccel code paths.

| Benchmark | Without hwaccel | With hwaccel | Delta |
|---|---|---|---|
| satisfies (10 devices, CUDA req) | 57 ns | 57 ns | 0% |
| satisfies (empty req) | 2.6 ns | 2.6 ns | 0% |
| devices_of_type CUDA | 51 ns | 51 ns | 0% |
| from_hwaccel (8 GPUs + CPU) | N/A | 39.7 us | hwaccel-only |

The `from_hwaccel` conversion (39.7 us) runs once at startup during hardware
detection. It does not affect per-request hot paths.

---

## Performance Summary by Hot Path

| Hot Path | Operation | Per-call | Budget | Status |
|---|---|---|---|---|
| **Per-request** | Tool registry lookup | 49 ns | <1 ms | OK |
| **Per-request** | Score + rank 10 agents | 1.68 us | <1 ms | OK |
| **Per-request** | JSON deserialize (agent) | 730 ns | <1 ms | OK |
| **Per-task** | Scheduler dequeue | 37 ns | <100 us | OK |
| **Per-task** | Capability record | 53 ns | <100 us | OK |
| **Per-task** | Q-learning update | 352 ns | <100 us | OK |
| **Per-event** | PubSub publish (10 subs) | 1.53 us | <1 ms | OK |
| **Per-event** | Pattern match | 106 ns | <100 us | OK |
| **Per-msg** | Relay send | 158 ns | <1 ms | OK |
| **Per-msg** | Relay receive + dedup | 223 ns | <1 ms | OK |
| **Per-node** | Fleet placement (50 nodes) | 1.08 us | <1 ms | OK |
| **Startup** | from_hwaccel detection | 39.7 us | <100 ms | OK |
| **Startup** | Load DAG (500 tasks) | 338 us | <100 ms | OK |

---

## Performance Targets (from Roadmap)

| Metric | v1 (Python/CrewAI) | AgnosAI Target | Current |
|---|---|---|---|
| Container image | ~1.5 GB | <50 MB | TBD (not yet containerized) |
| Boot to ready | 15-30 s | <2 s | TBD |
| Memory (idle) | 300-500 MB | <100 MB | TBD |
| Crew creation | ~500 ms | <10 ms | <1 ms (rank 10 agents + DAG load) |
| Concurrent crews | ~5-10 (GIL) | 100+ | Unlimited (async, no GIL) |
| Fleet msg overhead | ~50 ms | <1 ms | 0.22 us |
| Dependencies | 200+ | ~30 | 22 |
| Python in hot path | 100% | 0% | 0% |

---

## Benchmark Suite

9 benchmark files, 50 individual benchmarks:

| File | Benchmarks | Feature |
|---|---|---|
| `benches/resource.rs` | 6 | hwaccel (partial) |
| `benches/serde_types.rs` | 7 | — |
| `benches/scheduler.rs` | 6 | — |
| `benches/scoring.rs` | 4 | — |
| `benches/placement.rs` | 6 | fleet |
| `benches/pubsub.rs` | 8 | — |
| `benches/relay.rs` | 4 | fleet |
| `benches/learning.rs` | 10 | — |
| `benches/tools.rs` | 4 | — |

---

## How to Update This Document

After running benchmarks, update the median values:

```bash
cargo bench --all-features 2>&1 | grep -B1 'time:' | grep -v '^--$' | paste - -
```

Copy the median values (middle number in the `[low median high]` range) into
the corresponding table cells. Update the "Last updated" date at the top.
