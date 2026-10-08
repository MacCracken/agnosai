# 024 — An embedding consumer can record an HTTP server span through agnosai's exporter

**Status**: Accepted
**Date**: 2026-10-07

## Context

[ADR 023](023-genai-semconv-spans-and-w3c-trace-context.md) made every crew one trace: its
`invoke_workflow` span is a child of the span context the crew is handed, and a crew submitted
over agnosai's HTTP surface joins the request's inbound `traceparent`. agnostic links agnosai
in-process (`dist/agnosai.cyr`) and hands each crew a context of its own: it parses the
request's `traceparent`, or mints one, and passes it with `agnosai_crew_with_trace_parent`
(agnostic ADR 0018).

When agnostic mints that context, the crew's `invoke_workflow` span names a parent span id
that nothing ever exports. A backend then shows the trace's root as missing, though the trace id
still joins it to agnostic's log lines. agnostic's roadmap records the fix — "record an HTTP
SERVER span per request through agnosai's exporter, whose span id is the minted one" — and it
cannot do it today: agnosai's exporter carries only the four GenAI operations
(`chat`, `invoke_agent`, `invoke_workflow`, `execute_tool`), each encoded with `gen_ai.*`
attributes and kind CLIENT or INTERNAL. A second exporter in agnostic would mean a second ring,
thread and POST path per process, and a second implementation of the OTLP encoder.

agnosai's own server records no HTTP span either (roadmap B7, item 1: the oracle's
`#[instrument]` sites). That is a separate decision about agnosai's surface; this one is about
the exporter's vocabulary.

## Decision

**agnosai's span record and exporter carry one more kind of span: an HTTP server span, built by
`agnosai_http_server_span_a(a, method, route, path, scheme, status)` and recorded with
`agnosai_telemetry_record_span_in` like any other.**

- **What it is.** SERVER, named `<method> <route>` — `GET /api/v1/crews/{id}` — as the HTTP
  semantic conventions name a server span. It carries `http.request.method`, `http.route` (the
  matched route's pattern, low-cardinality), `http.response.status_code`, `url.path` and
  `url.scheme`, and none of the `gen_ai.*` attributes. Identity, parent and timing come from
  the export context exactly as for a GenAI span.
- **Where it lives.** In the caller's allocator. The export path encodes a span when it is
  recorded and keeps nothing of it, so a span built per request in the request's arena costs
  the global heap nothing (`_agnosai_genai_span_new_a`; the GenAI constructors keep their
  global form).
- **How it shares the record.** It is the GenAI span record with an `http.server` operation
  sentinel and four more slots — route, path, status, scheme — with the method in the
  operation's `Str`. `agnosai_span_is_http_server` tells the two apart; the kind and the
  encoder branch on it. A GenAI span's new slots are 0 and -1, and its encoding is unchanged.
- **Who records it.** The embedding consumer. agnosai's own server does not, until B7 decides
  that it should.

## Consequences

- **Positive**
  - agnostic can export a SERVER span per request with the span id it minted, so a crew's
    `invoke_workflow` span has a parent the backend can find, through the one exporter the
    process already runs.
  - When agnosai's own server records its routes (B7), the constructor and encoder exist.
  - Tested in `tests/telemetry_otlp.tcyr` (`_t_http_server_span`): the kind, the name, every
    attribute and accessor, no `gen_ai.*`, and no global allocation. The accessors answer 0 (the
    status -1) for a GenAI span and for none.
- **Negative**
  - The span record grows from 120 to 152 bytes for every GenAI span too.
  - The record now holds two vocabularies. A third kind of span would be the point to split it.
- **Neutral**
  - agnostic adopts it at its next re-pin (agnostic roadmap, *Telemetry*).

## Alternatives considered

- **A second exporter in agnostic.** Rejected: a second ring, thread and POST path per process,
  and a second OTLP encoder to keep in step with this one.
- **Have agnostic record its request as a GenAI `invoke_workflow` span.** Rejected: it is not a
  workflow, a backend would count every request as one, and it would carry no HTTP attributes.
- **A generic span API (any name, kind and attribute list).** Not now: it would let a consumer
  emit attributes the semantic conventions do not define under agnosai's scope, and the one need
  in hand is a server span.
