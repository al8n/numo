use core::{
  fmt::Debug,
  hint::black_box,
  iter::{Product, Sum},
  ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign},
};
use std::{
  boxed::Box,
  cell::RefCell,
  format,
  panic::{self, AssertUnwindSafe},
  path::PathBuf,
  string::{String, ToString},
  sync::Once,
  thread_local, vec,
  vec::Vec,
};

use num_bigint::BigInt;
use typenum::{
  IsGreater, NonZero, True, U0, U1, U2, U7, U8, U9, U16, U24, U32, U63, U64, U65, U100, U127, U128,
  U129, U200, U256, U1024, U4096,
};

use crate::{
  BitWidth, CheckedOps, ConstFixedInteger, ConstIntegerOne, FitsBits, FixedInteger, Int,
  IntReprMut, IntegerMath, IntegerOne, OverflowingOps, SaturatingOps, SignedFixedInteger,
  SignedMath, StrictOps, Uint, UintReprMut, UncheckedOps, UnsignedFixedInteger, UnsignedMath,
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

/// The values of `bits` bits that a stride may skip and every sweep checks: the limits, the values
/// next to them, -1, 0 and 1, where they are in range.
fn ends(bits: u32, signed: bool) -> Vec<i128> {
  let (min, max) = range(bits, signed);
  let mut ends: Vec<i128> = [min, min + 1, -1, 0, 1, max - 1, max]
    .into_iter()
    .filter(|value| (min..=max).contains(value))
    .collect();
  ends.sort_unstable();
  ends.dedup();
  ends
}

/// The values of `bits` bits that a sweep checks, in order: every `step`th value from the smallest,
/// and the values of [`ends`].
fn every_value(bits: u32, signed: bool, step: usize) -> Vec<i128> {
  let (min, max) = range(bits, signed);
  let mut values: Vec<i128> = (min..=max).step_by(step).collect();
  values.extend(ends(bits, signed));
  values.sort_unstable();
  values.dedup();
  values
}

/// The pairs of values of `bits` bits that a sweep checks, in order: every `step`th pair, counted
/// from `(min, min)` with the second value varying fastest, and every pair of the values of
/// [`ends`], such as `(MIN, -1)`, `(MIN, 0)` and `(MAX, MAX)`.
fn every_pair(bits: u32, signed: bool, step: usize) -> Vec<(i128, i128)> {
  let (min, max) = range(bits, signed);
  let count = max - min + 1;
  let mut pairs: Vec<(i128, i128)> = (0..count * count)
    .step_by(step)
    .map(|pair| (min + pair / count, min + pair % count))
    .collect();
  let ends = ends(bits, signed);
  pairs.extend(ends.iter().flat_map(|&a| ends.iter().map(move |&b| (a, b))));
  pairs.sort_unstable();
  pairs.dedup();
  pairs
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
/// pair, or under Miri every 7th value and every 61st pair, with the ends that `every_value` and
/// `every_pair` keep.
macro_rules! check_every_pair {
  ($ty:ty, $bits:expr, $signed:expr, $step:expr, $from:expr) => {{
    let from: fn(i128) -> $ty = $from;
    let (bits, signed) = ($bits, $signed);
    let (min, max) = range(bits, signed);
    let (value_step, pair_step) = if cfg!(miri) { (7, 61) } else { (1, $step) };
    for a in every_value(bits, signed, value_step) {
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
    for (a, b) in every_pair(bits, signed, pair_step) {
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
/// width. Every value and every `$step`th pair, or under Miri every 7th value and every 61st pair,
/// with the ends that `every_value` and `every_pair` keep.
macro_rules! check_unsigned_methods {
  ($ty:ty, $bits:expr, $step:expr, $from:expr) => {{
    let from: fn(i128) -> $ty = $from;
    let bits = $bits;
    let (value_step, pair_step) = if cfg!(miri) { (7, 61) } else { (1, $step) };
    for a in every_value(bits, false, value_step) {
      assert_eq!(both!(FixedInteger::is_one(&from(a))), a == 1, "{a}");
    }
    for (a, b) in every_pair(bits, false, pair_step) {
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
/// under Miri every 7th value and the ends that `every_value` keeps: `is_one`, the absolute value
/// in every form, the saturating negation, `signum` and `unsigned_abs`, whose result `$unsigned`
/// creates, through the inherent `const fn`s and the traits, against the exact results wrapped to
/// the width.
macro_rules! check_signed_methods {
  ($ty:ty, $bits:expr, $from:expr, $unsigned:expr) => {{
    let from: fn(i128) -> $ty = $from;
    let bits = $bits;
    let (min, max) = range(bits, true);
    let step = if cfg!(miri) { 7 } else { 1 };
    for a in every_value(bits, true, step) {
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
/// under Miri every 61st pair, with the ends that `every_pair` keeps.
macro_rules! check_operators {
  ($ty:ty, $bits:expr, $signed:expr, $step:expr, $from:expr) => {{
    let from: fn(i128) -> $ty = $from;
    let step = if cfg!(miri) { 61 } else { $step };
    for (a, b) in every_pair($bits, $signed, step) {
      let (x, y) = (from(a), from(b));
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

/// The sweeps keep the ends of the range under any stride, as under Miri: one signed bit keeps -1
/// and 0, the only operand of a legal negation, and wider ranges keep the limits, -1, 0 and 1, and
/// their pairs, such as `(MIN, -1)` and `(MAX, MAX)`.
#[test]
fn sampling_keeps_the_ends() {
  assert_eq!(every_value(0, true, 7), [0]);
  assert_eq!(every_value(1, true, 7), [-1, 0]);
  assert_eq!(
    every_pair(1, true, 61),
    [(-1, -1), (-1, 0), (0, -1), (0, 0)]
  );
  for (bits, signed) in [(2, true), (7, true), (7, false), (8, true), (8, false)] {
    let (min, max) = range(bits, signed);
    let values = every_value(bits, signed, 7);
    let pairs = every_pair(bits, signed, 61);
    let ends = ends(bits, signed);
    assert!(ends.contains(&min) && ends.contains(&max) && ends.contains(&0));
    assert_eq!(ends.contains(&-1), signed);
    for &a in &ends {
      assert!(values.contains(&a), "{a} in {bits} bits");
      for &b in &ends {
        assert!(pairs.contains(&(a, b)), "({a}, {b}) in {bits} bits");
      }
    }
    assert!(values.is_sorted() && pairs.is_sorted());
    assert!(values.iter().all(|value| (min..=max).contains(value)));
  }
}

/// `ONE` is the storage's one, and the constant of `ConstIntegerOne` and the function of
/// `IntegerOne` are the inherent one, from one bit for `Uint` and from two bits for `Int`. Zero
/// bits, which have no `ONE`, overflow on a carry and on a borrow, and negate `MIN`, which is zero,
/// to itself.
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
  assert_eq!(<Uint<U1, u8> as ConstIntegerOne>::ONE, Uint::<U1, u8>::ONE);
  assert_eq!(
    <Uint<U7, [u8; 2]> as ConstIntegerOne>::ONE,
    Uint::<U7, [u8; 2]>::ONE
  );
  assert_eq!(<Int<U2, i8> as ConstIntegerOne>::ONE, Int::<U2, i8>::ONE);
  assert_eq!(
    <Int<U8, [u8; 3]> as ConstIntegerOne>::ONE,
    Int::<U8, [u8; 3]>::ONE
  );
  assert_eq!(<Uint<U1, u8> as IntegerOne>::one(), Uint::<U1, u8>::ONE);
  assert_eq!(
    <Uint<U7, [u8; 2]> as IntegerOne>::one(),
    Uint::<U7, [u8; 2]>::ONE
  );
  assert_eq!(<Int<U2, i8> as IntegerOne>::one(), Int::<U2, i8>::ONE);
  assert_eq!(
    <Int<U8, [u8; 3]> as IntegerOne>::one(),
    Int::<U8, [u8; 3]>::ONE
  );
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
  // The location names the file with the separator of the host, and paths compare by component.
  for location in [sum, product] {
    let file = location.map(|(file, _)| PathBuf::from(file));
    let expected = checks.then(|| PathBuf::from("src/integer/arithmetic/mod.rs"));
    assert_eq!(file, expected);
  }
}

/// `-` on values and references, and `Product` over values and references, give the results of
/// `checked_neg` and `checked_mul` where these are `Some`, for every value and pair of 7 bits in a
/// native integer and in an array of one byte, or under Miri every 7th value, with the ends that
/// `every_value` keeps, against every value.
#[test]
fn negation_and_products_match_the_checked_methods() {
  let step = if cfg!(miri) { 7 } else { 1 };
  for a in every_value(7, true, step) {
    let a = a as i8;
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
  for a in every_value(7, false, step) {
    for b in 0..=127 {
      let (x, y) = (u7::new(a as u8), u7::new(b));
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

/// The results of the families, of `is_one` and `is_zero`, of the strict methods, the operators,
/// `+=` and `Sum` where they do not overflow, and of the functions of `IntegerMath` where they do
/// not panic, for the values `$a` and `$b` of `$ty`, as text. The methods are those that the
/// context reaches: the inherent `const fn`s for a concrete storage, and the traits in code over
/// `B: BitWidth`.
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
      format!(
        "{:?}",
        (
          a.checked_div(&b),
          a.checked_rem(&b),
          a.checked_div_euclid(&b),
          a.checked_rem_euclid(&b)
        )
      ),
      format!(
        "{:?}",
        (
          a.checked_next_multiple_of(&b),
          a.checked_div_exact(&b),
          a.midpoint(&b),
          a.abs_diff(&b)
        )
      ),
      // Every method that can panic is called only where the checked division returns a value.
      format!(
        "{:?}",
        a.checked_div(&b).map(|_| (
          a.overflowing_div(&b),
          a.overflowing_rem(&b),
          a.overflowing_div_euclid(&b),
          a.overflowing_rem_euclid(&b)
        ))
      ),
      format!(
        "{:?}",
        a.checked_div(&b).map(|_| (
          a.wrapping_div(&b),
          a.wrapping_rem(&b),
          a.wrapping_div_euclid(&b),
          a.wrapping_rem_euclid(&b),
          a.saturating_div(&b)
        ))
      ),
      format!(
        "{:?}",
        a.checked_div(&b).map(|_| (
          a.strict_div(&b),
          a.strict_rem(&b),
          a.strict_div_euclid(&b),
          a.strict_rem_euclid(&b)
        ))
      ),
      format!(
        "{:?}",
        a.checked_div(&b).map(|_| (
          a.div_euclid(&b),
          a.rem_euclid(&b),
          a.div_floor(&b),
          a.div_ceil(&b),
          a.div_exact(&b)
        ))
      ),
      format!(
        "{:?}",
        a.checked_next_multiple_of(&b)
          .map(|_| a.next_multiple_of(&b))
      ),
      format!(
        "{:?}",
        (
          a.overflowing_pow(3),
          a.checked_pow(2),
          a.wrapping_pow(u32::MAX),
          a.saturating_pow(5)
        )
      ),
      format!(
        "{:?}",
        a.checked_pow(2).map(|_| (a.strict_pow(2), a.pow(2)))
      ),
      format!(
        "{:?}",
        (
          a.checked_nth_root(3),
          a.checked_nth_root(2).map(|_| (a.isqrt(), a.nth_root(2))),
          a.checked_ilog(&b),
          a.checked_ilog(&b).map(|_| a.ilog(&b))
        )
      ),
      format!(
        "{:?}",
        (
          a.checked_ilog2(),
          a.checked_ilog10(),
          a.checked_ilog2().map(|_| (a.ilog2(), a.ilog10()))
        )
      ),
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
      format!(
        "{:?}",
        (
          a.is_multiple_of(&b),
          a.checked_next_power_of_two(),
          a.wrapping_next_power_of_two()
        )
      ),
      format!(
        "{:?}",
        a.checked_next_power_of_two().map(|_| a.next_power_of_two())
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
      format!("{:?}", a.checked_isqrt()),
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

/// The bound `B: BitWidth` gives `Uint<B>` and `Int<B>` the families, `IntegerMath`, the traits of
/// each sign and `UnsignedMath` and `SignedMath`, with their operators, the constants of the const
/// layer, and with a width that holds one, `IntegerOne`, `ConstIntegerOne` and `Product`. Every
/// storage is updated in place, and representative storages have every form of the operators, of
/// `Sum` and of `Product`.
#[test]
fn arithmetic_trait_surface() {
  fn families<T>()
  where
    T: OverflowingOps + CheckedOps + WrappingOps + SaturatingOps + StrictOps + UncheckedOps,
    T: IntegerMath,
  {
  }
  fn unsigned<T: FixedInteger + UnsignedFixedInteger + UnsignedMath>() {}
  fn signed<T: FixedInteger + SignedFixedInteger + SignedMath + Neg<Output = T>>() {}
  fn constants<T: FixedInteger + ConstFixedInteger>() {}
  fn integer_one<T: IntegerOne>() {}
  fn const_integer_one<T: ConstIntegerOne>() {}
  fn mutable<U: UintReprMut, S: IntReprMut>() {}
  fn operators<T>()
  where
    T: FixedInteger + Product + for<'a> Sum<&'a T> + for<'a> Product<&'a T>,
    T: for<'a> Add<&'a T, Output = T> + for<'a> Sub<&'a T, Output = T>,
    T: for<'a> Mul<&'a T, Output = T>,
    T: for<'a> Div<&'a T, Output = T> + for<'a> Rem<&'a T, Output = T>,
    T: for<'a> AddAssign<&'a T> + for<'a> SubAssign<&'a T> + for<'a> MulAssign<&'a T>,
    T: for<'a> DivAssign<&'a T> + for<'a> RemAssign<&'a T>,
    for<'a> &'a T: Add<T, Output = T> + Sub<T, Output = T> + Mul<T, Output = T>,
    for<'a> &'a T: Div<T, Output = T> + Rem<T, Output = T>,
    for<'a, 'b> &'a T: Add<&'b T, Output = T> + Sub<&'b T, Output = T> + Mul<&'b T, Output = T>,
    for<'a, 'b> &'a T: Div<&'b T, Output = T> + Rem<&'b T, Output = T>,
  {
  }
  // `FixedInteger` gives the operators on values and their compound assignments with values.
  fn fixed_operators<T: FixedInteger>() {
    fn division<T: Div<Output = T> + Rem<Output = T> + DivAssign + RemAssign>() {}
    division::<T>();
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
    constants::<Uint<B>>();
    constants::<Int<B>>();
    fixed_operators::<Uint<B>>();
    fixed_operators::<Int<B>>();
  }
  fn unsigned_one<B: BitWidth + NonZero>() {
    integer_one::<Uint<B>>();
    const_integer_one::<Uint<B>>();
  }
  fn signed_one<B: BitWidth + IsGreater<U1, Output = True>>() {
    integer_one::<Int<B>>();
    const_integer_one::<Int<B>>();
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
  // SAFETY: each sum, difference and product is in range, so the checked method would return it.
  const UNCHECKED: (u7, Uint<U8, u8>, Byte) = unsafe {
    (
      u7::new(100).unchecked_add(&u7::new(27)),
      Uint::<U8, u8>::new(15).unchecked_mul(&Uint::<U8, u8>::new(17)),
      Byte::MAX.unchecked_sub(&Byte::ONE),
    )
  };
  // SAFETY: each sum, difference, product and negation is in range, so the checked method would
  // return it.
  const UNCHECKED_SIGNED: (i7, [Int<U8, i8>; 2], [Bytes; 2]) = unsafe {
    (
      i7::MIN.unchecked_add(&i7::ONE).unchecked_neg(),
      [
        Int::<U8, i8>::new(-100).unchecked_sub(&Int::<U8, i8>::new(28)),
        Int::<U8, i8>::new(-127).unchecked_neg(),
      ],
      [
        Bytes::ONE.unchecked_mul(&Bytes::MIN),
        Bytes::MAX.unchecked_neg(),
      ],
    )
  };

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
  assert_eq!(
    UNCHECKED,
    (
      u7::MAX,
      Uint::<U8, u8>::MAX,
      Byte::MAX.wrapping_sub(&Byte::ONE)
    )
  );
  assert_eq!(
    UNCHECKED_SIGNED,
    (
      i7::MAX,
      [Int::<U8, i8>::MIN, Int::<U8, i8>::MAX],
      [Bytes::MIN, Bytes::MIN.wrapping_add(&Bytes::ONE)]
    )
  );
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
  assert_eq!((six / &(three + two)).to_string(), "1");
  assert_eq!((&six % three).to_string(), "0");
  let mut quotient = six;
  quotient /= two;
  quotient %= &two;
  assert_eq!(quotient.to_string(), "1");
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
  assert_eq!((minus_three / (one + one)).to_string(), "-1");
  assert_eq!((&minus_three % &(one + one)).to_string(), "-1");
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

  fn unsigned<B: FitsBits<R>, R: UintRepr>(a: Uint<B, R>, b: Uint<B, R>) -> [Uint<B, R>; 5] {
    [&a + &b, &a - &b, &a * &b, &a / &b, &a % &b]
  }

  fn signed<B: FitsBits<R>, R: IntRepr>(a: Int<B, R>, b: Int<B, R>) -> [Int<B, R>; 6] {
    [&a + &b, &a - &b, &a * &b, &a / &b, &a % &b, -&a]
  }

  assert_eq!(
    unsigned(u7::new(10), u7::new(3)),
    [u7::new(13), u7::new(7), u7::new(30), u7::new(3), u7::ONE]
  );
  let (ten, three) = (
    u256::from_limbs([10, 0, 0, 0]),
    u256::from_limbs([3, 0, 0, 0]),
  );
  assert_eq!(
    unsigned(ten, three).map(|value| value.to_string()),
    ["13", "7", "30", "3", "1"]
  );
  assert_eq!(
    signed(i7::new(-10), i7::new(3)),
    [
      i7::new(-7),
      i7::new(-13),
      i7::new(-30),
      i7::new(-3),
      i7::new(-1),
      i7::new(10)
    ]
  );
  let (minus_ten, three) = (
    -i256::from_limbs([10, 0, 0, 0]),
    i256::from_limbs([3, 0, 0, 0]),
  );
  assert_eq!(
    signed(minus_ten, three).map(|value| value.to_string()),
    ["-7", "-13", "-30", "-3", "-1", "10"]
  );
}

/// The values of `bits` bits that `from` creates from their value: every value, or under Miri every
/// 7th value and the ends that `every_value` keeps.
fn values_of<T>(bits: u32, signed: bool, from: impl Fn(i128) -> T) -> Vec<T> {
  let step = if cfg!(miri) { 7 } else { 1 };
  every_value(bits, signed, step)
    .into_iter()
    .map(from)
    .collect()
}

/// Checks the `unchecked_*` methods at every pair of `$values`, and with `neg` `unchecked_neg` at
/// every value, against the checked methods, through the inherent methods and through the traits,
/// `UncheckedOps` and `SignedFixedInteger`: each is called only where the checked method returns
/// a value, and both paths must return that value. Each operation must be called at least once,
/// so that the check cannot pass where the checked methods return no value: zero, which every set
/// of values here holds, gives each operation a call, and at zero bits each is called exactly once,
/// on zero.
macro_rules! check_unchecked {
  (@neg $x:ident, $calls:ident) => {
    if let Some(negated) = $x.checked_neg() {
      // SAFETY: `checked_neg` returns `Some`, which is the precondition of `unchecked_neg`.
      let results = unsafe { ($x.unchecked_neg(), SignedFixedInteger::unchecked_neg($x)) };
      assert_eq!(results, (negated, negated), "-{}", $x);
      $calls[3] += 1;
    }
  };
  (@called neg $calls:ident) => {
    assert!($calls[3] > 0, "no call of unchecked_neg");
  };
  ($values:expr $(, $neg:ident)?) => {{
    let values = $values;
    // The calls of `unchecked_add`, `unchecked_sub`, `unchecked_mul` and `unchecked_neg`.
    let mut calls = [0usize; 4];
    for x in &values {
      $(check_unchecked!(@$neg x, calls);)?
      for y in &values {
        if let Some(sum) = x.checked_add(y) {
          // SAFETY: `checked_add` returns `Some`, which is the precondition of `unchecked_add`.
          let results = unsafe { (x.unchecked_add(y), UncheckedOps::unchecked_add(x, y)) };
          assert_eq!(results, (sum, sum), "{x} + {y}");
          calls[0] += 1;
        }
        if let Some(difference) = x.checked_sub(y) {
          // SAFETY: `checked_sub` returns `Some`, which is the precondition of `unchecked_sub`.
          let results = unsafe { (x.unchecked_sub(y), UncheckedOps::unchecked_sub(x, y)) };
          assert_eq!(results, (difference, difference), "{x} - {y}");
          calls[1] += 1;
        }
        if let Some(product) = x.checked_mul(y) {
          // SAFETY: `checked_mul` returns `Some`, which is the precondition of `unchecked_mul`.
          let results = unsafe { (x.unchecked_mul(y), UncheckedOps::unchecked_mul(x, y)) };
          assert_eq!(results, (product, product), "{x} * {y}");
          calls[2] += 1;
        }
      }
    }
    assert!(calls[..3].iter().all(|&count| count > 0), "calls: {calls:?}");
    $(check_unchecked!(@called $neg calls);)?
  }};
}

/// Checks the `unchecked_*` methods of `Uint` and `Int` of `$bits` bits, the width `$width`, at
/// every value and every pair: `native` in `$uint` and `$int`, and `array` in `[$limb; $n]`.
macro_rules! check_unchecked_storage {
  (native $width:ty, $bits:expr, $uint:ident, $int:ident) => {{
    check_unchecked!(values_of($bits, false, |v| Uint::<$width, $uint>::new(
      v as $uint
    )));
    check_unchecked!(
      values_of($bits, true, |v| Int::<$width, $int>::new(v as $int)),
      neg
    );
  }};
  (array $width:ty, $bits:expr, [$limb:ident; $n:literal]) => {{
    check_unchecked!(values_of($bits, false, |v| {
      Uint::<$width, [$limb; $n]>::checked_from_limbs(limbs_of(v)).expect("in range")
    }));
    check_unchecked!(
      values_of($bits, true, |v| {
        Int::<$width, [$limb; $n]>::checked_from_limbs(limbs_of(v)).expect("in range")
      }),
      neg
    );
  }};
}

/// The `unchecked_*` methods, inherent and through the traits, return what the checked methods
/// return wherever these return a value, at every value and every pair of zero, one, 7 and 8 bits:
/// in native integers at a partial width and at their full width, where the inherent methods call
/// the native `unchecked_*`, and in arrays of no limbs, one limb and two limbs. No other input is
/// passed, here or in any test: the standard library checks the preconditions of unsafe functions
/// in debug builds and aborts on a violation, and in release builds a violation is undefined
/// behavior.
#[test]
fn unchecked_methods_return_the_checked_results() {
  check_unchecked_storage!(native U0, 0, u8, i8);
  check_unchecked_storage!(native U1, 1, u8, i8);
  check_unchecked_storage!(native U7, 7, u8, i8);
  check_unchecked_storage!(native U8, 8, u8, i8);
  check_unchecked_storage!(native U8, 8, u16, i16);
  check_unchecked_storage!(array U0, 0, [u8; 0]);
  check_unchecked_storage!(array U0, 0, [u64; 1]);
  check_unchecked_storage!(array U1, 1, [u8; 1]);
  check_unchecked_storage!(array U7, 7, [u8; 1]);
  check_unchecked_storage!(array U8, 8, [u8; 1]);
  check_unchecked_storage!(array U7, 7, [u64; 1]);
  check_unchecked_storage!(array U8, 8, [u8; 2]);
}

/// The same at the edge and random values of `big_samples`, in native integers at a partial and at
/// their full width and in arrays of several limbs at a partial and at a full width.
#[test]
#[cfg_attr(
  all(miri, target_pointer_width = "32"),
  ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
)]
fn unchecked_methods_return_the_checked_results_in_wide_storages() {
  macro_rules! samples {
    ($width:ty, $bits:literal; $($uint:ty, $int:ty);+ $(;)?) => {$(
      check_unchecked!(
        big_samples($bits, false, 16)
          .iter()
          .map(<Uint<$width, $uint>>::from_big)
          .collect::<Vec<_>>()
      );
      check_unchecked!(
        big_samples($bits, true, 16)
          .iter()
          .map(<Int<$width, $int>>::from_big)
          .collect::<Vec<_>>(),
        neg
      );
    )+};
  }
  samples!(U9, 9; u16, i16; [u8; 2], [u8; 2]);
  samples!(U16, 16; u16, i16; [u8; 2], [u8; 2]);
  samples!(U24, 24; u32, i32; [u16; 2], [u16; 2]);
  samples!(U32, 32; u32, i32; [u16; 2], [u16; 2]);
  samples!(U63, 63; u64, i64; [u32; 2], [u32; 2]);
  samples!(U64, 64; u64, i64; [u32; 2], [u32; 2]);
  samples!(U127, 127; u128, i128; [u64; 2], [u64; 2]);
  samples!(U128, 128; u128, i128; [u64; 2], [u64; 2]);
  samples!(U200, 200; [u64; 4], [u64; 4]; [u128; 2], [u128; 2]);
  samples!(U256, 256; [u64; 4], [u64; 4]; [u128; 2], [u128; 2]);
}

/// Code over `B: BitWidth` reaches the `unchecked_*` methods through `UncheckedOps` and
/// `SignedFixedInteger`, which `check_unchecked!` calls at every pair of `unsigned` and `signed`,
/// and `unchecked_div_exact` through `UncheckedOps`, which [`unchecked_exact_divisions`] calls at
/// least once for each sign at every width but zero bits, where every divisor is zero.
fn unchecked_over_the_width<B: BitWidth>(unsigned: Vec<Uint<B>>, signed: Vec<Int<B>>) {
  let calls = [
    unchecked_exact_divisions(&unsigned),
    unchecked_exact_divisions(&signed),
  ];
  assert_eq!(calls.contains(&0), Uint::<B>::BITS == 0, "calls: {calls:?}");
  check_unchecked!(unsigned);
  check_unchecked!(signed, neg);
}

/// Calls `unchecked_div_exact` through `UncheckedOps` at every pair of `values` where
/// `checked_div_exact` returns a value, which both must return, and returns the number of calls.
fn unchecked_exact_divisions<T: CheckedOps + UncheckedOps + PartialEq + Debug>(
  values: &[T],
) -> usize {
  let mut calls = 0;
  for x in values {
    for y in values {
      if let Some(quotient) = x.checked_div_exact(y) {
        // SAFETY: `checked_div_exact` returns `Some`, the precondition of `unchecked_div_exact`.
        let result = unsafe { x.unchecked_div_exact(y) };
        assert_eq!(result, quotient, "{x:?} / {y:?} exactly");
        calls += 1;
      }
    }
  }
  calls
}

/// The `unchecked_*` methods in code over `B: BitWidth`, at widths of each kind of default storage,
/// for the pairs of zero, the limits and the values that wrap from them.
#[test]
fn bit_width_gives_the_unchecked_methods() {
  macro_rules! at {
    ($($width:ident),+ $(,)?) => {$({
      type U = Uint<$width>;
      type I = Int<$width>;
      unchecked_over_the_width::<$width>(
        vec![U::ZERO, U::MAX, U::MAX.wrapping_add(&U::MAX), U::MAX.wrapping_mul(&U::MAX)],
        vec![
          I::ZERO,
          I::MIN,
          I::MAX,
          I::MIN.wrapping_sub(&I::MAX),
          I::MAX.wrapping_add(&I::MAX),
        ],
      );
    })+};
  }

  at!(U0, U1, U7, U8, U64, U65, U128, U129, U200, U1024);
}

/// The magnitude of `IntegerMath`, the type of `abs_diff`, is `Uint<B>` itself, and that of
/// `Int<B>` the `Uint` of the same width in a storage of the same size, the type that
/// `SignedFixedInteger::Unsigned` also names: code bounded by both traits names both without
/// ambiguity.
#[test]
fn the_magnitude_is_the_unsigned_integer() {
  use core::any::TypeId;

  fn magnitude<T: IntegerMath<Magnitude = M>, M>() {}
  fn types<T: SignedFixedInteger + IntegerMath>() -> (TypeId, TypeId)
  where
    T::Unsigned: 'static,
    T::Magnitude: 'static,
  {
    (TypeId::of::<T::Unsigned>(), TypeId::of::<T::Magnitude>())
  }
  fn width<B: BitWidth>() {
    magnitude::<Uint<B>, Uint<B>>();
    magnitude::<Int<B>, Uint<B>>();
  }

  width::<U0>();
  width::<U7>();
  width::<U200>();
  magnitude::<Int<U9, [u8; 2]>, Uint<U9, [u8; 2]>>();
  for (unsigned, magnitude) in [
    types::<Int<U7, i8>>(),
    types::<Int<U16, [u8; 2]>>(),
    types::<i256>(),
  ] {
    assert_eq!(unsigned, magnitude);
  }
  assert_eq!(types::<i7>().1, TypeId::of::<u7>());
}

/// `IntegerMath`, `UnsignedMath` and `SignedMath` need only the storage trait, `UintRepr` or
/// `IntRepr`, not a storage that holds the value inline: code bounded by it uses them, in a native
/// integer and in an array.
#[test]
fn the_math_traits_need_only_the_storage_trait() {
  use crate::{IntRepr, UintRepr};

  fn unsigned<B: FitsBits<R>, R: UintRepr>(a: &Uint<B, R>, b: &Uint<B, R>) -> String {
    let root = a.nth_root(3);
    format!(
      "{:?}",
      (
        a.midpoint(b),
        a.abs_diff(b),
        a.is_multiple_of(b),
        root,
        a.checked_ilog10()
      )
    )
  }

  fn signed<B: FitsBits<R>, R: IntRepr>(a: &Int<B, R>, b: &Int<B, R>) -> String {
    format!(
      "{:?}",
      (a.div_floor(b), a.abs_diff(b), a.checked_isqrt(), a.pow(2))
    )
  }

  assert_eq!(
    unsigned(&u7::new(100), &u7::new(25)),
    "(62, 75, true, 4, Some(2))"
  );
  let (hundred, five) = (
    u256::from_limbs([100, 0, 0, 0]),
    u256::from_limbs([5, 0, 0, 0]),
  );
  assert_eq!(unsigned(&hundred, &five), "(52, 95, true, 4, Some(2))");
  assert_eq!(signed(&i7::new(-7), &i7::new(2)), "(-4, 9, None, 49)");
  let minus_seven = i256::from_limbs([u64::MAX - 6, u64::MAX, u64::MAX, u64::MAX]);
  let two = i256::from_limbs([2, 0, 0, 0]);
  assert_eq!(signed(&minus_seven, &two), "(-4, 9, None, 49)");
}

/// How thoroughly the sweeps of the division and of the powers check: every value and pair that is
/// cheap enough in an optimized build, which runs them fast; fewer in an unoptimized build, which
/// every feature combination and every sanitizer of the tests runs; and the fewest under Miri.
#[derive(Clone, Copy, Debug, PartialEq, Eq, derive_more::IsVariant)]
enum Tier {
  /// Under Miri.
  Miri,
  /// In an unoptimized build, with debug assertions.
  Debug,
  /// In an optimized build.
  Full,
}

/// The tier of this build. `--cfg all_tests` does not raise it: the sanitizers that set that flag
/// run unoptimized code.
fn tier() -> Tier {
  if cfg!(miri) {
    Tier::Miri
  } else if cfg!(debug_assertions) {
    Tier::Debug
  } else {
    Tier::Full
  }
}

/// The value of the integer of at most 128 bits in `limbs`, least significant first, read in two's
/// complement if `signed`.
fn value_of_limbs<L: bigint::Limb, const N: usize>(limbs: &[L; N], signed: bool) -> i128 {
  let bits = L::BITS * N as u32;
  let mut value = 0u128;
  for (i, &limb) in limbs.iter().enumerate() {
    value |= limb.to_u128() << (L::BITS * i as u32);
  }
  if signed && 0 < bits && bits < 128 {
    // The arithmetic shift copies the top bit of the limbs into the bits above them.
    ((value << (128 - bits)) as i128) >> (128 - bits)
  } else {
    value as i128
  }
}

/// The pairs of values of up to 8 bits that a sweep checks, as [`every_pair`] gives them: in a
/// storage that every tier checks, `primary`, every pair in an optimized build, every 13th pair in
/// an unoptimized one and every 509th under Miri; in another storage, every 7th pair in an
/// optimized build, every 31st in an unoptimized one, and none under Miri.
fn small_pairs(bits: u32, signed: bool, primary: bool) -> Vec<(i128, i128)> {
  let step = match (tier(), primary) {
    (Tier::Full, true) => 1,
    (Tier::Full, false) => 7,
    (Tier::Debug, true) => 13,
    (Tier::Debug, false) => 31,
    (Tier::Miri, true) => 509,
    (Tier::Miri, false) => return Vec::new(),
  };
  every_pair(bits, signed, step)
}

/// The values of up to 8 bits that a sweep checks, as [`every_value`] gives them: every value in a
/// storage that every tier checks, `primary`, or under Miri every 61st; and in another storage,
/// every value in an optimized build, every 7th in an unoptimized one, and none under Miri.
fn small_values(bits: u32, signed: bool, primary: bool) -> Vec<i128> {
  match (tier(), primary) {
    (Tier::Full, _) | (Tier::Debug, true) => every_value(bits, signed, 1),
    (Tier::Debug, false) => every_value(bits, signed, 7),
    (Tier::Miri, true) => every_value(bits, signed, 61),
    (Tier::Miri, false) => Vec::new(),
  }
}

/// The pairs of values of 9 to 16 bits that a sweep checks: as many as the tier takes, `2^12` in an
/// optimized build, `2^9` in an unoptimized one and `2^6` under Miri, at an odd step through the
/// pairs, counted as [`every_pair`] counts them, near their number divided by the golden ratio, so
/// that both values of the pairs spread over the range; and the pairs of the values of [`ends`].
fn large_pairs(bits: u32, signed: bool) -> Vec<(i128, i128)> {
  let count: i128 = match tier() {
    Tier::Full => 1 << 12,
    Tier::Debug => 1 << 9,
    Tier::Miri => 1 << 6,
  };
  let (min, max) = range(bits, signed);
  let size = max - min + 1;
  let pairs = size * size;
  // `pairs * (sqrt(5) - 1) / 2`, from the fraction in 64 bits.
  let step = ((pairs as u128 * 0x9e37_79b9_7f4a_7c15) >> 64) as i128 | 1;
  let mut sampled: Vec<(i128, i128)> = (0..count.min(pairs))
    .map(|k| {
      let pair = k * step % pairs;
      (min + pair / size, min + pair % size)
    })
    .collect();
  let ends = ends(bits, signed);
  sampled.extend(ends.iter().flat_map(|&a| ends.iter().map(move |&b| (a, b))));
  sampled.sort_unstable();
  sampled.dedup();
  sampled
}

/// The values of 9 to 16 bits that a sweep checks, as [`every_value`] gives them: every 61st value
/// in an optimized build, every 127th in an unoptimized one, and every 1021st under Miri.
fn large_values(bits: u32, signed: bool) -> Vec<i128> {
  let step = match tier() {
    Tier::Full => 61,
    Tier::Debug => 127,
    Tier::Miri => 1021,
  };
  every_value(bits, signed, step)
}

/// Whether the sign token `$sign`, `unsigned` or `signed`, is that of a signed integer.
macro_rules! is_signed {
  (unsigned) => {
    false
  };
  (signed) => {
    true
  };
}

/// Checks `Uint` and `Int` of the width `$width`, `$bits` bits, in one storage with `$sweep!`, at
/// the pairs or values that `$samples` gives for the width and the sign: `native` in `$uint` and
/// `$int`, and `array` in `[$limb; $n]`. `$sweep!` takes the sign token, the type, the width, the
/// samples, and the functions that create a value from an `i128` and read back a value and a
/// magnitude, the result of `abs_diff`.
macro_rules! sweep_storage {
  ($sweep:ident, $samples:expr, native $width:ty, $bits:expr, $uint:ident, $int:ident) => {{
    let samples = $samples;
    $sweep!(
      unsigned;
      Uint<$width, $uint>,
      $bits,
      samples($bits, false),
      |v| Uint::<$width, $uint>::new(v as $uint),
      |x| x.value() as i128,
      |x| x.value() as i128
    );
    $sweep!(
      signed;
      Int<$width, $int>,
      $bits,
      samples($bits, true),
      |v| Int::<$width, $int>::new(v as $int),
      |x| x.value() as i128,
      |x| x.value() as i128
    );
  }};
  ($sweep:ident, $samples:expr, array $width:ty, $bits:expr, [$limb:ident; $n:literal]) => {{
    let samples = $samples;
    $sweep!(
      unsigned;
      Uint<$width, [$limb; $n]>,
      $bits,
      samples($bits, false),
      |v| Uint::<$width, [$limb; $n]>::checked_from_limbs(limbs_of(v)).expect("in range"),
      |x| value_of_limbs(x.as_limbs(), false),
      |x| value_of_limbs(x.as_limbs(), false)
    );
    $sweep!(
      signed;
      Int<$width, [$limb; $n]>,
      $bits,
      samples($bits, true),
      |v| Int::<$width, [$limb; $n]>::checked_from_limbs(limbs_of(v)).expect("in range"),
      |x| value_of_limbs(x.as_limbs(), true),
      |x| value_of_limbs(x.as_limbs(), false)
    );
  }};
}

/// Asserts that `$value` panics with the message `$message` and reports the line of the outermost
/// macro invocation that leads to it, in its file, as `line!` does, so that invocation must fit on
/// one line.
macro_rules! panics_here {
  ($value:expr, $message:expr) => {{
    let expected = Some(($message.clone(), file!().to_string(), line!()));
    assert_eq!(panic_of(|| $value), expected, "{}", stringify!($value));
  }};
}

/// Asserts that `$value` overflows as a native operation does: with overflow checks, it panics with
/// `$message` and reports the line of the outermost macro invocation that leads to it, in its file,
/// which must therefore fit on one line, or with `in core` a line of `core`, as the native `pow`
/// does; without them, it returns `$wrapped`.
macro_rules! overflows_here {
  ($value:expr, $message:expr, $wrapped:expr) => {
    overflows_here!(@check $value, $message, $wrapped, true)
  };
  ($value:expr, $message:expr, $wrapped:expr, in core) => {
    overflows_here!(@check $value, $message, $wrapped, false)
  };
  (@check $value:expr, $message:expr, $wrapped:expr, $here:expr) => {{
    match panic_of(|| $value) {
      Some((message, file, line)) => {
        let name = stringify!($value);
        assert!(overflow_checks(), "{name} panics without overflow checks");
        assert_eq!(message, $message, "{name}");
        let caller = (file == file!() && line == line!());
        assert_eq!(caller, $here, "{name} reports {file}:{line}");
      }
      None => {
        let name = stringify!($value);
        assert!(!overflow_checks(), "{name} returns with overflow checks");
        assert_eq!($value, $wrapped, "{name}");
      }
    }
  }};
}

mod division;
mod powers;
