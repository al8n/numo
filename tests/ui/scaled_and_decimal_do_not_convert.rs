use numo::{
  ConvertTo, Decimal, Scaled,
  typenum::{U2, U8, U10},
};

// A `Scaled` relates only to a `Scaled`: a bare decimal does not convert into a `Scaled`, nor a
// `Scaled` into a bare decimal.
fn into_scaled<T: ConvertTo<Scaled<Decimal<U10, U2>>>>(_: T) {}

fn into_bare<T: ConvertTo<Decimal<U10, U2>>>(_: T) {}

fn main() {
  into_scaled(Decimal::<U10, U8>::ZERO);
  into_bare(Scaled::from(Decimal::<U10, U8>::ZERO));
}
