#!/usr/bin/env bash
# gen-presets.sh — embed src/presets/*.json into a Cyrius source file.
#
# The oracle does this with `include_str!`, which the Cyrius compiler has no
# equivalent for: `include` is textual and takes a path to *source*, so a data
# file has to be turned into source first. That is all this script does — it is
# a build step written down rather than a design.
#
#   ./scripts/gen-presets.sh            regenerate src/definitions/presets_data.cyr
#   ./scripts/gen-presets.sh --check    fail if the checked-in file has drifted
#
# The generated file IS committed, so a clone builds without running this. The
# `--check` mode is what keeps that copy honest: edit a preset and forget to
# regenerate, and CI says so instead of shipping the stale bytes.
#
# ⚠ THE ORDER IS NOT THE GLOB ORDER. `builtin_presets()` returns a `Vec` and its
# eighteen `include_str!`s go domain by domain, lean → standard → large. Sorted
# alphabetically the list starts `data-engineering-large`, which is neither the
# right domain nor the right size tier. The order lives in PRESETS below and is
# asserted by tests/definitions_loader_presets.tcyr.
#
# Filed as an ergonomics gap upstream:
# cyrius/docs/development/proposals/2026-08-10-embed-data-files-as-source-strings.md

set -uo pipefail
cd "$(dirname "$0")/.." || exit 2

OUT="src/definitions/presets_data.cyr"
MODE="${1:-generate}"

gen=$(python3 - <<'PY'
import json, os, sys

# rust-old/src/definitions/loader.rs, builtin_presets(), in its order.
PRESETS = [
    ("Quality", ["quality-lean", "quality-standard", "quality-large"]),
    ("Software Engineering", ["software-engineering-lean",
                              "software-engineering-standard",
                              "software-engineering-large"]),
    ("DevOps", ["devops-lean", "devops-standard", "devops-large"]),
    ("Data Engineering", ["data-engineering-lean", "data-engineering-standard",
                          "data-engineering-large"]),
    ("Design", ["design-lean", "design-standard", "design-large"]),
    ("Security", ["security-lean", "security-standard", "security-large"]),
]

# One physical line stays under this many characters, because `cyrius lint`
# warns past 120 and the largest document is 2.2 KB.
#
# ⚠ A trailing backslash continues a Cyrius string literal across lines, and it
# DOES keep the newline — then `cyrius fmt` indents the continuation and those
# spaces land inside the string too. Both are invisible here, because a break is
# only ever taken between JSON tokens where whitespace is insignificant. That is
# the whole reason `atoms` splits on token boundaries instead of counting
# characters: a break inside a `"description"` value would splice a newline and
# four spaces into text a consumer displays.
WRAP = 88

flat = [name for _, names in PRESETS for name in names]
missing = [n for n in flat if not os.path.exists(f"src/presets/{n}.json")]
if missing:
    sys.stderr.write("missing preset files: %s\n" % ", ".join(missing))
    sys.exit(2)

extra = sorted(f[:-5] for f in os.listdir("src/presets") if f.endswith(".json"))
if extra != sorted(flat):
    sys.stderr.write(
        "src/presets/ does not match the oracle's list; add it to PRESETS in "
        "scripts/gen-presets.sh so its position in the Vec is deliberate.\n"
        "  on disk but not listed: %s\n" % ", ".join(set(extra) - set(flat)))
    sys.exit(2)


def atoms(text):
    """Split compact JSON into indivisible, already-escaped pieces.

    A piece is a whole string literal or a single non-string character. Breaking
    the line anywhere else would put a newline plus fmt's indentation *inside* a
    value — a `description` a consumer displays would grow whitespace.
    """
    out, i, n = [], 0, len(text)
    while i < n:
        if text[i] == '"':
            j = i + 1
            while j < n:
                if text[j] == "\\":
                    j += 2
                    continue
                if text[j] == '"':
                    break
                j += 1
            piece = text[i:j + 1]
            out.append(piece.replace("\\", "\\\\").replace('"', '\\"'))
            i = j + 1
        else:
            out.append(text[i])
            i += 1
    return out


def literal(text, prefix):
    """A Cyrius string literal, greedily packed to WRAP with continuations."""
    lines, line = [], ""
    room = WRAP - prefix - 1          # the opening quote sits on the first line
    for piece in atoms(text):
        # A trailing backslash continues the literal, so it costs a character.
        if line and len(line) + len(piece) + 1 > room:
            lines.append(line)
            line = ""
            room = WRAP
        line += piece
    lines.append(line)
    return '"' + "\\\n".join(lines) + '"'


L = []
L.append("# " + "=" * 71)
L.append("# definitions/presets_data — GENERATED FILE. Do not edit by hand.")
L.append("#")
L.append("# Regenerate with `./scripts/gen-presets.sh`; that script's `--check` mode")
L.append("# fails when this file has drifted from `src/presets/*.json`.")
L.append("#")
L.append("# The oracle embeds these with eighteen `include_str!`s in")
L.append("# `rust-old/src/definitions/loader.rs`. Cyrius `include` is textual and takes")
L.append("# a path to source, so the documents are turned into source here instead.")
L.append("#")
L.append("# ⚠ The ORDER is the oracle's — domain by domain, lean → standard → large —")
L.append("# and not the alphabetical order of the directory. `builtin_presets()`")
L.append("# returns a `Vec`, so position is observable.")
L.append("#")
L.append("# Each document is whitespace-collapsed to a single logical line. JSON")
L.append("# whitespace between tokens is insignificant, so the parse is unaffected.")
L.append("# " + "=" * 71)
L.append("")
L.append("# How many documents `agnosai_preset_json` will answer for.")
L.append("var AGNOSAI_PRESET_JSON_COUNT = %d;" % len(flat))
L.append("")

idx = 0
for group, names in PRESETS:
    L.append("# --- %s %s" % (group, "-" * max(0, 66 - len(group))))
    for name in names:
        raw = open("src/presets/%s.json" % name).read()
        # Reserialized rather than passed through: it proves the file parses at
        # generation time, and collapses the indentation the sources carry.
        compact = json.dumps(json.loads(raw), separators=(",", ":"))
        L.append("")
        L.append("# %s.json" % name)
        decl = "var _AGNOSAI_PRESET_JSON_%02d = " % idx
        L.append(decl + literal(compact, len(decl)) + ";")
        idx += 1
    L.append("")

L.append("# The `i`th embedded preset document as a NUL-terminated C string, or 0 when")
L.append("# `i` is out of range.")
L.append("#")
L.append("# A chain rather than a table because a `var` array of string literals has no")
L.append("# spelling in Cyrius, and the caller is a one-shot loop over eighteen items.")
L.append("fn agnosai_preset_json(i): i64 {")
for n in range(len(flat)):
    L.append("    if (i == %d) { return _AGNOSAI_PRESET_JSON_%02d; }" % (n, n))
L.append("    return 0;")
L.append("}")
L.append("")
sys.stdout.write("\n".join(L))
PY
)

status=$?
if [ $status -ne 0 ]; then
    echo "gen-presets: generation failed" >&2
    exit $status
fi

# Run the result through `cyrius fmt` before comparing or installing. fmt
# reindents the continuation lines inside the literals, so a raw generation
# would satisfy this script and fail `./scripts/check-clean.sh` — the two gates
# have to be reconciled here or they contradict each other forever.
tmp=$(mktemp /tmp/gen-presets.XXXXXX.cyr)
trap 'rm -f "$tmp"' EXIT
printf '%s' "$gen" > "$tmp"
# `cyrius fmt` formats the file IN PLACE and prints nothing on stdout. This used
# to redirect stdout into "$tmp.fmt" and read that back, which captured an empty
# file — so `gen` was empty, the write path emitted a 1-line
# presets_data.cyr, and `--check` compared 434 lines against nothing and reported
# "stale" unconditionally. Read the formatted file itself.
if ! cyrius fmt "$tmp" >/dev/null 2>&1; then
    echo "gen-presets: cyrius fmt failed on the generated source" >&2
    exit 1
fi
gen=$(cat "$tmp")

if [ "$MODE" = "--check" ]; then
    if [ ! -f "$OUT" ]; then
        echo "gen-presets: $OUT is missing — run ./scripts/gen-presets.sh" >&2
        exit 1
    fi
    if ! printf '%s\n' "$gen" | diff -u "$OUT" - >/dev/null; then
        echo "gen-presets: $OUT is stale — run ./scripts/gen-presets.sh" >&2
        printf '%s\n' "$gen" | diff -u "$OUT" - | head -40 >&2
        exit 1
    fi
    echo "gen-presets: $OUT is up to date"
    exit 0
fi

# `$(...)` strips trailing newlines; fmt wants the file to end with one.
printf '%s\n' "$gen" > "$OUT"
echo "gen-presets: wrote $OUT"
