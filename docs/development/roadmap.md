# AgnosAI Roadmap

> Rust-native agent orchestration — replacing CrewAI with a purpose-built framework.

For completed work, see [CHANGELOG.md](../../CHANGELOG.md).
For architecture and integration context, see [docs/architecture/overview.md](../architecture/overview.md).

---

## Remaining Work

### API & Protocol Integration

| Item | Source | Priority |
|------|--------|----------|
| MCP server (tool advertisement) | Agnostic v1 MCP routes | High |
| A2A protocol (webhooks) | Agnostic v1 A2A | High |
| SSE streaming for crew execution | New | Medium |
| JWT auth + token delegation | Agnostic v1 auth | Medium |
| Preset library (18 built-in presets) | Agnostic v1 presets | Medium |
| .agpkg bundle export/import | Agnostic v1 packaging | Low |

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
