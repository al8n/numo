//! The comparisons of [`Uint`] and [`Int`] with the native integers, both ways: `PartialEq` and
//! `PartialOrd` with each of the twelve native integers, which compare the values as integers,
//! whatever the widths and the signs, so that -1 of 7 bits is below `0u8` and `u256::MAX` is above
//! `u128::MAX`.
//!
//! Each comparison widens the native integer to `u128` or `i128`, which hold every native integer,
//! `usize` and `isize` under the assertion of the casts that they have at most 128 bits, and passes
//! it to the sealed `compare_u128` or `compare_i128` of the storage: a native storage widens its
//! value as well, and an array compares its limbs above 128 bits with zero or the copies of its
//! sign, then its low 128 bits. Integers of different numo types do not compare: a comparison
//! between them would take the place of the comparison of a type with itself, so the casts convert
//! one of them first.

use core::cmp::Ordering;

use super::{Int, Uint};
use crate::{
  layout::Token,
  width::{IntRepr, UintRepr},
};

/// Implements `PartialEq` and `PartialOrd` of [`Uint`] and [`Int`] over every storage of their
/// storage traits with each native integer, both ways, written `native: wide, compare;`, where
/// `wide`, `u128` or `i128`, holds every value of the native integer and `compare` is the sealed
/// method of the storages that compares a value with it.
macro_rules! native_comparisons {
  ($($native:ident: $wide:ident, $compare:ident;)+) => {$(
    native_comparisons!(@integer Uint: UintRepr, $native, $wide, $compare);
    native_comparisons!(@integer Int: IntRepr, $native, $wide, $compare);
  )+};
  (@integer $integer:ident: $repr:ident, $native:ident, $wide:ident, $compare:ident) => {
    impl<B, R: $repr> PartialEq<$native> for $integer<B, R> {
      /// Compares the values as integers.
      #[inline]
      fn eq(&self, other: &$native) -> bool {
        self.repr.$compare(*other as $wide, Token::new()).is_eq()
      }
    }

    impl<B, R: $repr> PartialOrd<$native> for $integer<B, R> {
      /// Compares the values as integers.
      #[inline]
      fn partial_cmp(&self, other: &$native) -> Option<Ordering> {
        Some(self.repr.$compare(*other as $wide, Token::new()))
      }
    }

    impl<B, R: $repr> PartialEq<$integer<B, R>> for $native {
      /// Compares the values as integers.
      #[inline]
      fn eq(&self, other: &$integer<B, R>) -> bool {
        other.repr.$compare(*self as $wide, Token::new()).is_eq()
      }
    }

    impl<B, R: $repr> PartialOrd<$integer<B, R>> for $native {
      /// Compares the values as integers.
      #[inline]
      fn partial_cmp(&self, other: &$integer<B, R>) -> Option<Ordering> {
        Some(other.repr.$compare(*self as $wide, Token::new()).reverse())
      }
    }
  };
}

native_comparisons! {
  u8: u128, compare_u128;
  u16: u128, compare_u128;
  u32: u128, compare_u128;
  u64: u128, compare_u128;
  u128: u128, compare_u128;
  usize: u128, compare_u128;
  i8: i128, compare_i128;
  i16: i128, compare_i128;
  i32: i128, compare_i128;
  i64: i128, compare_i128;
  i128: i128, compare_i128;
  isize: i128, compare_i128;
}
