use numo::{BitWidth, FixedNonZero, NonZeroUint};

// In generic code, a storage that is not one of `NonZeroUint` fails at the trait, which states the
// storages.
fn zeros<B: BitWidth>(nonzero: &NonZeroUint<B, String>) -> u32 {
  FixedNonZero::leading_zeros(nonzero)
}

fn main() {}
