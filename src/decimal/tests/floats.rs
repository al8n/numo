//! The conversions of `Decimal` with `f64` and `f32`. The core of the float module is checked at
//! every fraction count from 0 to 154: the conversion to a float against the standard library's
//! parsing of the text of the decimal, and the reading of the binary value of a float against the
//! exact rational in `num-bigint`. The decimals between adjacent floats, built in `num-bigint` from
//! the parts of the lower float, are checked against the rule of ties to even, over six ranges of
//! `f32` and random `f64`s, over which an `f32` is also read as the `f64` of the same value; then
//! the families of values where the rounding changes its case: the boundaries of the binades, the
//! infinity of `f32`, its subnormal floats and its zeros, the tiny floats and the edge of the eight
//! limbs; and the inherent `const fn`s of every storage, against the parsed text of the decimal
//! and against the exact rational in every mode, those of a type without fraction digits against
//! the integers' own conversions, the special values and the constant contexts. Every float is
//! compared by its bits, so that `-0.0` and the infinities count.
//!
//! The checks are generic over the float, through `TestFloat` and the local trait [`Float`], and
//! call the inherent functions of a decimal type through the pointers of [`Conversions`], which
//! [`conversions!`] takes from the type. Each type checks in a function of its own, never inlined,
//! whose frame holds the values of that type alone: an optimized build would inline a function
//! that has one call site, and the frame of the test would then hold the values of every type. The
//! tier of the build sizes every sweep, and every random value comes from a generator with a seed
//! written here, so that a failure replays.

use core::{any::type_name, f64::consts::LOG2_10, num::ParseFloatError, str::FromStr};
use std::{
  collections::BTreeSet,
  format,
  string::{String, ToString},
  vec::Vec,
};

use num_bigint::{BigInt, Sign};
use typenum::{
  U0, U1, U2, U4, U5, U8, U9, U10, U17, U18, U19, U25, U26, U28, U38, U39, U40, U54, U58, U76, U77,
  U78, U96, U97, U116, U135, U149, U153, U154,
};

use crate::{
  Decimal, FixedDecimal, FromFloatError, RoundingMode,
  bigint::{
    self, MODES, Tier,
    floats::{self, TestFloat, Words, pow2, pow2_f32},
    tier,
  },
  decimal::float,
  i192, i256, i320, i384, i448, i512,
  int::float::FloatParts,
  u192, u256, u320, u384, u448, u512,
};

// One type of each storage, most of them at the digit limit of the storage, among them types
// without integral digits and types without fraction digits, and the type with the most fraction
// digits.
type Di8 = Decimal<U1, U1, i8>;
type Di16 = Decimal<U0, U4, i16>;
type Di32 = Decimal<U5, U4, i32>;
type Di64 = Decimal<U10, U8, i64>;
type Di128 = Decimal<U19, U19, i128>;
type Di192 = Decimal<U39, U18, i192>;
type Di256 = Decimal<U58, U18, i256>;
type Di320 = Decimal<U96, U0, i320>;
type Di384 = Decimal<U97, U18, i384>;
type Di448 = Decimal<U116, U18, i448>;
type Di512 = Decimal<U135, U18, i512>;
type Du8 = Decimal<U1, U1, u8>;
type Du16 = Decimal<U2, U2, u16>;
type Du32 = Decimal<U0, U9, u32>;
type Du64 = Decimal<U10, U9, u64>;
type Du128 = Decimal<U19, U19, u128>;
type Du192 = Decimal<U39, U18, u192>;
type Du256 = Decimal<U77, U0, u256>;
type Du320 = Decimal<U78, U18, u320>;
type Du384 = Decimal<U97, U18, u384>;
type Du448 = Decimal<U116, U18, u448>;
type Du512 = Decimal<U154, U0, u512>;
type D0x154 = Decimal<U0, U154, u512>;

// The types that hold the midpoints of adjacent `f32`s exactly: below one, down to the odd
// multiples of `2^-150` between the subnormal floats; around one; from one up; and at the top of
// the range, where the midpoints are integers.
type BelowOne = Decimal<U0, U153, i512>;
type AroundOne = Decimal<U1, U25, i128>;
type AboveOne = Decimal<U39, U25, i256>;
type Top = Decimal<U39, U0, i192>;

/// A type that holds the midpoint of two adjacent `f64`s whose lowest bit is at `2^-70` or above,
/// up to `2^251`.
type Wide = Decimal<U77, U76, i512>;

/// What the checks need of a float beyond [`TestFloat`]: the standard library's parsing of a text,
/// and the conversions of a decimal type with this format through [`Conversions`].
trait Float: TestFloat + FromStr<Err = ParseFloatError> {
  /// The conversion of `value` to this format, through the inherent function of its type.
  fn of_decimal<T>(conversions: &Conversions<T>, value: &T) -> Self;

  /// The reading of the binary value of `value` into `T` with `mode`, through the inherent
  /// function of `T`.
  fn round_into<T>(
    conversions: &Conversions<T>,
    value: Self,
    mode: RoundingMode,
  ) -> Result<T, FromFloatError>;
}

impl Float for f64 {
  fn of_decimal<T>(conversions: &Conversions<T>, value: &T) -> Self {
    (conversions.to_f64)(value)
  }

  fn round_into<T>(
    conversions: &Conversions<T>,
    value: Self,
    mode: RoundingMode,
  ) -> Result<T, FromFloatError> {
    (conversions.round_from_f64)(value, mode)
  }
}

impl Float for f32 {
  fn of_decimal<T>(conversions: &Conversions<T>, value: &T) -> Self {
    (conversions.to_f32)(value)
  }

  fn round_into<T>(
    conversions: &Conversions<T>,
    value: Self,
    mode: RoundingMode,
  ) -> Result<T, FromFloatError> {
    (conversions.round_from_f32)(value, mode)
  }
}

/// The inherent conversions of the decimal type `T` with the floats, which a check generic over the
/// float calls through these pointers.
struct Conversions<T> {
  to_f64: fn(&T) -> f64,
  to_f32: fn(&T) -> f32,
  round_from_f64: fn(f64, RoundingMode) -> Result<T, FromFloatError>,
  round_from_f32: fn(f32, RoundingMode) -> Result<T, FromFloatError>,
}

/// The [`Conversions`] of the decimal type `$ty`: its inherent `const fn`s.
macro_rules! conversions {
  ($ty:ty) => {
    Conversions::<$ty> {
      to_f64: <$ty>::to_f64,
      to_f32: <$ty>::to_f32,
      round_from_f64: <$ty>::round_from_f64,
      round_from_f32: <$ty>::round_from_f32,
    }
  };
}

/// Calls the check `$check` with the [`Conversions`] of each type of the list.
macro_rules! for_each_type {
  ($check:ident: $($ty:ty),+ $(,)?) => {$(
    $check::<$ty>(&conversions!($ty));
  )+};
}

/// The value of `T` that `text` names, which must be in the range of `T`.
fn value<T: FixedDecimal>(text: &str) -> T {
  text.parse().unwrap_or_else(|err| panic!("{text:?}: {err}"))
}

/// The value of `T` whose coefficient is `coefficient`, which must be in the range of `T`.
fn of_coefficient<T: FixedDecimal>(coefficient: &BigInt) -> T {
  value(&format!("{coefficient}e-{}", T::FRACTION))
}

/// The coefficient of `2^exponent` in a type with `fraction` fraction digits,
/// `5^fraction * 2^(fraction + exponent)`, which is an integer for `fraction + exponent >= 0`.
fn power_of_two(exponent: i32, fraction: u32) -> BigInt {
  let shift = u32::try_from(fraction as i32 + exponent).expect("an integer coefficient");
  BigInt::from(5u8).pow(fraction) << shift
}

/// A random magnitude of exactly `digits` decimal digits.
fn random_magnitude(words: &mut Words, digits: u32) -> BigInt {
  let mut text = String::from(char::from(b'1' + (words.next() % 9) as u8));
  for _ in 1..digits {
    text.push(char::from(b'0' + (words.next() % 10) as u8));
  }
  text.parse().expect("digits")
}

/// The seed of a check of `T`: `base`, which the check writes, with the digit counts and the sign
/// of `T`, so that each type draws values of its own.
fn seed<T: FixedDecimal>(base: u64) -> u64 {
  base ^ (u64::from(T::DIGITS) << 32 | u64::from(T::FRACTION) << 8 | u64::from(T::SIGNED))
}

/// The fraction counts at which the core is checked: every count from 0 to 154; under Miri, 0,
/// 17 and 154.
fn fraction_counts() -> Vec<u32> {
  if tier().is_miri() {
    Vec::from([0, 17, 154])
  } else {
    (0..=154).collect()
  }
}

/// The magnitudes at which `float_bits` is checked at `fraction` fraction digits. In the Debug
/// tier: one; the largest value below one, `10^fraction - 1`, but at no fraction digits, where it
/// is zero; the largest value below ten, `10^(fraction + 1) - 1`, but at 154 fraction digits,
/// where it passes the range; the largest magnitude, `10^154 - 1`; and two random magnitudes of
/// random lengths. In the Full tier: one, `10^k - 1` for every `k` from 1 to 154, and a random
/// magnitude of every length from 1 to 154 digits. Under Miri, those of the Debug tier but the
/// random ones.
fn core_magnitudes(fraction: u32, words: &mut Words) -> Vec<BigInt> {
  let nines = |digits: u32| bigint::pow10(digits) - 1u8;
  let mut magnitudes = Vec::from([BigInt::from(1u8)]);
  if tier().is_full() {
    magnitudes.extend((1..=154).map(nines));
    magnitudes.extend((1..=154).map(|digits| random_magnitude(words, digits)));
    return magnitudes;
  }
  if fraction > 0 {
    magnitudes.push(nines(fraction));
  }
  if fraction < 154 {
    magnitudes.push(nines(fraction + 1));
  }
  magnitudes.push(nines(154));
  if tier().is_debug() {
    for _ in 0..2 {
      let digits = 1 + (words.next() % 154) as u32;
      magnitudes.push(random_magnitude(words, digits));
    }
  }
  magnitudes
}

/// Checks `float_bits` in the format of `F` at every fraction count against the float that the
/// standard library parses from the text `{magnitude}e-{fraction}`, compared by the bits, at each
/// magnitude of [`core_magnitudes`] and its negation; and zero apart, once with either sign, which
/// gives `+0.0`, while its text with a sign parses to `-0.0`.
#[inline(never)]
fn core_matches_the_parsed_text<F: Float>(base: u64) {
  let float_format = F::FORMAT;
  for negative in [false, true] {
    assert_eq!(
      float::float_bits(&[0u64; 8], 154, negative, float_format),
      0,
      "zero, negative {negative}, {float_format:?}"
    );
  }
  let mut words = Words::new(base);
  for fraction in fraction_counts() {
    for magnitude in core_magnitudes(fraction, &mut words) {
      let limbs = bigint::to_limbs::<8>(&magnitude, false).expect("below 2^512");
      for sign in ["", "-"] {
        let text = format!("{sign}{magnitude}e-{fraction}");
        let parsed: F = text.parse().expect("a float");
        assert_eq!(
          float::float_bits(&limbs, fraction, sign == "-", float_format),
          parsed.to_raw(),
          "{text}, {float_format:?}"
        );
      }
    }
  }
}

/// `float_bits`, the core of `to_f64` and `to_f32`, gives the bits of the float that the standard
/// library parses from the text of the decimal, at every fraction count and in both formats, for
/// the magnitudes that reach its bounds: one, the longest magnitude below one and below ten, and
/// the longest of all, which needs no shift.
#[test]
fn the_core_matches_the_parsed_text_at_every_fraction_count() {
  core_matches_the_parsed_text::<f64>(0x5eed_f064);
  core_matches_the_parsed_text::<f32>(0x5eed_f032);
}

/// Checks, for a finite float `x` and the float of the next bit pattern, `x⁺`, that the midpoint
/// of the two, an exact decimal of `T`, converts to the one of the two whose bit pattern is even,
/// that one unit of the last digit nearer to zero converts to `x`, and that one unit farther
/// converts to `x⁺`, which above the largest float is the infinity, whose pattern is even.
/// Requires `T` to hold the midpoint exactly.
fn check_tie<F: Float, T: FixedDecimal>(x: F, conversions: &Conversions<T>) {
  let parts = x.parts().expect("a finite float");
  let next = F::from_raw(x.to_raw() + 1);
  let tie = if x.to_raw() & 1 == 0 { x } else { next };
  // The midpoint is `(2 * significand + 1) * 2^(exponent - 1)`.
  let midpoint =
    BigInt::from(2 * parts.significand + 1) * power_of_two(parts.exponent - 1, T::FRACTION);
  for (offset, expected) in [(0i8, tie), (-1, x), (1, next)] {
    let magnitude = &midpoint + offset;
    let coefficient = if parts.negative {
      -magnitude
    } else {
      magnitude
    };
    let decimal: T = of_coefficient(&coefficient);
    assert_eq!(
      F::of_decimal(conversions, &decimal).to_raw(),
      expected.to_raw(),
      "the midpoint above {:#x} with {offset} units in {}",
      x.to_raw(),
      type_name::<T>()
    );
  }
}

/// Checks [`check_tie`] at each float of `floats` in `T`.
#[inline(never)]
fn ties_of<F: Float, T: FixedDecimal>(floats: &[F], conversions: &Conversions<T>) {
  for &x in floats {
    check_tie(x, conversions);
  }
}

/// The floats of six ranges of bit patterns of `f32`, grouped by the type that holds their
/// midpoints.
#[derive(Default)]
struct Ranges {
  /// The floats below one, which [`BelowOne`] holds.
  below_one: Vec<f32>,
  /// The floats around one, which [`AroundOne`] holds.
  around_one: Vec<f32>,
  /// The floats from one up but for the last range, which [`AboveOne`] holds.
  above_one: Vec<f32>,
  /// The largest floats, which [`Top`] holds.
  top: Vec<f32>,
}

/// The floats of `patterns`, with every sixteenth also with its sign bit set.
fn signed_floats(patterns: impl IntoIterator<Item = u32>) -> Vec<f32> {
  let mut floats = Vec::new();
  for (index, pattern) in patterns.into_iter().enumerate() {
    floats.push(f32::from_bits(pattern));
    if index % 16 == 0 {
      floats.push(f32::from_bits(pattern | 0x8000_0000));
    }
  }
  floats
}

/// The floats of six ranges of bit patterns of `f32`, where the rounding of a decimal to an `f32`
/// has its cases, each with every sixteenth float also with its sign bit set: `count` patterns
/// from zero up, the smallest subnormal floats; `count` below the smallest normal float and
/// `count` from it, where the subnormal floats carry into the normal ones; likewise around one,
/// where the spacing doubles; the last `count` up to the largest float, whose next pattern is the
/// infinity; a few fractions of every biased exponent from 1 to 254, zero, one, every bit and
/// random ones, and the alternating bits among many; and of every length of a subnormal
/// significand, from 1 to 23 bits, the smallest and the largest value and random ones, or every
/// value of a length that has no more. The Full tier takes 65 536 patterns a range, 256 fractions
/// an exponent and 256 values a length, and the Debug tier 256, 4 and 4; under Miri none.
fn ranges() -> Ranges {
  let (count, fractions, values): (u32, usize, u32) = match tier() {
    Tier::Full => (65_536, 256, 256),
    Tier::Debug => (256, 4, 4),
    Tier::Miri => return Ranges::default(),
  };
  let mut words = Words::new(0x5eed_0f32);
  let mut below_one = signed_floats(0..count);
  below_one.extend(signed_floats(0x0080_0000 - count..0x0080_0000 + count));
  let around_one = signed_floats(0x3f80_0000 - count..0x3f80_0000 + count);
  let top = signed_floats(0x7f80_0000 - count..0x7f80_0000);

  let mut patterns = Vec::new();
  for biased in 1..=254u32 {
    let mut chosen: Vec<u32> = Vec::from([0, 1, 0x7f_ffff]);
    if fractions > 4 {
      chosen.push(0x55_5555);
    }
    while chosen.len() < fractions {
      // 23 random bits.
      chosen.push((words.next() >> 41) as u32);
    }
    patterns.extend(chosen.into_iter().map(|fraction| biased << 23 | fraction));
  }
  let mut above_one = Vec::new();
  for x in signed_floats(patterns) {
    if (x.to_bits() >> 23) & 0xff < 127 {
      below_one.push(x);
    } else {
      above_one.push(x);
    }
  }

  let mut patterns = Vec::new();
  for length in 1..=23u32 {
    let (low, high) = (1u32 << (length - 1), (1u32 << length) - 1);
    let wanted = values.min(high - low + 1) as usize;
    let mut chosen = BTreeSet::from([low, high]);
    while chosen.len() < wanted {
      chosen.insert(low + (words.next() % u64::from(high - low + 1)) as u32);
    }
    patterns.extend(chosen);
  }
  below_one.extend(signed_floats(patterns));
  Ranges {
    below_one,
    around_one,
    above_one,
    top,
  }
}

/// Random `f64`s of either sign, with a significand of 53 random bits whose highest bit is at a
/// random exponent from -190 to 250: 200 000 in the Full tier and 2 000 in the Debug tier; under
/// Miri none.
fn random_f64s() -> Vec<f64> {
  let count = match tier() {
    Tier::Full => 200_000,
    Tier::Debug => 2_000,
    Tier::Miri => 0,
  };
  let mut words = Words::new(0x5eed_1f64);
  (0..count)
    .map(|_| {
      // The 52 bits below the implicit one are the top of the word, and the sign its lowest bit.
      let word = words.next();
      let biased = 1023 - 190 + words.next() % 441;
      f64::from_bits((word & 1) << 63 | biased << 52 | word >> 12)
    })
    .collect()
}

/// The decimals at the midpoints of adjacent floats and one unit of the last digit on either side
/// convert to the even float of the two, the lower and the upper one: over the six ranges of
/// `f32`, the first random `f64`s whose lowest bit is at `2^-70` or above, 500 in the Debug tier
/// and 24 000 in the Full tier, and the ties above one; and the powers of two, at the boundaries
/// of the binades, and one unit on either side, which convert to the power, but for an `f32`
/// beyond its largest float, where they give the infinity.
#[test]
fn decimals_between_adjacent_floats_round_to_the_nearer_and_ties_to_even() {
  let ranges = ranges();
  ties_of(&ranges.below_one, &conversions!(BelowOne));
  ties_of(&ranges.around_one, &conversions!(AroundOne));
  ties_of(&ranges.above_one, &conversions!(AboveOne));
  ties_of(&ranges.top, &conversions!(Top));
  let pairs = match tier() {
    Tier::Full => 24_000,
    Tier::Debug => 500,
    Tier::Miri => 0,
  };
  let floats: Vec<f64> = random_f64s()
    .into_iter()
    .filter(|x| x.parts().is_ok_and(|parts| parts.exponent >= -70))
    .take(pairs)
    .collect();
  assert_eq!(floats.len(), pairs);
  ties_of(&floats, &conversions!(Wide));

  for exponent in [-76, -64, -53, -24, -1, 0, 1, 24, 53, 64, 127, 128, 255] {
    let power = power_of_two(exponent, Wide::FRACTION);
    for offset in [-1i8, 0, 1] {
      let decimal: Wide = of_coefficient(&(&power + offset));
      assert_eq!(
        decimal.to_f64().to_bits(),
        pow2(exponent).to_bits(),
        "2^{exponent} with {offset} units"
      );
      // `pow2_f32` gives the infinity from `2^128` up.
      assert_eq!(
        decimal.to_f32().to_bits(),
        pow2_f32(exponent).to_bits(),
        "2^{exponent} with {offset} units"
      );
    }
  }

  // The tie of `1.0` and the next `f32`, `1 + 2^-24`, goes to `1.0`, and one unit above it to the
  // next `f32`, which it reaches by one rounding: through an `f64` it rounds to the tie first.
  let tie: Decimal<U1, U26> = value("1.000000059604644775390625");
  assert_eq!(tie.to_f32().to_bits(), 1.0f32.to_bits());
  let above: Decimal<U1, U26> = value("1.00000005960464477539062501");
  assert_eq!(above.to_f32().to_bits(), (1.0 + f32::EPSILON).to_bits());
  assert_eq!((above.to_f64() as f32).to_bits(), 1.0f32.to_bits());
  // The tie of `1.0` and the next `f64`, `1 + 2^-53`, and one unit above it.
  let tie: Decimal<U1, U54, i192> =
    value("1.00000000000000011102230246251565404236316680908203125");
  assert_eq!(tie.to_f64().to_bits(), 1.0f64.to_bits());
  let above: Decimal<U1, U54, i192> =
    value("1.000000000000000111022302462515654042363166809082031251");
  assert_eq!(above.to_f64().to_bits(), (1.0 + f64::EPSILON).to_bits());
}

/// The largest `f32`, the tie above it, `2^128 - 2^103`, one unit of the last digit on either side
/// of the tie, and `2^128`, each of either sign, give the largest float or the infinity of their
/// sign: the carry out of the largest float and the exponent above the largest. `MAX` of the type
/// with the most integral digits is an infinity as an `f32` and a finite `f64`.
#[test]
fn decimals_beyond_the_largest_f32_give_infinities() {
  let largest = BigInt::from((1u32 << 24) - 1) << 104u32;
  let tie = BigInt::from((1u32 << 25) - 1) << 103u32;
  assert_eq!(
    (largest.to_string(), tie.to_string()),
    (
      String::from("340282346638528859811704183484516925440"),
      String::from("340282356779733661637539395458142568448")
    )
  );
  let cases = [
    (largest, f32::MAX),
    (&tie - 1u8, f32::MAX),
    (tie.clone(), f32::INFINITY),
    (&tie + 1u8, f32::INFINITY),
    (BigInt::from(1u8) << 128u32, f32::INFINITY),
  ];
  for (magnitude, expected) in cases {
    for negative in [false, true] {
      let (coefficient, expected) = if negative {
        (-&magnitude, -expected)
      } else {
        (magnitude.clone(), expected)
      };
      let decimal: Top = of_coefficient(&coefficient);
      assert_eq!(
        decimal.to_f32().to_bits(),
        expected.to_bits(),
        "{coefficient}"
      );
    }
  }
  assert_eq!(Du512::MAX.to_f32().to_bits(), f32::INFINITY.to_bits());
  let parsed: f64 = Du512::MAX.to_string().parse().expect("a float");
  assert_eq!(Du512::MAX.to_f64().to_bits(), parsed.to_bits());
  assert!(parsed.is_finite());
}

/// Checks the subnormal floats and the zeros of `f32` in `T`, a type with at least 150 fraction
/// digits and no integral digit, at each value of either sign that the type holds: `2^-126` and
/// one unit of the last digit on either side, which give `2^-126`; the midpoint of the largest
/// subnormal float and `2^-126`, `2^-126 - 2^-150`, and one unit above it, which give `2^-126`,
/// the tie going to its even pattern; one unit below the midpoint and the largest subnormal float,
/// `2^-126 - 2^-149`, which give that float; `2^-149` and one unit above `2^-150`, which give
/// `2^-149`; and `2^-150` and one unit below it, which give a zero with the sign of the value.
#[inline(never)]
fn subnormals_and_zeros<T: FixedDecimal>(conversions: &Conversions<T>) {
  let fraction = T::FRACTION;
  let normal = power_of_two(-126, fraction);
  let midpoint = &normal - power_of_two(-150, fraction);
  let half = power_of_two(-150, fraction);
  let cases = [
    (normal.clone(), 0x0080_0000),
    (&normal - 1u8, 0x0080_0000),
    (&normal + 1u8, 0x0080_0000),
    (midpoint.clone(), 0x0080_0000),
    (&midpoint + 1u8, 0x0080_0000),
    (&midpoint - 1u8, 0x007f_ffff),
    (&normal - power_of_two(-149, fraction), 0x007f_ffff),
    (power_of_two(-149, fraction), 1),
    (&half + 1u8, 1),
    (half.clone(), 0),
    (&half - 1u8, 0),
  ];
  for (magnitude, pattern) in cases {
    let mut signs = Vec::from([(magnitude.clone(), pattern)]);
    if T::SIGNED {
      signs.push((-magnitude, pattern | 0x8000_0000));
    }
    for (coefficient, pattern) in signs {
      let decimal: T = of_coefficient(&coefficient);
      assert_eq!(
        (conversions.to_f32)(&decimal).to_bits(),
        pattern,
        "{coefficient}e-{fraction} in {}",
        type_name::<T>()
      );
    }
  }
}

/// `to_f32` of a magnitude below the smallest normal `f32` gives a subnormal float, `2^-126` from
/// the midpoint of the largest subnormal float and `2^-126` up, and a zero with the sign of the
/// value from half of the smallest subnormal float down; and at each count of kept bits, from 0 to
/// 23, the tie above the float whose pattern is that many ones, and one unit on either side of it,
/// convert to the even float of the two, the lower and the upper one.
#[test]
fn decimals_below_the_smallest_normal_f32_give_subnormals_and_zeros() {
  subnormals_and_zeros(&conversions!(BelowOne));
  subnormals_and_zeros(&conversions!(D0x154));
  let ones: Vec<f32> = (0..24)
    .map(|count| f32::from_bits((1 << count) - 1))
    .collect();
  ties_of(&ones, &conversions!(BelowOne));
}

/// The values at which the conversions of `T` are checked: zero, the limits, one, two and 25 units
/// of the last digit, negated too for a signed storage, and random coefficients, 150 in the Full
/// tier and 12 in the Debug tier, of random lengths and, for a signed storage, signs; under Miri
/// none.
fn storage_values<T: FixedDecimal>(words: &mut Words) -> Vec<T> {
  let mut values = Vec::from([T::zero(), T::min_value(), T::max_value()]);
  for units in [1i32, 2, 25] {
    values.push(of_coefficient(&BigInt::from(units)));
    if T::SIGNED {
      values.push(of_coefficient(&BigInt::from(-units)));
    }
  }
  let random = match tier() {
    Tier::Full => 150,
    Tier::Debug => 12,
    Tier::Miri => 0,
  };
  for _ in 0..random {
    let digits = 1 + (words.next() % u64::from(T::DIGITS)) as u32;
    let magnitude = random_magnitude(words, digits);
    let negative = T::SIGNED && words.next() & 1 == 1;
    values.push(of_coefficient(&if negative {
      -magnitude
    } else {
      magnitude
    }));
  }
  values
}

/// Checks that the conversion of `value` to `F` gives the float that the standard library parses
/// from the text of the decimal, compared by the bits.
fn check_parsed<F: Float, T: FixedDecimal>(value: &T, conversions: &Conversions<T>) {
  let text = value.to_string();
  let parsed: F = text.parse().expect("a float");
  assert_eq!(
    F::of_decimal(conversions, value).to_raw(),
    parsed.to_raw(),
    "{text} in {} to {}",
    type_name::<T>(),
    type_name::<F>()
  );
}

/// Checks `to_f64` and `to_f32` of `T` at each value of [`storage_values`] against the parsed text.
#[inline(never)]
fn to_floats_match_the_parsed_text<T: FixedDecimal>(conversions: &Conversions<T>) {
  let mut words = Words::new(seed::<T>(0x5eed_70f1));
  for value in storage_values::<T>(&mut words) {
    check_parsed::<f64, T>(&value, conversions);
    check_parsed::<f32, T>(&value, conversions);
  }
}

/// `to_f64` and `to_f32` of one type of each storage give the float that the standard library
/// parses from the text of the decimal, at zero, the limits, a few units of the last digit and
/// random values: the glue of each storage, which lends the coefficient's limbs; under Miri, of a
/// signed native, an unsigned native and a wide storage, at the fixed values. And `64250.5`, `0.1`
/// and `-0.1` as literals.
#[test]
fn floats_of_every_storage_match_the_parsed_text() {
  for_each_type!(to_floats_match_the_parsed_text: Di8, Du64, Di256);
  if tier().is_miri() {
    return;
  }
  for_each_type!(to_floats_match_the_parsed_text:
    Di16, Di32, Di64, Di128, Di192, Di320, Di384, Di448, Di512,
  );
  for_each_type!(to_floats_match_the_parsed_text:
    Du8, Du16, Du32, Du128, Du192, Du256, Du320, Du384, Du448, Du512, D0x154,
  );

  type Price = Decimal<U10, U8>;
  for (text, wide, narrow) in [
    ("64250.5", 64250.5, 64250.5),
    ("0.1", 0.1, 0.1),
    ("-0.1", -0.1, -0.1),
  ] {
    let price: Price = value(text);
    assert_eq!(price.to_f64().to_bits(), f64::to_bits(wide), "{text}");
    assert_eq!(price.to_f32().to_bits(), f32::to_bits(narrow), "{text}");
  }
}

/// Checks that `to_f64` and `to_f32` of `T`, a type without fraction digits, give what `integer`
/// gives for the coefficient, at each value of [`storage_values`].
#[inline(never)]
fn integers_agree<T: FixedDecimal>(
  conversions: &Conversions<T>,
  integer: fn(T::Repr) -> (f64, f32),
) {
  let mut words = Words::new(seed::<T>(0x5eed_1a7e));
  for value in storage_values::<T>(&mut words) {
    let (wide, narrow) = integer(value.clone().repr());
    assert_eq!(
      (
        (conversions.to_f64)(&value).to_bits(),
        (conversions.to_f32)(&value).to_bits()
      ),
      (wide.to_bits(), narrow.to_bits()),
      "{value} in {}",
      type_name::<T>()
    );
  }
}

/// A decimal without fraction digits converts as its coefficient does: as `as` converts a native
/// integer, and as the wide integers convert, for a native and a wide storage of each sign and for
/// the widest type, whose `MAX` is an infinity as an `f32`.
#[test]
fn decimals_without_fraction_digits_convert_as_their_integers() {
  integers_agree(&conversions!(Decimal<U18, U0, i64>), |repr| {
    (repr as f64, repr as f32)
  });
  integers_agree(&conversions!(Decimal<U19, U0, u64>), |repr| {
    (repr as f64, repr as f32)
  });
  integers_agree(&conversions!(Di320), |repr| (repr.to_f64(), repr.to_f32()));
  integers_agree(&conversions!(Du256), |repr| (repr.to_f64(), repr.to_f32()));
  integers_agree(&conversions!(Du512), |repr| (repr.to_f64(), repr.to_f32()));
}

/// The modes in which the readings of a float into each storage are checked: all eight; under
/// Miri, two.
fn modes() -> &'static [RoundingMode] {
  if tier().is_miri() {
    &[RoundingMode::HalfEven, RoundingMode::Up05]
  } else {
    &MODES
  }
}

/// The value of `T` whose coefficient is `coefficient`, a float rounded with a mode, or the error
/// of a coefficient outside the range of `T`: the overflow of its sign beyond `10^DIGITS - 1` in
/// magnitude, and `NegOverflow` for a negative coefficient of an unsigned storage.
fn in_type<T: FixedDecimal>(coefficient: &BigInt) -> Result<T, FromFloatError> {
  let negative = coefficient.sign() == Sign::Minus;
  let beyond = coefficient.magnitude() >= bigint::pow10(T::DIGITS).magnitude();
  match (beyond || (negative && !T::SIGNED), negative) {
    (false, _) => Ok(of_coefficient(coefficient)),
    (true, true) => Err(FromFloatError::NegOverflow),
    (true, false) => Err(FromFloatError::PosOverflow),
  }
}

/// The coefficient of the exact binary value of `parts` at `fraction` fraction digits, rounded to
/// an integer with `mode` in `num-bigint`: the quotient of
/// `±significand * 10^fraction * 2^max(exponent, 0)` by `2^max(-exponent, 0)`.
fn exact_coefficient(parts: FloatParts, fraction: u32, mode: RoundingMode) -> BigInt {
  let mut value = BigInt::from(parts.significand) * bigint::pow10(fraction);
  let mut divisor = BigInt::from(1u8);
  if parts.exponent >= 0 {
    value <<= parts.exponent.unsigned_abs();
  } else {
    divisor <<= parts.exponent.unsigned_abs();
  }
  if parts.negative {
    value = -value;
  }
  bigint::round(&value, &divisor, mode)
}

/// What `round_from_f64` or `round_from_f32` of `x` with `mode` gives in `T`, from the exact
/// rational: the error of NaN and of an infinity, and otherwise the rounded coefficient, or the
/// error of the side of the range that it passes.
fn rounded_binary<T: FixedDecimal, F: TestFloat>(
  x: F,
  mode: RoundingMode,
) -> Result<T, FromFloatError> {
  in_type(&exact_coefficient(x.parts()?, T::FRACTION, mode))
}

/// The floats at which `round_coefficient` is checked in the format of `F`: in the Debug tier, the
/// 32 finite floats of `floats::samples` without a width or a random float, zero, its eleven seeds
/// from `0.0001` to `6.5`, the smallest and the largest subnormal float, the smallest normal float
/// and the largest float, each of either sign; in the Full tier, the finite floats of `samples` at
/// the widths 1, 24, 53, 64, 128 and 511 with 32 random ones; under Miri, every eighth float of the
/// Debug tier.
fn core_floats<F: TestFloat>() -> Vec<F> {
  let samples = if tier().is_full() {
    floats::samples::<F>(&[1, 24, 53, 64, 128, 511], 32)
  } else {
    floats::samples::<F>(&[], 0)
  };
  // `samples` gives NaN and the infinities too, and the pattern after an infinity is a NaN.
  let finite = samples.into_iter().filter(|x| x.parts().is_ok());
  if tier().is_miri() {
    finite.step_by(8).collect()
  } else {
    finite.collect()
  }
}

/// Checks `round_coefficient` in the format of `F` at every fraction count against the magnitude
/// of [`exact_coefficient`], or `None` where it has more than 512 bits, at each float of
/// [`core_floats`]: in every mode in the Full tier, and otherwise in one mode taken in turn by the
/// index of the float and the fraction count, so that over the counts every float meets every
/// mode.
#[inline(never)]
fn core_rounds_the_exact_value<F: TestFloat>() {
  let floats = core_floats::<F>();
  for fraction in fraction_counts() {
    for (index, &x) in floats.iter().enumerate() {
      let parts = x.parts().expect("a finite float");
      let turn = [MODES[(index + fraction as usize) % MODES.len()]];
      let chosen: &[RoundingMode] = if tier().is_full() { &MODES } else { &turn };
      for &mode in chosen {
        let exact = BigInt::from(exact_coefficient(parts, fraction, mode).magnitude().clone());
        assert_eq!(
          float::round_coefficient(parts, fraction, mode),
          bigint::to_limbs::<8>(&exact, false),
          "{:#x} at {fraction} fraction digits, {mode}",
          x.to_raw()
        );
      }
    }
  }
}

/// `round_coefficient`, the core of `round_from_f64` and `round_from_f32`, rounds the exact binary
/// value of a float times `10^fraction` once, as the rational in `num-bigint` does, at every
/// fraction count and in both formats, and reports a magnitude beyond 512 bits.
#[test]
fn the_core_rounds_the_exact_value_at_every_fraction_count() {
  core_rounds_the_exact_value::<f64>();
  core_rounds_the_exact_value::<f32>();
}

/// Checks that `round_from_f32` of each float of `floats` gives in `T` what `round_from_f64`
/// gives for the `f64` of the same value, in one mode taken in turn.
#[inline(never)]
fn f32_reads_as_its_f64<T: FixedDecimal>(floats: &[f32], conversions: &Conversions<T>) {
  for (index, &x) in floats.iter().enumerate() {
    let mode = MODES[index % MODES.len()];
    assert_eq!(
      (conversions.round_from_f32)(x, mode),
      (conversions.round_from_f64)(f64::from(x), mode),
      "{:#x}, {mode} into {}",
      x.to_bits(),
      type_name::<T>()
    );
  }
}

/// `round_from_f32` gives what `round_from_f64` gives for the `f64` of the same value, over the
/// six ranges of `f32`, in the types that hold their midpoints: the parts of an `f32` and of its
/// `f64` differ, a shorter significand at a higher exponent, but name the same number.
#[test]
fn an_f32_rounds_as_the_f64_of_the_same_value() {
  let ranges = ranges();
  f32_reads_as_its_f64(&ranges.below_one, &conversions!(BelowOne));
  f32_reads_as_its_f64(&ranges.around_one, &conversions!(AroundOne));
  f32_reads_as_its_f64(&ranges.above_one, &conversions!(AboveOne));
  f32_reads_as_its_f64(&ranges.top, &conversions!(Top));
}

/// The smallest and the largest subnormal float and the smallest normal one of the format of `F`,
/// of either sign.
fn tiny_floats<F: TestFloat>() -> [F; 6] {
  // The bits of the fraction, which the largest subnormal float sets.
  let fraction = (1u64 << (F::FORMAT.precision() - 1)) - 1;
  let [smallest, largest, normal] = [1, fraction, fraction + 1].map(F::from_raw);
  [
    smallest,
    largest,
    normal,
    smallest.negate(),
    largest.negate(),
    normal.negate(),
  ]
}

/// Checks the reading of the binary value of each tiny float of `F` into `T`, in every mode,
/// against the exact rational and the range of `T`.
#[inline(never)]
fn tiny_floats_read<F: Float, T: FixedDecimal>(conversions: &Conversions<T>) {
  for x in tiny_floats::<F>() {
    for mode in MODES {
      assert_eq!(
        F::round_into(conversions, x, mode),
        rounded_binary::<T, F>(x, mode),
        "{:#x}, {mode} into {}",
        x.to_raw(),
        type_name::<T>()
      );
    }
  }
}

/// Checks `round_from_f64` and `round_from_f32` into `T` at the tiny floats of each format.
#[inline(never)]
fn tiny_floats_round<T: FixedDecimal>(conversions: &Conversions<T>) {
  tiny_floats_read::<f64, T>(conversions);
  tiny_floats_read::<f32, T>(conversions);
}

/// What a float below half a unit of the last digit of `T` gives with `mode`: zero, or one unit
/// with the sign of the float where `mode` rounds it away from zero, which is `NegOverflow` below
/// zero for an unsigned storage.
fn zero_or_one_unit<T: FixedDecimal>(
  negative: bool,
  mode: RoundingMode,
) -> Result<T, FromFloatError> {
  let away = match mode {
    RoundingMode::Up | RoundingMode::Up05 => true,
    RoundingMode::Ceiling => !negative,
    RoundingMode::Floor => negative,
    _ => false,
  };
  match (away, negative) {
    (false, _) => Ok(T::zero()),
    (true, false) => Ok(of_coefficient(&BigInt::from(1u8))),
    (true, true) if T::SIGNED => Ok(of_coefficient(&BigInt::from(-1i8))),
    (true, true) => Err(FromFloatError::NegOverflow),
  }
}

/// The smallest and the largest subnormal float and the smallest normal one of each format and
/// each sign round as the exact rational does, in every mode: in two types with eight and two
/// fraction digits, where each is below half a unit and gives zero, or one unit with the sign of
/// the float under `Up`, `Up05`, `Ceiling` of a positive float and `Floor` of a negative one,
/// which is `NegOverflow` for the unsigned type; in a type with 38 fraction digits, which reads
/// the larger subnormal `f32`s like any other value; and in two with 149 and 154 fraction digits,
/// the first of which holds the smallest `f32` exactly, so that it reads as itself in every mode.
#[test]
fn tiny_floats_round_to_zero_or_to_one_unit() {
  type Cents = Decimal<U10, U2, u64>;
  type Subnormal = Decimal<U0, U149, u512>;
  tiny_floats_round(&conversions!(Di64));
  tiny_floats_round(&conversions!(Cents));
  tiny_floats_round(&conversions!(Decimal<U0, U38>));
  tiny_floats_round(&conversions!(Subnormal));
  tiny_floats_round(&conversions!(D0x154));
  for mode in MODES {
    for x in tiny_floats::<f64>() {
      let negative = x.is_sign_negative();
      let (signed, unsigned) = (
        Di64::round_from_f64(x, mode),
        Cents::round_from_f64(x, mode),
      );
      assert_eq!(
        signed,
        zero_or_one_unit::<Di64>(negative, mode),
        "{x:e}, {mode}"
      );
      assert_eq!(
        unsigned,
        zero_or_one_unit::<Cents>(negative, mode),
        "{x:e}, {mode}"
      );
    }
    for x in tiny_floats::<f32>() {
      let negative = x.is_sign_negative();
      let (signed, unsigned) = (
        Di64::round_from_f32(x, mode),
        Cents::round_from_f32(x, mode),
      );
      assert_eq!(
        signed,
        zero_or_one_unit::<Di64>(negative, mode),
        "{x:e}, {mode}"
      );
      assert_eq!(
        unsigned,
        zero_or_one_unit::<Cents>(negative, mode),
        "{x:e}, {mode}"
      );
    }
    let smallest: Subnormal = of_coefficient(&power_of_two(-149, 149));
    assert_eq!(
      Subnormal::round_from_f32(f32::from_bits(1), mode),
      Ok(smallest),
      "{mode}"
    );
  }
}

/// Checks that `round_coefficient` gives the coefficient of `2^exponent` in the format of `F` at
/// `fraction` fraction digits, which has exactly 512 bits, and `None` for `2^(exponent + 1)`, in
/// every mode.
fn the_edge_of_the_limbs<F: Float>(fraction: u32, exponent: i32) {
  let at = F::narrow(pow2(exponent)).parts().expect("a finite float");
  let past = F::narrow(pow2(exponent + 1))
    .parts()
    .expect("a finite float");
  let coefficient = power_of_two(exponent, fraction);
  assert_eq!(
    coefficient.bits(),
    512,
    "2^{exponent} at {fraction} fraction digits"
  );
  let limbs = bigint::to_limbs::<8>(&coefficient, false);
  for mode in MODES {
    assert_eq!(
      float::round_coefficient(at, fraction, mode),
      limbs,
      "2^{exponent} at {fraction} fraction digits, {mode}, {}",
      type_name::<F>()
    );
    assert_eq!(
      float::round_coefficient(past, fraction, mode),
      None,
      "2^{} at {fraction} fraction digits, {mode}, {}",
      exponent + 1,
      type_name::<F>()
    );
  }
}

/// The coefficient of the float `2^k` at `F` fraction digits, `5^F * 2^(F + k)`, has
/// `k + F + bits(5^F)` bits: `round_coefficient` gives it where that is 512, and `None` one power
/// of two above, in every mode, at no fraction digits and at 77, which only an `f64` reaches, and
/// at 116 and 154, which both formats reach. Through the API, the widest type holds `2^511` and
/// not `2^512`, and one is beyond the range of 154 fraction digits by the comparison with the
/// largest coefficient, within the 512 bits.
#[test]
fn floats_at_the_edge_of_the_scratch_overflow() {
  for (fraction, exponent) in [(0u32, 511i32), (77, 256), (116, 126), (154, 0)] {
    let power_bits = BigInt::from(5u8).pow(fraction).bits();
    assert_eq!(
      i64::from(exponent),
      512 - i64::from(fraction) - power_bits as i64,
      "{fraction} fraction digits"
    );
    the_edge_of_the_limbs::<f64>(fraction, exponent);
    if exponent < 127 {
      the_edge_of_the_limbs::<f32>(fraction, exponent);
    }
  }
  let power: Du512 = of_coefficient(&(BigInt::from(1u8) << 511u32));
  for mode in MODES {
    assert_eq!(Du512::round_from_f64(pow2(511), mode), Ok(power), "{mode}");
    for (x, error) in [
      (pow2(512), FromFloatError::PosOverflow),
      (-pow2(512), FromFloatError::NegOverflow),
    ] {
      assert_eq!(Du512::round_from_f64(x, mode), Err(error), "{x:e}, {mode}");
    }
    assert_eq!(
      D0x154::round_from_f64(1.0, mode),
      Err(FromFloatError::PosOverflow),
      "{mode}"
    );
    assert_eq!(
      D0x154::round_from_f32(1.0, mode),
      Err(FromFloatError::PosOverflow),
      "{mode}"
    );
  }
}

/// The floats of `F` at which the readings of a float into `T` are checked: the float nearest to
/// `MAX` and the two beside it, and those of `MIN` for a signed storage; the halves of the type,
/// `2^-(Fraction + 1)` times 1, 3 and 5, of either sign, where `F` holds them, which lie halfway
/// between two values of the type; zero and `-0.0`; and random floats of either sign, from two
/// decimal digits below one unit of the last digit to one digit above `MAX`, as far as the format
/// reaches; of all of which the finite ones, 300 in the Full tier and 30 in the Debug tier; under
/// Miri, the first four.
fn storage_floats<F: Float, T: FixedDecimal>(words: &mut Words) -> Vec<F> {
  let mut floats = Vec::new();
  let mut limits = Vec::from([T::max_value()]);
  if T::SIGNED {
    limits.push(T::min_value());
  }
  for limit in limits {
    let nearest: F = limit.to_string().parse().expect("a float");
    let raw = nearest.to_raw();
    floats.extend([F::from_raw(raw - 1), nearest, F::from_raw(raw + 1)]);
  }
  let half = pow2(-(T::FRACTION as i32) - 1);
  for multiple in [1.0, 3.0, 5.0] {
    for value in [half * multiple, -half * multiple] {
      let x = F::narrow(value);
      if x.wide().to_bits() == value.to_bits() {
        floats.push(x);
      }
    }
  }
  floats.extend([F::narrow(0.0), F::narrow(-0.0)]);
  floats.retain(|x| x.parts().is_ok());
  // The binary exponents from `10^-(Fraction + 2)` to `10^(Integral + 1)`, within the format.
  let max_exponent = (1i32 << (F::FORMAT.exponent_bits() - 1)) - 1;
  let low = ((-f64::from(T::FRACTION) - 2.0) * LOG2_10).floor() as i32;
  let high = ((f64::from(T::INTEGRAL) + 1.0) * LOG2_10).ceil() as i32;
  let (low, high) = (low.max(F::MIN_EXPONENT), high.min(max_exponent));
  let count = if tier().is_full() { 300 } else { 30 };
  while floats.len() < count {
    let exponent = low + (words.next() % (high - low + 1) as u64) as i32;
    let magnitude = (1.0 + (words.next() >> 12) as f64 * pow2(-52)) * pow2(exponent);
    let x = F::narrow(if words.next() & 1 == 1 {
      -magnitude
    } else {
      magnitude
    });
    if x.parts().is_ok() {
      floats.push(x);
    }
  }
  if tier().is_miri() {
    floats.truncate(4);
  }
  floats
}

/// Checks the reading of the binary value of each float of [`storage_floats`] of `F` into `T`, in
/// each mode, against the exact rational and the range of `T`.
#[inline(never)]
fn binary_readings<F: Float, T: FixedDecimal>(conversions: &Conversions<T>, words: &mut Words) {
  for x in storage_floats::<F, T>(words) {
    for &mode in modes() {
      assert_eq!(
        F::round_into(conversions, x, mode),
        rounded_binary::<T, F>(x, mode),
        "{:#x}, {mode} into {}",
        x.to_raw(),
        type_name::<T>()
      );
    }
  }
}

/// Checks `round_from_f64` and `round_from_f32` into `T` at the floats of [`storage_floats`] of
/// each format, and at the largest `f64` of either sign, beyond every range.
#[inline(never)]
fn binary_readings_round<T: FixedDecimal>(conversions: &Conversions<T>) {
  let mut words = Words::new(seed::<T>(0x5eed_b1a7));
  binary_readings::<f64, T>(conversions, &mut words);
  binary_readings::<f32, T>(conversions, &mut words);
  for &mode in modes() {
    for (x, error) in [
      (f64::MAX, FromFloatError::PosOverflow),
      (-f64::MAX, FromFloatError::NegOverflow),
    ] {
      assert_eq!(
        (conversions.round_from_f64)(x, mode),
        Err(error),
        "{x:e}, {mode} into {}",
        type_name::<T>()
      );
    }
  }
}

/// The table of the docs of `RoundingMode`: each value, and the integer to which it rounds in each
/// mode, in the order of `MODES`.
const MODE_TABLE: [(f64, [i8; 8]); 12] = [
  (5.5, [6, 5, 6, 5, 6, 5, 6, 6]),
  (2.5, [3, 2, 3, 2, 3, 2, 2, 2]),
  (1.6, [2, 1, 2, 1, 2, 2, 2, 1]),
  (1.1, [2, 1, 2, 1, 1, 1, 1, 1]),
  (1.0, [1, 1, 1, 1, 1, 1, 1, 1]),
  (0.1, [1, 0, 1, 0, 0, 0, 0, 1]),
  (-0.1, [-1, 0, 0, -1, 0, 0, 0, -1]),
  (-1.0, [-1, -1, -1, -1, -1, -1, -1, -1]),
  (-1.1, [-2, -1, -1, -2, -1, -1, -1, -1]),
  (-1.6, [-2, -1, -1, -2, -2, -2, -2, -1]),
  (-2.5, [-3, -2, -2, -3, -3, -2, -2, -2]),
  (-5.5, [-6, -5, -5, -6, -6, -5, -6, -6]),
];

/// Checks that `read`, a reading of a float into a type of one integral digit and none of
/// fraction, gives each cell of the table of the docs of `RoundingMode`, and that `read_two`, into
/// two integral digits, rounds `0.5` and `-0.5` in each mode as that table rounds a half.
fn rounding_mode_table(
  read: fn(f64, RoundingMode) -> Result<Decimal<U1, U0>, FromFloatError>,
  read_two: fn(f64, RoundingMode) -> Result<Decimal<U2, U0>, FromFloatError>,
) {
  for (x, results) in MODE_TABLE {
    for (mode, result) in MODES.into_iter().zip(results) {
      let expected = of_coefficient::<Decimal<U1, U0>>(&BigInt::from(result));
      assert_eq!(read(x, mode), Ok(expected), "{x}, {mode}");
    }
  }
  let halves: [(f64, [i8; 8]); 2] = [
    (0.5, [1, 0, 1, 0, 1, 0, 0, 1]),
    (-0.5, [-1, 0, 0, -1, -1, 0, 0, -1]),
  ];
  for (x, results) in halves {
    for (mode, result) in MODES.into_iter().zip(results) {
      let expected = of_coefficient::<Decimal<U2, U0>>(&BigInt::from(result));
      assert_eq!(read_two(x, mode), Ok(expected), "{x}, {mode}");
    }
  }
}

/// `round_from_f64` and `round_from_f32` of one type of each storage round the exact binary value
/// of a float once with each mode, as the rational in `num-bigint` does, and report a value
/// outside the range on the side of its sign: at the floats next to the limits, the halves of the
/// type, zero, `-0.0` and random floats, and at the largest `f64`; under Miri, of a signed native,
/// an unsigned native and a wide storage, at four floats in two modes. And the literals: the
/// binary value of `0.1_f64` to 28 fraction digits and of `0.1_f32` in every mode, which holds it
/// exactly, `2.675` of each format, which is below 2.675, the largest `f32` in every mode, and the
/// table of the docs of `RoundingMode`.
#[test]
fn floats_round_into_every_storage_in_every_mode() {
  type Fine = Decimal<U10, U28>;
  type Cents = Decimal<U10, U2>;
  assert_eq!(
    Fine::round_from_f64(0.1, RoundingMode::HalfEven),
    Ok(value("0.1000000000000000055511151231"))
  );
  for mode in MODES {
    assert_eq!(
      Fine::round_from_f32(0.1, mode),
      Ok(value("0.100000001490116119384765625")),
      "{mode}"
    );
    assert_eq!(
      Top::round_from_f32(f32::MAX, mode),
      Ok(value("340282346638528859811704183484516925440")),
      "{mode}"
    );
  }
  assert_eq!(
    Cents::round_from_f64(2.675, RoundingMode::HalfUp),
    Ok(value("2.67"))
  );
  assert_eq!(
    Cents::round_from_f32(2.675, RoundingMode::HalfUp),
    Ok(value("2.67"))
  );
  rounding_mode_table(
    Decimal::<U1, U0>::round_from_f64,
    Decimal::<U2, U0>::round_from_f64,
  );

  for_each_type!(binary_readings_round: Di8, Du64, Di256);
  if tier().is_miri() {
    return;
  }
  for_each_type!(binary_readings_round:
    Di16, Di32, Di64, Di128, Di192, Di320, Di384, Di448, Di512,
  );
  for_each_type!(binary_readings_round:
    Du8, Du16, Du32, Du128, Du192, Du256, Du320, Du384, Du448, Du512, D0x154,
  );
}

/// Checks the special values in `T`, a type of ten integral digits and at most eight fraction
/// digits, through `read`, a reading of the floats of `F` into `T`: NaN is `Nan` and each infinity
/// `Infinite`, zero and `-0.0` are zero, the smallest positive float of either sign, below half a
/// unit of the last digit, is zero or one unit, and `10^10` of either sign, beyond `MAX`, is the
/// overflow of its sign, in every mode; and for an unsigned storage, `-0.001` rounds to zero with
/// `HalfEven`, and `-0.5` is below the range.
#[inline(never)]
fn special_values<F: Float, T: FixedDecimal>(
  read: fn(F, RoundingMode) -> Result<T, FromFloatError>,
) {
  let name = type_name::<T>();
  for mode in MODES {
    assert_eq!(
      read(F::NAN, mode),
      Err(FromFloatError::Nan),
      "NaN, {mode} into {name}"
    );
    for infinity in [f64::INFINITY, f64::NEG_INFINITY] {
      assert_eq!(
        read(F::narrow(infinity), mode),
        Err(FromFloatError::Infinite),
        "{infinity}, {mode} into {name}"
      );
    }
    for zero in [0.0, -0.0] {
      assert_eq!(
        read(F::narrow(zero), mode),
        Ok(T::zero()),
        "{zero:?}, {mode} into {name}"
      );
    }
    let smallest = F::from_raw(1);
    for (x, negative) in [(smallest, false), (smallest.negate(), true)] {
      assert_eq!(
        read(x, mode),
        zero_or_one_unit::<T>(negative, mode),
        "{:#x}, {mode} into {name}",
        x.to_raw()
      );
    }
    for (x, error) in [
      (1e10, FromFloatError::PosOverflow),
      (-1e10, FromFloatError::NegOverflow),
    ] {
      assert_eq!(
        read(F::narrow(x), mode),
        Err(error),
        "{x:e}, {mode} into {name}"
      );
    }
  }
  if !T::SIGNED {
    assert_eq!(
      read(F::narrow(-0.001), RoundingMode::HalfEven),
      Ok(T::zero()),
      "-0.001 into {name}"
    );
    assert_eq!(
      read(F::narrow(-0.5), RoundingMode::HalfEven),
      Err(FromFloatError::NegOverflow),
      "-0.5 into {name}"
    );
  }
}

/// NaN is `Nan` and each infinity `Infinite`, `-0.0` is zero, a value below half a unit of the
/// last digit is zero or one unit, a value beyond the range is the overflow of its sign, and a
/// negative value is zero in an unsigned storage if it rounds to zero, and `NegOverflow`
/// otherwise: through the readings of the binary value of both formats, for a signed and an
/// unsigned type. Zero converts to `+0.0`, and the largest `f32` is beyond 38 integral digits.
#[test]
fn nan_and_the_infinities_are_errors_and_negative_zero_is_zero() {
  type Unsigned = Decimal<U10, U2, u64>;
  special_values::<f64, Di64>(Di64::round_from_f64);
  special_values::<f32, Di64>(Di64::round_from_f32);
  special_values::<f64, Unsigned>(Unsigned::round_from_f64);
  special_values::<f32, Unsigned>(Unsigned::round_from_f32);
  assert_eq!(
    (Di64::ZERO.to_f64().to_bits(), Di64::ZERO.to_f32().to_bits()),
    (0, 0)
  );
  assert_eq!(
    (
      Unsigned::ZERO.to_f64().to_bits(),
      Unsigned::ZERO.to_f32().to_bits()
    ),
    (0, 0)
  );
  for mode in MODES {
    assert_eq!(
      Decimal::<U38, U0>::round_from_f32(f32::MAX, mode),
      Err(FromFloatError::PosOverflow),
      "{mode}"
    );
  }
}

/// The conversions to a float and the readings of the binary value of a float are `const fn`s:
/// they evaluate in `const` items, for a native and a wide storage of each sign.
#[test]
fn conversions_in_constant_contexts() {
  type Fee = Decimal<U10, U2, u64>;
  type Large = Decimal<U40, U17, i192>;
  type Amount = Decimal<U39, U18, u192>;
  const fn price(text: &str) -> Di64 {
    match Di64::parse(text) {
      Ok(price) => price,
      Err(_) => panic!("a price"),
    }
  }
  const fn large(text: &str) -> Large {
    match Large::parse(text) {
      Ok(large) => large,
      Err(_) => panic!("a large value"),
    }
  }
  const PRICE: Di64 = price("-64250.5");
  const TO: (f64, f32, f64, f32) = (
    PRICE.to_f64(),
    PRICE.to_f32(),
    large("-0.1").to_f64(),
    Amount::MAX.to_f32(),
  );
  const FROM: [Result<Di64, FromFloatError>; 2] = [
    Di64::round_from_f64(0.1, RoundingMode::HalfEven),
    Di64::round_from_f32(f32::NAN, RoundingMode::Down),
  ];
  const FROM_UNSIGNED: [Result<Fee, FromFloatError>; 2] = [
    Fee::round_from_f64(2.675, RoundingMode::HalfUp),
    Fee::round_from_f32(-1.0, RoundingMode::Up),
  ];
  const FROM_WIDE: (
    Result<Large, FromFloatError>,
    Result<Amount, FromFloatError>,
  ) = (
    Large::round_from_f64(-2.5, RoundingMode::HalfEven),
    Amount::round_from_f32(0.75, RoundingMode::Down),
  );
  assert_eq!(
    (
      TO.0.to_bits(),
      TO.1.to_bits(),
      TO.2.to_bits(),
      TO.3.to_bits()
    ),
    (
      (-64250.5f64).to_bits(),
      (-64250.5f32).to_bits(),
      (-0.1f64).to_bits(),
      f32::INFINITY.to_bits()
    )
  );
  assert_eq!(FROM, [Ok(price("0.1")), Err(FromFloatError::Nan)]);
  assert_eq!(
    FROM_UNSIGNED,
    [Ok(value("2.67")), Err(FromFloatError::NegOverflow)]
  );
  assert_eq!(FROM_WIDE, (Ok(large("-2.5")), Ok(value("0.75"))));
}
