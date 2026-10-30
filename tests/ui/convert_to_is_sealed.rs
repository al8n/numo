use numo::{
  ConvertError, ConvertTo, Decimal, RoundingMode,
  typenum::{U0, U1},
};

// The seal of `ConvertTo` holds for a decimal type with a target of another crate.
struct Local;

impl ConvertTo<Local> for Decimal<U1, U0> {
  fn try_convert(self) -> Result<Local, ConvertError> {
    Ok(Local)
  }

  fn round_to(self, _: RoundingMode) -> Result<Local, ConvertError> {
    Ok(Local)
  }
}

fn main() {}
