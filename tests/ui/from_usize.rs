use numo::{i256, u256};

fn main() {
  // `usize` and `isize` convert into a numo integer by `TryFrom` alone, at every width.
  let _ = u256::from(5usize);
  let _ = i256::from(-5isize);
}
