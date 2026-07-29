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
- **Cyrius port**: `learning` (5 modules + hub), `core` (6 modules + hub + shared helpers),
  `llm` (router, retry, hoosh seam client + hub) and `tools` (native, registry, echo,
  json_transform, load_testing, security_audit, and the nine AGNOS ecosystem tools over a
  shared client), plus `src/id.cyr`, `src/units.cyr`, `src/order.cyr` and
  `src/server_ssrf.cyr`. `src/main.cyr` is still the stub entry point — no CLI surface yet.

## Where the port is

Phase 0 (M1 scaffold) — **complete**. Phase 1 (M2 beachhead) — **complete**: `learning` and
`core` both ported and green against the oracle. Phase 2 (M3 `llm`) — **complete**: router, retry
and the hoosh seam client, with the live round trip verified. Phase 3 (M4 `tools`) —
**every builtin done — only `remote_registry` is left**.

| Gate | Status |
|---|---|
| Rust baseline green (the oracle) | ✅ fmt + clippy clean, all 9 feature combos compile, 863 + 2 + 1 tests pass |
| Terminal Rust benchmark capture | ✅ 112 rows at v1.1.0, frozen into `rust-old/bench-history.csv` |
| Blockers re-verified vs 6.4.86 | ✅ see [`cyrius-port-plan.md`](cyrius-port-plan.md) |
| `cyrius port` run | ✅ 2026-07-28 |
| `cyrius lib sync` + `cyrius deps` | ✅ 43/43 stdlib modules, 9 deps resolved, 0 errors, 79 locked |
| Hello-world builds and runs | ✅ `cyrius build src/main.cyr build/agnosai` → OK |
| `src/id.cyr` (uuid v4/v5) | ✅ v4 + v5, verified against the published RFC 4122 vector |
| `src/order.cyr` (sort / select_nth) | ✅ heapsort + quickselect — blocker #8 closed, benchmarked |
| Tool-sandbox approach decided | ✅ cx + kavach — [ADR-006](../adr/006-cx-tool-sandbox.md) |
| `learning` ported (M2, Phase 1) | ✅ 5 modules + hub, 112 assertions, 100% reference coverage |
| `core` ported (M2, Phase 1) | ✅ 6 of 6 + shared `core_json` helpers |
| Money representation decided | ✅ integer micro-USD (2026-07-28) — gates core BITE 8 |
| `llm` ported (M3, Phase 2) | ✅ router + retry + hoosh seam client |
| M3 exit: live chat-completion round trip | ✅ verified through `agnosai_hoosh_chat` (`scripts/stack.sh check`) |
| `tools` ported (M4, Phase 3) | 🟡 all 12 builtins done (echo, json_transform, load_testing, security_audit, 3x synapse, 3x mneme, 3x delta); remote_registry left |
| SSRF-via-redirect closed ([ADR 007](../adr/007-audit-redirect-revalidation.md)) | ✅ the guard re-runs on every hop — the oracle checks only the URL the caller supplied |
| Blocker #3 arena pattern in production | ✅ `load_testing` is the first real user — per-worker persistent + scratch arenas, one `reset_via` per request |
| `server/ssrf` ported (M6 leaf, pulled forward) | ✅ two M4 modules gate on it — hardened against octal/hex/short-form bypasses |

## Tests

**1365 assertions across 23 `.tcyr` suites, all passing** (plus the 2-assertion scaffold smoke):

| Suite | Assertions | Oracle |
|---|---|---|
| `learning_capability.tcyr` | 18 | 8 `#[cfg(test)]` tests |
| `learning_profile.tcyr` | 23 | 7 |
| `learning_strategy.tcyr` | 19 | 6 |
| `learning_replay.tcyr` | 29 | 7 |
| `learning_optimizer.tcyr` | 21 | 7 |
| `core_error.tcyr` | 30 | 16 |
| `core_message.tcyr` | 39 | 6 |
| `id.tcyr` | 26 | — (reimplementation; the `uuid` crate is the reference) |
| `core_task.tcyr` | 82 | 13 |
| `core_resource.tcyr` | 83 | 19 of 28 (9 are hwaccel-gated and defer) |
| `core_agent.tcyr` | 88 | 12 |
| `core_crew.tcyr` | 66 | 8 |
| `llm_router.tcyr` | 48 | 14 of 17 (3 are hwaccel-gated and defer) |
| `llm_retry.tcyr` | 43 | 11 (4 of them `#[tokio::test]`) |
| `llm_hoosh.tcyr` | 96 | — (replaces a `pub use` facade; no oracle tests) |
| `tools_native.tcyr` | 67 | native.rs + registry.rs |
| `order.tcyr` | 48 | — (Rust used `sort_unstable` / `select_nth_unstable`) |
| `tools_builtin_basic.tcyr` | 37 | 6 (echo.rs + json_transform.rs) |
| `server_ssrf.tcyr` | 81 | ~14 |
| `tools_builtin_load_testing.tcyr` | 88 | 2 (both drive an axum mock server — see below) |
| `tools_builtin_security_audit.tcyr` | 200 | 8 (5 of them drive an axum mock server) |
| `tools_agnos.tcyr` | 140 | 27 across synapse + mneme + delta (schemas only — see below) |

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

`cyrius coverage --min 80` → **100% (498/498 fns), gate OK**. Shared assertion helpers live in
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

**Not comparable to `rust-old/bench-history.csv`** — different allocator, different harness, no
criterion statistics. The Cyrius line starts its own baseline, captured by
`scripts/bench-history.sh` into the root `bench-history.csv` (35 rows).

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

**Finish M4 — `tools`** ([`roadmap.md`](roadmap.md), Phase 3). native, the
registry (with its mandatory mutex), the two utility builtins and load_testing
are done. Remaining: `builtin/security_audit.rs` (519), `builtin/synapse.rs`
(386), `builtin/mneme.rs` (442), `builtin/delta.rs` (471), and
`remote_registry.rs` (119).

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

1. **Awaiting the bayan 1.3.0 fold-in.** Root-caused and fixed upstream
   2026-07-28: Cyrius routes a call `X(a, …)` to `X_str` whenever `a` is
   Str-typed at the call site and `X_str` exists (the same overload dispatch
   that routes `&IDENT` to `_ptr`). Because bayan's cstr+len forms were named
   `bayan_json_v_parse_str` / `bayan_yaml_parse_str`, every bare
   `bayan_json_v_parse(someStr)` was rewritten into a 1-arg call to a 2-arg fn
   and returned 0 for valid JSON — silently, across ~26 files ecosystem-wide.
   bayan 1.3.0 renames those forms `_str` → `_buf`.
   **Action here when the fold lands:** four `*_from_json` entries currently
   call `bayan_json_v_parse_str` — in `core_message.cyr`, `core_task.cyr`,
   `core_agent.cyr` and `core_crew.cyr`. That name will no longer exist; switch
   each to the bare `bayan_json_v_parse(src)`. It is a compile error, not a
   silent break, so none can be missed.
2. **`lib/sakshi.cyr` is vendored at 2.4.3 while the 6.4.86 toolchain bundles
   2.4.6**, which every build reports as a shadow warning. `cyrius lib sync
   --full` re-syncs.
3. **Cyrius `_int` overload misdispatch — filed 2026-07-29.**
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
