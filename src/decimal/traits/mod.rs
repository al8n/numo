//! The traits that [`Decimal`] implements for generic code.
//!
//! Each trait mirrors inherent items of the same names, which concrete types reach without the
//! trait. [`FixedDecimal`], the sign traits and [`HasDecimalOne`] have one implementation for each
//! storage, which `storage_decimal!` emits next to the inherent items. [`ExactMul`], [`WidenTo`],
//! [`ConvertTo`] and [`DivRound`], which relate two or three decimal types, each have a single
//! implementation for every combination of them, with the bounds of the inherent method it calls.

use core::{
  fmt::{Debug, Display},
  hash::Hash,
  ops::{Add, BitAnd, Neg, Sub},
  str::FromStr,
};

use typenum::{And, Diff, Max, Maximum, Sum};

use sealed::Sealed;

use super::{ConvertError, Decimal, DivideError, ParseError, QuantizeError};
use crate::{
  layout::{DivIntermediate, Layout, Pick, Storage, Widen, WidenRepr},
  rounding::RoundingMode,
};

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

/// The exact product of two decimal numbers: [`Decimal::exact_mul`].
///
/// `exact_mul` is the inherent method of the same name, which a concrete type reaches without this
/// trait. [`Decimal`] implements this trait for every pair of decimal types whose product has at
/// most 153 digits, with the bounds of the inherent method, which generic code then does not
/// repeat. `Output` has the integral and fraction digits of both operands together, in the storage
/// that [`Pick`](crate::Pick) chooses for them, which is unsigned only if both operand storages
/// are. It implements [`FixedDecimal`], so generic code with that trait in scope computes with the
/// product without a bound of its own. As for [`Mul`](core::ops::Mul), `Rhs` defaults to `Self`.
///
/// ```
/// use numo::{
///   Decimal, ExactMul, FixedDecimal,
///   typenum::{U8, U10, U12, U16, U22},
/// };
///
/// // The total of the products of the pairs, or `None` if it is outside the range of the product.
/// fn notional<P: ExactMul<Q>, Q>(fills: &[(P, Q)]) -> Option<P::Output> {
///   fills.iter().try_fold(P::Output::ZERO, |sum, (price, quantity)| {
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
pub trait ExactMul<Rhs = Self>: Sealed {
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
/// storage is.
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
pub trait WidenTo<T>: Sealed {
  /// Converts to `T`, which has at least as many integral and fraction digits. Never fails.
  #[must_use]
  fn widen(self) -> T;
}

/// A conversion to any decimal type that fails instead of losing digits, or rounds them with a
/// mode: [`Decimal::try_convert`] and [`Decimal::round_to`].
///
/// The methods are the inherent methods of the same names, which a concrete type reaches without
/// this trait. [`Decimal`] implements this trait for every pair of decimal types, with the bounds
/// of the inherent methods.
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
pub trait ConvertTo<T>: Sealed {
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
/// bounds of the inherent method, which generic code then does not repeat.
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
pub trait DivRound<Rhs, Out>: Sealed {
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
