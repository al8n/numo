use numo::{
  Decimal, HasDecimalOne,
  typenum::{U0, U2},
};

fn one<T: HasDecimalOne>() -> T {
  T::ONE
}

fn main() {
  // One needs an integral digit.
  let _one = one::<Decimal<U0, U2>>();
}
