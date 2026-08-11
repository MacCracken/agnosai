# ADR-002: AGNOS Ecosystem as Optional Tool Backends

## Status: Accepted

**Date**: 2026-03-18 (Rust era) — re-verified against the Cyrius tree 2026-08-11.
The decision, the nine tools, their endpoints and their default ports all survive
the port unchanged; only the HTTP library and the constructor shape moved. See
the port note at the end.

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
- **No compile-time coupling** — the tools issue plain HTTP requests; none of the three services is a declared dependency
- **Configurable endpoints** — the base URL is a constructor argument, defaulted per service
- **Standalone deployments** — AgnosAI works without any AGNOS services running

## Consequences

- Agents must be configured with available tools at crew assembly time
- Tool availability should be checked at startup (health endpoints). **This has never been implemented** — in `rust-old/` or in the port. Nothing calls `synapse_status` or any other probe before a crew runs; an unreachable service surfaces as a per-invocation tool error
- Future tools for other AGNOS services follow the same pattern

## Port note — 2026-08-11

Everything above verifies against the Cyrius tree, item for item. Confirmed by
reading `src/tools/agnos.cyr` and `src/tools/builtin/{synapse,mneme,delta}.cyr`:
the three default base URLs are the literals `AGNOSAI_SYNAPSE_BASE_URL` /
`AGNOSAI_MNEME_BASE_URL` / `AGNOSAI_DELTA_BASE_URL` on ports 8420 / 8400 / 8070;
all nine tool names appear verbatim as schema names; every path in the tables
above is the string the tool actually sends, including the `?q=…&limit=…` query
that `agnosai_mneme_search_path` builds and the
`/api/v1/<owner>/<repo>/pipelines[/<id>]` that `_agnosai_delta_pipeline_path`
builds. `cyrius.cyml` declares no synapse, mneme or delta dependency, so
"no compile-time coupling" holds literally.

Three sentences needed correcting, none of which changes the decision:

- **`reqwest` is gone.** The Rationale said the tools *"use `reqwest` HTTP calls,
  not crate dependencies"*. The port's transport is
  `agnosai_agnos_http_transport` in `src/tools/agnos.cyr`, built on sandhi
  (`sandhi_http_request_auto_a` over a per-exchange arena). The point the bullet
  was making — HTTP at runtime rather than a linked SDK — is unaffected, so the
  library name has simply been dropped from it.
- **`new()` / `with_base_url()` are gone.** The port has one constructor pair:
  `agnosai_agnos_client(base_url, service)` and
  `agnosai_agnos_client_new(base_url, service, transport_fp)`, with
  `agnosai_{synapse,mneme,delta}_client()` supplying the defaults. `new()` was
  only ever `with_base_url(DEFAULT_BASE_URL)`, so nothing is lost. The third
  parameter is new: the transport is a function pointer so the tools can be
  tested end to end against a synthetic responder, since none of these services
  runs in CI.
- **The port adds a timeout the oracle does not have.** `reqwest::Client::new()`
  applies none, so a hung AGNOS service would hang the agent forever;
  `AGNOSAI_AGNOS_TIMEOUT_MS` is 30s and `AGNOSAI_AGNOS_MAX_BYTES` caps a response
  at 4 MiB. The port also does **not** follow redirects, where reqwest does. Both
  are stated in `src/tools/agnos.cyr`'s header; they strengthen "graceful
  degradation" rather than qualifying it.

One thing worth knowing that this ADR never said: **these tools deliberately do
not run the SSRF guard.** They exist to reach AGNOS services on loopback, which
`agnosai_is_safe_url` would reject outright. What a caller *does* control — the
path segments interpolated into the URL — is guarded by
`agnosai_agnos_segment_ok` at exactly the points the oracle guards it. The
reasoning is in `src/tools/agnos.cyr`'s header; see also
[ADR 007](007-audit-redirect-revalidation.md) for the guard that *does* apply to
caller-chosen URLs.
