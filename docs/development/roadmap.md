# AgnosAI Roadmap

> Rust-native agent orchestration — replacing CrewAI with a purpose-built framework.

For completed work, see [CHANGELOG.md](../../CHANGELOG.md).
For architecture and integration context, see [docs/architecture/overview.md](../architecture/overview.md).

---

## Remaining Work

### Code Audit & Review (P0)

Multiple rounds of review covering all aspects before any further feature work:

- [ ] Security audit — input validation, sandbox escape paths, auth bypass, injection vectors
- [ ] Error handling — panic paths, unwrap usage, error propagation completeness
- [ ] Concurrency — lock ordering, deadlock potential, race conditions, Send/Sync correctness
- [ ] API surface — public API consistency, breaking change risk, documentation accuracy
- [ ] Test coverage — gap analysis, edge cases, failure path testing
- [ ] Dependency audit — supply chain, minimal surface, version currency, advisory compliance
- [ ] Performance — unnecessary allocations, hot path efficiency, memory layout
- [ ] Code quality — dead code, naming consistency, module organization

### ai-hwaccel Integration

| Item | Notes | Priority |
|------|-------|----------|
| Optional feature-gated dependency | `[features] hwaccel = ["dep:ai-hwaccel"]` | High |
| `HardwareInventory::detect()` via ai-hwaccel | Auto-detect GPU/TPU/NPU at node startup | High |
| Map ai-hwaccel types → agnosai types | Compatibility shim for 13→6 accelerator variants | High |
| Replace `AcceleratorType` with ai-hwaccel's | 13 variants, `#[non_exhaustive]`, richer model | Future |
| `suggest_quantization()` in LLM model router | Automatic precision selection per hardware | Future |
| `plan_sharding()` in fleet coordinator | Model distribution across devices | Future |
| Training memory estimation in resource budget | Validate agents have enough VRAM for workload | Future |

See [ADR-005](../adr/005-ai-hwaccel-integration.md) for full design.

### Remaining API & Protocol Work

| Item | Notes | Priority |
|------|-------|----------|
| Full JWT validation (RS256, claims, expiry) | Currently shared-secret only | Medium |
| SSE integration with CrewRunner events | SSE endpoint exists, needs event wiring | Medium |
| Remaining 12 presets | 6 of 18 built-in, need design/data-eng/devops standard+large | Low |

### Fleet (blocked)

| Item | Source | Blocker |
|------|--------|---------|
| Inter-node relay (Redis pub/sub) | Agnostic v1 fleet | Needs Redis |
| Federation (multi-cluster) | Agnosticos federation | Needs hardware |

### Agnostic Migration (Phase 5)

| Item | Priority |
|------|----------|
| Feature flag: `AGNOSTIC_BACKEND=agnosai\|crewai` | High |
| Port unit tests to run against both backends | High |
| Port E2E tests (Docker compose with AgnosAI binary) | Medium |
| Migrate presets domain-by-domain | Medium |
| Port high-value Python tools to native Rust | Medium |
| Community tool SDK (WASM) | Low |
| Remove CrewAI dependency | Final |
| Remove Python fleet code | Final |

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
6. **Library first** — `agnosai-core` is a dependency, not a framework
