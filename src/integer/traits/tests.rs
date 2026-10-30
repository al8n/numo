use std::string::ToString;

use typenum::{
  IsGreater, NonZero, True, U1, U2, U7, U8, U9, U12, U24, U64, U65, U100, U127, U128, U129, U200,
  U256, U1024,
};

use super::{
  ArrayInteger, ConstFixedInteger, ConstIntegerOne, FixedInteger, FromLimbs, IntegerOne,
  NativeInteger, SignedFixedInteger, UnsignedFixedInteger,
};
use crate::{BitWidth, Int, Uint, bigint, i256, u7, u256};

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
        assert_eq!(ArrayInteger::to_limbs(integer), integer.to_limbs());
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
