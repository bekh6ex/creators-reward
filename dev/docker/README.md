<!--
=============================================================================
PARTIALLY VERIFIED — built and run once for real (2026-09)

Confirmed working: images build, all services start, the stack is reachable
through Caddy, `down` completes. One bug found and fixed by that first run:
node/sh services ignored SIGTERM (see Troubleshooting).
Still unverified by execution: live reload across the bind mounts, HMR over
the TLS proxy, the SSH port-forward path, and the direct LAN/VPN variant.
=============================================================================
-->

# Docker Dev Stack

Containerised twin of the native worktree harness (`dev/start.sh`).
Runs the same five services plus Caddy TLS proxy, reachable at the same
`*.crlocal` hostnames, on a **single published host port: `18080`**, bound to
`127.0.0.1` of the Docker host by default (see "Accessing the stack from
another machine" below).

## First run (one-time per machine)

### 1. /etc/hosts

```bash
sudo sh -c 'echo "127.0.0.1 website1.crlocal website2.crlocal app.crlocal creator.crlocal api.crlocal" >> /etc/hosts'
```

### 2. TLS certificates

Generate the wildcard CA + leaf (or reuse an existing one):

```bash
cd dev
./gen-cert.sh
```

Note that `dev/cert` is a **symlink** into the machine-wide store
(`~/.local/share/creators-reward/cert`, overridable via `CR_CERT_DIR`) so all
worktrees and the compose stack share one CA. Compose bind-mounts it as
`./dev/cert:/etc/caddy/certs:ro`; the daemon resolves the symlink, which is
transparent on Linux. On Docker Desktop the *target* must be in a shared
folder — `$HOME` is shared by default, so the default location is fine, but a
`CR_CERT_DIR` pointing somewhere exotic would need adding to file sharing.

Import `dev/cert/ca.crt` into your browser trust store. Without this, HTTPS
will be rejected and cross-site iframes will fail silently.

### 3. Build and start

```bash
docker compose up --build
```

Both CLIs parse this file: the `docker compose` plugin (v2, container names
like `creators-reward-creator-1`) and the legacy Python `docker-compose`
v1.27+ (names like `creators-reward_creator_1` — this is the variant an
actual run used so far).

The first backend build is cold and may take several minutes. Subsequent
restarts are fast because `/app/target` lives in a named volume.

## URLs

| Hostname            | URL                                   |
| ------------------- | ------------------------------------- |
| website1.crlocal    | `https://website1.crlocal:18080`      |
| website2.crlocal    | `https://website2.crlocal:18080`      |
| app.crlocal         | `https://app.crlocal:18080`           |
| creator.crlocal     | `https://creator.crlocal:18080`       |
| api.crlocal         | `https://api.crlocal:18080`           |

Internal services speak plain HTTP; Caddy terminates TLS and routes by `Host`.

## Service → port → hostname

| Service  | Internal port | Hostname         | Image / Dockerfile                  |
| -------- | ------------- | ---------------- | ----------------------------------- |
| caddy    | `18080`       | (edge proxy)     | `caddy:2-alpine`                    |
| backend  | `8000`        | `api.crlocal`    | `dev/docker/backend.Dockerfile`     |
| backend  | `8002`        | `creator.crlocal/api/*` (creator service) | `dev/docker/backend.Dockerfile` |
| viewer   | `5173`        | `app.crlocal`    | `dev/docker/viewer.Dockerfile`      |
| creator  | `5174`        | `creator.crlocal`| `dev/docker/creator.Dockerfile`     |
| site1    | `8081`        | `website1.crlocal`| `dev/docker/fake-sites.Dockerfile` |
| site2    | `8082`        | `website2.crlocal`| `dev/docker/fake-sites.Dockerfile` |

Only Caddy publishes a host port. Every other service is reachable solely on
the internal Compose network.

## How live reload works

### Rust backend

`cargo watch` (installed in the backend image) monitors `src/`, `services/`,
and `common/`. When a file changes on the bind-mounted host tree, `cargo watch`
triggers an incremental `cargo run`. The `backend-target` named volume persists
`/app/target` across restarts, so rebuilds are incremental and do not fight the
host's `backend/target` directory.

### Preact frontends (viewer + creator)

Vite's dev server runs inside each frontend container. The host source tree is
bind-mounted into `/app`; `node_modules` is kept in a named volume so
platform-specific binaries survive and the host tree is not clobbered.

Vite HMR connects over WebSocket. The Vite configs (`vite.config.ts`) already
read `CR_PORT`, `CR_VIEWER_HOST`, and `CR_CREATOR_HOST` and set `hmr.protocol`
to `wss` with `clientPort: 18080`. Caddy's `reverse_proxy` directive passes
`Upgrade: websocket` automatically, so HMR frames flow through the TLS proxy
without extra configuration.

### Fake news sites

The Node server inside `fake-sites/news/server.js` is **not watched** (same as
the native harness). If you edit `server.js`, run:

```bash
docker compose restart site1 site2
```

## Stopping

```bash
docker compose down        # stop and remove containers
docker compose down -v     # also remove named volumes (clears target + node_modules)
```

## Accessing the stack from another machine (SSH port-forward)

Caddy is the only published port, bound to `127.0.0.1` on the Docker host —
invisible to the network, but a single SSH tunnel carries the whole stack
because Caddy routes all five hostnames by `Host` header:

```bash
# on the laptop
ssh -N -L 18080:127.0.0.1:18080 <user>@<dev-host>
```

Prerequisites on the laptop:

1. The `127.0.0.1 website1.crlocal … api.crlocal` line in the laptop's
   `/etc/hosts` (First run §1) — the browser connects to `127.0.0.1:18080` and
   the tunnel catches it.
2. The dev machine's CA trusted in the laptop's browser:
   `scp <user>@<dev-host>:<repo>/dev/cert/ca.crt ~/crlocal-ca.crt`, then import
   using the Chrome/Firefox steps printed by `dev/gen-cert.sh`. Re-import when
   the dev machine's CA regenerates (~monthly; the expiry date is in its CN).

Keep the laptop-side tunnel port equal to `18080`: Vite bakes the HMR websocket
port (`CR_PORT`) at container start, so any other local port serves pages fine
but live-reload goes silent.

Direct LAN/VPN access instead of a tunnel: start with
`CR_PUBLISH_ADDR=0.0.0.0 docker compose up` (env var or `.env` next to
`docker-compose.yml`), point the laptop's `/etc/hosts` at the dev machine's IP
instead of `127.0.0.1`, and use the same URLs. That publishes the stack to the
network — trusted LAN/VPN only.

## Build context

Both Dockerfiles build from the **repo root** (`context: .`), so the root
`.dockerignore` is what keeps the context small. Without it the context is
~4 GB — `backend/target` alone is 3.7 GB — and every `docker compose build`
ships all of it to the daemon. Docker reads `.dockerignore` only from the
context root, which is why it lives at the repo root and not in this directory.

## Top 5 things most likely to be wrong

1. **File-system events not reaching `cargo watch`** — Some Docker Desktop file-sharing backends (VirtioFS, gRPC FUSE) can drop inotify events. If Rust edits do not trigger rebuilds, try adding `--poll` to the `cargo watch` command or switch Docker Desktop file-sharing settings.

2. **Vite HMR blocked by the proxy** — Caddy's `reverse_proxy` handles WebSocket upgrades by default, but if the Caddy version or adapter behaviour differs from expectation, the `wss://` HMR connection may 502. Check browser dev-tools Network → WS.

3. **Certificate not trusted** — Browsers (and headless Chromium inside iframes) reject an untrusted CA with no click-through. If the iframe is blank or the API returns `ERR_CERT_AUTHORITY_INVALID`, re-import `dev/cert/ca.crt`.

4. **Port `18080` already in use** — The native worktree harness can run concurrently, but it must use a different public port. Make sure no other process (including a native stack) is bound to `18080`.

5. **`pnpm install` fails inside the container** — If the host's lockfile was generated by a different pnpm version or the bind-mounted `package.json` is out of sync with the lockfile, `pnpm install --frozen-lockfile` will error. Run `pnpm install` on the host first to align them.

## Troubleshooting

### `docker compose down` warns "SIGTERM failed to stop container … resorting to SIGKILL"

Seen on the first real run for `viewer`, `creator`, `site1`, `site2`. Their
`command:` is `["sh", "-c", "pnpm install … && <server>"]`, making `sh` PID 1 of
the container — and the kernel only delivers a signal to PID 1 if the process
handles it explicitly, which neither `sh` nor node do. SIGTERM therefore did
nothing and every stop waited the full 10 s grace, then SIGKILLed.

Fixed with `init: true` on every service: Docker injects `tini` as PID 1,
which forwards signals to the command and reaps orphaned children. `backend`
(cargo-watch) and `caddy` already stopped cleanly on their own, but they get
`init: true` too — one rule for every service, so a future `command:` edit
cannot reintroduce this.

Apply it by recreating the containers (compose detects the config change):

```bash
docker compose up -d        # or: docker compose down && docker compose up
```

Adding `init: true` to a *new* service is the rule; if you ever see this
warning again, that is the line somebody removed.

## Assumptions made while authoring

- The host Docker engine supports the modern Compose Specification (no top-level
  `version:` key) and BuildKit.
- `rust:1-bookworm` contains a Rust toolchain new enough to compile the workspace.
- `node:24-bookworm` provides a Node runtime compatible with the project's
  packages (no `engines` field is present in `package.json`).
- The backend needs no system packages beyond the `rust:1-bookworm` image.
- `cargo watch` uses the host's inotify events through the bind mount on Linux;
  on macOS/Windows Docker Desktop file sharing may require polling.
- Caddy's `reverse_proxy` directive passes the WebSocket `Upgrade` header
  automatically (verified in Caddy documentation, not by execution).
- The wildcard certificate generated by `dev/gen-cert.sh` (`*.crlocal`) is
  sufficient for all five hostnames.
- The images install `pnpm@latest`; pnpm 9 rejects a `pnpm-workspace.yaml`
  without a `packages:` field, which newer pnpm writes for `allowBuilds`.
- No database, Redis, or additional services are required; the backend is
  entirely in-memory.
