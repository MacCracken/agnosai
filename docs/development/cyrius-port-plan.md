# AgnosAI — Rust → Cyrius Port Plan (v2.0.0)

> Status: **proposed** — not started. No code written, no tree changes.
> Baseline: Rust v1.1.0, 30,692 LOC, GREEN (`cargo fmt --check` clean,
> `cargo check --all-targets` passes). That green tree is the parity oracle.
> Method note: every capability claim below was verified by **reading the
> module**, not by grepping Rust crate names. See "Corrections" at the end.

## Verdict

This is a **port, not a rewrite** — but three of the ten groups need genuine
redesign (server transport, orchestrator concurrency, sandbox exec), and the
single biggest risk is not the language. It is that **agnosai would be the first
heavy production user of `sandhi_server_run_pooled`** (its only exercise today is
a 4-worker probe; daimon and mneme both still use single-flight `run_opts`).

Scale: ~122 commit-sized bites across 10 groups.

## The concurrency decision

**tokio → `sandhi_server_run_pooled` / `_pooled_tls` + `sandhi_server_router_handler`.**

Not the `lib/async.cyr` epoll reactor: it requires poll-structured, re-entrant
handlers (async.cyr:246-252, "resuming FROM THE TOP when woken"), which would
mean hand-rewriting every agnosai handler as a state machine. And sandhi's own
`run_async` does a blocking recv internally (`_sandhi_server_async_handler`,
:13020-13038) — it batches accepts but gives **zero handler concurrency**. For
handlers that block for seconds on hoosh LLM calls, a 128-conn batch serializes
to minutes.

Not hoosh's hand-rolled work-queue + 7-thread pool either. hoosh built that
**only** because it needed a heterogeneous job queue (JOB_CONN / JOB_BATCH_ITEM /
JOB_OTLP_EXPORT sharing one pool + crypto budget, pool.cyr:16-18). agnosai serves
plain HTTP conns, so sandhi's pool is a drop-in. **daimon delegates fully to
sandhi (server.cyr:183) — follow daimon, not hoosh.** hoosh's loop is also
strictly worse: a single `sock_recv` with no Content-Length loop (truncates on
TCP segmentation) and 64 KiB leaked per conn.

Handler shape `fn(ctx, cfd, req_buf, req_len)` is identical across run/run_async/
run_pooled (sandhi.cyr:13419-13420), so it maps 1:1 onto agnosai's axum Router.
Keep `lib/async.cyr` in scope for **client-side fan-out only**.

## Blockers (must be resolved or consciously accepted)

| # | Blocker | Detail |
|---|---|---|
| 1 | **sandhi 64 KiB body cap, silent truncation** | `HSV_REQ_BUF_SIZE = 65536` (sandhi.cyr:12724); `recv_request` falls out at max and `return have` — no 413, the string "413" does not appear in sandhi.cyr. Handler gets a truncated body + a Content-Length claiming more. agnosai's `MAX_BODY_BYTES = 10 MiB` (server/mod.rs:39) is **160x**. Cannot just raise it: it sizes the per-conn arena (128 × 10 MiB = 1.28 GiB). |
| 2 | **bayan JSON has no recursion-depth cap** — ✅ **resolved in bayan 1.1.1** | ~~`_JP_STATE_SIZE = 40` = 5 slots, no depth counter; `_jp_parse_value` recurses freely.~~ bayan 1.1.1 caps both descents at 128 (`_JP_MAX_DEPTH`, serde_json wire-parity); past the cap the parse fails `"nesting too deep"` through the per-call error path. Picked up by re-pinning bayan ≥ 1.1.1 (or the next cyrius `lib/bayan.cyr` refold). serde_json defaults to 128. A nested body on POST /crews, /a2a/receive, /mcp stack-overflowed the worker — a **parity regression**, now closed. |
| 3 | **`sandhi_router_dispatch` allocates per request, never frees** | Uses non-arena accessors on the global bump allocator; sandhi's own comment (:13359-13365) defers this. Unbounded RSS growth per request on a long-running server. |
| 4 | **`chan_try_send` does not exist** | `chan_try_recv` exists (thread.cyr:409); `chan_send` blocks (:331/:352). Breaks non-blocking `let _ = tx.send(..)` (crew_runner.rs:115,786) and majra pubsub fan-out (one wedged SSE subscriber stalls every topic). |
| 5 | **`SandboxPolicy` name collision** | agnosai `policy.rs:39` vs kavach `policy.cyr:7`. One flat namespace, both `#derive(accessors)` → both emit `SandboxPolicy_set_*` → last-definition-wins → **silently corrupted strength scores**. Rename agnosai's to `AgnSandboxPolicy`. |
| 6 | **No WASM runtime anywhere** | Verified 3 ways across 138 repos. kavach shells out to the `wasmtime` CLI (not installed on this box); `backend_is_available(WASM)` hardcodes `return 0` (a one-line bug — every sibling probes via `which_exists`). Defers wasm.rs + wasm_tool.rs + wasm_loader.rs. |
| 7 | **Live Rust bug, fix first** | `cargo check --no-default-features --features kavach` **FAILS today**: crew_runner.rs:199 gates on `kavach` but reaches into `crate::sandbox::`, which lib.rs:27 gates on `sandbox`. Only `full` hides it. The baseline must be green before it becomes the oracle. |

## Phases

Dependency-ordered. Every phase decomposes into commit-sized bites (each
independently compilable + testable).

### Phase 0 — Scaffold
`cyrius port` (Rust → `rust-old/`, scaffolds src/main.cyr + cyrius.cyml).
Ordered stdlib array (order matters — the resolver falls back to array order:
bayan before sigil; mmap before dynlib; dynlib/fdlopen/mmap before tls+sandhi —
see thoth/cyrius.cyml). Vendor `dist/bote-core.cyr` + `dist/majra.cyr` under
`src/vendor/` (NOT `[deps.bote]` — it recurses into libro+majra and collides).
`[deps.ai-hwaccel]`, `[deps.mneme]` (uuid), `[deps.tyche]` (RNG).
**Exit:** hello-world builds; `cyrius deps` resolves clean.
**First, though:** fix the `--features kavach` Rust bug so the baseline is green.

### Phase 1 — Beachhead: `learning` + `core`
Both are blocked by nothing. `learning` is the ideal first real code: **zero
coupling** (grep for its types outside src/learning/ returns empty), zero async,
zero serde, zero traits, zero I/O — and it exercises tyche, f64-as-bit-patterns,
and the .tcyr harness with no downstream risk. Then `core` — the root of the
graph, pure data, the vocabulary every other group speaks.
**Exit:** learning + core tests green against rust-old/ as oracle.

### Phase 2 — `llm` (the hoosh seam)
Cheapest group in the port; the reference implementation already exists
(thoth/src/hoosh.cyr). router + retry + seam client. llm **defines** the types
orchestrator consumes (ProviderType/InferenceRequest/Message) so it lands early.
**Exit:** a live chat-completion round-trip against `hoosh serve 8088`.

### Phase 3 — `tools`
native, registry (+ mandatory registry mutex, since run_pooled makes every worker
a thread), remote_registry, builtin/*. Defers python_tool/wasm_tool.
**Exit:** 94 of 115 tests green.

### Phase 4 — `orchestrator`
16 of 18 bites. The default-feature runtime path: orchestrator → crew_runner →
scoring/scheduler/output_validation, + approval, budget, audit, memory,
multi_tenant, plan_cache, durable_state (→ patra), hierarchical.
**Exit:** a crew runs end-to-end headless.

### Phase 5 — `server`
22 bites. Start with the pure leaves (ssrf → prompt_guard → output_filter →
prometheus — string/number work, independently testable). Then auth
(shared-secret half first; JWT half waits on sigil `pem_decode_pubkey` + bote
`jwt_verify_rs256`), hot_config, sse/EventBus, routes/*, router, main.
**Exit:** the 11-route API serves; SSE streams; load-tested (blocker #3 measured).

### Phase 6 — `sandbox` (77%)
policy (rename first — blocker #5), kavach_bridge, exec, process, python, oci,
manager. **Defer wasm.rs.** Note: kavach's lossy one-shot `exec_capture` blocks
nothing here — agnosai only consumes kavach's **scoring + gate**, which port 1:1.
Use `persistent_spawn/send/read/terminate` for the stdin-JSON tool protocol.

### Phase 7 — `fleet`
Cheapest 4,443 LOC in the port; zero consumers; sequence last. **Premise
correction:** `discovery.rs` does *not* map to `sandhi_discovery_*` (sandhi
resolves ONE service by name, no enumerate, no metadata map) — it's 174 lines of
stub needing no sandhi at all.

### Phase 8 — `telemetry` (partial)
Copy hoosh's proven `otlp.cyr` (199 lines) — this is the one place the remote
seam does NOT apply (OTLP export is in-process; a remote gateway cannot export
agnosai's spans). Thread-local trace context is **mandatory** under run_pooled
(sakshi's span stack + trace id are process globals, sakshi.cyr:1400-1407).

### Phase 9 — `definitions` (partial)
assembler, loader-JSON, presets, versioning, k8s_crd (which parses **JSON** only
— the ```yaml at k8s_crd.rs:33 is a doc comment). **Defer** ZIP container +
packaging + YAML.

## Parity definition — what v2.0.0 is

**Ships** (the whole default cargo build — `default = []` → core + orchestrator +
llm + tools + server + learning), plus fleet, plus 77% of sandbox, plus
JSON-only definitions, plus OTLP telemetry.

**Excluded, with reason:**
- **bhava / `personality`** — not ported (user decree, post-v2).
- **WASM**: wasm.rs, wasm_tool.rs, wasm_loader.rs, the tool SDK, examples/wasm-tools/ — no runtime exists.
- **definitions ZIP + YAML** — both behind the non-default `definitions` feature; both are upstream filings.
- **genai.rs, inference_queue.rs** — zero consumers; pending sign-off.

**Wire parity is the bar**, judged against `rust-old/`.

## Test strategy — do NOT copy hoosh

hoosh's "~71% retention" is **71% of test count against 170 locally-redefined
MIRRORS** of production logic (only 1 of 30 src/lib modules is included). The
mirrors reuse production symbol names, so the real modules can **never** be
included later (last-definition-wins, silently). Real modules **are** includable
— proved by probe.

- **Per-module `.tcyr`** (`cyrius tests` walks recursively; 464 tests compile+run in 2.46s).
- **Ban mirror-defining**: any `fn` in a .tcyr duplicating a src/ symbol is a defect. Enforce by CI grep.
- Inline `#[cfg(test)]` mods testing private fns are a **non-problem** — Cyrius has no module privacy.
- `.bcyr` goes in `benches/` (the only layout `cyrius bench` no-arg discovers).
- **Build our own coverage.** `cyrius coverage` is hardcoded to lib/ + tests/tcyr → measures the vendored **stdlib**, never src/ (7/1097 in hoosh). CLAUDE.md's 80% gate cannot be discharged by it.
- **Exit-code clamp hazard**: `sys_exit(assert_summary())` & 0xFF → exactly 256/512/768 failures scores **PASS**. agnosai has 865 tests. Clamp to 1.
- 155 of 865 tests are `#[tokio::test]` and Cyrius has **no async test harness**.
- **Freeze** the 124-row Rust bench-history.csv; start a fresh Cyrius baseline. tokio numbers are not comparable — do not claim wins/regressions across the port.

## Upstream filings (user files; never `gh` — curl to the GitHub API)

| Repo | Ask |
|---|---|
| bayan | ✅ **both filed 2026-07-16** — YAML parse → the existing tagged value tree: `bayan/docs/development/issues/2026-07-16-agnosai-yaml-parse-into-tagged-value-tree.md` (also accepted onto bayan's roadmap as `bayan_yaml_*`; the "draft written" here never materialized as a file — the filing supersedes it); JSON recursion-depth cap (blocker #2): `bayan/docs/development/issues/2026-07-16-agnosai-json-no-recursion-depth-cap.md` — ✅ **resolved in bayan 1.1.1** (cap 128, serde_json parity; 101/101 asserts green) |
| sankoch | ZIP archive container (deflate + crc32 already there; ~250 lines) |
| cyrius | `chan_try_send` (blocker #4); O(n log n) sort (every ecosystem sort is O(n²) — agora/board.cyr:805-808 explicitly invites the bite "if a consumer reports a perf concern"; load_testing.rs p50/p95/p99 over 100k entries is that consumer) |
| kavach | WASM availability one-liner; stderr capture; **exec timeout — an undocumented regression** (Rust 2.0.0 shipped it, the Cyrius port dropped it, ADR-004 omits it) |
| sigil | `pem_decode_pubkey` — a ~20-line clone of `pem_decode_privkey` |
| bote | `jwt_verify_rs256` — **its stated premise is stale**: jwt.cyr:9-11 says "RS256 needs an asymmetric primitive sigil doesn't yet expose", but sigil.cyr:17642 has `rsa_pkcs1v15_verify_sha256` and :17704 already accepts SPKI |
| majra | pubsub holds the hub mutex across blocking sends; relay's file-scope globals make `relay_receive` non-reentrant (its own comment says they're vestigial) |
| sandhi | body cap/413 (blocker #1); per-request allocation (blocker #3); `backlog` silently ignored by run_opts/run_async; chunked start hardcodes " OK" |

## Corrections to earlier claims (recorded so they are not re-derived)

- **"No HTTP server in Cyrius"** — FALSE. sandhi is a 14,171-line HTTP/2 stack with 65 `sandhi_server_*` fns. The claim came from grepping `http_server_run|http_server_new|http_serve`.
- **"kavach is not functional / carries 1 of 7"** — FALSE. 422 tests pass. It has two exec paths; the subagent scored only the lossy one. `persistent_*` has real live stdin+stdout pipes, SIGKILL+reap, pre-exec safety — live-tested against /bin/cat. And agnosai only consumes kavach's scoring+gate anyway.
- **"kavach has 12 backends"** — FALSE. 10 (`BACKEND_COUNT = 10`).
- **"No broadcast/fan-out"** — FALSE. majra pubsub fans out to per-subscriber channels; `fleet/relay.rs` maps onto majra `relay_*` near-1:1 (identical fields, API, and doc language).
- **"sigil has no SPKI decoder"** — FALSE. `rsa_pubkey_from_der` (sigil.cyr:17696-17703) explicitly accepts both PKCS#1 and X.509 SubjectPublicKeyInfo. Only the `-----BEGIN PUBLIC KEY-----` PEM label pair is missing.
- **"No ranged RNG"** — FALSE. tyche (`rng_uniform`/`rng_normal`/`rng_seed`). But **never for the audit key** (orchestrator.rs:63) — tyche is not a CSPRNG; that stays `random_bytes()`.
- **"No sort"** — half-false. `itihas/src/util.cyr:57 vec_sort(v, cmp)` exists but is O(n²), and is **unprefixed** → collides under the fold. Copy the 17 lines locally.
- **"Arc is a real gap"** — dissolves. 52 `Arc<` in src/, of which 49 are process-lifetime shared-immutable state = a raw pointer in an arena. Only 3 are `Arc<RwLock|Mutex>` → futex mutex + `async_rwlock_new`.
- **`cyrius coverage` / `cyrius bench`** — structurally unusable as-is (see Test strategy).

## Open questions (need a decision before the bites they gate)

1. **Money representation** — recommend integer micro-USD (per hoosh pricing.cyr:6-9). Round-trip wire-compatible but textually `0.002500` vs serde's `0.0025`. Gates core BITE 8 and every downstream cost path.
2. **`"personality": null`** — bhava is a *hard* dep today, so the default Rust build emits an explicit null. Emit the literal to keep byte-exact default-build wire parity? This is the line between "bhava deferred" and "the wire changed."
3. **Drop the ~20 zero-consumer hoosh re-exports** (src/llm/mod.rs) as a v2.0.0 Breaking change — and with them `inference_queue.rs`, `genai.rs`, and router's `suggest_quantization`/`estimate_model_memory` (all zero-consumer)?
