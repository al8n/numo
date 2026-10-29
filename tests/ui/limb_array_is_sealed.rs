use numo::{LimbArray, typenum::U1};

// The seal of `LimbArray` holds for every width, even a local type.
struct Local;

impl LimbArray<Local> for u64 {
  type Count = U1;
  type Array = [u64; 1];
}

fn main() {}
