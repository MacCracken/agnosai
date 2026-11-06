# 027 — The built-in presets are an example library

**Status**: Accepted
**Date**: 2026-11-05

## Context

agnosai embedded 18 presets, and agnostic ships 18 of its own. They share a naming scheme and no content:
15 names collide, not one with a matching description, roster or tool vocabulary. agnostic decided on
2026-08-20 that its library is canonical, and does not call `agnosai_builtin_presets()`. So two libraries
were drifting apart under one set of names
(`docs/development/issues/archive/2026-11-05-agnostic-presets-as-examples.md`).

## Decision

**agnosai embeds three presets, as examples of the format: `security-lean`, `security-standard` and
`security-large`.**

They are the three agnosai names agnostic's library does not use, so after this a name means one thing
across the two. Between them they show the size ladder, two, four and six agents, which is what a reader of
an example needs. The other fifteen documents are deleted, and stay in git history.

`GET /api/v1/presets` keeps its shape and answers these three, in that order. This diverges from the Rust
line's wire, which is why it is an ADR.

## Consequences

- **Positive**: the two libraries cannot collide, and agnosai has no domain content to keep in step.
- **Positive**: `GET /api/v1/presets` parses 3 documents per request rather than 18.
- **Negative**: anything that read one of the fifteen from `GET /api/v1/presets` no longer finds it. No
  consumer in this stack does: agnostic has its own, and thoth, joshua and kiran call none of this. A
  deployment using one can pass it to `agnosai_load_preset_from_file`, the loader being unchanged.
- **Neutral**: the bench rows `builtin_presets_18` and `preset_to_value_18` became `_3`, and do not
  continue the old rows.

## Alternatives considered

- **Keep one preset per domain.** Six documents, and each name in five of them collides.
- **Rename every kept preset `example-*`.** Clear, but it changes names a deployment may hold for no gain
  once no name collides.
- **Remove the presets altogether.** Leaves the format with no worked example, and the route with nothing
  to serve.
