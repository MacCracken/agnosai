# API Reference

AgnosAI exposes a REST API from the `agnosai` binary, served by
[sandhi](https://github.com/MacCracken/sandhi) over a pooled worker model.

> Both names in the previous sentence changed with the Cyrius port: the binary
> was `agnosai-server` and the server was axum. The routes, status codes and
> bodies below did not — they are held to the Rust tree as the parity oracle.

## Endpoints

⚠ This table listed **6** of the **18** routes the router serves. The rest were
undocumented, and `/api/v1/crews/{id}` was marked "(placeholder)" long after it
started returning real state.

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/health` | no | Liveness probe |
| GET | `/ready` | no | Readiness probe (includes version) |
| GET | `/metrics` | no | Prometheus exposition |
| POST | `/api/v1/crews` | yes | Create and execute a crew |
| GET | `/api/v1/crews/{id}` | yes | The crew's state |
| POST | `/api/v1/crews/{id}/cancel` | yes | Request cancellation |
| GET | `/api/v1/crews/{id}/stream` | yes | SSE event stream for one crew |
| GET | `/api/v1/agents/definitions` | yes | List agent definitions |
| POST | `/api/v1/agents/definitions` | yes | Register an agent definition |
| GET | `/api/v1/tools` | yes | List registered tools |
| GET | `/api/v1/tools/{name}` | yes | One tool's schema |
| GET | `/api/v1/presets` | yes | List presets |
| GET | `/api/v1/dashboard/crews` | yes | Crew history |
| GET | `/api/v1/dashboard/agents` | yes | Per-agent performance |
| POST | `/api/v1/approvals` | yes | Submit a human-in-the-loop decision |
| POST | `/api/v1/a2a/receive` | yes | Accept a delegated task |
| POST | `/api/v1/a2a/status` | yes | A2A status |
| POST | `/mcp` | yes | Model Context Protocol JSON-RPC |

The three probes are the only unauthenticated routes, and the allow-list is
written that way deliberately — a route added without thought defaults to
**protected**.

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

## Trace context

agnosai exports OpenTelemetry spans when `OTEL_EXPORTER_OTLP_ENDPOINT` is set
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
inference is its task's child.

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
  "version": "2.1.5"
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
      "tools": ["vulnerability_scan", "dependency_audit"],
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

**Task fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `description` | string | yes | What the task should accomplish |
| `expected_output` | string | no | Description of the expected result |
| `priority` | string | no | `"background"`, `"low"`, `"normal"` (default), `"high"`, or `"critical"` |
| `dependencies` | array of int | no | Indices into the tasks array indicating which tasks must complete first |

**Agent definition fields:**

| Field | Type | Required | Default |
|-------|------|----------|---------|
| `agent_key` | string | yes | -- |
| `name` | string | yes | -- |
| `role` | string | yes | -- |
| `goal` | string | yes | -- |
| `backstory` | string | no | null |
| `domain` | string | no | null |
| `tools` | array of string | no | [] |
| `complexity` | string | no | "medium" |
| `llm_model` | string | no | null |
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

`status` moves `pending` → `running` → `completed` | `failed` | `cancelled`. A crew is `pending` from
the moment it is registered, `running` once it is handed to its runner, and holds its terminal
status when it finishes. A cancel marks it `cancelled` at once and stands even if it lands before
the crew starts. A timed-out crew is `failed` with no results and no profile.
([ADR 022](../adr/022-crew-events-and-status-say-what-happened.md))

**One edge runs backwards: `running` → `pending`.** A DAG crew whose run ends in an error rather
than a state goes back to `pending` and stays there, since that arm stores no terminal status. A
poller can see it go `running` first, because the crew is marked `running` before its runner
checks the DAG. There are two such errors:

- **A cyclic DAG.** The runner's topological sort fails before any task runs, so the crew sends
  `crew_started` and no task events.
- **A DAG deadlock.** A task that depends on an id not in the spec is never ready, and the sort
  does not catch it, since it skips dependency ids it cannot find. Every wave before the stuck
  task runs, and those tasks send `task_started`, `task_completed` and `token` events as usual.
  Then the crew goes back to `pending`.

Neither error sends `crew_completed`. The run returns the error instead of a state.

`POST /api/v1/crews` cannot produce either error. Its `dependencies` are task indices, and the
route range-checks and cycle-checks them, so such a request gets a 400 before the crew is
registered. Only an embedding caller can reach this edge: one that builds its own spec and runs it
through `agnosai_orchestrator_submit_crew` or `agnosai_orchestrator_run_crew_err`.

An LLM-answered task's result `metadata` carries `model`, `provider`, `tokens`, `cost_micro_usd`
(when the gateway reports a cost), `task_duration_ms`, and `agent` — the assigned agent's key — so
the profile's `agent_cost_usd` is broken down per agent.

**Response (200 OK):**

```json
{
  "crew_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "status": "running",
  "results": []
}
```

**Response (404):** the id is not registered (finished crews are evicted once the registry holds
1,000). A malformed id is **422**.

```json
{
  "error": "crew not found"
}
```

### Crew events — GET /api/v1/crews/:id/stream

A Server-Sent Events stream of one crew's progress. Each frame is `event: <type>` and
`data: <json>`, where the JSON is `{"crew_id", "event_type", "data"}`. Every event carries the
crew's own id.

| `event_type` | When | `data` |
|---|---|---|
| `crew_started` | the run begins | `name`, `task_count` |
| `task_started` | the task is dispatched — for a parallel or DAG crew, when its batch starts | `task_id`, `description`, `agent` (key or null) |
| `token` | an LLM-answered task succeeds — one per task, in every process mode | `task_id`, `token` (the whole output), `complete: true` |
| `task_completed` | the task finishes — for a parallel or DAG crew, when its worker is joined (see below) | `task_id`, `status` |
| `crew_completed` | the run ends | `status`, `task_count`, `wall_ms` |

- A task's `token` precedes its `task_completed`. For a parallel or DAG crew, events of different
  tasks interleave (`s0 s1 c0 c1 s2 …`): do not assume every `task_started` precedes every
  `task_completed`.
- A parallel or DAG crew joins each batch in dispatch order, so `task_completed` arrives in
  dispatch order, not completion order. A task that finishes early is reported after any slower
  task dispatched before it in the same batch. Every task of a batch is reported before the next
  batch starts. A DAG wave is one batch.
- A task stopped before dispatch, by a cancel or the deadline, gets neither `task_started` nor
  `task_completed`.
- `crew_completed`'s `status` matches the state `GET /api/v1/crews/:id` stores: a timed-out crew
  says `failed` with `task_count: 0`.
- A DAG crew whose run ends in an error sends no `crew_completed`. That covers a cyclic DAG and
  a DAG deadlock (see the status lifecycle above). It sends `crew_started` and the task events of
  any waves that ran.
- The stream opens with a `connected` frame. A subscriber whose 256-event buffer overflows gets
  an `error` frame (`stream lagged, N events dropped`) and the stream closes. Events sent before
  a client subscribed are not replayed. The runner also checks for a subscriber only once per run
  (sequential) or once per wave (parallel, DAG) before building `task_started` and
  `task_completed`. A client that subscribes mid-run can therefore miss those events for the rest
  of that run or wave. It still receives `crew_completed`, which is checked when the run ends.

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

List available crew presets.

**Response (200 OK):**

```json
[]
```

Currently returns an empty array. The preset library endpoint is on the roadmap.

---

## Running the Server

```bash
cyrius build src/main.cyr build/agnosai
./build/agnosai
```

The server binds to `0.0.0.0:8080` by default. Use `PORT` or `AGNOSAI_PORT` to change the port.

## Content Type

All endpoints accept and return `application/json`.
