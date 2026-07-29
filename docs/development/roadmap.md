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

### M4 — `tools` (Phase 3)

native, registry (+ mandatory registry mutex — `run_pooled` makes every worker a
thread), remote_registry, builtin/*. Defers python_tool/wasm_tool.
**Exit:** 94 of 115 tests green.

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
- ⬜ **remaining builtins** (synapse 386, mneme 442, delta 471) and
  **remote_registry** (119). remote_registry's payload path
  (`.agpkg` ZIP + raw WASM) defers with those formats, so it can only deliver a
  guarded fetch.

### M5 — `orchestrator` (Phase 4)

16 of 18 bites. The default-feature runtime path: orchestrator → crew_runner →
scoring/scheduler/output_validation, + approval, budget, audit, memory,
multi_tenant, plan_cache, durable_state (→ patra), hierarchical. **Gate:** the
crew-event fan-out needs **overwrite-oldest** semantics
(`agnosai_chan_push_lossy`), not `chan_send` — a 1:1 port of tokio's broadcast
`tx.send` onto a blocking send converts never-block-lossy into block-forever.
See port-plan blocker #4.
**Exit:** a crew runs end-to-end headless.

### M6 — `server` (Phase 5)

22 bites. Pure leaves first (ssrf → prompt_guard → output_filter → prometheus —
string/number work, independently testable), then auth, hot_config, sse/EventBus,
routes/*, router, main. **Gates:** per-worker arena + `_a` variants throughout
(blocker #3); JWT RS256 implemented locally over sigil's existing
`rsa_pkcs1v15_verify_sha256` + SPKI decoder rather than waiting upstream.
**Exit:** the 11-route API serves; SSE streams; load-tested with `alloc_used()` asserted flat.

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
