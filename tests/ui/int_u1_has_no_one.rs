use numo::i1;

fn main() {
  // One bit holds only -1 and 0, so `Int` of one bit has no `ONE`, and no `Product`, which
  // multiplies from one.
  let _one = i1::ONE;
  let _product: i1 = [i1::MIN].into_iter().product();
}
