use numo::u256;

fn main() {
  // No signed integer converts into `Uint` by `From`.
  let _ = u256::from(5i64);
  let _ = literal();
}

// A literal without a suffix is an `i32`, which converts into `Int` but not into `Uint`.
fn literal() -> u256 {
  u256::from(5)
}
