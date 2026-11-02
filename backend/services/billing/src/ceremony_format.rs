//! Everything that crosses between the browser, the server and the offline key
//! ceremony: the sealed record format, the period key, and the batch and
//! totals the admin API hands over. Shared with `tools/billing_ceremony`,
//! which is the only code that opens records.
//!
//! Record format, produced by the events iframe:
//!
//! ```text
//! suite     = DHKEM(X25519, HKDF-SHA256), HKDF-SHA256, AES-256-GCM, base mode
//! info      = "creators-reward/v1/billing-usage/<period id>", aad = ""
//! plaintext = {"site":"<origin>"} padded with spaces to PADDED_LEN
//! record    = enc(32) ‖ ciphertext(PADDED_LEN + 16)
//! ```
//!
//! The period id in `info` binds a record to its period: opened with another
//! period's context it fails, so a record cannot be replayed into a later
//! period. One record is one unit of reading.

use std::collections::BTreeMap;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};

use crate::period::PeriodId;

/// Every plaintext is padded to this length, so a record's length does not
/// reveal the length of the site's origin.
pub const PADDED_LEN: usize = 256;
pub const ENC_LEN: usize = 32;
const TAG_LEN: usize = 16;
/// The only accepted record length.
pub const SEALED_USAGE_LEN: usize = ENC_LEN + PADDED_LEN + TAG_LEN;

pub fn hpke_info(period: PeriodId) -> String {
    format!("creators-reward/v1/billing-usage/{period}")
}

/// What the browser seals.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsagePlaintext {
    /// Origin of the site that was read. Stands for the creator until creators
    /// register their sites.
    pub site: String,
}

/// A record only the period's key holders can open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedUsage(Vec<u8>);

impl SealedUsage {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug, thiserror::Error)]
#[error("a sealed usage record must be {SEALED_USAGE_LEN} bytes, got {0}")]
pub struct InvalidSealedUsage(usize);

impl TryFrom<Vec<u8>> for SealedUsage {
    type Error = InvalidSealedUsage;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        if bytes.len() == SEALED_USAGE_LEN {
            Ok(SealedUsage(bytes))
        } else {
            Err(InvalidSealedUsage(bytes.len()))
        }
    }
}

/// Raw X25519 public key of a period; base64url-encoded on the wire and in
/// the ceremony's `.pub` files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeriodPublicKey([u8; 32]);

#[derive(Debug, thiserror::Error)]
#[error("a period public key must be 32 bytes of base64url")]
pub struct InvalidPeriodPublicKey;

impl PeriodPublicKey {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        PeriodPublicKey(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_base64url(self) -> String {
        URL_SAFE_NO_PAD.encode(self.0)
    }

    pub fn from_base64url(text: &str) -> Result<Self, InvalidPeriodPublicKey> {
        URL_SAFE_NO_PAD
            .decode(text.trim())
            .ok()
            .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
            .map(PeriodPublicKey)
            .ok_or(InvalidPeriodPublicKey)
    }

    /// Hex SHA-256 of the raw key: what the ceremony log publishes.
    pub fn fingerprint(&self) -> String {
        use sha2::{Digest, Sha256};
        Sha256::digest(self.0)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}

/// Exported by billing when a period closes; the ceremony's input. Carries no
/// reader ids, and the records are shuffled.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Batch {
    pub period: PeriodId,
    pub records: Vec<BatchRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchRecord {
    /// This record's share of its reader's charge, in thousandths of a cent.
    pub weight_millicents: u64,
    /// base64url of a [`SealedUsage`].
    pub sealed: String,
}

impl BatchRecord {
    pub fn new(weight_millicents: u64, sealed: &SealedUsage) -> Self {
        BatchRecord {
            weight_millicents,
            sealed: URL_SAFE_NO_PAD.encode(sealed.as_bytes()),
        }
    }

    pub fn sealed_bytes(&self) -> Option<Vec<u8>> {
        URL_SAFE_NO_PAD.decode(&self.sealed).ok()
    }
}

/// The ceremony's only output: sums per site, nothing per record.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub period: PeriodId,
    pub per_site_millicents: BTreeMap<String, u64>,
    /// Records in the batch.
    pub records: u64,
    /// Records that did not open or did not name a valid origin. They are
    /// left out of every total.
    pub dropped: u64,
}

#[cfg(test)]
mod tests {
    use googletest::prelude::*;
    use rstest::rstest;

    use super::*;

    #[gtest]
    #[rstest]
    #[case::exact(304, true)]
    #[case::short(303, false)]
    #[case::long(305, false)]
    fn accepts_only_the_padded_length(#[case] len: usize, #[case] accepted: bool) {
        expect_that!(
            SealedUsage::try_from(vec![0; len]).is_ok(),
            eq(accepted)
        );
    }

    #[gtest]
    #[rstest]
    fn public_key_round_trips_through_its_file_form() {
        let key = PeriodPublicKey::from_bytes([3; 32]);
        let file = format!("{}\n", key.to_base64url());
        expect_that!(PeriodPublicKey::from_base64url(&file), ok(eq(&key)));
        expect_that!(
            PeriodPublicKey::from_base64url("AAAA"),
            err(anything())
        );
        expect_that!(key.fingerprint(), char_count(eq(64)));
    }

    #[gtest]
    #[rstest]
    fn info_binds_the_period() {
        expect_that!(
            hpke_info(PeriodId::new(681)),
            eq("creators-reward/v1/billing-usage/681")
        );
    }

    #[gtest]
    #[rstest]
    fn sealed_usage_exposes_its_bytes() {
        let bytes = vec![7u8; SEALED_USAGE_LEN];
        let sealed = SealedUsage::try_from(bytes.clone()).unwrap();
        expect_that!(sealed.as_bytes(), eq(&bytes));
    }

    #[gtest]
    #[rstest]
    fn period_public_key_exposes_its_bytes() {
        let bytes = [9u8; 32];
        let key = PeriodPublicKey::from_bytes(bytes);
        expect_that!(key.as_bytes(), eq(&bytes));
    }

    #[gtest]
    #[rstest]
    fn batch_record_exposes_sealed_bytes() {
        let bytes = vec![3u8; SEALED_USAGE_LEN];
        let sealed = SealedUsage::try_from(bytes.clone()).unwrap();
        let record = BatchRecord::new(1234, &sealed);
        expect_that!(record.sealed_bytes(), some(eq(&bytes)));
    }
}
