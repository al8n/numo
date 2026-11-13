use std::rc::Rc;

use numo::{
  Decimal, DecimalAssign, Uint,
  typenum::{U18, U60, U320},
};

// `DecimalAssign` needs a storage that holds the coefficient in place, which a shared storage is
// not.
fn double<T: DecimalAssign>(mut value: T) -> T {
  value += value.clone();
  value
}

fn main() {
  let value: Decimal<U60, U18, Uint<U320, Rc<[u64; 5]>>> = "1.5".parse().unwrap();
  let _ = double(value);
}
