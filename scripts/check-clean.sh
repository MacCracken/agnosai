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

# --- lib/ actually matches the pinned toolchain snapshot -----------------
# Added 2026-08-03. This is NOT redundant with `deps --verify` above, and the
# difference is the whole reason it exists:
#
#   * `deps --verify` compares cyrius.lock against lib/ on disk. The lock is
#     WRITTEN FROM disk, so it can only catch a lock that has gone stale — never
#     a lib/ that has.
#   * This compares lib/ against ~/.cyrius/versions/<pin>/lib, i.e. against what
#     the pin actually says the stdlib is.
#
# Proven necessary the day it was added: after `cyrius lib sync --full` +
# `cyrius deps` for the 6.5.6 bump, `lib/vani.cyr` was still 1.1.2 against the
# snapshot's 1.1.3 — and `deps --verify` reported "105 verified, 0 failed",
# because the lock had been regenerated from the stale file.
#
# Root cause, confirmed by controlled mutation: **`cyrius lib sync` skips on
# file SIZE, not content.** A same-size edit survives `--full` (a one-character
# swap is not restored); a size-changing edit is synced. A version-comment bump
# like `1.1.2` -> `1.1.3` is exactly size-neutral, so a patch release whose only
# change is the stamp never lands. Filed upstream.
# **Recursive**, and that is not cosmetic. `cyrius lib sync` copies only the
# top level, so the snapshot's `unicode/` subdirectory never lands — and since
# `src/sandbox/oci.cyr` began calling `unicode_category` for oracle-exact
# `is_alphanumeric`, those files are load-bearing and vendored BY HAND. Nothing
# upstream will keep them current, so this gate is the only thing that notices
# when they go stale.
_snap="$HOME/.cyrius/versions/$(grep '^cyrius = ' cyrius.cyml | sed 's/.*"\(.*\)"/\1/')/lib"
if [ -d "$_snap" ]; then
    n=0
    for f in $(cd "$_snap" && find . -name '*.cyr' | sed 's|^\./||' | sort); do
        [ -e "lib/$f" ] || { note "lib: $f missing from lib/"; fail=1; continue; }
        if ! cmp -s "lib/$f" "$_snap/$f"; then
            note "lib: $f differs from the $(basename "$(dirname "$_snap")") snapshot"
            fail=1
        fi
        n=$((n + 1))
    done
    echo "lib snapshot: $n files"
else
    note "lib snapshot: $_snap not found — cannot verify lib/ against the pin"
    fail=1
fi

# --- The coverage corpus fits the tool's buffer ---------------------------
#
# `cyrius coverage` concatenates every `.tcyr` under `tests/` into a **fixed
# 1,048,576-byte** buffer (`cbt/quality.cyr:59`) and substring-searches it. Files
# that do not fit are truncated or dropped, and `if (n > 0)` makes a short read
# indistinguishable from a refused one — so past the cap the tool under-reports
# with no diagnostic and **exit 0**. Measured on this tree: 100% at 1,053,976
# bytes, 99% at 1,057,884, 85% at 1,253,884, every step reporting success.
#
# That is a gate reporting a number it knows is incomplete, so this makes it
# loud. A failure here does NOT mean coverage dropped — it means the coverage
# figure is no longer evidence either way, and must be established another way
# until the corpus fits again.
#
# Filed upstream as
# 2026-08-05-coverage-corpus-is-a-fixed-1-mib-buffer-and-silently-under-reports-past-it.md
_corpus=$(find tests -name '*.tcyr' -exec stat -c%s {} + | awk '{s+=$1} END {print s+0}')
_cap=1048575
echo "coverage corpus: $_corpus bytes (cap $_cap)"
if [ "$_corpus" -gt "$_cap" ]; then
    # **A warning, not a failure, and only because `scripts/check-coverage.sh`
    # covers what the tool cannot.** The overage is real and permanent until
    # upstream fixes the buffer, so failing here would leave a gate red forever
    # with no action available — which is how gates get ignored. What must not
    # happen is the overage going unmentioned, because then `cyrius coverage`'s
    # percentage looks like evidence.
    echo "  WARN coverage corpus: $_corpus bytes exceeds the tool's $_cap-byte"
    echo "       buffer by $((_corpus - _cap)) — \`cyrius coverage\` is truncating and its"
    echo "       percentage is NOT evidence. Use scripts/check-coverage.sh, which"
    echo "       reads the whole corpus. See the upstream filing named above."
fi

if [ "$fail" -ne 0 ]; then
    echo "cleanliness check FAILED"
    exit 1
fi
echo "cleanliness check OK — fmt, lint, doc, vet, deny, deps --verify, lib snapshot, \
coverage corpus all clean"
