use numo::{Decimal, typenum::U1};

type D = Decimal<U1, U1>;

fn main() {
  // `Sum` is implemented, and `Product` is not: a product of two values has the fraction digits
  // of both, so it does not stay in their type without rounding.
  let _sum: D = [D::ZERO; 2].into_iter().sum();
  let _product: D = [D::MAX; 2].into_iter().product();
}
