use numo::{ConstFixedInteger, Uint, typenum::U256};

// The const layer needs `Copy` and the constants, which an integer in a heap storage does not have.
fn zero<T: ConstFixedInteger>() -> T {
  T::ZERO
}

fn main() {
  let _ = zero::<Uint<U256, Box<[u64; 4]>>>();
}
