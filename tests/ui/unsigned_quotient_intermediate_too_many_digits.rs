use numo::{
  Decimal, RoundingMode,
  typenum::{U8, U10, U11, U18, U136},
  u512,
};

fn main() {
  // Two unsigned storages allow an intermediate of 154 digits, but not 136 + 8 + 11 = 155.
  let lhs = Decimal::<U136, U18, u512>::MAX;
  let rhs = Decimal::<U10, U8, u64>::ONE;
  let _quotient: Result<Decimal<U10, U11, u128>, _> = lhs.div_round(&rhs, RoundingMode::Down);
}
