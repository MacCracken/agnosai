#!/usr/bin/env bash
# compare-backends.sh — Run identical crew scenarios against CrewAI and AgnosAI
# and produce a comparison table.
#
# Usage: ./scripts/compare-backends.sh [ROUNDS]
# Requires: curl, jq, both servers running, Ollama with llama3.2:1b
set -euo pipefail

ROUNDS="${1:-3}"
AGNOSAI_URL="http://localhost:8080"
CREWAI_URL="http://localhost:8000"
MODEL="ollama/llama3.2:1b"
API_KEY="bench-key"

echo "# AgnosAI v1.0.0 vs CrewAI Benchmark"
echo ""
echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "Model: $MODEL"
echo "Rounds: $ROUNDS"
echo ""

# AgnosAI crew payload
agnosai_payload() {
    local name="$1" process="$2" task_count="$3"
    local agents='[{"agent_key":"bench-a","name":"Analyst","role":"analyst","goal":"Analyze data","backstory":"Expert analyst","tools":[],"complexity":"low","llm_model":"'$MODEL'"}]'
    local tasks="[]"
    local t=""
    for i in $(seq 1 "$task_count"); do
        [ -n "$t" ] && t="$t,"
        t="$t{\"description\":\"Task $i: Summarize quarterly revenue Q1=1.2M Q2=1.5M Q3=1.1M Q4=1.8M\"}"
    done
    tasks="[$t]"
    echo "{\"name\":\"$name\",\"agents\":$agents,\"tasks\":$tasks,\"process\":\"$process\"}"
}

# CrewAI crew payload
crewai_payload() {
    local name="$1" process="$2" task_count="$3"
    local agents='[{"agent_key":"bench-a","name":"Analyst","role":"analyst","goal":"Analyze data","backstory":"Expert analyst","tools":[],"complexity":"low","llm_model":"'$MODEL'"}]'
    local tasks="[]"
    local t=""
    for i in $(seq 1 "$task_count"); do
        [ -n "$t" ] && t="$t,"
        t="$t{\"description\":\"Task $i: Summarize quarterly revenue Q1=1.2M Q2=1.5M Q3=1.1M Q4=1.8M\",\"expected_output\":\"summary\"}"
    done
    tasks="[$t]"
    echo "{\"name\":\"$name\",\"title\":\"$name\",\"description\":\"benchmark\",\"agents\":$agents,\"tasks\":$tasks,\"process\":\"$process\"}"
}

run_agnosai() {
    local payload="$1"
    local start end elapsed
    start=$(date +%s%N)
    curl -s -X POST "$AGNOSAI_URL/api/v1/crews" \
        -H "Content-Type: application/json" \
        -d "$payload" > /dev/null 2>&1
    end=$(date +%s%N)
    elapsed=$(( (end - start) / 1000000 ))
    echo "$elapsed"
}

run_crewai() {
    local payload="$1"
    local start end elapsed
    start=$(date +%s%N)
    curl -s -X POST "$CREWAI_URL/api/v1/crews" \
        -H "Content-Type: application/json" \
        -H "X-API-Key: $API_KEY" \
        -d "$payload" > /dev/null 2>&1
    end=$(date +%s%N)
    elapsed=$(( (end - start) / 1000000 ))
    echo "$elapsed"
}

scenarios=("single-1:sequential:1" "sequential-3:sequential:3" "parallel-3:parallel:3" "large-6:sequential:6")

echo "| Scenario | Backend | Mean (ms) | Min (ms) | Max (ms) |"
echo "|----------|---------|-----------|----------|----------|"

for scenario in "${scenarios[@]}"; do
    IFS=: read -r name process tasks <<< "$scenario"

    # AgnosAI
    total=0; min=999999; max=0
    for r in $(seq 1 "$ROUNDS"); do
        payload=$(agnosai_payload "bench-$name" "$process" "$tasks")
        ms=$(run_agnosai "$payload")
        total=$((total + ms))
        [ "$ms" -lt "$min" ] && min=$ms
        [ "$ms" -gt "$max" ] && max=$ms
    done
    mean=$((total / ROUNDS))
    echo "| $name | AgnosAI | $mean | $min | $max |"

    # CrewAI
    total=0; min=999999; max=0
    for r in $(seq 1 "$ROUNDS"); do
        payload=$(crewai_payload "bench-$name" "$process" "$tasks")
        ms=$(run_crewai "$payload")
        total=$((total + ms))
        [ "$ms" -lt "$min" ] && min=$ms
        [ "$ms" -gt "$max" ] && max=$ms
    done
    mean=$((total / ROUNDS))
    echo "| $name | CrewAI | $mean | $min | $max |"
done
