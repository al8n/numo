use numo::{
  Decimal, DivRound, DivideError, RoundingMode,
  typenum::{U8, U10, U11, U18, U135},
};

fn quotient<A: DivRound<B, Q>, B, Q>(lhs: &A, rhs: &B) -> Result<Q, DivideError> {
  lhs.div_round(rhs, RoundingMode::Down)
}

fn main() {
  // The scaled dividend has 135 + 8 + 11 = 154 digits, which only two unsigned storages allow,
  // through the trait as through the inherent method.
  let lhs = Decimal::<U135, U18>::MAX;
  let rhs = Decimal::<U10, U8>::ONE;
  let _quotient: Result<Decimal<U10, U11>, _> = quotient(&lhs, &rhs);
}
