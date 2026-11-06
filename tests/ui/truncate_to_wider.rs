use numo::{TruncateFrom, u7};

fn main() {
  // `truncate` takes a target at most as wide as the source.
  let _ = u7::MAX.truncate::<u8>();
  let _ = u7::truncate_from(5u64).truncate::<u64>();
}
