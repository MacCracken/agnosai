# Contributing to AgnosAI

## Getting Started

⚠ **AgnosAI is a Cyrius project.** There is no `Cargo.toml` and no `Makefile` —
the Rust tree is frozen at `rust-old/` as the parity oracle and is never built.
This section previously said `cargo build` / `cargo test` / `make check`; none of
those exist, and following them left a contributor unable to build at all.

```bash
git clone https://github.com/maccracken/agnosai.git
cd agnosai

# 1. Provision the pinned stdlib snapshot. `cyrius deps` only OVERLAYS
#    [deps.NAME] on top of lib/ — against an empty lib/ it fails.
cyrius lib sync --full

# 2. Resolve the git dependencies
cyrius deps

# 3. Build — produces build/agnosai (NOT agnosai-server)
cyrius build src/main.cyr build/agnosai

# 4. Every test suite, recursively
cyrius tests tests

# 5. Benchmarks and the coverage gate (its own CI step)
cyrius bench
cyrius coverage --min 80

# 6. Everything else CI runs
./scripts/check-clean.sh && ./scripts/check-symbols.sh
```

⚠ **`check-clean.sh` does not compile `tests/` or `benches/`.** It can be green
while a suite fails to build, which has happened. Before opening a PR, confirm
every unit compiles:

```bash
for f in tests/*.tcyr benches/*.bcyr fuzz/*.fcyr; do
    cyrius build "$f" /tmp/cb >/dev/null || echo "FAIL: $f"
done
```

## Project Structure

`src/` mirrors `rust-old/src/` file for file — `foo/bar.rs` becomes
`foo/bar.cyr` — so a reader can check any module against its oracle. Cargo
features have no Cyrius equivalent and are **not** a scope boundary here: the
port covers the feature-gated code too.

```
agnosai
├── src/
│   ├── core/             Core types, traits, error handling
│   ├── orchestrator/     Task scheduling, agent scoring, crew execution
│   ├── llm/              LLM routing over the hoosh gateway (7 provider labels)
│   ├── fleet/            Distributed fleet coordination
│   ├── sandbox/          Tool execution isolation (WASM, OCI, process)
│   ├── tools/            Tool registry & execution
│   ├── learning/         Adaptive learning & reinforcement learning
│   ├── server/           HTTP API server
│   └── definitions/      Preset library, crew assembly
├── benches/              `.bcyr` benchmarks (NOT recursive — keep them flat)
├── tests/                `.tcyr` suites (discovered recursively)
├── fuzz/                 `.fcyr` fuzz targets
└── examples/             Usage examples
```

## Development Guidelines

### Code Style

- `cyrius fmt <file> --check` before committing — including `tests/*.tcyr`,
  which `check-clean.sh` does not reach
- `cyrius lint <file>` must be clean. ⚠ It takes a FILE; bare, it prints usage
  and exits 1, so a gate written without one lints nothing
- `cyrius vet src/main.cyr` and `cyrius deny src/main.cyr` must pass
- **Prefix every public symbol `agnosai_*`.** Cyrius has ONE flat namespace and
  last-definition-wins; `_`-prefix genuine internals so they leave the coverage
  denominator
- **Thread the `_a` allocator variants** on anything request-reachable; the bare
  form allocates on a process-wide no-free bump
- Every public function needs a doc comment — `cyrius doc --check` enforces it,
  and it must sit immediately above the `fn`

### Testing

- Suites are `tests/*.tcyr` and may `include "src/foo.cyr"` directly
- ⚠ Never end a `.tcyr` with the stock epilogue — the exit code is masked
  `& 0xFF`, so exactly 256/512/768 failures score PASS. Use:
  `var f = main(); if (f > 0) { f = 1; } syscall(60, f);`
- ⚠ Do NOT `include` a stdlib module in a `.tcyr`/`.bcyr` — the stdlib is
  auto-prepended, so an explicit include lands after it and single-passes into
  an undefined-symbol error
- **Mutation-verify anything that matters**: apply the mutation, re-run, name the
  assertion that failed, restore. A test that stays green under the mutation is
  not covering the thing you think it is

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
