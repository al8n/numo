use numo::{CheckedCastFrom, WidenFrom};

// The seal of the casts holds for a target of another crate, and for a source of another crate
// into a native integer.
#[derive(Clone, Copy)]
struct Local;

impl CheckedCastFrom<u8> for Local {
  fn checked_cast_from(_: u8) -> Option<Self> {
    None
  }

  fn strict_cast_from(_: u8) -> Self {
    Local
  }
}

impl WidenFrom<Local> for u64 {
  fn widen_from(_: Local) -> u64 {
    0
  }
}

fn main() {}
