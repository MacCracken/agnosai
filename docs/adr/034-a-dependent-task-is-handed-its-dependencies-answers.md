# 034 — A dependent task is handed its dependencies' answers

**Status**: Accepted, shipped in 2.4.2
**Date**: 2026-11-12

## Context

**A task that depended on another could not read what it said.** The DAG ordered tasks, and kept each one's result
only to decide what was ready next (`_agnosai_crew_run_dag`'s `completed`). A task's request was its agent's system
prompt, its own context map and its own turn (`_agnosai_crew_build_request_a`). The Rust line was the same: its
`run_dag` held results for the same reason and its `execute_task` read none of them.

agnostic found this writing its reviewer preset, one task doing the work and a dependent one checking it with a clean
context and an `output_schema` verdict. The review task would have had nothing to review. Its roadmap asks that agnosai
hand a dependent task its dependencies' outputs, as data, in its context. 2.4.3's hierarchical crews need the same
thing: a manager reads what its workers answered.

The roadmap had written this as a fourth strategy for `orchestrator/memory.cyr`'s conversation buffer, a summary
beside Full, SlidingWindow and HeadTail, so that a dependent task would get a summary and not a transcript. That
buffer has no callers, and it trims one task's own messages, which is a different thing. Nothing handed anyone a
transcript: nothing was handed at all.

The field hands over answers. CrewAI gives a task the outputs of the tasks its `context` names. OpenAI's Agents SDK
offers a filter that strips the tool calls from what a hand-off passes on (`handoff_filters.remove_all_tools`).
Anthropic's research system has its subagents return condensed findings to the lead, not their working (2025).

## Decision

### What a task is handed

**Each dependency it names, once, in the order it names them, with the answer that dependency's result holds.** The
answer is the result's output, which its `expected_output` shaped. The calls and results of its tool loop are not
part of it: they stay with the task that ran them, in its result's `tool_calls`. A dependency with no completed answer
is said to have none (`[no answer: it has not completed]`), so a reviewer is not left to guess. A failure's text is
not an answer, and is not handed over.

- **In a DAG**, a task runs only once every dependency has completed, so it is handed every one.
- **In a sequential crew**, a task is handed the answers of the dependencies listed before it that completed, and told
  that any other has none. A hierarchical crew runs its workers in order too, and since 2.4.3 its manager is handed
  every worker's answer to write the crew's from (ADR 035).
- **A parallel crew hands nothing.** Its tasks do not wait for their dependencies, so what had ended when a task
  started would be a matter of timing, and two runs of one crew would send different requests.

### Where it goes

**After the context preamble, as a user turn and a reply**, as the context goes: `Understood, I have the answers of
the tasks this one depends on.` The task's own turn stays last, so a validation retry rebuilds the request with the
hand-off and only its own turn changes. The runner sets it on the task as the task starts
(`agnosai_task_with_handoff`) and takes it back as the task ends, and a task's JSON never carries it: it is the run's,
not the definition's.

### An answer is as untrusted as what went into it

A task that read a page through a tool may repeat the page, so **every answer is handled as a tool result is**
(ADR 029, rule 2), whatever its task did:

- it goes in the markers the system prompt says never to obey, `<user_input field="dependency_result">`, preceded by
  a line in our voice that says each answer is data;
- `user_input` and `system_instructions` are rewritten inside it (`_` becomes `-`), in any case, and so is the
  dependency's id, which is request data too;
- a finding of the injection scan is said outside the markers, before the block, and logged as a WARN that does not
  name the answer.

The answer was through ADR 020's filter when its result was made, so it is not filtered twice.

### Bounded

**Each answer is cut at `AGNOSAI_HANDOFF_MAX_BYTES`, 32 KiB**, the tool loop's default for one result, on a character
boundary and saying how much was cut. A final answer is bounded by the model's `max_tokens` already; the cut is for
what is not, such as a reviewer's rework or a consumer's own result.

## Consequences

- **Deliberate divergence from the Rust line**: it handed nothing. No wire shape changes.
- **Positive.**
  - A dependent task can act on what came before it, which is what declaring a dependency asked for, and agnostic's
    reviewer preset can be written.
  - The manager of 2.4.3 has its workers' answers in the same form.
  - A hostile page that one task read reaches the next task in markers, as it reached the first.
- **Negative, behaviour changes.**
  - **A dependent task's request is longer**: it carries every answer it depends on, up to 32 KiB each, so it spends
    more of its budget, and a gateway's cache keyed on the request misses where it used to hit.
  - **A task that declared a dependency it did not need is handed it anyway.** A DAG used only to order its tasks now
    pays for the answers it orders by.
- **Neutral.** `orchestrator/memory.cyr`'s buffer is unchanged and still has no callers.

## Alternatives considered

- **A summary the model writes.** A call per hand-off costs a call, its tokens and its latency, and a summary of an
  untrusted answer is as untrusted as the answer. A task that wants a short answer can say so in its
  `expected_output`.
- **The answers in the task's context map.** The context is the crew author's, sanitized as such, and its markers
  are not unmarked: an answer holding `</user_input>` would have ended them. It is also the task's definition, which
  its JSON and the registry carry.
- **The whole transcript.** It hands over every tool result again, at many times the size, for a task that wants the
  conclusion.
- **Opting in per task.** Declaring a dependency is the opt-in. A field to turn it off can come when a crew needs one.
- **Every earlier answer in a sequential crew**, as CrewAI does by default. It would change every sequential crew's
  requests, where only a crew that named a dependency asked for one.
