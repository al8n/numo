use numo::{
  Decimal,
  typenum::{U0, U77},
  u256,
};

fn main() {
  // Two unsigned storages still need a `DigitCount`, which stops at 153 digits.
  let lhs = Decimal::<U77, U0, u256>::MAX;
  let rhs = Decimal::<U77, U0, u256>::MAX;
  let _product = lhs.exact_mul(&rhs);
}
