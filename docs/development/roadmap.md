# AgnosAI Roadmap

> Rust-native agent orchestration — replacing CrewAI with a purpose-built framework.

For completed work, see [CHANGELOG.md](../../CHANGELOG.md).
For architecture and integration context, see [docs/architecture/overview.md](../architecture/overview.md).

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

### Ecosystem & Scale (v0.22 — aligned with ai-hwaccel v0.22.3)

| Item | Priority | Notes |
|------|----------|-------|
| Topology-aware fleet scheduling | Medium | Leverage ai-hwaccel NVLink/XGMI data for GPU-affinity placement |
| Cost-aware crew planning | Medium | Cloud GPU pricing lookup for budget-constrained crews |
| Container/VM environment detection | Medium | Auto-detect resource limits in containers |
| Python bindings (PyO3) | Medium | Let Python callers use AgnosAI as a library |
| Multi-node fleet discovery (mDNS, Consul, K8s) | Medium | Auto-discover fleet nodes |
| Kubernetes operator | Low | CRDs for crew/agent definitions |
| WASM tool registry (remote fetch) | Low | Download community tools from a registry |
| Hot-reload tool registration | Low | Register/unregister tools without restart |

### Observability & Operations (v0.23 — aligned with ai-hwaccel v0.23.3)

| Item | Priority | Notes |
|------|----------|-------|
| Prometheus metrics export | High | Done — `/metrics` endpoint, hoosh metrics re-exported |
| OpenTelemetry tracing spans | High | hoosh 0.21.5 has `otel` feature — wire into AgnosAI |
| Health monitoring & alerting | Medium | Done — hoosh 0.21.5 has 3-strike health checker + majra heartbeat |
| Multi-tenancy (crew isolation, resource quotas) | Medium | Per-tenant budget enforcement |
| Dashboard API (crew history, agent performance) | Low | REST endpoints for operational dashboards |

### Engineering Backlog (from hoosh 0.21.5)

| Item | Priority | Notes |
|------|----------|-------|
| Cryptographic audit chain | Medium | HMAC-SHA256 tamper-proof inference event log via `hoosh::audit` |
| Per-endpoint rate limiting | Low | Sliding-window RPM via `hoosh::middleware::rate_limit` |
| Priority inference queue | Low | Batch/background inference via `hoosh::queue` + majra |
| OpenTelemetry integration | Medium | OTLP trace export via hoosh's `otel` feature gate |
| AgnosAI-specific Prometheus metrics | Medium | Crew execution counts, task durations, agent scoring histograms |
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
