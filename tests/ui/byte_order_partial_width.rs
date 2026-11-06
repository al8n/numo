use numo::{
  Int, Uint,
  typenum::{U7, U12},
};

fn main() {
  // 12 bits leave four bits of the second byte, so no order of whole bytes holds them.
  let value = Uint::<U12, u16>::new(0x123);
  let _swapped = value.swap_bytes();
  let _big = value.to_be();
  let _value = Uint::<U12, u16>::from_be(value);
  // Neither do 7 bits in an `i8`.
  let _swapped = Int::<U7, i8>::new(-1).swap_bytes();
}
