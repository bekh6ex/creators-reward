use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use ed25519_dalek::pkcs8::EncodePrivateKey;
use ed25519_dalek::{SigningKey, VerifyingKey};
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use rand::rngs::OsRng;

use auth::internal_service::InternalAuthService;
use auth::public_service::{AuthService, AuthServiceConfig};
use auth::{TokenSigningKey, TokenVerificationKey};
use billing::BillingConfig;
use billing::payments::PaymentProvider;
use common::billing_usage::UsageRecorded;
use common::viewer_registration::ViewerEventsKeyRegistered;
use id_storage::internal_api::ViewerRegistered;
use web_viewer_events::keys::ViewerPublicKeys;
use web_viewer_events::service::WebViewerEventsService;

use crate::queue::{InMemSink, create_memory_queue};

#[derive(Clone)]
pub(crate) struct Config {
    api_prefix: String,
    viewer_events_path_part: String,
    pub billing: BillingConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            api_prefix: "/api".to_string(),
            viewer_events_path_part: "/events".to_string(),
            billing: BillingConfig::default(),
        }
    }
}

impl Config {
    pub(crate) fn api_prefix(&self) -> String {
        self.api_prefix.clone()
    }
    pub(crate) fn viewer_events_path_part(&self) -> String {
        self.viewer_events_path_part.clone()
    }
    pub(crate) fn auth_service_config(&self) -> AuthServiceConfig {
        AuthServiceConfig::new(format!(
            "{}{}",
            self.api_prefix, self.viewer_events_path_part
        ))
    }
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub config: Config,
    pub auth:
        AuthService<InMemSink<ViewerRegistered>, InMemSink<ViewerEventsKeyRegistered>>,
    pub internal_auth: InternalAuthService,
    pub web_events: WebViewerEventsService<InMemSink<UsageRecorded>>,
    pub billing: billing::service::PublicService,
}

/// State of the admin gateway, which listens on its own port. Separate from
/// [`AppState`] so no viewer handler can reach an admin service.
#[derive(Clone)]
pub(crate) struct AdminState {
    pub billing: billing::admin_service::AdminService,
}

/// State of the creator service, which listens on its own port and shares
/// nothing with the viewer pipeline: it stands in for a separately deployed
/// service.
#[derive(Clone, Default)]
pub(crate) struct CreatorState {
    pub creator: creator::public_service::CreatorService,
}

fn generate_events_token_keys() -> (TokenSigningKey, TokenVerificationKey) {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = VerifyingKey::from(&signing_key);
    #[expect(
        clippy::expect_used,
        reason = "PKCS#8 encoding of a freshly generated Ed25519 signing key cannot fail"
    )]
    let der = signing_key
        .to_pkcs8_der()
        .expect("PKCS#8 encoding of a generated Ed25519 key");
    (
        TokenSigningKey::new(der.to_bytes().to_vec()),
        TokenVerificationKey::new(verifying_key.to_bytes().to_vec()),
    )
}

/// The viewer gateway's state only, for tests that do not need the admin
/// gateway.
#[cfg(test)]
pub(super) fn create_state(config: Config) -> (AppState, impl Future<Output = ()>) {
    let (state, _admin, listeners) = create_states(
        config,
        Arc::new(billing::payments::StubPaymentProvider::default()),
    );
    (state, listeners)
}

pub(super) fn create_states(
    config: Config,
    payments: Arc<dyn PaymentProvider>,
) -> (AppState, AdminState, impl Future<Output = ()>) {
    let (events_token_signing_key, events_token_verifying_key) =
        generate_events_token_keys();

    // Internal-only service: filled solely by the registration listener and
    // queried solely through the typed exchangers handed to services below.
    // Deliberately not part of AppState, so no HTTP handler can reach it.
    let id_storage = id_storage::create_wiring();
    let all_listeners: FuturesUnordered<Pin<Box<dyn Future<Output = ()>>>> =
        FuturesUnordered::new();

    let (registered_tx, registered_rx) = create_memory_queue::<ViewerRegistered>();
    all_listeners.push(Box::pin(
        id_storage.registration_listener(registered_rx),
    ));

    let viewer_public_keys = ViewerPublicKeys::new();
    let (key_registered_tx, key_registered_rx) =
        create_memory_queue::<ViewerEventsKeyRegistered>();
    all_listeners.push(Box::pin(
        viewer_public_keys.start_listener(key_registered_rx),
    ));

    let billing = billing::create_wiring(config.billing.clone());
    let (usage_tx, usage_rx) = create_memory_queue::<UsageRecorded>();
    all_listeners.push(Box::pin(
        billing.usage_listener(id_storage.exchanger(), usage_rx),
    ));
    let period_closer = billing.period_closer();
    all_listeners.push(Box::pin(async move {
        let mut ticks = tokio::time::interval(Duration::from_secs(1));
        loop {
            ticks.tick().await;
            period_closer.tick().await;
        }
    }));

    (
        AppState {
            config: config.clone(),
            auth: AuthService::new(
                config.auth_service_config(),
                id_storage.exchanger(),
                registered_tx,
                key_registered_tx,
                events_token_signing_key,
            ),
            internal_auth: InternalAuthService::new(events_token_verifying_key),
            web_events: WebViewerEventsService::new(usage_tx, viewer_public_keys),
            billing: billing.public_service(),
        },
        AdminState {
            billing: billing.admin_service(payments),
        },
        all_listeners.for_each(|_| async {}),
    )
}
