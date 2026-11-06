# The crew registry evicts finished crews no consumer has read — OPEN

**Discovered:** 2026-11-05, by agnostic's 0.2.2 audit. **Severity:** High for a consumer that polls.
**Affects:** agnosai 2.2.3, `_agnosai_orch_evict_locked` (`src/orchestrator/orchestrator.cyr`).
**Status:** 🔴 OPEN.

At `AGNOSAI_MAX_RETAINED_CREWS` (1000) the next submit removes **every** finished crew at once. A
consumer that reads state by polling loses whatever it had not reached: agnostic's collector looks
every 200 ms, and 1,300 unpolled crews at 190/s lost 885, each with its results. Clients polling every
crew to its end still lost 7 of 4,000.

**Ask:** evict oldest-first down to a low-water mark, and add a public call to forget a crew a
consumer has read, such as `agnosai_orchestrator_forget_crew(o, crew_id)`.

**agnostic meanwhile:** `_agnostic_crew_retire` deletes from `AGN_OR_CREWS` under `AGN_OR_MUTEX` once a
crew is terminal and its subscription has closed (agnostic ADR 0022). That reach goes when this ships.
