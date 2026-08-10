# 017 — Call the GenAI span helpers, which the oracle never does

## Status: Accepted (2026-08-10)

## Context

`rust-old/src/telemetry/genai.rs` defines four span helpers — `inference_span`,
`tool_span`, `crew_span`, `record_usage` — and fifteen OTel GenAI attribute
constants. **Nothing in the oracle calls any of them.** Verified by grep over
`rust-old/src/`: the only occurrences outside `genai.rs` itself are the module
declaration `pub mod genai;` and the `///` example in `inference_span`'s doc
comment. The only `telemetry::` call anywhere in the oracle is `main.rs`'s
`init_tracing`.

The oracle's own tests confirm the intent is "public API, unused here": four of
the seven are tautologies (`assert!(span.is_disabled() || !span.is_disabled())`)
and the rest check only the *shape* of the constants.

That is coherent for a **library**. `agnosai` is a Rust crate as well as a
binary, and a downstream consumer — the Python platform, daimon, joshua — can
`use agnosai::telemetry::genai::inference_span` and instrument its own calls.
The crate offers the vocabulary; the consumer decides where spans begin.

It is not coherent for **this port**. The Cyrius tree is a binary. There is no
downstream Cyrius consumer that links it and calls in — the consumers reach
agnosai over HTTP. So if `src/` does not create spans, nothing ever will:

- `src/telemetry/otlp.cyr`'s exporter thread runs against a permanently empty
  ring and POSTs nothing, forever.
- `src/telemetry/genai.cyr`'s fifteen attributes describe spans that are never
  constructed.
- `OTEL_EXPORTER_OTLP_ENDPOINT` becomes a setting that changes the stderr log
  format from JSON to text (the oracle's OTLP branch uses the text formatter)
  and does nothing else — which is worse than not supporting OTLP at all,
  because it looks like it works.

Porting the helpers faithfully and leaving them uncalled would therefore
reproduce the oracle's *code* while inverting its *effect*: in the oracle the
helpers are dormant because someone else will call them, and here they would be
dormant because nobody can.

## Decision

**Call them.** `src/` creates GenAI spans at three sites and feeds them to the
OTLP exporter:

| site | span | kind |
|---|---|---|
| `llm/hoosh.cyr` — the chat completion call | `inference_span` + `record_usage` | CLIENT |
| `tools/registry.cyr` — tool execution | `tool_span` | INTERNAL |
| `orchestrator/crew_runner.cyr` — a crew run | `crew_span` | INTERNAL |

This is a **deliberate divergence from `rust-old`**, recorded here because
CLAUDE.md requires one for any divergence, and flagged to the user before it was
made rather than after.

Two supporting decisions come with it:

1. **The exporter handle is a process global**, set by
   `agnosai_telemetry_init_tracing` and read by the call sites. That mirrors the
   oracle exactly: `tracing::subscriber::set_global_default` is a process
   global, and every `info_span!` in a Rust program reaches it without being
   threaded through. Threading an exporter handle from `main` down into
   `hoosh.cyr` would be a larger and less faithful change.

2. **Trace identity stays the caller's**, per `telemetry/otlp.cyr`'s header.
   sakshi's trace id is a process global and cannot be made per-thread, so each
   span site mints its own identity rather than reading sakshi's. Correlating
   several spans into one trace is left to whoever owns the request, which is
   the honest boundary until an OpenTelemetry library repo provides a
   thread-local current-span (see `roadmap.md`, *Out of scope for v2.0*).

## Consequences

- **The telemetry group becomes live.** With `OTEL_EXPORTER_OTLP_ENDPOINT` set,
  a collector receives inference, tool and crew spans. With it unset, the
  exporter is never constructed and every call site's span creation is a few
  stores into a 104-byte record that is then dropped — no syscalls, no I/O.
- **A cost on every inference, tool call and crew run**, bounded and measured
  rather than assumed: span construction is one `alloc` plus a dozen stores.
  Benchmarked with the bite; the numbers are in the CHANGELOG.
- **The divergence is one-directional and easy to reverse.** Deleting the call
  sites restores oracle-faithful behaviour without touching `telemetry/`.
- **A future re-audit will find `src/` calling something `rust-old` does not.**
  That is what this record is for. Do not "fix" it back to match the oracle
  without reading the Context above.
