# AgnosAI — Claude Code Instructions

> **Core rule**: this file is **preferences, process, and procedures** — durable
> rules that change rarely. Volatile state (current version, port progress, test
> counts, module line counts) lives in
> [`docs/development/state.md`](docs/development/state.md). Do not inline state here.

## Project Identity

**AgnosAI** (AGNOS + AI) — Provider-agnostic AI orchestration — crews, tasks, tools, agent delegation

- **Type**: Port (Rust → Cyrius). Module tree **mirrors `rust-old/`** — binary
- **License**: GPL-3.0-only
- **Language**: Cyrius (toolchain pinned in `cyrius.cyml [package].cyrius`)
- **Version**: `VERSION` at the project root is the source of truth — do not inline the number here
- **Rust reference**: 27,683 lines preserved at `rust-old/` (v1.1.0, green) — the **parity oracle**

## Consumers

Agnostic (Python platform), daimon (agent orchestration), joshua (NPC AI), kiran (game AI)

## The port

Scaffolded with `cyrius port` on 2026-07-28. `rust-old/` is the reference oracle.
The plan of record is
[`docs/development/cyrius-port-plan.md`](docs/development/cyrius-port-plan.md) —
read it before starting any bite. Its blocker table is a **reasoning archive** —
all eight are closed and nothing gates work — but it carries the corrections and
the analysis behind several still-live designs, which must not be re-derived.

### Layout — `src/` mirrors `rust-old/src/`

`rust-old/src/server/routes/crews.rs` → `src/server/routes/crews.cyr`. A group's
hub is `mod.cyr`, matching the oracle's `mod.rs`. Directories use the oracle's
spelling (`orchestrator/`, not `orch/`).

Cyrius `include` is textual and takes a path — the cyrius compiler's own tree does
this (`src/backend/x86/emit.cyr`). There was never a flat-layout constraint; the
correctness bar is "matches what Rust did", judged file-against-file, so the tree
has to show that correspondence.

Two things this does **not** change:

- **Symbol prefixes.** Cyrius has ONE flat namespace regardless of directory.
  `agnosai_*` on every public symbol is unaffected by where the file sits.
- **Include order.** Resolution is single-pass, callees before callers. Moving a
  file never reorders `src/main.cyr` — the same files in the same order preprocess
  to the same source, which is why the reorg was verifiable by a byte-identical
  binary.

Port-local modules with no oracle counterpart (`units`, `order`, `id`,
`guarded_fetch`, `chan_lossy`) stay at `src/` root — the directory tree means
"this mirrors rust-old", and inventing a home for them would dilute that.

Anything walking `src/` must recurse: `find src -name '*.cyr'`, not `src/*.cyr`,
which now matches **nothing** and fails open. `cyrius coverage` and `cyrius tests`
already recurse.

```sh
cyrius deps                                 # resolve dependencies into lib/
cyrius build src/main.cyr build/agnosai     # compile
cyrius tests tests                          # run every .tcyr (recursive)
cyrius coverage --min 80                    # the 80% gate — its own CI step
```

## Development Process

### P(-1): Scaffold Hardening (before any new features)

0. Read roadmap, CHANGELOG, and open issues — know what was intended before auditing what was built
1. Test + benchmark sweep of existing code
2. Cleanliness check: `cyrius fmt <file> --check`, `cyrius lint <file>`, `cyrius vet src/main.cyr`, `cyrius deny src/main.cyr`, `cyrius doc --check`
   — **`cyrius lint` takes a file**; bare, it prints usage and exits 1, so a gate written without one never lints anything
3. Get baseline benchmarks (`./scripts/bench-history.sh`)
4. Internal deep review — gaps, optimizations, security, logging/errors, docs
5. External research — domain completeness, missing capabilities, best practices, world-class accuracy
6. Cleanliness check — must be clean after review
7. Additional tests/benchmarks from findings
8. Post-review benchmarks — prove the wins
9. Repeat if heavy

### Work Loop / Working Loop (continuous)

1. Work phase — new features, roadmap items, bug fixes
2. Cleanliness check (as above) — **including `cyrius fmt --check` on `tests/*.tcyr`, not just `src/`**
3. Test + benchmark additions for new code
4. Run benchmarks (`./scripts/bench-history.sh`)
5. Internal review — performance, memory, security, throughput, correctness
6. Cleanliness check — must be clean after audit
7. Deeper tests/benchmarks from audit observations
8. Run benchmarks again — prove the wins
9. If audit heavy → return to step 5
10. Documentation — update CHANGELOG, roadmap, state.md, docs
11. Version check — `VERSION` is the source of truth; `cyrius.cyml` interpolates it
    (`version = "${file:VERSION}"`) so it cannot drift; confirm the CHANGELOG's top
    released heading matches. **Do not use `scripts/version-bump.sh` as-is** — it is
    the Rust-era script and still edits a root `Cargo.toml` that no longer exists
12. Return to step 1

### Task Sizing

- **Low/Medium effort**: Batch freely — multiple items per work loop cycle
- **Large effort**: Small bites only — break into sub-tasks, verify each before moving to the next. Never batch large items together
- **If unsure**: Treat it as large. Smaller bites are always safer than overcommitting

### Refactoring

- Refactor when the code tells you to — duplication, unclear boundaries, performance bottlenecks
- Never refactor speculatively. Wait for the third instance before extracting an abstraction
- Refactoring is part of the work loop, not a separate phase. If a review (step 5) reveals structural issues, refactor before moving to step 6
- Every refactor must pass the same cleanliness + benchmark gates as new code

### Key Principles

- **Cross-check against `rust-old/`.** The correctness bar is "matches what Rust did". Diverge only with an ADR.
- **Correctness over cleverness** — a Cyrius behavior that diverges *silently* from Rust is the worst outcome
- **Never skip benchmarks.** Numbers don't lie. The CSV history is the proof.
- **Tests + benchmarks are the way.** Minimum 80%+ coverage target.
- **Own the stack.** If an AGNOS crate wraps an external lib, depend on the AGNOS crate.
- **No magic.** Every operation is measurable, auditable, traceable.
- ONE change at a time — never bundle unrelated changes
- **`str_builder_*` over string concatenation** — avoid temporary allocations (the Cyrius form of "`write!` over `format!`")
- **Borrow pointers over copies** — allocate only when you must (the Cyrius form of "Cow over clone")
- **Vec arena over HashMap** — when indices are known, direct access beats hashing
- **Thread the `_a` allocator variants** — a per-request arena with one `reset_via` exit path. Bare `sandhi_server_*` / `_send_*` wrappers allocate on the no-free global bump; the design is in the port plan under its closed blocker #3
- **sakshi on all operations** — structured logging for audit trail
- **Prefix every public symbol `agnosai_*`.** Cyrius has ONE flat namespace and last-definition-wins. Short names (`add`, `run`) also get falsely credited by coverage's substring matching. `_`-prefix genuine internals so they leave the coverage denominator
- `var buf[N]` = N **bytes**, not N entries

## DO NOT

- **Do not commit or push** — the user handles all git operations (commit, push, tag)
- **NEVER use `gh` CLI** — use `curl` to GitHub API only
- **Do not modify `rust-old/`** — it is the frozen parity oracle
- Do not modify `lib/` — `cyrius deps` owns it
- Do not add unnecessary dependencies — keep it lean
- Do not skip benchmarks before claiming performance improvements
- Do not hardcode toolchain versions in CI YAML — `cyrius = "X.Y.Z"` in `cyrius.cyml` is the source of truth
- Do not use the stock `proj-tcyr` epilogue — the exit code is masked `& 0xFF`, so exactly 256/512/768 failures score PASS. Every `.tcyr` ends `var f = main(); if (f > 0) { f = 1; } syscall(60, f);`
- Do not put `.bcyr` files in subdirectories — `cyrius bench` no-arg discovery is **not** recursive
- Do not `include "lib/syscalls.cyr"` (or any stdlib module) in a `.tcyr`/`.bcyr` — the stdlib is auto-prepended, so an explicit include lands *after* it and single-passes into an undefined-symbol error

## Documentation Structure

```
Root files (required):
  README.md          — quick start, features, dependency stack, consumers, license
  CHANGELOG.md       — per-version changes (Added/Changed/Fixed/Removed)
  CLAUDE.md          — this file (durable process, principles, DO NOTs)
  CONTRIBUTING.md    — fork, branch, check, PR workflow
  SECURITY.md        — supported versions, scope, reporting
  CODE_OF_CONDUCT.md — Contributor Covenant
  LICENSE            — GPL-3.0
  VERSION            — source of truth for the version number

docs/ (required):
  architecture/
    overview.md      — module map, data flow, consumers, dependency stack
    math.md          — (if applicable) mathematical reference for algorithms/formulas
  development/
    state.md         — VOLATILE state: version, port progress, test counts (refreshed every release)
    roadmap.md       — milestones through v1.0, dependency gates
    cyrius-port-plan.md — the port's plan of record: phases, blockers, corrections

docs/ (when earned — not scaffolded empty):
  adr/
    NNN-title.md     — architectural decision records (when non-obvious choices are made)
  development/
    threat-model.md  — attack surface, mitigations (when security-relevant)
    dependency-watch.md — deps to monitor for updates/CVEs
  guides/
    usage.md         — patterns, philosophy, code examples
    testing.md       — test count, coverage, testing patterns

ADR format:
  # NNN — Title
  ## Status: Accepted/Superseded
  ## Context: Why this decision was needed
  ## Decision: What we chose
  ## Consequences: Trade-offs, what changes
```

## CHANGELOG Format

Follow [Keep a Changelog](https://keepachangelog.com/):

```markdown
# Changelog

## [Unreleased]
### Added — new features
### Changed — changes to existing features
### Fixed — bug fixes
### Removed — removed features
### Security — vulnerability fixes
### Performance — benchmark-proven improvements (include numbers)

## [X.Y.Z] - YYYY-MM-DD
### Added
- **module_name** — what was added and why
### Changed
- item: old behavior → new behavior
### Fixed
- issue description (root cause → fix)
### Performance
- benchmark_name: before → after (−XX%)
```

Rules:
- Every PR/commit that changes behavior gets a CHANGELOG entry
- Performance claims MUST include benchmark numbers
- Breaking changes get a **Breaking** section with migration guide
- Group by module when multiple changes in one release
- Link to ADR if a change was driven by an architectural decision
- **Do not compare Cyrius benchmarks to the frozen Rust `rust-old/bench-history.csv`.** The tokio-era numbers are not comparable across the port; the Cyrius line starts its own baseline

## Rust-era principles (apply when reading `rust-old/`)

These governed the Rust tree and have no Cyrius equivalent, but explain what you
will find there: `#[non_exhaustive]` on all public enums, `#[must_use]` on all
pure functions, `#[inline]` on hot-path functions, feature-gated optional deps.
