use numo::{FixedInteger, Uint, typenum::U9};

fn main() {
  // A constant of the trait reports the width check of the storage, as the inherent one does.
  let _max = <Uint<U9, u8> as FixedInteger>::MAX;
}
