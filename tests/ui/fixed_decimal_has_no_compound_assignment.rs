use numo::{ConstFixedDecimal, FixedDecimal};

// `FixedDecimal` has the operators on values but not their compound assignments, which
// `DecimalAssign` adds for a storage that holds the value in place.
fn accumulate<T: FixedDecimal>(mut total: T, value: T) -> T {
  total += value;
  total
}

// `ConstFixedDecimal` has them through `DecimalAssign`, the smaller bound that gives them.
fn accumulate_in_place<T: ConstFixedDecimal>(mut total: T, value: T) -> T {
  total += value;
  total
}

fn main() {}
