//! The order of the 128-bit integers in `const fn`s, with which the layers compare the integers
//! with the native integers: `Ord::cmp` is not `const`.

use core::cmp::Ordering;

/// Compares `lhs` with `rhs`, as `Ord::cmp` does.
pub(crate) const fn cmp_u128(lhs: u128, rhs: u128) -> Ordering {
  if lhs < rhs {
    Ordering::Less
  } else if lhs > rhs {
    Ordering::Greater
  } else {
    Ordering::Equal
  }
}

/// Compares `lhs` with `rhs`, as `Ord::cmp` does.
pub(crate) const fn cmp_i128(lhs: i128, rhs: i128) -> Ordering {
  if lhs < rhs {
    Ordering::Less
  } else if lhs > rhs {
    Ordering::Greater
  } else {
    Ordering::Equal
  }
}
