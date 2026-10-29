use numo::{
  Decimal, ExactMul,
  typenum::{U0, U1},
};

// The seal of `ExactMul` holds for a decimal type with a right-hand side of another crate.
struct Local;

impl ExactMul<Local> for Decimal<U1, U0> {
  type Output = Decimal<U1, U0>;

  fn exact_mul(&self, _: &Local) -> Self::Output {
    *self
  }
}

fn main() {}
