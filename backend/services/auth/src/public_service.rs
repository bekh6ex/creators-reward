#![expect(
    clippy::disallowed_types,
    reason = "auth mints ViewerIdSet once at registration and immediately hands it to id_storage"
)]

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use futures::{Sink, SinkExt};
use rand::rngs::OsRng;
use secrecy::SecretString;

use common::ViewerEventsId;
use common::ids::ViewerIdAuthentication;
use common::viewer_registration::{ViewerEventsKeyRegistered, ViewerPublicKey};
use id_storage::internal_api::{IdExchange, IdExchanger, ViewerIdSet, ViewerRegistered};

use crate::tokens::TokenSigningKey;
use crate::tokens::viewer_events::RawViewerEventsToken;

#[derive(Debug, thiserror::Error)]
pub enum RegisterError {
    #[error("Email already taken")]
    EmailTaken,
    #[error("Malformed request")]
    Malformed,
    #[error("Registration could not be propagated")]
    Unavailable,
}

#[derive(Debug, thiserror::Error)]
pub enum LoginError {
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Malformed request")]
    Malformed,
    #[error("Viewer ids not available yet")]
    Unavailable,
}

/// Both values are bearer credentials: whoever holds one is the viewer. They
/// leave the service only as cookies; `Debug` prints them redacted.
#[derive(Debug)]
pub struct AuthSessionResult {
    pub events_token: SecretString,
    pub session_id: SecretString,
}

#[derive(Debug)]
pub struct LoginResult {
    pub session: AuthSessionResult,
    pub encrypted_private_key: String,
}

/// What the browser generated at registration. The server cannot use either
/// half to read anything: the private key is sealed under a key derived from
/// the OPAQUE export key, which never leaves the browser.
pub struct ViewerKeyMaterial {
    pub public_key: String,
    pub encrypted_private_key: String,
}

// IV (12) + AES-GCM tag (16) + a PKCS#8 body; anything far beyond an
// X25519 PKCS#8 is not something our client produces.
const ENCRYPTED_PRIVATE_KEY_MIN_LEN: usize = 12 + 16 + 1;
const ENCRYPTED_PRIVATE_KEY_MAX_LEN: usize = 1024;

#[derive(Clone)]
pub struct AuthServiceConfig {
    pub events_token_cookie_name: String,
    pub events_path: String,
}

impl AuthServiceConfig {
    pub fn new(events_path: String) -> Self {
        AuthServiceConfig {
            events_token_cookie_name: "events_token".to_string(),
            events_path,
        }
    }
}

#[derive(Clone)]
pub struct AuthService<RS, KS> {
    events_ids: IdExchanger<ViewerIdAuthentication, ViewerEventsId>,
    registered_sink: RS,
    key_registered_sink: KS,
    pub config: AuthServiceConfig,
    events_token_signing_key: TokenSigningKey,
    opaque_server_setup: crate::opaque::ServerSetup,
    credential_store: crate::opaque::CredentialStore,
    pending_login_store: crate::opaque::PendingLoginStore,
    session_store: crate::opaque::SessionStore,
}

impl<RS, KS> AuthService<RS, KS>
where
    RS: Sink<ViewerRegistered> + Clone + Unpin,
    RS::Error: std::fmt::Debug,
    KS: Sink<ViewerEventsKeyRegistered> + Clone + Unpin,
    KS::Error: std::fmt::Debug,
{
    pub fn new(
        config: AuthServiceConfig,
        events_ids: IdExchanger<ViewerIdAuthentication, ViewerEventsId>,
        registered_sink: RS,
        key_registered_sink: KS,
        events_token_signing_key: TokenSigningKey,
    ) -> Self {
        AuthService {
            events_ids,
            registered_sink,
            key_registered_sink,
            config,
            events_token_signing_key,
            opaque_server_setup: crate::opaque::new_server_setup(),
            credential_store: crate::opaque::CredentialStore::new(),
            pending_login_store: crate::opaque::PendingLoginStore::new(),
            session_store: crate::opaque::SessionStore::new(),
        }
    }

    pub async fn register_start(
        &self,
        email: String,
        registration_request_b64: String,
    ) -> Result<String, RegisterError> {
        if email.is_empty() {
            return Err(RegisterError::Malformed);
        }
        if self.credential_store.contains(&email) {
            return Err(RegisterError::EmailTaken);
        }

        let bytes = URL_SAFE_NO_PAD
            .decode(registration_request_b64)
            .map_err(|_| RegisterError::Malformed)?;

        let registration_request = crate::opaque::RegistrationRequest::<
            crate::opaque::CrOpaque,
        >::deserialize(&bytes)
        .map_err(|_| RegisterError::Malformed)?;

        let server_registration =
            crate::opaque::ServerRegistration::<crate::opaque::CrOpaque>::start(
                &self.opaque_server_setup,
                registration_request,
                email.as_bytes(),
            )
            .map_err(|_| RegisterError::Malformed)?;

        let response_bytes = server_registration.message.serialize();
        Ok(URL_SAFE_NO_PAD.encode(response_bytes))
    }

    pub async fn register_finish(
        &self,
        email: String,
        registration_record_b64: String,
        keys: ViewerKeyMaterial,
    ) -> Result<AuthSessionResult, RegisterError> {
        if email.is_empty() {
            return Err(RegisterError::Malformed);
        }
        if self.credential_store.contains(&email) {
            return Err(RegisterError::EmailTaken);
        }

        let bytes = URL_SAFE_NO_PAD
            .decode(registration_record_b64)
            .map_err(|_| RegisterError::Malformed)?;

        let registration_upload = crate::opaque::RegistrationUpload::<
            crate::opaque::CrOpaque,
        >::deserialize(&bytes)
        .map_err(|_| RegisterError::Malformed)?;

        let public_key = URL_SAFE_NO_PAD
            .decode(keys.public_key)
            .ok()
            .and_then(|bytes| ViewerPublicKey::try_from(bytes.as_slice()).ok())
            .ok_or(RegisterError::Malformed)?;

        let encrypted_private_key = URL_SAFE_NO_PAD
            .decode(keys.encrypted_private_key)
            .map_err(|_| RegisterError::Malformed)?;
        if !(ENCRYPTED_PRIVATE_KEY_MIN_LEN..=ENCRYPTED_PRIVATE_KEY_MAX_LEN)
            .contains(&encrypted_private_key.len())
        {
            return Err(RegisterError::Malformed);
        }

        let password_file =
            crate::opaque::ServerRegistration::<crate::opaque::CrOpaque>::finish(
                registration_upload,
            );

        // All of the viewer's ids are minted here and handed to the id exchange
        // in one message; this service keeps only its own id and the events id
        // it needs for the events token.
        let ids = ViewerIdSet::new(ViewerIdAuthentication::new());
        let viewer_id = ids.auth();
        let events_id = ids.events();

        // Propagate before storing anything: a viewer whose ids never reached
        // the id exchange could log in but never get an events token.
        self.registered_sink
            .clone()
            .send(ViewerRegistered { ids })
            .await
            .map_err(|err| {
                tracing::error!("ViewerRegistered not propagated: {err:?}");
                RegisterError::Unavailable
            })?;
        self.key_registered_sink
            .clone()
            .send(ViewerEventsKeyRegistered {
                events_id,
                public_key,
            })
            .await
            .map_err(|err| {
                tracing::error!("ViewerEventsKeyRegistered not propagated: {err:?}");
                RegisterError::Unavailable
            })?;

        let record = crate::opaque::OpaqueViewerRecord {
            viewer_id,
            registration_record: password_file.serialize().to_vec(),
            encrypted_private_key,
        };
        self.credential_store
            .insert(email.clone(), record);

        Ok(AuthSessionResult {
            events_token: self.events_token(events_id).into(),
            session_id: self.create_session(viewer_id, email).into(),
        })
    }

    pub async fn login_start(
        &self,
        email: String,
        start_login_request_b64: String,
    ) -> Result<(String, String), LoginError> {
        if email.is_empty() {
            return Err(LoginError::Malformed);
        }

        let bytes = URL_SAFE_NO_PAD
            .decode(start_login_request_b64)
            .map_err(|_| LoginError::Malformed)?;

        let credential_request = crate::opaque::CredentialRequest::<
            crate::opaque::CrOpaque,
        >::deserialize(&bytes)
        .map_err(|_| LoginError::Malformed)?;

        let password_file = if let Some(record) = self.credential_store.get(&email) {
            Some(
                crate::opaque::ServerRegistration::<crate::opaque::CrOpaque>::deserialize(
                    &record.registration_record,
                )
                .map_err(|_| LoginError::Malformed)?,
            )
        } else {
            None
        };

        let mut server_rng = OsRng;
        let server_login = crate::opaque::ServerLogin::<crate::opaque::CrOpaque>::start(
            &mut server_rng,
            &self.opaque_server_setup,
            password_file,
            credential_request,
            email.as_bytes(),
            crate::opaque::ServerLoginParameters::default(),
        )
        .map_err(|_| LoginError::Malformed)?;

        let flow_id = crate::opaque::PendingLoginStore::generate_flow_id();
        let serialized_state = server_login.state.serialize().to_vec();
        self.pending_login_store
            .insert(flow_id.clone(), serialized_state, email);

        let response_bytes = server_login.message.serialize();
        let login_response = URL_SAFE_NO_PAD.encode(response_bytes);

        Ok((flow_id, login_response))
    }

    pub async fn login_finish(
        &self,
        login_flow_id: String,
        finish_login_request_b64: String,
    ) -> Result<LoginResult, LoginError> {
        let bytes = URL_SAFE_NO_PAD
            .decode(finish_login_request_b64)
            .map_err(|_| LoginError::Malformed)?;

        let credential_finalization = crate::opaque::CredentialFinalization::<
            crate::opaque::CrOpaque,
        >::deserialize(&bytes)
        .map_err(|_| LoginError::Malformed)?;

        let (state_bytes, email) = match self.pending_login_store.take(&login_flow_id) {
            Some((state, email)) => (state, email),
            None => return Err(LoginError::InvalidCredentials),
        };

        let server_login =
            match crate::opaque::ServerLogin::<crate::opaque::CrOpaque>::deserialize(
                &state_bytes,
            ) {
                Ok(s) => s,
                Err(_) => return Err(LoginError::InvalidCredentials),
            };

        let _result = match server_login.finish(
            credential_finalization,
            crate::opaque::ServerLoginParameters::default(),
        ) {
            Ok(r) => r,
            Err(_) => return Err(LoginError::InvalidCredentials),
        };

        let record = match self.credential_store.get(&email) {
            Some(r) => r,
            None => return Err(LoginError::InvalidCredentials),
        };

        let viewer_id = record.viewer_id;

        let events_id = self
            .events_ids
            .get_secondary(viewer_id)
            .await
            .ok_or_else(|| {
                tracing::error!("events id not (yet) known for a registered viewer");
                LoginError::Unavailable
            })?;

        Ok(LoginResult {
            session: AuthSessionResult {
                events_token: self.events_token(events_id).into(),
                session_id: self.create_session(viewer_id, email).into(),
            },
            encrypted_private_key: URL_SAFE_NO_PAD.encode(record.encrypted_private_key),
        })
    }

    pub fn logout(&self, session_id: String) {
        self.session_store.remove(&session_id);
    }

    pub fn session(&self, session_id: String) -> Option<String> {
        self.session_store
            .get(&session_id)
            .map(|e| e.email)
    }

    fn events_token(&self, events_id: ViewerEventsId) -> String {
        RawViewerEventsToken::generate(events_id, &self.events_token_signing_key)
            .to_string()
    }

    fn create_session(&self, viewer_id: ViewerIdAuthentication, email: String) -> String {
        let session_id = crate::opaque::SessionStore::generate_session_id();
        self.session_store
            .insert(session_id.clone(), viewer_id, email);
        session_id
    }
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::SigningKey;
    use ed25519_dalek::pkcs8::EncodePrivateKey;
    use futures::channel::mpsc;
    use googletest::prelude::*;
    use opaque_ke::{
        ClientLogin, ClientRegistration, ClientRegistrationFinishParameters,
        CredentialResponse,
    };
    use rand::rngs::OsRng;
    use rstest::{fixture, rstest};
    use secrecy::ExposeSecret;

    use super::*;

    type TestAuthService = AuthService<
        mpsc::Sender<ViewerRegistered>,
        mpsc::Sender<ViewerEventsKeyRegistered>,
    >;

    /// The service and the receiving ends of its two registration sinks.
    type Wired = (
        TestAuthService,
        mpsc::Receiver<ViewerRegistered>,
        mpsc::Receiver<ViewerEventsKeyRegistered>,
    );

    pub(crate) struct KeyPair {
        pub signing: TokenSigningKey,
    }

    #[fixture]
    fn key_pair() -> KeyPair {
        let signing_key = SigningKey::generate(&mut OsRng);
        KeyPair {
            signing: TokenSigningKey::new(
                signing_key
                    .to_pkcs8_der()
                    .unwrap()
                    .to_bytes()
                    .to_vec(),
            ),
        }
    }

    #[fixture]
    fn service(#[from(key_pair)] keys: KeyPair) -> Wired {
        let (registered_tx, registered_rx) = mpsc::channel(10);
        let (keys_tx, keys_rx) = mpsc::channel(10);
        let service = AuthService::new(
            AuthServiceConfig::new("/events".to_string()),
            id_storage::create_wiring().exchanger(),
            registered_tx,
            keys_tx,
            keys.signing,
        );
        (service, registered_rx, keys_rx)
    }

    async fn registration_record(service: &TestAuthService, email: &str) -> String {
        let password = b"password";
        let mut rng = OsRng;
        let client_start =
            ClientRegistration::<crate::opaque::CrOpaque>::start(&mut rng, password)
                .unwrap();
        let registration_response = service
            .register_start(
                email.to_string(),
                URL_SAFE_NO_PAD.encode(client_start.message.serialize()),
            )
            .await
            .unwrap();
        let response =
            opaque_ke::RegistrationResponse::<crate::opaque::CrOpaque>::deserialize(
                &URL_SAFE_NO_PAD
                    .decode(registration_response)
                    .unwrap(),
            )
            .unwrap();
        let client_finish = client_start
            .state
            .finish(
                &mut rng,
                password,
                response,
                ClientRegistrationFinishParameters::default(),
            )
            .unwrap();
        URL_SAFE_NO_PAD.encode(client_finish.message.serialize())
    }

    fn key_material(public_key: &[u8]) -> ViewerKeyMaterial {
        ViewerKeyMaterial {
            public_key: URL_SAFE_NO_PAD.encode(public_key),
            encrypted_private_key: URL_SAFE_NO_PAD.encode([9u8; 76]),
        }
    }

    async fn login_start_record(
        service: &TestAuthService,
        email: &str,
    ) -> (String, String) {
        let password = b"password";
        let mut rng = OsRng;
        let client_start =
            ClientLogin::<crate::opaque::CrOpaque>::start(&mut rng, password).unwrap();
        service
            .login_start(
                email.to_string(),
                URL_SAFE_NO_PAD.encode(client_start.message.serialize()),
            )
            .await
            .unwrap()
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    async fn register_finish_stores_encrypted_key_and_emits_split_events(
        #[from(service)] (service, mut registered_rx, mut keys_rx): Wired,
    ) {
        let email = "alice@example.com";
        let record = registration_record(&service, email).await;

        service
            .register_finish(email.to_string(), record, key_material(&[5u8; 32]))
            .await
            .unwrap();

        let stored = service.credential_store.get(email).unwrap();
        expect_that!(stored.encrypted_private_key, eq(&vec![9u8; 76]));

        let registered = registered_rx.try_recv().unwrap();
        expect_that!(registered.ids.auth(), eq(stored.viewer_id));

        let key_event = keys_rx.try_recv().unwrap();
        expect_that!(key_event.events_id, eq(registered.ids.events()));
        expect_that!(key_event.public_key.as_bytes(), eq(&[5u8; 32]));
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    #[expect(
        clippy::disallowed_methods,
        reason = "tests: verifying that Debug redacts secrets requires exposing them"
    )]
    async fn session_credentials_are_redacted_in_debug(
        #[from(service)] (service, _registered_rx, _keys_rx): Wired,
    ) {
        let email = "dave@example.com";
        let record = registration_record(&service, email).await;

        let session = service
            .register_finish(email.to_string(), record, key_material(&[5u8; 32]))
            .await
            .unwrap();

        expect_that!(
            format!("{session:?}"),
            all![
                not(contains_substring(
                    session.events_token.expose_secret()
                )),
                not(contains_substring(session.session_id.expose_secret())),
            ]
        );
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    async fn register_finish_rejects_wrong_size_public_key_without_side_effects(
        #[from(service)] (service, mut registered_rx, mut keys_rx): Wired,
    ) {
        let email = "bob@example.com";
        let record = registration_record(&service, email).await;

        let result = service
            .register_finish(email.to_string(), record, key_material(&[5u8; 31]))
            .await;

        expect_that!(result, err(pat!(RegisterError::Malformed)));
        expect_that!(service.credential_store.contains(email), is_false());
        expect_that!(registered_rx.try_recv(), err(anything()));
        expect_that!(keys_rx.try_recv(), err(anything()));
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    async fn register_finish_rejects_oversized_encrypted_private_key(
        #[from(service)] (service, _registered_rx, _keys_rx): Wired,
    ) {
        let email = "carol@example.com";
        let record = registration_record(&service, email).await;

        let result = service
            .register_finish(
                email.to_string(),
                record,
                ViewerKeyMaterial {
                    public_key: URL_SAFE_NO_PAD.encode([5u8; 32]),
                    encrypted_private_key: URL_SAFE_NO_PAD.encode(vec![
                        0u8;
                        ENCRYPTED_PRIVATE_KEY_MAX_LEN
                            + 1
                    ]),
                },
            )
            .await;

        expect_that!(result, err(pat!(RegisterError::Malformed)));
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    async fn register_finish_accepts_exact_minimum_encrypted_private_key(
        #[from(service)] (service, _registered_rx, _keys_rx): Wired,
    ) {
        let email = "dave@example.com";
        let record = registration_record(&service, email).await;
        // 12 (IV) + 16 (tag) + 1 (minimum PKCS#8 body).
        let min_len = 29usize;

        let result = service
            .register_finish(
                email.to_string(),
                record,
                ViewerKeyMaterial {
                    public_key: URL_SAFE_NO_PAD.encode([5u8; 32]),
                    encrypted_private_key: URL_SAFE_NO_PAD.encode(vec![0u8; min_len]),
                },
            )
            .await;

        expect_that!(result, ok(anything()));
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    async fn register_finish_rejects_encrypted_private_key_one_below_minimum(
        #[from(service)] (service, _registered_rx, _keys_rx): Wired,
    ) {
        let email = "ellen@example.com";
        let record = registration_record(&service, email).await;
        let min_len = 29usize;

        let result = service
            .register_finish(
                email.to_string(),
                record,
                ViewerKeyMaterial {
                    public_key: URL_SAFE_NO_PAD.encode([5u8; 32]),
                    encrypted_private_key: URL_SAFE_NO_PAD.encode(vec![0u8; min_len - 1]),
                },
            )
            .await;

        expect_that!(result, err(pat!(RegisterError::Malformed)));
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    #[expect(
        clippy::disallowed_methods,
        reason = "tests: verifying session lookup requires exposing the session id"
    )]
    async fn session_returns_the_registered_email(
        #[from(service)] (service, _registered_rx, _keys_rx): Wired,
    ) {
        let email = "frank@example.com";
        let record = registration_record(&service, email).await;
        let session = service
            .register_finish(email.to_string(), record, key_material(&[5u8; 32]))
            .await
            .unwrap();

        let looked_up = service.session(session.session_id.expose_secret().to_string());
        expect_that!(looked_up, some(eq(email)));

        let missing = service.session("not-a-session-id".to_string());
        expect_that!(missing, none());
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    #[expect(
        clippy::disallowed_methods,
        reason = "tests: verifying logout requires exposing the session id"
    )]
    async fn logout_removes_the_session(
        #[from(service)] (service, _registered_rx, _keys_rx): Wired,
    ) {
        let email = "grace@example.com";
        let record = registration_record(&service, email).await;
        let session = service
            .register_finish(email.to_string(), record, key_material(&[5u8; 32]))
            .await
            .unwrap();

        expect_that!(
            service.session(session.session_id.expose_secret().to_string()),
            some(eq(email))
        );
        service.logout(session.session_id.expose_secret().to_string());
        expect_that!(
            service.session(session.session_id.expose_secret().to_string()),
            none()
        );
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    #[expect(
        clippy::disallowed_methods,
        reason = "tests: comparing session credentials requires exposing them"
    )]
    async fn session_credentials_are_unique_per_registration(
        #[from(service)] (service, _registered_rx, _keys_rx): Wired,
    ) {
        let email1 = "henry@example.com";
        let email2 = "ida@example.com";
        let record1 = registration_record(&service, email1).await;
        let record2 = registration_record(&service, email2).await;

        let session1 = service
            .register_finish(email1.to_string(), record1, key_material(&[5u8; 32]))
            .await
            .unwrap();
        let session2 = service
            .register_finish(email2.to_string(), record2, key_material(&[5u8; 32]))
            .await
            .unwrap();

        expect_that!(
            session1.session_id.expose_secret(),
            ne(session2.session_id.expose_secret())
        );
        expect_that!(
            session1.events_token.expose_secret(),
            ne(session2.events_token.expose_secret())
        );
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    async fn login_start_returns_unique_deserializable_responses(
        #[from(service)] (service, _registered_rx, _keys_rx): Wired,
    ) {
        let email = "jack@example.com";
        let record = registration_record(&service, email).await;
        service
            .register_finish(email.to_string(), record, key_material(&[5u8; 32]))
            .await
            .unwrap();

        let (flow_id1, response1) = login_start_record(&service, email).await;
        let (flow_id2, response2) = login_start_record(&service, email).await;

        expect_that!(flow_id1, ne(&flow_id2));
        expect_that!(response1, ne(&response2));

        let bytes1 = URL_SAFE_NO_PAD.decode(response1).unwrap();
        let bytes2 = URL_SAFE_NO_PAD.decode(response2).unwrap();
        expect_that!(
            CredentialResponse::<crate::opaque::CrOpaque>::deserialize(&bytes1),
            ok(anything())
        );
        expect_that!(
            CredentialResponse::<crate::opaque::CrOpaque>::deserialize(&bytes2),
            ok(anything())
        );
    }
}
