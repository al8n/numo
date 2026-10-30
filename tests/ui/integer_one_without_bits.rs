use numo::{
  Int, IntegerOne, Uint,
  typenum::{U0, U1},
};

fn one<T: IntegerOne>() -> T {
  T::one()
}

fn main() {
  // `Uint` needs a bit to hold one.
  let _unsigned = one::<Uint<U0>>();
  // `Int` needs two bits to hold one.
  let _signed = one::<Int<U1>>();
}
