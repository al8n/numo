use std::{rc::Rc, thread};

use numo::{Uint, typenum::U256};

fn main() {
  // An integer in an `Rc` is neither `Send` nor `Sync`, as the `Rc` is not: an `Arc` shares a
  // value across threads.
  let value = Uint::<U256, Rc<[u64; 4]>>::from(1u64);
  thread::spawn(move || value);
}
