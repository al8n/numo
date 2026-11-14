use numo::{
  Decimal, RoundingMode,
  typenum::{U8, U10, U11, U18, U290},
  u1024,
};

fn main() {
  // Two unsigned storages allow an intermediate of 308 digits, but not 290 + 8 + 11 = 309.
  let lhs = Decimal::<U290, U18, u1024>::MAX;
  let rhs = Decimal::<U10, U8, u64>::ONE;
  let _quotient: Result<Decimal<U10, U11, u128>, _> = lhs.div_round(&rhs, RoundingMode::Down);
}
