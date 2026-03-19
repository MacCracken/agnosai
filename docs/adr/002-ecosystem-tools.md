# ADR-002: AGNOS Ecosystem as Optional Tool Backends

**Status:** Accepted
**Date:** 2026-03-18

## Context

The AGNOS ecosystem includes several sibling services:

- **Synapse** — LLM inference controller (7 backends, model routing, fleet)
- **Mneme** — AI-native knowledge base (semantic search, hybrid retrieval, vault persistence)
- **Delta** — Code hosting platform (git, CI/CD pipelines, artifact registry)

AgnosAI agents benefit from accessing these services, but hard-coupling to them would limit standalone deployments.

## Decision

Synapse, Mneme, and Delta are exposed as **optional native tools** — HTTP-client tools that agents can invoke at runtime. They are not compile-time dependencies of AgnosAI.

## Tool Inventory

### Synapse (default: `http://localhost:8420`)
| Tool | Endpoint | Purpose |
|------|----------|---------|
| `synapse_infer` | `POST /v1/chat/completions` | Run inference through local models |
| `synapse_list_models` | `GET /v1/models` | List available models |
| `synapse_status` | `GET /system/status` | Hardware, backends, loaded models |

### Mneme (default: `http://localhost:8400`)
| Tool | Endpoint | Purpose |
|------|----------|---------|
| `mneme_search` | `GET /api/search` | Hybrid keyword + semantic search |
| `mneme_get_note` | `GET /api/notes/:id` | Retrieve note with backlinks |
| `mneme_create_note` | `POST /api/notes` | Store agent findings as notes |

### Delta (default: `http://localhost:8070`)
| Tool | Endpoint | Purpose |
|------|----------|---------|
| `delta_list_repos` | `GET /api/v1/repos` | List repositories |
| `delta_trigger_pipeline` | `POST /api/v1/:owner/:name/pipelines` | Trigger CI/CD |
| `delta_get_pipeline` | `GET /api/v1/:owner/:name/pipelines/:id` | Pipeline status |

## Rationale

- **Graceful degradation** — if a service is unavailable, the tool returns an error; the agent continues
- **No compile-time coupling** — tools use `reqwest` HTTP calls, not crate dependencies
- **Configurable endpoints** — each tool has `new()` (default URL) and `with_base_url()` constructors
- **Standalone deployments** — AgnosAI works without any AGNOS services running

## Consequences

- Agents must be configured with available tools at crew assembly time
- Tool availability should be checked at startup (health endpoints)
- Future tools for other AGNOS services follow the same pattern
