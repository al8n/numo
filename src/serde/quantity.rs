//! Writes and reads an unsigned integer as Ethereum's JSON-RPC QUANTITY: a string of `0x` and the
//! lowercase hexadecimal digits of the value without leading zeros, `0x0` for zero.
//!
//! A value is written as the text of `{:#x}`, through `collect_str`, in every format, at every
//! width, and for a native integer; zero, and so every value of zero bits, is `0x0`, the one
//! QUANTITY of zero. It is read with `deserialize_str` in every format, from a string alone, as
//! go-ethereum's `hexutil` decodes a number: `0x` or `0X`, then at least one hexadecimal digit of
//! either case, without a leading zero unless the digits are `0`. This takes every QUANTITY that
//! the specification's pattern takes, and also `0X` and uppercase digits, as go-ethereum does. No
//! sign, `_` or whitespace reads, and neither does the empty string, which go-ethereum's JSON types
//! read as zero.
//!
//! Every other input gets the fixed message `expected a hexadecimal quantity as a string`. A string
//! without the prefix, the empty string included, gets
//! `hexadecimal quantity without the 0x prefix`, `0x` alone `hexadecimal quantity without digits`,
//! and digits that start with a zero `hexadecimal quantity with leading zero digits`. Then a byte
//! that is not a hexadecimal digit, a byte of a character outside ASCII included, gets the text of
//! std's [`ParseIntError`](core::num::ParseIntError) of an invalid digit, a value above the range
//! of the type that of a value too large, and zero into a non-zero type that of zero.
//!
//! A QUANTITY has no sign, so the module takes [`Uint`](crate::Uint),
//! [`NonZeroUint`](crate::NonZeroUint) and the native integers from `u8` to `u128` and their
//! `NonZero`.
//!
//! ```
//! # use serde_full as serde;
//! use numo::u256;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize, Debug, PartialEq)]
//! # #[serde(crate = "serde")]
//! struct Block {
//!   #[serde(with = "numo::serde::quantity::required")]
//!   number: u64,
//!   #[serde(with = "numo::serde::quantity::required")]
//!   difficulty: u256,
//! }
//!
//! let block = Block { number: 255, difficulty: u256::ZERO };
//! let json = serde_json::to_string(&block).unwrap();
//! assert_eq!(json, r#"{"number":"0xff","difficulty":"0x0"}"#);
//! assert_eq!(serde_json::from_str::<Block>(&json).unwrap(), block);
//!
//! // Either case reads, as go-ethereum reads it, and a leading zero does not.
//! let block = serde_json::from_str::<Block>(r#"{"number":"0XFF","difficulty":"0x0"}"#).unwrap();
//! assert_eq!(block.number, 255);
//! let error = serde_json::from_str::<Block>(r#"{"number":"0x0ff","difficulty":"0x0"}"#);
//! let error = error.unwrap_err().to_string();
//! assert!(error.starts_with("hexadecimal quantity with leading zero digits"));
//! ```

use super::QuantityCore;

/// The integers that [`quantity`](self) writes and reads: [`Uint`](crate::Uint) and
/// [`NonZeroUint`](crate::NonZeroUint) at every width and in every storage, the native integers
/// from `u8` to `u128`, and their `NonZero`.
///
/// This trait is sealed and cannot be implemented outside this crate.
#[diagnostic::on_unimplemented(
  message = "`numo::serde::quantity` takes an unsigned integer",
  note = "a QUANTITY has no sign: it takes `Uint`, `NonZeroUint`, `u8` to `u128` and their `NonZero`",
  note = "an `Option` field takes `numo::serde::quantity::optional`"
)]
pub trait Quantity: QuantityCore {}

/// The leaf for a field of an unsigned integer,
/// `#[serde(with = "numo::serde::quantity::required")]`.
pub mod required {
  use serde_core::{Deserializer, Serializer};

  use super::Quantity;
  use crate::layout::Token;

  /// Writes `value` as a QUANTITY, a string in every format.
  #[inline]
  pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
  where
    T: Quantity,
    S: Serializer,
  {
    T::write(value, serializer, Token::new())
  }

  /// Reads a value from a QUANTITY, as go-ethereum's `hexutil` decodes a number. Every input but a
  /// string gets the fixed message `expected a hexadecimal quantity as a string`.
  #[inline]
  pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
  where
    T: Quantity,
    D: Deserializer<'de>,
  {
    T::read(deserializer, Token::new())
  }
}

/// The leaf for an `Option` field of an unsigned integer,
/// `#[serde(default, with = "numo::serde::quantity::optional")]`, where `default` reads a missing
/// field as `None`.
pub mod optional {
  use serde_core::{Deserializer, Serializer};

  use super::Quantity;
  use crate::layout::Token;

  /// Writes `None` as the format writes a none, `null` in JSON, and `Some` as the QUANTITY of its
  /// value, a string in every format.
  #[inline]
  pub fn serialize<T, S>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
  where
    T: Quantity,
    S: Serializer,
  {
    T::write_option(value, serializer, Token::new())
  }

  /// Reads `None` from a none and from a unit, which serde's buffer gives for `null`, and `Some` of
  /// a value read as [`required`](super::required) reads it. Every other input gets the fixed
  /// message `expected a hexadecimal quantity as a string`.
  #[inline]
  pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
  where
    T: Quantity,
    D: Deserializer<'de>,
  {
    T::read_option(deserializer, Token::new())
  }
}
