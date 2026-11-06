//! The casts between integers through `CheckedCastFrom`, `BoundedCastFrom`, `WidenFrom` and
//! `TruncateFrom`: between the native integers against the native conversions, and between `Uint`
//! and `Int` and the native integers, both ways, and between `Uint` and `Int` of other widths,
//! storages and signs against the bit model. Each pair checks in an instantiation of a generic
//! function of its own, whose frame holds the values of that pair alone.

use core::{any::type_name, num::TryFromIntError};

use typenum::{U3, U5, U12, U17, U33, U150, U193};

use super::*;
use crate::{BoundedCastFrom, CheckedCastFrom, IntRepr, TruncateFrom, UintRepr, WidenFrom, i0, u0};

/// A participant of the casts, as the tests read it: its sign, its width, and its value as the
/// pattern of the width.
pub(super) trait Model: Copy + Debug + PartialEq {
  /// Whether the type is signed.
  const SIGNED: bool;

  /// The number of bits of a value.
  const WIDTH: u32;

  /// The bits of the value, bit 0 first.
  fn pattern(self) -> Vec<bool>;

  /// The value whose bits are `pattern`, of the width.
  fn of_pattern(pattern: &[bool]) -> Self;
}

/// A native integer, whose `as` casts and limits the tests of the native pairs compare with.
pub(super) trait Native: Model {
  /// The smallest value.
  const MIN: Self;

  /// The largest value.
  const MAX: Self;

  /// `self as u128`: the value extended by its sign.
  fn as_u128(self) -> u128;

  /// `bits as Self`: the low bits of `bits`.
  fn from_u128(bits: u128) -> Self;
}

/// Implements [`Model`] and [`Native`] for the native integers, written `native: signed`.
macro_rules! native_model {
  ($($native:ident: $signed:literal),+ $(,)?) => {$(
    impl Model for $native {
      const SIGNED: bool = $signed;
      const WIDTH: u32 = $native::BITS;

      fn pattern(self) -> Vec<bool> {
        model::of_u128(self as u128, Self::WIDTH)
      }

      fn of_pattern(pattern: &[bool]) -> Self {
        model::to_u128(pattern) as $native
      }
    }

    impl Native for $native {
      const MIN: Self = $native::MIN;
      const MAX: Self = $native::MAX;

      fn as_u128(self) -> u128 {
        self as u128
      }

      fn from_u128(bits: u128) -> Self {
        bits as $native
      }
    }
  )+};
}

native_model!(
  u8: false,
  u16: false,
  u32: false,
  u64: false,
  u128: false,
  usize: false,
  i8: true,
  i16: true,
  i32: true,
  i64: true,
  i128: true,
  isize: true,
);

impl<B: FitsBits<R>, R: UintRepr> Model for Uint<B, R>
where
  Self: Patterned,
{
  const SIGNED: bool = false;
  const WIDTH: u32 = B::U32;

  fn pattern(self) -> Vec<bool> {
    self.storage_bits()[..B::U32 as usize].to_vec()
  }

  fn of_pattern(pattern: &[bool]) -> Self {
    Self::from_pattern(pattern)
  }
}

impl<B: FitsBits<R>, R: IntRepr> Model for Int<B, R>
where
  Self: Patterned,
{
  const SIGNED: bool = true;
  const WIDTH: u32 = B::U32;

  fn pattern(self) -> Vec<bool> {
    self.storage_bits()[..B::U32 as usize].to_vec()
  }

  fn of_pattern(pattern: &[bool]) -> Self {
    Self::from_pattern(pattern)
  }
}

/// Whether the native integer `value` is negative.
fn is_negative<T: Native>(value: T) -> bool {
  T::SIGNED && (value.as_u128() as i128) < 0
}

/// What the casts of a value into `T` give, from the bit model.
struct Expected<T> {
  /// The value modulo `2^Bits` of `T`, read in two's complement for a signed `T`.
  wrapped: T,
  /// Whether `T` holds the value.
  fits: bool,
  /// The value, or the bound of the range of `T` that it passes.
  saturated: T,
}

/// The pattern of the largest value of `bits` bits, unsigned or in two's complement, if `max`, and
/// of the smallest otherwise.
fn limit(bits: u32, signed: bool, max: bool) -> Vec<bool> {
  model::pattern(bits, |i| match (signed, max) {
    (false, max) => max,
    // The bits below the sign bit, or the sign bit alone.
    (true, max) => (i + 1 < bits) == max,
  })
}

/// What the casts of `value` into `T` give, from the bit model: the low bits of the value, extended
/// by its sign, at the width of `T`, which `T` holds when they extend back to the value.
fn expected<S: Model, T: Model>(value: S) -> Expected<T> {
  let p = value.pattern();
  let wrapped = model::wrap(&p, T::WIDTH, S::SIGNED);
  // The value and the wrapped value, each extended by its sign to a width that holds both.
  let wide = model::width(&p).max(T::WIDTH) + 1;
  let fits = model::extend(&p, wide, S::SIGNED) == model::extend(&wrapped, wide, T::SIGNED);
  let negative = S::SIGNED && model::sign(&p);
  let saturated = if fits {
    wrapped.clone()
  } else {
    limit(T::WIDTH, T::SIGNED, !negative)
  };
  Expected {
    wrapped: T::of_pattern(&wrapped),
    fits,
    saturated: T::of_pattern(&saturated),
  }
}

/// The patterns of the values of `values`, of `bits` bits.
fn patterns_of(values: Vec<i128>, bits: u32) -> Vec<Vec<bool>> {
  values
    .into_iter()
    .map(|value| pattern_of(value, bits))
    .collect()
}

/// The values of `T` from which the casts are checked: every value of up to 8 bits and those of
/// [`large_values`] of 9 to 16 bits, as the tier takes them, and above 16 bits the patterns of
/// [`wide_patterns`]; under Miri the patterns of `model::few_patterns`.
pub(super) fn samples<T: Model>() -> Vec<T> {
  let (bits, signed) = (T::WIDTH, T::SIGNED);
  let patterns = match bits {
    _ if tier().is_miri() => model::few_patterns(bits, 64),
    0..=8 => patterns_of(small_values(bits, signed, true), bits),
    9..=16 => patterns_of(large_values(bits, signed), bits),
    17..=64 => wide_patterns::<1>(bits),
    65..=128 => wide_patterns::<2>(bits),
    129..=256 => wide_patterns::<4>(bits),
    _ => wide_patterns::<16>(bits),
  };
  patterns
    .iter()
    .map(|pattern| T::of_pattern(pattern))
    .collect()
}

/// Checks the casts into `T` of each value of `samples` through `CheckedCastFrom` and
/// `BoundedCastFrom` against the bit model: the checked, strict, wrapping and saturating casts, and
/// the unchecked cast of each value that `T` holds.
fn check_cast<S: Model, T: Model + BoundedCastFrom<S>>(samples: &[S]) {
  for &value in samples {
    let expected = expected::<S, T>(value);
    let name = || format!("{value:?} into {}", type_name::<T>());
    assert_eq!(
      T::checked_cast_from(value),
      expected.fits.then_some(expected.wrapped),
      "{}",
      name()
    );
    assert_eq!(T::wrapping_cast_from(value), expected.wrapped, "{}", name());
    assert_eq!(
      T::saturating_cast_from(value),
      expected.saturated,
      "{}",
      name()
    );
    if expected.fits {
      assert_eq!(T::strict_cast_from(value), expected.wrapped, "{}", name());
      // SAFETY: `checked_cast_from` returns `Some`, as asserted above, which is the precondition of
      // `unchecked_cast_from`.
      let unchecked = unsafe { T::unchecked_cast_from(value) };
      assert_eq!(unchecked, expected.wrapped, "{}", name());
    }
  }
}

/// Checks `widen` from `S` into `T`, which holds every value of `S`, at each value of `samples`
/// against the bit model.
fn check_widen<S: Model, T: Model + WidenFrom<S>>(samples: &[S]) {
  for &value in samples {
    let expected = expected::<S, T>(value);
    let name = || format!("{value:?} into {}", type_name::<T>());
    assert!(expected.fits, "{}", name());
    assert_eq!(T::widen_from(value), expected.wrapped, "{}", name());
  }
}

/// Checks `truncate` and its checked and saturating forms from `S` into `T` at each value of
/// `samples` against the bit model.
fn check_truncate<S: Model, T: Model + TruncateFrom<S>>(samples: &[S]) {
  for &value in samples {
    let expected = expected::<S, T>(value);
    assert_eq!(
      (
        T::truncate_from(value),
        T::checked_truncate_from(value),
        T::saturating_truncate_from(value),
      ),
      (
        expected.wrapped,
        expected.fits.then_some(expected.wrapped),
        expected.saturated,
      ),
      "{value:?} into {}",
      type_name::<T>()
    );
  }
}

/// Checks the casts from the native integer `S` into the native integer `T` at each value of
/// `samples` against the native conversions: the checked, the strict and the unchecked cast against
/// `TryFrom`, the wrapping one against `as`, and the saturating one against `TryFrom`, or the bound
/// of the range of `T` that a value outside it passes.
fn check_native_pair<S: Native, T: Native + BoundedCastFrom<S> + TryFrom<S>>(samples: &[S]) {
  for &value in samples {
    let checked = T::try_from(value).ok();
    let bound = if is_negative(value) { T::MIN } else { T::MAX };
    let name = || format!("{value:?} into {}", type_name::<T>());
    assert_eq!(T::checked_cast_from(value), checked, "{}", name());
    assert_eq!(
      T::wrapping_cast_from(value),
      T::from_u128(value.as_u128()),
      "{}",
      name()
    );
    assert_eq!(
      T::saturating_cast_from(value),
      checked.unwrap_or(bound),
      "{}",
      name()
    );
    if let Some(checked) = checked {
      assert_eq!(T::strict_cast_from(value), checked, "{}", name());
      // SAFETY: `checked_cast_from` returns `Some`, as asserted above, which is the precondition of
      // `unchecked_cast_from`.
      let unchecked = unsafe { T::unchecked_cast_from(value) };
      assert_eq!(unchecked, checked, "{}", name());
    }
  }
}

/// Checks `widen` from the native integer `S` into the native integer `T` at each value of
/// `samples`: the value, which `TryFrom` gives too.
fn check_native_widen<S: Native, T: Native + WidenFrom<S> + TryFrom<S>>(samples: &[S]) {
  for &value in samples {
    assert_eq!(
      Some(T::widen_from(value)),
      T::try_from(value).ok(),
      "{value:?} into {}",
      type_name::<T>()
    );
  }
}

/// Checks `truncate` and its checked and saturating forms from the native integer `S` into the
/// native integer `T` at each value of `samples` against `as`, `TryFrom` and the bound that a
/// value outside the range of `T` passes.
fn check_native_truncate<S: Native, T: Native + TruncateFrom<S> + TryFrom<S>>(samples: &[S]) {
  for &value in samples {
    let checked = T::try_from(value).ok();
    let bound = if is_negative(value) { T::MIN } else { T::MAX };
    assert_eq!(
      (
        T::truncate_from(value),
        T::checked_truncate_from(value),
        T::saturating_truncate_from(value),
      ),
      (
        T::from_u128(value.as_u128()),
        checked,
        checked.unwrap_or(bound)
      ),
      "{value:?} into {}",
      type_name::<T>()
    );
  }
}

/// Checks the casts from each type of `$sources` into each type of `$targets`, at the values of
/// [`samples`] of the source, each pair in an instantiation of `$check` of its own.
macro_rules! cast_pairs {
  ($check:ident: [$($source:ty),+ $(,)?] => $targets:tt) => {$(
    cast_pairs!(@from $check: $source => $targets);
  )+};
  (@from $check:ident: $source:ty => [$($target:ty),+ $(,)?]) => {{
    let samples = samples::<$source>();
    $($check::<$source, $target>(&samples);)+
  }};
}

/// Checks the pairs of `$source => $targets;`, at the values of [`samples`] of the source, each
/// pair in an instantiation of `$check` of its own.
macro_rules! pairs {
  ($check:ident: $($source:ty => $($target:ty),+;)+) => {$({
    let samples = samples::<$source>();
    $($check::<$source, $target>(&samples);)+
  })+};
}

/// The casts between every pair of native integers, from every value of 8 bits, the values of 16
/// bits that the tier takes and the structured patterns of the wider ones, against the native
/// conversions; under Miri from the patterns of `model::few_patterns`.
#[test]
fn native_pairs_match_the_native_conversions() {
  cast_pairs!(
    check_native_pair: [u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize]
      => [u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize]
  );
}

/// `widen` and `truncate` between the native integers at every pair of the native tables, which
/// take `usize` and `isize` as 16 bits, from no more than 16 bits and into themselves only, against
/// the native conversions.
#[test]
fn native_widen_and_truncate_match_the_native_conversions() {
  pairs!(check_native_widen:
    u8 => u8, u16, u32, u64, u128, usize;
    u16 => u16, u32, u64, u128, usize;
    u32 => u32, u64, u128;
    u64 => u64, u128;
    u128 => u128;
    usize => usize;
    i8 => i8, i16, i32, i64, i128, isize;
    i16 => i16, i32, i64, i128, isize;
    i32 => i32, i64, i128;
    i64 => i64, i128;
    i128 => i128;
    isize => isize;
  );
  pairs!(check_native_truncate:
    u8 => u8;
    u16 => u8, u16;
    u32 => u8, u16, u32;
    u64 => u8, u16, u32, u64;
    u128 => u8, u16, u32, u64, u128;
    usize => u8, u16, usize;
    i8 => i8;
    i16 => i8, i16;
    i32 => i8, i16, i32;
    i64 => i8, i16, i32, i64;
    i128 => i8, i16, i32, i64, i128;
    isize => i8, i16, isize;
  );
}

/// The casts from twelve numo integers, of widths from zero bits to 200 and of every kind of
/// storage, into every native integer, from the values of [`samples`], against the bit model.
#[test]
fn numo_integers_cast_into_native_integers() {
  cast_pairs!(
    check_cast: [
      Uint<U0>,
      Uint<U7, u8>,
      Int<U7, i8>,
      Uint<U8, [u8; 1]>,
      Int<U12, [u16; 1]>,
      Uint<U33, [u32; 2]>,
      Int<U64, i64>,
      Uint<U65, [u64; 2]>,
      Int<U100, [u128; 1]>,
      Uint<U128, u128>,
      Int<U129, [u64; 3]>,
      Uint<U200, [u8; 25]>,
    ] => [u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize]
  );
}

/// The casts from every native integer into the twelve numo integers of
/// [`numo_integers_cast_into_native_integers`], from the values of [`samples`], against the bit
/// model.
#[test]
fn native_integers_cast_into_numo_integers() {
  cast_pairs!(
    check_cast: [u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize] => [
      Uint<U0>,
      Uint<U7, u8>,
      Int<U7, i8>,
      Uint<U8, [u8; 1]>,
      Int<U12, [u16; 1]>,
      Uint<U33, [u32; 2]>,
      Int<U64, i64>,
      Uint<U65, [u64; 2]>,
      Int<U100, [u128; 1]>,
      Uint<U128, u128>,
      Int<U129, [u64; 3]>,
      Uint<U200, [u8; 25]>,
    ]
  );
}

/// `widen` and `truncate` between numo integers and the native integers, both ways, at the pairs of
/// the same sign whose widths the bounds accept, `usize` and `isize` among them as 16 bits, against
/// the bit model.
#[test]
fn numo_and_native_integers_widen_and_truncate() {
  pairs!(check_widen:
    Uint<U0> => u8, u64, usize, Uint<U7, u8>;
    Uint<U7, u8> => u8, u16, u32, u64, u128, usize;
    Int<U7, i8> => i8, i16, i32, i64, i128, isize;
    Uint<U8, [u8; 1]> => u8, u16, usize;
    Int<U12, [u16; 1]> => i16, i64, isize;
    Uint<U16, [u8; 2]> => u16, u32, usize;
    Uint<U33, [u32; 2]> => u64, u128;
    Int<U64, i64> => i64, i128;
    Uint<U65, [u64; 2]> => u128;
    Int<U100, [u128; 1]> => i128;
    Uint<U128, u128> => u128;
    u8 => Uint<U8, [u8; 1]>, Uint<U33, [u32; 2]>, Uint<U128, u128>, Uint<U200, [u8; 25]>;
    u16 => Uint<U33, [u32; 2]>, Uint<U65, [u64; 2]>;
    u64 => Uint<U65, [u64; 2]>, Uint<U200, [u8; 25]>;
    u128 => Uint<U128, u128>, Uint<U200, [u8; 25]>;
    i8 => Int<U12, [u16; 1]>, Int<U64, i64>, Int<U129, [u64; 3]>;
    i64 => Int<U64, i64>, Int<U100, [u128; 1]>;
    i128 => Int<U129, [u64; 3]>;
  );
  pairs!(check_truncate:
    Uint<U200, [u8; 25]> => u8, u16, u32, u64, u128;
    Int<U129, [u64; 3]> => i8, i16, i32, i64, i128;
    Uint<U65, [u64; 2]> => u8, u64, Uint<U0>;
    Int<U100, [u128; 1]> => i8, i64, Int<U7, i8>;
    Uint<U33, [u32; 2]> => u8, u16, u32;
    Int<U12, [u16; 1]> => i8, Int<U7, i8>;
    u8 => Uint<U0>, Uint<U7, u8>, Uint<U8, [u8; 1]>;
    u64 => Uint<U33, [u32; 2]>, Uint<U7, u8>;
    u128 => Uint<U65, [u64; 2]>, Uint<U128, u128>;
    i8 => Int<U7, i8>;
    i128 => Int<U100, [u128; 1]>, Int<U64, i64>;
    usize => Uint<U0>, Uint<U7, u8>, Uint<U16, [u8; 2]>;
    isize => Int<U7, i8>, Int<U12, [u16; 1]>, Int<U16, i16>;
  );
}

/// The casts between numo integers of the same storage and other widths, of native and array
/// storages, of each limb type and another, of both signs both ways, of zero bits and one signed
/// bit, from the values of [`samples`], against the bit model, and in an optimized build also
/// those of 256 and 1024 bits.
#[test]
fn numo_integers_cast_between_widths_storages_and_signs() {
  pairs!(check_cast:
    // The same storage at another width.
    Uint<U7, u8> => Uint<U5, u8>, Uint<U8, u8>;
    Int<U12, [u16; 1]> => Int<U9, [u16; 1]>, Int<U16, [u16; 1]>;
    Uint<U200, [u64; 4]> => Uint<U193, [u64; 4]>, Uint<U256, [u64; 4]>;
    Int<U129, [u64; 3]> => Int<U150, [u64; 3]>, Int<U65, [u64; 3]>;
    // A native storage and an array at the same width.
    Uint<U65, u128> => Uint<U65, [u64; 2]>;
    Uint<U65, [u64; 2]> => Uint<U65, u128>;
    Int<U100, i128> => Int<U100, [u128; 1]>;
    Int<U100, [u128; 1]> => Int<U100, i128>;
    Int<U7, i8> => Int<U7, [u64; 1]>;
    Uint<U8, u8> => Uint<U8, [u8; 1]>;
    // Each limb type with another, of both signs.
    Uint<U24, [u8; 3]> => Uint<U24, [u16; 2]>, Int<U24, [u32; 1]>;
    Int<U24, [u16; 2]> => Int<U24, [u8; 3]>, Uint<U12, [u64; 1]>;
    Uint<U65, [u8; 9]> => Int<U65, [u32; 3]>, Uint<U63, [u128; 1]>;
    Int<U65, [u32; 3]> => Uint<U65, [u8; 9]>, Int<U129, [u16; 9]>;
    Uint<U129, [u16; 9]> => Uint<U129, [u64; 3]>, Int<U100, [u128; 1]>;
    Int<U129, [u64; 3]> => Int<U129, [u8; 17]>, Uint<U129, [u128; 2]>;
    Uint<U200, [u128; 2]> => Int<U200, [u8; 25]>, Uint<U200, [u32; 7]>;
    Int<U200, [u32; 7]> => Int<U129, [u64; 3]>, Uint<U200, [u16; 13]>;
    // Both signs, both ways, at the widths next to each other.
    Uint<U8, u8> => Int<U8, i8>, Int<U9, i16>, Int<U7, i8>;
    Int<U8, i8> => Uint<U8, u8>, Uint<U7, u8>, Uint<U9, u16>;
    Uint<U64, u64> => Int<U64, i64>, Int<U65, [u64; 2]>;
    Int<U64, [u64; 1]> => Uint<U64, u64>, Uint<U63, [u32; 2]>;
    // Zero bits and one signed bit.
    Uint<U0> => Int<U8, i8>, Int<U0, [u8; 0]>;
    Int<U0, [u8; 0]> => Uint<U3, u8>;
    Int<U8, i8> => Uint<U0, u8>, Int<U0, [u16; 1]>, Int<U1, i8>;
    Int<U1, i8> => Uint<U1, u8>, Int<U200, [u64; 4]>, Uint<U0>;
    Uint<U8, u8> => Int<U1, [u8; 1]>;
    Uint<U1, u8> => Int<U1, i8>;
  );
  if tier().is_full() {
    pairs!(check_cast:
      Uint<U256, [u64; 4]> => Int<U1024, [u32; 32]>, Int<U256, [u8; 32]>;
      Int<U1024, [u64; 16]> => Uint<U256, [u8; 32]>, Int<U256, [u16; 16]>;
      Uint<U1024, [u128; 8]> => Int<U1024, [u64; 16]>, Uint<U200, [u64; 4]>;
    );
  }
}

/// The inherent casts of `Uint` and `Int` give what the traits of the target give, and infer the
/// target from the expected type: between numo integers of both signs, and with the native
/// integers, `usize` included.
#[test]
fn inherent_casts_agree_with_the_traits() {
  // The inherent casts of `$x` into `$target` and those of the traits, and the unchecked cast where
  // the checked one returns a value.
  macro_rules! agree {
    ($x:expr => $($target:ty),+) => {{
      let x = $x;
      $(
        assert_eq!(
          (
            x.checked_cast::<$target>(),
            x.wrapping_cast::<$target>(),
            x.saturating_cast::<$target>(),
          ),
          (
            <$target>::checked_cast_from(x),
            <$target>::wrapping_cast_from(x),
            <$target>::saturating_cast_from(x),
          ),
          "{x:?} into {}",
          type_name::<$target>()
        );
        if let Some(checked) = x.checked_cast::<$target>() {
          // SAFETY: `checked_cast` returns `Some`, which is the precondition of `unchecked_cast`.
          let unchecked = unsafe { x.unchecked_cast::<$target>() };
          assert_eq!(unchecked, checked, "{x:?} into {}", type_name::<$target>());
        }
      )+
    }};
  }
  agree!(u7::MAX => u8, i8, i7, Uint<U3, u8>, i256, usize, isize);
  agree!(i7::MIN => u8, i8, u7, Int<U3, i8>, u256, usize, isize);
  agree!(u256::MAX => u128, i128, u7, i7, i256, Uint<U200, [u8; 25]>);
  agree!(i256::MIN => u128, i128, u7, i7, u256, Int<U200, [u8; 25]>);
  agree!(Int::<U12>::new(-5) => u8, i8, u7, i7, u256, i256);

  assert_eq!(u7::new(100).strict_cast::<i8>(), 100);
  assert_eq!(
    i7::new(-5).strict_cast::<i256>(),
    i256::from_limbs([-5i64 as u64, u64::MAX, u64::MAX, u64::MAX])
  );
  assert_eq!(u7::MAX.widen::<u8>(), 127);
  assert_eq!(i7::MIN.widen::<isize>(), -64);
  let wide: u256 = Uint::<U16>::new(5).widen();
  assert_eq!(wide, u256::from_limbs([5, 0, 0, 0]));
  let pointer: usize = Uint::<U16>::MAX.widen();
  assert_eq!(pointer, 65535);
  assert_eq!(u256::MAX.truncate::<u8>(), u8::MAX);
  assert_eq!(i256::MIN.truncate::<i7>(), i7::ZERO);
  assert_eq!(u256::MAX.checked_truncate::<u128>(), None);
  assert_eq!(i7::new(-5).checked_truncate::<Int<U3, i8>>(), None);
  assert_eq!(
    i7::new(-3).checked_truncate::<Int<U3, i8>>(),
    Some(Int::<U3, i8>::new(-3))
  );
  assert_eq!(u256::MAX.saturating_truncate::<u7>(), u7::MAX);
  assert_eq!(i256::MIN.saturating_truncate::<i7>(), i7::MIN);
}

/// `strict_cast`, inherent and through `CheckedCastFrom`, panics in every build on a value outside
/// the range of the target, with the message of the native `strict_cast`, and reports the caller:
/// between numo integers of both signs, and with the native integers both ways.
#[test]
fn strict_casts_panic_outside_the_range() {
  let message = "attempt to cast integer with overflow".to_string();
  type Wide = Uint<U200, [u8; 25]>;
  panics_here!(u7::MAX.strict_cast::<i7>(), message);
  panics_here!(i7::new(-1).strict_cast::<u8>(), message);
  panics_here!(i256::MIN.strict_cast::<i128>(), message);
  panics_here!(u256::MAX.strict_cast::<i256>(), message);
  panics_here!(Wide::MAX.strict_cast::<u128>(), message);
  panics_here!(u8::strict_cast_from(i7::MIN), message);
  panics_here!(<i7 as CheckedCastFrom<u64>>::strict_cast_from(64), message);
  panics_here!(u7::strict_cast_from(-1i8), message);
  panics_here!(Int::<U0, [u8; 0]>::strict_cast_from(1u8), message);
}

/// The saturating cast of `x` into a `u64`, in code generic over the width alone.
fn saturating_into_u64<B: BitWidth>(x: Uint<B>) -> u64 {
  x.saturating_cast()
}

/// The saturating cast of `n` into `Uint<B>`, in code generic over the width alone.
fn saturating_from_u64<B: BitWidth>(n: u64) -> Uint<B> {
  Uint::<B>::saturating_cast_from(n)
}

/// The saturating cast of `x` into an `i128`, in code generic over the width alone.
fn saturating_into_i128<B: BitWidth>(x: Int<B>) -> i128 {
  x.saturating_cast()
}

/// The saturating cast of `n` into `Int<B>`, in code generic over the width alone.
fn saturating_from_i128<B: BitWidth>(n: i128) -> Int<B> {
  Int::<B>::saturating_cast_from(n)
}

/// Checks the saturating casts of `Uint<B>` and `Int<B>`, of `bits` bits, with `u64` and `i128`, in
/// code generic over the width alone, at the limits of both and at zero.
fn check_casts_over_the_width<B: BitWidth>(bits: u32) {
  // The largest value of `Uint<B>` that a `u64` holds, and the limits of `Int<B>` that an `i128`
  // holds.
  let max = if bits >= 64 {
    u64::MAX
  } else {
    (1 << bits) - 1
  };
  let (min, signed_max) = match bits {
    0 => (0, 0),
    1..=128 => (i128::MIN >> (128 - bits), i128::MAX >> (128 - bits)),
    _ => (i128::MIN, i128::MAX),
  };
  assert_eq!(
    (
      saturating_into_u64(Uint::<B>::MAX),
      saturating_into_u64(Uint::<B>::ZERO),
      saturating_into_u64(saturating_from_u64::<B>(u64::MAX)),
      saturating_from_u64::<B>(0),
    ),
    (max, 0, max, Uint::<B>::ZERO),
    "{bits} bits"
  );
  assert_eq!(
    (
      saturating_into_i128(Int::<B>::MIN),
      saturating_into_i128(Int::<B>::MAX),
      saturating_into_i128(saturating_from_i128::<B>(i128::MIN)),
      saturating_into_i128(saturating_from_i128::<B>(i128::MAX)),
      saturating_from_i128::<B>(0),
    ),
    (min, signed_max, min, signed_max, Int::<B>::ZERO),
    "{bits} bits"
  );
}

/// The casts in code generic over the width, whose bound `B: BitWidth` alone gives `Uint<B>` and
/// `Int<B>` the casts with the native integers, at widths from zero bits to 1024.
#[test]
fn casts_in_code_generic_over_the_width() {
  check_casts_over_the_width::<U0>(0);
  check_casts_over_the_width::<U1>(1);
  check_casts_over_the_width::<U7>(7);
  check_casts_over_the_width::<U8>(8);
  check_casts_over_the_width::<U64>(64);
  check_casts_over_the_width::<U65>(65);
  check_casts_over_the_width::<U128>(128);
  check_casts_over_the_width::<U129>(129);
  check_casts_over_the_width::<U200>(200);
  check_casts_over_the_width::<U1024>(1024);
}

/// Checks the casts between the signs of `$x`, a value of `$ty` of the sign `$sign` whose pattern
/// is `$p`, through the inherent `const fn`s and the traits, against the bit model: the cast to the
/// other sign, if the top bit of the pattern is clear, and otherwise `None`, the bound of the
/// other sign, `MAX` of `Int` or zero, and no strict cast.
macro_rules! sign_casts {
  (unsigned; $ty:ty, $x:expr, $p:expr) => {{
    let (x, p): ($ty, &[bool]) = ($x, $p);
    let cast = x.cast_signed();
    let fits = !model::sign(p);
    let max = <<$ty as UnsignedBits>::Signed>::MAX;
    assert_eq!(
      both!(UnsignedBits::checked_cast_signed(x)),
      fits.then_some(cast),
      "{}",
      model::text(p)
    );
    assert_eq!(
      both!(UnsignedBits::saturating_cast_signed(x)),
      if fits { cast } else { max },
      "{}",
      model::text(p)
    );
    if fits {
      assert_eq!(
        both!(UnsignedBits::strict_cast_signed(x)),
        cast,
        "{}",
        model::text(p)
      );
    }
  }};
  (signed; $ty:ty, $x:expr, $p:expr) => {{
    let (x, p): ($ty, &[bool]) = ($x, $p);
    let cast = x.cast_unsigned();
    let fits = !model::sign(p);
    let zero = <<$ty as IntegerMath>::Magnitude>::ZERO;
    assert_eq!(
      both!(SignedBits::checked_cast_unsigned(x)),
      fits.then_some(cast),
      "{}",
      model::text(p)
    );
    assert_eq!(
      both!(SignedBits::saturating_cast_unsigned(x)),
      if fits { cast } else { zero },
      "{}",
      model::text(p)
    );
    if fits {
      assert_eq!(
        both!(SignedBits::strict_cast_unsigned(x)),
        cast,
        "{}",
        model::text(p)
      );
    }
  }};
}

/// Checks the casts between the signs of `$ty`, of the sign `$sign` and of `$bits` bits, whose
/// values `$from` creates from an `i128`, at each value of `$values`.
macro_rules! sweep_sign_casts {
  ($sign:ident; $ty:ty, $bits:expr, $values:expr, $from:expr, $value:expr, $magnitude:expr) => {{
    let bits = $bits;
    let from: fn(i128) -> $ty = $from;
    for a in $values {
      sign_casts!($sign; $ty, from(a), &pattern_of(a, bits));
    }
  }};
}

/// Checks the casts between the signs at the width `B`, `bits` bits, at most 8, in `u8` and `i8`,
/// `[u8; 1]`, `[u8; 2]` and `[u16; 1]`, at the values of [`bit_values`] for a storage that every
/// tier checks.
fn sign_casts_of_small_widths<B>(bits: u32)
where
  B: FitsBits<u8> + FitsBits<i8> + FitsBits<[u8; 1]> + FitsBits<[u8; 2]> + FitsBits<[u16; 1]>,
{
  let values = |bits, signed| bit_values(bits, signed, true);
  sweep_storage!(sweep_sign_casts, values, native B, bits, u8, i8);
  sweep_storage!(sweep_sign_casts, values, array B, bits, [u8; 1]);
  sweep_storage!(sweep_sign_casts, values, array B, bits, [u8; 2]);
  sweep_storage!(sweep_sign_casts, values, array B, bits, [u16; 1]);
}

/// Checks the casts between the signs at the width `B`, `bits` bits, from 9 to 16, in `u16` and
/// `i16`, `[u8; 2]` and `[u16; 1]`, at the values of [`bit_values`].
fn sign_casts_of_large_widths<B>(bits: u32)
where
  B: FitsBits<u16> + FitsBits<i16> + FitsBits<[u8; 2]> + FitsBits<[u16; 1]>,
{
  let values = |bits, signed| bit_values(bits, signed, true);
  sweep_storage!(sweep_sign_casts, values, native B, bits, u16, i16);
  sweep_storage!(sweep_sign_casts, values, array B, bits, [u8; 2]);
  sweep_storage!(sweep_sign_casts, values, array B, bits, [u16; 1]);
}

/// The casts between the signs of every value of each width up to 16 bits, as the tier takes
/// them, in native integers and arrays, through the inherent `const fn`s and the traits, against
/// the bit model: zero bits, one bit, and partial and full widths. Miri checks 0, 1, 7, 8, 9 and 16
/// bits.
#[test]
fn sign_casts_of_up_to_16_bits() {
  sign_casts_of_small_widths::<U0>(0);
  sign_casts_of_small_widths::<U1>(1);
  sign_casts_of_small_widths::<U7>(7);
  sign_casts_of_small_widths::<U8>(8);
  sign_casts_of_large_widths::<U9>(9);
  sign_casts_of_large_widths::<U16>(16);
  if !tier().is_miri() {
    sign_casts_of_small_widths::<U2>(2);
    sign_casts_of_small_widths::<U3>(3);
    sign_casts_of_small_widths::<U5>(5);
    sign_casts_of_large_widths::<U12>(12);
  }
}

/// Checks the casts between the signs of `Uint` and `Int` of `$bits` bits, the width `$width`, in
/// each storage of `$storages`, written `unsigned, signed;`, at the patterns of [`wide_patterns`]
/// in `$words` words of 64 bits, through the inherent `const fn`s and the traits, each storage in a
/// frame of its own.
macro_rules! wide_sign_casts {
  ($width:ty, $bits:literal, $words:literal; $($uint:ty, $int:ty);+ $(;)?) => {{
    let patterns = wide_patterns::<$words>($bits);
    $({
      #[inline(never)]
      fn storage(patterns: &[Vec<bool>]) {
        type U = Uint<$width, $uint>;
        type I = Int<$width, $int>;
        for p in patterns {
          sign_casts!(unsigned; U, U::from_pattern(p), p);
          sign_casts!(signed; I, I::from_pattern(p), p);
        }
      }
      storage(&patterns);
    })+
  }};
}

/// The casts between the signs at the bit patterns of 24 to 1024 bits, in storages of each kind,
/// through the inherent `const fn`s and the traits, against the bit model. An optimized build adds
/// 1024 bits; Miri checks 65 and 129 bits, in two storages each.
#[test]
fn sign_casts_of_wide_widths() {
  wide_sign_casts!(U65, 65, 2; u128, i128; [u64; 2], [u64; 2]);
  wide_sign_casts!(U129, 129, 3; [u8; 17], [u8; 17]; [u128; 2], [u128; 2]);
  if !tier().is_miri() {
    wide_sign_casts!(U24, 24, 1; u32, i32; [u16; 2], [u16; 2]);
    wide_sign_casts!(U64, 64, 1; u64, i64; [u32; 2], [u32; 2]);
    wide_sign_casts!(U200, 200, 4; [u64; 4], [u64; 4]; [u8; 25], [u8; 25]);
  }
  if tier().is_full() {
    wide_sign_casts!(U1024, 1024, 16; [u64; 16], [u64; 16]; [u128; 8], [u128; 8]);
  }
}

/// `strict_cast_signed` and `strict_cast_unsigned`, inherent and through the traits, panic in
/// every build on a value that the other sign does not hold, with the message of the native
/// methods, and report the caller, in native integers and arrays and at one bit.
#[test]
fn strict_sign_casts_panic_outside_the_range() {
  let message = "attempt to cast integer with overflow".to_string();
  type Bytes = Uint<U24, [u8; 3]>;
  panics_here!(u7::MAX.strict_cast_signed(), message);
  panics_here!(UnsignedBits::strict_cast_signed(u7::new(64)), message);
  panics_here!(Bytes::MAX.strict_cast_signed(), message);
  panics_here!(u256::MAX.strict_cast_signed(), message);
  panics_here!(Uint::<U1, u8>::ONE.strict_cast_signed(), message);
  panics_here!(i7::MIN.strict_cast_unsigned(), message);
  panics_here!(SignedBits::strict_cast_unsigned(i7::new(-1)), message);
  panics_here!(i256::MIN.strict_cast_unsigned(), message);
  panics_here!(Int::<U1, i8>::MIN.strict_cast_unsigned(), message);
}

/// The casts between the signs in constant contexts, in a native integer and in an array, at zero
/// bits and at one bit, where one has no signed value and -1 no unsigned one.
#[test]
fn sign_casts_in_constant_contexts() {
  type Bytes = Uint<U24, [u8; 3]>;
  type SignedBytes = Int<U24, [u8; 3]>;
  type U1x8 = Uint<U1, u8>;
  type I1x8 = Int<U1, i8>;
  const UNSIGNED: (Option<i7>, i7, i7, Option<SignedBytes>, SignedBytes) = (
    u7::MAX.checked_cast_signed(),
    u7::MAX.saturating_cast_signed(),
    u7::new(5).strict_cast_signed(),
    Bytes::from_limbs([1, 2, 3]).checked_cast_signed(),
    Bytes::MAX.saturating_cast_signed(),
  );
  const SIGNED: (Option<u7>, u7, u7, Option<Bytes>, Bytes) = (
    i7::MIN.checked_cast_unsigned(),
    i7::MIN.saturating_cast_unsigned(),
    i7::MAX.strict_cast_unsigned(),
    SignedBytes::from_limbs([0, 0, 0x80]).checked_cast_unsigned(),
    SignedBytes::from_limbs([0, 0, 0x80]).saturating_cast_unsigned(),
  );
  const ZERO_BITS: (Option<i0>, Option<u0>) = (
    u0::ZERO.checked_cast_signed(),
    i0::ZERO.checked_cast_unsigned(),
  );
  const ONE_BIT: (Option<I1x8>, I1x8, Option<U1x8>, U1x8) = (
    U1x8::ONE.checked_cast_signed(),
    U1x8::ONE.saturating_cast_signed(),
    I1x8::MIN.checked_cast_unsigned(),
    I1x8::MIN.saturating_cast_unsigned(),
  );

  assert_eq!(
    UNSIGNED,
    (
      None,
      i7::MAX,
      i7::new(5),
      Some(SignedBytes::from_limbs([1, 2, 3])),
      SignedBytes::MAX
    )
  );
  assert_eq!(SIGNED, (None, u7::ZERO, u7::new(63), None, Bytes::ZERO));
  assert_eq!(ZERO_BITS, (Some(i0::ZERO), Some(u0::ZERO)));
  assert_eq!(ONE_BIT, (None, I1x8::ZERO, None, U1x8::ZERO));
}

/// The errors of the native conversions for a value above the range of the target, of the kind
/// `PosOverflow`, and below it, of the kind `NegOverflow`.
fn errors() -> (TryFromIntError, TryFromIntError) {
  (
    u8::try_from(u16::MAX).unwrap_err(),
    u8::try_from(-1i8).unwrap_err(),
  )
}

/// Checks `From` from the native integer `S` into `T`, whose width holds every value of `S`, at
/// each value of `samples` against the bit model.
fn check_from<S: Model, T: Model + From<S>>(samples: &[S]) {
  for &value in samples {
    let expected = expected::<S, T>(value);
    let name = || format!("{value:?} into {}", type_name::<T>());
    assert!(expected.fits, "{}", name());
    assert_eq!(T::from(value), expected.wrapped, "{}", name());
  }
}

/// Checks `TryFrom` from `S` into `T` at each value of `samples` against the bit model: the value,
/// or the error that the native conversions return for a value above the range of `T` or below
/// it.
fn check_try_from<S: Model, T: Model + TryFrom<S, Error = TryFromIntError>>(samples: &[S]) {
  let (above, below) = errors();
  for &value in samples {
    let expected = expected::<S, T>(value);
    let negative = S::SIGNED && model::sign(&value.pattern());
    let error = if negative { below } else { above };
    assert_eq!(
      T::try_from(value),
      if expected.fits {
        Ok(expected.wrapped)
      } else {
        Err(error)
      },
      "{value:?} into {}",
      type_name::<T>()
    );
  }
}

/// `From` from each native integer of up to 128 bits into numo integers of each kind of storage
/// whose width holds every value of it: at least its width for an integer of the same sign, and one
/// bit more for an unsigned native integer into `Int`.
#[test]
fn native_integers_convert_into_numo_integers_where_lossless() {
  pairs!(check_from:
    u8 => Uint<U8, u8>, Uint<U8, [u8; 1]>, Uint<U200, [u64; 4]>, Int<U9, i16>, Int<U9, [u16; 1]>;
    u16 => Uint<U16, [u8; 2]>, Uint<U33, [u32; 2]>, Int<U17, i32>, Int<U129, [u64; 3]>;
    u32 => Uint<U32, u32>, Int<U33, [u32; 2]>, Uint<U200, [u8; 25]>;
    u64 => Uint<U64, u64>, Uint<U65, [u64; 2]>, Int<U65, i128>, Int<U200, [u128; 2]>;
    u128 => Uint<U128, u128>, Uint<U128, [u16; 8]>, Int<U129, [u64; 3]>;
    i8 => Int<U8, i8>, Int<U8, [u8; 1]>, Int<U100, [u128; 1]>;
    i16 => Int<U16, i16>, Int<U24, [u8; 3]>;
    i32 => Int<U32, i32>, Int<U33, [u32; 2]>;
    i64 => Int<U64, i64>, Int<U64, [u16; 4]>, Int<U200, [u64; 4]>;
    i128 => Int<U128, i128>, Int<U129, [u64; 3]>, Int<U200, [u8; 25]>;
  );
}

/// `TryFrom` from `usize` and `isize` into `Uint` and `Int` of every width, zero bits included,
/// and from numo integers into each native integer, against the bit model, with the errors of the
/// native conversions.
#[test]
fn numo_and_native_integers_try_from_each_other() {
  pairs!(check_try_from:
    usize => Uint<U0>, Uint<U7, u8>, Uint<U16, [u8; 2]>, Uint<U64, u64>, Uint<U200, [u64; 4]>,
      Int<U0, [u8; 0]>, Int<U7, i8>, Int<U64, i64>, Int<U65, [u64; 2]>;
    isize => Uint<U0>, Uint<U7, u8>, Uint<U64, u64>, Uint<U200, [u64; 4]>, Int<U0, [u8; 0]>,
      Int<U7, i8>, Int<U33, [u32; 2]>, Int<U64, i64>, Int<U129, [u64; 3]>;
  );
  cast_pairs!(
    check_try_from: [
      Uint<U0>,
      Uint<U7, u8>,
      Int<U7, i8>,
      Int<U12, [u16; 1]>,
      Uint<U65, [u64; 2]>,
      Int<U129, [u64; 3]>,
      Uint<U200, [u8; 25]>,
    ] => [u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize]
  );
}

/// The errors of `TryFrom` are those that the native conversions return in the same situation,
/// with their text: of the kind `PosOverflow` above the range of the target, and `NegOverflow`
/// below it.
#[test]
fn conversion_errors_are_those_of_the_native_conversions() {
  let (above, below) = errors();
  assert_ne!(above, below);
  assert_eq!(above, u8::try_from(256u16).unwrap_err());
  assert_eq!(below, u8::try_from(i64::MIN).unwrap_err());
  assert_eq!(u8::try_from(u256::MAX), Err(above));
  assert_eq!(i8::try_from(i256::MAX), Err(above));
  assert_eq!(u8::try_from(i256::MIN), Err(below));
  assert_eq!(i128::try_from(i256::MIN), Err(below));
  assert_eq!(Uint::<U7>::try_from(128usize), Err(above));
  assert_eq!(Uint::<U7>::try_from(-1isize), Err(below));
  assert_eq!(Int::<U7>::try_from(-65isize), Err(below));
  assert_eq!(above.to_string(), "number too large to fit in target type");
  assert_eq!(below.to_string(), "number too small to fit in target type");
}

/// `From<bool>` gives zero and one where the width holds one, from one bit for `Uint` and from two
/// for `Int`, and `TryFrom` into `bool` gives `false` and `true` back from zero and one, and the
/// errors of the native conversions above one and below zero, at every width, zero bits included.
#[test]
fn bool_converts_both_ways() {
  let (above, below) = errors();
  // `$ty` converts `false` and `true` to zero and one, and back.
  macro_rules! from_bool {
    ($($ty:ty),+ $(,)?) => {$(
      assert_eq!(
        (<$ty>::from(false), <$ty>::from(true)),
        (<$ty>::ZERO, <$ty>::ONE),
        "{}",
        type_name::<$ty>()
      );
      assert_eq!(
        (bool::try_from(<$ty>::ZERO), bool::try_from(<$ty>::ONE)),
        (Ok(false), Ok(true)),
        "{}",
        type_name::<$ty>()
      );
    )+};
  }
  from_bool!(
    Uint<U1, u8>,
    Uint<U1, [u64; 1]>,
    u7,
    Uint<U200, [u8; 25]>,
    Int<U2, i8>,
    Int<U2, [u16; 1]>,
    i256,
  );
  assert_eq!(bool::try_from(Uint::<U2, u8>::new(2)), Err(above));
  assert_eq!(bool::try_from(u256::MAX), Err(above));
  assert_eq!(bool::try_from(Int::<U1, i8>::MIN), Err(below));
  assert_eq!(bool::try_from(i256::MIN), Err(below));
  assert_eq!(bool::try_from(u0::ZERO), Ok(false));
  assert_eq!(bool::try_from(i0::ZERO), Ok(false));
  // As `bool` converts from the native integers.
  assert_eq!(bool::try_from(2u8), Err(above));
  assert_eq!(bool::try_from(-1i8), Err(below));
}
