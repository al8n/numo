//! Asks the format for the type that the default of an integer writes, and reads only the form that
//! it writes: up to 128 bits the native integer of the class of the width, and above a string.
//!
//! A value is written as the default writes it, which [`serde`](crate::Uint#serde) describes: up to
//! 128 bits through the method of the smallest native integer of its sign that holds the width,
//! and above 128 bits as its decimal text. It is read in every format through the method of that
//! native integer up to 128 bits, and of a string above, without asking whether the format is
//! human-readable, and only that form reads: up to 128 bits every integer that a format gives,
//! checked against the width, and above 128 bits a decimal string in the syntax of `FromStr`. A
//! string up to 128 bits, an integer above, a map, serde_json's number token among them, and every
//! other input get the fixed message `expected an integer`. A number outside the range of the type
//! gets the text of std's [`TryFromIntError`](core::num::TryFromIntError), a string that does not
//! parse that of std's [`ParseIntError`](core::num::ParseIntError), and a non-zero type rejects
//! zero after every other error.
//!
//! It serves a format that cannot describe its values: in a human-readable format without
//! `deserialize_any`, such as serde-json-core, the default reads no value of 0 to 64 bits or of
//! more than 128 bits, and this module reads them, while a value of 65 to 128 bits stays unreadable
//! there, as a `u128` does. In a format that infers the type of a value from its text, as csv does,
//! it asks for a string above 128 bits, and so reads every value back. And it serves a field that
//! must not take the other form: it reads what the native integer of the class reads, in every
//! format and every build, so no string up to 128 bits, also where a format hands one over for the
//! request of an integer, as toml does, and `serde_json::Value` in a build with
//! `arbitrary_precision`; and above 128 bits a decimal string alone.
//!
//! It takes numo's integers alone: a native integer reads by its type already, through its own
//! `Deserialize`.
//!
//! ```
//! # use serde_full as serde;
//! use numo::{u7, u256};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize, Debug, PartialEq)]
//! # #[serde(crate = "serde")]
//! struct Limits {
//!   #[serde(with = "numo::serde::typed::required")]
//!   level: u7,
//!   #[serde(with = "numo::serde::typed::required")]
//!   total: u256,
//! }
//!
//! // The form of the defaults: a number up to 128 bits, and a string above.
//! let limits = Limits { level: u7::new(5), total: u256::from(5u8) };
//! let json = serde_json::to_string(&limits).unwrap();
//! assert_eq!(json, r#"{"level":5,"total":"5"}"#);
//! assert_eq!(serde_json::from_str::<Limits>(&json).unwrap(), limits);
//!
//! // The default of `u7` reads a string too; `typed` does not, as `u8` does not.
//! assert!(serde_json::from_str::<Limits>(r#"{"level":"5","total":"5"}"#).is_err());
//! assert!(serde_json::from_str::<Limits>(r#"{"level":5,"total":5}"#).is_err());
//! ```

use super::TypedCore;

/// The integers that [`typed`](self) writes and reads: [`Uint`](crate::Uint), [`Int`](crate::Int),
/// [`NonZeroUint`](crate::NonZeroUint) and [`NonZeroInt`](crate::NonZeroInt) at every width and in
/// every storage.
///
/// This trait is sealed and cannot be implemented outside this crate.
#[diagnostic::on_unimplemented(
  message = "`numo::serde::typed` takes an integer of numo",
  note = "the native integers read by type already, through their own `Deserialize`",
  note = "an `Option` field takes `numo::serde::typed::optional`"
)]
pub trait Typed: TypedCore {}

/// The leaf for a field of an integer, `#[serde(with = "numo::serde::typed::required")]`.
pub mod required {
  use serde_core::{Deserializer, Serializer};

  use super::Typed;
  use crate::layout::Token;

  /// Writes `value` as the default writes it: up to 128 bits as the native integer of its class,
  /// and above as its decimal text.
  #[inline]
  pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
  where
    T: Typed,
    S: Serializer,
  {
    T::write(value, serializer, Token::new())
  }

  /// Reads a value in the form that the default writes alone, asking every format for the native
  /// integer of the class of `T` up to 128 bits and for a string above. Every other input gets the
  /// fixed message `expected an integer`.
  #[inline]
  pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
  where
    T: Typed,
    D: Deserializer<'de>,
  {
    T::read(deserializer, Token::new())
  }
}

/// The leaf for an `Option` field of an integer,
/// `#[serde(default, with = "numo::serde::typed::optional")]`, where `default` reads a missing
/// field as `None`.
pub mod optional {
  use serde_core::{Deserializer, Serializer};

  use super::Typed;
  use crate::layout::Token;

  /// Writes `None` as the format writes a none, `null` in JSON, and `Some` as the default writes
  /// its value.
  #[inline]
  pub fn serialize<T, S>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
  where
    T: Typed,
    S: Serializer,
  {
    T::write_option(value, serializer, Token::new())
  }

  /// Reads `None` from a none and from a unit, which serde's buffer gives for `null`, and `Some` of
  /// a value read as [`required`](super::required) reads it. Every other input gets the fixed
  /// message `expected an integer`.
  #[inline]
  pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
  where
    T: Typed,
    D: Deserializer<'de>,
  {
    T::read_option(deserializer, Token::new())
  }
}
