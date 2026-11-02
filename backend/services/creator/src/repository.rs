use std::collections::HashMap;

use crate::public_service::{CreatorId, Website};

pub(crate) struct Account {
    pub email: String,
    /// Plain text: the agreed shortcut for now (see the crate docs).
    pub password: String,
}

/// One lock around everything, so "is this email / domain free" and the insert
/// that claims it are a single step.
#[derive(Default)]
pub(crate) struct Repository {
    pub accounts: HashMap<CreatorId, Account>,
    pub by_email: HashMap<String, CreatorId>,
    pub sessions: HashMap<String, CreatorId>,
    /// In the order they were added. A domain appears at most once.
    pub websites: Vec<(CreatorId, Website)>,
}
