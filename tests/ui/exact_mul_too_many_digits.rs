use numo::{
  Decimal, ExactMul,
  typenum::{U0, U77},
};

fn product<A: ExactMul<B>, B>(lhs: &A, rhs: &B) -> A::Output {
  lhs.exact_mul(rhs)
}

fn main() {
  // A product of 154 digits has no storage, through the trait as through the inherent method.
  let max = Decimal::<U77, U0>::MAX;
  let _product = product(&max, &max);
}
