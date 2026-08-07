# AgnosAI — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures
> (durable); this file is **state** (volatile).
> Last refreshed: 2026-07-30.

## Version

**1.1.0** (`VERSION`) — the last shipped Rust release, now preserved at
`rust-old/`. The Cyrius line targets **v2.0.0**; VERSION bumps once parity
lands, not before, so the number always names something that actually shipped.

## Toolchain

- **Cyrius pin**: `6.5.6` (`cyrius.cyml`) — bumped 2026-08-03. Folds **sandhi 1.9.9**
  (the stop facility agnosai filed), bayan 1.4.0, sigil 3.12.2, sakshi 2.4.7,
  yukti 2.3.2, mabda 4.0.8. 6.5.6 also lands **`sys_exit_group`** and
  **`async_await_readable_ms`**, both from agnosai filings.
- **`lib/` matches the pin exactly**: 0 of 106 stdlib files differ from
  `~/.cyrius/versions/6.5.6/lib`; build and test emit no drift or shadow warning.
  The six files outside the snapshot (ai-hwaccel, bote-core, kavach, libro, majra,
  tyche) are declared git deps, not staleness.

  **`lib/unicode/` is vendored BY HAND and nothing upstream maintains it.**
  `cyrius lib sync` copies only the top level, so declaring `"unicode"` in
  `[deps].stdlib` — which agnosai now does — does **not** bring the seven files
  under `lib/unicode/`. They were copied from the snapshot manually.

  It stopped being inert on 2026-08-03: `src/sandbox/oci.cyr` calls
  `unicode_category` to match the oracle's Unicode `is_alphanumeric` exactly, so
  those files are load-bearing. `scripts/check-clean.sh`'s snapshot check is
  therefore **recursive**, and is the only thing that will notice if they drift
  from the pin — mutation-verified against a one-byte edit.

  **Verify a bump by diffing the trees, not by a green `lib sync --full`.**
  `cyrius lib sync` skips on file **size**, not content, so a size-neutral change
  — in practice a version stamp — is never copied while the command reports
  success. The 6.5.6 bump left `lib/vani.cyr` a patch behind that way. **Impact
  here was nil** (vani is neither declared nor called), and that is the usual
  shape: real code edits change length and do sync. Filed as a papercut.
  `scripts/check-clean.sh` diffs `lib/` against the pinned snapshot, which
  `deps --verify` cannot do — the lock is written *from* disk, so a stale file
  just gets its stale hash recorded.

### Pins must name what is actually built — corrected 2026-08-03

`cyrius.cyml` pinned **bote 3.2.1** and **kavach 3.9.3** while `lib/` held **bote
3.3.0** and **kavach 3.11.0**. Both vendored bundles are byte-identical to their
upstream tag dists (verified by sha256 against `git show <tag>:dist/...`). The pins
are now **bote 3.3.0** / **kavach 3.11.0**, which is what was already being compiled
and tested.

**The mechanism, because it will recur.** Every `[deps.NAME]` carries
`path = "../NAME"` alongside `git` + `tag`, and **the local path wins**. A developer
whose sibling checkout has moved ahead silently builds a version the manifest does
not name; CI, which has no sibling checkouts, resolves the *tag* and builds
something else. Here that was kavach **3.11.0 locally against 3.9.3 in CI** — the
`~/.cyrius/deps/kavach/3.11.0/` clone predates the session that found it.

This is the inverse of the sigil rule recorded below, and both are live: a stale tag
can *overwrite* a newer folded copy (sigil's case) **or** be quietly overridden by a
newer local path (kavach's case). The lockfile does not save you from the second —
`cyrius.lock` recorded 3.9.3's sha256 against a 3.11.0 file on disk and nothing
surfaced it, because every other gate reads `src/` and the build compiles whatever
bytes `lib/` holds.

**Two gates now close the class**, both mutation-verified:

- `scripts/check-clean.sh` runs **`cyrius deps --verify`** (lock vs. working `lib/`).
  Restoring the stale hash makes it print `FAIL: lib/kavach.cyr (hash mismatch)` and
  exit **1**. Note the exit code must be read directly — piping through `tail` reports
  `tail`'s status and reads as a pass.
- CI gains a **Lockfile is honest** step: `git diff --exit-code -- cyrius.lock` after
  `cyrius deps`, which catches a committed lock that disagrees with a clean tag-only
  resolution — i.e. exactly the local/CI divergence above.

**What the two bumps actually change: nothing on any path agnosai executes.**
kavach 3.10.0/3.11.0 are `--agnos` target build fixes — nine additive `kv_*` shims
(`kv_unlink`, `kv_rmdir`, `kv_waitpid`, `kv_getgid`, `kv_lstat`, `kv_fork`, `kv_dup2`,
`kv_execve`, `kv_setsid`); agnosai calls only `score_agent`, `score_agent_with_tools`,
`sandbox_display`, `sandbox_strength`, and the diff touches none of them. bote 3.3.0
adds `dispatcher_set_server_info` and is additive by construction — an unconfigured
dispatcher emits the pre-3.3.0 wire byte for byte. The duplicate-fn warning count is
unchanged at **35**, and all 57 suites stayed green across the bump.

### What 6.5.6 lands on agnosai

- **sandhi 1.9.9 — the serve-loop stop facility agnosai filed.**
  `sandhi_server_options_stop_flag(opts, ptr)` on all five loops, returning 0 for
  a requested stop against 1 for a failure. This is what made graceful shutdown
  possible; consumed immediately in `src/server/serve.cyr` +`src/main.cyr`, and
  recorded as [ADR 013](../adr/013-graceful-shutdown-via-signalfd-and-stop-flag.md).
- **`sys_exit_group`** (`lib/syscalls_linux_common.cyr:155`) — also agnosai's
  filing. `_agnosai_exit_process` now composes it instead of a hand-rolled
  `syscall(SYS_EXIT_GROUP, …)`; the `#ifdef CYRIUS_TARGET_LINUX` guard stays,
  because that file is included only by the two Linux target files.
- **`async_await_readable_ms`** — agnosai's filing, found while building sandhi
  1.9.9. Not used here; it removes the reason sandhi's cooperative loop polls.

### What 6.5.5 landed

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
tree 2026-08-03:

| Group | Files | Lines | Oracle |
|---|---|---|---|
| `orchestrator/` | 16 | 5,311 | ✅ complete (M5) |
| `server/` | 11 | 4,008 | ✅ complete (M6) |
| `core/` | 8 | 3,129 | ✅ complete (M2) |
| `tools/builtin/` | 6 | 2,017 | ✅ complete (M4) |
| `server/routes/` | 10 | 2,040 | ✅ complete (M6) |
| `llm/` | 4 | 1,198 | ✅ complete (M3) |
| `tools/` | 5 | 1,057 | ✅ complete (M4) |
| `learning/` | 6 | 1,018 | ✅ complete (M2) |
| root (port-local) | 6 | 961 | no oracle — `main`, `units`, `order`, `id`, `guarded_fetch`, `chan_lossy` |
| **total** | **72** | **20,739** | against a 27,683-line oracle |

**Four oracle groups have zero Cyrius counterpart** — 8,473 lines, 31% of the
oracle, all scheduled and none silently missing:

| Group | Rust lines | Milestone |
|---|---|---|
| `fleet/` | 4,443 | M8 |
| `sandbox/` | 2,248 | M7 (77% of it; `wasm.rs` excluded — see roadmap) |
| `definitions/` | 1,460 | M10 (JSON only; ZIP + YAML excluded) |
| `telemetry/` | 322 | M9 (partial) |

**`src/main.cyr` is no longer a stub** — as of 2026-08-03 it wires the shared
state, installs a `signalfd` SIGINT/SIGTERM handler, and calls `agnosai_serve`
(roadmap A2, closed). **It drains on shutdown**
([ADR 013](../adr/013-graceful-shutdown-via-signalfd-and-stop-flag.md)); ADR 012,
which recorded that as impossible, is superseded — the blocker was fixed upstream
in sandhi 1.9.9 on agnosai's own filing, hours after it was written.

## Where the port is

**M0–M6 complete.** The server tier is done: bite 16 (bind) and bite 15c (SSE)
both landed 2026-08-03, so `./build/agnosai` binds, reads the oracle's
environment, answers the full route table, **streams crew events**, and drains
on SIGINT/SIGTERM.

`/api/v1/crews/{id}/stream` now streams. The 501 in
`agnosai_route_dispatch` is unreachable over the transport —
`agnosai_serve_handler` intercepts the route first, because a dispatcher that
returns "a status and a finished body" cannot express a stream. It remains the
honest answer for a direct dispatch, which is what `tests/server_router.tcyr`
still asserts.

**One divergence worth knowing before deploying**: an SSE stream holds one of
the 100 pool workers for its whole life, where the oracle serves effectively
unbounded concurrent streams —
[ADR 014](../adr/014-sse-stream-holds-a-pooled-worker.md) has the tower analysis
showing why the "the oracle starves at 100 too" intuition is wrong.

Verified live, not inferred: `/health` → 200 `{"status":"ok"}`, `/metrics`
renders the registry, `/api/v1/tools` lists the four registered builtins, and
the stream route returns 501. `PORT` / `AGNOSAI_PORT` / `HOOSH_URL` and the four
auth variables were each exercised against a running binary — see the CHANGELOG
for the eight port cases and nine auth cases and what each proves.

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

**65 suites, all passing.** The M7 audit's 43 findings are **all fixed** as of
2026-08-05 — `docs/development/m7-audit-2026-08-04.md` marks each one in place
and `roadmap.md`'s M7 section has the shape of the remediation. The sandbox
suites grew **627 → 727 assertions** working it:

| suite | before | after |
|---|---|---|
| `sandbox_policy` | 90 | **102** |
| `sandbox_oci` | 100 | **118** |
| `sandbox_kavach_bridge` | 93 | **136** |
| `sandbox_spawn` | 144 | **169** |
| `sandbox_process` | 108 | **130** |
| `sandbox_python` | 61 | **76** |
| `sandbox_manager` | 69 | **90** |
| `sandbox_cx` | 62 | **87** |

**41 of the 43 fixes are mutation-verified** — mutation applied, suite re-run,
failing assertion named, tree restored. The two that are not (L5, L9) are
log-only and their mutants survive; the audit document says so rather than
counting them.

**A green suite is not evidence and this is the standing example.** 4,372
assertions and 100% reference coverage coexisted with three live crashes, two
fail-open divergences, and five security controls that could be deleted without
a single failure. Reference coverage counts whether a symbol is *named* by a
test.

**The whole tree is green, verified two ways on 2026-08-05.** `cyrius tests
tests` completes and reports `66 passed, 0 failed` (suites, not assertions —
see the counting note below), summing to **5,403 assertions passed, 0 failed,
0 suites unclean**.

⚠ **A note here previously said the full run "exceeds ~570 s and stalls in
`server_sse`". It does not stall** — it completes, well inside a 25-minute
bound, and `server_sse` passes **56 assertions in seconds** when built and run
on its own. It *is* slow, so the per-suite loop is still the faster way to work:
`cyrius build tests/<name>.tcyr <out> && <out>`. The eight sandbox suites total
~75 s that way.

Coverage `./scripts/check-coverage.sh 80` → **100% (1074/1074 fns)**. The
denominator counts public symbols only — `_`-prefixed internals are excluded by
design, which is why adding six of them moved it by one. **`cyrius coverage
--min 80` reads 99% and is wrong**; the corpus outgrew its buffer, see the
warning below.

```sh
cyrius tests tests          # 66 suites; each prints "N passed" with an "(N total)" suffix
./scripts/check-coverage.sh 80   # the real gate — reads the whole corpus
cyrius coverage --min 80    # its own CI step, but truncating on this tree
```

**There is deliberately no per-suite table here any more.** One existed and it
drifted — six rows were wrong at the 2026-07-30 refresh, and the handoff note that
caught it said to *regenerate from command output rather than editing rows by
hand*. A table that can only be maintained by hand will drift again, so the two
commands above are the authority. Counting gotcha if you sum by hand:
`cyrius tests tests` prints one line **per suite** carrying an `(N total)` suffix,
plus a final `66 passed, 0 failed` line that counts **suites, not assertions** —
sum only the suffixed lines.

Corpus size: **1,082,744 bytes** of `.tcyr`. This crossed `cyrius coverage`'s
fixed 1 MiB corpus buffer on 2026-08-05 and the tool began under-reporting
silently at exit 0 — it named two fully-covered files as carrying unreferenced
functions. **Fixed upstream in cyrius 6.5.8** (four defects, including an
off-by-one at the corpus end and `pub fn` being invisible to the tool whose job
is counting public functions), and verified here: at 34 KB past the old cap it
reports 1084/1084, identical to an independent count over the untruncated
corpus.

`scripts/check-coverage.sh` existed for the fortnight in between and has been
**removed** — a second implementation of a working tool is two things to
maintain and two things to drift. The episode's lesson is kept as standing
rule 10.

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

**Four things that made an arena assertion look stronger than it was.** All four
were caught by mutation, and all four generalise to any further threading:

1. **An empty fixture cannot tell a threaded route from an un-threaded one.** An
   empty collection allocates almost nothing either way. `/api/v1/dashboard/agents`
   is the sharp case: it emits an object only for a result whose metadata *names*
   an agent, so a fixture of eight crews with no agent metadata renders `[]` and
   times its guard.
2. **A route's failure arms are separately threadable and separately
   forgettable.** Measuring only the success path let `route_error_a` revert to
   `route_error` inside `agnosai_route_get_crew_a` with the whole suite green.
   The 404 is also the arm an unauthenticated scan hits hardest.
3. **Round-number thresholds assert less than they look like they do.** 128 B sat
   above a 32 B baseline and below the 176 B of the smallest un-threading
   mutation — and still passed one, because a single hoisted `str_from` costs
   16 B and lands at 48. Prefer a bound **measured in the same run**:
   `agnosai_route_resolve`'s own cost is what the arena path should equal, so
   asserting against it is self-calibrating across allocator changes.
4. **Ratios stop discriminating once the numerator is near zero.** At 4,160 → 32
   every plausible mutation still clears 4x. Absolute equality against a measured
   residual is the stronger statement and the one that is actually true: the
   handler half is zero, not merely small.

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
| `audit_record` | 29.5 µs |
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

`benches/server.bcyr` — the HTTP request path, added 2026-08-05. No oracle bench
file exists: the Rust request path is axum's, and timing it would have measured
tokio. The port's is `agnosai_route_dispatch`, ordinary synchronous code.

Each threaded read route is timed on **both** arms, because threading is not free
by construction — every `_a` call carries an extra argument and `alloc_via` is one
indirection past `alloc`. The paired rows are what makes "the arena bought the
memory without costing latency" a measurement rather than an assertion. Fixture:
8 finished crews carrying agent metadata, 8 pending approvals, 1 definition, 1
registered tool. `reset_via` is **inside** the timed loop, since a sandhi worker
pays it per request.

| Benchmark | global | arena | Δ |
|---|---|---|---|
| `route_approvals_*` | 2.147 µs | **1.861 µs** | −13.3% |
| `route_tools_*` | 3.104 µs | **2.168 µs** | −30.2% |
| `route_definitions_*` | 3.356 µs | **2.529 µs** | −24.6% |
| `route_get_crew_*` | 4.830 µs | **3.621 µs** | −25.0% |
| `route_dashboard_crews_*` | 13.266 µs | **10.956 µs** | −17.4% |
| `route_dashboard_agents_*` | 16.452 µs | **14.048 µs** | −14.6% |
| `route_approvals_post_*` | 3.731 µs | **2.983 µs** | −20.0% |
| `route_mcp_*` | 5.618 µs | **3.808 µs** | −32.2% |
| `route_resolve` | 343 ns | — | in both |

The arena arm wins on all six because the global allocator is a **no-free bump**:
a request's garbage is never reclaimed, so a long-lived process walks an
ever-growing heap and loses the locality a reset arena keeps.

The other half of the same claim is memory, asserted rather than benchmarked —
`tests/server_serve.tcyr` measures it over the same fixture, bytes charged to the
global bump per request:

| Route | global | arena |
|---|---|---|
| `GET /api/v1/dashboard/agents` | 4,368 | **0** |
| `GET /api/v1/dashboard/crews` | 4,160 | **0** |
| `GET /api/v1/crews/{id}` | 2,352 | **0** |
| `GET /api/v1/tools` | 1,720 | **0** |
| `GET /api/v1/approvals` | 576 | **0** |
| `GET /api/v1/agents/definitions` | 384 | **0** |
| `GET /api/v1/crews/{unknown}` (404) | 320 | **0** |

**Every GET read route charges the global bump literally nothing.** Handler,
router, id validation, tool schemas and the response struct all land in the
request arena and are freed by one `reset_via`.

### ⚠ The arena is a fixed cliff, and the routes now spill rather than fault

`arena_alloc` answers **0** when full. It does not grow, and the stdlib has only
a fixed-capacity arena — no chunked or growable form. **Nothing in
`bayan_json_v_*`, `str_from_a` or `vec_push_a` checks its allocator's answer**,
so an exhausted arena hands out 0 and the next store segfaults.

That was harmless before B2, because handlers allocated from the unbounded
no-free global bump. Threading them onto a fixed 64 KiB per-request arena turned
a leak into a crash: **`GET /api/v1/dashboard/crews` with 200 crews registered
segfaults through a raw 64 KiB arena**, and `AGNOSAI_MAX_RETAINED_CREWS` is
1,000. `GET /api/v1/crews/{id}` on a long crew and `GET /api/v1/tools` with many
tools share the shape.

`agnosai_spill_arena` (`server/serve.cyr`) serves from the arena and falls back
to the global bump **per allocation**; `agnosai_serve_handler` wraps sandhi's
own request arena in one, cached per worker in a thread-local. sandhi still owns
and resets the arena — only the ceiling behaviour changes. Spilled bytes are
global-bump bytes and are not reclaimed, which is exactly the pre-threading
cost applied to the overflow alone.

**The lesson for any future arena work here: an allocation measurement proves
the common case and says nothing about the ceiling.** Every fixture in the suite
was small, so nothing caught this until the ceiling was probed deliberately.

**Stated precisely, because "nothing checks" is wrong**: `arena_alloc` returns 0
when full, `str_from_a` checks and returns 0, `vec_push_a` checks and returns
-1. The primitives are fine. The gap is that **a `Str` of 0 is indistinguishable
from a valid one** — there is no option type and no error channel through the
`_a` families — so the 0 flows on and the next deref faults. Guarding a few
hundred `_a` calls in a route handler is not a thing anyone will do, and there
is nowhere to return the failure to.

Filed upstream as
`2026-08-06-arena-is-fixed-capacity-and-answers-0-so-unbounded-work-cannot-use-one.md`,
asking for a growable arena. **That filing also records why `crew_runner` is
blocked**: 97% of a crew run is transient (32,608 B per 4-task run against 894 B
retained), but the size is unbounded, so no fixed capacity works and a spilling
wrapper would spill most of it.

### The write routes split — every one is threaded, three keep a floor

| write route | B/req global | B/req arena | retains data from the parse tree? |
|---|---|---|---|
| `POST /mcp` | 3,224 | **0** | no |
| `POST /api/v1/approvals` | 1,352 | **0** | no |
| `POST /api/v1/agents/definitions` | 2,280 | **392** | yes, but **cloned** |
| `POST /api/v1/crews` | 21,184 | **17,048** | yes, but **cloned** |
| `POST /api/v1/a2a/receive` | 17,192 | **15,624** | yes, but **cloned** |

`agnosai_agent_from_value` and the crew *request* deserialisers took
`bayan_json_v_str(...)` **without cloning**, and what they build is stored in
`AppState.definitions` and the orchestrator registry — both process-lifetime. So
a stored definition pointed *into the parsed request body*. Threading those
parses would make `reset_via` reclaim it at the end of the request, and the next
request would be handed the same bytes: **a stored definition's name becomes
whatever the next caller posted.** It corrupts, it does not crash.

**That code was correct only because the global allocator never frees**, and the
dependency was undocumented. `server_state`'s `definition_insert` had seen half
of it — it `str_clone`s the *key*, with the comment "the caller's Str may be
borrowed from a request buffer that does not outlive the handler" — and left the
value borrowed.

✅ **Fixed for agents.** `agnosai_agent_from_value_a(al, v)` clones every Str
into `al`, so the caller picks the lifetime: `create_definition_a` passes
`default_alloc()` for the definition while parsing and responding in the request
arena. 2,280 → **392 B/request**, and the 392 *is* the retained definition —
what remains on the global bump is exactly the object the route exists to keep.

✅ **Done for crews.** `agnosai_crew_req_from_value_a` clones the crew name and
`process`; `agnosai_task_req_from_value_a` clones each task's `description` and
`expected_output`. 21,184 → **17,048 B/request**. The remainder is crew
*execution* — task results held in the orchestrator registry — which is retained
state and must not move.

**What retains a borrowed string here is not the crew registry.** `CrewState`
holds a minted UUID, a status, results and a profile; nothing from the request.
The two real paths are the **audit chain** (`_agnosai_orch_register` and
`_agnosai_orch_finish` pass `agnosai_crew_name(spec)` as the audit *message*,
and `agnosai_audit_record` stores it without cloning into `AppState`'s chain)
and the **task result** (on the placeholder path `crew_runner` makes the
description the result's output, `crew_runner.cyr:460`). Asserting on
`agnosai_crew_state_crew_id` looked right and pinned nothing — every mutation
survived it.

⚠ **Not `agnosai_crew_from_value`**, which an earlier note named from its name
alone: it deserialises a *persisted* crew, requires an `id` the request shape
never carries, and **has no caller in `src/`**. Check the call chain from the
route rather than matching on a plausible name.

✅ **Done for a2a.** `agnosai_a2a_req_from_value_a` clones the task id, the
description and the four optional strings. 17,192 → **15,624 B/request**;
`metadata` still points into the parse tree, deliberately, because it is only
re-serialised for the size check and never retained.

**Every route in the table is now threaded.** The residuals are all retained
state rather than garbage.

✅ **`agnosai_audit_record` owns the strings it keeps.** `event`, `level`,
`message`, `provider` and `model` are cloned, so the chain no longer depends on
every caller happening to pass an owned Str — the same boundary
`definition_insert` already applies to its key. The clone is free at this scale:
`audit_record` stays inside its 28,335–29,785 ns band, because the SHA-256 chain
hash and signature dominate.

⚠ **`metadata` remains stored by reference**, and that contract is now narrowed
to it alone: bayan ships **no deep-copy primitive**, and a `build`-then-`parse`
round trip would cost a full serialise per audit record on the crew hot path.
Callers must pass a tree that outlives the chain; both build theirs with a bare
`bayan_json_v_obj_new()`.

**`str_builder_build`, not `str_builder_new`, decides where a built Str lives.**
The builder's scratch buffer can be anywhere; `build` allocates the finished Str
and copies into it. A mutation moving `str_builder_new` to an arena changes
nothing and passes; moving `str_builder_build` fails at once. Worth knowing
before writing a comment about which call is load-bearing.

The test that pinned the hazard was **inverted, not deleted** — it asserted a
stored name was destroyed after its arena was released and scribbled over, and
now asserts every field survives. Allocating the definition from the request
arena segfaults the suite, which is the documented crash signature.

It took four bites and each one exposed the next residual, which is the pattern
worth carrying forward — **a floor is only visible once everything above it is
zero**:

1. the handlers (`llm`, routes, tools, orchestrator groups) → the router's
   32-byte match struct became the whole residual;
2. the router (`agnosai_route_resolve_a`, plus the two-level restructure) →
   `agnosai_uuid_parse`'s 16 bytes became the whole residual on `{id}` routes;
3. `agnosai_uuid_is_valid` → only `/api/v1/tools` was left, at 288 B;
4. the tool vtable — `schema_fp` is now `fn(a, ctx)`, allocator first, threaded
   by all fourteen implementors.

Step 4 is a change to a **calling convention**, not an addition to one, and it
is safe to make because agnosai has no `[lib]` block and no `dist/` bundle:
`bins = ["agnosai"]` is the whole package, so every implementor is in this tree.
Consumers reach tools over HTTP and MCP, not by linking.

The `{id}` routes reached zero last, and the residual was `agnosai_uuid_parse`
allocating a buffer **all five of its callers in `src/` discarded** — every one
was `if (agnosai_uuid_parse(x) == 0)`, a validity test.
`agnosai_uuid_is_valid` is that test without the buffer, over the same
`_agnosai_uuid_scan`, so the two cannot drift on what they accept — which
matters because the routes decide **422-vs-404** on the validator where the
oracle's `Path<Uuid>` extractor decided it on the parser.

**`route_resolve` was 1.675 µs — 57% of the cheapest threaded request — and its
32 B was 100% of that route's remaining allocation. Both are fixed.**

The table was a flat list of sixteen patterns, each matched from byte 0, so
`/api/v1/dashboard/crews` matched on attempt sixteen having re-compared `api`
and `v1` twelve times. `/api/v1` is now consumed once and the following segment
selects a group of at most four patterns: **1.675 µs → 343 ns (−79%)**, and
`agnosai_route_resolve_a` puts the match struct, the parameter buffer and any
captured Str in the arena. `/api/v1/approvals` fell 2.947 → 1.861 µs, the
largest proportional gain, because resolve was most of it.

**The restructure is a filter, not a decision**, and that is the safety
argument: `_agnosai_seg_is` and `_agnosai_path_prefix_end` choose which patterns
are *tried*, while what a trial *answers* is still a full pattern comparison. So
neither selector can route a request somewhere it does not belong. This is
verified rather than argued — `_t_resolve_is_equivalent` holds the resolver
against the flat sixteen-attempt table it replaced, kept verbatim in
`tests/server_router.tcyr`, across 56 paths × 4 methods on id, captured
parameter and the 405-vs-404 flag. **Making `_agnosai_seg_is` a bare prefix test
changes no answer it produces**, because the resolver degenerates back into the
flat table; the two selectors therefore carry their own unit assertions, since
what they protect is the cost and the differential cannot see cost.

Not threaded, and not claimed to be: the write routes (`POST /api/v1/crews`,
`/api/v1/agents/definitions`, `/api/v1/approvals`, `/api/v1/a2a`, `/mcp`), which
parse a request body — the other half of the problem — and the orchestrator
group's off-request-path modules, `crew_runner` foremost.

**Not comparable to `rust-old/bench-history.csv`** — different allocator, different harness, no
criterion statistics. The Cyrius line starts its own baseline, captured by
`scripts/bench-history.sh` into the root `bench-history.csv` (85 rows per capture).

## Dependencies

**stdlib** (44 declared, now including `unicode`, order-sensitive — rationale in [`cyrius-port-plan.md`](cyrius-port-plan.md)):
base substrate · general utilities · bayan · patra · concurrency+crypto floor ·
dynamic-link floor · async · net/http/tls/ws/sakshi/sandhi

**git deps** (declare-ahead pattern, read from `cyrius.cyml` 2026-08-05):
sigil 3.12.2 · **bote 3.3.0** · majra 2.5.3 · **kavach 3.11.7** · ai-hwaccel 2.3.16 ·
tyche 1.0.0.

**kavach 3.11.7 verified end to end, 2026-08-05** — the check `state.md` itself
prescribes, run in all three places rather than assumed:

| bundle | sha256 (16) |
|---|---|
| `git show 3.11.7:dist/kavach.cyr` | `e959d81aa2b370f0` |
| kavach worktree / `origin/main` (`6567a65`) | `e959d81aa2b370f0` |
| `agnosai/cyrius.lock` | `e959d81aa2b370f0` |

Remote tag confirmed at `6567a65` through the GitHub API, so the local `path =
"../kavach"` override and a tag-only CI resolution produce the same bytes —
which is the condition the *Pins must name what is actually built* rule exists
to enforce, satisfied rather than merely intended.

`kavach_bridge` requires 3.11.7: `exec_result_set_stdout_n` does not exist
before it, so an older pin does not compile rather than silently under-scanning
— see roadmap C2.

**All six pins are now the newest upstream tag**, confirmed against the GitHub API
rather than a local `git fetch` — the dep remotes are SSH (`git@github.com:`) and a
keyless fetch fails *silently enough to look like "no new tags"*. Use
`curl -sf https://api.github.com/repos/MacCracken/<dep>/tags` to check.

> **bote 3.3.0 is pinned as of 2026-08-03** — it adds
> `dispatcher_set_server_info`, which removes the `"serverInfo":{"name":"bote"}`
> hardcode that stopped any consumer from using bote's MCP dispatcher without
> misidentifying itself. This unblocks roadmap B1's *next* surface; it does **not**
> mean `routes/mcp.cyr` should delegate to the dispatcher today, because the oracle
> uses bote's protocol types and explicitly declines its Dispatcher
> (`rust-old/src/server/routes/mcp.rs:3-5`), so hand-building the envelope IS the
> parity behaviour.
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

2. **`cyrius bench` accepts an argument it cannot use and reports success —
   same root cause as the `cyrius build` missing-file bug.** Measured 2026-08-05
   on cyrius 6.5.6: `cyrius bench benches` (a directory), `cyrius bench
   no-such-dir` and `cyrius bench no-such-file.bcyr` each run **zero benchmarks,
   print no diagnostic, and exit 0**. Only the **no-arg** form prints the
   `=== N passed, M failed ===` summary, so a gate written any other way has
   nothing to check and passes over nothing.

   **`bench <file>` builds and runs that file as a program** — it does not match
   the argument against a benchmark registry. `cyrius bench
   tests/sandbox_policy.tcyr` runs the *test suite* and prints `102 passed`. So
   an argument that matched nothing is the already-filed `cyrius build` case
   (stdlib prepended, entry file never opened, a valid empty translation unit)
   with a run appended — the compiler output still shows the `unreachable fns`
   note, i.e. it compiled the stdlib and ran that.

   `bench` is the only command in the family that fails this way. `cyrius tests
   <file>` and `cyrius tests no-such-dir` both exit **1** with `not a
   directory: ...`; bare `fmt` and `lint` exit 1 with usage; `cyrius lint
   no-such.cyr` exits 1 with `cannot read file`. Filed upstream as
   `2026-08-05-cyrius-bench-accepts-an-unusable-argument-and-exits-0.md`,
   cross-triaged with the `build` filing.

   `scripts/bench-history.sh` calls the bare form and is therefore correct.

   **CLAUDE.md's rule about subdirectories is right, and a note here previously
   said it was stale.** No-arg discovery scans `benches/` and `tests/` at their
   **top level only**: a probe `.bcyr` planted at `benches/zzsub/` was not found
   (still `6 passed`, marker absent from the output), and one planted at the
   repo root was not found either. `benches/` is a scanned location, not a
   counter-example to the rule.

   **Unexplained on the same run**: `lt_aggregate_100k_10workers` measured
   **25.3 ms** against the 79.2 ms recorded below, with `sort_100k` unchanged at
   20.5 ms against 20.3 — so it is not the sort, and nothing in the tools path
   was touched. Recorded as an observation, not claimed as a win; the table
   below is left at its published numbers until someone reproduces the gap.

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
> that is *owed* lives in [`roadmap.md`](roadmap.md) → *Owed work*, which is the
> single list; if an item is not there, it is not owed. What
> already shipped and why is in `CHANGELOG.md`. This file carries only the numbers
> and the standing rules.

### Where the port is

| | |
|---|---|
| Cyrius port | **24,694 lines**, 82 files, `src/` mirroring `rust-old/src/` |
| Rust oracle | 27,683 lines at `rust-old/` — frozen |
| Milestones | **M0–M6 complete** — the server tier is done and the binary serves |
| Remaining in M6 | nothing — bites 15c (SSE) and 16 (bind) both closed 2026-08-03 |
| In progress | **M7 `sandbox`** — all eight modules ported; the 2026-08-04 audit's 43 findings all fixed 2026-08-05 |
| Not started | M8 `fleet`, M9 `telemetry`, M10 `definitions` |
| Gates | build OK · 1,697 symbols / 82 files · fmt·lint·doc·vet·deny·deps--verify·lib-snapshot·corpus-warn clean · coverage **100% (1074/1074) via `scripts/check-coverage.sh`** — `cyrius coverage` reads 99% and is truncating |

**The binary serves as of 2026-08-03**, and **drains on SIGINT/SIGTERM** as of
the 6.5.6 bump the same day. `src/main.cyr` wires the state, installs a
`signalfd` handler, and calls `agnosai_serve`. ADR
[013](../adr/013-graceful-shutdown-via-signalfd-and-stop-flag.md) supersedes
[012](../adr/012-no-graceful-shutdown-on-sandhi.md): the blocker was never the
signal helper this file and the roadmap both used to cite — it was that sandhi's
accept loop could not be made to return, which agnosai filed and sandhi 1.9.9
fixed.

### Standing rules a new session should not re-derive

1. **`src/` mirrors `rust-old/src/`.** Anything walking it must recurse —
   `find src -name '*.cyr'`, never `src/*.cyr`, which now matches nothing and
   **fails open**. `cyrius coverage` and `cyrius tests` already recurse.
2. **Verify a toolchain bump by diffing `lib/` against the snapshot, and write
   the lockfile last.** A green `cyrius lib sync --full` is not proof: on the
   bayan repo the same command reported success and left five files stale.
   `cyrius deps` re-layers git deps but does **not** refresh the stdlib — and it
   writes `cyrius.lock` from what it just copied, so a sync afterwards leaves
   `deps --verify` failing. The order that satisfies both gates:

   ```sh
   cyrius deps --no-lock     # resolve git deps into lib/
   cyrius lib sync --full    # refresh the stdlib over them
   cyrius deps --lock        # record what is actually there
   ```

   **A sibling checkout can hold the whole tree back, and `lib sync` there is
   not the fix.** `[deps.*] path = "../X"` makes `cyrius deps` re-layer that
   checkout's own `lib/` over the fold — and `cyrius build` performs an implicit
   resolve, so the file flips back on **every build**, not just on an explicit
   `deps`. At 6.5.8 that kept `lib/sakshi.cyr` at 2.4.7 even after sakshi 2.4.8
   shipped.

   The load-bearing step is the **pin**, not the sync: `cyrius lib sync` reads
   `~/.cyrius/versions/<the pin>/lib`, so a sibling still on an older
   `cyrius = "X.Y.Z"` merely re-syncs its own old snapshot. Each sibling's
   sakshi version tracked its pin exactly (sigil 6.5.3, bote 6.5.4, kavach
   6.5.5, ai-hwaccel 6.5.2, majra 6.4.83, tyche 6.2.11), which is what
   identified the mechanism. Bump the sibling's pin first, then run the
   three-step there.
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
8. **When an assertion cannot be made to fail, that is usually a statement about
   the code's shape, not the test's.** The M7 audit remediation's recurring
   defect was an *unreachable* control, not an unasserted one — an env filter
   that reads `/proc/self/environ` cannot be tested by a process with no
   `setenv`. Five fixes worked by splitting the function so it takes its input
   as a parameter (`_agnosai_sanitized_envp_of`, `_agnosai_process_envp_from`,
   `_agnosai_cx_resolve_interpreter`, `_agnosai_cx_budget_ms`,
   `_agnosai_sandbox_manager_timeout_ms`); two more by **re-execing the suite
   through `agnosai_spawn_capture_input` with a planted `envp`**, which is the
   only way to put a variable in this process's own environment. Reach for one
   of those before concluding a behaviour is unobservable.
9. **`callptr` does not check arity, so changing a function-pointer signature is
   only half-caught by the compiler.** The tool vtable's `schema_fp` went from
   `fn(ctx)` to `fn(a, ctx)`. Implementors still declaring the one-parameter
   shape **compiled and passed** — `ctx` silently received the allocator and the
   real ctx was dropped, which is invisible in a schema that ignores `ctx`. Two
   test tools in `tools_native.tcyr` were in exactly that state and the suite
   reported 71/71.

   The *detectable* half is a direct call: `_agnosai_delegate_schema(0)` in
   `tools_builtin_delegate.tcyr` then passed 0 as the **allocator**, and
   `alloc_via` followed it — a segfault, so a failed suite with no `FAIL:` line
   (see the crash note in the harness rules).

   **Neither showed up in per-suite verification.** The change was mutation-
   tested against `server_serve` and `tools_native`, both green; only
   `cyrius tests tests` across the whole tree caught it. When a signature that
   is reached through `callptr` changes, grep for every implementor and every
   direct caller by name — the build will not.

10. **A tool that prints a number can still be lying, and the fix is upstream.**
   `cyrius coverage` under-reported for a fortnight because this tree's `.tcyr`
   corpus outgrew a fixed 1 MiB buffer — silently, at exit 0, naming
   fully-covered files as uncovered. It was worth a local reimplementation only
   until upstream shipped (6.5.8), and that reimplementation was then deleted.
   The durable half: when a gate's number moves and nothing in the diff explains
   it, **suspect the gate before the code** — and confirm by computing the same
   thing a second way rather than by reasoning about it.

11. **Verify the mutation is a faithful revert.** One written during the M7
   remediation changed half of a two-line fix, left the tree in a state neither
   version would produce, and "killed" the mutant for the wrong reason. Read
   what the mutated file actually says before believing the failure.

### Consumers

Agnostic (Python platform), daimon (agent orchestration), joshua (NPC AI),
kiran (game AI) — none consuming the Cyrius line yet.
