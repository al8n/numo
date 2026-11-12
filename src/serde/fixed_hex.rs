//! Writes and reads an integer as a hexadecimal string of fixed width: `0x` and two lowercase
//! hexadecimal digits for each byte of the width, so that every value of a type has a text of the
//! same length.
//!
//! The digits are those of the `Bits` bits of the value, zero-padded on the left. A negative `Int`
//! writes the pattern of its bits, its two's complement in `Bits` bits: `Int<U12>` writes -1 as
//! `0x0fff`, the text of the largest `Uint<U12>`, and its smallest value as `0x0800`, while `i8`
//! writes -1 as `0xff` and `i64` as `0x` and sixteen `f`, and `u16` writes 255 as `0x00ff`. Zero
//! bits write `0x`. At a width of whole bytes the form is that of Ethereum's fixed DATA, such as
//! `bytes32` at 256 bits.
//!
//! A value is written through `collect_str` and read with `deserialize_str`, in every format, from
//! a string alone: `0x` or `0X`, then exactly two hexadecimal digits of either case for each byte
//! of the width, read as the pattern of the `Bits` bits, in two's complement for `Int`, so that
//! `0x0fff` is -1 for `Int<U12>` and 4095 for `Uint<U12>`. The bits of the first byte above the
//! width must be zeros, for both signs.
//!
//! Every other input gets the fixed message `expected a hexadecimal string`. A string without the
//! prefix gets `hexadecimal string without the 0x prefix`, and one with another number of digits
//! `expected n hexadecimal digits`, with `n` two for each byte of the width. Then a byte that is
//! not a hexadecimal digit, a byte of a character outside ASCII included, gets the text of std's
//! [`ParseIntError`](core::num::ParseIntError) of an invalid digit, bits above the width that of a
//! value too large, and zero into a non-zero type that of zero.
//!
//! ```
//! # use serde_full as serde;
//! use numo::{Int, Uint, typenum::U12};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize, Debug, PartialEq)]
//! # #[serde(crate = "serde")]
//! struct Sample {
//!   #[serde(with = "numo::serde::fixed_hex::required")]
//!   level: Int<U12>,
//!   #[serde(with = "numo::serde::fixed_hex::required")]
//!   mask: Uint<U12>,
//! }
//!
//! // -1 is the pattern of its 12 bits, the text of the largest `Uint<U12>`.
//! let sample = Sample { level: Int::<U12>::new(-1), mask: Uint::<U12>::MAX };
//! let json = serde_json::to_string(&sample).unwrap();
//! assert_eq!(json, r#"{"level":"0x0fff","mask":"0x0fff"}"#);
//! assert_eq!(serde_json::from_str::<Sample>(&json).unwrap(), sample);
//!
//! // The bits above the width are too large for both signs, and the digits are exactly four.
//! assert!(serde_json::from_str::<Sample>(r#"{"level":"0xffff","mask":"0x0fff"}"#).is_err());
//! assert!(serde_json::from_str::<Sample>(r#"{"level":"0xfff","mask":"0x0fff"}"#).is_err());
//! ```

use super::FixedHexCore;

/// The integers that [`fixed_hex`](self) writes and reads: [`Uint`](crate::Uint),
/// [`Int`](crate::Int), [`NonZeroUint`](crate::NonZeroUint) and [`NonZeroInt`](crate::NonZeroInt)
/// at every width and in every storage, the native integers from `u8` to `u128` and from `i8` to
/// `i128`, and their `NonZero`.
///
/// This trait is sealed and cannot be implemented outside this crate.
#[diagnostic::on_unimplemented(
  message = "`numo::serde::fixed_hex` takes an integer of numo or a native integer",
  note = "an `Option` field takes `numo::serde::fixed_hex::optional`"
)]
pub trait FixedHex: FixedHexCore {}

/// The leaf for a field of an integer, `#[serde(with = "numo::serde::fixed_hex::required")]`.
pub mod required {
  use serde_core::{Deserializer, Serializer};

  use super::FixedHex;
  use crate::layout::Token;

  /// Writes `value` as `0x` and two lowercase hexadecimal digits for each byte of its width, a
  /// string in every format.
  #[inline]
  pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
  where
    T: FixedHex,
    S: Serializer,
  {
    T::write(value, serializer, Token::new())
  }

  /// Reads a value from `0x` and two hexadecimal digits for each byte of the width of `T`, the
  /// pattern of its `Bits` bits. Every input but a string gets the fixed message
  /// `expected a hexadecimal string`.
  #[inline]
  pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
  where
    T: FixedHex,
    D: Deserializer<'de>,
  {
    T::read(deserializer, Token::new())
  }
}

/// The leaf for an `Option` field of an integer,
/// `#[serde(default, with = "numo::serde::fixed_hex::optional")]`, where `default` reads a missing
/// field as `None`.
pub mod optional {
  use serde_core::{Deserializer, Serializer};

  use super::FixedHex;
  use crate::layout::Token;

  /// Writes `None` as the format writes a none, `null` in JSON, and `Some` as `0x` and two
  /// lowercase hexadecimal digits for each byte of the width of its value, a string in every
  /// format.
  #[inline]
  pub fn serialize<T, S>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
  where
    T: FixedHex,
    S: Serializer,
  {
    T::write_option(value, serializer, Token::new())
  }

  /// Reads `None` from a none and from a unit, which serde's buffer gives for `null`, and `Some` of
  /// a value read as [`required`](super::required) reads it. Every other input gets the fixed
  /// message `expected a hexadecimal string`.
  #[inline]
  pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
  where
    T: FixedHex,
    D: Deserializer<'de>,
  {
    T::read_option(deserializer, Token::new())
  }
}
