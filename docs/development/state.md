# AgnosAI — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures
> (durable); this file is **state** (volatile).
> Last refreshed: 2026-07-30.

## Version

**1.1.0** (`VERSION`) — the last shipped Rust release, now preserved at
`rust-old/`. The Cyrius line targets **v2.0.0**; VERSION bumps once parity
lands, not before, so the number always names something that actually shipped.

## Toolchain

- **Cyrius pin**: `6.5.3` (`cyrius.cyml`) — folds bayan 1.3.0, sandhi 1.9.7, sakshi 2.4.7.
  Bumped from 6.5.2 on 2026-07-30: `lib/` is **byte-identical** between the two tags
  (`git diff 6.5.2 6.5.3 -- lib/` is empty), so the bump moves no stdlib source. It is
  bugfix-only — correct diagnostic line numbers after an `include`, and an `install.sh`
  fix — and it clears the `manifest-pin: 6.5.2 (drift)` banner the installed CLI printed
  on every invocation
- Rust (for `rust-old/` only): `channel = "stable"`, currently rustc 1.96.0

## Source

- **Rust reference**: 27,683 lines at `rust-old/` — frozen, do not edit. It is the parity oracle.
- **Cyrius port**: `learning` (5 modules + hub), `core` (6 modules + hub + shared helpers),
  `llm` (router, retry, hoosh seam client + hub), `tools` (native, registry, echo,
  json_transform, load_testing, security_audit, and the nine AGNOS ecosystem tools over a
  shared client) + `tools/remote_registry`, and `orch` (15 modules + hub). Five `server`
  modules have landed — `server_ssrf`, `server_prompt_guard`, `server_sse`,
  `server_output_filter`, `server_prometheus`. Support modules: `src/id.cyr`,
  `src/units.cyr`, `src/order.cyr`, `src/chan_lossy.cyr` and `src/guarded_fetch.cyr`.
  `src/main.cyr` is still the stub entry point — no CLI surface yet.

## Where the port is

Phase 0 (M1 scaffold) — **complete**. Phase 1 (M2 beachhead) — **complete**: `learning` and
`core` both ported and green against the oracle. Phase 2 (M3 `llm`) — **complete**: router, retry
and the hoosh seam client, with the live round trip verified. Phase 3 (M4 `tools`) —
**complete**.

| Gate | Status |
|---|---|
| Rust baseline green (the oracle) | ✅ fmt + clippy clean, all 9 feature combos compile, 863 + 2 + 1 tests pass |
| Terminal Rust benchmark capture | ✅ 112 rows at v1.1.0, frozen into `rust-old/bench-history.csv` |
| Blockers re-verified vs 6.5.2 | ✅ see [`cyrius-port-plan.md`](cyrius-port-plan.md) |
| `cyrius port` run | ✅ 2026-07-28 |
| `cyrius lib sync` + `cyrius deps` | ✅ 43/43 stdlib modules, 9 deps resolved, 0 errors, 105 locked |
| Hello-world builds and runs | ✅ `cyrius build src/main.cyr build/agnosai` → OK |
| `src/id.cyr` (uuid v4/v5) | ✅ v4 + v5, verified against the published RFC 4122 vector |
| `src/order.cyr` (sort / select_nth) | ✅ heapsort + quickselect — blocker #8 closed, benchmarked |
| Tool-sandbox approach decided | ✅ cx + kavach — [ADR-006](../adr/006-cx-tool-sandbox.md) |
| `learning` ported (M2, Phase 1) | ✅ 5 modules + hub, 112 assertions, 100% reference coverage |
| `core` ported (M2, Phase 1) | ✅ 6 of 6 + shared `core_json` helpers |
| Money representation decided | ✅ integer micro-USD (2026-07-28) — gates core BITE 8 |
| `llm` ported (M3, Phase 2) | ✅ router + retry + hoosh seam client |
| M3 exit: live chat-completion round trip | ✅ verified through `agnosai_hoosh_chat` (`scripts/stack.sh check`) |
| `tools` ported (M4, Phase 3) | ✅ **complete** — native, registry, all 12 builtins, remote_registry |
| ADR 007 shared, not copied | ✅ `src/guarded_fetch.cyr` — extracted at the second consumer, since two copies of a security control drift silently |
| `orchestrator` ported (M5, Phase 4) | ✅ **COMPLETE** — all 15 modules, plus `server/sse` and `server/prompt_guard` pulled forward and an `orch_audit` chain the seam cannot delegate |
| `server` ported (M6, Phase 5) | 🟡 **5.5 of 21 files** — the pure-leaf sequence is done (`ssrf`, `prompt_guard`, `sse` came forward as M5 blockers; `output_filter` and `prometheus` are M6 bites 1-2), and `auth`'s **shared-secret half** is bite 3. Two remainders inside counted files: `sse.rs::event_stream` (`sse.rs:106-126`) is held back by `src/server_sse.cyr:9-11` for the transport tier, and `auth.rs`'s RS256 half is bite 4 — its branch is a loud 500, never a silent pass |
| `server/auth` shared-secret half | ✅ 5 oracle tests + 47 beyond; constant-time compare fixed vs the oracle ([ADR 009](../adr/009-auth-constant-time-secret-compare.md)) |
| Blocker #4 closed | ✅ `src/chan_lossy.cyr` — `agnosai_chan_push_lossy` gives tokio broadcast's never-block, evict-oldest contract over the public channel verbs |
| SSRF-via-redirect closed ([ADR 007](../adr/007-audit-redirect-revalidation.md)) | ✅ the guard re-runs on every hop — the oracle checks only the URL the caller supplied |
| Blocker #3 arena pattern in production | ✅ `load_testing` is the first real user — per-worker persistent + scratch arenas, one `reset_via` per request |
| `server/ssrf` ported (M6 leaf, pulled forward) | ✅ two M4 modules gate on it — hardened against octal/hex/short-form bypasses |

## Tests

**2734 assertions across 43 `.tcyr` suites, all passing**, plus the 2-assertion scaffold
smoke — **2736 across 44 files**, which is the figure `cyrius tests tests` reports:

| Suite | Assertions | Oracle |
|---|---|---|
| `learning_capability.tcyr` | 18 | 8 `#[cfg(test)]` tests |
| `learning_profile.tcyr` | 23 | 7 |
| `learning_strategy.tcyr` | 19 | 6 |
| `learning_replay.tcyr` | 29 | 7 |
| `learning_optimizer.tcyr` | 21 | 7 |
| `core_error.tcyr` | 30 | 16 |
| `core_message.tcyr` | 39 | 6 |
| `id.tcyr` | 37 | — (reimplementation; the `uuid` crate is the reference) |
| `core_task.tcyr` | 82 | 13 |
| `core_resource.tcyr` | 83 | 19 of 28 (9 are hwaccel-gated and defer) |
| `core_agent.tcyr` | 88 | 12 |
| `core_crew.tcyr` | 66 | 8 |
| `llm_router.tcyr` | 48 | 14 of 17 (3 are hwaccel-gated and defer) |
| `llm_retry.tcyr` | 43 | 11 (4 of them `#[tokio::test]`) |
| `llm_hoosh.tcyr` | 124 | — (replaces a `pub use` facade; no oracle tests) |
| `tools_native.tcyr` | 67 | native.rs + registry.rs |
| `order.tcyr` | 48 | — (Rust used `sort_unstable` / `select_nth_unstable`) |
| `tools_builtin_basic.tcyr` | 37 | 6 (echo.rs + json_transform.rs) |
| `server_ssrf.tcyr` | 81 | ~14 |
| `tools_builtin_load_testing.tcyr` | 88 | 2 (both drive an axum mock server — see below) |
| `tools_builtin_security_audit.tcyr` | 200 | 8 (5 of them drive an axum mock server) |
| `tools_agnos.tcyr` | 140 | 27 across synapse + mneme + delta (schemas only — see below) |
| `tools_remote_registry.tcyr` | 77 | 3 (one asserts a constant; the other two are the SSRF arm) |
| `orch_output_validation.tcyr` | 69 | 11 |
| `orch_pubsub.tcyr` | 78 | 19 (13 wildcard + 6 `#[tokio::test]` integration) |
| `orch_multi_tenant.tcyr` | 39 | 12 |
| `orch_ipc.tcyr` | 47 | 4 (all `#[tokio::test]`, all portable via socketpair) |
| `orch_scoring.tcyr` | 68 | 10 |
| `orch_scheduler.tcyr` | 77 | 16 |
| `orch_budget.tcyr` | 47 | 6 |
| `orch_approval.tcyr` | 57 | 9 (3 of them `#[tokio::test]`) |
| `orch_plan_cache.tcyr` | 40 | 7 |
| `orch_memory.tcyr` | 53 | 9 |
| `orch_hierarchical.tcyr` | 30 | 6 |
| `server_sse.tcyr` | 56 | 10 |
| `orch_durable_state.tcyr` | 82 | 8 |
| `orch_crew_runner.tcyr` | 188 | 30 |
| `server_prompt_guard.tcyr` | 62 | 22 |
| `orch_audit.tcyr` | 54 | — (hoosh's audit.rs tests) |
| `orch_orchestrator.tcyr` | 49 | 5 |
| `server_output_filter.tcyr` | 63 | 16 |
| `server_prometheus.tcyr` | 35 | 8 |
| `server_auth.tcyr` | 52 | 5 of 10 (the shared-secret half; the 5 JWT tests wait on bite 4) |

The Cyrius suites deliberately exceed the oracle's coverage: they also pin the UCB1 formula
itself, the `max_by` last-wins tie rule, replay's zero-priority and NaN fallback branches, and
the Q-table's packed-key distinctness — none of which the Rust tests reach.

`tools_builtin_load_testing.tcyr` is the one suite that could not follow its oracle's shape.
Both Rust tests stand up an axum mock server on loopback, and `agnosai_is_safe_url` correctly
refuses loopback — so the tool cannot be aimed at one, and pointing a test suite at a public
host is not acceptable. Instead the **real OS-thread fan-out** runs against a synthetic
executor, which exercises the worker threads, the per-worker arenas, the deadline and budget
loops, status aggregation, the sort and the percentile indices with no network at all. The only
path left untested is sandhi's own behaviour under `sandhi_http_get_a`, and the live
`scripts/stack.sh check` covers that seam separately.

`tools_builtin_security_audit.tcyr` solves the same problem the same way, and it pays off
better: five of its eight oracle tests stand up an axum mock server, and because the module is
split at the network boundary all five port exactly — `_t_mock_headers` is a direct
transcription of the oracle's `mock_audit_server(security_headers, cors_wildcard)`, down to the
`Apache/2.4.99` it always sets. The suite then goes well past the oracle: both sides of every
risk-band boundary, the reflected-origin CORS bypass the probe origin exists to catch, the
`to_str()` visible-ASCII gate, case-insensitive scheme handling, and a snapshot-survives-reset
test that scribbles over the released arena so a borrowed pointer would show up as corruption
rather than passing silently.

`tools_agnos.tcyr` covers the nine ecosystem tools, and it is where the seam pays off most. The
oracle's three suites test **only** names, descriptions and schemas — every execute path needs a
live service on loopback, so the Rust side never exercises one. Because
`agnosai_agnos_client_new` takes its transport as a function pointer, a recording stub turns the
whole untested half into ordinary assertions: URL construction, form encoding, body
construction, the path-traversal guards, and the response reshaping.

`cyrius coverage --min 80` → **100% (752/752 fns), gate OK**. Shared assertion helpers live in
`tests/test_helpers.cyr` (all `_t_`-prefixed, so they can never shadow a `src/` symbol and stay
out of the coverage denominator).

Every `.tcyr` ends `var f = main(); if (f > 0) { f = 1; } syscall(60, f);` — the stock
`proj-tcyr` epilogue masks the exit code `& 0xFF`, so exactly 256/512/768 failures score PASS.

Rust oracle for comparison: **863 unit + 2 integration + 1 doctest, all passing.**

## Benchmarks

`benches/learning.bcyr` — the 10 shapes of `rust-old/benches/learning.rs`. First Cyrius numbers
(x86_64 Linux, cyrius 6.5.2):

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

`benches/core.bcyr` — no oracle bench file exists for core, so these measure the paths core
actually gets hammered on: id minting/rendering and the JSON round trips at every request
boundary.

| Benchmark | Time |
|---|---|
| `hw_inventory_satisfies` | 165 ns |
| `uuid_to_str` | 217 ns |
| `uuid_parse` | 252 ns |
| `uuid_v4_generate` | 526 ns |
| `message_to_json` | 8.67 µs |
| `task_from_json` | 10.1 µs |
| `agent_from_json` | 10.2 µs |
| `task_to_json` | 14.7 µs |
| `crew_to_json_10x10` | 305 µs |
| `crew_from_json_10x10` | 710 µs |

`uuid_v4_generate` is dominated by the `random_bytes` getrandom syscall, which is the right
trade: v4 ids must come from kernel entropy, never tyche. If id minting ever shows up hot, the
fix is batching entropy, not changing the source.

`benches/order.bcyr` — the numbers that settle **port plan blocker #8**. The plan measured an
O(n^2) insertion sort over agnosai's 100k-entry percentile vector at **52.6 s**, and predicted
87 ms for heapsort and ~21 ms for three quickselects. Measured here (each round includes a full
100k copy, so the algorithms alone are faster still):

| Benchmark | Time | vs plan |
|---|---|---|
| `select_nth_100k_already_sorted` | 4.12 ms | median-of-3 holds: *faster* than random input |
| `sort_10k_heapsort` | 6.20 ms | |
| `select_nth_100k` | 6.67 ms | |
| `three_percentiles_100k` | 10.6 ms | predicted ~21 ms — **2x better** |
| `sort_100k_already_sorted` | 77.7 ms | same as random: heapsort has no adversarial input |
| `sort_100k_heapsort` | 78.1 ms | predicted 87 ms |

Against the 52.6 s baseline that is ~670x for the full sort and ~5,000x for the three
percentiles that `load_testing` actually needs. The two already-sorted rows are the guards on
the algorithm choice, not padding: heapsort's worst case equals its average, and quickselect's
median-of-3 pivot is what keeps sorted input off the O(n^2) path.

`benches/tools.bcyr` — the tool path, principally the aggregation that runs after the
`load_testing` worker threads join. `order.bcyr` already covers the sort and the selects in
isolation; what is measured here is what load_testing adds on top.

| Benchmark | Time |
|---|---|
| `tool_registry_get` | 467 ns |
| `tool_execute_echo` | 973 ns |
| `is_safe_url_public_host` | 1.30 µs |
| `is_safe_url_octal_host` | 1.38 µs |
| `lt_result_to_value` | 2.80 µs |
| `lt_aggregate_100k_10workers` | 79.2 ms |
| `lt_aggregate_100k_500workers` | 81.5 ms |
| `lt_aggregate_100k_200codes` | 149 ms |

Two design choices were on trial and both held:

- **The cross-worker merge is O(total), not O(workers).** Spreading the same 100k samples over
  500 workers instead of 10 costs 2.3 ms more — within noise of the 74.6 ms sort that dominates
  both. Each worker owns an arena freed with it, so aggregate copies rather than aliases; that
  copy is ~5 ms at the cap.
- **Status counts as a linear pair vec, not a map.** There is no `map_u64_keys` in the stdlib,
  so codes live in a vec of `[code, count]` pairs. At 200 distinct codes the scan costs 1.9x —
  but real HTTP has ~60 defined codes and a real run sees one to five, so the vec is the right
  call and the 149 ms row is the documented ceiling rather than an expected cost.

The two `is_safe_url` rows matter because every load test pays one before it touches the
network: the octal-host path, which must parse four different spellings before it can decide,
costs only 6% more than the plain hostname path.

`benches/orch.bcyr` — the orchestration path: the event fan-out on every task boundary, the
per-task ranking and delegation, the scheduler's DAG sort, the plan-cache key, and the
conversation trim that runs on every push. Logging is turned down to `SK_FATAL`, since
`agnosai_delegate_tasks` emits an INFO line per call (matching the oracle's `info!`) and left at
the default level the number measures sakshi writing to a pipe.

| Benchmark | Time |
|---|---|
| `conv_buffer_push_sliding_32` | 120 ns |
| `pattern_match_wildcard` | 803 ns |
| `event_round_trip_1_sub` | 1.99 µs |
| `plan_cache_get_hit` | 2.11 µs |
| `event_send_evicting` | 2.40 µs |
| `pubsub_publish_4_patterns` | 5.72 µs |
| `plan_key_16x16` | 11.7 µs |
| `rank_agents_16` | 12.8 µs |
| `kahn_sort_64_nodes` | 57.6 µs |
| `event_fanout_64_subs` | 101 µs |
| `delegate_16_tasks_16_agents` | 204 µs |
| `durable_deserialize_crew_state` | 2.68 µs |
| `durable_load_miss` | 2.80 µs |
| `durable_serialize_crew_state` | 3.88 µs |
| `durable_load_hit` | 4.43 µs |
| `durable_mkdir_p_existing_4deep` | 6.04 µs |
| `durable_save_atomic` | 21.5 µs |
| `crew_select_model_routed` | 301 ns |
| `crew_infer_provider_fallthrough` | 594 ns |
| `crew_build_system_prompt` | 1.47 µs |
| `prompt_scan_clean_67b` | 8.04 µs |
| `prompt_scan_clean_4k` | 273 µs |
| `audit_record` | 27.9 µs |
| `audit_verify_256` | 2.65 ms |
| `output_scan_clean` | 6.98 µs |
| `output_redact_clean` | 9.69 µs |
| `metrics_record_task` | 5 ns |
| `metrics_gather` | 7.94 µs |
| `auth_check_disabled` | 6 ns |
| `auth_check_secret_short_token` | 917 ns |
| `auth_check_secret_ok` | 945 ns |
| `auth_check_secret_reject` | 955 ns |

The three `auth_check_secret_*` rows are the evidence for
[ADR 009](../adr/009-auth-constant-time-secret-compare.md), not padding. They sit within
**4%** of each other — accept 945 ns, reject 955 ns, and a 1-byte token against the same
9-byte secret 917 ns. The oracle's `max(a.len(), b.len())` loop would have made that last
row track the *secret's* length, which is the timing leak the ADR closes; the residual
~28 ns spread tracks the **token's** length, which the attacker chose and already knows.
`auth_check_disabled` at 6 ns is the first branch, and it is what every deployment that
has not configured auth pays per request.

**These numbers are futex-bound, not algorithm-bound**, and that is the finding rather than an
excuse. `mutex_lock` + `mutex_unlock` costs **394 ns uncontended** because `lib/sync.cyr`'s
two-state mutex calls `FUTEX_WAKE` on every release whether or not a waiter is parked; a scratch
build with that one line deleted measures **46 ns**, an 8.6× gap. `chan_try_send` +
`chan_try_recv` is 1.59 µs, about four mutex pairs. So `event_round_trip_1_sub` at 1.99 µs is
three locks and almost nothing else, and `event_fanout_64_subs` is 128 channel operations.
Filed upstream in the cyrius repo as
`docs/development/issues/2026-07-29-mutex-unlock-unconditional-futex-wake.md`, with a repro.
Nothing is worked around here: the rows above are the honest current baseline, and a stdlib fix
will show up as a straight improvement rather than needing any change on this side.

Two rows are worth reading together. `event_send_evicting` (2.40 µs) is a send into a subscriber
that has stopped reading — the overwrite-oldest path blocker #4 exists for. It costs one extra
channel operation over the healthy `event_round_trip_1_sub` (1.99 µs), which is the number that
matters: **a stalled SSE client does not get more expensive to serve**, it just loses events. A
blocking `chan_send` there would have wedged the crew instead.

`delegate_16_tasks_16_agents` at 204 µs is 16 × `rank_agents_16` (12.8 µs) plus change, which is
the expected shape — hierarchical mode ranks every task independently, with no memoisation
across tasks. That is the oracle's behaviour and the reason the cost is linear in tasks × agents.

**Not comparable to `rust-old/bench-history.csv`** — different allocator, different harness, no
criterion statistics. The Cyrius line starts its own baseline, captured by
`scripts/bench-history.sh` into the root `bench-history.csv` (65 rows per capture).

## Dependencies

**stdlib** (43 declared, order-sensitive — rationale in [`cyrius-port-plan.md`](cyrius-port-plan.md)):
base substrate · general utilities · bayan · patra · concurrency+crypto floor ·
dynamic-link floor · async · net/http/tls/ws/sakshi/sandhi

**git deps** (declare-ahead pattern): sigil 3.12.1 · bote 3.1.4 (core profile) ·
majra 2.5.3 · kavach 3.9.3 · ai-hwaccel 2.3.16 · tyche 1.0.0 — each matching its
repo's newest tag as of 2026-07-30.
The hoosh seam targets **hoosh 2.6.0** — `usage.cost_micro_usd`, `usage.provider` and
`X-Hoosh-Cache` are read when present, and an older gateway degrades to an absent cost
rather than a fabricated one.
libro 2.8.4 arrives transitively via bote.

## Known issues in the current build

1. **36 duplicate-fn warnings at build — all benign today, but know the rule.**
   Cyrius is single-pass: a redefinition only rebinds call sites parsed *after*
   it. A dep's internal calls therefore keep binding to its own definition even
   when a later bundle redefines the name. Verified by probe — majra's
   `pubsub_subscribe` works correctly with libro in the same build, despite both
   defining `_sub_new` with different arity.

   **What this does mean for us:** agnosai's own modules are included last, so
   *our* calls bind to the last definition of everything. When we call a name
   that appears in more than one bundle, check which one wins. The current
   duplicates:

   - **33 from kavach** re-exporting symbols sigil also defines, with
     byte-identical bodies (no behavioural difference either way).
   - **`_sub_new`** (majra/libro) — we should never call it, it is a dep-private
     helper.
   - **`path_exists`** — `lib/kavach.cyr:2640` uses `sys_access(path, 0)` (F_OK,
     existence only); `lib/ai-hwaccel.cyr:1385` delegates to `file_exists`, which
     opens `O_RDONLY`. **ai-hwaccel's wins for our code** — it is included after
     kavach — and the two are *not* the same contract: a path that exists but is
     unreadable answers 1 under kavach's and 0 under ai-hwaccel's. Corrected
     2026-07-30; the earlier entry had this backwards on both counts.
   - **`_agnosai_is_digit`** — **ours, not a dep's**. Defined at
     `src/server_ssrf.cyr:39` and again at `src/server_output_filter.cyr:140`.
     The bodies are semantically identical today (only the parameter name
     differs, `c` vs `b`), so nothing misbehaves — but two copies of a scanner
     that agree by accident is exactly the condition
     [ADR 007](../adr/007-audit-redirect-revalidation.md) was written about.
     Hoist it into a shared helper at the next touch of either module.

## Consumers

Agnostic (Python platform), daimon (agent orchestration), joshua (NPC AI),
kiran (game AI) — none consuming the Cyrius line yet.

## Next

> **Session handoff — 2026-07-30**, re-audited against the live tree the same
> day. The gate figures (tests, coverage, fmt/vet/deny) all reproduced exactly;
> what had drifted was every hand-transcribed table — six assertion rows, the
> duplicate-fn count and its `path_exists` verdict, the locked-dep count, libro's
> version, and the ai-hwaccel pin. All are corrected above. **Lesson for the next
> refresh: regenerate the tables from command output rather than editing rows by
> hand** — `cyrius tests tests` prints a per-file `N passed` line, and the six
> stale rows were all transcription drift, not real change.

**M5 — `orchestrator` is COMPLETE.** All 15 modules, plus two pulled forward
from M6 (`server/sse`, `server/prompt_guard`) and an `orch_audit` chain the
hoosh seam cannot delegate.

**M6 — `server` is 5 of 21 files** (1,860 of 4,977 Rust lines). The plan's
pure-leaf sequence is finished: `ssrf` and `prompt_guard` and `sse` came forward
as M5 blockers, then `output_filter` and `prometheus` landed as M6 bites 1 and 2.
16 files remain, carrying 69 oracle tests of which **43 are `#[tokio::test]`**.

**The structural call that shapes the rest of M6:** most of those tokio tests are
async only because axum's `app.oneshot(...)` is, not because the handler logic is.
Writing each handler as a pure `fn(state, inputs) -> (status, json)` *before* any
sandhi adapter exists converts the majority of them into ordinary sync `.tcyr`
assertions, so the transport bite arrives with its logic already proven. The
ordering rule that follows: **nothing that needs sandhi's server transport until
every pure-leaf test is green.** Note that `src/` has zero `sandhi_server_*` call
sites today — agnosai uses sandhi purely as an HTTP *client* — so the router bite
is the first server-side use in the whole port, and it is where blocker #3's
per-worker arena and the `alloc_used()`-flat regression test must land together.

### Pick up here

> **Bite 3 (auth, shared-secret half) is DONE** — `src/server_auth.cyr`,
> 52 assertions, 15/15 fns covered, benchmarked. **Next is bite 4: the RS256
> JWT half.** Read the four decisions below *before* writing it; they are the
> whole risk in that bite. The five items under "~15 lines of glue undersells
> the bite" still apply — the `alg` check and the base64url allocator posture
> in particular, neither of which bite 3 touched.
>
> The background below is kept because it is what makes bite 4 straightforward.

**`server/auth.rs` (452 lines) is NOT blocked** — see the
correction in [`cyrius-port-plan.md`](cyrius-port-plan.md) Phase 5, which
replaces a stale gate that cost a session's worth of investigation to disprove.
Both named dependencies dissolve:

* sigil's `rsa_pubkey_from_der` already accepts SPKI, and its PEM helpers are
  label-generic — `pem_decode_pubkey` is ~15 lines of local glue.
* bote's only JWT is HS256, and its `src/jwt.cyr` ships in no bundle. Wrong
  dependency; dropped from the ask list.

Proven by execution, not inspection: a real 2048-bit SPKI PEM → 294-byte DER →
256-byte modulus + 3-byte exponent → `rsa_pkcs1v15_verify_sha256` returning **1**
for an openssl-signed RS256 token and **0** for both a tampered input and a
tampered signature. Every primitive is already resolved into `lib/`.

Suggested split, smallest verifiable bite first:

1. ~~**Shared-secret half**~~ — ✅ **DONE (bite 3).** `AuthConfig`, `JwtConfig` and
   its builders, case-sensitive `Bearer ` extraction, the `to_str()`
   visible-ASCII gate, constant-time compare, the disabled short-circuit. All 5
   shared-secret oracle tests plus 47 assertions beyond them. The compare is
   SHA-256-digest-based rather than the oracle's loop —
   [ADR 009](../adr/009-auth-constant-time-secret-compare.md) — because the
   oracle's loop bound leaks the secret's length while its own comment claims it
   does not. Same accept/reject set, no timing leak, 945 ns.
2. **PEM → `(n, e)`** — unlocks `jwt_garbage_token_rejected`, which asserts
   **401 not 500**, so it only passes if the PEM genuinely parses.
3. **Commit four RS256 token vectors** generated once from the frozen oracle
   (`rust-old/tests/fixtures/` has the keypair), making the port verify-only —
   no RSA signing anywhere in agnosai.
4. **JWT split + field-based `alg` pin + verify.** Read `alg` as a parsed JSON
   field, never a substring: bote's HS256 does a substring scan, so
   `{"alg":"none","kid":"HS256-2024"}` satisfies it. That shape must not be
   copied into a module that exposes two auth paths on one endpoint.
5. **Claims** — `exp`/`iss`/`aud` with jsonwebtoken's exact semantics. Biggest
   and riskiest; goes last, on green foundations.

**"~15 lines of glue" undersells the bite.** Re-verified 2026-07-30 by compiling
and running a throwaway `.tcyr` inside agnosai against its own `lib/` (27/27
green). Nothing is blocked, but five items belong in the work and none is an
upstream dependency:

* **Write the `alg == "RS256"` check by hand.** sigil exposes the raw primitive
  only; there is no JWT layer anywhere in `lib/` to inherit
  `Validation::new(Algorithm::RS256)` from. Without it the port ships an
  `alg: none` bypass the oracle does **not** have — and the oracle's own tests
  never exercise `alg`, so test parity will not catch its absence. Needs an
  explicit divergence test.
* **Decide the allocator posture for base64url first.** `bayan_base64url_decode`
  (`lib/bayan.cyr:131`) has no `_a` variant and allocates three times per call on
  the global bump — a decode table, the output buffer, and a `{ptr,len}` pair. On
  a per-request auth path that leaks forever, against the arena discipline the
  rest of the server follows. Either write a decode-into-caller-buffer local, or
  accept it consciously and file it in bayan.
* **Parse the public key once at startup**, not per request. The oracle re-parses
  the PEM inside `validate_jwt` on every authenticated request
  (`auth.rs:120-123`). Hoisting it is both the perf fix and the fix for sigil's
  non-atomic `_pem_init` once-guard — but it moves the failure mode from a
  per-request 500 to a boot-time abort, which the oracle suite cannot tell apart,
  so it must be deliberate.
* **Use `bayan_json_v_parse` + `bayan_json_v_obj_get`**, never the flat
  `bayan_json_parse` — the flat one desyncs on an escaped quote, and the two take
  different key types (Str vs cstr; see the API hazards at the end of this file).
* **`constant_time_eq` is not constant time in the oracle.** `auth.rs:24` bounds
  its loop with `a.len().max(b.len())`, leaking the secret's length, while its own
  comment at `auth.rs:16-17` claims it does not. This is a defect to fix, not a
  parity question — fix the loop and the comment together.

### Four decisions waiting on the maintainer

> Items 1-3 were re-verified 2026-07-30 against the **pinned jsonwebtoken 10.3.0
> source**, not against the summary. Item 3's stated mechanism was wrong and is
> corrected below. Item 4 is new and was missed entirely by the earlier pass.

1. **`iss`/`aud` absent passes.** jsonwebtoken's `set_issuer`/`set_audience` do
   not add those claims to `required_spec_claims`, so a token carrying **no
   `iss` at all** passes issuer validation even when an issuer is configured.
   **Recommendation: tighten, with an ADR.** It is the only one of the four that
   is a live security weakness rather than a quirk, and the cost is lopsided —
   tightening changes **zero** oracle assertions (`auth.rs:332-341` always
   populates both claims), while reproducing means an operator who sets
   `AGNOSAI_JWT_ISSUER` gets no issuer enforcement at all against any token that
   omits `iss`. With one static key and no `kid` routing (`auth.rs:68-75`),
   `iss`/`aud` are the only cross-tenant separation that exists. Implement it
   conditionally, mirroring the oracle's `if let Some` shape, so
   `JwtConfig::new(key)` with neither configured keeps passing.
2. **`exp: u64::MAX`** in the oracle's fixtures (`auth.rs:337`) does not fit i64 —
   it is `2 × i64::MAX + 1`. Rust copes because jsonwebtoken holds `exp` as
   `TryParse<u64>` and compares in u64. **Recommendation: `253402300799`**
   (9999-12-31T23:59:59Z), the conventional JWT never-expires sentinel, ~27× below
   i64::MAX. `4102444800` (2100-01-01) is equally safe. Decide before generating
   the four vectors in step 3 above. **Corollary:** a frozen vector cannot test
   the leeway boundary at all, so covering it needs an injectable `now` in the
   Cyrius validator — a design constraint on the claims bite that is much cheaper
   to accept now than to retrofit.
3. **`Claims.aud` is `Option<String>`** while the validator accepts
   `Audience::Multiple`. **The mechanism recorded earlier was backwards.**
   Deserialization runs *first* (`decoding.rs:285-287`), so any array-valued `aud`
   fails as `ErrorKind::Json` with validation never running — including the
   single-element form `["agnosai"]`. **Recommendation: inherit, and file it as a
   separate follow-up.** It is fail-closed, so inheriting adds no risk, and
   "reproduce" is simpler than it looked: reject any non-string `aud`. Fixing it
   properly would force an intersection-vs-membership decision at the same time,
   because jsonwebtoken's `is_subset` (`validation.rs:249-256`) accepts a
   single-element overlap despite its name — so naively widening `Claims.aud`
   turns a harmless compatibility bug into real audience confusion. Record in the
   module header that arrays are rejected and that this is **inherited, not
   designed**, or it will be re-discovered as a bug the first time an Auth0 or
   Cognito token hits the endpoint.
4. **NEW — jsonwebtoken's 60-second default leeway.** `Validation::new` seeds
   `leeway: 60` (`validation.rs:120`) and the oracle never overrides it. This is
   **invisible from `auth.rs` alone**, so a Cyrius port written faithfully from
   the oracle source will compare `exp < now` and ship a silently *stricter*
   server — precisely the failure mode CLAUDE.md calls the worst outcome.
   **Recommendation: reproduce the 60 s** (standard clock-skew practice, matches
   the oracle) as a **named constant with a comment citing `validation.rs:120`**,
   so the number is stated rather than implied.

Lower-priority calls worth making in the same pass, not separately: `nbf` is
silently ignored; `Bearer ` is case-sensitive; `AuthConfig::default()` fails
**open**; `/metrics` is unauthenticated (`mod.rs:88`); and validated claims are
discarded at `auth.rs:187` rather than passed downstream. Also note the
`exp.is_none()` guard at `auth.rs:143-146` is unreachable dead code — keep it, but
do not mistake it for the mechanism enforcing `exp`, which is the
`required_spec_claims = {exp}` default the port must reimplement **explicitly**.

### Cheap insurance before writing the module — ✅ DISCHARGED 2026-07-30

bote documents a `thread_local`-before-`sigil` ordering constraint where binaries
link clean and then SIGILL at first crypto use. This was cleared by execution: a
throwaway `.tcyr` compiled inside agnosai against its own `lib/` ran **27/27
green**, covering SPKI PEM → DER → `(n, e)` → `rsa_pkcs1v15_verify_sha256` (1 for
valid, 0 for tampered), plus `bayan_base64url_decode`, JSON claim extraction,
`clock_epoch_secs`, `ct_eq_bytes_lens` and `sandhi_server_find_header`. The RSA
path touches more of sigil's scratch than `orch_audit`'s `hmac_sha256` does, and
it is fine. No further insurance needed — go straight to the shared-secret half.

### Filed upstream this session, none blocking

Status re-verified against each repo's issue directory on 2026-07-30. cyrius files
under `docs/development/issues/` are open; those under `issues/archive/` are resolved.

| Repo | Issue | Status |
|---|---|---|
| cyrius | `mutex_unlock` unconditional `FUTEX_WAKE` — 394 ns, measured 8.6× | open (`2026-07-29-mutex-unlock-unconditional-futex-wake.md`); still verbatim in `lib/sync.cyr:72-75` on 6.5.3 |
| cyrius | `fmt_int_buf` renders `i64::MIN` as bare `-`, corrupting bayan JSON | open (`2026-07-29-fmt-int-buf-i64-min.md`), candidate fix supplied and run (11/11); still present in `lib/fmt.cyr:99` on 6.5.3 |
| cyrius | no O(n log n) sort in `lib/vec.cyr` — 18+ consumers each rolled their own O(n²) | open (`2026-07-28-agnosai-no-nlogn-sort-in-stdlib.md`) — **filed 2026-07-28**; the port plan's "still to file" line was stale |
| cyrius | `sock_send` `Result` allocates per call — 16 B/response survives the arena | open (`2026-07-28-sock-send-result-allocates-per-call.md`); pinned by an exact-bound test in sandhi |
| cyrius | no portable `xmkdir` in `io.cyr` | open (`2026-07-29-no-portable-xmkdir-in-io-cyr.md`) |
| cyrius | `chan_try_send` absent on all three backends (blocker #4) | ✅ **resolved in 6.4.84**, archived upstream |
| cyrius | `_int` overload misdispatch on a bare call-result argument | ✅ **resolved in 6.5.2**, archived upstream |
| hoosh | chat response hid cache-hit and cost | ✅ resolved in **2.6.0**, consumed here |
| ai-hwaccel | `json_v_parse_str` removed in bayan 1.3.0 | ✅ resolved in **2.3.16** — pin corrected here 2026-07-30 (the manifest still said 2.3.15) |
| ai-hwaccel | `load_models` returns 1 model instead of 26 | open — two candidate fixes, a compatibility call |
| bayan | YAML parse into the tagged value tree | open (`2026-07-16-agnosai-yaml-parse-into-tagged-value-tree.md`); gates M10's YAML half, nothing sooner |
| bote | `src/jwt.cyr` orphaned + documents an `exp` check it does not perform | **written but not yet upstream** — the file is untracked in the bote worktree and bote's HEAD is still 2026-07-17. The `exp` half is a false security claim, so this one is worth pushing |
| sigil | *(none filed)* | `pem_decode_pubkey` would be nice-to-have; nothing waits on it |

**Never filed, still just port-plan asks:** sankoch ZIP container, majra relay
reentrancy (confirmed live at `dist/majra.cyr:2105-2107`), sandhi `backlog` ignored
by `run_opts`/`run_async` (`dist/sandhi.cyr:13310`, `:13469`), sigil
`pem_decode_pubkey`, kavach exec timeout. The kavach one needs re-scoping before
filing: `SandboxConfig_set_timeout_ms`, `config_timeout_ms` and `KAVACH_ERR_TIMEOUT`
are all present in the shipped 3.9.3 bundle, so "the Cyrius port dropped exec
timeout" as written would be filed against a surface that partly exists.

### Repo state

As of 2026-07-30, agnosai is clean at `76c4ded` plus this session's pin bump and
doc corrections. **cyrius 6.5.3 shipped** — tagged, clean, release-gated — and
agnosai now pins it; the bump is free because `lib/` is byte-identical to 6.5.2.
hoosh 2.6.0, ai-hwaccel 2.3.16 and sigil 3.12.1 are clean and tagged. bote has
two untracked issue files and an unstaged roadmap edit that have not reached
upstream (see the table above).

**`load_testing` is the first production user of the port plan's blocker #3
arena pattern.** One OS thread per simulated user (a load generator that ran
sequentially would not be one), each owning two arenas: a persistent one sized
from the request budget that holds its latencies, and a scratch one
`reset_via`'d after every request. The scratch arena is not a refinement — a
single arena would accumulate every response body for the whole run, which is
unbounded growth the oracle does not have, since Rust drops each response as it
goes.

Two divergences from convention inside the module, both deliberate:
`_agnosai_lt_pct_index` uses the oracle's `(len * p / 100).min(len - 1)` rather
than `order.cyr`'s nearest-rank index, because for n=100 they differ (index 50
vs 49) and the reported figure has to be the oracle's; and throughput and error
rate are carried as integers (thousandths of a request/second, parts per
million) converted to float only at the wire boundary, the same treatment money
gets.

**`remote_registry.rs` is unblocked** by `src/server_ssrf.cyr`, though its
payload path (`.agpkg` ZIP + raw WASM) defers with those formats, so it can only
deliver a guarded fetch. python_tool / wasm_tool / wasm_loader defer with their
features.

**`src/server_ssrf.cyr` was pulled forward from M6** because two M4 modules gate
on `is_safe_url`; stubbing that guard twice would have been worse than porting
it once. It is hardened past a literal reading of the oracle: the Rust side gets
octal / hex / short-form / decimal-integer host normalisation free from the
`url` crate's WHATWG parser (so `http://0177.0.0.1/` is caught), and a naive
dotted-quad port would have classified every one of those as a hostname and let
them through. See the module header.

**Live testing.** `scripts/stack.sh` brings up the services agnosai needs
(today: hoosh only) and `scripts/stack.sh check` drives
`tests/smcyr/llm_live.smcyr` — the one harness that exercises
`agnosai_hoosh_chat` against a real gateway. It SKIPs (exit 0) when no gateway
is up, so `cyrius smoke` stays green on a machine with no stack, but fails
loudly if the gateway answers and the exchange is wrong.

Open items, none blocking:

1. ~~**Awaiting the bayan 1.3.0 fold-in.**~~ ✅ **DONE 2026-07-29 with the 6.5.2 fold-in.**
   Cyrius routes a call `X(a, …)` to `X_str` whenever `a` is Str-typed at the call
   site and `X_str` exists (the same dispatch that routes `&IDENT` to `_ptr`).
   Because bayan's cstr+len forms were named `bayan_json_v_parse_str` /
   `bayan_yaml_parse_str`, every bare `bayan_json_v_parse(someStr)` was rewritten
   into a 1-arg call to a 2-arg fn and returned 0 for valid JSON — silently,
   across ~26 files ecosystem-wide. bayan 1.3.0 renames those forms `_str` →
   `_buf`, and **23 call sites here moved from `bayan_json_v_parse_str` to
   `bayan_json_v_parse_buf`** across nine `src/` modules and seven suites. The
   bodies are byte-identical, so this was a pure rename with no semantic change,
   and a compile error rather than a silent break, so none could be missed.
2. ~~**Blocker #3 awaiting a toolchain fold-in.**~~ ✅ **CLOSED 2026-07-29.**
   Repaired in sandhi 1.9.7 and now shipped in the 6.5.2 bundle, so it has
   reached the port: `lib/sandhi.cyr` here carries
   `sandhi_server_options_req_arena` and `_get_req_arena` (a per-worker,
   per-request arena, default off), `sandhi_server_request_arena`, and the
   allocator-threaded `sandhi_router_dispatch_a` / `sandhi_server_router_handler_a`
   — all five verified present after the sync. With the option set, the routing
   path (the method/path accessors and the 404/405 writes) allocates in a
   rewindable arena instead of the no-free global bump. **M6 can now be built
   against it rather than around it.**

   **Known residual, already filed and not sandhi's to fix:** with the arena on,
   a response still grows the global bump by exactly 16 bytes — the `Result`
   that `lib/net.cyr`'s `sock_send` returns. Filed as cyrius
   `2026-07-28-sock-send-result-allocates-per-call.md`, re-confirmed present on
   6.5.0, and pinned by `sandhi/tests/sandhi.tcyr::test_server_req_arena` as an
   exact bound so the test will speak up if the stdlib fix lands.
3. **`lib/sakshi.cyr` lands at 2.4.3 even after `cyrius lib sync --full`**, which
   every build reports as a shadow warning against the 6.5.2 bundle's 2.4.7.
   This is **not agnosai's to fix and not a correctness problem.** Each git dep
   vendors its own sakshi distribution and `cyrius deps` copies them into `lib/`
   with last-write-wins: bote / majra / ai-hwaccel carry 2.4.6, sigil and kavach
   carry 2.4.3, tyche carries 2.2.10. A 2.4.3 copy wins. The 2.4.3 → 2.4.7 diff
   is three added public verbs (`sakshi_trace_id_hi`, `_lo`, `_trace_set_128`)
   plus `_`-prefixed internal churn, and the older bundle is a superset of the
   symbols anything here calls, so nothing breaks. It clears when sigil, kavach
   and tyche re-cut their bundles.
4. **Cyrius `_int` overload misdispatch — filed 2026-07-29.**
   `X(f(), …)` silently runs `X_int`'s body when `X_int` exists and the first
   argument is written as a bare call result; the same value via a variable
   dispatches correctly, with no diagnostic either way. Cost about an hour to
   bisect in `tools_agnos`, where a helper named `_t_add` ran `_t_add_int` and
   stored every string parameter as a JSON integer holding its own pointer.
   Filed at `cyrius/docs/development/issues/2026-07-29-agnosai-int-overload-call-result-misdispatch.md`
   with a standalone repro under `issues/repros/`.
   **Nothing here is blocked** — the helper was renamed, and the one such pair
   in our own source (`agnosai_tool_input_get` / `_get_int`) is dormant because
   every call site passes `input` as a variable, which `tests/tools_agnos.tcyr`
   now pins.

Two API hazards worth knowing when writing the rest of core:

- **`X_str` is a reserved overload slot.** Never name a fn `X_str` unless it
  genuinely takes a `Str` first — dispatch will hijack every `X(<Str>)` call.
  `_ptr` / `_buf` / `_ctx` siblings are unaffected.
- **bayan's JSON key types are asymmetric.** `bayan_json_v_obj_set` takes a
  `Str`, `bayan_json_v_obj_get` takes a C string. Getting it backwards is a
  segfault, not a clean error.
