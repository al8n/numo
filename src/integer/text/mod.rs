//! The text of [`Uint`] and [`Int`]: `FromStr`, which reads decimal digits as the native integers
//! do, the radix formats `Binary`, `Octal`, `LowerHex` and `UpperHex`, which write the two's
//! complement of a negative `Int` in its `Bits` bits, and the exponent formats `LowerExp` and
//! `UpperExp`, which round half to even as the native integers do; and the check of the radix of
//! `from_str_radix`.

use core::{fmt, num::ParseIntError, str::FromStr};

#[cfg(any(feature = "std", feature = "alloc"))]
use super::digits;
use super::{Int, Uint};
#[cfg(any(feature = "std", feature = "alloc"))]
use crate::width::IntCore;
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

// One item for every storage, which computes on the inline storage and allocates the text alone.
#[cfg(any(feature = "std", feature = "alloc"))]
impl<B: FitsBits<R>, R: UintRepr> Uint<B, R> {
  /// The text of the value in `radix`, from 2 to 36: its digits, with lowercase letters, and `"0"`
  /// for zero, with no sign and no prefix, which [`from_str_radix`](Self::from_str_radix) reads
  /// back. The radixes 2, 8, 16 and 10 give the text of `{:b}`, `{:o}`, `{:x}` and `Display`. The
  /// `String` is allocated once, with the capacity that the digits need: this is the form of
  /// `Display`'s text whose digits are on the heap.
  ///
  /// ```
  /// use numo::{Uint, typenum::U200, u7};
  ///
  /// assert_eq!(u7::new(100).to_str_radix(36), "2s");
  /// assert_eq!(u7::ZERO.to_str_radix(2), "0");
  /// let max = Uint::<U200>::MAX;
  /// assert_eq!(max.to_str_radix(10), max.to_string());
  /// assert_eq!(Uint::<U200>::from_str_radix(&max.to_str_radix(7), 7), Ok(max));
  /// ```
  ///
  /// # Panics
  ///
  /// Panics if `radix` is not from 2 to 36, reporting the caller.
  #[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
  #[must_use]
  #[inline]
  #[track_caller]
  pub fn to_str_radix(&self, radix: u32) -> std::string::String {
    digits::check_radix(
      radix,
      36,
      "to_str_radix: radix must lie in the range `[2, 36]`",
    );
    digits::write_text(self.repr.inline(Token::new()), false, radix, B::U32)
  }
}

// The magnitude is computed on the inline storage, so that a heap storage allocates the text alone.
#[cfg(any(feature = "std", feature = "alloc"))]
impl<B: FitsBits<R>, R: IntRepr> Int<B, R> {
  /// The text of the value in `radix`, from 2 to 36: `-` before the digits of the magnitude of a
  /// negative value, with lowercase letters, and `"0"` for zero, with no prefix, which
  /// [`from_str_radix`](Self::from_str_radix) reads back. The radix formats write the `Bits` bits
  /// of a negative value instead: `{:x}` of `i7` -1 is `7f`, and `to_str_radix(16)` is `-1`. The
  /// `String` is allocated once, with the capacity that the digits and the sign need.
  ///
  /// ```
  /// use numo::i7;
  ///
  /// assert_eq!(i7::new(-1).to_str_radix(16), "-1");
  /// assert_eq!(format!("{:x}", i7::new(-1)), "7f");
  /// assert_eq!(i7::MIN.to_str_radix(2), "-1000000");
  /// assert_eq!(i7::from_str_radix(&i7::MIN.to_str_radix(2), 2), Ok(i7::MIN));
  /// ```
  ///
  /// # Panics
  ///
  /// Panics if `radix` is not from 2 to 36, reporting the caller.
  #[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
  #[must_use]
  #[inline]
  #[track_caller]
  pub fn to_str_radix(&self, radix: u32) -> std::string::String {
    digits::check_radix(
      radix,
      36,
      "to_str_radix: radix must lie in the range `[2, 36]`",
    );
    let value = self.repr.inline(Token::new());
    let magnitude = value.unsigned_abs(Token::new());
    digits::write_text(&magnitude, value.is_negative(Token::new()), radix, B::U32)
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
      #[doc = concat!(
        "Writes the value in ",
        $digits,
        ", honoring `+`, the prefix of `#`, width,"
      )]
      /// fill, alignment and `0` as the native integers do, and ignoring the precision. A native
      /// storage writes as its native integer does, and an array writes its digits from the
      /// limbs, through a buffer of 64 bytes, in time linear in the digits.
      #[inline]
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.repr.fmt_pow2($shift, $upper, f, Token::new())
      }
    }

    impl<B: FitsBits<R>, R: IntRepr> fmt::$trait for Int<B, R> {
      #[doc = concat!(
        "Writes the `Bits` bits of the value, its two's complement, in ",
        $digits,
        ","
      )]
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
      #[doc = concat!(
        "Writes the value in scientific notation with `",
        $marker,
        "`, as the native"
      )]
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
      #[doc = concat!(
        "Writes the value in scientific notation with `",
        $marker,
        "`, as for [`Uint`],"
      )]
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
