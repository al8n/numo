//! The helper modules of `numo::serde`. `decimal` writes the decimal text at every width, in every
//! family of storages, of the native integers and of the non-zero integers, as a string in JSON and
//! postcard, and reads it back; it reads an integer that a format gives for the request of a
//! string, checked against the width, reads where the defaults do not, and rejects every other
//! input with its fixed message. `quantity` writes the QUANTITY of the unsigned integers and reads
//! it as go-ethereum's `hexutil` decodes a number, with the message of each rule of its grammar
//! and the range checked without forming the value. The leaf `optional` of each helper writes and
//! reads `None` and `Some` in JSON and postcard, a missing field with and without `default`, `null`
//! through serde's buffer and an empty csv field; and each helper reads in untagged enums and
//! flattened structs.
//!
//! Every expected text of std comes from std at run time. Each width group or family of storages
//! checks in a function of its own, and each generic check is `#[inline(never)]`, so that a frame
//! holds the values of one type alone.

use core::{
  any::type_name,
  fmt::{Display, LowerHex},
  num::NonZero,
};

use serde_core::{
  Deserialize, Serialize,
  de::{self, DeserializeOwned, IntoDeserializer},
};
use serde_json::error::Category;
use serde_test::{Token, assert_de_tokens, assert_de_tokens_error, assert_tokens};
use typenum::{U4, U12, U15, U17, U31, U33, U192, U8192, U16384, U65536};

use super::{
  serde::{
    CsvField, NUMBER_TOKEN, Native, json_message, parse_error, samples, text_patterns, too_large,
    too_small, zero_of_conversion,
  },
  *,
};
use crate::{
  CheckedCastFrom, NonZeroInt, NonZeroUint,
  serde::{
    decimal::{self, DecimalString},
    quantity::{self, Quantity},
  },
  u0,
};

/// The fixed message of `decimal`.
const DECIMAL: &str = "expected an integer as a decimal string";

/// Defines in the module `$module` the containers of the helper module whose trait is `$trait`,
/// for a `T` of that trait: `Required`, a newtype of a value with the leaf `$required`, and
/// `Optional`, a newtype of an `Option` with the leaf `$optional`, which JSON and postcard write as
/// their content; `Field`, a struct of a field with `$required`; and `Defaulted` and `Present`,
/// structs of an `Option` field with `$optional`, with and without `#[serde(default)]`.
macro_rules! containers {
  ($module:ident: $trait:ident, $required:literal, $optional:literal) => {
    mod $module {
      use super::$trait;

      /// A value, written and read with the leaf `required`.
      #[derive(Debug, PartialEq, serde_full::Serialize, serde_full::Deserialize)]
      #[serde(crate = "serde_full")]
      pub(super) struct Required<T: $trait>(#[serde(with = $required)] pub(super) T);

      /// An `Option`, written and read with the leaf `optional`.
      #[derive(Debug, PartialEq, serde_full::Serialize, serde_full::Deserialize)]
      #[serde(crate = "serde_full")]
      pub(super) struct Optional<T: $trait>(#[serde(with = $optional)] pub(super) Option<T>);

      /// A struct of a field with the leaf `required`.
      #[derive(Debug, PartialEq, serde_full::Deserialize)]
      #[serde(crate = "serde_full")]
      pub(super) struct Field<T: $trait> {
        /// The value.
        #[serde(with = $required)]
        pub(super) value: T,
      }

      /// A struct of an `Option` field with the leaf `optional` and `default`, which reads a
      /// missing field as `None`.
      #[derive(Debug, PartialEq, serde_full::Deserialize)]
      #[serde(crate = "serde_full")]
      pub(super) struct Defaulted<T: $trait> {
        /// The value.
        #[serde(default, with = $optional)]
        pub(super) value: Option<T>,
      }

      /// A struct of an `Option` field with the leaf `optional` and without `default`, which
      /// requires the field.
      #[derive(Debug, PartialEq, serde_full::Deserialize)]
      #[serde(crate = "serde_full")]
      pub(super) struct Present<T: $trait> {
        /// The value.
        #[serde(with = $optional)]
        pub(super) value: Option<T>,
      }
    }
  };
}

containers!(
  with_decimal: DecimalString,
  "crate::serde::decimal::required",
  "crate::serde::decimal::optional"
);

containers!(
  with_quantity: Quantity,
  "crate::serde::quantity::required",
  "crate::serde::quantity::optional"
);

/// An untagged enum of `C`, which serde buffers to try each variant.
#[derive(Debug, PartialEq, serde_full::Deserialize)]
#[serde(crate = "serde_full", untagged)]
enum Untagged<C> {
  /// The content.
  Held(C),
}

/// A struct into which `C` is flattened, which serde buffers to read the fields of `C`.
#[derive(Debug, PartialEq, serde_full::Deserialize)]
#[serde(crate = "serde_full")]
struct Flattened<C> {
  /// The content.
  #[serde(flatten)]
  held: C,
}

/// What `C` reads from `json` through each container of serde's buffer, an untagged enum and a
/// flattened struct, or the text of the error.
#[inline(never)]
fn through_the_buffer<C: DeserializeOwned>(json: &str) -> [Result<C, String>; 2] {
  let text = |error: serde_json::Error| error.to_string();
  [
    serde_json::from_str::<Untagged<C>>(json)
      .map(|Untagged::Held(held)| held)
      .map_err(text),
    serde_json::from_str::<Flattened<C>>(json)
      .map(|flattened| flattened.held)
      .map_err(text),
  ]
}

/// Asserts that `O`, a newtype of an `Option` with the leaf `optional` of a helper, writes `none`
/// as JSON's `null` and postcard's `[0]`, and `some` as the JSON text `json` and in postcard as
/// `[1]` followed by `content`, and reads both back.
#[inline(never)]
fn options_round_trip<O>(none: O, some: O, json: &str, content: &[u8])
where
  O: Serialize + DeserializeOwned + PartialEq + Debug,
{
  let name = type_name::<O>();
  let some_bytes = [&[1][..], content].concat();
  for (value, json, bytes) in [(none, "null", &[0][..]), (some, json, &some_bytes)] {
    assert_eq!(serde_json::to_string(&value).unwrap(), json, "{name}");
    assert_eq!(
      postcard::to_stdvec(&value).unwrap(),
      bytes,
      "{json} in {name}"
    );
    assert_eq!(serde_json::from_str::<O>(json).unwrap(), value, "{name}");
    assert_eq!(
      postcard::from_bytes::<O>(bytes).unwrap(),
      value,
      "{json} in {name}"
    );
  }
}

/// Asserts that `D`, a struct of an `Option` field with the leaf `optional` of a helper and
/// `default`, reads `defaulted[0]` from a missing field and from `null`, and `defaulted[1]` from
/// the JSON value `json` in the field, also through each container of serde's buffer, which gives
/// `null` as a unit; and that `P`, the same without `default`, reports a missing field and reads
/// `present` from `null` and from `json`.
#[inline(never)]
fn option_fields<D, P>(defaulted: [D; 2], present: [P; 2], json: &str)
where
  D: DeserializeOwned + PartialEq + Debug,
  P: DeserializeOwned + PartialEq + Debug,
{
  let name = type_name::<D>();
  let value = format!(r#"{{"value":{json}}}"#);
  let [none, some] = defaulted;
  assert_eq!(serde_json::from_str::<D>("{}").unwrap(), none, "{name}");
  for (json, expected) in [(r#"{"value":null}"#, &none), (value.as_str(), &some)] {
    assert_eq!(
      &serde_json::from_str::<D>(json).unwrap(),
      expected,
      "{json} into {name}"
    );
    for result in through_the_buffer::<D>(json) {
      assert_eq!(result.as_ref(), Ok(expected), "{json} into {name}");
    }
  }
  let error = serde_json::from_str::<P>("{}").unwrap_err();
  assert_eq!(json_message(&error), "missing field `value`", "{name}");
  let [none, some] = present;
  let null = serde_json::from_str::<P>(r#"{"value":null}"#).unwrap();
  assert_eq!(null, none, "{name}");
  assert_eq!(
    serde_json::from_str::<P>(&value).unwrap(),
    some,
    "{json} into {name}"
  );
}

/// The values of `bits` bits, at most 128, at which the helpers are checked, as their two's
/// complement in 128 bits: those of `samples`, but in an optimized build, where `samples` also
/// takes every value up to 8 bits and 64 patterns above, only its limits, -1, 0 and 1 and about 16
/// of its values, spread evenly, which keeps the helpers within their share of the tier.
fn helper_samples(bits: u32, signed: bool) -> Vec<u128> {
  let samples = samples(bits, signed);
  if !tier().is_full() {
    return samples;
  }
  let (min, max) = match (bits, signed) {
    (0, _) => (0, 0),
    (_, false) => (0, u128::MAX >> (128 - bits)),
    (_, true) => (
      (i128::MIN >> (128 - bits)) as u128,
      (i128::MAX >> (128 - bits)) as u128,
    ),
  };
  let step = samples.len().div_ceil(16);
  samples
    .iter()
    .enumerate()
    .filter(|&(index, pattern)| index % step == 0 || [min, max, 0, 1, u128::MAX].contains(pattern))
    .map(|(_, &pattern)| pattern)
    .collect()
}

/// The values of [`helper_samples`] as the native integer `N` of their class.
fn helper_natives<N: Native>(bits: u32, signed: bool) -> Vec<N> {
  helper_samples(bits, signed)
    .into_iter()
    .map(N::of_pattern)
    .collect()
}

/// The values of [`helper_samples`] but zero, as the native integer `N` of their class.
fn helper_nonzero_natives<N: Native>(bits: u32, signed: bool) -> Vec<N> {
  helper_samples(bits, signed)
    .into_iter()
    .filter(|&pattern| pattern != 0)
    .map(N::of_pattern)
    .collect()
}

/// Checks that `T` writes `value` with `decimal` as its decimal text, a JSON string and in postcard
/// the length and the text, as a string is written, and reads it back from both.
#[inline(never)]
fn decimal_round_trip<T>(value: T)
where
  T: DecimalString + Display + PartialEq + Debug,
{
  let name = type_name::<T>();
  let text = value.to_string();
  let value = with_decimal::Required(value);
  let json = serde_json::to_string(&value).unwrap();
  assert_eq!(json, format!("\"{text}\""), "{name}");
  let bytes = postcard::to_stdvec(&value).unwrap();
  assert_eq!(
    bytes,
    postcard::to_stdvec(&text).unwrap(),
    "{text} in {name}"
  );
  let from_json = serde_json::from_str::<with_decimal::Required<T>>(&json).unwrap();
  assert_eq!(from_json, value, "{text} in {name}");
  let from_bytes = postcard::from_bytes::<with_decimal::Required<T>>(&bytes).unwrap();
  assert_eq!(from_bytes, value, "{text} in {name}");
}

/// Checks [`decimal_round_trip`] at the value in `T` of each of `natives`, which `T` holds.
#[inline(never)]
fn decimal_round_trips<T, N>(natives: &[N])
where
  T: CheckedCastFrom<N> + DecimalString + Display + PartialEq + Debug,
  N: Native,
{
  for &native in natives {
    decimal_round_trip(T::checked_cast_from(native).expect("a value of the width"));
  }
}

/// Checks [`decimal_round_trips`] at `Uint` and `Int` of `$bits` bits, the width `$width`, whose
/// class has the native integers `$unsigned` and `$signed`, in each pair of storages of
/// `$storages`, written `unsigned, signed;`, at the values of [`helper_natives`].
macro_rules! decimals {
  ($width:ty, $bits:literal, $unsigned:ident, $signed:ident; $($uint:ty, $int:ty);+ $(;)?) => {{
    let unsigned: Vec<$unsigned> = helper_natives($bits, false);
    let signed: Vec<$signed> = helper_natives($bits, true);
    $(
      decimal_round_trips::<Uint<$width, $uint>, $unsigned>(&unsigned);
      decimal_round_trips::<Int<$width, $int>, $signed>(&signed);
    )+
  }};
}

/// The decimal text of the class of `u8` and `i8`, in the storages in which the defaults are
/// checked: zero bits in `[u64; 0]`, a native integer and `[u8; 1]`; one bit in a native integer,
/// `[u16; 1]` and `[u32; 1]`; 7 bits in a native integer, `[u64; 1]` and `[u128; 1]`; and 8 bits in
/// a native integer, a wider one and `[u8; 1]`.
#[inline(never)]
fn decimals_of_the_8_bit_class() {
  decimals!(U0, 0, u8, i8; [u64; 0], [u64; 0]; u8, i8; [u8; 1], [u8; 1]);
  decimals!(U1, 1, u8, i8; u8, i8; [u16; 1], [u16; 1]; [u32; 1], [u32; 1]);
  decimals!(U7, 7, u8, i8; u8, i8; [u64; 1], [u64; 1]; [u128; 1], [u128; 1]);
  decimals!(U8, 8, u8, i8; u8, i8; u16, i16; [u8; 1], [u8; 1]);
}

/// The decimal text of the class of `u16` and `i16`: 9 bits in a native integer, `[u8; 2]` and
/// `[u16; 1]`; 15 bits in a native integer, `[u32; 1]` and `[u64; 1]`; and 16 bits in a native
/// integer, `[u128; 1]` and a wider native integer.
#[inline(never)]
fn decimals_of_the_16_bit_class() {
  decimals!(U9, 9, u16, i16; u16, i16; [u8; 2], [u8; 2]; [u16; 1], [u16; 1]);
  decimals!(U15, 15, u16, i16; u16, i16; [u32; 1], [u32; 1]; [u64; 1], [u64; 1]);
  decimals!(U16, 16, u16, i16; u16, i16; [u128; 1], [u128; 1]; u32, i32);
}

/// The decimal text of the class of `u32` and `i32`: 17 bits in a native integer, `[u8; 3]` and
/// `[u16; 2]`; 31 bits in a native integer, `[u32; 1]` and `[u64; 1]`; and 32 bits in a native
/// integer, `[u128; 1]` and a wider native integer.
#[inline(never)]
fn decimals_of_the_32_bit_class() {
  decimals!(U17, 17, u32, i32; u32, i32; [u8; 3], [u8; 3]; [u16; 2], [u16; 2]);
  decimals!(U31, 31, u32, i32; u32, i32; [u32; 1], [u32; 1]; [u64; 1], [u64; 1]);
  decimals!(U32, 32, u32, i32; u32, i32; [u128; 1], [u128; 1]; u64, i64);
}

/// The decimal text of the class of `u64` and `i64`: 33 bits in a native integer, `[u8; 5]` and
/// `[u16; 3]`; 63 bits in a native integer, `[u32; 2]` and `[u64; 1]`; and 64 bits in a native
/// integer, `[u128; 1]` and a wider native integer.
#[inline(never)]
fn decimals_of_the_64_bit_class() {
  decimals!(U33, 33, u64, i64; u64, i64; [u8; 5], [u8; 5]; [u16; 3], [u16; 3]);
  decimals!(U63, 63, u64, i64; u64, i64; [u32; 2], [u32; 2]; [u64; 1], [u64; 1]);
  decimals!(U64, 64, u64, i64; u64, i64; [u128; 1], [u128; 1]; u128, i128);
}

/// The decimal text of the class of `u128` and `i128`: 65 bits in a native integer, `[u8; 9]` and
/// `[u16; 5]`; 100 bits in a native integer, `[u32; 4]` and `[u64; 2]`; 127 bits in a native
/// integer, `[u128; 1]` and `[u8; 16]`; and 128 bits in a native integer, `[u64; 2]` and
/// `[u32; 4]`.
#[inline(never)]
fn decimals_of_the_128_bit_class() {
  decimals!(U65, 65, u128, i128; u128, i128; [u8; 9], [u8; 9]; [u16; 5], [u16; 5]);
  decimals!(U100, 100, u128, i128; u128, i128; [u32; 4], [u32; 4]; [u64; 2], [u64; 2]);
  decimals!(U127, 127, u128, i128; u128, i128; [u128; 1], [u128; 1]; [u8; 16], [u8; 16]);
  decimals!(U128, 128, u128, i128; u128, i128; [u64; 2], [u64; 2]; [u32; 4], [u32; 4]);
}

/// Checks [`decimal_round_trip`] at the value in `T`, above 128 bits, of each pattern of
/// `patterns`.
#[inline(never)]
fn decimal_texts<T>(patterns: &[Vec<bool>])
where
  T: Patterned + DecimalString + Display,
{
  for p in patterns {
    decimal_round_trip(T::from_pattern(p));
  }
}

/// Checks [`decimal_texts`] at `Uint` and `Int` of `$bits` bits, the width `$width`, above 128
/// bits, in each pair of storages of `$storages`, written `unsigned, signed;`, at the patterns of
/// `text_patterns` in `$words` words of 64 bits.
macro_rules! decimal_texts {
  ($width:ty, $bits:literal, $words:literal; $($uint:ty, $int:ty);+ $(;)?) => {{
    let patterns = text_patterns::<$words>($bits);
    $(
      decimal_texts::<Uint<$width, $uint>>(&patterns);
      decimal_texts::<Int<$width, $int>>(&patterns);
    )+
  }};
}

/// The decimal text of 129, 192, 200, 256, 1024 and 4096 bits, in limbs of 64 and 8 bits or of 32
/// and 128 bits.
#[inline(never)]
fn decimals_above_128_bits() {
  decimal_texts!(U129, 129, 3; [u64; 3], [u64; 3]; [u8; 17], [u8; 17]);
  decimal_texts!(U192, 192, 3; [u32; 6], [u32; 6]; [u128; 2], [u128; 2]);
  decimal_texts!(U200, 200, 4; [u64; 4], [u64; 4]; [u8; 25], [u8; 25]);
  decimal_texts!(U256, 256, 4; [u32; 8], [u32; 8]; [u128; 2], [u128; 2]);
  decimal_texts!(U1024, 1024, 16; [u64; 16], [u64; 16]; [u8; 128], [u8; 128]);
  decimal_texts!(U4096, 4096, 64; [u32; 128], [u32; 128]; [u128; 32], [u128; 32]);
}

/// The decimal text of 8192, 16384 and 65536 bits in the default storage, in an optimized build.
#[inline(never)]
fn decimals_of_the_widest_widths() {
  decimal_texts!(U8192, 8192, 128; [u64; 128], [u64; 128]);
  decimal_texts!(U16384, 16384, 256; [u64; 256], [u64; 256]);
  decimal_texts!(U65536, 65536, 1024; [u64; 1024], [u64; 1024]);
}

/// Asserts that the non-zero `T` rejects zero with `decimal`, as the string `"0"` and as the
/// integer 0 that a format gives for the request of a string, with std's error of zero of the
/// parser and of a conversion.
#[inline(never)]
fn decimal_rejects_zero<T: DecimalString + Debug>() {
  let name = type_name::<T>();
  let error = serde_json::from_str::<with_decimal::Required<T>>("\"0\"").unwrap_err();
  assert_eq!(
    json_message(&error),
    parse_error::<NonZero<u8>>("0"),
    "{name}"
  );
  let zero: Result<T, de::value::Error> = decimal::required::deserialize(0u64.into_deserializer());
  assert_eq!(
    zero.unwrap_err().to_string(),
    zero_of_conversion(),
    "{name}"
  );
}

/// Checks [`decimal_round_trip`] at the smallest value, zero and the largest value of each native
/// integer `$native`, and at the smallest and the largest value of its `NonZero`, which rejects
/// zero.
macro_rules! native_decimals {
  ($($native:ident),+ $(,)?) => {$(
    for value in [$native::MIN, 0, $native::MAX] {
      decimal_round_trip(value);
    }
    for value in [NonZero::<$native>::MIN, NonZero::<$native>::MAX] {
      decimal_round_trip(value);
    }
    decimal_rejects_zero::<NonZero<$native>>();
  )+};
}

/// The decimal text of the native integers and of their `NonZero`.
#[inline(never)]
fn decimals_of_the_native_integers() {
  native_decimals!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);
}

/// Checks that the non-zero integer `T`, above 128 bits, writes the value of each pattern of
/// `patterns` but zero with `decimal` as the integer `I` of the same value does, and reads it back.
#[inline(never)]
fn nonzero_decimal_texts<T, I>(patterns: &[Vec<bool>])
where
  T: CheckedCastFrom<I> + DecimalString + Display + PartialEq + Debug,
  I: Patterned,
{
  for p in patterns {
    if let Some(value) = T::checked_cast_from(I::from_pattern(p)) {
      decimal_round_trip(value);
    }
  }
}

/// The decimal text of the non-zero integers, in the native `NonZero` of their storage and in
/// arrays of each type of limb, which reject zero.
#[inline(never)]
fn decimals_of_the_nonzero_integers() {
  decimal_round_trips::<NonZeroUint<U7>, u8>(&helper_nonzero_natives(7, false));
  decimal_round_trips::<NonZeroInt<U7, [u8; 1]>, i8>(&helper_nonzero_natives(7, true));
  decimal_round_trips::<NonZeroUint<U100>, u128>(&helper_nonzero_natives(100, false));
  decimal_round_trips::<NonZeroInt<U128, [u32; 4]>, i128>(&helper_nonzero_natives(128, true));
  nonzero_decimal_texts::<NonZeroUint<U256>, Uint<U256>>(&text_patterns::<4>(256));
  let patterns = text_patterns::<4>(200);
  nonzero_decimal_texts::<NonZeroInt<U200, [u8; 25]>, Int<U200, [u8; 25]>>(&patterns);
  decimal_rejects_zero::<NonZeroUint<U7>>();
  decimal_rejects_zero::<NonZeroInt<U128, [u32; 4]>>();
  decimal_rejects_zero::<NonZeroUint<U256>>();
  decimal_rejects_zero::<NonZeroInt<U0>>();
}

/// `decimal` writes the decimal text of a value, as a string, in JSON and in postcard, and reads it
/// back from both, at every width, in every family of storages, up to 128 bits at the values of
/// [`helper_samples`] and above at the patterns of the text of the defaults, in an optimized
/// build up to 65536 bits; at the ends of the native integers and of their `NonZero`; and at the
/// non-zero integers, which reject zero.
#[test]
fn decimal_writes_the_decimal_text_at_every_width_and_reads_it_back() {
  decimals_of_the_8_bit_class();
  decimals_of_the_16_bit_class();
  decimals_of_the_32_bit_class();
  decimals_of_the_64_bit_class();
  decimals_of_the_128_bit_class();
  decimals_above_128_bits();
  if tier().is_full() {
    decimals_of_the_widest_widths();
  }
  decimals_of_the_native_integers();
  decimals_of_the_nonzero_integers();
}

/// What `T` reads with `decimal` from serde's deserializer of `value`, which answers every request
/// with its value, as toml and MessagePack answer the request of a string with an integer, or the
/// text of the error.
#[inline(never)]
fn decimal_from<T, V>(value: V) -> Result<T, String>
where
  T: DecimalString,
  V: IntoDeserializer<'static, de::value::Error>,
{
  decimal::required::deserialize(value.into_deserializer())
    .map_err(|error: de::value::Error| error.to_string())
}

/// `decimal` reads an integer that a format gives for the request of a string, checked against the
/// width: serde_test's tokens of the integers, which it hands over for every request, and serde's
/// deserializers of `u64` and `i64`; outside the range of the width it gets std's texts of a
/// conversion.
#[test]
fn decimal_reads_an_integer_that_a_format_gives_for_a_string() {
  let tokens = |token| [Token::NewtypeStruct { name: "Required" }, token];
  assert_de_tokens(
    &with_decimal::Required(u7::new(100)),
    &tokens(Token::U8(100)),
  );
  assert_de_tokens(&with_decimal::Required(i7::MIN), &tokens(Token::I8(-64)));
  assert_de_tokens(
    &with_decimal::Required(Uint::<U100>::new(5)),
    &tokens(Token::U64(5)),
  );
  assert_de_tokens(
    &with_decimal::Required(i256::from(i64::MIN)),
    &tokens(Token::I64(i64::MIN)),
  );
  assert_de_tokens(&with_decimal::Required(-1i16), &tokens(Token::I32(-1)));
  assert_de_tokens_error::<with_decimal::Required<u7>>(&tokens(Token::U8(128)), &too_large());
  assert_de_tokens_error::<with_decimal::Required<u7>>(&tokens(Token::I8(-1)), &too_small());
  assert_eq!(decimal_from::<u7, _>(127u64), Ok(u7::MAX));
  assert_eq!(decimal_from::<u7, _>(128u64), Err(too_large()));
  assert_eq!(decimal_from::<Int<U12>, _>(-2048i64), Ok(Int::<U12>::MIN));
  assert_eq!(decimal_from::<Int<U12>, _>(-2049i64), Err(too_small()));
  assert_eq!(decimal_from::<Int<U12>, _>(2048i64), Err(too_large()));
  assert_eq!(decimal_from::<u256, _>(u64::MAX), Ok(u256::from(u64::MAX)));
  assert_eq!(decimal_from::<i256, _>(i64::MIN), Ok(i256::from(i64::MIN)));
  assert_eq!(decimal_from::<u8, _>(-1i64), Err(too_small()));
  assert_eq!(
    decimal_from::<NonZero<u16>, _>(65535u64),
    Ok(NonZero::<u16>::MAX)
  );
}

/// Asserts that `T` rejects with `decimal` every JSON value but a string with the fixed message and
/// the category of the data, a number of its class included, and serde_json's number token as an
/// object; and the strings `""`, `" 5"`, `"0x5"` and `"5 "` with the texts of std's parser; no
/// message holds the input.
#[inline(never)]
fn decimal_rejects_json<T: DecimalString + Debug>() {
  let name = type_name::<T>();
  let token = format!(r#"{{"{NUMBER_TOKEN}":"5"}}"#);
  for json in [
    "5", "-5", "0", "1.5", "-0", "1e2", "true", "null", "[]", "[5]", "{}", &token,
  ] {
    let error = serde_json::from_str::<with_decimal::Required<T>>(json).unwrap_err();
    assert_eq!(json_message(&error), DECIMAL, "{json} into {name}");
    assert_eq!(error.classify(), Category::Data, "{json} into {name}");
  }
  for text in ["", " 5", "0x5", "5 "] {
    let json = format!("\"{text}\"");
    let error = serde_json::from_str::<with_decimal::Required<T>>(&json).unwrap_err();
    assert_eq!(
      json_message(&error),
      parse_error::<u8>(text),
      "{text:?} into {name}"
    );
  }
}

/// Asserts that `T` rejects with `decimal` every token of serde_test that is neither an integer nor
/// a string with exactly the fixed message: `char` too, and a sequence and a map only empty, so
/// that the rejection leaves no more than one token unread; and serde_json's number token through
/// serde's deserializer of a map.
#[inline(never)]
fn decimal_rejects_other_tokens<T: DecimalString + Debug>() {
  for token in [
    Token::Bool(true),
    Token::F32(5.0),
    Token::F64(1.5),
    Token::Char('5'),
    Token::Bytes(b"5"),
    Token::None,
    Token::Unit,
    Token::UnitStruct { name: "Unit" },
  ] {
    let tokens = [Token::NewtypeStruct { name: "Required" }, token];
    assert_de_tokens_error::<with_decimal::Required<T>>(&tokens, DECIMAL);
  }
  for tokens in [
    &[Token::Some, Token::U8(5)][..],
    &[Token::Seq { len: Some(0) }, Token::SeqEnd],
    &[Token::Map { len: Some(0) }, Token::MapEnd],
  ] {
    let tokens = [&[Token::NewtypeStruct { name: "Required" }][..], tokens].concat();
    assert_de_tokens_error::<with_decimal::Required<T>>(&tokens, DECIMAL);
  }
  let map = de::value::MapDeserializer::new([(NUMBER_TOKEN, "5")].into_iter());
  let error: de::value::Error = decimal::required::deserialize::<T, _>(map).unwrap_err();
  assert_eq!(error.to_string(), DECIMAL, "{}", type_name::<T>());
}

/// `decimal` asks every format for a string: every JSON value but a string gets its fixed message,
/// a number of every class included, and so does every token of serde_test but an integer and a
/// string, and serde_json's number token, which `decimal` does not read; a string that does not
/// parse gets the text of std's parser.
#[test]
fn decimal_rejects_every_other_input_with_its_fixed_message() {
  decimal_rejects_json::<u7>();
  decimal_rejects_json::<i7>();
  decimal_rejects_json::<Uint<U64>>();
  decimal_rejects_json::<Int<U100>>();
  decimal_rejects_json::<Uint<U128, [u64; 2]>>();
  decimal_rejects_json::<u256>();
  decimal_rejects_json::<Int<U129, [u8; 17]>>();
  decimal_rejects_json::<u64>();
  decimal_rejects_json::<NonZero<i128>>();
  decimal_rejects_other_tokens::<u7>();
  decimal_rejects_other_tokens::<Int<U100>>();
  decimal_rejects_other_tokens::<u256>();
  decimal_rejects_other_tokens::<i32>();
}

/// `decimal` reads where the defaults do not: a quoted value of 65 to 128 bits from JSON, a value
/// of 65 to 128 bits in an untagged enum and a flattened struct, and every value from csv's field,
/// whose request of a string it answers with the text, `2^128` and `u256::MAX` among them.
#[test]
fn decimal_reads_where_the_defaults_do_not() {
  let five = Uint::<U100>::new(5);
  let read = serde_json::from_str::<with_decimal::Required<Uint<U100>>>("\"5\"");
  assert_eq!(read.unwrap(), with_decimal::Required(five));
  assert!(serde_json::from_str::<Uint<U100>>("\"5\"").is_err());
  for result in through_the_buffer::<with_decimal::Field<Uint<U100>>>(r#"{"value":"5"}"#) {
    assert_eq!(result, Ok(with_decimal::Field { value: five }));
  }
  let minus_five = Int::<U128, [u32; 4]>::checked_cast_from(-5i8).unwrap();
  for result in through_the_buffer::<with_decimal::Field<Int<U128, [u32; 4]>>>(r#"{"value":"-5"}"#)
  {
    assert_eq!(result, Ok(with_decimal::Field { value: minus_five }));
  }
  let power = u256::from(u128::MAX) + u256::from(1u8);
  for value in [power, u256::MAX] {
    let text = value.to_string();
    let read: Result<u256, de::value::Error> = decimal::required::deserialize(CsvField(&text));
    assert_eq!(read.unwrap(), value);
    let default: Result<u256, de::value::Error> = u256::deserialize(CsvField(&text));
    assert!(default.is_err());
  }
}

/// The leaf `optional` of `decimal`: `None` is JSON's `null` and postcard's `[0]`, and `Some` is
/// postcard's `[1]` followed by the text; a missing field reads as `None` with `default` and is
/// required without it; `null` reads as `None`, also through serde's buffer, which gives it as a
/// unit; an empty csv field is `None`; and every other input gets the fixed message.
#[test]
fn optional_decimal() {
  options_round_trip(
    with_decimal::Optional::<u7>(None),
    with_decimal::Optional(Some(u7::new(5))),
    "\"5\"",
    &postcard::to_stdvec("5").unwrap(),
  );
  options_round_trip(
    with_decimal::Optional::<Int<U100>>(None),
    with_decimal::Optional(Some(Int::<U100>::new(-5))),
    "\"-5\"",
    &postcard::to_stdvec("-5").unwrap(),
  );
  let max = u256::MAX.to_string();
  options_round_trip(
    with_decimal::Optional::<u256>(None),
    with_decimal::Optional(Some(u256::MAX)),
    &format!("\"{max}\""),
    &postcard::to_stdvec(&max).unwrap(),
  );
  option_fields(
    [
      with_decimal::Defaulted::<Uint<U100>> { value: None },
      with_decimal::Defaulted {
        value: Some(Uint::<U100>::new(5)),
      },
    ],
    [
      with_decimal::Present::<Uint<U100>> { value: None },
      with_decimal::Present {
        value: Some(Uint::<U100>::new(5)),
      },
    ],
    "\"5\"",
  );
  let read: Result<Option<u256>, de::value::Error> = decimal::optional::deserialize(CsvField(""));
  assert_eq!(read.unwrap(), None);
  let read: Result<Option<u256>, de::value::Error> = decimal::optional::deserialize(CsvField("5"));
  assert_eq!(read.unwrap(), Some(u256::from(5u8)));
  for json in ["5", "true", "[]", "{}"] {
    let error = serde_json::from_str::<with_decimal::Optional<u7>>(json).unwrap_err();
    assert_eq!(json_message(&error), DECIMAL, "{json}");
  }
  let tokens = [
    Token::NewtypeStruct { name: "Optional" },
    Token::Some,
    Token::Str("5"),
  ];
  assert_tokens(&with_decimal::Optional(Some(u7::new(5))), &tokens);
  let tokens = [Token::NewtypeStruct { name: "Optional" }, Token::Bool(true)];
  assert_de_tokens_error::<with_decimal::Optional<u7>>(&tokens, DECIMAL);
}

/// The fixed message of `quantity`.
const QUANTITY: &str = "expected a hexadecimal quantity as a string";

/// The message of `quantity` for a text without the prefix `0x` or `0X`.
const NO_PREFIX: &str = "hexadecimal quantity without the 0x prefix";

/// The message of `quantity` for the prefix alone.
const NO_DIGITS: &str = "hexadecimal quantity without digits";

/// The message of `quantity` for digits that start with a zero.
const LEADING_ZEROS: &str = "hexadecimal quantity with leading zero digits";

/// Checks that `T` writes `value` with `quantity` as the string `text`, in JSON and in postcard,
/// and reads it back from both.
#[inline(never)]
fn quantity_round_trip<T>(value: T, text: &str)
where
  T: Quantity + PartialEq + Debug,
{
  let name = type_name::<T>();
  let value = with_quantity::Required(value);
  let json = serde_json::to_string(&value).unwrap();
  assert_eq!(json, format!("\"{text}\""), "{name}");
  let bytes = postcard::to_stdvec(&value).unwrap();
  assert_eq!(
    bytes,
    postcard::to_stdvec(text).unwrap(),
    "{text} in {name}"
  );
  let from_json = serde_json::from_str::<with_quantity::Required<T>>(&json).unwrap();
  assert_eq!(from_json, value, "{text} in {name}");
  let from_bytes = postcard::from_bytes::<with_quantity::Required<T>>(&bytes).unwrap();
  assert_eq!(from_bytes, value, "{text} in {name}");
}

/// Checks [`quantity_round_trip`] at the value in `T` of each of `natives`, which `T` holds, whose
/// text is that of `{:#x}` of the native integer.
#[inline(never)]
fn quantity_round_trips<T, N>(natives: &[N])
where
  T: CheckedCastFrom<N> + Quantity + PartialEq + Debug,
  N: Native + LowerHex,
{
  for &native in natives {
    let value = T::checked_cast_from(native).expect("a value of the width");
    quantity_round_trip(value, &format!("{native:#x}"));
  }
}

/// Checks [`quantity_round_trips`] at `Uint` and `NonZeroUint` of `$bits` bits, the width `$width`,
/// whose class has the native integer `$native`, in the storages of each pair of `$storages`,
/// written `uint, nonzero;`, at the values of [`helper_natives`] and, but zero, of
/// [`helper_nonzero_natives`].
macro_rules! quantities {
  ($width:ty, $bits:literal, $native:ident; $($uint:ty, $nonzero:ty);+ $(;)?) => {{
    let values: Vec<$native> = helper_natives($bits, false);
    let nonzero: Vec<$native> = helper_nonzero_natives($bits, false);
    $(
      quantity_round_trips::<Uint<$width, $uint>, $native>(&values);
      quantity_round_trips::<NonZeroUint<$width, $nonzero>, $native>(&nonzero);
    )+
  }};
}

/// The QUANTITY of `Uint` and `NonZeroUint` of 0, 1, 4, 7, 8, 64, 65 and 128 bits, in the native
/// storage of the width and in an array.
#[inline(never)]
fn quantities_up_to_128_bits() {
  quantities!(U0, 0, u8; u8, NonZero<u8>; [u8; 1], [u8; 1]);
  quantities!(U1, 1, u8; u8, NonZero<u8>; [u16; 1], [u16; 1]);
  quantities!(U4, 4, u8; u8, NonZero<u8>; [u32; 1], [u32; 1]);
  quantities!(U7, 7, u8; u8, NonZero<u8>; [u8; 1], [u8; 1]);
  quantities!(U8, 8, u8; u8, NonZero<u8>; [u64; 1], [u64; 1]);
  quantities!(U64, 64, u64; u64, NonZero<u64>; [u8; 8], [u8; 8]);
  quantities!(U65, 65, u128; u128, NonZero<u128>; [u16; 5], [u16; 5]);
  quantities!(U128, 128, u128; u128, NonZero<u128>; [u32; 4], [u32; 4]);
}

/// Checks [`quantity_round_trip`] at the value in `T`, above 128 bits, of each pattern of
/// `patterns`, whose text is that of `{:#x}` of the integer `I` of the same value, and which `T`
/// holds unless it is zero for a non-zero `T`.
#[inline(never)]
fn quantity_texts<T, I>(patterns: &[Vec<bool>])
where
  T: CheckedCastFrom<I> + Quantity + PartialEq + Debug,
  I: Patterned + LowerHex,
{
  for p in patterns {
    let integer = I::from_pattern(p);
    if let Some(value) = T::checked_cast_from(integer) {
      quantity_round_trip(value, &format!("{integer:#x}"));
    }
  }
}

/// The QUANTITY of `Uint` and `NonZeroUint` of 256 and 1024 bits, in limbs of 64 and 8 bits or of
/// 64 and 128 bits.
#[inline(never)]
fn quantities_above_128_bits() {
  let patterns = text_patterns::<4>(256);
  quantity_texts::<Uint<U256>, Uint<U256>>(&patterns);
  quantity_texts::<Uint<U256, [u8; 32]>, Uint<U256, [u8; 32]>>(&patterns);
  quantity_texts::<NonZeroUint<U256>, Uint<U256>>(&patterns);
  quantity_texts::<NonZeroUint<U256, [u8; 32]>, Uint<U256, [u8; 32]>>(&patterns);
  let patterns = text_patterns::<16>(1024);
  quantity_texts::<Uint<U1024>, Uint<U1024>>(&patterns);
  quantity_texts::<Uint<U1024, [u128; 8]>, Uint<U1024, [u128; 8]>>(&patterns);
  quantity_texts::<NonZeroUint<U1024>, Uint<U1024>>(&patterns);
  quantity_texts::<NonZeroUint<U1024, [u128; 8]>, Uint<U1024, [u128; 8]>>(&patterns);
}

/// What `T` reads with `quantity` from the JSON string `text`, or the message of the error.
#[inline(never)]
fn quantity_from<T: Quantity>(text: &str) -> Result<T, String> {
  serde_json::from_str::<with_quantity::Required<T>>(&format!("\"{text}\""))
    .map(|with_quantity::Required(value)| value)
    .map_err(|error| json_message(&error))
}

/// Asserts that the non-zero `T` rejects `0x0` with `quantity`, with std's text of zero.
#[inline(never)]
fn quantity_rejects_zero<T: Quantity + Debug>() {
  let zero = parse_error::<NonZero<u8>>("0");
  let error = quantity_from::<T>("0x0").unwrap_err();
  assert_eq!(error, zero, "{}", type_name::<T>());
}

/// Checks [`quantity_round_trip`] at zero, one and the largest value of each native integer
/// `$native`, and at the smallest and the largest value of its `NonZero`, which rejects zero.
macro_rules! native_quantities {
  ($($native:ident),+ $(,)?) => {$(
    for value in [0, 1, $native::MAX] {
      quantity_round_trip(value, &format!("{value:#x}"));
    }
    for value in [NonZero::<$native>::MIN, NonZero::<$native>::MAX] {
      quantity_round_trip(value, &format!("{value:#x}"));
    }
    quantity_rejects_zero::<NonZero<$native>>();
  )+};
}

/// The QUANTITY of the unsigned native integers and of their `NonZero`.
#[inline(never)]
fn quantities_of_the_native_integers() {
  native_quantities!(u8, u16, u32, u64, u128);
}

/// `quantity` writes the text of `{:#x}` of a value, `0x` and its lowercase digits without leading
/// zeros, as a string in JSON and in postcard, and reads it back from both: `Uint` and
/// `NonZeroUint` of 0, 1, 4, 7, 8, 64, 65, 128, 256 and 1024 bits in two storages each, whose text
/// up to 128 bits is that of the native integer of the class, and the unsigned native integers
/// and their `NonZero` at their ends. Zero bits write and read `0x0` alone, and a non-zero type
/// rejects zero.
#[test]
fn quantity_writes_the_hexadecimal_quantity_and_reads_it_back() {
  quantities_up_to_128_bits();
  quantities_above_128_bits();
  quantities_of_the_native_integers();
  quantity_round_trip(u0::ZERO, "0x0");
  assert_eq!(quantity_from::<u0>("0x1"), Err(parse_error::<u8>("256")));
  quantity_rejects_zero::<NonZeroUint<U7>>();
  quantity_rejects_zero::<NonZeroUint<U0>>();
  quantity_rejects_zero::<NonZeroUint<U256, [u8; 32]>>();
}

/// Asserts that `T` reads with `quantity` each of the texts `0x0`, `0x1`, `0xff`, `0XFF` and
/// `0xAbC`, of either case, as its value where `T` holds it, and otherwise rejects it with std's
/// text of zero for a non-zero `T`, or of a value too large.
#[inline(never)]
fn quantity_reads_either_case<T>()
where
  T: Quantity + CheckedCastFrom<u16> + PartialEq + Debug,
{
  let name = type_name::<T>();
  for (text, value) in [
    ("0x0", 0u16),
    ("0x1", 1),
    ("0xff", 0xff),
    ("0XFF", 0xff),
    ("0xAbC", 0xabc),
  ] {
    let expected = match T::checked_cast_from(value) {
      Some(value) => Ok(value),
      None if value == 0 => Err(parse_error::<NonZero<u8>>("0")),
      None => Err(parse_error::<u8>("256")),
    };
    assert_eq!(quantity_from::<T>(text), expected, "{text} into {name}");
  }
}

/// `quantity` reads as go-ethereum's `hexutil` decodes a number: `0x` or `0X` and hexadecimal
/// digits of either case without a leading zero, checked against the width without forming the
/// value, so that one digit more than the largest value or a first digit of one bit more is too
/// large.
#[test]
fn quantity_reads_as_go_ethereum_decodes_a_number() {
  quantity_reads_either_case::<u0>();
  quantity_reads_either_case::<Uint<U1, [u16; 1]>>();
  quantity_reads_either_case::<u7>();
  quantity_reads_either_case::<Uint<U8, [u64; 1]>>();
  quantity_reads_either_case::<Uint<U12>>();
  quantity_reads_either_case::<u256>();
  quantity_reads_either_case::<NonZeroUint<U7>>();
  quantity_reads_either_case::<NonZero<u16>>();
  let too_large = parse_error::<u8>("256");
  assert_eq!(quantity_from::<u7>("0x7f"), Ok(u7::MAX));
  assert_eq!(quantity_from::<u7>("0x80"), Err(too_large.clone()));
  assert_eq!(quantity_from::<Uint<U9>>("0x1ff"), Ok(Uint::<U9>::MAX));
  assert_eq!(quantity_from::<Uint<U9>>("0x200"), Err(too_large.clone()));
  assert_eq!(quantity_from::<Uint<U9>>("0x1000"), Err(too_large.clone()));
  let max = format!("0x{}", "f".repeat(64));
  assert_eq!(quantity_from::<u256>(&max), Ok(u256::MAX));
  let above = format!("{max}f");
  assert_eq!(quantity_from::<u256>(&above), Err(too_large.clone()));
  let wide = format!("0x8{}", "0".repeat(63));
  let read = quantity_from::<u256>(&wide).map(|value| format!("{value:#x}"));
  assert_eq!(read, Ok(wide));
  assert_eq!(quantity_from::<u64>("0xffffffffffffffff"), Ok(u64::MAX));
  assert_eq!(quantity_from::<u64>("0x10000000000000000"), Err(too_large));
}

/// Asserts that `T` rejects with `quantity` each text that breaks a rule of its grammar, with the
/// message of the first rule that it breaks: the prefix, at least one digit, no leading zero, and
/// every byte a hexadecimal digit, which a byte of a character outside ASCII is not.
#[inline(never)]
fn quantity_rejects_texts<T: Quantity + Debug>() {
  let name = type_name::<T>();
  let invalid = parse_error::<u8>("g");
  let invalid = invalid.as_str();
  for (text, message) in [
    ("", NO_PREFIX),
    ("0", NO_PREFIX),
    ("ff", NO_PREFIX),
    ("x1", NO_PREFIX),
    ("-0x1", NO_PREFIX),
    ("+0x1", NO_PREFIX),
    (" 0x1", NO_PREFIX),
    ("0x", NO_DIGITS),
    ("0X", NO_DIGITS),
    ("0x00", LEADING_ZEROS),
    ("0x01", LEADING_ZEROS),
    ("0x0g", LEADING_ZEROS),
    ("0x+1", invalid),
    ("0x-1", invalid),
    ("0x1 ", invalid),
    ("0x_1", invalid),
    ("0x1_0", invalid),
    ("0xg", invalid),
    ("0x0x1", LEADING_ZEROS),
    ("0x\u{665}", invalid),
    ("0x1\u{665}", invalid),
  ] {
    assert_eq!(
      quantity_from::<T>(text).unwrap_err(),
      message,
      "{text:?} into {name}"
    );
  }
}

/// Asserts that `T` rejects with `quantity` every input but a string with the fixed message: JSON
/// numbers and the other values, serde_test's tokens of an integer and of the other kinds, and
/// serde's deserializers of an integer and of a map, as toml gives them.
#[inline(never)]
fn quantity_rejects_other_inputs<T: Quantity + Debug>() {
  let name = type_name::<T>();
  for json in ["5", "0", "1.5", "true", "null", "[]", "{}"] {
    let error = serde_json::from_str::<with_quantity::Required<T>>(json).unwrap_err();
    assert_eq!(json_message(&error), QUANTITY, "{json} into {name}");
    assert_eq!(error.classify(), Category::Data, "{json} into {name}");
  }
  for token in [
    Token::U8(5),
    Token::I64(5),
    Token::Bool(true),
    Token::Char('5'),
    Token::Bytes(b"0x5"),
    Token::Unit,
  ] {
    let tokens = [Token::NewtypeStruct { name: "Required" }, token];
    assert_de_tokens_error::<with_quantity::Required<T>>(&tokens, QUANTITY);
  }
  let read: Result<T, de::value::Error> = quantity::required::deserialize(5i64.into_deserializer());
  assert_eq!(read.unwrap_err().to_string(), QUANTITY, "{name}");
  let map = de::value::MapDeserializer::new([(NUMBER_TOKEN, "5")].into_iter());
  let read: Result<T, de::value::Error> = quantity::required::deserialize(map);
  assert_eq!(read.unwrap_err().to_string(), QUANTITY, "{name}");
}

/// `quantity` rejects each text that breaks a rule of its grammar with the message of the first
/// that it breaks: a text without the prefix, the empty string included, the prefix alone, digits
/// that start with a zero, and then a byte that is not a hexadecimal digit, with std's text of an
/// invalid digit, so that a sign, `_`, whitespace and a digit outside ASCII do not read. Every
/// input but a string gets the fixed message, an integer included.
#[test]
fn quantity_rejects_what_go_ethereum_does_not_decode() {
  quantity_rejects_texts::<u0>();
  quantity_rejects_texts::<u7>();
  quantity_rejects_texts::<Uint<U100, [u32; 4]>>();
  quantity_rejects_texts::<u256>();
  quantity_rejects_texts::<NonZeroUint<U64, [u8; 8]>>();
  quantity_rejects_texts::<u128>();
  quantity_rejects_other_inputs::<u7>();
  quantity_rejects_other_inputs::<Uint<U100>>();
  quantity_rejects_other_inputs::<u256>();
  quantity_rejects_other_inputs::<NonZero<u32>>();
}

/// The leaf `optional` of `quantity`: `None` is JSON's `null` and postcard's `[0]`, and `Some` is
/// postcard's `[1]` followed by the text; a missing field reads as `None` with `default` and is
/// required without it; `null` reads as `None`, also through serde's buffer; an empty csv field is
/// `None`; and every other input gets the fixed message. The leaf `required` reads through serde's
/// buffer, in an untagged enum and a flattened struct.
#[test]
fn quantity_in_options_and_serde_buffer() {
  options_round_trip(
    with_quantity::Optional::<u64>(None),
    with_quantity::Optional(Some(255)),
    "\"0xff\"",
    &postcard::to_stdvec("0xff").unwrap(),
  );
  options_round_trip(
    with_quantity::Optional::<u256>(None),
    with_quantity::Optional(Some(u256::ZERO)),
    "\"0x0\"",
    &postcard::to_stdvec("0x0").unwrap(),
  );
  option_fields(
    [
      with_quantity::Defaulted::<u256> { value: None },
      with_quantity::Defaulted {
        value: Some(u256::from(255u8)),
      },
    ],
    [
      with_quantity::Present::<u256> { value: None },
      with_quantity::Present {
        value: Some(u256::from(255u8)),
      },
    ],
    "\"0xff\"",
  );
  let read: Result<Option<u64>, de::value::Error> = quantity::optional::deserialize(CsvField(""));
  assert_eq!(read.unwrap(), None);
  let read: Result<Option<u64>, de::value::Error> =
    quantity::optional::deserialize(CsvField("0xff"));
  assert_eq!(read.unwrap(), Some(255));
  for json in ["255", "true", "[]", "{}"] {
    let error = serde_json::from_str::<with_quantity::Optional<u64>>(json).unwrap_err();
    assert_eq!(json_message(&error), QUANTITY, "{json}");
  }
  let error = serde_json::from_str::<with_quantity::Optional<u64>>("\"ff\"").unwrap_err();
  assert_eq!(json_message(&error), NO_PREFIX);
  for result in through_the_buffer::<with_quantity::Field<u256>>(r#"{"value":"0xff"}"#) {
    assert_eq!(
      result,
      Ok(with_quantity::Field {
        value: u256::from(255u8)
      })
    );
  }
  for result in through_the_buffer::<with_quantity::Field<Uint<U100>>>(r#"{"value":"0xFF"}"#) {
    assert_eq!(
      result,
      Ok(with_quantity::Field {
        value: Uint::<U100>::new(255)
      })
    );
  }
}
