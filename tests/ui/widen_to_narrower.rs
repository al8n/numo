use numo::{Uint, typenum::U9, u7};

fn main() {
  // `widen` takes a target at least as wide as the source.
  let _ = Uint::<U9>::MAX.widen::<u8>();
  let _: u7 = Uint::<U9>::MAX.widen();
}
