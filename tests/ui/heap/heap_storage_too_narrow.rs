use numo::{Uint, typenum::U300};

fn main() {
  // Four 64-bit limbs hold 256 bits, in a box as inline.
  let _: Uint<U300, Box<[u64; 4]>> = Default::default();
}
