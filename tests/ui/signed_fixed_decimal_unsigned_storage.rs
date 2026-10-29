use numo::{
  Decimal, SignedFixedDecimal,
  typenum::{U9, U10},
};

fn negatives<T: SignedFixedDecimal>(values: &[T]) -> usize {
  values.iter().filter(|value| value.is_negative()).count()
}

fn main() {
  // An unsigned storage has no negative values, and no `SignedFixedDecimal`.
  let _count = negatives(&[Decimal::<U10, U9, u64>::ZERO]);
}
