//! The conversions of `Decimal` to `f64` and `f32`. The core of the float module is checked at
//! every fraction count from 0 to 154 against the standard library's parsing of the text of the
//! decimal; the decimals between adjacent floats, built in `num-bigint` from the parts of the
//! lower float, against the rule of ties to even, over six ranges of `f32` and random `f64`s; the
//! families of values where the rounding changes its case, the boundaries of the binades, the
//! infinity of `f32`, its subnormal floats and its zeros; and the inherent `const fn`s of every
//! storage against the parsed text of the decimal, and those of a type without fraction digits
//! against the integers' own conversions. Every float is compared by its bits, so that `-0.0` and
//! the infinities count.
//!
//! The checks are generic over the float, through `TestFloat` and the local trait [`Float`], and
//! call the inherent functions of a decimal type through the pointers of [`Conversions`], which
//! [`conversions!`] takes from the type. Each type checks in a function of its own, never inlined,
//! whose frame holds the values of that type alone: an optimized build would inline a function
//! that has one call site, and the frame of the test would then hold the values of every type. The
//! tier of the build sizes every sweep, and every random value comes from a generator with a seed
//! written here, so that a failure replays.

use core::{any::type_name, num::ParseFloatError, str::FromStr};
use std::{
  collections::BTreeSet,
  format,
  string::{String, ToString},
  vec::Vec,
};

use num_bigint::BigInt;
use typenum::{
  U0, U1, U2, U4, U5, U8, U9, U10, U18, U19, U25, U26, U39, U54, U58, U76, U77, U78, U96, U97,
  U116, U135, U153, U154,
};

use crate::{
  Decimal, FixedDecimal,
  bigint::{
    self, Tier,
    floats::{TestFloat, Words, pow2, pow2_f32},
    tier,
  },
  decimal::float,
  i192, i256, i320, i384, i448, i512, u192, u256, u320, u384, u448, u512,
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
/// and the conversion of a decimal to this format through [`Conversions`].
trait Float: TestFloat + FromStr<Err = ParseFloatError> {
  /// The conversion of `value` to this format, through the inherent function of its type.
  fn of_decimal<T>(conversions: &Conversions<T>, value: &T) -> Self;
}

impl Float for f64 {
  fn of_decimal<T>(conversions: &Conversions<T>, value: &T) -> Self {
    (conversions.to_f64)(value)
  }
}

impl Float for f32 {
  fn of_decimal<T>(conversions: &Conversions<T>, value: &T) -> Self {
    (conversions.to_f32)(value)
  }
}

/// The inherent conversions of the decimal type `T` with the floats, which a check generic over the
/// float calls through these pointers.
struct Conversions<T> {
  to_f64: fn(&T) -> f64,
  to_f32: fn(&T) -> f32,
}

/// The [`Conversions`] of the decimal type `$ty`: its inherent `const fn`s.
macro_rules! conversions {
  ($ty:ty) => {
    Conversions::<$ty> {
      to_f64: <$ty>::to_f64,
      to_f32: <$ty>::to_f32,
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
