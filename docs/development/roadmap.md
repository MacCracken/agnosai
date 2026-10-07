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

## Deleting `rust-old/` — scheduled for the release after 2.1.0, still owed at 2.1.6

None of 2.1.1 to 2.1.6 carried the deletion. The deletion itself needs the user's OK.

**The 2.1.0 audit (2026-09-26)** read every item and every `#[test]` in `rust-old/` against
the Cyrius tree — seven module groups plus everything outside `src/`. Every `.rs` file has a
Cyrius counterpart; nothing outside `rust-old/` reads it at build, test, bench, fuzz or release
time. What it found that was easy is fixed in 2.1.0 (see the CHANGELOG); the rest is owed work
**B4–B16** under *Owed work* below. Every Rust spec stays reachable after deletion:
`git show 2.1.0:rust-old/src/<path>`.

- [ ] `git rm -r rust-old`, then delete the ignored `rust-old/target/` (~13 GB) from disk — `git
      rm` leaves it.
- [ ] Update the text that describes it: CLAUDE.md (identity line, layout section, the DO NOT
      rule — the user's file, and **B23** moves with it), `CONTRIBUTING.md:47`,
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

## Moving the cyrius pin to 6.6.15 — found at the 2.1.5 cut, not started

cyrius **6.6.15** is tagged upstream (seen 2026-10-04). Its fold carries sigil **3.13.9**
(sigil tag commit `7d7a880`), which contains 3.13.8 (`bbaecc4`). agnosai pins 6.6.14 and
`[deps.sigil] tag = "3.13.7"`, the 6.6.14 fold. Neither 2.1.5 nor 2.1.6 moved the pin.

**What sigil 3.13.8 / 3.13.9 fix that agnosai reaches at 6.6.14.** Sources: sigil CHANGELOG
[3.13.8] and [3.13.9], cyrius CHANGELOG [6.6.15].

- **Not the key exchange.** At 6.6.14 native TLS offers x25519 only:
  `lib/tls_native_hs13.cyr:11` ("Offers x25519 only (no P-256 ECDH in sigil yet)") and
  `lib/tls_native_hs12.cyr:303-304` ("Our ECDHE stays x25519 alone"). 3.13.8's constant-time
  P-256 / P-384 ECDH (`ecdh_p256_*` / `ecdh_p384_*`) is new API. Native TLS first uses it in
  6.6.15, for TLS 1.2 ECDHE on secp256r1 / secp384r1. The move gains a capability here; it
  fixes no path agnosai runs today.
- **Not ECDSA signing (CVE-68, HIGH), as agnosai runs today.** 3.13.7's `ecdsa_p256_sign` /
  `ecdsa_p384_sign` ran the nonce and the private key through variable-time arithmetic. Native
  TLS signs only in `_tn_sign` (`lib/tls_native_hs13.cyr:1540-1549`), for the 1.3
  CertificateVerify, the 1.2 ServerKeyExchange and the 1.2 client CertificateVerify. That
  path runs only for a TLS server or for a client that holds a client certificate. agnosai is
  neither. Its HTTPS is outbound through sandhi (`SANDHI_TLS_POLICY_DEFAULT` at
  `src/tools/remote_registry.cyr:168`, `src/tools/agnos.cyr:282` and
  `src/tools/builtin/security_audit.cyr:644`). Nothing in `src/` loads a client key, and no
  other `lib/` module calls the signers. A consumer of `dist/agnosai.cyr` that installs a
  client certificate would reach this path.
- **Yes: HMAC and HKDF state left in dead stack (CVE-68 part B, MEDIUM).** Through 3.13.7,
  `hmac_sha256` / `hmac_sha384` never wiped their SHA context or inner hash, and the context's
  final state is the MAC. `sha384_finalize` kept its output in scratch, and the SHA-NI and AES-NI
  asm blocks left their state in vector registers. Every outbound HTTPS handshake does the
  following:
  - It runs the TLS 1.3 key schedule on sigil's HKDF (`lib/tls_native_keysched.cyr:114-155`).
  - It computes the Finished MACs with those HMACs (`lib/tls_native_hs13.cyr:553-621`).
  - It encrypts records with AES-GCM (`lib/tls_native_keysched.cyr:573-612`).

  The cyrius [6.6.15] notes name "any HMAC / HKDF output (the TLS key schedule)" as left in the
  dead SHA context. agnosai's audit chain (`src/orchestrator/audit.cyr:210`) and kavach
  (`lib/kavach.cyr:7511`) call the same `hmac_sha256`. What those two leave behind is the MAC
  they publish anyway, plus `hmac_sha256`'s unwiped inner hash (`lib/sigil.cyr:6800-6801`), which
  the audit chain does not publish but which does not stand in for the key.
- **Nothing in 3.13.9.** Its one security fix (cyrius CVE-73) is for Windows only: the TPM,
  Secure Boot, IMA, dm-verity and LUKS helpers probed rooted POSIX paths, which are
  drive-relative on Windows. agnosai builds for x86_64 and aarch64 Linux and calls none of
  those helpers. The rest of 3.13.9 supports that Windows fix (`agnosys_rooted_paths_untrusted()`,
  `[lib.secureboot]` carrying `src/sys_util.cyr`, `dist/sigil-tpm.deps` gaining `sys`), plus sigil's own
  toolchain pin (6.6.9 → 6.6.14) and tests. None of it reaches agnosai.

Two fixes in cyrius 6.6.15 itself, not in sigil, also reach agnosai's outbound TLS:

- **CVE-70:** neither TLS 1.3 side zeroised its ephemeral private key or shared secret after
  use. At 6.6.14 `lib/tls_native_hs13.cyr:24` and `:181` store them and nothing wipes them.
- **CVE-71:** neither TLS 1.3 side refused an all-zero x25519 result (RFC 8446 §7.4.2).

A third, **CVE-69**, fixes a `secret var` epilogue that left its return registers in dead stack
after its own wipe. It applies to every function that 6.6.14 compiles with a `secret var`.
Native TLS has none of those. In agnosai's `lib/`, the ones that exist are sigil's Ed25519
key-pair generation and libro's key generation and timestamp nonce (`lib/libro.cyr:3928-4728`).

**Siblings first.** Fix this at the source: release the siblings, and add no override pins
here.

- kavach **3.13.2** and libro **2.10.6** are their repos' latest tags. Both pin cyrius **6.6.14**
  and `[deps.sigil] tag = "3.13.7"`.
- libro also takes sigil's `src/sha_ni.cyr`, `src/sha256.cyr`, `src/hex.cyr` and
  `dist/sigil-mldsa.cyr` at that tag. Its optional `[deps.sigil_tpm]` moves in lockstep with
  it.
- If only agnosai's sigil moves, both siblings declare a sigil that is off the fold.
  Closest-wins would skip both, as it skipped kavach 3.13.1's at 2.1.3. But `state.md`'s *Now*
  would no longer hold: it records that every dep's own pins name the folds.
- libro reaches agnosai only through bote. bote **3.3.16** pins libro 2.10.6 and cyrius 6.6.14,
  and takes `sigil` from its stdlib fold. So bote needs a release after libro, the same
  three-repo chain as 2.1.3.

- [ ] Release kavach and libro on cyrius 6.6.15 with `[deps.sigil] tag = "3.13.9"` (libro's
      `[deps.sigil_tpm]` with it). Then release bote on 6.6.15 with that libro. Certify each in
      a sibling-free replica with an empty dep cache.
- [ ] Read 6.6.15's release notes against this tree, checking each numbered item with file:line
      evidence. The model is the 6.6.6 read-through: `git show
      2.1.6:docs/development/roadmap.md`, section *Moving the cyrius pin to 6.6.6*.
- [ ] Move `cyrius = "6.6.15"`, `[deps.sigil] tag = "3.13.9"` and the new kavach and bote tags
      together (the sigil tag tracks the fold: rule 1 of the 2.0.10 notes in `state.md`). Then
      run `cyrius lib sync --full` and `cyrius deps`, and certify in a sibling-free replica with
      an empty dep cache. Before certifying, check that this host's `~/.cyrius/versions/6.6.15`
      `SOURCE_COMMIT` reads `2f1ed9d1` with `tree-matches-tag: yes` (it did at the 2.1.6 cut,
      2026-10-04): a concurrent session working on cyrius rewrites these slots.
- [ ] The full gate, including `check-clean.sh`'s lib snapshot against the 6.6.15 snapshot and
      `check-symbols.sh` on both targets.

## Owed work

**Status as of 2.1.6 (2026-10-04). Nothing here blocks anything.** Every remaining item
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
section A (it blocked M6 and was retired 2026-08-03), and B1–B3, B17, B21, F6 and F7 are closed.
B2's allocator-threading traps, which `src/` and `tests/` comments still cite as "roadmap B2", are
in `state.md` (*Test-design decisions worth not re-deriving*, *Standing rules a new session should
not re-derive*) and CHANGELOG [2.0.0].
Rows below still name F6 (the GenAI spans, [ADR 023](../adr/023-genai-semconv-spans-and-w3c-trace-context.md))
as where they were found; F7's non-goal is under *Settled decisions*.

### B. Owed — flagged in earlier bites, never done

| # | Item | Effort | Notes |
|---|------|--------|-------|
| B4 | **Hard rate-limit key cap: adopt majra's force-evict when it ships** | Small | majra 2.8.1 made `ratelimit_evict_stale` skip any bucket not yet refilled to burst, so agnosai's zero-threshold pressure sweep (`_agnosai_rl_maybe_sweep`, `src/server/rate_limit.cyr:419`) no longer holds the 4,096-key cap under a strict rate: 72,447 keys at 1 req/s against 4,352 on majra 2.7.2 (default 100 req/s unaffected). Requested 2026-09-26 in majra's `docs/development/issues/2026-09-26-agnosai-ratelimit-no-way-to-enforce-a-key-cap.md` (still open; majra 2.9.2, the pin, has no force-evict). When a majra release carries the call: bump `[deps.majra]`, call it from the pressure sweep only, and re-run the 200k-key spray at 1 req/s. |
| B5 | **Agent personality (bhava)** | Large, blocked | `personality`, `with_personality`, `mood_adjusted_temperature`, the personality score and the system-prompt block. `../bhava` has no Cyrius port, so nothing can supply a profile. Unblocked by porting bhava (a separate repo). See *What agnosai is* and D2. |
| B6 | **Crew concurrency** | Medium | No cap on concurrent crews: `submit_crew` spawns a thread per call, where the Rust tree held a semaphore of `max_concurrent_tasks` (default 10). Parallel mode runs fixed batches, so one slow task idles the other slots until its batch ends. Both want a counting semaphore, which the stdlib does not have. |
| B7 | **Tracing export** | Large | The Rust tree exported every `#[tracing::instrument]` span over OTLP (20 of them: sandbox, routes, crew runner, orchestrator, scoring, cost planning); agnosai exports the four GenAI semconv operations (`invoke_workflow`, `invoke_agent`, `chat`, `execute_tool`), linked into one trace and joining an inbound `traceparent` since 2.1.5 ([ADR 023](../adr/023-genai-semconv-spans-and-w3c-trace-context.md)). **What remains:** (1) the other `#[instrument]` sites as spans, under the same explicit span-context threading (no thread-local: ADR 023 records why); (2) trace and span ids in JSON log lines, and per-module targets, which needs sakshi's log hook to expose them; (3) aggregate usage on `invoke_agent` (the sum of its `chat` spans' tokens); (4) request parameters on `chat` (`gen_ai.request.temperature`, `max_tokens`, declared and never set); (5) `gen_ai.response.finish_reasons`, a `string[]` that needs an OTLP `arrayValue` the encoder does not write; (6) `Span.flags` with the W3C is-remote bit for a span whose parent arrived in a header; (7) `tracestate` passthrough; (8) tool-call arguments and results on `execute_tool`, which need F1; (9) **the `delegate` tool starts an unlinked trace** (found 2026-10-03, F6 review): `src/tools/builtin/delegate.cyr:78-83` builds the delegated spec and calls `agnosai_orchestrator_submit_crew` with no trace parent, because the tool vtable carries no span context. **Repro:** a deployment that calls `agnosai_register_delegate_builtin`, with the exporter on, sends `/mcp` `tools/call` `delegate` with a `traceparent`: the `execute_tool delegate` span joins the header's trace, the delegated crew's `invoke_workflow delegate:<agent>` span is a new root. The pinned semconv (`gen-ai-agent-spans.md`, *Invoke workflow span*) also says an `invoke_workflow` span SHOULD NOT be reported when an agent or tool spins up a workflow to delegate to a sub-agent. **Fix direction:** once a tool call has a span context (F1, or a context on the vtable), format it with `agnosai_otlp_span_context_format_a` and stamp it on the spec through `agnosai_crew_with_trace_parent`, and decide whether a delegated crew emits `invoke_workflow` or only its `invoke_agent` spans; (10) **an operator switch for an untrusted inbound context** (found 2026-10-03, F6 review): any caller of the three traced routes picks the trace id agnosai's spans join and can turn their export off with sampled flag `0` (`src/telemetry/mod.cyr`, `agnosai_telemetry_record_span_in`: `if (agnosai_otlp_span_context_sampled(sc) == 0) { return 0; }`; pinned by `tests/server_serve.tcyr`'s unsampled-header check). Recorded as surface 7 in `threat-model.md` and in ADR 023. **Fix direction:** an env switch read at boot, e.g. `AGNOSAI_OTEL_INBOUND=honour|restart|record` — `restart` ignores the header (new root, the inbound id at most as a span link), `record` keeps the trace but samples regardless of the flag. |
| B8 | **Server edges** | Medium each | An SSE stream holds one of the 100 pool workers for the crew's life and does not notice a departed client (sandhi's `send_chunk` ignores send errors; filed, see *C*, sandhi); shutdown does not drain in-flight requests; chunked request bodies are 501 (sandhi has no decoder); a 405 carries no `Allow` header; path parameters are not percent-decoded; a certificate-format JWT key (and a key over 4096 bits) is refused; `hot_config` has no change notification. |
| B9 | **WASM** | Medium | A module with no `_start` export runs and "succeeds"; a module is validated by its header only, so a malformed body fails at execute; the Rust store limits (10 instances/tables/memories, trap on failed grow) and stdin over 64 KiB need kavach options; WASM tools held by a registry never free their staged module (`agnosai_wasm_module_free` exists; the registry has no destroy hook). The execute path runs the `wasmtime` CLI through kavach's WASM backend ([ADR 019](../adr/019-wasm-tools-spawn-wasmtime-directly.md); the filename names the reversed draft). |
| B10 | **JSON forms nothing calls** | Easy each | Serde shapes the Rust types derived but no Rust code serialised: fleet `NodeInfo`/`NodeStatus`/`NodeTopology`/`DeviceLink`/`InterconnectType`/`FederationRole`/`ClusterStatus`/`ClusterInfo`/`RelayMessage`; `RetryConfig`; `TaskProfile`, `BudgetExceeded`, `CachedPlan` and `ToolSchema`/`ToolOutput` parsers; `ToolInput` and the manifests' writers; `KavachToolResult`. |
| B11 | **Structured-log fields** | Easy each | Log events that lost their fields or are absent: fleet (17, federation's nine above all), wasm (no sakshi calls at all), kavach bridge, process/OCI exit lines, wasm loader and remote registry error details, the MCP tool-call line's `success`/`duration_ms`, a2a rejections' id prefix and `metadata_bytes`, `latency_ms` in task-result metadata. |
| B12 | **Behaviours no suite asserts yet** | Easy each | Orchestrator: diamond DAGs (sort, ready set, a run), parallel/DAG `task_ms`, event-bus delivery, the cancel audit entry, per-task audit level, a 64 KiB+ IPC echo, and **concurrent cancel** — mid-execution interruption and parallel/DAG cancel stress (`tests/orch_crew_runner.tcyr:1382-1408` cancels only before a run starts; the Rust tree never tested it either, see *Carried over from the Rust line*). Core: `Cpu` → `"cpu"`, multi-device filtering and sums, a negative non-zero cost. Sandbox: real fuel exhaustion (-2), a real timeout, empty stdin to a reading guest, the Python bridge's `"error": null`. Tools: AGNOS schema names/descriptions/parameter types and missing-parameter errors. Server: output-filter pattern text, `http://[::1]:8080`, the echo tool's parameter type. |
| B13 | **Network-facing tools** | Medium | The load tester has no connection pool (every request is a fresh handshake); its success path and the security audit's live sequence have no test, since no suite stands up a loopback server. |
| B14 | **Seven benchmarks** | Easy–medium | The sandbox execute path (process argv, stdin, shell; WASM hello) and `audit_recent(10/100/1000)`. |
| B15 | **Smaller gaps** | Easy each | Output-filter leak detection folds ASCII case only; a pub/sub subscriber cannot detach alone (only `unsubscribe_all(pattern)`); `StateStore` (pluggable persistence) is not ported; `strcase` is ASCII-only though the crew assembler matches user-supplied names. **Suites that exit 0 with an assertion failing:** `tests/telemetry_wiring.tcyr` (seen 2026-10-03 under a mutant), because a thread is still alive when its epilogue's `syscall(60, f)` ends only the main thread. `cyrius test` still fails it on the printed summary, so the gate holds, but CI's per-suite bisect reads exit codes and would not name it. **Not one suite (F6 gate run, 2026-10-03):** `tests/telemetry_otlp.tcyr` and `tests/telemetry_mod.tcyr` exit 0 with failing assertions too, while `core_crew` and `server_serve` exited 1. Whether a suite is hit depends on whether an exporter thread is still running at exit: `agnosai_otlp_exporter_stop` only sets a flag and flushes (`src/telemetry/otlp.cyr:1278-1281`, no join), and the thread trampoline exits with status 0 (`lib/thread.cyr:197-199`). **Repro:** a `.tcyr` that fails one `assert` and then `thread_create`s a body that sleeps 300 ms exits 0; the same file without the thread exits 1. **Fix direction:** an epilogue that calls exit_group (a CLAUDE.md rule change); or make the OTLP exporter thread joinable, so `stop` sets the flag, joins, then flushes and no exporter thread outlives `main` (2.1.6 serialised the two flushes under one lock but added no join); or, with no code change, have CI's bisect also fail any suite whose run log matches `[1-9][0-9]* failed`. **`/mcp` `tools/call` dereferences a tool's output without checking it** (found 2026-10-03, F6): `src/server/routes/mcp.cyr:281` passes `agnosai_tool_output_success(out)` to the log line, and `:283` and `:289` read it again, while the vtable contract allows a tool to answer 0 (`src/tools/native.cyr`, `agnosai_tool_execute_in`, which null-checks for exactly that reason and records `error.type` `no_output`). **Repro:** register a tool whose execute fn returns 0 and call it through `POST /mcp` `tools/call`: the handler segfaults. No builtin returns 0 today, so it is latent. **Fix:** treat `out == 0` as a tool-level failure — `isError: true` with "Unknown error", the oracle's fallback text — before any read. |
| B16 | **Upstream gaps — noted, NOT raised** | — | ⛔ **Do not raise any of these until agnosai is on the latest version of that project**, then re-check the gap against that version first. bayan, sigil and sandhi are cyrius stdlib folds: agnosai takes the version the pinned cyrius folds, and their newer releases wait for the next cyrius fold. As of 2.1.6 (cyrius 6.6.14), bayan 1.5.11, sandhi 1.10.4 and kavach 3.13.2 are their projects' latest tags, so those three gaps are due their re-check against those versions; none has been re-checked yet. sigil's repo is at 3.13.9, which agnosai takes only with the 6.6.15 pin (*Moving the cyrius pin to 6.6.15* above), so the rule holds the sigil gap until that move; 6.6.15 folds the same bayan and sandhi. **bayan** (fold 1.5.11; listed at 1.5.6) — YAML beyond the subset parser: block scalars, flow mappings, anchors, tags, escape decoding in quoted strings. **sigil** (fold 3.13.7; listed at 3.12.18) — RSA keys over 4096 bits (a certificate-format key is agnosai's own to fix, B8). **sandhi** (fold 1.10.4; listed at 1.9.17) — decoding chunked request bodies. **kavach** (3.13.2, a direct dep; 3.13.2 rebuilt the WASM backend's capture and stdin, so re-check the stdin gap against it) — WASM store limits (instance/table/memory caps, trap on a failed grow) and guest stdin over 64 KiB. |
| B18 | **`lt_aggregate_100k_500workers` is ~2x its 2026-08 band** | Medium | Found at 2.0.6 (88.2 ms against ~24 ms), 45.4 ms at 2.1.3 on cyrius 6.6.14, and still open. The measurements, what is ruled out (not the compiler, not the benchmarked source, not noise) and where to start (the 2026-08-03 `_a` allocator work, a ~6-round bisect over 40 commits) are in `state.md`, "⚠ OPEN — a ~3.6x regression, found at 2.0.6". |
| B19 | **Suites compile with `warning: undefined function '_agnosai_loader_read'`** (found 2026-10-03, F7 gate run) | Easy | `src/sandbox/wasm.cyr:235` and `src/tools/wasm_loader.cyr:142,163` call `definitions/loader`'s private reader. A suite that includes `src/sandbox/mod.cyr` or `src/tools/mod.cyr` without `src/definitions/loader.cyr` compiles with a dangling call: 20 such warnings in a full `cyrius test` log at F7's gate run, and 23 at the 2026-10-04 integration gate and in a full run during 2.1.6's work. **Repro:** `cyrius build tests/telemetry_wiring.tcyr build/tw` prints the warning. No suite reaches the call, so they pass, but a suite that did would jump through an unresolved symbol. The standing warning also hides any *new* `undefined function` line, which is how real breakage has been caught before (CHANGELOG, `_agnosai_to_ascii_lower`). **Fix:** do the hoist `src/sandbox/wasm.cyr:228-234` already prescribes. Move one whole-file reader into a port-local root module, used by `definitions/loader`, `sandbox/wasm`, `tools/wasm_loader` and `orchestrator/durable_state`'s `_agnosai_read_file_exact`. Then check that the full `cyrius test` log has no `undefined function` line. |
| B20 | **A second live thread moves a crew's cost — both ways** (found 2026-10-03, F6 fix round) | Medium | Measured with one process per row, seven interleaved rounds, medians: an idle detached thread that only `sleep_ms(25)`s took `run_crew_10_tasks_sequential` **334 → 400 µs (+20%)** and `run_crew_10_tasks_parallel_4` **624 → 456 µs (−27%)**, at 2.1.4 and on the then-current tree alike. `benches/orch.bcyr` already runs those rows at the second-thread level (parallel ~456 µs; sequential 380–400 µs across two gate runs), so some earlier row leaves a thread alive, which is also the likeliest reading of the `crew_runner_10_tasks_parallel_4_quiet` level shift recorded under CHANGELOG 2.1.5 → Performance. **Repro:** add a mode to a copy of `benches/orch.bcyr` that calls `thread_create_detached` on a `while (!stop) { sleep_ms(25); }` body before `_b_run_crew("run_crew_10_tasks_parallel_4", ...)`, and run it beside the plain row in separate processes. **Why it matters:** a production server always has other threads (the pool, the exporter), so the single-threaded bench level is not what a deployment sees, and a row's number depends on what ran before it. **Direction:** find which stdlib path switches on a second live thread (mutex/futex, the allocator, `thread_create`/`thread_join`, stack mmap) — if it is the stdlib, file it against cyrius rather than patch it — and then decide whether `benches/orch.bcyr` should pin the level explicitly (start one idle thread in `main`) so rows stop depending on their neighbours. Related: **B27**, the same file's rows moved by heap position. |
| B22 | **Comments and docs that point at places that no longer say it** — four at `CHANGELOG [Unreleased]` for work that shipped in 2.0.0 (found 2026-10-04, at the 2.1.5 cut), and the pointers into this roadmap that clearing it left (2026-10-04) | Easy | `src/server/router.cyr:15` (bite 15b, the sandhi adapter), `src/server/auth.cyr:113` (the RS256 half, M6), `src/server/routes/a2a.cyr:266` (the A2A callback's SSRF gate) and `tests/server_routes_health.tcyr:136` (ADR 011's wiring job) each cite `CHANGELOG [Unreleased]`, which since the 2.0.0 cut has meant whatever the next release holds. Written 2026-07-31 to 2026-08-03. Point each at the release that carried it (or at the ADR), then regenerate `dist/agnosai.cyr` with `cyrius distlib --all`, since the three `src/` comments are in the fold. `src/server/auth.cyr:113` also cites `docs/development/roadmap.md (M6)`, a section this file no longer has; point it at CHANGELOG [2.0.0] in the same edit. **The same sweep, for pointers into this roadmap** (left when completed items were cleared from it, 2026-10-04): line numbers that now land elsewhere — `src/core/mod.cyr:34` (`roadmap.md:43`) and `src/orchestrator/crew_runner.cyr:194` (`roadmap.md:18`), both meaning the bhava carve-out, now *What agnosai is* and D2; `crew_runner.cyr:1582,1649` (`roadmap.md:160`, the hierarchical fallback, now F2). Sections that are gone — `src/main.cyr:88` (M8), `src/telemetry/mod.cyr:9` (M9), `src/telemetry/otlp.cyr:795,1054` ("section E"; 2.1.6's `tests/telemetry_otlp.tcyr` now POSTs to a local stub, so `:1054`'s "not reachable from the suite" is stale too). Comments that still call B2 open (`src/server/serve.cyr:32`, `src/server/routes/mod.cyr:29`, `src/server/auth.cyr:647`); B2 is closed and its traps are in `state.md`. Name sections, not line numbers, and run `cyrius distlib --all` after. Outside `src/`: `tests/sandbox_spawn.tcyr:6` and `tests/sandbox_policy.tcyr:18` say the *Carried over* **table** records env sanitization, which is now the prose under it; `docs/adr/017-*.md:99,135` cite *Out of scope for v2.0*, now *Out of scope* → *Owed to the ecosystem* (a dated note, since ADRs are records); `docs/adr/006-*.md:147` cites an M7 record now only in CHANGELOG [2.0.0]; `state.md` cites the M7/M11/M12 sections (`:1510`, `:1513`, `:1616`, `:1717`), A2 (`:1595`), D1 as open (`:2305`) and B2 as open (`:2699`) — fix at the next state refresh; `docs/guides/api-reference.md:441` says "the preset library endpoint is on the roadmap", which it is not. |
| B23 | **`CLAUDE.md` describes `rust-old/` as of 2.1.0, and its consumer list is stale** (found 2026-10-04, at the 2.1.5 cut) | Easy | `CLAUDE.md:16` (the identity line) says rust-old/ is "scheduled for deletion in the release after 2.1.0". `CLAUDE.md:24` says it is reachable at "any tag up to 2.1.0". rust-old/ is still present in 2.1.1 through 2.1.6. Fix: name the last tag that carries rust-old/ once it is deleted, or until then "every tag through the current one". `CLAUDE.md:20` lists daimon as a consumer, but daimon has no `[deps.agnosai]` (its deps are sakshi, ai-hwaccel, samay, sigil, libro, majra, bote) and no HTTP client aimed at an agnosai endpoint — checked at 2.0.0 and still true at 2.1.6; agnostic is the consumer. `CLAUDE.md:187` describes `roadmap.md` as "milestones through v1.0, dependency gates". **CLAUDE.md is the user's file: the user edits it, or approves the edit.** `state.md`'s *Consumers* (`:2929-2930`) also says "none consuming the Cyrius line yet"; fix it at the next state refresh. Moves with the `rust-old/` deletion. |
| B24 | **`cyrius distlib --all --check` is a gate in `state.md` but nothing runs it** (found 2026-10-03, integration pass) | Easy | Neither `.github/workflows/ci.yml` nor `scripts/check-clean.sh` calls it, so a `src/` change committed without `cyrius distlib --all` would ship a stale `dist/agnosai.cyr` to every consumer, and CI would stay green. It is run by hand at each cut (2.1.5, 2.1.6) and passed. Fix: add it to check-clean.sh, which CI already runs. Then mutation-check the gate: edit one `src/` comment without regenerating, and the gate must fail. |
| B25 | **The serve handler's arena read of `traceparent` is not covered by a test** (found 2026-10-03, F6) | Medium | `sandhi_server_find_header_a(…, "traceparent")` runs only on the arena branch, which needs a sandhi worker arena. The pipe harness reaches only the bare branch, as it does for every other header. Fix: a harness that runs one request through a real pooled worker with an arena, asserting that the crew's spans carry the inbound trace id. That would cover the other arena-branch header reads too. |
| B26 | **x86_64 and aarch64 binary growth do not track each other** (found 2026-10-04, at the 2.1.5 cut) | Easy (investigation) | At 2.1.5 x86_64 grew 16,976 B but aarch64 only 592 B. The x86 growth is accounted for item by item (+4,112, +4,352, +8,512); the aarch64 figure was not explained. At 2.1.6 the reverse: x86_64 grew 8,520 B (5,240,304 → 5,248,824) and aarch64 65,864 B (6,423,568 → 6,489,432) — but `state.md` labels the 2.1.6 aarch64 figure "(DCE)" and the 2.1.5 one "cross-build", so first confirm both were built the same way. Both binaries contain the new code and answer `/ready` with their version. Fix: compare the symbol sizes of the two builds (`CYRIUS_DCE_VERBOSE=1`) to see whether aarch64's DCE or code generation absorbs the difference, or whether something is missing; investigate both releases' deltas together. If it reveals a toolchain anomaly, file it with cyrius; do not patch the toolchain. |
| B27 | **`crew_runner_10_tasks_parallel_4_{quiet,watched}` measure the global heap's position, not the runner** (found 2026-10-04, at 2.1.6) | Easy–medium | In `benches/orch.bcyr`, with the traced block before them (`run_crew_10_tasks_*_traced`, which starts and stops an OTLP exporter) the 2.1.5 tag reads ~420 µs; with that block removed, ~507 µs — and 2.1.6, whose exporter no longer leaks there, reads ~505 µs either way. Interleaved A/B builds ruled out job false sharing, the data-section layout and the exporter's timeouts. **Fix:** run these rows first, or in their own `.bcyr`, so a change elsewhere in the file cannot move them; then rebase their history. Related: **B20**. |
| B28 | **18 log literals whose declared length is not the literal's** (found 2026-10-04, at 2.1.6) | Easy | 17 cut the message short — mostly a multi-byte character (`—`) counted as one byte. Under `src/`: `fleet/discovery.cyr:141`, `fleet/environment.cyr:263,271,273,279,283,287,290`, `main.cyr:211,378,383,412`, `orchestrator/pubsub.cyr:248`, `orchestrator/scoring.cyr:116`, `server/output_filter.cyr:292`, `server/sse.cyr:352` — and `main.cyr:434` declares the NUL (28 for a 27-byte literal), so the shutdown line ends `gracefully\u0000` in the JSON log. agnostic's `scripts/check-log-lengths.py` finds them all when run from this repo's root. **Fix:** fix the 18 and add that script to `scripts/check-clean.sh`, as agnostic did, so none comes back. |
| B29 | **`tests/smcyr/llm_live.smcyr` does not compile** (found 2026-10-04, at 2.1.6) | Easy | It includes `src/llm/mod.cyr` without `src/telemetry/mod.cyr` (seven undefined `agnosai_genai_*` / `agnosai_telemetry_*` functions since ADR 017 put span sites in the chat path) and, since 2.1.6, without `src/arena_pool.cyr`; it also fails `cyrius fmt --check`. A live-gateway smoke test, so `cyrius tests` does not run it and nothing noticed. **Fix:** add both includes and format it. |
| B30 | **Oracle flaws reproduced before the "never copy a Rust flaw forward" rule** (recorded at the fleet port, 2026-08-08; the rule is `CLAUDE.md:121`, added at 2.0.10) | Easy each | Four behaviours that read like defects were reproduced from the oracle and pinned by assertions so nobody would "fix" them by accident: (1) **`fleet/gpu`: allocating twice for one task id leaks the first allocation** — `allocate` ends with a map insert that replaces, so the first allocation stays charged to its device with no record and `release` can never return it (`src/fleet/gpu.cyr:24-31`, which points here); (2) **`FleetCoordinator`'s `max_retries` allows one retry fewer than it names** — `task_failed` increments first and then tests `count < max_retries`, so the default 3 yields two Retry answers and Exhausted on the third failure (`src/fleet/coordinator.cyr:7-12`; the oracle's own tests pin both ends); (3) **`topology_score` is not clamped to the 1.0 its doc promises** — a raw link count over the device-pair count, so three links between two GPUs score 3.0 (`src/fleet/topology.cyr:10-16,149`); (4) **`FederationManager::declare_coordinator` adopts any term that is not stale** (`src/fleet/federation.cyr:19-26`). `CLAUDE.md:121` now says never copy a Rust flaw forward and never propose "restore parity" as an option. **Fix direction:** fix each, invert its pinning assertion, and add an ADR where the change is visible on the wire. Look at (4) before changing it: accepting a higher term is the usual leader-election rule, and its header already says not to change the guard to `<=` without an ADR; if it is correct as it is, say so in place and drop it from this row. |

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
| sandhi | **Filed 2026-10-03, open:** `2026-10-03-chunked-response-verbs-discard-send-result.md` — `sandhi_server_send_chunk` and the chunked start/end verbs return 0 for a client that has gone, so B8's SSE stream cannot notice a departed client; reproduced on 6.6.14 (sandhi 1.10.4). Reaches agnosai only with a cyrius release that refolds sandhi. **Not filed:** `backlog` is silently ignored by `run_opts`/`run_async` (only `run_pooled`, `run_tls` and `run_pooled_tls` read it); the chunked start hardcodes `" OK"`; **inbound** chunked decoding is unsupported (answers 501 — honest, but not support; B8, B16). |
| sigil | **Not filed, not probed (re-check against 3.13.9 after the 6.6.15 move, B16 rule).** At the pinned 3.13.7 the asymmetric stack (RSA sign and verify, PSS, the bignum engine) has been stack-local since 3.12.9, but the symmetric/EC scratch — sha256/512, hmac, hkdf, aes-gcm, chacha20-poly1305, ecdsa, ed25519, x25519 — is still `cbank()`-banked per lane (e.g. sha256's message schedule, `var Wb = &W + cbank() * 512`, `lib/sigil.cyr:6419`), and lanes are never released. The bound is 63 *lifetime* crypto-touching threads, which agnosai passes: 100 pool workers (JWT sha256, `AGNOSAI_SERVE_WORKERS`, `src/server/serve.cyr:149`), plus one detached thread per crew (the audit chain's `hmac_sha256`, `src/orchestrator/audit.cyr:210`, and outbound TLS to hoosh), one per A2A callback, and the exporter. Past it, two threads can share a lane. sigil calls the result fail-closed (`lib/sigil.cyr:5286-5317`): a corrupted digest or a failed handshake, not a forged accept — which here could still mean a corrupted audit-chain HMAC or a spuriously failed outbound handshake. sigil offers `crypto_banks_exhausted()` for consumers to poll at steady state; nothing in `src/` calls it. **Next step:** probe it (poll after the pool is up, and under a crew load), then decide between polling, capping, or a sigil filing. `tests/server_auth_lane_race.tcyr` stays as the RSA lane-bypass regression guard. |
| hoosh | **To file (found 2026-10-03, F6).** hoosh 2.7.1 extracts an inbound `traceparent` strictly (`src/lib/trace.cyr:99-141`, `src/main.cyr:124-127`), but exports its own server span with the inbound **parent-id as its own spanId** and no `parentSpanId` (`src/lib/otlp.cyr:72-77`), then forwards the same traceparent to providers. **The ask:** mint a fresh span id for hoosh's span, emit the inbound parent-id as its `parentSpanId`, and forward a traceparent carrying hoosh's OWN span id. **Then agnosai's half:** `agnosai_hoosh_chat_in` adds an outbound `traceparent` header built from its `chat` span's context (`agnosai_otlp_span_context_format_a` already exists), and hoosh's span becomes the chat span's child. Until hoosh changes, sending the header would give hoosh's span the same id as agnosai's `chat` span inside one trace, so agnosai deliberately sends none ([ADR 023](../adr/023-genai-semconv-spans-and-w3c-trace-context.md)). Not filed from this lane: the upstream filing is a separate step. |
| kavach | **Two open (2026-10-03), both reaching agnosai's sandbox.** `2026-10-03-pinned-exec-breaks-uutils-coreutils.md`: kavach execs a pinned fd, and uutils coreutils (Ubuntu's default from 25.10) refuse that exec, so `/bin/echo`, `cat` and the rest exit 1 under kavach on such a host (44 of kavach's suite fail natively on an Ubuntu 26.04 Pi; 3.13.1 and 3.13.2 alike). `2026-10-02-basic-seccomp-kills-native-tls-writes.md` (read from kavach's source by its filer, not traced): the `basic` profile, which agnosai applies to every process-isolated tool, allows no `sendto`, and cyrius 6.6.14's native TLS writes with it, so a sandboxed native-TLS writer on an inherited socket would be killed (a decision for kavach). |
| bote | **To file (found 2026-10-07).** The client half of MCP: sending `initialize`, `tools/list` and `tools/call` to a server named in bote's `HostRegistry`, whose entries can already declare a `tools/call` capability behind its SSRF guard. F1's registry needs it for tools from external MCP servers (F1, harness rule 4). bote 3.3.16, the pin, is the server half only — dispatch, six transports, sessions. agnostic records the same ask (its `roadmap.md`, *Upstream*). |

**Recorded by cyrius 6.6.16 (2026-10-05) — what that release changes here.** ⛔ Nothing to do until cyrius
6.6.16 is tagged and out; none of it needs an agnosai code change. Fold into the pin move after 6.6.15.

- **sandhi row — the chunked-verbs filing reaches agnosai at 6.6.16.** cyrius 6.6.16 folds sandhi **1.10.7**
  (1.10.5 and 1.10.6 were never folded on their own): the one-shot and chunked response verbs report their
  write results (B8's SSE stream can notice a departed client), and concurrent requests no longer run under
  each other's TLS policy hook. Re-check B8 / B16 against it at the bump.
- **kavach row — a correction to the `basic`-profile reach.** cyrius 6.6.16's review of kavach 3.13.2
  (recorded in kavach's `2026-10-02-basic-seccomp-kills-native-tls-writes.md`, 2026-10-05) found that the
  `basic` POLICY never selects `security_create_basic_seccomp_filter`: every spawn path loads the deny-list
  `security_create_exec_seccomp_filter`, which allows `sendto` / `poll` / `fcntl`, and the policy's
  `seccomp_profile` string is never read to pick a filter. So agnosai's process-isolated tools are NOT killed
  by it; only a process that loads the basic filter on itself is (from 6.6.16 also on a plain socket write).
- **Channels (cyrius thr-1).** `src/arena_pool.cyr`'s `chan_try_recv` / `chan_try_send` never wait, so their
  semantics were already right everywhere, but before 6.6.16 the ring was unlocked on arm64 macOS — a data race
  under real threads; 6.6.16 locks it. `src/llm/inference_queue.cyr:223`'s per-request `chan_new(1)` reply
  channel now really waits on Windows (it answered 0 at once before); it holds no kernel handle per channel —
  a waiting call borrows an I/O completion port from a small process-wide pool (measured on cass: 400 reply
  channels waited on once each, 84 → 85 process handles). Channels are still never freed (no `chan_free`;
  bump bytes only). Every peer now exports `CHAN_BLOCKING`, and `THREADS_CONCURRENT` reads 1 on Windows.
- **SIGPIPE (cyrius CVE-74).** Every `lib/net.cyr` socket write (`src/server/serve.cyr:625`'s `sock_send`
  included) uses `sendto(…, MSG_NOSIGNAL)` on Linux and `SO_NOSIGPIPE` on macOS, so a client that resets
  mid-write cannot kill the server through a plain socket.

**C2 — one local workaround owed for deletion.**
Not a defect; it is code that exists only because an upstream API could not express
the thing, and it has a precise deletion condition. Recorded here because this
list is the project's definition of "owed", and a temporary workaround that no
list names is a permanent one.

| Workaround | Where | Delete when |
|---|---|---|
| `_agnosai_signal_default` | `src/sandbox/spawn.cyr:518` | cyrius ships `signal_default` (filed `2026-08-05-syscalls-has-signal-ignore-but-no-way-back-to-sig-dfl.md`). ~20 lines, Linux arms only; the call site in the child stays, only the local definition goes. ⚠ **The condition is met:** `signal_default` is in the stdlib from cyrius 6.5.7 (kavach 3.13.2's notes; `lib/syscalls.cyr:171` at 6.6.14), so the deletion is unblocked. |

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

## Recorded by cyrius 6.6.17 (2026-10-05) — for the next cyrius pin move

⛔ **Nothing to do until cyrius 6.6.17 is tagged and out.** Docs-only note from the cyrius 6.6.17 lanes; each item
is this repo's to adopt when it pins ≥ 6.6.17. Nothing here gates a cyrius release.

- ⚠ **This CAN be a new red at the 6.6.17 pin bump** (cyrius t1). CI runs `lib sync` → `deps` with no final
  `deps --verify`; before 6.6.17 `lib sync` never looked at the lock, so a lock committed after a build-first
  pin move went unchecked. That lock carries the previous pin's rows for files `deps` does not vendor, and
  from 6.6.17 `lib sync` refuses them by name. After a pin move run `cyrius lib sync --full` before committing
  the lock, or `cyrius lib sync --full --relock` if `deps` / `build` already ran under the new pin.

## Recorded by cyrius 6.6.19 (2026-10-06) — for the next cyrius pin move

⛔ **Needs cyrius >= 6.6.19 — do not bump the pin until 6.6.19 is tagged and out.** Docs-only note from the cyrius
6.6.19 lanes; each item is this repo's to adopt when it pins ≥ 6.6.19. Nothing here gates a cyrius release.

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
