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

823 tests, 106 benchmarks across 17 files. Key remaining gaps: HTTP tool execute
paths (load_testing, security_audit need mock servers), SSE streaming edge cases,
telemetry OTLP init paths, adversarial input tests (prompt injection), sandbox
escape tests, concurrent cancel stress tests.

### Future Features (demand-gated)

| Item | Notes |
|------|-------|
| Python bindings (PyO3) | Needs separate crate with `cdylib` target + maturin build |

### v1.0.0 Release

| Item | Notes |
|------|-------|
| Crate conformance audit | Align with AGNOS public crate standards (see hisab 1.3, abaco 1.1, kavach 1.0, majra 1.0) — bench-history.csv, bench-latest.md, Makefile, codecov.yml, scripts/ |
| Version bump 0.25 → 1.0.0 | Via `scripts/version-bump.sh` — VERSION, Cargo.toml, recipe sync |
| Release checklist | `cargo publish` dry-run, crates.io metadata, docs.rs build, README examples verified |

### Post-v1.0.0

| Item | Notes |
|------|-------|
| Final migration: remove CrewAI from Agnostic | Exit criteria: Agnostic runs entirely on AgnosAI. Zero Python in the hot path |

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
