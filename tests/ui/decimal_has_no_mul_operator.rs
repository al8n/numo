use numo::{Decimal, typenum::U1};

type D = Decimal<U1, U1>;

fn main() {
  // A product has the fraction digits of both operands, so no `*` gives it in their type:
  // `exact_mul` returns it in the type that holds it.
  let _product = D::MAX * D::MAX;
}
