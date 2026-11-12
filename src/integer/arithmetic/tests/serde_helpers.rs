//! The helper modules of `numo::serde`. `decimal` writes the decimal text at every width, in every
//! family of storages, of the native integers and of the non-zero integers, as a string in JSON and
//! postcard, and reads it back; it reads an integer that a format gives for the request of a
//! string, checked against the width, reads where the defaults do not, and rejects every other
//! input with its fixed message. `quantity` writes the QUANTITY of the unsigned integers and reads
//! it as go-ethereum's `hexutil` decodes a number, with the message of each rule of its grammar
//! and the range checked without forming the value. `fixed_hex` writes two digits for each byte of
//! the width, the pattern of the bits of a negative `Int`, and reads that pattern back, with the
//! message of each rule. `typed` writes the form of the defaults and reads it alone, asking every
//! format for the class of the width, through a recording stand-in, JSON, postcard, a format
//! without `deserialize_any` and serde's deserializers. `be_bytes` and `le_bytes` write the bytes
//! of a value in either order, as a JSON array and as postcard's length and bytes, against the
//! bytes of the native integers and numo's own, and read a byte string and a sequence of bytes
//! back, with the message of each rule and of a wrong element. The leaf `optional` of each helper
//! writes and reads `None` and `Some` in JSON and postcard, a missing field with and without
//! `default`, `null` through serde's buffer and an empty csv field; and each helper reads in
//! untagged enums and flattened structs.
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
  Deserialize, Deserializer, Serialize,
  de::{self, DeserializeOwned, IntoDeserializer},
};
use serde_json::error::Category;
use serde_test::{Token, assert_de_tokens, assert_de_tokens_error, assert_tokens};
use typenum::{Diff, U4, U12, U15, U17, U31, U33, U192, U8192, U16384, U65536, U262144};

use super::{
  serde::{
    CsvField, FIXED, NUMBER_TOKEN, Native, Probe, error_from, json_message, parse_error, samples,
    text_patterns, too_large, too_small, zero_of_conversion,
  },
  *,
};
use crate::{
  CheckedCastFrom, IntegerBytes, NonZeroInt, NonZeroUint, i0, i1,
  serde::{
    be_bytes::{self, BeBytes},
    decimal::{self, DecimalString},
    fixed_hex::{self, FixedHex},
    le_bytes::{self, LeBytes},
    quantity::{self, Quantity},
    typed::{self, Typed},
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

containers!(
  with_fixed_hex: FixedHex,
  "crate::serde::fixed_hex::required",
  "crate::serde::fixed_hex::optional"
);

containers!(
  with_typed: Typed,
  "crate::serde::typed::required",
  "crate::serde::typed::optional"
);

containers!(
  with_be_bytes: BeBytes,
  "crate::serde::be_bytes::required",
  "crate::serde::be_bytes::optional"
);

containers!(
  with_le_bytes: LeBytes,
  "crate::serde::le_bytes::required",
  "crate::serde::le_bytes::optional"
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

/// The fixed message of `fixed_hex`.
const FIXED_HEX: &str = "expected a hexadecimal string";

/// The message of `fixed_hex` for a text without the prefix `0x` or `0X`.
const NO_HEX_PREFIX: &str = "hexadecimal string without the 0x prefix";

/// The text that `fixed_hex` writes for the value of `bits` bits, at most 128, whose two's
/// complement in 128 bits is `pattern`: `0x` and two digits for each byte of the width, the bits of
/// the width zero-padded.
fn fixed_hex_text(pattern: u128, bits: u32) -> String {
  let digits = 2 * bits.div_ceil(8) as usize;
  let pattern = if bits == 128 {
    pattern
  } else {
    pattern & ((1 << bits) - 1)
  };
  if digits == 0 {
    "0x".to_string()
  } else {
    format!("0x{pattern:0digits$x}")
  }
}

/// The text that `fixed_hex` writes for the value whose bits, bit 0 first, are `pattern`: `0x` and
/// two digits for each byte of the width, the most significant first.
fn fixed_hex_text_of_pattern(pattern: &[bool]) -> String {
  let digits = 2 * pattern.len().div_ceil(8);
  let mut text = String::from("0x");
  for k in (0..digits).rev() {
    let digit = (0..4).fold(0, |digit, i| {
      digit | u32::from(pattern.get(4 * k + i).copied().unwrap_or(false)) << i
    });
    text.push(char::from_digit(digit, 16).expect("a hexadecimal digit"));
  }
  text
}

/// Checks that `T` writes `value` with `fixed_hex` as the string `text`, in JSON and in postcard,
/// and reads it back from both; and that `text` is that of `{:#0w$x}`, with `w` its length, where a
/// formatting width holds `w`, up to `u16::MAX`, and but at zero bits, where `{:#02x}` writes a
/// digit and the helper none.
#[inline(never)]
fn fixed_hex_round_trip<T>(value: T, text: &str)
where
  T: FixedHex + LowerHex + PartialEq + Debug,
{
  let name = type_name::<T>();
  let width = text.len();
  if (3..=usize::from(u16::MAX)).contains(&width) {
    assert_eq!(format!("{value:#0width$x}"), text, "{name}");
  }
  let value = with_fixed_hex::Required(value);
  let json = serde_json::to_string(&value).unwrap();
  assert_eq!(json, format!("\"{text}\""), "{name}");
  let bytes = postcard::to_stdvec(&value).unwrap();
  assert_eq!(
    bytes,
    postcard::to_stdvec(text).unwrap(),
    "{text} in {name}"
  );
  let from_json = serde_json::from_str::<with_fixed_hex::Required<T>>(&json).unwrap();
  assert_eq!(from_json, value, "{text} in {name}");
  let from_bytes = postcard::from_bytes::<with_fixed_hex::Required<T>>(&bytes).unwrap();
  assert_eq!(from_bytes, value, "{text} in {name}");
}

/// Checks [`fixed_hex_round_trip`] at the value in `T`, of `bits` bits, at most 128, of each
/// pattern of [`helper_samples`], read as the native integer `N` of the class of the width, whose
/// text has the bits of the pattern; a non-zero `T` holds every value but zero.
#[inline(never)]
fn fixed_hex_round_trips<T, N>(bits: u32, signed: bool)
where
  T: CheckedCastFrom<N> + FixedHex + LowerHex + PartialEq + Debug,
  N: Native,
{
  for pattern in helper_samples(bits, signed) {
    match T::checked_cast_from(N::of_pattern(pattern)) {
      Some(value) => fixed_hex_round_trip(value, &fixed_hex_text(pattern, bits)),
      None => assert_eq!(pattern, 0, "{}", type_name::<T>()),
    }
  }
}

/// Checks [`fixed_hex_round_trips`] at the four integer types of `$bits` bits, the width `$width`,
/// in the default storage, whose class has the native integers `$unsigned` and `$signed`.
macro_rules! fixed_hexes {
  ($($width:ty, $bits:literal: $unsigned:ident, $signed:ident;)+) => {$(
    fixed_hex_round_trips::<Uint<$width>, $unsigned>($bits, false);
    fixed_hex_round_trips::<Int<$width>, $signed>($bits, true);
    fixed_hex_round_trips::<NonZeroUint<$width>, $unsigned>($bits, false);
    fixed_hex_round_trips::<NonZeroInt<$width>, $signed>($bits, true);
  )+};
}

/// The text of `fixed_hex` at the four integer types of 0, 1, 4, 7, 8, 12, 16, 64, 65, 100 and 128
/// bits, in the default storage, and at four of them in arrays.
#[inline(never)]
fn fixed_hexes_up_to_128_bits() {
  fixed_hexes! {
    U0, 0: u8, i8;
    U1, 1: u8, i8;
    U4, 4: u8, i8;
    U7, 7: u8, i8;
    U8, 8: u8, i8;
    U12, 12: u16, i16;
    U16, 16: u16, i16;
    U64, 64: u64, i64;
    U65, 65: u128, i128;
    U100, 100: u128, i128;
    U128, 128: u128, i128;
  }
  fixed_hex_round_trips::<Uint<U12, [u8; 2]>, u16>(12, false);
  fixed_hex_round_trips::<Int<U12, [u8; 2]>, i16>(12, true);
  fixed_hex_round_trips::<Int<U65, [u64; 2]>, i128>(65, true);
  fixed_hex_round_trips::<NonZeroInt<U100, [u32; 4]>, i128>(100, true);
}

/// Checks [`fixed_hex_round_trip`] at the value in `T`, above 128 bits, of each pattern of
/// `patterns`, read as the integer `I` of the same width, whose text has the bits of the pattern; a
/// non-zero `T` holds every value but zero.
#[inline(never)]
fn fixed_hex_texts<T, I>(patterns: &[Vec<bool>])
where
  T: CheckedCastFrom<I> + FixedHex + LowerHex + PartialEq + Debug,
  I: Patterned,
{
  for p in patterns {
    match T::checked_cast_from(I::from_pattern(p)) {
      Some(value) => fixed_hex_round_trip(value, &fixed_hex_text_of_pattern(p)),
      None => assert!(p.iter().all(|&bit| !bit), "{}", type_name::<T>()),
    }
  }
}

/// Checks [`fixed_hex_texts`] at the four integer types of `$bits` bits, the width `$width`, above
/// 128 bits, in the default storage, at the patterns of `text_patterns` in `$words` words of 64
/// bits.
macro_rules! fixed_hex_texts {
  ($($width:ty, $bits:literal, $words:literal;)+) => {$({
    let patterns = text_patterns::<$words>($bits);
    fixed_hex_texts::<Uint<$width>, Uint<$width>>(&patterns);
    fixed_hex_texts::<Int<$width>, Int<$width>>(&patterns);
    fixed_hex_texts::<NonZeroUint<$width>, Uint<$width>>(&patterns);
    fixed_hex_texts::<NonZeroInt<$width>, Int<$width>>(&patterns);
  })+};
}

/// The text of `fixed_hex` at the four integer types of 129, 256 and 1024 bits.
#[inline(never)]
fn fixed_hexes_above_128_bits() {
  fixed_hex_texts! {
    U129, 129, 3;
    U256, 256, 4;
    U1024, 1024, 16;
  }
}

/// The text of `fixed_hex` at the four integer types of 16384 bits, in an optimized build.
#[inline(never)]
fn fixed_hexes_of_16384_bits() {
  fixed_hex_texts! {
    U16384, 16384, 256;
  }
}

/// What `T` reads with `fixed_hex` from the JSON string `text`, or the message of the error.
#[inline(never)]
fn fixed_hex_from<T: FixedHex>(text: &str) -> Result<T, String> {
  serde_json::from_str::<with_fixed_hex::Required<T>>(&format!("\"{text}\""))
    .map(|with_fixed_hex::Required(value)| value)
    .map_err(|error| json_message(&error))
}

/// Asserts that the non-zero `T` rejects with `fixed_hex` `zero`, its text of zero, with std's text
/// of zero.
#[inline(never)]
fn fixed_hex_rejects_zero<T: FixedHex + Debug>(zero: &str) {
  let error = fixed_hex_from::<T>(zero).unwrap_err();
  let expected = parse_error::<NonZero<u8>>("0");
  assert_eq!(error, expected, "{zero} into {}", type_name::<T>());
}

/// Checks [`fixed_hex_round_trip`] at the smallest value, zero, one and the largest value of each
/// native integer `$native`, and at the smallest and the largest value of its `NonZero`, which
/// rejects zero.
macro_rules! native_fixed_hexes {
  ($($native:ident),+ $(,)?) => {$(
    for value in [$native::MIN, 0, 1, $native::MAX] {
      fixed_hex_round_trip(value, &fixed_hex_text(value as u128, $native::BITS));
    }
    for value in [NonZero::<$native>::MIN, NonZero::<$native>::MAX] {
      fixed_hex_round_trip(value, &fixed_hex_text(value.get() as u128, $native::BITS));
    }
    fixed_hex_rejects_zero::<NonZero<$native>>(&fixed_hex_text(0, $native::BITS));
  )+};
}

/// The text of `fixed_hex` at the native integers and their `NonZero`.
#[inline(never)]
fn fixed_hexes_of_the_native_integers() {
  native_fixed_hexes!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);
}

/// `fixed_hex` writes `0x` and two lowercase hexadecimal digits for each byte of the width, the
/// `Bits` bits of the value zero-padded, as a string in JSON and in postcard, and reads it back
/// from both: at the four integer types of 0, 1, 4, 7, 8, 12, 16, 64, 65, 100, 128, 129, 256 and
/// 1024 bits, in an optimized build also 16384 bits, and at the native integers and their
/// `NonZero`. A negative `Int` writes the pattern of its bits, so that `Int<U12>` writes -1 as
/// `0x0fff`; zero bits write `0x`; and a non-zero type rejects zero.
#[test]
fn fixed_hex_writes_two_digits_for_each_byte_of_the_width() {
  fixed_hexes_up_to_128_bits();
  fixed_hexes_above_128_bits();
  if tier().is_full() {
    fixed_hexes_of_16384_bits();
  }
  fixed_hexes_of_the_native_integers();
  fixed_hex_round_trip(Int::<U12>::new(-1), "0x0fff");
  fixed_hex_round_trip(Int::<U12>::MIN, "0x0800");
  fixed_hex_round_trip(Uint::<U12>::MAX, "0x0fff");
  fixed_hex_round_trip(i1::new(-1), "0x01");
  fixed_hex_round_trip(-1i64, &format!("0x{}", "f".repeat(16)));
  fixed_hex_round_trip(255u16, "0x00ff");
  fixed_hex_round_trip(u0::ZERO, "0x");
  fixed_hex_round_trip(i0::ZERO, "0x");
  fixed_hex_rejects_zero::<NonZeroUint<U12>>("0x0000");
  fixed_hex_rejects_zero::<NonZeroInt<U0>>("0x");
  fixed_hex_rejects_zero::<NonZeroInt<U256, [u8; 32]>>(&format!("0x{}", "0".repeat(64)));
}

/// `fixed_hex` at `Uint` of 262128 bits, the widest width whose text a formatting width holds, in
/// 4096 limbs of 64 bits: zero and the largest value.
#[inline(never)]
fn fixed_hex_of_262128_bits() {
  type Wide = Uint<Diff<U262144, U16>, [u64; 4096]>;
  fixed_hex_round_trip(Wide::ZERO, &format!("0x{}", "0".repeat(65532)));
  fixed_hex_round_trip(Wide::MAX, &format!("0x{}", "f".repeat(65532)));
}

/// `fixed_hex` at `Uint` of 262129 bits, whose first byte holds one bit, in 4096 limbs of 64 bits:
/// zero and the largest value.
#[inline(never)]
fn fixed_hex_of_262129_bits() {
  type Wide = Uint<Diff<U262144, U15>, [u64; 4096]>;
  fixed_hex_round_trip(Wide::ZERO, &format!("0x{}", "0".repeat(65534)));
  fixed_hex_round_trip(Wide::MAX, &format!("0x01{}", "f".repeat(65532)));
}

/// `fixed_hex` at `Uint` of 262144 bits, in 4096 limbs of 64 bits: zero and the largest value.
#[inline(never)]
fn fixed_hex_of_262144_bits() {
  type Wide = Uint<U262144, [u64; 4096]>;
  fixed_hex_round_trip(Wide::ZERO, &format!("0x{}", "0".repeat(65536)));
  fixed_hex_round_trip(Wide::MAX, &format!("0x{}", "f".repeat(65536)));
}

/// `fixed_hex` at `Int` of 262144 bits, in 4096 limbs of 64 bits: -1 and the smallest value.
#[inline(never)]
fn fixed_hex_of_262144_signed_bits() {
  type Wide = Int<U262144, [u64; 4096]>;
  let minus_one = Wide::checked_cast_from(-1i8).unwrap();
  fixed_hex_round_trip(minus_one, &format!("0x{}", "f".repeat(65536)));
  fixed_hex_round_trip(Wide::MIN, &format!("0x8{}", "0".repeat(65535)));
}

/// `fixed_hex` writes the digits of the words of a value itself, so that it writes and reads every
/// width: `Uint` of 262128 bits, the widest whose text a formatting width holds and equals, and of
/// 262129 and 262144 bits, whose text is longer than `u16::MAX`, the limit of a formatting width,
/// at zero and the largest value, and `Int` of 262144 bits at -1 and its smallest value.
#[test]
fn fixed_hex_writes_widths_beyond_a_formatting_width() {
  fixed_hex_of_262128_bits();
  fixed_hex_of_262129_bits();
  fixed_hex_of_262144_bits();
  fixed_hex_of_262144_signed_bits();
}

/// `fixed_hex` reads `0x` or `0X` and exactly two hexadecimal digits of either case for each byte
/// of the width, as the pattern of the `Bits` bits, in two's complement for `Int`; the bits of the
/// first byte above the width are too large for both signs; and zero bits read `0x` alone.
#[test]
fn fixed_hex_reads_the_pattern_of_the_bits_of_the_width() {
  assert_eq!(
    fixed_hex_from::<Int<U12>>("0X0FFF"),
    Ok(Int::<U12>::new(-1))
  );
  assert_eq!(fixed_hex_from::<Uint<U12>>("0X0FFF"), Ok(Uint::<U12>::MAX));
  assert_eq!(fixed_hex_from::<Int<U12>>("0x07Ff"), Ok(Int::<U12>::MAX));
  assert_eq!(fixed_hex_from::<Int<U12>>("0x0800"), Ok(Int::<U12>::MIN));
  let too_large = parse_error::<u8>("256");
  for text in ["0x1fff", "0xffff", "0x1000", "0xf000"] {
    let unsigned = fixed_hex_from::<Uint<U12>>(text).unwrap_err();
    assert_eq!(unsigned, too_large, "{text}");
    let signed = fixed_hex_from::<Int<U12>>(text).unwrap_err();
    assert_eq!(signed, too_large, "{text}");
  }
  assert_eq!(fixed_hex_from::<i1>("0x01"), Ok(i1::new(-1)));
  assert_eq!(fixed_hex_from::<i1>("0x02"), Err(too_large.clone()));
  assert_eq!(fixed_hex_from::<Int<U8>>("0xff"), Ok(Int::<U8>::new(-1)));
  assert_eq!(fixed_hex_from::<i8>("0x80"), Ok(i8::MIN));
  assert_eq!(fixed_hex_from::<Uint<U9>>("0x01ff"), Ok(Uint::<U9>::MAX));
  assert_eq!(fixed_hex_from::<Uint<U9>>("0x0200"), Err(too_large));
  let none = "expected 0 hexadecimal digits".to_string();
  assert_eq!(fixed_hex_from::<u0>("0x"), Ok(u0::ZERO));
  assert_eq!(fixed_hex_from::<i0>("0X"), Ok(i0::ZERO));
  assert_eq!(fixed_hex_from::<u0>("0x0"), Err(none.clone()));
  assert_eq!(fixed_hex_from::<i0>("0x00"), Err(none));
  assert_eq!(fixed_hex_from::<u0>(""), Err(NO_HEX_PREFIX.to_string()));
}

/// Asserts that `T`, whose width has `count` hexadecimal digits, two or more, rejects with
/// `fixed_hex` each text that breaks a rule, with the message of the first that it breaks: one and
/// two digits fewer or more, a text without the prefix, the empty string included, and `+`, `g` or
/// a character outside ASCII of two bytes in place of digits, while such a character in place of
/// one digit makes a wrong count; and every input but a string with the fixed message.
#[inline(never)]
fn fixed_hex_rejections<T: FixedHex + Debug>(count: usize) {
  let name = type_name::<T>();
  let zeros = |n: usize| "0".repeat(n);
  let wrong_count = format!("expected {count} hexadecimal digits");
  for n in [count - 2, count - 1, count + 1, count + 2] {
    let text = format!("0x{}", zeros(n));
    let error = fixed_hex_from::<T>(&text).unwrap_err();
    assert_eq!(error, wrong_count, "{text} into {name}");
  }
  let non_ascii = format!("0x{}\u{665}", zeros(count - 1));
  let error = fixed_hex_from::<T>(&non_ascii).unwrap_err();
  assert_eq!(error, wrong_count, "{name}");
  for text in [
    String::new(),
    zeros(count),
    format!("x{}", zeros(count)),
    format!("0{}", zeros(count)),
  ] {
    let error = fixed_hex_from::<T>(&text).unwrap_err();
    assert_eq!(error, NO_HEX_PREFIX, "{text:?} into {name}");
  }
  let invalid = parse_error::<u8>("g");
  for text in [
    format!("0x+{}", zeros(count - 1)),
    format!("0x{}g", zeros(count - 1)),
    format!("0x{}\u{665}", zeros(count - 2)),
  ] {
    let error = fixed_hex_from::<T>(&text).unwrap_err();
    assert_eq!(error, invalid, "{text:?} into {name}");
  }
  for json in ["5", "0", "true", "null", "[]", "{}"] {
    let error = serde_json::from_str::<with_fixed_hex::Required<T>>(json).unwrap_err();
    assert_eq!(json_message(&error), FIXED_HEX, "{json} into {name}");
  }
  for token in [Token::U8(5), Token::Bool(true), Token::Bytes(b"0x05")] {
    let tokens = [Token::NewtypeStruct { name: "Required" }, token];
    assert_de_tokens_error::<with_fixed_hex::Required<T>>(&tokens, FIXED_HEX);
  }
  let read: Result<T, de::value::Error> =
    fixed_hex::required::deserialize(5i64.into_deserializer());
  assert_eq!(read.unwrap_err().to_string(), FIXED_HEX, "{name}");
  let map = de::value::MapDeserializer::new([(NUMBER_TOKEN, "5")].into_iter());
  let read: Result<T, de::value::Error> = fixed_hex::required::deserialize(map);
  assert_eq!(read.unwrap_err().to_string(), FIXED_HEX, "{name}");
}

/// `fixed_hex` rejects each text that breaks a rule of its grammar with the message of the first
/// that it breaks: the prefix, the number of digits, which counts the bytes of a character outside
/// ASCII, and then a byte that is not a hexadecimal digit, with std's text of an invalid digit.
/// Every input but a string gets the fixed message, an integer included.
#[test]
fn fixed_hex_rejects_each_broken_rule_with_its_message() {
  fixed_hex_rejections::<Uint<U12>>(4);
  fixed_hex_rejections::<Int<U12>>(4);
  fixed_hex_rejections::<i8>(2);
  fixed_hex_rejections::<Int<U100, [u32; 4]>>(26);
  fixed_hex_rejections::<u256>(64);
  fixed_hex_rejections::<NonZero<u64>>(16);
}

/// The leaf `optional` of `fixed_hex`: `None` is JSON's `null` and postcard's `[0]`, and `Some` is
/// postcard's `[1]` followed by the text; a missing field reads as `None` with `default` and is
/// required without it; `null` reads as `None`, also through serde's buffer; an empty csv field is
/// `None`; and every other input gets the fixed message. The leaf `required` reads through serde's
/// buffer, in an untagged enum and a flattened struct.
#[test]
fn fixed_hex_in_options_and_serde_buffer() {
  options_round_trip(
    with_fixed_hex::Optional::<Int<U12>>(None),
    with_fixed_hex::Optional(Some(Int::<U12>::new(-1))),
    "\"0x0fff\"",
    &postcard::to_stdvec("0x0fff").unwrap(),
  );
  let max = format!("\"0x{}\"", "f".repeat(64));
  option_fields(
    [
      with_fixed_hex::Defaulted::<u256> { value: None },
      with_fixed_hex::Defaulted {
        value: Some(u256::MAX),
      },
    ],
    [
      with_fixed_hex::Present::<u256> { value: None },
      with_fixed_hex::Present {
        value: Some(u256::MAX),
      },
    ],
    &max,
  );
  let read: Result<Option<i8>, de::value::Error> = fixed_hex::optional::deserialize(CsvField(""));
  assert_eq!(read.unwrap(), None);
  let read: Result<Option<i8>, de::value::Error> =
    fixed_hex::optional::deserialize(CsvField("0xff"));
  assert_eq!(read.unwrap(), Some(-1));
  for json in ["255", "true", "[]", "{}"] {
    let error = serde_json::from_str::<with_fixed_hex::Optional<i8>>(json).unwrap_err();
    assert_eq!(json_message(&error), FIXED_HEX, "{json}");
  }
  let error = serde_json::from_str::<with_fixed_hex::Optional<i8>>("\"ff\"").unwrap_err();
  assert_eq!(json_message(&error), NO_HEX_PREFIX);
  for result in through_the_buffer::<with_fixed_hex::Field<Int<U12>>>(r#"{"value":"0x0fff"}"#) {
    assert_eq!(
      result,
      Ok(with_fixed_hex::Field {
        value: Int::<U12>::new(-1)
      })
    );
  }
  // -1 at 100 bits: the four bits of the first byte, then twelve bytes.
  let json = format!(r#"{{"value":"0x0f{}"}}"#, "f".repeat(24));
  for result in through_the_buffer::<with_fixed_hex::Field<Int<U100>>>(&json) {
    assert_eq!(
      result,
      Ok(with_fixed_hex::Field {
        value: Int::<U100>::new(-1)
      })
    );
  }
}

/// Checks that `T` writes `value` with `typed` as the default writes it, in JSON and in postcard,
/// and reads it back from both, by the type of its class.
#[inline(never)]
fn typed_round_trip<T>(value: T)
where
  T: Typed + Serialize + PartialEq + Debug,
{
  let name = type_name::<T>();
  let json = serde_json::to_string(&value).unwrap();
  let bytes = postcard::to_stdvec(&value).unwrap();
  let value = with_typed::Required(value);
  assert_eq!(serde_json::to_string(&value).unwrap(), json, "{name}");
  assert_eq!(
    postcard::to_stdvec(&value).unwrap(),
    bytes,
    "{json} in {name}"
  );
  let from_json = serde_json::from_str::<with_typed::Required<T>>(&json).unwrap();
  assert_eq!(from_json, value, "{json} into {name}");
  let from_bytes = postcard::from_bytes::<with_typed::Required<T>>(&bytes).unwrap();
  assert_eq!(from_bytes, value, "{json} into {name}");
}

/// Checks [`typed_round_trip`] at the value in `T` of each of `natives`, which `T` holds.
#[inline(never)]
fn typed_round_trips<T, N>(natives: &[N])
where
  T: CheckedCastFrom<N> + Typed + Serialize + PartialEq + Debug,
  N: Native,
{
  for &native in natives {
    typed_round_trip(T::checked_cast_from(native).expect("a value of the width"));
  }
}

/// Checks [`typed_round_trips`] at the four integer types of `$bits` bits, the width `$width`, in
/// the default storage, whose class has the native integers `$unsigned` and `$signed`, at the
/// values of [`helper_natives`], and of [`helper_nonzero_natives`] for the non-zero types.
macro_rules! typed_forms {
  ($($width:ty, $bits:literal: $unsigned:ident, $signed:ident;)+) => {$({
    let unsigned: Vec<$unsigned> = helper_natives($bits, false);
    let signed: Vec<$signed> = helper_natives($bits, true);
    typed_round_trips::<Uint<$width>, $unsigned>(&unsigned);
    typed_round_trips::<Int<$width>, $signed>(&signed);
    let unsigned: Vec<$unsigned> = helper_nonzero_natives($bits, false);
    let signed: Vec<$signed> = helper_nonzero_natives($bits, true);
    typed_round_trips::<NonZeroUint<$width>, $unsigned>(&unsigned);
    typed_round_trips::<NonZeroInt<$width>, $signed>(&signed);
  })+};
}

/// The forms of `typed` at the four integer types of up to 128 bits, at the ends of each class.
#[inline(never)]
fn typed_forms_up_to_128_bits() {
  typed_forms! {
    U0, 0: u8, i8;
    U1, 1: u8, i8;
    U8, 8: u8, i8;
    U9, 9: u16, i16;
    U16, 16: u16, i16;
    U17, 17: u32, i32;
    U32, 32: u32, i32;
    U33, 33: u64, i64;
    U64, 64: u64, i64;
    U65, 65: u128, i128;
    U128, 128: u128, i128;
  }
}

/// Checks [`typed_round_trip`] at the value in `T`, above 128 bits, of each pattern of `patterns`,
/// read as the integer `I` of the same width; a non-zero `T` holds every value but zero.
#[inline(never)]
fn typed_texts<T, I>(patterns: &[Vec<bool>])
where
  T: CheckedCastFrom<I> + Typed + Serialize + PartialEq + Debug,
  I: Patterned,
{
  for p in patterns {
    match T::checked_cast_from(I::from_pattern(p)) {
      Some(value) => typed_round_trip(value),
      None => assert!(p.iter().all(|&bit| !bit), "{}", type_name::<T>()),
    }
  }
}

/// The forms of `typed` at the four integer types of 129 bits and at `Uint` and `Int` of 256 bits
/// in arrays.
#[inline(never)]
fn typed_forms_above_128_bits() {
  let patterns = text_patterns::<3>(129);
  typed_texts::<Uint<U129>, Uint<U129>>(&patterns);
  typed_texts::<Int<U129>, Int<U129>>(&patterns);
  typed_texts::<NonZeroUint<U129>, Uint<U129>>(&patterns);
  typed_texts::<NonZeroInt<U129>, Int<U129>>(&patterns);
  let patterns = text_patterns::<4>(256);
  typed_texts::<Uint<U256, [u8; 32]>, Uint<U256, [u8; 32]>>(&patterns);
  typed_texts::<Int<U256, [u32; 8]>, Int<U256, [u32; 8]>>(&patterns);
}

/// What `T` reads with `typed` from `json`, or the message of the error.
#[inline(never)]
fn typed_from<T: Typed>(json: &str) -> Result<T, String> {
  serde_json::from_str::<with_typed::Required<T>>(json)
    .map(|with_typed::Required(value)| value)
    .map_err(|error| json_message(&error))
}

/// Asserts that `T`, of 0 to 64 bits, reads with `typed` the JSON number 5 alone, as the native
/// integer of its class does, and rejects the string `"5"` and every other value with the fixed
/// message.
#[inline(never)]
fn typed_reads_numbers<T>()
where
  T: Typed + CheckedCastFrom<u8> + PartialEq + Debug,
{
  let name = type_name::<T>();
  let five = T::checked_cast_from(5u8).unwrap();
  assert_eq!(typed_from::<T>("5"), Ok(five), "{name}");
  for json in ["\"5\"", "1.5", "true", "null", "[]", "{}"] {
    assert_eq!(
      typed_from::<T>(json).unwrap_err(),
      FIXED,
      "{json} into {name}"
    );
  }
}

/// Asserts that `T`, of 65 to 128 bits, reads with `typed` the JSON number 5, and rejects the
/// string `"5"` with the error of its native integer `N`, which serde_json builds for the request
/// of a 128-bit integer.
#[inline(never)]
fn typed_reads_128_bit_numbers<T, N>()
where
  T: Typed + CheckedCastFrom<u8> + PartialEq + Debug,
  N: DeserializeOwned + Debug,
{
  let name = type_name::<T>();
  let five = T::checked_cast_from(5u8).unwrap();
  assert_eq!(typed_from::<T>("5"), Ok(five), "{name}");
  let error = serde_json::from_str::<with_typed::Required<T>>("\"5\"").unwrap_err();
  let native = serde_json::from_str::<N>("\"5\"").unwrap_err();
  assert_eq!(error.to_string(), native.to_string(), "{name}");
}

/// Asserts that `T`, above 128 bits, reads with `typed` the JSON string `"5"` alone, and rejects
/// the number 5 and every other value with the fixed message.
#[inline(never)]
fn typed_reads_strings<T>()
where
  T: Typed + CheckedCastFrom<u8> + PartialEq + Debug,
{
  let name = type_name::<T>();
  let five = T::checked_cast_from(5u8).unwrap();
  assert_eq!(typed_from::<T>("\"5\""), Ok(five), "{name}");
  for json in ["5", "-5", "1.5", "true", "null", "[]", "{}"] {
    assert_eq!(
      typed_from::<T>(json).unwrap_err(),
      FIXED,
      "{json} into {name}"
    );
  }
}

/// `typed` writes what the default writes, in JSON and in postcard, at every class, in arrays of
/// limbs too, and at the non-zero integers, and reads it back by the type of the class: in JSON a
/// number and no string up to 128 bits, as the native integer of the class reads, and a string and
/// no number above, where serde_json's own error of the request of a 128-bit integer passes
/// through.
#[test]
fn typed_writes_the_form_of_the_defaults_and_reads_it_alone() {
  typed_forms_up_to_128_bits();
  typed_forms_above_128_bits();
  typed_reads_numbers::<u7>();
  typed_reads_numbers::<i7>();
  typed_reads_numbers::<Uint<U16, [u8; 2]>>();
  typed_reads_numbers::<Int<U64>>();
  typed_reads_numbers::<NonZeroUint<U33>>();
  typed_reads_128_bit_numbers::<Uint<U100>, u128>();
  typed_reads_128_bit_numbers::<Int<U128, [u64; 2]>, i128>();
  typed_reads_strings::<u256>();
  typed_reads_strings::<Int<U129, [u8; 17]>>();
  typed_reads_strings::<NonZeroUint<U1024>>();
}

/// Asserts that `T` asks with `typed` every format, human-readable or not, for what its native
/// integer `N` asks a format that is not human-readable for.
#[inline(never)]
fn typed_asks_as_its_class<T, N>()
where
  T: Typed + Debug,
  N: DeserializeOwned + Debug,
{
  let name = type_name::<T>();
  let class = error_from::<N>(Probe::Record(false));
  for readable in [true, false] {
    let read: Result<T, de::value::Error> = typed::required::deserialize(Probe::Record(readable));
    assert_eq!(read.unwrap_err().to_string(), class, "{name}");
  }
}

/// What `T` reads with `typed` from a human-readable format without `deserialize_any`, which hands
/// over 5 for the requests of the integers up to 64 bits and of a string, or the text of the
/// error.
#[inline(never)]
fn typed_without_any<T: Typed>() -> Result<T, String> {
  typed::required::deserialize(Probe::WithoutAny)
    .map_err(|error: de::value::Error| error.to_string())
}

/// `typed` asks every format for the method of the class, whether it is human-readable or not:
/// the method of the native integer of the class up to 128 bits and that of a string above. So in
/// a human-readable format without `deserialize_any`, as serde-json-core, it reads 0 to 64 bits
/// and above 128 bits, where the defaults read nothing, and fails from 65 to 128 bits as the
/// native `u128` and `i128` do.
#[test]
fn typed_asks_every_format_for_the_class() {
  typed_asks_as_its_class::<u0, u8>();
  typed_asks_as_its_class::<i7, i8>();
  typed_asks_as_its_class::<Uint<U9>, u16>();
  typed_asks_as_its_class::<Int<U16>, i16>();
  typed_asks_as_its_class::<Uint<U17>, u32>();
  typed_asks_as_its_class::<Int<U32>, i32>();
  typed_asks_as_its_class::<Uint<U33>, u64>();
  typed_asks_as_its_class::<Int<U64>, i64>();
  typed_asks_as_its_class::<Uint<U65>, u128>();
  typed_asks_as_its_class::<Int<U128>, i128>();
  typed_asks_as_its_class::<NonZeroUint<U7>, u8>();
  for readable in [true, false] {
    let read: Result<u256, de::value::Error> =
      typed::required::deserialize(Probe::Record(readable));
    assert_eq!(read.unwrap_err().to_string(), "deserialize_str");
    let read: Result<Int<U129>, de::value::Error> =
      typed::required::deserialize(Probe::Record(readable));
    assert_eq!(read.unwrap_err().to_string(), "deserialize_str");
  }
  assert_eq!(typed_without_any::<u7>(), Ok(u7::new(5)));
  assert_eq!(typed_without_any::<i7>(), Ok(i7::new(5)));
  assert_eq!(typed_without_any::<Uint<U64>>(), Ok(Uint::<U64>::new(5)));
  assert_eq!(typed_without_any::<Int<U64>>(), Ok(Int::<U64>::new(5)));
  assert_eq!(typed_without_any::<u256>(), Ok(u256::from(5u8)));
  assert_eq!(typed_without_any::<i256>(), Ok(i256::from(5i8)));
  let without = Probe::WithoutAny;
  assert_eq!(
    typed_without_any::<Uint<U100>>(),
    Err(error_from::<u128>(without))
  );
  assert_eq!(
    typed_without_any::<Int<U100>>(),
    Err(error_from::<i128>(without))
  );
  assert_eq!(error_from::<u7>(without), "any is unsupported");
  assert_eq!(error_from::<u256>(without), "any is unsupported");
}

/// What `T` reads from the deserializer that `deserializer` makes with `typed` and with its
/// default, or the texts of the errors.
#[inline(never)]
fn typed_and_default<T, D>(deserializer: impl Fn() -> D) -> [Result<T, String>; 2]
where
  T: Typed + DeserializeOwned,
  D: Deserializer<'static, Error = de::value::Error>,
{
  let text = |error: de::value::Error| error.to_string();
  [
    typed::required::deserialize(deserializer()).map_err(text),
    T::deserialize(deserializer()).map_err(text),
  ]
}

/// The fixed message of the defaults and of `typed`, as the error of a reading of `T`.
fn fixed<T>() -> Result<T, String> {
  Err(FIXED.to_string())
}

/// serde's deserializers hand their value over for every request, through `deserialize_any`: the
/// form that `typed` does not take gets its fixed message, and the defaults read it, a string up to
/// 128 bits, an integer above 128 bits, and serde_json's number token at every class.
#[test]
fn typed_rejects_the_other_form_that_the_defaults_read() {
  let token = || de::value::MapDeserializer::new([(NUMBER_TOKEN, "5")].into_iter());
  assert_eq!(
    typed_and_default::<u7, _>(|| "5".into_deserializer()),
    [fixed(), Ok(u7::new(5))]
  );
  assert_eq!(typed_and_default::<u7, _>(token), [fixed(), Ok(u7::new(5))]);
  let five = Uint::<U100>::new(5);
  assert_eq!(
    typed_and_default::<Uint<U100>, _>(|| "5".into_deserializer()),
    [fixed(), Ok(five)]
  );
  assert_eq!(
    typed_and_default::<Uint<U100>, _>(token),
    [fixed(), Ok(five)]
  );
  let five = u256::from(5u8);
  assert_eq!(
    typed_and_default::<u256, _>(|| 5u64.into_deserializer()),
    [fixed(), Ok(five)]
  );
  assert_eq!(typed_and_default::<u256, _>(token), [fixed(), Ok(five)]);
}

/// The leaf `optional` of `typed`: `None` is JSON's `null` and postcard's `[0]`, and `Some` is the
/// form of the defaults after postcard's `[1]`; a missing field reads as `None` with `default` and
/// is required without it; `null` reads as `None`, also through serde's buffer; an empty csv field
/// is `None`; and every other input gets the fixed message. The leaf `required` reads through
/// serde's buffer what the default reads there, and every value from csv's field, of which it asks
/// for a string above 128 bits.
#[test]
fn typed_in_options_serde_buffer_and_csv() {
  options_round_trip(
    with_typed::Optional::<u7>(None),
    with_typed::Optional(Some(u7::new(5))),
    "5",
    &[5],
  );
  options_round_trip(
    with_typed::Optional::<Int<U100>>(None),
    with_typed::Optional(Some(Int::<U100>::new(-5))),
    "-5",
    &postcard::to_stdvec(&-5i128).unwrap(),
  );
  options_round_trip(
    with_typed::Optional::<u256>(None),
    with_typed::Optional(Some(u256::from(5u8))),
    "\"5\"",
    &postcard::to_stdvec("5").unwrap(),
  );
  option_fields(
    [
      with_typed::Defaulted::<u7> { value: None },
      with_typed::Defaulted {
        value: Some(u7::new(5)),
      },
    ],
    [
      with_typed::Present::<u7> { value: None },
      with_typed::Present {
        value: Some(u7::new(5)),
      },
    ],
    "5",
  );
  let read: Result<Option<u7>, de::value::Error> = typed::optional::deserialize(CsvField(""));
  assert_eq!(read.unwrap(), None);
  let read: Result<Option<u7>, de::value::Error> = typed::optional::deserialize(CsvField("5"));
  assert_eq!(read.unwrap(), Some(u7::new(5)));
  for json in ["\"5\"", "true", "[]", "{}"] {
    let error = serde_json::from_str::<with_typed::Optional<u7>>(json).unwrap_err();
    assert_eq!(json_message(&error), FIXED, "{json}");
  }
  for result in through_the_buffer::<with_typed::Field<u7>>(r#"{"value":5}"#) {
    assert_eq!(result, Ok(with_typed::Field { value: u7::new(5) }));
  }
  for result in through_the_buffer::<with_typed::Field<u256>>(r#"{"value":"5"}"#) {
    assert_eq!(
      result,
      Ok(with_typed::Field {
        value: u256::from(5u8)
      })
    );
  }
  for result in through_the_buffer::<with_typed::Field<u7>>(r#"{"value":"5"}"#) {
    assert!(result.is_err());
  }
  for result in through_the_buffer::<with_typed::Field<u256>>(r#"{"value":5}"#) {
    assert!(result.is_err());
  }
  for result in through_the_buffer::<with_typed::Field<Uint<U100>>>(r#"{"value":5}"#) {
    assert!(result.is_err());
  }
  for value in [u256::from(u128::MAX) + u256::from(1u8), u256::MAX] {
    let text = value.to_string();
    let read: Result<u256, de::value::Error> = typed::required::deserialize(CsvField(&text));
    assert_eq!(read.unwrap(), value);
  }
}

/// The fixed message of `be_bytes` and `le_bytes`.
const BYTE_ARRAY: &str = "expected an array of bytes";

/// The message of the bytes helpers for bytes of another number than `count`, the bytes of the
/// width: `expected 1 byte` for one, and `expected {count} bytes` for every other count.
fn wrong_byte_count(count: u64) -> String {
  if count == 1 {
    "expected 1 byte".to_string()
  } else {
    format!("expected {count} bytes")
  }
}

/// Checks that the container `C` writes its value as `bytes`, serde's bytes: in JSON an array of
/// numbers and in postcard the length and the bytes; and reads it back from both, from JSON as a
/// sequence and from postcard as a byte string.
#[inline(never)]
fn written_as_bytes<C>(container: C, bytes: &[u8])
where
  C: Serialize + DeserializeOwned + PartialEq + Debug,
{
  let name = type_name::<C>();
  let json = serde_json::to_string(&container).unwrap();
  assert_eq!(json, serde_json::to_string(bytes).unwrap(), "{name}");
  let mut expected = postcard::to_stdvec(&bytes.len()).unwrap();
  expected.extend_from_slice(bytes);
  let written = postcard::to_stdvec(&container).unwrap();
  assert_eq!(written, expected, "{json} in {name}");
  let from_json = serde_json::from_str::<C>(&json).unwrap();
  assert_eq!(from_json, container, "{json} into {name}");
  let from_bytes = postcard::from_bytes::<C>(&written).unwrap();
  assert_eq!(from_bytes, container, "{json} into {name}");
}

/// Checks [`written_as_bytes`] at `value` with `le_bytes`, whose bytes are `le`, least significant
/// first, and with `be_bytes`, whose bytes are those of `le` in reverse order.
#[inline(never)]
fn bytes_round_trip<T>(value: T, le: &[u8])
where
  T: BeBytes + LeBytes + Clone + PartialEq + Debug,
{
  let be: Vec<u8> = le.iter().rev().copied().collect();
  written_as_bytes(with_be_bytes::Required(value.clone()), &be);
  written_as_bytes(with_le_bytes::Required(value), le);
}

/// Checks [`bytes_round_trip`] at the value in `T`, of `bits` bits, at most 128, of each pattern of
/// [`helper_samples`], read as the native integer `N` of the class of the width, whose bytes are
/// the low bytes of the pattern, those that `to_le_bytes` gives of a native integer of 128 bits; a
/// non-zero `T` holds every value but zero.
#[inline(never)]
fn bytes_round_trips<T, N>(bits: u32, signed: bool)
where
  T: CheckedCastFrom<N> + BeBytes + LeBytes + Clone + PartialEq + Debug,
  N: Native,
{
  let count = bits.div_ceil(8) as usize;
  for pattern in helper_samples(bits, signed) {
    match T::checked_cast_from(N::of_pattern(pattern)) {
      Some(value) => bytes_round_trip(value, &pattern.to_le_bytes()[..count]),
      None => assert_eq!(pattern, 0, "{}", type_name::<T>()),
    }
  }
}

/// Checks [`bytes_round_trips`] at the four integer types of `$bits` bits, the width `$width`, in
/// the default storage, whose class has the native integers `$unsigned` and `$signed`.
macro_rules! bytes_forms {
  ($($width:ty, $bits:literal: $unsigned:ident, $signed:ident;)+) => {$(
    bytes_round_trips::<Uint<$width>, $unsigned>($bits, false);
    bytes_round_trips::<Int<$width>, $signed>($bits, true);
    bytes_round_trips::<NonZeroUint<$width>, $unsigned>($bits, false);
    bytes_round_trips::<NonZeroInt<$width>, $signed>($bits, true);
  )+};
}

/// The bytes of `be_bytes` and `le_bytes` at the four integer types of 0, 1, 7, 8, 12, 16, 64, 65,
/// 100 and 128 bits, in the default storage, and at four of them in arrays.
#[inline(never)]
fn bytes_up_to_128_bits() {
  bytes_forms! {
    U0, 0: u8, i8;
    U1, 1: u8, i8;
    U7, 7: u8, i8;
    U8, 8: u8, i8;
    U12, 12: u16, i16;
    U16, 16: u16, i16;
    U64, 64: u64, i64;
    U65, 65: u128, i128;
    U100, 100: u128, i128;
    U128, 128: u128, i128;
  }
  bytes_round_trips::<Uint<U12, [u8; 2]>, u16>(12, false);
  bytes_round_trips::<Int<U12, [u16; 1]>, i16>(12, true);
  bytes_round_trips::<Int<U65, [u64; 2]>, i128>(65, true);
  bytes_round_trips::<NonZeroInt<U100, [u32; 4]>, i128>(100, true);
}

/// Checks [`bytes_round_trip`] at the value in `T`, above 128 bits, of each pattern of `patterns`,
/// read as the integer `I` of the same width, whose bytes are those that its `copy_le_bytes_to`
/// writes; a non-zero `T` holds every value but zero.
#[inline(never)]
fn bytes_of_patterns<T, I>(patterns: &[Vec<bool>])
where
  T: CheckedCastFrom<I> + BeBytes + LeBytes + Clone + PartialEq + Debug,
  I: Patterned + IntegerBytes,
{
  for p in patterns {
    let integer = I::from_pattern(p);
    let mut le = vec![0; I::BYTES];
    integer.copy_le_bytes_to(&mut le);
    match T::checked_cast_from(integer) {
      Some(value) => bytes_round_trip(value, &le),
      None => assert!(p.iter().all(|&bit| !bit), "{}", type_name::<T>()),
    }
  }
}

/// Checks [`bytes_of_patterns`] at the four integer types of `$bits` bits, the width `$width`,
/// above 128 bits, in the default storage, at the patterns of `text_patterns` in `$words` words of
/// 64 bits.
macro_rules! bytes_of_patterns {
  ($($width:ty, $bits:literal, $words:literal;)+) => {$({
    let patterns = text_patterns::<$words>($bits);
    bytes_of_patterns::<Uint<$width>, Uint<$width>>(&patterns);
    bytes_of_patterns::<Int<$width>, Int<$width>>(&patterns);
    bytes_of_patterns::<NonZeroUint<$width>, Uint<$width>>(&patterns);
    bytes_of_patterns::<NonZeroInt<$width>, Int<$width>>(&patterns);
  })+};
}

/// The bytes of `be_bytes` and `le_bytes` at the four integer types of 129, 200, 256 and 1024 bits,
/// and at 200 and 256 bits in arrays of other limbs.
#[inline(never)]
fn bytes_above_128_bits() {
  bytes_of_patterns! {
    U129, 129, 3;
    U200, 200, 4;
    U256, 256, 4;
    U1024, 1024, 16;
  }
  let patterns = text_patterns::<4>(200);
  bytes_of_patterns::<Uint<U200, [u8; 25]>, Uint<U200, [u8; 25]>>(&patterns);
  bytes_of_patterns::<Int<U200, [u32; 7]>, Int<U200, [u32; 7]>>(&patterns);
  let patterns = text_patterns::<4>(256);
  bytes_of_patterns::<Uint<U256, [u128; 2]>, Uint<U256, [u128; 2]>>(&patterns);
  bytes_of_patterns::<NonZeroInt<U256, [u16; 16]>, Int<U256, [u16; 16]>>(&patterns);
}

/// The bytes of `be_bytes` and `le_bytes` at the four integer types of 8192 bits, the widest that
/// they take, in an optimized build.
#[inline(never)]
fn bytes_of_8192_bits() {
  bytes_of_patterns! {
    U8192, 8192, 128;
  }
}

/// What `T` reads with `be_bytes` and with `le_bytes` from the JSON text `json`, or the message of
/// each error.
#[inline(never)]
fn bytes_from_json<T: BeBytes + LeBytes>(json: &str) -> [Result<T, String>; 2] {
  let message = |error: serde_json::Error| json_message(&error);
  [
    serde_json::from_str::<with_be_bytes::Required<T>>(json)
      .map(|with_be_bytes::Required(value)| value)
      .map_err(message),
    serde_json::from_str::<with_le_bytes::Required<T>>(json)
      .map(|with_le_bytes::Required(value)| value)
      .map_err(message),
  ]
}

/// What `T` reads from `be`, its bytes most significant first, with `be_bytes`, and from the same
/// bytes in reverse order with `le_bytes`, each from a JSON array and from serde's deserializer of
/// a byte string, or the message of each error.
#[inline(never)]
fn bytes_read<T: BeBytes + LeBytes>(be: &[u8]) -> [Result<T, String>; 4] {
  let le: Vec<u8> = be.iter().rev().copied().collect();
  let message = |error: serde_json::Error| json_message(&error);
  let text = |error: de::value::Error| error.to_string();
  [
    serde_json::from_str::<with_be_bytes::Required<T>>(&serde_json::to_string(be).unwrap())
      .map(|with_be_bytes::Required(value)| value)
      .map_err(message),
    serde_json::from_str::<with_le_bytes::Required<T>>(&serde_json::to_string(&le).unwrap())
      .map(|with_le_bytes::Required(value)| value)
      .map_err(message),
    be_bytes::required::deserialize(de::value::BytesDeserializer::new(be)).map_err(text),
    le_bytes::required::deserialize(de::value::BytesDeserializer::new(&le)).map_err(text),
  ]
}

/// What `T` reads from `be` as [`bytes_read`] reads it, alike in every way, or the message of the
/// error.
#[inline(never)]
fn bytes_value<T>(be: &[u8]) -> Result<T, String>
where
  T: BeBytes + LeBytes + PartialEq + Debug,
{
  let [first, rest @ ..] = bytes_read::<T>(be);
  for result in rest {
    assert_eq!(result, first, "{be:?} into {}", type_name::<T>());
  }
  first
}

/// Checks [`bytes_round_trip`] at the smallest value, zero, one and the largest value of each
/// native integer `$native`, whose bytes are those of its `to_le_bytes`, and at the smallest and
/// the largest value of its `NonZero`, which rejects zero.
macro_rules! native_bytes {
  ($($native:ident),+ $(,)?) => {$(
    for value in [$native::MIN, 0, 1, $native::MAX] {
      bytes_round_trip(value, &value.to_le_bytes());
    }
    for value in [NonZero::<$native>::MIN, NonZero::<$native>::MAX] {
      bytes_round_trip(value, &value.get().to_le_bytes());
    }
    let zero = bytes_value::<NonZero<$native>>(&[0; size_of::<$native>()]);
    assert_eq!(zero, Err(parse_error::<NonZero<u8>>("0")));
  )+};
}

/// The bytes of `be_bytes` and `le_bytes` at the native integers and their `NonZero`.
#[inline(never)]
fn bytes_of_the_native_integers() {
  native_bytes!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);
}

/// `be_bytes` and `le_bytes` write the bytes of a value, most and least significant first, as
/// serde's bytes, a JSON array and in postcard the length and the bytes, and read them back from
/// both: at the four integer types of 0, 1, 7, 8, 12, 16, 64, 65, 100, 128, 129, 200, 256 and 1024
/// bits, in an optimized build also 8192 bits, against the bytes of the native integers and numo's
/// own, and at the native integers and their `NonZero`. A negative `Int` writes its two's
/// complement extended by copies of the sign, zero bits write no bytes, and a non-zero type
/// rejects zero.
#[test]
fn bytes_write_the_bytes_of_the_value_in_either_order() {
  bytes_up_to_128_bits();
  bytes_above_128_bits();
  if tier().is_full() {
    bytes_of_8192_bits();
  }
  bytes_of_the_native_integers();
  bytes_round_trip(Int::<U12>::new(-1), &[255, 255]);
  bytes_round_trip(Int::<U12>::MIN, &[0, 248]);
  bytes_round_trip(Uint::<U12>::MAX, &[255, 15]);
  bytes_round_trip(i1::new(-1), &[255]);
  bytes_round_trip(u0::ZERO, &[]);
  bytes_round_trip(i0::ZERO, &[]);
  let zero = parse_error::<NonZero<u8>>("0");
  assert_eq!(bytes_value::<NonZeroUint<U12>>(&[0, 0]).unwrap_err(), zero);
  assert_eq!(bytes_value::<NonZeroInt<U0>>(&[]).unwrap_err(), zero);
  assert_eq!(
    bytes_value::<NonZeroUint<U0, [u8; 0]>>(&[]).unwrap_err(),
    zero
  );
  let zeros = [0; 32];
  assert_eq!(
    bytes_value::<NonZeroInt<U256, [u8; 32]>>(&zeros).unwrap_err(),
    zero
  );
}

/// `be_bytes` and `le_bytes` read the bytes of the width alone: the bits of the most significant
/// byte above the width are too large for `Uint`, and for `Int` too large or, for the bytes of a
/// negative value, too small, unless they copy the sign; at a width of whole bytes every byte
/// reads; and zero bits read no bytes alone. Both read a byte string, a JSON array, serde_test's
/// bytes and sequence, and a sequence of the integers that toml gives.
#[test]
fn bytes_read_the_bits_of_the_width_alone() {
  let too_large = parse_error::<u8>("256");
  let too_small = parse_error::<i8>("-129");
  assert_eq!(bytes_value::<Uint<U12>>(&[15, 255]), Ok(Uint::<U12>::MAX));
  assert_eq!(bytes_value::<Int<U12>>(&[15, 255]).unwrap_err(), too_large);
  assert_eq!(
    bytes_value::<Int<U12>>(&[255, 255]),
    Ok(Int::<U12>::new(-1))
  );
  assert_eq!(
    bytes_value::<Uint<U12>>(&[255, 255]).unwrap_err(),
    too_large
  );
  assert_eq!(bytes_value::<Uint<U12>>(&[16, 0]).unwrap_err(), too_large);
  assert_eq!(bytes_value::<Int<U12>>(&[7, 255]), Ok(Int::<U12>::MAX));
  assert_eq!(bytes_value::<Int<U12>>(&[8, 0]).unwrap_err(), too_large);
  assert_eq!(bytes_value::<Int<U12>>(&[248, 0]), Ok(Int::<U12>::MIN));
  assert_eq!(bytes_value::<Int<U12>>(&[247, 255]).unwrap_err(), too_small);
  let read = bytes_value::<Int<U12, [u8; 2]>>(&[128, 0]);
  assert_eq!(read.unwrap_err(), too_small);
  assert_eq!(bytes_value::<i1>(&[255]), Ok(i1::new(-1)));
  assert_eq!(bytes_value::<i1>(&[0]), Ok(i1::new(0)));
  assert_eq!(bytes_value::<i1>(&[1]).unwrap_err(), too_large);
  assert_eq!(bytes_value::<i1>(&[254]).unwrap_err(), too_small);
  assert_eq!(
    bytes_value::<Uint<U1, [u16; 1]>>(&[1]),
    Ok(Uint::<U1, [u16; 1]>::MAX)
  );
  let read = bytes_value::<Uint<U1, [u16; 1]>>(&[2]);
  assert_eq!(read.unwrap_err(), too_large);
  // 65 bits have nine bytes, of which the most significant holds one bit of the width.
  let power = Uint::<U65, [u16; 5]>::checked_cast_from(1u128 << 64).unwrap();
  assert_eq!(bytes_value(&[1, 0, 0, 0, 0, 0, 0, 0, 0]), Ok(power));
  let read = bytes_value::<Uint<U65, [u16; 5]>>(&[2, 0, 0, 0, 0, 0, 0, 0, 0]);
  assert_eq!(read.unwrap_err(), too_large);
  assert_eq!(
    bytes_value::<Int<U65>>(&[255, 0, 0, 0, 0, 0, 0, 0, 0]),
    Ok(Int::<U65>::MIN)
  );
  let read = bytes_value::<Int<U65>>(&[254, 255, 255, 255, 255, 255, 255, 255, 255]);
  assert_eq!(read.unwrap_err(), too_small);
  assert_eq!(bytes_value::<i8>(&[128]), Ok(i8::MIN));
  assert_eq!(bytes_value::<u8>(&[255]), Ok(u8::MAX));
  assert_eq!(
    bytes_value::<Int<U16, [u8; 2]>>(&[128, 0]),
    Ok(Int::<U16, [u8; 2]>::MIN)
  );
  assert_eq!(bytes_value::<u0>(&[]), Ok(u0::ZERO));
  assert_eq!(bytes_value::<i0>(&[]), Ok(i0::ZERO));
  let none = "expected 0 bytes";
  assert_eq!(bytes_value::<u0>(&[0]).unwrap_err(), none);
  assert_eq!(bytes_value::<i0>(&[0, 0]).unwrap_err(), none);
  let max = Uint::<U12>::MAX;
  let tokens = [
    Token::NewtypeStruct { name: "Required" },
    Token::Bytes(&[15, 255]),
  ];
  assert_tokens(&with_be_bytes::Required(max), &tokens);
  let tokens = [
    Token::NewtypeStruct { name: "Required" },
    Token::Seq { len: Some(2) },
    Token::U8(255),
    Token::U8(15),
    Token::SeqEnd,
  ];
  assert_de_tokens(&with_le_bytes::Required(max), &tokens);
  let read: Result<Uint<U12>, de::value::Error> =
    be_bytes::required::deserialize(de::value::SeqDeserializer::new([15i64, 255].into_iter()));
  assert_eq!(read.unwrap(), max);
}

/// Asserts that `T`, whose width has `count` bytes, one or more, rejects with `be_bytes` and
/// `le_bytes` a byte string and a JSON array of one byte fewer or more with the message of
/// [`wrong_byte_count`], and so a JSON array of two elements more that are not bytes, which is
/// read to its end; a JSON array whose first element is not an integer from 0 to 255 with the
/// fixed message, at once; and every other JSON value, every token of serde_test of another kind,
/// and serde's deserializers of a string, an integer and a map, with the fixed message.
#[inline(never)]
fn bytes_rejections<T>(count: usize)
where
  T: BeBytes + LeBytes + PartialEq + Debug,
{
  let name = type_name::<T>();
  let wrong_count = wrong_byte_count(count as u64);
  for n in [count - 1, count + 1] {
    for result in bytes_read::<T>(&vec![0; n]) {
      assert_eq!(result, Err(wrong_count.clone()), "{n} bytes into {name}");
    }
  }
  let long = format!(r#"[{}"a",{{"b":[1.5]}}]"#, "0,".repeat(count));
  for result in bytes_from_json::<T>(&long) {
    assert_eq!(result, Err(wrong_count.clone()), "{long} into {name}");
  }
  for element in ["256", "-1", "\"a\"", "1.5", "true", "null", "[0]", "{}"] {
    let json = format!("[{element}{}]", ",0".repeat(count - 1));
    for result in bytes_from_json::<T>(&json) {
      assert_eq!(result, Err(BYTE_ARRAY.to_string()), "{json} into {name}");
    }
  }
  for json in ["5", "0", "-1", "1.5", "true", "null", "{}"] {
    for result in bytes_from_json::<T>(json) {
      assert_eq!(result, Err(BYTE_ARRAY.to_string()), "{json} into {name}");
    }
  }
  for token in [
    Token::U8(5),
    Token::I64(-1),
    Token::Bool(true),
    Token::Char('a'),
    Token::Str("a"),
    Token::Unit,
    Token::None,
  ] {
    let tokens = [Token::NewtypeStruct { name: "Required" }, token];
    assert_de_tokens_error::<with_be_bytes::Required<T>>(&tokens, BYTE_ARRAY);
    assert_de_tokens_error::<with_le_bytes::Required<T>>(&tokens, BYTE_ARRAY);
  }
  let tokens = [
    Token::NewtypeStruct { name: "Required" },
    Token::Seq { len: Some(0) },
    Token::SeqEnd,
  ];
  assert_de_tokens_error::<with_be_bytes::Required<T>>(&tokens, &wrong_count);
  let text = |error: de::value::Error| error.to_string();
  let map = || de::value::MapDeserializer::new([(NUMBER_TOKEN, "5")].into_iter());
  for read in [
    be_bytes::required::deserialize::<T, _>(map()).map_err(text),
    le_bytes::required::deserialize::<T, _>(map()).map_err(text),
    be_bytes::required::deserialize::<T, _>("a".into_deserializer()).map_err(text),
    le_bytes::required::deserialize::<T, _>(5u8.into_deserializer()).map_err(text),
  ] {
    assert_eq!(read, Err(BYTE_ARRAY.to_string()), "{name}");
  }
}

/// `be_bytes` and `le_bytes` reject bytes of another number than the width has with
/// `expected n bytes`, or `expected 1 byte` at a width of one byte, also a sequence too long whose
/// surplus is not bytes; an element of a sequence that is not an integer from 0 to 255 with the
/// fixed message, which names nothing of it; and every input but bytes and a sequence with the
/// fixed message.
#[test]
fn bytes_reject_each_broken_rule_with_its_message() {
  bytes_rejections::<Uint<U12>>(2);
  bytes_rejections::<Int<U12>>(2);
  bytes_rejections::<i8>(1);
  bytes_rejections::<NonZeroUint<U7>>(1);
  bytes_rejections::<Int<U100, [u32; 4]>>(13);
  bytes_rejections::<u256>(32);
  bytes_rejections::<NonZero<u64>>(8);
}

/// serde_json gives a string as its bytes when it reads the value directly, so the JSON string of
/// two ASCII characters reads into a type of two bytes, while through serde's buffer, in an
/// untagged enum or a flattened struct, it stays a string, which `be_bytes` and `le_bytes` reject;
/// a JSON array reads there.
#[test]
fn bytes_read_a_json_string_directly_alone() {
  assert_eq!(
    bytes_from_json::<Uint<U16>>("\"ab\""),
    [Ok(Uint::<U16>::new(24930)), Ok(Uint::<U16>::new(25185))]
  );
  for result in through_the_buffer::<with_be_bytes::Field<Uint<U16>>>(r#"{"value":"ab"}"#) {
    assert!(result.is_err());
  }
  for result in through_the_buffer::<with_le_bytes::Field<Uint<U16>>>(r#"{"value":"ab"}"#) {
    assert!(result.is_err());
  }
  for result in through_the_buffer::<with_be_bytes::Field<Int<U12>>>(r#"{"value":[255,255]}"#) {
    assert_eq!(
      result,
      Ok(with_be_bytes::Field {
        value: Int::<U12>::new(-1)
      })
    );
  }
  for result in through_the_buffer::<with_le_bytes::Field<Uint<U12>>>(r#"{"value":[255,15]}"#) {
    assert_eq!(
      result,
      Ok(with_le_bytes::Field {
        value: Uint::<U12>::MAX
      })
    );
  }
}

/// The leaves `optional` of `be_bytes` and `le_bytes`: `None` is JSON's `null` and postcard's
/// `[0]`, and `Some` is postcard's `[1]` followed by the length and the bytes; a missing field
/// reads as `None` with `default` and is required without it; `null` reads as `None`, also through
/// serde's buffer; an empty csv field is `None`; and every other input gets the fixed message.
#[test]
fn bytes_in_options() {
  options_round_trip(
    with_be_bytes::Optional::<Int<U12>>(None),
    with_be_bytes::Optional(Some(Int::<U12>::new(-1))),
    "[255,255]",
    &[2, 255, 255],
  );
  options_round_trip(
    with_le_bytes::Optional::<Uint<U12>>(None),
    with_le_bytes::Optional(Some(Uint::<U12>::MAX)),
    "[255,15]",
    &[2, 255, 15],
  );
  option_fields(
    [
      with_be_bytes::Defaulted::<u16> { value: None },
      with_be_bytes::Defaulted { value: Some(258) },
    ],
    [
      with_be_bytes::Present::<u16> { value: None },
      with_be_bytes::Present { value: Some(258) },
    ],
    "[1,2]",
  );
  option_fields(
    [
      with_le_bytes::Defaulted::<u16> { value: None },
      with_le_bytes::Defaulted { value: Some(513) },
    ],
    [
      with_le_bytes::Present::<u16> { value: None },
      with_le_bytes::Present { value: Some(513) },
    ],
    "[1,2]",
  );
  let read: Result<Option<u256>, de::value::Error> = be_bytes::optional::deserialize(CsvField(""));
  assert_eq!(read.unwrap(), None);
  let read: Result<Option<u256>, de::value::Error> = le_bytes::optional::deserialize(CsvField(""));
  assert_eq!(read.unwrap(), None);
  for json in ["5", "true", "{}"] {
    let error = serde_json::from_str::<with_be_bytes::Optional<u16>>(json).unwrap_err();
    assert_eq!(json_message(&error), BYTE_ARRAY, "{json}");
    let error = serde_json::from_str::<with_le_bytes::Optional<u16>>(json).unwrap_err();
    assert_eq!(json_message(&error), BYTE_ARRAY, "{json}");
  }
  let tokens = [
    Token::NewtypeStruct { name: "Optional" },
    Token::Some,
    Token::Bytes(&[1, 2]),
  ];
  assert_tokens(&with_be_bytes::Optional(Some(258u16)), &tokens);
  assert_tokens(&with_le_bytes::Optional(Some(513u16)), &tokens);
}
