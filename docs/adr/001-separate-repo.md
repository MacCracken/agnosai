# ADR-001: AgnosAI as a Separate Repository

## Status: Accepted

**Date**: 2026-03-18 (Rust era) — re-verified against the Cyrius tree 2026-08-11.

## Context

Agnostic v1 uses CrewAI as an embedded Python dependency. AgnosAI replaces CrewAI with a native orchestration engine. The question is whether AgnosAI should live inside the Agnostic repo or as a standalone project.

## Decision

AgnosAI lives in its own repository (`agnosai/`) as a standalone project. Agnostic consumes it rather than vendoring it.

## Rationale

- **Reusability** — any consumer can use AgnosAI without pulling in Agnostic's Python stack
- **Independent versioning** — AgnosAI can release on its own cadence
- **Clean dependency direction** — Agnostic depends on AgnosAI, not the reverse
- **Build isolation** — AgnosAI's compilation doesn't block Python CI and vice versa

## Consequences

- SecureYeoman's relationship is unchanged — it talks to Agnostic via A2A/MCP
- AgnosAI needs its own CI, releases, and documentation. It has them: `.github/workflows/{ci,release}.yml`, `CHANGELOG.md`, `docs/`

## What the port changed — the consumption mechanism, not the repo split

The separate-repo decision itself is untouched by the Rust → Cyrius port. Two of
the sentences underneath it are no longer true, and one of them was never true.

**AgnosAI is now a BINARY ONLY. There is no library artifact to depend on.**

The original text read *"a standalone Rust workspace. Agnostic depends on it as a
library — the same relationship CrewAI has today."* That described the Rust tree,
where `rust-old/Cargo.toml` declares one crate with both a lib target
(`rust-old/src/lib.rs`) and `[[bin]] agnosai-server`, so `agnosai = "1.1.0"` was a
real dependency line.

The Cyrius tree publishes nothing linkable:

- `cyrius.cyml` has `[build] entry = "src/main.cyr"`, `output = "build/agnosai"`
  and `[release] bins = ["agnosai"]` — no `modules` export stanza.
- There is no `dist/` directory, and no `cyrius distlib` invocation anywhere in
  `scripts/` or `.github/workflows/`. Every AGNOS sibling that *is* consumed as a
  library ships one (`[deps.majra] modules = ["dist/majra.cyr"]` and friends);
  agnosai ships none.
- CLAUDE.md's Project Identity line names the type outright: *"Port (Rust →
  Cyrius). Module tree mirrors `rust-old/` — **binary**"*.

So consumers reach agnosai over its **server surface** — the REST/MCP/A2A/SSE API
in `src/server/` — not by linking it. That is a genuine change in the relationship
this ADR describes, and CLAUDE.md was the only place recording it until now.

**`agnosai-core` was real, and this ADR was accurate the day it was written.**
An earlier pass of this review asserted the crate "never existed" and that
"nothing in the tree records" whether a split was planned — both wrong, and the
second is a manufactured absence of evidence. `git show a0d629f:Cargo.toml` is a
`[workspace]` with **nine** members: `crates/agnosai-{core,orchestrator,llm,
fleet,sandbox,tools,learning,server,definitions}`. `agnosai-core`'s own manifest
described it as "Core types, traits, and error handling for AgnosAI agent
orchestration", and `CHANGELOG.md` [0.20.3] still carries the per-crate headings.

The workspace was flattened into a single crate two days after this ADR, in
`6782120` ("flatten directory", **2026-03-20**), which is why `rust-old/`
declares `name = "agnosai"`, singular, with no `[workspace]` stanza. So the
Reusability bullet is **history, not error** — it is left as written and dated,
because an ADR records the decision as taken. The flatten is the thing that went
unrecorded, and this note is that record.

(The name still appears in `docs/guides/crew-patterns.md` and
`docs/guides/adding-providers.md`, which are Rust-era guides outside this ADR's
scope.)

**The consumer list has grown.** This ADR names Agnostic and SecureYeoman.
CLAUDE.md's Consumers line today reads Agnostic, daimon, joshua and kiran. The
SecureYeoman → Agnostic → AgnosAI chain is asserted in
`docs/architecture/overview.md` and is not verifiable from this repo's code either
way; it is left as written.
