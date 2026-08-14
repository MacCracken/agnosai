# Security Policy

## Scope

AgnosAI orchestrates AI agents, executes LLM provider HTTP calls, and optionally
runs tools in sandboxed environments (WASM, subprocess). Bugs in input handling,
tool execution, or network communication could have security implications.

## Supported versions

Only the latest released version receives security fixes.

| Version | Supported |
|---|---|
| Latest | Yes |
| Older | No |

## Reporting a vulnerability

**Do not open a public issue for security vulnerabilities.**

Instead, please report vulnerabilities privately via
[GitHub Security Advisories](https://github.com/maccracken/agnosai/security/advisories/new)
or by emailing the maintainer directly.

Include:

- A description of the vulnerability.
- Steps to reproduce or a proof of concept.
- The potential impact.

You should receive an acknowledgement within 72 hours. We aim to release a fix
within 14 days of confirmation.

## Security considerations

- **Tool execution** — the tiers differ, and the difference matters:
  - **WASM** (`src/sandbox/wasm.cyr`) is the confined tier. Modules run under
    `wasmtime` with a **fuel budget** (CPU) and a **memory ceiling**, and kavach
    wraps the `wasmtime` process itself. WASI is stdio-only: no `--dir` preopen
    is emitted, so the guest has no filesystem. See
    [ADR 019](docs/adr/019-wasm-tools-spawn-wasmtime-directly.md).
  - **OCI** (`src/sandbox/oci.cyr`) delegates to a container runtime, which
    provides whatever isolation that runtime provides.
  - ⚠ **Process/Python is NOT kernel-confined.** It is a subprocess with a
    sanitized environment, a working directory and a SIGKILL deadline —
    **no seccomp-bpf, no Landlock, no cgroups, no network namespace**. This
    document previously claimed otherwise; the claim was false.
    `src/sandbox/python.cyr` says so in its own header, and the oracle's
    `python.rs:4` names that confinement as "(future)". kavach can supply it,
    but it is not wired into this tier. **Treat a process-tier tool as running
    with your server's privileges** and review it as you would a native tool.
  - **Native** tools run in-process and should be reviewed carefully before
    registration.
- **LLM provider calls**: HTTP requests are made to configured provider
  endpoints. Ensure provider URLs and API keys are sourced from trusted
  configuration.
- **Deserialization**: agent definitions and crew specs are parsed from JSON and
  YAML. The parsers reject a record missing a required field rather than
  substituting a default, but they do not validate semantics — if you accept
  definitions from untrusted sources, apply your own validation layer.
- **Rate limiting** is mounted by default at 100 req/s per client key
  (burst 200) and can be disabled with `AGNOSAI_RATE_LIMIT=0`. ⚠ The key is
  derived from `X-Forwarded-For`/`X-Real-IP`, which are client-settable: behind a
  proxy that OVERWRITES them it is trustworthy, but a proxy that APPENDS (nginx's
  documented `$proxy_add_x_forwarded_for`) lets a client choose its own key — and
  therefore drain a named victim's bucket. `/health`, `/ready` and `/metrics` are
  exempt so a flood cannot take out a liveness probe. See
  [ADR 021](docs/adr/021-rate-limit-mounted-by-default.md).
- **Outbound requests** (A2A callbacks, remote tool packages, the security-audit
  tool) go through an SSRF gate that refuses private and loopback addresses and
  re-checks **every redirect hop**, refusing an HTTPS-to-HTTP downgrade.
- **Fleet communication**: When fleet features are enabled, inter-node messages
  traverse the network. Deploy behind authenticated transports in production.
