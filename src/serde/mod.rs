//! `serde` support: the forms in which the types of this crate are written and read, and helper
//! modules that write and read an integer in another form, for `#[serde(with = "...")]`.
//!
//! The impls are written against `serde_core`, whose traits `serde` re-exports from version
//! 1.0.220 on, so they need a build whose `serde` is 1.0.220 or later. The feature turns on no
//! feature of any other crate, and numo allocates nothing to write or read a value.
//!
//! # The forms of the types
//!
//! A decimal is written as a string, the text that `Display` writes, and read from a string in the
//! syntax of [`parse`](Decimal::parse), in every format, human-readable or binary. Reading rejects
//! a number, so a decimal never passes through a binary floating-point value, and the coefficient
//! is never written alone, as read back into a type with another number of fraction digits it
//! would be off by a power of ten. A [`Scaled`] is written as its text with its scale and read with
//! the scale of the text, in the same way, and a [`RoundingMode`] as its lowercase name, read from
//! a name of any case.
//!
//! An integer, a [`Uint`](crate::Uint) or an [`Int`](crate::Int), is written in a form that depends
//! only on its width, its sign and its value, in every format and every storage: up to 128 bits as
//! a number, through the method of the smallest native integer of its sign that holds the width,
//! and above 128 bits as its decimal text. In a human-readable format, a value of up to 64 or of
//! more than 128 bits reads from any integer that the format gives and from a decimal string, and
//! one of 65 to 128 bits from a 128-bit integer, as `u128` reads; in other formats each class reads
//! through its own method. A [`NonZeroUint`](crate::NonZeroUint) or a
//! [`NonZeroInt`](crate::NonZeroInt) is written and read as its integer, and zero does not read.
//! The table of the classes and the details are in [`serde`](crate::Uint#serde).
//!
//! A text is written through `collect_str`, which numo streams with no buffer and no allocation. A
//! format that keeps serde's default `collect_str`, such as bincode, ciborium, rmp-serde or toml,
//! collects the text in a `String` of its own.
//!
//! # Errors
//!
//! No error that numo builds names the rejected input. An input of the wrong kind gets the fixed
//! message of its path, such as `expected an integer` for the default of an integer, and so does an
//! error that the format builds from numo's description of the expected input. An input of the
//! right kind that numo rejects gets the text of std's error of its path: the text of
//! [`TryFromIntError`](core::num::TryFromIntError) for a number outside the range of an integer,
//! that of [`ParseIntError`](core::num::ParseIntError) for a string that does not parse as one, and
//! the [`ParseError`] of a decimal. An error that a format builds without numo's description of the
//! expected input passes through as the format wrote it and may name the input: ciborium's type
//! errors, toml's line of the value, and serde's buffer of untagged enums and flattened structs
//! when it meets a 128-bit integer. So does every other error of the format, such as the end of the
//! input, an I/O error or a missing field, with its category.
//!
//! # Where the defaults of the integers fall short
//!
//! - A field of 65 to 128 bits fails in an untagged enum, an internally tagged enum or a flattened
//!   struct, as a `u128` field does: serde buffers the content of such a container, and its buffer
//!   holds no 128-bit integer. For the same reason, `serde_json::to_value` of such a value outside
//!   the 64-bit range fails. [`decimal`] writes a string, which reads there.
//! - A quoted value of 65 to 128 bits does not read from JSON, as for `u128`: serde_json reads
//!   digits alone when it is asked for a 128-bit integer. [`decimal`] reads it.
//! - A type above 128 bits reads a JSON number outside the 64-bit range only in a build whose
//!   serde_json has `arbitrary_precision`. Even then, `serde_json::Value` does not hand over a
//!   number outside the 128-bit range that a 64-bit float prints with the same digits, such as 1
//!   followed by 39 zeros. Quote such a value, or use [`decimal`]. With the feature, serde_json
//!   hands such a number over as a map of one entry, whose key is a token private to serde_json;
//!   numo reads a map of that one entry as the integer that its value writes, in every format and
//!   every build, and rejects it if serde_json changes the token.
//! - A JSON number above 2^53 loses its exact value in a reader whose numbers are 64-bit floats,
//!   such as JavaScript. [`decimal`] writes a string.
//! - A format that infers the type of a value from its text, as csv does, gives a value outside the
//!   128-bit range as a float, so the default of a type above 128 bits does not read such a value
//!   back from it: the text of `u256::MAX` does not read back from csv. [`decimal`] asks for a
//!   string and reads every value, and so does [`typed`] above 128 bits, in the form of the
//!   defaults.
//! - In a human-readable format the defaults need `deserialize_any`: a format without it, such as
//!   serde-json-core, reads no value of 0 to 64 bits or of more than 128 bits, and none of 65 to
//!   128 bits, as for `u128`. [`typed`] asks such a format for the type of the value, and so
//!   reads every value there but one of 65 to 128 bits.
//! - serde_test's bare deserializer panics when it is asked whether it is human-readable, so a test
//!   wraps a value of 0 to 64 or of more than 128 bits with `readable()` or `compact()`.
//! - A numo integer takes a decimal string, so in an untagged enum of an integer and a string, the
//!   JSON string `"5"` selects the integer, where a native integer leaves it to the string.
//!
//! # Helpers
//!
//! A helper is a module with two leaves, `required` for a field of an integer and `optional` for an
//! `Option` of it, which a field names in `#[serde(with = "...")]`. Each takes numo's integers and
//! the native integers through a sealed trait, and each but `ruint`, whose form is ruint's, writes
//! the same form in every format:
//!
//! - [`decimal`]: the decimal text, as a string, at every width. Its trait
//!   [`DecimalString`](decimal::DecimalString) takes `Uint`, `Int`, `NonZeroUint` and `NonZeroInt`,
//!   the native integers from `u8` to `u128` and from `i8` to `i128`, and their `NonZero`.
//! - [`quantity`]: Ethereum's JSON-RPC QUANTITY, a string of `0x` and the lowercase hexadecimal
//!   digits without leading zeros, read as go-ethereum's `hexutil` decodes a number. Its trait
//!   [`Quantity`](quantity::Quantity) takes `Uint`, `NonZeroUint`, the native integers from `u8` to
//!   `u128`, and their `NonZero`.
//! - [`fixed_hex`]: `0x` and two lowercase hexadecimal digits for each byte of the width, the
//!   pattern of the `Bits` bits of a negative `Int`. Its trait [`FixedHex`](fixed_hex::FixedHex)
//!   takes `Uint`, `Int`, `NonZeroUint` and `NonZeroInt`, the native integers from `u8` to `u128`
//!   and from `i8` to `i128`, and their `NonZero`.
//! - [`typed`]: the form of the defaults, read alone and by its type in every format: up to 128
//!   bits the native integer of the class of the width, and above a decimal string. Its trait
//!   [`Typed`](typed::Typed) takes `Uint`, `Int`, `NonZeroUint` and `NonZeroInt`.
//! - [`be_bytes`] and [`le_bytes`]: the bytes of the value, most or least significant first, in
//!   two's complement extended by copies of the sign for `Int`, as serde's bytes. Their traits
//!   [`BeBytes`](be_bytes::BeBytes) and [`LeBytes`](le_bytes::LeBytes) take `Uint`, `Int`,
//!   `NonZeroUint` and `NonZeroInt` of up to 8192 bits, the native integers from `u8` to `u128` and
//!   from `i8` to `i128`, and their `NonZero`.
//! - [`ruint`]: the form of ruint's `Uint`, which alloy's `U256` has: a QUANTITY in a
//!   human-readable format and the bytes of the value, most significant first, in another, read as
//!   ruint 1.21 reads it, and more. Its trait [`RuintForm`](ruint::RuintForm) takes `Uint` and
//!   `NonZeroUint` of up to 8192 bits, the native integers from `u8` to `u128`, and their
//!   `NonZero`.
//!
//! `required` on an `Option` field does not compile, with a note that names `optional`, and
//! `optional` on a field that is not an `Option` is a mismatch of types. `optional` writes `None`
//! as the format writes a none, `null` in JSON, and reads `None` from a none and from a unit, which
//! serde's buffer gives for `null`. With `#[serde(default)]`, a missing field reads as `None`;
//! without it, the field is required.
//!
//! A transaction of Ethereum's JSON-RPC, with an `Option` field:
//!
//! ```
//! # use serde_full as serde;
//! use numo::u256;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize)]
//! # #[serde(crate = "serde")]
//! struct Transaction {
//!   #[serde(with = "numo::serde::quantity::required")]
//!   value: u256,
//!   // `default` reads a missing field as `None`; the field is always written, `null` in JSON.
//!   #[serde(default, with = "numo::serde::quantity::optional")]
//!   gas: Option<u256>,
//! }
//!
//! let transaction = Transaction { value: u256::from(255u8), gas: None };
//! let json = serde_json::to_string(&transaction).unwrap();
//! assert_eq!(json, r#"{"value":"0xff","gas":null}"#);
//! let transaction = serde_json::from_str::<Transaction>(r#"{"value":"0x5"}"#).unwrap();
//! assert_eq!((transaction.value, transaction.gas), (u256::from(5u8), None));
//! ```
//!
//! `skip_serializing_if` may be added to such a field only when every format that the type is
//! written in names its fields, as JSON, TOML and YAML do: a positional format, such as postcard,
//! bincode or MessagePack's arrays, reads the next field in the place of a skipped one.
//!
//! # `use numo::*`
//!
//! A crate that imports all of numo with `use numo::*` brings this module into scope, where its
//! name collides with the `serde` crate: `use serde::Serialize` is then ambiguous, and
//! `use ::serde::Serialize` compiles. As features unify, such a crate breaks as soon as any crate
//! of its build turns the `serde` feature of numo on. Import numo's items by name, or through
//! [`aliases`](crate::aliases).

use core::{
  cell::Cell,
  fmt::{self, Write as _},
  marker::PhantomData,
  str::FromStr,
};

use serde_core::{
  Deserialize, Deserializer, Serialize, Serializer,
  de::{self, EnumAccess, MapAccess, SeqAccess, Visitor},
  ser,
};

use crate::{
  Decimal, FixedDecimal, ParseError, RoundingMode, Scaled, int,
  layout::{Layout, Storage},
};

pub(crate) use sealed::{
  BeBytesCore, DecimalStringCore, FixedHexCore, LeBytesCore, QuantityCore, RuintFormCore, TypedCore,
};

pub mod be_bytes;
pub mod decimal;
pub mod fixed_hex;
pub mod le_bytes;
pub mod quantity;
pub mod ruint;
#[cfg(test)]
mod tests;
pub mod typed;

mod sealed {
  use serde_core::{Deserializer, Serializer};

  use crate::layout::Token;

  /// Declares the sealed supertrait of the trait of each helper module, written
  /// `Core: helper::Trait;`. Its methods carry the work of the leaves `required` and `optional` of
  /// the helper and take a [`Token`], which only this crate can make, so that no other crate can
  /// implement the trait of the helper or call the methods.
  macro_rules! cores {
    ($($core:ident: $helper:ident::$trait:ident;)+) => {$(
      #[doc = concat!(
        "Seals [`",
        stringify!($trait),
        "`](super::",
        stringify!($helper),
        "::",
        stringify!($trait),
        ") and carries the work of the leaves of [`",
        stringify!($helper),
        "`](super::",
        stringify!($helper),
        ")."
      )]
      pub trait $core: Sized {
        /// Writes the value in the form of the helper.
        fn write<S: Serializer>(&self, serializer: S, token: Token) -> Result<S::Ok, S::Error>;

        /// Writes `value`: `None`, or `Some` of the value in the form of the helper.
        fn write_option<S: Serializer>(
          value: &Option<Self>,
          serializer: S,
          token: Token,
        ) -> Result<S::Ok, S::Error>;

        /// Reads a value in the form of the helper.
        fn read<'de, D: Deserializer<'de>>(deserializer: D, token: Token) -> Result<Self, D::Error>;

        /// Reads `None`, or `Some` of a value in the form of the helper.
        fn read_option<'de, D: Deserializer<'de>>(
          deserializer: D,
          token: Token,
        ) -> Result<Option<Self>, D::Error>;
      }
    )+};
  }

  cores! {
    DecimalStringCore: decimal::DecimalString;
    QuantityCore: quantity::Quantity;
    FixedHexCore: fixed_hex::FixedHex;
    TypedCore: typed::Typed;
    BeBytesCore: be_bytes::BeBytes;
    LeBytesCore: le_bytes::LeBytes;
    RuintFormCore: ruint::RuintForm;
  }
}

/// Length of a buffer that holds the text of any decimal. The widest texts are those of
/// `Decimal<U0, U154, u512>::MAX`, `0.` and 154 digits, and `Decimal<U0, U153, i512>::MIN`, `-0.`
/// and 153 digits: 156 bytes each.
const TEXT_LEN: usize = int::DIGIT_BUF_LEN + 2;

/// Length of a buffer that holds the text of any [`Scaled`]: a sign, 154 integral digits, the
/// point and 255 fraction digits, the largest scale, which is more than any decimal has. The widest
/// texts take one byte less, because no signed storage holds 154 digits.
const SCALED_TEXT_LEN: usize = 1 + 154 + 1 + u8::MAX as usize;

/// A fixed buffer of `N` bytes that `Display` writes into, failing once it is full.
struct Text<const N: usize> {
  buf: [u8; N],
  len: usize,
}

impl<const N: usize> fmt::Write for Text<N> {
  fn write_str(&mut self, s: &str) -> fmt::Result {
    let end = match self.len.checked_add(s.len()) {
      Some(end) if end <= N => end,
      _ => return Err(fmt::Error),
    };
    self.buf[self.len..end].copy_from_slice(s.as_bytes());
    self.len = end;
    Ok(())
  }
}

/// Serializes the text that `value` writes with [`Display`](fmt::Display) as a string, through a
/// buffer of `N` bytes.
fn serialize_text<const N: usize, T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
  T: fmt::Display,
  S: Serializer,
{
  let mut text = Text::<N> {
    buf: [0; N],
    len: 0,
  };
  // `Display` fails only when the buffer is full, which no value fills; the error is reported
  // rather than unwrapped all the same.
  write!(text, "{value}").map_err(<S::Error as ser::Error>::custom)?;
  let text =
    core::str::from_utf8(&text.buf[..text.len]).map_err(<S::Error as ser::Error>::custom)?;
  serializer.serialize_str(text)
}

impl<I, F, R> Serialize for Decimal<I, F, R>
where
  R: Storage,
  (I, F): Layout<R>,
{
  /// Writes the decimal as a string, the text that [`Display`](fmt::Display) writes, in every
  /// format.
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serialize_text::<TEXT_LEN, _, _>(self, serializer)
  }
}

#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
impl<D> Serialize for Scaled<D>
where
  D: FixedDecimal,
{
  /// Writes the value as a string, the text that [`Display`](fmt::Display) writes with the scale,
  /// in every format.
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serialize_text::<SCALED_TEXT_LEN, _, _>(self, serializer)
  }
}

#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
impl<'de, D> Deserialize<'de> for Scaled<D>
where
  Self: FromStr<Err = ParseError>,
{
  /// Reads a value and the scale of its text from a string in the syntax of `parse`, in every
  /// format. Any other input, such as a number, is rejected with a fixed message that names
  /// nothing of the input; a string that does not parse reports its [`ParseError`].
  fn deserialize<De>(deserializer: De) -> Result<Self, De::Error>
  where
    De: Deserializer<'de>,
  {
    deserialize_text(deserializer)
  }
}

/// What the visitors of a reading path share with [`read`]: whether the format asked for the
/// description of the expected input, and that description.
#[derive(Clone, Copy)]
pub(crate) struct Expected<'a> {
  consulted: &'a Cell<bool>,
  description: &'static str,
}

impl Expected<'_> {
  /// Writes the description of the expected input, for `Visitor::expecting`, and records that the
  /// format asked for it.
  pub(crate) fn describe(self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.consulted.set(true);
    f.write_str(self.description)
  }

  /// The error of a wrong kind of input: the fixed message of the path, `expected` and the
  /// description, which names nothing of the input.
  pub(crate) fn reject<E: de::Error>(self) -> E {
    E::custom(format_args!("expected {}", self.description))
  }
}

/// Reads a value of type `T` through `drive`, which hands `deserializer` the visitor of a reading
/// path, whose fixed message is `expected` followed by `description`.
///
/// The visitors of the path share an [`Expected`], which records whether the format asked for the
/// description of the expected input. A format error built after that is replaced with the fixed
/// message: serde builds that way every error that names a rejected input, with `invalid_type`,
/// `invalid_value` or `invalid_length`. Every other format error, such as the end of the input, an
/// I/O error, invalid UTF-8 or a missing field, passes through unchanged, with its category. A
/// visitor rejects a wrong kind of input with the fixed message itself, and gives the error of a
/// value that it rejects, which names no input either, as `Ok(Err(..))`.
pub(crate) fn read<'de, T, R, D>(
  deserializer: D,
  description: &'static str,
  drive: impl FnOnce(D, Expected<'_>) -> Result<Result<T, R>, D::Error>,
) -> Result<T, D::Error>
where
  R: fmt::Display,
  D: Deserializer<'de>,
{
  let consulted = Cell::new(false);
  let expected = Expected {
    consulted: &consulted,
    description,
  };
  match drive(deserializer, expected) {
    Ok(Ok(value)) => Ok(value),
    // The error of a value leaves the format as a value, so that nothing the format does with the
    // errors of a visitor can make it a format error, which could be replaced.
    Ok(Err(rejected)) => Err(de::Error::custom(rejected)),
    Err(_) if consulted.get() => Err(expected.reject()),
    Err(error) => Err(error),
  }
}

/// What a decimal is read from, whose fixed message is `expected a decimal value as a string`.
const DECIMAL: &str = "a decimal value as a string";

/// Reads a decimal of type `T` from a string with its `FromStr`, which every decimal type
/// implements with `parse`, through [`read`]: every other kind of input gets the fixed message
/// `expected a decimal value as a string`, and a string that does not parse reports its
/// [`ParseError`].
pub(crate) fn deserialize_text<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
  T: FromStr<Err = ParseError>,
  D: Deserializer<'de>,
{
  read(deserializer, DECIMAL, |deserializer, expected| {
    deserializer.deserialize_str(TextVisitor {
      expected,
      marker: PhantomData,
    })
  })
}

/// Reads the text of a decimal of type `T` and parses it, and rejects every other kind of input
/// with the fixed message of its path.
struct TextVisitor<'a, T> {
  expected: Expected<'a>,
  marker: PhantomData<T>,
}

/// Implements the visit methods of inputs that carry a value, written `method(input)`, as
/// rejections with the fixed message of the path, which the field `expected` of the visitor holds.
macro_rules! reject_inputs {
  ($($method:ident($input:ty)),+ $(,)?) => {$(
    fn $method<E: serde_core::de::Error>(self, _: $input) -> Result<Self::Value, E> {
      Err(self.expected.reject())
    }
  )+};
}

pub(crate) use reject_inputs;

impl<'de, T> Visitor<'de> for TextVisitor<'_, T>
where
  T: FromStr<Err = ParseError>,
{
  type Value = Result<T, ParseError>;

  fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.expected.describe(f)
  }

  fn visit_str<E>(self, text: &str) -> Result<Self::Value, E>
  where
    E: de::Error,
  {
    Ok(text.parse())
  }

  // Every other kind of input is rejected below. `visit_borrowed_str` reads through `visit_str`,
  // and `visit_string` and `visit_byte_buf`, which `serde_core` has only with `alloc`, read
  // through `visit_str` and `visit_bytes`.
  reject_inputs! {
    visit_bool(bool),
    visit_i8(i8),
    visit_i16(i16),
    visit_i32(i32),
    visit_i64(i64),
    visit_i128(i128),
    visit_u8(u8),
    visit_u16(u16),
    visit_u32(u32),
    visit_u64(u64),
    visit_u128(u128),
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

/// The names of the rounding modes, in the order of the variants.
const MODE_NAMES: &[&str] = &[
  "up", "down", "ceiling", "floor", "halfup", "halfdown", "halfeven",
];

#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
impl Serialize for RoundingMode {
  /// Writes the lowercase name of the mode, such as `"halfeven"`, as a string in every format. The
  /// name is the one that [`as_str`](RoundingMode::as_str) returns.
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serializer.serialize_str(match self {
      Self::Up => "up",
      Self::Down => "down",
      Self::Ceiling => "ceiling",
      Self::Floor => "floor",
      Self::HalfUp => "halfup",
      Self::HalfDown => "halfdown",
      Self::HalfEven => "halfeven",
    })
  }
}

#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
impl<'de> Deserialize<'de> for RoundingMode {
  /// Reads the name of a mode from a string. Surrounding whitespace is ignored and the case does
  /// not matter, so `"halfeven"`, `"HalfEven"` and `" HALFEVEN "` all read as `HalfEven`. The names
  /// of the half modes may also separate their two words with one space, `-` or `_`, as in
  /// `"half even"`, `"half-even"` and `"half_even"`. Any other input is rejected.
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    deserializer.deserialize_str(ModeVisitor)
  }
}

/// Reads a rounding mode from its name, ignoring surrounding whitespace and case.
struct ModeVisitor;

impl Visitor<'_> for ModeVisitor {
  type Value = RoundingMode;

  fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("the name of a rounding mode as a string")
  }

  fn visit_str<E>(self, name: &str) -> Result<RoundingMode, E>
  where
    E: de::Error,
  {
    let name = name.trim();
    Ok(match () {
      () if name.eq_ignore_ascii_case("up") => RoundingMode::Up,
      () if name.eq_ignore_ascii_case("down") => RoundingMode::Down,
      () if name.eq_ignore_ascii_case("ceiling") => RoundingMode::Ceiling,
      () if name.eq_ignore_ascii_case("floor") => RoundingMode::Floor,
      () if name.eq_ignore_ascii_case("halfup")
        || name.eq_ignore_ascii_case("half up")
        || name.eq_ignore_ascii_case("half-up")
        || name.eq_ignore_ascii_case("half_up") =>
      {
        RoundingMode::HalfUp
      }
      () if name.eq_ignore_ascii_case("halfdown")
        || name.eq_ignore_ascii_case("half down")
        || name.eq_ignore_ascii_case("half-down")
        || name.eq_ignore_ascii_case("half_down") =>
      {
        RoundingMode::HalfDown
      }
      () if name.eq_ignore_ascii_case("halfeven")
        || name.eq_ignore_ascii_case("half even")
        || name.eq_ignore_ascii_case("half-even")
        || name.eq_ignore_ascii_case("half_even") =>
      {
        RoundingMode::HalfEven
      }
      _ => return Err(E::unknown_variant(name, MODE_NAMES)),
    })
  }
}
