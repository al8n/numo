//! The heap storages of [`Uint`](crate::Uint) and [`Int`](crate::Int) and of their non-zero
//! integers: an array of limbs `[L; N]` in a `Box`, with the `std` or the `alloc` feature.
//!
//! The array is the inline storage of the pointer, `Inline`, whose rows compute every value. Each
//! row of a heap storage lends the arrays of its operands to the row of the same name of the
//! array, through `Borrow`, and puts each array that it returns in an allocation of its own, as
//! `from_inline` does. The exceptions are zero, which is built in its allocation, the conversions
//! with the non-zero storage, which is the pointer itself, and the casts between the signs, which
//! store their result in the allocation that they consume.

use core::{
  borrow::{Borrow, BorrowMut},
  cmp::Ordering,
  fmt,
  hash::Hasher,
  num::ParseIntError,
};
use std::{boxed::Box, vec};

use super::{
  BitsFill, BitsOf, InlineIntRepr, InlineUintRepr, IntCore, IntRepr, IntReprMut, MutableRepr,
  NonZeroIntRepr, NonZeroRepr, NonZeroUintRepr, ReprLimb, UintCore, UintRepr, UintReprMut, View,
};
use crate::{
  FromFloatError, Int, RoundingMode, Uint,
  int::float::{FloatFormat, FloatParts},
  layout::Token,
};

/// A limb type of an array storage, `u8`, `u16`, `u32`, `u64` or `u128`, whose zero builds the zero
/// of a heap storage in its allocation.
#[diagnostic::on_unimplemented(
  message = "`{Self}` is not a limb type of an array storage",
  label = "not a limb type",
  note = "the limb types are `u8`, `u16`, `u32`, `u64` and `u128`"
)]
pub trait ArrayLimb: Copy {
  /// Zero.
  const ZERO: Self;
}

/// An array of limbs, `[u8; N]` to `[u128; N]`, the inline storage of a heap storage. The
/// supertraits are what the rows of a heap storage call: the storage traits of the array, whose
/// associated storages are the array itself, as those of the pointer are the pointer.
// The zero of the limb comes from `ArrayLimb`, not from a supertrait `ReprLimb` of this trait,
// under which `<[L; N] as ReprLimb>::Limb` would no longer normalize to `L` in code bounded by
// `[L; N]: ArrayRepr`.
#[diagnostic::on_unimplemented(
  message = "`{Self}` is not an array of limbs",
  note = "a heap storage holds an array `[u8; N]` to `[u128; N]`"
)]
pub trait ArrayRepr:
  InlineUintRepr
  + InlineIntRepr
  + UintRepr<Signed = Self, NonZero = Self>
  + IntRepr<Unsigned = Self, NonZero = Self>
  + UintCore<Inline = Self, Complement = Self, NonZeroStorage = Self>
  + IntCore<Inline = Self, Magnitude = Self, NonZeroStorage = Self>
{
}

/// Implements [`ArrayLimb`] for each limb type, and [`ArrayRepr`] for its arrays.
macro_rules! array_limbs {
  ($($limb:ident),+ $(,)?) => {$(
    impl ArrayLimb for $limb {
      const ZERO: Self = 0;
    }

    impl<const N: usize> ArrayRepr for [$limb; N] {}
  )+};
}

array_limbs!(u8, u16, u32, u64, u128);

/// The zero of `N` limbs in a `Box`, built in its allocation: `vec!` of zero limbs takes zeroed
/// memory, and the conversion of the boxed slice into the boxed array keeps the allocation, so no
/// array of the limbs is on the stack.
fn boxed_zero<L: ArrayLimb, const N: usize>() -> Box<[L; N]> {
  match vec![L::ZERO; N].into_boxed_slice().try_into() {
    Ok(zero) => zero,
    Err(_) => length(),
  }
}

/// Panics on a slice of limbs whose length is not that of its array: a bug of this crate, as the
/// slice is made with the length of the array.
#[cold]
fn length() -> ! {
  panic!("a slice of limbs does not have the length of its array")
}

/// The unsigned storage of `value`, the array that a row returns, in an allocation of its own: the
/// `from_inline` of [`UintCore`].
#[inline(always)]
fn uint<P: UintCore>(value: P::Inline) -> P {
  P::from_inline(value, Token::new())
}

/// The signed storage of `value`, the array that a row returns, in an allocation of its own: the
/// `from_inline` of [`IntCore`].
#[inline(always)]
fn int<P: IntCore>(value: P::Inline) -> P {
  P::from_inline(value, Token::new())
}

/// Emits the rows that the storage cores of a heap storage of the array `[L; N]` share, written
/// `Core, new, zero`, where `new` puts an array that a row returns in an allocation of its own and
/// `zero` builds zero in its allocation. Each row lends the arrays of the receiver and of every
/// operand to the row of the same name of the array, and passes its result on with each array in
/// it put in an allocation of its own.
macro_rules! heap_rows {
  ($core:ident, $new:ident, $zero:ident) => {
    #[inline(always)]
    fn inline(&self, _: Token) -> &[L; N] {
      self.borrow()
    }

    #[inline]
    fn from_inline(value: [L; N], _: Token) -> Self {
      Self::new(value)
    }

    #[inline]
    fn zero(_: Token) -> Self {
      $zero()
    }

    #[inline]
    fn compare(&self, other: &Self, token: Token) -> Ordering {
      <[L; N] as $core>::compare(self.borrow(), other.borrow(), token)
    }

    #[inline]
    fn compare_u128(&self, rhs: u128, token: Token) -> Ordering {
      <[L; N] as $core>::compare_u128(self.borrow(), rhs, token)
    }

    #[inline]
    fn compare_i128(&self, rhs: i128, token: Token) -> Ordering {
      <[L; N] as $core>::compare_i128(self.borrow(), rhs, token)
    }

    #[inline]
    fn hash_limbs<H: Hasher>(&self, bits: u32, state: &mut H, token: Token) {
      <[L; N] as $core>::hash_limbs(self.borrow(), bits, state, token);
    }

    #[inline]
    fn fmt_decimal(&self, f: &mut fmt::Formatter<'_>, token: Token) -> fmt::Result {
      <[L; N] as $core>::fmt_decimal(self.borrow(), f, token)
    }

    #[inline]
    fn fmt_exp(&self, upper: bool, f: &mut fmt::Formatter<'_>, token: Token) -> fmt::Result {
      <[L; N] as $core>::fmt_exp(self.borrow(), upper, f, token)
    }

    #[inline]
    fn from_str_radix_bits(
      src: &[u8],
      radix: u32,
      bits: u32,
      token: Token,
    ) -> Result<Self, ParseIntError> {
      <[L; N] as $core>::from_str_radix_bits(src, radix, bits, token).map($new)
    }

    #[inline]
    fn write_bytes(&self, out: &mut [u8], big_endian: bool, token: Token) {
      <[L; N] as $core>::write_bytes(self.borrow(), out, big_endian, token);
    }

    #[inline]
    fn checked_read_bytes(bytes: &[u8], big_endian: bool, bits: u32, token: Token) -> Option<Self> {
      <[L; N] as $core>::checked_read_bytes(bytes, big_endian, bits, token).map($new)
    }

    #[inline]
    fn wrapping_read_bytes(bytes: &[u8], big_endian: bool, bits: u32, token: Token) -> Self {
      $new(<[L; N] as $core>::wrapping_read_bytes(
        bytes, big_endian, bits, token,
      ))
    }

    #[inline]
    fn saturating_read_bytes(bytes: &[u8], big_endian: bool, bits: u32, token: Token) -> Self {
      $new(<[L; N] as $core>::saturating_read_bytes(
        bytes, big_endian, bits, token,
      ))
    }

    #[inline]
    fn byte_at(&self, index: usize, token: Token) -> u8 {
      <[L; N] as $core>::byte_at(self.borrow(), index, token)
    }

    #[inline]
    fn swap_bytes_bits(&self, bits: u32, token: Token) -> Self {
      $new(<[L; N] as $core>::swap_bytes_bits(
        self.borrow(),
        bits,
        token,
      ))
    }

    #[inline]
    fn is_zero(&self, token: Token) -> bool {
      <[L; N] as $core>::is_zero(self.borrow(), token)
    }

    #[inline]
    fn fits_bits(&self, bits: u32, token: Token) -> bool {
      <[L; N] as $core>::fits_bits(self.borrow(), bits, token)
    }

    #[inline]
    fn overflowing_add_bits(&self, rhs: &Self, bits: u32, token: Token) -> (Self, bool) {
      let (value, overflow) =
        <[L; N] as $core>::overflowing_add_bits(self.borrow(), rhs.borrow(), bits, token);
      ($new(value), overflow)
    }

    #[inline]
    fn overflowing_sub_bits(&self, rhs: &Self, bits: u32, token: Token) -> (Self, bool) {
      let (value, overflow) =
        <[L; N] as $core>::overflowing_sub_bits(self.borrow(), rhs.borrow(), bits, token);
      ($new(value), overflow)
    }

    #[inline]
    fn overflowing_mul_bits(&self, rhs: &Self, bits: u32, token: Token) -> (Self, bool) {
      let (value, overflow) =
        <[L; N] as $core>::overflowing_mul_bits(self.borrow(), rhs.borrow(), bits, token);
      ($new(value), overflow)
    }

    #[inline]
    fn overflowing_neg_bits(&self, bits: u32, token: Token) -> (Self, bool) {
      let (value, overflow) = <[L; N] as $core>::overflowing_neg_bits(self.borrow(), bits, token);
      ($new(value), overflow)
    }

    #[inline]
    fn is_one(&self, token: Token) -> bool {
      <[L; N] as $core>::is_one(self.borrow(), token)
    }

    #[inline]
    fn div_rem_bits(&self, rhs: &Self, bits: u32, token: Token) -> Option<(Self, Self, bool)> {
      <[L; N] as $core>::div_rem_bits(self.borrow(), rhs.borrow(), bits, token)
        .map(|(quotient, remainder, overflow)| ($new(quotient), $new(remainder), overflow))
    }

    #[inline]
    fn div_rem_euclid_bits(
      &self,
      rhs: &Self,
      bits: u32,
      token: Token,
    ) -> Option<(Self, Self, bool)> {
      <[L; N] as $core>::div_rem_euclid_bits(self.borrow(), rhs.borrow(), bits, token)
        .map(|(quotient, remainder, overflow)| ($new(quotient), $new(remainder), overflow))
    }

    #[inline]
    fn div_floor_bits(&self, rhs: &Self, bits: u32, token: Token) -> Option<(Self, bool)> {
      <[L; N] as $core>::div_floor_bits(self.borrow(), rhs.borrow(), bits, token)
        .map(|(quotient, overflow)| ($new(quotient), overflow))
    }

    #[inline]
    fn div_ceil_bits(&self, rhs: &Self, bits: u32, token: Token) -> Option<(Self, bool)> {
      <[L; N] as $core>::div_ceil_bits(self.borrow(), rhs.borrow(), bits, token)
        .map(|(quotient, overflow)| ($new(quotient), overflow))
    }

    #[inline]
    fn next_multiple_of_bits(&self, rhs: &Self, bits: u32, token: Token) -> Option<(Self, bool)> {
      <[L; N] as $core>::next_multiple_of_bits(self.borrow(), rhs.borrow(), bits, token)
        .map(|(multiple, overflow)| ($new(multiple), overflow))
    }

    #[inline]
    fn checked_div_exact_bits(&self, rhs: &Self, bits: u32, token: Token) -> Option<Self> {
      <[L; N] as $core>::checked_div_exact_bits(self.borrow(), rhs.borrow(), bits, token).map($new)
    }

    #[inline]
    fn overflowing_pow_bits(&self, exp: u32, bits: u32, token: Token) -> (Self, bool) {
      let (value, overflow) =
        <[L; N] as $core>::overflowing_pow_bits(self.borrow(), exp, bits, token);
      ($new(value), overflow)
    }

    #[inline]
    fn checked_isqrt(&self, token: Token) -> Option<Self> {
      <[L; N] as $core>::checked_isqrt(self.borrow(), token).map($new)
    }

    #[inline]
    fn checked_nth_root(&self, n: u32, token: Token) -> Option<Self> {
      <[L; N] as $core>::checked_nth_root(self.borrow(), n, token).map($new)
    }

    #[inline]
    fn checked_ilog(&self, base: &Self, token: Token) -> Option<u32> {
      <[L; N] as $core>::checked_ilog(self.borrow(), base.borrow(), token)
    }

    #[inline]
    fn checked_ilog2(&self, token: Token) -> Option<u32> {
      <[L; N] as $core>::checked_ilog2(self.borrow(), token)
    }

    #[inline]
    fn checked_ilog10(&self, token: Token) -> Option<u32> {
      <[L; N] as $core>::checked_ilog10(self.borrow(), token)
    }

    #[inline]
    fn midpoint(&self, rhs: &Self, token: Token) -> Self {
      $new(<[L; N] as $core>::midpoint(
        self.borrow(),
        rhs.borrow(),
        token,
      ))
    }

    #[inline]
    fn and(&self, rhs: &Self, token: Token) -> Self {
      $new(<[L; N] as $core>::and(self.borrow(), rhs.borrow(), token))
    }

    #[inline]
    fn or(&self, rhs: &Self, token: Token) -> Self {
      $new(<[L; N] as $core>::or(self.borrow(), rhs.borrow(), token))
    }

    #[inline]
    fn xor(&self, rhs: &Self, token: Token) -> Self {
      $new(<[L; N] as $core>::xor(self.borrow(), rhs.borrow(), token))
    }

    #[inline]
    fn not_bits(&self, bits: u32, token: Token) -> Self {
      $new(<[L; N] as $core>::not_bits(self.borrow(), bits, token))
    }

    #[inline]
    fn count_ones_bits(&self, bits: u32, token: Token) -> u32 {
      <[L; N] as $core>::count_ones_bits(self.borrow(), bits, token)
    }

    #[inline]
    fn leading_zeros_bits(&self, bits: u32, token: Token) -> u32 {
      <[L; N] as $core>::leading_zeros_bits(self.borrow(), bits, token)
    }

    #[inline]
    fn trailing_zeros_bits(&self, bits: u32, token: Token) -> u32 {
      <[L; N] as $core>::trailing_zeros_bits(self.borrow(), bits, token)
    }

    #[inline]
    fn leading_ones_bits(&self, bits: u32, token: Token) -> u32 {
      <[L; N] as $core>::leading_ones_bits(self.borrow(), bits, token)
    }

    #[inline]
    fn trailing_ones_bits(&self, bits: u32, token: Token) -> u32 {
      <[L; N] as $core>::trailing_ones_bits(self.borrow(), bits, token)
    }

    #[inline]
    fn highest_one_bits(&self, bits: u32, token: Token) -> Option<u32> {
      <[L; N] as $core>::highest_one_bits(self.borrow(), bits, token)
    }

    #[inline]
    fn lowest_one(&self, token: Token) -> Option<u32> {
      <[L; N] as $core>::lowest_one(self.borrow(), token)
    }

    #[inline]
    fn isolate_highest_one_bits(&self, bits: u32, token: Token) -> Self {
      $new(<[L; N] as $core>::isolate_highest_one_bits(
        self.borrow(),
        bits,
        token,
      ))
    }

    #[inline]
    fn isolate_lowest_one_bits(&self, bits: u32, token: Token) -> Self {
      $new(<[L; N] as $core>::isolate_lowest_one_bits(
        self.borrow(),
        bits,
        token,
      ))
    }

    #[inline]
    fn reverse_bits_bits(&self, bits: u32, token: Token) -> Self {
      $new(<[L; N] as $core>::reverse_bits_bits(
        self.borrow(),
        bits,
        token,
      ))
    }

    #[inline]
    fn rotate_left_bits(&self, n: u32, bits: u32, token: Token) -> Self {
      $new(<[L; N] as $core>::rotate_left_bits(
        self.borrow(),
        n,
        bits,
        token,
      ))
    }

    #[inline]
    fn rotate_right_bits(&self, n: u32, bits: u32, token: Token) -> Self {
      $new(<[L; N] as $core>::rotate_right_bits(
        self.borrow(),
        n,
        bits,
        token,
      ))
    }

    #[inline]
    fn overflowing_shl_bits(&self, rhs: u32, bits: u32, token: Token) -> (Self, bool) {
      let (value, overflow) =
        <[L; N] as $core>::overflowing_shl_bits(self.borrow(), rhs, bits, token);
      ($new(value), overflow)
    }

    #[inline]
    fn overflowing_shr_bits(&self, rhs: u32, bits: u32, token: Token) -> (Self, bool) {
      let (value, overflow) =
        <[L; N] as $core>::overflowing_shr_bits(self.borrow(), rhs, bits, token);
      ($new(value), overflow)
    }

    #[inline]
    fn unbounded_shl_bits(&self, rhs: u32, bits: u32, token: Token) -> Self {
      $new(<[L; N] as $core>::unbounded_shl_bits(
        self.borrow(),
        rhs,
        bits,
        token,
      ))
    }

    #[inline]
    fn unbounded_shr_bits(&self, rhs: u32, bits: u32, token: Token) -> Self {
      $new(<[L; N] as $core>::unbounded_shr_bits(
        self.borrow(),
        rhs,
        bits,
        token,
      ))
    }

    #[inline]
    fn shl_exact_bits(&self, rhs: u32, bits: u32, token: Token) -> Option<Self> {
      <[L; N] as $core>::shl_exact_bits(self.borrow(), rhs, bits, token).map($new)
    }

    #[inline]
    fn shr_exact_bits(&self, rhs: u32, bits: u32, token: Token) -> Option<Self> {
      <[L; N] as $core>::shr_exact_bits(self.borrow(), rhs, bits, token).map($new)
    }

    #[inline]
    fn bit(&self, index: u32, bits: u32, token: Token) -> bool {
      <[L; N] as $core>::bit(self.borrow(), index, bits, token)
    }

    #[inline]
    fn with_bit(&self, index: u32, set: bool, bits: u32, token: Token) -> Option<Self> {
      <[L; N] as $core>::with_bit(self.borrow(), index, set, bits, token).map($new)
    }

    // The pointer is its own non-zero storage, so the conversions with the non-zero storage pass it
    // on, test it and lend it, and never allocate.
    #[inline]
    fn to_nonzero(self, token: Token) -> Option<Self> {
      if <[L; N] as $core>::is_zero(self.borrow(), token) {
        None
      } else {
        Some(self)
      }
    }

    #[inline]
    fn from_nonzero(nonzero: Self, _: Token) -> Self {
      nonzero
    }

    #[inline]
    fn with_nonzero<T>(nonzero: &Self, f: impl FnOnce(&Self) -> T, _: Token) -> T {
      f(nonzero)
    }

    #[inline]
    fn known_nonzero(self, token: Token) -> Self {
      debug_assert!(
        !<[L; N] as $core>::is_zero(self.borrow(), token),
        "a non-zero result is zero"
      );
      self
    }

    #[inline]
    fn word(&self, index: usize, token: Token) -> u64 {
      <[L; N] as $core>::word(self.borrow(), index, token)
    }

    #[inline]
    fn from_words(word: impl Fn(usize) -> u64, bits: u32, token: Token) -> Self {
      $new(<[L; N] as $core>::from_words(word, bits, token))
    }

    #[inline]
    fn float_bits(&self, format: FloatFormat, token: Token) -> u64 {
      <[L; N] as $core>::float_bits(self.borrow(), format, token)
    }

    #[inline]
    fn from_float(
      parts: FloatParts,
      mode: RoundingMode,
      bits: u32,
      token: Token,
    ) -> Result<Self, FromFloatError> {
      <[L; N] as $core>::from_float(parts, mode, bits, token).map($new)
    }

    // The readers of limbs leave out the clause `where Self: ReprLimb` of the trait, as those of
    // the arrays do: under it, `<Self as ReprLimb>::Limb` would not normalize to `L`.
    #[inline]
    fn checked_read_limbs(
      limbs: &[<Self as ReprLimb>::Limb],
      bits: u32,
      token: Token,
    ) -> Option<Self> {
      <[L; N] as $core>::checked_read_limbs(limbs, bits, token).map($new)
    }

    #[inline]
    fn wrapping_read_limbs(limbs: &[<Self as ReprLimb>::Limb], bits: u32, token: Token) -> Self {
      $new(<[L; N] as $core>::wrapping_read_limbs(limbs, bits, token))
    }

    #[inline]
    fn saturating_read_limbs(limbs: &[<Self as ReprLimb>::Limb], bits: u32, token: Token) -> Self {
      $new(<[L; N] as $core>::saturating_read_limbs(limbs, bits, token))
    }
  };
}

/// Implements the storage traits of [`Uint`], [`Int`] and their non-zero integers, the storage
/// cores and the width relations, for the arrays of limbs in the heap pointer `$pointer`, written
/// `pointer, zero`, where the pointer holds its array in place and writes it through `BorrowMut`,
/// and `zero` builds zero in its allocation. Each storage core delegates to the array through
/// `heap_rows!`, and the rows that differ between the cores are written for each.
macro_rules! heap_storage {
  ($pointer:ident, $zero:ident) => {
    // The width relations of the array, so that generic code bounded by `B: FitsBits<[L; N]>` has
    // `B: FitsBits<$pointer<[L; N]>>`, and a width that the array does not hold reports the error
    // of the array.
    impl<B, L, const N: usize> BitsOf<$pointer<[L; N]>> for B where B: BitsOf<[L; N]> {}

    impl<B, L, const N: usize> BitsFill<$pointer<[L; N]>> for B where B: BitsFill<[L; N]> {}

    impl<L, const N: usize> ReprLimb for $pointer<[L; N]> {
      type Limb = L;
    }

    #[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
    impl<L: ArrayLimb, const N: usize> UintRepr for $pointer<[L; N]>
    where
      [L; N]: ArrayRepr,
    {
      type Signed = Self;
      type NonZero = Self;
    }

    impl<L: ArrayLimb, const N: usize> UintCore for $pointer<[L; N]>
    where
      [L; N]: ArrayRepr,
    {
      type Complement = Self;
      type NonZeroStorage = Self;
      type Inline = [L; N];

      heap_rows!(UintCore, uint, $zero);

      #[inline]
      fn set_inline(&mut self, value: [L; N], _: Token) {
        *BorrowMut::<[L; N]>::borrow_mut(self) = value;
      }

      #[inline]
      fn fmt_pow2(
        &self,
        shift: u32,
        upper: bool,
        f: &mut fmt::Formatter<'_>,
        token: Token,
      ) -> fmt::Result {
        <[L; N] as UintCore>::fmt_pow2(self.borrow(), shift, upper, f, token)
      }

      #[inline]
      fn abs_diff(&self, rhs: &Self, token: Token) -> Self {
        uint(<[L; N] as UintCore>::abs_diff(
          self.borrow(),
          rhs.borrow(),
          token,
        ))
      }

      #[inline]
      fn is_multiple_of(&self, rhs: &Self, token: Token) -> bool {
        <[L; N] as UintCore>::is_multiple_of(self.borrow(), rhs.borrow(), token)
      }

      #[inline]
      fn next_power_of_two_bits(&self, bits: u32, token: Token) -> (Self, bool) {
        let (value, overflow) =
          <[L; N] as UintCore>::next_power_of_two_bits(self.borrow(), bits, token);
        (uint(value), overflow)
      }

      // The signed storage is the pointer, so the cast stores its result in the allocation that
      // it consumes.
      #[inline]
      fn cast_signed_bits(mut self, bits: u32, token: Token) -> Self {
        let array: &mut [L; N] = self.borrow_mut();
        *array = <[L; N] as UintCore>::cast_signed_bits(*array, bits, token);
        self
      }

      #[inline]
      fn lend_nonzero<B>(nonzero: &Uint<B, Self>, _: Token) -> View<'_, Uint<B, Self>> {
        View::Lent(nonzero)
      }
    }

    #[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
    impl<L: ArrayLimb, const N: usize> IntRepr for $pointer<[L; N]>
    where
      [L; N]: ArrayRepr,
    {
      type Unsigned = Self;
      type NonZero = Self;
    }

    impl<L: ArrayLimb, const N: usize> IntCore for $pointer<[L; N]>
    where
      [L; N]: ArrayRepr,
    {
      type Magnitude = Self;
      type NonZeroStorage = Self;
      type Inline = [L; N];

      heap_rows!(IntCore, int, $zero);

      #[inline]
      fn set_inline(&mut self, value: [L; N], _: Token) {
        *BorrowMut::<[L; N]>::borrow_mut(self) = value;
      }

      #[inline]
      fn fmt_pow2(
        &self,
        bits: u32,
        shift: u32,
        upper: bool,
        f: &mut fmt::Formatter<'_>,
        token: Token,
      ) -> fmt::Result {
        <[L; N] as IntCore>::fmt_pow2(self.borrow(), bits, shift, upper, f, token)
      }

      #[inline]
      fn is_negative(&self, token: Token) -> bool {
        <[L; N] as IntCore>::is_negative(self.borrow(), token)
      }

      #[inline]
      fn is_positive(&self, token: Token) -> bool {
        <[L; N] as IntCore>::is_positive(self.borrow(), token)
      }

      #[inline]
      fn signum(&self, token: Token) -> Self {
        int(<[L; N] as IntCore>::signum(self.borrow(), token))
      }

      // The magnitudes are unsigned storages, which are the pointer too.
      #[inline]
      fn unsigned_abs(&self, token: Token) -> Self {
        uint(<[L; N] as IntCore>::unsigned_abs(self.borrow(), token))
      }

      #[inline]
      fn abs_diff(&self, rhs: &Self, token: Token) -> Self {
        uint(<[L; N] as IntCore>::abs_diff(
          self.borrow(),
          rhs.borrow(),
          token,
        ))
      }

      // The unsigned storage is the pointer, so the cast stores its result in the allocation that
      // it consumes.
      #[inline]
      fn cast_unsigned_bits(mut self, bits: u32, token: Token) -> Self {
        let array: &mut [L; N] = self.borrow_mut();
        *array = <[L; N] as IntCore>::cast_unsigned_bits(*array, bits, token);
        self
      }

      #[inline]
      fn lend_nonzero<B>(nonzero: &Int<B, Self>, _: Token) -> View<'_, Int<B, Self>> {
        View::Lent(nonzero)
      }
    }

    impl<L: ArrayLimb, const N: usize> NonZeroRepr for $pointer<[L; N]> where [L; N]: ArrayRepr {}

    #[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
    impl<L: ArrayLimb, const N: usize> NonZeroUintRepr for $pointer<[L; N]>
    where
      [L; N]: ArrayRepr,
    {
      type Integer = Self;
      type Signed = Self;
    }

    #[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
    impl<L: ArrayLimb, const N: usize> NonZeroIntRepr for $pointer<[L; N]>
    where
      [L; N]: ArrayRepr,
    {
      type Integer = Self;
      type Unsigned = Self;
    }

    impl<L: ArrayLimb, const N: usize> MutableRepr for $pointer<[L; N]> where [L; N]: ArrayRepr {}

    #[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
    impl<L: ArrayLimb, const N: usize> UintReprMut for $pointer<[L; N]> where [L; N]: ArrayRepr {}

    #[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
    impl<L: ArrayLimb, const N: usize> IntReprMut for $pointer<[L; N]> where [L; N]: ArrayRepr {}
  };
}

heap_storage!(Box, boxed_zero);
