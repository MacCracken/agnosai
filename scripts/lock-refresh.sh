#!/usr/bin/env bash
# lock-refresh.sh — regenerate cyrius.lock the way CI will.
#
# ## Why this exists
#
# `cyrius.cyml` carries `path = "../NAME"` beside each `[deps.NAME]` git+tag, so
# a developer with sibling checkouts builds against their working copies. That is
# deliberate and useful — but the path WINS over the tag, so a local
# `cyrius deps` writes a lock with **no commit pins**, while a CI runner (which
# has no siblings) resolves from the tags and writes one **with** them.
#
# CI's "Lockfile is honest" step is `git diff --exit-code -- cyrius.lock`. A
# path-shaped lock therefore fails it every time.
#
# ⚠ **Any cyrius invocation that resolves deps rewrites the lock** — not just
# `cyrius deps`, but a bare `cyrius build`, `cyrius tests`, even
# `cyrius tests --help`. Measured: a tag-only lock has 8 `commit` lines; one
# bare build takes it to 0. So this cannot be "run it once at the start" — it is
# the LAST thing you do before committing, after all building and testing.
#
# ## Usage
#
#   ./scripts/lock-refresh.sh     # then `git add cyrius.lock` and commit
#
# It restores cyrius.cyml from a backup rather than `git checkout`, so it is safe
# to run with uncommitted manifest edits in the tree.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

CYML="cyrius.cyml"
LOCK="cyrius.lock"

[ -f "$CYML" ] || { echo "ERROR: no $CYML"; exit 1; }

# ⚠ Nothing else may be resolving deps while this runs. A concurrent
# `cyrius build` / `cyrius tests` — a background full-suite gate, say — rewrites
# the lock path-shaped underneath us, and the result looks like this script is
# non-deterministic when it is not. Observed exactly that on 2026-08-14.
if pgrep -x cycc >/dev/null 2>&1; then
    echo "ERROR: a cyrius compile is running (cycc). Wait for it to finish —"
    echo "       it will rewrite cyrius.lock path-shaped underneath this script."
    exit 1
fi

BACKUP="$(mktemp)"
trap 'cp "$BACKUP" "$CYML"; rm -f "$BACKUP"' EXIT
cp "$CYML" "$BACKUP"

PATHS="$(grep -c '^path = "\.\./' "$CYML" || true)"
echo "Refreshing $LOCK as a clean tag-only resolution (${PATHS} sibling override(s) held aside)..."

# Drop the sibling overrides so every [deps.NAME] resolves from git + tag,
# exactly as a runner does.
sed -i '/^path = "\.\.\//d' "$CYML"

cyrius deps >/dev/null

# The manifest comes back via the EXIT trap; the lock stays as produced.
cp "$BACKUP" "$CYML"

PINS="$(grep -c '^commit' "$LOCK" || true)"
if [ "$PINS" -eq 0 ]; then
    echo "ERROR: $LOCK has no commit pins — the tag-only resolution did not happen."
    echo "       Check that every [deps.NAME] has a \`tag =\` and that the tags are pushed."
    exit 1
fi

echo "  ${PINS} commit pins written:"
grep '^commit' "$LOCK" | awk '{printf "    %-12s %s\n", $3, $5}'
echo
echo "Now commit it WITHOUT building again:"
echo "    git add $LOCK && git commit"
echo
echo "⚠ If you build or test after this, re-run the script — the lock will have"
echo "  been rewritten path-shaped and CI will reject it."
