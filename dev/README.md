# Native Dev Stack Harness

One-command local development stack with TLS, reverse proxy, and per-worktree slot isolation.

## Prerequisites

- `nix develop` shell (provides caddy, cargo-watch, cargo, node, pnpm, openssl, flock, curl)
- `/etc/hosts` entries for `.crlocal` hostnames
- CA certificate trusted in your browser (only once)

## First Run

1. **Enter the nix shell** (if not already inside):
   ```bash
   nix develop
   ```

2. **Add /etc/hosts entries** (the start script checks first, before doing anything else, that each of the five names resolves to an address local to this machine, and aborts naming the offenders if not):
   ```bash
   sudo sh -c 'echo "127.0.0.1 website1.crlocal website2.crlocal app.crlocal creator.crlocal api.crlocal" >> /etc/hosts'
   ```

3. **Generate certificates** (done automatically by `start.sh`, but you can run it manually):
   ```bash
   cd dev
   ./gen-cert.sh
   ```
   (`just dev::start` runs this step when the certificates are missing.)
   Follow the printed instructions to import `dev/cert/ca.crt` into your browser.

   The CA is **shared machine-wide**, in `~/.local/share/creators-reward/cert`
   (override with `CR_CERT_DIR`); `dev/cert` in each checkout is a symlink to it.
   Every worktree and clone therefore presents the same certificate, so you trust
   it once rather than once per worktree. The leaf is a wildcard for `*.crlocal`
   and ports are not part of a certificate, so one pair covers every slot.

4. **Install frontend dependencies** (one-time per checkout):
   ```bash
   cd frontend/viewer && just install
   cd frontend/creator && just install
   ```

## Commands

Every command below is also a `just` recipe in this directory; run `just --list`
here (or `just dev::<recipe>` from the root) to invoke it.

### Start the stack

Foreground (Ctrl-C stops everything):
```bash
just start
```

Background (detached, block until ready):
```bash
just start-detach
```

Force a specific slot:
```bash
just start-slot 2
```

### Stop the stack

```bash
just stop        # auto-detect running slot
just stop-slot 2 # stop slot 2
```

### Check status

```bash
just status      # all slots
just status-slot 2 # slot 2
```

## Port / Slot Table

| Slot | Public Port | Internal Base | backend | viewer | creator | site1 | site2 | admin | creator-api |
|------|-------------|---------------|---------|--------|---------|-------|-------|-------|-------------|
| 0    | 18090       | 19000         | 19000   | 19001  | 19002   | 19003 | 19004 | 19005 | 19006       |
| 1    | 18091       | 19010         | 19010   | 19011  | 19012   | 19013 | 19014 | 19015 | 19016       |
| 2    | 18092       | 19020         | 19020   | 19021  | 19022   | 19023 | 19024 | 19025 | 19026       |
| ...  | ...         | ...           | ...     | ...    | ...     | ...   | ...   | ...   | ...         |
| 9    | 18099       | 19090         | 19090   | 19091  | 19092   | 19093 | 19094 | 19095 | 19096       |

Formula:
- `PUBLIC_PORT  = 18090 + SLOT`
- `INTERNAL_BASE = 19000 + SLOT * 10`
- `backend = INTERNAL_BASE + 0`
- `viewer  = INTERNAL_BASE + 1`
- `creator = INTERNAL_BASE + 2`
- `site1   = INTERNAL_BASE + 3`
- `site2   = INTERNAL_BASE + 4`
- `admin   = INTERNAL_BASE + 5` (backend admin gateway, loopback, not behind Caddy)
- `creator-api = INTERNAL_BASE + 6` (creator service; Caddy sends `creator.crlocal/api/*` here)

## Public URLs

After start, `dev/.state/<slot>/urls.json` contains:

```json
{
  "website1": "https://website1.crlocal:<PORT>",
  "website2": "https://website2.crlocal:<PORT>",
  "app":      "https://app.crlocal:<PORT>",
  "creator":  "https://creator.crlocal:<PORT>",
  "api":      "https://api.crlocal:<PORT>"
}
```

Internal services speak plain HTTP; Caddy terminates TLS and routes by `Host` header.

## Architecture

- **Caddy** listens on the single public HTTPS port with a local wildcard certificate, shared by every worktree from one machine-wide store.
- **Slot allocation** uses `flock` lockfiles so multiple agents can start stacks concurrently without colliding.
- **State** lives under `dev/.state/<slot>/` (pids, logs, env, urls.json, Caddyfile, lock).
- **Logs** are one file per service; foreground mode multiplexes them to stdout with `[service]` prefixes.
- **Process groups**: every service and the lock holder runs in its own session (`setsid`), so its pid file holds a process-group id. `stop.sh` and the start-failure teardown send TERM to the whole group (pnpm → vite, cargo watch → backend), wait up to 6 s, then KILL.
- **Backend build** runs in the foreground (`cargo build` in `backend/`, output on the terminal) before any service is launched; a compile error stops the start at once.
- **Readiness** is polled through the public URLs (not internal ports) with `curl`, every 0.5 s. Budgets, waited in turn: fake sites 10 s each, viewer and creator 20 s each, backend 30 s (constants at the top of `start.sh`). The first failure stops the start: a service that exited, a backend whose `cargo watch` reports a finished run, or a timeout. The service's last 20 log lines and the probed URL are printed, and the slot is torn down.

## Environment Variables Exported to Children

These names are fixed — other tooling depends on them:

- `CR_PORT` — public HTTPS port
- `CR_BIND_ADDR` — bind address (127.0.0.1)
- `CR_BACKEND_PORT` — internal backend port
- `CR_VIEWER_PORT` — internal viewer Vite port
- `CR_CREATOR_PORT` — internal creator Vite port
- `CR_SITE1_PORT` — internal fake news site 1 port
- `CR_SITE2_PORT` — internal fake news site 2 port
- `CR_VIEWER_HOST` — public viewer hostname (app.crlocal)
- `CR_CREATOR_HOST` — public creator hostname (creator.crlocal)
- `CR_ADMIN_PORT` — backend admin gateway (`internal base + 5`), loopback only, not behind Caddy
- `CR_CREATOR_API_PORT` — creator service (`internal base + 6`), behind Caddy as `creator.crlocal/api/*`
- `CR_BILLING_PERIOD` — billing period length passed to the backend, `month` (default) or e.g. `60s`; recorded in the slot's `env` so tools and e2e tests know it

## Billing

Nothing is billed until a period has a key. The key holders' tool keeps the
key-handling steps offline and talks to the admin gateway only to move public
keys, batches and totals. Use a short period to watch one close and settle:

```bash
CR_BILLING_PERIOD=60s just dev::start-detach
source dev/.state/$(cat dev/.state/current-slot)/env
cd backend
export CR_ADMIN_TOKEN=dev-admin-token            # hardcoded, dev only
C="cargo run -q -p billing_ceremony --"
# offline: keys for the next 30 minutes
$C keygen --keys-out /tmp/keys --shares-out /tmp/shares --period-length 60s --period current --count 30
# online: hand the public keys to the server
$C upload-keys --api http://127.0.0.1:$CR_ADMIN_PORT $(printf -- '--key %s ' /tmp/keys/*.pub)
# read on a site with donations enabled; once the period has ended:
$C fetch-batch --api http://127.0.0.1:$CR_ADMIN_PORT --period <id> --out /tmp/batch.json
$C tally --batch /tmp/batch.json --out /tmp/totals.json \
    --share /tmp/shares/<id>-holder-1.json --share /tmp/shares/<id>-holder-2.json   # offline
$C upload-totals --api http://127.0.0.1:$CR_ADMIN_PORT --totals /tmp/totals.json
curl -s -H "Authorization: Bearer $CR_ADMIN_TOKEN" http://127.0.0.1:$CR_ADMIN_PORT/admin/billing/log
```

The backend then logs the stub charges and payouts. Real key holders keep
their shares offline and sealed to themselves.

## Troubleshooting

### "dev hostnames must resolve to a local address"
A name resolves to an address that is not local to this machine (or does not resolve), so every probe would hang. Check with `getent ahostsv4 <name>` and `ip route get <addr>` (must print `local …`). The first matching `/etc/hosts` line wins, so an earlier line for the same name overrides the one you added; fix or remove it. Use `127.0.0.1 website1.crlocal website2.crlocal app.crlocal creator.crlocal api.crlocal`.

### "No free slot found"
All 10 slots (0-9) are in use. Run `just dev::status` to see which slots are running, then stop one with `just dev::stop-slot <slot>`.

### Certificate errors in browser
Re-run `dev/gen-cert.sh` and import the new `dev/cert/ca.crt` into your browser trust store. One import covers every worktree, since they all resolve to the same store.

### api.crlocal returns 502
Caddy reached no backend on `CR_BACKEND_PORT`. Check `dev/.state/<slot>/logs/backend.log` — usually a hot rebuild by `cargo watch` is still running or failed to compile. The backend serves plain HTTP whenever `CR_BACKEND_PORT` is set; it only falls back to its own TLS on `127.0.0.1:3000` when started with no environment at all.

### Port already in use but slot appears free
The flock lock may have been left behind by a crashed run. Check `dev/status.sh`; if no processes are actually running, remove the stale lockfile manually:
```bash
rm dev/.state/<slot>/lock
```

### Cargo build is slow on first start
`start.sh` runs `cargo build` in the foreground before launching anything and has no time limit on it, so a cold build just takes as long as it takes. The 30 s backend readiness budget only starts afterwards, when `cargo watch` runs the already built binary.
