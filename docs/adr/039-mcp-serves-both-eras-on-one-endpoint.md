# 039 — `/mcp` serves both MCP eras on one endpoint, by hand

**Status**: Accepted, shipped in 2.6.0
**Date**: 2026-11-13

## Context

`/mcp` spoke MCP 2025-03-26: an `initialize` handshake, then `tools/list`, `tools/call`, and since ADRs 015 and 016
`prompts/*` and `resources/*`, every answer an HTTP 200 carrying a JSON-RPC envelope. MCP 2026-07-28 changed the
protocol's shape:

- **No handshake and no session.** `initialize`, `notifications/initialized` and `Mcp-Session-Id` are gone. Every
  request names its revision in `params._meta["io.modelcontextprotocol/protocolVersion"]` and carries the client's
  capabilities in `_meta["io.modelcontextprotocol/clientCapabilities"]`; `server/discover`, which a server MUST
  answer, says what it speaks and can do.
- **Headers the server checks against the body.** A POST carries `MCP-Protocol-Version`, `Mcp-Method`, and for
  `tools/call`, `prompts/get` and `resources/read` an `Mcp-Name` mirroring `params.name` or `params.uri`, Base64'd
  in a `=?base64?…?=` sentinel when it cannot be a plain header value. A missing, malformed or disagreeing one is
  HTTP 400 and `-32020`; an unsupported revision is 400 and `-32022`, naming the supported ones.
- **New result fields.** Every result carries `resultType`; the list results and `resources/read` carry `ttlMs` and
  `cacheScope`. An unknown method is 404 and `-32601`, a notification 202 with no body, and a POST one message.
- **An unknown tool is a protocol error**, `-32602`, where the oracle answered a success with `isError`.

bote 3.8.15 serves both eras on one endpoint (`src/http_era.cyr`). The roadmap asked agnosai to adopt that or
follow it by hand, and to say which here. bote's era layer is in its full bundle, `dist/bote.cyr`, not the core
bundle agnosai pins, and it answers on the socket itself: it reads the request, writes the response, and returns.

## Decision

**`/mcp` serves 2026-07-28 and 2025-03-26 on one endpoint, deciding per request, and follows bote's era rules by
hand in `src/server/routes/mcp.cyr`.** The user chose this over delegating to bote-core's dispatcher and over the full
bundle.

- **The era is the request's**, as bote decides it. A request whose header or `_meta` names 2026-07-28 is served
  under that revision. One with no header, or a header naming 2025-03-26, is served as before, byte for byte, the
  oracle's wire, an absent `id` echoed as null included (`tests/server_routes_mcp.tcyr` is unchanged). A body naming
  2026-07-28 with no header is refused for the header; a header naming another revision than the body, or one this
  endpoint does not speak, is refused. The endpoint speaks `["2026-07-28", "2025-03-26"]`.
- **The checks, in bote's order**: the body, then `_meta`'s version and capabilities (`-32602`, 400), then each
  header against the body (`-32020`, 400, in the spec's words: *"Header mismatch: Mcp-Name header value 'foo' does
  not match body value 'bar'"*). A header on two lines is malformed, as lines of one field combine into a value no body
  matches. `initialize` under 2026-07-28 is 404, as that revision has none.
- **The answers**: `resultType: "complete"` on every result; `ttlMs` 0 and `cacheScope: "private"` on the lists and
  `resources/read`, as a tool or definition added later is announced to no one and every answer is behind the
  endpoint's auth; `server/discover` 60 seconds; the server's identity in each result's `_meta`. A missing resource's
  error names its `uri`. Any cursor but the empty one is invalid, as every list is one page.
- **Statuses**: 404 for `-32601`, 400 for `-32600`, `-32700`, `-32020` and `-32022`, and 200 for the rest, in band,
  as bote's `_he_status_for` has it.
- **The headers reach the route as the request itself.** `src/http_headers.cyr` is a 24-byte view of sandhi's buffer,
  where its header lines end found once, that the serve layer passes through `agnosai_route_dispatch_h_a`; one pass
  over the lines reads a header's first value and how many lines name it. A caller that dispatches directly passes 0,
  and every header is then absent, which is what such a caller always had.

## Consequences

- **Positive**: a 2026-07-28 client is served under its revision, and every earlier client as before. agnostic's M7
  builds its own `/mcp` on bote, and this endpoint is the binary's.
- **Positive**: the answer stays the route's. It goes through the serve layer's auth, rate limit, metrics and request
  arena, and a tool call keeps its span. A 2026-07-28 request costs the global heap what the same method costs the
  earlier revision (`tests/server_routes_mcp_2026.tcyr`).
- **Negative**: about 400 lines track the spec here, apart from bote's. A change to the revision's rules is made
  twice, in bote and here.
- **Negative**: **an MCP `tools/call` keeps 601 B a call on the global heap**, in either era: the tool's input and
  output are built there. It predates this release and is measured, not fixed, here.
- **Not here**: the Tasks extension, `subscriptions/listen`, input-required results and `x-mcp-header` parameters,
  none advertised: no tool declares a parameter header, and nothing pushes. `logLevel` in `_meta` is not read, as
  this endpoint sends no log notifications. The `Origin` check against DNS rebinding that both revisions ask of a
  server is not made, in either era, as before.

## Alternatives considered

- **bote-core's dispatcher.** It answers 2026-07-28's JSON-RPC already: `server/discover`, `resultType`, the caching
  fields, the errors. agnosai's tools, prompts and resources would be registered into it and mirrored as they change,
  as both registries change at run time, and every MCP test would change with the envelope's owner.
- **The full bote bundle.** The least code here, and a 28k-line bundle whose era layer writes to the socket around the
  serve layer.
- **2026-07-28 alone.** A client of an earlier revision would have no way in: it sends `initialize` and no version
  header.
