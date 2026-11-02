use std::fmt;
use std::future::Future;
use std::sync::Arc;

use futures::{Stream, StreamExt};

use common::billing_usage::UsageRecorded;
use common::{ViewerBillingId, ViewerEventsId};
use id_storage::internal_api::{IdExchange, IdExchanger};

use crate::EuroCents;
use crate::admin_service::AdminService;
use crate::ceremony_format::SealedUsage;
use crate::core::{Billing, Pricing};
use crate::payments::PaymentProvider;
use crate::period::{Clock, PeriodId, PeriodLength, now_unix_sec};
use crate::service::PublicService;

#[derive(Clone)]
pub struct BillingConfig {
    pub period_length: PeriodLength,
    pub unit_price: EuroCents,
    pub default_limit: EuroCents,
    pub clock: Clock,
}

impl fmt::Debug for BillingConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BillingConfig")
            .field("period_length", &self.period_length)
            .field("unit_price", &self.unit_price)
            .field("default_limit", &self.default_limit)
            .field("clock", &"<dyn Fn() -> u64>")
            .finish()
    }
}

impl Default for BillingConfig {
    fn default() -> Self {
        BillingConfig {
            period_length: PeriodLength::CalendarMonth,
            unit_price: EuroCents(10),
            default_limit: EuroCents(1000),
            clock: Arc::new(now_unix_sec),
        }
    }
}

/// Billing as seen by the composition root (`state.rs`).
pub struct Wiring {
    pub(crate) billing: Billing,
    config: BillingConfig,
}

pub fn create_wiring(config: BillingConfig) -> Wiring {
    Wiring {
        billing: Billing::new(config.clock.clone()),
        config,
    }
}

impl Wiring {
    /// For the viewer gateway.
    pub fn public_service(&self) -> PublicService {
        PublicService::new(
            self.billing.clone(),
            self.config.period_length,
            self.config.clock.clone(),
        )
    }

    /// For the admin gateway only.
    pub fn admin_service(&self, payments: Arc<dyn PaymentProvider>) -> AdminService {
        AdminService::new(
            self.billing.clone(),
            self.config.period_length,
            payments,
            self.config.clock.clone(),
        )
    }

    /// Stores the events service's [`UsageRecorded`] under the reader's
    /// billing id. The events id is used only for that translation.
    pub fn usage_listener<S>(
        &self,
        exchanger: IdExchanger<ViewerEventsId, ViewerBillingId>,
        stream: S,
    ) -> impl Future<Output = ()> + use<S>
    where
        S: Stream<Item = UsageRecorded>,
    {
        let billing = self.billing.clone();
        stream.for_each(move |usage| {
            let billing = billing.clone();
            let exchanger = exchanger.clone();
            async move {
                let Some(viewer) = exchanger.get_secondary(usage.viewer_id).await else {
                    tracing::error!("billing: usage from an unknown viewer dropped");
                    return;
                };
                let Ok(sealed) = SealedUsage::try_from(usage.sealed) else {
                    tracing::error!("billing: malformed usage record dropped");
                    return;
                };
                let period = PeriodId::new(usage.period);
                if let Err(err) = billing.record_usage(viewer, period, sealed).await {
                    tracing::info!("billing: usage record dropped: {err}");
                }
            }
        })
    }

    pub fn period_closer(&self) -> PeriodCloser {
        PeriodCloser {
            billing: self.billing.clone(),
            period_length: self.config.period_length,
            pricing: Pricing {
                unit_price: self.config.unit_price,
                default_limit: self.config.default_limit,
            },
            clock: self.config.clock.clone(),
        }
    }
}

/// Closes ended periods into batches for the ceremony. Call [`tick`] often;
/// the composition root does it every second.
///
/// [`tick`]: PeriodCloser::tick
pub struct PeriodCloser {
    billing: Billing,
    period_length: PeriodLength,
    pricing: Pricing,
    clock: Clock,
}

impl PeriodCloser {
    pub async fn tick(&self) {
        let current = self.period_length.period_at((self.clock)());
        for period in self.billing.due(current).await {
            if let Some(batch) = self.billing.close(period, self.pricing).await {
                tracing::info!(
                    "billing: closed period {} with {} records",
                    self.period_length.label(period),
                    batch.records.len()
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU64;
    use std::sync::atomic::{AtomicU64, Ordering};

    use crate::ceremony_format::PeriodPublicKey;

    use googletest::prelude::*;
    use rstest::rstest;

    use super::*;

    #[gtest]
    #[rstest]
    fn closes_periods_once_they_end() {
        futures::executor::block_on(async {
            let length = PeriodLength::Seconds(NonZeroU64::new(60).unwrap());
            let period = PeriodId::new(10);
            let clock = Arc::new(AtomicU64::new(
                length.end(period).checked_sub(1).unwrap(),
            ));
            let clock_fn: Clock = {
                let clock = clock.clone();
                Arc::new(move || clock.load(Ordering::SeqCst))
            };
            let wiring = create_wiring(BillingConfig {
                period_length: length,
                clock: clock_fn,
                ..BillingConfig::default()
            });
            wiring
                .billing
                .add_period_key(period, PeriodPublicKey::from_bytes([1; 32]))
                .await
                .unwrap();
            let closer = wiring.period_closer();

            closer.tick().await;
            expect_that!(
                wiring.billing.period_key(period).await,
                some(anything())
            );
            clock.store(length.end(period), Ordering::SeqCst);
            closer.tick().await;
            expect_that!(wiring.billing.period_key(period).await, none());
            expect_that!(wiring.billing.log().await, len(eq(2)));
        });
    }
}
