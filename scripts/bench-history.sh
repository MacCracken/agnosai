#!/usr/bin/env bash
# bench-history.sh — run every Cyrius benchmark and append the results to
# bench-history.csv.
#
# `cyrius bench` already does most of what the Rust-era script had to do by
# hand: it discovers the harnesses, runs them, times them, and prints one
# self-describing line per benchmark. So this script only has to normalise the
# unit and stamp each row with the date and version.
#
# The Rust-era harness is frozen at rust-old/scripts/bench-history.sh. It drove
# `cargo bench` and reconstructed names from criterion's two output layouts —
# none of which applies here. Do NOT compare rows across the two files: the
# allocator, harness and statistics are all different, and the Cyrius line
# starts its own baseline. See CLAUDE.md.
#
# Output line shape this parses (lib/bench.cyr `bench_report`):
#   "  ucb1_select_50arms: 1.154us avg (min=1.154us max=1.154us) [20000 iters]"
set -euo pipefail

CSV="bench-history.csv"
DATE="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
VERSION="$(cat VERSION 2>/dev/null || echo unknown)"

# Create CSV header if missing. Column names match the Rust-era file so the two
# can be read by the same tooling, even though the rows are not comparable.
if [ ! -f "$CSV" ]; then
    echo "date,version,benchmark,time_ns" > "$CSV"
fi

TMPFILE=$(mktemp)
trap 'rm -f "$TMPFILE"' EXIT

echo "Running benchmarks..."
cyrius bench 2>&1 | tee "$TMPFILE"

# One row per reported benchmark. The pattern is anchored on the "<value><unit>
# avg" shape so build notes, shadow-lib warnings and duplicate-fn warnings on
# the same stream cannot be mistaken for results.
grep -E '^[[:space:]]+[A-Za-z_][A-Za-z0-9_]*: [0-9.]+(ns|us|ms|s) avg' "$TMPFILE" \
    | while IFS= read -r line; do
        name=$(echo "$line" | sed -E 's/^[[:space:]]*([A-Za-z_][A-Za-z0-9_]*):.*/\1/')
        value=$(echo "$line" | sed -E 's/^[^:]*:[[:space:]]*([0-9.]+)(ns|us|ms|s) avg.*/\1/')
        unit=$(echo "$line" | sed -E 's/^[^:]*:[[:space:]]*([0-9.]+)(ns|us|ms|s) avg.*/\2/')

        # Normalise to nanoseconds. ERE alternation is leftmost-longest, so
        # "ns"/"us"/"ms" always win over the bare "s" branch.
        case "$unit" in
            ns) scale=1 ;;
            us) scale=1000 ;;
            ms) scale=1000000 ;;
            s)  scale=1000000000 ;;
            *)  continue ;;
        esac
        ns=$(awk -v v="$value" -v s="$scale" 'BEGIN { printf "%.0f", v * s }')

        if [ -n "$name" ] && [ "$ns" != "0" ]; then
            echo "${DATE},${VERSION},${name},${ns}" >> "$CSV"
        fi
    done

ENTRIES=$(grep -c "^${DATE}," "$CSV" || echo 0)
echo "Appended $ENTRIES benchmark entries to $CSV"
