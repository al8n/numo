use numo::{
  Decimal, ParseError,
  typenum::{U8, U10},
};

// 18 digits do not fit in an `i32`, which holds 9. `Default` and `FromStr` each have one impl for
// the storage, through which each reports the storage. Each sits in a function of its own, so that
// the compiler reports each trait.
fn zero() -> Decimal<U10, U8, i32> {
  Default::default()
}

fn parsed() -> Result<Decimal<U10, U8, i32>, ParseError> {
  "1.5".parse()
}

fn main() {
  let _ = zero();
  let _ = parsed();
}
