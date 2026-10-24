use numo::{Int, Uint, typenum::U7};

fn main() {
  // `Int` stores a signed native integer, and `Uint` an unsigned one.
  let _: Int<U7, u8> = Default::default();
  let _max = Uint::<U7, i8>::MAX;
}
