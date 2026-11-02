//! The billing key ceremony, the offline half of billing.
//!
//! - [`generate`]: makes a period's X25519 key pair and Shamir-splits the
//!   private key into `holders` shares, any `threshold` of which recover it.
//!   The whole private key exists only in this process's memory.
//! - [`tally`]: recovers the key from enough shares, opens every record of a
//!   period's batch, and returns only the sum of weights per site. No record,
//!   reader or per-record result leaves this function.
//!
//! The server never runs this code. It only receives the public key before
//! the period and the totals after it.

use std::collections::{BTreeMap, BTreeSet};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use blahaj::{Share, Sharks};
use hpke::aead::AesGcm256;
use hpke::kdf::HkdfSha256;
use hpke::kem::X25519HkdfSha256;
use hpke::{Deserializable, Kem as _, OpModeR, OpModeS, Serializable};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize, Serializer};
use zeroize::Zeroizing;

use billing::ceremony_format::{
    Batch, ENC_LEN, PADDED_LEN, PeriodPublicKey, SealedUsage, Totals, UsagePlaintext,
    hpke_info,
};
use billing::period::PeriodId;

type Kem = X25519HkdfSha256;

/// One key holder's part of a period's private key. Holds the public key too,
/// so a tally can check that the shares recovered the right key. `Debug`
/// prints the share redacted; only writing the share file exposes it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareFile {
    pub period: PeriodId,
    pub threshold: u8,
    pub holders: u8,
    pub holder: u8,
    pub public_key: String,
    #[serde(serialize_with = "expose_share")]
    pub share: SecretString,
}

#[expect(
    clippy::disallowed_methods,
    reason = "offline key-holder share file: the secret must be written to disk for the ceremony"
)]
fn expose_share<S: Serializer>(
    share: &SecretString,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(share.expose_secret())
}

#[derive(Debug)]
pub struct KeyGeneration {
    pub public_key: PeriodPublicKey,
    pub shares: Vec<ShareFile>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("need 1 <= threshold <= holders <= 255, got {threshold} of {holders}")]
    BadThreshold { threshold: u8, holders: u8 },
    #[error("shares are not all for period {0} with the same key and threshold")]
    MixedShares(PeriodId),
    #[error("need {needed} different holders' shares, got {got}")]
    NotEnoughShares { needed: u8, got: usize },
    #[error("malformed share")]
    MalformedShare,
    #[error("the shares do not recover the period's key")]
    WrongKey,
    #[error("per-site weight overflow")]
    WeightOverflow,
}

pub fn generate(
    period: PeriodId,
    threshold: u8,
    holders: u8,
) -> Result<KeyGeneration, Error> {
    if threshold == 0 || threshold > holders || holders == u8::MAX {
        return Err(Error::BadThreshold { threshold, holders });
    }
    let (private_key, public_key) = Kem::gen_keypair();
    let secret = Zeroizing::new(private_key.to_bytes().to_vec());
    let public_key = PeriodPublicKey::from_bytes(public_key.to_bytes().into());
    let shares = Sharks(threshold)
        .dealer(&secret)
        .zip(1..=holders)
        .map(|(share, holder)| ShareFile {
            period,
            threshold,
            holders,
            holder,
            public_key: public_key.to_base64url(),
            share: URL_SAFE_NO_PAD
                .encode(Zeroizing::new(Vec::from(&share)))
                .into(),
        })
        .collect();
    Ok(KeyGeneration { public_key, shares })
}

#[expect(
    clippy::disallowed_methods,
    reason = "offline ceremony reads shares from key-holder files to reconstruct the period key"
)]
fn recover(
    period: PeriodId,
    shares: &[ShareFile],
) -> Result<<Kem as hpke::Kem>::PrivateKey, Error> {
    let first = shares
        .first()
        .ok_or(Error::NotEnoughShares { needed: 1, got: 0 })?;
    if shares.iter().any(|s| {
        s.period != period
            || s.public_key != first.public_key
            || s.threshold != first.threshold
    }) {
        return Err(Error::MixedShares(period));
    }
    let holders: BTreeSet<u8> = shares.iter().map(|s| s.holder).collect();
    if holders.len() < first.threshold as usize {
        return Err(Error::NotEnoughShares {
            needed: first.threshold,
            got: holders.len(),
        });
    }
    let decoded = shares
        .iter()
        .map(|s| {
            let bytes = Zeroizing::new(
                URL_SAFE_NO_PAD
                    .decode(s.share.expose_secret())
                    .map_err(|_| Error::MalformedShare)?,
            );
            Share::try_from(bytes.as_slice()).map_err(|_| Error::MalformedShare)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let secret = Zeroizing::new(
        Sharks(first.threshold)
            .recover(&decoded)
            .map_err(|_| Error::MalformedShare)?,
    );
    let private_key = <Kem as hpke::Kem>::PrivateKey::from_bytes(&secret)
        .map_err(|_| Error::WrongKey)?;
    let expected = PeriodPublicKey::from_base64url(&first.public_key)
        .map_err(|_| Error::MalformedShare)?;
    let actual: [u8; 32] = Kem::sk_to_pk(&private_key).to_bytes().into();
    if actual != *expected.as_bytes() {
        return Err(Error::WrongKey);
    }
    Ok(private_key)
}

/// Opens every record and sums the weights per site. Records that do not open
/// or do not name an origin are counted as dropped and left out.
pub fn tally(batch: &Batch, shares: &[ShareFile]) -> Result<Totals, Error> {
    let private_key = recover(batch.period, shares)?;
    let info = hpke_info(batch.period);
    let mut per_site_millicents: BTreeMap<String, u64> = BTreeMap::new();
    let mut dropped: u64 = 0;
    for record in &batch.records {
        match record
            .sealed_bytes()
            .and_then(|bytes| open_site(&private_key, info.as_bytes(), &bytes))
        {
            Some(site) => {
                let entry = per_site_millicents.entry(site).or_default();
                *entry = entry
                    .checked_add(record.weight_millicents)
                    .ok_or(Error::WeightOverflow)?;
            }
            None => {
                // At most batch.records.len(), so this never saturates.
                dropped = dropped.saturating_add(1);
            }
        }
    }
    Ok(Totals {
        period: batch.period,
        per_site_millicents,
        records: batch.records.len() as u64,
        dropped,
    })
}

fn open_site(
    private_key: &<Kem as hpke::Kem>::PrivateKey,
    info: &[u8],
    sealed: &[u8],
) -> Option<String> {
    let sealed = SealedUsage::try_from(sealed.to_vec()).ok()?;
    let (enc, ciphertext) = sealed.as_bytes().split_at(ENC_LEN);
    let enc = <Kem as hpke::Kem>::EncappedKey::from_bytes(enc).ok()?;
    let plaintext = Zeroizing::new(
        hpke::single_shot_open::<AesGcm256, HkdfSha256, Kem>(
            &OpModeR::Base,
            private_key,
            &enc,
            info,
            ciphertext,
            b"",
        )
        .ok()?,
    );
    let usage: UsagePlaintext = serde_json::from_slice(&plaintext).ok()?;
    is_origin(&usage.site).then_some(usage.site)
}

fn is_origin(site: &str) -> bool {
    url::Url::parse(site)
        .ok()
        .filter(|url| matches!(url.scheme(), "https" | "http"))
        .is_some_and(|url| url.origin().ascii_serialization() == site)
}

/// What the events iframe does, in Rust: the reference sender for tests.
#[expect(
    clippy::expect_used,
    reason = "seal_usage is a test/reference sender with a valid X25519 key and short origin"
)]
pub fn seal_usage(
    public_key: &PeriodPublicKey,
    period: PeriodId,
    site: &str,
) -> SealedUsage {
    let mut plaintext = serde_json::to_vec(&UsagePlaintext {
        site: site.to_string(),
    })
    .expect("serializable");
    assert!(plaintext.len() <= PADDED_LEN, "site origin too long");
    plaintext.resize(PADDED_LEN, b' ');
    let recipient = <Kem as hpke::Kem>::PublicKey::from_bytes(public_key.as_bytes())
        .expect("32-byte X25519 key");
    let (enc, ciphertext) = hpke::single_shot_seal::<AesGcm256, HkdfSha256, Kem>(
        &OpModeS::Base,
        &recipient,
        hpke_info(period).as_bytes(),
        &plaintext,
        b"",
    )
    .expect("sealing");
    let mut record = enc.to_bytes().to_vec();
    record.extend(ciphertext);
    SealedUsage::try_from(record).expect("padded record has the fixed length")
}

#[cfg(test)]
mod tests {
    use billing::ceremony_format::BatchRecord;
    use googletest::prelude::*;
    use rstest::{fixture, rstest};

    use super::*;

    const P: PeriodId = PeriodId::new(681);

    /// Keys for `P` that any two of three holders can recover.
    #[fixture]
    fn keys() -> KeyGeneration {
        generate(P, 2, 3).unwrap()
    }

    fn batch(key: &PeriodPublicKey, period: PeriodId, records: &[(u64, &str)]) -> Batch {
        Batch {
            period,
            records: records
                .iter()
                .map(|(weight, site)| {
                    BatchRecord::new(*weight, &seal_usage(key, period, site))
                })
                .collect(),
        }
    }

    #[gtest]
    #[rstest]
    #[case::first_and_second([0, 1])]
    #[case::first_and_third([0, 2])]
    #[case::second_and_third([1, 2])]
    fn any_two_of_three_holders_recover_the_totals(
        keys: KeyGeneration,
        #[case] holders: [usize; 2],
    ) {
        let batch = batch(
            &keys.public_key,
            P,
            &[
                (10_000, "https://a.example"),
                (2_500, "https://b.example:8443"),
                (10_000, "https://a.example"),
            ],
        );
        let shares = holders.map(|i| keys.shares[i].clone());

        let totals = tally(&batch, &shares).unwrap();

        expect_that!(
            totals.per_site_millicents,
            elements_are![
                (eq("https://a.example"), eq(&20_000)),
                (eq("https://b.example:8443"), eq(&2_500)),
            ]
        );
        expect_that!(totals.records, eq(3));
        expect_that!(totals.dropped, eq(0));
    }

    #[gtest]
    #[rstest]
    fn one_share_is_not_enough(keys: KeyGeneration) {
        let batch = batch(&keys.public_key, P, &[(1, "https://a.example")]);
        expect_that!(
            tally(&batch, &keys.shares[..1]),
            err(pat!(Error::NotEnoughShares {
                needed: eq(&2),
                got: eq(&1)
            }))
        );
        let twice = [keys.shares[0].clone(), keys.shares[0].clone()];
        expect_that!(
            tally(&batch, &twice),
            err(pat!(Error::NotEnoughShares { .. }))
        );
    }

    #[gtest]
    #[rstest]
    fn the_threshold_is_configurable() {
        let keys = generate(P, 3, 5).unwrap();
        expect_that!(keys.shares, len(eq(5)));
        let batch = batch(&keys.public_key, P, &[(7, "https://a.example")]);
        expect_that!(tally(&batch, &keys.shares[1..3]), err(anything()));
        let totals = tally(&batch, &keys.shares[2..5]).unwrap();
        expect_that!(
            totals.per_site_millicents,
            contains((eq("https://a.example"), eq(&7)))
        );
    }

    #[gtest]
    #[rstest]
    #[case::zero(0, 3)]
    #[case::more_than_holders(4, 3)]
    fn the_threshold_must_be_between_one_and_the_holders(
        #[case] threshold: u8,
        #[case] holders: u8,
    ) {
        expect_that!(
            generate(P, threshold, holders),
            err(pat!(Error::BadThreshold { .. }))
        );
    }

    #[gtest]
    #[rstest]
    #[expect(
        clippy::disallowed_methods,
        reason = "tests: verifying that Debug redacts secrets requires exposing them"
    )]
    fn shares_are_redacted_in_debug_but_written_to_the_share_file(keys: KeyGeneration) {
        let share = keys.shares[0].share.expose_secret();
        expect_that!(format!("{keys:?}"), not(contains_substring(share)));
        expect_that!(
            serde_json::to_string(&keys.shares[0]).unwrap(),
            contains_substring(format!(r#""share":"{share}""#))
        );
    }

    #[gtest]
    #[rstest]
    fn shares_of_another_period_are_refused(keys: KeyGeneration) {
        let other = generate(P.next(), 2, 3).unwrap();
        let batch = batch(&keys.public_key, P, &[(1, "https://a.example")]);
        expect_that!(
            tally(&batch, &other.shares[..2]),
            err(pat!(Error::MixedShares(_)))
        );
        let mixed = [keys.shares[0].clone(), other.shares[1].clone()];
        expect_that!(tally(&batch, &mixed), err(pat!(Error::MixedShares(_))));
    }

    #[gtest]
    #[rstest]
    fn records_of_another_period_or_not_naming_an_origin_are_dropped(
        keys: KeyGeneration,
    ) {
        let mut batch = batch(
            &keys.public_key,
            P,
            &[(5, "https://a.example"), (5, "https://a.example/article")],
        );
        // Sealed for the next period with the same key: the info differs.
        batch.records.push(BatchRecord::new(
            5,
            &seal_usage(&keys.public_key, P.next(), "https://a.example"),
        ));
        batch.records.push(BatchRecord {
            weight_millicents: 5,
            sealed: "not base64!".into(),
        });

        let totals = tally(&batch, &keys.shares[..2]).unwrap();

        expect_that!(
            totals.per_site_millicents,
            elements_are![(eq("https://a.example"), eq(&5))]
        );
        expect_that!(totals.records, eq(4));
        expect_that!(totals.dropped, eq(3));
    }
}
