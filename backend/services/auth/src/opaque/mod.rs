use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Instant;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use common::ids::ViewerIdAuthentication;
use opaque_ke::CipherSuite;
use rand::RngCore;
use rand::rngs::OsRng;

/// Where the login and session stores read the time, in milliseconds:
/// `monotonic_millis` in production, a clock the test advances in tests.
pub type Clock = Arc<dyn Fn() -> u64 + Send + Sync>;

/// Milliseconds since the first call, on the monotonic clock. The stores only
/// need the age of an entry for its TTL, so no wall-clock time is stored.
#[expect(
    clippy::disallowed_methods,
    reason = "measures how old a login flow or session is, for its TTL; only an offset from process start is stored next to the email, never a time of day"
)]
pub fn monotonic_millis() -> u64 {
    static START: LazyLock<Instant> = LazyLock::new(Instant::now);
    u64::try_from(START.elapsed().as_millis()).unwrap_or(u64::MAX)
}

pub struct CrOpaque;

impl CipherSuite for CrOpaque {
    type OprfCs = opaque_ke::Ristretto255;
    type KeyExchange = opaque_ke::TripleDh<opaque_ke::Ristretto255, sha2::Sha512>;
    type Ksf = opaque_ke::ksf::Identity;
}

pub type ServerSetup = opaque_ke::ServerSetup<CrOpaque>;

pub fn new_server_setup() -> ServerSetup {
    let mut rng = rand::thread_rng();
    ServerSetup::new(&mut rng)
}

pub use opaque_ke::CredentialFinalization;
pub use opaque_ke::CredentialRequest;
pub use opaque_ke::RegistrationRequest;
pub use opaque_ke::RegistrationUpload;
pub use opaque_ke::ServerLogin;
pub use opaque_ke::ServerLoginParameters;
pub use opaque_ke::ServerRegistration;

// ------------------------------------------------------------------
// In-memory stores for OPAQUE authentication
// ------------------------------------------------------------------

#[derive(Clone)]
pub struct OpaqueViewerRecord {
    pub viewer_id: ViewerIdAuthentication,
    pub registration_record: Vec<u8>,
    pub encrypted_private_key: Vec<u8>,
}

#[derive(Clone)]
pub struct CredentialStore {
    inner: Arc<Mutex<HashMap<String, OpaqueViewerRecord>>>,
}

impl CredentialStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn insert(&self, email: String, record: OpaqueViewerRecord) {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        guard.insert(email, record);
    }

    pub fn get(&self, email: &str) -> Option<OpaqueViewerRecord> {
        let guard = self
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        guard.get(email).cloned()
    }

    pub fn contains(&self, email: &str) -> bool {
        let guard = self
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        guard.contains_key(email)
    }
}

impl Default for CredentialStore {
    fn default() -> Self {
        Self::new()
    }
}

struct PendingLoginEntry {
    state: Vec<u8>,
    email: String,
    created_at_millis: u64,
}

#[derive(Clone)]
pub struct PendingLoginStore {
    inner: Arc<Mutex<HashMap<String, PendingLoginEntry>>>,
    ttl_millis: u64,
    clock: Clock,
}

impl PendingLoginStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            ttl_millis: 60_000,
            clock: Arc::new(monotonic_millis),
        }
    }

    #[cfg(test)]
    pub fn with_clock(ttl_millis: u64, clock: Clock) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            ttl_millis,
            clock,
        }
    }

    /// Abandoned flows are never deleted yet.
    pub fn insert(&self, flow_id: String, state: Vec<u8>, email: String) {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        guard.insert(
            flow_id,
            PendingLoginEntry {
                state,
                email,
                created_at_millis: (self.clock)(),
            },
        );
    }

    pub fn take(&self, flow_id: &str) -> Option<(Vec<u8>, String)> {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let entry = guard.remove(flow_id)?;
        drop(guard);
        if (self.clock)().saturating_sub(entry.created_at_millis) > self.ttl_millis {
            return None;
        }
        Some((entry.state, entry.email))
    }

    pub fn generate_flow_id() -> String {
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        URL_SAFE_NO_PAD.encode(bytes)
    }
}

impl Default for PendingLoginStore {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SessionEntry {
    pub viewer_id: ViewerIdAuthentication,
    pub email: String,
    /// When the session was created, on the store's clock.
    pub created_at: u64,
}

#[derive(Clone)]
pub struct SessionStore {
    inner: Arc<Mutex<HashMap<String, SessionEntry>>>,
    ttl_millis: u64,
    clock: Clock,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            ttl_millis: 24 * 60 * 60 * 1000,
            clock: Arc::new(monotonic_millis),
        }
    }

    #[cfg(test)]
    pub fn with_clock(ttl_millis: u64, clock: Clock) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            ttl_millis,
            clock,
        }
    }

    /// Sessions never read again are never deleted yet.
    pub fn insert(
        &self,
        session_id: String,
        viewer_id: ViewerIdAuthentication,
        email: String,
    ) {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        guard.insert(
            session_id,
            SessionEntry {
                viewer_id,
                email,
                created_at: (self.clock)(),
            },
        );
    }

    pub fn get(&self, session_id: &str) -> Option<SessionEntry> {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let entry = guard.get(session_id)?.clone();
        if (self.clock)().saturating_sub(entry.created_at) > self.ttl_millis {
            guard.remove(session_id);
            drop(guard);
            return None;
        }
        drop(guard);
        Some(entry)
    }

    pub fn remove(&self, session_id: &str) -> Option<SessionEntry> {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        guard.remove(session_id)
    }

    pub fn generate_session_id() -> String {
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        URL_SAFE_NO_PAD.encode(bytes)
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use googletest::prelude::*;
    use opaque_ke::{
        ClientLogin, ClientLoginFinishParameters, ClientRegistration,
        ClientRegistrationFinishParameters, ServerLogin, ServerLoginParameters,
        ServerRegistration, ServerSetup,
    };
    use rand::rngs::OsRng;
    use rstest::rstest;

    use super::CrOpaque;

    #[gtest]
    #[rstest]
    #[timeout(std::time::Duration::from_secs(5))]
    fn monotonic_millis_advances_with_time() -> googletest::Result<()> {
        let before = super::monotonic_millis();
        std::thread::sleep(std::time::Duration::from_millis(5));
        verify_that!(
            super::monotonic_millis(),
            ge(before.checked_add(5).unwrap())
        )
    }

    #[gtest]
    #[rstest]
    fn opaque_register_login_round_trip() {
        let mut rng = OsRng;
        let server_setup = ServerSetup::<CrOpaque>::new(&mut rng);

        let mut client_rng = OsRng;
        let client_registration_start =
            ClientRegistration::<CrOpaque>::start(&mut client_rng, b"password").unwrap();

        let server_registration_start = ServerRegistration::<CrOpaque>::start(
            &server_setup,
            client_registration_start.message,
            b"alice@example.com",
        )
        .unwrap();

        let client_registration_finish = client_registration_start
            .state
            .finish(
                &mut client_rng,
                b"password",
                server_registration_start.message,
                ClientRegistrationFinishParameters::default(),
            )
            .unwrap();

        let password_file =
            ServerRegistration::<CrOpaque>::finish(client_registration_finish.message);

        let client_login_start =
            ClientLogin::<CrOpaque>::start(&mut client_rng, b"password").unwrap();

        let mut server_rng = OsRng;
        let server_login_start = ServerLogin::<CrOpaque>::start(
            &mut server_rng,
            &server_setup,
            Some(password_file),
            client_login_start.message,
            b"alice@example.com",
            ServerLoginParameters::default(),
        )
        .unwrap();

        let client_login_finish = client_login_start
            .state
            .finish(
                &mut client_rng,
                b"password",
                server_login_start.message,
                ClientLoginFinishParameters::default(),
            )
            .unwrap();

        let server_login_finish = server_login_start
            .state
            .finish(
                client_login_finish.message,
                ServerLoginParameters::default(),
            )
            .unwrap();

        expect_that!(
            client_login_finish.session_key,
            eq(server_login_finish.session_key)
        );
        expect_that!(
            client_registration_finish.export_key,
            eq(client_login_finish.export_key)
        );
    }

    mod stores {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::thread;

        use common::ids::ViewerIdAuthentication;
        use googletest::prelude::*;
        use rstest::rstest;

        use super::super::{
            Clock, CredentialStore, OpaqueViewerRecord, PendingLoginStore, SessionStore,
        };

        #[gtest]
        #[rstest]
        fn pending_login_single_use() {
            let store = PendingLoginStore::new();
            let flow_id = PendingLoginStore::generate_flow_id();
            store.insert(
                flow_id.clone(),
                vec![1, 2, 3],
                "a@example.com".to_string(),
            );

            let first = store.take(&flow_id);
            expect_that!(first, some((eq(&vec![1, 2, 3]), eq("a@example.com"))));

            let second = store.take(&flow_id);
            expect_that!(second, none());
        }

        #[gtest]
        #[rstest]
        fn pending_login_replay_rejected() {
            let store = PendingLoginStore::new();
            let flow_id = PendingLoginStore::generate_flow_id();
            store.insert(
                flow_id.clone(),
                vec![4, 5, 6],
                "b@example.com".to_string(),
            );

            let _ = store.take(&flow_id);
            let replay = store.take(&flow_id);
            expect_that!(replay, none());
        }

        /// A clock the test sets by hand, starting at 0 ms.
        fn manual_clock() -> (Arc<AtomicU64>, Clock) {
            let now = Arc::new(AtomicU64::new(0));
            let reader = now.clone();
            (now, Arc::new(move || reader.load(Ordering::SeqCst)))
        }

        #[gtest]
        #[rstest]
        #[case::at_the_ttl(100, true)]
        #[case::past_the_ttl(101, false)]
        fn pending_login_expires_after_its_ttl(#[case] age: u64, #[case] live: bool) {
            let (now, clock) = manual_clock();
            let store = PendingLoginStore::with_clock(100, clock);
            let flow_id = PendingLoginStore::generate_flow_id();
            store.insert(
                flow_id.clone(),
                vec![7, 8, 9],
                "c@example.com".to_string(),
            );

            now.store(age, Ordering::SeqCst);
            expect_that!(store.take(&flow_id).is_some(), eq(live));
        }

        #[gtest]
        #[rstest]
        fn pending_login_concurrent_distinct_flow_ids() {
            let store = std::sync::Arc::new(PendingLoginStore::new());
            let store1 = store.clone();
            let store2 = store.clone();

            let handle1 = thread::spawn(move || {
                let flow_id = PendingLoginStore::generate_flow_id();
                store1.insert(flow_id.clone(), vec![1], "d1@example.com".to_string());
                flow_id
            });

            let handle2 = thread::spawn(move || {
                let flow_id = PendingLoginStore::generate_flow_id();
                store2.insert(flow_id.clone(), vec![2], "d2@example.com".to_string());
                flow_id
            });

            let flow_id1 = handle1.join().unwrap();
            let flow_id2 = handle2.join().unwrap();

            expect_that!(flow_id1, ne(&flow_id2));
            expect_that!(
                store.take(&flow_id1),
                some((eq(&vec![1]), eq("d1@example.com")))
            );
            expect_that!(
                store.take(&flow_id2),
                some((eq(&vec![2]), eq("d2@example.com")))
            );
        }

        #[gtest]
        #[rstest]
        #[case::at_the_ttl(100, true)]
        #[case::past_the_ttl(101, false)]
        fn session_expires_after_its_ttl(#[case] age: u64, #[case] live: bool) {
            let (now, clock) = manual_clock();
            let store = SessionStore::with_clock(100, clock);
            let session_id = SessionStore::generate_session_id();
            store.insert(
                session_id.clone(),
                ViewerIdAuthentication::new(),
                "test@example.com".to_string(),
            );

            now.store(age, Ordering::SeqCst);
            expect_that!(store.get(&session_id).is_some(), eq(live));
        }

        #[gtest]
        #[rstest]
        fn session_get_returns_clone() {
            let store = SessionStore::new();
            let session_id = SessionStore::generate_session_id();
            store.insert(
                session_id.clone(),
                ViewerIdAuthentication::new(),
                "alice@example.com".to_string(),
            );

            let got = store.get(&session_id).unwrap();
            expect_that!(got.email, eq("alice@example.com"));
        }

        #[gtest]
        #[rstest]
        fn session_remove() {
            let store = SessionStore::new();
            let session_id = SessionStore::generate_session_id();
            store.insert(
                session_id.clone(),
                ViewerIdAuthentication::new(),
                "bob@example.com".to_string(),
            );

            expect_that!(store.get(&session_id), some(anything()));
            store.remove(&session_id);
            expect_that!(store.get(&session_id), none());
        }

        #[gtest]
        #[rstest]
        fn generate_flow_id_unique() {
            let id1 = PendingLoginStore::generate_flow_id();
            let id2 = PendingLoginStore::generate_flow_id();
            expect_that!(id1, ne(&id2));
        }

        #[gtest]
        #[rstest]
        fn generate_session_id_unique() {
            let id1 = SessionStore::generate_session_id();
            let id2 = SessionStore::generate_session_id();
            expect_that!(id1, ne(&id2));
        }

        #[gtest]
        #[rstest]
        fn credential_store_roundtrip() {
            let store = CredentialStore::new();
            let record = OpaqueViewerRecord {
                viewer_id: ViewerIdAuthentication::new(),
                registration_record: vec![10, 20, 30],
                encrypted_private_key: vec![40, 50, 60],
            };
            store.insert("alice@example.com".to_string(), record.clone());

            expect_that!(store.contains("alice@example.com"), is_true());
            let got = store.get("alice@example.com").unwrap();
            expect_that!(got.viewer_id, eq(record.viewer_id));
            expect_that!(
                got.registration_record,
                eq(&record.registration_record)
            );
            expect_that!(store.contains("bob@example.com"), is_false());
        }
    }
}
