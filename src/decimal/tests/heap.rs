//! Decimals over the heap storages, a wide integer whose limbs are in a `Box`, an `Rc`, an `Arc`
//! or a `triomphe::Arc`, against their twins, the decimals with the same digit counts over the
//! integer of the same width and sign over the array itself: the conversions between a heap
//! decimal and its twin and into other types, the exact product, the texts, the order and the
//! hash, and the layout and the auto traits. The oracle of each check is the twin.
//!
//! The checks of each pointer run for one type of each of six storages, of 192 to 1024 bits and of
//! both signs, each at the digit limit of its storage. Each type checks in a function of its own,
//! never inlined, whose frame holds the values of that type alone: an optimized build would inline
//! a function that has one call site, and the frame of the test would then hold the values of
//! every type. The tier of the build sizes every sweep, and every random value comes from a
//! generator with a seed written here, so that a failure replays.

use core::{
  any::TypeId,
  fmt::{Debug, Display, LowerExp, UpperExp},
  hash::BuildHasher,
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

#[cfg(all(feature = "triomphe", target_has_atomic = "ptr"))]
use triomphe::Arc as TriompheArc;
use typenum::{
  U0, U1, U2, U18, U20, U39, U40, U58, U78, U135, U155, U192, U256, U308, U320, U512, U576, U1024,
};

use crate::{
  ConstFixedDecimal, ConvertError, Decimal, DivideError, FixedDecimal, Int, Layout, RoundingMode,
  Storage, Uint,
  bigint::{MODES, floats::Words, tier},
};

/// The values at which a type is checked, as values of its twin `T`: zero, the limits, one, two
/// and 25 units of the last digit, negated too for a signed storage, and random coefficients of
/// random lengths and, for a signed storage, signs, 16 in the Full tier and 3 in the Debug tier;
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
    let count = if tier().is_full() { 16 } else { 3 };
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

/// Checks a type with [`check_values`], written `Integral, Fraction, Storage`.
macro_rules! values_agree {
  ($integral:ty, $fraction:ty, $repr:ty) => {
    check_values::<$integral, $fraction, $repr>()
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
}

/// `Rc<[u64; N]>`, which is neither `Send` nor `Sync`.
#[cfg(any(feature = "std", feature = "alloc"))]
mod rc {
  use super::*;

  heap_tests!(Rc);
}

/// `Arc<[u64; N]>`.
#[cfg(all(any(feature = "std", feature = "alloc"), target_has_atomic = "ptr"))]
mod arc {
  use super::*;

  heap_tests!(Arc);

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

  /// The decimals in a `triomphe::Arc` are `Send` and `Sync`, as the pointer is.
  #[test]
  fn decimals_are_send_and_sync() {
    send_and_sync::<Decimal<U39, U18, Uint<U192, TriompheArc<[u64; 3]>>>>();
    send_and_sync::<Decimal<U58, U18, Int<U256, TriompheArc<[u64; 4]>>>>();
  }
}
