//! Writes and reads an unsigned integer in the form of ruint's `Uint`, which alloy's `U256` has: a
//! QUANTITY, as [`quantity`](super::quantity) writes it, in a human-readable format, and the bytes
//! of the value, most significant first, as [`be_bytes`](super::be_bytes) writes them, in another.
//!
//! A value is written as ruint 1.21 writes it: in a human-readable format, such as JSON, the text
//! of `{:#x}`, `0x` and the lowercase hexadecimal digits without leading zeros, `0x0` for zero and
//! so at zero bits; in another, such as postcard or bincode, serde's bytes, `ceil(Bits / 8)` of
//! them, the most significant first.
//!
//! It is read as ruint reads its `Uint`, with `deserialize_any` in a human-readable format and with
//! `deserialize_bytes` in another, and one reader takes every form, whatever the format says of
//! itself: a string in ruint's grammar; every integer that the format gives, checked against the
//! width; and the bytes of the value, most significant first, as a byte string or a sequence of
//! integers from 0 to 255, exactly `ceil(Bits / 8)` of them, the bits of the first above the width
//! zeros. So serde's buffer of an untagged enum or a flattened struct, which says of every format
//! that it is human-readable, reads the bytes that a binary format buffered. Like ruint itself, the
//! form needs `deserialize_any` in a human-readable format.
//!
//! ruint's grammar, on the bytes of the text: the first two bytes `0x` or `0X`, `0o` or `0O`, and
//! `0b` or `0B` choose the radix 16, 8 or 2, and any other text is decimal; then digits of the
//! radix, of either case, any number of leading zeros, and `_` anywhere among them, which counts
//! for nothing. No sign or whitespace reads, and a text without digits, such as `""`, `"0x"` or
//! `"_"`, is zero. A `char` reads as the string of that one character, as ruint reads it.
//!
//! It reads everything that ruint 1.21 reads, and more in three places: at zero bits ruint reads
//! `"0x0"` alone, and this module every string whose value in ruint's grammar is zero; ruint takes
//! the integers of a format as unsigned ones alone, `u64` and `u128`, so toml's, which are `i64`,
//! fail there and read here; and ruint reads bytes only in a format that is not human-readable,
//! while this module reads bytes and a sequence of bytes in every format: a value of CBOR that an
//! untagged enum buffered, and a JSON array of exactly `ceil(Bits / 8)` integers, such as `[5]`
//! into `u8` and `[]` at zero bits.
//!
//! Every other input gets the fixed message `expected an integer as a number, a string or bytes`.
//! A byte of a string that is neither `_` nor a digit of its radix, a byte of a character outside
//! ASCII included, gets the text of std's [`ParseIntError`](core::num::ParseIntError) of an invalid
//! digit, a string above the range of the type that of a value too large, and zero into a non-zero
//! type that of zero. Bytes of another number get `expected n bytes`, with `n` the bytes of the
//! width, or `expected 1 byte` at a width of one byte, and bits above the width the text of a value
//! too large. A number outside the range of the type gets the text of std's
//! [`TryFromIntError`](core::num::TryFromIntError).
//!
//! It takes the widths up to 8192 bits, at which the array of the bytes of a width exists.
//!
//! ```
//! # use serde_full as serde;
//! use numo::u256;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize, Debug, PartialEq)]
//! # #[serde(crate = "serde")]
//! struct Transfer {
//!   #[serde(with = "numo::serde::ruint::required")]
//!   amount: u256,
//! }
//!
//! let transfer = Transfer { amount: u256::from(255u8) };
//! // A QUANTITY in JSON, and the 32 bytes of the value in postcard, after their length.
//! assert_eq!(serde_json::to_string(&transfer).unwrap(), r#"{"amount":"0xff"}"#);
//! let bytes = postcard::to_stdvec(&transfer).unwrap();
//! assert_eq!((bytes.len(), bytes[0], bytes[32]), (33, 32, 255));
//! assert_eq!(postcard::from_bytes::<Transfer>(&bytes).unwrap(), transfer);
//!
//! // ruint's grammar: padded hexadecimal, decimal, octal and binary with `_`, and a JSON number.
//! for amount in [r#""0x00ff""#, r#""255""#, r#""0o377""#, r#""0b1111_1111""#, "255"] {
//!   let json = format!(r#"{{"amount":{amount}}}"#);
//!   assert_eq!(serde_json::from_str::<Transfer>(&json).unwrap(), transfer);
//! }
//! ```

use super::RuintFormCore;

/// The unsigned integers that [`ruint`](self) writes and reads: [`Uint`](crate::Uint) and
/// [`NonZeroUint`](crate::NonZeroUint) of up to 8192 bits, in every storage, the native integers
/// from `u8` to `u128`, and their `NonZero`.
///
/// This trait is sealed and cannot be implemented outside this crate.
#[diagnostic::on_unimplemented(
  message = "`numo::serde::ruint` takes an unsigned integer",
  note = "an `Option` field takes `numo::serde::ruint::optional`"
)]
pub trait RuintForm: RuintFormCore {}

/// The leaf for a field of an unsigned integer, `#[serde(with = "numo::serde::ruint::required")]`.
pub mod required {
  use serde_core::{Deserializer, Serializer};

  use super::RuintForm;
  use crate::layout::Token;

  /// Writes `value` in ruint's form: as a QUANTITY in a human-readable format, and as the bytes of
  /// its value, most significant first, in another.
  #[inline]
  pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
  where
    T: RuintForm,
    S: Serializer,
  {
    T::write(value, serializer, Token::new())
  }

  /// Reads a value in ruint's form, whatever the format says of itself: a string in ruint's
  /// grammar, an integer that the format gives, or the bytes of the value, most significant first.
  /// Every other input gets the fixed message `expected an integer as a number, a string or bytes`.
  #[inline]
  pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
  where
    T: RuintForm,
    D: Deserializer<'de>,
  {
    T::read(deserializer, Token::new())
  }
}

/// The leaf for an `Option` field of an unsigned integer,
/// `#[serde(default, with = "numo::serde::ruint::optional")]`, where `default` reads a missing
/// field as `None`.
pub mod optional {
  use serde_core::{Deserializer, Serializer};

  use super::RuintForm;
  use crate::layout::Token;

  /// Writes `None` as the format writes a none, `null` in JSON, and `Some` of a value in ruint's
  /// form.
  #[inline]
  pub fn serialize<T, S>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
  where
    T: RuintForm,
    S: Serializer,
  {
    T::write_option(value, serializer, Token::new())
  }

  /// Reads `None` from a none and from a unit, which serde's buffer gives for `null`, and `Some` of
  /// a value read as [`required`](super::required) reads it. Every other input gets the fixed
  /// message `expected an integer as a number, a string or bytes`.
  #[inline]
  pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
  where
    T: RuintForm,
    D: Deserializer<'de>,
  {
    T::read_option(deserializer, Token::new())
  }
}
