//! What the admin gateway may call: the online half of the key ceremony. The
//! offline half (`tools/billing_ceremony`) generates keys and opens batches;
//! this side takes public keys in, hands batches out and takes totals back.
//! Nothing here ever sees a private key.

use std::sync::Arc;

use crate::ceremony_format::{Batch, PeriodPublicKey, Totals};
pub use crate::core::{KeyRejected, LogEntry, SettleError};

use crate::EuroCents;
use crate::core::Billing;
use crate::payments::PaymentProvider;
use crate::period::{Clock, PeriodId, PeriodLength};

#[derive(Clone)]
pub struct AdminService {
    billing: Billing,
    period_length: PeriodLength,
    payments: Arc<dyn PaymentProvider>,
    clock: Clock,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AddKeyError {
    #[error("period {0} has already ended")]
    Ended(PeriodId),
    #[error(transparent)]
    Rejected(#[from] KeyRejected),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettlementSummary {
    pub charged: EuroCents,
    pub paid_out: EuroCents,
}

impl AdminService {
    pub(crate) fn new(
        billing: Billing,
        period_length: PeriodLength,
        payments: Arc<dyn PaymentProvider>,
        clock: Clock,
    ) -> Self {
        AdminService {
            billing,
            period_length,
            payments,
            clock,
        }
    }

    pub async fn add_period_key(
        &self,
        period: PeriodId,
        key: PeriodPublicKey,
    ) -> Result<(), AddKeyError> {
        if period < self.period_length.period_at((self.clock)()) {
            return Err(AddKeyError::Ended(period));
        }
        Ok(self.billing.add_period_key(period, key).await?)
    }

    /// Present from the period's close until its totals are settled.
    pub async fn batch(&self, period: PeriodId) -> Option<Batch> {
        self.billing.batch(period).await
    }

    /// Readers are charged only now, after a successful ceremony.
    pub async fn settle(
        &self,
        totals: &Totals,
    ) -> Result<SettlementSummary, SettleError> {
        let settlement = self.billing.settle(totals).await?;
        for (viewer, amount) in &settlement.charges {
            if amount.0 > 0 {
                self.payments
                    .charge(*viewer, settlement.period, *amount);
            }
        }
        for (site, amount) in &settlement.payouts {
            self.payments
                .payout(site, settlement.period, *amount);
        }
        let summary = SettlementSummary {
            charged: settlement.charges.iter().map(|(_, a)| *a).sum(),
            paid_out: settlement.payouts.iter().map(|(_, a)| *a).sum(),
        };
        tracing::info!(
            "billing: settled period {}: charged {}, paid out {}",
            self.period_length.label(settlement.period),
            summary.charged,
            summary.paid_out
        );
        Ok(summary)
    }

    pub async fn log(&self) -> Vec<LogEntry> {
        self.billing.log().await
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use common::ViewerBillingId;

    use crate::ceremony_format::{SEALED_USAGE_LEN, SealedUsage};
    use crate::core::Pricing;
    use crate::payments::StubPaymentProvider;

    use googletest::prelude::*;
    use rstest::rstest;

    use super::*;

    #[gtest]
    #[rstest]
    fn zero_amount_charges_are_not_passed_to_payments() {
        futures::executor::block_on(async {
            let length = PeriodLength::CalendarMonth;
            let period = length.current();
            let billing = Billing::new(Arc::new(crate::period::now_unix_sec));
            let payments = StubPaymentProvider::default();
            let admin = AdminService::new(
                billing.clone(),
                length,
                Arc::new(payments.clone()),
                Arc::new(crate::period::now_unix_sec),
            );
            let key = PeriodPublicKey::from_bytes([4; 32]);
            admin.add_period_key(period, key).await.unwrap();

            let zero_viewer = ViewerBillingId::new();
            billing.set_limit(zero_viewer, EuroCents(0)).await;
            let sealed = SealedUsage::try_from(vec![1; SEALED_USAGE_LEN]).unwrap();
            billing
                .record_usage(zero_viewer, period, sealed)
                .await
                .unwrap();

            let pricing = Pricing {
                unit_price: EuroCents(10),
                default_limit: EuroCents(1000),
            };
            billing.close(period, pricing).await.unwrap();

            let totals = Totals {
                period,
                per_site_millicents: BTreeMap::new(),
                records: 1,
                dropped: 0,
            };
            admin.settle(&totals).await.unwrap();

            expect_that!(payments.charges(), is_empty());
        });
    }

    #[gtest]
    #[rstest]
    fn charges_and_pays_only_after_the_totals() {
        futures::executor::block_on(async {
            let length = PeriodLength::CalendarMonth;
            let period = length.current();
            let billing = Billing::new(Arc::new(crate::period::now_unix_sec));
            let payments = StubPaymentProvider::default();
            let admin = AdminService::new(
                billing.clone(),
                length,
                Arc::new(payments.clone()),
                Arc::new(crate::period::now_unix_sec),
            );
            let key = PeriodPublicKey::from_bytes([4; 32]);

            expect_that!(
                admin
                    .add_period_key(
                        PeriodId::new(period.get().checked_sub(1).unwrap()),
                        key
                    )
                    .await,
                err(eq(&AddKeyError::Ended(PeriodId::new(
                    period.get().checked_sub(1).unwrap()
                ))))
            );
            admin.add_period_key(period, key).await.unwrap();
            let viewer = ViewerBillingId::new();
            for _ in 0..2 {
                let sealed = SealedUsage::try_from(vec![1; SEALED_USAGE_LEN]).unwrap();
                billing
                    .record_usage(viewer, period, sealed)
                    .await
                    .unwrap();
            }
            let pricing = Pricing {
                unit_price: EuroCents(10),
                default_limit: EuroCents(1000),
            };
            billing.close(period, pricing).await.unwrap();
            expect_that!(
                admin.batch(period).await,
                some(field!(&Batch.records, ref len(eq(2))))
            );
            expect_that!(payments.charges(), is_empty());

            let totals = Totals {
                period,
                per_site_millicents: BTreeMap::from([(
                    "https://a.example".into(),
                    20_000,
                )]),
                records: 2,
                dropped: 0,
            };
            let summary = admin.settle(&totals).await.unwrap();

            expect_that!(
                summary,
                eq(SettlementSummary {
                    charged: EuroCents(20),
                    paid_out: EuroCents(20)
                })
            );
            expect_that!(
                payments.charges(),
                elements_are![(eq(&viewer), eq(&period), eq(&EuroCents(20)))]
            );
            expect_that!(
                payments.payouts(),
                elements_are![(
                    eq("https://a.example"),
                    eq(&period),
                    eq(&EuroCents(20))
                )]
            );
            expect_that!(admin.batch(period).await, none());
            expect_that!(admin.log().await, len(eq(3)));
        });
    }
}
