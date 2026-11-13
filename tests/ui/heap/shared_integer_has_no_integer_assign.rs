use std::rc::Rc;

use numo::{IntegerAssign, Uint, typenum::U256};

// `IntegerAssign` needs a storage that holds the value in place, which a shared storage is not.
fn double<T: IntegerAssign>(mut value: T) -> T {
  value += value.clone();
  value
}

fn main() {
  let _ = double(Uint::<U256, Rc<[u64; 4]>>::from(1u64));
}
