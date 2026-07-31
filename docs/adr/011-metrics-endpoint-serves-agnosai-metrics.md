# 011 — `/metrics` serves agnosai's own registry, not hoosh's

## Status: Accepted

## Context

`rust-old/src/server/routes/health.rs:16` is the whole handler:

```rust
pub async fn metrics() -> String {
    crate::llm::llm_metrics::gather()
}
```

and `rust-old/src/llm/mod.rs:26` is:

```rust
pub use hoosh::metrics as llm_metrics;
```

So in the Rust build, `GET /metrics` exposes the **hoosh crate's in-process
metrics registry**. That works there because hoosh is a linked dependency:
`crew_runner.rs:810` and `:864` call `llm_metrics::record_request(..)` directly,
writing into the same process-global registry the endpoint reads.

**The Cyrius port reaches hoosh over an HTTP seam** ([ADR 003](003-llm-native-http.md)).
There is no in-process hoosh, no linked registry, and nothing for
`llm_metrics::gather()` to map onto. `src/llm/hoosh.cyr:16-21` already records
this consequence for the whole family — audit chain, response cache, cost
tracker, token budget, DLP scanner, rate-limit registry and the metrics registry
are all hoosh-side under the seam — and notes that every agnosai consumer of them
lands in M5 or M6. This is that reckoning for the metrics one.

Meanwhile the port already contains `src/server/prometheus.cyr`, a complete port
of `rust-old/src/server/prometheus.rs`: six counters and a Prometheus text
renderer. In the Rust tree that module is **dead code** — `grep -rn AgnosMetrics
rust-old/src/` outside its own file returns nothing. It was written and never
wired to anything.

Three options:

1. **Serve nothing** (empty body, or drop the route). Wire-visible removal of an
   endpoint operators scrape, and it discards a working module.
2. **Proxy hoosh's `/metrics` over the seam.** Turns a metrics scrape into a
   network round trip on the request path, adds a failure mode to a liveness-
   adjacent endpoint, and is redundant: hoosh serves its own `/metrics` already,
   so an operator who wants hoosh's numbers can scrape hoosh.
3. **Serve `agnosai_metrics_gather`.**

## Decision

**Option 3.** `GET /metrics` serves the port's own `AgnosMetrics` registry, via
the process-global `agnosai_metrics_global()`.

The singleton shape is deliberate and matches the oracle's: `metrics()` takes no
`State` there because `llm_metrics::gather()` reads a process global, so the
route needs no state here either — and the recording sites, which are inside the
crew runner's worker threads, need nothing threaded down to them.

## Consequences

**The `/metrics` body differs from the oracle's.** This is the wire divergence,
and it is unavoidable under the seam: the oracle's body reports *hoosh's* view of
LLM traffic (per-provider request counts, latencies, token counts), while this
reports *agnosai's* view of orchestration (crews created and active, tasks
completed and failed, inference tokens and cost). Different subjects, not a
subset — an operator running both should scrape hoosh's `/metrics` as well, and
gets strictly more information than the Rust deployment offered from one port.

**Metric names do not collide**, so both can be scraped into one Prometheus
without relabeling: everything here is prefixed `agnosai_`
(`agnosai_crews_total`, `agnosai_tasks_completed_total`,
`agnosai_inference_cost_usd_total`, …).

**`AgnosMetrics` gets a consumer for the first time.** It was dead code in Rust.
That is a point in favour rather than an accident: the module exists, is tested,
is benchmarked at 5 ns per record, and measures exactly the things a crew
orchestrator should expose.

**The producer side is not wired yet, and that is the follow-up.** The oracle
records at `crew_runner.rs:810` and `:864` into hoosh's registry; the equivalent
calls into `agnosai_metrics_record_*` are a separate bite in
`src/orchestrator/crew_runner.cyr`. Until then `/metrics` renders a well-formed
exposition of zeros. Deliberately staged: `crew_runner` is finished M5 code and
this ADR is an M6 route decision, so widening one to satisfy the other in the
same bite would blur what each milestone verified. Tracked in
`docs/development/state.md`.

**The oracle's own test still passes.** `get_metrics_returns_200_with_prometheus_format`
(`health.rs:88-108`) asserts only that every line is a comment, contains a space,
or is empty — it never inspects a metric name. A zeros exposition satisfies it,
and so does a populated one.
