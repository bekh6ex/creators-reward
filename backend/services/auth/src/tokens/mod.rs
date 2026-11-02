use std::fmt::Formatter;
use std::ops::Add;

use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode,
};
use secrecy::ExposeSecret;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use common::Uuid;
use common::ids::TaggedId;

pub(crate) mod viewer_events;

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum Audience {
    ViewerEvents,
    ViewerDashboard,
}

/// The `aud` value, the same string serde writes for the variant.
impl std::fmt::Display for Audience {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Audience::ViewerEvents => "ViewerEvents",
            Audience::ViewerDashboard => "ViewerDashboard",
        })
    }
}

type Timestamp = i64;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Claims<Id, CustomFields> {
    #[serde(rename = "aud")]
    pub audience: Audience,
    #[serde(rename = "sub")]
    pub subject: Id,
    #[serde(rename = "iat")]
    issued_at: Timestamp,
    #[serde(rename = "nbf")]
    not_before: Timestamp,
    #[serde(rename = "exp")]
    expires_at: Timestamp,
    #[serde(rename = "jti")]
    token_id: Uuid,
    #[serde(flatten)]
    pub custom: CustomFields,
}

impl<Id, Custom> Claims<Id, Custom> {
    #[expect(
        clippy::disallowed_methods,
        reason = "JWT iat/nbf/exp; the token travels only in the viewer's own cookie, the server keeps no copy, and it is not tied to a reading event"
    )]
    fn new(vid: Id, aud: Audience, custom: Custom) -> Self {
        let now = chrono::Utc::now();
        Claims {
            audience: aud,
            subject: vid,
            issued_at: now.timestamp(),
            not_before: now.timestamp(),
            expires_at: now.add(chrono::Duration::hours(1)).timestamp(),
            token_id: Uuid::new_v4(),
            custom,
        }
    }
}

pub(crate) fn generate_token<Id: TaggedId, Custom: Serialize>(
    vid: Id,
    aud: Audience,
    custom: Custom,
    private_key: &TokenSigningKey,
) -> String {
    let header = Header::new(Algorithm::EdDSA);

    #[expect(
        clippy::disallowed_methods,
        reason = "the signing key bytes are passed to jsonwebtoken to sign the viewer's own events token; they are never logged or stored outside the process"
    )]
    let key = EncodingKey::from_ed_der(private_key.key.expose_secret());

    #[expect(
        clippy::expect_used,
        reason = "JWT encoding with a generated Ed25519 key and serializable claims cannot fail"
    )]
    {
        encode(&header, &Claims::new(vid, aud, custom), &key).expect("JWT encoding")
    }
}

pub(crate) fn decode_token<Id: TaggedId, Custom: DeserializeOwned>(
    token: &str,
    aud: Audience,
    key: &TokenVerificationKey,
) -> Result<TokenData<Claims<Id, Custom>>, jsonwebtoken::errors::Error> {
    let mut validation = Validation::new(Algorithm::EdDSA);
    validation.validate_exp = true;
    validation.set_audience(&[aud]);
    validation.validate_nbf = true;

    decode::<Claims<Id, Custom>>(token, &key.key, &validation)
}

#[derive(Debug, thiserror::Error)]
#[error("Token validation error: {0}")]
pub struct TokenValidationError(#[from] jsonwebtoken::errors::Error);

#[derive(Clone)]
pub struct TokenVerificationKey {
    key: DecodingKey,
}

impl TokenVerificationKey {
    pub fn new(der: Vec<u8>) -> Self {
        TokenVerificationKey {
            key: DecodingKey::from_ed_der(&der),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TokenSigningKey {
    key: secrecy::SecretSlice<u8>,
}

impl TokenSigningKey {
    pub fn new(der: Vec<u8>) -> TokenSigningKey {
        TokenSigningKey { key: der.into() }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use ed25519_dalek::pkcs8::EncodePrivateKey;
    use ed25519_dalek::{SigningKey, VerifyingKey};
    use googletest::prelude::*;
    use rand::rngs::OsRng;
    use rstest::{fixture, rstest};

    use common::{ViewerEventsId, ViewerIdAuthentication};

    use crate::tokens::viewer_events::{
        EventsClaims, RawViewerEventsToken, ValidatedViewerEventsToken,
    };
    use crate::tokens::{
        Audience, TokenSigningKey, TokenVerificationKey, decode_token, generate_token,
    };

    pub(crate) struct KeyPair {
        pub signing: TokenSigningKey,
        pub verifying: TokenVerificationKey,
    }

    #[fixture]
    pub(crate) fn key_pair() -> KeyPair {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = VerifyingKey::from(&signing_key);
        KeyPair {
            signing: TokenSigningKey::new(
                signing_key
                    .to_pkcs8_der()
                    .unwrap()
                    .to_bytes()
                    .to_vec(),
            ),
            verifying: TokenVerificationKey::new(verifying_key.to_bytes().to_vec()),
        }
    }

    #[fixture]
    fn other_key_pair() -> KeyPair {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = VerifyingKey::from(&signing_key);
        KeyPair {
            signing: TokenSigningKey::new(
                signing_key
                    .to_pkcs8_der()
                    .unwrap()
                    .to_bytes()
                    .to_vec(),
            ),
            verifying: TokenVerificationKey::new(verifying_key.to_bytes().to_vec()),
        }
    }

    #[gtest]
    #[rstest]
    fn token_encoding_decoding(#[from(key_pair)] keys: KeyPair) {
        let vid = ViewerIdAuthentication::new();
        let mut custom = HashMap::new();
        custom.insert("custom-key", "custom-value");
        let token = generate_token(vid, Audience::ViewerDashboard, custom, &keys.signing);

        let result = decode_token::<ViewerIdAuthentication, HashMap<String, String>>(
            &token,
            Audience::ViewerDashboard,
            &keys.verifying,
        )
        .expect("Generated bad token");

        expect_that!(result.claims.subject, eq(vid));
        expect_that!(result.claims.audience, eq(&Audience::ViewerDashboard));
        expect_that!(
            result.claims.custom,
            has_entry("custom-key".to_string(), eq("custom-value"))
        );
    }

    #[gtest]
    #[rstest]
    fn events_token_roundtrip(#[from(key_pair)] keys: KeyPair) {
        let id = ViewerEventsId::new();
        let raw_token = RawViewerEventsToken::generate(id, &keys.signing);

        let validated =
            ValidatedViewerEventsToken::try_from(raw_token, &keys.verifying).unwrap();

        expect_that!(validated.claims.subject, eq(id));
    }

    #[gtest]
    #[rstest]
    fn token_signed_by_another_key_is_rejected(
        #[from(key_pair)] keys: KeyPair,
        #[from(other_key_pair)] other: KeyPair,
    ) {
        let id = ViewerEventsId::new();
        let raw_token = RawViewerEventsToken::generate(id, &keys.signing);

        let result = ValidatedViewerEventsToken::try_from(raw_token, &other.verifying);

        expect_that!(result, err(anything()));
    }

    #[gtest]
    #[rstest]
    fn token_with_wrong_algorithm_is_rejected(#[from(key_pair)] keys: KeyPair) {
        use jsonwebtoken::{EncodingKey, Header, encode};

        let id = ViewerEventsId::new();
        let token = encode(
            &Header::new(jsonwebtoken::Algorithm::HS256),
            &super::Claims::new(id, Audience::ViewerEvents, EventsClaims {}),
            &EncodingKey::from_secret(b"not-the-signing-key"),
        )
        .unwrap();

        let result = ValidatedViewerEventsToken::try_from(
            RawViewerEventsToken::from(token),
            &keys.verifying,
        );

        expect_that!(result, err(anything()));
    }

    #[gtest]
    #[rstest]
    fn raw_events_token_converts_to_its_raw_string(#[from(key_pair)] keys: KeyPair) {
        let id = ViewerEventsId::new();
        let raw_token = RawViewerEventsToken::generate(id, &keys.signing);
        let as_string = String::from(raw_token);

        expect_that!(as_string, contains_substring("."));
        let validated =
            ValidatedViewerEventsToken::try_from(&as_string, &keys.verifying).unwrap();
        expect_that!(validated.claims.subject, eq(id));
    }
}
