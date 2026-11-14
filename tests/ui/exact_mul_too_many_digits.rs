use numo::{
  Decimal, ExactMul,
  typenum::{U0, U154},
};

fn product<A: ExactMul<B>, B>(lhs: &A, rhs: &B) -> A::Output {
  lhs.exact_mul(rhs)
}

fn main() {
  // A signed product of 308 digits has no storage, through the trait as through the inherent
  // method.
  let max = Decimal::<U154, U0>::MAX;
  let _product = product(&max, &max);
}
