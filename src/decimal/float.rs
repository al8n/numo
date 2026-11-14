//! The conversions of a decimal with the floats, computed in integers alone, as `int::float`
//! computes those of the integers. A decimal is `c / 10^Fraction`, which is
//! `c / 5^Fraction * 2^-Fraction`: `float_bits` divides the magnitude of the coefficient by the
//! power of five in the limbs that it is given, and rounds the quotient once, to the bits of the
//! nearest float. `round_coefficient` reads a float the other way: its value times
//! `10^Fraction` is `significand * 5^Fraction * 2^(exponent + Fraction)`, a product shifted by a
//! power of two, which it rounds once to the magnitude of a coefficient. `shortest` reads the
//! digits of the shortest text of a float, which the formatter of the standard library writes, so
//! that it alone is not `const`.

use core::{
  cmp::Ordering,
  fmt::{self, Write},
};

use crate::{
  int::{
    float::{FloatFormat, FloatParts},
    unsigned::b64,
  },
  rounding::{self, RoundingMode},
};

/// The number of bits of `value`, the index of its highest one plus one, or zero for zero.
const fn bit_length<const N: usize>(value: &[u64; N]) -> u32 {
  match b64::checked_ilog2(value) {
    Some(log) => log + 1,
    None => 0,
  }
}

/// The bits of the float of `format` nearest to `magnitude / 10^fraction`, ties to even, with the
/// sign bit set if `negative`: those of an infinity if the value rounds beyond the largest float,
/// of a subnormal float or of a zero if it rounds below the smallest normal one, and of `+0.0` for
/// a magnitude of zero, whatever `negative`. Computes in the limbs of `magnitude`. Requires
/// `bits(5^fraction) + precision + 2 <= 64 * N`, which the float scratch of a storage gives for
/// every fraction count of a type over it.
pub(super) const fn float_bits<const N: usize>(
  magnitude: &[u64; N],
  fraction: u32,
  negative: bool,
  format: FloatFormat,
) -> u64 {
  let precision = format.precision();
  let exponent_bits = format.exponent_bits();
  // The exponents of the largest and of the smallest normal floats.
  let max_exponent: i32 = (1 << (exponent_bits - 1)) - 1;
  let min_exponent = 1 - max_exponent;
  // Above the exponent and the fraction: bit 63 of an `f64` and bit 31 of an `f32`.
  let sign = (negative as u64) << (precision - 1 + exponent_bits);

  let length = bit_length(magnitude);
  // A decimal has one zero, and zero has no highest bit to place.
  if length == 0 {
    return 0;
  }
  // `5^fraction`, which the requirement keeps inside the limbs. Zero fraction digits give one.
  let (power, overflow) = b64::overflowing_pow(&b64::from_u64(5), fraction);
  debug_assert!(!overflow, "the power of five passes the limbs");
  let power_length = bit_length(&power);
  // The magnitude is shifted so that the quotient has at least `precision + 2` bits: the kept
  // bits, the guard and one more. A magnitude of `wanted` bits or more needs no shift, so the
  // shift saturates at zero.
  let wanted = power_length + precision + 2;
  let shift = wanted.saturating_sub(length);
  // At most `max(length, power_length + precision + 2) <= 64 * N` bits by the requirement, and a
  // shift that passed the limbs would cut the magnitude without a word.
  debug_assert!(
    length + shift <= 64 * N as u32,
    "the shifted magnitude passes the limbs"
  );
  let (quotient, remainder) = b64::div_rem(&b64::shift_left(magnitude, shift), &power);
  let quotient_length = bit_length(&quotient);
  // The shifted magnitude is at least `2^(length + shift - 1)` and the power below
  // `2^power_length`, so the quotient is at least `2^(precision + 1)`.
  debug_assert!(
    quotient_length >= precision + 2,
    "the quotient is shorter than the kept bits and the guard"
  );

  // The value is `(quotient + remainder / power) * 2^-(shift + fraction)`, in
  // `[2^exponent, 2^(exponent + 1))`. Below one the exponent is negative, so it is signed.
  let exponent = quotient_length as i32 - 1 - shift as i32 - fraction as i32;
  if exponent > max_exponent {
    // The biased exponent of every bit set, with a fraction of zero.
    return sign | (((1 << exponent_bits) - 1) << (precision - 1));
  }
  // The bits that the float keeps: all of its precision for a normal float, one fewer for each
  // step of the exponent below the smallest normal one, and none from half of the smallest
  // subnormal float down.
  let kept_bits = if exponent >= min_exponent {
    precision as i32
  } else {
    precision as i32 - (min_exponent - exponent)
  };
  // Below half of the smallest subnormal float.
  if kept_bits < 0 {
    return sign;
  }
  // `quotient_length >= precision + 2 > kept_bits`, so at least two bits are dropped and the
  // guard is a bit of the quotient. With nothing kept, the guard is its highest bit, and a shift
  // by its whole length gives zero, also at the full length of the limbs.
  let dropped = quotient_length - kept_bits as u32;
  let kept = b64::word(&b64::shift_right(&quotient, dropped), 0);
  let guard = dropped - 1;
  let guard_set = (b64::word(&quotient, (guard / 64) as usize) >> (guard % 64)) & 1 == 1;
  // The value is inexact below the guard if a lower bit of the quotient is set, or if the
  // division left a remainder, which no bit of the quotient shows.
  let lower_set = match b64::lowest_one(&quotient) {
    Some(lowest) => lowest < guard,
    None => false,
  };
  let below = lower_set || !b64::is_zero(&remainder);
  // Ties to even: the parity of `kept` is that of the bit pattern.
  let up = guard_set && (below || kept & 1 == 1);
  // A sum, and not `|`: for a normal float the highest kept bit adds the one that the biased
  // exponent `exponent - min_exponent` lacks, and `up` carries from the top of a binade into the
  // next, from the largest subnormal float into the smallest normal one, and from the largest
  // float into the infinity. The largest sum is the pattern of the infinity, below the sign bit.
  let biased = if exponent > min_exponent {
    (exponent - min_exponent) as u64
  } else {
    0
  };
  sign | ((biased << (precision - 1)) + kept + up as u64)
}

/// The magnitude of the value of `parts` times `10^fraction`, rounded to an integer with `mode` as
/// `rounding::increments` rounds a quotient, or `None` if it has more than `64 * N` bits. Requires
/// `bits(5^fraction) + precision + 2 <= 64 * N`.
pub(super) const fn round_coefficient<const N: usize>(
  parts: FloatParts,
  fraction: u32,
  mode: RoundingMode,
) -> Option<[u64; N]> {
  let FloatParts {
    negative,
    significand,
    exponent,
    precision,
  } = parts;
  // The value times `10^fraction` is `significand * 5^fraction * 2^(exponent + fraction)`.
  let (power, overflow) = b64::overflowing_pow(&b64::from_u64(5), fraction);
  debug_assert!(!overflow, "the power of five passes the limbs");
  let product = b64::mul(&b64::from_u64(significand), &power);
  let length = bit_length(&product);
  // The significand is below `2^precision`, so the product has at most `precision` bits more than
  // the power, which the requirement keeps two bits short of the limbs: the product is exact, and
  // the rounding up below does not carry out of the limbs.
  debug_assert!(
    length <= precision + bit_length(&power),
    "the product of the significand and the power of five passes its bound"
  );
  // The exponent reaches -1074 below zero, so the shift is signed.
  let shift = exponent + fraction as i32;
  if shift >= 0 {
    // An integer, and exact. It is checked before the shift, which would cut a longer one without
    // a word, and then find a small magnitude in range.
    let shift = shift as u32;
    if length + shift > 64 * N as u32 {
      return None;
    }
    return Some(b64::shift_left(&product, shift));
  }
  let dropped = shift.unsigned_abs();
  let (quotient, remainder_is_zero, half) = if dropped > length {
    // The product is below `2^length <= 2^(dropped - 1)`, half of the divisor, so the quotient is
    // zero and the remainder, the product itself, is below half. The bit below the quotient would
    // lie above the product, beyond the limbs once more bits are dropped than the limbs have.
    ([0; N], b64::is_zero(&product), Ordering::Less)
  } else {
    // `dropped <= length`, so every bit that is read is inside the limbs.
    let guard = dropped - 1;
    let guard_set = (b64::word(&product, (guard / 64) as usize) >> (guard % 64)) & 1 == 1;
    let lower_set = match b64::lowest_one(&product) {
      Some(lowest) => lowest < guard,
      None => false,
    };
    let half = match (guard_set, lower_set) {
      (false, _) => Ordering::Less,
      (true, false) => Ordering::Equal,
      (true, true) => Ordering::Greater,
    };
    (
      b64::shift_right(&product, dropped),
      !guard_set && !lower_set,
      half,
    )
  };
  let away = rounding::increments(
    mode,
    negative,
    remainder_is_zero,
    half,
    b64::word(&quotient, 0) & 1 == 1,
    // Only `Up05` reads the last decimal digit, which a quotient of zero has: zero ends in 0.
    matches!(mode, RoundingMode::Up05) && b64::div_rem_limb(&quotient, 5).1 == 0,
  );
  // The quotient has at most the bits of the product, two fewer than the limbs, so one more does
  // not carry out of them.
  if away {
    Some(b64::overflowing_add(&quotient, &b64::one()).0)
  } else {
    Some(quotient)
  }
}

/// The text of a float in 32 bytes, more than the formatter writes for any float: a sign, the 17
/// digits of its own buffer, the point, `e`, `-` and the five digits of a `u16` are 26 bytes.
struct Text {
  bytes: [u8; 32],
  len: usize,
}

impl Write for Text {
  /// Appends `text`, or fails if the bytes do not hold it.
  fn write_str(&mut self, text: &str) -> fmt::Result {
    let end = match self.len.checked_add(text.len()) {
      Some(end) if end <= self.bytes.len() => end,
      _ => return Err(fmt::Error),
    };
    self.bytes[self.len..end].copy_from_slice(text.as_bytes());
    self.len = end;
    Ok(())
  }
}

/// The digits of the shortest text of `value`, a finite float, and the exponent of the last of
/// them: the value is `digits * 10^exponent`.
pub(super) fn shortest<T: fmt::LowerExp>(value: T) -> (u64, i32) {
  let mut text = Text {
    bytes: [0; 32],
    len: 0,
  };
  // Without a precision, `{:e}` writes the shortest digits that parse back to the float in its own
  // format, with an exponent, and reads no alternate flag. NaN and the infinities have no such
  // text, which the callers rule out with the parts of the float.
  if write!(text, "{value:e}").is_err() {
    unreachable!("the text `{value:e}` of a float passes 32 bytes");
  }
  match read_exponent_text(&text.bytes[..text.len]) {
    Some(read) => read,
    None => unreachable!("the text `{value:e}` of a float is not in the form of `{{:e}}`"),
  }
}

/// The digits that `text` writes in the form of `{:e}` and the exponent of the last of them, or
/// `None` for a text of any other form. That form is an optional `-`, digits with at most one
/// point, `e`, an optional `-` or `+` and the digits of the exponent, which may begin with zeros,
/// with at most 19 digits before `e`, which a `u64` holds, and an exponent that an `i32` holds.
pub(super) fn read_exponent_text(text: &[u8]) -> Option<(u64, i32)> {
  // The callers take the sign from the parts of the float.
  let text = text.strip_prefix(b"-").unwrap_or(text);
  let split = text.iter().position(|&byte| byte == b'e')?;
  let (mantissa, exponent) = (&text[..split], &text[split + 1..]);
  let mut digits: u64 = 0;
  let mut count: i32 = 0;
  // The number of digits before the point, for a mantissa that has one.
  let mut point = None;
  for &byte in mantissa {
    match byte {
      b'0'..=b'9' if count < 19 => {
        digits = digits * 10 + u64::from(byte - b'0');
        count += 1;
      }
      b'.' if point.is_none() => point = Some(count),
      _ => return None,
    }
  }
  let (negative, exponent) = match exponent {
    [b'-', rest @ ..] => (true, rest),
    [b'+', rest @ ..] => (false, rest),
    _ => (false, exponent),
  };
  if count == 0 || exponent.is_empty() {
    return None;
  }
  let mut written: i32 = 0;
  for &byte in exponent {
    if !byte.is_ascii_digit() {
      return None;
    }
    written = written
      .checked_mul(10)?
      .checked_add(i32::from(byte - b'0'))?;
  }
  let written = if negative { -written } else { written };
  // The last digit is as many places below the written exponent as there are digits after the
  // point.
  let exponent = written.checked_sub(count - point.unwrap_or(count))?;
  Some((digits, exponent))
}
