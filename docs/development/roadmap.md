# AgnosAI — Roadmap

> Milestone plan through v2.0. State lives in [`state.md`](state.md); this file
> is the sequencing — what ships, in what order, against what dependency gates.
> The technical plan of record is
> [`cyrius-port-plan.md`](cyrius-port-plan.md); this is its schedule.

## What v2.0.0 is

The Rust default build (core + orchestrator + llm + tools + server + learning),
plus fleet, plus 77% of sandbox, plus JSON-only definitions, plus OTLP telemetry.
**Wire parity is the bar**, judged against `rust-old/`.

**Excluded, with reason:**

| Excluded | Why |
|---|---|
| bhava / `personality` | user decree, post-v2 |
| WASM **as a format** — the tool SDK and `examples/wasm-tools/` | Explicit cyrius non-goal. The *capability* ships instead: the tool sandbox rides **cx** + kavach ([ADR-006](../adr/006-cx-tool-sandbox.md)). Existing `.wasm` tools must be **rewritten in Cyrius** — not a drop-in. Linux-x86 only until cx arc C |
| definitions ZIP + YAML | both behind the non-default `definitions` feature; both are upstream filings |
| `genai.rs`, `inference_queue.rs` | zero consumers; pending sign-off |

## v2.0 criteria

- [ ] Wire parity verified against `rust-old/` for every shipped surface
- [ ] `cyrius coverage --min 80` green (its own CI step — `cyrius audit` does not run it)
- [ ] Cyrius benchmark baseline established (fresh CSV; **not** compared to the frozen Rust one)
- [ ] At least one downstream consumer green
- [ ] CHANGELOG complete from v2.0.0 onward
- [ ] Security audit pass

## Milestones

Dependency-ordered. Each phase decomposes into commit-sized bites (each
independently compilable + testable). ~122 bites across 10 groups.

### M0 — Port scaffold — ✅ shipped 2026-07-28

- Rust baseline greened: blocker #7 fixed, fmt + clippy clean, all 9 feature combos compile, 863 + 2 + 1 tests pass
- Terminal Rust benchmark row set captured (112 rows at v1.1.0), CSV frozen into `rust-old/`
- Blockers re-verified against cyrius 6.4.83 with adversarial refutation of every "resolved" verdict
- Upstream: **sandhi 1.9.4** shipped (blocker #1 + two adjacent silent-truncation paths found while fixing it); `chan_try_send` filed against cyrius and shipped in 6.4.84 (blocker #4)
- `cyrius port` run; Rust preserved at `rust-old/`

### M1 — Dependency scaffold (Phase 0)

Ordered stdlib array + declare-ahead git deps (daimon/stiva pattern, **not**
vendoring). **Gates:** sigil pinned as a git dep and removed from the stdlib
array (two packagings = daimon's 227 duplicate-fn warnings); UUID reimplemented
in `src/id.cyr` (mneme is unusable — no lib block, and AGPL against our GPL);
`ai_sort`/`ai_select_nth` vendored (blocker #8).
**Exit:** hello-world builds; `cyrius deps` resolves clean.

### M2 — Beachhead: `learning` + `core` (Phase 1)

Both blocked by nothing. `learning` first — zero coupling, zero async, zero
serde, zero traits, zero I/O — it exercises tyche, f64-as-bit-patterns, and the
`.tcyr` harness with no downstream risk. Then `core`, the root of the graph.
**Exit:** learning + core tests green against `rust-old/` as oracle.

- ✅ **`learning` done** — 5 modules + hub, 112 assertions green against the
  oracle's 35 tests, 100% reference coverage, 10 benchmarks seeding the Cyrius
  baseline. It confirmed the harness assumptions the rest of the port rests on:
  a `.tcyr` can include real `src/` modules (no hoosh-style mirror-defining
  needed), `cyrius tests` walks `tests/` recursively, `cyrius coverage --min`
  measures project `src/` and gates properly, and the `agnosai_*` prefix rule
  keeps our symbols clear of the fold (zero duplicate-fn warnings from our code).
- ✅ **`core` done** — all six oracle submodules plus a shared `core_json`
  helper module: 388 assertions green, plus 26 for the `src/id.cyr` UUID
  prerequisite that `message`, `task` and `crew` all key on. 100% reference
  coverage across the project. The money-representation question is settled (integer micro-USD),
  and converting to f64 only at the wire boundary means the wire stays
  byte-identical to serde. Two documented exclusions, both outside the v2.0.0
  parity bar: `AgentDefinition::personality` (bhava, post-v2 — still emits
  `null` for wire parity) and the `#[cfg(feature = "hwaccel")]` half of
  `resource.rs`.

**M2 exit met:** learning + core tests green against `rust-old/` as oracle.

### M3 — `llm`, the hoosh seam (Phase 2) — ✅ done

Cheapest group; the reference implementation already exists (`thoth/src/hoosh.cyr`).
llm **defines** the types orchestrator consumes, so it lands early.
**Exit:** a live chat-completion round-trip against `hoosh serve 8088`.

- ✅ **router + retry + hoosh seam client**, 187 assertions green, 100% reference
  coverage across the project. **Exit met and verified** through
  `agnosai_hoosh_chat` itself (`scripts/stack.sh check`), not curl.
- **Correction to the plan's open question 3.** "~20 zero-consumer re-exports"
  is wrong: 11 have real consumers. Five are data types the seam now defines
  locally (ProviderType, Message, Role, InferenceRequest, HooshClient); six are
  server-side subsystems with no client-side equivalent over an HTTP seam
  (AuditChain at 36 refs, ResponseCache, CostTracker, llm_metrics, cache_key,
  LlmProvider). **Every consumer of that second set is in M5 or M6**, so
  whether agnosai reimplements them locally or drops them is a decision for
  those milestones — AuditChain especially, since it is the tamper-evident
  audit trail. Also: `genai.rs` is `src/telemetry/genai.rs`, not llm's, so it
  belongs to M9; `inference_queue.rs` is llm's, `majra`-gated, and genuinely
  zero-consumer.

### M4 — `tools` (Phase 3) — ✅ **COMPLETE**

native, registry (+ mandatory registry mutex — `run_pooled` makes every worker a
thread), remote_registry, builtin/*. Defers python_tool/wasm_tool.
**Exit:** 94 of 115 tests green — met and exceeded; the Cyrius suites carry 1446
assertions across 24 files, and every execute path that the Rust suites left
untested (because it needed a live server) is covered here through a transport
seam.

- ✅ **native + registry done** — 67 assertions. Two forced shape changes, both
  documented in-module: the `NativeTool` trait becomes a function-pointer vtable
  (no traits in Cyrius) with a **synchronous** `execute` (no futures), and the
  lock-free `DashMap` becomes a hashmap behind a futex mutex.
- ✅ **blocker #8 closed** — `src/order.cyr` lands heapsort + quickselect ahead
  of `builtin/load_testing.rs`, which needs them for its 100k percentile vector.
  Benchmarked; see state.md.
- ✅ **echo + json_transform** — 37 assertions, registering and dispatching
  through the registry.
- ✅ **`server/ssrf` pulled forward from M6** — `builtin/load_testing.rs` and
  `remote_registry.rs` both gate on `is_safe_url`, so it was port-once rather
  than stub-twice. 81 assertions, weighted toward the bypass classes.
- ✅ **load_testing** — 88 assertions, and the **first production user of the
  blocker #3 arena pattern**: one OS thread per simulated user, each with a
  persistent arena for its latencies plus a scratch arena `reset_via`'d after
  every request. Both of its blockers (`order` for percentiles, `ssrf` for the
  URL guard) were cleared ahead of it. The oracle's two tests drive an axum mock
  server on loopback, which `is_safe_url` rightly refuses — so the port tests the
  real thread fan-out against a synthetic executor instead, and the network seam
  is covered separately by `scripts/stack.sh check`.
- ✅ **security_audit** — 200 assertions. Split at the network boundary like
  load_testing, which here lets all five of the oracle's mock-server tests port
  exactly rather than being replaced. Four inherited-default corrections and one
  deliberate security divergence, all in
  [ADR 007](../adr/007-audit-redirect-revalidation.md): the SSRF guard now
  re-runs on **every redirect hop**, where the oracle validates only the URL the
  caller supplied and then lets reqwest follow up to 10 hops unchecked.
- ✅ **synapse + mneme + delta** — nine tools, 140 assertions, over one shared
  client in `src/tools/agnos.cyr`. The third instance of an identical HTTP shape
  is where CLAUDE.md says to extract, and extracting also created the seam: the
  transport is a function pointer, so all nine execute paths are tested without
  a service running, which the oracle's own suites never manage. These are the
  one tool family that must **not** run the SSRF guard — they target loopback
  services by design, and the guard would reject all three default base URLs.
- ✅ **remote_registry** — 77 assertions. **A complete port, not a partial
  one:** the oracle's doc comment promises `.agpkg` ZIP and WASM handling plus
  registration, but the file contains none of it and is `pub mod` with zero
  consumers. Nothing defers with those formats here — there was never any
  behaviour to defer. ADR 007 applies and mattered more than it did in
  security_audit, since this fetches a payload the doc intends to become
  executable: the guard now re-runs on every redirect hop via the shared
  `src/guarded_fetch.cyr`, extracted from security_audit at this second
  consumer rather than left as a copy. remote_registry's payload path
  (`.agpkg` ZIP + raw WASM) defers with those formats, so it can only deliver a
  guarded fetch.

### M5 — `orchestrator` (Phase 4)

✅ **COMPLETE.** The default-feature runtime path: orchestrator → crew_runner →
scoring/scheduler/output_validation, + approval, budget, audit, memory,
multi_tenant, plan_cache, durable_state, hierarchical — all 15 modules, plus
`server/sse` and `server/prompt_guard` pulled forward from M6 and an `orch_audit`
chain the hoosh seam cannot delegate. **Gate (met):** the crew-event fan-out needs
**overwrite-oldest** semantics (`agnosai_chan_push_lossy`), not `chan_send` — a
1:1 port of tokio's broadcast `tx.send` onto a blocking send converts
never-block-lossy into block-forever. See port-plan blocker #4.

**Correction (2026-07-29, verified while porting):** this line read
`durable_state (→ patra)`. It does not use patra. patra is a full embedded SQL
database over its own paged format with no dump/export verb, and its `jsonl` mode
opens `O_APPEND` with no `O_TRUNC`; `durable_state`'s contract is one overwritable,
human-readable JSON file per crew at a caller-chosen path, which patra
structurally cannot produce. It is built on `lib/io.cyr` instead, and **no M5
module touches patra** — `grep -rn 'patra_\|jsonl_' src/ tests/` is empty. patra
stays a declared stdlib dep for later phases.

**Exit:** a crew runs end-to-end headless.

### M6 — `server` (Phase 5)

22 bites. Pure leaves first (ssrf → prompt_guard → output_filter → prometheus —
string/number work, independently testable), then auth, hot_config, sse/EventBus,
routes/*, router, main. **Gates:** per-worker arena + `_a` variants throughout
(blocker #3); JWT RS256 implemented locally over sigil's existing
`rsa_pkcs1v15_verify_sha256` + SPKI decoder rather than waiting upstream.
**Exit:** the 11-route API serves; SSE streams; load-tested with `alloc_used()` asserted flat.

**Status:** 20 of 21 files. Bites 1-15b are done — the routes tier, the router,
and the sandhi adapter all land. Remaining: **15c (SSE)** and **16 (`main.rs`)**;
both, plus everything else still owed, are enumerated under
[Open blockers and owed work](#open-blockers-and-owed-work).

The `alloc_used()`-flat exit criterion is **partly met and cannot be fully met
here.** Blocker #3's arena makes sandhi's half flat, but bayan threads no
allocator on parse/build, so the handler half still grows the global bump —
measured, and owed upstream as B3. The exit bar should read "transport flat,
handler cost bounded and measured" until that filing lands.

### M7 — `sandbox`, 77% (Phase 6)

policy (rename to `AgnSandboxPolicy` first), kavach_bridge, exec, process, python,
oci, manager. **`wasm.rs`'s successor rides cx** per
[ADR-006](../adr/006-cx-tool-sandbox.md): `cycc_cx` → `.cyx` → `cxvm`, spawned
**inside** a kavach sandbox. `cxvm` does no syscall filtering of its own, so
kavach's seccomp + landlock *are* the security boundary — an unwrapped `cxvm`
spawn is a full escape, and the milestone needs a test asserting a `.cyx` that
attempts `open("/etc/passwd")` is refused. Tool code is **integer-only** until cx
arc B (float literals miscompile) and **Linux-x86 only** until arc C. agnosai consumes only kavach's scoring + gate,
which port 1:1; uses `persistent_spawn/send/read/terminate` for the stdin-JSON
tool protocol.

### M8 — `fleet` (Phase 7)

Cheapest 4,443 lines; zero consumers; sequence last. `discovery.rs` needs no
sandhi at all — it is 174 lines of stub.

### M9 — `telemetry`, partial (Phase 8)

Copy hoosh's proven `otlp.cyr` (199 lines) — the one place the remote seam does
NOT apply, since OTLP export is in-process. Thread-local trace context is
**mandatory** under `run_pooled`: sakshi's span stack and trace id are process
globals.

### M10 — `definitions`, partial (Phase 9)

assembler, loader-JSON, presets, versioning, k8s_crd (which parses **JSON** only —
the ` ```yaml ` in its doc comment is a doc comment). Defers ZIP container +
packaging + YAML.

## Open blockers and owed work

**Status as of 2026-07-31, after M6 bite 15b.** Every item below is open. Each is
self-contained: file paths, measured numbers, and what "done" means, so it can be
picked up without reading the session that found it. Ordered by what blocks what.

Nothing here is a discovery in progress — this is the complete list. If an item is
not on it, it is not owed.

### A. Blocks M6 completion

| # | Item | Effort | Notes |
|---|------|--------|-------|
| A1 | **Bite 15c — `sse.rs::event_stream` + `routes/sse.rs`** (241 lines) | Large — own session | The last hard bite. Thread-per-connection capacity is a **design decision, not a transcription**: there is no async runtime, so each stream holds a worker from the `run_pooled` pool of 100. Held back at `src/server/sse.cyr:9-11`. Until it lands, `/api/v1/crews/{id}/stream` answers **501** deliberately (`src/server/router.cyr:388`) — not 404, because the route exists and only its handler is missing. |
| A2 | **Bite 16 — `src/main.cyr` bind** | Small | `getenv` is at `lib/io.cyr:587`. Graceful shutdown needs a raw `rt_sigaction`; **no signal helper exists in `lib/`** — either write one locally or file it upstream. `agnosai_serve(state, addr, port)` is ready and returns 1 on bind failure (tested). |

### B. Owed — flagged in earlier bites, never done

| # | Item | Effort | Notes |
|---|------|--------|-------|
| B1 | **Wire the metrics producer** | Small | [ADR 011](../adr/011-metrics-endpoint-serves-agnosai-metrics.md) gave `/metrics` agnosai's own registry, but **nothing records into it** — verified: zero `agnosai_metrics_record*` calls in `src/orchestrator/crew_runner.cyr`. The endpoint renders zeros. The oracle records at `rust-old/.../crew_runner.rs:810` and `:864`. Staged deliberately (M5 code, M6 decision); the staging has now outlived its reason. |
| B2 | **Decide `[deps.bote]`** | Decision + small | `mcp.rs` was its only intended consumer and builds its own JSON-RPC envelope, so agnosai calls **zero bote symbols** (verified) while carrying ~93 KB / 233 fns in every build. Either drop the dep or find it a job. |
| B3 | **Thread the bayan `_a` constructors (local), then file the narrowed ask** | **Large (local) + Small (filing)** | **The premise of this row was wrong and is corrected here (2026-07-31).** It read "bayan ships no `_a` variants", generalized from a grep for exactly two names (`parse_buf_a`, `build_a`). **bayan ships 15 `_a` variants**, including the whole JSON value-constructor set — `bayan_json_v_{null,bool_new,int_new,float_new,str_new,arr_new,obj_new,arr_push}_a` plus `bayan_json_pair_new_a`. **agnosai calls them zero times** and the non-`_a` forms **401 times**. Measured split of one response build (`agnosai_task_to_json`, 1944 B total): **tree construction 1240 B (64%) — arena-able today, no upstream change**; serialize `bayan_json_v_build` 704 B (36%) — genuinely has no `_a`. So the local work is the majority of the win and is not blocked on anything. The narrowed upstream ask is **`parse_buf_a` / `obj_set_a` / `build_a`** only (`bayan_json_v_obj_set` has no `_a` although `bayan_json_pair_new_a` exists, so it is a thin one). The local half is **large** — it puts an allocator parameter on every `*_to_value` and threads it through the call graph — so it needs its own bite and an API-shape decision, not a batch. |
| B4 | **Migrate `src/order.cyr` to the stdlib sort** | Small | `vec_sort_by` / `vec_select_nth` shipped in cyrius 6.5.4, closing agnosai's own filing. Measured head-to-head at 100k: stdlib introsort **3.85× faster** (20.0 ms vs 77.1 ms), quickselect **1.34×** (4.22 ms vs 5.65 ms). No name collision — `src/order.cyr` is fully `agnosai_*`-prefixed. Flagged at the 6.5.4 bump, not taken. |
| B5 | **Remaining `str_from("lit")` classes** | Medium | 86 `str_eq(x, str_from("lit"))` sites are **done** (→ `str_eq_cstr`, which already existed at `lib/str.cyr:617`): decode path 482 ns / 128 B per 3-decode round → **213 ns / 0 B**. `src/` went 910 → 824 sites. **What is left, and why the obvious next step is NOT worth taking yet:** 149 `return str_from("lit")` constant returns look like the natural follow-on, but hoisting them to init-time globals costs **121 new top-level symbols** and buys **48 B of 1944 B (2.5%)** on the encode path — measured, not estimated. Do B3's local half first; it is 64% of the same number. Revisit this only if a profile still shows it after B3. The ~49 in-loop `str_from` hoists (11 modules, worst are `server/routes/dashboard.cyr` and `orchestrator/crew_runner.cyr` at 10 each) are the better of the two leftovers — same mechanical shape, no new symbols. The 380 sites under `tests/` are deliberately left: a test binary is short-lived, so the leak is inert. |

### C. Upstream — filed and waiting

Nothing here blocks agnosai today; each is a residual agnosai measured and handed off.

| Dep | Open filings |
|---|---|
| cyrius | `2026-07-28-agnosai-no-nlogn-sort-in-stdlib.md` — ✅ **resolved in 6.5.4** (see B4, consumption still owed). Still open: `2026-07-28-sock-send-result-allocates-per-call.md` (16 B/response, pinned by an exact-bound test in sandhi), `2026-07-29-no-portable-xmkdir-in-io-cyr.md`, `2026-07-29-mutex-unlock-unconditional-futex-wake.md`, `2026-07-29-fmt-int-buf-i64-min.md` |
| sandhi | `backlog` silently ignored by `run_opts`/`run_async`; chunked start hardcodes `" OK"`; **inbound** chunked decoding unsupported (1.9.4 answers 501 — honest, but not support) |
| bayan | YAML parse into the tagged value tree (`2026-07-16-...`) — gates M10's YAML half, nothing sooner. Plus B3, unfiled. |
| sigil | `2026-07-30-rsa-verify-uses-secret-exponent-ladder.md` — ✅ **archived upstream**, fixed in sigil's "rsa repairs" commit and contained in the pinned **3.12.2**. See C1 below. |

**C1 — re-resolve deps and re-measure the RS256 verify path.** agnosai pins sigil
3.12.2, which contains the fix, but the vendored `lib/sigil.cyr` still documents
`bn_mont_modexp` (the constant-time secret-exponent ladder) as carrying "the live
RSA verify + sign paths" — so **`cyrius deps` has not been re-run since the fix
landed.** Everything downstream of the old measurement is therefore unverified:
the 3.29 ms verify, the ~235× gap against OpenSSL, and the **~300
JWT-verifies/sec ceiling that was the entire argument for mounting `rate_limit`**.
Re-resolve, re-measure, then revisit D1.

### D. Decisions deferred to a human

| # | Decision | Where it stands |
|---|---|---|
| D1 | **Mount `rate_limit`?** | Ported and tested (bite 14), **not mounted** — matching the oracle, which never installs the middleware. `agnosai_serve_with_rate_limit` is the opt-in path. Mounting it by default is a **wire change**: clients fine today would start seeing 429s at a threshold agnosai chose, not one the oracle documents. The argument for mounting rested on the JWT-verify ceiling — see C1, now unverified. |
| D2 | **`"personality": null` on the wire** | bhava is a *hard* dep in the oracle, so the default Rust build emits an explicit null. Emitting the literal keeps byte-exact default-build parity; omitting it is the line between "bhava deferred" and "the wire changed." See `cyrius-port-plan.md:274`. |

### E. Known-unreachable code kept for oracle shape

Not defects and not owed — recorded so nobody re-derives them as findings. Each is
documented in place as unreachable rather than implied to fire: `agents.rs`'s
serialize skip, the cycle-detector's `== 2` memoization arm, `crews.rs`'s profile
skip, and `crew_runner.rs`'s personality prompt block (bhava, post-v2 per the
user decree at line 18 of this file).

## Carried over from the Rust line

Test-coverage gaps the Rust tree never closed; the port should not reintroduce them:

| Area | What was missing |
|------|------------------|
| Process sandbox | env sanitization, timeout enforcement, kill-on-drop |
| Python sandbox | subprocess execution, timeout, env sanitization |
| Concurrent cancel | mid-execution interruption, parallel/DAG cancel stress |
| Telemetry init | OTLP error paths, env var override, guard lifecycle |

Demand-gated, unchanged: Python bindings (separate crate, `cdylib`, maturin build).

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

## Design principles

1. **Wire compatibility** — REST/MCP/A2A surface matches `rust-old/`
2. **Sandbox by default** — untrusted code never runs unsandboxed
3. **Single binary** — no container orchestration for single-node deployments
4. **Concurrency via `sandhi_server_run_pooled`** — follow daimon, not hoosh (see port plan, "The concurrency decision")
5. **Library first** — agnosai is a library with a binary, not a framework
6. **Lockstep with ai-hwaccel** — aligned versioning, shared practices, same CI rigor

## Out of scope for v2.0

Everything in the exclusion table above, plus: inbound chunked request bodies
(sandhi 1.9.4 answers 501 — honest, but not support), and any comparison of
Cyrius benchmark numbers against the frozen Rust CSV.
