# 032 — The crew lifecycle is one state machine, numbered, with the approval gate in it

**Status**: Accepted, shipped in 2.4.0
**Date**: 2026-11-12

## Context

[ADR 022](022-crew-events-and-status-say-what-happened.md) made every crew event and registry status describe something
that happened, and left five things to this one: an event `seq`, an `awaiting_approval` state, a reason on
`crew_completed`, a terminal state for a crew that cannot run, and `task_completed` in completion order. Two more join
them from the roadmap's 2.4.0:

- **The approval gate is built and never asked.** `orchestrator/approval.cyr` and `POST /api/v1/approvals` exist, as
  in the Rust line, and nothing waits on them. The Rust line built its gate in `main.rs` and never wired it either. A
  task marked `risk: high` runs unreviewed.
- **A watcher sees nothing of a task's tool calls** ([ADR 029](029-the-tool-loop.md)): a task with tools can run for
  minutes of turns with no event between `task_started` and `task_completed`.

agnostic builds its live views on these events (its ADR 0009) and numbers them itself, so it cannot tell an event the
bus lost from one that was never sent. Its roadmap waits on this for a crew roll-up of who needs a human, and it holds
agnosai to the rules recorded with turnstone's: a model judge may only tighten, and an automatic approval covers a
whole batch or none of it.

## Decision

**A crew and each of its tasks move through one written-down set of states, every move is an event, and every event a
crew's sender delivers is numbered.**

### The states

| | States |
|---|---|
| Crew | `pending`, `running`, `awaiting_approval`, `completed`, `failed`, `cancelled` |
| Task | `pending`, `queued`, `running`, `awaiting_approval`, `completed`, `failed`, `cancelled` |

`completed`, `failed` and `cancelled` are terminal, and a terminal state never changes (ADR 022's 2.1.6 note).

### The transitions, and the event that reports each

| From | To | Event | Who |
|---|---|---|---|
| crew `pending` | `running` | `crew_started` | the orchestrator stores it, the runner announces it |
| crew `pending` | `cancelled` | none: nothing ran | `cancel_crew` |
| crew `pending` | `failed` | none: nothing ran (`reason: not_started`) | a submit whose thread would not start |
| crew `running` | `awaiting_approval` | the first task's `approval_requested` | the runner, while any task awaits |
| crew `awaiting_approval` | `running` | the last waiting task's `approval_decided` | the runner |
| crew `running`, `awaiting_approval` | a terminal state | `crew_completed`, with `reason` unless `completed` | the runner |
| task `queued` | `running` | `task_started` | the runner, at dispatch |
| task `running` | `awaiting_approval` | `approval_requested` | the runner, after the answer |
| task `awaiting_approval` | `running` | `approval_decided` | the runner, with the decision |
| task `running` | `completed`, `failed` | `task_completed` | the runner |

`token`, `tool_call_started` and `tool_call_completed` move nothing: each is emitted only while its task is `running`.

A crew's `awaiting_approval` is a roll-up: it holds while at least one of its tasks awaits a decision, as herdr's
`blocked` does. A cancel overrides it, as it overrides `running`.

### `seq`

Every event a crew's sender delivers carries `seq`, from 1, assigned under the bus lock as the event goes out, so every
subscriber sees one order. A subscriber attached for the whole run sees 1, 2, 3 … with no gap. A gap is an event its
ring dropped, which is what `lagged` already counts. An event nobody was subscribed to receive is not built and not
numbered (ADR 022's guard on construction), so a subscriber that attaches mid-run starts at whatever the sender has
reached.

### A crew that cannot run

A crew whose tasks form a cycle, or depend on a task the spec does not hold, ends `failed` with `reason: invalid_dag`,
announces it in `crew_completed`, and its channel closes. Through 2.3.8 it went `running` → `pending` and stayed there
(ADR 022, decision 6), so a subscriber waited on it for good. `agnosai_orchestrator_run_crew_err` still returns 0 and
the error for it, since that caller asked to run a crew and none ran.

### `reason`

`crew_completed` and a terminal crew state carry `reason` when the status is not `completed`: `task_failed`, `timeout`,
`cancelled`, `invalid_dag` or `not_started`. The first four are the crew span's `error.type` vocabulary (ADR 023).

### `task_completed` in completion order

A parallel or DAG batch reports each task as its worker finishes, as the oracle's `join_next` did. Through 2.3.8 it
joined in dispatch order, so a fast task waited behind any slower one dispatched before it (ADR 022). Each batch has a
slot word per worker in the crew's arena; a finished worker puts its job in the next one as its last act, and the wave
waits on the slots in turn. Every task of a batch is still reported before the next batch starts. **The results stay
in dispatch order**, so a crew's state does not depend on which task happened to finish first.

### Subscribers that attach mid-run

The runner asks whether anyone is subscribed per task (sequential) and per batch (waves), where it asked once per run.
A subscriber that attached after the run began was told nothing until `crew_completed`. The sequential path also hands
a task the sender only when someone is subscribed, so an unwatched bus builds no token, as a wave's workers already did
not.

### The approval gate

**A task whose answer needs a reviewer waits for one.** `high` risk always does, and `medium` does when the
orchestrator is told so (`agnosai_orchestrator_with_medium_approval`). Only a completed answer is reviewed: a task that
failed has nothing to approve.

- The orchestrator mints its own gate, as it mints its own audit chain (`agnosai_orchestrator_approval_gate`), and
  gives it to every runner. The server's state shares it, so `POST /api/v1/approvals` decides what a crew waits on.
  The Rust line minted a second gate for the server, and no crew waited on either.
- The task goes `awaiting_approval`, the request is registered on the gate, and then `approval_requested` goes out with
  the answer, its risk and its agent, so a reviewer acting on the event finds it pending.
- It waits until a reviewer decides, the gate's timeout passes (five minutes by default), or the crew is cancelled or
  runs out of time. Each of those ends the wait at once and withdraws the request.
- `approval_decided` says the decision and why: `reviewer`, `timeout`, `cancelled`, `deadline`, `capacity` (the gate
  holds 1,000) or `no_gate`. **Only `reviewer` can approve**, and every other reason rejects, as the gate always has.
- An approved answer stands. A rejected one fails its task, and its output is replaced with
  `output withheld: approval rejected (<reason>)`, so it reaches neither the crew's results nor a dependent task.
  Either way the result's metadata carries `approval` and `approval_reason`, and the audit trail an `approval_decided`
  entry.
- A deadline that passes while a task waits times the crew out, as its next loop head would have.
- **With no gate** (`agnosai_orchestrator_with_approval_gate(o, 0)`, or a runner given none), such a task is rejected
  at once (`no_gate`). It never runs unreviewed.

A parallel or DAG task waits on its own worker's thread, so it holds up no other task of its batch. The crew's
registered status rolls up to `awaiting_approval` while any of its tasks waits, by compare-and-swap, so a cancel that
lands meanwhile stands.

**The rules agnostic holds this to, recorded for when a judge is added.** A model judge may only tighten: it may refuse
what these rules allow and never allow what they refuse, and gives its reasons from a fixed list. An automatic approval
covers a whole batch or none of it, as the tool loop's gates already do for a turn's calls
(`_agnosai_tool_turn_approve`), and anything uncertain goes to a human. agnosai has no judge and approves nothing
automatically.

### Tool calls

A task's tool calls are events too: `tool_call_started` as a call is journaled and about to run, and
`tool_call_completed` as its result is delivered, with `status` `ok`, `error` or `refused`. A refused call never ran, so
it has only the second. Each names its `task_id`, `call_id` and `tool`; a name the agent does not hold is `unknown_tool`,
as in the transcript, with what the model asked for beside it in `asked`. They are built in the crew's allocator, not
the turn's, as a subscriber may hold one after the turn's arena goes back.

## Consequences

- **Deliberate divergences from the Rust line**, as CLAUDE.md requires: `seq` on every event and an SSE `id:`; the
  `awaiting_approval` states; `reason` on a crew's state and its `crew_completed`; the approval, tool and decision
  events; a crew that cannot run ending `failed`; completion order, which is the oracle's again. The REST shapes gain
  fields and lose none.
- **Positive.**
  - A subscriber can tell a lost event from one never sent, and follows one order whatever thread sent each event.
  - A watcher sees which crew needs a human, and the human's decision, and a task's tool calls as they happen.
  - A high-risk answer no longer reaches its crew's results or a dependent task unreviewed.
  - No crew stays `pending` for good, and no subscriber waits for good on one that cannot run.
  - Every lifecycle claim is checked as an invariant: `tests/orch_lifecycle.tcyr` replays every stream it runs
    through the table above, and refuses a stream with a gap, an illegal move, or an event after the last.
- **Negative, behaviour changes.**
  - **A `high`-risk task waits for a reviewer**, up to five minutes, and is rejected if none decides. Through 2.3.8 it
    ran unreviewed. A `POST /api/v1/crews` holding such a task holds its pool worker while it waits.
  - **A submitted crew that cannot run reads `failed`**, where it read `pending`, and gets a `crew_completed`.
  - **Event order within a batch is completion order.** A consumer must not assume dispatch order.
  - **Event volume rises** by two events per tool call and two per reviewed task.
- **Neutral.** A snapshot to resync from mid-run, which agnostic asks for, is not here: it needs a task's status and
  the crew's `seq` written under one lock. 2.5.0's crew log is where it belongs.

## Alternatives considered

- **Number events per crew whether or not anyone receives them.** A subscriber that attached mid-run would see a gap for
  every event before it, which reads as loss. Numbering what was delivered makes a gap mean what it says.
- **A reason enum on the state instead of a word.** The words are the span's `error.type` vocabulary already, and a
  consumer reads JSON.
- **The runner holding a registry handle for the roll-up.** ADR 022 rejected that coupling; a pointer to the one
  registered state, moved by compare-and-swap, needs no handle.
- **A mutex per runner for the roll-up.** It would cost every crew a global-heap allocation for a path most never take.
  One process-wide lock is taken only when a task starts or stops waiting.
- **Run a high-risk task unreviewed when there is no gate.** The point of the risk level is that it is not.
- **A judge model now.** Nothing asks for one yet, and the rules it must keep are written above.
