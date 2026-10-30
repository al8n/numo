use numo::{ConstFixedInteger, FixedInteger};

// `FixedInteger` does not require `Copy`, so the second use of `value` is a use after a move.
fn twice<T: FixedInteger>(value: T) -> T {
  value + value
}

// A clone keeps the value.
fn twice_clone<T: FixedInteger>(value: T) -> T {
  value.clone() + value
}

// `ConstFixedInteger` requires `Copy`.
fn twice_copy<T: ConstFixedInteger>(value: T) -> T {
  value + value
}

fn main() {}
