//! The internal API of the id exchange: what other services use, and the only
//! part of this crate they should import.
//!
//! - [`ViewerIdSet`] + [`ViewerRegistered`] — the registration message the
//!   auth service sends; it carries all of a new viewer's ids at once.
//! - [`IdExchange`] / [`IdExchanger`] — a typed, single-direction translation
//!   handle (a stand-in for an API call). A service holds one per translation
//!   it needs and can translate nothing else.

use std::marker::PhantomData;

use async_trait::async_trait;
use getset::CopyGetters;
use serde::{Deserialize, Serialize};

use common::ids::TaggedId;
use common::{ViewerBillingId, ViewerEventsId, ViewerIdAuthentication};

use crate::service::IdStorage;

/// Every id of one viewer.
#[derive(Debug, Clone, Copy, CopyGetters, Serialize, Deserialize)]
#[getset(get_copy = "pub")]
#[serde(rename_all = "camelCase")]
pub struct ViewerIdSet {
    auth: ViewerIdAuthentication,
    billing: ViewerBillingId,
    events: ViewerEventsId,
}

impl ViewerIdSet {
    /// Mints the remaining ids of a viewer the auth service just registered.
    pub fn new(auth_id: ViewerIdAuthentication) -> ViewerIdSet {
        ViewerIdSet {
            auth: auth_id,
            billing: ViewerBillingId::new(),
            events: ViewerEventsId::new(),
        }
    }
}

/// Sent by the auth service, consumed only by the id exchange: every id the
/// auth service minted for a new viewer, in one message. Services never
/// receive this; they translate ids through their `IdExchanger`s.
#[derive(Debug, Clone, Copy)]
pub struct ViewerRegistered {
    pub ids: ViewerIdSet,
}

#[async_trait]
pub trait IdExchange<Id1: TaggedId, Id2: TaggedId>
where
    Self: Sync + Send,
{
    async fn get_secondary(&self, from: Id1) -> Option<Id2>;
}

/// Translates `Id1` into `Id2` and nothing else. Only `IdStorage` creates
/// these, at wiring time.
#[derive(Clone)]
pub struct IdExchanger<Id1, Id2> {
    id_storage: IdStorage,
    _id1: PhantomData<Id1>,
    _id2: PhantomData<Id2>,
}

impl<Id1, Id2> IdExchanger<Id1, Id2> {
    pub(crate) fn new(id_storage: IdStorage) -> Self {
        IdExchanger {
            id_storage,
            _id1: PhantomData,
            _id2: PhantomData,
        }
    }
}

#[async_trait]
impl<Id1: TaggedId + 'static, Id2: TaggedId + 'static> IdExchange<Id1, Id2>
    for IdExchanger<Id1, Id2>
{
    async fn get_secondary(&self, from: Id1) -> Option<Id2> {
        self.id_storage.get_secondary(from).await
    }
}
