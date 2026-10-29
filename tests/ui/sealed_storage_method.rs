use numo::Storage;

// A bound `R: Storage` reaches the methods of its sealed supertrait, which take a token that only
// the crate can make.
fn product<R: Storage>(lhs: R, rhs: R) {
  let _product = lhs.mul(&rhs);
}

// The token has no `Default`, so it cannot be inferred either.
fn forged<R: Storage>(lhs: R, rhs: R) {
  let _product = lhs.mul(&rhs, Default::default());
}

fn main() {}
