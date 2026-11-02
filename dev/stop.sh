#!/usr/bin/env bash
#
# dev/stop.sh — stop a running dev stack slot
#
# Usage:
#   ./stop.sh        # find and stop any running slot
#   ./stop.sh 3      # stop slot 3 specifically
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
readonly SCRIPT_DIR
STATE_ROOT="$SCRIPT_DIR/.state"
readonly STATE_ROOT

SLOT="${1:-}"

# Prefer the slot this worktree started; $STATE_ROOT may hold several.
if [[ -z "$SLOT" && -f "$STATE_ROOT/current-slot" ]]; then
    SLOT="$(cat "$STATE_ROOT/current-slot")"
    if [[ ! -d "$STATE_ROOT/$SLOT" ]]; then
        SLOT=""
    fi
fi

# --- Find running slot if not specified ---
if [[ -z "$SLOT" ]]; then
    for d in "$STATE_ROOT"/*/; do
        if [[ -d "$d" ]]; then
            local_slot=$(basename "$d")
            # Check holder.pid
            if [[ -f "$d/holder.pid" ]]; then
                pid=$(cat "$d/holder.pid")
                if kill -0 "$pid" 2>/dev/null; then
                    SLOT="$local_slot"
                    break
                fi
            fi
            # Check service pids
            found=false
            for pidfile in "$d/pids"/*.pid; do
                if [[ -f "$pidfile" ]]; then
                    pid=$(cat "$pidfile")
                    if kill -0 "$pid" 2>/dev/null; then
                        found=true
                        break
                    fi
                fi
            done
            if [[ "$found" == true ]]; then
                SLOT="$local_slot"
                break
            fi
        fi
    done
fi

if [[ -z "$SLOT" ]]; then
    echo "No running stack found" >&2
    exit 1
fi

echo "Stopping slot $SLOT ..."

# Seconds a stopping service gets after TERM before KILL.
readonly KILL_GRACE=6

# Every service (and the lock holder) runs in its own session, so its pid is also
# its process group id; signalling -PID reaches the whole tree (pnpm -> vite, ...).
# A pid file from before that change names a plain process: fall back to the pid.
groups=()
pids_to_remove=()

# Holder first (releases flock lock)
if [[ -f "$STATE_ROOT/$SLOT/holder.pid" ]]; then
    groups+=("$(cat "$STATE_ROOT/$SLOT/holder.pid")")
    pids_to_remove+=("$STATE_ROOT/$SLOT/holder.pid")
fi

for pidfile in "$STATE_ROOT/$SLOT/pids"/*.pid; do
    if [[ -f "$pidfile" ]]; then
        pid=$(cat "$pidfile")
        echo "  Killing $(basename "$pidfile" .pid) (pid $pid)"
        groups+=("$pid")
        pids_to_remove+=("$pidfile")
    fi
done

for pid in "${groups[@]}"; do
    kill -TERM -- "-$pid" 2>/dev/null || kill -TERM "$pid" 2>/dev/null || true
done
deadline=$((SECONDS + KILL_GRACE))
while [[ $SECONDS -lt $deadline ]]; do
    alive=false
    for pid in "${groups[@]}"; do
        if kill -0 -- "-$pid" 2>/dev/null || kill -0 "$pid" 2>/dev/null; then
            alive=true
        fi
    done
    [[ "$alive" == true ]] || break
    sleep 0.2
done
for pid in "${groups[@]}"; do
    kill -KILL -- "-$pid" 2>/dev/null || kill -KILL "$pid" 2>/dev/null || true
done
if [[ ${#pids_to_remove[@]} -gt 0 ]]; then
    rm -f "${pids_to_remove[@]}"
fi

if [[ -f "$STATE_ROOT/current-slot" && "$(cat "$STATE_ROOT/current-slot")" == "$SLOT" ]]; then
    rm -f "$STATE_ROOT/current-slot"
fi

echo "Slot $SLOT stopped"
