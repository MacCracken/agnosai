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

* **Crew presets.** At the time this was written, `GET /api/v1/presets`
  returned an empty array, because `src/definitions/` was not ported yet.
  Projecting nothing is not a capability. ⚠ **This half has since expired** —
  see the re-check at the bottom.
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

## Re-check — 2026-08-11, post-port

The **Decision is unchanged and is what the tree does.** Verified against
`src/server/routes/mcp.cyr`: `_agnosai_mcp_prompts_list` names each entry by
`agnosai_agent_key(def)` and attaches one `required: false` `task` argument;
`_agnosai_mcp_prompts_get` builds its text from
`_agnosai_crew_build_system_prompt(def)` and appends `"\n\nTask: "` only for a
non-empty string argument; the message role is `AGN_JV_USER`;
`_agnosai_mcp_initialize` sets `capabilities.prompts` to an empty object with no
`listChanged`; and `agnosai_route_mcp_a` dispatches seven methods against the
oracle's three (`rust-old/src/server/routes/mcp.rs:57-59`).
`tests/server_routes_mcp.tcyr` asserts the text is byte-identical to
`_agnosai_crew_build_system_prompt`, so the one-renderer property is pinned
rather than described. bote still ships exactly 18 `prompt_*` functions in
`lib/bote-core.cyr`, and `dispatcher_set_server_info` is present.

**One Context claim went stale, and the port is why.** The presets bullet was
true when written: `src/definitions/` did not exist in this tree until
2026-08-09. It now does, and `agnosai_route_list_presets_a`
(`src/server/routes/definitions.cyr:36`) returns all eighteen embedded presets
via `agnosai_builtin_presets` (`src/definitions/loader.cyr:571`,
`AGNOSAI_PRESET_JSON_COUNT = 18`). The oracle was never unconditionally empty
either — `rust-old/src/server/routes/definitions.rs:26-38` asserts *non-empty*
with the `definitions` feature and empty without it, and this port carries the
feature.

**That does not reopen the decision, and it was not re-taken.** A prompt is
rendered by `_agnosai_crew_build_system_prompt`, which takes **one agent
definition**; a preset is a crew composition holding several agents and has no
persona of its own, so there is no single string for `prompts/get` to return.
Whether presets deserve their own MCP projection — as `resources`, or as a
prompt per preset agent — has not been evaluated since they became real. It is
recorded here as **open**, not as rejected: the reason given above for rejecting
them in August no longer applies, and no replacement reason has been argued.
