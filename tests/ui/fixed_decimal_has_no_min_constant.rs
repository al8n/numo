use numo::{ConstFixedDecimal, FixedDecimal};

// `FixedDecimal` has the smallest value as the function `min_value`, not as a constant, and zero
// and the largest value likewise.
fn smallest<T: FixedDecimal>() -> T {
  T::MIN
}

// The function returns the smallest value.
fn smallest_value<T: FixedDecimal>() -> T {
  T::min_value()
}

// `ConstFixedDecimal` has the constant, also in a `const fn`.
const fn smallest_constant<T: ConstFixedDecimal>() -> T {
  T::MIN
}

fn main() {}

// `MIN` rather than `ZERO`: the help lists each trait of the dependencies that defines the item,
// and optional dependencies define `ZERO`, so the list would change with the features.
