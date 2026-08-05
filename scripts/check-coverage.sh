#!/usr/bin/env bash
# check-coverage.sh — reference coverage, computed without a fixed buffer.
#
# ## Why this exists rather than `cyrius coverage --min 80`
#
# `cyrius coverage` answers the same question — is each public symbol in `src/`
# *named* anywhere in the `.tcyr` corpus — by concatenating every test file into
# a **fixed 1,048,576-byte** allocation (`cbt/quality.cyr:59`) and substring-
# searching it. Files past the cap are truncated or dropped, and `if (n > 0)`
# there makes a short read indistinguishable from a refused one. So above 1 MiB
# of tests the tool under-reports **silently, at exit 0**, and the percentage it
# prints stops being evidence in either direction.
#
# This tree crossed that on 2026-08-05 and the effect was immediate and
# misleading: `cyrius coverage` reported 99% and named `sandbox/python.cyr` and
# `routes/approval.cyr` as carrying unreferenced functions. Every symbol in both
# is referenced by a test. Nothing had regressed; the corpus no longer fit.
#
# Filed upstream as
# `2026-08-05-coverage-corpus-is-a-fixed-1-mib-buffer-and-silently-under-reports-past-it.md`
# with the root cause and three ordered fixes. Until one lands, the gate has to
# come from somewhere that can read the whole corpus.
#
# **The definition is deliberately identical**, so the two agree whenever the
# tool is able to answer: every `^fn name` in `src/**/*.cyr` is the denominator,
# and a symbol counts as covered when its name appears anywhere in the corpus.
# That means this inherits the tool's weakness as well as its scope — reference
# coverage counts whether a symbol is *named*, not whether an assertion would
# fail without it. It is a floor, not a correctness proof; the M7 audit found 43
# live defects behind a green 100%.
#
# `_`-prefixed internals are excluded from the denominator, matching the tool
# and matching CLAUDE.md's rule for marking something a genuine internal.
set -euo pipefail

MIN="${1:-80}"

python3 - "$MIN" <<'PY'
import os, re, sys

min_pct = float(sys.argv[1])

corpus = []
for root, _, files in os.walk("tests"):
    for f in files:
        if f.endswith(".tcyr"):
            with open(os.path.join(root, f), encoding="utf-8", errors="replace") as fh:
                corpus.append(fh.read())
corpus = "\n".join(corpus)

# Every public `fn` in src/, recursively — `find src -name '*.cyr'`, never
# `src/*.cyr`, which matches nothing since the tree mirrors rust-old/.
syms = []
for root, _, files in os.walk("src"):
    for f in sorted(files):
        if not f.endswith(".cyr"):
            continue
        path = os.path.join(root, f)
        with open(path, encoding="utf-8", errors="replace") as fh:
            for line in fh:
                m = re.match(r"fn ([A-Za-z][A-Za-z0-9_]*)", line)
                if m:
                    syms.append((path, m.group(1)))

missing = [(p, s) for p, s in syms if s not in corpus]
total = len(syms)
covered = total - len(missing)
pct = 100.0 * covered / total if total else 100.0

# **Floored, never rounded.** At 1074/1075 the rounded figure is "100%", which
# would report full coverage while one symbol is unreferenced — the display
# contradicting the fraction printed beside it. `cyrius coverage` floors for the
# same reason. The comparison against --min uses the exact value.
shown = int(pct) if pct < 100.0 else 100

by_file = {}
for p, s in missing:
    by_file.setdefault(p, []).append(s)
for p in sorted(by_file):
    for s in sorted(by_file[p]):
        print(f"  UNREFERENCED  {p}  {s}")

corpus_bytes = sum(
    os.path.getsize(os.path.join(r, f))
    for r, _, fs in os.walk("tests")
    for f in fs
    if f.endswith(".tcyr")
)
print(f"corpus: {corpus_bytes} bytes across the whole of tests/ — no buffer limit")
print(f"reference coverage: {covered}/{total} ({shown}%)  "
      f"[a floor, not a correctness proof]")

if pct < min_pct:
    print(f"coverage check FAILED — {shown}% < --min {min_pct:.0f}%")
    sys.exit(1)
print(f"coverage check OK — {shown}% >= {min_pct:.0f}%")
PY
