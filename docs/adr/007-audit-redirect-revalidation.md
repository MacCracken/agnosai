# 007 — Re-validate the SSRF guard on every redirect hop

## Status: Accepted

Date: 2026-07-28. Applies to `src/tools_builtin_security_audit.cyr`, and to
`src/tools_remote_registry.cyr` when it lands.

## Context

`rust-old/src/tools/builtin/security_audit.rs` guards its outbound request with
`crate::server::ssrf::is_safe_url(&target_url)` and then hands the URL to a
`reqwest::Client`. That is one check, against one string, before one call — and
reqwest's default redirect policy follows up to 10 hops.

The gap is the classic SSRF-via-redirect bypass. A caller supplies
`http://attacker.example.com/`, which is a genuinely public address and passes
the guard. The server answers `302 Location: http://169.254.169.254/latest/meta-data/`.
reqwest follows it, and the tool returns the cloud metadata service's response
headers to the caller. The guard is never consulted about the address actually
contacted, because by the time redirects are being followed the check has
already happened and the URL it validated is no longer the URL in play.

This matters more here than it would elsewhere: a security-audit tool is
plausibly reachable from an agent acting on untrusted input, and its entire
output is "here is what that server sent back".

Porting to Cyrius forced the question rather than allowing it to be inherited,
because sandhi's default is the opposite of reqwest's:
`sandhi_http_options_new` initialises `SANDHI_HTTP_OPT_OFF_FOLLOW` to 0, so
nothing is followed unless asked. Three options were on the table.

**Match the oracle exactly** — enable following, keep the single up-front check.
Reproduces the oracle's behaviour, including the hole, in a tool whose purpose
is finding holes.

**Inherit sandhi's default** — never follow. Closes the hole, but breaks the
tool's primary use case. Auditing `http://example.com` when it redirects to
`https://example.com` would analyse the redirect's own response, which carries
essentially no headers, and report a well-configured site as 0 of 7 headers
present, score 0, risk critical. Silently wrong answers on the common input.

**Follow, and re-check each hop.** More work, and a deliberate divergence.

## Decision

Follow redirects, up to reqwest's limit of 10 hops, and run
`agnosai_is_safe_url` against every resolved hop **before** the request to it is
issued. `_agnosai_audit_fetch` owns the loop; sandhi's own follower is not used,
because its hop loop is internal and offers no per-hop callback.

Six supporting rules:

- **An https → http hop is refused.** sandhi refuses this in its own follower
  and the reasoning carries: an audit must not be talked down onto a cleartext
  channel by the host it is auditing.
- **Location resolution fails closed.** Absolute URLs and absolute paths are
  resolved; a relative reference like `../admin` or a protocol-relative
  `//host/path` is refused, and the response already in hand is reported
  instead. Full RFC 3986 path merging is not worth the attack surface for a
  form that essentially never appears in a redirect.
- **A refused hop is distinguishable from a failed request.**
  `AGNOSAI_AUDIT_REDIRECT_BLOCKED` propagates to a specific error —
  "target redirected to a private/internal address" — rather than a generic
  failure. An attempted bypass should read like one in the logs.
- **The CORS preflight does not escalate.** A blocked or failed OPTIONS hop is
  dropped and CORS is reported as unconfigured, matching the oracle's treatment
  of a failed preflight. Only the header GET can fail the audit.
- **The 15s budget spans the chain, not the hop.** `Client::timeout` in reqwest
  is one deadline covering a request and every redirect it follows. Handing each
  hop a fresh 15s would let an 11-hop chain run for 165 seconds, twice over, so
  `total_ms` is rewritten from the remaining budget before every request.
- **Scheme comparison is case-insensitive here.** `sandhi_url_parse` compares
  schemes case-insensitively via `_sandhi_url_sceq`, so `agnosai_is_safe_url`
  accepts `HTTPS://`. A byte-exact test in the hop loop would therefore skip the
  downgrade refusal on an upper-case target, and slice the origin one byte short
  when resolving a relative `Location` — pointing the next request at a
  different host entirely. The byte-exact test in `agnosai_audit_analyze` is
  left alone: the oracle's HTTPS recommendation really is
  `!target_url.starts_with("https://")`, and parity wins on a cosmetic
  recommendation where it must not on a security control.

## Consequences

The port is **more secure than the oracle** on this axis and **functionally
equivalent** on every input where the oracle was not exploitable — a followed
chain of public redirects lands in the same place, and the analysis is
unchanged.

The divergences a consumer can observe:

| Input | Oracle | Port |
|---|---|---|
| public URL → public redirect | follows, audits destination | same |
| public URL → private redirect | follows, **returns internal headers** | refused, specific error |
| https URL → http redirect | follows the downgrade | refused |
| redirect with a relative `Location` | resolves and follows | reports the redirect response |
| more than 10 hops | error | error |

Only the second row is a behaviour a caller could be depending on, and
depending on it means depending on the bypass.

Cost: roughly 60 lines of hop loop plus resolution, and the port no longer uses
sandhi's redirect follower, so a future improvement there does not reach this
tool automatically. That is the price of the per-hop hook.

Owning the loop also made the scratch arena's sizing this module's problem
rather than sandhi's. Each exchange allocates a receive buffer of
`max_response_bytes + 1`, so an arena sized for a whole audit would have to
scale with hop count — at 10 hops across two chains that is 22 exchanges. The
arena is therefore reset after every hop and again between the two chains, and
sized for exactly one exchange. What crosses a reset is a snapshot of the
thirteen headers the audit actually reads, copied through the default allocator
by `_agnosai_audit_snapshot`.

The response cap itself is a second inherited-default correction, unrelated to
redirects but found the same way. sandhi defaults `max_response_bytes` to 256
KiB and treats an over-cap response as a hard protocol error; the oracle cannot
fail that way at all, because reqwest's `send()` resolves on the response head
and the body is never read. Left inherited, any homepage over 256 KiB would have
returned "security audit failed" where the oracle returns a full result. The cap
is raised to 2 MiB — deliberate slack, since sandhi negotiates
`Accept-Encoding: identity` and these bytes arrive uncompressed.

**The Rust tree is not being fixed.** `rust-old/` is the frozen parity oracle
and modifying it is out of scope for the port; this ADR is the record that the
divergence is intentional and which direction it runs in. If the Rust line is
ever revived, `analyze_cors` and `run_security_audit` need the same treatment.

## Related

- [ADR 006](006-cx-tool-sandbox.md) — the tool sandbox story this sits inside.
- `src/server_ssrf.cyr` — the guard itself, pulled forward from M6, already
  hardened past the oracle on octal/hex/short-form host spellings for the same
  reason: a bypass in the guard is worth more attention than a bug in the
  thing it guards.
