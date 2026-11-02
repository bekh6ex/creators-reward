#![expect(
    clippy::disallowed_types,
    reason = "id_storage owns ViewerIdSet; every other service translates ids through IdExchanger"
)]

//! The id exchange: the only place that knows which ids belong to the same
//! viewer.
//!
//! An internal-only service. It is never reachable from an HTTP gateway:
//! it is filled solely by `ViewerRegistered` events and queried solely through
//! the typed [`internal_api::IdExchanger`]s handed to each service at wiring
//! time, one per translation that service needs.
//!
//! - [`internal_api`] — everything other services use.
//! - [`Wiring`] — for the composition root (`create_state`) only: hands out
//!   exchangers and the registration listener. The storage itself is private.

pub mod internal_api;
mod service;
mod wiring;

pub use wiring::{Wiring, create_wiring};
