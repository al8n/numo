use numo::{Decimal, FixedDecimal, typenum::U10};

fn zero<T: FixedDecimal>() -> T {
  T::zero()
}

fn main() {
  // 20 digits do not fit in an `i64`, which holds 18.
  let _zero = zero::<Decimal<U10, U10, i64>>();
}
