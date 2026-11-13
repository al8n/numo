//! The floats as integers, which the layers of every storage share: the parts of a float, its value
//! rounded to an integer with a rounding mode, the range of a width that such an integer must fit,
//! and the formats of the floats that the layers write.
//!
//! A finite float is `significand * 2^exponent` with its sign apart, as its parts give it: the
//! significand has at most the precision of the format, 53 bits for `f64` and 24 for `f32`, the
//! implicit bit included, and the exponent places its lowest bit. Rounded to an integer, it is
//! `significand * 2^shift`, again with its sign apart, whose significand is at most `2^53`: the
//! layers check it against the range of a width and place it in their storage. Nothing here
//! computes with floats: the parts come from the bits of a float, and the layers write the bits of
//! the floats that they convert to.

use core::cmp::Ordering;

use super::order;
use crate::{
  FromFloatError,
  rounding::{self, RoundingMode},
};

/// The format of a float that the layers write: its precision, the bits of its significand with
/// the implicit bit, and the bits of its exponent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatFormat {
  /// `f64`, of 53 bits of significand and 11 of exponent.
  F64,
  /// `f32`, of 24 bits of significand and 8 of exponent.
  F32,
}

impl FloatFormat {
  /// The bits of the significand, the implicit bit included.
  #[inline(always)]
  pub(crate) const fn precision(self) -> u32 {
    match self {
      Self::F64 => f64::MANTISSA_DIGITS,
      Self::F32 => f32::MANTISSA_DIGITS,
    }
  }

  /// The bits of the exponent.
  #[inline(always)]
  pub(crate) const fn exponent_bits(self) -> u32 {
    match self {
      Self::F64 => 11,
      Self::F32 => 8,
    }
  }
}

/// The parts of a finite float, zero and the subnormals included: its value is
/// `significand * 2^exponent`, negative if `negative`, and its significand is below
/// `2^precision`, the precision of its format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FloatParts {
  /// Whether the sign bit is set, as it is for `-0.0`.
  pub(crate) negative: bool,
  /// The significand, with the implicit bit of a normal float.
  pub(crate) significand: u64,
  /// The exponent of the lowest bit of the significand.
  pub(crate) exponent: i32,
  /// The precision of the format: 53 for `f64` and 24 for `f32`.
  pub(crate) precision: u32,
}

impl FloatParts {
  /// Returns `true` if the value is an integer: if no bit of the significand is below the point,
  /// as for zero and for every value whose exponent is not negative.
  #[inline(always)]
  pub(crate) const fn is_integer(&self) -> bool {
    self.exponent >= 0
      || self.significand == 0
      || self.significand.trailing_zeros() >= self.exponent.unsigned_abs()
  }
}

/// The parts of the float of `format` whose bits are `bits`, its sign bit on top, or the error of a
/// float that has none: `Nan` for NaN and `Infinite` for an infinity.
const fn parts(bits: u64, format: FloatFormat) -> Result<FloatParts, FromFloatError> {
  let precision = format.precision();
  let exponent_bits = format.exponent_bits();
  // The bits below the exponent, which a normal float extends with the implicit bit.
  let fraction_bits = precision - 1;
  let fraction = bits & ((1 << fraction_bits) - 1);
  let biased = (bits >> fraction_bits) & ((1 << exponent_bits) - 1);
  let negative = (bits >> (fraction_bits + exponent_bits)) & 1 == 1;
  // The largest biased exponent marks NaN and the infinities.
  if biased == (1 << exponent_bits) - 1 {
    return Err(if fraction == 0 {
      FromFloatError::Infinite
    } else {
      FromFloatError::Nan
    });
  }
  // The bias and the bits of the fraction, by which the biased exponent of a normal float exceeds
  // the exponent of the lowest bit of its significand. A subnormal float, and zero, has no implicit
  // bit and the exponent of the smallest normal float.
  let offset = (1 << (exponent_bits - 1)) - 1 + fraction_bits as i32;
  let (significand, biased) = if biased == 0 {
    (fraction, 1)
  } else {
    (fraction | 1 << fraction_bits, biased as i32)
  };
  Ok(FloatParts {
    negative,
    significand,
    exponent: biased - offset,
    precision,
  })
}

/// The parts of `value`, or the error of NaN or of an infinity.
#[inline(always)]
pub(crate) const fn float_parts_f64(value: f64) -> Result<FloatParts, FromFloatError> {
  parts(value.to_bits(), FloatFormat::F64)
}

/// The parts of `value`, or the error of NaN or of an infinity.
#[inline(always)]
pub(crate) const fn float_parts_f32(value: f32) -> Result<FloatParts, FromFloatError> {
  parts(value.to_bits() as u64, FloatFormat::F32)
}

/// A float rounded to an integer: `significand * 2^shift`, negative if `negative` and the
/// significand is not zero, whose significand is at most `2^53`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Rounded {
  /// The sign of the float, which a value of zero drops.
  pub(crate) negative: bool,
  /// The significand of the integer.
  pub(crate) significand: u64,
  /// The exponent of the lowest bit of the significand.
  pub(crate) shift: u32,
}

impl Rounded {
  /// The number of bits of the magnitude, `significand * 2^shift`: none for zero.
  #[inline]
  pub(crate) const fn length(&self) -> u32 {
    match self.significand {
      0 => 0,
      significand => u64::BITS - significand.leading_zeros() + self.shift,
    }
  }
}

/// The value of `parts` rounded to an integer with `mode`, as `rounding::increments` rounds a
/// quotient: a value that is an integer is unchanged, and a negative value that rounds to zero
/// gives zero.
pub(crate) const fn round_magnitude(parts: FloatParts, mode: RoundingMode) -> Rounded {
  let FloatParts {
    negative,
    significand,
    exponent,
    precision,
  } = parts;
  // An exponent of zero or more shifts the significand up, which is exact.
  if exponent >= 0 {
    return Rounded {
      negative,
      significand,
      shift: exponent as u32,
    };
  }
  // The value is `significand / 2^dropped`: the quotient of that division and its remainder decide
  // the rounding.
  let dropped = exponent.unsigned_abs();
  let (quotient, remainder_is_zero, half) = if dropped > precision {
    // The significand is below `2^precision`, at most `2^(dropped - 1)`: the value is below one
    // half, so the quotient is zero and the remainder, the significand itself, is nearer to it than
    // to one. A shift by `dropped` would be by 64 bits or more for most such values.
    (0, significand == 0, Ordering::Less)
  } else {
    // `dropped <= precision <= 53`, so the divisor fits, and the remainder is below it.
    let divisor = 1 << dropped;
    let remainder = significand & (divisor - 1);
    let half = order::cmp_u128(remainder as u128, (divisor - remainder) as u128);
    (significand >> dropped, remainder == 0, half)
  };
  let away = rounding::increments(
    mode,
    negative,
    remainder_is_zero,
    half,
    quotient % 2 == 1,
    quotient % 5 == 0,
  );
  // The quotient is below `2^(precision - dropped)`, so one more does not overflow.
  Rounded {
    negative,
    significand: quotient + away as u64,
    shift: 0,
  }
}

/// Checks that `rounded` is within the range of `bits` bits, in two's complement if `signed` and
/// unsigned otherwise: from zero to `2^bits - 1`, or from `-2^(bits - 1)` to `2^(bits - 1) - 1`,
/// and only zero at zero bits. Returns the error of a value outside that range: `NegOverflow` below
/// it, as every negative value but zero is for an unsigned integer, and `PosOverflow` above it.
pub(crate) const fn check_range(
  rounded: Rounded,
  bits: u32,
  signed: bool,
) -> Result<(), FromFloatError> {
  let length = rounded.length();
  let fits = if length == 0 {
    true
  } else if !signed {
    !rounded.negative && length <= bits
  } else {
    // `2^(bits - 1)` has `bits` bits, and only its negation is in the range.
    length < bits || (rounded.negative && length == bits && rounded.significand.is_power_of_two())
  };
  match (fits, rounded.negative) {
    (true, _) => Ok(()),
    (false, true) => Err(FromFloatError::NegOverflow),
    (false, false) => Err(FromFloatError::PosOverflow),
  }
}
