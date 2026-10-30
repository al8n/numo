use numo::{
  Decimal, Scaled,
  typenum::{U8, U10},
};

// A `Scaled` over a decimal whose storage does not hold its digits has no `parse` and no
// constants: 18 digits do not fit an `i32`.
fn main() {
  let _ = Scaled::<Decimal<U10, U8, i32>>::parse("1");
  let _ = <Scaled<Decimal<U10, U8, i32>> as numo::ConstFixedDecimal>::ZERO;
}
