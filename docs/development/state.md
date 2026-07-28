# AgnosAI — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures
> (durable); this file is **state** (volatile).
> Last refreshed: 2026-07-28.

## Version

**1.1.0** (`VERSION`) — the last shipped Rust release, now preserved at
`rust-old/`. The Cyrius line targets **v2.0.0**; VERSION bumps once parity
lands, not before, so the number always names something that actually shipped.

## Toolchain

- **Cyrius pin**: `6.4.86` (`cyrius.cyml`) — folds sandhi 1.9.5
- Rust (for `rust-old/` only): `channel = "stable"`, currently rustc 1.96.0

## Source

- **Rust reference**: 27,683 lines at `rust-old/` — frozen, do not edit. It is the parity oracle.
- **Cyrius port**: `learning` landed (5 modules + hub, ~1,000 lines). `src/main.cyr` is still the
  stub entry point — no CLI surface yet.

## Where the port is

Phase 0 (M1 scaffold) — **complete**. Phase 1 (M2 beachhead) — **`learning` done, `core` next**.

| Gate | Status |
|---|---|
| Rust baseline green (the oracle) | ✅ fmt + clippy clean, all 9 feature combos compile, 863 + 2 + 1 tests pass |
| Terminal Rust benchmark capture | ✅ 112 rows at v1.1.0, frozen into `rust-old/bench-history.csv` |
| Blockers re-verified vs 6.4.86 | ✅ see [`cyrius-port-plan.md`](cyrius-port-plan.md) |
| `cyrius port` run | ✅ 2026-07-28 |
| `cyrius lib sync` + `cyrius deps` | ✅ 43/43 stdlib modules, 9 deps resolved, 0 errors, 79 locked |
| Hello-world builds and runs | ✅ `cyrius build src/main.cyr build/agnosai` → OK |
| `src/id.cyr` (uuid v4/v5) | ⬜ not started |
| `src/order.cyr` (`ai_sort` / `ai_select_nth`) | ⬜ not started |
| Tool-sandbox approach decided | ✅ cx + kavach — [ADR-006](../adr/006-cx-tool-sandbox.md) |
| `learning` ported (M2, Phase 1) | ✅ 5 modules + hub, 112 assertions, 100% reference coverage |
| `core` ported (M2, Phase 1) | ⬜ not started |

## Tests

**112 assertions across 5 `.tcyr` suites, all passing** (plus the 2-assertion scaffold smoke):

| Suite | Assertions | Oracle |
|---|---|---|
| `learning_capability.tcyr` | 18 | 8 `#[cfg(test)]` tests |
| `learning_profile.tcyr` | 23 | 7 |
| `learning_strategy.tcyr` | 19 | 6 |
| `learning_replay.tcyr` | 29 | 7 |
| `learning_optimizer.tcyr` | 21 | 7 |

The Cyrius suites deliberately exceed the oracle's coverage: they also pin the UCB1 formula
itself, the `max_by` last-wins tie rule, replay's zero-priority and NaN fallback branches, and
the Q-table's packed-key distinctness — none of which the Rust tests reach.

`cyrius coverage --min 80` → **100% (56/56 fns), gate OK**. Shared assertion helpers live in
`tests/test_helpers.cyr` (all `_t_`-prefixed, so they can never shadow a `src/` symbol and stay
out of the coverage denominator).

Every `.tcyr` ends `var f = main(); if (f > 0) { f = 1; } syscall(60, f);` — the stock
`proj-tcyr` epilogue masks the exit code `& 0xFF`, so exactly 256/512/768 failures score PASS.

Rust oracle for comparison: **863 unit + 2 integration + 1 doctest, all passing.**

## Benchmarks

`benches/learning.bcyr` — the 10 shapes of `rust-old/benches/learning.rs`. First Cyrius numbers
(x86_64 Linux, cyrius 6.4.86):

| Benchmark | Time |
|---|---|
| `capability_scorer_confidence_50caps` | 81 ns |
| `ucb1_select_10arms` | 250 ns |
| `capability_scorer_record_50caps` | 284 ns |
| `profile_success_rate_20agents` | 337 ns |
| `ucb1_select_50arms` | 1.13 µs |
| `qlearner_best_action_1000_state_actions` | 1.27 µs |
| `profile_record_20agents` | 1.84 µs |
| `qlearner_update_1000_state_actions` | 1.92 µs |
| `replay_buffer_push_1000cap_full` | 7.18 µs |
| `replay_buffer_sample32_from1000` | 583 µs |

The two replay figures are the oracle's own algorithmic cost, not a port regression: `push` runs
an O(n) lowest-priority scan on every insert once full, and `sample` recomputes the remaining
priority sum each round — which `replay.rs:90` does deliberately, "to avoid bias". Changing
either would diverge from parity and needs an ADR first.

**Not comparable to `rust-old/bench-history.csv`** — different allocator, different harness, no
criterion statistics. The Cyrius line starts its own baseline.

## Dependencies

**stdlib** (43 declared, order-sensitive — rationale in [`cyrius-port-plan.md`](cyrius-port-plan.md)):
base substrate · general utilities · bayan · patra · concurrency+crypto floor ·
dynamic-link floor · async · net/http/tls/ws/sakshi/sandhi

**git deps** (declare-ahead pattern): sigil 3.12.1 · bote 3.1.4 (core profile) ·
majra 2.5.3 · kavach 3.9.3 · ai-hwaccel 2.3.15 · tyche 1.0.0.
libro 2.8.2 arrives transitively via bote.

## Known issues in the current build

1. **35 duplicate-fn warnings at build — all benign, but know the rule.**
   Cyrius is single-pass: a redefinition only rebinds call sites parsed *after*
   it. A dep's internal calls therefore keep binding to its own definition even
   when a later bundle redefines the name. Verified by probe — majra's
   `pubsub_subscribe` works correctly with libro in the same build, despite both
   defining `_sub_new` with different arity.

   **What this does mean for us:** agnosai's own modules are included last, so
   *our* calls bind to the last definition of everything. When we call a name
   that appears in more than one bundle, check which one wins. The current
   duplicates: 33 are kavach re-exporting symbols sigil also defines, with
   byte-identical bodies (no behavioural difference either way); `_sub_new`
   (majra/libro — we should never call it, it is a dep-private helper); and
   `path_exists` (ai-hwaccel uses `file_exists`, kavach uses `sys_access` —
   same contract, different implementation, **kavach's wins for our code**).

## Consumers

Agnostic (Python platform), daimon (agent orchestration), joshua (NPC AI),
kiran (game AI) — none consuming the Cyrius line yet.

## Next

`core` — the second half of M2 in [`roadmap.md`](roadmap.md). The root of the
graph: pure data, the vocabulary every other group speaks. Its BITE 8 is gated
on the **money representation** open question (integer micro-USD is the
recommendation) — see the open questions in
[`cyrius-port-plan.md`](cyrius-port-plan.md).

Two items surfaced by the `learning` bite, neither blocking:

1. **`scripts/bench-history.sh` still drives `cargo bench`.** It parses criterion
   output into `bench-history.csv`, so it cannot record Cyrius numbers — the
   root `bench-history.csv` does not exist yet. Driving `cyrius bench` instead
   is a rewrite of the parser half.
2. **`lib/sakshi.cyr` is vendored at 2.4.3 while the 6.4.86 toolchain bundles
   2.4.6**, which every build reports as a shadow warning. `cyrius lib sync
   --full` re-syncs.
