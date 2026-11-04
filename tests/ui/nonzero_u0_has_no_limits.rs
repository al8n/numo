use numo::{NonZeroInt, NonZeroUint, typenum::U0};

fn main() {
  // Zero bits hold no value but zero, so the non-zero integers of zero bits have no limits.
  let _max = NonZeroUint::<U0>::MAX;
  let _min = NonZeroInt::<U0>::MIN;
}
