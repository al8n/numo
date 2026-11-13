use numo::{Storage, Uint, typenum::U200};

// Code generic over the storage of a decimal.
fn storage<R: Storage>() {}

fn main() {
  // A heap storage of a decimal is a wide integer whose width fills its array of 64-bit limbs, as
  // that of an inline storage does: 200 bits do not fill four limbs.
  storage::<Uint<U200, Box<[u64; 4]>>>();
}
