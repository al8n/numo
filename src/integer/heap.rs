//! The integers in a heap storage, an array of limbs `[L; N]` in a `Box`: their limbs, through
//! [`ArrayInteger`] and [`FromLimbs`], and the conversions of the pointer into and out of an
//! integer, which move the pointer and copy no limb.

use core::borrow::Borrow;
use std::boxed::Box;

use super::{ArrayInteger, FromLimbs, Int, Uint, bytes::strict_limbs_slice};
use crate::{
  layout::Token,
  width::{ArrayLimb, ArrayRepr, FillsRepr, FitsBits, IntCore, UintCore},
};

/// Implements [`ArrayInteger`] and [`FromLimbs`] for the integers over the arrays of limbs in the
/// heap pointer `$pointer`, with the array as `Array`, and `From` of such an integer into its
/// pointer and of the pointer into an integer whose width fills the array, written
/// `pointer; Integer: Core, ..`. The limbs are lent and copied out through `Borrow`, an array is
/// put in an allocation of its own by `from_inline`, and the slices of limbs are read by the rows
/// of the storage.
macro_rules! heap_integers {
  ($pointer:ident; $($integer:ident: $core:ident),+ $(,)?) => {$(
    #[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
    impl<B, L, const N: usize> ArrayInteger for $integer<B, $pointer<[L; N]>>
    where
      B: FitsBits<$pointer<[L; N]>>,
      L: ArrayLimb,
      [L; N]: ArrayRepr,
    {
      type Array = [L; N];

      type Limb = L;

      // The limbs are checked as an array, then put in an allocation of their own.
      #[inline]
      fn checked_from_limbs(limbs: [L; N]) -> Option<Self> {
        if <[L; N] as $core>::fits_bits(&limbs, B::U32, Token::new()) {
          Some(Self::wrap(<$pointer<[L; N]> as $core>::from_inline(
            limbs,
            Token::new(),
          )))
        } else {
          None
        }
      }

      #[inline]
      fn to_limbs(&self) -> [L; N] {
        *self.as_limbs()
      }

      #[inline]
      fn as_limbs(&self) -> &[L; N] {
        self.repr.borrow()
      }

      #[inline]
      fn into_limbs(self) -> [L; N] {
        *self.repr.borrow()
      }

      #[inline]
      fn checked_from_limbs_slice(limbs: &[L]) -> Option<Self> {
        <$pointer<[L; N]> as $core>::checked_read_limbs(limbs, B::U32, Token::new())
          .map(Self::wrap)
      }

      #[inline]
      fn wrapping_from_limbs_slice(limbs: &[L]) -> Self {
        Self::wrap(<$pointer<[L; N]> as $core>::wrapping_read_limbs(
          limbs,
          B::U32,
          Token::new(),
        ))
      }

      #[inline]
      fn overflowing_from_limbs_slice(limbs: &[L]) -> (Self, bool) {
        match <Self as ArrayInteger>::checked_from_limbs_slice(limbs) {
          Some(value) => (value, false),
          None => (
            <Self as ArrayInteger>::wrapping_from_limbs_slice(limbs),
            true,
          ),
        }
      }

      #[inline]
      fn saturating_from_limbs_slice(limbs: &[L]) -> Self {
        Self::wrap(<$pointer<[L; N]> as $core>::saturating_read_limbs(
          limbs,
          B::U32,
          Token::new(),
        ))
      }

      #[inline]
      #[track_caller]
      fn strict_from_limbs_slice(limbs: &[L]) -> Self {
        match <Self as ArrayInteger>::checked_from_limbs_slice(limbs) {
          Some(value) => value,
          None => strict_limbs_slice(),
        }
      }
    }

    #[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
    impl<B, L, const N: usize> FromLimbs for $integer<B, $pointer<[L; N]>>
    where
      B: FillsRepr<$pointer<[L; N]>>,
      L: ArrayLimb,
      [L; N]: ArrayRepr,
    {
      #[inline]
      fn from_limbs(limbs: [L; N]) -> Self {
        Self::wrap(<$pointer<[L; N]> as $core>::from_inline(
          limbs,
          Token::new(),
        ))
      }
    }

    #[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
    impl<B, L, const N: usize> From<$integer<B, $pointer<[L; N]>>> for $pointer<[L; N]> {
      /// Returns the pointer that holds the limbs of `value`, least significant first, without a
      /// copy.
      #[inline]
      fn from(value: $integer<B, $pointer<[L; N]>>) -> Self {
        value.repr
      }
    }

    #[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
    impl<B, L, const N: usize> From<$pointer<[L; N]>> for $integer<B, $pointer<[L; N]>>
    where
      B: FillsRepr<$pointer<[L; N]>>,
      L: ArrayLimb,
      [L; N]: ArrayRepr,
    {
      /// Takes `limbs`, least significant first, as the storage of the integer that they hold,
      /// without a copy: the width fills the array, so every bit pattern is a value.
      #[inline]
      fn from(limbs: $pointer<[L; N]>) -> Self {
        Self::wrap(limbs)
      }
    }
  )+};
}

heap_integers!(Box; Uint: UintCore, Int: IntCore);
