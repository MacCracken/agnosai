# API Reference

AgnosAI exposes a REST API from the `agnosai` binary, served by
[sandhi](https://github.com/MacCracken/sandhi) over a pooled worker model.

> Both names in the previous sentence changed with the Cyrius port: the binary
> was `agnosai-server` and the server was axum. The routes, status codes and
> bodies below did not — they are held to the Rust tree as the parity oracle.

## Endpoints

⚠ This table listed **6** of the **19** routes the router served (20 since 2.5.0's snapshot). The rest were
undocumented, and `/api/v1/crews/{id}` was marked "(placeholder)" long after it
started returning real state. Until 2.1.7 it still named 18: it listed a `GET`
on `/api/v1/tools/{name}`, which the router has never served (it is `DELETE`, as
in the oracle), and left out `GET /api/v1/approvals`.

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/health` | no | Liveness probe |
| GET | `/ready` | no | Readiness probe (includes version) |
| GET | `/metrics` | no | Prometheus exposition |
| POST | `/api/v1/crews` | yes | Create and execute a crew |
| GET | `/api/v1/crews/{id}` | yes | The crew's state |
| POST | `/api/v1/crews/{id}/cancel` | yes | Request cancellation |
| GET | `/api/v1/crews/{id}/stream` | yes | SSE event stream for one crew |
| GET | `/api/v1/crews/{id}/snapshot` | yes | Each task's status and the crew's `seq`, to resync a stream from (2.5.0) |
| GET | `/api/v1/agents/definitions` | yes | List agent definitions |
| POST | `/api/v1/agents/definitions` | yes | Register an agent definition |
| GET | `/api/v1/tools` | yes | List registered tools |
| DELETE | `/api/v1/tools/{name}` | yes | Unregister a tool |
| GET | `/api/v1/presets` | yes | List presets |
| GET | `/api/v1/dashboard/crews` | yes | Crew history |
| GET | `/api/v1/dashboard/agents` | yes | Per-agent performance |
| GET | `/api/v1/approvals` | yes | Pending human-in-the-loop decisions |
| POST | `/api/v1/approvals` | yes | Submit a human-in-the-loop decision |
| POST | `/api/v1/a2a/receive` | yes | Accept a delegated task |
| POST | `/api/v1/a2a/status` | yes | A2A status |
| POST | `/mcp` | yes | Model Context Protocol JSON-RPC, 2026-07-28 and 2025-03-26 (2.6.0) |
| GET | `/.well-known/agent-card.json` | no | The A2A 1.0 Agent Card (2.6.0) |
| POST | `/a2a` | yes | A2A 1.0, the JSON-RPC binding (2.6.0) |

The three probes and the Agent Card are the only unauthenticated routes, and the allow-list is
written that way deliberately — a route added without thought defaults to
**protected**. The card is public because discovery cannot carry a token: it says how to
authenticate to the endpoint it names.

A path in this table requested with another method is **405**, with an `Allow`
header naming the methods it takes (`Allow: GET, POST` for the definitions
route; since 2.1.7). A path not in it is 404.

## Rate limiting

⚠ **Mounted by DEFAULT** at **100 req/s per client key, burst 200**
([ADR 021](../adr/021-rate-limit-mounted-by-default.md)) — a deliberate
divergence from the Rust tree, which ported the middleware and never installed
it. Over the limit a route answers **429**.

| variable | effect |
|---|---|
| `AGNOSAI_RATE_LIMIT` | requests/second per key. **`0` disables it entirely** |
| `AGNOSAI_RATE_LIMIT_BURST` | burst allowance (default 200) |

- The key is `X-Forwarded-For`'s first entry, else `X-Real-IP`, else the shared
  literal `"unknown"`.
- ⚠ Those headers are **client-settable**. Behind a proxy that OVERWRITES them
  the key is trustworthy; behind one that APPENDS (nginx's documented
  `$proxy_add_x_forwarded_for`) a client picks its own key — and can therefore
  drain a *named victim's* bucket, since the limiter runs before auth.
- `/health`, `/ready` and `/metrics` are **exempt**, so a flood cannot push a
  liveness probe into 429 and get the process killed.

## Model selection

An agent with no `llm_model` gets a model from hoosh's `GET /v1/models/catalog`, read once per process and again when a
selection fails ([ADR 030](../adr/030-a-model-comes-from-the-gateways-catalog.md)). `complexity` picks the tier: `low`
and `medium` are Capable, `high` is Premium.

| variable | effect |
|---|---|
| `AGNOSAI_MODEL_FAST` | the id the Fast tier means, sent exactly as written and checked against nothing |
| `AGNOSAI_MODEL_CAPABLE` | the same for Capable. With one set, that tier needs no catalog |
| `AGNOSAI_MODEL_PREMIUM` | the same for Premium |

An empty variable is none. Fast is reachable from the library's router and not from a crew today.

- ⚠ **With no catalog a crew that names no model fails**, before any request, and the task's `output` reads
  `LLM error: no model catalog: <why>; name a model on the agent, or set AGNOSAI_MODEL_CAPABLE`. A gateway with no
  `/v1/models/catalog` is read through `/v1/models`, where every tier is the first chat model listed.
- A selection the gateway refuses reads the catalog again, at most once a minute, and selects once more, unless a tool has
  already run.
- **On hoosh 2.10.2 or newer, hoosh's `tier` decides**: Capable is the first `standard` model listed, and Premium the
  first `premium` one, then `reasoning`. A tier with nothing in it takes the nearest tier below, then the nearest above.
  On hoosh's table today Premium is `claude-fable-5-1`, at $10/$50 per million tokens. Set `AGNOSAI_MODEL_PREMIUM` to
  hold a crew's `high` agents to less. A model hoosh's `kind` calls anything but `chat` is never chosen.
- **An agent with tools is never given a model hoosh says takes none.** On an OpenAI route, a `complexity: high` agent
  with tools gets `gpt-5.6-sol`, and one without gets `gpt-6-astra`, which takes no tools on Chat Completions. Where
  nothing listed takes tools, the task fails before any request with
  `LLM error: no model catalog: hoosh lists no chat model that takes tools; …`, and the list is read again at most once a
  minute. The agent's own `llm_model` and the `AGNOSAI_MODEL_*` overrides are sent as written.
- ⚠ **A gateway older than hoosh 2.10.2 writes none of this.** There Premium is the dearest model that declares the most
  reasoning, selection cannot see tools, and only twelve words in a model's id keep an embedder off a tier. See ADR 030
  for what that gets wrong.

## Tool sandboxes

Process and Python tools run under kavach's confinement
([ADR 031](../adr/031-confine-the-process-and-python-tiers-in-agnosais-spawner.md)): NO_NEW_PRIVS, kavach's seccomp
filter, and landlock rules giving read and execute on `/usr`, `/bin`, `/sbin`, `/lib*`, `/etc` and `/proc/self`, and
read and write on `/tmp`, `/dev/null`, `/dev/shm` and the call's work directory.

| variable | effect |
|---|---|
| `AGNOSAI_SANDBOX_READ_PATHS` | `:`-separated trees every confined tool may also read and execute |
| `AGNOSAI_SANDBOX_WRITE_PATHS` | `:`-separated trees every confined tool may also read and write |

Read once, at start-up. A path the host does not have is skipped.

- ⚠ **A tool reading outside the rules gets EACCES**, which includes `$HOME`, `~/.local`'s site-packages, `/var`,
  `/opt` and another process's `/proc`.
- ⚠ **A process or Python tool has no network unless its config allows it** (2.3.8): a TCP connect or bind, and any
  socket but a unix one, reads EACCES. `agnosai_process_config_set_network(c, 1)` and
  `agnosai_python_sandbox_with_network(s, 1)` allow it, and the manager's process tier follows the tool's policy. A
  kernel before landlock ABI 4 (Linux 6.7) leaves the tool the network, and the server warns once. WASM tools have none,
  and an OCI tool has none unless its policy allows it.
- ⚠ **A server running as root without `CAP_SETPCAP` refuses every process and Python tool**, with `failed to confine
  <program>: cannot bound capabilities`, since 2.3.6. With it, or as any other user, a tool holds no capabilities.
- ⚠ **A host where kavach cannot build its seccomp filter refuses every process and Python tool**, with
  `failed to confine: …`. A kernel with no landlock runs them with no filesystem rules, and the server warns once.

## Trace context

agnosai exports OpenTelemetry spans when `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT` (a complete URL,
used as-is) or `OTEL_EXPORTER_OTLP_ENDPOINT` (a base URL: `v1/traces` is appended, as the OTel
specification has it, unless the base already ends in `/v1/traces`) is set
(OTLP/HTTP+JSON; `OTEL_EXPORTER_OTLP_HEADERS` and `OTEL_SERVICE_NAME` are
honoured). The spans follow the OTel GenAI semantic conventions, pinned at
`open-telemetry/semantic-conventions-genai` commit `e07f4eba`
([ADR 023](../adr/023-genai-semconv-spans-and-w3c-trace-context.md)):

| span | kind | one per |
|---|---|---|
| `invoke_workflow {crew name}` | INTERNAL | crew run |
| `invoke_agent {agent name}` | INTERNAL | task |
| `chat {model}` | CLIENT | inference attempt (hoosh call) |
| `execute_tool {tool name}` | INTERNAL | tool call |

Within a crew they form one trace: each task is the workflow's child, and each
inference is its task's child. Since 2.3.8 each `chat` call sends `traceparent`
to the gateway, naming its own span, so hoosh 2.10.3 and later record theirs as
its child and the trace runs on to the provider. With telemetry off nothing is
sent.

agnosai's own server records no span per HTTP request (roadmap B7). A program
that embeds agnosai and serves its own HTTP can record one through the same
exporter: `agnosai_http_server_span_a` builds a SERVER span named
`<method> <route>` with the HTTP semantic-convention attributes, and
`agnosai_telemetry_record_span_in` records it
([ADR 024](../adr/024-an-embedding-consumer-records-http-server-spans.md)).

**A W3C `traceparent` header joins the caller's trace** on the three routes
that start traced work:

- `POST /api/v1/crews` — the crew's workflow span is the header's child;
- `POST /api/v1/a2a/receive` — the delegated crew joins the delegating
  system's trace;
- `POST /mcp` with method `tools/call` — the tool span is the header's child.

Every other route ignores it. The header name is case-insensitive.

- **It never changes a response.** A missing, malformed or unsupported
  `traceparent` (anything but W3C Trace Context Level 1 version `00`: 55 bytes,
  lowercase hex, non-zero ids) is never a 4xx. The work simply starts a new
  trace.
- **Sampling is ParentBased**, the OTel SDK default: a `traceparent` whose
  trace-flags byte has the sampled bit clear (`-00`) suppresses that crew's
  spans entirely.
- ⚠ **The header is trusted as-is.** Any client that can reach these routes
  chooses the trace its spans join, and `-00` is a caller-controlled off switch
  for that work's spans (logs, audit and metrics are unaffected). See the
  [threat model](../development/threat-model.md), surface 7.
- **No span is exported without `OTEL_EXPORTER_OTLP_ENDPOINT`**, header or not.
- **A tool's arguments and result** go on its `execute_tool` span only with
  `AGNOSAI_OTEL_CAPTURE_TOOL_CONTENT=1`, as either can hold what the tool was pointed at. Each is
  filtered by `AGNOSAI_OUTPUT_REDACT` when that is on, and cut at 4 KiB.
- ⚠ **Crew names become span names** (`invoke_workflow {crew name}`, and the
  `gen_ai.workflow.name` attribute). The convention requires that name to be
  low-cardinality, so a client that puts run-specific text in every crew's name
  inflates span-name cardinality in its backend.
- agnosai does not yet send `traceparent` onward to hoosh: hoosh 2.7.1 would
  reuse the chat span's id as its own. That waits on a hoosh fix.

---

## Health Probes

### GET /health

Liveness check. Always returns 200 if the server process is running.

**Response:**

```json
{
  "status": "ok"
}
```

### GET /ready

Readiness check. Returns 200 when the server is fully initialized and ready to accept requests.
`version` is the build's `VERSION` (`AGNOSAI_VERSION`).

**Response:**

```json
{
  "status": "ready",
  "version": "2.1.6"
}
```

---

## Crews

### POST /api/v1/crews

Create and execute a crew. The server assembles the crew from the provided agents and tasks, runs it according to the specified process mode, and returns the results synchronously.

**Request body:**

```json
{
  "name": "security-audit",
  "agents": [
    {
      "agent_key": "security-analyst",
      "name": "Security Analyst",
      "role": "security analyst",
      "goal": "Identify vulnerabilities in the codebase",
      "domain": "security",
      "tools": ["security_audit"],
      "complexity": "high"
    },
    {
      "agent_key": "reporter",
      "name": "Report Writer",
      "role": "technical writer",
      "goal": "Produce clear security reports"
    }
  ],
  "tasks": [
    {
      "description": "Scan all dependencies for known CVEs",
      "expected_output": "List of vulnerabilities with severity ratings",
      "priority": "high"
    },
    {
      "description": "Generate executive summary of findings",
      "expected_output": "One-page security report",
      "priority": "normal",
      "dependencies": [0]
    }
  ],
  "process": "dag"
}
```

**Request fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | yes | Crew name |
| `agents` | array | yes | Agent definitions (see Agent Definition below) |
| `tasks` | array | yes | Task specifications |
| `process` | string | no | Execution mode: `"sequential"` (default), `"parallel"`, `"dag"`, or `"hierarchical"` |
| `manager` | string | for `hierarchical` | The `agent_key` of the agent that manages a hierarchical crew, one of `agents` (2.4.3, [ADR 035](../adr/035-a-manager-assigns-its-workers-and-writes-the-crews-answer.md)). Without it a hierarchical request is a 400 (`a hierarchical crew must name its manager`), as is one naming an agent it does not hold (`manager names no agent in agents`) and a `manager` on any other crew (`manager is only for a hierarchical crew`) |
| `budget` | object | no | `{"max_tokens": N, "max_cost_usd": X}`, either key optional: the most of the model the crew may use across its tasks. It only tightens what the server gives every crew. A crew that reaches it stops, `failed` with `reason: budget_exceeded` (2.4.1) |

**Task fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `description` | string | yes | What the task should accomplish |
| `expected_output` | string | no | Description of the expected result |
| `priority` | string | no | `"background"`, `"low"`, `"normal"` (default), `"high"`, or `"critical"` |
| `dependencies` | array of int | no | Indices into the tasks array indicating which tasks must complete first. Since 2.4.2 a task in a `dag` or `sequential` crew is handed each one's answer, as data, in its request ([ADR 034](../adr/034-a-dependent-task-is-handed-its-dependencies-answers.md)); a `parallel` crew hands none |
| `risk` | string | no | `"low"` (default), `"medium"` or `"high"`. A `high` task's answer waits for a reviewer on `/api/v1/approvals` (2.4.0; the Rust line had no such field) |
| `budget` | object | no | The task's own, in the crew's shape. Reaching it fails this task, and its crew runs on (2.4.1) |

**Agent definition fields:**

| Field | Type | Required | Default |
|-------|------|----------|---------|
| `agent_key` | string | yes | -- |
| `name` | string | yes | -- |
| `role` | string | yes | -- |
| `goal` | string | yes | -- |
| `backstory` | string | no | null |
| `domain` | string | no | null |
| `tools` | array of string | no | [] — the registered tools this agent may call, and no others. ⚠ A name the server does not hold fails the task, naming the tool. That includes the names in `GET /api/v1/presets`, which are a vocabulary for a consumer to resolve |
| `complexity` | string | no | "medium" |
| `llm_model` | string | no | null — the tier `complexity` calls for, from hoosh's catalog (see *Model selection*). An id as hoosh lists it, sent exactly as listed. There are no tier aliases: `"capable"` is sent as the word |
| `gpu_required` | bool | no | false |
| `gpu_preferred` | bool | no | false |
| `gpu_memory_min_mb` | int | no | null |

**Response (200 OK):**

```json
{
  "crew_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "status": "completed",
  "results": [
    {
      "task_id": "f0e1d2c3-b4a5-6789-0fed-cba987654321",
      "output": "Scan all dependencies for known CVEs",
      "status": "completed"
    },
    {
      "task_id": "11223344-5566-7788-99aa-bbccddeeff00",
      "output": "Generate executive summary of findings",
      "status": "completed"
    }
  ]
}
```

**Error response (500):**

```json
{
  "error": "description of what went wrong"
}
```

### GET /api/v1/crews/:id

Retrieve a crew's state from the orchestrator's registry: its `crew_id`, `status`, `results`, and
its `profile` once it has finished.

`status` moves `pending` → `running` → `completed` | `failed` | `cancelled`, and between `running` and
`awaiting_approval` while any of its tasks waits on a reviewer (2.4.0, [ADR 032](../adr/032-the-crew-lifecycle-is-one-state-machine.md)).
A crew is `pending` from the moment it is registered, `running` once it is handed to its runner, and holds its terminal
status when it finishes. A cancel marks it `cancelled` at once and stands even if it lands before the crew starts; a
cancel of a crew that has already finished answers **409** and changes nothing (2.1.6). A timed-out crew is `failed`
with no results and no profile. A `dag` crew whose failed task stranded the tasks that depend on it ends `failed` with
the results it has (2.1.6). ([ADR 022](../adr/022-crew-events-and-status-say-what-happened.md))

**`reason`** says why a crew that did not complete ended, and is left out when it did: `task_failed`, `timeout`,
`cancelled`, `invalid_dag` (its tasks form a cycle, or depend on one the spec does not hold), `not_started` (a
submitted crew whose thread would not start), `budget_exceeded` (its budget, or its tenant's, ran out, 2.4.1) or
`invalid_manager` (a hierarchical crew whose manager names no agent of its own, 2.4.3).
`POST /api/v1/crews` and this route both carry it (2.4.0).

**A crew waits for a turn** (2.4.1): the server runs `max_concurrent_tasks` crews at once (10 by default), and the rest
read `pending` until one ends, in the order they came. ⚠ A waiting `POST /api/v1/crews` holds its connection, and a
pool worker. **A budget is checked before every model call**, and a call it refuses fails its task with the breach as
the task's output: `crew token budget exceeded: 4300 used, 4290 limit` ([ADR 033](../adr/033-budgets-caps-and-crew-concurrency-are-enforced.md)).

**No edge runs backwards since 2.4.0.** Through 2.3.8 a DAG crew that could not run went `running` → `pending` and
stayed there; it now ends `failed` with `reason: invalid_dag` and sends `crew_completed`. `POST /api/v1/crews` cannot
produce one: its `dependencies` are task indices, range-checked and cycle-checked, so such a request gets a 400 before
the crew is registered. An embedding caller that builds its own spec can.

**A task whose risk is `high` waits for a reviewer** (2.4.0). Its answer is held, the task reads `awaiting_approval`,
and `GET /api/v1/approvals` lists it until `POST /api/v1/approvals` decides, the gate's five minutes pass, or the crew
is cancelled or runs out of time. Approved, the answer stands. Anything else rejects: the task fails, and its output is
`output withheld: approval rejected (<reason>)`. A `medium` task waits too when the orchestrator is told so. The task's
result `metadata` gains `approval` (`approved` or `rejected`) and `approval_reason`. ⚠ A `POST /api/v1/crews` holding
such a task holds its connection, and a pool worker, while it waits.

A task whose agent called tools adds `metadata.tool_calls`, and an entry in this response's `results`
gains the same list as `tool_calls`, one entry per call in the order the model made them: `id`, `tool`, `arguments`, `effect` (`committed`, `none` or `unknown`), `success`, and
`result` or `error`, with `flagged`, `held_back` or `asked_for` when they apply ([ADR 029](../adr/029-the-tool-loop.md)).
`metadata.tool_calls_dropped` counts calls past the limit of a turn, when there were any.

**A hierarchical crew** (2.4.3, [ADR 035](../adr/035-a-manager-assigns-its-workers-and-writes-the-crews-answer.md))
assigns each task to the best of its workers, every agent but its `manager`, and runs them in order, each handed the
answers of the dependencies listed before it. Each worker's entry in `results` carries `assigned_by`, the manager's
key, and `withheld_tools`, the tools its definition names that its manager does not hold, which it was not offered.
Then the manager writes the crew's answer from every worker's, offered no tool: **its entry is the last**, with
`"role": "manager"`, so a crew of three tasks returns four results, and `crew_started`'s `task_count` says 4. A crew
whose worker failed, or that was stopped, has no manager's answer and does not complete.

An LLM-answered task's result `metadata` carries `model`, `tokens`, `task_duration_ms`, `provider`
(the gateway's `usage.provider`, else the catalog's `owned_by` for the model, and omitted when neither
says), `cost_micro_usd` (when the gateway reports a cost), and `agent` — the assigned agent's key — so
the profile's `agent_cost_usd` is broken down per agent.

**Response (200 OK):**

```json
{
  "crew_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "status": "running",
  "results": []
}
```

**Response (404):** the id is not registered. Once 1,000 finished crews are held, a submit evicts
the ones that ended longest ago, down to 900 ([ADR 026](../adr/026-the-crew-registry-keeps-what-has-not-been-read.md)),
so a crew stays readable until 900 others have ended after it. A malformed id is **422**.

```json
{
  "error": "crew not found"
}
```

### Crew events — GET /api/v1/crews/:id/stream

A Server-Sent Events stream of one crew's progress. Each frame is `id: <seq>`, `event: <type>` and
`data: <json>`, where the JSON is `{"crew_id", "event_type", "seq", "data"}`. Every event carries the
crew's own id ([ADR 032](../adr/032-the-crew-lifecycle-is-one-state-machine.md)).

**`seq` numbers the events the crew's sender delivered**, from 1, in the one order every subscriber sees, whatever
thread sent each. A subscriber attached for the whole run sees 1, 2, 3 … with no gap, so a gap is an event its buffer
dropped. An event nobody was subscribed to receive takes no number, so one that attaches mid-run starts where the count
had reached. The stream's own `connected` and `error` frames have no `seq` and no `id:`. Nothing is replayed from a
`Last-Event-ID`.

| `event_type` | When | `data` |
|---|---|---|
| `crew_started` | the run begins | `name`, `task_count`, and `resumed: true` for a crew resumed from its log (2.5.0) |
| `task_started` | the task is dispatched — for a parallel or DAG crew, when its batch starts | `task_id`, `description`, `agent` (key or null) |
| `tool_call_started` | a tool call is about to run | `task_id`, `call_id`, `tool` |
| `tool_call_completed` | a tool call's result is delivered | `task_id`, `call_id`, `tool`, `status` (`ok`, `error`, `refused`), `asked` for a tool the agent does not hold |
| `token` | an LLM-answered task succeeds — one per task, in every process mode | `task_id`, `token` (the whole output), `complete: true` |
| `approval_requested` | the task's answer waits for a reviewer, or, with `resume: true`, a resumed task waits to run again over calls whose effect may stand (2.5.0) | `task_id`, `output`, `risk`, `agent`, `resume` |
| `approval_decided` | the wait ends | `task_id`, `decision` (`approved`, `rejected`), `reason` (`reviewer`, `timeout`, `cancelled`, `deadline`, `capacity`, `no_gate`) |
| `task_completed` | the task finishes | `task_id`, `status` |
| `crew_completed` | the run ends — the last event, on every exit | `status`, `reason` (unless `completed`), `task_count`, `wall_ms` |

- Every event is a legal move of the lifecycle ADR 032 writes down, and `tests/orch_lifecycle.tcyr` checks every
  stream it runs against it. A task's `token`, tool calls and approval events come between its `task_started` and
  its `task_completed`. A refused tool call never ran, so it has `tool_call_completed` only.
- For a parallel or DAG crew, events of different tasks interleave (`s0 s1 c1 c0 s2 …`): do not assume every
  `task_started` precedes every `task_completed`.
- **`task_completed` arrives in completion order** (2.4.0): a task that finishes early is reported before a slower one
  dispatched before it. Since 2.4.1 a parallel crew is a sliding window, not batches: a task starts as soon as a slot
  frees, so a `task_started` can come between two `task_completed`. Every task of a DAG wave is reported before the
  next wave starts. The crew's `results` stay in dispatch order.
- A task stopped before dispatch, by a cancel, the deadline or its crew's budget, gets neither `task_started` nor
  `task_completed`.
- `crew_completed`'s `status` and `reason` match the state `GET /api/v1/crews/:id` stores: a timed-out crew says
  `failed`, `reason: timeout`, with `task_count: 0`. A crew that cannot run sends `crew_started` and `crew_completed`
  (`failed`, `invalid_dag`) and nothing between.
- The stream opens with a `connected` frame. A subscriber whose 256-event buffer overflows gets an `error` frame
  (`stream lagged, N events dropped`) and the stream closes. Events sent before a client subscribed are not replayed.
  The runner checks for a subscriber per task (sequential) and per batch (parallel, DAG), so a client that subscribes
  mid-run is told about the tasks after it (2.4.0; through 2.3.8 it heard nothing until `crew_completed`).
- A crew with a log (2.5.0, `AGNOSAI_STATE_DIR`) numbers every event, as its log receives each one, so `seq` counts
  from the crew's start whoever is watching; a resumed crew's go on from its log's last.

### Crew snapshot — GET /api/v1/crews/:id/snapshot

Where a crew is, read in one step with its events (2.5.0,
[ADR 036](../adr/036-a-crew-keeps-a-log-and-resumes-from-it.md)). A client whose stream fell behind, or that
reconnected, reads this, takes each task's status as it says, and then takes the events after `seq`: no event at or
before `seq` is missing from it, and none after is in it.

```json
{
  "crew_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
  "status": "running",
  "seq": 7,
  "tasks": [
    {"task_id": "…", "status": "completed"},
    {"task_id": "…", "status": "running"},
    {"task_id": "…", "status": "pending"}
  ]
}
```

`status` is the crew's: `pending` before `crew_started`, `running`, `awaiting_approval` while a task waits on a
reviewer, then its end with `crew_completed`. The tasks are the crew's in order, then a hierarchical crew's manager
task once it has one. A crew put back from its log after a restart has no events in this process and reads its state,
at `seq` 0. 404 for a crew the registry does not hold, 422 for an id that is not a UUID.

---

## Agent Definitions

### GET /api/v1/agents/definitions

List all registered agent definitions.

**Response (200 OK):**

```json
[]
```

Currently returns an empty array. Full persistence is on the roadmap.

### POST /api/v1/agents/definitions

Register a new agent definition.

**Request body:**

```json
{
  "agent_key": "data-engineer",
  "name": "Data Engineer",
  "role": "data engineer",
  "goal": "Build reliable data pipelines",
  "domain": "data-engineering",
  "tools": ["json_transform"],
  "complexity": "high"
}
```

**Response (201 Created):**

Returns the accepted definition as JSON (echo).

---

## Tools

### GET /api/v1/tools

List all registered tools with their schemas.

**Response (200 OK):**

```json
[
  {
    "name": "echo",
    "description": "Echoes the input back (for testing)",
    "parameters": [
      {
        "name": "input",
        "description": "The text to echo",
        "param_type": "string",
        "required": true
      }
    ]
  },
  {
    "name": "json_transform",
    "description": "Extract fields from a JSON object",
    "parameters": [
      {
        "name": "data",
        "description": "JSON object to transform",
        "param_type": "object",
        "required": true
      },
      {
        "name": "fields",
        "description": "List of field paths to extract",
        "param_type": "array",
        "required": true
      }
    ]
  }
]
```

---

## Presets

### GET /api/v1/presets

List the built-in crew presets: three examples of the format compiled from `src/presets/`,
`security-lean`, `security-standard` and `security-large`
([ADR 027](../adr/027-the-built-in-presets-are-an-example-library.md)), each with its agents
rendered as an agent definition is.

**Response (200 OK):**

```json
[
  {"name": "security-lean", "description": "…", "domain": "security", "size": "lean",
   "version": "…", "agents": [{"agent_key": "…", "name": "…", "role": "…", "goal": "…"}]}
]
```

(Until the 2.1.7 docs sweep this page said the endpoint returned `[]` and was "on the
roadmap"; it has served the library since the preset port.)

---

## Running the Server

```bash
cyrius build src/main.cyr build/agnosai
./build/agnosai
```

The server binds to `0.0.0.0:8080` by default. Use `PORT` or `AGNOSAI_PORT` to change the port.

### Durable crews — `AGNOSAI_STATE_DIR`

With `AGNOSAI_STATE_DIR` set, every crew keeps a log there, `<crew_id>.log`, one JSON line a record: its spec, every
event it sends, each tool call before it runs, and each finished task's result
([ADR 036](../adr/036-a-crew-keeps-a-log-and-resumes-from-it.md)). A start reads them back before the server listens:

- a crew whose log ended answers `GET /api/v1/crews/{id}` as it ended;
- a crew whose log did not, because the last process stopped mid-run, is **resumed**: what it finished stands and is
  not run again, an answer that waited on a reviewer is asked about again, and the rest runs, its events numbered on
  from its log's last;
- **a task whose earlier attempt made a tool call that may have taken effect waits for a reviewer** before it runs
  again (`approval_requested` with `resume: true`, decided through `POST /api/v1/approvals`): approved, it runs; rejected,
  timed out after five minutes, or with no gate, it fails as `not resumed: …`, and its result's `uncertain_calls`
  names each call.

A log is written `0600`, as it holds answers and tool arguments, and is removed when its crew leaves the registry.
Unset or empty, crews keep no log and a restart loses them, as through 2.4.3. A server claims its directory as it
starts, a lock on `<dir>/.owner`, so a second on the same directory refuses to start rather than resume the first's
live crews.

## MCP — `POST /mcp`

One endpoint, two revisions, chosen per request ([ADR 039](../adr/039-mcp-serves-both-eras-on-one-endpoint.md)):

- **2026-07-28**, when `MCP-Protocol-Version: 2026-07-28` is sent. Every request carries `_meta` with
  `io.modelcontextprotocol/protocolVersion` and `io.modelcontextprotocol/clientCapabilities`, and the headers
  `Mcp-Method` and, for `tools/call`, `prompts/get` and `resources/read`, `Mcp-Name`, which must say what the body
  says. `server/discover` answers what the endpoint speaks and can do; there is no `initialize`. A header that is
  missing or disagrees is 400 with `-32020`, an unsupported revision 400 with `-32022` and the supported ones in
  `data.supported`, an unknown method 404, and a notification 202.

  ```bash
  curl -s localhost:8080/mcp -H 'Content-Type: application/json' \
    -H 'MCP-Protocol-Version: 2026-07-28' -H 'Mcp-Method: tools/call' -H 'Mcp-Name: echo' \
    -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"echo","arguments":{"message":"hi"},
        "_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28",
                 "io.modelcontextprotocol/clientCapabilities":{}}}}'
  ```

- **2025-03-26**, for a request with no version header or one naming it: `initialize`, then the same methods, every
  answer HTTP 200, as through 2.5.x.

## A2A 1.0 — the Agent Card and `POST /a2a`

`GET /.well-known/agent-card.json` answers the Agent Card, and `POST /a2a` the JSON-RPC binding at 1.0, whose tasks
are crews ([ADR 040](../adr/040-a2a-v1-tasks-are-crews.md)). Every request sends `A2A-Version: 1.0`; without it the
request is 0.3's and refused with `VersionNotSupportedError` (`-32009`). Every answer is HTTP 200, an error in band.

| Method | What it does |
|---|---|
| `SendMessage` | Starts a crew for the message's text and data parts, waits for it unless `returnImmediately`, and answers `{"task": Task}` |
| `GetTask` | The task, with `historyLength` messages of its history |
| `ListTasks` | The tasks this endpoint started, by id, filtered by `contextId` and `status`, `pageSize` at a time |
| `CancelTask` | Cancels it; a task that has ended is `TaskNotCancelableError` |
| `SendStreamingMessage`, `SubscribeToTask` | An SSE stream: the task, its status updates, its artifacts, and its end |

Push notifications and the extended card are not supported, and the card says so. A task lives as long as its crew is
in the registry.

```bash
curl -s localhost:8080/a2a -H 'Content-Type: application/json' -H 'A2A-Version: 1.0' \
  -d '{"jsonrpc":"2.0","id":1,"method":"SendMessage","params":{"message":{"messageId":"m-1",
      "role":"ROLE_USER","parts":[{"text":"review the release notes"}]}}}'
```

| Variable | |
|---|---|
| `AGNOSAI_PUBLIC_URL` | The URL clients reach the server at, `https://host[:port]`, which the card names `/a2a` under. Unset, the card names each request's `Host`, over `http` |
| `AGNOSAI_A2A_SIGNING_KEY` | A PEM private key, P-256 (ES256) or RSA (RS256), that signs the card once at start-up. Needs `AGNOSAI_PUBLIC_URL` |
| `AGNOSAI_A2A_SIGNING_KID` | The key's id in each signature's header; `agnosai` unset |
| `AGNOSAI_A2A_JKU` | Where the key's public half is published, a JWKS URL, named in the header when set |

`POST /api/v1/a2a/receive`, the Rust line's delegation shape, is unchanged.

## Content Type

All endpoints accept and return `application/json`.
