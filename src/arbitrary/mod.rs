//! `arbitrary` support: a decimal and a rounding mode from unstructured bytes, for fuzzing.
//!
//! Each choice of the generator reads one byte through `Unstructured::int_in_range`, or two for a
//! choice among more than 256 values, which only a type of more than 256 digits makes. It maps the
//! bytes to a choice by their remainder, so a choice among `n` values is exactly uniform only when
//! `n` divides 256, or 65536 for two bytes. Which bytes give which value is not stable, as for
//! `arbitrary` itself.

use ::arbitrary::{Arbitrary, Error, Result, Unstructured};

use crate::{
  Decimal, RoundingMode,
  layout::{Layout, LayoutDigits, Storage, Token},
  sample::{self, Source},
};

/// A source that reads one byte per choice from `data`, or two for a choice among more than 256
/// values. It keeps the first error that reading reports, which the impls return instead of the
/// value.
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
    // `int_in_range` reads one byte for at most 256 values, and two for more, up to the 512 of a
    // choice.
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
  /// Draws a decimal as the crate documentation describes, reading one byte per choice, or two for
  /// a choice among more than 256 values, which only a type of more than 256 digits makes. Once
  /// the data runs out, every further choice is the first one, so empty data gives zero.
  fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
    let mut source = Bytes {
      data: u,
      error: None,
    };
    let coeff = sample::coefficient::<I, F, R>(&mut source);
    // The coefficient has at most the digits of the type.
    source.finish(Decimal::from_coeff(R::from_twin(coeff, Token::new())))
  }

  fn size_hint(depth: usize) -> (usize, Option<usize>) {
    let _ = depth;
    let digits = <(I, F) as LayoutDigits<R>>::DIGITS as usize;
    // A choice among two to 256 values reads one byte, a choice among more reads two, and a choice
    // among one reads none, as the number of digits and the number of trailing zeros do for a
    // single digit. The shortest path is the edge value zero: the branch and the edge kind. The
    // longest is the digit-wise branch with all the digits and trailing zeros: the branch, the
    // number of digits, the digits, whether to zero trailing digits, how many, and the sign of a
    // signed storage. An edge value takes the branch, the kind, at most one byte for its exponent,
    // or two for a type of more than 256 digits, and the sign, never more.
    let count = usize::from(digits >= 2) + usize::from(digits > 256);
    let longest = 1 + count + digits + 1 + count + usize::from(R::SIGNED);
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
