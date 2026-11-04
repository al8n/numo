//! Helpers for the unit tests that compare with `num-bigint`: conversions between limb arrays and
//! big integers, the limits, powers of ten and bit patterns that they test, rounding by distance,
//! and fewer samples, exponents and bit indexes under Miri. A limb is any unsigned integer type
//! from `u8` to `u128`; the functions without a limb type parameter take `u64` limbs.
//!
//! The tests of the bit operations compare with no big integer: [`model`] computes their results
//! one bit at a time, and the tiers of [`tier`] size every sweep by the build.

use core::{
  cmp::Ordering,
  num::IntErrorKind,
  ops::{Not, Shl, Sub},
};

use num_bigint::{BigInt, Sign};
use std::{format, string::String, vec, vec::Vec};

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

  /// The limb as a `u128`, which holds every limb.
  fn to_u128(self) -> u128;

  /// The limb of the low `BITS` bits of `value`.
  fn from_u128(value: u128) -> Self;
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

      fn to_u128(self) -> u128 {
        u128::from(self)
      }

      fn from_u128(value: u128) -> Self {
        value as $limb
      }
    }
  )+};
}

limb!(u8, u16, u32, u64, u128);

/// How thoroughly a sweep checks: every value, pair and pattern that is cheap enough in an
/// optimized build, which runs them fast; fewer in an unoptimized build, which every feature
/// combination and every sanitizer of the tests runs; and the fewest under Miri.
#[derive(Clone, Copy, Debug, PartialEq, Eq, derive_more::IsVariant)]
pub(crate) enum Tier {
  /// Under Miri.
  Miri,
  /// In an unoptimized build, with debug assertions.
  Debug,
  /// In an optimized build.
  Full,
}

/// The tier of this build. `--cfg all_tests` does not raise it: the sanitizers that set that flag
/// run unoptimized code.
pub(crate) fn tier() -> Tier {
  if cfg!(miri) {
    Tier::Miri
  } else if cfg!(debug_assertions) {
    Tier::Debug
  } else {
    Tier::Full
  }
}

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

/// `a / b` rounded down, for the values of up to 64 bits that `i128` checks quickly. Requires
/// `b != 0`.
pub(crate) fn floor_div(a: i128, b: i128) -> i128 {
  let (quotient, remainder) = (a / b, a % b);
  if remainder != 0 && (remainder < 0) != (b < 0) {
    quotient - 1
  } else {
    quotient
  }
}

/// `a / b` rounded up, as [`floor_div`]. Requires `b != 0`.
pub(crate) fn ceil_div(a: i128, b: i128) -> i128 {
  -floor_div(-a, b)
}

/// `ceil(a / b) * b`, as [`next_multiple_of`] for `i128`. Requires `b != 0`.
pub(crate) fn next_multiple(a: i128, b: i128) -> i128 {
  ceil_div(a, b) * b
}

/// `value^exp` wrapped to the range of `bits` bits from `min` to `max`, at most 64 bits, and
/// whether it is outside that range, as [`pow_bits`] for `i128`.
pub(crate) fn pow_wrapped(value: i128, exp: u32, bits: u32, min: i128, max: i128) -> (i128, bool) {
  let modulus = 1i128 << bits;
  // The power modulo `2^128`, and so modulo `2^bits`, moved into the range.
  let residue = (value as u128).wrapping_pow(exp) as i128 & (modulus - 1);
  let wrapped = (residue - min).rem_euclid(modulus) + min;
  // A power that `i128` does not hold is outside every range of 64 bits.
  let outside = value
    .checked_pow(exp)
    .is_none_or(|exact| exact < min || max < exact);
  (wrapped, outside)
}

/// The `n`th root of `value`, rounded toward zero, as [`BigInt::nth_root`] for `i128`. Requires
/// `n >= 1`, and `n` odd for a negative `value`.
pub(crate) fn nth_root(value: i128, n: u32) -> i128 {
  let magnitude = value.unsigned_abs();
  // The largest `root` with `root^n <= magnitude`, by bisection from `low^n <= magnitude` and
  // `high^n > magnitude`.
  let (mut low, mut high) = (0u128, magnitude + 1);
  while high - low > 1 {
    let middle = low + (high - low) / 2;
    if middle
      .checked_pow(n)
      .is_some_and(|power| power <= magnitude)
    {
      low = middle;
    } else {
      high = middle;
    }
  }
  let root = i128::try_from(low).expect("a root of an i128");
  if value < 0 { -root } else { root }
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
  bit_patterns_by(widths, cfg!(miri))
}

/// The bit patterns of [`bit_patterns_of`], at every index, or if `reduced` only at the indexes
/// that it keeps under Miri.
pub(crate) fn bit_patterns_by<L, const N: usize>(widths: &[u32], reduced: bool) -> Vec<[L; N]>
where
  L: Limb + Not<Output = L> + Shl<u32, Output = L> + Sub<Output = L>,
{
  let bits = L::BITS * N as u32;
  let indexes: Vec<u32> = if reduced {
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

/// The amounts of the shifts and the rotations, which also serve as bit indexes, at which the bit
/// operations of `width` bits in words of `word_bits` bits are checked: 0, 1, `word_bits - 1`,
/// `word_bits`, `word_bits + 1`, `width - 1`, `width`, `width + 1`, `2 * width + 1` and
/// `u32::MAX`, or under Miri 0, 1, `width - 1`, `width`, `width + 1` and `u32::MAX`.
pub(crate) fn bit_amounts(word_bits: u32, width: u32) -> Vec<u32> {
  let mut amounts = vec![0, 1, width.saturating_sub(1), width, width + 1, u32::MAX];
  if !tier().is_miri() {
    amounts.extend([word_bits - 1, word_bits, word_bits + 1, 2 * width + 1]);
  }
  amounts.sort_unstable();
  amounts.dedup();
  amounts
}

/// The pattern of the low `width` bits of `limbs`, least significant limb first, reading each bit
/// of a limb alone. Requires `width` to be at most the bits of the limbs.
pub(crate) fn pattern_of_limbs<L: Limb>(limbs: &[L], width: u32) -> Vec<bool> {
  model::pattern(width, |i| {
    (limbs[(i / L::BITS) as usize].to_u128() >> (i % L::BITS)) & 1 == 1
  })
}

/// The `N` limbs of type `L`, least significant first, that hold `pattern` extended by zeros, or
/// if `signed` by copies of its sign, written one bit at a time. Requires the limbs to have at
/// least the bits of `pattern`.
pub(crate) fn limbs_of_pattern<L: Limb, const N: usize>(pattern: &[bool], signed: bool) -> [L; N] {
  let size = L::BITS as usize;
  let extended = model::extend(pattern, L::BITS * N as u32, signed);
  core::array::from_fn(|k| {
    let bits = &extended[size * k..size * (k + 1)];
    L::from_u128(
      bits
        .iter()
        .rev()
        .fold(0, |limb, &bit| limb << 1 | u128::from(bit)),
    )
  })
}

/// The symbols of the texts that the parsers of the integers are checked on: the digits 0, 1 and
/// 9, the letters `a` and `z`, digits from radix 11 and in radix 36, both signs, and `_`, which no
/// radix reads.
const TEXT_SYMBOLS: &[u8] = b"019az+-_";

/// Every text of up to `length` symbols of [`TEXT_SYMBOLS`], the empty text first.
fn texts_up_to(length: usize) -> Vec<Vec<u8>> {
  let mut texts = vec![Vec::new()];
  // The texts of the length before, from which the next length grows.
  let mut start = 0;
  for _ in 0..length {
    let end = texts.len();
    for k in start..end {
      for &symbol in TEXT_SYMBOLS {
        let mut text = texts[k].clone();
        text.push(symbol);
        texts.push(text);
      }
    }
    start = end;
  }
  texts
}

/// The texts at which the parsers of the integers are checked, as the tier takes them: every text
/// of up to four symbols of [`TEXT_SYMBOLS`] in an optimized build, of up to three in an
/// unoptimized one, and of up to two under Miri.
pub(crate) fn parse_texts() -> Vec<Vec<u8>> {
  texts_up_to(match tier() {
    Tier::Full => 4,
    Tier::Debug => 3,
    Tier::Miri => 2,
  })
}

/// The radixes at which the parsers of the integers are checked: 2, 10, 16 and 36, or under Miri
/// 10 and 36.
pub(crate) fn parse_radixes() -> &'static [u32] {
  if tier().is_miri() {
    &[10, 36]
  } else {
    &[2, 10, 16, 36]
  }
}

/// The smallest and the largest value of `bits` bits, at most 127, unsigned or in two's
/// complement.
fn text_range(bits: u32, signed: bool) -> (i128, i128) {
  match (bits, signed) {
    (0, _) => (0, 0),
    (_, false) => (0, i128::MAX >> (127 - bits)),
    (_, true) => (i128::MIN >> (128 - bits), i128::MAX >> (128 - bits)),
  }
}

/// What the native `from_str_radix` of an integer of `bits` bits, at most 127, unsigned or in two's
/// complement, returns for `src`: the value, or the kind of the error. Written as the native parser
/// reads, one digit at a time: an empty text is `Empty`, a lone sign `InvalidDigit`, `-` a sign only
/// for a signed integer, and at each digit the product by `radix`, then the digit, whose absence is
/// `InvalidDigit`, then the overflow of the product or the sum out of the range of `bits` bits.
pub(crate) fn parse(src: &[u8], radix: u32, bits: u32, signed: bool) -> Result<i128, IntErrorKind> {
  let (min, max) = text_range(bits, signed);
  let (negative, digits) = match src {
    [] => return Err(IntErrorKind::Empty),
    [b'+' | b'-'] => return Err(IntErrorKind::InvalidDigit),
    [b'+', rest @ ..] => (false, rest),
    [b'-', rest @ ..] if signed => (true, rest),
    _ => (false, src),
  };
  let overflow = if negative {
    IntErrorKind::NegOverflow
  } else {
    IntErrorKind::PosOverflow
  };
  let mut value: i128 = 0;
  for &byte in digits {
    let product = value.checked_mul(i128::from(radix));
    let digit = char::from(byte)
      .to_digit(radix)
      .ok_or(IntErrorKind::InvalidDigit)?;
    let next = product.and_then(|product| {
      if negative {
        product.checked_sub(i128::from(digit))
      } else {
        product.checked_add(i128::from(digit))
      }
    });
    value = next
      .filter(|next| (min..=max).contains(next))
      .ok_or(overflow)?;
  }
  Ok(value)
}

/// The digits of `magnitude` in `radix`, at most 36, most significant first and in lowercase, and
/// `0` for zero.
pub(crate) fn radix_digits(mut magnitude: u128, radix: u32) -> String {
  let mut digits = Vec::new();
  loop {
    let digit = (magnitude % u128::from(radix)) as u32;
    digits.push(char::from_digit(digit, radix).expect("a digit of the radix"));
    magnitude /= u128::from(radix);
    if magnitude == 0 {
      return digits.iter().rev().collect();
    }
  }
}

/// `digits`, the digits of a number in `radix`, plus one.
fn increment(digits: &str, radix: u32) -> String {
  let mut digits: Vec<u32> = digits
    .chars()
    .map(|digit| digit.to_digit(radix).expect("a digit of the radix"))
    .collect();
  // The carry runs from the last digit up through every `radix - 1`.
  match digits.iter().rposition(|&digit| digit + 1 < radix) {
    Some(k) => {
      digits[k] += 1;
      digits[k + 1..].fill(0);
    }
    None => {
      digits.fill(0);
      digits.insert(0, 1);
    }
  }
  digits
    .iter()
    .map(|&digit| char::from_digit(digit, radix).expect("a digit of the radix"))
    .collect()
}

/// The texts in `radix` of the values next to the limits of `bits` bits, at most 128, unsigned or
/// in two's complement: zero, one, each limit and the values next to it on both sides, with `-`
/// before a negative value, each also after two leading zeros and a non-negative one after `+`, and
/// each also followed by `x`, a digit only from radix 34, and by `_`, which no radix reads.
pub(crate) fn limit_texts(bits: u32, signed: bool, radix: u32) -> Vec<Vec<u8>> {
  // The largest value, and for a signed integer the magnitude of the smallest one.
  let (max, min): (u128, u128) = match (bits, signed) {
    (0, _) => (0, 0),
    (_, false) => (u128::MAX >> (128 - bits), 0),
    (1, true) => (0, 1),
    (_, true) => (u128::MAX >> (129 - bits), 1 << (bits - 1)),
  };
  let text_of = |magnitude: u128| radix_digits(magnitude, radix);
  // Each value as its sign and the digits of its magnitude: one more than the largest value of 128
  // unsigned bits needs 129 bits.
  let above = match max.checked_add(1) {
    Some(above) => text_of(above),
    None => increment(&text_of(max), radix),
  };
  let mut values = vec![
    (false, text_of(0)),
    (false, text_of(1)),
    (false, text_of(max.saturating_sub(1))),
    (false, text_of(max)),
    (false, above),
  ];
  if signed {
    values.extend([
      (true, text_of(1)),
      (true, text_of(min.saturating_sub(1))),
      (true, text_of(min)),
      (true, text_of(min + 1)),
    ]);
  }
  values.sort_unstable();
  values.dedup();
  let mut texts = Vec::new();
  for (negative, digits) in values {
    let sign = if negative { "-" } else { "" };
    let mut forms = vec![format!("{sign}{digits}"), format!("{sign}00{digits}")];
    if !negative {
      forms.push(format!("+{digits}"));
    }
    for form in forms {
      for suffix in ["", "x", "_"] {
        texts.push(format!("{form}{suffix}").into_bytes());
      }
    }
  }
  texts
}

/// The bit operations on patterns of bits, bit 0 first, as wide as the integers that hold them:
/// the model that the tests of the bit operations compare with. Each operation is written from its
/// definition, one bit at a time, and reads a pattern as unsigned, or in two's complement where it
/// takes `signed`.
pub(crate) mod model {
  use std::{format, string::String, vec::Vec};

  /// The pattern of `width` bits whose bit `i` is `bit(i)`.
  pub(crate) fn pattern(width: u32, bit: impl FnMut(u32) -> bool) -> Vec<bool> {
    (0..width).map(bit).collect()
  }

  /// The width of `p`, its number of bits.
  pub(crate) fn width(p: &[bool]) -> u32 {
    p.len() as u32
  }

  /// The pattern of the low `width` bits of `value`. Requires `width <= 128`.
  pub(crate) fn of_u128(value: u128, width: u32) -> Vec<bool> {
    pattern(width, |i| (value >> i) & 1 == 1)
  }

  /// The unsigned integer of `p`. Requires a width of at most 128 bits.
  pub(crate) fn to_u128(p: &[bool]) -> u128 {
    p.iter()
      .rev()
      .fold(0, |value, &bit| value << 1 | u128::from(bit))
  }

  /// The integer of `p` in two's complement, zero for zero bits. Requires a width of at most 128
  /// bits.
  pub(crate) fn to_i128(p: &[bool]) -> i128 {
    to_u128(&extend(p, 128, true)) as i128
  }

  /// `p` as text, from the top bit down, for the messages of the assertions.
  pub(crate) fn text(p: &[bool]) -> String {
    let digits: String = p
      .iter()
      .rev()
      .map(|&bit| if bit { '1' } else { '0' })
      .collect();
    format!("0b{digits}")
  }

  /// The bit at `index` of `p`, or `false` from its width up.
  pub(crate) fn bit(p: &[bool], index: u32) -> bool {
    p.get(index as usize).copied().unwrap_or(false)
  }

  /// The sign of `p` in two's complement: its top bit, or `false` for zero bits.
  pub(crate) fn sign(p: &[bool]) -> bool {
    p.last().copied().unwrap_or(false)
  }

  /// `p` extended to `width` bits by zeros, or if `signed` by copies of its sign. Requires `width`
  /// to be at least the width of `p`.
  pub(crate) fn extend(p: &[bool], width: u32, signed: bool) -> Vec<bool> {
    let fill = signed && sign(p);
    pattern(width, |i| p.get(i as usize).copied().unwrap_or(fill))
  }

  /// Every bit of `p` flipped.
  pub(crate) fn not(p: &[bool]) -> Vec<bool> {
    p.iter().map(|&bit| !bit).collect()
  }

  /// `p & q`, bit by bit. Requires `q` to have the width of `p`.
  pub(crate) fn and(p: &[bool], q: &[bool]) -> Vec<bool> {
    p.iter().zip(q).map(|(&a, &b)| a && b).collect()
  }

  /// `p | q`, bit by bit. Requires `q` to have the width of `p`.
  pub(crate) fn or(p: &[bool], q: &[bool]) -> Vec<bool> {
    p.iter().zip(q).map(|(&a, &b)| a || b).collect()
  }

  /// `p ^ q`, bit by bit. Requires `q` to have the width of `p`.
  pub(crate) fn xor(p: &[bool], q: &[bool]) -> Vec<bool> {
    p.iter().zip(q).map(|(&a, &b)| a != b).collect()
  }

  /// The number of ones of `p`.
  pub(crate) fn count_ones(p: &[bool]) -> u32 {
    p.iter().filter(|&&bit| bit).count() as u32
  }

  /// The number of zeros of `p` from its top bit down: its width for zero.
  pub(crate) fn leading_zeros(p: &[bool]) -> u32 {
    p.iter().rev().take_while(|&&bit| !bit).count() as u32
  }

  /// The number of zeros of `p` from bit 0 up: its width for zero.
  pub(crate) fn trailing_zeros(p: &[bool]) -> u32 {
    p.iter().take_while(|&&bit| !bit).count() as u32
  }

  /// The number of ones of `p` from its top bit down.
  pub(crate) fn leading_ones(p: &[bool]) -> u32 {
    p.iter().rev().take_while(|&&bit| bit).count() as u32
  }

  /// The number of ones of `p` from bit 0 up.
  pub(crate) fn trailing_ones(p: &[bool]) -> u32 {
    p.iter().take_while(|&&bit| bit).count() as u32
  }

  /// The index of the highest one of `p`, or `None` for zero.
  pub(crate) fn highest_one(p: &[bool]) -> Option<u32> {
    p.iter().rposition(|&bit| bit).map(|index| index as u32)
  }

  /// The index of the lowest one of `p`, or `None` for zero.
  pub(crate) fn lowest_one(p: &[bool]) -> Option<u32> {
    p.iter().position(|&bit| bit).map(|index| index as u32)
  }

  /// The pattern of `width` bits whose only one is at `index`, or zero for `None`.
  pub(crate) fn only(width: u32, index: Option<u32>) -> Vec<bool> {
    pattern(width, |i| Some(i) == index)
  }

  /// The bits of `p` in reverse order.
  pub(crate) fn reverse(p: &[bool]) -> Vec<bool> {
    p.iter().rev().copied().collect()
  }

  /// `p` rotated left by `n` places within its width: bit `i` moves to bit `(i + n) % width`.
  pub(crate) fn rotate_left(p: &[bool], n: u32) -> Vec<bool> {
    let width = width(p);
    if width == 0 {
      return Vec::new();
    }
    let k = n % width;
    pattern(width, |i| p[((i + width - k) % width) as usize])
  }

  /// `p` rotated right by `n` places within its width: bit `(i + n) % width` moves to bit `i`.
  pub(crate) fn rotate_right(p: &[bool], n: u32) -> Vec<bool> {
    let width = width(p);
    if width == 0 {
      return Vec::new();
    }
    let k = n % width;
    pattern(width, |i| p[((i + k) % width) as usize])
  }

  /// `p` shifted left by `n` places within its width: zeros come in, and the bits that pass the top
  /// are lost, every bit once `n` is the width or more.
  pub(crate) fn shl(p: &[bool], n: u32) -> Vec<bool> {
    pattern(width(p), |i| i >= n && p[(i - n) as usize])
  }

  /// `p` shifted right by `n` places: copies of its sign come in if `signed`, and zeros otherwise.
  pub(crate) fn shr(p: &[bool], n: u32, signed: bool) -> Vec<bool> {
    let fill = signed && sign(p);
    pattern(width(p), |i| match i.checked_add(n) {
      Some(j) if j < width(p) => p[j as usize],
      _ => fill,
    })
  }

  /// `p` shifted left by `n` modulo its width, and whether `n` is the width or more, as it always
  /// is at zero bits.
  pub(crate) fn overflowing_shl(p: &[bool], n: u32) -> (Vec<bool>, bool) {
    match width(p) {
      0 => (Vec::new(), true),
      width => (shl(p, n % width), n >= width),
    }
  }

  /// `p` shifted right by `n` modulo its width, as [`shr`] shifts it, and whether `n` is the width
  /// or more, as it always is at zero bits.
  pub(crate) fn overflowing_shr(p: &[bool], n: u32, signed: bool) -> (Vec<bool>, bool) {
    match width(p) {
      0 => (Vec::new(), true),
      width => (shr(p, n % width, signed), n >= width),
    }
  }

  /// `p` shifted left by `n`, if `n` is below its width and shifting the result back, as [`shr`]
  /// does, gives `p`; or `None`.
  pub(crate) fn shl_exact(p: &[bool], n: u32, signed: bool) -> Option<Vec<bool>> {
    let shifted = shl(p, n);
    (n < width(p) && shr(&shifted, n, signed) == p).then_some(shifted)
  }

  /// `p` shifted right by `n`, as [`shr`] shifts it, if `n` is below its width and shifting the
  /// result back gives `p`; or `None`.
  pub(crate) fn shr_exact(p: &[bool], n: u32, signed: bool) -> Option<Vec<bool>> {
    let shifted = shr(p, n, signed);
    (n < width(p) && shl(&shifted, n) == p).then_some(shifted)
  }

  /// `p` with the bit at `index` set to `value`, or `None` from its width up.
  pub(crate) fn with_bit(p: &[bool], index: u32, value: bool) -> Option<Vec<bool>> {
    (index < width(p)).then(|| {
      let mut changed = p.to_vec();
      changed[index as usize] = value;
      changed
    })
  }

  /// The patterns of `width` bits that the tier of Miri checks: zero, every bit, the top bit, the
  /// top two bits, alternating bits from bit 0, and bit 0 of each limb of `limb_bits` bits.
  pub(crate) fn few_patterns(width: u32, limb_bits: u32) -> Vec<Vec<bool>> {
    let mut patterns = Vec::from([
      pattern(width, |_| false),
      pattern(width, |_| true),
      pattern(width, |i| i + 1 == width),
      pattern(width, |i| i + 2 >= width),
      pattern(width, |i| i % 2 == 0),
      pattern(width, |i| i % limb_bits == 0),
    ]);
    patterns.sort_unstable();
    patterns.dedup();
    patterns
  }
}
