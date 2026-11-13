use numo::{Uint, typenum::U64};

fn main() {
  // A heap storage boxes an array of limbs, not a native integer.
  let _: Uint<U64, Box<u64>> = Default::default();
}
