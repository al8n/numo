use numo::{
  FillsRepr, Int, Uint,
  typenum::{U9, U129},
};

fn fills<B: FillsRepr<u16>>() {}

fn main() {
  // A native integer holds at most its own number of bits.
  let _: Uint<U9, u8> = Default::default();
  let _max = Int::<U129, i128>::MAX;
  // 9 bits fit in a `u16` but do not fill it.
  fills::<U9>();
}
