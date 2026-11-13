use std::rc::Rc;

use numo::{Uint, typenum::U256};

fn main() {
  // A shared storage never writes the value that it holds, so its integer has no compound
  // assignment: `a = &a + &b` puts the sum in a new allocation.
  let mut a = Uint::<U256, Rc<[u64; 4]>>::from(1u64);
  let b = Uint::<U256, Rc<[u64; 4]>>::from(2u64);
  a += &b;
}
