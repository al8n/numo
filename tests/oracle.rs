//! Property tests that compare `Decimal` with exact big-integer arithmetic.
//!
//! Coefficients come from three sources: random digit strings biased toward `0` and `9`, random
//! bit patterns scaled to the decimal bound, and the edges of the range. Parsing is checked against
//! an independent implementation of the grammar on numbers generated in every syntactic form, on
//! infinities and NaNs, on sequences of grammar tokens and on random text, and directly against
//! `f64::from_str` on token sequences, rendered `f64` text, `f64`'s documented grammar and near
//! misses of it; every value must read back from its text, also when that text is rewritten with
//! an exponent. Each suite covers one decimal type, with a signed or an unsigned storage, native or
//! wide.
//!
//! Rounding is checked against a reference that picks the nearer of the two bracketing multiples by
//! distance, and `round_to` also against `bigdecimal`, which decides from the first dropped digit
//! and whether any later one is nonzero; neither shares the crate's rule. Values at and next to
//! ties are generated for every number of dropped digits, and every mode is checked on each value,
//! together with the symmetry between a value and its negation.
//!
//! Division is checked against the same reference, applied to the exact quotient with both operands
//! scaled and a divisor of either sign, on random operands, the extremes of each type and quotients
//! at and next to ties. Each case also checks the identities between the signs of the operands and
//! the quotient, and that dividing by one rounds as `round_to` does.
//!
//! Scientific notation (`{:e}` and `{:E}`) is checked, for every precision, against a reference
//! that finds the first digit and its exponent in the positional text, and, for values of at most
//! 15 significant digits, against `f64`, whose shortest text is then the exact one. The text of
//! every value must also read back, with a sign, zero padding and extra fraction digits.

// Under Miri on a 32-bit target, the `num-bigint` and `proptest` allocations of this suite use up
// Miri's 32-bit address space. It still runs natively, and under Miri on the 64-bit targets.
#![cfg(not(all(miri, target_pointer_width = "32")))]

use bigdecimal::BigDecimal;
use num_bigint::{BigInt, Sign};
use numo::{
  ConvertError, Decimal, DivideError, ParseErrorKind, QuantizeError, RoundingMode, i192, i256,
  i320, i384, i448, i512,
  typenum::{
    U0, U1, U2, U3, U4, U5, U6, U7, U8, U9, U10, U11, U12, U14, U16, U17, U18, U19, U20, U22, U26,
    U28, U30, U36, U37, U38, U39, U49, U58, U59, U60, U61, U69, U70, U76, U78, U79, U90, U96, U97,
    U98, U116, U117, U135, U153, U154,
  },
  u192, u256, u320, u384, u448, u512,
};
use proptest::{prelude::*, test_runner::RngSeed};

fn config() -> ProptestConfig {
  if cfg!(miri) {
    ProptestConfig {
      cases: 4,
      failure_persistence: None,
      rng_seed: RngSeed::Fixed(0x5eed),
      ..ProptestConfig::default()
    }
  } else {
    ProptestConfig::default()
  }
}

fn pow10(exp: u32) -> BigInt {
  BigInt::from(10u8).pow(exp)
}

fn max_coeff(digits: u32) -> BigInt {
  pow10(digits) - 1u8
}

fn in_range(coeff: &BigInt, digits: u32) -> bool {
  coeff.magnitude() <= max_coeff(digits).magnitude()
}

/// The integer that `limbs` holds, least significant limb first, in two's complement if `signed`.
fn limbs_to_big(limbs: &[u64], signed: bool) -> BigInt {
  let bytes: Vec<u8> = limbs.iter().flat_map(|limb| limb.to_le_bytes()).collect();
  if signed {
    BigInt::from_signed_bytes_le(&bytes)
  } else {
    BigInt::from_bytes_le(Sign::Plus, &bytes)
  }
}

/// `value` in `N` limbs, least significant first, in two's complement if `signed`, or `None` if it
/// is out of range.
fn big_to_limbs<const N: usize>(value: &BigInt, signed: bool) -> Option<[u64; N]> {
  let negative = value.sign() == Sign::Minus;
  // The shortest two's complement or unsigned encoding, which fits exactly when the value does.
  let mut bytes = match (signed, negative) {
    (true, _) => value.to_signed_bytes_le(),
    (false, false) => value.to_bytes_le().1,
    (false, true) => return None,
  };
  if bytes.len() > 8 * N {
    return None;
  }
  bytes.resize(8 * N, if negative { 0xff } else { 0 });
  let mut limbs = [0; N];
  for (limb, chunk) in limbs.iter_mut().zip(bytes.as_chunks::<8>().0) {
    *limb = u64::from_le_bytes(*chunk);
  }
  Some(limbs)
}

/// A storage type, compared with the reference through `num-bigint` integers.
trait Repr: Copy + PartialEq + std::fmt::Debug + 'static {
  /// The width in bits.
  const BITS: u32;

  fn to_big(self) -> BigInt;

  /// The value equal to `value`, if the storage holds it.
  fn from_big(value: &BigInt) -> Option<Self>;

  /// Every bit pattern of the storage, uniformly.
  fn any() -> BoxedStrategy<Self>;
}

macro_rules! native_repr {
  ($($repr:ident),+) => {$(
    impl Repr for $repr {
      const BITS: u32 = $repr::BITS;

      fn to_big(self) -> BigInt {
        BigInt::from(self)
      }

      fn from_big(value: &BigInt) -> Option<Self> {
        $repr::try_from(value).ok()
      }

      fn any() -> BoxedStrategy<Self> {
        any::<$repr>().boxed()
      }
    }
  )+};
}

native_repr!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128);

macro_rules! wide_repr {
  ($($repr:ident: $signed:literal, $limbs:literal),+) => {$(
    impl Repr for $repr {
      const BITS: u32 = 64 * $limbs;

      fn to_big(self) -> BigInt {
        limbs_to_big(&self.to_limbs(), $signed)
      }

      fn from_big(value: &BigInt) -> Option<Self> {
        big_to_limbs(value, $signed).map($repr::from_limbs)
      }

      fn any() -> BoxedStrategy<Self> {
        any::<[u64; $limbs]>().prop_map($repr::from_limbs).boxed()
      }
    }
  )+};
}

wide_repr!(
  i192: true, 3,
  i256: true, 4,
  i320: true, 5,
  i384: true, 6,
  i448: true, 7,
  i512: true, 8,
  u192: false, 3,
  u256: false, 4,
  u320: false, 5,
  u384: false, 6,
  u448: false, 7,
  u512: false, 8
);

/// The shortest exact text of `coeff / 10^fraction`, padded to at least `precision` fraction
/// digits.
fn oracle_text(coeff: &BigInt, fraction: u32, precision: usize) -> String {
  let fraction = fraction as usize;
  let digits = coeff.magnitude().to_string();
  let digits = if digits.len() <= fraction {
    format!("{}{digits}", "0".repeat(fraction + 1 - digits.len()))
  } else {
    digits
  };
  let (integral, fraction_digits) = digits.split_at(digits.len() - fraction);
  let mut fraction_digits = fraction_digits.trim_end_matches('0').to_owned();
  while fraction_digits.len() < precision {
    fraction_digits.push('0');
  }

  let mut text = String::new();
  if coeff.sign() == Sign::Minus {
    text.push('-');
  }
  text.push_str(integral);
  if !fraction_digits.is_empty() {
    text.push('.');
    text.push_str(&fraction_digits);
  }
  text
}

/// The scientific text of `coeff / 10^fraction`: the significant digits with the point after the
/// first, padded to at least `precision` fraction digits, then `e` or `E` and the exponent of the
/// first digit. Zero is `0e0`.
///
/// The first digit and its exponent are read off the positional text, as the place of the first
/// nonzero digit relative to the point, where the crate computes them from the number of digits.
fn oracle_exp_text(coeff: &BigInt, fraction: u32, precision: usize, upper: bool) -> String {
  let positional = oracle_text(coeff, fraction, 0);
  let magnitude = positional.trim_start_matches('-');
  let (integral, fraction_digits) = magnitude.split_once('.').unwrap_or((magnitude, ""));
  let (digits, exponent) = if integral != "0" {
    (
      format!("{integral}{fraction_digits}"),
      integral.len() as i64 - 1,
    )
  } else {
    let zeros = fraction_digits.len() - fraction_digits.trim_start_matches('0').len();
    (fraction_digits[zeros..].to_owned(), -(zeros as i64) - 1)
  };
  let digits = digits.trim_end_matches('0');
  let (digits, exponent) = if digits.is_empty() {
    ("0", 0)
  } else {
    (digits, exponent)
  };

  let (first, rest) = digits.split_at(1);
  let mut text = String::new();
  if coeff.sign() == Sign::Minus {
    text.push('-');
  }
  text.push_str(first);
  if !rest.is_empty() || precision > 0 {
    text.push('.');
    text.push_str(rest);
    text.push_str(&"0".repeat(precision.saturating_sub(rest.len())));
  }
  text.push(if upper { 'E' } else { 'e' });
  text.push_str(&exponent.to_string());
  text
}

/// The number of digits from the first to the last nonzero digit of the decimal text `text`.
fn significant_digits(text: &str) -> usize {
  let digits: String = text.chars().filter(char::is_ascii_digit).collect();
  digits.trim_matches('0').len()
}

/// The coefficient that `input` denotes for `integral` and `fraction` digits, where only a signed
/// storage accepts a negative number and an unsigned one reports it as `Negative`.
///
/// The crate scans the input once, byte by byte, and tracks digit positions. This reference splits
/// the text with string methods instead, and computes the value with big-integer powers of ten.
fn oracle_parse(
  input: &str,
  integral: u32,
  fraction: u32,
  signed: bool,
) -> Result<BigInt, ParseErrorKind> {
  if input.is_empty() {
    return Err(ParseErrorKind::Empty);
  }
  let (negative, body) = match input.strip_prefix('-') {
    Some(rest) => (true, rest),
    None => (false, input.strip_prefix('+').unwrap_or(input)),
  };

  let lower = body.to_ascii_lowercase();
  if lower == "inf" || lower == "infinity" {
    return Err(ParseErrorKind::NotFinite);
  }
  if let Some(payload) = lower
    .strip_prefix("snan")
    .or_else(|| lower.strip_prefix("nan"))
  {
    return Err(if payload.bytes().all(|b| b.is_ascii_digit()) {
      ParseErrorKind::NotFinite
    } else {
      ParseErrorKind::InvalidSyntax
    });
  }

  let (mantissa_text, exponent_text) = match body.split_once(['e', 'E']) {
    Some((mantissa_text, exponent_text)) => (mantissa_text, Some(exponent_text)),
    None => (body, None),
  };
  // `1` and `1.` mean the same, so a missing fraction and an empty one need not be told apart.
  let (integral_text, fraction_text) = mantissa_text.split_once('.').unwrap_or((mantissa_text, ""));
  // Empty, or a digit followed by digits and underscores.
  let is_run = |text: &str| {
    text.is_empty()
      || (text.starts_with(|c: char| c.is_ascii_digit())
        && text.chars().all(|c| c.is_ascii_digit() || c == '_'))
  };
  let no_digits = integral_text.is_empty() && fraction_text.is_empty();
  if !is_run(integral_text) || !is_run(fraction_text) || no_digits {
    return Err(ParseErrorKind::InvalidSyntax);
  }

  let exponent = match exponent_text {
    None => BigInt::ZERO,
    Some(text) => {
      let (exponent_negative, digits) = match text.strip_prefix('-') {
        Some(digits) => (true, digits),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
      };
      let valid = digits.chars().all(|c| c.is_ascii_digit() || c == '_')
        && digits.chars().any(|c| c.is_ascii_digit());
      if !valid {
        return Err(ParseErrorKind::InvalidSyntax);
      }
      let magnitude: BigInt = digits.replace('_', "").parse().expect("ASCII digits");
      if exponent_negative {
        -magnitude
      } else {
        magnitude
      }
    }
  };

  // The input is a finite number, which an unsigned storage rejects when it is negative, whatever
  // its value.
  if negative && !signed {
    return Err(ParseErrorKind::Negative);
  }

  let mantissa: BigInt = format!("{integral_text}{fraction_text}")
    .replace('_', "")
    .parse()
    .expect("ASCII digits");
  if mantissa.sign() == Sign::NoSign {
    return Ok(BigInt::ZERO);
  }
  // A nonzero mantissa has at most `input.len()` digits, so scaled by an exponent beyond this
  // bound it is at least `10^200` or below `10^-200`: out of range, or inexact for every type.
  let bound = BigInt::from(input.len() + 200);
  if exponent > bound {
    return Err(ParseErrorKind::OutOfRange);
  }
  if exponent < -bound {
    return Err(ParseErrorKind::Inexact);
  }

  // The coefficient is `mantissa * 10^shift`, which must be an integer of at most
  // `integral + fraction` digits.
  let fraction_digits = fraction_text.replace('_', "").len();
  let shift =
    i64::try_from(exponent).expect("bounded") - fraction_digits as i64 + i64::from(fraction);
  let max = max_coeff(integral + fraction);
  let magnitude = if shift >= 0 {
    mantissa * pow10(shift as u32)
  } else {
    let divisor = pow10(shift.unsigned_abs() as u32);
    let quotient = &mantissa / &divisor;
    // The value's integral part has more than `integral` digits exactly when the truncated
    // coefficient exceeds the largest one, so this is checked before exactness.
    if quotient > max {
      return Err(ParseErrorKind::OutOfRange);
    }
    if mantissa % divisor != BigInt::ZERO {
      return Err(ParseErrorKind::Inexact);
    }
    quotient
  };
  if magnitude > max {
    return Err(ParseErrorKind::OutOfRange);
  }
  Ok(if negative { -magnitude } else { magnitude })
}

/// The result of converting `coeff` from `from` to `to` fraction digits, for a target with
/// `digits` digits in total and a signed or an unsigned storage.
fn oracle_convert(
  coeff: &BigInt,
  from: u32,
  to: u32,
  digits: u32,
  signed: bool,
) -> Result<BigInt, ConvertError> {
  // A negative value has no unsigned representation, however many digits are dropped.
  if !signed && coeff.sign() == Sign::Minus {
    return Err(ConvertError::OutOfRange);
  }
  let converted = if to >= from {
    coeff * pow10(to - from)
  } else {
    let divisor = pow10(from - to);
    let quotient = coeff / &divisor;
    if !in_range(&quotient, digits) {
      return Err(ConvertError::OutOfRange);
    }
    if coeff % &divisor != BigInt::ZERO {
      return Err(ConvertError::Inexact);
    }
    quotient
  };
  if in_range(&converted, digits) {
    Ok(converted)
  } else {
    Err(ConvertError::OutOfRange)
  }
}

/// Digit strings, possibly with leading zeros or extra fraction digits, for inputs that are
/// syntactically valid.
fn digit_text(max_len: usize) -> impl Strategy<Value = String> {
  proptest::collection::vec(
    prop_oneof![Just(b'0'), Just(b'9'), b'0'..=b'9'],
    1..=max_len,
  )
  .prop_map(|digits| String::from_utf8(digits).expect("ASCII digits"))
}

/// `digits` with up to two underscores after each digit, mostly none, as a digit run may have them
/// after its first digit.
fn separated(digits: String) -> impl Strategy<Value = String> {
  let underscores = prop_oneof![6 => Just(0usize), 1 => Just(1usize), 1 => Just(2usize)];
  proptest::collection::vec(underscores, digits.len()).prop_map(move |counts| {
    let mut text = String::new();
    for (digit, count) in digits.chars().zip(counts) {
      text.push(digit);
      text.extend(std::iter::repeat_n('_', count));
    }
    text
  })
}

/// The grammar's `digits`: a digit string from `digit_text` with underscores after its digits.
fn digit_run(max_len: usize) -> impl Strategy<Value = String> {
  digit_text(max_len).prop_flat_map(separated)
}

/// An exponent: `e` or `E`, an optional sign, then digits with underscores anywhere. Its magnitude
/// is mostly at most `max`, sometimes with leading zeros, and otherwise a run of up to 30 digits,
/// which can exceed every native integer.
fn exponent_text(max: u32) -> impl Strategy<Value = String> {
  let magnitude = prop_oneof![
    6 => (0..=max, 0usize..3)
      .prop_map(|(value, zeros)| format!("{}{value}", "0".repeat(zeros))),
    1 => digit_text(30),
  ];
  (
    prop_oneof![Just('e'), Just('E')],
    prop_oneof![Just(""), Just("+"), Just("-")],
    0usize..3,
    magnitude.prop_flat_map(separated),
  )
    .prop_map(|(marker, sign, leading, digits)| {
      format!("{marker}{sign}{}{digits}", "_".repeat(leading))
    })
}

/// Finite numbers in every syntactic form: any sign, a mantissa with or without either part and
/// the point, and an optional exponent.
fn number_text(integral: usize, fraction: usize, exponent: u32) -> impl Strategy<Value = String> {
  let mantissa = prop_oneof![
    digit_run(integral),
    digit_run(integral).prop_map(|digits| format!("{digits}.")),
    (digit_run(integral), digit_run(fraction))
      .prop_map(|(integral, fraction)| format!("{integral}.{fraction}")),
    digit_run(fraction).prop_map(|digits| format!(".{digits}")),
  ];
  (
    prop_oneof![Just(""), Just("+"), Just("-")],
    mantissa,
    proptest::option::of(exponent_text(exponent)),
  )
    .prop_map(|(sign, mantissa, exponent)| {
      format!("{sign}{mantissa}{}", exponent.unwrap_or_default())
    })
}

/// Infinities and NaNs, with any sign, random letter case and NaN payloads.
fn nonfinite_text() -> impl Strategy<Value = String> {
  let word = prop_oneof![
    Just(String::from("inf")),
    Just(String::from("infinity")),
    (
      prop_oneof![Just(""), Just("s")],
      proptest::option::of(digit_text(6))
    )
      .prop_map(|(signaling, payload)| format!("{signaling}nan{}", payload.unwrap_or_default())),
  ];
  (
    prop_oneof![Just(""), Just("+"), Just("-")],
    word,
    proptest::collection::vec(any::<bool>(), 10),
  )
    .prop_map(|(sign, word, upper)| {
      let mut text = String::from(sign);
      for (c, upper) in word
        .chars()
        .zip(upper.into_iter().chain(std::iter::repeat(false)))
      {
        text.push(if upper { c.to_ascii_uppercase() } else { c });
      }
      text
    })
}

/// Sequences of grammar pieces and near misses, which form valid and almost valid inputs far more
/// often than random characters do.
fn token_text() -> impl Strategy<Value = String> {
  let token = proptest::sample::select(vec![
    "0", "1", "9", "00", "_", "__", ".", "e", "E", "+", "-", "inf", "INF", "infinity", "Infinity",
    "nan", "NaN", "s", "i", "n", "f", " ", "\t", ",", "x", "\u{663}",
  ]);
  proptest::collection::vec(token, 0..10).prop_map(|tokens| tokens.concat())
}

/// Rewrites decimal text such as `-12.5` with its point moved `shift` places to the left and the
/// exponent `shift`, such as `-0.125e2` for 2 or `-1250e-2` for -2, keeping any sign.
fn with_exponent(text: &str, shift: i32, marker: char) -> String {
  let digits_start = text
    .find(|c: char| c.is_ascii_digit())
    .expect("decimal text has a digit");
  let (sign, magnitude) = text.split_at(digits_start);
  let (integral, fraction) = magnitude.split_once('.').unwrap_or((magnitude, ""));
  let mut digits = format!("{integral}{fraction}");
  // The point follows the first `point` digits.
  let mut point = integral.len() as i64 - i64::from(shift);
  if point < 0 {
    digits.insert_str(0, &"0".repeat(point.unsigned_abs() as usize));
    point = 0;
  }
  let point = point as usize;
  if point > digits.len() {
    digits.push_str(&"0".repeat(point - digits.len()));
  }
  let (integral, fraction) = digits.split_at(point);
  if fraction.is_empty() {
    format!("{sign}{integral}{marker}{shift}")
  } else {
    format!("{sign}{integral}.{fraction}{marker}{shift}")
  }
}

macro_rules! suite {
  (
    $name:ident: $ty:ty,
    $repr:ident,
    $sign:ident,
    integral = $integral:literal,
    fraction = $fraction:literal
  ) => {
    mod $name {
      use super::*;

      pub(super) type T = $ty;
      pub(super) const INTEGRAL: u32 = $integral;
      pub(super) const FRACTION: u32 = $fraction;
      pub(super) const DIGITS: u32 = $integral + $fraction;
  pub(super) const SIGNED: bool = suite!(@signed $sign);

      pub(super) fn big(value: T) -> BigInt {
        value.repr().to_big()
      }

      pub(super) fn from_big(coeff: &BigInt) -> Option<T> {
        <$repr as Repr>::from_big(coeff).and_then(T::from_repr)
      }

      /// Coefficients at the edges of the range and at powers of ten and two inside it.
      fn edges() -> Vec<$repr> {
        // Computed once per suite: every test of the suite draws from them, and big-integer
        // arithmetic is slow under Miri.
        static EDGES: std::sync::OnceLock<Vec<$repr>> = std::sync::OnceLock::new();
        EDGES
          .get_or_init(|| {
            let max = max_coeff(DIGITS);
            let mut values = vec![BigInt::ZERO, BigInt::from(1u8), max.clone(), &max - 1u8];
            let mut power = BigInt::from(1u8);
            for _ in 0..=DIGITS {
              values.extend([power.clone(), &power - 1u8]);
              power *= 10u8;
            }
            for exp in 0..<$repr as Repr>::BITS {
              let power = BigInt::from(1u8) << exp;
              values.extend([power.clone(), power - 1u8]);
            }
            values
              .into_iter()
              .flat_map(|value| [-&value, value])
              .filter(|value| value.magnitude() <= max.magnitude())
              .filter(|value| SIGNED || value.sign() != Sign::Minus)
              .map(|value| <$repr as Repr>::from_big(&value).expect("the storage holds the digits"))
              .collect()
          })
          .clone()
      }

      pub(super) fn coeff() -> BoxedStrategy<$repr> {
        let digits = (any::<bool>(), digit_text(DIGITS as usize)).prop_map(|(negative, text)| {
          let magnitude: BigInt = text.parse().expect("ASCII digits");
          let coeff = if negative && SIGNED {
            -magnitude
          } else {
            magnitude
          };
          <$repr as Repr>::from_big(&coeff).expect("at most DIGITS digits fit")
        });
        // Shifts keep random low bits, and a random sign for a signed storage, while bringing the
        // magnitude close to the decimal bound, which needs `bound_bits` bits.
        let bits = <$repr as Repr>::BITS;
        let bound_bits = max_coeff(DIGITS).bits() as u32;
        let value_bits = if SIGNED { bits - 1 } else { bits };
        let min_shift = value_bits.saturating_sub(bound_bits);
        let bits = (<$repr as Repr>::any(), min_shift..bits).prop_filter_map(
          "outside the decimal range",
          |(bits, shift)| {
            // An arithmetic shift, as `>>` is on a signed native integer.
            let coeff = bits.to_big() >> shift;
            <$repr as Repr>::from_big(&coeff).filter(|&coeff| T::from_repr(coeff).is_some())
          },
        );
        let edges = proptest::sample::select(edges());
        prop_oneof![digits, bits, edges].boxed()
      }

      pub(super) fn value() -> BoxedStrategy<T> {
        coeff()
          .prop_map(|coeff| T::from_repr(coeff).expect("coefficient in range"))
          .boxed()
      }

      /// Values of at most 15 significant digits: up to 15 random digits, then as many zeros as a
      /// random count that the type has room for.
      fn short_value() -> BoxedStrategy<T> {
        (digit_text((DIGITS as usize).min(15)), any::<bool>())
          .prop_flat_map(|(digits, negative)| {
            let room = DIGITS as usize - digits.len();
            (Just(digits), 0..=room, Just(negative))
          })
          .prop_map(|(digits, zeros, negative)| {
            let magnitude: BigInt = format!("{digits}{}", "0".repeat(zeros))
              .parse()
              .expect("ASCII digits");
            let coeff = if negative && SIGNED {
              -magnitude
            } else {
              magnitude
            };
            from_big(&coeff).expect("at most DIGITS digits fit")
          })
          .boxed()
      }

      proptest! {
        #![proptest_config(config())]

        #[test]
        fn from_repr_checks_the_range(bits in <$repr as Repr>::any()) {
          let expected = in_range(&bits.to_big(), DIGITS);
          prop_assert_eq!(T::from_repr(bits).is_some(), expected);
          if let Some(value) = T::from_repr(bits) {
            prop_assert_eq!(value.repr(), bits);
          }
        }

        #[test]
        fn arithmetic_matches_big_integers(a in value(), b in value()) {
          let (big_a, big_b) = (big(a), big(b));
          prop_assert_eq!(a.checked_add(&b).map(big), from_big(&(&big_a + &big_b)).map(big));
          prop_assert_eq!(a.checked_sub(&b).map(big), from_big(&(&big_a - &big_b)).map(big));
          prop_assert_eq!(a.cmp(&b), big_a.cmp(&big_b));
          prop_assert_eq!(a == b, big_a == big_b);
          prop_assert_eq!(a.is_zero(), big_a.sign() == Sign::NoSign);
          prop_assert_eq!(a.is_positive(), big_a.sign() == Sign::Plus);
          suite!(@signed_arithmetic $sign, a, big_a);
        }

        #[test]
        fn display_matches_the_oracle(a in value(), precision in 0..(FRACTION as usize + 4)) {
          let text = oracle_text(&big(a), FRACTION, 0);
          prop_assert_eq!(a.to_string(), text.clone());
          prop_assert_eq!(format!("{a:?}"), text);
          prop_assert_eq!(
            format!("{a:.precision$}"),
            oracle_text(&big(a), FRACTION, precision)
          );
        }

        #[test]
        fn text_round_trips(
          a in value(),
          precision in 0..(FRACTION as usize + 4),
          width in 0usize..48,
        ) {
          prop_assert_eq!(T::parse(&a.to_string()), Ok(a));
          prop_assert_eq!(format!("{a:.precision$}").parse::<T>(), Ok(a));
          // Zero padding goes after the sign, so the padded text is still valid input.
          prop_assert_eq!(format!("{a:0width$.precision$}").parse::<T>(), Ok(a));
          // A `+` sign is accepted, for every storage.
          prop_assert_eq!(format!("{a:+}").parse::<T>(), Ok(a));
          prop_assert_eq!(format!("{a:+0width$.precision$}").parse::<T>(), Ok(a));
        }

        #[test]
        fn exp_matches_the_oracle(a in value(), precision in 0..(DIGITS as usize + 3)) {
          prop_assert_eq!(
            format!("{a:e}"),
            oracle_exp_text(&big(a), FRACTION, 0, false)
          );
          prop_assert_eq!(
            format!("{a:E}"),
            oracle_exp_text(&big(a), FRACTION, 0, true)
          );
          prop_assert_eq!(
            format!("{a:.precision$e}"),
            oracle_exp_text(&big(a), FRACTION, precision, false)
          );
          prop_assert_eq!(
            format!("{a:.precision$E}"),
            oracle_exp_text(&big(a), FRACTION, precision, true)
          );
        }

        #[test]
        fn exp_text_round_trips(
          a in value(),
          precision in 0..(DIGITS as usize + 3),
          width in 0usize..48,
        ) {
          prop_assert_eq!(T::parse(&format!("{a:e}")), Ok(a));
          prop_assert_eq!(T::parse(&format!("{a:E}")), Ok(a));
          prop_assert_eq!(format!("{a:.precision$e}").parse::<T>(), Ok(a));
          prop_assert_eq!(format!("{a:.precision$E}").parse::<T>(), Ok(a));
          // Zero padding goes after the sign, so the padded text is still valid input.
          prop_assert_eq!(format!("{a:0width$.precision$e}").parse::<T>(), Ok(a));
          // A `+` sign is accepted, for every storage.
          prop_assert_eq!(format!("{a:+e}").parse::<T>(), Ok(a));
          prop_assert_eq!(format!("{a:+0width$.precision$E}").parse::<T>(), Ok(a));
        }

        /// `f64` prints the shortest digits that read back as the same `f64`, and a decimal of at
        /// most 15 significant digits is the shortest of its own `f64`, because decimals that
        /// short are farther apart than the spacing of `f64`: the texts must be equal. The `f64`
        /// is parsed from the decimal's text, which rounds correctly, where `powi` does not. From
        /// 16 digits the digits differ: 9007199254740993 is 2^53 + 1, whose nearest `f64` is 2^53,
        /// and prints as 9.007199254740992e15. Every exponent of a decimal is within the range of
        /// the normal `f64`s, and the comparison has no precision, which `f64` rounds.
        #[test]
        fn exp_matches_f64_for_short_values(a in short_value()) {
          let text = a.to_string();
          prop_assert!(significant_digits(&text) <= 15);
          let float: f64 = text.parse().expect("decimal text is a float");
          prop_assert_eq!(format!("{a:e}"), format!("{float:e}"));
          prop_assert_eq!(format!("{a:E}"), format!("{float:E}"));
        }

        #[test]
        fn text_round_trips_with_exponents(
          a in value(),
          shift in -(DIGITS as i32 + 3)..=(DIGITS as i32 + 3),
          plus in any::<bool>(),
          upper in any::<bool>(),
        ) {
          let text = if plus { format!("{a:+}") } else { a.to_string() };
          let text = with_exponent(&text, shift, if upper { 'E' } else { 'e' });
          prop_assert_eq!(T::parse(&text), Ok(a), "text {:?}", text);
          prop_assert_eq!(
            oracle_parse(&text, INTEGRAL, FRACTION, SIGNED),
            Ok(big(a)),
            "text {:?}",
            text
          );
        }

        #[test]
        fn parse_matches_the_oracle_on_numbers(
          input in number_text(INTEGRAL as usize + 3, FRACTION as usize + 3, DIGITS + 3),
        ) {
          let expected = oracle_parse(&input, INTEGRAL, FRACTION, SIGNED);
          let actual = T::parse(&input).map(big).map_err(|err| err.kind());
          prop_assert_eq!(actual, expected, "input {:?}", input);
        }

        #[test]
        fn parse_matches_the_oracle_on_nonfinite_text(input in nonfinite_text()) {
          let expected = oracle_parse(&input, INTEGRAL, FRACTION, SIGNED);
          let actual = T::parse(&input).map(big).map_err(|err| err.kind());
          prop_assert_eq!(actual, expected, "input {:?}", input);
        }

        #[test]
        fn parse_matches_the_oracle_on_tokens(input in token_text()) {
          let expected = oracle_parse(&input, INTEGRAL, FRACTION, SIGNED);
          let actual = T::parse(&input).map(big).map_err(|err| err.kind());
          prop_assert_eq!(actual, expected, "input {:?}", input);
        }

        #[test]
        fn parse_matches_the_oracle_on_digit_strings(
          negative in any::<bool>(),
          integral in digit_text(INTEGRAL as usize + 3),
          fraction in proptest::option::of(digit_text(FRACTION as usize + 3)),
        ) {
          let input = format!(
            "{}{integral}{}",
            if negative { "-" } else { "" },
            fraction.map(|digits| format!(".{digits}")).unwrap_or_default(),
          );
          let expected = oracle_parse(&input, INTEGRAL, FRACTION, SIGNED);
          let actual = T::parse(&input).map(big).map_err(|err| err.kind());
          prop_assert_eq!(actual, expected, "input {:?}", input);
        }

        #[test]
        fn parse_matches_the_oracle_on_random_text(
          input in "[-+0-9.eE_iInNfFaAsStTyY,x \t\n\u{663}\u{ff11}]{0,16}",
        ) {
          let expected = oracle_parse(&input, INTEGRAL, FRACTION, SIGNED);
          let actual = T::parse(&input).map(big).map_err(|err| err.kind());
          prop_assert_eq!(actual, expected, "input {:?}", input);
        }
      }
    }
  };

  (@signed signed) => {
    true
  };
  (@signed unsigned) => {
    false
  };

  (@signed_arithmetic signed, $a:ident, $big_a:ident) => {
    prop_assert_eq!(big($a.neg()), -&$big_a);
    prop_assert_eq!(big(-$a), -&$big_a);
    prop_assert_eq!(big($a.abs()), BigInt::from($big_a.magnitude().clone()));
    prop_assert_eq!($a.is_negative(), $big_a.sign() == Sign::Minus);
  };
  (@signed_arithmetic unsigned, $a:ident, $big_a:ident) => {};
}

suite!(d1x1: Decimal<U1, U1>, i8, signed, integral = 1, fraction = 1);
suite!(d0x2: Decimal<U0, U2>, i8, signed, integral = 0, fraction = 2);
suite!(d2x0: Decimal<U2, U0>, i8, signed, integral = 2, fraction = 0);
suite!(d3x1: Decimal<U3, U1>, i16, signed, integral = 3, fraction = 1);
suite!(d4x5: Decimal<U4, U5>, i32, signed, integral = 4, fraction = 5);
suite!(d10x8: Decimal<U10, U8>, i64, signed, integral = 10, fraction = 8);
suite!(d12x8: Decimal<U12, U8>, i128, signed, integral = 12, fraction = 8);
suite!(d19x0: Decimal<U19, U0>, i128, signed, integral = 19, fraction = 0);
suite!(d19x19: Decimal<U19, U19>, i128, signed, integral = 19, fraction = 19);
suite!(d38x0: Decimal<U38, U0>, i128, signed, integral = 38, fraction = 0);
suite!(d0x38: Decimal<U0, U38>, i128, signed, integral = 0, fraction = 38);
suite!(wide: Decimal<U10, U8, i128>, i128, signed, integral = 10, fraction = 8);
suite!(d2x2_in_i64: Decimal<U2, U2, i64>, i64, signed, integral = 2, fraction = 2);
suite!(d0x36: Decimal<U0, U36>, i128, signed, integral = 0, fraction = 36);

suite!(u1x1: Decimal<U1, U1, u8>, u8, unsigned, integral = 1, fraction = 1);
suite!(u0x2: Decimal<U0, U2, u8>, u8, unsigned, integral = 0, fraction = 2);
suite!(u3x1: Decimal<U3, U1, u16>, u16, unsigned, integral = 3, fraction = 1);
suite!(u4x5: Decimal<U4, U5, u32>, u32, unsigned, integral = 4, fraction = 5);
suite!(u10x0: Decimal<U10, U0, u64>, u64, unsigned, integral = 10, fraction = 0);
suite!(u10x9: Decimal<U10, U9, u64>, u64, unsigned, integral = 10, fraction = 9);
suite!(u19x0: Decimal<U19, U0, u64>, u64, unsigned, integral = 19, fraction = 0);
suite!(u19x19: Decimal<U19, U19, u128>, u128, unsigned, integral = 19, fraction = 19);
suite!(u38x0: Decimal<U38, U0, u128>, u128, unsigned, integral = 38, fraction = 0);

suite!(w20x19: Decimal<U20, U19>, i192, signed, integral = 20, fraction = 19);
suite!(w39x18: Decimal<U39, U18>, i192, signed, integral = 39, fraction = 18);
suite!(w58x18: Decimal<U58, U18>, i256, signed, integral = 58, fraction = 18);
suite!(w78x18: Decimal<U78, U18>, i320, signed, integral = 78, fraction = 18);
suite!(w96x0: Decimal<U96, U0>, i320, signed, integral = 96, fraction = 0);
suite!(w0x96: Decimal<U0, U96>, i320, signed, integral = 0, fraction = 96);
suite!(d2x2_in_i320: Decimal<U2, U2, i320>, i320, signed, integral = 2, fraction = 2);

suite!(uw20x19: Decimal<U20, U19, u192>, u192, unsigned, integral = 20, fraction = 19);
suite!(uw39x18: Decimal<U39, U18, u192>, u192, unsigned, integral = 39, fraction = 18);
suite!(uw59x18: Decimal<U59, U18, u256>, u256, unsigned, integral = 59, fraction = 18);
suite!(uw78x18: Decimal<U78, U18, u320>, u320, unsigned, integral = 78, fraction = 18);
suite!(uw96x0: Decimal<U96, U0, u320>, u320, unsigned, integral = 96, fraction = 0);
// Any uint256 amount with 18 fraction digits.
suite!(uint256: Decimal<U60, U18, u320>, u320, unsigned, integral = 60, fraction = 18);

// Each of the storages from 384 to 512 bits, at the fewest and the most digits that it holds.
suite!(w79x18: Decimal<U79, U18>, i384, signed, integral = 79, fraction = 18);
suite!(w97x18: Decimal<U97, U18>, i384, signed, integral = 97, fraction = 18);
suite!(w98x18: Decimal<U98, U18>, i448, signed, integral = 98, fraction = 18);
suite!(w116x18: Decimal<U116, U18>, i448, signed, integral = 116, fraction = 18);
suite!(w117x18: Decimal<U117, U18>, i512, signed, integral = 117, fraction = 18);
suite!(w135x18: Decimal<U135, U18>, i512, signed, integral = 135, fraction = 18);
suite!(w153x0: Decimal<U153, U0>, i512, signed, integral = 153, fraction = 0);
suite!(w0x153: Decimal<U0, U153>, i512, signed, integral = 0, fraction = 153);
suite!(d2x2_in_i512: Decimal<U2, U2, i512>, i512, signed, integral = 2, fraction = 2);

suite!(uw79x18: Decimal<U79, U18, u384>, u384, unsigned, integral = 79, fraction = 18);
suite!(uw97x18: Decimal<U97, U18, u384>, u384, unsigned, integral = 97, fraction = 18);
suite!(uw98x18: Decimal<U98, U18, u448>, u448, unsigned, integral = 98, fraction = 18);
suite!(uw116x18: Decimal<U116, U18, u448>, u448, unsigned, integral = 116, fraction = 18);
suite!(uw117x18: Decimal<U117, U18, u512>, u512, unsigned, integral = 117, fraction = 18);
suite!(uw135x18: Decimal<U135, U18, u512>, u512, unsigned, integral = 135, fraction = 18);
// 154 digits, one more than any signed storage holds.
suite!(uw154x0: Decimal<U154, U0, u512>, u512, unsigned, integral = 154, fraction = 0);
suite!(uw0x154: Decimal<U0, U154, u512>, u512, unsigned, integral = 0, fraction = 154);

// Operands for products and conversions.
suite!(d10x10: Decimal<U10, U10>, i128, signed, integral = 10, fraction = 10);
suite!(u10x10: Decimal<U10, U10, u128>, u128, unsigned, integral = 10, fraction = 10);
suite!(uw58x18: Decimal<U58, U18, u256>, u256, unsigned, integral = 58, fraction = 18);

// Operands for divisions: a budget, and a dividend whose quotient by a 38-digit integer needs a
// 68-digit intermediate.
suite!(d12x2: Decimal<U12, U2>, i64, signed, integral = 12, fraction = 2);
suite!(d1x30: Decimal<U1, U30>, i128, signed, integral = 1, fraction = 30);

/// The reference and the crate both agree with coefficients computed by hand, so that they cannot
/// share a mistake unnoticed.
#[test]
fn hand_computed_examples() {
  use ParseErrorKind::{Empty, Inexact, InvalidSyntax, Negative, NotFinite, OutOfRange};

  type Signed = Decimal<U4, U4>;
  type Unsigned = Decimal<U4, U4, u32>;

  // Coefficients for four integral and four fraction digits. An unsigned storage gives the same
  // result, except that a finite number with a leading `-` is negative, whatever its value.
  let cases: &[(&str, Result<i64, ParseErrorKind>)] = &[
    ("", Err(Empty)),
    (" ", Err(InvalidSyntax)),
    ("\t", Err(InvalidSyntax)),
    ("1_000.000_1", Ok(10_000_001)),
    ("1__0", Ok(100_000)),
    ("1_", Ok(10_000)),
    ("5_.5", Ok(55_000)),
    ("1e_0_1", Ok(100_000)),
    ("1e_1_0", Err(OutOfRange)),
    ("+.5", Ok(5_000)),
    ("5.", Ok(50_000)),
    (".5e3", Ok(5_000_000)),
    ("1e+_3", Ok(10_000_000)),
    ("1e_3", Ok(10_000_000)),
    ("1e3_", Ok(10_000_000)),
    ("-1e-4", Ok(-1)),
    ("1E-04", Ok(1)),
    ("0e-5", Ok(0)),
    ("-0", Ok(0)),
    ("-0e5", Ok(0)),
    ("0e999999999", Ok(0)),
    ("0.0001e4", Ok(10_000)),
    ("1000e-3", Ok(10_000)),
    ("1e999999999", Err(OutOfRange)),
    ("1e-999999999", Err(Inexact)),
    ("12345", Err(OutOfRange)),
    ("0.00001", Err(Inexact)),
    ("12345.00001", Err(OutOfRange)),
    ("-12345", Err(OutOfRange)),
    ("-0.00001", Err(Inexact)),
    ("inf", Err(NotFinite)),
    ("-Infinity", Err(NotFinite)),
    ("+sNaN", Err(NotFinite)),
    ("NaN123", Err(NotFinite)),
    ("NaN1_2", Err(InvalidSyntax)),
    ("infinit", Err(InvalidSyntax)),
    ("_1", Err(InvalidSyntax)),
    ("._5", Err(InvalidSyntax)),
    ("1._5", Err(InvalidSyntax)),
    ("1e_", Err(InvalidSyntax)),
    ("1e_+5", Err(InvalidSyntax)),
    (".", Err(InvalidSyntax)),
    ("+", Err(InvalidSyntax)),
    ("-", Err(InvalidSyntax)),
    ("--1", Err(InvalidSyntax)),
    ("e5", Err(InvalidSyntax)),
    ("1e", Err(InvalidSyntax)),
    ("1e+", Err(InvalidSyntax)),
    ("1,5", Err(InvalidSyntax)),
    ("0x1", Err(InvalidSyntax)),
    ("1e999_x", Err(InvalidSyntax)),
    ("-1e999_x", Err(InvalidSyntax)),
  ];
  for &(input, expected) in cases {
    let unsigned = match expected {
      Ok(_) | Err(OutOfRange | Inexact) if input.starts_with('-') => Err(Negative),
      other => other,
    };
    let expected = expected.map(BigInt::from);
    let unsigned = unsigned.map(BigInt::from);
    assert_eq!(oracle_parse(input, 4, 4, true), expected, "input {input:?}");
    assert_eq!(
      oracle_parse(input, 4, 4, false),
      unsigned,
      "input {input:?}"
    );
    let signed_result = Signed::parse(input).map(|value| BigInt::from(value.repr()));
    assert_eq!(
      signed_result.map_err(|err| err.kind()),
      expected,
      "input {input:?}"
    );
    let unsigned_result = Unsigned::parse(input).map(|value| BigInt::from(value.repr()));
    assert_eq!(
      unsigned_result.map_err(|err| err.kind()),
      unsigned,
      "input {input:?}"
    );
  }
}

macro_rules! exact_mul {
  ($name:ident: $lhs:ident * $rhs:ident = $product:ty) => {
    proptest! {
      #![proptest_config(config())]

      #[test]
      fn $name(a in $lhs::value(), b in $rhs::value()) {
        let product: $product = a.exact_mul(&b);
        let expected = $lhs::big(a) * $rhs::big(b);
        prop_assert_eq!(product.repr().to_big(), expected.clone());
        prop_assert_eq!(
          product.to_string(),
          oracle_text(&expected, $lhs::FRACTION + $rhs::FRACTION, 0)
        );
      }
    }
  };
}

exact_mul!(mul_i8_limit: d1x1 * d1x1 = Decimal<U2, U2>);
exact_mul!(mul_fraction_by_integral: d0x2 * d2x0 = Decimal<U2, U2>);
exact_mul!(mul_i16_by_i32: d3x1 * d4x5 = Decimal<U7, U6>);
exact_mul!(mul_price_by_quantity: d10x8 * d12x8 = Decimal<U22, U16>);
exact_mul!(mul_i128_limit: d19x0 * d19x0 = Decimal<U38, U0>);
exact_mul!(mul_fraction_limit: d0x2 * d0x36 = Decimal<U0, U38>);
exact_mul!(mul_explicit_storage: wide * d2x2_in_i64 = Decimal<U12, U10>);

// Two unsigned storages give the smallest unsigned storage for the product's digits.
exact_mul!(mul_u8_limit: u1x1 * u1x1 = Decimal<U2, U2, u16>);
exact_mul!(mul_u16_by_u32: u3x1 * u4x5 = Decimal<U7, U6, u64>);
exact_mul!(mul_u64_limit: u4x5 * u10x0 = Decimal<U14, U5, u64>);
exact_mul!(mul_u128_limit: u19x0 * u19x0 = Decimal<U38, U0, u128>);
exact_mul!(mul_unsigned_fraction: u10x9 * u0x2 = Decimal<U10, U11, u128>);
// A signed storage on either side gives the smallest signed storage.
exact_mul!(mul_signed_by_unsigned: d1x1 * u1x1 = Decimal<U2, U2, i16>);
exact_mul!(mul_unsigned_by_signed: u10x9 * d10x8 = Decimal<U20, U17, i128>);

// Two 38-digit native coefficients give a 76-digit product in a wide storage.
exact_mul!(mul_i128_squares_into_i256: d19x19 * d19x19 = Decimal<U38, U38>);
exact_mul!(mul_u128_squares_into_u256: u38x0 * u38x0 = Decimal<U76, U0, u256>);
exact_mul!(mul_u128_fractions_into_u256: u19x19 * u19x19 = Decimal<U38, U38, u256>);
// A native by a wide operand, in either order.
exact_mul!(mul_i64_by_i192: d10x8 * w39x18 = Decimal<U49, U26>);
exact_mul!(mul_i256_by_i8: w58x18 * d1x1 = Decimal<U59, U19>);
// Two wide operands, up to 96 digits.
exact_mul!(mul_i192_by_i192: w39x18 * w20x19 = Decimal<U59, U37>);
exact_mul!(mul_u192_by_u192: uw39x18 * uw20x19 = Decimal<U59, U37, u320>);
// A wide operand whose product type has a native storage.
exact_mul!(mul_explicit_wide_storage: d2x2_in_i320 * d1x1 = Decimal<U3, U3>);
// An unsigned and a signed operand give a signed product; two unsigned ones an unsigned product.
exact_mul!(mul_u256_by_i8: uw59x18 * d1x1 = Decimal<U60, U19>);
exact_mul!(mul_i64_by_u256: d10x8 * uw59x18 = Decimal<U69, U26>);
exact_mul!(mul_uint256_by_u8: uint256 * u1x1 = Decimal<U61, U19, u320>);

// Products of 97 to 153 digits, in the storages from 384 to 512 bits. A uint256 amount by a
// 20-digit price has 98 digits.
exact_mul!(mul_uint256_by_signed_price: uint256 * d10x10 = Decimal<U70, U28>);
exact_mul!(mul_uint256_by_unsigned_price: uint256 * u10x10 = Decimal<U70, U28, u384>);
exact_mul!(mul_into_i384_limit: w58x18 * w20x19 = Decimal<U78, U37>);
exact_mul!(mul_into_u384_limit: uw58x18 * uw20x19 = Decimal<U78, U37, u384>);
exact_mul!(mul_into_i448_start: w78x18 * d12x8 = Decimal<U90, U26>);
exact_mul!(mul_into_i448_limit: w58x18 * w39x18 = Decimal<U97, U36>);
exact_mul!(mul_into_u448_start: uw59x18 * uw20x19 = Decimal<U79, U37, u448>);
exact_mul!(mul_into_u448_limit: uw59x18 * uw39x18 = Decimal<U98, U36, u448>);
exact_mul!(mul_into_i512_start: w96x0 * w20x19 = Decimal<U116, U19>);
exact_mul!(mul_into_u512_start: uw96x0 * uw20x19 = Decimal<U116, U19, u512>);
// 77 digits by 76 digits, the widest product.
exact_mul!(mul_u256_by_i256_into_i512: uw59x18 * w58x18 = Decimal<U117, U36>);
exact_mul!(mul_u256_by_u256_into_u512: uw59x18 * uw58x18 = Decimal<U117, U36, u512>);
// A wide operand whose product is in a wider storage, and one whose product is native.
exact_mul!(mul_i384_by_i8: w97x18 * d1x1 = Decimal<U98, U19>);
exact_mul!(mul_u384_by_u8: uw97x18 * u1x1 = Decimal<U98, U19, u448>);
exact_mul!(mul_i512_by_i8: d2x2_in_i512 * d1x1 = Decimal<U3, U3>);

macro_rules! convert {
  ($name:ident: $source:ident -> $target:ident) => {
    proptest! {
      #![proptest_config(config())]

      #[test]
      fn $name(a in $source::value()) {
        let expected = oracle_convert(
          &$source::big(a),
          $source::FRACTION,
          $target::FRACTION,
          $target::DIGITS,
          $target::SIGNED,
        );
        let actual: Result<$target::T, ConvertError> = a.try_convert();
        prop_assert_eq!(actual.map($target::big), expected);
      }
    }
  };
}

convert!(convert_to_more_integral_digits: d10x8 -> d12x8);
convert!(convert_to_wider_storage: d10x8 -> wide);
convert!(convert_to_fewer_integral_digits: d12x8 -> d10x8);
convert!(convert_to_narrower_storage: wide -> d2x2_in_i64);
convert!(convert_i128_limit_to_i8_limit: d19x19 -> d1x1);
convert!(convert_i8_limit_to_i128_limit: d1x1 -> d19x19);
convert!(convert_to_fewer_fraction_digits: d0x38 -> d0x2);
convert!(convert_to_more_fraction_digits: d0x2 -> d0x38);
convert!(convert_integral_to_fraction: d38x0 -> d0x38);
convert!(convert_fraction_to_integral: d0x38 -> d38x0);
convert!(convert_mixed: d4x5 -> d3x1);

convert!(convert_signed_to_unsigned: d10x8 -> u10x9);
convert!(convert_signed_fraction_to_unsigned: d0x38 -> u1x1);
convert!(convert_signed_integral_to_unsigned: d38x0 -> u19x0);
convert!(convert_unsigned_to_signed: u19x0 -> d19x0);
convert!(convert_unsigned_to_narrower_signed: u10x9 -> d10x8);
convert!(convert_unsigned_to_fewer_digits: u19x19 -> u1x1);
convert!(convert_unsigned_to_more_digits: u1x1 -> u19x19);
convert!(convert_unsigned_integral_to_fraction: u38x0 -> u19x19);

// From a wide storage to a native one, scaling up and down.
convert!(convert_i192_to_i128_with_more_fraction_digits: w39x18 -> d19x19);
convert!(convert_i256_to_i64: w58x18 -> d10x8);
convert!(convert_u320_to_u64: uw78x18 -> u10x9);
convert!(convert_i320_to_u64: w78x18 -> u19x0);
convert!(convert_i320_fraction_to_i8: w0x96 -> d1x1);
// Between wide storages.
convert!(convert_i320_to_i192: w78x18 -> w39x18);
convert!(convert_i320_integral_to_fraction: w96x0 -> w0x96);
convert!(convert_i320_fraction_to_integral: w0x96 -> w96x0);
convert!(convert_uint256_to_u256: uint256 -> uw59x18);
convert!(convert_i320_to_u320: w78x18 -> uw78x18);
convert!(convert_u320_to_i320: uw78x18 -> w78x18);
convert!(convert_i256_to_u192: w58x18 -> uw39x18);
convert!(convert_u256_to_i256: uw59x18 -> w58x18);
// From a native storage to a wide one.
convert!(convert_i128_to_i192_with_fewer_fraction_digits: d19x19 -> w39x18);
convert!(convert_i8_to_i320_fraction: d1x1 -> w0x96);
convert!(convert_u128_to_i320: u38x0 -> w96x0);
convert!(convert_i64_to_u192: d10x8 -> uw39x18);
convert!(convert_i8_to_explicit_i320: d1x1 -> d2x2_in_i320);

// Between the storages from 384 to 512 bits, and with every other wide storage.
convert!(convert_i384_to_i448: w97x18 -> w116x18);
convert!(convert_i448_to_i384: w116x18 -> w97x18);
convert!(convert_i512_to_i448: w135x18 -> w116x18);
convert!(convert_i384_to_i512: w97x18 -> w135x18);
convert!(convert_u384_to_u448: uw97x18 -> uw116x18);
convert!(convert_u512_to_u384: uw135x18 -> uw97x18);
convert!(convert_u448_to_u512: uw116x18 -> uw135x18);
convert!(convert_i320_to_i384: w78x18 -> w97x18);
convert!(convert_i384_to_i320: w97x18 -> w78x18);
convert!(convert_u320_to_u384: uw78x18 -> uw97x18);
convert!(convert_u384_to_u320: uw97x18 -> uw78x18);
convert!(convert_u256_to_u512: uw59x18 -> uw135x18);
convert!(convert_i512_to_i192: w135x18 -> w39x18);
convert!(convert_i192_to_i512: w39x18 -> w135x18);
// Between signs: the widest unsigned storage holds one digit more than the widest signed one.
convert!(convert_i512_to_u512: w135x18 -> uw135x18);
convert!(convert_u512_to_i512: uw135x18 -> w135x18);
convert!(convert_u512_limit_to_i512: uw154x0 -> w153x0);
convert!(convert_i512_limit_to_u512: w153x0 -> uw154x0);
convert!(convert_i448_to_u384: w116x18 -> uw97x18);
convert!(convert_u448_to_i384: uw116x18 -> w97x18);
// Integral digits to fraction digits and back, by the most digits.
convert!(convert_i512_integral_to_fraction: w153x0 -> w0x153);
convert!(convert_i512_fraction_to_integral: w0x153 -> w153x0);
convert!(convert_u512_integral_to_fraction: uw154x0 -> uw0x154);
convert!(convert_u512_fraction_to_integral: uw0x154 -> uw154x0);
// With the native storages, scaling up and down.
convert!(convert_i512_to_i8: d2x2_in_i512 -> d1x1);
convert!(convert_i8_to_i512: d1x1 -> d2x2_in_i512);
convert!(convert_i448_to_i64: w116x18 -> d10x8);
convert!(convert_i64_to_i448: d10x8 -> w116x18);
convert!(convert_u512_to_u64: uw135x18 -> u10x9);
convert!(convert_u128_to_u512_fraction: u19x19 -> uw0x154);
convert!(convert_i8_fraction_to_i512_fraction: d0x2 -> w0x153);
convert!(convert_i512_fraction_to_i8_fraction: w0x153 -> d0x2);

macro_rules! widen {
  ($name:ident: $source:ident -> $target:ident) => {
    proptest! {
      #![proptest_config(config())]

      #[test]
      fn $name(a in $source::value()) {
        let widened: $target::T = a.widen();
        let expected = $source::big(a) * pow10($target::FRACTION - $source::FRACTION);
        prop_assert_eq!($target::big(widened), expected);
        prop_assert_eq!(widened.to_string(), a.to_string());
      }
    }
  };
}

widen!(widen_integral_digits: d10x8 -> d12x8);
widen!(widen_storage: d10x8 -> wide);
widen!(widen_i8_to_i128_limit: d1x1 -> d19x19);
widen!(widen_fraction_digits: d0x2 -> d0x38);
widen!(widen_integral_only: d2x0 -> d38x0);
widen!(widen_both: d2x2_in_i64 -> d4x5);

widen!(widen_u8_to_i8: u1x1 -> d1x1);
widen!(widen_u64_limit_to_i128: u19x0 -> d19x0);
widen!(widen_unsigned_to_signed: u10x9 -> d19x19);
widen!(widen_unsigned_storage: u10x9 -> u19x19);
widen!(widen_unsigned_fraction_digits: u0x2 -> u19x19);

// From a native storage to a wide one.
widen!(widen_i64_to_i192: d10x8 -> w39x18);
widen!(widen_i128_integral_to_i320: d38x0 -> w96x0);
widen!(widen_i128_fraction_to_i320: d0x38 -> w0x96);
widen!(widen_u128_to_uint256: u38x0 -> uint256);
widen!(widen_u64_to_i256: u19x0 -> w58x18);
widen!(widen_i8_to_explicit_i320: d1x1 -> d2x2_in_i320);
// Between wide storages, also from unsigned to signed.
widen!(widen_i192_to_i256: w39x18 -> w58x18);
widen!(widen_i256_to_i320: w58x18 -> w78x18);
widen!(widen_u192_to_u256: uw39x18 -> uw59x18);
widen!(widen_u256_to_u320: uw59x18 -> uw78x18);
widen!(widen_u256_to_uint256: uw59x18 -> uint256);
widen!(widen_u192_to_i256: uw39x18 -> w58x18);
widen!(widen_u256_to_i320: uw59x18 -> w78x18);

// Into and between the storages from 384 to 512 bits.
widen!(widen_i64_to_i384: d10x8 -> w97x18);
widen!(widen_i128_integral_to_i512: d38x0 -> w153x0);
widen!(widen_i128_fraction_to_i512: d0x38 -> w0x153);
widen!(widen_u128_to_u512: u38x0 -> uw154x0);
widen!(widen_i8_to_explicit_i512: d1x1 -> d2x2_in_i512);
widen!(widen_i320_to_i384: w78x18 -> w97x18);
widen!(widen_u320_to_u384: uw78x18 -> uw97x18);
widen!(widen_i256_to_i448: w58x18 -> w116x18);
widen!(widen_u256_to_u512: uw59x18 -> uw135x18);
widen!(widen_i384_to_i448: w97x18 -> w116x18);
widen!(widen_i448_to_i512: w116x18 -> w135x18);
widen!(widen_u384_to_u448: uw97x18 -> uw116x18);
widen!(widen_u448_to_u512: uw116x18 -> uw135x18);
widen!(widen_u192_to_i512: uw39x18 -> w135x18);
widen!(widen_u384_to_i448: uw97x18 -> w116x18);
widen!(widen_u512_to_i512: uw135x18 -> w135x18);
widen!(widen_u128_to_i512: u38x0 -> w153x0);
widen!(widen_i8_fraction_to_i512: d0x2 -> w0x153);

/// uint256 amounts of every magnitude, as four limbs, least significant first.
fn uint256_limbs() -> impl Strategy<Value = [u64; 4]> {
  let edges = proptest::sample::select(vec![
    [0; 4],
    [1, 0, 0, 0],
    [u64::MAX; 4],
    [0, 0, 0, 1 << 63],
    [u64::MAX, u64::MAX, u64::MAX, 0],
  ]);
  let shifted = (any::<[u64; 4]>(), 0u32..256).prop_map(|(limbs, shift)| {
    big_to_limbs(&(limbs_to_big(&limbs, false) >> shift), false).expect("a shift keeps the range")
  });
  prop_oneof![any::<[u64; 4]>(), shifted, edges]
}

proptest! {
  #![proptest_config(config())]

  /// Every uint256 amount is a `Decimal<U60, U18, u320>` exactly, built from its limbs with a zero
  /// limb on top or parsed from its digits with the exponent `-18`.
  #[test]
  fn every_uint256_amount_fits(limbs in uint256_limbs()) {
    let raw = limbs_to_big(&limbs, false);
    let [l0, l1, l2, l3] = limbs;
    let amount = uint256::T::from_repr(u320::from_limbs([l0, l1, l2, l3, 0]));
    prop_assert!(amount.is_some());
    let amount = amount.expect("checked above");
    prop_assert_eq!(uint256::big(amount), raw.clone());
    prop_assert_eq!(amount.to_string(), oracle_text(&raw, 18, 0));
    prop_assert_eq!(uint256::T::parse(&format!("{raw}e-18")), Ok(amount));
  }
}

/// Renders a random `f64`, including NaN, the infinities and subnormals, with one of the four
/// formatting traits the standard library implements for it.
fn f64_text() -> impl Strategy<Value = String> {
  (any::<f64>(), 0..4u8).prop_map(|(value, variant)| match variant {
    0 => format!("{value}"),
    1 => format!("{value:?}"),
    2 => format!("{value:e}"),
    _ => format!("{value:E}"),
  })
}

/// Whether `input`, after an optional sign, is `snan` followed by any number of ASCII digits or
/// `nan` followed by at least one, ignoring ASCII case: the General Decimal Arithmetic NaN forms
/// that `f64::from_str` has no syntax for (it accepts a bare `nan`, but neither `snan` nor a digit
/// payload).
fn gda_only_nan_form(input: &str) -> bool {
  let body = input
    .strip_prefix('-')
    .or_else(|| input.strip_prefix('+'))
    .unwrap_or(input);
  let lower = body.to_ascii_lowercase();
  match lower.strip_prefix("snan") {
    Some(payload) => payload.bytes().all(|b| b.is_ascii_digit()),
    None => lower
      .strip_prefix("nan")
      .is_some_and(|payload| !payload.is_empty() && payload.bytes().all(|b| b.is_ascii_digit())),
  }
}

/// `word`, with each letter's case chosen independently at random.
fn random_case(word: &'static str) -> impl Strategy<Value = String> {
  proptest::collection::vec(any::<bool>(), word.len()).prop_map(move |upper| {
    word
      .chars()
      .zip(upper)
      .map(|(c, upper)| if upper { c.to_ascii_uppercase() } else { c })
      .collect()
  })
}

/// One of `f64::from_str`'s number forms, with no `_`: 1-5 mantissa digits biased toward `0` and
/// `9`, as `D`, `D.`, `D.D` or `.D`, with an optional exponent of `e`/`E`, an optional sign, and
/// 1-3 digits.
fn f64_grammar_number() -> impl Strategy<Value = String> {
  let mantissa = prop_oneof![
    digit_text(5),
    digit_text(5).prop_map(|d| format!("{d}.")),
    (digit_text(5), digit_text(5)).prop_map(|(i, f)| format!("{i}.{f}")),
    digit_text(5).prop_map(|d| format!(".{d}")),
  ];
  let exponent = (
    prop_oneof![Just('e'), Just('E')],
    prop_oneof![Just(""), Just("+"), Just("-")],
    digit_text(3),
  )
    .prop_map(|(marker, sign, digits)| format!("{marker}{sign}{digits}"));
  (mantissa, proptest::option::of(exponent))
    .prop_map(|(mantissa, exponent)| format!("{mantissa}{}", exponent.unwrap_or_default()))
}

/// Strings built directly from the number grammar `f64::from_str`'s documentation describes: an
/// optional sign, then either `inf`, `infinity` or `nan` in random letter case, or a number. Never
/// contains `_`.
fn f64_grammar_text() -> impl Strategy<Value = String> {
  let word = prop_oneof![
    random_case("inf"),
    random_case("infinity"),
    random_case("nan")
  ];
  (
    prop_oneof![Just(""), Just("+"), Just("-")],
    prop_oneof![word, f64_grammar_number()],
  )
    .prop_map(|(sign, body)| format!("{sign}{body}"))
}

/// One byte a near miss may insert: digits, the point, an exponent marker, a sign, a space, or a
/// letter from one of the three words; never `_`, which `f64` rejects regardless of position.
fn near_miss_byte() -> impl Strategy<Value = u8> {
  proptest::sample::select(b"0123456789.eE+- naifs".to_vec())
}

/// An index into a string of length `len`: for deleting or duplicating the byte there (`len`
/// itself is out of bounds), or, with `inclusive`, for inserting before it (`len` means
/// appending). Weighted toward both ends, because an edit in the middle of a digit run almost
/// always changes only the value, while the start and the end are where this grammar's syntax
/// actually breaks.
fn edit_index(len: usize, inclusive: bool) -> impl Strategy<Value = usize> {
  let max = if inclusive { len } else { len - 1 };
  prop_oneof![2 => Just(0), 2 => Just(max), 1 => 0..=max]
}

/// An `f64_grammar_text()` string with exactly one byte deleted, inserted or duplicated. Many
/// results are invalid by one character (`1e`, `1..5`, `+`, `e5`, ...), which is what property (b)
/// below needs. `f64_grammar_text()` never produces an empty string, so every index is valid.
fn near_miss_text() -> impl Strategy<Value = String> {
  f64_grammar_text().prop_flat_map(|text| {
    let len = text.len();
    (
      edit_index(len, false),
      edit_index(len, true),
      0u8..3,
      near_miss_byte(),
    )
      .prop_map(move |(at, insert_at, kind, byte)| {
        let mut bytes = text.as_bytes().to_vec();
        match kind {
          0 => {
            bytes.remove(at);
          }
          1 => bytes.insert(insert_at, byte),
          _ => {
            let duplicated = bytes[at];
            bytes.insert(at, duplicated);
          }
        }
        String::from_utf8(bytes).expect("ASCII bytes")
      })
  })
}

proptest! {
  #![proptest_config(config())]

  /// `Decimal::parse` accepts every syntactic form that `f64::from_str` accepts; checked directly
  /// against the standard library, outside the per-type suites so this runs once, not once per
  /// suite.
  #[test]
  fn parse_accepts_every_f64_form(
    input in prop_oneof![
      token_text().prop_map(|text| text.replace('_', "")),
      f64_text(),
      f64_grammar_text(),
      near_miss_text(),
    ]
  ) {
    type Signed = Decimal<U2, U2>;
    type Unsigned = Decimal<U2, U2, u16>;

    let f64_ok = input.parse::<f64>().is_ok();

    let signed_result = Signed::parse(&input).map_err(|err| err.kind());
    let unsigned_result = Unsigned::parse(&input).map_err(|err| err.kind());

    if f64_ok {
      prop_assert!(
        !matches!(signed_result, Err(ParseErrorKind::InvalidSyntax)),
        "input {:?}",
        input
      );
      prop_assert!(
        !matches!(unsigned_result, Err(ParseErrorKind::InvalidSyntax)),
        "input {:?}",
        input
      );
    }

    // The unsigned storage gives the result of the signed one, except that a finite number with a
    // leading `-` is negative, whatever its value.
    let negative = input.starts_with('-')
      && matches!(
        signed_result,
        Ok(_) | Err(ParseErrorKind::OutOfRange | ParseErrorKind::Inexact)
      );
    if negative {
      prop_assert_eq!(
        unsigned_result,
        Err(ParseErrorKind::Negative),
        "input {:?}",
        input
      );
    } else {
      prop_assert_eq!(
        unsigned_result.map(|value| i128::from(value.repr())),
        signed_result.map(|value| i128::from(value.repr())),
        "input {:?}",
        input
      );
    }

    // Beyond `_` separators, the crate's grammar is exactly `f64`'s forms plus the General Decimal
    // Arithmetic NaN payloads that `f64` lacks; a signed result beyond invalid syntax or an empty
    // input must fall into one of those two cases.
    let signed_recognized = !matches!(
      signed_result,
      Err(ParseErrorKind::InvalidSyntax) | Err(ParseErrorKind::Empty)
    );
    if signed_recognized {
      prop_assert!(f64_ok || gda_only_nan_form(&input), "input {:?}", input);
    }
  }
}

/// Every rounding mode.
const MODES: [RoundingMode; 7] = [
  RoundingMode::Up,
  RoundingMode::Down,
  RoundingMode::Ceiling,
  RoundingMode::Floor,
  RoundingMode::HalfUp,
  RoundingMode::HalfDown,
  RoundingMode::HalfEven,
];

/// The mode that rounds `-x` as `mode` rounds `x`, negated: `Ceiling` and `Floor` swap, and every
/// other mode depends only on the magnitude.
fn mirror(mode: RoundingMode) -> RoundingMode {
  match mode {
    RoundingMode::Ceiling => RoundingMode::Floor,
    RoundingMode::Floor => RoundingMode::Ceiling,
    other => other,
  }
}

/// `value / divisor` rounded to an integer with `mode`, for any nonzero divisor.
///
/// Picks between the integers below and above the exact quotient by their distances to it, scaled
/// by `|divisor|`, and breaks a tie by direction or parity, taking the direction from the sign of
/// the exact quotient. The crate instead makes the divisor positive and compares the remainder of a
/// division toward zero with its distance to the divisor.
fn oracle_round(value: &BigInt, divisor: &BigInt, mode: RoundingMode) -> BigInt {
  // `/` rounds toward zero, which is the floor unless the exact quotient is negative and not an
  // integer.
  let truncated = value / divisor;
  if &truncated * divisor == *value {
    return truncated;
  }
  let negative = (value.sign() == Sign::Minus) != (divisor.sign() == Sign::Minus);
  let floor = if negative { truncated - 1u8 } else { truncated };
  let ceiling = &floor + 1u8;
  // `(q - floor) * |divisor|` and `(ceiling - q) * |divisor|` for the exact quotient `q`.
  let below = (value - &floor * divisor).magnitude().clone();
  let above = (&ceiling * divisor - value).magnitude().clone();
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
        std::cmp::Ordering::Less => floor,
        std::cmp::Ordering::Greater => ceiling,
        std::cmp::Ordering::Equal => match mode {
          RoundingMode::HalfUp => away_from_zero,
          RoundingMode::HalfDown => toward_zero,
          _ if (&floor % 2u8).sign() == Sign::NoSign => floor,
          _ => ceiling,
        },
      }
    }
  }
}

/// `coeff` with `digits` digits in total checked against a target with a signed or an unsigned
/// storage.
fn in_target(coeff: BigInt, digits: u32, signed: bool) -> Result<BigInt, ConvertError> {
  if in_range(&coeff, digits) && (signed || coeff.sign() != Sign::Minus) {
    Ok(coeff)
  } else {
    Err(ConvertError::OutOfRange)
  }
}

/// The result of rounding `coeff` from `from` to `to` fraction digits with `mode`, for a target
/// with `digits` digits in total and a signed or an unsigned storage. Without dropped digits, this
/// is the conversion.
fn oracle_round_to(
  coeff: &BigInt,
  from: u32,
  to: u32,
  digits: u32,
  signed: bool,
  mode: RoundingMode,
) -> Result<BigInt, ConvertError> {
  if to >= from {
    return oracle_convert(coeff, from, to, digits, signed);
  }
  in_target(oracle_round(coeff, &pow10(from - to), mode), digits, signed)
}

/// The same rounding by `bigdecimal`, from the text of the value.
fn bigdecimal_round_to(
  text: &str,
  to: u32,
  digits: u32,
  signed: bool,
  mode: RoundingMode,
) -> Result<BigInt, ConvertError> {
  let mode = match mode {
    RoundingMode::Up => bigdecimal::RoundingMode::Up,
    RoundingMode::Down => bigdecimal::RoundingMode::Down,
    RoundingMode::Ceiling => bigdecimal::RoundingMode::Ceiling,
    RoundingMode::Floor => bigdecimal::RoundingMode::Floor,
    RoundingMode::HalfUp => bigdecimal::RoundingMode::HalfUp,
    RoundingMode::HalfDown => bigdecimal::RoundingMode::HalfDown,
    RoundingMode::HalfEven => bigdecimal::RoundingMode::HalfEven,
  };
  let value: BigDecimal = text.parse().expect("decimal text");
  let (coeff, scale) = value
    .with_scale_round(i64::from(to), mode)
    .into_bigint_and_scale();
  assert_eq!(scale, i64::from(to));
  // `bigdecimal` has its own version of `num-bigint`, so the coefficient crosses as text.
  let coeff: BigInt = coeff.to_string().parse().expect("integer text");
  in_target(coeff, digits, signed)
}

/// The multiple of `step` that `mode` selects for `coeff`, with `digits` digits in total.
fn oracle_quantize(
  coeff: &BigInt,
  step: &BigInt,
  mode: RoundingMode,
  digits: u32,
) -> Result<BigInt, QuantizeError> {
  if step.sign() != Sign::Plus {
    return Err(QuantizeError::InvalidStep);
  }
  let multiple = oracle_round(coeff, step, mode) * step;
  if in_range(&multiple, digits) {
    Ok(multiple)
  } else {
    Err(QuantizeError::OutOfRange)
  }
}

/// Coefficients of at most `digits` digits whose last `dropped` digits are at or next to a tie:
/// zero, one, `5 * 10^(dropped - 1)` and its neighbors, and `10^dropped - 1`, under random higher
/// digits, which give both parities, and with a random sign if `signed`.
fn near_ties(digits: u32, dropped: u32, signed: bool) -> impl Strategy<Value = BigInt> {
  let unit = pow10(dropped);
  let half = &unit / 2u8;
  let lows = vec![
    BigInt::ZERO,
    BigInt::from(1u8),
    &half - 1u8,
    half.clone(),
    &half + 1u8,
    &unit - 1u8,
  ];
  let high = if digits > dropped {
    digit_text((digits - dropped) as usize)
      .prop_map(|text| text.parse::<BigInt>().expect("ASCII digits"))
      .boxed()
  } else {
    Just(BigInt::ZERO).boxed()
  };
  (high, proptest::sample::select(lows), any::<bool>()).prop_map(move |(high, low, negative)| {
    let magnitude = high * &unit + low;
    if negative && signed {
      -magnitude
    } else {
      magnitude
    }
  })
}

/// Values of a suite for rounding away `dropped` fraction digits: any value, or one at or next to a
/// tie.
fn rounding_values<T: Clone + std::fmt::Debug + 'static>(
  value: BoxedStrategy<T>,
  from_big: fn(&BigInt) -> Option<T>,
  digits: u32,
  dropped: u32,
  signed: bool,
) -> BoxedStrategy<T> {
  if dropped == 0 {
    return value;
  }
  let ties = near_ties(digits, dropped, signed)
    .prop_filter_map("outside the decimal range", move |coeff| from_big(&coeff));
  prop_oneof![value, ties].boxed()
}

macro_rules! round {
  ($name:ident: $source:ident -> $target:ident) => {
    proptest! {
      #![proptest_config(config())]

      #[test]
      fn $name(
        a in rounding_values(
          $source::value(),
          $source::from_big,
          $source::DIGITS,
          $source::FRACTION.saturating_sub($target::FRACTION),
          $source::SIGNED,
        ),
      ) {
        let big_a = $source::big(a);
        let text = a.to_string();
        let negated = $source::from_big(&-&big_a);
        for mode in MODES {
          let rounded: Result<$target::T, ConvertError> = a.round_to(mode);
          let rounded = rounded.map($target::big);
          let expected = oracle_round_to(
            &big_a,
            $source::FRACTION,
            $target::FRACTION,
            $target::DIGITS,
            $target::SIGNED,
            mode,
          );
          prop_assert_eq!(&rounded, &expected, "{} {:?}", text, mode);
          let decimal =
            bigdecimal_round_to(&text, $target::FRACTION, $target::DIGITS, $target::SIGNED, mode);
          prop_assert_eq!(&rounded, &decimal, "{} {:?} against bigdecimal", text, mode);
          if $target::FRACTION >= $source::FRACTION {
            let converted: Result<$target::T, ConvertError> = a.try_convert();
            prop_assert_eq!(&rounded, &converted.map($target::big), "{} {:?}", text, mode);
          }
          // `round(-x, m) == -round(x, mirror(m))` between signed types.
          if let (true, Some(negated)) = ($target::SIGNED, negated) {
            let mirrored: Result<$target::T, ConvertError> = negated.round_to(mirror(mode));
            prop_assert_eq!(
              mirrored.map(|value| -$target::big(value)),
              rounded,
              "{} {:?}",
              text,
              mode
            );
          }
        }
      }
    }
  };
}

// One fraction digit and a few, between native storages of either sign.
round!(round_i8_one_digit: d1x1 -> d2x0);
round!(round_i8_fraction_one_digit: d0x2 -> d1x1);
round!(round_i8_fraction_two_digits: d0x2 -> d2x0);
round!(round_i32_to_i16: d4x5 -> d3x1);
round!(round_to_fewer_integral_digits: d10x8 -> d2x2_in_i64);
round!(round_i128_to_i64: d10x10 -> d10x8);
round!(round_unsigned_one_digit: u0x2 -> u1x1);
round!(round_u64_nine_digits: u10x9 -> u10x0);
round!(round_u128_to_u32: u10x10 -> u4x5);
// Without dropped digits, rounding is the conversion.
round!(round_same_fraction_fewer_integral_digits: d12x8 -> d10x8);
round!(round_same_fraction_more_integral_digits: d10x8 -> d12x8);
round!(round_to_more_fraction_digits: d1x1 -> d19x19);
// Up to 38 dropped digits, within and across the 19-digit steps of the limb operations.
round!(round_i128_eighteen_digits: d19x19 -> d1x1);
round!(round_i128_nineteen_digits: d19x19 -> d19x0);
round!(round_i128_thirty_six_digits: d0x38 -> d0x2);
round!(round_i128_thirty_seven_digits: d0x38 -> d1x1);
round!(round_i128_thirty_eight_digits: d0x38 -> d38x0);
round!(round_i128_fraction_to_integral: d0x36 -> d2x0);
round!(round_unsigned_eighteen_digits: u19x19 -> u1x1);
// Between signs: a negative value converts to an unsigned storage only when it rounds to zero.
round!(round_unsigned_to_signed: u10x9 -> d10x8);
round!(round_signed_to_unsigned_without_dropped_digits: d10x8 -> u10x9);
round!(round_signed_fraction_to_unsigned: d0x38 -> u1x1);
round!(round_signed_to_unsigned_nineteen_digits: d19x19 -> u19x0);
round!(round_signed_wide_to_unsigned_wide: w78x18 -> uw78x18);
round!(round_signed_wide_to_unsigned_without_dropped_digits: w153x0 -> u1x1);
// From wide storages, by every number of dropped digits up to 154.
round!(round_i192_to_i64: w39x18 -> d10x8);
round!(round_i192_to_fewer_integral_digits: w20x19 -> d19x0);
round!(round_i192_one_digit: w20x19 -> w39x18);
round!(round_u192_one_digit: uw20x19 -> uw39x18);
round!(round_i128_to_i192_twenty_digits: d0x38 -> w39x18);
round!(round_i256_to_i8: w58x18 -> d2x0);
round!(round_u320_to_u64: uw78x18 -> u10x9);
round!(round_uint256_to_u64: uint256 -> u10x9);
round!(round_i320_seventy_eight_digits: w0x96 -> w39x18);
round!(round_i320_ninety_five_digits: w0x96 -> d1x1);
round!(round_i320_ninety_six_digits: w0x96 -> w96x0);
round!(round_i384_to_i128: w97x18 -> d12x8);
round!(round_u448_same_fraction: uw116x18 -> uw97x18);
round!(round_i512_same_fraction: w135x18 -> w116x18);
round!(round_i512_fewer_integral_digits: w117x18 -> w97x18);
round!(round_u512_to_i64: uw135x18 -> d10x8);
round!(round_i512_one_digit: d2x2_in_i512 -> d1x1);
round!(round_i512_one_hundred_fifty_one_digits: w0x153 -> d0x2);
round!(round_i512_one_hundred_fifty_two_digits: w0x153 -> d1x1);
round!(round_i512_one_hundred_fifty_three_digits: w0x153 -> w153x0);
round!(round_i512_to_more_fraction_digits: w0x153 -> uw0x154);
round!(round_u512_one_hundred_fifty_three_digits: uw0x154 -> u1x1);
round!(round_u512_one_hundred_fifty_four_digits: uw0x154 -> uw154x0);

/// How a test of `quantize` picks its step.
#[derive(Clone, Debug)]
enum Step {
  /// A fixed coefficient.
  Fixed(BigInt),
  /// The value itself.
  Value,
  /// The magnitude of the value plus this offset, so that the quotient is zero.
  AboveValue(BigInt),
}

/// Steps for values of `digits` digits: 1, 5, `25 * 10^k`, `10^k`, random values of one to three
/// digits, random coefficients from `coeff`, the value itself and steps above the value.
fn steps(digits: u32, coeff: BoxedStrategy<BigInt>) -> impl Strategy<Value = Step> {
  prop_oneof![
    Just(Step::Fixed(BigInt::from(1u8))),
    Just(Step::Fixed(BigInt::from(5u8))),
    (0..digits - 1).prop_map(|k| Step::Fixed(pow10(k) * 25u8)),
    (0..digits).prop_map(|k| Step::Fixed(pow10(k))),
    (1u32..1000).prop_map(|step| Step::Fixed(BigInt::from(step))),
    coeff.prop_map(Step::Fixed),
    Just(Step::Value),
    (1u32..1000).prop_map(|offset| Step::AboveValue(BigInt::from(offset))),
  ]
}

macro_rules! quantize {
  ($name:ident: $suite:ident) => {
    proptest! {
      #![proptest_config(config())]

      #[test]
      fn $name(
        a in $suite::value(),
        step in steps($suite::DIGITS, $suite::coeff().prop_map(|coeff| coeff.to_big()).boxed()),
        negative_step in proptest::bool::weighted(0.125),
      ) {
        use RoundingMode::{Down, HalfDown, HalfEven, HalfUp};

        let big_a = $suite::big(a);
        let big_step = match step {
          Step::Fixed(step) => step,
          Step::Value => big_a.clone(),
          Step::AboveValue(offset) => BigInt::from(big_a.magnitude().clone()) + offset,
        };
        let big_step = if negative_step { -big_step } else { big_step };
        let Some(step) = $suite::from_big(&big_step) else {
          // Outside the range of the type.
          return Ok(());
        };

        for mode in MODES {
          prop_assert_eq!(
            a.quantize(&step, mode).map($suite::big),
            oracle_quantize(&big_a, &big_step, mode, $suite::DIGITS),
            "{} by {} {:?}",
            a,
            step,
            mode
          );
          // The smallest unit leaves every value unchanged.
          let unit = $suite::from_big(&BigInt::from(1u8)).expect("one unit fits");
          prop_assert_eq!(a.quantize(&unit, mode), Ok(a));
        }
        let multiple = if big_step.sign() == Sign::NoSign {
          big_a.sign() == Sign::NoSign
        } else {
          (&big_a % &big_step).sign() == Sign::NoSign
        };
        prop_assert_eq!(a.is_multiple_of(&step), multiple, "{} by {}", a, step);
        if big_step.sign() == Sign::Plus {
          prop_assert_eq!(a.is_multiple_of(&step), a.quantize(&step, Down) == Ok(a));
          if multiple {
            for mode in MODES {
              prop_assert_eq!(a.quantize(&step, mode), Ok(a));
            }
          }
          // An odd step has no ties, so the half modes agree.
          if (&big_step % 2u8).sign() != Sign::NoSign {
            let half_up = a.quantize(&step, HalfUp);
            prop_assert_eq!(half_up, a.quantize(&step, HalfDown));
            prop_assert_eq!(half_up, a.quantize(&step, HalfEven));
          }
          // Below the step, the quotient is zero, and the result is zero or one step.
          if big_a.magnitude() < big_step.magnitude() {
            for mode in MODES {
              if let Ok(rounded) = a.quantize(&step, mode) {
                let rounded = $suite::big(rounded);
                prop_assert!(
                  rounded.sign() == Sign::NoSign || rounded.magnitude() == big_step.magnitude()
                );
              }
            }
          }
        }
        // `quantize(-x, m) == -quantize(x, mirror(m))` for a signed type.
        if let Some(negated) = $suite::from_big(&-&big_a) {
          for mode in MODES {
            prop_assert_eq!(
              negated.quantize(&step, mirror(mode)).map(|value| -$suite::big(value)),
              a.quantize(&step, mode).map($suite::big),
              "{} by {} {:?}",
              a,
              step,
              mode
            );
          }
        }
      }
    }
  };
}

quantize!(quantize_i8: d1x1);
quantize!(quantize_i8_fraction: d0x2);
quantize!(quantize_i8_integral: d2x0);
quantize!(quantize_i32: d4x5);
quantize!(quantize_i64: d10x8);
quantize!(quantize_i128: d19x19);
quantize!(quantize_i128_integral: d38x0);
quantize!(quantize_i128_fraction: d0x38);
quantize!(quantize_u8: u1x1);
quantize!(quantize_u8_fraction: u0x2);
quantize!(quantize_u64: u10x9);
quantize!(quantize_u64_limit: u19x0);
quantize!(quantize_u128: u19x19);
quantize!(quantize_u128_integral: u38x0);
quantize!(quantize_i192: w20x19);
quantize!(quantize_i192_limit: w39x18);
quantize!(quantize_i256: w58x18);
quantize!(quantize_i320_integral: w96x0);
quantize!(quantize_i320_fraction: w0x96);
quantize!(quantize_i512: w135x18);
quantize!(quantize_i512_integral: w153x0);
quantize!(quantize_i512_fraction: w0x153);
quantize!(quantize_few_digits_in_i512: d2x2_in_i512);
quantize!(quantize_u192: uw39x18);
quantize!(quantize_uint256: uint256);
quantize!(quantize_u512: uw135x18);
quantize!(quantize_u512_integral: uw154x0);
quantize!(quantize_u512_fraction: uw0x154);

/// The coefficient of the quotient `numerator / denominator` rounded with `mode`, checked against a
/// target with `digits` digits in total and a signed or an unsigned storage.
///
/// The caller scales both operands: `a * 10^(F2 + F3)` by `b * 10^F`, where the crate scales one.
fn oracle_div_round(
  numerator: &BigInt,
  denominator: &BigInt,
  digits: u32,
  signed: bool,
  mode: RoundingMode,
) -> Result<BigInt, DivideError> {
  if denominator.sign() == Sign::NoSign {
    return Err(DivideError::DivisionByZero);
  }
  in_target(oracle_round(numerator, denominator, mode), digits, signed)
    .map_err(|_| DivideError::OutOfRange)
}

/// The fraction digits, the largest coefficient and the sign of an operand's type.
#[derive(Clone, Debug)]
struct Operand {
  fraction: u32,
  max: BigInt,
  signed: bool,
}

impl Operand {
  /// Zero, one and two units and the two largest coefficients, also negated for a signed type.
  fn extremes(&self) -> Vec<BigInt> {
    let mut values = vec![
      BigInt::ZERO,
      BigInt::from(1u8),
      BigInt::from(2u8),
      self.max.clone(),
      &self.max - 1u8,
    ];
    if self.signed {
      let negated: Vec<BigInt> = values.iter().map(|value| -value).collect();
      values.extend(negated);
    }
    values
  }
}

/// Coefficients of a dividend and a divisor whose quotient, in units of `10^-to`, is an integer or
/// halfway between two, or is moved from one by a unit of the dividend; `None` if no tie fits the
/// divisor's type.
///
/// With `k = F2 + to - F`, the quotient is `a * 10^k / b`. For `k >= 0`, `a = m * t` and
/// `b = 2^(k + 1) * t` give `m * 5^k / 2`; for `k < 0`, `a = m * t * 5 * 10^(-k - 1)` and `b = t`
/// give `m / 2`. Either is a tie for an odd `m` and an integer for an even one.
fn quotients_near_ties(
  dividend: &Operand,
  divisor: &Operand,
  to: u32,
) -> Option<BoxedStrategy<(BigInt, BigInt)>> {
  let k = i64::from(divisor.fraction) + i64::from(to) - i64::from(dividend.fraction);
  let (a_unit, b_unit) = if k >= 0 {
    (
      BigInt::from(1u8),
      BigInt::from(2u8).pow(k.unsigned_abs() as u32 + 1),
    )
  } else {
    (pow10(k.unsigned_abs() as u32 - 1) * 5u8, BigInt::from(1u8))
  };
  // `t` keeps `b` in range and leaves room in `a` for `m = 1` and one more unit.
  let max_t = (&divisor.max / &b_unit).min((&dividend.max - 1u8) / &a_unit);
  if max_t.sign() == Sign::NoSign {
    return None;
  }
  let (dividend, divisor) = (dividend.clone(), divisor.clone());
  let length = |max: &BigInt| max.to_string().len();
  let t = prop_oneof![digit_text(2), digit_text(length(&max_t))];
  let m = prop_oneof![digit_text(2), digit_text(length(&dividend.max))];
  let strategy = (t, m, any::<bool>(), -1i8..=1, any::<bool>(), any::<bool>()).prop_map(
    move |(t, m, tie, offset, negative_a, negative_b)| {
      let t = t.parse::<BigInt>().expect("ASCII digits") % &max_t + 1u8;
      let max_m = (&dividend.max - 1u8) / (&t * &a_unit);
      let mut m = m.parse::<BigInt>().expect("ASCII digits") % (&max_m + 1u8);
      // An odd `m` gives a tie and an even one an integer.
      if ((&m % 2u8).sign() == Sign::NoSign) == tie {
        m = if m < max_m { m + 1u8 } else { m - 1u8 };
      }
      let a = BigInt::from((m * &t * &a_unit + offset).magnitude().clone());
      let b = &t * &b_unit;
      (
        if negative_a && dividend.signed { -a } else { a },
        if negative_b && divisor.signed { -b } else { b },
      )
    },
  );
  Some(strategy.boxed())
}

/// Dividends and divisors for a division to `to` fraction digits: random coefficients, the extremes
/// of each type, and, where a tie fits the divisor, quotients at and next to ties and integers.
fn division_operands(
  dividends: BoxedStrategy<BigInt>,
  dividend: &Operand,
  divisors: BoxedStrategy<BigInt>,
  divisor: &Operand,
  to: u32,
) -> BoxedStrategy<(BigInt, BigInt)> {
  let a = prop_oneof![3 => dividends, 1 => proptest::sample::select(dividend.extremes())];
  let b = prop_oneof![3 => divisors, 1 => proptest::sample::select(divisor.extremes())];
  match quotients_near_ties(dividend, divisor, to) {
    Some(ties) => prop_oneof![2 => (a, b), 1 => ties].boxed(),
    None => (a, b).boxed(),
  }
}

macro_rules! div_round {
  ($name:ident: $lhs:ident / $rhs:ident -> $target:ident) => {
    proptest! {
      #![proptest_config(config())]

      #[test]
      fn $name(
        (big_a, big_b) in division_operands(
          $lhs::value().prop_map($lhs::big).boxed(),
          &Operand { fraction: $lhs::FRACTION, max: max_coeff($lhs::DIGITS), signed: $lhs::SIGNED },
          $rhs::value().prop_map($rhs::big).boxed(),
          &Operand { fraction: $rhs::FRACTION, max: max_coeff($rhs::DIGITS), signed: $rhs::SIGNED },
          $target::FRACTION,
        ),
      ) {
        let a = $lhs::from_big(&big_a).expect("a dividend in range");
        let b = $rhs::from_big(&big_b).expect("a divisor in range");
        let numerator = &big_a * pow10($rhs::FRACTION + $target::FRACTION);
        let denominator = &big_b * pow10($lhs::FRACTION);
        let negated_a = $lhs::from_big(&-&big_a);
        let negated_b = $rhs::from_big(&-&big_b);
        let one = $rhs::from_big(&pow10($rhs::FRACTION));
        for mode in MODES {
          let quotient: Result<$target::T, DivideError> = a.div_round(&b, mode);
          let quotient = quotient.map($target::big);
          let expected =
            oracle_div_round(&numerator, &denominator, $target::DIGITS, $target::SIGNED, mode);
          prop_assert_eq!(&quotient, &expected, "{} / {} {:?}", a, b, mode);
          // `div_round(-a, -b, m) == div_round(a, b, m)`.
          if let (Some(negated_a), Some(negated_b)) = (negated_a, negated_b) {
            let both: Result<$target::T, DivideError> = negated_a.div_round(&negated_b, mode);
            prop_assert_eq!(both.map($target::big), quotient.clone(), "{} / {} {:?}", a, b, mode);
          }
          // `div_round(-a, b, m) == -div_round(a, b, mirror(m))`, and the same for `-b`, into a
          // signed type.
          if let (true, Some(negated_a)) = ($target::SIGNED, negated_a) {
            let mirrored: Result<$target::T, DivideError> = negated_a.div_round(&b, mirror(mode));
            prop_assert_eq!(
              mirrored.map(|q| -$target::big(q)),
              quotient.clone(),
              "{} / {} {:?}",
              a,
              b,
              mode
            );
          }
          if let (true, Some(negated_b)) = ($target::SIGNED, negated_b) {
            let mirrored: Result<$target::T, DivideError> = a.div_round(&negated_b, mirror(mode));
            prop_assert_eq!(
              mirrored.map(|q| -$target::big(q)),
              quotient.clone(),
              "{} / {} {:?}",
              a,
              b,
              mode
            );
          }
          // Dividing by one rounds as `round_to` does.
          if let Some(one) = one {
            let divided: Result<$target::T, DivideError> = a.div_round(&one, mode);
            let rounded: Result<$target::T, ConvertError> = a.round_to(mode);
            prop_assert_eq!(
              divided.map($target::big),
              rounded.map($target::big).map_err(|_| DivideError::OutOfRange),
              "{} {:?}",
              a,
              mode
            );
          }
        }
      }
    }
  };
}

// Native operands with a native intermediate, of either sign, with the dividend scaled, neither or
// the divisor scaled, to fewer and to more fraction digits than the dividend has.
div_round!(div_i8_by_i8: d1x1 / d1x1 -> d1x1);
div_round!(div_without_scaling: d3x1 / d1x1 -> d2x0);
div_round!(div_to_fewer_fraction_digits: d4x5 / d3x1 -> d3x1);
div_round!(div_to_more_fraction_digits: d3x1 / d4x5 -> d4x5);
div_round!(div_fraction_by_integral: d0x2 / d2x0 -> d0x2);
div_round!(div_price_by_price: d10x8 / d10x8 -> d12x8);
div_round!(div_budget_by_price: d12x2 / d10x8 -> d12x8);
div_round!(div_wide_storage_native_intermediate: d2x2_in_i512 / d1x1 -> d2x2_in_i320);
// Two unsigned storages divide in an unsigned intermediate, and any other pair in a signed one. A
// negative quotient converts to an unsigned target only when it rounds to zero.
div_round!(div_u8_by_u8: u1x1 / u1x1 -> u1x1);
div_round!(div_unsigned: u10x9 / u10x9 -> u10x9);
div_round!(div_unsigned_by_signed: u10x9 / d10x8 -> d10x8);
div_round!(div_signed_into_unsigned: d10x8 / d10x8 -> u10x9);
div_round!(div_signed_by_unsigned_into_unsigned: d4x5 / u3x1 -> u4x5);
// Native operands with a wide intermediate: 39 digits with the dividend or the divisor scaled, and
// 68 digits, more than either operand or the target has.
div_round!(div_i128_into_i192: d19x19 / d1x1 -> w20x19);
div_round!(div_scaled_divisor_in_i192: d0x38 / d1x1 -> d2x0);
div_round!(div_i128_by_i128_in_i256: d1x30 / d38x0 -> d2x0);
div_round!(div_i8_by_i192: d1x1 / w20x19 -> d2x0);
// Wide operands, of either sign, into wide and native targets.
div_round!(div_i192_by_i192: w39x18 / w20x19 -> w39x18);
div_round!(div_u192_by_u192: uw39x18 / uw20x19 -> uw39x18);
div_round!(div_i64_by_i192: d10x8 / w39x18 -> d10x8);
div_round!(div_i256_by_i192_into_i64: w58x18 / w39x18 -> d10x8);
div_round!(div_u256_by_i192: uw59x18 / w20x19 -> w58x18);
div_round!(div_uint256_by_signed_price: uint256 / d10x10 -> uint256);
div_round!(div_uint256_by_unsigned_price: uint256 / u10x10 -> uint256);
div_round!(div_scaled_divisor_in_i384: w0x96 / d1x1 -> d2x0);
// The widest intermediates: 153 digits with the dividend scaled, neither or the divisor scaled, and
// 154 digits for two unsigned storages.
div_round!(div_i512_scaled_dividend: w97x18 / d0x38 -> w58x18);
div_round!(div_i512_without_scaling: w135x18 / d10x8 -> d10x10);
div_round!(div_i512_scaled_divisor: w0x153 / d1x1 -> d1x1);
div_round!(div_u512_scaled_dividend: uw135x18 / u10x9 -> u10x10);
div_round!(div_u512_without_scaling: uw154x0 / u10x0 -> uw154x0);
