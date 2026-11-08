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
  - **OCI** (`src/sandbox/oci.cyr`) delegates to a container runtime. Since
    2.3.4 the run line carries a read-only root, `--network=none` unless the
    policy allows it, `--memory`, `--cap-drop=ALL`,
    `--security-opt=no-new-privileges` and `--pids-limit 256`.
  - **Process/Python is kernel-confined since 2.3.4**
    ([ADR 031](docs/adr/031-confine-the-process-and-python-tiers-in-agnosais-spawner.md)):
    NO_NEW_PRIVS, kavach's seccomp filter, and landlock rules giving read and
    execute on the system's own trees and read and write on `/tmp` and the
    call's work directory, with the binary pinned by descriptor. Through 2.3.3
    it was a plain subprocess with the server's privileges.
    Since 2.3.6 these tools hold no capabilities, even under a root server.
    ⚠ **Still open:** the network is not isolated on this tier; a kernel without landlock runs them
    with no filesystem rules, and the server warns once. There is no memory
    limit and no cgroup. `docs/development/threat-model.md` has the detail.
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
