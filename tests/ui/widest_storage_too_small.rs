use numo::{
  Decimal,
  typenum::{U0, U2, U309},
  u1024,
};

fn main() {
  // 309 digits do not fit the widest storage: a constant reports the unsatisfied bounds of its impl.
  let _zero = Decimal::<U309, U0, u1024>::ZERO;
  // A conversion reports the storage diagnostic of its target.
  let _converted = Decimal::<U2, U2>::ZERO.try_convert::<U309, U0, u1024>();
}
