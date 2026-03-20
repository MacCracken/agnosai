# Contributing to AgnosAI

## Getting Started

```bash
# Clone and build
git clone https://github.com/maccracken/agnosai.git
cd agnosai
cargo build

# Run tests
cargo test

# Run all CI checks locally
make check
```

## Project Structure

AgnosAI is a single Rust crate with feature-gated modules:

```
agnosai
├── src/
│   ├── core/             Core types, traits, error handling
│   ├── orchestrator/     Task scheduling, agent scoring, crew execution
│   ├── llm/              LLM provider abstraction (8 providers)
│   ├── fleet/            Distributed fleet coordination [feature: fleet]
│   ├── sandbox/          Tool execution isolation (WASM) [feature: sandbox]
│   ├── tools/            Tool registry & execution
│   ├── learning/         Adaptive learning & reinforcement learning
│   ├── server/           HTTP API server
│   └── definitions/      Preset library, crew assembly [feature: definitions]
├── benches/              Criterion benchmarks
├── tests/                Integration tests
└── examples/             Usage examples
```

## Development Guidelines

### Code Style

- `cargo fmt` before committing
- `cargo clippy` must pass with zero warnings (`-D warnings`)
- Use `thiserror` for library error types, `anyhow` only in binaries and tests
- Prefer `Arc<RwLock<T>>` over `Mutex` when readers dominate
- Use `DashMap` for concurrent registries with high read:write ratio
- Every public item should have a doc comment

### Testing

- Unit tests live next to the code (`#[cfg(test)] mod tests`)
- Integration tests in `tests/`
- All async tests use `#[tokio::test]`

### Commit Messages

Use conventional commits:

```
feat(orchestrator): add DAG topological sort
fix(llm): handle provider timeout gracefully
refactor(core): simplify TaskPriority ordering
test(scheduler): add priority queue edge cases
docs: update roadmap with Phase 2 progress
```

## Reporting Issues

Open an issue with:
- What you expected
- What happened
- Minimal reproduction steps
- Rust version (`rustc --version`)

## Security

See [SECURITY.md](SECURITY.md) for vulnerability reporting. Do not open public
issues for security vulnerabilities.
