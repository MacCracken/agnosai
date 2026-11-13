# AgnosAI — Roadmap

> **Forward-facing only, by release.** Each release below says what it does, in the order the releases can ship.
> What shipped is in [`CHANGELOG.md`](../../CHANGELOG.md) and leaves this file when it ships. Why a design was chosen is
> in the [ADRs](../adr/). What is true right now (versions, pins, test counts, sizes) is in [`state.md`](state.md).
> [`cyrius-port-plan.md`](cyrius-port-plan.md) is the port's reasoning archive, and
> [`known-unreachable.md`](known-unreachable.md) records code kept for oracle shape.

**Status as of 2.6.0 (2026-11-13).**

**How releases are ordered.** A release holds only work that can ship now. Work that waits on another project is not
given a number: it is listed under [*Waiting on another project*](#waiting-on-another-project), with its filed issue,
and takes the next free release once the fix ships. A pin can move, but it moves here, on purpose. agnostic's roadmap
leans on the minor lines (2.3 tools, 2.4 approvals and budgets, 2.5 the durable log, 2.6 MCP and A2A), so a line's
theme does not change without telling it.

## What agnosai is

Provider-agnostic agent orchestration in Cyrius: crews, tasks, tools, delegation. Ported from the Rust line
(`rust-old/`), at parity with it and past it. A library with a binary (design principle 5), and every deliberate
divergence from the Rust line's wire carries an ADR.

**The one carve-out, a dependency fact rather than a scope call:** bhava's `personality` is not ported, because bhava
has no Cyrius port to depend on. The wire keeps emitting `null`, which is what the default Rust build emits. Deferred,
never dropped: it is under *Waiting on another project*, and it is the only item the user ever set aside.

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

## Releases

| Release | What it does |
|---|---|
| **2.6.1** | Server edges: SSE, shutdown, paths, config, JWT keys |
| **2.6.2** | The tracing export: the remaining spans, and trace ids in log lines |
| **2.7.0** | Tests and benchmarks for what nothing covers yet |
| **2.7.1** | Benchmark levels that measure the code, not the heap |
| **2.7.2** | Housekeeping |

### 2.3.x — the tool-calling harness (agnostic's M6 waits on this line)

2.3.0 shipped the loop ([ADR 029](../adr/029-the-tool-loop.md)), 2.3.1 chose its models from hoosh's catalog
([ADR 030](../adr/030-a-model-comes-from-the-gateways-catalog.md)), 2.3.2 gave each tool call an arena of its own,
2.3.3 chose a model by hoosh's tier, kind and tools bit (ADR 030's addendum), 2.3.4 confined the process and Python
tools with kavach's primitives in agnosai's own spawner, and hardened the OCI tier's container
([ADR 031](../adr/031-confine-the-process-and-python-tiers-in-agnosais-spawner.md)), 2.3.5 validated a WASM module at
load, freed it with its tool, and took the spawner's idle sleep out (ADR 019's 2.3.5 addendum), 2.3.6 gave each
simulated user of a load test one connection, took a tool's capabilities away under a root server, moved the URL check
off the global heap, and put the tool list and a task's keys in order, 2.3.7 gave the Rust line's types their JSON
forms and its log events their fields, and 2.3.8 took the network from a process or Python tool whose policy denies it
(ADR 031's 2.3.8 addendum) and sent the chat span's `traceparent` to hoosh (ADR 023's 2.3.8 addendum). The line is
done.

### 2.4.x — the orchestration contract

Source for the harness rules below: a comparison with [turnstone](https://github.com/turnstonelabs/turnstone) ("the
model proposes; the gate disposes"), mirrored in agnostic's roadmap (*Waiting on agnosai*), which holds agnosai to
them.

2.4.0 wrote the crew lifecycle down as one state machine and numbered its events, wired the approval gate, and gave a
crew that cannot run a terminal state, a task's tool calls their events, and a batch's completions their own order
([ADR 032](../adr/032-the-crew-lifecycle-is-one-state-machine.md)). Its rules for a model judge, should one be added,
are written there. 2.4.1 enforced budgets and caps: crews at once, outbound TLS at 64, a parallel crew as a sliding
window, and token and cost per tenant, crew and task, each under the one above it
([ADR 033](../adr/033-budgets-caps-and-crew-concurrency-are-enforced.md)). 2.4.2 handed a dependent task its
dependencies' answers, as data, in a DAG and in a sequential crew
([ADR 034](../adr/034-a-dependent-task-is-handed-its-dependencies-answers.md)). 2.4.3 ran a hierarchical crew as its
name says: its manager, an agent of its own named by key, assigns each task to its best worker, each worker holds at
most its manager's tools, and the manager writes the crew's answer from every worker's; the `delegate` tool bounds the
crew it starts by its caller's tools and what its task has left
([ADR 035](../adr/035-a-manager-assigns-its-workers-and-writes-the-crews-answer.md)). The line is done.

### 2.5.x — durable execution

2.5.0 gave a crew a log it resumes from: its spec, every event numbered, each tool call journaled before it runs and
each finished task's result, through a `StateStore` that is now a table of a backend's verbs. A resume keeps what
finished, asks a reviewer before re-running a task whose calls may have taken effect, and goes on numbering where the
log stopped; the server resumes at start-up with `AGNOSAI_STATE_DIR`; and a snapshot read under the bus lock lets a
reader that fell behind its events resync ([ADR 036](../adr/036-a-crew-keeps-a-log-and-resumes-from-it.md)).

2.5.1 put the server's own crews in arenas its orchestrator gives back: a crew from `POST /api/v1/crews`, an A2A
delegation or the `delegate` tool goes back once its registry lets it go and no route holds it, every route that reads
one holds it and copies what it renders, and the REST loop leaves 0 B a crew where 2.5.0 left 13,144
([ADR 037](../adr/037-the-servers-crews-are-the-orchestrators-to-give-back.md)).

2.5.2 gave the audit chain back what it lets go: each entry lives in a slot of its ring, an arena the chain resets when
the ring takes the slot again, with its strings and a copy of its metadata, and the hashing goes in a scratch that is
given back. With the server's chain on the REST loop reads 0 B a crew, where it kept 6,360
([ADR 038](../adr/038-an-audit-entry-lives-in-its-ring-slot.md)). The line is done.

### 2.6.x — protocol and server currency

2.6.0 brought the protocols current: `/mcp` serves MCP 2026-07-28 beside 2025-03-26, deciding per request, its
headers checked against the body, by hand after bote's era rules
([ADR 039](../adr/039-mcp-serves-both-eras-on-one-endpoint.md)); and agnosai speaks A2A 1.0, an Agent Card the
operator can sign and a JSON-RPC endpoint whose tasks are crews, streamed over SSE
([ADR 040](../adr/040-a2a-v1-tasks-are-crews.md)).

#### 2.6.1 — server edges: SSE, shutdown, paths, config, JWT keys

An SSE stream holds one of the 100 pool workers for the crew's life and does not notice a departed client:
`src/server/routes/sse.cyr` discards all six write results sandhi now returns. Shutdown does not drain in-flight
requests. Path parameters are not percent-decoded. `/mcp` does not check `Origin`, in either MCP era, which both
revisions ask of a server against DNS rebinding (ADR 039). An MCP `tools/call` keeps **601 B a call** on the global
heap, in either era, its tool's input and output built there: measured at 2.6.0 in
`tests/server_routes_mcp_2026.tcyr`, which asserts only that the newer era adds nothing to it. `hot_config` has no change notification. A certificate-format JWT
key is refused, where the Rust line took one: `auth.cyr` takes SPKI and PKCS#1 PEM only.

**The serve handler's arena read of `traceparent` is not covered.** `sandhi_server_find_header_a(…, "traceparent")`
runs only on the arena branch, which needs a sandhi worker arena, and the pipe harness reaches only the bare branch, as
it does for every header. **Fix:** a harness that runs one request through a real pooled worker with an arena,
asserting that the crew's spans carry the inbound trace id.

#### 2.6.2 — the tracing export: the remaining spans, and trace ids in log lines

**The rest of the Rust tree's spans.** It exported 20 through `#[tracing::instrument]`, and agnosai exports the four
GenAI operations, so sandbox, routes, scoring and cost planning have none. Under ADR 023's explicit span-context
threading, with no thread-local. The routes' HTTP server spans can use `agnosai_http_server_span_a` (ADR 024), which
only an embedding consumer calls so far.

**And the rest of the export:**

- **Trace and span ids in JSON log lines, and per-module targets.** sakshi's emit hook already passes a log line's
  fields apart (sakshi 2.4.10, `src/output.cyr`), so agnosai passes the ids and the target as fields and its hook
  writes them where the format wants them. Nothing waits on sakshi.
- Aggregate usage on `invoke_agent`.
- `gen_ai.request.temperature` and `max_tokens` on `chat`, declared and never set.
- `gen_ai.response.finish_reasons`, a `string[]`, which needs an OTLP `arrayValue` the encoder does not write.
- `Span.flags` with the W3C is-remote bit.
- `tracestate` passthrough.
- **An operator switch for an untrusted inbound context.** Any caller of the three traced routes picks the trace id
  agnosai's spans join, and can turn their export off with sampled flag `0` (threat-model surface 7, ADR 023).
  **Fix direction:** `AGNOSAI_OTEL_INBOUND=honour|restart|record`, read at boot.

### 2.7.x — quality

#### 2.7.0 — tests and benchmarks for what nothing covers yet

**Behaviours no suite asserts yet.** Orchestrator: a diamond DAG's ready set and a run of one (its sort is pinned),
parallel and DAG `task_ms`, the cancel audit entry, per-task audit level, a 64 KiB+ IPC echo through send and receive,
and **concurrent cancel**: mid-execution interruption and parallel/DAG cancel stress, which the Rust tree never tested
either. Core: `Cpu` → `"cpu"`, multi-device filtering. Sandbox: real fuel exhaustion (-2), empty stdin to a guest that
reads, the Python bridge's `"error": null`. Tools: AGNOS schema parameter types, the echo tool's parameter type. Server:
the output filter's pattern text past AWS and `custom`.

**Seven benchmarks.** The sandbox execute path (process argv, stdin, shell; WASM hello) and `audit_recent(10/100/1000)`.

#### 2.7.1 — benchmark levels that measure the code, not the heap

**`lt_aggregate_100k_500workers` reads about 2× its 2026-08 band.** Not the compiler, not the benchmarked source, not
noise (`state.md`, "⚠ OPEN — a ~3.6x regression, found at 2.0.6"). Start from the 2026-08-03 `_a` allocator work, a
~6-round bisect over 40 commits.

**A second live thread moves a crew's cost, both ways.** An idle detached thread took `run_crew_10_tasks_sequential`
+20% and `run_crew_10_tasks_parallel_4` −27%, so a row's number depends on what ran before it, and a production server
always has other threads. **Direction:** find which stdlib path switches on a second live thread (mutex/futex, the
allocator, `thread_create`, stack mmap). File it against cyrius if it is the stdlib. Then decide whether
`benches/orch.bcyr` pins the level by starting one idle thread in `main`.

**`task_four_tool_calls_at_once` and `_kept` read 150 µs at the end of `benches/orch.bcyr` and 250 µs alone**
(measured at 2.4.0, both builds alike, three runs each). A 2.4.0 build read them 38% faster than 2.3.8's in the A/B
with nothing on their path changed, so they move with what the rows before them leave behind, and want a file of their
own as much as the two below.

**`crew_runner_10_tasks_parallel_4_{quiet,watched}` measure the global heap's position, not the runner.** They cannot
simply run first: 2.2.3 tried, and about 50 rows then read 25–157% slower at the threads-have-existed level (above).
**The fix is a file of their own**, `benches/crew_runner.bcyr`, with copies of `_b_o_spec` and the stub chat, then a
rebase of their history.

#### 2.7.2 — housekeeping

- Output-filter leak detection folds ASCII case only. A pub/sub subscriber cannot detach alone, only
  `unsubscribe_all(pattern)`. `strcase` is ASCII-only though the crew assembler matches user-supplied names.
- **The injection scan reads a text once per pattern.** `agnosai_prompt_scan_input` makes 31 case-blind passes:
  347 µs for 4 KiB (`prompt_scan_clean_4k`), so a 32 KiB tool result or dependency's answer costs about 2.8 ms of
  scan, and a hand-off of three 4 KiB answers 1.05 of its 1.5 ms (measured at 2.4.2). One pass that tries only the
  patterns a byte's lower case can start must still answer the **first pattern in table order** that matches anywhere,
  not the first position, as the description it returns is the table's.
- **`lib/<file>:<line>` citations in `src/` comments drift with every fold.** 63 of them. Convert each to the
  `` `lib/x.cyr`'s `fn_name` `` form by hand, as an automated rewrite picked the wrong function for some, then gate new
  ones.
- **`[embed]` for the presets.** `scripts/gen-presets.sh` and the checked-in `src/definitions/presets_data.cyr` can
  retire for cyrius's `[embed]` (6.6.19): `[embed] NAME = "path"` gives every compile `NAME()` and `NAME_len()`, read
  at build time, explicit entries only. All embeds share cycc's 2 MiB string pool, and every binary of the project
  carries every declared embed. One entry per preset, in order, so `_AGNOSAI_PRESET_JSON_NN` and
  `agnosai_preset_json(i)` become those calls. **Stays ours:** compaction, as `[embed]` takes the bytes verbatim.
- **`durable_state` hand-rolls a recursive mkdir the stdlib now has.** Its `_agnosai_mkdir_one` and `_agnosai_mkdir_p`
  were written because `lib/io.cyr` had no portable `xmkdir` (filed upstream in 2026-07); it now has `xmkdir` and
  `xmkdir_p`. Move to them, keeping the two behaviours its tests hold: an existing directory is success, and a
  component that exists as a file is `-ENOTDIR`. The module header's "Three places the stdlib does not reach" goes with
  them. Found at 2.5.0, which made `durable_state` the crew log's store.

## Waiting on another project

None of this has a release number, and none of it holds a release above. Each takes the next free release once its
project ships the fix. **⚠ A filing is only visible upstream once it is committed and pushed in that project's repo.**
The *Issue* column says where each stands.

| What agnosai would do | Waits on | Issue | agnosai's half, once it ships |
|---|---|---|---|
| **Port `personality`, then delete `rust-old/`** | bhava has no Cyrius port yet, and the user plans to start one soon | None needed: the user's own plan | `personality`, `with_personality`, `mood_adjusted_temperature`, the personality score and the system-prompt block. Then the deletion checklist below |
| **Verify JWT keys over 4096 bits** | sigil: `_rsa_recover_em` refuses `n_len > 512`; the Rust line took 8192 | sigil `docs/development/issues/2026-11-05-agnosai-rsa-over-4096-bits-cannot-verify.md`, ⚠ **written, not committed** | Raise `AGNOSAI_JWT_MAX_MODULUS` from 512 to 1024 bytes, and the buffers it sizes in `src/server/auth.cyr` |
| **A burst of TLS handshakes all succeeds** | sigil: of 100 threads, 64 verify and 36 are refused by its lane pool | sigil `docs/development/issues/2026-11-05-agnosai-lane-pool-refuses-the-overflow-of-a-burst.md`, ⚠ **written, not committed** | Since 2.4.1 outbound https is capped at 64 exchanges (ADR 033), which also caps throughput to an https gateway. Delete `src/tls_permit.cyr` and its six call sites. `tests/server_auth_lane_race.tcyr` pins the RSA verify's lane-freedom |
| **An HTTP exchange leaves nothing behind** | sandhi keeps 408–480 B a request on the global heap: about 40 B a response header, and 120 B for the host header | sandhi `docs/development/issues/2026-11-05-agnostic-outbound-calls-keep-bytes-on-the-global-heap.md` (agnostic's, with agnosai's addendum), ⚠ **written, not committed** | Tighten the per-exchange bounds to 0: `tests/orch_crew_arena.tcyr`, `tools_agnos`, `tools_gate`, `telemetry_otlp`, `llm_hoosh` |
| **Accept a chunked request body** | sandhi answers 501 (`sandhi_server_request_has_unsupported_te`) | sandhi `docs/development/issues/2026-11-05-agnosai-server-cannot-receive-a-chunked-request-body.md`, ⚠ **written, not committed** | Nothing: out of scope until it lands |
| **Offer a model the tools of a remote MCP server** | bote has no MCP client, **and its roadmap lists one as a non-goal**, so this needs a decision first (*Decisions for the user*) | bote `docs/development/issues/2026-11-07-agnosai-no-mcp-client-to-call-a-hosts-tools.md`, ⚠ **written 2026-11-07, not committed** | Register each remote tool with `agnosai_tool_new`: the registry and the loop need nothing else (ADR 029, rule 4) |
| **Render a crew's log with bayan's own JSON writer** | bayan escapes a string with one builder call per byte: 148 µs for 4 KiB of prose, where copying runs takes 14 µs | bayan `docs/development/issues/2026-11-12-agnosai-json-build-escapes-one-byte-a-call.md`, ⚠ **written 2026-11-12, not committed** | Drop `_agnosai_crew_log_value_a` and its string and word helpers from `src/crew_log.cyr`, and render with `bayan_json_v_build_a`; `test_writer_matches_bayan` in `tests/orch_crew_log.tcyr` goes with them |
| **A kept connection costs what a new one costs, or less** | sandhi allocates a pooled connection from the request's allocator, builds the pool's route key on the global heap for each take and put, 256 B a request, resolves the host before it asks the pool, and cannot end a blocked read with `total_ms` alone | sandhi `docs/development/issues/2026-11-08-agnosai-a-pooled-connection-lives-in-the-requests-allocator.md`, ⚠ **written 2026-11-08, not committed** | Drop the load tester's third arena, and lower `_T_SANDHI_BYTES_PER_KEPT_REQUEST` in `tests/tools_builtin_load_testing.tcyr` |

bayan's refusals of block scalars, flow mappings, anchors and tags are deliberate (its ADR 0008), so that is a scope
request to raise only if a definition file needs one, and it is not owed.

### Deleting `rust-old/` — the release after `personality`

**The Rust port is not done while bhava is missing** (*Settled decisions*), so this has no number until bhava lands.
Everything else the Rust line had is pinned above: the
certificate JWT key at 2.6.1 and the spans at 2.6.2 (`StateStore` shipped in 2.5.0). Every Rust spec stays reachable after:
`git show 2.1.0:rust-old/src/<path>`.

- [ ] `git rm -r rust-old`, then delete the ignored `rust-old/target/` (~13 GB) from disk, which `git rm` leaves.
- [ ] Update the text that describes it: CLAUDE.md (identity line, layout section, the DO NOT rule; the user's
      file), `CONTRIBUTING.md`, `docs/architecture/overview.md`, and README.md's note, tree line and footer.
- [ ] Prune `.gitignore`, still the Rust-era file: `/target`, `**/target/`, `**/*.rs.bk`, `criterion/`,
      `tarpaulin-report.*`, `.cargo/`, and a `Cargo.lock` comment naming `agnosai-server`. Keep what
      `sdk/agnosai-tool-sdk/` needs, as it stays a Rust crate (*Settled decisions*).

The ~650 comment lines citing a `rust-old/` path stay as historical pointers. Nothing breaks if they dangle.

## Decisions for the user

> ⚠ **A decision already made does not belong here.** The `personality` deferral sat here for weeks after bhava was
> settled and got asked again. When an item is decided, move it to *Settled decisions*, written out.

| Decision | Needed by | Where it stands |
|---|---|---|
| **Who owns the MCP client: bote, or agnosai and agnostic?** | Before remote MCP tools | bote's roadmap lists an MCP client as a non-goal (`docs/development/roadmap.md:206`), its own consumer audit lists it as a gap daimon and thoth want (G10), and agnosai and agnostic settled on 2026-10-07 that it is bote's. The bote filing above states all three |
| **Require hoosh 2.10.2 or newer?** | Any time | It would drop three workarounds for older gateways. `_agnosai_hoosh_detail_a`'s raw scan of an error body that is not JSON. `_AGN_CAT_REREAD_MS`'s ration on re-reading the catalog, which 2.3.3 also applies to a task with tools on a list where nothing takes them: hoosh 2.10.2 holds the catalog's body, so a GET no longer grows its heap. And `_agnosai_model_is_chat`'s twelve words, which on 2.10.2 only repeat what hoosh already did (ADR 030's addendum). ADR 030's price rule stays either way, for models hoosh has no row for. **Since 2.3.8 the floor would be 2.10.3**: a chat call sends `traceparent`, and a gateway before 2.10.3 takes its parent-id as its own span id, repeating the chat span's in one trace (ADR 023's 2.3.8 addendum) |
| **Make CI's Lockfile step fail on a drifted lock (`cyrius deps --locked`)?** | Any time | `CLAUDE.md` has CI resolve the lock rather than audit a committed one. Since cyrius 6.7.6 a plain local `cyrius deps` writes the same 8-pin lock CI does, so `--locked` would now catch a tag that moved upstream. Adopting it changes the policy that file states |
| **Should the server run crews for tenants?** | Before the server takes requests for more than one party | 2.4.1 gave the orchestrator tenants (ADR 033), and agnostic, which has its own, can name one per crew. The server has none: who a request runs for would have to come from its JWT, which says nothing of a tenant today, and a tenant registry would need configuring. Until then, each server's crews share its per-crew budget and its turns |
| **Note the 2.1.5 exception in the CHANGELOG, or change its policy line?** | Any time | `CHANGELOG.md:6` claims Semantic Versioning, but 2.1.5 shipped a `### Breaking` section under a patch number (*Settled decisions*) and neither says so |

## Settled decisions

Decided, and **not to be re-raised**. Each entry gives the date, the decision, the reason, and where the full reasoning
lives.

- **Scope: the whole Rust line was owed** (2026-08-07). The user overturned a narrowing of v2.0 to the default cargo
  build. That narrowing was written by earlier sessions and repeated across four handoffs, but never decided by the
  user; only bhava was a user decree. **A cargo feature gate is not a scope boundary.** Never add an exclusion row: if
  something looks impossible, prove it — name the missing primitive, grep `lib/` for it, and file the upstream ask.
  [`cyrius-port-plan.md`](cyrius-port-plan.md) (lines 12, 320).
- **`rate_limit` is mounted by default** (2026-08-13). 100 req/s per client key, burst 200; `AGNOSAI_RATE_LIMIT=0`
  restores the oracle's exact wire. The user's reason, verbatim: *"agnosai ships safe-by-default for operators who
  don't read docs; cause most people and even agents TLDR."* [ADR 021](../adr/021-rate-limit-mounted-by-default.md).
- **`personality` is deferred, never dropped** (recorded 2026-08-07, re-affirmed 2026-08-13). bhava is coming and has
  no Cyrius port, so `personality` stays unported and the wire keeps emitting `null` until bhava lands in Cyrius. It is
  the one carve-out the user ever set; listing it as an open question once caused it to be asked again. *What agnosai
  is*, *Waiting on another project*, [`cyrius-port-plan.md`](cyrius-port-plan.md) (:297, :380).
- **`builtin_presets()` stays cold** (2026-08-13). No memoization; the oracle's shape stands
  (`rust-old/src/definitions/loader.rs:122` has no cache either) and no ADR is owed. The user's reason, verbatim:
  *"presets can be cold; as agent might get different instruction sets than presets and seems a waste of speed
  improvements."* The route is not hot, and a process-lifetime cache of parsed structs is shared mutable state this tier
  avoids. The cost accepted: ~0.9 ms CPU and ~1 MB of arena churn per `GET /api/v1/presets`, reclaimed per request
  since 2026-08-11. CHANGELOG [2.0.0].
- **Benchmarks run rounds × batch** (2026-08-13). Every `.bcyr` runs `_BR = 5` rounds of batch/5, so min/max are real
  samples (one sample per row had made min == max == avg, hiding ~5% run-to-run spread); total work is unchanged, and
  the whole CSV was re-baselined that day. `tool_registry_register` stays single-batch (it would measure fill level).
  **Standing rule for bench edits:** a row that consumes state (drains, dedups, subscribes) must index a distinct slice
  per round (`_r * (n / _BR)`) or be exempted. Find such rows by diffing every average against the pre-change run: of
  the six found, an assertion caught one. CHANGELOG [2.0.0].
- **Dispatch bench rows include body serialization** (2026-08-13). `benches/server.bcyr`'s `route_*` rows measure the
  full route: `_b_render_body` / `_b_render_body_a` reproduce `_agnosai_serve_send`'s branch set, matching the oracle's
  tower-service bench (`rust-old/benches/server.rs:43-62`). Stopping at the response object hid serialization
  regressions. CHANGELOG [2.0.0].
- **2.1.5 shipped as a patch despite a `### Breaking` section** (2026-10-04). The user tagged 2.1.5, then 2.1.6: the
  span-vocabulary and genai API break shipped under a patch number, not 2.2.0 or 3.0.0. No known consumer was affected
  (agnostic, daimon and thoth were checked and call none of the removed names). Open follow-up: *Decisions for the
  user*. CHANGELOG [2.1.5].
- **A selection explanation keeps no per-run record** (2026-10-03). `agnosai_explain_selection_a` is a library call with
  no wire field and no ADR. The crew run keeps no breakdown, which would cost agents × tasks entries on the global bump
  every run; consumers recompute from the spec. That is exact while selection is a pure function of roster and task,
  and hierarchical crews (2.4.3) kept it one: a run ranks the workers `agnosai_hierarchical_workers_a` returns
  (ADR 035). CHANGELOG [2.1.5].
- **The tool SDK and `examples/wasm-tools/` stay Rust** (2026-08-11). Not a deferral. `sdk/agnosai-tool-sdk/` is a crate
  third-party tool authors compile to `wasm32-wasip1`; rewriting it in Cyrius would break every tool author on the
  published protocol. agnosai owes conformance only — the `{"parameters": …}` stdin and `{"result", "success",
  "error"}` stdout contract — which `src/tools/wasm_tool.cyr` implements and `tests/tools_wasm.tcyr` pins.
  [`adding-wasm-tools.md`](../guides/adding-wasm-tools.md).
- **No history rewrite for `probe_key_tmp.pem`** (2026-08-10). The deleted test key stays reachable in `bb76e67`. It
  was generated locally for a probe and never a credential; rewriting `main` costs more than it saves. `.gitignore`
  carries `*.pem` / `*.key`. `state.md`, *Known issues*, item 0.
- **Trace context is threaded explicitly, not thread-local** (2026-10-03). agnosai passes span contexts down its call
  path and never reads sakshi's process-global trace id: `thread_create` gives each worker a zeroed TLS block,
  agnostic's crews run on a detached thread, and `thread_local_get` faults on a main thread with no block.
  [ADR 023](../adr/023-genai-semconv-spans-and-w3c-trace-context.md).
- **Per-task audit only in sequential mode is parity** (2026-08-06). The oracle's only `audit_record` call site is in
  its `run_sequential` (`rust-old/src/orchestrator/crew_runner.rs:294`), so parallel and DAG modes never audit per
  task, and no per-worker audit scratch is owed. CHANGELOG [2.0.0].
- **Literal hoisting scope** (2026-08-07). Hoist a literal to a process-lifetime global only when a loop body or an
  unconditional envelope reaches it; an arena allocation is not free (~11 ns plus a 16 B header), and hoisting took
  `/dashboard/crews` 6,881 → 5,217 ns. Error messages stay inline: built at most once, for a request that already
  failed. The `str_from_a` sites left in `src/` (338 at the count) are deliberate, as are three in-loop sites on return
  paths (`sandbox/oci.cyr:250-251`, `crew_runner.cyr:990`) that a naive scan keeps finding. The `_a` forms keep their
  signature and return the global. [`cyrius-port-plan.md`](cyrius-port-plan.md) (:372), CHANGELOG [2.0.0].
- **The JWT `alg` check runs after signature verification** (2026-07-31). By design: parsing attacker-controlled JSON
  before authenticating measured ~53x heap amplification, so a rejected token paying the modexp is the accepted cost.
  CHANGELOG [2.0.0], *Security*.
- **cyrius 6.6.6's by-value struct deep copy does not reach `: Str` parameters** (2026-09-26). 42 functions take a
  `: Str` and 141 sites store one into a heap object or a `vec` that outlives the call; they would dangle if the copy
  applied. It does not: `Str`, `Result`, `Option` and `Tagged` are excluded by name (`_local_is_sptr_param`, 6.6.6
  bite 16b) and stay pointer rebinds, and agnosai declares no struct-typed parameters. No change needed. Full
  read-through: `git show 2.1.6:docs/development/roadmap.md`, *Moving the cyrius pin to 6.6.6*.
- **Not adopted: free-form swarms or parallel writers as an engine mode, and screen-scraping coordination**
  (2026-10-03). The evidence runs against the first: MAST (NeurIPS 2025), Cognition 2026, and Tran & Kiela 2026 (single
  agent ≥ multi-agent at an equal token budget). agnosai has structured events, which is why it needs no herdr-style
  scraping.
- **Federation adopts any declaration whose term is not stale** (2026-11-05).
  `agnosai_federation_declare_coordinator` keeps its `term < current` guard. Adopting a higher term is the
  leader-election rule. Refusing a conflicting declaration at the same term would buy nothing, because with no vote a
  peer could declare at term+1 instead. Federation trusts its peers; `threat-model.md` surface 4 says so, and the
  reasoning is in `src/fleet/federation.cyr`'s header.
- **`rust-old/` goes once every Rust porting item is done, and bhava is one of them** (2026-11-05 and 2026-11-06, the
  user). "If all the rust porting items are done then deleting at 2.3.0 is fine", then, when asked whether the
  carve-out was exempt: "rust port is not done if bhava is still missing", with the bhava port to start soon. So the
  deletion waits on `personality` and has no number. **RSA keys over 4096 bits do not hold it** (the same day): that
  gap is sigil's, and the user's ask was that it be filed (*Waiting on another project*).
- **`assert` stays in `[deps] stdlib`; only `bench` moved to `[test] stdlib`** (2026-11-05, 2.2.3 audit).
  `src/learning/optimizer.cyr` calls the stdlib's `panic()`, which lives in `lib/assert.cyr` and is documented there as
  the runtime guard for library code. So `assert` is a real production leaf, and a consumer's sidecar naming it is
  correct, not test-leaf leakage.
- **The MCP client is bote's** (2026-10-07). A crew's agents reach external MCP servers through bote, by way of
  agnosai's tool registry; neither agnosai nor agnostic carries its own MCP client. bote is the ecosystem's MCP layer,
  built so that apps stop implementing MCP each for themselves. Recorded in agnostic's `roadmap.md` (*Settled
  decisions*) the same day. ⚠ **bote's roadmap disagrees** (*Decisions for the user*).
- **The roadmap is by release, in plain words, and blocked work never goes first** (2026-11-07, the user). No item
  codes: a release says what it does. A release holds only work that can ship now, and work waiting on another project
  goes under *Waiting on another project* with its filed issue, until the fix ships.

## Out of scope

- The bhava carve-out (see *What agnosai is*).
- Inbound chunked request bodies: sandhi answers 501, honest but not support (*Waiting on another project*).
- Any comparison of Cyrius benchmark numbers against the frozen Rust CSV.
- `fleet/discovery` beyond the oracle's own 174-line stub, which `rust-old` calls a stub itself. `lib/net.cyr` has no
  SRV resolver. Finishing it is a scope decision nobody has made, not owed work.
- Python bindings: demand-gated, a separate crate (`cdylib`, maturin).

### A note, not a filing: an OpenTelemetry library

There is no Cyrius OTel library, and hoosh (`src/lib/otlp.cyr`) and agnosai (`src/telemetry/otlp.cyr`) hand-rolled the
same subset, because ADR 003 forbids depending on hoosh as a library. By the port's rule (`CLAUDE.md`, *Refactoring*)
the third instance is when the abstraction gets extracted. It would be a sibling repo with its own `dist/` bundle, not a
stdlib module, and naming is the user's call. It would own OTLP/protobuf (itself probably a separate codec repo), the
span model, context propagation with an explicit-context API (ADR 023 records why thread-local cannot serve agnosai), a
batching exporter, and metrics and logs. `src/telemetry/otlp.cyr` would collapse to an adapter, as `fleet/relay` did
onto majra.

## Item codes in older text

Releases before 2.3.2 named items by code, and older comments, ADRs and `state.md` still cite them. This only maps a
code to where its work is now. Nothing new gets one.

⚠ **Three release numbers moved at 2.3.3.** Confinement left *Waiting on another project* when kavach 3.14.7 shipped,
and took 2.3.4 as the threat model's largest gap. So text written for 2.3.2, its CHANGELOG entry included, means
2.3.5 where it says WASM tools at 2.3.4, 2.3.6 where it says network tools at 2.3.5, and 2.3.7 where it says JSON forms
and log fields at 2.3.6.

| Code | Now |
|---|---|
| B5 | `personality` (*Waiting on another project*) |
| B6 | shipped in 2.4.1, budgets and caps (ADR 033) |
| B7 | 2.6.2, the tracing export (its item 8 and 9 shipped in 2.3.0) |
| B8 | 2.6.1, server edges; RSA over 4096 bits is *Waiting on another project* |
| B9, B32 | shipped in 2.3.5, WASM tools (ADR 019's 2.3.5 addendum) |
| B10, B11 | 2.3.7, JSON forms and log fields |
| B12, B14 | 2.7.0, tests and benchmarks |
| B13, B36 | 2.3.6, network tools |
| B15 | `StateStore` shipped in 2.5.0 (ADR 036); the rest at 2.7.2, housekeeping |
| B16 | chunked request bodies (*Waiting on another project*) |
| B18, B20, B27 | 2.7.1, benchmark levels |
| B25 | 2.6.1, server edges |
| B34 | 2.7.2, housekeeping |
| B33 | 2.3.1, and its hoosh half in 2.3.3 |
| B35, B40 | shipped: confinement in 2.3.4 (ADR 031), and B40's WASM half in 2.3.3 with kavach 3.14.7 |
| B37 | shipped in 2.4.0, the crew lifecycle (ADR 032) |
| B38 | 2.5.1, the server's own crews in arenas |
| B39 | an HTTP exchange leaves nothing behind (*Waiting on another project*) |
| B41 | shipped in 2.3.2 |
| C | the rows of *Waiting on another project* |
| D1–D9, F7 | *Settled decisions* and *Decisions for the user*, by their titles |
| F2 | shipped in 2.4.3, hierarchical crews (ADR 035) |
| F3 | shipped in 2.4.0, the crew lifecycle (ADR 032); its resync snapshot in 2.5.0 (ADR 036) |
| F4 | shipped in 2.5.0, durable execution (ADR 036) |
| F5 | shipped in 2.4.1, budgets and caps (ADR 033); the delegated worker's level in 2.4.3 (ADR 035) |
| F8 | shipped in 2.6.0: MCP 2026-07-28 (ADR 039) and A2A 1.0 (ADR 040) |
| F9 | shipped in 2.4.2, the hand-off of a dependency's answer (ADR 034) |
