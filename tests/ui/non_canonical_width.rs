use numo::{
  Int, Limbs, Uint,
  typenum::{B0, B1, UInt, UTerm},
};

// `typenum` writes no number with leading zero bits, which these have: 0 in one and two bits,
// and 1 in two bits, which compares as greater than `U1`.
type Zero1 = UInt<UTerm, B0>;
type Zero2 = UInt<UInt<UTerm, B0>, B0>;
type One2 = UInt<UInt<UTerm, B0>, B1>;

// Such a width has no default storage, and no array of limbs.
fn default_storage(_: Uint<Zero1>, _: Int<One2>) {}

fn limbs(_: Limbs<Zero1>) {}

fn main() {
  // Nor does a native or an array storage hold it.
  let _: Uint<Zero1, u8> = Default::default();
  let _ = Int::<One2, i8>::MIN;
  let _: Uint<Zero2, [u64; 1]> = Default::default();
  let _ = Int::<One2, [u64; 1]>::checked_from_limbs([0]);
}
