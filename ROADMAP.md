# AgnosAI Roadmap

> Rust-native agent orchestration — replacing CrewAI with a purpose-built framework.

AgnosAI distills production-proven patterns from three systems:

- **Agnosticos (daimon)** — Orchestrator, IPC, pub/sub, scoring, scheduling, resource management, RL optimizer, federation (8997+ tests)
- **Agnosticos (hoosh)** — LLM provider abstraction, health tracking, rate limiting, token accounting, response caching
- **SecureYeoman** — 13-provider AI routing, model router, cost budgeting, 9-tier sandbox stack
- **Agnostic v1** — Agent definitions, crew assembly, tool registry, fleet distribution, GPU scheduling, 18 presets across 5 domains

---

## Current Phase: 1 — Core Foundation

### Phase 1: Core Crate (Foundation)

Build `agnosai-core` and `agnosai-orchestrator` with essential primitives.

| Item | Source | Status |
|------|--------|--------|
| Core types (Agent, Task, Crew, Message, Resource) | Agnosticos `agnos-common` | Scaffolded |
| Error types (`AgnosaiError` via thiserror) | New | Scaffolded |
| Orchestrator with `Arc<RwLock<State>>` | Agnosticos `daimon/orchestrator` | Scaffolded |
| Priority task scheduler with DAG resolution | Agnosticos `scheduling.rs` + new DAG | Pending |
| Agent scoring (CPU, GPU, capability, affinity) | Agnosticos `scoring.rs` | Pending |
| IPC (Unix sockets, length-prefixed framing) | Agnosticos `ipc.rs` | Pending |
| Topic pub/sub with wildcards | Agnosticos `pubsub.rs` | Pending |
| Agent definitions (JSON/YAML loading) | Agnostic v1 format | Pending |
| Crew runner (assemble → execute → aggregate) | New, replaces CrewAI Crew | Pending |

**Exit criteria**: Define agents in JSON, assemble a crew, execute a task DAG in a single process with native Rust tools.

---

### Phase 2: LLM & Tools

| Item | Source |
|------|--------|
| `LlmProvider` trait + OpenAI provider | Agnosticos hoosh + SY model router |
| Anthropic, Ollama, Gemini providers | SY provider implementations |
| Remaining providers (DeepSeek, Mistral, Groq, LM Studio, hoosh) | SY + Agnosticos |
| Model router (task-complexity scoring) | SY `model-router.ts` |
| Provider health ring buffer + failover | SY health scoring |
| Response cache (LRU + TTL) | SY + Agnosticos |
| Token budget accounting | Agnosticos hoosh |
| Rate limiter (semaphore-based) | Agnosticos `rate_limiter.rs` |
| Native Rust tool trait + registry | Agnostic `tool_registry.py` |
| WASM tool sandbox (wasmtime) | Agnosticos `sandbox_mod/` |
| Python tool bridge (sandboxed subprocess) | New |

**Exit criteria**: Run a crew that calls LLMs and executes tools (native, WASM, or sandboxed Python).

---

### Phase 3: Fleet Distribution

| Item | Source |
|------|--------|
| Node registry + heartbeat | Agnostic v1 fleet → Rust |
| Placement engine (5 scheduling policies) | Agnostic v1 fleet → Rust |
| Inter-node relay (Redis pub/sub) | Agnostic v1 fleet → Rust |
| Fleet coordinator (fan-out, aggregation, failover) | Agnostic v1 fleet → Rust |
| Crew state manager (barrier sync, checkpoints) | Agnostic v1 fleet → Rust |
| GPU detection + scheduler | Agnostic v1 gpu → Rust |
| Federation (multi-cluster) | Agnosticos federation |

**Exit criteria**: Distribute a crew across multiple nodes with lockstep execution, failover, and GPU-aware placement.

---

### Phase 4: API Server & Integration

| Item | Source |
|------|--------|
| axum HTTP server with REST API | Mirrors Agnostic v1 FastAPI |
| MCP server (tool advertisement) | Agnostic v1 MCP routes |
| A2A protocol (webhooks) | Agnostic v1 A2A |
| SSE streaming for crew execution | New |
| JWT auth + token delegation | Agnostic v1 auth |
| Preset library (18 presets) | Agnostic v1 presets |
| Crew assembler (team spec → agents) | Agnostic v1 crew assembler |
| Learning & RL module | Agnosticos learning + RL optimizer |
| Definition versioning & .agpkg packaging | Agnostic v1 versioning + packaging |

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

1. **Concurrency over parallelism hacks** — tokio async, not thread pools with GIL workarounds
2. **Compile-time safety** — Rust type system catches what Python tests miss
3. **Single binary** — no container orchestration needed for single-node deployments
4. **Sandbox by default** — untrusted code never runs unsandboxed
5. **Wire compatibility** — same REST/MCP/A2A API surface as Agnostic v1
6. **Library first** — `agnosai-core` is a dependency, not a framework. Your code calls it, not the other way around.
