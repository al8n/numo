use numo::{Int, Uint, typenum::U1};

// An array of no limbs holds zero bits only: a wider integer stored in it has no constants and no
// `Default`.
fn main() {
  let _ = Uint::<U1, [u64; 0]>::ZERO;
  let _: Int<U1, [u8; 0]> = Default::default();
}
