# 035 — A manager assigns its workers and writes the crew's answer

**Status**: Accepted, shipped in 2.4.3
**Date**: 2026-11-12

## Context

**A hierarchical crew ran sequentially.** The runner warned `hierarchical mode not yet implemented` and ran every task
in order, as the Rust line did. Its `orchestrator/hierarchical.rs` assigned each task to its best-scoring worker
(`delegate_tasks`), and nothing called it: it was the Rust line's unbuilt Phase 2. So agnostic refused the mode with a
400, as accepting it would have given its caller a different one.

**Its manager named no agent.** The process carried `manager: Uuid`, and an agent has no id, only its `agent_key`.
`POST /api/v1/crews` with `"process": "hierarchical"` gave it a fresh `Uuid::new_v4()`.

**A delegated agent held whatever its definition named.** The `delegate` tool started a crew for any agent definition,
with all of that agent's tools and the server's per-crew budget, however little the delegating agent held.

agnostic holds agnosai to a harness rule for this, recorded with turnstone's: **authority only narrows**. A worker
holds at most its manager's tools and budget, and a need beyond them goes up to a human; neither the manager nor the
model can grant it. And 2.4.2 handed a dependent task its dependencies' answers (ADR 034), the form in which a
manager can read its workers'.

Orchestrator-worker is where the field converged: Google and DeepMind's 2025 scaling study measured error
amplification of 17.2× for independent agents against 4.4× for centralised coordination, and Anthropic's research
system has a lead agent combine what its subagents return.

## Decision

### The manager is an agent of the crew, by its key

`agnosai_process_hierarchical(manager)` takes the manager's `agent_key`, and a crew's JSON carries it as
`{"hierarchical": {"manager": "<key>"}}`. Any non-empty string is a key, so a UUID from 2.4.2 still reads, as a key
that names no agent.

**A manager that names no agent of the crew's cannot run.** The run returns no state and an
`invalid_definition` error, as a cycle does, and the crew ends `failed`, `reason: invalid_manager`, with
`crew_completed` sent, the reason ADR 032's list gains.

### The manager assigns, the workers answer, the manager writes

1. **Each task goes to its best worker.** The workers are every agent but the manager, and each task is ranked
   against them as any task is ranked against a crew's agents. The manager is never a candidate: it would win a task
   on score, which is what a hierarchical crew exists to prevent. With no other agent, the manager takes every task,
   as the Rust line's `delegate_tasks` falls back to it.
2. **The workers run in order**, each handed the answers of the dependencies listed before it (ADR 034), as a
   sequential crew's are. Each result records `assigned_by`, the manager's key.
3. **The manager writes the crew's answer** once every worker has given one, in one more call: it is handed every
   task's answer, in the crew's order, and told what each task was. Its result is the crew's last, with
   `role: "manager"`. **It writes and calls nothing**: it is offered no tool, and what it knows is what its workers
   answered.

A crew whose worker failed fails `task_failed` without the manager's answer, and one cancelled, past its deadline or
out of budget before it stops there too: a call that could not help is not made. A crew that has not had its
manager's answer has not completed, so `crew_started`'s `task_count` counts it.

**The assignment stays explainable.** Selection is still a pure function of the roster and the task, the roster now
being the workers as they are narrowed: `agnosai_explain_selection_a` over `agnosai_hierarchical_workers_a` names
the agent each task ran on, exactly, and the record says it too.

### Authority only narrows

- **A worker holds at most its manager's tools**: those both name, in the worker's order
  (`agnosai_worker_within_a`), on a copy, so its definition is untouched. Its system prompt lists those, the tool loop
  offers those, and it is ranked on those, so a task needing a tool the manager does not hold is not assigned on the
  strength of it. **A tool withheld is recorded**: `withheld_tools` on the worker's result, and a WARN once a worker a
  run. Nothing in the run widens a grant. The crew's author does, by giving the manager the tool.
- **A worker holds at most its manager's budget** without anything new: every task of the crew already spends under
  its crew's tracker (ADR 033).
- **The delegate tool narrows the same way.** A tool call's input now carries its caller, the agent as its task holds
  it, and its task's budget tracker (`agnosai_tool_input_agent`, `agnosai_tool_input_budget`). The agent delegated to
  holds at most the caller's tools, and its crew at most what the caller's task has left
  (`agnosai_budget_tracker_tokens_left`, `_cost_left_micro_usd`). A task with nothing left hands on nothing: the call
  is refused. The tool's answer names the tools withheld, so the model knows.
- ⚠ **The delegated budget is a ceiling taken when it delegates, not a share.** The delegated crew runs past the call
  and its task, and a tracker under the task's would outlive it, so what the delegated crew spends is not recorded
  above it, and a task that delegates and spends on can together pass what it had.

### On the wire

`POST /api/v1/crews` takes `manager`, an `agent_key` of the request's agents. A hierarchical request without one, or
naming one it does not hold, is a 400 (`a hierarchical crew must name its manager`, `manager names no agent in
agents`), as is a manager on any other crew (`manager is only for a hierarchical crew`). These are checked after
every check the Rust line made, so a request it refused reports what it reported. Each result in the response carries
`assigned_by`, `withheld_tools` and `role` when it has them, as it carries `tool_calls`.

## Consequences

- **Deliberate divergences from the Rust line**: the mode runs as its name says; the manager is a key; a hierarchical
  request must name it; a delegated agent is bounded by its caller. The REST shapes gain fields and lose none, and a
  hierarchical request accepted through 2.4.2 without a `manager` is now a 400.
- **Positive.**
  - agnostic can accept `hierarchical`, and a crew gets one answer, written by its manager from what its workers found.
  - A worker cannot do more than its manager, whether a run assigned it or a model delegated to it, and the record
    says what was withheld.
  - A selection explanation stays exact for a hierarchical crew.
- **Negative, behaviour changes.**
  - **A hierarchical crew makes one more call** than it has tasks, and returns one more result.
  - **A hierarchical crew built through 2.4.2 with a UUID manager fails `invalid_manager`**, where it ran
    sequentially.
  - **A worker may lose tools**, and a task needing one runs without it.
  - **A delegated crew is bounded by its caller**: its agent may lose tools, and its crew holds what the delegating
    task had left.
- **Neutral.** `orchestrator/hierarchical.cyr`'s `delegate_tasks` and `ManagerAgent` stay as the Rust line wrote them;
  a run ranks the same workers the same way, and a test holds the two to agree.

## Alternatives considered

- **The manager's model assigns each task.** One more call per crew, an assignment that is no longer a pure function
  of the roster and so no longer explainable by recomputing it, and an injection surface: a task's text could steer
  which worker gets which tool.
- **Assigning only, as the Rust line designed.** The manager would never call the model, and nobody would read its
  workers' answers, which is what a manager is for.
- **A manager that writes with its tools.** A manager acting itself is a worker that cannot be held under anyone.
- **A task that needs a withheld tool waits for a human.** The approval gate approves answers, not grants, and
  widening one per task is a new kind of decision for a reviewer. The record names what was withheld, and the crew's
  author grants it.
- **A hierarchical request without a manager managed by its first agent.** It keeps every request accepted, and
  chooses an agent the caller did not.
- **A delegated crew's tracker under the delegating task's.** What it spends would count against its caller, and the
  tracker would outlive the task's arena the moment the task ended.
