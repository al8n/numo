use numo::{
  Decimal,
  typenum::{U8, U10},
};

fn main() {
  // 18 digits do not fit in an `i32`, which holds 9. `Default` and `FromStr` each have one impl for
  // the storage, through which each reports the storage.
  let _zero: Decimal<U10, U8, i32> = Default::default();
  let _parsed = "1.5".parse::<Decimal<U10, U8, i32>>();
}
