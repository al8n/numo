use core::{
  any::TypeId,
  fmt::{Debug, Display, LowerExp, UpperExp},
  hash::{BuildHasher, Hash},
  ops::{Add, BitAnd, Sub},
  str::FromStr,
};
use std::{
  collections::hash_map::RandomState,
  format,
  string::{String, ToString},
  vec::Vec,
};

use num_bigint::BigInt;
use typenum::{
  And, Diff, Max, Maximum, Sum, U0, U1, U2, U3, U4, U5, U6, U8, U9, U10, U11, U12, U15, U16, U17,
  U18, U19, U20, U21, U22, U26, U28, U30, U34, U37, U38, U39, U40, U49, U58, U59, U60, U70, U76,
  U77, U78, U96, U97, U116, U120, U135, U136, U143, U144, U153, U154, Unsigned,
};

use super::{ConvertError, Decimal, DivideError, ParseError, ParseErrorKind, QuantizeError};
use crate::{
  DivIntermediate, Layout, RoundingMode, Storage, bigint, i192, i256, i320, i384, i448, i512, u192,
  u256, u320, u384, u448, u512,
};

/// `i8` at its 2-digit limit.
type D1x1 = Decimal<U1, U1>;
/// `i8` without integral digits.
type D0x2 = Decimal<U0, U2>;
/// `i8` without fraction digits.
type D2x0 = Decimal<U2, U0>;
type D2x2 = Decimal<U2, U2>;
type D5x4 = Decimal<U5, U4>;
type Price = Decimal<U10, U8>;
type Quantity = Decimal<U12, U8>;
/// `i128` at its 38-digit limit.
type D19x19 = Decimal<U19, U19>;
type D19x0 = Decimal<U19, U0>;
type D38x0 = Decimal<U38, U0>;
type D0x38 = Decimal<U0, U38>;
type WidePrice = Decimal<U10, U8, i128>;

/// `u8` at its 2-digit limit.
type D1x1u8 = Decimal<U1, U1, u8>;
type D0x2u8 = Decimal<U0, U2, u8>;
type D2x0u8 = Decimal<U2, U0, u8>;
type D2x2u16 = Decimal<U2, U2, u16>;
type D5x4u32 = Decimal<U5, U4, u32>;
/// `u64` at its 19-digit limit, one digit more than `i64` holds.
type D10x9u64 = Decimal<U10, U9, u64>;
type D19x0u64 = Decimal<U19, U0, u64>;
/// `u128` at its 38-digit limit.
type D19x19u128 = Decimal<U19, U19, u128>;
type D38x0u128 = Decimal<U38, U0, u128>;

const TEN_POW_19: i128 = 10_000_000_000_000_000_000;
const MAX_38: i128 = 99_999_999_999_999_999_999_999_999_999_999_999_999;
const MAX_19_U64: u64 = 9_999_999_999_999_999_999;
const MAX_38_U128: u128 = 99_999_999_999_999_999_999_999_999_999_999_999_999;

fn nines(count: usize) -> String {
  "9".repeat(count)
}

fn zeros(count: usize) -> String {
  "0".repeat(count)
}

#[test]
fn storage_layout() {
  let _: i8 = D1x1::ZERO.repr();
  let _: i8 = D0x2::ZERO.repr();
  let _: i8 = D2x0::ZERO.repr();
  let _: i16 = D2x2::ZERO.repr();
  let _: i32 = D5x4::ZERO.repr();
  let _: i64 = Price::ZERO.repr();
  let _: i128 = Quantity::ZERO.repr();
  let _: i128 = D19x19::ZERO.repr();
  let _: i128 = WidePrice::ZERO.repr();
  let _: u8 = D1x1u8::ZERO.repr();
  let _: u16 = D2x2u16::ZERO.repr();
  let _: u32 = D5x4u32::ZERO.repr();
  let _: u64 = D10x9u64::ZERO.repr();
  let _: u128 = D19x19u128::ZERO.repr();

  assert_eq!(size_of::<D1x1>(), size_of::<i8>());
  assert_eq!(size_of::<D2x2>(), size_of::<i16>());
  assert_eq!(size_of::<D5x4>(), size_of::<i32>());
  assert_eq!(size_of::<Price>(), size_of::<i64>());
  assert_eq!(align_of::<Price>(), align_of::<i64>());
  assert_eq!(size_of::<Quantity>(), size_of::<i128>());
  assert_eq!(align_of::<WidePrice>(), align_of::<i128>());
  assert_eq!(size_of::<Option<Price>>(), size_of::<Option<i64>>());
  assert_eq!(size_of::<D1x1u8>(), size_of::<u8>());
  assert_eq!(size_of::<D10x9u64>(), size_of::<u64>());
  assert_eq!(align_of::<D10x9u64>(), align_of::<u64>());
  assert_eq!(size_of::<D19x19u128>(), size_of::<u128>());
}

#[test]
fn trait_surface() {
  fn assert_traits<
    T: Copy + Eq + Ord + Hash + Default + Debug + Display + FromStr + Send + Sync + Unpin + 'static,
  >() {
  }

  assert_traits::<D1x1>();
  assert_traits::<D0x2>();
  assert_traits::<Price>();
  assert_traits::<D19x19>();
  assert_traits::<WidePrice>();
  assert_traits::<D1x1u8>();
  assert_traits::<D2x2u16>();
  assert_traits::<D5x4u32>();
  assert_traits::<D10x9u64>();
  assert_traits::<D19x19u128>();
}

#[test]
fn generic_bounds() {
  // Storing and passing values needs no bounds.
  struct Holder<I, F, R> {
    value: Decimal<I, F, R>,
  }

  fn pass<I, F, R>(value: Decimal<I, F, R>) -> Decimal<I, F, R> {
    value
  }

  // `Storage` alone lets generic code copy, compare and hash values.
  fn storage_traits<I, F, R: Storage>() {
    fn assert<T: Copy + Ord + Hash>() {}
    assert::<Decimal<I, F, R>>();
  }

  // Formatting depends on the digit counts, so it also needs the layout.
  fn layout_traits<I, F, R>()
  where
    R: Storage,
    (I, F): Layout<R>,
  {
    fn assert<T: Debug + Display>() {}
    assert::<Decimal<I, F, R>>();
  }

  let holder = Holder {
    value: pass(Price::ONE),
  };
  assert_eq!(holder.value, Price::ONE);
  storage_traits::<U10, U8, i64>();
  layout_traits::<U10, U8, i64>();
  storage_traits::<U10, U9, u64>();
  layout_traits::<U10, U9, u64>();
}

#[test]
fn constants() {
  assert_eq!(D1x1::ZERO.repr(), 0);
  assert_eq!(D1x1::MAX.repr(), 99);
  assert_eq!(D1x1::MIN.repr(), -99);
  assert_eq!(D1x1::ONE.repr(), 10);
  assert_eq!(D0x2::MAX.repr(), 99);
  assert_eq!(D0x2::MIN.repr(), -99);
  assert_eq!(D2x0::MAX.repr(), 99);
  assert_eq!(D2x0::ONE.repr(), 1);
  assert_eq!(D2x2::MAX.repr(), 9_999);
  assert_eq!(D2x2::ONE.repr(), 100);
  assert_eq!(D5x4::MAX.repr(), 999_999_999);
  assert_eq!(Price::MAX.repr(), 999_999_999_999_999_999);
  assert_eq!(Price::ONE.repr(), 100_000_000);
  assert_eq!(WidePrice::MAX.repr(), 999_999_999_999_999_999);
  assert_eq!(D19x19::MAX.repr(), MAX_38);
  assert_eq!(D19x19::MIN.repr(), -MAX_38);
  assert_eq!(D19x19::ONE.repr(), TEN_POW_19);
  assert_eq!(D38x0::MAX.repr(), MAX_38);
  assert_eq!(D38x0::ONE.repr(), 1);
  assert_eq!(D0x38::MAX.repr(), MAX_38);

  assert_eq!(D1x1::MIN, D1x1::MAX.neg());
  assert_eq!(D19x19::MIN, -D19x19::MAX);
  assert_eq!(Price::default(), Price::ZERO);
  assert_eq!(D0x2::default(), D0x2::ZERO);

  assert_eq!(Price::INTEGRAL, 10);
  assert_eq!(Price::FRACTION, 8);
  assert_eq!(Price::DIGITS, 18);
  assert_eq!(D0x2::INTEGRAL, 0);
  assert_eq!(D0x2::DIGITS, 2);
  assert_eq!(D38x0::FRACTION, 0);
  assert_eq!([Price::SIGNED, WidePrice::SIGNED, D1x1::SIGNED], [true; 3]);
}

#[test]
fn sign_predicates() {
  for (value, zero, negative, positive) in [
    (Price::ZERO, true, false, false),
    (Price::ONE, false, false, true),
    (Price::MAX, false, false, true),
    (Price::MIN, false, true, false),
    (Price::from_repr(-1).unwrap(), false, true, false),
  ] {
    assert_eq!(value.is_zero(), zero);
    assert_eq!(value.is_negative(), negative);
    assert_eq!(value.is_positive(), positive);
  }
}

#[test]
fn from_repr_checks_the_decimal_range() {
  assert_eq!(D1x1::from_repr(99), Some(D1x1::MAX));
  assert_eq!(D1x1::from_repr(-99), Some(D1x1::MIN));
  assert_eq!(D1x1::from_repr(100), None);
  assert_eq!(D1x1::from_repr(-100), None);
  assert_eq!(D1x1::from_repr(i8::MAX), None);
  assert_eq!(D1x1::from_repr(i8::MIN), None);

  assert_eq!(D19x19::from_repr(MAX_38), Some(D19x19::MAX));
  assert_eq!(D19x19::from_repr(-MAX_38), Some(D19x19::MIN));
  assert_eq!(D19x19::from_repr(MAX_38 + 1), None);
  assert_eq!(D19x19::from_repr(-MAX_38 - 1), None);
  assert_eq!(D19x19::from_repr(i128::MAX), None);
  assert_eq!(D19x19::from_repr(i128::MIN), None);

  assert_eq!(WidePrice::from_repr(1_000_000_000_000_000_000), None);
  assert_eq!(
    WidePrice::from_repr(999_999_999_999_999_999),
    Some(WidePrice::MAX)
  );
}

#[test]
fn checked_add_and_sub_at_the_i8_limit() {
  let ulp = D1x1::from_repr(1).unwrap();
  // The coefficient 100 fits in `i8`, so only the decimal bound rejects it.
  assert_eq!(D1x1::MAX.checked_add(&ulp), None);
  assert_eq!(D1x1::MIN.checked_sub(&ulp), None);
  assert_eq!(ulp.checked_sub(&D1x1::MIN), None);
  // 99 + 99 overflows `i8` itself.
  assert_eq!(D1x1::MAX.checked_add(&D1x1::MAX), None);
  assert_eq!(D1x1::MIN.checked_add(&D1x1::MIN), None);
  assert_eq!(D1x1::MAX.checked_sub(&D1x1::MIN), None);

  assert_eq!(D1x1::MAX.checked_add(&D1x1::MIN), Some(D1x1::ZERO));
  assert_eq!(D1x1::MAX.checked_sub(&D1x1::MAX), Some(D1x1::ZERO));
  assert_eq!(D1x1::ZERO.checked_sub(&D1x1::MIN), Some(D1x1::MAX));
  assert_eq!(
    D1x1::MAX.checked_sub(&ulp),
    Some(D1x1::from_repr(98).unwrap())
  );

  assert_eq!(D2x0::MAX.checked_add(&D2x0::ONE), None);
  assert_eq!(D2x0::MIN.checked_sub(&D2x0::ONE), None);
  assert_eq!(
    D0x2::MAX.checked_add(&D0x2::from_repr(-1).unwrap()),
    D0x2::from_repr(98)
  );
}

#[test]
fn checked_add_and_sub_at_the_i128_limit() {
  let ulp = D19x19::from_repr(1).unwrap();
  // 10^38 fits in `i128`, so only the decimal bound rejects it.
  assert_eq!(D19x19::MAX.checked_add(&ulp), None);
  assert_eq!(D19x19::MIN.checked_sub(&ulp), None);
  // 2 * (10^38 - 1) overflows `i128` itself.
  assert_eq!(D19x19::MAX.checked_add(&D19x19::MAX), None);
  assert_eq!(D19x19::MIN.checked_add(&D19x19::MIN), None);
  assert_eq!(D19x19::MAX.checked_sub(&D19x19::MIN), None);
  assert_eq!(D19x19::MIN.checked_sub(&D19x19::MAX), None);

  assert_eq!(D19x19::MAX.checked_add(&D19x19::MIN), Some(D19x19::ZERO));
  assert_eq!(D38x0::MAX.checked_add(&D38x0::ONE), None);
  assert_eq!(
    D38x0::MAX.checked_sub(&D38x0::ONE),
    D38x0::from_repr(MAX_38 - 1)
  );
}

#[test]
fn checked_add_and_sub_values() {
  let a: Price = "1.5".parse().unwrap();
  let b: Price = "-2.25".parse().unwrap();
  assert_eq!(a.checked_add(&b), Some("-0.75".parse().unwrap()));
  assert_eq!(a.checked_sub(&b), Some("3.75".parse().unwrap()));
  assert_eq!(b.checked_sub(&a), Some("-3.75".parse().unwrap()));
  assert_eq!(a.checked_add(&Price::ZERO), Some(a));
}

#[test]
fn neg_and_abs() {
  assert_eq!(D1x1::MIN.neg(), D1x1::MAX);
  assert_eq!(D1x1::MIN.abs(), D1x1::MAX);
  assert_eq!(D1x1::MAX.abs(), D1x1::MAX);
  assert_eq!(D19x19::MIN.abs(), D19x19::MAX);
  assert_eq!(Price::ZERO.neg(), Price::ZERO);
  assert!(!Price::ZERO.neg().is_negative());

  let value: Price = "-12.5".parse().unwrap();
  assert_eq!(-value, value.neg());
  assert_eq!(-(-value), value);
  assert_eq!(value.abs().to_string(), "12.5");
  assert_eq!((-value).to_string(), "12.5");
}

#[test]
fn ordering_and_equality() {
  let mut values: Vec<D2x2> = ["0.01", "-99.99", "0", "99.99", "-0.01", "1", "-1"]
    .iter()
    .map(|s| s.parse().unwrap())
    .collect();
  values.sort();
  let sorted: Vec<String> = values.iter().map(ToString::to_string).collect();
  assert_eq!(sorted, ["-99.99", "-1", "-0.01", "0", "0.01", "1", "99.99"]);
  assert!(D19x19::MIN < D19x19::ZERO && D19x19::ZERO < D19x19::MAX);
  assert_eq!(
    "1.50".parse::<D2x2>().unwrap(),
    "001.5".parse::<D2x2>().unwrap()
  );
  assert_ne!(D2x2::ONE, D2x2::ONE.neg());
}

#[test]
fn equal_values_hash_equally() {
  let state = RandomState::new();
  let a: Price = "1.50".parse().unwrap();
  let b: Price = "0001.5000".parse().unwrap();
  assert_eq!(a, b);
  assert_eq!(state.hash_one(a), state.hash_one(b));
}

fn parse_kind<T: FromStr<Err = ParseError>>(input: &str) -> Option<ParseErrorKind> {
  input.parse::<T>().err().map(|err| err.kind())
}

#[test]
fn parse_valid_inputs() {
  let cases: &[(&str, i16)] = &[
    ("0", 0),
    ("-0", 0),
    ("-0.00", 0),
    ("00", 0),
    ("0.0", 0),
    ("1", 100),
    ("-1", -100),
    ("12.34", 1_234),
    ("-12.34", -1_234),
    ("12.3", 1_230),
    ("012.30", 1_230),
    ("0.01", 1),
    ("-0.01", -1),
    ("99.99", 9_999),
    ("-99.99", -9_999),
    ("1.230000", 123),
    ("00000000000000000000000000000000000000000099.99", 9_999),
    ("1.50000000000000000000000000000000000000000000000000", 150),
  ];
  for &(input, coeff) in cases {
    assert_eq!(
      D2x2::parse(input).map(D2x2::repr),
      Ok(coeff),
      "input {input:?}"
    );
    assert_eq!(input.parse::<D2x2>().map(D2x2::repr), Ok(coeff));
  }
  assert!(!D2x2::parse("-0").unwrap().is_negative());
}

#[test]
fn parse_rejects_invalid_syntax() {
  for input in [
    // No digits: a lone sign or point, or an exponent alone.
    "-",
    "+",
    ".",
    "+.",
    "-.",
    "e5",
    "E5",
    ".e5",
    "-e5",
    // Signs.
    "--1",
    "-+1",
    "+-1",
    "++1",
    "1-",
    "1+",
    "1.5-",
    // Whitespace, even alone; only the empty input is `Empty`.
    " ",
    "   ",
    "\t",
    "\n",
    "\r\n",
    " 1",
    "1 ",
    "1\n",
    "\t1",
    "1 000",
    "- 1",
    // Grouping separators, other bases and other suffixes.
    "1,5",
    "1,000",
    "1'000",
    "0x10",
    "0X1F",
    "0b1",
    "0o7",
    "1f",
    "1d1",
    "x",
    "1x",
    "1.5x",
    // Points and exponents.
    "1.2.3",
    "1..2",
    "..5",
    "1e",
    "1E",
    "1e+",
    "1e-",
    "1.e",
    "1e5.5",
    "1e1e1",
    "1ee1",
    "1e+-1",
    "1e--1",
    "1e 1",
    "1 e1",
    // Non-ASCII digits, spaces and signs.
    "\u{0}",
    "\u{ff11}",
    "1\u{ff10}",
    "\u{663}",
    "1.\u{663}",
    "1e\u{663}",
    "1\u{a0}000",
    "\u{2212}1",
  ] {
    assert_eq!(
      parse_kind::<D2x2>(input),
      Some(ParseErrorKind::InvalidSyntax),
      "input {input:?}"
    );
    assert_eq!(
      parse_kind::<D2x2u16>(input),
      Some(ParseErrorKind::InvalidSyntax),
      "input {input:?}"
    );
  }
  assert_eq!(parse_kind::<D2x2>(""), Some(ParseErrorKind::Empty));
  assert_eq!(parse_kind::<D2x2u16>(""), Some(ParseErrorKind::Empty));
}

#[test]
fn parse_signs_and_points() {
  let cases: &[(&str, i16)] = &[
    ("+0", 0),
    ("+1", 100),
    ("+12.34", 1_234),
    ("+0.01", 1),
    ("1.", 100),
    ("-1.", -100),
    ("+1.", 100),
    ("5.", 500),
    ("00.", 0),
    ("99.", 9_900),
    (".5", 50),
    ("-.5", -50),
    ("+.5", 50),
    (".01", 1),
    ("-.99", -99),
    ("-.0", 0),
    (".000", 0),
  ];
  for &(input, coeff) in cases {
    assert_eq!(
      D2x2::parse(input).map(D2x2::repr),
      Ok(coeff),
      "input {input:?}"
    );
  }
  assert!(!D2x2::parse("-.0").unwrap().is_negative());
}

#[test]
fn parse_exponents() {
  let cases: &[(&str, i16)] = &[
    ("1e0", 100),
    ("1E0", 100),
    ("1e+0", 100),
    ("1e-0", 100),
    ("1e00", 100),
    ("1e1", 1_000),
    ("1E+1", 1_000),
    ("1e-1", 10),
    ("1e-2", 1),
    ("-1e-2", -1),
    ("1.e1", 1_000),
    (".1e1", 100),
    ("0.0001e4", 100),
    ("1000e-3", 100),
    ("10e-1", 100),
    ("12.34e0", 1_234),
    ("1234e-2", 1_234),
    ("0.1234e2", 1_234),
    (".001234E4", 1_234),
    ("123400e-4", 1_234),
    ("-9999e-2", -9_999),
    ("9.999e1", 9_999),
    ("0.9999E+2", 9_999),
    ("+.5e-1", 5),
    ("5e-2", 5),
    ("0e0", 0),
    ("-0e5", 0),
    ("-0e-1", 0),
    ("0.000e3", 0),
  ];
  for &(input, coeff) in cases {
    assert_eq!(
      D2x2::parse(input).map(D2x2::repr),
      Ok(coeff),
      "input {input:?}"
    );
  }
  assert!(!D2x2::parse("-0e5").unwrap().is_negative());
  assert_eq!(D5x4::parse(".5e3").map(D5x4::repr), Ok(5_000_000));

  for (input, kind) in [
    (".5e3", ParseErrorKind::OutOfRange),
    ("1e2", ParseErrorKind::OutOfRange),
    ("-1E2", ParseErrorKind::OutOfRange),
    ("1000e-1", ParseErrorKind::OutOfRange),
    ("0.1e3", ParseErrorKind::OutOfRange),
    ("1e-3", ParseErrorKind::Inexact),
    ("9.9999e1", ParseErrorKind::Inexact),
    ("123.45e-1", ParseErrorKind::Inexact),
    ("-5e-3", ParseErrorKind::Inexact),
    // Out of range and inexact at once.
    ("12345e-2", ParseErrorKind::OutOfRange),
    ("1.0001e2", ParseErrorKind::OutOfRange),
  ] {
    assert_eq!(parse_kind::<D2x2>(input), Some(kind), "input {input:?}");
  }

  // Without integral or fraction digits.
  assert_eq!(D0x2::parse("5e-1").map(D0x2::repr), Ok(50));
  assert_eq!(D0x2::parse("1e-2").map(D0x2::repr), Ok(1));
  assert_eq!(D0x2::parse("10e-3").map(D0x2::repr), Ok(1));
  assert_eq!(parse_kind::<D0x2>("1e0"), Some(ParseErrorKind::OutOfRange));
  assert_eq!(parse_kind::<D0x2>("1e-3"), Some(ParseErrorKind::Inexact));
  assert_eq!(D2x0::parse("1e1").map(D2x0::repr), Ok(10));
  assert_eq!(D2x0::parse("990e-1").map(D2x0::repr), Ok(99));
  assert_eq!(D2x0::parse("0.5e1").map(D2x0::repr), Ok(5));
  assert_eq!(parse_kind::<D2x0>("1e2"), Some(ParseErrorKind::OutOfRange));
  assert_eq!(parse_kind::<D2x0>("5e-1"), Some(ParseErrorKind::Inexact));

  // At the 38-digit limit.
  assert_eq!(D38x0::parse("1e37").map(D38x0::repr), Ok(10i128.pow(37)));
  assert_eq!(D38x0::parse(&format!("{}0e-1", nines(38))), Ok(D38x0::MAX));
  assert_eq!(D38x0::parse(&format!("0.{}e38", nines(38))), Ok(D38x0::MAX));
  assert_eq!(
    parse_kind::<D38x0>("1e38"),
    Some(ParseErrorKind::OutOfRange)
  );
  assert_eq!(
    parse_kind::<D38x0>(&format!("{}e-1", nines(39))),
    Some(ParseErrorKind::Inexact)
  );
  assert_eq!(D0x38::parse("1e-38").map(D0x38::repr), Ok(1));
  assert_eq!(D0x38::parse(&format!("{}e-38", nines(38))), Ok(D0x38::MAX));
  assert_eq!(parse_kind::<D0x38>("1e-39"), Some(ParseErrorKind::Inexact));
  assert_eq!(
    D19x19::parse(&format!("{}e-19", nines(38))),
    Ok(D19x19::MAX)
  );
  assert_eq!(
    D19x19::parse(&format!("-{}e-19", nines(38))),
    Ok(D19x19::MIN)
  );
}

#[test]
fn parse_underscores() {
  // After the first digit of a run, `_` may appear anywhere, also repeated or at the end; in the
  // exponent, anywhere after the sign.
  let cases: &[(&str, i32)] = &[
    ("1_000.000_1", 10_000_001),
    ("1__0", 100_000),
    ("1_", 10_000),
    ("1___", 10_000),
    ("5_.5", 55_000),
    ("1_2_3_4_5.6_7_8_9", 123_456_789),
    ("0_0.0_0", 0),
    ("1.0_0_", 10_000),
    ("1_.", 10_000),
    ("-1_.5_", -15_000),
    ("+.5_", 5_000),
    ("1_e1", 100_000),
    ("1_.e1", 100_000),
    ("1.5_e1", 150_000),
    ("1e_1", 100_000),
    ("1e1_", 100_000),
    ("1e_0_1", 100_000),
    ("1e__1__", 100_000),
    ("1e+_1", 100_000),
    ("1e-_1", 1_000),
    ("1E-0_1", 1_000),
  ];
  for &(input, coeff) in cases {
    assert_eq!(
      D5x4::parse(input).map(D5x4::repr),
      Ok(coeff),
      "input {input:?}"
    );
  }
  assert_eq!(
    D19x19::parse("1e_1_0").map(D19x19::repr),
    Ok(10i128.pow(29))
  );

  for input in [
    "_", "__", "_1", "-_1", "+_1", "_.5", "._5", "1._5", "1.__5", "._", "1._", "1._e1", "_e1",
    "1e_", "1e__", "1e+_", "1e_+1", "1e_-1", "1_e", "1_e_", "1.5_e",
  ] {
    assert_eq!(
      parse_kind::<D5x4>(input),
      Some(ParseErrorKind::InvalidSyntax),
      "input {input:?}"
    );
  }

  // Long runs of `_` wherever they are allowed.
  let underscores = "_".repeat(10_000);
  assert_eq!(
    Price::parse(&format!(
      "1{underscores}.5{underscores}e{underscores}1{underscores}"
    ))
    .map(Price::repr),
    Ok(1_500_000_000)
  );
}

#[test]
fn parse_nonfinite() {
  for word in [
    "inf",
    "INF",
    "Inf",
    "iNf",
    "infinity",
    "Infinity",
    "INFINITY",
    "InFiNiTy",
    "nan",
    "NaN",
    "nAn",
    "NAN",
    "snan",
    "sNaN",
    "SNAN",
    "sNAN",
    "NaN12",
    "nan0",
    "NAN0123456789",
    "sNaN12",
    "snan007",
  ] {
    for sign in ["", "+", "-"] {
      let input = format!("{sign}{word}");
      assert_eq!(
        parse_kind::<D2x2>(&input),
        Some(ParseErrorKind::NotFinite),
        "input {input:?}"
      );
      // An unsigned storage rejects a leading `-` first.
      let unsigned = if sign == "-" {
        ParseErrorKind::InvalidSyntax
      } else {
        ParseErrorKind::NotFinite
      };
      assert_eq!(
        parse_kind::<D2x2u16>(&input),
        Some(unsigned),
        "input {input:?}"
      );
    }
  }
  assert_eq!(
    parse_kind::<Price>(&format!("NaN{}", nines(10_000))),
    Some(ParseErrorKind::NotFinite)
  );

  for input in [
    "i",
    "in",
    "n",
    "na",
    "s",
    "sn",
    "sna",
    "infi",
    "infin",
    "infinit",
    "infinityy",
    "infinite",
    "infs",
    "inf1",
    "inf_",
    "inf.",
    "infe1",
    "infinity1",
    "nan_1",
    "NaN1_2",
    "nan-1",
    "nan+1",
    "nan.1",
    "nane1",
    "nanx",
    "nan ",
    " nan",
    "inf ",
    " inf",
    "+-inf",
    "--inf",
    "++nan",
    "ssnan",
    "nnan",
    "naan",
    "sinf",
    "sinfinity",
    "snanf",
    "1inf",
    "1nan",
    "0nan",
    ".inf",
    "inf\u{0}",
    "NaN\u{663}",
    "\u{221e}",
    "-\u{221e}",
  ] {
    assert_eq!(
      parse_kind::<D2x2>(input),
      Some(ParseErrorKind::InvalidSyntax),
      "input {input:?}"
    );
    assert_eq!(
      parse_kind::<D2x2u16>(input),
      Some(ParseErrorKind::InvalidSyntax),
      "input {input:?}"
    );
  }
}

#[test]
fn parse_huge_exponents() {
  // The exponent saturates while parsing, and the result is still decided exactly.
  let u64_max = u64::MAX.to_string();
  let long = nines(10_000);
  for exponent in [
    "999999999",
    u64_max.as_str(),
    "18446744073709551616",
    long.as_str(),
  ] {
    assert_eq!(
      parse_kind::<D2x2>(&format!("1e{exponent}")),
      Some(ParseErrorKind::OutOfRange)
    );
    assert_eq!(
      parse_kind::<D2x2>(&format!("-1E+{exponent}")),
      Some(ParseErrorKind::OutOfRange)
    );
    assert_eq!(
      parse_kind::<D2x2>(&format!("1e-{exponent}")),
      Some(ParseErrorKind::Inexact)
    );
    assert_eq!(
      parse_kind::<D0x38>(&format!("-0.5e-{exponent}")),
      Some(ParseErrorKind::Inexact)
    );
    assert_eq!(
      parse_kind::<D38x0>(&format!("{}e{exponent}", nines(38))),
      Some(ParseErrorKind::OutOfRange)
    );
    assert_eq!(
      parse_kind::<D2x2u16>(&format!("1e-{exponent}")),
      Some(ParseErrorKind::Inexact)
    );
    // Zero is zero with any exponent.
    for input in [
      format!("0e{exponent}"),
      format!("-0e-{exponent}"),
      format!("0.000e+{exponent}"),
      format!("+.0e{exponent}"),
    ] {
      assert_eq!(D2x2::parse(&input), Ok(D2x2::ZERO), "input {input:?}");
      assert_eq!(D0x38::parse(&input), Ok(D0x38::ZERO), "input {input:?}");
    }
    assert_eq!(D2x2u16::parse(&format!("0e-{exponent}")), Ok(D2x2u16::ZERO));
  }

  // Leading zeros of the exponent do not saturate it.
  let long_zeros = zeros(10_000);
  assert_eq!(D2x2::parse(&format!("1e{long_zeros}")), Ok(D2x2::ONE));
  assert_eq!(
    D2x2::parse(&format!("1e{long_zeros}1")).map(D2x2::repr),
    Ok(1_000)
  );
  assert_eq!(
    D2x2::parse(&format!("1e-{long_zeros}2")).map(D2x2::repr),
    Ok(1)
  );
}

#[test]
fn parse_long_mantissas_with_exponents() {
  // An exponent cancels any number of zeros exactly.
  for count in [0, 1, 37, 38, 39, 1_000, 10_000] {
    let zeros = zeros(count);
    let one = format!("1{zeros}e-{count}");
    assert_eq!(D2x2::parse(&one), Ok(D2x2::ONE), "input {one:?}");
    assert_eq!(D2x2u16::parse(&one), Ok(D2x2u16::ONE), "input {one:?}");
    assert_eq!(
      D2x2::parse(&format!("0.{zeros}1e{}", count + 1)),
      Ok(D2x2::ONE)
    );
    assert_eq!(
      D2x2::parse(&format!("-.{zeros}1E+{}", count + 1)),
      Ok(D2x2::ONE.neg())
    );
    assert_eq!(
      D2x2::parse(&format!("1{zeros}e-{}", count + 2)).map(D2x2::repr),
      Ok(1)
    );
    assert_eq!(
      parse_kind::<D2x2>(&format!("1{zeros}e-{}", count + 3)),
      Some(ParseErrorKind::Inexact)
    );
    assert_eq!(
      parse_kind::<D2x2>(&format!("1{zeros}00e-{count}")),
      Some(ParseErrorKind::OutOfRange)
    );
    // The largest values, written with extra zeros.
    assert_eq!(
      D19x19::parse(&format!("{}{zeros}e-{}", nines(38), count + 19)),
      Ok(D19x19::MAX)
    );
    assert_eq!(
      D0x38::parse(&format!("0.{zeros}{}e{count}", nines(38))),
      Ok(D0x38::MAX)
    );
  }
}

#[test]
fn parse_range_and_exactness() {
  for (input, kind) in [
    ("100", ParseErrorKind::OutOfRange),
    ("-100", ParseErrorKind::OutOfRange),
    ("100.00", ParseErrorKind::OutOfRange),
    ("0100", ParseErrorKind::OutOfRange),
    ("1.234", ParseErrorKind::Inexact),
    ("-0.001", ParseErrorKind::Inexact),
    ("99.991", ParseErrorKind::Inexact),
    ("1.2300001", ParseErrorKind::Inexact),
  ] {
    assert_eq!(parse_kind::<D2x2>(input), Some(kind), "input {input:?}");
  }
}

#[test]
fn parse_error_priority() {
  // Syntax is checked over the whole input before range or exactness.
  assert_eq!(
    parse_kind::<D2x2>("99999x"),
    Some(ParseErrorKind::InvalidSyntax)
  );
  assert_eq!(
    parse_kind::<D2x2>("1.234x"),
    Some(ParseErrorKind::InvalidSyntax)
  );
  assert_eq!(
    parse_kind::<D2x2>("123.4x"),
    Some(ParseErrorKind::InvalidSyntax)
  );
  assert_eq!(
    parse_kind::<D2x2>("123.456."),
    Some(ParseErrorKind::InvalidSyntax)
  );
  // Also after an exponent that would put the value out of range.
  for input in ["1e999_x", "1e-999 ", "-1e99999999999999999999999.", "1e9e9"] {
    assert_eq!(
      parse_kind::<D2x2>(input),
      Some(ParseErrorKind::InvalidSyntax),
      "input {input:?}"
    );
  }
  // An infinity or a NaN is reported only when the whole input is one.
  assert_eq!(parse_kind::<D2x2>("inf"), Some(ParseErrorKind::NotFinite));
  assert_eq!(
    parse_kind::<D2x2>("infx"),
    Some(ParseErrorKind::InvalidSyntax)
  );
  assert_eq!(
    parse_kind::<D2x2>("NaN1_2"),
    Some(ParseErrorKind::InvalidSyntax)
  );
  // An unsigned storage rejects a leading `-` before anything else.
  assert_eq!(
    parse_kind::<D2x2u16>("inf"),
    Some(ParseErrorKind::NotFinite)
  );
  for input in ["-inf", "-NaN", "-1e999", "-1e-999", "-0e5"] {
    assert_eq!(
      parse_kind::<D2x2u16>(input),
      Some(ParseErrorKind::InvalidSyntax),
      "input {input:?}"
    );
  }
  // Out of range and inexact at once reports out of range.
  assert_eq!(
    parse_kind::<D2x2>("123.456"),
    Some(ParseErrorKind::OutOfRange)
  );
  assert_eq!(
    parse_kind::<D0x2>("1.001"),
    Some(ParseErrorKind::OutOfRange)
  );
  assert_eq!(
    parse_kind::<D2x2>("0.123456e3"),
    Some(ParseErrorKind::OutOfRange)
  );
  assert_eq!(
    parse_kind::<D0x2>("1001e-3"),
    Some(ParseErrorKind::OutOfRange)
  );
}

#[test]
fn parse_at_the_i8_limit() {
  assert_eq!(D1x1::parse("9.9"), Ok(D1x1::MAX));
  assert_eq!(D1x1::parse("-9.9"), Ok(D1x1::MIN));
  assert_eq!(D1x1::parse("09.90").map(D1x1::repr), Ok(99));
  assert_eq!(parse_kind::<D1x1>("10"), Some(ParseErrorKind::OutOfRange));
  assert_eq!(parse_kind::<D1x1>("9.99"), Some(ParseErrorKind::Inexact));

  assert_eq!(D0x2::parse("0.99"), Ok(D0x2::MAX));
  assert_eq!(D0x2::parse("-0.5").map(D0x2::repr), Ok(-50));
  assert_eq!(D0x2::parse("00.010").map(D0x2::repr), Ok(1));
  assert_eq!(D0x2::parse("0"), Ok(D0x2::ZERO));
  assert_eq!(parse_kind::<D0x2>("1"), Some(ParseErrorKind::OutOfRange));
  assert_eq!(
    parse_kind::<D0x2>("-1.00"),
    Some(ParseErrorKind::OutOfRange)
  );
  assert_eq!(parse_kind::<D0x2>("0.001"), Some(ParseErrorKind::Inexact));
  assert_eq!(D0x2::parse(".99"), Ok(D0x2::MAX));
  assert_eq!(D0x2::parse("-.99"), Ok(D0x2::MIN));

  assert_eq!(D2x0::parse("99"), Ok(D2x0::MAX));
  assert_eq!(D2x0::parse("-99.000").map(D2x0::repr), Ok(-99));
  assert_eq!(parse_kind::<D2x0>("12.5"), Some(ParseErrorKind::Inexact));
  assert_eq!(parse_kind::<D2x0>("100"), Some(ParseErrorKind::OutOfRange));
}

#[test]
fn parse_at_the_i128_limit() {
  let max = format!("{}.{}", nines(19), nines(19));
  assert_eq!(D19x19::parse(&max), Ok(D19x19::MAX));
  assert_eq!(D19x19::parse(&format!("-{max}")), Ok(D19x19::MIN));
  assert_eq!(
    parse_kind::<D19x19>(&format!("1{}", zeros(19))),
    Some(ParseErrorKind::OutOfRange)
  );
  assert_eq!(
    parse_kind::<D19x19>(&format!("{max}9")),
    Some(ParseErrorKind::Inexact)
  );

  assert_eq!(D38x0::parse(&nines(38)), Ok(D38x0::MAX));
  assert_eq!(D38x0::parse(&format!("-{}", nines(38))), Ok(D38x0::MIN));
  // 39 and more digits exceed `i128` as well; parsing must not overflow.
  assert_eq!(
    parse_kind::<D38x0>(&nines(39)),
    Some(ParseErrorKind::OutOfRange)
  );
  assert_eq!(
    parse_kind::<D38x0>(&format!("-{}", nines(60))),
    Some(ParseErrorKind::OutOfRange)
  );

  assert_eq!(D0x38::parse(&format!("0.{}", nines(38))), Ok(D0x38::MAX));
  assert_eq!(
    D0x38::parse(&format!("0.{}1", zeros(37))).map(D0x38::repr),
    Ok(1)
  );
  assert_eq!(
    parse_kind::<D0x38>(&format!("0.{}1", zeros(38))),
    Some(ParseErrorKind::Inexact)
  );
}

#[test]
fn parse_long_inputs() {
  let long_zeros = zeros(10_000);
  assert_eq!(
    Price::parse(&format!("{long_zeros}1")).map(Price::repr),
    Ok(100_000_000)
  );
  assert_eq!(
    Price::parse(&format!("1.{long_zeros}")).map(Price::repr),
    Ok(100_000_000)
  );
  assert_eq!(
    parse_kind::<Price>(&format!("1{long_zeros}")),
    Some(ParseErrorKind::OutOfRange)
  );
  assert_eq!(
    parse_kind::<Price>(&format!("1.{long_zeros}1")),
    Some(ParseErrorKind::Inexact)
  );
  assert_eq!(
    parse_kind::<Price>(&format!("1{long_zeros}x")),
    Some(ParseErrorKind::InvalidSyntax)
  );
}

#[test]
fn parse_in_const_context() {
  const TICK: Price = match Price::parse("0.01") {
    Ok(tick) => tick,
    Err(_) => panic!("invalid tick"),
  };
  const ERROR: Result<Price, ParseError> = Price::parse("1.000000001");
  const EXPONENT: Price = match Price::parse("+1_000.5E-3") {
    Ok(value) => value,
    Err(_) => panic!("invalid value"),
  };
  const NOT_FINITE: Result<Price, ParseError> = Price::parse("-Infinity");
  const SATURATED: Result<Price, ParseError> = Price::parse("1e-99999999999999999999999");

  assert_eq!(TICK.repr(), 1_000_000);
  assert_eq!(
    ERROR.map_err(|err| err.kind()),
    Err(ParseErrorKind::Inexact)
  );
  assert_eq!(EXPONENT.repr(), 100_050_000);
  assert_eq!(
    NOT_FINITE.map_err(|err| err.kind()),
    Err(ParseErrorKind::NotFinite)
  );
  assert_eq!(
    SATURATED.map_err(|err| err.kind()),
    Err(ParseErrorKind::Inexact)
  );
}

#[test]
fn methods_in_const_context() {
  const TICK: Price = match Price::from_repr(1_000_000) {
    Some(tick) => tick,
    None => panic!("tick out of range"),
  };
  const SUM: Option<Price> = TICK.checked_add(&TICK);
  const DIFFERENCE: Option<Price> = TICK.checked_sub(&Price::ONE);
  const OVERFLOW: Option<Price> = Price::MAX.checked_add(&TICK);
  const NEGATED: Price = TICK.neg();
  const ABSOLUTE: Price = NEGATED.abs();
  const SIGNS: [bool; 3] = [
    NEGATED.is_zero(),
    NEGATED.is_negative(),
    NEGATED.is_positive(),
  ];
  const COEFF: i64 = NEGATED.repr();

  assert_eq!(SUM.map(Price::repr), Some(2_000_000));
  assert_eq!(DIFFERENCE.map(Price::repr), Some(-99_000_000));
  assert_eq!(OVERFLOW, None);
  assert_eq!(ABSOLUTE, TICK);
  assert_eq!(SIGNS, [false, true, false]);
  assert_eq!(COEFF, -1_000_000);
}

#[test]
fn display_shortest_exact_form() {
  for (coeff, text) in [
    (0, "0"),
    (1, "0.01"),
    (10, "0.1"),
    (100, "1"),
    (1_230, "12.3"),
    (1_234, "12.34"),
    (-5, "-0.05"),
    (-100, "-1"),
    (9_999, "99.99"),
    (-9_999, "-99.99"),
  ] {
    let value = D2x2::from_repr(coeff).unwrap();
    assert_eq!(value.to_string(), text);
    assert_eq!(format!("{value:?}"), text);
  }

  assert_eq!(D2x0::from_repr(-7).unwrap().to_string(), "-7");
  assert_eq!(D2x0::MAX.to_string(), "99");
  assert_eq!(D0x2::from_repr(5).unwrap().to_string(), "0.05");
  assert_eq!(D0x2::MIN.to_string(), "-0.99");
  assert_eq!(D1x1::MIN.to_string(), "-9.9");

  let max = format!("{}.{}", nines(19), nines(19));
  assert_eq!(D19x19::MAX.to_string(), max);
  assert_eq!(D19x19::MIN.to_string(), format!("-{max}"));
  assert_eq!(D19x19::ONE.to_string(), "1");
  assert_eq!(D38x0::MIN.to_string(), format!("-{}", nines(38)));
  assert_eq!(
    D0x38::from_repr(1).unwrap().to_string(),
    format!("0.{}1", zeros(37))
  );
  assert_eq!(D0x38::MAX.to_string(), format!("0.{}", nines(38)));

  // The text depends only on the value, not on the storage.
  let narrow: Price = "-1234.5678".parse().unwrap();
  let wide: WidePrice = narrow.widen();
  assert_eq!(narrow.to_string(), wide.to_string());
}

#[test]
fn display_precision_pads_but_never_rounds() {
  let value = D2x2::from_repr(1_230).unwrap();
  assert_eq!(format!("{value:.0}"), "12.3");
  assert_eq!(format!("{value:.1}"), "12.3");
  assert_eq!(format!("{value:.2}"), "12.30");
  assert_eq!(format!("{value:.5}"), "12.30000");

  let digits = D2x2::from_repr(125).unwrap();
  assert_eq!(format!("{digits:.1}"), "1.25");
  assert_eq!(format!("{digits:.0}"), "1.25");

  assert_eq!(format!("{:.2}", D2x2::ZERO), "0.00");
  assert_eq!(format!("{:.0}", D2x2::ZERO), "0");
  assert_eq!(format!("{:.3}", D2x2::ONE), "1.000");
  assert_eq!(format!("{:.0}", D2x2::ONE), "1");
  assert_eq!(format!("{:.3}", D2x2::from_repr(-5).unwrap()), "-0.050");
  assert_eq!(format!("{:.4}", D2x0::MAX), "99.0000");
  assert_eq!(format!("{:.2?}", D2x2::ONE), "1.00");

  let tiny = D2x2::from_repr(1).unwrap();
  assert_eq!(format!("{tiny:.200}"), format!("0.01{}", zeros(198)));
  assert_eq!(
    format!("{:.100}", D19x19::MAX),
    format!("{}.{}{}", nines(19), nines(19), zeros(81))
  );
}

#[test]
fn display_flags_match_integers() {
  for value in [-99i8, -42, -7, 0, 7, 42, 99] {
    let decimal = D2x0::from_repr(value).unwrap();
    macro_rules! assert_same {
      ($($spec:tt),+ $(,)?) => {$(
        assert_eq!(
          format!($spec, decimal),
          format!($spec, value),
          "format {} of {}",
          $spec,
          value
        );
      )+};
    }
    assert_same!(
      "{}",
      "{:1}",
      "{:5}",
      "{:<5}",
      "{:^5}",
      "{:>5}",
      "{:^6}",
      "{:*<6}",
      "{:*^6}",
      "{:*>6}",
      "{:\u{e9}^7}",
      "{:+}",
      "{:+6}",
      "{:<+6}",
      "{:05}",
      "{:+05}",
      "{:<05}",
      "{:^05}",
      "{:x>+05}",
      "{:02}",
      "{:?}",
      "{:5?}",
      "{:<5?}",
      "{:^+7?}",
      "{:05?}",
    );
  }
}

#[test]
fn display_flags_with_fraction_digits() {
  let value = D2x2::from_repr(-1_230).unwrap();
  assert_eq!(format!("{value:8}"), "   -12.3");
  assert_eq!(format!("{value:>8}"), "   -12.3");
  assert_eq!(format!("{value:<8}"), "-12.3   ");
  assert_eq!(format!("{value:^8}"), " -12.3  ");
  assert_eq!(format!("{value:*^9}"), "**-12.3**");
  assert_eq!(format!("{value:2}"), "-12.3");
  assert_eq!(format!("{value:08}"), "-00012.3");
  assert_eq!(format!("{value:<08}"), "-00012.3");
  assert_eq!(format!("{value:08.2}"), "-0012.30");
  assert_eq!(format!("{value:>10.3}"), "   -12.300");
  assert_eq!(format!("{value:^12?}"), "   -12.3    ");

  let positive = value.abs();
  assert_eq!(format!("{positive:+}"), "+12.3");
  assert_eq!(format!("{positive:+08}"), "+00012.3");
  assert_eq!(format!("{positive:08}"), "000012.3");
  assert_eq!(format!("{positive:+.3}"), "+12.300");
  assert_eq!(format!("{:+}", D2x2::ZERO), "+0");
  assert_eq!(format!("{:+.1}", D2x2::ZERO), "+0.0");

  for spec_width in [70, 200] {
    let padded = format!("{positive:0spec_width$}");
    assert_eq!(padded.len(), spec_width);
    assert_eq!(padded, format!("{}12.3", zeros(spec_width - 4)));
    let filled = format!("{positive:_>spec_width$}");
    assert_eq!(filled, format!("{}12.3", "_".repeat(spec_width - 4)));
  }
}

#[test]
fn exp_shortest_form() {
  for (coeff, text) in [
    (0, "0e0"),
    (1, "1e-2"),
    (10, "1e-1"),
    (100, "1e0"),
    (1_230, "1.23e1"),
    (1_234, "1.234e1"),
    (-5, "-5e-2"),
    (-100, "-1e0"),
    (9_999, "9.999e1"),
    (-9_999, "-9.999e1"),
  ] {
    let value = D2x2::from_repr(coeff).unwrap();
    assert_eq!(format!("{value:e}"), text);
    assert_eq!(format!("{value:E}"), text.replace('e', "E"));
  }

  assert_eq!(format!("{:e}", D2x0::from_repr(-7).unwrap()), "-7e0");
  assert_eq!(format!("{:e}", D2x0::MAX), "9.9e1");
  assert_eq!(format!("{:e}", D0x2::from_repr(5).unwrap()), "5e-2");
  assert_eq!(format!("{:e}", D0x2::MIN), "-9.9e-1");
  assert_eq!(format!("{:e}", D1x1::MIN), "-9.9e0");

  let mantissa = format!("9.{}", nines(37));
  assert_eq!(format!("{:e}", D19x19::MAX), format!("{mantissa}e18"));
  assert_eq!(format!("{:e}", D19x19::MIN), format!("-{mantissa}e18"));
  assert_eq!(format!("{:e}", D19x19::ONE), "1e0");
  assert_eq!(format!("{:e}", D38x0::MIN), format!("-{mantissa}e37"));
  assert_eq!(format!("{:e}", D0x38::from_repr(1).unwrap()), "1e-38");
  assert_eq!(format!("{:e}", D0x38::MAX), format!("{mantissa}e-1"));

  assert_eq!(
    format!("{:e}", "64250.5".parse::<Price>().unwrap()),
    "6.42505e4"
  );
  assert_eq!(
    format!("{:e}", "0.00012".parse::<Price>().unwrap()),
    "1.2e-4"
  );
  assert_eq!(format!("{:e}", "-3".parse::<Price>().unwrap()), "-3e0");
  assert_eq!(format!("{:e}", "1200".parse::<Price>().unwrap()), "1.2e3");
  assert_eq!(
    format!("{:e}", "0.1".parse::<Decimal<U0, U1>>().unwrap()),
    "1e-1"
  );

  // The text depends only on the value, not on the storage or the digit counts.
  let narrow: Price = "-1234.5678".parse().unwrap();
  let wide: WidePrice = narrow.widen();
  let more_digits: Decimal<U12, U10> = narrow.widen();
  assert_eq!(format!("{narrow:e}"), "-1.2345678e3");
  assert_eq!(format!("{wide:e}"), format!("{narrow:e}"));
  assert_eq!(format!("{more_digits:e}"), format!("{narrow:e}"));
  assert_eq!(format!("{more_digits:E}"), "-1.2345678E3");
}

#[test]
fn exp_precision_pads_but_never_rounds() {
  let value = D2x2::from_repr(1_230).unwrap();
  assert_eq!(format!("{value:.0e}"), "1.23e1");
  assert_eq!(format!("{value:.1e}"), "1.23e1");
  assert_eq!(format!("{value:.2e}"), "1.23e1");
  assert_eq!(format!("{value:.5e}"), "1.23000e1");
  assert_eq!(format!("{value:.5E}"), "1.23000E1");

  // The primitive types round a precision below the digits of the mantissa, to even on a tie;
  // `Decimal` writes every digit.
  let digits = D5x4::from_repr(12_345).unwrap();
  assert_eq!(format!("{digits:.2e}"), "1.2345e0");
  assert_eq!(format!("{digits:.0e}"), "1.2345e0");
  for (coeff, precision, text) in [
    (1_250, 1, "1.25e3"),
    (1_350, 1, "1.35e3"),
    (15, 0, "1.5e1"),
    (25, 0, "2.5e1"),
  ] {
    let value = D38x0::from_repr(coeff).unwrap();
    assert_eq!(format!("{value:.precision$e}"), text);
    assert_eq!(format!("{value:.precision$E}"), text.replace('e', "E"));
  }

  assert_eq!(format!("{:.2e}", D2x2::ZERO), "0.00e0");
  assert_eq!(format!("{:.0e}", D2x2::ZERO), "0e0");
  assert_eq!(format!("{:.3e}", D2x2::ONE), "1.000e0");
  assert_eq!(format!("{:.0e}", D2x2::ONE), "1e0");
  assert_eq!(format!("{:.3e}", D2x2::from_repr(-5).unwrap()), "-5.000e-2");
  assert_eq!(format!("{:.4e}", D2x0::MAX), "9.9000e1");

  let tiny = D2x2::from_repr(1).unwrap();
  assert_eq!(format!("{tiny:.200e}"), format!("1.{}e-2", zeros(200)));
  assert_eq!(
    format!("{:.100e}", D19x19::MAX),
    format!("9.{}{}e18", nines(37), zeros(63))
  );
}

#[test]
fn exp_zero_is_0e0_for_every_layout() {
  // The digit writer gives zero a single `0`, so the exponent is not `-Fraction`.
  assert_eq!(format!("{:e}", D2x2::ZERO), "0e0");
  assert_eq!(format!("{:E}", D2x2::ZERO), "0E0");
  assert_eq!(format!("{:e}", D2x0::ZERO), "0e0");
  assert_eq!(format!("{:e}", D0x38::ZERO), "0e0");
  assert_eq!(format!("{:e}", D38x0::ZERO), "0e0");
  assert_eq!(format!("{:e}", D0x153::ZERO), "0e0");
  assert_eq!(format!("{:e}", D154x0u512::ZERO), "0e0");
  assert_eq!(format!("{:e}", D0x154u512::ZERO), "0e0");
  assert_eq!(format!("{:e}", D2x2u16::ZERO), "0e0");

  assert_eq!(format!("{:.2e}", D0x38::ZERO), "0.00e0");
  assert_eq!(format!("{:.0E}", D0x38::ZERO), "0E0");
  assert_eq!(format!("{:+e}", D0x38::ZERO), "+0e0");
  assert_eq!(format!("{:010e}", D0x38::ZERO), "00000000e0");
  assert_eq!(format!("{:+010.2E}", D0x38::ZERO), "+0000.00E0");
  assert_eq!(format!("{:*^9e}", D0x154u512::ZERO), "***0e0***");
}

#[test]
fn exp_flags_match_integers() {
  // The primitive types round a precision below the fraction digits of their mantissa and
  // `Decimal` never does, so a precision is compared only at and above those digits, which the
  // primitive's own shortest text counts.
  macro_rules! assert_same {
    (@specs $decimal:ident, $value:ident, $($spec:tt),+ $(,)?) => {$(
      assert_eq!(
        format!($spec, $decimal),
        format!($spec, $value),
        "format {} of {}",
        $spec,
        $value
      );
    )+};
    ($decimal:expr, $value:expr) => {{
      let decimal = $decimal;
      let value = $value;
      assert_same!(
        @specs decimal,
        value,
        "{:e}",
        "{:E}",
        "{:1e}",
        "{:8e}",
        "{:<8e}",
        "{:^8E}",
        "{:>8e}",
        "{:^9e}",
        "{:*<9E}",
        "{:*^9e}",
        "{:*>9E}",
        "{:\u{e9}^10e}",
        "{:+e}",
        "{:+9E}",
        "{:<+9e}",
        "{:08e}",
        "{:+08E}",
        "{:<08e}",
        "{:^08e}",
        "{:x>+08E}",
        "{:02e}",
        "{:45e}",
        "{:<45E}",
        "{:^46e}",
        "{:*>45E}",
        "{:+045e}",
        "{:045E}",
        "{:#e}",
        "{:#012E}",
      );

      let shortest = format!("{value:e}");
      let (mantissa, _) = shortest.split_once('e').unwrap();
      let digits = mantissa.split_once('.').map_or(0, |(_, fraction)| fraction.len());
      for precision in [digits, digits + 1, digits + 7] {
        assert_eq!(
          format!("{decimal:.precision$e}"),
          format!("{value:.precision$e}")
        );
        assert_eq!(
          format!("{decimal:.precision$E}"),
          format!("{value:.precision$E}")
        );
        assert_eq!(
          format!("{decimal:+9.precision$e}"),
          format!("{value:+9.precision$e}")
        );
        assert_eq!(
          format!("{decimal:*^45.precision$E}"),
          format!("{value:*^45.precision$E}")
        );
        assert_eq!(
          format!("{decimal:+045.precision$e}"),
          format!("{value:+045.precision$e}")
        );
      }
    }};
  }

  for value in [-99i8, -42, -7, 0, 7, 42, 99] {
    assert_same!(D2x0::from_repr(value).unwrap(), value);
  }
  for value in [
    -MAX_38,
    -1_000_000_000_000_000_000_000,
    -12_345_678_901_234_567_890,
    -1_250,
    -1,
    0,
    1,
    15,
    25,
    100,
    1_250,
    1_350,
    10_000_000_000_000_000_000,
    12_345_678_901_234_567_890_123_456_789_012_345_678,
    MAX_38,
  ] {
    assert_same!(D38x0::from_repr(value).unwrap(), value);
  }
  for value in [
    0u128,
    1,
    9,
    10,
    99,
    100,
    123_456_789,
    100_000_000_000_000_000_000,
    MAX_38_U128,
  ] {
    assert_same!(D38x0u128::from_repr(value).unwrap(), value);
  }
}

#[test]
fn exp_flags_with_fraction_digits() {
  let value = D2x2::from_repr(-1_230).unwrap();
  assert_eq!(format!("{value:10e}"), "   -1.23e1");
  assert_eq!(format!("{value:>10e}"), "   -1.23e1");
  assert_eq!(format!("{value:<10e}"), "-1.23e1   ");
  assert_eq!(format!("{value:^10e}"), " -1.23e1  ");
  assert_eq!(format!("{value:*^11e}"), "**-1.23e1**");
  assert_eq!(format!("{value:^14E}"), "   -1.23E1    ");
  assert_eq!(format!("{value:010e}"), "-0001.23e1");
  assert_eq!(format!("{value:<010e}"), "-0001.23e1");
  assert_eq!(format!("{value:012.4e}"), "-0001.2300e1");
  assert_eq!(format!("{value:>12.3e}"), "    -1.230e1");

  let positive = value.abs();
  assert_eq!(format!("{positive:+e}"), "+1.23e1");
  assert_eq!(format!("{positive:+010e}"), "+0001.23e1");
  assert_eq!(format!("{positive:010e}"), "00001.23e1");
  assert_eq!(format!("{positive:+.3e}"), "+1.230e1");
  assert_eq!(format!("{:+e}", D2x2::ZERO), "+0e0");
  assert_eq!(format!("{:+.1e}", D2x2::ZERO), "+0.0e0");

  // `#` is ignored, as for the primitive types.
  assert_eq!(format!("{positive:#e}"), "1.23e1");
  assert_eq!(format!("{positive:#010E}"), "00001.23E1");

  for spec_width in [70, 200] {
    let padded = format!("{positive:0spec_width$e}");
    assert_eq!(padded.len(), spec_width);
    assert_eq!(padded, format!("{}1.23e1", zeros(spec_width - 6)));
    let filled = format!("{positive:_>spec_width$E}");
    assert_eq!(filled, format!("{}1.23E1", "_".repeat(spec_width - 6)));
  }
}

/// Checks that `{:e}` writes `text` for `value`, and that the width counts that whole text,
/// exponent included, under each alignment and the `0` flag, at widths below, at and above its
/// length.
fn assert_exp_padding<T: LowerExp + UpperExp>(value: T, text: &str) {
  assert_eq!(format!("{value:e}"), text);
  assert_eq!(format!("{value:E}"), text.replace('e', "E"));

  let (sign, magnitude) = match text.strip_prefix('-') {
    Some(magnitude) => ("-", magnitude),
    None => ("", text),
  };
  for width in [
    text.len() - 1,
    text.len(),
    text.len() + 1,
    text.len() + 2,
    text.len() + 9,
  ] {
    let padding = width.saturating_sub(text.len());
    let before = " ".repeat(padding / 2);
    let after = " ".repeat(padding - padding / 2);
    for (padded, expected) in [
      (
        format!("{value:>width$e}"),
        format!("{}{text}", " ".repeat(padding)),
      ),
      (
        format!("{value:<width$e}"),
        format!("{text}{}", " ".repeat(padding)),
      ),
      (
        format!("{value:^width$e}"),
        format!("{before}{text}{after}"),
      ),
      (
        format!("{value:0width$e}"),
        format!("{sign}{}{magnitude}", zeros(padding)),
      ),
    ] {
      assert_eq!(padded.len(), width.max(text.len()), "{expected:?}");
      assert_eq!(padded, expected);
    }
  }
}

#[test]
fn exp_width_counts_the_whole_text() {
  type D39x0 = Decimal<U39, U0>;

  // Width 9 with each exponent text, `e0`, `e-1`, `e38`, `e-38`, `e153` and `e-154`: the right,
  // left and center alignments, and the `0` flag.
  macro_rules! assert_padded {
    ($value:expr, [$right:literal, $left:literal, $center:literal, $zero:literal]) => {{
      let value = $value;
      assert_eq!(format!("{value:>9e}"), $right);
      assert_eq!(format!("{value:<9e}"), $left);
      assert_eq!(format!("{value:^9e}"), $center);
      assert_eq!(format!("{value:09e}"), $zero);
      for padded in [$right, $left, $center, $zero] {
        assert_eq!(padded.len(), 9);
      }
    }};
  }
  assert_padded!(
    D2x2::ONE,
    ["      1e0", "1e0      ", "   1e0   ", "0000001e0"]
  );
  assert_padded!(
    D2x2::from_repr(-10).unwrap(),
    ["    -1e-1", "-1e-1    ", "  -1e-1  ", "-00001e-1"]
  );
  assert_padded!(
    "1e38".parse::<D39x0>().unwrap(),
    ["     1e38", "1e38     ", "  1e38   ", "000001e38"]
  );
  assert_padded!(
    D0x38::from_repr(1).unwrap(),
    ["    1e-38", "1e-38    ", "  1e-38  ", "00001e-38"]
  );
  assert_padded!(
    "1e153".parse::<D154x0u512>().unwrap(),
    ["    1e153", "1e153    ", "  1e153  ", "00001e153"]
  );
  assert_padded!(
    "1e-154".parse::<D0x154u512>().unwrap(),
    ["   1e-154", "1e-154   ", " 1e-154  ", "0001e-154"]
  );

  // Exponents of two to five bytes, in values with and without a sign, at widths below, at and
  // above the length of the text.
  assert_exp_padding(D2x2::ONE, "1e0");
  assert_exp_padding(D2x2::from_repr(-10).unwrap(), "-1e-1");
  assert_exp_padding("1e38".parse::<D39x0>().unwrap(), "1e38");
  assert_exp_padding("-1e38".parse::<D39x0>().unwrap(), "-1e38");
  assert_exp_padding(D0x38::from_repr(1).unwrap(), "1e-38");
  assert_exp_padding(D0x38::from_repr(-1).unwrap(), "-1e-38");
  assert_exp_padding("1e153".parse::<D154x0u512>().unwrap(), "1e153");
  assert_exp_padding(D153x0::MIN, &format!("-9.{}e152", nines(152)));
  assert_exp_padding("1e-154".parse::<D0x154u512>().unwrap(), "1e-154");
  assert_exp_padding("-1e-153".parse::<D0x153>().unwrap(), "-1e-153");

  // A mantissa with a fraction, with a long exponent text.
  assert_exp_padding(D154x0u512::MAX, &format!("9.{}e153", nines(153)));
  assert_exp_padding("1.5e-153".parse::<D0x154u512>().unwrap(), "1.5e-153");
  assert_exp_padding(D2x2::ZERO, "0e0");
}

#[test]
fn exp_needs_the_bounds_of_display() {
  fn layout_traits<I, F, R>()
  where
    R: Storage,
    (I, F): Layout<R>,
  {
    fn assert<T: Display + LowerExp + UpperExp>() {}
    assert::<Decimal<I, F, R>>();
  }

  layout_traits::<U10, U8, i64>();
  layout_traits::<U10, U9, u64>();
  layout_traits::<U60, U18, u320>();
}

#[test]
fn exact_mul() {
  let price: Price = "64250.5".parse().unwrap();
  let quantity: Quantity = "-0.0015".parse().unwrap();
  let notional: Decimal<U22, U16> = price.exact_mul(&quantity);
  assert_eq!(notional.repr(), -963_757_500_000_000_000);
  assert_eq!(notional.to_string(), "-96.37575");

  // `MAX * MAX` at the 38-digit limit.
  let square: D38x0 = D19x0::MAX.exact_mul(&D19x0::MAX);
  assert_eq!(square.repr(), (TEN_POW_19 - 1) * (TEN_POW_19 - 1));
  let negative: D38x0 = D19x0::MIN.exact_mul(&D19x0::MAX);
  assert_eq!(negative, square.neg());
  let price_square: Decimal<U20, U16> = Price::MAX.exact_mul(&Price::MAX);
  assert_eq!(price_square.repr(), 999_999_999_999_999_999i128.pow(2));

  // `i8` operands produce an `i16` result.
  let small: Decimal<U2, U2> = D1x1::MAX.exact_mul(&D1x1::MIN);
  assert_eq!(small.to_string(), "-98.01");
  let fraction_only: Decimal<U0, U4> = D0x2::MAX.exact_mul(&D0x2::MAX);
  assert_eq!(fraction_only.to_string(), "0.9801");
  let integral_only: Decimal<U2, U2> = D0x2::MAX.exact_mul(&D2x0::MAX);
  assert_eq!(integral_only.to_string(), "98.01");

  // Mixed storages.
  let mixed: Decimal<U15, U12> = D5x4::MAX.exact_mul(&WidePrice::MIN);
  assert_eq!(mixed.repr(), -999_999_999i128 * 999_999_999_999_999_999i128);
  assert_eq!(D5x4::ZERO.exact_mul(&WidePrice::MIN).repr(), 0);
}

#[test]
fn widen() {
  let small: D2x2 = "-12.5".parse().unwrap();
  let price: Price = small.widen();
  assert_eq!(price.repr(), -1_250_000_000);
  let wide: WidePrice = small.widen();
  assert_eq!(wide.repr(), -1_250_000_000);

  // Same digits in a smaller storage.
  let stored: Decimal<U2, U2, i64> = small.widen();
  let back: D2x2 = stored.widen();
  assert_eq!(back, small);

  let max: D19x19 = D1x1::MAX.widen();
  assert_eq!(max.repr(), 99 * TEN_POW_19 / 10);
  let min: Decimal<U19, U19> = D1x1::MIN.widen();
  assert_eq!(min, max.neg());
  let integral: D38x0 = D2x0::MIN.widen();
  assert_eq!(integral.repr(), -99);
  let fraction: D0x38 = D0x2::MAX.widen();
  assert_eq!(fraction.to_string(), "0.99");
  let same: D19x19 = D19x19::MIN.widen();
  assert_eq!(same, D19x19::MIN);
}

#[test]
fn try_convert() {
  let value: D2x2 = "-1.20".parse().unwrap();
  assert_eq!(value.try_convert::<U2, U2, i16>(), Ok(value));
  assert_eq!(
    value.try_convert::<U2, U1, i16>().map(|v| v.repr()),
    Ok(-12)
  );
  assert_eq!(
    value.try_convert::<U10, U8, i64>(),
    Ok(value.widen::<U10, U8, i64>())
  );

  // A dropped digit is not zero.
  let inexact: D2x2 = "-1.25".parse().unwrap();
  assert_eq!(
    inexact.try_convert::<U2, U1, i16>(),
    Err(ConvertError::Inexact)
  );
  assert_eq!(
    inexact.try_convert::<U2, U0, i8>(),
    Err(ConvertError::Inexact)
  );

  // Range, at the boundary.
  let below: D2x2 = "9.99".parse().unwrap();
  assert_eq!(
    below.try_convert::<U1, U2, i16>().map(|v| v.repr()),
    Ok(999)
  );
  let limit: D2x2 = "10".parse().unwrap();
  assert_eq!(
    limit.try_convert::<U1, U2, i16>(),
    Err(ConvertError::OutOfRange)
  );
  assert_eq!(
    limit.neg().try_convert::<U1, U2, i16>(),
    Err(ConvertError::OutOfRange)
  );

  // Out of range and inexact at once reports out of range.
  assert_eq!(
    D2x2::MAX.try_convert::<U1, U1, i8>(),
    Err(ConvertError::OutOfRange)
  );
  assert_eq!(
    D2x2::MIN.try_convert::<U1, U1, i8>(),
    Err(ConvertError::OutOfRange)
  );

  // `Integral = 0` targets.
  let half: D2x2 = "-0.5".parse().unwrap();
  assert_eq!(half.try_convert::<U0, U2, i8>().map(|v| v.repr()), Ok(-50));
  assert_eq!(
    D2x2::ONE.try_convert::<U0, U2, i8>(),
    Err(ConvertError::OutOfRange)
  );

  // Scaling up overflows `i128` before the range check.
  assert_eq!(
    D38x0::MAX.try_convert::<U0, U38, i128>(),
    Err(ConvertError::OutOfRange)
  );
  assert_eq!(
    D38x0::ONE.try_convert::<U0, U38, i128>(),
    Err(ConvertError::OutOfRange)
  );
  assert_eq!(D38x0::ZERO.try_convert::<U0, U38, i128>(), Ok(D0x38::ZERO));

  // Scaling down by 38 digits.
  assert_eq!(
    D0x38::MAX.try_convert::<U38, U0, i128>(),
    Err(ConvertError::Inexact)
  );
  let tenth: D0x38 = "-0.1".parse().unwrap();
  assert_eq!(tenth.try_convert::<U1, U1, i8>().map(|v| v.repr()), Ok(-1));

  // Narrower storage and fewer digits, when the value fits.
  let price: Price = "-12.5".parse().unwrap();
  assert_eq!(
    price.try_convert::<U2, U2, i16>().map(|v| v.repr()),
    Ok(-1_250)
  );
  assert_eq!(
    price.try_convert::<U2, U0, i8>(),
    Err(ConvertError::Inexact)
  );
  let whole: Price = "12.00000000".parse().unwrap();
  assert_eq!(whole.try_convert::<U2, U0, i8>().map(|v| v.repr()), Ok(12));
  assert_eq!(
    Quantity::MAX.try_convert::<U10, U8, i64>(),
    Err(ConvertError::OutOfRange)
  );
  let quantity: Quantity = "1234567890.12345678".parse().unwrap();
  assert_eq!(
    quantity.try_convert::<U10, U8, i64>().map(|v| v.repr()),
    Ok(123_456_789_012_345_678)
  );
}

#[test]
fn errors() {
  let empty = D2x2::parse("").unwrap_err();
  assert_eq!(empty.kind(), ParseErrorKind::Empty);
  assert_eq!(empty.to_string(), "cannot parse decimal from empty string");
  assert_eq!(
    D2x2::parse("x").unwrap_err().to_string(),
    "invalid decimal syntax"
  );
  let not_finite = D2x2::parse("NaN").unwrap_err();
  assert_eq!(not_finite.kind(), ParseErrorKind::NotFinite);
  assert_eq!(
    not_finite.to_string(),
    "decimal cannot represent infinity or NaN"
  );
  assert_eq!(
    D2x2::parse("100").unwrap_err().to_string(),
    "decimal out of range of the target type"
  );
  assert_eq!(
    D2x2::parse("0.001").unwrap_err().to_string(),
    "decimal cannot be represented exactly in the target type"
  );
  assert_eq!(
    ConvertError::OutOfRange.to_string(),
    "decimal out of range of the target type"
  );
  assert_eq!(
    ConvertError::Inexact.to_string(),
    "decimal cannot be represented exactly in the target type"
  );
  assert_eq!(
    QuantizeError::InvalidStep.to_string(),
    "step must be positive"
  );
  assert_eq!(
    QuantizeError::OutOfRange.to_string(),
    "quantized decimal out of range of the type"
  );
  assert_eq!(DivideError::DivisionByZero.to_string(), "division by zero");
  assert_eq!(
    DivideError::OutOfRange.to_string(),
    "quotient out of range of the target type"
  );

  let errors: [&dyn core::error::Error; 6] = [
    &empty,
    &ConvertError::Inexact,
    &QuantizeError::InvalidStep,
    &QuantizeError::OutOfRange,
    &DivideError::DivisionByZero,
    &DivideError::OutOfRange,
  ];
  assert!(errors.iter().all(|err| err.source().is_none()));
}

#[test]
fn unsigned_constants() {
  assert_eq!(D1x1u8::ZERO.repr(), 0);
  assert_eq!(D1x1u8::MAX.repr(), 99);
  assert_eq!(D1x1u8::MIN, D1x1u8::ZERO);
  assert_eq!(D1x1u8::ONE.repr(), 10);
  assert_eq!(D0x2u8::MAX.repr(), 99);
  assert_eq!(D2x0u8::ONE.repr(), 1);
  assert_eq!(D2x2u16::MAX.repr(), 9_999);
  assert_eq!(D5x4u32::MAX.repr(), 999_999_999);
  assert_eq!(D10x9u64::MAX.repr(), MAX_19_U64);
  assert_eq!(D10x9u64::ONE.repr(), 1_000_000_000);
  assert_eq!(D19x0u64::MAX.repr(), MAX_19_U64);
  assert_eq!(D19x0u64::MIN, D19x0u64::ZERO);
  assert_eq!(D19x19u128::MAX.repr(), MAX_38_U128);
  assert_eq!(D19x19u128::ONE.repr(), TEN_POW_19 as u128);
  assert_eq!(D38x0u128::MAX.repr(), MAX_38_U128);
  assert_eq!(D38x0u128::ONE.repr(), 1);
  assert_eq!(D10x9u64::default(), D10x9u64::ZERO);
  assert_eq!(D0x2u8::default(), D0x2u8::ZERO);

  assert_eq!(D10x9u64::INTEGRAL, 10);
  assert_eq!(D10x9u64::FRACTION, 9);
  assert_eq!(D10x9u64::DIGITS, 19);
  assert_eq!(D0x2u8::INTEGRAL, 0);
  assert_eq!(
    [D10x9u64::SIGNED, D1x1u8::SIGNED, D19x19u128::SIGNED],
    [false; 3]
  );
}

#[test]
fn unsigned_predicates() {
  for (value, zero, positive) in [
    (D10x9u64::ZERO, true, false),
    (D10x9u64::ONE, false, true),
    (D10x9u64::MAX, false, true),
    (D10x9u64::from_repr(1).unwrap(), false, true),
  ] {
    assert_eq!(value.is_zero(), zero);
    assert_eq!(value.is_positive(), positive);
  }
}

#[test]
fn unsigned_from_repr_checks_the_decimal_range() {
  assert_eq!(D1x1u8::from_repr(99), Some(D1x1u8::MAX));
  assert_eq!(D1x1u8::from_repr(0), Some(D1x1u8::ZERO));
  assert_eq!(D1x1u8::from_repr(100), None);
  assert_eq!(D1x1u8::from_repr(u8::MAX), None);

  assert_eq!(D10x9u64::from_repr(MAX_19_U64), Some(D10x9u64::MAX));
  assert_eq!(D10x9u64::from_repr(MAX_19_U64 + 1), None);
  assert_eq!(D10x9u64::from_repr(u64::MAX), None);

  assert_eq!(D38x0u128::from_repr(MAX_38_U128), Some(D38x0u128::MAX));
  assert_eq!(D38x0u128::from_repr(MAX_38_U128 + 1), None);
  assert_eq!(D38x0u128::from_repr(u128::MAX), None);
}

#[test]
fn unsigned_parse() {
  let cases: &[(&str, u16)] = &[
    ("0", 0),
    ("00", 0),
    ("0.0", 0),
    ("1", 100),
    ("12.34", 1_234),
    ("012.30", 1_230),
    ("0.01", 1),
    ("99.99", 9_999),
    ("1.230000", 123),
    // A `+` sign, and a `-` in the exponent, are accepted.
    ("+1", 100),
    ("+.5", 50),
    ("5.", 500),
    ("1e-2", 1),
    ("1E-02", 1),
    ("0e-5", 0),
    ("+0e-5", 0),
    ("125e-2", 125),
    ("+0.5e+1", 500),
  ];
  for &(input, coeff) in cases {
    assert_eq!(
      D2x2u16::parse(input).map(D2x2u16::repr),
      Ok(coeff),
      "input {input:?}"
    );
    assert_eq!(input.parse::<D2x2u16>().map(D2x2u16::repr), Ok(coeff));
  }
  assert_eq!(D10x9u64::parse("1e-5").map(D10x9u64::repr), Ok(10_000));
  assert_eq!(D10x9u64::parse("1E-05").map(D10x9u64::repr), Ok(10_000));
  assert_eq!(D10x9u64::parse("0e-5"), Ok(D10x9u64::ZERO));
  assert_eq!(
    D38x0u128::parse("1e37").map(D38x0u128::repr),
    Ok(10u128.pow(37))
  );

  // A leading `-` is invalid syntax, even for zero, and syntax comes before range and exactness.
  for input in [
    "-", "-0", "-0.0", "-00", "-1", "-0.01", "-99.99", "-100", "-0.001", "-.5", "--1", "-+1",
    "-0e5", "-0e-5", "-1e-1", "-1e5", "-inf", "-nan", "-sNaN1",
  ] {
    assert_eq!(
      parse_kind::<D2x2u16>(input),
      Some(ParseErrorKind::InvalidSyntax),
      "input {input:?}"
    );
  }
  assert_eq!(parse_kind::<D2x2u16>(""), Some(ParseErrorKind::Empty));
  assert_eq!(
    parse_kind::<D2x2u16>("100"),
    Some(ParseErrorKind::OutOfRange)
  );
  assert_eq!(
    parse_kind::<D2x2u16>("1e2"),
    Some(ParseErrorKind::OutOfRange)
  );
  assert_eq!(
    parse_kind::<D2x2u16>("1.234"),
    Some(ParseErrorKind::Inexact)
  );
  assert_eq!(parse_kind::<D2x2u16>("1e-3"), Some(ParseErrorKind::Inexact));
  assert_eq!(
    parse_kind::<D2x2u16>("+inf"),
    Some(ParseErrorKind::NotFinite)
  );

  // Signed storage still reads `-0` as zero.
  assert_eq!(D2x2::parse("-0"), Ok(D2x2::ZERO));
  assert_eq!(D2x2::parse("-0.00"), Ok(D2x2::ZERO));

  assert_eq!(D1x1u8::parse("9.9"), Ok(D1x1u8::MAX));
  assert_eq!(parse_kind::<D1x1u8>("10"), Some(ParseErrorKind::OutOfRange));
  let max_19 = format!("{}.{}", nines(10), nines(9));
  assert_eq!(D10x9u64::parse(&max_19), Ok(D10x9u64::MAX));
  assert_eq!(
    parse_kind::<D10x9u64>(&format!("1{}", zeros(10))),
    Some(ParseErrorKind::OutOfRange)
  );
  assert_eq!(D19x0u64::parse(&nines(19)), Ok(D19x0u64::MAX));
  assert_eq!(
    parse_kind::<D19x0u64>(&nines(20)),
    Some(ParseErrorKind::OutOfRange)
  );
  assert_eq!(D38x0u128::parse(&nines(38)), Ok(D38x0u128::MAX));
  assert_eq!(
    parse_kind::<D38x0u128>(&nines(39)),
    Some(ParseErrorKind::OutOfRange)
  );
  assert_eq!(
    parse_kind::<D38x0u128>(&format!("-{}", nines(38))),
    Some(ParseErrorKind::InvalidSyntax)
  );
}

#[test]
fn unsigned_checked_add_and_sub() {
  let ulp = D19x0u64::from_repr(1).unwrap();
  // 2 * (10^19 - 1) overflows `u64` itself.
  assert_eq!(D19x0u64::MAX.checked_add(&D19x0u64::MAX), None);
  // 10^19 fits in `u64`, so only the decimal bound rejects it.
  assert_eq!(D19x0u64::MAX.checked_add(&ulp), None);
  assert_eq!(
    D19x0u64::MAX.checked_sub(&ulp),
    D19x0u64::from_repr(MAX_19_U64 - 1)
  );
  assert_eq!(
    D19x0u64::MAX.checked_sub(&D19x0u64::MAX),
    Some(D19x0u64::ZERO)
  );
  // Below zero.
  assert_eq!(D19x0u64::ZERO.checked_sub(&ulp), None);
  assert_eq!(ulp.checked_sub(&D19x0u64::MAX), None);

  // 99 + 99 fits in `u8`, so only the decimal bound rejects it.
  assert_eq!(D1x1u8::MAX.checked_add(&D1x1u8::MAX), None);
  assert_eq!(D1x1u8::ZERO.checked_sub(&D1x1u8::MAX), None);
  assert_eq!(D38x0u128::MAX.checked_add(&D38x0u128::ONE), None);
  assert_eq!(
    D38x0u128::MAX.checked_sub(&D38x0u128::ONE),
    D38x0u128::from_repr(MAX_38_U128 - 1)
  );

  let a: D10x9u64 = "1.5".parse().unwrap();
  let b: D10x9u64 = "2.25".parse().unwrap();
  assert_eq!(a.checked_add(&b), Some("3.75".parse().unwrap()));
  assert_eq!(b.checked_sub(&a), Some("0.75".parse().unwrap()));
  assert_eq!(a.checked_sub(&b), None);
}

#[test]
fn unsigned_ordering() {
  let mut values: Vec<D2x2u16> = ["0.01", "99.99", "0", "1", "10.5"]
    .iter()
    .map(|s| s.parse().unwrap())
    .collect();
  values.sort();
  let sorted: Vec<String> = values.iter().map(ToString::to_string).collect();
  assert_eq!(sorted, ["0", "0.01", "1", "10.5", "99.99"]);
  assert!(D19x19u128::MIN < D19x19u128::MAX);
}

#[test]
fn unsigned_methods_in_const_context() {
  const TICK: D10x9u64 = match D10x9u64::parse("0.000000001") {
    Ok(tick) => tick,
    Err(_) => panic!("invalid tick"),
  };
  const SUM: Option<D10x9u64> = TICK.checked_add(&D10x9u64::ONE);
  const BELOW_ZERO: Option<D10x9u64> = TICK.checked_sub(&D10x9u64::ONE);
  const SIGNS: [bool; 2] = [TICK.is_zero(), TICK.is_positive()];
  const COEFF: u64 = TICK.repr();
  const REJECTED: Result<D10x9u64, ParseError> = D10x9u64::parse("-0");

  assert_eq!(SUM.map(D10x9u64::repr), Some(1_000_000_001));
  assert_eq!(BELOW_ZERO, None);
  assert_eq!(SIGNS, [false, true]);
  assert_eq!(COEFF, 1);
  assert_eq!(
    REJECTED.map_err(|err| err.kind()),
    Err(ParseErrorKind::InvalidSyntax)
  );
}

#[test]
fn unsigned_display() {
  for (coeff, text) in [
    (0, "0"),
    (1, "0.01"),
    (10, "0.1"),
    (100, "1"),
    (1_230, "12.3"),
    (9_999, "99.99"),
  ] {
    let value = D2x2u16::from_repr(coeff).unwrap();
    assert_eq!(value.to_string(), text);
    assert_eq!(format!("{value:?}"), text);
  }

  let value = D2x2u16::from_repr(125).unwrap();
  assert_eq!(format!("{value:+}"), "+1.25");
  assert_eq!(format!("{:+}", D2x2u16::ZERO), "+0");
  assert_eq!(format!("{value:+08.3}"), "+001.250");
  assert_eq!(format!("{value:08}"), "00001.25");
  assert_eq!(format!("{value:>7}"), "   1.25");

  assert_eq!(
    D10x9u64::MAX.to_string(),
    format!("{}.{}", nines(10), nines(9))
  );
  assert_eq!(D19x0u64::MAX.to_string(), nines(19));
  assert_eq!(D38x0u128::MAX.to_string(), nines(38));
  assert_eq!(
    D19x19u128::from_repr(1).unwrap().to_string(),
    format!("0.{}1", zeros(18))
  );

  // The flags behave as they do for unsigned integers, including `+`.
  for value in [0u8, 7, 42, 99] {
    let decimal = D2x0u8::from_repr(value).unwrap();
    for (formatted, expected) in [
      (format!("{decimal}"), format!("{value}")),
      (format!("{decimal:+}"), format!("{value:+}")),
      (format!("{decimal:+05}"), format!("{value:+05}")),
      (format!("{decimal:*^6}"), format!("{value:*^6}")),
      (format!("{decimal:<4?}"), format!("{value:<4?}")),
    ] {
      assert_eq!(formatted, expected);
    }
  }
}

#[test]
fn unsigned_exp() {
  for (coeff, text) in [
    (0, "0e0"),
    (1, "1e-2"),
    (10, "1e-1"),
    (100, "1e0"),
    (1_230, "1.23e1"),
    (9_999, "9.999e1"),
  ] {
    let value = D2x2u16::from_repr(coeff).unwrap();
    assert_eq!(format!("{value:e}"), text);
    assert_eq!(format!("{value:E}"), text.replace('e', "E"));
  }

  let value = D2x2u16::from_repr(125).unwrap();
  assert_eq!(format!("{value:+e}"), "+1.25e0");
  assert_eq!(format!("{:+e}", D2x2u16::ZERO), "+0e0");
  assert_eq!(format!("{value:+010.3e}"), "+001.250e0");
  assert_eq!(format!("{value:010e}"), "00001.25e0");
  assert_eq!(format!("{value:>9e}"), "   1.25e0");

  assert_eq!(format!("{:e}", D10x9u64::MAX), format!("9.{}e9", nines(18)));
  assert_eq!(
    format!("{:e}", D19x0u64::MAX),
    format!("9.{}e18", nines(18))
  );
  assert_eq!(
    format!("{:e}", D38x0u128::MAX),
    format!("9.{}e37", nines(37))
  );
  assert_eq!(format!("{:e}", D19x19u128::from_repr(1).unwrap()), "1e-19");
}

#[test]
fn exact_mul_storage_follows_the_operand_signs() {
  // Two unsigned operands give the smallest unsigned storage.
  let both: Decimal<U2, U2, u16> = D1x1u8::MAX.exact_mul(&D1x1u8::MAX);
  assert_eq!(both.repr(), 9_801);
  assert_eq!(both.to_string(), "98.01");
  let square: Decimal<U38, U0, u128> = D19x0u64::MAX.exact_mul(&D19x0u64::MAX);
  assert_eq!(
    square.repr(),
    u128::from(MAX_19_U64) * u128::from(MAX_19_U64)
  );
  // 19 digits need an `i128` when signed but fit in a `u64`.
  let nineteen: Decimal<U19, U0, u64> =
    Decimal::<U18, U0, u64>::MAX.exact_mul(&Decimal::<U1, U0, u8>::MAX);
  assert_eq!(nineteen.repr(), 999_999_999_999_999_999 * 9);
  let mixed_widths: Decimal<U21, U17, u128> =
    D10x9u64::MAX.exact_mul(&Decimal::<U11, U8, u64>::ONE);
  assert_eq!(mixed_widths.to_string(), D10x9u64::MAX.to_string());

  // A signed operand on either side gives the smallest signed storage.
  let signed_first: Decimal<U2, U2, i16> = D1x1::MIN.exact_mul(&D1x1u8::MAX);
  assert_eq!(signed_first.to_string(), "-98.01");
  let unsigned_first: Decimal<U2, U2, i16> = D1x1u8::MAX.exact_mul(&D1x1::MIN);
  assert_eq!(unsigned_first.to_string(), "-98.01");
  let wide: Decimal<U20, U17, i128> = D10x9u64::MAX.exact_mul(&Price::MIN);
  assert_eq!(
    wide.repr(),
    -(i128::from(MAX_19_U64) * 999_999_999_999_999_999)
  );
  // The product of two signed operands keeps the smallest signed storage.
  let signed: Decimal<U2, U2> = D1x1::MAX.exact_mul(&D1x1::MAX);
  let _: i16 = signed.repr();
}

#[test]
fn widen_from_unsigned() {
  let small: D2x2u16 = "12.5".parse().unwrap();
  let price: Price = small.widen();
  assert_eq!(price.repr(), 1_250_000_000);
  let unsigned: D10x9u64 = small.widen();
  assert_eq!(unsigned.repr(), 12_500_000_000);
  let back: D2x2u16 = Decimal::<U2, U2, u128>::MAX.widen();
  assert_eq!(back, D2x2u16::MAX);

  // A 19-digit `u64` widens to the 19-digit default storage, `i128`.
  let max: Decimal<U19, U0> = D19x0u64::MAX.widen();
  assert_eq!(max.repr(), i128::from(MAX_19_U64));
  let fraction: D19x19 = D10x9u64::MAX.widen();
  assert_eq!(fraction.to_string(), D10x9u64::MAX.to_string());
  let unsigned_limit: D19x19u128 = D1x1u8::MAX.widen();
  assert_eq!(unsigned_limit.to_string(), "9.9");
}

#[test]
fn try_convert_with_unsigned_storage() {
  // A negative value is out of range for an unsigned target, even if it truncates to zero.
  let negative: D2x2 = "-1.20".parse().unwrap();
  assert_eq!(
    negative.try_convert::<U2, U2, u16>(),
    Err(ConvertError::OutOfRange)
  );
  let half: D2x2 = "-0.5".parse().unwrap();
  assert_eq!(
    half.try_convert::<U1, U0, u8>(),
    Err(ConvertError::OutOfRange)
  );
  assert_eq!(half.try_convert::<U1, U0, i8>(), Err(ConvertError::Inexact));
  let tiny: Decimal<U0, U38> = format!("-0.{}1", zeros(37)).parse().unwrap();
  assert_eq!(
    tiny.try_convert::<U38, U0, u128>(),
    Err(ConvertError::OutOfRange)
  );
  assert_eq!(
    D2x2::MIN.try_convert::<U19, U19, u128>(),
    Err(ConvertError::OutOfRange)
  );

  // Zero and positive values convert.
  assert_eq!(
    D2x2::ZERO.try_convert::<U1, U0, u8>(),
    Ok(Decimal::<U1, U0, u8>::ZERO)
  );
  let positive: D2x2 = "1.20".parse().unwrap();
  assert_eq!(
    positive.try_convert::<U1, U1, u8>().map(|v| v.repr()),
    Ok(12)
  );
  assert_eq!(
    positive.try_convert::<U19, U0, u64>(),
    Err(ConvertError::Inexact)
  );

  // From unsigned storage, range and exactness work as for signed storage.
  assert_eq!(
    D19x0u64::MAX.try_convert::<U18, U0, i64>(),
    Err(ConvertError::OutOfRange)
  );
  assert_eq!(
    D19x0u64::MAX
      .try_convert::<U19, U0, i128>()
      .map(|v| v.repr()),
    Ok(i128::from(MAX_19_U64))
  );
  assert_eq!(
    D10x9u64::MAX.try_convert::<U10, U8, i64>(),
    Err(ConvertError::Inexact)
  );
  let value: D10x9u64 = "1234567890.12345678".parse().unwrap();
  assert_eq!(
    value.try_convert::<U10, U8, i64>().map(|v| v.repr()),
    Ok(123_456_789_012_345_678)
  );
  assert_eq!(
    value.try_convert::<U2, U2, u16>(),
    Err(ConvertError::OutOfRange)
  );
  assert_eq!(
    D38x0u128::MAX.try_convert::<U0, U38, u128>(),
    Err(ConvertError::OutOfRange)
  );
}

/// `i192` at its 57-digit limit.
type D39x18 = Decimal<U39, U18>;
/// `u192` at its 57-digit limit.
type D39x18u192 = Decimal<U39, U18, u192>;
/// `i256` at its 76-digit limit.
type D58x18 = Decimal<U58, U18>;
/// `u256` at its 77-digit limit, one digit more than `i256` holds.
type D77x0u256 = Decimal<U77, U0, u256>;
/// `i320` at its 96-digit limit.
type D96x0 = Decimal<U96, U0>;
type D0x96 = Decimal<U0, U96>;
/// `u320` at its 96-digit limit.
type D96x0u320 = Decimal<U96, U0, u320>;
/// Any uint256 amount with 18 fraction digits.
type TokenAmount = Decimal<U60, U18, u320>;
/// `i384` at its 115-digit limit.
type D97x18 = Decimal<U97, U18>;
/// `u384` at its 115-digit limit.
type D97x18u384 = Decimal<U97, U18, u384>;
/// `i448` at its 134-digit limit.
type D116x18 = Decimal<U116, U18>;
/// `i512` at its 153-digit limit.
type D153x0 = Decimal<U153, U0>;
type D0x153 = Decimal<U0, U153>;
type D135x18 = Decimal<U135, U18>;
/// `u512` at its 154-digit limit, one digit more than `i512` holds.
type D154x0u512 = Decimal<U154, U0, u512>;

#[test]
fn wide_storage_layout() {
  let _: i192 = D39x18::ZERO.repr();
  let _: i192 = Decimal::<U38, U1>::ZERO.repr();
  let _: u192 = D39x18u192::ZERO.repr();
  let _: i256 = D58x18::ZERO.repr();
  let _: i256 = Decimal::<U38, U38>::ZERO.repr();
  let _: u256 = D77x0u256::ZERO.repr();
  let _: i320 = Decimal::<U77, U0>::ZERO.repr();
  let _: i320 = D96x0::ZERO.repr();
  let _: u320 = D96x0u320::ZERO.repr();
  let _: u320 = TokenAmount::ZERO.repr();
  let _: i384 = D97x18::ZERO.repr();
  let _: u384 = D97x18u384::ZERO.repr();
  let _: i448 = D116x18::ZERO.repr();
  let _: i512 = D135x18::ZERO.repr();
  let _: i512 = D153x0::ZERO.repr();
  let _: u512 = D154x0u512::ZERO.repr();

  assert_eq!(size_of::<D39x18>(), 24);
  assert_eq!(size_of::<D58x18>(), 32);
  assert_eq!(size_of::<D96x0>(), 40);
  assert_eq!(size_of::<D97x18>(), 48);
  assert_eq!(size_of::<D116x18>(), 56);
  assert_eq!(size_of::<D135x18>(), 64);
  assert_eq!(size_of::<D154x0u512>(), size_of::<u512>());
  assert_eq!(size_of::<TokenAmount>(), size_of::<u320>());
  assert_eq!(align_of::<TokenAmount>(), align_of::<u64>());
  // A few digits in an explicitly chosen wide storage.
  assert_eq!(size_of::<Decimal<U2, U2, i320>>(), 40);
}

#[test]
fn wide_trait_surface() {
  fn assert_traits<
    T: Copy + Eq + Ord + Hash + Default + Debug + Display + FromStr + Send + Sync + Unpin + 'static,
  >() {
  }

  assert_traits::<D39x18>();
  assert_traits::<D39x18u192>();
  assert_traits::<D58x18>();
  assert_traits::<D77x0u256>();
  assert_traits::<D96x0>();
  assert_traits::<D96x0u320>();
  assert_traits::<TokenAmount>();
  assert_traits::<Decimal<U2, U2, i320>>();
  assert_traits::<D97x18>();
  assert_traits::<D97x18u384>();
  assert_traits::<D116x18>();
  assert_traits::<D0x153>();
  assert_traits::<D154x0u512>();
}

#[test]
fn wide_constants() {
  assert_eq!(D96x0::MAX.to_string(), nines(96));
  assert_eq!(D96x0::MIN.to_string(), format!("-{}", nines(96)));
  assert_eq!(D96x0::MIN, -D96x0::MAX);
  assert_eq!(D96x0::ONE.repr().to_string(), "1");
  assert_eq!(D0x96::MAX.to_string(), format!("0.{}", nines(96)));
  assert_eq!(D58x18::ONE.repr().to_string(), format!("1{}", zeros(18)));
  assert_eq!(
    D58x18::MAX.to_string(),
    format!("{}.{}", nines(58), nines(18))
  );
  assert_eq!(D77x0u256::MAX.to_string(), nines(77));
  assert_eq!(D77x0u256::MIN, D77x0u256::ZERO);
  assert_eq!(D39x18u192::MAX.repr().to_string(), nines(57));
  assert_eq!(
    TokenAmount::MAX.to_string(),
    format!("{}.{}", nines(60), nines(18))
  );
  assert_eq!(
    TokenAmount::ONE.repr().to_string(),
    format!("1{}", zeros(18))
  );
  assert_eq!(TokenAmount::default(), TokenAmount::ZERO);
  assert_eq!(D96x0::default(), D96x0::ZERO);
  assert!(D96x0::MIN.is_negative() && D96x0::MAX.is_positive() && D96x0::ZERO.is_zero());
  assert!(!D96x0u320::ZERO.is_positive() && D96x0u320::MAX.is_positive());
  assert_eq!(D96x0::MIN.abs(), D96x0::MAX);

  assert_eq!(D97x18::MIN, -D97x18::MAX);
  assert_eq!(
    D135x18::MAX.to_string(),
    format!("{}.{}", nines(135), nines(18))
  );
  assert_eq!(D153x0::MAX.to_string(), nines(153));
  assert_eq!(D0x153::MAX.to_string(), format!("0.{}", nines(153)));
  assert_eq!(D154x0u512::MAX.to_string(), nines(154));
  assert_eq!(D154x0u512::MIN, D154x0u512::ZERO);

  assert_eq!(TokenAmount::INTEGRAL, 60);
  assert_eq!(TokenAmount::FRACTION, 18);
  assert_eq!(TokenAmount::DIGITS, 78);
  assert_eq!(D0x153::DIGITS, 153);
  assert_eq!(D154x0u512::DIGITS, 154);
  assert_eq!(
    [D58x18::SIGNED, D96x0u320::SIGNED, D154x0u512::SIGNED],
    [true, false, false]
  );
}

#[test]
fn wide_from_repr_checks_the_decimal_range() {
  let i320_of = |value: BigInt| i320::from_limbs(bigint::to_limbs(&value, true).expect("fits"));
  let u256_of = |value: BigInt| u256::from_limbs(bigint::to_limbs(&value, false).expect("fits"));

  let max_96 = bigint::pow10(96) - 1u8;
  assert_eq!(D96x0::from_repr(i320_of(max_96.clone())), Some(D96x0::MAX));
  assert_eq!(D96x0::from_repr(i320_of(-max_96)), Some(D96x0::MIN));
  assert_eq!(D96x0::from_repr(i320_of(bigint::pow10(96))), None);
  assert_eq!(D96x0::from_repr(i320_of(-bigint::pow10(96))), None);
  // `i320::MIN` and `i320::MAX`.
  assert_eq!(
    D96x0::from_repr(i320::from_limbs([0, 0, 0, 0, 1 << 63])),
    None
  );
  assert_eq!(
    D96x0::from_repr(i320::from_limbs([
      u64::MAX,
      u64::MAX,
      u64::MAX,
      u64::MAX,
      (1 << 63) - 1
    ])),
    None
  );
  assert_eq!(
    D0x96::from_repr(i320::from_limbs([1, 0, 0, 0, 0])).map(|value| value.to_string()),
    Some(format!("0.{}1", zeros(95)))
  );

  let max_77 = bigint::pow10(77) - 1u8;
  assert_eq!(D77x0u256::from_repr(u256_of(max_77)), Some(D77x0u256::MAX));
  assert_eq!(D77x0u256::from_repr(u256_of(bigint::pow10(77))), None);
  assert_eq!(D77x0u256::from_repr(u256::from_limbs([u64::MAX; 4])), None);
  assert_eq!(D77x0u256::from_repr(u256::ZERO), Some(D77x0u256::ZERO));

  // `i512` holds 153 digits and `u512` holds 154.
  let i512_of = |value: BigInt| i512::from_limbs(bigint::to_limbs(&value, true).expect("fits"));
  let u512_of = |value: BigInt| u512::from_limbs(bigint::to_limbs(&value, false).expect("fits"));
  let max_153 = bigint::pow10(153) - 1u8;
  assert_eq!(
    D153x0::from_repr(i512_of(max_153.clone())),
    Some(D153x0::MAX)
  );
  assert_eq!(D153x0::from_repr(i512_of(-max_153)), Some(D153x0::MIN));
  assert_eq!(D153x0::from_repr(i512_of(bigint::pow10(153))), None);
  assert_eq!(
    D153x0::from_repr(i512::from_limbs([0, 0, 0, 0, 0, 0, 0, 1 << 63])),
    None
  );
  let max_154 = bigint::pow10(154) - 1u8;
  assert_eq!(
    D154x0u512::from_repr(u512_of(max_154)),
    Some(D154x0u512::MAX)
  );
  assert_eq!(D154x0u512::from_repr(u512_of(bigint::pow10(154))), None);
  assert_eq!(D154x0u512::from_repr(u512::from_limbs([u64::MAX; 8])), None);
  assert_eq!(D154x0u512::from_repr(u512::ZERO), Some(D154x0u512::ZERO));
}

#[test]
fn wide_checked_add_and_sub_at_the_storage_limit() {
  // 2 * (10^77 - 1) overflows `u256` itself.
  assert_eq!(D77x0u256::MAX.checked_add(&D77x0u256::MAX), None);
  // 10^77 fits in `u256`, so only the decimal bound rejects it.
  assert_eq!(D77x0u256::MAX.checked_add(&D77x0u256::ONE), None);
  let below = D77x0u256::MAX.checked_sub(&D77x0u256::ONE).unwrap();
  assert_eq!(below.checked_add(&D77x0u256::ONE), Some(D77x0u256::MAX));
  let lower: D77x0u256 = format!("4{}", nines(76)).parse().unwrap();
  let upper: D77x0u256 = format!("5{}", zeros(76)).parse().unwrap();
  assert_eq!(lower.checked_add(&upper), Some(D77x0u256::MAX));
  assert_eq!(
    D77x0u256::MAX.checked_sub(&D77x0u256::MAX),
    Some(D77x0u256::ZERO)
  );
  assert_eq!(D77x0u256::ZERO.checked_sub(&D77x0u256::ONE), None);

  // 2 * (10^96 - 1) overflows `i320` itself, on either side.
  assert_eq!(D96x0::MAX.checked_add(&D96x0::MAX), None);
  assert_eq!(D96x0::MIN.checked_add(&D96x0::MIN), None);
  assert_eq!(D96x0::MAX.checked_sub(&D96x0::MIN), None);
  assert_eq!(D96x0::MIN.checked_sub(&D96x0::MAX), None);
  // 10^96 fits in `i320`, so only the decimal bound rejects it.
  assert_eq!(D96x0::MAX.checked_add(&D96x0::ONE), None);
  assert_eq!(D96x0::MIN.checked_sub(&D96x0::ONE), None);
  assert_eq!(D96x0::MAX.checked_add(&D96x0::MIN), Some(D96x0::ZERO));
  let below = D96x0::MAX.checked_sub(&D96x0::ONE).unwrap();
  assert_eq!(below.checked_add(&D96x0::ONE), Some(D96x0::MAX));
  assert_eq!(below.neg().checked_sub(&D96x0::ONE), Some(D96x0::MIN));

  // 2 * (10^115 - 1) overflows `i384` itself, on either side.
  assert_eq!(D97x18::MAX.checked_add(&D97x18::MAX), None);
  assert_eq!(D97x18::MIN.checked_add(&D97x18::MIN), None);
  assert_eq!(D97x18::MAX.checked_sub(&D97x18::MIN), None);
  assert_eq!(D97x18::MAX.checked_add(&D97x18::MIN), Some(D97x18::ZERO));
  assert_eq!(D97x18::MAX.checked_add(&D97x18::ONE), None);

  // 2 * (10^154 - 1) overflows `u512` itself, and 10^154 fits.
  assert_eq!(D154x0u512::MAX.checked_add(&D154x0u512::MAX), None);
  assert_eq!(D154x0u512::MAX.checked_add(&D154x0u512::ONE), None);
  let below = D154x0u512::MAX.checked_sub(&D154x0u512::ONE).unwrap();
  assert_eq!(below.checked_add(&D154x0u512::ONE), Some(D154x0u512::MAX));
  assert_eq!(D154x0u512::ZERO.checked_sub(&D154x0u512::ONE), None);

  // The other storages hold twice their largest coefficient; the decimal bound rejects the sum.
  assert_eq!(D96x0u320::MAX.checked_add(&D96x0u320::MAX), None);
  assert_eq!(D39x18::MAX.checked_add(&D39x18::MAX), None);
  assert_eq!(D39x18u192::MAX.checked_add(&D39x18u192::MAX), None);
  assert_eq!(D58x18::MIN.checked_add(&D58x18::MIN), None);
  assert_eq!(D97x18u384::MAX.checked_add(&D97x18u384::MAX), None);
  assert_eq!(D116x18::MAX.checked_add(&D116x18::MAX), None);
  assert_eq!(D153x0::MIN.checked_add(&D153x0::MIN), None);
}

#[test]
fn wide_parse_and_display_at_the_limits() {
  assert_eq!(D96x0::parse(&nines(96)), Ok(D96x0::MAX));
  assert_eq!(D96x0::parse(&format!("-{}", nines(96))), Ok(D96x0::MIN));
  assert_eq!(
    parse_kind::<D96x0>(&nines(97)),
    Some(ParseErrorKind::OutOfRange)
  );
  assert_eq!(
    parse_kind::<D96x0>("1e96"),
    Some(ParseErrorKind::OutOfRange)
  );
  assert_eq!(
    D96x0::parse("1e95").map(|value| value.repr().to_string()),
    Ok(format!("1{}", zeros(95)))
  );
  assert_eq!(
    parse_kind::<D96x0>(&format!("{}e-1", nines(97))),
    Some(ParseErrorKind::Inexact)
  );
  assert_eq!(D0x96::parse(&format!("0.{}", nines(96))), Ok(D0x96::MAX));
  assert_eq!(
    D0x96::parse("1e-96").map(|value| value.repr().to_string()),
    Ok("1".to_string())
  );
  assert_eq!(parse_kind::<D0x96>("1e-97"), Some(ParseErrorKind::Inexact));
  assert_eq!(parse_kind::<D0x96>("1"), Some(ParseErrorKind::OutOfRange));
  assert_eq!(D96x0u320::parse(&nines(96)), Ok(D96x0u320::MAX));
  assert_eq!(
    parse_kind::<D96x0u320>(&format!("-{}", nines(96))),
    Some(ParseErrorKind::InvalidSyntax)
  );
  assert_eq!(
    parse_kind::<D96x0u320>("-0"),
    Some(ParseErrorKind::InvalidSyntax)
  );
  assert_eq!(D77x0u256::parse(&nines(77)), Ok(D77x0u256::MAX));
  assert_eq!(
    parse_kind::<D77x0u256>(&nines(78)),
    Some(ParseErrorKind::OutOfRange)
  );
  assert_eq!(parse_kind::<D58x18>("NaN"), Some(ParseErrorKind::NotFinite));

  // Separators, signs, exponents and formatting flags work as for native storages.
  let value = D58x18::parse("-1_234.567_8e+2").unwrap();
  assert_eq!(value.to_string(), "-123456.78");
  assert_eq!(D58x18::parse("-123456.78"), Ok(value));
  assert_eq!(format!("{value:.3}"), "-123456.780");
  assert_eq!(format!("{value:>12}"), "  -123456.78");
  assert_eq!(format!("{:012}", value.abs()), "000123456.78");
  assert_eq!(format!("{:+}", value.abs()), "+123456.78");
  assert_eq!(format!("{:>100}", D96x0::MIN), format!("   -{}", nines(96)));

  assert_eq!(D153x0::parse(&nines(153)), Ok(D153x0::MAX));
  assert_eq!(D153x0::parse(&format!("-{}", nines(153))), Ok(D153x0::MIN));
  assert_eq!(
    parse_kind::<D153x0>(&nines(154)),
    Some(ParseErrorKind::OutOfRange)
  );
  assert_eq!(D0x153::parse(&format!("0.{}", nines(153))), Ok(D0x153::MAX));
  assert_eq!(
    parse_kind::<D0x153>("1e-154"),
    Some(ParseErrorKind::Inexact)
  );
  assert_eq!(D154x0u512::parse(&nines(154)), Ok(D154x0u512::MAX));
  assert_eq!(
    parse_kind::<D154x0u512>(&nines(155)),
    Some(ParseErrorKind::OutOfRange)
  );
  assert_eq!(
    parse_kind::<D154x0u512>("-0"),
    Some(ParseErrorKind::InvalidSyntax)
  );
  assert_eq!(
    format!("{:>160}", D153x0::MIN),
    format!("{:>160}", format!("-{}", nines(153)))
  );
}

#[test]
fn wide_exp_at_the_limits() {
  // The exponent runs from -154, the last place of 154 fraction digits, to 153, the first of 154
  // integral digits.
  let max = format!("9.{}", nines(153));
  assert_eq!(format!("{:e}", D154x0u512::MAX), format!("{max}e153"));
  assert_eq!(format!("{:E}", D154x0u512::MAX), format!("{max}E153"));
  assert_eq!(format!("{:e}", D0x154u512::MAX), format!("{max}e-1"));
  let last_place: D0x154u512 = "1e-154".parse().unwrap();
  assert_eq!(last_place.to_string(), format!("0.{}1", zeros(153)));
  assert_eq!(format!("{last_place:e}"), "1e-154");
  assert_eq!(format!("{last_place:E}"), "1E-154");
  assert_eq!(format!("{:e}", D154x0u512::ONE), "1e0");

  let max = format!("9.{}", nines(152));
  assert_eq!(format!("{:e}", D153x0::MIN), format!("-{max}e152"));
  assert_eq!(format!("{:e}", D153x0::MAX), format!("{max}e152"));
  assert_eq!(format!("{:e}", D0x153::MIN), format!("-{max}e-1"));
  let unit: D0x153 = "1e-153".parse().unwrap();
  assert_eq!(unit.to_string(), format!("0.{}1", zeros(152)));
  assert_eq!(format!("{unit:e}"), "1e-153");
  assert_eq!(format!("{:e}", -unit), "-1e-153");

  // The text reads back at the limits, with a sign, a precision and zero padding.
  for value in [D154x0u512::MAX, D154x0u512::ONE, D154x0u512::ZERO] {
    assert_eq!(format!("{value:e}").parse::<D154x0u512>(), Ok(value));
    assert_eq!(format!("{value:+e}").parse::<D154x0u512>(), Ok(value));
    assert_eq!(format!("{value:0170.10E}").parse::<D154x0u512>(), Ok(value));
  }
  for value in [D0x154u512::MAX, last_place, D0x154u512::ZERO] {
    assert_eq!(format!("{value:e}").parse::<D0x154u512>(), Ok(value));
    assert_eq!(format!("{value:+e}").parse::<D0x154u512>(), Ok(value));
    assert_eq!(format!("{value:0170.10E}").parse::<D0x154u512>(), Ok(value));
  }
  for value in [D153x0::MIN, D153x0::MAX, -D153x0::ONE] {
    assert_eq!(format!("{value:e}").parse::<D153x0>(), Ok(value));
    assert_eq!(format!("{value:0170.10E}").parse::<D153x0>(), Ok(value));
  }
  for value in [D0x153::MIN, D0x153::MAX, unit] {
    assert_eq!(format!("{value:e}").parse::<D0x153>(), Ok(value));
    assert_eq!(format!("{value:0170.10E}").parse::<D0x153>(), Ok(value));
  }

  // Wide storages of other sizes.
  assert_eq!(format!("{:e}", D96x0::MAX), format!("9.{}e95", nines(95)));
  assert_eq!(format!("{:e}", D0x96::MAX), format!("9.{}e-1", nines(95)));
  let amount: TokenAmount = "18446744073709551621e-18".parse().unwrap();
  assert_eq!(format!("{amount:e}"), "1.8446744073709551621e1");
  let largest: TokenAmount =
    "115792089237316195423570985008687907853269984665640564039457584007913129639935e-18"
      .parse()
      .unwrap();
  assert_eq!(
    format!("{largest:e}"),
    "1.15792089237316195423570985008687907853269984665640564039457584007913129639935e59"
  );
}

#[test]
fn wide_exact_mul() {
  // Two 38-digit native coefficients give a 76-digit product in `i256` or `u256`.
  let square: Decimal<U38, U38> = D19x19::MAX.exact_mul(&D19x19::MAX);
  let _: i256 = square.repr();
  let square_text = "9999999999999999999999999999999999999800000000000000000000000000000000000001";
  assert_eq!(square.repr().to_string(), square_text);
  let negative: Decimal<U38, U38> = D19x19::MIN.exact_mul(&D19x19::MAX);
  assert_eq!(negative, square.neg());
  let unsigned: Decimal<U76, U0, u256> = D38x0u128::MAX.exact_mul(&D38x0u128::MAX);
  assert_eq!(unsigned.to_string(), square_text);

  // A native by a wide operand.
  let price: Price = "-64250.5".parse().unwrap();
  let amount: D39x18 = "1.000000000000000001".parse().unwrap();
  let product: Decimal<U49, U26> = price.exact_mul(&amount);
  let _: i256 = product.repr();
  assert_eq!(product.to_string(), "-64250.5000000000000642505");

  // Two wide operands, up to 96 digits.
  let max_57 = bigint::pow10(57) - 1u8;
  let max_39 = bigint::pow10(39) - 1u8;
  let wide: Decimal<U59, U37> = D39x18::MAX.exact_mul(&Decimal::<U20, U19>::MIN);
  let _: i320 = wide.repr();
  assert_eq!(wide.repr().to_string(), (-(&max_57 * &max_39)).to_string());
  let unsigned: Decimal<U59, U37, u320> =
    D39x18u192::MAX.exact_mul(&Decimal::<U20, U19, u192>::MAX);
  assert_eq!(unsigned.repr().to_string(), (&max_57 * &max_39).to_string());

  // An unsigned and a signed operand give a signed product.
  let mixed: Decimal<U78, U1> = D77x0u256::MAX.exact_mul(&D1x1::MIN);
  let _: i320 = mixed.repr();
  assert_eq!(
    mixed.repr().to_string(),
    "-9899999999999999999999999999999999999999999999999999999999999999999999999999901"
  );

  // Operands in wide storages whose product type has a native storage.
  let small: Decimal<U2, U0> =
    Decimal::<U1, U0, i192>::MIN.exact_mul(&Decimal::<U1, U0, u320>::MAX);
  assert_eq!(small.repr(), -81);

  // A uint256 amount by a 20-digit price has 98 digits: `i384` for a signed price, `u384` for an
  // unsigned one.
  let amount: TokenAmount = "1.5".parse().unwrap();
  let price: Decimal<U10, U10> = "-1234.5678".parse().unwrap();
  let notional: Decimal<U70, U28> = amount.exact_mul(&price);
  let _: i384 = notional.repr();
  assert_eq!(notional.to_string(), "-1851.8517");
  let price: Decimal<U10, U10, u128> = "1234.5678".parse().unwrap();
  let notional: Decimal<U70, U28, u384> = amount.exact_mul(&price);
  assert_eq!(notional.to_string(), "1851.8517");
  let largest: Decimal<U70, U28, u384> =
    TokenAmount::MAX.exact_mul(&Decimal::<U10, U10, u128>::MAX);
  let max_78 = bigint::pow10(78) - 1u8;
  let max_20 = bigint::pow10(20) - 1u8;
  assert_eq!(largest.repr().to_string(), (&max_78 * &max_20).to_string());

  // 120 digits in `i448` or `u448`, and 153 digits in `i512` or `u512`.
  let square: Decimal<U120, U0> = Decimal::<U60, U0>::MIN.exact_mul(&Decimal::<U60, U0>::MAX);
  let _: i448 = square.repr();
  let max_60 = bigint::pow10(60) - 1u8;
  assert_eq!(
    square.repr().to_string(),
    (-(&max_60 * &max_60)).to_string()
  );
  let square: Decimal<U120, U0, u448> =
    Decimal::<U60, U0, u256>::MAX.exact_mul(&Decimal::<U60, U0, u256>::MAX);
  assert_eq!(square.repr().to_string(), (&max_60 * &max_60).to_string());
  let max_77 = bigint::pow10(77) - 1u8;
  let max_76 = bigint::pow10(76) - 1u8;
  let widest: Decimal<U153, U0> = Decimal::<U77, U0>::MAX.exact_mul(&Decimal::<U76, U0>::MIN);
  let _: i512 = widest.repr();
  assert_eq!(
    widest.repr().to_string(),
    (-(&max_77 * &max_76)).to_string()
  );
  let widest: Decimal<U153, U0, u512> = D77x0u256::MAX.exact_mul(&Decimal::<U76, U0, u256>::MAX);
  assert_eq!(widest.repr().to_string(), (&max_77 * &max_76).to_string());
}

#[test]
fn wide_widen() {
  let price: Price = "-12.5".parse().unwrap();
  let wide: D39x18 = price.widen();
  assert_eq!(wide.repr().to_string(), "-12500000000000000000");
  let wider: D58x18 = wide.widen();
  assert_eq!(wider.to_string(), "-12.5");
  let widest: Decimal<U78, U18> = wider.widen();
  assert_eq!(widest.to_string(), "-12.5");
  let fraction: D0x96 = D0x38::MIN.widen();
  assert_eq!(fraction.to_string(), format!("-0.{}", nines(38)));
  let integral: D96x0 = D38x0::MAX.widen();
  assert_eq!(integral.to_string(), nines(38));

  // Unsigned storages widen to any storage, wide or native.
  let signed: D96x0 = D77x0u256::MAX.widen();
  assert_eq!(signed.to_string(), nines(77));
  let token: TokenAmount = D38x0u128::MAX.widen();
  assert_eq!(token.to_string(), nines(38));
  let narrow: Decimal<U2, U0, i8> = Decimal::<U1, U0, u192>::MAX.widen();
  assert_eq!(narrow.repr(), 9);

  // Into the widest storages, signed and unsigned.
  let widest: D135x18 = D96x0::MIN.widen();
  assert_eq!(widest.to_string(), format!("-{}", nines(96)));
  let unsigned: D154x0u512 = D77x0u256::MAX.widen();
  assert_eq!(unsigned.to_string(), nines(77));
  let signed: D135x18 = D97x18u384::MAX.widen();
  assert_eq!(signed.to_string(), format!("{}.{}", nines(97), nines(18)));
}

#[test]
fn wide_try_convert() {
  // The range is checked in the target storage, which may hold more digits than the source.
  let value: Decimal<U3, U3> = "999.999".parse().unwrap();
  assert_eq!(
    value.try_convert::<U39, U1, i192>(),
    Err(ConvertError::Inexact)
  );
  let whole: Decimal<U3, U3> = "999.9".parse().unwrap();
  assert_eq!(
    whole
      .try_convert::<U39, U1, i192>()
      .map(|value| value.to_string()),
    Ok("999.9".to_string())
  );
  assert_eq!(
    whole
      .try_convert::<U34, U6, i192>()
      .map(|value| value.repr().to_string()),
    Ok("999900000".to_string())
  );

  // Scaling up by 96 digits.
  assert_eq!(
    D96x0::ONE.try_convert::<U0, U96, i320>(),
    Err(ConvertError::OutOfRange)
  );
  assert_eq!(
    D96x0::MAX.try_convert::<U0, U96, i320>(),
    Err(ConvertError::OutOfRange)
  );
  assert_eq!(D96x0::ZERO.try_convert::<U0, U96, i320>(), Ok(D0x96::ZERO));
  // A native value scaled beyond the native storage.
  let scaled: Decimal<U38, U38, u256> = D38x0u128::MAX.try_convert().unwrap();
  assert_eq!(scaled.to_string(), nines(38));

  // Scaling down by 96 digits.
  assert_eq!(
    D0x96::MAX.try_convert::<U96, U0, i320>(),
    Err(ConvertError::Inexact)
  );
  let half: D0x96 = "-0.5".parse().unwrap();
  assert_eq!(
    half.try_convert::<U1, U1, i8>().map(|value| value.repr()),
    Ok(-5)
  );
  assert_eq!(half.try_convert::<U1, U0, i8>(), Err(ConvertError::Inexact));
  // Out of range comes first, and a negative value never converts to an unsigned storage.
  assert_eq!(
    half.try_convert::<U1, U0, u8>(),
    Err(ConvertError::OutOfRange)
  );
  assert_eq!(
    D96x0::MIN.try_convert::<U96, U0, u320>(),
    Err(ConvertError::OutOfRange)
  );
  assert_eq!(
    D58x18::MAX.try_convert::<U10, U8, i64>(),
    Err(ConvertError::OutOfRange)
  );
  assert_eq!(
    D58x18::MIN.try_convert::<U2, U0, u8>(),
    Err(ConvertError::OutOfRange)
  );
  let precise: D58x18 = "12.000000000000000001".parse().unwrap();
  assert_eq!(
    precise.try_convert::<U2, U0, u8>(),
    Err(ConvertError::Inexact)
  );
  assert_eq!(
    precise.try_convert::<U1, U0, u8>(),
    Err(ConvertError::OutOfRange)
  );
  assert_eq!(
    precise
      .try_convert::<U2, U18, i128>()
      .map(|value| value.to_string()),
    Ok("12.000000000000000001".to_string())
  );

  // Between wide storages of either sign.
  let positive: D96x0 = "12".parse().unwrap();
  assert_eq!(
    positive
      .try_convert::<U77, U0, u256>()
      .map(|value| value.to_string()),
    Ok("12".to_string())
  );
  assert_eq!(
    D77x0u256::MAX.try_convert::<U76, U0, i256>(),
    Err(ConvertError::OutOfRange)
  );
  assert_eq!(
    D77x0u256::MAX.try_convert::<U96, U0, i320>(),
    Ok(D77x0u256::MAX.widen())
  );
  assert_eq!(
    TokenAmount::MAX.try_convert::<U59, U18, u256>(),
    Err(ConvertError::OutOfRange)
  );

  // The widest unsigned storage holds one digit more than the widest signed one.
  assert_eq!(
    D154x0u512::MAX.try_convert::<U153, U0, i512>(),
    Err(ConvertError::OutOfRange)
  );
  assert_eq!(
    D153x0::MAX
      .try_convert::<U154, U0, u512>()
      .map(|value| value.to_string()),
    Ok(nines(153))
  );
  assert_eq!(
    D153x0::MIN.try_convert::<U154, U0, u512>(),
    Err(ConvertError::OutOfRange)
  );
  assert_eq!(
    D153x0::ONE.try_convert::<U0, U153, i512>(),
    Err(ConvertError::OutOfRange)
  );
  assert_eq!(
    D153x0::ZERO.try_convert::<U0, U153, i512>(),
    Ok(D0x153::ZERO)
  );
  assert_eq!(
    D0x153::MAX.try_convert::<U153, U0, i512>(),
    Err(ConvertError::Inexact)
  );
  assert_eq!(
    D135x18::MIN.try_convert::<U97, U18, i384>(),
    Err(ConvertError::OutOfRange)
  );
}

#[test]
fn wide_methods_in_const_context() {
  const RAW: u320 = u320::from_limbs([1_000_000_000_000_000_000, 0, 0, 0, 0]);
  const ONE_TOKEN: TokenAmount = match TokenAmount::from_repr(RAW) {
    Some(amount) => amount,
    None => panic!("amount out of range"),
  };
  const WEI: TokenAmount = match TokenAmount::parse("0.000_000_000_000_000_001") {
    Ok(amount) => amount,
    Err(_) => panic!("invalid amount"),
  };
  const SUM: Option<TokenAmount> = ONE_TOKEN.checked_add(&WEI);
  const OVERFLOW: Option<TokenAmount> = TokenAmount::MAX.checked_add(&WEI);
  const NEGATED: D96x0 = D96x0::MAX.neg();
  const SIGNS: [bool; 3] = [
    NEGATED.is_zero(),
    NEGATED.is_negative(),
    NEGATED.is_positive(),
  ];
  const LIMBS: [u64; 5] = ONE_TOKEN.repr().to_limbs();
  const ERROR: Result<D96x0, ParseError> = D96x0::parse("1e96");
  const WIDEST: Result<D154x0u512, ParseError> = D154x0u512::parse("1e153");
  const WIDEST_OVERFLOW: Option<D154x0u512> = D154x0u512::MAX.checked_add(&D154x0u512::ONE);

  assert_eq!(ONE_TOKEN, TokenAmount::ONE);
  assert_eq!(
    SUM.map(|amount| amount.to_string()),
    Some("1.000000000000000001".to_string())
  );
  assert_eq!(OVERFLOW, None);
  assert_eq!(NEGATED, D96x0::MIN);
  assert_eq!(SIGNS, [false, true, false]);
  assert_eq!(LIMBS, [1_000_000_000_000_000_000, 0, 0, 0, 0]);
  assert_eq!(
    ERROR.map_err(|err| err.kind()),
    Err(ParseErrorKind::OutOfRange)
  );
  assert_eq!(
    WIDEST.map(|value| value.repr().to_string()),
    Ok(format!("1{}", zeros(153)))
  );
  assert_eq!(WIDEST_OVERFLOW, None);
}

#[test]
fn uint256_amounts_reach_the_decimal_exactly() {
  for limbs in [
    [0; 4],
    [1, 0, 0, 0],
    [u64::MAX; 4],
    [0, 0, 0, 1 << 63],
    [12_345, 0, 1, 0],
    [u64::MAX, 0, u64::MAX, 0],
  ] {
    let raw = bigint::from_limbs(&limbs, false);
    let [l0, l1, l2, l3] = limbs;
    let amount = TokenAmount::from_repr(u320::from_limbs([l0, l1, l2, l3, 0]))
      .expect("every uint256 amount fits");
    assert_eq!(amount.repr().to_string(), raw.to_string());
    assert_eq!(TokenAmount::parse(&format!("{raw}e-18")), Ok(amount));
  }
  let max = TokenAmount::from_repr(u320::from_limbs([
    u64::MAX,
    u64::MAX,
    u64::MAX,
    u64::MAX,
    0,
  ]));
  assert_eq!(
    max.map(|amount| amount.to_string()),
    Some(
      "115792089237316195423570985008687907853269984665640564039457.584007913129639935".to_string()
    )
  );
}

#[test]
fn wide_ordering_and_hashing() {
  let state = RandomState::new();
  let mut values: Vec<D58x18> = ["0.000000000000000001", "-1", "0", "1", "-0.5"]
    .iter()
    .map(|s| s.parse().unwrap())
    .collect();
  values.extend([D58x18::MIN, D58x18::MAX]);
  values.sort();
  let sorted: Vec<String> = values.iter().map(ToString::to_string).collect();
  assert_eq!(sorted[0], D58x18::MIN.to_string());
  assert_eq!(
    sorted[1..6],
    ["-1", "-0.5", "0", "0.000000000000000001", "1"]
  );
  assert_eq!(sorted[6], D58x18::MAX.to_string());

  let a: TokenAmount = "1.50".parse().unwrap();
  let b: TokenAmount = "0001.5000e0".parse().unwrap();
  assert_eq!(a, b);
  assert_eq!(state.hash_one(a), state.hash_one(b));
}

/// `i8` without fraction digits.
type D1x0 = Decimal<U1, U0>;
type D1x2 = Decimal<U1, U2>;
type D4x4 = Decimal<U4, U4>;
/// `i320`, with steps and values of several limbs.
type D40x40 = Decimal<U40, U40>;
/// `i192`, with 38 fraction digits.
type D1x38 = Decimal<U1, U38>;
/// `u512`, with only fraction digits.
type D0x154u512 = Decimal<U0, U154, u512>;

/// The table in the documentation of `RoundingMode`: coefficients with one fraction digit, and
/// what they round to with no fraction digits, in the order of `bigint::MODES`.
const MODE_TABLE: [(i8, [i8; 7]); 10] = [
  (55, [6, 5, 6, 5, 6, 5, 6]),
  (25, [3, 2, 3, 2, 3, 2, 2]),
  (16, [2, 1, 2, 1, 2, 2, 2]),
  (11, [2, 1, 2, 1, 1, 1, 1]),
  (10, [1, 1, 1, 1, 1, 1, 1]),
  (-10, [-1, -1, -1, -1, -1, -1, -1]),
  (-11, [-2, -1, -1, -2, -1, -1, -1]),
  (-16, [-2, -1, -1, -2, -2, -2, -2]),
  (-25, [-3, -2, -2, -3, -3, -2, -2]),
  (-55, [-6, -5, -5, -6, -6, -5, -6]),
];

#[test]
fn rounding_mode_table() {
  for (coeff, row) in MODE_TABLE {
    let value = D1x1::from_repr(coeff).unwrap();
    let wide: Decimal<U1, U1, i512> = value.widen();
    for (mode, expected) in bigint::MODES.into_iter().zip(row) {
      assert_eq!(
        value.round_to::<U1, U0, i8>(mode).map(|v| v.repr()),
        Ok(expected),
        "{value} {mode:?}"
      );
      assert_eq!(
        wide.round_to::<U1, U0, i8>(mode).map(|v| v.repr()),
        Ok(expected),
        "{value} {mode:?}"
      );
      assert_eq!(
        value.quantize(&D1x1::ONE, mode).map(|v| v.repr()),
        Ok(expected * 10),
        "{value} {mode:?}"
      );
    }
  }
}

/// The rounded quotients are zero or one, so the sign must come from the value, which a quotient
/// of zero no longer has.
#[test]
fn rounding_takes_the_sign_of_the_value() {
  use RoundingMode::{Ceiling, Floor, HalfDown, HalfEven, HalfUp};

  for (text, mode, expected) in [
    ("-0.6", HalfUp, -1),
    ("-0.4", Floor, -1),
    ("-0.9", Ceiling, 0),
    ("-0.5", HalfEven, 0),
    ("-1.5", HalfEven, -2),
  ] {
    let value: D1x1 = text.parse().unwrap();
    assert_eq!(
      value.round_to::<U1, U0, i8>(mode).map(|v| v.repr()),
      Ok(expected),
      "{text} {mode:?}"
    );
    let wide: D1x38 = value.widen();
    assert_eq!(
      wide.round_to::<U1, U0, i8>(mode).map(|v| v.repr()),
      Ok(expected),
      "{text} {mode:?}"
    );
  }
  // The same shapes with steps that are not powers of ten.
  for (text, step, mode, expected) in [
    ("-0.2", "0.3", HalfUp, "-0.3"),
    ("-0.1", "0.3", Floor, "-0.3"),
    ("-0.2", "0.3", Ceiling, "0"),
    ("-0.2", "0.4", HalfEven, "0"),
    ("-0.6", "0.4", HalfEven, "-0.8"),
    ("-0.2", "0.4", HalfUp, "-0.4"),
    ("-0.2", "0.4", HalfDown, "0"),
    ("0.2", "0.4", HalfEven, "0"),
    ("0.6", "0.4", HalfEven, "0.8"),
  ] {
    let (value, step): (D1x1, D1x1) = (text.parse().unwrap(), step.parse().unwrap());
    assert_eq!(
      value.quantize(&step, mode).map(|v| v.to_string()),
      Ok(expected.to_string()),
      "{text} by {step} {mode:?}"
    );
    // In a wide storage, where the step has several limbs.
    let (value, step): (D1x38, D1x38) = (value.widen(), step.widen());
    assert_eq!(
      value.quantize(&step, mode).map(|v| v.to_string()),
      Ok(expected.to_string()),
      "{text} by {step} {mode:?}"
    );
  }
}

#[test]
fn rounding_to_zero_has_no_sign() {
  let small: D1x1 = "-0.4".parse().unwrap();
  let rounded: D1x0 = small.round_to(RoundingMode::HalfUp).unwrap();
  assert!(rounded.is_zero() && !rounded.is_negative());
  assert_eq!(rounded.to_string(), "0");
  let quantized = small.quantize(&D1x1::ONE, RoundingMode::Down).unwrap();
  assert!(quantized.is_zero() && !quantized.is_negative());
  assert_eq!(quantized.to_string(), "0");
  let wide: D0x153 = format!("-0.4{}", nines(152)).parse().unwrap();
  let rounded: D1x0 = wide.round_to(RoundingMode::Ceiling).unwrap();
  assert_eq!(rounded.to_string(), "0");
  let quantized = wide.quantize(&D0x153::MAX, RoundingMode::Down).unwrap();
  assert!(quantized.is_zero() && !quantized.is_negative());
}

/// Unlike `try_convert`, `round_to` checks the rounded value against an unsigned target.
#[test]
fn round_to_checks_the_rounded_value_for_an_unsigned_target() {
  use RoundingMode::{Ceiling, Floor, HalfUp};

  for (text, mode, expected) in [
    ("-0.4", HalfUp, Ok(0)),
    ("-0.6", HalfUp, Err(ConvertError::OutOfRange)),
    ("-0.9", Ceiling, Ok(0)),
    ("-0.9", Floor, Err(ConvertError::OutOfRange)),
  ] {
    let value: D1x1 = text.parse().unwrap();
    assert_eq!(
      value.round_to::<U1, U0, u8>(mode).map(|v| v.repr()),
      expected,
      "{text} {mode:?}"
    );
    let wide: D1x38 = value.widen();
    assert_eq!(
      wide.round_to::<U1, U0, u8>(mode).map(|v| v.repr()),
      expected,
      "{text} {mode:?}"
    );
    assert_eq!(
      wide.round_to::<U1, U0, u256>(mode).map(|v| v.to_string()),
      expected.map(|coeff| coeff.to_string()),
      "{text} {mode:?}"
    );
  }
  // Without dropped digits, as for `try_convert`, a negative value is out of range and zero
  // converts.
  let tenth: D1x1 = "-0.1".parse().unwrap();
  for mode in bigint::MODES {
    assert_eq!(
      tenth.round_to::<U1, U1, u8>(mode),
      Err(ConvertError::OutOfRange)
    );
    assert_eq!(
      tenth.round_to::<U2, U2, u16>(mode),
      Err(ConvertError::OutOfRange)
    );
    assert_eq!(D1x1::ZERO.round_to::<U1, U1, u8>(mode), Ok(D1x1u8::ZERO));
  }
}

#[test]
fn round_to_checks_the_range_after_rounding() {
  use RoundingMode::{Down, Floor, HalfUp, Up};

  for (text, expected) in [
    ("9.99", Err(ConvertError::OutOfRange)),
    ("-9.96", Err(ConvertError::OutOfRange)),
    ("9.94", Ok(99)),
  ] {
    let value: D1x2 = text.parse().unwrap();
    assert_eq!(
      value.round_to::<U1, U1, i8>(HalfUp).map(|v| v.repr()),
      expected,
      "{text}"
    );
  }
  let value: D2x2 = "99.95".parse().unwrap();
  assert_eq!(
    value.round_to::<U2, U1, i16>(Up),
    Err(ConvertError::OutOfRange)
  );
  assert_eq!(
    value.neg().round_to::<U2, U1, i16>(Floor),
    Err(ConvertError::OutOfRange)
  );
  assert_eq!(
    value.round_to::<U2, U1, i16>(Down).map(|v| v.repr()),
    Ok(999)
  );
  assert_eq!(
    Price::MAX.round_to::<U10, U0, i64>(Up),
    Err(ConvertError::OutOfRange)
  );
  assert_eq!(
    Price::MAX.round_to::<U10, U0, i64>(Down).map(|v| v.repr()),
    Ok(9_999_999_999)
  );
  // A target with one more integral digit holds the carry.
  assert_eq!(
    Price::MAX.round_to::<U11, U0, i64>(Up).map(|v| v.repr()),
    Ok(10_000_000_000)
  );
}

/// Rounding away 20, 38, 153 and 154 fraction digits, which divide by powers of ten of two to
/// eight limbs.
#[test]
fn round_to_across_limb_steps() {
  use RoundingMode::{Ceiling, Down, Floor, HalfDown, HalfEven, HalfUp, Up};

  let half: D0x154u512 = "0.5".parse().unwrap();
  let below: D0x154u512 = format!("0.4{}", nines(153)).parse().unwrap();
  for (mode, at_half, below_half) in [
    (Up, 1, 1),
    (Down, 0, 0),
    (Ceiling, 1, 1),
    (Floor, 0, 0),
    (HalfUp, 1, 0),
    (HalfDown, 0, 0),
    (HalfEven, 0, 0),
  ] {
    assert_eq!(
      half.round_to::<U1, U0, u8>(mode).map(|v| v.repr()),
      Ok(at_half),
      "{mode:?}"
    );
    assert_eq!(
      below.round_to::<U1, U0, u8>(mode).map(|v| v.repr()),
      Ok(below_half),
      "{mode:?}"
    );
  }

  let below: D0x153 = format!("-0.4{}", nines(152)).parse().unwrap();
  for (mode, expected) in [
    (Up, -1),
    (HalfUp, 0),
    (HalfEven, 0),
    (Floor, -1),
    (Ceiling, 0),
  ] {
    assert_eq!(
      below.round_to::<U1, U0, i8>(mode).map(|v| v.repr()),
      Ok(expected),
      "{mode:?}"
    );
  }
  let half: D0x153 = "-0.5".parse().unwrap();
  for (mode, expected) in [(HalfUp, -1), (HalfDown, 0), (HalfEven, 0)] {
    assert_eq!(
      half.round_to::<U1, U0, i8>(mode).map(|v| v.repr()),
      Ok(expected),
      "{mode:?}"
    );
  }

  let tie: Decimal<U1, U20> = format!("1.5{}", zeros(19)).parse().unwrap();
  let tie_38: D1x38 = "-2.5".parse().unwrap();
  let above_38: D1x38 = format!("-2.5{}1", zeros(36)).parse().unwrap();
  for (mode, tie_20, tie_38_expected, above_38_expected) in [
    (Up, 2, -3, -3),
    (Down, 1, -2, -2),
    (Ceiling, 2, -2, -2),
    (Floor, 1, -3, -3),
    (HalfUp, 2, -3, -3),
    (HalfDown, 1, -2, -3),
    (HalfEven, 2, -2, -3),
  ] {
    assert_eq!(
      tie.round_to::<U1, U0, i8>(mode).map(|v| v.repr()),
      Ok(tie_20),
      "{mode:?}"
    );
    assert_eq!(
      tie_38.round_to::<U1, U0, i8>(mode).map(|v| v.repr()),
      Ok(tie_38_expected),
      "{mode:?}"
    );
    assert_eq!(
      above_38.round_to::<U1, U0, i8>(mode).map(|v| v.repr()),
      Ok(above_38_expected),
      "{mode:?}"
    );
  }
}

#[test]
fn round_to_examples() {
  use RoundingMode::{Ceiling, Down, Floor, HalfEven, HalfUp, Up};

  let price: Decimal<U10, U4> = "2.345".parse().unwrap();
  for (value, mode, expected) in [
    (price, HalfEven, "2.34"),
    (price, HalfUp, "2.35"),
    (price, Down, "2.34"),
    (price, Up, "2.35"),
    (price.neg(), HalfEven, "-2.34"),
    (price.neg(), Floor, "-2.35"),
    (price.neg(), Ceiling, "-2.34"),
  ] {
    let rounded: Decimal<U10, U2> = value.round_to(mode).unwrap();
    assert_eq!(rounded.to_string(), expected, "{value} {mode:?}");
  }
  for (text, expected) in [("1.25", "1.2"), ("1.35", "1.4")] {
    let value: D2x2 = text.parse().unwrap();
    let rounded: D1x1 = value.round_to(HalfEven).unwrap();
    assert_eq!(rounded.to_string(), expected);
  }
}

#[test]
fn round_to_without_dropped_digits_is_try_convert() {
  for text in ["0", "1.25", "-1.25", "99.99", "-99.99", "0.01"] {
    let value: D2x2 = text.parse().unwrap();
    for mode in bigint::MODES {
      assert_eq!(value.round_to::<U2, U2, i16>(mode), Ok(value));
      assert_eq!(
        value.round_to::<U10, U8, i64>(mode),
        value.try_convert::<U10, U8, i64>()
      );
      assert_eq!(
        value.round_to::<U1, U3, i16>(mode),
        value.try_convert::<U1, U3, i16>()
      );
      assert_eq!(
        value.round_to::<U2, U2, u16>(mode),
        value.try_convert::<U2, U2, u16>()
      );
      assert_eq!(
        value.round_to::<U39, U18, i192>(mode),
        value.try_convert::<U39, U18, i192>()
      );
    }
  }
}

#[test]
fn quantize_to_steps() {
  use RoundingMode::{Ceiling, Down, Floor, HalfDown, HalfEven, HalfUp, Up};

  for (value, step, mode, expected) in [
    ("12.3456", "1", HalfUp, "12"),
    ("12.3456", "5", HalfUp, "10"),
    ("12.3456", "5", Up, "15"),
    ("12.3456", "0.05", HalfEven, "12.35"),
    ("12.3456", "0.05", Down, "12.3"),
    ("12.3456", "0.05", Up, "12.35"),
    ("12.3456", "0.001", HalfEven, "12.346"),
    ("12.3455", "0.001", HalfEven, "12.346"),
    ("12.3465", "0.001", HalfEven, "12.346"),
    ("12.3456", "0.001", Floor, "12.345"),
    ("-12.3456", "0.001", Floor, "-12.346"),
    ("-12.3456", "0.001", Ceiling, "-12.345"),
    ("-12.3456", "0.05", HalfUp, "-12.35"),
    ("12.3456", "12.3456", Up, "12.3456"),
    ("12.3456", "20", Down, "0"),
    ("12.3456", "20", HalfUp, "20"),
    ("12.3456", "20", Up, "20"),
    ("-12.3456", "20", Up, "-20"),
    ("-12.3456", "20", HalfDown, "-20"),
    ("9999.9999", "5", Down, "9995"),
  ] {
    let (value, step): (D4x4, D4x4) = (value.parse().unwrap(), step.parse().unwrap());
    assert_eq!(
      value.quantize(&step, mode).map(|v| v.to_string()),
      Ok(expected.to_string()),
      "{value} by {step} {mode:?}"
    );
    let (wide, wide_step): (D40x40, D40x40) = (value.widen(), step.widen());
    assert_eq!(
      wide.quantize(&wide_step, mode).map(|v| v.to_string()),
      Ok(expected.to_string()),
      "{value} by {step} {mode:?}"
    );
  }
  // A multiple of the step is returned unchanged in every mode.
  let tick: D4x4 = "0.05".parse().unwrap();
  for text in ["0", "12.35", "-12.35", "9999.95", "-0.05"] {
    let value: D4x4 = text.parse().unwrap();
    for mode in bigint::MODES {
      assert_eq!(value.quantize(&tick, mode), Ok(value), "{text} {mode:?}");
    }
  }
}

/// A zero step has no multiple but zero, so it is rejected, as is a negative step; the result is
/// never zero for a step that cannot be divided by.
#[test]
fn quantize_rejects_steps_that_are_not_positive() {
  let value: D4x4 = "12.3456".parse().unwrap();
  for mode in bigint::MODES {
    for step in [D4x4::ZERO, D4x4::ONE.neg(), D4x4::MIN] {
      assert_eq!(value.quantize(&step, mode), Err(QuantizeError::InvalidStep));
      assert_eq!(
        D4x4::ZERO.quantize(&step, mode),
        Err(QuantizeError::InvalidStep)
      );
    }
    assert_eq!(
      D2x0u8::MAX.quantize(&D2x0u8::ZERO, mode),
      Err(QuantizeError::InvalidStep)
    );
    assert_eq!(
      D153x0::MAX.quantize(&D153x0::ZERO, mode),
      Err(QuantizeError::InvalidStep)
    );
    assert_eq!(
      D153x0::MAX.quantize(&D153x0::MIN, mode),
      Err(QuantizeError::InvalidStep)
    );
    assert_eq!(
      D154x0u512::MAX.quantize(&D154x0u512::ZERO, mode),
      Err(QuantizeError::InvalidStep)
    );
    assert_eq!(
      TokenAmount::ONE.quantize(&TokenAmount::ZERO, mode),
      Err(QuantizeError::InvalidStep)
    );
  }
}

/// Rounding away from zero can overflow the storage or only the decimal range; rounding toward
/// zero never overflows.
#[test]
fn quantize_reports_both_kinds_of_overflow() {
  use RoundingMode::{Down, Floor, HalfDown, HalfEven, HalfUp, Up};

  // 99 + 98 = 196 overflows an `i8`.
  let (value, step) = (D2x0::from_repr(99).unwrap(), D2x0::from_repr(98).unwrap());
  assert_eq!(value.quantize(&step, Up), Err(QuantizeError::OutOfRange));
  assert_eq!(
    value.neg().quantize(&step, Up),
    Err(QuantizeError::OutOfRange)
  );
  assert_eq!(value.quantize(&step, Down).map(|v| v.repr()), Ok(98));
  assert_eq!(value.quantize(&step, HalfUp).map(|v| v.repr()), Ok(98));
  // 90 + 10 = 100 fits an `i8` but not two digits.
  let (value, step) = (D2x0::from_repr(95).unwrap(), D2x0::from_repr(10).unwrap());
  for mode in [Up, HalfUp, HalfEven] {
    assert_eq!(value.quantize(&step, mode), Err(QuantizeError::OutOfRange));
  }
  assert_eq!(
    value.neg().quantize(&step, Floor),
    Err(QuantizeError::OutOfRange)
  );
  assert_eq!(value.quantize(&step, HalfDown).map(|v| v.repr()), Ok(90));
  // 99 + 98 = 196 fits a `u8` but not two digits.
  let (value, step) = (
    D2x0u8::from_repr(99).unwrap(),
    D2x0u8::from_repr(98).unwrap(),
  );
  assert_eq!(value.quantize(&step, Up), Err(QuantizeError::OutOfRange));
  // 2 * (10^19 - 2) overflows a `u64`.
  let step = D19x0u64::from_repr(MAX_19_U64 - 1).unwrap();
  assert_eq!(
    D19x0u64::MAX.quantize(&step, Up),
    Err(QuantizeError::OutOfRange)
  );
  assert_eq!(
    D19x0u64::MAX.quantize(&step, HalfUp).map(|v| v.repr()),
    Ok(MAX_19_U64 - 1)
  );

  // The wide storages whose largest sums overflow: `u256` with 77 digits, `i320` with 96 and
  // `u512` with 154.
  let step = D77x0u256::MAX.checked_sub(&D77x0u256::ONE).unwrap();
  assert_eq!(
    D77x0u256::MAX.quantize(&step, Up),
    Err(QuantizeError::OutOfRange)
  );
  assert_eq!(D77x0u256::MAX.quantize(&step, Down), Ok(step));
  let step = D96x0::MAX.checked_sub(&D96x0::ONE).unwrap();
  assert_eq!(
    D96x0::MAX.quantize(&step, Up),
    Err(QuantizeError::OutOfRange)
  );
  assert_eq!(
    D96x0::MIN.quantize(&step, Up),
    Err(QuantizeError::OutOfRange)
  );
  assert_eq!(D96x0::MIN.quantize(&step, Down), Ok(step.neg()));
  let step = D154x0u512::MAX.checked_sub(&D154x0u512::ONE).unwrap();
  assert_eq!(
    D154x0u512::MAX.quantize(&step, Up),
    Err(QuantizeError::OutOfRange)
  );
  assert_eq!(D154x0u512::MAX.quantize(&step, HalfEven), Ok(step));
}

/// Values and steps of several limbs against rounding by distance.
#[test]
fn quantize_with_multi_limb_steps() {
  fn check(value: D153x0, step: D153x0) {
    let big = |value: D153x0| bigint::from_limbs(&value.repr().to_limbs(), true);
    let (big_value, big_step) = (big(value), big(step));
    for mode in bigint::MODES {
      let multiple = bigint::round(&big_value, &big_step, mode) * &big_step;
      let expected = if multiple.magnitude() < bigint::pow10(153).magnitude() {
        Ok(multiple)
      } else {
        Err(QuantizeError::OutOfRange)
      };
      assert_eq!(
        value.quantize(&step, mode).map(big),
        expected,
        "{value} by {step} {mode:?}"
      );
    }
  }

  let from_big = |value: BigInt| {
    D153x0::from_repr(i512::from_limbs(bigint::to_limbs(&value, true).unwrap())).unwrap()
  };
  let values = [
    D153x0::MAX,
    D153x0::MIN,
    D153x0::ONE,
    from_big(bigint::pow10(152) * 3u8 + 12_345u32),
    from_big(-(bigint::pow10(100) * 7u8) - 1u8),
  ];
  let steps = [
    from_big(bigint::pow10(80) + 1u8),
    from_big(bigint::pow10(40)),
    from_big(bigint::pow10(100) * 25u8),
    from_big(bigint::pow10(60) * 3u8 + 7u8),
    from_big(bigint::pow10(152)),
    D153x0::MAX,
  ];
  for value in values {
    for step in steps {
      check(value, step);
    }
  }
}

#[test]
fn is_multiple_of_steps() {
  let tick: Price = "0.05".parse().unwrap();
  let on: Price = "12.35".parse().unwrap();
  let off: Price = "12.34".parse().unwrap();
  assert!(on.is_multiple_of(&tick) && !off.is_multiple_of(&tick));
  assert!(on.neg().is_multiple_of(&tick) && !off.neg().is_multiple_of(&tick));
  // A negative step counts as its magnitude.
  assert!(on.is_multiple_of(&tick.neg()) && !off.is_multiple_of(&tick.neg()));
  // Only zero is a multiple of zero.
  assert!(Price::ZERO.is_multiple_of(&Price::ZERO));
  assert!(!tick.is_multiple_of(&Price::ZERO) && Price::ZERO.is_multiple_of(&tick));
  // Every value is a multiple of the smallest unit, of either sign.
  let unit = Price::from_repr(1).unwrap();
  assert!(Price::MAX.is_multiple_of(&unit) && Price::MIN.is_multiple_of(&unit.neg()));
  assert!(!unit.is_multiple_of(&Price::MAX) && Price::MAX.is_multiple_of(&Price::MIN));

  let lot: D10x9u64 = "0.001".parse().unwrap();
  assert!("5.002".parse::<D10x9u64>().unwrap().is_multiple_of(&lot));
  assert!(!"5.0025".parse::<D10x9u64>().unwrap().is_multiple_of(&lot));
  assert!(!lot.is_multiple_of(&D10x9u64::ZERO) && D10x9u64::ZERO.is_multiple_of(&D10x9u64::ZERO));

  let gwei: TokenAmount = "0.000000001".parse().unwrap();
  assert!(
    TokenAmount::MAX
      .is_multiple_of(&TokenAmount::from_repr(u320::from_limbs([1, 0, 0, 0, 0])).unwrap())
  );
  assert!(!TokenAmount::MAX.is_multiple_of(&gwei));
  assert!(
    "123.000000004"
      .parse::<TokenAmount>()
      .unwrap()
      .is_multiple_of(&gwei)
  );
  let step = D153x0::MAX.checked_sub(&D153x0::ONE).unwrap();
  assert!(step.neg().is_multiple_of(&step) && !D153x0::MAX.is_multiple_of(&step));
  assert!(D153x0::MIN.is_multiple_of(&D153x0::MAX));
}

#[test]
fn rounding_in_const_context() {
  const TICK: Price = match Price::from_repr(5_000_000) {
    Some(tick) => tick,
    None => panic!("tick out of range"),
  };
  const PRICE: Price = match Price::parse("12.3456") {
    Ok(price) => price,
    Err(_) => panic!("invalid price"),
  };
  const ROUNDED: Result<Price, QuantizeError> = PRICE.quantize(&TICK, RoundingMode::HalfEven);
  const DOWN: Result<Price, QuantizeError> = PRICE.neg().quantize(&TICK, RoundingMode::Down);
  const INVALID: Result<Price, QuantizeError> = PRICE.quantize(&Price::ZERO, RoundingMode::Up);
  const OVERFLOW: Result<Price, QuantizeError> = Price::MAX.quantize(&TICK, RoundingMode::Up);

  const GWEI: TokenAmount = match TokenAmount::parse("0.000000001") {
    Ok(gwei) => gwei,
    Err(_) => panic!("invalid amount"),
  };
  const AMOUNT: TokenAmount = match TokenAmount::parse("1.234567891234567891") {
    Ok(amount) => amount,
    Err(_) => panic!("invalid amount"),
  };
  const WIDE: Result<TokenAmount, QuantizeError> = AMOUNT.quantize(&GWEI, RoundingMode::HalfUp);
  const ON_STEP: [bool; 3] = [
    PRICE.is_multiple_of(&TICK),
    TICK.is_multiple_of(&TICK),
    AMOUNT.is_multiple_of(&GWEI),
  ];

  assert_eq!(ROUNDED.map(|v| v.to_string()), Ok("12.35".to_string()));
  assert_eq!(DOWN.map(|v| v.to_string()), Ok("-12.3".to_string()));
  assert_eq!(INVALID, Err(QuantizeError::InvalidStep));
  assert_eq!(OVERFLOW, Err(QuantizeError::OutOfRange));
  assert_eq!(WIDE.map(|v| v.to_string()), Ok("1.234567891".to_string()));
  assert_eq!(ON_STEP, [false, true, false]);
}

/// `i64`, with two fraction digits.
type Budget = Decimal<U12, U2>;
/// `i16` without fraction digits.
type D3x0 = Decimal<U3, U0>;
/// `i192`, with 38 fraction digits, so that dividing by an integer scales the divisor.
type D2x38 = Decimal<U2, U38>;
/// `i128`, whose quotient by a 38-digit integer needs a 68-digit intermediate.
type D1x30 = Decimal<U1, U30>;
/// `i192`, with 39 digits, the fewest that need it.
type D20x19 = Decimal<U20, U19>;
/// `i512`, the target of a division whose intermediate has 153 digits.
type D143x10 = Decimal<U143, U10>;
/// `u512`, divided with an intermediate of 154 digits.
type D136x10u512 = Decimal<U136, U10, u512>;
type D144x10u512 = Decimal<U144, U10, u512>;

/// The digit count of the intermediate of `div_round`, from the bounds of the method.
type DivDigits<I, F, I2, F2, F3> =
  Maximum<Sum<I, Maximum<Sum<F2, F3>, F>>, Sum<I2, Diff<Maximum<Sum<F2, F3>, F>, F3>>>;

/// The digit count, the type and the size of the intermediate storage that `div_round` picks for a
/// dividend `Decimal<I, F, R>`, a divisor `Decimal<I2, F2, R2>` and `F3` fraction digits.
fn intermediate<I, F, R, I2, F2, R2, F3>() -> (u32, TypeId, usize)
where
  R: Storage,
  R2: Storage,
  F2: Add<F3>,
  Sum<F2, F3>: Max<F>,
  I: Add<Maximum<Sum<F2, F3>, F>>,
  Maximum<Sum<F2, F3>, F>: Sub<F3>,
  I2: Add<Diff<Maximum<Sum<F2, F3>, F>, F3>>,
  Sum<I, Maximum<Sum<F2, F3>, F>>: Max<Sum<I2, Diff<Maximum<Sum<F2, F3>, F>, F3>>>,
  R::IsUnsigned: BitAnd<R2::IsUnsigned>,
  DivDigits<I, F, I2, F2, F3>:
    Unsigned + DivIntermediate<And<R::IsUnsigned, R2::IsUnsigned>, Storage: 'static>,
{
  type Picked<D, B> = <D as DivIntermediate<B>>::Storage;
  (
    <DivDigits<I, F, I2, F2, F3> as Unsigned>::U32,
    TypeId::of::<Picked<DivDigits<I, F, I2, F2, F3>, And<R::IsUnsigned, R2::IsUnsigned>>>(),
    size_of::<Picked<DivDigits<I, F, I2, F2, F3>, And<R::IsUnsigned, R2::IsUnsigned>>>(),
  )
}

/// The intermediates of the documented divisions: native operands with a native or a wide
/// intermediate, an intermediate wider than both operands and the target, the uint256 amount by a
/// price, and the widest, of 153 digits or, for two unsigned storages, 154.
#[test]
fn div_round_intermediates() {
  // A budget by a price.
  assert_eq!(
    intermediate::<U12, U2, i64, U10, U8, i64, U8>(),
    (28, TypeId::of::<i128>(), 16)
  );
  // A change by an integer, as in the documentation.
  assert_eq!(
    intermediate::<U2, U2, i16, U1, U0, i8, U1>(),
    (4, TypeId::of::<i16>(), 2)
  );
  // Two 38-digit native operands need 39 digits.
  assert_eq!(
    intermediate::<U19, U19, i128, U1, U1, i8, U19>(),
    (39, TypeId::of::<i192>(), 24)
  );
  // The divisor scaled by `10^30` needs more digits than either operand or the target.
  assert_eq!(
    intermediate::<U1, U30, i128, U38, U0, i128, U0>(),
    (68, TypeId::of::<i256>(), 32)
  );
  // An unscaled divisor of 39 digits sets the intermediate.
  assert_eq!(
    intermediate::<U1, U1, i8, U20, U19, i192, U0>(),
    (39, TypeId::of::<i192>(), 24)
  );
  // A uint256 amount by a signed or an unsigned 20-digit price.
  assert_eq!(
    intermediate::<U60, U18, u320, U10, U10, i128, U18>(),
    (88, TypeId::of::<i320>(), 40)
  );
  assert_eq!(
    intermediate::<U60, U18, u320, U10, U10, u128, U18>(),
    (88, TypeId::of::<u320>(), 40)
  );
  // The widest: 153 digits, with the dividend or the divisor scaled, and 154 when both storages
  // are unsigned.
  assert_eq!(
    intermediate::<U135, U18, i512, U10, U8, i64, U10>(),
    (153, TypeId::of::<i512>(), 64)
  );
  assert_eq!(
    intermediate::<U0, U153, i512, U1, U0, i8, U1>(),
    (153, TypeId::of::<i512>(), 64)
  );
  assert_eq!(
    intermediate::<U136, U10, u512, U10, U8, u64, U10>(),
    (154, TypeId::of::<u512>(), 64)
  );
  // 19 digits fit a `u64` when both storages are unsigned, and need an `i128` otherwise.
  assert_eq!(
    intermediate::<U10, U9, u64, U1, U0, u8, U9>(),
    (19, TypeId::of::<u64>(), 8)
  );
  assert_eq!(
    intermediate::<U10, U9, u64, U1, U0, i8, U9>(),
    (19, TypeId::of::<i128>(), 16)
  );
  // Wide operand storages with few digits divide in a native storage.
  assert_eq!(
    intermediate::<U2, U2, i512, U1, U1, u320, U2>(),
    (5, TypeId::of::<i32>(), 4)
  );
}

/// Division by zero is an error for every storage, whatever the dividend, the mode and the
/// target, also for `0 / 0`; it is reported before anything else, such as a quotient out of range.
#[test]
fn div_round_by_zero() {
  macro_rules! assert_by_zero {
    ($($repr:ident),+) => {$(
      {
        type D = Decimal<U1, U1, $repr>;
        let name = stringify!($repr);
        for mode in bigint::MODES {
          for value in [D::ZERO, D::ONE, D::MAX, D::MIN] {
            assert_eq!(
              value.div_round::<U1, U1, $repr, _, _, _>(&D::ZERO, mode),
              Err(DivideError::DivisionByZero),
              "{value} in {name}, {mode:?}"
            );
            assert_eq!(
              value.div_round::<U0, U1, i8, _, _, _>(&D::ZERO, mode),
              Err(DivideError::DivisionByZero),
              "{value} in {name}, {mode:?}"
            );
            assert_eq!(
              value.div_round::<U0, U1, u8, _, _, _>(&D::ZERO, mode),
              Err(DivideError::DivisionByZero),
              "{value} in {name}, {mode:?}"
            );
          }
          // A negative dividend into an unsigned target.
          assert_eq!(
            D1x1::MIN.div_round::<U1, U1, u8, _, _, _>(&D::ZERO, mode),
            Err(DivideError::DivisionByZero),
            "{name}, {mode:?}"
          );
        }
      }
    )+};
  }

  assert_by_zero!(
    i8, i16, i32, i64, i128, i192, i256, i320, i384, i448, i512, u8, u16, u32, u64, u128, u192,
    u256, u320, u384, u448, u512
  );
  for mode in bigint::MODES {
    assert_eq!(
      D153x0::MAX.div_round::<U153, U0, i512, _, _, _>(&D153x0::ZERO, mode),
      Err(DivideError::DivisionByZero)
    );
    assert_eq!(
      D154x0u512::MAX.div_round::<U154, U0, u512, _, _, _>(&D154x0u512::ZERO, mode),
      Err(DivideError::DivisionByZero)
    );
  }
}

/// `1 / 4` and `10 / 4` in every mode, with every combination of signs, to fewer fraction digits
/// than the exact quotient has and to as many, in the order of `bigint::MODES`.
#[test]
fn div_round_quarters() {
  let rows: [(i8, i8, [i8; 7]); 8] = [
    (10, 4, [3, 2, 3, 2, 3, 2, 2]),
    (-10, 4, [-3, -2, -2, -3, -3, -2, -2]),
    (10, -4, [-3, -2, -2, -3, -3, -2, -2]),
    (-10, -4, [3, 2, 3, 2, 3, 2, 2]),
    (1, 4, [1, 0, 1, 0, 0, 0, 0]),
    (-1, 4, [-1, 0, 0, -1, 0, 0, 0]),
    (1, -4, [-1, 0, 0, -1, 0, 0, 0]),
    (-1, -4, [1, 0, 1, 0, 0, 0, 0]),
  ];
  for (a, b, row) in rows {
    let (a, b) = (D2x0::from_repr(a).unwrap(), D1x0::from_repr(b).unwrap());
    // With 38 fraction digits, the divisor is scaled instead of the dividend, in an `i192`.
    let wide: D2x38 = a.widen();
    for (mode, expected) in bigint::MODES.into_iter().zip(row) {
      assert_eq!(
        a.div_round::<U2, U0, i8, _, _, _>(&b, mode)
          .map(|q| q.repr()),
        Ok(expected),
        "{a} / {b} {mode:?}"
      );
      assert_eq!(
        wide
          .div_round::<U2, U0, i8, _, _, _>(&b, mode)
          .map(|q| q.repr()),
        Ok(expected),
        "{a} / {b} {mode:?}"
      );
    }
  }
  // To one fraction digit, `0.25` is a tie.
  for (a, b, row) in [
    (1, 4, [3, 2, 3, 2, 3, 2, 2]),
    (-1, 4, [-3, -2, -2, -3, -3, -2, -2]),
    (1, -4, [-3, -2, -2, -3, -3, -2, -2]),
    (-1, -4, [3, 2, 3, 2, 3, 2, 2]),
  ] {
    let (a, b) = (D2x0::from_repr(a).unwrap(), D1x0::from_repr(b).unwrap());
    for (mode, expected) in bigint::MODES.into_iter().zip(row) {
      assert_eq!(
        a.div_round::<U1, U1, i8, _, _, _>(&b, mode)
          .map(|q| q.repr()),
        Ok(expected),
        "{a} / {b} {mode:?}"
      );
    }
  }
  // To two fraction digits, both quotients are exact.
  for (a, b, expected) in [(1, 4, 25), (-1, 4, -25), (10, 4, 250), (10, -4, -250)] {
    let (a, b) = (D2x0::from_repr(a).unwrap(), D1x0::from_repr(b).unwrap());
    for mode in bigint::MODES {
      assert_eq!(
        a.div_round::<U2, U2, i16, _, _, _>(&b, mode)
          .map(|q| q.repr()),
        Ok(expected),
        "{a} / {b} {mode:?}"
      );
    }
  }
}

/// With `HalfEven`, a tie goes to the even quotient: the quotient rounded toward zero stays if it
/// is even and steps away from zero if it is odd.
#[test]
fn div_round_half_even_ties() {
  for (a, b, expected) in [
    (1, 2, 0),
    (3, 2, 2),
    (5, 2, 2),
    (7, 2, 4),
    (-1, 2, 0),
    (-3, 2, -2),
    (-5, 2, -2),
    (-7, 2, -4),
    (5, -2, -2),
    (-7, -2, 4),
    (9, 6, 2),
    (15, 6, 2),
    (-21, 6, -4),
  ] {
    let (a, b) = (D2x0::from_repr(a).unwrap(), D1x0::from_repr(b).unwrap());
    assert_eq!(
      a.div_round::<U2, U0, i8, _, _, _>(&b, RoundingMode::HalfEven)
        .map(|q| q.repr()),
      Ok(expected),
      "{a} / {b}"
    );
    let wide: D2x38 = a.widen();
    assert_eq!(
      wide
        .div_round::<U2, U0, i8, _, _, _>(&b, RoundingMode::HalfEven)
        .map(|q| q.repr()),
      Ok(expected),
      "{a} / {b}"
    );
  }
  // At the last fraction digit of the target, by divisors that are not powers of ten.
  for (a, b, expected) in [
    ("0.125", "1", "0.12"),
    ("0.135", "1", "0.14"),
    ("-0.125", "1", "-0.12"),
    ("0.0025", "0.02", "0.12"),
    ("0.0075", "-0.02", "-0.38"),
  ] {
    let (a, b): (D4x4, D4x4) = (a.parse().unwrap(), b.parse().unwrap());
    assert_eq!(
      a.div_round::<U4, U2, i32, _, _, _>(&b, RoundingMode::HalfEven)
        .map(|q| q.to_string()),
      Ok(expected.to_string()),
      "{a} / {b}"
    );
  }
}

/// A negative quotient converts to an unsigned target only if it rounds to zero, whichever operand
/// is negative; zero divided by a negative divisor is zero, without a sign.
#[test]
fn div_round_negative_quotient_into_an_unsigned_target() {
  use DivideError::OutOfRange;
  use RoundingMode::{Ceiling, Down, Floor, HalfDown, HalfEven, HalfUp, Up};

  for (a, b, mode, expected) in [
    ("-0.1", "4", HalfUp, Ok(0)),
    ("-0.1", "4", HalfEven, Ok(0)),
    ("-0.1", "4", Down, Ok(0)),
    ("-0.1", "4", Ceiling, Ok(0)),
    ("-0.1", "4", Up, Err(OutOfRange)),
    ("-0.1", "4", Floor, Err(OutOfRange)),
    ("0.1", "-4", HalfUp, Ok(0)),
    ("0.1", "-4", Floor, Err(OutOfRange)),
    // `-0.05` is a tie.
    ("-0.2", "4", HalfDown, Ok(0)),
    ("-0.2", "4", HalfEven, Ok(0)),
    ("-0.2", "4", HalfUp, Err(OutOfRange)),
    ("-0.2", "-4", Up, Ok(1)),
    ("-0.4", "4", Ceiling, Err(OutOfRange)),
  ] {
    let (a, b): (D2x2, D1x0) = (a.parse().unwrap(), b.parse().unwrap());
    assert_eq!(
      a.div_round::<U1, U1, u8, _, _, _>(&b, mode)
        .map(|q| q.repr()),
      expected,
      "{a} / {b} {mode:?}"
    );
    assert_eq!(
      a.div_round::<U1, U1, u256, _, _, _>(&b, mode)
        .map(|q| q.repr().to_string()),
      expected.map(|coeff| coeff.to_string()),
      "{a} / {b} {mode:?}"
    );
  }
  // An unsigned dividend by a negative divisor divides in a signed intermediate.
  let tenth: D2x2u16 = "0.1".parse().unwrap();
  let minus_four: D1x0 = "-4".parse().unwrap();
  assert_eq!(
    tenth
      .div_round::<U1, U1, u8, _, _, _>(&minus_four, Ceiling)
      .map(|q| q.repr()),
    Ok(0)
  );
  assert_eq!(
    tenth.div_round::<U1, U1, u8, _, _, _>(&minus_four, Up),
    Err(OutOfRange)
  );
  for mode in bigint::MODES {
    for divisor in [minus_four, D1x0::MIN, D1x0::MAX] {
      let zero = D2x2::ZERO.div_round::<U1, U1, i8, _, _, _>(&divisor, mode);
      assert!(zero.is_ok_and(|q| q.is_zero() && !q.is_negative()));
      assert_eq!(
        D2x2::ZERO.div_round::<U1, U1, u8, _, _, _>(&divisor, mode),
        Ok(D1x1u8::ZERO)
      );
    }
  }
}

/// The range of the target is checked after rounding: a quotient exactly at `MAX` fits, and one
/// that rounds past it does not, even where the storage holds it. A quotient beyond the target's
/// storage is out of range, not truncated.
#[test]
fn div_round_checks_the_range_after_rounding() {
  use DivideError::OutOfRange;
  use RoundingMode::{Ceiling, Down, Floor, HalfDown, HalfEven, HalfUp, Up};

  let two: D1x0 = "2".parse().unwrap();
  let at_max: Decimal<U3, U2> = "199.98".parse().unwrap();
  for mode in bigint::MODES {
    assert_eq!(
      at_max.div_round::<U2, U2, i16, _, _, _>(&two, mode),
      Ok(D2x2::MAX)
    );
    assert_eq!(
      at_max.neg().div_round::<U2, U2, i16, _, _, _>(&two, mode),
      Ok(D2x2::MIN)
    );
  }
  // `99.995` rounds to `99.99` or to `100.00`, which an `i16` holds but two integral digits do
  // not; with `HalfEven`, the quotient toward zero, `9999`, is odd.
  let past: Decimal<U3, U2> = "199.99".parse().unwrap();
  for (mode, positive, negative) in [
    (Up, Err(OutOfRange), Err(OutOfRange)),
    (Down, Ok(D2x2::MAX), Ok(D2x2::MIN)),
    (Ceiling, Err(OutOfRange), Ok(D2x2::MIN)),
    (Floor, Ok(D2x2::MAX), Err(OutOfRange)),
    (HalfUp, Err(OutOfRange), Err(OutOfRange)),
    (HalfDown, Ok(D2x2::MAX), Ok(D2x2::MIN)),
    (HalfEven, Err(OutOfRange), Err(OutOfRange)),
  ] {
    assert_eq!(
      past.div_round::<U2, U2, i16, _, _, _>(&two, mode),
      positive,
      "{mode:?}"
    );
    assert_eq!(
      past.neg().div_round::<U2, U2, i16, _, _, _>(&two, mode),
      negative,
      "{mode:?}"
    );
  }
  // `300` and `-200` truncate to two digits in an `i8` and a `u8`, but are out of range.
  let one: D1x0 = "1".parse().unwrap();
  for mode in bigint::MODES {
    assert_eq!(
      D3x0::from_repr(600)
        .unwrap()
        .div_round::<U2, U0, i8, _, _, _>(&two, mode),
      Err(OutOfRange)
    );
    assert_eq!(
      D3x0::from_repr(-200)
        .unwrap()
        .div_round::<U2, U0, u8, _, _, _>(&one, mode),
      Err(OutOfRange)
    );
    // `2^64 + 5` in an `i192` truncates to `5` in an `i64`.
    let beyond: D39x18 = "18446744073709551621".parse().unwrap();
    assert_eq!(
      beyond.div_round::<U18, U0, i64, _, _, _>(&one, mode),
      Err(OutOfRange)
    );
    assert_eq!(
      beyond
        .div_round::<U20, U0, i128, _, _, _>(&one, mode)
        .map(|q| q.repr()),
      Ok(18_446_744_073_709_551_621)
    );
  }
}

/// `MIN` divided by a coefficient of `-1` and by itself, in its own type: the intermediate holds
/// every coefficient, so neither operand is the intermediate's `MIN`.
#[test]
fn div_round_at_the_limits() {
  for mode in bigint::MODES {
    let minus_one = D2x0::from_repr(-1).unwrap();
    assert_eq!(
      D2x0::MIN.div_round::<U2, U0, i8, _, _, _>(&minus_one, mode),
      Ok(D2x0::MAX)
    );
    assert_eq!(
      D2x0::MIN.div_round::<U2, U0, i8, _, _, _>(&D2x0::MIN, mode),
      Ok(D2x0::ONE)
    );
    assert_eq!(
      D2x0::MAX.div_round::<U2, U0, i8, _, _, _>(&D2x0::MIN, mode),
      Ok(D2x0::ONE.neg())
    );
    let minus_one = D38x0::from_repr(-1).unwrap();
    assert_eq!(
      D38x0::MIN.div_round::<U38, U0, i128, _, _, _>(&minus_one, mode),
      Ok(D38x0::MAX)
    );
    assert_eq!(
      D38x0::MIN.div_round::<U38, U0, i128, _, _, _>(&D38x0::MIN, mode),
      Ok(D38x0::ONE)
    );
    let minus_one = D153x0::ONE.neg();
    assert_eq!(
      D153x0::MIN.div_round::<U153, U0, i512, _, _, _>(&minus_one, mode),
      Ok(D153x0::MAX)
    );
    assert_eq!(
      D153x0::MIN.div_round::<U153, U0, i512, _, _, _>(&D153x0::MIN, mode),
      Ok(D153x0::ONE)
    );
    assert_eq!(
      D154x0u512::MAX.div_round::<U154, U0, u512, _, _, _>(&D154x0u512::MAX, mode),
      Ok(D154x0u512::ONE)
    );
    assert_eq!(
      D154x0u512::MAX.div_round::<U154, U0, u512, _, _, _>(&D154x0u512::ONE, mode),
      Ok(D154x0u512::MAX)
    );
    // `-9.9 / -0.1` into a type with more integral digits.
    let tenth = D1x1::from_repr(-1).unwrap();
    assert_eq!(
      D1x1::MIN
        .div_round::<U2, U0, i8, _, _, _>(&tenth, mode)
        .map(|q| q.repr()),
      Ok(99)
    );
  }
}

/// One case of each family of storages, against quotients computed by hand, in the order of
/// `bigint::MODES` where the mode matters.
#[test]
fn div_round_storage_families() {
  // Native operands with a native intermediate: the quantity that a budget buys at a price.
  let budget: Budget = "1000".parse().unwrap();
  let price: Price = "64250.5".parse().unwrap();
  for (mode, expected) in bigint::MODES.into_iter().zip([
    "0.01556409",
    "0.01556408",
    "0.01556409",
    "0.01556408",
    "0.01556408",
    "0.01556408",
    "0.01556408",
  ]) {
    let quantity: Quantity = budget.div_round(&price, mode).unwrap();
    assert_eq!(quantity.to_string(), expected, "{mode:?}");
  }

  // Native operands with a wide intermediate of 39 digits.
  let divisor: D1x1 = "0.7".parse().unwrap();
  let seventh = "14285714285714285714.2857142857142857";
  for (mode, last) in bigint::MODES
    .into_iter()
    .zip(["142", "141", "142", "141", "141", "141", "141"])
  {
    let quotient: D20x19 = D19x19::MAX.div_round(&divisor, mode).unwrap();
    assert_eq!(quotient.to_string(), format!("{seventh}{last}"), "{mode:?}");
  }

  // A native dividend by a 39-digit divisor, whose digits alone need an `i192`.
  for (mode, (positive, negative)) in
    bigint::MODES
      .into_iter()
      .zip([(1, -1), (0, 0), (1, 0), (0, -1), (0, 0), (0, 0), (0, 0)])
  {
    assert_eq!(
      D1x1::MAX
        .div_round::<U2, U0, i8, _, _, _>(&D20x19::MAX, mode)
        .map(|q| q.repr()),
      Ok(positive),
      "{mode:?}"
    );
    assert_eq!(
      D1x1::MAX
        .div_round::<U2, U0, i8, _, _, _>(&D20x19::MIN, mode)
        .map(|q| q.repr()),
      Ok(negative),
      "{mode:?}"
    );
  }

  // Wide operands, with a 76-digit intermediate.
  let seven: D20x19 = "7".parse().unwrap();
  let digits = "142857142857142857142857142857142857142.857142857142857";
  for (mode, positive, negative) in [
    (RoundingMode::Up, "143", "143"),
    (RoundingMode::Down, "142", "142"),
    (RoundingMode::Ceiling, "143", "142"),
    (RoundingMode::Floor, "142", "143"),
    (RoundingMode::HalfUp, "143", "143"),
    (RoundingMode::HalfDown, "143", "143"),
    (RoundingMode::HalfEven, "143", "143"),
  ] {
    let quotient: D39x18 = D39x18::MAX.div_round(&seven, mode).unwrap();
    assert_eq!(
      quotient.to_string(),
      format!("{digits}{positive}"),
      "{mode:?}"
    );
    let quotient: D39x18 = D39x18::MAX.div_round(&seven.neg(), mode).unwrap();
    assert_eq!(
      quotient.to_string(),
      format!("-{digits}{negative}"),
      "{mode:?}"
    );
  }

  // Two unsigned storages divide in an unsigned intermediate, and mixed signs in a signed one.
  let third: D10x9u64 = "3".parse().unwrap();
  let quotient: D10x9u64 = D10x9u64::MAX.div_round(&third, RoundingMode::Up).unwrap();
  assert_eq!(quotient.to_string(), "3333333333.333333333");
  let one: D10x9u64 = "1".parse().unwrap();
  let minus_three: Price = "-3".parse().unwrap();
  for (mode, expected) in bigint::MODES.into_iter().zip([
    "-0.33333334",
    "-0.33333333",
    "-0.33333333",
    "-0.33333334",
    "-0.33333333",
    "-0.33333333",
    "-0.33333333",
  ]) {
    let quotient: Price = one.div_round(&minus_three, mode).unwrap();
    assert_eq!(quotient.to_string(), expected, "{mode:?}");
  }

  // An intermediate of 68 digits, wider than both operands and the target: the divisor is scaled
  // by `10^30`.
  let five: D1x30 = "5".parse().unwrap();
  let two: D38x0 = "2".parse().unwrap();
  let almost_ten: D1x30 = format!("9.{}", nines(30)).parse().unwrap();
  let large: D38x0 = format!("1{}", zeros(37)).parse().unwrap();
  for (mode, (tie, tiny)) in
    bigint::MODES
      .into_iter()
      .zip([(3, 1), (2, 0), (3, 1), (2, 0), (3, 0), (2, 0), (2, 0)])
  {
    assert_eq!(
      five
        .div_round::<U1, U0, i8, _, _, _>(&two, mode)
        .map(|q| q.repr()),
      Ok(tie),
      "{mode:?}"
    );
    assert_eq!(
      almost_ten
        .div_round::<U1, U0, i8, _, _, _>(&large, mode)
        .map(|q| q.repr()),
      Ok(tiny),
      "{mode:?}"
    );
  }

  // A uint256 amount by a 20-digit price, in an 88-digit intermediate: signed for a signed price,
  // so a negative price gives a negative quotient, which is out of range.
  let amount: TokenAmount = "1000000".parse().unwrap();
  let price: Decimal<U10, U10> = "1234.5678".parse().unwrap();
  let down: TokenAmount = amount.div_round(&price, RoundingMode::Down).unwrap();
  assert_eq!(down.to_string(), "810.00006642000544644");
  let up: TokenAmount = amount.div_round(&price, RoundingMode::Up).unwrap();
  assert_eq!(up.to_string(), "810.000066420005446441");
  assert_eq!(
    amount.div_round::<U60, U18, u320, _, _, _>(&price.neg(), RoundingMode::Down),
    Err(DivideError::OutOfRange)
  );
  let price: Decimal<U10, U10, u128> = "1234.5678".parse().unwrap();
  let down: TokenAmount = amount.div_round(&price, RoundingMode::Down).unwrap();
  assert_eq!(down.to_string(), "810.00006642000544644");
}

/// The widest intermediates: 153 digits with the dividend scaled or with the divisor scaled, and
/// 154 digits for two unsigned storages.
#[test]
fn div_round_with_the_widest_intermediates() {
  let tick: Price = "0.00000001".parse().unwrap();
  let one: Price = "1".parse().unwrap();
  let max_by_tick = format!("{}.{}", nines(143), nines(10));
  for mode in bigint::MODES {
    let quotient: D143x10 = D135x18::MAX.div_round(&tick, mode).unwrap();
    assert_eq!(quotient.to_string(), max_by_tick, "{mode:?}");
  }
  for (mode, expected) in bigint::MODES.into_iter().zip([
    format!("1{}", zeros(135)),
    format!("{}.{}", nines(135), nines(10)),
    format!("1{}", zeros(135)),
    format!("{}.{}", nines(135), nines(10)),
    format!("1{}", zeros(135)),
    format!("1{}", zeros(135)),
    format!("1{}", zeros(135)),
  ]) {
    let quotient: D143x10 = D135x18::MAX.div_round(&one, mode).unwrap();
    assert_eq!(quotient.to_string(), expected, "{mode:?}");
    let quotient: D143x10 = D135x18::MIN.div_round(&one, mirror(mode)).unwrap();
    assert_eq!(quotient.to_string(), format!("-{expected}"), "{mode:?}");
  }

  // The divisor scaled by `10^152`.
  let three: D1x0 = "3".parse().unwrap();
  let quarter: D0x153 = "0.25".parse().unwrap();
  let unit: D1x0 = "1".parse().unwrap();
  for (mode, (third, tie)) in
    bigint::MODES
      .into_iter()
      .zip([(4, 3), (3, 2), (4, 3), (3, 2), (3, 3), (3, 2), (3, 2)])
  {
    assert_eq!(
      D0x153::MAX
        .div_round::<U1, U1, i8, _, _, _>(&three, mode)
        .map(|q| q.repr()),
      Ok(third),
      "{mode:?}"
    );
    assert_eq!(
      quarter
        .div_round::<U1, U1, i8, _, _, _>(&unit, mode)
        .map(|q| q.repr()),
      Ok(tie),
      "{mode:?}"
    );
  }

  let tick: Decimal<U10, U8, u64> = "0.00000001".parse().unwrap();
  let expected = format!("{}.99", nines(144));
  for mode in bigint::MODES {
    let quotient: D144x10u512 = D136x10u512::MAX.div_round(&tick, mode).unwrap();
    assert_eq!(quotient.to_string(), expected, "{mode:?}");
  }
}

/// The mode that rounds `-x` as `mode` rounds `x`, negated.
fn mirror(mode: RoundingMode) -> RoundingMode {
  match mode {
    RoundingMode::Ceiling => RoundingMode::Floor,
    RoundingMode::Floor => RoundingMode::Ceiling,
    other => other,
  }
}

#[cfg(feature = "serde")]
mod with_serde {
  use serde_test::{Configure, Token, assert_de_tokens, assert_de_tokens_error, assert_tokens};
  use std::{format, string::ToString};

  use super::{
    D0x153, D0x154u512, D1x1, D1x1u8, D2x2, D153x0, D154x0u512, Decimal, Price, TokenAmount, U1,
    U2, nines,
  };
  use crate::{RoundingMode, bigint};

  #[test]
  fn decimals_are_strings_in_every_format() {
    let price: Price = "-12.5".parse().unwrap();
    assert_tokens(&price.readable(), &[Token::Str("-12.5")]);
    assert_tokens(&price.compact(), &[Token::Str("-12.5")]);
    let amount: TokenAmount = "1.000000000000000001".parse().unwrap();
    assert_tokens(&amount.readable(), &[Token::Str("1.000000000000000001")]);
    assert_tokens(&amount.compact(), &[Token::Str("1.000000000000000001")]);
    assert_tokens(&D1x1u8::MAX.compact(), &[Token::Str("9.9")]);
    // Borrowed and owned strings read the same text, with the full grammar of `parse`.
    assert_de_tokens(&price, &[Token::BorrowedStr("-1_25e-1")]);
    assert_de_tokens(&price, &[Token::String("-12.500")]);
  }

  #[test]
  fn decimals_reject_everything_but_strings() {
    assert_de_tokens_error::<Price>(
      &[Token::F64(16.7)],
      "invalid type: floating point `16.7`, expected a decimal number as a string",
    );
    assert_de_tokens_error::<Price>(
      &[Token::I64(167)],
      "invalid type: integer `167`, expected a decimal number as a string",
    );
    assert_de_tokens_error::<Price>(
      &[Token::Bytes(b"1.5")],
      "invalid type: byte array, expected a decimal number as a string",
    );
    // A string that does not parse reports why.
    assert_de_tokens_error::<Price>(&[Token::Str("1.5x")], "invalid decimal syntax");
    assert_de_tokens_error::<D2x2>(
      &[Token::Str("0.001")],
      "decimal cannot be represented exactly in the target type",
    );
    assert_de_tokens_error::<D1x1u8>(&[Token::Str("-1")], "invalid decimal syntax");
  }

  #[test]
  fn json_round_trips() {
    let price: Price = "64250.5".parse().unwrap();
    assert_eq!(serde_json::to_string(&price).unwrap(), "\"64250.5\"");
    assert_eq!(serde_json::from_str::<Price>("\"64250.5\"").unwrap(), price);
    // The widest texts, 156 bytes each, and the integers with the most digits.
    assert_eq!(
      serde_json::to_string(&D0x154u512::MAX).unwrap(),
      format!("\"0.{}\"", nines(154))
    );
    assert_eq!(
      serde_json::to_string(&D0x153::MIN).unwrap(),
      format!("\"-0.{}\"", nines(153))
    );
    assert_eq!(
      serde_json::to_string(&D154x0u512::MAX).unwrap(),
      format!("\"{}\"", nines(154))
    );
    assert_eq!(
      serde_json::to_string(&D153x0::MIN).unwrap(),
      format!("\"-{}\"", nines(153))
    );
    for value in [D0x154u512::MAX, D0x154u512::ZERO] {
      let json = serde_json::to_string(&value).unwrap();
      assert_eq!(serde_json::from_str::<D0x154u512>(&json).unwrap(), value);
    }
    for value in [D0x153::MIN, D0x153::MAX] {
      let json = serde_json::to_string(&value).unwrap();
      assert_eq!(serde_json::from_str::<D0x153>(&json).unwrap(), value);
    }
    let json = serde_json::to_string(&D154x0u512::MAX).unwrap();
    assert_eq!(
      serde_json::from_str::<D154x0u512>(&json).unwrap(),
      D154x0u512::MAX
    );
    let json = serde_json::to_string(&D153x0::MIN).unwrap();
    assert_eq!(serde_json::from_str::<D153x0>(&json).unwrap(), D153x0::MIN);
  }

  /// A JSON number would pass through `f64`, which holds `16.7` only approximately.
  #[test]
  fn json_reads_text_exactly_and_rejects_numbers() {
    type D2x1 = Decimal<U2, U1>;
    let value: D2x1 = serde_json::from_str("\"16.7\"").unwrap();
    assert_eq!(value.repr(), 167);
    let err = serde_json::from_str::<D2x1>("16.7").unwrap_err();
    assert!(err.is_data(), "{err}");
    assert!(
      err
        .to_string()
        .starts_with("invalid type: floating point `16.7`"),
      "{err}"
    );
    let err = serde_json::from_str::<D2x1>("16").unwrap_err();
    assert!(
      err.to_string().starts_with("invalid type: integer `16`"),
      "{err}"
    );
    let err = serde_json::from_str::<D2x1>("\"16.7.1\"").unwrap_err();
    assert!(
      err.to_string().starts_with("invalid decimal syntax"),
      "{err}"
    );
  }

  #[test]
  fn negative_zero_reads_as_zero() {
    let zero: D1x1 = serde_json::from_str("\"-0.0\"").unwrap();
    assert!(zero.is_zero() && !zero.is_negative());
    assert_eq!(serde_json::to_string(&zero).unwrap(), "\"0\"");
  }

  #[test]
  fn rounding_modes_are_their_names() {
    let names = [
      "up", "down", "ceiling", "floor", "halfup", "halfdown", "halfeven",
    ];
    for (mode, name) in bigint::MODES.into_iter().zip(names) {
      assert_eq!(mode.as_str(), name);
      assert_tokens(&mode.readable(), &[Token::Str(name)]);
      assert_tokens(&mode.compact(), &[Token::Str(name)]);
      let json = format!("\"{name}\"");
      assert_eq!(serde_json::to_string(&mode).unwrap(), json);
      assert_eq!(serde_json::from_str::<RoundingMode>(&json).unwrap(), mode);
    }

    // Reading ignores the case and the whitespace around the name, and a half mode may separate
    // its two words with one space, `-` or `_`.
    let accepted: [(RoundingMode, &[&str]); 7] = [
      (
        RoundingMode::Up,
        &["up", "Up", "UP", " up", "up ", "\t\nUp\r "],
      ),
      (RoundingMode::Down, &["down", "Down", "DOWN", " down "]),
      (
        RoundingMode::Ceiling,
        &["ceiling", "Ceiling", "CEILING", " ceiling "],
      ),
      (RoundingMode::Floor, &["floor", "Floor", "FLOOR", " floor "]),
      (
        RoundingMode::HalfUp,
        &[
          "halfup",
          "HalfUp",
          "HALFUP",
          "half up",
          "Half Up",
          "half-up",
          "HALF-UP",
          "half_up",
          "Half_Up",
          " half up ",
        ],
      ),
      (
        RoundingMode::HalfDown,
        &[
          "halfdown",
          "HalfDown",
          "HALFDOWN",
          "half down",
          "Half Down",
          "half-down",
          "HALF-DOWN",
          "half_down",
          "Half_Down",
          " half down ",
        ],
      ),
      (
        RoundingMode::HalfEven,
        &[
          "halfeven",
          "HalfEven",
          "halfEven",
          "HALFEVEN",
          "half even",
          "Half Even",
          "half-even",
          "HALF-EVEN",
          "half_even",
          "Half_Even",
          " half even ",
          "\thalf-even\n",
        ],
      ),
    ];
    for (mode, inputs) in accepted {
      for input in inputs {
        assert_de_tokens(&mode, &[Token::Str(input)]);
        let json = format!("\"{}\"", input.escape_default());
        assert_eq!(
          serde_json::from_str::<RoundingMode>(&json).unwrap(),
          mode,
          "{input:?} reads as {mode:?}"
        );
      }
    }

    // Anything else is rejected, and the error names the trimmed input and the written names.
    let expected = "`up`, `down`, `ceiling`, `floor`, `halfup`, `halfdown`, `halfeven`";
    for rejected in [
      "half",
      "halfeven2",
      "half  even",
      "half--even",
      "half-_even",
      "half\teven",
      "round_half_even",
      "ROUND_HALF_EVEN",
      "ups",
      "",
      " ",
    ] {
      assert_de_tokens_error::<RoundingMode>(
        &[Token::Str(rejected)],
        &format!(
          "unknown variant `{}`, expected one of {expected}",
          rejected.trim()
        ),
      );
    }
    assert_de_tokens_error::<RoundingMode>(
      &[Token::U8(6)],
      "invalid type: integer `6`, expected the name of a rounding mode as a string",
    );
    let err = serde_json::from_str::<RoundingMode>("6").unwrap_err();
    assert!(
      err.to_string().starts_with("invalid type: integer `6`"),
      "{err}"
    );
    let err = serde_json::from_str::<RoundingMode>("\"ROUND_HALF_EVEN\"").unwrap_err();
    assert!(err.to_string().starts_with("unknown variant"), "{err}");
  }
}
