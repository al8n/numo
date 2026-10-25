//! Helpers for the unit tests that compare with `num-bigint`: conversions between limb arrays and
//! big integers, the limits, powers of ten and bit patterns that they test, rounding by distance,
//! and fewer samples, exponents and bit indexes under Miri. A limb is any unsigned integer type
//! from `u8` to `u128`; the functions without a limb type parameter take `u64` limbs.

use core::{
  cmp::Ordering,
  ops::{Not, Shl, Sub},
};

use num_bigint::{BigInt, Sign};
use std::{vec, vec::Vec};

use crate::RoundingMode;

/// An unsigned integer type that holds the limbs of an array, least significant first.
pub(crate) trait Limb: Copy {
  /// The number of bits.
  const BITS: u32;

  /// Zero.
  const ZERO: Self;

  /// One.
  const ONE: Self;

  /// Every bit set.
  const MAX: Self;

  /// Appends the bytes of `self` to `bytes`, least significant first.
  fn push_le_bytes(self, bytes: &mut Vec<u8>);

  /// The limb whose bytes, least significant first, are `bytes`. Requires `BITS / 8` bytes.
  fn from_le_slice(bytes: &[u8]) -> Self;
}

macro_rules! limb {
  ($($limb:ident),+) => {$(
    impl Limb for $limb {
      const BITS: u32 = $limb::BITS;
      const ZERO: Self = 0;
      const ONE: Self = 1;
      const MAX: Self = $limb::MAX;

      fn push_le_bytes(self, bytes: &mut Vec<u8>) {
        bytes.extend(self.to_le_bytes());
      }

      fn from_le_slice(bytes: &[u8]) -> Self {
        $limb::from_le_bytes(bytes.try_into().expect("the bytes of one limb"))
      }
    }
  )+};
}

limb!(u8, u16, u32, u64, u128);

/// Every rounding mode.
pub(crate) const MODES: [RoundingMode; 7] = [
  RoundingMode::Up,
  RoundingMode::Down,
  RoundingMode::Ceiling,
  RoundingMode::Floor,
  RoundingMode::HalfUp,
  RoundingMode::HalfDown,
  RoundingMode::HalfEven,
];

/// `value / divisor` rounded to an integer with `mode`, for `divisor > 0`.
///
/// Picks between the integers below and above the exact quotient by their distances to it, and
/// breaks a tie by direction or parity. The crate instead compares the remainder of a division
/// toward zero with its distance to the divisor.
pub(crate) fn round(value: &BigInt, divisor: &BigInt, mode: RoundingMode) -> BigInt {
  // `floor * divisor <= value < (floor + 1) * divisor`; `/` rounds toward zero.
  let mut floor = value / divisor;
  if &floor * divisor > *value {
    floor -= 1u8;
  }
  let below = value - &floor * divisor;
  if below.sign() == Sign::NoSign {
    return floor;
  }
  let ceiling = &floor + 1u8;
  let above = &ceiling * divisor - value;
  let negative = value.sign() == Sign::Minus;
  let (toward_zero, away_from_zero) = if negative {
    (ceiling.clone(), floor.clone())
  } else {
    (floor.clone(), ceiling.clone())
  };
  match mode {
    RoundingMode::Up => away_from_zero,
    RoundingMode::Down => toward_zero,
    RoundingMode::Ceiling => ceiling,
    RoundingMode::Floor => floor,
    RoundingMode::HalfUp | RoundingMode::HalfDown | RoundingMode::HalfEven => {
      match below.cmp(&above) {
        Ordering::Less => floor,
        Ordering::Greater => ceiling,
        Ordering::Equal => match mode {
          RoundingMode::HalfUp => away_from_zero,
          RoundingMode::HalfDown => toward_zero,
          _ if (&floor % 2u8).sign() == Sign::NoSign => floor,
          _ => ceiling,
        },
      }
    }
  }
}

/// The integer that `limbs` holds, least significant limb first, in two's complement if `signed`.
pub(crate) fn from_limbs<L: Limb>(limbs: &[L], signed: bool) -> BigInt {
  let mut bytes = Vec::new();
  for &limb in limbs {
    limb.push_le_bytes(&mut bytes);
  }
  if signed {
    BigInt::from_signed_bytes_le(&bytes)
  } else {
    BigInt::from_bytes_le(Sign::Plus, &bytes)
  }
}

/// The smallest integer of `bits` bits, signed or unsigned.
pub(crate) fn min(bits: u32, signed: bool) -> BigInt {
  if signed {
    -(BigInt::from(1u8) << (bits - 1))
  } else {
    BigInt::ZERO
  }
}

/// The largest integer of `bits` bits, signed or unsigned.
pub(crate) fn max(bits: u32, signed: bool) -> BigInt {
  (BigInt::from(1u8) << (bits - u32::from(signed))) - 1u8
}

/// `value` in `N` limbs of type `L`, least significant first, in two's complement if `signed`, or
/// `None` if it is out of range.
pub(crate) fn to_limbs_of<L: Limb, const N: usize>(value: &BigInt, signed: bool) -> Option<[L; N]> {
  let negative = value.sign() == Sign::Minus;
  // The shortest two's complement or unsigned encoding, which fits exactly when the value does.
  let mut bytes = match (signed, negative) {
    (true, _) => value.to_signed_bytes_le(),
    (false, false) => value.to_bytes_le().1,
    (false, true) => return None,
  };
  let size = (L::BITS / 8) as usize;
  if bytes.len() > size * N {
    return None;
  }
  bytes.resize(size * N, if negative { 0xff } else { 0 });
  let mut limbs = [L::ZERO; N];
  for (limb, chunk) in limbs.iter_mut().zip(bytes.chunks_exact(size)) {
    *limb = L::from_le_slice(chunk);
  }
  Some(limbs)
}

/// `value` in `N` limbs of type `u64`, as [`to_limbs_of`] gives it.
pub(crate) fn to_limbs<const N: usize>(value: &BigInt, signed: bool) -> Option<[u64; N]> {
  to_limbs_of(value, signed)
}

/// `10^exp`.
pub(crate) fn pow10(exp: u32) -> BigInt {
  BigInt::from(10u8).pow(exp)
}

/// The largest `exp` such that `10^exp <= max`.
pub(crate) fn max_exp(max: &BigInt) -> u32 {
  let ten = BigInt::from(10u8);
  let mut exp = 0;
  // `10^(exp + 1)`, multiplied up by one factor of ten per step.
  let mut next = ten.clone();
  while next <= *max {
    exp += 1;
    next *= &ten;
  }
  exp
}

/// The exponents `0..=max_exp` to test, or under Miri only a few of them.
///
/// Scaling and division by powers of ten step by at most `step` digits, so the exponents kept are
/// the edges of one and two steps, plus the top. Under Miri big-integer arithmetic is slow, and on
/// a 32-bit target every exponent would use up its address space.
pub(crate) fn exponents_by(max_exp: u32, step: u32) -> Vec<u32> {
  if !cfg!(miri) {
    return (0..=max_exp).collect();
  }
  let mut exps: Vec<u32> = [
    0,
    1,
    step - 1,
    step,
    step + 1,
    2 * step,
    2 * step + 1,
    max_exp.saturating_sub(1),
    max_exp,
  ]
  .into_iter()
  .filter(|&exp| exp <= max_exp)
  .collect();
  exps.sort_unstable();
  exps.dedup();
  exps
}

/// [`exponents_by`] with the step of `int::unsigned::b64`, 19 digits (`MAX_STEP`).
pub(crate) fn exponents(max_exp: u32) -> Vec<u32> {
  exponents_by(max_exp, 19)
}

/// `samples`, or every third of them under Miri, which runs big-integer arithmetic slowly.
pub(crate) fn thin<T>(samples: Vec<T>) -> Vec<T> {
  let step = if cfg!(miri) { 3 } else { 1 };
  samples.into_iter().step_by(step).collect()
}

/// Bit patterns of `N` limbs of type `L`: zero, all ones, and for each bit index `k`, bit `k`
/// alone, every bit but `k`, every bit below `k`, and every bit from `k` up.
///
/// Every index is swept, or under Miri only the indexes next to the first limb boundary, to the
/// top bit and to each of `widths`, which give the limits of those widths and the patterns just
/// beyond them.
pub(crate) fn bit_patterns_of<L, const N: usize>(widths: &[u32]) -> Vec<[L; N]>
where
  L: Limb + Not<Output = L> + Shl<u32, Output = L> + Sub<Output = L>,
{
  let bits = L::BITS * N as u32;
  let indexes: Vec<u32> = if cfg!(miri) {
    let mut indexes = vec![0, 1, L::BITS - 1, L::BITS, L::BITS + 1, bits - 2, bits - 1];
    for &width in widths {
      indexes.extend([
        width.saturating_sub(2),
        width.saturating_sub(1),
        width,
        width + 1,
      ]);
    }
    indexes.retain(|&index| index < bits);
    indexes.sort_unstable();
    indexes.dedup();
    indexes
  } else {
    (0..bits).collect()
  };
  let mut patterns = vec![[L::ZERO; N], [L::MAX; N]];
  for index in indexes {
    let (limb, bit) = ((index / L::BITS) as usize, index % L::BITS);
    let mut single = [L::ZERO; N];
    single[limb] = L::ONE << bit;
    let mut below = [L::ZERO; N];
    below[..limb].fill(L::MAX);
    below[limb] = (L::ONE << bit) - L::ONE;
    patterns.extend([single, single.map(|l| !l), below, below.map(|l| !l)]);
  }
  patterns
}
