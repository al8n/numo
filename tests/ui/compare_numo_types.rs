use numo::{Uint, typenum::U8, u7};

fn main() {
  // Integers of two numo types do not compare, whatever their values: a cast converts one to the
  // type of the other first, as in `u7::new(5).widen::<Uint<U8>>()`.
  let _ = u7::new(5) == Uint::<U8>::new(5);
}
