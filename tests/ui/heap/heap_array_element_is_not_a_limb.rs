use numo::{Uint, typenum::U64};

fn main() {
  // An array of a type that is not a limb is not a storage, in a box either.
  let _: Uint<U64, Box<[f32; 4]>> = Default::default();
}
