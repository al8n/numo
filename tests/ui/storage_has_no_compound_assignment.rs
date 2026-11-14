use numo::{Decimal, FixedDecimal, MutableStorage, Storage};

// `+=` stores into the storage of its target, so it needs a storage that holds the coefficient in
// place, which `Storage` alone does not promise. The bound on the decimal of the inline twin is
// that of the operator, and holds.
fn accumulate<I, F, R>(mut total: Decimal<I, F, R>, value: Decimal<I, F, R>) -> Decimal<I, F, R>
where
  R: Storage,
  Decimal<I, F, R::Inline>: FixedDecimal,
{
  total += value;
  total
}

// The fix: `MutableStorage`, or `InlineStorage`, which implies it.
fn accumulate_in_place<I, F, R>(
  mut total: Decimal<I, F, R>,
  value: Decimal<I, F, R>,
) -> Decimal<I, F, R>
where
  R: MutableStorage,
  Decimal<I, F, R::Inline>: FixedDecimal,
{
  total += value;
  total
}

fn main() {}
