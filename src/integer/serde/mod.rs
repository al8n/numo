//! `Serialize` and `Deserialize` of the integers: the form of a value depends only on its width,
//! its sign and its value, never on its storage.
//!
//! Up to 128 bits, a value is written through the method of its class, the smallest native integer
//! of its sign that holds the width, and above 128 bits as its decimal text, through `collect_str`,
//! which streams `Display` with no buffer and no allocation. The value of a class is the low 128
//! bits of the words that the casts read, cut to the class with `as`.
//!
//! Reading asks a human-readable format for any value at 0 to 64 bits and above 128 bits, and
//! another format for the method of the class, or for a string above 128 bits; from 65 to 128 bits
//! it asks every format for a 128-bit integer, which serde_json reads exactly only when asked for
//! one. One visitor reads every class, through [`read`]: every integer that a format gives, checked
//! against the width with the casts, and a decimal string in the syntax of `FromStr`. Every other
//! input gets the fixed message `expected an integer`, and a rejected value the text of the error
//! of the casts or of `FromStr`, so that no error that numo builds names the input.

use core::{
  fmt,
  marker::PhantomData,
  num::{ParseIntError, TryFromIntError},
  str::FromStr,
};

use serde_core::{
  Deserialize, Deserializer, Serialize, Serializer,
  de::{self, EnumAccess, MapAccess, SeqAccess, Visitor},
};

use super::{
  Int, Uint,
  cast::{CastSource, CheckedTarget, try_cast},
};
use crate::{
  serde::{Expected, read, reject_inputs},
  width::{self, FitsBits, IntRepr, UintRepr},
};

/// What the defaults read, whose fixed message is `expected an integer`.
const INTEGER: &str = "an integer";

/// A value of the right kind that the defaults reject, which prints the text of its error.
enum Rejection {
  /// A number outside the range of the target, with the error of the casts.
  Range(TryFromIntError),
  /// A string that `FromStr` rejects, with its error.
  Parse(ParseIntError),
}

impl fmt::Display for Rejection {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Range(error) => fmt::Display::fmt(error, f),
      Self::Parse(error) => fmt::Display::fmt(error, f),
    }
  }
}

/// Writes `value` through the method of its class up to 128 bits, and as its decimal text above.
fn serialize_integer<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
  T: CastSource + fmt::Display,
  S: Serializer,
{
  // The low 128 bits of the value, in two's complement for a signed type. `as` cuts them to the
  // class exactly, since the value has no more bits than the class in every storage.
  let low = || width::u128_of_words(|index| value.word(index), T::BITS);
  match (T::BITS, T::SIGNED) {
    (0..=8, false) => serializer.serialize_u8(low() as u8),
    (9..=16, false) => serializer.serialize_u16(low() as u16),
    (17..=32, false) => serializer.serialize_u32(low() as u32),
    (33..=64, false) => serializer.serialize_u64(low() as u64),
    (65..=128, false) => serializer.serialize_u128(low()),
    (0..=8, true) => serializer.serialize_i8(low() as i8),
    (9..=16, true) => serializer.serialize_i16(low() as i16),
    (17..=32, true) => serializer.serialize_i32(low() as i32),
    (33..=64, true) => serializer.serialize_i64(low() as i64),
    (65..=128, true) => serializer.serialize_i128(low() as i128),
    _ => serializer.collect_str(value),
  }
}

/// Reads a value of `T` with [`IntegerVisitor`], asking the format for what the width and the sign
/// of `T` call for.
fn deserialize_integer<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
  T: CheckedTarget + FromStr<Err = ParseIntError>,
  D: Deserializer<'de>,
{
  read(deserializer, INTEGER, |deserializer, expected| {
    let visitor = IntegerVisitor::<T> {
      expected,
      marker: PhantomData,
    };
    match (T::BITS, T::SIGNED) {
      (65..=128, false) => deserializer.deserialize_u128(visitor),
      (65..=128, true) => deserializer.deserialize_i128(visitor),
      // A human-readable format gives a number or a string, as it describes its values.
      _ if deserializer.is_human_readable() => deserializer.deserialize_any(visitor),
      (0..=8, false) => deserializer.deserialize_u8(visitor),
      (9..=16, false) => deserializer.deserialize_u16(visitor),
      (17..=32, false) => deserializer.deserialize_u32(visitor),
      (33..=64, false) => deserializer.deserialize_u64(visitor),
      (0..=8, true) => deserializer.deserialize_i8(visitor),
      (9..=16, true) => deserializer.deserialize_i16(visitor),
      (17..=32, true) => deserializer.deserialize_i32(visitor),
      (33..=64, true) => deserializer.deserialize_i64(visitor),
      _ => deserializer.deserialize_str(visitor),
    }
  })
}

/// `value` in `T`, or the error of the casts for a value outside the range of `T`.
fn cast<S: CastSource, T: CheckedTarget>(value: S) -> Result<T, Rejection> {
  try_cast(value).map_err(Rejection::Range)
}

/// Reads an integer of type `T` at every class: every integer that a format gives, checked against
/// the width of `T` rather than its class, so that `u7` rejects 128, which a `u8` holds, and a
/// decimal string in the syntax of `FromStr`. Every other input gets the fixed message.
struct IntegerVisitor<'a, T> {
  expected: Expected<'a>,
  marker: PhantomData<T>,
}

impl<'de, T> Visitor<'de> for IntegerVisitor<'_, T>
where
  T: CheckedTarget + FromStr<Err = ParseIntError>,
{
  type Value = Result<T, Rejection>;

  fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.expected.describe(f)
  }

  // The narrower integers read through these, which serde forwards them to.
  fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
    Ok(cast(value))
  }

  fn visit_i128<E: de::Error>(self, value: i128) -> Result<Self::Value, E> {
    Ok(cast(value))
  }

  fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
    Ok(cast(value))
  }

  fn visit_u128<E: de::Error>(self, value: u128) -> Result<Self::Value, E> {
    Ok(cast(value))
  }

  fn visit_str<E: de::Error>(self, text: &str) -> Result<Self::Value, E> {
    Ok(text.parse().map_err(Rejection::Parse))
  }

  // Every other kind of input is rejected below. `visit_borrowed_str` reads through `visit_str`,
  // and `visit_string` and `visit_byte_buf`, which `serde_core` has only with `alloc`, read
  // through `visit_str` and `visit_bytes`.
  reject_inputs! {
    visit_bool(bool),
    visit_f32(f32),
    visit_f64(f64),
    visit_char(char),
    visit_bytes(&[u8]),
    visit_borrowed_bytes(&'de [u8]),
  }

  fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
    Err(self.expected.reject())
  }

  fn visit_some<D: Deserializer<'de>>(self, _: D) -> Result<Self::Value, D::Error> {
    Err(self.expected.reject())
  }

  fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
    Err(self.expected.reject())
  }

  fn visit_newtype_struct<D: Deserializer<'de>>(self, _: D) -> Result<Self::Value, D::Error> {
    Err(self.expected.reject())
  }

  fn visit_seq<A: SeqAccess<'de>>(self, _: A) -> Result<Self::Value, A::Error> {
    Err(self.expected.reject())
  }

  fn visit_map<A: MapAccess<'de>>(self, _: A) -> Result<Self::Value, A::Error> {
    Err(self.expected.reject())
  }

  fn visit_enum<A: EnumAccess<'de>>(self, _: A) -> Result<Self::Value, A::Error> {
    Err(self.expected.reject())
  }
}

/// Implements `Serialize` and `Deserialize` for [`Uint`] and [`Int`] over every storage of their
/// storage traits, written `Integer: Repr;`.
macro_rules! integer_serde {
  ($($integer:ident: $repr:ident;)+) => {$(
    #[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
    impl<B: FitsBits<R>, R: $repr> Serialize for $integer<B, R> {
      /// Writes the value through the method of the smallest native integer of its sign that
      /// holds `Bits` bits, up to 128 bits, and as its decimal text above, in every format and
      /// every storage.
      #[inline]
      fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serialize_integer(self, serializer)
      }
    }

    #[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
    impl<'de, B: FitsBits<R>, R: $repr> Deserialize<'de> for $integer<B, R> {
      /// Reads a value from any integer that the format gives, checked against `Bits`, or from a
      /// decimal string in the syntax of `FromStr`: through `deserialize_any` in a human-readable
      /// format at 0 to 64 bits and above 128 bits, through the method of the 128-bit integer of
      /// the sign from 65 to 128 bits, and in another format through the method of the class, or a
      /// string above 128 bits. Every other input is rejected with the fixed message
      /// `expected an integer`.
      #[inline]
      fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserialize_integer(deserializer)
      }
    }
  )+};
}

integer_serde! {
  Uint: UintRepr;
  Int: IntRepr;
}
