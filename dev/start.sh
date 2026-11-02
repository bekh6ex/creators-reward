#!/usr/bin/env bash
#
# dev/start.sh — native local dev stack harness
#
# Boots five services behind a single Caddy TLS reverse proxy on one public
# HTTPS port, with per-git-worktree slot isolation.
#
# Usage:
#   ./start.sh                          # foreground, Ctrl-C stops everything
#   ./start.sh --detach --wait-ready    # background, blocks until ready
#   ./start.sh --slot 3                 # force slot 3
#

set -euo pipefail

# --- Constants (formula defined in ONE place) ---
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
readonly SCRIPT_DIR
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
readonly PROJECT_ROOT
readonly STATE_ROOT="$SCRIPT_DIR/.state"
readonly CERT_DIR="$SCRIPT_DIR/cert"
readonly PUBLIC_BASE=18090
readonly INTERNAL_BASE=19000
readonly MAX_SLOT=9
# The five public names, each of which must resolve to an address local to this machine.
readonly DEV_HOSTS=(website1.crlocal website2.crlocal app.crlocal creator.crlocal api.crlocal)
# Readiness budgets in seconds per service, waited in turn (probes go through Caddy, so it is covered too).
readonly SITE_READY_TIMEOUT=10     # fake news sites: plain node servers
readonly VITE_READY_TIMEOUT=20     # viewer and creator: vite dev servers
readonly BACKEND_READY_TIMEOUT=30  # backend: cargo watch starting an already built binary
readonly POLL_INTERVAL=0.5         # seconds between readiness probes
readonly PROBE_MAX_TIME=2          # curl --max-time per probe, seconds
readonly PROGRESS_INTERVAL=5       # seconds between "not ready yet" lines
readonly LOG_TAIL_LINES=20         # log lines shown when a service fails
readonly KILL_GRACE=6              # seconds a stopping service gets after TERM before KILL

# --- Parse arguments ---
DETACH=false
WAIT_READY=false
FORCED_SLOT=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --detach)
            DETACH=true
            shift
            ;;
        --wait-ready)
            WAIT_READY=true
            shift
            ;;
        --slot)
            if [[ -z "${2:-}" ]]; then
                echo "ERROR: --slot requires an argument" >&2
                exit 1
            fi
            FORCED_SLOT="$2"
            shift 2
            ;;
        --help|-h)
            cat <<'EOF'
Usage: start.sh [OPTIONS]

Options:
  --detach        Run services in the background.
  --wait-ready    With --detach, block until all services answer 200.
  --slot N        Force slot N (0-9). Default: lowest free slot.
  --help, -h      Show this help.
EOF
            exit 0
            ;;
        *)
            echo "ERROR: Unknown argument: $1" >&2
            exit 1
            ;;
    esac
done

if [[ "$DETACH" == false && "$WAIT_READY" == true ]]; then
    echo "WARNING: --wait-ready without --detach has no effect" >&2
fi

# --- Helpers ---

port_free() {
    local port="$1"
    ! bash -c "exec 3<>/dev/tcp/127.0.0.1/$port" 2>/dev/null
}

port_open() {
    local port="$1"
    bash -c "exec 3<>/dev/tcp/127.0.0.1/$port" 2>/dev/null
}

# Resolve each host the way curl will (first IPv4 address) and require it to be
# local to this machine. A name pointing at an unreachable address would make
# every readiness probe hang until its timeout.
check_hosts() {
    local -a hosts=("$@")
    local -a bad=()
    local host addr route
    for host in "${hosts[@]}"; do
        addr=""
        read -r addr _ < <(getent ahostsv4 "$host" 2>/dev/null) || true
        if [[ -z "$addr" ]]; then
            bad+=("$host -> does not resolve")
            continue
        fi
        if command -v ip >/dev/null 2>&1; then
            route=""
            read -r route _ < <(ip route get "$addr" 2>/dev/null) || true
            [[ "$route" == "local" ]] && continue
        else
            # No iproute2: accept loopback only.
            [[ "$addr" == 127.* ]] && continue
        fi
        bad+=("$host -> $addr (not local to this machine)")
    done

    if [[ ${#bad[@]} -eq 0 ]]; then
        return 0
    fi

    echo "ERROR: dev hostnames must resolve to a local address:" >&2
    printf '  %s\n' "${bad[@]}" >&2
    echo "Fix /etc/hosts (the first matching line wins), e.g.:" >&2
    echo "  127.0.0.1 ${DEV_HOSTS[*]}" >&2
    return 1
}

# Last lines of a service log, indented, on stderr.
print_log_tail() {
    local log="$1"
    echo "  --- last $LOG_TAIL_LINES lines of $log ---" >&2
    tail -n "$LOG_TAIL_LINES" "$log" 2>/dev/null | sed 's/^/  | /' >&2 || true
}

# True when the last thing cargo-watch logged is a finished (failed or exited) run,
# e.g. "[Finished running. Exit status: 101]" after "error: could not compile".
cargo_watch_run_finished() {
    local last
    last=$(grep -E "^\[(Running '|Finished running)" "$1" 2>/dev/null | tail -n 1 || true)
    [[ "$last" == "[Finished running"* ]]
}

claim_slot() {
    local slot="$1"
    local state_dir="$STATE_ROOT/$slot"
    mkdir -p "$state_dir"
    local lockfile="$state_dir/lock"

    exec 200>"$lockfile"
    if ! flock -n 200; then
        exec 200>&-
        return 1
    fi

    local public_port=$((PUBLIC_BASE + slot))
    local internal_base=$((INTERNAL_BASE + slot * 10))
    local ports=("$public_port")
    local i
    for i in 0 1 2 3 4 5 6; do
        ports+=("$((internal_base + i))")
    done

    for p in "${ports[@]}"; do
        if ! port_free "$p"; then
            flock -u 200
            exec 200>&-
            return 1
        fi
    done

    echo "$slot"
    return 0
}

find_slot() {
    local slot
    for slot in $(seq 0 "$MAX_SLOT"); do
        local result
        if result=$(claim_slot "$slot"); then
            echo "$result"
            return 0
        fi
    done
    echo "ERROR: No free slot found (0-$MAX_SLOT)" >&2
    return 1
}

generate_caddyfile() {
    cat > "$STATE_DIR/Caddyfile" <<EOF
{
    admin off
    auto_https off
}

:${CR_PORT} {
    bind ${CR_BIND_ADDR}
    tls ${CERT_DIR}/cert.pem ${CERT_DIR}/key.pem

    @website1 host website1.crlocal
    reverse_proxy @website1 ${CR_BIND_ADDR}:${CR_SITE1_PORT}

    @website2 host website2.crlocal
    reverse_proxy @website2 ${CR_BIND_ADDR}:${CR_SITE2_PORT}

    @app host app.crlocal
    route @app {
        handle /api/* {
            reverse_proxy ${CR_BIND_ADDR}:${CR_BACKEND_PORT}
        }
        reverse_proxy ${CR_BIND_ADDR}:${CR_VIEWER_PORT}
    }

    @creator host creator.crlocal
    route @creator {
        handle /api/* {
            reverse_proxy ${CR_BIND_ADDR}:${CR_CREATOR_API_PORT}
        }
        reverse_proxy ${CR_BIND_ADDR}:${CR_CREATOR_PORT}
    }

    @api host api.crlocal
    reverse_proxy @api ${CR_BIND_ADDR}:${CR_BACKEND_PORT}
}
EOF
}

# wait_for_service NAME URL TIMEOUT_SECONDS PID LOGFILE [cargo-watch]
# Fails at once when the service (or Caddy) has exited, or, with cargo-watch,
# when its last run finished; otherwise polls URL until TIMEOUT_SECONDS.
wait_for_service() {
    local name="$1"
    local url="$2"
    local timeout="$3"
    local pid="$4"
    local log="$5"
    local watcher="${6:-}"
    local start=$SECONDS
    local next_progress=$((SECONDS + PROGRESS_INTERVAL))

    echo "Waiting for $name ..."
    while true; do
        if ! kill -0 "$pid" 2>/dev/null; then
            echo "  $name exited" >&2
            print_log_tail "$log"
            return 1
        fi
        if ! kill -0 "$CADDY_PID" 2>/dev/null; then
            echo "  caddy exited" >&2
            print_log_tail "$STATE_DIR/logs/caddy.log"
            return 1
        fi
        if [[ "$watcher" == "cargo-watch" ]] && cargo_watch_run_finished "$log"; then
            echo "  $name run failed (cargo-watch reported a finished run)" >&2
            print_log_tail "$log"
            return 1
        fi
        if curl --silent --fail --cacert "$CERT_DIR/ca.crt" --max-time "$PROBE_MAX_TIME" "$url" >/dev/null 2>&1; then
            echo "  $name is ready"
            return 0
        fi
        if [[ $((SECONDS - start)) -ge $timeout ]]; then
            echo "  $name not answering within ${timeout}s (probed $url)" >&2
            print_log_tail "$log"
            return 1
        fi
        if [[ $SECONDS -ge $next_progress ]]; then
            echo "  $name not ready yet ($((SECONDS - start))s/${timeout}s)"
            next_progress=$((SECONDS + PROGRESS_INTERVAL))
        fi
        sleep "$POLL_INTERVAL"
    done
}

# Every service runs in its own session (setsid), so its pid is also its process
# group id and one signal reaches the whole tree (pnpm -> vite, cargo watch -> backend).
# stop_groups PID...: TERM each group, wait up to KILL_GRACE seconds, then KILL what is left.
stop_groups() {
    local pid
    local deadline=$((SECONDS + KILL_GRACE))
    for pid in "$@"; do
        kill -TERM -- "-$pid" 2>/dev/null || kill -TERM "$pid" 2>/dev/null || true
    done
    while [[ $SECONDS -lt $deadline ]]; do
        local alive=false
        for pid in "$@"; do
            if kill -0 -- "-$pid" 2>/dev/null || kill -0 "$pid" 2>/dev/null; then
                alive=true
            fi
        done
        [[ "$alive" == true ]] || return 0
        sleep 0.2
    done
    for pid in "$@"; do
        kill -KILL -- "-$pid" 2>/dev/null || kill -KILL "$pid" 2>/dev/null || true
    done
}

CLEANED_UP=false
declare -a TAIL_PIDS=()

cleanup() {
    # shellcheck disable=SC2320 # first statement of a trap handler: $? is the exit status being handled
    exit_code=$?
    if [[ "$CLEANED_UP" == true ]]; then
        exit "$exit_code"
    fi
    CLEANED_UP=true
    echo ""
    echo "Shutting down slot $SLOT ..."

    # Kill log tailers
    if [[ ${#TAIL_PIDS[@]} -gt 0 ]]; then
        local pid
        for pid in "${TAIL_PIDS[@]}"; do
            kill "$pid" 2>/dev/null || true
        done
    fi

    # Kill the holder and every service, each as a whole process group
    local -a groups=()
    local pidfile
    for pidfile in "$STATE_DIR/holder.pid" "$STATE_DIR/pids"/*.pid; do
        if [[ -f "$pidfile" ]]; then
            groups+=("$(cat "$pidfile")")
            rm -f "$pidfile"
        fi
    done
    if [[ ${#groups[@]} -gt 0 ]]; then
        stop_groups "${groups[@]}"
    fi

    if [[ -f "$STATE_ROOT/current-slot" && "$(cat "$STATE_ROOT/current-slot")" == "$SLOT" ]]; then
        rm -f "$STATE_ROOT/current-slot"
    fi

    # Release lock
    exec 200>&- 2>/dev/null || true

    exit "$exit_code"
}

# --- 1. Check that the dev hostnames resolve to a local address ---
check_hosts "${DEV_HOSTS[@]}" || exit 1

# --- 2. Check / generate certificates ---
if [[ ! -f "$CERT_DIR/ca.crt" || ! -f "$CERT_DIR/cert.pem" ]]; then
    echo "Certificates missing, generating..."
    "$SCRIPT_DIR/gen-cert.sh"
elif ! openssl x509 -in "$CERT_DIR/ca.crt" -noout -checkend 604800 >/dev/null 2>&1; then
    # The CA, not the leaf: the CA lives 30 days and the leaf 825, so testing
    # the leaf would let the CA lapse and break the chain without regenerating.
    echo "CA expiring within 7 days, regenerating..."
    "$SCRIPT_DIR/gen-cert.sh"
fi

# --- 2b. Build the backend ---
# cargo-watch keeps running after a failed compile, so a build error would only
# surface as a readiness timeout. Build in the foreground first.
echo "=== Building backend ==="
if ! (cd "$PROJECT_ROOT/backend" && cargo build); then
    echo "ERROR: backend build failed" >&2
    exit 1
fi

# --- 3. Allocate slot ---
if [[ -n "$FORCED_SLOT" ]]; then
    if ! [[ "$FORCED_SLOT" =~ ^[0-9]+$ ]] || [[ "$FORCED_SLOT" -lt 0 ]] || [[ "$FORCED_SLOT" -gt "$MAX_SLOT" ]]; then
        echo "ERROR: Invalid slot: $FORCED_SLOT (must be 0-$MAX_SLOT)" >&2
        exit 1
    fi
    SLOT=$(claim_slot "$FORCED_SLOT") || { echo "ERROR: Slot $FORCED_SLOT is not available" >&2; exit 1; }
else
    SLOT=$(find_slot) || exit 1
fi

STATE_DIR="$STATE_ROOT/$SLOT"
mkdir -p "$STATE_DIR/pids" "$STATE_DIR/logs"
trap cleanup EXIT INT TERM
echo "$SLOT" > "$STATE_ROOT/current-slot"

echo "=== Allocated slot $SLOT ==="

# --- 4. Compute environment ---
CR_PORT=$((PUBLIC_BASE + SLOT))
CR_INTERNAL_BASE=$((INTERNAL_BASE + SLOT * 10))
CR_BACKEND_PORT=$((CR_INTERNAL_BASE + 0))
CR_VIEWER_PORT=$((CR_INTERNAL_BASE + 1))
CR_CREATOR_PORT=$((CR_INTERNAL_BASE + 2))
CR_SITE1_PORT=$((CR_INTERNAL_BASE + 3))
CR_SITE2_PORT=$((CR_INTERNAL_BASE + 4))
# Backend admin gateway: loopback only, deliberately not routed through Caddy.
CR_ADMIN_PORT=$((CR_INTERNAL_BASE + 5))
# Creator service (same backend binary, own listener): creator.crlocal/api/*.
CR_CREATOR_API_PORT=$((CR_INTERNAL_BASE + 6))
CR_BIND_ADDR="127.0.0.1"
CR_VIEWER_HOST="app.crlocal"
CR_CREATOR_HOST="creator.crlocal"
# Billing period length, passed through to the backend (`month` or e.g. `60s`).
CR_BILLING_PERIOD="${CR_BILLING_PERIOD:-month}"

export CR_PORT CR_BIND_ADDR CR_BACKEND_PORT CR_VIEWER_PORT CR_CREATOR_PORT
export CR_SITE1_PORT CR_SITE2_PORT CR_VIEWER_HOST CR_CREATOR_HOST
export CR_ADMIN_PORT CR_CREATOR_API_PORT CR_BILLING_PERIOD

# --- 5. Write state files ---
cat > "$STATE_DIR/env" <<EOF
CR_PORT=$CR_PORT
CR_BIND_ADDR=$CR_BIND_ADDR
CR_BACKEND_PORT=$CR_BACKEND_PORT
CR_VIEWER_PORT=$CR_VIEWER_PORT
CR_CREATOR_PORT=$CR_CREATOR_PORT
CR_SITE1_PORT=$CR_SITE1_PORT
CR_SITE2_PORT=$CR_SITE2_PORT
CR_VIEWER_HOST=$CR_VIEWER_HOST
CR_CREATOR_HOST=$CR_CREATOR_HOST
CR_ADMIN_PORT=$CR_ADMIN_PORT
CR_CREATOR_API_PORT=$CR_CREATOR_API_PORT
CR_BILLING_PERIOD=$CR_BILLING_PERIOD
EOF

cat > "$STATE_DIR/urls.json" <<EOF
{
  "website1": "https://website1.crlocal:$CR_PORT",
  "website2": "https://website2.crlocal:$CR_PORT",
  "app": "https://app.crlocal:$CR_PORT",
  "creator": "https://creator.crlocal:$CR_PORT",
  "api": "https://api.crlocal:$CR_PORT"
}
EOF

# --- 6. Start lock holder if detach mode ---
if [[ "$DETACH" == true ]]; then
    (
        exec setsid bash -c 'while true; do sleep 3600; done'
    ) >/dev/null 2>&1 &
    HOLDER_PID=$!
    disown "$HOLDER_PID" 2>/dev/null || true
    echo "$HOLDER_PID" > "$STATE_DIR/holder.pid"
fi

# --- 7. Launch services ---
echo ""
echo "=== Launching services ==="
echo "Public:  https://<host>.crlocal:$CR_PORT"
echo "Internal: backend=$CR_BACKEND_PORT viewer=$CR_VIEWER_PORT creator=$CR_CREATOR_PORT site1=$CR_SITE1_PORT site2=$CR_SITE2_PORT admin=$CR_ADMIN_PORT creator-api=$CR_CREATOR_API_PORT"
echo ""

# Backend
: > "$STATE_DIR/logs/backend.log"
(
    cd "$PROJECT_ROOT/backend"
    exec setsid cargo watch -w src -w services -w common -w templates -x run
) >>"$STATE_DIR/logs/backend.log" 2>&1 &
BACKEND_PID=$!
disown "$BACKEND_PID" 2>/dev/null || true
echo "$BACKEND_PID" > "$STATE_DIR/pids/backend.pid"
echo "Backend    pid=$BACKEND_PID"

# Site 1
: > "$STATE_DIR/logs/site1.log"
(
    cd "$PROJECT_ROOT/fake-sites/news"
    PORT="$CR_SITE1_PORT" CR_BIND_ADDR="$CR_BIND_ADDR" \
        SITE_ID=1 SITE_NAME="The Daily Ledger" \
        SITE_TAGLINE="News you can trust" SITE_ACCENT="#1d7d2f" \
        SITE_SEED="ledger-seed" exec setsid node server.js
) >>"$STATE_DIR/logs/site1.log" 2>&1 &
SITE1_PID=$!
disown "$SITE1_PID" 2>/dev/null || true
echo "$SITE1_PID" > "$STATE_DIR/pids/site1.pid"
echo "Site1      pid=$SITE1_PID"

# Site 2
: > "$STATE_DIR/logs/site2.log"
(
    cd "$PROJECT_ROOT/fake-sites/news"
    PORT="$CR_SITE2_PORT" CR_BIND_ADDR="$CR_BIND_ADDR" \
        SITE_ID=2 SITE_NAME="Kompas Weekly" \
        SITE_TAGLINE="Your weekly compass" SITE_ACCENT="#b00020" \
        SITE_SEED="kompas-seed" exec setsid node server.js
) >>"$STATE_DIR/logs/site2.log" 2>&1 &
SITE2_PID=$!
disown "$SITE2_PID" 2>/dev/null || true
echo "$SITE2_PID" > "$STATE_DIR/pids/site2.pid"
echo "Site2      pid=$SITE2_PID"

# Viewer SPA
: > "$STATE_DIR/logs/viewer.log"
(
    cd "$PROJECT_ROOT/frontend/viewer"
    CR_BIND_ADDR="$CR_BIND_ADDR" CR_VIEWER_PORT="$CR_VIEWER_PORT" \
        CR_PORT="$CR_PORT" CR_VIEWER_HOST="$CR_VIEWER_HOST" \
        exec setsid pnpm run dev
) >>"$STATE_DIR/logs/viewer.log" 2>&1 &
VIEWER_PID=$!
disown "$VIEWER_PID" 2>/dev/null || true
echo "$VIEWER_PID" > "$STATE_DIR/pids/viewer.pid"
echo "Viewer     pid=$VIEWER_PID"

# Creator portal
: > "$STATE_DIR/logs/creator.log"
(
    cd "$PROJECT_ROOT/frontend/creator"
    CR_BIND_ADDR="$CR_BIND_ADDR" CR_CREATOR_PORT="$CR_CREATOR_PORT" \
        CR_PORT="$CR_PORT" CR_CREATOR_HOST="$CR_CREATOR_HOST" \
        exec setsid pnpm run dev
) >>"$STATE_DIR/logs/creator.log" 2>&1 &
CREATOR_PID=$!
disown "$CREATOR_PID" 2>/dev/null || true
echo "$CREATOR_PID" > "$STATE_DIR/pids/creator.pid"
echo "Creator    pid=$CREATOR_PID"

# --- 8. Generate Caddyfile ---
# Caddy owns TLS for the whole stack; every upstream is plain HTTP.
generate_caddyfile

# --- 9. Launch Caddy ---
: > "$STATE_DIR/logs/caddy.log"
(
    export XDG_DATA_HOME="$STATE_DIR/caddy/data"
    export XDG_CONFIG_HOME="$STATE_DIR/caddy/config"
    mkdir -p "$XDG_DATA_HOME" "$XDG_CONFIG_HOME"
    exec setsid caddy run --config "$STATE_DIR/Caddyfile" --adapter caddyfile
) >>"$STATE_DIR/logs/caddy.log" 2>&1 &
CADDY_PID=$!
disown "$CADDY_PID" 2>/dev/null || true
echo "$CADDY_PID" > "$STATE_DIR/pids/caddy.pid"
echo "Caddy      pid=$CADDY_PID"

# --- 10. Wait for readiness ---
if [[ "$DETACH" == true && "$WAIT_READY" == true ]] || [[ "$DETACH" == false ]]; then
    echo ""
    echo "=== Waiting for services to be ready ==="

    # The first failure stops the start; the EXIT trap tears the slot down.
    wait_for_service "website1" "https://website1.crlocal:$CR_PORT/healthz" "$SITE_READY_TIMEOUT" "$SITE1_PID" "$STATE_DIR/logs/site1.log" || exit 1
    wait_for_service "website2" "https://website2.crlocal:$CR_PORT/healthz" "$SITE_READY_TIMEOUT" "$SITE2_PID" "$STATE_DIR/logs/site2.log" || exit 1
    wait_for_service "viewer" "https://app.crlocal:$CR_PORT/" "$VITE_READY_TIMEOUT" "$VIEWER_PID" "$STATE_DIR/logs/viewer.log" || exit 1
    wait_for_service "creator" "https://creator.crlocal:$CR_PORT/" "$VITE_READY_TIMEOUT" "$CREATOR_PID" "$STATE_DIR/logs/creator.log" || exit 1
    wait_for_service "api" "https://api.crlocal:$CR_PORT/api/embed.js" "$BACKEND_READY_TIMEOUT" "$BACKEND_PID" "$STATE_DIR/logs/backend.log" cargo-watch || exit 1

    echo ""
    echo "=== All services ready ==="
    echo "Slot: $SLOT"
    echo "URLs file: $STATE_DIR/urls.json"
    cat "$STATE_DIR/urls.json"
fi

# --- 11. Foreground or detach ---
if [[ "$DETACH" == true ]]; then
    echo ""
    echo "Stack running in background (slot $SLOT)"
    # Unset trap so our own exit doesn't trigger cleanup
    trap - EXIT INT TERM
    exit 0
fi

# Foreground mode: tail logs and wait for children
echo ""
echo "=== Logs (Ctrl-C to stop) ==="

tail -F "$STATE_DIR/logs/backend.log" > >(sed 's/^/[backend] /') &
TAIL_PIDS+=("$!")
tail -F "$STATE_DIR/logs/viewer.log" > >(sed 's/^/[viewer] /') &
TAIL_PIDS+=("$!")
tail -F "$STATE_DIR/logs/creator.log" > >(sed 's/^/[creator] /') &
TAIL_PIDS+=("$!")
tail -F "$STATE_DIR/logs/site1.log" > >(sed 's/^/[site1] /') &
TAIL_PIDS+=("$!")
tail -F "$STATE_DIR/logs/site2.log" > >(sed 's/^/[site2] /') &
TAIL_PIDS+=("$!")
tail -F "$STATE_DIR/logs/caddy.log" > >(sed 's/^/[caddy] /') &
TAIL_PIDS+=("$!")

while true; do
    for pidfile in "$STATE_DIR/pids"/*.pid; do
        if [[ -f "$pidfile" ]]; then
            pid=$(cat "$pidfile")
            if ! kill -0 "$pid" 2>/dev/null; then
                name=$(basename "$pidfile" .pid)
                echo ""
                echo "WARNING: $name (pid $pid) has exited" >&2
                cleanup
            fi
        fi
    done
    sleep 2
done
