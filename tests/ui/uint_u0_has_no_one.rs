use numo::u0;

fn main() {
  // Zero bits hold only zero, so `Uint` of zero bits has no `ONE`, and no `Product`, which
  // multiplies from one.
  let _one = u0::ONE;
  let _product: u0 = [u0::ZERO].into_iter().product();
}
