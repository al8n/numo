use core::{
  fmt::Debug,
  hint::black_box,
  iter::{Product, Sum},
  ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};
use std::{
  boxed::Box,
  cell::RefCell,
  format,
  panic::{self, AssertUnwindSafe},
  string::{String, ToString},
  sync::Once,
  thread_local, vec,
  vec::Vec,
};

use num_bigint::BigInt;
use typenum::{
  IsGreater, NonZero, True, U0, U1, U2, U7, U8, U9, U24, U63, U64, U65, U100, U127, U128, U129,
  U200, U256, U1024, U4096,
};

use crate::{
  BitWidth, CheckedOps, FitsBits, FixedInteger, HasOne, Int, IntReprMut, OverflowingOps,
  SaturatingOps, SignedFixedInteger, StrictOps, Uint, UintReprMut, UnsignedFixedInteger,
  WrappingOps, bigint, i7, i256, u7, u256, u4096,
};

/// The smallest and the largest value of `bits` bits, unsigned or in two's complement.
fn range(bits: u32, signed: bool) -> (i128, i128) {
  match (bits, signed) {
    (0, _) => (0, 0),
    (_, false) => (0, (1 << bits) - 1),
    (_, true) => (-(1 << (bits - 1)), (1 << (bits - 1)) - 1),
  }
}

/// `exact` wrapped to the range of `bits` bits, and whether it is outside that range.
fn wrap(exact: i128, bits: u32, signed: bool) -> (i128, bool) {
  let (min, max) = range(bits, signed);
  let wrapped = (exact - min).rem_euclid(1 << bits) + min;
  (wrapped, exact < min || max < exact)
}

/// `value` in `N` limbs of type `L`, least significant first, in two's complement. Requires the
/// limbs to have at most 128 bits.
fn limbs_of<L: bigint::Limb, const N: usize>(value: i128) -> [L; N] {
  let bytes = value.to_le_bytes();
  let size = (L::BITS / 8) as usize;
  core::array::from_fn(|i| L::from_le_slice(&bytes[size * i..size * (i + 1)]))
}

/// The result of a method through the inherent `const fn` of the storage and through the trait,
/// which must agree.
macro_rules! both {
  ($trait:ident::$method:ident($receiver:expr $(, $arg:expr)*)) => {{
    let inherent = $receiver.$method($($arg),*);
    let generic = $trait::$method($receiver $(, $arg)*);
    assert_eq!(inherent, generic, "{} through the trait", stringify!($method));
    inherent
  }};
}

/// Checks the values of `$ty`, of `$bits` bits, created by `$from` from their value: the
/// overflowing, checked, wrapping, saturating and strict methods, through the inherent `const fn`s
/// and the traits, against the exact results wrapped to the width. Every value and every `$step`th
/// pair, or under Miri every 7th value and every 61st pair.
macro_rules! check_every_pair {
  ($ty:ty, $bits:expr, $signed:expr, $step:expr, $from:expr) => {{
    let from: fn(i128) -> $ty = $from;
    let (bits, signed) = ($bits, $signed);
    let (min, max) = range(bits, signed);
    let (value_step, pair_step) = if cfg!(miri) { (7, 61) } else { (1, $step) };
    for a in (min..=max).step_by(value_step) {
      let x = from(a);
      let (negated, overflow) = wrap(-a, bits, signed);
      assert_eq!(
        both!(OverflowingOps::overflowing_neg(&x)),
        (from(negated), overflow),
        "-{a} in {bits} bits"
      );
      assert_eq!(
        both!(CheckedOps::checked_neg(&x)),
        (!overflow).then(|| from(negated)),
        "-{a} in {bits} bits"
      );
      assert_eq!(
        both!(WrappingOps::wrapping_neg(&x)),
        from(negated),
        "-{a} in {bits} bits"
      );
      if !overflow {
        assert_eq!(
          both!(StrictOps::strict_neg(&x)),
          from(negated),
          "-{a} in {bits} bits"
        );
      }
    }
    let count = max - min + 1;
    for pair in (0..count * count).step_by(pair_step) {
      let (a, b) = (min + pair / count, min + pair % count);
      let (x, y) = (from(a), from(b));
      for (exact, (value, overflow), checked, wrapping, saturating) in [
        (
          a + b,
          both!(OverflowingOps::overflowing_add(&x, &y)),
          both!(CheckedOps::checked_add(&x, &y)),
          both!(WrappingOps::wrapping_add(&x, &y)),
          both!(SaturatingOps::saturating_add(&x, &y)),
        ),
        (
          a - b,
          both!(OverflowingOps::overflowing_sub(&x, &y)),
          both!(CheckedOps::checked_sub(&x, &y)),
          both!(WrappingOps::wrapping_sub(&x, &y)),
          both!(SaturatingOps::saturating_sub(&x, &y)),
        ),
        (
          a * b,
          both!(OverflowingOps::overflowing_mul(&x, &y)),
          both!(CheckedOps::checked_mul(&x, &y)),
          both!(WrappingOps::wrapping_mul(&x, &y)),
          both!(SaturatingOps::saturating_mul(&x, &y)),
        ),
      ] {
        let (wrapped, overflowed) = wrap(exact, bits, signed);
        assert_eq!(
          (value, overflow),
          (from(wrapped), overflowed),
          "{a} and {b} in {bits} bits"
        );
        assert_eq!(
          checked,
          (!overflowed).then(|| from(wrapped)),
          "{a} and {b} in {bits} bits"
        );
        assert_eq!(wrapping, from(wrapped), "{a} and {b} in {bits} bits");
        assert_eq!(
          saturating,
          from(exact.clamp(min, max)),
          "{a} and {b} in {bits} bits"
        );
      }
      if !wrap(a + b, bits, signed).1 {
        assert_eq!(
          both!(StrictOps::strict_add(&x, &y)),
          from(a + b),
          "{a} + {b}"
        );
      }
      if !wrap(a - b, bits, signed).1 {
        assert_eq!(
          both!(StrictOps::strict_sub(&x, &y)),
          from(a - b),
          "{a} - {b}"
        );
      }
      if !wrap(a * b, bits, signed).1 {
        assert_eq!(
          both!(StrictOps::strict_mul(&x, &y)),
          from(a * b),
          "{a} * {b}"
        );
      }
    }
  }};
}

/// Checks the values of `$ty`, unsigned, of `$bits` bits, created by `$from` from their value:
/// `is_one`, and the carrying addition and the borrowing subtraction with and without a carry,
/// through the inherent `const fn`s and the traits, against the exact results wrapped to the
/// width. Every value and every `$step`th pair, or under Miri every 7th value and every 61st pair.
macro_rules! check_unsigned_methods {
  ($ty:ty, $bits:expr, $step:expr, $from:expr) => {{
    let from: fn(i128) -> $ty = $from;
    let bits = $bits;
    let (min, max) = range(bits, false);
    let (value_step, pair_step) = if cfg!(miri) { (7, 61) } else { (1, $step) };
    for a in (min..=max).step_by(value_step) {
      assert_eq!(both!(FixedInteger::is_one(&from(a))), a == 1, "{a}");
    }
    let count = max - min + 1;
    for pair in (0..count * count).step_by(pair_step) {
      let (a, b) = (min + pair / count, min + pair % count);
      let (x, y) = (from(a), from(b));
      for carry in [false, true] {
        let c = i128::from(carry);
        let (sum, overflow) = wrap(a + b + c, bits, false);
        assert_eq!(
          both!(UnsignedFixedInteger::carrying_add(&x, &y, carry)),
          (from(sum), overflow),
          "{a} + {b} + {c} in {bits} bits"
        );
        let (difference, overflow) = wrap(a - b - c, bits, false);
        assert_eq!(
          both!(UnsignedFixedInteger::borrowing_sub(&x, &y, carry)),
          (from(difference), overflow),
          "{a} - {b} - {c} in {bits} bits"
        );
      }
    }
  }};
}

/// Checks every value of `$ty`, signed, of `$bits` bits, created by `$from` from its value, or
/// under Miri every 7th value: `is_one`, the absolute value in every form, the saturating
/// negation, `signum` and `unsigned_abs`, whose result `$unsigned` creates, through the inherent
/// `const fn`s and the traits, against the exact results wrapped to the width.
macro_rules! check_signed_methods {
  ($ty:ty, $bits:expr, $from:expr, $unsigned:expr) => {{
    let from: fn(i128) -> $ty = $from;
    let bits = $bits;
    let (min, max) = range(bits, true);
    let step = if cfg!(miri) { 7 } else { 1 };
    for a in (min..=max).step_by(step) {
      let x = from(a);
      assert_eq!(both!(FixedInteger::is_one(&x)), a == 1, "{a}");
      let (abs, overflow) = wrap(a.abs(), bits, true);
      assert_eq!(
        both!(SignedFixedInteger::overflowing_abs(&x)),
        (from(abs), overflow),
        "|{a}| in {bits} bits"
      );
      assert_eq!(
        both!(SignedFixedInteger::checked_abs(&x)),
        (!overflow).then(|| from(abs)),
        "|{a}| in {bits} bits"
      );
      assert_eq!(
        both!(SignedFixedInteger::wrapping_abs(&x)),
        from(abs),
        "|{a}| in {bits} bits"
      );
      assert_eq!(
        both!(SignedFixedInteger::saturating_abs(&x)),
        from(a.abs().min(max)),
        "|{a}| in {bits} bits"
      );
      if !overflow {
        assert_eq!(
          both!(SignedFixedInteger::strict_abs(&x)),
          from(abs),
          "|{a}|"
        );
        assert_eq!(both!(SignedFixedInteger::abs(&x)), from(abs), "|{a}|");
      }
      assert_eq!(
        both!(SignedFixedInteger::saturating_neg(&x)),
        from((-a).clamp(min, max)),
        "-{a} in {bits} bits"
      );
      assert_eq!(
        both!(SignedFixedInteger::signum(&x)),
        from(a.signum()),
        "{a}"
      );
      assert_eq!(
        both!(SignedFixedInteger::unsigned_abs(&x)),
        $unsigned(a.abs()),
        "{a}"
      );
    }
  }};
}

/// Checks the values of `$ty`, of `$bits` bits, created by `$from` from their value: where the
/// sum, the difference or the product is in range, the operator in its four forms of values and
/// references, its compound assignment with a value and with a reference, and for the sum `Sum`
/// over values and over references give the result of the checked method. Every `$step`th pair, or
/// under Miri every 61st pair.
macro_rules! check_operators {
  ($ty:ty, $bits:expr, $signed:expr, $step:expr, $from:expr) => {{
    let from: fn(i128) -> $ty = $from;
    let (min, max) = range($bits, $signed);
    let step = if cfg!(miri) { 61 } else { $step };
    let count = max - min + 1;
    for pair in (0..count * count).step_by(step) {
      let (x, y) = (from(min + pair / count), from(min + pair % count));
      check_operators!(@op x, y, +, +=, x.checked_add(&y));
      check_operators!(@op x, y, -, -=, x.checked_sub(&y));
      check_operators!(@op x, y, *, *=, x.checked_mul(&y));
      if let Some(sum) = x.checked_add(&y) {
        assert_eq!([x, y].into_iter().sum::<$ty>(), sum);
        assert_eq!([x, y].iter().sum::<$ty>(), sum);
      }
    }
  }};
  (@op $x:ident, $y:ident, $op:tt, $assign:tt, $checked:expr) => {
    if let Some(expected) = $checked {
      assert_eq!($x $op $y, expected, "{} {} {}", $x, stringify!($op), $y);
      assert_eq!($x $op &$y, expected);
      assert_eq!(&$x $op $y, expected);
      assert_eq!(&$x $op &$y, expected);
      let mut target = $x;
      target $assign $y;
      assert_eq!(target, expected);
      let mut target = $x;
      target $assign &$y;
      assert_eq!(target, expected);
    }
  };
}

/// Checks `Uint` and `Int` of `$bits` bits, the width `$width`, in one storage of each kind, with
/// every check above, at every value and every `$step`th pair: `native` checks `$uint` and `$int`,
/// and `array` checks `[$limb; $n]`.
macro_rules! check_storage {
  (native $width:ty, $bits:expr, $uint:ident, $int:ident, $step:expr) => {{
    let unsigned: fn(i128) -> Uint<$width, $uint> = |v| Uint::<$width, $uint>::new(v as $uint);
    let signed: fn(i128) -> Int<$width, $int> = |v| Int::<$width, $int>::new(v as $int);
    check_storage!(@check Uint<$width, $uint>, Int<$width, $int>, $bits, $step, unsigned, signed);
  }};
  (array $width:ty, $bits:expr, [$limb:ident; $n:literal], $step:expr) => {{
    let unsigned: fn(i128) -> Uint<$width, [$limb; $n]> =
      |v| Uint::<$width, [$limb; $n]>::checked_from_limbs(limbs_of(v)).expect("in range");
    let signed: fn(i128) -> Int<$width, [$limb; $n]> =
      |v| Int::<$width, [$limb; $n]>::checked_from_limbs(limbs_of(v)).expect("in range");
    check_storage!(
      @check Uint<$width, [$limb; $n]>, Int<$width, [$limb; $n]>, $bits, $step, unsigned, signed
    );
  }};
  (@check $uint:ty, $int:ty, $bits:expr, $step:expr, $unsigned:ident, $signed:ident) => {{
    check_every_pair!($uint, $bits, false, $step, $unsigned);
    check_every_pair!($int, $bits, true, $step, $signed);
    check_unsigned_methods!($uint, $bits, $step, $unsigned);
    check_signed_methods!($int, $bits, $signed, $unsigned);
    check_operators!($uint, $bits, false, $step, $unsigned);
    check_operators!($int, $bits, true, $step, $signed);
  }};
}

/// Every value and every pair of 7 and of 8 bits in `u8` and `i8` and in an array of one byte, and
/// every 7th pair in wider native integers and in the other arrays, through the inherent
/// `const fn`s and the traits, against the exact results wrapped to the width. A result equal to
/// the value that `new` or `checked_from_limbs` creates holds a value of the width.
#[test]
#[allow(clippy::op_ref, reason = "the operators on references are under test")]
fn every_pair_of_7_and_8_bits() {
  check_storage!(native U7, 7, u8, i8, 1);
  check_storage!(native U7, 7, u16, i16, 7);
  check_storage!(native U7, 7, u128, i128, 7);
  check_storage!(array U7, 7, [u8; 1], 1);
  check_storage!(array U7, 7, [u16; 1], 7);
  check_storage!(array U7, 7, [u32; 1], 7);
  check_storage!(array U7, 7, [u64; 1], 7);
  check_storage!(array U7, 7, [u128; 1], 7);
  check_storage!(array U7, 7, [u8; 2], 7);
  check_storage!(native U8, 8, u8, i8, 1);
  check_storage!(native U8, 8, u16, i16, 7);
  check_storage!(native U8, 8, u128, i128, 7);
  check_storage!(array U8, 8, [u8; 1], 1);
  check_storage!(array U8, 8, [u16; 1], 7);
  check_storage!(array U8, 8, [u32; 1], 7);
  check_storage!(array U8, 8, [u64; 1], 7);
  check_storage!(array U8, 8, [u128; 1], 7);
  check_storage!(array U8, 8, [u8; 2], 7);
}

/// Every value and every pair of one bit, which `Int` holds only -1 and 0 in, and of zero bits,
/// which hold only zero, in native integers and in arrays of no limbs, one limb and two limbs.
#[test]
#[allow(clippy::op_ref, reason = "the operators on references are under test")]
fn every_pair_of_one_and_zero_bits() {
  check_storage!(native U1, 1, u8, i8, 1);
  check_storage!(native U1, 1, u64, i64, 1);
  check_storage!(array U1, 1, [u8; 1], 1);
  check_storage!(array U1, 1, [u128; 1], 1);
  check_storage!(array U1, 1, [u16; 2], 1);
  check_storage!(native U0, 0, u8, i8, 1);
  check_storage!(native U0, 0, u128, i128, 1);
  check_storage!(array U0, 0, [u8; 0], 1);
  check_storage!(array U0, 0, [u64; 0], 1);
  check_storage!(array U0, 0, [u8; 1], 1);
  check_storage!(array U0, 0, [u32; 1], 1);
  check_storage!(array U0, 0, [u64; 2], 1);
}

/// `ONE` is the storage's one, and the constant of `HasOne` is the inherent one, from one bit for
/// `Uint` and from two bits for `Int`. Zero bits, which have no `ONE`, overflow on a carry and on a
/// borrow, and negate `MIN`, which is zero, to itself.
#[test]
fn one_and_zero_bits() {
  assert_eq!(Uint::<U1, u8>::ONE.value(), 1);
  assert_eq!(Uint::<U7, u128>::ONE.value(), 1);
  assert_eq!(Uint::<U1, [u64; 1]>::ONE.to_limbs(), [1]);
  assert_eq!(Uint::<U7, [u8; 2]>::ONE.to_limbs(), [1, 0]);
  assert_eq!(Int::<U2, i8>::ONE.value(), 1);
  assert_eq!(Int::<U7, i128>::ONE.value(), 1);
  assert_eq!(Int::<U2, [u16; 1]>::ONE.to_limbs(), [1]);
  assert_eq!(Int::<U8, [u8; 3]>::ONE.to_limbs(), [1, 0, 0]);
  assert_eq!(<Uint<U1, u8> as HasOne>::ONE, Uint::<U1, u8>::ONE);
  assert_eq!(<Uint<U7, [u8; 2]> as HasOne>::ONE, Uint::<U7, [u8; 2]>::ONE);
  assert_eq!(<Int<U2, i8> as HasOne>::ONE, Int::<U2, i8>::ONE);
  assert_eq!(<Int<U8, [u8; 3]> as HasOne>::ONE, Int::<U8, [u8; 3]>::ONE);
  assert!(Uint::<U7, u8>::ONE.is_one() && Int::<U7, [u8; 1]>::ONE.is_one());

  type U = Uint<U0, u8>;
  type I = Int<U0, [u64; 0]>;
  assert_eq!(U::ZERO.carrying_add(&U::ZERO, true), (U::ZERO, true));
  assert_eq!(U::ZERO.borrowing_sub(&U::ZERO, true), (U::ZERO, true));
  assert_eq!(U::ZERO.carrying_add(&U::ZERO, false), (U::ZERO, false));
  assert_eq!(I::MIN.checked_neg(), Some(I::ZERO));
  assert_eq!(I::MIN.overflowing_abs(), (I::ZERO, false));
  assert!(!U::ZERO.is_one() && !I::ZERO.is_one());
  // One bit holds no positive value, so `signum` is never one.
  assert_eq!(Int::<U1, i8>::MIN.signum(), Int::<U1, i8>::MIN);
}

/// Converts the values of an integer to and from big integers.
trait Big: Copy + Debug + PartialEq {
  /// Whether the integer is signed.
  const SIGNED: bool;

  /// The integer of `value`, which it holds.
  fn from_big(value: &BigInt) -> Self;

  /// The value of the integer.
  fn to_big(self) -> BigInt;
}

/// Implements [`Big`] for `Uint` and `Int` over the native integers, written `Integer<native>`,
/// and over the arrays of each limb type, written `limb`.
macro_rules! big {
  (native $($integer:ident<$native:ident>: $signed:literal),+ $(,)?) => {$(
    impl<B: FitsBits<$native>> Big for $integer<B, $native> {
      const SIGNED: bool = $signed;

      fn from_big(value: &BigInt) -> Self {
        Self::new($native::try_from(value).expect("in range"))
      }

      fn to_big(self) -> BigInt {
        BigInt::from(self.value())
      }
    }
  )+};
  (array $($limb:ident),+ $(,)?) => {$(
    impl<B: FitsBits<[$limb; N]>, const N: usize> Big for Uint<B, [$limb; N]> {
      const SIGNED: bool = false;

      fn from_big(value: &BigInt) -> Self {
        Self::checked_from_limbs(bigint::to_limbs_of(value, false).expect("in range"))
          .expect("in range")
      }

      fn to_big(self) -> BigInt {
        bigint::from_limbs(self.as_limbs(), false)
      }
    }

    impl<B: FitsBits<[$limb; N]>, const N: usize> Big for Int<B, [$limb; N]> {
      const SIGNED: bool = true;

      fn from_big(value: &BigInt) -> Self {
        Self::checked_from_limbs(bigint::to_limbs_of(value, true).expect("in range"))
          .expect("in range")
      }

      fn to_big(self) -> BigInt {
        bigint::from_limbs(self.as_limbs(), true)
      }
    }
  )+};
}

big!(
  native Uint<u8>: false,
  Uint<u16>: false,
  Uint<u32>: false,
  Uint<u64>: false,
  Uint<u128>: false,
  Int<i8>: true,
  Int<i16>: true,
  Int<i32>: true,
  Int<i64>: true,
  Int<i128>: true,
);
big!(array u8, u16, u32, u64, u128);

/// The values of `bits` bits at which the arithmetic is checked against `num-bigint`: the limits,
/// the values next to them and to zero, and `count` values from a generator seeded by the width,
/// or under Miri, which runs big-integer arithmetic slowly, one value from it.
fn big_samples(bits: u32, signed: bool, count: usize) -> Vec<BigInt> {
  let count = if cfg!(miri) { 1 } else { count };
  let (min, max) = (bigint::min(bits, signed), bigint::max(bits, signed));
  let mut values = vec![
    min.clone(),
    &min + 1u8,
    BigInt::from(-1),
    BigInt::ZERO,
    BigInt::from(1u8),
    &max - 1u8,
    max.clone(),
  ];
  // A xorshift generator, so that a failure replays from the width.
  let mut state = 0x5eed_b16e_ab17_u64 ^ u64::from(bits) ^ u64::from(signed);
  let mut word = move || {
    state ^= state >> 12;
    state ^= state << 25;
    state ^= state >> 27;
    state.wrapping_mul(0x2545_f491_4f6c_dd1d)
  };
  for _ in 0..count {
    let words: Vec<u64> = (0..bits.div_ceil(64)).map(|_| word()).collect();
    let random = bigint::from_limbs(&words, false) >> (64 * words.len() as u32 - bits);
    values.push(bigint::wrap(&random, bits, &min));
  }
  values.retain(|value| min <= *value && *value <= max);
  values.sort_unstable();
  values.dedup();
  values
}

/// Checks the overflowing, checked, wrapping, saturating and strict methods of `T`, of `bits` bits,
/// through the traits against `num-bigint`, at every pair of `samples`.
fn check_families_against_big_integers<T>(bits: u32, samples: &[BigInt])
where
  T: Big + OverflowingOps + CheckedOps + WrappingOps + SaturatingOps + StrictOps,
{
  let (min, max) = (bigint::min(bits, T::SIGNED), bigint::max(bits, T::SIGNED));
  let expected = |exact: &BigInt| {
    (
      bigint::wrap(exact, bits, &min),
      *exact < min || max < *exact,
    )
  };
  let values: Vec<T> = samples.iter().map(T::from_big).collect();
  for (a, &x) in samples.iter().zip(&values) {
    let (negated, overflow) = expected(&-a);
    let (value, flag) = x.overflowing_neg();
    assert_eq!((value.to_big(), flag), (negated.clone(), overflow), "-{a}");
    assert_eq!(
      x.checked_neg().map(T::to_big),
      (!overflow).then(|| negated.clone())
    );
    assert_eq!(x.wrapping_neg().to_big(), negated);
    if !overflow {
      assert_eq!(x.strict_neg().to_big(), negated);
    }
    for (b, &y) in samples.iter().zip(&values) {
      for (exact, op, (value, flag), checked, wrapping, saturating) in [
        (
          a + b,
          '+',
          x.overflowing_add(&y),
          x.checked_add(&y),
          x.wrapping_add(&y),
          x.saturating_add(&y),
        ),
        (
          a - b,
          '-',
          x.overflowing_sub(&y),
          x.checked_sub(&y),
          x.wrapping_sub(&y),
          x.saturating_sub(&y),
        ),
        (
          a * b,
          '*',
          x.overflowing_mul(&y),
          x.checked_mul(&y),
          x.wrapping_mul(&y),
          x.saturating_mul(&y),
        ),
      ] {
        let (wrapped, overflow) = expected(&exact);
        assert_eq!(
          (value.to_big(), flag),
          (wrapped.clone(), overflow),
          "{a} {op} {b} in {bits} bits"
        );
        assert_eq!(checked.map(T::to_big), (!overflow).then(|| wrapped.clone()));
        assert_eq!(wrapping.to_big(), wrapped);
        assert_eq!(saturating.to_big(), exact.clamp(min.clone(), max.clone()));
      }
      if let Some(sum) = x.checked_add(&y) {
        assert_eq!(x.strict_add(&y), sum);
      }
      if let Some(difference) = x.checked_sub(&y) {
        assert_eq!(x.strict_sub(&y), difference);
      }
      if let Some(product) = x.checked_mul(&y) {
        assert_eq!(x.strict_mul(&y), product);
      }
    }
  }
}

/// Checks the carrying addition and the borrowing subtraction of `T`, unsigned, of `bits` bits,
/// through the trait against `num-bigint`, at every pair of `samples` with each carry.
fn check_unsigned_against_big_integers<T>(bits: u32, samples: &[BigInt])
where
  T: Big + UnsignedFixedInteger,
{
  let max = bigint::max(bits, false);
  let values: Vec<T> = samples.iter().map(T::from_big).collect();
  for (a, &x) in samples.iter().zip(&values) {
    for (b, &y) in samples.iter().zip(&values) {
      for carry in [false, true] {
        let (sum, overflow) = x.carrying_add(&y, carry);
        let exact = a + b + u8::from(carry);
        let wrapped = bigint::wrap(&exact, bits, &BigInt::ZERO);
        assert_eq!(
          (sum.to_big(), overflow),
          (wrapped, exact > max),
          "{a} + {b} + {carry}"
        );
        let (difference, overflow) = x.borrowing_sub(&y, carry);
        let exact = a - b - u8::from(carry);
        let wrapped = bigint::wrap(&exact, bits, &BigInt::ZERO);
        let borrowed = exact < BigInt::ZERO;
        assert_eq!(
          (difference.to_big(), overflow),
          (wrapped, borrowed),
          "{a} - {b} - {carry}"
        );
      }
    }
  }
}

/// Checks the absolute value, the saturating negation, `signum` and `unsigned_abs` of `T`, signed,
/// of `bits` bits, through the trait against `num-bigint`, at every value of `samples`.
fn check_signed_against_big_integers<T>(bits: u32, samples: &[BigInt])
where
  T: Big + SignedFixedInteger,
  T::Unsigned: Big,
{
  let (min, max) = (bigint::min(bits, true), bigint::max(bits, true));
  let zero = BigInt::ZERO;
  for a in samples {
    let x = T::from_big(a);
    let magnitude = if *a < zero { -a } else { a.clone() };
    let (abs, overflow) = x.overflowing_abs();
    let wrapped = bigint::wrap(&magnitude, bits, &min);
    assert_eq!(
      (abs.to_big(), overflow),
      (wrapped, magnitude > max),
      "|{a}|"
    );
    assert_eq!(x.saturating_abs().to_big(), (&magnitude).min(&max).clone());
    assert_eq!(
      x.saturating_neg().to_big(),
      (-a).clamp(min.clone(), max.clone())
    );
    let sign = if *a < zero { -1 } else { i8::from(*a > zero) };
    assert_eq!(x.signum().to_big(), BigInt::from(sign), "{a}");
    assert_eq!(x.unsigned_abs().to_big(), magnitude, "{a}");
  }
}

/// Checks that the inherent `const fn`s of `$ty` agree with the traits at every pair of
/// `$samples`.
macro_rules! check_paths {
  ($ty:ty, $samples:expr) => {{
    let values: Vec<$ty> = $samples.iter().map(<$ty>::from_big).collect();
    for &x in &values {
      both!(OverflowingOps::overflowing_neg(&x));
      for &y in &values {
        both!(OverflowingOps::overflowing_add(&x, &y));
        both!(OverflowingOps::overflowing_sub(&x, &y));
        both!(OverflowingOps::overflowing_mul(&x, &y));
        both!(SaturatingOps::saturating_add(&x, &y));
        both!(SaturatingOps::saturating_sub(&x, &y));
        both!(SaturatingOps::saturating_mul(&x, &y));
      }
    }
  }};
}

/// Checks the arithmetic of `Uint` and `Int` of `$bits` bits, the width `$width`, in each pair of
/// storages written `unsigned, signed;`, against `num-bigint`, with `$count` random samples.
macro_rules! agree {
  ($width:ty, $bits:literal, $count:literal; $($uint:ty, $int:ty);+ $(;)?) => {{
    let unsigned = big_samples($bits, false, $count);
    let signed = big_samples($bits, true, $count);
    $(
      check_families_against_big_integers::<Uint<$width, $uint>>($bits, &unsigned);
      check_unsigned_against_big_integers::<Uint<$width, $uint>>($bits, &unsigned);
      check_paths!(Uint<$width, $uint>, unsigned);
      check_families_against_big_integers::<Int<$width, $int>>($bits, &signed);
      check_signed_against_big_integers::<Int<$width, $int>>($bits, &signed);
      check_paths!(Int<$width, $int>, signed);
    )+
  }};
}

/// The same edge and random values of 1 to 4096 bits in every storage that holds them, native
/// integers up to 128 bits and arrays of each limb type: every method agrees with `num-bigint`,
/// through the traits, and the inherent `const fn`s agree with the traits. Under Miri, the widths
/// above 256 bits are left out.
#[test]
#[cfg_attr(
  all(miri, target_pointer_width = "32"),
  ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
)]
fn storages_agree_on_the_arithmetic() {
  agree!(U1, 1, 10; u8, i8; u16, i16; u32, i32; u64, i64; u128, i128);
  agree!(U1, 1, 10; [u8; 1], [u8; 1]; [u16; 1], [u16; 1]; [u64; 1], [u64; 1]; [u128; 1], [u128; 1]);
  agree!(U7, 7, 10; u8, i8; u16, i16; u32, i32; u64, i64; u128, i128);
  agree!(U7, 7, 10; [u8; 1], [u8; 1]; [u16; 1], [u16; 1]; [u32; 1], [u32; 1]);
  agree!(U7, 7, 10; [u64; 1], [u64; 1]; [u128; 1], [u128; 1]);
  agree!(U8, 8, 10; u8, i8; u16, i16; u32, i32; u64, i64; u128, i128);
  agree!(U8, 8, 10; [u8; 1], [u8; 1]; [u32; 1], [u32; 1]; [u128; 1], [u128; 1]);
  agree!(U9, 9, 10; u16, i16; u32, i32; u64, i64; u128, i128);
  agree!(U9, 9, 10; [u8; 2], [u8; 2]; [u16; 1], [u16; 1]; [u64; 1], [u64; 1]; [u128; 1], [u128; 1]);
  agree!(U63, 63, 10; u64, i64; u128, i128; [u8; 8], [u8; 8]; [u16; 4], [u16; 4]);
  agree!(U63, 63, 10; [u32; 2], [u32; 2]; [u64; 1], [u64; 1]; [u128; 1], [u128; 1]);
  agree!(U64, 64, 10; u64, i64; u128, i128; [u8; 8], [u8; 8]; [u16; 4], [u16; 4]);
  agree!(U64, 64, 10; [u32; 2], [u32; 2]; [u64; 1], [u64; 1]; [u128; 1], [u128; 1]);
  agree!(U65, 65, 10; u128, i128; [u8; 9], [u8; 9]; [u16; 5], [u16; 5]; [u32; 3], [u32; 3]);
  agree!(U65, 65, 10; [u64; 2], [u64; 2]; [u128; 1], [u128; 1]);
  agree!(U100, 100, 10; u128, i128; [u8; 13], [u8; 13]; [u16; 7], [u16; 7]; [u32; 4], [u32; 4]);
  agree!(U100, 100, 10; [u64; 2], [u64; 2]; [u128; 1], [u128; 1]);
  agree!(U127, 127, 10; u128, i128; [u8; 16], [u8; 16]; [u32; 4], [u32; 4]);
  agree!(U127, 127, 10; [u64; 2], [u64; 2]; [u128; 1], [u128; 1]);
  agree!(U128, 128, 10; u128, i128; [u8; 16], [u8; 16]; [u16; 8], [u16; 8]; [u32; 4], [u32; 4]);
  agree!(U128, 128, 10; [u64; 2], [u64; 2]; [u128; 1], [u128; 1]);
  agree!(U129, 129, 10; [u8; 17], [u8; 17]; [u16; 9], [u16; 9]; [u32; 5], [u32; 5]);
  agree!(U129, 129, 10; [u64; 3], [u64; 3]; [u128; 2], [u128; 2]);
  agree!(U200, 200, 10; [u8; 25], [u8; 25]; [u16; 13], [u16; 13]; [u32; 7], [u32; 7]);
  agree!(U200, 200, 10; [u64; 4], [u64; 4]; [u128; 2], [u128; 2]);
  agree!(U256, 256, 10; [u8; 32], [u8; 32]; [u16; 16], [u16; 16]; [u32; 8], [u32; 8]);
  agree!(U256, 256, 10; [u64; 4], [u64; 4]; [u128; 2], [u128; 2]);
  if !cfg!(miri) {
    agree!(U4096, 4096, 2; [u8; 512], [u8; 512]; [u16; 256], [u16; 256]; [u32; 128], [u32; 128]);
    agree!(U4096, 4096, 2; [u64; 64], [u64; 64]; [u128; 32], [u128; 32]);
  }
}

/// A panic: its message, and the file and the line that it reports.
type Panic = (String, String, u32);

thread_local! {
  /// The panic of this thread while `panic_of` captures it: `None` outside `panic_of`, and
  /// `Some(None)` inside it until a panic.
  static CAPTURED: RefCell<Option<Option<Panic>>> = const { RefCell::new(None) };
}

/// Runs `f` and returns its panic, or `None` if it returns. The panic hook records the panic of
/// this thread meanwhile instead of printing it, and passes the panics of other threads and of
/// other code to the hook that it replaced.
fn panic_of<T>(f: impl FnOnce() -> T) -> Option<Panic> {
  static HOOK: Once = Once::new();
  HOOK.call_once(|| {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
      let recorded = CAPTURED.with(|captured| match captured.borrow_mut().as_mut() {
        Some(slot) => {
          let message = info.payload_as_str().unwrap_or_default().to_string();
          let (file, line) = info
            .location()
            .map_or((String::new(), 0), |at| (at.file().to_string(), at.line()));
          *slot = Some((message, file, line));
          true
        }
        None => false,
      });
      if !recorded {
        previous(info);
      }
    }));
  });
  CAPTURED.with(|captured| *captured.borrow_mut() = Some(None));
  let result = panic::catch_unwind(AssertUnwindSafe(f));
  let panic = CAPTURED
    .with(|captured| captured.borrow_mut().take())
    .flatten();
  assert_eq!(result.is_err(), panic.is_some(), "a panic and its record");
  panic
}

/// Whether this build checks arithmetic for overflow, as the native operators and the operators of
/// this crate then do.
fn overflow_checks() -> bool {
  panic_of(|| black_box(u8::MAX) + 1).is_some()
}

/// With overflow checks, as in a debug build, an operator that overflows panics with the message
/// of the native integers, and without them, as in a release build, it returns the wrapped value,
/// as the native operators do: in partial and full widths of native integers and arrays, through
/// `abs`, `Sum` and `Product`. A panicking compound assignment leaves its target unchanged.
#[test]
fn operators_follow_overflow_checks() {
  let checks = overflow_checks();
  // `$value` overflows with `$message`, or gives `$wrapped`.
  macro_rules! overflows {
    ($value:expr, $message:literal, $wrapped:expr) => {
      match panic_of(|| $value) {
        Some((message, _, _)) => {
          assert!(
            checks,
            "{} panics without overflow checks",
            stringify!($value)
          );
          assert_eq!(message, $message, "{}", stringify!($value));
        }
        None => {
          assert!(
            !checks,
            "{} returns with overflow checks",
            stringify!($value)
          );
          assert_eq!($value, $wrapped, "{}", stringify!($value));
        }
      }
    };
  }

  type Byte = Uint<U8, u8>;
  type U200x4 = Uint<U200, [u64; 4]>;
  overflows!(u7::MAX + u7::ONE, "attempt to add with overflow", u7::ZERO);
  overflows!(
    Byte::MAX + Byte::ONE,
    "attempt to add with overflow",
    Byte::ZERO
  );
  overflows!(
    U200x4::MAX + U200x4::ONE,
    "attempt to add with overflow",
    U200x4::ZERO
  );
  overflows!(
    u256::MAX + u256::ONE,
    "attempt to add with overflow",
    u256::ZERO
  );
  overflows!(
    u7::ZERO - u7::ONE,
    "attempt to subtract with overflow",
    u7::MAX
  );
  overflows!(
    u256::ZERO - u256::ONE,
    "attempt to subtract with overflow",
    u256::MAX
  );
  overflows!(
    u7::MAX * u7::new(2),
    "attempt to multiply with overflow",
    u7::new(126)
  );
  overflows!(
    U200x4::MAX * U200x4::MAX,
    "attempt to multiply with overflow",
    U200x4::ONE
  );
  overflows!(i7::MAX + i7::ONE, "attempt to add with overflow", i7::MIN);
  overflows!(
    i7::MIN - i7::ONE,
    "attempt to subtract with overflow",
    i7::MAX
  );
  overflows!(
    i7::MIN * -i7::ONE,
    "attempt to multiply with overflow",
    i7::MIN
  );
  overflows!(-i7::MIN, "attempt to negate with overflow", i7::MIN);
  overflows!(-i256::MIN, "attempt to negate with overflow", i256::MIN);
  overflows!(i7::MIN.abs(), "attempt to negate with overflow", i7::MIN);
  overflows!(
    i256::MIN.abs(),
    "attempt to negate with overflow",
    i256::MIN
  );
  overflows!(
    [u7::MAX, u7::ONE].into_iter().sum::<u7>(),
    "attempt to add with overflow",
    u7::ZERO
  );
  overflows!(
    [i7::MIN, i7::new(-1)].iter().sum::<i7>(),
    "attempt to add with overflow",
    i7::MAX
  );
  overflows!(
    [u7::MAX, u7::new(2)].iter().product::<u7>(),
    "attempt to multiply with overflow",
    u7::new(126)
  );
  overflows!(
    [i7::MIN, -i7::ONE].into_iter().product::<i7>(),
    "attempt to multiply with overflow",
    i7::MIN
  );

  let mut target = u7::MAX;
  let panic = panic_of(|| target += u7::ONE);
  assert_eq!(panic.is_some(), checks);
  assert_eq!(target, if checks { u7::MAX } else { u7::ZERO });
  let mut target = i256::MIN;
  let panic = panic_of(|| target -= &i256::ONE);
  assert_eq!(panic.is_some(), checks);
  assert_eq!(target, if checks { i256::MIN } else { i256::MAX });
  let mut target = Int::<U24, [u8; 3]>::MIN;
  let panic = panic_of(|| target *= -Int::<U24, [u8; 3]>::ONE);
  assert_eq!(panic.is_some(), checks);
  assert_eq!(target, Int::<U24, [u8; 3]>::MIN);
}

/// The strict methods panic on overflow in every build, with the messages of the native integers,
/// through the inherent `const fn`s and the traits.
#[test]
fn strict_methods_always_panic() {
  let message = |panic: Option<Panic>| panic.map(|(message, _, _)| message);
  let add = Some("attempt to add with overflow".to_string());
  let sub = Some("attempt to subtract with overflow".to_string());
  let mul = Some("attempt to multiply with overflow".to_string());
  let neg = Some("attempt to negate with overflow".to_string());

  assert_eq!(message(panic_of(|| u7::MAX.strict_add(&u7::ONE))), add);
  assert_eq!(message(panic_of(|| u256::ZERO.strict_sub(&u256::ONE))), sub);
  assert_eq!(message(panic_of(|| u7::MAX.strict_mul(&u7::new(2)))), mul);
  assert_eq!(message(panic_of(|| u7::ONE.strict_neg())), neg);
  assert_eq!(message(panic_of(|| i7::MAX.strict_add(&i7::ONE))), add);
  assert_eq!(message(panic_of(|| i256::MIN.strict_sub(&i256::ONE))), sub);
  assert_eq!(message(panic_of(|| i7::MIN.strict_mul(&-i7::ONE))), mul);
  assert_eq!(message(panic_of(|| i7::MIN.strict_neg())), neg);
  assert_eq!(message(panic_of(|| i7::MIN.strict_abs())), neg);
  assert_eq!(message(panic_of(|| i256::MIN.strict_abs())), neg);
  let max = Uint::<U200, [u64; 4]>::MAX;
  assert_eq!(message(panic_of(|| StrictOps::strict_add(&max, &max))), add);
  assert_eq!(message(panic_of(|| StrictOps::strict_mul(&max, &max))), mul);
  assert_eq!(message(panic_of(|| StrictOps::strict_neg(&max))), neg);
  assert_eq!(
    message(panic_of(|| SignedFixedInteger::strict_abs(&i7::MIN))),
    neg
  );
  // Zero bits negate their only value, zero, to itself.
  assert_eq!(Int::<U0, i8>::MIN.strict_neg(), Int::<U0, i8>::ZERO);
}

/// A panic of an operator or of a strict method reports the expression that overflowed, through
/// `#[track_caller]`, as the native operators do. `Sum` and `Product` report the addition and the
/// multiplication in this crate that they fold with, as those of the native integers do.
#[test]
#[allow(clippy::op_ref, reason = "the operators on references are under test")]
fn overflow_panics_report_the_caller() {
  let checks = overflow_checks();
  // The file and the line that the panic of `$value` reports, and the line of the invocation.
  macro_rules! location {
    ($value:expr) => {{
      let panic = panic_of(|| $value);
      (panic.map(|(_, file, line)| (file, line)), line!())
    }};
  }
  let here = |line| Some((file!().to_string(), line));

  let (strict, line) = location!(u7::MAX.strict_add(&u7::ONE));
  assert_eq!(strict, here(line));
  let (strict, line) = location!(StrictOps::strict_add(&u256::MAX, &u256::ONE));
  assert_eq!(strict, here(line));
  let (strict, line) = location!(i7::MIN.strict_abs());
  assert_eq!(strict, here(line));

  let mut target = u7::MAX;
  let locations = [
    location!(u7::MAX + u7::ONE),
    location!(&u7::MAX + &u7::ONE),
    location!(u256::ZERO - u256::ONE),
    location!(i7::MIN * -i7::ONE),
    location!(-i256::MIN),
    location!(i7::MIN.abs()),
    location!(target += u7::ONE),
  ];
  for (location, line) in locations {
    assert_eq!(location, if checks { here(line) } else { None });
  }
  let (sum, _) = location!([u7::MAX, u7::ONE].into_iter().sum::<u7>());
  let (product, _) = location!([u7::MAX, u7::MAX].iter().product::<u7>());
  for location in [sum, product] {
    let file = location.map(|(file, _)| file);
    let expected = checks.then(|| "src/integer/arithmetic/mod.rs".to_string());
    assert_eq!(file, expected);
  }
}

/// `-` on values and references, and `Product` over values and references, give the results of
/// `checked_neg` and `checked_mul` where these are `Some`, for every value and pair of 7 bits in a
/// native integer and in an array of one byte, or under Miri every 7th.
#[test]
fn negation_and_products_match_the_checked_methods() {
  let step = if cfg!(miri) { 7 } else { 1 };
  for a in (-64..=63).step_by(step) {
    let native = i7::new(a);
    let array = Int::<U7, [u8; 1]>::checked_from_limbs(limbs_of(a.into())).expect("7 bits");
    if let Some(negated) = native.checked_neg() {
      assert_eq!((-native, -&native), (negated, negated));
      let negated = negated.value() as u8;
      assert_eq!(
        ((-array).to_limbs(), (-&array).to_limbs()),
        ([negated], [negated])
      );
    }
    for b in -64..=63 {
      let (x, y) = (i7::new(a), i7::new(b));
      if let Some(product) = x.checked_mul(&y) {
        assert_eq!([x, y].into_iter().product::<i7>(), product);
        assert_eq!([x, y].iter().product::<i7>(), product);
      }
    }
  }
  for a in (0..=127u8).step_by(step) {
    for b in 0..=127 {
      let (x, y) = (u7::new(a), u7::new(b));
      if let Some(product) = x.checked_mul(&y) {
        assert_eq!([x, y].into_iter().product::<u7>(), product);
        assert_eq!([x, y].iter().product::<u7>(), product);
      }
    }
  }
  assert_eq!(-Int::<U0, i8>::MIN, Int::<U0, i8>::ZERO);
  assert_eq!(core::iter::empty::<u7>().product::<u7>(), u7::ONE);
  assert_eq!(core::iter::empty::<&i256>().product::<i256>(), i256::ONE);
}

/// The results of the families, of `is_one` and `is_zero`, and of the strict methods, the
/// operators, `+=` and `Sum` where they do not overflow, for the values `$a` and `$b` of `$ty`, as
/// text. The methods are those that the context reaches: the inherent `const fn`s for a concrete
/// storage, and the traits in code over `B: BitWidth`.
macro_rules! family_results {
  ($ty:ty; $a:expr, $b:expr) => {{
    let (a, b): ($ty, $ty) = ($a, $b);
    let assigned = |mut target: $ty| {
      target += b;
      target
    };
    vec![
      format!("{:?}", (a.overflowing_add(&b), a.overflowing_sub(&b))),
      format!("{:?}", (a.overflowing_mul(&b), a.overflowing_neg())),
      format!("{:?}", (a.checked_add(&b), a.checked_sub(&b))),
      format!("{:?}", (a.checked_mul(&b), a.checked_neg())),
      format!("{:?}", (a.wrapping_add(&b), a.wrapping_sub(&b))),
      format!("{:?}", (a.wrapping_mul(&b), a.wrapping_neg())),
      format!(
        "{:?}",
        (
          a.saturating_add(&b),
          a.saturating_sub(&b),
          a.saturating_mul(&b)
        )
      ),
      format!("{:?}", (a.is_zero(), a.is_one())),
      format!(
        "{:?}",
        a.checked_add(&b).map(|_| (
          a.strict_add(&b),
          a + b,
          assigned(a),
          [a, b].into_iter().sum::<$ty>()
        ))
      ),
      format!("{:?}", a.checked_sub(&b).map(|_| (a.strict_sub(&b), a - b))),
      format!("{:?}", a.checked_mul(&b).map(|_| (a.strict_mul(&b), a * b))),
      format!("{:?}", a.checked_neg().map(|_| a.strict_neg())),
    ]
  }};
}

/// The results of the methods of `Uint` alone for `$a` and `$b`, as [`family_results!`] gives
/// them.
macro_rules! unsigned_results {
  ($a:expr, $b:expr) => {{
    let (a, b) = ($a, $b);
    vec![
      format!(
        "{:?}",
        (a.carrying_add(&b, false), a.carrying_add(&b, true))
      ),
      format!(
        "{:?}",
        (a.borrowing_sub(&b, false), a.borrowing_sub(&b, true))
      ),
    ]
  }};
}

/// The results of the methods of `Int` alone for `$a`, as [`family_results!`] gives them.
macro_rules! signed_results {
  ($a:expr) => {{
    let a = $a;
    vec![
      format!(
        "{:?}",
        (a.overflowing_abs(), a.checked_abs(), a.wrapping_abs())
      ),
      format!("{:?}", (a.saturating_abs(), a.saturating_neg(), a.signum())),
      format!("{:?}", (a.unsigned_abs(), a.is_negative(), a.is_positive())),
      format!("{:?}", a.checked_abs().map(|_| (a.strict_abs(), a.abs()))),
      format!("{:?}", a.checked_neg().map(|_| -a)),
    ]
  }};
}

/// The results of every method of `Uint<B>` for `a` and `b` through the traits.
fn unsigned_through_traits<B: BitWidth>(a: Uint<B>, b: Uint<B>) -> Vec<String> {
  let mut results = family_results!(Uint<B>; a, b);
  results.extend(unsigned_results!(a, b));
  results
}

/// The results of every method of `Int<B>` for `a` and `b` through the traits.
fn signed_through_traits<B: BitWidth>(a: Int<B>, b: Int<B>) -> Vec<String> {
  let mut results = family_results!(Int<B>; a, b);
  results.extend(signed_results!(a));
  results
}

/// Code over `B: BitWidth` reaches every method of `Uint<B>` and `Int<B>`, which gives the results
/// of the inherent `const fn`s of the default storages, at widths of each kind of default storage:
/// for the pairs of zero, the limits and the values that wrap from them.
#[test]
fn generic_over_the_width() {
  macro_rules! agrees {
    ($($width:ident),+ $(,)?) => {$({
      type U = Uint<$width>;
      type I = Int<$width>;
      let unsigned = [U::ZERO, U::MAX, U::MAX.wrapping_add(&U::MAX), U::MAX.wrapping_mul(&U::MAX)];
      for &a in &unsigned {
        for &b in &unsigned {
          let mut inherent = family_results!(U; a, b);
          inherent.extend(unsigned_results!(a, b));
          assert_eq!(unsigned_through_traits::<$width>(a, b), inherent, "{a} and {b}");
        }
      }
      let signed = [
        I::ZERO,
        I::MIN,
        I::MAX,
        I::MIN.wrapping_sub(&I::MAX),
        I::MAX.wrapping_add(&I::MAX),
      ];
      for &a in &signed {
        for &b in &signed {
          let mut inherent = family_results!(I; a, b);
          inherent.extend(signed_results!(a));
          assert_eq!(signed_through_traits::<$width>(a, b), inherent, "{a} and {b}");
        }
      }
    })+};
  }

  agrees!(U0, U1, U7, U8, U64, U65, U128, U129, U200, U1024);
}

/// `ONE` and `Product` in code over `B: BitWidth`, with the bound on the width that holds one.
#[test]
fn bit_width_gives_one_and_product() {
  fn unsigned<B: BitWidth + NonZero>() -> (Uint<B>, Uint<B>) {
    let one = Uint::<B>::ONE;
    (one, [one, one].into_iter().product())
  }
  fn signed<B: BitWidth + IsGreater<U1, Output = True>>() -> (Int<B>, Int<B>) {
    let one = Int::<B>::ONE;
    (one, [-one, -one].iter().product())
  }
  assert_eq!(unsigned::<U1>(), (Uint::<U1>::ONE, Uint::<U1>::ONE));
  assert_eq!(unsigned::<U200>(), (Uint::<U200>::ONE, Uint::<U200>::ONE));
  assert_eq!(signed::<U2>(), (Int::<U2>::ONE, Int::<U2>::ONE));
  assert_eq!(signed::<U129>(), (Int::<U129>::ONE, Int::<U129>::ONE));
}

/// The bound `B: BitWidth` gives `Uint<B>` and `Int<B>` the families and the traits of each sign,
/// with their operators, and with a width that holds one, `HasOne` and `Product`. Every storage is
/// updated in place, and representative storages have every form of the operators, of `Sum` and
/// of `Product`.
#[test]
fn arithmetic_trait_surface() {
  fn families<T: OverflowingOps + CheckedOps + WrappingOps + SaturatingOps + StrictOps>() {}
  fn unsigned<T: FixedInteger + UnsignedFixedInteger>() {}
  fn signed<T: FixedInteger + SignedFixedInteger + Neg<Output = T>>() {}
  fn has_one<T: HasOne>() {}
  fn mutable<U: UintReprMut, S: IntReprMut>() {}
  fn operators<T>()
  where
    T: FixedInteger + Product + for<'a> Sum<&'a T> + for<'a> Product<&'a T>,
    T: for<'a> Add<&'a T, Output = T> + for<'a> Sub<&'a T, Output = T>,
    T: for<'a> Mul<&'a T, Output = T>,
    T: for<'a> AddAssign<&'a T> + for<'a> SubAssign<&'a T> + for<'a> MulAssign<&'a T>,
    for<'a> &'a T: Add<T, Output = T> + Sub<T, Output = T> + Mul<T, Output = T>,
    for<'a, 'b> &'a T: Add<&'b T, Output = T> + Sub<&'b T, Output = T> + Mul<&'b T, Output = T>,
  {
  }
  fn negation<T: Neg<Output = T>>()
  where
    for<'a> &'a T: Neg<Output = T>,
  {
  }
  fn width<B: BitWidth>() {
    families::<Uint<B>>();
    families::<Int<B>>();
    unsigned::<Uint<B>>();
    signed::<Int<B>>();
  }
  fn unsigned_one<B: BitWidth + NonZero>() {
    has_one::<Uint<B>>();
  }
  fn signed_one<B: BitWidth + IsGreater<U1, Output = True>>() {
    has_one::<Int<B>>();
  }

  width::<U0>();
  width::<U1>();
  width::<U200>();
  width::<U4096>();
  unsigned_one::<U1>();
  unsigned_one::<U4096>();
  signed_one::<U2>();
  signed_one::<U4096>();
  mutable::<u8, i8>();
  mutable::<u128, i128>();
  mutable::<[u8; 0], [u64; 0]>();
  mutable::<[u64; 4], [u128; 2]>();
  operators::<u7>();
  operators::<Uint<U8, [u8; 1]>>();
  operators::<Uint<U200, [u64; 4]>>();
  operators::<i7>();
  operators::<i256>();
  operators::<Int<U24, [u8; 3]>>();
  negation::<i7>();
  negation::<i256>();
  negation::<Int<U0, [u8; 0]>>();
}

/// The arithmetic in constant contexts, each family in a native integer and in an array, with
/// inputs that do not overflow where an overflow would panic.
#[test]
fn arithmetic_in_constant_contexts() {
  type Byte = Uint<U7, [u8; 1]>;
  type Bytes = Int<U24, [u8; 3]>;
  const OVERFLOWING: [(u7, bool); 2] = [
    u7::MAX.overflowing_add(&u7::ONE),
    u7::MAX.overflowing_mul(&u7::MAX),
  ];
  const OVERFLOWING_ARRAY: [(Bytes, bool); 2] = [
    Bytes::MIN.overflowing_sub(&Bytes::ONE),
    Bytes::MIN.overflowing_neg(),
  ];
  const CHECKED: [Option<i7>; 2] = [i7::MIN.checked_neg(), i7::MAX.checked_sub(&i7::ONE)];
  const CHECKED_ARRAY: [Option<Byte>; 2] = [
    Byte::MAX.checked_sub(&Byte::ONE),
    Byte::ZERO.checked_sub(&Byte::ONE),
  ];
  const WRAPPING: [u7; 2] = [u7::ZERO.wrapping_sub(&u7::ONE), u7::ONE.wrapping_neg()];
  const WRAPPING_ARRAY: u4096 = u4096::MAX.wrapping_mul(&u4096::MAX);
  const SATURATING: [i7; 2] = [
    i7::MIN.saturating_sub(&i7::ONE),
    i7::MAX.saturating_mul(&i7::new(2)),
  ];
  const SATURATING_ARRAY: [Byte; 2] = [
    Byte::MAX.saturating_add(&Byte::ONE),
    Byte::ZERO.saturating_sub(&Byte::ONE),
  ];
  const STRICT: [i7; 2] = [i7::new(-5).strict_mul(&i7::new(12)), i7::MAX.strict_neg()];
  const STRICT_ARRAY: [Bytes; 3] = [
    Bytes::MIN.strict_add(&Bytes::ONE),
    Bytes::MAX.strict_sub(&Bytes::ONE),
    Bytes::ONE.strict_neg(),
  ];
  const STRICT_ABS: Bytes = Bytes::MIN.strict_add(&Bytes::ONE).strict_abs();
  const ONES: [bool; 4] = [
    u7::ONE.is_one(),
    Byte::ONE.is_one(),
    i256::ONE.is_one(),
    i7::MIN.is_one(),
  ];
  const SIGNS: [i256; 3] = [i256::MIN.signum(), i256::ZERO.signum(), i256::MAX.signum()];
  const MAGNITUDES: (u7, u256) = (i7::MIN.unsigned_abs(), i256::MIN.unsigned_abs());
  const CARRIES: [(u256, bool); 2] = [
    u256::MAX.carrying_add(&u256::ZERO, true),
    u256::ZERO.borrowing_sub(&u256::ZERO, true),
  ];
  const ABS: [i7; 3] = [
    i7::new(-5).abs(),
    i7::MIN.wrapping_abs(),
    i7::MIN.saturating_abs(),
  ];

  assert_eq!(OVERFLOWING, [(u7::ZERO, true), (u7::ONE, true)]);
  assert_eq!(OVERFLOWING_ARRAY, [(Bytes::MAX, true), (Bytes::MIN, true)]);
  assert_eq!(CHECKED, [None, Some(i7::new(62))]);
  assert_eq!(
    CHECKED_ARRAY,
    [Some(Byte::MAX.wrapping_sub(&Byte::ONE)), None]
  );
  assert_eq!(WRAPPING, [u7::MAX, u7::MAX]);
  assert_eq!(WRAPPING_ARRAY, u4096::ONE);
  assert_eq!(SATURATING, [i7::MIN, i7::MAX]);
  assert_eq!(SATURATING_ARRAY, [Byte::MAX, Byte::ZERO]);
  assert_eq!(STRICT, [i7::new(-60), i7::new(-63)]);
  assert_eq!(
    STRICT_ARRAY.map(|value| value.to_string()),
    ["-8388607", "8388606", "-1"]
  );
  assert_eq!(STRICT_ABS.to_string(), "8388607");
  assert_eq!(ONES, [true, true, true, false]);
  assert_eq!(SIGNS, [-i256::ONE, i256::ZERO, i256::ONE]);
  assert_eq!(MAGNITUDES.0, u7::new(64));
  assert_eq!(MAGNITUDES.1.to_limbs(), [0, 0, 0, 1 << 63]);
  assert_eq!(CARRIES, [(u256::ZERO, true), (u256::MAX, true)]);
  assert_eq!(ABS, [i7::new(5), i7::MIN, i7::MAX]);
}

/// Uses the operators, `Sum` and `Product` that the bound `B: BitWidth` gives `Uint<B>` and
/// `Int<B>`, with `ONE` from the bounds on the width, which is at least 7 bits.
#[allow(clippy::op_ref, reason = "the operators on references are under test")]
fn generic_operators<B: BitWidth + NonZero + IsGreater<U1, Output = True>>() {
  let one = Uint::<B>::ONE;
  let (two, three) = (one + one, one + &one + one);
  let mut six = two;
  six *= three;
  assert_eq!(six.to_string(), "6");
  assert_eq!((six - &three).to_string(), "3");
  assert_eq!((&six * &two).to_string(), "12");
  assert_eq!([one, two, three].iter().sum::<Uint<B>>(), six);
  assert_eq!([two, three].into_iter().product::<Uint<B>>(), six);

  let one = Int::<B>::ONE;
  let minus_three = -(one + one + one);
  let mut value = minus_three;
  value -= one;
  value += &one;
  value *= -&one;
  assert_eq!(value.to_string(), "3");
  assert_eq!((minus_three * minus_three).to_string(), "9");
  assert_eq!([minus_three, one, one].into_iter().sum::<Int<B>>(), -one);
  assert_eq!([minus_three, -one].iter().product::<Int<B>>(), value);
}

#[test]
fn bit_width_gives_the_operators() {
  generic_operators::<U7>();
  generic_operators::<U8>();
  generic_operators::<U64>();
  generic_operators::<U65>();
  generic_operators::<U128>();
  generic_operators::<U129>();
  generic_operators::<U200>();
  generic_operators::<U1024>();
}

/// The operators on two references and the negation of a reference need only the storage trait,
/// `UintRepr` or `IntRepr`, not a storage that holds the value inline: code bounded by it uses
/// them, in a native integer and in an array.
#[test]
fn reference_operators_need_only_the_storage_trait() {
  use crate::{FitsBits, IntRepr, UintRepr, i7, i256, u7, u256};

  fn unsigned<B: FitsBits<R>, R: UintRepr>(a: Uint<B, R>, b: Uint<B, R>) -> [Uint<B, R>; 3] {
    [&a + &b, &a - &b, &a * &b]
  }

  fn signed<B: FitsBits<R>, R: IntRepr>(a: Int<B, R>, b: Int<B, R>) -> [Int<B, R>; 4] {
    [&a + &b, &a - &b, &a * &b, -&a]
  }

  assert_eq!(
    unsigned(u7::new(10), u7::new(3)),
    [u7::new(13), u7::new(7), u7::new(30)]
  );
  let (ten, three) = (
    u256::from_limbs([10, 0, 0, 0]),
    u256::from_limbs([3, 0, 0, 0]),
  );
  assert_eq!(
    unsigned(ten, three).map(|value| value.to_string()),
    ["13", "7", "30"]
  );
  assert_eq!(
    signed(i7::new(-10), i7::new(3)),
    [i7::new(-7), i7::new(-13), i7::new(-30), i7::new(10)]
  );
  let (minus_ten, three) = (
    -i256::from_limbs([10, 0, 0, 0]),
    i256::from_limbs([3, 0, 0, 0]),
  );
  assert_eq!(
    signed(minus_ten, three).map(|value| value.to_string()),
    ["-7", "-13", "-30", "10"]
  );
}
