# 015 — MCP resources project agent definitions, and nothing else

## Status: Accepted (2026-08-05)

## Context

When this was written, `src/server/routes/mcp.cyr` answered exactly three
JSON-RPC methods — `initialize`, `tools/list`, `tools/call` — because that is
exactly what `rust-old/src/server/routes/mcp.rs` answers. Parity was met and had
been since M6. (It answers seven today; see the Neutral consequence.)

Roadmap B1 asks to grow the surface, on the grounds that bote is the MCP layer
and the pinned bundle already carries the machinery. That is true of
**prompts and resources**: `lib/bote-core.cyr` ships `prompt_registry_*` /
`prompt_def_*` / `prompt_arg_*` and `resource_registry_*` / `resource_def_*`,
and bote's own dispatcher answers `prompts/list`, `prompts/get`,
`resources/list`, `resources/read`.

It is **not** true of subscriptions, and B1's wording ("prompts, resources and
subscriptions") overstates the bundle. Verified 2026-08-05: there is no
`resources/subscribe` anywhere in `lib/bote-core.cyr`. What exists is
`req_is_notification` / `dispatcher_notifications`, which is JSON-RPC
notification *detection* — a different thing from an MCP resource subscription,
and not a substitute for one.

CLAUDE.md's bar is "matches what Rust did. Diverge only with an ADR." Answering
a method where the oracle answers `-32601 Method not found` is wire-visible, so
it needs this record whether or not it is an improvement.

## Decision

**Add `resources/list` and `resources/read`, projecting stored agent
definitions and nothing else.**

The URI scheme is `agnosai://agents/<name>`, where `<name>` is the definition's
key in `AGN_ST_DEFINITIONS` (`src/server/state.cyr:48`). `resources/read` is a
single `agnosai_app_state_definition_get`, and the served `text` is
`agnosai_agent_to_value_a(a, def)` — the same builder, on the same request
arena, that `GET /api/v1/agents/definitions` uses (`src/server/routes/agents.cyr:43`),
so the bytes match.

In scope:

- `resources/list` → one descriptor per stored definition.
- `resources/read` → that definition's JSON under MCP's `contents` envelope.
- `capabilities.resources` in `initialize`, **without** `subscribe`.

Out of scope, deliberately:

- **Subscriptions.** Not in the bundle, and nothing in agnosai can push a
  `notifications/resources/updated` — the MCP route is a request/response
  handler with no channel to the client. Advertising `subscribe` would leave a
  client waiting for an event that cannot arrive.
- **Prompts.** A separate bite; the registry exists but what agnosai should
  *expose* as a prompt is a product question this ADR does not answer.
- **Crews, tasks, runs.** Runtime state with a lifecycle. Definitions are inert
  stored documents, which is what makes them a safe first projection.
- **Any new state.** This adds no storage and no lifecycle. Every byte it serves
  was already reachable over REST.

## Consequences

- **Positive** — an MCP client can discover and read agent definitions without a
  second protocol. The three oracle methods are untouched and their tests
  unchanged, so parity where parity exists is not disturbed.
- **Positive** — the projection is closed. `resources/read` can only reach keys
  the definitions map already holds; there is no path parameter, no filesystem,
  and no way to address anything else.
- **Negative** — agnosai's MCP surface is now a **superset** of the oracle's, so
  `mcp.cyr` can no longer be diffed against `mcp.rs` as a whole file. The three
  ported handlers stay individually comparable; the added ones are marked in
  place as beyond-oracle.
- **Negative** — a wire contract we now own. `agnosai://agents/<name>` is ours
  to keep stable; the oracle offers no guidance because it has no equivalent.
- **Neutral, and now settled** — this said "prompts remain owed under B1, and
  the 'subscriptions' half of that roadmap line should be struck rather than
  carried". Both halves landed the next day: prompts shipped as `prompts/list` +
  `prompts/get` under [ADR 016](016-mcp-prompts-project-agent-personas.md),
  projecting the same stored definitions as personas, and roadmap B1 closed
  2026-08-06 with subscriptions explicitly struck for the reason given above.
  `src/server/routes/mcp.cyr` therefore answers **seven** methods today — the
  oracle's three plus this ADR's two and ADR 016's two — and `capabilities`
  advertises `tools`, `resources` and `prompts`, none of them carrying
  `subscribe` or `listChanged`.

## Alternatives considered

- **Delegate to bote's `Dispatcher`.** Rejected, and the reason predates this
  ADR: the oracle uses bote's protocol *types* and explicitly declines its
  Dispatcher (`mcp.rs:3-5`), so hand-building the envelope IS the parity
  behaviour. bote 3.3.0's `dispatcher_set_server_info` removed the
  `"serverInfo":{"name":"bote"}` blocker, which clears the way for a future
  delegation — it does not make delegating correct today.
- **Project crews as resources instead.** Rejected for now. Crews are runtime
  state with a lifecycle; a resource read of a running crew raises questions
  about consistency and cancellation that a stored definition does not.
  Definitions first is the smaller claim.
- **Advertise `subscribe` and answer it with an error.** Rejected. A capability
  key is a promise a client acts on; advertising one that always fails is worse
  than not advertising it.
- **Do nothing — parity is met.** Rejected because B1 asks for the surface, and
  the projection costs no new state. But it is the honest default, and it is why
  the scope here is deliberately the smallest thing that is useful.
