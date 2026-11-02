---
name: worktree-dev
description: Start the local dev stack (2 fake news sites + rust backend + viewer SPA + creator portal behind one HTTPS reverse proxy) inside a git worktree, without Docker, and run e2e tests against it. Use when you need a running instance of this project to verify behaviour in a browser, drive Playwright, curl the API, or check that a code change actually works. Triggers: run the app, start the stack, start the dev server, e2e test, Playwright against this repo, verify in a browser, is it actually working, website1.crlocal, app.crlocal, api.crlocal.
---

# Worktree dev stack

Boots five services behind a single Caddy HTTPS port. Native processes, no containers, so several worktrees can each run their own stack at once.

```
https://website1.crlocal:<PORT>   fake news site 1   (node, fake-sites/news)
https://website2.crlocal:<PORT>   fake news site 2   (same code, different env)
https://app.crlocal:<PORT>        viewer SPA         (Vite dev server, HMR)
https://creator.crlocal:<PORT>    creator portal     (Vite dev server, HMR; /api/* → creator service)
https://api.crlocal:<PORT>        rust backend       (cargo watch)
```

Everything is edit-and-refresh. Do NOT restart the stack after changing code — see [Code changes](#code-changes).

## Start it

```bash
nix develop                          # required: provides caddy, cargo-watch, flock
just dev::start-detach               # blocks until all five answer, then exits 0
```

`start-detach` is the form to use when you are scripting or about to run tests. Bare `just dev::start` runs in the foreground with multiplexed logs and tears everything down on Ctrl-C.

Then:

```bash
just dev::status   # which slots are up, pid per service
just dev::stop     # stop the slot owned by this worktree
```

## Never hardcode the port

Each worktree gets its own **slot** (0-9), and the slot decides every port:

```
public HTTPS  = 18090 + SLOT          # docker-compose uses 18080 — do not collide
internal base = 19000 + SLOT * 10     # backend +0, viewer +1, creator +2, site1 +3, site2 +4, admin +5 (loopback, no Caddy), creator-api +6 (creator.crlocal/api/*)
```

`start.sh` picks the lowest slot whose public port *and* whole internal block are free, so a second worktree lands on the next slot automatically. Read the resolved URLs instead of guessing:

```bash
SLOT=$(cat dev/.state/current-slot)           # written by start.sh
cat "dev/.state/$SLOT/urls.json"
# {"website1":"https://website1.crlocal:18090", ... ,"api":"https://api.crlocal:18090"}
```

`dev/.state/<SLOT>/env` holds the same values as shell exports (`CR_PORT`, `CR_BACKEND_PORT`, …) for sourcing.

## e2e tests

One CA is shared by every worktree and clone on the machine
(`~/.local/share/creators-reward/cert`, overridable with `CR_CERT_DIR`); each
checkout's `dev/cert` is a symlink to it. So `dev/cert/ca.crt` works from
wherever you are, and trusting it in a browser is a once-per-machine job:

```bash
curl --cacert dev/cert/ca.crt "$(jq -r .api "dev/.state/$SLOT/urls.json")/api/viewer/dashboard/views"
```

For Playwright, launch with the repo's Chromium and trust that CA. Feed base URLs in from `urls.json` — never a literal port:

```ts
// playwright.config.ts
const slot = readFileSync('dev/.state/current-slot', 'utf8').trim();
const urls = JSON.parse(readFileSync(`dev/.state/${slot}/urls.json`, 'utf8'));

export default defineConfig({
  use: {
    baseURL: urls.app,
    ignoreHTTPSErrors: true,                      // or load dev/cert/ca.crt
    launchOptions: { executablePath: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH },
  },
});
```

Article content is **deterministic**: each fake site seeds its corpus from `SITE_SEED`, so headlines and article bodies are byte-identical across restarts and safe to assert on. `website1` and `website2` use different seeds and never share a headline.

Why real hostnames and not `localhost`: this product's whole point is third-party-cookie and cross-site iframe behaviour. `127.0.0.1` is a "potentially trustworthy" origin and gets relaxed treatment that production never will, which makes any such test meaningless.

## Code changes

| You edit | What happens | Cost |
|---|---|---|
| `frontend/*/src/**` TS/TSX | Vite HMR pushes it over `wss` through the proxy | instant |
| `backend/src`, `backend/services`, `backend/common` | `cargo watch` rebuilds and rebinds; Caddy reconnects on its own | ~13 s incremental |
| `fake-sites/news/server.js` | **not watched** — restart the stack, or run that one site by hand | manual |

A `cargo watch` rebuild drops the API for a few seconds. If a test hits `api.crlocal` right after a Rust edit, poll until it answers rather than asserting immediately.

## First run in a fresh worktree

1. **The branch must contain `dev/` and `fake-sites/`.** A worktree created from a commit that predates them has no harness to run.
2. **Install the per-package deps** — `node_modules` is gitignored, so a new worktree has none:
   ```bash
   (cd frontend/viewer && just install)
   (cd frontend/creator && just install)
   (cd fake-sites/news && pnpm install)
   ```
   pnpm only, never npm/yarn. The shared pnpm store makes this fast.
3. **`/etc/hosts` is global** and already set up once per machine, not per worktree. `start.sh` aborts with the exact command if the entries are missing:
   ```
   127.0.0.1 website1.crlocal website2.crlocal app.crlocal creator.crlocal api.crlocal
   ```
4. **Certificates are automatic and shared.** `start.sh` runs `dev/gen-cert.sh` when `dev/cert` is missing or the CA expires within 7 days. A new worktree links to the machine's existing CA rather than minting its own, so there is nothing to re-trust.
5. **The first backend build is cold** — a fresh worktree has its own empty `backend/target`, so expect minutes, not seconds. `--wait-ready` waits it out; later starts reuse the cache.

## Troubleshooting

**`api.crlocal` returns 502** — Caddy found nothing on `CR_BACKEND_PORT`. Read `dev/.state/<SLOT>/logs/backend.log`; it is nearly always a cold build still running or a compile error.

**`api.crlocal` returns 404** — correct. The backend has no route at `/`; use a real path such as `/api/viewer/dashboard/views`.

**Vite replies "Blocked request / host not allowed"** — the dev server was started without `CR_PORT`/`CR_VIEWER_HOST`, so `allowedHosts` never got the public hostname. Start through `dev/start.sh`, which exports them.

**`No free slot found (0-9)`** — ten stacks are up. `just dev::status`, then `just dev::stop-slot <slot>`.

**Port busy but the slot looks free** — a crashed run left a lock. Confirm with `just dev::status`, then `rm dev/.state/<slot>/lock`.

**Browser rejects the certificate** — import `dev/cert/ca.crt` once per machine; every worktree resolves that symlink to the same CA. Inside an iframe there is no click-through, so an untrusted cert silently kills the frame. The CA lives 30 days; after that `start.sh` regenerates it and you re-import once.

## Rules

- Never point a `.crlocal` hostname at public IP space. `53.0.0.0/8` belongs to Daimler AG and is globally routable, so it resolves to an address with no local route and every connection times out. Use `127.0.0.1` for these five names, or `192.0.2.0/24` (RFC 5737 TEST-NET-1) when a distinct address is genuinely needed.
- Never give a service its own certificate. Caddy terminates TLS once at the edge; every upstream is plain HTTP on its internal port.
- Never hardcode `18090` or an internal port. The formula lives in `dev/start.sh` (`PUBLIC_BASE`, `INTERNAL_BASE`); everything else reads `urls.json` or `env`.
