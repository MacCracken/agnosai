# ADR-003: Native HTTP for LLM Providers

**Status:** Accepted
**Date:** 2026-03-18

## Context

Agnostic v1 uses litellm (via CrewAI) for LLM provider abstraction. This pulls in 200+ transitive Python dependencies and introduces version conflicts.

## Decision

Every LLM provider is implemented as direct HTTP calls via `reqwest`. No Python SDKs, no litellm.

## Providers Implemented

| Provider | Protocol | Notes |
|----------|----------|-------|
| OpenAI | REST `/v1/chat/completions` | Also covers OpenAI-compatible (vLLM, TGI) |
| Anthropic | REST `/v1/messages` | Direct HTTP with `x-api-key` header |
| Ollama | REST `/api/chat` | Local inference |

## Provider Infrastructure

- **Model Router** — task-complexity scoring selects tier (Fast / Capable / Premium)
- **Health Ring Buffer** — 5-point buffer per provider; 3 consecutive failures → unhealthy
- **Rate Limiter** — semaphore-based concurrent request limiting
- **Response Cache** — LRU + TTL (planned)
- **Token Budget** — per-agent accounting (planned)

## Rationale

- Zero Python in the LLM path
- Each provider is ~100-150 lines of Rust — auditable, no hidden behavior
- `with_base_url()` constructors support any OpenAI-compatible API
- Health tracking enables automatic failover

## Consequences

- New providers require implementing the `LlmProvider` trait (~100 LoC each)
- Streaming responses need separate implementation per provider protocol
- No automatic model name mapping — caller specifies the model string
