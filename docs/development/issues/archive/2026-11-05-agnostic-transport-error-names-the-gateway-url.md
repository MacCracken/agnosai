# A hoosh transport error names the gateway's URL — RESOLVED

**Discovered:** 2026-11-05, by agnostic's 0.2.2 audit. **Severity:** Medium.
**Affects:** agnosai 2.2.3, the hoosh client's transport-error path (`hoosh transport error: <KIND> (<url>)`).
**Status:** ✅ **RESOLVED in agnosai 2.2.4** (2026-11-05). Was 🔴 OPEN.
**Resolution:** the text is `hoosh transport error: <KIND>`, and which gateway goes to a WARN line. That line is rebuilt from sandhi's parsed scheme, host and port, and withheld if the URL holds an `@` or the host holds anything a host name does not, so a password cannot reach it either. **For agnostic:** `_agnostic_scrub_gateway_a` has nothing left to scrub from this path.

That text becomes a task's `output` and `error`, so a consumer's API serves it to any reader and
stores it durably. A URL with userinfo would put the password there too.

**Ask:** say which kind of failure it was and leave the URL to a log line. **agnostic meanwhile**
refuses userinfo at start-up and scrubs its configured URL from results (`_agnostic_scrub_gateway_a`).
