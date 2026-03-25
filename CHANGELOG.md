# Changelog

All notable changes to AgnosAI will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.24.3] — 2026-03-24

### Added

#### Features
- **OpenTelemetry tracing spans** (`otel` feature flag): `src/telemetry.rs` with `init_tracing()`, OTLP gRPC export, `TracingGuard`, env var auto-detection (`OTEL_EXPORTER_OTLP_ENDPOINT`, `OTEL_SERVICE_NAME`)
- **`#[tracing::instrument]`** on `Orchestrator::run_crew`, `cancel_crew`, `CrewRunner::run/run_sequential/run_parallel/run_dag`, `execute_task`, `score_agent`, `create_crew`, `a2a::receive`, `mcp_handler`
- **Crew cancellation**: `cancel_crew()` stops running crews via `AtomicBool` token — sequential breaks between tasks, parallel aborts pre-semaphore, DAG halts between waves
- **Cryptographic audit chain**: HMAC-SHA256 tamper-proof event logging via `hoosh::audit::AuditChain` — records `crew_accepted`, `crew_finished`, `crew_cancelled`, `task_completed` with metadata
- **SSRF protection module**: `server::ssrf` shared utilities — `is_safe_url()`, `is_private_ip()`, `is_private_ipv4()` — used by A2A callbacks, `LoadTestingTool`, and `SecurityAuditTool`
- **Configurable parallel concurrency**: `CrewRunRequest.max_concurrency` field (default 4, clamped 1–64)
- Constructors/builders for `ComputeDevice`, `HardwareInventory`, `HardwareRequirement`, `TaskDAG`, `ResourceBudget`, `Experience`, `RelayMessage`, `PlacementRequest`, `TaskProfile`, `TeamMember`, `ToolInput`
- `#[non_exhaustive]` on 48+ public structs across core, server, sandbox, fleet, learning, definitions
- `#[must_use]` on 30+ pure functions across scoring, learning, SSE, pubsub, tools, router
- `#[inline]` on 20+ hot-path accessors

#### Tests (620 total, up from 323)
- SSRF validation: 27 tests (IPv4/IPv6/mapped/localhost/schemes/metadata)
- Crew validation: 8 tests (cycle detection, self-deps, DAG mode)
- Fleet federation: 8 tests (election, roles, eviction)
- Fleet registry: 7 tests (heartbeat, capability search, online filtering)
- Scheduler `topological_sort_tasks`: 7 tests (chain, diamond, cycle, priority)
- Route handlers: SSE (3), tools (3), agents (3), definitions (1)
- WASM tool: 8 tests (manifest serde, output parsing)
- Python tool: 10 tests (JSON protocol, error handling)
- Audit chain: 3 tests (lifecycle, per-task events, cancel event)
- Telemetry: 2 tests

#### Benchmarks (106 across 17 files, up from 85 across 9)
- New: `orchestrator` (5), `llm_router` (10), `definitions` (3), `audit` (7), `server` (6), `sandbox` (4), `ipc` (3), `fleet` (7)
- Extended: `scoring` (+3), `relay` (+2), `tools` (+5)

### Changed

#### Dependencies
- hoosh: 0.22.3 → 0.23.4, now from crates.io (was local path)
- bhava: 0.23.3 → 1.0.0, now from crates.io, **always-on** (no longer optional)
- ai-hwaccel: 0.21.3 → 0.23.3
- wasmtime/wasmtime-wasi: 42 → 43
- `personality` feature flag **removed** — bhava is a required dependency
- Scoring weights permanently personality-aware (tool: 0.35, complexity: 0.25, GPU: 0.10, domain: 0.15, personality: 0.15)
- GPL-3.0 / GPL-3.0-only added to `deny.toml` license allowlist

#### API
- `PubSub::subscribe()` → returns `Option` (capped at 10,000 patterns)
- `Ucb1::select()` / `best_arm()` → return `Option<usize>`
- `SandboxManager::execute()` replaced by `execute_argv()` (no shell interpretation)
- `OrchestratorState` → `pub(crate)` (internal state no longer leaked)
- `AgentDefinition.personality` always compiled (was `#[cfg(feature = "personality")]`)
- MCP server reports `CARGO_PKG_VERSION`
- Domain scoring uses case-insensitive comparison
- Duplicate `topological_sort` in crew_runner eliminated — delegates to `scheduler::topological_sort_tasks()`
- Shared `reqwest::Client` via `OnceLock` in synapse/mneme/delta tools (was per-instance)

### Security
- **Constant-time comparison**: length comparison uses full `usize` (was truncated to `u8`)
- **SSRF hardening**: IPv6 private ranges (fc00::/7, fe80::/10), IPv6-mapped IPv4 (::ffff:x.x.x.x), bracketed IPv6 URL parsing via `url::Host`
- **SSRF on tools**: `LoadTestingTool` and `SecurityAuditTool` reject private/internal target URLs
- **URL path traversal**: mneme `note_id`, delta `owner`/`repo`/`pipeline_id` reject `/` and `..`
- **A2A callback timeout**: 30s limit on fire-and-forget callbacks
- **Process sandbox env sanitization**: `execute()` strips `LD_PRELOAD`/`LD_LIBRARY_PATH`/`DYLD_*`
- **PubSub DoS**: subscription patterns capped at 10,000
- **EventBus DoS**: channel count monitored with orphan cleanup
- **Fleet unbounded growth**: `CrewStateManager` evicts at 1,000 completed runs, `FleetCoordinator` evicts at 10,000 completed tasks
- **Replay buffer**: fixed biased weighted sampling
- **StringInterner**: `checked_add` on u32 ID allocation

### Fixed
- **DAG priority ordering**: sort ascending for `pop()` (was inverted — highest priority processed last)
- **Double inference call**: streaming now emits captured response instead of re-invoking LLM
- **Wrong crew_id in streaming events**: was using task_id
- **JoinError in parallel/DAG**: synthesizes Failed `TaskResult` (was silently dropped)
- **`is_zero` NaN masking**: corrupt cost data now serialized (was silently dropped)
- **`ComputeDevice.memory_available_mb`**: kept in sync on allocate/release (was stale after first alloc)
- **`remove_node` barrier deadlock**: removing a node auto-satisfies pending barriers
- **`declare_coordinator` stale role**: resets all clusters to Follower before setting new coordinator
- **IPC TOCTOU race**: socket removal uses unconditional `remove_file` + ignore NotFound
- **SSE serialization failure**: emits error JSON instead of empty string
- **DAG failure propagation**: failed tasks no longer treated as completed dependencies

### Performance
- `complexity_level` / `parse_complexity` / `domain_score`: zero-alloc via `eq_ignore_ascii_case`
- `format!` → `write!` on system prompt construction, error messages, expected output
- `#[inline]` on scoring hot paths, tool accessors, fleet GPU methods
- `CapabilityScore::recent` bounded to 64 entries
- `PerformanceProfile` records bounded to 10,000 per agent
- DAG priority lookup: O(1) HashMap (was quadratic scan)
- Consolidated `use std::fmt::Write` import in crew_runner

## [0.22.3] — 2026-03-23

### Added
- `personality` feature flag — optional bhava integration for agent personality modeling
- `AgentDefinition.personality` field — attach a `PersonalityProfile` to any agent (feature-gated)
- `with_personality()` builder method on `AgentDefinition`
- `build_system_prompt()` injects personality behavioral disposition into system prompts when personality is set
- Mood-driven temperature adjustment — `mood_adjusted_temperature()` maps creativity/curiosity/precision/risk traits to inference temperature (0.1–1.5)
- Personality-aware agent scoring — `personality_score()` factors trait groups and specific trait levels into task-agent assignment (15% weight)
- Task context fields: `personality_group` and `personality_trait` for personality-based agent selection
- bhava 0.22.3 as optional dependency (15-trait personality system, mood vectors, sentiment analysis)

### Changed
- hoosh dependency updated to 0.22.3 (with sentiment analysis support)
- Scoring weights redistributed when `personality` feature enabled (tool: 0.35, complexity: 0.25, GPU: 0.10, domain: 0.15, personality: 0.15)
- `full` feature now includes `personality`

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
