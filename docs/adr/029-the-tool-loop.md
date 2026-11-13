# 029 — The tool loop: the model proposes, and agnosai disposes

**Status**: Accepted
**Date**: 2026-11-06

## Context

An agent was one prompt. `execute_task` made one chat completion, plus a retry for an output schema, and nothing
in `crew_runner` or `src/llm/` handled a tool call, so a registered tool was never invoked by a model. agnostic's
M6 is gated on this: 36 tools written against a loop that does not exist run nothing. The Rust line never had one,
so this diverges from the wire the oracle defined, and design principle 1 asks for this ADR.

agnostic and a comparison with turnstone hold the loop to four rules, recorded 2026-10-07:

1. Every call is journaled before it runs, and each result says what it did: committed, none or unknown.
2. A tool's output is data, never instructions. Each result is checked before it reaches the model, and none may
   change the plan, the agent's grants or its budget.
3. The calls a model makes in one turn are approved as a set, as "read the secret" and "post to the web" each pass
   alone and leak together.
4. Each agent sees only the tools granted to it, and the registry can hold tools from external MCP servers.

[ADR 025](025-a-tool-call-passes-a-consumers-gate.md) built the chokepoint this stands on, and left it two things:
call `agnosai_tool_execute_for` with the task, and decide what rule 2 does with a flagged result.

## Decision

**An agent is offered the tools it names, the model calls them until it answers, and everything between one answer
and the next request is one pipeline in `src/tool_loop.cyr`.** `crew_runner` sends the request and reads the answer.
Nothing a model or a tool writes decides anything in between.

### The seam

`agnosai_orchestrator_with_tools(o, registry)` hands an orchestrator a registry, which stays the caller's. This is
option 1 of agnostic's `handoff.md` §8: it stands up its own, registers its QA tools, and may serve the same one on
`/mcp`. With no registry a crew runs as it always did, and an agent's `tools` are words in its prompt, so nothing
changes for a library consumer until it opts in. The binary hands over the registry `/mcp` serves.

### Rule 4: the grant is the list the agent names

An agent is offered exactly what its `tools` names, in that order, each once. An agent that names none is offered
none, and a task with no agent likewise, so a crew that never used a tool sends the bytes it always sent, and hoosh's
cache keys on those bytes. **This is not `agnosai_is_tool_allowed`**, where an empty list means every tool: applied
here, that would turn every existing crew into a tool-calling one.

The grant is resolved once, before the first call, into a list held by the task. A call to anything else is refused
with `effect: none`, though the registry may hold it, and no later text can add to it.

> **2.4.3 note** ([ADR 035](035-a-manager-assigns-its-workers-and-writes-the-crews-answer.md)): a hierarchical crew's
> worker names only the tools its manager holds too, so its grant is that. A call's input also names its caller, the
> agent and its task's budget tracker (`agnosai_tool_input_agent`, `agnosai_tool_input_budget`), so a tool that hands
> work on, `delegate`, gives it no more than the caller holds.

⚠ **A name the registry does not hold fails the task, before the first call, naming the tool.** Going on would be a
smaller agent than the crew asked for, told by its own system prompt that it holds a tool it cannot call, and a model
in that spot writes down what the tool would have said. In a QA report that is a fabricated finding. A name no
provider takes (OpenAI's `^[a-zA-Z0-9_-]{1,64}$`) fails the same way, rather than failing every request later.

External MCP tools need nothing here: each is an `agnosai_tool_new` over bote's client, once bote has one.

### A turn

An answer that asks for tools takes one turn:

| Step | What happens |
|---|---|
| **prepare** | Each call against the grant, its arguments parsed. One that cannot run is refused here and still answered |
| **approve** | The calls that are left, as a set, by each gate that holds one of them, once each |
| **journal** | Every call about to run, before any of them does |
| **run** | At once, with one on the task's thread, so a task making one call spawns nothing |
| **deliver** | Each result checked and appended, in the order the model asked, and the record kept |

Every call a provider saw gets a reply, or it refuses the transcript. **What goes back is rebuilt, never echoed.** hoosh
turns the arguments Str into an Anthropic `input` object on the way out, and keeps a `\\n` in it, so a malformed call,
or a well-formed one with a line break between its keys, would have the provider refuse every later turn. Arguments
that parse go back compact, those that do not go back as `{}`, and a name the agent does not hold goes back as
`unknown_tool`. That name, not the model's, is what the journal and the audit chain see, as they are kept for good. The
task's record has it as `asked_for`, filtered, when it is one a provider could take.

An id is unique across the task, not the turn. A reply only has to match its own turn, but hoosh restarts at `call_0` on
each response from Ollama and Gemini, so a journal keying on the id would take a later turn's intent as settled by an
earlier result, which is the failure rule 1 exists to prevent. A repeat is minted, as is an empty id, one outside a
provider's charset, and, with redaction on, one the filter would touch.

⚠ **A tool's input lasts as long as its call.** It lives in the conversation's arena, which goes back with the task. A
tool that keeps anything from it past `execute_fp`'s return, for a crew it submits or a thread it leaves running, copies
it first. `delegate` is the one built-in that does. A tool's output is always copied, into the model's text and the
record, before the turn's arenas go back.

### Rule 1: journaled before it runs

`agnosai_tool_journal_new(record_fp, ctx)` is a consumer's journal, and `record_fp(ctx, phase, call)` is called at
INTENT and at RESULT. **It is a write-ahead log, so a call it cannot take does not run:** answering a reason at INTENT
refuses that call, with `effect: none`. Every intent in a turn is written before anything in it runs, so a crash
leaves an intent with no result, which reads as an unknown effect. A call refused before it could run has a RESULT
and no INTENT.

The crew's audit chain, when it has one, gets `tool_call_started`, `tool_call_finished` and `tool_call_refused` too.
Neither is durable by itself: the chain is in memory, by design (`orch_durable_state` does not snapshot one), so what
survives a crash is what a consumer's journal put on disk before returning. agnosai's own durable log is F4. The
chain holds the tool's name and the effect and never the arguments, as it is kept for good.

> **2.5.0 note** ([ADR 036](036-a-crew-keeps-a-log-and-resumes-from-it.md)): a crew with a log journals each call in it,
> synced, before any consumer's journal is asked, and a call whose intent the log cannot take does not run. A resume
> reads an intent with no result, or a result that committed or cannot say, as a call whose effect may stand, and asks
> a reviewer before its task runs again.

The effect is ADR 025's: `none`, `committed` or `unknown`. turnstone's `partial` and `rolled_back` are not adopted,
as no tool here can report them.

### Rule 2: a tool's output is data

**A flagged result is annotated, and is not held back by default.** agnosai's injection table is a heuristic: it is
byte-oriented, a zero-width space defeats it, and `you are a` fires on a great deal of honest text. Held back on that
evidence, a system under test could blind its own QA with one phrase on every page, and an attacker who knows the
table would still get through. The defence is structural, and the scan is a signal:

- **A result reaches nothing but the model.** No branch reads it, the grant was fixed before it existed, and the
  limits are the process's. A model that obeys a page can only call what it was granted, and each of those calls
  still meets the gate.
- **A result goes in the markers the system prompt says never to obey**, `<user_input field="tool_result">`.
- ⚠ **`user_input` and `system_instructions` are rewritten inside it** (`_` becomes `-`), in any case. Unlike
  `agnosai_prompt_sanitize_a`, whose input is the crew author's, this text is the system under test's, and a page
  holding `</user_input>` would end the markers and read as ours. Removing the word rather than escaping `<` leaves no
  `< /user_input >` for a model to read generously. This closes the tag, not a model's imagination: it is byte-oriented
  like the scan, so `</user_<ZWSP>input>` gets through, and `</user-input>` is what it writes. Neither ends the markers,
  and a model may still take one for the tag, which is why the first point is the one that holds. A failed answer going
  back on a validation retry is unmarked too, as a model repeating a page repeats its marker into a user turn.
- **A finding is said outside the markers**, in our voice, and goes on the record as `flagged`, with a WARN line.

A consumer who wants more has `agnosai_tool_gate_with_result`: `result_fp(ctx, input, out, finding)` answers the text
the model reads instead, and the record marks it `held_back`. It is asked only about a call that ran. Under either, the
record keeps the whole result, so a measurement survives a model that may not read it. ADR 020's redaction, when on, covers what the model reads as well
as the record, and only strings, so a measured number stays the number it was.

### Rule 3: a turn's calls are approved as a set

`agnosai_tool_gate_with_batch` adds `batch_fp(ctx, calls)` to ADR 025's gate. It sees every call that could run, before
any of them does, and its reason refuses all of them. That is deliberate: an approval covers a whole set or none of
it, and the model is not told which half was objected to. Gates are read from the tools, so a path cannot forget one,
and an approved call still meets `call_fp`.

### Limits

A task stops, failing with a reason of its own, at **16 turns**, on a **cancel**, or at the crew's **deadline**, each
checked before a turn starts and after it ends, so a stop never leaves a call half-run or unanswered. A deadline
raises the runner's flag, and the crew ends as any timed-out crew does. A turn runs at most **8 calls**. Past that they
are dropped, never echoed, counted in `metadata.tool_calls_dropped`, and the model is told in a turn of our own: 16 ×
(1 + 8) keeps a task inside hoosh's 256 messages. A result is cut at **32 KiB**, on a character boundary, and a task
reads **256 KiB** in all, shared evenly across a turn, and after that it is told what it may not read and can still
answer. What the model wrote counts against that too, as it goes back on every request.

⚠ **The budget holds only because a result is cleaned first.** bayan writes a control byte as `\u00XX`, six bytes for
one, and writes a stray 0xFF as it finds it, which no provider takes as JSON. Either would fail every later request of
the task, as each one carries the transcript, so one binary page could cost it. Control bytes (but tab, CR and LF), DEL
and anything that is not UTF-8 become `?`, in what the model reads. That leaves JSON at twice the text at worst, so 256
KiB fits hoosh's 1 MiB beside the prompts. `agnosai_tool_loop_set_limits` sets all four, process-wide, and refuses what
would pass 240 messages or 400 KiB. Since 2.4.1 a crew may tighten each (`agnosai_crew_with_tool_limits`, [ADR 033](033-budgets-caps-and-crew-concurrency-are-enforced.md)). A model that writes more than the budget by itself ends on
hoosh's 413, as a named failure.

### A failed schema keeps the transcript

A retry rebuilds its prompt from the original (2.2.5) so that it cannot grow. A task that has called a tool must not:
rebuilt, the model would lose what its tools said and run them again, with whatever effect that has. So the retry
prompt goes on the end of the transcript. Each adds one failed answer, so it grows by at most
`AGNOSAI_MAX_VALIDATION_RETRIES` of them and never doubles, which is what the rule was for. A task that was offered tools
and called none rebuilds as ever, and is offered them again.

### What it costs

A conversation lives in an arena of its own from a pool built on first use, and what outlives it is copied out: the
final response and the records. Copied, not borrowed, as a tool may answer with its own input. So a crew on the
global heap does not keep its transcript. **A call through the loop leaves 0 B on the global heap**
(`tests/orch_tool_loop.tcyr`), for one call, for two at once and for a refusal.

**So does a tool that answers in `agnosai_tool_input_alloc`** (2.3.2). `execute_fp` is unchanged: the allocator is on the
input, as the task is, so a tool that ignores it works as it did and keeps what it built on the global heap. Each call
that runs takes **an arena of its own** from the same pool, on the turn's thread before any worker starts, and every one
goes back after delivery, the one place the model and the record have each taken their copy. Not `ra`: a turn's calls
run at once and an arena has no lock, so sharing it hands two calls one block. A pool acquire never blocks, and with no
arena to be had a call answers on the global heap, as it would off the loop. Giving back also zeroes the call's output,
so a journal that kept the call reads 0, and not what the next turn wrote there.

A tool on the network is not at 0, and what is left is not the tool's: sandhi leaves 408–480 B an exchange (waiting on sandhi), the SSRF
guard parses a URL on the global heap, 136 B a check, and `load_testing` joins its threads, 4,120 B a user (roadmap 2.3.6).

What is read of a result is rendered to the cap and no further, and redaction reads 256 bytes past the cut, so it sees a
key the cut would halve. `bayan_json_v_build_a` renders all of it, and an arena keeps its chunks: a 4 MiB object left this
pool at 170 MB, for good, to hand a model 32 KiB. The record copies the whole result, into the crew's allocator.

### Tracing (roadmap B7, items 8 and 9)

`execute_tool` nests under the agent that made the call, with `gen_ai.tool.call.id`, the agent's name and the task's
id. `gen_ai.tool.call.arguments` and `.result` are Opt-In in the conventions, as either can hold what a tool was
pointed at, so they need `AGNOSAI_OTEL_CAPTURE_TOOL_CONTENT=1`, are filtered by ADR 020, and are cut at 4 KiB.

A tool that starts more work reads `agnosai_tool_input_span_ctx(input)`. `delegate` puts it on the crew it submits and
marks the crew delegated, so the crew joins the call's trace, and with a parent it reports no `invoke_workflow` span
of its own, as the conventions say a tool delegating to a sub-agent SHOULD NOT. Its `invoke_agent` spans go straight
under the tool's. A crew outliving its parent span is fine: only the id is handed on.

## Consequences

- **Positive**: a registered tool runs. A crew that names none is byte for byte what it was, and so is one on an
  orchestrator with no registry, so a library consumer changes nothing until it opts in.
- **Positive**: rule 2's structural half needs no heuristic to hold. A test has a model obey a hostile page, and the
  call it asks for is refused.
- ⚠ **Negative, a behaviour change in the binary**: a crew on `POST /api/v1/crews` that names a tool the server does
  not hold now fails, where the model used to answer and the name was only words. The fix is on the caller's side,
  and the error names the tool. A crew naming `echo`, `json_transform`, `load_testing` or `security_audit` now runs it.
- **Neutral, a wire addition**: an entry in `POST /api/v1/crews`' `results` gains `tool_calls`, when the task called any.
  The oracle's `TaskResultResponse` carries no `metadata`, so the binary's caller could not otherwise tell what a tool
  measured from what a model said it measured. A task that called none reads exactly as it did.
- ⚠ **Negative**: every agent in the three example presets (ADR 027) names tools no binary holds, such as
  `vulnerability_scanning`, so posting one unchanged now fails. The binary lists presets and never runs one, and they were
  always a vocabulary a consumer resolves itself. Their names stay.
- **Negative**: a model now reaches the tool tiers, so the process, Python and OCI tiers' lack of confinement (B35)
  matters more than it did, and any caller who may post a crew can grant one. They could already call `/mcp`, under
  the same auth, so this widens what a model can do and not what a caller can.
- **Negative**: the default lets a flagged result through. A model can be misled by what it reads. It cannot be
  granted anything by it. A consumer who disagrees has `result_fp`.
- **Negative**: a watcher sees nothing of a task's turns: no event carries a tool call. F3 writes the lifecycle down,
  and they belong in it, with its `seq`.
- **Negative**: a delegated crew's agents hold what their own definitions name, whatever the delegating agent holds.
  F2's "authority only narrows" closes that, and until then `delegate` stays opt-in, as it is.
- **Negative**: a crew on the global heap keeps its records for good, which is B38's to fix.
- **Neutral**: the pool settles at peak concurrency × the largest conversation, which is the budget plus what the model
  wrote. `max_tokens` bounds that at about 16 KiB a call in practice, and sandhi's 256 KiB response cap at worst.
- **Neutral**: a tool may run on a thread of its own now. It already ran on many at once on `/mcp`, so this asks
  nothing new, and a gate's callbacks already say so.

## Alternatives considered

- **`agnosai_is_tool_allowed` as the grant.** An empty list means every tool there, which is the oracle's rule for an
  allow-list and the wrong one for an offer. It would make every crew a tool-calling one, with every tool.
- **A miss logs and offers the rest.** Keeps every crew running, and is what agnostic's audit calls the defect: a miss
  that silently produces a smaller agent. Here the system prompt would still claim the tool.
- **Hold back a flagged result by default.** Rejected above: it trades a misled model for a blind one, on a heuristic.
- **A result's text through `agnosai_prompt_sanitize_a` unchanged.** It alters nothing inside the markers, so a page
  ends them.
- **Run a turn's calls one at a time.** Simpler, and a model that asks for two calls at once would wait for their sum.
  A tool can take 300 s. A set is approved whole, so nothing is gained by staggering it.
- **The audit chain as the journal.** One sink, and in memory, so the rule's reader after a crash finds nothing.
- **Retry a failed schema from the original, tools and all.** The model would have to run its tools again.
- **Stop at the step limit with one last call offering no tools.** Spends a call to get an answer the model may not
  stand behind, and reads as a success. A task that did not finish says so.
