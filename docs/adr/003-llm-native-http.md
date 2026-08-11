# ADR-003: Native HTTP for LLM Providers

## Status: Accepted — the principle holds, the mechanism moved twice

**Date**: 2026-03-18 (Rust era). **Amended 2026-08-11** after reviewing the ADR
against the Cyrius tree. The original text described agnosai implementing three
providers itself; it has not done that since before `rust-old/` was frozen, and
the port moved the seam again. The revision history below is the load-bearing
part of this document.

## Context

Agnostic v1 uses litellm (via CrewAI) for LLM provider abstraction. This pulls in 200+ transitive Python dependencies and introduces version conflicts.

## Decision

**No Python and no litellm anywhere in the LLM path.** Inference is reached over
plain HTTP, and agnosai depends on nothing that carries a provider SDK.

Today that means: agnosai implements **zero** providers. It speaks the
OpenAI-compatible chat-completions protocol to **hoosh**, which owns every
provider implementation, and it does so over a **network seam** rather than a
linked library.

## Revision history — the same decision, three different mechanisms

**As written, 2026-03-18.** agnosai implemented each provider itself as direct
`reqwest` calls — OpenAI (`POST /v1/chat/completions`), Anthropic
(`POST /v1/messages`, `x-api-key`), Ollama (`POST /api/chat`) — roughly 100–150
lines of Rust each, behind an `LlmProvider` trait.

**By `rust-old/` v1.1.0, already superseded — providers moved into hoosh.** This
is visible in the frozen oracle and predates the port entirely:

- `rust-old/Cargo.toml:96` —
  `hoosh = { version = "1.1.0", default-features = false, features = ["all-providers", "dlp"] }`
- `rust-old/src/llm/mod.rs` is a **`pub use hoosh::…` facade and nothing else**.
  Its own doc comment says so: *"All provider implementations, token budgeting,
  response caching, streaming, cost tracking, and metrics are provided by the
  `hoosh` crate."* There is no provider code in `rust-old/src/llm/` — the
  directory holds only `mod.rs`, `router.rs`, `retry.rs` and `inference_queue.rs`.

So the "Providers Implemented" table and the "~100-150 lines of Rust each,
auditable" rationale describe a shape the Rust tree had **stopped having** before
it was frozen. That was never recorded here.

**The port, 2026 — hoosh becomes an HTTP seam, because it cannot be linked.**
hoosh in Cyrius is a **binary**: no `dist/` module, no `[lib]` stanza, nothing to
declare in `[deps]`. Every `pub use hoosh::…` line therefore evaporates, and the
facade is replaced by a client that talks to a running gateway. The reference
implementation for the seam is `thoth/src/hoosh.cyr`.

`src/llm/hoosh.cyr` is that client:

| | |
|---|---|
| Default gateway | `agnosai_hoosh_default_url()` → `http://127.0.0.1:8088` |
| Chat | `agnosai_hoosh_chat_path()` → `POST /v1/chat/completions` |
| Models | `agnosai_hoosh_models_path()` → `GET /v1/models` |
| Client | `agnosai_hoosh_client_new(base_url, api_key)` — two `Str`s, no lazy init |
| The one I/O call | `agnosai_hoosh_chat`, over `sandhi_http_post` |

Everything else in the module — request/response builders, extractors, the
`AgnosaiChatRole` / `AgnosaiProviderType` / `InferenceRequest` /
`InferenceResponse` types — is pure, which is what lets the seam be unit-tested
with no gateway running. The seam targets **hoosh 2.6.0**
(`usage.cost_micro_usd`, `usage.provider`, `X-Hoosh-Cache` are read when present;
an older gateway degrades to an absent cost rather than a fabricated one).

`AgnosaiProviderType` carries seven variants (OpenAI, Anthropic, Google, Mistral,
DeepSeek, Grok, Ollama), but **agnosai never puts one on a wire it controls** —
the gateway is addressed by model string and hoosh owns the provider mapping. The
enum exists because the rest of agnosai references it, not because it routes
anything.

## What agnosai still owns, and what it does not

**Still agnosai's, and ported:**

- **Model router** — task-complexity scoring selects Fast / Capable / Premium
  (`src/llm/router.cyr`).
- **Retry** — exponential backoff with jitter and an error-text classifier
  (`src/llm/retry.cyr`, `agnosai_is_retryable`).
- **Priority inference queue** — background work waits behind interactive crew
  work (`src/llm/inference_queue.cyr`, over majra's `cpq_*`).
- **Token and cost budget** — `src/orchestrator/budget.cyr`, checked before every
  inference call. The original "Token Budget — per-agent accounting (planned)"
  line was wrong in two ways: it shipped, and it is **per-crew**, not per-agent.

**Not agnosai's, because an HTTP seam has no client-side equivalent:**
`AuditChain`, `ResponseCache`, `CostTracker`, the metrics registry, `DlpScanner`,
`RateLimitRegistry`, `TokenCounter`, `ContextCompactor`. hoosh implements these
server-side and does not expose them over the wire. Deleting the client-side
response cache in particular is a **correctness** fix rather than a
simplification — see `src/orchestrator/crew_runner.cyr`'s header. The measured
reference counts for every dropped re-export are in `src/llm/mod.cyr`'s header.

**Two Provider-Infrastructure bullets described things that are not in this repo
at all**, in `rust-old/` or in `src/`, and have been removed rather than left to
mislead:

- *"Health Ring Buffer — 5-point buffer per provider; 3 consecutive failures →
  unhealthy"*. Nothing matching this exists in either tree. If it exists it is
  hoosh's, and this repo cannot vouch for the numbers.
- *"Rate Limiter — semaphore-based concurrent request limiting"*. agnosai's only
  rate limiter is `src/server/rate_limit.cyr` — a majra token bucket on
  **inbound** HTTP, not on outbound inference, and it is not mounted by default.

## Consequences

- **Zero Python in the LLM path**, which is the whole point and is still true.
- **Adding a provider is not an agnosai change.** It is a hoosh change. The
  `LlmProvider` trait no longer exists here; `docs/guides/adding-providers.md`
  still documents implementing it and is stale.
- **The gateway is an operational dependency.** With `llm_url` unset the
  orchestrator takes a placeholder path rather than failing, but real inference
  needs a reachable `hoosh serve`. The Rust tree could link hoosh into the
  binary; the Cyrius tree cannot.
- **Streaming and model-name mapping are the gateway's problem**, not agnosai's.
  The original consequences said the opposite for both.
- Base URL and API key are constructor arguments, so any OpenAI-compatible
  gateway can stand in for hoosh.
