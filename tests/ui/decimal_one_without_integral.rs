use numo::{
  Decimal, DecimalOne,
  typenum::{U0, U2},
};

fn one<T: DecimalOne>() -> T {
  T::one()
}

fn main() {
  // One needs an integral digit.
  let _one = one::<Decimal<U0, U2>>();
}
