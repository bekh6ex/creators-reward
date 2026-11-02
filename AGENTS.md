# PROJECT KNOWLEDGE BASE

Guidance for contributors and coding agents working in this repository.

## OVERVIEW

Privacy-first donation platform: viewers donate to content creators, and what a viewer reads decides how much of that viewer's own donation goes to whom — up to a monthly limit the viewer sets, and never as a debt. The goal is that what a viewer reads is untrackable by design (PRODUCT INVARIANT 4). Rust/axum workspace backend + Preact viewer SPA + Preact creator portal; creator sites embed an iframe served by the backend (`backend/templates/events/web/`). Early stage (pre-alpha): in-memory storage, stub payment provider; the events-token Ed25519 key pair is generated at startup and lives only in memory.

## PRODUCT INVARIANTS (NON-NEGOTIABLE)

These are product law, not preferences. They outrank convenience, performance, and feature
requests. If a change cannot be made without breaking one of them, the change does not happen.

**1. Every transaction is a donation. There is no gated content, ever.**
Nothing is withheld pending payment — no metered access, no "unlock", no donor-only feature,
no degraded experience for a viewer who gives less. Access gating was considered and is
permanently dropped. The reason is load-bearing rather than cosmetic: gating access would make
the money *consideration for a supply*, which converts every donation into taxable revenue for
a service and destroys the donation characterisation the whole model rests on.
Corollary: reading **never** creates a debt. Reading does decide the amount, but only up to the
monthly limit the viewer set; once the total reaches the limit, every share is scaled down
proportionally, so from then on reading only decides the split. Every item stays cancellable
until the month is finalised, and a limit of 0 is always valid (invariant 3).

**2. A creator can never discover whether a given visitor donates to them.**
Not from the portal, not from statistics, not from any API response, not from a webhook, not
from timing, and not by inference from a payout. Concretely forbidden:

- any field, flag, count, or boolean that distinguishes donating from non-donating visitors
- any per-visitor record reaching the creator side at all (statistics are aggregates — see
  `frontend/creator/AGENTS.md`)
- any payout breakdown that is per-viewer rather than per-period

There is no "supporter" concept in this product. Do not add one under any name — not "member",
"backer", "subscriber", or "verified reader".

**3. A monthly limit of 0 is fully supported and behaviourally indistinguishable.**
A viewer who donates nothing gets the same pages, events, history, dashboard, and widget as a
viewer donating 50 eur. That makes the pipeline **amount-blind**: no stage may branch on the
donation amount, skip work for zero-amount viewers, or label them. A "fast path for free
viewers" is a privacy bug, because any observable difference re-creates exactly the
donor/non-donor distinction invariant 2 forbids. The 0 case is what gives every viewer
deniability, so it is a security control rather than a generosity setting — test it as one.

**4. Goal: no trail connecting a reader to what they read — anywhere, in any system.**
Not to an article, and not even to a creator. This is stated separately from invariant 2
because the money layer is a second channel to the same secret: a per-reader-to-per-creator
payment record *is* a reading history, no matter how well the event pipeline behaves.

This is the ultimate goal, not a guarantee every part of the system can meet. Where meeting it
is impossible, the fallback is:

1. **Store as little as possible.** Data linking a reader to content (a plaintext URL, for
   instance) exists only in memory while it is processed; nothing written to storage can be
   attributed to a reader.
2. **Make any remaining correlation controllable**: possible only for a small group of people
   holding cryptographic keys, never for anyone who merely has database or server access.

Falling back is a recorded exception, not a default: note where the goal is not met and why.

The adversary is explicitly not just a curious engineer. It includes:

- the creator, correlating payouts or statistics against their own access logs
- the payment provider, which holds both legs of its own ledger
- **us, under legal compulsion** — a production order, a subpoena, or a hostile acquirer
- a state actor with access to any of the above

So access control is not sufficient. The aim is **inability, not policy**: you cannot be
compelled to produce what you do not hold. Any design in which the operator holds a usable
join between a reader identity and the content they consumed falls short of this goal *even
if nothing ever queries it*, and must use the fallback above.

**The design benchmark.** A reader must be able to donate to an adult-content creator — or to
a site that implies their sexuality, health, religion, or politics — without that fact being
discoverable by the creator, by us, by our payment provider, or by a government compelling any
of us. Measure every change against that test; one that falls short needs the fallback above
and a recorded reason. These categories are not
hypothetical framing: they are GDPR Art. 9 special-category data, and the concrete harm is
outing a person to an employer, a family, or a state that criminalises them.

**Settled consequence for the money layer.** The model is **pool-and-distribute**: many
inbound reader donations, independent per-creator outbound payouts, and no linking reference on
either leg. Split-payment and destination-charge products (Stripe Connect destination charges,
wallet-to-wallet transfers, any "marketplace split") create the reader-to-creator edge by
construction and are forbidden.

## STRUCTURE

```
├── backend/          # Rust Cargo workspace — axum server + 7 lib crates + offline billing-ceremony tool. System of record.
├── frontend/viewer/  # Preact SPA (Vite + Tailwind 4/daisyUI 5) — NOT React. Viewer UI; the dashboard still renders dummy data.
├── frontend/creator/ # Preact SPA, same stack, :5174 — website-owner portal. Accounts + websites real (creator service), the rest faked.
├── e2e-tests/        # Playwright + playwright-bdd scenarios against the dev stack.
├── dev/              # Native dev stack (dev/start.sh, Caddy in front of every host) and the docker-compose alternative (dev/docker/).
├── fake-sites/news/  # Deterministic fake creator sites (website1/website2.crlocal) for the dev stack and e2e.
└── ast-grep/         # Convention rules clippy and oxlint cannot express, with their tests (sgconfig.yml).
```
Root: README.md (values, requirements, License section), ABOUT_PROJECT.md (motivation), REUSE.toml + LICENSES/ (licence of every file: AGPL code, CC-BY-4.0 docs, MIT embed.js — a new third-party file needs its own annotation there), flake.nix (dev shell), justfile, docker-compose.yml, prek.toml, .gitleaks.toml.

## WHERE TO LOOK

| Task | Location | Notes |
|---|---|---|
| Add/modify HTTP route | `backend/src/http/api/viewer/` | nest pattern |
| Service wiring / pipeline | `backend/src/state.rs` `create_state` | mpsc channels, FuturesUnordered |
| Add a service crate | `backend/services/` + root `Cargo.toml` | see `backend/services/AGENTS.md` |
| Auth / tokens / keys | `backend/services/auth/src/tokens/` | EdDSA (Ed25519) JWT; signing key generated at startup, in memory only |
| ID privacy machinery | `backend/services/id_storage/` | internal-only service; other services import only `internal_api` |
| Billing / key ceremony | `backend/services/billing/`, `backend/tools/billing_ceremony/` | flow in `backend/services/billing/src/lib.rs`, dev walkthrough in `dev/README.md` → Billing |
| Events iframe + embed script | `backend/templates/events/web/` | `iframe.html`, `hpke.js`, `embed.js` |
| Viewer SPA page | `frontend/viewer/src/pages/<Name>/` | routes wired in `src/index.tsx` |
| Creator portal page | `frontend/creator/src/pages/<Name>/` | routes wired in `src/app.tsx`; API client `src/creatorApi.ts`, remaining fakes in `src/api.ts` |
| Creator accounts / websites | `backend/services/creator/`, `backend/src/http/creator.rs` | separate service on its own port |
| Dev stack / e2e | `dev/README.md`, `e2e-tests/README.md` | `.agents/skills/worktree-dev/SKILL.md` for agents |
| Dev environment | `flake.nix` | `nix develop`; see COMMANDS |

## CODE MAP

| Symbol | Type | Location | Refs | Role |
|---|---|---|---|---|
| `create_state` | fn | `backend/src/state.rs` | main.rs, tests | Wires all services + queues |
| `axum_app` | fn | `backend/src/http/mod.rs` | main.rs, api/ | Router + middleware assembly |
| `AuthService::register_finish` | fn | `backend/services/auth/src/public_service.rs` | auth.rs handler | Stores OPAQUE record + sealed viewer private key, emits registration events |
| `ViewerRegistered`/`ViewerEventsKeyRegistered` | struct | `backend/common/src/viewer_registration.rs` | auth → id exchange / events | Split registration events: the viewer's ids → IdStorage (`id_storage::internal_api`); events id + public key → events (`common::viewer_registration`) |
| `ValidatedViewerEventsToken` | type | `backend/services/auth/src/tokens/viewer_events.rs` | events/web.rs | Cookie JWT validation |
| `WebViewerEventsService` | struct | `backend/services/web_viewer_events/src/service.rs` | state.rs | Enabled donations + viewer public keys; forwards sealed usage records to billing |
| `IdStorage`/`IdExchanger` | struct | `backend/services/id_storage/src/{service,internal_api}.rs` | state, auth, billing | Privacy boundary. IdStorage = crate-private state of an internal-only service; `create_state` uses `id_storage::create_wiring()` for the registration listener and typed `IdExchanger`s; services import only `internal_api` |
| `StupidAsyncMemoryMap` | struct | `backend/common/src/lib.rs` | in-mem stores | Universal in-mem KV (the events service's viewer public keys) |
| `tagged_id` | proc-macro | `backend/tagged_id/src/lib.rs` | common/ids.rs | `#tag:<uuid>` serde IDs |
| `App`/`Routes` | fn/enum | `frontend/viewer/src/{index.tsx,routes.ts}` | pages | Preact router (all 7 routes wired) |
| `App`/`PortalProvider` | fn | `frontend/creator/src/{app.tsx,store.tsx}` | all creator pages | Creator router (11 routes) + shared portal state |

## CONVENTIONS

- Rust: edition 2024; `backend/.rustfmt.toml` (style edition follows the crate edition): max_width=90, fn_call_width=55, chain_width=50, use_try_shorthand; `[profile.dev] opt-level=3`.
- Today all inter-service flow goes through injected `Sink`/`Stream` channels (futures mpsc, buffer 100) wired via `FuturesUnordered` in `create_state`; the target architecture also permits sync HTTP between services.
- Each service knows only its own tagged ID type; cross-service ID translation ONLY via `IdExchanger`. Never leak `ViewerIdSet` across services.
- Serde DTO fields camelCase (`enabledDonationId`, `publicKey`) to match TS clients.
- Docs describe the current state. History belongs in a doc only when it explains why something that exists is the way it is (a rejected alternative, a recorded exception). Never document what was removed or why something no longer exists ("used to need X, which is gone"): that goes in the commit message, where `git log` finds it.
- Terminology: a viewer *enables donations* for a creator — UI "Enabled donations" / "Enable donations" / "Disable donations", type `EnabledDonation`. Never "subscription": nothing recurs and no fixed amount is committed.
- Services return typed error enums; the gateway maps each enum to an HTTP status through an exhaustive match in `backend/src/http/error.rs`. Handlers may still return `StatusCode` for gateway-only concerns such as cookie extraction or request parsing, but never for a condition the service already models.
- Tests inline `#[cfg(test)] mod tests` at file bottom; HTTP integration via `axum-test` `TestServer` (save_cookies, JSON). `#[ignore]` = fixture-generator utility.
- Rust tests use `rstest` (`default-features = false, features = ["async-timeout"]`; `crate-name` is off since nothing renames the crate): `#[timeout(Duration)]` goes on any test that waits on the wall clock. Every test is `#[rstest]`, tables are named `#[case::name(...)]` (one test per case, never a `for` loop over inputs), repeated setup is a `#[fixture]`. Loops stay only where order matters (a second call depends on the first). Async: `#[rstest]`, then the `#[case]`s, then `#[tokio::test]` — a `#[case]` placed after `#[tokio::test]` is silently dropped except the first; check with `cargo test -- --list`.
- Rust assertions use `googletest` matchers, never `assert!`/`assert_eq!`. Every test starts with `#[gtest]`, placed **above** `#[rstest]` (below it, each test runs twice). Checks are `expect_that!(actual, matcher)`: they fail the test but keep running, so one run reports every broken check. `assert_that!` only where the rest of the test cannot run after a failure. Prefer a matcher that says what is checked over `is_true()` on a computed bool: `err(eq(&E::X))`, `elements_are![..]`, `field!(&T.f, ..)`, `pat!(E::V { .. })`, `contains_substring(..)`. Matchers take a non-`Copy` value by reference, so compare with `eq(&expected)`; a `Copy` value takes `eq(expected)`. The compiler says which one applies. A type holding a secret gets `Debug` by wrapping the secret in `secrecy::SecretString` (redacted), never by printing it, so a test can match it like any other value. Failures are recorded per thread: keep `expect_that!` on the test's own thread (`#[tokio::test]` is current-thread; a sync `#[timeout]` runs the body on another thread). axum-test's `assert_status`/`assert_json` stay: they are fatal and print the request.
- Viewer is Preact — import from `preact` directly; `jsxImportSource: preact`.
- pnpm only (never npm/yarn).
- A commonly used tool missing from the environment (e.g. `jq`) is a reason to add it to `flake.nix` `nativeBuildInputs`, not to work around it with an ad-hoc script. Add it in the same change that needed it.
- New dependencies use the latest released version unless there is a concrete problem with it (record the reason next to the pin). Check the registry, not the local cache: the cargo index cache goes stale (`cargo info` reported 0.27.0 when 0.28.0 was out), so confirm with `cargo search <crate>`. In 0.x, read the changelog first — a minor bump is a major one.

## ANTI-PATTERNS (THIS PROJECT)

- Never reintroduce gated, metered, or paid-for access under any name — PRODUCT INVARIANT 1.
- Never add anything that lets a creator tell donating from non-donating visitors — PRODUCT INVARIANT 2. This includes "harmless" aggregates that become distinguishing at low visitor counts.
- Never branch on the donation amount in the event/decision pipeline, and never special-case a 0 limit — PRODUCT INVARIANT 3.
- Never hold a usable operator-side join between a reader identity and the content they consumed — PRODUCT INVARIANT 4. Where one truly cannot be avoided, apply its fallback (memory only, or usable only with keys a small group holds) and record the exception. `IdStorage` currently meets neither the goal nor the fallback (all three ids per viewer, plaintext, one record): redesign it before building anything new on it.
- Never use split-payment or destination-charge PSP products — PRODUCT INVARIANT 4. They create the reader-to-creator edge by construction.
- Never put PII (incl. user ids) in tokens — README hard rule: metadata only.
- Never store trackable data (URLs, media IDs) unencrypted; plaintext may exist in memory only, while an event is processed. Timestamps 15-min precision.
- Never point a dev hostname at squatted public IP space. `53.0.0.0/8`, for instance, is allocated to Daimler AG: it is globally routable, so the name resolves to an address with no local route and every connection just times out. Use `127.0.0.1`, or `192.0.2.0/24` (RFC 5737 TEST-NET-1) when a distinct non-loopback address is genuinely needed.

## UNIQUE STYLES

- `web_` prefix = web-platform adapter; `internal_*` = cross-service modules; `public_*` = HTTP-bound.
- `wiring.rs` pattern (billing, id_storage) centralizes service construction.

## COMMANDS

`just` is the repo-wide task runner: every project directory has a `justfile`,
and `just --list` inside it shows that project's recipes. From the repository
root, `just --list --list-submodules` shows all projects, and recipes are
reachable as `just <project>::<recipe>` (for example, `just backend::check` or
`just frontend::viewer::check`). Run `nix develop` first to get the tools.

```bash
nix develop
just backend::run              # HTTPS backend on 127.0.0.1:3000 (local TLS pair in backend/cert/)
just backend::test             # workspace tests
just backend::bench            # criterion benchmark for auth
just backend::check            # full gate: fmt, clippy, deny, vet, shear, cov, doc-tests
just frontend::viewer::dev     # viewer Vite dev server
just frontend::viewer::check   # typecheck + lint + knip + tests @ 85%
just frontend::creator::dev    # creator portal Vite dev server on :5174
just frontend::creator::check  # typecheck + lint + knip + tests @ 85%
just e2e-tests::test           # Playwright + BDD scenarios
just e2e-tests::qa             # tsc + prettier + bddgen wiring
just dev::start                # native dev stack, foreground
just dev::start-detach         # native dev stack, background
just dev::stop                 # stop the running stack
just dev::status               # show running slots
just ast-grep-test             # ast-grep convention rule tests
just ast-grep-scan             # ast-grep convention scan
just gitleaks                  # secret scan of full git history
just reuse                     # licence/copyright check
just prek                      # pre-commit checks on all files
just check                     # root checks + backend, viewer, creator gates
```

No CI, no root Dockerfile, no Makefile — `just` is the repo-wide task runner.
`docker-compose.yml` is an alternative containerised dev stack; otherwise
recipes call `pnpm run` and `cargo` directly.

**Pre-commit hook (prek).** `prek install` installs the git pre-commit hook, which runs
`prek run`. Checks live in
`prek.toml` at the root and in `backend/`, `frontend/viewer/`, `frontend/creator/`,
`e2e-tests/`; a project's checks run only when one of its files is staged. Only checks that
take seconds belong there (backend: fmt, clippy, shear, deny bans+licenses+sources; frontends: tsc,
oxlint, knip, related vitest tests; e2e: tsc, prettier; root: gitleaks on the staged changes,
config and allowlist in `.gitleaks.toml`, `reuse lint` on every commit, and the ast-grep
rules and their tests, see
`backend/AGENTS.md` → STATIC ANALYSIS). `prek run --all-files` checks everything;
`git commit --no-verify` bypasses (WIP only).

## LOCAL HOSTNAMES

The dev stack serves `website1`/`website2`/`app`/`creator`/`api.crlocal` through one Caddy port, all on `127.0.0.1` (`dev/README.md`). A test that needs a distinct, non-loopback address uses `192.0.2.0/24`. Containers (this repo's dev container included) run without `CAP_NET_ADMIN`, so `ip addr add` fails there with `RTNETLINK answers: Operation not permitted`; either grant it at the runtime (`podman run --cap-add=NET_ADMIN`) or work inside a private network namespace (`unshare -Ur -n`, `util-linux` and `iproute2` are in `flake.nix`), from which every process that must reach the address has to be started.

## NOTES

- Events side: the iframe (in `backend/templates/events/web/`) enables and disables donations (browser-sealed records) and posts sealed billing usage records; billing collects them. The reading-session pipeline and the viewer's reading history are not implemented yet.
- Viewer keys: the browser generates an X25519 key pair at registration (`frontend/viewer/src/viewerKeys.ts`). The server stores the private key only sealed under a key derived from the OPAQUE export key (auth `OpaqueViewerRecord`), and the public key only in the events service (`web_viewer_events::keys`), keyed by events id. The unsealed private key lives in IndexedDB as a non-extractable `CryptoKey` for as long as the session.
- Viewer auth is real: `@serenity-kit/opaque` drives `POST /api/auth/register/start|finish`, `POST /api/auth/login/start|finish`, `POST /api/auth/logout`, and `GET /api/auth/session` via `authApi.ts`. Enabled donations are real and revocable: the events iframe seals `{site, enabledAtSec}` to the viewer's public key in the browser (`backend/templates/events/web/hpke.js`) and POSTs only the record to `/api/events/web/enabled-donations`, so the server never receives the site. It keeps the returned record id in its per-site (partitioned) storage. The viewer's Enabled donations page opens the records (`hpke.ts`, `enabledDonationsApi.ts`), and Disable `DELETE`s every record id of a site; the iframe can disable too. Usage records must carry a live `enabledDonationId`, so revoking stops collection. `embed.js` lives at `/api/embed.js`, outside the events cookie path, and loads the iframe with `referrerpolicy=no-referrer`; the iframe gets the site from the parent's postMessage origin. The dashboard is still dummy data in `src/api.ts`.
- Billing is real end to end, with a stub payment provider: the iframe seals one record per page view of a site with donations enabled to the current period's key (`GET /api/billing/period-key`, no cookie) and posts it to `/api/events/web/usage`; billing stores it under the billing id. At period end (month, or `CR_BILLING_PERIOD=120s`) it exports a shuffled batch without reader ids; key holders fetch it from the admin gateway (own port `CR_ADMIN_PORT`, loopback, hardcoded bearer token, dev only), two of three run `billing-ceremony tally` offline, which returns only per-site totals, and upload them; only then are readers charged `min(limit, n·price)` and sites paid. The server never holds the private key. Nothing is collected until a period's public key has been uploaded (`dev/README.md` → Billing).
- Sessions persist across reload via `GET /session` and the `cr_session` cookie (24h TTL). A backend restart invalidates all OPAQUE registrations because `ServerSetup` and the credential store are in-memory (accepted dev-stage limitation).
- `frontend/creator` is a standalone copy of the viewer's stack (own lockfile, no pnpm workspace); config, `style.css`, `Link`/`Menu`/`AuthenticatedHeader`/`AuthenticatedLayout` and `test/setup.ts` are duplicated on purpose — extracting a shared package is a deliberate follow-up.
- **Creator accounts and websites are real** (`backend/services/creator`, portal client `frontend/creator/src/creatorApi.ts`). It mimics a separately deployed service: its own listener (`CR_CREATOR_API_PORT`, dev stack: `creator.crlocal/api/*`), its own state, and routes under plain `/api`. Email + password, a `cr_creator_session` cookie, and a registry of bare domains, one creator per domain. Shortcuts: plaintext passwords, a site is connected without any ownership check, in memory only. The registry decides nothing yet: the iframe accepts any site and billing pays the site origin, not its creator. Payouts, statistics, profile and password recovery are still faked in `frontend/creator/src/api.ts`.
- Creator portal fakes worth knowing: no email is sent, so the reset link is shown on screen. IBAN validation is real (MOD-97-10 + per-country lengths, EU-only for now: EU first, other regions later).
- Password recovery exists in the creator portal but not in the viewer: the viewer's password derives its history encryption key, so a plain reset would make the history unreadable, while a creator's password encrypts nothing. Whether a viewer recovery that keeps the history readable is possible without weakening privacy is open research.
