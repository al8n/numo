use numo::{NonZeroInt, i7, typenum::U7};

// A signed division can overflow even by a divisor that is not zero, as `MIN / -1` does, so `Int`
// has no `/` or `%` by a `NonZeroInt`, as the native signed integers have none by a native
// `NonZero`.
fn main() {
  let divisor = NonZeroInt::<U7>::new(i7::new(-1)).unwrap();
  let _quotient = i7::MIN / divisor;
}
