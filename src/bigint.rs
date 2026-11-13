//! Helpers for the unit tests that compare with `num-bigint`: conversions between limb arrays and
//! big integers, the limits, powers of ten and bit patterns that they test, rounding by distance,
//! and fewer samples, exponents and bit indexes under Miri. A limb is any unsigned integer type
//! from `u8` to `u128`; the functions without a limb type parameter take `u64` limbs.
//!
//! The tests of the bit operations compare with no big integer: [`model`] computes their results
//! one bit at a time, and the tiers of [`tier`] size every sweep by the build. The tests of the
//! conversions with the floats compare with the floats of std and the bit model, through
//! [`floats`].

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
pub(crate) const MODES: [RoundingMode; 8] = [
  RoundingMode::Up,
  RoundingMode::Down,
  RoundingMode::Ceiling,
  RoundingMode::Floor,
  RoundingMode::HalfUp,
  RoundingMode::HalfDown,
  RoundingMode::HalfEven,
  RoundingMode::Up05,
];

/// `value / divisor` rounded to an integer with `mode`, for `divisor > 0`.
///
/// Picks between the integers below and above the exact quotient by their distances to it, and
/// breaks a tie by direction or parity. `Up05` has no distance: it reads the last digit of the
/// integer toward zero. The crate instead compares the remainder of a division toward zero with its
/// distance to the divisor.
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
    RoundingMode::Up05 if (&toward_zero % 5u8).sign() == Sign::NoSign => away_from_zero,
    RoundingMode::Up05 => toward_zero,
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
/// reads, one digit at a time: an empty text is `Empty`, a lone sign `InvalidDigit`, `-` a sign
/// only for a signed integer, and at each digit the product by `radix`, then the digit, whose
/// absence is `InvalidDigit`, then the overflow of the product or the sum out of the range of
/// `bits` bits.
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

/// The slices of bytes, least significant first, at which the readers of bytes are checked for
/// `p`, the pattern of a value read in two's complement if `signed`: at each length of no bytes,
/// one byte, and from one less than the bytes of its width, `ceil(width / 8)`, to two more, the
/// bytes of `p` extended or cut to the length, and those bytes with the most significant one
/// replaced by `0x00`, `0xff`, `0x01` or `0x80`, which keep or break the width and set or clear the
/// sign of the slice. Under Miri, the lengths of no bytes, of the width and one more, and `0x80`
/// alone.
pub(crate) fn byte_slices(p: &[bool], signed: bool) -> Vec<Vec<u8>> {
  let bytes = model::width(p).div_ceil(8) as usize;
  let (mut lengths, highs): (Vec<usize>, &[u8]) = if tier().is_miri() {
    (vec![0, bytes, bytes + 1], &[0x80])
  } else {
    (
      vec![0, 1, bytes.saturating_sub(1), bytes, bytes + 1, bytes + 2],
      &[0x00, 0xff, 0x01, 0x80],
    )
  };
  lengths.sort_unstable();
  lengths.dedup();
  let mut slices = Vec::new();
  for len in lengths {
    let natural = model::le_bytes(p, len, signed);
    if len > 0 {
      for &high in highs {
        let mut slice = natural.clone();
        slice[len - 1] = high;
        slices.push(slice);
      }
    }
    slices.push(natural);
  }
  slices.sort_unstable();
  slices.dedup();
  slices
}

/// The slices of limbs of type `L`, least significant first, at which the readers of limbs into
/// `n` limbs are checked for `p`, the pattern of a value read in two's complement if `signed`: at
/// each length of no limbs, one, and from one less than `n` to two more, the limbs of `p` extended
/// or cut to the length, and those limbs with the most significant one replaced by zero, every
/// bit, one or the top bit alone, which keep or break the width and set or clear the sign of the
/// slice. Under Miri, the lengths of no limbs, `n` and one more, and the top bit alone.
pub(crate) fn limb_slices<L>(p: &[bool], signed: bool, n: usize) -> Vec<Vec<L>>
where
  L: Limb + Ord + Shl<u32, Output = L>,
{
  let top = L::ONE << (L::BITS - 1);
  let (mut lengths, highs) = if tier().is_miri() {
    (vec![0, n, n + 1], vec![top])
  } else {
    (
      vec![0, 1, n.saturating_sub(1), n, n + 1, n + 2],
      vec![L::ZERO, L::MAX, L::ONE, top],
    )
  };
  lengths.sort_unstable();
  lengths.dedup();
  let size = (L::BITS / 8) as usize;
  let mut slices = Vec::new();
  for len in lengths {
    let natural: Vec<L> = model::le_bytes(p, size * len, signed)
      .chunks_exact(size)
      .map(L::from_le_slice)
      .collect();
    if len > 0 {
      for &high in &highs {
        let mut slice = natural.clone();
        slice[len - 1] = high;
        slices.push(slice);
      }
    }
    slices.push(natural);
  }
  slices.sort_unstable();
  slices.dedup();
  slices
}

/// The bytes of `limbs`, least significant first, which hold the integer of the limbs.
pub(crate) fn bytes_of_limbs<L: Limb>(limbs: &[L]) -> Vec<u8> {
  let mut bytes = Vec::new();
  for &limb in limbs {
    limb.push_le_bytes(&mut bytes);
  }
  bytes
}

/// The bit operations on patterns of bits, bit 0 first, as wide as the integers that hold them:
/// the model that the tests of the bit operations compare with. Each operation is written from its
/// definition, one bit at a time, and reads a pattern as unsigned, or in two's complement where it
/// takes `signed`.
pub(crate) mod model {
  use core::cmp::Ordering;
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

  /// The digits of `p`, read unsigned, in the radix `2^shift`, binary for 1, octal for 3 and
  /// hexadecimal for 4, in uppercase if `upper`: each digit from `shift` bits of the pattern, from
  /// the digit of the highest one down, and `0` for zero.
  pub(crate) fn radix_text(p: &[bool], shift: u32, upper: bool) -> String {
    let len = highest_one(p).map_or(0, |index| index + 1);
    (0..len.div_ceil(shift).max(1))
      .rev()
      .map(|k| {
        let digit = (0..shift).fold(0, |digit, i| digit | u32::from(bit(p, k * shift + i)) << i);
        let digit = char::from_digit(digit, 1 << shift).expect("a digit of the radix");
        if upper {
          digit.to_ascii_uppercase()
        } else {
          digit
        }
      })
      .collect()
  }

  /// The sign of `p` in two's complement: its top bit, or `false` for zero bits.
  pub(crate) fn sign(p: &[bool]) -> bool {
    p.last().copied().unwrap_or(false)
  }

  /// The order of the integers of `p` and `q`, each read in two's complement if its flag is set
  /// and unsigned otherwise: by their signs, then by their bits from the top down, each extended as
  /// it is read to the width of the wider.
  pub(crate) fn cmp(p: &[bool], p_signed: bool, q: &[bool], q_signed: bool) -> Ordering {
    let (p_fill, q_fill) = (p_signed && sign(p), q_signed && sign(q));
    if p_fill != q_fill {
      // The negative one is the smaller.
      return q_fill.cmp(&p_fill);
    }
    // Of the same sign, extended by the same bit, the integers compare as their bits do.
    for i in (0..p.len().max(q.len())).rev() {
      let (a, b) = (
        p.get(i).copied().unwrap_or(p_fill),
        q.get(i).copied().unwrap_or(q_fill),
      );
      if a != b {
        return a.cmp(&b);
      }
    }
    Ordering::Equal
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

  /// The `len` bytes of `p`, least significant first: its bits extended by zeros, or if `signed`
  /// by copies of its sign, or cut, to `8 * len` bits.
  pub(crate) fn le_bytes(p: &[bool], len: usize, signed: bool) -> Vec<u8> {
    let fill = signed && sign(p);
    (0..len)
      .map(|k| {
        (0..8).fold(0, |byte, j| {
          byte | u8::from(p.get(8 * k + j).copied().unwrap_or(fill)) << j
        })
      })
      .collect()
  }

  /// The pattern of the integer in `bytes`, least significant byte first: its `8 * bytes.len()`
  /// bits.
  pub(crate) fn of_le_bytes(bytes: &[u8]) -> Vec<bool> {
    bytes
      .iter()
      .flat_map(|&byte| (0..8).map(move |j| (byte >> j) & 1 == 1))
      .collect()
  }

  /// The integer of `p`, read in two's complement if `signed` and unsigned otherwise, as a pattern
  /// of `bits` bits, or `None` if `bits` bits do not hold it: if its low `bits` bits, extended as
  /// `p` is, do not give `p` back. Zero bits hold only zero.
  pub(crate) fn fit(p: &[bool], bits: u32, signed: bool) -> Option<Vec<bool>> {
    let wide = extend(p, bits.max(width(p)), signed);
    let cut = wide[..bits as usize].to_vec();
    (extend(&cut, width(&wide), signed) == wide).then_some(cut)
  }

  /// The integer of `p`, read in two's complement if `signed` and unsigned otherwise, modulo
  /// `2^bits`: the low `bits` bits of `p` extended as it is, which, read with the same sign, are
  /// within the range of `bits` bits.
  pub(crate) fn wrap(p: &[bool], bits: u32, signed: bool) -> Vec<bool> {
    extend(p, bits.max(width(p)), signed)[..bits as usize].to_vec()
  }

  /// The bytes of `p`, a pattern of whole bytes, in reverse order.
  pub(crate) fn swap_bytes(p: &[bool]) -> Vec<bool> {
    p.chunks(8).rev().flatten().copied().collect()
  }

  /// The 64-bit word of `p` at `index`, its bits from `64 * index` to `64 * index + 63`, extended
  /// beyond its width by zeros, or if `signed` by copies of its sign.
  pub(crate) fn word(p: &[bool], index: usize, signed: bool) -> u64 {
    let fill = signed && sign(p);
    (0..64).fold(0, |word, j| {
      let bit = p.get(64 * index + j).copied().unwrap_or(fill);
      word | u64::from(bit) << j
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

/// The models of the conversions between the integers and the floats, which compute with the floats
/// of std as the code under test never does: the rounding functions of std for each rounding mode,
/// the powers of two that bound the widths, the integer that a float holds as a bit pattern, the
/// float nearest to a bit pattern of any width, what a layer gives for a float rounded to an
/// integer, and the floats and the patterns at which the conversions are checked.
pub(crate) mod floats {
  use core::fmt::Debug;
  use std::{vec, vec::Vec};

  use super::model;
  use crate::{
    FromFloatError, RoundingMode,
    int::float::{FloatFormat, FloatParts, Rounded, float_parts_f32, float_parts_f64},
  };

  /// A float of the tests, `f64` or `f32`: what the models read of it, and the conversions of std
  /// that they compare with.
  pub(crate) trait TestFloat: Copy + Debug + PartialEq {
    /// The format of the float.
    const FORMAT: FloatFormat;

    /// The exponent of the lowest bit of a subnormal float, that of the smallest positive float.
    const MIN_EXPONENT: i32;

    /// NaN.
    const NAN: Self;

    /// The float nearest to `value`, which is `value` itself where the float holds it.
    fn narrow(value: f64) -> Self;

    /// The value of the float as an `f64`, which holds every `f32` exactly.
    fn wide(self) -> f64;

    /// The bits of the float.
    fn to_raw(self) -> u64;

    /// The float whose bits are the low bits of `bits`.
    fn from_raw(bits: u64) -> Self;

    /// The parts of the float, as the code under test reads them.
    fn parts(self) -> Result<FloatParts, FromFloatError>;

    /// The float nearest to `value`, ties to even, as `as` converts it.
    fn of_u64(value: u64) -> Self;

    /// The float nearest to `value`, ties to even, or the infinity above the range of the float,
    /// as `as` converts it.
    fn of_u128(value: u128) -> Self;

    /// The float nearest to `value`, ties to even, as `as` converts it.
    fn of_i128(value: i128) -> Self;

    /// `self * 2^exponent`, which is exact unless it passes the largest float, and an infinity
    /// then.
    fn scale(self, exponent: u32) -> Self;

    /// `-self`.
    fn negate(self) -> Self;
  }

  impl TestFloat for f64 {
    const FORMAT: FloatFormat = FloatFormat::F64;
    const MIN_EXPONENT: i32 = -1074;
    const NAN: Self = f64::NAN;

    fn narrow(value: f64) -> Self {
      value
    }

    fn wide(self) -> f64 {
      self
    }

    fn to_raw(self) -> u64 {
      self.to_bits()
    }

    fn from_raw(bits: u64) -> Self {
      f64::from_bits(bits)
    }

    fn parts(self) -> Result<FloatParts, FromFloatError> {
      float_parts_f64(self)
    }

    fn of_u64(value: u64) -> Self {
      value as f64
    }

    fn of_u128(value: u128) -> Self {
      value as f64
    }

    fn of_i128(value: i128) -> Self {
      value as f64
    }

    fn scale(self, exponent: u32) -> Self {
      self * pow2(i32::try_from(exponent).unwrap_or(i32::MAX))
    }

    fn negate(self) -> Self {
      -self
    }
  }

  impl TestFloat for f32 {
    const FORMAT: FloatFormat = FloatFormat::F32;
    const MIN_EXPONENT: i32 = -149;
    const NAN: Self = f32::NAN;

    fn narrow(value: f64) -> Self {
      value as f32
    }

    fn wide(self) -> f64 {
      f64::from(self)
    }

    fn to_raw(self) -> u64 {
      u64::from(self.to_bits())
    }

    fn from_raw(bits: u64) -> Self {
      f32::from_bits(bits as u32)
    }

    fn parts(self) -> Result<FloatParts, FromFloatError> {
      float_parts_f32(self)
    }

    fn of_u64(value: u64) -> Self {
      value as f32
    }

    fn of_u128(value: u128) -> Self {
      value as f32
    }

    fn of_i128(value: i128) -> Self {
      value as f32
    }

    fn scale(self, exponent: u32) -> Self {
      self * pow2_f32(i32::try_from(exponent).unwrap_or(i32::MAX))
    }

    fn negate(self) -> Self {
      -self
    }
  }

  /// `2^exponent` as an `f64`, from its bits: zero below `2^-1074`, the smallest positive float,
  /// and an infinity from `2^1024` up.
  pub(crate) fn pow2(exponent: i32) -> f64 {
    match exponent {
      i32::MIN..=-1075 => 0.0,
      -1074..=-1023 => f64::from_bits(1 << (exponent + 1074)),
      -1022..=1023 => f64::from_bits(((exponent + 1023) as u64) << 52),
      _ => f64::INFINITY,
    }
  }

  /// `2^exponent` as an `f32`, from its bits: zero below `2^-149`, the smallest positive float,
  /// and an infinity from `2^128` up.
  pub(crate) fn pow2_f32(exponent: i32) -> f32 {
    match exponent {
      i32::MIN..=-150 => 0.0,
      -149..=-127 => f32::from_bits(1 << (exponent + 149)),
      -126..=127 => f32::from_bits(((exponent + 127) as u32) << 23),
      _ => f32::INFINITY,
    }
  }

  /// `value`, a finite float, rounded to an integer with `mode` by the functions of std: `trunc`
  /// for `Down`, `ceil` for `Ceiling`, `floor` for `Floor`, `round` for `HalfUp`, `round_ties_even`
  /// for `HalfEven`, the magnitude rounded up for `Up`, which std has no function for, for
  /// `HalfDown` `round` but at a tie, which `trunc` rounds toward zero, and for `Up05` `trunc`,
  /// stepped away from zero where the value is not an integer and its truncation ends in 0 or 5.
  pub(crate) fn round(value: f64, mode: RoundingMode) -> f64 {
    match mode {
      RoundingMode::Up => value.abs().ceil().copysign(value),
      RoundingMode::Down => value.trunc(),
      RoundingMode::Ceiling => value.ceil(),
      RoundingMode::Floor => value.floor(),
      RoundingMode::HalfUp => value.round(),
      // The fraction of a float is exact, so a tie compares as one half.
      RoundingMode::HalfDown if (value - value.trunc()).abs() == 0.5 => value.trunc(),
      RoundingMode::HalfDown => value.round(),
      RoundingMode::HalfEven => value.round_ties_even(),
      // `%` of floats is exact, and a truncation of 2^53 or more is the value itself, so the step
      // is exact too.
      RoundingMode::Up05 => {
        let truncated = value.trunc();
        if truncated != value && truncated % 5.0 == 0.0 {
          truncated + 1.0f64.copysign(value)
        } else {
          truncated
        }
      }
    }
  }

  /// Returns `true` if `value`, an integer, is in the range of `bits` bits, in two's complement if
  /// `signed` and unsigned otherwise: from zero to `2^bits - 1`, or from `-2^(bits - 1)` to
  /// `2^(bits - 1) - 1`. An integer is below the power of two above the range exactly when it is at
  /// most the largest value, which a float may not hold.
  pub(crate) fn in_range(value: f64, bits: u32, signed: bool) -> bool {
    let bits = bits as i32;
    match (bits, signed) {
      (0, _) => value == 0.0,
      (_, false) => 0.0 <= value && value < pow2(bits),
      (_, true) => -pow2(bits - 1) <= value && value < pow2(bits - 1),
    }
  }

  /// The pattern of `width` bits of `value`, an integer in the range of `width` bits, in two's
  /// complement for a negative value: its significand, read from its bits, moved to its exponent,
  /// which leaves no bit below the point for an integer.
  pub(crate) fn pattern_of(value: f64, width: u32) -> Vec<bool> {
    let bits = value.abs().to_bits();
    let (biased, fraction) = ((bits >> 52) as i32, bits & ((1 << 52) - 1));
    // A subnormal float, and zero, has no implicit bit and the exponent of the smallest normal one.
    let (significand, exponent) = match biased {
      0 => (fraction, -1074),
      _ => (fraction | 1 << 52, biased - 1075),
    };
    // One bit more than the width, for the magnitude of the smallest value of two's complement.
    let magnitude = model::pattern(width + 1, |i| {
      let index = i64::from(i) - i64::from(exponent);
      (0..64).contains(&index) && (significand >> index) & 1 == 1
    });
    let pattern = if value < 0.0 {
      negate(&magnitude)
    } else {
      magnitude
    };
    pattern[..width as usize].to_vec()
  }

  /// The float of `F` nearest to the integer of `p`, read in two's complement if `signed` and
  /// unsigned otherwise, ties to even, or the infinity of its sign beyond the range of `F`, and
  /// `+0.0` for zero: what `as` gives for at most 128 bits, and above them the top 64 bits of the
  /// magnitude rounded to odd, with their lowest bit set if a bit below them is, converted as `as`
  /// converts a `u64` and scaled by the power of two of the bits below them. A float has at most
  /// 53 bits of significand, so the rounding to odd in 64 bits leaves the second rounding exact.
  pub(crate) fn float_of_pattern<F: TestFloat>(p: &[bool], signed: bool) -> F {
    let width = model::width(p);
    if width <= 128 {
      return if signed {
        F::of_i128(model::to_i128(p))
      } else {
        F::of_u128(model::to_u128(p))
      };
    }
    let negative = signed && model::sign(p);
    let magnitude = if negative {
      negate(&model::extend(p, width + 1, true))
    } else {
      p.to_vec()
    };
    let length = match model::highest_one(&magnitude) {
      Some(highest) => highest + 1,
      None => return F::of_u64(0),
    };
    let float = if length <= 64 {
      F::of_u64(model::to_u128(&magnitude[..64]) as u64)
    } else {
      let dropped = length - 64;
      let top = model::to_u128(&magnitude[dropped as usize..length as usize]) as u64;
      let sticky = magnitude[..dropped as usize].iter().any(|&bit| bit);
      F::of_u64(top | u64::from(sticky)).scale(dropped)
    };
    if negative { float.negate() } else { float }
  }

  /// `p + 1` modulo `2^width`.
  fn increment(p: &[bool]) -> Vec<bool> {
    let mut carry = true;
    p.iter()
      .map(|&bit| {
        let sum = bit != carry;
        carry = bit && carry;
        sum
      })
      .collect()
  }

  /// `p - 1` modulo `2^width`.
  fn decrement(p: &[bool]) -> Vec<bool> {
    let mut borrow = true;
    p.iter()
      .map(|&bit| {
        let difference = bit != borrow;
        borrow = !bit && borrow;
        difference
      })
      .collect()
  }

  /// The two's complement of `p`, `-p` modulo `2^width`: every bit flipped, plus one.
  pub(crate) fn negate(p: &[bool]) -> Vec<bool> {
    increment(&model::not(p))
  }

  /// What a layer that places `rounded`, a float rounded to an integer, at `bits` bits gives, from
  /// the bit model: the pattern of the integer, in two's complement if `signed` and unsigned
  /// otherwise, or the error of an integer outside the range, `NegOverflow` below it, as every
  /// negative integer is for an unsigned one, and `PosOverflow` above it.
  pub(crate) fn place(
    rounded: Rounded,
    bits: u32,
    signed: bool,
  ) -> Result<Vec<bool>, FromFloatError> {
    // The magnitude in enough bits to hold it with a sign bit above it.
    let width = 64 + rounded.shift + 1;
    let significand = model::of_u128(u128::from(rounded.significand), 64);
    let magnitude = model::shl(&model::extend(&significand, width, false), rounded.shift);
    let negative = rounded.negative && rounded.significand != 0;
    let fitted = match (negative, signed) {
      (true, false) => None,
      (true, true) => model::fit(&negate(&magnitude), bits, true),
      (false, _) => model::fit(&magnitude, bits, signed),
    };
    match fitted {
      Some(pattern) => Ok(pattern),
      None if negative => Err(FromFloatError::NegOverflow),
      None => Err(FromFloatError::PosOverflow),
    }
  }

  /// The integers rounded from floats at which a layer is checked for a width of `bits` bits:
  /// significands of zero, one, a few small values, every bit, `2^52`, `2^53` and alternating
  /// bits, shifted to the low bits, to those next to the width, below and above it, and to those
  /// of large floats, each of both signs; under Miri fewer significands and shifts.
  pub(crate) fn rounded_samples(bits: u32) -> Vec<Rounded> {
    let miri = super::tier().is_miri();
    let significands: &[u64] = if miri {
      &[0, 1, 3, (1 << 53) - 1, 1 << 53]
    } else {
      &[
        0,
        1,
        2,
        3,
        5,
        (1 << 53) - 1,
        1 << 52,
        1 << 53,
        0x15_5555_5555_5555,
      ]
    };
    let (mut shifts, offsets): (Vec<u32>, &[u32]) = if miri {
      (vec![0], &[53, 1])
    } else {
      (vec![0, 1, 2, 300, 971], &[55, 54, 53, 2, 1])
    };
    for &offset in offsets {
      shifts.extend(bits.checked_sub(offset));
    }
    shifts.extend([bits, bits + 1]);
    shifts.sort_unstable();
    shifts.dedup();
    let mut samples = Vec::new();
    for &significand in significands {
      for &shift in &shifts {
        for negative in [false, true] {
          samples.push(Rounded {
            negative,
            significand,
            shift,
          });
        }
      }
    }
    samples
  }

  /// A xorshift generator of 64-bit words, seeded so that a failure replays.
  pub(crate) struct Words(u64);

  impl Words {
    /// The generator of `seed`.
    pub(crate) fn new(seed: u64) -> Self {
      Self(0x9e37_79b9_7f4a_7c15 ^ seed)
    }

    /// The next word.
    pub(crate) fn next(&mut self) -> u64 {
      self.0 ^= self.0 >> 12;
      self.0 ^= self.0 << 25;
      self.0 ^= self.0 >> 27;
      self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
  }

  /// The floats at which the conversions into integers of the widths of `bits` are checked: NaN,
  /// zero, the infinities, the largest float, the smallest and the largest subnormal float and the
  /// smallest normal one, the values below one half `0.0005` and `0.0001`, the halves and the
  /// quarters near zero, `2^k` for `k` each width and one bit less, with the floats next to it and
  /// the integers and the halves next to it, which the float may round to it, and `random` floats
  /// of random magnitudes, from below one half to beyond the widths; each also negated.
  pub(crate) fn samples<F: TestFloat>(bits: &[u32], random: usize) -> Vec<F> {
    let mut seeds: Vec<f64> = vec![
      0.0,
      f64::INFINITY,
      f64::MAX,
      0.0005,
      0.0001,
      0.25,
      0.5,
      0.75,
      1.0,
      1.25,
      1.5,
      2.5,
      3.5,
      6.5,
    ];
    let mut floats: Vec<F> = Vec::new();
    for &width in bits {
      for exponent in [width as i32 - 1, width as i32] {
        let power = pow2(exponent);
        seeds.extend([power, power - 1.0, power + 1.0, power - 0.5, power + 0.5]);
        let raw = F::narrow(power).to_raw();
        floats.extend([F::from_raw(raw - 1), F::from_raw(raw + 1)]);
      }
    }
    let widest = bits.iter().copied().max().unwrap_or(0);
    let mut words = Words::new(u64::from(widest));
    for _ in 0..random {
      // A significand of 53 random bits whose top bit is at a random exponent.
      let exponent = (words.next() % u64::from(widest + 72)) as i32 - 64;
      seeds.push((words.next() >> 11) as f64 * pow2(exponent - 52));
    }
    floats.extend(seeds.into_iter().map(F::narrow));
    // The smallest positive float, the largest subnormal float, the smallest normal float and the
    // largest float.
    let fraction = (1 << (F::FORMAT.precision() - 1)) - 1;
    let infinity = F::narrow(f64::INFINITY).to_raw();
    floats.extend([
      F::from_raw(1),
      F::from_raw(fraction),
      F::from_raw(fraction + 1),
      F::from_raw(infinity - 1),
    ]);
    let negated: Vec<F> = floats.iter().map(|&x| x.negate()).collect();
    floats.extend(negated);
    floats.push(F::NAN);
    floats.sort_by_key(|&x| x.to_raw());
    floats.dedup_by_key(|&mut x| x.to_raw());
    floats
  }

  /// The patterns of `width` bits whose magnitude, read unsigned, a float of 24 or of 53 bits of
  /// significand rounds at a tie, and those next to them: for each length from one bit more than
  /// the significand, among those next to it, up to 64, 128 and the width, and for top bits of a
  /// one and zeros, of a one, zeros and a one, of ones and of alternating bits, the top bits
  /// followed by the guard bit alone, a tie, and the patterns one below and one above it.
  pub(crate) fn tie_patterns(width: u32) -> Vec<Vec<bool>> {
    let mut patterns = Vec::new();
    for precision in [24, 53] {
      let tops = [
        model::pattern(precision, |i| i + 1 == precision),
        model::pattern(precision, |i| i == 0 || i + 1 == precision),
        model::pattern(precision, |_| true),
        model::pattern(precision, |i| i % 2 == 0 || i + 1 == precision),
      ];
      let lengths = [
        precision + 1,
        precision + 2,
        precision + 11,
        64,
        65,
        128,
        129,
        width.saturating_sub(1),
        width,
      ];
      for length in lengths {
        if length <= precision || length > width {
          continue;
        }
        // The guard bit, below the top `precision` bits of the length.
        let guard = length - precision - 1;
        for top in &tops {
          let tie = model::pattern(width, |i| {
            i == guard || (guard < i && i < length && top[(i - guard - 1) as usize])
          });
          patterns.extend([decrement(&tie), increment(&tie), tie]);
        }
      }
    }
    patterns.sort_unstable();
    patterns.dedup();
    patterns
  }

  /// `count` patterns of `width` bits of random bits, from a generator seeded by `seed`.
  pub(crate) fn random_patterns(width: u32, count: usize, seed: u64) -> Vec<Vec<bool>> {
    let mut words = Words::new(seed);
    (0..count)
      .map(|_| {
        let mut word = 0;
        model::pattern(width, |i| {
          if i % 64 == 0 {
            word = words.next();
          }
          (word >> (i % 64)) & 1 == 1
        })
      })
      .collect()
  }
}
