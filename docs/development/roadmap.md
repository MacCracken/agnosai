# AgnosAI Roadmap

> Rust-native agent orchestration — replacing CrewAI with a purpose-built framework.

For completed work, see [CHANGELOG.md](../../CHANGELOG.md).
For architecture and integration context, see [docs/architecture/overview.md](../architecture/overview.md).

---

## Remaining Work

### Test Coverage (ongoing)

| Milestone | Target | Current |
|-----------|--------|---------|
| CI gate (blocking) | ≥55% | ~70% |
| Near-term | ≥75% | — |
| Target | ≥85% | — |

658 tests, 106 benchmarks across 17 files. Key remaining gaps: HTTP tool execute
paths (load_testing, security_audit need mock servers), SSE streaming edge cases,
telemetry OTLP init paths, adversarial input tests (prompt injection), sandbox
escape tests, concurrent cancel stress tests.

### Ecosystem & Scale (v0.23+)

| Item | Priority | Notes |
|------|----------|-------|
| Topology-aware fleet scheduling | Medium | Leverage ai-hwaccel NVLink/XGMI data for GPU-affinity placement |
| Cost-aware crew planning | Medium | Cloud GPU pricing lookup for budget-constrained crews |
| Container/VM environment detection | Medium | Auto-detect cgroup/namespace resource limits via ai-hwaccel `RuntimeEnvironment` |
| Python bindings (PyO3) | Medium | Let Python callers use AgnosAI as a library |
| Multi-node fleet discovery (mDNS, Consul, K8s) | Medium | Auto-discover fleet nodes |
| Kubernetes operator | Low | CRDs for crew/agent definitions |
| WASM tool registry (remote fetch) | Low | Download community tools from a registry |
| Hot-reload tool registration | Low | Register/unregister tools without restart |

### ~~Kavach Integration (Sandboxed Execution)~~ ✓ Complete

All four items shipped in the `kavach` feature flag:
- ✓ Sandboxed crew execution via kavach (`kavach_bridge::execute()`)
- ✓ Externalization gate on tool outputs (`kavach_bridge::scan_output()`)
- ✓ Sandbox strength in crew metadata (`CrewProfile.sandbox_strength`)
- ✓ Per-crew isolation policy (`CrewSpec.trust_level` → `policy_for_trust()`)

### Observability & Operations

| Item | Priority | Notes |
|------|----------|-------|
| AgnosAI-specific Prometheus metrics | Medium | Crew execution counts, task durations, agent scoring histograms |
| Multi-tenancy (crew isolation, resource quotas) | Medium | Per-tenant budget enforcement |
| Dashboard API (crew history, agent performance) | Low | REST endpoints for operational dashboards |

### Resilience & Context (P1)

| Item | Priority | Notes |
|------|----------|-------|
| LLM inference retry with exponential backoff | High | Retry on transient failures (rate limits, 503s, timeouts) at the HooshClient call path |
| Token/cost budget enforcement per task | High | Enforce `ResourceBudget.max_tokens_per_task` / `max_cost_per_crew` in crew runner |
| Multi-turn conversation memory | High | `ConversationBuffer` per agent — full, sliding window, or summarize-and-compress strategies |
| OTel GenAI semantic convention spans | High | Emit `gen_ai.operation.name`, `gen_ai.agent.name`, `gen_ai.usage.*` per OpenTelemetry v1.37 |
| Per-task cost attribution metrics | Medium | Break down `CrewProfile.cost_usd` per-task and per-agent for optimization |

### Durability & Advanced Modes (P2)

| Item | Priority | Notes |
|------|----------|-------|
| Durable crew state / resume-from-checkpoint | Medium | Serialize crew state to disk/database with resume capability after crash |
| Hierarchical process mode | Medium | Manager agent dynamically delegates sub-tasks to sub-agents (currently falls back to sequential) |
| Crew state HashMap instead of Vec linear scan | Medium | `OrchestratorState.active_crews` → `HashMap<CrewId, CrewState>` for O(1) lookup |
| Sensitive information output filter | Medium | Post-inference filter scanning for system prompt leaks, API keys, PII patterns |
| Plan caching for repeated similar crews | Low | Semantic similarity cache for agent assignment decisions and task decomposition plans |

### Engineering Backlog

| Item | Priority | Notes |
|------|----------|-------|
| Per-endpoint rate limiting | Low | Sliding-window RPM via `hoosh::middleware::rate_limit` |
| Priority inference queue | Low | Batch/background inference via `hoosh::queue` + majra |
| Hot-reload configuration | Low | `arc-swap` pattern for config changes without restart |

### Final Migration

| Item | Priority |
|------|----------|
| Remove CrewAI dependency from Agnostic | Final |

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
6. **Library first** — `agnosai` is a library with feature-gated modules, not a framework
7. **Lockstep with ai-hwaccel** — aligned versioning, shared practices, same CI rigor
