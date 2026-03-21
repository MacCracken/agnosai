# API Reference

AgnosAI exposes a REST API via the `agnosai-server` binary, built on [axum](https://github.com/tokio-rs/axum).

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Liveness probe |
| GET | `/ready` | Readiness probe (includes version) |
| POST | `/api/v1/crews` | Create and execute a crew |
| GET | `/api/v1/crews/:id` | Get crew status (placeholder) |
| GET | `/api/v1/agents/definitions` | List agent definitions |
| POST | `/api/v1/agents/definitions` | Register an agent definition |
| GET | `/api/v1/tools` | List registered tools |
| GET | `/api/v1/presets` | List presets |

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

**Response:**

```json
{
  "status": "ready",
  "version": "0.21.3"
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

Retrieve the status of a previously submitted crew. Currently returns 404 for all requests (full state tracking is future work).

**Response (404):**

```json
{
  "error": "crew not found"
}
```

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
cargo run --bin agnosai-server
```

The server binds to `0.0.0.0:8080` by default. Use `PORT` or `AGNOSAI_PORT` to change the port.

## Content Type

All endpoints accept and return `application/json`.
