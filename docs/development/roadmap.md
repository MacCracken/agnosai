# AgnosAI Roadmap

> Rust-native agent orchestration — replacing CrewAI with a purpose-built framework.

AgnosAI distills production-proven patterns from three systems:

- **Agnosticos (daimon)** — Orchestrator, IPC, pub/sub, scoring, scheduling, resource management, RL optimizer, federation (8997+ tests)
- **Agnosticos (hoosh)** — LLM provider abstraction, health tracking, rate limiting, token accounting, response caching
- **SecureYeoman** — 13-provider AI routing, model router, cost budgeting, 9-tier sandbox stack
- **Agnostic v1** — Agent definitions, crew assembly, tool registry, fleet distribution, GPU scheduling, 18 presets across 5 domains

---

## Current Phase: 4 — API Server & Integration

### Phase 1: Core Crate (Foundation) — Complete

Build `agnosai-core` and `agnosai-orchestrator` with essential primitives.

| Item | Source | Status |
|------|--------|--------|
| Core types (Agent, Task, Crew, Message, Resource) | Agnosticos `agnos-common` | Done |
| Error types (`AgnosaiError` via thiserror) | New | Done |
| Orchestrator with `Arc<RwLock<State>>` | Agnosticos `daimon/orchestrator` | Done |
| Priority task scheduler with DAG resolution | Agnosticos `scheduling.rs` + new DAG | Done (9 tests) |
| Agent scoring (tools, complexity, GPU, domain) | Agnosticos `scoring.rs` | Done (11 tests) |
| Topic pub/sub with wildcards | Agnosticos `pubsub.rs` | Done (21 tests) |
| Agent definitions (JSON/YAML loading) | Agnostic v1 format | Done (11 tests) |
| Crew runner (assemble → execute → aggregate) | New, replaces CrewAI Crew | Done (11 tests) |

**Exit criteria**: Define agents in JSON, assemble a crew, execute a task DAG in a single process with native Rust tools. **Met — 63 tests passing.**

---

### Phase 2: LLM & Tools — Complete

| Item | Source | Status |
|------|--------|--------|
| `LlmProvider` trait + OpenAI provider | Agnosticos hoosh + SY model router | Done |
| Anthropic, Ollama providers | SY provider implementations | Done |
| DeepSeek, Mistral, Groq, LM Studio, hoosh providers | OpenAI-compatible wrappers | Done |
| Model router (task-complexity scoring) | SY `model-router.ts` | Done (10 tests) |
| Provider health ring buffer + failover | SY health scoring | Done (8 tests) |
| Response cache (LRU + TTL) | SY + Agnosticos | Done (8 tests) |
| Token budget accounting | Agnosticos hoosh | Done (10 tests) |
| Rate limiter (semaphore-based) | Agnosticos `rate_limiter.rs` | Done (4 tests) |
| Native Rust tool trait + registry | Agnostic `tool_registry.py` | Done (10 tests) |
| Built-in tools: Synapse (3), Mneme (3), Delta (3) | AGNOS ecosystem integration | Done |
| WASM tool sandbox (wasmtime) | Agnosticos `sandbox_mod/` | Done (5 tests) |
| Python tool bridge (sandboxed subprocess) | New | Done (5 tests) |

**Exit criteria**: Run a crew that calls LLMs and executes tools (native, WASM, or sandboxed Python). **Met — 140 tests passing.**

---

### Phase 3: Fleet Distribution — Partial (blocked items deferred)

| Item | Source | Status |
|------|--------|--------|
| IPC (Unix sockets, length-prefixed framing) | Agnosticos `ipc.rs` | Done (4 tests) |
| Node registry + heartbeat | Agnostic v1 fleet → Rust | Done (9 tests) |
| Placement engine (5 scheduling policies) | Agnostic v1 fleet → Rust | Done (9 tests) |
| GPU detection + scheduler | Agnostic v1 gpu → Rust | Done (8 tests) |
| Crew state manager (barrier sync, checkpoints) | Agnostic v1 fleet → Rust | Done (14 tests) |
| Fleet coordinator (fan-out, aggregation, failover) | Agnostic v1 fleet → Rust | Done (13 tests) |
| Inter-node relay (Redis pub/sub) | Agnostic v1 fleet → Rust | Deferred (needs Redis) |
| Federation (multi-cluster) | Agnosticos federation | Deferred (needs hardware) |

**Exit criteria**: Distribute a crew across multiple nodes with lockstep execution, failover, and GPU-aware placement. **Partial — 57 tests passing. Relay + federation deferred.**

---

### Phase 4: API Server & Integration

| Item | Source | Status |
|------|--------|--------|
| axum HTTP server with REST API | Mirrors Agnostic v1 FastAPI | Done (5 tests) |
| MCP server (tool advertisement) | Agnostic v1 MCP routes | Deferred |
| A2A protocol (webhooks) | Agnostic v1 A2A | Deferred |
| SSE streaming for crew execution | New | Deferred |
| JWT auth + token delegation | Agnostic v1 auth | Deferred |
| Preset library (18 presets) | Agnostic v1 presets | Placeholder |
| Crew assembler (team spec → agents) | Agnostic v1 crew assembler | Done (6 tests) |
| Learning & RL module | Agnosticos learning + RL optimizer | Done (35 tests) |
| Definition versioning & .agpkg packaging | Agnostic v1 versioning + packaging | Done (versioning: 5 tests) |

**Exit criteria**: Full API compatibility with Agnostic v1. Drop-in backend replacement.

---

### Phase 5: Agnostic Migration

| Item |
|------|
| Feature flag: `AGNOSTIC_BACKEND=agnosai\|crewai` |
| Port unit tests to run against both backends |
| Port E2E tests (Docker compose with AgnosAI binary) |
| Migrate presets domain-by-domain |
| Port high-value Python tools to native Rust |
| Community tool SDK (WASM) |
| Remove CrewAI dependency |
| Remove Python fleet code |

**Exit criteria**: Agnostic runs entirely on AgnosAI. Zero Python in the hot path. CrewAI removed.

---

## Performance Targets

| Metric | v1 (Python/CrewAI) | AgnosAI Target |
|--------|-------------------|----------------|
| Container image | ~1.5 GB | <50 MB |
| Boot to ready | 15-30s | <2s |
| Memory (idle) | 300-500 MB | <100 MB |
| Crew creation | ~500ms | <10ms |
| Concurrent crews | ~5-10 (GIL) | 100+ |
| Fleet msg overhead | ~50ms | <1ms |
| Dependencies | 200+ | ~30 |
| Python in hot path | 100% | 0% |

---

## Design Principles

See [docs/architecture/overview.md](../architecture/overview.md) for the full integration map and system context.

1. **Concurrency over parallelism hacks** — tokio async, not thread pools with GIL workarounds
2. **Compile-time safety** — Rust type system catches what Python tests miss
3. **Single binary** — no container orchestration needed for single-node deployments
4. **Sandbox by default** — untrusted code never runs unsandboxed
5. **Wire compatibility** — same REST/MCP/A2A API surface as Agnostic v1
6. **Library first** — `agnosai-core` is a dependency, not a framework. Your code calls it, not the other way around.
