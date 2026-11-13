# 036 — A crew keeps a log, and resumes from it

**Status**: Accepted, shipped in 2.5.0
**Date**: 2026-11-12

## Context

**A crew did not survive its process.** Everything a crew had was in memory, so a restart lost its work, finished or
not. agnostic answers a restart by marking the crews it had in flight `interrupted` (its ADR 0013), keeps what their
finished tasks answered from their `token` events (its ADR 0019), and waits on agnosai for the rest: its roadmap's F4,
"turns an `interrupted` crew into a resumable one". This is baseline in the field: LangGraph's checkpointers, Pydantic
AI on Temporal, Microsoft Agent Framework's Durable Task and OpenHands' event sourcing all resume a run from its last
step.

**`orchestrator/durable_state` had no caller.** The Rust line could save and load a crew state through a `StateStore`
trait with one file implementor and no caller anywhere, and the port kept it flat for that reason.

**A reader that fell behind had nowhere to go.** ADR 032 numbered a crew's events (`seq`), so a subscriber can tell an
event its ring dropped from one never sent, and then it can do nothing: agnostic's ring reports `missed`, and
re-reading the crew mid-run recovers nothing. ADR 032 left a snapshot to resync from to this release, and said what it
needs: a task's status and the crew's `seq` written under one lock.

**A tool call's journal was the consumer's.** ADR 029's rule 1 journals each call before it runs, so a crash leaves an
intent with no result, an unknown effect, and only a consumer's journal could hold it. A resume needs to know which
calls those were.

## Decision

**A crew can keep a log: its spec, every event it sends, numbered, each tool call journaled before it runs, and what a
resume needs beside them. A crew whose process stopped is resumed from its log: what it finished stands, and the rest
runs. A snapshot of a crew's tasks and `seq`, read in one step, lets a reader that fell behind go on from there.**

### `StateStore` is a backend table

`durable_state` keeps the oracle's two verbs, `save` and `load`, and gains the log's four: `append` one record,
synced on request; `read_log` the whole of one; `list` the crews that have one; `remove` a crew's log and snapshot.
`agnosai_state_store_backend(ctx, save_fp, load_fp, append_fp, read_log_fp, list_fp, remove_fp)` takes a consumer's
own, a database say, and `agnosai_state_store_new(dir)` still builds the file backend, so every 2.4.3 caller is
unchanged. Every verb refuses an unsafe crew id before a backend sees it (ADR 008). The file backend writes a log as
`{dir}/{crew_id}.log` beside the snapshot's `.json`, `0600` and `O_CLOEXEC`, as it holds answers and tool arguments,
and appends each record with one `O_APPEND` write, opening the file per record so nothing is held open.

### The log

One JSON object a line (`src/crew_log.cyr`):

| Line | Written | Synced |
|---|---|---|
| `{"crew":{...}}` | first, as the crew registers: its spec, with the tenant, tool limits, `delegated` and `trace_parent` its wire JSON leaves out | yes |
| `{"seq":N,"event":"...","data":{...}}` | every event the crew sends | `task_completed`, `approval_requested`, `approval_decided`, `crew_completed` |
| `{"journal":"intent",...}` | each tool call, before it runs: task, call id, tool, arguments, run | yes |
| `{"journal":"result",...}` | each tool call, when it is over: its effect | no |
| `{"state":{...}}` | a crew that ended without its run starting (cancelled while it waited, or no thread) | yes |

**With a log, every event is built and numbered**, whether or not anyone subscribed. The log is a tap on the crew's
sender (`agnosai_event_sender_with_tap`), and ADR 032 numbers an event when something receives it: the log receives
every one, so its `seq` is the one a subscriber sees. A crew with a log and no bus gets a sender of its own. The tap
writes outside the bus lock, on the sending thread, before the send returns, so a crew's next step never runs ahead of
its log; two threads' lines can land in either order, and a reader puts them back by `seq`.

**Beside three events, what a resume needs** (`agnosai_crew_event_with_extra`), which no subscriber sees: the task's
whole `result` on `task_completed`, and on `approval_requested`, so an answer waiting on its reviewer is asked about
again rather than paid for again; the crew's `state` on `crew_completed`, so a finished crew answers after a restart;
and the run number on `crew_started`.

**The crew's journal comes first.** With a log, a crew's tools journal through it (`agnosai_crew_log_journal_a`)
before any consumer's journal, which is still asked. A call whose intent will not go down does not run, as rule 1
says of any journal. A hierarchical worker's `assigned_by` and the manager's `role` now go on its result before its
`task_completed`, where they were added after, so the log keeps the result whole.

**A disk that refuses.** A spec that will not go down leaves the crew with no log, and an ERROR: it runs as a crew
with none, and a restart loses it, as one always did. An event that will not go down is an ERROR once a crew, and the
crew goes on with a short log, so a resume re-runs what it cannot see finished. An intent that will not go down
refuses its call.

### The snapshot

`agnosai_orchestrator_crew_snapshot_a(a, o, crew_id)`, and `GET /api/v1/crews/{id}/snapshot`:
`{"crew_id","status","seq","tasks":[{"task_id","status"}]}`. **Every move of a task's status, and of the run's own
phase, goes through one helper with the event that reports it, under the bus lock** (`agnosai_event_sender_send_set`),
and the snapshot reads them under the same lock. So no event at or before its `seq` is missing from what it says, and
none after it is in it: a reader takes each task as it says and goes on from the first event after `seq`.
`tests/orch_lifecycle.tcyr` takes snapshots from another thread throughout sequential, parallel, DAG and gated crews
and replays the stream against every one.

The crew's status is the run's phase, PENDING until `crew_started`, RUNNING, then its end with `crew_completed`, and
`awaiting_approval` while a task is, as the registry's roll-up says. A crew that ended without its run starting reads
as the registry has it. A crew put back from its log after a restart has no runner, and its snapshot is its state, at
`seq` 0. A wave's task now goes RUNNING at dispatch, with its `task_started`; through 2.4.3 it stayed QUEUED until it
ended, which nothing read until now.

### Resuming

`agnosai_crew_log_read` folds a log into a replay, and `agnosai_crew_runner_with_replay` runs from it:

- **A task the log finished is not run again**, completed or failed: a resume is not a retry. Its result stands, in
  its place, and its status moves with no event, as it finished before this run. In every process mode; a DAG's
  dependents are handed what it answered, from the log.
- **An answer that waited on a reviewer** is asked about again with that answer, and not paid for again.
- **A task whose earlier attempt made a call whose effect may stand waits for a reviewer before it runs again**: an
  intent with no result, or a result that `committed` or is `unknown`. A call that reported `none` is settled. It goes
  `awaiting_approval` with `approval_requested`, whose `output` lists the calls and their arguments and which carries
  `resume: true`. Approved, it runs again, and its result's `uncertain_calls` names them. Rejected, timed out, or with
  no gate, it fails as `not resumed: re-running it was rejected (<reason>)`, naming them, and its tool does not run.
  This is the rule agnostic holds agnosai to, that anything uncertain goes to a human, and **the user chose it** over
  failing such a task at once and over re-running it with the calls in its prompt. A wave's task holds its slot while
  it waits here, where an answer's review gives it back (ADR 033): approved, it runs at once.
- **Everything else runs.** A hierarchical crew's manager writes its answer once every worker has one, carried or new.
- **Its events go on from the log's last `seq`**, so the log has no gap across the stop, and `crew_started` says
  `resumed: true`. Each run has a number, 1 and one more a resume, on `crew_started`'s line and every journal line, as
  a model's call ids repeat from one attempt to the next.
- **What its finished tasks spent counts against its budget**, and so its tenant's, before it spends more: each
  result's `tokens.total` and `cost_micro_usd`. A task with tool calls spent more turns than its result says, so this
  is a floor.

### The orchestrator and the server

`agnosai_orchestrator_with_state_store(o, store)` gives every crew it registers a log, after admission.
`agnosai_orchestrator_resume_crew_err(o, crew_id, out_err)` reads one back: a crew whose log ended goes back in the
registry as it ended; one whose log did not is admitted again, registered and run on a thread of its own, as
`submit_crew` runs one. `agnosai_orchestrator_recover(o, out_restored)` does that for every log in the store.

**A crew's log is kept until the crew leaves the registry**, forgotten, evicted or released, and **the user chose
that** over removing it as the crew ends: so after a restart a finished crew still answers, and a consumer that
crashed before it saved an outcome can still read it.

**`AGNOSAI_STATE_DIR` turns it on for the server**, which claims the directory first (a lock on `<dir>/.owner`, so a
second server on it refuses to start), and the server **resumes at start-up, before it listens**, every crew whose log
did not end, which **the user chose** over registering them as `interrupted` until someone asks. Unset,
crews keep no log, as through 2.4.3.

## Consequences

- **Deliberate divergences from the Rust line**, as CLAUDE.md requires: the log, resuming, the snapshot and its
  route, a store with six verbs. The REST shapes gain fields and lose none: `crew_started.resumed`,
  `approval_requested.resume` and a result's `uncertain_calls`, on a resumed crew only.
- **Positive.**
  - A crew outlives its process. A restart costs the work in flight, not the work done, and nothing whose effect may
    stand is repeated without a human saying so.
  - A reader that fell behind, agnostic's ring or an SSE client, can resync mid-run.
  - The tool journal ADR 029 left to a consumer is agnosai's own, and a resume names what it could not see finish.
  - `StateStore` has a caller, and a consumer can put the log in its own database.
- **Negative.**
  - **With a log, every event is built and logged**, where an unwatched crew built none (ADR 022): its allocations
    return, and every event is a write. Without a log nothing changes, which `tests/orch_crew_runner.tcyr`'s
    allocation checks hold.
  - **Each tool call waits for an fsync** before it runs, and each task's end for another. On a fast disk that is far
    below a model call; on a slow one it is not.
  - A log holds a crew's answers and tool arguments on disk until the crew leaves the registry.
  - A crash that cuts a line in half leaves it unread, and a power cut may take the unsynced lines before it; a resume
    reads either as work not yet done. The directory is not fsynced when a log is created, so a power cut just after a
    crew registers may lose its log, which a process crash does not.
- **Neutral.**
  - A resumed REST crew's spec is on the global heap, as a crew from a request is; 2.5.1 moves the server's crews into
    arenas.
  - Only one process may own a store: a second would resume the first's live crews. The server claims its directory
    with an exclusive lock on `<dir>/.owner`, held for its life, and a second refuses to start, as agnostic claims its
    own database; a library caller that keeps logs claims its own store.

## Alternatives considered

- **patra's jsonl mode.** It opens `O_APPEND` without `O_TRUNC`, which a log wants, but patra is a database with its own
  page format, and `durable_state`'s header records why its contract does not fit. A consumer that wants patra plugs it
  in as a backend.
- **Persist the event ring alone.** It would not hold a result or a spec, so it could not resume anything; agnostic's
  ADR 0019 rejected it for the same reason.
- **Number the log's lines apart from the events.** Two counters would leave a reader of the log and a reader of the
  stream unable to say which they were looking at. One number keeps them the same record.
- **Read the snapshot from the log on disk.** Correct, and a read of the file per snapshot. The moves it folds are the
  ones made under the bus lock, so it reads them there.
- **Fail a task with uncertain calls at once, or re-run it with the calls in its prompt.** Offered to the user, who
  chose the reviewer. Failing loses work a human would have let run; re-running trusts a model with a decision about
  side effects it cannot check.
- **Register an unfinished crew as `interrupted` until someone asks.** Offered to the user, who chose resuming at
  start-up: it is what durable execution means, and a library caller can still resume one crew at a time.
- **Remove a log as its crew ends.** Offered to the user, who chose keeping it until the crew leaves the registry.
