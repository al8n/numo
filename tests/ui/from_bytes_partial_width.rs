use numo::{Uint, typenum::U12};

fn main() {
  // 12 bits leave four bits of the second byte unused, so not every array of bytes is a value.
  let _value = Uint::<U12, [u8; 2]>::from_le_bytes([0x34, 0x12]);
}
