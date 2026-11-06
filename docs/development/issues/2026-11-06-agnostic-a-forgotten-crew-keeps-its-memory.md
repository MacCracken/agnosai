# A crew a consumer has forgotten still holds about 11 KB, for the life of the process

**Status:** 🟡 **OPEN**
**Placement:** unpinned. agnostic's 0.3.x line carried its half at 0.3.1, and this is what is left.
**Discovered:** 2026-11-05 (agnostic's audit, P2). Measured again 2026-11-06 at agnosai 2.2.4.
**Severity:** Medium. Nothing fails. A long-lived server spends about 11 GB per million crews, and only a restart gives it back.
**Affects:** agnosai 2.2.4.

## Summary

`agnosai_orchestrator_forget_crew` takes a crew out of the registry and frees nothing, because the global heap has no `free`, and its comment says so: "a registry entry is a pointer and nothing here frees it". So eviction bounds how many crews are *reachable*, and not what they cost.

agnostic 0.3.1 recycles the one large thing it owned, its 4,096 B event ring. That took a finished crew from 15,176 B to 11,080 B, measured over 6,000 placeholder crews past the ledger's cap, and it is everything a consumer can do from outside.

## What the number is

| | Bytes a crew | Whose |
|---|---|---|
| Before agnostic 0.3.1 | 15,176 | |
| agnostic's event ring, now recycled | −4,096 | agnostic |
| **Kept for life at agnostic 0.3.1** | **11,080** | |
| of which the subscribe channel | about 2,400 | agnosai |
| of which the crew's spec | about 2,000 | agnosai |

The two rows at the bottom are from the audit at 2.2.3, not from this measurement. A crew with real output costs about 1.6× that output on top.

## Reproduction

In agnostic, a loop of 1,000 `POST /api/v1/crews` with a collector sweep every 64, then wait for the registry to empty, then read `alloc_used()`. Six rounds, on placeholder crews. Steady state is reached by the third.

## The ask

A crew-scoped arena: everything the engine allocates for one crew comes from it, and giving the crew back gives the arena back to a pool.

⚠ **It cannot simply be `forget_crew`.** agnostic keeps reading after it forgets: the spec, for `GET /crews/:id/plan`, and the results, until its outcome is written. Its ledger holds both pointers, and its contract with 2.2.4 is that they stay valid. So this needs a second step, or a copy:

- **A release call**, `agnosai_orchestrator_release_crew(o, state)` or the like, made when the consumer's own entry goes. agnostic would call it at ledger eviction, where it now gives back its ring. It would have to clear its pointers under its ledger lock first, because an evicted entry can still be held by a request.
- **Or the consumer copies what it keeps** into memory of its own at retirement, and `forget_crew` frees. That is simpler here and costs a copy per crew.

## Done means

agnostic's measurement above reads a figure that does not grow with crews served, or the part that does is named and owned.
