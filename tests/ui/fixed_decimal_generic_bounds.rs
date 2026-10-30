use numo::{Decimal, FixedDecimal, Layout, Storage};

// `FixedDecimal` is implemented for each storage, so the bounds of the inherent items do not give
// `Decimal<I, F, R>` its methods.
fn is_zero<I, F, R>(value: &Decimal<I, F, R>) -> bool
where
  R: Storage,
  (I, F): Layout<R>,
{
  value.is_zero()
}

// The bound `Decimal<I, F, R>: FixedDecimal` gives them.
fn is_zero_with_bound<I, F, R>(value: &Decimal<I, F, R>) -> bool
where
  Decimal<I, F, R>: FixedDecimal,
{
  value.is_zero()
}

fn main() {}
