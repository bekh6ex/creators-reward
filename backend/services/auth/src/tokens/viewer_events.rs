use std::fmt;

use serde::{Deserialize, Serialize};

use common::ViewerEventsId;

use crate::tokens;
use crate::tokens::{
    Audience, Claims, TokenSigningKey, TokenValidationError, TokenVerificationKey,
};

pub struct RawViewerEventsToken {
    raw: String,
}

impl RawViewerEventsToken {
    pub fn generate(
        id: ViewerEventsId,
        private_key: &TokenSigningKey,
    ) -> RawViewerEventsToken {
        let raw = tokens::generate_token(
            id,
            Audience::ViewerEvents,
            EventsClaims {},
            private_key,
        );
        RawViewerEventsToken { raw }
    }
}

impl fmt::Display for RawViewerEventsToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl AsRef<str> for RawViewerEventsToken {
    fn as_ref(&self) -> &str {
        &self.raw
    }
}

impl From<RawViewerEventsToken> for String {
    fn from(token: RawViewerEventsToken) -> String {
        token.raw
    }
}

impl From<String> for RawViewerEventsToken {
    fn from(raw: String) -> Self {
        RawViewerEventsToken { raw }
    }
}

#[derive(Debug)]
pub struct ValidatedViewerEventsToken {
    pub claims: Claims<ViewerEventsId, EventsClaims>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventsClaims {}

impl ValidatedViewerEventsToken {
    pub fn try_from<S: AsRef<str>>(
        unverified_token: S,
        pub_key: &TokenVerificationKey,
    ) -> Result<Self, TokenValidationError> {
        let data = tokens::decode_token::<ViewerEventsId, EventsClaims>(
            unverified_token.as_ref(),
            Audience::ViewerEvents,
            pub_key,
        )?;

        Ok(ValidatedViewerEventsToken {
            claims: data.claims,
        })
    }

    pub fn viewer_id(&self) -> ViewerEventsId {
        self.claims.subject
    }
}
