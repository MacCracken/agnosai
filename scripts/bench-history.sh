#!/usr/bin/env bash
# bench-history.sh — run all benchmarks and append summary to bench-history.csv
set -euo pipefail

CSV="bench-history.csv"
DATE="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
VERSION="$(cat VERSION 2>/dev/null || echo unknown)"

# Create CSV header if missing.
if [ ! -f "$CSV" ]; then
    echo "date,version,benchmark,time_ns" > "$CSV"
fi

echo "Running benchmarks..."
TMPFILE=$(mktemp)
cargo bench --all-features 2>&1 | tee "$TMPFILE"

# Extract timing lines: "benchmark_name  time:   [low median high]"
grep -E '^\S.*time:' "$TMPFILE" | while IFS= read -r line; do
    name=$(echo "$line" | sed 's/\s*time:.*//')
    # Extract median (middle value in brackets).
    median=$(echo "$line" | grep -oP '\[\S+ \K\S+' | head -1)
    # Normalise to nanoseconds.
    if echo "$median" | grep -q 'µs'; then
        ns=$(echo "$median" | sed 's/µs//' | awk '{printf "%.0f", $1 * 1000}')
    elif echo "$median" | grep -q 'ms'; then
        ns=$(echo "$median" | sed 's/ms//' | awk '{printf "%.0f", $1 * 1000000}')
    elif echo "$median" | grep -q 'ns'; then
        ns=$(echo "$median" | sed 's/ns//' | awk '{printf "%.0f", $1}')
    else
        ns="$median"
    fi
    echo "${DATE},${VERSION},${name},${ns}" >> "$CSV"
done

rm -f "$TMPFILE"
echo "Baseline appended to $CSV"
