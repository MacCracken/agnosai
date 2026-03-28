# AgnosAI Roadmap

> Rust-native AI agent orchestration framework.

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

823 tests, 106 benchmarks across 17 files. Key remaining gaps: HTTP tool execute
paths (load_testing, security_audit need mock servers), SSE streaming edge cases,
telemetry OTLP init paths, adversarial input tests (prompt injection), sandbox
escape tests, concurrent cancel stress tests.

### Future Features (demand-gated)

| Item | Notes |
|------|-------|
| Python bindings (PyO3) | Needs separate crate with `cdylib` target + maturin build |

---

## Performance Targets

| Metric | Target | Measured |
|--------|--------|---------|
| Container image | <50 MB | — |
| Boot to ready | <2s | <1s |
| Memory (idle) | <100 MB | ~11 MB RSS |
| Crew creation | <10ms | <1ms |
| Concurrent crews | 100+ | Unlimited (async) |
| Fleet msg overhead | <1ms | 166 ns |
| Dependencies | ~30 | ~30 crates |

---

## Design Principles

1. **Concurrency over parallelism hacks** — tokio async, true multi-threaded
2. **Compile-time safety** — Rust type system catches what tests miss
3. **Single binary** — no container orchestration needed for single-node deployments
4. **Sandbox by default** — untrusted code never runs unsandboxed
5. **Wire compatibility** — REST/MCP/A2A API surface
6. **Library first** — `agnosai` is a library with feature-gated modules, not a framework
7. **Lockstep with ai-hwaccel** — aligned versioning, shared practices, same CI rigor
