//! `arbitrary` support: a decimal and a rounding mode from unstructured bytes, for fuzzing.
//!
//! Each choice of the generator reads one byte through `Unstructured::int_in_range`, which maps the
//! byte to a choice by its remainder, so a choice among `n` values is exactly uniform only when `n`
//! divides 256. Which bytes give which value is not stable, as for `arbitrary` itself.

use ::arbitrary::{Arbitrary, Error, Result, Unstructured};

use crate::{
  Decimal, RoundingMode,
  layout::{Layout, SealedLayout, Storage},
  sample::{self, Source},
};

/// A source that reads one byte per choice from `data`. It keeps the first error that reading
/// reports, which the impls return instead of the value.
struct Bytes<'u, 'a> {
  data: &'u mut Unstructured<'a>,
  error: Option<Error>,
}

impl Bytes<'_, '_> {
  /// Returns `value`, or the first error that reading the bytes for it reported.
  fn finish<T>(self, value: T) -> Result<T> {
    match self.error {
      Some(error) => Err(error),
      None => Ok(value),
    }
  }
}

impl Source for Bytes<'_, '_> {
  fn below(&mut self, n: u32) -> u32 {
    match self.data.int_in_range(0..=n - 1) {
      Ok(value) => value,
      Err(error) => {
        if self.error.is_none() {
          self.error = Some(error);
        }
        0
      }
    }
  }
}

#[cfg_attr(docsrs, doc(cfg(feature = "arbitrary")))]
impl<'a, I, F, R> Arbitrary<'a> for Decimal<I, F, R>
where
  R: Storage,
  (I, F): Layout<R>,
{
  /// Draws a decimal as the crate documentation describes, reading one byte per choice. Once the
  /// data runs out, every further choice is the first one, so empty data gives zero.
  fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
    let mut source = Bytes {
      data: u,
      error: None,
    };
    let coeff = sample::coefficient::<I, F, R>(&mut source);
    // The coefficient has at most the digits of the type.
    source.finish(Decimal::from_coeff(coeff))
  }

  fn size_hint(depth: usize) -> (usize, Option<usize>) {
    let _ = depth;
    let digits = <(I, F) as SealedLayout<R>>::DIGITS as usize;
    // A choice among two or more values reads one byte, and a choice among one reads none, as
    // the number of digits and the number of trailing zeros do for a single digit. The shortest
    // path is the edge value zero: the branch and the edge kind. The longest is the digit-wise
    // branch with all the digits and trailing zeros: the branch, the number of digits, the digits,
    // whether to zero trailing digits, how many, and the sign of a signed storage. An edge value
    // takes the branch, the kind, at most one byte for its exponent and the sign, never more.
    let several = usize::from(digits >= 2);
    let longest = 1 + several + digits + 1 + several + usize::from(R::SIGNED);
    (2, Some(longest))
  }
}

#[cfg_attr(docsrs, doc(cfg(feature = "arbitrary")))]
impl<'a> Arbitrary<'a> for RoundingMode {
  /// Chooses a mode from one byte. Empty data gives [`Down`](RoundingMode::Down).
  fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
    let mut source = Bytes {
      data: u,
      error: None,
    };
    let mode = sample::mode(&mut source);
    source.finish(mode)
  }

  fn size_hint(depth: usize) -> (usize, Option<usize>) {
    let _ = depth;
    (1, Some(1))
  }
}

#[cfg(test)]
mod tests;
