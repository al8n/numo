//! Decimals over the heap storages, a wide integer whose limbs are in a `Box`, an `Rc`, an `Arc`
//! or a `triomphe::Arc`, against their twins, the decimals with the same digit counts over the
//! integer of the same width and sign over the array itself: every item of the general layer of
//! the traits, with its `None` and its errors; the conversions between a heap decimal and its twin
//! and into other types, and the exact product; the texts and their parsing, the order, the hash
//! and serde; the generators; a `Scaled` over a heap decimal; code generic over the decimal and
//! over the storage; where the result of each operation lives, in the allocation of an operand or
//! in one of its own, which the address of its limbs tells; the literals, which reach a heap
//! decimal through its twin, and the type that a literal operand of an inline decimal takes beside
//! the operators of the heap decimals; and the layout and the auto traits. The oracle of each
//! check is the twin. The operators and their panics are checked in `decimal::tests::operators`,
//! whose hook captures the panics: this module catches none.
//!
//! The checks of each pointer run for one type of each of six storages, of 192 to 1024 bits and of
//! both signs, each at the digit limit of its storage. Each type checks in a function of its own,
//! never inlined, whose frame holds the values of that type alone: an optimized build would inline
//! a function that has one call site, and the frame of the test would then hold the values of
//! every type. The tier of the build sizes every sweep, and every random value comes from a
//! generator with a seed written here, so that a failure replays.

#[cfg(any(feature = "std", feature = "alloc"))]
use core::ops::{AddAssign, SubAssign};
use core::{
  any::TypeId,
  fmt::{Debug, Display, LowerExp, UpperExp},
  hash::BuildHasher,
  num::NonZero,
  ops::Add,
};
#[cfg(all(any(feature = "std", feature = "alloc"), target_has_atomic = "ptr"))]
use std::sync::Arc;
#[cfg(any(feature = "std", feature = "alloc"))]
use std::{boxed::Box, rc::Rc};
use std::{
  collections::hash_map::RandomState,
  format,
  string::{String, ToString},
  vec::Vec,
};

#[cfg(feature = "proptest")]
use ::proptest::{
  arbitrary::any,
  strategy::{Strategy, ValueTree},
  test_runner::{Config, RngAlgorithm, TestRng, TestRunner},
};
#[cfg(feature = "serde")]
use serde_core::{Serialize, de::DeserializeOwned};
#[cfg(all(feature = "triomphe", target_has_atomic = "ptr"))]
use triomphe::Arc as TriompheArc;
use typenum::{
  U0, U1, U2, U8, U10, U18, U20, U39, U40, U58, U78, U135, U155, U192, U256, U308, U320, U512,
  U576, U1024,
};

use crate::{
  ArrayInteger, ConstFixedDecimal, ConvertError, Decimal, DecimalAssign, DecimalFloat, DecimalOne,
  DivideError, FillsRepr, FixedDecimal, FixedInteger, FromLimbs, InlineStorage, Int, IntegerOne,
  Layout, MutableStorage, ParseError, RoundingMode, Scaled, SignedFixedDecimal, Storage, Uint,
  UnsignedFixedDecimal,
  bigint::{MODES, floats::Words, tier},
  dec, i256,
};

/// A heap storage of a decimal, as the checks of where a result lives read it: whether its pointer
/// holds the coefficient in place, and the address of its limbs.
trait Allocated: Storage {
  /// Whether the pointer holds its coefficient in place, so that a result stored into an operand
  /// keeps the allocation of the operand, as in a `Box`. A shared pointer gives each result an
  /// allocation of its own.
  const IN_PLACE: bool;

  /// The address of the limbs, which `ArrayInteger::as_limbs` of the integer lends.
  fn limbs(&self) -> *const u64;
}

/// Implements [`Allocated`] for the wide integers whose limbs are in the pointer `$pointer`, which
/// holds its coefficient in place if `$in_place`.
macro_rules! allocated {
  ($pointer:ident, $in_place:literal) => {
    allocated!(@integer Uint, $pointer, $in_place);
    allocated!(@integer Int, $pointer, $in_place);
  };
  (@integer $integer:ident, $pointer:ident, $in_place:literal) => {
    impl<B, const N: usize> Allocated for $integer<B, $pointer<[u64; N]>>
    where
      B: FillsRepr<[u64; N]>,
      $integer<B, [u64; N]>: InlineStorage,
    {
      const IN_PLACE: bool = $in_place;

      fn limbs(&self) -> *const u64 {
        ArrayInteger::as_limbs(self).as_ptr()
      }
    }
  };
}

#[cfg(any(feature = "std", feature = "alloc"))]
allocated!(Box, true);
#[cfg(any(feature = "std", feature = "alloc"))]
allocated!(Rc, false);
#[cfg(all(any(feature = "std", feature = "alloc"), target_has_atomic = "ptr"))]
allocated!(Arc, false);
#[cfg(all(feature = "triomphe", target_has_atomic = "ptr"))]
allocated!(TriompheArc, false);

/// The values at which a type is checked, as values of its twin `T`: zero, the limits, one, two
/// and 25 units of the last digit, negated too for a signed storage, and random coefficients of
/// random lengths and, for a signed storage, signs, 12 in the Full tier and 2 in the Debug tier;
/// under Miri, zero and the limits. Each value is listed once.
fn samples<T: ConstFixedDecimal>() -> Vec<T> {
  let mut candidates = Vec::from([T::ZERO, T::MIN, T::MAX]);
  if !tier().is_miri() {
    for units in [1, 2, 25] {
      candidates.push(units_of(units));
      if T::SIGNED {
        candidates.push(units_of(-units));
      }
    }
    let count = if tier().is_full() { 12 } else { 2 };
    let mut words = Words::new(seed::<T>());
    candidates.extend((0..count).map(|_| random::<T>(&mut words)));
  }
  let mut values = Vec::new();
  for value in candidates {
    if !values.contains(&value) {
      values.push(value);
    }
  }
  values
}

/// The value of `T` of `units` units of its last digit.
fn units_of<T: FixedDecimal>(units: i32) -> T {
  format!("{units}e-{}", T::FRACTION)
    .parse()
    .expect("in range")
}

/// A random value of `T`: a coefficient of a random number of digits, at most those of `T`, whose
/// first digit is not zero, with a random sign for a signed storage.
fn random<T: FixedDecimal>(words: &mut Words) -> T {
  let digits = 1 + words.next() % u64::from(T::DIGITS);
  let mut text = String::new();
  if T::SIGNED && words.next() & 1 == 1 {
    text.push('-');
  }
  text.push(char::from(b'1' + (words.next() % 9) as u8));
  for _ in 1..digits {
    text.push(char::from(b'0' + (words.next() % 10) as u8));
  }
  format!("{text}e-{}", T::FRACTION)
    .parse()
    .expect("in range")
}

/// The seed of the random values of `T`, with its digit counts and its sign, so that each type
/// draws values of its own.
fn seed<T: FixedDecimal>() -> u64 {
  0x5eed_4ea9 ^ (u64::from(T::DIGITS) << 32 | u64::from(T::FRACTION) << 8 | u64::from(T::SIGNED))
}

/// The [`samples`] of the twin `T`, each beside the value that `store` puts in the heap storage.
fn pairs<T: ConstFixedDecimal, H>(store: fn(T) -> H) -> Vec<(T, H)> {
  samples::<T>()
    .into_iter()
    .map(|twin| (twin, store(twin)))
    .collect()
}

/// The rounding modes of a check: every mode in the Full tier, and otherwise `Down`, `HalfEven`
/// and `Ceiling`, a mode toward zero, a half mode and a mode away from zero for a positive value.
fn modes() -> &'static [RoundingMode] {
  if tier().is_full() {
    &MODES
  } else {
    &[
      RoundingMode::Down,
      RoundingMode::HalfEven,
      RoundingMode::Ceiling,
    ]
  }
}

/// The values that a generator draws for each type: 64 in the Full tier and 8 in the Debug tier;
/// under Miri, one.
#[cfg(any(feature = "arbitrary", feature = "proptest", feature = "quickcheck"))]
fn draws() -> usize {
  if tier().is_full() {
    64
  } else if tier().is_miri() {
    1
  } else {
    8
  }
}

/// The texts of `value` in the formats of a decimal, with a width, a precision and the flags among
/// them.
fn texts<T>(value: &T) -> [String; 9]
where
  T: Display + Debug + LowerExp + UpperExp,
{
  [
    value.to_string(),
    format!("{value:?}"),
    format!("{value:e}"),
    format!("{value:E}"),
    format!("{value:#e}"),
    format!("{value:>40.3}"),
    format!("{value:+.2e}"),
    format!("{value:#012E}"),
    format!("{value:<9.25?}"),
  ]
}

/// The type of `value`.
fn type_of<T: 'static>(_: &T) -> TypeId {
  TypeId::of::<T>()
}

/// The size and the alignment of `T` and of `Option<T>`.
fn layout<T>() -> [usize; 4] {
  [
    size_of::<T>(),
    align_of::<T>(),
    size_of::<Option<T>>(),
    align_of::<Option<T>>(),
  ]
}

/// Asserts that `T` is `Send` and `Sync`.
fn send_and_sync<T: Send + Sync>() {}

/// Checks `Decimal<I, F, R>`, over the heap storage `R`, at each of its samples: `from_inline`
/// stores the value of the twin and `to_inline` gives it back; the texts are those of the twin;
/// the comparison of each pair is that of the twins; and the hash through one hasher is that of
/// the twin.
#[inline(never)]
fn check_values<I, F, R>()
where
  R: Storage,
  (I, F): Layout<R>,
  Decimal<I, F, R::Inline>: ConstFixedDecimal + LowerExp + UpperExp,
{
  let values = pairs(Decimal::<I, F, R>::from_inline);
  let state = RandomState::new();
  for (twin, heap) in &values {
    assert_eq!(heap.to_inline(), *twin);
    assert_eq!(texts(heap), texts(twin), "{twin}");
    assert_eq!(state.hash_one(heap), state.hash_one(twin), "{twin}");
    for (other, other_heap) in &values {
      assert_eq!(heap.cmp(other_heap), twin.cmp(other), "{twin}, {other}");
      assert_eq!(heap == other_heap, twin == other, "{twin}, {other}");
    }
  }
}

/// Checks every item of the general layer of the heap decimal `T` against its twin, the trait of
/// the twin named in each call: the four constants; zero, the limits, `Default` and one; at each
/// sample, the predicates, the counts, the parts, the rounding to significant digits in the modes
/// of the tier and the conversions to the floats, compared by their bits; at each pair of samples,
/// the checked and the saturating sums and differences, the remainder, `quantize` in the modes of
/// the tier and `is_multiple_of`; and the readings of floats, the special ones and those of the
/// samples, in the same modes. A result is that of the twin, stored, and each `None` and each error
/// are those of the twin.
#[inline(never)]
fn check_trait_layer<T>()
where
  T: DecimalOne + DecimalFloat,
  T::Inline: DecimalOne + DecimalFloat,
{
  assert_eq!(
    [T::INTEGRAL, T::FRACTION, T::DIGITS],
    [
      <T::Inline as FixedDecimal>::INTEGRAL,
      <T::Inline as FixedDecimal>::FRACTION,
      <T::Inline as FixedDecimal>::DIGITS,
    ]
  );
  assert_eq!(T::SIGNED, <T::Inline as FixedDecimal>::SIGNED);
  let zero = <T::Inline as ConstFixedDecimal>::ZERO;
  assert_eq!(T::zero().to_inline(), zero);
  assert_eq!(T::default().to_inline(), zero);
  assert_eq!(
    T::max_value().to_inline(),
    <T::Inline as ConstFixedDecimal>::MAX
  );
  assert_eq!(
    T::min_value().to_inline(),
    <T::Inline as ConstFixedDecimal>::MIN
  );
  assert_eq!(T::one().to_inline(), <T::Inline as DecimalOne>::one());

  let values = pairs(T::from_inline);
  for (a, x) in &values {
    assert_eq!(x.to_inline(), *a);
    assert_eq!(x.is_zero(), a.is_zero(), "{a}");
    assert_eq!(x.is_positive(), a.is_positive(), "{a}");
    assert_eq!(x.is_integer(), a.is_integer(), "{a}");
    assert_eq!(x.order_of_magnitude(), a.order_of_magnitude(), "{a}");
    assert_eq!(x.significant_digits(), a.significant_digits(), "{a}");
    assert_eq!(x.fraction_digits(), a.fraction_digits(), "{a}");
    assert_eq!(x.trunc().to_inline(), a.trunc(), "{a}");
    assert_eq!(x.fract().to_inline(), a.fract(), "{a}");
    assert_eq!(x.floor().map(|v| v.to_inline()), a.floor(), "{a}");
    assert_eq!(x.ceil().map(|v| v.to_inline()), a.ceil(), "{a}");
    for count in [1, 2, T::DIGITS] {
      let digits = NonZero::new(count).expect("a count of digits");
      for &mode in modes() {
        assert_eq!(
          x.round_significant(digits, mode).map(|v| v.to_inline()),
          a.round_significant(digits, mode),
          "{a} {count} {mode:?}"
        );
      }
    }
    assert_eq!(x.to_f64().to_bits(), a.to_f64().to_bits(), "{a}");
    assert_eq!(x.to_f32().to_bits(), a.to_f32().to_bits(), "{a}");
    for (b, y) in &values {
      assert_eq!(
        x.checked_add(y).map(|v| v.to_inline()),
        a.checked_add(b),
        "{a} + {b}"
      );
      assert_eq!(
        x.checked_sub(y).map(|v| v.to_inline()),
        a.checked_sub(b),
        "{a} - {b}"
      );
      assert_eq!(
        x.checked_rem(y).map(|v| v.to_inline()),
        a.checked_rem(b),
        "{a} % {b}"
      );
      assert_eq!(
        x.saturating_add(y).to_inline(),
        a.saturating_add(b),
        "{a} + {b}"
      );
      assert_eq!(
        x.saturating_sub(y).to_inline(),
        a.saturating_sub(b),
        "{a} - {b}"
      );
      assert_eq!(x.is_multiple_of(y), a.is_multiple_of(b), "{a}, {b}");
      for &mode in modes() {
        assert_eq!(
          x.quantize(y, mode).map(|v| v.to_inline()),
          a.quantize(b, mode),
          "{a} to {b} {mode:?}"
        );
      }
    }
  }

  let specials = [
    0.0,
    -0.0,
    0.1,
    -1.5,
    2.675,
    1e-30,
    1e30,
    f64::MIN_POSITIVE,
    f64::MAX,
    f64::NAN,
    f64::INFINITY,
    f64::NEG_INFINITY,
  ];
  let floats = specials
    .into_iter()
    .chain(values.iter().map(|(a, _)| a.to_f64()));
  for value in floats {
    let narrow = value as f32;
    for &mode in modes() {
      assert_eq!(
        T::round_from_f64(value, mode).map(|v| v.to_inline()),
        <T::Inline as DecimalFloat>::round_from_f64(value, mode),
        "{value} {mode:?}"
      );
      assert_eq!(
        T::from_f64_shortest(value, mode).map(|v| v.to_inline()),
        <T::Inline as DecimalFloat>::from_f64_shortest(value, mode),
        "{value} {mode:?}"
      );
      assert_eq!(
        T::round_from_f32(narrow, mode).map(|v| v.to_inline()),
        <T::Inline as DecimalFloat>::round_from_f32(narrow, mode),
        "{narrow} {mode:?}"
      );
      assert_eq!(
        T::from_f32_shortest(narrow, mode).map(|v| v.to_inline()),
        <T::Inline as DecimalFloat>::from_f32_shortest(narrow, mode),
        "{narrow} {mode:?}"
      );
    }
  }
}

/// Checks the sign trait of the heap decimal `T`, over a signed storage, at each sample against its
/// twin: `is_negative`, `neg` and `abs` of the trait, and the operator `-` on a value.
#[inline(never)]
fn check_signed<T>()
where
  T: SignedFixedDecimal,
  T::Inline: SignedFixedDecimal,
{
  for (a, x) in pairs(T::from_inline) {
    assert_eq!(x.is_negative(), a.is_negative(), "{a}");
    assert_eq!(
      SignedFixedDecimal::neg(&x).to_inline(),
      SignedFixedDecimal::neg(&a),
      "{a}"
    );
    assert_eq!(x.abs().to_inline(), a.abs(), "{a}");
    assert_eq!((-x).to_inline(), -a, "{a}");
  }
}

/// What remains of `budget` after `cost`, or `None` below zero: code bounded by the sign trait of
/// an unsigned decimal.
fn remaining<T: UnsignedFixedDecimal>(budget: &T, cost: &T) -> Option<T> {
  budget.checked_sub(cost)
}

/// Checks code bounded by `UnsignedFixedDecimal` with the heap decimal `T`, over an unsigned
/// storage, against its twin, at each pair of samples.
#[inline(never)]
fn check_unsigned<T>()
where
  T: UnsignedFixedDecimal,
  T::Inline: UnsignedFixedDecimal,
{
  let values = pairs(T::from_inline);
  for (a, x) in &values {
    for (b, y) in &values {
      let left = remaining(x, y).map(|v| v.to_inline());
      assert_eq!(left, remaining(a, b), "{a} - {b}");
    }
  }
}

/// Checks that a text parses into the heap decimal `T` as into its twin, to the value stored or to
/// the same error: the texts of each sample in each format, and texts out of the range of `T`, with
/// a fraction digit more than it holds, negative, and of no number.
#[inline(never)]
fn check_parsing<T>()
where
  T: FixedDecimal,
  T::Inline: LowerExp + UpperExp,
{
  let max = <T::Inline as ConstFixedDecimal>::MAX;
  let mut inputs = Vec::from([
    format!("{max}9"),
    format!("1e{}", T::INTEGRAL),
    format!("1e-{}", T::FRACTION + 1),
    String::from("-1"),
    String::from("-0"),
    String::new(),
    String::from("1.2.3"),
    String::from("NaN"),
    String::from("-inf"),
  ]);
  for (a, _) in pairs(T::from_inline) {
    inputs.extend(texts(&a));
  }
  for text in &inputs {
    let parsed: Result<T, ParseError> = text.parse();
    let expected: Result<T::Inline, ParseError> = text.parse();
    assert_eq!(parsed.map(|v| v.to_inline()), expected, "{text:?}");
  }
}

/// Checks that the heap decimal `T` writes in JSON and in postcard what its twin writes, and reads
/// it back, at each sample.
#[cfg(feature = "serde")]
#[inline(never)]
fn check_serde<T>()
where
  T: FixedDecimal + Serialize + DeserializeOwned,
  T::Inline: Serialize,
{
  for (a, x) in pairs(T::from_inline) {
    let json = serde_json::to_string(&x).expect("a text");
    assert_eq!(json, serde_json::to_string(&a).expect("a text"), "{a}");
    let back: T = serde_json::from_str(&json).expect("a decimal");
    assert_eq!(back.to_inline(), a, "{a}");
    let bytes = postcard::to_allocvec(&x).expect("bytes");
    assert_eq!(bytes, postcard::to_allocvec(&a).expect("bytes"), "{a}");
    let back: T = postcard::from_bytes(&bytes).expect("a decimal");
    assert_eq!(back.to_inline(), a, "{a}");
  }
}

/// Checks that `quickcheck` draws for the heap decimal `T` the values that it draws for its twin
/// from the same seed, each in the range of `T`, and that the shrinking candidates of each are
/// those of its twin.
#[cfg(feature = "quickcheck")]
#[inline(never)]
fn check_quickcheck<T>()
where
  T: FixedDecimal + ::quickcheck::Arbitrary,
  T::Inline: ::quickcheck::Arbitrary,
{
  let (min, max) = (
    <T::Inline as ConstFixedDecimal>::MIN,
    <T::Inline as ConstFixedDecimal>::MAX,
  );
  let mut heap = ::quickcheck::Gen::from_size_and_seed(100, seed::<T>());
  let mut twin = ::quickcheck::Gen::from_size_and_seed(100, seed::<T>());
  for _ in 0..draws() {
    let value = <T as ::quickcheck::Arbitrary>::arbitrary(&mut heap);
    let expected = <T::Inline as ::quickcheck::Arbitrary>::arbitrary(&mut twin);
    assert_eq!(value.to_inline(), expected);
    assert!(min <= expected && expected <= max, "{expected}");
    let candidates: Vec<T::Inline> = ::quickcheck::Arbitrary::shrink(&value)
      .map(|candidate| candidate.to_inline())
      .collect();
    let expected_candidates: Vec<T::Inline> = ::quickcheck::Arbitrary::shrink(&expected).collect();
    assert_eq!(candidates, expected_candidates, "{expected}");
  }
}

/// A runner of `proptest` with a fixed seed that writes no failure to a file.
#[cfg(feature = "proptest")]
fn runner() -> TestRunner {
  TestRunner::new_with_rng(
    Config {
      failure_persistence: None,
      ..Config::default()
    },
    TestRng::deterministic_rng(RngAlgorithm::ChaCha),
  )
}

/// Checks that `proptest` draws for the heap decimal `T` the values that it draws for its twin
/// with the same seed, each in the range of `T`, and that the value tree of each moves through the
/// candidates of the tree of its twin: from the first, where `simplify` moves to, to the value
/// again, where `complicate` moves past the last.
#[cfg(feature = "proptest")]
#[inline(never)]
fn check_proptest<T>()
where
  T: FixedDecimal + ::proptest::arbitrary::Arbitrary,
  T::Inline: ::proptest::arbitrary::Arbitrary,
{
  let (min, max) = (
    <T::Inline as ConstFixedDecimal>::MIN,
    <T::Inline as ConstFixedDecimal>::MAX,
  );
  let (mut heap, mut twin) = (runner(), runner());
  for _ in 0..draws() {
    let mut tree = any::<T>().new_tree(&mut heap).expect("a tree");
    let mut expected = any::<T::Inline>().new_tree(&mut twin).expect("a tree");
    let value = expected.current();
    assert_eq!(tree.current().to_inline(), value);
    assert!(min <= value && value <= max, "{value}");
    let mut moved = tree.simplify();
    assert_eq!(moved, expected.simplify(), "{value}");
    while moved {
      assert_eq!(tree.current().to_inline(), expected.current(), "{value}");
      moved = tree.complicate();
      assert_eq!(moved, expected.complicate(), "{value}");
    }
  }
}

/// Checks that `arbitrary` reads for the heap decimal `T` from random bytes the values that it
/// reads for its twin from the same bytes, each in the range of `T`.
#[cfg(feature = "arbitrary")]
#[inline(never)]
fn check_arbitrary<T>()
where
  T: FixedDecimal + for<'a> ::arbitrary::Arbitrary<'a>,
  T::Inline: for<'a> ::arbitrary::Arbitrary<'a>,
{
  let (min, max) = (
    <T::Inline as ConstFixedDecimal>::MIN,
    <T::Inline as ConstFixedDecimal>::MAX,
  );
  let mut words = Words::new(seed::<T>());
  for _ in 0..draws() {
    // More bytes than the longest path of a type of 308 digits reads.
    let bytes: Vec<u8> = (0..400).map(|_| words.next() as u8).collect();
    let mut data = ::arbitrary::Unstructured::new(&bytes);
    let value = <T as ::arbitrary::Arbitrary>::arbitrary(&mut data).expect("a value");
    let mut data = ::arbitrary::Unstructured::new(&bytes);
    let expected = <T::Inline as ::arbitrary::Arbitrary>::arbitrary(&mut data).expect("a value");
    assert_eq!(value.to_inline(), expected);
    assert!(min <= expected && expected <= max, "{expected}");
  }
}

/// Checks `Scaled` over the heap decimal `T` against the `Scaled` over its twin. At each sample
/// with the scales 0, the fraction digits of `T` and one more: the text, its parsing, and
/// `to_inline` and `from_inline`, which keep the scale. With those scales by turns: the value and
/// the scale of each method of the general layer at each sample, and at each pair of samples for
/// those with an operand.
#[inline(never)]
fn check_scaled<T>()
where
  T: FixedDecimal,
  Scaled<T>: FixedDecimal<Inline = Scaled<T::Inline>>,
  Scaled<T::Inline>: ConstFixedDecimal,
{
  let fraction = u16::try_from(T::FRACTION).expect("a scale");
  let scales = [0, fraction, fraction + 1];
  let parts = |scaled: Scaled<T>| scaled.to_inline().into_parts();
  let mut values = Vec::new();
  for (index, (a, x)) in pairs(T::from_inline).into_iter().enumerate() {
    for scale in scales {
      let (twin, heap) = (Scaled::new(a, scale), Scaled::new(x.clone(), scale));
      let text = twin.to_string();
      assert_eq!(heap.to_string(), text);
      let parsed: Scaled<T> = text.parse().expect("the text of a value");
      let expected: Scaled<T::Inline> = text.parse().expect("the text of a value");
      assert_eq!(parts(parsed), expected.into_parts(), "{text}");
      assert_eq!(heap.to_inline().into_parts(), (a, scale), "{text}");
      assert_eq!(parts(Scaled::<T>::from_inline(twin)), (a, scale), "{text}");
    }
    let scale = scales[index % scales.len()];
    values.push((Scaled::new(a, scale), Scaled::new(x, scale)));
  }
  for (a, x) in &values {
    assert_eq!(parts(x.trunc()), a.trunc().into_parts(), "{a}");
    assert_eq!(parts(x.fract()), a.fract().into_parts(), "{a}");
    assert_eq!(
      x.floor().map(parts),
      a.floor().map(Scaled::into_parts),
      "{a}"
    );
    assert_eq!(x.ceil().map(parts), a.ceil().map(Scaled::into_parts), "{a}");
    let digits = NonZero::new(2).expect("two digits");
    for &mode in modes() {
      assert_eq!(
        x.round_significant(digits, mode).map(parts),
        a.round_significant(digits, mode).map(Scaled::into_parts),
        "{a} {mode:?}"
      );
    }
    for (b, y) in &values {
      let sum = x.checked_add(y).map(parts);
      assert_eq!(sum, a.checked_add(b).map(Scaled::into_parts), "{a} + {b}");
      let difference = x.checked_sub(y).map(parts);
      assert_eq!(
        difference,
        a.checked_sub(b).map(Scaled::into_parts),
        "{a} - {b}"
      );
      let remainder = x.checked_rem(y).map(parts);
      assert_eq!(
        remainder,
        a.checked_rem(b).map(Scaled::into_parts),
        "{a} % {b}"
      );
      let sum = parts(x.saturating_add(y));
      assert_eq!(sum, a.saturating_add(b).into_parts(), "{a} + {b}");
      let difference = parts(x.saturating_sub(y));
      assert_eq!(difference, a.saturating_sub(b).into_parts(), "{a} - {b}");
      for &mode in modes() {
        let quantized = x.quantize(y, mode).map(parts);
        let expected = a.quantize(b, mode).map(Scaled::into_parts);
        assert_eq!(quantized, expected, "{a} to {b} {mode:?}");
      }
    }
  }
}

/// The value doubled, computed in its inline twin and stored once, or `None` outside the range:
/// code bounded by `FixedDecimal` alone.
fn doubled<T: FixedDecimal>(value: &T) -> Option<T> {
  let twin = value.to_inline();
  twin.checked_add(&twin).map(T::from_inline)
}

/// The total of the values, each added in place: code bounded by `DecimalAssign`.
fn total<T: DecimalAssign>(values: &[T]) -> T {
  let mut total = T::zero();
  for value in values {
    total += value.clone();
  }
  total
}

/// The larger of two values and its text: code over any storage, which clones, compares and
/// formats a value.
fn larger<I, F, R>(a: &Decimal<I, F, R>, b: &Decimal<I, F, R>) -> (Decimal<I, F, R>, String)
where
  R: Storage,
  (I, F): Layout<R>,
{
  let larger = if a >= b { a.clone() } else { b.clone() };
  let text = larger.to_string();
  (larger, text)
}

/// `value` added to `total` in place: code over a storage that holds its coefficient in place.
fn accumulated<I, F, R>(mut total: Decimal<I, F, R>, value: Decimal<I, F, R>) -> Decimal<I, F, R>
where
  R: MutableStorage,
  Decimal<I, F, R::Inline>: FixedDecimal,
{
  total += value;
  total
}

/// Checks [`doubled`], bounded by `FixedDecimal`, and [`larger`], over any storage, with
/// `Decimal<I, F, R>` against its twin, at each sample and each pair of samples.
#[inline(never)]
fn check_generic_code<I, F, R>()
where
  R: Storage,
  (I, F): Layout<R>,
  Decimal<I, F, R>: FixedDecimal,
  Decimal<I, F, R::Inline>: ConstFixedDecimal,
{
  let values = pairs(Decimal::<I, F, R>::from_inline);
  for (a, x) in &values {
    assert_eq!(doubled(x).map(|v| v.to_inline()), doubled(a), "{a}");
    for (b, y) in &values {
      let expected = if a >= b { *a } else { *b };
      let (largest, text) = larger(x, y);
      assert_eq!(largest.to_inline(), expected, "{a}, {b}");
      assert_eq!(text, expected.to_string(), "{a}, {b}");
    }
  }
}

/// Checks [`total`], bounded by `DecimalAssign`, and [`accumulated`], over a storage that holds its
/// coefficient in place, with `Decimal<I, F, R>` against its twin, at each pair of samples whose
/// sum is in the range.
#[inline(never)]
fn check_assignments<I, F, R>()
where
  R: MutableStorage,
  (I, F): Layout<R>,
  Decimal<I, F, R>: DecimalAssign,
  Decimal<I, F, R::Inline>: ConstFixedDecimal,
{
  let values = pairs(Decimal::<I, F, R>::from_inline);
  for (a, x) in &values {
    for (b, y) in &values {
      if let Some(sum) = a.checked_add(b) {
        let both = [x.clone(), y.clone()];
        assert_eq!(total(&both).to_inline(), sum, "{a} + {b}");
        assert_eq!(
          accumulated(x.clone(), y.clone()).to_inline(),
          sum,
          "{a} + {b}"
        );
      }
    }
  }
}

/// The address of the limbs of `value`, read through the storage of the decimal.
fn address<I, F, R: Allocated>(value: &Decimal<I, F, R>) -> *const u64 {
  value.repr.limbs()
}

/// The value of `value` in its twin, and the address of its limbs.
fn placed<I, F, R: Allocated>(value: &Decimal<I, F, R>) -> (Decimal<I, F, R::Inline>, *const u64) {
  (value.to_inline(), address(value))
}

/// Checks where the results of `Decimal<I, F, R>` live, over `R`, a heap storage that holds its
/// coefficient in place, at each sample and each pair of samples whose sum or difference is in the
/// range: `+=` and `-=` by value and by reference, `clone_from`, also of a `Scaled`, and
/// `from_repr` of the storage that `repr` moved out keep the allocation of their target; `a + b`,
/// `a + &b` and a sum of values store into the allocation of `a`, the first item, and `&a + b`
/// into that of `b`.
#[cfg(any(feature = "std", feature = "alloc"))]
#[inline(never)]
fn check_in_place<I, F, R>()
where
  R: Allocated,
  Decimal<I, F, R>: DecimalAssign
    + for<'a> AddAssign<&'a Decimal<I, F, R>>
    + for<'a> SubAssign<&'a Decimal<I, F, R>>
    + for<'a> Add<&'a Decimal<I, F, R>, Output = Decimal<I, F, R>>,
  for<'a> &'a Decimal<I, F, R>: Add<Decimal<I, F, R>, Output = Decimal<I, F, R>>,
  Decimal<I, F, R::Inline>: ConstFixedDecimal,
{
  assert!(R::IN_PLACE);
  let values = pairs(Decimal::<I, F, R>::from_inline);
  for (a, x) in &values {
    let value = x.clone();
    let at = address(&value);
    let value = Decimal::<I, F, R>::from_repr(value.repr()).expect("in range");
    assert_eq!(placed(&value), (*a, at), "{a}");
    for (b, y) in &values {
      let mut target = x.clone();
      let at = address(&target);
      target.clone_from(y);
      assert_eq!(placed(&target), (*b, at), "{a} from {b}");
      let mut target = Scaled::new(x.clone(), 3);
      let at = address(target.value());
      target.clone_from(&Scaled::new(y.clone(), 5));
      let place = (placed(target.value()), target.scale());
      assert_eq!(place, ((*b, at), 5), "{a} from {b}");
      if let Some(sum) = a.checked_add(b) {
        let mut target = x.clone();
        let at = address(&target);
        target += y.clone();
        assert_eq!(placed(&target), (sum, at), "{a} += {b}");
        let mut target = x.clone();
        let at = address(&target);
        target += y;
        assert_eq!(placed(&target), (sum, at), "{a} += &{b}");
        let lhs = x.clone();
        let at = address(&lhs);
        assert_eq!(placed(&(lhs + y.clone())), (sum, at), "{a} + {b}");
        let lhs = x.clone();
        let at = address(&lhs);
        assert_eq!(placed(&(lhs + y)), (sum, at), "{a} + &{b}");
        let rhs = y.clone();
        let at = address(&rhs);
        assert_eq!(placed(&(x + rhs)), (sum, at), "&{a} + {b}");
        let items = [x.clone(), y.clone()];
        let at = address(&items[0]);
        let total: Decimal<I, F, R> = items.into_iter().sum();
        assert_eq!(placed(&total), (sum, at), "the sum of {a} and {b}");
      }
      if let Some(difference) = a.checked_sub(b) {
        let mut target = x.clone();
        let at = address(&target);
        target -= y.clone();
        assert_eq!(placed(&target), (difference, at), "{a} -= {b}");
        let mut target = x.clone();
        let at = address(&target);
        target -= y;
        assert_eq!(placed(&target), (difference, at), "{a} -= &{b}");
      }
    }
  }
}

/// Checks where the results of `Decimal<I, F, R>` live, over `R`, a shared heap storage, at each
/// sample and each pair of samples whose sum is in the range: `clone` shares the allocation of its
/// source, and so does `from_repr` of the storage that `repr` moved out; `clone_from`, also of a
/// `Scaled`, shares that of its source; and `a + b`, `a + &b`, `&a + b`, `&a + &b` and a sum of
/// values store into an allocation of their own, while the other owner of each operand keeps its
/// value and its address.
#[inline(never)]
fn check_shared<I, F, R>()
where
  R: Allocated,
  Decimal<I, F, R>: FixedDecimal + for<'a> Add<&'a Decimal<I, F, R>, Output = Decimal<I, F, R>>,
  for<'a> &'a Decimal<I, F, R>: Add<Decimal<I, F, R>, Output = Decimal<I, F, R>>
    + Add<&'a Decimal<I, F, R>, Output = Decimal<I, F, R>>,
  Decimal<I, F, R::Inline>: ConstFixedDecimal,
{
  assert!(!R::IN_PLACE);
  let values = pairs(Decimal::<I, F, R>::from_inline);
  for (a, x) in &values {
    let at = address(x);
    let shared = x.clone();
    assert_eq!(placed(&shared), (*a, at), "{a}");
    let value = Decimal::<I, F, R>::from_repr(shared.repr()).expect("in range");
    assert_eq!(placed(&value), (*a, at), "{a}");
    for (b, y) in &values {
      let owners = [placed(x), placed(y)];
      let mut target = x.clone();
      target.clone_from(y);
      assert_eq!(placed(&target), owners[1], "{a} from {b}");
      let mut target = Scaled::new(x.clone(), 3);
      target.clone_from(&Scaled::new(y.clone(), 5));
      let place = (placed(target.value()), target.scale());
      assert_eq!(place, (owners[1], 5), "{a} from {b}");
      if let Some(sum) = a.checked_add(b) {
        let results: [Decimal<I, F, R>; 5] = [
          x.clone() + y.clone(),
          x.clone() + y,
          x + y.clone(),
          x + y,
          [x.clone(), y.clone()].into_iter().sum(),
        ];
        for result in &results {
          let (value, place) = placed(result);
          assert_eq!(value, sum, "{a} + {b}");
          assert!(place != owners[0].1 && place != owners[1].1, "{a} + {b}");
        }
      }
      assert_eq!([placed(x), placed(y)], owners, "{a}, {b}");
    }
  }
}

/// Checks where `-a` stores, for `Decimal<I, F, R>` over a signed heap storage `R`, at each
/// sample: into the allocation of `a` in a pointer that holds its coefficient in place, and in a
/// shared pointer into an allocation of its own, while the other owner of `a` keeps its value and
/// its address.
#[inline(never)]
fn check_negation_place<I, F, R>()
where
  R: Allocated,
  Decimal<I, F, R>: SignedFixedDecimal,
  Decimal<I, F, R::Inline>: ConstFixedDecimal + SignedFixedDecimal,
{
  for (a, x) in pairs(Decimal::<I, F, R>::from_inline) {
    let other = x.clone();
    let at = address(&x);
    let negated = -x;
    assert_eq!(negated.to_inline(), -a, "-{a}");
    if R::IN_PLACE {
      assert_eq!(address(&negated), at, "-{a}");
    } else {
      assert_ne!(address(&negated), at, "-{a}");
      assert_eq!(placed(&other), (a, at), "-{a}");
    }
  }
}

/// Checks a type with [`check_values`], written `Integral, Fraction, Storage`.
macro_rules! values_agree {
  ($integral:ty, $fraction:ty, $repr:ty) => {
    check_values::<$integral, $fraction, $repr>()
  };
}

/// Checks a type with [`check_generic_code`], written `Integral, Fraction, Storage`.
macro_rules! generic_code_agrees {
  ($integral:ty, $fraction:ty, $repr:ty) => {
    check_generic_code::<$integral, $fraction, $repr>()
  };
}

/// Checks a type in a `Box` with [`check_in_place`], written `Integral, Fraction, Storage`.
#[cfg(any(feature = "std", feature = "alloc"))]
macro_rules! in_place_agrees {
  ($integral:ty, $fraction:ty, $repr:ty) => {
    check_in_place::<$integral, $fraction, $repr>()
  };
}

/// Checks a type in a shared pointer with [`check_shared`], written `Integral, Fraction, Storage`.
macro_rules! shared_agrees {
  ($integral:ty, $fraction:ty, $repr:ty) => {
    check_shared::<$integral, $fraction, $repr>()
  };
}

/// Checks the conversions of `Decimal<Integral, Fraction, Storage>`, written so, over a heap
/// storage, at each of its samples against those of its twin: `widen` into the twin and back; and
/// into a type with fewer digits over the same storage and into the twin of that type,
/// `try_convert`, `round_to` in the modes of the tier and `div_round` by a divisor of one digit,
/// zero among them, in the same modes. A result in a heap type is that of the twin, stored, and an
/// error is that of the twin. Each type checks in a function of its own.
macro_rules! conversions_agree {
  ($integral:ty, $fraction:ty, $repr:ty) => {{
    #[inline(never)]
    fn check() {
      type Heap = Decimal<$integral, $fraction, $repr>;
      type Twin = Decimal<$integral, $fraction, <$repr as Storage>::Inline>;
      // Fewer digits than each type checked, so that a value is out of range there, or loses
      // digits, over the heap storage and over its twin. A quotient has an intermediate of the
      // digits of the type, and by a divisor of no fraction digits, that of 308 digits too.
      type Narrow = Decimal<U20, U2, $repr>;
      type NarrowTwin = Decimal<U20, U2, <$repr as Storage>::Inline>;
      type Quotient = Decimal<U40, U0, $repr>;
      type QuotientTwin = Decimal<U40, U0, <$repr as Storage>::Inline>;
      type Divisor = Decimal<U1, U0, u8>;
      let divisors: [Divisor; 4] = [
        Divisor::ZERO,
        Divisor::ONE,
        "3".parse().expect("a digit"),
        Divisor::MAX,
      ];
      for (twin, heap) in pairs::<Twin, Heap>(Heap::from_inline) {
        let back: Twin = heap.clone().widen();
        assert_eq!(back, twin);
        let stored: Heap = twin.widen();
        assert_eq!(stored.to_inline(), twin);

        let expected: Result<NarrowTwin, ConvertError> = twin.try_convert();
        let converted: Result<NarrowTwin, ConvertError> = heap.clone().try_convert();
        assert_eq!(converted, expected, "{twin}");
        let converted: Result<Narrow, ConvertError> = heap.clone().try_convert();
        assert_eq!(converted.map(|value| value.to_inline()), expected, "{twin}");
        let converted: Result<Narrow, ConvertError> = twin.try_convert();
        assert_eq!(converted.map(|value| value.to_inline()), expected, "{twin}");
        for &mode in modes() {
          let expected: Result<NarrowTwin, ConvertError> = twin.round_to(mode);
          let rounded: Result<NarrowTwin, ConvertError> = heap.clone().round_to(mode);
          assert_eq!(rounded, expected, "{twin} {mode:?}");
          let rounded: Result<Narrow, ConvertError> = heap.clone().round_to(mode);
          assert_eq!(
            rounded.map(|value| value.to_inline()),
            expected,
            "{twin} {mode:?}"
          );
          for divisor in &divisors {
            let expected: Result<QuotientTwin, DivideError> = twin.div_round(divisor, mode);
            let quotient: Result<QuotientTwin, DivideError> = heap.div_round(divisor, mode);
            assert_eq!(quotient, expected, "{twin} / {divisor} {mode:?}");
            let quotient: Result<Quotient, DivideError> = heap.div_round(divisor, mode);
            let quotient = quotient.map(|value| value.to_inline());
            assert_eq!(quotient, expected, "{twin} / {divisor} {mode:?}");
          }
        }
      }
    }
    check();
  }};
}

/// Checks `exact_mul` of values of `Decimal<Integral, Fraction, Storage>`, written so, over a
/// heap storage, against the product of their twins: with each other at every pair of the samples,
/// or with each of zero and the limits of the decimal of one digit `Rhs`, written after a `;`. The
/// product is that of the twins, of the type of that product, in an inline storage. Each type
/// checks in a function of its own.
macro_rules! products_agree {
  ($integral:ty, $fraction:ty, $repr:ty; $rhs:ty) => {{
    #[inline(never)]
    fn check() {
      type Heap = Decimal<$integral, $fraction, $repr>;
      type Twin = Decimal<$integral, $fraction, <$repr as Storage>::Inline>;
      type Rhs = $rhs;
      for (twin, heap) in pairs::<Twin, Heap>(Heap::from_inline) {
        for rhs in [Rhs::ZERO, Rhs::MIN, Rhs::MAX] {
          let product = heap.exact_mul(&rhs);
          let expected = twin.exact_mul(&rhs);
          assert_eq!(type_of(&product), type_of(&expected), "{twin} * {rhs}");
          assert_eq!(product, expected, "{twin} * {rhs}");
        }
      }
    }
    check();
  }};
  ($integral:ty, $fraction:ty, $repr:ty) => {{
    #[inline(never)]
    fn check() {
      type Heap = Decimal<$integral, $fraction, $repr>;
      type Twin = Decimal<$integral, $fraction, <$repr as Storage>::Inline>;
      let values = pairs::<Twin, Heap>(Heap::from_inline);
      for (a, heap_a) in &values {
        for (b, heap_b) in &values {
          let product = heap_a.exact_mul(heap_b);
          let expected = a.exact_mul(b);
          assert_eq!(type_of(&product), type_of(&expected), "{a} * {b}");
          assert_eq!(product, expected, "{a} * {b}");
        }
      }
    }
    check();
  }};
}

/// Checks `repr` and `from_repr` of `Decimal<Integral, Fraction, Storage>`, written so, over a heap
/// storage: at each sample, the storage that `repr` moves out makes the value again; at the largest
/// coefficient of the type, `10^DIGITS - 1`, at `10^DIGITS` and, for a signed storage, at their
/// negations, `from_repr` gives what the twin's `from_repr` gives for the same coefficient; and the
/// largest value of the storage, built from its limbs, which has a digit more than the type holds,
/// makes none. Each type checks in a function of its own.
macro_rules! repr_agrees {
  ($integral:ty, $fraction:ty, $repr:ty) => {{
    #[inline(never)]
    fn check() {
      type Heap = Decimal<$integral, $fraction, $repr>;
      type Twin = Decimal<$integral, $fraction, <$repr as Storage>::Inline>;
      for (twin, heap) in pairs::<Twin, Heap>(Heap::from_inline) {
        let value = Heap::from_repr(heap.repr()).expect("in range");
        assert_eq!(value.to_inline(), twin);
      }
      let one = <<$repr as Storage>::Inline as IntegerOne>::one();
      let largest = Twin::MAX.repr();
      let mut coefficients = Vec::from([largest, largest + one]);
      if <Twin as FixedDecimal>::SIGNED {
        let smallest = Twin::MIN.repr();
        coefficients.extend([smallest, smallest - one]);
      }
      for coefficient in coefficients {
        let storage = <$repr as FromLimbs>::from_limbs(ArrayInteger::into_limbs(coefficient));
        let value = Heap::from_repr(storage).map(|value| value.to_inline());
        assert_eq!(value, Twin::from_repr(coefficient), "{coefficient}");
      }
      let max = <<$repr as Storage>::Inline as FixedInteger>::max_value();
      let above = <$repr as FromLimbs>::from_limbs(ArrayInteger::into_limbs(max));
      assert!(Heap::from_repr(above).is_none());
    }
    check();
  }};
}

/// Runs the check `$check`, a macro written `$check!(Integral, Fraction, Storage)`, for one type of
/// each of six storages in the heap pointer `$pointer`, each at the digit limit of its storage:
/// the integers of 192, 256, 320, 512, 576 and 1024 bits, unsigned and signed by turns; under
/// Miri, the first two, an unsigned and a signed storage.
macro_rules! for_each_type {
  ($check:ident, $pointer:ident) => {
    $check!(U39, U18, Uint<U192, $pointer<[u64; 3]>>);
    $check!(U58, U18, Int<U256, $pointer<[u64; 4]>>);
    if !tier().is_miri() {
      $check!(U78, U18, Uint<U320, $pointer<[u64; 5]>>);
      $check!(U135, U18, Int<U512, $pointer<[u64; 8]>>);
      $check!(U155, U18, Int<U576, $pointer<[u64; 9]>>);
      $check!(U308, U0, Uint<U1024, $pointer<[u64; 16]>>);
    }
  };
}

/// Calls the check `$check`, a function generic over a decimal, for the types of `for_each_type!`
/// in the heap pointer `$pointer`.
macro_rules! for_each_decimal {
  ($check:ident, $pointer:ident) => {
    $check::<Decimal<U39, U18, Uint<U192, $pointer<[u64; 3]>>>>();
    $check::<Decimal<U58, U18, Int<U256, $pointer<[u64; 4]>>>>();
    if !tier().is_miri() {
      $check::<Decimal<U78, U18, Uint<U320, $pointer<[u64; 5]>>>>();
      $check::<Decimal<U135, U18, Int<U512, $pointer<[u64; 8]>>>>();
      $check::<Decimal<U155, U18, Int<U576, $pointer<[u64; 9]>>>>();
      $check::<Decimal<U308, U0, Uint<U1024, $pointer<[u64; 16]>>>>();
    }
  };
}

/// Defines the tests of the decimals over the heap pointer `$pointer`.
macro_rules! heap_tests {
  ($pointer:ident) => {
    /// `from_inline` stores a value and `to_inline` gives it back, and the texts, the order and the
    /// hash of a value are those of its twin, for each type over the pointer.
    #[test]
    fn values_match_their_twins() {
      for_each_type!(values_agree, $pointer);
    }

    /// `widen` converts a value to its twin and back, and `try_convert`, `round_to` and
    /// `div_round` into a heap type and into an inline type give the results and the errors of
    /// the twin, for each type over the pointer.
    #[test]
    fn conversions_match_the_twin() {
      for_each_type!(conversions_agree, $pointer);
    }

    /// `exact_mul` of two values over the pointer gives the product of their twins, in an inline
    /// storage: for each type whose square has a storage, and for the type of 576 bits by a decimal
    /// of one digit in an `i8`.
    #[test]
    fn products_are_those_of_the_twins() {
      products_agree!(U39, U18, Uint<U192, $pointer<[u64; 3]>>);
      products_agree!(U58, U18, Int<U256, $pointer<[u64; 4]>>);
      if !tier().is_miri() {
        products_agree!(U78, U18, Uint<U320, $pointer<[u64; 5]>>);
        products_agree!(U135, U18, Int<U512, $pointer<[u64; 8]>>);
        products_agree!(U155, U18, Int<U576, $pointer<[u64; 9]>>; Decimal<U1, U0, i8>);
      }
    }

    /// Every item of `FixedDecimal`, `DecimalOne` and `DecimalFloat` gives for a value over the
    /// pointer what it gives for its twin, stored, with the same `None` and errors, for each type
    /// over the pointer.
    #[test]
    fn trait_layer_matches_the_twin() {
      for_each_decimal!(check_trait_layer, $pointer);
    }

    /// `repr` moves the storage out and `from_repr` takes it back, and refuses a storage above the
    /// range of the type, for each type over the pointer.
    #[test]
    fn repr_and_from_repr_move_the_storage() {
      for_each_type!(repr_agrees, $pointer);
    }

    /// The decimals over a signed storage are `SignedFixedDecimal` and those over an unsigned one
    /// `UnsignedFixedDecimal`, with the results of their twins.
    #[test]
    fn sign_traits_match_the_twin() {
      check_unsigned::<Decimal<U39, U18, Uint<U192, $pointer<[u64; 3]>>>>();
      check_signed::<Decimal<U58, U18, Int<U256, $pointer<[u64; 4]>>>>();
      if !tier().is_miri() {
        check_unsigned::<Decimal<U78, U18, Uint<U320, $pointer<[u64; 5]>>>>();
        check_signed::<Decimal<U135, U18, Int<U512, $pointer<[u64; 8]>>>>();
        check_signed::<Decimal<U155, U18, Int<U576, $pointer<[u64; 9]>>>>();
        check_unsigned::<Decimal<U308, U0, Uint<U1024, $pointer<[u64; 16]>>>>();
      }
    }

    /// A text parses into a type over the pointer as into its twin, for each type over the
    /// pointer.
    #[test]
    fn texts_parse_as_into_the_twin() {
      for_each_decimal!(check_parsing, $pointer);
    }

    /// A value over the pointer writes in JSON and in postcard what its twin writes, and reads
    /// back, for each type over the pointer.
    #[cfg(feature = "serde")]
    #[test]
    fn serde_writes_what_the_twin_writes() {
      for_each_decimal!(check_serde, $pointer);
    }

    /// `quickcheck` draws and shrinks a value over the pointer as its twin, for each type over the
    /// pointer.
    #[cfg(feature = "quickcheck")]
    #[test]
    fn quickcheck_draws_and_shrinks_as_for_the_twin() {
      for_each_decimal!(check_quickcheck, $pointer);
    }

    /// `proptest` draws and shrinks a value over the pointer as its twin, for each type over the
    /// pointer.
    #[cfg(feature = "proptest")]
    #[test]
    fn proptest_draws_and_shrinks_as_for_the_twin() {
      for_each_decimal!(check_proptest, $pointer);
    }

    /// `arbitrary` reads a value over the pointer as its twin, for each type over the pointer.
    #[cfg(feature = "arbitrary")]
    #[test]
    fn arbitrary_reads_as_for_the_twin() {
      for_each_decimal!(check_arbitrary, $pointer);
    }

    /// A `Scaled` over a decimal over the pointer parses, writes and computes as the `Scaled` over
    /// its twin, with the same scales, for each type over the pointer.
    #[test]
    fn scaled_matches_the_scaled_twin() {
      for_each_decimal!(check_scaled, $pointer);
    }

    /// Code bounded by `FixedDecimal` and code over any storage take a decimal over the pointer and
    /// give the results of its twin, for each type over the pointer.
    #[test]
    fn generic_code_takes_the_decimals() {
      for_each_type!(generic_code_agrees, $pointer);
    }

    /// `-a` stores into the allocation of `a` in a pointer that holds its coefficient in place,
    /// and into an allocation of its own in a shared one, for each signed type over the pointer.
    #[test]
    fn negation_stores_where_the_pointer_stores() {
      check_negation_place::<U58, U18, Int<U256, $pointer<[u64; 4]>>>();
      if !tier().is_miri() {
        check_negation_place::<U135, U18, Int<U512, $pointer<[u64; 8]>>>();
        check_negation_place::<U155, U18, Int<U576, $pointer<[u64; 9]>>>();
      }
    }

    /// A literal reaches a decimal over the pointer through its twin: as the value that
    /// `from_inline` stores, whose parameter gives the literal its type, and as a literal of the
    /// twin, widened.
    #[test]
    fn literals_come_through_the_twin() {
      type Heap = Decimal<U58, U18, Int<U256, $pointer<[u64; 4]>>>;
      let stored = Heap::from_inline(dec!(1.5));
      assert_eq!(stored.to_string(), "1.5");
      let widened: Heap = dec!(1.5 as Decimal<U58, U18, i256>).widen();
      assert_eq!(widened, stored);
    }

    /// A decimal over the pointer has the size and the alignment of a pointer, and so has its
    /// `Option`, which takes the niche of the pointer, for each type over the pointer.
    #[test]
    fn decimals_have_the_layout_of_a_pointer() {
      let (size, align) = (size_of::<usize>(), align_of::<usize>());
      let pointer = [size, align, size, align];
      assert_eq!(layout::<Decimal<U39, U18, Uint<U192, $pointer<[u64; 3]>>>>(), pointer);
      assert_eq!(layout::<Decimal<U58, U18, Int<U256, $pointer<[u64; 4]>>>>(), pointer);
      assert_eq!(layout::<Decimal<U78, U18, Uint<U320, $pointer<[u64; 5]>>>>(), pointer);
      assert_eq!(layout::<Decimal<U135, U18, Int<U512, $pointer<[u64; 8]>>>>(), pointer);
      assert_eq!(layout::<Decimal<U155, U18, Int<U576, $pointer<[u64; 9]>>>>(), pointer);
      assert_eq!(layout::<Decimal<U308, U0, Uint<U1024, $pointer<[u64; 16]>>>>(), pointer);
    }
  };
}

/// Defines the tests of the decimals over the shared heap pointer `$pointer`, which never writes
/// the coefficient that it holds.
macro_rules! shared_tests {
  ($pointer:ident) => {
    /// A result over the pointer takes an allocation of its own, while the other owners of its
    /// operands keep their values and their addresses, and `clone`, `clone_from` and `repr` with
    /// `from_repr` share an allocation, for each type over the pointer.
    #[test]
    fn results_take_allocations_of_their_own() {
      for_each_type!(shared_agrees, $pointer);
    }
  };
}

/// The generic code of the checks takes a decimal over an inline storage too, as its own twin:
/// code bounded by `FixedDecimal` and by `DecimalAssign`, over any storage and over a storage that
/// holds its coefficient in place.
#[test]
fn generic_code_takes_an_inline_decimal() {
  check_generic_code::<U10, U8, i64>();
  check_assignments::<U10, U8, i64>();
}

/// In this build, which has the heap storages and their operators with one owned operand, a literal
/// on the right of `+` and of `+=` takes the type of the left operand, a decimal over an inline
/// storage or a `Scaled` over one: no operator of a heap decimal is a candidate for them.
#[test]
fn a_literal_operand_takes_the_type_of_an_inline_decimal() {
  type Price = Decimal<U10, U8>;
  let price: Price = "64250.5".parse().expect("a price");
  assert_eq!((price + dec!(0.5)).to_string(), "64251");
  let mut total = price;
  total += dec!(0.25);
  assert_eq!(total.to_string(), "64250.75");
  let scaled = Scaled::new(price, 2);
  assert_eq!((scaled + dec!(0.5)).to_string(), "64251.00");
  let mut total = scaled;
  total += dec!(0.25);
  assert_eq!(total.to_string(), "64250.75");
}

/// `Box<[u64; N]>`, which holds its coefficient in place.
#[cfg(any(feature = "std", feature = "alloc"))]
mod boxed {
  use super::*;

  heap_tests!(Box);

  /// The decimals in a `Box` are `Send` and `Sync`, as the `Box` is.
  #[test]
  fn decimals_are_send_and_sync() {
    send_and_sync::<Decimal<U39, U18, Uint<U192, Box<[u64; 3]>>>>();
    send_and_sync::<Decimal<U58, U18, Int<U256, Box<[u64; 4]>>>>();
  }

  /// The product of a decimal in a `Box` and one in an `Rc` is that of their twins, of the type of
  /// that product, in an inline storage.
  #[test]
  fn product_of_a_box_and_an_rc() {
    type Boxed = Decimal<U58, U18, Int<U256, Box<[u64; 4]>>>;
    type Shared = Decimal<U58, U18, Int<U256, Rc<[u64; 4]>>>;
    let rcs = pairs(Shared::from_inline);
    for (a, boxed) in pairs(Boxed::from_inline) {
      for (b, shared) in &rcs {
        let product = boxed.exact_mul(shared);
        let expected = a.exact_mul(b);
        assert_eq!(type_of(&product), type_of(&expected), "{a} * {b}");
        assert_eq!(product, expected, "{a} * {b}");
      }
    }
  }

  /// A result in a `Box` stays in the allocation of the operand that the operation owns: of its
  /// target for an assignment, by value or by reference, and for `clone_from`; of the left operand
  /// of `a + b` and `a + &b` and of the right one of `&a + b`; of the first item of a sum; and of
  /// the storage that `repr` moved out and `from_repr` took back.
  #[test]
  fn results_stay_in_the_allocation_of_an_operand() {
    for_each_type!(in_place_agrees, Box);
  }

  /// Code bounded by `DecimalAssign` and code over a storage that holds its coefficient in place
  /// take the decimals in a `Box`, and give the results of their twins.
  #[test]
  fn assignments_take_the_decimals_in_a_box() {
    check_assignments::<U39, U18, Uint<U192, Box<[u64; 3]>>>();
    check_assignments::<U58, U18, Int<U256, Box<[u64; 4]>>>();
    if !tier().is_miri() {
      check_assignments::<U78, U18, Uint<U320, Box<[u64; 5]>>>();
      check_assignments::<U135, U18, Int<U512, Box<[u64; 8]>>>();
      check_assignments::<U155, U18, Int<U576, Box<[u64; 9]>>>();
      check_assignments::<U308, U0, Uint<U1024, Box<[u64; 16]>>>();
    }
  }
}

/// `Rc<[u64; N]>`, which is neither `Send` nor `Sync`.
#[cfg(any(feature = "std", feature = "alloc"))]
mod rc {
  use super::*;

  heap_tests!(Rc);
  shared_tests!(Rc);
}

/// `Arc<[u64; N]>`.
#[cfg(all(any(feature = "std", feature = "alloc"), target_has_atomic = "ptr"))]
mod arc {
  use super::*;

  heap_tests!(Arc);
  shared_tests!(Arc);

  /// The decimals in an `Arc` are `Send` and `Sync`, as the `Arc` is.
  #[test]
  fn decimals_are_send_and_sync() {
    send_and_sync::<Decimal<U39, U18, Uint<U192, Arc<[u64; 3]>>>>();
    send_and_sync::<Decimal<U58, U18, Int<U256, Arc<[u64; 4]>>>>();
  }
}

/// `triomphe::Arc<[u64; N]>`.
#[cfg(all(feature = "triomphe", target_has_atomic = "ptr"))]
mod triomphe_arc {
  use super::*;

  heap_tests!(TriompheArc);
  shared_tests!(TriompheArc);

  /// The decimals in a `triomphe::Arc` are `Send` and `Sync`, as the pointer is.
  #[test]
  fn decimals_are_send_and_sync() {
    send_and_sync::<Decimal<U39, U18, Uint<U192, TriompheArc<[u64; 3]>>>>();
    send_and_sync::<Decimal<U58, U18, Int<U256, TriompheArc<[u64; 4]>>>>();
  }
}
