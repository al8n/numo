//! The text of [`Uint`] and [`Int`]: `FromStr`, which reads decimal digits as the native integers
//! do, the radix formats `Binary`, `Octal`, `LowerHex` and `UpperHex`, which write the two's
//! complement of a negative `Int` in its `Bits` bits, and the exponent formats `LowerExp` and
//! `UpperExp`, which round half to even as the native integers do; and the check of the radix of
//! `from_str_radix`.

use core::{fmt, num::ParseIntError, str::FromStr};

use super::{Int, Uint};
use crate::{
  layout::Token,
  width::{FitsBits, IntRepr, UintRepr},
};

/// Panics unless `radix` is from 2 to 36, as the native `from_str_radix` does, reporting the
/// caller.
#[track_caller]
pub(crate) const fn check_radix(radix: u32) {
  if !matches!(radix, 2..=36) {
    panic!("from_str_radix: radix must lie in the range `[2, 36]`");
  }
}

impl<B: FitsBits<R>, R: UintRepr> FromStr for Uint<B, R> {
  type Err = ParseIntError;

  /// Parses an integer from decimal digits after an optional `+`, as `from_str_radix` with the
  /// radix 10 does, with the errors of the native integers.
  #[inline]
  fn from_str(src: &str) -> Result<Self, ParseIntError> {
    R::from_str_radix_bits(src.as_bytes(), 10, B::U32, Token::new()).map(Self::wrap)
  }
}

impl<B: FitsBits<R>, R: IntRepr> FromStr for Int<B, R> {
  type Err = ParseIntError;

  /// Parses an integer from decimal digits after an optional `+` or `-`, as `from_str_radix` with
  /// the radix 10 does, with the errors of the native integers.
  #[inline]
  fn from_str(src: &str) -> Result<Self, ParseIntError> {
    R::from_str_radix_bits(src.as_bytes(), 10, B::U32, Token::new()).map(Self::wrap)
  }
}

/// Implements the radix formats `$trait` of [`Uint`] and [`Int`], written
/// `trait: shift, upper, digits;`, for the radix `2^shift` in `digits`, in uppercase if `upper`,
/// with the sealed storage methods, and the exponent formats, written `exp trait: upper, marker;`.
// Trait methods cannot be `const`.
macro_rules! text_formats {
  ($($trait:ident: $shift:literal, $upper:literal, $digits:literal);+ $(;)?) => {$(
    impl<B, R: UintRepr> fmt::$trait for Uint<B, R> {
      #[doc = concat!("Writes the value in ", $digits, ", honoring `+`, the prefix of `#`, width,")]
      /// fill, alignment and `0` as the native integers do, and ignoring the precision. A native
      /// storage writes as its native integer does, and an array writes its digits from the
      /// limbs, through a buffer of 64 bytes, in time linear in the digits.
      #[inline]
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.repr.fmt_pow2($shift, $upper, f, Token::new())
      }
    }

    impl<B: FitsBits<R>, R: IntRepr> fmt::$trait for Int<B, R> {
      #[doc = concat!("Writes the `Bits` bits of the value, its two's complement, in ", $digits, ",")]
      /// as the native integers write the bits of their width: a negative value writes the digits
      /// of `2^Bits` plus the value, whatever the storage holds above the width. The flags are
      /// those of [`Uint`].
      #[inline]
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.repr.fmt_pow2(B::U32, $shift, $upper, f, Token::new())
      }
    }
  )+};
  ($(exp $trait:ident: $upper:literal, $marker:literal);+ $(;)?) => {$(
    impl<B, R: UintRepr> fmt::$trait for Uint<B, R> {
      #[doc = concat!("Writes the value in scientific notation with `", $marker, "`, as the native")]
      /// integers do: the first digit and, after a point, the other digits without their trailing
      /// zeros, then the exponent; a precision gives exactly that many digits after the point,
      /// zeros added or the digits rounded half to even, which can carry into the exponent. Width,
      /// fill, alignment, `+` and `0` are honored as for `Display`. A native storage writes as its
      /// native integer does, and an array writes the decimal digits that `Display` writes, with a
      /// buffer of the same size.
      #[inline]
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.repr.fmt_exp($upper, f, Token::new())
      }
    }

    impl<B, R: IntRepr> fmt::$trait for Int<B, R> {
      #[doc = concat!("Writes the value in scientific notation with `", $marker, "`, as for [`Uint`],")]
      /// with `-` before a negative value.
      #[inline]
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.repr.fmt_exp($upper, f, Token::new())
      }
    }
  )+};
}

text_formats!(
  Binary: 1, false, "binary";
  Octal: 3, false, "octal";
  LowerHex: 4, false, "hexadecimal with lowercase digits";
  UpperHex: 4, true, "hexadecimal with uppercase digits";
);

text_formats!(
  exp LowerExp: false, "e";
  exp UpperExp: true, "E";
);
