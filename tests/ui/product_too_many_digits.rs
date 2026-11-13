use numo::{
  Decimal,
  typenum::{U0, U154},
};

fn main() {
  let lhs = Decimal::<U154, U0>::MAX;
  let rhs = Decimal::<U154, U0>::MAX;
  let _product = lhs.exact_mul(&rhs);
}
