//! Billing: one charge per reader and one payout per creator per period,
//! without ever holding which reader funded which creator.
//!
//! The flow:
//!
//! 1. Every period has an X25519 key pair, generated offline by the key
//!    ceremony (`tools/billing_ceremony`). The private key is split between
//!    key holders and never reaches the server; only the public key does.
//! 2. While a reader reads, the events iframe seals `{site}` to the period
//!    public key in the browser. Billing stores the sealed record under the
//!    reader's [`ViewerBillingId`](common::ViewerBillingId) and cannot open it.
//! 3. When the period ends, billing computes per reader the record count `n`,
//!    the amount `T = min(limit, n * unit price)` and the weight `T / n` of
//!    each record. It strips the reader ids, shuffles, and exports a
//!    [`Batch`](ceremony_format::Batch) of `(weight, sealed)` pairs.
//! 4. Key holders fetch the batch through the admin gateway, reconstruct the
//!    key offline, and the published tool opens the batch and outputs only
//!    per-site [`Totals`](ceremony_format::Totals), which they upload.
//! 5. Only then are readers charged and creators paid. The period's records
//!    are gone by then, and the key is destroyed after the ceremony.
//!
//! Gateways get [`service::PublicService`] (viewer) or
//! [`admin_service::AdminService`] (admin); keys, batches and totals move
//! only through the latter.
//!
//! Amount-blind by construction (PRODUCT INVARIANT 3): a reader with a limit
//! of 0 produces records like anyone else; they just get weight 0. The only
//! place an amount changes behaviour is the money leg, which never calls the
//! payment provider for a 0 charge.

pub mod admin_service;
pub mod ceremony_format;
mod core;
mod money;
pub mod payments;
pub mod period;
#[cfg(kani)]
mod proofs;
pub mod service;
pub mod wiring;

pub use money::EuroCents;
pub use wiring::{BillingConfig, Wiring, create_wiring};
