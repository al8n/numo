use numo::{
  Decimal, Uint,
  typenum::{U2, U18, U60, U256},
};

fn main() {
  // 78 digits do not fit in the four limbs of a `u256`, in a box as inline. Parsing reports the
  // storage diagnostic of the inline storage, through which the heap storage computes.
  let _parsed = "1".parse::<Decimal<U60, U18, Uint<U256, Box<[u64; 4]>>>>();
  // A conversion reports the storage diagnostic of its target.
  let _converted = Decimal::<U2, U2>::ZERO.try_convert::<U60, U18, Uint<U256, Box<[u64; 4]>>>();
}
