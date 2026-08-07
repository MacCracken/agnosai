# AgnosAI — Rust → Cyrius Port Plan (v2.0.0)

> Status: **Phases 0-6 complete** — `server` finished 2026-08-03 (the API serves,
> streams and drains) and `sandbox` finished 2026-08-05 (all 43 findings of the
> 2026-08-04 audit fixed, 41 mutation-verified). Remaining: Phases 7-9.
> Pinned **cyrius 6.5.10** (2026-08-07).
>
> ⚠ **Read the Phase 7-9 headings before scheduling them.** Measured against the
> parity bar defined below — the **default cargo build**, and `default = []` —
> `fleet` and `definitions` are `full`-feature-only and are therefore *not*
> default-build gaps at all. Only part of `telemetry` is. That materially changes
> what "remaining" means, and it is recorded at each phase.
>
> **All eight numbered blockers are closed, and nothing blocks anything.** The
> table below is kept as a *reasoning archive*, not a work list — several of its
> verdicts shaped designs that are still live, and re-deriving them costs more
> than reading them. **A `blocker #N` citation in `src/` points here**; it is a
> footnote to a closed analysis, never an open issue.
>
> | # | Closed by |
> |---|---|
> | 1 sandhi body cap | sandhi 1.9.4 |
> | 2 bayan JSON depth cap | bayan 1.1.1 |
> | 3 per-request allocation | sandhi 1.9.7 (transport) + **bayan 1.4.0 / cyrius 6.5.5 (handlers)** — the handler half was the last piece and it is measured at 0 B/response for the `core` group |
> | 4 `chan_try_send` | cyrius 6.4.84; `src/chan_lossy.cyr` gives evict-oldest on top |
> | 5 `SandboxPolicy` collision | downgraded to hygiene, rename still owed at M7 |
> | 6 tool sandbox | decided — cx + kavach, [ADR 006](../adr/006-cx-tool-sandbox.md) |
> | 7 live Rust bug | fixed 2026-07-28; the oracle is valid |
> | 8 no O(n log n) sort | cyrius 6.5.4 shipped `vec_sort_by`/`vec_select_nth`; `src/order.cyr` consumes them |
>
> **Live owed work is not here — it is in [`roadmap.md`](roadmap.md)** under
> *Owed work*. This file is the plan of record and the reasoning archive; the
> roadmap is the schedule.
> Original 2026-07-28 framing follows, kept because the reasoning must not be
> re-derived: baseline greened, blockers re-verified against cyrius 6.4.83, two
> upstream fixes shipped.
> Baseline: Rust v1.1.0, GREEN — `cargo fmt --check` clean,
> `cargo clippy --all-features --all-targets -D warnings` clean, all 9 feature
> combinations compile, **863 + 2 + 1 tests pass**. That green tree is the
> parity oracle.
> Method note: every capability claim below was verified by **reading the
> module**, not by grepping Rust crate names — and on the 2026-07-28 pass every
> "resolved" verdict was independently refuted before being accepted, with the
> JWT and sort findings confirmed by *compiling and running* code against the
> real toolchain. See "Corrections" at the end.
>
> **Done so far:** blocker #7 fixed (the Rust tree is now a valid oracle);
> blocker #1 fixed upstream in **sandhi 1.9.4** (three silent-truncation paths,
> not one); blocker #4 shipped in cyrius 6.4.84; blocker #5 downgraded to
> hygiene; blocker #8 (sort) discovered and quantified; blocker #6 **resolved by decision** — the tool sandbox rides **cx** +
> kavach, recorded in ADR-006. Blocker #3 is **closed** as of the cyrius 6.5.2
> fold-in (2026-07-29); it was previously the one genuinely blocking item, with a
> known agnosai-side mitigation.

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

## Blockers — all closed (reasoning archive)

> **Every row below is resolved.** Nothing in this table gates work; it is kept
> because the *reasoning* behind several still-live designs is here and nowhere
> else — the evict-oldest channel semantics, the per-request arena invariant,
> the cx-is-not-a-sandbox finding. Treat it as the footnotes for
> `blocker #N` citations in `src/`.
>
> **Re-verified 2026-07-28 against cyrius 6.4.83** by reading each module (not by
> grepping crate names), with every "resolved" verdict independently refuted
> before it was accepted. Status column records how each closed; the Detail
> column keeps the original finding verbatim.

| # | Blocker | Status | Detail |
|---|---|---|---|
| 1 | **sandhi 64 KiB body cap, silent truncation** | ✅ **FIXED upstream in sandhi 1.9.4** | `HSV_REQ_BUF_SIZE = 65536`; `recv_request` fell out at max and `return have` — no 413. The constant `HTTP_PAYLOAD_TOO_LARGE = 413` was *declared* with **zero references**; nothing ever sent it. Reading the fn turned up **two more** silent paths: a peer that hangs up mid-request returned the partial bytes as if whole, and `Transfer-Encoding: chunked` with no Content-Length slipped past both smuggling guards, collapsing `need_body` to the header terminator so the handler ran with an **empty body** while the chunked bytes sat unread. 1.9.4 gives each a distinct sentinel (`ERR_TOO_LARGE` / `ERR_INCOMPLETE`) answered **413** / **400** / **501**, adds `sandhi_server_options_max_request` so the cap is policy (default unchanged at 64 KiB), and drains briefly before closing a refusal so the status is not destroyed by an RST. **Correction to the original entry:** the 1.28 GiB figure was attributed to the wrong function — `run_pooled` has no arena, each worker holds one reusable buffer, so cost is `cap × workers` and workers is already tunable. `run_async` is the one that eagerly allocates `max_conns`-sized. 10 MiB bodies cost ~80 MiB at 8 workers. |
| 2 | **bayan JSON has no recursion-depth cap** | ✅ **resolved (bayan 1.1.1, folded)** | Confirmed present in the **folded** `~/.cyrius/lib/bayan.cyr`, not just upstream: `_JP_MAX_DEPTH = 128` (:3304), depth counter at `ps+40`, guards in **both** recursive descents — the tree parser `_jp_parse_value` (:3539) and the streaming parser `_js_parse_value` (:4059). Failure is a retrievable per-call error, not a crash. **Residual:** the *serializer* `_jb_walk` (:3761-3826) recurses on `JTAG_ARR`/`JTAG_OBJ` with **no cap** — uncapped upstream too, so it is not a fold lag. Only reachable from a tree agnosai itself built, so it is not an untrusted-input path; do not build unbounded-depth values from user data. |
| 3 | **`sandhi_router_dispatch` allocates per request, never frees** | ✅ **CLOSED 2026-07-31** — transport half by sandhi 1.9.7, handler half by bayan 1.4.0 (cyrius 6.5.5). Original finding kept below. | Confirmed. But bypassing the router alone buys ~32 B/req out of a **~700–4096 B/req** leak: the whole bare `sandhi_server_*` accessor + `_send_*` family allocates on the global bump. Mitigation is agnosai-side and needs no upstream change: write our own handler on the raw `fn(ctx, cfd, buf, blen)` signature, take a per-worker arena, and use the `_a` variants **everywhere**, with one `reset_via` exit path. Invariant: nothing outliving the request (registry, router table, config) may come from the arena. **Residual we cannot close from a handler:** `_sandhi_server_pool_worker` itself calls bare `sandhi_server_send_status` on the smuggling rejects (~176 B/req before our handler runs) — a malformed-request flood at 10k req/s leaks ~1.76 MB/s. If internet-facing, replace `run_pooled` with a ~40-line accept loop (every primitive is in the fold); if behind a reverse proxy, accept it. Regression test: assert `alloc_used()` delta ≈ 0 over N thousand requests. |
| 4 | **`chan_try_send` does not exist** | ✅ **SHIPPED in cyrius 6.4.84** | Was absent on all three backends. Filed 2026-07-28, shipped the same day: `chan_try_send(ch, val)` on `thread.cyr:409` / `thread_win.cyr:112` / `thread_agnos.cyr:116`, uniform contract **0** enqueued / **−1** closed / **−2** full, gated by a 20-assertion `vr01_` test on real hardware. The fix also uncovered that `chan_try_recv` and `chan_close` had been raising SIGSYS on macOS since forever — the channel fns called `SYS_FUTEX` directly, bypassing the macOS mutex backend; all three now route through `_chan_wake`. agnosai is pinned to 6.4.85. **Important correction to the mapping:** every `let _ = tx.send(..)` in agnosai is a tokio **broadcast** send, which never blocks and never fails — a lagging subscriber gets the ring's *oldest* overwritten and reads `Lagged(n)` (sse.rs:266-279). So the Cyrius equivalent is **not** "drop the newest when full", it is "**evict the oldest**". A naive `tx.send` → `chan_send` port converts never-block-lossy into block-forever, the worst possible regression for SSE fan-out. Port as a private `agnosai_chan_push_lossy` built **on top of** `chan_try_send` — the primitive gives "drop the newest when full", agnosai needs "evict the oldest", so on `-2` advance `head` and retry, keeping a lag counter. **Our filing was wrong on one point:** it proposed aliasing `chan_try_send` to `chan_send` on agnos/Windows since those never block. They return `-1` on FULL and never inspect `closed`, so that alias would have made closed and full indistinguishable on exactly those backends. Upstream wrote all three out properly instead. **Downstream status:** the majra half is **fixed in majra 2.5.3** (2026-07-28) — publish now snapshots each subscriber list under one short lock and walks it unlocked, so a lagging subscriber no longer freezes unrelated topics. 2.5.3 also closed two silent data-loss races in `pubsub_subscribe` and `mq_enqueue` (unlocked `fl_alloc` handing two subscribers the same block). agnosai is pinned to 2.5.3. |
| 5 | **`SandboxPolicy` name collision** | 🟢 **downgraded to hygiene** | Not reachable as filed. The two structs' **field names are disjoint**, so no accessor symbol actually collides — and 6.4.83 *warns* on duplicate definitions rather than silently taking the last. Also, agnosai's strength lives on `IsolationLevel` (policy.rs:30-33), which has no kavach counterpart at all. Still do the `AgnSandboxPolicy` rename: one-line edit, cheap insurance, and the safety margin here should not depend on two type definitions staying disjoint by accident. |
| 6 | ~~No WASM runtime anywhere~~ → **tool sandbox rides `cx`** | ✅ **DECIDED — see [ADR-006](../adr/006-cx-tool-sandbox.md)** | The earlier verdict conflated the *format* with the *capability*. WASM is an explicit cyrius non-goal, but what those 955 lines buy is **sandboxed portable execution of untrusted tool code**, and the **cx** arc — opened because a consumer "hit the wasm-shaped wall" — ships that today. Verified: `cycc_cx` compiles `.cyr` → `.cyx`, `cxvm` runs it from stdin, exit code passes through (6×7 probe → 42). Both installed. **The critical finding: `cxvm` is NOT a sandbox** — opcode `0x70` dispatches guest syscalls straight to the host kernel (`programs/cxvm.cyr:309-311`), with no allowlist, no filtering, no capability model. Every isolation guarantee therefore comes from **kavach** (seccomp `strict`/`basic` + landlock, both real). Filesystem/network confinement gets *stronger* (kernel-enforced); CPU metering gets *weaker* (wall-clock timeout, no fuel equivalent). **Hard requirement: no path may exec a `.cyx` outside a kavach sandbox** — an unwrapped `cxvm` spawn is a full escape. Arc-A limits, all verified: x86-Linux only, 64 KB caps, **float literals silently miscompile**, bare of stdlib. |
| 7 | **Live Rust bug, fix first** | ✅ **FIXED 2026-07-28** | `cargo check --no-default-features --features kavach` failed: crew_runner.rs:199 gated on `kavach` but reached into `crate::sandbox::`, which lib.rs:27 gates on `sandbox`; only `full` hid it. Now `all(feature = "kavach", feature = "sandbox")`. All 9 feature combos compile; fmt + clippy clean; **863 + 2 + 1 tests pass**. The baseline is now a valid oracle. |
| 8 | **No O(n log n) sort** *(promoted from "Corrections")* | ✅ **CLOSED** — cyrius 6.5.4 shipped `vec_sort_by`/`vec_select_nth`; `src/order.cyr` is now a 98-line wrapper over them. Original measurements kept below. | Measured on this box at 6.4.83: an itihas-style insertion sort over agnosai's 100k-entry percentile vector (load_testing.rs) costs **52.6 seconds**; heapsort costs 87 ms (605×); three quickselects cost ~21 ms. Vendor both `ai_sort` (iterative in-place heapsort — O(n log n) *worst* case, O(1) extra memory, no recursion depth; not stable, and no site needs stability) and `ai_select_nth` (Hoare quickselect, median-of-3) — percentiles never needed a full sort. **Naming is load-bearing:** prefix `ai_*`; never define a bare `vec_sort`, which itihas exports unprefixed into the flat namespace. Second-order: scheduler.rs:257's `BinaryHeap` has no stdlib equivalent either — build it on the same sift primitive, or just sort the (small) ready-set each round. |

## Phases

Dependency-ordered. Every phase decomposes into commit-sized bites (each
independently compilable + testable).

### Phase 0 — Scaffold
`cyrius port` (Rust → `rust-old/`, scaffolds src/main.cyr + cyrius.cyml).
Ordered stdlib array (order matters — it is the auto-include order, prepended
ahead of every unit's own includes, and resolution is single-pass, so callees
must precede callers).

**Revised 2026-07-28 — use the daimon/stiva declare-ahead pattern, NOT
vendoring.** The agnosys collision that justified hoosh's `src/vendor/` is gone,
and agnosai wants majra as a first-class dep anyway. So `bote` (core
profile), `majra`, `kavach`, `sigil`, `ai-hwaccel`,
`tyche` — each declared explicitly *even where only transitively needed*,
so `cyrius deps`' recursion resolves to **our** pins. Vendoring stays the
fallback if `cyrius deps` misbehaves (then `src/vendor/`, never a top-level
`vendor/` — cyaudit reads that as untrusted).

Three hard constraints the scaffold must honor:
- **sigil goes in `sigil` and comes OUT of the stdlib array.** kavach,
  libro and majra all pin sigil themselves, so `cyrius deps` vendors a
  `dist/sigil.cyr` regardless; leaving `"sigil"` in the stdlib array too gives
  two packagings of one version — that is daimon's 227 "duplicate fn (last
  definition wins)" warnings. But `ct`/`keccak`/`random`/`thread_local` **stay**
  in the array: they are not in sigil's bundle, and omitting one links clean
  then SIGILLs (exit 132) at first crypto use.
- **`mneme` cannot exist** — mneme has no lib block, ships no `dist/`
  bundle, and is AGPL-3.0 against our GPL-3.0-only. Reimplement UUID v4/v5 in
  `src/id.cyr` over stdlib `random_bytes` + `sha1` (~15 lines; add `"sha1"` to
  the array). Do not copy mneme's text.
- **`sakshi` and `patra` are folded stdlib at 6.4.83** — no git pin needed
  unless a shadow warning appears.

**Exit:** hello-world builds; `cyrius deps` resolves clean.
**First, though:** ~~fix the `--features kavach` Rust bug~~ — ✅ done 2026-07-28.

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
multi_tenant, plan_cache, durable_state, hierarchical.
**Correction (2026-07-29, verified while porting):** this line read
`durable_state (→ patra)`. It does not. patra is a full embedded SQL database
over its own paged `.patra` format with no dump/export verb, and its `jsonl`
mode opens `O_APPEND` with no `O_TRUNC`; `durable_state`'s contract is one
overwritable, human-readable JSON file per crew at a caller-chosen path, which
patra structurally cannot produce. It is built on `lib/io.cyr` instead, and
**no M5 module touches patra** — `grep -rn 'patra_\|jsonl_' src/ tests/` is
empty. patra stays a declared stdlib dep for later phases.
**Exit:** a crew runs end-to-end headless.

### Phase 5 — `server`
22 bites. Start with the pure leaves (ssrf → prompt_guard → output_filter →
prometheus — string/number work, independently testable). Then auth, hot_config,
sse/EventBus, routes/*, router, main.
**Exit:** the 11-route API serves; SSE streams; load-tested (blocker #3 measured).

**Correction (2026-07-30, verified by running it):** this line read *"JWT half
waits on sigil `pem_decode_pubkey` + bote `jwt_verify_rs256`"*. **Neither gate
exists.** Both were checked by compiling and executing an RS256 verification
inside agnosai against its own vendored `lib/`:

* **sigil needs nothing.** `rsa_pubkey_from_der` already accepts an X.509
  SubjectPublicKeyInfo, not just a bare PKCS#1 key — sigil's own X.509 parser
  uses that branch — and the PEM helpers (`_pem_find`, `_pem_b64_decode`,
  `_pem_match_at`) are generic over the label, which `pem_decode_privkey` proves
  by handling three of them. So `pem_decode_pubkey` is ~15 lines of agnosai-side
  glue, not an upstream feature. A first-class sigil export would be *nice*, and
  is filed there, but nothing waits on it.
* **bote is the wrong dependency entirely.** Its only JWT is **HS256** — a
  different algorithm — and `src/jwt.cyr` is in neither `[lib]` nor `[lib.core]`,
  so `dist/bote-core.cyr` (which is what agnosai pins) ships zero `jwt_*`
  symbols. agnosai calls nothing from bote today. Filed upstream; drop it from
  the ask list.
* **The topology assumption was also wrong.** sigil is a **git-tag dep with a
  local path override** (`[deps.sigil] path = "../sigil", tag = "3.12.1"`), not a
  cyrius stdlib fold — so even a real upstream change would be a tag away, not a
  toolchain release away.

Measured end to end with a real 2048-bit key and an openssl-signed token: SPKI
PEM → 294-byte DER → 256-byte modulus + 3-byte exponent → `1` for a valid
signature, `0` for a tampered input and `0` for a tampered signature.

**`auth.rs` therefore ports whole, in one module, with zero upstream change.**
Splitting it into bites is a size-discipline choice (the shared-secret half alone
unlocks 5 of the 11 oracle tests with no crypto), not a constraint. See the
blocker table below, which already said this and was not read.

### Phase 6 — `sandbox` (77%) — ✅ **COMPLETE 2026-08-05**
policy (rename first — blocker #5), kavach_bridge, exec, process, python, oci,
manager. **wasm.rs deferred**, per the parity definition below. Note: kavach's
lossy one-shot `exec_capture` blocks nothing here — agnosai only consumes
kavach's **scoring + gate**, which port 1:1. Uses
`persistent_spawn/send/read/terminate` for the stdin-JSON tool protocol.

**Exit:** all eight modules ported; the 2026-08-04 audit found **43 confirmed
defects behind a green suite** and all 43 are fixed, 41 mutation-verified — see
[m7-audit-2026-08-04.md](m7-audit-2026-08-04.md), where each is marked in place.
That audit is the strongest evidence in the port that *a green suite is not a
correctness proof*, and is worth reading before auditing any other phase.

### Phase 7 — `fleet`
Cheapest 4,443 LOC in the port; zero consumers; sequence last. **Premise
correction:** `discovery.rs` does *not* map to `sandhi_discovery_*` (sandhi
resolves ONE service by name, no enumerate, no metadata map) — it's 174 lines of
stub needing no sandhi at all.

⚠ **Not a default-build gap.** `fleet` appears only in
`full = ["sandbox", "fleet", "definitions", "hwaccel", "otel", "kavach", "majra"]`
and `default = []`, so the build this port takes as its parity oracle never
compiles it. Porting it *extends* agnosai past the bar rather than closing a hole
in it — a legitimate goal, but schedule it as scope, not as debt.

### Phase 8 — `telemetry` (partial)
Copy hoosh's proven `otlp.cyr` (199 lines) — this is the one place the remote
seam does NOT apply (OTLP export is in-process; a remote gateway cannot export
agnosai's spans). Thread-local trace context is **mandatory** under run_pooled
(sakshi's span stack + trace id are process globals, sakshi.cyr:1400-1407).

⚠ **This is the only one of the three that touches the default build**, and only
in part. `rust-old/src/lib.rs:25` declares `pub mod telemetry;` with **no
`#[cfg]`**, and `main.rs` initialises logging at startup either way — the OTLP
arm is `#[cfg(feature = "otel")]`, but the `#[cfg(not(...))]` arm still runs
`tracing_subscriber::fmt().with_env_filter(..agnosai=info..).json().init()`.

So the default-build gap is **JSON-formatted stderr logging with an `EnvFilter`**,
not OTLP export. sakshi has neither a JSON output mode nor an env filter, so the
port emits plain text at sakshi's default level. That is a **stated** divergence,
documented at `src/main.cyr:22-26` — log *content* matches the oracle line for
line, transport and format do not. Decide whether to close it (needs sakshi
work, i.e. an upstream ask) before treating Phase 8 as purely additive.

### Phase 9 — `definitions` (partial)
assembler, loader-JSON, presets, versioning, k8s_crd (which parses **JSON** only
— the ```yaml at k8s_crd.rs:33 is a doc comment). **Defer** ZIP container +
packaging + YAML.

⚠ **Not a default-build gap**, same as Phase 7 — `definitions` is `full`-only.
The visible consequence is `GET /api/v1/presets`, which the port answers `[]`.
**That is correct parity, not a stub**: the oracle's body is
`#[cfg(feature = "definitions")]` / `#[cfg(not(..))]`, the `not` arm returns
`Json(vec![])`, and the oracle's own test asserts empty under default features.
Verified 2026-08-07 against `rust-old/Cargo.toml` and
`rust-old/src/server/routes/definitions.rs`. The port documents this in place at
`src/server/routes/tools.cyr`. Porting the loader turns `[]` into a branch;
until then, do not "fix" the empty array.

## Parity definition — what v2.0.0 is

**Ships** (the whole default cargo build — `default = []` → core + orchestrator +
llm + tools + server + learning), plus fleet, plus 77% of sandbox, plus
JSON-only definitions, plus OTLP telemetry.

**Excluded, with reason:**
- **bhava / `personality`** — not ported (user decree, post-v2).
- **WASM as a FORMAT** — not ported (explicit cyrius non-goal). The *capability* ships: the tool sandbox rides **cx** + kavach per ADR-006. Existing `.wasm` tools, the tool SDK and `examples/wasm-tools/` do **not** port — they must be rewritten in Cyrius.
- **definitions ZIP + YAML** — both behind the non-default `definitions` feature; both are upstream filings.
- **genai.rs, inference_queue.rs** — zero consumers; pending sign-off.

**Wire parity is the bar**, judged against `rust-old/`.

## Test strategy — do NOT copy hoosh

hoosh's "~71% retention" is **71% of test count against 170 locally-redefined
MIRRORS** of production logic (only 1 of 30 src/lib modules is included). The
mirrors reuse production symbol names, so the real modules can **never** be
included later (last-definition-wins, silently). Real modules **are** includable
— proved by probe.

- **Per-module `.tcyr`** (`cyrius tests` walks recursively — verified to depth 3 — aggregates pass/fail, and exits 1 on any failure; safe as the single test entrypoint).
- **Ban mirror-defining**: any `fn` in a .tcyr duplicating a src/ symbol is a defect. Enforce by CI grep.
- Inline `#[cfg(test)]` mods testing private fns are a **non-problem** — Cyrius has no module privacy.
- **`cyrius bench` no-arg is NON-recursive** — it lists `benches/`, `tests/bcyr/` and `tests/` one level each. Verified: `benches/nested/deep.bcyr` was silently skipped while `benches/top.bcyr` ran. Keep every `.bcyr` **flat**.
- ~~**Build our own coverage.**~~ **CORRECTED 2026-07-28 — this claim is stale.** `cyrius coverage --min 80` walks project **`src/`** recursively and reads `tests/**/*.tcyr` as the corpus; `--min` genuinely gates (non-zero exit). CLAUDE.md's 80% gate **is** dischargeable today. (cyrius archived its own issue `2026-07-23-hoosh-coverage-reports-stdlib-not-local-repo.md` — the hoosh 7/1097 number was the bug, and it is fixed.) Two conditions: matching is **substring-based**, so keep the `agnosai_*` prefix on every public fn (a bare `add` or `run` gets falsely credited) and `_`-prefix genuine internals so they leave the denominator; and `cyrius audit` does **not** run coverage, so CI needs it as its own step.
- ~~**Hard constraint — the 1 MiB corpus cliff.**~~ **RETIRED 2026-08-07 — there is no corpus ceiling.** The prediction was right and the cliff was real: agnosai crossed it on 2026-08-05 at 1,053,976 bytes, coverage silently under-reported (measured 100% → 85% across seven corpus sizes, with padding to *one* suite deleting an unrelated suite's evidence), and `--min` still exited 0. Filed, and **fixed upstream in 6.5.8**; the fixed `alloc(1048576)` in `cbt/quality.cyr` is replaced by a grow-and-retry loop. Re-verified on **6.5.10** by the filing's own repro: the corpus is 1,125,915 bytes today and padded to **1,765,916** — well past the 1,376,773 that used to report 85% and 64/75 files — `cyrius coverage --min 80` still reads **75/75 files, 1099/1099 functions, 100%**. The consumer-side workaround (`scripts/check-coverage.sh` plus a corpus-size gate) is deleted. **Do not reintroduce a corpus budget**, and do not split `tests/` to dodge a cap that no longer exists.
- **Exit-code clamp hazard**: `sys_exit(assert_summary())` & 0xFF → exactly 256/512/768 failures scores **PASS**. agnosai has 865 tests. Still live in 6.4.83 — do **not** use the stock `proj-tcyr` epilogue. Every `.tcyr` ends `var f = main(); if (f > 0) { f = 1; } syscall(60, f);`, and CI greps for the raw pattern.
- 155 of 865 tests are `#[tokio::test]` and Cyrius has **no async test harness**.
- **Freeze** the 124-row Rust bench-history.csv; start a fresh Cyrius baseline. tokio numbers are not comparable — do not claim wins/regressions across the port. *(A terminal v1.1.0 row set was captured 2026-07-28 before the freeze, closing the gap since the last rows at v1.0.2/2026-04-02. `scripts/bench-history.sh` also had a name-capture bug — criterion prints short names on the same line as their timing, so `time: [...]` landed inside the name field for 24 of 237 rows; fixed before the final capture.)*

## Upstream filings (user files; never `gh` — curl to the GitHub API)

| Repo | Ask |
|---|---|
| bayan | ✅ **both filed 2026-07-16** — YAML parse → the existing tagged value tree: `bayan/docs/development/issues/2026-07-16-agnosai-yaml-parse-into-tagged-value-tree.md` (also accepted onto bayan's roadmap as `bayan_yaml_*`; the "draft written" here never materialized as a file — the filing supersedes it); JSON recursion-depth cap (blocker #2): `bayan/docs/development/issues/2026-07-16-agnosai-json-no-recursion-depth-cap.md` — ✅ **resolved in bayan 1.1.1** (cap 128, serde_json parity; 101/101 asserts green) |
| sankoch | ZIP archive container (deflate + crc32 already there; ~250 lines) |
| cyrius | **Every agnosai filing to date is now resolved.** Shipped since this table was written, in order: `chan_try_send` (6.4.84), `vec_sort_by`/`vec_select_nth` (6.5.4), `sys_exit_group` + `async_await_readable_ms` (6.5.6), `thread_create_detached` and the coverage corpus buffer (6.5.8), the unconditional-futex-wake mutex and the fixed-capacity arena (6.5.9), and **`alloc_via` call-chain overhead (6.5.10)**. `2026-07-29-fmt-int-buf-i64-min.md` is also closed — `fmt.cyr`, `string.cyr`, `log.cyr` and sakshi 2.4.8 all guard `i64::MIN` now, verified by formatting it. Still open at 6.5.10: `2026-07-28-sock-send-result-allocates-per-call.md` (16 B/response) and `2026-07-29-no-portable-xmkdir-in-io-cyr.md`. Original row follows. ✅ **`chan_try_send` filed 2026-07-28** and **resolved in 6.4.84** (blocker #4; the load-bearing evidence is majra's hub-mutex-across-blocking-send, not agnosai's own call sites) — archived upstream. ✅ **`vec_sort_by` / `vec_select_nth` filed 2026-07-28** as `2026-07-28-agnosai-no-nlogn-sort-in-stdlib.md`, still open — 18+ consumers (itihas, sankhya, goonj, naad, agora, mela, mneme, samay, stiva, takumi, sit, shakti, varna, chakshu, darshini, hisab, nous, dhvani) have each independently reimplemented an O(n²) insertion sort, which is the strongest possible argument that this is a stdlib gap rather than an app concern. *(This row previously read "still to file" — it had already been filed the same day.)* Also open from agnosai: `2026-07-28-sock-send-result-allocates-per-call.md`, `2026-07-29-no-portable-xmkdir-in-io-cyr.md`, `2026-07-29-mutex-unlock-unconditional-futex-wake.md`, `2026-07-29-fmt-int-buf-i64-min.md` |
| kavach | WASM availability one-liner; stderr capture; **exec timeout — an undocumented regression** (Rust 2.0.0 shipped it, the Cyrius port dropped it, ADR-004 omits it) |
| sigil | `pem_decode_pubkey` — a ~20-line clone of `pem_decode_privkey`. **Nice-to-have, not a blocker** (see below) |
| bote | `jwt_verify_rs256` — **its stated premise is stale**: jwt.cyr:9-11 says "RS256 needs an asymmetric primitive sigil doesn't yet expose", but sigil has `rsa_pkcs1v15_verify_sha256` and `rsa_pubkey_from_der` already accepts SPKI. **Do not wait on this** — verified 2026-07-28 by *compiling and running* a scratch RS256 verify against the real toolchain. Implement the JWT half locally in agnosai over `bayan_base64url_decode` + `clock_epoch_secs` + sigil's existing RSA/SHA-256 primitives |
| majra | ✅ **hub-mutex-across-blocking-sends fixed in 2.5.3**. Still open: relay's file-scope globals make `relay_receive` non-reentrant (its own comment says they're vestigial) |
| sandhi | ✅ **body cap/413 shipped in 1.9.4** (blocker #1) — plus the two adjacent silent paths found while fixing it (peer-hangup, TE-chunked-only) and a configurable `max_request`. ✅ **per-request allocation (blocker #3) shipped in 1.9.7, folded in with cyrius 6.5.2 on 2026-07-29** — `sandhi_server_options_req_arena` / `_get_req_arena` (per-worker, per-request, default off), `sandhi_server_request_arena`, and the allocator-threaded `sandhi_router_dispatch_a` / `sandhi_server_router_handler_a`, all five verified present in agnosai's `lib/sandhi.cyr`. Residual: 16 B/response from `lib/net.cyr`'s `sock_send` `Result`, filed as a cyrius issue and pinned by an exact-bound test in sandhi. **Still open:** `backlog` silently ignored by run_opts/run_async; chunked start hardcodes " OK"; **inbound** chunked decoding (1.9.4 answers 501, which is honest but not the same as support) |

## Corrections to earlier claims (recorded so they are not re-derived)

- **"No HTTP server in Cyrius"** — FALSE. sandhi is a 14,171-line HTTP/2 stack with 65 `sandhi_server_*` fns. The claim came from grepping `http_server_run|http_server_new|http_serve`.
- **"kavach is not functional / carries 1 of 7"** — FALSE. 422 tests pass. It has two exec paths; the subagent scored only the lossy one. `persistent_*` has real live stdin+stdout pipes, SIGKILL+reap, pre-exec safety — live-tested against /bin/cat. And agnosai only consumes kavach's scoring+gate anyway.
- **"kavach has 12 backends"** — FALSE. 10 (`BACKEND_COUNT = 10`).
- **"No broadcast/fan-out"** — FALSE. majra pubsub fans out to per-subscriber channels; `fleet/relay.rs` maps onto majra `relay_*` near-1:1 (identical fields, API, and doc language).
- **"sigil has no SPKI decoder"** — FALSE. `rsa_pubkey_from_der` (sigil.cyr:17696-17703) explicitly accepts both PKCS#1 and X.509 SubjectPublicKeyInfo. Only the `-----BEGIN PUBLIC KEY-----` PEM label pair is missing.
- **"No ranged RNG"** — FALSE. tyche (`rng_uniform`/`rng_normal`/`rng_seed`). But **never for the audit key** (orchestrator.rs:63) — tyche is not a CSPRNG; that stays `random_bytes()`.
- **"No sort"** — half-false, and **promoted to blocker #8** with measurements. `itihas/src/util.cyr:57 vec_sort(v, cmp)` exists but is O(n²) and **unprefixed** → collides under the fold. "Copy the 17 lines locally" was the wrong remedy: at agnosai's 100k percentile workload that shape costs **52.6 s**. Vendor `ai_sort` (heapsort, 87 ms) + `ai_select_nth` (quickselect, ~21 ms for all three percentiles) instead.
- **"Arc is a real gap"** — dissolves. 52 `Arc<` in src/, of which 49 are process-lifetime shared-immutable state = a raw pointer in an arena. Only 3 are `Arc<RwLock|Mutex>` → futex mutex + `async_rwlock_new`.
- ~~**`cyrius coverage` / `cyrius bench`** — structurally unusable as-is.~~ **RETRACTED 2026-07-28.** `cyrius coverage --min 80` measures project `src/` and gates properly; cyrius fixed and archived its own issue for this. `cyrius bench` is usable too — the real constraint is just that no-arg discovery is **non-recursive**, so keep `.bcyr` flat. See the rewritten Test strategy.
- **"`cyrius fmt --check` is a no-op that always exits 0"** — FALSE (a claim that surfaced during this re-verification and was checked). It is silent by design and returns **exit 1 on drift** — confirmed live against sandhi, where it caught real drift in `tests/sandhi.tcyr`. CI should read the exit code, **not** diff `cyrius fmt`'s stdout. Related trap: `cyrius fmt FILE` (no `--check`) prints the formatted file to **stdout** and does not write in place; redirect through a temp file to apply.
- **"agnosai's `tx.send` needs a non-blocking send"** — half-true, and the half that matters is different. They are tokio **broadcast** sends: never block, never fail, and **evict the oldest** on lag. See blocker #4 — the port needs overwrite-oldest semantics, not drop-newest.

### Added 2026-08-07

- **"An arena allocation is free, so wire spellings should come from `*_to_wire_a` rather than globals"** — FALSE, and it was this file's sibling guidance in roadmap B3 until 2026-08-07. An arena allocation is an `alloc_via`: **15.1 ns on 6.5.9, 11.1 ns on 6.5.10**, plus a 16-byte `Str` header. Hoisting the wire values and per-item keys to process-lifetime globals took `GET /api/v1/dashboard/crews` **6,881 → 5,217 ns (−24%)** and **160 → 112 allocations (−30%)** on one toolchain. The scope rule is **"a literal a loop body or an unconditional envelope reaches"** — not "a literal": 338 `str_from_a(a, "…")` sites over 237 strings deliberately remain, because an error *message* is built at most once per request and only for a request that already failed.
- **`alloc_via` is the route-latency floor, and ~11 ns is where it stops.** `arena_alloc`'s fast path is ~8 instructions; the rest was the call chain. 6.5.10 inlined the two accessor loads and dropped the `_arena_*` trampolines. What remains is inherent to a vtable — hand-inlined `fncall2(load64(a), load64(a+32), size)` measures 8.9 ns and `arena_alloc(state, size)` direct measures 6.2. **Count allocations, not bytes**, when reasoning about route latency; a counting allocator wrapped around the arena's own vtable (`allocator_new(&counting_alloc, .., arena)`) gives the exact number.
- **"A sibling's vendored `lib/` is what re-layers a stale stdlib module"** — FALSE, and it cost a wrong upstream filing that was written and then deleted. The real mechanism for `lib/sakshi.cyr` was a **transitive git-dep pin**: sigil and bote declare `[deps.sakshi]` in their *own* manifests, and `cyrius deps` overlays that resolution on top of the `lib sync --full` snapshot — on every `cyrius build`, since build performs an implicit resolve. Proven by hashing every candidate: the file on disk matched `~/.cyrius/deps/sakshi/2.4.7/dist/sakshi.cyr`, and syncing all seven siblings to 2.4.8 changed nothing until the **tags** moved. **Diagnose this class by hashing, never by reading version stamps** — several sources can hold the same version and only one is the writer.
- **`GET /api/v1/presets` answering `[]` is correct parity, not a stub.** Verified against `rust-old/Cargo.toml` (`default = []`) and `routes/definitions.rs`, whose `#[cfg(not(feature = "definitions"))]` arm returns `Json(vec![])` and whose own test asserts empty under default features. Recorded because the empty array reads like an omission and has been re-questioned.

## Open questions (need a decision before the bites they gate)

1. ~~**Money representation**~~ — ✅ **DECIDED 2026-07-28: integer micro-USD** (user, following hoosh pricing.cyr:6-9). Exact, wire round-trip compatible, no f64 accumulation drift across a crew's cost path. The known cost: serialization is textually `0.002500` where serde emitted `0.0025` — numerically equal, so any JSON-number consumer is unaffected; only a byte-exact string diff would notice. Applies to core BITE 8 and every downstream cost path.
2. **`"personality": null`** — bhava is a *hard* dep today, so the default Rust build emits an explicit null. Emit the literal to keep byte-exact default-build wire parity? This is the line between "bhava deferred" and "the wire changed."
3. **Drop the ~20 zero-consumer hoosh re-exports** (src/llm/mod.rs) as a v2.0.0 Breaking change — and with them `inference_queue.rs`, `genai.rs`, and router's `suggest_quantization`/`estimate_model_memory` (all zero-consumer)?
