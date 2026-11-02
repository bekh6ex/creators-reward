use std::hash::Hash;

use serde::Serialize;
use serde::de::DeserializeOwned;
use uuid::Uuid;

use tagged_id::tagged_id;

pub trait TaggedId:
    Hash + Copy + Eq + Send + Sync + Serialize + DeserializeOwned
{
    fn new() -> Self;
}

#[tagged_id(tag = "VA")]
pub struct ViewerIdAuthentication(Uuid);

#[tagged_id(tag = "VE")]
pub struct ViewerEventsId(Uuid);

#[tagged_id(tag = "VB")]
pub struct ViewerBillingId(Uuid);
