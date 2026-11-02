//! Billing periods: calendar months (UTC) by default, or fixed-length periods
//! counted from the Unix epoch, which tests and the dev stack use to see a
//! period close within seconds.

use std::fmt;
use std::num::NonZeroU64;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use time::{Date, Month, OffsetDateTime};

/// Index of a period: months since 1970-01 for calendar months, otherwise
/// `unix seconds / period length`. Meaningful only with its [`PeriodLength`].
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct PeriodId(u64);

impl PeriodId {
    pub const fn new(index: u64) -> Self {
        PeriodId(index)
    }

    pub fn get(self) -> u64 {
        self.0
    }

    pub fn next(self) -> Self {
        // One period per second would need ~584 billion years to overflow u64.
        PeriodId(self.0.saturating_add(1))
    }
}

impl fmt::Display for PeriodId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PeriodLength {
    #[default]
    CalendarMonth,
    Seconds(NonZeroU64),
}

#[derive(Debug, thiserror::Error)]
#[error(
    "period length must be \"month\" or a positive number of seconds like \"120s\", got {0:?}"
)]
pub struct InvalidPeriodLength(String);

impl FromStr for PeriodLength {
    type Err = InvalidPeriodLength;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "month" {
            return Ok(PeriodLength::CalendarMonth);
        }
        s.strip_suffix('s')
            .and_then(|n| n.parse::<u64>().ok())
            .and_then(NonZeroU64::new)
            .map(PeriodLength::Seconds)
            .ok_or_else(|| InvalidPeriodLength(s.to_string()))
    }
}

impl PeriodLength {
    /// # Panics
    ///
    /// For calendar months, if `unix_sec` is after the year 9999.
    #[expect(
        clippy::expect_used,
        reason = "documented panic: `time` represents no date after the year 9999"
    )]
    pub fn period_at(self, unix_sec: u64) -> PeriodId {
        match self {
            PeriodLength::CalendarMonth => {
                let date = i64::try_from(unix_sec)
                    .ok()
                    .and_then(|sec| OffsetDateTime::from_unix_timestamp(sec).ok())
                    .expect("a time before the year 10000")
                    .date();
                // A u64 timestamp is in 1970..=9999, so nothing saturates.
                let years = u64::from(date.year().unsigned_abs()).saturating_sub(1970);
                let month0 = u64::from(u8::from(date.month())).saturating_sub(1);
                PeriodId(years.saturating_mul(12).saturating_add(month0))
            }
            PeriodLength::Seconds(len) => PeriodId(unix_sec / len),
        }
    }

    pub fn current(self) -> PeriodId {
        self.period_at(now_unix_sec())
    }

    /// First second of the period.
    ///
    /// # Panics
    ///
    /// For calendar months, if the period starts after the year 9999.
    #[expect(
        clippy::expect_used,
        reason = "documented panic: `time` represents no date after the year 9999"
    )]
    pub fn start(self, period: PeriodId) -> u64 {
        match self {
            PeriodLength::CalendarMonth => {
                let month = Month::January.nth_next((period.0 % 12) as u8);
                let first_day = i32::try_from((period.0 / 12).saturating_add(1970))
                    .ok()
                    .and_then(|year| Date::from_calendar_date(year, month, 1).ok())
                    .expect("a billing period before the year 10000");
                first_day.midnight().assume_utc().unix_timestamp() as u64
            }
            PeriodLength::Seconds(len) => {
                // period.0 == unix_sec / len, so this product is <= unix_sec + len
                // and stays far below u64::MAX for any realistic timestamp.
                period.0.saturating_mul(len.get())
            }
        }
    }

    /// First second after the period.
    pub fn end(self, period: PeriodId) -> u64 {
        self.start(period.next())
    }

    /// For people: `2026-10` for months, the index otherwise.
    pub fn label(self, period: PeriodId) -> String {
        match self {
            PeriodLength::CalendarMonth => format!(
                "{}-{:02}",
                (period.0 / 12).saturating_add(1970),
                (period.0 % 12).saturating_add(1)
            ),
            PeriodLength::Seconds(len) => format!("{}s#{}", len, period.0),
        }
    }
}

/// Where billing reads the time (Unix seconds): `now_unix_sec` in production,
/// a clock the test advances in tests.
pub type Clock = std::sync::Arc<dyn Fn() -> u64 + Send + Sync>;

#[expect(
    clippy::disallowed_methods,
    reason = "period boundaries and the admin log (key added, closed, settled); never a per-reader timestamp"
)]
#[expect(
    clippy::expect_used,
    reason = "the system clock is after the Unix epoch"
)]
pub fn now_unix_sec() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after 1970")
        .as_secs()
}

#[cfg(test)]
mod tests {
    use googletest::prelude::*;
    use rstest::rstest;

    use super::*;

    const MONTH: PeriodLength = PeriodLength::CalendarMonth;

    // Reference values from Python's calendar.timegm.
    #[gtest]
    #[rstest]
    #[case::epoch(0, 0, "1970-01")]
    #[case::before_y2k(944_006_400, 359, "1999-12")]
    #[case::after_a_leap_february(951_868_800, 362, "2000-03")]
    #[case::leap_february(1_706_745_600, 649, "2024-02")]
    #[case::after_leap_day(1_709_251_200, 650, "2024-03")]
    #[case::recent(1_790_812_800, 681, "2026-10")]
    #[case::new_year(1_798_761_600, 684, "2027-01")]
    fn calendar_months_match_utc_dates(
        #[case] start: u64,
        #[case] index: u64,
        #[case] label: &str,
    ) {
        let period = PeriodId::new(index);
        expect_that!(MONTH.start(period), eq(start));
        expect_that!(MONTH.period_at(start), eq(period));
        expect_that!(MONTH.label(period), eq(label));
    }

    #[gtest]
    #[rstest]
    fn last_second_belongs_to_the_month() {
        // 2024-02-29T23:59:59Z, a leap day.
        expect_that!(MONTH.period_at(1_709_251_199), eq(PeriodId::new(649)));
        expect_that!(MONTH.end(PeriodId::new(649)), eq(1_709_251_200));
        // 2026-10-28T13:00:00Z
        expect_that!(MONTH.period_at(1_793_192_400), eq(PeriodId::new(681)));
    }

    #[gtest]
    #[rstest]
    fn fixed_periods_count_from_the_epoch() {
        let len: PeriodLength = "120s".parse().unwrap();
        expect_that!(len.period_at(239), eq(PeriodId::new(1)));
        expect_that!(len.period_at(240), eq(PeriodId::new(2)));
        expect_that!(len.start(PeriodId::new(2)), eq(240));
        expect_that!(len.end(PeriodId::new(2)), eq(360));
    }

    #[gtest]
    #[rstest]
    fn parses_a_calendar_month() {
        expect_that!("month".parse::<PeriodLength>(), ok(eq(&MONTH)));
    }

    #[gtest]
    #[rstest]
    fn period_id_get_returns_the_index() {
        expect_that!(PeriodId::new(681).get(), eq(681));
    }

    #[gtest]
    #[rstest]
    #[case::empty("")]
    #[case::zero_seconds("0s")]
    #[case::no_unit("12")]
    #[case::no_number("s")]
    #[case::negative("-5s")]
    #[case::unknown_word("monthly")]
    fn refuses_malformed_lengths(#[case] length: &str) {
        expect_that!(length.parse::<PeriodLength>(), err(anything()));
    }
}
