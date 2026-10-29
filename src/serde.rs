//! `serde` support: a decimal as its text and a rounding mode as its name, as strings in every
//! format.
//!
//! The traits come from `serde_core`, which `serde` 1.0.220 and later re-export. There is no
//! branch on `is_human_readable`: a binary format writes the same text, because a coefficient read
//! back with another number of fraction digits would be off by a power of ten.

use core::{
  fmt::{self, Write as _},
  marker::PhantomData,
  str::FromStr,
};

use serde_core::{
  Deserialize, Deserializer, Serialize, Serializer,
  de::{self, Visitor},
  ser,
};

use crate::{
  Decimal, ParseError, RoundingMode, int,
  layout::{Layout, Storage},
};

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

/// Reads a decimal of type `T` from a string with its `FromStr`, which every storage implements
/// with `parse`.
pub(crate) struct TextVisitor<T>(PhantomData<T>);

impl<T> TextVisitor<T> {
  pub(crate) const fn new() -> Self {
    Self(PhantomData)
  }
}

impl<T> Visitor<'_> for TextVisitor<T>
where
  T: FromStr<Err = ParseError>,
{
  type Value = T;

  fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("a decimal number as a string")
  }

  fn visit_str<E>(self, text: &str) -> Result<T, E>
  where
    E: de::Error,
  {
    text.parse().map_err(E::custom)
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
