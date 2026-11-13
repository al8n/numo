use std::rc::Rc;

use numo::{
  Decimal, Uint,
  typenum::{U18, U60, U320},
};

type Shared = Decimal<U60, U18, Uint<U320, Rc<[u64; 5]>>>;

fn main() {
  // A shared storage never writes the coefficient that it holds, so its decimal has no compound
  // assignment: `a = &a + &b` puts the sum in a new allocation.
  let mut a: Shared = "1.5".parse().unwrap();
  let b: Shared = "0.25".parse().unwrap();
  a += b;
}
