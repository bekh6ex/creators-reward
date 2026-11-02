use crate::EuroCents;
use crate::core::charge;

// Ranges: unit price <= 10 000 EUR/read, limit <= 10 billion EUR.
// Real values are default 0.10 EUR/read and 10 EUR limit, so these bounds
// are absurdly loose while keeping all intermediate products far below u64::MAX.
const MAX_UNIT_PRICE_CENTS: u64 = 1_000_000;
const MAX_LIMIT_CENTS: u64 = 1_000_000_000_000;

#[kani::proof]
fn charge_amount_never_exceeds_limit() {
    let units: u64 = kani::any();
    let unit_price = EuroCents(kani::any());
    let limit = EuroCents(kani::any());
    kani::assume(unit_price.0 <= MAX_UNIT_PRICE_CENTS);
    kani::assume(limit.0 <= MAX_LIMIT_CENTS);

    let result = charge(units, unit_price, limit);

    assert!(result.amount.0 <= limit.0);
}

#[kani::proof]
fn charge_weight_times_units_fits_amount() {
    let units: u64 = kani::any();
    let unit_price = EuroCents(kani::any());
    let limit = EuroCents(kani::any());
    kani::assume(unit_price.0 <= MAX_UNIT_PRICE_CENTS);
    kani::assume(limit.0 <= MAX_LIMIT_CENTS);

    let result = charge(units, unit_price, limit);

    // amount.0 <= limit.0 <= MAX_LIMIT_CENTS, so amount.0 * 1000 is far below
    // u64::MAX and the multiplication in this assertion cannot overflow.
    assert!(result.weight_millicents * units <= result.amount.0 * 1000);
}

#[kani::proof]
fn charge_never_panics() {
    let units: u64 = kani::any();
    let unit_price = EuroCents(kani::any());
    let limit = EuroCents(kani::any());

    let _ = charge(units, unit_price, limit);
}
