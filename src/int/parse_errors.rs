//! The errors of the parsers of the integers and of the non-zero integers: the values of
//! `core::num::ParseIntError` that the native `from_str_radix` returns.
//!
//! `ParseIntError` has no public constructor, so each constant is the error that the native parser
//! returns for a fixed text, made in a constant context. The `match` of a constant accepts an error
//! of any kind, and only an `Ok` reaches its `panic!`: the `const` assertion beside each constant
//! pins its kind, so these assertions, not the `Ok` arm, fail the build if the native parser ever
//! returns another kind for that text.

use core::num::{IntErrorKind, NonZero, ParseIntError};

/// The error of an empty text.
pub(crate) const EMPTY: ParseIntError = match u8::from_str_radix("", 10) {
  Err(error) => error,
  Ok(_) => panic!("an empty text parses"),
};

const _: () = assert!(matches!(EMPTY.kind(), IntErrorKind::Empty));

/// The error of a byte that is not a digit of the radix, and of a lone sign.
pub(crate) const INVALID_DIGIT: ParseIntError = match u8::from_str_radix("-", 10) {
  Err(error) => error,
  Ok(_) => panic!("a lone sign parses"),
};

const _: () = assert!(matches!(INVALID_DIGIT.kind(), IntErrorKind::InvalidDigit));

/// The error of digits above the largest value.
pub(crate) const POS_OVERFLOW: ParseIntError = match u8::from_str_radix("256", 10) {
  Err(error) => error,
  Ok(_) => panic!("256 fits in a `u8`"),
};

const _: () = assert!(matches!(POS_OVERFLOW.kind(), IntErrorKind::PosOverflow));

/// The error of digits below the smallest value.
pub(crate) const NEG_OVERFLOW: ParseIntError = match i8::from_str_radix("-129", 10) {
  Err(error) => error,
  Ok(_) => panic!("-129 fits in an `i8`"),
};

const _: () = assert!(matches!(NEG_OVERFLOW.kind(), IntErrorKind::NegOverflow));

/// The error of digits of zero for a non-zero integer.
pub(crate) const ZERO: ParseIntError = match NonZero::<u8>::from_str_radix("0", 10) {
  Err(error) => error,
  Ok(_) => panic!("zero parses as a non-zero integer"),
};

const _: () = assert!(matches!(ZERO.kind(), IntErrorKind::Zero));

/// The error of digits outside the range: below the smallest value if `negative`, which only a
/// text with `-` reaches, and above the largest value otherwise.
#[inline]
pub(crate) const fn overflow(negative: bool) -> ParseIntError {
  if negative { NEG_OVERFLOW } else { POS_OVERFLOW }
}
