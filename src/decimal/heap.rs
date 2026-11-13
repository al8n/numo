//! The traits of [`Decimal`] over a heap storage, a wide integer whose limbs are in a `Box`, an
//! `Rc`, an `Arc` or a `triomphe::Arc`; its operators with one owned operand, `a + &b` and
//! `&a + b`, and for a `Box` its assignments by reference, `a += &b`; and the parsing and the
//! operators with one owned operand of a [`Scaled`] over such a decimal.
//!
//! Each impl names its pointer in its header, so that no impl of this module matches a decimal over
//! an inline storage: the candidates of such a decimal, and so what the compiler infers and reports
//! for it, are those of its own storage. One impl over every storage that a sealed trait marks as a
//! heap storage would match every decimal, and be a second candidate beside the impl of an inline
//! storage. An inline decimal has no operator with one owned operand, so that a literal on the
//! right takes the type of the left operand, as in `price + dec!(0.5)`; with these, a heap decimal
//! has two forms of `+` with an owned left operand.
//!
//! Every item computes in the twin, the decimal over the inline storage of the same width and sign,
//! through the trait of the twin, which each call names. In an impl of a trait, `Self::` names an
//! inherent item first, which a heap decimal has only where the item is generic over the storage,
//! as `to_inline` and `from_inline` are, and otherwise the item of that trait itself. A value is
//! stored once, by `from_inline`, after the twin returned it: nothing is built for `None` or an
//! error. An operator stores into the operand that it owns, after the checked method of the twins
//! returned, so that one that panics leaves its operands.

#[cfg(any(feature = "std", feature = "alloc"))]
use core::ops::{AddAssign, SubAssign};
use core::{
  ops::{Add, Neg, Sub},
  str::FromStr,
};
#[cfg(all(any(feature = "std", feature = "alloc"), target_has_atomic = "ptr"))]
use std::sync::Arc;
#[cfg(any(feature = "std", feature = "alloc"))]
use std::{boxed::Box, rc::Rc};

#[cfg(all(feature = "triomphe", target_has_atomic = "ptr"))]
use triomphe::Arc as TriompheArc;

use super::{
  ConstDecimalOne, ConstFixedDecimal, ConvertError, Decimal, DecimalFloat, DecimalOne,
  FixedDecimal, ParseError, QuantizeError, Scaled, SignedFixedDecimal, UnsignedFixedDecimal,
  ops::overflow,
};
use crate::{
  FromFloatError, Int, Uint,
  layout::{InlineCore, InlineStorage, Layout, StorageCore, Token},
  rounding::RoundingMode,
  width::FillsRepr,
};

/// Implements the traits of [`Decimal`] over the wide integers whose limbs are in the heap pointer
/// `$pointer`, written `pointer, kind, [cfg]`, where `kind` is `in_place` for a pointer that holds
/// its coefficient in place and `shared` for one that never writes it, and `cfg` is the predicate
/// of the features and the target that give the pointer, which the docs show. Each impl is bounded
/// as the storage is, by the width that fills the array and by the integer over the array being an
/// inline storage, and by the trait of the twin. The impls that both integers have come from the
/// arm `@shared`, those of one sign are written for its integer by name, and the assignments by
/// reference come from the arm `@mutable` of the kind.
macro_rules! heap_decimals {
  ($pointer:ident, $kind:ident, [$cfg:meta]) => {
    heap_decimals!(@shared Uint, $pointer, [$cfg]);
    heap_decimals!(@shared Int, $pointer, [$cfg]);
    heap_decimals!(@mutable $kind, Uint, $pointer, [$cfg]);
    heap_decimals!(@mutable $kind, Int, $pointer, [$cfg]);

    #[cfg_attr(docsrs, doc(cfg($cfg)))]
    impl<I, F, B, const N: usize> SignedFixedDecimal for Decimal<I, F, Int<B, $pointer<[u64; N]>>>
    where
      B: FillsRepr<[u64; N]>,
      Int<B, [u64; N]>: InlineStorage,
      (I, F): Layout<Int<B, $pointer<[u64; N]>>>,
      Decimal<I, F, Int<B, [u64; N]>>: ConstFixedDecimal + SignedFixedDecimal,
    {
      #[inline]
      fn is_negative(&self) -> bool {
        SignedFixedDecimal::is_negative(&self.to_inline())
      }

      #[inline]
      fn neg(&self) -> Self {
        Self::from_inline(SignedFixedDecimal::neg(&self.to_inline()))
      }

      #[inline]
      fn abs(&self) -> Self {
        Self::from_inline(SignedFixedDecimal::abs(&self.to_inline()))
      }
    }

    #[cfg_attr(docsrs, doc(cfg($cfg)))]
    impl<I, F, B, const N: usize> Neg for Decimal<I, F, Int<B, $pointer<[u64; N]>>>
    where
      B: FillsRepr<[u64; N]>,
      Int<B, [u64; N]>: InlineStorage,
      Decimal<I, F, Int<B, [u64; N]>>: SignedFixedDecimal,
    {
      type Output = Self;

      /// Returns `-self`. Never fails, because the range is symmetric. The value is stored into
      /// `self`: in its allocation for a `Box`, and in a new allocation for a shared pointer.
      #[inline]
      fn neg(mut self) -> Self {
        let negated = SignedFixedDecimal::neg(&self.to_inline());
        StorageCore::set_twin(&mut self.repr, negated.repr, Token::new());
        self
      }
    }

    #[cfg_attr(docsrs, doc(cfg($cfg)))]
    impl<I, F, B, const N: usize> UnsignedFixedDecimal
      for Decimal<I, F, Uint<B, $pointer<[u64; N]>>>
    where
      B: FillsRepr<[u64; N]>,
      Uint<B, [u64; N]>: InlineStorage,
      (I, F): Layout<Uint<B, $pointer<[u64; N]>>>,
      Decimal<I, F, Uint<B, [u64; N]>>: ConstFixedDecimal + UnsignedFixedDecimal,
    {
    }
  };
  (@shared $integer:ident, $pointer:ident, [$cfg:meta]) => {
    #[cfg_attr(docsrs, doc(cfg($cfg)))]
    impl<I, F, B, const N: usize> Default for Decimal<I, F, $integer<B, $pointer<[u64; N]>>>
    where
      B: FillsRepr<[u64; N]>,
      $integer<B, [u64; N]>: InlineStorage,
      Decimal<I, F, $integer<B, [u64; N]>>: ConstFixedDecimal,
    {
      /// Returns zero.
      #[inline]
      fn default() -> Self {
        Self::from_inline(<Decimal<I, F, $integer<B, [u64; N]>> as ConstFixedDecimal>::ZERO)
      }
    }

    #[cfg_attr(docsrs, doc(cfg($cfg)))]
    impl<I, F, B, const N: usize> FromStr for Decimal<I, F, $integer<B, $pointer<[u64; N]>>>
    where
      B: FillsRepr<[u64; N]>,
      $integer<B, [u64; N]>: InlineStorage,
      Decimal<I, F, $integer<B, [u64; N]>>: ConstFixedDecimal,
    {
      type Err = ParseError;

      /// Parses a decimal from text in the syntax of [`Decimal::parse`], as its twin, and stores
      /// the value: nothing is built for an error.
      #[inline]
      fn from_str(s: &str) -> Result<Self, ParseError> {
        <Decimal<I, F, $integer<B, [u64; N]>> as FromStr>::from_str(s).map(Self::from_inline)
      }
    }

    // `INTEGRAL`, `FRACTION`, `DIGITS`, `to_inline` and `from_inline` are inherent too, and
    // `Self::` names an inherent item before the item of this trait. The formats, which are
    // supertraits, need the layout of the heap storage itself, which the compiler does not derive
    // from that of the twin.
    #[cfg_attr(docsrs, doc(cfg($cfg)))]
    impl<I, F, B, const N: usize> FixedDecimal for Decimal<I, F, $integer<B, $pointer<[u64; N]>>>
    where
      B: FillsRepr<[u64; N]>,
      $integer<B, [u64; N]>: InlineStorage,
      (I, F): Layout<$integer<B, $pointer<[u64; N]>>>,
      Decimal<I, F, $integer<B, [u64; N]>>: ConstFixedDecimal,
    {
      const INTEGRAL: u32 = <Decimal<I, F, $integer<B, [u64; N]>> as FixedDecimal>::INTEGRAL;
      const FRACTION: u32 = <Decimal<I, F, $integer<B, [u64; N]>> as FixedDecimal>::FRACTION;
      const DIGITS: u32 = <Decimal<I, F, $integer<B, [u64; N]>> as FixedDecimal>::DIGITS;
      const SIGNED: bool = <Decimal<I, F, $integer<B, [u64; N]>> as FixedDecimal>::SIGNED;

      type Repr = $integer<B, $pointer<[u64; N]>>;

      type Inline = Decimal<I, F, $integer<B, [u64; N]>>;

      #[inline]
      fn to_inline(&self) -> Self::Inline {
        Self::to_inline(self)
      }

      #[inline]
      fn from_inline(value: Self::Inline) -> Self {
        Self::from_inline(value)
      }

      #[inline]
      fn repr(self) -> Self::Repr {
        self.repr
      }

      // The range is checked on the twin of the storage with the row with which `try_convert`
      // checks its target: the `Repr` of the twin decimal, and so its `from_repr`, is not known
      // here. The storage that the caller gave is wrapped, or dropped.
      #[inline]
      fn from_repr(repr: Self::Repr) -> Option<Self> {
        let coefficient = StorageCore::twin(&repr, Token::new());
        let digits = <Decimal<I, F, $integer<B, [u64; N]>> as FixedDecimal>::DIGITS;
        if InlineCore::within_digits(&coefficient, digits, Token::new()) {
          Some(Self::from_coeff(repr))
        } else {
          None
        }
      }

      #[inline]
      fn zero() -> Self {
        Self::from_inline(<Decimal<I, F, $integer<B, [u64; N]>> as ConstFixedDecimal>::ZERO)
      }

      #[inline]
      fn max_value() -> Self {
        Self::from_inline(<Decimal<I, F, $integer<B, [u64; N]>> as ConstFixedDecimal>::MAX)
      }

      #[inline]
      fn min_value() -> Self {
        Self::from_inline(<Decimal<I, F, $integer<B, [u64; N]>> as ConstFixedDecimal>::MIN)
      }

      #[inline]
      fn is_zero(&self) -> bool {
        FixedDecimal::is_zero(&self.to_inline())
      }

      #[inline]
      fn is_positive(&self) -> bool {
        FixedDecimal::is_positive(&self.to_inline())
      }

      #[inline]
      fn checked_add(&self, rhs: &Self) -> Option<Self> {
        FixedDecimal::checked_add(&self.to_inline(), &rhs.to_inline()).map(Self::from_inline)
      }

      #[inline]
      fn checked_sub(&self, rhs: &Self) -> Option<Self> {
        FixedDecimal::checked_sub(&self.to_inline(), &rhs.to_inline()).map(Self::from_inline)
      }

      #[inline]
      fn checked_rem(&self, rhs: &Self) -> Option<Self> {
        FixedDecimal::checked_rem(&self.to_inline(), &rhs.to_inline()).map(Self::from_inline)
      }

      #[inline]
      fn saturating_add(&self, rhs: &Self) -> Self {
        Self::from_inline(FixedDecimal::saturating_add(&self.to_inline(), &rhs.to_inline()))
      }

      #[inline]
      fn saturating_sub(&self, rhs: &Self) -> Self {
        Self::from_inline(FixedDecimal::saturating_sub(&self.to_inline(), &rhs.to_inline()))
      }

      #[inline]
      fn quantize(&self, step: &Self, mode: RoundingMode) -> Result<Self, QuantizeError> {
        FixedDecimal::quantize(&self.to_inline(), &step.to_inline(), mode).map(Self::from_inline)
      }

      #[inline]
      fn is_multiple_of(&self, step: &Self) -> bool {
        FixedDecimal::is_multiple_of(&self.to_inline(), &step.to_inline())
      }

      #[inline]
      fn trunc(&self) -> Self {
        Self::from_inline(FixedDecimal::trunc(&self.to_inline()))
      }

      #[inline]
      fn fract(&self) -> Self {
        Self::from_inline(FixedDecimal::fract(&self.to_inline()))
      }

      #[inline]
      fn floor(&self) -> Result<Self, ConvertError> {
        FixedDecimal::floor(&self.to_inline()).map(Self::from_inline)
      }

      #[inline]
      fn ceil(&self) -> Result<Self, ConvertError> {
        FixedDecimal::ceil(&self.to_inline()).map(Self::from_inline)
      }

      #[inline]
      fn is_integer(&self) -> bool {
        FixedDecimal::is_integer(&self.to_inline())
      }

      #[inline]
      fn order_of_magnitude(&self) -> i32 {
        FixedDecimal::order_of_magnitude(&self.to_inline())
      }

      #[inline]
      fn significant_digits(&self) -> u32 {
        FixedDecimal::significant_digits(&self.to_inline())
      }

      #[inline]
      fn fraction_digits(&self) -> u32 {
        FixedDecimal::fraction_digits(&self.to_inline())
      }

      #[inline]
      fn round_significant(
        &self,
        digits: core::num::NonZero<u32>,
        mode: RoundingMode,
      ) -> Result<Self, ConvertError> {
        FixedDecimal::round_significant(&self.to_inline(), digits, mode).map(Self::from_inline)
      }
    }

    #[cfg_attr(docsrs, doc(cfg($cfg)))]
    impl<I, F, B, const N: usize> DecimalOne for Decimal<I, F, $integer<B, $pointer<[u64; N]>>>
    where
      B: FillsRepr<[u64; N]>,
      $integer<B, [u64; N]>: InlineStorage,
      (I, F): Layout<$integer<B, $pointer<[u64; N]>>>,
      Decimal<I, F, $integer<B, [u64; N]>>: ConstDecimalOne,
    {
      #[inline]
      fn one() -> Self {
        Self::from_inline(<Decimal<I, F, $integer<B, [u64; N]>> as ConstDecimalOne>::ONE)
      }
    }

    #[cfg_attr(docsrs, doc(cfg($cfg)))]
    impl<I, F, B, const N: usize> DecimalFloat for Decimal<I, F, $integer<B, $pointer<[u64; N]>>>
    where
      B: FillsRepr<[u64; N]>,
      $integer<B, [u64; N]>: InlineStorage,
      (I, F): Layout<$integer<B, $pointer<[u64; N]>>>,
      Decimal<I, F, $integer<B, [u64; N]>>: ConstFixedDecimal + DecimalFloat,
    {
      #[inline]
      fn to_f64(&self) -> f64 {
        DecimalFloat::to_f64(&self.to_inline())
      }

      #[inline]
      fn to_f32(&self) -> f32 {
        DecimalFloat::to_f32(&self.to_inline())
      }

      #[inline]
      fn round_from_f64(value: f64, mode: RoundingMode) -> Result<Self, FromFloatError> {
        <Decimal<I, F, $integer<B, [u64; N]>> as DecimalFloat>::round_from_f64(value, mode)
          .map(Self::from_inline)
      }

      #[inline]
      fn round_from_f32(value: f32, mode: RoundingMode) -> Result<Self, FromFloatError> {
        <Decimal<I, F, $integer<B, [u64; N]>> as DecimalFloat>::round_from_f32(value, mode)
          .map(Self::from_inline)
      }

      #[inline]
      fn from_f64_shortest(value: f64, mode: RoundingMode) -> Result<Self, FromFloatError> {
        <Decimal<I, F, $integer<B, [u64; N]>> as DecimalFloat>::from_f64_shortest(value, mode)
          .map(Self::from_inline)
      }

      #[inline]
      fn from_f32_shortest(value: f32, mode: RoundingMode) -> Result<Self, FromFloatError> {
        <Decimal<I, F, $integer<B, [u64; N]>> as DecimalFloat>::from_f32_shortest(value, mode)
          .map(Self::from_inline)
      }
    }

    #[cfg(feature = "serde")]
    #[cfg_attr(docsrs, doc(cfg(all($cfg, feature = "serde"))))]
    impl<'de, I, F, B, const N: usize> serde_core::Deserialize<'de>
      for Decimal<I, F, $integer<B, $pointer<[u64; N]>>>
    where
      Self: FromStr<Err = ParseError>,
    {
      /// Reads a decimal from a string in the syntax of [`Decimal::parse`], in every format. Any
      /// other input, such as a number, is rejected, so a decimal never passes through a binary
      /// floating-point value, with a fixed message that names nothing of the input; a string that
      /// does not parse reports its [`ParseError`](crate::ParseError).
      fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
      where
        D: serde_core::Deserializer<'de>,
      {
        crate::serde::deserialize_text(deserializer)
      }
    }

    #[cfg_attr(docsrs, doc(cfg($cfg)))]
    impl<I, F, B, const N: usize> FromStr for Scaled<Decimal<I, F, $integer<B, $pointer<[u64; N]>>>>
    where
      B: FillsRepr<[u64; N]>,
      $integer<B, [u64; N]>: InlineStorage,
      Scaled<Decimal<I, F, $integer<B, [u64; N]>>>: FromStr<Err = ParseError>,
    {
      type Err = ParseError;

      /// Parses a decimal and the scale of its text, as the `Scaled` over the twin does, and
      /// stores the decimal: nothing is built for an error.
      #[inline]
      fn from_str(s: &str) -> Result<Self, ParseError> {
        let scaled: Scaled<Decimal<I, F, $integer<B, [u64; N]>>> = s.parse()?;
        let (value, scale) = scaled.into_parts();
        Ok(Scaled::new(Decimal::from_inline(value), scale))
      }
    }

    // The operators with one owned operand store into that operand, as those on two values store
    // into the first: a `Box` in its allocation, and a shared pointer in a new allocation.
    #[cfg_attr(docsrs, doc(cfg($cfg)))]
    impl<I, F, B, const N: usize> Add<&Decimal<I, F, $integer<B, $pointer<[u64; N]>>>>
      for Decimal<I, F, $integer<B, $pointer<[u64; N]>>>
    where
      B: FillsRepr<[u64; N]>,
      $integer<B, [u64; N]>: InlineStorage,
      Decimal<I, F, $integer<B, [u64; N]>>: FixedDecimal,
    {
      type Output = Self;

      /// Adds `rhs`, by reference, to `self`, and stores the sum into `self`: in its allocation for
      /// a `Box`, and in a new allocation for a shared pointer.
      ///
      /// # Panics
      ///
      /// Panics if the sum is outside the range of the type, in every build, whatever
      /// `overflow-checks` is. [`checked_add`](FixedDecimal::checked_add) returns `None` instead.
      #[inline]
      #[track_caller]
      fn add(mut self, rhs: &Self) -> Self {
        let sum = match FixedDecimal::checked_add(&self.to_inline(), &rhs.to_inline()) {
          Some(sum) => sum,
          None => overflow::add(),
        };
        StorageCore::set_twin(&mut self.repr, sum.repr, Token::new());
        self
      }
    }

    #[cfg_attr(docsrs, doc(cfg($cfg)))]
    impl<I, F, B, const N: usize> Add<Decimal<I, F, $integer<B, $pointer<[u64; N]>>>>
      for &Decimal<I, F, $integer<B, $pointer<[u64; N]>>>
    where
      B: FillsRepr<[u64; N]>,
      $integer<B, [u64; N]>: InlineStorage,
      Decimal<I, F, $integer<B, [u64; N]>>: FixedDecimal,
    {
      type Output = Decimal<I, F, $integer<B, $pointer<[u64; N]>>>;

      /// Adds `rhs` to `self`, by reference, and stores the sum into `rhs`: in its allocation for
      /// a `Box`, and in a new allocation for a shared pointer.
      ///
      /// # Panics
      ///
      /// Panics if the sum is outside the range of the type, in every build, whatever
      /// `overflow-checks` is. [`checked_add`](FixedDecimal::checked_add) returns `None` instead.
      #[inline]
      #[track_caller]
      fn add(self, mut rhs: Decimal<I, F, $integer<B, $pointer<[u64; N]>>>) -> Self::Output {
        let sum = match FixedDecimal::checked_add(&self.to_inline(), &rhs.to_inline()) {
          Some(sum) => sum,
          None => overflow::add(),
        };
        StorageCore::set_twin(&mut rhs.repr, sum.repr, Token::new());
        rhs
      }
    }

    #[cfg_attr(docsrs, doc(cfg($cfg)))]
    impl<I, F, B, const N: usize> Sub<&Decimal<I, F, $integer<B, $pointer<[u64; N]>>>>
      for Decimal<I, F, $integer<B, $pointer<[u64; N]>>>
    where
      B: FillsRepr<[u64; N]>,
      $integer<B, [u64; N]>: InlineStorage,
      Decimal<I, F, $integer<B, [u64; N]>>: FixedDecimal,
    {
      type Output = Self;

      /// Subtracts `rhs`, by reference, from `self`, and stores the difference into `self`: in its
      /// allocation for a `Box`, and in a new allocation for a shared pointer.
      ///
      /// # Panics
      ///
      /// Panics if the difference is outside the range of the type, which for an unsigned storage
      /// is every difference below zero, in every build, whatever `overflow-checks` is.
      /// [`checked_sub`](FixedDecimal::checked_sub) returns `None` instead.
      #[inline]
      #[track_caller]
      fn sub(mut self, rhs: &Self) -> Self {
        let difference = match FixedDecimal::checked_sub(&self.to_inline(), &rhs.to_inline()) {
          Some(difference) => difference,
          None => overflow::sub(),
        };
        StorageCore::set_twin(&mut self.repr, difference.repr, Token::new());
        self
      }
    }

    // The difference is `self - rhs`, which `rhs`, the operand that the form owns, stores.
    #[cfg_attr(docsrs, doc(cfg($cfg)))]
    impl<I, F, B, const N: usize> Sub<Decimal<I, F, $integer<B, $pointer<[u64; N]>>>>
      for &Decimal<I, F, $integer<B, $pointer<[u64; N]>>>
    where
      B: FillsRepr<[u64; N]>,
      $integer<B, [u64; N]>: InlineStorage,
      Decimal<I, F, $integer<B, [u64; N]>>: FixedDecimal,
    {
      type Output = Decimal<I, F, $integer<B, $pointer<[u64; N]>>>;

      /// Subtracts `rhs` from `self`, by reference, and stores the difference into `rhs`: in its
      /// allocation for a `Box`, and in a new allocation for a shared pointer.
      ///
      /// # Panics
      ///
      /// Panics if the difference is outside the range of the type, which for an unsigned storage
      /// is every difference below zero, in every build, whatever `overflow-checks` is.
      /// [`checked_sub`](FixedDecimal::checked_sub) returns `None` instead.
      #[inline]
      #[track_caller]
      fn sub(self, mut rhs: Decimal<I, F, $integer<B, $pointer<[u64; N]>>>) -> Self::Output {
        let difference = match FixedDecimal::checked_sub(&self.to_inline(), &rhs.to_inline()) {
          Some(difference) => difference,
          None => overflow::sub(),
        };
        StorageCore::set_twin(&mut rhs.repr, difference.repr, Token::new());
        rhs
      }
    }

    // Each form of a `Scaled` is the same form of its decimal, so that it stores where that form
    // stores. The fields of `Scaled` are private to its module: its public items read them.
    #[cfg_attr(docsrs, doc(cfg($cfg)))]
    impl<I, F, B, const N: usize> Add<&Scaled<Decimal<I, F, $integer<B, $pointer<[u64; N]>>>>>
      for Scaled<Decimal<I, F, $integer<B, $pointer<[u64; N]>>>>
    where
      B: FillsRepr<[u64; N]>,
      $integer<B, [u64; N]>: InlineStorage,
      Decimal<I, F, $integer<B, [u64; N]>>: FixedDecimal,
      Self: FixedDecimal,
    {
      type Output = Self;

      /// Adds `rhs`, by reference, to `self`, the decimal of `rhs` to that of `self` as `+` does,
      /// which stores the sum into the decimal of `self`. The sum takes the larger of the two
      /// scales.
      ///
      /// # Panics
      ///
      /// Panics if the sum is outside the range of the decimal, in every build, whatever
      /// `overflow-checks` is. [`checked_add`](FixedDecimal::checked_add) returns `None` instead.
      #[inline]
      #[track_caller]
      fn add(self, rhs: &Self) -> Self {
        let scale = self.scale().max(rhs.scale());
        Self::new(self.into_value() + rhs.value(), scale)
      }
    }

    #[cfg_attr(docsrs, doc(cfg($cfg)))]
    impl<I, F, B, const N: usize> Add<Scaled<Decimal<I, F, $integer<B, $pointer<[u64; N]>>>>>
      for &Scaled<Decimal<I, F, $integer<B, $pointer<[u64; N]>>>>
    where
      B: FillsRepr<[u64; N]>,
      $integer<B, [u64; N]>: InlineStorage,
      Decimal<I, F, $integer<B, [u64; N]>>: FixedDecimal,
      Scaled<Decimal<I, F, $integer<B, $pointer<[u64; N]>>>>: FixedDecimal,
    {
      type Output = Scaled<Decimal<I, F, $integer<B, $pointer<[u64; N]>>>>;

      /// Adds `rhs` to `self`, by reference, the decimal of `rhs` to that of `self` as `+` does,
      /// which stores the sum into the decimal of `rhs`. The sum takes the larger of the two
      /// scales.
      ///
      /// # Panics
      ///
      /// Panics if the sum is outside the range of the decimal, in every build, whatever
      /// `overflow-checks` is. [`checked_add`](FixedDecimal::checked_add) returns `None` instead.
      #[inline]
      #[track_caller]
      fn add(self, rhs: Scaled<Decimal<I, F, $integer<B, $pointer<[u64; N]>>>>) -> Self::Output {
        let scale = self.scale().max(rhs.scale());
        Scaled::new(self.value() + rhs.into_value(), scale)
      }
    }

    #[cfg_attr(docsrs, doc(cfg($cfg)))]
    impl<I, F, B, const N: usize> Sub<&Scaled<Decimal<I, F, $integer<B, $pointer<[u64; N]>>>>>
      for Scaled<Decimal<I, F, $integer<B, $pointer<[u64; N]>>>>
    where
      B: FillsRepr<[u64; N]>,
      $integer<B, [u64; N]>: InlineStorage,
      Decimal<I, F, $integer<B, [u64; N]>>: FixedDecimal,
      Self: FixedDecimal,
    {
      type Output = Self;

      /// Subtracts `rhs`, by reference, from `self`, the decimal of `rhs` from that of `self` as
      /// `-` does, which stores the difference into the decimal of `self`. The difference takes
      /// the larger of the two scales.
      ///
      /// # Panics
      ///
      /// Panics if the difference is outside the range of the decimal, which for an unsigned
      /// storage is every difference below zero, in every build, whatever `overflow-checks` is.
      /// [`checked_sub`](FixedDecimal::checked_sub) returns `None` instead.
      #[inline]
      #[track_caller]
      fn sub(self, rhs: &Self) -> Self {
        let scale = self.scale().max(rhs.scale());
        Self::new(self.into_value() - rhs.value(), scale)
      }
    }

    #[cfg_attr(docsrs, doc(cfg($cfg)))]
    impl<I, F, B, const N: usize> Sub<Scaled<Decimal<I, F, $integer<B, $pointer<[u64; N]>>>>>
      for &Scaled<Decimal<I, F, $integer<B, $pointer<[u64; N]>>>>
    where
      B: FillsRepr<[u64; N]>,
      $integer<B, [u64; N]>: InlineStorage,
      Decimal<I, F, $integer<B, [u64; N]>>: FixedDecimal,
      Scaled<Decimal<I, F, $integer<B, $pointer<[u64; N]>>>>: FixedDecimal,
    {
      type Output = Scaled<Decimal<I, F, $integer<B, $pointer<[u64; N]>>>>;

      /// Subtracts `rhs` from `self`, by reference, the decimal of `rhs` from that of `self` as
      /// `-` does, which stores the difference into the decimal of `rhs`. The difference takes the
      /// larger of the two scales.
      ///
      /// # Panics
      ///
      /// Panics if the difference is outside the range of the decimal, which for an unsigned
      /// storage is every difference below zero, in every build, whatever `overflow-checks` is.
      /// [`checked_sub`](FixedDecimal::checked_sub) returns `None` instead.
      #[inline]
      #[track_caller]
      fn sub(self, rhs: Scaled<Decimal<I, F, $integer<B, $pointer<[u64; N]>>>>) -> Self::Output {
        let scale = self.scale().max(rhs.scale());
        Scaled::new(self.value() - rhs.into_value(), scale)
      }
    }
  };
  // A pointer that holds its coefficient in place has the assignments by reference, which store
  // into the allocation of their target. A shared pointer never writes the coefficient that it
  // holds, which another owner may read, so it has no assignment.
  (@mutable in_place, $integer:ident, $pointer:ident, [$cfg:meta]) => {
    #[cfg_attr(docsrs, doc(cfg($cfg)))]
    impl<I, F, B, const N: usize> AddAssign<&Decimal<I, F, $integer<B, $pointer<[u64; N]>>>>
      for Decimal<I, F, $integer<B, $pointer<[u64; N]>>>
    where
      B: FillsRepr<[u64; N]>,
      $integer<B, [u64; N]>: InlineStorage,
      Decimal<I, F, $integer<B, [u64; N]>>: FixedDecimal,
    {
      /// Adds `rhs`, by reference, to `self`, in the allocation of `self`.
      ///
      /// # Panics
      ///
      /// Panics if the sum is outside the range of the type, in every build, whatever
      /// `overflow-checks` is, and leaves `self` unchanged.
      /// [`checked_add`](FixedDecimal::checked_add) returns `None` instead.
      #[inline]
      #[track_caller]
      fn add_assign(&mut self, rhs: &Self) {
        match FixedDecimal::checked_add(&self.to_inline(), &rhs.to_inline()) {
          Some(sum) => StorageCore::set_twin(&mut self.repr, sum.repr, Token::new()),
          None => overflow::add(),
        }
      }
    }

    #[cfg_attr(docsrs, doc(cfg($cfg)))]
    impl<I, F, B, const N: usize> SubAssign<&Decimal<I, F, $integer<B, $pointer<[u64; N]>>>>
      for Decimal<I, F, $integer<B, $pointer<[u64; N]>>>
    where
      B: FillsRepr<[u64; N]>,
      $integer<B, [u64; N]>: InlineStorage,
      Decimal<I, F, $integer<B, [u64; N]>>: FixedDecimal,
    {
      /// Subtracts `rhs`, by reference, from `self`, in the allocation of `self`.
      ///
      /// # Panics
      ///
      /// Panics if the difference is outside the range of the type, which for an unsigned storage
      /// is every difference below zero, in every build, whatever `overflow-checks` is, and
      /// leaves `self` unchanged. [`checked_sub`](FixedDecimal::checked_sub) returns `None`
      /// instead.
      #[inline]
      #[track_caller]
      fn sub_assign(&mut self, rhs: &Self) {
        match FixedDecimal::checked_sub(&self.to_inline(), &rhs.to_inline()) {
          Some(difference) => StorageCore::set_twin(&mut self.repr, difference.repr, Token::new()),
          None => overflow::sub(),
        }
      }
    }
  };
  (@mutable shared, $integer:ident, $pointer:ident, [$cfg:meta]) => {};
}

#[cfg(any(feature = "std", feature = "alloc"))]
heap_decimals!(Box, in_place, [any(feature = "std", feature = "alloc")]);
#[cfg(any(feature = "std", feature = "alloc"))]
heap_decimals!(Rc, shared, [any(feature = "std", feature = "alloc")]);
#[cfg(all(any(feature = "std", feature = "alloc"), target_has_atomic = "ptr"))]
heap_decimals!(
  Arc,
  shared,
  [all(
    any(feature = "std", feature = "alloc"),
    target_has_atomic = "ptr"
  )]
);
#[cfg(all(feature = "triomphe", target_has_atomic = "ptr"))]
heap_decimals!(
  TriompheArc,
  shared,
  [all(feature = "triomphe", target_has_atomic = "ptr")]
);
