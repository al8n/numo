//! The serde of `Uint` and `Int`. Up to 128 bits, the form of each class against the native integer
//! of the class, in JSON and postcard and in every family of storages, and the request of each
//! class in each column through a stand-in format that records it; serde_test's tokens in both
//! configurations; every integer and every string, checked against the width; the rejections, with
//! the fixed message, std's texts and the errors of a format that pass through; the decimal text
//! above 128 bits; serde_json's number token; csv's inference, a human-readable format without
//! `deserialize_any`, serde's buffer of untagged enums, flattened structs and internally tagged
//! enums, and JSON map keys; zero bits and one signed bit; and code generic over the storage traits
//! or over the width alone.
//!
//! Every expected text of std comes from std at run time. Each width group or family of storages
//! checks in a function of its own, and each generic check is `#[inline(never)]`, so that a frame
//! holds the values of one type alone.

use core::{any::type_name, fmt::Display, num::ParseIntError, str::FromStr};
use std::collections::BTreeMap;

use serde_core::{
  Deserialize, Deserializer, Serialize,
  de::{self, DeserializeOwned, IntoDeserializer, Visitor},
  forward_to_deserialize_any,
};
use serde_json::error::Category;
use serde_test::{
  Compact, Configure, Readable, Token, assert_de_tokens, assert_de_tokens_error, assert_tokens,
};
use typenum::{U12, U15, U17, U31, U33, U192, U8192, U16384, U65536};

use super::*;
use crate::{CheckedCastFrom, IntRepr, UintRepr, i0, i1, u0};

/// The fixed message of the defaults: the error of every input that is neither an integer nor a
/// string.
const FIXED: &str = "expected an integer";

/// The text of std's error of a conversion of a number above the range of its target.
fn too_large() -> String {
  u8::try_from(256u16).unwrap_err().to_string()
}

/// The text of std's error of a conversion of a number below the range of its target.
fn too_small() -> String {
  u8::try_from(-1i8).unwrap_err().to_string()
}

/// The text of the error of std's parser of `text` into `N`.
fn parse_error<N: FromStr<Err = ParseIntError>>(text: &str) -> String {
  match text.parse::<N>() {
    Ok(_) => panic!("{text:?} parses"),
    Err(error) => error.to_string(),
  }
}

/// The message of `error` without the position that serde_json appends to it.
fn json_message(error: &serde_json::Error) -> String {
  let message = error.to_string();
  let position = format!(" at line {} column {}", error.line(), error.column());
  message
    .strip_suffix(&position)
    .unwrap_or(&message)
    .to_string()
}

/// A native integer, whose own serde is the oracle of the form of its class.
trait Native: Serialize + DeserializeOwned + Copy + Debug + PartialEq {
  /// The native integer of the low bits of `pattern`, read in two's complement for a signed one.
  fn of_pattern(pattern: u128) -> Self;
}

/// Implements [`Native`] for the native integers.
macro_rules! native {
  ($($native:ident),+) => {$(
    impl Native for $native {
      fn of_pattern(pattern: u128) -> Self {
        pattern as $native
      }
    }
  )+};
}

native!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

/// Whether the value whose two's complement in 128 bits is `pattern` has `bits` bits, at most 128,
/// unsigned or signed: its bits from the width up, or from the sign bit up, copy its sign.
fn in_width(pattern: u128, bits: u32, signed: bool) -> bool {
  match (bits, signed) {
    (0, _) => pattern == 0,
    (128, _) => true,
    (_, false) => pattern >> bits == 0,
    (_, true) => ((pattern as i128) << (128 - bits) >> (128 - bits)) as u128 == pattern,
  }
}

/// `count` of `samples`, spread evenly from the first, or all of them if there are no more.
fn spread<T: Clone>(samples: &[T], count: usize) -> Vec<T> {
  if samples.len() <= count {
    return samples.to_vec();
  }
  (0..count)
    .map(|k| samples[k * samples.len() / count].clone())
    .collect()
}

/// The values of `bits` bits, at most 128, at which the forms of a class are checked, as their
/// two's complement in 128 bits: the limits, the values next to them, -1, 0 and 1, where they are
/// in range; in an optimized build every value up to 8 bits and 64 of the patterns of
/// [`wide_patterns`] above; and in an unoptimized one 8 of the patterns above 16 bits.
fn samples(bits: u32, signed: bool) -> Vec<u128> {
  let (min, max) = match (bits, signed) {
    (0, _) => (0, 0),
    (_, false) => (0, u128::MAX >> (128 - bits)),
    (_, true) => (
      (i128::MIN >> (128 - bits)) as u128,
      (i128::MAX >> (128 - bits)) as u128,
    ),
  };
  // -1 is every bit set.
  let ends = [
    min,
    min.wrapping_add(1),
    u128::MAX,
    0,
    1,
    max.wrapping_sub(1),
    max,
  ];
  let mut samples: Vec<u128> = ends
    .into_iter()
    .filter(|&pattern| in_width(pattern, bits, signed))
    .collect();
  let patterns = || -> Vec<u128> {
    wide_patterns::<2>(bits)
      .iter()
      .map(|p| model::to_u128(&model::extend(p, 128, signed)))
      .collect()
  };
  match tier() {
    Tier::Full if bits <= 8 => samples.extend((0..1u128 << bits).map(|pattern| {
      if signed {
        bigint::to_signed(pattern, bits) as u128
      } else {
        pattern
      }
    })),
    Tier::Full => samples.extend(spread(&patterns(), 64)),
    Tier::Debug if bits > 16 => samples.extend(spread(&patterns(), 8)),
    _ => {}
  }
  samples.sort_unstable();
  samples.dedup();
  samples
}

/// The values of [`samples`] as the native integer `N` of their class.
fn natives<N: Native>(bits: u32, signed: bool) -> Vec<N> {
  samples(bits, signed)
    .into_iter()
    .map(N::of_pattern)
    .collect()
}

/// Checks that `T`, of the class whose native integer is `N`, writes each value of `values` as `N`
/// writes it, in JSON and in postcard, and reads it back from both: in JSON through
/// `deserialize_any` or the request of a 128-bit integer, and in postcard through the request of
/// the class.
#[inline(never)]
fn written_as_native<T, N>(values: &[N])
where
  T: CheckedCastFrom<N> + Serialize + DeserializeOwned + PartialEq + Debug,
  N: Native,
{
  let name = type_name::<T>();
  for &native in values {
    let value = T::checked_cast_from(native).expect("a value of the width");
    let json = serde_json::to_string(&native).unwrap();
    let bytes = postcard::to_stdvec(&native).unwrap();
    assert_eq!(
      serde_json::to_string(&value).unwrap(),
      json,
      "{native:?} in {name}"
    );
    assert_eq!(
      postcard::to_stdvec(&value).unwrap(),
      bytes,
      "{native:?} in {name}"
    );
    assert_eq!(
      serde_json::from_str::<T>(&json).unwrap(),
      value,
      "{json} in {name}"
    );
    assert_eq!(
      postcard::from_bytes::<T>(&bytes).unwrap(),
      value,
      "{native:?} in {name}"
    );
  }
}

/// Checks the forms of `Uint` and `Int` of `$bits` bits, the width `$width`, whose class has the
/// native integers `$unsigned` and `$signed`, with [`written_as_native`] in each pair of storages
/// of `$storages`, written `unsigned, signed;`, at the values of [`samples`].
macro_rules! forms_agree {
  ($width:ty, $bits:literal, $unsigned:ident, $signed:ident; $($uint:ty, $int:ty);+ $(;)?) => {{
    let unsigned: Vec<$unsigned> = natives($bits, false);
    let signed: Vec<$signed> = natives($bits, true);
    $(
      written_as_native::<Uint<$width, $uint>, $unsigned>(&unsigned);
      written_as_native::<Int<$width, $int>, $signed>(&signed);
    )+
  }};
}

/// The class of `u8` and `i8`: zero bits in `[u64; 0]`, a native integer and `[u8; 1]`; one bit in
/// a native integer, `[u16; 1]` and `[u32; 1]`; 7 bits in a native integer, `[u64; 1]` and
/// `[u128; 1]`; and 8 bits in a native integer, a wider one and `[u8; 1]`.
#[inline(never)]
fn forms_of_the_8_bit_class() {
  forms_agree!(U0, 0, u8, i8; [u64; 0], [u64; 0]; u8, i8; [u8; 1], [u8; 1]);
  forms_agree!(U1, 1, u8, i8; u8, i8; [u16; 1], [u16; 1]; [u32; 1], [u32; 1]);
  forms_agree!(U7, 7, u8, i8; u8, i8; [u64; 1], [u64; 1]; [u128; 1], [u128; 1]);
  forms_agree!(U8, 8, u8, i8; u8, i8; u16, i16; [u8; 1], [u8; 1]);
}

/// The class of `u16` and `i16`: 9 bits in a native integer, `[u8; 2]` and `[u16; 1]`; 15 bits in a
/// native integer, `[u32; 1]` and `[u64; 1]`; and 16 bits in a native integer, `[u128; 1]` and a
/// wider native integer.
#[inline(never)]
fn forms_of_the_16_bit_class() {
  forms_agree!(U9, 9, u16, i16; u16, i16; [u8; 2], [u8; 2]; [u16; 1], [u16; 1]);
  forms_agree!(U15, 15, u16, i16; u16, i16; [u32; 1], [u32; 1]; [u64; 1], [u64; 1]);
  forms_agree!(U16, 16, u16, i16; u16, i16; [u128; 1], [u128; 1]; u32, i32);
}

/// The class of `u32` and `i32`: 17 bits in a native integer, `[u8; 3]` and `[u16; 2]`; 31 bits in
/// a native integer, `[u32; 1]` and `[u64; 1]`; and 32 bits in a native integer, `[u128; 1]` and a
/// wider native integer.
#[inline(never)]
fn forms_of_the_32_bit_class() {
  forms_agree!(U17, 17, u32, i32; u32, i32; [u8; 3], [u8; 3]; [u16; 2], [u16; 2]);
  forms_agree!(U31, 31, u32, i32; u32, i32; [u32; 1], [u32; 1]; [u64; 1], [u64; 1]);
  forms_agree!(U32, 32, u32, i32; u32, i32; [u128; 1], [u128; 1]; u64, i64);
}

/// The class of `u64` and `i64`: 33 bits in a native integer, `[u8; 5]` and `[u16; 3]`; 63 bits in
/// a native integer, `[u32; 2]` and `[u64; 1]`; and 64 bits in a native integer, `[u128; 1]` and a
/// wider native integer.
#[inline(never)]
fn forms_of_the_64_bit_class() {
  forms_agree!(U33, 33, u64, i64; u64, i64; [u8; 5], [u8; 5]; [u16; 3], [u16; 3]);
  forms_agree!(U63, 63, u64, i64; u64, i64; [u32; 2], [u32; 2]; [u64; 1], [u64; 1]);
  forms_agree!(U64, 64, u64, i64; u64, i64; [u128; 1], [u128; 1]; u128, i128);
}

/// The class of `u128` and `i128`, which no wider native integer stores: 65 bits in a native
/// integer, `[u8; 9]` and `[u16; 5]`; 100 bits in a native integer, `[u32; 4]` and `[u64; 2]`; 127
/// bits in a native integer, `[u128; 1]` and `[u8; 16]`; and 128 bits in a native integer,
/// `[u64; 2]` and `[u32; 4]`.
#[inline(never)]
fn forms_of_the_128_bit_class() {
  forms_agree!(U65, 65, u128, i128; u128, i128; [u8; 9], [u8; 9]; [u16; 5], [u16; 5]);
  forms_agree!(U100, 100, u128, i128; u128, i128; [u32; 4], [u32; 4]; [u64; 2], [u64; 2]);
  forms_agree!(U127, 127, u128, i128; u128, i128; [u128; 1], [u128; 1]; [u8; 16], [u8; 16]);
  forms_agree!(U128, 128, u128, i128; u128, i128; [u64; 2], [u64; 2]; [u32; 4], [u32; 4]);
}

/// Up to 128 bits, a value is written as the native integer of its class writes it, in JSON and in
/// postcard, and reads back from both, in the default storage of each width and in two others,
/// which rotate over the widths so that arrays of every type of limb and a wider native integer
/// meet every class.
#[test]
fn up_to_128_bits_a_value_is_written_as_the_native_integer_of_its_class() {
  forms_of_the_8_bit_class();
  forms_of_the_16_bit_class();
  forms_of_the_32_bit_class();
  forms_of_the_64_bit_class();
  forms_of_the_128_bit_class();
}

/// A stand-in format, which answers each request as its mode says.
#[derive(Clone, Copy, Debug)]
enum Probe {
  /// Fails every request with the name of its method, in a human-readable format if the flag is
  /// set. The requests of the 128-bit integers keep serde's default, whose error names the integer.
  Record(bool),
  /// A human-readable format without `deserialize_any`, as serde-json-core: `deserialize_any`
  /// fails, the requests of the 128-bit integers keep serde's default, and the requests of the
  /// integers up to 64 bits and of a string hand over 5.
  WithoutAny,
}

/// Implements the requests of a [`Probe`] that fail in every mode, written `method(inputs)`, with
/// the name of the method.
macro_rules! failing_requests {
  ($($method:ident($($input:ty),*)),+ $(,)?) => {$(
    fn $method<V: Visitor<'de>>(self, $(_: $input,)* _: V) -> Result<V::Value, Self::Error> {
      Err(de::Error::custom(stringify!($method)))
    }
  )+};
}

/// Implements the requests of a [`Probe`] that hand over a value without `deserialize_any`,
/// written `method: visit(value)`, and fail in a record with the name of the method.
macro_rules! handing_requests {
  ($($method:ident: $visit:ident($value:expr)),+ $(,)?) => {$(
    fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
      match self {
        Self::Record(_) => Err(de::Error::custom(stringify!($method))),
        Self::WithoutAny => visitor.$visit($value),
      }
    }
  )+};
}

impl<'de> Deserializer<'de> for Probe {
  type Error = de::value::Error;

  fn deserialize_any<V: Visitor<'de>>(self, _: V) -> Result<V::Value, Self::Error> {
    Err(de::Error::custom(match self {
      Self::Record(_) => "deserialize_any",
      Self::WithoutAny => "any is unsupported",
    }))
  }

  handing_requests! {
    deserialize_u8: visit_u8(5),
    deserialize_u16: visit_u16(5),
    deserialize_u32: visit_u32(5),
    deserialize_u64: visit_u64(5),
    deserialize_i8: visit_i8(5),
    deserialize_i16: visit_i16(5),
    deserialize_i32: visit_i32(5),
    deserialize_i64: visit_i64(5),
    deserialize_str: visit_str("5"),
  }

  failing_requests! {
    deserialize_bool(),
    deserialize_f32(),
    deserialize_f64(),
    deserialize_char(),
    deserialize_string(),
    deserialize_bytes(),
    deserialize_byte_buf(),
    deserialize_option(),
    deserialize_unit(),
    deserialize_unit_struct(&'static str),
    deserialize_newtype_struct(&'static str),
    deserialize_seq(),
    deserialize_tuple(usize),
    deserialize_tuple_struct(&'static str, usize),
    deserialize_map(),
    deserialize_struct(&'static str, &'static [&'static str]),
    deserialize_enum(&'static str, &'static [&'static str]),
    deserialize_identifier(),
    deserialize_ignored_any(),
  }

  fn is_human_readable(&self) -> bool {
    match self {
      Self::Record(readable) => *readable,
      Self::WithoutAny => true,
    }
  }
}

/// The text of the error of `T` from `probe`.
#[inline(never)]
fn error_from<T: DeserializeOwned + Debug>(probe: Probe) -> String {
  T::deserialize(probe).unwrap_err().to_string()
}

/// Asserts that `T`, of the class whose native integer is `N`, asks a format for what `N` asks for,
/// but for any value in a human-readable format if `any`.
#[inline(never)]
fn asks_as_its_class<T, N>(any: bool)
where
  T: DeserializeOwned + Debug,
  N: DeserializeOwned + Debug,
{
  let name = type_name::<T>();
  let readable = if any {
    "deserialize_any".to_string()
  } else {
    error_from::<N>(Probe::Record(true))
  };
  assert_eq!(error_from::<T>(Probe::Record(true)), readable, "{name}");
  assert_eq!(
    error_from::<T>(Probe::Record(false)),
    error_from::<N>(Probe::Record(false)),
    "{name}"
  );
}

/// Checks [`asks_as_its_class`] at `Uint` and `Int` of each width `$width`, in the default storage,
/// whose class has the native integers `$unsigned` and `$signed`.
macro_rules! asks_as_its_class {
  ($any:literal; $($width:ty: $unsigned:ty, $signed:ty;)+) => {$(
    asks_as_its_class::<Uint<$width>, $unsigned>($any);
    asks_as_its_class::<Int<$width>, $signed>($any);
  )+};
}

/// The defaults ask a format for exactly the method of their column, at each class and sign: any
/// value in a human-readable format at 0 to 64 bits and above 128 bits; the request of the class
/// in another format, which is what its native integer asks for; the request of a 128-bit integer
/// in every format from 65 to 128 bits; and a string in another format above 128 bits. Postcard
/// writes the integers of 16 to 128 bits alike, so only a recording stand-in tells their requests
/// apart.
#[test]
fn each_class_asks_for_the_method_of_its_column() {
  asks_as_its_class!(true;
    U0: u8, i8;
    U1: u8, i8;
    U8: u8, i8;
    U9: u16, i16;
    U16: u16, i16;
    U17: u32, i32;
    U32: u32, i32;
    U33: u64, i64;
    U64: u64, i64;
  );
  asks_as_its_class!(false;
    U65: u128, i128;
    U128: u128, i128;
  );
  for readable in [true, false] {
    let expected = if readable {
      "deserialize_any"
    } else {
      "deserialize_str"
    };
    assert_eq!(error_from::<Uint<U129>>(Probe::Record(readable)), expected);
    assert_eq!(error_from::<Int<U129>>(Probe::Record(readable)), expected);
    assert_eq!(error_from::<u4096>(Probe::Record(readable)), expected);
  }
}

/// Asserts that `value` is written as `token` and reads back from it, in both configurations of
/// serde_test, which answers every request through `deserialize_any`, and that the decimal string
/// `text` reads as `value` in both.
#[inline(never)]
fn tokens_round_trip<T>(value: T, token: Token, text: &'static str)
where
  T: Serialize + DeserializeOwned + PartialEq + Debug + Clone,
{
  assert_tokens(&value.clone().readable(), &[token]);
  assert_tokens(&value.clone().compact(), &[token]);
  assert_de_tokens(&value.clone().readable(), &[Token::Str(text)]);
  assert_de_tokens(&value.compact(), &[Token::Str(text)]);
}

/// A value of each class up to 64 bits is written as the token of the native integer of its class,
/// and above 128 bits as a string, and reads back from it, in both configurations of serde_test; a
/// decimal string reads in both, as the visitor is the same in both columns.
#[test]
fn tokens_in_both_configurations() {
  tokens_round_trip(u7::new(100), Token::U8(100), "100");
  tokens_round_trip(Uint::<U16>::MAX, Token::U16(u16::MAX), "65535");
  tokens_round_trip(Uint::<U24>::new(70_000), Token::U32(70_000), "70000");
  tokens_round_trip(
    Uint::<U64>::MAX,
    Token::U64(u64::MAX),
    "18446744073709551615",
  );
  tokens_round_trip(i7::MIN, Token::I8(-64), "-64");
  tokens_round_trip(Int::<U16>::MIN, Token::I16(i16::MIN), "-32768");
  tokens_round_trip(Int::<U24>::new(-70_000), Token::I32(-70_000), "-70000");
  tokens_round_trip(
    Int::<U64>::MIN,
    Token::I64(i64::MIN),
    "-9223372036854775808",
  );
  // 2^128 and its negation, beyond every native integer.
  let (power, negated) = (
    "340282366920938463463374607431768211456",
    "-340282366920938463463374607431768211456",
  );
  tokens_round_trip(power.parse::<u256>().unwrap(), Token::Str(power), power);
  tokens_round_trip(
    negated.parse::<i256>().unwrap(),
    Token::Str(negated),
    negated,
  );
  tokens_round_trip(
    negated.parse::<Int<U129, [u32; 5]>>().unwrap(),
    Token::Str(negated),
    negated,
  );
}

/// A number that a reader is handed, as the widest native integer of its sign.
#[derive(Clone, Copy, Debug)]
enum Number {
  /// A number below zero.
  Negative(i128),
  /// A number from zero up.
  NonNegative(u128),
}

impl Number {
  /// Whether the number is in the range of `bits` bits, unsigned or in two's complement.
  fn fits(self, bits: u32, signed: bool) -> bool {
    match self {
      Self::Negative(n) => signed && (bits > 128 || (bits > 0 && n >= i128::MIN >> (128 - bits))),
      Self::NonNegative(n) => match (bits, signed) {
        (0, _) => n == 0,
        (129.., _) => true,
        (_, false) => n <= u128::MAX >> (128 - bits),
        (_, true) => n <= (i128::MAX >> (128 - bits)) as u128,
      },
    }
  }

  /// The decimal text of the number.
  fn text(self) -> String {
    match self {
      Self::Negative(n) => n.to_string(),
      Self::NonNegative(n) => n.to_string(),
    }
  }
}

/// The numbers at which the readers of `bits` bits are checked: up to 128 bits zero, the limits of
/// the width and the numbers next to them outside, where a native integer holds them; and above
/// 128 bits the limits of the 128-bit integers, -1 and zero.
fn numbers(bits: u32, signed: bool) -> Vec<Number> {
  use Number::{Negative, NonNegative};
  match (bits, signed) {
    (129.., _) => vec![
      Negative(i128::MIN),
      Negative(-1),
      NonNegative(0),
      NonNegative(u128::MAX),
    ],
    (0, _) => vec![Negative(-1), NonNegative(0), NonNegative(1)],
    (_, false) => {
      let max = u128::MAX >> (128 - bits);
      let mut numbers = vec![Negative(-1), NonNegative(0), NonNegative(max)];
      numbers.extend(max.checked_add(1).map(NonNegative));
      numbers
    }
    (_, true) => {
      let (min, max) = (i128::MIN >> (128 - bits), i128::MAX >> (128 - bits));
      let mut numbers: Vec<Number> = min.checked_sub(1).map(Negative).into_iter().collect();
      numbers.extend([
        Negative(min),
        NonNegative(0),
        NonNegative(max as u128),
        NonNegative(max as u128 + 1),
      ]);
      numbers
    }
  }
}

/// What `T` reads from serde's deserializer of `number` as the native integer `P`, with the text of
/// its error, if `P` holds the number.
#[inline(never)]
fn read_as<T, P>(number: Number) -> Option<Result<T, String>>
where
  T: DeserializeOwned,
  P: TryFrom<i128> + TryFrom<u128> + IntoDeserializer<'static, de::value::Error>,
{
  let native = match number {
    Number::Negative(n) => P::try_from(n).ok()?,
    Number::NonNegative(n) => P::try_from(n).ok()?,
  };
  Some(T::deserialize(native.into_deserializer()).map_err(|error| error.to_string()))
}

/// Asserts that `T`, of `bits` bits, reads each number of [`numbers`] from serde's deserializer of
/// every native integer that holds it, which hands it over through `deserialize_any`, and its
/// decimal text from serde's deserializer of a string: the value inside the range of the width;
/// outside it the text of std's error of a conversion for a number, and of std's parser for a
/// text, an invalid digit for `-` before the digits of an unsigned integer.
#[inline(never)]
fn reads_numbers_and_strings<T>(bits: u32, signed: bool)
where
  T: DeserializeOwned + CheckedCastFrom<i128> + CheckedCastFrom<u128> + PartialEq + Debug,
{
  let name = type_name::<T>();
  for number in numbers(bits, signed) {
    let fits = number.fits(bits, signed);
    let value = || match number {
      Number::Negative(n) => T::checked_cast_from(n).expect("in range"),
      Number::NonNegative(n) => T::checked_cast_from(n).expect("in range"),
    };
    let expected = match (fits, number) {
      (true, _) => Ok(value()),
      (false, Number::Negative(_)) => Err(too_small()),
      (false, Number::NonNegative(_)) => Err(too_large()),
    };
    let results = [
      read_as::<T, u8>(number),
      read_as::<T, u16>(number),
      read_as::<T, u32>(number),
      read_as::<T, u64>(number),
      read_as::<T, u128>(number),
      read_as::<T, i8>(number),
      read_as::<T, i16>(number),
      read_as::<T, i32>(number),
      read_as::<T, i64>(number),
      read_as::<T, i128>(number),
    ];
    for result in results.into_iter().flatten() {
      assert_eq!(result, expected, "{number:?} into {name}");
    }
    let text = number.text();
    let expected = match (fits, number) {
      (true, _) => Ok(value()),
      (false, Number::NonNegative(_)) => Err(parse_error::<u8>("256")),
      (false, Number::Negative(_)) if signed => Err(parse_error::<i8>("-129")),
      (false, Number::Negative(_)) => Err(parse_error::<u8>("-1")),
    };
    let result = T::deserialize(text.as_str().into_deserializer());
    let result = result.map_err(|error: de::value::Error| error.to_string());
    assert_eq!(result, expected, "{text:?} into {name}");
  }
}

/// Checks [`reads_numbers_and_strings`] at `Uint` and `Int` of each width `$width`, `$bits` bits,
/// in the default storage.
macro_rules! numbers_and_strings {
  ($($width:ty: $bits:literal),+ $(,)?) => {$(
    reads_numbers_and_strings::<Uint<$width>>($bits, false);
    reads_numbers_and_strings::<Int<$width>>($bits, true);
  )+};
}

/// [`reads_numbers_and_strings`] at the widths of up to 64 bits.
#[inline(never)]
fn numbers_and_strings_up_to_64_bits() {
  numbers_and_strings!(
    U0: 0,
    U1: 1,
    U7: 7,
    U8: 8,
    U9: 9,
    U15: 15,
    U16: 16,
    U17: 17,
    U31: 31,
    U32: 32,
    U33: 33,
    U63: 63,
    U64: 64,
  );
}

/// [`reads_numbers_and_strings`] at the widths of 65 to 128 bits and above, where serde's
/// deserializers answer the request of a 128-bit integer through `deserialize_any`.
#[inline(never)]
fn numbers_and_strings_from_65_bits() {
  numbers_and_strings!(
    U65: 65,
    U100: 100,
    U127: 127,
    U128: 128,
    U129: 129,
    U256: 256,
  );
}

/// Asserts that `T`, which holds 5, reads the JSON strings `"5"`, `"+5"` and `"005"` as 5, and
/// `"-0"` as zero if `signed`.
#[inline(never)]
fn reads_json_strings<T>(signed: bool)
where
  T: DeserializeOwned + CheckedCastFrom<u8> + PartialEq + Debug,
{
  let five = T::checked_cast_from(5u8).unwrap();
  for json in ["\"5\"", "\"+5\"", "\"005\""] {
    assert_eq!(serde_json::from_str::<T>(json).unwrap(), five, "{json}");
  }
  if signed {
    let zero = T::checked_cast_from(0u8).unwrap();
    assert_eq!(serde_json::from_str::<T>("\"-0\"").unwrap(), zero);
  }
}

/// Every integer reads, checked against the width rather than the class: from serde's
/// deserializer of each native integer, at the limits of the width and one beyond, the value
/// inside and std's texts of a conversion outside. Every decimal string reads, from serde's
/// deserializer of a string with std's texts of its parser outside the range, from JSON at 0 to 64
/// bits and above 128 bits, and from serde_test at 65 to 128 bits.
#[test]
fn every_integer_and_every_string_reads_checked_against_the_width() {
  numbers_and_strings_up_to_64_bits();
  numbers_and_strings_from_65_bits();
  reads_json_strings::<u7>(false);
  reads_json_strings::<i7>(true);
  reads_json_strings::<Uint<U64, [u16; 4]>>(false);
  reads_json_strings::<Int<U33, [u8; 5]>>(true);
  reads_json_strings::<u256>(false);
  reads_json_strings::<Int<U200, [u128; 2]>>(true);
  // At 65 to 128 bits a string reads where a format gives one to the request of a 128-bit integer.
  for text in ["5", "+5", "005"] {
    assert_de_tokens(&Uint::<U100>::new(5).readable(), &[Token::Str(text)]);
    assert_de_tokens(&Int::<U65>::new(5).readable(), &[Token::Str(text)]);
  }
  assert_de_tokens(&Int::<U128>::new(0).readable(), &[Token::Str("-0")]);
}

/// Asserts that `T` rejects every token of serde_test that is neither an integer nor a string with
/// exactly the fixed message, in both configurations: `char` too, and a sequence and a map only
/// empty, so that the rejection leaves no more than one token unread; and that it rejects the
/// strings `""`, `" 5"` and `"0x5"` with the texts of std's parser.
#[inline(never)]
fn rejects_other_tokens<T: DeserializeOwned>() {
  for tokens in [
    &[Token::Bool(true)][..],
    &[Token::F32(5.0)],
    &[Token::F64(1.5)],
    &[Token::Char('5')],
    &[Token::Bytes(b"5")],
    &[Token::BorrowedBytes(b"5")],
    &[Token::ByteBuf(b"5")],
    &[Token::None],
    &[Token::Some, Token::U8(5)],
    &[Token::Unit],
    &[Token::UnitStruct { name: "Unit" }],
    &[Token::NewtypeStruct { name: "Newtype" }, Token::U8(5)],
    &[Token::Seq { len: Some(0) }, Token::SeqEnd],
    &[Token::Tuple { len: 0 }, Token::TupleEnd],
    &[Token::Map { len: Some(0) }, Token::MapEnd],
  ] {
    assert_de_tokens_error::<Readable<T>>(tokens, FIXED);
    assert_de_tokens_error::<Compact<T>>(tokens, FIXED);
  }
  for text in ["", " 5", "0x5"] {
    let expected = parse_error::<u8>(text);
    assert_de_tokens_error::<Readable<T>>(&[Token::Str(text)], &expected);
    assert_de_tokens_error::<Compact<T>>(&[Token::Str(text)], &expected);
  }
}

/// Asserts that `T`, of 0 to 64 bits or above 128 bits, rejects the JSON values that are neither
/// integers nor strings with the fixed message and the category of the data, a fraction, an
/// exponent, `-0` and a number beyond the 64-bit range included, which serde_json gives as floats,
/// and the strings `""`, `" 5"` and `"0x5"` with the texts of std's parser; no message holds the
/// input.
#[inline(never)]
fn rejects_other_json<T: DeserializeOwned + Debug>() {
  let name = type_name::<T>();
  for json in [
    "1.5",
    "5.0",
    "1e2",
    "-0",
    "true",
    "null",
    "[]",
    "{}",
    "[5]",
    r#"{"a":1}"#,
    "18446744073709551616",
  ] {
    let error = serde_json::from_str::<T>(json).unwrap_err();
    assert_eq!(json_message(&error), FIXED, "{json} into {name}");
    assert_eq!(error.classify(), Category::Data, "{json} into {name}");
  }
  for text in ["", " 5", "0x5"] {
    let error = serde_json::from_str::<T>(&format!("\"{text}\"")).unwrap_err();
    assert_eq!(
      json_message(&error),
      parse_error::<u8>(text),
      "{text:?} into {name}"
    );
  }
}

/// Asserts that `T`, of 65 to 128 bits, rejects JSON values as its native integer `N` does:
/// serde_json reads digits alone for the request of a 128-bit integer, without the visitor, so that
/// its own errors pass through: a string, a literal, an array and an object are an invalid number,
/// and a fraction trailing characters; and for an unsigned integer `-5` and `-0` are out of range.
#[inline(never)]
fn rejects_json_as<T, N>(signed: bool)
where
  T: DeserializeOwned + Debug,
  N: DeserializeOwned + Debug,
{
  let name = type_name::<T>();
  let unsigned = if signed { &[][..] } else { &["-5", "-0"][..] };
  for &json in ["\"5\"", "true", "null", "[5]", "{}", "1.5"]
    .iter()
    .chain(unsigned)
  {
    let error = serde_json::from_str::<T>(json).unwrap_err();
    let native = serde_json::from_str::<N>(json).unwrap_err();
    assert_eq!(error.to_string(), native.to_string(), "{json} into {name}");
    assert_eq!(error.classify(), native.classify(), "{json} into {name}");
  }
}

/// Asserts that `T` passes postcard's error of an input that ends early through, with its
/// category: the bytes of `value` without the last.
#[inline(never)]
fn truncated_postcard_passes_through<T>(value: &T)
where
  T: Serialize + DeserializeOwned + Debug,
{
  let bytes = postcard::to_stdvec(value).unwrap();
  assert_eq!(
    postcard::from_bytes::<T>(&bytes[..bytes.len() - 1]).unwrap_err(),
    postcard::Error::DeserializeUnexpectedEnd,
    "{value:?}"
  );
}

/// Checks the rejections of `Uint` and `Int` of each width `$width` of 0 to 64 bits or above 128
/// bits, in the default storage: [`rejects_other_tokens`], [`rejects_other_json`] and
/// [`truncated_postcard_passes_through`] at the largest value.
macro_rules! rejections {
  ($($width:ty),+ $(,)?) => {$(
    rejects_other_tokens::<Uint<$width>>();
    rejects_other_tokens::<Int<$width>>();
    rejects_other_json::<Uint<$width>>();
    rejects_other_json::<Int<$width>>();
    truncated_postcard_passes_through(&Uint::<$width>::max_value());
    truncated_postcard_passes_through(&Int::<$width>::min_value());
  )+};
}

/// The rejections at the widths read through `deserialize_any` in a human-readable format.
#[inline(never)]
fn rejections_read_with_any() {
  rejections!(U0, U1, U7, U8, U9, U16, U17, U32, U33, U64, U129, U256);
}

/// The rejections at 65 to 128 bits, read through the request of a 128-bit integer.
#[inline(never)]
fn rejections_of_the_128_bit_class() {
  rejects_other_tokens::<Uint<U65>>();
  rejects_other_tokens::<Int<U65>>();
  rejects_other_tokens::<Uint<U128>>();
  rejects_other_tokens::<Int<U128>>();
  rejects_json_as::<Uint<U65>, u128>(false);
  rejects_json_as::<Int<U65>, i128>(true);
  rejects_json_as::<Uint<U100, [u64; 2]>, u128>(false);
  rejects_json_as::<Int<U128, [u8; 16]>, i128>(true);
  truncated_postcard_passes_through(&Uint::<U100>::MAX);
  truncated_postcard_passes_through(&Int::<U128>::MIN);
}

/// Every input that is neither an integer nor a string gets exactly the fixed message, which names
/// nothing of it, with the category of the data in JSON, and a string that does not parse the text
/// of std's parser; the errors that serde_json builds for the request of a 128-bit integer without
/// the visitor pass through as they do for the native integer, and so does postcard's error of an
/// input that ends early.
#[test]
fn every_other_input_gets_the_fixed_message() {
  rejections_read_with_any();
  rejections_of_the_128_bit_class();
}

/// The patterns of `bits` bits, above 128, at which the text is checked: zero, one, every bit,
/// every bit but the lowest, the top bit and every bit below it, the limits of `Uint` and `Int`;
/// and of the patterns of [`wide_patterns`] in `K` words of 64 bits, in an unoptimized build 8 up
/// to 256 bits, and in an optimized one 32 up to 4096 bits and 4 above, as the text of a wide value
/// takes time quadratic in its limbs.
fn text_patterns<const K: usize>(bits: u32) -> Vec<Vec<bool>> {
  let mut patterns = vec![
    model::pattern(bits, |_| false),
    model::pattern(bits, |i| i == 0),
    model::pattern(bits, |_| true),
    model::pattern(bits, |i| i > 0),
    model::pattern(bits, |i| i + 1 == bits),
    model::pattern(bits, |i| i + 1 < bits),
  ];
  let count = match (tier(), bits) {
    (Tier::Full, ..=4096) => 32,
    (Tier::Full, _) => 4,
    (Tier::Debug, ..=256) => 8,
    _ => 0,
  };
  patterns.extend(spread(&wide_patterns::<K>(bits), count));
  patterns.sort_unstable();
  patterns.dedup();
  patterns
}

/// Checks that `T`, above 128 bits, writes the value of each pattern of `patterns` as its decimal
/// text, a JSON string and in postcard the length and the text, as a string is written, and reads
/// it back from both: through `deserialize_any` in JSON and `deserialize_str` in postcard.
#[inline(never)]
fn written_as_text<T>(patterns: &[Vec<bool>])
where
  T: Patterned + Serialize + DeserializeOwned + Display,
{
  let name = type_name::<T>();
  for p in patterns {
    let value = T::from_pattern(p);
    let text = value.to_string();
    let json = serde_json::to_string(&value).unwrap();
    assert_eq!(json, format!("\"{text}\""), "{name}");
    let bytes = postcard::to_stdvec(&value).unwrap();
    assert_eq!(
      bytes,
      postcard::to_stdvec(&text).unwrap(),
      "{text} in {name}"
    );
    assert_eq!(
      serde_json::from_str::<T>(&json).unwrap(),
      value,
      "{text} in {name}"
    );
    assert_eq!(
      postcard::from_bytes::<T>(&bytes).unwrap(),
      value,
      "{text} in {name}"
    );
  }
}

/// Checks the text of `Uint` and `Int` of `$bits` bits, the width `$width`, above 128 bits, with
/// [`written_as_text`] in each pair of storages of `$storages`, written `unsigned, signed;`, at the
/// patterns of [`text_patterns`] in `$words` words of 64 bits.
macro_rules! texts_agree {
  ($width:ty, $bits:literal, $words:literal; $($uint:ty, $int:ty);+ $(;)?) => {{
    let patterns = text_patterns::<$words>($bits);
    $(
      written_as_text::<Uint<$width, $uint>>(&patterns);
      written_as_text::<Int<$width, $int>>(&patterns);
    )+
  }};
}

/// The text of 129, 192, 200 and 256 bits in limbs of 64, 8, 32 and 128 bits.
#[inline(never)]
fn texts_of_129_to_256_bits() {
  texts_agree!(U129, 129, 3; [u64; 3], [u64; 3]; [u8; 17], [u8; 17]);
  texts_agree!(U129, 129, 3; [u32; 5], [u32; 5]; [u128; 2], [u128; 2]);
  texts_agree!(U192, 192, 3; [u64; 3], [u64; 3]; [u8; 24], [u8; 24]);
  texts_agree!(U192, 192, 3; [u32; 6], [u32; 6]; [u128; 2], [u128; 2]);
  texts_agree!(U200, 200, 4; [u64; 4], [u64; 4]; [u8; 25], [u8; 25]);
  texts_agree!(U200, 200, 4; [u32; 7], [u32; 7]; [u128; 2], [u128; 2]);
  texts_agree!(U256, 256, 4; [u64; 4], [u64; 4]; [u8; 32], [u8; 32]);
  texts_agree!(U256, 256, 4; [u32; 8], [u32; 8]; [u128; 2], [u128; 2]);
}

/// The text of 1024 and 4096 bits in limbs of 64, 8, 32 and 128 bits.
#[inline(never)]
fn texts_of_1024_and_4096_bits() {
  texts_agree!(U1024, 1024, 16; [u64; 16], [u64; 16]; [u8; 128], [u8; 128]);
  texts_agree!(U1024, 1024, 16; [u32; 32], [u32; 32]; [u128; 8], [u128; 8]);
  texts_agree!(U4096, 4096, 64; [u64; 64], [u64; 64]; [u8; 512], [u8; 512]);
  texts_agree!(U4096, 4096, 64; [u32; 128], [u32; 128]; [u128; 32], [u128; 32]);
}

/// The text of 8192, 16384 and 65536 bits in the default storage, in an optimized build.
#[inline(never)]
fn texts_of_the_widest_widths() {
  texts_agree!(U8192, 8192, 128; [u64; 128], [u64; 128]);
  texts_agree!(U16384, 16384, 256; [u64; 256], [u64; 256]);
  texts_agree!(U65536, 65536, 1024; [u64; 1024], [u64; 1024]);
}

/// Asserts that `T`, above 128 bits, reads the texts of its limits from serde's deserializer of a
/// string, and rejects each limit followed by a zero with the text of std's parser: too large, and
/// too small below the smallest value of `Int`.
#[inline(never)]
fn reads_the_texts_of_its_limits<T>(signed: bool)
where
  T: FixedInteger + DeserializeOwned,
{
  let name = type_name::<T>();
  let read = |text: &str| {
    T::deserialize(text.into_deserializer()).map_err(|error: de::value::Error| error.to_string())
  };
  let (max, min) = (T::max_value(), T::min_value());
  let (max_text, min_text) = (max.to_string(), min.to_string());
  assert_eq!(read(&max_text), Ok(max), "{name}");
  assert_eq!(read(&min_text), Ok(min), "{name}");
  let too_large = parse_error::<u8>("256");
  assert_eq!(read(&format!("{max_text}0")), Err(too_large), "{name}");
  if signed {
    let too_small = parse_error::<i8>("-129");
    assert_eq!(read(&format!("{min_text}0")), Err(too_small), "{name}");
  }
}

/// Above 128 bits, a value is written as its decimal text, a JSON string and in postcard the length
/// and the text, and reads back from both, at 129, 192, 200, 256, 1024 and 4096 bits in limbs of
/// each type, and in an optimized build also at 8192, 16384 and 65536 bits. The texts of the limits
/// read, and beyond them std's texts of the parser.
#[test]
fn above_128_bits_a_value_is_written_as_its_decimal_text() {
  texts_of_129_to_256_bits();
  texts_of_1024_and_4096_bits();
  if tier().is_full() {
    texts_of_the_widest_widths();
  }
  reads_the_texts_of_its_limits::<u256>(false);
  reads_the_texts_of_its_limits::<i256>(true);
  reads_the_texts_of_its_limits::<Uint<U129, [u8; 17]>>(false);
  reads_the_texts_of_its_limits::<Int<U4096, [u32; 128]>>(true);
}

/// Above 128 bits, a JSON number reads within the 64-bit range, and one beyond it, which serde_json
/// gives as a float, gets the fixed message, as a float does: `2^64` and the 78 digits of
/// `u256::MAX`. serde_test's 64-bit tokens read in both configurations, and so do the integers that
/// serde's deserializers of the 128-bit integers hand over.
#[test]
fn above_128_bits_a_number_reads_as_the_format_gives_it() {
  assert_eq!(
    serde_json::from_str::<u256>("18446744073709551615").unwrap(),
    u256::from(u64::MAX)
  );
  assert_eq!(
    serde_json::from_str::<i256>("-9223372036854775808").unwrap(),
    i256::from(i64::MIN)
  );
  for json in [
    "18446744073709551616".to_string(),
    u256::MAX.to_string(),
    "1e300".to_string(),
  ] {
    let error = serde_json::from_str::<u256>(&json).unwrap_err();
    assert_eq!(json_message(&error), FIXED, "{json}");
    assert!(!error.to_string().contains(&json), "{json}");
  }
  assert_de_tokens(&u256::from(u64::MAX).readable(), &[Token::U64(u64::MAX)]);
  assert_de_tokens(&u256::from(u64::MAX).compact(), &[Token::U64(u64::MAX)]);
  assert_de_tokens(&i256::from(i64::MIN).readable(), &[Token::I64(i64::MIN)]);
  assert_de_tokens(&i256::from(i64::MIN).compact(), &[Token::I64(i64::MIN)]);
  let max = u256::deserialize(u128::MAX.into_deserializer());
  let max: Result<u256, de::value::Error> = max;
  assert_eq!(max.unwrap(), u256::from(u128::MAX));
  let min = i256::deserialize(i128::MIN.into_deserializer());
  let min: Result<i256, de::value::Error> = min;
  assert_eq!(min.unwrap(), i256::from(i128::MIN));
}

/// The key of serde_json's number token.
const NUMBER_TOKEN: &str = "$serde_json::private::Number";

/// What `T` reads from serde's deserializer of the map of `entries`, which hands the map to the
/// visitor, with the text of its error.
#[inline(never)]
fn read_map<T: DeserializeOwned>(entries: &[(&'static str, &'static str)]) -> Result<T, String> {
  let map = de::value::MapDeserializer::new(entries.iter().copied());
  T::deserialize(map).map_err(|error: de::value::Error| error.to_string())
}

/// Asserts that `T` reads `expected`, a value or the message of its error, from the map of
/// `entries`, each a key and a value, as a JSON object of strings and through serde's deserializer
/// of a map, which hand the visitor the same map.
#[inline(never)]
fn reads_the_map<T>(entries: &[(&'static str, &'static str)], expected: Result<T, String>)
where
  T: DeserializeOwned + PartialEq + Debug,
{
  let json: Vec<String> = entries
    .iter()
    .map(|(key, value)| format!("\"{key}\":\"{value}\""))
    .collect();
  let json = format!("{{{}}}", json.join(","));
  let from_json = serde_json::from_str::<T>(&json).map_err(|error| json_message(&error));
  assert_eq!(from_json, expected, "{json}");
  assert_eq!(read_map::<T>(entries), expected, "{entries:?}");
}

/// serde_json's number token, through which serde_json with `arbitrary_precision` hands over a
/// number outside the 64-bit range, is tested without the feature: an object of one entry whose key
/// is the token and whose value is the text of an integer of JSON's grammar reads as the integer,
/// as JSON, through serde's deserializer of a map and through serde_test, also from 65 to 128 bits
/// where a format gives a map to the request of a 128-bit integer. Any other text, `-0` included,
/// another key, a second entry, no entry and a number as the value get the fixed message, and a
/// negative integer into an unsigned type is too small, as it is as a number. The string `"-0"`
/// still reads as zero into `Int`, and so does the number `-0` from 65 to 128 bits, as for `i128`.
#[test]
fn serde_json_number_token() {
  let wide = "12345678901234567890123456789";
  reads_the_map::<u256>(&[(NUMBER_TOKEN, wide)], Ok(wide.parse().unwrap()));
  reads_the_map::<u256>(&[(NUMBER_TOKEN, "5")], Ok(u256::from(5u8)));
  reads_the_map::<i256>(&[(NUMBER_TOKEN, "-5")], Ok(i256::from(-5i8)));
  reads_the_map::<u7>(&[(NUMBER_TOKEN, "100")], Ok(u7::new(100)));
  reads_the_map::<u7>(&[(NUMBER_TOKEN, "128")], Err(parse_error::<u8>("256")));
  let too_small = parse_error::<i8>("-129");
  reads_the_map::<u256>(&[(NUMBER_TOKEN, "-5")], Err(too_small.clone()));
  reads_the_map::<u7>(&[(NUMBER_TOKEN, "-5")], Err(too_small));
  let fixed = FIXED.to_string();
  for text in ["-0", "1.5", "1e2", "007", "+5", "", "-", " 5", "0x5"] {
    reads_the_map::<u256>(&[(NUMBER_TOKEN, text)], Err(fixed.clone()));
    reads_the_map::<i256>(&[(NUMBER_TOKEN, text)], Err(fixed.clone()));
  }
  reads_the_map::<u256>(&[(NUMBER_TOKEN, "5"), ("a", "1")], Err(fixed.clone()));
  let twice = [(NUMBER_TOKEN, "5"), (NUMBER_TOKEN, "5")];
  reads_the_map::<u256>(&twice, Err(fixed.clone()));
  reads_the_map::<u256>(&[("a", "5")], Err(fixed.clone()));
  reads_the_map::<u256>(&[], Err(fixed));
  let json = format!(r#"{{"{NUMBER_TOKEN}":5}}"#);
  let error = serde_json::from_str::<u256>(&json).unwrap_err();
  assert_eq!(json_message(&error), FIXED);
  let map = de::value::MapDeserializer::new([(NUMBER_TOKEN, 5u64)].into_iter());
  let error: de::value::Error = u256::deserialize(map).unwrap_err();
  assert_eq!(error.to_string(), FIXED);
  let read = read_map::<Uint<U100>>(&[(NUMBER_TOKEN, wide)]);
  assert_eq!(read, Ok(wide.parse().unwrap()));
  let tokens = [
    Token::Map { len: Some(1) },
    Token::Str(NUMBER_TOKEN),
    Token::Str("123"),
    Token::MapEnd,
  ];
  assert_de_tokens(&u256::from(123u8).readable(), &tokens);
  assert_de_tokens(&u256::from(123u8).compact(), &tokens);
  assert_de_tokens(&u7::new(123).readable(), &tokens);
  assert_de_tokens(&Uint::<U100>::new(123).compact(), &tokens);
  assert_eq!(serde_json::from_str::<i7>("\"-0\"").unwrap(), i7::ZERO);
  assert_eq!(serde_json::from_str::<i256>("\"-0\"").unwrap(), i256::ZERO);
  assert_eq!(serde_json::from_str::<i128>("-0").unwrap(), 0);
  let zero = serde_json::from_str::<Int<U100>>("-0").unwrap();
  assert_eq!(zero, Int::<U100>::ZERO);
}

/// A stand-in of one field of csv 1.4, which reads a field by its text: `deserialize_any` infers a
/// `bool`, a `u64`, an `i64`, a `u128`, an `i128`, an `f64`, and only then a string; the requests
/// of the integers take hexadecimal digits after `0x`, which the inference does not; and an empty
/// field is `None`.
struct CsvField<'de>(&'de str);

/// Implements the requests of the integers of a [`CsvField`], written `method: visit(native)`:
/// hexadecimal digits after `0x`, and decimal ones otherwise.
macro_rules! csv_integer_requests {
  ($($method:ident: $visit:ident($native:ty)),+ $(,)?) => {$(
    fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
      let number = match self.0.strip_prefix("0x") {
        Some(digits) => <$native>::from_str_radix(digits, 16),
        None => self.0.parse(),
      };
      visitor.$visit(number.map_err(de::Error::custom)?)
    }
  )+};
}

impl<'de> Deserializer<'de> for CsvField<'de> {
  type Error = de::value::Error;

  fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
    let text = self.0;
    if let Ok(value) = text.parse() {
      visitor.visit_bool(value)
    } else if let Ok(number) = text.parse() {
      visitor.visit_u64(number)
    } else if let Ok(number) = text.parse() {
      visitor.visit_i64(number)
    } else if let Ok(number) = text.parse() {
      visitor.visit_u128(number)
    } else if let Ok(number) = text.parse() {
      visitor.visit_i128(number)
    } else if let Ok(number) = text.parse() {
      visitor.visit_f64(number)
    } else {
      visitor.visit_borrowed_str(text)
    }
  }

  csv_integer_requests! {
    deserialize_u8: visit_u8(u8),
    deserialize_u16: visit_u16(u16),
    deserialize_u32: visit_u32(u32),
    deserialize_u64: visit_u64(u64),
    deserialize_u128: visit_u128(u128),
    deserialize_i8: visit_i8(i8),
    deserialize_i16: visit_i16(i16),
    deserialize_i32: visit_i32(i32),
    deserialize_i64: visit_i64(i64),
    deserialize_i128: visit_i128(i128),
  }

  fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
    visitor.visit_borrowed_str(self.0)
  }

  fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
    if self.0.is_empty() {
      visitor.visit_none()
    } else {
      visitor.visit_some(self)
    }
  }

  forward_to_deserialize_any! {
    bool f32 f64 char string bytes byte_buf unit unit_struct newtype_struct seq tuple tuple_struct
    map struct enum identifier ignored_any
  }
}

/// The text of the error of `T` from the csv field `text`.
#[inline(never)]
fn csv_error<T: DeserializeOwned + Debug>(text: &str) -> String {
  T::deserialize(CsvField(text)).unwrap_err().to_string()
}

/// csv infers the type of a field from its text, so the default of a type above 128 bits reads a
/// value that a 128-bit integer holds and rejects one beyond, which csv gives as a float, with the
/// fixed message, also the text that numo writes for `u256::MAX`; the request of a 128-bit integer
/// takes the hexadecimal digits after `0x`, which the inference does not; and an empty field is
/// `None`.
#[test]
fn csv_infers_the_type_of_a_field_from_its_text() {
  let one = u256::from(1u8);
  let max = u128::MAX.to_string();
  assert_eq!(
    u256::deserialize(CsvField(&max)).unwrap(),
    u256::from(u128::MAX)
  );
  let power = (u256::from(u128::MAX) + one).to_string();
  assert_eq!(csv_error::<u256>(&power), FIXED);
  assert_eq!(csv_error::<u256>(&u256::MAX.to_string()), FIXED);
  let min = i128::MIN.to_string();
  assert_eq!(
    i256::deserialize(CsvField(&min)).unwrap(),
    i256::from(i128::MIN)
  );
  let below = (i256::from(i128::MIN) - i256::from(1i8)).to_string();
  assert_eq!(csv_error::<i256>(&below), FIXED);
  assert_eq!(
    Uint::<U100>::deserialize(CsvField("0x10")).unwrap(),
    Uint::<U100>::new(16)
  );
  assert_eq!(csv_error::<u7>("0x10"), parse_error::<u8>("0x10"));
  assert_eq!(csv_error::<u256>("0x10"), parse_error::<u8>("0x10"));
  assert_eq!(Option::<u256>::deserialize(CsvField("")).unwrap(), None);
  assert_eq!(
    Option::<u256>::deserialize(CsvField("5")).unwrap(),
    Some(u256::from(5u8))
  );
}

/// In a human-readable format without `deserialize_any`, as serde-json-core, the defaults read
/// nothing at 0 to 64 bits and above 128 bits, where they fail with the error of the format, and
/// fail at 65 to 128 bits with serde's error of the request of a 128-bit integer, as the native
/// `u128` and `i128` do, while the native `u8` reads.
#[test]
fn a_human_readable_format_without_deserialize_any() {
  let without = Probe::WithoutAny;
  for error in [
    error_from::<u7>(without),
    error_from::<Uint<U64>>(without),
    error_from::<i7>(without),
    error_from::<Int<U64>>(without),
    error_from::<u256>(without),
    error_from::<i256>(without),
  ] {
    assert_eq!(error, "any is unsupported");
  }
  assert_eq!(
    error_from::<Uint<U100>>(without),
    error_from::<u128>(without)
  );
  assert_eq!(
    error_from::<Int<U100>>(without),
    error_from::<i128>(without)
  );
  assert_eq!(u8::deserialize(without).unwrap(), 5);
}

/// A struct of one field, which the containers below hold.
#[derive(Debug, serde_full::Deserialize)]
#[serde(crate = "serde_full")]
struct Field<T> {
  /// The integer.
  value: T,
}

/// An untagged enum of a [`Field`], which serde buffers to try each variant.
#[derive(Debug, serde_full::Deserialize)]
#[serde(crate = "serde_full", untagged)]
enum Untagged<T> {
  /// The field.
  Held(Field<T>),
}

/// A struct into which a [`Field`] is flattened, which serde buffers to read the field.
#[derive(Debug, serde_full::Deserialize)]
#[serde(crate = "serde_full")]
struct Flattened<T> {
  /// The field.
  #[serde(flatten)]
  held: Field<T>,
}

/// An internally tagged enum of a [`Field`], which serde buffers to find the tag.
#[derive(Debug, serde_full::Deserialize)]
#[serde(crate = "serde_full", tag = "kind")]
enum Tagged<T> {
  /// The field.
  Held(Field<T>),
}

/// The value that each container of serde's buffer reads from JSON whose field holds `value`, or
/// the text of the error.
#[inline(never)]
fn through_the_buffer<T: DeserializeOwned>(value: &str) -> [Result<T, String>; 3] {
  let field = format!(r#"{{"value":{value}}}"#);
  let tagged = format!(r#"{{"kind":"Held","value":{value}}}"#);
  let text = |error: serde_json::Error| error.to_string();
  [
    serde_json::from_str::<Untagged<T>>(&field)
      .map(|Untagged::Held(field)| field.value)
      .map_err(text),
    serde_json::from_str::<Flattened<T>>(&field)
      .map(|flattened| flattened.held.value)
      .map_err(text),
    serde_json::from_str::<Tagged<T>>(&tagged)
      .map(|Tagged::Held(field)| field.value)
      .map_err(text),
  ]
}

/// Asserts that `T`, of 0 to 64 bits or above 128 bits, reads the number 5 and the string `"5"` in
/// the field of each container of serde's buffer, which replays numbers and strings.
#[inline(never)]
fn reads_through_the_buffer<T>()
where
  T: DeserializeOwned + CheckedCastFrom<u8> + PartialEq + Debug,
{
  for value in ["5", "\"5\""] {
    for result in through_the_buffer::<T>(value) {
      assert_eq!(result, Ok(T::checked_cast_from(5u8).unwrap()), "{value}");
    }
  }
}

/// Asserts that `T`, of 65 to 128 bits, fails in the field of each container of serde's buffer,
/// which holds no 128-bit integer, with the error of a field of its native integer `N`.
#[inline(never)]
fn fails_through_the_buffer_as<T, N>()
where
  T: DeserializeOwned + Debug,
  N: DeserializeOwned + Debug,
{
  for value in ["5", "\"5\""] {
    let results = through_the_buffer::<T>(value).map(|result| result.unwrap_err());
    let natives = through_the_buffer::<N>(value).map(|result| result.unwrap_err());
    assert_eq!(results, natives, "{value}");
  }
}

/// An untagged enum of a number or a string, in which the defaults take a string.
#[derive(Debug, PartialEq, serde_full::Deserialize)]
#[serde(crate = "serde_full", untagged)]
enum NumberOrText<T> {
  /// The number.
  Number(T),
  /// The text.
  Text(String),
}

/// Through serde's buffer, as in untagged enums, flattened structs and internally tagged enums, the
/// defaults read numbers and strings at 0 to 64 bits and above 128 bits, and fail at 65 to 128 bits
/// as a field of `u128` or `i128` does. In an untagged enum of a number or a string, a numo integer
/// takes the string `"5"`, which the native `u8` leaves to the string.
#[test]
fn serde_buffers_untagged_enums_and_flattened_structs() {
  reads_through_the_buffer::<u7>();
  reads_through_the_buffer::<i7>();
  reads_through_the_buffer::<Uint<U16>>();
  reads_through_the_buffer::<Int<U32>>();
  reads_through_the_buffer::<Uint<U64>>();
  reads_through_the_buffer::<u256>();
  reads_through_the_buffer::<Int<U200, [u32; 7]>>();
  fails_through_the_buffer_as::<Uint<U65>, u128>();
  fails_through_the_buffer_as::<Int<U65>, i128>();
  fails_through_the_buffer_as::<Uint<U128>, u128>();
  fails_through_the_buffer_as::<Int<U128>, i128>();
  assert_eq!(
    serde_json::from_str::<NumberOrText<u7>>("\"5\"").unwrap(),
    NumberOrText::Number(u7::new(5))
  );
  assert_eq!(
    serde_json::from_str::<NumberOrText<u8>>("\"5\"").unwrap(),
    NumberOrText::Text("5".to_string())
  );
}

/// Asserts that a JSON map whose keys are values of `T`, of the class whose native integer is `N`,
/// at each of `keys`, writes its keys as a map of `N` does, as strings, and reads back.
#[inline(never)]
fn keys_round_trip<T, N>(keys: &[N])
where
  T: CheckedCastFrom<N> + Serialize + DeserializeOwned + Ord + Debug,
  N: Native + Ord,
{
  let map: BTreeMap<T, u8> = keys
    .iter()
    .map(|&key| (T::checked_cast_from(key).unwrap(), 1))
    .collect();
  let natives: BTreeMap<N, u8> = keys.iter().map(|&key| (key, 1)).collect();
  let json = serde_json::to_string(&map).unwrap();
  assert_eq!(json, serde_json::to_string(&natives).unwrap());
  assert_eq!(serde_json::from_str::<BTreeMap<T, u8>>(&json).unwrap(), map);
}

/// Asserts that a JSON map whose keys are values of `T`, above 128 bits, at the patterns of
/// `patterns`, writes each key as its decimal text and reads back.
#[inline(never)]
fn text_keys_round_trip<T>(patterns: &[Vec<bool>])
where
  T: Patterned + Serialize + DeserializeOwned + Display + Ord,
{
  let map: BTreeMap<T, u8> = patterns.iter().map(|p| (T::from_pattern(p), 1)).collect();
  let texts: BTreeMap<String, u8> = map.keys().map(|key| (key.to_string(), 1)).collect();
  let json = serde_json::to_string(&map).unwrap();
  assert_eq!(
    serde_json::from_str::<BTreeMap<String, u8>>(&json).unwrap(),
    texts
  );
  assert_eq!(serde_json::from_str::<BTreeMap<T, u8>>(&json).unwrap(), map);
}

/// JSON map keys of every class write and read back: serde_json gives a key to `deserialize_any` as
/// a string, which `FromStr` reads at 0 to 64 bits and above 128 bits, and to the request of a
/// 128-bit integer as its digits; so `"005"` and `"+5"` read as keys of `u7`.
#[test]
fn json_map_keys_of_every_class() {
  keys_round_trip::<u7, u8>(&[0, 5, 127]);
  keys_round_trip::<i7, i8>(&[-64, -1, 0, 63]);
  keys_round_trip::<Uint<U16, [u8; 2]>, u16>(&[0, u16::MAX]);
  keys_round_trip::<Int<U32>, i32>(&[i32::MIN, i32::MAX]);
  keys_round_trip::<Uint<U64>, u64>(&[0, u64::MAX]);
  keys_round_trip::<Int<U100>, i128>(&[-(1 << 99), 5, (1 << 99) - 1]);
  keys_round_trip::<Uint<U128, [u64; 2]>, u128>(&[0, u128::MAX]);
  text_keys_round_trip::<u256>(&text_patterns::<4>(256));
  text_keys_round_trip::<Int<U129, [u8; 17]>>(&text_patterns::<3>(129));
  for json in [r#"{"005":1}"#, r#"{"+5":1}"#] {
    let keys = serde_json::from_str::<BTreeMap<u7, u8>>(json).unwrap();
    assert_eq!(
      keys.into_iter().collect::<Vec<_>>(),
      [(u7::new(5), 1)],
      "{json}"
    );
  }
}

/// Asserts that `T`, of zero bits, writes zero as the `u8` or `i8` zero is written, reads the
/// number 0 and the spellings of zero, `"0"`, `"00"`, `"+0"`, and `"-0"` if `signed`, and nothing
/// else: one and minus one, as numbers and as texts, get std's texts.
#[inline(never)]
fn zero_bits<T>(signed: bool)
where
  T: Serialize + DeserializeOwned + Default + PartialEq + Debug,
{
  let name = type_name::<T>();
  let zero = T::default();
  assert_eq!(serde_json::to_string(&zero).unwrap(), "0", "{name}");
  let bytes = postcard::to_stdvec(&zero).unwrap();
  assert_eq!(bytes, postcard::to_stdvec(&0u8).unwrap(), "{name}");
  assert_eq!(postcard::from_bytes::<T>(&bytes).unwrap(), zero, "{name}");
  for json in ["0", "\"0\"", "\"00\"", "\"+0\""] {
    assert_eq!(
      serde_json::from_str::<T>(json).unwrap(),
      zero,
      "{json} into {name}"
    );
  }
  let minus_zero = serde_json::from_str::<T>("\"-0\"");
  if signed {
    assert_eq!(minus_zero.unwrap(), zero, "{name}");
  } else {
    let message = json_message(&minus_zero.unwrap_err());
    assert_eq!(message, parse_error::<u8>("-0"), "{name}");
  }
  let message = |json: &str| json_message(&serde_json::from_str::<T>(json).unwrap_err());
  assert_eq!(message("1"), too_large(), "{name}");
  assert_eq!(message("-1"), too_small(), "{name}");
  assert_eq!(message("\"1\""), parse_error::<u8>("256"), "{name}");
  let minus_one = if signed {
    parse_error::<i8>("-129")
  } else {
    parse_error::<u8>("-1")
  };
  assert_eq!(message("\"-1\""), minus_one, "{name}");
}

/// Asserts that `T`, of one signed bit, writes -1 and 0 as the `i8` of the same value is written,
/// reads them back, and the texts `"-1"`, `"-0"` and `"+0"`, and rejects 1 and -2, as numbers and
/// as texts, with std's texts.
#[inline(never)]
fn one_signed_bit<T>()
where
  T: Serialize + DeserializeOwned + CheckedCastFrom<i8> + PartialEq + Debug,
{
  let name = type_name::<T>();
  for native in [-1i8, 0] {
    let value = T::checked_cast_from(native).unwrap();
    let json = serde_json::to_string(&native).unwrap();
    let bytes = postcard::to_stdvec(&native).unwrap();
    assert_eq!(serde_json::to_string(&value).unwrap(), json, "{name}");
    assert_eq!(postcard::to_stdvec(&value).unwrap(), bytes, "{name}");
    assert_eq!(serde_json::from_str::<T>(&json).unwrap(), value, "{name}");
    assert_eq!(postcard::from_bytes::<T>(&bytes).unwrap(), value, "{name}");
  }
  let (minus_one, zero) = (
    T::checked_cast_from(-1i8).unwrap(),
    T::checked_cast_from(0i8).unwrap(),
  );
  assert_eq!(
    serde_json::from_str::<T>("\"-1\"").unwrap(),
    minus_one,
    "{name}"
  );
  assert_eq!(serde_json::from_str::<T>("\"-0\"").unwrap(), zero, "{name}");
  assert_eq!(serde_json::from_str::<T>("\"+0\"").unwrap(), zero, "{name}");
  let message = |json: &str| json_message(&serde_json::from_str::<T>(json).unwrap_err());
  assert_eq!(message("1"), too_large(), "{name}");
  assert_eq!(message("-2"), too_small(), "{name}");
  assert_eq!(message("\"1\""), parse_error::<u8>("256"), "{name}");
  assert_eq!(message("\"-2\""), parse_error::<i8>("-129"), "{name}");
}

/// Zero bits are written as the zero of `u8` or `i8`, in `[u64; 0]`, `u8` and `[u8; 1]`, and read
/// zero and nothing else; one signed bit is written as the `i8` -1 or 0, `[255]` or `[0]` in
/// postcard, and reads -1 and 0 only.
#[test]
fn zero_bits_and_one_signed_bit() {
  zero_bits::<u0>(false);
  zero_bits::<Uint<U0, u8>>(false);
  zero_bits::<Uint<U0, [u8; 1]>>(false);
  zero_bits::<i0>(true);
  zero_bits::<Int<U0, i8>>(true);
  zero_bits::<Int<U0, [u8; 1]>>(true);
  one_signed_bit::<i1>();
  one_signed_bit::<Int<U1, [u64; 1]>>();
  assert_eq!(postcard::to_stdvec(&i1::new(-1)).unwrap(), [255]);
}

/// The JSON text and the postcard bytes of `value`, and the values read back from them, in code
/// that knows only the storage traits of `Uint`.
#[inline(never)]
fn forms_over_uint_storage<B, R>(value: &Uint<B, R>) -> (String, Vec<u8>, Uint<B, R>, Uint<B, R>)
where
  B: FitsBits<R>,
  R: UintRepr,
{
  let json = serde_json::to_string(value).unwrap();
  let bytes = postcard::to_stdvec(value).unwrap();
  let from_json = serde_json::from_str(&json).unwrap();
  let from_bytes = postcard::from_bytes(&bytes).unwrap();
  (json, bytes, from_json, from_bytes)
}

/// The JSON text and the postcard bytes of `value`, and the values read back from them, in code
/// that knows only the storage traits of `Int`.
#[inline(never)]
fn forms_over_int_storage<B, R>(value: &Int<B, R>) -> (String, Vec<u8>, Int<B, R>, Int<B, R>)
where
  B: FitsBits<R>,
  R: IntRepr,
{
  let json = serde_json::to_string(value).unwrap();
  let bytes = postcard::to_stdvec(value).unwrap();
  let from_json = serde_json::from_str(&json).unwrap();
  let from_bytes = postcard::from_bytes(&bytes).unwrap();
  (json, bytes, from_json, from_bytes)
}

/// The JSON texts of the largest `Uint<B>` and of the smallest `Int<B>`, which read back as the
/// same values, in code that knows only that `B` is a width.
#[inline(never)]
fn texts_over_the_width<B: BitWidth>() -> [String; 2] {
  let (max, min) = (Uint::<B>::max_value(), Int::<B>::min_value());
  let texts = [
    serde_json::to_string(&max).unwrap(),
    serde_json::to_string(&min).unwrap(),
  ];
  assert_eq!(serde_json::from_str::<Uint<B>>(&texts[0]).unwrap(), max);
  assert_eq!(serde_json::from_str::<Int<B>>(&texts[1]).unwrap(), min);
  texts
}

/// Checks [`texts_over_the_width`] at each width `$width`, against the texts of the concrete types.
macro_rules! texts_over_the_width {
  ($($width:ty),+ $(,)?) => {$(
    assert_eq!(
      texts_over_the_width::<$width>(),
      [
        serde_json::to_string(&Uint::<$width>::MAX).unwrap(),
        serde_json::to_string(&Int::<$width>::MIN).unwrap(),
      ]
    );
  )+};
}

/// The impls need only the bounds of the storage traits, `B: FitsBits<R>` with `R: UintRepr` or
/// `R: IntRepr`, and the default storages only `B: BitWidth`: code generic over them writes and
/// reads what the concrete types do.
#[test]
fn code_generic_over_the_storage_traits_or_the_width() {
  let (json, bytes, from_json, from_bytes) = forms_over_uint_storage(&u7::new(100));
  assert_eq!((json.as_str(), bytes.as_slice()), ("100", &[100][..]));
  assert_eq!((from_json, from_bytes), (u7::new(100), u7::new(100)));
  let value = Uint::<U100, [u8; 13]>::MAX;
  let (json, bytes, from_json, from_bytes) = forms_over_uint_storage(&value);
  assert_eq!(json, serde_json::to_string(&(u128::MAX >> 28)).unwrap());
  assert_eq!(bytes, postcard::to_stdvec(&(u128::MAX >> 28)).unwrap());
  assert_eq!((from_json, from_bytes), (value, value));
  let value = Int::<U12, [u8; 2]>::MIN;
  let (json, bytes, from_json, from_bytes) = forms_over_int_storage(&value);
  assert_eq!(json, serde_json::to_string(&-2048i16).unwrap());
  assert_eq!(bytes, postcard::to_stdvec(&-2048i16).unwrap());
  assert_eq!((from_json, from_bytes), (value, value));
  let (json, bytes, from_json, from_bytes) = forms_over_int_storage(&i256::MIN);
  assert_eq!(json, format!("\"{}\"", i256::MIN));
  assert_eq!(bytes, postcard::to_stdvec(&i256::MIN.to_string()).unwrap());
  assert_eq!((from_json, from_bytes), (i256::MIN, i256::MIN));
  texts_over_the_width!(U0, U1, U7, U64, U65, U100, U128, U129, U200, U1024);
}
