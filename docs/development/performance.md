# Performance Testing & Benchmarks

> How agnosai's benchmarks run and where their history lives. The numbers themselves
> are in `bench-history.csv` and the per-release notes in [`state.md`](state.md).

This page used to describe the Rust tree's Criterion benchmarks (`cargo bench`). That
tree is `rust-old/`, frozen at 1.1.0 and scheduled for deletion; its numbers are
**not comparable** with the Cyrius line — the allocator, harness and statistics all
differ — so the Cyrius benchmarks start their own baseline.

## Running benchmarks

```sh
cyrius bench                     # every benches/*.bcyr (and tests/*.bcyr)
cyrius bench benches/orch.bcyr   # one file
./scripts/bench-history.sh       # run all and append the rows to bench-history.csv
```

`bench-history.sh` stamps each row with the date and `VERSION` and normalises every
figure to nanoseconds. Record from a quiet machine: CI compiles the benchmarks (its
`Benchmarks` step exists so a harness that stops compiling is caught) but its timings
are too noisy to gate on.

⚠ From cyrius 6.6.6, `lib/bench.cyr` takes min/max only from timing windows at least
100× the clock's worst-case error and accounts in picoseconds, so averages move by up
to ~1 ns and min/max mean something different from earlier rows. Compare rows across
that boundary by their averages.

## The suites

| file | covers |
|---|---|
| `benches/core.bcyr` | core types, JSON round trips, resources |
| `benches/definitions.bcyr` | definition loading, assembly, packaging |
| `benches/fleet.bcyr` | registry, placement, GPU scheduling, relay |
| `benches/learning.bcyr` | capability, strategy, replay, profiles |
| `benches/llm.bcyr` | router, retry, inference queue |
| `benches/orch.bcyr` | orchestration shapes, scheduler, scoring, audit, IPC |
| `benches/order.bcyr` | sorting and selection |
| `benches/server.bcyr` | route handlers, rate limiting, request allocation |
| `benches/telemetry.bcyr` | GenAI spans, OTLP encoding |
| `benches/tools.bcyr` | tool registry and builtins |
| `benches/harness.bcyr` | the harness's own overhead |

## Reading a regression

Bound a change against a band of earlier rows, never a single row — one outlier low
reads as a regression that is not there. When a number moves, attribute it before
claiming anything: same source on two toolchains (the `CYRIUS_HOME` shim in
`state.md`) separates compiler effects from code effects.
