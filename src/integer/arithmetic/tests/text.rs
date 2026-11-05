//! The text of `Uint` and `Int`: `from_str_radix` and `FromStr` against the model of the native
//! parser and by round trips, the radix formats against the bit model and the exponent formats
//! against the native integers, with their flags, at every width up to 16 bits and at wider widths;
//! the errors and the panic of the radix; constant contexts; zero bits and one signed bit; code
//! generic over the width; and the text that `FixedInteger` requires.

use core::{
  fmt::{Binary, LowerExp, LowerHex, Octal, UpperExp, UpperHex},
  num::{IntErrorKind, ParseIntError},
  str::FromStr,
};

use num_bigint::{BigUint, Sign};
use typenum::{U3, U4, U5, U6, U10, U11, U12, U13, U14, U15};

use super::*;
use crate::{i0, i1, int::parse_errors, u0};

/// The result of a parser with the kind of its error, as the model of the native parser gives it.
fn kind<T>(result: Result<T, ParseIntError>) -> Result<T, IntErrorKind> {
  result.map_err(|error| *error.kind())
}

/// The texts of `x` in the radix formats: plain, in binary, octal and both cases of hexadecimal,
/// which the parsers read back, and in an unoptimized or optimized build also with `#`, `+`, a
/// width, an alignment and `0`.
fn radix_texts<T: Binary + Octal + LowerHex + UpperHex>(x: &T) -> Vec<String> {
  let mut texts = vec![
    format!("{x:b}"),
    format!("{x:o}"),
    format!("{x:x}"),
    format!("{x:X}"),
  ];
  if !tier().is_miri() {
    texts.extend([
      format!("{x:+#012b}"),
      format!("{x:<#9o}"),
      format!("{x:^+#8x}"),
      format!("{x:#010X}"),
    ]);
  }
  texts
}

/// The texts of `x` in the exponent formats: plain, and in an unoptimized or optimized build also
/// with a precision, `+`, a width, an alignment and `0`.
fn exp_texts<T: LowerExp + UpperExp>(x: &T) -> Vec<String> {
  let mut texts = vec![format!("{x:e}"), format!("{x:E}")];
  if !tier().is_miri() {
    texts.extend([format!("{x:+011.2e}"), format!("{x:>12.0E}")]);
  }
  texts
}

/// The decimal texts at which the parsers of `bits` bits, at most 16, are checked against the model
/// of the native parser: the empty text, lone signs, `-0`, bytes that are no digit, before and
/// after a digit, a second sign, a prefix, and texts past the limits, with a trailing byte that is
/// no digit after one of them.
fn error_texts(bits: u32, signed: bool) -> Vec<String> {
  let (min, max) = range(bits, signed);
  let mut texts: Vec<String> = [
    "", "+", "-", "-0", "x", "1_", "_1", " 1", "1 ", "+-1", "--1", "0x1",
  ]
  .map(String::from)
  .to_vec();
  texts.extend([
    (max + 1).to_string(),
    format!("{}x", max + 1),
    (min - 1).to_string(),
    format!("{max}0"),
  ]);
  texts
}

/// The values of up to 16 bits at which the text of a storage is checked: those of
/// [`small_values`] up to 8 bits and of [`large_values`] above, or under Miri the [`ends`] in a
/// storage that every tier checks, `primary`, and none in another.
fn text_values(bits: u32, signed: bool, primary: bool) -> Vec<i128> {
  match tier() {
    Tier::Miri if primary => ends(bits, signed),
    Tier::Miri => Vec::new(),
    _ if bits <= 8 => small_values(bits, signed, primary),
    _ => large_values(bits, signed),
  }
}

/// The result of `from_str_radix` of `$ty` for `$text` in `$radix` through the inherent `const fn`
/// of the storage and through `FixedInteger`, which must agree.
macro_rules! parse_both {
  ($ty:ty, $text:expr, $radix:expr) => {{
    let (text, radix): (&str, u32) = ($text, $radix);
    let inherent = <$ty>::from_str_radix(text, radix);
    let generic = <$ty as FixedInteger>::from_str_radix(text, radix);
    assert_eq!(
      inherent, generic,
      "{text:?} in radix {radix} through the trait"
    );
    inherent
  }};
}

/// Checks that the texts of `$x`, the value `$a` of `$ty`, parse back through `from_str_radix` and
/// `FromStr`: for `Uint` `$texts`, the plain radix texts of [`radix_texts`], and its decimal text,
/// and for `Int` its decimal text and the texts of its magnitude after `-` for a negative value.
macro_rules! parses_back {
  (unsigned; $ty:ty, $x:expr, $a:expr, $texts:expr) => {{
    let (x, texts): ($ty, &[String]) = ($x, $texts);
    for (radix, text) in [2, 8, 16, 16].into_iter().zip(texts) {
      assert_eq!(
        parse_both!($ty, text, radix),
        Ok(x),
        "{text} in radix {radix}"
      );
    }
    let decimal = x.to_string();
    assert_eq!(parse_both!($ty, &decimal, 10), Ok(x), "{decimal}");
    assert_eq!(decimal.parse::<$ty>(), Ok(x), "{decimal}");
  }};
  (signed; $ty:ty, $x:expr, $a:expr, $texts:expr) => {{
    let (x, a): ($ty, i128) = ($x, $a);
    let (sign, magnitude) = (if a < 0 { "-" } else { "" }, x.unsigned_abs());
    for (radix, text) in [
      (2, format!("{sign}{magnitude:b}")),
      (8, format!("{sign}{magnitude:o}")),
      (16, format!("{sign}{magnitude:X}")),
    ] {
      assert_eq!(
        parse_both!($ty, &text, radix),
        Ok(x),
        "{text} in radix {radix}"
      );
    }
    let decimal = x.to_string();
    assert_eq!(parse_both!($ty, &decimal, 10), Ok(x), "{decimal}");
    assert_eq!(decimal.parse::<$ty>(), Ok(x), "{decimal}");
  }};
}

/// Checks the text of `$ty`, of the sign `$sign` and of `$bits` bits, whose values `$from` creates
/// from an `i128`, at each value of `$values`: the radix formats give the texts of the native
/// `u128` of the pattern of the value, the exponent formats those of the native `i128` of the
/// value, and the texts parse back; and at the texts of [`error_texts`], `from_str_radix`, through
/// the inherent `const fn` and `FixedInteger`, and `FromStr` give what the model of the native
/// parser gives.
macro_rules! sweep_text {
  ($sign:ident; $ty:ty, $bits:expr, $values:expr, $from:expr, $value:expr, $magnitude:expr) => {{
    let (bits, signed) = ($bits, is_signed!($sign));
    let from: fn(i128) -> $ty = $from;
    for a in $values {
      let x = from(a);
      let texts = radix_texts(&x);
      let pattern = model::to_u128(&pattern_of(a, bits));
      assert_eq!(texts, radix_texts(&pattern), "{a} in {bits} bits");
      assert_eq!(exp_texts(&x), exp_texts(&a), "{a} in {bits} bits");
      parses_back!($sign; $ty, x, a, &texts);
    }
    for text in error_texts(bits, signed) {
      let expected = bigint::parse(text.as_bytes(), 10, bits, signed).map(from);
      assert_eq!(
        kind(parse_both!($ty, &text, 10)),
        expected,
        "{text:?} at {bits} bits"
      );
      assert_eq!(kind(text.parse::<$ty>()), expected, "{text:?} at {bits} bits");
    }
  }};
}

/// Checks the text at the width `B`, `bits` bits, at most 8, in `u8` and `i8`, `[u8; 1]`, `[u8; 2]`
/// and `[u16; 1]` at the values of [`text_values`] for a storage that every tier checks, and in
/// `u16` and `i16`, `u128` and `i128`, `[u64; 1]` and `[u128; 1]` at those for another. Under Miri,
/// in `u8`, `i8` and `[u8; 2]`.
fn text_of_small_widths<B>(bits: u32)
where
  B: FitsBits<u8> + FitsBits<i8> + FitsBits<u16> + FitsBits<i16> + FitsBits<u128> + FitsBits<i128>,
  B: FitsBits<[u8; 1]> + FitsBits<[u8; 2]> + FitsBits<[u16; 1]>,
  B: FitsBits<[u64; 1]> + FitsBits<[u128; 1]>,
{
  let primary = |bits, signed| text_values(bits, signed, true);
  let secondary = |bits, signed| text_values(bits, signed, false);
  sweep_storage!(sweep_text, primary, native B, bits, u8, i8);
  sweep_storage!(sweep_text, primary, array B, bits, [u8; 2]);
  if !tier().is_miri() {
    sweep_storage!(sweep_text, primary, array B, bits, [u8; 1]);
    sweep_storage!(sweep_text, primary, array B, bits, [u16; 1]);
    sweep_storage!(sweep_text, secondary, native B, bits, u16, i16);
    sweep_storage!(sweep_text, secondary, native B, bits, u128, i128);
    sweep_storage!(sweep_text, secondary, array B, bits, [u64; 1]);
    sweep_storage!(sweep_text, secondary, array B, bits, [u128; 1]);
  }
}

/// Checks the text at the width `B`, `bits` bits, from 9 to 16, in `[u8; 2]`, `u16` and `i16` and
/// `[u16; 1]`, at the values of [`text_values`]. Under Miri, in `[u8; 2]`.
fn text_of_large_widths<B>(bits: u32)
where
  B: FitsBits<u16> + FitsBits<i16> + FitsBits<[u8; 2]> + FitsBits<[u16; 1]>,
{
  let values = |bits, signed| text_values(bits, signed, true);
  sweep_storage!(sweep_text, values, array B, bits, [u8; 2]);
  if !tier().is_miri() {
    sweep_storage!(sweep_text, values, native B, bits, u16, i16);
    sweep_storage!(sweep_text, values, array B, bits, [u16; 1]);
  }
}

/// Every value of each width up to 8 bits, or the values that the tier takes, in native integers
/// and in arrays of one and two limbs: the radix formats against the bit model, through the native
/// `u128` of the pattern, the exponent formats against the native `i128`, the texts parse back,
/// and the errors are those of the model of the native parser. Miri checks 0, 1, 7 and 8 bits at
/// the ends, in `u8`, `i8` and `[u8; 2]`.
#[test]
fn text_of_up_to_8_bits() {
  text_of_small_widths::<U0>(0);
  text_of_small_widths::<U1>(1);
  text_of_small_widths::<U7>(7);
  text_of_small_widths::<U8>(8);
  if !tier().is_miri() {
    text_of_small_widths::<U2>(2);
    text_of_small_widths::<U3>(3);
    text_of_small_widths::<U4>(4);
    text_of_small_widths::<U5>(5);
    text_of_small_widths::<U6>(6);
  }
}

/// The values that the tier takes of widths from 9 to 16 bits, with the ends, in two limbs of 8
/// bits, in `u16` and `i16` and in one limb of 16 bits: every width in an optimized build, 9, 12,
/// 15 and 16 bits in an unoptimized one, and 16 bits in two limbs of 8 bits under Miri.
#[test]
fn text_of_9_to_16_bits() {
  text_of_large_widths::<U16>(16);
  if !tier().is_miri() {
    text_of_large_widths::<U9>(9);
    text_of_large_widths::<U12>(12);
    text_of_large_widths::<U15>(15);
  }
  if tier().is_full() {
    text_of_large_widths::<U10>(10);
    text_of_large_widths::<U11>(11);
    text_of_large_widths::<U13>(13);
    text_of_large_widths::<U14>(14);
  }
}

/// The value of `p` if it is below `2^128`.
fn small_unsigned(p: &[bool]) -> Option<u128> {
  match model::highest_one(p) {
    Some(index) if index >= 128 => None,
    _ => Some(model::to_u128(&p[..p.len().min(128)])),
  }
}

/// The value of `p` in two's complement if it is within the range of `i128`: if every bit from bit
/// 127 up copies its sign.
fn small_signed(p: &[bool]) -> Option<i128> {
  let sign = model::sign(p);
  p.iter()
    .skip(127)
    .all(|&bit| bit == sign)
    .then(|| model::to_i128(&p[..p.len().min(128)]))
}

/// The patterns of `bits` bits, above 16, at which the text of the wide widths is checked: those of
/// [`wide_patterns`] in `K` words of 64 bits, or under Miri four: zero, every bit, the top bit and
/// one.
fn text_patterns<const K: usize>(bits: u32) -> Vec<Vec<bool>> {
  if tier().is_miri() {
    vec![
      model::pattern(bits, |_| false),
      model::pattern(bits, |_| true),
      model::pattern(bits, |i| i + 1 == bits),
      model::pattern(bits, |i| i == 0),
    ]
  } else {
    wide_patterns::<K>(bits)
  }
}

/// Checks the text of `$uint` and `$int`, of `$bits` bits, at each pattern of `$patterns`: the
/// radix formats of both against `model::radix_text`, whose texts parse back as `$uint`, the texts
/// of the magnitude of `$int` after `-` for a negative value parse back as `$int`, `Display`
/// parses back as both, and the exponent formats of the values within the range of the native
/// `u128` and `i128` give their texts.
macro_rules! check_wide_text {
  ($uint:ty, $int:ty, $bits:expr, $patterns:expr) => {{
    for p in $patterns {
      let (x, y) = (<$uint>::from_pattern(p), <$int>::from_pattern(p));
      let texts = [
        model::radix_text(p, 1, false),
        model::radix_text(p, 3, false),
        model::radix_text(p, 4, false),
        model::radix_text(p, 4, true),
      ];
      let text = model::text(p);
      assert_eq!(
        [
          format!("{x:b}"),
          format!("{x:o}"),
          format!("{x:x}"),
          format!("{x:X}")
        ],
        texts,
        "{text}"
      );
      assert_eq!(
        [
          format!("{y:b}"),
          format!("{y:o}"),
          format!("{y:x}"),
          format!("{y:X}")
        ],
        texts,
        "{text}"
      );
      for (radix, digits) in [2, 8, 16, 16].into_iter().zip(&texts) {
        assert_eq!(
          <$uint>::from_str_radix(digits, radix),
          Ok(x),
          "{digits} in radix {radix}"
        );
      }
      let sign = if model::sign(p) { "-" } else { "" };
      let magnitude = y.unsigned_abs();
      for (radix, digits) in [
        (2, format!("{sign}{magnitude:b}")),
        (16, format!("{sign}{magnitude:x}")),
      ] {
        assert_eq!(
          <$int>::from_str_radix(&digits, radix),
          Ok(y),
          "{digits} in radix {radix}"
        );
      }
      assert_eq!(x.to_string().parse::<$uint>(), Ok(x), "{text}");
      assert_eq!(y.to_string().parse::<$int>(), Ok(y), "{text}");
      if let Some(value) = small_unsigned(p) {
        assert_eq!(exp_texts(&x), exp_texts(&value), "{text}");
      }
      if let Some(value) = small_signed(p) {
        assert_eq!(exp_texts(&y), exp_texts(&value), "{text}");
      }
    }
  }};
}

/// Checks the text of `Uint` and `Int` of `$bits` bits, the width `$width`, in each storage of
/// `$storages`, written `unsigned, signed;`, at the patterns of [`text_patterns`] in `$words`
/// words of 64 bits.
macro_rules! text_agrees {
  ($width:ty, $bits:literal, $words:literal; $($uint:ty, $int:ty);+ $(;)?) => {{
    let patterns = text_patterns::<$words>($bits);
    $(check_wide_text!(Uint<$width, $uint>, Int<$width, $int>, $bits, &patterns);)+
  }};
}

/// The bit patterns of 65 to 4096 bits in arrays of each limb type that hold them: the radix
/// formats agree with the bit model, the texts parse back, and the exponent formats of the values
/// that a native integer holds are those of the native integer. 65, 129 and 200 bits in an
/// unoptimized build, and also 256, 1024 and 4096 bits in an optimized one; Miri checks 65 and
/// 129 bits, at four patterns, in limbs of 8 and 64 bits.
#[test]
fn text_of_wide_widths() {
  text_agrees!(U65, 65, 2; [u8; 9], [u8; 9]; [u64; 2], [u64; 2]);
  text_agrees!(U129, 129, 3; [u8; 17], [u8; 17]; [u64; 3], [u64; 3]);
  if !tier().is_miri() {
    text_agrees!(U65, 65, 2; [u32; 3], [u32; 3]; [u128; 1], [u128; 1]);
    text_agrees!(U129, 129, 3; [u32; 5], [u32; 5]; [u128; 2], [u128; 2]);
    text_agrees!(U200, 200, 4; [u8; 25], [u8; 25]; [u32; 7], [u32; 7]; [u64; 4], [u64; 4]);
    text_agrees!(U200, 200, 4; [u128; 2], [u128; 2]);
  }
  if tier().is_full() {
    text_agrees!(U256, 256, 4; [u8; 32], [u8; 32]; [u32; 8], [u32; 8]; [u64; 4], [u64; 4]);
    text_agrees!(U256, 256, 4; [u128; 2], [u128; 2]);
    text_agrees!(U1024, 1024, 16; [u8; 128], [u8; 128]; [u32; 32], [u32; 32]);
    text_agrees!(U1024, 1024, 16; [u64; 16], [u64; 16]; [u128; 8], [u128; 8]);
    text_agrees!(U4096, 4096, 64; [u8; 512], [u8; 512]; [u32; 128], [u32; 128]);
    text_agrees!(U4096, 4096, 64; [u64; 64], [u64; 64]; [u128; 32], [u128; 32]);
  }
}

/// The texts of `x` in the exponent formats at precisions from 0 to 40, which round the digits
/// after the first at each place, or keep them all and add zeros.
fn rounded_texts<T: LowerExp + UpperExp>(x: &T) -> Vec<String> {
  vec![
    format!("{x:.0e}"),
    format!("{x:.1e}"),
    format!("{x:.2E}"),
    format!("{x:.5e}"),
    format!("{x:.18e}"),
    format!("{x:.37E}"),
    format!("{x:.40e}"),
  ]
}

/// Runs of nines and ties, whose rounding carries into the exponent or goes to the even digit, as
/// texts of up to 38 digits, which `u128` holds, and the decimal texts of `Uint` and `Int` of 200
/// bits in arrays read them: their exponent formats at every precision are those of the native
/// integers, for both signs. Under Miri, six of the texts: `99`, 20 nines and `9995`, which carry,
/// `25` and `1250`, ties that go to the even digit, and `15000000000000000001`, whose last digit
/// breaks the tie.
#[test]
fn rounding_of_nines_and_ties_is_that_of_the_native_integers() {
  type U200x8 = Uint<U200, [u8; 25]>;
  type I200x64 = Int<U200, [u64; 4]>;
  type I200x128 = Int<U200, [u128; 2]>;
  let mut texts: Vec<String> = [1, 2, 3, 19, 20, 38]
    .iter()
    .map(|&count| "9".repeat(count))
    .collect();
  texts.extend(
    [
      "5",
      "15",
      "25",
      "35",
      "95",
      "125",
      "1250",
      "1251",
      "1350",
      "1249",
      "9995",
      "9985",
      "2500",
      "25000000000000000000",
      "15000000000000000001",
      "99999999999999999995",
      "12345678901234567890123456789012345678",
    ]
    .map(String::from),
  );
  if tier().is_miri() {
    let kept = [
      "99",
      "99999999999999999999",
      "9995",
      "25",
      "1250",
      "15000000000000000001",
    ];
    texts.retain(|text| kept.contains(&text.as_str()));
  }
  for text in texts {
    let native: u128 = text.parse().expect("38 digits at most");
    let negative = -i128::try_from(native).expect("below 2^127");
    let x: U200x8 = text.parse().expect("in range");
    assert_eq!(rounded_texts(&x), rounded_texts(&native), "{text}");
    let y: I200x64 = format!("-{text}").parse().expect("in range");
    assert_eq!(rounded_texts(&y), rounded_texts(&negative), "-{text}");
    let z: I200x128 = text.parse().expect("in range");
    assert_eq!(rounded_texts(&z), rounded_texts(&native), "{text}");
  }
}

/// The scientific notation of `value` with `marker`, written as the exponent formats of the native
/// integers write it, from its decimal digits and with the division of `num-bigint` for rounding.
fn exp_of(value: &BigInt, precision: Option<usize>, marker: char) -> String {
  let sign = if value.sign() == Sign::Minus { "-" } else { "" };
  let digits = value.magnitude().to_string();
  let (first, rest) = digits.split_at(1);
  let mut exponent = rest.len();
  let mantissa = match precision {
    None => match rest.trim_end_matches('0') {
      "" => first.to_string(),
      fraction => format!("{first}.{fraction}"),
    },
    Some(0) if rest.is_empty() => first.to_string(),
    Some(precision) if precision >= rest.len() => {
      format!("{first}.{rest}{}", "0".repeat(precision - rest.len()))
    }
    Some(precision) => {
      let scale = BigUint::from(10u8).pow((rest.len() - precision) as u32);
      let (quotient, remainder) = (value.magnitude() / &scale, value.magnitude() % &scale);
      let twice = remainder * 2u8;
      let odd = quotient.bit(0);
      let rounded = if twice > scale || (twice == scale && odd) {
        quotient + 1u8
      } else {
        quotient
      };
      let mut kept = rounded.to_string();
      // A carry to `10^(precision + 1)` moves the point one place.
      if kept.len() > precision + 1 {
        kept.pop();
        exponent += 1;
      }
      let (lead, fraction) = kept.split_at(1);
      if fraction.is_empty() {
        lead.to_string()
      } else {
        format!("{lead}.{fraction}")
      }
    }
  };
  format!("{sign}{mantissa}{marker}{exponent}")
}

/// The value of `p`, read unsigned or in two's complement, as a big integer.
fn big_of(p: &[bool], signed: bool) -> BigInt {
  let unsigned = p
    .iter()
    .rev()
    .fold(BigInt::ZERO, |value, &bit| (value << 1u8) + u8::from(bit));
  if signed && model::sign(p) {
    unsigned - (BigInt::from(1u8) << p.len())
  } else {
    unsigned
  }
}

/// Checks `$ty`, of `$bits` bits, at each pattern of `$patterns`, read in two's complement if
/// `$signed`: its texts in radix 7 and 36 by `num-bigint` parse back, and its exponent formats are
/// those of [`exp_of`].
macro_rules! check_big_text {
  ($ty:ty, $bits:expr, $signed:expr, $patterns:expr) => {{
    for p in $patterns {
      let x = <$ty>::from_pattern(p);
      let value = big_of(p, $signed);
      for radix in [7, 36] {
        let text = value.to_str_radix(radix);
        assert_eq!(
          <$ty>::from_str_radix(&text, radix),
          Ok(x),
          "{text} in radix {radix}"
        );
      }
      assert_eq!(format!("{x:e}"), exp_of(&value, None, 'e'), "{value}");
      assert_eq!(format!("{x:.0E}"), exp_of(&value, Some(0), 'E'), "{value}");
      assert_eq!(format!("{x:.3e}"), exp_of(&value, Some(3), 'e'), "{value}");
      assert_eq!(
        format!("{x:.70E}"),
        exp_of(&value, Some(70), 'E'),
        "{value}"
      );
    }
  }};
}

/// The wide widths against `num-bigint`: the texts in radix 7 and 36 parse back, and the exponent
/// formats round as the native integers do, beyond the range of `u128`, at 129 and 200 bits, and
/// in an optimized build also at 1024 bits.
#[test]
#[cfg_attr(miri, ignore = "num-bigint, the oracle, is slow under Miri")]
fn wide_text_matches_big_integers() {
  macro_rules! big {
    ($width:ty, $bits:literal, $words:literal; $($uint:ty, $int:ty);+ $(;)?) => {{
      let patterns = text_patterns::<$words>($bits);
      $(
        check_big_text!(Uint<$width, $uint>, $bits, false, &patterns);
        check_big_text!(Int<$width, $int>, $bits, true, &patterns);
      )+
    }};
  }
  big!(U129, 129, 3; [u8; 17], [u8; 17]; [u64; 3], [u64; 3]; [u128; 2], [u128; 2]);
  big!(U200, 200, 4; [u32; 7], [u32; 7]; [u64; 4], [u64; 4]);
  if tier().is_full() {
    big!(U1024, 1024, 16; [u8; 128], [u8; 128]; [u64; 16], [u64; 16]);
  }
}

/// The texts of `x` in the radix formats at each of 27 specs, with `#`, widths, fills, `é`
/// included, each alignment, `+`, `0` and precisions, which these formats ignore, or under Miri at
/// the first 7, each in binary, octal and both cases of hexadecimal.
fn radix_spec_texts<T: Binary + Octal + LowerHex + UpperHex>(x: &T) -> Vec<String> {
  macro_rules! specs {
    ($($spec:literal),+ $(,)?) => {
      [$(
        format!(concat!("{:", $spec, "b}"), x),
        format!(concat!("{:", $spec, "o}"), x),
        format!(concat!("{:", $spec, "x}"), x),
        format!(concat!("{:", $spec, "X}"), x),
      )+]
    };
  }
  let mut texts = Vec::from(specs!("", "#", "+", "+#", "10", "<10", "^10"));
  if !tier().is_miri() {
    texts.extend(specs!(
      ">10",
      "#10",
      "#<12",
      "#^12",
      "*>12",
      "*<#12",
      "\u{e9}^11",
      "+^11",
      "010",
      "#010",
      "+#012",
      "<010",
      "0>10",
      "0<#10",
      ".3",
      "#.5",
      "1",
      "+#1",
      "130",
      "#0130",
    ));
  }
  texts
}

/// The texts of `x` in the exponent formats at each of 19 specs, with precisions that round, keep
/// and add digits, widths, fills, `é` included, each alignment, `+`, `0` and `#`, which has no
/// effect, or under Miri at the first 5, each with `e` and `E`.
fn exp_spec_texts<T: LowerExp + UpperExp>(x: &T) -> Vec<String> {
  macro_rules! specs {
    ($($spec:literal),+ $(,)?) => {
      [$(
        format!(concat!("{:", $spec, "e}"), x),
        format!(concat!("{:", $spec, "E}"), x),
      )+]
    };
  }
  let mut texts = Vec::from(specs!("", "+", ".0", ".1", ".3"));
  if !tier().is_miri() {
    texts.extend(specs!(
      ".40",
      "12",
      "<12",
      "^12",
      "*>12",
      "\u{e9}^13",
      "012",
      "+012",
      "012.3",
      "+012.1",
      "0>12",
      "^14.2",
      "#",
      "1.2",
    ));
  }
  texts
}

/// The values at which the flags are checked, or under Miri the first two.
fn flag_values<T: Copy>(values: &[T]) -> &[T] {
  if tier().is_miri() {
    &values[..2]
  } else {
    values
  }
}

/// Checks the radix and exponent formats of `$ty` at each spec of [`radix_spec_texts`] and
/// [`exp_spec_texts`], at each value of `$values` of the native integer `$native` of its width,
/// which `$from` stores, against the native integer.
macro_rules! flags_match {
  ($ty:ty, $native:ty, $from:expr, $values:expr) => {{
    let from: fn($native) -> $ty = $from;
    for &value in flag_values(&$values) {
      let x = from(value);
      assert_eq!(radix_spec_texts(&x), radix_spec_texts(&value), "{value}");
      assert_eq!(exp_spec_texts(&x), exp_spec_texts(&value), "{value}");
    }
  }};
}

/// The radix and exponent formats honor every flag as the native integers do, at the 27 radix and
/// 19 exponent specs, at 8 and 128 bits, for both signs, in a native and an array storage: at the
/// limits, zero, one and a value between, or under Miri at 7 radix and 5 exponent specs and two of
/// the values.
#[test]
fn flags_match_the_native_integers() {
  type U8x8 = Uint<U8, u8>;
  type U8x1 = Uint<U8, [u8; 1]>;
  type I8x8 = Int<U8, i8>;
  type I8x16 = Int<U8, [u16; 1]>;
  let unsigned_8: [u8; 4] = [u8::MAX, 0x5a, 0, 1];
  let signed_8: [i8; 5] = [i8::MIN, -0x5a, 0, 1, i8::MAX];
  flags_match!(U8x8, u8, U8x8::new, unsigned_8);
  flags_match!(U8x1, u8, |v| U8x1::from_limbs([v]), unsigned_8);
  flags_match!(I8x8, i8, I8x8::new, signed_8);
  flags_match!(
    I8x16,
    i8,
    |v| I8x16::checked_from_limbs(limbs_of(v.into())).expect("8 bits"),
    signed_8
  );

  type U128x128 = Uint<U128, u128>;
  type U128x64 = Uint<U128, [u64; 2]>;
  type I128x128 = Int<U128, i128>;
  type I128x32 = Int<U128, [u32; 4]>;
  let middle = 0x0123_4567_89ab_cdef_fedc_ba98_7654_3210;
  let unsigned_128: [u128; 4] = [u128::MAX, middle, 0, 1];
  let signed_128: [i128; 5] = [i128::MIN, -(middle as i128), 0, 1, i128::MAX];
  flags_match!(U128x128, u128, U128x128::new, unsigned_128);
  flags_match!(
    U128x64,
    u128,
    |v| U128x64::from_limbs([v as u64, (v >> 64) as u64]),
    unsigned_128
  );
  flags_match!(I128x128, i128, I128x128::new, signed_128);
  flags_match!(
    I128x32,
    i128,
    |v| I128x32::from_limbs(limbs_of(v)),
    signed_128
  );
}

/// The errors of the parsers are those of the native integers: each constant has its kind and its
/// message, and equals the error that the native parser gives for the same text.
#[test]
fn errors_are_those_of_the_native_integers() {
  for (error, native, expected, message) in [
    (
      parse_errors::EMPTY,
      "".parse::<u8>().unwrap_err(),
      IntErrorKind::Empty,
      "cannot parse integer from empty string",
    ),
    (
      parse_errors::INVALID_DIGIT,
      "-".parse::<u8>().unwrap_err(),
      IntErrorKind::InvalidDigit,
      "invalid digit found in string",
    ),
    (
      parse_errors::POS_OVERFLOW,
      "256".parse::<u8>().unwrap_err(),
      IntErrorKind::PosOverflow,
      "number too large to fit in target type",
    ),
    (
      parse_errors::NEG_OVERFLOW,
      "-129".parse::<i8>().unwrap_err(),
      IntErrorKind::NegOverflow,
      "number too small to fit in target type",
    ),
  ] {
    assert_eq!(*error.kind(), expected);
    assert_eq!(error.to_string(), message);
    assert_eq!(error, native);
  }
  assert_eq!(parse_errors::overflow(false), parse_errors::POS_OVERFLOW);
  assert_eq!(parse_errors::overflow(true), parse_errors::NEG_OVERFLOW);
  // The parsers give these errors, as the native ones do for the same texts.
  assert_eq!(u7::from_str_radix("", 10), Err(parse_errors::EMPTY));
  assert_eq!(
    i256::from_str_radix("1_0", 10),
    Err(parse_errors::INVALID_DIGIT)
  );
  assert_eq!("128".parse::<i7>(), Err(parse_errors::POS_OVERFLOW));
  assert_eq!("-65".parse::<i7>(), Err(parse_errors::NEG_OVERFLOW));
}

/// A radix outside 2 to 36 panics in every build, with the message of the native parser in the
/// name of the method, and reports the caller, through the inherent `const fn` and through
/// `FixedInteger`, also with an empty text or a lone sign, since the radix is checked first, as
/// the native parser checks it.
#[test]
fn a_radix_outside_2_to_36_panics() {
  let message = "from_str_radix: radix must lie in the range `[2, 36]`".to_string();
  type Bytes = Int<U24, [u8; 3]>;
  type Wide = Uint<U200>;
  for radix in [0, 1, 37, u32::MAX] {
    panics_here!(u7::from_str_radix("1", radix), message);
    panics_here!(i7::from_str_radix("", radix), message);
    panics_here!(Wide::from_str_radix("1", radix), message);
    panics_here!(Bytes::from_str_radix("-1", radix), message);
    panics_here!(u0::from_str_radix("0", radix), message);
    panics_here!(<u7 as FixedInteger>::from_str_radix("1", radix), message);
    panics_here!(<i7 as FixedInteger>::from_str_radix("", radix), message);
    panics_here!(<Wide as FixedInteger>::from_str_radix("1", radix), message);
    panics_here!(<Bytes as FixedInteger>::from_str_radix("-", radix), message);
  }
}

/// `from_str_radix` in constant contexts, in native integers and arrays, at zero bits and at one
/// signed bit, with an error.
#[test]
fn parsing_in_constant_contexts() {
  type Bytes = Int<U24, [u8; 3]>;
  type Wide = Uint<U200, [u64; 4]>;
  const SMALL: Result<u7, ParseIntError> = u7::from_str_radix("7f", 16);
  const SIGNED: Result<i7, ParseIntError> = i7::from_str_radix("-64", 10);
  const BYTES: Result<Bytes, ParseIntError> = Bytes::from_str_radix("-800000", 16);
  const WIDE: Result<Wide, ParseIntError> = Wide::from_str_radix("+zz", 36);
  const ZERO_BITS: (Result<u0, ParseIntError>, Result<i0, ParseIntError>) =
    (u0::from_str_radix("00", 10), i0::from_str_radix("-0", 2));
  const ONE_BIT: Result<i1, ParseIntError> = i1::from_str_radix("-1", 2);
  const ERROR: Result<u7, ParseIntError> = u7::from_str_radix("80", 16);
  const VALUE: u7 = match u7::from_str_radix("100", 10) {
    Ok(value) => value,
    Err(_) => panic!("100 has 7 bits"),
  };

  assert_eq!(SMALL, Ok(u7::MAX));
  assert_eq!(SIGNED, Ok(i7::MIN));
  assert_eq!(BYTES, Ok(Bytes::MIN));
  assert_eq!(
    WIDE,
    Ok(Wide::checked_from_limbs([1295, 0, 0, 0]).expect("200 bits"))
  );
  assert_eq!(ZERO_BITS, (Ok(u0::ZERO), Ok(i0::ZERO)));
  assert_eq!(ONE_BIT, Ok(i1::MIN));
  assert_eq!(kind(ERROR), Err(IntErrorKind::PosOverflow));
  assert_eq!(VALUE, u7::new(100));
}

/// Zero bits hold only zero: every spelling of zero parses, `-0` only for `Int`, and any other
/// value overflows, in native integers and arrays; zero writes `0` in every radix, with the prefix
/// for `#`, and `0e0`. One signed bit holds -1 and 0, and -1 writes its one bit, `1`, in every
/// radix, and `-1e0`.
#[test]
fn zero_bits_and_one_signed_bit() {
  macro_rules! zero_bits {
    ($($ty:ty: $signed:literal),+ $(,)?) => {$({
      type T = $ty;
      for text in ["0", "+0", "00", "000"] {
        assert_eq!(T::from_str_radix(text, 10), Ok(T::ZERO), "{text}");
        assert_eq!(text.parse::<T>(), Ok(T::ZERO), "{text}");
      }
      let minus_zero = if $signed { Ok(T::ZERO) } else { Err(IntErrorKind::InvalidDigit) };
      assert_eq!(kind(T::from_str_radix("-0", 36)), minus_zero);
      let minus_one = if $signed { IntErrorKind::NegOverflow } else { IntErrorKind::InvalidDigit };
      assert_eq!(kind(T::from_str_radix("-1", 10)), Err(minus_one));
      assert_eq!(kind(T::from_str_radix("1", 2)), Err(IntErrorKind::PosOverflow));
      assert_eq!(kind("".parse::<T>()), Err(IntErrorKind::Empty));
      let zero = T::ZERO;
      assert_eq!(
        [
          format!("{zero:b}"),
          format!("{zero:#o}"),
          format!("{zero:#x}"),
          format!("{zero:X}"),
          format!("{zero:e}"),
          format!("{zero:.2E}"),
        ],
        ["0", "0o0", "0x0", "0", "0e0", "0.00E0"]
      );
    })+};
  }
  zero_bits!(
    u0: false,
    Uint<U0, u8>: false,
    Uint<U0, [u16; 1]>: false,
    i0: true,
    Int<U0, i8>: true,
    Int<U0, [u8; 1]>: true,
  );

  macro_rules! one_signed_bit {
    ($($ty:ty),+ $(,)?) => {$({
      type T = $ty;
      for (text, expected) in [
        ("-1", Ok(T::MIN)),
        ("0", Ok(T::ZERO)),
        ("-0", Ok(T::ZERO)),
        ("1", Err(IntErrorKind::PosOverflow)),
        ("-10", Err(IntErrorKind::NegOverflow)),
      ] {
        assert_eq!(kind(T::from_str_radix(text, 2)), expected, "{text}");
        assert_eq!(kind(text.parse::<T>()), expected, "{text}");
      }
      let min = T::MIN;
      assert_eq!(
        [
          format!("{min:b}"),
          format!("{min:o}"),
          format!("{min:#x}"),
          format!("{min:e}"),
        ],
        ["1", "1", "0x1", "-1e0"]
      );
    })+};
  }
  one_signed_bit!(i1, Int<U1, [u16; 1]>, Int<U1, [u64; 1]>);
}

/// The texts of `a` and `b` through the traits, in code over `B: BitWidth`, and what `FromStr` and
/// `from_str_radix` of `FixedInteger` read back from their decimal and hexadecimal texts.
fn text_over_the_width<B: BitWidth>(a: Uint<B>, b: Int<B>) -> Vec<String> {
  let decimal = (
    a.to_string().parse::<Uint<B>>(),
    b.to_string().parse::<Int<B>>(),
  );
  let radix = (
    <Uint<B> as FixedInteger>::from_str_radix(&format!("{a:x}"), 16),
    <Int<B> as FixedInteger>::from_str_radix(&b.to_string(), 10),
  );
  vec![
    format!("{a:b} {a:#o} {a:x} {a:X} {a:e} {a:.2E}"),
    format!("{b:b} {b:#o} {b:x} {b:X} {b:e} {b:.2E}"),
    format!("{decimal:?} {radix:?}"),
  ]
}

/// Code over `B: BitWidth` reaches the text of `Uint<B>` and `Int<B>`, which is that of the default
/// storages, and parses their texts back through `FromStr` and `FixedInteger`, as the inherent
/// `from_str_radix` does: at widths of each kind of default storage, at zero, the limits and values
/// that wrap from them. Under Miri, at 0, 7, 65 and 129 bits.
#[test]
fn text_over_the_width_matches_the_default_storages() {
  macro_rules! agrees {
    ($($width:ident),+ $(,)?) => {$({
      type U = Uint<$width>;
      type I = Int<$width>;
      for (a, b) in [
        (U::ZERO, I::ZERO),
        (U::MAX, I::MIN),
        (U::MAX.wrapping_mul(&U::MAX), I::MAX),
        (U::MAX.wrapping_add(&U::MAX), I::MIN.wrapping_sub(&I::MAX)),
      ] {
        let decimal = (Ok::<U, ParseIntError>(a), Ok::<I, ParseIntError>(b));
        let radix = (
          U::from_str_radix(&format!("{a:x}"), 16),
          I::from_str_radix(&b.to_string(), 10),
        );
        assert_eq!(radix, decimal, "{a} and {b}");
        let inherent = vec![
          format!("{a:b} {a:#o} {a:x} {a:X} {a:e} {a:.2E}"),
          format!("{b:b} {b:#o} {b:x} {b:X} {b:e} {b:.2E}"),
          format!("{decimal:?} {radix:?}"),
        ];
        assert_eq!(text_over_the_width::<$width>(a, b), inherent, "{a} and {b}");
      }
    })+};
  }

  agrees!(U0, U7, U65, U129);
  if !tier().is_miri() {
    agrees!(U1, U8, U64, U128, U200, U1024);
  }
}

/// A value read from its decimal text through `FromStr` alone, with the error of the native
/// integers, which the native integers and the integers of this crate share.
fn parse_any<T: FromStr<Err = ParseIntError>>(text: &str) -> Result<T, IntErrorKind> {
  kind(text.parse::<T>())
}

/// The text of `x` through `FixedInteger` alone: its radix and exponent formats, and what `FromStr`
/// and `from_str_radix` read back from its decimal text.
fn fixed_text<T: FixedInteger>(
  x: &T,
) -> (String, Result<T, ParseIntError>, Result<T, ParseIntError>) {
  let decimal = x.to_string();
  (
    format!("{x:b} {x:o} {x:x} {x:X} {x:e} {x:E}"),
    decimal.parse(),
    T::from_str_radix(&decimal, 10),
  )
}

/// `FixedInteger` requires the text of the integers on every storage, native or array, of both
/// signs, at zero bits and at one signed bit: `FromStr` with the error of the native integers, so
/// that one function bounded by `FromStr<Err = ParseIntError>` takes `u8` and `u7` alike, the
/// radix and exponent formats, and `from_str_radix`, which gives the results of the inherent items.
#[test]
fn text_trait_surface() {
  assert_eq!(parse_any::<u8>("200"), Ok(200));
  assert_eq!(parse_any::<u7>("100"), Ok(u7::new(100)));
  assert_eq!(parse_any::<u8>("256"), Err(IntErrorKind::PosOverflow));
  assert_eq!(parse_any::<u7>("128"), Err(IntErrorKind::PosOverflow));
  assert_eq!(parse_any::<i8>("-129"), Err(IntErrorKind::NegOverflow));
  assert_eq!(parse_any::<i256>("-1"), Ok(-i256::ONE));

  macro_rules! surface {
    ($($ty:ty: $value:expr),+ $(,)?) => {$({
      type T = $ty;
      let x: T = $value;
      let decimal = x.to_string();
      let inherent = (
        format!("{x:b} {x:o} {x:x} {x:X} {x:e} {x:E}"),
        Ok(x),
        T::from_str_radix(&decimal, 10),
      );
      assert_eq!(fixed_text(&x), inherent, "{x}");
      for (text, radix) in [("+11", 2), ("-z", 36), ("", 10)] {
        assert_eq!(
          <T as FixedInteger>::from_str_radix(text, radix),
          T::from_str_radix(text, radix),
          "{text} in radix {radix}"
        );
      }
    })+};
  }
  surface!(
    u7: u7::MAX,
    i7: i7::MIN,
    Uint<U8, u8>: Uint::<U8, u8>::MAX,
    Int<U64, i64>: Int::<U64, i64>::MIN,
    Uint<U24, [u8; 3]>: Uint::<U24, [u8; 3]>::MAX,
    Int<U24, [u8; 3]>: Int::<U24, [u8; 3]>::MIN,
    Uint<U65, [u16; 5]>: Uint::<U65, [u16; 5]>::MAX,
    Int<U129, [u32; 5]>: Int::<U129, [u32; 5]>::MIN,
    Uint<U200, [u128; 2]>: Uint::<U200, [u128; 2]>::MAX,
    u256: u256::MAX,
    i256: i256::MIN,
    u0: u0::ZERO,
    i0: i0::ZERO,
    i1: i1::MIN,
  );
}
