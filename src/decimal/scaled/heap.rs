//! The assignments by reference of a [`Scaled`] over a decimal whose storage holds its coefficient
//! in place, a wide integer whose limbs are in a `Box`: `a += &b` and `a -= &b`.
//!
//! They assign to the decimal of their target, which a `Scaled` lends to no code outside its
//! module, so they are written in a child of that module, which reads its private fields. A shared
//! pointer never writes the coefficient that it holds, so a `Scaled` over a decimal in one has no
//! assignment, as the decimal has none.

use core::ops::{AddAssign, SubAssign};
use std::boxed::Box;

use super::Scaled;
use crate::{Decimal, FixedDecimal, Int, Uint, layout::InlineStorage, width::FillsRepr};

/// Implements `+=` and `-=` by reference of a [`Scaled`] over the decimals whose storage is a wide
/// integer with its limbs in the heap pointer `$pointer`, which holds its coefficient in place,
/// written `pointer, [cfg]; Integer, ..`, where `cfg` is the predicate of the features that give
/// the pointer, which the docs show. Each is the same assignment of the decimal, and then the
/// larger scale, as the assignments on values of `Scaled` are.
macro_rules! heap_scaled {
  ($pointer:ident, [$cfg:meta]; $($integer:ident),+ $(,)?) => {$(
    #[cfg_attr(docsrs, doc(cfg($cfg)))]
    impl<I, F, B, const N: usize> AddAssign<&Scaled<Decimal<I, F, $integer<B, $pointer<[u64; N]>>>>>
      for Scaled<Decimal<I, F, $integer<B, $pointer<[u64; N]>>>>
    where
      B: FillsRepr<[u64; N]>,
      $integer<B, [u64; N]>: InlineStorage,
      Decimal<I, F, $integer<B, [u64; N]>>: FixedDecimal,
      Self: FixedDecimal,
    {
      /// Adds `rhs`, by reference, to `self`, the decimal of `rhs` to that of `self` as `+=` does,
      /// in the allocation of the decimal of `self`, which takes the larger of the two scales.
      ///
      /// # Panics
      ///
      /// Panics if the sum is outside the range of the decimal, in every build, whatever
      /// `overflow-checks` is, and leaves `self` unchanged.
      /// [`checked_add`](FixedDecimal::checked_add) returns `None` instead.
      #[inline]
      #[track_caller]
      fn add_assign(&mut self, rhs: &Self) {
        self.value += &rhs.value;
        self.scale = self.scale.max(rhs.scale);
      }
    }

    #[cfg_attr(docsrs, doc(cfg($cfg)))]
    impl<I, F, B, const N: usize> SubAssign<&Scaled<Decimal<I, F, $integer<B, $pointer<[u64; N]>>>>>
      for Scaled<Decimal<I, F, $integer<B, $pointer<[u64; N]>>>>
    where
      B: FillsRepr<[u64; N]>,
      $integer<B, [u64; N]>: InlineStorage,
      Decimal<I, F, $integer<B, [u64; N]>>: FixedDecimal,
      Self: FixedDecimal,
    {
      /// Subtracts `rhs`, by reference, from `self`, the decimal of `rhs` from that of `self` as
      /// `-=` does, in the allocation of the decimal of `self`, which takes the larger of the two
      /// scales.
      ///
      /// # Panics
      ///
      /// Panics if the difference is outside the range of the decimal, which for an unsigned
      /// storage is every difference below zero, in every build, whatever `overflow-checks` is,
      /// and leaves `self` unchanged. [`checked_sub`](FixedDecimal::checked_sub) returns `None`
      /// instead.
      #[inline]
      #[track_caller]
      fn sub_assign(&mut self, rhs: &Self) {
        self.value -= &rhs.value;
        self.scale = self.scale.max(rhs.scale);
      }
    }
  )+};
}

heap_scaled!(Box, [any(feature = "std", feature = "alloc")]; Uint, Int);
