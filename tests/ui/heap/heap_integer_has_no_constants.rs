use numo::{Uint, typenum::U256};

fn main() {
  // A heap storage has no constants: `FixedInteger::zero` and the other functions give the values.
  let _zero = Uint::<U256, Box<[u64; 4]>>::ZERO;
}
