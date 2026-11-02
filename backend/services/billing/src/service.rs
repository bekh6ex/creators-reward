use common::ViewerBillingId;

use crate::EuroCents;
use crate::ceremony_format::PeriodPublicKey;
use crate::core::Billing;
use crate::period::{Clock, PeriodId, PeriodLength};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("no key has been uploaded for the current period")]
pub struct CurrentPeriodKeyError;

#[cfg(test)]
mod tests {
    use std::num::NonZeroU64;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};

    use common::ViewerBillingId;
    use googletest::prelude::*;
    use rstest::rstest;

    use crate::EuroCents;
    use crate::ceremony_format::{BatchRecord, SEALED_USAGE_LEN, SealedUsage};
    use crate::core::Pricing;
    use crate::period::Clock;
    use crate::period::PeriodLength;
    use crate::wiring::{BillingConfig, create_wiring};

    use super::*;

    #[gtest]
    #[rstest]
    fn current_period_key_returns_key_when_present() {
        futures::executor::block_on(async {
            let length = PeriodLength::Seconds(NonZeroU64::new(60).unwrap());
            let period = PeriodId::new(10);
            let clock = Arc::new(AtomicU64::new(
                length.start(period).checked_add(1).unwrap(),
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
            let key = PeriodPublicKey::from_bytes([1; 32]);
            wiring
                .billing
                .add_period_key(period, key)
                .await
                .unwrap();

            let public = wiring.public_service();
            let result = public.current_period_key().await;

            expect_that!(result, pat!(Ok((eq(&period), eq(&key)))));
        });
    }

    #[gtest]
    #[rstest]
    fn set_monthly_limit_affects_period_charges() {
        futures::executor::block_on(async {
            let length = PeriodLength::Seconds(NonZeroU64::new(60).unwrap());
            let period = PeriodId::new(10);
            let clock = Arc::new(AtomicU64::new(
                length.start(period).checked_add(1).unwrap(),
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
            let key = PeriodPublicKey::from_bytes([1; 32]);
            wiring
                .billing
                .add_period_key(period, key)
                .await
                .unwrap();

            let viewer = ViewerBillingId::new();
            let public = wiring.public_service();
            public
                .set_monthly_limit(viewer, EuroCents(0))
                .await;

            let sealed = SealedUsage::try_from(vec![1u8; SEALED_USAGE_LEN]).unwrap();
            wiring
                .billing
                .record_usage(viewer, period, sealed)
                .await
                .unwrap();

            let batch = wiring
                .billing
                .close(
                    period,
                    Pricing {
                        unit_price: EuroCents(10),
                        default_limit: EuroCents(1000),
                    },
                )
                .await
                .unwrap();

            expect_that!(
                batch.records,
                each(field!(&BatchRecord.weight_millicents, eq(0)))
            );
        });
    }
}

/// What gateways may call.
#[derive(Clone)]
pub struct PublicService {
    billing: Billing,
    period_length: PeriodLength,
    clock: Clock,
}

impl PublicService {
    pub(crate) fn new(
        billing: Billing,
        period_length: PeriodLength,
        clock: Clock,
    ) -> Self {
        PublicService {
            billing,
            period_length,
            clock,
        }
    }

    /// The key the iframe seals this period's records to. The same for every
    /// reader, so it can be served without knowing who asks.
    pub async fn current_period_key(
        &self,
    ) -> Result<(PeriodId, PeriodPublicKey), CurrentPeriodKeyError> {
        let period = self.period_length.period_at((self.clock)());
        let key = self
            .billing
            .period_key(period)
            .await
            .ok_or(CurrentPeriodKeyError)?;
        Ok((period, key))
    }

    /// Not routed yet: the viewer has no limit setting.
    pub async fn set_monthly_limit(&self, viewer: ViewerBillingId, limit: EuroCents) {
        self.billing.set_limit(viewer, limit).await
    }
}
