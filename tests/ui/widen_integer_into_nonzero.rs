use core::num::NonZero;

use numo::{NonZeroUint, WidenFrom, typenum::U16, u7};

fn main() {
  // An integer may be zero, so `widen` takes into a non-zero integer only a non-zero source;
  // `checked_cast` converts an integer, and returns `None` for zero.
  let _ = u7::new(5).widen::<NonZeroUint<U16>>();
  let _ = NonZero::<u16>::widen_from(5u8);
}
