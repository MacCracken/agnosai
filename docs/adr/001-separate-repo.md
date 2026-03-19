# ADR-001: AgnosAI as a Separate Repository

**Status:** Accepted
**Date:** 2026-03-18

## Context

Agnostic v1 uses CrewAI as an embedded Python dependency. AgnosAI replaces CrewAI with a Rust-native orchestration engine. The question is whether AgnosAI should live inside the Agnostic repo or as a standalone project.

## Decision

AgnosAI lives in its own repository (`agnosai/`) as a standalone Rust workspace. Agnostic depends on it as a library — the same relationship CrewAI has today.

## Rationale

- **Reusability** — any Rust project can depend on `agnosai-core` without pulling in Agnostic's Python stack
- **Independent versioning** — AgnosAI can release on its own cadence
- **Clean dependency direction** — Agnostic depends on AgnosAI, not the reverse
- **Build isolation** — Rust compilation doesn't block Python CI and vice versa

## Consequences

- Agnostic adds AgnosAI as a git or crates.io dependency
- SecureYeoman's relationship is unchanged — it talks to Agnostic via A2A/MCP
- AgnosAI needs its own CI, releases, and documentation
