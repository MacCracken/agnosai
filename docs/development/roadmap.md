# AgnosAI Roadmap

> Rust-native agent orchestration — replacing CrewAI with a purpose-built framework.

For completed work, see [CHANGELOG.md](../../CHANGELOG.md).
For architecture and integration context, see [docs/architecture/overview.md](../architecture/overview.md).

---

## Completed

### Code Audit & Review (P0)

- [x] Security audit — input validation, sandbox escape paths, auth bypass, injection vectors
- [x] Error handling — panic paths, unwrap usage, error propagation completeness
- [x] Concurrency — lock ordering, deadlock potential, race conditions, Send/Sync correctness
- [x] API surface — public API consistency, breaking change risk, documentation accuracy
- [x] Test coverage — gap analysis, edge cases, failure path testing
- [x] Dependency audit — supply chain, minimal surface, version currency, advisory compliance
- [x] Performance — unnecessary allocations, hot path efficiency, memory layout
- [x] Code quality — dead code, naming consistency, module organization

### API & Protocol Work

- [x] Full JWT validation (RS256, claims, expiry)
- [x] SSE integration with CrewRunner events
- [x] All 18 presets (6 domains x 3 sizes)

### Agnostic Migration (Phase 5)

- [x] Feature flag: `AGNOSTIC_BACKEND=agnosai|crewai`
- [x] Port unit tests to run against both backends
- [x] Port E2E tests (Docker compose with AgnosAI binary)
- [x] Migrate presets domain-by-domain (18 presets)
- [x] Port high-value Python tools to native Rust (load_testing, security_audit)
- [x] Fleet shim (Python fleet → AgnosAI fleet via HTTP)
- [x] Community tool SDK (WASM)

### Hardening (ai-hwaccel parity)

- [x] Fuzz testing (4 targets: agent definitions, crew requests, presets, tool input)
- [x] cargo-vet supply chain auditing
- [x] `#[serde(deny_unknown_fields)]` on API input types
- [x] Threat model document
- [x] Benchmark CI job
- [x] Environment sanitization (LD_PRELOAD/DYLD stripping in all sandboxes)

---

## Remaining Work

### Test Coverage (ongoing)

| Milestone | Target | Current |
|-----------|--------|---------|
| CI gate (blocking) | ≥55% | 69% |
| Near-term | ≥75% | — |
| Target | ≥85% | — |

Key gaps: HTTP tool execute paths (load_testing, security_audit), fleet relay/registry
(feature-gated), A2A route handlers, SSE streaming paths.

### Scale Readiness (v0.21 — aligned with ai-hwaccel v0.21.3)

| Item | Priority | Notes |
|------|----------|-------|
| Lazy agent/provider initialization | High | Only load LLM providers when first used |
| Parallel crew execution profiling (`--profile`) | High | Identify bottlenecks in orchestration |
| Connection pooling for LLM providers | High | Share HTTP clients across crews |
| Caching infrastructure (detection, model routing) | Medium | TTL-based cache for repeated crew configs |
| Topology-aware fleet scheduling | Medium | Leverage ai-hwaccel NVLink/XGMI data for GPU-affinity placement |
| Cost-aware crew planning | Medium | Cloud GPU pricing lookup for budget-constrained crews |
| Container/VM environment detection | Medium | Auto-detect resource limits in containers |
| Real LLM execution (Phase 2) | High | Replace placeholder execute_task with actual provider calls |

### Ecosystem (v0.22 — aligned with ai-hwaccel v0.22.3)

| Item | Priority | Notes |
|------|----------|-------|
| Python bindings (PyO3) | Medium | Let Python callers use AgnosAI as a library |
| Multi-node fleet discovery (mDNS, Consul, K8s) | Medium | Auto-discover fleet nodes |
| Kubernetes operator | Low | CRDs for crew/agent definitions |
| WASM tool registry (remote fetch) | Low | Download community tools from a registry |
| Hot-reload tool registration | Low | Register/unregister tools without restart |

### Observability & Operations (v0.23 — aligned with ai-hwaccel v0.23.3)

| Item | Priority | Notes |
|------|----------|-------|
| Prometheus metrics export | High | Crew duration, throughput, error rate, GPU utilization |
| OpenTelemetry tracing spans | High | Distributed tracing across crew execution |
| Health monitoring & alerting | Medium | Agent health, provider latency, fleet node status |
| Multi-tenancy (crew isolation, resource quotas) | Medium | Per-tenant budget enforcement |
| Dashboard API (crew history, agent performance) | Low | REST endpoints for operational dashboards |

### Engineering Backlog

| Item | Severity | Notes |
|------|----------|-------|
| Split `core/resource.rs` (978 lines) into focused modules | Medium | accelerator.rs, device.rs, budget.rs, training.rs |
| DNS rebinding protection for A2A callbacks | Medium | Resolve-once + validate before connect |
| Request rate limiting per client IP | Medium | Tower middleware, token bucket or sliding window |
| Crew execution timeout | Medium | Configurable max wall-clock per crew via `tokio::time::timeout` |
| `#[must_use]` on Result-returning functions | Medium | Prevent accidental error swallowing |
| Clippy `unwrap_used` restriction lint | Medium | Forbid `.unwrap()` in non-test code via CI |
| SSE validate crew existence before subscribing | Medium | Return 404 for unknown crew_id, prevent EventBus leak |
| Fleet checkpoint phase isolation | Medium | Separate checkpointing flag from phase enum |
| Fleet barrier caller-side timeout | Medium | Async timeout to detect dead nodes and call `force_barrier()` |
| Doc comments on ~45 public items | Medium | Struct fields, type aliases, enum variants |
| Resolve `orchestrator/orchestrator.rs` module inception | Low | Rename inner file |
| EventBus LRU eviction for orphaned channels | Low | Background cleanup task |
| `ResourceBudget` enforcement in orchestrator | Low | Check `max_tokens`, `max_cost_usd` during execution |
| QLearner string interning | Low | Numeric IDs for large state spaces |

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
