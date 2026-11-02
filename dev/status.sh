#!/usr/bin/env bash
#
# dev/status.sh — show status of running dev stack slots
#
# Usage:
#   ./status.sh      # show all slots
#   ./status.sh 3    # show slot 3
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
readonly SCRIPT_DIR
STATE_ROOT="$SCRIPT_DIR/.state"
readonly STATE_ROOT

SLOT="${1:-}"

show_slot() {
    local s="$1"
    local state_dir="$STATE_ROOT/$s"

    echo "=== Slot $s ==="

    if [[ -f "$state_dir/holder.pid" ]]; then
        local pid
        pid=$(cat "$state_dir/holder.pid")
        if kill -0 "$pid" 2>/dev/null; then
            echo "Lock holder: running (pid $pid)"
        else
            echo "Lock holder: dead (pid $pid)"
        fi
    else
        echo "Lock holder: none"
    fi

    local pidfile
    for pidfile in "$state_dir/pids"/*.pid; do
        if [[ -f "$pidfile" ]]; then
            local name pid
            name=$(basename "$pidfile" .pid)
            pid=$(cat "$pidfile")
            if kill -0 "$pid" 2>/dev/null; then
                echo "  $name: running (pid $pid)"
            else
                echo "  $name: dead (pid $pid)"
            fi
        fi
    done

    if [[ -f "$state_dir/env" ]]; then
        echo "  env: $state_dir/env"
    fi
    if [[ -f "$state_dir/urls.json" ]]; then
        echo "  urls: $state_dir/urls.json"
    fi
}

if [[ -n "$SLOT" ]]; then
    if [[ -d "$STATE_ROOT/$SLOT" ]]; then
        show_slot "$SLOT"
    else
        echo "Slot $SLOT does not exist" >&2
        exit 1
    fi
else
    found=false
    for d in "$STATE_ROOT"/*/; do
        if [[ -d "$d" ]]; then
            show_slot "$(basename "$d")"
            echo ""
            found=true
        fi
    done
    if [[ "$found" == false ]]; then
        echo "No slots found in $STATE_ROOT"
    fi
fi
