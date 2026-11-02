# BACKEND KNOWLEDGE BASE

## OVERVIEW

Rust Cargo workspace: backend bin + workspace members (services, common, tagged_id, tools/billing_ceremony). Axum 0.8.9 + axum-server 0.8.0 (rustls 0.23 TLS) + tower 0.5.3/tower-http 0.7.1/tower-cookies 0.11.0. Binds 127.0.0.1:3000 HTTPS.

## STRUCTURE

src/
  main.rs                 Spawn FuturesUnordered listeners
  state.rs                AppState, AdminState, Config (api_prefix "/api"), create_state(s)
  queue.rs                mpsc channels, buffer 100
  http/
    mod.rs                axum_app (viewer gateway) + admin_app (admin gateway) + creator_app (creator service), shared middleware, start_http_server
    api/viewer/           Route nests
    creator.rs            Creator service API: own listener, own CreatorState
    admin/                Admin gateway: bearer DEV_ADMIN_TOKEN (hardcoded, dev only), /admin/billing/*
common/
  lib.rs                  StupidAsyncMemoryMap (in-mem KV) + re-exports (bytes, uuid)
  ids.rs                  Tagged ID types
  viewer_registration.rs  ViewerEventsKeyRegistered + ViewerPublicKey (auth → events)
tagged_id/                Proc-macro: #[tagged_id(tag="VA")] → serde "#tag:<uuid>"
tools/billing_ceremony/   Offline key ceremony (bin billing-ceremony: keygen, tally) — never runs on the server
cert/                     Local TLS PEMs for DevTls (gitignored, not committed)
services/                 Has own AGENTS.md; do not detail crates

## WHERE TO LOOK

| Task | Location | Notes |
|---|---|---|
| Add route | src/http/api/viewer/ | Nest under api/viewer/ |
| Middleware changes | src/http/mod.rs axum_app | ServiceBuilder: redact AUTHORIZATION/COOKIE/SET_COOKIE/REFERER, TraceLayer+body chunks, 10s Timeout (408), compression |
| Pipeline wiring | src/state.rs create_state | Channels + service starts |
| Config | src/state.rs | api_prefix = "/api" |
| Integration tests | src/http/mod.rs mod tests | axum-test TestServer |
| TLS config | src/http/mod.rs, src/main.rs | TlsMode: CR_TLS_CERT + CR_TLS_KEY → HTTPS; CR_BIND_ADDR or CR_BACKEND_PORT → plain HTTP (behind Caddy); neither → DevTls, PEMs from cert/ via CARGO_MANIFEST_DIR (not committed: put a local pair there, e.g. the leaf from dev/gen-cert.sh) |

Route tree:
POST /api/auth/register/start|finish, /api/auth/login/start|finish (OPAQUE) → events_token (path=/api/events, SameSite=None, 365d) + cr_session (path=/, SameSite=Lax, 24h); login/finish body returns the sealed viewer private key
POST /api/auth/logout, GET /api/auth/session
GET /api/embed.js → creator-site embed script (outside the /api/events cookie path on purpose)
GET /api/events/web/iframe.html, /api/events/web/hpke.js → events iframe + its HPKE sender
GET /api/events/web/public-key; POST/GET /api/events/web/enabled-donations; GET/DELETE /api/events/web/enabled-donations/{id} → browser-sealed records, events token
POST /api/events/web/usage → billing record sealed in the iframe + live enabledDonationId → 202/401/409 (not the current period)/422
GET /api/billing/period-key → current period's public key, no cookie needed → 200/503

The reading-session endpoint and the dashboard are not implemented yet: no route reads a reading session, and no route serves the viewer's view history (the SPA renders dummy data).

Admin gateway — own listener on 127.0.0.1:CR_ADMIN_PORT (default backend port + 1), never behind Caddy, own AdminState (viewer handlers cannot reach it), every route needs Authorization: Bearer <DEV_ADMIN_TOKEN>:
PUT /admin/billing/periods/{period}/key {publicKey} → 204 (also for the same key again) / 409 (another key, closed or ended period) / 422
GET /admin/billing/periods/{period}/batch → 200 Batch (from close until settled) / 404
POST /admin/billing/periods/{period}/totals Totals → 200 {chargedCents, paidOutCents}, charges + payouts / 409 not awaiting / 422
GET /admin/billing/log → [LogEntry]

Creator service. Mimics a separately deployed service: own listener on CR_BIND_ADDR:CR_CREATOR_API_PORT (default backend port + 2; dev stack: slot +6, Caddy routes creator.crlocal/api/* to it), own CreatorState built in main.rs outside create_states (shares nothing with the viewer pipeline), own cookie cr_creator_session (path /api, HttpOnly, Secure, SameSite=Strict, 24h):
POST /api/auth/register {email,password} → 201 + cookie / 409 email taken / 422 (password < 8)
POST /api/auth/login {email,password} → 204 + cookie / 401
POST /api/auth/logout → 204; GET /api/auth/session → {email} / 401
GET /api/websites → [Website] / 401; POST {domain,title} → 201 Website (status verified, no ownership check yet) / 409 domain taken by anyone / 422 not a bare domain

Auth pattern: NO middleware/extractor. Inline in web.rs events_viewer: cookie → RawViewerEventsToken → ValidatedViewerEventsToken::try_from(pub_key from InternalAuthService) → ViewerEventsId.

Wiring graph:
usage_tx → billing usage_listener (ViewerEventsId→ViewerBillingId via IdExchanger) → Billing; billing period_closer ticks every 1 s, reads the injected clock, and closes ended periods into batches; keys and totals arrive through the admin gateway
registered_tx → id_storage registration_listener; key_registered_tx → ViewerPublicKeys listener; the viewer gateway calls web_events directly (enabled donations, public key), and web_events forwards the sealed usage records to billing over usage_tx
create_states(config, payments) → (AppState, AdminState, listeners); create_state(config) → (AppState, listeners) for viewer-only tests
Env (main.rs): CR_ADMIN_PORT, CR_CREATOR_API_PORT, CR_BILLING_PERIOD (month | <n>s), CR_BILLING_UNIT_PRICE_CENTS (10), CR_BILLING_DEFAULT_LIMIT_CENTS (1000)

## CONVENTIONS

- futures::lock::Mutex everywhere (runtime-agnostic; NOT tokio::sync)
- In-memory stores = `StupidAsyncMemoryMap` (common) or a `futures::lock::Mutex<HashMap>`
- Integration test idiom: `#[gtest]` + rstest fixtures in src/http/mod.rs tests (checks via googletest `expect_that!`, status via axum-test `assert_status`; root AGENTS.md → CONVENTIONS) — `viewer: Viewer` (create_state(Config::default()) + axum_app; fields server, state, listeners to poll) and `#[from(creator)] (server, _tracing)`; both via create_server (TestServerConfig {default_content_type: json, save_cookies: true})
- Handlers: tracing::debug!/info!, then either `Json<T>`/`StatusCode` directly for gateway-only concerns or `.map_err(|e| e.status())?` for service errors via `http::error::HttpError`.

## STATIC ANALYSIS

- Full gate: `just check` (or `just backend::check` from the root), run it before handing off backend work. Steps, in order: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo deny check advisories bans licenses sources`, `cargo vet --locked`, `cargo shear`, `cargo llvm-cov --workspace --fail-under-lines 90 --no-report`, `cargo test --workspace --doc`. Line coverage is ~90%; the `billing_ceremony` bin's `main.rs` is covered by unit tests that exercise the offline commands and the admin-gateway HTTP clients against a local fake server. `cargo llvm-cov` does not run doc-tests, so they are kept as a separate step. Every step is reported; the recipe exits non-zero and lists the steps that failed.
- Pre-commit hook (`backend/prek.toml`): `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo deny check bans licenses sources`, `cargo shear`. No tests and no `cargo deny check advisories` (advisories need the network and can change without any commit).
- `[workspace.lints]` in `backend/Cargo.toml`: `rust.unsafe_code = "forbid"`; `rust.unreachable_pub = "warn"` (rustc never reports unused `pub` items; narrowing them to `pub(crate)` lets `dead_code` see code that is dead across crates); `clippy.dbg_macro = "warn"`, `clippy.float_arithmetic = "warn"` (money stays in integer cents/millicents), `clippy.allow_attributes = "warn"` and `clippy.allow_attributes_without_reason = "warn"` (an exception is `#[expect(lint, reason = "...")]`, which warns once the lint stops firing); `clippy.unwrap_used = "warn"` and `clippy.expect_used = "warn"` (every panic path is explicit and justified, or propagated as an error); `clippy.arithmetic_side_effects = "warn"` (integer overflow is impossible or handled with `checked_*`/`saturating_*`, especially in money code); `clippy.todo = "warn"` (no committed `todo!()` stubs); `clippy.significant_drop_tightening = "warn"` (async lock guards are dropped as early as possible). Every member opts in with `[lints] workspace = true`.
- `backend/clippy.toml`: `allow-unwrap-in-tests = true` and `allow-expect-in-tests = true` so `#[cfg(test)]` code can fail fast; benchmarks are not covered and carry their own `#[expect]`.
- `[profile.release] overflow-checks = true` in `backend/Cargo.toml`: release builds panic on integer overflow rather than wrapping. It applies to dependencies too; none of them is known to rely on wrapping (`ring` < 0.17, RUSTSEC-2025-0009, is not in the graph).
- Kani model checking for billing money math: `cargo install --locked kani-verifier && cargo kani setup` once, then `cargo kani -p billing` (also available as `just kani`). The `#[cfg(kani)] mod proofs` harnesses in `services/billing/src/proofs.rs` prove that `charge` returns `amount <= limit`, `weight_millicents * units <= amount * 1000`, and never panics for all `u64` inputs (bounded by realistic cent ranges so the assertion multiplications stay below `u64::MAX`). Not included in `just check` because it needs Kani's own nightly toolchain.
- `cargo mutants -p billing -p auth --timeout 120 -j 2` (`just mutants`; output in the gitignored `mutants.out/`) mutates the code and reports every change no test notices. Run it after non-trivial changes to `billing` or `auth`; it takes minutes, so it is in neither the gate nor the hook. Expected survivors: Kani-only `services/billing/src/proofs.rs` and `BillingConfig` Debug.
- `backend/clippy.toml` states product invariants as clippy rules, tests included:
  - `secrecy::ExposeSecret::expose_secret`: every place a secret leaves its wrapper.
  - `SystemTime::now`, `Instant::now`, `chrono::Utc::now`, `time::OffsetDateTime::now_utc`: clock reads. Reader-event timestamps are rounded to 15 minutes in the browser before sealing (`templates/events/web/iframe.html`); the backend reads the clock only for login and session TTLs, billing periods, the admin log and creator metadata, and timing can correlate a reader (invariant 2).
  - `id_storage::internal_api::ViewerIdSet`: never leak it across services, translate ids through `IdExchanger`.
  - A path from a crate not every member depends on needs `allow-invalid = true` (the path need not resolve).
  - Every allowed use carries `#[expect(clippy::disallowed_methods, reason = "...")]` or `#[expect(clippy::disallowed_types, reason = "...")]`, and the reason must be true for that specific site, not boilerplate copied from another site.
- `backend/deny.toml` (cargo-deny):
  - `advisories`: every ignore names the dependency that pulls it in, whether it is dev-only, and why it cannot be fixed yet.
  - `bans`: `billing` is allowed directly only under `backend` and `billing_ceremony`, so the event-ingest side cannot name `EuroCents` (invariant 3 becomes structural); `id_storage` only under `backend`, `auth`, `billing`.
  - `[bans.workspace-dependencies]`: a dependency used by two or more members must come from `[workspace.dependencies]` (path dependencies included), and unused workspace entries are errors.
  - `multiple-versions = "warn"`; `sources` allows crates.io only.
  - `[licenses]`: a dependency must be under an allowed licence: AGPL-3.0-or-later (the project's own, REUSE.toml) or a permissive one it can carry. A new licence fails the check and is added to `allow` as a decision.
  - CLI: `--config` is a top-level flag (`cargo deny --config x.toml check ...`), and a bare `cargo deny check` also runs `licenses`, so always name the checks.
- `backend/supply-chain/` (cargo-vet; `config.toml`, `audits.toml`, `imports.lock`): imports the audit sets of Google, Mozilla, Bytecode Alliance, Embark, ISRG and Zcash; every dependency those do not cover is an `[[exemptions.*]]` entry, and `audits.toml` holds no audits of our own. `cargo vet --locked` (a `just backend::check` step, not in prek) runs offline against `imports.lock`. A new or upgraded dependency fails the gate until it is audited or exempted: `cargo vet` shows what is unvetted, `cargo vet regenerate exemptions` accepts it and is a conscious decision. `cargo vet import <name>` / `cargo vet` refresh `imports.lock`, which needs the network.
- Dead code is removed, never annotated with allow/expect.
- ast-grep (`sgconfig.yml` and `ast-grep/rules/` at the repo root, each rule tested in `ast-grep/tests/`) checks what clippy cannot see: `#[gtest]` above `#[rstest]`, no `#[case]` after `#[tokio::test]`, no `assert!`/`assert_eq!`/`assert_ne!` in tests, `#[serde(rename_all = "camelCase")]` on serde types with named fields, and no supporter/backer/subscriber/subscription/membership identifiers (Rust, TS, TSX). An exception is `// ast-grep-ignore: <rule-id>` on the line above the item, with the reason on the line above that; the tree has none today.

## ANTI-PATTERNS

- Never `match` on a service error in a handler to pick a `StatusCode`: the mapping lives in `http::error::HttpError`, one exhaustive `match` per enum; handlers call `err.status()`.

## NOTES

- state.rs InternalAuthService::viewer_event_token_pub_key() returns a reference to the Ed25519 verifying key generated at startup.
- tracing FmtSubscriber DEBUG global, TRACE per-test.
