//! The conversions of a decimal with the floats, computed in integers alone, as `int::float`
//! computes those of the integers. A decimal is `c / 10^Fraction`, which is
//! `c / 5^Fraction * 2^-Fraction`: `float_bits` divides the magnitude of the coefficient by the
//! power of five in eight limbs, which hold every coefficient, and rounds the quotient once, to
//! the bits of the nearest float. `round_coefficient` reads a float the other way: its value times
//! `10^Fraction` is `significand * 5^Fraction * 2^(exponent + Fraction)`, a product shifted by a
//! power of two, which it rounds once to the magnitude of a coefficient.

use core::cmp::Ordering;

use crate::{
  int::{
    float::{FloatFormat, FloatParts},
    unsigned::b64,
  },
  rounding::{self, RoundingMode},
};

/// The number of bits of `value`, the index of its highest one plus one, or zero for zero.
const fn bit_length(value: &[u64; 8]) -> u32 {
  match b64::checked_ilog2(value) {
    Some(log) => log + 1,
    None => 0,
  }
}

/// The bits of the float of `format` nearest to `magnitude / 10^fraction`, ties to even, with the
/// sign bit set if `negative`: those of an infinity if the value rounds beyond the largest float,
/// of a subnormal float or of a zero if it rounds below the smallest normal one, and of `+0.0` for
/// a magnitude of zero, whatever `negative`. Requires `magnitude < 10^154` and `fraction <= 154`.
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

  // `10^154 < 2^512`, so eight limbs hold every magnitude.
  let magnitude: [u64; 8] = b64::resize(magnitude);
  let length = bit_length(&magnitude);
  // A decimal has one zero, and zero has no highest bit to place.
  if length == 0 {
    return 0;
  }
  // `5^fraction` has at most 358 bits: `5^154 < 2^358`. Zero fraction digits give one.
  let (power, _) = b64::overflowing_pow(&b64::from_u64(5), fraction);
  let power_length = bit_length(&power);
  // The magnitude is shifted so that the quotient has at least `precision + 2` bits: the kept
  // bits, the guard and one more. A long magnitude needs no shift, so the lengths are compared
  // before one is subtracted from the other.
  let wanted = power_length + precision + 2;
  let shift = if wanted > length { wanted - length } else { 0 };
  // At most `max(length, 358 + precision + 2) <= 512` bits, and a shift that passed the limbs
  // would cut the magnitude without a word.
  debug_assert!(
    length + shift <= 512,
    "the shifted magnitude passes eight limbs"
  );
  let (quotient, remainder) = b64::div_rem(&b64::shift_left(&magnitude, shift), &power);
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
  // by its whole length gives zero, also at 512 bits.
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
/// `rounding::increments` rounds a quotient, or `None` if it has more than 512 bits.
pub(super) const fn round_coefficient(
  parts: FloatParts,
  fraction: u32,
  mode: RoundingMode,
) -> Option<[u64; 8]> {
  let FloatParts {
    negative,
    significand,
    exponent,
    precision,
  } = parts;
  // The value times `10^fraction` is `significand * 5^fraction * 2^(exponent + fraction)`.
  let (power, _) = b64::overflowing_pow(&b64::from_u64(5), fraction);
  let product = b64::mul(&b64::from_u64(significand), &power);
  let length = bit_length(&product);
  // The significand is below `2^precision` and `5^154` below `2^358`: below `2^411` for an `f64`
  // and `2^382` for an `f32`.
  debug_assert!(
    length <= precision + 358,
    "the product of the significand and the power of five passes its bound"
  );
  // The exponent reaches -1074 below zero, so the shift is signed.
  let shift = exponent + fraction as i32;
  if shift >= 0 {
    // An integer, and exact. It is checked before the shift, which would cut a longer one without
    // a word, and then find a small magnitude in range.
    let shift = shift as u32;
    if length + shift > 512 {
      return None;
    }
    return Some(b64::shift_left(&product, shift));
  }
  let dropped = shift.unsigned_abs();
  let (quotient, remainder_is_zero, half) = if dropped > length {
    // The product is below `2^length <= 2^(dropped - 1)`, half of the divisor, so the quotient is
    // zero and the remainder, the product itself, is below half. The bit below the quotient would
    // lie above the product, beyond the limbs from 513 dropped bits on.
    ([0; 8], b64::is_zero(&product), Ordering::Less)
  } else {
    // `dropped <= length <= 411`, so every bit that is read is inside the limbs.
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
  // The quotient is below `2^411`, so one more does not carry out of the limbs.
  if away {
    Some(b64::overflowing_add(&quotient, &b64::one()).0)
  } else {
    Some(quotient)
  }
}
