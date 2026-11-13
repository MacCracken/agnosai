# 040 — A2A v1.0: an Agent Card, and a JSON-RPC endpoint whose tasks are crews

**Status**: Accepted, shipped in 2.6.0
**Date**: 2026-11-13

## Context

The Rust line's A2A is `POST /api/v1/a2a/receive`: a delegation shape of its own, from before the protocol, which
takes a task id and a description, runs a crew, and posts the answer to an optional callback. It is wire-compatible
with the Rust line (design principle 1) and agnostic's oracle uses it.

The A2A protocol is at 1.0.0. An agent publishes an **Agent Card** at `/.well-known/agent-card.json`: who it is, the
interfaces it answers on (`supportedInterfaces`, each a URL, a binding and a protocol version), what it can do
(`capabilities`), its skills, how a caller authenticates, and optionally `signatures`, each a JWS (RFC 7515) over the
card's RFC 8785 canonical form. On the JSON-RPC binding a client sends PascalCase methods, `SendMessage`, `GetTask`,
`ListTasks`, `CancelTask`, `SendStreamingMessage`, `SubscribeToTask`, four push-configuration methods and
`GetExtendedAgentCard`, with an `A2A-Version` header; a request without one is 0.3's. A task has an id, a context, a
status whose state is a `TASK_STATE_*`, artifacts and a history, and A2A's errors carry a `google.rpc.ErrorInfo`.
agnostic's M7 waits on agnosai for A2A v1.0.x with signed cards.

## Decision

**agnosai serves the card and `POST /a2a`, the JSON-RPC binding at 1.0, in `src/a2a_v1.cyr`, beside
`/api/v1/a2a/receive`, which stays as it is. A task is a crew. The card can be signed, ES256 or RS256, once, at
start-up.** The user chose the whole surface, a signed card included; push notifications are not in it.

- **A task is a crew.** `SendMessage` reads a message's text and data parts (a data part as its JSON, a raw or URL
  part `ContentTypeNotSupportedError`) into a one-task crew in an arena of the orchestrator's (ADR 037), and the
  task's id is the crew's. The crew keeps what a task has and a crew does not, in its metadata: its `contextId` (the
  client's, or one minted) and the message, which is the task's history. `GetTask`, `ListTasks` and `CancelTask` read
  the registry, holding and copying as ADR 037 says, so a task lives as long as its crew. A crew the REST API or a
  consumer started is no task.

  | crew | task |
  |---|---|
  | pending | `TASK_STATE_SUBMITTED` |
  | running, or awaiting a reviewer | `TASK_STATE_WORKING` |
  | completed, failed, cancelled | `TASK_STATE_COMPLETED`, `_FAILED`, `_CANCELED` |
  | refused at admission (ADR 033) | `TASK_STATE_REJECTED`, never registered |

  A crew awaiting a reviewer is `WORKING`, not `INPUT_REQUIRED`: what it waits for is a reviewer's, not the client's.
  A failed or cancelled task's status carries the agent's message: the crew's reason, and for a failed one its failed
  result's error, `task_failed: <error>`, as the reason alone says that a task failed and not why.
- **`SendMessage` blocks** until the crew ends, unless `returnImmediately`, when the crew is submitted and the task
  answered at once. Each completed result is an artifact, its output as `text/plain`. A message to an existing task is
  `UnsupportedOperationError`: a crew takes no further input. The text is 10,000 bytes at most and the metadata 64 KiB,
  as `/api/v1/a2a/receive`'s are.
- **`ListTasks`** lists the endpoint's tasks by id, filtered by context and state, a page at a time; a page token is
  the next page's offset. `statusTimestampAfter` is `UnsupportedOperationError`, as crews keep no status time and a
  filter not applied would answer wrong.
- **The streams** are the route's answer, run on the connection (`agnosai_route_stream_a`): `text/event-stream`, each
  event `data:` and a JSON-RPC response whose `result` is a `StreamResponse`. The task first, then a `statusUpdate`
  each time its state moves, then each artifact and a last `statusUpdate`, and the stream closes. A stream reads the
  registry from every millisecond, doubling while nothing moves, to every 50 ms, the crew stream's interval: an
  event's payload does not carry a task's artifacts, and a lagging subscription would lose what the registry still
  holds. A fixed 50 ms made a stream of a task that ends in microseconds last 50.5 ms; it lasts 1.3. Each frame is built in a pooled scratch reset after it. A
  client that leaves ends the stream, not the task.
- **The version**: `A2A-Version` 1.0, a patch ignored; none, 0.3, or any other is `VersionNotSupportedError`. **Every
  JSON-RPC answer is HTTP 200**, errors in band: the binding names no status for them.
- **Not supported, and said so**: push notifications (`pushNotifications: false`, `PushNotificationNotSupportedError`
  from the four methods and from a `SendMessage` asking for one) and the extended card (`extendedAgentCard: false`,
  `UnsupportedOperationError`).
- **The card** names the endpoint under `AGNOSAI_PUBLIC_URL`, or, without one, under the request's `Host` over `http`,
  a host checked to be one a URL can carry, else `localhost`. With auth on it declares the bearer scheme the endpoint
  checks, a JWT's when one is configured. It is public, as discovery cannot carry a token, and kept five minutes by any
  cache when it names the public URL, and by none when it names a request's host.
- **The signed card.** `AGNOSAI_A2A_SIGNING_KEY` names a PEM private key, P-256 (ES256) or RSA (RS256), and
  `AGNOSAI_A2A_SIGNING_KID` its id (`agnosai` unset), with `AGNOSAI_A2A_JKU` in the header when set. The card is built
  and signed once at start-up for the public URL (`agnosai_a2a_config_seal`) and answered as it is: a card that named
  each request's host would need a signature a request, and one per host a client could send. So a key without a public
  URL is an ERROR and the card goes out unsigned, as a JWT key that will not decode boots. The canonical form is
  `src/jcs.cyr`'s, which refuses what it cannot write canonically, a float or a name twice, rather than write it wrong.
  `agnosai_a2a_card_verify_es256_a` and `_rs256_a` check a card's signature, as a client should before trusting one.

## Consequences

- **Positive**: an A2A 1.0 client can find agnosai, delegate a task, follow it, list and cancel it, and check that the
  card it read is the operator's. agnostic's M7 has what it waits for, and the card's signer and verifier are in the
  library for its own card.
- **Positive**: a task costs what a crew does and nothing more: a message sent and listed, or streamed, leaves 0 B on
  the global heap once its crew is evicted (`tests/a2a_v1.tcyr`). A signature verified by an independent
  implementation, openssl over Python's canonical form, matches.
- **Negative**: **a stream holds a pool worker for its task's life**, as the crew stream does (ADR 014).
- **Negative**: **a task is gone when its crew is evicted**, past `AGNOSAI_MAX_RETAINED_CREWS`, and `GetTask` then
  answers `TaskNotFoundError`. `ListTasks` holds every registered crew in turn, so it costs the registry's size.
- **Negative**: tasks are scoped to the endpoint's auth, not to a caller: agnosai has one credential, and every
  authenticated caller sees every task.
- **Negative**: the signing key is in memory for the process's life, as the JWT key is.
- **Neutral**: the 0.3 method names (`message/send`, …) are `-32601`; a 0.3 client is told its version is not spoken
  before that.

## Alternatives considered

- **Make `/api/v1/a2a/receive` the protocol.** It would break the Rust line's wire, which agnostic's oracle reads.
- **A task table of its own.** A second record of every task beside the registry, which would have to be bounded,
  evicted and kept in step with it; the crew's metadata holds the two things a task adds.
- **Streams from the event bus.** A `task_completed` event carries the task's id and status, not its output, so the
  artifact would be read from the registry anyway, and a lagging subscription loses events the registry still reflects.
- **Sign per request**, for a card that names the request's host. An ECDSA signature a request on a public route.
