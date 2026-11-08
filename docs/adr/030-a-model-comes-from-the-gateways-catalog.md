# 030 — A model comes from the gateway's catalog, not from a name agnosai carries

**Status**: Accepted
**Date**: 2026-11-06

## Context

hoosh routes by the id it is given. An agent with no `llm_model` was sent `llama3:70b` or `llama3:405b`, names the Rust
line carried, so a gateway with no `llama*` route answered 404 and one with an Ollama route sent the task to a 70B model
whether the box had one or not. Every agnostic crew takes that path, and M6 waits on it.

agnosai named models in three more places. With no `usage.provider`, nine name prefixes attributed the response, so
`o4-mini`, `gpt-oss` and `qwen/…` billed to Ollama. `BUDGET_MODELS` priced eight models that no longer exist, for a
function nothing calls. And `_agnosai_crew_strip_provider_prefix_a` rewrote what an agent named: Groq serves
`openai/gpt-oss-120b` and OpenRouter's ids are `anthropic/…`, so an id picked from a catalog could reach hoosh renamed.

hoosh has answered this since 2.8.0 (its ADR 012). `GET /v1/models/catalog` lists what each provider serves now, each
entry with `owned_by` and, when known, `capabilities` and `pricing`. Older gateways have `/v1/models`, with `id` and
`owned_by`. `POST /v1/cost/recommend` ranks routes' exact patterns, not the catalog, so it cannot answer this.

Wire compatibility is design principle 1, and three things here diverge from the Rust line, so they are decided here:
a guessed id is gone, `provider` can be absent from a result, and `agnosai_select_cheapest_model` is removed.

## Decision

**An id agnosai sends is one the agent named, one the operator named, or one the gateway listed. There is no fourth
kind.** `src/model_catalog.cyr` reads the catalog and answers from it.

### The read

Once per process per gateway, on first need, into storage that outlives every crew, and again when a selection fails.
`/v1/models` is tried only on a 404: any other failure is a gateway that has a catalog and is not giving it, and an older
list in its place would quietly lose the prices. A read that fails keeps what was held. Readers take no lock, and
publication is `atomic_fence` then the store: `atomic_store` is a plain store, and on aarch64, which ships, a reader could
otherwise see the pointer before what it points at.

⚠ **A catalog is kept only if its bytes are new.** The heap has no free, a reader may hold the old one, and a re-read is
driven by failures, which nobody can bound. So what is retained is one catalog per change at hoosh, and a re-read that
finds the same bytes keeps nothing. On a real socket one leaves under 1 KiB, which is sandhi's own (roadmap B39).

⚠ **A re-read is rationed, for the gateway's sake.** hoosh's heap never frees either, and each `GET /v1/models/catalog`
grew it by 15.4 KB at 9 entries (hoosh 2.10.1, 3,000 GETs), so about 340 KB at 200. One per refused task would let anyone who
can submit a crew grow the gateway without bound by naming models. So a re-read asks at most once a minute
(`agnosai_model_catalog_set_reread_ms`), and asks nothing at all for an id no list could change. A task inside the interval
still selects again from what the last read found, which is how the rest of a crew follows the first task that looked. A
selection with nothing to pick is not rationed: that is a gateway coming back, and it costs hoosh next to nothing.

⚠ **A thread that waited takes the read it waited for.** A crew's first tasks arrive together, and without this each
would read in turn, so a dead gateway costs a crew its width times the connect timeout.

### The tier

The catalog has no tier field, so a tier has to be read from what it has. **Every clause here is a wrong answer on a real
price list first.** hoosh's table, the current Claude lineup:

| Model | $ per M in / out | Declares |
|---|---|---|
| `claude-opus-4-1` | 15 / 75 | `thinking`, an empty `effort` |
| `claude-fable-5-1` | 10 / 50 | `thinking`, `effort` |
| `claude-opus-5-5` | 4 / 20 | `thinking`, `effort` |
| `claude-sonnet-4-6` | 3 / 15 | `thinking`, `effort` |
| `claude-sonnet-5-5` | 2 / 10 | `thinking`, `effort` |
| `claude-haiku-4-5` | 1 / 5 | `thinking`, an empty `effort` |
| `claude-haiku-5-5` | 0.1 / 0.5 | `thinking`, an empty `effort` |

1. **The ladder is the paid chat models.** A price of 0 says local, not cheap and good: hoosh writes 0 for every local
   route, so with free models on the ladder a gateway with Ollama beside Anthropic answers every tier with whatever the
   box has pulled. An unpriced model is a spend nobody can state. When nothing is paid, every chat model is on the ladder,
   which is what a box of local models needs.
2. **`effort` outranks `thinking`, and thinking outranks undeclared.** Premium asks for the most the ladder declares.
   By price alone Premium is `claude-opus-4-1`, an older model at one and a half times Fable's price. Only models with
   tunable reasoning list `effort`, so it separates the two where price cannot. Capable asks for reasoning, when anything
   declares it. Fast asks for nothing. Undeclared means nobody said, not that a model cannot, which is why a catalog with
   no capabilities still has a ladder.
3. **Each tier names a price, and the first listed at that price wins.** Fast is the lowest and Premium the highest.
   Capable is the lower median of the *distinct* prices: a median over entries is dragged toward whichever model a
   provider lists the most dated aliases of. Rate is input plus output, in the catalog's own unit. hoosh lists the
   operator's `[[models]]` first and then routes in router order, so first listed is a preference someone expressed, and
   the only one a list of one price has. Taken over a tie by position, the middle or the end of a list of local models
   would be an accident of its order.
4. **Capable never costs more than Premium.** Their candidates differ, so nothing else says so: beside `claude-sonnet-5-5`,
   a `claude-sonnet-4-5` and a `claude-opus-4-1` that only think leave Premium at $2/$10 and the median of what thinks at
   $3/$15. On the table above this also takes `claude-opus-4-1` out of Capable's median, where it pushed the pick to
   `claude-sonnet-4-6`, an older model at a higher price.

On the table above: Fast is `claude-haiku-5-5`, Capable `claude-sonnet-5-5`, Premium `claude-fable-5-1`. Run through a real
hoosh, a real crew and a stub provider, `medium` reached the provider as `claude-sonnet-5-5` and `high` as
`claude-fable-5-1`, past the dearer `claude-opus-4-1`.

⚠ **This is a heuristic, and these are its known wrong answers.** The override is the answer to each, and hoosh could end
them (see *Consequences*):

- **It cannot see tools.** The catalog has no such bit. hoosh's table says `gpt-6-astra` and `gpt-6.1-sol` take no function
  calling on Chat Completions, so on an OpenAI-only list a `complexity: high` agent with tools gets `gpt-6-astra`, and a
  provider's 400, which no re-selection answers. This is the sharpest of the four for agnostic, whose agents hold tools.
- **What is declared beats what nobody said.** Only Anthropic declares `effort` today, so beside OpenAI, Capable and Premium
  are Anthropic's. With an Anthropic route limited to Haiku, Premium is a Haiku and `gpt-6-astra` is never picked.
- **A catalog of prices alone** gets the priciest model as Premium, which for OpenAI can be an old one.
- **The twelve words below are not all of them.** On a box of local models the first listed is every tier, so a tag like
  `bge-m3` listed first is sent to chat, and refused with a 400.

### What is not a chat model

**One list of names stays**, twelve words: `embed`, `whisper`, `tts`, `transcribe`, `dall-e`, `image`, `moderation`,
`realtime`, `audio`, `davinci`, `babbage`, `sora`. They are hoosh's own (`catalog_openai_nonchat`), and hoosh applies them
to OpenAI-compatible lists only, so Ollama's tags and the compiled table reach the catalog with `nomic-embed-text` and
`whisper-1` on them, and an embedder is the cheapest thing on any list. This is not a way of choosing models: a word here
can only keep an id from standing for a tier. It is still listed, and an agent can still name it.

### The operator's word

`AGNOSAI_MODEL_FAST`, `AGNOSAI_MODEL_CAPABLE` and `AGNOSAI_MODEL_PREMIUM`, or `agnosai_model_set_tier_override`, are sent
exactly as written. They are not checked against the catalog and need none, which makes them the way to run on a gateway
with no list at all. Precedence is the agent's `llm_model`, then the override, then the catalog.

### With no catalog

**The task fails, before any request, with an error that starts `no model catalog: `**, says why, and names the variable
of the tier that wanted a model. Sending a guess is what this replaced. The crew ends failed, as for any failed task. The
text has a status or a transport kind and never the URL, as it becomes a task's `output`. An agent that names its model
does not need a catalog and runs.

### A selection the gateway refuses

A held catalog goes stale, as hoosh refreshes every six hours and a provider retires an id between two of them. A refused
id reads the list again, as rationed above, and the task selects again, once. The same id coming back is the true failure,
and it stands. An id no list could change goes no further: one the strip left alone, or one the operator set.

⚠ **A refusal has two spellings, and the common one is a 502.** hoosh answers 404 only when no route matches. A provider's
own 404, which is what a retired id gets, comes back as `hoosh returned HTTP 502: upstream provider returned HTTP 404: …`.
Testing for 404 alone would miss the case a stale catalog actually hits, and a 502 alone would re-read on every bad
gateway. Both spellings were read off a running hoosh.

⚠ **And that 502 is not always JSON.** With no `message` key in the provider's answer hoosh passes the raw body on,
unescaped, inside a JSON string, so Ollama's 404 reached agnosai as a body that parses to nothing, and the error as a bare
`hoosh returned HTTP 502`. Those words are read off the raw body, and nothing else from it is, as the rest is a provider's
text that would reach a task's `output`.

⚠ **Only before any answer.** No answer yet is what says no tool has run. Selecting again rebuilds the request, and a task
that has called a tool keeps its transcript (ADR 029), so an id refused after a tool ran fails the task rather than
running the tool twice.

An id is not excluded after a refusal. The list going stale heals at hoosh's next refresh, and a refusal list would need
forgiving too, or a provider that 404s everything for a minute bricks a process until restart. The span follows: a task
re-selected onto a new id reports that one as `gen_ai.request.model`, which is the case an operator opens a trace for.

### Attribution

A response with no `usage.provider` gets its entry's `owned_by`, then that of the id asked for, as a provider answers with
the dated form of an alias. The spelling is hoosh's, so it matches what `usage.provider` writes. **With neither, `provider`
is omitted**, for the reason `cost_micro_usd` is: a name that was guessed cannot be told from one that was reported. The
prefix needles are gone, and with them the fall through to Ollama. Only a hoosh before 2.6.0 reaches this at all.

### A named id

A listed id goes out exactly as listed. The strip stays for an id that is not listed, as litellm's `ollama/llama3` spelling
is what it has always served, and with no catalog, when the two cannot be told apart. An id the strip would not touch asks
nothing, so an agent naming `gpt-4o` causes no read to select it, and none when it is refused.

### `BUDGET_MODELS` is dropped

Pricing it from the catalog means a task's token split that nobody has, a home after `src/llm/` for a function in `fleet/`,
and a public API with no caller. F5 is where budgets get a design, and the catalog's prices are public to it. A consumer
that wants the cheapest listed model has `agnosai_model_catalog_pick`. `agnosai_estimate_crew_cost` stays: it is a
self-hosted GPU-hour estimate, not a provider's price.

## Consequences

- **Positive.** A crew works on any gateway that lists a chat model, with no configuration and no agnosai release when a
  provider ships a model. A model that does not exist can no longer be sent by default. Selection allocates nothing, where
  it minted a 16-byte Str per task.
- **Negative.** A crew with no `llm_model` now needs the gateway to be up before its first request, not just at it, and
  a gateway with an empty list fails such a crew with `no model catalog`, where it used to 404.
- **Negative.** `Premium` means the dearest current model, and on the table above that is $10/$50. An agnostic crew with
  `complexity: high` costs five times what Capable does, which it did not when both were the same 404. The override is the
  control, and F5 is where a budget would enforce one.
- **Negative.** A model the gateway lists and refuses fails every crew that selects it until hoosh's list changes or an
  operator overrides the tier. The error says which id.
- **Negative.** `provider` can be absent from `metadata`, on a gateway older than 2.6.0 and only for an id nobody lists.
- **Negative.** A process with a catalog keeps one for each change at hoosh for its life. At hoosh's default of one
  refresh in six hours, with a list that changes each time, that is four a day at about 30 KiB for 200 models. Two replicas
  behind one URL that disagree would keep one per re-read instead, which the ration bounds at one a minute. The read itself
  reserves 2 MiB of address space per gateway for good, as sandhi sizes its receive buffer to the cap before a byte arrives.
- **Negative.** A task refused within a minute of the last read fails rather than reading again, and the next one
  past the minute recovers. An id retired hours after a read recovers on the first refusal.
- **Neutral.** `agnosai_default_model` and `agnosai_select_cheapest_model` are removed from a patch release. agnostic and
  thoth call neither. joshua and kiran are not on the machine this was written on, so theirs is unchecked. Leaving the first
  would keep a function whose only job is to name a model nobody listed.
- **Neutral.** hoosh could end the heuristics here: its table already has a tier and a tools bit for each model it knows,
  and whether an id takes chat is known at the route. These are asks on the roadmap, with the two hoosh bugs this found,
  and none changes a selection until it ships.

## Alternatives considered

- **Tier aliases at hoosh.** `"llm_model": "capable"` was in our docs and is what a reader expects. hoosh has none, and it
  would move this rule to a gateway that has no idea what a crew is, with the same inputs.
- **`POST /v1/cost/recommend`.** It ranks routes' exact patterns, so a route of `claude-*` is not in it at all.
- **Price alone.** Picks `claude-opus-4-1` as Premium, as above.
- **The widest `effort` list for Premium.** The count of levels is not a rank: a cheap model that takes all five would beat
  a dearer one that takes four, and `thinking` modes count backwards, as older models take both.
- **A median over entries.** Moved by aliases: three dated forms of one model drag Capable to that model.
- **Free models on the ladder.** Fast and Capable become whatever the box has pulled, which is the failure this began with.
- **Reading at start-up.** A gateway down at boot would then hold nothing until a selection failed anyway, so this adds a
  path and a boot dependency for nothing.
- **One catalog per process.** "Once per process" reads that way, and is wrong the moment a library consumer names two
  gateways, as one's ids are guesses on the other.
- **A timed refresh.** A process would poll for a list that matters only when it stops working, and a refused id is the
  signal that it has.
- **Excluding a refused id.** See *A selection the gateway refuses*.
- **No ration on a re-read.** The first version of this had none, on the reasoning that a GET costs little beside the chat
  call that failed. That is true of agnosai and false of hoosh, whose heap keeps what the GET allocates. Review found it, and
  a measurement settled it.

## Addendum (2.3.3): hoosh says the tier, the kind and the tools

hoosh 2.10.2 answered the three asks above ([its ADR 012 addendum](https://github.com/MacCracken/hoosh/blob/main/docs/decisions/012-live-model-catalog.md#addendum-2102-tier-kind-and-tools)).
A catalog entry can carry `tier` (`economy`, `standard`, `premium` or `reasoning`), `kind` (`chat`, `embedding` or
`other`) and `capabilities.tools`, and each is **absent when nobody said**. agnosai reads all three, and they decide
wherever they are present. The rule above stays for a gateway that writes none of them.

### Kind

**`kind` outranks the twelve words.** `chat` is a chat model even when its id holds one of the words
(`gpt-4o-audio-preview`), and any other word is not, including one hoosh adds later, since hoosh writes `chat` for a
model that does both. The words are read only for an entry with no `kind`. On hoosh 2.10.2 that is an entry the words
do not match: hoosh runs every entry through the same twelve (`metadata_lend`, then `catalog_id_kind`) and writes a
`kind` for each match. So the words now matter only on an older gateway, and retire when agnosai requires hoosh 2.10.2
(the roadmap's *Decisions for the user*).

### Tier

**Where any model on the ladder has a tier, the tier decides, and the first listed in it wins.** The ladder is
unchanged: paid models first, and free ones only when nothing usable is paid.

| agnosai's tier | Takes hoosh's tiers, nearest first |
|---|---|
| Fast | `economy`, then `standard`, `premium`, `reasoning` |
| Capable | `standard`, then `economy`, then `premium`, `reasoning` |
| Premium | `premium`, then `reasoning`, then `standard`, `economy` |

- **`reasoning` is read as Premium, after `premium` itself.** hoosh gives it to `o3`, `o4-mini`, `deepseek-reasoner`
  and `deepseek-r1`. These are a kind of model, not a rung above `claude-fable-5-1`, so a Premium task takes one only
  where nothing is `premium`, and Capable reaches one last.
- **A tier with nothing in it takes the nearest, below before above.** A crew that asked for less did not agree to
  spend more. An Anthropic route limited to Haiku answers Premium with its first Haiku, not a failure.
- **First listed, not cheapest or dearest.** hoosh lists `[[models]]` first, then routes in router order, then each
  provider's own order, and Anthropic's is newest first. So `claude-opus-4-1`, `premium` at $15/$75, loses to
  `claude-fable-5-1` because it is listed later, and the price rule is not consulted. An operator who lists it first,
  or sets `AGNOSAI_MODEL_PREMIUM`, gets it.
- **A model with no tier is left out while any model on the ladder has one.** Nobody said what it is, so a cheap
  untiered `custom-x` does not become Fast beside a tiered Claude. It is still listed, and an agent can name it. The
  price rule runs over the untiered models only when no model on the ladder has a tier.

### Tools

**A task whose request carries tools is never given a model hoosh says takes none.** A task carries tools when the
crew has a tool loop and its agent names a tool (ADR 029). A model nobody spoke for may take tools, so only a stated
`false` is ruled out. The ladder for such a task is the paid models it can send, so with every paid model saying
`false`, a local model that takes tools is chosen rather than nothing.

A catalog holds six picks, three tiers for each answer to "does the task carry tools". Where no entry says `false`,
both sets are the same entries, and the second three are not computed.

⚠ **A list with chat models and none that takes tools is asked again at the ration's pace.** "Nothing to pick reads
again at once" is for a gateway coming back. Here only hoosh changing its list would help, and an unrationed read per
task would let any crew with tools grow the gateway's heap on every task, which is what the ration exists to stop. The
task fails before any request, with `no model catalog: hoosh lists no chat model that takes tools`. The agent's own
`llm_model` and the operator's override are not checked for tools, as they are not checked for anything.

A refused id selects again under the same rule, so a task with tools is re-selected onto a model that takes them.

### The known wrong answers, on hoosh 2.10.2

| Was | Now |
|---|---|
| `gpt-6-astra` for a `complexity: high` agent with tools, and a provider's 400 | The first `premium` model that takes tools: `gpt-5.6-sol` on an OpenAI route |
| Beside OpenAI, Capable and Premium are Anthropic's. An Anthropic route limited to Haiku makes Premium a Haiku | hoosh's tier decides, and the route order breaks a tie between providers. Premium is `gpt-6-astra` beside a Haiku-only route |
| A catalog of prices alone gets the dearest model as Premium | The first `premium` model listed, at any price |
| `bge-m3` listed first is every tier on a box of local models | `kind: embedding` keeps it off every tier |

**Measured end to end.** A real hoosh 2.10.2 at stub providers, and the real agnosai server running one-agent crews.
The stubs listed `gpt-6-astra`, `gpt-6.1-sol`, `gpt-6-sol`, `gpt-5.6-sol`, `gpt-6-luna` and `text-embedding-3-small`
on an OpenAI route, and `bge-m3:latest` then `llama3.2:3b` on an Ollama route. Each cell is the model the provider was
asked for:

| Crew | 2.3.2 | 2.3.3 |
|---|---|---|
| `high`, with tools | `gpt-6-astra` | `gpt-5.6-sol` |
| `high`, no tools | `gpt-6-astra` | `gpt-6-astra` |
| `medium`, with tools | `gpt-6.1-sol` | `gpt-6-sol` |
| `medium`, no tools | `gpt-6.1-sol` | `gpt-6.1-sol` |
| Ollama, `high` | `bge-m3:latest` | `llama3.2:3b` |
| Ollama, `medium`, with tools | `bge-m3:latest` | `llama3.2:3b` |

On 2.3.2, all four `gpt-6-astra` and `gpt-6.1-sol` calls carried tools, which hoosh's table says those models do not
take on Chat Completions, and both Ollama calls sent chat to an embedder.

### Still a heuristic

- **First listed is the provider's order.** Anthropic's is newest first. OpenAI's `/v1/models` order means nothing, so
  on an OpenAI route an older `premium` model listed before a newer one is Premium. hoosh's tier says both are
  premium, and agnosai has nothing better to go on. The override, or a `[[models]]` entry, says which.
- **Older gateways keep the price rule and its four wrong answers.** That is every hoosh before 2.10.2 and any gateway
  that is not hoosh.
