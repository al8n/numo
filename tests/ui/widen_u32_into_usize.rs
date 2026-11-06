use numo::{
  Uint, WidenFrom,
  typenum::{U16, U17},
};

fn main() {
  // `usize` holds at least 16 bits, so `widen` takes into it no source of more bits.
  let _ = usize::widen_from(5u32);
  let _ = Uint::<U17>::MAX.widen::<usize>();
  // `usize` has no fixed upper bound, so it widens into nothing but itself, and nothing but itself
  // truncates into it, not even a value of 16 bits.
  let _ = u64::widen_from(5usize);
  let _ = Uint::<U16>::MAX.truncate::<usize>();
}
