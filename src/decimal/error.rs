use core::fmt;

/// The error returned when parsing a [`Decimal`](crate::Decimal) or a [`Scaled`](crate::Scaled)
/// fails.
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
      ParseErrorKind::Negative => "decimal is negative, and the target type is unsigned",
      ParseErrorKind::OutOfRange => "decimal out of range of the target type",
      ParseErrorKind::Inexact => "decimal cannot be represented exactly in the target type",
      ParseErrorKind::ScaleTooLarge => "decimal has more than 255 fraction digits",
    })
  }
}

impl core::error::Error for ParseError {}

/// Why parsing a [`Decimal`](crate::Decimal) or a [`Scaled`](crate::Scaled) failed.
///
/// [`Decimal::parse`](crate::Decimal::parse) and the `parse` of [`Scaled`](crate::Scaled) report
/// the first kind that applies, in the order of the variants. `Decimal::parse` never reports
/// [`ScaleTooLarge`](Self::ScaleTooLarge), and `Scaled`'s `parse` reports it only for a text that
/// `Decimal::parse` accepts: the value is decided before the scale, so `1e-99999999999999999999`
/// is inexact and `1e99999999999999999999` out of range for every type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::IsVariant)]
#[non_exhaustive]
pub enum ParseErrorKind {
  /// The input is empty.
  Empty,
  /// The input does not match the grammar of [`Decimal::parse`](crate::Decimal::parse).
  InvalidSyntax,
  /// The input is an infinity or a NaN, such as `inf`, `-Infinity`, `NaN` or `sNaN`, which a
  /// decimal cannot represent.
  NotFinite,
  /// The target has an unsigned storage, and the input is `-` followed by a finite number: one
  /// that the type holds, zero included, so `-0`, `-0.0` and `-0e5` are negative, or one that is
  /// out of range or inexact for it. A `-` followed by anything else, such as a second sign, is
  /// [`InvalidSyntax`](Self::InvalidSyntax), and a negative infinity or NaN is
  /// [`NotFinite`](Self::NotFinite).
  Negative,
  /// Once the exponent has moved the point, the integral part has more than `Integral` digits
  /// after its leading zeros.
  OutOfRange,
  /// Once the exponent has moved the point, a fraction digit beyond the first `Fraction` is not
  /// zero.
  Inexact,
  /// The scale of the text, the number of fraction digits written minus the exponent, is above
  /// 255, the largest that a [`Scaled`](crate::Scaled) keeps, as in `0.` followed by 256 zeros or
  /// in `0e-256`.
  ScaleTooLarge,
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
