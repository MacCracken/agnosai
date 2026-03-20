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

- **Tool execution**: Sandboxed tools run under WASM (wasmtime) or subprocess
  isolation with seccomp-bpf, Landlock, and cgroups. Native tools run in-process
  and should be reviewed carefully before registration.
- **LLM provider calls**: HTTP requests are made to configured provider
  endpoints. Ensure provider URLs and API keys are sourced from trusted
  configuration.
- **Serialization**: Agent definitions and crew specs derive `Serialize` and
  `Deserialize`. If you deserialize untrusted input, apply your own validation
  layer.
- **Fleet communication**: When fleet features are enabled, inter-node messages
  traverse the network. Deploy behind authenticated transports in production.
