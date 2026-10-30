//! A decimal that keeps the scale of its text.

use core::{
  cmp::Ordering,
  fmt,
  hash::{Hash, Hasher},
  ops::Neg,
  str::FromStr,
};

use super::{
  ConstDecimalOne, ConstFixedDecimal, ConvertError, ConvertTo, DecimalOne, DivRound, DivideError,
  ExactMul, FixedDecimal, ParseError, QuantizeError, SignedFixedDecimal, UnsignedFixedDecimal,
  WidenTo, format,
};
use crate::{
  int,
  layout::{StorageCore, Token},
  rounding::RoundingMode,
};

#[cfg(test)]
mod tests;

/// A decimal and the scale of the text it came from: the number of fraction digits that the text
/// showed.
///
/// `D` is a [`Decimal`](crate::Decimal), whose text depends only on its value: `0.00100000` parses
/// into a decimal that prints as `0.001`. A `Scaled` keeps the scale beside the value, here 8, and
/// [`Display`](fmt::Display) writes at least that many fraction digits, so the text comes back as
/// it came. The scale is presentation only. Equality, ordering and hashing compare the value, so
/// `0.001` and `0.00100000` are equal, hash alike and are one element of a set; code that must
/// reproduce a text compares [`scale`](Self::scale) as well. [`Debug`](fmt::Debug) shows both, as
/// in `Scaled { value: 0.001, scale: 8 }`.
///
/// ```
/// use numo::{
///   Decimal, FixedDecimal, Scaled,
///   typenum::{U8, U11},
/// };
///
/// type Amount = Scaled<Decimal<U11, U8, u64>>;
///
/// let padded: Amount = "0.00100000".parse()?;
/// let short: Amount = "0.001".parse()?;
/// assert_eq!(padded.to_string(), "0.00100000");
/// assert_eq!((padded.scale(), short.scale()), (8, 3));
///
/// // The scale is presentation only.
/// assert_eq!(padded, short);
/// assert_eq!(padded.value(), short.value());
///
/// // A precision raises the number of fraction digits and never lowers it.
/// assert_eq!(format!("{short:.5}"), "0.00100");
/// assert_eq!(format!("{padded:.2}"), "0.00100000");
///
/// // A sum takes the larger scale.
/// let sum = padded.checked_add(&short).unwrap();
/// assert_eq!(sum.to_string(), "0.00200000");
/// # Ok::<(), numo::ParseError>(())
/// ```
///
/// # Text
///
/// `parse`, a `const fn` of each storage, and [`FromStr`](core::str::FromStr) read the text as
/// [`Decimal::parse`](crate::Decimal::parse) does, with the same grammar, value and errors, and
/// take its scale: the number of fraction digits written, not counting `_`, minus the exponent,
/// and at least 0. So `2.50` and `250e-2` have scale 2, and `1E+2` has scale 0. The scale may
/// exceed the fraction digits of `D`: a type with 8 fraction digits keeps
/// `9999999999.0000000000000000` with scale 16 and writes it back. A scale above 255 is
/// [`ScaleTooLarge`](crate::ParseErrorKind::ScaleTooLarge), which only a text that
/// `Decimal::parse` accepts can give.
///
/// The text comes back as written except for these forms: a `+` sign; an exponent, which is
/// written out with the same scale, so `25e-1` comes back as `2.5` and `1E+2` as `100`; `_`
/// separators; leading zeros of the integral part, as in `007.10`, which comes back as `7.10`; a
/// missing integral digit, as in `.5` for `0.5`; a point without fraction digits, as in `5.` for
/// `5`; and a negative zero, which a signed storage writes back without its sign, as in `-0.0` for
/// `0.0`, and an unsigned storage rejects as
/// [`Negative`](crate::ParseErrorKind::Negative).
///
/// [`Display`](fmt::Display) writes at least `scale` fraction digits, or the precision of the
/// format spec where that is larger: every significant digit of the value, padded with zeros,
/// never rounded or truncated, and never in exponent notation. The width, fill, alignment, `+` and
/// `0` flags behave as for the decimal. The zeros are written in chunks, so no precision needs a
/// buffer of its size. `{:e}` and `{:E}` are not implemented.
///
/// A value made from a decimal rather than from text has scale 0, so its text is the shortest exact
/// text of the decimal: [`From`], [`Default`], and the zero, limits, one and coefficients of the
/// traits below. [`new`](Self::new) takes any scale.
///
/// # Arithmetic
///
/// Every decimal trait of this crate is implemented for `Scaled<D>` once, for every decimal `D`
/// that implements it: [`FixedDecimal`], [`ConstFixedDecimal`], [`SignedFixedDecimal`],
/// [`UnsignedFixedDecimal`], [`DecimalOne`] and [`ConstDecimalOne`], and between `Scaled` types
/// [`ExactMul`], [`WidenTo`], [`ConvertTo`] and [`DivRound`]. Generic code bounded by them takes a
/// `Scaled` and a bare decimal alike, and the arithmetic of a `Scaled` is reached through them,
/// with the trait in scope. A `Scaled` relates only to a `Scaled`: no trait converts a bare decimal
/// into a `Scaled`, or multiplies one by the other.
///
/// A result has the value that the decimal's own operation gives, and the scale of the operation:
///
/// - a sum or a difference takes the larger scale of its operands;
/// - an exact product takes the sum of the scales, saturating at 255, which only limits the text:
///   the product is never rounded;
/// - [`quantize`](FixedDecimal::quantize) takes the scale of the step;
/// - [`widen`](WidenTo::widen) and [`try_convert`](ConvertTo::try_convert) keep the scale;
/// - [`round_to`](ConvertTo::round_to) and [`div_round`](DivRound::div_round) take the fraction
///   digits of the target type, so an exact `1.00 / 4` into a type with 8 fraction digits is
///   `0.25000000`, where the General Decimal Arithmetic would give `0.25`;
/// - negation and [`abs`](SignedFixedDecimal::abs) keep the scale;
/// - zero, the limits, one and a value made by [`from_repr`](FixedDecimal::from_repr) have scale
///   0, and `Scaled::new` gives a coefficient another scale.
///
/// A concrete `Scaled<Decimal<I, F, R>>` has every trait with no bound of its own. Its parsing is
/// defined for each storage, so code generic over the decimal `D` that computes with `Scaled<D>`
/// adds the bound `Scaled<D>: FixedDecimal`, as code generic over the storage adds
/// `Decimal<I, F, R>: FixedDecimal`. A `Scaled` over anything but a decimal of this crate, such as
/// a `Scaled<Scaled<D>>`, has none of the traits.
///
/// ```
/// use numo::{
///   Decimal, ExactMul, FixedDecimal, Scaled,
///   typenum::{U8, U10, U12},
/// };
///
/// type Price = Scaled<Decimal<U10, U8>>;
/// type Quantity = Scaled<Decimal<U12, U8>>;
///
/// // Generic code takes a `Scaled` and a bare decimal alike.
/// fn total<T: FixedDecimal>(values: &[T]) -> Option<T> {
///   values.iter().try_fold(T::zero(), |sum, value| sum.checked_add(value))
/// }
///
/// let prices: [Price; 2] = ["64250.5".parse()?, "0.25000".parse()?];
/// assert_eq!(total(&prices).unwrap().to_string(), "64250.75000");
/// let bare = prices.map(|price| *price.value());
/// assert_eq!(total(&bare).unwrap().to_string(), "64250.75");
///
/// // A product takes the sum of the scales.
/// let quantity: Quantity = "0.0015".parse()?;
/// assert_eq!(prices[0].exact_mul(&quantity).to_string(), "96.37575");
///
/// // Code generic over the decimal names the bound of the `Scaled`.
/// fn doubled<D>(value: D, scale: u8) -> Option<Scaled<D>>
/// where
///   D: FixedDecimal,
///   Scaled<D>: FixedDecimal,
/// {
///   let scaled = Scaled::new(value, scale);
///   scaled.checked_add(&scaled)
/// }
/// assert_eq!(doubled(bare[1], 2).unwrap().to_string(), "0.50");
/// # Ok::<(), numo::ParseError>(())
/// ```
///
/// # Layout
///
/// The scale is a plain `u8` beside the decimal, so a `Scaled` takes the size of its decimal plus
/// the padding to its alignment, and `Option<Scaled<D>>` has no niche: where an `i128` is aligned
/// to 16 bytes, `Scaled<Decimal<U20, U18>>` takes 32 bytes and its `Option` 48.
#[derive(Clone, Copy, Debug)]
pub struct Scaled<D> {
  value: D,
  scale: u8,
}

impl<D> Scaled<D> {
  /// Wraps `value` with the scale `scale`: the minimum number of fraction digits of its text. Any
  /// scale is allowed, also one beyond the fraction digits of `D`.
  #[must_use]
  #[inline]
  pub const fn new(value: D, scale: u8) -> Self {
    Self { value, scale }
  }

  /// Returns the decimal.
  #[must_use]
  #[inline]
  pub const fn value(&self) -> &D {
    &self.value
  }

  /// Returns the scale: the minimum number of fraction digits of the text.
  #[must_use]
  #[inline]
  pub const fn scale(&self) -> u8 {
    self.scale
  }
}

// Not `const`: stable Rust rejects a `const fn` that moves a field out of a value of a generic
// type, whose destructor it cannot evaluate (E0493).
impl<D> Scaled<D> {
  /// Returns the decimal, dropping the scale.
  #[must_use]
  #[inline]
  pub fn into_value(self) -> D {
    self.value
  }

  /// Splits the value into the decimal and its scale.
  #[must_use]
  #[inline]
  pub fn into_parts(self) -> (D, u8) {
    (self.value, self.scale)
  }
}

impl<D> From<D> for Scaled<D> {
  /// Wraps `value` with scale 0, so that its text is the shortest exact text of the decimal.
  #[inline]
  fn from(value: D) -> Self {
    Self::new(value, 0)
  }
}

impl<D> Default for Scaled<D>
where
  D: Default,
{
  /// Returns the default decimal, zero, with scale 0.
  #[inline]
  fn default() -> Self {
    Self::new(D::default(), 0)
  }
}

// Hand-written, because the derived implementations would compare the scale, which is
// presentation only.
impl<D> PartialEq for Scaled<D>
where
  D: PartialEq,
{
  #[inline]
  fn eq(&self, other: &Self) -> bool {
    self.value == other.value
  }
}

impl<D> Eq for Scaled<D> where D: Eq {}

impl<D> PartialOrd for Scaled<D>
where
  D: Ord,
{
  #[inline]
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl<D> Ord for Scaled<D>
where
  D: Ord,
{
  #[inline]
  fn cmp(&self, other: &Self) -> Ordering {
    self.value.cmp(&other.value)
  }
}

impl<D> Hash for Scaled<D>
where
  D: Hash,
{
  #[inline]
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.value.hash(state);
  }
}

impl<D> fmt::Display for Scaled<D>
where
  D: FixedDecimal,
{
  /// Writes at least `scale` fraction digits, or the precision of the format spec where that is
  /// larger, honoring width, fill, alignment, `+` and `0` as described on [`Scaled`]. Nothing is
  /// rounded or truncated.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let repr = self.value.clone().repr();
    let mut buf = [0; int::DIGIT_BUF_LEN];
    let start = repr.write_digits(&mut buf, Token::new());
    let scale = usize::from(self.scale);
    let min_fraction = f
      .precision()
      .map_or(scale, |precision| precision.max(scale));
    format::write(
      f,
      repr.is_negative(Token::new()),
      &buf[start..],
      D::FRACTION as usize,
      min_fraction,
    )
  }
}

/// The scale of a value rounded or divided into `T`: the fraction digits of `T`, of which a decimal
/// has at most 154.
fn fraction_scale<T: FixedDecimal>() -> u8 {
  u8::try_from(T::FRACTION).unwrap_or(u8::MAX)
}

// `parse` and `FromStr` are per storage, emitted by `storage_decimal!` beside `Decimal::parse`, so
// the implementations over a generic decimal require the `FromStr` that `FixedDecimal` needs.
impl<D> FixedDecimal for Scaled<D>
where
  D: FixedDecimal,
  Self: FromStr<Err = ParseError>,
{
  const INTEGRAL: u32 = D::INTEGRAL;
  const FRACTION: u32 = D::FRACTION;
  const DIGITS: u32 = D::DIGITS;
  const SIGNED: bool = D::SIGNED;

  type Repr = D::Repr;

  /// Returns the coefficient of the decimal, which does not depend on the scale.
  #[inline]
  fn repr(self) -> D::Repr {
    self.value.repr()
  }

  /// Creates a value from its coefficient with scale 0, or returns `None` if the coefficient is
  /// outside the range of `D`.
  #[inline]
  fn from_repr(repr: D::Repr) -> Option<Self> {
    D::from_repr(repr).map(Self::from)
  }

  /// Returns zero with scale 0.
  #[inline]
  fn zero() -> Self {
    Self::from(D::zero())
  }

  /// Returns the largest value with scale 0.
  #[inline]
  fn max_value() -> Self {
    Self::from(D::max_value())
  }

  /// Returns the smallest value with scale 0.
  #[inline]
  fn min_value() -> Self {
    Self::from(D::min_value())
  }

  #[inline]
  fn is_zero(&self) -> bool {
    self.value.is_zero()
  }

  #[inline]
  fn is_positive(&self) -> bool {
    self.value.is_positive()
  }

  /// Adds two values, returning `None` if the sum is outside the range of `D`. The sum takes the
  /// larger of the two scales.
  fn checked_add(&self, rhs: &Self) -> Option<Self> {
    let sum = self.value.checked_add(&rhs.value)?;
    Some(Self::new(sum, self.scale.max(rhs.scale)))
  }

  /// Subtracts `rhs` from `self`, returning `None` if the difference is outside the range of `D`.
  /// The difference takes the larger of the two scales.
  fn checked_sub(&self, rhs: &Self) -> Option<Self> {
    let difference = self.value.checked_sub(&rhs.value)?;
    Some(Self::new(difference, self.scale.max(rhs.scale)))
  }

  /// Rounds to a multiple of `step` with `mode`, as [`Decimal::quantize`](crate::Decimal::quantize)
  /// describes. The result takes the scale of `step`.
  fn quantize(&self, step: &Self, mode: RoundingMode) -> Result<Self, QuantizeError> {
    let quantized = self.value.quantize(&step.value, mode)?;
    Ok(Self::new(quantized, step.scale))
  }

  #[inline]
  fn is_multiple_of(&self, step: &Self) -> bool {
    self.value.is_multiple_of(&step.value)
  }
}

impl<D> ConstFixedDecimal for Scaled<D>
where
  D: ConstFixedDecimal,
  Self: FromStr<Err = ParseError>,
{
  /// Zero with scale 0.
  const ZERO: Self = Self::new(D::ZERO, 0);

  /// The largest value with scale 0.
  const MAX: Self = Self::new(D::MAX, 0);

  /// The smallest value with scale 0.
  const MIN: Self = Self::new(D::MIN, 0);
}

impl<D> Neg for Scaled<D>
where
  D: Neg<Output = D>,
{
  type Output = Self;

  /// Returns `-self`, with the same scale.
  #[inline]
  fn neg(self) -> Self {
    Self::new(-self.value, self.scale)
  }
}

impl<D> SignedFixedDecimal for Scaled<D>
where
  D: SignedFixedDecimal,
  Self: FromStr<Err = ParseError>,
{
  #[inline]
  fn is_negative(&self) -> bool {
    self.value.is_negative()
  }

  /// Returns `-self`, with the same scale.
  #[inline]
  fn neg(&self) -> Self {
    Self::new(SignedFixedDecimal::neg(&self.value), self.scale)
  }

  /// Returns the absolute value, with the same scale.
  #[inline]
  fn abs(&self) -> Self {
    Self::new(self.value.abs(), self.scale)
  }
}

impl<D> UnsignedFixedDecimal for Scaled<D>
where
  D: UnsignedFixedDecimal,
  Self: FromStr<Err = ParseError>,
{
}

impl<D> DecimalOne for Scaled<D>
where
  D: DecimalOne,
  Self: FromStr<Err = ParseError>,
{
  /// Returns one with scale 0.
  #[inline]
  fn one() -> Self {
    Self::from(D::one())
  }
}

impl<D> ConstDecimalOne for Scaled<D>
where
  D: ConstDecimalOne,
  Self: FromStr<Err = ParseError>,
{
  /// One with scale 0.
  const ONE: Self = Self::new(D::ONE, 0);
}

impl<D, D2> ExactMul<Scaled<D2>> for Scaled<D>
where
  D: ExactMul<D2>,
  Scaled<<D as ExactMul<D2>>::Output>: FixedDecimal,
{
  type Output = Scaled<<D as ExactMul<D2>>::Output>;

  /// Multiplies exactly. The product takes the sum of the two scales, saturating at 255; its value
  /// is never rounded.
  #[inline]
  fn exact_mul(&self, rhs: &Scaled<D2>) -> Self::Output {
    Scaled::new(
      self.value.exact_mul(&rhs.value),
      self.scale.saturating_add(rhs.scale),
    )
  }
}

impl<D, T> WidenTo<Scaled<T>> for Scaled<D>
where
  D: WidenTo<T>,
{
  /// Widens the decimal, keeping the scale.
  #[inline]
  fn widen(self) -> Scaled<T> {
    Scaled::new(self.value.widen(), self.scale)
  }
}

impl<D, T> ConvertTo<Scaled<T>> for Scaled<D>
where
  D: ConvertTo<T>,
  T: FixedDecimal,
{
  /// Converts the decimal exactly, keeping the scale.
  #[inline]
  fn try_convert(self) -> Result<Scaled<T>, ConvertError> {
    Ok(Scaled::new(self.value.try_convert()?, self.scale))
  }

  /// Rounds the decimal into `T`. The result takes the fraction digits of `T` as its scale.
  #[inline]
  fn round_to(self, mode: RoundingMode) -> Result<Scaled<T>, ConvertError> {
    Ok(Scaled::new(
      self.value.round_to(mode)?,
      fraction_scale::<T>(),
    ))
  }
}

impl<D, D2, T> DivRound<Scaled<D2>, Scaled<T>> for Scaled<D>
where
  D: DivRound<D2, T>,
  T: FixedDecimal,
{
  /// Divides and rounds into `T`. The quotient takes the fraction digits of `T` as its scale.
  #[inline]
  fn div_round(&self, rhs: &Scaled<D2>, mode: RoundingMode) -> Result<Scaled<T>, DivideError> {
    Ok(Scaled::new(
      self.value.div_round(&rhs.value, mode)?,
      fraction_scale::<T>(),
    ))
  }
}
