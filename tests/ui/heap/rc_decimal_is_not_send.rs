use std::{rc::Rc, thread};

use numo::{
  Decimal, Uint,
  typenum::{U18, U60, U320},
};

fn main() {
  // A decimal in an `Rc` is neither `Send` nor `Sync`, as the `Rc` is not: an `Arc` shares a value
  // across threads.
  let value: Decimal<U60, U18, Uint<U320, Rc<[u64; 5]>>> = "1.5".parse().unwrap();
  thread::spawn(move || value);
}
