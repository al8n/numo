use numo::{
  Decimal, Uint,
  typenum::{U18, U60, U320},
};

type Boxed = Decimal<U60, U18, Uint<U320, Box<[u64; 5]>>>;

fn main() {
  // A heap storage has no constants: `FixedDecimal::zero` and the other functions give the values,
  // and `from_inline` stores a constant of the inline storage.
  let _zero = Boxed::ZERO;
}
