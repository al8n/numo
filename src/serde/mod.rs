//! `serde` support: a decimal as its text and a rounding mode as its name, as strings in every
//! format.
//!
//! The traits come from `serde_core`, which `serde` 1.0.220 and later re-export. There is no
//! branch on `is_human_readable`: a binary format writes the same text, because a coefficient read
//! back with another number of fraction digits would be off by a power of ten.
//!
//! No error of reading a decimal names the rejected input: [`deserialize_text`] replaces a format
//! error that describes it with a fixed message and passes every other one through with its
//! category. This covers decimal values only: reading a rounding mode reports the rejected name
//! and the accepted ones.

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
  Decimal, ParseError, RoundingMode, int,
  layout::{Layout, Storage},
};

#[cfg(test)]
mod tests;

/// Length of a buffer that holds the text of any decimal. The widest texts are those of
/// `Decimal<U0, U154, u512>::MAX`, `0.` and 154 digits, and `Decimal<U0, U153, i512>::MIN`, `-0.`
/// and 153 digits: 156 bytes each.
const TEXT_LEN: usize = int::DIGIT_BUF_LEN + 2;

/// A fixed buffer that `Display` writes into, failing once it is full.
struct Text {
  buf: [u8; TEXT_LEN],
  len: usize,
}

impl fmt::Write for Text {
  fn write_str(&mut self, s: &str) -> fmt::Result {
    let end = match self.len.checked_add(s.len()) {
      Some(end) if end <= TEXT_LEN => end,
      _ => return Err(fmt::Error),
    };
    self.buf[self.len..end].copy_from_slice(s.as_bytes());
    self.len = end;
    Ok(())
  }
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
    let mut text = Text {
      buf: [0; TEXT_LEN],
      len: 0,
    };
    // `Display` fails only when the buffer is full, which no decimal fills; the error is reported
    // rather than unwrapped all the same.
    write!(text, "{self}").map_err(<S::Error as ser::Error>::custom)?;
    let text =
      core::str::from_utf8(&text.buf[..text.len]).map_err(<S::Error as ser::Error>::custom)?;
    serializer.serialize_str(text)
  }
}

/// The message of every input that is not a string, which names nothing of the input.
const EXPECTED: &str = "expected a decimal value as a string";

/// Reads a decimal of type `T` from a string with its `FromStr`, which every decimal type
/// implements with `parse`, and rejects every other kind of input with [`EXPECTED`].
///
/// A format error whose construction consulted the visitor's description of the expected input is
/// replaced with [`EXPECTED`] too: serde builds that way every error that names a rejected input,
/// with `invalid_type`, `invalid_value` or `invalid_length`. Every other format error, such as the
/// end of the input, an I/O error, invalid UTF-8 or a missing field, passes through unchanged, with
/// its category. A string that does not parse reports its [`ParseError`], which names no input
/// either.
pub(crate) fn deserialize_text<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
  T: FromStr<Err = ParseError>,
  D: Deserializer<'de>,
{
  let consulted = Cell::new(false);
  let visitor = TextVisitor {
    consulted: &consulted,
    marker: PhantomData,
  };
  match deserializer.deserialize_str(visitor) {
    Ok(Ok(value)) => Ok(value),
    // The parse error leaves the format as a value, so that nothing the format does with the
    // errors of a visitor can make it a format error, which could be replaced.
    Ok(Err(error)) => Err(de::Error::custom(error)),
    Err(_) if consulted.get() => Err(de::Error::custom(EXPECTED)),
    Err(error) => Err(error),
  }
}

/// Reads the text of a decimal of type `T` and parses it, and rejects every other kind of input
/// with [`EXPECTED`]. `consulted` records whether the format asked for the description of the
/// expected input.
struct TextVisitor<'a, T> {
  consulted: &'a Cell<bool>,
  marker: PhantomData<T>,
}

impl<T> TextVisitor<'_, T> {
  fn reject<E: de::Error>() -> Result<Result<T, ParseError>, E> {
    Err(E::custom(EXPECTED))
  }
}

/// Implements the visit methods of inputs that carry a value, other than a string, as rejections.
macro_rules! reject_inputs {
  ($($method:ident($input:ty)),+ $(,)?) => {$(
    fn $method<E: de::Error>(self, _: $input) -> Result<Self::Value, E> {
      Self::reject()
    }
  )+};
}

impl<'de, T> Visitor<'de> for TextVisitor<'_, T>
where
  T: FromStr<Err = ParseError>,
{
  type Value = Result<T, ParseError>;

  fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.consulted.set(true);
    f.write_str("a decimal value as a string")
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
    Self::reject()
  }

  fn visit_some<D: Deserializer<'de>>(self, _: D) -> Result<Self::Value, D::Error> {
    Self::reject()
  }

  fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
    Self::reject()
  }

  fn visit_newtype_struct<D: Deserializer<'de>>(self, _: D) -> Result<Self::Value, D::Error> {
    Self::reject()
  }

  fn visit_seq<A: SeqAccess<'de>>(self, _: A) -> Result<Self::Value, A::Error> {
    Self::reject()
  }

  fn visit_map<A: MapAccess<'de>>(self, _: A) -> Result<Self::Value, A::Error> {
    Self::reject()
  }

  fn visit_enum<A: EnumAccess<'de>>(self, _: A) -> Result<Self::Value, A::Error> {
    Self::reject()
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
