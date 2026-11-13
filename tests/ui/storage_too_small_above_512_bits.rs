use numo::{
  Decimal, i576,
  typenum::{U0, U2, U174},
};

fn main() {
  // 174 digits need an `i640`: a constant reports the unsatisfied bounds of its impl.
  let _zero = Decimal::<U174, U0, i576>::ZERO;
  // A conversion reports the storage diagnostic of its target.
  let _converted = Decimal::<U2, U2>::ZERO.try_convert::<U174, U0, i576>();
}
