use numo::{
  Decimal, Scaled, WidenTo,
  typenum::{U0, U1},
};

// The seal of `WidenTo` holds for a `Scaled` with a target of another crate.
struct Local;

impl WidenTo<Local> for Scaled<Decimal<U1, U0>> {
  fn widen(self) -> Local {
    Local
  }
}

fn main() {}
