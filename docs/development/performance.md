# Performance Testing & Benchmarks

> Benchmark results, testing matrix, and performance targets for AgnosAI.

All benchmarks use [Criterion.rs](https://bheisler.github.io/criterion.rs/book/) with 100-sample
statistical analysis. Results are from the development machine and are relative, not absolute.

Last updated: 2026-03-21 (v0.21.3, hoosh 0.21.4, ai-hwaccel 0.21.3)

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

| Benchmark | Scale | Median | Prev | Delta |
|---|---|---|---|---|
| score_agent (rich context) | 1 agent, 4 tools | 176 ns | 165 ns | +7% |
| score_agent (no context) | 1 agent, 0 tools | 26 ns | 25 ns | +4% |
| rank_agents (10 agents) | 10 agents | 1.79 us | 1.68 us | +6% |
| rank_agents (100 agents) | 100 agents | 18.3 us | 17.7 us | +3% |

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

| Benchmark | Scale | Median | Prev | Delta |
|---|---|---|---|---|
| matches_pattern literal | 3 segments | 75 ns | 106 ns | **-29%** |
| matches_pattern single `*` | 3 segments | 68 ns | 88 ns | **-23%** |
| matches_pattern multi-level `#` | 3 segments | 114 ns | 128 ns | **-11%** |
| matches_pattern deep (10 segments) | 10 segments | 260 ns | 281 ns | **-7%** |
| publish (1 subscriber) | 1 sub | 826 ns | 904 ns | **-9%** |
| publish (10 subscribers) | 10 subs | 1.55 us | 1.53 us | +1% |
| publish (100 subscribers) | 100 subs | 1.45 us | 1.74 us | **-17%** |
| subscribe (new pattern) | — | 140 ns | 122 ns | +15% |

### Fleet Relay (`benches/relay.rs`)

| Benchmark | Scale | Feature | Median |
|---|---|---|---|
| send (targeted) | — | fleet | 158 ns |
| broadcast | — | fleet | 137 ns |
| receive (no dupes) | — | fleet | 223 ns |
| receive (50% dupes) | — | fleet | 152 ns |

### Learning Module (`benches/learning.rs`)

| Benchmark | Scale | Median | Prev | Delta |
|---|---|---|---|---|
| CapabilityScorer record | 50 capabilities | 38 ns | 53 ns | **-28%** |
| CapabilityScorer confidence | 50 capabilities | 18 ns | 25 ns | **-28%** |
| Ucb1 select (10 arms) | 10 arms | 59 ns | 47 ns | +26% |
| Ucb1 select (50 arms) | 50 arms | 268 ns | 250 ns | +7% |
| ReplayBuffer push (full) | 1000 buffer | 637 ns | 750 ns | **-15%** |
| ReplayBuffer sample(32) | 1000 buffer | 42 us | 17 us | +147% |
| QLearner update | 1000 state-actions | 464 ns | 352 ns | +32% |
| QLearner best_action | 1000 state-actions | 432 ns | 438 ns | -1% |
| PerformanceProfile record | 20 agents | 160 ns | 152 ns | +5% |
| PerformanceProfile success_rate | 20 agents | 48 ns | 48 ns | 0% |

### Tool Registry (`benches/tools.rs`)

| Benchmark | Scale | Median | Prev | Delta |
|---|---|---|---|---|
| get (5 tools) | 5 tools | 50 ns | 49 ns | +2% |
| get (50 tools) | 50 tools | 55 ns | 53 ns | +4% |
| list (50 tools) | 50 tools | 8.3 us | 8.5 us | **-2%** |
| register | — | 931 ns | 948 ns | -2% |

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
| **Per-request** | Tool registry lookup | 50 ns | <1 ms | OK |
| **Per-request** | Score + rank 10 agents | 1.79 us | <1 ms | OK |
| **Per-request** | JSON deserialize (agent) | 804 ns | <1 ms | OK |
| **Per-task** | Scheduler dequeue | 32 ns | <100 us | OK |
| **Per-task** | Capability record | 38 ns | <100 us | OK |
| **Per-task** | Q-learning update | 464 ns | <100 us | OK |
| **Per-event** | PubSub publish (10 subs) | 1.55 us | <1 ms | OK |
| **Per-event** | Pattern match | 75 ns | <100 us | OK |
| **Per-msg** | Relay send | 158 ns | <1 ms | OK |
| **Per-msg** | Relay receive + dedup | 223 ns | <1 ms | OK |
| **Per-node** | Fleet placement (50 nodes) | 1.08 us | <1 ms | OK |
| **Startup** | from_hwaccel detection | 39.7 us | <100 ms | OK |
| **Startup** | Load DAG (500 tasks) | 337 us | <100 ms | OK |

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

## Notable Changes (v0.21.3)

**Improved:**
- PubSub pattern matching: -7% to -29% across all patterns
- CapabilityScorer: -28% for both record and confidence
- ReplayBuffer push: -15%
- Scheduler dequeue: -14%

**Regressed (monitoring):**
- ReplayBuffer sample(32): +147% (42us, needs investigation — likely allocation pattern change)
- QLearner update: +32% (464ns, still well under budget)
- Ucb1 select (10 arms): +26% (59ns, still sub-100ns)

All hot-path operations remain well within their budgets.

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
