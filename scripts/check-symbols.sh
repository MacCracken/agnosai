#!/usr/bin/env bash
# check-symbols.sh — guard the flat namespace.
#
# Cyrius has ONE global symbol table and last-definition-wins. The compiler
# warns on a duplicate `fn` and is **silent** on a duplicate `var` or a duplicate
# enum member, so `grep "duplicate fn"` on the build log — the obvious check —
# structurally cannot catch the worst case.
#
# That case is real: an audit on 2026-07-31 found four duplicated enum constants
# in src/, three with DIFFERENT values, three of them struct sizes passed
# straight to `alloc()`. Nothing misbehaved, but only because each file's own
# uses happened to be parsed before the redefinition — reordering the includes
# in src/main.cyr would have silently under-allocated a heap struct.
#
# Two rules, both cheap:
#   1. Every top-level symbol in src/ is `agnosai_*` / `_agnosai_*` / `_`-internal
#      / `AGN*` / `AGNOSAI_*`, or the entry point `main`. That single rule closes
#      the entire 180-name unprefixed export surface of lib/bote-core.cyr at once,
#      with no list to maintain.
#   2. No name is defined twice anywhere in src/, across ALL definition kinds:
#      fn, var, and enum members.
#
# Exits non-zero on any violation. Run from the repo root.

set -uo pipefail
cd "$(dirname "$0")/.." || exit 2

fail=0

# --- Rule 2: duplicate definitions of any kind ---------------------------
dupes=$(python3 - <<'PY'
import re, glob, collections
defs = collections.defaultdict(list)
for path in sorted(glob.glob("src/**/*.cyr", recursive=True)):
    in_enum = None
    for lineno, line in enumerate(open(path), 1):
        m = re.match(r'^enum\s+(\w+)\s*\{', line)
        if m:
            in_enum = m.group(1)
            defs[m.group(1)].append((path, lineno, "enum"))
            continue
        if in_enum is not None and line.strip() == "}":
            in_enum = None
            continue
        if in_enum is not None:
            mm = re.match(r'^\s*(\w+)\s*=', line)
            if mm:
                defs[mm.group(1)].append((path, lineno, f"member of {in_enum}"))
            continue
        mv = re.match(r'^var\s+(\w+)\s*=', line)
        if mv:
            defs[mv.group(1)].append((path, lineno, "var"))
        mf = re.match(r'^fn\s+(\w+)\s*\(', line)
        if mf:
            defs[mf.group(1)].append((path, lineno, "fn"))
for name, sites in sorted(defs.items()):
    if len(sites) > 1:
        print(f"DUPLICATE: {name}")
        for path, lineno, kind in sites:
            print(f"    {path}:{lineno}  ({kind})")
PY
)
if [ -n "$dupes" ]; then
    echo "$dupes"
    echo
    echo "error: a symbol is defined more than once in src/."
    echo "       Cyrius resolves these last-definition-wins, and warns only for 'fn'."
    fail=1
fi

# --- Rule 1: every top-level symbol is prefixed ---------------------------
unprefixed=$(python3 - <<'PY'
import re, glob
bad = []
for path in sorted(glob.glob("src/**/*.cyr", recursive=True)):
    in_enum = False
    for lineno, line in enumerate(open(path), 1):
        if re.match(r'^enum\s+\w+\s*\{', line):
            in_enum = True
        elif in_enum and line.strip() == "}":
            in_enum = False
        m = re.match(r'^(?:fn|var)\s+(\w+)', line)
        if not m:
            continue
        name = m.group(1)
        if name == "main":
            continue
        if name.startswith(("agnosai_", "_agnosai_", "AGNOSAI_", "AGN_", "_")):
            continue
        bad.append(f"    {path}:{lineno}  {name}")
if bad:
    print("\n".join(bad))
PY
)
if [ -n "$unprefixed" ]; then
    echo "$unprefixed"
    echo
    echo "error: a top-level symbol in src/ is not agnosai-prefixed."
    echo "       lib/ exports ~180 unprefixed names (bote-core alone); an unprefixed"
    echo "       symbol here can silently replace one of them for the whole build."
    fail=1
fi

# --- Rule 4: no lib/ <-> lib/ constant is defined twice with different values ---
# Rules 1-3 scan src/. They cannot see a collision BETWEEN TWO DEPENDENCIES, and
# neither can the compiler: cycc warns on a duplicate `fn` and is SILENT for `var`
# and for enum members. That blind spot is not hypothetical — it is how kavach's
# `var BACKEND_COUNT = 10` was silently resolved to ai-hwaccel's 18, admitting ids
# 0-17 into a 10-slot function-pointer table and calling whatever sat 224 bytes past
# its end. Nothing in this repo, or in any sibling, would have reported it; see
# CHANGELOG 2.0.4.
#
# The check resolves the REAL compile set from cyrius.cyml — [deps].stdlib, each
# dist's .deps sidecar, and every dep dist lib/ carries, including transitive ones
# no [deps.*] block names (libro arrives via bote) — and excludes the per-platform
# stdlib variants, which define the same names on purpose and never co-compile.
#
# Verified against the historical defect: with kavach 3.11.14 and ai-hwaccel 2.3.17
# restored, it reports BACKEND_COUNT 18-vs-10 and fails.
#
# Divergences that are known, filed upstream and unreachable from src/ are listed in
# scripts/lib-symbol-allow.txt, each with its issue reference.
if ! python3 scripts/check-lib-symbols.py; then
    fail=1
fi

if [ "$fail" -eq 0 ]; then
    n=$(find src -name '*.cyr' -exec grep -hcE '^(fn|var|enum) ' {} + | awk '{s+=$1} END {print s}')
    echo "symbol check OK — $n top-level definitions across $(find src -name "*.cyr" | wc -l) files, no duplicates, all prefixed"
fi
exit "$fail"
