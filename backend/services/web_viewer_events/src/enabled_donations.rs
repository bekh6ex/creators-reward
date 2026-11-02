//! Which sites a viewer enabled donations for.
//!
//! The record is sealed in the viewer's browser (the events iframe), to the
//! viewer's own public key, with HPKE (RFC 9180). This service never receives
//! the site: it stores opaque fixed-size blobs under random record ids, hands
//! them back to the viewer, and deletes them on request. Only the viewer's
//! browser, which holds the private key, can open them.
//!
//! Format the browser must produce (checked here only by length):
//!
//! ```text
//! suite     = DHKEM(X25519, HKDF-SHA256), HKDF-SHA256, AES-256-GCM, base mode
//! info      = "creators-reward/v1/enabled-donation", aad = ""
//! plaintext = {"site":"<origin>","enabledAtSec":<unix sec, 15-min precision>}
//!             padded with spaces to PADDED_LEN
//! record    = enc(32) ‖ ciphertext(PADDED_LEN + 16)
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use futures::lock::Mutex;

use common::{Uuid, ViewerEventsId};

/// Every plaintext is padded to this length, so the record length does not
/// reveal the length of the site's origin.
pub const PADDED_LEN: usize = 256;
const ENC_LEN: usize = 32;
const TAG_LEN: usize = 16;
/// The only accepted record length; anything else was not padded or not
/// sealed with the expected suite.
pub const SEALED_LEN: usize = ENC_LEN + PADDED_LEN + TAG_LEN;

/// Random, server-assigned; means nothing without the sealed record, which
/// only the viewer can open.
pub type EnabledDonationId = Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedEnabledDonation(Vec<u8>);

impl SealedEnabledDonation {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug, thiserror::Error)]
#[error("a sealed enabled donation must be {SEALED_LEN} bytes, got {0}")]
pub struct InvalidSealedEnabledDonation(usize);

impl TryFrom<Vec<u8>> for SealedEnabledDonation {
    type Error = InvalidSealedEnabledDonation;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        if bytes.len() == SEALED_LEN {
            Ok(SealedEnabledDonation(bytes))
        } else {
            Err(InvalidSealedEnabledDonation(bytes.len()))
        }
    }
}

type Records = HashMap<ViewerEventsId, Vec<(EnabledDonationId, SealedEnabledDonation)>>;

#[derive(Clone)]
pub struct EnabledDonations {
    records: Arc<Mutex<Records>>,
}

impl Default for EnabledDonations {
    fn default() -> Self {
        Self::new()
    }
}

impl EnabledDonations {
    pub fn new() -> Self {
        EnabledDonations {
            records: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn add(
        &self,
        viewer: ViewerEventsId,
        sealed: SealedEnabledDonation,
    ) -> EnabledDonationId {
        let id = Uuid::new_v4();
        self.records
            .lock()
            .await
            .entry(viewer)
            .or_default()
            .push((id, sealed));
        id
    }

    /// Everything stored for `viewer`, oldest first.
    pub async fn list(
        &self,
        viewer: ViewerEventsId,
    ) -> Vec<(EnabledDonationId, SealedEnabledDonation)> {
        self.records
            .lock()
            .await
            .get(&viewer)
            .cloned()
            .unwrap_or_default()
    }

    pub async fn contains(&self, viewer: ViewerEventsId, id: EnabledDonationId) -> bool {
        self.records
            .lock()
            .await
            .get(&viewer)
            .is_some_and(|records| records.iter().any(|(record, _)| *record == id))
    }

    /// Idempotent; removing an id that does not exist, or is another
    /// viewer's, changes nothing.
    pub async fn remove(&self, viewer: ViewerEventsId, id: EnabledDonationId) {
        if let Some(records) = self.records.lock().await.get_mut(&viewer) {
            records.retain(|(record, _)| *record != id);
        }
    }
}

#[cfg(test)]
mod tests {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use futures::executor::block_on;
    use googletest::prelude::*;
    use hpke::aead::AesGcm256;
    use hpke::kdf::HkdfSha256;
    use hpke::kem::X25519HkdfSha256;
    use hpke::{Deserializable, Kem as _, OpModeR, OpModeS, Serializable};
    use rstest::rstest;

    use super::*;

    type Kem = X25519HkdfSha256;
    const INFO: &[u8] = b"creators-reward/v1/enabled-donation";
    const PLAINTEXT: &str =
        r#"{"site":"https://news.example","enabledAtSec":1793191500}"#;

    fn record(byte: u8) -> SealedEnabledDonation {
        SealedEnabledDonation::try_from(vec![byte; SEALED_LEN]).unwrap()
    }

    fn seal(public_key: &<Kem as hpke::Kem>::PublicKey) -> (String, Vec<u8>) {
        let plaintext = format!("{PLAINTEXT:<PADDED_LEN$}");
        let (enc, ciphertext) = hpke::single_shot_seal::<AesGcm256, HkdfSha256, Kem>(
            &OpModeS::Base,
            public_key,
            INFO,
            plaintext.as_bytes(),
            b"",
        )
        .unwrap();
        let mut sealed = enc.to_bytes().to_vec();
        sealed.extend_from_slice(&ciphertext);
        (plaintext, sealed)
    }

    #[gtest]
    #[rstest]
    #[case::sealed(SEALED_LEN, true)]
    #[case::short(SEALED_LEN - 1, false)]
    #[case::long(SEALED_LEN + 1, false)]
    fn accepts_only_records_of_the_sealed_length(
        #[case] len: usize,
        #[case] accepted: bool,
    ) {
        expect_that!(
            SealedEnabledDonation::try_from(vec![0; len]).is_ok(),
            eq(accepted)
        );
    }

    #[gtest]
    #[rstest]
    fn stores_lists_and_removes_per_viewer() {
        let donations = EnabledDonations::new();
        let alice = ViewerEventsId::new();
        let bob = ViewerEventsId::new();

        let first = block_on(donations.add(alice, record(1)));
        let second = block_on(donations.add(alice, record(2)));
        let bobs = block_on(donations.add(bob, record(3)));
        expect_that!(first, ne(second));

        expect_that!(
            block_on(donations.list(alice)),
            elements_are![(eq(&first), eq(&record(1))), (eq(&second), eq(&record(2))),]
        );
        expect_that!(block_on(donations.contains(alice, first)), is_true());
        expect_that!(block_on(donations.contains(alice, bobs)), is_false());

        // Removing another viewer's id is a no-op.
        block_on(donations.remove(alice, bobs));
        expect_that!(block_on(donations.contains(bob, bobs)), is_true());

        block_on(donations.remove(alice, first));
        block_on(donations.remove(alice, first));
        expect_that!(
            block_on(donations.list(alice)),
            elements_are![(eq(&second), eq(&record(2)))]
        );
        expect_that!(block_on(donations.contains(alice, first)), is_false());
        expect_that!(
            block_on(donations.list(ViewerEventsId::new())),
            is_empty()
        );
    }

    /// The format documented above is standard HPKE: rust-hpke seals it to
    /// exactly SEALED_LEN bytes and opens it again.
    #[gtest]
    #[rstest]
    fn documented_format_is_standard_hpke() {
        let (private_key, public_key) = Kem::derive_keypair(b"format seed");
        let (plaintext, sealed) = seal(&public_key);
        expect_that!(
            SealedEnabledDonation::try_from(sealed.clone()),
            ok(anything())
        );

        let opened = hpke::single_shot_open::<AesGcm256, HkdfSha256, Kem>(
            &OpModeR::Base,
            &private_key,
            &<Kem as hpke::Kem>::EncappedKey::from_bytes(&sealed[..ENC_LEN]).unwrap(),
            INFO,
            &sealed[ENC_LEN..],
            b"",
        )
        .unwrap();
        expect_that!(opened, eq(plaintext.as_bytes()));
    }

    /// Fixture for `frontend/viewer/src/test/hpkeFixture.ts`: a record sealed
    /// by rust-hpke, so the browser's HPKE `open` is tested against a
    /// reference implementation and not only against our own `seal`.
    #[gtest]
    #[rstest]
    #[ignore]
    fn print_browser_fixture() {
        const PKCS8_X25519_PREFIX: [u8; 16] = [
            0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x6e, 0x04,
            0x22, 0x04, 0x20,
        ];
        let (private_key, public_key) = Kem::derive_keypair(b"browser fixture seed");
        let (_, sealed) = seal(&public_key);
        let mut pkcs8 = PKCS8_X25519_PREFIX.to_vec();
        pkcs8.extend_from_slice(&private_key.to_bytes());

        println!("privateKeyPkcs8: {}", URL_SAFE_NO_PAD.encode(pkcs8));
        println!(
            "publicKey: {}",
            URL_SAFE_NO_PAD.encode(public_key.to_bytes())
        );
        println!("sealed: {}", URL_SAFE_NO_PAD.encode(sealed));
    }
}
