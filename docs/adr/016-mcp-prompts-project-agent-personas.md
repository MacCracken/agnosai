# 016 — MCP prompts project agent personas

## Status: Accepted

## Context

Roadmap B1 called for growing the MCP surface onto bote, which ships 18
`prompt_*` functions. `src/server/routes/mcp.cyr` answered `initialize`,
`tools/list` and `tools/call` — the oracle's three, and therefore full parity —
plus `resources/list` and `resources/read` from [ADR 015](015-mcp-resources-project-agent-definitions.md).

`prompts/list` and `prompts/get` were the remaining half. The question was not
whether to answer them but **what a prompt should be**, given that agnosai has no
prompt store and inventing one would mean new state, new lifecycle and a new
management surface.

Two candidates existed:

* **Crew presets.** `GET /api/v1/presets` returns an empty array — a stub the
  oracle also leaves empty. Projecting nothing is not a capability.
* **Agent definitions.** Already stored by `POST /api/v1/agents/definitions`,
  already addressable as `agnosai://agents/<name>`, and already carry exactly
  what a prompt template is: a role, a goal, a backstory, a domain and a tool
  list.

## Decision

**A prompt is a stored agent definition, rendered by the crew's own system-prompt
builder.**

* `prompts/list` returns one entry per definition, **named by agent key** — the
  same identifier `POST /api/v1/agents/definitions` stores it under and
  `agnosai://agents/<name>` addresses. One name across three surfaces.
* Each declares a single **optional** `task` argument. The persona is useful on
  its own; a client wanting the agent's framing should not have to invent a task.
* `prompts/get` returns one message whose text is
  `_agnosai_crew_build_system_prompt(def)`, with `\n\nTask: <task>` appended when
  the argument is supplied and non-empty.
* The message role is **`user`**. MCP's `PromptMessage` role is `user` or
  `assistant` — there is no `system` — so the persona is delivered as the opening
  user turn, which is what the specification's own examples do.
* `capabilities.prompts` is advertised, **without `listChanged`**.

## Consequences

**One renderer, so preview cannot drift from execution.** The text a client pulls
through MCP is byte-for-byte what a crew sends the model for that agent. A
separate template here would have been a second source of truth for the same
string, and the two would have diverged the first time either changed.
`server_routes_mcp` asserts the identity directly against
`_agnosai_crew_build_system_prompt` rather than re-describing the format, so a
second renderer fails the suite.

**No new state and no new lifecycle.** Prompts appear and disappear exactly as
definitions do. Nothing is stored, nothing is reference-counted, and there is no
management surface to secure.

**`listChanged` is deliberately absent**, for the same reason `subscribe` is
absent from `resources` in ADR 015: definitions can be added at any time by the
REST route and nothing here can push a `notifications/prompts/list_changed`. A
capability key is a promise a client acts on; claiming this one would leave a
client waiting for a notification that never arrives.

**This is beyond the oracle.** `rust-old` answers three MCP methods; agnosai now
answers seven. The divergence is additive — no oracle behaviour changes — which
is the same basis ADR 015 was accepted on.

**Still not delegating to bote's `Dispatcher`.** The oracle uses bote's protocol
*types* and hand-builds its envelope (`mcp.rs:3-5`), and that remains the parity
behaviour. bote 3.3.0's `dispatcher_set_server_info` cleared the blocker to
delegating, but doing so is a separate decision from answering more methods.
