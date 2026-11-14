use numo::{
  ConstFixedDecimal, Decimal, Uint,
  typenum::{U18, U60, U320},
};

// The const layer needs `Copy` and the constants, which a decimal in a heap storage does not have.
fn zero<T: ConstFixedDecimal>() -> T {
  T::ZERO
}

fn main() {
  let _ = zero::<Decimal<U60, U18, Uint<U320, Box<[u64; 5]>>>>();
}
