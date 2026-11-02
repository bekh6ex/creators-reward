//! Event emitted by the events service for billing.

use crate::ViewerEventsId;

/// One unit of reading on a site the viewer enabled donations for, sealed in
/// the browser to the period key (format: `billing::ceremony_format`).
/// Consumed only by billing, which translates the events id on arrival and
/// never stores it. Neither service can open `sealed`.
#[derive(Debug, Clone)]
pub struct UsageRecorded {
    pub viewer_id: ViewerEventsId,
    pub period: u64,
    pub sealed: Vec<u8>,
}
