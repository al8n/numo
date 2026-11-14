use numo::{
  Decimal,
  typenum::{U0, U154, U155},
  u512, u576,
};

fn main() {
  // Two unsigned storages have a product of up to 308 digits, and not of 309.
  let lhs = Decimal::<U155, U0, u576>::MAX;
  let rhs = Decimal::<U154, U0, u512>::MAX;
  let _product = lhs.exact_mul(&rhs);
}
