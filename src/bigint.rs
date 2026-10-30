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

use crate::{
  FitsBits, FixedInteger, InlineIntRepr, InlineUintRepr, Int, RoundingMode, SignedFixedInteger,
  Uint,
};

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

/// The smallest integer of `bits` bits, signed or unsigned: zero for zero bits.
pub(crate) fn min(bits: u32, signed: bool) -> BigInt {
  if signed && bits > 0 {
    -(BigInt::from(1u8) << (bits - 1))
  } else {
    BigInt::ZERO
  }
}

/// The largest integer of `bits` bits, signed or unsigned: zero for zero bits.
pub(crate) fn max(bits: u32, signed: bool) -> BigInt {
  (BigInt::from(1u8) << bits.saturating_sub(u32::from(signed))) - 1u8
}

/// `value` in `N` limbs of type `L`, least significant first, in two's complement if `signed`, or
/// `None` if it is out of range.
pub(crate) fn to_limbs_of<L: Limb, const N: usize>(value: &BigInt, signed: bool) -> Option<[L; N]> {
  // `num-bigint` writes zero as one byte, which no limbs hold, though they hold zero.
  if value.sign() == Sign::NoSign {
    return Some([L::ZERO; N]);
  }
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

/// `exact` wrapped to the range of `bits` bits that starts at `min`: `min` plus `exact - min`
/// modulo `2^bits`.
pub(crate) fn wrap(exact: &BigInt, bits: u32, min: &BigInt) -> BigInt {
  let modulus = BigInt::from(1u8) << bits;
  // `%` takes the sign of the dividend, so a negative remainder moves up by one modulus.
  let offset = (exact - min) % &modulus;
  if offset.sign() == Sign::Minus {
    offset + modulus + min
  } else {
    offset + min
  }
}

/// The quotient and the remainder of `a / b` with the remainder at least zero, as the Euclidean
/// division of the native integers gives them. Requires `b != 0`.
pub(crate) fn div_rem_euclid(a: &BigInt, b: &BigInt) -> (BigInt, BigInt) {
  // `/` and `%` round toward zero, so the remainder has the sign of `a`.
  let (quotient, remainder) = (a / b, a % b);
  if remainder.sign() != Sign::Minus {
    (quotient, remainder)
  } else if b.sign() == Sign::Minus {
    (quotient + 1u8, remainder - b)
  } else {
    (quotient - 1u8, remainder + b)
  }
}

/// `a / b` rounded down. Requires `b != 0`.
pub(crate) fn div_floor(a: &BigInt, b: &BigInt) -> BigInt {
  let (quotient, remainder) = (a / b, a % b);
  if remainder.sign() != Sign::NoSign && remainder.sign() != b.sign() {
    quotient - 1u8
  } else {
    quotient
  }
}

/// `a / b` rounded up. Requires `b != 0`.
pub(crate) fn div_ceil(a: &BigInt, b: &BigInt) -> BigInt {
  -div_floor(&-a, b)
}

/// For a positive `b`, the smallest multiple of `b` at least `a`, and for a negative `b`, the
/// largest multiple of `b` at most `a`: `ceil(a / b) * b` for both. Requires `b != 0`.
pub(crate) fn next_multiple_of(a: &BigInt, b: &BigInt) -> BigInt {
  div_ceil(a, b) * b
}

/// `value^exp` wrapped to the range of `bits` bits from `min` to `max`, and whether it is outside
/// that range.
pub(crate) fn pow_bits(
  value: &BigInt,
  exp: u32,
  bits: u32,
  min: &BigInt,
  max: &BigInt,
) -> (BigInt, bool) {
  let magnitude_bits = value.magnitude().bits();
  // A magnitude of at least 2^k has a power of at least `2^(k * exp)`, outside the range once
  // `k * exp >= bits`; otherwise the exact power has fewer than `bits + exp` bits.
  if magnitude_bits >= 2 && (magnitude_bits - 1) * u64::from(exp) >= u64::from(bits) {
    let modulus = BigInt::from(1u8) << bits;
    let residue = value.modpow(&BigInt::from(exp), &modulus);
    (wrap(&residue, bits, min), true)
  } else {
    let exact = value.pow(exp);
    (wrap(&exact, bits, min), exact < *min || *max < exact)
  }
}

/// The logarithm of `value` to the base `base`, rounded down, or `None` if `value` is not positive
/// or `base` is below 2.
pub(crate) fn ilog(value: &BigInt, base: &BigInt) -> Option<u32> {
  if value.sign() != Sign::Plus || *base < BigInt::from(2u8) {
    return None;
  }
  // A floating-point estimate, which the powers below correct to the exact logarithm.
  let mut log = (log2(value) / log2(base)).floor() as u32;
  while log > 0 && base.pow(log) > *value {
    log -= 1;
  }
  while base.pow(log + 1) <= *value {
    log += 1;
  }
  Some(log)
}

/// An estimate of the base-2 logarithm of `value`, which is positive, from its top 64 bits.
fn log2(value: &BigInt) -> f64 {
  let shift = value.bits().saturating_sub(64);
  let top = (value.magnitude() >> shift)
    .iter_u64_digits()
    .next()
    .unwrap_or(0);
  shift as f64 + (top as f64).log2()
}

/// The smallest power of two at least `value`, which is not negative.
pub(crate) fn next_power_of_two(value: &BigInt) -> BigInt {
  if *value <= BigInt::from(1u8) {
    BigInt::from(1u8)
  } else {
    BigInt::from(1u8) << (value - 1u8).bits()
  }
}

/// The widths at which the arithmetic of a width is checked in `limbs` limbs of `limb_bits` bits:
/// every width from zero to `limb_bits * limbs` for limbs of at most 16 bits, and otherwise, or
/// under Miri, zero, one, two and the widths next to each multiple of `limb_bits`.
pub(crate) fn arithmetic_widths(limb_bits: u32, limbs: usize) -> Vec<u32> {
  let top = limb_bits * limbs as u32;
  if limb_bits <= 16 && !cfg!(miri) {
    return (0..=top).collect();
  }
  let mut widths = vec![0, 1, 2];
  for k in 1..=limbs as u32 {
    widths.extend([limb_bits * k - 1, limb_bits * k, limb_bits * k + 1]);
  }
  widths.retain(|&width| width <= top);
  widths.sort_unstable();
  widths.dedup();
  widths
}

/// The values from `min` to `max`, the range of `bits` bits, at which the arithmetic of a width is
/// checked for limbs of `limb_bits` bits: the limits and the values next to them and to zero, the
/// powers `±2^k` and the values next to them for `k` next to the limb boundaries and to `bits`, and
/// values from `random`, wrapped to the range. Under Miri, fewer random values, and the values that
/// [`thin`] keeps.
pub(crate) fn arithmetic_samples(
  bits: u32,
  limb_bits: u32,
  min: &BigInt,
  max: &BigInt,
  mut random: impl FnMut() -> BigInt,
) -> Vec<BigInt> {
  let mut values = vec![
    min.clone(),
    min + 1u8,
    min + 2u8,
    max - 2u8,
    max - 1u8,
    max.clone(),
  ];
  values.extend([-2, -1, 0, 1, 2].map(BigInt::from));
  let mut indexes = vec![0, 1, bits.saturating_sub(2), bits.saturating_sub(1)];
  for k in 1..=bits / limb_bits {
    indexes.extend([limb_bits * k - 1, limb_bits * k, limb_bits * k + 1]);
  }
  for k in indexes.into_iter().filter(|&k| k < bits) {
    let power = BigInt::from(1u8) << k;
    values.extend([&power - 1u8, power.clone(), &power + 1u8]);
    values.extend([1u8 - &power, -&power, -&power - 1u8]);
  }
  let count = if cfg!(miri) { 2 } else { 16 };
  values.extend((0..count).map(|_| wrap(&random(), bits, min)));
  values.retain(|value| min <= value && value <= max);
  values.sort_unstable();
  values.dedup();
  thin(values)
}

/// Bit patterns of `bits` bits, at most 128, read as unsigned integers: zero, one, two, every bit
/// set and the two patterns below it, the limits of the two's complement range and the patterns
/// next to them, `2^k`, `2^k ± 1` and `2^bits - 2^k` for every `k` below `bits`, `2^8` and the
/// patterns next to it, and 16 seeded random patterns, each reduced to `bits` bits.
pub(crate) fn structured_patterns(bits: u32) -> Vec<u128> {
  let mask = match bits {
    0 => 0,
    _ => u128::MAX >> (128 - bits),
  };
  // The smallest value of the two's complement range, or zero for zero bits.
  let min: u128 = match bits {
    0 => 0,
    _ => 1 << (bits - 1),
  };
  let mut patterns = vec![0, 1, 2, mask, mask.wrapping_sub(1), mask.wrapping_sub(2)];
  patterns.extend([
    min.wrapping_sub(3),
    min.wrapping_sub(2),
    min.wrapping_sub(1),
  ]);
  patterns.extend([min, min.wrapping_add(1), min.wrapping_add(2), 255, 256, 257]);
  for k in 0..bits {
    let power = 1u128 << k;
    patterns.extend([
      power.wrapping_sub(1),
      power,
      power.wrapping_add(1),
      mask.wrapping_sub(power).wrapping_add(1),
    ]);
  }
  // A xorshift generator seeded by the width, so that a failure replays.
  let mut state = 0x9e37_79b9_7f4a_7c15 ^ u64::from(bits);
  for _ in 0..16 {
    let mut word = || {
      state ^= state >> 12;
      state ^= state << 25;
      state ^= state >> 27;
      state.wrapping_mul(0x2545_f491_4f6c_dd1d)
    };
    patterns.push(u128::from(word()) << 64 | u128::from(word()));
  }
  for pattern in &mut patterns {
    *pattern &= mask;
  }
  patterns.sort_unstable();
  patterns.dedup();
  patterns
}

/// `pattern`, of `bits` bits, at most 128, read in two's complement.
pub(crate) fn to_signed(pattern: u128, bits: u32) -> i128 {
  match bits {
    0 => 0,
    // Every pattern of 128 bits is the `i128` of the same bits.
    128 => pattern as i128,
    _ if pattern >> (bits - 1) == 1 => pattern as i128 - (1 << bits),
    _ => pattern as i128,
  }
}

/// A sample that [`thin`] keeps when it is zero or a unit, which a stride may skip.
pub(crate) trait Sample {
  /// Whether the sample is zero, one or minus one; for limbs, whether they are all zeros, one or
  /// all ones.
  fn is_unit_or_zero(&self) -> bool;
}

impl Sample for BigInt {
  fn is_unit_or_zero(&self) -> bool {
    self.magnitude().bits() <= 1
  }
}

macro_rules! sample {
  (unsigned $($int:ident),+) => {$(
    impl Sample for $int {
      fn is_unit_or_zero(&self) -> bool {
        *self <= 1
      }
    }
  )+};
  (signed $($int:ident),+) => {$(
    impl Sample for $int {
      fn is_unit_or_zero(&self) -> bool {
        (-1..=1).contains(self)
      }
    }
  )+};
}

sample!(unsigned u8, u16, u32, u64, u128);
sample!(signed i8, i16, i32, i64, i128);

impl<L: Limb + PartialEq, const N: usize> Sample for [L; N] {
  fn is_unit_or_zero(&self) -> bool {
    let above_first_are_zero = self.iter().skip(1).all(|&limb| limb == L::ZERO);
    let first = self.first().copied().unwrap_or(L::ZERO);
    (above_first_are_zero && (first == L::ZERO || first == L::ONE))
      || self.iter().all(|&limb| limb == L::MAX)
  }
}

impl<B: FitsBits<R>, R: InlineUintRepr> Sample for Uint<B, R> {
  fn is_unit_or_zero(&self) -> bool {
    FixedInteger::is_zero(self) || FixedInteger::is_one(self)
  }
}

impl<B: FitsBits<R>, R: InlineIntRepr> Sample for Int<B, R> {
  fn is_unit_or_zero(&self) -> bool {
    // The sign is the value exactly for -1, 0 and 1.
    SignedFixedInteger::signum(self) == *self
  }
}

/// `samples`, or under Miri, which runs big-integer arithmetic slowly, every third of them, as
/// [`thin_by`] keeps them.
pub(crate) fn thin<T: Sample>(samples: Vec<T>) -> Vec<T> {
  thin_by(samples, 3, T::is_unit_or_zero)
}

/// `samples`, or under Miri every `step`th of them, the first, the last and every sample for which
/// `keep` returns `true`, such as zero and the units, which a stride may skip.
pub(crate) fn thin_by<T>(samples: Vec<T>, step: usize, keep: impl Fn(&T) -> bool) -> Vec<T> {
  if !cfg!(miri) {
    return samples;
  }
  let last = samples.len().saturating_sub(1);
  samples
    .into_iter()
    .enumerate()
    .filter(|(i, sample)| i % step == 0 || *i == last || keep(sample))
    .map(|(_, sample)| sample)
    .collect()
}

/// The values of `strided` and of `ends`, which a stride may skip, sorted and without duplicates.
pub(crate) fn with_ends<T: Ord>(
  strided: impl IntoIterator<Item = T>,
  ends: impl IntoIterator<Item = T>,
) -> Vec<T> {
  let mut values: Vec<T> = strided.into_iter().chain(ends).collect();
  values.sort_unstable();
  values.dedup();
  values
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
    let mut indexes = vec![
      0,
      1,
      L::BITS - 1,
      L::BITS,
      L::BITS + 1,
      bits.saturating_sub(2),
      bits.saturating_sub(1),
    ];
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
