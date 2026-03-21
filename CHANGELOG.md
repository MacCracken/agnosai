# Changelog

All notable changes to AgnosAI will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.21.3] — 2026-03-21

### Added
- Lazy LLM provider initialisation — `HooshClient` created on first inference via `OnceLock`, not at server startup
- Crew execution profiling — `CrewProfile` on every `CrewState` with wall time and per-task `task_duration_ms` metadata
- Inference response caching — hoosh `ResponseCache` (TTL + LRU eviction) wired into `execute_task`, shared across crews
- Dockerfile (multi-stage build, `rust:1.89-bookworm` builder, `debian:bookworm-slim` runtime)
- `strip_provider_prefix()` — normalises LiteLLM-style `provider/model` identifiers for inference

### Changed
- hoosh dependency updated from 0.20 to 0.21.3
- `Orchestrator::with_llm_url()` replaces eager `with_llm(Arc<HooshClient>)` as primary init path
- Server startup no longer creates LLM client — deferred to first crew execution

## [0.20.3] — 2026-03-18

### Added

#### Core (`agnosai-core`)
- `AgentDefinition` with JSON/YAML deserialization, default complexity, GPU fields
- `Task` with priority (5-tier), status lifecycle, dependency tracking, context map
- `TaskDAG` with `ProcessMode` variants: Sequential, Parallel, DAG, Hierarchical
- `CrewSpec` and `CrewState` for crew lifecycle management
- `Message` with topic-based targeting (Agent, Topic, Broadcast)
- `ResourceBudget` with token, cost, duration, and concurrency limits
- `GpuDevice` type for VRAM tracking
- `AgnosaiError` enum (13 variants) with `thiserror` derives and `From` impls

#### Orchestrator (`agnosai-orchestrator`)
- `Orchestrator` with `Arc<RwLock<State>>`, delegates to `CrewRunner`
- `Scheduler` — priority queue (5-tier VecDeque) + DAG topological sort (Kahn's algorithm) with cycle detection
- `CrewRunner` — full crew lifecycle: Sequential, Parallel (semaphore-bounded), DAG (wave execution), Hierarchical (sequential fallback)
- Agent scoring — 4-factor weighted (tool coverage 0.40, complexity 0.30, GPU 0.15, domain 0.15)
- Topic pub/sub with wildcard matching (`*` one segment, `#` zero or more), broadcast channels, DashMap
- IPC — Unix socket server/client with length-prefixed framing (4-byte BE u32 + JSON payload, 16 MiB max)

#### LLM (`agnosai-llm`)
- `LlmProvider` trait with `infer()` and `list_models()`
- 8 providers: OpenAI, Anthropic (direct HTTP), Ollama, DeepSeek, Mistral, Groq, LM Studio, hoosh
- OpenAI-compatible providers delegate to `OpenAiProvider` via newtype pattern
- Model router — task-complexity scoring across 7 task types × 3 complexity levels → Fast/Capable/Premium
- `ProviderHealth` — 5-point ring buffer, 3 consecutive failures → unhealthy, one success resets
- `ResponseCache` — LRU with TTL expiration, deterministic cache keys
- `TokenBudget` — per-agent + global token accounting with `BudgetExceeded` errors
- `RateLimiter` — semaphore-based concurrent request limiting

#### Tools (`agnosai-tools`)
- `NativeTool` trait (object-safe with `Pin<Box<dyn Future>>`)
- `ToolRegistry` — thread-safe DashMap-backed tool storage
- Built-in tools: `EchoTool`, `JsonTransformTool`
- AGNOS ecosystem tools (optional HTTP clients, not hard dependencies):
  - Synapse: `synapse_infer`, `synapse_list_models`, `synapse_status`
  - Mneme: `mneme_search`, `mneme_get_note`, `mneme_create_note`
  - Delta: `delta_list_repos`, `delta_trigger_pipeline`, `delta_get_pipeline`

#### Sandbox (`agnosai-sandbox`)
- WASM sandbox via wasmtime — WASI preview 1, fuel-based CPU limits, epoch interruption for timeouts, memory caps, no filesystem/network
- Python subprocess bridge — stdin/stdout JSON protocol, `tokio::time::timeout`, kill-on-drop, tool wrapper script generation

#### Fleet (`agnosai-fleet`)
- `NodeRegistry` — node inventory with heartbeat TTL, status transitions (Online → Suspect → Offline), capability search
- `PlacementEngine` — 5 scheduling policies: GpuAffinity, Balanced, Locality, Cost, Manual
- `GpuScheduler` — device management, VRAM tracking, best-fit allocation/release
- `CrewStateManager` — distributed crew phases, barrier sync (per-node arrival tracking), named checkpoints, progress aggregation
- `FleetCoordinator` — task fan-out with node assignments, completion/failure tracking, retry with configurable max, reassignment

#### Learning (`agnosai-learning`)
- `PerformanceProfile` — per-agent action recording, success rates, duration averages
- `Ucb1` — multi-armed bandit strategy selection with UCB1 formula
- `ReplayBuffer` — prioritized experience replay with weighted sampling, lowest-priority eviction
- `CapabilityScorer` — dynamic confidence scoring with trend detection (Improving/Stable/Declining)
- `QLearner` — tabular Q-learning with configurable learning rate and discount factor

#### Definitions (`agnosai-definitions`)
- JSON/YAML loader — `load_from_file` (auto-detect), `load_all_from_dir`
- `assemble_team` — match `TeamMember` specs to agent definitions via role/tool/complexity scoring
- `VersionStore` — definition versioning with auto-incrementing version numbers, rollback

#### Server (`agnosai-server`)
- axum HTTP server with health/ready probes
- `POST /api/v1/crews` — create and execute a crew (index-based dependency mapping)
- `GET /api/v1/tools` — list registered tools
- Agent definition and preset placeholder endpoints
- `AppState` with shared `Orchestrator` + `ToolRegistry`

#### Documentation
- ADRs: separate repo, ecosystem tools, native HTTP providers, concurrency model
- Architecture overview with system context diagram and crate dependency graph
- Developer guides: getting started, adding providers, adding tools
- Contributing guide with commit conventions

#### Project
- Single-crate layout (edition 2024, MSRV 1.89)
- `rust-toolchain.toml` (stable channel)
- Release profile: `lto = "fat"`, `strip = true`, `panic = "abort"`, `codegen-units = 1`
- AGPL-3.0-only license
- 323 tests passing

#### Server — MCP, A2A, SSE, Auth
- MCP server (JSON-RPC 2.0 over HTTP POST): `initialize`, `tools/list`, `tools/call` with ToolRegistry integration
- A2A protocol: `POST /api/v1/a2a/receive` — webhook-based crew delegation with optional callback URL
- SSE streaming: `GET /api/v1/crews/:id/stream` — event stream endpoint with `CrewEvent` types
- Auth middleware: shared-secret Bearer token validation, configurable enable/disable

#### Definitions — Presets & Packaging
- 18 built-in presets (6 domains × 3 sizes): quality, software-engineering, devops, data-engineering, design, security
- `PresetSpec` type with `builtin_presets()`, `load_preset_from_json()`, `load_preset_from_file()`, `load_all_presets()`
- `.agpkg` ZIP packaging with decompression bomb protection (1 MiB per file, 100 entries max, path traversal rejection)
- `GET /api/v1/presets` returns built-in presets (feature-gated on `definitions`)

#### Auth — Full JWT (RS256)
- RS256 JWT validation with configurable issuer, audience, and expiry
- Constant-time shared-secret comparison (no timing or length leaks)
- Defense-in-depth: explicit `exp` claim requirement after decode
- Environment configuration: `AGNOSAI_AUTH_ENABLED`, `AGNOSAI_AUTH_SECRET`, `AGNOSAI_JWT_PUBLIC_KEY`
- `AuthConfig::with_secret()`, `AuthConfig::with_jwt()`, `JwtConfig::new()` builder methods

#### SSE — Full CrewRunner Integration
- `EventBus` with per-crew broadcast channels, lazy creation, orphan cleanup
- `CrewRunner` emits `crew_started`, `task_started`, `task_completed`, `crew_completed` events
- `Orchestrator` wires event bus to runners, cleans up channels on completion
- SSE endpoint handles lagged receivers (warns + notifies client)
- Unknown crew IDs return error event instead of leaking EventBus channels

#### Tools — Ported from Python + Community SDK
- `LoadTestingTool` — concurrent HTTP load generation with p50/p95/p99 latency, throughput, status codes
- `SecurityAuditTool` — HTTP header analysis, CORS detection, information disclosure, security scoring
- `agnosai-tool-sdk` crate for building WASM tools (ToolInput, ToolResult, run_tool)
- `wasm_loader` — load manifest.json + .wasm tool packages from directories
- Example WASM tool: `examples/wasm-tools/hello-tool/`

#### Agnostic Migration (Phase 5)
- Backend abstraction: `agents/backend/` package with `CrewBackend` trait, `CrewAIBackend`, `AgnosAIBackend`
- Feature flag: `AGNOSTIC_BACKEND=agnosai|crewai` routes crew execution
- Fleet shim: delegates fleet operations to AgnosAI via HTTP when backend is `agnosai`
- Docker Compose: `agnosai-server` service with `agnosai` and `e2e` profiles
- Dual-backend test infrastructure (unit + E2E)

### Changed
- License corrected from Apache-2.0 to AGPL-3.0-only in README
- Architecture diagram updated from fake workspace to actual single-crate structure
- Quick start commands and usage imports corrected
- `CONTRIBUTING.md` rewritten for actual single-crate structure
- Configurable server port via `PORT` / `AGNOSAI_PORT` env vars (was hardcoded 8080)

### Security
- Constant-time auth comparison (prevents timing attacks on shared secret)
- JWT expiry enforcement (defense-in-depth, rejects tokens without `exp` claim)
- `#[serde(deny_unknown_fields)]` on all API input types (TaskRequest, CrewRunRequest, A2ARequest, JsonRpcRequest)
- ZIP bomb protection in definition packaging (size limits, entry count, path traversal)
- NaN panic fix in replay buffer (`partial_cmp` with fallback)
- Load test duration clamped to 1-300 seconds (prevents division by zero)
- Environment sanitization: `LD_PRELOAD`, `LD_LIBRARY_PATH`, `DYLD_*` stripped from all sandboxed subprocesses
- Unbounded `active_crews` growth fixed (capped at 1000 with eviction)
- Concurrent crew limit enforced via semaphore (from `ResourceBudget.max_concurrent_tasks`)
- Crew execution timeout via `tokio::time::timeout` (from `ResourceBudget.max_duration_secs`)
- PubSub recursion depth limit (MAX_MATCH_DEPTH=32) prevents stack overflow
- IPC zero-length frame rejection
- IPC EOF vs truncated frame distinction in error messages
- Fleet barrier deadlock prevention (`force_barrier()`, `remove_node()`)
- Fleet checkpoint phase isolation (`is_checkpointing` flag)
- Fleet relay poisoned mutex recovery (resets seen-map instead of using corrupted data)
- A2A field validation (string length 10k, metadata 64 KiB)
- A2A DNS rebinding protection (blocks `.local`, `.internal`, `.localhost` suffixes)
- A2A shared HTTP client with 30s timeout (was creating new client per callback)
- Request concurrency limit (100 concurrent requests via tower)
- Scoring penalizes malformed `required_tools` context (0.5 instead of 1.0)
- Crew dependency cycle detection at API level (DFS before orchestrator)
- Error leakage prevention (internal errors logged at ERROR, generic message returned to client)

### Infrastructure
- `.gitignore`: `**/target/` (catches SDK/example build dirs), `.claude/`
- `SECURITY.md`, `CODE_OF_CONDUCT.md` added
- `deny.toml` for cargo-deny (license allowlist, ban wildcards, crates.io only)
- `supply-chain/config.toml` for cargo-vet (imports Mozilla audits)
- `docs/development/threat-model.md` — 6 attack surfaces mapped
- `docs/guides/adding-wasm-tools.md` — community SDK guide
- Fuzz testing: 4 targets (agent_definition, crew_request, preset_json, tool_input)
- CI: cargo-vet job, coverage job (≥55% gate), benchmark job, fuzz job (5 min), feature matrix testing, MSRV validation
- Makefile: `audit`, `deny`, `vet`, `bench`, `fuzz`, `coverage` targets
- `Cargo.toml`: `homepage`, `documentation`, `keywords`, `categories`, `exclude` fields
- Comprehensive structured logging (auth, orchestrator, crews, MCP, A2A)
- `#[must_use]` on `Result<T>` type alias
- Doc comments on all public types, enums, type aliases, and struct fields
