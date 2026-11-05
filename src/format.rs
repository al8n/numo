//! The formatting that the decimals and the integers share: the padding that the integer formats
//! of the standard library apply, written by hand so that a text can be written in parts, without a
//! buffer of its size.

use core::fmt::{self, Alignment, Formatter, Write};

/// Zeros written in chunks, so padding and precision need no buffer of their size.
const ZEROS: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Writes `sign`, the ASCII `prefix` and a body of `len` bytes, which `body` writes, padded to the
/// width of `f` as `Formatter::pad_integral` pads an integer: with the `0` flag, zeros between the
/// prefix and the body, written in chunks; otherwise the fill of `f` on the side that its alignment
/// gives, right by default, and for the center half of it before the text and the rest after.
/// The caller decides the sign and the prefix, such as `+` for the `+` flag and `0x` for `#`.
pub(crate) fn pad(
  f: &mut Formatter<'_>,
  sign: Option<char>,
  prefix: &str,
  len: usize,
  body: impl FnOnce(&mut Formatter<'_>) -> fmt::Result,
) -> fmt::Result {
  let text_len = usize::from(sign.is_some())
    .saturating_add(prefix.len())
    .saturating_add(len);
  let padding = match f.width() {
    Some(width) => width.saturating_sub(text_len),
    None => 0,
  };
  if padding == 0 {
    write_prefix(f, sign, prefix)?;
    return body(f);
  }

  if f.sign_aware_zero_pad() {
    write_prefix(f, sign, prefix)?;
    write_zeros(f, padding)?;
    return body(f);
  }

  let (before, after) = match f.align() {
    Some(Alignment::Left) => (0, padding),
    Some(Alignment::Center) => (padding / 2, padding - padding / 2),
    Some(Alignment::Right) | None => (padding, 0),
  };
  let fill = f.fill();
  write_fill(f, fill, before)?;
  write_prefix(f, sign, prefix)?;
  body(f)?;
  write_fill(f, fill, after)
}

fn write_prefix(f: &mut Formatter<'_>, sign: Option<char>, prefix: &str) -> fmt::Result {
  if let Some(sign) = sign {
    f.write_char(sign)?;
  }
  f.write_str(prefix)
}

/// Writes `count` zeros, in chunks.
pub(crate) fn write_zeros(f: &mut Formatter<'_>, mut count: usize) -> fmt::Result {
  while count > 0 {
    let chunk = count.min(ZEROS.len());
    f.write_str(&ZEROS[..chunk])?;
    count -= chunk;
  }
  Ok(())
}

fn write_fill(f: &mut Formatter<'_>, fill: char, count: usize) -> fmt::Result {
  for _ in 0..count {
    f.write_char(fill)?;
  }
  Ok(())
}

/// Views bytes that are always ASCII, such as the digits produced by the integer core, as a string.
pub(crate) const fn ascii(digits: &[u8]) -> Result<&str, fmt::Error> {
  match core::str::from_utf8(digits) {
    Ok(text) => Ok(text),
    Err(_) => Err(fmt::Error),
  }
}
