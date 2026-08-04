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

**Status: ✅ COMPLETE (2026-08-03).** All 21 files, and **the binary serves** — bite 16 landed
2026-08-03, so `./build/agnosai` binds, reads the oracle's environment, and
answers the route table instead of printing `agnosai ready` and exiting. Bites
1-15b landed earlier (routes tier, router, sandhi adapter); **15c (SSE)** and
**16 (bind)** closed the milestone.

**Exit met**, with one qualification worth reading before deploying: SSE streams
*do* stream, but each holds one of the 100 pool workers for its life, where the
oracle serves effectively unbounded concurrent streams
([ADR 014](../adr/014-sse-stream-holds-a-pooled-worker.md)).

**Graceful shutdown ships too**, though it took a same-day round trip through
two repos. Bite 16 first landed *without* it and with an ADR explaining why it
was impossible ([012](../adr/012-no-graceful-shutdown-on-sandhi.md)) — the
blocker being that sandhi's accept loop could not be made to return, **not** the
missing signal helper this roadmap used to cite (`signal_ignore` exists at
`lib/syscalls.cyr:98`, and `sys_signalfd4` / `sys_rt_sigprocmask` are both
wrapped). agnosai filed it, sandhi 1.9.9 added the stop flag, cyrius 6.5.6
vendored it, and `main` now installs a `signalfd` handler and drains
([ADR 013](../adr/013-graceful-shutdown-via-signalfd-and-stop-flag.md),
superseding 012).

The `alloc_used()`-flat exit criterion is **partly met and cannot be fully met
here.** Blocker #3's arena makes sandhi's half flat, but bayan threads no
allocator on parse/build, so the handler half still grows the global bump —
measured, and owed upstream as B3. The exit bar should read "transport flat,
handler cost bounded and measured" until that filing lands.

### M7 — `sandbox`, 77% (Phase 6) — 🟡 in progress

**Bites 1-7 done (2026-08-04). The spawn primitive is complete** — `process`,
`oci`'s exec half, `python` and `manager` all build directly on it.

Bite 7: the deadline. Cyrius has no `kill_on_drop`, so `_agnosai_spawn_kill_and_reap`
does by hand what tokio does on drop; a child that ignores SIGTERM proves why the
signal is SIGKILL. Two defects in the already-shipped loop surfaced with it: it
**burned 100% of a core** for as long as any child ran (both read ends are
`O_NONBLOCK` and it simply retried — measured at 2.006 s of CPU against
`sleep 2`, now 0%), and `agnosai_spawn_capture` was a second copy of the loop
that left the child's stdin **inherited from the server**, where the oracle pipes
stdin on every spawn. The copy is gone — `capture` is now `capture_input` with an
empty input.

Bite 6: the stdin feed — a second deadlock,
distinct from the output one and caused by the oracle's own write-then-read
shape; the write now interleaves with the drain.

Bite 5: `agnosai_spawn_capture` — fork, three
pipes, exec, status decode. Spawn failure is distinguishable from exit 127, and
the drain interleaves so a child filling both pipes cannot deadlock it. Two
spawn sub-bites remain: stdin feed, then deadline/SIGKILL/reap.

Bite 4: `spawn`'s sanitized `envp` + the
CLOEXEC primitive (77 assertions) — the first of four spawn sub-bites, and the
only one testable without forking.

Bite 3: `kavach_bridge`'s pure half — backend
mapping, config, strength scoring, the 4→2 scan collapse, the trust table
(66 assertions).

Bite 2: `oci`'s pure half — config, image
validation, and the `docker run` argv built as a testable value (75 assertions).
Bite 1: `policy` + the group hub `mod` — isolation levels,
the five named policies, `effective_isolation`, the JSON wire, and the shared
env-sanitization list. 90 assertions, all 11 oracle tests ported.

A survey of the remaining modules was refuted 3/3 on adversarial review. The
sequencing that came out of it, and the three findings that shape it:

**Order** (each independently compilable + testable): `oci` pure half →
`kavach_bridge` pure half → **the spawn primitive** (`envp` + sanitization →
fork/pipes/execve → stdin feed + interleaved drain → deadline/SIGKILL/reap) →
`process` → `oci` exec half → `python` → `manager`. The spawn primitive is the
irreducible prerequisite: nothing in `lib/` supplies the four-tuple these
modules need (separate stdout, separate stderr, real exit code, stdin write).

**✅ CLEARED — the Landlock hole is fixed.** ADR-006 makes kavach's seccomp +
Landlock the *entire* security boundary for untrusted tool code, and on the
Landlock half that premise did not hold: `security_apply_landlock` named only
**3 of Landlock's 13 filesystem rights** in `handled_access`, and Landlock
permits every right it is not told to handle. **Measured: a confined process
deleted files and directories outside its allowed path, and created directories
anywhere** — while the obvious smoke test ("cannot read `/etc/passwd`") passed,
because reading was one of the three that *were* handled.

Fixed in **kavach 3.11.1** (2026-08-03, agnosai-reported and agnosai-fixed): all
thirteen ABI v1 rights, plus `REFER` (v2) and `TRUNCATE` (v3) where the kernel
knows them, with an ABI-version query masking down so an older kernel does not
EINVAL. `EXECUTE` is granted inside read-only paths deliberately — it was
permitted *everywhere* before, and a sandbox that read-only-mounts `/usr` to run
`python3` is the ordinary case. agnosai pins 3.11.1 and re-ran the original
probe: the victim file and directory now survive. The cx bites are unblocked.

**✅ CLEARED — spawn-failure parity is reachable.** `fork` + `execve` alone
cannot distinguish "spawn failed" from "child ran and exited 127", and three
oracle sites need `Err` on the former (`oci.rs:219`, `process.rs:331`,
`python.rs:87`). The fix is a fourth **exec-errno pipe** whose write end carries
`FD_CLOEXEC`, so it closes itself iff `execve` succeeds: a parent that reads
bytes knows the exec failed, and EOF means it worked.

**Proven on this box before committing to it** (2026-08-03). `pipe2` is not
wrapped on x86_64, but `sys_pipe` + `syscall(SYS_FCNTL, wfd, F_SETFD,
FD_CLOEXEC)` is — `SYS_FCNTL = 72` and `O_CLOEXEC = 524288` are both defined. A
probe run against two children — `/bin/sh -c 'exit 127'` and
`/nonexistent/binary` — reports **spawned (exit 127)** for the first and **spawn
failed** for the second, which is exactly the distinction the three oracle sites
need.

**Smaller, recorded so they are not re-derived:** kavach's `SandboxConfig` has no
`externalization` field, so `build_config`'s per-call policy is unrepresentable
and one oracle test cannot port as written; `kavach_bridge::execute` cannot
honour `max_duration_secs` because nothing on the process path reads
`timeout_ms`; and `scan_output` is a lossy 4→2 collapse
(`{Pass,Warn}→Pass`, `{Block,Quarantine}→Block`) that a naive port returning the
raw verdict would get wrong in both directions.


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

## Owed work

**Status as of 2026-08-03. Nothing here blocks anything.** Every remaining item
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

Completed items are removed from these tables rather than struck through, so the
list stays short enough to read. What shipped and why lives in `CHANGELOG.md`; the
one-line ledger is under *Recently closed* at the end of this section.

### A. Blocked the M6 milestone — ✅ retired 2026-08-03

Empty by completion, and kept as a heading only so the **B/C/D/E** letters below
stay stable: they are cited from `state.md` and from `src/` comments, and
renumbering to close a cosmetic gap would break every one of them. A1 (SSE) and
A2 (the `main` bind) are in *Recently closed*.

### B. Owed — flagged in earlier bites, never done

| # | Item | Effort | Notes |
|---|------|--------|-------|
| B1 | **Grow the MCP surface onto bote** *(was: "decide whether to drop `[deps.bote]`" — wrong framing, corrected 2026-07-31)* | Medium | **bote IS the MCP layer**, and this is an agent-orchestration system: `dist/bote-core.cyr` ships the JSON-RPC protocol, registry, dispatcher, schema validation, plus **18 `prompt_*`** and **15 `resource_*`** fns. The earlier row read "agnosai calls zero bote symbols, so decide whether the dep earns ~93 KB" — the observation is true, the conclusion was backwards. `src/server/routes/mcp.cyr` answers `initialize` / `tools/list` / `tools/call`, which is **exactly the oracle's three methods and therefore full parity**; it is a thin slice of MCP, not evidence the protocol library is unneeded. The real work is the reverse of dropping it: MCP prompts, resources and subscriptions are the natural next surface for a crew orchestrator, and they are already in the pinned bundle. **One genuine blocker to delegating, verified:** `dispatcher_dispatch` hardcodes `"serverInfo":{"name":"bote"` at `lib/bote-core.cyr:1559` where the oracle emits `"name":"agnosai"`, so wiring the dispatcher today would change the wire. **Fixed upstream: bote 3.3.0 adds `dispatcher_set_server_info(d, name, version)`** (ships in `[lib.core]`, which is the profile agnosai pins). ✅ **`[deps.bote]` is pinned to `tag = "3.3.0"` as of 2026-08-03**, so the blocker is cleared and only the surface work remains. Note this does **not** mean the route should delegate today — the oracle uses bote's protocol *types* but explicitly declines its Dispatcher (`mcp.rs:3-5`), so hand-building the envelope IS the parity behaviour. 3.3.0 clears the way for the surface after this one. |
| B2 | **Thread the bayan `_a` constructors** — 🟡 **core group done, 6 of ~20 modules** | Large (small bites) | **Unblocked by cyrius 6.5.5** (bayan 1.4.0); the upstream ask this row used to carry shipped there, so nothing waits on anything. ✅ **`core` complete** — json, task, resource, agent, message, crew: **27 `_a` forms**, each with a bare-name wrapper delegating through `default_alloc()`. Measured: `agnosai_task_to_json` **1792 → 0 B/response**, `agnosai_crew_to_json` on a 10x10 crew **44,032 → 0 B** and 11% faster. Every `_a` form is pinned as agreeing byte-for-byte with its global twin. **Two traps for the next bite:** (1) **Cyrius silently accepts a duplicate parameter name** — `agnosai_agent_to_value(a)` already used `a`, and adding an allocator also named `a` compiled clean while every `load64(a + OFFSET)` read the allocator, SIGSEGV with no assertion output. Check the existing parameter names before prepending. (2) **`map_keys` survives every substitution** — it allocates its key vec via `vec_new()` on the global bump with no `_a` form, and it silently caps the win; it turned up three separate times. Use `_agnosai_map_slots` / `_slot_live` / `_slot_key` / `_slot_val` from `src/core/json.cyr`. **Remaining: orchestrator, tools, llm, server/routes** — the routes tier is where the arena actually runs, but it consumes core, which is why core went first. |
| B3 | **Remaining `str_from("lit")` classes** | Medium | 86 `str_eq(x, str_from("lit"))` sites are **done** (→ `str_eq_cstr`, which already existed at `lib/str.cyr:617`): the decode path went 482 ns / 128 B per 3-decode round → **213 ns / 0 B**, and `src/` from 910 to 824 sites. **Re-scoped now that B2's core group has landed.** The 149 `return str_from("lit")` constant returns were measured at 48 B of 1944 B (2.5%) and deferred as not worth 121 new top-level symbols — that verdict **no longer applies to a module B2 has threaded**, because in `core` those wire spellings now come from the arena via `*_to_wire_a` and cost nothing. So: **do not hoist constant returns to globals.** Give them `_a` forms as part of each module's B2 bite, which is what `core/task` and `core/crew` did. What genuinely remains here is the ~49 **in-loop** `str_from` hoists (11 modules; worst are `server/routes/dashboard.cyr` and `orchestrator/crew_runner.cyr` at 10 each) — same mechanical shape, no new symbols, and independent of B2. The 380 sites under `tests/` stay: a test binary is short-lived, so the leak is inert. |

### Recently closed

One line each; the reasoning and measurements are in `CHANGELOG.md`.

| Closed | What |
|---|---|
| 2026-07-31 | **Wire the metrics producer** — `/metrics` stopped rendering zeros; ADR 011's staged producer landed in `crew_runner`. Found and fixed a gauge leak on the cyclic-DAG path in the same change. |
| 2026-07-31 | **`src/order.cyr` → stdlib sort** — 184 → 98 lines; `sort_100k` 79.6 → 20.3 ms, already-sorted 79.1 → 3.31 ms. Public API and bounds contract kept as a wrapper, because `vec_select_nth` aborts where this returns 0. |
| 2026-07-31 | **`src/` mirrors `rust-old/src/`** — 65 `git mv` renames, verified by a byte-identical binary. |
| 2026-07-31 | **bayan `_a` JSON surface** — filed, implemented and released as bayan 1.4.0, folded in cyrius 6.5.5. Was the blocker under B2. |
| 2026-07-31 | **bote `serverInfo` hardcode** — filed and fixed as bote 3.3.0 (`dispatcher_set_server_info`). Pin bump owed under B1. |
| 2026-08-03 | **Bite 15c — SSE (A1), the last file in M6** — `/api/v1/crews/{id}/stream` streams for real. The plan drafted for this was refuted 3/3 on adversarial review and every finding was addressed: the capacity divergence is [ADR 014](../adr/014-sse-stream-holds-a-pooled-worker.md) (the oracle serves *unbounded* streams — tower's permit drops before the body streams), `Closed` reads a flag on our own subscription rather than the bus or chan's layout, `Lagged` terminates *before* draining, ids are canonicalised, all three oracle warns and fallbacks ported. Found and fixed an unauthenticated SIGSEGV on a malformed id while testing. |
| 2026-08-03 | **Graceful shutdown (ADR 013, superseding 012)** — cyrius 6.5.6 vendored sandhi 1.9.9's stop flag, which agnosai had filed hours earlier. `main` installs a `signalfd` SIGINT/SIGTERM handler; `agnosai_serve` returns 0 for a requested stop vs 1 for a failure. Verified live: both signals exit 0 in ~100 ms, an in-flight request still completes 200. |
| 2026-08-03 | **Bite 16 — `src/main.cyr` bind (A2)** — the binary serves for the first time. Env config (`PORT`/`AGNOSAI_PORT`/`HOOSH_URL`/the four auth vars) ports branch-for-branch, `agnosai_serve_parse_port` reproduces `u16::from_str` where neither stdlib parser does, and the epilogue moved to `exit_group` because `SYS_EXIT` exits one thread. Graceful shutdown is deliberately absent — [ADR 012](../adr/012-no-graceful-shutdown-on-sandhi.md). |
| 2026-08-03 | **Dep pins corrected to name what is actually built** — `cyrius.cyml` said bote 3.2.1 / kavach 3.9.3 while `lib/` held 3.3.0 / 3.11.0, because `path = "../NAME"` beats `tag` locally and CI has no sibling checkouts. Both bundles verified byte-identical to their upstream tag dists; all six pins are now the newest upstream tag. Gated going forward by `cyrius deps --verify` in `scripts/check-clean.sh` and a *Lockfile is honest* CI step. |

### C. Upstream — filed and waiting

Nothing here blocks agnosai today; each is a residual agnosai measured and handed off.

| Dep | Open filings |
|---|---|
| cyrius | `2026-07-28-agnosai-no-nlogn-sort-in-stdlib.md` — ✅ **resolved in 6.5.4 and consumed** (`src/order.cyr` is now a wrapper). **Filed 2026-08-03:** `2026-08-03-agnosai-no-sys-exit-group-wrapper.md` (no `sys_exit_group` wrapper — `sys_exit` ends one thread, so a threaded program's idiomatic epilogue hangs the process; repro + measured 124-vs-0 included) and `2026-08-03-sandhi-async-await-readable-has-no-timeout.md` (hardcoded `-1` epoll timeout at `lib/async.cyr:823` makes a cooperative server unwakeable; found while building sandhi 1.9.9's stop facility, worked around there with a bounded sleep). Still open: `2026-07-28-sock-send-result-allocates-per-call.md` (16 B/response, pinned by an exact-bound test in sandhi), `2026-07-29-no-portable-xmkdir-in-io-cyr.md`, `2026-07-29-mutex-unlock-unconditional-futex-wake.md`, `2026-07-29-fmt-int-buf-i64-min.md` |
| sandhi | **Serve-loop stop facility — ✅ FILED, FIXED as sandhi 1.9.9, VENDORED in cyrius 6.5.6, and CONSUMED** (all 2026-08-03). `sandhi_server_options_stop_flag(opts, ptr)` on all five loops; agnosai now drains on SIGINT/SIGTERM ([ADR 013](../adr/013-graceful-shutdown-via-signalfd-and-stop-flag.md), superseding 012). Still open: `backlog` silently ignored by `run_opts`/`run_async`; chunked start hardcodes `" OK"`; **inbound** chunked decoding unsupported (1.9.4 answers 501 — honest, but not support) |
| bayan | **Nothing open.** The `_a` JSON ask shipped as bayan 1.4.0 (folded in cyrius 6.5.5). The YAML ask (`2026-07-16-...`) has **also shipped and this row was stale** — `bayan_yaml_parse` / `_parse_buf` / `_parse_ctx` return a `json_v*` tagged value tree, plus `bayan_yaml_frontmatter_split`. Verified 2026-08-03 by parsing a scalar+sequence document, resolving a key through `bayan_json_v_obj_get`, and re-serializing via `bayan_json_v_build`. **This unblocks M10's YAML half**, which the exclusion table still lists as deferred — revisit that scope call before starting M10. |
| sigil | `2026-07-30-rsa-verify-uses-secret-exponent-ladder.md` — ✅ **archived upstream**, fixed, vendored and **measured** (see C1). |
| bote | `serverInfo` hardcoded to `"bote"` in `dispatcher_dispatch` — ✅ **fixed as bote 3.3.0** (`dispatcher_set_server_info`). Pin bump owed under B1. |

**C1 — ✅ RESOLVED 2026-07-31.** This read *"`cyrius deps` has not been re-run
since the fix landed"*, which is no longer true: `lib/sigil.cyr` is the pinned
**3.12.2** and `lib/` matches the 6.5.5 snapshot exactly. Re-measured on this box:

| benchmark | before (3.12.1 era) | now |
|---|---|---|
| `auth_jwt_verify_ok` | 3.31 ms | **1.202 ms** (2.75x) |
| `auth_jwt_reject_bad_alg` | 3.29 ms | **1.201 ms** |
| `auth_jwt_key_prepare` | 10.7 µs | 10.5 µs |

The per-core JWT ceiling therefore moves from **~300/sec to ~830/sec**. The
`alg` check still sits *after* signature verification by design — parsing
attacker-controlled JSON before authenticating was measured at ~53x heap
amplification — so a rejected token still pays the modexp, which is why the two
rows match. That ordering is unchanged and deliberate; see the CHANGELOG's
Security section for the reasoning. **D1's argument moves with these numbers.**

### D. Decisions deferred to a human

| # | Decision | Where it stands |
|---|---|---|
| D1 | **Mount `rate_limit`?** | Ported and tested (bite 14), **not mounted** — matching the oracle, which never installs the middleware. `agnosai_serve_with_rate_limit` is the opt-in path. Mounting it by default is a **wire change**: clients fine today would start seeing 429s at a threshold agnosai chose, not one the oracle documents. The argument for mounting rested on the JWT-verify ceiling, which **C1 has now re-measured at 1.202 ms — ~830 verifies/sec per core, up from ~300**. That weakens the case for mounting by default without removing it: an unauthenticated flood still costs a modexp per request, because the `alg` check deliberately sits after signature verification. Still a human call. |
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
