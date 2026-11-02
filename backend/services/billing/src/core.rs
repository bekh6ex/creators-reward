//! Billing state: period keys, limits, the records of open periods, the
//! batches and charges of periods waiting for their ceremony, and the log.
//! In memory for now, like the rest of the backend.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use futures::lock::Mutex;
use rand::seq::SliceRandom;
use serde::Serialize;

use common::ViewerBillingId;

use crate::EuroCents;
use crate::ceremony_format::{Batch, BatchRecord, PeriodPublicKey, SealedUsage, Totals};
use crate::period::{Clock, PeriodId};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Pricing {
    /// What one unit of reading (one record) is worth before the limit.
    pub unit_price: EuroCents,
    /// Limit of readers who never set one.
    pub default_limit: EuroCents,
}

/// A site: the stand-in for a creator until creators register sites.
pub(crate) type CreatorSite = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Charge {
    pub amount: EuroCents,
    pub weight_millicents: u64,
}

/// Pure money arithmetic for one reader's period.
///
/// `units` is the number of sealed records. `amount` is capped at `limit`,
/// so reading never creates a debt. `weight_millicents` is the per-record
/// weight, rounded down, so `weight_millicents * units` is at most `amount * 1000`
/// and payouts never exceed charges. Saturating: proven panic-free in `proofs.rs`.
pub(crate) fn charge(units: u64, unit_price: EuroCents, limit: EuroCents) -> Charge {
    let raw = unit_price.0.saturating_mul(units).min(limit.0);
    let amount = EuroCents(raw);
    let weight_millicents = amount
        .0
        .saturating_mul(1000)
        .checked_div(units)
        .unwrap_or(0);
    Charge {
        amount,
        weight_millicents,
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Settlement {
    pub period: PeriodId,
    pub charges: Vec<(ViewerBillingId, EuroCents)>,
    pub payouts: Vec<(CreatorSite, EuroCents)>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum KeyRejected {
    #[error("period {0} already has another key")]
    Exists(PeriodId),
    #[error("period {0} is closed")]
    Closed(PeriodId),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(crate) enum UsageRejected {
    #[error("no key for period {0}")]
    NoKey(PeriodId),
    #[error("period {0} is closed")]
    Closed(PeriodId),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SettleError {
    #[error("period {0} is not waiting for totals")]
    NotAwaiting(PeriodId),
    #[error("totals pay out {paid} but readers were charged {charged}")]
    MoreThanCharged { paid: EuroCents, charged: EuroCents },
}

/// What happened to each period, for the key holders and, later, the public.
/// Never anything per reader or per site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "event",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum LogEntry {
    KeyAdded {
        at: u64,
        period: PeriodId,
        key_fingerprint: String,
    },
    Closed {
        at: u64,
        period: PeriodId,
        key_fingerprint: Option<String>,
        records: usize,
    },
    Settled {
        at: u64,
        period: PeriodId,
        charged_cents: EuroCents,
        paid_out_cents: EuroCents,
        dropped_records: u64,
    },
}

struct Awaiting {
    charges: Vec<(ViewerBillingId, EuroCents)>,
    batch: Batch,
}

#[derive(Default)]
struct State {
    keys: BTreeMap<PeriodId, PeriodPublicKey>,
    limits: HashMap<ViewerBillingId, EuroCents>,
    open: BTreeMap<PeriodId, HashMap<ViewerBillingId, Vec<SealedUsage>>>,
    awaiting_totals: BTreeMap<PeriodId, Awaiting>,
    done: BTreeSet<PeriodId>,
    log: Vec<LogEntry>,
}

impl State {
    fn is_closed(&self, period: PeriodId) -> bool {
        self.awaiting_totals.contains_key(&period) || self.done.contains(&period)
    }
}

#[derive(Clone)]
pub(crate) struct Billing {
    state: Arc<Mutex<State>>,
    clock: Clock,
}

impl Billing {
    pub(crate) fn new(clock: Clock) -> Self {
        Self {
            state: Arc::new(Mutex::new(State::default())),
            clock,
        }
    }

    /// Uploading the same key again is fine; replacing it is not, since
    /// records may already be sealed to it.
    pub(crate) async fn add_period_key(
        &self,
        period: PeriodId,
        key: PeriodPublicKey,
    ) -> Result<(), KeyRejected> {
        let mut state = self.state.lock().await;
        if state.is_closed(period) {
            return Err(KeyRejected::Closed(period));
        }
        let result = match state.keys.get(&period) {
            Some(existing) if *existing == key => Ok(()),
            Some(_) => Err(KeyRejected::Exists(period)),
            None => {
                state.keys.insert(period, key);
                state.log.push(LogEntry::KeyAdded {
                    at: (self.clock)(),
                    period,
                    key_fingerprint: key.fingerprint(),
                });
                Ok(())
            }
        };
        drop(state);
        result
    }

    pub(crate) async fn period_key(&self, period: PeriodId) -> Option<PeriodPublicKey> {
        self.state.lock().await.keys.get(&period).copied()
    }

    pub(crate) async fn set_limit(&self, viewer: ViewerBillingId, limit: EuroCents) {
        self.state
            .lock()
            .await
            .limits
            .insert(viewer, limit);
    }

    pub(crate) async fn record_usage(
        &self,
        viewer: ViewerBillingId,
        period: PeriodId,
        sealed: SealedUsage,
    ) -> Result<(), UsageRejected> {
        let mut state = self.state.lock().await;
        if state.is_closed(period) {
            return Err(UsageRejected::Closed(period));
        }
        if !state.keys.contains_key(&period) {
            return Err(UsageRejected::NoKey(period));
        }
        state
            .open
            .entry(period)
            .or_default()
            .entry(viewer)
            .or_default()
            .push(sealed);
        drop(state);
        Ok(())
    }

    /// Periods before `current` that still hold records or a key.
    pub(crate) async fn due(&self, current: PeriodId) -> Vec<PeriodId> {
        let state = self.state.lock().await;
        state
            .open
            .keys()
            .chain(state.keys.keys())
            .copied()
            .filter(|period| *period < current && !state.is_closed(*period))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// Closes the period and returns its batch: reader ids stripped, records
    /// shuffled. Afterwards billing keeps only each reader's charge and the
    /// batch, until the ceremony's totals arrive. `None` if already closed.
    pub(crate) async fn close(
        &self,
        period: PeriodId,
        pricing: Pricing,
    ) -> Option<Batch> {
        let batch = {
            let mut state = self.state.lock().await;
            if state.is_closed(period) {
                return None;
            }
            let key = state.keys.remove(&period);
            let readers = state.open.remove(&period).unwrap_or_default();

            let mut records = Vec::new();
            let mut charges = Vec::with_capacity(readers.len());
            for (viewer, sealed) in readers {
                let units = sealed.len() as u64;
                let limit = state
                    .limits
                    .get(&viewer)
                    .copied()
                    .unwrap_or(pricing.default_limit);
                // Reading decides the amount up to the limit; above it, every
                // share is scaled down proportionally via the weight.
                let Charge {
                    amount,
                    weight_millicents,
                } = charge(units, pricing.unit_price, limit);
                records.extend(
                    sealed
                        .iter()
                        .map(|record| BatchRecord::new(weight_millicents, record)),
                );
                charges.push((viewer, amount));
            }
            records.shuffle(&mut rand::thread_rng());
            let batch = Batch { period, records };
            state.log.push(LogEntry::Closed {
                at: (self.clock)(),
                period,
                key_fingerprint: key.map(|key| key.fingerprint()),
                records: batch.records.len(),
            });
            if batch.records.is_empty() {
                // Nothing to open: no ceremony needed, nobody to charge.
                state.done.insert(period);
            } else {
                let awaiting = Awaiting {
                    charges,
                    batch: batch.clone(),
                };
                state.awaiting_totals.insert(period, awaiting);
            }
            batch
        };
        Some(batch)
    }

    /// The batch of a period waiting for its ceremony.
    pub(crate) async fn batch(&self, period: PeriodId) -> Option<Batch> {
        let state = self.state.lock().await;
        state
            .awaiting_totals
            .get(&period)
            .map(|awaiting| awaiting.batch.clone())
    }

    pub(crate) async fn log(&self) -> Vec<LogEntry> {
        self.state.lock().await.log.clone()
    }

    /// Turns the ceremony's totals into charges and payouts, and forgets the
    /// batch. Payouts are rounded down to the cent; the remainder stays with
    /// the platform.
    pub(crate) async fn settle(
        &self,
        totals: &Totals,
    ) -> Result<Settlement, SettleError> {
        let settlement = {
            let mut state = self.state.lock().await;
            let charges = &state
                .awaiting_totals
                .get(&totals.period)
                .ok_or(SettleError::NotAwaiting(totals.period))?
                .charges;
            let payouts: Vec<_> = totals
                .per_site_millicents
                .iter()
                .map(|(site, millicents)| (site.clone(), EuroCents(millicents / 1000)))
                .filter(|(_, amount)| amount.0 > 0)
                .collect();
            let charged: EuroCents = charges.iter().map(|(_, a)| *a).sum();
            let paid: EuroCents = payouts.iter().map(|(_, a)| *a).sum();
            if paid > charged {
                return Err(SettleError::MoreThanCharged { paid, charged });
            }
            let charges = state
                .awaiting_totals
                .remove(&totals.period)
                .map(|awaiting| awaiting.charges)
                .unwrap_or_default();
            state.done.insert(totals.period);
            state.log.push(LogEntry::Settled {
                at: (self.clock)(),
                period: totals.period,
                charged_cents: charged,
                paid_out_cents: paid,
                dropped_records: totals.dropped,
            });
            let settlement = Settlement {
                period: totals.period,
                charges,
                payouts,
            };
            drop(state);
            settlement
        };
        Ok(settlement)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use futures::executor::block_on;
    use googletest::prelude::*;
    use rstest::{fixture, rstest};

    use crate::ceremony_format::SEALED_USAGE_LEN;

    use super::*;
    use crate::period::now_unix_sec;

    const P: PeriodId = PeriodId::new(681);
    const PRICING: Pricing = Pricing {
        unit_price: EuroCents(10),
        default_limit: EuroCents(1000),
    };

    fn record(byte: u8) -> SealedUsage {
        SealedUsage::try_from(vec![byte; SEALED_USAGE_LEN]).unwrap()
    }

    /// Billing that accepts records for `P`.
    #[fixture]
    fn billing() -> Billing {
        let billing = Billing::new(Arc::new(now_unix_sec));
        block_on(billing.add_period_key(P, PeriodPublicKey::from_bytes([1; 32])))
            .unwrap();
        billing
    }

    async fn read(billing: &Billing, viewer: ViewerBillingId, byte: u8, times: usize) {
        for _ in 0..times {
            billing
                .record_usage(viewer, P, record(byte))
                .await
                .unwrap();
        }
    }

    #[gtest]
    #[rstest]
    #[case::zero_limit(4, 10, 0, 0, 0)]
    #[case::no_units(0, 10, 1000, 0, 0)]
    #[case::under_limit(3, 10, 1000, 30, 10_000)]
    #[case::exactly_at_limit(100, 10, 1000, 1000, 10_000)]
    #[case::huge_units(u64::MAX, 10, 1000, 1000, 0)]
    fn charge_computes_amount_and_weight(
        #[case] units: u64,
        #[case] unit_price: u64,
        #[case] limit: u64,
        #[case] expected_amount: u64,
        #[case] expected_weight: u64,
    ) {
        let got = charge(units, EuroCents(unit_price), EuroCents(limit));
        expect_that!(got.amount, eq(EuroCents(expected_amount)));
        expect_that!(got.weight_millicents, eq(expected_weight));
        expect_that!(got.amount, le(EuroCents(limit)));
        expect_that!(
            got.weight_millicents.saturating_mul(units),
            le(got.amount.0.saturating_mul(1000))
        );
    }

    #[gtest]
    #[rstest]
    fn records_need_the_period_key() {
        block_on(async {
            let billing = Billing::new(Arc::new(now_unix_sec));
            let result = billing
                .record_usage(ViewerBillingId::new(), P, record(1))
                .await;
            expect_that!(result, err(eq(&UsageRejected::NoKey(P))));
        });
    }

    #[gtest]
    #[rstest]
    fn reading_decides_the_amount_up_to_the_limit(billing: Billing) {
        block_on(async {
            let light = ViewerBillingId::new();
            let heavy = ViewerBillingId::new();
            billing.set_limit(heavy, EuroCents(50)).await;
            read(&billing, light, 1, 3).await; // 30 cents, under the default limit
            read(&billing, heavy, 2, 20).await; // 200 cents, scaled down to 50

            let batch = billing.close(P, PRICING).await.unwrap();

            expect_that!(batch.period, eq(P));
            expect_that!(batch.records, len(eq(23)));
            let weights: HashSet<_> = batch
                .records
                .iter()
                .map(|r| r.weight_millicents)
                .collect();
            expect_that!(
                weights,
                unordered_elements_are![eq(&10_000), eq(&2_500)]
            );
            let total: u64 = batch
                .records
                .iter()
                .map(|r| r.weight_millicents)
                .sum();
            expect_that!(total, eq(80_000));
            expect_that!(
                billing.batch(P).await,
                some(field!(&Batch.records, ref len(eq(23))))
            );
        });
    }

    #[gtest]
    #[rstest]
    fn a_key_can_be_uploaded_again_but_not_replaced(billing: Billing) {
        block_on(async {
            let same = PeriodPublicKey::from_bytes([1; 32]);
            let other = PeriodPublicKey::from_bytes([2; 32]);
            expect_that!(billing.add_period_key(P, same).await, ok(anything()));
            expect_that!(
                billing.add_period_key(P, other).await,
                err(eq(&KeyRejected::Exists(P)))
            );
            billing.close(P, PRICING).await.unwrap();
            expect_that!(
                billing.add_period_key(P, same).await,
                err(eq(&KeyRejected::Closed(P)))
            );
            expect_that!(
                billing.log().await,
                elements_are![
                    pat!(LogEntry::KeyAdded { .. }),
                    pat!(LogEntry::Closed {
                        records: eq(&0),
                        ..
                    }),
                ]
            );
        });
    }

    #[gtest]
    #[rstest]
    fn a_zero_limit_produces_the_same_records_with_weight_zero(billing: Billing) {
        block_on(async {
            let viewer = ViewerBillingId::new();
            billing.set_limit(viewer, EuroCents(0)).await;
            read(&billing, viewer, 1, 4).await;

            let batch = billing.close(P, PRICING).await.unwrap();

            expect_that!(
                batch.records,
                all![
                    len(eq(4)),
                    each(field!(&BatchRecord.weight_millicents, eq(0)))
                ]
            );
            let totals = Totals {
                period: P,
                per_site_millicents: BTreeMap::new(),
                records: 4,
                dropped: 0,
            };
            let settlement = billing.settle(&totals).await.unwrap();
            expect_that!(
                settlement.charges,
                elements_are![(eq(&viewer), eq(&EuroCents(0)))]
            );
        });
    }

    #[gtest]
    #[rstest]
    fn closing_strips_readers_and_refuses_late_records(billing: Billing) {
        block_on(async {
            let viewer = ViewerBillingId::new();
            read(&billing, viewer, 1, 1).await;

            expect_that!(billing.due(P).await, is_empty());
            expect_that!(billing.due(P.next()).await, elements_are![eq(&P)]);
            let batch = billing.close(P, PRICING).await.unwrap();
            let json = serde_json::to_string(&batch).unwrap();
            let viewer_json = serde_json::to_string(&viewer).unwrap();
            expect_that!(
                json,
                not(contains_substring(viewer_json.trim_matches('"')))
            );

            expect_that!(billing.close(P, PRICING).await, none());
            expect_that!(billing.due(P.next()).await, is_empty());
            expect_that!(
                billing.record_usage(viewer, P, record(1)).await,
                err(eq(&UsageRejected::Closed(P)))
            );
            expect_that!(billing.period_key(P).await, none());
        });
    }

    #[gtest]
    #[rstest]
    fn a_period_without_records_needs_no_ceremony(billing: Billing) {
        block_on(async {
            expect_that!(billing.due(P.next()).await, elements_are![eq(&P)]);
            let batch = billing.close(P, PRICING).await.unwrap();
            expect_that!(batch.records, is_empty());
            expect_that!(billing.batch(P).await, none());
            expect_that!(billing.due(P.next()).await, is_empty());
        });
    }

    #[gtest]
    #[rstest]
    fn settling_pays_creators_and_charges_readers_once(billing: Billing) {
        block_on(async {
            let viewer = ViewerBillingId::new();
            read(&billing, viewer, 1, 3).await;
            billing.close(P, PRICING).await.unwrap();
            let totals = Totals {
                period: P,
                per_site_millicents: BTreeMap::from([
                    ("https://a.example".to_string(), 20_000),
                    ("https://b.example".to_string(), 9_999),
                    ("https://c.example".to_string(), 999),
                ]),
                records: 3,
                dropped: 0,
            };

            let settlement = billing.settle(&totals).await.unwrap();

            expect_that!(
                settlement.charges,
                elements_are![(eq(&viewer), eq(&EuroCents(30)))]
            );
            expect_that!(
                settlement.payouts,
                elements_are![
                    (eq("https://a.example"), eq(&EuroCents(20))),
                    (eq("https://b.example"), eq(&EuroCents(9))),
                ]
            );
            expect_that!(
                billing.settle(&totals).await,
                err(eq(&SettleError::NotAwaiting(P)))
            );
            expect_that!(billing.batch(P).await, none());
            expect_that!(
                billing.log().await.last(),
                some(pat!(LogEntry::Settled {
                    charged_cents: eq(&EuroCents(30)),
                    paid_out_cents: eq(&EuroCents(29)),
                    ..
                }))
            );
        });
    }

    #[gtest]
    #[rstest]
    fn totals_cannot_pay_out_more_than_was_charged(billing: Billing) {
        block_on(async {
            read(&billing, ViewerBillingId::new(), 1, 1).await;
            billing.close(P, PRICING).await.unwrap();
            let totals = Totals {
                period: P,
                per_site_millicents: BTreeMap::from([(
                    "https://a.example".into(),
                    11_000,
                )]),
                records: 1,
                dropped: 0,
            };
            expect_that!(
                billing.settle(&totals).await,
                err(eq(&SettleError::MoreThanCharged {
                    paid: EuroCents(11),
                    charged: EuroCents(10)
                }))
            );
            expect_that!(billing.batch(P).await, some(anything()));
        });
    }
}
