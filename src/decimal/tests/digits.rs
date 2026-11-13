//! The parts of a decimal, `trunc`, `fract`, `floor`, `ceil` and `is_integer`, and the digits of
//! a value, `order_of_magnitude`, `significant_digits` and `fraction_digits`, with
//! `round_significant`, of every storage against the coefficients in `num-bigint`, as literals, at
//! the edges of the layouts and in constant contexts. The literals of rust-decimal's documentation
//! are kept apart from this module's own.
//!
//! A sweep checks each type at values built from digit strings, which vary one at a time the
//! number of digits, the digits themselves and the trailing zeros. Each type checks in a function
//! of its own, whose frame holds the values of that type alone: the function is `#[inline(never)]`,
//! since an optimized build would inline a function that has one call site, and the frame of the
//! test would then hold the values of every type.
use core::num::NonZero;
use std::{
  format,
  string::{String, ToString},
  vec::Vec,
};

use num_bigint::{BigInt, Sign};
use typenum::{
  U0, U1, U2, U4, U5, U8, U9, U10, U18, U19, U39, U58, U77, U78, U96, U97, U116, U135, U153, U154,
};

use crate::{
  ConvertError, Decimal, FixedDecimal, RoundingMode,
  bigint::{self, tier},
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

/// A type with room for the literals.
type Price = Decimal<U10, U8>;

// Counts of significant digits, written as constants.
const ONE: NonZero<u32> = NonZero::<u32>::MIN;
const TWO: NonZero<u32> = NonZero::new(2).unwrap();
const THREE: NonZero<u32> = NonZero::new(3).unwrap();
const FOUR: NonZero<u32> = NonZero::new(4).unwrap();
const FIVE: NonZero<u32> = NonZero::new(5).unwrap();
const SIX: NonZero<u32> = NonZero::new(6).unwrap();
const SEVEN: NonZero<u32> = NonZero::new(7).unwrap();

/// The value of `T` that `text` names, which must be in the range of `T`.
fn value<T: FixedDecimal>(text: &str) -> T {
  text.parse().unwrap_or_else(|err| panic!("{text:?}: {err}"))
}

/// The value of `T` whose coefficient is `coefficient`, which must be in the range of `T`.
fn of_coefficient<T: FixedDecimal>(coefficient: &BigInt) -> T {
  value(&format!("{coefficient}e-{}", T::FRACTION))
}

/// `len` digits, `1` to `9` in turn.
fn base_pattern(len: usize) -> String {
  (0..len)
    .map(|index| char::from(b'1' + (index % 9) as u8))
    .collect()
}

/// The values at which a sweep checks `T`, each with its coefficient: zero, the limits, and the
/// coefficients of digit strings that vary one thing at a time, each negated too for a signed
/// storage, and each once. Every length takes the base pattern; every other pattern and every
/// count of trailing zeros under the base pattern is taken at the length `DIGITS` and at one in
/// the middle. A string is taken only where the type has room for it. Under Miri the first five.
#[inline(never)]
fn sweep<T: FixedDecimal>() -> Vec<(T, BigInt)> {
  let digits = T::DIGITS as usize;
  let fraction = T::FRACTION as usize;
  let middle = (digits / 2).max(1);

  let mut patterns = Vec::from(["9".repeat(digits)]);
  for len in [1, 2, 3, middle, digits - 1, digits] {
    if (1..=digits).contains(&len) {
      patterns.push(base_pattern(len));
    }
  }
  for len in [middle, digits] {
    patterns.push("9".repeat(len));
    patterns.push(format!("1{}", "0".repeat(len - 1)));
    // A tie one place below the five: a digit, nines, and then the five.
    if len >= 3 {
      patterns.push(format!("1{}5", "9".repeat(len - 2)));
    }
    patterns.push(format!("5{}", "0".repeat(len - 1)));
    for zeros in [1, 2, fraction, fraction + 1] {
      if (1..len).contains(&zeros) {
        patterns.push(format!(
          "{}{}",
          base_pattern(len - zeros),
          "0".repeat(zeros)
        ));
      }
    }
  }

  let mut coefficients = Vec::from([BigInt::ZERO]);
  for pattern in &patterns {
    let magnitude: BigInt = pattern.parse().expect("digits");
    let negated = -&magnitude;
    for coefficient in [magnitude, negated] {
      let fits = T::SIGNED || coefficient.sign() != Sign::Minus;
      if fits && !coefficients.contains(&coefficient) {
        coefficients.push(coefficient);
      }
    }
  }
  if tier().is_miri() {
    coefficients.truncate(5);
  }
  coefficients
    .into_iter()
    .map(|coefficient| (of_coefficient::<T>(&coefficient), coefficient))
    .collect()
}

/// The value of `T` whose coefficient is `coefficient`, or `OutOfRange` if `T` does not hold it.
fn in_range<T: FixedDecimal>(coefficient: &BigInt) -> Result<T, ConvertError> {
  if coefficient.magnitude() < bigint::pow10(T::DIGITS).magnitude() {
    Ok(of_coefficient(coefficient))
  } else {
    Err(ConvertError::OutOfRange)
  }
}

/// Checks `trunc`, `fract`, `floor`, `ceil` and `is_integer`, the inherent `const fn`s, of each
/// type of the list at every value of its [`sweep`] against the coefficient in `num-bigint`: the
/// truncation, the floor division and its ceiling by `10^Fraction`, and the rest of the
/// coefficient. Each type checks in a function of its own.
macro_rules! parts_agree {
  ($($ty:ty),+ $(,)?) => {$({
    #[inline(never)]
    fn storage() {
      type T = $ty;
      let scale = bigint::pow10(T::FRACTION);
      let integral = |coefficient: &BigInt, mode| bigint::round(coefficient, &scale, mode) * &scale;
      for (value, coefficient) in sweep::<T>() {
        let trunc = integral(&coefficient, RoundingMode::Down);
        let fract = &coefficient - &trunc;
        assert_eq!(value.trunc(), of_coefficient::<T>(&trunc), "trunc of {value}");
        assert_eq!(value.fract(), of_coefficient::<T>(&fract), "fract of {value}");
        let floor = integral(&coefficient, RoundingMode::Floor);
        assert_eq!(value.floor(), in_range::<T>(&floor), "floor of {value}");
        let ceil = integral(&coefficient, RoundingMode::Ceiling);
        assert_eq!(value.ceil(), in_range::<T>(&ceil), "ceil of {value}");
        assert_eq!(value.is_integer(), fract == BigInt::ZERO, "{value}");
      }
    }
    storage();
  })+};
}

/// The five parts of one type of each storage against the coefficients in `num-bigint`, at every
/// value of the sweep; under Miri, of a signed native, an unsigned native and a wide storage.
#[test]
fn parts_match_the_coefficients() {
  parts_agree!(Di8, Du64, Di256);
  if tier().is_miri() {
    return;
  }
  parts_agree!(Di16, Di32, Di64, Di128, Di192, Di320, Di384, Di448, Di512);
  parts_agree!(
    Du8, Du16, Du32, Du128, Du192, Du256, Du320, Du384, Du448, Du512, D0x154
  );
}

/// The number of digits of `coefficient`, zero's one digit included, and the number of zeros that
/// end them, from its digit string in `num-bigint`.
fn digit_string(coefficient: &BigInt) -> (u32, u32) {
  let digits = coefficient.magnitude().to_string();
  let zeros = if digits == "0" {
    0
  } else {
    digits.len() - digits.trim_end_matches('0').len()
  };
  (digits.len() as u32, zeros as u32)
}

/// Checks `order_of_magnitude`, `significant_digits` and `fraction_digits`, the inherent
/// `const fn`s, of each type of the list at every value of its [`sweep`]: against the digit string
/// of the coefficient in `num-bigint`, against the texts that the type writes, the exponent and the
/// mantissa of `{:e}` and the fraction of `Display`, and against their identity, which holds for
/// zero too. Each type checks in a function of its own.
macro_rules! digits_agree {
  ($($ty:ty),+ $(,)?) => {$({
    #[inline(never)]
    fn storage() {
      type T = $ty;
      for (value, coefficient) in sweep::<T>() {
        let order = value.order_of_magnitude();
        let significant = value.significant_digits();
        let fraction = value.fraction_digits();

        let (digits, zeros) = digit_string(&coefficient);
        if coefficient == BigInt::ZERO {
          assert_eq!((order, significant, fraction), (0, 1, 0), "{value}");
        } else {
          assert_eq!(order, digits as i32 - 1 - T::FRACTION as i32, "{value}");
          assert_eq!(significant, digits - zeros, "{value}");
          assert_eq!(fraction, T::FRACTION.saturating_sub(zeros), "{value}");
        }

        let scientific = format!("{value:e}");
        let (mantissa, exponent) = scientific.split_once('e').expect("an exponent");
        assert_eq!(exponent.parse::<i32>(), Ok(order), "{value}");
        let mantissa_digits = mantissa.chars().filter(char::is_ascii_digit).count();
        assert_eq!(mantissa_digits, significant as usize, "{value}");
        let text = value.to_string();
        let shown = text.split_once('.').map_or(0, |(_, fraction)| fraction.len());
        assert_eq!(shown, fraction as usize, "{value}");

        let identity = (i64::from(significant) - 1 - i64::from(order)).max(0);
        assert_eq!(i64::from(fraction), identity, "{value}");
      }
    }
    storage();
  })+};
}

/// The three methods on the digits of one type of each storage, at every value of the sweep;
/// under Miri, of a signed native, an unsigned native and a wide storage.
#[test]
fn digits_match_the_digit_strings_and_the_texts() {
  digits_agree!(Di8, Du64, Di256);
  if tier().is_miri() {
    return;
  }
  digits_agree!(Di16, Di32, Di64, Di128, Di192, Di320, Di384, Di448, Di512);
  digits_agree!(
    Du8, Du16, Du32, Du128, Du192, Du256, Du320, Du384, Du448, Du512, D0x154
  );
}

/// The counts at which `round_significant` is checked for a coefficient of `digits` digits: 1, 2,
/// one below, at and one above the digits, and `u32::MAX`, each once and none of them zero; under
/// Miri, one and the digits.
fn counts(digits: u32) -> Vec<NonZero<u32>> {
  let candidates: &[u32] = if tier().is_miri() {
    &[1, digits]
  } else {
    &[1, 2, digits - 1, digits, digits + 1, u32::MAX]
  };
  let mut counts = Vec::new();
  for count in candidates.iter().copied().filter_map(NonZero::new) {
    if !counts.contains(&count) {
      counts.push(count);
    }
  }
  counts
}

/// The modes in which `round_significant` is checked: all eight; under Miri, two.
fn modes() -> &'static [RoundingMode] {
  if tier().is_miri() {
    &[RoundingMode::HalfEven, RoundingMode::Up05]
  } else {
    &bigint::MODES
  }
}

/// Checks `round_significant`, the inherent `const fn`, of each type of the list at every value
/// of its [`sweep`], at the [`counts`] of its coefficient and in the [`modes`], against the
/// coefficient in `num-bigint` rounded by `10^k`, where `k` is the number of digits that the count
/// drops, and the range of the type. Each type checks in a function of its own.
macro_rules! rounding_agrees {
  ($($ty:ty),+ $(,)?) => {$({
    #[inline(never)]
    fn storage() {
      type T = $ty;
      for (value, coefficient) in sweep::<T>() {
        let (len, _) = digit_string(&coefficient);
        for &digits in &counts(len) {
          for &mode in modes() {
            let expected = match len.checked_sub(digits.get()) {
              Some(place) if place > 0 => {
                let step = bigint::pow10(place);
                in_range::<T>(&(bigint::round(&coefficient, &step, mode) * &step))
              }
              _ => Ok(value),
            };
            assert_eq!(
              value.round_significant(digits, mode),
              expected,
              "{value} to {digits} digits, {mode}"
            );
          }
        }
      }
    }
    storage();
  })+};
}

/// `round_significant` of one type of each storage against the coefficients in `num-bigint`, at
/// every value and count of the sweep, in the eight modes; under Miri, of a signed native, an
/// unsigned native and a wide storage, with two counts and two modes.
#[test]
fn rounding_to_significant_digits_matches_the_coefficients() {
  rounding_agrees!(Di8, Du64, Di256);
  if tier().is_miri() {
    return;
  }
  rounding_agrees!(Di16, Di32, Di64, Di128, Di192, Di320, Di384, Di448, Di512);
  rounding_agrees!(
    Du8, Du16, Du32, Du128, Du192, Du256, Du320, Du384, Du448, Du512, D0x154
  );
}

/// The parts of values with and without a fraction, of either sign, and of the limits, whose
/// `floor` or `ceil` is out of range.
#[test]
fn parts_of_literals() {
  type T = Decimal<U2, U2>;
  // The value, its `trunc`, `fract`, `floor` and `ceil`, `None` where out of range, and whether
  // it is an integer.
  type Row = (
    &'static str,
    &'static str,
    &'static str,
    Option<&'static str>,
    Option<&'static str>,
    bool,
  );
  let rows: [Row; 9] = [
    ("1.9", "1", "0.9", Some("1"), Some("2"), false),
    ("-1.9", "-1", "-0.9", Some("-2"), Some("-1"), false),
    ("2", "2", "0", Some("2"), Some("2"), true),
    ("-2", "-2", "0", Some("-2"), Some("-2"), true),
    ("0.5", "0", "0.5", Some("0"), Some("1"), false),
    ("-0.5", "0", "-0.5", Some("-1"), Some("0"), false),
    ("0", "0", "0", Some("0"), Some("0"), true),
    ("99.99", "99", "0.99", Some("99"), None, false),
    ("-99.99", "-99", "-0.99", None, Some("-99"), false),
  ];
  let parsed = |text: Option<&str>| text.map(value::<T>).ok_or(ConvertError::OutOfRange);
  for (text, trunc, fract, floor, ceil, integer) in rows {
    let x: T = value(text);
    assert_eq!(x.trunc(), value(trunc), "trunc of {text}");
    assert_eq!(x.fract(), value(fract), "fract of {text}");
    assert_eq!(x.floor(), parsed(floor), "floor of {text}");
    assert_eq!(x.ceil(), parsed(ceil), "ceil of {text}");
    assert_eq!(x.is_integer(), integer, "{text}");
  }
  assert_eq!(T::MAX.to_string(), "99.99");
  assert_eq!(T::MIN.to_string(), "-99.99");
}

/// The parts in a type without integral digits, where one is out of range, in a type without
/// fraction digits, where every value is an integer, in an unsigned storage, whose `floor` never
/// fails, and at the limits of a wide storage.
#[test]
fn parts_at_the_edges_of_the_layouts() {
  type Fraction = Decimal<U0, U2>;
  let half: Fraction = value("0.5");
  assert_eq!(half.trunc(), Fraction::ZERO);
  assert_eq!(half.fract(), half);
  assert_eq!(half.floor(), Ok(Fraction::ZERO));
  assert_eq!(half.ceil(), Err(ConvertError::OutOfRange));
  assert!(!half.is_integer());
  let minus_half: Fraction = value("-0.5");
  assert_eq!(minus_half.trunc(), Fraction::ZERO);
  assert_eq!(minus_half.fract(), minus_half);
  assert_eq!(minus_half.floor(), Err(ConvertError::OutOfRange));
  assert_eq!(minus_half.ceil(), Ok(Fraction::ZERO));
  let zero = Fraction::ZERO;
  assert_eq!(
    (zero.trunc(), zero.fract(), zero.floor(), zero.ceil()),
    (zero, zero, Ok(zero), Ok(zero))
  );
  assert!(zero.is_integer());

  type Integral = Decimal<U2, U0>;
  for text in ["-99", "-5", "0", "7", "99"] {
    let x: Integral = value(text);
    assert_eq!(x.trunc(), x, "{text}");
    assert_eq!(x.fract(), Integral::ZERO, "{text}");
    assert_eq!(x.floor(), Ok(x), "{text}");
    assert_eq!(x.ceil(), Ok(x), "{text}");
    assert!(x.is_integer(), "{text}");
  }

  type Unsigned = Decimal<U2, U2, u16>;
  let x: Unsigned = value("1.9");
  assert_eq!(x.floor(), Ok(value("1")));
  assert_eq!(x.ceil(), Ok(value("2")));
  assert_eq!(Unsigned::MAX.floor(), Ok(value("99")));
  assert_eq!(Unsigned::MAX.ceil(), Err(ConvertError::OutOfRange));
  for (x, _) in sweep::<Unsigned>() {
    assert!(x.floor().is_ok(), "floor of {x}");
  }

  assert_eq!(Di192::MIN.floor(), Err(ConvertError::OutOfRange));
  assert_eq!(Di192::MAX.ceil(), Err(ConvertError::OutOfRange));
  assert_eq!(Di192::MIN.ceil(), Ok(Di192::MIN.trunc()));
  assert_eq!(Di192::MAX.floor(), Ok(Di192::MAX.trunc()));
}

/// The nine methods in constant contexts, in a signed native storage and in an unsigned wide one.
#[test]
fn methods_in_constant_contexts() {
  const fn price(text: &str) -> Price {
    match Price::parse(text) {
      Ok(price) => price,
      Err(_) => panic!("a price"),
    }
  }
  const CHANGE: Price = price("-64250.75");
  const PARTS: [Price; 2] = [CHANGE.trunc(), CHANGE.fract()];
  const ROUNDED: [Result<Price, ConvertError>; 3] =
    [CHANGE.floor(), CHANGE.ceil(), Price::MAX.ceil()];
  const INTEGERS: [bool; 2] = [CHANGE.is_integer(), Price::ONE.is_integer()];
  assert_eq!(PARTS, [price("-64250"), price("-0.75")]);
  assert_eq!(
    ROUNDED,
    [
      Ok(price("-64251")),
      Ok(price("-64250")),
      Err(ConvertError::OutOfRange)
    ]
  );
  assert_eq!(INTEGERS, [false, true]);
  const DIGITS: (i32, u32, u32) = (
    CHANGE.order_of_magnitude(),
    CHANGE.significant_digits(),
    CHANGE.fraction_digits(),
  );
  const SIGNIFICANT: [Result<Price, ConvertError>; 3] = [
    CHANGE.round_significant(FIVE, RoundingMode::HalfEven),
    CHANGE.round_significant(TWO, RoundingMode::Down),
    Price::MAX.round_significant(ONE, RoundingMode::HalfEven),
  ];
  assert_eq!(DIGITS, (4, 7, 2));
  assert_eq!(
    SIGNIFICANT,
    [
      Ok(price("-64251")),
      Ok(price("-64000")),
      Err(ConvertError::OutOfRange)
    ]
  );

  type Amount = Decimal<U39, U18, u192>;
  const AMOUNT: Amount = match Amount::parse("2.5") {
    Ok(amount) => amount,
    Err(_) => panic!("an amount"),
  };
  const AMOUNT_PARTS: [Amount; 2] = [AMOUNT.trunc(), AMOUNT.fract()];
  const AMOUNT_ROUNDED: [Result<Amount, ConvertError>; 2] = [AMOUNT.floor(), AMOUNT.ceil()];
  const AMOUNT_INTEGER: bool = AMOUNT.is_integer();
  assert_eq!(AMOUNT_PARTS, [value("2"), value("0.5")]);
  assert_eq!(AMOUNT_ROUNDED, [Ok(value("2")), Ok(value("3"))]);
  assert!(!AMOUNT_INTEGER);
  const AMOUNT_DIGITS: (i32, u32, u32) = (
    AMOUNT.order_of_magnitude(),
    AMOUNT.significant_digits(),
    AMOUNT.fraction_digits(),
  );
  const AMOUNT_SIGNIFICANT: [Result<Amount, ConvertError>; 2] = [
    AMOUNT.round_significant(ONE, RoundingMode::HalfEven),
    AMOUNT.round_significant(ONE, RoundingMode::Up),
  ];
  assert_eq!(AMOUNT_DIGITS, (0, 2, 1));
  assert_eq!(AMOUNT_SIGNIFICANT, [Ok(value("2")), Ok(value("3"))]);
}

/// Asserts the order of magnitude, the significant digits and the fraction digits of `value`.
#[track_caller]
fn assert_digits<T: FixedDecimal>(value: &T, order: i32, significant: u32, fraction: u32) {
  assert_eq!(
    (
      value.order_of_magnitude(),
      value.significant_digits(),
      value.fraction_digits()
    ),
    (order, significant, fraction),
    "{value}"
  );
}

/// The three methods on the digits of values with and without a fraction, below one, of zero, of
/// one unit of the last digit and of the limits, in types with and without integral and fraction
/// digits.
#[test]
fn digits_of_literals() {
  assert_digits(&value::<Price>("64250.5"), 4, 6, 1);
  assert_digits(&value::<Price>("-64250.5"), 4, 6, 1);
  assert_digits(&value::<Price>("0.00012"), -4, 2, 5);
  assert_digits(&value::<Price>("1500"), 3, 2, 0);
  assert_digits(&Price::ZERO, 0, 1, 0);
  assert_digits(&Price::ONE, 0, 1, 0);
  assert_digits(&value::<Price>("0.00000001"), -8, 1, 8);
  assert_digits(&Price::MAX, 9, 18, 8);

  type Fraction = Decimal<U0, U2>;
  assert_digits(&value::<Fraction>("0.1"), -1, 1, 1);
  assert_digits(&value::<Fraction>("0.01"), -2, 1, 2);

  assert_digits(&value::<D0x154>("1e-154"), -154, 1, 154);
  assert_digits(&D0x154::MAX, -1, 154, 154);
  assert_digits(&Du512::MAX, 153, 154, 0);
  assert_digits(&value::<Du512>("1e153"), 153, 1, 0);
  assert_digits(&Decimal::<U153, U0>::MIN, 152, 153, 0);
}

/// The results that rust-decimal documents for `round_sf` and `round_sf_with_strategy`, at its
/// values: its strategies `MidpointNearestEven`, `ToZero` and `AwayFromZero` are `HalfEven`,
/// `Down` and `Up` here.
#[test]
fn rounding_to_significant_digits_as_rust_decimal_documents() {
  let x: Price = value("305.459");
  for (digits, half_even, down) in [
    (ONE, "300", "300"),
    (TWO, "310", "300"),
    (THREE, "305", "305"),
    (FOUR, "305.5", "305.4"),
    (FIVE, "305.46", "305.45"),
    (SIX, "305.459", "305.459"),
    (SEVEN, "305.459", "305.459"),
  ] {
    assert_eq!(
      x.round_significant(digits, RoundingMode::HalfEven),
      Ok(value(half_even))
    );
    assert_eq!(
      x.round_significant(digits, RoundingMode::Down),
      Ok(value(down))
    );
  }
  let small: Price = value("0.012301");
  assert_eq!(
    small.round_significant(THREE, RoundingMode::HalfEven),
    Ok(value("0.0123"))
  );
  assert_eq!(
    small.round_significant(THREE, RoundingMode::Up),
    Ok(value("0.0124"))
  );
}

/// `round_significant` with `Up05`, with the modes that read the sign, after a carry, at ties, at
/// the limits, where the carry leaves the type, and with more digits than any value has.
#[test]
fn rounding_to_significant_digits_of_literals() {
  let x: Price = value("305.459");
  for (digits, rounded) in [
    (ONE, "300"),
    (TWO, "310"),
    (THREE, "306"),
    (FOUR, "305.4"),
    (FIVE, "305.46"),
    (SIX, "305.459"),
  ] {
    assert_eq!(
      x.round_significant(digits, RoundingMode::Up05),
      Ok(value(rounded)),
      "{digits}"
    );
  }

  let negative: Price = value("-305.459");
  for (mode, rounded) in [
    (RoundingMode::Floor, "-310"),
    (RoundingMode::Ceiling, "-300"),
    (RoundingMode::Up, "-310"),
    (RoundingMode::Down, "-300"),
  ] {
    assert_eq!(
      negative.round_significant(TWO, mode),
      Ok(value(rounded)),
      "{mode}"
    );
  }

  let carry: Price = value("999.96");
  assert_eq!(
    carry.round_significant(FOUR, RoundingMode::HalfEven),
    Ok(value("1000"))
  );
  assert_eq!(
    carry.round_significant(FOUR, RoundingMode::Down),
    Ok(value("999.9"))
  );

  let half_even =
    |text: &str, digits| value::<Price>(text).round_significant(digits, RoundingMode::HalfEven);
  assert_eq!(half_even("2.5", ONE), Ok(value("2")));
  assert_eq!(half_even("3.5", ONE), Ok(value("4")));
  assert_eq!(half_even("125", TWO), Ok(value("120")));
  assert_eq!(half_even("135", TWO), Ok(value("140")));

  let out = Err(ConvertError::OutOfRange);
  let seventeen = NonZero::new(17).expect("not zero");
  assert_eq!(
    Price::MAX.round_significant(ONE, RoundingMode::HalfEven),
    out
  );
  assert_eq!(
    Price::MAX.round_significant(ONE, RoundingMode::Down),
    Ok(value("9000000000"))
  );
  assert_eq!(
    Price::MAX.round_significant(seventeen, RoundingMode::HalfEven),
    out
  );
  assert_eq!(
    Price::MAX.round_significant(seventeen, RoundingMode::Down),
    Ok(value("9999999999.9999999"))
  );
  assert_eq!(Price::MIN.round_significant(ONE, RoundingMode::Floor), out);
  assert_eq!(
    Price::MIN.round_significant(ONE, RoundingMode::Ceiling),
    Ok(value("-9000000000"))
  );

  for x in [
    Price::ZERO,
    Price::MAX,
    Price::MIN,
    value("305.459"),
    value("-0.00000001"),
  ] {
    for mode in bigint::MODES {
      assert_eq!(
        x.round_significant(NonZero::<u32>::MAX, mode),
        Ok(x),
        "{x}, {mode}"
      );
    }
  }
}
