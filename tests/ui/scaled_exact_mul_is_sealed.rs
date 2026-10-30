use numo::{
  Decimal, ExactMul, Scaled,
  typenum::{U0, U1},
};

// The seal of `ExactMul` holds for a `Scaled` with a right-hand side of another crate.
struct Local;

impl ExactMul<Local> for Scaled<Decimal<U1, U0>> {
  type Output = Scaled<Decimal<U1, U0>>;

  fn exact_mul(&self, _: &Local) -> Self::Output {
    *self
  }
}

fn main() {}
