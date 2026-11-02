use std::fmt::Debug;

use futures::SinkExt;
use thiserror;

use common::ViewerEventsId;
use common::billing_usage::UsageRecorded;
use common::viewer_registration::ViewerPublicKey;

use crate::enabled_donations::{
    EnabledDonationId, EnabledDonations, SealedEnabledDonation,
};
use crate::keys::ViewerPublicKeys;

#[derive(Clone)]
pub struct WebViewerEventsService<US> {
    usage: US,
    keys: ViewerPublicKeys,
    enabled_donations: EnabledDonations,
}

#[derive(thiserror::Error, Debug)]
pub enum Error<SinkError> {
    SendError(#[from] SinkError),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("public key not registered for this viewer")]
pub struct PublicKeyError;

impl<US> WebViewerEventsService<US>
where
    US: futures::Sink<UsageRecorded> + Clone + Unpin,
{
    pub fn new(usage: US, keys: ViewerPublicKeys) -> Self {
        WebViewerEventsService {
            usage,
            enabled_donations: EnabledDonations::new(),
            keys,
        }
    }

    pub async fn public_key(
        &self,
        viewer_id: ViewerEventsId,
    ) -> Result<ViewerPublicKey, PublicKeyError> {
        self.keys
            .get(viewer_id)
            .await
            .ok_or(PublicKeyError)
    }

    /// Stores a record the viewer's browser sealed; returns its id.
    pub async fn enable_donations(
        &self,
        viewer_id: ViewerEventsId,
        sealed: SealedEnabledDonation,
    ) -> EnabledDonationId {
        self.enabled_donations
            .add(viewer_id, sealed)
            .await
    }

    pub async fn enabled_donations(
        &self,
        viewer_id: ViewerEventsId,
    ) -> Vec<(EnabledDonationId, SealedEnabledDonation)> {
        self.enabled_donations.list(viewer_id).await
    }

    pub async fn donations_enabled(
        &self,
        viewer_id: ViewerEventsId,
        id: EnabledDonationId,
    ) -> bool {
        self.enabled_donations
            .contains(viewer_id, id)
            .await
    }

    pub async fn disable_donations(
        &self,
        viewer_id: ViewerEventsId,
        id: EnabledDonationId,
    ) {
        self.enabled_donations.remove(viewer_id, id).await
    }

    /// Passes a billing record on. It is sealed to the period key, which
    /// neither this service nor billing holds.
    pub async fn record_usage(
        &self,
        usage: UsageRecorded,
    ) -> Result<(), Error<US::Error>> {
        let mut sink = self.usage.clone();
        sink.send(usage).await?;
        Ok(())
    }
}
