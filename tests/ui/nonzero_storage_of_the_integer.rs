use numo::{NonZeroUint, typenum::U7, u7};

// The storage of `NonZeroUint` is a non-zero storage, such as `core::num::NonZero<u8>`, not the
// storage of its integer: `NonZeroUint<U7, u8>` has no constructor and no methods.
fn main() {
  let _five = NonZeroUint::<U7, u8>::new(u7::new(5));
}

fn value(nonzero: NonZeroUint<U7, u8>) -> u7 {
  nonzero.get()
}
