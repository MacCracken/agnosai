# 023 — GenAI spans follow the semconv agent operations, and join the caller's W3C trace

## Status: Accepted (2026-10-04), shipped in 2.1.5. Proposed 2026-10-03. Amends [ADR 017](017-genai-span-call-sites.md).

Note (2026-10-04, 2.1.6): the export this ADR turned on was unbounded. Each
batch POST used the bare `sandhi_http_post` (the no-free global bump, ~258 KiB of
RSS per batch, and no timeouts), two flushes could share one batch's memory, and
the generic `OTEL_EXPORTER_OTLP_ENDPOINT` was read with the per-signal rule (a
base with a path was posted to as given). 2.1.6 gives the exporter its own arena,
options with timeouts, and a flush lock; reads the generic endpoint as a base URL
with `v1/traces` appended; and adds the per-signal
`OTEL_EXPORTER_OTLP_TRACES_ENDPOINT`, used as-is. The span vocabulary and the
trace-context decisions below are unchanged.

## Context

ADR 017 made `src/` call the GenAI span helpers that
`rust-old/src/telemetry/genai.rs` declares and never calls. It kept the oracle's
span vocabulary and minted a fresh identity for every span. Reading the code
against the current OTel GenAI conventions shows three problems. **Every line
citation below is as of tag 2.1.4.**

1. **The names describe the wrong operations.**
   - The span named `gen_ai.invoke_agent` (`genai.cyr:78`) was the hoosh
     chat-completion call (`hoosh.cyr:682`), exported as CLIENT
     (`otlp.cyr:300-304`). It is an inference, not an agent invocation.
   - There was no per-task span at all: `agnosai_execute_task` recorded nothing.
   - The crew span `agnosai.crew.run` carried no `gen_ai.operation.name`
     (`genai.cyr:162-173`), so a backend grouping by operation could not
     classify crew runs. The port reproduced the oracle's omission on purpose.
   - `gen_ai.system` is deprecated (base semconv v1.44.0) in favour of
     `gen_ai.provider.name`.
   - `agnosai.tool.name` duplicates `gen_ai.tool.name`, which is now in the
     spec; the comment saying it is not (`genai.cyr:58-59`) was stale.
2. **No two spans were ever in the same trace.** `agnosai_telemetry_record_span`
   (`mod.cyr:636-643`) built the trace id and the span id from one
   `agnosai_uuid_v4`, so a span's id was the high half of its own trace id.
   Neither the export context nor the encoder had a parent field. Every span
   was a one-span trace.
3. **A caller's trace could not be joined.** agnostic mints or honours a W3C
   `traceparent` per request (`agnostic/src/trace.cyr`), and so can any client
   of `POST /api/v1/crews`. agnosai offered no way to take one in. And agnostic
   never called `agnosai_telemetry_init_tracing` — it cannot, because that call
   replaces sakshi's emit hook, and agnostic installs its own — so inside
   agnostic's process no agnosai span was ever exported.

**The oracle never did any of this.** `rust-old/` has no `traceparent`
extraction and no propagator (grep of `rust-old/src` and `Cargo.toml`). Its only
correlation was `tracing`'s automatic in-process nesting of `#[instrument]`
spans (`crew_runner.rs:129`, `:242`, `:314`, `:439`, `:618`), which the port does
not have (roadmap B7). This is a divergence past parity, so per CLAUDE.md it is
recorded here.

**The convention is still "Development", so the version is pinned.**

- `open-telemetry/semantic-conventions-genai` at commit
  `e07f4ebacb08f56db8c4c882d117720333fbca04` (2026-10-02). The repo has no tags
  or releases.
- Its `model/manifest.yaml` declares schema
  `https://opentelemetry.io/schemas/gen-ai-dev/1.42.0-dev` on base semconv
  v1.44.0, stability `development`.
- Read at that commit: `docs/gen-ai/gen-ai-spans.md` (Inference, Execute tool),
  `docs/gen-ai/gen-ai-agent-spans.md` (Invoke agent internal, Invoke workflow)
  and `docs/registry/attributes/gen-ai.md`.

**Thread-local storage was considered and cannot carry the context.**

- The cyrius stdlib has `thread_local_*`, but `thread_create` gives each worker
  a zeroed TLS block (`lib/thread.cyr`), and parallel and DAG tasks run on
  `thread_create` workers.
- agnostic submits crews through the orchestrator's detached thread
  (`orchestrator.cyr`, `agnosai_orchestrator_submit_crew`).
- `thread_local_get` faults on a main thread that never called
  `thread_local_init` (`audit.cyr`, `arena_pool.cyr`), and every suite drives
  the runner from main.

A context set on the caller's thread is invisible to the crew in every process
mode, and reading one at a span site could crash a test binary.

## Decision

**Emit the four semconv operations, link every span to its parent, and accept
an inbound W3C `traceparent`.**

| Operation | Site | Span name | Kind | Attributes |
|---|---|---|---|---|
| `invoke_workflow` | `crew_runner` `agnosai_crew_runner_run` | `invoke_workflow {crew name}` | INTERNAL | `gen_ai.operation.name`, `gen_ai.workflow.name`, `error.type`, `agnosai.crew.id`, `agnosai.crew.task_count` |
| `invoke_agent` | `crew_runner` `agnosai_execute_task_in_crew` | `invoke_agent {agent name}` | INTERNAL (in-process agent) | `gen_ai.operation.name`, `gen_ai.request.model`, `gen_ai.agent.name`, `error.type`, `agnosai.task.id`, `agnosai.agent.key` |
| `chat` | `llm/hoosh` `agnosai_hoosh_chat_in` | `chat {gen_ai.request.model}` | CLIENT | `gen_ai.operation.name`, `gen_ai.provider.name` = `hoosh`, `gen_ai.request.model`, `gen_ai.response.model`, `error.type`, `gen_ai.usage.input_tokens` / `output_tokens` |
| `execute_tool` | `tools/native` `agnosai_tool_execute_in` | `execute_tool {gen_ai.tool.name}` | INTERNAL | `gen_ai.operation.name`, `gen_ai.tool.name`, `error.type` |

**Span names.** A name falls back to the bare operation when its subject is
absent or empty. The encoder writes the operation and the subject as two escaped
parts inside one pair of quotes, so no name is built per span;
`agnosai_genai_span_full_name_a` builds the same string for callers, and a test
holds the two together.

**The agent's key is `agnosai.agent.key`, not `gen_ai.agent.id`.** At the
pinned commit the registry defines `gen_ai.agent.id` as "the unique and stable
identifier of the GenAI **hosted** agent resource" (a Bedrock agent ARN),
advises against in-memory ids, and the in-process `invoke_agent` span does not
list it. An agnosai agent is in-process and its key is a definition key, so it
goes out under agnosai's own prefix. It is the key `metadata.agent` carries on a
task result (ADR 022), which lets a span be joined to its result.
`AGNOSAI_GENAI_AGENT_ID` stays declared and unemitted.

**`gen_ai.provider.name` is `hoosh`, not a guess at the upstream.** The registry
says the value is the instrumentation's best knowledge and "may differ from the
actual upstream provider" behind a proxy. hoosh is the proxy agnosai calls, and
hoosh's own server span names the provider. Guessing from the model name
(`_agnosai_crew_infer_provider`) would put every unknown model under Ollama.

**`error.type` vocabulary.** Low-cardinality, as the spec requires, and listed
here because the spec says instrumentations SHOULD document what they report:

- workflow: `timeout`, `cancelled`, `task_failed`, `invalid_dag` (a cycle or a
  dangling dependency);
- agent: `task_failed`;
- chat: `no_response`, `invalid_response`, the decimal HTTP status (`503`),
  and for a transport failure sandhi's error kind **lowercased** (`parse`,
  `connect`, `tls`, `timeout`, `remote`, `protocol`, `auth`, `discovery`,
  `internal`, `unknown`). `sandhi_err_kind_name` spells them in capitals; sent
  as-is, a chat's `TIMEOUT` and a crew's `timeout` would be two classes to a
  backend grouping by `error.type`;
- tool: `no_output`, `tool_error`.

The workflow span reads the timeout from the runner itself, so it agrees with
the orchestrator's FAILED however `status` was reached (ADR 022).

**Span context is threaded explicitly, never through thread-local storage.**

- A 32-byte span context {trace hi, trace lo, span id, flags} is passed down
  the call path: the runner (`AGN_CR_TRACE`), then the wave job
  (`AGN_CJ_TRACE`), then `agnosai_execute_task_in_crew`'s trailing `parent_sc`,
  then the retry context (`AGN_ICTX_TRACE`), then the hoosh client's chat
  pointer, whose contract gains a fourth argument, then `agnosai_hoosh_chat_in`.
- Each span takes a fresh random 8-byte span id under its parent's trace id; a
  root takes a random 16-byte trace id too. Both are forced non-zero.
- The OTLP encoder emits `parentSpanId` for every non-root span and omits the
  key for a root.

**Inbound context.**

- **In-process.** `agnosai_crew_with_trace_parent(spec, tp)` stores a COPY of a
  55-byte traceparent on the crew spec. The copy is required: agnostic's value
  lives in its per-request arena, and an async crew runs after that request has
  been answered. The field is not serialised — trace context is per-execution
  transport, not part of the crew's definition — so the wire and any persisted
  spec are unchanged.
- **Parsing is telemetry's, at run time, and strict.** W3C Trace Context
  Level 1 version `00`: exactly 55 bytes, version `00`, `-` at offsets 2, 35
  and 52, lowercase hex everywhere else, non-zero trace and parent ids.
  Anything else starts a new root trace and is never an error.
- **HTTP.** `agnosai_serve_handler` reads the `traceparent` header (sandhi's
  lookup ignores case) and passes it through `agnosai_route_dispatch_tp_a` to
  the three routes that start traced work: `POST /api/v1/crews`,
  `POST /api/v1/a2a/receive` and `/mcp` `tools/call`. Every response is
  byte-identical with or without it.

**Sampling is ParentBased, the OTel SDK default.** An inbound context whose
sampled flag is 0 suppresses the crew's spans; children inherit the flags.

**The inbound context is trusted as-is — a recorded trust-boundary choice.**
The three routes sit behind auth, so any authenticated caller (anyone, with
auth off, which is the default) can:

- choose the trace id agnosai's spans join, and so plant spans in any trace on
  the collector; and
- switch off span export for the work it starts (its crew, its tool call) by
  sending sampled flag `0`.

W3C Trace Context leaves it to the receiving service whether a trace crossing a
trust boundary is honoured or restarted. agnosai honours it, because joining
the caller's trace is the feature, and only OTLP spans are lost under `-00`:
sakshi logs, the audit chain, `/metrics` and the task results are not. The
threat model records it as surface 7 and an accepted risk. An operator switch
to restart an untrusted inbound context (new root, or record regardless of the
sampled bit) is a roadmap follow-up, not part of this decision.

**In-process consumers can export without agnosai's logging.**
`agnosai_telemetry_init_export(endpoint, service_name)` starts the exporter and
leaves sakshi's level and emit hook alone. `agnosai_telemetry_init_tracing`
calls it. A second call starts no second exporter thread: it hands back a guard
over the one already running.

**The off path allocates nothing.** Under ADR 017 every tool call, inference and
crew run built its span record and read the clock twice for a recorder that then
discarded it. Each site now builds its span, draws its random bytes and reads
the clock only while the exporter is on; a suite compares each instrumented
entry point's allocations with the bare work it wraps. With telemetry on, the
export context is built on the recorder's stack, since the ring encodes it and
keeps nothing.

**No dual emission of the old names.** The exporter has been live only since
2.x, and no consumer keys on the old names (agnostic never exported them at
all).

**Out of scope, recorded:**

- **Outbound `traceparent` to hoosh.** hoosh 2.7.1 honours an inbound
  traceparent but exports its own span with the inbound parent-id as its spanId
  and no parentSpanId (`hoosh/src/lib/otlp.cyr:72-77`), so propagating now
  would duplicate the chat span's id inside one trace. That is a hoosh fix
  first (roadmap, *Upstream*), then agnosai adds the header.
- **B7.** The remaining `#[instrument]` span coverage, trace ids in JSON log
  lines, aggregate usage on `invoke_agent`, request parameters, `finish_reasons`
  (a `string[]` needs an OTLP `arrayValue`), `Span.flags` is_remote, and
  `tracestate`.
- **F1's tool loop** must call `agnosai_tool_execute_in` with the task's span
  context, and should pass the agent name; today `/mcp` `tools/call` is the only
  `execute_tool` producer.
- **The opt-in `delegate` tool** submits its crew with no trace parent, since the
  tool vtable carries no span context, so a delegated crew starts a new root
  trace. The pinned semconv also says an `invoke_workflow` span SHOULD NOT be
  reported when a tool spins up a workflow to delegate to a sub-agent. Both wait
  on a tool-call span context (roadmap B7, item 9).
- **An operator switch** to restart, rather than honour, an untrusted inbound
  context (roadmap B7, item 10).

## Consequences

- **Positive.**
  - One trace now covers caller → `invoke_workflow` → `invoke_agent` → `chat`
    in every process mode, including parallel and DAG workers, and joins
    agnostic's or any HTTP client's trace.
  - Backends that understand GenAI semconv (Datadog, Grafana, Arize, Langfuse)
    classify agnosai's spans without custom mapping.
  - The off path is cheaper than under 017: no span record, no random bytes and
    no clock reads. `tool_execute_echo` measured 3.67 → 0.79 µs.
  - An in-process consumer can export agnosai's spans at all.
- **Negative.**
  - **The telemetry wire changes**: span names, three attribute keys, and the
    old `gen_ai.invoke_agent` (CLIENT) span becomes `chat`. Dashboards built on
    2.1.4 must be updated. The CHANGELOG carries the old → new table.
  - Span names now include model, agent, tool and crew names. The spec requires
    `gen_ai.workflow.name` to be low-cardinality and crew names are
    caller-supplied, so a caller that names every crew uniquely inflates
    span-name cardinality. Documented, not truncated.
  - Until agnostic exports its own SERVER span, the workflow span's parent id
    points at a span the collector never receives. Backends show a missing
    parent; the trace id still groups correctly.
  - The hoosh client's chat-pointer contract gained a fourth argument. Any stub
    written for ADR 022's three-argument form must take it — and any caller
    must pass it: a three-argument `callptr` hands the callee a stale register
    as its span context, which the live call dereferences once the exporter is
    on.
  - **A caller controls the trace context** (see *The inbound context is
    trusted as-is*): it picks the trace id and can turn its crews' spans off.
    Recorded in the threat model; the operator switch is on the roadmap.
- **Neutral.**
  - Each traced call site gained an `_in` form taking a parent span context
    (`agnosai_hoosh_chat_in`, `agnosai_tool_execute_in`,
    `agnosai_telemetry_record_span_in`), and the three traced routes a `_tp_a`
    form. The existing public signatures delegate with 0, so no caller changes.
    The one exception is `agnosai_execute_task_in_crew`, new in the same
    release (2.1.5, ADR 022), which gained a trailing `parent_sc` rather
    than a third task entry point.
  - The semconv pin must be re-checked when the convention leaves Development.

## Alternatives considered

- **Thread-local current span.** Rejected: TLS does not cross `thread_create`
  or the detached submit thread, and reading it faults on an uninitialised main
  thread (see Context).
- **Keep the oracle names and only add trace linkage.** Rejected: the names
  misdescribe the operations, which is the defect the semconv fixes.
- **`gen_ai.agent.id` for the agent key**, as first planned. Rejected after
  reading the pinned registry: the attribute is for hosted agents' provider ids.
- **Put the traceparent in crew `metadata`.** Rejected: metadata is serialised
  by `agnosai_crew_to_value_a`, so trace context would leak into the wire and
  into persisted specs.
- **A setter on the orchestrator.** Rejected: the spec is the one object that
  already crosses from the caller to the detached runner thread, so a spec
  field needs no per-crew side table.
- **A new `agnosai_execute_task_in(task, agent, client, event_tx, parent_sc)`**,
  as first planned. Rejected because ADR 022 had just added
  `agnosai_execute_task_in_crew` for the crew-scoped call; the parent span
  context is crew-scoped too, so it joined that signature rather than adding a
  third entry point.
- **Stash the span context on the hoosh client** instead of widening the chat
  pointer. Rejected: one client serves every worker of a parallel crew at once.
- **Infer `gen_ai.provider.name` from the model prefix.** Rejected: a guess with
  an Ollama fallback, where hoosh knows the real provider.
- **Dual-emit old and new names** (`OTEL_SEMCONV_STABILITY_OPT_IN` style).
  Rejected: there is no installed base to protect.
