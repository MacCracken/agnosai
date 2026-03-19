# Contributing to AgnosAI

## Getting Started

```bash
# Clone and build
git clone https://github.com/maccracken/agnosai.git
cd agnosai
cargo build

# Run tests
cargo test

# Run clippy
cargo clippy --all-targets --all-features

# Format
cargo fmt --all
```

## Project Structure

AgnosAI is a Cargo workspace with 9 crates. See [README.md](README.md) for the architecture overview.

### Crate Dependency Order

```
agnosai-core          (no internal deps)
  ├── agnosai-orchestrator
  ├── agnosai-llm
  ├── agnosai-fleet
  ├── agnosai-sandbox
  │     └── agnosai-tools
  ├── agnosai-learning
  ├── agnosai-definitions
  └── agnosai-server  (depends on most crates)
```

## Development Guidelines

### Code Style

- `cargo fmt` before committing
- `cargo clippy` must pass with no warnings
- Use `thiserror` for library error types, `anyhow` only in binaries and tests
- Prefer `Arc<RwLock<T>>` over `Mutex` when readers dominate
- Use `DashMap` for concurrent registries with high read:write ratio

### Testing

- Unit tests live next to the code (`#[cfg(test)] mod tests`)
- Integration tests in `tests/integration/`
- E2E tests in `tests/e2e/`
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
