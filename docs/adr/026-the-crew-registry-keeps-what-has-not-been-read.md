# 026 — The crew registry keeps what has not been read

**Status**: Accepted
**Date**: 2026-11-05

## Context

The oracle's `MAX_RETAINED_CREWS` eviction is a `retain` that drops every finished crew at once. The port
reproduced it. A consumer that reads state by polling loses whatever it has not reached: agnostic's
collector looks every 200 ms, and 1,300 unpolled crews at 190/s lost 885, each with its results. Clients
polling every crew to its end still lost 7 of 4,000
(`docs/development/issues/archive/2026-11-05-agnostic-registry-evicts-unread-finished-crews.md`).
agnostic worked around it by deleting from `AGN_OR_CREWS` under `AGN_OR_MUTEX` itself.

## Decision

**Eviction takes the oldest ended crews, down to a low-water mark of 900, and a consumer can forget a
crew it has read.** The number stays the oracle's 1000. This diverges from the oracle on purpose.

- **The marks are over crews that have ended, not over the registry.** Eviction fires at 1000 ended crews
  and stops at 900, however many are running. Counting running crews gave the window away under exactly
  the load it exists for: with 950 running and 50 ended unread, a submit would evict all 50 to reach 900.
- **Oldest means ended longest ago, not submitted longest ago.** What matters is how long a reader has had.
  A crew that ran for an hour and ended a moment ago is the newest here.
- **A queue holds crews in the order they end**, by their final state's pointer, not their id. A forgotten
  crew or an id run again leaves a stale slot, and going by id would evict a live re-run on the strength
  of its predecessor's slot. It is compacted in place once it outgrows twice what the registry could need,
  or a consumer that forgets every crew, and so never reaches the cap, would keep one slot per crew ever run.
- **Only a crew no runner holds can go.** `cancel_crew` marks a crew CANCELLED while its work finishes, so
  a terminal status does not mean it stopped. Such a crew is queued when it stops, and until then neither
  eviction nor `agnosai_orchestrator_forget_crew` will take it: its runner's final state would bring it
  back. The test is the cancel flag, which is published with the state and leaves under the same lock as
  the final state lands, so the two can never disagree. A thread that will not start gives its flag up.
- **`agnosai_orchestrator_forget_crew(o, crew_id)`** answers 0, a CrewNotFound error, or an Other error for
  a crew still running, as `cancel_crew` does. A crew left PENDING with no runner, which a cyclic DAG
  leaves, can be forgotten, or nothing could remove it.

900 keeps the window at 90% of the cap. The gap exists so a submit at the cap is cheap on average: one
in a hundred pays for a batch.

## Consequences

- **Positive**: a consumer that keeps up loses nothing, and one that forgets what it has read never
  meets eviction. agnostic can drop `_agnostic_crew_retire_a`'s reach into the orchestrator's fields.
- **Positive**: a registry full of running crews costs a submit one comparison, not a walk of the map.
- **Negative**: the registry now holds up to 1000 finished crews for good, where it used to fall to
  nearly none, and that is beside whatever is running, so it can hold more than 1000 entries.
  `GET /api/v1/crews` renders them all, so an idle server after a burst answers a longer list.
- **Negative**: `AGN_OR_SIZE` grew by two fields and each end pushes one slot.
- **Neutral**: eviction still loses a crew nobody read once 900 others have ended after it. That is the
  point of a cap. A consumer that cannot fall behind should forget, not poll.
- **Neutral**: all of this holds for one run of an id at a time. Overlapping runs of one id have always
  shared an entry and a flag, and the first to end takes both.

## Alternatives considered

- **Evict one per submit, no low-water mark.** Keeps the window at 999, and was what the queue makes
  cheap. The report asked for a low-water mark, and a gap of 100 costs a reader little.
- **Sort the finished crews by a stamp at eviction.** No queue to keep, but a walk and a sort on every
  eviction, and a walk on every submit once the registry is full of running crews.
- **A doubly linked list through the crew state.** Exact and O(1) to forget, at the price of two fields in
  a core type for what is the registry's concern.
