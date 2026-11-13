use numo::{
  Decimal, Uint, dec,
  typenum::{U18, U60, U320},
};

type Boxed = Decimal<U60, U18, Uint<U320, Box<[u64; 5]>>>;

fn main() {
  // A literal is a constant, which a heap storage cannot be: `Boxed::from_inline(dec!(1.5))` stores
  // a literal of the inline storage.
  let _fee: Boxed = dec!(1.5);
}
