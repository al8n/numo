use numo::{
  ConstIntegerOne, Int, Uint,
  typenum::{U0, U1},
};

fn one<T: ConstIntegerOne>() -> T {
  T::ONE
}

fn main() {
  // `Uint` needs a bit to hold one, also as a constant.
  let _unsigned = one::<Uint<U0>>();
  // `Int` needs two bits to hold one, also as a constant.
  let _signed = one::<Int<U1>>();
}
