//! The formatting that the decimals and the integers share: the padding that the integer formats
//! of the standard library apply, written by hand so that a text can be written in parts, without a
//! buffer of its size, and the radix and exponent formats of the integers stored in limbs.

use core::fmt::{self, Alignment, Formatter, Write};

/// Zeros written in chunks, so padding and precision need no buffer of their size.
const ZEROS: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// The digits of the radix formats, in lowercase and in uppercase.
const DIGITS: [&[u8; 16]; 2] = [b"0123456789abcdef", b"0123456789ABCDEF"];

/// The bytes of the exponent of the exponent formats: the marker and at most 10 digits, which hold
/// the exponent of an integer of fewer than `2^32` bits, below `1.3 * 10^9`.
const EXPONENT_LEN: usize = 11;

/// Writes the digits of the unsigned integer in `limbs`, least significant limb first, in the
/// radix `2^shift`, binary for 1, octal for 3 and hexadecimal for 4, in uppercase if `upper`, as
/// the radix formats of the native integers write them: the digits from the highest one down, `0`
/// for zero, `+` with the `+` flag, the prefix `0b`, `0o` or `0x` with `#`, and the width as
/// [`pad`] pads it; the precision is ignored. The digits are read from the limbs a few bits at a
/// time and written through a buffer of 64 bytes, whatever their number.
pub(crate) fn write_pow2<L: Copy + Into<u128>>(
  limbs: &[L],
  shift: u32,
  upper: bool,
  f: &mut Formatter<'_>,
) -> fmt::Result {
  let limb_bits = 8 * size_of::<L>() as u64;
  // The number of bits up to the highest one, and of the digits that hold them, one for zero.
  let bit_len = match limbs.iter().rposition(|&limb| limb.into() != 0) {
    Some(top) => top as u64 * limb_bits + u64::from(u128::BITS - limbs[top].into().leading_zeros()),
    None => 0,
  };
  let count = bit_len.div_ceil(u64::from(shift)).max(1);
  let prefix = match (f.alternate(), shift) {
    (false, _) => "",
    (true, 1) => "0b",
    (true, 3) => "0o",
    (true, _) => "0x",
  };
  let sign = f.sign_plus().then_some('+');
  let digits = DIGITS[usize::from(upper)];
  // A count beyond `usize` exceeds every width, so the saturated length pads nothing either.
  let len = usize::try_from(count).unwrap_or(usize::MAX);
  pad(f, sign, prefix, len, |f| {
    let mut buf = [0; 64];
    let mut remaining = count;
    while remaining > 0 {
      let chunk = remaining.min(64) as usize;
      for slot in &mut buf[..chunk] {
        remaining -= 1;
        *slot = digits[digit_at(limbs, remaining * u64::from(shift), shift)];
      }
      f.write_str(ascii(&buf[..chunk])?)?;
    }
    Ok(())
  })
}

/// The `shift` bits of `limbs` from bit `index` up, at most 4, read as one digit: a digit across
/// two limbs, as an octal digit can be, takes its top bits from the next limb, and the bits past
/// the last limb are zeros.
fn digit_at<L: Copy + Into<u128>>(limbs: &[L], index: u64, shift: u32) -> usize {
  let limb_bits = 8 * size_of::<L>() as u64;
  let (limb, offset) = ((index / limb_bits) as usize, (index % limb_bits) as u32);
  let read = |k: usize| limbs.get(k).map_or(0, |&limb| limb.into());
  let mut bits = read(limb) >> offset;
  // The shift is below `limb_bits`, as `offset` is above `limb_bits - shift`, and above zero.
  if u64::from(offset + shift) > limb_bits {
    bits |= read(limb + 1) << (limb_bits as u32 - offset);
  }
  (bits & ((1 << shift) - 1)) as usize
}

/// Writes the integer whose magnitude has the decimal digits `digits`, without leading zeros (zero
/// is a single `0`), negative if `negative`, in scientific notation with `marker`, `e` or `E`, as
/// the exponent formats of the native integers write it, padded as [`pad`] pads it. The mantissa
/// is the first digit and, after a point, the other digits without their trailing zeros, or with a
/// precision `p`, exactly `p` digits: zeros added to fill them, or the digits rounded half to even
/// on the first dropped digit and whether a later one is not zero, which can carry into the
/// exponent, as `999` at a precision of 1 gives `1.0e3`. The exponent is the number of digits
/// after the first. The added zeros are written in chunks. Requires fewer than `10^10` digits.
pub(crate) fn write_integer_exp(
  f: &mut Formatter<'_>,
  negative: bool,
  digits: &[u8],
  marker: u8,
) -> fmt::Result {
  let (&first, rest) = digits.split_first().ok_or(fmt::Error)?;
  let mut exponent = rest.len();
  // The mantissa: `lead`, then after the point the digits `head` as they are, the digit `bumped`
  // that rounding raised, and `zeros` zeros.
  let (mut lead, mut head, mut bumped, mut zeros) = (first, rest, None, 0);
  match f.precision() {
    None => {
      let significant = rest
        .iter()
        .rposition(|&digit| digit != b'0')
        .map_or(0, |last| last + 1);
      head = &rest[..significant];
    }
    Some(precision) if precision >= rest.len() => zeros = precision - rest.len(),
    Some(precision) => {
      let (kept, dropped) = rest.split_at(precision);
      head = kept;
      let last_kept = kept.last().copied().unwrap_or(first);
      let round_up = match dropped[0] {
        b'6'..=b'9' => true,
        b'5' => dropped[1..].iter().any(|&digit| digit != b'0') || last_kept % 2 == 1,
        _ => false,
      };
      if round_up {
        // The carry turns the trailing nines into zeros and raises the digit before them.
        match kept.iter().rposition(|&digit| digit != b'9') {
          Some(k) => {
            head = &kept[..k];
            bumped = Some(kept[k] + 1);
            zeros = precision - k - 1;
          }
          None => {
            head = &[];
            zeros = precision;
            if first == b'9' {
              // Every digit was a nine: the mantissa is `10`, written as `1` and one more power
              // of ten.
              lead = b'1';
              exponent += 1;
            } else {
              lead = first + 1;
            }
          }
        }
      }
    }
  }

  // The precision, or without one at most `rest.len()`, so the sum does not overflow.
  let fraction_len = head.len() + usize::from(bumped.is_some()) + zeros;
  let mut exponent_buf = [0; EXPONENT_LEN];
  let exponent_text = exponent_text(&mut exponent_buf, marker, exponent)?;
  let len = exp_body_len(fraction_len, exponent_text.len());
  let sign = if negative {
    Some('-')
  } else {
    f.sign_plus().then_some('+')
  };
  pad(f, sign, "", len, |f| {
    f.write_char(char::from(lead))?;
    if fraction_len > 0 {
      f.write_char('.')?;
      f.write_str(ascii(head)?)?;
      if let Some(digit) = bumped {
        f.write_char(char::from(digit))?;
      }
      write_zeros(f, zeros)?;
    }
    f.write_str(exponent_text)
  })
}

/// The length of the text of an exponent format after its sign: the lead digit, the point and the
/// `fraction_len` digits after it if there are any, and the `exponent_len` bytes of the marker and
/// the exponent. It saturates at `usize::MAX`, as the standard library saturates the length of a
/// formatted number: the largest precision, `u16::MAX`, is `usize::MAX` on a 16-bit target, where
/// no width exceeds the saturated length, so nothing is padded.
pub(crate) const fn exp_body_len(fraction_len: usize, exponent_len: usize) -> usize {
  let point_and_fraction = if fraction_len > 0 {
    fraction_len.saturating_add(1)
  } else {
    0
  };
  point_and_fraction
    .saturating_add(1)
    .saturating_add(exponent_len)
}

/// Writes `marker` and the decimal digits of `exponent`, which has at most 10 of them, to the end
/// of `buf`, and returns them as a string.
fn exponent_text(
  buf: &mut [u8; EXPONENT_LEN],
  marker: u8,
  mut exponent: usize,
) -> Result<&str, fmt::Error> {
  let mut start = EXPONENT_LEN;
  loop {
    start -= 1;
    buf[start] = b'0' + (exponent % 10) as u8;
    exponent /= 10;
    if exponent == 0 {
      break;
    }
  }
  start -= 1;
  buf[start] = marker;
  ascii(&buf[start..])
}

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
