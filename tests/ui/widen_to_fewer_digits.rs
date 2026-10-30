use numo::{
  Decimal, WidenTo,
  typenum::{U2, U4, U8, U10},
};

fn widen<T: WidenTo<U>, U>(value: T) -> U {
  value.widen()
}

fn main() {
  // Through the trait, as through the inherent method, `widen` keeps every digit.
  let _fewer_integral: Decimal<U2, U8> = widen(Decimal::<U10, U8>::ZERO);
  let _fewer_fraction: Decimal<U10, U4> = widen(Decimal::<U10, U8>::ZERO);
}
