use core::{
  cmp::Ordering,
  fmt::{self, Write as _},
  hash::BuildHasher,
  str::FromStr,
};
use std::{
  collections::{HashSet, hash_map::RandomState},
  format,
  string::{String, ToString},
};

use typenum::{U0, U1, U2, U4, U8, U10, U11, U12, U16, U18, U20, U22, U36, U40, U153, U154};

use super::Scaled;
use crate::{
  ConstDecimalOne, ConstFixedDecimal, ConvertError, ConvertTo, Decimal, DecimalOne, DivRound,
  DivideError, ExactMul, FixedDecimal, ParseError, ParseErrorKind, QuantizeError, RoundingMode,
  SignedFixedDecimal, UnsignedFixedDecimal, WidenTo, bigint::MODES, i256, i512, u256, u512,
};

/// 20 integral and 18 fraction digits in an `i128`.
type Wide = Decimal<U20, U18>;
type WideS = Scaled<Wide>;
/// 11 integral and 8 fraction digits in a `u64`.
type Narrow = Decimal<U11, U8, u64>;
type NarrowS = Scaled<Narrow>;
type Price = Scaled<Decimal<U10, U8>>;
type Quantity = Scaled<Decimal<U12, U8>>;
type Notional = Scaled<Decimal<U22, U16>>;
type Cents = Scaled<Decimal<U10, U2>>;
/// The product of two `WideS`.
type WideNotional = Scaled<Decimal<U40, U36, i256>>;

fn value<T: FromStr<Err = ParseError>>(text: &str) -> T {
  text.parse().unwrap_or_else(|err| panic!("{text:?}: {err}"))
}

fn kind<T: FromStr<Err = ParseError>>(text: &str) -> Result<T, ParseErrorKind> {
  text.parse().map_err(|err: ParseError| err.kind())
}

/// Asserts that `scaled` writes `text` and has the scale `scale`.
fn assert_text<D: FixedDecimal>(scaled: &Scaled<D>, text: &str, scale: u8) {
  assert_eq!(scaled.to_string(), text, "text of {scaled:?}");
  assert_eq!(scaled.scale(), scale, "scale of {text}");
}

/// Texts with as many fraction digits as their writer chose, each with its scale.
const EXAMPLES: [(&str, u8); 24] = [
  ("0.00100000", 8),
  ("4.00000100", 8),
  ("48.000012", 6),
  ("0.052", 3),
  ("5838", 0),
  ("0", 0),
  ("10000.000000", 6),
  ("9999999999.0000000000000000", 16),
  ("29792.0", 1),
  ("0.0", 1),
  ("10000.00", 2),
  ("0.0010", 4),
  ("0.001", 3),
  ("0.00000192834", 11),
  ("-0.00000192834", 11),
  ("4850.435693622894", 12),
  ("4834.3170936228935", 13),
  ("92233720368.54775807", 8),
  ("18.62041381", 8),
  ("1852229076.12716007", 8),
  ("100000000000000000", 0),
  ("0.026868", 6),
  ("19.99500000", 8),
  ("0.00000112", 8),
];

#[test]
fn parse_keeps_the_scale_and_reproduces_the_text() {
  for (text, scale) in EXAMPLES {
    let scaled: WideS = value(text);
    assert_text(&scaled, text, scale);
    assert_eq!(*scaled.value(), value::<Wide>(text), "{text}");
    // The inherent `parse` and `FromStr` agree.
    assert_eq!(
      WideS::parse(text).map(Scaled::into_parts),
      Ok(scaled.into_parts()),
      "{text}"
    );
  }
  // The unsigned 19-digit type rejects the texts that it cannot hold, with the kinds of
  // `Decimal::parse`.
  let rejected = [
    ("0.00000192834", ParseErrorKind::Inexact),
    ("-0.00000192834", ParseErrorKind::Negative),
    ("4850.435693622894", ParseErrorKind::Inexact),
    ("4834.3170936228935", ParseErrorKind::Inexact),
    ("100000000000000000", ParseErrorKind::OutOfRange),
  ];
  for (text, scale) in EXAMPLES {
    match rejected.iter().find(|(rejected, _)| *rejected == text) {
      Some(&(_, expected)) => {
        assert_eq!(kind::<NarrowS>(text), Err(expected), "{text}");
        assert_eq!(kind::<Narrow>(text), Err(expected), "{text}");
      }
      None => assert_text(&value::<NarrowS>(text), text, scale),
    }
  }
  // The scale may exceed the fraction digits of the type.
  let padded: NarrowS = value("9999999999.0000000000000000");
  assert_text(&padded, "9999999999.0000000000000000", 16);
  assert_eq!(padded, value("9999999999"));
}

#[test]
fn exponents_and_lenient_forms() {
  for (text, written, scale) in [
    ("1e-8", "0.00000001", 8),
    ("1E+2", "100", 0),
    ("25e-1", "2.5", 1),
    ("250e-2", "2.50", 2),
    ("250_000e-5", "2.50000", 5),
    ("1.50e1", "15.0", 1),
    ("1.5e1", "15", 0),
    ("1.5e3", "1500", 0),
    ("0e-5", "0.00000", 5),
    ("0.0e2", "0", 0),
    ("0e99999999999999999999", "0", 0),
    ("+1.5", "1.5", 1),
    ("1_000.000_1", "1000.0001", 4),
    ("007.10", "7.10", 2),
    ("5.", "5", 0),
    (".5", "0.5", 1),
    ("+.5", "0.5", 1),
    ("1.2_3_", "1.23", 2),
    ("1.0_0e+1", "10.0", 1),
    ("1e_0_1", "10", 0),
    ("1e-0_2", "0.01", 2),
  ] {
    assert_text(&value::<WideS>(text), written, scale);
    assert_text(&value::<NarrowS>(text), written, scale);
  }
  // A signed storage writes a negative zero back without its sign.
  assert_text(&value::<WideS>("-0"), "0", 0);
  assert_text(&value::<WideS>("-0.0"), "0.0", 1);
  assert_text(&value::<WideS>("-0e-3"), "0.000", 3);
}

#[test]
fn scale_limits() {
  let largest = format!("0.{}", "0".repeat(255));
  assert_text(&value::<WideS>(&largest), &largest, 255);
  assert_text(&value::<NarrowS>(&largest), &largest, 255);
  assert_text(&value::<WideS>("0e-255"), &largest, 255);
  let unit = format!("0.{}1{}", "0".repeat(17), "0".repeat(237));
  assert_text(&value::<WideS>(&unit), &unit, 255);
  let one = format!("1.{}", "0".repeat(255));
  assert_text(&value::<NarrowS>(&one), &one, 255);

  // 256 fraction digits or more, however the text writes them, once the value fits.
  let too_many = format!("0.{}", "0".repeat(256));
  let padded_one = format!("1.{}", "0".repeat(256));
  let exact_ten = format!("1{}e-256", "0".repeat(257));
  for text in [
    too_many.as_str(),
    padded_one.as_str(),
    exact_ten.as_str(),
    "0e-256",
    "0.0e-255",
    "0e-99999999999999999999",
    "0e-99999999999999999999999999999999999999",
    "0.000e-18446744073709551615",
  ] {
    assert_eq!(
      kind::<WideS>(text),
      Err(ParseErrorKind::ScaleTooLarge),
      "{text}"
    );
    assert_eq!(
      kind::<NarrowS>(text),
      Err(ParseErrorKind::ScaleTooLarge),
      "{text}"
    );
  }

  // The value is decided before the scale: these fail for their digits, which every type
  // decides alike.
  for (text, expected) in [
    ("1e-99999999999999999999", ParseErrorKind::Inexact),
    ("1e99999999999999999999", ParseErrorKind::OutOfRange),
    (&format!("1.{}1", "0".repeat(300)), ParseErrorKind::Inexact),
    (
      &format!("1{}.{}", "0".repeat(30), "0".repeat(300)),
      ParseErrorKind::OutOfRange,
    ),
  ] {
    assert_eq!(kind::<WideS>(text), Err(expected), "{text}");
    assert_eq!(kind::<NarrowS>(text), Err(expected), "{text}");
  }
  // The sign of a negative text for an unsigned storage is decided before the scale, and the
  // syntax before everything.
  let negative = format!("-0.{}", "0".repeat(256));
  assert_eq!(kind::<NarrowS>(&negative), Err(ParseErrorKind::Negative));
  assert_eq!(kind::<WideS>(&negative), Err(ParseErrorKind::ScaleTooLarge));
  let garbled = format!("0.{}x", "0".repeat(256));
  assert_eq!(kind::<WideS>(&garbled), Err(ParseErrorKind::InvalidSyntax));
  assert_eq!(
    kind::<NarrowS>(&garbled),
    Err(ParseErrorKind::InvalidSyntax)
  );
  assert_eq!(
    kind::<WideS>(&format!("-inf{}", "0".repeat(256))),
    Err(ParseErrorKind::InvalidSyntax)
  );
}

#[test]
fn decimal_parse_never_reports_scale_too_large() {
  let long = format!("0.{}", "0".repeat(300));
  assert_eq!(Wide::parse(&long), Ok(Wide::ZERO));
  assert_eq!(Narrow::parse(&long), Ok(Narrow::ZERO));
  assert_eq!(Wide::parse("0e-99999999999999999999"), Ok(Wide::ZERO));
  assert_eq!(
    Wide::parse(&format!("1.{}", "0".repeat(300))),
    Ok(Wide::ONE)
  );
  assert_eq!(
    Narrow::parse(&format!("1{}e-256", "0".repeat(257))).map(Narrow::repr),
    Ok(1_000_000_000)
  );
}

#[test]
fn parse_kinds_are_those_of_the_decimal() {
  let table = [
    ("", ParseErrorKind::Empty),
    ("abc", ParseErrorKind::InvalidSyntax),
    ("-abc", ParseErrorKind::InvalidSyntax),
    ("-", ParseErrorKind::InvalidSyntax),
    ("-+5", ParseErrorKind::InvalidSyntax),
    ("--5", ParseErrorKind::InvalidSyntax),
    ("-+inf", ParseErrorKind::InvalidSyntax),
    ("-1e9_x", ParseErrorKind::InvalidSyntax),
    ("-1 ", ParseErrorKind::InvalidSyntax),
    ("inf", ParseErrorKind::NotFinite),
    ("-inf", ParseErrorKind::NotFinite),
    ("-Infinity", ParseErrorKind::NotFinite),
    ("-NaN", ParseErrorKind::NotFinite),
    ("-sNaN12", ParseErrorKind::NotFinite),
    ("-0.5", ParseErrorKind::Negative),
    ("-0", ParseErrorKind::Negative),
    ("-0.0", ParseErrorKind::Negative),
    ("-0e5", ParseErrorKind::Negative),
    ("-1e30", ParseErrorKind::Negative),
    ("-0.000000001", ParseErrorKind::Negative),
    ("-99999999999.99999999", ParseErrorKind::Negative),
    ("-.5", ParseErrorKind::Negative),
    ("-5.", ParseErrorKind::Negative),
    ("1e30", ParseErrorKind::OutOfRange),
    ("0.000000001", ParseErrorKind::Inexact),
  ];
  for (text, expected) in table {
    assert_eq!(kind::<NarrowS>(text), Err(expected), "Scaled {text:?}");
    assert_eq!(
      NarrowS::parse(text).map_err(|err| err.kind()),
      Err(expected),
      "Scaled::parse {text:?}"
    );
    assert_eq!(kind::<Narrow>(text), Err(expected), "Decimal {text:?}");
  }
  // A signed storage parses a negative number, or rejects it for its digits.
  assert_text(&value::<WideS>("-0.5"), "-0.5", 1);
  assert_eq!(kind::<WideS>("-1e30"), Err(ParseErrorKind::OutOfRange));
  assert_eq!(kind::<WideS>("-1e-30"), Err(ParseErrorKind::Inexact));
  assert_eq!(kind::<WideS>("-abc"), Err(ParseErrorKind::InvalidSyntax));
  assert_eq!(kind::<WideS>("-inf"), Err(ParseErrorKind::NotFinite));
  // The messages name no digit of the input.
  let negative = "-7351946".parse::<NarrowS>().unwrap_err();
  assert!(negative.kind().is_negative());
  assert!(!negative.to_string().contains("7351946"));
  let too_large = "0e-999".parse::<WideS>().unwrap_err();
  assert!(too_large.kind().is_scale_too_large());
  assert_eq!(
    too_large.to_string(),
    "decimal has more than 255 fraction digits"
  );
}

#[test]
fn parse_at_the_storage_limits() {
  type D154u512 = Scaled<Decimal<U154, U0, u512>>;
  type D0x153i512 = Scaled<Decimal<U0, U153, i512>>;
  let max = format!("{}.{}", "9".repeat(154), "0".repeat(255));
  let widest: D154u512 = value(&max);
  assert_text(&widest, &max, 255);
  assert_eq!(*widest.value(), Decimal::<U154, U0, u512>::MAX);
  let min = format!("-0.{}{}", "9".repeat(153), "0".repeat(102));
  assert_text(&value::<D0x153i512>(&min), &min, 255);
  let small: Scaled<Decimal<U1, U1, u8>> = value("9.90");
  assert_text(&small, "9.90", 2);
  let fraction_only: Scaled<Decimal<U0, U2>> = value("-0.50");
  assert_text(&fraction_only, "-0.50", 2);
}

#[test]
fn display_flags_and_precision() {
  let positive: WideS = value("1.25");
  let negative: WideS = value("-1.25");
  let padded: WideS = value("0.00100000");
  assert_eq!(format!("{positive}"), "1.25");
  assert_eq!(format!("{positive:>8}"), "    1.25");
  assert_eq!(format!("{positive:<8}|"), "1.25    |");
  assert_eq!(format!("{positive:*^8}"), "**1.25**");
  assert_eq!(format!("{positive:+}"), "+1.25");
  assert_eq!(format!("{positive:08}"), "00001.25");
  assert_eq!(format!("{positive:+08}"), "+0001.25");
  assert_eq!(format!("{positive:+8.3}"), "  +1.250");
  assert_eq!(format!("{negative:08.3}"), "-001.250");
  assert_eq!(format!("{negative:>8}"), "   -1.25");
  assert_eq!(format!("{negative:+}"), "-1.25");
  assert_eq!(format!("{padded:>14}"), "    0.00100000");
  assert_eq!(format!("{positive:.4}"), "1.2500");
  assert_eq!(format!("{positive:.1}"), "1.25");
  assert_eq!(format!("{positive:.0}"), "1.25");
  assert_eq!(format!("{padded:.10}"), "0.0010000000");
  assert_eq!(format!("{padded:.3}"), "0.00100000");
  assert_eq!(format!("{}", value::<WideS>("0.000")), "0.000");
  assert_eq!(format!("{:.3}", WideS::default()), "0.000");
  assert_eq!(format!("{:+}", value::<WideS>("0.0")), "+0.0");
  let unsigned: NarrowS = value("1.25");
  assert_eq!(format!("{unsigned:+08.3}"), "+001.250");

  // With scale 0, every flag writes what it writes for the decimal, byte for byte.
  for text in [
    "-12.3",
    "0",
    "99.99",
    "-0.05",
    "1",
    "-99999999999999999999.999999999999999999",
  ] {
    let decimal: Wide = value(text);
    let scaled = WideS::from(decimal);
    for (decimal_text, scaled_text) in [
      (format!("{decimal}"), format!("{scaled}")),
      (format!("{decimal:>9}"), format!("{scaled:>9}")),
      (format!("{decimal:<9}"), format!("{scaled:<9}")),
      (format!("{decimal:^9}"), format!("{scaled:^9}")),
      (format!("{decimal:*^9}"), format!("{scaled:*^9}")),
      (format!("{decimal:+}"), format!("{scaled:+}")),
      (format!("{decimal:09}"), format!("{scaled:09}")),
      (format!("{decimal:+09.3}"), format!("{scaled:+09.3}")),
      (format!("{decimal:.5}"), format!("{scaled:.5}")),
      (format!("{decimal:.1}"), format!("{scaled:.1}")),
      (format!("{decimal:.30}"), format!("{scaled:.30}")),
    ] {
      assert_eq!(decimal_text, scaled_text, "{text}");
    }
  }
}

/// A writer that accepts at most `remaining` bytes and then fails.
struct Limited {
  remaining: usize,
}

impl fmt::Write for Limited {
  fn write_str(&mut self, s: &str) -> fmt::Result {
    self.remaining = self.remaining.checked_sub(s.len()).ok_or(fmt::Error)?;
    Ok(())
  }
}

#[test]
fn display_streams_a_precision_beyond_any_scale() {
  let positive: WideS = value("1.25");
  let negative: WideS = value("-1.25");
  let min: WideS = value("-99999999999999999999.999999999999999999");
  let digits = format!("99999999999999999999.{}{}", "9".repeat(18), "0".repeat(237));
  assert_eq!(format!("{min:.255}"), format!("-{digits}"));
  assert_eq!(format!("{min:>280.255}"), format!("   -{digits}"));
  assert_eq!(format!("{min:0280.255}"), format!("-000{digits}"));
  assert_eq!(
    format!("{positive:.300}"),
    format!("1.25{}", "0".repeat(298))
  );
  assert_eq!(
    format!("{positive:>310.300}"),
    format!("{}1.25{}", " ".repeat(8), "0".repeat(298))
  );
  assert_eq!(
    format!("{negative:0700.600}"),
    format!("-{}1.25{}", "0".repeat(97), "0".repeat(598))
  );
  assert_eq!(
    format!("{positive:*<+700.600}"),
    format!("+1.25{}{}", "0".repeat(598), "*".repeat(97))
  );
  // The zeros are streamed: a sink that fails stops the formatting, and a sink that holds the
  // text receives exactly its bytes. The formatter caps a width or a precision at `u16::MAX`.
  let huge = usize::from(u16::MAX);
  let mut sink = Limited { remaining: 10 };
  assert_eq!(write!(sink, "{positive:.huge$}"), Err(fmt::Error));
  let mut sink = Limited { remaining: 10 };
  assert_eq!(write!(sink, "{positive:>huge$.300}"), Err(fmt::Error));
  let mut sink = Limited { remaining: 70_000 };
  assert_eq!(write!(sink, "{positive:.huge$}"), Ok(()));
  assert_eq!(sink.remaining, 70_000 - (2 + huge));
  let padded = Scaled::new(*positive.value(), u8::MAX);
  let mut sink = Limited { remaining: 300 };
  assert_eq!(write!(sink, "{padded}"), Ok(()));
  assert_eq!(sink.remaining, 300 - (2 + 255));
}

#[test]
fn equality_ordering_and_hashing_compare_the_value_only() {
  let short: WideS = value("0.001");
  let padded: WideS = value("0.00100000");
  assert_eq!(short, padded);
  assert_ne!(short.scale(), padded.scale());
  assert_eq!(short.cmp(&padded), Ordering::Equal);
  assert_eq!(short.partial_cmp(&padded), Some(Ordering::Equal));
  assert!(value::<WideS>("0.000999") < short);
  assert!(value::<WideS>("-0.001") < short);
  assert!(value::<WideS>("0.0011") > padded);
  let state = RandomState::new();
  assert_eq!(state.hash_one(short), state.hash_one(padded));
  assert_eq!(state.hash_one(short), state.hash_one(short.value()));
  assert_eq!(HashSet::from([short, padded]).len(), 1);
  let narrow: [NarrowS; 2] = [value("0.001"), value("0.00100000")];
  assert_eq!(narrow[0], narrow[1]);
  assert_eq!(HashSet::from(narrow).len(), 1);
}

#[test]
fn construction_and_accessors() {
  let padded: WideS = value("0.00100000");
  let (decimal, scale) = padded.into_parts();
  assert_eq!((decimal, scale), (*padded.value(), 8));
  assert_eq!(padded.into_value(), decimal);
  assert_text(&Scaled::new(decimal, scale), "0.00100000", 8);
  assert_text(&Scaled::new(decimal, 3), "0.001", 3);
  // A scale below the digits of the value never cuts them.
  assert_text(&Scaled::new(decimal, 0), "0.001", 0);
  // A value made from a decimal has scale 0, so its text is the shortest exact one.
  assert_text(&WideS::from(decimal), "0.001", 0);
  assert_text(&WideS::default(), "0", 0);
  assert_eq!(*WideS::default().value(), Wide::ZERO);
  assert_text(&NarrowS::default(), "0", 0);
}

#[test]
fn debug_shows_the_scale() {
  assert_eq!(
    format!("{:?}", value::<WideS>("0.00100000")),
    "Scaled { value: 0.001, scale: 8 }"
  );
  assert_eq!(
    format!("{:?}", value::<NarrowS>("5838")),
    "Scaled { value: 5838, scale: 0 }"
  );
}

#[test]
fn parse_and_accessors_in_const_context() {
  const PADDED: WideS = match WideS::parse("0.00100000") {
    Ok(value) => value,
    Err(_) => panic!("valid"),
  };
  const NARROW: NarrowS = match NarrowS::parse("1e-8") {
    Ok(value) => value,
    Err(_) => panic!("valid"),
  };
  const NEGATIVE: Result<NarrowS, ParseError> = NarrowS::parse("-1");
  const TOO_LARGE: Result<WideS, ParseError> = WideS::parse("0e-256");
  const SCALE: u8 = PADDED.scale();
  const DECIMAL: Wide = *PADDED.value();
  const MADE: Scaled<Decimal<U2, U4>> = Scaled::new(Decimal::<U2, U4>::ONE, 4);
  assert_text(&PADDED, "0.00100000", 8);
  assert_text(&NARROW, "0.00000001", 8);
  assert_eq!(
    NEGATIVE.map_err(|err| err.kind()),
    Err(ParseErrorKind::Negative)
  );
  assert_eq!(
    TOO_LARGE.map_err(|err| err.kind()),
    Err(ParseErrorKind::ScaleTooLarge)
  );
  assert_eq!(SCALE, 8);
  assert_eq!(DECIMAL.to_string(), "0.001");
  assert_text(&MADE, "1.0000", 4);
}

#[test]
fn layout() {
  type Bare = Decimal<U10, U8>;
  assert_eq!(size_of::<Scaled<Decimal<U1, U1, u8>>>(), 2);
  // The scale takes the padding to the alignment of the decimal.
  assert_eq!(size_of::<Price>(), size_of::<Bare>() + align_of::<Bare>());
  assert_eq!(size_of::<WideS>(), size_of::<Wide>() + align_of::<Wide>());
  // The scale is a plain `u8`, so an `Option` has no niche.
  assert_eq!(
    size_of::<Option<WideS>>(),
    size_of::<WideS>() + align_of::<WideS>()
  );
}

#[test]
fn scale_rules_of_derived_values() {
  let short: WideS = value("0.001");
  let padded: WideS = value("0.00100000");
  // A sum or a difference takes the larger scale.
  assert_text(&short.checked_add(&padded).unwrap(), "0.00200000", 8);
  assert_text(&padded.checked_add(&short).unwrap(), "0.00200000", 8);
  assert_text(&short.checked_sub(&padded).unwrap(), "0.00000000", 8);
  assert_text(&short.checked_add(&short).unwrap(), "0.002", 3);
  assert_eq!(WideS::MAX.checked_add(&short), None);
  assert_eq!(NarrowS::ZERO.checked_sub(&value("0.001")), None);

  // An exact product takes the sum of the scales, saturating at 255; the value is never rounded.
  let price: Price = value("64250.5");
  let quantity: Quantity = value("0.0015000");
  let notional: Notional = price.exact_mul(&quantity);
  assert_text(&notional, "96.37575000", 8);
  let saturated =
    Scaled::new(price.into_value(), 200).exact_mul(&Scaled::new(quantity.into_value(), 100));
  assert_eq!(saturated.scale(), 255);
  assert_eq!(*saturated.value(), *notional.value());
  assert_eq!(saturated.to_string().len(), "96".len() + 1 + 255);

  // Quantizing takes the scale of the step.
  let tick: Price = value("0.10");
  assert_text(
    &price.quantize(&tick, RoundingMode::HalfEven).unwrap(),
    "64250.50",
    2,
  );
  assert_text(
    &value::<Price>("64250.55")
      .quantize(&value("0.1"), RoundingMode::Down)
      .unwrap(),
    "64250.5",
    1,
  );
  assert_eq!(
    price.quantize(&Price::ZERO, RoundingMode::Up),
    Err(QuantizeError::InvalidStep)
  );
  assert!(price.is_multiple_of(&tick));
  assert!(!value::<Price>("64250.55").is_multiple_of(&tick));

  // Widening and an exact conversion keep the scale; rounding takes the fraction digits of the
  // target.
  let widened: WideS = price.widen();
  assert_text(&widened, "64250.5", 1);
  let cents: Cents = value::<Price>("2.5000").try_convert().unwrap();
  assert_text(&cents, "2.5000", 4);
  assert_eq!(
    ConvertTo::<Cents>::try_convert(value::<Price>("2.345")),
    Err(ConvertError::Inexact)
  );
  let rounded: Cents = value::<Price>("2.345")
    .round_to(RoundingMode::HalfEven)
    .unwrap();
  assert_text(&rounded, "2.34", 2);
  let exact: WideS = value::<Price>("2.5")
    .round_to(RoundingMode::HalfUp)
    .unwrap();
  assert_text(&exact, "2.500000000000000000", 18);

  // A quotient takes the fraction digits of the target.
  let budget: Scaled<Decimal<U12, U2>> = value("1000.00");
  let bought: Quantity = budget.div_round(&price, RoundingMode::Down).unwrap();
  assert_text(&bought, "0.01556408", 8);
  assert_eq!(
    DivRound::<Price, Quantity>::div_round(&budget, &Price::ZERO, RoundingMode::Down),
    Err(DivideError::DivisionByZero)
  );
  let quarter: WideS = value::<WideS>("-0.1")
    .div_round(&value::<WideS>("4"), RoundingMode::Floor)
    .unwrap();
  assert_text(&quarter, "-0.025000000000000000", 18);

  // Negation and the absolute value keep the scale.
  let fee: WideS = value("-0.00000192834");
  assert_text(&-fee, "0.00000192834", 11);
  assert_text(&SignedFixedDecimal::neg(&fee), "0.00000192834", 11);
  assert_text(&fee.abs(), "0.00000192834", 11);
  assert!(fee.is_negative() && !fee.abs().is_negative());
  assert_text(&-value::<WideS>("1.000"), "-1.000", 3);

  // Zero, the limits, one and a value made from its coefficient have scale 0.
  assert_text(&WideS::zero(), "0", 0);
  assert_text(
    &WideS::max_value(),
    "99999999999999999999.999999999999999999",
    0,
  );
  assert_text(&Price::one(), "1", 0);
  assert_text(&Price::ONE, "1", 0);
  assert_text(&Price::from_repr(150_000_000).unwrap(), "1.5", 0);
  assert_eq!(Price::from_repr(1_000_000_000_000_000_000), None);
}

/// Asserts, for a `Scaled` over each decimal type, that each trait method gives the value of the
/// decimal's method, with the scale of its rule.
macro_rules! assert_agrees {
  ($($decimal:ty),+ $(,)?) => {$({
    type D = $decimal;
    type S = Scaled<D>;
    let a: S = value("1.5");
    let b: S = value("0.25000");
    let step: S = value("0.5");
    assert_eq!(
      a.checked_add(&b).map(Scaled::into_parts),
      a.value().checked_add(b.value()).map(|sum| (sum, 5))
    );
    assert_eq!(
      a.checked_sub(&b).map(Scaled::into_parts),
      a.value().checked_sub(b.value()).map(|difference| (difference, 5))
    );
    for mode in MODES {
      assert_eq!(
        a.quantize(&step, mode).map(Scaled::into_parts),
        a.value().quantize(step.value(), mode).map(|quantized| (quantized, 1))
      );
    }
    assert_eq!(a.is_multiple_of(&step), a.value().is_multiple_of(step.value()));
    assert_eq!(S::zero().into_parts(), (D::ZERO, 0));
    assert_eq!(S::max_value().into_parts(), (D::MAX, 0));
    assert_eq!(S::min_value().into_parts(), (D::MIN, 0));
    assert_eq!(S::one().into_parts(), (D::ONE, 0));
    assert_eq!(<S as ConstFixedDecimal>::ZERO.into_parts(), (D::ZERO, 0));
    assert_eq!(<S as ConstFixedDecimal>::MAX.into_parts(), (D::MAX, 0));
    assert_eq!(<S as ConstFixedDecimal>::MIN.into_parts(), (D::MIN, 0));
    assert_eq!(<S as ConstDecimalOne>::ONE.into_parts(), (D::ONE, 0));
    assert_eq!(S::default().into_parts(), (D::default(), 0));
    assert_eq!(
      S::from_repr(a.into_value().repr()).map(Scaled::into_parts),
      Some((a.into_value(), 0))
    );
    // Equality ignores the scale, so a value equals the one made from its coefficient.
    assert_eq!(S::from_repr(a.repr()), Some(a));
    assert_eq!(FixedDecimal::repr(a), a.into_value().repr());
    assert_eq!(<S as FixedDecimal>::INTEGRAL, D::INTEGRAL);
    assert_eq!(<S as FixedDecimal>::FRACTION, D::FRACTION);
    assert_eq!(<S as FixedDecimal>::DIGITS, D::DIGITS);
    assert_eq!(<S as FixedDecimal>::SIGNED, D::SIGNED);
    assert!(a.is_positive() && !a.is_zero());
    assert!(S::zero().is_zero() && !S::zero().is_positive());
    let product = a.exact_mul(&b);
    assert_eq!(product.scale(), 6);
    assert_eq!(
      product.value().to_string(),
      a.value().exact_mul(b.value()).to_string()
    );
  })+};
}

#[test]
fn trait_results_agree_with_the_decimal() {
  assert_agrees!(
    Decimal<U2, U2>,
    Decimal<U10, U8>,
    Decimal<U20, U18>,
    Decimal<U2, U2, u16>,
    Decimal<U11, U8, u64>,
    Decimal<U20, U18, u128>,
    Decimal<U40, U36, i256>,
    Decimal<U40, U36, u256>,
  );
  fn signed<T: SignedFixedDecimal>() {}
  fn unsigned<T: UnsignedFixedDecimal>() {}
  signed::<WideS>();
  signed::<WideNotional>();
  unsigned::<NarrowS>();
  unsigned::<Scaled<Decimal<U40, U36, u256>>>();
}

#[test]
fn generic_code_takes_scaled_and_bare_decimals_alike() {
  fn total<T: FixedDecimal>(values: &[T]) -> Option<T> {
    values
      .iter()
      .try_fold(T::zero(), |sum, value| sum.checked_add(value))
  }
  fn notional<P: ExactMul<Q>, Q>(fills: &[(P, Q)]) -> Option<P::Output> {
    fills
      .iter()
      .try_fold(P::Output::zero(), |sum, (price, quantity)| {
        sum.checked_add(&price.exact_mul(quantity))
      })
  }
  fn cents<T: ConvertTo<C>, C>(amount: T) -> Result<C, ConvertError> {
    amount.round_to(RoundingMode::HalfEven)
  }
  fn quantity<B: DivRound<P, Q>, P, Q>(budget: &B, price: &P) -> Result<Q, DivideError> {
    budget.div_round(price, RoundingMode::Down)
  }
  fn sum<A: WidenTo<T>, B: WidenTo<T>, T: FixedDecimal>(a: A, b: B) -> Option<T> {
    a.widen().checked_add(&b.widen())
  }
  fn padded<T: FixedDecimal>(value: &T) -> String {
    let digits = T::FRACTION as usize;
    format!("{value:.digits$}")
  }
  // Code generic over the decimal that computes with a `Scaled` of it names the bound.
  fn doubled<D>(value: D) -> Option<Scaled<D>>
  where
    D: FixedDecimal,
    Scaled<D>: FixedDecimal,
  {
    let scaled = Scaled::new(value, 2);
    scaled.checked_add(&scaled)
  }

  let prices: [Price; 2] = [value("64250.5"), value("0.25000")];
  let bare: [Decimal<U10, U8>; 2] = [value("64250.5"), value("0.25")];
  assert_text(&total(&prices).unwrap(), "64250.75000", 5);
  assert_eq!(total(&bare).unwrap().to_string(), "64250.75");

  let fills: [(Price, Quantity); 2] = [
    (value("64250.5"), value("0.0015")),
    (value("64251"), value("0.001")),
  ];
  let bare_fills: [(Decimal<U10, U8>, Decimal<U12, U8>); 2] = [
    (value("64250.5"), value("0.0015")),
    (value("64251"), value("0.001")),
  ];
  let scaled_total: Notional = notional(&fills).unwrap();
  assert_text(&scaled_total, "160.62675", 5);
  let bare_total: Decimal<U22, U16> = notional(&bare_fills).unwrap();
  assert_eq!(bare_total, *scaled_total.value());

  let scaled_cents: Cents = cents(value::<Price>("2.345")).unwrap();
  assert_text(&scaled_cents, "2.34", 2);
  let bare_cents: Decimal<U10, U2> = cents(value::<Decimal<U10, U8>>("2.345")).unwrap();
  assert_eq!(bare_cents, *scaled_cents.value());

  let budget: Scaled<Decimal<U12, U2>> = value("1000");
  let bought: Quantity = quantity(&budget, &prices[0]).unwrap();
  assert_text(&bought, "0.01556408", 8);
  let bare_bought: Decimal<U12, U8> = quantity(budget.value(), &bare[0]).unwrap();
  assert_eq!(bare_bought, *bought.value());

  let fee: Scaled<Decimal<U2, U4, u32>> = value("1.2500");
  assert_text(
    &sum::<_, _, WideS>(fee, prices[0]).unwrap(),
    "64251.7500",
    4,
  );
  let bare_sum = sum::<_, _, Wide>(fee.into_value(), bare[0]).unwrap();
  assert_eq!(bare_sum.to_string(), "64251.75");

  assert_eq!(padded(&prices[1]), "0.25000000");
  assert_eq!(padded(&value::<Price>("0.250000000000")), "0.250000000000");
  assert_eq!(padded(&bare[1]), "0.25000000");

  assert_text(&doubled(value::<Wide>("1.5")).unwrap(), "3.00", 2);
  assert_text(&doubled(value::<Narrow>("0.001")).unwrap(), "0.002", 2);
  assert_eq!(doubled(Wide::MAX), None);
}

#[test]
fn generic_constants_in_const_context() {
  const fn limits<T: ConstFixedDecimal>() -> [T; 3] {
    [T::MIN, T::ZERO, T::MAX]
  }
  const fn units<T: ConstDecimalOne>() -> [T; 2] {
    [T::ZERO, T::ONE]
  }
  const SCALED_LIMITS: [WideS; 3] = limits();
  const BARE_LIMITS: [Wide; 3] = limits();
  const SCALED_UNITS: [NarrowS; 2] = units();
  const BARE_UNITS: [Narrow; 2] = units();
  for (scaled, bare) in SCALED_LIMITS.into_iter().zip(BARE_LIMITS) {
    assert_eq!(scaled.into_parts(), (bare, 0));
  }
  for (scaled, bare) in SCALED_UNITS.into_iter().zip(BARE_UNITS) {
    assert_eq!(scaled.into_parts(), (bare, 0));
  }
  assert_text(&SCALED_UNITS[1], "1", 0);
}

#[test]
fn products_at_the_extremes_and_rounding_in_every_mode() {
  let square = WideS::MAX.exact_mul(&WideS::MAX);
  assert_eq!(square.scale(), 0);
  assert_eq!(
    square.to_string(),
    "9999999999999999999999999999999999999800.000000000000000000000000000000000001"
  );
  let narrow_square = NarrowS::MAX.exact_mul(&NarrowS::MAX);
  assert_eq!(
    narrow_square.to_string(),
    "9999999999999999998000.0000000000000001"
  );
  assert_eq!(
    ConvertTo::<WideS>::try_convert(square),
    Err(ConvertError::OutOfRange)
  );
  assert_eq!(
    ConvertTo::<WideS>::round_to(square, RoundingMode::Down),
    Err(ConvertError::OutOfRange)
  );

  let tiny: WideNotional = value::<WideS>("1e-9").exact_mul(&value::<WideS>("1e-10"));
  assert_text(&tiny, "0.0000000000000000001", 19);
  let tie: WideNotional = value::<WideS>("5e-10").exact_mul(&value::<WideS>("1e-9"));
  for mode in MODES {
    let rounded: WideS = tiny.round_to(mode).unwrap();
    let expected = match mode {
      RoundingMode::Up | RoundingMode::Ceiling => "0.000000000000000001",
      _ => "0.000000000000000000",
    };
    assert_text(&rounded, expected, 18);
    let rounded_tie: WideS = tie.round_to(mode).unwrap();
    let expected_tie = match mode {
      RoundingMode::Up | RoundingMode::Ceiling | RoundingMode::HalfUp => "0.000000000000000001",
      _ => "0.000000000000000000",
    };
    assert_text(&rounded_tie, expected_tie, 18);
  }
  // The exact product keeps every digit of a wide operand, and converts back exactly.
  let wide: WideS = value("92233720368.54775807");
  let unit: WideS = value("0.00000001");
  let product: WideNotional = wide.exact_mul(&unit);
  assert_text(&product, "922.3372036854775807", 16);
  let back: WideS = product.try_convert().unwrap();
  assert_text(&back, "922.3372036854775807", 16);
}

#[test]
fn widening_between_storages_keeps_the_scale() {
  let narrow: NarrowS = value("48.000012");
  let wide: WideS = narrow.widen();
  assert_text(&wide, "48.000012", 6);
  let wider: WideNotional = wide.widen();
  assert_text(&wider, "48.000012", 6);
  let unsigned_wide: Scaled<Decimal<U40, U36, u256>> = narrow.widen();
  assert_text(&unsigned_wide, "48.000012", 6);
  let back: NarrowS = wider.try_convert().unwrap();
  assert_eq!(back.into_parts(), (narrow.into_value(), 6));
  assert_eq!(
    ConvertTo::<NarrowS>::try_convert(value::<WideS>("-1")),
    Err(ConvertError::OutOfRange)
  );
}

#[cfg(feature = "serde")]
mod with_serde {
  use serde_core::{
    Deserialize,
    de::{self, IntoDeserializer},
  };
  use serde_test::{Configure, Token, assert_de_tokens_error, assert_tokens};
  use std::{format, string::String, vec::Vec};

  use super::{Decimal, NarrowS, Scaled, U0, U153, U154, WideS, assert_text, i512, u512, value};

  #[test]
  fn a_value_is_its_scaled_text_in_every_format() {
    for text in [
      "0.00100000",
      "9999999999.0000000000000000",
      "-0.00000192834",
      "5838",
      "0.0",
    ] {
      let wide: WideS = value(text);
      assert_tokens(&wide.readable(), &[Token::Str(text)]);
      assert_tokens(&wide.compact(), &[Token::Str(text)]);
      let json = serde_json::to_string(&wide).unwrap();
      assert_eq!(json, format!("\"{text}\""));
      assert_text(
        &serde_json::from_str::<WideS>(&json).unwrap(),
        text,
        wide.scale(),
      );
      let bytes = postcard::to_stdvec(&wide).unwrap();
      assert_text(
        &postcard::from_bytes::<WideS>(&bytes).unwrap(),
        text,
        wide.scale(),
      );
    }
    // postcard writes the length of the text, then the text.
    let narrow: NarrowS = value("0.00100000");
    let mut bytes = Vec::from([10]);
    bytes.extend_from_slice(b"0.00100000");
    assert_eq!(postcard::to_stdvec(&narrow).unwrap(), bytes);

    // A borrowed and an owned string read with the grammar of `parse` and take the scale.
    let borrowed = de::value::BorrowedStrDeserializer::<de::value::Error>::new("1_50e-2");
    assert_text(&WideS::deserialize(borrowed).unwrap(), "1.50", 2);
    let owned: de::value::StringDeserializer<de::value::Error> =
      String::from("1.5000").into_deserializer();
    assert_text(&WideS::deserialize(owned).unwrap(), "1.5000", 4);
  }

  #[test]
  fn the_widest_texts_serialize() {
    // 154 integral digits, or a sign and 153, and 255 fraction digits: 410 bytes.
    let text = format!("{}.{}", "9".repeat(154), "0".repeat(255));
    let widest = Scaled::new(Decimal::<U154, U0, u512>::MAX, 255);
    assert_eq!(
      serde_json::to_string(&widest).unwrap(),
      format!("\"{text}\"")
    );
    let read: Scaled<Decimal<U154, U0, u512>> =
      serde_json::from_str(&format!("\"{text}\"")).unwrap();
    assert_text(&read, &text, 255);
    let text = format!("-{}.{}", "9".repeat(153), "0".repeat(255));
    let widest = Scaled::new(Decimal::<U153, U0, i512>::MIN, 255);
    assert_eq!(
      serde_json::to_string(&widest).unwrap(),
      format!("\"{text}\"")
    );
    let bytes = postcard::to_stdvec(&widest).unwrap();
    let read: Scaled<Decimal<U153, U0, i512>> = postcard::from_bytes(&bytes).unwrap();
    assert_text(&read, &text, 255);
  }

  #[test]
  fn a_string_that_does_not_parse_reports_why() {
    assert_de_tokens_error::<WideS>(&[Token::Str("1.5x")], "invalid decimal syntax");
    assert_de_tokens_error::<NarrowS>(
      &[Token::Str("-1")],
      "decimal is negative, and the target type is unsigned",
    );
    assert_de_tokens_error::<WideS>(
      &[Token::Str("0e-999")],
      "decimal has more than 255 fraction digits",
    );
    assert_de_tokens_error::<NarrowS>(
      &[Token::Str("0.000000001")],
      "decimal cannot be represented exactly in the target type",
    );
  }
}
