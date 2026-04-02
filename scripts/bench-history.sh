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

# Criterion outputs benchmark name on one line, timing on the next.
# Pattern: "benchmark_name\n                        time:   [low median high]"
# We pair them by tracking the last non-indented line as the benchmark name.
LAST_NAME=""
while IFS= read -r line; do
    # Lines starting with non-space are benchmark names.
    if echo "$line" | grep -qE '^[A-Za-z]' && ! echo "$line" | grep -q 'Benchmarking\|Compiling\|Finished\|Running\|warning\|Found\|Gnuplot'; then
        LAST_NAME=$(echo "$line" | sed 's/\s*$//')
    fi
    # Lines with "time:" contain the measurements.
    if echo "$line" | grep -q 'time:'; then
        # Extract median: second value inside brackets [low median high]
        median=$(echo "$line" | sed 's/.*\[//;s/\].*//' | awk '{print $3 $4}')
        if [ -z "$median" ]; then
            continue
        fi
        # Normalise to nanoseconds.
        if echo "$median" | grep -q 'µs'; then
            ns=$(echo "$median" | sed 's/µs//' | awk '{printf "%.0f", $1 * 1000}')
        elif echo "$median" | grep -q 'ms'; then
            ns=$(echo "$median" | sed 's/ms//' | awk '{printf "%.0f", $1 * 1000000}')
        elif echo "$median" | grep -q 'ns'; then
            ns=$(echo "$median" | sed 's/ns//' | awk '{printf "%.0f", $1}')
        elif echo "$median" | grep -q 'ps'; then
            ns=$(echo "$median" | sed 's/ps//' | awk '{printf "%.0f", $1 / 1000}')
        elif echo "$median" | grep -q 's$'; then
            ns=$(echo "$median" | sed 's/s$//' | awk '{printf "%.0f", $1 * 1000000000}')
        else
            ns="$median"
        fi
        if [ -n "$LAST_NAME" ] && [ "$ns" != "0" ]; then
            echo "${DATE},${VERSION},${LAST_NAME},${ns}" >> "$CSV"
        fi
    fi
done < "$TMPFILE"

ENTRIES=$(grep -c "$DATE" "$CSV" 2>/dev/null || echo 0)
rm -f "$TMPFILE"
echo "Appended $ENTRIES benchmark entries to $CSV"
