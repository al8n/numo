//! A decimal that keeps the scale of its text.

use core::{
  cmp::Ordering,
  fmt,
  hash::{Hash, Hasher},
};

use super::{FixedDecimal, format};
use crate::{
  int,
  layout::{StorageCore, Token},
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
///   Decimal, Scaled,
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
/// text of the decimal: [`From`] and [`Default`]. [`new`](Self::new) takes any scale.
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
