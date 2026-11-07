# AgnosAI — Roadmap

> **Forward-facing only.** Every open item is pinned to one release below, and nothing else is here.
> What shipped is in [`CHANGELOG.md`](../../CHANGELOG.md), one entry per release, and leaves this file
> when it ships. Why a design was chosen is in the [ADRs](../adr/). What is true right now (versions, pins,
> test counts, sizes) is in [`state.md`](state.md). [`cyrius-port-plan.md`](cyrius-port-plan.md) is the port's
> reasoning archive, and [`known-unreachable.md`](known-unreachable.md) records code kept for oracle shape.

**Status as of 2.3.1 (2026-11-07).** Labels (B, C, D, F) are never reused, because `state.md`, ADRs and `src/`
comments cite them. A label that is not here is closed, and the CHANGELOG has it.

## What agnosai is

Provider-agnostic agent orchestration in Cyrius: crews, tasks, tools, delegation. Ported from the Rust line
(`rust-old/`), at parity with it and past it. A library with a binary (design principle 5), and every deliberate
divergence from the Rust line's wire carries an ADR.

**The one carve-out, a dependency fact rather than a scope call:** bhava / `personality` is not ported, because
bhava has no Cyrius port to depend on. The wire keeps emitting `null`, which is what the default Rust build
emits. Deferred, never dropped: this is **B5**, the decision is **D2** under *Settled decisions*, and it is the
only item the user ever set aside.

## Design principles

1. **Wire compatibility** — the REST/MCP/A2A surface matches the Rust line's (`rust-old/`; after its deletion,
   `git show 2.1.0:rust-old/`). Every deliberate divergence carries an ADR.
2. **Sandbox by default** — untrusted code never runs unsandboxed
3. **Single binary** — no container orchestration for single-node deployments
4. **Concurrency via `sandhi_server_run_pooled`** — follow daimon, not hoosh (see port plan, "The concurrency decision")
5. **Library first** — agnosai is a library with a binary, not a framework
6. **Lockstep with ai-hwaccel** — aligned versioning, shared practices, same CI rigor

## Performance targets

Carried from the Rust line as *goals*, not as comparisons: the Cyrius baseline starts fresh (see CLAUDE.md).

| Metric | Target |
|--------|--------|
| Boot to ready | <2s |
| Memory (idle) | <100 MB |
| Crew creation | <10ms |
| Concurrent crews | 100+ |
| Fleet msg overhead | <1ms |

## Release pins

One minor line per theme, one patch per piece. Each pin says what ships and what "done" means. A pin is an
intention and can move, but it moves here, on purpose: an item is never "any release". agnostic's roadmap leans
on these minor lines (2.3 tools, 2.4 approvals and budgets, 2.5 durable log, 2.6 MCP and A2A), so a line's theme
does not change without telling it.

| Release | Theme | Items | Waits on |
|---|---|---|---|
| **2.3.2** | Tools in a confined tier | B35, B41 | B40 in kavach |
| **2.3.3** | The WASM tool's lifecycle | B9, B32 | |
| **2.3.4** | Tools on the network | B13, B36 | |
| **2.3.5** | The Rust line's forms and fields | B10, B11 | |
| **2.3.6** | The Rust line's spans, keys and store | B7 (1), B8 (JWT key), B15 (`StateStore`) | |
| **2.3.7** | `rust-old/` deleted | the checklist | D9 |
| **2.4.0** | The crew lifecycle as a contract | F3, B37 | |
| **2.4.1** | Budgets and caps enforced | F5, B6 | |
| **2.4.2** | Compressed hand-off | F9 | |
| **2.4.3** | Hierarchical crews | F2 | F9 |
| **2.5.0** | Durable execution | F4 | F3 |
| **2.5.1** | The binary's crews in arenas | B38 | |
| **2.6.0** | Protocol currency | F8 | bote |
| **2.6.1** | Server edges | B8 (rest), B25 | |
| **2.6.2** | The tracing export | B7 (2–7, 10) | |
| **2.7.0** | What no suite or bench covers | B12, B14 | |
| **2.7.1** | Benchmark levels | B18, B20, B27 | |
| **2.7.2** | Housekeeping | B15 (rest), B34, `[embed]` | |
| **no pin** | Waits on another project | B5, B8 (RSA over 4096), B39, B40, B16, C | see *Waiting on upstream* |

### 2.3.x — the tool-calling harness (agnostic's M6 waits on this line)

2.3.0 shipped the loop ([ADR 029](../adr/029-the-tool-loop.md)), and 2.3.1 chose its models from hoosh's catalog
([ADR 030](../adr/030-a-model-comes-from-the-gateways-catalog.md)).

#### 2.3.2 — tools in a confined tier

**B35. The process, Python and OCI tiers run with no kernel confinement.** All three spawn through
`src/sandbox/spawn.cyr`: a SIGKILL deadline, a controlled working directory, a sanitized environment, and no
seccomp, landlock, namespaces or memory limit (`threat-model.md`, *Accepted Risks*). kavach's confined backend is
bridged and tested (`agnosai_kavach_execute`), and no production tier calls it. Since 2.3.0 a model can reach these
tiers, so this is the threat model's largest gap. **The work:** route each tier through it, keeping its argv,
stdin, environment and deadline semantics, with an ADR, since a tool that relied on network access will notice.

⛔ **Waits on B40.** Routed before kavach stops, every exec on three more tiers keeps 1 MiB, resident, for good.

**B41. What a tool allocates stays on the global heap for good** (found 2026-11-06, ADR 029). `execute_fp` takes
no allocator, so a tool's `ToolOutput` and result are the one thing on the crew path a crew arena cannot give back.
The loop itself leaves 0 B (`tests/orch_tool_loop.tcyr`). **The work:** an allocator on the input
(`agnosai_tool_input_alloc`), with no change to `execute_fp`, and the built-ins moved onto it. **Done means:** the
suite's per-call bound for a real tool reads 0.

#### 2.3.3 — the WASM tool's lifecycle

**B9. WASM validation.** A module with no `_start` export runs and "succeeds", and a module is validated by its
header only, so a malformed body fails at execute. The execute path runs `wasmtime` through kavach's backend
([ADR 019](../adr/019-wasm-tools-spawn-wasmtime-directly.md)).

**B32. A staged module outlives its tool, and nine suites ignore `TMPDIR`.** `agnosai_wasm_load_module` stages at
`/tmp/agnosai-wasm-<uuid>.wasm`, and `agnosai_wasm_module_free` has no caller outside the suites, so
`DELETE /api/v1/tools/{name}` or a re-register leaves the file for the life of the host. Freeing on removal is not
enough: another thread may hold the tool and not yet have started wasmtime, so the free must wait for in-flight
executions (a reference count on the tool, or unlink once wasmtime has opened it). Stage under `$TMPDIR`. In the
suites, `server_routes_sse`, `sandbox_oci` and `sandbox_manager` never unlink, and `orch_durable_state`,
`definitions_packaging`, `definitions_loader`, `sandbox_wasm`, `orch_ipc` and `tools_wasm` unlink only some, so two
runs at once collide. agnostic 0.1.15 fixed the same thing with a shared `tests/support/tmp.cyr`.

#### 2.3.4 — tools on the network

**B13. Network-facing tools.** The load tester has no connection pool, so every request is a fresh handshake, and
its success path and the security audit's live sequence have no test, since no suite stands up a loopback server.

**B36. `GET /api/v1/tools`, MCP `tools/list` and a task's `metadata` keys come out in a random order per process.**
The registry and the metadata map are hash maps, and `lib/hashmap.cyr` seeds per process. The Rust line's was
random too, so this is wire-compatible, but a stable order by name is kinder to clients. Measure
`tool_registry_list_{50,500}` before and after. (A crew's `tools` request is already in the agent's order, ADR 029.)

#### 2.3.5 — the Rust line's forms and fields

**B10. JSON forms the Rust types derived and nothing serialised.** Writers: the fleet types (`NodeInfo`,
`NodeStatus`, `NodeTopology`, `DeviceLink`, `InterconnectType`, `FederationRole`, `ClusterStatus`, `ClusterInfo`,
`RelayMessage`), `RetryConfig`, `ToolInput`, the manifests, `KavachToolResult`. Parsers: `TaskProfile`,
`BudgetExceeded`, `CachedPlan`, `ToolSchema`, `ToolOutput`.

**B11. Structured-log fields.** Events that lost their fields or are absent: fleet (17, federation's nine above
all; it logs through plain `sakshi_info`), the WASM sandbox and tool (no sakshi calls at all), the wasm loader's
and remote registry's error details, the kavach bridge, process and OCI exit lines, the MCP tool-call line's
`success` and `duration_ms`, a2a rejections' id prefix and `metadata_bytes`, `latency_ms` in task-result metadata.

#### 2.3.6 — the Rust line's spans, keys and store

**B7 (1). The rest of the `#[tracing::instrument]` spans.** The Rust tree exported 20. agnosai exports the four
GenAI operations; sandbox, routes, scoring and cost planning have none. Under ADR 023's explicit span-context
threading, with no thread-local. The routes' HTTP server spans can use `agnosai_http_server_span_a` (ADR 024),
which only an embedding consumer calls so far.

**B8 (certificate-format JWT key).** The Rust line took one, and `auth.cyr` takes SPKI and PKCS#1 PEM only.

**B15 (`StateStore`).** The pluggable persistence trait. `src/orchestrator/durable_state.cyr`'s header records why it
is flat today: one implementor, no callers. F4 builds on what lands here.

#### 2.3.7 — `rust-old/` deleted

The first release that can meet the user's condition, "if all the rust porting items are done" (*Settled
decisions*). Every Rust spec stays reachable after: `git show 2.1.0:rust-old/src/<path>`. ⛔ **Needs D9.**

- [ ] `git rm -r rust-old`, then delete the ignored `rust-old/target/` (~13 GB) from disk, which `git rm` leaves.
- [ ] Update the text that describes it: CLAUDE.md (identity line, layout section, the DO NOT rule; the user's
      file), `CONTRIBUTING.md`, `docs/architecture/overview.md`, and README.md's note, tree line and footer.
- [ ] Prune `.gitignore`, still the Rust-era file: `/target`, `**/target/`, `**/*.rs.bk`, `criterion/`,
      `tarpaulin-report.*`, `.cargo/`, and a `Cargo.lock` comment naming `agnosai-server`. Keep what
      `sdk/agnosai-tool-sdk/` needs, as it stays a Rust crate (*Settled decisions*).

The ~650 comment lines citing a `rust-old/` path stay as historical pointers. Nothing breaks if they dangle.

### 2.4.x — the orchestration contract

Source for the harness rules below: a comparison with [turnstone](https://github.com/turnstonelabs/turnstone)
("the model proposes; the gate disposes"), mirrored in agnostic's roadmap (*Waiting on agnosai*), which holds
agnosai to them.

#### 2.4.0 — the crew lifecycle as a contract

**F3. Wire the approval gate, and specify the crew and task lifecycle as one state machine.**
`orchestrator/approval.cyr` is built and its routes exist, but `crew_runner` never calls it. Add
`awaiting_approval` to the task and crew states, and write the lifecycle down: the states, the legal transitions,
and a **per-crew monotonic `seq` on every event**. Test it as an invariant: no event without a legal transition,
and no `seq` gap. Build on [ADR 022](../adr/022-crew-events-and-status-say-what-happened.md). It inherits two gaps:
a parallel batch reports `task_completed` in dispatch order where the oracle's `join_next` reports completion
order, and the error arm's `running` → `pending` revert, which should become a terminal state (B37). The model is
herdr's self-report contract and its `blocked` roll-up. Tool calls get their events here too, as a watcher sees
nothing of a task's turns today (ADR 029). **A model judge may only tighten:** if approvals gain an LLM judge, it
may refuse what the rules allow and never allow what they refuse, and it gives its reasons from a fixed list. An
automatic approval covers a whole batch or none of it, and anything uncertain goes to a human.

**B37. A cyclic crew stays PENDING, and its event channel stays open.** The run-error arm of
`_agnosai_orch_finish_err` stores no terminal state (ADR 022) and returns before `agnosai_event_bus_remove`, so a
submitted crew whose DAG is cyclic reads `pending` for good and a subscriber waits for good. **Done means:** a
submitted crew that cannot run reaches a terminal state, with its reason, and its channel closes. It decides ADR
022's "the crew never ran", so it wants a line in that ADR.

#### 2.4.1 — budgets and caps enforced

**F5. Enforce budgets and caps.** `budget.cyr` and `multi_tenant.cyr` have no callers outside their own files; only
`max_duration_secs` is read. Enforce token and cost per crew and per tenant, plus max tasks per crew. Multi-agent
runs about 15× chat tokens (Anthropic 2025), and Claude Code caps subagents at 20 concurrent and depth 3. agnostic
removed `max_concurrent_tasks` because nothing enforced it, and gets it back on this. **Budgets subdivide** down the
tree: crew, then task, then delegated worker (F2). The tool loop's step limit and transcript budget are
process-wide until this lands (ADR 029).

**B6. Crew concurrency.** No cap on concurrent crews: `submit_crew` spawns a thread per call, where the Rust tree
held a semaphore of `max_concurrent_tasks` (default 10). Parallel mode runs fixed batches, so one slow task idles
the other slots until its batch ends. Both want a counting semaphore, which the stdlib does not have. sigil's lane
pool (*C*) wants the same one, capping outbound TLS at 64.

#### 2.4.2 — compressed hand-off

**F9. Hand downstream tasks compressed context.** A summarizing strategy beside Full, SlidingWindow and HeadTail
(`orchestrator/memory.cyr`): a dependent task receives its upstream's `expected_output` or summary, not the whole
transcript. F2 needs it. **A summary is as untrusted as what it summarised:** a hand-off built from tool output
carries that output's trust, so it reaches the next task as data (ADR 029's rule 2).

#### 2.4.3 — hierarchical crews

**F2. Wire hierarchical (manager → workers).** `orchestrator/hierarchical.cyr` is built and tested, and
`crew_runner.cyr` logs and falls back to sequential. Orchestrator-worker is where the field converged, and
Google/DeepMind's 2025 scaling study measured error amplification of 17.2× for independent agents against 4.4×
centralized. Workers return summaries, not transcripts (F9). agnostic drops its `hierarchical` refusal on the
re-pin. **Constraint from F7:** `agnosai_explain_selection_a` is exact only while selection is a pure function of
roster and task, so F2 must record its choice. **Authority only narrows:** a worker holds at most its manager's
tools and budget, and a need beyond them goes up to a human. Today a delegated crew's agents hold whatever their
own definitions name, however little the delegating agent holds, and that is the gap this closes.

### 2.5.x — durable execution

#### 2.5.0 — durable execution

**F4. An event-sourced crew log, resumable from the last completed task.** `orchestrator/durable_state.cyr` has zero
callers. This is baseline in the field now: LangGraph checkpointers, Pydantic AI on Temporal, MAF's Durable Task,
OpenHands' event sourcing. Build on F3's `seq` stream. Today agnostic answers a restart by marking crews
`interrupted`; this is what would let it resume them. `durable_state` is on `lib/io.cyr`, not patra, and an
event-sourced log may want patra's jsonl mode, which opens `O_APPEND` without `O_TRUNC`. The tool journal ADR 029
gives a consumer becomes agnosai's own here, so a resume can name its unknown effects.

#### 2.5.1 — the binary's crews in arenas

**B38. The binary's own crews stay on the global heap.** `POST /api/v1/crews` builds its spec with
`agnosai_crew_from_value`, so the server keeps about 9 KiB per crew for good, past eviction, plus each crew's tool
records since 2.3.0. A route reads a registry entry with no hold on it, so eviction cannot release under it. **The
work:** a reader count on a registry entry, or a copy of what a route renders, then release on eviction. **Done
means:** `tests/orch_crew_arena.tcyr`'s loop reads 0 B through the REST path.

### 2.6.x — protocol and server currency

#### 2.6.0 — protocol currency

**F8.** (a) `/mcp` speaks the `initialize`-handshake revision. MCP 2026-07-28 removed the handshake and
`Mcp-Session-Id`, added the `Mcp-Method` and `Mcp-Name` headers, made **Tasks** an official extension, and
deprecated Sampling, Roots and Logging. The dispatch belongs to bote, so this starts as a **bote filing**. `/mcp`
hand-builds its JSON-RPC envelope as the oracle did rather than delegating to bote's `Dispatcher`, and delegating
is a separate decision. `capabilities` advertises no `subscribe` or `listChanged`
([ADR 015](../adr/015-mcp-resources-project-agent-definitions.md)). (b) Target A2A v1.0.x.

#### 2.6.1 — server edges

**B8 (rest).** An SSE stream holds one of the 100 pool workers for the crew's life and does not notice a departed
client: `src/server/routes/sse.cyr` discards all six write results sandhi now returns. Shutdown does not drain
in-flight requests. Path parameters are not percent-decoded. `hot_config` has no change notification.

**B25. The serve handler's arena read of `traceparent` is not covered.** `sandhi_server_find_header_a(…,
"traceparent")` runs only on the arena branch, which needs a sandhi worker arena, and the pipe harness reaches only
the bare branch, as it does for every header. **Fix:** a harness that runs one request through a real pooled worker
with an arena, asserting that the crew's spans carry the inbound trace id.

#### 2.6.2 — the tracing export

**B7 (2–7, 10).** (2) trace and span ids in JSON log lines, and per-module targets, which needs sakshi's log hook to
expose them; (3) aggregate usage on `invoke_agent`; (4) `gen_ai.request.temperature` and `max_tokens` on `chat`,
declared and never set; (5) `gen_ai.response.finish_reasons`, a `string[]` needing an OTLP `arrayValue` the encoder
does not write; (6) `Span.flags` with the W3C is-remote bit; (7) `tracestate` passthrough; (10) **an operator switch
for an untrusted inbound context:** any caller of the three traced routes picks the trace id agnosai's spans join,
and can turn their export off with sampled flag `0` (threat-model surface 7, ADR 023). **Fix direction:**
`AGNOSAI_OTEL_INBOUND=honour|restart|record`, read at boot.

### 2.7.x — quality

#### 2.7.0 — what no suite or bench covers

**B12. Behaviours no suite asserts yet.** Orchestrator: a diamond DAG's ready set and a run of one (its sort is
pinned), parallel and DAG `task_ms`, the cancel audit entry, per-task audit level, a 64 KiB+ IPC echo through
send and receive, and **concurrent cancel**: mid-execution interruption and parallel/DAG cancel stress, which the
Rust tree never tested either. Core: `Cpu` → `"cpu"`, multi-device filtering. Sandbox: real fuel exhaustion (-2),
empty stdin to a guest that reads, the Python bridge's `"error": null`. Tools: AGNOS schema parameter types, the echo
tool's parameter type. Server: the output filter's pattern text past AWS and `custom`.

**B14. Seven benchmarks.** The sandbox execute path (process argv, stdin, shell; WASM hello) and
`audit_recent(10/100/1000)`.

#### 2.7.1 — benchmark levels

**B18. `lt_aggregate_100k_500workers` reads about 2x its 2026-08 band.** Not the compiler, not the benchmarked
source, not noise (`state.md`, "⚠ OPEN — a ~3.6x regression, found at 2.0.6"). Start from the 2026-08-03 `_a`
allocator work, a ~6-round bisect over 40 commits.

**B20. A second live thread moves a crew's cost, both ways.** An idle detached thread took
`run_crew_10_tasks_sequential` +20% and `run_crew_10_tasks_parallel_4` −27%, so a row's number depends on what ran
before it, and a production server always has other threads. **Direction:** find which stdlib path switches on a
second live thread (mutex/futex, the allocator, `thread_create`, stack mmap). File it against cyrius if it is the
stdlib. Then decide whether `benches/orch.bcyr` pins the level by starting one idle thread in `main`.

**B27. `crew_runner_10_tasks_parallel_4_{quiet,watched}` measure the global heap's position, not the runner.** They
cannot simply run first: 2.2.3 tried, and about 50 rows then read 25–157% slower at the threads-have-existed level
(B20). **The fix is a file of their own**, `benches/crew_runner.bcyr`, with copies of `_b_o_spec` and the stub chat,
then a rebase of their history.

#### 2.7.2 — housekeeping

**B15 (rest).** Output-filter leak detection folds ASCII case only. A pub/sub subscriber cannot detach alone, only
`unsubscribe_all(pattern)`. `strcase` is ASCII-only though the crew assembler matches user-supplied names.

**B34. `lib/<file>:<line>` citations in `src/` comments drift with every fold.** 63 of them. Convert each to the
`` `lib/x.cyr`'s `fn_name` `` form by hand, as an automated rewrite picked the wrong function for some, then gate
new ones.

**`[embed]` for the presets.** `scripts/gen-presets.sh` and the checked-in `src/definitions/presets_data.cyr` can
retire for cyrius's `[embed]` (6.6.19): `[embed] NAME = "path"` gives every compile `NAME()` and `NAME_len()`, read
at build time, explicit entries only. All embeds share cycc's 2 MiB string pool, and every binary of the project
carries every declared embed. One entry per preset, in order, so `_AGNOSAI_PRESET_JSON_NN` and
`agnosai_preset_json(i)` become those calls. **Stays ours:** compaction, as `[embed]` takes the bytes verbatim.

## Waiting on upstream

Nothing here blocks a pin above unless its row says so. Each is taken in the first release after the project ships
it, so none has a number of its own.

| Item | Waits on | State | agnosai's half |
|---|---|---|---|
| **B5** `personality` | bhava | ⛔ No Cyrius port. `personality`, `with_personality`, `mood_adjusted_temperature`, the personality score and the system-prompt block | Port them against it. D2 |
| **B8** RSA over 4096 bits | sigil | Filed (sigil `docs/development/issues/2026-11-05-agnosai-rsa-over-4096-bits-cannot-verify.md`). `_rsa_recover_em`'s `n_len > 512`; the Rust line took 8192 | Nothing until then. D9 |
| **B39** 480 B per exchange | sandhi | Filed, as an addendum to sandhi `docs/development/issues/2026-11-05-agnostic-outbound-calls-keep-bytes-on-the-global-heap.md`. 328 B is `_sandhi_http_recv_framed`'s bare header parse (about 40 B per response header) and 120 B `_sandhi_client_host_header_a`'s builder | Tighten `_T_SANDHI_BYTES_PER_CALL` to 0 in `tests/orch_crew_arena.tcyr`, and the bounds in `telemetry_otlp` and `llm_hoosh`. A `chan_reopen` in the stdlib would retire `agnosai_chan_reopen`'s reach into `closed`, not filed |
| **B40** 1 MiB per exec | kavach | Filed (kavach `docs/development/issues/2026-11-06-agnosai-every-exec-keeps-its-1-mib-capture-buffer.md`). All nine backends `alloc(out_cap)` per exec: 400 execs took RSS from 9.5 MB to 421 MB | `tests/sandbox_wasm.tcyr`'s cost of a call reads near 0. ⛔ **Gates 2.3.2** |
| **B16** chunked requests | sandhi | Filed. `sandhi_server_request_has_unsupported_te` answers 501 | Nothing: out of scope until it lands |
| **C** lane pool | sigil | Filed (sigil `docs/development/issues/2026-11-05-agnosai-lane-pool-refuses-the-overflow-of-a-burst.md`). Of 100 threads, 64 verify and 36 are refused, so a burst fails a TLS handshake. `tests/server_auth_lane_race.tcyr` pins the RSA verify's lane-freedom | Cap outbound TLS at 64 with B6's semaphore |
| **C** span ids | hoosh | **To file.** hoosh exports its span with the inbound parent-id as its own spanId and no `parentSpanId` (`src/lib/otlp.cyr`), then forwards that traceparent. **The ask:** mint a span id, emit the inbound one as `parentSpanId`, forward hoosh's own | `agnosai_hoosh_chat_in` adds an outbound `traceparent` from its `chat` span. It sends none today, or hoosh's span would repeat the chat span's id |
| **C** catalog fields | hoosh | **To file.** `GET /v1/models/catalog` has no tier, no tools bit and no way to tell a chat model, though hoosh's table holds the first two for each model it knows and each route knows what it serves, so agnosai reads a tier from prices and capabilities, cannot keep an agent with tools off `gpt-6-astra`, and keeps hoosh's twelve non-chat words (ADR 030). **The ask:** `tier`, `tools` in `capabilities`, and a `kind` or an `endpoints` list. The word filter also skips Ollama's tags and the compiled table at hoosh, so a picker offers `nomic-embed-text` as a chat model | Read them in `_agnosai_model_pick_a` and `_agnosai_model_is_chat`, and pass a task's need for tools to the pick. Nothing until then |
| **C** error bodies | hoosh | **To file.** `_resp_err` writes a provider's raw text into a JSON string unescaped, so a provider 404 with no `message` key (Ollama's) answers `{"error":{"message":"upstream provider returned HTTP 404: {"error": …`, which no client can parse. Measured on 2.10.1. **The ask:** `sb_add_json_escaped`, which `provider_err.cyr` already has | Drop `_agnosai_hoosh_detail_a`'s raw scan, and take the detail off the tree |
| **C** catalog GET | hoosh | **To file.** `handle_models_catalog` builds on the heap that never frees: 15.4 KB per GET at 9 entries, measured on 2.10.1 over 3,000 GETs. Every picker open pays it. **The ask:** the request arena its ADR 013 plans, or a cached body rebuilt at each refresh | Widen `_AGN_CAT_REREAD_MS`'s reasoning, as the ration is there for this |
| **C** MCP client | bote | **To file.** bote 3.8.15 has `HostRegistry` entries and no client send path. **The ask:** `initialize`, `tools/list` and `tools/call` to a server named there. agnostic records the same | Register each remote tool with `agnosai_tool_new`: the registry and the loop need nothing else (ADR 029, rule 4) |
| **C** server verbs | sandhi | **Not filed.** `backlog` is ignored by `run_opts` and `run_async`, and the chunked start hardcodes `" OK"` | None needed today |

bayan's refusals of block scalars, flow mappings, anchors and tags are deliberate (its ADR 0008), so that is a scope
request to raise only if a definition file needs one, and it is not owed.

## Decisions for the user

> ⚠ **A decision already made does not belong here.** D2 sat here for weeks after bhava was settled and got asked
> again. When an item is decided, move it to *Settled decisions*, written out.

| # | Decision | Needed by | Where it stands |
|---|---|---|---|
| **D9** | **Are B5 and RSA over 4096 exempt from "every Rust porting item is done"?** | 2.3.7 | B5 is the carve-out the user set (D2), so it reads as exempt, but that is the user's call. Both are blocked upstream, so the alternative is the deletion waiting on bhava and sigil |
| **D8** | **Make CI's Lockfile step fail on a drifted lock (`cyrius deps --locked`)?** | Any time | `CLAUDE.md` has CI resolve the lock rather than audit a committed one. Since cyrius 6.7.6 a plain local `cyrius deps` writes the same 8-pin lock CI does, so `--locked` would now catch a tag that moved upstream. Adopting it changes the policy that file states |
| **D7** | **Note the 2.1.5 exception in the CHANGELOG, or change its policy line?** | Any time | `CHANGELOG.md:6` claims Semantic Versioning, but 2.1.5 shipped a `### Breaking` section under a patch number (D6) and neither says so |

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
- **Not adopted: free-form swarms or parallel writers as an engine mode, and screen-scraping coordination**
  (2026-10-03). The evidence runs against the first: MAST (NeurIPS 2025), Cognition 2026, and Tran & Kiela 2026
  (single agent ≥ multi-agent at an equal token budget). agnosai has structured events, which is why it needs
  no herdr-style scraping.
- **Federation adopts any declaration whose term is not stale** (2026-11-05, the former B30 item 4).
  `agnosai_federation_declare_coordinator` keeps its `term < current` guard. Adopting a higher term is the
  leader-election rule. Refusing a conflicting declaration at the same term would buy nothing, because with no
  vote a peer could declare at term+1 instead. Federation trusts its peers; `threat-model.md` surface 4 says so,
  and the reasoning is in `src/fleet/federation.cyr`'s header.
- **`rust-old/` goes once every Rust porting item is done** (2026-11-05, the user: "if all the rust porting
  items are done then deleting at 2.3.0 is fine"). The condition was not met at 2.3.0, so the deletion is pinned
  to 2.3.7, the first release that can meet it. Whether the two blocked items are exempt is **D9**.
- **`assert` stays in `[deps] stdlib`; only `bench` moved to `[test] stdlib`** (2026-11-05, 2.2.3 audit).
  `src/learning/optimizer.cyr` calls the stdlib's `panic()`, which lives in `lib/assert.cyr` and is documented
  there as the runtime guard for library code. So `assert` is a real production leaf, and a consumer's sidecar
  naming it is correct, not test-leaf leakage.
- **The MCP client is bote's** (2026-10-07). A crew's agents reach external MCP servers through
  bote, by way of agnosai's tool registry (F1); neither agnosai nor agnostic carries its own MCP
  client. bote is the ecosystem's MCP layer, built so that apps stop implementing MCP each for
  themselves. The client half is not built yet (*C*, bote). Recorded in agnostic's `roadmap.md`
  (*Settled decisions*) the same day.

## Out of scope

- The bhava carve-out (see *What agnosai is*).
- Inbound chunked request bodies: sandhi answers 501, honest but not support (*Waiting on upstream*).
- Any comparison of Cyrius benchmark numbers against the frozen Rust CSV.
- `fleet/discovery` beyond the oracle's own 174-line stub, which `rust-old` calls a stub itself. `lib/net.cyr` has
  no SRV resolver. Finishing it is a scope decision nobody has made, not owed work.
- Python bindings: demand-gated, a separate crate (`cdylib`, maturin).

### A note, not a filing: an OpenTelemetry library

There is no Cyrius OTel library, and hoosh (`src/lib/otlp.cyr`) and agnosai (`src/telemetry/otlp.cyr`) hand-rolled
the same subset, because ADR 003 forbids depending on hoosh as a library. By the port's rule (`CLAUDE.md`,
*Refactoring*) the third instance is when the abstraction gets extracted. It would be a sibling repo with its own
`dist/` bundle, not a stdlib module, and naming is the user's call. It would own OTLP/protobuf (itself probably a
separate codec repo), the span model, context propagation with an explicit-context API (ADR 023 records why
thread-local cannot serve agnosai), a batching exporter, and metrics and logs. `src/telemetry/otlp.cyr` would
collapse to an adapter, as `fleet/relay` did onto majra.
