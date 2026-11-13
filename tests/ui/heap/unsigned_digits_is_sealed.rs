use core::iter::FusedIterator;

use numo::UnsignedDigits;

// The seal of `UnsignedDigits` holds even for a type with every item.
struct Local;

impl UnsignedDigits for Local {
  fn checked_from_radix_be(_: &[u8], _: u32) -> Option<Self> {
    None
  }

  fn checked_from_radix_le(_: &[u8], _: u32) -> Option<Self> {
    None
  }

  fn checked_from_base_be<I: IntoIterator<Item = u64>>(_: I, _: u64) -> Option<Self> {
    None
  }

  fn checked_from_base_le<I: IntoIterator<Item = u64>>(_: I, _: u64) -> Option<Self> {
    None
  }

  fn to_base_le(&self, _: u64) -> impl FusedIterator<Item = u64> + use<> {
    core::iter::empty()
  }

  fn to_base_be(&self, _: u64) -> impl FusedIterator<Item = u64> + use<> {
    core::iter::empty()
  }

  fn to_radix_be(&self, _: u32) -> Vec<u8> {
    Vec::new()
  }

  fn to_radix_le(&self, _: u32) -> Vec<u8> {
    Vec::new()
  }
}

fn main() {}
