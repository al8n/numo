//! Writes and reads an integer as the bytes of its value, least significant first, as
//! [`to_le_bytes`](crate::ByteArray::to_le_bytes) gives them: `ceil(Bits / 8)` bytes, in two's
//! complement for `Int`, extended by copies of the sign, so that the bits of the last byte above
//! the width are zeros for `Uint` and copies of the sign for `Int`.
//!
//! A value is written through `serialize_bytes`, as serde's bytes, in every format: JSON writes an
//! array of numbers, postcard and bincode the length and the bytes, CBOR a byte string and
//! MessagePack a `bin`. Zero bits write no bytes. It is read with `deserialize_bytes`, in every
//! format, from a byte string of exactly `ceil(Bits / 8)` bytes, borrowed or owned, or from a
//! sequence of as many integers from 0 to 255, such as a JSON array; the bits of the last byte
//! above the width must be zeros for `Uint` and copies of the sign for `Int`. serde_json gives a
//! string as its bytes when it reads the value directly, so the JSON string `"ab"` reads into
//! `Uint<U16>` as 25185, while through serde's buffer of an untagged enum or a flattened struct it
//! stays a string, which is rejected.
//!
//! Every other input gets the fixed message `expected an array of bytes`, and so does an element of
//! a sequence that is not an integer from 0 to 255. Bytes of another number get `expected n bytes`,
//! with `n` the bytes of the width, or `expected 1 byte` at a width of one byte. Then bits above
//! the width get the text of std's [`ParseIntError`](core::num::ParseIntError) of a value too
//! large, or of one too small for the bytes of a negative value below the range of an `Int`, and
//! zero into a non-zero type that of zero.
//!
//! [`fixed_hex`](super::fixed_hex) writes the bits of the width alone, as hexadecimal digits: it
//! writes -1 of `Int<U12>` as `0x0fff`, the pattern of its 12 bits, which is also the text of the
//! largest `Uint<U12>`. This module writes the bytes of the value extended by copies of the sign:
//! -1 of `Int<U12>` is `ff ff`, and the largest `Uint<U12>` is `ff 0f`.
//!
//! It takes the widths up to 8192 bits, at which the array of the bytes of a width exists, as for
//! `to_le_bytes`. [`be_bytes`](super::be_bytes) writes the same bytes, most significant first.
//!
//! ```
//! # use serde_full as serde;
//! use numo::{Int, Uint, typenum::U12};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize, Debug, PartialEq)]
//! # #[serde(crate = "serde")]
//! struct Sample {
//!   #[serde(with = "numo::serde::le_bytes::required")]
//!   level: Int<U12>,
//!   #[serde(with = "numo::serde::le_bytes::required")]
//!   mask: Uint<U12>,
//! }
//!
//! // -1 is its two's complement, extended by copies of the sign; the largest `Uint<U12>` is not.
//! let sample = Sample { level: Int::<U12>::new(-1), mask: Uint::<U12>::MAX };
//! let json = serde_json::to_string(&sample).unwrap();
//! assert_eq!(json, r#"{"level":[255,255],"mask":[255,15]}"#);
//! assert_eq!(serde_json::from_str::<Sample>(&json).unwrap(), sample);
//! // postcard writes the length and the bytes of each field.
//! assert_eq!(postcard::to_stdvec(&sample).unwrap(), [2, 255, 255, 2, 255, 15]);
//!
//! // The bits above the width are copies of the sign for `Int` and zeros for `Uint`.
//! assert!(serde_json::from_str::<Sample>(r#"{"level":[255,15],"mask":[255,15]}"#).is_err());
//! assert!(serde_json::from_str::<Sample>(r#"{"level":[255,255],"mask":[255,255]}"#).is_err());
//! ```

use super::LeBytesCore;

/// The integers that [`le_bytes`](self) writes and reads: [`Uint`](crate::Uint),
/// [`Int`](crate::Int), [`NonZeroUint`](crate::NonZeroUint) and [`NonZeroInt`](crate::NonZeroInt)
/// of up to 8192 bits, in every storage, the native integers from `u8` to `u128` and from `i8` to
/// `i128`, and their `NonZero`.
///
/// This trait is sealed and cannot be implemented outside this crate.
#[diagnostic::on_unimplemented(
  message = "`numo::serde::le_bytes` takes an integer of numo or a native integer",
  note = "an `Option` field takes `numo::serde::le_bytes::optional`"
)]
pub trait LeBytes: LeBytesCore {}

/// The leaf for a field of an integer, `#[serde(with = "numo::serde::le_bytes::required")]`.
pub mod required {
  use serde_core::{Deserializer, Serializer};

  use super::LeBytes;
  use crate::layout::Token;

  /// Writes `value` as the bytes of its value, least significant first, serde's bytes in every
  /// format.
  #[inline]
  pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
  where
    T: LeBytes,
    S: Serializer,
  {
    T::write(value, serializer, Token::new())
  }

  /// Reads a value from its bytes, least significant first: a byte string, or a sequence of
  /// integers from 0 to 255, of as many bytes as the width of `T` has. Every other input gets the
  /// fixed message `expected an array of bytes`.
  #[inline]
  pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
  where
    T: LeBytes,
    D: Deserializer<'de>,
  {
    T::read(deserializer, Token::new())
  }
}

/// The leaf for an `Option` field of an integer,
/// `#[serde(default, with = "numo::serde::le_bytes::optional")]`, where `default` reads a missing
/// field as `None`.
pub mod optional {
  use serde_core::{Deserializer, Serializer};

  use super::LeBytes;
  use crate::layout::Token;

  /// Writes `None` as the format writes a none, `null` in JSON, and `Some` as the bytes of its
  /// value, least significant first, serde's bytes in every format.
  #[inline]
  pub fn serialize<T, S>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
  where
    T: LeBytes,
    S: Serializer,
  {
    T::write_option(value, serializer, Token::new())
  }

  /// Reads `None` from a none and from a unit, which serde's buffer gives for `null`, and `Some` of
  /// a value read as [`required`](super::required) reads it. Every other input gets the fixed
  /// message `expected an array of bytes`.
  #[inline]
  pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
  where
    T: LeBytes,
    D: Deserializer<'de>,
  {
    T::read_option(deserializer, Token::new())
  }
}
