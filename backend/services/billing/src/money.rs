use std::fmt;
use std::iter::Sum;
use std::ops::Add;

use serde::{Deserialize, Serialize};

/// An amount in euro cents.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct EuroCents(pub u64);

impl EuroCents {
    pub const ZERO: EuroCents = EuroCents(0);
}

impl Add for EuroCents {
    type Output = EuroCents;

    fn add(self, rhs: Self) -> Self::Output {
        // u64::MAX cents is about 1.8e17 EUR, out of reach for any sum of charges.
        EuroCents(self.0.saturating_add(rhs.0))
    }
}

impl Sum for EuroCents {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(EuroCents::ZERO, EuroCents::add)
    }
}

impl fmt::Display for EuroCents {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EUR {}.{:02}", self.0 / 100, self.0 % 100)
    }
}

#[cfg(test)]
mod tests {
    use googletest::prelude::*;
    use rstest::rstest;

    use super::*;

    #[gtest]
    #[rstest]
    #[case::euros_and_cents(1005, "EUR 10.05")]
    #[case::cents_only(7, "EUR 0.07")]
    fn displays_euros_and_cents(#[case] cents: u64, #[case] shown: &str) {
        expect_that!(EuroCents(cents).to_string(), eq(shown));
    }

    #[gtest]
    #[rstest]
    fn sums() {
        let total: EuroCents = [EuroCents(10), EuroCents(15)].into_iter().sum();
        expect_that!(total, eq(EuroCents(25)));
    }
}
