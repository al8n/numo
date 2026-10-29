use std::string::ToString;

use typenum::{U1, U7, U8, U9, U12, U24, U64, U65, U100, U127, U128, U129, U200, U256, U1024};

use super::{
  ArrayInteger, FixedInteger, FromLimbs, NativeInteger, SignedFixedInteger, UnsignedFixedInteger,
};
use crate::{BitWidth, Int, Uint, bigint, i256, u256};

/// Asserts that the constants and the queries of [`FixedInteger`] are the inherent ones for each
/// type, at zero and the limits.
macro_rules! assert_fixed {
  ($($ty:ty),+ $(,)?) => {$({
    type T = $ty;
    assert_eq!(<T as FixedInteger>::BITS, T::BITS);
    assert_eq!(<T as FixedInteger>::ZERO, T::ZERO);
    assert_eq!(<T as FixedInteger>::MIN, T::MIN);
    assert_eq!(<T as FixedInteger>::MAX, T::MAX);
    for value in [T::ZERO, T::MIN, T::MAX] {
      assert_eq!(FixedInteger::is_zero(&value), value.is_zero(), "{value}");
    }
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

/// Uses only what the bound `B: BitWidth` gives `Uint<B>` and `Int<B>`: the constants, the traits
/// and the implementations of the default storages.
fn check_width<B: BitWidth>(bits: u32) {
  assert_eq!(Uint::<B>::BITS, bits);
  assert_eq!(Int::<B>::BITS, bits);
  assert_eq!(<Uint<B> as FixedInteger>::BITS, bits);
  assert_eq!(<Int<B> as FixedInteger>::BITS, bits);
  assert_eq!(Uint::<B>::MIN, Uint::<B>::ZERO);
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
  assert_eq!(unsigned, <Uint<B> as FixedInteger>::MAX);
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
  fn unsigned<T: FixedInteger + UnsignedFixedInteger>() {}
  fn signed<T: FixedInteger + SignedFixedInteger<Unsigned = U>, U: UnsignedFixedInteger>() {}
  fn implied<B: BitWidth>() {
    unsigned::<Uint<B>>();
    signed::<Int<B>, Uint<B>>();
  }
  implied::<U1>();
  implied::<U129>();
}
