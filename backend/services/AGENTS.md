# SERVICES KNOWLEDGE BASE

## OVERVIEW

Actor-style service crates, each generic over injected `Sink<T>`/`Stream<T>` futures channels.

## STRUCTURE

| Crate | Role | State | Details |
|---|---|---|---|
| auth | OPAQUE auth, sessions, events token | ACTIVE | public_service.rs: AuthService<RS,KS> register_start/finish, login_start/finish, logout, session; register_finish stores the browser-sealed viewer private key (opaque blob) and emits ViewerRegistered + ViewerEventsKeyRegistered into its two sinks. opaque/: CrOpaque suite + in-memory CredentialStore/PendingLoginStore/SessionStore. internal_service.rs: Ed25519 verifying key for events-token validation. tokens/: jsonwebtoken 11.1 EdDSA, Claims{aud,sub:TaggedId,iat,nbf,exp=+1h,jti}, Audience enum; viewer_events.rs carries no custom claims. The signing key is generated at startup in backend/src/state.rs and lives only in memory. benches/argon2.rs criterion, [[bench]] harness=false. |
| id_storage | Id exchange (internal-only) | ACTIVE | internal_api.rs: everything other services import — ViewerIdSet, ViewerRegistered (auth → id exchange, the viewer's three ids in one message), IdExchange trait, IdExchanger<Id1,Id2> (typed one-way translation handle, only constructible inside the crate). wiring.rs: create_wiring() → Wiring{exchanger::<Id1,Id2>(), registration_listener(stream)} — composition root (state.rs) only, never in AppState or a service. service.rs: IdStorage, crate-private state. |
| web_viewer_events | Enabled donations + viewer public keys + billing usage | ACTIVE | enabled_donations.rs: EnabledDonations — stores browser-sealed records (exactly SEALED_LEN = 304 B, HPKE X25519/HKDF-SHA256/AES-256-GCM, info creators-reward/v1/enabled-donation; format in the module doc) under random record ids: add/list/contains/remove per events id; never sees the site (in-memory). hpke is a dev-dependency only: `documented_format_is_standard_hpke` pins the format, `print_browser_fixture` (#[ignore]) regenerates the viewer's HPKE fixture. keys.rs: ViewerPublicKeys (events id → X25519 public key), filled by a ViewerEventsKeyRegistered listener; the only place the public key is stored. service.rs: WebViewerEventsService<Sink<UsageRecorded>> — record_usage forwards sealed billing records (common::billing_usage) to billing. |
| billing | Period charges + payouts | ACTIVE | No crypto: stores records the iframe sealed to a per-period key and cannot open them. period.rs: PeriodLength{CalendarMonth (default), Seconds(n)} → PeriodId. ceremony_format.rs: record format (304 B, HPKE info creators-reward/v1/billing-usage/<period>), PeriodPublicKey, Batch{period, records:[{weightMillicents, sealed}]}, Totals{perSiteMillicents} — shared with tools/billing_ceremony. core.rs: Billing (crate-private, in memory) add_period_key (same key again ok, another refused)/record_usage/close (T=min(limit, n·price), weight T/n, reader ids stripped, shuffled, batch kept until settled)/settle, plus LogEntry log {keyAdded, closed, settled}. service.rs: PublicService (viewer gateway: current period key). admin_service.rs: AdminService (admin gateway only: add_period_key, batch, settle → PaymentProvider, log). payments.rs: PaymentProvider + StubPaymentProvider (records + logs). wiring.rs: BillingConfig, create_wiring → public_service, admin_service(payments), usage_listener (IdExchanger<ViewerEventsId,ViewerBillingId>), period_closer (tick() reads the injected clock; state.rs drives it every second). |
| creator | Creator accounts + website registry | ACTIVE | public_service.rs: CreatorService (Clone, one futures Mutex around repository.rs Repository, so email/domain uniqueness and insert are one step): register (email trimmed+lower-cased, password ≥ 8 → session id) / login / logout / session → Creator{id, email} / websites(creator) / add_website(creator, domain, title) → Website (camelCase DTO = the portal's type; random crv- token; status always Verified). Shortcuts: plaintext passwords, no ownership check, in memory, no session expiry. Used by nothing in the viewer pipeline yet; served on its own port (backend src/http/creator.rs). |

## WHERE TO LOOK

| Task | Location |
|---|---|
| Add token audience | auth/src/tokens/ (one module per audience) |
| Viewer key lifecycle | frontend/viewer/src/viewerKeys.ts, web_viewer_events/src/keys.rs |
| Service construction pattern | billing/src/wiring.rs |
| Browser-sealed donation records | web_viewer_events/src/enabled_donations.rs |
| ID translation | id_storage/src/internal_api.rs (IdExchanger) |
| Billing money types | billing/src/money.rs (EuroCents) |
| Billing record/batch formats | billing/src/ceremony_format.rs |

## CONVENTIONS

- Crate layout: service.rs + repository.rs (+ internal_interface.rs for inbound cross-service types, public_* for HTTP-bound, wiring.rs optional centralizer).
- Generic over Sink/Stream — no runtime/tokio deps inside service logic.
- futures::lock::Mutex.
- Each service defines its own thiserror::Error, returned as Result<T, Error<SinkError>>.
- Secrets in memory: `secrecy::SecretString`/`SecretBox` for values that must not reach `Debug` or logs (session ids, events tokens, key shares), exposed only through an explicit `expose_secret()`; `zeroize::Zeroizing` for byte buffers that are only wiped. Neither does `mlock`/`mprotect` (ceremony hardening is still open).

## NOTES

- EdDSA (Ed25519) in auth is the server keypair that signs the events JWT. The key pair is generated in backend/src/state.rs at startup and lives only in memory; a restart invalidates existing events tokens, as it already loses every OPAQUE registration. Viewer keys are X25519, generated in the browser; the server never holds the viewer private key unsealed. Loading the signing key from a file is not implemented yet.
- argon2 dep present but only used by criterion bench.
- Token expiry 1h.
