use numo::{
  Decimal, RoundingMode,
  typenum::{U8, U10, U11, U18, U289},
};

fn main() {
  // The scaled dividend has 289 + 8 + 11 = 308 digits, which only two unsigned storages allow.
  let lhs = Decimal::<U289, U18>::MAX;
  let rhs = Decimal::<U10, U8>::ONE;
  let _quotient: Result<Decimal<U10, U11>, _> = lhs.div_round(&rhs, RoundingMode::Down);
}
