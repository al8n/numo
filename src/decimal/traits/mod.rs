//! The traits that [`Decimal`] and [`Scaled`] implement for generic code.
//!
//! The traits on one decimal type form two layers. The general layer, [`FixedDecimal`], the sign
//! traits and [`DecimalOne`], requires `Clone` but not `Copy`, and has zero, the limits and one as
//! the functions `zero`, `max_value`, `min_value` and `one`, which return the inherent constants
//! `ZERO`, `MAX`, `MIN` and `ONE`; its other items mirror inherent items of the same names, which
//! concrete types reach without the trait. The const layer, [`ConstFixedDecimal`] and
//! [`ConstDecimalOne`], adds `Copy` and the constants themselves, which are the inherent constants
//! and which a generic `const fn` can read. Each of these traits has one implementation for each
//! storage, which `storage_decimal!` emits next to the inherent items. [`DecimalFloat`], the
//! conversions with the floats, has one implementation for each storage too, and mirrors the
//! inherent functions of the same names. [`ExactMul`], [`WidenTo`], [`ConvertTo`] and [`DivRound`],
//! which relate two or three decimal types, mirror inherent methods of the same names, and each has
//! a single implementation for every combination of them, with the bounds of the inherent method it
//! calls.
//!
//! The operators come with the layers as supertraits. [`FixedDecimal`] requires `+` and `-` on two
//! values and `Sum` over values, and [`DecimalAssign`] the assignments `+=` and `-=`, which need a
//! storage that holds its value in place: it has one implementation for every
//! [`MutableStorage`](crate::MutableStorage), and [`ConstFixedDecimal`] requires it. Each operator
//! has one implementation for every storage, which computes in the inline twin of the decimal,
//! bounded by the [`FixedDecimal`] of the twin, whose checked methods it calls. The operators on
//! two references and `Sum` over references are implemented for every decimal type as well, but
//! are not supertraits: generic code that adds by reference calls
//! [`checked_add`](FixedDecimal::checked_add), or adds the bound
//! `for<'a> &'a T: Add<&'a T, Output = T>`.
//!
//! [`Scaled`] implements each trait once, for every decimal that implements it, with the decimal's
//! result and the scale that the documentation of `Scaled` gives each operation. A `Scaled` relates
//! only to a `Scaled`. Its parsing is defined for each storage, so its implementations of the
//! traits on one type require the `FromStr` that [`FixedDecimal`] needs, and those of the traits
//! between types require every `Scaled` that they relate to be a [`FixedDecimal`]. Code generic
//! over the decimal `D` adds the bound `Scaled<D>: FixedDecimal`.

use core::{
  fmt::{Debug, Display},
  hash::Hash,
  ops::{Add, AddAssign, BitAnd, Neg, Sub, SubAssign},
  str::FromStr,
};

use typenum::{And, Diff, Max, Maximum, Sum};

use sealed::{Sealed, SealedPair, SealedTriple};

use super::{ConvertError, Decimal, DivideError, ParseError, QuantizeError, Scaled};
use crate::{
  FromFloatError,
  layout::{DivIntermediate, Layout, MutableStorage, Pick, Storage, Widen, WidenRepr},
  rounding::RoundingMode,
};

mod sealed {
  /// Seals the traits on one type: only [`Decimal`](crate::Decimal) and
  /// [`Scaled`](crate::Scaled) implement them.
  pub trait Sealed {}

  impl<I, F, R> Sealed for super::Decimal<I, F, R> {}

  // Unconditional: another crate cannot implement a trait of this crate for a `Scaled` over a type
  // of its own anyway, because `Scaled` is not one of its types.
  impl<D> Sealed for super::Scaled<D> {}

  /// Seals the traits that relate a decimal type to another, including the parameter: only pairs
  /// of [`Decimal`](crate::Decimal)s and pairs of [`Scaled`](crate::Scaled)s implement them.
  pub trait SealedPair<T> {}

  impl<I, F, R, I2, F2, R2> SealedPair<super::Decimal<I2, F2, R2>> for super::Decimal<I, F, R> {}

  impl<D, D2> SealedPair<super::Scaled<D2>> for super::Scaled<D> {}

  /// Seals the traits that relate a decimal type to two others, including the parameters: only
  /// triples of [`Decimal`](crate::Decimal)s and triples of [`Scaled`](crate::Scaled)s implement
  /// them.
  pub trait SealedTriple<T, U> {}

  impl<I, F, R, I2, F2, R2, I3, F3, R3>
    SealedTriple<super::Decimal<I2, F2, R2>, super::Decimal<I3, F3, R3>>
    for super::Decimal<I, F, R>
  {
  }

  impl<D, D2, D3> SealedTriple<super::Scaled<D2>, super::Scaled<D3>> for super::Scaled<D> {}
}

/// An exact fixed-point decimal number: [`Decimal`] over any storage, or [`Scaled`] over such a
/// decimal.
///
/// [`zero`](Self::zero), [`max_value`](Self::max_value) and [`min_value`](Self::min_value) return
/// the inherent constants `ZERO`, `MAX` and `MIN`, and have no inherent counterparts; the other
/// constants and methods are the inherent items of the same names, which a concrete type reaches
/// without this trait, also in constant contexts. This trait requires `Clone` rather than `Copy`,
/// so generic code bounded by it alone clones a value that it uses again, or computes with it in
/// [`to_inline`](Self::to_inline), which is `Copy`, and stores the result once with
/// [`from_inline`](Self::from_inline); [`ConstFixedDecimal`] adds `Copy` and the constants `ZERO`,
/// `MAX` and `MIN`. Generic code reaches the items through this trait, which `Decimal<I, F, R>`
/// implements for each storage `R` where `(I, F): Layout<R>`; a method call needs the trait in
/// scope. Because the implementations are per storage, code generic over the storage adds the bound
/// `Decimal<I, F, R>: FixedDecimal`, which `R: Storage` and `(I, F): Layout<R>` do not imply.
/// [`Scaled<D>`](Scaled) implements this trait for every `D` that does, keeping the scale of its
/// text, and code generic over `D` that computes with `Scaled<D>` adds the bound
/// `Scaled<D>: FixedDecimal`. The supertraits give generic code cloning, comparison, hashing,
/// formatting, parsing through `FromStr`, `Default`, which is zero, and the operators below. The
/// functions `zero`, `max_value` and `min_value`, and [`one`](DecimalOne::one), have the names of
/// those of `Zero`, `Bounded` and `One` in the `num-traits` crate, so code bounded by traits of
/// both crates names the trait in the path, as in `FixedDecimal::zero()`.
///
/// The supertraits `Add`, `Sub` and `Sum` give generic code the operators `+` and `-` on two
/// values and the sum of an iterator of values. They take their operands by value, so code bounded
/// by this trait alone clones an operand that it uses again, and they panic where
/// [`checked_add`](Self::checked_add) and [`checked_sub`](Self::checked_sub) return `None`. The
/// compound assignments `+=` and `-=` come with [`DecimalAssign`], which [`ConstFixedDecimal`]
/// requires. The operators on two references and the sum of an iterator of references are not
/// supertraits: generic code calls `checked_add`, or adds the bound
/// `for<'a> &'a T: Add<&'a T, Output = T>`.
///
/// ```
/// use numo::{
///   Decimal, FixedDecimal,
///   typenum::{U2, U8, U10},
/// };
///
/// // The sum of the values, or `None` if it is outside the range of the type.
/// fn total<T: FixedDecimal>(values: &[T]) -> Option<T> {
///   values.iter().try_fold(T::zero(), |sum, value| sum.checked_add(value))
/// }
///
/// // Whether a value is one of the limits of its type.
/// fn is_limit<T: FixedDecimal>(value: &T) -> bool {
///   *value == T::min_value() || *value == T::max_value()
/// }
///
/// // The text of a value with every fraction digit of its type.
/// fn padded<T: FixedDecimal>(value: &T) -> String {
///   let digits = T::FRACTION as usize;
///   format!("{value:.digits$}")
/// }
///
/// // The value doubled, computed in its inline twin, which is `Copy`, and stored once.
/// fn doubled<T: FixedDecimal>(value: &T) -> Option<T> {
///   let inline = value.to_inline();
///   inline.checked_add(&inline).map(T::from_inline)
/// }
///
/// type Price = Decimal<U10, U8>;
/// type Fee = Decimal<U10, U2, u64>;
///
/// let prices: [Price; 2] = ["64250.5".parse()?, "0.25".parse()?];
/// assert_eq!(total(&prices), Some("64250.75".parse()?));
/// assert_eq!(total(&[Price::MAX, Price::MAX]), None);
/// assert_eq!(doubled(&prices[1]), Some("0.5".parse()?));
/// assert_eq!(doubled(&Price::MAX), None);
/// assert!(is_limit(&Price::MIN) && !is_limit(&prices[0]));
/// assert!(is_limit(&Fee::ZERO));
/// assert_eq!(padded(&prices[1]), "0.25000000");
/// assert_eq!(padded(&"1.5".parse::<Fee>()?), "1.50");
/// assert!(!<Fee as FixedDecimal>::SIGNED);
/// # Ok::<(), numo::ParseError>(())
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait FixedDecimal:
  Sealed
  + Clone
  + Eq
  + Ord
  + Hash
  + Debug
  + Display
  + Default
  + FromStr<Err = ParseError>
  + Add<Output = Self>
  + Sub<Output = Self>
  + core::iter::Sum
{
  /// The number of integral digits, `Integral`.
  const INTEGRAL: u32;

  /// The number of fraction digits, `Fraction`.
  const FRACTION: u32;

  /// The total number of digits, `Integral + Fraction`.
  const DIGITS: u32;

  /// `true` for a signed storage, whose values can be negative, and `false` for an unsigned one.
  const SIGNED: bool;

  /// The storage of the coefficient, `Repr`.
  type Repr: Storage;

  /// The decimal with the same digit counts over the inline storage of the same width and sign, or
  /// for a [`Scaled`] the `Scaled` over that decimal: `Self` for an inline storage.
  type Inline: ConstFixedDecimal;

  /// Returns the value in its inline storage. This is a copy and allocates nothing.
  #[must_use]
  fn to_inline(&self) -> Self::Inline;

  /// Returns `value` in the storage of `Self`: `value` itself for an inline storage.
  #[must_use]
  fn from_inline(value: Self::Inline) -> Self;

  /// Returns the coefficient: the stored integer `c` such that the value is `c / 10^Fraction`.
  #[must_use]
  fn repr(self) -> Self::Repr;

  /// Creates a value from its coefficient `repr`, meaning `repr / 10^Fraction`, or returns `None`
  /// if `repr` is outside the range of the type.
  #[must_use]
  fn from_repr(repr: Self::Repr) -> Option<Self>;

  /// Returns zero.
  #[must_use]
  fn zero() -> Self;

  /// Returns the largest value, `10^Integral - 10^-Fraction`: every digit is `9`.
  #[must_use]
  fn max_value() -> Self;

  /// Returns the smallest value: the largest value negated for a signed storage, and zero for an
  /// unsigned one.
  #[must_use]
  fn min_value() -> Self;

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

  /// Returns the remainder of `self` divided by `rhs`, which has the sign of `self`, or `None` if
  /// `rhs` is zero.
  #[must_use]
  fn checked_rem(&self, rhs: &Self) -> Option<Self>;

  /// Adds two values, returning the limit that the sum passes if it is outside the range of the
  /// type: the largest value above the range and the smallest below it.
  #[must_use]
  fn saturating_add(&self, rhs: &Self) -> Self;

  /// Subtracts `rhs` from `self`, returning the limit that the difference passes if it is outside
  /// the range of the type: the smallest value below the range, which is zero for an unsigned
  /// storage, and the largest above it.
  #[must_use]
  fn saturating_sub(&self, rhs: &Self) -> Self;

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

  /// Returns the integral part, rounded toward zero, as [`Decimal::trunc`] describes.
  #[must_use]
  fn trunc(&self) -> Self;

  /// Returns the fraction, with the sign of `self`, as [`Decimal::fract`] describes.
  #[must_use]
  fn fract(&self) -> Self;

  /// Returns the largest integer at most `self`, as [`Decimal::floor`] describes.
  ///
  /// # Errors
  ///
  /// Returns [`ConvertError::OutOfRange`] if that integer is outside the range of the type.
  #[must_use = "this returns the rounded value, without modifying the original"]
  fn floor(&self) -> Result<Self, ConvertError>;

  /// Returns the smallest integer at least `self`, as [`Decimal::ceil`] describes.
  ///
  /// # Errors
  ///
  /// Returns [`ConvertError::OutOfRange`] if that integer is outside the range of the type.
  #[must_use = "this returns the rounded value, without modifying the original"]
  fn ceil(&self) -> Result<Self, ConvertError>;

  /// Returns `true` if every fraction digit is zero.
  #[must_use]
  fn is_integer(&self) -> bool;

  /// Returns the exponent of the first significant digit, 0 for zero, as
  /// [`Decimal::order_of_magnitude`] describes.
  #[must_use]
  fn order_of_magnitude(&self) -> i32;

  /// Returns the number of significant digits, 1 for zero, as [`Decimal::significant_digits`]
  /// describes.
  #[doc(alias = "digits")]
  #[must_use]
  fn significant_digits(&self) -> u32;

  /// Returns the number of fraction digits that `Display` writes for the value, as
  /// [`Decimal::fraction_digits`] describes: at most [`FRACTION`](Self::FRACTION), the fraction
  /// digits of the type.
  #[doc(alias = "fractional_digit_count")]
  #[must_use]
  fn fraction_digits(&self) -> u32;

  /// Rounds to `digits` significant digits with `mode`, as [`Decimal::round_significant`]
  /// describes.
  ///
  /// # Errors
  ///
  /// Returns [`ConvertError::OutOfRange`] if the rounded value is outside the range of the type.
  #[doc(alias("round_sf", "with_precision_round"))]
  #[must_use = "this returns the rounded value, without modifying the original"]
  fn round_significant(
    &self,
    digits: core::num::NonZero<u32>,
    mode: RoundingMode,
  ) -> Result<Self, ConvertError>;
}

/// A decimal number whose storage holds the coefficient in place: [`Decimal`] over a
/// [`MutableStorage`](crate::MutableStorage), as every storage of this crate is, or [`Scaled`] over
/// such a decimal.
///
/// The supertraits `AddAssign` and `SubAssign` give generic code the compound assignments `+=` and
/// `-=`, each with a value on the right, as the operators of [`FixedDecimal`] take one. They panic
/// where the operators `+` and `-` do, and then leave their target unchanged. [`ConstFixedDecimal`]
/// requires this trait.
///
/// ```
/// use numo::{
///   Decimal, DecimalAssign,
///   typenum::{U8, U10},
/// };
///
/// // The total of the values, each added in place.
/// fn total<T: DecimalAssign>(values: &[T]) -> T {
///   let mut total = T::zero();
///   for value in values {
///     total += value.clone();
///   }
///   total
/// }
///
/// type Price = Decimal<U10, U8>;
///
/// let prices: [Price; 2] = ["64250.5".parse()?, "0.25".parse()?];
/// assert_eq!(total(&prices), "64250.75".parse()?);
/// # Ok::<(), numo::ParseError>(())
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait DecimalAssign: FixedDecimal + AddAssign + SubAssign {}

// Bounded by `MutableStorage`, so that a decimal has the assignments only where its storage holds
// the coefficient in place, and by the twin, in which the assignments compute.
impl<I, F, R> DecimalAssign for Decimal<I, F, R>
where
  R: MutableStorage,
  Self: FixedDecimal,
  Decimal<I, F, R::Inline>: FixedDecimal,
{
}

/// An exact fixed-point decimal number whose zero and limits are constants: [`Decimal`] over any
/// storage, or [`Scaled`] over such a decimal.
///
/// `ZERO`, `MAX` and `MIN` are the inherent constants of the same names, which a concrete type
/// reaches without this trait, also in constant contexts. Generic code reads them through this
/// trait as `T::ZERO`, `T::MAX` and `T::MIN`, also in a generic `const fn`, which cannot call
/// [`zero`](FixedDecimal::zero), [`max_value`](FixedDecimal::max_value) or
/// [`min_value`](FixedDecimal::min_value) because trait methods are not `const`. The supertrait
/// `Copy` lets generic code use a value again without cloning it, and [`DecimalAssign`] gives it
/// the compound assignments `+=` and `-=`, which panic where the operators `+` and `-` do and then
/// leave their target unchanged. A type of this layer is its own [`Inline`](FixedDecimal::Inline)
/// type, so that [`to_inline`](FixedDecimal::to_inline) returns the value itself.
/// `Decimal<I, F, R>` implements this trait for each storage `R` where `(I, F): Layout<R>`, as it
/// implements [`FixedDecimal`].
///
/// ```
/// use numo::{
///   ConstFixedDecimal, Decimal,
///   typenum::{U2, U8, U10},
/// };
///
/// // The smallest value, zero and the largest value, in a constant context.
/// const fn limits<T: ConstFixedDecimal>() -> [T; 3] {
///   [T::MIN, T::ZERO, T::MAX]
/// }
///
/// // The coefficient and the value, which `Copy` keeps after `repr` takes it.
/// fn with_repr<T: ConstFixedDecimal>(value: T) -> (T::Repr, T) {
///   (value.repr(), value)
/// }
///
/// type Price = Decimal<U10, U8>;
/// type Fee = Decimal<U10, U2, u64>;
///
/// const PRICES: [Price; 3] = limits();
/// const FEES: [Fee; 3] = limits();
/// assert_eq!(PRICES, [Price::MIN, Price::ZERO, Price::MAX]);
/// assert_eq!(FEES, [Fee::ZERO, Fee::ZERO, Fee::MAX]);
/// let price: Price = "1.5".parse()?;
/// assert_eq!(with_repr(price), (150_000_000, price));
/// # Ok::<(), numo::ParseError>(())
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait ConstFixedDecimal: DecimalAssign + FixedDecimal<Inline = Self> + Copy {
  /// Zero.
  const ZERO: Self;

  /// The largest value, `10^Integral - 10^-Fraction`: every digit is `9`.
  const MAX: Self;

  /// The smallest value: `-MAX` for a signed storage, and zero for an unsigned one.
  const MIN: Self;
}

/// A decimal number with a signed storage: [`Decimal`] over `i8`, `i16`, `i32`, `i64`, `i128`,
/// [`i192`](crate::i192), [`i256`](crate::i256), [`i320`](crate::i320), [`i384`](crate::i384),
/// [`i448`](crate::i448) or [`i512`](crate::i512), or [`Scaled`] over such a decimal.
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
/// [`u320`](crate::u320), [`u384`](crate::u384), [`u448`](crate::u448) or [`u512`](crate::u512),
/// or [`Scaled`] over such a decimal.
///
/// This trait has no items of its own. As a bound, it states that values are never negative, so
/// that [`min_value`](FixedDecimal::min_value) returns zero and
/// [`checked_sub`](FixedDecimal::checked_sub) returns `None` below zero.
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
/// storage, or [`Scaled`] over such a decimal.
///
/// [`one`](Self::one) returns the inherent constant `ONE` and has no inherent counterpart; a
/// concrete type reaches `ONE` without this trait, also in constant contexts, and
/// [`ConstDecimalOne`] gives it to generic code as a constant. One needs an integral digit, so a
/// type whose `Integral` is zero has no `ONE`, rather than one that is out of range.
/// `Decimal<I, F, R>` implements this trait for each storage `R` where `I: NonZero` and
/// `(I, F): Layout<R>`. It is separate from the integer trait [`IntegerOne`](crate::IntegerOne),
/// which also requires `Product`.
///
/// ```
/// use numo::{
///   Decimal, DecimalOne,
///   typenum::{U2, U8, U10},
/// };
///
/// // The value plus one, or `None` if the sum is outside the range of the type.
/// fn successor<T: DecimalOne>(value: &T) -> Option<T> {
///   value.checked_add(&T::one())
/// }
///
/// type Price = Decimal<U10, U8>;
///
/// assert_eq!(successor(&"1.5".parse::<Price>()?), Some("2.5".parse()?));
/// assert_eq!(successor(&Price::MAX), None);
/// assert_eq!(<Decimal<U2, U2, u16> as DecimalOne>::one().to_string(), "1");
/// # Ok::<(), numo::ParseError>(())
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait DecimalOne: FixedDecimal {
  /// Returns one.
  #[must_use]
  fn one() -> Self;
}

/// A decimal number that holds one as a constant: [`Decimal`] with at least one integral digit,
/// over any storage, or [`Scaled`] over such a decimal.
///
/// `ONE` is the inherent constant of the same name, which a concrete type reaches without this
/// trait, also in constant contexts. Generic code reads it through this trait as `T::ONE`, also in
/// a generic `const fn`, which cannot call [`one`](DecimalOne::one). `Decimal<I, F, R>` implements
/// this trait for each storage `R` where `I: NonZero` and `(I, F): Layout<R>`, as it implements
/// [`DecimalOne`].
///
/// ```
/// use numo::{
///   ConstDecimalOne, Decimal,
///   typenum::{U8, U10},
/// };
///
/// // Zero and one, in a constant context.
/// const fn units<T: ConstDecimalOne>() -> [T; 2] {
///   [T::ZERO, T::ONE]
/// }
///
/// type Price = Decimal<U10, U8>;
///
/// const UNITS: [Price; 2] = units();
/// assert_eq!(UNITS, [Price::ZERO, Price::ONE]);
/// assert_eq!(UNITS[1].to_string(), "1");
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait ConstDecimalOne: DecimalOne + ConstFixedDecimal {
  /// One.
  const ONE: Self;
}

/// The conversions of a decimal number with the floats `f64` and `f32`: [`Decimal`] over any
/// storage, or [`Scaled`] over such a decimal.
///
/// The methods are the inherent functions of the same names, which a concrete type reaches
/// without this trait, also in constant contexts but for the two that read the shortest text:
/// [`to_f64`](Self::to_f64) and [`to_f32`](Self::to_f32), which round to the nearest float, ties
/// to even; [`round_from_f64`](Self::round_from_f64), which reads the exact binary value of a
/// float, and [`from_f64_shortest`](Self::from_f64_shortest), which reads its shortest text, each
/// rounded once with a [`RoundingMode`] to the fraction digits of the type; and the `f32` form of
/// each reading. They are for interchange and display, not for arithmetic that must be exact. The
/// errors are those of [`FromFloatError`], which reports NaN, the infinities, a value above the
/// range and one below it apart. `Decimal<I, F, R>` implements this trait for each storage `R`
/// where `(I, F): Layout<R>`, and [`Scaled<D>`](Scaled) for every `D` that does: its conversions to
/// a float are those of the value, and a value read from a float takes the fraction digits of the
/// type as its scale, or by the shortest text the fraction digits of the text, at most those of
/// the type.
///
/// ```
/// use numo::{
///   Decimal, DecimalFloat, FromFloatError, RoundingMode, Scaled,
///   typenum::{U2, U8, U10},
/// };
///
/// // The mean of the values, as an `f64`.
/// fn mean<T: DecimalFloat>(values: &[T]) -> f64 {
///   values.iter().map(T::to_f64).sum::<f64>() / values.len() as f64
/// }
///
/// // A measurement read by its shortest text into any decimal type.
/// fn reading<T: DecimalFloat>(value: f64) -> Result<T, FromFloatError> {
///   T::from_f64_shortest(value, RoundingMode::HalfEven)
/// }
///
/// type Price = Decimal<U10, U8>;
///
/// let prices: [Price; 2] = ["1.5".parse()?, "2.5".parse()?];
/// assert_eq!(mean(&prices), 2.0);
/// let cents: Decimal<U10, U2> = reading(2.675)?;
/// assert_eq!(cents.to_string(), "2.68");
/// let scaled: Scaled<Price> = reading(0.5)?;
/// assert_eq!((scaled.to_string(), scaled.scale()), ("0.5".to_string(), 1));
/// assert_eq!(reading::<Price>(f64::NAN), Err(FromFloatError::Nan));
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait DecimalFloat: FixedDecimal {
  /// Converts the value to the nearest `f64`, ties to even, as [`Decimal::to_f64`] describes.
  #[must_use]
  fn to_f64(&self) -> f64;

  /// Converts the value to the nearest `f32`, ties to even, as [`Decimal::to_f32`] describes.
  #[must_use]
  fn to_f32(&self) -> f32;

  /// Converts the exact binary value of `value`, rounded once with `mode` to the fraction digits
  /// of the type, as [`Decimal::round_from_f64`] describes.
  ///
  /// # Errors
  ///
  /// Returns [`FromFloatError::Nan`] for NaN, [`Infinite`](FromFloatError::Infinite) for an
  /// infinity, and for a rounded value outside the range of the type,
  /// [`PosOverflow`](FromFloatError::PosOverflow) above it and
  /// [`NegOverflow`](FromFloatError::NegOverflow) below it.
  fn round_from_f64(value: f64, mode: RoundingMode) -> Result<Self, FromFloatError>;

  /// Converts the exact binary value of `value`, rounded once with `mode` to the fraction digits
  /// of the type, as [`Decimal::round_from_f32`] describes.
  ///
  /// # Errors
  ///
  /// Returns [`FromFloatError::Nan`] for NaN, [`Infinite`](FromFloatError::Infinite) for an
  /// infinity, and for a rounded value outside the range of the type,
  /// [`PosOverflow`](FromFloatError::PosOverflow) above it and
  /// [`NegOverflow`](FromFloatError::NegOverflow) below it.
  fn round_from_f32(value: f32, mode: RoundingMode) -> Result<Self, FromFloatError>;

  /// Converts the decimal that the shortest text of `value` names, rounded once with `mode` to the
  /// fraction digits of the type, as [`Decimal::from_f64_shortest`] describes.
  ///
  /// # Errors
  ///
  /// Returns [`FromFloatError::Nan`] for NaN, [`Infinite`](FromFloatError::Infinite) for an
  /// infinity, and for a rounded value outside the range of the type,
  /// [`PosOverflow`](FromFloatError::PosOverflow) above it and
  /// [`NegOverflow`](FromFloatError::NegOverflow) below it.
  #[doc(alias = "from_f64")]
  fn from_f64_shortest(value: f64, mode: RoundingMode) -> Result<Self, FromFloatError>;

  /// Converts the decimal that the shortest text of `value` names, rounded once with `mode` to the
  /// fraction digits of the type, as [`Decimal::from_f32_shortest`] describes.
  ///
  /// # Errors
  ///
  /// Returns [`FromFloatError::Nan`] for NaN, [`Infinite`](FromFloatError::Infinite) for an
  /// infinity, and for a rounded value outside the range of the type,
  /// [`PosOverflow`](FromFloatError::PosOverflow) above it and
  /// [`NegOverflow`](FromFloatError::NegOverflow) below it.
  #[doc(alias = "from_f32")]
  fn from_f32_shortest(value: f32, mode: RoundingMode) -> Result<Self, FromFloatError>;
}

/// The exact product of two decimal numbers: [`Decimal::exact_mul`].
///
/// `exact_mul` is the inherent method of the same name, which a concrete type reaches without this
/// trait. [`Decimal`] implements this trait for every pair of decimal types whose product has at
/// most 153 digits, with the bounds of the inherent method, which generic code then does not
/// repeat. `Output` has the integral and fraction digits of both operands together, in the storage
/// that [`Pick`](crate::Pick) chooses for them, which is unsigned only if both operand storages
/// are. It implements [`FixedDecimal`], so generic code with that trait in scope computes with the
/// product without a bound of its own. As for [`Mul`](core::ops::Mul), `Rhs` defaults to `Self`.
/// Two [`Scaled`] values multiply into a `Scaled` with the product of their decimals and the sum
/// of their scales, saturating at 65535.
///
/// ```
/// use numo::{
///   Decimal, ExactMul, FixedDecimal,
///   typenum::{U8, U10, U12, U16, U22},
/// };
///
/// // The total of the products of the pairs, or `None` if it is outside the range of the product.
/// fn notional<P: ExactMul<Q>, Q>(fills: &[(P, Q)]) -> Option<P::Output> {
///   fills.iter().try_fold(P::Output::zero(), |sum, (price, quantity)| {
///     sum.checked_add(&price.exact_mul(quantity))
///   })
/// }
///
/// type Price = Decimal<U10, U8>;
/// type Quantity = Decimal<U12, U8>;
///
/// let fills: [(Price, Quantity); 2] = [
///   ("64250.5".parse()?, "0.0015".parse()?),
///   ("64251".parse()?, "0.001".parse()?),
/// ];
/// let total: Decimal<U22, U16> = notional(&fills).unwrap();
/// assert_eq!(total.to_string(), "160.62675");
/// # Ok::<(), numo::ParseError>(())
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait ExactMul<Rhs = Self>: SealedPair<Rhs> {
  /// The type of the product.
  type Output: FixedDecimal;

  /// Multiplies two decimal numbers exactly. The product always fits `Output` and is never
  /// rounded.
  #[must_use]
  fn exact_mul(&self, rhs: &Rhs) -> Self::Output;
}

/// A conversion that never fails, to a decimal type with at least as many integral and fraction
/// digits: [`Decimal::widen`].
///
/// `widen` is the inherent method of the same name, which a concrete type reaches without this
/// trait. [`Decimal`] implements this trait for every target that `widen` accepts, with the bounds
/// of the inherent method: one whose storage holds its digits and is signed if the source's
/// storage is. A [`Scaled`] widens to a `Scaled` with its scale.
///
/// ```
/// use numo::{
///   Decimal, WidenTo,
///   typenum::{U2, U4, U8, U10, U18, U20},
/// };
///
/// type Total = Decimal<U20, U18, i128>;
///
/// // The sum of two values of different types, in a type that holds both.
/// fn sum<A: WidenTo<Total>, B: WidenTo<Total>>(a: A, b: B) -> Option<Total> {
///   a.widen().checked_add(&b.widen())
/// }
///
/// let fee: Decimal<U2, U4, u32> = "1.25".parse()?;
/// let change: Decimal<U10, U8> = "-64250.5".parse()?;
/// assert_eq!(sum(fee, change), Some("-64249.25".parse()?));
/// # Ok::<(), numo::ParseError>(())
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait WidenTo<T>: SealedPair<T> {
  /// Converts to `T`, which has at least as many integral and fraction digits. Never fails.
  #[must_use]
  fn widen(self) -> T;
}

/// A conversion to any decimal type that fails instead of losing digits, or rounds them with a
/// mode: [`Decimal::try_convert`] and [`Decimal::round_to`].
///
/// The methods are the inherent methods of the same names, which a concrete type reaches without
/// this trait. [`Decimal`] implements this trait for every pair of decimal types, with the bounds
/// of the inherent methods. A [`Scaled`] converts to a `Scaled`: `try_convert` keeps its scale, and
/// `round_to` gives the fraction digits of `T` as the scale.
///
/// ```
/// use numo::{
///   ConvertError, ConvertTo, Decimal, RoundingMode,
///   typenum::{U2, U8, U10},
/// };
///
/// type Cents = Decimal<U10, U2>;
///
/// // An amount in cents, or an error if a dropped digit is not zero.
/// fn exact_cents<T: ConvertTo<Cents>>(amount: T) -> Result<Cents, ConvertError> {
///   amount.try_convert()
/// }
///
/// // An amount in cents, rounded half to even.
/// fn cents<T: ConvertTo<Cents>>(amount: T) -> Result<Cents, ConvertError> {
///   amount.round_to(RoundingMode::HalfEven)
/// }
///
/// let price: Decimal<U10, U8> = "2.345".parse()?;
/// assert_eq!(exact_cents(price), Err(ConvertError::Inexact));
/// assert_eq!(cents(price), Ok("2.34".parse()?));
/// let fee: Decimal<U2, U2, u16> = "0.5".parse()?;
/// assert_eq!(exact_cents(fee), Ok("0.5".parse()?));
/// # Ok::<(), numo::ParseError>(())
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait ConvertTo<T>: SealedPair<T> {
  /// Converts to `T`, failing instead of losing digits, as [`Decimal::try_convert`] describes.
  ///
  /// # Errors
  ///
  /// Returns [`ConvertError::OutOfRange`] if the value is outside the range of `T`, and otherwise
  /// [`ConvertError::Inexact`] if `T` has fewer fraction digits and a dropped digit is not zero.
  fn try_convert(self) -> Result<T, ConvertError>;

  /// Converts to `T`, rounding with `mode` where `T` has fewer fraction digits, as
  /// [`Decimal::round_to`] describes.
  ///
  /// # Errors
  ///
  /// Returns [`ConvertError::OutOfRange`] if the rounded value is outside the range of `T`. Never
  /// returns [`ConvertError::Inexact`].
  #[must_use = "this returns the rounded value, without modifying the original"]
  fn round_to(self, mode: RoundingMode) -> Result<T, ConvertError>;
}

/// Division into any decimal type, rounding the quotient once with a mode:
/// [`Decimal::div_round`].
///
/// `div_round` is the inherent method of the same name, which a concrete type reaches without this
/// trait. [`Decimal`] implements this trait for every dividend, divisor and quotient type whose
/// intermediate has at most 153 digits, or 154 when both operand storages are unsigned, with the
/// bounds of the inherent method, which generic code then does not repeat. A [`Scaled`] divides by
/// a `Scaled` into a `Scaled` whose scale is the fraction digits of `Out`.
///
/// ```
/// use numo::{
///   Decimal, DivRound, DivideError, RoundingMode,
///   typenum::{U2, U8, U10, U12},
/// };
///
/// // The quantity that a budget buys at a price, rounded down.
/// fn quantity<B, P, Q>(budget: &B, price: &P) -> Result<Q, DivideError>
/// where
///   B: DivRound<P, Q>,
/// {
///   budget.div_round(price, RoundingMode::Down)
/// }
///
/// let budget: Decimal<U12, U2> = "1000".parse()?;
/// let price: Decimal<U10, U8> = "64250.5".parse()?;
/// let bought: Decimal<U12, U8> = quantity(&budget, &price)?;
/// assert_eq!(bought.to_string(), "0.01556408");
/// assert_eq!(
///   quantity::<_, _, Decimal<U12, U8>>(&budget, &Decimal::<U10, U8>::ZERO),
///   Err(DivideError::DivisionByZero)
/// );
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait DivRound<Rhs, Out>: SealedTriple<Rhs, Out> {
  /// Divides by `rhs`, rounding the quotient once with `mode` to the fraction digits of `Out`, as
  /// [`Decimal::div_round`] describes.
  ///
  /// # Errors
  ///
  /// Returns [`DivideError::DivisionByZero`] if `rhs` is zero, and otherwise
  /// [`DivideError::OutOfRange`] if the rounded quotient is outside the range of `Out`.
  #[must_use = "this returns the rounded quotient, without modifying the original"]
  fn div_round(&self, rhs: &Rhs, mode: RoundingMode) -> Result<Out, DivideError>;
}

// Each implementation has the bounds of the inherent method that it calls, those of its impl block
// first, and calls it through `Self::`, which names the inherent method before the trait's.
impl<I, F, R, I2, F2, R2> ExactMul<Decimal<I2, F2, R2>> for Decimal<I, F, R>
where
  R: Storage,
  R2: Storage,
  I: Add<I2>,
  F: Add<F2>,
  Sum<I, I2>: Add<Sum<F, F2>>,
  R::IsUnsigned: BitAnd<R2::IsUnsigned>,
  And<R::IsUnsigned, R2::IsUnsigned>: Pick<Sum<Sum<I, I2>, Sum<F, F2>>>,
  Decimal<
    Sum<I, I2>,
    Sum<F, F2>,
    <And<R::IsUnsigned, R2::IsUnsigned> as Pick<Sum<Sum<I, I2>, Sum<F, F2>>>>::Storage,
  >: FixedDecimal,
{
  type Output = Decimal<
    Sum<I, I2>,
    Sum<F, F2>,
    <And<R::IsUnsigned, R2::IsUnsigned> as Pick<Sum<Sum<I, I2>, Sum<F, F2>>>>::Storage,
  >;

  #[inline]
  fn exact_mul(&self, rhs: &Decimal<I2, F2, R2>) -> Self::Output {
    Self::exact_mul(self, rhs)
  }
}

impl<I, F, R, I2, F2, R2> WidenTo<Decimal<I2, F2, R2>> for Decimal<I, F, R>
where
  R: Storage,
  (I, F): Layout<R>,
  R: WidenRepr<R2>,
  R2: Storage,
  (I2, F2): Layout<R2>,
  (I, F): Widen<(I2, F2)>,
{
  #[inline]
  fn widen(self) -> Decimal<I2, F2, R2> {
    Self::widen(self)
  }
}

impl<I, F, R, I2, F2, R2> ConvertTo<Decimal<I2, F2, R2>> for Decimal<I, F, R>
where
  R: Storage,
  (I, F): Layout<R>,
  R2: Storage,
  (I2, F2): Layout<R2>,
{
  #[inline]
  fn try_convert(self) -> Result<Decimal<I2, F2, R2>, ConvertError> {
    Self::try_convert(self)
  }

  #[inline]
  fn round_to(self, mode: RoundingMode) -> Result<Decimal<I2, F2, R2>, ConvertError> {
    Self::round_to(self, mode)
  }
}

impl<I, F, R, I2, F2, R2, I3, F3, R3> DivRound<Decimal<I2, F2, R2>, Decimal<I3, F3, R3>>
  for Decimal<I, F, R>
where
  R: Storage,
  (I, F): Layout<R>,
  R2: Storage,
  (I2, F2): Layout<R2>,
  R3: Storage,
  (I3, F3): Layout<R3>,
  F2: Add<F3>,
  Sum<F2, F3>: Max<F>,
  I: Add<Maximum<Sum<F2, F3>, F>>,
  Maximum<Sum<F2, F3>, F>: Sub<F3>,
  I2: Add<Diff<Maximum<Sum<F2, F3>, F>, F3>>,
  Sum<I, Maximum<Sum<F2, F3>, F>>: Max<Sum<I2, Diff<Maximum<Sum<F2, F3>, F>, F3>>>,
  R::IsUnsigned: BitAnd<R2::IsUnsigned>,
  Maximum<Sum<I, Maximum<Sum<F2, F3>, F>>, Sum<I2, Diff<Maximum<Sum<F2, F3>, F>, F3>>>:
    DivIntermediate<And<R::IsUnsigned, R2::IsUnsigned>>,
{
  #[inline]
  fn div_round(
    &self,
    rhs: &Decimal<I2, F2, R2>,
    mode: RoundingMode,
  ) -> Result<Decimal<I3, F3, R3>, DivideError> {
    Self::div_round(self, rhs, mode)
  }
}

#[cfg(test)]
mod tests;
