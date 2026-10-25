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
