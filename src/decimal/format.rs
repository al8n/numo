use core::fmt::{self, Alignment, Formatter, Write};

/// Zeros written in chunks, so padding and precision need no buffer of their size.
const ZEROS: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Bytes of the longest exponent text, `e-154`: the marker, a sign and three digits. A coefficient
/// has between 1 and 154 digits and a decimal at most 154 fraction digits, so the exponent of any
/// decimal is between -154 and 153.
const EXPONENT_LEN: usize = 5;

/// The pieces of the text of a decimal, in output order.
struct Parts<'a> {
  sign: Option<char>,
  integral: &'a str,
  /// Zeros between the decimal point and `fraction` that the magnitude digits do not contain, as
  /// in `0.05` for the digits `5`.
  leading_zeros: usize,
  /// Fraction digits from the magnitude, ending at the last nonzero digit.
  fraction: &'a str,
  /// Zeros after `fraction` requested by the precision, or by the scale of a
  /// [`Scaled`](crate::Scaled).
  trailing_zeros: usize,
  /// The marker and value of the exponent of scientific notation, as in `e-4`, after the digits.
  exponent: Option<&'a str>,
}

impl Parts<'_> {
  fn fraction_len(&self) -> usize {
    self
      .leading_zeros
      .saturating_add(self.fraction.len())
      .saturating_add(self.trailing_zeros)
  }

  fn len(&self) -> usize {
    let point = match self.fraction_len() {
      0 => 0,
      len => len.saturating_add(1),
    };
    usize::from(self.sign.is_some())
      .saturating_add(self.integral.len())
      .saturating_add(point)
      .saturating_add(self.exponent.map_or(0, str::len))
  }

  fn write_sign(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self.sign {
      Some(sign) => f.write_char(sign),
      None => Ok(()),
    }
  }

  fn write_digits(&self, f: &mut Formatter<'_>) -> fmt::Result {
    f.write_str(self.integral)?;
    if self.fraction_len() > 0 {
      f.write_char('.')?;
      write_zeros(f, self.leading_zeros)?;
      f.write_str(self.fraction)?;
      write_zeros(f, self.trailing_zeros)?;
    }
    if let Some(exponent) = self.exponent {
      f.write_str(exponent)?;
    }
    Ok(())
  }
}

/// Writes a decimal whose magnitude has the ASCII digits `digits`, without leading zeros (zero is
/// a single `0`), of which the last `fraction` are fraction digits.
///
/// The shortest exact form is padded like an integer: `+` adds a sign to non-negative values, `0`
/// pads with zeros after the sign, and otherwise the fill character pads to the width with the
/// requested alignment, right by default. `min_fraction` is the minimum number of fraction digits:
/// a shorter fraction is padded with zeros, which are written in chunks. The precision of `f` is
/// not read; the caller passes it in `min_fraction`.
pub(super) fn write(
  f: &mut Formatter<'_>,
  negative: bool,
  digits: &[u8],
  fraction: usize,
  min_fraction: usize,
) -> fmt::Result {
  let (integral, fraction_digits, leading_zeros) = match digits.len().checked_sub(fraction) {
    Some(integral_len) if integral_len > 0 => {
      let (integral, fraction_digits) = digits.split_at(integral_len);
      (integral, fraction_digits, 0)
    }
    _ => (&b"0"[..], digits, fraction - digits.len()),
  };

  let significant_len = fraction_digits
    .iter()
    .rposition(|&digit| digit != b'0')
    .map_or(0, |last| last + 1);
  let (leading_zeros, fraction_digits) = match significant_len {
    0 => (0, &fraction_digits[..0]),
    len => (leading_zeros, &fraction_digits[..len]),
  };
  let shown = leading_zeros + fraction_digits.len();
  let trailing_zeros = min_fraction.saturating_sub(shown);

  let sign = if negative {
    Some('-')
  } else if f.sign_plus() {
    Some('+')
  } else {
    None
  };

  let parts = Parts {
    sign,
    integral: ascii(integral)?,
    leading_zeros,
    fraction: ascii(fraction_digits)?,
    trailing_zeros,
    exponent: None,
  };
  write_parts(f, &parts)
}

/// Writes `parts` with the width, fill, alignment and `0` flag of `f`.
fn write_parts(f: &mut Formatter<'_>, parts: &Parts<'_>) -> fmt::Result {
  let padding = match f.width() {
    Some(width) => width.saturating_sub(parts.len()),
    None => 0,
  };
  if padding == 0 {
    parts.write_sign(f)?;
    return parts.write_digits(f);
  }

  if f.sign_aware_zero_pad() {
    parts.write_sign(f)?;
    write_zeros(f, padding)?;
    return parts.write_digits(f);
  }

  let (before, after) = match f.align() {
    Some(Alignment::Left) => (0, padding),
    Some(Alignment::Center) => (padding / 2, padding - padding / 2),
    Some(Alignment::Right) | None => (padding, 0),
  };
  let fill = f.fill();
  write_fill(f, fill, before)?;
  parts.write_sign(f)?;
  parts.write_digits(f)?;
  write_fill(f, fill, after)
}

/// Writes a decimal whose magnitude has the ASCII digits `digits`, as for `write`, in scientific
/// notation, with `marker` (`e` or `E`) between the mantissa and the exponent.
///
/// The mantissa has the significant digits, the first before the point. The exponent is the place
/// of the first digit, `digits.len() - 1 - fraction`, and is 0 for zero, whatever `fraction` is.
/// Sign and padding are as for `write`, over the whole text, exponent included. The precision is
/// the minimum number of mantissa fraction digits, so it never rounds.
pub(super) fn write_exp(
  f: &mut Formatter<'_>,
  negative: bool,
  digits: &[u8],
  fraction: usize,
  marker: u8,
) -> fmt::Result {
  let (first, rest) = digits.split_at_checked(1).ok_or(fmt::Error)?;
  let significant_len = rest
    .iter()
    .rposition(|&digit| digit != b'0')
    .map_or(0, |last| last + 1);
  let rest = &rest[..significant_len];
  // The digit writer gives zero a single `0`, which the formula would put at `-fraction`.
  let exponent = if digits == b"0" {
    0
  } else {
    digits.len() as i32 - 1 - fraction as i32
  };
  let trailing_zeros = f
    .precision()
    .map_or(0, |precision| precision.saturating_sub(rest.len()));

  let sign = if negative {
    Some('-')
  } else if f.sign_plus() {
    Some('+')
  } else {
    None
  };

  // The exponent is formatted once, here, so that the width counts the bytes that are written.
  let mut exponent_buf = [0; EXPONENT_LEN];
  let parts = Parts {
    sign,
    integral: ascii(first)?,
    leading_zeros: 0,
    fraction: ascii(rest)?,
    trailing_zeros,
    exponent: Some(exponent_text(&mut exponent_buf, marker, exponent)?),
  };
  write_parts(f, &parts)
}

/// Writes `marker` and `exponent`, as in `e-4`, to the end of `buf` and returns them as a string.
/// Requires `-154 <= exponent <= 153`, so that they fit.
fn exponent_text(
  buf: &mut [u8; EXPONENT_LEN],
  marker: u8,
  exponent: i32,
) -> Result<&str, fmt::Error> {
  let mut start = EXPONENT_LEN;
  let mut magnitude = exponent.unsigned_abs();
  loop {
    start -= 1;
    buf[start] = b'0' + (magnitude % 10) as u8;
    magnitude /= 10;
    if magnitude == 0 {
      break;
    }
  }
  if exponent < 0 {
    start -= 1;
    buf[start] = b'-';
  }
  start -= 1;
  buf[start] = marker;
  ascii(&buf[start..])
}

/// Views bytes that are always ASCII, such as the digits produced by the integer core, as a
/// string.
fn ascii(digits: &[u8]) -> Result<&str, fmt::Error> {
  core::str::from_utf8(digits).map_err(|_| fmt::Error)
}

fn write_zeros(f: &mut Formatter<'_>, mut count: usize) -> fmt::Result {
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
