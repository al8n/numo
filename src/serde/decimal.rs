//! Writes an integer as its decimal text, as a string in every format; a
//! [`Decimal`](crate::Decimal) writes its text already, and this module is for the integers.
//!
//! A value is written as the text of `Display`, such as `"5"` or `"-1"`, through `collect_str`, at
//! every width: also up to 128 bits, where the default writes a number, and for a native integer.
//! It is read with `deserialize_str` in every format, so that it keeps one form: a string in the
//! syntax of `FromStr`, an optional `+`, or `-` for a signed type, then ASCII digits, leading zeros
//! allowed; and an integer that a format gives for the request of a string, as toml, MessagePack
//! and serde_test give one, checked against the width. A JSON number is rejected, and so is a map,
//! serde_json's number token among them.
//!
//! It is the form for a value that the default does not read back or does not keep exact: a value
//! of 65 to 128 bits that JSON quotes, or that sits in an untagged enum, an internally tagged enum
//! or a flattened struct; a value outside the 128-bit range in a format that infers the type of a
//! value from its text, as csv does; a value above 2^53 for a reader whose numbers are 64-bit
//! floats; and every value in a format without `deserialize_any`, when the writer used this module
//! too.
//!
//! Every other input gets the fixed message `expected an integer as a decimal string`. A number
//! outside the range of the type gets the text of std's
//! [`TryFromIntError`](core::num::TryFromIntError), a string that does not parse that of std's
//! [`ParseIntError`](core::num::ParseIntError), and a non-zero type rejects zero after every other
//! error.
//!
//! ```
//! # use serde_full as serde;
//! use numo::{Int, Uint, typenum::U100};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize, Debug, PartialEq)]
//! # #[serde(crate = "serde")]
//! struct Account {
//!   #[serde(with = "numo::serde::decimal::required")]
//!   balance: Uint<U100>,
//!   // `default` reads a missing field as `None`.
//!   #[serde(default, with = "numo::serde::decimal::optional")]
//!   change: Option<Int<U100>>,
//! }
//!
//! let account = Account { balance: Uint::<U100>::MAX, change: Some(Int::<U100>::new(-5)) };
//! let json = serde_json::to_string(&account).unwrap();
//! assert_eq!(json, r#"{"balance":"1267650600228229401496703205375","change":"-5"}"#);
//! assert_eq!(serde_json::from_str::<Account>(&json).unwrap(), account);
//!
//! let account = serde_json::from_str::<Account>(r#"{"balance":"5"}"#).unwrap();
//! assert_eq!(account.change, None);
//! // A number is rejected, with a message that names nothing of it.
//! let error = serde_json::from_str::<Account>(r#"{"balance":5}"#).unwrap_err();
//! assert!(error.to_string().starts_with("expected an integer as a decimal string"));
//! ```

use super::DecimalStringCore;

/// The integers that [`decimal`](self) writes and reads: [`Uint`](crate::Uint),
/// [`Int`](crate::Int), [`NonZeroUint`](crate::NonZeroUint) and [`NonZeroInt`](crate::NonZeroInt)
/// at every width and in every storage, the native integers from `u8` to `u128` and from `i8` to
/// `i128`, and their `NonZero`.
///
/// This trait is sealed and cannot be implemented outside this crate.
#[diagnostic::on_unimplemented(
  message = "`numo::serde::decimal` takes an integer of numo or a native integer",
  note = "`Decimal` writes its decimal text already; this module is for the integers",
  note = "an `Option` field takes `numo::serde::decimal::optional`"
)]
pub trait DecimalString: DecimalStringCore {}

/// The leaf for a field of an integer, `#[serde(with = "numo::serde::decimal::required")]`.
pub mod required {
  use serde_core::{Deserializer, Serializer};

  use super::DecimalString;
  use crate::layout::Token;

  /// Writes `value` as its decimal text, a string in every format.
  #[inline]
  pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
  where
    T: DecimalString,
    S: Serializer,
  {
    T::write(value, serializer, Token::new())
  }

  /// Reads a value from a decimal string, or from an integer that the format gives for the request
  /// of a string, checked against the width of `T`. Every other input gets the fixed message
  /// `expected an integer as a decimal string`.
  #[inline]
  pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
  where
    T: DecimalString,
    D: Deserializer<'de>,
  {
    T::read(deserializer, Token::new())
  }
}

/// The leaf for an `Option` field of an integer,
/// `#[serde(default, with = "numo::serde::decimal::optional")]`, where `default` reads a missing
/// field as `None`.
pub mod optional {
  use serde_core::{Deserializer, Serializer};

  use super::DecimalString;
  use crate::layout::Token;

  /// Writes `None` as the format writes a none, `null` in JSON, and `Some` as the decimal text of
  /// its value, a string in every format.
  #[inline]
  pub fn serialize<T, S>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
  where
    T: DecimalString,
    S: Serializer,
  {
    T::write_option(value, serializer, Token::new())
  }

  /// Reads `None` from a none and from a unit, which serde's buffer gives for `null`, and `Some` of
  /// a value read as [`required`](super::required) reads it. Every other input gets the fixed
  /// message `expected an integer as a decimal string`.
  #[inline]
  pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
  where
    T: DecimalString,
    D: Deserializer<'de>,
  {
    T::read_option(deserializer, Token::new())
  }
}
