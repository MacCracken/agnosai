# AgnosAI Roadmap

> Rust-native AI agent orchestration framework.

For completed work, see [CHANGELOG.md](../../CHANGELOG.md).
For architecture and integration context, see [docs/architecture/overview.md](../architecture/overview.md).

---

## Remaining Work

### Test Coverage

863 tests, 112 benchmarks across 19 files. CI gate ≥55% exceeded (~70%).

**Remaining gaps:**

| Area | What's missing |
|------|---------------|
| Process sandbox | env sanitization, timeout enforcement, kill-on-drop |
| Python sandbox | subprocess execution, timeout, env sanitization |
| Concurrent cancel | mid-execution interruption, parallel/DAG cancel stress |
| Telemetry init | OTLP error paths, env var override, guard lifecycle |

**Target:** ≥85% line coverage.

### Future Features (demand-gated)

| Item | Notes |
|------|-------|
| Python bindings (PyO3) | Separate crate, `cdylib` target, maturin build |

---

## Performance Targets

| Metric | Target | Measured |
|--------|--------|---------|
| Container image | <50 MB | — |
| Boot to ready | <2s | ✅ <1s |
| Memory (idle) | <100 MB | ✅ ~11 MB RSS |
| Crew creation | <10ms | ✅ <1ms |
| Concurrent crews | 100+ | ✅ Unlimited (async) |
| Fleet msg overhead | <1ms | ✅ 166 ns |
| Dependencies | ~30 | ✅ ~30 crates |

---

## Design Principles

1. **Concurrency over parallelism hacks** — tokio async, true multi-threaded
2. **Compile-time safety** — Rust type system catches what tests miss
3. **Single binary** — no container orchestration needed for single-node deployments
4. **Sandbox by default** — untrusted code never runs unsandboxed
5. **Wire compatibility** — REST/MCP/A2A API surface
6. **Library first** — `agnosai` is a library with feature-gated modules, not a framework
7. **Lockstep with ai-hwaccel** — aligned versioning, shared practices, same CI rigor
