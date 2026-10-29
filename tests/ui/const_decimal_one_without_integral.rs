use numo::{
  ConstDecimalOne, Decimal,
  typenum::{U0, U2},
};

fn one<T: ConstDecimalOne>() -> T {
  T::ONE
}

fn main() {
  // One needs an integral digit, also as a constant.
  let _one = one::<Decimal<U0, U2>>();
}
