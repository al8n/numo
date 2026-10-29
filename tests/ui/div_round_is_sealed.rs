use numo::{
  Decimal, DivRound, DivideError, RoundingMode,
  typenum::{U0, U1},
};

// The seal of `DivRound` holds for a decimal type with a divisor of another crate.
struct Local;

impl DivRound<Local, Decimal<U1, U0>> for Decimal<U1, U0> {
  fn div_round(&self, _: &Local, _: RoundingMode) -> Result<Decimal<U1, U0>, DivideError> {
    Ok(*self)
  }
}

fn main() {}
