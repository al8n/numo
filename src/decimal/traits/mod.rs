//! The traits that [`Decimal`] implements for generic code.
//!
//! Each trait mirrors inherent items of the same names, which concrete types reach without the
//! trait. [`FixedDecimal`], the sign traits and [`HasDecimalOne`] have one implementation for each
//! storage, which `storage_decimal!` emits next to the inherent items.

use core::{
  fmt::{Debug, Display},
  hash::Hash,
  ops::Neg,
  str::FromStr,
};

use sealed::Sealed;

use super::{Decimal, ParseError, QuantizeError};
use crate::{layout::Storage, rounding::RoundingMode};

mod sealed {
  /// Seals the decimal traits: only [`Decimal`](crate::Decimal) implements them.
  pub trait Sealed {}

  impl<I, F, R> Sealed for super::Decimal<I, F, R> {}
}

/// An exact fixed-point decimal number: [`Decimal`] over any storage.
///
/// The constants and methods are the inherent items of the same names, which a concrete type
/// reaches without this trait, also in constant contexts. Generic code reaches them through this
/// trait, which `Decimal<I, F, R>` implements for each storage `R` where `(I, F): Layout<R>`; a
/// method call needs the trait in scope. Because the implementations are per storage, code
/// generic over the storage adds the bound `Decimal<I, F, R>: FixedDecimal`, which `R: Storage`
/// and `(I, F): Layout<R>` do not imply. The supertraits give generic code comparison, hashing,
/// formatting, parsing through `FromStr` and `Default`, which is zero.
///
/// ```
/// use numo::{
///   Decimal, FixedDecimal,
///   typenum::{U2, U8, U10},
/// };
///
/// // The sum of the values, or `None` if it is outside the range of the type.
/// fn total<T: FixedDecimal>(values: &[T]) -> Option<T> {
///   values.iter().try_fold(T::ZERO, |sum, value| sum.checked_add(value))
/// }
///
/// // The text of a value with every fraction digit of its type.
/// fn padded<T: FixedDecimal>(value: &T) -> String {
///   let digits = T::FRACTION as usize;
///   format!("{value:.digits$}")
/// }
///
/// type Price = Decimal<U10, U8>;
/// type Fee = Decimal<U10, U2, u64>;
///
/// let prices: [Price; 2] = ["64250.5".parse()?, "0.25".parse()?];
/// assert_eq!(total(&prices), Some("64250.75".parse()?));
/// assert_eq!(total(&[Price::MAX, Price::MAX]), None);
/// assert_eq!(padded(&prices[1]), "0.25000000");
/// assert_eq!(padded(&"1.5".parse::<Fee>()?), "1.50");
/// assert!(!<Fee as FixedDecimal>::SIGNED);
/// # Ok::<(), numo::ParseError>(())
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait FixedDecimal:
  Sealed + Copy + Eq + Ord + Hash + Debug + Display + Default + FromStr<Err = ParseError>
{
  /// The number of integral digits, `Integral`.
  const INTEGRAL: u32;

  /// The number of fraction digits, `Fraction`.
  const FRACTION: u32;

  /// The total number of digits, `Integral + Fraction`.
  const DIGITS: u32;

  /// `true` for a signed storage, whose values can be negative, and `false` for an unsigned one.
  const SIGNED: bool;

  /// Zero.
  const ZERO: Self;

  /// The largest value, `10^Integral - 10^-Fraction`: every digit is `9`.
  const MAX: Self;

  /// The smallest value: `-MAX` for a signed storage, and zero for an unsigned one.
  const MIN: Self;

  /// The storage of the coefficient, `Repr`.
  type Repr: Storage;

  /// Returns the coefficient: the stored integer `c` such that the value is `c / 10^Fraction`.
  #[must_use]
  fn repr(self) -> Self::Repr;

  /// Creates a value from its coefficient `repr`, meaning `repr / 10^Fraction`, or returns `None`
  /// if `repr` is outside the range of the type.
  #[must_use]
  fn from_repr(repr: Self::Repr) -> Option<Self>;

  /// Returns `true` if the value is zero.
  #[must_use]
  fn is_zero(&self) -> bool;

  /// Returns `true` if the value is greater than zero.
  #[must_use]
  fn is_positive(&self) -> bool;

  /// Adds two values, returning `None` if the sum is outside the range of the type.
  #[must_use]
  fn checked_add(&self, rhs: &Self) -> Option<Self>;

  /// Subtracts `rhs` from `self`, returning `None` if the difference is outside the range of the
  /// type.
  #[must_use]
  fn checked_sub(&self, rhs: &Self) -> Option<Self>;

  /// Rounds to a multiple of `step` with `mode`, as [`Decimal::quantize`] describes.
  ///
  /// # Errors
  ///
  /// Returns [`QuantizeError::InvalidStep`] if `step` is zero or negative, and
  /// [`QuantizeError::OutOfRange`] if the selected multiple is outside the range of the type.
  #[must_use = "this returns the rounded value, without modifying the original"]
  fn quantize(&self, step: &Self, mode: RoundingMode) -> Result<Self, QuantizeError>;

  /// Returns `true` if `self` is a multiple of `step`, as [`Decimal::is_multiple_of`] describes.
  #[must_use]
  fn is_multiple_of(&self, step: &Self) -> bool;
}

/// A decimal number with a signed storage: [`Decimal`] over `i8`, `i16`, `i32`, `i64`, `i128`,
/// [`i192`](crate::i192), [`i256`](crate::i256), [`i320`](crate::i320), [`i384`](crate::i384),
/// [`i448`](crate::i448) or [`i512`](crate::i512).
///
/// The methods are the inherent methods of the same names, which a concrete type reaches without
/// this trait, also in constant contexts. The range of a signed storage is symmetric, so negation
/// never fails, and the supertrait `Neg` gives generic code the operator `-`. On a value, `-x` and
/// `x.neg()` call `Neg::neg`, while `x.neg()` on a reference calls [`neg`](Self::neg), which takes
/// a reference; both return the same value.
///
/// ```
/// use numo::{
///   Decimal, SignedFixedDecimal,
///   typenum::{U8, U10},
/// };
///
/// fn negatives<T: SignedFixedDecimal>(values: &[T]) -> usize {
///   values.iter().filter(|value| value.is_negative()).count()
/// }
///
/// // The distance between two values, or `None` if it is outside the range of the type.
/// fn distance<T: SignedFixedDecimal>(a: &T, b: &T) -> Option<T> {
///   a.checked_sub(b).map(|difference| difference.abs())
/// }
///
/// type Change = Decimal<U10, U8>;
///
/// let changes: [Change; 3] = ["-0.5".parse()?, "1.25".parse()?, Change::ZERO];
/// assert_eq!(negatives(&changes), 1);
/// assert_eq!(distance(&changes[0], &changes[1]), Some("1.75".parse()?));
/// assert_eq!(distance(&Change::MIN, &Change::MAX), None);
/// # Ok::<(), numo::ParseError>(())
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait SignedFixedDecimal: FixedDecimal + Neg<Output = Self> {
  /// Returns `true` if the value is less than zero.
  #[must_use]
  fn is_negative(&self) -> bool;

  /// Returns `-self`. Never fails, because the range is symmetric.
  #[must_use]
  fn neg(&self) -> Self;

  /// Returns the absolute value. Never fails, because the range is symmetric.
  #[must_use]
  fn abs(&self) -> Self;
}

/// A decimal number with an unsigned storage, whose values are never negative: [`Decimal`] over
/// `u8`, `u16`, `u32`, `u64`, `u128`, [`u192`](crate::u192), [`u256`](crate::u256),
/// [`u320`](crate::u320), [`u384`](crate::u384), [`u448`](crate::u448) or [`u512`](crate::u512).
///
/// This trait has no items of its own. As a bound, it states that values are never negative, so
/// that [`MIN`](FixedDecimal::MIN) is zero and [`checked_sub`](FixedDecimal::checked_sub) returns
/// `None` below zero.
///
/// ```
/// use numo::{
///   Decimal, UnsignedFixedDecimal,
///   typenum::{U8, U10},
/// };
///
/// // What remains of a budget after a cost, or `None` if the cost is larger.
/// fn remaining<T: UnsignedFixedDecimal>(budget: &T, cost: &T) -> Option<T> {
///   budget.checked_sub(cost)
/// }
///
/// type Amount = Decimal<U10, U8, u64>;
///
/// let budget: Amount = "100".parse()?;
/// assert_eq!(remaining(&budget, &"64.5".parse()?), Some("35.5".parse()?));
/// assert_eq!(remaining(&budget, &"100.01".parse()?), None);
/// # Ok::<(), numo::ParseError>(())
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait UnsignedFixedDecimal: FixedDecimal {}

/// A decimal number that holds one: [`Decimal`] with at least one integral digit, over any
/// storage.
///
/// `ONE` is the inherent constant of the same name, which a concrete type reaches without this
/// trait, also in constant contexts. One needs an integral digit, so a type whose `Integral` is
/// zero has no `ONE`, rather than one that is out of range. `Decimal<I, F, R>` implements this
/// trait for each storage `R` where `I: NonZero` and `(I, F): Layout<R>`. It is separate from the
/// integer trait [`HasOne`](crate::HasOne), which also requires `Product`.
///
/// ```
/// use numo::{
///   Decimal, HasDecimalOne,
///   typenum::{U2, U8, U10},
/// };
///
/// // The value plus one, or `None` if the sum is outside the range of the type.
/// fn successor<T: HasDecimalOne>(value: &T) -> Option<T> {
///   value.checked_add(&T::ONE)
/// }
///
/// type Price = Decimal<U10, U8>;
///
/// assert_eq!(successor(&"1.5".parse::<Price>()?), Some("2.5".parse()?));
/// assert_eq!(successor(&Price::MAX), None);
/// assert_eq!(<Decimal<U2, U2, u16> as HasDecimalOne>::ONE.to_string(), "1");
/// # Ok::<(), numo::ParseError>(())
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait HasDecimalOne: FixedDecimal {
  /// One.
  const ONE: Self;
}

#[cfg(test)]
mod tests;
