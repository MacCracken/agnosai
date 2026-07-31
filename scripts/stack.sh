#!/usr/bin/env bash
# stack.sh — dev convenience: bring up the AGNOS backend services agnosai needs
# for live testing, and verify the seam with one command.
#
# Modelled on thoth/scripts/stack.sh (same helpers, same $STACK_HOME layout) but
# scoped to what agnosai actually consumes. Today that is **hoosh only**: the
# port plan makes hoosh a remote HTTP seam rather than a linked crate, so
# `src/llm/hoosh.cyr` POSTs OpenAI-compatible chat completions at it. daimon and
# bote enter when agnosai grows tool calling (M4) and its own server (M6) —
# the hooks are marked TODO below rather than started early.
#
# Everything that must persist (logs, pidfiles, runtime config) lives under
# $STACK_HOME (default ~/.agnos-stack), OUTSIDE the repos — shared with thoth's
# stack.sh on purpose, so both can drive one hoosh.
#
# Usage:
#   scripts/stack.sh up       start hoosh
#   scripts/stack.sh check    live chat-completion round trip (the M3 exit check)
#   scripts/stack.sh status   show what's listening
#   scripts/stack.sh down     stop the services
#   scripts/stack.sh logs     tail each service log
#
# Override via env: AGNOS_STACK_HOME, HOOSH_DIR, AGNOS_KEY_FILE, HOOSH_PORT,
# AGNOSAI_MODEL.

set -u

STACK_HOME="${AGNOS_STACK_HOME:-$HOME/.agnos-stack}"
HOOSH_DIR="${HOOSH_DIR:-$HOME/Repos/hoosh}"
AGNOSAI_DIR="${AGNOSAI_DIR:-$HOME/Repos/agnosai}"
KEY_FILE="${AGNOS_KEY_FILE:-$HOME/.ssh/.api_keys}"
HOOSH_PORT="${HOOSH_PORT:-8088}"
# The default_model() Fast tier from src/llm/router.cyr. Override for a
# provider-backed model (e.g. AGNOSAI_MODEL=claude-opus-4-8).
MODEL="${AGNOSAI_MODEL:-llama3}"

RUN_DIR="$STACK_HOME/run"
LOG_DIR="$STACK_HOME/logs"

c_g=$'\033[32m'; c_r=$'\033[31m'; c_y=$'\033[33m'; c_d=$'\033[2m'; c_0=$'\033[0m'
say()  { printf '%s\n' "$*"; }
ok()   { printf "  ${c_g}✓${c_0} %s\n" "$*"; }
warn() { printf "  ${c_y}!${c_0} %s\n" "$*"; }
err()  { printf "  ${c_r}✗${c_0} %s\n" "$*" >&2; }

# PID listening on a TCP port (via ss, else lsof), empty if none.
port_pid() {
  local p="$1"
  if command -v ss >/dev/null 2>&1; then
    ss -tlnpH "sport = :$p" 2>/dev/null | grep -oE 'pid=[0-9]+' | head -1 | cut -d= -f2
  elif command -v lsof >/dev/null 2>&1; then
    lsof -tiTCP:"$p" -sTCP:LISTEN 2>/dev/null | head -1
  fi
}
listening() { [ -n "$(port_pid "$1")" ]; }

# Poll until a port is listening. curl's connect-timeout paces the loop, so this
# needs no sleep and stays safe under restricted shells.
wait_listen() {
  local p="$1" n="${2:-40}" i=0
  while [ "$i" -lt "$n" ]; do
    listening "$p" && return 0
    i=$((i + 1))
    curl -s -o /dev/null --max-time 0.25 "http://127.0.0.1:$p" 2>/dev/null || true
  done
  return 1
}

load_key() {
  if [ -z "${ANTHROPIC_API_KEY:-}" ] && [ -f "$KEY_FILE" ]; then
    set +u; . "$KEY_FILE" >/dev/null 2>&1 || true; set -u
  fi
}

# start_svc <name> <workdir> <log> <pidfile> <port> <cmd...>
start_svc() {
  local name="$1" wd="$2" log="$3" pf="$4" port="$5"; shift 5
  if listening "$port"; then ok "$name already up on :$port (pid $(port_pid "$port"))"; return 0; fi
  ( cd "$wd" && exec nohup "$@" >"$log" 2>&1 </dev/null ) &
  echo $! > "$pf"
  if wait_listen "$port"; then ok "$name up on :$port (pid $(port_pid "$port"))"
  else err "$name failed to listen on :$port — tail $log"; return 1; fi
}

# stop_svc <name> <port> <pidfile>
stop_svc() {
  local name="$1" port="$2" pf="$3" pid=""
  [ -f "$pf" ] && pid=$(cat "$pf" 2>/dev/null)
  { [ -z "$pid" ] || ! kill -0 "$pid" 2>/dev/null; } && pid=$(port_pid "$port")
  if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
    kill "$pid" 2>/dev/null || true
    local i=0
    while [ "$i" -lt 8 ] && kill -0 "$pid" 2>/dev/null; do
      i=$((i + 1)); curl -s -o /dev/null --max-time 0.3 "http://127.0.0.1:$port" 2>/dev/null || true
    done
    kill -0 "$pid" 2>/dev/null && kill -9 "$pid" 2>/dev/null || true
    ok "$name stopped (pid $pid)"
  else
    warn "$name not running"
  fi
  rm -f "$pf"
}

svc_status() {
  local name="$1" port="$2" pid; pid=$(port_pid "$port")
  if [ -n "$pid" ]; then ok "$name  :$port  (pid $pid)"; else err "$name  :$port  down"; fi
}

cmd_up() {
  mkdir -p "$RUN_DIR" "$LOG_DIR"
  load_key
  if [ -n "${ANTHROPIC_API_KEY:-}" ]; then ok "ANTHROPIC_API_KEY loaded (${ANTHROPIC_API_KEY:0:7}…)"
  else warn "ANTHROPIC_API_KEY not set — hoosh can't reach Anthropic. Export it, or use a local model (AGNOSAI_MODEL)."; fi

  local hb="$HOOSH_DIR/build/hoosh"
  if [ ! -x "$hb" ]; then err "missing binary: $hb — build hoosh first"; return 1; fi

  start_svc hoosh "$HOOSH_DIR" "$LOG_DIR/hoosh.log" "$RUN_DIR/hoosh.pid" "$HOOSH_PORT" "$hb" serve "$HOOSH_PORT"

  # TODO(M4 tools / M6 server): daimon (:8090) for MCP tool dispatch and bote
  # (:9000) for the fs tools join here, and agnosai's own `serve` once
  # src/server lands. See thoth/scripts/stack.sh for the registration dance.
  say ""
  cmd_status
}

cmd_down() {
  stop_svc hoosh "$HOOSH_PORT" "$RUN_DIR/hoosh.pid"
}

cmd_status() {
  say "agnosai stack — home: ${c_d}$STACK_HOME${c_0}"
  svc_status hoosh "$HOOSH_PORT"
  say ""
  if listening "$HOOSH_PORT"; then
    say "verify the seam:  ${c_g}$0 check${c_0}"
    say "gateway:          ${c_d}http://127.0.0.1:$HOOSH_PORT/v1/chat/completions${c_0}"
  else
    say "bring it up:  ${c_g}$0 up${c_0}"
  fi
}

# The M3 exit check: a live chat-completion round trip against the gateway,
# driven through **agnosai's own seam client** rather than curl.
#
# That distinction is the whole point: a curl POST proves the gateway is up, but
# only tests/smcyr/llm_live.smcyr exercises `agnosai_hoosh_chat` — the one I/O
# call in src/llm/hoosh.cyr. Everything else about the seam is unit-tested
# offline in tests/llm_hoosh.tcyr.
cmd_check() {
  if ! listening "$HOOSH_PORT"; then err "hoosh is not up on :$HOOSH_PORT — run '$0 up'"; return 1; fi

  local harness="$AGNOSAI_DIR/tests/smcyr/llm_live.smcyr"
  local bin="$AGNOSAI_DIR/build/llm_live"
  [ -f "$harness" ] || { err "missing harness: $harness"; return 1; }

  say "building the live harness…"
  if ! ( cd "$AGNOSAI_DIR" && cyrius build "$harness" "$bin" >/dev/null 2>&1 ); then
    err "harness failed to build — run: cyrius build $harness $bin"
    return 1
  fi

  say "${c_d}driving agnosai_hoosh_chat against :$HOOSH_PORT${c_0}"
  say ""
  "$bin" "$MODEL"
  local rc=$?
  say ""
  if [ "$rc" -eq 0 ]; then
    say "${c_g}M3 exit criterion met:${c_0} a live chat-completion round trip through agnosai's client."
  else
    err "the live round trip failed (exit $rc) — see the output above"
  fi
  return "$rc"
}

cmd_logs() {
  for s in hoosh; do
    say "${c_d}== $s ==${c_0}"
    tail -n 20 "$LOG_DIR/$s.log" 2>/dev/null || say "  (no log yet)"
  done
}

usage() {
  sed -n '2,25p' "$0" | sed 's/^# \{0,1\}//'
}

case "${1:-}" in
  up)     cmd_up ;;
  down)   cmd_down ;;
  check)  cmd_check ;;
  status) cmd_status ;;
  logs)   cmd_logs ;;
  ""|-h|--help|help) usage ;;
  *) err "unknown command: $1"; usage; exit 2 ;;
esac
