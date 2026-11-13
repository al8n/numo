use numo::{
  Decimal, Uint, dec,
  typenum::{U18, U60, U320},
};

type Boxed = Decimal<U60, U18, Uint<U320, Box<[u64; 5]>>>;

fn main() {
  // A heap decimal has two forms of `+` with an owned left operand, `a + b` and `a + &b`, so a
  // literal on the right gets no type, and no heap decimal is a literal anyway.
  let fee: Boxed = "1.5".parse().unwrap();
  let _ = fee + dec!(0.5);
}
