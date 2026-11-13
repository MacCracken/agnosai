# 038 — An audit entry lives in its ring slot

**Status**: Accepted, shipped in 2.5.2
**Date**: 2026-11-13

## Context

The orchestrator's audit chain is a ring of 10,000 entries (`AGNOSAI_AUDIT_MAX_ENTRIES`), and the server mints one.
The ring bounded how many entries the chain kept, not what they cost: the global heap has no `free`, so an entry the
ring let go kept its memory for good, with what came with it.

- **The entry**, its id, its cloned strings, its hash link and its signature.
- **Its metadata**, which the chain kept by pointer, so the caller had to build it to outlive the chain: on the
  global heap, owning copies of its strings (ADR 028).
- **The hashing**: a JSON tree serialized to feed the digest, twice per record once the ring is full, as eviction
  re-anchors on the entry it lets go. The orchestrator's own records hashed on the global heap; the crew runner minted
  a growable scratch per run, also on the global heap, and used it only when running in order, as a wave's workers
  would reset each other's.

Measured on 2.5.1 through the routes, as ADR 037's loop measures: a two-task crew posted, read, snapshotted and listed
kept **6,360 B for good** with the server's chain on, where it kept 0 without one. A crew there records two entries,
its acceptance and its end. Eviction also shifted the ring: `vec_remove(entries, 0)` moved 10,000 pointers per record.

## Decision

**Each entry lives in a slot of the ring, an arena the chain resets for the entry that takes the slot, and everything
the hash and the signature build goes in a scratch that is given back.**

- **The ring is a ring.** Slots and entries are two vecs of the cap's length, filled as the chain grows; past the cap
  a record takes the slot at the head, moves the head, and resets the slot's arena (`arena_allocator_growable`,
  `AGNOSAI_AUDIT_SLOT_BYTES` = 1,024 B, which keeps any chunk it grew). Entries read oldest first from the head. The
  last hash and the anchor `verify` starts from are two 64-byte cells the chain overwrites in place.
- **An entry is built in its slot, whole.** The record path builds a temporary entry in its scratch from the caller's
  strings, hashes it outside the lock, then under the lock copies it into the slot: id, strings, a copy of the
  metadata (`agnosai_json_copy_a`, ADR 037's), the link and the signature. The hash covers what the caller passed and
  `verify` re-hashes the copy, and the two serialize the same.
- **The scratch is the caller's, or a pool's.** `agnosai_audit_record_a` takes one; `agnosai_audit_record` and
  `agnosai_audit_verify` take one from the chain's pool, built eagerly as `src/arena_pool.cyr` requires. The orchestrator,
  the crew runner and the tool loop build their metadata in a pooled scratch (`agnosai_audit_scratch_acquire`), record
  through it and give it back. The runner's per-run scratch is gone, so a parallel or DAG crew hashes off the global
  heap too.
- **`agnosai_audit_chain_recent` hands back copies**, taken under the chain's lock, in the allocator `_a` gives or on
  the global heap. Through 2.5.1 it handed out the chain's own entries, which never went away; a slot is now reset by
  whichever thread records past the cap, so a live entry handed out could change under its reader.
- **The hash chain and `verify` did not change.** The same entry serializes the same, the link and signature are the
  same functions, and `verify` walks the ring from the anchor eviction carried forward.

## Consequences

- **Positive**: with the server's chain on, the REST loop reads **0 B a crew**, and so does an A2A delegation
  (`tests/orch_crew_arena.tcyr`). A full chain records, and verifies, for 0 B through either form
  (`tests/orch_audit.tcyr`). The server's memory for its chain is its ring, where it grew with every crew it ran.
- **Positive**: eviction is constant time, where it shifted the ring by one.
- **Negative**: **a full chain costs its cap times a slot**, whatever its entries need: the server's 10,000 slots of
  1,024 B and their headers measure 11,888,696 B, 1,188 B a slot, held from the 10,000th entry on, and a second wrap
  adds 0. The server's entries need 768 B (a
  crew's acceptance) to 1,000 B (its end); a slot that once held a larger one, a task's long description, keeps the
  chunk it grew.
- **Negative**: **the entry `agnosai_audit_record` returns is valid until the ring takes its slot again**, the cap's
  records later, which a chain other threads write to can reach at once. No caller in the tree keeps one; a reader
  takes copies from `recent`.
- **Negative**: `recent` allocates a copy of every entry it returns, under the chain's lock. It is a reader's call,
  not a recorder's.
- **Neutral**: an audit-less run costs nothing more: the pool is the chain module's, four scratches made once.

## Alternatives considered

- **A fixed slab per slot**, entries written into a fixed number of bytes. An entry's size is its caller's strings
  and metadata, unbounded, so a slab either truncates an audit entry, which a tamper-evident log cannot do, or falls
  back to an arena anyway.
- **One arena for the whole ring, reset when the ring wraps.** The oldest entries would go with the newest, and a ring
  that keeps its last 10,000 entries would sometimes keep one.
- **Serialize each entry and keep only the bytes.** `verify` would parse every entry back, and the accessors would
  answer from a parse; a larger change for no saving over an arena of the same bytes.
- **Keep the caller's metadata and make the caller own its lifetime.** It is what 2.5.1 did; every caller had to put
  it where nothing gives memory back.
