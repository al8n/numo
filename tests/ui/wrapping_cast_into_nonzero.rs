use core::num::NonZero;

use numo::{NonZeroUint, typenum::U8, u7};

fn main() {
  // A wrapped or saturated value may be zero, so a non-zero integer is a target of the casts that
  // check only, such as `checked_cast`.
  let _ = u7::new(5).wrapping_cast::<NonZeroUint<U8>>();
  let _ = u7::new(5).saturating_cast::<NonZero<u8>>();
}
