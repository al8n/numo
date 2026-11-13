use core::fmt;

/// The error returned by `try_new` of [`Uint`](crate::Uint::try_new) and
/// [`Int`](crate::Int::try_new) when the value does not fit in `Bits` bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[error("the value does not fit in {0} bits")]
#[repr(transparent)]
pub struct TryNewError(u32);

impl TryNewError {
  /// Returns the number of bits that the value does not fit in.
  #[inline(always)]
  pub const fn bits(&self) -> u32 {
    self.0
  }

  #[inline(always)]
  pub(crate) const fn new(bits: u32) -> Self {
    Self(bits)
  }
}

/// The error returned by the conversions of a float to [`Uint`] and [`Int`] that report why the
/// float has no value in the target type: [`round_from_f64`](crate::Uint::round_from_f64), which
/// rounds the float to an integer, [`exact_from_f64`](crate::Uint::exact_from_f64), which takes
/// only an integer, and their `f32` forms.
///
/// The error does not contain the float. NaN, the infinities, a value above the range and one below
/// it, as a negative value is for [`Uint`], are reported apart.
///
/// [`Uint`]: crate::Uint
/// [`Int`]: crate::Int
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::IsVariant)]
#[non_exhaustive]
pub enum FromFloatError {
  /// The float is NaN.
  Nan,
  /// The float is an infinity, positive or negative.
  Infinite,
  /// The value is above `MAX`: the rounded value for `round_from_f64`, and the value of the float
  /// itself for `exact_from_f64`, so that `255.5` is above the range of 8 unsigned bits.
  PosOverflow,
  /// The value is below `MIN`, as a negative value is for an unsigned integer: the rounded value
  /// for `round_from_f64`, so that a negative value that rounds to zero converts, and the value of
  /// the float itself for `exact_from_f64`, so that `-0.5` is below the range of an unsigned one.
  NegOverflow,
  /// The value is in the range of the target type and has a fraction, which only `exact_from_f64`
  /// reports.
  Inexact,
}

impl fmt::Display for FromFloatError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(match self {
      Self::Nan => "NaN cannot be converted to the target type",
      Self::Infinite => "infinity cannot be converted to the target type",
      Self::PosOverflow => "number too large to fit in target type",
      Self::NegOverflow => "number too small to fit in target type",
      Self::Inexact => "the value is not an integer",
    })
  }
}

impl core::error::Error for FromFloatError {}
