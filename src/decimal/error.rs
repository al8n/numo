use core::fmt;

/// The error returned when parsing a [`Decimal`](crate::Decimal) fails.
///
/// The error does not contain the input; [`kind`](Self::kind) tells why parsing failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParseError {
  kind: ParseErrorKind,
}

impl ParseError {
  pub(crate) const fn new(kind: ParseErrorKind) -> Self {
    Self { kind }
  }

  /// Returns why parsing failed.
  #[must_use]
  pub const fn kind(&self) -> ParseErrorKind {
    self.kind
  }
}

impl fmt::Display for ParseError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(match self.kind {
      ParseErrorKind::Empty => "cannot parse decimal from empty string",
      ParseErrorKind::InvalidSyntax => "invalid decimal syntax",
      ParseErrorKind::NotFinite => "decimal cannot represent infinity or NaN",
      ParseErrorKind::OutOfRange => "decimal out of range of the target type",
      ParseErrorKind::Inexact => "decimal cannot be represented exactly in the target type",
    })
  }
}

impl core::error::Error for ParseError {}

/// Why parsing a [`Decimal`](crate::Decimal) failed.
///
/// [`Decimal::parse`](crate::Decimal::parse) reports the first kind that applies, in the order
/// of the variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::IsVariant)]
#[non_exhaustive]
pub enum ParseErrorKind {
  /// The input is empty.
  Empty,
  /// The input does not match the grammar of [`Decimal::parse`](crate::Decimal::parse), or the
  /// target has an unsigned storage and the input starts with `-`.
  InvalidSyntax,
  /// The input is an infinity or a NaN, such as `inf`, `-Infinity`, `NaN` or `sNaN`, which a
  /// decimal cannot represent.
  NotFinite,
  /// Once the exponent has moved the point, the integral part has more than `Integral` digits
  /// after its leading zeros.
  OutOfRange,
  /// Once the exponent has moved the point, a fraction digit beyond the first `Fraction` is not
  /// zero.
  Inexact,
}

/// The error returned by [`Decimal::try_convert`](crate::Decimal::try_convert) and
/// [`Decimal::round_to`](crate::Decimal::round_to), which returns only
/// [`OutOfRange`](Self::OutOfRange).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::IsVariant)]
#[non_exhaustive]
pub enum ConvertError {
  /// The value is outside the range of the target type.
  OutOfRange,
  /// The target type has fewer fraction digits, and a dropped digit is not zero.
  Inexact,
}

impl fmt::Display for ConvertError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(match self {
      Self::OutOfRange => "decimal out of range of the target type",
      Self::Inexact => "decimal cannot be represented exactly in the target type",
    })
  }
}

impl core::error::Error for ConvertError {}

/// The error returned by [`Decimal::quantize`](crate::Decimal::quantize).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::IsVariant)]
#[non_exhaustive]
pub enum QuantizeError {
  /// The step is zero or negative.
  InvalidStep,
  /// The selected multiple of the step is outside the range of the type.
  OutOfRange,
}

impl fmt::Display for QuantizeError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(match self {
      Self::InvalidStep => "step must be positive",
      Self::OutOfRange => "quantized decimal out of range of the type",
    })
  }
}

impl core::error::Error for QuantizeError {}

/// The error returned by [`Decimal::div_round`](crate::Decimal::div_round).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::IsVariant)]
#[non_exhaustive]
pub enum DivideError {
  /// The divisor is zero.
  DivisionByZero,
  /// The rounded quotient is outside the range of the target type.
  OutOfRange,
}

impl fmt::Display for DivideError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(match self {
      Self::DivisionByZero => "division by zero",
      Self::OutOfRange => "quotient out of range of the target type",
    })
  }
}

impl core::error::Error for DivideError {}
