//! `Serialize` and `Deserialize` of the integers and the non-zero integers: the form of a value
//! depends only on its width, its sign and its value, never on its storage, and a non-zero integer
//! has the form of its integer, which reads zero as an error.
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
//! against the width with the casts, a decimal string in the syntax of `FromStr`, and serde_json's
//! number token, a map of one entry through which serde_json with `arbitrary_precision` hands over
//! a number outside the 64-bit range. Every other input gets the fixed message
//! `expected an integer`, and a rejected value the text of the error of the casts or of `FromStr`,
//! so that no error that numo builds names the input.
//!
//! The helper modules of `numo::serde` write and read through the same paths, each with a fixed
//! message of its own: `decimal` writes the decimal text and reads with the visitor of the
//! defaults, without serde_json's number token; `quantity` writes the text of `{:#x}` and
//! `fixed_hex` the digits of the words of the casts, and both read their grammars on the bytes of a
//! string, building the value with the casts from words that they decode from the digits, with no
//! buffer; `typed` writes as the defaults do and asks every format for the class of the width,
//! with a visitor that takes the form of the defaults alone, through their value paths; and
//! `be_bytes` and `le_bytes` write the bytes of the words of the casts through the array of the
//! bytes of the width, and read a byte string, or a sequence of bytes into that array, each element
//! read by a seed of their own, building the value with the casts from the words of the bytes. The
//! sealed trait of each helper is implemented here for every type that its paths take, and its
//! public trait for numo's integers and the native integers that it serves.

use core::{
  fmt,
  marker::PhantomData,
  num::{NonZero, ParseIntError, TryFromIntError},
  str::FromStr,
};

use serde_core::{
  Deserialize, Deserializer, Serialize, Serializer,
  de::{self, DeserializeSeed, EnumAccess, IgnoredAny, MapAccess, SeqAccess, Visitor},
};

use super::{
  Int, NonZeroInt, NonZeroUint, Uint,
  cast::{CastSource, CheckedTarget, try_cast},
};
use crate::{
  int::parse_errors,
  layout::Token,
  serde::{
    BeBytesCore, DecimalStringCore, Expected, FixedHexCore, LeBytesCore, QuantityCore, TypedCore,
    be_bytes::BeBytes, decimal::DecimalString, fixed_hex::FixedHex, le_bytes::LeBytes,
    quantity::Quantity, read, reject_inputs, typed::Typed,
  },
  width::{self, FitsBits, IntRepr, LimbArray, NonZeroIntRepr, NonZeroUintRepr, UintRepr},
};

/// What the defaults read, whose fixed message is `expected an integer`.
const INTEGER: &str = "an integer";

/// What `decimal` reads, whose fixed message is `expected an integer as a decimal string`.
const DECIMAL: &str = "an integer as a decimal string";

/// What `quantity` reads, whose fixed message is `expected a hexadecimal quantity as a string`.
const QUANTITY: &str = "a hexadecimal quantity as a string";

/// The error of a QUANTITY without the prefix `0x` or `0X`, the empty string included.
const QUANTITY_PREFIX: &str = "hexadecimal quantity without the 0x prefix";

/// The error of a QUANTITY of the prefix alone.
const QUANTITY_DIGITS: &str = "hexadecimal quantity without digits";

/// The error of a QUANTITY whose digits start with a zero, other than `0x0`.
const QUANTITY_LEADING_ZEROS: &str = "hexadecimal quantity with leading zero digits";

/// What `fixed_hex` reads, whose fixed message is `expected a hexadecimal string`.
const FIXED_HEX: &str = "a hexadecimal string";

/// The error of a hexadecimal string of fixed width without the prefix `0x` or `0X`.
const FIXED_HEX_PREFIX: &str = "hexadecimal string without the 0x prefix";

/// What `be_bytes` and `le_bytes` read, whose fixed message is `expected an array of bytes`.
const BYTE_ARRAY: &str = "an array of bytes";

/// The key of the one entry of the map through which serde_json with its feature
/// `arbitrary_precision` hands over a number that no 64-bit integer holds, whose value is the text
/// of the number. It is private to serde_json: if it changes, such a map is rejected.
const NUMBER_TOKEN: &str = "$serde_json::private::Number";

/// A value of the right kind that a reading path rejects, which prints the text of its error.
enum Rejection {
  /// A number outside the range of the target, with the error of the casts.
  Range(TryFromIntError),
  /// A string that `FromStr` or the grammar of a hexadecimal helper rejects, with its error.
  Parse(ParseIntError),
  /// A string that breaks a rule of the grammar of a hexadecimal helper before its digits are
  /// read, with the message of the rule.
  Text(&'static str),
  /// A hexadecimal string of fixed width with another number of digits than its width has, which
  /// this one is.
  Digits(u64),
  /// A byte string or a sequence of bytes of another length than the width has, which this one
  /// is.
  Bytes(u64),
}

impl fmt::Display for Rejection {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Range(error) => fmt::Display::fmt(error, f),
      Self::Parse(error) => fmt::Display::fmt(error, f),
      Self::Text(message) => f.write_str(message),
      Self::Digits(count) => write!(f, "expected {count} hexadecimal digits"),
      Self::Bytes(1) => f.write_str("expected 1 byte"),
      Self::Bytes(count) => write!(f, "expected {count} bytes"),
    }
  }
}

/// Writes `value` through the method of its class up to 128 bits, and as its decimal text above. A
/// non-zero integer lends its integer to `word` and `Display`, through `with_nonzero`.
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
      token: true,
      marker: PhantomData,
    };
    // A human-readable format gives a number or a string, as it describes its values. A value of 65
    // to 128 bits is asked for as a 128-bit integer in every format, which serde_json reads exactly
    // only when it is asked for one.
    if !matches!(T::BITS, 65..=128) && deserializer.is_human_readable() {
      deserializer.deserialize_any(visitor)
    } else {
      deserialize_class::<T, _, _>(deserializer, visitor)
    }
  })
}

/// Asks `deserializer` for the method of the class of `T`, which the defaults ask a format that is
/// not human-readable for, and `typed` every format: the native integer of the sign of `T` that
/// holds its width up to 128 bits, and a string above.
fn deserialize_class<'de, T, D, V>(deserializer: D, visitor: V) -> Result<V::Value, D::Error>
where
  T: CheckedTarget,
  D: Deserializer<'de>,
  V: Visitor<'de>,
{
  match (T::BITS, T::SIGNED) {
    (0..=8, false) => deserializer.deserialize_u8(visitor),
    (9..=16, false) => deserializer.deserialize_u16(visitor),
    (17..=32, false) => deserializer.deserialize_u32(visitor),
    (33..=64, false) => deserializer.deserialize_u64(visitor),
    (65..=128, false) => deserializer.deserialize_u128(visitor),
    (0..=8, true) => deserializer.deserialize_i8(visitor),
    (9..=16, true) => deserializer.deserialize_i16(visitor),
    (17..=32, true) => deserializer.deserialize_i32(visitor),
    (33..=64, true) => deserializer.deserialize_i64(visitor),
    (65..=128, true) => deserializer.deserialize_i128(visitor),
    _ => deserializer.deserialize_str(visitor),
  }
}

/// `value` in `T`, or the error of the casts for a value outside the range of `T`, and for zero
/// into a non-zero `T`, after the range.
fn cast<S: CastSource, T: CheckedTarget>(value: S) -> Result<T, Rejection> {
  try_cast(value).map_err(Rejection::Range)
}

/// The value in `T` of the text of a number that serde_json's token holds, or `None` for a text
/// that is not an integer of JSON's grammar other than `-0`: an optional `-`, then `0`, or a digit
/// from 1 to 9 followed by any number of digits. Without `arbitrary_precision` serde_json gives
/// `-0` as a float, so the token refuses it too, and both builds agree. `FromStr` parses the text,
/// except that a negative number into an unsigned type is too small, as it is when it arrives as a
/// number, rather than an invalid digit.
fn number_of_token<T>(text: &str) -> Option<Result<T, Rejection>>
where
  T: CheckedTarget + FromStr<Err = ParseIntError>,
{
  let digits = text.strip_prefix('-');
  let negative = digits.is_some();
  match digits.unwrap_or(text).as_bytes() {
    [b'0'] if !negative => {}
    [b'1'..=b'9', rest @ ..] if rest.iter().all(u8::is_ascii_digit) => {}
    _ => return None,
  }
  Some(if negative && !T::SIGNED {
    Err(Rejection::Parse(parse_errors::NEG_OVERFLOW))
  } else {
    text.parse().map_err(Rejection::Parse)
  })
}

/// A part of serde_json's number token, its key or its value: reads a string with `read`, which
/// gives `None` for a text that the part does not hold, and rejects such a text and every other
/// kind of input with the fixed message of its path.
struct TokenPart<'a, F> {
  expected: Expected<'a>,
  read: F,
}

impl<'de, F, V> DeserializeSeed<'de> for TokenPart<'_, F>
where
  F: FnOnce(&str) -> Option<V>,
{
  type Value = V;

  fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<V, D::Error> {
    deserializer.deserialize_str(self)
  }
}

impl<'de, F, V> Visitor<'de> for TokenPart<'_, F>
where
  F: FnOnce(&str) -> Option<V>,
{
  type Value = V;

  fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.expected.describe(f)
  }

  fn visit_str<E: de::Error>(self, text: &str) -> Result<V, E> {
    let Self { expected, read } = self;
    read(text).ok_or_else(|| expected.reject())
  }

  // Every other kind of input is rejected below. The narrower integers and `f32` reach these
  // through serde's forwarding, and the other forms of a string read through `visit_str`.
  reject_inputs! {
    visit_bool(bool),
    visit_i64(i64),
    visit_i128(i128),
    visit_u64(u64),
    visit_u128(u128),
    visit_f64(f64),
    visit_char(char),
    visit_bytes(&[u8]),
    visit_borrowed_bytes(&'de [u8]),
  }

  fn visit_none<E: de::Error>(self) -> Result<V, E> {
    Err(self.expected.reject())
  }

  fn visit_some<D: Deserializer<'de>>(self, _: D) -> Result<V, D::Error> {
    Err(self.expected.reject())
  }

  fn visit_unit<E: de::Error>(self) -> Result<V, E> {
    Err(self.expected.reject())
  }

  fn visit_newtype_struct<D: Deserializer<'de>>(self, _: D) -> Result<V, D::Error> {
    Err(self.expected.reject())
  }

  fn visit_seq<A: SeqAccess<'de>>(self, _: A) -> Result<V, A::Error> {
    Err(self.expected.reject())
  }

  fn visit_map<A: MapAccess<'de>>(self, _: A) -> Result<V, A::Error> {
    Err(self.expected.reject())
  }

  fn visit_enum<A: EnumAccess<'de>>(self, _: A) -> Result<V, A::Error> {
    Err(self.expected.reject())
  }
}

/// Reads an integer of type `T` at every class: every integer that a format gives, checked against
/// the width of `T` rather than its class, so that `u7` rejects 128, which a `u8` holds, a decimal
/// string in the syntax of `FromStr`, and serde_json's number token if `token`. Every other input
/// gets the fixed message.
struct IntegerVisitor<'a, T> {
  expected: Expected<'a>,
  /// Whether the visitor reads serde_json's number token, as the defaults do and `decimal` does
  /// not.
  token: bool,
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

  // serde_json's number token, where the visitor reads it: one entry, whose key is the token and
  // whose value is the text of the number. A map without an entry, with another key or with a
  // second entry is rejected; numo cannot tell the token from a map of that one entry.
  fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
    let expected = self.expected;
    if !self.token {
      return Err(expected.reject());
    }
    let key = || TokenPart {
      expected,
      read: |key: &str| (key == NUMBER_TOKEN).then_some(()),
    };
    if map.next_key_seed(key())?.is_none() {
      return Err(expected.reject());
    }
    let value = map.next_value_seed(TokenPart {
      expected,
      read: number_of_token::<T>,
    })?;
    match map.next_key_seed(key())? {
      None => Ok(value),
      Some(()) => Err(expected.reject()),
    }
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

/// Implements `Serialize` and `Deserialize` for [`NonZeroUint`] and [`NonZeroInt`] over every
/// storage of their storage traits, written `Name: Repr;`, which write and read as the integer of
/// the same value.
macro_rules! nonzero_serde {
  ($($name:ident: $repr:ident;)+) => {$(
    #[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
    impl<B: FitsBits<R::Integer>, R: $repr> Serialize for $name<B, R> {
      /// Writes the value as the integer of the same value writes it: through the method of the
      /// smallest native integer of its sign that holds `Bits` bits, up to 128 bits, and as its
      /// decimal text above, in every format and every storage.
      #[inline]
      fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serialize_integer(self, serializer)
      }
    }

    #[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
    impl<'de, B: FitsBits<R::Integer>, R: $repr> Deserialize<'de> for $name<B, R> {
      /// Reads a value as the integer of the same width reads it, then rejects zero, after every
      /// error of the integer, with std's error of zero for a non-zero integer.
      #[inline]
      fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserialize_integer(deserializer)
      }
    }
  )+};
}

nonzero_serde! {
  NonZeroUint: NonZeroUintRepr;
  NonZeroInt: NonZeroIntRepr;
}

/// Writes the decimal text of `value`, the text of `Display`, through `collect_str`.
fn serialize_decimal<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
  T: fmt::Display,
  S: Serializer,
{
  serializer.collect_str(value)
}

/// Asks the format for a string, which `decimal` reads with [`IntegerVisitor`] without serde_json's
/// number token: a decimal string, and an integer that the format gives instead.
fn drive_decimal<'de, T, D>(
  deserializer: D,
  expected: Expected<'_>,
) -> Result<Result<T, Rejection>, D::Error>
where
  T: CheckedTarget + FromStr<Err = ParseIntError>,
  D: Deserializer<'de>,
{
  deserializer.deserialize_str(IntegerVisitor {
    expected,
    token: false,
    marker: PhantomData,
  })
}

/// Implements the work of a helper module, written
/// `Core, description; write(Bounds) with writer, Some; read(Bounds) with drive, Option;`: the
/// sealed `Core` of the helper for every type with both bounds, which writes a value with `writer`
/// and reads one through [`read`], with `description`, the fixed message of the helper, and
/// `drive`, which hands a format the visitor of the helper and gives back a value or its rejection;
/// `Some`, which lends a value to `writer` as the `Some` of an `Option` field; and `Option`, the
/// visitor of an `Option` field, which shares the consultation flag of the path with the visitor
/// of the helper.
macro_rules! helper {
  (
    $core:ident, $description:ident;
    write($($write:tt)+) with $writer:ident, $some:ident;
    read($($read:tt)+) with $drive:ident, $option:ident;
  ) => {
    /// Lends a value to the writer of its helper, as the `Some` of an `Option` field.
    struct $some<'a, T>(&'a T);

    impl<T: $($write)+> Serialize for $some<'_, T> {
      fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        $writer(self.0, serializer)
      }
    }

    /// Reads an `Option` field of its helper: `None` from a none and from a unit, which serde's
    /// buffer gives for `null`, and the value of a `Some` with the visitor of the helper, under the
    /// same consultation flag. Every other input gets the fixed message of the helper.
    struct $option<'a, T> {
      expected: Expected<'a>,
      marker: PhantomData<T>,
    }

    impl<'de, T: $($read)+> Visitor<'de> for $option<'_, T> {
      type Value = Result<Option<T>, Rejection>;

      fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.expected.describe(f)
      }

      fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(Ok(None))
      }

      fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(Ok(None))
      }

      fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        Ok($drive(deserializer, self.expected)?.map(Some))
      }

      // Every other kind of input is rejected below. The narrower integers and `f32` reach these
      // through serde's forwarding, and the other forms of a string read through `visit_str`.
      reject_inputs! {
        visit_bool(bool),
        visit_i64(i64),
        visit_i128(i128),
        visit_u64(u64),
        visit_u128(u128),
        visit_f64(f64),
        visit_char(char),
        visit_str(&str),
        visit_bytes(&[u8]),
        visit_borrowed_bytes(&'de [u8]),
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

    impl<T> $core for T
    where
      T: $($write)+ + $($read)+,
    {
      #[inline]
      fn write<S: Serializer>(&self, serializer: S, _: Token) -> Result<S::Ok, S::Error> {
        $writer(self, serializer)
      }

      #[inline]
      fn write_option<S: Serializer>(
        value: &Option<Self>,
        serializer: S,
        _: Token,
      ) -> Result<S::Ok, S::Error> {
        match value {
          Some(value) => serializer.serialize_some(&$some(value)),
          None => serializer.serialize_none(),
        }
      }

      #[inline]
      fn read<'de, D: Deserializer<'de>>(deserializer: D, _: Token) -> Result<Self, D::Error> {
        read(deserializer, $description, $drive)
      }

      #[inline]
      fn read_option<'de, D: Deserializer<'de>>(
        deserializer: D,
        _: Token,
      ) -> Result<Option<Self>, D::Error> {
        read(deserializer, $description, |deserializer, expected| {
          deserializer.deserialize_option($option {
            expected,
            marker: PhantomData,
          })
        })
      }
    }
  };
}

helper! {
  DecimalStringCore, DECIMAL;
  write(fmt::Display) with serialize_decimal, DecimalSome;
  read(CheckedTarget + FromStr<Err = ParseIntError>) with drive_decimal, DecimalOption;
}

/// Implements the public trait of a helper module for numo's integers over every storage of their
/// storage traits, written `Trait for Integer: Repr, ..`, for the non-zero integers, written
/// `Trait for nonzero Name: Repr, ..`, and for native integers and their `NonZero`, written
/// `Trait for natives native, ..`; and for the widths of numo's integers and of the non-zero
/// integers that have an array of bytes, up to 8192 bits, written
/// `Trait for bytes Integer: Repr, ..` and `Trait for bytes nonzero Name: Repr, ..`.
macro_rules! helper_impls {
  ($trait:ident for bytes nonzero $($name:ident: $repr:ident),+) => {$(
    // The bound on `u8` names the array of the bytes of the width, as `ByteBuffer` needs it.
    #[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
    impl<B: FitsBits<R::Integer>, R: $repr, const M: usize> $trait for $name<B, R>
    where
      u8: LimbArray<B, Array = [u8; M]>,
    {
    }
  )+};
  ($trait:ident for bytes $($integer:ident: $repr:ident),+) => {$(
    // The bound on `u8` names the array of the bytes of the width, as `ByteBuffer` needs it.
    #[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
    impl<B: FitsBits<R>, R: $repr, const M: usize> $trait for $integer<B, R>
    where
      u8: LimbArray<B, Array = [u8; M]>,
    {
    }
  )+};
  ($trait:ident for nonzero $($name:ident: $repr:ident),+) => {$(
    #[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
    impl<B: FitsBits<R::Integer>, R: $repr> $trait for $name<B, R> {}
  )+};
  ($trait:ident for natives $($native:ident),+) => {$(
    #[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
    impl $trait for $native {}

    #[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
    impl $trait for NonZero<$native> {}
  )+};
  ($trait:ident for $($integer:ident: $repr:ident),+) => {$(
    #[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
    impl<B: FitsBits<R>, R: $repr> $trait for $integer<B, R> {}
  )+};
}

helper_impls!(DecimalString for Uint: UintRepr, Int: IntRepr);
helper_impls!(DecimalString for nonzero NonZeroUint: NonZeroUintRepr, NonZeroInt: NonZeroIntRepr);
helper_impls!(DecimalString for natives u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

/// Writes `value` as a QUANTITY, the text of `{:#x}`: `0x` and the lowercase hexadecimal digits of
/// the value without leading zeros, `0x0` for zero, through `collect_str`.
fn serialize_quantity<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
  T: fmt::LowerHex,
  S: Serializer,
{
  serializer.collect_str(&format_args!("{value:#x}"))
}

/// The value of `byte` as a hexadecimal digit of either case, or `None` for any other byte, among
/// them a sign, `_`, whitespace and every byte of a character outside ASCII.
fn hex_digit(byte: u8) -> Option<u64> {
  match byte {
    b'0'..=b'9' => Some(u64::from(byte - b'0')),
    b'a'..=b'f' => Some(u64::from(byte - b'a') + 10),
    b'A'..=b'F' => Some(u64::from(byte - b'A') + 10),
    _ => None,
  }
}

/// The 64-bit word at `index` of the value whose hexadecimal digits, most significant first, are
/// `digits`, every one of which [`hex_digit`] reads: the sixteen digits of the word, counted from
/// the end of the text, or as many of them as there are, and zero beyond the digits.
fn hex_word(digits: &[u8], index: usize) -> u64 {
  // The digits of the words below `index` end the text.
  let end = digits.len().saturating_sub(index.saturating_mul(16));
  digits[end.saturating_sub(16)..end]
    .iter()
    .fold(0, |word, &digit| word << 4 | hex_digit(digit).unwrap_or(0))
}

/// The value in `T` of the bytes `text` of a QUANTITY, as go-ethereum's `hexutil` decodes a number,
/// or the error of the first of its rules that `text` breaks, in this order: the prefix `0x` or
/// `0X`, which the empty string lacks too; at least one digit; no leading zero, unless the digits
/// are `0`; every byte a hexadecimal digit, of either case; the range of `T`; and, for a non-zero
/// `T`, a value that is not zero.
fn parse_quantity<T: CheckedTarget>(text: &[u8]) -> Result<T, Rejection> {
  let digits = match text {
    [b'0', b'x' | b'X', digits @ ..] => digits,
    _ => return Err(Rejection::Text(QUANTITY_PREFIX)),
  };
  let first = match digits {
    [] => return Err(Rejection::Text(QUANTITY_DIGITS)),
    [b'0', _, ..] => return Err(Rejection::Text(QUANTITY_LEADING_ZEROS)),
    [first, ..] => *first,
  };
  if !digits.iter().all(|&digit| hex_digit(digit).is_some()) {
    return Err(Rejection::Parse(parse_errors::INVALID_DIGIT));
  }
  // The value has the bits of its first digit, none for the digit 0, and four for each other digit.
  // The comparison forms no product of the number of digits and casts none to `u32`, so that no
  // length of the text can overflow it.
  let first_bits = u64::BITS - hex_digit(first).unwrap_or(0).leading_zeros();
  let in_range =
    first_bits <= T::BITS && (digits.len() - 1) as u64 <= u64::from((T::BITS - first_bits) / 4);
  if !in_range {
    return Err(Rejection::Parse(parse_errors::POS_OVERFLOW));
  }
  T::from_words(|index| hex_word(digits, index)).ok_or(Rejection::Parse(parse_errors::ZERO))
}

/// Reads a string with `read`, the grammar of a hexadecimal helper, which takes the bytes of the
/// text and gives back a value or its rejection, and rejects every other kind of input with the
/// fixed message of its path.
struct HexVisitor<'a, F> {
  expected: Expected<'a>,
  read: F,
}

impl<'de, T, F> Visitor<'de> for HexVisitor<'_, F>
where
  F: FnOnce(&[u8]) -> Result<T, Rejection>,
{
  type Value = Result<T, Rejection>;

  fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.expected.describe(f)
  }

  fn visit_str<E: de::Error>(self, text: &str) -> Result<Self::Value, E> {
    Ok((self.read)(text.as_bytes()))
  }

  // Every other kind of input is rejected below. The narrower integers and `f32` reach these
  // through serde's forwarding, and the other forms of a string read through `visit_str`.
  reject_inputs! {
    visit_bool(bool),
    visit_i64(i64),
    visit_i128(i128),
    visit_u64(u64),
    visit_u128(u128),
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

/// Asks the format for a string, which `quantity` reads with [`HexVisitor`] by the grammar of
/// [`parse_quantity`].
fn drive_quantity<'de, T, D>(
  deserializer: D,
  expected: Expected<'_>,
) -> Result<Result<T, Rejection>, D::Error>
where
  T: CheckedTarget,
  D: Deserializer<'de>,
{
  deserializer.deserialize_str(HexVisitor {
    expected,
    read: parse_quantity::<T>,
  })
}

helper! {
  QuantityCore, QUANTITY;
  write(fmt::LowerHex) with serialize_quantity, QuantitySome;
  read(CheckedTarget) with drive_quantity, QuantityOption;
}

helper_impls!(Quantity for Uint: UintRepr);
helper_impls!(Quantity for nonzero NonZeroUint: NonZeroUintRepr);
helper_impls!(Quantity for natives u8, u16, u32, u64, u128);

/// The lowercase hexadecimal digits, by value.
const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// The text of a value that `fixed_hex` writes: `0x` and the lowercase hexadecimal digits of its
/// `Bits` bits, two for each byte of the width, the most significant first, read from the words of
/// the casts, with the bits from `Bits` up as zeros, so that a negative `Int` writes the pattern of
/// its bits, zero-padded, and zero bits write `0x`. It writes the digits itself, through a buffer
/// on the stack, a chunk at a time, as a formatting width cannot pad the text: `core::fmt` panics
/// on a width above `u16::MAX`, which the text of a width above 262128 bits exceeds.
struct FixedHexText<'a, T>(&'a T);

impl<T: CastSource> fmt::Display for FixedHexText<'_, T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let bits = u64::from(T::BITS);
    // The digits still to write, two for each byte of the width, counted in `u64`, which holds the
    // count of every width below `2^32` bits. Digit `k` holds bits `4k` to `4k + 3`, in word
    // `k / 16`.
    let mut digits = 2 * bits.div_ceil(8);
    let mut buf = [0; 64];
    let mut len = 0;
    f.write_str("0x")?;
    while digits > 0 {
      let index = (digits - 1) / 16;
      // The bits of the word from `Bits` up count as zeros.
      let valid = bits.saturating_sub(64 * index);
      let mask = if valid >= 64 {
        u64::MAX
      } else {
        (1 << valid) - 1
      };
      let word = self.0.word(usize::try_from(index).map_err(|_| fmt::Error)?) & mask;
      while digits > 16 * index {
        digits -= 1;
        buf[len] = HEX_DIGITS[((word >> (4 * (digits % 16))) & 0xf) as usize];
        len += 1;
        if len == buf.len() {
          f.write_str(core::str::from_utf8(&buf).map_err(|_| fmt::Error)?)?;
          len = 0;
        }
      }
    }
    f.write_str(core::str::from_utf8(&buf[..len]).map_err(|_| fmt::Error)?)
  }
}

/// Writes `value` as `0x` and two lowercase hexadecimal digits for each byte of its width, the text
/// of [`FixedHexText`], through `collect_str`.
fn serialize_fixed_hex<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
  T: CastSource,
  S: Serializer,
{
  serializer.collect_str(&FixedHexText(value))
}

/// The value in `T` of the bytes `text` of a hexadecimal string of fixed width, or the error of the
/// first of its rules that `text` breaks, in this order: the prefix `0x` or `0X`; exactly two
/// digits for each byte of the width of `T`; every byte a hexadecimal digit, of either case; the
/// bits of the first byte above the width zeros, for both signs; and, for a non-zero `T`, a value
/// that is not zero. The digits are the pattern of the `Bits` bits of the value, which the casts
/// read in two's complement for a signed `T`.
fn parse_fixed_hex<T: CheckedTarget>(text: &[u8]) -> Result<T, Rejection> {
  let digits = match text {
    [b'0', b'x' | b'X', digits @ ..] => digits,
    _ => return Err(Rejection::Text(FIXED_HEX_PREFIX)),
  };
  // The digits of the width, counted in `u64`, which holds them and every length of a text, so
  // that no count overflows and no length is cast to `u32`.
  let count = 2 * u64::from(T::BITS.div_ceil(8));
  if digits.len() as u64 != count {
    return Err(Rejection::Digits(count));
  }
  if !digits.iter().all(|&digit| hex_digit(digit).is_some()) {
    return Err(Rejection::Parse(parse_errors::INVALID_DIGIT));
  }
  // The first byte holds the bits of the width above its last whole byte, one to eight, and zeros
  // above them. Zero bits have no digits, and so no first byte.
  if let [high, low, ..] = digits {
    let first = hex_digit(*high).unwrap_or(0) << 4 | hex_digit(*low).unwrap_or(0);
    if first >> ((T::BITS - 1) % 8 + 1) != 0 {
      return Err(Rejection::Parse(parse_errors::POS_OVERFLOW));
    }
  }
  T::from_words(|index| hex_word(digits, index)).ok_or(Rejection::Parse(parse_errors::ZERO))
}

/// Asks the format for a string, which `fixed_hex` reads with [`HexVisitor`] by the grammar of
/// [`parse_fixed_hex`].
fn drive_fixed_hex<'de, T, D>(
  deserializer: D,
  expected: Expected<'_>,
) -> Result<Result<T, Rejection>, D::Error>
where
  T: CheckedTarget,
  D: Deserializer<'de>,
{
  deserializer.deserialize_str(HexVisitor {
    expected,
    read: parse_fixed_hex::<T>,
  })
}

helper! {
  FixedHexCore, FIXED_HEX;
  write(CastSource) with serialize_fixed_hex, FixedHexSome;
  read(CheckedTarget) with drive_fixed_hex, FixedHexOption;
}

helper_impls!(FixedHex for Uint: UintRepr, Int: IntRepr);
helper_impls!(FixedHex for nonzero NonZeroUint: NonZeroUintRepr, NonZeroInt: NonZeroIntRepr);
helper_impls!(FixedHex for natives u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

/// Reads an integer of type `T` in the form that the defaults write, alone: up to 128 bits every
/// integer that a format gives, checked against the width of `T`, and above 128 bits a decimal
/// string in the syntax of `FromStr`, through the value paths of the defaults. Every other input
/// gets the fixed message: a string up to 128 bits, an integer above, and every map, so that
/// serde_json's number token does not read.
struct TypedVisitor<'a, T> {
  expected: Expected<'a>,
  marker: PhantomData<T>,
}

impl<T: CheckedTarget> TypedVisitor<'_, T> {
  /// `value` in `T`, as the defaults read a number, up to 128 bits, and the fixed message above,
  /// where `T` reads a string alone.
  fn number<S: CastSource, E: de::Error>(self, value: S) -> Result<Result<T, Rejection>, E> {
    if T::BITS <= 128 {
      Ok(cast(value))
    } else {
      Err(self.expected.reject())
    }
  }
}

impl<'de, T> Visitor<'de> for TypedVisitor<'_, T>
where
  T: CheckedTarget + FromStr<Err = ParseIntError>,
{
  type Value = Result<T, Rejection>;

  fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.expected.describe(f)
  }

  // The narrower integers read through these, which serde forwards them to.
  fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
    self.number(value)
  }

  fn visit_i128<E: de::Error>(self, value: i128) -> Result<Self::Value, E> {
    self.number(value)
  }

  fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
    self.number(value)
  }

  fn visit_u128<E: de::Error>(self, value: u128) -> Result<Self::Value, E> {
    self.number(value)
  }

  // A string above 128 bits, as the defaults read it, and the fixed message up to 128 bits, where
  // `T` reads a number alone.
  fn visit_str<E: de::Error>(self, text: &str) -> Result<Self::Value, E> {
    if T::BITS > 128 {
      Ok(text.parse().map_err(Rejection::Parse))
    } else {
      Err(self.expected.reject())
    }
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

/// Asks every format for the class of `T`, as the defaults ask a format that is not
/// human-readable, which `typed` reads with [`TypedVisitor`].
fn drive_typed<'de, T, D>(
  deserializer: D,
  expected: Expected<'_>,
) -> Result<Result<T, Rejection>, D::Error>
where
  T: CheckedTarget + FromStr<Err = ParseIntError>,
  D: Deserializer<'de>,
{
  deserialize_class::<T, _, _>(
    deserializer,
    TypedVisitor {
      expected,
      marker: PhantomData,
    },
  )
}

helper! {
  TypedCore, INTEGER;
  write(CastSource + fmt::Display) with serialize_integer, TypedSome;
  read(CheckedTarget + FromStr<Err = ParseIntError>) with drive_typed, TypedOption;
}

helper_impls!(Typed for Uint: UintRepr, Int: IntRepr);
helper_impls!(Typed for nonzero NonZeroUint: NonZeroUintRepr, NonZeroInt: NonZeroIntRepr);

/// The array of the bytes of a value of a type, `[u8; BYTES]`, through which the bytes helpers
/// write a value and read a sequence of bytes: for numo's integers the array of the width that
/// `u8: LimbArray<B>` names, as for `ByteArray`, which exists up to 8192 bits, and for a native
/// integer and its `NonZero` the array of its size.
trait ByteBuffer {
  /// The array.
  type Bytes: AsMut<[u8]>;

  /// The array, of zeros.
  fn zero_bytes() -> Self::Bytes;
}

/// Implements [`ByteBuffer`] for numo's integers, written `Integer, ..`, and for native integers
/// and their `NonZero`, written `natives native, ..`.
macro_rules! byte_buffers {
  (natives $($native:ident),+) => {$(
    impl ByteBuffer for $native {
      type Bytes = [u8; size_of::<$native>()];

      #[inline]
      fn zero_bytes() -> Self::Bytes {
        [0; size_of::<$native>()]
      }
    }

    impl ByteBuffer for NonZero<$native> {
      type Bytes = [u8; size_of::<$native>()];

      #[inline]
      fn zero_bytes() -> Self::Bytes {
        [0; size_of::<$native>()]
      }
    }
  )+};
  ($($integer:ident),+) => {$(
    // The bound on `u8` fixes `M` at `BYTES`, so that `Bytes` is the array of the width.
    impl<B, R, const M: usize> ByteBuffer for $integer<B, R>
    where
      u8: LimbArray<B, Array = [u8; M]>,
    {
      type Bytes = [u8; M];

      #[inline]
      fn zero_bytes() -> [u8; M] {
        [0; M]
      }
    }
  )+};
}

byte_buffers!(Uint, Int, NonZeroUint, NonZeroInt);
byte_buffers!(natives u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

/// Writes the bytes of `value`, most significant first if `big_endian`, through `serialize_bytes`
/// of its array of bytes: the byte at `i` from the least significant, its bits from `8 * i` to
/// `8 * i + 7`, is that of the word of the casts at `i / 8`, so that a negative `Int` writes its
/// two's complement extended by copies of the sign, as `to_be_bytes` and `to_le_bytes` give it, and
/// zero bits write no bytes.
fn serialize_bytes_of<T, S>(value: &T, big_endian: bool, serializer: S) -> Result<S::Ok, S::Error>
where
  T: CastSource + ByteBuffer,
  S: Serializer,
{
  let mut buffer = T::zero_bytes();
  let bytes = buffer.as_mut();
  let count = bytes.len();
  for (position, byte) in bytes.iter_mut().enumerate() {
    let i = if big_endian {
      count - 1 - position
    } else {
      position
    };
    *byte = (value.word(i / 8) >> (8 * (i % 8))) as u8;
  }
  serializer.serialize_bytes(bytes)
}

/// Writes the bytes of `value`, most significant first, through `serialize_bytes`.
fn serialize_be_bytes<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
  T: CastSource + ByteBuffer,
  S: Serializer,
{
  serialize_bytes_of(value, true, serializer)
}

/// Writes the bytes of `value`, least significant first, through `serialize_bytes`.
fn serialize_le_bytes<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
  T: CastSource + ByteBuffer,
  S: Serializer,
{
  serialize_bytes_of(value, false, serializer)
}

/// The number of bytes of a value of `T`, `ceil(Bits / 8)`, counted in `u64` from the `u32` of its
/// bits, so that the count overflows nothing and no length is cast to `u32`.
fn byte_count<T: CheckedTarget>() -> u64 {
  u64::from(T::BITS.div_ceil(8))
}

/// The 64-bit word at `index` of the value whose bytes are `bytes`, most significant first if
/// `big_endian`: its bytes from `8 * index` to `8 * index + 7`, counted from the least significant,
/// and zero beyond the bytes.
fn word_of_bytes(bytes: &[u8], big_endian: bool, index: usize) -> u64 {
  // The byte at `at` from the least significant, if there is one.
  let byte = |at: usize| {
    let at = if big_endian {
      bytes.len().checked_sub(at)?.checked_sub(1)?
    } else {
      at
    };
    bytes.get(at).copied()
  };
  let start = index.saturating_mul(8);
  (0..8).rev().fold(0, |word, k| {
    word << 8 | u64::from(byte(start.saturating_add(k)).unwrap_or(0))
  })
}

/// The rejection of `top`, the most significant byte of the bytes of a value of `T`, which has at
/// least one bit, or `None` if its bits above the width of `T` are zeros for an unsigned `T` and
/// copies of the sign for a signed one, as in the bytes of every value of the width: too large, or
/// too small for the bytes of a negative value below the range of a signed `T`.
fn top_byte_rejection<T: CheckedTarget>(top: u8) -> Option<Rejection> {
  // The bits of the width in the most significant byte, one to eight.
  let bits = (T::BITS - 1) % 8 + 1;
  let in_range = if T::SIGNED {
    // The byte with its bits from `bits - 1` up replaced by copies of that bit.
    let shift = 8 - bits;
    (top as i8) << shift >> shift == top as i8
  } else {
    u32::from(top) >> bits == 0
  };
  if in_range {
    None
  } else {
    let negative = T::SIGNED && (top as i8) < 0;
    Some(Rejection::Parse(parse_errors::overflow(negative)))
  }
}

/// The value in `T` of `bytes`, most significant first if `big_endian`, or the error of the first
/// of its rules that they break, in this order: exactly as many bytes as the width of `T` has; the
/// bits of the most significant byte above the width zeros for an unsigned `T` and copies of the
/// sign for a signed one, a rule that zero bits, which have no bytes, do not have; and, for a
/// non-zero `T`, a value that is not zero. The casts read the bytes as words, in two's complement
/// for a signed `T`.
fn value_of_bytes<T: CheckedTarget>(bytes: &[u8], big_endian: bool) -> Result<T, Rejection> {
  let count = byte_count::<T>();
  if bytes.len() as u64 != count {
    return Err(Rejection::Bytes(count));
  }
  let top = if big_endian {
    bytes.first()
  } else {
    bytes.last()
  };
  if let Some(rejection) = top.and_then(|&top| top_byte_rejection::<T>(top)) {
    return Err(rejection);
  }
  T::from_words(|index| word_of_bytes(bytes, big_endian, index))
    .ok_or(Rejection::Parse(parse_errors::ZERO))
}

/// An element of a sequence of bytes, read with `deserialize_u8`: every integer that the format
/// gives from 0 to 255, as toml gives `i64` and serde_json `u64`. Every other input, an integer
/// outside that range included, gets the fixed message of its path at once, which names nothing of
/// the input, where the error of serde's own `u8` would.
struct ByteSeed<'a> {
  expected: Expected<'a>,
}

impl ByteSeed<'_> {
  /// `value` as a byte, or the fixed message of the path for a value outside 0 to 255.
  fn byte<V, E: de::Error>(self, value: V) -> Result<u8, E>
  where
    u8: TryFrom<V>,
  {
    u8::try_from(value).map_err(|_| self.expected.reject())
  }
}

impl<'de> DeserializeSeed<'de> for ByteSeed<'_> {
  type Value = u8;

  fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<u8, D::Error> {
    deserializer.deserialize_u8(self)
  }
}

impl<'de> Visitor<'de> for ByteSeed<'_> {
  type Value = u8;

  fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.expected.describe(f)
  }

  // The narrower integers read through these, which serde forwards them to.
  fn visit_i64<E: de::Error>(self, value: i64) -> Result<u8, E> {
    self.byte(value)
  }

  fn visit_i128<E: de::Error>(self, value: i128) -> Result<u8, E> {
    self.byte(value)
  }

  fn visit_u64<E: de::Error>(self, value: u64) -> Result<u8, E> {
    self.byte(value)
  }

  fn visit_u128<E: de::Error>(self, value: u128) -> Result<u8, E> {
    self.byte(value)
  }

  // Every other kind of input is rejected below. `f32` reaches these through serde's forwarding,
  // and the other forms of a string read through `visit_str`.
  reject_inputs! {
    visit_bool(bool),
    visit_f64(f64),
    visit_char(char),
    visit_str(&str),
    visit_bytes(&[u8]),
    visit_borrowed_bytes(&'de [u8]),
  }

  fn visit_none<E: de::Error>(self) -> Result<u8, E> {
    Err(self.expected.reject())
  }

  fn visit_some<D: Deserializer<'de>>(self, _: D) -> Result<u8, D::Error> {
    Err(self.expected.reject())
  }

  fn visit_unit<E: de::Error>(self) -> Result<u8, E> {
    Err(self.expected.reject())
  }

  fn visit_newtype_struct<D: Deserializer<'de>>(self, _: D) -> Result<u8, D::Error> {
    Err(self.expected.reject())
  }

  fn visit_seq<A: SeqAccess<'de>>(self, _: A) -> Result<u8, A::Error> {
    Err(self.expected.reject())
  }

  fn visit_map<A: MapAccess<'de>>(self, _: A) -> Result<u8, A::Error> {
    Err(self.expected.reject())
  }

  fn visit_enum<A: EnumAccess<'de>>(self, _: A) -> Result<u8, A::Error> {
    Err(self.expected.reject())
  }
}

/// Reads the bytes of a value of `T` from `seq`, most significant first if `big_endian`, into the
/// array of the bytes of `T`: exactly as many elements as `T` has bytes, each read by
/// [`ByteSeed`]. A sequence of another length is rejected with its length, as a value, once it has
/// ended: the surplus of a long one is read to its end with `IgnoredAny`, so that the length is
/// reported whatever the surplus holds, and the format finds no input left over in its place.
fn read_byte_sequence<'de, T, A>(
  mut seq: A,
  expected: Expected<'_>,
  big_endian: bool,
) -> Result<Result<T, Rejection>, A::Error>
where
  T: CheckedTarget + ByteBuffer,
  A: SeqAccess<'de>,
{
  let mut buffer = T::zero_bytes();
  let bytes = buffer.as_mut();
  for byte in bytes.iter_mut() {
    match seq.next_element_seed(ByteSeed { expected })? {
      Some(element) => *byte = element,
      None => return Ok(Err(Rejection::Bytes(byte_count::<T>()))),
    }
  }
  let mut surplus = false;
  while seq.next_element::<IgnoredAny>()?.is_some() {
    surplus = true;
  }
  if surplus {
    return Ok(Err(Rejection::Bytes(byte_count::<T>())));
  }
  Ok(value_of_bytes(bytes, big_endian))
}

/// Reads the bytes of a value of `T`, most significant first if `big_endian`: a byte string of
/// exactly as many bytes as the width of `T` has, borrowed or owned, or a sequence of as many
/// integers from 0 to 255, through [`read_byte_sequence`]. Every other input gets the fixed message
/// of its path.
struct BytesVisitor<'a, T> {
  expected: Expected<'a>,
  big_endian: bool,
  marker: PhantomData<T>,
}

impl<'de, T> Visitor<'de> for BytesVisitor<'_, T>
where
  T: CheckedTarget + ByteBuffer,
{
  type Value = Result<T, Rejection>;

  fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.expected.describe(f)
  }

  // `visit_borrowed_bytes` reads through this, and so does `visit_byte_buf`, which `serde_core`
  // has only with `alloc`.
  fn visit_bytes<E: de::Error>(self, bytes: &[u8]) -> Result<Self::Value, E> {
    Ok(value_of_bytes(bytes, self.big_endian))
  }

  fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
    read_byte_sequence(seq, self.expected, self.big_endian)
  }

  // Every other kind of input is rejected below. The narrower integers and `f32` reach these
  // through serde's forwarding, and the other forms of a string read through `visit_str`.
  reject_inputs! {
    visit_bool(bool),
    visit_i64(i64),
    visit_i128(i128),
    visit_u64(u64),
    visit_u128(u128),
    visit_f64(f64),
    visit_char(char),
    visit_str(&str),
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

  fn visit_map<A: MapAccess<'de>>(self, _: A) -> Result<Self::Value, A::Error> {
    Err(self.expected.reject())
  }

  fn visit_enum<A: EnumAccess<'de>>(self, _: A) -> Result<Self::Value, A::Error> {
    Err(self.expected.reject())
  }
}

/// Asks the format for bytes, which `be_bytes` reads with [`BytesVisitor`], most significant first.
fn drive_be_bytes<'de, T, D>(
  deserializer: D,
  expected: Expected<'_>,
) -> Result<Result<T, Rejection>, D::Error>
where
  T: CheckedTarget + ByteBuffer,
  D: Deserializer<'de>,
{
  deserializer.deserialize_bytes(BytesVisitor {
    expected,
    big_endian: true,
    marker: PhantomData,
  })
}

/// Asks the format for bytes, which `le_bytes` reads with [`BytesVisitor`], least significant
/// first.
fn drive_le_bytes<'de, T, D>(
  deserializer: D,
  expected: Expected<'_>,
) -> Result<Result<T, Rejection>, D::Error>
where
  T: CheckedTarget + ByteBuffer,
  D: Deserializer<'de>,
{
  deserializer.deserialize_bytes(BytesVisitor {
    expected,
    big_endian: false,
    marker: PhantomData,
  })
}

helper! {
  BeBytesCore, BYTE_ARRAY;
  write(CastSource + ByteBuffer) with serialize_be_bytes, BeBytesSome;
  read(CheckedTarget + ByteBuffer) with drive_be_bytes, BeBytesOption;
}

helper! {
  LeBytesCore, BYTE_ARRAY;
  write(CastSource + ByteBuffer) with serialize_le_bytes, LeBytesSome;
  read(CheckedTarget + ByteBuffer) with drive_le_bytes, LeBytesOption;
}

helper_impls!(BeBytes for bytes Uint: UintRepr, Int: IntRepr);
helper_impls!(BeBytes for bytes nonzero NonZeroUint: NonZeroUintRepr, NonZeroInt: NonZeroIntRepr);
helper_impls!(BeBytes for natives u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);
helper_impls!(LeBytes for bytes Uint: UintRepr, Int: IntRepr);
helper_impls!(LeBytes for bytes nonzero NonZeroUint: NonZeroUintRepr, NonZeroInt: NonZeroIntRepr);
helper_impls!(LeBytes for natives u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);
