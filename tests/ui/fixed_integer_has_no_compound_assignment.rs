use numo::{FixedInteger, IntegerAssign};

// `FixedInteger` has the operators on values but not their compound assignments, which
// `IntegerAssign` adds for a storage that holds the value in place.
fn accumulate<T: FixedInteger>(mut total: T, value: T) -> T {
  total += value;
  total
}

// `IntegerAssign` has them.
fn accumulate_in_place<T: IntegerAssign>(mut total: T, value: T) -> T {
  total += value;
  total
}

fn main() {}
