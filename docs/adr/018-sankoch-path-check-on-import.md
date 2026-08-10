# 018 — sankoch's path check makes `.agpkg` stricter than the oracle

## Status: Accepted

## Context

`rust-old/src/definitions/packaging.rs` reads and writes `.agpkg` ZIP bundles
through the `zip` crate. The Cyrius port reads and writes them through
`lib/sankoch.cyr`, and the two libraries disagree about which member names are
allowed. The disagreement is not a bug in either: it is one library taking a
position the other leaves to its caller.

**The oracle's only filter is one substring test, applied on import, to the
definitions scan only** (`packaging.rs:127`):

```rust
if name.contains("..") { return None; }
```

It sits inside a `filter_map`, so a matching entry is a **silent skip** — the
import still returns `Ok`, with that definition absent. Nothing filters names on
export: `format!("definitions/{}.json", def.agent_key)` (`:70`) interpolates the
key raw.

**sankoch applies `_zip_path_safe` (`lib/sankoch.cyr:14051`) on both sides** —
at `:14774` inside `zip_add_meta` (write) and at `:14541` inside `_zip_prepare`
(read) — and returns `ERR_UNSAFE_PATH` rather than skipping. It rejects a `..`
*component*, a leading `/`, a backslash, a NUL or control byte, and an empty
interior component.

The two filters differ **in both directions**:

| name | oracle | sankoch |
|---|---|---|
| `definitions/a..b.json` | skipped (substring `..`) | accepted (no `..` component) |
| `definitions/../x.json` | skipped | rejected |
| `definitions/a\b.json` | **read and parsed** | rejected |
| `definitions//b.json` | **read and parsed** | rejected |
| `definitions/\x01.json` | **read and parsed** | rejected |

Three of these are reachable only from a hostile or hand-built archive; the
export side is reachable from ordinary code, because an `agent_key` of `../x`
produces `definitions/../x.json`.

## Decision

**Reproduce the oracle's filter exactly, and let sankoch's stand where it fires
past it.**

1. `_agnosai_pkg_is_definition_member` applies the oracle's substring `..` test
   as a **silent skip**, including its false positives (`a..b.json`). The port
   drops exactly what the oracle drops.
2. A name that survives that and then trips sankoch's check surfaces as an
   error rather than being read:
   - on **import**, `agnosai_error_io` — the same variant the oracle raises when
     its own `read_to_string` fails on an unreadable member;
   - on **export**, `agnosai_error_other("zip error: …")` — the same variant the
     oracle raises when `start_file` refuses a name.
3. sankoch's check is **not bypassed**. There is no unchecked extract entry
   point, and adding one to chase parity would mean carrying a zip-slip primitive
   in the tree for the sake of reading a member whose name is already malformed.

Four further differences fall out of the same library swap and are accepted with
it, both in the safe direction:

4. **The 1 MiB per-file guard is real here and advisory in the oracle.**
   `file.size()` is attacker-written central-directory metadata and the `zip`
   crate never enforces it against the decompressed stream, so a bomb declaring
   `uncompressed_size = 10` passes the oracle's guard and then inflates
   unbounded into RAM. sankoch's `zip_extract` bounds the output at `dst_cap`
   and `_zip_verify` (`:14587`) requires the produced length to equal the
   declared one *and* the CRC to match, so the declared size is the real ceiling.
5. **Export is byte-reproducible here and is not upstream.** zip 8's
   `SimpleFileOptions::default()` stamps `OffsetDateTime::now_utc()` into every
   header, so two exports of the same package differ. `zip_add` passes
   `mtime = 0`, so two exports here are identical.
6. **An encrypted member fails the whole import.** `zip_open` rejects an archive
   with the encryption flag set on any member (`lib/sankoch.cyr:14364`), where
   zip 8 opens it and the oracle's `by_index(i).ok()?` silently skips just that
   entry. An archive mixing a valid manifest with an encrypted member is
   adversarial in every realistic case and sankoch's refusal is loud.
7. **Three compression methods are skipped rather than read.** sankoch decodes
   store, deflate, bzip2, zstd and xz; zip 8's default features add deflate64,
   LZMA and PPMd. `_agnosai_pkg_can_decode` turns an undecodable member into the
   oracle's own *silent skip* (`by_index(i).ok()?`) rather than an error, so the
   shape matches even where the set does not.

## Consequences

- An archive whose definition member names are malformed in a way the oracle
  tolerates fails to import, loudly, instead of yielding definitions. The
  affected names cannot be produced by this port's own `export`, and the
  behaviour is a refusal rather than a silent difference.
- An `agent_key` containing a `..` path component, a backslash or a control byte
  makes `export` fail. Upstream it exports and then silently loses that
  definition on the way back in — the oracle's own substring filter drops
  `definitions/../x.json`. A loud failure at write time is the better half of
  that trade, and it is the only one available.
- `.agpkg` files are not byte-compatible between the Rust build and this one —
  pretty-printer differences, metadata key order (serde_json sorts, bayan
  preserves insertion order) and the timestamp above. They remain **format**
  compatible: each side reads what the other writes.
- `tests/definitions_packaging.tcyr` pins every row of the table above, so a
  future sankoch that relaxes or tightens `_zip_path_safe` shows up as a test
  failure rather than a behaviour change.
