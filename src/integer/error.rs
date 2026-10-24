use core::fmt;

/// The error returned by `try_new` of [`Uint`](crate::Uint::try_new) and
/// [`Int`](crate::Int::try_new) when the value does not fit in `Bits` bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct TryNewError;

impl fmt::Display for TryNewError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("the value does not fit in `Bits` bits")
  }
}

impl core::error::Error for TryNewError {}
