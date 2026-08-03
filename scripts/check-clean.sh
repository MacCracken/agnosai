#!/usr/bin/env bash
# The cleanliness gate: fmt, lint, doc, vet, deny, deps --verify.
#
# CLAUDE.md's work loop runs these at steps 2 and 6, but until 2026-07-31 CI ran
# none of them — only the symbol check, build, and test. The drift that caused
# was not hypothetical: `cyrius doc --check` had accumulated **31 undocumented
# public symbols** across five modules, and `cyrius lint` four untracked
# deferrals, none of which any pipeline would ever have reported.
#
# `cyrius fmt`, `cyrius lint` and `cyrius doc` all take a FILE. Written bare they
# print usage and exit 1 — which a gate that only checks the exit code reads as
# a failure, and a gate that ignores it reads as a pass over zero files. Both
# are wrong, so this loops explicitly.
set -uo pipefail

fail=0
note() { printf '  %s\n' "$1"; }

# --- fmt: src/ AND tests/ ------------------------------------------------
# Test files are covered deliberately. CLAUDE.md calls this out because a
# `src/`-only sweep is the easy mistake, and `.tcyr` files are two-fifths of the
# tree by count.
n=0
for f in $(find src -name "*.cyr" | sort) $(find tests -name "*.tcyr" | sort); do
    [ -e "$f" ] || continue
    n=$((n + 1))
    if ! cyrius fmt "$f" --check >/dev/null 2>&1; then
        note "fmt: $f"
        fail=1
    fi
done
echo "fmt: $n files"

# --- lint: warnings and untracked deferrals ------------------------------
# A deferral comment ("deferred", "not yet", "TODO") must cross-reference a
# CHANGELOG, issue, or roadmap entry on the SAME line, or carry `#skip-lint`.
# The rule is what keeps a deferral from quietly becoming permanent.
n=0
for f in $(find src -name "*.cyr" | sort); do
    [ -e "$f" ] || continue
    n=$((n + 1))
    out=$(cyrius lint "$f" 2>&1)
    d=$(printf '%s' "$out" | grep -oE '^[0-9]+ untracked' | grep -oE '^[0-9]+' || echo 0)
    w=$(printf '%s' "$out" | grep -oE '^[0-9]+ warnings' | grep -oE '^[0-9]+' || echo 0)
    if [ "${d:-0}" != "0" ] || [ "${w:-0}" != "0" ]; then
        note "lint: $f — ${d:-0} untracked deferral(s), ${w:-0} warning(s)"
        printf '%s\n' "$out" | grep -E 'deferral line|warning' | sed 's/^/      /'
        fail=1
    fi
done
echo "lint: $n files"

# --- doc: every public symbol documented ---------------------------------
n=0
for f in $(find src -name "*.cyr" | sort); do
    [ -e "$f" ] || continue
    n=$((n + 1))
    if ! out=$(cyrius doc --check "$f" 2>&1); then
        note "doc: $f"
        printf '%s\n' "$out" | grep 'undocumented:' | sed 's/^/      /'
        fail=1
    fi
done
echo "doc: $n files"

# --- vet + deny: the dependency gates ------------------------------------
if ! cyrius vet src/main.cyr >/dev/null 2>&1; then
    note "vet: src/main.cyr"
    fail=1
fi
if ! cyrius deny src/main.cyr >/dev/null 2>&1; then
    note "deny: src/main.cyr"
    fail=1
fi
echo "vet + deny: ok"

# --- deps --verify: cyrius.lock describes the lib/ actually on disk ------
# Added 2026-08-03 after finding lib/kavach.cyr was 3.11.0 while cyrius.lock
# recorded 3.9.3's sha256 and cyrius.cyml pinned `tag = "3.9.3"`. Nothing
# reported it: every other gate reads src/, and the build happily compiles
# whatever bytes lib/ holds.
#
# The mechanism is that each [deps.NAME] carries `path = "../NAME"` alongside
# `git`/`tag`, and the local path WINS. So a developer whose sibling checkout
# has moved ahead builds against a version the manifest does not name, while CI
# — which has no sibling checkouts — resolves the tag and builds something
# else. Here that was kavach 3.11.0 locally against 3.9.3 in CI.
#
# Mutation-verified: restoring the stale 3.9.3 hash makes this print
# `FAIL: lib/kavach.cyr (hash mismatch)` and exit 1.
if ! out=$(cyrius deps --verify 2>&1); then
    note "deps --verify: cyrius.lock does not match lib/"
    printf '%s\n' "$out" | grep -E 'FAIL|failed' | sed 's/^/      /'
    fail=1
fi
echo "deps --verify: $(printf '%s' "$out" | grep -oE '^[0-9]+ verified' || echo 'ok')"

if [ "$fail" -ne 0 ]; then
    echo "cleanliness check FAILED"
    exit 1
fi
echo "cleanliness check OK — fmt, lint, doc, vet, deny, deps --verify all clean"
