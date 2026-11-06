use numo::u256;

fn main() {
  // 256 bits have 32 bytes, which is the length of the type of their array.
  let _bytes: [u8; 31] = u256::MAX.to_le_bytes();
  let _value = u256::checked_from_le_bytes([0; 31]);
}
