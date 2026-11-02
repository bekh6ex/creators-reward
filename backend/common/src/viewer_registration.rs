//! Events emitted by the auth service when a viewer registers.
//!
//! Registration is split into two events on purpose: each consumer learns only
//! what it needs. The id exchange receives every id of the viewer but never the
//! public key (`id_storage::internal_api::ViewerRegistered`); the events
//! service receives the public key but only its own id (below).

use crate::ViewerEventsId;

/// Consumed by the events service, which is the only place the public key is
/// stored.
#[derive(Debug, Clone)]
pub struct ViewerEventsKeyRegistered {
    pub events_id: ViewerEventsId,
    pub public_key: ViewerPublicKey,
}

/// Raw X25519 public key, generated in the viewer's browser. The matching
/// private key never reaches the server in usable form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewerPublicKey([u8; ViewerPublicKey::LEN]);

impl ViewerPublicKey {
    pub const LEN: usize = 32;

    pub fn as_bytes(&self) -> &[u8; Self::LEN] {
        &self.0
    }
}

#[derive(Debug, thiserror::Error)]
#[error(
    "viewer public key must be {} bytes, got {0}",
    ViewerPublicKey::LEN
)]
pub struct InvalidViewerPublicKey(usize);

impl TryFrom<&[u8]> for ViewerPublicKey {
    type Error = InvalidViewerPublicKey;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        <[u8; Self::LEN]>::try_from(bytes)
            .map(ViewerPublicKey)
            .map_err(|_| InvalidViewerPublicKey(bytes.len()))
    }
}

#[cfg(test)]
mod tests {
    use googletest::prelude::*;
    use rstest::rstest;

    use super::*;

    #[gtest]
    #[rstest]
    #[case::exact(32, true)]
    #[case::short(31, false)]
    #[case::long(33, false)]
    fn public_key_accepts_exactly_32_bytes(#[case] len: usize, #[case] accepted: bool) {
        expect_that!(
            ViewerPublicKey::try_from(&vec![7u8; len][..]).is_ok(),
            eq(accepted)
        );
    }
}
