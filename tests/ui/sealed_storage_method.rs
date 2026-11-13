use numo::{InlineStorage, Storage};

// A bound `R: InlineStorage` reaches the rows of its sealed supertrait, which take a token that
// only the crate can make.
fn product<R: InlineStorage>(lhs: R, rhs: R) {
  let _product = lhs.mul(&rhs);
}

// The token has no `Default`, so it cannot be inferred either.
fn forged<R: InlineStorage>(lhs: R, rhs: R) {
  let _product = lhs.mul(&rhs, Default::default());
}

// A bound `R: Storage` reaches the methods that give a storage its inline twin, which take the
// token too.
fn twin<R: Storage>(value: R) {
  let _twin = value.twin();
}

fn main() {}
