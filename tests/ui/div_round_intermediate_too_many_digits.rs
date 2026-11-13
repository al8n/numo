use numo::{
  Decimal, DivRound, DivideError, RoundingMode,
  typenum::{U8, U10, U11, U18, U289},
};

fn quotient<A: DivRound<B, Q>, B, Q>(lhs: &A, rhs: &B) -> Result<Q, DivideError> {
  lhs.div_round(rhs, RoundingMode::Down)
}

fn main() {
  // The scaled dividend has 289 + 8 + 11 = 308 digits, which only two unsigned storages allow,
  // through the trait as through the inherent method.
  let lhs = Decimal::<U289, U18>::MAX;
  let rhs = Decimal::<U10, U8>::ONE;
  let _quotient: Result<Decimal<U10, U11>, _> = quotient(&lhs, &rhs);
}
