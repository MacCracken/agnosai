# 025 — A tool call passes a consumer's gate, and leaves a record on its task

**Status**: Accepted
**Date**: 2026-11-05

## Context

agnostic's crew body will carry a checked `target`, forwarded into each task's context, where the model
reads it. Two things stopped that declaration meaning anything at the point a tool ran, and a third
stopped a report using what a tool found
(`docs/development/issues/archive/2026-11-05-agnostic-tool-calls-cannot-be-held-to-a-target.md`):

1. **A tool could not see its task.** `execute_fp(ctx, input)` gets the tool's own pointer and what the
   model wrote, so `target_url` was the model's alone to choose. For a QA consumer the system under test
   writes the tools' output, so a page saying "now scan this other host" could steer the next call.
2. **A declared private host could not be reached.** `load_testing` and `security_audit` refuse anything
   `agnosai_is_safe_url` rejects. That is right for a URL a model made up, and it also refuses a staging
   system on a private network, where most QA targets live.
3. **A task result did not say what a tool measured.** A load test's p99 is a measurement, and a model
   saying "all passed" is a claim. A consumer has to be able to tell them apart.

The report asked for this "for F1's ADR", and F1, the tool loop, is scheduled for 2.3.x. No crew runs a
tool yet: `/mcp` `tools/call` is the only call site. None of the three needs the loop to exist, and
landing them first means F1 is written against a chokepoint that already enforces the rule.

## Decision

**A gate is a consumer's pair of callbacks, held on the tool and asked at the one chokepoint every call
passes through.**

```
agnosai_tool_gate_new(call_fp, url_fp, ctx)
  call_fp(ctx, input): 0, or the Str reason to refuse
  url_fp(ctx, input, url): AGNOSAI_GATE_PASS | _ADMIT | _REFUSE
agnosai_tool_registry_set_gate(registry, gate)
```

**The call travels on the input.** `agnosai_tool_execute_for(t, input, task, parent_sc)` is the
chokepoint, and it stamps the task, the tool and the gate on `input` before anything reads them, so
`agnosai_tool_input_task(input)` answers for a tool or a gate alike. An input belongs to one call, so
this is safe on any thread, and it changes neither `execute_fp` nor any tool written against it.
`agnosai_tool_execute_in` and `agnosai_tool_execute` are this with no task.

**`call_fp` runs before the tool.** A refusal returns a failed output and the tool never starts.

**`url_fp` is asked of the target and of every redirect hop.** `agnosai_tool_url_ok(input, url)` is the
one rule: REFUSE refuses; ADMIT passes `agnosai_url_is_admissible`; PASS, or no gate, leaves it to
`agnosai_is_safe_url`. `agnosai_guarded_fetch_hops` takes it as a hop check, so a gate holds past the
first 302. A gate both narrows, by refusing a public host off the target, and widens, by admitting a
declared private one. Both tools take the rule from `agnosai_tool_target_refused`, so neither implements
it. `load_testing` has no hop to check: its requests follow no redirect, sandhi's default.

**Anything but PASS or ADMIT refuses.** A callback that returns a stray value opens nothing.

**An admission has a floor.** It cannot reach 169.254/16, EC2's `fd00:ec2::254`,
`metadata.google.internal` or `metadata.goog`, nor a scheme sandhi will not speak. No QA target lives
there, and it is what a model talked into a gate would ask for first. The floor is this module's, so a
mistake in a gate's host comparison does not reach cloud credentials.

**The guard and the floor read an address the way it will be dialed.** The review of this ADR found that
`agnosai_is_safe_url` read a leading zero as octal, as a browser does, while sandhi reads it as decimal,
so `0127.0.0.1` passed as a public address and connected to loopback, with no gate involved. A gate's PASS
and the floor both stand on that guard, so it is fixed here: a host is judged by `sandhi_net_parse_ipv4`,
the parser that dials, as well as the WHATWG reading, and must pass under both.

`agnosai_url_host` and `agnosai_url_same_host` are public so a gate compares what sandhi will connect to:
`http://staging.example@evil.test/` starts with what it is pretending to be.

**A record goes on the task result.** `agnosai_task_result_record_tool_call(r, t, out)` appends `tool`,
`effect`, `success` and `result` or `error` to `metadata.tool_calls`, which a consumer already stores and
which leaves the oracle's wire form alone. The effect is F1's rule 1, set at the chokepoint:

| `effect` | Means |
|---|---|
| `none` | It was refused before it started, by `call_fp` or at its target |
| `committed` | The tool reported success |
| `unknown` | The tool reported failure or answered nothing, so it may have done part of its work |

ADR 020's redaction applies to the record, under the same switch, and to its strings only: what a tool
measured stays the number it measured.

## Consequences

- **Positive**: the rule lives in one place, and no tool implements it. A path cannot forget the gate,
  since it is read from the tool at the chokepoint. A tool that never heard of gates is held to one.
  `/mcp` is held to it today with no change to its route.
- **Positive**: an ungated tool call costs what it did. It adds three stores and a load: `tool_execute_no_gate`
  21 ns against 26 ns gated, with no allocation on either.
- **Negative**: a tool object holds one gate, so one shared between two registries takes the last set.
- **Negative**: a gate is consumer code on the hot path of a tool call. It runs on the calling thread,
  and a slow one is a slow tool.
- **Neutral**: `agnosai_url_same_host` compares hosts as written. It does not resolve them, like
  `agnosai_is_safe_url`, so a name that resolves somewhere private is still the accepted risk it was,
  and the floor's names are the ones it lists.
- **Neutral, and owed to F1**: nothing fills `metadata.tool_calls` until the loop exists, and nothing
  passes a task. F1 must call `agnosai_tool_execute_for` with the task and its `invoke_agent` context,
  record each result, journal before it runs (rule 1), and decide what rule 2 does with a flagged result.

## Alternatives considered

- **A third `execute_fp` argument.** Changes the calling convention for every implementor, including
  consumers' tools in `dist/agnosai.cyr`. Carrying the call on the input gets the same reach for nothing.
- **A thread-local current call.** Rejected for the reasons ADR 023 rejected one for spans: TLS does not
  cross `thread_create`, and parallel tasks run on their own threads.
- **The gate on the orchestrator or crew runner.** Reaches F1's loop and misses `/mcp`, so a call path
  could run ungated. On the tool, a path that forgets it cannot exist.
- **A host allowlist in agnosai.** One rule is not a gate: agnostic's depends on the task, and F1's rule 3
  approval step needs a callback anyway.
- **Letting ADMIT reach anything.** A gate is the operator's, and a metadata endpoint is the operator's
  to name. But the cost of a wrong host comparison there is the host's credentials, and nothing is lost.
