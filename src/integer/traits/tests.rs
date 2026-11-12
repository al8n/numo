use std::{
  format,
  string::{String, ToString},
};

use typenum::{
  IsGreater, NonZero, True, U1, U2, U7, U8, U9, U12, U24, U64, U65, U100, U127, U128, U129, U200,
  U256, U1024,
};

use super::{
  ArrayInteger, CheckedOps, ConstFixedInteger, ConstIntegerOne, ConstNonZeroLimits, FixedInteger,
  FixedNonZero, FromLimbs, IntegerAssign, IntegerOne, NativeInteger, NonZeroLimits, SaturatingOps,
  SignedFixedInteger, SignedNonZero, UnsignedFixedInteger, UnsignedNonZero,
};
use crate::{
  BitWidth, FitsBits, Int, IntRepr, NonZeroInt, NonZeroIntRepr, NonZeroUint, NonZeroUintRepr, Uint,
  UintRepr,
  bigint::{self, tier},
  i7, i256, u7, u256,
};

/// Asserts that the constants, the functions and the queries of [`FixedInteger`] and
/// [`ConstFixedInteger`] are the inherent ones for each type, at zero and the limits.
macro_rules! assert_fixed {
  ($($ty:ty),+ $(,)?) => {$({
    type T = $ty;
    assert_eq!(<T as FixedInteger>::BITS, T::BITS);
    assert_eq!(<T as FixedInteger>::zero(), T::ZERO);
    assert_eq!(<T as FixedInteger>::min_value(), T::MIN);
    assert_eq!(<T as FixedInteger>::max_value(), T::MAX);
    assert_eq!(<T as ConstFixedInteger>::ZERO, T::ZERO);
    assert_eq!(<T as ConstFixedInteger>::MIN, T::MIN);
    assert_eq!(<T as ConstFixedInteger>::MAX, T::MAX);
    assert_eq!(T::default(), <T as FixedInteger>::zero());
    for value in [T::ZERO, T::MIN, T::MAX] {
      assert_eq!(FixedInteger::is_zero(&value), value.is_zero(), "{value}");
    }
  })+};
}

/// Asserts that the function of [`IntegerOne`] and the constant of [`ConstIntegerOne`] are the
/// inherent `ONE` for each type, and that `ONE` is one.
macro_rules! assert_one {
  ($($ty:ty),+ $(,)?) => {$({
    type T = $ty;
    assert_eq!(<T as IntegerOne>::one(), T::ONE);
    assert_eq!(<T as ConstIntegerOne>::ONE, T::ONE);
    assert!(FixedInteger::is_one(&<T as IntegerOne>::one()));
  })+};
}

/// As [`assert_fixed!`], and the sign queries of [`SignedFixedInteger`].
macro_rules! assert_signed {
  ($($ty:ty),+ $(,)?) => {$({
    assert_fixed!($ty);
    type T = $ty;
    for value in [T::ZERO, T::MIN, T::MAX] {
      assert_eq!(SignedFixedInteger::is_negative(&value), value.is_negative(), "{value}");
      assert_eq!(SignedFixedInteger::is_positive(&value), value.is_positive(), "{value}");
    }
  })+};
}

/// Asserts that [`NativeInteger`] creates and returns the values that the inherent `new`, `try_new`
/// and `value` do, for each type and native value, inside and outside its range.
macro_rules! assert_native {
  ($($ty:ty: [$($value:expr),+ $(,)?];)+) => {$({
    type T = $ty;
    for value in [$($value),+] {
      assert_eq!(<T as NativeInteger>::try_new(value), T::try_new(value), "{value}");
      if let Ok(integer) = T::try_new(value) {
        assert_eq!(<T as NativeInteger>::new(value), integer);
        assert_eq!(NativeInteger::value(integer), integer.value());
      }
    }
  })+};
}

/// Asserts that [`ArrayInteger`] creates and returns the limbs that the inherent `checked_from_limbs`,
/// `to_limbs` and `as_limbs` do, for each type and pattern of limbs, inside and outside its range.
macro_rules! assert_array {
  ($($ty:ty: [$($limbs:expr),+ $(,)?];)+) => {$({
    type T = $ty;
    for limbs in [$($limbs),+] {
      let checked = <T as ArrayInteger>::checked_from_limbs(limbs);
      assert_eq!(checked, T::checked_from_limbs(limbs), "{limbs:x?}");
      if let Some(integer) = checked {
        assert_eq!(ArrayInteger::to_limbs(&integer), integer.to_limbs());
        assert_eq!(ArrayInteger::as_limbs(&integer), integer.as_limbs());
      }
    }
  })+};
}

#[test]
fn constants_and_queries_are_the_inherent_ones() {
  assert_fixed!(
    Uint<U1, u8>,
    Uint<U7, u8>,
    Uint<U8, u8>,
    Uint<U9, u16>,
    Uint<U24, u32>,
    Uint<U64, u64>,
    Uint<U100, u128>,
    Uint<U7, u128>,
    Uint<U7, [u8; 1]>,
    Uint<U24, [u8; 3]>,
    Uint<U100, [u16; 7]>,
    Uint<U100, [u32; 4]>,
    Uint<U200, [u64; 4]>,
    Uint<U256, [u128; 2]>,
    u256,
  );
  assert_signed!(
    Int<U1, i8>,
    Int<U7, i8>,
    Int<U8, i8>,
    Int<U9, i16>,
    Int<U24, i32>,
    Int<U65, i128>,
    Int<U127, i128>,
    Int<U1, i64>,
    Int<U1, [u8; 1]>,
    Int<U24, [u8; 3]>,
    Int<U100, [u16; 7]>,
    Int<U100, [u32; 4]>,
    Int<U200, [u64; 4]>,
    Int<U129, [u128; 2]>,
    i256,
  );
}

#[test]
fn one_is_the_inherent_one() {
  assert_one!(
    Uint<U1, u8>,
    Uint<U7, u8>,
    Uint<U8, u8>,
    Uint<U9, u16>,
    Uint<U24, u32>,
    Uint<U64, u64>,
    Uint<U100, u128>,
    Uint<U7, u128>,
    Uint<U7, [u8; 1]>,
    Uint<U24, [u8; 3]>,
    Uint<U100, [u16; 7]>,
    Uint<U100, [u32; 4]>,
    Uint<U200, [u64; 4]>,
    Uint<U256, [u128; 2]>,
    u256,
  );
  assert_one!(
    Int<U7, i8>,
    Int<U8, i8>,
    Int<U9, i16>,
    Int<U24, i32>,
    Int<U65, i128>,
    Int<U127, i128>,
    Int<U24, [u8; 3]>,
    Int<U100, [u16; 7]>,
    Int<U100, [u32; 4]>,
    Int<U200, [u64; 4]>,
    Int<U129, [u128; 2]>,
    i256,
  );
}

#[test]
fn construction_is_the_inherent_one() {
  assert_native!(
    Uint<U1, u8>: [0, 1, 2, u8::MAX];
    Uint<U7, u8>: [0, 100, 127, 128, u8::MAX];
    Uint<U8, u8>: [0, u8::MAX];
    Uint<U9, u16>: [511, 512];
    Uint<U64, u128>: [u128::from(u64::MAX), u128::from(u64::MAX) + 1];
    Uint<U128, u128>: [0, u128::MAX];
    Int<U1, i8>: [-2, -1, 0, 1];
    Int<U7, i8>: [-65, -64, -1, 0, 63, 64];
    Int<U8, i8>: [i8::MIN, i8::MAX];
    Int<U24, i32>: [-(1 << 23) - 1, -(1 << 23), (1 << 23) - 1, 1 << 23];
    Int<U127, i128>: [i128::MIN, i128::MIN / 2 - 1, i128::MIN / 2, i128::MAX / 2, i128::MAX];
  );
  assert_array!(
    Uint<U7, [u64; 1]>: [[0], [127], [128], [u64::MAX]];
    Uint<U12, [u8; 2]>: [[0xff, 0x0f], [0x00, 0x10]];
    Uint<U200, [u64; 4]>: [[u64::MAX, u64::MAX, u64::MAX, 0xff], [0, 0, 0, 0x100]];
    Uint<U129, [u128; 2]>: [[u128::MAX, 1], [0, 2]];
    Int<U7, [u64; 1]>: [[u64::MAX], [63], [127], [-65i64 as u64]];
    Int<U12, [u8; 2]>: [[0xff, 0xff], [0x00, 0xf8], [0x00, 0x08]];
    Int<U100, [u32; 4]>: [[0, 0, 0, 0xffff_fff8], [0, 0, 0, 0x8]];
    Int<U129, [u128; 2]>: [[u128::MAX, u128::MAX], [0, 1]];
  );

  // A width that fills its limbs takes every bit pattern, as the inherent `from_limbs` does.
  for limbs in [[0; 4], [u64::MAX; 4], [1, 0, 0, 1 << 63]] {
    assert_eq!(
      <u256 as FromLimbs>::from_limbs(limbs),
      u256::from_limbs(limbs)
    );
    assert_eq!(
      <i256 as FromLimbs>::from_limbs(limbs),
      i256::from_limbs(limbs)
    );
  }
  for limbs in [[0; 3], [u8::MAX; 3], [1, 2, 3]] {
    assert_eq!(
      <Uint<U24, [u8; 3]> as FromLimbs>::from_limbs(limbs),
      Uint::<U24, [u8; 3]>::from_limbs(limbs)
    );
    assert_eq!(
      <Int<U24, [u8; 3]> as FromLimbs>::from_limbs(limbs),
      Int::<U24, [u8; 3]>::from_limbs(limbs)
    );
  }
}

/// Checks that the functions of [`FixedInteger`] and the default return the constants of
/// [`ConstFixedInteger`].
fn check_layers<T: ConstFixedInteger>() {
  assert_eq!(T::zero(), T::ZERO);
  assert_eq!(T::min_value(), T::MIN);
  assert_eq!(T::max_value(), T::MAX);
  assert_eq!(T::default(), T::ZERO);
}

/// Uses only what the bound `B: BitWidth` gives `Uint<B>` and `Int<B>`: the constants, the traits
/// and the implementations of the default storages.
fn check_width<B: BitWidth>(bits: u32) {
  assert_eq!(Uint::<B>::BITS, bits);
  assert_eq!(Int::<B>::BITS, bits);
  assert_eq!(<Uint<B> as FixedInteger>::BITS, bits);
  assert_eq!(<Int<B> as FixedInteger>::BITS, bits);
  assert_eq!(
    <Uint<B> as ConstFixedInteger>::MIN,
    <Uint<B> as ConstFixedInteger>::ZERO
  );
  check_layers::<Uint<B>>();
  check_layers::<Int<B>>();
  assert_eq!(Uint::<B>::default(), Uint::<B>::ZERO);
  assert_eq!(Int::<B>::default(), Int::<B>::ZERO);
  assert!(Uint::<B>::ZERO.is_zero() && !Uint::<B>::MAX.is_zero());
  assert!(Int::<B>::ZERO.is_zero() && !Int::<B>::ZERO.is_negative());
  assert!(Int::<B>::MIN.is_negative() && !Int::<B>::MIN.is_positive());
  // One signed bit holds only -1 and 0.
  assert_eq!(Int::<B>::MAX.is_positive(), bits > 1);
  assert!(Uint::<B>::ZERO < Uint::<B>::MAX && Int::<B>::MIN < Int::<B>::ZERO);
  assert_eq!(
    Uint::<B>::MAX.to_string(),
    bigint::max(bits, false).to_string()
  );
  assert_eq!(
    Int::<B>::MIN.to_string(),
    bigint::min(bits, true).to_string()
  );
  assert_eq!(
    Int::<B>::MAX.to_string(),
    bigint::max(bits, true).to_string()
  );
  // The unsigned integer of `Int<B>` is `Uint<B>`.
  let unsigned: <Int<B> as SignedFixedInteger>::Unsigned = Uint::<B>::MAX;
  assert_eq!(unsigned, <Uint<B> as ConstFixedInteger>::MAX);
}

#[test]
#[cfg_attr(
  all(miri, target_pointer_width = "32"),
  ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
)]
fn bit_width_gives_the_constants_and_the_traits() {
  check_width::<U1>(1);
  check_width::<U7>(7);
  check_width::<U8>(8);
  check_width::<U64>(64);
  check_width::<U65>(65);
  check_width::<U128>(128);
  check_width::<U129>(129);
  check_width::<U200>(200);
  check_width::<U1024>(1024);
}

#[test]
fn bit_width_implies_every_trait() {
  fn unsigned<T: FixedInteger + ConstFixedInteger + UnsignedFixedInteger>() {}
  fn signed<
    T: FixedInteger + ConstFixedInteger + SignedFixedInteger<Unsigned = U>,
    U: UnsignedFixedInteger,
  >() {
  }
  fn one<T: IntegerOne + ConstIntegerOne>() {}
  fn implied<B: BitWidth>() {
    unsigned::<Uint<B>>();
    signed::<Int<B>, Uint<B>>();
  }
  fn unsigned_one<B: BitWidth + NonZero>() {
    one::<Uint<B>>();
  }
  fn signed_one<B: BitWidth + IsGreater<U1, Output = True>>() {
    one::<Int<B>>();
  }
  implied::<U1>();
  implied::<U129>();
  unsigned_one::<U1>();
  unsigned_one::<U129>();
  signed_one::<U2>();
  signed_one::<U129>();
}

/// The sum of the values, from clones.
fn sum_of<T: FixedInteger>(values: &[T]) -> T {
  values.iter().cloned().sum()
}

/// The smallest value, zero and the largest value, through the functions.
fn limits_of<T: FixedInteger>() -> [T; 3] {
  [T::min_value(), T::zero(), T::max_value()]
}

/// Whether the value is one of the limits, which the functions return by value.
fn is_limit<T: FixedInteger>(value: &T) -> bool {
  *value == T::min_value() || *value == T::max_value()
}

/// Three times the value, which is used three times through clones.
fn triple<T: FixedInteger>(value: T) -> T {
  value.clone() + value.clone() + value
}

/// The product of the values, from clones.
fn product_of<T: IntegerOne>(values: &[T]) -> T {
  values.iter().cloned().product()
}

/// The value plus one, through the function.
fn successor<T: IntegerOne>(value: T) -> T {
  value + T::one()
}

/// The general layer needs `Clone` only: the helpers above never copy, and they compile for every
/// storage, native or array, that implements the traits.
#[test]
fn the_general_layer_needs_no_copy() {
  type Bytes = Uint<U24, [u8; 3]>;
  type Signed = Int<U7>;
  let bytes = |low, high| Bytes::from_limbs([low, high, 0]);
  let minus = |count: u64| i256::from_limbs([u64::MAX - (count - 1), u64::MAX, u64::MAX, u64::MAX]);

  assert_eq!(sum_of(&[u7::new(100), u7::new(27)]), u7::MAX);
  assert_eq!(sum_of(&[bytes(200, 1), bytes(100, 2)]), bytes(44, 4));
  assert_eq!(
    sum_of(&[Signed::new(50), Signed::new(-20)]),
    Signed::new(30)
  );
  assert_eq!(sum_of(&[minus(1), minus(2)]), minus(3));
  assert_eq!(sum_of::<u7>(&[]), u7::ZERO);
  assert_eq!(sum_of::<i256>(&[]), i256::ZERO);

  assert_eq!(limits_of::<u7>(), [u7::MIN, u7::ZERO, u7::MAX]);
  assert_eq!(limits_of::<Bytes>(), [Bytes::MIN, Bytes::ZERO, Bytes::MAX]);
  assert_eq!(
    limits_of::<Signed>(),
    [Signed::MIN, Signed::ZERO, Signed::MAX]
  );
  assert_eq!(limits_of::<i256>(), [i256::MIN, i256::ZERO, i256::MAX]);
  assert!(is_limit(&u7::MAX) && is_limit(&u7::MIN) && !is_limit(&u7::new(5)));
  assert!(is_limit(&Bytes::MAX) && !is_limit(&bytes(1, 0)));
  assert!(is_limit(&Signed::MIN) && is_limit(&Signed::MAX) && !is_limit(&Signed::ZERO));
  assert!(is_limit(&i256::MIN) && !is_limit(&minus(1)));

  assert_eq!(triple(u7::new(30)), u7::new(90));
  assert_eq!(triple(bytes(200, 1)), bytes(88, 5));
  assert_eq!(triple(Signed::new(-20)), Signed::new(-60));
  assert_eq!(triple(minus(1)), minus(3));

  assert_eq!(product_of(&[u7::new(5), u7::new(25)]), u7::new(125));
  assert_eq!(product_of(&[bytes(10, 0), bytes(20, 0)]), bytes(200, 0));
  assert_eq!(
    product_of(&[Signed::new(-8), Signed::new(-7)]),
    Signed::new(56)
  );
  assert_eq!(product_of(&[minus(1), minus(1)]), i256::ONE);
  assert_eq!(product_of::<u7>(&[]), u7::ONE);
  assert_eq!(product_of::<i256>(&[]), i256::ONE);
  assert_eq!(successor(u7::new(126)), u7::MAX);
  assert_eq!(successor(bytes(255, 0)), bytes(0, 1));
  assert_eq!(successor(Signed::new(-1)), Signed::ZERO);
  assert_eq!(successor(minus(1)), i256::ZERO);
}

/// The non-zero traits need only the non-zero storage trait, `NonZeroUintRepr` or
/// `NonZeroIntRepr`, with the width relation of its integer's storage, and code bounded by the
/// storage trait of the integer reaches them through `NonZeroUint<B, R::NonZero>`: in a native
/// integer and in an array, they give the inherent results. The limits need a width of one bit or
/// more, `Copy` follows the storage, and `ConstNonZeroLimits` needs a storage whose integer holds
/// the value inline.
#[test]
fn the_nonzero_traits_need_only_the_storage_trait() {
  fn unsigned<B: FitsBits<R::Integer>, R: NonZeroUintRepr>(x: &NonZeroUint<B, R>) -> String {
    format!(
      "{:?}",
      (
        x.leading_zeros(),
        x.checked_mul(x),
        x.ilog10(),
        x.clone().get()
      )
    )
  }

  fn signed<B: FitsBits<R::Integer>, R: NonZeroIntRepr>(x: &NonZeroInt<B, R>) -> String {
    format!(
      "{:?}",
      (
        x.is_negative(),
        x.saturating_abs(),
        x.count_ones(),
        x.clone().cast_unsigned()
      )
    )
  }

  fn by_integer<B: FitsBits<R>, R: UintRepr>(x: &NonZeroUint<B, R::NonZero>) -> u32 {
    x.trailing_zeros()
  }

  fn limits<B: FitsBits<R::Integer> + NonZero, R: NonZeroIntRepr>() -> [NonZeroInt<B, R>; 2] {
    [NonZeroLimits::min_value(), NonZeroLimits::max_value()]
  }

  fn copies<B, R: Copy>(x: NonZeroUint<B, R>) -> [NonZeroUint<B, R>; 2] {
    [x, x]
  }

  fn constants<T: ConstNonZeroLimits>() -> [T; 2] {
    [T::MIN, T::MAX]
  }

  let five = NonZeroUint::<U7>::new(u7::new(5)).expect("not zero");
  assert_eq!(unsigned(&five), "(4, Some(25), 0, 5)");
  let wide = NonZeroUint::<U256>::MAX;
  assert_eq!(
    unsigned(&wide),
    format!(
      "{:?}",
      (
        wide.leading_zeros(),
        wide.checked_mul(&wide),
        wide.ilog10(),
        wide.get()
      )
    )
  );
  let minus_eight = NonZeroInt::<U7>::new(i7::new(-8)).expect("not zero");
  assert_eq!(signed(&minus_eight), "(true, 8, 4, 120)");
  let min = NonZeroInt::<U200, [u32; 7]>::MIN;
  assert_eq!(
    signed(&min),
    format!(
      "{:?}",
      (
        min.is_negative(),
        min.saturating_abs(),
        min.count_ones(),
        min.cast_unsigned()
      )
    )
  );
  assert_eq!(by_integer::<U7, u8>(&five), 0);
  let top = NonZeroUint::<U200>::MAX.isolate_highest_one();
  assert_eq!(by_integer::<U200, [u64; 4]>(&top), 199);
  assert_eq!(
    limits::<U7, core::num::NonZero<i8>>(),
    [NonZeroInt::<U7>::MIN, NonZeroInt::<U7>::MAX]
  );
  assert_eq!(limits::<U1, [u8; 1]>(), [NonZeroInt::<U1, [u8; 1]>::MIN; 2]);
  assert_eq!(copies(five), [five; 2]);
  assert_eq!(
    constants::<NonZeroUint<U200>>(),
    [NonZeroUint::<U200>::MIN, NonZeroUint::<U200>::MAX]
  );
  assert_eq!(constants::<NonZeroInt<U1>>(), [NonZeroInt::<U1>::MIN; 2]);
  assert_eq!(<NonZeroUint<U7> as FixedNonZero>::BITS, 7);
  assert_eq!(UnsignedNonZero::cast_signed(five).get(), i7::new(5));
  assert_eq!(SignedNonZero::unsigned_abs(&minus_eight).get(), u7::new(8));
}

/// The forms of a binary operator with an owned operand: two values, a value and a reference, and a
/// reference and a value, or `None` where the checked method of the operation returns `None`.
type Forms<T> = Option<[T; 3]>;

/// The results of the operators with an owned operand: eight binary operators in their [`Forms`],
/// then `N` unary operators and shifts, each `None` where the checked method returns `None`.
type Operators<T, const N: usize> = ([Forms<T>; 8], [Option<T>; N]);

/// The operators with an owned operand of `a` and `b`, which make each operand again rather than
/// clone it: `+`, `-`, `*`, `/` and `%` in the three [`Forms`] where the checked method returns a
/// value, and `&`, `|` and `^` in all three; then `!a`, and `a << n` and `a >> n` with the amount
/// by value and by reference where the checked shift returns a value. The code is bounded by the
/// storage trait alone.
#[inline(never)]
#[allow(clippy::op_ref, reason = "the operators on references are under test")]
fn unsigned_operators<B: FitsBits<R>, R: UintRepr>(
  a: impl Fn() -> Uint<B, R>,
  b: impl Fn() -> Uint<B, R>,
  n: u32,
) -> Operators<Uint<B, R>, 5> {
  (
    [
      a()
        .checked_add(&b())
        .map(|_| [a() + b(), a() + &b(), &a() + b()]),
      a()
        .checked_sub(&b())
        .map(|_| [a() - b(), a() - &b(), &a() - b()]),
      a()
        .checked_mul(&b())
        .map(|_| [a() * b(), a() * &b(), &a() * b()]),
      a()
        .checked_div(&b())
        .map(|_| [a() / b(), a() / &b(), &a() / b()]),
      a()
        .checked_rem(&b())
        .map(|_| [a() % b(), a() % &b(), &a() % b()]),
      Some([a() & b(), a() & &b(), &a() & b()]),
      Some([a() | b(), a() | &b(), &a() | b()]),
      Some([a() ^ b(), a() ^ &b(), &a() ^ b()]),
    ],
    [
      Some(!a()),
      a().checked_shl(n).map(|_| a() << n),
      a().checked_shl(n).map(|_| a() << &n),
      a().checked_shr(n).map(|_| a() >> n),
      a().checked_shr(n).map(|_| a() >> &n),
    ],
  )
}

/// The operators with an owned operand of `a` and `b`, as [`unsigned_operators`] gives them, and
/// `-a` where the checked negation returns a value.
#[inline(never)]
#[allow(clippy::op_ref, reason = "the operators on references are under test")]
fn signed_operators<B: FitsBits<R>, R: IntRepr>(
  a: impl Fn() -> Int<B, R>,
  b: impl Fn() -> Int<B, R>,
  n: u32,
) -> Operators<Int<B, R>, 6> {
  (
    [
      a()
        .checked_add(&b())
        .map(|_| [a() + b(), a() + &b(), &a() + b()]),
      a()
        .checked_sub(&b())
        .map(|_| [a() - b(), a() - &b(), &a() - b()]),
      a()
        .checked_mul(&b())
        .map(|_| [a() * b(), a() * &b(), &a() * b()]),
      a()
        .checked_div(&b())
        .map(|_| [a() / b(), a() / &b(), &a() / b()]),
      a()
        .checked_rem(&b())
        .map(|_| [a() % b(), a() % &b(), &a() % b()]),
      Some([a() & b(), a() & &b(), &a() & b()]),
      Some([a() | b(), a() | &b(), &a() | b()]),
      Some([a() ^ b(), a() ^ &b(), &a() ^ b()]),
    ],
    [
      Some(!a()),
      a().checked_shl(n).map(|_| a() << n),
      a().checked_shl(n).map(|_| a() << &n),
      a().checked_shr(n).map(|_| a() >> n),
      a().checked_shr(n).map(|_| a() >> &n),
      a().checked_neg().map(|_| -a()),
    ],
  )
}

/// `Sum` and `Product` of the two values that `values` makes, of values and of references, where
/// the checked sum and the checked product return a value; the code is bounded by the storage trait
/// alone and a width that holds one.
#[inline(never)]
fn unsigned_folds<B: FitsBits<R> + NonZero, R: UintRepr>(
  values: impl Fn() -> [Uint<B, R>; 2],
) -> [Option<[Uint<B, R>; 2]>; 2] {
  let [a, b] = values();
  [
    a.checked_add(&b)
      .map(|_| [values().into_iter().sum(), values().iter().sum()]),
    a.checked_mul(&b)
      .map(|_| [values().into_iter().product(), values().iter().product()]),
  ]
}

/// `Sum` and `Product` of the two values that `values` makes, as [`unsigned_folds`] gives them.
#[inline(never)]
fn signed_folds<B: FitsBits<R> + IsGreater<U1, Output = True>, R: IntRepr>(
  values: impl Fn() -> [Int<B, R>; 2],
) -> [Option<[Int<B, R>; 2]>; 2] {
  let [a, b] = values();
  [
    a.checked_add(&b)
      .map(|_| [values().into_iter().sum(), values().iter().sum()]),
    a.checked_mul(&b)
      .map(|_| [values().into_iter().product(), values().iter().product()]),
  ]
}

/// The results of the five saturating methods, `None` for a division by zero, and of the four
/// carrying and borrowing methods.
type Saturating<T> = ([Option<T>; 5], [(T, bool); 4]);

/// The saturating methods of `a` and `b` and `UnsignedFixedInteger`, in code bounded by the storage
/// trait alone; the saturating division where `b` is not zero.
#[inline(never)]
fn unsigned_saturating_and_sign<B: FitsBits<R>, R: UintRepr>(
  a: &Uint<B, R>,
  b: &Uint<B, R>,
  exp: u32,
) -> Saturating<Uint<B, R>> {
  let divisor = *b != Uint::<B, R>::default();
  (
    [
      Some(a.saturating_add(b)),
      Some(a.saturating_sub(b)),
      Some(a.saturating_mul(b)),
      divisor.then(|| a.saturating_div(b)),
      Some(a.saturating_pow(exp)),
    ],
    [
      a.carrying_add(b, false),
      a.carrying_add(b, true),
      a.borrowing_sub(b, false),
      a.borrowing_sub(b, true),
    ],
  )
}

/// The saturating methods of `a` and `b`, as [`unsigned_saturating_and_sign`] gives them, in code
/// bounded by the storage trait alone.
#[inline(never)]
fn signed_saturating<B: FitsBits<R>, R: IntRepr>(
  a: &Int<B, R>,
  b: &Int<B, R>,
  exp: u32,
) -> [Option<Int<B, R>>; 5] {
  let divisor = *b != Int::<B, R>::default();
  [
    Some(a.saturating_add(b)),
    Some(a.saturating_sub(b)),
    Some(a.saturating_mul(b)),
    divisor.then(|| a.saturating_div(b)),
    Some(a.saturating_pow(exp)),
  ]
}

/// The results of the methods of `SignedFixedInteger` of `a` that do not panic, in code bounded by
/// the storage trait and `Clone`.
type SignResults<B, R> = (
  [bool; 2],
  [Option<Int<B, R>>; 4],
  (Int<B, R>, bool),
  Uint<B, <R as IntRepr>::Unsigned>,
);

/// The sign queries, the saturating negation, the absolute value in the forms that do not panic,
/// `signum` and `unsigned_abs` of `a`, through `SignedFixedInteger`.
#[inline(never)]
fn signed_sign<B: FitsBits<R>, R: IntRepr + Clone>(a: &Int<B, R>) -> SignResults<B, R> {
  (
    [a.is_negative(), a.is_positive()],
    [
      Some(a.saturating_neg()),
      a.checked_abs(),
      Some(a.wrapping_abs()),
      Some(a.saturating_abs()),
    ],
    (a.signum(), a.overflowing_abs().1),
    a.unsigned_abs(),
  )
}

/// Zero, the limits and one through `FixedInteger` and `IntegerOne`, in code bounded by the storage
/// trait and `Clone`, with a width that holds one.
#[inline(never)]
fn unsigned_general<B: FitsBits<R> + NonZero, R: UintRepr + Clone>() -> [Uint<B, R>; 4] {
  [
    FixedInteger::zero(),
    FixedInteger::min_value(),
    FixedInteger::max_value(),
    IntegerOne::one(),
  ]
}

/// Zero, the limits and one through `FixedInteger` and `IntegerOne`, as [`unsigned_general`] gives
/// them.
#[inline(never)]
fn signed_general<B, R>() -> [Int<B, R>; 4]
where
  B: FitsBits<R> + IsGreater<U1, Output = True>,
  R: IntRepr + Clone,
{
  [
    FixedInteger::zero(),
    FixedInteger::min_value(),
    FixedInteger::max_value(),
    IntegerOne::one(),
  ]
}

/// The ten compound assignments of a clone of `a` by `b`, and by `n` for the shifts, where the
/// checked method of the operation returns a value, in code bounded by `IntegerAssign` and the
/// checked family alone.
#[inline(never)]
fn assignments<T: IntegerAssign + CheckedOps>(a: &T, b: &T, n: u32) -> [Option<T>; 10] {
  let assigned = |assign: fn(&mut T, T)| {
    let mut target = a.clone();
    assign(&mut target, b.clone());
    target
  };
  let shifted = |assign: fn(&mut T, u32)| {
    let mut target = a.clone();
    assign(&mut target, n);
    target
  };
  [
    a.checked_add(b).map(|_| assigned(|x, y| *x += y)),
    a.checked_sub(b).map(|_| assigned(|x, y| *x -= y)),
    a.checked_mul(b).map(|_| assigned(|x, y| *x *= y)),
    a.checked_div(b).map(|_| assigned(|x, y| *x /= y)),
    a.checked_rem(b).map(|_| assigned(|x, y| *x %= y)),
    Some(assigned(|x, y| *x &= y)),
    Some(assigned(|x, y| *x |= y)),
    Some(assigned(|x, y| *x ^= y)),
    a.checked_shl(n).map(|_| shifted(|x, n| *x <<= n)),
    a.checked_shr(n).map(|_| shifted(|x, n| *x >>= n)),
  ]
}

/// The amount of a shift of `bits` bits from the pair at indexes `i` and `j`: one of 0, half the
/// width, the width less one and the width, which overflows.
fn amount(bits: u32, i: usize, j: usize) -> u32 {
  [0, bits / 2, bits.saturating_sub(1), bits][(i + j) % 4]
}

/// Checks the general layer of `Uint` over `$ty`, a storage of `$bits` bits, in a function of its
/// own, at every pair of zero, one, the largest value and the largest value of half the width:
/// the operators with an owned operand, the folds, the saturating methods, `UnsignedFixedInteger`,
/// the functions of `FixedInteger` and `IntegerOne` and the compound assignments, against the
/// inherent `const fn`s and constants of the storage.
macro_rules! unsigned_general_layer {
  ($($ty:ty),+ $(,)?) => {$({
    #[inline(never)]
    fn storage() {
      type T = $ty;
      let values = [T::ZERO, T::ONE, T::MAX, T::MAX.unbounded_shr(T::BITS / 2)];
      let general: [T; 4] = unsigned_general();
      assert_eq!(general, [T::ZERO, T::MIN, T::MAX, T::ONE], "{}", stringify!($ty));
      for (i, &a) in values.iter().enumerate() {
        for (j, &b) in values.iter().enumerate() {
          let n = amount(T::BITS, i, j);
          let forms = |checked: Option<T>| checked.map(|value| [value; 3]);
          let operators = (
            [
              forms(a.checked_add(&b)),
              forms(a.checked_sub(&b)),
              forms(a.checked_mul(&b)),
              forms(a.checked_div(&b)),
              forms(a.checked_rem(&b)),
              Some([a.bitand(b); 3]),
              Some([a.bitor(b); 3]),
              Some([a.bitxor(b); 3]),
            ],
            [
              Some(a.not()),
              a.checked_shl(n),
              a.checked_shl(n),
              a.checked_shr(n),
              a.checked_shr(n),
            ],
          );
          assert_eq!(unsigned_operators(|| a, || b, n), operators, "{a} and {b} by {n}");
          let folds = [
            a.checked_add(&b).map(|sum| [sum; 2]),
            a.checked_mul(&b).map(|product| [product; 2]),
          ];
          assert_eq!(unsigned_folds(|| [a, b]), folds, "{a} and {b}");
          let saturating = (
            [
              Some(a.saturating_add(&b)),
              Some(a.saturating_sub(&b)),
              Some(a.saturating_mul(&b)),
              (!b.is_zero()).then(|| a.saturating_div(&b)),
              Some(a.saturating_pow(n)),
            ],
            [
              a.carrying_add(&b, false),
              a.carrying_add(&b, true),
              a.borrowing_sub(&b, false),
              a.borrowing_sub(&b, true),
            ],
          );
          assert_eq!(unsigned_saturating_and_sign(&a, &b, n), saturating, "{a} and {b}");
          let assigned = [
            a.checked_add(&b),
            a.checked_sub(&b),
            a.checked_mul(&b),
            a.checked_div(&b),
            a.checked_rem(&b),
            Some(a.bitand(b)),
            Some(a.bitor(b)),
            Some(a.bitxor(b)),
            a.checked_shl(n),
            a.checked_shr(n),
          ];
          assert_eq!(assignments(&a, &b, n), assigned, "{a} and {b} by {n}");
        }
      }
    }
    storage();
  })+};
}

/// Checks the general layer of `Int` over `$ty` as [`unsigned_general_layer!`] does for `Uint`, at
/// every pair of zero, the limits and the limits of half the width, with the negation and
/// `SignedFixedInteger`, and with the folds and one at two bits or more, where one exists.
macro_rules! signed_general_layer {
  (@one one, $t:ty) => {
    let general: [$t; 4] = signed_general();
    assert_eq!(general, [<$t>::ZERO, <$t>::MIN, <$t>::MAX, <$t>::ONE], "{}", stringify!($t));
  };
  (@one no_one, $t:ty) => {};
  (@folds one, $a:ident, $b:ident) => {
    let folds = [
      $a.checked_add(&$b).map(|sum| [sum; 2]),
      $a.checked_mul(&$b).map(|product| [product; 2]),
    ];
    assert_eq!(signed_folds(|| [$a, $b]), folds, "{} and {}", $a, $b);
  };
  (@folds no_one, $a:ident, $b:ident) => {};
  ($($ty:ty: $one:tt),+ $(,)?) => {$({
    #[inline(never)]
    fn storage() {
      type T = $ty;
      let half = T::BITS / 2;
      let values = [
        T::ZERO,
        T::MIN,
        T::MAX,
        T::MIN.unbounded_shr(half),
        T::MAX.unbounded_shr(half),
      ];
      signed_general_layer!(@one $one, T);
      for (i, &a) in values.iter().enumerate() {
        for (j, &b) in values.iter().enumerate() {
          let n = amount(T::BITS, i, j);
          let forms = |checked: Option<T>| checked.map(|value| [value; 3]);
          let operators = (
            [
              forms(a.checked_add(&b)),
              forms(a.checked_sub(&b)),
              forms(a.checked_mul(&b)),
              forms(a.checked_div(&b)),
              forms(a.checked_rem(&b)),
              Some([a.bitand(b); 3]),
              Some([a.bitor(b); 3]),
              Some([a.bitxor(b); 3]),
            ],
            [
              Some(a.not()),
              a.checked_shl(n),
              a.checked_shl(n),
              a.checked_shr(n),
              a.checked_shr(n),
              a.checked_neg(),
            ],
          );
          assert_eq!(signed_operators(|| a, || b, n), operators, "{a} and {b} by {n}");
          signed_general_layer!(@folds $one, a, b);
          let saturating = [
            Some(a.saturating_add(&b)),
            Some(a.saturating_sub(&b)),
            Some(a.saturating_mul(&b)),
            (!b.is_zero()).then(|| a.saturating_div(&b)),
            Some(a.saturating_pow(n)),
          ];
          assert_eq!(signed_saturating(&a, &b, n), saturating, "{a} and {b}");
          let assigned = [
            a.checked_add(&b),
            a.checked_sub(&b),
            a.checked_mul(&b),
            a.checked_div(&b),
            a.checked_rem(&b),
            Some(a.bitand(b)),
            Some(a.bitor(b)),
            Some(a.bitxor(b)),
            a.checked_shl(n),
            a.checked_shr(n),
          ];
          assert_eq!(assignments(&a, &b, n), assigned, "{a} and {b} by {n}");
        }
        let sign = (
          [a.is_negative(), a.is_positive()],
          [
            Some(a.saturating_neg()),
            a.checked_abs(),
            Some(a.wrapping_abs()),
            Some(a.saturating_abs()),
          ],
          (a.signum(), a.overflowing_abs().1),
          a.unsigned_abs(),
        );
        assert_eq!(signed_sign(&a), sign, "{a}");
      }
    }
    storage();
  })+};
}

/// The general layer, the operators with an owned operand in their three forms, `Sum`, `Product`,
/// the saturating methods, the sign traits, `FixedInteger`, `IntegerOne` and `IntegerAssign`, needs
/// only the storage trait, with `Clone` for the traits that clone and a storage that holds the
/// value in place for the assignments: in the natives and the arrays of the module's type list,
/// code bounded by them gives what the inherent `const fn`s and constants give. Under Miri, in one
/// native integer and one array of each sign.
#[test]
fn the_general_layer_needs_only_the_storage_trait() {
  unsigned_general_layer!(Uint<U7, u8>, Uint<U200, [u64; 4]>);
  signed_general_layer!(Int<U7, i8>: one, Int<U24, [u8; 3]>: one);
  if tier().is_miri() {
    return;
  }
  unsigned_general_layer!(
    Uint<U1, u8>,
    Uint<U8, u8>,
    Uint<U9, u16>,
    Uint<U24, u32>,
    Uint<U64, u64>,
    Uint<U100, u128>,
    Uint<U7, u128>,
    Uint<U7, [u8; 1]>,
    Uint<U24, [u8; 3]>,
    Uint<U100, [u16; 7]>,
    Uint<U100, [u32; 4]>,
    Uint<U256, [u128; 2]>,
    u256,
  );
  signed_general_layer!(
    Int<U1, i8>: no_one,
    Int<U8, i8>: one,
    Int<U9, i16>: one,
    Int<U24, i32>: one,
    Int<U65, i128>: one,
    Int<U127, i128>: one,
    Int<U1, i64>: no_one,
    Int<U1, [u8; 1]>: no_one,
    Int<U100, [u16; 7]>: one,
    Int<U100, [u32; 4]>: one,
    Int<U200, [u64; 4]>: one,
    Int<U129, [u128; 2]>: one,
    i256: one,
  );
}
