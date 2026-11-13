use core::{
  any::TypeId,
  cmp::Ordering,
  fmt::{self, Write as _},
  hash::BuildHasher,
  num::NonZero,
  str::FromStr,
};
use std::{
  collections::{HashSet, hash_map::RandomState},
  format,
  string::{String, ToString},
};

use typenum::{
  U0, U1, U2, U4, U8, U10, U11, U12, U16, U18, U20, U22, U36, U40, U153, U154, U307, U308,
};

use super::Scaled;
use crate::{
  ConstDecimalOne, ConstFixedDecimal, ConvertError, ConvertTo, Decimal, DecimalAssign,
  DecimalFloat, DecimalOne, DivRound, DivideError, ExactMul, FixedDecimal, FromFloatError,
  ParseError, ParseErrorKind, QuantizeError, RoundingMode, SignedFixedDecimal,
  UnsignedFixedDecimal, WidenTo, bigint::MODES, dec, i256, i512, i1024, u256, u512, u1024,
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
fn assert_text<D: FixedDecimal>(scaled: &Scaled<D>, text: &str, scale: u16) {
  assert_eq!(scaled.to_string(), text, "text of {scaled:?}");
  assert_eq!(scaled.scale(), scale, "scale of {text}");
}

/// Texts with as many fraction digits as their writer chose, each with its scale.
const EXAMPLES: [(&str, u16); 24] = [
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
  let largest = format!("0.{}", "0".repeat(65535));
  assert_text(&value::<WideS>(&largest), &largest, 65535);
  assert_text(&value::<NarrowS>(&largest), &largest, 65535);
  assert_text(&value::<WideS>("0e-65535"), &largest, 65535);
  let unit = format!("0.{}1{}", "0".repeat(17), "0".repeat(65517));
  assert_text(&value::<WideS>(&unit), &unit, 65535);
  let one = format!("1.{}", "0".repeat(65535));
  assert_text(&value::<NarrowS>(&one), &one, 65535);

  // 65536 fraction digits or more, however the text writes them, once the value fits.
  let too_many = format!("0.{}", "0".repeat(65536));
  let padded_one = format!("1.{}", "0".repeat(65536));
  let exact_ten = format!("1{}e-65536", "0".repeat(65537));
  for text in [
    too_many.as_str(),
    padded_one.as_str(),
    exact_ten.as_str(),
    "0e-65536",
    "0.0e-65535",
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
    (
      &format!("1.{}1", "0".repeat(65536)),
      ParseErrorKind::Inexact,
    ),
    (
      &format!("1{}.{}", "0".repeat(30), "0".repeat(65536)),
      ParseErrorKind::OutOfRange,
    ),
  ] {
    assert_eq!(kind::<WideS>(text), Err(expected), "{text}");
    assert_eq!(kind::<NarrowS>(text), Err(expected), "{text}");
  }
  // The sign of a negative text for an unsigned storage is decided before the scale, and the
  // syntax before everything.
  let negative = format!("-0.{}", "0".repeat(65536));
  assert_eq!(kind::<NarrowS>(&negative), Err(ParseErrorKind::Negative));
  assert_eq!(kind::<WideS>(&negative), Err(ParseErrorKind::ScaleTooLarge));
  let garbled = format!("0.{}x", "0".repeat(65536));
  assert_eq!(kind::<WideS>(&garbled), Err(ParseErrorKind::InvalidSyntax));
  assert_eq!(
    kind::<NarrowS>(&garbled),
    Err(ParseErrorKind::InvalidSyntax)
  );
  assert_eq!(
    kind::<WideS>(&format!("-inf{}", "0".repeat(65536))),
    Err(ParseErrorKind::InvalidSyntax)
  );
}

#[test]
fn decimal_parse_never_reports_scale_too_large() {
  let long = format!("0.{}", "0".repeat(65536));
  assert_eq!(Wide::parse(&long), Ok(Wide::ZERO));
  assert_eq!(Narrow::parse(&long), Ok(Narrow::ZERO));
  assert_eq!(Wide::parse("0e-99999999999999999999"), Ok(Wide::ZERO));
  assert_eq!(
    Wide::parse(&format!("1.{}", "0".repeat(65536))),
    Ok(Wide::ONE)
  );
  assert_eq!(
    Narrow::parse(&format!("1{}e-65536", "0".repeat(65537))).map(Narrow::repr),
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
  let too_large = "0e-65536".parse::<WideS>().unwrap_err();
  assert!(too_large.kind().is_scale_too_large());
  assert_eq!(
    too_large.to_string(),
    "decimal has more than 65535 fraction digits"
  );
}

#[test]
fn parse_at_the_storage_limits() {
  type D154u512 = Scaled<Decimal<U154, U0, u512>>;
  type D0x153i512 = Scaled<Decimal<U0, U153, i512>>;
  let max = format!("{}.{}", "9".repeat(154), "0".repeat(65535));
  let widest: D154u512 = value(&max);
  assert_text(&widest, &max, 65535);
  assert_eq!(*widest.value(), Decimal::<U154, U0, u512>::MAX);
  let min = format!("-0.{}{}", "9".repeat(153), "0".repeat(65382));
  assert_text(&value::<D0x153i512>(&min), &min, 65535);
  type D308u1024 = Scaled<Decimal<U308, U0, u1024>>;
  type D0x307i1024 = Scaled<Decimal<U0, U307, i1024>>;
  let max = format!("{}.{}", "9".repeat(308), "0".repeat(65535));
  let widest: D308u1024 = value(&max);
  assert_text(&widest, &max, 65535);
  assert_eq!(*widest.value(), Decimal::<U308, U0, u1024>::MAX);
  let min = format!("-0.{}{}", "9".repeat(307), "0".repeat(65228));
  assert_text(&value::<D0x307i1024>(&min), &min, 65535);
  let small: Scaled<Decimal<U1, U1, u8>> = value("9.90");
  assert_text(&small, "9.90", 2);
  let fraction_only: Scaled<Decimal<U0, U2>> = value("-0.50");
  assert_text(&fraction_only, "-0.50", 2);
}

/// Checks that each limit of `D` with the scale `scale`, which is above the fraction digits of
/// `D`, writes `scale` fraction digits and parses back to its value and scale.
#[inline(never)]
fn check_scale_above_the_fraction<D>(scale: u16)
where
  D: ConstFixedDecimal,
  Scaled<D>: FixedDecimal,
{
  for decimal in [D::MIN, D::MAX] {
    let text = Scaled::new(decimal, scale).to_string();
    let fraction = text.split_once('.').map_or(0, |(_, digits)| digits.len());
    assert_eq!(fraction, usize::from(scale), "{decimal} with scale {scale}");
    let read: Scaled<D> = value(&text);
    assert_eq!(
      read.into_parts(),
      (decimal, scale),
      "{decimal} with scale {scale}"
    );
  }
}

/// Scales that a byte does not count, up to the largest, write their text and parse back, for a
/// narrow type and for the widest of 512 and of 1024 bits.
#[test]
fn scales_above_255_parse_back() {
  for scale in [256, 300, 65535] {
    check_scale_above_the_fraction::<Decimal<U1, U1, i8>>(scale);
    check_scale_above_the_fraction::<Decimal<U154, U0, u512>>(scale);
    check_scale_above_the_fraction::<Decimal<U0, U153, i512>>(scale);
    check_scale_above_the_fraction::<Decimal<U308, U0, u1024>>(scale);
    check_scale_above_the_fraction::<Decimal<U0, U307, i1024>>(scale);
  }
}

/// 300 significant digits after the point: `1` to `9` in turn.
fn fraction_of_300_digits() -> String {
  (0..300)
    .map(|index| char::from(b'1' + (index % 9) as u8))
    .collect()
}

/// A `Scaled` over the type of 308 fraction digits, with 300 significant fraction digits and with
/// the scale 308, which writes every fraction digit of the type: each writes its text and parses
/// back to the value and the scale.
#[test]
fn scaled_texts_of_308_fraction_digits() {
  type S = Scaled<Decimal<U0, U308, u1024>>;
  let digits = fraction_of_300_digits();
  let text = format!("0.{digits}");
  let short: S = value(&text);
  assert_text(&short, &text, 300);
  assert_eq!(short.value().fraction_digits(), 300);
  let widest = Scaled::new(*short.value(), 308);
  let text = format!("0.{digits}{}", "0".repeat(8));
  assert_text(&widest, &text, 308);
  assert_text(&value::<S>(&text), &text, 308);
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
  let padded = Scaled::new(*positive.value(), u16::MAX);
  let mut sink = Limited { remaining: 70_000 };
  assert_eq!(write!(sink, "{padded}"), Ok(()));
  assert_eq!(sink.remaining, 70_000 - (2 + huge));
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

/// The twin of a `Scaled` is the `Scaled` over the twin of its decimal, which is the type itself
/// over an inline storage, and `to_inline` and `from_inline` keep the scale.
#[test]
fn the_twin_keeps_the_scale() {
  fn twin<T: FixedDecimal>(value: &T) -> T::Inline {
    value.to_inline()
  }
  assert_eq!(
    TypeId::of::<<WideS as FixedDecimal>::Inline>(),
    TypeId::of::<WideS>()
  );
  assert_eq!(
    TypeId::of::<<NarrowS as FixedDecimal>::Inline>(),
    TypeId::of::<NarrowS>()
  );
  let padded: WideS = value("0.00100000");
  let inline = padded.to_inline();
  assert_text(&inline, "0.00100000", 8);
  assert_text(&WideS::from_inline(inline), "0.00100000", 8);
  let fee: NarrowS = value("2.5000");
  assert_text(&twin(&fee), "2.5000", 4);
  assert_text(&<NarrowS as FixedDecimal>::from_inline(fee), "2.5000", 4);
}

/// Through the bounds of generic code, a `Scaled` assigns in place, computes in its twin, and
/// clones, also with `clone_from`, which gives the value and the scale of the source.
#[test]
fn generic_assignment_twin_and_clone_keep_the_scale() {
  fn assigned<T: DecimalAssign>(mut a: T, b: T) -> T {
    a += b;
    a
  }
  fn doubled<T: FixedDecimal>(value: &T) -> Option<T> {
    let twin = value.to_inline();
    twin.checked_add(&twin).map(T::from_inline)
  }
  fn cloned<T: Clone>(value: &T) -> T {
    value.clone()
  }
  fn cloned_into<T: Clone>(target: &mut T, source: &T) {
    target.clone_from(source);
  }
  assert_text(&assigned::<WideS>(value("1.5"), value("0.250")), "1.750", 3);
  assert_text(
    &assigned::<NarrowS>(value("2.50"), value("0.0010")),
    "2.5010",
    4,
  );
  assert_text(&doubled::<WideS>(&value("1.50")).unwrap(), "3.00", 2);
  assert_eq!(doubled(&WideS::from(Wide::MAX)), None);

  let source: WideS = value("1.50");
  assert_text(&cloned(&source), "1.50", 2);
  let mut target: WideS = value("-0.001");
  cloned_into(&mut target, &source);
  assert_text(&target, "1.50", 2);
  let mut narrow: NarrowS = value("0.25000");
  cloned_into(&mut narrow, &value("3"));
  assert_text(&narrow, "3", 0);
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
  const TOO_LARGE: Result<WideS, ParseError> = WideS::parse("0e-65536");
  const SCALE: u16 = PADDED.scale();
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

/// A literal keeps the scale of its text, as `parse` does. `==` compares the value alone, so the
/// text and the scale are asserted.
#[test]
fn a_literal_keeps_its_scale() {
  type Tenths = Scaled<Decimal<U2, U1>>;
  // The literal that the test writes bare.
  const _: WideS = dec!(0.125);
  const PADDED: WideS = dec!(1.50);
  const EXPONENT: WideS = dec!(250e-2);
  const WHOLE: WideS = dec!(1E+2);
  const NEGATIVE: WideS = dec!(-1.50);
  const ZERO: WideS = dec!(0e-65535);
  const FINE: Tenths = dec!(2.5000);
  const NAMED: [WideS; 5] = [
    dec!(1.50 as WideS),
    dec!(250e-2 as WideS),
    dec!(1E+2 as WideS),
    dec!(-1.50 as WideS),
    dec!(0e-65535 as WideS),
  ];
  const NAMED_FINE: Tenths = dec!(2.5000 as Tenths);
  const _: () = assert!(PADDED.scale() == 2 && EXPONENT.scale() == 2 && WHOLE.scale() == 0);
  const _: () = assert!(NEGATIVE.scale() == 2 && ZERO.scale() == 65535 && FINE.scale() == 4);
  const _: () = assert!(NAMED[0].scale() == 2 && NAMED[1].scale() == 2 && NAMED[2].scale() == 0);
  const _: () = assert!(NAMED[3].scale() == 2 && NAMED[4].scale() == 65535);
  const _: () = assert!(NAMED_FINE.scale() == 4);

  let zeros = format!("0.{}", "0".repeat(65535));
  let inferred = [PADDED, EXPONENT, WHOLE, NEGATIVE, ZERO];
  let expected = [
    ("1.50", 2),
    ("2.50", 2),
    ("100", 0),
    ("-1.50", 2),
    (zeros.as_str(), 65535),
  ];
  for ((value, named), (text, scale)) in inferred.iter().zip(&NAMED).zip(expected) {
    assert_text(value, text, scale);
    assert_text(named, text, scale);
  }
  assert_text(&FINE, "2.5000", 4);
  assert_text(&NAMED_FINE, "2.5000", 4);
  // A sum takes the larger scale.
  assert_text(&(PADDED + dec!(0.125)), "1.625", 3);
}

#[test]
fn layout() {
  type Bare = Decimal<U10, U8>;
  // A one-byte decimal and its `u16` scale, padded to the alignment of the `u16`, which is two
  // bytes on every target that runs the tests.
  assert_eq!(size_of::<Scaled<Decimal<U1, U1, u8>>>(), 4);
  // Over a wider storage, the scale takes the padding to the alignment of the decimal.
  assert_eq!(size_of::<Price>(), size_of::<Bare>() + align_of::<Bare>());
  assert_eq!(size_of::<WideS>(), size_of::<Wide>() + align_of::<Wide>());
  // The scale is a plain `u16`, so an `Option` has no niche.
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

  // An exact product takes the sum of the scales, saturating at 65535; the value is never
  // rounded.
  let price: Price = value("64250.5");
  let quantity: Quantity = value("0.0015000");
  let notional: Notional = price.exact_mul(&quantity);
  assert_text(&notional, "96.37575000", 8);
  let long =
    Scaled::new(price.into_value(), 200).exact_mul(&Scaled::new(quantity.into_value(), 100));
  assert_eq!(long.scale(), 300);
  let saturated =
    Scaled::new(price.into_value(), 40_000).exact_mul(&Scaled::new(quantity.into_value(), 30_000));
  assert_eq!(saturated.scale(), 65535);
  assert_eq!(*saturated.value(), *notional.value());
  assert_eq!(saturated.to_string().len(), "96".len() + 1 + 65535);

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

/// Checks, for a `Scaled` over `D`, that `checked_rem`, `saturating_add` and `saturating_sub` give
/// the value of the decimal's method with the larger scale of the two operands, also where the
/// decimal's result is a limit: at zero, the limits, `1.5` and `0.25`, with the larger scale on
/// either side.
#[inline(never)]
fn check_remainder_and_saturation<D>()
where
  D: ConstFixedDecimal,
  Scaled<D>: FixedDecimal,
{
  let decimals: [D; 5] = [D::ZERO, D::MIN, D::MAX, value("1.5"), value("0.25")];
  for a in decimals {
    for b in decimals {
      for (left, right) in [(0u16, 0u16), (1, 5), (5, 1), (255, 3)] {
        let (x, y) = (Scaled::new(a, left), Scaled::new(b, right));
        let scale = left.max(right);
        assert_eq!(
          x.checked_rem(&y).map(Scaled::into_parts),
          a.checked_rem(&b).map(|remainder| (remainder, scale)),
          "{a} % {b}"
        );
        assert_eq!(
          x.saturating_add(&y).into_parts(),
          (a.saturating_add(&b), scale),
          "{a} + {b}"
        );
        assert_eq!(
          x.saturating_sub(&y).into_parts(),
          (a.saturating_sub(&b), scale),
          "{a} - {b}"
        );
      }
    }
  }
}

/// `checked_rem`, `saturating_add` and `saturating_sub` take the larger scale of their operands,
/// also where the result is a limit, whose own scale is 0.
#[test]
fn remainders_and_saturated_results_take_the_larger_scale() {
  let short: WideS = value("7.5");
  let padded: WideS = value("2.000");
  assert_text(&short.checked_rem(&padded).unwrap(), "1.500", 3);
  assert_text(&padded.checked_rem(&short).unwrap(), "2.000", 3);
  assert_eq!(short.checked_rem(&WideS::ZERO), None);
  assert_text(&short.saturating_add(&padded), "9.500", 3);
  assert_text(&padded.saturating_sub(&short), "-5.500", 3);

  let max = "99999999999999999999.999999999999999999";
  assert_text(&WideS::MAX.saturating_add(&padded), max, 3);
  assert_text(&padded.saturating_add(&WideS::MAX), max, 3);
  assert_text(&WideS::MIN.saturating_sub(&short), &format!("-{max}"), 1);
  assert_text(&WideS::MAX.saturating_add(&WideS::MAX), max, 0);
  let small: NarrowS = value("0.00100");
  assert_text(&NarrowS::ZERO.saturating_sub(&small), "0.00000", 5);
  assert_text(&small.saturating_sub(&NarrowS::MAX), "0.00000", 5);

  check_remainder_and_saturation::<Decimal<U2, U2>>();
  check_remainder_and_saturation::<Narrow>();
  check_remainder_and_saturation::<Decimal<U40, U36, i256>>();
  check_remainder_and_saturation::<Decimal<U40, U36, u256>>();
}

/// Checks, for a `Scaled` over `D`, that `trunc`, `fract`, `floor`, `ceil` and `is_integer` give
/// the value of the decimal's method, or its error: `trunc`, `floor` and `ceil` with scale 0, and
/// `fract` with the scale of the value. At zero, the limits, `1.5` and `0.25`, each with a few
/// scales.
#[inline(never)]
fn check_parts<D>()
where
  D: ConstFixedDecimal,
  Scaled<D>: FixedDecimal,
{
  let decimals: [D; 5] = [D::ZERO, D::MIN, D::MAX, value("1.5"), value("0.25")];
  for a in decimals {
    for scale in [0u16, 3, 255] {
      let x = Scaled::new(a, scale);
      assert_eq!(x.trunc().into_parts(), (a.trunc(), 0), "trunc of {a}");
      assert_eq!(x.fract().into_parts(), (a.fract(), scale), "fract of {a}");
      assert_eq!(
        x.floor().map(Scaled::into_parts),
        a.floor().map(|floor| (floor, 0)),
        "floor of {a}"
      );
      assert_eq!(
        x.ceil().map(Scaled::into_parts),
        a.ceil().map(|ceil| (ceil, 0)),
        "ceil of {a}"
      );
      assert_eq!(x.is_integer(), a.is_integer(), "{a}");
    }
  }
}

/// `trunc`, `floor` and `ceil` give scale 0, `fract` keeps the scale, and `is_integer` reads the
/// value, whatever the scale.
#[test]
fn parts_take_their_scales() {
  let x: Price = value("-1.90");
  assert_text(&x.trunc(), "-1", 0);
  assert_text(&x.fract(), "-0.90", 2);
  assert_text(&x.floor().unwrap(), "-2", 0);
  assert_text(&x.ceil().unwrap(), "-1", 0);
  assert!(!x.is_integer());
  assert!(value::<Price>("2.00").is_integer());
  assert_eq!(Price::MAX.ceil(), Err(ConvertError::OutOfRange));
  assert_eq!(Price::MIN.floor(), Err(ConvertError::OutOfRange));

  check_parts::<Decimal<U2, U2>>();
  check_parts::<Narrow>();
  check_parts::<Decimal<U40, U36, i256>>();
  check_parts::<Decimal<U40, U36, u256>>();
}

/// Checks, for a `Scaled` over `D`, that the methods on the digits give the value of the
/// decimal's method whatever the scale, and that `round_significant` gives the value of the
/// decimal's method, or its error, with the scale that the digits of the result need. At zero,
/// the limits, `1.5` and `0.25`, each with a few scales, at a few counts in two modes.
#[inline(never)]
fn check_digits<D>()
where
  D: ConstFixedDecimal,
  Scaled<D>: FixedDecimal,
{
  let decimals: [D; 5] = [D::ZERO, D::MIN, D::MAX, value("1.5"), value("0.25")];
  let counts = [1, 2, D::DIGITS, u32::MAX].map(|count| NonZero::new(count).expect("not zero"));
  for a in decimals {
    for scale in [0u16, 3, 255] {
      let x = Scaled::new(a, scale);
      assert_eq!(x.order_of_magnitude(), a.order_of_magnitude(), "{a}");
      assert_eq!(x.significant_digits(), a.significant_digits(), "{a}");
      assert_eq!(x.fraction_digits(), a.fraction_digits(), "{a}");
      for digits in counts {
        for mode in [RoundingMode::HalfEven, RoundingMode::Up] {
          let expected = a.round_significant(digits, mode).map(|rounded| {
            let scale = if rounded.is_zero() {
              0
            } else {
              i64::from(digits.get()) - 1 - i64::from(rounded.order_of_magnitude())
            };
            (
              rounded,
              u16::try_from(scale.clamp(0, i64::from(u16::MAX))).expect("a scale"),
            )
          });
          assert_eq!(
            x.round_significant(digits, mode).map(Scaled::into_parts),
            expected,
            "{a} to {digits} digits, {mode}"
          );
        }
      }
    }
  }
}

/// The methods on the digits are the value's, whatever the scale, and `round_significant` takes
/// the scale that the digits of its result need, after a carry too, and 0 for zero.
#[test]
fn digits_are_the_values_and_rounding_takes_the_scale_of_its_digits() {
  let x: Price = value("2.50");
  assert_eq!(x.scale(), 2);
  assert_eq!(
    (
      x.significant_digits(),
      x.order_of_magnitude(),
      x.fraction_digits()
    ),
    (2, 0, 1)
  );

  let digits = |count: u32| NonZero::new(count).expect("not zero");
  let round = |text: &str, count: u32| {
    value::<Price>(text)
      .round_significant(digits(count), RoundingMode::HalfEven)
      .unwrap()
  };
  for (count, text, scale) in [
    (1, "300", 0),
    (2, "310", 0),
    (3, "305", 0),
    (4, "305.5", 1),
    (5, "305.46", 2),
    (6, "305.459", 3),
    (7, "305.4590", 4),
  ] {
    assert_text(&round("305.459", count), text, scale);
  }
  assert_text(&round("999.96", 3), "1000", 0);
  assert_text(&round("999.96", 4), "1000", 0);
  assert_text(&round("999.96", 5), "999.96", 2);
  assert_text(&round("0.012301", 3), "0.0123", 4);
  assert_text(&round("1.5000", 1), "2", 0);
  assert_text(&round("1.5000", 3), "1.50", 2);
  assert_text(&round("0.000", 3), "0", 0);
  assert_text(&round("1.5", 300), &format!("1.5{}", "0".repeat(298)), 299);
  // More digits than a `u16` scale counts: the scale saturates.
  let long = format!("1.5{}", "0".repeat(65534));
  assert_text(&round("1.5", 65_537), &long, 65535);
  assert_text(&round("1.5", u32::MAX), &long, 65535);

  check_digits::<Decimal<U2, U2>>();
  check_digits::<Narrow>();
  check_digits::<Decimal<U40, U36, i256>>();
  check_digits::<Decimal<U40, U36, u256>>();
}

/// Checks, for a `Scaled` over `D`, that each reading of a float gives the value of the decimal's
/// own function, or its error, with the scale of its reading: the fraction digits of `D` for the
/// binary value, and for the shortest text the fraction digits of the text that `Display` writes
/// for the float, at most those of `D`. At a few floats of each format, in two modes. And that
/// `to_f64` and `to_f32` are those of the value, whatever the scale: at zero, the limits and `1.5`,
/// each with the scales 0, 3 and 255.
#[inline(never)]
fn check_floats<D>()
where
  D: ConstFixedDecimal + DecimalFloat,
  Scaled<D>: DecimalFloat,
{
  let fraction = u16::try_from(D::FRACTION).expect("a scale");
  // The fraction digits of the text that `Display` writes, at most those of `D`.
  let text_scale = |text: String| {
    let digits = text.split_once('.').map_or(0, |(_, digits)| digits.len());
    u16::try_from(digits.min(D::FRACTION as usize)).expect("a scale")
  };
  for x in [0.5, -2.5, 0.1, 1e-3, 123.456, 1e21, -0.0, 5e-324] {
    let narrow = x as f32;
    for mode in [RoundingMode::HalfEven, RoundingMode::Up] {
      assert_eq!(
        Scaled::<D>::round_from_f64(x, mode).map(Scaled::into_parts),
        D::round_from_f64(x, mode).map(|value| (value, fraction)),
        "{x:e}, {mode}"
      );
      assert_eq!(
        Scaled::<D>::round_from_f32(narrow, mode).map(Scaled::into_parts),
        D::round_from_f32(narrow, mode).map(|value| (value, fraction)),
        "{narrow:e}, {mode}"
      );
      assert_eq!(
        Scaled::<D>::from_f64_shortest(x, mode).map(Scaled::into_parts),
        D::from_f64_shortest(x, mode).map(|value| (value, text_scale(x.to_string()))),
        "{x:e}, {mode}"
      );
      assert_eq!(
        Scaled::<D>::from_f32_shortest(narrow, mode).map(Scaled::into_parts),
        D::from_f32_shortest(narrow, mode).map(|value| (value, text_scale(narrow.to_string()))),
        "{narrow:e}, {mode}"
      );
    }
  }
  let decimals: [D; 4] = [D::ZERO, D::MIN, D::MAX, value("1.5")];
  for decimal in decimals {
    for scale in [0u16, 3, 255] {
      let scaled = Scaled::new(decimal, scale);
      assert_eq!(
        (scaled.to_f64().to_bits(), scaled.to_f32().to_bits()),
        (decimal.to_f64().to_bits(), decimal.to_f32().to_bits()),
        "{decimal}, scale {scale}"
      );
    }
  }
}

/// The readings of a float through `Scaled` take the scale of their reading: the binary value the
/// fraction digits of the type, and the shortest text its own fraction digits, at most those of
/// the type, so that a text rounded to the type keeps them all and `-0.0` reads as zero with scale
/// 0. NaN and the infinities are the errors of the decimal's own readings, which `Scaled` returns
/// before it formats the float. `to_f64` and `to_f32` are those of the value.
#[test]
fn floats_read_with_the_scale_of_their_reading() {
  let (down, even, half_up) = (
    RoundingMode::Down,
    RoundingMode::HalfEven,
    RoundingMode::HalfUp,
  );
  assert_text(&Price::round_from_f64(0.5, even).unwrap(), "0.50000000", 8);
  assert_text(&Price::from_f64_shortest(0.5, even).unwrap(), "0.5", 1);
  let large = Scaled::<Decimal<U22, U16>>::from_f64_shortest(1e21, down).unwrap();
  assert_text(&large, "1000000000000000000000", 0);
  assert_text(&Cents::from_f64_shortest(1.9999, even).unwrap(), "2.00", 2);
  assert_text(
    &Cents::from_f64_shortest(2.675, half_up).unwrap(),
    "2.68",
    2,
  );
  assert_text(&Cents::round_from_f64(2.675, half_up).unwrap(), "2.67", 2);
  assert_text(&Price::from_f32_shortest(0.1, even).unwrap(), "0.1", 1);
  let widened = Price::from_f64_shortest(f64::from(0.1_f32), even).unwrap();
  assert_text(&widened, "0.10000000", 8);
  for zero in [
    Price::from_f64_shortest(-0.0, down),
    Price::from_f32_shortest(-0.0, down),
  ] {
    assert_text(&zero.unwrap(), "0", 0);
  }
  for zero in [
    Price::round_from_f64(-0.0, down),
    Price::round_from_f32(-0.0, down),
  ] {
    assert_text(&zero.unwrap(), "0.00000000", 8);
  }

  for mode in [down, even] {
    for (x, error) in [
      (f64::NAN, FromFloatError::Nan),
      (f64::INFINITY, FromFloatError::Infinite),
      (f64::NEG_INFINITY, FromFloatError::Infinite),
    ] {
      let narrow = x as f32;
      assert_eq!(Price::round_from_f64(x, mode), Err(error), "{x}, {mode}");
      assert_eq!(
        Price::round_from_f32(narrow, mode),
        Err(error),
        "{x}, {mode}"
      );
      assert_eq!(Price::from_f64_shortest(x, mode), Err(error), "{x}, {mode}");
      assert_eq!(
        Price::from_f32_shortest(narrow, mode),
        Err(error),
        "{x}, {mode}"
      );
    }
  }

  check_floats::<Decimal<U2, U2>>();
  check_floats::<Narrow>();
  check_floats::<Decimal<U40, U36, i256>>();
  check_floats::<Decimal<U40, U36, u256>>();
}

/// The operators take the larger scale of their operands, as `checked_add` and `checked_sub` do,
/// and a sum over an iterator the largest scale of its items, or scale 0 without an item.
#[test]
#[allow(clippy::op_ref, reason = "the operators on references are under test")]
fn operators_and_sums_take_the_larger_scale() {
  let short: WideS = value("0.001");
  let padded: WideS = value("0.00100000");
  assert_text(&(short + padded), "0.00200000", 8);
  assert_text(&(padded + short), "0.00200000", 8);
  assert_text(&(&short + &padded), "0.00200000", 8);
  assert_text(&(short - padded), "0.00000000", 8);
  assert_text(&(padded - short), "0.00000000", 8);
  assert_text(&(&short - &padded), "0.00000000", 8);
  assert_text(&(short + short), "0.002", 3);
  let mut total = short;
  total += padded;
  assert_text(&total, "0.00200000", 8);
  total -= short;
  assert_text(&total, "0.00100000", 8);
  let mut total = padded;
  total -= short;
  assert_text(&total, "0.00000000", 8);
  total += short;
  assert_text(&total, "0.00100000", 8);

  let items: [WideS; 3] = [value("1.5"), value("-0.250"), value("2")];
  assert_text(&items.into_iter().sum::<WideS>(), "3.250", 3);
  assert_text(&items.iter().sum::<WideS>(), "3.250", 3);
  assert_text(&[padded].into_iter().sum::<WideS>(), "0.00100000", 8);
  assert_text(&[padded].iter().sum::<WideS>(), "0.00100000", 8);
  assert_text(&core::iter::empty::<WideS>().sum::<WideS>(), "0", 0);
  assert_text(&core::iter::empty::<&WideS>().sum::<WideS>(), "0", 0);

  let fee: NarrowS = value("0.0010");
  let amount: NarrowS = value("2.5");
  assert_text(&(amount + fee), "2.5010", 4);
  assert_text(&(amount - fee), "2.4990", 4);
  assert_text(&(&amount - &fee), "2.4990", 4);
  assert_text(&[amount, fee, amount].iter().sum::<NarrowS>(), "5.0010", 4);
  assert_text(&core::iter::empty::<NarrowS>().sum::<NarrowS>(), "0", 0);
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
      RoundingMode::Up | RoundingMode::Ceiling | RoundingMode::Up05 => "0.000000000000000001",
      _ => "0.000000000000000000",
    };
    assert_text(&rounded, expected, 18);
    let rounded_tie: WideS = tie.round_to(mode).unwrap();
    let expected_tie = match mode {
      RoundingMode::Up | RoundingMode::Ceiling | RoundingMode::HalfUp | RoundingMode::Up05 => {
        "0.000000000000000001"
      }
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
  use core::fmt::{self, Display};

  use serde_core::{
    Deserialize, Serialize,
    de::{self, IntoDeserializer},
    ser,
  };
  use serde_test::{Configure, Token, assert_de_tokens_error, assert_tokens};
  use std::{
    format,
    string::{String, ToString},
    vec::Vec,
  };

  use super::{
    ConstFixedDecimal, Decimal, FixedDecimal, Narrow, NarrowS, Scaled, U0, U1, U153, U154, U307,
    U308, WideS, assert_text, fraction_of_300_digits, i512, i1024, u512, u1024, value,
  };

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
    // 154 integral digits, or a sign and 153, and 255 fraction digits, above those of the type.
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

    // 308 integral digits, or a sign and 307, and the same fraction digits.
    let text = format!("{}.{}", "9".repeat(308), "0".repeat(255));
    let widest = Scaled::new(Decimal::<U308, U0, u1024>::MAX, 255);
    assert_eq!(
      serde_json::to_string(&widest).unwrap(),
      format!("\"{text}\"")
    );
    let read: Scaled<Decimal<U308, U0, u1024>> =
      serde_json::from_str(&format!("\"{text}\"")).unwrap();
    assert_text(&read, &text, 255);
    let text = format!("-{}.{}", "9".repeat(307), "0".repeat(255));
    let widest = Scaled::new(Decimal::<U307, U0, i1024>::MIN, 255);
    assert_eq!(
      serde_json::to_string(&widest).unwrap(),
      format!("\"{text}\"")
    );
    let bytes = postcard::to_stdvec(&widest).unwrap();
    let read: Scaled<Decimal<U307, U0, i1024>> = postcard::from_bytes(&bytes).unwrap();
    assert_text(&read, &text, 255);
  }

  /// Asserts that JSON and postcard write each limit of `D` with the scale `scale` as they write
  /// the text of its `Display`, and read the value and the scale back.
  #[inline(never)]
  fn check_formats<D>(scale: u16)
  where
    D: ConstFixedDecimal,
    Scaled<D>: FixedDecimal,
  {
    for decimal in [D::MIN, D::MAX] {
      let scaled = Scaled::new(decimal, scale);
      let text = scaled.to_string();
      let json = serde_json::to_string(&scaled).unwrap();
      assert_eq!(json, serde_json::to_string(&text).unwrap(), "{text}");
      let read: Scaled<D> = serde_json::from_str(&json).unwrap();
      assert_eq!(read.into_parts(), (decimal, scale), "{text}");
      let bytes = postcard::to_stdvec(&scaled).unwrap();
      assert_eq!(bytes, postcard::to_stdvec(&text).unwrap(), "{text}");
      let read: Scaled<D> = postcard::from_bytes(&bytes).unwrap();
      assert_eq!(read.into_parts(), (decimal, scale), "{text}");
    }
  }

  /// The method of a serializer through which a value wrote its text.
  #[derive(Clone, Copy, Debug, PartialEq, Eq)]
  enum Path {
    /// `serialize_str`, with the text that the value wrote into a buffer.
    Str,
    /// `collect_str`, with the value itself.
    Collect,
  }

  /// A serializer that takes a string, and returns the method that wrote it and the text. Every
  /// other value is [`NotAString`].
  struct Recorder;

  /// The error of [`Recorder`] for a value that is not a string.
  #[derive(Debug, PartialEq, Eq)]
  struct NotAString;

  impl Display for NotAString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      f.write_str("not a string")
    }
  }

  impl core::error::Error for NotAString {}

  impl ser::Error for NotAString {
    fn custom<T: Display>(_: T) -> Self {
      Self
    }
  }

  /// What [`Recorder`] returns for a compound value, which it refuses.
  type Refused = ser::Impossible<(Path, String), NotAString>;

  /// Writes the methods of [`Recorder`] that refuse their value, each with its parameters and the
  /// type that it would return.
  macro_rules! refuse {
    ($($method:ident($($parameter:ty),*) -> $ok:ty;)+) => {$(
      fn $method(self, $(_: $parameter),*) -> Result<$ok, NotAString> {
        Err(NotAString)
      }
    )+};
  }

  impl ser::Serializer for Recorder {
    type Ok = (Path, String);
    type Error = NotAString;
    type SerializeSeq = Refused;
    type SerializeTuple = Refused;
    type SerializeTupleStruct = Refused;
    type SerializeTupleVariant = Refused;
    type SerializeMap = Refused;
    type SerializeStruct = Refused;
    type SerializeStructVariant = Refused;

    fn serialize_str(self, text: &str) -> Result<Self::Ok, NotAString> {
      Ok((Path::Str, text.to_string()))
    }

    fn collect_str<T: Display + ?Sized>(self, value: &T) -> Result<Self::Ok, NotAString> {
      Ok((Path::Collect, value.to_string()))
    }

    fn serialize_some<T: Serialize + ?Sized>(self, _: &T) -> Result<Self::Ok, NotAString> {
      Err(NotAString)
    }

    fn serialize_newtype_struct<T: Serialize + ?Sized>(
      self,
      _: &'static str,
      _: &T,
    ) -> Result<Self::Ok, NotAString> {
      Err(NotAString)
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(
      self,
      _: &'static str,
      _: u32,
      _: &'static str,
      _: &T,
    ) -> Result<Self::Ok, NotAString> {
      Err(NotAString)
    }

    refuse! {
      serialize_bool(bool) -> Self::Ok;
      serialize_i8(i8) -> Self::Ok;
      serialize_i16(i16) -> Self::Ok;
      serialize_i32(i32) -> Self::Ok;
      serialize_i64(i64) -> Self::Ok;
      serialize_u8(u8) -> Self::Ok;
      serialize_u16(u16) -> Self::Ok;
      serialize_u32(u32) -> Self::Ok;
      serialize_u64(u64) -> Self::Ok;
      serialize_f32(f32) -> Self::Ok;
      serialize_f64(f64) -> Self::Ok;
      serialize_char(char) -> Self::Ok;
      serialize_bytes(&[u8]) -> Self::Ok;
      serialize_none() -> Self::Ok;
      serialize_unit() -> Self::Ok;
      serialize_unit_struct(&'static str) -> Self::Ok;
      serialize_unit_variant(&'static str, u32, &'static str) -> Self::Ok;
      serialize_seq(Option<usize>) -> Refused;
      serialize_tuple(usize) -> Refused;
      serialize_tuple_struct(&'static str, usize) -> Refused;
      serialize_tuple_variant(&'static str, u32, &'static str, usize) -> Refused;
      serialize_map(Option<usize>) -> Refused;
      serialize_struct(&'static str, usize) -> Refused;
      serialize_struct_variant(&'static str, u32, &'static str, usize) -> Refused;
    }
  }

  /// Checks [`check_formats`] at the scale `Fraction`, the last that the buffer of a decimal
  /// writes, and at `Fraction + 1`, the first that `collect_str` writes, and that a [`Recorder`]
  /// sees each limit of `D` written through the method of its scale, with the text of `Display`.
  #[inline(never)]
  fn check_both_paths<D>()
  where
    D: ConstFixedDecimal,
    Scaled<D>: FixedDecimal,
  {
    let fraction = u16::try_from(D::FRACTION).expect("a scale");
    check_formats::<D>(fraction);
    check_formats::<D>(fraction + 1);
    for (scale, path) in [(fraction, Path::Str), (fraction + 1, Path::Collect)] {
      for decimal in [D::MIN, D::MAX] {
        let scaled = Scaled::new(decimal, scale);
        assert_eq!(
          scaled.serialize(Recorder),
          Ok((path, scaled.to_string())),
          "{scaled:?}"
        );
      }
    }
  }

  /// A scale of at most the fraction digits of the type is written through a buffer, with
  /// `serialize_str`, and a larger one through `collect_str`, and JSON and postcard write the same
  /// bytes for both as for the text itself: for a native and for four wide storages. The widest
  /// types of 512 and of 1024 bits write their longest texts through the buffer.
  #[test]
  fn both_paths_write_the_text() {
    check_both_paths::<Narrow>();
    check_both_paths::<Decimal<U0, U153, i512>>();
    check_both_paths::<Decimal<U0, U154, u512>>();
    check_both_paths::<Decimal<U0, U307, i1024>>();
    check_both_paths::<Decimal<U0, U308, u1024>>();
  }

  /// Scales that a byte does not count, up to the largest, through JSON and postcard, for a narrow
  /// type and for the widest of 512 and of 1024 bits.
  #[test]
  fn scales_above_255_through_json_and_postcard() {
    for scale in [256, 300, 65535] {
      check_formats::<Decimal<U1, U1, i8>>(scale);
      check_formats::<Decimal<U154, U0, u512>>(scale);
      check_formats::<Decimal<U308, U0, u1024>>(scale);
    }
  }

  /// A `Scaled` over the type of 308 fraction digits, with 300 significant fraction digits and
  /// with the scale 308, through JSON and postcard: each format writes the text and reads back the
  /// value and the scale.
  #[test]
  fn scaled_texts_of_308_fraction_digits_in_every_format() {
    type S = Scaled<Decimal<U0, U308, u1024>>;
    let short: S = value(&format!("0.{}", fraction_of_300_digits()));
    for scaled in [short, Scaled::new(*short.value(), 308)] {
      let text = scaled.to_string();
      let json = serde_json::to_string(&scaled).unwrap();
      assert_eq!(json, format!("\"{text}\""));
      let read: S = serde_json::from_str(&json).unwrap();
      assert_text(&read, &text, scaled.scale());
      let bytes = postcard::to_stdvec(&scaled).unwrap();
      assert_eq!(bytes, postcard::to_stdvec(&text).unwrap(), "{text}");
      let read: S = postcard::from_bytes(&bytes).unwrap();
      assert_text(&read, &text, scaled.scale());
    }
  }

  #[test]
  fn a_string_that_does_not_parse_reports_why() {
    assert_de_tokens_error::<WideS>(&[Token::Str("1.5x")], "invalid decimal syntax");
    assert_de_tokens_error::<NarrowS>(
      &[Token::Str("-1")],
      "decimal is negative, and the target type is unsigned",
    );
    assert_de_tokens_error::<WideS>(
      &[Token::Str("0e-65536")],
      "decimal has more than 65535 fraction digits",
    );
    assert_de_tokens_error::<NarrowS>(
      &[Token::Str("0.000000001")],
      "decimal cannot be represented exactly in the target type",
    );
  }
}
