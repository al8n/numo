use numo::{
  Decimal, DivRound, DivideError, RoundingMode, Scaled,
  typenum::{U0, U1},
};

// The seal of `DivRound` holds for a `Scaled` with a divisor of another crate.
struct Local;

impl DivRound<Local, Scaled<Decimal<U1, U0>>> for Scaled<Decimal<U1, U0>> {
  fn div_round(
    &self,
    _: &Local,
    _: RoundingMode,
  ) -> Result<Scaled<Decimal<U1, U0>>, DivideError> {
    Ok(*self)
  }
}

fn main() {}
