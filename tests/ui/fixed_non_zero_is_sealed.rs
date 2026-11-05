use core::{fmt, num::NonZero};

use numo::FixedNonZero;

// The seal of `FixedNonZero` holds even for a type with every supertrait and item.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
struct Local;

impl fmt::Display for Local {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("1")
  }
}

impl FixedNonZero for Local {
  type Integer = u8;

  const BITS: u32 = 1;

  fn new(_: u8) -> Option<Self> {
    Some(Self)
  }

  fn get(self) -> u8 {
    1
  }

  unsafe fn new_unchecked(_: u8) -> Self {
    Self
  }

  fn leading_zeros(&self) -> u32 {
    0
  }

  fn trailing_zeros(&self) -> u32 {
    0
  }

  fn count_ones(&self) -> NonZero<u32> {
    NonZero::<u32>::MIN
  }

  fn highest_one(&self) -> u32 {
    0
  }

  fn lowest_one(&self) -> u32 {
    0
  }

  fn isolate_highest_one(&self) -> Self {
    Self
  }

  fn isolate_lowest_one(&self) -> Self {
    Self
  }

  fn reverse_bits(&self) -> Self {
    Self
  }

  fn rotate_left(&self, _: u32) -> Self {
    Self
  }

  fn rotate_right(&self, _: u32) -> Self {
    Self
  }

  fn checked_mul(&self, _: &Self) -> Option<Self> {
    Some(Self)
  }

  fn checked_pow(&self, _: u32) -> Option<Self> {
    Some(Self)
  }

  fn saturating_mul(&self, _: &Self) -> Self {
    Self
  }

  fn saturating_pow(&self, _: u32) -> Self {
    Self
  }
}

fn main() {}
