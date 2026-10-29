use numo::{
  Decimal,
  typenum::{U0, U77},
};

fn main() {
  let lhs = Decimal::<U77, U0>::MAX;
  let rhs = Decimal::<U77, U0>::MAX;
  let _product = lhs.exact_mul(&rhs);
}
