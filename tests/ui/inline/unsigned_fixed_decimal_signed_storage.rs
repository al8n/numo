use numo::{
  Decimal, UnsignedFixedDecimal,
  typenum::{U8, U10},
};

fn remaining<T: UnsignedFixedDecimal>(budget: &T, cost: &T) -> Option<T> {
  budget.checked_sub(cost)
}

fn main() {
  // A signed storage has negative values, and no `UnsignedFixedDecimal`.
  let zero = Decimal::<U10, U8>::ZERO;
  let _remaining = remaining(&zero, &zero);
}
