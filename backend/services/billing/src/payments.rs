//! The money leg. Pool-and-distribute (PRODUCT INVARIANT 4): charges go into
//! one pool and each creator gets an independent payout from it; nothing a
//! provider receives may link a payout to a charge. No real provider yet:
//! the stub only records and logs.

use std::sync::{Arc, Mutex};

use common::ViewerBillingId;

use crate::EuroCents;
use crate::period::PeriodId;

pub trait PaymentProvider: Send + Sync {
    /// Debits a reader. Never called for 0.
    fn charge(&self, viewer: ViewerBillingId, period: PeriodId, amount: EuroCents);
    /// Pays a creator from the pool. Carries no reference to any charge.
    fn payout(&self, site: &str, period: PeriodId, amount: EuroCents);
}

#[derive(Clone, Default)]
pub struct StubPaymentProvider {
    charges: Arc<Mutex<Vec<(ViewerBillingId, PeriodId, EuroCents)>>>,
    payouts: Arc<Mutex<Vec<(String, PeriodId, EuroCents)>>>,
}

impl StubPaymentProvider {
    pub fn charges(&self) -> Vec<(ViewerBillingId, PeriodId, EuroCents)> {
        self.charges
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn payouts(&self) -> Vec<(String, PeriodId, EuroCents)> {
        self.payouts
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

impl PaymentProvider for StubPaymentProvider {
    fn charge(&self, viewer: ViewerBillingId, period: PeriodId, amount: EuroCents) {
        tracing::info!("stub payment provider: charge {amount} for period {period}");
        self.charges
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push((viewer, period, amount));
    }

    fn payout(&self, site: &str, period: PeriodId, amount: EuroCents) {
        tracing::info!(
            "stub payment provider: pay out {amount} to {site} for period {period}"
        );
        self.payouts
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push((site.to_string(), period, amount));
    }
}
