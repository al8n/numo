use numo::{ConstFixedDecimal, FixedDecimal};

// `FixedDecimal` does not require `Copy`, so `repr`, which takes the value, moves it.
fn repr_and_value<T: FixedDecimal>(value: T) -> (T::Repr, T) {
  (value.repr(), value)
}

// A clone keeps the value.
fn repr_and_clone<T: FixedDecimal>(value: T) -> (T::Repr, T) {
  (value.clone().repr(), value)
}

// `ConstFixedDecimal` requires `Copy`.
fn repr_and_copy<T: ConstFixedDecimal>(value: T) -> (T::Repr, T) {
  (value.repr(), value)
}

fn main() {}
