# AgnosAI — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures
> (durable); this file is **state** (volatile).
> Last refreshed: 2026-11-13 (the 2.6.0 release gate).

## Now — 2.6.0

| | |
|---|---|
| **version** | **2.6.0** (cut 2026-11-13, `version-bump.sh`'s UTC date): protocol currency. `/mcp` serves MCP 2026-07-28 beside 2025-03-26, deciding per request, by hand after bote's era rules (ADR 039); agnosai speaks A2A 1.0, an Agent Card at `/.well-known/agent-card.json` the operator can sign and `POST /a2a`, whose tasks are crews, streamed over SSE (ADR 040) |
| **cyrius pin** | **6.7.18**, unchanged |
| **tests** | **118 suites, 13,632 assertions, 0 failed** under `CYRIUS_DCE=1` with wasmtime 50.0.1, counted from each suite's printed line as well as its exit code. 2.5.2's tree read 13,307: +325, the two new suites `a2a_v1` (203: the card, the version, every method's answer and refusal, the streams, JCS, the signer, and what a task costs the heap) and `server_routes_mcp_2026` (116: the header view, `server/discover`, the methods and every refusal under 2026-07-28, the earlier revision on the same endpoint, the router, and the heap), and `server_serve` 338 (both protocols through the real handler). **75 mutants**, the first 62 in two groups run side by side in copies of the tree: 54 killed on the first pass, and of the 8 that got through, 7 were gaps each closed by a test and 1 was a status mapping no answer reached, removed. 11 more for the header reads rewritten after the first benchmarks and the stream's backoff: 8 killed, 2 gaps closed, and 1, the stream's poll falling back to 1 ms after a change, which moves latency alone and no functional test can see. 2 for a failed task's error text: 1 gap, closed. Writing them found two things no test made: a body line that looks like a header, and a page boundary that falls on the last task |
| **coverage** | **99%**: 116/116 files, 2,101/2,111 fns (gate `--min 80`), the same 10 unreferenced as 2.5.2. A first run read 2,094/2,111: seven new functions only other code reached, each now named by a test |
| **dist** | `dist/agnosai.cyr` 53,685 lines + `dist/agnosai-guard.cyr` 1,034, both v2.6.0, by `wc -l` (2.5.2's read 51,376 and 1,034). Sidecars unchanged. `[lib]` gains `src/http_headers.cyr`, `src/jcs.cyr` and `src/a2a_v1.cyr`, 125 modules |
| **binary** | `build/agnosai` **3,260,888 B** (2.5.2: 3,178,600). aarch64 **7,307,248 B** (7,237,256), not compacted, built and not started. The native binary answers `/ready` with `"version":"2.6.0"` and exits 0 on SIGTERM, and over real HTTP serves the card, answers an A2A `SendMessage` (with no gateway listening, a failed task whose status names the gateway's error), and an MCP 2026-07-28 `server/discover` |
| **deps** | Unchanged from 2.5.2, and not re-checked against GitHub this release. bote at 3.8.15, whose `src/http_era.cyr` the MCP route follows; hoosh at 2.12.3 |
| **lock** | `cyrius.lock` unchanged: 119 files, 8 commit pins |
| **gates** | `check-symbols.sh`: 3,825 top-level definitions across 126 files, no duplicates, all prefixed. `check-clean.sh`: fmt and lint 263 files, doc 127, doctest 1, vet, deny, `deps --verify` 119, lib snapshot 113, generated sources, log lengths 262, dist matches src. Fuzz 4/4. The example runs. CI replica (ubuntu:26.04, wasmtime 47.0.3, fresh user, all 8 pins from GitHub): **every `ci.yml` step passed**, 118 suites, 13,604 assertions, 0 failed, coverage, the example, fuzz and `cyrius bench` |
| **benchmarks** | **What the protocols cost** (`benches/protocols.bcyr`, new): MCP `tools/list` 3.4 µs under 2025-03-26 and 8.2 µs under 2026-07-28, `tools/call` 8.4 and 13.5 µs, `server/discover` 7.9 µs, the newer revision's larger body 2.4 µs of the difference; A2A's card 22.9 µs per request and 26.3 µs sealed and signed, its canonical form 16.3 µs, an ES256 signature 3.15 ms (once), `SendMessage` 50.8 µs, `GetTask` 7.6 µs, a streamed task 1.28 ms. The first cut read 10.0 µs and 16.6 µs for the 2026-07-28 rows, every header read looking for the end of the headers again, and 50.5 ms a stream, its fixed 50 ms poll. Interleaved against a 2.5.2 build on the same `lib/`, three rounds of `orch`, `server`, `core` and `rest_crews` at a mean load of 4.72: median +0.4% across 158 rows, 141 within 5%, the route rows −3.7% to +2.1% but for the earlier revision's `initialize`, +4.6% and +5.3%, about 0.5 µs. `bench-history.sh` appended 281 rows for 2.6.0 at a starting load of 3.5, among them `mcp_tools_list_2026_07_28` 7.8 µs, `a2a_send_message_1_task` 47.8 µs and `a2a_send_streaming_message_1_task` 1.26 ms; both kept-bytes lines read 0 |
| **memory** | A2A: a task sent and listed, or streamed, leaves **0 B** on the global heap once its crew is evicted, the registry's cap at 8 (`tests/a2a_v1.tcyr`); a sealed card is answered for 0 B. MCP: a 2026-07-28 request costs the global heap what the same method costs 2025-03-26 (`tests/server_routes_mcp_2026.tcyr`). ⚠ **An MCP `tools/call` keeps 601 B a call** in either revision, its tool's input and output built on the global heap: it predates this release, measured here, and is roadmap 2.6.1. Both of `benches/rest_crews.bcyr`'s kept-bytes lines read 0 |
| **interop** | A card signed here verifies with openssl over Python's canonical form of it (`json.dumps(sort_keys=True, separators=(",", ":"))`, RFC 8785's for an ASCII card with no floats): `Verified OK`. The suite checks the same canonical form against Python's, and the signature against sigil's ES256 over the spec's signing input built apart from the signer |
| **build warnings** | Unchanged from 2.5.2: large static data, the DCE note, the two "refusing to overwrite stdlib leaf" lines, and the two `src/tool_loop.cyr` "assigning non-pointer to typed pointer" lines, at 1005 and 1182 |
| **consumer** | Not re-run. agnostic pins 2.2.4; its M7 waits on this release for both protocols, and has the card's signer and verifier in the library for its own card |
| **upstream filings** | ⚠ **Seven not committed**, so not visible upstream, unchanged from 2.5.2: sigil (RSA over 4096 bits; the lane pool), sandhi (bytes per exchange; chunked bodies; the pooled connection's allocator), bote (MCP client), bayan (its escape one builder call a byte) |

### 2.5.2 — the release (2026-11-13)

| | |
|---|---|
| **version** | **2.5.2** (cut 2026-11-13, `version-bump.sh`'s UTC date): the audit chain gives back what it lets go (ADR 038). Each entry lives in a slot of its ring, an arena the chain resets when the ring takes it again, with its strings, its link, its signature and a copy of its metadata; the hashing goes in a scratch that is given back; `agnosai_audit_chain_recent` hands back copies |
| **cyrius pin** | **6.7.18**, unchanged |
| **tests** | **116 suites, 13,307 assertions, 0 failed** under `CYRIUS_DCE=1` with wasmtime 50.0.1, counted from each suite's printed line as well as its exit code. 2.5.1's tree read 13,262: +45, `orch_audit` 99 (from 71: a full chain records and verifies for 0 B, the ring's order, its anchor and its slots across wraps, four recorders and a reader), `orch_crew_arena` 1,389 (the REST loop and A2A with the server's chain on), `orch_tool_loop` 406, `orch_crew_runner` 326 and `orch_approval` 78. **31 mutants, all killed**, in two groups run side by side in copies of the tree: the ring, its copies and its scratch 24, the callers 6, and one for the fix below. 28 were killed on the first pass; the 2 that got through, a reviewer's decision keeping its scratch and a tool call's metadata on the global heap, were each closed by a heap test, and the first of those tests found a decision's spelling minted on the global heap per call, 16 B, fixed |
| **coverage** | **99%**: 113/113 files, 2,071/2,081 fns (gate `--min 80`), the same 10 unreferenced as 2.5.1. `audit.cyr` 19/19 |
| **dist** | `dist/agnosai.cyr` 51,376 lines + `dist/agnosai-guard.cyr` 1,034, both v2.5.2, by `wc -l` (2.5.1's read 51,271 and 1,034). Sidecars unchanged. `[lib]` unchanged, 122 modules |
| **binary** | `build/agnosai` **3,178,600 B** (2.5.1: 3,178,552). aarch64 **7,237,256 B** (7,237,216), not compacted, built and not started. The native binary answers `/ready` with `"version":"2.5.2"` and exits 0 on SIGTERM |
| **deps** | Unchanged from 2.5.1, and not re-checked against GitHub this release. hoosh at 2.12.3 |
| **lock** | `cyrius.lock` unchanged: 119 files, 8 commit pins |
| **gates** | `check-symbols.sh`: 3,652 top-level definitions across 123 files, no duplicates, all prefixed. `check-clean.sh`: fmt and lint 257 files, doc 124, doctest 1, vet, deny, `deps --verify` 119, lib snapshot 113, generated sources, log lengths 256, dist matches src. Fuzz 4/4. The example runs. CI replica (ubuntu:26.04, wasmtime 47.0.3, fresh user, all 8 pins from GitHub): **every `ci.yml` step passed**, 116 suites, 13,277 assertions, 0 failed, coverage, the example, fuzz and `cyrius bench`, run again after three internals this release left without a caller were removed |
| **benchmarks** | Interleaved against a 2.5.1 build on the same `lib/`, three rounds of `orch`, `server`, `rest_crews`, `manager` and `handoff` at a mean load of 2.45: `audit_record` 26.9 → 16.7 µs (−38%), eviction moving the ring's head where it shifted 10,000 pointers; `audit_verify_256` 2.52 → 2.29 ms (−9%); `rest_post_crew_2_tasks_chain_on` (new) 400 → 334 µs (−17%); the orchestrator's crews, whose chain is on, `run_crew_1_task_sequential` −13%, `run_crew_then_release` −13%, `run_crew_then_forget` −12%, `run_crew_at_registry_cap` −10%, `run_crew_10_tasks_sequential` −8%. Median +0.4% across 145 rows, 122 within 5%. ⚠ `run_crew_10_tasks_parallel_4` read +53% and `crew_runner_10_tasks_parallel_4_watched` +63%, the second with no chain at all: run alone, interleaved three times, both read the same on the two trees, and in the full file 2.5.1 read them at 440 and 397 µs where alone it reads 727–757 and 684–748, the row-position effect of roadmap 2.7.1. `bench-history.sh` appended 269 rows for 2.5.2 at a starting load of 4.5, among them `audit_record` 17.2 µs, `audit_verify_256` 2.37 ms and `rest_post_crew_2_tasks_chain_on` 366.5 µs; both kept-bytes lines read 0 |
| **memory** | With the server's audit chain on, `tests/orch_crew_arena.tcyr`'s REST loop reads **0 B a crew**, and an A2A delegation 0 B (the ring at 96 so the loop wraps it). `benches/rest_crews.bcyr` at the binary's 10,000: **2.5.1 kept 9,480 B a crew for good, 2.5.2 keeps 0**; 2.5.1's 6,360 was measured before its ring filled, and past it each record also re-hashed the entry it let go on the global heap. A full chain holds **11,888,696 B**, 1,188 B a slot, and a second wrap adds 0. A server entry needs 768 B (a crew's acceptance) to 1,000 B (its end), inside a slot's 1,024. A full chain records and verifies for 0 B, a reviewer's decision audited 0 B, a tool call with the chain on 0 B |
| **the ring** | `tests/orch_audit.tcyr`: past its cap a record takes the oldest's slot and the chain mints none; `recent` reads from the head, oldest first, and its copies outlive the slots they came from, where the entry `record` returned reads whatever took its slot last; the oldest survivor links to the anchor eviction carried forward, so the chain verifies after ten wraps, and an edit to that survivor fails as entry 0; a cap below 1 keeps 1. Four threads recording into a ring of 16 while a fifth copies it: every copy one whole entry, and the chain verifies |
| **build warnings** | Unchanged from 2.5.1: large static data, the DCE note, the two "refusing to overwrite stdlib leaf" lines, and the two `src/tool_loop.cyr` "assigning non-pointer to typed pointer" lines, now at 1005 and 1182 |
| **consumer** | Not re-run. agnostic pins 2.2.4 and reads no chain entries (its `src/engine/audit.cyr` names the chain's key, in a comment): nothing changes for it. `agnosai_audit_chain_recent` hands back copies now, and an entry `record` returns is valid until the ring takes its slot again |
| **upstream filings** | ⚠ **Seven not committed**, so not visible upstream, unchanged from 2.5.1: sigil (RSA over 4096 bits; the lane pool), sandhi (bytes per exchange; chunked bodies; the pooled connection's allocator), bote (MCP client), bayan (its escape one builder call a byte) |

### 2.5.1 — the release (2026-11-13)

| | |
|---|---|
| **version** | **2.5.1** (cut 2026-11-13, `version-bump.sh`'s UTC date): the server's own crews live in arenas their orchestrator gives back (ADR 037). A crew from `POST /api/v1/crews`, an A2A delegation or the `delegate` tool goes back once its registry lets it go and no route is reading it; every route that reads one holds it and copies what it renders |
| **cyrius pin** | **6.7.18**, unchanged, the latest tag |
| **tests** | **116 suites, 13,262 assertions, 0 failed** under `CYRIUS_DCE=1` with wasmtime 50.0.1, counted from each suite's printed line as well as its exit code. 2.5.0's tree read 13,156: +106, nearly all `orch_crew_arena` (1,380, from 1,288), its REST loop, its holds and its copies, with `server_routes_crews` (211), `server_routes_sse` (40), `server_sse` (59) and `tools_builtin_delegate` (64) grown. **39 mutants, all killed**, in two groups run side by side in copies of the tree: the orchestrator's holds, its give-back and the copy 22, the routes, the stream and the tool 17. The first 37 were killed on the first pass; of the 2 written for the give-back's skipped walk, one got through, a crew forgotten while held whose ended entry no test looked for, closed. Writing them found more first: an answer read once its crew had gone back, which no test made, and a crew read back from its log evicting an owned one |
| **coverage** | **99%**: 113/113 files, 2,068/2,078 fns (gate `--min 80`), the same 10 unreferenced as 2.5.0. A first run read 2,064/2,078: four new functions only the runs reached, each now named by a test |
| **dist** | `dist/agnosai.cyr` 51,271 lines + `dist/agnosai-guard.cyr` 1,034, both v2.5.1, by `wc -l` (2.5.0's read 50,921 and 1,034). Sidecars **6 / 1** leaves, unchanged. `[lib]` unchanged, 122 modules |
| **binary** | `build/agnosai` **3,178,552 B** (2.5.0: 3,166,216). aarch64 **7,237,216 B** (7,237,160), not compacted, built and not started. The native binary answers `/ready` with `"version":"2.5.1"` and exits 0 on SIGTERM |
| **deps** | Unchanged from 2.5.0: every pin on its latest tag. hoosh at 2.12.3 |
| **lock** | `cyrius.lock` unchanged: 119 files, 8 commit pins |
| **gates** | `check-symbols.sh`: 3,641 top-level definitions across 123 files, no duplicates, all prefixed. `check-clean.sh`: fmt and lint 257 files, doc 124, doctest 1, vet, deny, `deps --verify` 119, lib snapshot 113, generated sources, log lengths 256, dist matches src. Fuzz 4/4. The examples run. CI replica (ubuntu:26.04, wasmtime 47.0.3, fresh user, all 8 pins from GitHub): **every `ci.yml` step passed**, 116 suites, 13,232 assertions, 0 failed, coverage, the examples, fuzz and `cyrius bench` |
| **benchmarks** | Interleaved against a 2.5.0 build on the same `lib/`, three rounds of `server` and `rest_crews` at a load of 2.08: `route_create_crew_*` 92.6 → 109.8 µs (+19%), `route_get_crew_*` 17.1 → 21.7 µs (+27%), the dashboard's routes +10–13%; `rest_get_crew_2_tasks` 2.0 → 6.1 µs, `rest_post_crew_2_tasks` 271 → 288 µs; and what a posted crew keeps, 13,144 → 0 B. A first cut walked the ended queue for every crew given back, 40 µs a post at a full registry (`route_create_crew_*` 144 µs), and the walk is skipped for a crew eviction gives back. The wider A/B of six files before that, at 2.42: median +0.0% across 183 rows, 168 within 5%, the crew runner's rows −2% to +2%. `bench-history.sh` appended 268 rows for 2.5.1 at a starting load of 4.1 |
| **memory** | `tests/orch_crew_arena.tcyr`'s REST loop, with the registry's cap at 8 so every post past it evicts: a two-task crew posted, read, snapshotted and listed leaves **0 B on the global heap, where 2.5.0 left 13,144** (measured in a 2.5.0 tree, the same loop); an A2A delegation 0 B, where its field list alone was 264; a refused request 0 B, where validation was 152. Its arenas number the registry's cap plus two at most, and are used again. ⚠ **With the server's audit chain on, a crew still keeps 6,360 B**, its entries past the chain's ring of 10,000, which keep their memory: roadmap 2.5.2 |
| **the holds** | `tests/orch_crew_arena.tcyr`: a crew held past its eviction stays whole until let go and is scrubbed then; one never registered goes back at its last hold; forgotten, it goes back at once, or when let go if held; a consumer's crew is not held and eviction leaves it to its consumer; the registry's ids and a snapshot's are copies; a crew read back from its log at start-up evicts as a registration does; a JSON copy owns its strings; and every route's answer, rendered before its crew is evicted and given back and again after, is the same text. `tests/server_routes_sse.tcyr`: a stream's subscription goes back to the bus, and none is made for a crew that does not exist. `tests/tools_builtin_delegate.tcyr`: a delegated crew is owned, and its id handed back a copy |
| **build warnings** | Unchanged from 2.5.0: large static data, the DCE note, the two "refusing to overwrite stdlib leaf" lines, and the two `src/tool_loop.cyr` "assigning non-pointer to typed pointer" lines, at 1007 and 1184 |
| **consumer** | Not re-run. agnostic pins 2.2.4 and builds its own crews, which are its own to give back (ADR 028): nothing changes for it. `agnosai_orchestrator_crew_ids_a` hands out copies now |
| **upstream filings** | ⚠ **Seven not committed**, so not visible upstream, unchanged from 2.5.0: sigil (RSA over 4096 bits; the lane pool), sandhi (bytes per exchange; chunked bodies; the pooled connection's allocator), bote (MCP client), bayan (its escape one builder call a byte) |

### 2.5.0 — the release (2026-11-13)

| | |
|---|---|
| **version** | **2.5.0** (cut 2026-11-13, `version-bump.sh`'s UTC date): durable execution (ADR 036). A crew can keep a log, its spec, every event numbered, each tool call journaled before it runs and each finished task's result, through a `StateStore` that is now a backend table; a crew a stopped process left mid-run resumes from it, its finished tasks standing and a task whose calls may have taken effect waiting for a reviewer; and a snapshot read under the bus lock lets a reader that fell behind resync. The server keeps logs in `AGNOSAI_STATE_DIR` and resumes before it listens |
| **cyrius pin** | **6.7.18**, unchanged, the latest tag |
| **tests** | **116 suites, 13,156 assertions, 0 failed** under `CYRIUS_DCE=1` with wasmtime 50.0.1, counted from each suite's printed line as well as its exit code. 2.4.3's tree read 12,849: +307, with `orch_crew_log` (97) and `orch_resume` (108) new, and `orch_durable_state` (115, from 83), `orch_lifecycle` (165, from 134), `server_routes_crews` (207) and `server_router` grown. **58 mutants, all killed**, in three groups run side by side in copies of the tree: the store and the log's writing 23, the snapshot and the moves 13, resuming 22. A first pass let six through, each a gap in a test, all closed and each killed when run again: a log made where its directory already was, a write the disk refuses, a move made while the bus lock is held, a snapshot read through the runner rather than the state alone, a DAG resumed past a failure, and a resume's run through the orchestrator. Writing them found five more gaps first, closed before they ran, one of them a bug (below) |
| **coverage** | **99%**: 112/112 files, 2,059/2,069 fns (gate `--min 80`), the same 10 unreferenced as 2.4.3. A first run read 2,040/2,069: nineteen new functions only the runs reached, each now named by a test |
| **dist** | `dist/agnosai.cyr` 50,921 lines + `dist/agnosai-guard.cyr` 1,034, both v2.5.0, by `wc -l` (2.4.3's read 49,219 and 1,034). Sidecars **6 / 1** leaves, unchanged. `[lib]` gains `src/crew_log.cyr`, 122 modules |
| **binary** | `build/agnosai` **3,166,216 B** (2.4.3: 3,107,608). aarch64 **7,237,160 B** (7,170,368), not compacted, built and not started. The native binary answers `/ready` with `"version":"2.5.0"` and exits 0 on SIGTERM |
| **deps** | Unchanged from 2.4.3: every pin on its latest tag. hoosh at 2.12.3 |
| **lock** | `cyrius.lock` unchanged: 119 files, 8 commit pins |
| **gates** | `check-symbols.sh`: 3,616 top-level definitions across 123 files, no duplicates, all prefixed. `check-clean.sh`: fmt and lint 256 files, doc 124, doctest 1, vet, deny, `deps --verify` 119, lib snapshot 113, generated sources, log lengths 255, dist matches src. Fuzz 4/4. The examples run. CI replica (ubuntu:26.04, wasmtime 47.0.3, fresh user, all 8 pins from GitHub): **every `ci.yml` step passed**, 116 suites, 13,126 assertions, 0 failed, coverage, the examples, fuzz and `cyrius bench` |
| **benchmarks** | The interleaved A/B against a 2.4.3 build on the same `lib/`, four rounds of `orch`, `server`, `core`, `tools`, `manager` and `handoff` at a mean load of 3.78: median +0.1% across 181 rows, 145 within 5%. No crew without a log is slower; the `run_crew_*` rows read 2–6% faster, not claimed, as the 2.4.3 side carried the load's spikes. **What a log costs** (`benches/crew_log.bcyr`, new), a four-task crew on the stub model, medians of three at a load near 5: unlogged 199 µs, a store that keeps nothing 380 µs, the file store on tmpfs 490 µs; a snapshot 1.4 µs; a log read back 277 µs. The log's JSON writer: 4 KiB of prose 148 → 14 µs against bayan's. `bench-history.sh` appended 265 rows for 2.5.0 at a starting load of 5.2, among them `crew_log_run_4_task_file_store` 476.2 µs and `crew_snapshot_4_tasks` 1.4 µs |
| **end to end** | Through the binary, with a stub gateway answering in 1.5 s: a server killed (`kill -9`) partway through the second of three tasks, and started again on the same `AGNOSAI_STATE_DIR`, resumed the crew before it listened (`crews recovered from their logs resumed=1`). Task one kept its answer, task two ran again and task three ran, four model calls in all where the uninterrupted crew makes three, the fourth the call the kill cut off. The log read 13 events with no gap across the stop, `0600`, and `GET /api/v1/crews/{id}/snapshot` answered `completed` at `seq` 13. A second server on the same directory exited 1 (`another process holds this state directory`), and a third started once the first had gone |
| **the log and resuming** | `tests/orch_crew_log.tcyr`: the spec first and rebuilt whole, every event a line numbered with no gap though nobody subscribed, a result and the crew's state beside their events, a parallel crew put back by `seq`, the tap off the sender at the end; a tool's intent on disk before the tool runs and before a consumer's journal is asked; a spec, an intent and an event the disk refuses; a managed crew's marks in the log; and the log's writer byte for byte against bayan's, every byte at every place in a word. `tests/orch_resume.tcyr`: a stop staged by cutting a log back, resumed in each process mode; a failed task not retried, in a sequence and in a DAG; an answer asked about again and not paid for; a call whose effect may stand rejected, refused with no gate, and approved; the fold of a hand-written log; the budget; and through the orchestrator recover, restore, resume once, a crew cancelled while it waited, and a log that goes with its crew, forgotten, evicted or released. `tests/orch_lifecycle.tcyr`: snapshots taken throughout sequential, parallel, DAG, gated and managed crews, each checked against the stream, and a move that waits for the bus lock. `tests/orch_durable_state.tcyr`: the four new verbs and a backend of a consumer's own |
| **found on the way** | Two bugs the new tests caught before release, neither shipped: the log's struct prefix `AGN_CL_*` collided with `src/tools/agnos.cyr`'s, and Cyrius takes the last definition, so one of the two allocated by the other's size (`check-symbols.sh` named it; now `AGN_CLOG_*`); and a crew cancelled while it waited for its turn read `pending` in its snapshot, as bayan's `obj_set` appends a key rather than replacing it. `src/orchestrator/durable_state.cyr` hand-rolls a recursive mkdir that `lib/io.cyr`'s `xmkdir_p` now provides, which its header says the stdlib lacks: housekeeping |
| **build warnings** | Unchanged from 2.4.3: large static data, the DCE note, the two "refusing to overwrite stdlib leaf" lines, and the two `src/tool_loop.cyr` "assigning non-pointer to typed pointer" lines, at 1007 and 1184 |
| **consumer** | Not re-run. agnostic pins 2.2.4. It can now resume an `interrupted` crew (its roadmap's F4) by giving its orchestrator a store and calling `agnosai_orchestrator_recover`, and resync its ring from `agnosai_orchestrator_crew_snapshot_a` when it reports `missed` (F3); its ADR 0019's kept outputs are superseded by the log |
| **upstream filings** | ⚠ **Seven not committed**, so not visible upstream: sigil (RSA over 4096 bits; the lane pool), sandhi (bytes per exchange; chunked bodies; the pooled connection's allocator), bote (MCP client), and new, bayan (`2026-11-12-agnosai-json-build-escapes-one-byte-a-call.md`, its escape one builder call a byte) |

### 2.4.3 — the release (2026-11-12)

| | |
|---|---|
| **version** | **2.4.3** (cut 2026-11-12, `version-bump.sh`'s UTC date): a hierarchical crew is managed (ADR 035). Its manager, an agent of its own named by key, assigns each task to its best worker, each worker holds at most its manager's tools, and the manager writes the crew's answer from every worker's; the `delegate` tool bounds the crew it starts by its caller. The 2.4.x line is done |
| **cyrius pin** | **6.7.18**, unchanged, the latest tag |
| **tests** | **114 suites, 12,849 assertions, 0 failed** under `CYRIUS_DCE=1` with wasmtime 50.0.1, counted from each suite's printed line as well as its exit code. 2.4.2's tree read 12,712: +137, with `orch_manager` new at 83, and `server_routes_crews` (198), `tools_builtin_delegate` (60), `orch_budget` (120), `orch_tool_loop` (402), `orch_crew_runner` and `core_task` grown. **43 mutants, all killed**, in three groups run side by side in copies of the tree: the narrowing, the assignment, the record, the manager's answer and `invalid_manager` 24, the route 8, and the delegate tool, the budget left and the loop's caller 11. A first pass let one through, a gap in a new test (a crew with less left than its task), closed |
| **coverage** | **99%**: 111/111 files, 2,012/2,022 fns (gate `--min 80`), the same 10 unreferenced as 2.4.2. A first run read 2,010/2,022: `agnosai_log_withheld` and `agnosai_crew_req_manager`, which only the runs reached, each now named by a test |
| **dist** | `dist/agnosai.cyr` 49,219 lines + `dist/agnosai-guard.cyr` 1,034, both v2.4.3, by `wc -l` (2.4.2's read 48,843 and 1,034). Sidecars **6 / 1** leaves, unchanged. `[lib]` unchanged, 121 modules |
| **binary** | `build/agnosai` **3,107,608 B** (2.4.2: 3,098,888). aarch64 **7,170,368 B** (7,169,832), not compacted, built and not started. The native binary answers `/ready` with `"version":"2.4.3"` and exits 0 on SIGTERM |
| **deps** | Unchanged from 2.4.2: every pin on its latest tag. hoosh at 2.12.3 |
| **lock** | `cyrius.lock` unchanged: 119 files, 8 commit pins |
| **gates** | `check-symbols.sh`: 3,509 top-level definitions across 122 files, no duplicates, all prefixed. `check-clean.sh`: fmt and lint 251 files, doc 123, doctest 1, vet, deny, `deps --verify` 119, lib snapshot 113, generated sources, log lengths 250, dist matches src. Fuzz 4/4. The examples run. CI replica (ubuntu:26.04, wasmtime 47.0.3, fresh user, all 8 pins from GitHub): **every `ci.yml` step passed**, 114 suites, 12,819 assertions, 0 failed, coverage, the examples, fuzz and `cyrius bench` |
| **benchmarks** | The interleaved A/B against a 2.4.2 build on the same `lib/`, four rounds of `orch`, `server`, `core`, `tools` and `manager` at a mean load of 4.06: median +1.1% across 177 rows, 152 within 5%. **What a manager costs** (`benches/manager.bcyr`, new), against 2.4.2, medians of three interleaved: `run_crew_4_task_hierarchical` 181.0 → 241.5 µs (+33%), its manager's answer a fifth call, where 2.4.2 ran the crew sequentially; the same crew sequential 182.8 → 181.3 µs; the narrowing 1.1 µs. ⚠ **2.4.2's hand-off row measured crews that failed**, for want of a model catalog: with one, against 2.4.1, a DAG chain of four 721 → 729 µs and a sequential chain of four 238 → 254 µs, about 5 µs a hand-off. Every crew row of the two new files now exits unless its crew completed. `bench-history.sh` appended 260 rows for 2.4.3 at a starting load of 2.5, among them `run_crew_4_task_hierarchical` 241.9 µs, `run_crew_4_task_sequential` 184.2 µs and the chain, now real, 778.0 µs |
| **the manager** | `tests/orch_manager.tcyr`: a worker within its manager's tools, on a copy, and what it lost; the manager found by key and never a worker; each task by the best worker, matching `agnosai_explain_selection_a` over the workers and the Rust line's `delegate_tasks`; the writer offered and told of no withheld tool; the manager's answer, last, by the manager, offered no tool, handed every answer in order and told every task; a manager alone; no answer after a failure, once the budget is spent, or once cancelled; the events, `task_count` 4; and `invalid_manager`, through the runner and the orchestrator. `tests/server_routes_crews.tcyr`: the three refusals, a managed crew through the route, and the result fields. `tests/tools_builtin_delegate.tcyr`: the delegated agent within its caller, the ceiling and the refusal |
| **build warnings** | Unchanged from 2.4.2: large static data, the DCE note, the two "refusing to overwrite stdlib leaf" lines, and the two `src/tool_loop.cyr` "assigning non-pointer to typed pointer" lines, now at 1007 and 1184 |
| **consumer** | Not re-run. agnostic pins 2.2.4. It can now accept `hierarchical` (its roadmap's F2), with a `manager` its request must name. Its `dag` crews hand dependents their answers since 2.4.2 |
| **upstream filings** | ⚠ **Six not committed**, so not visible upstream, unchanged: sigil (RSA over 4096 bits; the lane pool), sandhi (bytes per exchange; chunked bodies; the pooled connection's allocator), bote (MCP client) |

### 2.4.2 — the release (2026-11-12)

| | |
|---|---|
| **version** | **2.4.2** (cut 2026-11-12, `version-bump.sh`'s UTC date): a dependent task is handed its dependencies' answers, as data (ADR 034), in a DAG and in a sequential crew. The roadmap had it as a summarizing strategy for `memory.cyr`'s buffer, but nothing had ever been handed over: the DAG read none of its results |
| **cyrius pin** | **6.7.18**, unchanged, the latest tag |
| **tests** | **113 suites, 12,712 assertions, 0 failed** under `CYRIUS_DCE=1` with wasmtime 50.0.1, counted from each suite's printed line as well as its exit code. 2.4.1's tree read 12,631: +81, all `orch_handoff`, new. No suite pinned a dependent task's request, so none changed. **22 mutants, all killed** on the first pass, in two groups run side by side in copies of the tree: the hand-off's rules 14 (the cap, the cut, the order, once each, no answer, the markers, the finding, the line it opens with) and the request and both runners 8 (the turn, its reply, its role, each runner setting and taking back). Four crashed the suite rather than failing it, each as expected: a null handed to the builder, or a message read that a mutant took away |
| **coverage** | **99%**: 111/111 files, 2,000/2,010 fns (gate `--min 80`), the same 10 unreferenced as 2.4.1. A first run read 1,999: `agnosai_handoff_render_a`, which only the runners called, now named by a test that builds a hand-off in an arena |
| **dist** | `dist/agnosai.cyr` 48,843 lines + `dist/agnosai-guard.cyr` 1,034, both v2.4.2, by `wc -l` (2.4.1's read 48,680 and 1,034). Sidecars **6 / 1** leaves, unchanged. `[lib]` gains `src/handoff.cyr`, 121 modules |
| **binary** | `build/agnosai` **3,098,888 B** (2.4.1: 3,094,352). aarch64 **7,169,832 B** (7,169,384), not compacted, built and not started. The native binary answers `/ready` with `"version":"2.4.2"` and exits 0 on SIGTERM |
| **deps** | Unchanged from 2.4.1: every pin on its latest tag. hoosh at 2.12.3 |
| **lock** | `cyrius.lock` unchanged: 119 files, 8 commit pins |
| **gates** | `check-symbols.sh`: 3,481 top-level definitions across 122 files, no duplicates, all prefixed. `check-clean.sh`: fmt and lint 249 files, doc 123, doctest 1, vet, deny, `deps --verify` 119, lib snapshot 113, generated sources, log lengths 248, dist matches src. Fuzz 4/4. The examples run. CI replica (ubuntu:26.04, wasmtime 47.0.3, fresh user, all 8 pins from GitHub): **every `ci.yml` step passed**, 113 suites, 12,681 assertions, 0 failed, coverage, the examples, fuzz and `cyrius bench` |
| **benchmarks** | The interleaved A/B against a 2.4.1 build on the same `lib/`, four rounds of `orch`, `server`, `core` and `tools` at a mean load of 4.36: median −1.7% across 175 rows, 145 within 5%; the sequential crews −2.7% to −3.8%. **What a hand-off costs** is `benches/handoff.bcyr`, new: one short answer 5.5 µs, three of 4 KiB 1.5 ms, two thirds of that the injection scan (roadmap 2.7.2), and a four-task DAG chain 211 → 203 µs against 2.4.1, medians of four interleaved. ⚠ The chain is a file of its own because, run last in `orch.bcyr`, it read 550 → 800 µs, and 817 µs with the render row taken out: the heap's position (roadmap 2.7.1). `bench-history.sh` appended 257 rows for 2.4.2, at a starting load of 4.5, not used for any claim. ⚠ **Corrected in 2.4.3**: the chain row's tasks failed for want of a model catalog, so it measured no hand-off; the real cost is in 2.4.3's CHANGELOG *Fixed* |
| **the hand-off** | `tests/orch_handoff.tcyr`: one hand-off byte for byte; the order a task names its dependencies, each once; no answer for a failed or absent one, and not the failure's text; an answer and an id that cannot end the markers; a finding said outside them, before its block; the cut at the cap, on a character boundary; an arena. Crews through a scripted gateway that keeps each task's last request in its own slot, so a wave's threads can share it: a DAG chain and a diamond, a dependency that failed, a sequential crew handed what came before and told what has not, a parallel crew handed nothing, the context before the hand-off, and a hostile answer. Every hand-off is taken back when the run ends |
| **build warnings** | Unchanged from 2.4.1: large static data, the DCE note, the two "refusing to overwrite stdlib leaf" lines, and the two `src/tool_loop.cyr` "assigning non-pointer to typed pointer" lines, at 997 and 1173 |
| **consumer** | Not re-run. agnostic pins 2.2.4. ⚠ Its `dag` crews that name dependencies now hand each task those answers, which its reviewer preset waits on (its roadmap, "A reviewer preset"); its other `dag` crews pay for answers they only ordered by (CHANGELOG *Behaviour changes*) |
| **upstream filings** | ⚠ **Six not committed**, so not visible upstream, unchanged: sigil (RSA over 4096 bits; the lane pool), sandhi (bytes per exchange; chunked bodies; the pooled connection's allocator), bote (MCP client) |

### 2.4.1 — the release (2026-11-12)

| | |
|---|---|
| **version** | **2.4.1** (cut 2026-11-12, `version-bump.sh`'s UTC date): budgets, caps and crew concurrency are enforced (ADR 033). Crews wait their turn, a parallel crew is a sliding window, outbound https is capped at 64, and token and cost are held per tenant, crew and task, each under the one above it |
| **cyrius pin** | **6.7.18**, unchanged, the latest tag |
| **tests** | **112 suites, 12,631 assertions, 0 failed** under `CYRIUS_DCE=1` with wasmtime 50.0.1, counted from each suite's printed line as well as its exit code. 2.4.0's tree read 12,192: +439, with `orch_caps` (142), `semaphore` (108) and `tls_permit` (34) new and `orch_lifecycle` at 134 from 107. **77 mutants, all killed**, in six groups run side by side in copies of the tree: the semaphore 12, crews' turns, tenants and budgets 29, the sliding window 7, TLS permits 12, trackers, tool limits and the wire 15, and the cancel's wake 2. A first pass let one through, a gap in a new test (a subscriber that attached mid-task heard that task's end), closed. Writing them found three more gaps first, each closed before they ran |
| **coverage** | **99%**: 110/110 files, 1,995/2,005 fns (gate `--min 80`), the same 10 unreferenced as 2.4.0. A first run read 1,984: eleven new functions only the suites' paths reached, each now named by a test |
| **dist** | `dist/agnosai.cyr` 48,680 lines + `dist/agnosai-guard.cyr` 1,034, both v2.4.1, by `wc -l` (2.4.0's read 47,506 and 1,034). Sidecars **6 / 1** leaves, unchanged. `[lib]` gains `src/semaphore.cyr` and `src/tls_permit.cyr`, 120 modules |
| **binary** | `build/agnosai` **3,094,352 B** (2.4.0: 3,073,192). aarch64 **7,169,384 B** (7,103,176), not compacted, built and not started. The native binary answers `/ready` with `"version":"2.4.1"` and exits 0 on SIGTERM |
| **deps** | Unchanged from 2.4.0: every pin on its latest tag. hoosh at 2.12.3 |
| **lock** | `cyrius.lock` unchanged: 119 files, 8 commit pins |
| **gates** | `check-symbols.sh`: 3,471 top-level definitions across 121 files, no duplicates, all prefixed. `check-clean.sh`: fmt and lint 247 files, doc 122, doctest 1, vet, deny, `deps --verify` 119, lib snapshot 113, generated sources, log lengths 246, dist matches src. Fuzz 4/4. The examples run. CI replica (ubuntu:26.04, wasmtime 47.0.3, fresh user, all 8 pins from GitHub): **every `ci.yml` step passed**, 112 suites, 12,599 assertions, 0 failed, coverage, the examples, fuzz and `cyrius bench` |
| **benchmarks** | The interleaved A/B against a 2.4.0 build on the same `lib/`, four rounds of `orch`, `server`, `core`, `tools` and `llm` at a mean load of 3.16: median −0.2% across 192 rows, 164 within 5%. `run_crew_10_tasks_parallel_4_traced` 687.4 → 608.1 µs (−11.5%, every round), the sliding window; the other parallel rows 4–8% faster; `orchestrator_new` 4.3 → 5.4 µs (+24.3%, real). `bench-history.sh` appended 254 rows for 2.4.1, among them `tls_permit_take_give_https` 142 ns, from 1,450 in the A/B before a free permit was taken without reading the clock, and `budget_check_and_record_3_levels` 42 ns |
| **the caps** | `tests/orch_caps.tcyr`: a crew past the cap waits `pending` and runs when a turn frees, in the order it came, inline as well as submitted; its deadline starts at its run; a cancel ends its wait at once and it never runs. Crew, task and orchestrator budgets, a refused call that is not retried though its message holds `429`, tenants spent, re-registered and reset, a tenant at its crews, and the task cap. `tests/orch_lifecycle.tcyr`: the sliding window, a reviewed task giving its slot back once, a late subscriber hearing no end without its start, and a crew its budget stopped, each a legal stream |
| **build warnings** | Unchanged from 2.4.0: large static data, the DCE note, the two "refusing to overwrite stdlib leaf" lines, and the two `src/tool_loop.cyr` "assigning non-pointer to typed pointer" lines, now at 997 and 1173 |
| **consumer** | Not re-run. agnostic pins 2.2.4. ⚠ It passes `max_concurrent_tasks` 10, now enforced: its 11th crew waits. Its `high` tasks wait for a reviewer since 2.4.0. It can now bring back `AGNOSTIC_CREW_MAX_CONCURRENT_TASKS` and give each crew its tenant (CHANGELOG *Behaviour changes*) |
| **upstream filings** | ⚠ **Six not committed**, so not visible upstream: sigil (RSA over 4096 bits; the lane pool), sandhi (bytes per exchange; chunked bodies; the pooled connection's allocator), bote (MCP client) |

### 2.4.0 — the release (2026-11-12)

| | |
|---|---|
| **version** | **2.4.0** (cut 2026-11-12, `version-bump.sh`'s UTC date): the crew lifecycle is one state machine, written down in ADR 032 and checked as an invariant. Every event is numbered, a high-risk task's answer waits for a reviewer, and a crew that cannot run ends `failed` |
| **cyrius pin** | **6.7.18**, unchanged, the latest tag |
| **tests** | **109 suites, 12,192 assertions, 0 failed** under `CYRIUS_DCE=1` with wasmtime 50.0.1, counted from each suite's printed line as well as its exit code. 2.3.8's tree read 11,989: +203, `orch_lifecycle` new at 107. **64 mutants, all killed**, in five bites: the never-ran crew and its reason 19, `seq` 12, completion order 2, the approval gate 25, tool-call events 6. A first pass let four through, each a gap in a new test (a cancelled cyclic crew's last word, the orchestrator's timeout state and its audit carrying the reason, a tool call that fails), and all were closed. A full `cyrius test` leaves nothing in `/tmp` |
| **coverage** | **99%**: 108/108 files, 1,936/1,946 fns (gate `--min 80`), the same 10 unreferenced as 2.3.8 |
| **dist** | `dist/agnosai.cyr` 47,506 lines + `dist/agnosai-guard.cyr` 1,034, both v2.4.0, by `wc -l` (2.3.8's read 46,930 and 1,034). Sidecars **6 / 1** leaves, unchanged |
| **binary** | `build/agnosai` **3,073,192 B** (2.3.8: 3,056,320). aarch64 **7,103,176 B** (7,102,688), not compacted, built and not started. The native binary answers `/ready` with `"version":"2.4.0"` and exits 0 on SIGTERM |
| **deps** | Unchanged from 2.3.8: every pin on its latest tag. hoosh at 2.12.3 |
| **lock** | `cyrius.lock` unchanged: 119 files, 8 commit pins |
| **gates** | `check-symbols.sh` 3,383 definitions across 119 files; `check-clean.sh` fmt 242, lint 242, doc 120, doctest 1 files; `deps --verify` 119; lib snapshot 113; log lengths; dist. Then build, aarch64, tests under DCE, coverage, the example built and run, fuzz 4/4, `distlib --all --check`. **The `ubuntu:26.04` CI replica ran** on the bumped tree laid over a clean copy, as a fresh user, wasmtime 47.0.3, all 8 pins from GitHub: every `ci.yml` step passed, 109 suites, 12,048 assertions, 0 failed, `orch_lifecycle` all 107, and `cyrius bench` |
| **benchmarks** | The interleaved A/B against a 2.3.8 build on the same `lib/`, four rounds of `orch`, `server`, `core` and `tools` at a mean load of 1.55: **median +0.0% across 170 rows**, 164 within 5%. `orchestrator_new` +33.5%, 3.2 → 4.3 µs, the gate each orchestrator mints. `task_four_tool_calls_at_once` and `_kept` −38%, not claimed: alone in a process both builds read about 250 µs, three runs each, so it is what the rows before them leave (roadmap 2.7.1). `bench-history.csv` has 249 rows at 2.4.0, one sweep at a load of 1.1–1.8: median −0.8% against 2.3.8's rows, not claimed |
| **the lifecycle** | `tests/orch_lifecycle.tcyr` replays every stream it runs through ADR 032's table, and is first shown streams it must refuse: a task event before `crew_started`, a `task_completed` for a task never started, a `seq` gap, a failed crew with no reason. Every process mode, a failing branch, a cancel and a deadline mid-run, a cancel before the first task, a cycle, 24 tasks on 8 workers, a subscriber that attaches mid-run, approvals in each outcome, and tool calls ok, failing and refused |
| **approvals** | **End to end through the binary**: a crew posted with a `high` task waited, `GET /api/v1/approvals` listed it, `POST /api/v1/approvals` approved it and the crew completed with its answers; rejected, it failed `task_failed` with `output withheld: approval rejected (reviewer)`. In the suites: the registry reads `awaiting_approval` while a task waits and `running` once it is decided, a cancel ends a wait at once, a deadline times the crew out, no gate rejects at once, `medium` only when asked, and every decision is audited |
| **build warnings** | Unchanged from 2.3.8: large static data, the DCE note, the two "refusing to overwrite stdlib leaf" lines, and the two `src/tool_loop.cyr` "assigning non-pointer to typed pointer" lines |
| **consumer** | Not re-run. agnostic pins 2.2.4. ⚠ A `high` task waits for a reviewer, a crew that cannot run reads `failed`, `task_completed` comes in completion order, events carry `seq` (CHANGELOG *Behaviour changes*). agnostic accepts `risk` from its users, so its `high` tasks will wait on `agnosai_orchestrator_approval_gate(o)` once it re-pins |
| **upstream filings** | ⚠ **Six not committed**, so not visible upstream: sigil (RSA over 4096 bits; the lane pool), sandhi (bytes per exchange; chunked bodies; the pooled connection's allocator), bote (MCP client) |

### 2.3.8 — the release (2026-11-12)

| | |
|---|---|
| **version** | **2.3.8** (cut 2026-11-12, by `version-bump.sh`'s UTC date): a process or Python tool whose policy denies the network loses it, and a chat call sends `traceparent`, so one request reads as one trace from a crew to the provider. The 2.3.x line is done |
| **cyrius pin** | **6.7.18**, unchanged, the latest tag |
| **tests** | **108 suites, 11,989 assertions, 0 failed** under `CYRIUS_DCE=1` with wasmtime 50.0.1, counted from each suite's printed line as well as its exit code. 2.3.7's tree read 11,917: +72, in `sandbox_confine` (144), `telemetry_wiring` (138) and `sandbox_wasm` (147, from 145). 20 mutants, all killed: 16 on the network, 4 on the header. A first pass let two through, both gaps in new tests (a `with_path` confinement denied the network; the older spawn form keeping it), closed. **A full `cyrius test` leaves nothing in `/tmp` again**: 2.3.7's left four files (CHANGELOG *Fixed*), measured on both trees |
| **coverage** | **99%**: 108/108 files, 1,918/1,928 fns (gate `--min 80`), the same 10 unreferenced as 2.3.7 |
| **dist** | `dist/agnosai.cyr` 46,930 lines + `dist/agnosai-guard.cyr` 1,034, both v2.3.8, by `wc -l` (2.3.7's read 46,804 and 1,034). Sidecars **6 / 1** leaves, unchanged |
| **binary** | `build/agnosai` **3,056,320 B** (2.3.7: 3,056,208). aarch64 **7,102,688 B** (7,102,576), not compacted, built and not started. The native binary answers `/ready` with `"version":"2.3.8"` and exits 0 on SIGTERM |
| **deps** | **None moved.** Checked against GitHub at the start of this release: sigil 3.13.15, bote 3.8.15, majra 2.13.0, kavach 3.14.9, ai-hwaccel 2.5.6, tyche 1.2.2, and libro 2.11.3 and patra 1.16.6 through them, each its latest tag. cyrius 6.7.18 is the latest. hoosh, which agnosai talks to and does not pin, is at 2.12.3 |
| **lock** | `cyrius.lock` unchanged: 119 files, 8 commit pins, re-verified |
| **gates** | `check-symbols.sh` 3,336 definitions across 119 files; `check-clean.sh` fmt 241, lint 241, doc 120, doctest 1 files; `deps --verify` 119; lib snapshot 113; log lengths 240 files; dist. Then build, aarch64, tests under DCE, coverage, the example built and run, fuzz 4/4, `distlib --all --check`. **The `ubuntu:26.04` CI replica ran** on the bumped tree laid over a clean copy, as a fresh user, wasmtime 47.0.3, all 8 pins from GitHub: every `ci.yml` step passed, 108 suites, 11,845 assertions, 0 failed, and `cyrius bench`. `sandbox_confine` ran all 144 inside docker: the network denied under docker's seccomp profile, and the old-kernel child's own filter loaded |
| **benchmarks** | The interleaved A/B against a 2.3.7 build on the same `lib/`, four rounds of `sandbox`, `llm`, `tools` and `orch` at a mean load of 2.35: **median −0.1% across 143 rows**, 140 within 5%. `tool_registry_has_50_miss` +60.7%, bimodal on both sides (2.3.7's rounds 91, 91, 92, 236 ns) and unchanged code, not claimed. **The denied network costs a spawn about 57 µs**: `spawn_true_confined_no_network` 1.297 ms against `spawn_true_confined` 1.240 ms in one binary (+4.6%). A Python tool call does not show it, 36.24 → 36.03 ms. `bench-history.csv` has 249 rows at 2.3.8, the two new rows included, one sweep at a load of about 2–2.2: median +6.9% against 2.3.7's rows, where the same-`lib/` A/B reads −0.1%, so not claimed |
| **the network** | A process or Python tool denied the network, measured from inside the child on Linux 7.2.14 (landlock ABI 10): a TCP connect, a TCP bind and a UDP socket read EACCES, a unix socket opens, and a tool allowed the network connects. In a work directory and in a `with_path` confinement too. The manager's process tier, the shell entry point and a Python tool on the loop's path, all denied. **A kernel before ABI 4, measured on this one**: a forked child of the suite hides landlock behind a seccomp filter answering ENOSYS, and its default confinement warns, says it cannot deny, and a denied child connects. A process call still keeps 0 B in its arena, asserted |
| **the trace** | **End to end against hoosh 2.12.3** on loopback, the agnosai binary and hoosh both exporting OTLP to a stub collector, hoosh calling a stub provider: a crew posted to `/api/v1/crews` gave `invoke_workflow` → `invoke_agent` → `chat` → hoosh's `inference` in one trace, hoosh's span the chat span's child with an id of its own, no span id repeated. The 2.3.7 binary against the same hoosh: hoosh's span a root in a trace of its own. In the suite: the header names the chat span, flags `00` reach the gateway as `00`, a root chat sends its own trace, telemetry off sends none |
| **build warnings** | Unchanged from 2.3.7: large static data, the DCE note, the two "refusing to overwrite stdlib leaf" lines, and **two** `src/tool_loop.cyr` "assigning non-pointer to typed pointer" lines, at 895 and 1070, which the 2.3.7 tree prints too and this row did not list. They date from 2.3.0's tool loop |
| **consumer** | Not re-run. agnostic pins 2.2.4, and calls none of the process, Python, spawn or manager functions directly, checked in its `src/`. ⚠ A process or Python tool loses the network, and a chat call sends `traceparent` (CHANGELOG *Behaviour changes*) |
| **upstream filings** | ⚠ **Six not committed**, so not visible upstream: sigil (RSA over 4096 bits; the lane pool), sandhi (bytes per exchange; chunked bodies; the pooled connection's allocator), bote (MCP client) |

### 2.3.7 — the release (2026-11-08)

| | |
|---|---|
| **version** | **2.3.7** (cut 2026-11-08): the Rust line's types have their JSON forms, written and parsed, and its log events have their fields. A task's metadata has `latency_ms`, and a failed remote download says why |
| **cyrius pin** | **6.7.18**, unchanged |
| **tests** | **108 suites, 11,917 assertions, 0 failed** under `CYRIUS_DCE=1` with wasmtime 50.0.1, counted from each suite's printed line as well as its exit code. 2.3.6's tree read 11,710. `log_fields` is new at 23. 88 mutants, all killed. A first pass let four through: three gaps in new tests, closed, and one guard that could not matter, removed |
| **coverage** | **99%**: 108/108 files, 1,912/1,922 fns (gate `--min 80`), the same 10 unreferenced as 2.3.6 |
| **dist** | `dist/agnosai.cyr` 46,804 lines + `dist/agnosai-guard.cyr` 1,034, both v2.3.7, by `wc -l` (2.3.6's read 45,951 and 1,034). Sidecars **6 / 1** leaves, unchanged |
| **binary** | `build/agnosai` **3,056,208 B** (2.3.6: 3,050,920). aarch64 **7,102,576 B** (7,101,384), not compacted, built and not started. The native binary answers `/ready` with `"version":"2.3.7"` and exits 0 on SIGTERM |
| **deps** | Unchanged: kavach 3.14.9, and every other dep on its latest tag, checked at the start of this release |
| **lock** | `cyrius.lock` unchanged: 119 files, 8 commit pins |
| **gates** | `check-symbols.sh` 3,328 definitions across 119 files; `check-clean.sh` fmt 241, lint 241, doc 120, doctest 1 files; `deps --verify` 119; lib snapshot 113; log lengths 240 files; dist. fmt and lint on every test, bench and fuzz file. Then build, aarch64, every test and fuzz unit compiles, tests under DCE, coverage, the example, fuzz 4/4, `distlib --all --check`. **The `ubuntu:26.04` CI replica ran** on the bumped tree with this release's new files laid over a clean clone, wasmtime 47.0.3, all 8 pins from GitHub: every `ci.yml` step, 108 suites, 11,764 assertions, 0 failed, and `cyrius bench`, seven assertions before the last test-only additions |
| **benchmarks** | The interleaved A/B against a 2.3.6 build on the same `lib/`, four rounds of `tools`, `orch`, `server` and `fleet` at a mean load of 2.5: **median +0.1% across 167 rows**, 165 within 5%. `tool_registry_has_50_miss` +15.6% and `is_safe_url_octal_host` −5.7%, the first of which moved −38.6% and +1.1% across 2.3.6's two runs with no change to it. `agnosai_relay_receive` 1.67 → 1.76 µs, the sender's last sequence read so a gap can be logged. `bench-history.csv` has 247 rows at 2.3.7, one sweep after the CI replica at a load of about 1–2.5: median −5.6% against 2.3.6's rows, not claimed |
| **JSON forms** | Writers for `NodeInfo`, `ClusterInfo`, `DeviceLink`, `NodeTopology`, `RelayMessage`, `RetryConfig`, `ToolInput`, the package and tool manifests and `KavachToolResult`, each in the caller's allocator with 0 B on the global heap, asserted. Parsers for `TaskProfile`, `BudgetExceeded`, `CachedPlan`, `ToolSchema`, `ParameterSchema` and `ToolOutput`, each refusing what the type could not have written |
| **log fields** | Asserted through a capturing hook. A relay duplicate is told from a message for another node. Two of the Rust line's fleet events belonged to `select_cheapest_model`, which ADR 030 removed. `wasm_tool.rs` logged nothing. A task id cannot write a line of log of its own |
| **build warnings** | Unchanged: large static data, the DCE note, the two "refusing to overwrite stdlib leaf" lines |
| **consumer** | Not re-run. agnostic pins 2.2.4. ⚠ A remote-registry failure's text gained a reason, and a task's metadata gained `latency_ms` (CHANGELOG *Behaviour changes*) |
| **upstream filings** | ⚠ **Six not committed**, so not visible upstream: sigil (RSA over 4096 bits; the lane pool), sandhi (bytes per exchange; chunked bodies; the pooled connection's allocator), bote (MCP client) |

### 2.3.6 — the release (2026-11-08)

| | |
|---|---|
| **version** | **2.3.6** (cut 2026-11-08): a load test keeps one connection per simulated user, and a target that accepts and never answers no longer holds a call for good. A tool holds no capabilities, even under a root server. The URL check is off the global heap, and the tool list and a task's keys are in byte order |
| **cyrius pin** | **6.7.18**, unchanged |
| **tests** | **107 suites, 11,710 assertions, 0 failed** under `CYRIUS_DCE=1` with wasmtime 50.0.1, counted from each suite's printed line as well as its exit code. 2.3.5's tree read 11,347. `tools_builtin_load_testing` +245, `done_word` is new at 71. 42 mutants, all killed, the capability one in a container as uid 0. A first pass let two through, each a gap in a new test, and both were closed. ⚠ **The first push failed GitHub's CI**, in `sandbox_wasm`: 1e9 fuel lasts under 300 ms on that runner and 476 ms here, so a spinning guest ran out of fuel before its deadline. Reproduced by cutting the budget to a quarter, fixed in the suite alone. This box and its replica cannot show it |
| **coverage** | **99%**: 107/107 files, 1,862/1,872 fns (gate `--min 80`), 10 unreferenced, one fewer than 2.3.5 |
| **dist** | `dist/agnosai.cyr` 45,951 lines + `dist/agnosai-guard.cyr` 1,034, both v2.3.6, by `wc -l` (2.3.5's read 45,750 and 1,034). Sidecars **6 / 1** leaves, unchanged |
| **binary** | `build/agnosai` **3,050,920 B** (2.3.5: 3,025,400; kavach 3.14.9 is about 21 KB of that). aarch64 **7,101,384 B** (7,034,912), not compacted, built and not started. The native binary answers `/ready` with `"version":"2.3.6"` and exits 0 on SIGTERM |
| **deps** | **kavach 3.14.7 → 3.14.9.** Every other dep was already on its latest tag, checked at the start of this release. hoosh is at 2.10.3, which agnosai talks to and does not pin |
| **lock** | `cyrius.lock` re-resolved: 119 files, 8 commit pins, kavach's lines the only change |
| **gates** | `check-symbols.sh` 3,253 definitions across 118 files; `check-clean.sh` fmt 239, lint 239, doc 119, doctest 1 files; `deps --verify` 119; lib snapshot 113; log lengths 238 files; dist. fmt and lint on every test, bench and fuzz file. Then build, aarch64, every test and fuzz unit compiles, tests under DCE, coverage, the example, fuzz 4/4, `distlib --all --check`. **The `ubuntu:26.04` CI replica ran** on the bumped tree with this release's new files laid over a clean clone, wasmtime 47.0.3, all 8 pins from GitHub: every `ci.yml` step, 107 suites, 11,565 assertions, 0 failed, and `cyrius bench`, one assertion before the suite-only fix above |
| **benchmarks** | The interleaved A/B against a 2.3.5 build on the same `lib/`: five rounds of `tools`, `orch` and `server` at a mean load of 1.5, **median −0.2% across 151 rows**, then four rounds of `tools` once the list was fixed, at 2.4, median +0.5% across 21. `load_test_request_kept` 37.4 µs and `load_test_request_new_connection` 127.5 µs. `tool_registry_list_50` −3.6% and `_500` −2.5%, in name order. A sort per list read +83.5% and +139.4%, so the registry keeps the order. `bench-history.csv` has 247 rows at 2.3.6, one sweep after the CI replica at a load of 2–2.5: median +1.8% against 2.3.5's rows, taken at 1.8–2.3, not claimed |
| **a load test** | One connection per user, on a loopback server through sandhi's pool. Asserted there: the status counted, the connections opened, a 404, a chunked body, a close with and without warning, a timeout, a target that accepts and never answers. A run leaves 0 B per user (it was 4,120). A kept request leaves 568 B on the global heap against 312 B for a new one, sandhi's pool building its route key there for each take and put. Filed |
| **a silent target** | Fixed on all four paths that set only sandhi's total deadline: a 150 ms budget was still blocked at 300 s, and now times out on time. Measured for the load tester, the guarded fetch and the agnos tools |
| **capabilities** | A confined tool as uid 0 in a container reads `CapEff`, `CapPrm`, `CapBnd` and `CapAmb` all zero. With `--cap-drop=SETPCAP` every confined tool is refused, exit 116. kavach's concurrent stderr fix measured at 3.14.9: 0 of 600 runs read another's, from 100–125 of 200 |
| **build warnings** | Unchanged: large static data, the DCE note, the two "refusing to overwrite stdlib leaf" lines |
| **consumer** | Not re-run. agnostic pins 2.2.4. ⚠ A root server without `CAP_SETPCAP` runs no process or Python tool, and a load test's numbers are not comparable with earlier runs (CHANGELOG *Behaviour changes*) |
| **upstream filings** | ⚠ **Six not committed**, so not visible upstream: sigil (RSA over 4096 bits; the lane pool), sandhi (bytes per exchange; chunked bodies; the pooled connection's allocator, new at 2.3.6), bote (MCP client). kavach's and hoosh's are closed by their releases |

### 2.3.5 — the release (2026-11-08)

| | |
|---|---|
| **version** | **2.3.5** (cut 2026-11-08): a spawned tool is answered when it finishes, a WASM module is checked when it is loaded, and a staged module goes with its tool. The spawner waits in `ppoll`, not a 5 ms sleep. A module needs its sections to fit, an `_start` function and, where wasmtime is installed, `wasmtime compile`'s yes. A registry holds its tools, and the last to let a WASM tool go frees its module, once no run holds it. The suites take their scratch paths from `$TMPDIR` and leave none |
| **cyrius pin** | **6.7.18**, unchanged |
| **tests** | **106 suites, 11,347 assertions, 0 failed** under `CYRIUS_DCE=1` with wasmtime 50.0.1, counted from each suite's printed line as well as its exit code. 2.3.4's tree reads 11,298. `sandbox_wasm` +23, `sandbox_spawn` +9, `tools_wasm` +9, `tools_native` +8. 22 mutants, all killed. **A full `cyrius test` leaves nothing in `/tmp`**, compared before and after |
| **coverage** | **99%**: 106/106 files, 1,853/1,864 fns (gate `--min 80`), the same 11 unreferenced as 2.3.3 |
| **dist** | `dist/agnosai.cyr` 45,750 lines + `dist/agnosai-guard.cyr` 1,034, both v2.3.5, by `wc -l` (2.3.4's read 45,353 and 1,034). Sidecars **6 / 1** leaves, unchanged |
| **binary** | `build/agnosai` **3,025,400 B** (2.3.4: 3,020,792). aarch64 **7,034,912 B** (7,034,400), not compacted, built and not started. The native binary answers `/ready` with `"version":"2.3.5"` and exits 0 on SIGTERM |
| **deps** | Unchanged: kavach 3.14.7, and every other dep on its latest tag |
| **lock** | `cyrius.lock` unchanged: 119 files, 8 commit pins |
| **gates** | `check-symbols.sh` 3,238 definitions across 117 files; `check-clean.sh` fmt 236, lint 236, doc 118, doctest 1 files; `deps --verify` 119; lib snapshot 113; log lengths 235 files; dist. fmt and lint on every test, bench and fuzz file. Then build, aarch64, every test and fuzz unit compiles, tests under DCE, coverage, the example, fuzz 4/4, `distlib --all --check`, `cyrius bench`. **The `ubuntu:26.04` CI replica ran** on the bumped tree with this release's new files laid over a clean clone, wasmtime 47.0.3, all 8 pins from GitHub: every `ci.yml` step, 106 suites, 11,201 assertions, 0 failed. Its first run refused every WASM module, as a fresh user has no `~/.cache/wasmtime` and the confinement gives no `$HOME`, which is why the check runs `-C cache=n` |
| **benchmarks** | The interleaved A/B against a 2.3.4 build, four rounds of `sandbox` and `tools` at a mean load of 3.3, reads **median +0.0% across 25 rows**. `spawn_true_unconfined` 10.26 → 0.51 ms (−95.1%), `spawn_true_confined` 10.27 → 1.12 ms (−89.1%), the Python tool calls −17% and −19%. `tool_registry_register` +16.9%, 661 → 773 ns, in a second `tools` A/B: the hold, the replaced tool's lookup and its drop. `bench-history.csv` has 245 rows at 2.3.5, one sweep at a load of 1.8–2.3: median +3.6% against 2.3.4's rows, taken at 0.3–0.9, not claimed |
| **WASM at load** | Refused, each with its reason: a bare header (no `_start`), an `_start` that is a memory, a section past the end, a size that never ends, an unknown section id, a broken export section, and a module wasmtime will not compile. A load runs `wasmtime compile` confined, about 4 ms |
| **staged modules** | Freed when the last registry lets a WASM tool go. A free 400 ms into a run keeps the file until the run ends, and a run after the free is refused. 305 staged modules had built up in this box's `/tmp` from earlier test runs, and 9 files from the fixed-path suites |
| **build warnings** | Unchanged: large static data, the DCE note, the two "refusing to overwrite stdlib leaf" lines |
| **consumer** | Not re-run. agnostic pins 2.2.4. ⚠ A WASM module with no `_start` no longer loads, and a tool removed from its registry no longer runs (CHANGELOG *Behaviour changes*) |
| **upstream filings** | ⚠ **Eight not committed**, so not visible upstream: kavach (concurrent runs read each other's stderr; a network-denying policy needs a namespace), sigil (RSA over 4096 bits; the lane pool), sandhi (bytes per exchange; chunked bodies), hoosh (span ids) and bote (the MCP client, with a scope conflict). The roadmap's *Waiting on another project* has each path |

### 2.3.4 — the release (2026-11-08)

| | |
|---|---|
| **version** | **2.3.4** (cut 2026-11-08, the bump script's UTC date; 2026-11-07 Pacific): process and Python tools run confined. The child applies kavach's `confine_child_caps` (NO_NEW_PRIVS, landlock, kavach's seccomp filter) in agnosai's own spawner, with the binary pinned by descriptor (ADR 031). OCI containers drop every capability, take `no-new-privileges` and at most 256 processes. The image runs as uid 10001. A process or Python tool call in the call's arena keeps 0 B |
| **cyrius pin** | **6.7.18**, unchanged |
| **tests** | **106 suites, 11,298 assertions, 0 failed** under `CYRIUS_DCE=1` with wasmtime 50.0.1, counted from each suite's printed line as well as its exit code. 2.3.3's tree reads 11,199. New: `sandbox_confine` 87. `tools_wasm` +7, `sandbox_oci` +6. 26 mutants, all killed |
| **coverage** | **99%**: 106/106 files, 1,846/1,857 fns (gate `--min 80`), the same 11 unreferenced as 2.3.3 |
| **dist** | `dist/agnosai.cyr` 45,353 lines + `dist/agnosai-guard.cyr` 1,034, both v2.3.4, by `wc -l` (2.3.3's read 44,690 and 1,034). Sidecars **6 / 1** leaves, unchanged |
| **binary** | `build/agnosai` **3,020,792 B** (2.3.3: 3,011,696). aarch64 **7,034,400 B** (7,033,504), not compacted, built and not started. The native binary answers `/ready` with `"version":"2.3.4"` and exits 0 on SIGTERM. **The image** builds, answers `/ready` and runs as `uid=10001(agnosai)` |
| **deps** | Unchanged from 2.3.3: kavach 3.14.7, and every other dep on its latest tag |
| **lock** | `cyrius.lock` unchanged: 119 files, 8 commit pins |
| **gates** | `check-symbols.sh` 3,212 definitions across 117 files; `check-clean.sh` fmt 236, lint 236, doc 118, doctest 1 files; `deps --verify` 119; lib snapshot 113; log lengths 235 files; dist. fmt and lint on every test, bench and fuzz file. Then build, aarch64, every test and fuzz unit compiles, tests under DCE, coverage, the example, fuzz 4/4, `distlib --all --check`, `cyrius bench`. **The `ubuntu:26.04` CI replica ran**, on the bumped tree with this release's new files laid over a clean clone, wasmtime 47.0.3, all 8 pins from GitHub, inside docker's default seccomp profile: every `ci.yml` step, 106 suites, 11,152 assertions, 0 failed, `sandbox_confine` 87 of 87. Its first run failed 4 checks of `sandbox_confine`, found the `/proc/self` limit ADR 031 now states, and the suite was fixed |
| **benchmarks** | New file `benches/sandbox.bcyr`: `spawn_true_unconfined` 10.310 ms and `spawn_true_confined` 10.320 ms, `python_tool_call_unconfined` 41.27 ms and `python_tool_call_confined` 41.24 ms. Confinement shows nothing; both are the drain loop's 5 ms idle tick, which roadmap 2.3.5 takes out. No row in the other ten files runs code this release changed. `bench-history.csv` has 245 rows at 2.3.4, one sweep at a load of 0.3–0.9: median −1.2% against 2.3.3's rows, 188 of 241 within ±5%, not claimed |
| **confinement** | Measured from inside the child (`tests/sandbox_confine.tcyr`): a read of the server's tree and of `/proc/1/environ` is EACCES, a write outside `/tmp` and the work directory is EACCES, `NoNewPrivs` 1, `Seccomp` 2, `unshare` killed, a binary outside the rules refused at the exec, a program's own exit 124 not read as kavach's landlock step, 100 calls on four threads with no result crossing. Landlock ABI 10 on this kernel, 14 rules |
| **bytes per call** | A process call in an arena **0 B** (2.3.3: 13,992 B on the global heap), a Python tool call **0 B** (11,032 B), a bare process call 368 B. A WASM call 3,160 B, kavach's run. The first confined call builds the confinement, the environment and the bridge script once, 22,888 B |
| **build warnings** | Unchanged: large static data, the DCE note, the two "refusing to overwrite stdlib leaf" lines |
| **consumer** | Not re-run. agnostic pins 2.2.4, and neither it nor thoth calls these tiers directly. ⚠ A tool that read `$HOME`, `/var` or `/opt` gets EACCES (CHANGELOG *Behaviour changes*) |
| **upstream filings** | ⚠ **Eight not committed**, so not visible upstream: kavach (concurrent runs read each other's stderr; a network-denying policy needs a namespace, both new), sigil (RSA over 4096 bits; the lane pool), sandhi (bytes per exchange; chunked bodies), hoosh (span ids) and bote (the MCP client, with a scope conflict). kavach's own capability issue is committed and on GitHub. The roadmap's *Waiting on another project* has each path |

### 2.3.3 — the release (2026-11-08)

| | |
|---|---|
| **version** | **2.3.3** (cut 2026-11-08, the bump script's UTC date; 2026-11-07 Pacific): a model is chosen by what hoosh says it is. Where hoosh 2.10.2's catalog gives `tier`, `kind` and `tools`, they decide (ADR 030's addendum). An agent with tools is never given a model hoosh says takes none, and an embedder is never sent to chat. kavach moves to 3.14.7, which gives back each WASM call's 1 MiB. Confinement of the process, Python and OCI tools takes 2.3.4, as kavach shipped its fix |
| **cyrius pin** | **6.7.18**, unchanged |
| **tests** | **105 suites, 11,199 assertions, 0 failed** under `CYRIUS_DCE=1` with wasmtime 50.0.1, counted from each suite's printed line as well as its exit code. 2.3.2's tree reads 11,105. `llm_model_catalog` 257 (+65), `orch_model_selection` 139 (+28), `sandbox_wasm` +1, and `sandbox_kavach_bridge` pins the process score at 55. 28 mutants, all killed |
| **coverage** | **99%**: 106/106 files, 1,823/1,834 fns (gate `--min 80`), the same 11 unreferenced as 2.3.2 |
| **dist** | `dist/agnosai.cyr` 44,690 lines + `dist/agnosai-guard.cyr` 1,034, both v2.3.3, by `wc -l` (2.3.2's read 44,448 and 1,034 the same way). Sidecars **6 / 1** leaves, unchanged |
| **binary** | `build/agnosai` **3,011,696 B** (2.3.2: 2,998,968, and 3,007,424 with kavach 3.14.7 alone). aarch64 **7,033,504 B** (6,967,520), not compacted, built and not started. The native binary answers `/ready` with `"version":"2.3.3"` and exits 0 on SIGTERM |
| **deps** | **kavach 3.14.6 → 3.14.7**, at `f3e64f6a`, its tag on GitHub. Every other dep was already on its latest tag: sigil 3.13.15, bote 3.8.15, majra 2.13.0, ai-hwaccel 2.5.6, tyche 1.2.2, libro 2.11.3, patra 1.16.6 |
| **lock** | `cyrius.lock` 119 files, 8 commit pins. ⚠ `cyrius deps` kept kavach's 3.14.6 pin beside the new tag and said so, so the lock was deleted and resolved again. The other seven pins and every other line are byte-identical |
| **gates** | `check-symbols.sh` 3,153 definitions across 117 files; `check-clean.sh` fmt 234, lint 234, doc 118, doctest 1 files; `deps --verify` 119; lib snapshot 113; log lengths 233 files; dist. fmt and lint on every test, bench and fuzz file. Then build, aarch64, every test and fuzz unit compiles, tests under DCE, coverage, the example, fuzz 4/4, `distlib --all --check`, `cyrius bench`. **The `ubuntu:26.04` CI replica ran** on the bumped tree with wasmtime 47.0.3, resolving all 8 pins from GitHub: every `ci.yml` step, 105 suites, 11,053 assertions, 0 failed, as suites that count per environment do. `sandbox_wasm` 113 of 113, three runs |
| **benchmarks** | The interleaved A/B against a 2.3.2 build with the same `lib/`, five rounds of `llm` and `orch` at a mean load of 1.9, reads **median −0.1% across 115 rows**, 109 within ±5%. After the catalog's read was written out, `model_for_tier_*` reads 59 → 62 ns, the bare form's call to the tools form, and `crew_select_model_routed` 144 → 151 ns. `model_catalog_parse_200` +3.3%. New: `model_catalog_parse_200_tiered` 2.08 ms, `model_for_tier_premium_tools` 58 ns. `bench-history.csv` has 241 rows at 2.3.3, one sweep at a load of 1.0–1.3: median +0.0% against 2.3.2's rows, 210 of 239 within ±5%, not claimed |
| **end to end** | A real hoosh 2.10.2 at stub OpenAI and Ollama providers, and the real server running one-agent crews. On 2.3.3 a `high` agent with tools reaches the provider as `gpt-5.6-sol`, and as `gpt-6-astra` without tools. A `medium` one reaches it as `gpt-6-sol` or `gpt-6.1-sol`. Ollama's `bge-m3`, listed first, is passed over for `llama3.2:3b`. 2.3.2 sent tools to `gpt-6-astra` and `gpt-6.1-sol`, and chat to `bge-m3` (CHANGELOG 2.3.3) |
| **bytes per crew** | A WASM call keeps **3,160 B**, where kavach measured 1,051,513 B on 3.14.6, and `tests/sandbox_wasm.tcyr` bounds it at 16 KiB. A held catalog is 164 B an entry, 24 B more than 2.3.2's. The rest as 2.3.2 below |
| **build warnings** | Unchanged: large static data, the DCE note, the two "refusing to overwrite stdlib leaf" lines |
| **consumer** | Not re-run. agnostic pins 2.2.4. Nothing is removed. ⚠ A crew profile's `sandbox_strength` for the process policy reads 55 where it read 60, kavach 3.14.7's scoring (CHANGELOG *Changed*) |
| **upstream filings** | kavach's, the capture buffer, is fixed in 3.14.7 and archived there. ⚠ **The other six are still not committed**, so not visible upstream: sigil (RSA over 4096 bits; the lane pool), sandhi (bytes per exchange; chunked bodies), hoosh (span ids) and bote (the MCP client, with a scope conflict). The roadmap's *Waiting on another project* has each path |

### 2.3.2 — the release (2026-11-07)

| | |
|---|---|
| **version** | **2.3.2** (cut 2026-11-07): a tool's answer goes back with its call. Each call the tool loop runs takes an arena of its own, a tool reads it from `agnosai_tool_input_alloc`, and every built-in answers there. A suite that failed could exit 0 and pass CI; every suite now ends in `sys_exit_group`. The roadmap is by release, in plain words. Confining the process, Python and OCI tools, planned for this release, waits on kavach |
| **cyrius pin** | **6.7.18**, unchanged |
| **tests** | **105 suites, 11,105 assertions, 0 failed** under `CYRIUS_DCE=1` with wasmtime 50.0.1, counted from each suite's printed line as well as its exit code. 2.3.1's tree reads 10,986 here. The `ubuntu:26.04` CI replica reads 10,959, as suites that count per environment do. 31 mutants, all killed |
| **coverage** | **99%**: 106/106 files, 1,817/1,828 fns (gate `--min 80`), the same 11 unreferenced as 2.3.1 |
| **dist** | `dist/agnosai.cyr` 44,098 lines + `dist/agnosai-guard.cyr` 1,014, both v2.3.2. Sidecars **6 / 1** leaves, unchanged |
| **binary** | `build/agnosai` **2,998,968 B** (2.3.1: 2,994,840). aarch64 **6,967,520 B** (6,967,496), not compacted, built and not started. The native binary answers `/ready` with `"version":"2.3.2"` and exits cleanly on SIGTERM |
| **lock** | `cyrius.lock` unchanged: 119 files, 8 commit pins |
| **gates** | `check-symbols.sh` 3,136 definitions across 117 files; `check-clean.sh` fmt 234, lint 234, doc 118, doctest 1 files; `deps --verify` 119; lib snapshot 113; log lengths; dist. Then build, aarch64, tests under DCE, coverage, `distlib --all --check`, the example, fuzz 4/4, `cyrius bench`. **The `ubuntu:26.04` CI replica ran**, on the bumped tree with wasmtime 47.0.3: every `ci.yml` step, 105 suites, 0 failed |
| **benchmarks** | The interleaved A/B against a 2.3.1 build, three rounds of `tools`, `orch` and `server` at a mean load of 3.3, reads **median +0.2% across 149 rows**, 109 within ±5%. `agnos_form_encode_52b` −34%, `agnos_segment_ok` −18%, `lt_result_to_value` −10%. `task_one_tool_call` +2.0%, about 600 ns for the call's arena. New: `task_one_tool_call_kept` and `task_four_tool_calls_kept`. `bench-history.csv` has 239 rows at 2.3.2, one sweep at a load of 1.2–1.4: median −2.8% against 2.3.1's rows, 169 of 236 within ±5%, not claimed, as 2.3.1's sweep ran at a load of 3.2 |
| **bytes per crew** | **0** for a call whose tool answers in its arena, on every arm of `echo`, `json_transform` and all nine AGNOS tools. What the network tools still leave is not theirs: sandhi's 352–480 B an exchange, the SSRF guard's 136 B a check, and `load_testing`'s joined threads at 4,120 B a user (roadmap 2.3.6) |
| **build warnings** | Unchanged: large static data, the DCE note, the two "refusing to overwrite stdlib leaf" lines |
| **consumer** | Not re-run. agnostic pins 2.2.4. ⚠ `transport_fp` and `shape_fp` take an allocator first (CHANGELOG *Breaking*): agnostic and thoth call neither, and joshua and kiran are not on this machine, so that neither does is unchecked |
| **upstream filings** | kavach's, the capture buffer that holds back confinement, is committed and on GitHub. ⚠ **The other six are not committed**, so not visible upstream: sigil (RSA over 4096 bits; the lane pool), sandhi (bytes per exchange; chunked bodies), hoosh (span ids, new) and bote (the MCP client, new, with a scope conflict). The roadmap's *Waiting on another project* has each path |

### 2.3.1 — the release (2026-11-07)

| | |
|---|---|
| **version** | **2.3.1** (cut 2026-11-07, the bump script's UTC date; 2026-11-06 Pacific): models from hoosh's catalog, B33. A tier means what the gateway lists, by what each model declares and costs, a listed id goes out as listed, and with no catalog and no override a crew that names no model fails by name (ADR 030). With 2.3.0 this is all of what agnostic's M6 was waiting on |
| **cyrius pin** | **6.7.18**, unchanged: still the latest tag |
| **tests** | **105 suites, 10,990 assertions, 0 failed** under `CYRIUS_DCE=1` with wasmtime 50.0.1 present. 2.3.0's tree reads 10,721. The new suites are `llm_model_catalog` 192 and `orch_model_selection` 111, less what moved out of `orch_crew_runner` and `fleet_cost_planning` with the code they pinned |
| **coverage** | **99%**: 106/106 files, 1,795/1,806 fns (gate `--min 80`), the same 11 unreferenced as 2.3.0. Every new public name is named by a test |
| **dist** | `dist/agnosai.cyr` 43,861 lines + `dist/agnosai-guard.cyr` 1,014, both v2.3.1. Sidecars **6 / 1** leaves, unchanged. `src/model_catalog.cyr` joins `[lib]` after `src/llm/hoosh.cyr` |
| **binary** | `build/agnosai` **2,994,840 B** (2.3.0: 2,977,840). aarch64 **6,967,496 B** (6,966,880), not compacted, built and not started |
| **lock** | `cyrius.lock` unchanged: 119 files, 8 commit pins. ⚠ A fresh worktree needs `cyrius lib sync --full` first, as CI does: `cyrius deps` alone leaves `lib/` at 87 files, the lock 26 lines short, and the lib snapshot gate failing on files it was never given |
| **gates** | `check-symbols.sh` 3,110 definitions across 117 files; `check-clean.sh` fmt 234, lint 234, doc 118, doctest 1 files; `deps --verify` 119; lib snapshot 113; log lengths; dist. Then build, tests under DCE, coverage, `distlib --all --check`, examples, fuzz 4/4. The `ubuntu:26.04` CI replica was not run |
| **benchmarks** | The interleaved A/B against a 2.3.0 build, three rounds of `llm`, `orch`, `fleet` and `server` at a load of about 2.3, reads **median −0.6% across 157 rows**, 152 within ±5%. `crew_select_model_routed` is +15.9%, 126 to 146 ns, and real: the walk to a gateway's slot. `crew_runner_10_tasks_parallel_4_{quiet,llm_quiet,llm_watched}` read −19% to −29% and are not claimed (B27). New: `model_for_tier_*` 57 ns, `crew_attribute_owned_by` 143 ns, `model_catalog_owner_200_last` 3.8 µs, `model_catalog_parse_200` 1.8 ms. `bench-history.csv` has 236 rows at 2.3.1, one sweep ending at a load of 3.2: median +0.7% against 2.3.0's rows, 178 of 230 within ±5% (CHANGELOG 2.3.1 *Performance*) |
| **bytes per crew** | **0** for a selection once a catalog is held. A re-read that finds the same bytes leaves under 1 KiB on a real socket, which is sandhi's (B39). A catalog is about 150 B per entry for each change at hoosh, and the first live read reserves 2 MiB of address space per gateway for good. The rest as 2.3.0 below |
| **build warnings** | Unchanged: large static data, the DCE note, the two "refusing to overwrite stdlib leaf" lines |
| **consumer** | Not re-run. agnostic pins 2.2.4. On 2.3.1 a crew with no `llm_model` gets a model its gateway lists and needs no change. ⚠ `complexity: high` then costs what the gateway's dearest reasoning model costs, `claude-fable-5-1` at $10/$50 on hoosh's table, and selection cannot see tools (ADR 030), so a crew on an OpenAI-only gateway needs `AGNOSAI_MODEL_PREMIUM`. joshua and kiran are not on this machine, so that neither calls the removed functions is unchecked |

Direct deps: as 2.2.5 below.

### 2.3.0 — the release (2026-11-07)

| | |
|---|---|
| **version** | **2.3.0** (cut 2026-11-07, the bump script's UTC date; 2026-11-06 Pacific): the tool-calling harness. An agent is offered the tools it names, and the model calls them until it answers, under ADR 029's four rules. B7 items 8 and 9 hang on it, and `delegate`'s crew joins the caller's trace. The audits moved to `docs/audit/`, and the roadmap is now release pins from 2.3.1 to 2.7.2, with `rust-old/`'s deletion waiting on bhava (B5) |
| **cyrius pin** | **6.7.18**, unchanged: still the latest tag |
| **tests** | **103 suites, 10,721 assertions, 0 failed** under `CYRIUS_DCE=1` with wasmtime 50.0.1 present. 2.2.5's tree reads 10,281. The +440 is mostly the new `orch_tool_loop` 361, with `llm_hoosh` +40 and the rest in `telemetry_wiring`, `tools_builtin_delegate` and `server_routes_crews`. 74 mutants, all killed. `sandbox_spawn` fails one assertion in a tree under `/tmp`, as it asserts a spawned `pwd` holds no `/tmp` |
| **coverage** | **99%**: 105/105 files, 1,772/1,783 fns (gate `--min 80`), the same 11 unreferenced as 2.2.5. Every new public name is named by a test. The loop's internals are `_`-prefixed, as only `crew_runner` calls them |
| **dist** | `dist/agnosai.cyr` 43,076 lines + `dist/agnosai-guard.cyr` 1,014, both v2.3.0. Sidecars **6 / 1** leaves, unchanged. `src/tool_loop.cyr` joins `[lib]` before `src/orchestrator/crew_runner.cyr`. ⚠ `distlib` warns of 34 names undefined on `x86_64-windows` and `x86_64-agnos`. 2.2.5 prints the same list |
| **binary** | `build/agnosai` **2,977,840 B** (2.2.5: 2,939,232). aarch64 **6,966,880 B** (6,899,584), not compacted, built and not started. The native binary answers `/ready` with `"version":"2.3.0"`, ran a sequential and a parallel crew through two tools against a stub gateway on a socket, reports them in `results[].tool_calls`, fails a crew naming a tool it does not hold, and exits cleanly on SIGTERM |
| **lock** | `cyrius.lock` unchanged: 119 files, 8 commit pins |
| **gates** | `check-symbols.sh` 3,057 definitions across 116 files; `check-clean.sh` fmt 231, lint 231, doc 117, doctest 1 files; `deps --verify` 119; lib snapshot 113; log lengths; dist. Then build, tests under DCE, coverage, `distlib --all --check`, examples, fuzz 4/4, `cyrius bench`. Not run: the `ubuntu:26.04` CI replica, and a live hoosh or provider |
| **benchmarks** | The interleaved A/B against a 2.2.5 build, three rounds of the five files this touches, reads **median +0.3% across 172 rows**, 146 within ±5%, at a load of about 3.5, and the tool-less `run_crew_*` rows are flat. `task_one_tool_call` 31.0 µs and `task_four_tool_calls_at_once` 244.9 µs are new. `otlp_encode_span` reads +10.3%, with the rows either side flat (CHANGELOG 2.3.0 *Performance*). `bench-history.csv` has 234 rows at 2.3.0, one sweep ending at a load of 3.3: median +0.1% against 2.2.5's rows, 141 of 230 within ±5%. Two earlier sweeps of this code ran beside other sessions' VMs at loads of 7–9, read medians of +9% and +10.5%, and were discarded under the rule below |
| **bytes per crew** | **0** for a call through the loop, for one call, two at once and a refusal. A small real tool leaves 489 B a call on the global heap (B41), and a crew off ADR 028 keeps its records for good (B38). The rest as 2.2.5 below |
| **build warnings** | Unchanged: large static data, the DCE note, the two "refusing to overwrite stdlib leaf" lines |
| **consumer** | Not re-run. agnostic pins 2.2.4 and needs `agnosai_orchestrator_with_tools` at engine init to get the loop, which answers its `handoff.md` §8. Until then nothing changes for it. thoth's guard profile is unchanged. ⚠ A crew naming a tool the registry does not hold now fails, so agnostic's presets need its resolver first |

Direct deps: as 2.2.5 below.

### 2.2.5 — the release (2026-11-06)

| | |
|---|---|
| **version** | **2.2.5** (cut 2026-11-06): agnostic's two reports repaired, with `docs/development/issues/` holding only `archive/`. A task is held to its `output_schema`, which is in its first prompt, and one that never meets it is `failed`. A crew built with `agnosai_crew_new_in` lives in an arena that `agnosai_orchestrator_release_crew` gives back (ADR 028). kavach 3.14.6 closed two of agnosai's reports, so the 64 KiB cap on WASM input is gone and a guest is held to the Rust line's store limits (B9). An independent review of the crew work found six defects, all fixed here |
| **cyrius pin** | **6.7.18**, unchanged: still the latest tag |
| **tests** | **102 suites, 10,281 assertions, 0 failed** under `CYRIUS_DCE=1` with wasmtime 50.0.1 present. 2.2.4's tree reads 8,908 in the same shell (its recorded 8,912 was a different environment). The +1,373 is the new `orch_crew_arena` 1,288, `sandbox_wasm` +42, `orch_crew_runner` +38 and `arena_pool` +5. `sandbox_spawn` fails one assertion in a tree under `/tmp`, as it asserts a spawned `pwd` holds no `/tmp` |
| **coverage** | **99%**: 104/104 files, 1,724/1,735 fns (gate `--min 80`), 11 unreferenced. Every new public name is named by a test |
| **dist** | `dist/agnosai.cyr` 41,191 lines + `dist/agnosai-guard.cyr` 1,014, both v2.2.5. Sidecars **6 / 1** leaves, unchanged. `src/crew_arena.cyr` joins `[lib]` after `src/orchestrator/crew_runner.cyr` |
| **binary** | `build/agnosai` **2,939,232 B** (2.2.4: 2,908,008). aarch64 **6,899,584 B** (6,897,048), which is not compacted. The native binary answers `/ready` with `"version":"2.2.5"`, ran a sequential and a parallel crew against a gateway on a socket, and exits cleanly on SIGTERM |
| **lock** | `cyrius.lock` **119 files, 8 commit pins**. One pin moved: kavach 3.14.6 at `b772c152`, matching its tag on GitHub. ⚠ `cyrius deps` keeps a pin the manifest has moved past, and says so in a note, so a dep bump needs the lock deleted first |
| **gates** | `check-symbols.sh` 2,931 definitions across 115 files; `check-clean.sh` fmt 229, lint 229, doc 116, doctest 1 files; `deps --verify` 119; lib snapshot 113; log lengths; dist. Then build, tests under DCE, coverage, `distlib --all --check`, examples, fuzz 4/4, `cyrius bench`. Not run: the `ubuntu:26.04` CI replica. CHANGELOG 2.2.5 *Verified* has what was |
| **benchmarks** | `bench-history.csv` has 230 rows at 2.2.5, one sweep at a load of 3.0–3.3: median +0.2% against 2.2.4's rows, 185 of 229 within ±5%. The interleaved A/B against a 2.2.4 build reads `orch.bcyr` median 0.0%, 79 of 89 within ±5%, and `server.bcyr` −0.0%, 34 of 34, on a quiet machine. Parallel crews 12–28% faster, `run_crew_then_release` 87.4 µs beside `run_crew_then_forget` 92.2 µs (CHANGELOG 2.2.5 *Performance*). No row reaches the WASM path |
| **bytes per crew** | **0** on the global heap for a crew in an arena, released, in every process mode. 8,361 for a crew that does not opt in (2.2.4: 9,368). Against a socket the floor is sandhi's 480 B per exchange (B39). A WASM call leaves about 1 MiB of kavach's, whatever its input (B40) |
| **build warnings** | Unchanged: large static data, the DCE note, the two "refusing to overwrite stdlib leaf" lines |
| **consumer** | Not re-run. agnostic pins 2.2.4, and nothing in its schema path changes: `output_valid` reads completed tasks, and a failed one carries `error`. Its memory does not move until it adopts `agnosai_crew_new_in` and `release_crew` (ADR 028). thoth's guard profile gains `_a` forms and no new leaf |

Direct deps: as 2.2.4 below, with `kavach` **3.14.6**. Everything else was already on its latest tag.

### 2.2.4 — the release (2026-11-06)

| | |
|---|---|
| **version** | **2.2.4** (cut 2026-11-06, the bump script's UTC date; 2026-11-05 Pacific): agnostic's four reports repaired, with `docs/development/issues/` now holding them under `archive/`. Crew retention is oldest-first with `agnosai_orchestrator_forget_crew` (ADR 026); a consumer's tool gate, target admission, redirect check and call record (ADR 025); three example presets (ADR 027); a transport error that no longer names the gateway. An independent review of that work found an SSRF bypass older than it, and building it turned up a second, both fixed here: a leading zero got a private address past `agnosai_is_safe_url`, and a trailing dot took a name off its blocked list |
| **cyrius pin** | **6.7.18**, unchanged: still the latest tag |
| **tests** | **101 suites, 8,912 assertions, 0 failed** under `CYRIUS_DCE=1` with wasmtime 50.0.1 present. 2.2.3's tree reads 8,748 in the same shell (its recorded 8,746 was a different environment: `sandbox_spawn` and `sandbox_process` count per environment variable). The +164 is the new `tools_gate` 130 and `orch_crew_retention` 66, `server_ssrf` +37, `llm_hoosh` +18 and `server_routes_mcp` +3, less 90 from `definitions_loader_presets` as the set went from eighteen to three. `sandbox_spawn` fails one assertion in a tree under `/tmp`, as it asserts a spawned `pwd` holds no `/tmp` |
| **coverage** | **99%**: 103/103 files, 1,660/1,672 fns (gate `--min 80`), the same 12 unreferenced. Every new public name is named by a test |
| **dist** | `dist/agnosai.cyr` 40,386 lines + `dist/agnosai-guard.cyr` 949, both v2.2.4. Sidecars **6 / 1** leaves, unchanged. `src/tool_call.cyr` joins `[lib]` after `src/tools/registry.cyr` |
| **binary** | `build/agnosai` **2,908,008 B** (2.2.3: 2,897,776; bote adds what DCE cannot drop). aarch64 **6,897,048 B** (6,632,872), which is not compacted. The native binary answers `/ready` with `"version":"2.2.4"`, lists three presets, refuses `http://0127.0.0.1:<port>/` and `http://169.0254.169.254/` on `/mcp`, and exits cleanly on SIGTERM. A crew against a dead gateway answers `LLM error: hoosh transport error: CONNECT`, with the URL on a WARN line only. A `HOOSH_URL` with userinfo logs `(withheld)` at start-up |
| **lock** | `cyrius.lock` **119 files, 8 commit pins**. One pin moved: bote 3.8.15 at `1503109d`, matching its tag on GitHub. ⚠ `cyrius deps` keeps a pin the manifest has moved past, and says so in a note, so a dep bump needs the lock deleted first |
| **gates** | `check-symbols.sh` 2,818 definitions across 114 files; `check-clean.sh` fmt 227, lint 227, doc 115, doctest 1 files; `deps --verify` 119; lib snapshot 113; log lengths; dist. Then build, tests under DCE, coverage, `distlib --all --check`, examples, fuzz 4/4, `cyrius bench`. CI replica: CHANGELOG 2.2.4 *Verified* |
| **benchmarks** | `bench-history.csv` has 229 rows at 2.2.4, one sweep at a load of 1.9–2.1: median −1.0% against 2.2.3's rows, which ran at 3.2–3.4, and 183 of 222 within ±5%. The interleaved A/B against a 2.2.3 build, three rounds of all eleven files, reads **median −0.0%**, 208 of 222 within ±5%. The rows outside it are placement or B27, and two are real: `is_safe_url_octal_host` +8.8% and `orchestrator_new` +6.1% (CHANGELOG 2.2.4 *Performance*) |
| **build warnings** | Unchanged: large static data, the DCE note, the two "refusing to overwrite stdlib leaf" lines |
| **consumer** | Not re-run. agnostic pins 2.1.7, and its roadmap can now take the re-pin: `forget_crew` replaces `_agnostic_crew_retire_a`'s reach, and `_agnostic_scrub_gateway_a` has nothing left to scrub from this path. thoth's guard profile is unchanged. No consumer in the stack reads the fifteen presets that went |

Direct deps: as 2.2.1 below, with `bote` **3.8.15**. Everything else was already on its latest tag.

### 2.2.3 — the release (2026-11-05)

| | |
|---|---|
| **version** | **2.2.3** (cut 2026-11-05): the P(-1) audit and hardening after the language bumps. A server that cannot bind now says why; WASM input over 64 KiB is refused (a kavach capture deadlock); a stats-block leak is fixed; two guards that could skip silently now assert; the threat model is refreshed against the Cyrius tree; the release binary is DCE-compacted; and the upstream gaps are filed in kavach's, sigil's and sandhi's own `docs/development/issues/`. No dependency change |
| **cyrius pin** | **6.7.18**, unchanged |
| **tests** | **99 suites, 8,746 assertions, 0 failed** with wasmtime 50.0.1 present: 2.2.2's 8,728 plus the audit's 18 (`server_serve` +8, `sandbox_cx` +4, `sandbox_wasm` +4, `tools_wasm` +1, `server_rate_limit` +1). With `CYRIUS_DCE=1`, as CI now runs them, the suite passes too. On a host without the cx sandbox's namespaces, `sandbox_cx` reads fewer (its fail-closed arm) |
| **coverage** | **99%**: 102/102 files, 1,635/1,647 fns (gate `--min 80`) |
| **dist** | `dist/agnosai.cyr` 40,049 lines + `dist/agnosai-guard.cyr` 949, both v2.2.3. The sidecars are unchanged at **6 / 1** leaves; `assert` stays a production leaf (`panic`) |
| **binary** | `build/agnosai` **2,897,776 B** with dead-code elimination (`[build] dce = true`; 5,805,848 without, and 2.2.2 shipped 5,826,992). aarch64 **6,632,872 B**: NOP-filled, not compacted, since cyrius has no aarch64 compaction. The same 24 requests to the 2.2.2 binary and this one differ only in the version string and the per-process hash order of tool lists and metadata keys (B36). A port conflict now logs `server failed` with `bind (errno 98)` |
| **lock** | Unchanged: 119 files, 8 commit pins (dropping `hashmap_fast`, declaring `sys` and moving `bench` to `[test] stdlib` change no locked row) |
| **gates** | as 2.2.2, with CI's Test step under `CYRIUS_DCE=1`: `check-symbols.sh` 2,791 definitions; `check-clean.sh` fmt 224, lint 224, doc 114, doctest 1; then the `ubuntu:26.04` CI replica (CHANGELOG 2.2.3 *Verified*) |
| **benchmarks** | `bench-history.csv` has 224 rows at 2.2.3, one sweep at a load of 3.2–3.4: median −1.3% against 2.2.2's, 206 of 224 within ±5%. The interleaved A/B against a 2.2.2 build reads flat: 137 rows outside `orch.bcyr` median −0.2%, and `orch.bcyr`'s 87 median −0.1% once the B27 attempt was reverted (with it, ~50 rows read 25–157% slower: B20's thread level). Sweeps are kept only if the load at their end is below 5 |
| **build warnings** | Unchanged: large static data, the DCE note, the two "refusing to overwrite stdlib leaf" lines |
| **consumer** | Not re-run. agnostic pins 2.1.7; thoth takes the guard profile, unchanged. A consumer sending a WASM tool more than 64 KiB of input now gets an error |

Direct deps: unchanged from 2.2.1.

### 2.2.2 — the release (2026-11-05)

| | |
|---|---|
| **version** | **2.2.2** (cut 2026-11-05): the quick roadmap items. Three fleet defects copied from the Rust line are fixed (B30: GPU re-allocation, `max_retries`, `topology_score`); `check-clean.sh` lints `tests/` (B31); the upstream gaps are re-checked (B16); stale facts in `CLAUDE.md` are corrected (B23). The roadmap is restructured into a release plan, **2.2.3 = P(-1) audit and hardening**. No toolchain or dependency change |
| **cyrius pin** | **6.7.18**, unchanged (2.2.1 has the slot details) |
| **tests** | **99 suites, 8,728 assertions, 0 failed** with wasmtime 50.0.1 present. That is 2.2.1's 8,718 plus the fleet fixes' tests (`fleet_gpu` +4, `fleet_coordinator` +4, `fleet_topology` +2) |
| **coverage** | **99%**: 102/102 files, 1,635/1,647 fns (gate `--min 80`); the new `_agnosai_topology_high_bw_pairs` is `_`-prefixed |
| **dist** | `dist/agnosai.cyr` 39,989 lines + `dist/agnosai-guard.cyr` 949, both v2.2.2; sidecars **6 / 1** leaves, unchanged |
| **binary** | `build/agnosai` **5,826,992 B** and aarch64 **6,698,984 B**, the same sizes as 2.2.1 (the fleet code is not reached from `main`). The native binary answers `GET /ready` with `"version":"2.2.2"`, a 405 carries `Allow`, SIGTERM shuts down gracefully, and no startup line carries a NUL |
| **lock** | Unchanged from 2.2.1: 119 files, 8 commit pins |
| **gates** | as 2.2.1, with `check-clean.sh`'s lint now covering `tests/`: fmt 224, **lint 224** (2.2.1: 125), doc 114, doctest 1 files; `check-symbols.sh` 2,788 definitions; then the `ubuntu:26.04` CI replica (CHANGELOG 2.2.2 *Verified*) |
| **benchmarks** | `bench-history.csv` has 224 rows at 2.2.2, one unpinned sweep at a load of 3.8–4.1: median +2.5% against 2.2.1's, 213 of 224 within ±5%. The fleet A/B against a 2.2.1 build (5 interleaved rounds) moves only `fleet_compute_allocate_8dev` +12% and `fleet_compute_release_realloc_8dev` +16.5%, the GPU fix's release-first lookup (CHANGELOG 2.2.2 *Performance*) |
| **build warnings** | Unchanged from 2.2.1 |
| **consumer** | Not re-run. agnostic pins agnosai 2.1.7 and calls none of the changed fleet functions; thoth takes the guard profile, which is unchanged |

Direct deps: unchanged from 2.2.1 (below).

### 2.2.1 — the release (2026-11-05)

| | |
|---|---|
| **version** | **2.2.1** (cut 2026-11-05): cyrius 6.7.9 → **6.7.18**, with every `[deps.*]` tag on its latest release; the guard bundle's `unicode` requirement restored after 6.7.18's `distlib` dropped it; `server_auth_lane_race`'s pinned arm re-armed (sigil 3.13.14 made `crypto_bank_set` a no-op); `sandbox_cx`'s guest tests made namespace-aware, since 6.7.18's tarball ships `cycc_cx` and CI now runs them. ⚠ JSON bodies and JWTs that repeat a key or carry a leading-zero number are now refused (bayan 1.11.1) |
| **cyrius pin** | **6.7.18** (`~/.cyrius/versions/6.7.18` `SOURCE_COMMIT` `a1cf8c02`, `tree-matches-tag: yes`), the latest cyrius tag seen 2026-11-04 |
| **tests** | **99 suites, 8,718 assertions, 0 failed** with wasmtime 50.0.1 present. 2.2.0's tree, run suite by suite in the same shell on its own pin (6.7.9), reads 8,716 (its recorded 8,718 was a different environment). The one suite that differs is `server_auth_lane_race`, +2 for the lane preconditions, so the move itself changes no count. On a host that cannot create the cx sandbox's namespaces (docker, which refuses the `unshare`; GitHub's `ubuntu-26.04` runner, which refuses the id maps), `sandbox_cx` reads 68 rather than 87, with its fail-closed arm in place of the guest tests |
| **coverage** | **99%**: 102/102 files, 1,635/1,647 fns (gate `--min 80`); the same 12 unreferenced |
| **dist** | `dist/agnosai.cyr` + `dist/agnosai-guard.cyr`, both v2.2.1. The sidecars list **6 / 1** stdlib leaves (2.2.0: 7 / 1): `unicode` became `unicode/_decode` in both, and `math` left the base sidecar because bayan 1.11.1 includes it (CHANGELOG 2.2.1 *Fixed*) |
| **binary** | `build/agnosai` **5,826,992 B** (2.2.0: 5,380,856). The growth is unreachable code: the DCE note reads 2,930,627 dead bytes, against 2,401,264. aarch64 **6,698,984 B** (6,097,136). `CYRIUS_DCE=1` now compacts x86_64 to 2,898,352 B; not adopted (untested). The native binary answers `GET /ready` with `"version":"2.2.1"`, a 405 carries `Allow`, SIGTERM shuts down gracefully, and no startup line carries a NUL |
| **lock** | `cyrius.lock` **119 files, 8 commit pins**, regenerated from empty. Every pin matches its tag on GitHub |
| **gates** | `cyrius lib sync --full` (113 files) → `deps` → `check-symbols.sh` (2,787 definitions, both targets) → `check-clean.sh` (fmt 224, lint 125, doc 114, doctest 1 files; `deps --verify` 119; lib snapshot 113; log lengths; dist) → build → test → coverage → `distlib --all --check` → examples (1) → fuzz 4/4 → `cyrius bench`, then the `ubuntu:26.04` CI replica (CHANGELOG 2.2.1 *Verified*) |
| **benchmarks** | `bench-history.csv` has 224 rows at 2.2.1, one unpinned sweep at a load of 2.5–4.4: median −1.7% against 2.2.0's rows. The same-box interleaved A/B against a 2.2.0 build (3 rounds of all eleven files) reads **median +0.3%**, 199 of 224 within ±5%. ⚠ **JSON parsing is 38–66% slower** (`*_from_json`, `builtin_presets_18` +55%, the JSON-body routes +12–20%), and a three-way run puts all of it on the toolchain, bayan 1.11.1's bounds-checked parser. `auth_jwt_key_prepare` is +14% (sigil 3.13.15's DER walks). Kept as hardening (CHANGELOG 2.2.1 *Performance*) |
| **build warnings** | `build/agnosai`: large static data and the DCE note, plus the two "refusing to overwrite stdlib leaf" lines. sigil's static-array warning is gone (sigil 3.13.15) |
| **consumer** | Not re-run against this tree. thoth (the guard profile) gets `unicode/_decode` from the restored requires block; agnostic pins agnosai 2.1.7 |

Direct deps (`[deps.*]`, all six), **none with an active `path`**, each on its latest release:
`sigil` **3.13.15** (= the 6.7.18 fold, byte-identical), `bote` **3.8.11**, `majra` **2.13.0**, `kavach`
**3.14.2**, `ai-hwaccel` **2.5.6**, `tyche` **1.2.2**. Transitively through bote: libro **2.11.3**. Every
pin inside those tags names a 6.7.18 fold. kavach 3.14.2 and libro 2.11.3 declare sigil 3.13.15, and
libro declares patra 1.16.6. ai-hwaccel 2.5.6 and samay 1.5.1 declare bayan 1.11.1. samay is kavach's
optional dep and is not vendored.

### 2.2.0 — the release (2026-11-04)

| | |
|---|---|
| **version** | **2.2.0** (cut 2026-11-04): cyrius 6.6.20 → **6.7.9**, with every `[deps.*]` tag on its 6.7.9 release; the CI and release workflows on the latest stable GitHub Actions (checkout v7.0.1, upload-artifact v7.0.2, action-gh-release v3.0.3, SHA-pinned); kavach 3.14.0 runs uutils coreutils, closing 2.1.8's recorded limitation; `telemetry_otlp`'s thread-count race fixed; the rate limiter's key cap holds at any rate (B4, majra 2.12.0's `ratelimit_evict_to`) |
| **cyrius pin** | **6.7.9** (`~/.cyrius/versions/6.7.9` `SOURCE_COMMIT` `8bb8df56`, `tree-matches-tag: yes`). The user's chosen target: cyrius has tagged up to 6.7.18 (seen 2026-11-04) |
| **tests** | **99 suites, 8,718 assertions, 0 failed** with wasmtime 50.0.1 present. 2.1.8's tree, run suite by suite in the same shell on its own pin (6.6.20), reads 8,706 (its recorded 8,704 was a different environment: `sandbox_spawn` and `sandbox_process` count per environment variable). The suites that differ are `sandbox_kavach_bridge` +3 (the host-binary test) and `server_serve` +9 (B4's strict-rate tests), so the move itself changes no count |
| **coverage** | **99%**: 102/102 files, 1,635/1,647 fns (gate `--min 80`); the same 12 unreferenced |
| **dist** | `dist/agnosai.cyr` 39,926 lines + `dist/agnosai-guard.cyr` 941 (`wc -l`), both v2.2.0. They differ from 2.1.8's in the version lines and B4's `src/server/rate_limit.cyr`, and the sidecars still list **7 / 1** stdlib leaves |
| **binary** | `build/agnosai` **5,380,856 B** (2.1.8: 5,337,008); aarch64 **6,097,136 B** (2.1.8: 6,032,848). The native one answers `GET /ready` with `"version":"2.2.0"`, a 405 carries `Allow`, SIGTERM shuts down gracefully, and no startup line carries a NUL. The aarch64 one was not run on the Pi this release |
| **lock** | `cyrius.lock` **119 files, 8 commit pins**, regenerated from empty (`lib/` and the lock removed, then `lib sync --full` + `deps`). Every pin matches its tag on GitHub (`git ls-remote`) |
| **gates** | `cyrius lib sync --full` (113 files) → `deps` → `check-symbols.sh` (2,787 definitions, both targets) → `check-clean.sh` (fmt 224, lint 125, doc 114, doctest 1 files; `deps --verify` 119; lib snapshot 113; log lengths; dist) → build → test → coverage → `distlib --all --check` → examples (1) → fuzz 4/4 → `cyrius bench`, then the `ubuntu:26.04` CI replica (CHANGELOG 2.2.0 *Verified*) |
| **benchmarks** | `bench-history.csv` has 224 rows at 2.2.0, one unpinned sweep at a load of 4–5: median +0.0% against 2.1.8's rows, 207 of 223 within ±5%. The new `rate_limit_spray_strict_rate` (B4) reads 2.97 µs, against 9.5–11.5 µs without the trim. The same-box interleaved A/B against a 2.1.8 build (3 rounds of all eleven files, then 5 of six) reads **median +0.0%**, 206 of 223 within ±5%. A three-way run that adds 2.1.8's source on the 6.7.9 toolchain splits the movers: `agnos_form_encode_52b` −12% and `replay_buffer_push_1000cap_full` +15% are the compiler, and `route_approvals_{global,arena}` +13% is placement, because the toolchain-only build reads ±0.3% and the route's callees are byte-identical (CHANGELOG 2.2.0 *Performance*) |
| **build warnings** | `build/agnosai`: toolchain notes only, the same set as 2.1.8 (sigil's static array, large static data, the two "refusing to overwrite stdlib leaf" lines, the DCE note) |
| **consumer** | Not re-run against this tree. agnostic pins agnosai 2.1.7 on cyrius 6.6.14, and thoth pins cyrius 6.6.6 |

Direct deps (`[deps.*]`, all six), **none with an active `path`**: `sigil` **3.13.11** (= the 6.7.9
fold, byte-identical), `bote` **3.4.10**, `majra` **2.12.0**, `kavach` **3.14.0**, `tyche` **1.2.0**, each
repo's last release pinned to cyrius 6.7.9, not its latest tag. `ai-hwaccel` stays **2.4.2**: it has no
release on 6.7.9 (2.4.2 pins 6.6.20, 2.5.0 pins 6.7.18), and kavach 3.14.0 and samay 1.2.0, both on 6.7.9,
declare 2.4.2 as well. Transitively through bote: libro **2.11.0**. Two pins inside the tags are off the
6.7.9 folds, and the fold wins both: libro 2.11.0 declares sigil 3.13.12, and ai-hwaccel 2.4.2 declares
bayan 1.5.12. The other deps' own pins name the folds (kavach 3.14.0: sigil 3.13.11; libro 2.11.0:
patra 1.16.0; samay 1.2.0: bayan 1.5.13).

### 2.1.8 — the release (2026-11-04)

| | |
|---|---|
| **version** | **2.1.8** (cut 2026-11-04): cyrius 6.6.14 → **6.6.20**, with every `[deps.*]` tag on its 6.6.20 release; a 2.1.7 regression fixed (four startup warnings 3 bytes long, and the log-length checker that passed them); the 16 B arena leak in `agnosai_agent_new_a` agnostic 0.1.17 found; `sandbox_kavach_bridge` runs its own payload, so `ubuntu-latest` (now 26.04, uutils coreutils) passes it |
| **cyrius pin** | **6.6.20** (`~/.cyrius/versions/6.6.20` `SOURCE_COMMIT` `ae682e23`, `tree-matches-tag: yes`). The user's chosen target: cyrius has tagged 6.7.x since (6.7.18 seen 2026-11-04), and every dep has newer 6.7-line tags |
| **tests** | **99 suites, 8,704 assertions, 0 failed** with wasmtime 50.0.1 present. 2.1.7's tree, run in the same shell the same day on its own pin (6.6.14), reads 8,701, and every suite's count is the same except `core_agent` +3, the arena fix's tests. So the move changes no assertion count |
| **coverage** | **99%**: 102/102 files, 1,635/1,647 fns (gate `--min 80`); the same 12 unreferenced |
| **dist** | `dist/agnosai.cyr` 39,894 lines + `dist/agnosai-guard.cyr` 941 (`wc -l`), both v2.1.8. Each now opens with a `# Requires` block (cyrius 6.6.18), and the sidecars list **7 / 1** stdlib leaves (2.1.7: 45 / 6) — see CHANGELOG 2.1.8 for what that asks of a consumer |
| **binary** | `build/agnosai` **5,337,008 B** (2.1.7: 5,253,128; text +81,312, data +1,960, all toolchain and stdlib); aarch64 **6,032,848 B** (2.1.7: 6,489,640), the same with `CYRIUS_DCE=1`. Both answer `GET /ready` with `"version":"2.1.8"`, the aarch64 one natively on the Pi; a 405 carries `Allow`; SIGTERM shuts down gracefully; no startup line carries a NUL |
| **lock** | `cyrius.lock` **119 files, 8 commit pins**, regenerated from empty (a re-resolve over 2.1.7's lock kept the old tags' pins beside the new: roadmap, *Recorded by cyrius 6.6.17*) |
| **gates** | `cyrius lib sync --full` (113 files) → `deps` → `check-symbols.sh` (2,786 definitions, both targets) → `check-clean.sh` (fmt 224, lint 125, doc 114, doctest 1 files; `deps --verify` 119; lib snapshot 113; log lengths, now self-tested; dist) → build → test → coverage → `distlib --all --check` → examples (1) → fuzz 4/4, then the benchmarks and the sibling-free, empty-cache replica (CHANGELOG 2.1.8 *Verified*) |
| **benchmarks** | `bench-history.csv` has 223 rows at 2.1.8, one unpinned sweep at a load of 4–5 (other sessions): median +3.4% against 2.1.7's rows, which were taken at a load of 1–2. The same-box interleaved A/B against a 2.1.7 build (3 rounds of all eleven files, then 5 of four) reads **median +0.0%**, 179 of 223 within ±5%. The channel paths (event bus, pub/sub, relay, approval) are 1.2x to 6x faster, from 6.6.16's eventcount channel. `agnos_form_encode_52b` is +15%, the compiler's code with identical source. The `default_model_*` rows swap again, which is placement (CHANGELOG 2.1.8 *Performance*) |
| **build warnings** | `build/agnosai`: toolchain notes only, the same set as 2.1.7 rebuilt on 6.6.14 (sigil's static array, large static data, the two "refusing to overwrite stdlib leaf" lines, the DCE note). 6.6.20 printed one more, `src/server/routes/a2a.cyr:245`, fixed |
| **consumer** | Not re-run against this tree. agnostic pins cyrius 6.6.14 and agnosai 2.1.7; thoth pins cyrius 6.6.6. Both meet the smaller sidecars at their re-pin |

Direct deps (`[deps.*]`, all six), **none with an active `path`**: `sigil` **3.13.10** (= the 6.6.20
fold, byte-identical), `bote` **3.3.17**, `majra` **2.9.3**, `kavach` **3.13.3**, `ai-hwaccel` **2.4.2**,
`tyche` **1.1.1** — each repo's release on the 6.6.20 line, not its latest tag. Transitively through bote:
libro **2.10.8**. Every dep's own pins name the 6.6.20 folds: kavach 3.13.3 and libro 2.10.8 declare sigil
3.13.10, libro patra 1.15.2, ai-hwaccel 2.4.2 and samay 1.1.7 bayan 1.5.12. Every tag was confirmed on
GitHub (`git ls-remote`) before the move.

### 2.1.7 — the release (2026-10-08)

| | |
|---|---|
| **version** | **2.1.7** (cut 2026-10-08): owed roadmap work and one addition, with no toolchain or dependency change — an HTTP server span an embedding consumer records through agnosai's exporter (ADR 024), `Allow` on a 405 (B8), `/mcp` `tools/call` of a tool that answers no output (B15), the OTLP exporter thread joined at stop (B15), 17 log lengths (B28), one whole-file reader (B19), the stdlib's `signal_default` (C2), and B26 explained |
| **cyrius pin** | **6.6.14** — unchanged; the move to 6.6.15 is on hold (no lower-lib re-releases for now) |
| **tests** | **99 suites, 8,679 assertions, 0 failed** in a plain shell with wasmtime 49.0.2 present. 2.1.6's tree, run in the same shell the same day, reads 8,617 (8,611 at its cut: `sandbox_spawn` and `sandbox_process` count environment variables), so the +62 are 2.1.7's tests: `telemetry_otlp` +28 (the HTTP span and its accessors, the exporter thread), `server_router` +27 (`Allow`), `server_serve` +4, `server_routes_mcp` +3 |
| **coverage** | **99%**: 102/102 files, 1,635/1,647 fns (gate `--min 80`); the 12 unreferenced are the same 12 as at 2.1.5 |
| **dist** | `dist/agnosai.cyr` **39,541** lines + `dist/agnosai-guard.cyr` 915, both v2.1.7 (2.1.6: 39,324 / 915); `distlib --all --check` current, and a `check-clean.sh` gate since 2.1.7 (B24) |
| **binary** | `build/agnosai` **5,253,128 B** (2.1.6: 5,248,824); aarch64 **6,489,640 B** (6,489,432), the same with or without `CYRIUS_DCE=1`. Both answer `GET /ready` with `"version":"2.1.7"`, the aarch64 one natively on the Pi; a 405 carries `Allow`; SIGTERM shuts it down gracefully. The aarch64 file grows in 64 KiB steps (CHANGELOG 2.1.7 *Verified*, B26): 55,624 B of text remain before the next |
| **lock** | `cyrius.lock` byte-identical to 2.1.6's: no dependency or toolchain change |
| **gates** | `cyrius lib sync --full` (112 files) → `deps` → `check-symbols.sh` (2,785 definitions, both targets) → `check-clean.sh` (fmt 224, lint 125, doc 114, doctest 1 files; `deps --verify` 118; lib snapshot 112; log lengths; dist) → build → test → coverage → `distlib --all --check` → examples (1) → fuzz 4/4 → `cyrius bench` (11 files, 223 benchmarks), then the sibling-free, empty-cache replica: all green. ⚠ The lint loop does not cover `tests/` (B31) |
| **benchmarks** | `bench-history.csv` has 223 rows at 2.1.7, one unpinned sweep at a 1-minute load of 1–2: median +1.1% against 2.1.6, 209 of 223 within ±5%. The six rows past ±10% were re-run interleaved against a 2.1.6 build: four are flat, `default_model_premium` is code placement, `crew_runner_10_tasks_parallel_4_quiet` is B27 (CHANGELOG 2.1.7 *Performance*) |
| **build warnings** | `build/agnosai`: toolchain notes only, the same four lines as 2.1.6. The full test log carries **no** `undefined function` line (23 at 2.1.6; B19) |
| **consumer** | agnostic 0.1.15 (pinned to 2.1.6): all 32 suites pass against this tree through a replica-only `path`, 2,701 assertions, each suite's count as against 2.1.6. It adopts ADR 024's span at its re-pin to 2.1.7 |

### 2.1.6 — the release (2026-10-04)

| | |
|---|---|
| **version** | **2.1.6** (cut 2026-10-04): five fixes from agnostic 0.1.14's review of its 2.1.5 re-pin — the OTLP exporter and the hoosh chat call bounded in memory and time, the generic OTLP endpoint read as a base URL (plus the per-signal one), a stranded `dag` run ends FAILED, a finished crew's cancel is refused (409) |
| **cyrius pin** | **6.6.14** — unchanged |
| **tests** | **99 suites, 8,611 assertions, 0 failed** in a plain shell with wasmtime present (2.1.5: 8,573). `server_auth` is counted from its own run: in the full run it failed to compile until the arena-pool include it was missing was added. The seven chat-path suites were re-run after the pool became lazy |
| **coverage** | **99%**: 102/102 files, 1,624/1,636 fns (gate `--min 80`) |
| **dist** | `dist/agnosai.cyr` **39,324** lines + `dist/agnosai-guard.cyr` 915, both v2.1.6 (2.1.5: 38,905 / 915); `distlib --all --check` current |
| **binary** | `build/agnosai` **5,248,824 B** (2.1.5: 5,240,304); aarch64 (DCE) **6,489,432 B**. `GET /ready` answers `"version":"2.1.6"`; SIGTERM shuts it down gracefully |
| **lock** | `cyrius.lock` byte-identical to 2.1.5's (and so to 2.1.4's): no dependency or toolchain change |
| **gates** | `check-symbols.sh` (2,764 definitions, both targets) → `check-clean.sh` → build (native, aarch64) → test → coverage → `distlib --all --check` → examples (1) → fuzz 4/4 → `cyrius bench`, then the sibling-free, empty-cache replica (see CHANGELOG 2.1.6 *Verified*) |
| **benchmarks** | `bench-history.csv` has 223 rows at 2.1.6, one unpinned sweep at a 1-minute load of 2–3: median +1.0% against 2.1.5, 174 of 223 within ±5%. The LLM-path crew rows are −27/−28% (the chat arena); `crew_runner_10_tasks_parallel_4_{quiet,watched}` read +22%/+19%, which a bisect shows is the heap position the traced block before them leaves, not the runner — 2.1.5 itself reads ~507 µs with that block removed (CHANGELOG 2.1.6 *Performance*; roadmap) |

### 2.1.5 — the release (2026-10-04)

| | |
|---|---|
| **version** | **2.1.5** (cut 2026-10-04): B17, F7 and F6, with ADRs 022 and 023 Accepted |
| **cyrius pin** | **6.6.14** |
| **tests** | **99 suites, 8,573 assertions, 0 failed** in a plain shell with wasmtime 49.0.1 (CI pins 47.0.3), so the WASM paths ran. ⚠ `sandbox_spawn` and `sandbox_process` assert once per environment variable, so the total moves with the environment (2.1.4: 8,065 in a plain shell) |
| **coverage** | **99%**: 102/102 files, 1,616/1,628 fns (gate is `--min 80`) |
| **dist** | `dist/agnosai.cyr` 38,905 lines + `dist/agnosai-guard.cyr` 915 lines, both v2.1.5 (distlib's counts; 2.1.4: 37,619 / 915); sidecars 45 / 6 leaves |
| **binary** | `build/agnosai` **5,240,304 B** (2.1.4: 5,223,328); aarch64 cross-build **6,423,568 B** (6,422,976). Both answer `GET /ready` with `"version":"2.1.5"`, the aarch64 one on the Pi |
| **lock** | `cyrius.lock` 118 files, 8 commit pins, byte-identical to 2.1.4's: no dependency or toolchain change |
| **gates** | `cyrius lib sync --full` (112 files) → `deps` → lock pin check → `check-symbols.sh` (2,733 definitions, both targets) → `check-clean.sh` (fmt 223, lint 124, doc 113, doctest 1 files; `deps --verify` 118; lib snapshot 112) → build → test → coverage → `distlib --all --check` → examples (1) → fuzz 4/4 → `cyrius bench` (11 files, 223 benchmarks): all green |
| **benchmarks** | `bench-history.csv` has 223 rows at 2.1.5 (212 at 2.1.4, plus 11 new rows), one unpinned sweep at a 1-minute load of 0.1 to 1.7. Read them against 2.1.3's (median +0.0%, 175 of 212 within ±5%), not 2.1.4's, which was pinned to one CPU at a load of 3–5. CHANGELOG 2.1.5 *Performance* |
| **build warnings** | `build/agnosai`: toolchain notes only (sigil's static array, large static data, and the two "refusing to overwrite stdlib leaf" lines for sigil and patra). The suite builds print **23** `undefined function '_agnosai_loader_read'` lines (roadmap B19) |

Direct deps (`[deps.*]`, all six), **none with an active `path`**, unchanged since 2.1.4:
`sigil` **3.13.7** (= the 6.6.14 fold, byte-identical), `bote` **3.3.16**, `majra` **2.9.2**,
`kavach` **3.13.2**, `ai-hwaccel` **2.4.1**, `tyche` **1.1.0**. The last five are their repos'
latest tags. sigil's repo is at 3.13.9, which cyrius 6.6.15 folds, so it moves with the pin
(roadmap, *Moving the cyrius pin to 6.6.15*). Transitively through bote: libro
**2.10.6**, which declares the folds' patra 1.15.1 and sigil 3.13.7. `cyrius.lock` holds 118
files and 8 commit pins, and every pin on a folded module names the fold `lib/` holds. Every
dep's own pins now name the folds too: kavach 3.13.2 declares sigil 3.13.7, ai-hwaccel 2.4.1
bayan 1.5.11. That holds only at the 6.6.14 pin: kavach 3.13.2 and libro 2.10.6 pin sigil 3.13.7
and cyrius 6.6.14, so they (and bote, for libro) need releases before the 6.6.15 move (roadmap).

### 2.1.5 — B17, F7 and F6 (the integration gate, 2026-10-04)

This gate ran on the working tree before the cut, with `VERSION` still 2.1.4; the release gate's
figures are in *Now*. The three item sections below record each item's own deltas. These are the
figures for all three together, from one gate run in CI order with `TMPDIR` off `/tmp`:

| | |
|---|---|
| **tests** | **99 suites, 8,573 assertions, 0 failed** (2.1.4: 8,065). wasmtime 49.0.1 present, so the WASM paths ran |
| **coverage** | **99%**: 102/102 files, 1,616/1,628 fns (gate `--min 80`) |
| **dist** | `cyrius distlib --all --check`: both bundles current. `dist/agnosai-guard.cyr` is unchanged from 2.1.4; sidecars 45 / 6 leaves |
| **binary** | `build/agnosai` **5,240,304 B** (2.1.4: 5,223,328) |
| **gates** | `cyrius lib sync --full` → `deps` (118 locked, 8 commit pins, `cyrius.lock` byte-identical before and after) → `check-symbols.sh` (both targets, 2,733 definitions) → `check-clean.sh` (fmt 223, lint 124, doc 113 files; `deps --verify` 118) → build → test → coverage → distlib → examples (1) → fuzz 4/4 → `cyrius bench` (11 files): all green |
| **build warnings** | `build/agnosai` as at 2.1.4: toolchain notes only. The full test log carries **23** `undefined function '_agnosai_loader_read'` lines from suite builds (roadmap B19, not from these items; it counted 20 at F7's gate) |

The bench rows these items claim reproduce in the gate's single `cyrius bench` run, against
`bench-history.csv`'s 2.1.4 rows: `tool_execute_echo` 3,834 → 823 ns, `score_agent_rich_context`
1,208 → 879 ns, `rank_agents_16` 11.9 → 7.9 µs, `delegate_16_tasks_16_agents` 191.3 → 125.7 µs,
`otlp_encode_span` 12.8 → 11.1 µs, `run_crew_10_tasks_sequential` 403.5 → 383.2 µs. B18's
`lt_aggregate_100k_500workers` reads 45.2 ms, unchanged. `bench-history.sh` was held for the release,
because it stamps `VERSION`; it ran at the cut (see *Now*).

### 2.1.5 — B17: crew events and status say what happened (ADR 022)

Shipped in 2.1.5. Measured on the working tree before the cut:

- **What changed** — all six of agnostic 0.1.9's findings, in
  [ADR 022](../adr/022-crew-events-and-status-say-what-happened.md): a parallel/DAG
  `task_started` at dispatch and `task_completed` per join; `token` events from parallel and DAG
  workers, carrying the spec's crew id in every mode; `metadata.agent` on LLM-answered results,
  with `agnosai_crew_profile_record_agent_cost` now summing; a timed-out runner reports `failed`
  with no results itself; the registry stores `running`, and the DAG error arm puts `pending` back.
- **New test seam** — the hoosh client's chat pointer (see *Test-design decisions* below), so the
  crew runner's LLM success arm runs offline. 22 mutants across the change, each killed, plus seven
  on the event gates. Those seven are killed by an exact check that an unwatched bus allocates
  what no bus does. Two of them, the wave's worker sender and dispatch-time `task_started`,
  survived the first review round.
- **Review fixes** — review found two more surviving mutants, and both are now killed. The
  first deferred each `task_completed` to the end of its batch. A stub now holds the second task
  of a two-wide batch until it sees the first task's `task_completed`. The second kept a timed-out
  crew's GenAI span OK, and `telemetry_wiring` now drains the OTLP ring after a run whose deadline
  has already passed. `docs/guides/api-reference.md` now documents the `running` → `pending`
  edge on the DAG error arm, and `POST /api/v1/crews` cannot reach that edge. That adds 7
  assertions (`orch_crew_runner` 283 → 286, `telemetry_wiring` 26 → 30), so the working tree,
  with F7 below included, runs **99 suites, 8,297 assertions, 0 failed**. No `src/` change, so
  dist, binary and benchmarks are as recorded.
- **tests** — **99 suites, 8,157 assertions, 0 failed** (2.1.4: 8,065). The +92 are all this
  change's: `orch_crew_runner` 208 → 276, `orch_orchestrator` 72 → 84, `llm_hoosh` 131 → 139,
  `core_crew` 99 → 103. **coverage** 1,579/1,592 fns, 102/102 files (99%).
- **dist** — `dist/agnosai.cyr` 37,841 lines (distlib's count; 37,619 at 2.1.4);
  `dist/agnosai-guard.cyr` unchanged. **binary** `build/agnosai` 5,227,440 B (+4,112).
- **Benchmarks flat** — `run_crew_*`, three interleaved rounds against a 2.1.4 build of
  `benches/orch.bcyr`, medians: `1_task_sequential` 99.1 → 99.0 µs, `10_tasks_sequential`
  399.7 → 405.5 µs (ranges overlap), `10_tasks_parallel_4` 615.4 → 618.9 µs. Those rows have no
  event bus, so they cannot see the wave change. Four new rows,
  `crew_runner_10_tasks_parallel_4_{watched,quiet,llm_watched,llm_quiet}`, run the runner with a
  sender attached. The two placeholder rows ran eight interleaved rounds against 2.1.4, medians:
  `_watched` 710.6 → 699.1 µs, `_quiet` 648.0 → 641.3 µs. The LLM rows are new-only
  (CHANGELOG *Performance*). `bench-history.sh` was held for the release, because it stamps
  `VERSION`.
- **agnostic** needs only the re-pin and a comment refresh (its "registry never reports RUNNING"
  comments, ADR 0009's note, Swarm Command's workaround comments).

### 2.1.5 — F7: explain agent selection (library only)

Shipped in 2.1.5. Measured on the working tree beside B17:

- **What changed** — `agnosai_explain_selection_a` returns each agent's total and five component
  scores in the runner's rank order, so entry 0 is the agent `_agnosai_crew_pick_best_agent`
  assigns (a test pins it). `agnosai_selection_to_value_a` and
  `agnosai_selection_scorer_to_value_a` render the JSON. Library only: the wire is unchanged, so
  there is no ADR. Personality is reported as its neutral 0.5 and listed as `unmeasured`.
  Nothing is retained on the crew path: a consumer recomputes, which is exact while selection is
  a pure function of roster and task.
- **Prerequisite, bite 1** — the scorer's four context keys are module globals, the "medium"
  default is `AGN_JV_MEDIUM`, and the domain compare is `agnosai_str_eq_ci` on the two `Str`s,
  with no `str_cstr` copy. The component scorers and `agnosai_score_agent_with_tools` now
  allocate nothing. `agnosai_score_agent` still extracts `required_tools` into a 152 B vec on the
  global bump when the task has them, and `agnosai_rank_agents` still allocates its entries. A
  domain with an embedded NUL no longer matches its prefix. `AGN_JK_AGENT_KEY` moved to
  `core/json.cyr`.
- **tests** — **99 suites, 8,297 assertions, 0 failed** on the working tree (B17's tree: 8,157;
  B17's review fixes add 7). The +133 are F7's: `orch_scoring` 109 → 235 and `orch_crew_runner`
  276 → 283. 32 mutants killed, 3 equivalent (CHANGELOG *Tests*). **coverage** 1,587/1,600 fns
  (+8 public), 102/102 files (99%).
- **dist** — `dist/agnosai.cyr` 38,145 lines (distlib's count; 37,841 after B17);
  `dist/agnosai-guard.cyr` unchanged. **binary** `build/agnosai` 5,231,792 B (+4,352 on B17's).
- **Benchmarks** — five interleaved rounds of the scoring rows against the pre-F7 tree, medians:
  `score_agent_*` −22% to −41% (rich context 1,185 → 922 ns), `rank_agents_varied_100/300/1000`
  −21% / −21% / −19%, `rank_agents_16` −30%, `delegate_16_tasks_16_agents` −32%,
  `run_crew_10_tasks_sequential` −1.6% (ranges overlap). Two new rows: `explain_selection_100`
  74.1 µs and `selection_to_value_100` 56.3 µs. The full table is in CHANGELOG *Performance*.
  `bench-history.sh` was held for the release.
- **agnostic** surfaces it when it re-pins: `GET /api/v1/crews/{id}/plan?explain=selection`,
  with its own ADR for recomputing rather than recording. It shows that agnostic's selection is
  degenerate today: it forwards no task context, so tools, domain and GPU score 1.0 for every
  agent and complexity alone decides.

### 2.1.5 — F6: OTel GenAI semconv spans and inbound trace context (ADR 023)

Shipped in 2.1.5. Measured on the working tree beside B17 and F7. agnosai's half only — agnostic's
lands at its re-pin.

- **What changed** — [ADR 023](../adr/023-genai-semconv-spans-and-w3c-trace-context.md),
  amending ADR 017. Four semconv operations, linked parent to child under one trace per crew in
  every process mode: `invoke_workflow {crew}`, a new per-task `invoke_agent {agent}`, the hoosh
  call re-labelled `chat {model}` (it was the span called `gen_ai.invoke_agent`), and
  `execute_tool {tool}`. `gen_ai.provider.name`, `gen_ai.tool.name`, `gen_ai.workflow.name`,
  `error.type`, and the agent key as `agnosai.agent.key`. Pinned at
  `semantic-conventions-genai` `e07f4eba` (gen-ai-dev/1.42.0-dev on base v1.44.0,
  Development). Span contexts are threaded explicitly (runner → wave job → task → retry context
  → the hoosh client's chat pointer → `agnosai_hoosh_chat_in`), never thread-local. Inbound
  `traceparent`: `agnosai_crew_with_trace_parent` in process, and the header on
  `POST /api/v1/crews`, `POST /api/v1/a2a/receive` and `/mcp` `tools/call`. ParentBased sampling.
  `agnosai_telemetry_init_export` exports without touching sakshi. With the exporter off no span
  site builds anything.
- **Deviations from the plan, recorded in ADR 023:** the agent key goes out as
  `agnosai.agent.key`, because the pinned registry reserves `gen_ai.agent.id` for hosted agents;
  the task entry point is ADR 022's `agnosai_execute_task_in_crew` with a trailing `parent_sc`,
  not a new `agnosai_execute_task_in`; the chat pointer's contract gained the fourth argument.
- **tests** — **99 suites, 8,557 assertions, 0 failed** on the working tree (8,297 before
  F6). 60 mutants across five bites, all killed; three survived their first run (CHANGELOG
  *Tests*). **coverage** 1,616/1,628 fns, 102/102 files (99%); every new public fn is referenced.
- **dist** — `dist/agnosai.cyr` 38,905 lines after fix round 1 (distlib's count; 38,893 at F6,
  38,145 after F7), `.deps` unchanged at 45 leaves; `dist/agnosai-guard.cyr` unchanged.
  **binary** `build/agnosai` 5,240,304 B (+8,512 on F7's; unchanged by fix round 1).
- **Benchmarks** — against a pre-F6 build of this tree, interleaved: `tool_execute_echo`
  3.67 → 0.79 µs (−78.5%, the instrumentation it paid with telemetry off), `run_crew_*` −0.4% to
  −2.4%, the OTLP encode/enqueue/drain rows −6.5% to −17% on a re-based `chat` fixture. Three new
  rows: `otlp_encode_span_child` 11.54 µs, `otlp_span_context_parse` 303 ns,
  `otlp_span_context_child` 501 ns. `crew_runner_10_tasks_parallel_4_quiet` reads +19% in the
  full file and equal when run alone; CHANGELOG *Performance* has both. `bench-history.sh` was held
  for the release.
- **agnostic, at its re-pin:** `agnosai_crew_with_trace_parent(spec, agnostic_trace_current())`
  where it submits a crew; `agnosai_telemetry_init_export` at boot when
  `OTEL_EXPORTER_OTLP_ENDPOINT` is set, and `agnosai_telemetry_shutdown` on shutdown; tighten its
  traceparent check from length-only to the W3C shape. Its roadmap's "sakshi W3C trace id" is
  agnostic's own thread-local (`src/trace.cyr`), not sakshi's.
- **Fix round 1 (review findings)** — the runner's `invoke_agent` span is asserted with an agent
  ASSIGNED, in `telemetry_wiring` and end to end from the wire in `server_serve`: before, every
  traced crew was unassigned and swapping the agent's name and key on the span survived every
  suite (mutant M8; now 4 failures in `telemetry_wiring`, 3 in `server_serve`). The chat span's
  transport `error.type` is sandhi's kind lowercased (`connect`, `timeout`), its value pinned. Both
  `llm_hoosh` chat-pointer `callptr`s pass the fourth argument. The inbound-traceparent trust
  boundary is threat-model surface 7 and in ADR 023, with an unsampled `-00` header pinned at the
  wire. Two telemetry-on bench rows price the 1 + N spans: a 10-task crew costs **+228 µs
  sequential, +137 µs parallel_4** with the exporter on (2.1.4: +13 / +14 µs, one span), measured
  against an idle-thread baseline because a second live thread alone moves these rows (B20).
  **tests** — **99 suites, 8,573 assertions, 0 failed** (+16: `telemetry_wiring` 91 → 102,
  `server_serve` 306 → 311). **coverage** 1,616/1,628 fns, 102/102 files (99%).
- **Left:** hoosh's own span id and `parentSpanId`, then agnosai's outbound header (roadmap C,
  hoosh row, to file); the rest of B7, which now includes the opt-in `delegate` tool's unlinked
  trace (item 9) and an operator switch for an untrusted inbound context (item 10); B20 (a second
  live thread moves crew cost) and B21 (two drainers share the OTLP doc arena at shutdown).

### 2.1.4 — kavach 3.13.2 and ai-hwaccel 2.4.1; no code warning anywhere

- **Two dependency releases, made for this one.** ai-hwaccel 2.4.1 (cyrius 6.6.14, bayan 1.5.11,
  the three deprecated getter calls renamed), then samay 1.1.6 (the same, 18 calls) and kavach
  3.13.2 (its 3.13.2 work plus cyrius 6.6.14, sigil 3.13.7, ai-hwaccel 2.4.1, samay 1.1.6). Each
  was certified sibling-free before the next could resolve it. No agnosai `src/` change but the
  version literal.
- **What kavach 3.13.2 changes under agnosai:** wasmtime runs under kavach's exec seccomp filter
  (a host without seccomp refuses with 125; agnosai's CI is x86-64 only); the process capture
  cuts a payload off at 1 MiB of stdout with SIGPIPE, keeps each result's stderr, kills payloads
  with their kavach, and resets SIGPIPE to default; OCI runs take the config's deadline and
  stdin. The unset-stdin and TCP-port changes do not reach agnosai (it sets `config_stdin` on
  both paths and no port counts). All 99 suites pass against it.
- ⚠ **Open in kavach, reaching the sandbox** (roadmap C): uutils coreutils (Ubuntu 25.10+)
  refuse kavach's pinned exec, so `/bin/echo` and the like exit 1 on such a host; and the
  `basic` profile has no `sendto` for cyrius 6.6.14's TLS writes. `ubuntu-latest` became 26.04
  and the first half turned CI red; since 2.1.8 the bridge suite execs itself, not coreutils.

### 2.1.3 — cyrius 6.6.14; the build is warning-free and the lock is CI's

- **The working tree resolves exactly as CI does.** The `path` overrides on bote, majra and
  tyche are commented out (ai-hwaccel's already was), so a local `cyrius deps` writes the
  same 8-pin lock a runner writes. A sibling-free replica with an empty dep cache reproduces
  `lib/` and `cyrius.lock` byte for byte; with the overrides active it could not, since the
  local lock had no pin for the three and `../bote` chained to `../libro`.
- **A three-repo chain was released for one lock line.** bote 3.3.15 pinned libro 2.10.4,
  which declared patra 1.14.3, so agnosai's lock named a patra commit `lib/` did not hold.
  libro 2.10.6 (sigil 3.13.7, cyrius 6.6.14) and bote 3.3.16 (libro 2.10.6, cyrius 6.6.14, no
  `path` lines) were released first, each certified the same sibling-free way.
- ⚠ **From cyrius 6.6.11 a thin profile of a stdlib leaf is not vendored.** A consumer that
  lists `sigil` in `[deps] stdlib` (bote) keeps the fold and never gets libro's thin sigil
  files ("not vendored" notes in `cyrius deps`). agnosai is unaffected either way: its own
  `[deps.sigil]` wins closest-wins, so libro's sigil block is skipped.
- **Deps whose own tags still pin a module off the fold** (all harmless, recorded): kavach
  3.13.1 declares sigil 3.12.18 (skipped, closest-wins) and ai-hwaccel 2.4.0 declares bayan
  1.5.6 (the stdlib leaf wins) and calls the deprecated getter three times.
- **Benchmarks.** `bench-history.csv` has its first sweep since 2.0.6 (212 rows, load
  0.7–1.3). An interleaved A/B against 2.1.2 on 6.6.12 is a wash: overall median −0.8%, and
  the quiet re-runs of `server` and `orch` +1.2% / +0.6%. ⚠ A first pass of those two read
  +6% / −5% while another session held 7 cores, and neither reproduced. **Interleave, and
  re-run anything measured above a 1-minute load of about 2.**

### 2.1.2 — cyrius 6.6.12 and every dep at its latest

A toolchain-and-pins release so that a consumer linking `dist/agnosai.cyr` gets current
versions from agnosai itself. Agnostic 0.1.7 carried root-level pins for bote, majra,
ai-hwaccel and tyche "ahead of" agnosai's; it turned out they never took effect (a root
`[deps.X]` with no `modules` is never cloned), and with this release they are not needed.

- **dist is unchanged but for the version.** `dist/agnosai.cyr` differs from 2.1.1 in the
  header and the `AGNOSAI_VERSION` literal only; the working tree's regeneration and the
  clean copy's are byte-identical.
- **`check-lib-symbols.py` now evaluates `#ifdef` per target.** sigil 3.13.5 declares eight
  errno names under `#ifdef CYRIUS_TARGET_MACOS` and again under `#ifndef`, and the gate
  read both arms as live — a false positive against kavach on every target built here.
  Rules 4 and 5 now run once each for x86_64-linux and aarch64-linux. Mutation-checked in
  both directions (see the CHANGELOG).
- ⚠ **Locally, `lib/libro.cyr` is NOT the tag's.** bote's `path = "../libro"` chains from
  this repo's `path = "../bote"`, so a local resolve reads the `../libro` checkout. dist does
  not contain libro and is unaffected; the suites above ran on the tag-resolved copy.
  ✅ Resolved at 2.1.3: the `path` overrides are commented out and bote 3.3.16 commits none.

### 2.1.1 — kavach's backend enum, named by its own name

This is a build-compatibility patch and nothing else. Seven code sites spelled `Backend.X`, but
kavach renamed that enum `KavachBackend`. Up to cyrius 6.6.10 the compiler ignored the qualifier,
so they compiled. 6.6.11 refuses each one with `'Backend' is not an enum`. They now read
`KavachBackend.X`, and `dist/agnosai.cyr` is regenerated.

- On the pinned 6.6.6, HEAD with and without the fix builds a byte-identical `build/agnosai`
  and `tests/sandbox_kavach_bridge.tcyr`.
- On the 6.6.11 compiler, the old tree fails with those seven errors. The fixed tree builds,
  and `sandbox_kavach_bridge` (138) and `sandbox_wasm` (66) pass.
- The cyrius pin stays 6.6.6. `rust-old/` is still present: this patch does not carry the
  deletion that the 2.1.0 notes below schedule for "the next release".

### 2.1.0 — the `rust-old/` audit

Seven read-only audits compared every item and every `#[test]` in `rust-old/` with the Cyrius
tree: orchestrator, server, fleet, tools, core + llm + telemetry, sandbox + definitions +
learning, and everything outside `src/`. Every `.rs` file has a counterpart, and nothing outside
`rust-old/` reads it. **It is scheduled for deletion in the next release**; the checklist and
the remaining findings (B4–B16) are in [`roadmap.md`](roadmap.md). The easy findings were fixed
here, each with a test that fails on the old code — see the CHANGELOG.

**Measured, not assumed:**

- The IPC SIGPIPE fix: forcing the old `write(2)` path makes the new test die with exit 141.
- The stats leaks: restoring the old accessors fails the LIFO-freelist assertions.
- The server path: `benches/server.bcyr` on 2.0.10 against 2.1.0, same machine and session — all
  33 rows within ±5.4%, most within ±2%, so no regression is claimed and no speed-up either.
  Single runs; the full `bench-history.sh` sweep was not recorded for this release.

⚠ **Not covered by a test:** the load tester's new timeout, response cap and header arena sit on
the network path, which needs a live target that no suite stands up (B13).

## 2.0.10 — kept for the record

| | |
|---|---|
| **version** | **2.0.10** |
| **tests** | 99 suites, 7,975 assertions, 0 failed |
| **coverage** | 99% — 103/103 files, 1,578/1,585 fns |
| **binary** | `build/agnosai` 5,112,352 B |

Direct deps (`[deps.*]`, all six): `sigil` **3.12.18** (= the 6.6.6 fold, byte-identical),
`bote` **3.3.13**, `majra` **2.9.1**, `kavach` **3.13.1**, `ai-hwaccel` **2.4.0** (`path`
commented out — its sibling sits a docs-only commit past the tag), `tyche` **1.1.0**. Folded
from the toolchain: sandhi 1.9.17, sakshi 2.5.2, patra 1.14.3, bayan 1.5.6, sankoch 2.8.0 (plus
vani, yukti, niyama, mabda, ganita, yantra). Transitively through bote: libro 2.10.3.

### ⚠ What will bite the next person — learned at 2.0.10

**1. A stdlib-folded dep is pinned to the FOLD, never to its repo's newer tag.** The fold table
is cyrius's `docs/ecosystem.md` at the pinned tag. At 2.0.10 sigil's repo was at 3.13.2 and
`../patra` at 1.15.0; 6.6.6 folds 3.12.18 and 1.14.3, and those are what agnosai compiles. The
resolver refuses to overwrite a folded leaf ("refusing to overwrite stdlib leaf 'patra'"), but
see 3 for what it does NOT protect.

**2. After a tag bump, finish with a plain `cyrius deps`, not `cyrius deps --lock`.** From 6.6.4
the bare `--lock` verb resolves nothing and carries the OLD lock's `commit` pins forward: after
the three-step it wrote pins naming sigil 3.12.16 / kavach 3.12.5 and none for ai-hwaccel. A
plain `cyrius deps` wrote the right three.

**3. If agnosai's own sigil resolution fails, libro's `[deps.sigil]` takes over — from
`../sigil`.** libro 2.10.3 declares `[deps.sigil] path = "../sigil"` with four extra modules
(`dist/sigil-mldsa.cyr`, `src/sha_ni.cyr`, `src/sha256.cyr`, `src/hex.cyr`). Normally
closest-wins skips it. When the primary resolve failed (see 4), the walk vendored those four
from the local **3.13.2** checkout into `lib/`. A normal resolve never includes them (checked
with `cyrius build -v`), but they stay on disk and in the lock until `lib/` is regenerated:
`rm -rf lib && cyrius lib sync --full && cyrius deps` (`lib/` is gitignored and toolchain-owned).

**4. `/tmp` is a tmpfs with a per-user quota.** Hitting it (99 retained ~4.5 MB suite binaries
plus two scratch tree copies) produced `could not write the output … errno 122` (EDQUOT), a git
"refusing tampered cache" on `~/.cyrius/deps/sigil/3.12.18` (the cache was fine — `git fsck`
clean) and "cannot hash the pinned snapshot". Delete each test binary after running it.

**5. `file_exists` cannot see a Unix socket.** It `open`s the path, which fails with ENXIO on a
socket. Test for presence with `xstat`.

**6. The rate-limit key cap is bounded by time under a strict rate** — majra 2.8.1. 72,447 keys
at 1 req/s against a 4,096 cap (the default 100 req/s holds). Owed as roadmap **B4**, requested
from majra.

### Found by the 2.0.10 dependency review — not caused by it (status as of 2.1.0)

- ✅ **2.1.0:** `agnosai_relay_message_release` lets a subscriber drop a delivered message (refcounted since majra 2.9.0).
- ✅ **2.1.0:** the `ratelimit_stats` / `relay_stats` snapshots are freed after reading.
- ✅ **2.1.0:** process/OCI tools get an empty stdin (`config_stdin(c, "", 0)`) instead of the
  server's.
- ✅ **2.1.0:** the relay and `pq_enqueue` prose is corrected. Still open: ~100 `lib/<file>.cyr:NNN`
  line citations in comments (into majra, bench, bote-core and others) rot with every dependency
  bump and several no longer point at the cited code — citing function names would not.

## 2.0.6 — kept for the record

| | |
|---|---|
| **version** | **2.0.8** |
| **cyrius pin** | **6.5.35** (folds patra 1.13.10 — required, see below) |
| **tests** | **99 suites, 7,944 assertions, 0 failed** |
| **coverage** | **99%** — 103/103 files, 1,578/1,585 fns (gate is `--min 80`) |
| **dist** | `dist/agnosai.cyr`, 37,268 lines, stamped v2.0.6 |
| **binary** | `build/agnosai` **5,119,848 B** |
| **gates** | `check-symbols.sh` (**5 rules**) · `check-clean.sh` · `distlib --check` · fuzz 4/4 green |

### ⚠ Four things that will bite the next person

**1. The Cyrius pin and a transitive `[deps.patra]` are ONE change.** libro
declares `[deps.patra] = 1.13.10`; `cyrius deps` overlays a declared dep's copy on
top of the `lib sync --full` snapshot on every resolve. Against a pin that folds a
different patra, that puts a non-matching file in `lib/` and `check-clean.sh`'s
lib-snapshot rule — which allows **no** file to differ, on purpose — fails. 6.5.35
folds 1.13.10 and libro 2.8.12 declares 1.13.10, so pin and overlay agree and the
gate passes **108/108**. The delivery path is `patra tag → fold into the cyrius
stdlib → cyrius release → consumer pin bump`; a consumer sitting between steps 2
and 4 fails this gate every time. Do not reach for a `[deps.patra]` hold or a
`check-clean` allowance — bump the pin.

**2. `~/.cyrius/versions/<pin>/bin/cyrius` does NOT pin `cycc`.** ⚠ **Stale as of 6.6.x** — on 2026-09-26 `cyrius build -v` under a 6.6.2 pin reported `compiler: ~/.cyrius/versions/6.6.2/bin/cycc`, and the build printed 6.6.2's own diagnostic text, so the pinned wrapper now resolves its own compiler. Kept for the history: It resolves the
compiler through `$CYRIUS_HOME/bin` → `~/.cyrius/current`, not relative to itself
and not via `PATH`. Build a `CYRIUS_HOME` shim (`bin`, `lib`, `versions`, `deps`
symlinks + a `current` file) and read the drift line to confirm which compiler you
actually got — under the shim it reads `cyrius 6.5.34 / manifest-pin: 6.5.35
(drift — wrapper is 6.5.34)`, which is how the A/B below was measured. ⚠ And
**never read `~/.cyrius/versions/<V>/lib/` as ground truth** — a concurrent cyrius
session rewrites those files in place. Use `git show <tag>:lib/<mod>`.

**3. The GitHub tags API does not return tags in version order.** Reading its
first row picks `3.9.x` over `3.12.x`. Every pin must come from a `sort -V` of the
full list — and then be cross-checked against each sibling's `VERSION`, because a
version prepared but never tagged is the *other* half of the trap (it bit the
2026-08-12 four-repo sweep). At 2.0.6 all eight repos have
`VERSION` == local tag == remote tag.

**4. A divergent `fn` shared by two deps now fails the build — Rule 5.** It used to
be caught by nothing: Rule 4 covers `var` and enum members on the reasoning that cycc
*warns* for a duplicate `fn`, and it does warn, but no gate read the warning.
`scripts/check-lib-symbols.py` Rule 5 closes it, deferrals in
`scripts/lib-fn-allow.txt`. Like Rule 4 it fails on **divergence**, not duplication —
**12** duplicates are byte-identical (kavach and sigil vendor the same `agnosys`
layer) and pass silently; **6** diverge and are allowlisted with reasons. ⚠ The two
that matter are `_sub_new` (majra `fl_alloc(40)` vs libro `alloc(24)`) and
`attestation_result_new` (sigil 16 B vs kavach 48 B, unrelated constructors) — both
unreachable *here*, neither fixed upstream.

### 2.0.6 — what moved, and the one defect it exposed

**The pin bump is pure codegen.** `git diff --stat 6.5.34 6.5.35 -- lib/` is
**empty** — the folded stdlib is byte-identical across the two pins, so the bump
moves only the code generator. That made a clean A/B possible (same `src/`, same
`lib/`, two compilers via the shim in note 2):

| | 6.5.34 | 6.5.35 |
|---|---|---|
| `build/agnosai` | 5,156,712 B | **5,119,848 B** (−36,864, −0.71%) |
| frame (`%rbp`-relative) accesses | 163,654 | **154,077** (−5.9%) |

⚠ **No runtime win is claimed.** Cyrius measured one at best-of-7 and it collapsed
to ±2% at best-of-25. Smaller binaries using more registers is the honest result.

**`AGNOSAI_VERSION` had drifted for six releases.** It read the Rust oracle's
**1.1.0** while `VERSION` had said 2.x since 2026-08-14, so `GET /ready` and the
MCP `initialize` `serverInfo` announced 1.1.0 from a 2.0.5 binary. Fixed to
`2.0.6` and verified **on the wire**, not inferred from the rebuild.

⚠ **The three tests covering that field cannot catch it, by construction.**
`server_routes_health.tcyr` (×2) and `server_routes_mcp.tcyr` assert the response
against `AGNOSAI_VERSION` — the *symbol*, not the file — so they are
self-referential and stay green at any value. **Both halves of the guard shipped in
2.0.6**: `tests/server_routes_version_pin.tcyr` compares the constant to the root
`VERSION` file, and `scripts/version-bump.sh` refuses the bump before writing
anything if they disagree.

⭐ **The mutation demonstrates the gap rather than asserting it.** Restoring the
real historical `1.1.0` fails the new suite while `server_routes_health` and
`server_routes_mcp` report **2 passed, 0 failed** on the identical mutation.

### ⚠ OPEN — a ~3.6x regression, found at 2.0.6, NOT caused by it

**At 2.1.3 (2026-10-03, cyrius 6.6.14):** `lt_aggregate_100k_500workers` reads **45.4 ms**,
halved since 2.0.6's 88.2 ms but still about twice the 2026-08 band. Still open.

The benchmark sweep had not run since **2026-08-13** (10 days, 5 releases).
Running it at 2.0.6 surfaced three regressions that were already in the tree:

| benchmark | 08-1x avg | 2.0.6 | |
|---|---|---|---|
| `lt_aggregate_100k_500workers` | ~24 ms | **88.2 ms** | ~3.6x |
| `durable_load_miss` | ~2.6 us | **4.50 us** | +73% |
| `durable_load_hit` | ~4.2 us | **6.45 us** | +54% |
| `run_crew_10_tasks_parallel_4` | ~497 us | 572 us | +15% |

**Three things are already ruled out, by measurement:**

1. **Not the compiler.** Same source, two compilers via the `CYRIUS_HOME` shim —
   `lt_aggregate` 88.536 ms on 6.5.35 vs **88.928 ms on 6.5.34**; the durable pair
   likewise within noise. If anything 6.5.35 is faster.
2. **Not the benchmarked source.** `src/tools/builtin/load_testing.cyr` and
   `benches/tools.bcyr` differ from the 08-13 baseline by **reindentation plus one
   divide-by-zero guard**. `_b_workers(500, 200, 3, …)` measures identical work.
3. **Not noise.** Three repeats land 88.321 / 88.277 / 88.650 ms — a ±0.6% spread.

**Where to start.** `lt_aggregate`'s history is a step function: ~80-88 ms through
2026-07-31, **~26 ms from 2026-08-03**, ~23-25 ms by 08-11, and 88.2 ms today.
Today's value is the *pre-optimisation* one, so whatever landed on 2026-08-03 has
been defeated. That window is the `_a` allocator threading (roadmap **B2**) and
the arena work around it, and `durable_load_*` moving together points the same
way. **40 commits** sit between the baseline and 2.0.6; a bisect is ~6 build+bench
rounds.

⚠ **`fleet_reach_barrier_3nodes` is NOT affected**, though a first pass here said
it was: against the 08-1x *average* (2.03 us) today's 2.04 us is unchanged, and
the apparent +38% came from differencing one 08-13 row that was an outlier low.
**Bound against a period band, never a single row.**

⚠ **The process failure is the real lesson.** CLAUDE.md's work loop puts
benchmarks at steps 4 and 8 and says *"Never skip benchmarks. Numbers don't lie."*
Five releases shipped without a sweep, so a 3.6x regression sat in `main`
unreported. `cyrius bench` runs in CI — but that step exists for the **compile**,
not the numbers, and it says so; nothing compares timings against the CSV.

### What was verified rather than assumed at this bump

- **All 24 majra entry points this repo uses are signature-identical** across
  2.6.7 → 2.7.0. agnosai reads exactly one majra struct by raw offset —
  `ratelimit_stats`'s 32-byte result at 0/8/16/24 — and that function is
  byte-identical between the tags, despite 2.7.0 growing five internal layouts.
- **`src/` calls zero bote-core functions**, by design (see
  `src/server/routes/mcp.cyr`). The bundle grew 236 → 249 fns; the binary executes
  none of them. bote's four releases reach `dist/agnosai.cyr` consumers only.
- **Every vendored bundle hash-matches its pinned tag** (all seven), so the local
  build — where `path = "../NAME"` wins — and CI, which resolves `git` + `tag`,
  compile identical bytes. `majra` and `ai-hwaccel` each sit one docs-only commit
  past their tag, which is exactly when that stops being true by accident.

### ⚠ Inherited from libro — for consumers, not for agnosai

libro 2.8.11/2.8.12 changed the entry-hash preimage, the Merkle construction
(RFC 9162 domain tags; odd trailing node **promoted**, not self-paired) and the
tree-head signature. **A chain written by libro ≤ 2.8.10 will not verify under
2.8.12.** Nothing here migrates — `src/` has no `libro_*` / `chain_*` / `merkle_*`
/ `anchor_*` call site, and `src/orchestrator/audit.cyr` is agnosai's own chain,
keyed per-process and deliberately never snapshotted. But anyone serving a durable
audit trail behind bote's `libro_*` tools through `dist/agnosai.cyr` takes libro's
migration: re-export and re-anchor before upgrading.

### Kept from 2.0.4 — the collision class that started the lib-symbol gate

kavach and ai-hwaccel both defined `var BACKEND_COUNT` (10 vs 18); Cyrius has one
flat symbol table with last-definition-wins and is **silent** on a duplicate
`var`, so kavach's `_backend_fp` bounds check read 18 against a 10-slot table.
That is why `scripts/check-lib-symbols.py` exists. ⚠ Still open: kavach's enum
*members* (`PROCESS`, `WASM`, `OCI`, `NOOP`) remain generic and unprefixed, and
kavach and sigil share the whole `syserr_*` family — `fn` collisions, so they warn
at build time rather than resolving silently. See note 4 on what that is worth.

---

## Historical — the 2026-08-12 four-repo sweep *(resolved; kept for the record)*

## ✅ RESOLVED 2026-08-12 — the four-repo chain is closed at every root

**The handoff below is complete.** All four repos landed in order, each verified
before the next, and agnosai is on **cyrius 6.5.20** with `lib/sakshi.cyr`
holding at the snapshot's 2.4.10 after a build. What each repo needs from the
project owner is a **commit + tag** — nothing is left to build.

| repo | version | tagged on GitHub? | action |
|---|---|---|---|
| **patra** | 1.13.0 | ✅ yes | none — already released |
| **majra** | **2.6.3** | ❌ no (2.6.2 is) | commit + tag `2.6.3` |
| **libro** | **2.8.5** | ❌ no (2.8.4 is) | commit + tag `2.8.5` |
| **bote** | **3.3.1** | ❌ no (3.3.0 is) | commit + tag `3.3.1` |
| **agnosai** | **2.0.0** (cut 2026-08-14) | — | commit + tag `2.0.0` |

Version rule applied: a version already published on GitHub got a **patch bump**
(libro 2.8.4 → 2.8.5); a version prepared but never tagged absorbed the new work
**without a second bump** (majra 2.6.3, bote 3.3.1). Confirmed against the GitHub
tags API, not local tags — local and remote agreed in all four cases.

⚠ **Until those tags exist, every `[deps.*]` in this chain resolves through
`path = "../NAME"`, not the tag** — that is the local-path-beats-tag hazard
recorded further down this file, and it is live right now by construction, since
each manifest names a tag that has not been pushed yet. **It was checked rather
than assumed**: every vendored bundle is byte-identical to the `dist/` its repo
will publish, so a clean CI checkout resolving tags builds exactly what was
tested here.

| consumer's vendored copy | source `dist/` | sha256 (first 16) |
|---|---|---|
| `bote/lib/libro.cyr` | `libro/dist/libro.cyr` | `ad03fe77af2b4ac4` |
| `agnosai/lib/libro.cyr` | `libro/dist/libro.cyr` | `ad03fe77af2b4ac4` |
| `agnosai/lib/bote-core.cyr` | `bote/dist/bote-core.cyr` | `f87c7696e69b96b8` |
| `agnosai/lib/majra.cyr` | `majra/dist/majra.cyr` | `491c0bfadfb0df31` |

⚠ **Re-run that comparison if any of the three trees is edited before tagging** —
it is only true while the working trees and the bundles agree.

### ⚠ Four corrections to the plan this section used to carry

Each was established by measurement and each is the **opposite** of what the
handoff instructed. Do not restore the originals.

1. **cyrius 6.5.20 was never the blocker — 6.5.19 already folded patra 1.13.0.**
   patra 1.13.0 was committed and tagged **2026-08-11 20:02:27**; 6.5.19's
   binary was built **2026-08-11 20:09:37**, seven minutes later. All three of
   `6.5.19/lib/patra.cyr`, `6.5.20/lib/patra.cyr` and `git show
   1.13.0:dist/patra.cyr` hash to `b21cfd17…`. **The work was unblocked before
   the release it was waiting on shipped.**

   ⚠ This also retires the "snapshot overwritten in place" suspicion for this
   case: `6.5.19/lib` holding patra 1.13.0 is **correct**, not corruption.
   6.5.18 → 6.5.19 differs by exactly **one** file (patra.cyr), and
   **6.5.19/lib → 6.5.20/lib differs by zero** — 6.5.20 is a pure compiler
   release.

2. **libro must KEEP `[deps.patra]`, bumped rather than deleted.** The handoff
   said to drop it in favour of a `[deps].stdlib` entry. bote consumes
   `dist/libro.cyr`, which calls `patra_*` **54 times and defines none**, and
   bote declares no patra of its own — so bote can only obtain patra by
   resolving libro's block transitively. Deleting it strands bote.

3. **libro must KEEP its THIN `[deps.sigil]` sub-bundle selection.** The handoff
   said to add `"sigil"` to `[deps].stdlib`. The fold is the **monolith**:
   `lib/sigil.cyr` is 27,550 lines / 1,079,160 bytes carrying the x509+RSA
   bignum banks. Measured both ways — majra, which takes `dist/sigil.cyr`,
   builds with `warning: large static data (10794336 bytes)`; libro's thin build
   is `.bss` **80,224 bytes**. Only the **tag** needed to move, 3.12.1 → 3.12.7.

4. **libro fed the stale-sakshi overlay through TWO pins, not one.** Alongside
   patra 1.12.12's `[deps.sakshi]` 2.4.2, **sigil 3.12.1's own manifest declared
   `[deps.sakshi]` at 2.4.3**. Both terminate at the tags now pinned (patra
   1.13.0 and sigil 3.12.7 each declare zero `[deps.*]`).

### ⚠ Two defects surfaced by this work, neither blocking

- **libro's `dist/libro.deps` no longer declares `sakshi`, and never did on its
  own merits.** The leaf reached the sidecar only via patra 1.12.12's
  `[deps.sakshi]` block — the defect being fixed — so fixing it *unmasked* a
  latent hole. Measured: **27** leaves on patra 1.12.12, **26** on 1.13.0, **26**
  with the block removed entirely. `dist/libro.cyr` calls `sakshi_*` 44 times and
  defines none. Consumers are fine today (bote and agnosai each declare
  `"sakshi"` in their own `[deps].stdlib`), but a clean-room consumer relying
  only on the sidecar would fail where 2.8.4 worked. ⚠ `cyrius distlib`'s
  self-check **cannot** catch this class: undefined *functions* are downgraded to
  warnings, so only an undefined *variable* fails a bundle.
- **A `#` comment INSIDE a manifest array silently truncates the list.** No
  error. Hit while editing libro's `[deps].stdlib`: a comment after `"sakshi",`
  cut the sidecar from 27 leaves to 21 and the only symptom appeared much later,
  as `distlib` failing on an unrelated undefined variable. Keep manifest comments
  outside the brackets.

### Original handoff, kept for the reasoning

A four-repo dependency defect was traced to its root and fixed at the root.

### The defect, once, so it is not re-derived

`cyrius deps` overlays a git dep's resolution **on top of** the
`lib sync --full` snapshot, **recursing through sibling manifests**, on **every
`cyrius build`** — not only on `deps`. So any repo pinning a module that is also
folded into the cyrius stdlib silently **downgrades** that module for every
consumer that reaches it, however many levels away.

`patra` — itself folded into the stdlib — pinned `[deps.sakshi]` at **2.4.2**
while the snapshot shipped **2.4.10**. It reached agnosai four levels down:

```
agnosai -> bote -> [deps.libro] -> [deps.patra] -> [deps.sakshi] 2.4.2
```

⚠ **Verification rule, learned the hard way: check AFTER a `cyrius build`, never
after the three-step.** The three-step ends correct and the next build silently
reverts it. `deps --verify` cannot catch it — the lock is written *from disk*, so
it records the downgraded file's hash. The only signal is an unnamed
"1 bundled lib(s) differ" shadow warning.

⚠ **Trace chains DOWN, not one level.** agnosai's manifest note named the culprit
three times and was wrong every time ("sigil and bote", "majra 2.6.0", "bote"),
because each version read only direct siblings.

### Repo status

| repo | version | pin | state |
|---|---|---|---|
| **patra** | **1.13.0** | 6.5.19 | ✅ **committed + tagged** (`9aed017`, 2026-08-11 20:02). Zero `[deps.*]` blocks; sakshi from the fold. 893 tests, 7/7 fuzz, benches, fmt/lint/vet/deny clean. |
| **majra** | **2.6.5** | **6.5.20** | ✅ **done, awaiting tag** — last tag 2.6.4. **258 assertions** (198 core + 43 backends + 17 patra_queue). **2.6.5** closes both relay divergences agnosai carried as "owed to majra": `capacity` was accepted and discarded (`relay_subscribe` hardcoded `chan_new(256)`, so 4 and 4096 behaved identically) and the message timestamp was `CLOCK_MONOTONIC` where the oracle stamps `DateTime<Utc>` — meaningless once a message leaves the emitting process. New `relay_with_capacity` / `relay_capacity` / `relay_msg_timestamp`; struct 96 -> 104, appended. ⚠ **The first cut FAILED CI**: it used the stdlib's `clock_epoch_ns`, which lives in `chrono` and is not prepended by the `--no-deps` build CI runs — see the `--no-deps` lesson above. Fixed with majra's own `time_epoch_ns`. **2.6.4** fixed a rate limiter that refused nothing: both limiters stored the caller's key pointer by reference, so a key built per request minted a fresh full-burst bucket each time, and eviction leaked key, bucket and its own scratch vecs. 9 mutation probes, 9 kills across both releases. ⚠ `tests/test_live.tcyr` needs Redis and PostgreSQL, absent here. |
| **libro** | **2.8.5** | **6.5.20** | ✅ **done, awaiting tag** — last tag 2.8.4, so this took a **patch bump**. **512 assertions** (524 with `-D LIBRO_TPM`), fmt clean, 23 files lint / 0 warnings, vet 45 deps / 0 untrusted, deny 0 violations, 3 bench binaries green, `.bss` held at **80,224 bytes**. Content: pin 6.5.10 → 6.5.20, `[deps.patra]` 1.12.12 → **1.13.0**, `[deps.sigil]` + `[deps.sigil_tpm]` 3.12.1 → **3.12.7** with the thin selection kept. |
| **bote** | **3.3.1** | **6.5.20** | ✅ **done, awaiting tag** — last tag 3.3.0. **867 assertions across 13 suites** + the core-only drift guard, 14 benchmarks, both `dist/` bundles at v3.3.1. Content: defensive `[deps.sakshi]` **DELETED**, `[deps.libro]` → 2.8.5, `[deps.majra]` → 2.6.3, pin → 6.5.20. ⚠ bote should take **majra 2.6.4** on its next touch — it is a correctness fix in a module bote republishes. |
| **kavach** | **3.11.12** | **6.5.20** | ✅ **done, awaiting tag** — last tag 3.11.10. **3.11.12** closes the WASM **memory** bound: `policy_basic()` leaves `memory_limit_mb` at 0 and the backend emitted `-W max-memory-size` only when it was > 0, so every caller on the default policy ran an **unbounded** guest — a wasm32 module can claim 4 GiB. Verified against wasmtime 47 with a 128 MiB module. Now always emitted, defaulting to the oracle's 64 MiB. **684 assertions.** **3.11.11**: Adds `config_fuel`: the WASM fuel budget was derived from `timeout_ms` (`* 1e6`, so 3e10 at the default 30 s — **30x** the oracle's `set_fuel(1e9)`), and no caller could ask for a CPU bound without also shortening its wall clock. `SandboxConfig` 104 -> 112, `fuel` appended. **676 assertions**, mutation-verified. Filed from agnosai. |
| **agnosai** | **2.0.0** | **6.5.21** | ✅ **done, awaiting commit.** **99 suites, 8,038 assertions, 0 failed**; `check-clean.sh` green, `deps --verify` 113/0, and **every `.tcyr`/`.bcyr`/`.fcyr` compiled explicitly** — `check-clean` does not reach `tests/`, and a hub include broke two suites this session while it stayed green. `[deps.majra]` **2.6.6**, `[deps.kavach]` **3.11.12**, toolchain **6.5.21** with `lib/` re-synced (107 files) and read back AFTER a build. **VERSION cut to 2.0.0 on 2026-08-14** — parity reached, all six v2.0 criteria met. |
| **cyrius** | 6.5.20 | — | ✅ not a blocker, and **never was**: 6.5.19 already folded patra 1.13.0. |

## 🔎 Open findings — P(-1) sweep, 2026-08-12 (cyrius 6.5.20)

Five dimensions reviewed with an adversarial verifier on every finding. **None of
these is fixed** — they are deliberately not bundled with the toolchain bump
(CLAUDE.md: ONE change at a time). Ordered by what to do first. Every number
below was re-measured locally, not taken from the reviewer.

⚠ **The most serious class is not allocation — it is code that is BUILT, TESTED,
and NEVER CALLED.** Three separate features are fully implemented, carry passing
assertions, and have zero production call sites. A green suite plus 100% coverage
did not catch any of them, because *reference* coverage counts whether a symbol is
named by a **test**, which these all are.

0a. ✅ **FIXED 2026-08-12.** **The OTLP exporter posted to the collector ROOT, so
    the whole telemetry tier was silently non-functional in the documented
    configuration.**
    `_agnosai_otlp_post` (`src/telemetry/otlp.cyr:563`) calls
    `sandhi_http_post(str_cstr(endpoint), …)` on the raw configured endpoint.
    **`agnosai_otlp_path` (`:290`) — which exists solely to append the spec's
    `/v1/traces` — has ZERO callers in `src/`**; its only references are five
    assertions in `tests/telemetry_otlp.tcyr:228-240`. A collector configured as
    `http://localhost:4318` (the documented form, and what `:48`'s comment
    describes) receives `POST /` and answers 404. **One line to fix.**

0a-bis. ✅ **FIXED 2026-08-12.** **No `OTEL_EXPORTER_OTLP_HEADERS`, so no hosted
    collector was reachable.**
    `src/telemetry/mod.cyr` reads `OTEL_SERVICE_NAME` (`:84`) and
    `OTEL_EXPORTER_OTLP_ENDPOINT` (`:92`) and nothing else; every hosted collector
    (Honeycomb, Grafana Cloud, Datadog) authenticates via that header.

    ⚠ **Frame this precisely — there is NO oracle counterpart to compare against.**
    `rust-old/src/telemetry/mod.rs:50` delegates the entire transport to
    `hoosh::telemetry::init_otel(endpoint, &service_name)`, so the oracle never
    implements path construction, headers, or retry; it inherits them from an SDK.
    Because ADR 003 consumes hoosh as a **remote HTTP seam rather than linking
    it**, `src/telemetry/otlp.cyr` is hand-rolled and port-original. The bar for
    it is therefore "what an OTLP/HTTP SDK does", not "what a Rust line does" —
    and both 0a and this item are places it falls short of the spec.

0b. ✅ **FIXED 2026-08-12 as a DELIBERATE DIVERGENCE — [ADR 020](../adr/020-output-filter-wired-into-task-output.md).**
    ⚠ **The sweep's fact was right and its implication was wrong: the ORACLE
    never calls it either** (`rust-old/src/server/mod.rs:11` is the only mention
    in the whole Rust tree), so the port was at PARITY, not defective. What
    justified changing it is the asymmetry with `prompt_guard`, whose oracle DOES
    wire it. Scanning is now unconditional; redaction is opt-in via
    `AGNOSAI_OUTPUT_REDACT` because it rewrites legitimate answers.
    **`src/server/output_filter.cyr` was 20 functions, fully tested, never
    called.** The file is `include`d at `src/main.cyr:90` and that is its only
    appearance outside itself; a per-symbol sweep of every public name finds
    **zero** references anywhere in `src/`. This is the *outbound* half of the
    prompt-injection defence — the inbound half (`prompt_guard`) is wired.

0c. ✅ **FIXED 2026-08-12.** **The crew execution timeout was computed and never
    enforced.** ⚠ The port's deadline is COOPERATIVE (checked at loop heads),
    where the oracle's `tokio::time::timeout` aborts work in flight — so a single
    task that hangs forever still holds the worker. That needs a request timeout
    on the LLM transport and is its own bite.
    **The original finding:**
    `agnosai_orchestrator_timeout_secs` (`src/orchestrator/orchestrator.cyr:157`)
    resolves the budget or `AGNOSAI_DEFAULT_CREW_TIMEOUT_SECS = 3600`, and has
    **zero call sites in `src/`** — its only `src/` mention is a *comment* at
    `main.cyr:316`. `_agnosai_orch_finish` calls `agnosai_crew_runner_run` with no
    deadline, and the runner's loops poll only cancellation
    (`crew_runner.cyr:870`, `:1017`). The oracle wraps the run in
    `tokio::time::timeout` (`rust-old/src/orchestrator/orchestrator.rs:197-214`)
    and on expiry logs `"crew execution timed out"` and returns a `Failed`
    `CrewState`. `POST /api/v1/crews` runs the crew inline, so a wedged crew holds
    one of the 100 pool workers **forever**, with no Failed state, no sakshi
    event, and no audit record. ⚠ `main.cyr:316` and `CHANGELOG.md` both claim
    the function "dereferences it, so a 0 faults the moment a crew runs" — false
    as written, since it never runs; correct both with the fix.

**The four allocation findings below share one root**: allocating on the
**no-free global bump** inside a long-running server. That is not a leak Cyrius
can collect; the process grows monotonically.

⚠ **Read the arena figures as VIRTUAL ADDRESS SPACE, not RSS.** Arena chunks are
anonymous overcommitted mmaps and `arena_new` touches only the header
(`lib/alloc.cyr:150-155`, "only touched pages cost RAM"), so a 12 MiB arena costs
12 MiB of VA and only the pages actually written in RSS. RSS still grows
monotonically — the correction is to the magnitude, not to the defect.

1. ✅ **FIXED 2026-08-12 — and `orchestrator/audit.cyr` turned out to be a FALSE POSITIVE.**
   **`str_builder_new_a` is threaded, then every add/build is BARE — the arena
   receives only the 88-byte header.** The builder struct is 24 bytes
   `{buf,len,cap}` and **stores no allocator** (`lib/str.cyr:428`), so bare
   `_sb_grow` and `str_builder_build` hardcode `default_alloc()`. `lib/str.cyr:433`
   says so outright: *"Subsequent grows must use the `_a` add variants."* This is
   a usage defect here, not a stdlib one. Swept the whole tree:

   | file | `new_a` | add bare / `_a` | build bare / `_a` | |
   |---|---|---|---|---|
   | `server/routes/sse.cyr` | 3 | 0 / 10 | 0 / 3 | ✅ clean |
   | `server/routes/mcp.cyr` | 2 | 0 / 5 | 0 / 2 | ✅ clean |
   | `server/routes/approval.cyr` | 1 | 0 / 7 | 0 / 2 | ✅ clean |
   | `telemetry/otlp.cyr` | 2 | 0 / 33 | 0 / 4 | ✅ fixed 2026-08-12 |
   | `orchestrator/audit.cyr` | 2 | 5 / 3 | 1 / 2 | ✅ correct — see below |

   Measured cost in otlp: **~3.9 KB permanently leaked per exported span**, and
   `agnosai_otlp_exporter_start` (`:627`) runs the drain on a **1000 ms loop even
   with no endpoint configured** (`:584-586`). Both 256 KiB arenas at `:378-383`
   are dead reservation, and the `drain` header's contract — "the returned Str
   lives in the ring's `doc_arena`" — is false.

   ⚠ **This is also a false claim in this very file.** The Tests section states
   *"Every module threaded for arenas carries an `alloc_used()`-delta assertion …
   mutation-verified — un-threading a single call fails them."* `tests/telemetry_otlp.tcyr`
   contains **no `alloc_used` assertion at all**, which is exactly why 29 bare
   calls went unnoticed. `tests/orch_audit.tcyr` has 4 and still missed 6.

   ⚠ **`orchestrator/audit.cyr` needed no change — the sweep's grep miscounted it.**
   Checked 2026-08-12, per site:
   * `_agnosai_audit_sign_a` is **already fully threaded** (`add_a`, `add_cstr_a`,
     `build_a`). Its two bare `str_builder_putc` calls are safe and documented at
     the site: 64 hex chars into the builder's 64-byte inline buffer is an exact
     fit, so it never grows. **Measured: 88 B/sign on the global bump, and that
     is entirely the deliberate `str_clone`** (16-byte header + 72 rounded from
     65) which exists because the signature outlives the scratch arena. `putc`
     contributes zero.
   * All 5 bare adds + 1 bare build are in `_agnosai_audit_failure`, which builds
     the text `agnosai_audit_verify` returns on a **corrupted chain**. It is an
     error path, `agnosai_audit_verify` has no `_a` form and no caller in `src/`
     at all, and the returned Str must outlive any request arena. Global is
     correct there.

   ⚠ `str_builder_putc` has **no `_a` form**, so the hex trace/span id path needs
   either its own short-lived `str_builder_new_a` + `str_builder_add_a`, or an
   upstream `putc_a` filing.

2. ✅ **FIXED — all five sites pooled via `src/arena_pool.cyr`.**
   `arena_allocator(n)` charges the no-free global bump `n + 112` bytes per call
   and `reset_via` returns none of it.

   | site | was | now |
   |---|---|---|
   | `tools/agnos.cyr` — every outbound HTTP request | **5,242,992 B** | ✅ **0**, 8 slots |
   | `tools/remote_registry.cyr` — per package fetch | 12 MiB + 112 B | ✅ pooled, 2 slots |
   | `tools/builtin/security_audit.cyr` — per audit | 2.5 MiB + 112 B | ✅ pooled, 4 slots |
   | `server/serve.cyr` — per a2a callback | 64 KiB + 112 B | ✅ pooled, 4 slots |
   | `tools/builtin/load_testing.cyr` — **twice PER USER** | varies + 1 MiB | ✅ two pools, 16 slots each |

   Measured on the agnos path: **0 bytes per exchange** against 5,242,992 before,
   over 32 cycles, 8 arenas minted, 0 fallbacks.

   ⚠ **A note here previously claimed the two thread-spawning sites "cannot reach
   a request-path pool". That was WRONG** — the pool is a process-global whose
   channel is mutex-guarded, so a detached callback thread or a load-test worker
   uses it exactly like a sandhi worker. Nothing about it is request-scoped.

   ⚠ `load_testing` holds **two** arenas per worker and gets two pools, because
   the persistent one's size varies with the user count
   (`max_requests = 100000 / users`: ~4 MiB at one user, ~73 KiB at 500). Pooled
   arenas are **growable**, so no single number is needed — they converge. Two
   resources per holder cannot deadlock because **acquire never blocks**; an empty
   pool mints a fallback.

   The design and the three rejected alternatives (thread-local, sandhi's request
   arena, one shared arena under a mutex) are in `src/arena_pool.cyr`'s header.
   The escape analysis that made a pool the right shape — **nothing escapes any
   of the five sites** — is what rules the lifetime-oriented fixes out.

3. ✅ **FIXED 2026-08-12 — in the MATCHER, not just the assertion.** The capture
   is deferred to a confirmed full match: **64 B → 48 B per parameterised
   resolve**, one `Str` header saved. The assertion now reads `deep hit + exactly
   one Str`, both measured in the same run. `str_sub_a` **borrows**, so a capture
   is 16 B regardless of parameter length — verified with 1-char vs 46-char.
   **The original finding:** `server_router.tcyr:453` asserted a property the
   matcher already violated, under a bound 2.4x too loose to notice. It claims *"a parameterised hit
   captures one Str, not one per pattern tried"*, but
   `_agnosai_path_matches_at` (`src/server/router.cyr:180-183`) runs
   `str_sub_a` on reaching a `*` segment and only then compares the remaining
   segments — so a trial that subsequently fails has already captured. The
   threshold stays green up to seven captures. Fix the assertion to the true
   property, or fix the matcher to capture only on a confirmed match.

4. ✅ **FIXED 2026-08-12 — 560 B → 0 B per clean scan**, 16 B only on a
   detection. Needles go through `agnosai_str_contains_ci_cstr`; messages are
   built only on the branch that returns them. Mutation-verified both ways.
   **The original finding:** `agnosai_prompt_scan_input` allocated 560 B of `Str`
   headers per CLEAN scan. `src/server/prompt_guard.cyr:112-174` holds 36 textual `str_from`
   calls, 35 on the clean path. `agnosai_str_contains_ci_cstr`
   (`src/strcase.cyr:96`) already exists for exactly this and is already used in
   this literal shape at `sandbox/wasm.cyr:251-266` (5 sites) and
   `fleet/cost_planning.cyr:169` — prompt_guard is the outlier. The oracle
   allocates **nothing**: `rust-old/src/server/prompt_guard.rs:30` is a
   `const INJECTION_PATTERNS: &[(&str,&str)]` static table.

5. **SSE on an unknown crew id orphans a 256-slot channel + sender per request.**
   `src/server/routes/sse.cyr:149-151` does has → subscribe → remove, which is
   **parity** with `rust-old/src/server/routes/sse.rs:24-32` in *order* — but not
   in reclamation: Rust's `remove` drops the `broadcast::Sender` and the receiver
   drops at scope end, while `agnosai_event_bus_remove` (`src/server/sse.cyr:398-410`)
   only `map_delete`s, leaving the sender (`:195`), the subscriber `chan_new(256)`
   (`:144`) and the cloned key (`:385`) on the global bump. Unauthenticated
   requests to a nonexistent crew id reach this.

### ⚠ sandhi 1.9.10 fixes a null-body P1 — agnosai does NOT have it yet

`_sandhi_resp_frame_a` reported **`SANDHI_OK` with a null `body_ptr`** and a
positive `body_len`. Reproduced against the folded sandhi in this tree
(8,000-byte body, 4 KiB arena): `err_kind == SANDHI_OK` -> **1**,
`body_len` -> **8000**, `body_ptr` -> **0**.

It needs no memory pressure: sandhi allocates its receive buffer **eagerly at
the full `max_response_bytes` cap** (`lib/sandhi.cyr:4644`) before a byte
arrives, so the body copy gets only the arena's remainder — **~1 MiB** for
`tools/agnos.cyr` (4 MiB cap, 5 MiB arena) and **~2 MiB** for
`tools/remote_registry.cyr` (10 MiB cap, 12 MiB arena). Above that,
`bayan_json_v_parse_buf(0, len)` and `str_from_buf(0, len)` read from address 0.

- ✅ **Both agnosai sites now guard**, fail closed, and keep the guard
  permanently — this tree must not depend on a transport's internal invariant to
  avoid a segfault.
- ✅ **Fixed at the root in sandhi 1.9.10 — RELEASED and tagged 2026-08-12**,
  confirmed published. 1,250 assertions across its four suites, fuzz 8/8, all
  five `dist/` bundles regenerated, pin 6.5.5 -> 6.5.20.
- 🟡 **`lib/sandhi.cyr` here is still `SANDHI_VERSION = "1.9.9"`, and that is
  expected, not drift.** sandhi is folded into the cyrius stdlib rather than
  taken as a git dep, so the fix lands **in the next cyrius release** (6.5.20
  predates it). Nothing to do in this repo when it does — the three-step picks it
  up — but re-check `SANDHI_VERSION` in `lib/sandhi.cyr` after that pin bump.

  Until then the local guards are the entire protection. ⚠ `tests/tools_agnos.tcyr`
  asserts against the **buggy** folded version deliberately, and is written to
  stay green either way: the `err_kind == SANDHI_OK` branch is conditional, so
  the re-fold flips it to the error path without a failure.
- ⚠ **Not reachable from the shipped binary today**, and the sweep that found it
  claimed it was, via `_agnosai_mcp_tools_call`.
  `agnosai_register_{mneme,synapse,delta}_builtins` have **zero callers in
  `src/`**; `main.cyr:351-353` registers only basic, load_testing and
  security_audit. Latent — live the moment an agnos-backed tool is registered.

### ✅ The a2a callback thread leaked its whole stack mapping — fixed 2026-08-12

`agnosai_serve_dispatch_callback` called `thread_create` and **discarded the
handle**. `thread_join` is the ONLY caller of `munmap_stack`, so nothing released
the mapping: `mmap_stack` maps `THREAD_STACK_SIZE + THREAD_GUARD_SIZE` =
**2,101,248 bytes** and mprotects the guard page, which splits it into **2 VMAs**
— per callback, permanently, plus a 4 KiB TLS block and a 24-byte handle on the
no-free bump that `thread_join` would not have reclaimed either.

Measured here before the fix: **20 unjoined threads → +40 VMAs**, still present
after they had exited. **20 detached → +0.** Now `thread_create_detached`, which
has the child unmap its own stack and carves the TLS from the top of that same
mapping so one `munmap` frees both. It also reports failure rather than returning
1 for a dispatch that never started.

⚠ **The identical fix already shipped in `orchestrator/submit_crew`** — see its
header and the upstream filing behind cyrius 6.5.8's `thread_create_detached`.
This site was missed. The rest are clean: `crew_runner:1066` and
`load_testing:471` keep handles and join, `inference_queue:437` is already
detached, `agnosai_serve_install_signals` is one thread for process lifetime.

⚠ **The call site is NOT offline-reachable, so nothing asserts on it.**
`agnosai_route_a2a_callback_allowed` refuses loopback and private ranges by
design, and a public host means DNS plus a 30 s connect timeout that leaves the
stack mapped while any assertion looks. `tests/server_serve.tcyr` pins the
primitive instead (`_t_detached_threads_free_their_stacks`). ⚠ A first attempt
used a loopback URL and was **vacuous** — the gate refused it before any thread
spawned, so reverting the fix passed the entire suite. The replacement is
mutation-verified (breaking it reports `got 15, expected 0`).

The 64 KiB arena at `serve.cyr:259` is a separate matter and still open — it is
one of the five sites in the arena-pool bite above.

### ⚠ A spin count is not a duration — never wait on iterations

2026-08-13. A test waiting on a detached callback thread used
`while (flag == 0 && spins < 20000) { spins = spins + 1; }`. It passed every
time the suite ran alone and **failed in the full 99-suite sweep**, where the
thread had not run yet and the assertions read a body nobody had written. A busy
spin also starves the very thread it waits for on a loaded box.

Wait on the **clock** (`sleep_ms` in a loop with a generous wall-clock bound) and
assert you did not hit the bound, so a real hang still fails rather than passing
silently. ⚠ Note the honest limit of the verification: 8 synthetic spinners did
**not** reproduce the flake — a full sweep interleaves compiling and running 99
binaries, which is a different environment. The fix is right because iterations
are not time, not because a repro was produced.

### ⚠ A FIX is a change, and earns the same review as the code it replaces

A second adversarial review of the first review's 22 fixes found **five more
defects, three of them introduced BY those fixes** — two critical. 2026-08-13:

- **Porting half a guard is worse than porting none.** The oracle floors the
  load-test user count at the **divide** (`concurrent_users.max(1)`), not at the
  clamp. Removing the port's clamp floor "for parity" without moving the guard
  left `100000 / 0` reachable unauthenticated — **SIGFPE kills the process**,
  it is not per-thread recoverable. When removing a guard to match an oracle,
  find where the oracle put ITS guard.
- **Making a parameter work can weaponise a divergence.** majra's relay capacity
  was accepted and discarded; honouring it turned a latent blocking send into a
  live permanent deadlock at whatever depth the caller names. ⚠ And the test
  written for that release missed it by filling the ring with `chan_try_send`
  **directly on the channel**, routing around the very call that blocks —
  **exercise the API under test, not the primitive beneath it.**
- **I violated a rule I had written myself, in the module I wrote it in.**
  `arena_pool.cyr`'s header forbids lazy `if (pool == 0)` construction; the crew
  runner did exactly that. Writing a rule down does not make you follow it —
  grep for the anti-pattern.
- **A fallback allocator must never be handed to a release path.** Substituting
  `default_alloc()` for a pool arena and then "releasing" it calls
  `alloc_reset()` on the **whole process heap**. Acquire failure means "no
  arena", never "use a different one".
- **A process global on a pooled-worker path is a race, however wide the
  window.** The window here was two HTTP chains wide — seconds — and the payload
  was one tenant's transport outcome appearing in another's response.

### ⚠ A test that reads the ENVIRONMENT will pass here and fail in CI

2026-08-13/14, three instances of one pattern — **a check passes locally because
this box supplies something a runner does not**:

1. **`--no-deps`** — majra's CI builds with it; I built without, so a stdlib
   module that a `--no-deps` build does not prepend resolved here and failed
   there.
2. **`lib/` drift** — the pinned snapshot had moved to sandhi 1.9.10 while this
   tree still held 1.9.9; `check-clean` only re-compares when a dep pin changes.
3. **DNS** — a test drove `agnosai_run_security_audit_err` at an RFC 2606
   `.invalid` host and asserted the sandhi error kind was captured. With a
   resolver present sandhi returns a result carrying `SANDHI_ERR_DISCOVERY`; with
   **no resolver at all it can return a NULL result**, so there is no kind to
   report **by construction** and `agnosai_guarded_fetch_full` correctly writes
   nothing. The code was right; the assertion was wrong.

⚠ **The fix is to assert what is INVARIANT across environments**, not to widen
the assertion until it cannot fail. Here that is: the audit fails, and it never
reports a reason it did not observe — with the resolver-present arm still
pinning `DISCOVERY` and a specific message, so the mutation is caught wherever a
resolver exists.

⚠ And an availability guard must assert BOTH arms. `tests/sandbox_wasm.tcyr`
does this correctly: with no `wasmtime`, execute must return a Sandbox error
rather than half-working — a skip would have proven nothing.

### ⚠ A green local build is not evidence about CI, and `lib/` goes stale silently

Two 2026-08-13 findings that cost real time, both the same shape: **a check that
could not have detected the problem was read as if it had passed.**

**`cyrius build` without `--no-deps` is not what CI runs.** majra's CI does
`lib sync --full` -> `deps` -> `build --no-deps`, and that last flag is
load-bearing: without it the build auto-resolves and pulls modules a `--no-deps`
build does not prepend. `relay_msg_new` reaching for the stdlib's
`clock_epoch_ns` (in `chrono`) compiled cleanly here and failed CI with
`undefined function`. ⚠ **Adding `chrono` to `[deps] stdlib` does NOT fix it** —
that list controls provisioning, not what a `--no-deps` build has in scope; the
first fix looked plausible, was wrong, and only reproducing CI's exact
invocation showed it. The real fix was a local `time_epoch_ns` beside the
`time_now_ns` majra already had. ⚠ agnosai is unexposed — it declares `chrono`
AND its CI omits `--no-deps` — but that was **verified, not assumed**.

**`lib/` drifts from the pinned snapshot without anything failing.** agnosai
built against **sandhi 1.9.9 for a whole session** while the 6.5.20 snapshot
carried 1.9.10, because `lib sync --full` was never re-run after the fold.
`check-clean`'s lib-snapshot gate only re-compared once a dep pin changed.
**Bumping a dep pin and running `cyrius deps` does not refresh the snapshot** —
run the three-step, and read `lib/` AFTER a build.

### ⚠ A security control that is *mounted* is not a security control that *works*

Learned 2026-08-13, mounting `rate_limit` by default per [ADR 021](../adr/021-rate-limit-mounted-by-default.md).
An adversarial review of the change raised 45 findings; **21 survived double
refutation**, and every one was in code or docs written in that same sitting.
The pattern worth keeping is that a green suite plus 100% coverage said nothing
about any of them, because each was a property no assertion asked about:

- **The limiter refused nothing.** A `Str` was passed where a cstr-keyed map was
  expected, so `hash_str` walked the Str header — whose first eight bytes are the
  data pointer — and every request minted its own full-burst bucket. **A test
  using string literals cannot catch this**: a literal is one pooled address
  reused at every call site, so a pointer-keyed limiter passes happily. Every
  key in a limiter test must be built at a fresh address.
- **Nothing ever called the eviction sweep.** It had one caller in the tree and
  it was a test. `grep -rn <fn> src/` is a five-second check that the mechanism
  a doc credits actually runs, and it was never done — the ADR asserted memory
  was "bounded in time" and shipped that claim as Accepted.
- **Idle eviction cannot bound attacker-chosen keys.** A fresh key per request
  keeps every bucket inside the idle window, so the sweep finds nothing while
  the map grows. A cap needs a rung that fires regardless of age.
- **Unbounded input became permanent allocation.** The key had no length cap, so
  one 10 MiB header could pin a megabyte for the process lifetime; over 4 KiB the
  freelist mmaps a VMA per key that is never unmapped.
- **The default config path was untested.** The one assertion mentioning the
  default compared a variable to the value the test had written to it one line
  above. `src/main.cyr` cannot be included by a `.tcyr`, which is *why* the
  config lived somewhere untestable — moving it into the module it configures
  fixed both.
- **Testing a helper does not test the wiring.** Asserting the strict parser
  directly left a mutation that reverted its *callers* to the old u16 port parser
  passing the entire suite. Split "read the environment" from "interpret the
  value" so a test can drive the decision the caller actually makes.

⚠ **Reusing a parser for a domain it was not written for is a silent clamp.**
The rate went through `agnosai_serve_parse_port`, whose -1 sentinel fires above
65535 — so `AGNOSAI_RATE_LIMIT=200000` silently became 100 req/s. And the obvious
replacement is worse: `str_to_int` skips non-digits and answers **0** on garbage,
and here `0` means *disable the control*.

### ⚠ `check-clean.sh` does NOT compile the suites — do not read it as "the tree builds"

Learned the hard way 2026-08-12, wiring `src/arena_pool.cyr` into
`src/tools/mod.cyr`'s dependents. `check-clean.sh` runs fmt, lint, doc, doctest,
vet, deny, `deps --verify`, the lib-snapshot diff and the generated-source check.
**Nothing in it touches `tests/` or `benches/`.** It was green while **17 suites
and one `.bcyr` failed to compile.**

The trap is specific to how Cyrius builds: `include` is textual, there are **no
include guards**, and every compilation unit lists its own dependencies. So
adding a module that an existing hub pulls in — here `src/tools/mod.cyr:57`
includes `tools/agnos.cyr` — silently breaks every `.tcyr`/`.bcyr` that includes
that hub and does not also include the new module. It is a hard compile error,
never a warning, and it surfaces in exactly two places: `cyrius tests tests` and
`cyrius bench`.

⚠ **"N suites pass individually" says nothing about the ones not run.** The three
suites checked directly were the three whose includes had been updated; the other
18 units were untouched and unbuilt.

**Rule: after touching any module that a `mod.cyr` hub includes, run the full
`cyrius tests tests` AND `cyrius bench` before reporting.** This is the same
class as the `.bcyr` rot recorded under Benchmarks — *the gate was never silent,
it was never invoked*.

### Still outstanding, not blocking

- **14 repos still declare `[deps.sakshi]`** — re-enumerated 2026-08-12 with
  `grep -l '^\[deps\.sakshi\]' ../*/cyrius.cyml`, which is the count to trust
  over any figure carried forward in prose:

  | tag | repos |
  |---|---|
  | 2.3.0 | shakti |
  | 2.4.2 | argonaut |
  | 2.4.3 | ghurni, prani, garjan |
  | 2.4.4 | owl, vidya |
  | 2.4.6 | anuenue, yukti, daimon, stiva, phylax |
  | 2.4.7 | sit |
  | **2.4.10** | **hisab** — current, so 13 are stale, not 14 |

  **agnosai, bote, libro, majra, patra and sigil now declare none at all**, which
  is the stronger end state than "pinned current": there is no block to go stale.
  Each of the 13 is the same one-line fix patra took; none is in agnosai's
  dependency path, so none blocks the port.
- **`sha256` keeps a `cbank()`-banked message schedule** (`&W + cbank() * 512`)
  in sigil. Fail-closed for a verify (a corrupted digest mismatches), but the
  same structural pattern as the RSA lane bug. Not probed.
- ⚠ **A `~/.cyrius/versions/<v>/lib` snapshot can be overwritten in place with a
  later version's content** — observed with 6.5.18 on 2026-08-11, which made
  `check-clean` fail against it while `lib/` was untouched. A snapshot mismatch
  is not automatically drift in this tree; check the toolchain side first, and do
  not re-sync just to make the gate pass.

  ⚠ **But do not reach for this explanation first — it was wrong on 2026-08-12.**
  `6.5.19/lib/patra.cyr` reading **1.13.0** looked exactly like an in-place
  overwrite (6.5.18 and earlier read 1.12.12, and `6.5.19/lib` is byte-identical
  to `6.5.20/lib`). It was not. patra 1.13.0 was tagged **2026-08-11 20:02:27**
  and 6.5.19's binary was built **20:09:37**, seven minutes later, so 6.5.19
  folded it legitimately. **Check the dependency's tag date against the
  toolchain binary's build date before concluding corruption** — and note that
  `~/.cyrius/versions/<v>/bin/cyrius --version` self-reports correctly, so the
  binary is a reliable witness even when the `lib/` tree looks suspicious.

## Version

**2.1.5** (`VERSION`), cut 2026-10-04; 2.1.0 was cut 2026-09-26. **2.0.0** was cut 2026-08-14, the first Cyrius release. 1.1.0 was
the last shipped Rust release and is preserved at `rust-old/` as the parity
oracle. The rule held: VERSION bumped once parity landed, so the number always
names something that actually shipped.

⚠ **"At least one downstream consumer green" is post-tag work, not a gate.** A
consumer cannot integrate against a port that does not exist — the 2.0.0 tag is
what it pins. This file's own M8 note states the principle: *"zero consumers is
a consequence of it not being ported, not a reason to leave it."* daimon, listed
in `CLAUDE.md` as a consumer, has no coupling to agnosai at all and could never
have served as the check.

## Toolchain

- **Cyrius pin: `6.6.14`** (since 2.1.3, 2026-10-03; unchanged at 2.1.5) — at the 2.1.5
  release gate `cyrius lib sync --full` wrote 112 files, `check-clean.sh`'s lib snapshot matched
  112, `deps --verify` verified 118, and `cyrius.lock` was byte-identical to 2.1.4's. See Now.
  cyrius 6.6.15 is tagged upstream; the move is the roadmap's *Moving the cyrius pin to 6.6.15*.
- Previously **`6.6.12`** (2.1.2, 2026-10-01), and before that **`6.6.6`** (2.0.10,
  2026-09-26): at 6.6.6 `lib/` matched the snapshot 111/111, the installed snapshot matched the
  cyrius `6.6.6` tag 111/111, `deps --verify` 117/0, and a fresh CI-style resolve (empty `lib/`,
  no siblings) reproduced the local lock's 117 hashes.
- Previously **Cyrius pin**: `6.5.20` (`cyrius.cyml`) — bumped 2026-08-12, three-step,
  `lib/` diffed after the sync AND again after a build: 107 files, zero content
  differences, `deps --verify` **113 verified / 0 failed**, duplicate-fn warning
  count unchanged at **35**. Folds **patra 1.13.0**, **sigil 3.12.7**, **sakshi
  2.4.10**, **bayan 1.4.1**, **sandhi 1.9.9** and **sankoch 2.7.7**. Full gate on
  the bumped tree: **97 suites**, coverage **1561/1561 (100%)**, `check-clean.sh`
  green, bench sweep **199 rows, 0 regressions**.

  **Sibling pins as of the 2026-08-12 bump**: bote **3.3.1**, majra **2.6.3**,
  kavach 3.11.10, sigil 3.12.7, ai-hwaccel 2.3.16, tyche 1.0.0. No
  `[deps.sakshi]` anywhere in the chain — see the RESOLVED banner at the top.

  ⚠ **6.5.20's stdlib snapshot is byte-identical to 6.5.19's.** `diff -rq`
  across both `lib/` trees reports **0 differing files**, so this is a **pure
  compiler release** and the bump buys code generation, not modules. It also
  means the thing the handoff was waiting for — a re-fold of patra 1.13.0 — had
  **already happened in 6.5.19** (6.5.18 → 6.5.19 differs by exactly one file,
  `patra.cyr`).

  **What 6.5.20 fixes:**

  - **P1 — a `switch` / `match` case body could only be left safely by
    `return`.** Falling through after a store to a local gave a **wrong answer
    with no diagnostic**, or a segfault, on every target. Cause: the v5.6.27
    regalloc NOP-harvest compactor repairs jump disp32s and fixup CPs but never
    knew the switch jump table existed, so entries shifted **+4 per preceding
    case body** (`16, 30, 47, 64` where correct is `16, 26, 39, 52`). `case 0`
    alone worked; `{ return N; }` bodies never saw it — no local store, no NOP.

    ⚠ **agnosai's own source is not exposed** — zero statement-position
    `switch`/`match` across `src/`, `tests/`, `benches/`. Three *vendored* sites
    carry the trigger shape: `lib/sakshi.cyr:1101` (`_sk_level_str`, ten arms,
    every one a `store64` pair with fallthrough — it formats **every sakshi log
    line**), `lib/kavach.cyr:795` and `lib/sigil.cyr:221` (both `syserr_print`,
    arms declaring locals, no `_ =>` default).

    ⚠ **Whether any of the three actually miscompiled under 6.5.19 is
    undetermined, and a green suite does not settle it.** A minimal repro
    compiles correctly on 6.5.18, .19 and .20 alike — the defect needs enough
    register pressure for the picker to leave harvestable NOPs, which a small
    function does not create. The failure mode is a silent wrong answer, so the
    97-suite baseline being green on 6.5.19 is evidence, not proof.
  - **`#derive` no longer inflates line numbering**, freeing one file-map entry
    per derive against the 1024 cap. agnosai authors **zero** `#derive`
    directives, but **93** expand upstream of it: `lib/kavach.cyr` 35,
    `lib/libro.cyr` 27, `lib/ai-hwaccel.cyr` 16, `lib/sigil.cyr` 15.

  **Previously 6.5.19**, whose two consumed filings are recorded below because
  the corrections still matter:

  **6.5.19 consumed both defects then open against it from this tree**, and in
  both cases went past what was filed:

  - **`fl_alloc` is thread-safe** (`lib/freelist.cyr`). Filed as **one** race
    (two threads popping the same block off `_fl_heads[cls]`); upstream found
    **five** and locked all of them. ⚠ This makes
    `src/llm/inference_queue.cyr`'s enqueue mutex **redundant** — it is left in
    place pending a deliberate removal, and the module header and call site both
    now say so.
  - **The benchmark timer floor is measured, not declared** (`lib/bench.cyr`,
    `bench_clock_overhead_ns()`). ⚠ Two corrections to the filing: a corrected
    *constant* would still have been wrong (230× spread across the four gate
    hosts), and the cause it named — raw `syscall(228)` versus the vDSO — is
    wrong on this box, whose clocksource is **hpet**, so the vDSO falls back to
    the syscall anyway.

  ✅ `bench-history.csv` is unaffected: 6.5.19 also re-sized `bench_run`, which
  floored 57 of 79 rows in cyrius's own history, but **all ten agnosai `.bcyr`
  batch explicitly and none calls `bench_run`**.

  ⚠ **A `~/.cyrius/versions/<v>/lib` snapshot can be overwritten in place with a
  later version's content.** Observed here: `check-clean` failed with
  `lib/bench.cyr differs from the 6.5.18 snapshot` while `lib/` was untouched,
  because the 6.5.18 snapshot had been rewritten with pre-release 6.5.19 files.
  A 6.5.18-vs-6.5.19 diff consequently reads empty. **A snapshot mismatch is not
  automatically drift in this tree** — check the toolchain side before syncing,
  and do not re-sync to make the gate pass.

  ✅ **The defensive `[deps.sakshi]` is REMOVED (2026-08-12), and the root cause
  was never where this file or the manifest said it was.** Both blamed a direct
  sibling — first "sigil and bote", then "majra 2.6.0", then "bote". The actual
  chain is four links deep and ends in a **folded stdlib module**:

  ```
  agnosai -> bote -> [deps.libro] 2.8.4 -> [deps.patra] 1.12.12
          -> [deps.sakshi] 2.4.2
  ```

  ⚠ **patra is itself folded into the cyrius stdlib** (`lib/patra.cyr` v1.12.12
  in the 6.5.19 snapshot) while pinning a sakshi **eight patch releases behind**
  the one that snapshot ships. `cyrius deps` overlays it — recursing through
  sibling manifests — on every `cyrius build`, not just on `deps`.

  bote 3.3.1 pins sakshi forward at 2.4.10 to absorb it, which is what makes the
  pin removable here. Verified after a **build**, not after the sync: sakshi
  stays 2.4.10, `lib/` clean.

  ⚠ **Re-derive by following the chain DOWN, not one level.** Three successive
  versions of that note were wrong because each read only direct siblings.
  `grep -l '^\[deps\.sakshi\]' ../*/cyrius.cyml` is the start, not the answer —
  **16 repos pin it, spanning 2.3.0 to 2.4.10**; only agnosai, hisab and bote are
  current.

  ⚠ **The real fix is upstream and is NOT done**: patra must drop or bump its
  `[deps.sakshi]`, and cyrius must re-fold patra. patra's own toolchain pin is
  **6.4.65**. Until then every libro consumer needs bote's defensive pin.

  ⚠ **Every build emits 35 `duplicate fn … (last definition wins)` warnings.**
  Audited 2026-08-10: **none is currently miscompiling**, two are latent hazards,
  and the audit's own first two conclusions were wrong. The corrections are in
  CHANGELOG and matter more than the finding:

  - **`err_io` (kavach ∩ sigil) does NOT diverge.** sigil has two unrelated error
    enums and comparing them with prefixes stripped merges them. The syscall-error
    tables are identical on all eight shared kinds. Probed: `err_io(5,…)` → kind
    8, errno 5.
  - **`_sub_new` (majra ∩ libro) works here.** Probed: `sub + 8 == 0`, so majra's
    body runs, and `pubsub_publish` delivers. Correct **by accident of include
    order** — both directions corrupt if it flips. agnosai is doubly safe:
    `src/orchestrator/pubsub.cyr` calls no majra `pubsub_*`.
  - Genuinely latent: **`attestation_result_new`** (kavach ∩ sigil), two unrelated
    functions sharing a name, inert only because neither library calls it.

  ⚠ **The warning's `file:line` is NOT usable to decide which copy wins** —
  `lib/kavach.cyr:11512` names a file of 11,321 lines, so the offsets are into the
  preprocessed stream. Both wrong conclusions above came from trusting it. Filed
  to cyrius. **Run the symbol; do not read the warning.**

  ⚠ **A caller parsed BEFORE a redefinition still binds to the later body.**
  Measured (`early_caller()` returns 22, not 11). This is why a duplicate is never
  "harmless because our copy comes first" — but it is also not enough on its own to
  say which copy that is.

  Previously 6.5.14 (sigil 3.12.6).

  **6.5.14 is not optional here.** It fixes the tail-call frame-release bug in
  standing rule 12, and sigil 3.12.6 — which fixes an **RSA-PSS authentication
  bypass**, the same class agnosai reported against PKCS#1 v1.5 — declares
  `cyrius >= 6.5.14` as its floor. agnosai reaches PSS through TLS 1.3
  CertificateVerify (`lib/tls_native_hs13.cyr:257,260`) on every outbound HTTPS
  call from `tools/agnos.cyr`, `tools/remote_registry.cyr` and
  `guarded_fetch.cyr`, so holding at an older sigil would ship a TLS peer-auth
  bypass.

  ⚠ **`sigil.cyr` is BOTH folded into the snapshot AND a git dep with
  `path = "../sigil"`, and when the sibling is ahead of the fold those two
  demands conflict.** The three-step leaves `lib/` matching the snapshot, then
  **every `cyrius build` re-layers the sibling's dist** — build performs an
  implicit resolve and the local path beats the tag. Seen live on 2026-08-08
  with the fold at 3.12.5 and the sibling at 3.12.6. A `check-clean` allowance
  was written for it and then **deleted unused**, because the next toolchain
  snapshot carried 3.12.6 and the conflict evaporated. If it recurs, the choice
  is a narrow newer-only allowance with a stated exit condition, or dropping
  `path` so local resolution matches CI — not silently living with a red gate.

  ⚠ **Do not trust a single `lib sync --full` when the toolchain was installed
  moments earlier.** The 6.5.13 bump's first sync reported `copied 107 .cyr
  files` and left `lib/syscalls_x86_64_agnos.cyr` at the previous pin —
  `check-clean` caught it, a second sync fixed it. **Always diff `lib/` after
  syncing, and again after any `deps` run.**

  Previously 6.5.13 (sigil 3.12.5), 6.5.12 (3.12.4), 6.5.11.

- **`lib/` matches the pin exactly**: 0 of 106 stdlib files differ from
  `~/.cyrius/versions/6.5.10/lib`; build and test emit no drift or shadow warning.
  The six files outside the snapshot (ai-hwaccel, bote-core, kavach, libro, majra,
  tyche) are declared git deps, not staleness.

  **`lib/unicode/` is vendored BY HAND and nothing upstream maintains it.**
  `cyrius lib sync` copies only the top level, so declaring `"unicode"` in
  `[deps].stdlib` — which agnosai now does — does **not** bring the seven files
  under `lib/unicode/`. They were copied from the snapshot manually.

  It stopped being inert on 2026-08-03: `src/sandbox/oci.cyr` calls
  `unicode_category` to match the oracle's Unicode `is_alphanumeric` exactly, so
  those files are load-bearing. `scripts/check-clean.sh`'s snapshot check is
  therefore **recursive**, and is the only thing that will notice if they drift
  from the pin — mutation-verified against a one-byte edit.

  **Verify a bump by diffing the trees, not by a green `lib sync --full`.**
  `cyrius lib sync` skips on file **size**, not content, so a size-neutral change
  — in practice a version stamp — is never copied while the command reports
  success. The 6.5.6 bump left `lib/vani.cyr` a patch behind that way. **Impact
  here was nil** (vani is neither declared nor called), and that is the usual
  shape: real code edits change length and do sync. Filed as a papercut.
  `scripts/check-clean.sh` diffs `lib/` against the pinned snapshot, which
  `deps --verify` cannot do — the lock is written *from* disk, so a stale file
  just gets its stale hash recorded.

### Pins must name what is actually built — corrected 2026-08-03

`cyrius.cyml` pinned **bote 3.2.1** and **kavach 3.9.3** while `lib/` held **bote
3.3.0** and **kavach 3.11.0**. Both vendored bundles are byte-identical to their
upstream tag dists (verified by sha256 against `git show <tag>:dist/...`). The pins
are now **bote 3.3.0** / **kavach 3.11.0**, which is what was already being compiled
and tested.

**The mechanism, because it will recur.** Every `[deps.NAME]` carries
`path = "../NAME"` alongside `git` + `tag`, and **the local path wins**. A developer
whose sibling checkout has moved ahead silently builds a version the manifest does
not name; CI, which has no sibling checkouts, resolves the *tag* and builds
something else. Here that was kavach **3.11.0 locally against 3.9.3 in CI** — the
`~/.cyrius/deps/kavach/3.11.0/` clone predates the session that found it.

This is the inverse of the sigil rule recorded below, and both are live: a stale tag
can *overwrite* a newer folded copy (sigil's case) **or** be quietly overridden by a
newer local path (kavach's case). The lockfile does not save you from the second —
`cyrius.lock` recorded 3.9.3's sha256 against a 3.11.0 file on disk and nothing
surfaced it, because every other gate reads `src/` and the build compiles whatever
bytes `lib/` holds.

**Two gates now close the class**, both mutation-verified:

- `scripts/check-clean.sh` runs **`cyrius deps --verify`** (lock vs. working `lib/`).
  Restoring the stale hash makes it print `FAIL: lib/kavach.cyr (hash mismatch)` and
  exit **1**. Note the exit code must be read directly — piping through `tail` reports
  `tail`'s status and reads as a pass.
- CI gains a **Lockfile is honest** step: `git diff --exit-code -- cyrius.lock` after
  `cyrius deps`, which catches a committed lock that disagrees with a clean tag-only
  resolution — i.e. exactly the local/CI divergence above.

**What the two bumps actually change: nothing on any path agnosai executes.**
kavach 3.10.0/3.11.0 are `--agnos` target build fixes — nine additive `kv_*` shims
(`kv_unlink`, `kv_rmdir`, `kv_waitpid`, `kv_getgid`, `kv_lstat`, `kv_fork`, `kv_dup2`,
`kv_execve`, `kv_setsid`); agnosai calls only `score_agent`, `score_agent_with_tools`,
`sandbox_display`, `sandbox_strength`, and the diff touches none of them. bote 3.3.0
adds `dispatcher_set_server_info` and is additive by construction — an unconfigured
dispatcher emits the pre-3.3.0 wire byte for byte. The duplicate-fn warning count is
unchanged at **35**, and all 57 suites stayed green across the bump.

### What 6.5.6 lands on agnosai

- **sandhi 1.9.9 — the serve-loop stop facility agnosai filed.**
  `sandhi_server_options_stop_flag(opts, ptr)` on all five loops, returning 0 for
  a requested stop against 1 for a failure. This is what made graceful shutdown
  possible; consumed immediately in `src/server/serve.cyr` +`src/main.cyr`, and
  recorded as [ADR 013](../adr/013-graceful-shutdown-via-signalfd-and-stop-flag.md).
- **`sys_exit_group`** (`lib/syscalls_linux_common.cyr:155`) — also agnosai's
  filing. `_agnosai_exit_process` now composes it instead of a hand-rolled
  `syscall(SYS_EXIT_GROUP, …)`; the `#ifdef CYRIUS_TARGET_LINUX` guard stays,
  because that file is included only by the two Linux target files.
- **`async_await_readable_ms`** — agnosai's filing, found while building sandhi
  1.9.9. Not used here; it removes the reason sandhi's cooperative loop polls.

### What 6.5.5 landed

- **bayan 1.4.0 completes the `_a` JSON surface** — `obj_set_a`, `build_a`,
  `build_pretty_a`, `parse_a`, `parse_buf_a`, `parse_ctx_a`, joining the eight value
  constructors that already had `_a` forms. This is the one that mattered: before it,
  a per-request arena could hold part of a response tree and the pair cells, the parse
  and the serialized text still landed on the no-free global bump. **Blocker #3's
  handler half is now closed** and needs no further upstream work. First consumer:
  `src/core/task.cyr`, measured **1792 → 0 bytes/response**. See roadmap B3.

### Carried forward from 6.5.4

- **`vec_sort_by` / `vec_select_nth`** closed agnosai's own sort filing.
  ✅ **Consumed** — `src/order.cyr` is now a 98-line wrapper over them (was 184).
- **sandhi 1.9.7 → 1.9.8** changed a **return contract** the transport tier depends on:
  all five serve loops previously spun a core forever on a persistent accept error and
  never returned once listening; they now return 1 on a structurally dead listener or
  after 200 consecutive resource failures. `src/server/serve.cyr` is the caller.
- **sigil 3.12.1 → 3.12.2** fixed a 144-byte-per-call `sha256_init` leak. **It never
  affected us** — `_agnosai_auth_secret_eq` uses the banked `sha256()` one-shot, which
  is allocator-free — confirmed by measuring the shared-secret path at **32
  bytes/request** before and after.

  **The sigil pin has to move with the fold.** `cyrius deps` copies each git dep's
  vendored bundle into `lib/` with last-write-wins, so leaving `[deps.sigil] tag =
  "3.12.1"` would overwrite the fold's 3.12.2 back down to 3.12.1.
- Rust (for `rust-old/` only): `channel = "stable"`, currently rustc 1.96.0

## Source

`src/` mirrors `rust-old/src/` — see CLAUDE.md's *Layout* rule. **Regenerated
from the tree 2026-10-04 (2.1.5)** with the commands below; every number was measured, not
carried forward. The previous table was dated 2026-08-12 (112 files, 36,721 lines); at
tag 2.1.4 the tree was 112 files, 38,140 lines, `telemetry/` 1,786:

| Group | Files | Lines | Oracle |
|---|---|---|---|
| `orchestrator/` | 16 | 6,621 | ✅ complete (M5) |
| `server/` | 11 | 4,871 | ✅ complete (M6) |
| `sandbox/` | 10 | 3,798 | ✅ complete (M7) |
| `core/` | 8 | 3,754 | ✅ complete (M2) |
| `fleet/` | 12 | 3,676 | ✅ complete (M8) |
| `server/routes/` | 11 | 2,863 | ✅ complete (M6) |
| `definitions/` | 7 | 2,843 | ✅ complete (M10) |
| `tools/builtin/` | 7 | 2,280 | ✅ complete (M4) |
| `telemetry/` | 3 | 2,270 | ✅ complete (M9) — `mod` + `genai` + OTLP, with the span call sites wired ([ADR 017](../adr/017-genai-span-call-sites.md)). 2.1.5's F6 ([ADR 023](../adr/023-genai-semconv-spans-and-w3c-trace-context.md)): four semconv span kinds (`invoke_workflow`, `invoke_agent`, `chat`, `execute_tool`) pinned at `semantic-conventions-genai` `e07f4eba`, linked by `parentSpanId`, inbound W3C `traceparent` (in process and on three routes), `agnosai_telemetry_init_export` |
| `tools/` | 8 | 1,984 | ✅ complete (M4) |
| `llm/` | 5 | 1,852 | ✅ complete (M3) |
| `learning/` | 6 | 1,043 | ✅ complete (M2) |
| root (port-local) | 8 | 1,571 | no oracle — `main`, `units`, `order`, `id`, `guarded_fetch`, `chan_lossy`, `strcase`, **`arena_pool`** |
| **total** | **112** | **39,426** | against **27,683** lines of `rust-old/src/` |

Regenerate with:

```sh
find src -name '*.cyr' | wc -l
find src -name '*.cyr' | xargs wc -l | tail -1
for d in $(find src -type d | sort); do
  n=$(find "$d" -maxdepth 1 -name '*.cyr' | wc -l); [ "$n" -gt 0 ] || continue
  printf '%-22s %2s %6s\n' "${d#src}" "$n" \
    "$(find "$d" -maxdepth 1 -name '*.cyr' | xargs wc -l | tail -1 | awk '{print $1}')"
done
```

⚠ **The 2026-08-08 table was stale in every row and omitted a whole group.** It
read 97 files / 31,163 lines and had no `definitions/` row at
all, despite M10 completing on 2026-08-09 — so it under-counted by 14 files and
4,787 lines. The `find … -maxdepth 1` per-group form it documented cannot find a
group nobody adds a row for; the loop above enumerates directories instead, so a
new group cannot go missing.

⚠ **The oracle denominator is 27,683; the "41,163" this file asserted counts the
wrong things.** Measured 2026-08-12:

| what | lines |
|---|---|
| `rust-old/src` `.rs` (98 files) | **27,683** |
| all `rust-old` `.rs` — + tests 161, benches 2,494, examples 39, fuzz 46 (124 files) | 30,423 |
| **every file type** in `rust-old` — also `.json` fixtures, `.toml`, `.lock`, `.pem`, `.md` | **40,491** |

41,163 is within 672 of that last row, so it most likely came from an all-files
count taken earlier — **not** from nothing, but from the wrong denominator for a
*source* port: it counts JSON fixtures, manifests and lockfiles as lines to port.
27,683 is what CLAUDE.md has said all along and is the number to use.

⚠ The per-milestone oracle sub-tables further down are **not** affected — the
error was confined to this one headline figure. Spot-checked 2026-08-12: `fleet/`
is exactly 4,443 and `definitions/` exactly 1,460, as they claim.

⚠ **The port being LONGER than the oracle (35,950 vs 27,683) is expected, not
scope creep.** Cyrius sits below Rust: no `?`, no derives, no iterator
combinators, and manual arena threading, so a faithful port of a given module
runs longer. Do not read the ratio as a completeness signal in either direction.

**Oracle groups still owed** (`sandbox/` completed at M7, so this table is
narrower than it was; the scope qualifiers on the rest are **removed** — see the
corrected section below):

| Group | Rust lines | Milestone | Scope |
|---|---|---|---|
| ~~`definitions/`~~ | ✅ 0 left | M10 | **COMPLETE 2026-08-09** — all six modules, 1,460 lines. Both former blockers were already in `lib/`: YAML came from bayan's parser, and ZIP needed one line in `cyrius.cyml`'s `[deps].stdlib` rather than the upstream sankoch ask the roadmap predicted. |
| ~~`telemetry/`~~ | ✅ 0 left | M9 | **Source-complete 2026-08-09** — `mod.rs` (116, with the JSON logging init) and `genai.rs` (206). What remains of M9 is not an oracle file: the **OTLP exporter body**, copied from hoosh's `otlp.cyr`, behind the branch `agnosai_telemetry_init_tracing` already takes. ⚠ The planned **sakshi filing for JSON output was never needed** — `sakshi_set_emit_hook` already exists; see roadmap M9. |

`fleet/` left this table on **2026-08-08**: all twelve of the oracle's modules
are ported.

**M11 bite log — ✅ COMPLETE 2026-08-10, 6 of 6.** Both `hwaccel` halves,
`tools/python_tool`, `sandbox/wasm`, `tools/wasm_tool`, `tools/wasm_loader` —
**48 mutation probes, 48 kills**. ⚠ `wasmtime` is a host requirement and is not
installed here, so the execute path's end-to-end arm is guarded; everything else
— the manifest parse, the header validator, the exit-code classifier, the SDK
stdin wrapper and both result ladders — is reachable without it, because each
was split out of its caller for exactly that reason.

**M11 bite log — 🟡 4 of 6, 2026-08-10.** `sandbox/wasm` (43 assertions, 10/10
mutants) landed on kavach 3.11.8's WASM backend. ⚠ Three upstream round trips
closed to get there: kavach 3.11.8 (backend reachable, stdin, real exit codes),
sankoch 2.7.7 (`zip_bound`, `zip_last_error`, arena-backed readers) and cyrius
6.5.17 (the `distlib` self-check). All three are consumed, not merely filed.

**M11 bite log — 🟡 3 of 6, 2026-08-09.** `llm/router` hwaccel (2 fns, 15
assertions), `core/resource` hwaccel (6 items + a `sched_getaffinity` core
count, ~50 assertions), `tools/python_tool` (50 assertions). 25 mutation probes,
25 kills. Remaining: `sandbox/wasm.rs` (521 — **absent from the roadmap's M11
row until now**), `tools/wasm_loader.rs` (169), `tools/wasm_tool.rs` (265).
⚠ `wasmtime` is **not installed** on this box and kavach's WASM backend cannot
carry stdin or a guest exit code; see the roadmap's M11 row.

> ✅ **Both halves of that last warning are now closed** (2026-08-11), and the
> resolution is worth keeping because it exposed three mutually-concealing bugs.
> `wasmtime` is installed (Arch package `wasmtime`); kavach **3.11.9** added
> stdin, a guest exit code and stderr via `confine_capture_input`, plus
> `backend_is_available(Backend.WASM)`. Installing it turned the sandbox suite
> red and uncovered a chain that only a real module could reach:
> kavach's WASM argv was **never valid** (`--max-memory-size` / `--fuel` emitted
> as top-level `wasmtime run` flags when both live in its `-W` group — fixed in
> **3.11.10**, [ADR 019](../adr/019-wasm-tools-spawn-wasmtime-directly.md));
> `agnosai_wasm_execute` never called `_agnosai_kavach_ensure_init()`; and the
> memory ceiling was parsed but never applied to the policy. Each hid the next.
> ⚠ The tests that were supposed to cover this **could not fail** — the default
> policy emits no flags at all, and a nop module declares no memory section, so
> the suites now set a memory limit *and* a timeout and drive a 41-byte module
> declaring 4 MiB.

**M10 `definitions` bite log — ✅ COMPLETE 2026-08-09.**

| module | oracle lines | assertions | oracle tests |
|---|---|---|---|
| `versioning` | 252 | 45 | 8 |
| `assembler` | 236 | 40 | 7 |
| `k8s_crd` | 238 | 84 | 4 |
| `loader` | 415 | 127 | 15 of 17 |
| `loader` presets | (`src/presets/*.json`) | 137 | the other 2 |
| `packaging` | 303 | 127 | 7 |
| `mod` | 16 | — | — |
| **total** | **1,460 (100%)** | **560** | **43** |

Two things landed alongside the ports rather than inside them. **`src/strcase.cyr`**
is a port-local root module extracted at the third caller
(`server/prompt_guard`, `fleet/cost_planning`, `definitions/assembler`) — no
oracle counterpart, since Rust gets it from `eq_ignore_ascii_case`; 41
assertions. **`src/definitions/presets_data.cyr` is GENERATED** by
`./scripts/gen-presets.sh` from `src/presets/*.json` and committed;
`scripts/check-clean.sh` runs the generator's `--check` so the two cannot drift.
It stands in for the oracle's eighteen `include_str!`s, which Cyrius has no
equivalent for.

`packaging` is the only module in the group that swaps a dependency rather than
porting logic: the oracle's `zip` crate becomes `lib/sankoch.cyr`, and the two
disagree about member names, decodable methods, encrypted archives and whether
the 1 MiB guard is enforced. Seven differences,
[ADR 018](../adr/018-sankoch-path-check-on-import.md), every one of them in the
stricter direction and every one pinned by a test.

⚠ **`GET /api/v1/presets` answers eighteen presets now, not `[]`** — and the
handler moved from `server/routes/tools.cyr` to `server/routes/definitions.cyr`,
which is where the oracle keeps it. The old `[]` was read off the oracle's
*default cargo build*, where `default = []` gates the `definitions` feature off;
this port has no features and ships the whole crate, so the populated arm is the
true one. `tests/server_router.tcyr`'s assertion was inverted with it.

**M8 `fleet` bite log — ✅ COMPLETE 2026-08-08.** Each module carries its own
`.tcyr`; the assertion counts are the suites', not the oracle's:

| module | oracle lines | assertions | oracle tests |
|---|---|---|---|
| `cost_planning` | 275 | 48 | 10 |
| `registry` | 573 | 67 | 20 |
| `placement` | 443 | 58 | 12 |
| `gpu` | 445 | 65 | 15 |
| `state` | 558 | 95 | 17 |
| `environment` | 261 | 62 | 8 |
| `relay` | 350 | 58 | 9 |
| `discovery` | 174 | 29 | 7 |
| `coordinator` | 530 | 81 | 17 |
| `federation` | 587 | 119 | 19 |
| `topology` | 219 | 39 | 6 |
| `mod` | 28 | — | — |
| **total** | **4,443 (100%)** | **721** | **140** |

Two of the group's bites were not just ports. `relay` is a genuine wrapper over
majra only because four upstream defects were fixed first (**majra 2.6.0**);
`coordinator` and `federation` each reproduce oracle arithmetic that reads like
a bug — `max_retries` allowing one fewer retry than it names, and
`declare_coordinator` adopting any term that is not stale.

**`src/main.cyr` is no longer a stub** — as of 2026-08-03 it wires the shared
state, installs a `signalfd` SIGINT/SIGTERM handler, and calls `agnosai_serve`
(roadmap A2, closed). **It drains on shutdown**
([ADR 013](../adr/013-graceful-shutdown-via-signalfd-and-stop-flag.md)); ADR 012,
which recorded that as impossible, is superseded — the blocker was fixed upstream
in sandhi 1.9.9 on agnosai's own filing, hours after it was written.

## Where the port is

**M0–M11 complete. `src/` is FULLY PORTED as of 2026-08-10** —
`llm/inference_queue` was the last module without a Cyrius counterpart, and
`src/llm/mod.cyr`'s claim that it "defers with that feature" was wrong on the
standing rule that a cargo feature gate is not a scope boundary.

**M12's implementation work is COMPLETE as of 2026-08-11** — nine bites, nothing
left under `src/`, `tests/`, `benches/`, `fuzz/` or `examples/`. What remains is
the WASM **execute** path (needs `wasmtime` installed; the manifest half is
tested) and five open decisions: **D1** mount `rate_limit`, **D2**
`"personality": null`, **D3** memoize `builtin_presets()`, **D4** `rounds x batch`
for the bench sweep, **D5** dispatch rows including body serialization. All five
are the user's call, not owed work.

**M12 was never about `src/`.** What is left is the test
and bench surface plus the non-`src` artifacts; `roadmap.md`'s M12 section
carries the measured table and the per-bite status. The headline numbers:
**863 oracle test fns across 84 modules**, **117 oracle criterion bench ids
across 19 files**, against 96 Cyrius suites and 7 `.bcyr`.

⚠ **The bench gate was broken and nobody noticed.** Three of the six `.bcyr`
files did not compile — stale include lists against `src/` — so `cyrius bench`
reported `5 passed, 3 failed` and **50 of the tree's 79 benchmarks, including
the entire 35-shape orchestration set, had stopped running.** Fixed 2026-08-10.

The mechanism matters more than the fix. `cyrius bench` **exits 1** on a compile
error and `bench-history.sh` runs it under `set -euo pipefail`, so it would have
aborted rather than recorded partial rows — the gate was never silent, it was
**never invoked**. Last recorded run: 2026-08-07, 89 rows. The structural hole
was that `check-clean.sh` did not sweep `benches/` and CI never compiled it, so
the only thing between a rotted benchmark and nobody noticing was someone
remembering to run the script. Both closed: `.bcyr` joins the fmt and lint loops,
and CI has a `Benchmarks` step for the compile (not the numbers — CI timings are
too noisy to gate on, and `bench-history.csv` is recorded from a quiet machine).

⚠ **The oracle-test screen came back nearly clean and that is not proof.** It
matches oracle fn-name tokens against suite prose. It found a whole missing
suite (`routes/sse`), so it earned its keep, but a per-module read of oracle
test bodies against suite assertions has **not** been done.

**M0–M6 detail.** The server tier is done: bite 16 (bind) and bite 15c (SSE)
both landed 2026-08-03, so `./build/agnosai` binds, reads the oracle's
environment, answers the full route table, **streams crew events**, and drains
on SIGINT/SIGTERM.

`/api/v1/crews/{id}/stream` now streams. The 501 in
`agnosai_route_dispatch` is unreachable over the transport —
`agnosai_serve_handler` intercepts the route first, because a dispatcher that
returns "a status and a finished body" cannot express a stream. It remains the
honest answer for a direct dispatch, which is what `tests/server_router.tcyr`
still asserts.

**One divergence worth knowing before deploying**: an SSE stream holds one of
the 100 pool workers for its whole life, where the oracle serves effectively
unbounded concurrent streams —
[ADR 014](../adr/014-sse-stream-holds-a-pooled-worker.md) has the tower analysis
showing why the "the oracle starves at 100 too" intuition is wrong.

Verified live, not inferred: `/health` → 200 `{"status":"ok"}`, `/metrics`
renders the registry, `/api/v1/tools` lists the four registered builtins, and
the stream route returns 501. `PORT` / `AGNOSAI_PORT` / `HOOSH_URL` and the four
auth variables were each exercised against a running binary — see the CHANGELOG
for the eight port cases and nine auth cases and what each proves.

Standing decisions that shaped the port, each with its record:

| Decision | Where |
|---|---|
| Money is integer micro-USD, converted to f64 only at the wire | port plan, open question 1 |
| Tool sandbox rides **cx + kavach**, and WASM alongside it | [ADR 006](../adr/006-cx-tool-sandbox.md), [ADR 019](../adr/019-wasm-tools-spawn-wasmtime-directly.md) |
| LLM reached over an **HTTP seam**, not linked | [ADR 003](../adr/003-llm-native-http.md) |
| SSRF guard re-runs on every redirect hop (oracle checks only the first) | [ADR 007](../adr/007-audit-redirect-revalidation.md) |
| Auth compare is SHA-256-digest-based, not the oracle's length-leaking loop | [ADR 009](../adr/009-auth-constant-time-secret-compare.md) |
| JWT requires configured `iss`/`aud` | [ADR 010](../adr/010-jwt-require-configured-iss-aud.md) |
| `/metrics` serves agnosai's registry, not hoosh's | [ADR 011](../adr/011-metrics-endpoint-serves-agnosai-metrics.md) |
| `agnosai_chan_push_lossy` gives tokio-broadcast evict-oldest semantics | `src/chan_lossy.cyr` |
| Crew events and registry status say what happened: dispatch-time `task_started`, per-join `task_completed`, worker `token` events with the spec's crew id, `metadata.agent`, a timeout reported by the runner, `running` stored | [ADR 022](../adr/022-crew-events-and-status-say-what-happened.md) |
| An agent-selection explanation is recomputed from roster and task, never recorded on the run; library only, no wire field (F7) | `src/orchestrator/scoring.cyr` header, *Explaining a selection* |
| GenAI spans are the four semconv operations (`invoke_workflow`, `invoke_agent`, `chat`, `execute_tool`), pinned at `semantic-conventions-genai` `e07f4eba`, linked parent to child by span contexts threaded explicitly (never thread-local), and joining an inbound W3C `traceparent`; ParentBased sampling; no span is built with the exporter off (F6) | [ADR 023](../adr/023-genai-semconv-spans-and-w3c-trace-context.md), amending [ADR 017](../adr/017-genai-span-call-sites.md) |

## Tests

**Current (2.1.5 release gate, 2026-10-04): 99 suites, 8,573 assertions, 0 failed**, plus
**4 fuzz harnesses** (4/4) and **1 doctest**; coverage 99% (1,616/1,628 fns, 102/102 files).
*Now* above has the detail. The rest of this section is dated history: each suite, assertion
and coverage figure in it is as of the date it names, not the current tree.

**At 2026-08-12: 97 suites, all passing** on cyrius 6.5.20, plus 4 fuzz harnesses and 1
doctest. The most recent then: `tests/core_mod_doctest.tcyr` (13 — the ONE real
doctest under `rust-old/src/`; the other three fenced blocks are
`text`/`ignore`/`yaml` and `cargo test` compiles none of them), and:
`tests/llm_inference_queue.tcyr` (69 assertions, 18 mutation probes / 18 kills),
`tests/server_routes_sse.tcyr` (31 — the routes tier's one module that had no
suite), `tests/integration_crew_with_tools.tcyr` (29 — the oracle's own
`rust-old/tests/` integration file, invisible to a per-module screen),
`tests/orch_orchestrator.tcyr` (55, up from 51: it only ever ran ONE crew, so a
single-slot registry passed it), and `tests/tools_wasm.tcyr` from M11.

⚠ **Four `cyrius` gates existed and only ONE was wired into CI.** As of
2026-08-10 all five are: `tests`, **`bench`**, **`fuzz`**, **`doctest`** (via
`check-clean.sh`) and `coverage`. Everything ungated had rotted or was vacuous:

| gate | what was found on 2026-08-10 |
|---|---|
| `cyrius bench` | 3 of 6 `.bcyr` did not compile; 50 of 79 shapes had stopped running |
| `cyrius fuzz` | the only harness was a `cyrius port` scaffold whose `fuzz_main` ignored its input and returned 0 — passing since 2026-07-28 |
| `cyrius doctest` | never run; the tree had no doctest at all, and a note in `src/core/mod.cyr` wrongly claimed the runner did not exist |

⚠ **Two of those were claimed absent without checking.** Bare `cyrius` lists
`fuzz` and `doctest` under *Quality*. Run it before writing around a gap.

None of the three was silent — each exits non-zero. They were simply never
invoked. Read the `N passed, M failed` line on all of them.

**65 suites, all passing** was true at the M7 audit, whose 43 findings are **all fixed** as of
2026-08-05 — `docs/audit/m7-audit-2026-08-04.md` marks each one in place
and `roadmap.md`'s M7 section has the shape of the remediation. The sandbox
suites grew **627 → 727 assertions** working it:

| suite | before | after |
|---|---|---|
| `sandbox_policy` | 90 | **102** |
| `sandbox_oci` | 100 | **118** |
| `sandbox_kavach_bridge` | 93 | **136** |
| `sandbox_spawn` | 144 | **169** |
| `sandbox_process` | 108 | **130** |
| `sandbox_python` | 61 | **76** |
| `sandbox_manager` | 69 | **90** |
| `sandbox_cx` | 62 | **87** |

**41 of the 43 fixes are mutation-verified** — mutation applied, suite re-run,
failing assertion named, tree restored. The two that are not (L5, L9) are
log-only and their mutants survive; the audit document says so rather than
counting them.

**A green suite is not evidence and this is the standing example.** 4,372
assertions and 100% reference coverage coexisted with three live crashes, two
fail-open divergences, and five security controls that could be deleted without
a single failure. Reference coverage counts whether a symbol is *named* by a
test.

**The whole tree is green on cyrius 6.5.20, re-run 2026-08-12**: `cyrius tests
tests` reports **`97 passed, 0 failed`** (suites, not assertions — see the
counting note below), with zero `FAIL:` lines anywhere in the run. Identical to
the pre-bump baseline on 6.5.19, so the toolchain bump is behaviour-neutral here.

⚠ **No assertion total is recorded for that run.** It was captured through
`| tail -25`, which discards the per-suite `(N total)` lines the sum needs — so
quoting one would be inventing it. The earlier figure of **5,559 assertions
across 66 suites (2026-08-05)** was the last one measured at the time (the current measured
total is in the lead paragraph); it predates
31 suites and must not be presented as current. To get a real total, run without
truncating and sum only the `(N total)`-suffixed lines.

⚠ **A note here previously said the full run "exceeds ~570 s and stalls in
`server_sse`". It does not stall** — it completes, well inside a 25-minute
bound, and `server_sse` passes **56 assertions in seconds** when built and run
on its own. It *is* slow, so the per-suite loop is still the faster way to work:
`cyrius build tests/<name>.tcyr <out> && <out>`. The eight sandbox suites total
~75 s that way.

Coverage `cyrius coverage --min 80` → **100% (1561/1561 fns, 102/102 files)**,
re-measured 2026-08-12 on 6.5.20. The denominator counts public symbols only —
`_`-prefixed internals are excluded by design, which is why adding six of them
moved it by one. (The **1099/1099** this line carried until now was the
2026-08-05 figure and had gone stale by 462 functions.)

```sh
cyrius test tests           # 99 suites; each prints "N passed" with an "(N total)" suffix ("cyrius tests" is gone since cyrius 6.7.12)
cyrius coverage --min 80    # the gate, and its own CI step
```

**There is deliberately no per-suite table here any more.** One existed and it
drifted — six rows were wrong at the 2026-07-30 refresh, and the handoff note that
caught it said to *regenerate from command output rather than editing rows by
hand*. A table that can only be maintained by hand will drift again, so the two
commands above are the authority. Counting gotcha if you sum by hand:
`cyrius test tests` prints one line **per suite** carrying an `(N total)` suffix,
plus a final `N passed, 0 failed` line that counts **suites, not assertions** —
sum only the suffixed lines. ⚠ And do not pipe the run through `tail`: it drops
the very lines the sum needs, which is how the 2026-08-12 run ended up with no
recorded assertion total.

Corpus size: **1,927,115 bytes** of `.tcyr` at 2.1.5 (1,125,915 when this note was written). `cyrius coverage` had a fixed 1 MiB
corpus buffer that silently under-reported past it; fixed upstream in **6.5.8**,
so there is no corpus ceiling to work around and no local coverage script.
Re-verified on 6.5.9 by the filing's own repro — padded to 1,765,916 bytes, well
past the 1,376,773 that used to report 85% and 64/75 files, it still reads
75/75 and 100%.

### Test-design decisions worth not re-deriving

The Cyrius suites deliberately exceed the oracle's coverage: they pin the UCB1
formula itself, the `max_by` last-wins tie rule, replay's zero-priority and NaN
fallback branches, and the Q-table's packed-key distinctness — none of which the
Rust tests reach.

**Four seams exist so an offline test can reach a path that needs a network.**
They are a pattern, not one-offs, and the third one confirmed it:

| module | seam | what it makes reachable |
|---|---|---|
| `tools/agnos.cyr` | transport fn pointer on the client (`:67`) | URL construction, query encoding, the path-traversal guards, response reshaping |
| `tools/wasm_tool.cyr` | `agnosai_wasm_tool_output_of` split out of `execute` | the whole result ladder, with `wasmtime` absent |
| `llm/inference_queue.cyr` | `run_item_with(item, client, chat_fp)` | the **success** arm — see below |
| `llm/hoosh.cyr` | chat fn pointer on the client (`AGN_HC_CHAT_FP`, `agnosai_hoosh_client_with_chat`) | the crew runner's LLM **success** arm: `token` events, `metadata.agent`, ADR 020's filter on model output, and the registry read from inside a running crew ([ADR 022](../adr/022-crew-events-and-status-say-what-happened.md)). Since ADR 023 its fourth argument is the task's span context, so a stub checks the `invoke_agent` → `chat` threading offline (`tests/telemetry_wiring.tcyr`) |

⚠ **The inference-queue seam exists because a mutant survived without it.** With
no gateway listening only the failure arm is reachable, and on that arm the
response is unused — so a mutant passing 0 to `settle` instead of the response
passed every assertion. That is the class of defect these seams exist to catch:
not "the code is wrong" but "no test can tell." Nothing is injected in
production in any of the four.

**A threaded test must wait on a CAUSE, not a duration.** `server_routes_sse`
drives the streaming handler on its own thread and ends it by removing the crew
from the bus; the handoff waits for the sender's **receiver count** to reach 2
rather than sleeping. A sleep races in the direction that hides the bug — tear
the crew down too early and the handler takes the unknown-crew path, where every
assertion still has something to match on the wrong frame. Same reason
`llm_inference_queue` gives the worker an `EXITED` flag: `_stopped` only reads
back what `_stop` wrote, so a worker ignoring the flag entirely passed until the
thread's own acknowledgement was observable.

`tools_builtin_load_testing.tcyr` could not follow its oracle's shape. Both Rust
tests stand up an axum mock server on loopback, and `agnosai_is_safe_url` correctly
refuses loopback — so the tool cannot be aimed at one, and pointing a test suite at
a public host is not acceptable. The **real OS-thread fan-out** runs against a
synthetic executor instead: worker threads, per-worker arenas, deadline and budget
loops, status aggregation, the sort and the percentile indices, no network. The one
path left untested is sandhi's behaviour under `sandhi_http_get_a`, covered
separately by the live `scripts/stack.sh check`.

`tools_builtin_security_audit.tcyr` solves it the same way and pays off better:
because the module is split at the network boundary, all five of its mock-server
oracle tests port exactly — `_t_mock_headers` transcribes
`mock_audit_server(security_headers, cors_wildcard)` down to the `Apache/2.4.99` it
always sets. It then goes past the oracle: both sides of every risk-band boundary,
the reflected-origin CORS bypass the probe origin exists to catch, the `to_str()`
visible-ASCII gate, case-insensitive scheme handling, and a snapshot-survives-reset
test that scribbles over the released arena so a borrowed pointer shows up as
corruption rather than passing silently.

`tools_agnos.tcyr` is where the transport seam pays off most. The oracle's three
suites test **only** names, descriptions and schemas, because every execute path
needs a live service. Since `agnosai_agnos_client_new` takes its transport as a
function pointer, a recording stub turns the whole untested half into ordinary
assertions: URL construction, form encoding, body construction, path-traversal
guards, response reshaping.

**Allocator assertions are a first-class test category now.** A module threaded
for arenas should carry an `alloc_used()`-delta assertion (0 bytes over N
iterations) plus a byte-identical-wire check against its global twin. Both are
**mutation-verified** where they exist — un-threading a single call fails them.
Treat that pair as the contract for any further threading work.

⚠ **"Every module" was FALSE, corrected 2026-08-12.** `tests/telemetry_otlp.tcyr`
has **no `alloc_used` assertion at all**, and `src/telemetry/otlp.cyr` accordingly
threads `str_builder_new_a` and then makes **29 bare `str_builder_add*` and 4 bare
`str_builder_build`** calls, every one of which lands on the global bump.
`tests/orch_audit.tcyr` has 4 such assertions and still missed 6 bare calls in
`orchestrator/audit.cyr`. Verify per module with the sweep in the *Open findings*
section near the top of this file; do not trust this paragraph's old claim.

**Four things that made an arena assertion look stronger than it was.** All four
were caught by mutation, and all four generalise to any further threading:

1. **An empty fixture cannot tell a threaded route from an un-threaded one.** An
   empty collection allocates almost nothing either way. `/api/v1/dashboard/agents`
   is the sharp case: it emits an object only for a result whose metadata *names*
   an agent, so a fixture of eight crews with no agent metadata renders `[]` and
   times its guard.
2. **A route's failure arms are separately threadable and separately
   forgettable.** Measuring only the success path let `route_error_a` revert to
   `route_error` inside `agnosai_route_get_crew_a` with the whole suite green.
   The 404 is also the arm an unauthenticated scan hits hardest.
3. **Round-number thresholds assert less than they look like they do.** 128 B sat
   above a 32 B baseline and below the 176 B of the smallest un-threading
   mutation — and still passed one, because a single hoisted `str_from` costs
   16 B and lands at 48. Prefer a bound **measured in the same run**:
   `agnosai_route_resolve`'s own cost is what the arena path should equal, so
   asserting against it is self-calibrating across allocator changes.
4. **Ratios stop discriminating once the numerator is near zero.** At 4,160 → 32
   every plausible mutation still clears 4x. Absolute equality against a measured
   residual is the stronger statement and the one that is actually true: the
   handler half is zero, not merely small.

### What a crew run allocates

Per **four-task** crew run, no audit chain, no event subscribers:

| part | bytes | whose |
|---|---|---|
| building the spec (crew + 4 tasks + agent + vecs) | 4,472 | the **caller's** — in production, the request parse |
| `orch_crew_runner`'s own | **6,088** | this module |

Within `crew_runner`'s own: `agnosai_execute_task` ~528 B/task, crew state plus
four results ~200 B, remainder is the profile and metrics. No single remaining
target large enough to be worth a bite.

With a chain attached the audit path adds ~7 KB per four-task run (was ~18 KB
before the hashing scratch).

### JSON keys are hoisted, and pointer identity is what pins it

`AGN_JK_*` in `core/crew.cyr` and `core/task.cyr` hold the fifteen serialiser
keys as process-lifetime `Str`s over static literals. A `str_from_a` per key per
object costs ~32 ns and a 16-byte header; measured, four sets ran 430 ns with
keys allocated against 314 hoisted, and `agnosai_crew_state_to_value_a` went
**1,661 → 1,216 ns**.

⚠ Distinct from the constant-*return* hoist this file declines (149 sites, 2.5%
of bytes, 121 symbols). Fifteen keys, measured in time, on the paths every
serialising route runs.

**The assertion is pointer identity, not a byte bound.** A single re-allocated
key is 16 bytes and disappears inside any usable threshold — a 1,600 B bound
over a 1,488 B baseline caught a wholesale regression and missed all three
single-key mutations. `bayan_json_v_obj_key(v, i) == AGN_JK_*` is exact and
needs no calibration.

### In-loop `str_from`

Two classes, and the distinction matters: a bare `str_from` in a loop costs 16 B
on the no-free global bump per iteration; `str_from_a` costs arena bytes that the
next `reset_via` reclaims. 27 bare ones were found and **24 fixed**; 19
arena-backed ones remain and are not worth chasing.

⚠ **Three bare ones stay on purpose** — `sandbox/oci.cyr:250-251` and
`crew_runner.cyr:990` are inside loops but on `return` paths, so they run at most
once and hoisting would move an allocation onto the hot path. A naive
"`str_from` in a loop" scan will keep reporting them.

### Harness rules

- Shared helpers live in `tests/test_helpers.cyr`, all `_t_`-prefixed, so they can
  never shadow a `src/` symbol and stay out of the coverage denominator.
- Every `.tcyr` ends `var f = main(); if (f > 0) { f = 1; } syscall(60, f);` — the
  stock `proj-tcyr` epilogue masks the exit code `& 0xFF`, so exactly 256/512/768
  failures would score PASS.
- **A crash prints no assertion output.** A suite that dies mid-run reports as a
  failed *suite* with no `FAIL:` line — if a suite count drops with no failing
  assertion named, look for a segfault, not a bad expectation.
- Never `include` a stdlib module in a `.tcyr`; the stdlib is auto-prepended and an
  explicit include lands after it, single-passing into undefined symbols.
- Include order inside a `.tcyr` is load-bearing, callees before callers — e.g.
  `src/server/prometheus.cyr` must precede `src/orchestrator/crew_runner.cyr`.

Rust oracle for comparison: **863 unit + 2 integration + 1 doctest, all passing.**

## Benchmarks

**Current (2.1.5, 2026-10-04): 11 `.bcyr` files, 223 rows per sweep** — `bench-history.csv`'s
2.1.5 sweep, and the release gate's `cyrius bench` reads `11 passed, 0 failed`. *Now* above says
which earlier sweep to read the rows against. The rest of this section is dated history; each
row count in it is as of the date it names.

**At 2026-08-12: 11 `.bcyr` files, 207 rows per sweep, all compiling** — re-run on
6.5.20 after the OTLP fix: `11 passed, 0 failed`. `benches/telemetry.bcyr` is NEW (8 ids) — `grep
'agnosai_otlp_' benches/*.bcyr` was empty, so the span encoder every instrumented
operation crosses had never been timed.

Row-by-row against the pre-bump sweep: 199/199 prior ids in common, ZERO
regressions past 15% on the toolchain bump, one improvement
(`prompt_scan_clean_500b`, 95,555 → 63,109 ns, −34.0%). After the OTLP fix, two
ids move past 15% — `default_model_premium` 40 → 54 ns and `metrics_record_task`
4 → 5 ns. ⚠ Both are **noise, not regressions**: sub-100 ns shapes against a
measured ~1.3 µs timer floor, where one nanosecond is 25%, and neither is on the
telemetry path.

After the crew-timeout and `output_filter` work: **0 regressions past 15% across
all 207 ids**. The crew path moves `run_crew_1_task_sequential` +6.4%,
`run_crew_10_tasks_sequential` +8.0%, `run_crew_10_tasks_parallel_4` +1.7% —
that is the timeout's per-loop-head deadline check, not the filter, which is free
on this path with redaction off. ⚠ An earlier shape of the filter — scanning the
placeholder path unconditionally — measured **+90%** on the 10-task sequential
run and was restructured rather than shipped; `agnosai_output_scan` alone is
16.7 µs against a ~40 µs/task crew path. See ADR 020. Previously verified
2026-08-11 — against 106 rows and three non-compiling files that morning. See
the Tests section for how they had stopped. `benches/llm.bcyr`,
`benches/fleet.bcyr` and `benches/definitions.bcyr` are new; `llm` was the first
to touch `src/llm/` at all, and `harness.bcyr` is the relocated `noop` floor.

⚠ **The audit is remediated, and the audit itself had six errors.**
[`m12-bench-audit-2026-08-10.md`](../audit/m12-bench-audit-2026-08-10.md) carries the six
adversarial reports verbatim, plus a *Known errors IN this document* section —
a wrong `.bcyr` count used as load-bearing reasoning, 15-vs-14 field arithmetic,
a bad struct citation, a wrong envelope byte count, "fires twice" where three
fire, and a "small fraction" that is 41–44%. **42 findings applied, 14 rejected
with evidence, 11 escalated.** Verify against the tree before acting on any of
it, including the corrections.

⚠ **BENCH DISCONTINUITY 2026-08-11 — nine rows changed what they measure.**
`benches/server.bcyr`'s fixture registered one tool where the oracle registers
two (moves `route_tools_*` and `route_mcp_*` by 50–108%), and
`benches/tools.bcyr`'s registry keys were unpadded with a mismatched miss key
(re-baselines the five `tool_registry_*` ids). Both corrections are right on the
merits. Every other pre-existing row is within ±3%, so the discontinuity is
exactly those nine. **Do not compare them across this date.**

### The rate limiter on the default path, 2026-08-13 — 212 rows (was 207)

⚠ **`benches/server.bcyr` included `src/server/rate_limit.cyr` and benchmarked
nothing from it.** Mounting the limiter by default ([ADR 021](../adr/021-rate-limit-mounted-by-default.md))
therefore put a cost on every request that no row covered — and the sweep run
that showed "no regressions" was never evidence about the change. Five rows now
cover it, reproducing tightly across two independent idle-box runs:

| row | run A | run B |
|---|---|---|
| `rate_limit_check_hot` | 1.511 µs | 1.507 µs |
| `rate_limit_check_new_key` | 2.189 µs | 2.195 µs |
| `rate_limit_client_key_global` | 190 ns | 190 ns |
| `rate_limit_client_key_arena` | 174 ns | 175 ns |
| `rate_limit_sweep_1k_live` | 7.227 µs | 7.188 µs |

- ⚠ **~80% of the check is ONE clock read, not the bucket.** `time_now_ns` is
  unavoidable for a lazy-refill token bucket, and this box measures a clock read
  at **1.211 µs** — `clock_epoch_secs_baseline` reads 1.210 µs and the harness
  independently reports 1.211 µs as its own timer floor. Mutex, map lookup,
  arithmetic and counters together are the remaining ~300 ns. **There is nothing
  worth optimising in the bucket**; do not re-derive this as a finding.
- **The arena key fix is faster as well as leak-free** (174 ns vs 190 ns), so it
  is not a trade.
- **The sweep amortises to ~28 ns/check** (7.19 µs / `AGNOSAI_RL_SWEEP_EVERY` =
  256), ~113 ns/check at the `AGNOSAI_RL_MAX_KEYS` ceiling.

⚠ **Reading the sweep-to-sweep diff needs care on this tree.** Sub-10 ns rows
(`noop` at 2 ns, `auth_check_disabled` at 6 ns) swing ±50% on a one-nanosecond
quantum and mean nothing, and `select_nth_100k_already_sorted` carries genuine
intra-run dispersion — one run reported +10.5%, min 3.625 / max 4.474 ms, and
the next reverted to −8.9%. Check a row's own min/max before calling it a
regression. On the idle-box pair above, the worst real mover was **+2.3%** and
**zero rows moved more than 10%**.

### Two O(n²) removals, 2026-08-11 — both found by benchmarks that did not exist

| what | before | after | |
|---|---|---|---|
| `rank_agents` at 1000 agents | 5093.0 µs | **1021.0 µs** | −80.0% |
| `scheduler_load_dag` at 500 tasks | 3129.0 µs | **1271.0 µs** | −59.4% |

**`agnosai_rank_agents`** hand-rolled an insertion sort where the oracle uses
`sort_by`. Now `agnosai_sort` (introsort). Scaling per 10x the agents dropped
**39.9x → 10.7x**. Safe because `_agnosai_scored_cmp` is a total order — index
breaks every score tie — which is what makes stability irrelevant; the old
comment had that backwards, treating stability as load-bearing and the tie-break
as belt-and-braces.

**`agnosai_scheduler_load_dag`** insertion-sorted every DAG key before
`kahn_sort`, which sorts the seed and each successor list itself — so the
caller's order could never reach the output. Not a divergence, **dead work**; an
audit agent reported it as a divergence needing an ADR and was wrong.

⚠ **Neither was visible before the benchmarks.** The only ranking bench used 16
**identical** agents, so the insertion sort did zero shifts. Both guards were
widened *before* the change — 40 identical agents (past introsort's 16-element
threshold, below which a smaller case proves nothing) and twelve roots in reverse
— and both suites report identical results either side.

### majra's `pq_dequeue` is O(n), so draining its priority queue is O(n²)

Found by `benches/llm.bcyr`, which is the argument for writing the benchmark
before assuming the dependency is fine. `pq_dequeue` pops with
`vec_remove(tier, 0)`, shifting the whole tail. Mean cost of one pop while
draining a tier of that depth:

| depth | per pop | ratio |
|---|---|---|
| 2,000 | 2.00 µs | — |
| 4,000 | 4.02 µs | **2.01×** |
| 8,000 | 7.92 µs | **1.97×** |
| 16,000 | 15.56 µs | **1.96×** |

Doubling the depth doubles the per-pop cost, which rules out cache effects. At
200,000 queued the mean pop was **198.7 µs** and the drain took ~40 s of memmove.

⚠ **This is the design case, not a pathological one** — `llm/inference_queue`
exists so background work is *allowed* to accumulate behind interactive work.
Filed upstream. `benches/llm.bcyr` measures the drain at **two** depths so the
slope stays legible here; if majra fixes it, the two rows converge, which is a
better regression signal than either number alone.

### The route-latency floor is `alloc_via` — measured 2026-08-07, largely fixed the same day

Do not re-derive this. Every threaded route's remaining cost is dominated by the
**number of allocations**, and each one costs an `alloc_via`.

On **6.5.9** that was **15.1 ns** — measured against a 16 ns `reset_via` control,
ten per iteration. `arena_alloc`'s fast path is ~8 instructions, so that was not
the bump; it was the five-call chain above it. Filed upstream, and **6.5.10
shipped both suggested fixes**: `alloc_via` now inlines its two accessor loads,
and `arena_allocator` registers `&arena_alloc` / `&arena_reset` directly instead
of the `_arena_*` trampolines.

**On 6.5.10 `alloc_via` is 11.1 ns** (−26%), `reset_via` 12. The remainder is
inherent to a vtable and not worth chasing: the hand-inlined
`fncall2(load64(a), load64(a+32), size)` measures 8.9 and `arena_alloc(state,
size)` called directly measures 6.2, so what is left is `alloc_via`'s own frame
plus one indirect call. **Treat ~11 ns per allocation as the floor.**

Counted exactly with a counting allocator wrapped around the arena's vtable:

| route | allocations | `alloc_via` share of latency (6.5.10) |
|---|---|---|
| `GET /api/v1/dashboard/crews` | 112 | 1,243 ns of 4,656 — **27%** |
| `GET /api/v1/crews/{id}` | 59 | 655 ns of 2,245 — **29%** |
| `POST /mcp` `tools/list` | 40 | 444 ns of 2,486 — 18% |
| `agnosai_route_resolve_a` alone | 3 | 33 ns |

Those counts are **post-hoist** and now structural: `bayan_json_v_obj_new_a` is
three allocations on its own (node, vec struct, vec data) and each pair is two
more, so a four-field object is eleven before any nesting. The response shapes
are parity-fixed against the oracle, so the count cannot fall further without an
ADR. With the upstream fix now landed, **there is no large lever left on these
routes** — the remaining cost is the object graph the wire format requires.

Two things that were measured and are *not* levers, so they need not be retried:
the dispatch shell between `route_resolve_a` and the handler is **110 ns** (A/B
in one arena: full dispatch 2,595 vs. resolve-plus-handler 2,485), and
`bayan_json_v_obj_set_a` is already minimal — pair plus `vec_push`, no duplicate
scan, and `vec_new_a` pre-sizes to capacity 16 so pushes never realloc.

`benches/learning.bcyr` — the 10 shapes of `rust-old/benches/learning.rs`. First Cyrius numbers
(x86_64 Linux, cyrius 6.5.2):

| Benchmark | Time |
|---|---|
| `capability_scorer_confidence_50caps` | 81 ns |
| `ucb1_select_10arms` | 250 ns |
| `capability_scorer_record_50caps` | 284 ns |
| `profile_success_rate_20agents` | 337 ns |
| `ucb1_select_50arms` | 1.13 µs |
| `qlearner_best_action_1000_state_actions` | 1.27 µs |
| `profile_record_20agents` | 1.84 µs |
| `qlearner_update_1000_state_actions` | 1.92 µs |
| `replay_buffer_push_1000cap_full` | 7.18 µs |
| `replay_buffer_sample32_from1000` | 583 µs |

The two replay figures are the oracle's own algorithmic cost, not a port regression: `push` runs
an O(n) lowest-priority scan on every insert once full, and `sample` recomputes the remaining
priority sum each round — which `replay.rs:90` does deliberately, "to avoid bias". Changing
either would diverge from parity and needs an ADR first.

`benches/core.bcyr` — no oracle bench file exists for core, so these measure the paths core
actually gets hammered on: id minting/rendering and the JSON round trips at every request
boundary.

| Benchmark | Time |
|---|---|
| `hw_inventory_satisfies` | 165 ns |
| `uuid_to_str` | 217 ns |
| `uuid_parse` | 252 ns |
| `uuid_v4_generate` | 526 ns |
| `message_to_json` | 8.67 µs |
| `task_from_json` | 10.1 µs |
| `agent_from_json` | 10.2 µs |
| `task_to_json` | 14.7 µs |
| `crew_to_json_10x10` | 305 µs |
| `crew_from_json_10x10` | 710 µs |

`uuid_v4_generate` is dominated by the `random_bytes` getrandom syscall, which is the right
trade: v4 ids must come from kernel entropy, never tyche. If id minting ever shows up hot, the
fix is batching entropy, not changing the source.

`benches/order.bcyr` — the numbers that settled **port plan blocker #8**, and
then retired the code that settled it. The plan measured an O(n^2) insertion sort
over agnosai's 100k percentile vector at **52.6 s**. `src/order.cyr` first
vendored heapsort + quickselect to fix that; at cyrius **6.5.4** `vec_sort_by` /
`vec_select_nth` shipped (closing agnosai's own filing) and the module became a
thin wrapper. Both transitions, measured on this box:

| Benchmark | plan baseline | vendored | stdlib (current) |
|---|---|---|---|
| `sort_100k` | 52.6 s | 79.6 ms | **20.3 ms** |
| `sort_10k` | — | 6.30 ms | **1.71 ms** |
| `sort_100k_already_sorted` | — | 79.1 ms | **3.31 ms** |
| `three_percentiles_100k` | ~21 ms predicted | 10.7 ms | **7.81 ms** |
| `select_nth_100k` | — | 6.89 ms | **5.09 ms** |
| `select_nth_100k_already_sorted` | — | 4.29 ms | **3.77 ms** |

Against the 52.6 s baseline that is **~2,600x** for the full sort. The
already-sorted row moved 23.9x in the last step alone and is a difference in
kind: heapsort's worst case equals its average so it had no fast path, while
introsort checks for pre-sorted input first. A latency vector that arrives
roughly ordered now costs a scan.

`src/order.cyr` is **98 lines, down from 184** — the vendored algorithms are
gone. What did NOT delegate is the bounds contract: `vec_select_nth` aborts the
process on `k < 0` or `k >= len`, where `agnosai_select_nth` returns 0, pinned by
three assertions in `tests/order.tcyr`. An empty latency vector is an ordinary
state for a load test that recorded no samples.

`benches/tools.bcyr` — the tool path, principally the aggregation that runs after the
`load_testing` worker threads join. `order.bcyr` already covers the sort and the selects in
isolation; what is measured here is what load_testing adds on top.

| Benchmark | Time |
|---|---|
| `tool_registry_get` | 115 ns |
| `tool_execute_echo` | 973 ns |
| `is_safe_url_public_host` | 1.30 µs |
| `is_safe_url_octal_host` | 1.38 µs |
| `lt_result_to_value` | 2.80 µs |
| `lt_aggregate_100k_10workers` | 79.2 ms |
| `lt_aggregate_100k_500workers` | 81.5 ms |
| `lt_aggregate_100k_200codes` | 149 ms |

Two design choices were on trial and both held:

- **The cross-worker merge is O(total), not O(workers).** Spreading the same 100k samples over
  500 workers instead of 10 costs 2.3 ms more — within noise of the 74.6 ms sort that dominates
  both. Each worker owns an arena freed with it, so aggregate copies rather than aliases; that
  copy is ~5 ms at the cap.
- **Status counts as a linear pair vec, not a map.** There is no `map_u64_keys` in the stdlib,
  so codes live in a vec of `[code, count]` pairs. At 200 distinct codes the scan costs 1.9x —
  but real HTTP has ~60 defined codes and a real run sees one to five, so the vec is the right
  call and the 149 ms row is the documented ceiling rather than an expected cost.

The two `is_safe_url` rows matter because every load test pays one before it touches the
network: the octal-host path, which must parse four different spellings before it can decide,
costs only 6% more than the plain hostname path.

`benches/orch.bcyr` — the orchestration path: the event fan-out on every task boundary, the
per-task ranking and delegation, the scheduler's DAG sort, the plan-cache key, and the
conversation trim that runs on every push. Logging is turned down to `SK_FATAL`, since
`agnosai_delegate_tasks` emits an INFO line per call (matching the oracle's `info!`) and left at
the default level the number measures sakshi writing to a pipe.

| Benchmark | Time |
|---|---|
| `conv_buffer_push_sliding_32` | 120 ns |
| `pattern_match_wildcard` | 803 ns |
| `event_round_trip_1_sub` | 908 ns |
| `plan_cache_get_hit` | 1.72 µs |
| `event_send_evicting` | 933 ns |
| `pubsub_publish_4_patterns` | 4.50 µs |
| `plan_key_16x16` | 11.7 µs |
| `rank_agents_16` | 12.8 µs |
| `kahn_sort_64_nodes` | 57.6 µs |
| `event_fanout_64_subs` | 53.40 µs |
| `delegate_16_tasks_16_agents` | 204 µs |
| `durable_deserialize_crew_state` | 2.68 µs |
| `durable_load_miss` | 2.80 µs |
| `durable_serialize_crew_state` | 3.88 µs |
| `durable_load_hit` | 4.43 µs |
| `durable_mkdir_p_existing_4deep` | 6.04 µs |
| `durable_save_atomic` | 21.5 µs |
| `crew_select_model_routed` | 301 ns |
| `crew_infer_provider_fallthrough` | 594 ns |
| `crew_build_system_prompt` | 1.47 µs |
| `prompt_scan_clean_67b` | 8.04 µs |
| `prompt_scan_clean_4k` | 273 µs |
| `audit_record` | 29.5 µs |
| `audit_verify_256` | 2.65 ms |
| `output_scan_clean` | 6.98 µs |
| `output_redact_clean` | 9.69 µs |
| `metrics_record_task` | 5 ns |
| `metrics_gather` | 7.94 µs |
| `auth_check_disabled` | 6 ns |
| `auth_check_secret_short_token` | 942 ns |
| `auth_check_secret_ok` | 980 ns |
| `auth_check_secret_reject` | 961 ns |
| `auth_jwt_reject_bad_alg` | **1.201 ms** |
| `auth_jwt_key_prepare` | 10.5 µs |
| `auth_jwt_verify_ok` | **1.202 ms** |

The three `auth_check_secret_*` rows are the evidence for
[ADR 009](../adr/009-auth-constant-time-secret-compare.md), not padding. They sit within
**4%** of each other — accept 980 ns, reject 961 ns, and a 1-byte token against the same
9-byte secret 942 ns. The oracle's `max(a.len(), b.len())` loop would have made that last
row track the *secret's* length, which is the timing leak the ADR closes; the residual
~28 ns spread tracks the **token's** length, which the attacker chose and already knows.
`auth_check_disabled` at 6 ns is the first branch, and it is what every deployment that
has not configured auth pays per request. **That row is also why
`agnosai_auth_check` reads the clock only on the JWT path**: the obvious spelling,
`check_at(..., clock_epoch_secs())`, reads it before the config is inspected, and the bench
caught that turning this row into 1.35 µs and `auth_check_secret_ok` into 2.35 µs — both
paths where `now` is never looked at.

**`auth_jwt_verify_ok` was a sigil finding, and sigil fixed it.** It measured
3.31 ms while `lib/sigil.cyr` was 3.12.1-era, with the raw
`rsa_pkcs1v15_verify_sha256` at 3.29 ms — the port's own parsing, base64 and JSON
work was the remaining ~20 µs. sigil's "rsa repairs" (filed by agnosai as
`2026-07-30-rsa-verify-uses-secret-exponent-ladder.md`, archived upstream) removed
a constant-time secret-exponent ladder from the *public verify* path, where the
call site's own comment said it was unnecessary. Re-measured on 3.12.2:

| | 3.12.1 era | 3.12.2 (current) |
|---|---|---|
| `auth_jwt_verify_ok` | 3.31 ms | **1.202 ms** (2.75x) |
| `auth_jwt_reject_bad_alg` | 3.29 ms | **1.201 ms** |

Per-core JWT ceiling accordingly moves **~300/sec → ~830/sec**. A residual gap
against OpenSSL (14 µs for the same RSA-2048 verify on this box) remains and is
the inherent portable-bignum cost, not a further defect.

**`reject_bad_alg` still costs the same as a valid verify, and that is deliberate.**
The `alg` check sits *after* signature verification, so a rejected token pays the
modexp. The cheaper ordering was measured and rejected: checking `alg` first
rejects in 3.75 µs but parses attacker-controlled JSON before authenticating, and
`bayan_json_v_parse_buf` allocates on the no-free global bump — **62,248 bytes per
rejected request, ~53x amplification**, no credential required. Permanent memory
exhaustion is worse than recoverable CPU exhaustion. The answer to a flood is
`rate_limit`, which is ported but **not mounted** (roadmap D1, a human decision).

**These numbers were futex-bound and the stdlib fix landed in cyrius 6.5.9.**
`lib/sync.cyr`'s two-state mutex called `FUTEX_WAKE` on every release: 394 ns
per uncontended pair, now **48 ns** with the three-state lock. agnosai changed
nothing and gained 15–77% across every lock-bound benchmark — `tool_registry_get`
512 → 115 ns, `event_round_trip_1_sub` 1,982 → 908, `event_fanout_64_subs`
100.6 → 53.4 µs, every route arm 15–37%. The tables above are the post-fix
baseline.

⚠ The `chan_*` figures below predate the bump and are due a re-measure.

⚠ Both figures more than halved at the 6.5.9 mutex fix and the *gap* between them
closed with them — at 6.5.8 they were 2.40 µs against 1.99 µs. The conclusion is
unchanged and now holds with more margin, but the arithmetic behind it was
mutex-dominated and should not be quoted from the old numbers.

`delegate_16_tasks_16_agents` at 204 µs is 16 × `rank_agents_16` (12.8 µs) plus change, which is
the expected shape — hierarchical mode ranks every task independently, with no memoisation
across tasks. That is the oracle's behaviour and the reason the cost is linear in tasks × agents.

`benches/server.bcyr` — the HTTP request path, added 2026-08-05. No oracle bench
file exists: the Rust request path is axum's, and timing it would have measured
tokio. The port's is `agnosai_route_dispatch`, ordinary synchronous code.

Each threaded read route is timed on **both** arms, because threading is not free
by construction — every `_a` call carries an extra argument and `alloc_via` is one
indirection past `alloc`. The paired rows are what makes "the arena bought the
memory without costing latency" a measurement rather than an assertion. Fixture:
8 finished crews carrying agent metadata, 8 pending approvals, 1 definition, 1
registered tool. `reset_via` is **inside** the timed loop, since a sandhi worker
pays it per request.

| Benchmark | global | arena | Δ |
|---|---|---|---|
| `route_approvals_*` | 1.543 µs | **1.300 µs** | -15.7% |
| `route_tools_*` | 2.444 µs | **1.659 µs** | -32.1% |
| `route_definitions_*` | 2.674 µs | **1.997 µs** | -25.3% |
| `route_get_crew_*` | 4.231 µs | **3.126 µs** | -26.1% |
| `route_dashboard_crews_*` | 8.859 µs | **6.887 µs** | -22.3% |
| `route_dashboard_agents_*` | 11.914 µs | **9.696 µs** | -18.6% |
| `route_approvals_post_*` | 3.096 µs | **2.580 µs** | -16.7% |
| `route_mcp_*` | 4.861 µs | **3.316 µs** | -31.8% |
| `route_resolve` | 325 ns | — | in both |

The arena arm wins on all six because the global allocator is a **no-free bump**:
a request's garbage is never reclaimed, so a long-lived process walks an
ever-growing heap and loses the locality a reset arena keeps.

The other half of the same claim is memory, asserted rather than benchmarked —
`tests/server_serve.tcyr` measures it over the same fixture, bytes charged to the
global bump per request:

| Route | global | arena |
|---|---|---|
| `GET /api/v1/dashboard/agents` | 4,368 | **0** |
| `GET /api/v1/dashboard/crews` | 4,160 | **0** |
| `GET /api/v1/crews/{id}` | 2,352 | **0** |
| `GET /api/v1/tools` | 1,720 | **0** |
| `GET /api/v1/approvals` | 576 | **0** |
| `GET /api/v1/agents/definitions` | 384 | **0** |
| `GET /api/v1/crews/{unknown}` (404) | 320 | **0** |

**Every GET read route charges the global bump literally nothing.** Handler,
router, id validation, tool schemas and the response struct all land in the
request arena and are freed by one `reset_via`.

### Arena exhaustion: the request arena spills

An exhausted arena returns 0, and a `Str` of 0 is indistinguishable from a valid
one — there is no option type or error channel through the `_a` families, so the
0 flows on and the next deref faults. The primitives are fine
(`arena_alloc` → 0, `str_from_a` → 0, `vec_push_a` → -1); nothing above them can
practically check.

**The request arena is therefore set to `ARENA_FULL_SPILL`** in
`server/serve.cyr` — overflow goes to the global bump instead of faulting.
Spilled bytes are not reclaimed, which is exactly the pre-threading cost applied
to the overflow alone.

**SPILL, not GROW, and the test pins it.** GROW retains chunks across
`arena_reset`, so each of 100 workers would hold its worst-ever request forever.
`server_serve` asserts `arena_capacity_total` is unchanged as well as that the
arena filled, so swapping the policy fails.

cyrius 6.5.9 added the policy (`ARENA_FULL_NULL`/`GROW`/`SPILL`/`ABORT`) in
answer to an agnosai filing; there is no local wrapper any more.

### The write routes — every one threaded, three keep a floor

| write route | B/req global | B/req arena | retains parse-tree data? |
|---|---|---|---|
| `POST /mcp` | 3,224 | **0** | no |
| `POST /api/v1/approvals` | 1,352 | **0** | no |
| `POST /api/v1/agents/definitions` | 2,280 | **392** | yes, cloned |
| `POST /api/v1/crews` | 21,184 | **17,048** | yes, cloned |
| `POST /api/v1/a2a/receive` | 17,192 | **15,624** | yes, cloned |

Every residual is **retained state, not garbage**: 392 B is the stored agent
definition; the crew figures are execution results held in the orchestrator
registry.

**The ownership rule, which is the durable part.** A deserialiser that feeds
process-lifetime state must **clone**, not borrow, or a threaded parse leaves the
stored object pointing into a reclaimed request body — corruption, not a crash.
`agnosai_agent_from_value_a`, `agnosai_crew_req_from_value_a`,
`agnosai_task_req_from_value_a` and `agnosai_a2a_req_from_value_a` all take an
allocator and clone into it; the routes pass `default_alloc()` for whatever is
retained and the request arena for everything else.

**What actually retains, on the crew path**: the **audit chain** (the crew name
is an audit *message*) and the **task result** (the description becomes its
output, `crew_runner.cyr:460`). `CrewState` itself holds only a minted UUID, a
status, results and a profile — asserting on `crew_state_crew_id` pins nothing.

`agnosai_audit_record` clones `event`, `level`, `message`, `provider`, `model`.
⚠ `metadata` is stored **by reference** — bayan has no deep-copy primitive — so
callers must pass a tree that outlives the chain.

⚠ `agnosai_crew_from_value` is *not* on any request path: it deserialises a
persisted crew, needs an `id` no request carries, and has no caller in `src/`.

## Dependencies

**stdlib** (45 declared, order-sensitive — rationale in [`cyrius-port-plan.md`](cyrius-port-plan.md)):
base substrate · general utilities · `unicode` · bayan · patra · concurrency+crypto floor ·
dynamic-link floor · async · net/http/tls/ws/sakshi/sandhi · sankoch

**git deps** (read from `cyrius.cyml` 2026-10-04, 2.1.5; *Now* above has the detail):
sigil 3.13.7 (the 6.6.14 fold) · bote 3.3.16 · majra 2.9.2 · kavach 3.13.2 · ai-hwaccel 2.4.1 ·
tyche 1.1.0, none with an active `path`. Transitively through bote: libro 2.10.6. Upstream tags
checked through the GitHub API the same day: the last five are their repos' latest, and sigil's
repo is at 3.13.9, which agnosai takes with the cyrius 6.6.15 pin (roadmap, *Moving the cyrius
pin to 6.6.15*).

The hoosh seam targets **hoosh 2.6.0** — `usage.cost_micro_usd`, `usage.provider` and
`X-Hoosh-Cache` are read when present, and an older gateway degrades to an absent cost
rather than a fabricated one.

**Everything below in this section is dated history (2026-08-03 to 2026-08-05)**: the pins it
names (kavach 3.11.7, bote 3.3.0, libro 2.8.4) were current then, not now. The method it
records — checking upstream tags through the GitHub API — still applies.

**kavach 3.11.7 verified end to end, 2026-08-05** — the check `state.md` itself
prescribes, run in all three places rather than assumed:

| bundle | sha256 (16) |
|---|---|
| `git show 3.11.7:dist/kavach.cyr` | `e959d81aa2b370f0` |
| kavach worktree / `origin/main` (`6567a65`) | `e959d81aa2b370f0` |
| `agnosai/cyrius.lock` | `e959d81aa2b370f0` |

Remote tag confirmed at `6567a65` through the GitHub API, so the local `path =
"../kavach"` override and a tag-only CI resolution produce the same bytes —
which is the condition the *Pins must name what is actually built* rule exists
to enforce, satisfied rather than merely intended.

`kavach_bridge` requires 3.11.7: `exec_result_set_stdout_n` does not exist
before it, so an older pin does not compile rather than silently under-scanning
— see roadmap C2.

**All six pins were then the newest upstream tag** (2026-08-05), confirmed against the GitHub API
rather than a local `git fetch` — the dep remotes are SSH (`git@github.com:`) and a
keyless fetch fails *silently enough to look like "no new tags"*. Use
`curl -sf https://api.github.com/repos/MacCracken/<dep>/tags` to check.

> **bote 3.3.0 is pinned as of 2026-08-03** — it adds
> `dispatcher_set_server_info`, which removes the `"serverInfo":{"name":"bote"}`
> hardcode that stopped any consumer from using bote's MCP dispatcher without
> misidentifying itself. This unblocks roadmap B1's *next* surface; it does **not**
> mean `routes/mcp.cyr` should delegate to the dispatcher today, because the oracle
> uses bote's protocol types and explicitly declines its Dispatcher
> (`rust-old/src/server/routes/mcp.rs:3-5`), so hand-building the envelope IS the
> parity behaviour.

libro 2.8.4 arrived transitively via bote then (2.10.6 at 2.1.5).

## Known issues in the current build

0. **⚠ A private key was committed at the repo root — `probe_key_tmp.pem`.
   Deleted from the tree 2026-08-10; STILL IN HISTORY.**

   1,704 bytes, `-----BEGIN PRIVATE KEY-----`, tracked, committed in
   `bb76e67 "errors and jwt work"`, referenced by nothing — no `.cyr`, `.tcyr`,
   `.bcyr`, `.sh`, `.yml` or doc.

   **Verified unneeded before removal**, three ways: it is not the oracle's
   fixture (sha256 `36c3a7e2…` vs `a5cdfea6…` for
   `rust-old/tests/fixtures/test_rsa_private.pem`); its public half does not
   match the frozen vectors agnosai verifies against (`…AQEAlv/hFeMqWBO6…` vs
   `…AQEA5Wu/jjUwgB2e1/Bn…` in `tests/server_auth_vectors.cyr`); and both auth
   suites pass without it — **133 + 9 assertions, 0 failures**.

   ⚠ **Deleting the file does not remove the key** — it stays reachable from
   `bb76e67` for anyone who clones. **Maintainer decision 2026-08-10: no history
   rewrite. Accepted as is.** The key was generated locally for a probe and was
   never a credential for anything, so the cost of rewriting `main` outweighs it.
   This is recorded so nobody re-opens it as a finding: it is a known, accepted
   artefact of history, not an oversight. Nothing in the port needs a private
   key at all: agnosai never signs, it only verifies, and the one keypair any
   test needs is baked into `tests/server_auth_vectors.cyr` as frozen RS256
   vectors precisely so no key file has to exist.

   `.gitignore` now carries `*.pem` / `*.key` with that reasoning, since agnosai
   has **no** legitimate PEM in the tree — so a future one is always debris.

   Also noticed alongside it, both cosmetic and both non-`src`-surface work:
   README's Quick Start is still Rust-era (`cargo build`,
   `cargo run --bin agnosai-server`, `make check`) against a tree with no root
   `Cargo.toml`, and `.gitignore` is otherwise the Rust-era file — it still
   ignores `/target`, `**/*.rs.bk`, `criterion/`, `*.profraw` and
   `tarpaulin-report.*`.

1. **35 duplicate-fn warnings at build, none from `src/` — all benign today, but know the rule.**
   ⚠ **The stated rule here was WRONG and is corrected as of 2026-08-10.** It said
   "a redefinition only rebinds call sites parsed *after* it. A dep's internal
   calls therefore keep binding to its own definition." **They do not.** Measured:

   ```cyr
   fn dup_fn(): i64 { return 11; }
   fn early_caller(): i64 { return dup_fn(); }   # parsed BEFORE the redefinition
   fn dup_fn(): i64 { return 22; }
   ```

   `early_caller()` returns **22**. Every call site binds to the last body,
   wherever it was parsed. The conclusion this entry reached — that majra's
   `pubsub_subscribe` is fine alongside libro — is **still correct**, re-verified
   by probing the subscriber struct (`sub + 8 == 0` ⇒ majra's body ran). But it
   was correct for the wrong reason, and the wrong reason would license ignoring
   every future duplicate.

   **What this does mean for us:** agnosai's own modules are included last, so
   *our* calls bind to the last definition of everything. When we call a name
   that appears in more than one bundle, check which one wins. The current
   duplicates:

   - **33 from kavach** re-exporting symbols sigil also defines, with
     byte-identical bodies (no behavioural difference either way).
   - **`_sub_new`** (majra/libro) — we should never call it, it is a dep-private
     helper.
   - **`path_exists`** — `lib/kavach.cyr:2640` uses `sys_access(path, 0)` (F_OK,
     existence only); `lib/ai-hwaccel.cyr:1385` delegates to `file_exists`, which
     opens `O_RDONLY`. **ai-hwaccel's wins for our code** — it is included after
     kavach — and the two are *not* the same contract: a path that exists but is
     unreadable answers 1 under kavach's and 0 under ai-hwaccel's. Corrected
     2026-07-30; the earlier entry had this backwards on both counts.
   - ~~**`_agnosai_is_digit`**~~ — ✅ **FIXED 2026-07-31.** Hoisted into
     `src/units.cyr`; both local copies removed. **agnosai's own duplicate-fn
     count is now zero** — all 35 remaining warnings are lib-vs-lib.

   **The compiler only warns for `fn`.** A duplicate `var` or enum member is
   **silent**, which a 2026-07-31 audit found the hard way: four duplicated
   top-level constants in `src/`, three with *different* values, three of them
   struct sizes passed straight to `alloc()`. Nothing misbehaved — purely because
   each file's own `alloc()` was parsed after its own definition and before the
   redefinition — but reordering `src/main.cyr`'s includes would have silently
   under-allocated a heap struct. All renamed to module-unique names, and
   `scripts/check-symbols.sh` now gates the whole class in CI ahead of the build.
   **`grep "duplicate fn"` on the build log is not a sufficient check** and
   should not be treated as one.

2. **`cyrius bench` accepts an argument it cannot use and reports success —
   same root cause as the `cyrius build` missing-file bug.** Measured 2026-08-05
   on cyrius 6.5.6: `cyrius bench benches` (a directory), `cyrius bench
   no-such-dir` and `cyrius bench no-such-file.bcyr` each run **zero benchmarks,
   print no diagnostic, and exit 0**. Only the **no-arg** form prints the
   `=== N passed, M failed ===` summary, so a gate written any other way has
   nothing to check and passes over nothing.

   **`bench <file>` builds and runs that file as a program** — it does not match
   the argument against a benchmark registry. `cyrius bench
   tests/sandbox_policy.tcyr` runs the *test suite* and prints `102 passed`. So
   an argument that matched nothing is the already-filed `cyrius build` case
   (stdlib prepended, entry file never opened, a valid empty translation unit)
   with a run appended — the compiler output still shows the `unreachable fns`
   note, i.e. it compiled the stdlib and ran that.

   `bench` is the only command in the family that fails this way. `cyrius tests
   <file>` and `cyrius tests no-such-dir` both exit **1** with `not a
   directory: ...`; bare `fmt` and `lint` exit 1 with usage; `cyrius lint
   no-such.cyr` exits 1 with `cannot read file`. Filed upstream as
   `2026-08-05-cyrius-bench-accepts-an-unusable-argument-and-exits-0.md`,
   cross-triaged with the `build` filing.

   `scripts/bench-history.sh` calls the bare form and is therefore correct.

   **CLAUDE.md's rule about subdirectories is right, and a note here previously
   said it was stale.** No-arg discovery scans `benches/` and `tests/` at their
   **top level only**: a probe `.bcyr` planted at `benches/zzsub/` was not found
   (still `6 passed`, marker absent from the output), and one planted at the
   repo root was not found either. `benches/` is a scanned location, not a
   counter-example to the rule.

   **Unexplained on the same run**: `lt_aggregate_100k_10workers` measured
   **25.3 ms** against the 79.2 ms recorded below, with `sort_100k` unchanged at
   20.5 ms against 20.3 — so it is not the sort, and nothing in the tools path
   was touched. Recorded as an observation, not claimed as a win; the table
   below is left at its published numbers until someone reproduces the gap.

3. **The cleanliness gates were never in CI — only run by hand.** Until
   2026-07-31 `.github/workflows/ci.yml` ran the symbol check, build, and test
   and nothing else: no `fmt`, `lint`, `doc`, `vet`, `deny`, or `coverage`,
   despite CLAUDE.md specifying them at work-loop steps 2 and 6 and calling
   coverage out as "its own CI step". The drift that hid behind it was real —
   **31 undocumented public symbols** across five modules and four untracked
   lint deferrals. `scripts/check-clean.sh` now runs all five, and coverage is
   its own step after Test.

   Two traps it exists to avoid. **`cyrius fmt`, `lint` and `doc` each take a
   FILE**; written bare they print usage and exit 1, so a gate that checks only
   the exit code reads that as a failure and one that ignores it passes over
   zero files. And **`cyrius fmt <file>` without `--check` prints the formatted
   text to stdout rather than rewriting the file** — it is not an in-place
   formatter. The gate is itself mutation-tested: it was confirmed to fail on an
   undocumented symbol, a formatting violation, and an untracked deferral.
   (`cyrius vet` is the dependency-trust gate, not a code checker — an undefined
   symbol is caught by `build`, which refuses to emit.)

## Handoff

> **Refreshed 2026-08-07.** This section is a pointer, not a diary. Everything
> that is *owed* lives in [`roadmap.md`](roadmap.md) → *Owed work*, which is the
> single list; if an item is not there, it is not owed. What
> already shipped and why is in `CHANGELOG.md`. This file carries only the numbers
> and the standing rules.

### Start here

The tree is **green and idle** — nothing is half-done, and no bite is in flight.
Read in this order:

1. `CLAUDE.md` — process, principles, DO NOTs. Non-negotiable.
2. [`cyrius-port-plan.md`](cyrius-port-plan.md) — the plan of record. Its blocker
   table is a **closed reasoning archive**, not a work list; its *Corrections*
   section is the list of things already got wrong once, and re-deriving any of
   them is wasted work.
3. [`roadmap.md`](roadmap.md) → *Owed work* — the only list of what is owed.
4. This file for the numbers.

**Then run the gate before changing anything**, so a later failure is
attributable:

```sh
cyrius build src/main.cyr build/agnosai && cyrius test tests && \
  cyrius coverage --min 80 && ./scripts/check-clean.sh && ./scripts/check-symbols.sh
```

Expected as of 2026-08-09: build clean, **79 suites / 6,389 assertions / 0
failed**, coverage **100% (1338/1338)** over 87 referenced files, cleanliness
clean (fmt 174 · lint 95 · doc 95 · vet+deny · deps 113 verified · lib snapshot
107), **2,189 symbols across 95 files**.

`cyrius test tests` takes several minutes; for a single suite use
`cyrius build tests/<name>.tcyr /tmp/t && /tmp/t`, which is seconds.

⚠ A backgrounded sweep enumerates its suites **once, at launch**. A suite added
while it runs is silently excluded and the pass count still reads green — which
happened on 2026-08-08 (77 of 78, missing `fleet_topology`). Re-enumerate, or
re-run the sweep, after adding a `.tcyr`.

**What a next session would most usefully pick up**, in rough order of value:

> ⚠ **Rewritten 2026-08-07.** The previous version of this table ranked M8/M10
> last as *"past the parity bar rather than debt"*. That framing is retired.
>
> **The oracle is 41,163 lines, not 27,683.** 15,427 were unported when this
> banner was written; **M8 `fleet` closed 4,443 of them on 2026-08-08**, leaving
> **10,984 (26.7%)**. The 27,683 headline counts only `rust-old/src/**/*.rs`. It excludes
> `rust-old/` minus `src/` (11,998 lines — benches, supply-chain, fuzz, tests,
> examples, Makefile), `src/presets/*.json` (810 lines, `include_str!`'d **into
> the binary**), and 672 lines of Rust-era files at this repo's own root
> (`sdk/agnosai-tool-sdk`, README, CONTRIBUTING, Dockerfile).
>
> **A first draft of this banner said "~6,225 lines" — fleet + definitions +
> telemetry only. That undercounts the src figure by 24% and the true figure by
> 2.5x.** Of the 8,206 unported *src* lines it then counted, only 6,225 were in
> the unstarted groups; the rest are holes inside groups this file calls
> complete. With `fleet` closed and `telemetry/` source-complete the *src*
> remainder is **3,441**: M10 `definitions` 1,460, and **1,981 that are holes
> inside groups this file calls complete**. M9 has no oracle *source* left — only
> the OTLP exporter body, which is copied from hoosh rather than from
> `rust-old/`. The one named milestone is now the *smaller* half of the work.
>
> ⚠ **"M0–M8 complete" is false for five of the nine.** M2 (`hwaccel` half of
> `core/resource.rs`), M3 (`hwaccel` half of `llm/router.rs`, all of
> `llm/inference_queue.rs`), M4 (`tools/{python_tool,wasm_tool,wasm_loader}.rs`
> absent), M6 (`routes/definitions.rs`'s populated arm absent; `/metrics` serves
> a different registry than the oracle), M7 (`sandbox/wasm.rs` absent).

| | Why |
|---|---|
| **M10 `definitions`** (1,460 lines) | Whole, including ZIP and YAML. **Declaring `sankoch` in `cyrius.cyml` is the entire ZIP prerequisite** — 26 `zip_*` fns are already in `lib/`. bayan's YAML is already there too. Turns `/api/v1/presets` from `[]` into a real branch. |
| **M9 `telemetry`** (322 lines) | OTLP export (copy hoosh's `otlp.cyr`, 199 lines; thread-local trace context is mandatory under `run_pooled`) plus the JSON-logging + `EnvFilter` divergence, which starts as a sakshi filing — file it early so it lands while the rest is being ported. |
| The `sandbox`-gated tools | `tools/{python_tool,wasm_tool,wasm_loader}.rs`, plus the `hwaccel` half of `core/resource.cyr`. **kavach already has a wasmtime backend with `--fuel`** (`lib/kavach.cyr:9867`), so the WASM pair is portable now. |
| roadmap B2's remaining tail | `crew_runner`'s off-request-path bayan calls — the largest un-threaded allocator surface left. Everything request-reachable is done. |
| ~~D1 / D2~~ | ✅ **All five roadmap decisions (D1–D5) are closed as of 2026-08-13.** Nothing is waiting on a human. Do not re-open them from this table. |

⚠ **Do not** start by re-auditing performance on the request path. It was
decomposed to the floor on 2026-08-07: the remaining cost is `alloc_via` × the
allocation count, the count is what the parity-fixed wire format requires, and
the upstream half already shipped in 6.5.10. The measurements and the two
dead-end levers are recorded under *Benchmarks*.

### Where the port is

| | |
|---|---|
| Cyrius port | **37,662 lines**, 112 files, `src/` mirroring `rust-old/src/` |
| Rust oracle | **27,683 lines** under `rust-old/src/` (41,163 counting the whole frozen tree) — never built, parity reference only |
| Milestones | ⚠ **ALL CLOSED.** M0–M12. The last open item, the wasmtime-gated WASM execute half, closed 2026-08-13 when `tests/tools_wasm.tcyr` began running a real 152-byte WASI module end to end. **All five roadmap decisions D1–D5 are closed too.** |
| Module parity | 98 oracle `.rs` under `src/`; **93 have a same-named `.cyr`**, and the 5 that do not are accounted for: `lib.rs` (crate re-exports, no Cyrius meaning — ONE flat namespace), `server/mod.rs` (→ `src/server/router.cyr`), and `tools/builtin/{echo,json_transform,mod}.rs` (→ `src/tools/builtin/basic.cyr`) |
| Gates | build OK · `check-clean.sh` green · **99 suites / 8,038 assertions / 0 failed** · coverage **99% (1578/1585)** via `cyrius coverage --min 80` · `deps --verify` 113/0 · every `.tcyr`/`.bcyr`/`.fcyr` compiled explicitly · 7,108 benchmark rows across 67 dated runs |
| ⚠ v2.0 criteria | **5 of 6 met.** The unmet one is *"at least one downstream consumer green"* — see the note below. |

⚠ **The one v2.0 criterion still open: downstream consumer verification.**
`CLAUDE.md` lists daimon as a consumer, but daimon has **no coupling to agnosai
at all** — not a `[deps.agnosai]` entry (its deps are sakshi, ai-hwaccel, samay,
sigil, libro, majra, bote), nothing in its `src/`, and no HTTP client pointed at
an agnosai endpoint. It is a sibling in the AGNOS ecosystem, not a downstream.
So this criterion **cannot be satisfied by building daimon**, and no other
consumer (Agnostic, joshua, kiran) has been verified either. Either wire a real
consumer against the HTTP API and prove it green, or waive the criterion
explicitly in the release notes — but do not record it as met.

### What "M8/M9/M10 not started" actually means — corrected 2026-08-07

> ⚠ **This section used to argue that only part of M9 was a real gap**, because
> `rust-old/Cargo.toml` has `default = []` and `fleet`/`definitions` are
> `full`-only. **That framing is retired.** It was self-issued here, never a user
> decision, and it survived four handoffs telling each new session that two of
> the three remaining milestones were "past the bar rather than debt". **A cargo
> feature gate is not a scope boundary.** All three are owed, in full, along with
> the `sandbox`-gated tools, the `hwaccel` half of `core/resource.cyr`,
> `genai.rs`, `inference_queue.rs`, the `#[tokio::test]` suites, `benches/` and
> the non-`src` surface.

| oracle module | lines | status |
|---|---|---|
| `fleet/` | 4,443 | owed in full — the largest unported group |
| `definitions/` | 1,460 | owed in full, **including ZIP and YAML**: `lib/sankoch.cyr` already exports 26 `zip_*` fns (it is merely undeclared in `cyrius.cyml`) and bayan already ships `bayan_yaml_parse`. Both were listed as upstream filings owed; neither was ever real. |
| `telemetry/` | 322 | owed in full — OTLP export **and** the ungated logging init |

The logging half is the one that is also a *default-build* divergence, and it is
not OTLP: the oracle's `#[cfg(not(feature = "otel"))]` arm still runs
`tracing_subscriber::fmt().with_env_filter(..agnosai=info..).json().init()`.
sakshi has neither a JSON output mode nor an env filter, so the port emits plain
text at sakshi's default level — a **stated** divergence documented at
`src/main.cyr:22-26`, where log *content* matches line for line and transport and
format do not. Closing it needs sakshi work, i.e. an upstream ask.

Also verified rather than assumed, because both read like omissions:
`tools/{python_tool,wasm_tool,wasm_loader}.rs` are `#[cfg(feature = "sandbox")]`;
and `tools/builtin/{echo,json_transform}.rs` are **not missing** — they are folded
into `src/tools/builtin/basic.cyr`, which is a layout divergence from the
mirror rule worth knowing before diffing file-against-file.

**The binary serves as of 2026-08-03**, and **drains on SIGINT/SIGTERM** as of
the 6.5.6 bump the same day. `src/main.cyr` wires the state, installs a
`signalfd` handler, and calls `agnosai_serve`. ADR
[013](../adr/013-graceful-shutdown-via-signalfd-and-stop-flag.md) supersedes
[012](../adr/012-no-graceful-shutdown-on-sandhi.md): the blocker was never the
signal helper this file and the roadmap both used to cite — it was that sandhi's
accept loop could not be made to return, which agnosai filed and sandhi 1.9.9
fixed.

### Standing rules a new session should not re-derive

1. **`src/` mirrors `rust-old/src/`.** Anything walking it must recurse —
   `find src -name '*.cyr'`, never `src/*.cyr`, which now matches nothing and
   **fails open**. `cyrius coverage` and `cyrius tests` already recurse.
2. **Verify a toolchain bump by diffing `lib/` against the snapshot, and write
   the lockfile last.** A green `cyrius lib sync --full` is not proof: on the
   bayan repo the same command reported success and left five files stale.
   `cyrius deps` re-layers git deps but does **not** refresh the stdlib — and it
   writes `cyrius.lock` from what it just copied, so a sync afterwards leaves
   `deps --verify` failing. The order that satisfies both gates:

   ```sh
   cyrius deps --no-lock     # resolve git deps into lib/
   cyrius lib sync --full    # refresh the stdlib over them
   cyrius deps --lock        # record what is actually there
   ```

   **A dep's transitive pin can hold a stdlib module back, and `lib sync` is
   not the fix.** `cyrius build` performs an implicit resolve, so a stale module
   flips back on **every build**. Diagnose by hashing — several sources can hold
   the same version and only one is the writer, so version stamps mislead:

   ```sh
   sha256sum lib/<mod>.cyr ../*/lib/<mod>.cyr \
             ~/.cyrius/deps/<mod>/*/dist/<mod>.cyr \
             ~/.cyrius/versions/<pin>/lib/<mod>.cyr
   ```

   `deps --verify` cannot catch this — the lock is written *from disk*, so a
   downgraded file gets its downgraded hash recorded and the check passes.
   `scripts/check-clean.sh`'s snapshot diff is the only gate that sees it, and
   it now allows no exceptions. The reasoning and the wrong first diagnosis are
   in [`cyrius-port-plan.md`](cyrius-port-plan.md) → *Corrections*.

   Also: `cyrius lib sync` reads `~/.cyrius/versions/<the pin>/lib`, so a
   sibling on an older `cyrius = "X.Y.Z"` merely re-syncs its own old snapshot.
   Bump the sibling's pin first, then run the three-step there. All seven
   siblings are on 6.5.10 as of 2026-08-07.
3. **Git-dep pins must move with the fold.** `cyrius deps` copies each dep's
   bundle into `lib/` last-write-wins, so a stale `[deps.sigil] tag` overwrites a
   newer folded copy back down.
4. **Cyrius silently accepts a duplicate parameter name.** Adding an allocator
   parameter named `a` to a fn whose first parameter was already `a` compiles
   clean, and every `load64(a + OFFSET)` then reads the allocator. It SIGSEGVs with
   no assertion output. Check existing parameter names before prepending.
5. **`map_keys` defeats arena threading.** It materialises a key vec through
   `vec_new()` on the no-free global bump and has no `_a` form, so it survives
   every other substitution and silently caps the win. Use `_agnosai_map_slots` /
   `_slot_live` / `_slot_key` / `_slot_val` from `src/core/json.cyr`.
6. **The duplicate-symbol hazard is wider than `fn`.** The compiler warns on a
   duplicate `fn` and is **silent** on a duplicate `var` or enum member.
   `scripts/check-symbols.sh` gates the whole class; `grep "duplicate fn"` on a
   build log is not a sufficient check.
7. **Measure before claiming, and mutation-test the assertion.** Two findings this
   session reversed on measurement (the `to_wire` hoist looked worth 2.5% and was
   not worth 121 new globals; `map_keys` looked negligible and was the entire
   residual), and two bugs were caught only because the pinning assertion was
   mutation-verified.
8. **When an assertion cannot be made to fail, that is usually a statement about
   the code's shape, not the test's.** The M7 audit remediation's recurring
   defect was an *unreachable* control, not an unasserted one — an env filter
   that reads `/proc/self/environ` cannot be tested by a process with no
   `setenv`. Five fixes worked by splitting the function so it takes its input
   as a parameter (`_agnosai_sanitized_envp_of`, `_agnosai_process_envp_from`,
   `_agnosai_cx_resolve_interpreter`, `_agnosai_cx_budget_ms`,
   `_agnosai_sandbox_manager_timeout_ms`); two more by **re-execing the suite
   through `agnosai_spawn_capture_input` with a planted `envp`**, which is the
   only way to put a variable in this process's own environment. Reach for one
   of those before concluding a behaviour is unobservable.
9. **`callptr` does not check arity, so changing a function-pointer signature is
   only half-caught by the compiler.** The tool vtable's `schema_fp` went from
   `fn(ctx)` to `fn(a, ctx)`. Implementors still declaring the one-parameter
   shape **compiled and passed** — `ctx` silently received the allocator and the
   real ctx was dropped, which is invisible in a schema that ignores `ctx`. Two
   test tools in `tools_native.tcyr` were in exactly that state and the suite
   reported 71/71.

   The *detectable* half is a direct call: `_agnosai_delegate_schema(0)` in
   `tools_builtin_delegate.tcyr` then passed 0 as the **allocator**, and
   `alloc_via` followed it — a segfault, so a failed suite with no `FAIL:` line
   (see the crash note in the harness rules).

   **Neither showed up in per-suite verification.** The change was mutation-
   tested against `server_serve` and `tools_native`, both green; only
   `cyrius tests tests` across the whole tree caught it. When a signature that
   is reached through `callptr` changes, grep for every implementor and every
   direct caller by name — the build will not.

10. **A tool that prints a number can still be lying, and the fix is upstream.**
   `cyrius coverage` under-reported for a fortnight because this tree's `.tcyr`
   corpus outgrew a fixed 1 MiB buffer — silently, at exit 0, naming
   fully-covered files as uncovered. It was worth a local reimplementation only
   until upstream shipped (6.5.8), and that reimplementation was then deleted.
   The durable half: when a gate's number moves and nothing in the diff explains
   it, **suspect the gate before the code** — and confirm by computing the same
   thing a second way rather than by reasoning about it.

11. **Verify the mutation is a faithful revert.** One written during the M7
   remediation changed half of a two-line fix, left the tree in a state neither
   version would produce, and "killed" the mutant for the wrong reason. Read
   what the mutated file actually says before believing the failure.

   **The inverse failure happened on 2026-08-08 and cost two rounds.** The
   mutation for `tests/server_auth_lane_race.tcyr` reverted the *call site* in
   `_agnosai_auth_validate_jwt`, while the suite calls
   `_agnosai_auth_rsa_verify_locked` **directly** — so the test never saw the
   change and reported 9/9 against a knowingly-broken build. **Mutate what the
   test actually calls**, not what the production path calls, and if a mutant
   survives, suspect the mutation's reach before rewriting the test. (A real
   test weakness was also found the same way: 2 threads × 200 iterations with no
   start barrier left the unlocked build fully clean, because the workers barely
   overlapped. The suite now uses a barrier and 2,000 iterations.)

12. ⚠ **A `return f(&local_or_pointer_into_this_frame, ...)` with EXACTLY SIX
   arguments is a tail call, and cycc frees the frame before jumping.** The
   callee then reads a released frame. This cost sigil three releases and cost
   this tree a wrong diagnosis, so it is worth knowing cold:

   - cycc declines TCO **above** six arguments, so the same code localised
     cleanly in a 10-argument function and "broke the tests" in a 6-argument
     one. **The arity was the variable, not the buffer.**
   - cycc's own guard fires only on a `&` spelled literally in the argument
     list, so `var p = &buf; ... return f(p)` walks straight past it.
   - Fixed in **cyrius 6.5.14**. Below that pin, the hazard is live.

   **The wrong turn, recorded because it was persuasive.** Localising sigil's
   PSS and sign buffers regressed tests; three sigil releases (and a note in
   this tree) recorded "cause not understood" and kept the shared lanes — and
   those lanes *were* an authentication bypass. This tree's probe that
   "ruled out callee-clobbers-caller" was **not a faithful reproduction**: it
   called the callee and then did more work, which is not a tail call, so TCO
   never applied and the probe returned a false negative. The probe that
   settled it compares two addresses — a callee local sitting *above* every
   caller local only happens if the caller's frame is already gone.

   **When a buffer "cannot" be localised, count the callee's arguments first.**

13. **sigil's crypto scratch is shared across threads, and the JWT path is the
   only place agnosai is protected.** `cbank()` gives 63 lanes and never
   releases one, so the bound is 63 *lifetime* crypto-touching threads — a
   count agnosai passes with its 100 pool workers alone. Both operands of
   `_rsa_pkcs1v15_check`'s final `ct_eq_bytes` live in that shared lane, which
   is an authentication bypass, not merely a spurious-401 race: **888 of 400,000
   forged signatures were accepted** before the fix. `src/server/auth.cyr`'s
   `_agnosai_auth_rsa_verify_locked` serialises the verify; **do not remove that
   mutex to reclaim throughput.** ⚠ 62 file-scope banked globals remain,
   including the shared bignum engine and the TLS 1.3 peer-auth lanes every
   outbound HTTPS call uses — those are **not** covered by agnosai's lock. Full
   analysis in `CHANGELOG.md` under *Security*; the upstream ask is in
   `roadmap.md` → C.

   ⚠ **Do not re-test this by pinning every thread to one lane.** Max contention
   shows zero false accepts and near-total false rejects, which reads as
   fail-closed and is wrong — the bypass lives in the low-multiplicity regime
   that 100 threads over 63 lanes actually produce.

### Consumers

agnostic consumes the Cyrius line (`[deps.agnosai]`, tag 2.1.7, `dist/agnosai.cyr`), and thoth
consumes the `[lib.guard]` profile (`dist/agnosai-guard.cyr`; its `src/guard.cyr` calls
`agnosai_prompt_scan_input`). joshua (NPC AI) and kiran (game AI) are planned; there is no checkout
here to check. daimon has no agnosai dependency (checked at 2.0.0, 2.1.6 and 2.2.2), so it is no
longer listed (roadmap B23).
