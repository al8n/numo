use numo::{Decimal, typenum::U1};

type D = Decimal<U1, U1>;

// The operators take two values or two references, never a value and a reference.
fn main() {
  let (a, b) = (D::ZERO, D::MAX);
  let _ = a + &b;
  let _ = &a + b;
  let mut total = a;
  total += &b;
  let _ = total;
}
