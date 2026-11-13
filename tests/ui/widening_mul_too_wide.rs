use numo::{
  Uint,
  typenum::{U1, U65536},
};

fn main() {
  // The product would have 65537 bits, which have no default storage.
  let _ = Uint::<U65536>::MAX.widening_mul(&Uint::<U1>::MAX);
}
