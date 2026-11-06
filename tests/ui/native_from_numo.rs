use numo::{i7, u7};

fn main() {
  // A native integer converts from a numo integer by `TryFrom` or by `widen`, never by `From`.
  let _ = u64::from(u7::MAX);
  let _: i64 = i7::MIN.into();
}
