# 037 — The server's crews are the orchestrator's to give back

**Status**: Accepted, shipped in 2.5.1
**Date**: 2026-11-13

## Context

[ADR 028](028-a-crew-lives-in-an-arena-its-consumer-gives-back.md) let a consumer build a crew in an arena and give it
back with `agnosai_orchestrator_release_crew`, which agnostic does when its own entry for the crew goes. The binary was
left out: a crew from `POST /api/v1/crews` was built with `agnosai_crew_new`, on the global heap, which has no `free`,
and stayed there past its eviction from the registry. So did its request, the crew an A2A delegation starts, and the
crew the `delegate` tool starts. ADR 028 said why it could not simply release on eviction: **a route reads a registry
entry with no hold on it**, so eviction cannot know a reader is finished, and a crew given back is scrubbed.

Measured on 2.5.0, through the routes with no audit chain, the loop below: a two-task crew posted, read, snapshotted
and listed left **13,144 B for good**, and a refused request 152 B, the cycle check's marks. The SSE stream minted a
subscription for every stream, about 2.4 KiB, and never gave it back.

## Decision

**A crew the server builds lives in an arena from its orchestrator's own pool, and the orchestrator gives it back when
its registry has let it go and nothing is reading it. A route that reads a crew holds it while it reads, and copies what
it keeps.**

- **The orchestrator owns what it is given.** `agnosai_orchestrator_crew_arena(o)` takes an arena from a pool the
  orchestrator makes on first use; the server builds the crew in it (`agnosai_crew_new_in`), and
  `agnosai_orchestrator_own_crew(o, spec)` gives it to the orchestrator, returning a hold for the builder. A crew is
  given back, through `agnosai_orchestrator_release_crew`, once its holds are 0 and its registry no longer has it:
  evicted, forgotten, or never registered at all, as a crew admission refuses. A spec on the global heap cannot be
  owned.
- **A reader holds.** `agnosai_orchestrator_hold_crew(o, id, &hold)` reads a registered crew's state and, for a crew the
  orchestrator owns, counts a hold, under the registry lock; `agnosai_orchestrator_let_go(o, hold)` lets go, and the
  last hold on a crew the registry let go gives it back. The count is on the spec, beside who owns it and whether the
  registry places it (`AGN_CREW_OWNER`, `_HOLDS`, `_PLACED`), all under the one lock.
- **A reader copies.** A JSON string holds its `Str` by pointer, and a route's body is serialized after the route
  returns, so a route renders the crew, copies the value (`agnosai_json_copy_a`, every string and key into the request
  arena), and lets go before it answers. `GET /api/v1/crews/{id}`, `POST /api/v1/crews` and A2A answer that way; the
  snapshot and the dashboard copy each string as they build; `agnosai_orchestrator_crew_ids_a` hands out copies, not the
  registry's keys.
- **What reads a crew:**

  | Reader | Holds |
  |---|---|
  | `POST /api/v1/crews`, A2A | from building the crew to its answer, as another request's crew may evict it meanwhile |
  | `GET /api/v1/crews/{id}`, the snapshot | while it renders |
  | the dashboard's two routes | each crew, while its row renders |
  | the SSE stream | the whole stream, as its subscription holds the crew's events until it is drained |
  | the `delegate` tool | until it has copied the crew's id |

- **A crew eviction gives back is not looked for in the ended queue.** `release_crew` walks the queue for entries in
  the crew's arena, an arena check an entry, as a forgotten crew's entry stays there; eviction has just taken an
  evicted crew's one entry off the queue's head, so the walk would find nothing, and at a full registry it cost a post
  40 µs. The spec says how the registry let it go (`AGN_CREW_PLACED`: not placed, placed, or evicted), and only an
  evicted crew skips the walk. This leans on an owned crew running once, which the server's do.
- **A crew a consumer built is not the orchestrator's.** `hold_crew` reads it with no hold, and eviction leaves it to
  its consumer, as ADR 028 says: a consumer's window is its own, not the registry's.
- **The request is read into the crew's arena**, as the spec takes its strings and agents as they are, and an arena no
  crew came of, a request refused, goes back at once. Validation and its messages are the request arena's
  (`agnosai_validate_crew_request_a`), and so is A2A's field list.
- **The SSE stream takes the bus's idle subscriptions** (`agnosai_event_bus_subscribe_a`) and gives its own back
  (`agnosai_event_bus_recycle`), and subscribes only to a crew that exists. Through 2.5.0 it subscribed to one that did
  not, which minted a sender and removed the channel of a crew that registered in between.

## Consequences

- **Positive**: `tests/orch_crew_arena.tcyr` posts, reads, snapshots and lists crews through the routes in a loop, and
  once the registry is at its cap the global heap does not move: 0 B a crew, where 2.5.0 left 13,144, and 0 B an
  A2A delegation and a refused request.
  A server's memory for crews is now its registry's cap times what a crew needs, held in its pool's arenas, where
  it grew with every crew served.
- **Positive**: every route that reads a crew is safe against a crew going back under it, which is what makes the
  release possible at all.
- **Negative**: **an owned crew is gone once let go.** Nothing it had is valid: a caller that wants something past its
  hold copies it, as the routes do. A library caller reading an owned crew with `agnosai_orchestrator_crew`, which takes
  no hold, could read one going back; the server's crews are read only by the routes.
- **Negative**: a pool keeps what it minted (ADR 028): as many arenas as the registry holds at most, each the size of
  the largest crew it served.
- **Negative**: a held crew past its eviction keeps its arena until let go. An SSE stream can hold one for as long as
  its client takes to drain it.
- **Named, and not here**: **the audit chain.** The server mints one, and each crew it runs leaves its entries on the
  global heap, 6,360 B for the two-task crew above, measured the same way: the chain is a ring of 10,000, and an entry
  that leaves it keeps its memory. That is the server's remaining per-crew cost, on the roadmap as its own release.
  A crew resumed from its log at start-up (ADR 036) is on the global heap, as it is rebuilt there; it is one per crew a
  restart found unfinished.

## Alternatives considered

- **A copy of what a route renders, and release on eviction with no count.** The roadmap offered it. A copy alone does
  not stop eviction scrubbing a crew mid-render: the route still reads the crew to make the copy, so it needs the lock
  or a hold for that long. Copying under the registry lock would hold it for a render of every result; a count holds
  it for two increments.
- **Holding until the response is sent**, with a release the serve layer calls after writing. It would spare the copy,
  and leave a hold to every caller of a route function, a test included, to remember.
- **Release on eviction for every arena crew.** ADR 028 rejected it: a consumer reads its crews after the registry lets
  them go.
- **Make every route answer pre-serialized JSON.** It would end the pointer problem at its root, and change what a route
  returns for every test that reads a body as a value.
