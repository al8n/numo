use numo::{
  Decimal, WidenTo,
  typenum::{U2, U4},
};

fn widen<T: WidenTo<U>, U>(value: T) -> U {
  value.widen()
}

fn main() {
  // Through the trait, as through the inherent method, a signed storage widens only to a signed
  // one.
  let _unsigned: Decimal<U2, U4, u32> = widen(Decimal::<U2, U2>::ZERO);
}
