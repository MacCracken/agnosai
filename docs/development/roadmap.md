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
| WASM — `wasm.rs`, `wasm_tool.rs`, `wasm_loader.rs`, the tool SDK, `examples/wasm-tools/` (955 lines) | **no WASM runtime exists in the ecosystem** (port-plan blocker #6, re-confirmed 2026-07-28) |
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
- Upstream: **sandhi 1.9.4** shipped (blocker #1 + two adjacent silent-truncation paths found while fixing it); `chan_try_send` filed against cyrius (blocker #4)
- `cyrius port` run; Rust preserved at `rust-old/`

### M1 — Dependency scaffold (Phase 0)

Ordered stdlib array + declare-ahead `[deps.*]` (daimon/stiva pattern, **not**
vendoring). **Gates:** sigil pinned via `[deps.sigil]` and removed from the
stdlib array (two packagings = daimon's 227 duplicate-fn warnings); UUID
reimplemented in `src/id.cyr` (no `[deps.mneme]` — no `[lib]` block, and AGPL
against our GPL); `ai_sort`/`ai_select_nth` vendored (blocker #8).
**Exit:** hello-world builds; `cyrius deps` resolves clean.

### M2 — Beachhead: `learning` + `core` (Phase 1)

Both blocked by nothing. `learning` first — zero coupling, zero async, zero
serde, zero traits, zero I/O — it exercises tyche, f64-as-bit-patterns, and the
`.tcyr` harness with no downstream risk. Then `core`, the root of the graph.
**Exit:** learning + core tests green against `rust-old/` as oracle.

### M3 — `llm`, the hoosh seam (Phase 2)

Cheapest group; the reference implementation already exists (`thoth/src/hoosh.cyr`).
llm **defines** the types orchestrator consumes, so it lands early.
**Exit:** a live chat-completion round-trip against `hoosh serve 8088`.

### M4 — `tools` (Phase 3)

native, registry (+ mandatory registry mutex — `run_pooled` makes every worker a
thread), remote_registry, builtin/*. Defers python_tool/wasm_tool.
**Exit:** 94 of 115 tests green.

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
oci, manager. **Defers `wasm.rs`.** agnosai consumes only kavach's scoring + gate,
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
