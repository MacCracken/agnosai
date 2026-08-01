# AgnosAI — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures
> (durable); this file is **state** (volatile).
> Last refreshed: 2026-07-30.

## Version

**1.1.0** (`VERSION`) — the last shipped Rust release, now preserved at
`rust-old/`. The Cyrius line targets **v2.0.0**; VERSION bumps once parity
lands, not before, so the number always names something that actually shipped.

## Toolchain

- **Cyrius pin**: `6.5.5` (`cyrius.cyml`) — bumped 2026-07-31. Folds **bayan 1.4.0**,
  sandhi 1.9.8, sigil 3.12.2, sakshi 2.4.7, yukti 2.3.2, mabda 4.0.8.
- **`lib/` matches the pin exactly**: 0 of 99 stdlib files differ from
  `~/.cyrius/versions/6.5.5/lib`; build and test emit no drift or shadow warning.
  The six files outside the snapshot (ai-hwaccel, bote-core, kavach, libro, majra,
  tyche) are declared git deps, not staleness.

  **Verify a bump by diffing the trees, not by a green `lib sync --full`.** On the
  bayan repo the same command reported success and left five files stale; here it
  was clean, but the check is cheap and the failure is silent.

### What 6.5.5 lands on agnosai

- **bayan 1.4.0 completes the `_a` JSON surface** — `obj_set_a`, `build_a`,
  `build_pretty_a`, `parse_a`, `parse_buf_a`, `parse_ctx_a`, joining the eight value
  constructors that already had `_a` forms. This is the one that mattered: before it,
  a per-request arena could hold part of a response tree and the pair cells, the parse
  and the serialized text still landed on the no-free global bump. **Blocker #3's
  handler half is now closed** and needs no further upstream work. First consumer:
  `src/core/task.cyr`, measured **1792 → 0 bytes/response**. See roadmap B3.

### Carried forward from 6.5.4

- **`vec_sort_by` / `vec_select_nth`** closed agnosai's own sort filing.
  ✅ **Consumed** — `src/order.cyr` is now a 98-line wrapper over them (was 184).
- **sandhi 1.9.7 → 1.9.8** changed a **return contract** the transport tier depends on:
  all five serve loops previously spun a core forever on a persistent accept error and
  never returned once listening; they now return 1 on a structurally dead listener or
  after 200 consecutive resource failures. `src/server/serve.cyr` is the caller.
- **sigil 3.12.1 → 3.12.2** fixed a 144-byte-per-call `sha256_init` leak. **It never
  affected us** — `_agnosai_auth_secret_eq` uses the banked `sha256()` one-shot, which
  is allocator-free — confirmed by measuring the shared-secret path at **32
  bytes/request** before and after.

  **The sigil pin has to move with the fold.** `cyrius deps` copies each git dep's
  vendored bundle into `lib/` with last-write-wins, so leaving `[deps.sigil] tag =
  "3.12.1"` would overwrite the fold's 3.12.2 back down to 3.12.1.
- Rust (for `rust-old/` only): `channel = "stable"`, currently rustc 1.96.0

## Source

`src/` mirrors `rust-old/src/` — see CLAUDE.md's *Layout* rule. Generated from the
tree 2026-07-31:

| Group | Files | Lines | Oracle |
|---|---|---|---|
| `orchestrator/` | 16 | 5,311 | ✅ complete (M5) |
| `server/` | 11 | 3,782 | 🟡 M6, 20 of 21 files |
| `core/` | 8 | 3,129 | ✅ complete (M2) |
| `tools/builtin/` | 6 | 2,017 | ✅ complete (M4) |
| `server/routes/` | 9 | 1,786 | ✅ complete |
| `llm/` | 4 | 1,198 | ✅ complete (M3) |
| `tools/` | 5 | 1,057 | ✅ complete (M4) |
| `learning/` | 6 | 1,018 | ✅ complete (M2) |
| root (port-local) | 6 | 652 | no oracle — `main`, `units`, `order`, `id`, `guarded_fetch`, `chan_lossy` |
| **total** | **71** | **19,950** | against a 27,683-line oracle |

**Four oracle groups have zero Cyrius counterpart** — 8,473 lines, 31% of the
oracle, all scheduled and none silently missing:

| Group | Rust lines | Milestone |
|---|---|---|
| `fleet/` | 4,443 | M8 |
| `sandbox/` | 2,248 | M7 (77% of it; `wasm.rs` excluded — see roadmap) |
| `definitions/` | 1,460 | M10 (JSON only; ZIP + YAML excluded) |
| `telemetry/` | 322 | M9 (partial) |

**`src/main.cyr` is still a stub** — it prints `agnosai ready` and exits.
`agnosai_serve` exists and is tested but nothing calls it (roadmap A2).

## Where the port is

M0–M5 complete. **M6 (`server`) is 20 of 21 files**; the two remaining are
`sse.rs::event_stream` + `routes/sse.rs` (roadmap A1, large — its own session) and
the `main.rs` bind (A2, small). Until A1 lands,
`/api/v1/crews/{id}/stream` answers **501 deliberately** — not 404, because the
route exists and only its handler is missing (`src/server/router.cyr`).

Standing decisions that shaped the port, each with its record:

| Decision | Where |
|---|---|
| Money is integer micro-USD, converted to f64 only at the wire | port plan, open question 1 |
| Tool sandbox rides **cx + kavach**, not WASM | [ADR 006](../adr/006-cx-tool-sandbox.md) |
| LLM reached over an **HTTP seam**, not linked | [ADR 003](../adr/003-llm-native-http.md) |
| SSRF guard re-runs on every redirect hop (oracle checks only the first) | [ADR 007](../adr/007-audit-redirect-revalidation.md) |
| Auth compare is SHA-256-digest-based, not the oracle's length-leaking loop | [ADR 009](../adr/009-auth-constant-time-secret-compare.md) |
| JWT requires configured `iss`/`aud` | [ADR 010](../adr/010-jwt-require-configured-iss-aud.md) |
| `/metrics` serves agnosai's registry, not hoosh's | [ADR 011](../adr/011-metrics-endpoint-serves-agnosai-metrics.md) |
| `agnosai_chan_push_lossy` gives tokio-broadcast evict-oldest semantics | `src/chan_lossy.cyr` |

## Tests

**57 suites, 3,600 assertions, all passing.** Coverage `cyrius coverage --min 80`
→ **100% (899/899 fns)**, 64/64 files referenced.

```sh
cyrius tests tests        # 57 suites; each prints "N passed" with an "(N total)" suffix
cyrius coverage --min 80  # its own CI step — `cyrius audit` does not run it
```

**There is deliberately no per-suite table here any more.** One existed and it
drifted — six rows were wrong at the 2026-07-30 refresh, and the handoff note that
caught it said to *regenerate from command output rather than editing rows by
hand*. A table that can only be maintained by hand will drift again, so the two
commands above are the authority. Counting gotcha if you sum by hand:
`cyrius tests tests` prints one line **per suite** carrying an `(N total)` suffix,
plus a final `57 passed, 0 failed` line that counts **suites, not assertions** —
sum only the suffixed lines.

Corpus size: **807 KB** of `tests/`, against the **1,048,575-byte** coverage cliff
above which `--min` silently under-reports. ~77% consumed; watch it.

### Test-design decisions worth not re-deriving

The Cyrius suites deliberately exceed the oracle's coverage: they pin the UCB1
formula itself, the `max_by` last-wins tie rule, replay's zero-priority and NaN
fallback branches, and the Q-table's packed-key distinctness — none of which the
Rust tests reach.

`tools_builtin_load_testing.tcyr` could not follow its oracle's shape. Both Rust
tests stand up an axum mock server on loopback, and `agnosai_is_safe_url` correctly
refuses loopback — so the tool cannot be aimed at one, and pointing a test suite at
a public host is not acceptable. The **real OS-thread fan-out** runs against a
synthetic executor instead: worker threads, per-worker arenas, deadline and budget
loops, status aggregation, the sort and the percentile indices, no network. The one
path left untested is sandhi's behaviour under `sandhi_http_get_a`, covered
separately by the live `scripts/stack.sh check`.

`tools_builtin_security_audit.tcyr` solves it the same way and pays off better:
because the module is split at the network boundary, all five of its mock-server
oracle tests port exactly — `_t_mock_headers` transcribes
`mock_audit_server(security_headers, cors_wildcard)` down to the `Apache/2.4.99` it
always sets. It then goes past the oracle: both sides of every risk-band boundary,
the reflected-origin CORS bypass the probe origin exists to catch, the `to_str()`
visible-ASCII gate, case-insensitive scheme handling, and a snapshot-survives-reset
test that scribbles over the released arena so a borrowed pointer shows up as
corruption rather than passing silently.

`tools_agnos.tcyr` is where the transport seam pays off most. The oracle's three
suites test **only** names, descriptions and schemas, because every execute path
needs a live service. Since `agnosai_agnos_client_new` takes its transport as a
function pointer, a recording stub turns the whole untested half into ordinary
assertions: URL construction, form encoding, body construction, path-traversal
guards, response reshaping.

**Allocator assertions are a first-class test category now.** Every module threaded
for arenas carries an `alloc_used()`-delta assertion (0 bytes over N iterations)
plus a byte-identical-wire check against its global twin. Both are
**mutation-verified** — un-threading a single call fails them. Treat that pair as
the contract for any further threading work.

### Harness rules

- Shared helpers live in `tests/test_helpers.cyr`, all `_t_`-prefixed, so they can
  never shadow a `src/` symbol and stay out of the coverage denominator.
- Every `.tcyr` ends `var f = main(); if (f > 0) { f = 1; } syscall(60, f);` — the
  stock `proj-tcyr` epilogue masks the exit code `& 0xFF`, so exactly 256/512/768
  failures would score PASS.
- **A crash prints no assertion output.** A suite that dies mid-run reports as a
  failed *suite* with no `FAIL:` line — if a suite count drops with no failing
  assertion named, look for a segfault, not a bad expectation.
- Never `include` a stdlib module in a `.tcyr`; the stdlib is auto-prepended and an
  explicit include lands after it, single-passing into undefined symbols.
- Include order inside a `.tcyr` is load-bearing, callees before callers — e.g.
  `src/server/prometheus.cyr` must precede `src/orchestrator/crew_runner.cyr`.

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

`benches/order.bcyr` — the numbers that settled **port plan blocker #8**, and
then retired the code that settled it. The plan measured an O(n^2) insertion sort
over agnosai's 100k percentile vector at **52.6 s**. `src/order.cyr` first
vendored heapsort + quickselect to fix that; at cyrius **6.5.4** `vec_sort_by` /
`vec_select_nth` shipped (closing agnosai's own filing) and the module became a
thin wrapper. Both transitions, measured on this box:

| Benchmark | plan baseline | vendored | stdlib (current) |
|---|---|---|---|
| `sort_100k` | 52.6 s | 79.6 ms | **20.3 ms** |
| `sort_10k` | — | 6.30 ms | **1.71 ms** |
| `sort_100k_already_sorted` | — | 79.1 ms | **3.31 ms** |
| `three_percentiles_100k` | ~21 ms predicted | 10.7 ms | **7.81 ms** |
| `select_nth_100k` | — | 6.89 ms | **5.09 ms** |
| `select_nth_100k_already_sorted` | — | 4.29 ms | **3.77 ms** |

Against the 52.6 s baseline that is **~2,600x** for the full sort. The
already-sorted row moved 23.9x in the last step alone and is a difference in
kind: heapsort's worst case equals its average so it had no fast path, while
introsort checks for pre-sorted input first. A latency vector that arrives
roughly ordered now costs a scan.

`src/order.cyr` is **98 lines, down from 184** — the vendored algorithms are
gone. What did NOT delegate is the bounds contract: `vec_select_nth` aborts the
process on `k < 0` or `k >= len`, where `agnosai_select_nth` returns 0, pinned by
three assertions in `tests/order.tcyr`. An empty latency vector is an ordinary
state for a load test that recorded no samples.

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
| `auth_check_secret_short_token` | 942 ns |
| `auth_check_secret_ok` | 980 ns |
| `auth_check_secret_reject` | 961 ns |
| `auth_jwt_reject_bad_alg` | **1.201 ms** |
| `auth_jwt_key_prepare` | 10.5 µs |
| `auth_jwt_verify_ok` | **1.202 ms** |

The three `auth_check_secret_*` rows are the evidence for
[ADR 009](../adr/009-auth-constant-time-secret-compare.md), not padding. They sit within
**4%** of each other — accept 980 ns, reject 961 ns, and a 1-byte token against the same
9-byte secret 942 ns. The oracle's `max(a.len(), b.len())` loop would have made that last
row track the *secret's* length, which is the timing leak the ADR closes; the residual
~28 ns spread tracks the **token's** length, which the attacker chose and already knows.
`auth_check_disabled` at 6 ns is the first branch, and it is what every deployment that
has not configured auth pays per request. **That row is also why
`agnosai_auth_check` reads the clock only on the JWT path**: the obvious spelling,
`check_at(..., clock_epoch_secs())`, reads it before the config is inspected, and the bench
caught that turning this row into 1.35 µs and `auth_check_secret_ok` into 2.35 µs — both
paths where `now` is never looked at.

**`auth_jwt_verify_ok` was a sigil finding, and sigil fixed it.** It measured
3.31 ms while `lib/sigil.cyr` was 3.12.1-era, with the raw
`rsa_pkcs1v15_verify_sha256` at 3.29 ms — the port's own parsing, base64 and JSON
work was the remaining ~20 µs. sigil's "rsa repairs" (filed by agnosai as
`2026-07-30-rsa-verify-uses-secret-exponent-ladder.md`, archived upstream) removed
a constant-time secret-exponent ladder from the *public verify* path, where the
call site's own comment said it was unnecessary. Re-measured on 3.12.2:

| | 3.12.1 era | 3.12.2 (current) |
|---|---|---|
| `auth_jwt_verify_ok` | 3.31 ms | **1.202 ms** (2.75x) |
| `auth_jwt_reject_bad_alg` | 3.29 ms | **1.201 ms** |

Per-core JWT ceiling accordingly moves **~300/sec → ~830/sec**. A residual gap
against OpenSSL (14 µs for the same RSA-2048 verify on this box) remains and is
the inherent portable-bignum cost, not a further defect.

**`reject_bad_alg` still costs the same as a valid verify, and that is deliberate.**
The `alg` check sits *after* signature verification, so a rejected token pays the
modexp. The cheaper ordering was measured and rejected: checking `alg` first
rejects in 3.75 µs but parses attacker-controlled JSON before authenticating, and
`bayan_json_v_parse_buf` allocates on the no-free global bump — **62,248 bytes per
rejected request, ~53x amplification**, no credential required. Permanent memory
exhaustion is worse than recoverable CPU exhaustion. The answer to a flood is
`rate_limit`, which is ported but **not mounted** (roadmap D1, a human decision).

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

**git deps** (declare-ahead pattern, read from `cyrius.cyml` 2026-07-31):
sigil 3.12.2 · **bote 3.2.1** · majra 2.5.3 · kavach 3.9.3 · ai-hwaccel 2.3.16 ·
tyche 1.0.0.

> **bote 3.3.0 is released and not yet pinned** — it adds
> `dispatcher_set_server_info`, which removes the `"serverInfo":{"name":"bote"}`
> hardcode that stopped any consumer from using bote's MCP dispatcher without
> misidentifying itself. Bump when tagged (roadmap B1).
The hoosh seam targets **hoosh 2.6.0** — `usage.cost_micro_usd`, `usage.provider` and
`X-Hoosh-Cache` are read when present, and an older gateway degrades to an absent cost
rather than a fabricated one.
libro 2.8.4 arrives transitively via bote.

## Known issues in the current build

1. **35 duplicate-fn warnings at build, none from `src/` — all benign today, but know the rule.**
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
   - ~~**`_agnosai_is_digit`**~~ — ✅ **FIXED 2026-07-31.** Hoisted into
     `src/units.cyr`; both local copies removed. **agnosai's own duplicate-fn
     count is now zero** — all 35 remaining warnings are lib-vs-lib.

   **The compiler only warns for `fn`.** A duplicate `var` or enum member is
   **silent**, which a 2026-07-31 audit found the hard way: four duplicated
   top-level constants in `src/`, three with *different* values, three of them
   struct sizes passed straight to `alloc()`. Nothing misbehaved — purely because
   each file's own `alloc()` was parsed after its own definition and before the
   redefinition — but reordering `src/main.cyr`'s includes would have silently
   under-allocated a heap struct. All renamed to module-unique names, and
   `scripts/check-symbols.sh` now gates the whole class in CI ahead of the build.
   **`grep "duplicate fn"` on the build log is not a sufficient check** and
   should not be treated as one.

3. **The cleanliness gates were never in CI — only run by hand.** Until
   2026-07-31 `.github/workflows/ci.yml` ran the symbol check, build, and test
   and nothing else: no `fmt`, `lint`, `doc`, `vet`, `deny`, or `coverage`,
   despite CLAUDE.md specifying them at work-loop steps 2 and 6 and calling
   coverage out as "its own CI step". The drift that hid behind it was real —
   **31 undocumented public symbols** across five modules and four untracked
   lint deferrals. `scripts/check-clean.sh` now runs all five, and coverage is
   its own step after Test.

   Two traps it exists to avoid. **`cyrius fmt`, `lint` and `doc` each take a
   FILE**; written bare they print usage and exit 1, so a gate that checks only
   the exit code reads that as a failure and one that ignores it passes over
   zero files. And **`cyrius fmt <file>` without `--check` prints the formatted
   text to stdout rather than rewriting the file** — it is not an in-place
   formatter. The gate is itself mutation-tested: it was confirmed to fail on an
   undocumented symbol, a formatting violation, and an untracked deferral.
   (`cyrius vet` is the dependency-trust gate, not a code checker — an undefined
   symbol is caught by `build`, which refuses to emit.)

## Handoff

> **Refreshed 2026-07-31.** This section is a pointer, not a diary. Everything
> that is *owed* lives in [`roadmap.md`](roadmap.md) → *Open blockers and owed
> work*, which is the single list; if an item is not there, it is not owed. What
> already shipped and why is in `CHANGELOG.md`. This file carries only the numbers
> and the standing rules.

### Where the port is

| | |
|---|---|
| Cyrius port | **19,950 lines**, 71 files, `src/` mirroring `rust-old/src/` |
| Rust oracle | 27,683 lines at `rust-old/` — frozen |
| Milestones | M0–M5 complete; **M6 (`server`) at 20 of 21 files** |
| Remaining in M6 | `sse.rs::event_stream` + `routes/sse.rs` (roadmap A1), `main.rs` bind (A2) |
| Not started | M7 `sandbox`, M8 `fleet`, M9 `telemetry`, M10 `definitions` |
| Gates | build OK · 1,421 symbols / 71 files · fmt·lint·doc·vet·deny clean · 57 suites / 3,600 assertions · coverage 100% |

**The binary does not serve yet.** `agnosai_serve` exists and is tested, but
nothing calls it — `./build/agnosai` prints `agnosai ready` and exits. That is
roadmap A2, and it is small.

### Standing rules a new session should not re-derive

1. **`src/` mirrors `rust-old/src/`.** Anything walking it must recurse —
   `find src -name '*.cyr'`, never `src/*.cyr`, which now matches nothing and
   **fails open**. `cyrius coverage` and `cyrius tests` already recurse.
2. **Verify a toolchain bump by diffing `lib/` against the snapshot.** A green
   `cyrius lib sync --full` is not proof: on the bayan repo the same command
   reported success and left five files stale. `cyrius deps` re-layers git deps but
   does **not** refresh the stdlib.
3. **Git-dep pins must move with the fold.** `cyrius deps` copies each dep's
   bundle into `lib/` last-write-wins, so a stale `[deps.sigil] tag` overwrites a
   newer folded copy back down.
4. **Cyrius silently accepts a duplicate parameter name.** Adding an allocator
   parameter named `a` to a fn whose first parameter was already `a` compiles
   clean, and every `load64(a + OFFSET)` then reads the allocator. It SIGSEGVs with
   no assertion output. Check existing parameter names before prepending.
5. **`map_keys` defeats arena threading.** It materialises a key vec through
   `vec_new()` on the no-free global bump and has no `_a` form, so it survives
   every other substitution and silently caps the win. Use `_agnosai_map_slots` /
   `_slot_live` / `_slot_key` / `_slot_val` from `src/core/json.cyr`.
6. **The duplicate-symbol hazard is wider than `fn`.** The compiler warns on a
   duplicate `fn` and is **silent** on a duplicate `var` or enum member.
   `scripts/check-symbols.sh` gates the whole class; `grep "duplicate fn"` on a
   build log is not a sufficient check.
7. **Measure before claiming, and mutation-test the assertion.** Two findings this
   session reversed on measurement (the `to_wire` hoist looked worth 2.5% and was
   not worth 121 new globals; `map_keys` looked negligible and was the entire
   residual), and two bugs were caught only because the pinning assertion was
   mutation-verified.

### Consumers

Agnostic (Python platform), daimon (agent orchestration), joshua (NPC AI),
kiran (game AI) — none consuming the Cyrius line yet.
