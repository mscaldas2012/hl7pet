#!/usr/bin/env bash
# Restarts the HL7-PET playground webapp (spec 9000-playground-webapp):
# stops whatever instance is currently running (tracked PID and/or anything
# bound to the target port), then starts a fresh one in the background.
#
# Usage:
#   playground/bounce.sh [--stop-only] [--port N]
#
# Env vars:
#   PORT        port to bind (default 8080 -- NOT 5000: on macOS, port 5000
#               is also claimed by ControlCenter/AirPlay Receiver on *:5000
#               (both IPv4 and IPv6), and since Flask only binds IPv4, a
#               browser resolving "localhost" to IPv6 first lands on AirPlay
#               Receiver instead -- a real, previously-hit 403, not a typo)
#   FLASK_HOST  host to bind (default 127.0.0.1)

set -euo pipefail

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
PLAYGROUND_DIR="$REPO_ROOT/playground"
PID_FILE="$PLAYGROUND_DIR/.bounce.pid"
LOG_FILE="$PLAYGROUND_DIR/.bounce.log"

PORT="${PORT:-8080}"
HOST="${FLASK_HOST:-127.0.0.1}"
STOP_ONLY=false

while [ $# -gt 0 ]; do
    case "$1" in
        --stop-only) STOP_ONLY=true ;;
        --port) shift; PORT="$1" ;;
        --help|-h)
            echo "Usage: $0 [--stop-only] [--port N]"
            exit 0
            ;;
        *)
            echo "error: unknown argument '$1'" >&2
            exit 1
            ;;
    esac
    shift
done

log() { echo "[bounce] $*"; }

stop_existing() {
    local stopped=false

    if [ -f "$PID_FILE" ]; then
        local old_pid
        old_pid="$(cat "$PID_FILE")"
        if [ -n "$old_pid" ] && kill -0 "$old_pid" 2>/dev/null; then
            log "stopping tracked instance (pid $old_pid)"
            kill "$old_pid" 2>/dev/null || true
            for _ in $(seq 1 20); do
                kill -0 "$old_pid" 2>/dev/null || break
                sleep 0.25
            done
            kill -0 "$old_pid" 2>/dev/null && kill -9 "$old_pid" 2>/dev/null || true
            stopped=true
        fi
        rm -f "$PID_FILE"
    fi

    # Catch a stray instance bound to the port but not tracked by our PID
    # file (e.g. started outside this script). Only ever touches a process
    # whose own command line actually looks like this Flask app -- never
    # blindly kill "whatever is on the port": on macOS, port 5000 is also
    # macOS's own AirPlay Receiver (ControlCenter, "commplex-main"), and
    # killing an unrelated process is exactly the kind of surprise this
    # script must not cause.
    if command -v lsof >/dev/null 2>&1; then
        local pid cmd
        while IFS= read -r pid; do
            [ -n "$pid" ] || continue
            cmd="$(ps -p "$pid" -o command= 2>/dev/null || true)"
            case "$cmd" in
                *playground.app*|*flask*)
                    log "stopping stray instance on port $PORT (pid $pid: $cmd)"
                    kill "$pid" 2>/dev/null || true
                    sleep 0.5
                    kill -0 "$pid" 2>/dev/null && kill -9 "$pid" 2>/dev/null || true
                    stopped=true
                    ;;
                *)
                    log "warning: port $PORT is held by an unrelated process (pid $pid: ${cmd:-unknown}) -- leaving it alone. Use --port to pick a different port."
                    ;;
            esac
        done <<< "$(lsof -ti "tcp:$PORT" 2>/dev/null || true)"
    fi

    if [ "$stopped" = true ]; then
        log "stopped"
    else
        log "nothing running"
    fi
}

stop_existing

if [ "$STOP_ONLY" = true ]; then
    exit 0
fi

# Prefer the repo's own venv (per playground/README.md's install instructions)
# if it exists; otherwise fall back to whatever `python3`/`flask` is on PATH.
if [ -f "$REPO_ROOT/.venv/bin/activate" ]; then
    # shellcheck disable=SC1091
    source "$REPO_ROOT/.venv/bin/activate"
fi

if ! command -v flask >/dev/null 2>&1; then
    echo "error: 'flask' not found. Install per playground/README.md:" >&2
    echo "  python3 -m venv .venv && source .venv/bin/activate" >&2
    echo "  pip install maturin && (cd crates/python && maturin develop)" >&2
    echo "  pip install -r playground/requirements.txt" >&2
    exit 1
fi

log "starting on http://$HOST:$PORT"
(
    cd "$REPO_ROOT"
    FLASK_APP=playground.app nohup flask --app playground.app run --host "$HOST" --port "$PORT" \
        > "$LOG_FILE" 2>&1 &
    echo $! > "$PID_FILE"
)

sleep 1
new_pid="$(cat "$PID_FILE")"
if kill -0 "$new_pid" 2>/dev/null; then
    log "started (pid $new_pid), logs at $LOG_FILE"
else
    log "failed to start -- see $LOG_FILE"
    rm -f "$PID_FILE"
    exit 1
fi
