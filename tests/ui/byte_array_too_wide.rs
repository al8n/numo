use numo::{Uint, typenum::U16384};

fn main() {
  // The arrays of bytes stop at 8192 bits; a wider value copies its bytes to a buffer.
  let _bytes = Uint::<U16384, [u64; 256]>::MAX.to_le_bytes();
}
