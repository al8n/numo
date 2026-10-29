use numo::{
  Decimal, WidenTo,
  typenum::{U0, U1},
};

// The seal of `WidenTo` holds for a decimal type with a target of another crate.
struct Local;

impl WidenTo<Local> for Decimal<U1, U0> {
  fn widen(self) -> Local {
    Local
  }
}

fn main() {}
