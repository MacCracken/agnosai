# Changelog

All notable changes to AgnosAI will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.2] — 2026-03-29

### Changed

#### Dependencies
- hoosh: 1.0.0 → 1.1.0

## [1.0.1] — 2026-03-28

### Changed

#### Dependencies
- majra: 1.0.1 → 1.0.2
- rustc-hash: 2.1.1 → 2.1.2 (transitive)
- zerocopy: 0.8.47 → 0.8.48 (transitive)
- zerocopy-derive: 0.8.47 → 0.8.48 (transitive)

## [1.0.0] — 2026-03-27

### Added

#### Security — Prompt Injection & Tool Allow-Lists
- **Prompt injection detection** (`server::prompt_guard`): heuristic scanner for 30+ injection patterns (instruction override, role hijack, prompt leak, delimiter injection) with case-insensitive matching
- **Input sanitization**: `sanitize()` truncates inputs to 50K chars, wraps in `<user_input>` boundary markers, logs warnings on suspicious content
- **System prompt hardening**: `wrap_system_prompt()` adds `<system_instructions>` delimiters and anti-injection directive to all LLM system prompts
- **Per-agent tool allow-list enforcement**: `ToolRegistry::get_allowed()` validates tool calls against agent's `tools` field before execution; empty list means "all tools"
- `ToolRegistry::is_tool_allowed()` static helper for allow-list checks

#### Structured Output Validation
- **Output validation module** (`orchestrator::output_validation`): validates LLM responses against JSON Schema (`type` and `required` field checks)
- **Retry-on-parse-failure**: when `Task.output_schema` is set, failed validation triggers up to 2 retries with error feedback injected into the prompt and temperature forced to 0.1
- **Markdown fence extraction**: `extract_and_validate()` automatically extracts JSON from ````json` code blocks in LLM responses
- `Task.output_schema` field — optional JSON Schema for output validation with retry

#### Human-in-the-Loop Approval Gates
- **Approval gate module** (`orchestrator::approval`): suspends crew runner via oneshot channels, resumes on HTTP callback
- `ApprovalGate` with configurable timeout (default 5 min), max 1,000 pending approvals, capacity enforcement
- `TaskRisk` enum (Low/Medium/High) on `Task` — determines whether human approval is required
- `ApprovalGate::requires_approval()` — configurable per-risk-level gating
- **REST endpoints**: `POST /api/v1/approvals` (submit decision), `GET /api/v1/approvals` (list pending)
- `ApprovalDecision` enum (Approved/Rejected) with serde support
- `AppState.approval_gate` — shared approval gate accessible from all route handlers

#### Kavach Integration (`kavach` feature flag)
- **`sandbox::kavach_bridge`** — bridge module mapping AgnosAI sandbox policies to kavach sandboxes
- `map_backend()` — maps `IsolationLevel` (None/Wasm/Process/Oci) to kavach `Backend`
- `build_config()` — converts `SandboxPolicy` to kavach `SandboxConfig` with externalization gate, seccomp, and agent ID
- `strength_for_policy()` — computes kavach `StrengthScore` (0–100) for any sandbox policy
- `execute()` — full lifecycle: create → start → exec → stop → destroy, with tracing and security metadata
- `scan_output()` — standalone externalization gate for scanning native tool outputs (secrets, code violations, PII)
- `policy_for_trust()` — maps crew trust levels ("minimal"/"strict"/"basic") to kavach `ExternalizationPolicy` presets
- `KavachToolResult` — result struct carrying output, exit code, strength score, and scan verdict
- kavach 1.0.1 as optional dependency with `process` feature (seccomp, Landlock, credential scanning)
- **Sandbox strength in crew metadata**: `CrewProfile.sandbox_strength` — kavach strength score (0–100) carried in crew execution results
- **Per-crew isolation policy**: `CrewSpec.trust_level` ("minimal"/"basic"/"strict") — controls externalization gate thresholds via `policy_for_trust()`

#### Resilience & Context (P1)
- **LLM inference retry with exponential backoff** (`llm::retry`): configurable `RetryConfig` (max retries, base delay, max delay, jitter), `with_retry()` async wrapper, `is_retryable()` heuristic for transient errors (rate limits, 503s, timeouts, connection resets), wired into crew runner inference path
- **Token/cost budget enforcement** (`orchestrator::budget`): `BudgetTracker` with atomic counters, `check()` validates before inference, `record_tokens()`/`record_cost()` after, `BudgetExceeded` error enum (Tokens/Cost variants)
- **Multi-turn conversation memory** (`orchestrator::memory`): `ConversationBuffer` with three strategies — `Full` (unlimited), `SlidingWindow` (evict oldest), `HeadTail` (keep first + last N), per-agent context accumulation
- **OTel GenAI semantic convention spans** (`telemetry::genai`): `inference_span()`, `tool_span()`, `crew_span()` helpers emitting standardized attributes per OTel v1.37 (`gen_ai.operation.name`, `gen_ai.agent.name`, `gen_ai.usage.input_tokens`, `gen_ai.response.model`, etc.)
- **Per-task cost attribution**: `CrewProfile.task_cost_usd` (per-task) and `CrewProfile.agent_cost_usd` (per-agent) cost breakdowns populated from TaskResult metadata

#### Majra Integration (`majra` feature flag)
- **Priority inference queue** (`llm::inference_queue`): `InferenceQueue` backed by majra's `ConcurrentPriorityQueue` — enqueue inference requests at 5 priority tiers (Critical→Background), async worker loop dispatches in priority order, oneshot reply channels
- `map_priority()` — maps AgnosAI `TaskPriority` to majra `Priority`
- `InferenceQueue::spawn_worker()` — background task that pops and executes queued inference requests
- **Per-endpoint rate limiting** (`server::rate_limit`): `RateLimitState` backed by majra's token bucket `RateLimiter` — per-IP rate limiting with X-Forwarded-For/X-Real-IP extraction, stale key eviction, HTTP 429 middleware
- majra 1.0.1 as optional dependency with `queue`, `ratelimit`, `pubsub` features

#### Medium-Priority Batch — Ecosystem, Observability, Durability
- **Topology-aware fleet scheduling** (`fleet::topology`): `NodeTopology`, `DeviceLink`, `InterconnectType` (PCIe/NVLink/XGMI/CXL), `topology_score()` for multi-GPU placement, `supports_tensor_parallel()` check
- **Cost-aware crew planning** (`fleet::cost_planning`): `GpuPricing` with per-hour rates for 5 GPU types, `estimate_crew_cost()`, `select_cheapest_model()` budget-constrained selection
- **Container/VM environment detection** (`fleet::environment`): `RuntimeEnvironment` enum (Bare/Container/Vm/Kubernetes/Unknown), `detect()` via cgroup/hypervisor/env inspection, `resource_limits()` from cgroup v1/v2
- **Multi-node fleet discovery** (`fleet::discovery`): `DiscoveryBackend` trait, `StaticDiscovery` impl, `DnsDiscovery` stub for DNS SRV
- **Prometheus metrics** (`server::prometheus`): `AgnosMetrics` with atomic counters (crews, tasks, tokens, cost), `gather()` in Prometheus exposition format
- **Multi-tenancy** (`orchestrator::multi_tenant`): `TenantRegistry` with DashMap, `TenantBudget`, per-tenant budget checking
- **Durable crew state** (`orchestrator::durable_state`): `StateStore` trait, `FileStateStore` impl (JSON to disk), `serialize_crew_state()` / `deserialize_crew_state()`
- **Hierarchical process mode** (`orchestrator::hierarchical`): `delegate_tasks()` using scoring module to assign tasks to best-fit agents, replaces sequential fallback
- **Sensitive information output filter** (`server::output_filter`): `OutputFilter` scanning for system prompt leakage, API keys (AWS/GitHub/Bearer), PII (email/phone/SSN), `scan()` + `redact()`

#### Low-Priority Batch
- **Hot-reload tool registration**: `DELETE /api/v1/tools/{name}` for runtime tool unregistration
- **Dashboard API**: `GET /api/v1/dashboard/crews` (crew history summaries), `GET /api/v1/dashboard/agents` (agent performance from recent runs)
- **Remote WASM tool registry** (`tools::remote_registry`): `fetch_package()` downloads tool packages from URL with SSRF protection and 10 MB size limit
- **Hot-reload configuration** (`server::hot_config`): `ConfigHolder<T>` backed by `tokio::sync::watch` — zero-contention reads, instant propagation, `RuntimeConfig` struct with reloadable settings
- **Plan caching** (`orchestrator::plan_cache`): `PlanCache` with LRU eviction (256 max), TTL expiry, order-independent crew spec hashing via `PlanKey`
- **Kubernetes CRD types** (`definitions::k8s_crd`): `CrewCrd`, `AgentCrdSpec`, `TaskCrdSpec` with `agnosai.io/v1` API group — serde-compatible, no k8s client dependency

#### Infrastructure
- **Graceful shutdown** in `main.rs` — handles SIGTERM and SIGINT via `tokio::signal`, logs shutdown reason
- **`scripts/bench-history.sh`** — runs all benchmarks and appends median times to `bench-history.csv`

### Changed
- `main.rs`: HTTP client build uses `?` instead of `.expect()` (no longer panics on TLS init failure)
- Crew runner: system prompts wrapped with anti-injection boundaries via `prompt_guard::wrap_system_prompt()`
- Crew runner: task descriptions and context values sanitized via `prompt_guard::sanitize()` before LLM submission
- Crew runner: output validation retry loop when `Task.output_schema` is set

#### `#[must_use]` additions (26 methods)
- `fleet::registry`: `get`, `list`, `list_online`, `count`, `count_online`, `find_by_capability`
- `fleet::placement`: `place`, `rank_nodes`
- `fleet::gpu`: `compute_devices`, `devices`, `devices_of_type`, `total_memory_mb`, `available_memory_mb`, `total_vram_mb`, `available_vram_mb`, `best_device`, `allocations`, `vram_available_mb`
- `fleet::state`: `get`, `active_runs`, `overall_progress`
- `fleet::coordinator`: `tasks_for_node`, `is_complete`, `completion_pct`, `pending_reassignment`, `state_manager`
- `definitions::versioning`: `get`, `latest`, `list_versions`

### Security
- **Prompt injection defence-in-depth**: boundary markers, anti-injection directives, heuristic scanning
- **Tool call allow-list**: prevents LLM from invoking tools outside agent's declared tool set
- **VersionStore bounded growth**: capped at 500 versions per agent with oldest-first eviction
- **Load testing tool request cap**: total requests capped at 100K across all concurrent users

### Fixed
- **Output validation retry prompt injection**: failed LLM outputs are now sanitized via `prompt_guard::sanitize()` before inclusion in retry prompts
- **Output validation retry prompt growth**: retry prompts now use the original prompt as base, preventing exponential accumulation
- **JSON fence extraction**: closing delimiter now requires newline prefix (`\n`````), preventing false termination on literal triple backticks inside JSON string values
- `#[non_exhaustive]` added to `WasmToolManifest`

### Performance
- **`rank_agents` (10 agents)**: 2.95 µs → 870 ns (−71%) — pre-extract `required_tools` once per task instead of re-deserializing per agent
- **Crew cancel/update**: O(n) → O(1) — `active_crews` changed from `Vec<CrewState>` to `HashMap<CrewId, CrewState>`
- **DAG topological sort**: O(n² log n) → O(n log n) — replaced Vec + sort() with BinaryHeap for priority ordering
- **`scan_input` prompt guard**: zero-alloc — replaced `to_ascii_lowercase()` with `eq_ignore_ascii_case` byte-window search
- **`rank_agents` scoring loop**: single `extract_required_tools()` call shared across all agents (was N calls)
- **Server endpoints**: GET /health −43%, POST /mcp −40%, EchoTool −37%

### Observability
- `llm::router::route()` — `tracing::debug` on model tier routing decisions
- `tools::builtin::load_testing` — `tracing::info` on test start/completion with metrics
- `learning::profile` — `tracing::debug` on action recording
- `learning::optimizer` — `tracing::debug` on Q-value updates
- `learning::capability` — `tracing::debug` on capability success/failure with confidence and trend

### Tests (823 total, up from 620)
- Prompt guard: 12 tests (injection patterns, sanitization, boundary wrapping)
- Output validation: 12 tests (JSON parsing, type checks, required fields, fence extraction, retry prompts, backtick edge case)
- Approval gate: 7 tests (approve/reject flow, timeout, capacity, cancel, listing)
- Tool allow-list: 5 tests (empty list, allow/block, missing tool)
- Kavach bridge: 16 tests (backend mapping, strength scoring, config building, externalization gate, trust policies)
- Crew trust/strength: 5 tests (default trust, custom trust, strength serialization, serde roundtrip)
- LLM retry: 11 tests (exponential backoff, retryable detection, transient recovery, exhaustion, non-retryable skip)
- Budget tracker: 6 tests (token/cost enforcement, accumulation, display)
- Conversation memory: 8 tests (full/sliding/head-tail strategies, clear, serde)
- GenAI spans: 4 tests (attribute naming, span creation)
- Inference queue: 4 tests (creation, priority mapping, enqueue, background)
- Rate limiter: 7 tests (burst, separate keys, stats, eviction, header extraction)
- Topology scheduling: 7 tests (single GPU, no links, full NVLink, partial, tensor parallel)
- Cost planning: 8 tests (estimation, model selection, budget constraints)
- Environment detection: 7 tests (enum variants, resource limits, detection)
- Fleet discovery: 7 tests (static backend, discovered node, trait impl)
- Prometheus metrics: 7 tests (counters, gauges, gather format)
- Multi-tenancy: 11 tests (registration, budget check, concurrent access)
- Durable state: 7 tests (serialize/deserialize, file store, tempdir)
- Hierarchical mode: 6 tests (task delegation, scoring-based assignment)
- Output filter: 16 tests (API keys, PII, system prompt leak, redaction)
- Plan cache: 7 tests (insert/get, order-independent hashing, TTL expiry, LRU eviction)
- K8s CRD types: 4 tests (serde roundtrip, YAML compat, defaults, API constants)
- Hot config: 5 tests (initial value, update, receiver, runtime config defaults, serde)
- Remote registry: 3 tests (SSRF rejection, localhost rejection, size constant)
- VersionStore eviction: 1 test

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
