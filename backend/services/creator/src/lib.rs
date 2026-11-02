//! Creator accounts and the websites they connect: the registry behind the
//! creator portal. Nothing here touches readers, events or money: which site a
//! creator connected decides nothing in the viewer pipeline yet.
//!
//! Shortcuts taken on purpose:
//! - passwords are stored and compared in plain text;
//! - a site is connected as soon as it is added: no ownership check runs, so
//!   the first creator to claim a domain gets it;
//! - everything (accounts, sessions, websites) lives in memory only;
//! - sessions never expire server-side, logins are not rate limited.

pub mod public_service;
mod repository;
