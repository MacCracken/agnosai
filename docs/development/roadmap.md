# AgnosAI — Roadmap

> **This file is forward-facing only**: open and planned work. What shipped lives in
> [`CHANGELOG.md`](../../CHANGELOG.md), one entry per release; why a design was chosen lives in
> the [ADRs](../adr/). What is true right now — versions, pins, test counts, sizes — lives in
> [`state.md`](state.md). [`cyrius-port-plan.md`](cyrius-port-plan.md) is the port's reasoning
> archive (the numbered-blocker table). When an item ships it leaves this file: completed items
> are removed, not ticked or struck through.

## What agnosai is

Provider-agnostic agent orchestration in Cyrius — crews, tasks, tools, delegation — ported from
the Rust line (`rust-old/`), at parity with it and now past it (*Owed work*, section F). agnosai
is a library with a binary (design principle 5), and every deliberate divergence from the Rust
line's wire carries an ADR.

**The one carve-out, a dependency fact rather than a scope call:**

| Not ported | Why |
|---|---|
| bhava / `personality` | **bhava itself has not been ported to Cyrius yet**, so there is nothing to depend on. The wire keeps emitting `null`, which is what the default Rust build emits. Deferred, never dropped: revisit the moment bhava lands in Cyrius. The work is **B5**; the decision is **D2** under *Settled decisions*. This is the only item the user ever set aside. |

## Design principles

1. **Wire compatibility** — the REST/MCP/A2A surface matches the Rust line's (`rust-old/`; after
   its deletion, `git show 2.1.0:rust-old/`). Every deliberate divergence carries an ADR.
2. **Sandbox by default** — untrusted code never runs unsandboxed
3. **Single binary** — no container orchestration for single-node deployments
4. **Concurrency via `sandhi_server_run_pooled`** — follow daimon, not hoosh (see port plan, "The concurrency decision")
5. **Library first** — agnosai is a library with a binary, not a framework
6. **Lockstep with ai-hwaccel** — aligned versioning, shared practices, same CI rigor

## Performance targets

Carried from the Rust line as *goals*, not as comparisons — the Cyrius baseline
starts fresh (see CLAUDE.md).

| Metric | Target |
|--------|--------|
| Boot to ready | <2s |
| Memory (idle) | <100 MB |
| Crew creation | <10ms |
| Concurrent crews | 100+ |
| Fleet msg overhead | <1ms |

## Release plan

Which release takes which owed item. The items themselves, with their evidence and what "done" means, are
under *Owed work*; this section only sequences them. One minor line per theme, patch releases inside a
line as each piece lands. Reorder freely: nothing in *Owed work* blocks anything else unless its row says so.

### 2.2.3 — P(-1) audit and hardening after the language bumps

**Why now.** Three releases moved the toolchain 6.6.14 → 6.6.20 → 6.7.9 → 6.7.18 and every dep with it (2.1.8,
2.2.0, 2.2.1). Each move was read for what it changes, but nothing has reviewed the whole tree since the 2.1.0
audit. Two of 2.2.1's fixes are the class of defect such a move leaves behind *without failing anything*. A
sigil change turned `tests/server_auth_lane_race.tcyr`'s pinned arm into a copy of its control. And
`tests/sandbox_cx.tcyr`'s guest tests had never run in CI: the 6.7.9 tarball shipped no `cycc_cx`.

**What.** `CLAUDE.md`'s P(-1), steps 0–9, over the whole tree: read, test and benchmark sweep, cleanliness,
baseline benchmarks, internal deep review, external research, clean again, tests from the findings,
post-review benchmarks, and repeat if heavy. The inputs known going in:

- **Tests that can pass without testing.** Sweep every guard that returns before asserting (`_t_*_have_*`,
  an early `return 0` ahead of assertions) and every test leaning on a stdlib or dep API whose semantics
  moved in the bumps. Each must either run or assert the arm it took, as `sandbox_cx` does since 2.2.1.
- **Toolchain capabilities not adopted:**
  - `[test] stdlib` (6.7.6): move `assert` and `bench` out of `[deps] stdlib`, so `dist/agnosai.deps` stops
    publishing `assert` to consumers.
  - Drop `hashmap_fast`, which has no caller.
  - Declare `sys`: bote-core 3.8.11's and sigil's sidecars need it, and today each bundle includes it itself.
  - `CYRIUS_DCE=1`: native 5.83 MB → 2.90 MB. Adopt only after the full suite and a smoke run pass on the
    compacted binary.
  - `--locked` in CI's Lockfile step, so a drifted lock fails instead of printing a notice.
  - `sandhi_server_last_failure`, for serve-failure diagnostics.
  - `[embed]` for the presets (*Recorded by cyrius 6.6.19*, below).
- **Leaks and stale code the dep readers noticed:**
  - `agnosai_rate_limit_total_evicted` (`src/server/rate_limit.cyr`) never frees its stats block; use
    `_agnosai_rl_stat`.
  - `src/fleet/relay.cyr` says `relay_stats` has no free. majra 2.11.0 added `relay_stats_free`.
- **The JSON parse cost.** bayan 1.11.1 made parsing 38–66% slower (CHANGELOG 2.2.1 *Performance*). Profile
  `crew_from_json` and `builtin_presets` for anything agnosai-side, such as a double parse. File upstream with
  the numbers if the cost is bayan's alone.
- **Benchmarks that measure their neighbours:** B18, B20, B27. Make the harness trustworthy before 2.3 changes
  the hot paths.
- **sigil's lane pool under agnosai's thread counts** (*C*, sigil): probe outbound TLS with more than 64
  concurrent verifies.
- **The threat model** (`docs/development/threat-model.md`) still describes the Rust build: axum,
  `Arc<RwLock>`, the `fleet` feature. Refresh it against the Cyrius tree and the `CYRIUS-2026-NNNN` ids.
- **Stale line citations.** `lib/<file>:<line>` references in `src/` comments drift with every fold; 2.2.2
  fixed one in `src/server/auth.cyr`. Sweep them, or cite function names.
- **B16's raise-ready gaps** (sigil, sandhi, kavach): draft the upstream filings, each with its repro.
  The kavach WASM stdin deadlock first.
- B25 and B32, which the audit's test sweep reaches anyway.

**Done means** step 6 clean, step 8's numbers in `bench-history.csv`, and every finding fixed or moved to a
row under *Owed work* with a measure.

### 2.3.x — the tool-calling harness (agnostic's M6 waits on it)

- **F1**, the tool-calling loop, with its ADR. This is the core of the line.
- **B33**, models from hoosh's catalog. Every agnostic crew takes the guessed-id path today.
- The tracing items that hang on F1: **B7** (8) tool-call arguments and results, and (9) the `delegate` tool's
  unlinked trace.
- The **`rust-old/` deletion** at 2.3.0, if the user OKs it (its section below).

### 2.4.x — the orchestration contract

- **F3**: the approval gate, and the crew and task lifecycle as a state machine with a per-crew `seq`.
- **F5** with **B6**: budgets and caps enforced, and a cap on concurrent crews.
- **F9** (compressed hand-off), then **F2** (hierarchical), which needs F9.

### 2.5.x — durable execution

- **F4**: an event-sourced crew log, resumable from the last completed task. It builds on F3's `seq` stream,
  and B15's `StateStore` goes with it.

### 2.6.x — protocol and server currency

- **F8**: MCP 2026-07-28 through bote, and A2A v1.0.x.
- **B8**'s server edges: the SSE stream noticing a departed client, draining in-flight requests, path
  percent-decoding, certificate-format JWT keys, and `hot_config` notification.
- The rest of **B7**, the tracing export.

### Any release — quality work that fits beside the above

B9–B15, plus B18, B20 and B27 if 2.2.3 does not close them. Each is Easy or Medium and self-contained.

## Deleting `rust-old/` — owed since the release after 2.1.0; proposed for 2.3.0

None of 2.1.1 to 2.2.2 carried the deletion. The deletion itself needs the user's OK; the *Release plan*
proposes 2.3.0, the first minor after the 2.2.3 audit.

**The 2.1.0 audit (2026-09-26)** read every item and every `#[test]` in `rust-old/` against
the Cyrius tree — seven module groups plus everything outside `src/`. Every `.rs` file has a
Cyrius counterpart; nothing outside `rust-old/` reads it at build, test, bench, fuzz or release
time. What it found that was easy is fixed in 2.1.0 (see the CHANGELOG); the rest is owed work
**B4–B16** under *Owed work* below. Every Rust spec stays reachable after deletion:
`git show 2.1.0:rust-old/src/<path>`.

- [ ] `git rm -r rust-old`, then delete the ignored `rust-old/target/` (~13 GB) from disk — `git
      rm` leaves it.
- [ ] Update the text that describes it: CLAUDE.md (identity line, layout section, the DO NOT
      rule — the user's file; 2.2.2 corrected its other stale facts under B23, so only the `rust-old/` lines remain), `CONTRIBUTING.md:47`,
      `docs/architecture/overview.md:34,53`, and README.md's note (`:8-11`), tree line (`:42`)
      and `:239`.
- [ ] `scripts/gen-presets.sh:124` writes a `rust-old/` comment into
      `src/definitions/presets_data.cyr` — edit it and regenerate that file together
      (`check-clean.sh` runs `--check` on it), then `cyrius distlib --all`.
- [ ] Prune `.gitignore`, which is still the Rust-era file: `/target`, `**/target/`,
      `**/*.rs.bk`, `criterion/`, `tarpaulin-report.*`, `.cargo/`, and a `Cargo.lock` comment
      naming `agnosai-server`. Drop what only `rust-old/` needed; `sdk/agnosai-tool-sdk/` is still
      a Rust crate (see *Settled decisions*), so keep what it needs.

The ~650 comment lines that cite a `rust-old/` path (`# Origin:` headers, test section labels)
stay as historical pointers; nothing breaks if they dangle.

## Owed work

**Status as of 2.2.2 (2026-11-05). Nothing here blocks anything; the *Release plan* says which release takes what.** Every remaining item
is work we owe ourselves — quality, coverage, or a decision — and any of it can
be picked up in any order. Each entry is self-contained: file paths, measured
numbers, and what "done" means, so it can be started without reading the session
that found it.

> **On the word "blocker".** This section was *"Open blockers and owed work"*
> through M6, when the distinction mattered: some items genuinely stopped the
> next bite from starting. **All eight of the port plan's numbered blockers are
> closed**, and so is the last thing that blocked a milestone. The numbered
> references still scattered through `src/` comments (*"blocker #3's arena"*,
> *"blocker #4's second consumer"*) are **historical citations, not open
> issues** — they name the analysis that produced a design, and the port plan's
> blocker table is where that analysis lives.

Nothing here is a discovery in progress — this is the complete list. **If an item
is not on it, it is not owed.**

Completed items are removed rather than struck through; what shipped is in `CHANGELOG.md`.
Labels are never reused, because `state.md`, ADRs and `src/` comments cite them: there is no
section A (it blocked M6 and was retired 2026-08-03), and B1–B4, B17, B19, B21–B24, B26, B28–B31, C2, F6 and F7 are closed (B23, B30 and B31 at 2.2.2; CHANGELOG 2.2.2).
B2's allocator-threading traps, which `src/` and `tests/` comments still cite as "roadmap B2", are
in `state.md` (*Test-design decisions worth not re-deriving*, *Standing rules a new session should
not re-derive*) and CHANGELOG [2.0.0].
Rows below still name F6 (the GenAI spans, [ADR 023](../adr/023-genai-semconv-spans-and-w3c-trace-context.md))
as where they were found; F7's non-goal is under *Settled decisions*.

### B. Owed — flagged in earlier bites, never done

| # | Item | Effort | Notes |
|---|------|--------|-------|
| B5 | **Agent personality (bhava)** | Large, blocked | `personality`, `with_personality`, `mood_adjusted_temperature`, the personality score and the system-prompt block. `../bhava` has no Cyrius port, so nothing can supply a profile. Unblocked by porting bhava (a separate repo). See *What agnosai is* and D2. |
| B6 | **Crew concurrency** | Medium | No cap on concurrent crews: `submit_crew` spawns a thread per call, where the Rust tree held a semaphore of `max_concurrent_tasks` (default 10). Parallel mode runs fixed batches, so one slow task idles the other slots until its batch ends. Both want a counting semaphore, which the stdlib does not have. |
| B7 | **Tracing export** | Large | The Rust tree exported every `#[tracing::instrument]` span over OTLP (20 of them: sandbox, routes, crew runner, orchestrator, scoring, cost planning); agnosai exports the four GenAI semconv operations (`invoke_workflow`, `invoke_agent`, `chat`, `execute_tool`), linked into one trace and joining an inbound `traceparent` since 2.1.5 ([ADR 023](../adr/023-genai-semconv-spans-and-w3c-trace-context.md)). **What remains:** (1) the other `#[instrument]` sites as spans, under the same explicit span-context threading (no thread-local: ADR 023 records why) — the routes' HTTP server spans can use `agnosai_http_server_span_a` (2.1.7, ADR 024), which an embedding consumer already records through; (2) trace and span ids in JSON log lines, and per-module targets, which needs sakshi's log hook to expose them; (3) aggregate usage on `invoke_agent` (the sum of its `chat` spans' tokens); (4) request parameters on `chat` (`gen_ai.request.temperature`, `max_tokens`, declared and never set); (5) `gen_ai.response.finish_reasons`, a `string[]` that needs an OTLP `arrayValue` the encoder does not write; (6) `Span.flags` with the W3C is-remote bit for a span whose parent arrived in a header; (7) `tracestate` passthrough; (8) tool-call arguments and results on `execute_tool`, which need F1; (9) **the `delegate` tool starts an unlinked trace** (found 2026-10-03, F6 review): `src/tools/builtin/delegate.cyr:78-83` builds the delegated spec and calls `agnosai_orchestrator_submit_crew` with no trace parent, because the tool vtable carries no span context. **Repro:** a deployment that calls `agnosai_register_delegate_builtin`, with the exporter on, sends `/mcp` `tools/call` `delegate` with a `traceparent`: the `execute_tool delegate` span joins the header's trace, the delegated crew's `invoke_workflow delegate:<agent>` span is a new root. The pinned semconv (`gen-ai-agent-spans.md`, *Invoke workflow span*) also says an `invoke_workflow` span SHOULD NOT be reported when an agent or tool spins up a workflow to delegate to a sub-agent. **Fix direction:** once a tool call has a span context (F1, or a context on the vtable), format it with `agnosai_otlp_span_context_format_a` and stamp it on the spec through `agnosai_crew_with_trace_parent`, and decide whether a delegated crew emits `invoke_workflow` or only its `invoke_agent` spans; (10) **an operator switch for an untrusted inbound context** (found 2026-10-03, F6 review): any caller of the three traced routes picks the trace id agnosai's spans join and can turn their export off with sampled flag `0` (`src/telemetry/mod.cyr`, `agnosai_telemetry_record_span_in`: `if (agnosai_otlp_span_context_sampled(sc) == 0) { return 0; }`; pinned by `tests/server_serve.tcyr`'s unsampled-header check). Recorded as surface 7 in `threat-model.md` and in ADR 023. **Fix direction:** an env switch read at boot, e.g. `AGNOSAI_OTEL_INBOUND=honour|restart|record` — `restart` ignores the header (new root, the inbound id at most as a span link), `record` keeps the trace but samples regardless of the flag. |
| B8 | **Server edges** | Medium each | An SSE stream holds one of the 100 pool workers for the crew's life and does not notice a departed client (since the 6.6.20 pin, 2.1.8, sandhi 1.10.8's chunked verbs return the write result, but `src/server/routes/sse.cyr` discards all six: lines 94, 154, 157, 180, 249 and 257; see *C*, sandhi); shutdown does not drain in-flight requests; chunked request bodies are 501 (sandhi has no decoder); path parameters are not percent-decoded; a certificate-format JWT key (and a key over 4096 bits) is refused; `hot_config` has no change notification. |
| B9 | **WASM** | Medium | A module with no `_start` export runs and "succeeds"; a module is validated by its header only, so a malformed body fails at execute; the Rust store limits (10 instances/tables/memories, trap on failed grow) and stdin over 64 KiB need kavach options; WASM tools held by a registry never free their staged module (`agnosai_wasm_module_free` exists; the registry has no destroy hook). The execute path runs the `wasmtime` CLI through kavach's WASM backend ([ADR 019](../adr/019-wasm-tools-spawn-wasmtime-directly.md); the filename names the reversed draft). **Found at 2.2.2 (B16):** a WASM tool that echoes its input as it reads can hang its worker for good when the input passes roughly two pipe buffers (about 100–200 KB), because kavach's capture writes stdin before it drains stdout and its deadline never fires there. Until kavach fixes it, agnosai could refuse tool input over 64 KiB, or stage it in a file. |
| B10 | **JSON forms nothing calls** | Easy each | Serde shapes the Rust types derived but no Rust code serialised: fleet `NodeInfo`/`NodeStatus`/`NodeTopology`/`DeviceLink`/`InterconnectType`/`FederationRole`/`ClusterStatus`/`ClusterInfo`/`RelayMessage`; `RetryConfig`; `TaskProfile`, `BudgetExceeded`, `CachedPlan` and `ToolSchema`/`ToolOutput` parsers; `ToolInput` and the manifests' writers; `KavachToolResult`. |
| B11 | **Structured-log fields** | Easy each | Log events that lost their fields or are absent: fleet (17, federation's nine above all), wasm (no sakshi calls at all), kavach bridge, process/OCI exit lines, wasm loader and remote registry error details, the MCP tool-call line's `success`/`duration_ms`, a2a rejections' id prefix and `metadata_bytes`, `latency_ms` in task-result metadata. |
| B12 | **Behaviours no suite asserts yet** | Easy each | Orchestrator: diamond DAGs (sort, ready set, a run), parallel/DAG `task_ms`, event-bus delivery, the cancel audit entry, per-task audit level, a 64 KiB+ IPC echo, and **concurrent cancel** — mid-execution interruption and parallel/DAG cancel stress (`tests/orch_crew_runner.tcyr:1382-1408` cancels only before a run starts; the Rust tree never tested it either, see *Carried over from the Rust line*). Core: `Cpu` → `"cpu"`, multi-device filtering and sums, a negative non-zero cost. Sandbox: real fuel exhaustion (-2), a real timeout, empty stdin to a reading guest, the Python bridge's `"error": null`. Tools: AGNOS schema names/descriptions/parameter types and missing-parameter errors. Server: output-filter pattern text, `http://[::1]:8080`, the echo tool's parameter type. |
| B13 | **Network-facing tools** | Medium | The load tester has no connection pool (every request is a fresh handshake); its success path and the security audit's live sequence have no test, since no suite stands up a loopback server. |
| B14 | **Seven benchmarks** | Easy–medium | The sandbox execute path (process argv, stdin, shell; WASM hello) and `audit_recent(10/100/1000)`. |
| B15 | **Smaller gaps** | Easy each | Output-filter leak detection folds ASCII case only; a pub/sub subscriber cannot detach alone (only `unsubscribe_all(pattern)`); `StateStore` (pluggable persistence) is not ported; `strcase` is ASCII-only though the crew assembler matches user-supplied names. |
| B16 | **Upstream gaps — re-checked at 2.2.2 against each project's latest release; three are ready to raise** | Small each | The rule was: raise none until agnosai is on that project's latest version, then re-check first. 2.2.1 met it (cyrius 6.7.18's folds bayan 1.11.1, sigil 3.13.15 and sandhi 1.13.7, and kavach 3.14.2), and 2.2.2 re-checked each with a probe. None is filed upstream. **bayan:** escape decoding in quoted scalars is **closed** (since bayan 1.8.0). Block scalars, flow mappings, anchors/aliases and tags are still refused (`lib/bayan.cyr`'s `yaml … not supported` errors), but bayan's roadmap lists full YAML 1.2 under *Out of scope* and its ADR 0008 makes the refusals deliberate, so this is a scope request, not a bug report; raise it only if a definition file needs one. **sigil:** RSA keys over 4096 bits are still refused (`_rsa_recover_em`'s `n_len > 512`): an RSA-8192 key parses and then fails to verify. sigil's CHANGELOG 3.6.2 states the bound. Ready to raise. A certificate-format JWT key is agnosai's own to fix (B8). **sandhi:** a chunked request body is still 501. `sandhi_server_request_has_unsupported_te` refuses any `Transfer-Encoding`, and the pooled worker agnosai runs answers 501 before calling the handler. Ready to raise. **kavach:** (1) WASM store limits: the backend passes only `max-memory-size` and fuel to `wasmtime`, and exposes no option for `max-instances`, `max-tables`, `max-table-elements`, `max-memories` or `trap-on-grow-failure`, all of which wasmtime 50 offers. Ready to raise. (2) ⚠ **Not a 64 KiB limit but a deadlock:** `confine_capture_input` writes the whole stdin with one blocking write before it starts draining stdout. A guest that reads all its input and then writes works at 64 KiB, 100 KB and 200 KB (1 MiB is cut by the 1 MiB stdout cap). A guest that **writes while it reads** hung at 200 KB with kavach blocked on stdin and wasmtime blocked on stdout, and kavach's own timeout never fired. kavach's CHANGELOG 3.11.8 says to pass a large input by file, but nothing refuses it. Raise first; meanwhile see B9. |
| B18 | **`lt_aggregate_100k_500workers` is ~2x its 2026-08 band** | Medium | Found at 2.0.6 (88.2 ms against ~24 ms), 45.4 ms at 2.1.3 on cyrius 6.6.14, and still open. The measurements, what is ruled out (not the compiler, not the benchmarked source, not noise) and where to start (the 2026-08-03 `_a` allocator work, a ~6-round bisect over 40 commits) are in `state.md`, "⚠ OPEN — a ~3.6x regression, found at 2.0.6". |
| B20 | **A second live thread moves a crew's cost — both ways** (found 2026-10-03, F6 fix round) | Medium | Measured with one process per row, seven interleaved rounds, medians: an idle detached thread that only `sleep_ms(25)`s took `run_crew_10_tasks_sequential` **334 → 400 µs (+20%)** and `run_crew_10_tasks_parallel_4` **624 → 456 µs (−27%)**, at 2.1.4 and on the then-current tree alike. `benches/orch.bcyr` already runs those rows at the second-thread level (parallel ~456 µs; sequential 380–400 µs across two gate runs), so some earlier row leaves a thread alive, which is also the likeliest reading of the `crew_runner_10_tasks_parallel_4_quiet` level shift recorded under CHANGELOG 2.1.5 → Performance. **Repro:** add a mode to a copy of `benches/orch.bcyr` that calls `thread_create_detached` on a `while (!stop) { sleep_ms(25); }` body before `_b_run_crew("run_crew_10_tasks_parallel_4", ...)`, and run it beside the plain row in separate processes. **Why it matters:** a production server always has other threads (the pool, the exporter), so the single-threaded bench level is not what a deployment sees, and a row's number depends on what ran before it. **Direction:** find which stdlib path switches on a second live thread (mutex/futex, the allocator, `thread_create`/`thread_join`, stack mmap) — if it is the stdlib, file it against cyrius rather than patch it — and then decide whether `benches/orch.bcyr` should pin the level explicitly (start one idle thread in `main`) so rows stop depending on their neighbours. Related: **B27**, the same file's rows moved by heap position. |
| B25 | **The serve handler's arena read of `traceparent` is not covered by a test** (found 2026-10-03, F6) | Medium | `sandhi_server_find_header_a(…, "traceparent")` runs only on the arena branch, which needs a sandhi worker arena. The pipe harness reaches only the bare branch, as it does for every other header. Fix: a harness that runs one request through a real pooled worker with an arena, asserting that the crew's spans carry the inbound trace id. That would cover the other arena-branch header reads too. |
| B27 | **`crew_runner_10_tasks_parallel_4_{quiet,watched}` measure the global heap's position, not the runner** (found 2026-10-04, at 2.1.6) | Easy–medium | In `benches/orch.bcyr`, with the traced block before them (`run_crew_10_tasks_*_traced`, which starts and stops an OTLP exporter) the 2.1.5 tag reads ~420 µs; with that block removed, ~507 µs — and 2.1.6, whose exporter no longer leaks there, reads ~505 µs either way. Interleaved A/B builds ruled out job false sharing, the data-section layout and the exporter's timeouts. At 2.1.8 (cyrius 6.6.20) the rows read 578 → 433 µs (quiet) interleaved against a 2.1.7 build. The runner makes no channel call, but the benches before it now run faster (6.6.16's channels), so this too is not claimed. **Fix:** run these rows first, or in their own `.bcyr`, so a change elsewhere in the file cannot move them; then rebase their history. Related: **B20**. |
| B32 | **A WASM tool's staged module outlives the tool, and staging and nine suites ignore `TMPDIR`** (found 2026-10-08, at 2.1.7) | Medium | `agnosai_wasm_load_module` stages every module at `/tmp/agnosai-wasm-<uuid>.wasm` (`src/sandbox/wasm.cyr:192`), because wasmtime runs it from a path. `agnosai_wasm_module_free` removes it, but nothing calls it outside the suites: `agnosai_tool_registry_remove` only drops the map entry, so `DELETE /api/v1/tools/{name}` of a WASM tool, or re-registering one under the same name, leaves its file for the life of the host. Freeing on removal is not enough by itself: another thread may have looked the tool up and not yet started wasmtime on the path, so the free must wait for in-flight executions (a reference count on the tool, or unlink after wasmtime has opened it). Stage under `$TMPDIR` (`/tmp` when unset). The suites: nine write fixed `/tmp` paths — `server_routes_sse`, `sandbox_oci` and `sandbox_manager` never unlink theirs; `orch_durable_state`, `definitions_packaging`, `definitions_loader`, `sandbox_wasm`, `orch_ipc` and `tools_wasm` unlink some — so two runs at once collide, and two full runs at 2.1.7 left 28 staged modules and nine other files behind. agnostic 0.1.15 fixed the same thing in its suites with a shared `tests/support/tmp.cyr`. |
| B33 | **Choose and attribute models from hoosh's catalog, not from names agnosai carries** (found 2026-10-09, thoth 0.52.7 / hoosh 2.8.0) | Medium | agnosai names models itself in four places. (1) **Selection.** An agent with no `llm_model` gets `agnosai_default_model(agnosai_route(profile))` (`_agnosai_crew_select_model`, `src/orchestrator/crew_runner.cyr:301`, called at `:706` and on the validation retry at `:744`; `src/llm/router.cyr:189-193`): `llama3:70b` for Capable, `llama3:405b` for Premium, and `llama3:70b` with no agent at all. hoosh routes by the id it is given, so a gateway with no `llama*` route answers every such crew 404 ("no provider for model"), and one with an Ollama route sends it to a local 70B model whether the box has one or not. Every agnostic crew takes this path: none of its presets sets `llm_model`. (2) **Attribution.** When a response carries no `usage.provider` (`crew_runner.cyr:646-653`), `_agnosai_crew_infer_provider` (`:160`) guesses from nine name prefixes, so `o4-mini` (pinned, `tests/orch_crew_runner.tcyr:528`), `gpt-oss`, `qwen/…` and every newer family bill to Ollama. Until hoosh 2.8.0 a *streamed* response carried no usage at all, so every streamed crew took this path. (3) **`fleet/cost_planning.cyr`**'s `BUDGET_MODELS` and `agnosai_select_cheapest_model` (`:42`, `:242`, `:283`): the oracle's model and price list, a public library API that nothing in `src/` calls. (4) **Docs**: README.md:178 and `docs/architecture/overview.md:146` show `"llm_model": "capable"`, which is sent literally (hoosh has no tier aliases), and `scripts/stack.sh:35` defaults `AGNOSAI_MODEL` to `llama3` (its comment at `:34` names `claude-opus-4-8`). It also **rewrites the id an agent names**: `_agnosai_crew_strip_provider_prefix` (`crew_runner.cyr:274`, segments at `:243`) drops a leading `openai/`, `anthropic/`, `deepseek/` and nine more, for the oracle's litellm-style ids. Some providers' real ids carry exactly that prefix (Groq serves `openai/gpt-oss-120b`; OpenRouter's ids are `anthropic/…` and `openai/…`), so an id picked from the catalog reaches hoosh renamed, and hoosh routes the bare name to another provider or to none. **What hoosh 2.8.0 offers** (hoosh ADR 012): `GET /v1/models/catalog` lists what each provider serves now, asked of each route live and refreshed every six hours. Each entry is `{id, owned_by, source, routes}` plus, when known, `display_name`, `context_window`, `max_output_tokens`, `capabilities` (`vision`, `thinking`, `effort`) and `pricing` (`input_micro_usd_per_1k`, `output_micro_usd_per_1k`). A stream now ends with a usage chunk carrying `provider` and `cost_micro_usd`. agnosai already has `agnosai_hoosh_models_path` and `agnosai_hoosh_extract_models` (`src/llm/hoosh.cyr:101`, `:557`), exercised by the tests only. **The work:** (a) read the catalog (`/v1/models/catalog`, falling back to `/v1/models` on a 404) once per process and again when a selection fails, into process-lifetime storage, not a request arena; (b) map the router's tier onto it — the catalog carries prices and capabilities but no tier, so either rank the entries whose `capabilities` fit the task by `pricing`, or ask hoosh's `POST /v1/cost/recommend` once it ranks the live catalog (today it ranks only routes' literal patterns, `handle_cost_recommend`; recorded on hoosh's roadmap 2026-10-09) — with an operator override per tier, the choice made in this item's ADR; (c) attribute a response with no `usage.provider` to its catalog entry's `owned_by`, and retire the prefix needles; (d) with no catalog (hoosh unreachable, or an empty list), fail the crew with a named error rather than send a guessed id; (e) price `BUDGET_MODELS` from the catalog's `pricing`, or drop the oracle list — the same ADR decides; (f) fix the two docs and `stack.sh`; (g) send an id the catalog lists exactly as listed, and strip a `provider/` prefix only from one it does not (or retire the strip; the same ADR decides). **Tests:** a canned catalog body, selection per tier, attribution from `owned_by`, the no-catalog error, a listed `openai/…` id sent unchanged; the `llama3:*` assertions (`tests/llm_router.tcyr:84,86`, `tests/orch_crew_runner.tcyr:699`) move with the change. agnostic waits on this (its roadmap, *Waiting on agnosai*). |

### C. Upstream — filed and waiting

Nothing here blocks agnosai today; each is a residual agnosai measured and handed off, or one
still to hand off.

**To file (found 2026-10-04, at 2.1.6):** an `_a` HTTP request still allocates on the global bump.
With the exporter and the chat call both posting through `sandhi_http_post_opts_a` into their own
arenas, what remains on the no-free global bump is sandhi's own: **480 B per successful POST** and
**~120 B per refused connect**, measured with `alloc_used()` around 100 posts to a local stub. 16 B
of it is `sockaddr_in` (`lib/net.cyr:142-143`, a bare `alloc(16)` on every connect); the rest is in
sandhi's dispatch and connect path, not yet located. At one exporter batch a second that is ~41 MB a
day. **The ask:** an `_a` form of `sockaddr_in` (or a caller-supplied buffer), and sandhi's
`_sandhi_http_dispatch_a` keeping every allocation in `a`. Pinned meanwhile by bounds, not zero:
`tests/telemetry_otlp.tcyr` (< 512 B a POST) and `tests/llm_hoosh.tcyr` (< 1 KiB a call).

| Dep | Open |
|---|---|
| sandhi | **Fixed upstream, reached at 2.1.8:** `2026-10-03-chunked-response-verbs-discard-send-result.md` — sandhi 1.10.5 made the chunked verbs (and 1.10.8 the one-shot verbs) return the write result, and the cyrius 6.6.20 fold agnosai pins carries 1.10.8 (`lib/sandhi.cyr:13715-13757`). agnosai's half is open: `src/server/routes/sse.cyr` discards the result (B8). **Not filed:** `backlog` is silently ignored by `run_opts`/`run_async` (only `run_pooled`, `run_tls` and `run_pooled_tls` read it); the chunked start hardcodes `" OK"`; **inbound** chunked decoding is unsupported (answers 501 — honest, but not support; B8, B16). |
| sigil | **The lane bound changed at sigil 3.13.14, so re-probe it in the 2.2.3 audit.** Through 3.13.13 a lane was drawn from a never-decremented counter, so past 63 lifetime crypto threads two live threads could share one; agnosai passes that with its 100 pool workers plus a thread per crew. Since 3.13.14 (agnosai's pin carries 3.13.15) lanes are pooled, and no two live threads ever hold one (CYRIUS-2026-0040). The 64 lanes are taken only while a lane-scoped operation runs, and with none free it **refuses**. Two kinds of refusal: the scoped entry points (ECDSA P-256/P-384 verify, the Poly1305 stream) return failure, but a direct call into the variable-time EC arithmetic outside a scope takes a lane for the thread's life and, with none free, **ends the process (exit 70)**. A thread that retires holding a sticky lane keeps it out of the pool. Read at 2.2.2, not probed: agnosai's outbound native TLS reaches only the scoped kind (`ecdsa_p256_verify_der` and `ecdsa_p384_verify` are scoped; ECDH goes through the constant-time `_ect_*` path, which keeps no lane state; hashing and HMAC are function-scope). So the worst case there is a failed handshake when more than 64 verifies run at once, never a shared lane or an exit. **Next step (2.2.3):** probe concurrent outbound TLS past 64 threads and confirm no path reaches a sticky claim. `tests/server_auth_lane_race.tcyr` pins the RSA verify's lane-freedom with `_crypto_lane_alias` since 2.2.1. |
| hoosh | **To file (found 2026-10-03, F6).** hoosh 2.7.1 extracts an inbound `traceparent` strictly (`src/lib/trace.cyr:99-141`, `src/main.cyr:124-127`), but exports its own server span with the inbound **parent-id as its own spanId** and no `parentSpanId` (`src/lib/otlp.cyr:72-77`), then forwards the same traceparent to providers. **The ask:** mint a fresh span id for hoosh's span, emit the inbound parent-id as its `parentSpanId`, and forward a traceparent carrying hoosh's OWN span id. **Then agnosai's half:** `agnosai_hoosh_chat_in` adds an outbound `traceparent` header built from its `chat` span's context (`agnosai_otlp_span_context_format_a` already exists), and hoosh's span becomes the chat span's child. Until hoosh changes, sending the header would give hoosh's span the same id as agnosai's `chat` span inside one trace, so agnosai deliberately sends none ([ADR 023](../adr/023-genai-semconv-spans-and-w3c-trace-context.md)). Not filed from this lane: the upstream filing is a separate step. **Recorded on hoosh's roadmap (2026-10-09, for B33):** `POST /v1/cost/recommend` ranks only routes' exact-model patterns (`handle_cost_recommend`, which skips every pattern ending in `*`), not the live catalog hoosh 2.8.0 keeps, so a gateway whose routes are all `claude-*`-style answers it 404 ("no capable exact-model route"). B33 can map tiers without it; the ADR picks. |
| bote | **To file (found 2026-10-07).** The client half of MCP: sending `initialize`, `tools/list` and `tools/call` to a server named in bote's `HostRegistry`, whose entries can already declare a `tools/call` capability behind its SSRF guard. F1's registry needs it for tools from external MCP servers (F1, harness rule 4). bote 3.3.17 (read at 2.1.8) was the server half only: dispatch, six transports, sessions. Re-check against 3.8.11, the pin since 2.2.1, before filing. agnostic records the same ask (its `roadmap.md`, *Upstream*). |

### D. Decisions deferred to a human

> ⚠ **Before adding anything here, or re-raising anything in it: a decision the
> user has ALREADY MADE does not belong in this table.** D2 sat here for weeks
> after bhava was settled and got asked again as a result, which is a documented
> way to waste the user's time. When an item is decided, move it to *Settled
> decisions* with the decision written out — do not leave it phrased as a question.
> D1–D6 are there.

| # | Decision | Where it stands |
|---|---|---|
| D7 | **Note the 2.1.5 exception in the CHANGELOG, or change its policy line?** | **Open — the user's call.** `CHANGELOG.md:6` says agnosai adheres to Semantic Versioning, but 2.1.5 shipped a `### Breaking` section — the F6 span vocabulary and the genai library API (`agnosai_genai_inference_span` → `agnosai_genai_chat_span` and the rest) — under a patch number (D6), and neither the policy line nor the 2.1.5 entry says so. The options: note the exception in the 2.1.5 entry, or change the policy line. |

### E. Known-unreachable code kept for oracle shape

Not defects and not owed — recorded so nobody re-derives them as findings. Each is
documented in place as unreachable rather than implied to fire: `agents.rs`'s
serialize skip, the cycle-detector's `== 2` memoization arm, `crews.rs`'s profile
skip, and `crew_runner.rs`'s personality prompt block (bhava; see *What agnosai is*).

The entries below came from the 2026-08-04 sandbox audit
([`m7-audit-2026-08-04.md`](m7-audit-2026-08-04.md)) and the fleet and telemetry ports. Each
was reached and correctly rated not a defect; "no test covers this" is true of each and will
stay true:

- **`kavach_bridge`'s "start failed" arm.** kavach's `valid_transition` accepts
  `CREATED -> RUNNING` unconditionally, so a freshly created sandbox always
  starts. Kept because the oracle has the arm. It is the one of the three error
  paths that *can* carry a kavach error code, and now does.
- **`fleet/gpu`'s two `saturating_sub` guards (added 2026-08-08).** `used` moves
  only by `allocate` (+) and `release` (-), and `release` removes the record
  before subtracting, so `used` is always >= any single record and the guard
  cannot fire. The oracle needs it because Rust's `u64` underflow panics in
  debug. Mutation-verified as unreachable: deleting the guard in `release`
  leaves all 65 assertions green. Kept for oracle shape, documented in place.
- **`fleet/federation`'s `elect_by_lowest_id` `None` arm (added 2026-08-08).**
  The candidate list is the online peers **plus this cluster's own id**, so it
  is never empty and `candidates.first()` is always `Some`. The port returns the
  winner and never 0; a lone cluster elects itself. Kept in the doc comment
  only, and asserted from the reachable side (`fleet_federation.tcyr` pins that
  a peerless manager still returns a winner).
- **`fleet/federation`'s strict `>` liveness comparison (added 2026-08-08).**
  `check_liveness` demotes on `elapsed > timeout` where `NodeRegistry`'s sweep
  uses `>=`. Discriminating the two needs elapsed to land *exactly* on the
  threshold, and the sweep reads its own `clock_now_ns()` after a test has set
  the instant, so the boundary is unreachable from outside. Both directions are
  asserted with a 1 ms margin; the strict comparison itself is documented in
  place, not claimed as tested. Not a defect — the difference from `registry` is
  the oracle's, and reproducing it is the point.
- **`telemetry/otlp`'s two-arena separation (added 2026-08-09).** The export
  ring keeps fragments and the batch document in different arenas so `drain`
  cannot reset the arena it is about to read from. Two mutants of this survive
  every assertion — allocating fragments into the document arena, and resetting
  the document arena after the build instead of before — because **a bump-arena
  reset only rewinds an offset**. The bytes stay intact, so use-after-reset
  reads correct data in a single-threaded test and the output is identical.
  A test pins that two arenas exist (killing the collapse-to-one
  simplification); the rest is correct by construction and by review, not by
  test. Do not "simplify" it on the evidence of a green suite.
- **`fleet/topology`'s `total_pairs == 0` guard (added 2026-08-08).** It is
  reached only when `gpu_count >= 2`, where `n * (n - 1) / 2 >= 1`, so it cannot
  fire. Kept because the oracle has it. Mutation-verified from the reachable
  side: the six other topology mutants all die, and this branch has no input
  that reaches it.
- **`fleet/federation`'s three unread config fields (added 2026-08-08).**
  `endpoint`, `seeds` and `election_timeout` are stored and never read by any
  method, exactly as upstream: the oracle logs `seeds.len()` once at startup and
  its `election_timeout` doc claims a randomization nothing performs. Accessors
  exist so a caller that owns the election timer has one place to configure it.
- **`fleet/state`'s `is_checkpointing` is observably always false.** `checkpoint()`
  sets it, pushes, and clears it before returning, and every oracle method takes
  `&mut self`, so no caller can observe it set. The field and both writes are
  ported anyway: the oracle's comment says it exists so barrier operations can be
  queued against it, a contract for a future concurrent caller rather than dead
  code (`src/fleet/state.cyr:26-32`).
- **`fleet/cost_planning`'s missing `model_throughput` `None` arm.** The oracle's
  `let Some(..) = model_throughput(..) else { return 0.0 }` is unreachable — every
  branch of `model_throughput` returns `Some`, including its fallthrough. There is no
  Cyrius branch because `_agnosai_cost_tier` is total; the oracle's `0.0` is documented
  in place rather than reproduced as a branch nothing can enter
  (`src/fleet/cost_planning.cyr:224-228`).
- **Two log-only fixes whose mutants survive — L5 and L9.** Nothing in this tree
  captures sakshi output, so `sakshi_warn`'s corrected length and the manager's
  restored dispatch `debug!` cannot be asserted. Both are correct; neither is
  verifiable. Said plainly in the audit document's Status section rather than
  counted as mutation-verified, and repeated here because a future sweep will
  otherwise flag them as untested code.

### F. Past parity — the multi-agent gaps (recorded 2026-10-03)

**Source:** a landscape review of herdr and the 2025–26 multi-agent field, written in agnostic as
`agnostic/docs/development/research/2026-10-03-herdr-and-multi-agent-landscape.md`. The
**harness rules** in F1–F3, F5 and F9 come from a comparison with
[turnstone](https://github.com/turnstonelabs/turnstone), a self-hosted, tool-using agent harness,
whose [`PRIMER.md`](https://github.com/turnstonelabs/turnstone/blob/dev/PRIMER.md) states them ("the
model proposes; the gate disposes"). Recorded 2026-10-07, and mirrored in agnostic's `roadmap.md`
(*Waiting on agnosai*), which holds agnosai to them.

**What the review found:** agnosai already has most of the pieces the field now treats as
baseline. Hierarchical delegation, the approval gate, budgets, `durable_state`, fleet and learning
are all **built and tested, but not called from `crew_runner`**. agnosai has **no tool-calling
loop** at all.

**These are past parity, not parity debt.** Checked against `rust-old/` on 2026-10-03:
- `crew_runner.rs:148` falls back from hierarchical to sequential.
- Nothing in the Rust orchestrator outside `approval.rs` calls the approval gate.

So design principle 1 (wire compatibility) is why **each item that changes behaviour the oracle
defined needs its own ADR** recording the divergence (as 007/009/010/019/020/021 do). The
order is by leverage.

| # | Item | Effort | Notes |
|---|------|--------|-------|
| F1 | **A tool-calling loop in `execute_task`** | Large | Model → `tool_calls` → execute via the registry and sandbox tier → append the result → model, until a final answer, a step cap or a budget. It is the core of every harness (Claude Agent SDK, Codex, OpenHands). Without it an agent is a single prompt and registered tools are never invoked. Verified 2026-10-03: no `tool_call` handling in `crew_runner.cyr` or `src/llm/`. hoosh's OpenAI-compatible `/v1/chat/completions` carries `tools` / `tool_calls`, so the seam exists. **agnostic M6 is gated on this.** Pair it with the tool-registry ownership question agnostic records in its `handoff.md`. ADR required. **Tracing ([ADR 023](../adr/023-genai-semconv-spans-and-w3c-trace-context.md)):** the loop must call `agnosai_tool_execute_in` with the task's `invoke_agent` span context, so tool spans nest under the agent that made them, and should pass the agent name onto the span. **Harness rules (2026-10-07; see *Source*), each for this ADR:** (1) **Journal every call before it runs, and give each result an effect status** — committed, none (it never started) or unknown (it may have run) — so whoever reads the record after a crash (agnostic's `interrupted` crews today, F4's resume later) can tell an unknown effect from none; turnstone also records `partial` and `rolled_back`. (2) **A tool's output is data, never instructions:** check each result for injected instructions before it reaches the model, and never let one change the plan, an agent's grants or its budget. agnostic is a QA product, so the system under test writes the tools' output and that output is untrusted by definition. turnstone's output guard is heuristic and only annotates; decide here whether a flagged result is held back. (3) **Approve the calls a model makes in one turn as a set:** two calls that each pass alone can leak together (read a secret, post to the web). turnstone's pipeline is a template: prepare each call in turn, approve the batch, run it in parallel, then check the results and append them in one step. (4) **Each agent sees only the tools granted to it, and the registry can hold tools from external MCP servers** through bote's client, which bote does not have yet (*C*, bote). turnstone's harness owns its registry, gives each role its own subset (interactive, coordinator, sub-agent) and merges a session's MCP tools in when it starts — a data point for the registry question above. |
| F2 | **Wire hierarchical (manager → workers)** | Medium | `orchestrator/hierarchical.cyr` is built and tested; `crew_runner.cyr:1647-1650` logs and falls back. Orchestrator-worker is the pattern the field converged on: Claude subagents, Codex subagents, manager Devins, the ADK supervisor, Magentic-One. Google/DeepMind's 2025 scaling study measured error amplification of 17.2× for independent agents against 4.4× centralized. Workers return summaries, not transcripts (see F9). The comment at `crew_runner.cyr:1580-1584` defers this "until it lands upstream". The upstream is the frozen `rust-old/`, so it never will, and this is the ADR that lifts that deferral. agnostic drops its `hierarchical` refusal on the re-pin. **Constraint from F7:** `agnosai_explain_selection_a` is exact only while selection is a pure function of roster and task, and consumers recompute it from the spec. F2, learning-driven selection or a stateful bhava personality ends that, so the run must then record its choice. **Authority only narrows (2026-10-07; see *Source*):** a worker holds at most its manager's tools and budget. A need beyond them goes up to a human; neither the manager nor the model can grant it. |
| F3 | **Wire the approval gate, and specify the crew/task lifecycle as a contract** | Medium | `orchestrator/approval.cyr` is built at `main.cyr:391` and its routes exist, but `crew_runner` never calls it. Add `awaiting_approval` to the task and crew states. Write the lifecycle down as one state machine: the states, the legal transitions, and a **per-crew monotonic `seq` on every event**. Test it as an invariant: no event without a legal transition, and no `seq` gap. Build on [ADR 022](../adr/022-crew-events-and-status-say-what-happened.md)'s event semantics rather than replacing them. It inherits one ordering gap ADR 022 records: a parallel/DAG batch reports `task_completed` in dispatch order, where the oracle's `join_next` reports completion order (bounded by the batch). It also inherits the error arm's `running` → `pending` revert (a cyclic DAG, or a dependency on a task the spec does not hold; `src/orchestrator/orchestrator.cyr:412-424`), which the state machine should replace with a terminal state. Since 2.1.6 a branch stranded by a failure already ends FAILED. The model is herdr's self-report contract (`--state … --seq N`) and its `blocked` roll-up. **A model judge may only tighten (2026-10-07; see *Source*):** if approvals gain an LLM judge, it may refuse what the rules allow and never allow what they refuse, and it gives its reasons from a fixed list, not free text. An automatic approval covers a whole batch or none of it, and anything uncertain goes to a human (turnstone's Smart Approvals). |
| F4 | **Durable execution — an event-sourced crew log, resumable from the last completed task** | Large | `orchestrator/durable_state.cyr` has **zero callers**, and B15 notes `StateStore` is unported. This is now baseline in the field: LangGraph checkpointers, Pydantic AI and the OpenAI Agents SDK on Temporal, MAF's Durable Task, OpenHands' event sourcing, Claude's resumable workflows. Build on F3's `seq` stream. Today agnostic answers a restart by marking crews `interrupted`; this is what would let it resume them. `durable_state` is built on `lib/io.cyr`, not patra (`src/orchestrator/durable_state.cyr:31-34`). An event-sourced log may want patra's jsonl mode, which opens `O_APPEND` without `O_TRUNC`; patra is first-party and can be extended upstream. |
| F5 | **Enforce budgets and caps** | Medium | `budget.cyr` and `multi_tenant.cyr` have no callers outside their own files; only `max_duration_secs` is read. Enforce token and cost per crew and per tenant, plus max tasks per crew. Pair with **B6** (concurrent crews, and the counting semaphore the stdlib lacks). Multi-agent runs about 15× chat tokens (Anthropic 2025); Claude Code caps subagents at 20 concurrent and depth 3. agnostic removed `max_concurrent_tasks` because nothing enforced it, and gets it back on this. **Budgets subdivide (2026-10-07; see *Source*)** down the tree: crew, then task, then delegated worker (F2). |
| F8 | **Protocol currency** | Medium, partly upstream | (a) `/mcp` speaks the `initialize`-handshake revision (`server/routes/mcp.cyr`). MCP 2026-07-28 removed the handshake and `Mcp-Session-Id`, added the `Mcp-Method` / `Mcp-Name` headers and made **Tasks** an official extension, and deprecated Sampling, Roots and Logging. The dispatch belongs to bote, so this starts as a **bote filing**. `/mcp` hand-builds its JSON-RPC envelope as the oracle did (`mcp.rs:3-5`) instead of delegating to bote's `Dispatcher`; bote 3.3.0's `dispatcher_set_server_info` removed the obstacle, and delegating is a separate decision. `capabilities` advertises no `subscribe` or `listChanged` ([ADR 015](../adr/015-mcp-resources-project-agent-definitions.md)). (b) Target A2A v1.0.x. The A2A callback POST is already sent (since 2.0.0): one detached thread per callback, posting the serialized `A2AResponse` through the SSRF-guarded fetch with a 30 s timeout, result ignored as the oracle does (`src/server/serve.cyr:309-337`). |
| F9 | **Hand downstream tasks compressed context** | Medium | A summarizing hand-off strategy beside Full, SlidingWindow and HeadTail (`orchestrator/memory.cyr`): a dependent task receives its upstream's `expected_output` or summary, not the full transcript. Every coordinator-worker system the review covered relies on clean-context workers that return short summaries. F2 needs it. **A summary is as untrusted as what it summarised (2026-10-07; see *Source*):** a hand-off built from tool output carries that output's trust, so it reaches the next task as data, never as instructions (F1's rule 2). |

Not adopted, recorded so nobody re-derives it:
- **Free-form swarms or parallel writers as an engine mode.** The evidence runs against them:
  MAST (NeurIPS 2025), Cognition 2026, and Tran & Kiela 2026 (single agent ≥ multi-agent at an
  equal token budget).
- **herdr-style screen-scraping coordination.** agnosai has structured events.

## Carried over from the Rust line

Test-coverage gaps the Rust tree never closed; the port should not reintroduce them.

| Area | What was missing |
|------|------------------|
| Concurrent cancel | mid-execution interruption, parallel/DAG cancel stress (owed as part of **B12**) |

The Rust tree also never tested process and Python sandbox env sanitization, timeout
enforcement and kill-on-drop, or the telemetry init paths (OTLP error paths, env var
override, guard lifecycle). The port's suites cover them (`tests/sandbox_spawn.tcyr`,
`sandbox_policy.tcyr`, `sandbox_process.tcyr`, `sandbox_python.tcyr`, `telemetry_mod.tcyr`,
`telemetry_otlp.tcyr`) and must keep doing so.

Demand-gated, unchanged: Python bindings (separate crate, `cdylib`, maturin build).

## Settled decisions

Decided, and **not to be re-raised**. Each entry gives the date, the decision, the reason, and
where the full reasoning lives. The D and F labels are kept because benches, `src/` comments and
docs cite them.

- **Scope: the whole Rust line was owed** (2026-08-07). The user overturned a narrowing of v2.0
  to the default cargo build. That narrowing was written by earlier sessions and repeated across
  four handoffs, but never decided by the user; only bhava was a user decree. **A cargo feature
  gate is not a scope boundary.** Never add an exclusion row: if something looks impossible,
  prove it — name the missing primitive, grep `lib/` for it, and file the upstream ask.
  [`cyrius-port-plan.md`](cyrius-port-plan.md) (lines 12, 320).
- **D1 — `rate_limit` is mounted by default** (2026-08-13). 100 req/s per client key, burst
  200; `AGNOSAI_RATE_LIMIT=0` restores the oracle's exact wire. The user's reason, verbatim:
  *"agnosai ships safe-by-default for operators who don't read docs; cause most people and even
  agents TLDR."* [ADR 021](../adr/021-rate-limit-mounted-by-default.md).
- **D2 — `personality` is deferred, never dropped** (recorded 2026-08-07, re-affirmed
  2026-08-13). bhava is coming and has no Cyrius port, so `personality` stays unported and the
  wire keeps emitting `null` until bhava lands in Cyrius. It is the one carve-out the user ever
  set; listing it as an open question once caused it to be asked again. *What agnosai is*,
  **B5**, [`cyrius-port-plan.md`](cyrius-port-plan.md) (:297, :380).
- **D3 — `builtin_presets()` stays cold** (2026-08-13). No memoization; the oracle's shape
  stands (`rust-old/src/definitions/loader.rs:122` has no cache either) and no ADR is owed. The
  user's reason, verbatim: *"presets can be cold; as agent might get different instruction sets
  than presets and seems a waste of speed improvements."* The route is not hot, and a
  process-lifetime cache of parsed structs is shared mutable state this tier avoids. The cost
  accepted: ~0.9 ms CPU and ~1 MB of arena churn per `GET /api/v1/presets`, reclaimed per
  request since 2026-08-11. CHANGELOG [2.0.0].
- **D4 — benchmarks run rounds × batch** (2026-08-13). Every `.bcyr` runs `_BR = 5` rounds of
  batch/5, so min/max are real samples (one sample per row had made min == max == avg, hiding
  ~5% run-to-run spread); total work is unchanged, and the whole CSV was re-baselined that day.
  `tool_registry_register` stays single-batch (it would measure fill level). **Standing rule for
  bench edits:** a row that consumes state (drains, dedups, subscribes) must index a distinct
  slice per round (`_r * (n / _BR)`) or be exempted. Find such rows by diffing every average
  against the pre-change run: of the six found, an assertion caught one. CHANGELOG [2.0.0].
- **D5 — dispatch bench rows include body serialization** (2026-08-13). `benches/server.bcyr`'s
  `route_*` rows measure the full route: `_b_render_body` / `_b_render_body_a` reproduce
  `_agnosai_serve_send`'s branch set, matching the oracle's tower-service bench
  (`rust-old/benches/server.rs:43-62`). Stopping at the response object hid serialization
  regressions. CHANGELOG [2.0.0].
- **D6 — 2.1.5 shipped as a patch despite a `### Breaking` section** (2026-10-04). The user
  tagged 2.1.5, then 2.1.6: the F6 span-vocabulary and genai API break shipped under a patch
  number, not 2.2.0 or 3.0.0. No known consumer was affected (agnostic, daimon and thoth were
  checked and call none of the removed names). Open follow-up: **D7**. CHANGELOG [2.1.5].
- **F7 — selection explanation keeps no per-run record** (2026-10-03).
  `agnosai_explain_selection_a` is a library call with no wire field and no ADR. The crew run
  keeps no breakdown, which would cost agents × tasks entries on the global bump every run;
  consumers recompute from the spec. That is exact while selection is a pure function of roster
  and task — **F2** carries the constraint for when it stops being one. CHANGELOG [2.1.5].
- **The tool SDK and `examples/wasm-tools/` stay Rust** (2026-08-11). Not a deferral.
  `sdk/agnosai-tool-sdk/` is a crate third-party tool authors compile to `wasm32-wasip1`;
  rewriting it in Cyrius would break every tool author on the published protocol. agnosai owes
  conformance only — the `{"parameters": …}` stdin and `{"result", "success", "error"}` stdout
  contract — which `src/tools/wasm_tool.cyr` implements and `tests/tools_wasm.tcyr` pins.
  [`adding-wasm-tools.md`](../guides/adding-wasm-tools.md).
- **No history rewrite for `probe_key_tmp.pem`** (2026-08-10). The deleted test key stays
  reachable in `bb76e67`. It was generated locally for a probe and never a credential; rewriting
  `main` costs more than it saves. `.gitignore` carries `*.pem` / `*.key`. `state.md`, *Known
  issues*, item 0.
- **Trace context is threaded explicitly, not thread-local** (2026-10-03). agnosai passes span
  contexts down its call path and never reads sakshi's process-global trace id: `thread_create`
  gives each worker a zeroed TLS block, agnostic's crews run on a detached thread, and
  `thread_local_get` faults on a main thread with no block.
  [ADR 023](../adr/023-genai-semconv-spans-and-w3c-trace-context.md).
- **Per-task audit only in sequential mode is parity** (2026-08-06). The oracle's only
  `audit_record` call site is in its `run_sequential` (`rust-old/src/orchestrator/crew_runner.rs:294`),
  so parallel and DAG modes never audit per task, and no per-worker audit scratch is owed.
  CHANGELOG [2.0.0].
- **Literal hoisting scope** (2026-08-07, the former B3). Hoist a literal to a process-lifetime
  global only when a loop body or an unconditional envelope reaches it; an arena allocation is
  not free (~11 ns plus a 16 B header), and hoisting took `/dashboard/crews` 6,881 → 5,217 ns.
  Error messages stay inline: built at most once, for a request that already failed. The
  `str_from_a` sites left in `src/` (338 at the count) are deliberate, as are three in-loop sites
  on return paths (`sandbox/oci.cyr:250-251`, `crew_runner.cyr:990`) that a naive scan keeps
  finding. The `_a` forms keep their signature and return the global.
  [`cyrius-port-plan.md`](cyrius-port-plan.md) (:372), CHANGELOG [2.0.0].
- **The JWT `alg` check runs after signature verification** (2026-07-31). By design: parsing
  attacker-controlled JSON before authenticating measured ~53x heap amplification, so a rejected
  token paying the modexp is the accepted cost. CHANGELOG [2.0.0], *Security*.
- **cyrius 6.6.6's by-value struct deep copy does not reach `: Str` parameters** (2026-09-26).
  42 functions take a `: Str` and 141 sites store one into a heap object or a `vec` that
  outlives the call; they would dangle if the copy applied. It does not: `Str`, `Result`,
  `Option` and `Tagged` are excluded by name (`_local_is_sptr_param`, 6.6.6 bite 16b) and stay
  pointer rebinds, and agnosai declares no struct-typed parameters. No change needed. Full
  read-through: `git show 2.1.6:docs/development/roadmap.md`, *Moving the cyrius pin to 6.6.6*.
- **Not adopted: free-form swarms and screen-scraping coordination** (2026-10-03). See F,
  *Not adopted*.
- **Federation adopts any declaration whose term is not stale** (2026-11-05, the former B30 item 4).
  `agnosai_federation_declare_coordinator` keeps its `term < current` guard. Adopting a higher term is the
  leader-election rule. Refusing a conflicting declaration at the same term would buy nothing, because with no
  vote a peer could declare at term+1 instead. Federation trusts its peers; `threat-model.md` surface 4 says so,
  and the reasoning is in `src/fleet/federation.cyr`'s header.
- **The MCP client is bote's** (2026-10-07). A crew's agents reach external MCP servers through
  bote, by way of agnosai's tool registry (F1); neither agnosai nor agnostic carries its own MCP
  client. bote is the ecosystem's MCP layer, built so that apps stop implementing MCP each for
  themselves. The client half is not built yet (*C*, bote). Recorded in agnostic's `roadmap.md`
  (*Settled decisions*) the same day.

## Out of scope

- The bhava carve-out (see *What agnosai is*).
- Inbound chunked request bodies — sandhi answers 501, honest but not support; B8 and B16
  track the upstream gap.
- Any comparison of Cyrius benchmark numbers against the frozen Rust CSV.
- `fleet/discovery` beyond the oracle's own 174-line stub. `rust-old` says it is a stub in its
  own doc comment, and `lib/net.cyr` has no SRV resolver. Finishing it is a scope decision
  nobody has made, not owed work.

### Owed to the ecosystem — an OpenTelemetry library repo

**There is no Cyrius OTel library, and two projects have now hand-rolled the
same subset independently.** hoosh wrote `src/lib/otlp.cyr` (199 lines);
agnosai wrote `src/telemetry/otlp.cyr` from the same shape, because the
alternative was depending on hoosh as a *library*, which ADR 003 forbids. That
is the second instance. The third will be whoever needs traces next, and by the
port's own rule (`CLAUDE.md`, *Refactoring*) the third instance is when the
abstraction gets extracted.

⚠ **It is a separate repo, not a cyrius stdlib module.** Not `lib/otel.cyr` and
not a language feature — a sibling library like sakshi, sigil, majra or bote:
its own repo, scaffolded with `cyrius init`, publishing a `dist/<name>.cyr`
bundle, consumed the way every other one already is —

```toml
[deps.<name>]
git = "https://github.com/MacCracken/<name>.git"
path = "../<name>"
tag = "X.Y.Z"
modules = ["dist/<name>.cyr"]
```

Naming is the user's call; the AGNOS convention is a Sanskrit/Persian word, not
`otel`.

What it would own, in rough order of value:

- **OTLP/protobuf.** Both hand-rolls emit OTLP/JSON purely because there is no
  Cyrius protobuf codec. JSON is a first-class OTLP transport so nothing is
  broken, but it is several times the bytes on the wire and every collector
  prefers the binary encoding. ⚠ The protobuf codec is itself probably a
  *separate* repo — a general wire-format library, not an OTel concern — so it
  is a prerequisite with its own decision attached, not a sub-task.
- **The span model** — attributes, events, links, a parent, a status.
  agnosai's `telemetry/genai` had to invent one because sakshi's spans are name
  + timing with no attribute channel, and it is deliberately shaped for GenAI
  rather than for anything general.
- **Context propagation.** W3C `traceparent` parse/serialise, and a current
  span. sakshi's trace id and span stack are process globals, so under
  `sandhi_server_run_pooled` two concurrent requests share one trace id.
  agnosai parses and formats `traceparent` itself (`telemetry/otlp.cyr`'s
  span-context section) and threads span contexts **explicitly** down its call
  path ([ADR 023](../adr/023-genai-semconv-spans-and-w3c-trace-context.md)). The
  cyrius stdlib has `thread_local_*`, but a thread-local current span would not
  help here: `thread_create` gives each worker a zeroed TLS block
  (`lib/thread.cyr`), agnostic's crews run on the orchestrator's detached
  thread, and `thread_local_get` faults on a main thread with no block
  (`orchestrator/audit.cyr`, `arena_pool.cyr`). A library would still own this
  for everyone else — with an explicit-context API, not only a thread-local
  one.
- **A batching exporter** with a bounded queue and a drain thread. Both
  hand-rolls have one; hoosh's is a 256-slot ring behind a mutex.
- **Metrics and logs.** OTel is three signals; both hand-rolls do traces only.

Once it exists, `src/telemetry/otlp.cyr` collapses to a thin adapter and
`telemetry/genai`'s span record becomes that library's span type — the same way
`fleet/relay` collapsed onto majra once majra could carry it.

⚠ **This is a note, not a filing.** It is recorded here so the next consumer
finds the prior art instead of writing a third encoder, and so the cost is
visible when someone decides whether the ecosystem wants the repo.

## `[embed]` for the presets — adoptable since 2.1.8, a 2.2.3 input (recorded by cyrius 6.6.19, 2026-10-06)

**Not adopted yet** — the presets are still generated by
`scripts/gen-presets.sh`. Docs-only note from the cyrius 6.6.19 lanes. Nothing here gates a cyrius release.

- **`scripts/gen-presets.sh` and the checked-in `src/definitions/presets_data.cyr` can retire for `[embed]`**
  (cyrius P2 — your 2026-08-10 proposal, shipped in 6.6.19).
  `[embed] NAME = "path"` in cyrius.cyml gives every compile `NAME()` (the file's bytes, NUL-terminated) and
  `NAME_len()`, read from the file at build time — no generated `.cyr`, nothing to drift. Explicit entries only
  (the `{dir, glob}` set form is refused by name). All embeds share cycc's 2 MiB string pool with the program's
  own literals, and every binary of the project (test binaries too) carries every declared embed. Reference: the
  cyrius guide's *Embedding data files: [embed]*, CHANGELOG [6.6.19] *Embed — P2*.
  - One entry per preset, in the order you list them: `AGNOSAI_PRESET_QUALITY_LEAN =
    "src/presets/quality-lean.json"`, … ×18.
  - `var AGNOSAI_PRESET_X` becomes the call `AGNOSAI_PRESET_X()` (a fn, not a var: a top-level string var is a
    deferred runtime store); the length is `AGNOSAI_PRESET_X_len()`.
  - Stays yours: the JSON COMPACTION the script does. `[embed]` embeds the bytes verbatim, so the presets keep
    their whitespace (~810 lines). Keep a compaction step (or commit compact JSON) if binary size matters — a
    data-file step, no longer a source generator.
