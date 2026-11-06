# A hoosh transport error names the gateway's URL — OPEN

**Discovered:** 2026-11-05, by agnostic's 0.2.2 audit. **Severity:** Medium.
**Affects:** agnosai 2.2.3, the hoosh client's transport-error path (`hoosh transport error: <KIND> (<url>)`).
**Status:** 🔴 OPEN.

That text becomes a task's `output` and `error`, so a consumer's API serves it to any reader and
stores it durably. A URL with userinfo would put the password there too.

**Ask:** say which kind of failure it was and leave the URL to a log line. **agnostic meanwhile**
refuses userinfo at start-up and scrubs its configured URL from results (`_agnostic_scrub_gateway_a`).
