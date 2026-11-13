# 028 — A crew lives in an arena its consumer gives back

**Status**: Accepted
**Date**: 2026-11-06

## Context

The global heap has no `free`. `agnosai_orchestrator_forget_crew` takes a crew out of the registry and
leaves every byte where it was, so eviction bounds how many crews are reachable and not what they cost. A
long-lived server spends memory per crew served, and only a restart gives it back
(`docs/development/issues/archive/2026-11-06-agnostic-a-forgotten-crew-keeps-its-memory.md`).

Measured through this tree before the change, on one placeholder task: 9,368 B per crew, of which the spec
is 1,816, the subscribe channel 2,392, registration 696 and the run 4,464. A task a model answered added
about 4.3 KiB, and a task in a parallel crew about 4.1 KiB of thread handle and TLS that `thread_join` does
not give back. agnostic's 11,080 B is this plus about 1.7 KiB of its own.

It cannot simply be `forget_crew`. agnostic reads a crew after it forgets it: the spec for
`GET /crews/:id/plan`, the results until its outcome is written. Its contract with 2.2.4 is that those
pointers stay valid, and ADR 026 says so in as many words.

`alloc` has no redirect, so nothing can move the stdlib's allocations from outside. The arena has to be
threaded through every call on the crew's path, and any bare `alloc`, `str_from` or `vec_new` left on that
path is a leak in an arena crew.

## Decision

**A crew built with `agnosai_crew_new_in(a, name)` lives in a crew arena, and everything the engine allocates
for it comes from there. `agnosai_orchestrator_release_crew(o, spec)` ends it and gives the arena back to its
pool.** `forget_crew` is unchanged.

```cyrius
var pool = agnosai_crew_pool_new(AGNOSAI_CREW_ARENA_BYTES);      # once

var a = agnosai_crew_pool_acquire(pool);                         # per crew
var spec = agnosai_crew_new_in(a, name);                         # names, tasks: the `_a` forms, in `a`
var sub = agnosai_event_bus_subscribe_a(a, bus, agnosai_crew_id(spec));
agnosai_orchestrator_submit_crew(o, spec);
# ... the channel closes; read what is wanted; forget ...
agnosai_orchestrator_forget_crew(o, agnosai_crew_id(spec));
# ... later, when the consumer's own entry goes, and no reader can hold a pointer ...
agnosai_orchestrator_release_crew(o, spec);
agnosai_event_bus_recycle(bus, sub);
```

- **Opt-in, crew by crew.** The allocator is read off the spec (`agnosai_crew_alloc`), so a crew built with
  `agnosai_crew_new` runs on the global heap as it always did. The binary's own `POST /api/v1/crews` is one of
  those. `agnosai_crew_new_in` refuses any other allocator: `agnosai_crew_new_a` places a spec, and a request
  arena must not be asked to hold a crew that outlives its request.
- **A second call, not a change to `forget_crew`.** The issue offered this or a copy at retirement. A copy per
  crew pays for what the consumer already has, and breaks the contract 2.2.4 gave. A consumer forgets when it
  has read, and is done some time later. The two are different moments.
- **`src/crew_arena.cyr`, and not `arena_pool`.** A crew's arena takes a mutex, as a parallel crew's workers
  share it. It is scrubbed on release, as `arena_reset` only rewinds and a crew's prompts and keys must not be
  readable by the next tenant through an allocation it never wrote. That also keeps `alloc`'s promise that
  memory arrives zeroed. And its pool is an unbounded stack that starts empty, because the number held is the
  consumer's retention and not this process's concurrency: a bounded pool would have to prefill thousands of
  arenas, or drop what it could not park, which is the leak again.
- **The subscribe channel is reused and not moved.** `chan_new` allocates on the global heap and a closed
  channel cannot be opened again through the public verbs, so `agnosai_chan_reopen` reaches into `closed` at
  +48, the one place the port does. It is at +48 in all four thread backends, and the suite closes, reopens
  and sends. The ring goes back to the bus with `agnosai_event_bus_recycle`, not to the crew.
- **`agnosai_event_bus_remove` empties the sender's list**, as a sender that listed a recycled subscription
  would publish one crew's events to another's reader. This is the one change a consumer that does not opt
  in can observe: a stale sender's `send` answers 0, where it answered the closed channels it could not reach.
- **The worker pool's threads are detached**, on every crew. `thread_create` allocates a handle and 4 KiB of
  TLS on the global heap and `thread_join` returns neither, so a parallel crew cost more than the arena saved.
  A worker's last act is a futex-woken `DONE` word on its job, which is what the wave waits on.
- **What outlives a crew stays off its arena.** The audit chain keeps its entries' metadata, and a JSON string
  holds its `Str` by pointer, so that metadata is on the global heap and owns copies. Built only when there is a
  chain. A span's fields are encoded when it is queued, and nothing keeps them.
- **`release_crew` waits out the run's last instructions, and takes the crew out after them.** A run drops its
  flag under the registry lock and then still has its event channel to remove, with the crew's id in hand.
  `AGN_CREW_RUNS` falls as the run's last touch of the crew, and `release_crew` reads the flag and the count under
  one lock, so nothing can put the crew back after it is gone. That is a few instructions, except for two
  overlapping runs of one spec, which share a flag: there it waits for the second run.
- **The stdlib's atomics do not order, so the job word has fences.** `atomic_load` and `atomic_store` are plain
  moves. `thread_join` went through the kernel and ordered everything, and a wave that finds `DONE` set does not.

## Consequences

- **Positive**: on the measurement the issue asked for, the global heap does not move: 0 B per crew over every
  process mode, a model answering, a schema failing, redaction on, and 600 crews submitted and released as
  each channel closed. A pool settles at the most crews held at once times the most any needed.
- **Positive**: the work this took helps every crew. A prompt-leak scan no longer mints a `Str` per window,
  4 KiB on every task a model answered. `agnosai_is_retryable` allocates nothing. A parallel crew leaves no
  thread handle. An `arena_pool` keeps what overflows it instead of dropping it for good. The bound on a crew
  that does not opt in fell by those amounts.
- **Negative**: after `release_crew` nothing the crew had is valid: spec, id, tasks, state, results, events.
  A consumer clears its own pointers first, under whatever lock its readers take, and calls it exactly once. A
  second call is refused only until the arena serves another crew. Reads that race a release are the
  consumer's to prevent, and the engine cannot do it for them.
- **Negative**: `agnosai_orchestrator_crew_ids_a` hands out registry keys, which for an arena crew are in the
  arena, so a consumer that both walks the registry and releases crews needs the same care. **2.5.1 note**: it
  hands out copies, in the allocator it is given (ADR 037).
- **Negative**: a pool keeps what it minted. A burst of held crews stays parked, and an arena that served a
  large crew keeps its chunks. The heap below has no way to give them back.
- **Negative**: `agnosai_chan_reopen` knows a private offset.
- **Named, and owned elsewhere** (the issue's *the part that does is named and owned*):
  - **sandhi, 480 B per exchange on a socket, with four response headers.** `sandhi_http_post_opts_a` leaves them
    on the global heap whatever arena it is given: 328 B in `_sandhi_http_recv_framed`'s bare header parse, which
    grows with the response's headers, and 120 B in `_sandhi_client_host_header_a`'s builder.
    `tests/orch_crew_arena.tcyr` holds the crew path to exactly that bound. Owned by sandhi, in `lib/`, and added
    to agnostic's open report there (roadmap B39).
  - **agnostic, about 1.7 KiB per crew.** Its ledger entry and what it clones. Owned by agnostic, which also
    has to adopt this: it builds specs with `agnosai_crew_new` today, so none of this reaches it until it does.
  - **The audit chain**, for a consumer with one: entries, and the run's scratch. A log, and off in agnostic.
  - **Telemetry**, with the exporter on: a span per task and per call, on the global heap.
  - **The binary's REST path.** A crew from `POST /api/v1/crews` is on the global heap, and stays. A route
    reads a registry entry with no hold on it, so eviction cannot release under it. That needs a reader count.
    **2.5.1 note** ([ADR 037](037-the-servers-crews-are-the-orchestrators-to-give-back.md)): it has one. The
    server's crews live in arenas of the orchestrator's own, which gives each back once its registry lets it go
    and no route holds it, and the REST loop reads 0 B. A consumer's crews are as this ADR says.

## Alternatives considered

**A redirect of `alloc` per thread.** Would have spared threading an allocator through 80 functions. `alloc` has
no hook, a definition shadowing it would change every consumer's allocator, and a crew spans threads.

**Move the channel into the arena.** `chan_new` has no `_a` form, so that means rebuilding its layout by hand.
It is more reach into the same structure than one store, and it costs the 2 KiB ring every crew.

**End-of-stream sentinel in place of `chan_close`, so nothing need be reopened.** Two closing modes, and a sentinel
any raw reader of `agnosai_event_sub_chan` would have to know about.

**Release on eviction.** Eviction has no way to know a reader is finished. agnostic's window is its ledger and
not the registry.

**A lease object separate from the spec.** A wider API for one thing: the spec knows its arena, and scrubbing it is
what makes a second release detectable.
