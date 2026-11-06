use numo::{Int, typenum::U16, u7};

fn main() {
  // `widen` and `truncate` keep the sign; the casts convert between the signs.
  let _ = u7::MAX.widen::<i16>();
  let _ = Int::<U16>::MIN.truncate::<u8>();
}
