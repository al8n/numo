use std::fmt;

use numo::serde::quantity::Quantity;

// The trait of a helper is sealed: a type of another crate cannot implement it, even with the
// format that the helper writes.
struct Local;

impl fmt::LowerHex for Local {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("0")
  }
}

impl Quantity for Local {}

fn main() {}
