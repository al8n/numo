use numo::{Uint, typenum::U64};

fn main() {
  // An array of a type that is not a limb is not a storage, and holds no width.
  let _: Uint<U64, [f32; 4]> = Default::default();
}
