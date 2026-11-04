//! The non-zero integers `NonZeroUint` and `NonZeroInt`: their layout and the niche of a native
//! storage, the constructors `new` and `new_unchecked`, the value and the constants, also in
//! constant contexts, the standard traits against the integer of the same value, and the table of
//! zero bits and one bit; and their rows, through the inherent `const fn`s and the traits, against
//! the rows of the integer up to 8 bits and at wider widths, against the native non-zero integers
//! at 8 and 16 bits, in constant contexts and in code over the width, with the overflow of `abs`;
//! and their operators, `/` and `%` of `Uint` by `NonZeroUint`, `|` and the negation, against the
//! methods and the native ones.

use core::{
  cmp::Ordering,
  hash::{Hash, Hasher},
  num::NonZero,
};
use std::hash::DefaultHasher;

use typenum::{U3, U4, U5, U6};

use super::*;
use crate::{
  ConstNonZeroLimits, FixedNonZero, NonZeroInt, NonZeroLimits, NonZeroOf, NonZeroUint,
  SignedNonZero, UnsignedNonZero,
};

/// The size and the alignment of `T` and of `Option<T>`.
fn layout<T>() -> [usize; 4] {
  [
    size_of::<T>(),
    align_of::<T>(),
    size_of::<Option<T>>(),
    align_of::<Option<T>>(),
  ]
}

/// The layout of the non-zero integers and of their `Option`: in a native storage, that of the
/// native integer, also for `Option`, whose `None` takes the niche of zero, at a partial and the
/// full width of each native integer, of both signs; in an array, that of the array, and an
/// `Option` that is larger; and no space at zero bits, in the default storage.
#[test]
fn nonzero_layout() {
  macro_rules! native {
    ($($partial:ident, $full:ident: $uint:ident, $int:ident);+ $(;)?) => {$({
      let native = [size_of::<$uint>(), align_of::<$uint>(), size_of::<$uint>(), align_of::<$uint>()];
      assert_eq!(layout::<NonZeroUint<$partial, NonZero<$uint>>>(), native);
      assert_eq!(layout::<NonZeroUint<$full, NonZero<$uint>>>(), native);
      assert_eq!(layout::<NonZeroInt<$partial, NonZero<$int>>>(), native);
      assert_eq!(layout::<NonZeroInt<$full, NonZero<$int>>>(), native);
    })+};
  }
  native!(U7, U8: u8, i8; U9, U16: u16, i16; U24, U32: u32, i32; U63, U64: u64, i64);
  native!(U100, U128: u128, i128);
  // The default storages up to 128 bits are the native ones.
  assert_eq!(layout::<NonZeroUint<U7>>(), [1, 1, 1, 1]);
  assert_eq!(layout::<NonZeroInt<U100>>(), layout::<NonZero<i128>>());

  let bytes = layout::<NonZeroUint<U24, [u8; 3]>>();
  assert_eq!(bytes[..2], [3, 1]);
  assert!(bytes[2] > 3, "an array has no niche");
  let wide = layout::<NonZeroInt<U200>>();
  assert_eq!(wide[..2], [32, align_of::<u64>()]);
  assert!(wide[2] > 32, "an array has no niche");
  assert_eq!(
    layout::<NonZeroUint<U129, [u128; 2]>>()[..2],
    layout::<[u128; 2]>()[..2]
  );

  assert_eq!(size_of::<NonZeroUint<U0>>(), 0);
  assert_eq!(size_of::<NonZeroInt<U0>>(), 0);
}

/// `new`, `get` and the constants in constant contexts, in a native and an array storage, at zero
/// bits, where `new` returns `None` and the type has no limits, at one bit, at 7 bits and at 200
/// bits; the constants infer their storage from the expected type.
#[test]
fn construction_and_constants_in_constant_contexts() {
  type U7x8 = NonZeroUint<U7, [u8; 1]>;
  type I1x16 = NonZeroInt<U1, [u16; 1]>;
  const SMALL: NonZeroUint<U7> = NonZeroUint::<U7>::new(u7::new(100)).unwrap();
  const VALUE: u7 = SMALL.get();
  const ZERO: Option<NonZeroUint<U7>> = NonZeroUint::<U7>::new(u7::ZERO);
  const LIMITS: [NonZeroUint<U7>; 2] = [NonZeroUint::MIN, NonZeroUint::MAX];
  const ARRAY: [Uint<U7, [u8; 1]>; 2] = [U7x8::MIN.get(), U7x8::MAX.get()];
  const SIGNED: [i7; 2] = [NonZeroInt::<U7>::MIN.get(), NonZeroInt::<U7>::MAX.get()];
  const UNSIGNED_BIT: [Uint<U1>; 2] = [NonZeroUint::<U1>::MIN.get(), NonZeroUint::<U1>::MAX.get()];
  const SIGNED_BIT: [Int<U1, [u16; 1]>; 2] = [I1x16::MIN.get(), I1x16::MAX.get()];
  const WIDE: [Uint<U200>; 2] = [
    NonZeroUint::<U200>::MIN.get(),
    NonZeroUint::<U200>::MAX.get(),
  ];
  const WIDE_SIGNED: Int<U200> = NonZeroInt::<U200>::new(Int::<U200>::MIN).unwrap().get();
  type U0x0 = NonZeroUint<U0>;
  type I0x8 = NonZeroInt<U0, NonZero<i8>>;
  const ZERO_BITS: (Option<U0x0>, Option<I0x8>) =
    (U0x0::new(Uint::<U0>::ZERO), I0x8::new(Int::<U0, i8>::ZERO));
  const BITS: [u32; 4] = [
    NonZeroUint::<U0>::BITS,
    NonZeroInt::<U1>::BITS,
    U7x8::BITS,
    NonZeroInt::<U200>::BITS,
  ];

  assert_eq!(VALUE, u7::new(100));
  assert_eq!(ZERO, None);
  assert_eq!(LIMITS.map(NonZeroUint::<U7>::get), [u7::ONE, u7::MAX]);
  assert_eq!(ARRAY, [Uint::<U7, [u8; 1]>::ONE, Uint::<U7, [u8; 1]>::MAX]);
  assert_eq!(SIGNED, [i7::MIN, i7::MAX]);
  assert_eq!(UNSIGNED_BIT, [Uint::<U1>::ONE; 2]);
  assert_eq!(SIGNED_BIT, [Int::<U1, [u16; 1]>::MIN; 2]);
  assert_eq!(WIDE, [Uint::<U200>::ONE, Uint::<U200>::MAX]);
  assert_eq!(WIDE_SIGNED, Int::<U200>::MIN);
  assert_eq!(ZERO_BITS, (None, None));
  assert_eq!(BITS, [0, 1, 7, 200]);

  // The expected type names the storage of a constant.
  let max: NonZeroUint<U7> = NonZeroUint::MAX;
  let min: NonZeroInt<U200> = NonZeroInt::MIN;
  assert_eq!((max.get(), min.get()), (u7::MAX, Int::<U200>::MIN));
}

/// The hash that `value` writes.
fn hash_of(value: &impl Hash) -> u64 {
  let mut hasher = DefaultHasher::new();
  value.hash(&mut hasher);
  hasher.finish()
}

/// Checks the standard traits of the non-zero integers of `$ty`, whose integer is `$integer`, at
/// the values `$value`: `Ord`, `PartialEq`, `Hash`, `Display` with its flags, `Debug`, `From` and
/// `Clone` give what they give for the integer of the same value.
macro_rules! traits_of_the_integer {
  ($ty:ty, $integer:ty; $($value:expr),+ $(,)?) => {{
    let values: Vec<$ty> = vec![$(<$ty>::new($value).expect("not zero")),+];
    for x in &values {
      let integer: $integer = x.get();
      assert_eq!(hash_of(x), hash_of(&integer), "{x}");
      assert_eq!(x.to_string(), integer.to_string());
      assert_eq!(format!("{x:+>8} {x:?}"), format!("{integer:+>8} {integer:?}"));
      assert_eq!(<$integer>::from(x.clone()), integer);
      for y in &values {
        assert_eq!(x.cmp(y), integer.cmp(&y.get()), "{x} and {y}");
        assert_eq!(x.partial_cmp(y), integer.partial_cmp(&y.get()), "{x} and {y}");
        assert_eq!(x == y, integer == y.get(), "{x} and {y}");
      }
    }
  }};
}

/// The standard traits of the non-zero integers are those of the integer of the same value, in
/// native integers and arrays, at a partial and a full width; the hash of a value is the same in
/// every storage, as for the integers.
#[test]
fn standard_traits_are_those_of_the_integer() {
  traits_of_the_integer!(NonZeroUint<U7>, u7; u7::ONE, u7::new(64), u7::MAX);
  traits_of_the_integer!(NonZeroInt<U7>, i7; i7::MIN, i7::new(-1), i7::ONE, i7::MAX);
  type U7x8 = Uint<U7, [u8; 1]>;
  traits_of_the_integer!(NonZeroUint<U7, [u8; 1]>, U7x8; U7x8::ONE, U7x8::MAX);
  type I7x64 = Int<U7, [u64; 1]>;
  traits_of_the_integer!(NonZeroInt<U7, [u64; 1]>, I7x64; I7x64::MIN, -I7x64::ONE, I7x64::MAX);
  traits_of_the_integer!(NonZeroUint<U8, NonZero<u8>>, Uint<U8, u8>; Uint::ONE, Uint::MAX);
  traits_of_the_integer!(NonZeroUint<U200>, Uint<U200>; Uint::ONE, Uint::MAX);
  traits_of_the_integer!(NonZeroInt<U256>, i256; i256::MIN, -i256::ONE, i256::ONE, i256::MAX);

  // The same value hashes alike in every storage.
  let native = NonZeroInt::<U7>::new(i7::new(-5)).expect("not zero");
  let limbs = Int::<U7, [u16; 1]>::checked_from_limbs(limbs_of(-5)).expect("7 bits");
  let array = NonZeroInt::<U7, [u16; 1]>::new(limbs).expect("not zero");
  assert_eq!(native.to_string(), "-5");
  assert_eq!(array.to_string(), "-5");
  assert_eq!(hash_of(&native), hash_of(&array));
  assert_eq!(hash_of(&native), hash_of(&i7::new(-5)));
}

/// Construction and the limits of zero bits and one bit, in a native and an array storage: zero bits
/// hold no value but zero, so `new` returns `None` there in every storage; one unsigned bit holds
/// 1 and one signed bit -1, each both `MIN` and `MAX`.
#[test]
fn zero_bits_and_one_bit_construction() {
  assert_eq!(NonZeroUint::<U0, NonZero<u8>>::new(Uint::ZERO), None);
  assert_eq!(NonZeroUint::<U0, [u64; 1]>::new(Uint::ZERO), None);
  assert_eq!(NonZeroInt::<U0, NonZero<i8>>::new(Int::ZERO), None);
  assert_eq!(NonZeroInt::<U0, [u8; 0]>::new(Int::ZERO), None);

  type U1x8 = NonZeroUint<U1, NonZero<u8>>;
  type U1x1 = NonZeroUint<U1, [u8; 1]>;
  type I1x8 = NonZeroInt<U1, NonZero<i8>>;
  type I1x1 = NonZeroInt<U1, [u16; 1]>;
  assert_eq!(U1x8::new(Uint::ZERO), None);
  assert_eq!(U1x8::new(Uint::ONE), Some(U1x8::MIN));
  assert_eq!((U1x8::MIN, U1x8::MAX.get()), (U1x8::MAX, Uint::ONE));
  assert_eq!(U1x1::new(Uint::ONE), Some(U1x1::MAX));
  assert_eq!((U1x1::MIN, U1x1::MAX.get()), (U1x1::MAX, Uint::ONE));
  assert_eq!(I1x8::new(Int::ZERO), None);
  assert_eq!(I1x8::new(Int::MIN), Some(I1x8::MAX));
  assert_eq!((I1x8::MIN, I1x8::MAX.get()), (I1x8::MAX, Int::MIN));
  assert_eq!(I1x1::new(Int::MIN), Some(I1x1::MIN));
  assert_eq!((I1x1::MIN, I1x1::MAX.get()), (I1x1::MAX, Int::MIN));
}

/// `new_unchecked` of a value that is not zero gives the value that `new` gives, through the
/// inherent `const unsafe fn` of a native and an array storage of each sign, also in constant
/// contexts, and through the trait: at one bit, at 7 bits, at the full width of a native integer
/// and at 200 bits, at one, -1 and the limits. Through the trait, `new_unchecked` of zero panics in
/// every build and reports the caller, at zero bits too.
#[test]
fn new_unchecked_of_values_that_are_not_zero() {
  // Checks `new_unchecked` of the values `$value` of the integer `$integer` of `$ty`, which are
  // not zero, against `new`, through the inherent method and the trait.
  macro_rules! unchecked {
    ($ty:ty, $integer:ty; $($value:expr),+ $(,)?) => {$({
      let value: $integer = $value;
      let expected = <$ty>::new(value).expect("not zero");
      // SAFETY: `new` returns `Some`, so `value` is not zero.
      let inherent = unsafe { <$ty>::new_unchecked(value) };
      // SAFETY: as above.
      let generic = unsafe { <$ty as FixedNonZero>::new_unchecked(value) };
      assert_eq!([inherent, generic], [expected; 2], "{}", stringify!($value));
    })+};
  }

  unchecked!(NonZeroUint<U1>, Uint<U1>; Uint::ONE);
  unchecked!(NonZeroUint<U1, [u8; 1]>, Uint<U1, [u8; 1]>; Uint::ONE);
  unchecked!(NonZeroUint<U7>, u7; u7::ONE, u7::new(64), u7::MAX);
  unchecked!(NonZeroUint<U7, [u16; 1]>, Uint<U7, [u16; 1]>; Uint::ONE, Uint::MAX);
  unchecked!(NonZeroUint<U8, NonZero<u8>>, Uint<U8, u8>; Uint::ONE, Uint::MAX);
  unchecked!(NonZeroUint<U128, NonZero<u128>>, Uint<U128, u128>; Uint::ONE, Uint::MAX);
  unchecked!(NonZeroUint<U200>, Uint<U200>; Uint::ONE, Uint::MAX);
  unchecked!(NonZeroInt<U1>, Int<U1>; Int::MIN);
  unchecked!(NonZeroInt<U1, [u8; 1]>, Int<U1, [u8; 1]>; Int::MIN);
  unchecked!(NonZeroInt<U7>, i7; i7::MIN, i7::new(-1), i7::ONE, i7::MAX);
  unchecked!(NonZeroInt<U7, [u64; 1]>, Int<U7, [u64; 1]>; Int::MIN, -Int::ONE, Int::ONE, Int::MAX);
  unchecked!(NonZeroInt<U8, NonZero<i8>>, Int<U8, i8>; Int::MIN, -Int::ONE, Int::ONE, Int::MAX);
  unchecked!(NonZeroInt<U128, NonZero<i128>>, Int<U128, i128>; Int::MIN, -Int::ONE, Int::MAX);
  unchecked!(NonZeroInt<U200>, Int<U200>; Int::MIN, -Int::ONE, Int::ONE, Int::MAX);

  type Bytes = NonZeroUint<U7, [u8; 1]>;
  type Wide = NonZeroInt<U200>;
  // SAFETY: 100 is not zero.
  const SMALL: NonZeroUint<U7> = unsafe { NonZeroUint::<U7>::new_unchecked(u7::new(100)) };
  // SAFETY: one is not zero.
  const LIMBS: Bytes = unsafe { Bytes::new_unchecked(Uint::ONE) };
  // SAFETY: -1 is not zero.
  const SIGNED: NonZeroInt<U7> = unsafe { NonZeroInt::<U7>::new_unchecked(i7::new(-1)) };
  // SAFETY: `MIN` is not zero.
  const WIDE: Wide = unsafe { Wide::new_unchecked(Int::<U200>::MIN) };
  assert_eq!(SMALL.get(), u7::new(100));
  assert_eq!(LIMBS, Bytes::MIN);
  assert_eq!(SIGNED.get(), i7::new(-1));
  assert_eq!(WIDE, Wide::MIN);

  // `new_unchecked` of `value` through the trait, whose panic reports the caller of this function.
  #[track_caller]
  fn through_the_trait<T: FixedNonZero>(value: T::Integer) -> T {
    // SAFETY: the bodies of the trait check the value and panic on zero, so a call with zero, which
    // breaks the contract, has no undefined behavior: the test makes one to check that.
    unsafe { T::new_unchecked(value) }
  }

  let message = "`new_unchecked` requires a value that is not zero".to_string();
  type Byte = NonZeroInt<U8, NonZero<i8>>;
  panics_here!(through_the_trait::<NonZeroUint<U7>>(u7::ZERO), message);
  panics_here!(through_the_trait::<NonZeroUint<U200>>(Uint::ZERO), message);
  panics_here!(through_the_trait::<Byte>(Int::ZERO), message);
  panics_here!(through_the_trait::<NonZeroInt<U200>>(Int::ZERO), message);
  panics_here!(through_the_trait::<NonZeroUint<U0>>(Uint::ZERO), message);
}

/// The exponents at which the powers are checked.
const EXPONENTS: [u32; 7] = [0, 1, 2, 3, 7, 8, u32::MAX];

/// What the rows of `NonZeroUint` return for one value, each non-zero result as the integer of
/// its value, `U`, and the cast as the integer of the other sign, `S`.
#[derive(Debug, PartialEq)]
struct UnsignedRows<U, S> {
  /// `leading_zeros`, `trailing_zeros`, `count_ones`, `highest_one`, `lowest_one`, `ilog2`,
  /// `ilog10` and `bit_width`.
  counts: [u32; 8],
  /// `is_power_of_two`.
  power_of_two: bool,
  /// `isolate_highest_one`, `isolate_lowest_one`, `reverse_bits` and `isqrt`.
  values: [U; 4],
  /// `checked_next_power_of_two`.
  next_power_of_two: Option<U>,
  /// `rotate_left` and `rotate_right` at each amount.
  rotated: Vec<[U; 2]>,
  /// `checked_pow` and `saturating_pow` at each exponent of [`EXPONENTS`].
  powers: Vec<(Option<U>, U)>,
  /// `cast_signed`.
  signed: S,
}

/// What the rows of `NonZeroInt` return for one value, each non-zero result as the integer of its
/// value, `I`, and the unsigned results as the integer of the other sign, `U`.
#[derive(Debug, PartialEq)]
struct SignedRows<I, U> {
  /// `leading_zeros`, `trailing_zeros`, `count_ones`, `highest_one` and `lowest_one`.
  counts: [u32; 5],
  /// `is_negative` and `is_positive`.
  signs: [bool; 2],
  /// `isolate_highest_one`, `isolate_lowest_one` and `reverse_bits`.
  values: [I; 3],
  /// `rotate_left` and `rotate_right` at each amount.
  rotated: Vec<[I; 2]>,
  /// `checked_pow` and `saturating_pow` at each exponent of [`EXPONENTS`].
  powers: Vec<(Option<I>, I)>,
  /// `overflowing_neg` and `overflowing_abs`.
  overflowing: [(I, bool); 2],
  /// `checked_neg` and `checked_abs`.
  checked: [Option<I>; 2],
  /// `wrapping_neg`, `saturating_neg`, `wrapping_abs` and `saturating_abs`.
  bounded: [I; 4],
  /// `abs`, where the absolute value is in the range of the type.
  abs: Option<I>,
  /// `unsigned_abs` and `cast_unsigned`.
  unsigned: [U; 2],
}

/// What the rows of `NonZeroUint` with a second operand return: `checked_add` and `saturating_add`
/// of an integer, which may be zero, and where the operand is not zero `checked_mul`,
/// `saturating_mul`, `midpoint`, `div_ceil` and the order.
#[derive(Debug, PartialEq)]
struct UnsignedPair<U> {
  add: (Option<U>, U),
  nonzero: Option<(Option<U>, U, U, U, Ordering)>,
}

/// What the rows of `NonZeroInt` with a second operand return: `checked_mul`, `saturating_mul`
/// and the order.
#[derive(Debug, PartialEq)]
struct SignedPair<I> {
  mul: (Option<I>, I, Ordering),
}

/// The [`UnsignedRows`] of `$x`, a reference to a value, and `$owned`, the value, at the amounts
/// `$amounts`, through the methods that the context reaches: the inherent `const fn`s for a
/// concrete storage, and the traits in generic code.
macro_rules! unsigned_rows {
  ($x:expr, $owned:expr, $amounts:expr) => {{
    let (x, owned, amounts) = ($x, $owned, $amounts);
    UnsignedRows {
      counts: [
        x.leading_zeros(),
        x.trailing_zeros(),
        x.count_ones().get(),
        x.highest_one(),
        x.lowest_one(),
        x.ilog2(),
        x.ilog10(),
        x.bit_width().get(),
      ],
      power_of_two: x.is_power_of_two(),
      values: [
        x.isolate_highest_one().get(),
        x.isolate_lowest_one().get(),
        x.reverse_bits().get(),
        x.isqrt().get(),
      ],
      next_power_of_two: x.checked_next_power_of_two().map(|power| power.get()),
      rotated: amounts
        .iter()
        .map(|&n| [x.rotate_left(n).get(), x.rotate_right(n).get()])
        .collect(),
      powers: EXPONENTS
        .iter()
        .map(|&exp| {
          let power = x.checked_pow(exp).map(|power| power.get());
          (power, x.saturating_pow(exp).get())
        })
        .collect(),
      signed: owned.cast_signed().get(),
    }
  }};
}

/// The [`SignedRows`] of `$x`, a reference to a value, and `$owned`, the value, at the amounts
/// `$amounts`, through the methods that the context reaches, as [`unsigned_rows`] does.
macro_rules! signed_rows {
  ($x:expr, $owned:expr, $amounts:expr) => {{
    let (x, owned, amounts) = ($x, $owned, $amounts);
    let (negated, negation_overflows) = x.overflowing_neg();
    let (abs, abs_overflows) = x.overflowing_abs();
    SignedRows {
      counts: [
        x.leading_zeros(),
        x.trailing_zeros(),
        x.count_ones().get(),
        x.highest_one(),
        x.lowest_one(),
      ],
      signs: [x.is_negative(), x.is_positive()],
      values: [
        x.isolate_highest_one().get(),
        x.isolate_lowest_one().get(),
        x.reverse_bits().get(),
      ],
      rotated: amounts
        .iter()
        .map(|&n| [x.rotate_left(n).get(), x.rotate_right(n).get()])
        .collect(),
      powers: EXPONENTS
        .iter()
        .map(|&exp| {
          let power = x.checked_pow(exp).map(|power| power.get());
          (power, x.saturating_pow(exp).get())
        })
        .collect(),
      overflowing: [
        (negated.get(), negation_overflows),
        (abs.get(), abs_overflows),
      ],
      checked: [
        x.checked_neg().map(|negated| negated.get()),
        x.checked_abs().map(|abs| abs.get()),
      ],
      bounded: [
        x.wrapping_neg().get(),
        x.saturating_neg().get(),
        x.wrapping_abs().get(),
        x.saturating_abs().get(),
      ],
      abs: x.checked_abs().map(|_| x.abs().get()),
      unsigned: [x.unsigned_abs().get(), owned.cast_unsigned().get()],
    }
  }};
}

/// The [`UnsignedPair`] of `$x` with the integer `$b` and, where it is not zero, the value `$y`,
/// references both, through the methods that the context reaches.
macro_rules! unsigned_pair {
  ($x:expr, $b:expr, $y:expr) => {{
    let (x, b, y) = ($x, $b, $y);
    UnsignedPair {
      add: (
        x.checked_add(b).map(|sum| sum.get()),
        x.saturating_add(b).get(),
      ),
      nonzero: y.map(|y| {
        let product = x.checked_mul(y).map(|product| product.get());
        (
          product,
          x.saturating_mul(y).get(),
          x.midpoint(y).get(),
          x.div_ceil(y).get(),
          x.cmp(y),
        )
      }),
    }
  }};
}

/// The [`SignedPair`] of `$x` and `$y`, references both, through the methods that the context
/// reaches.
macro_rules! signed_pair {
  ($x:expr, $y:expr) => {{
    let (x, y) = ($x, $y);
    let product = x.checked_mul(y).map(|product| product.get());
    SignedPair {
      mul: (product, x.saturating_mul(y).get(), x.cmp(y)),
    }
  }};
}

/// The [`UnsignedRows`] of `x` through the traits, which need no `Copy`.
fn unsigned_rows_through_traits<T>(
  x: &T,
  amounts: &[u32],
) -> UnsignedRows<T::Integer, <T::Signed as FixedNonZero>::Integer>
where
  T: UnsignedNonZero,
  T::Signed: FixedNonZero,
{
  unsigned_rows!(x, x.clone(), amounts)
}

/// The [`SignedRows`] of `x` through the traits, which need no `Copy`.
fn signed_rows_through_traits<T>(
  x: &T,
  amounts: &[u32],
) -> SignedRows<T::Integer, <T::Unsigned as FixedNonZero>::Integer>
where
  T: SignedNonZero,
  T::Unsigned: FixedNonZero,
{
  signed_rows!(x, x.clone(), amounts)
}

/// The [`UnsignedPair`] of `x` through the traits.
fn unsigned_pair_through_traits<T: UnsignedNonZero>(
  x: &T,
  b: &T::Integer,
  y: Option<&T>,
) -> UnsignedPair<T::Integer> {
  unsigned_pair!(x, b, y)
}

/// The [`SignedPair`] of `x` and `y` through the traits.
fn signed_pair_through_traits<T: SignedNonZero>(x: &T, y: &T) -> SignedPair<T::Integer> {
  signed_pair!(x, y)
}

/// The [`UnsignedRows`] of the value `i`, which is not zero, from the rows of the integer: their
/// results, and `MAX` where a saturating row passes it.
fn expected_unsigned_rows<U>(i: &U, amounts: &[u32]) -> UnsignedRows<U, U::Signed>
where
  U: FixedInteger + CheckedOps + IntegerBits + IntegerMath + UnsignedMath + UnsignedBits + Copy,
{
  let max = U::max_value();
  UnsignedRows {
    counts: [
      i.leading_zeros(),
      i.trailing_zeros(),
      i.count_ones(),
      i.highest_one().expect("not zero"),
      i.lowest_one().expect("not zero"),
      i.ilog2(),
      i.ilog10(),
      i.bit_width(),
    ],
    power_of_two: i.is_power_of_two(),
    values: [
      i.isolate_highest_one(),
      i.isolate_lowest_one(),
      i.reverse_bits(),
      i.isqrt(),
    ],
    next_power_of_two: i.checked_next_power_of_two(),
    rotated: amounts
      .iter()
      .map(|&n| [i.rotate_left(n), i.rotate_right(n)])
      .collect(),
    powers: EXPONENTS
      .iter()
      .map(|&exp| {
        let power = i.checked_pow(exp);
        (power, power.unwrap_or(max))
      })
      .collect(),
    signed: i.cast_signed(),
  }
}

/// The bounds of a saturating row of `NonZeroInt` of `I`: `MIN`, and `MAX` from two bits; one bit
/// holds no value but -1 that is not zero, so both bounds are -1 there.
fn signed_bounds<I: FixedInteger>() -> (I, I) {
  let max = if I::BITS > 1 {
    I::max_value()
  } else {
    I::min_value()
  };
  (I::min_value(), max)
}

/// The [`SignedRows`] of the value `i`, which is not zero, from the rows of the integer: their
/// results, and the bounds of [`signed_bounds`] where a saturating row passes them.
fn expected_signed_rows<I>(i: &I, amounts: &[u32]) -> SignedRows<I, I::Magnitude>
where
  I: FixedInteger + SignedFixedInteger<Unsigned = <I as IntegerMath>::Magnitude>,
  I: CheckedOps + OverflowingOps + WrappingOps + IntegerBits + IntegerMath + SignedBits + Copy,
{
  let (min, max) = signed_bounds::<I>();
  let negative = i.is_negative();
  SignedRows {
    counts: [
      i.leading_zeros(),
      i.trailing_zeros(),
      i.count_ones(),
      i.highest_one().expect("not zero"),
      i.lowest_one().expect("not zero"),
    ],
    signs: [negative, !negative],
    values: [
      i.isolate_highest_one(),
      i.isolate_lowest_one(),
      i.reverse_bits(),
    ],
    rotated: amounts
      .iter()
      .map(|&n| [i.rotate_left(n), i.rotate_right(n)])
      .collect(),
    powers: EXPONENTS
      .iter()
      .map(|&exp| {
        let power = i.checked_pow(exp);
        let bound = if negative && exp % 2 == 1 { min } else { max };
        (power, power.unwrap_or(bound))
      })
      .collect(),
    overflowing: [i.overflowing_neg(), i.overflowing_abs()],
    checked: [i.checked_neg(), i.checked_abs()],
    bounded: [
      i.wrapping_neg(),
      i.checked_neg().unwrap_or(max),
      i.wrapping_abs(),
      i.checked_abs().unwrap_or(max),
    ],
    abs: i.checked_abs(),
    unsigned: [i.unsigned_abs(), i.cast_unsigned()],
  }
}

/// The [`UnsignedPair`] of the value `i`, which is not zero, and the integer `b`, from the rows of
/// the integer.
fn expected_unsigned_pair<U>(i: &U, b: &U) -> UnsignedPair<U>
where
  U: FixedInteger + CheckedOps + IntegerMath + Copy,
{
  let max = U::max_value();
  let sum = i.checked_add(b);
  UnsignedPair {
    add: (sum, sum.unwrap_or(max)),
    nonzero: (!b.is_zero()).then(|| {
      let product = i.checked_mul(b);
      (
        product,
        product.unwrap_or(max),
        i.midpoint(b),
        i.div_ceil(b),
        i.cmp(b),
      )
    }),
  }
}

/// The [`SignedPair`] of the values `i` and `j`, which are not zero, from the rows of the integer:
/// an overflowing product passes `MIN` exactly when the signs differ.
fn expected_signed_pair<I>(i: &I, j: &I) -> SignedPair<I>
where
  I: FixedInteger + SignedFixedInteger + CheckedOps + Copy,
{
  let (min, max) = signed_bounds::<I>();
  let product = i.checked_mul(j);
  let bound = if i.is_negative() == j.is_negative() {
    max
  } else {
    min
  };
  SignedPair {
    mul: (product, product.unwrap_or(bound), i.cmp(j)),
  }
}

/// Checks the rows of `$x`, a value of the sign `$sign` that is not zero, `$a` in `$bits` bits,
/// through the inherent `const fn`s and the traits, against the rows of its integer at the amounts
/// `$amounts`.
macro_rules! check_rows {
  (unsigned; $x:expr, $amounts:expr, $a:expr, $bits:expr) => {{
    let (x, amounts) = (&$x, $amounts);
    let expected = expected_unsigned_rows(&x.get(), amounts);
    assert_eq!(
      unsigned_rows!(x, *x, amounts),
      expected,
      "{} in {} bits",
      $a,
      $bits
    );
    assert_eq!(
      unsigned_rows_through_traits(x, amounts),
      expected,
      "{} in {} bits, through the traits",
      $a,
      $bits
    );
  }};
  (signed; $x:expr, $amounts:expr, $a:expr, $bits:expr) => {{
    let (x, amounts) = (&$x, $amounts);
    let expected = expected_signed_rows(&x.get(), amounts);
    assert_eq!(
      signed_rows!(x, *x, amounts),
      expected,
      "{} in {} bits",
      $a,
      $bits
    );
    assert_eq!(
      signed_rows_through_traits(x, amounts),
      expected,
      "{} in {} bits, through the traits",
      $a,
      $bits
    );
  }};
}

/// Checks the rows of `$x`, a value of the sign `$sign` that is not zero, with the integer `$b`
/// and, where it is not zero, the value `$y`, the pair `($a, $bv)` in `$bits` bits, through the
/// inherent `const fn`s and the traits, against the rows of the integers.
macro_rules! check_pair {
  (unsigned; $x:expr, $b:expr, $y:expr, $a:expr, $bv:expr, $bits:expr) => {{
    let (x, b, y) = (&$x, &$b, $y);
    let expected = expected_unsigned_pair(&x.get(), b);
    assert_eq!(
      unsigned_pair!(x, b, y.as_ref()),
      expected,
      "{} and {} in {} bits",
      $a,
      $bv,
      $bits
    );
    assert_eq!(
      unsigned_pair_through_traits(x, b, y.as_ref()),
      expected,
      "{} and {} in {} bits, through the traits",
      $a,
      $bv,
      $bits
    );
  }};
  (signed; $x:expr, $b:expr, $y:expr, $a:expr, $bv:expr, $bits:expr) => {{
    if let Some(y) = $y {
      let x = &$x;
      let expected = expected_signed_pair(&x.get(), &y.get());
      assert_eq!(
        signed_pair!(x, &y),
        expected,
        "{} and {} in {} bits",
        $a,
        $bv,
        $bits
      );
      assert_eq!(
        signed_pair_through_traits(x, &y),
        expected,
        "{} and {} in {} bits, through the traits",
        $a,
        $bv,
        $bits
      );
    }
  }};
}

/// Checks the rows of the non-zero integer of `$ty`, the integer of the sign `$sign` and of `$bits`
/// bits, whose values `$from` creates from an `i128`, at the values and the pairs of `$samples`
/// whose first value is not zero, as [`sweep_storage`] passes them, through the inherent
/// `const fn`s and the traits, against the rows of the integer.
macro_rules! sweep_nonzero {
  ($sign:ident; $ty:ty, $bits:expr, $samples:expr, $from:expr, $value:expr, $magnitude:expr) => {{
    let from: fn(i128) -> $ty = $from;
    let bits: u32 = $bits;
    let (values, pairs): (Vec<i128>, Vec<(i128, i128)>) = $samples;
    let amounts = shift_amounts(bits);
    let nonzero = |a: i128| NonZeroOf::<$ty>::new(from(a)).expect("not zero");
    for a in values.into_iter().filter(|&a| a != 0) {
      check_rows!($sign; nonzero(a), &amounts, a, bits);
    }
    for (a, b) in pairs.into_iter().filter(|&(a, _)| a != 0) {
      check_pair!($sign; nonzero(a), from(b), (b != 0).then(|| nonzero(b)), a, b, bits);
    }
  }};
}

/// The values and the pairs of values of up to 8 bits at which the rows of a storage are checked:
/// those of [`small_values`] and [`small_pairs`], or under Miri the [`ends`] and their pairs, in a
/// storage that Miri checks, `miri`.
fn nonzero_samples(
  bits: u32,
  signed: bool,
  primary: bool,
  miri: bool,
) -> (Vec<i128>, Vec<(i128, i128)>) {
  if tier().is_miri() {
    if !miri {
      return (Vec::new(), Vec::new());
    }
    let ends = ends(bits, signed);
    let pairs = ends
      .iter()
      .flat_map(|&a| ends.iter().map(move |&b| (a, b)))
      .collect();
    (ends, pairs)
  } else {
    (
      small_values(bits, signed, primary),
      small_pairs(bits, signed, primary),
    )
  }
}

/// Checks the rows at the width `B`, `bits` bits, at most 8: in `u8` and `i8` and in `[u8; 1]`, at
/// the samples of [`nonzero_samples`] for a storage that every tier checks, in `[u8; 2]` and
/// `[u16; 1]` at those of a storage that every tier but Miri checks, and in `u16` and `i16`,
/// `u128` and `i128`, `[u64; 1]` and `[u128; 1]` at those of another.
fn nonzero_of_small_widths<B>(bits: u32)
where
  B: FitsBits<u8> + FitsBits<i8> + FitsBits<u16> + FitsBits<i16> + FitsBits<u128> + FitsBits<i128>,
  B: FitsBits<[u8; 1]> + FitsBits<[u8; 2]> + FitsBits<[u16; 1]>,
  B: FitsBits<[u64; 1]> + FitsBits<[u128; 1]>,
{
  let everywhere = |bits, signed| nonzero_samples(bits, signed, true, true);
  let primary = |bits, signed| nonzero_samples(bits, signed, true, false);
  let secondary = |bits, signed| nonzero_samples(bits, signed, false, false);
  sweep_storage!(sweep_nonzero, everywhere, native B, bits, u8, i8);
  sweep_storage!(sweep_nonzero, everywhere, array B, bits, [u8; 1]);
  sweep_storage!(sweep_nonzero, primary, array B, bits, [u8; 2]);
  sweep_storage!(sweep_nonzero, primary, array B, bits, [u16; 1]);
  sweep_storage!(sweep_nonzero, secondary, native B, bits, u16, i16);
  sweep_storage!(sweep_nonzero, secondary, native B, bits, u128, i128);
  sweep_storage!(sweep_nonzero, secondary, array B, bits, [u64; 1]);
  sweep_storage!(sweep_nonzero, secondary, array B, bits, [u128; 1]);
}

/// Every value and pair that is not zero of each width up to 8 bits, or those that the tier takes,
/// in native integers and in arrays of one and two limbs: every row through the inherent
/// `const fn`s and the traits against the rows of the integer, and the order against that of the
/// integers. Zero bits hold no value that is not zero, and one signed bit only -1. Miri checks 1,
/// 2, 7 and 8 bits.
#[test]
fn nonzero_methods_of_up_to_8_bits() {
  nonzero_of_small_widths::<U0>(0);
  nonzero_of_small_widths::<U1>(1);
  nonzero_of_small_widths::<U2>(2);
  nonzero_of_small_widths::<U7>(7);
  nonzero_of_small_widths::<U8>(8);
  if !tier().is_miri() {
    nonzero_of_small_widths::<U3>(3);
    nonzero_of_small_widths::<U4>(4);
    nonzero_of_small_widths::<U5>(5);
    nonzero_of_small_widths::<U6>(6);
  }
}

/// Checks the non-zero integers `$ty`, of the width of the native unsigned `$native`, against
/// `NonZero<$native>`, through the inherent `const fn`s and the traits: every row that the native
/// one has, the bit reversal and the rotations, which are unstable there, against those of
/// `$native`, the order, the hash and the text, at each value of `$values` and each pair of
/// `$pairs` that is not zero, and the addition also of zero. `$integer` creates the integer of a
/// native value, `$value` reads an integer back, and `$cast` reads the integer of the other sign.
macro_rules! unsigned_against_std {
  (
    $ty:ty, $native:ident;
    $integer:expr, $value:expr, $cast:expr;
    $values:expr, $pairs:expr
  ) => {{
    let (integer, value, cast) = ($integer, $value, $cast);
    let read = |x: $ty| value(&x.get());
    let amounts = shift_amounts($native::BITS);
    for v in $values
      .into_iter()
      .map(|v| v as $native)
      .filter(|&v| v != 0)
    {
      let x = <$ty>::new(integer(v)).expect("not zero");
      let n = NonZero::<$native>::new(v).expect("not zero");
      assert_eq!(
        [
          both!(FixedNonZero::leading_zeros(&x)),
          both!(FixedNonZero::trailing_zeros(&x)),
          both!(FixedNonZero::count_ones(&x)).get(),
          both!(FixedNonZero::highest_one(&x)),
          both!(FixedNonZero::lowest_one(&x)),
          both!(UnsignedNonZero::ilog2(&x)),
          both!(UnsignedNonZero::ilog10(&x)),
          both!(UnsignedNonZero::bit_width(&x)).get(),
        ],
        [
          n.leading_zeros(),
          n.trailing_zeros(),
          n.count_ones().get(),
          n.highest_one(),
          n.lowest_one(),
          n.ilog2(),
          n.ilog10(),
          n.bit_width().get(),
        ],
        "{v}"
      );
      assert_eq!(
        both!(UnsignedNonZero::is_power_of_two(&x)),
        n.is_power_of_two(),
        "{v}"
      );
      assert_eq!(
        [
          read(both!(FixedNonZero::isolate_highest_one(&x))),
          read(both!(FixedNonZero::isolate_lowest_one(&x))),
          read(both!(UnsignedNonZero::isqrt(&x))),
          read(both!(FixedNonZero::reverse_bits(&x))),
        ],
        [
          n.isolate_highest_one().get(),
          n.isolate_lowest_one().get(),
          n.isqrt().get(),
          v.reverse_bits(),
        ],
        "{v}"
      );
      assert_eq!(
        both!(UnsignedNonZero::checked_next_power_of_two(&x)).map(read),
        n.checked_next_power_of_two().map(NonZero::get),
        "{v}"
      );
      for &k in &amounts {
        assert_eq!(
          [
            read(both!(FixedNonZero::rotate_left(&x, k))),
            read(both!(FixedNonZero::rotate_right(&x, k))),
          ],
          [v.rotate_left(k), v.rotate_right(k)],
          "{v} rotated by {k}"
        );
      }
      for exp in EXPONENTS {
        assert_eq!(
          both!(FixedNonZero::checked_pow(&x, exp)).map(read),
          n.checked_pow(exp).map(NonZero::get),
          "{v}^{exp}"
        );
        assert_eq!(
          read(both!(FixedNonZero::saturating_pow(&x, exp))),
          n.saturating_pow(exp).get(),
          "{v}^{exp}"
        );
      }
      assert_eq!(
        cast(&both!(UnsignedNonZero::cast_signed(x)).get()),
        n.cast_signed().get(),
        "{v}"
      );
      assert_eq!(
        (x.to_string(), format!("{x:?}")),
        (n.to_string(), format!("{n:?}"))
      );
      // Each hashes as its integer does.
      assert_eq!((hash_of(&x), hash_of(&n)), (hash_of(&x.get()), hash_of(&v)));
    }
    for (v, w) in $pairs
      .into_iter()
      .map(|(v, w)| (v as $native, w as $native))
    {
      let Some(n) = NonZero::<$native>::new(v) else {
        continue;
      };
      let (x, b) = (<$ty>::new(integer(v)).expect("not zero"), integer(w));
      assert_eq!(
        both!(UnsignedNonZero::checked_add(&x, &b)).map(read),
        n.checked_add(w).map(NonZero::get),
        "{v} + {w}"
      );
      assert_eq!(
        read(both!(UnsignedNonZero::saturating_add(&x, &b))),
        n.saturating_add(w).get(),
        "{v} + {w}"
      );
      if let (Some(y), Some(m)) = (<$ty>::new(b), NonZero::new(w)) {
        assert_eq!(
          both!(FixedNonZero::checked_mul(&x, &y)).map(read),
          n.checked_mul(m).map(NonZero::get),
          "{v} * {w}"
        );
        assert_eq!(
          [
            read(both!(FixedNonZero::saturating_mul(&x, &y))),
            read(both!(UnsignedNonZero::midpoint(&x, &y))),
            read(both!(UnsignedNonZero::div_ceil(&x, &y))),
          ],
          [
            n.saturating_mul(m).get(),
            n.midpoint(m).get(),
            n.div_ceil(m).get(),
          ],
          "{v} and {w}"
        );
        assert_eq!((x.cmp(&y), x == y), (n.cmp(&m), n == m), "{v} and {w}");
      }
    }
  }};
}

/// Checks the non-zero integers `$ty`, of the width of the native signed `$native`, against
/// `NonZero<$native>`, as [`unsigned_against_std`] does for the unsigned ones: `$cast` reads the
/// unsigned integer of the casts and of `unsigned_abs`.
macro_rules! signed_against_std {
  (
    $ty:ty, $native:ident;
    $integer:expr, $value:expr, $cast:expr;
    $values:expr, $pairs:expr
  ) => {{
    let (integer, value, cast) = ($integer, $value, $cast);
    let read = |x: $ty| value(&x.get());
    let amounts = shift_amounts($native::BITS);
    for v in $values
      .into_iter()
      .map(|v| v as $native)
      .filter(|&v| v != 0)
    {
      let x = <$ty>::new(integer(v)).expect("not zero");
      let n = NonZero::<$native>::new(v).expect("not zero");
      assert_eq!(
        [
          both!(FixedNonZero::leading_zeros(&x)),
          both!(FixedNonZero::trailing_zeros(&x)),
          both!(FixedNonZero::count_ones(&x)).get(),
          both!(FixedNonZero::highest_one(&x)),
          both!(FixedNonZero::lowest_one(&x)),
        ],
        [
          n.leading_zeros(),
          n.trailing_zeros(),
          n.count_ones().get(),
          n.highest_one(),
          n.lowest_one(),
        ],
        "{v}"
      );
      assert_eq!(
        [
          both!(SignedNonZero::is_negative(&x)),
          both!(SignedNonZero::is_positive(&x)),
        ],
        [n.is_negative(), n.is_positive()],
        "{v}"
      );
      assert_eq!(
        [
          read(both!(FixedNonZero::isolate_highest_one(&x))),
          read(both!(FixedNonZero::isolate_lowest_one(&x))),
          read(both!(FixedNonZero::reverse_bits(&x))),
          read(both!(SignedNonZero::wrapping_neg(&x))),
          read(both!(SignedNonZero::saturating_neg(&x))),
          read(both!(SignedNonZero::wrapping_abs(&x))),
          read(both!(SignedNonZero::saturating_abs(&x))),
        ],
        [
          n.isolate_highest_one().get(),
          n.isolate_lowest_one().get(),
          v.reverse_bits(),
          n.wrapping_neg().get(),
          n.saturating_neg().get(),
          n.wrapping_abs().get(),
          n.saturating_abs().get(),
        ],
        "{v}"
      );
      let (negated, overflow) = both!(SignedNonZero::overflowing_neg(&x));
      let (native, native_overflow) = n.overflowing_neg();
      assert_eq!(
        (read(negated), overflow),
        (native.get(), native_overflow),
        "-{v}"
      );
      let (abs, overflow) = both!(SignedNonZero::overflowing_abs(&x));
      let (native, native_overflow) = n.overflowing_abs();
      assert_eq!(
        (read(abs), overflow),
        (native.get(), native_overflow),
        "|{v}|"
      );
      assert_eq!(
        [
          both!(SignedNonZero::checked_neg(&x)).map(read),
          both!(SignedNonZero::checked_abs(&x)).map(read),
        ],
        [
          n.checked_neg().map(NonZero::get),
          n.checked_abs().map(NonZero::get),
        ],
        "{v}"
      );
      if v != $native::MIN {
        assert_eq!(read(both!(SignedNonZero::abs(&x))), n.abs().get(), "|{v}|");
      }
      assert_eq!(
        [
          cast(&both!(SignedNonZero::unsigned_abs(&x)).get()),
          cast(&both!(SignedNonZero::cast_unsigned(x)).get()),
        ],
        [n.unsigned_abs().get(), n.cast_unsigned().get()],
        "{v}"
      );
      for &k in &amounts {
        assert_eq!(
          [
            read(both!(FixedNonZero::rotate_left(&x, k))),
            read(both!(FixedNonZero::rotate_right(&x, k))),
          ],
          [v.rotate_left(k), v.rotate_right(k)],
          "{v} rotated by {k}"
        );
      }
      for exp in EXPONENTS {
        assert_eq!(
          both!(FixedNonZero::checked_pow(&x, exp)).map(read),
          n.checked_pow(exp).map(NonZero::get),
          "{v}^{exp}"
        );
        assert_eq!(
          read(both!(FixedNonZero::saturating_pow(&x, exp))),
          n.saturating_pow(exp).get(),
          "{v}^{exp}"
        );
      }
      assert_eq!(
        (x.to_string(), format!("{x:?}")),
        (n.to_string(), format!("{n:?}"))
      );
      assert_eq!((hash_of(&x), hash_of(&n)), (hash_of(&x.get()), hash_of(&v)));
    }
    for (v, w) in $pairs
      .into_iter()
      .map(|(v, w)| (v as $native, w as $native))
    {
      let (Some(n), Some(m)) = (NonZero::<$native>::new(v), NonZero::<$native>::new(w)) else {
        continue;
      };
      let x = <$ty>::new(integer(v)).expect("not zero");
      let y = <$ty>::new(integer(w)).expect("not zero");
      assert_eq!(
        both!(FixedNonZero::checked_mul(&x, &y)).map(read),
        n.checked_mul(m).map(NonZero::get),
        "{v} * {w}"
      );
      assert_eq!(
        read(both!(FixedNonZero::saturating_mul(&x, &y))),
        n.saturating_mul(m).get(),
        "{v} * {w}"
      );
      assert_eq!((x.cmp(&y), x == y), (n.cmp(&m), n == m), "{v} and {w}");
    }
  }};
}

/// The non-zero integers of 8 bits in `u8`, `i8` and `[u8; 1]`, and of 16 bits in `u16`, `i16`
/// and `[u8; 2]`, against the native non-zero integers: every row that the native ones have,
/// through the inherent `const fn`s and the traits, the order, the hash and the text, at every
/// value of 8 bits and every pair in an optimized build, every 13th pair in an unoptimized one, and
/// at the values and the pairs that the tier takes of 16 bits. The native bit reversal and
/// rotations of a non-zero integer are unstable, and those of the native integers stand in. Miri
/// checks the ends of 8 bits.
#[test]
fn nonzero_against_the_native_nonzero() {
  let samples = |signed| {
    if tier().is_miri() {
      let ends = ends(8, signed);
      let pairs: Vec<(i128, i128)> = ends
        .iter()
        .flat_map(|&a| ends.iter().map(move |&b| (a, b)))
        .collect();
      (ends, pairs)
    } else {
      (every_value(8, signed, 1), small_pairs(8, signed, true))
    }
  };
  let (values, pairs) = samples(false);
  unsigned_against_std!(
    NonZeroUint<U8, NonZero<u8>>, u8;
    Uint::<U8, u8>::new, |i: &Uint<U8, u8>| i.value(), |i: &Int<U8, i8>| i.value();
    values.clone(), pairs.clone()
  );
  unsigned_against_std!(
    NonZeroUint<U8, [u8; 1]>, u8;
    |v| Uint::<U8, [u8; 1]>::from_limbs([v]),
    |i: &Uint<U8, [u8; 1]>| i.to_limbs()[0],
    |i: &Int<U8, [u8; 1]>| i.to_limbs()[0] as i8;
    values, pairs
  );
  let (values, pairs) = samples(true);
  signed_against_std!(
    NonZeroInt<U8, NonZero<i8>>, i8;
    Int::<U8, i8>::new, |i: &Int<U8, i8>| i.value(), |i: &Uint<U8, u8>| i.value();
    values.clone(), pairs.clone()
  );
  signed_against_std!(
    NonZeroInt<U8, [u8; 1]>, i8;
    |v: i8| Int::<U8, [u8; 1]>::from_limbs([v as u8]),
    |i: &Int<U8, [u8; 1]>| i.to_limbs()[0] as i8,
    |i: &Uint<U8, [u8; 1]>| i.to_limbs()[0];
    values, pairs
  );
  if tier().is_miri() {
    return;
  }
  let (values, pairs) = (large_values(16, false), large_pairs(16, false));
  unsigned_against_std!(
    NonZeroUint<U16, NonZero<u16>>, u16;
    Uint::<U16, u16>::new, |i: &Uint<U16, u16>| i.value(), |i: &Int<U16, i16>| i.value();
    values.clone(), pairs.clone()
  );
  unsigned_against_std!(
    NonZeroUint<U16, [u8; 2]>, u16;
    |v: u16| Uint::<U16, [u8; 2]>::from_limbs(v.to_le_bytes()),
    |i: &Uint<U16, [u8; 2]>| u16::from_le_bytes(i.to_limbs()),
    |i: &Int<U16, [u8; 2]>| i16::from_le_bytes(i.to_limbs());
    values, pairs
  );
  let (values, pairs) = (large_values(16, true), large_pairs(16, true));
  signed_against_std!(
    NonZeroInt<U16, NonZero<i16>>, i16;
    Int::<U16, i16>::new, |i: &Int<U16, i16>| i.value(), |i: &Uint<U16, u16>| i.value();
    values.clone(), pairs.clone()
  );
  signed_against_std!(
    NonZeroInt<U16, [u8; 2]>, i16;
    |v: i16| Int::<U16, [u8; 2]>::from_limbs(v.to_le_bytes()),
    |i: &Int<U16, [u8; 2]>| i16::from_le_bytes(i.to_limbs()),
    |i: &Uint<U16, [u8; 2]>| u16::from_le_bytes(i.to_limbs());
    values, pairs
  );
}

/// The pairs of the patterns at which the rows of two operands of a wide width are checked: each
/// of the first eight with each of the last eight, at most 64.
fn pattern_pairs(patterns: &[Vec<bool>]) -> Vec<(Vec<bool>, Vec<bool>)> {
  let first = patterns.iter().take(8);
  first
    .flat_map(|p| {
      patterns
        .iter()
        .rev()
        .take(8)
        .map(move |q| (p.clone(), q.clone()))
    })
    .collect()
}

/// Checks the rows of the non-zero integer of `$ty`, an integer of the sign `$sign`, at the patterns
/// `$patterns` and the pairs `$pairs`, which are not zero: through the traits against the rows of
/// the integer, and through the inherent `const fn`s against the traits, at the amounts
/// `$amounts`. Returns the results as text, which depends only on the values.
macro_rules! check_wide {
  (unsigned; $ty:ty, $patterns:expr, $pairs:expr, $amounts:expr) => {{
    let amounts: &[u32] = $amounts;
    let mut text = Vec::new();
    for p in $patterns {
      let i = <$ty>::from_pattern(p);
      let x = NonZeroOf::<$ty>::new(i).expect("not zero");
      let rows = unsigned_rows_through_traits(&x, amounts);
      assert_eq!(
        rows,
        expected_unsigned_rows(&i, amounts),
        "{}",
        model::text(p)
      );
      assert_eq!(unsigned_rows!(&x, x, amounts), rows, "{}", model::text(p));
      text.push(format!("{rows:?}"));
    }
    for (p, q) in $pairs {
      let (i, j) = (<$ty>::from_pattern(p), <$ty>::from_pattern(q));
      let x = NonZeroOf::<$ty>::new(i).expect("not zero");
      let y = NonZeroOf::<$ty>::new(j).expect("not zero");
      let pair = unsigned_pair_through_traits(&x, &j, Some(&y));
      let name = format!("{} and {}", model::text(p), model::text(q));
      assert_eq!(pair, expected_unsigned_pair(&i, &j), "{name}");
      assert_eq!(unsigned_pair!(&x, &j, Some(&y)), pair, "{name}");
      text.push(format!("{pair:?}"));
    }
    text
  }};
  (signed; $ty:ty, $patterns:expr, $pairs:expr, $amounts:expr) => {{
    let amounts: &[u32] = $amounts;
    let mut text = Vec::new();
    for p in $patterns {
      let i = <$ty>::from_pattern(p);
      let x = NonZeroOf::<$ty>::new(i).expect("not zero");
      let rows = signed_rows_through_traits(&x, amounts);
      assert_eq!(
        rows,
        expected_signed_rows(&i, amounts),
        "{}",
        model::text(p)
      );
      assert_eq!(signed_rows!(&x, x, amounts), rows, "{}", model::text(p));
      text.push(format!("{rows:?}"));
    }
    for (p, q) in $pairs {
      let (i, j) = (<$ty>::from_pattern(p), <$ty>::from_pattern(q));
      let x = NonZeroOf::<$ty>::new(i).expect("not zero");
      let y = NonZeroOf::<$ty>::new(j).expect("not zero");
      let pair = signed_pair_through_traits(&x, &y);
      let name = format!("{} and {}", model::text(p), model::text(q));
      assert_eq!(pair, expected_signed_pair(&i, &j), "{name}");
      assert_eq!(signed_pair!(&x, &y), pair, "{name}");
      text.push(format!("{pair:?}"));
    }
    text
  }};
}

/// Checks the rows of the non-zero integers of `Uint` and `Int` of `$bits` bits, the width
/// `$width`, in each storage of `$storages`, written `unsigned, signed;`, with [`check_wide`], at
/// the patterns of [`wide_patterns`] in `$words` words of 64 bits that are not zero and the pairs
/// of [`pattern_pairs`]: every storage of a sign gives the results of the first.
macro_rules! nonzero_agree {
  ($width:ty, $bits:literal, $words:literal; $($uint:ty, $int:ty);+ $(;)?) => {{
    let patterns: Vec<Vec<bool>> = wide_patterns::<$words>($bits)
      .into_iter()
      .filter(|p| p.contains(&true))
      .collect();
    let pairs = pattern_pairs(&patterns);
    let amounts = bigint::bit_amounts(64, $bits);
    let (mut unsigned, mut signed): (Vec<Vec<String>>, Vec<Vec<String>>) = (Vec::new(), Vec::new());
    $(
      unsigned.push(check_wide!(unsigned; Uint<$width, $uint>, &patterns, &pairs, &amounts));
      signed.push(check_wide!(signed; Int<$width, $int>, &patterns, &pairs, &amounts));
    )+
    for texts in [unsigned, signed] {
      for text in &texts {
        assert_eq!(text, &texts[0], "{} bits", $bits);
      }
    }
  }};
}

/// The non-zero patterns of 24 to 4096 bits in storages of each kind that hold them: the rows
/// through the traits agree with the rows of the integer, the inherent `const fn`s agree with the
/// traits, and every storage gives the same results. An optimized build adds 1024 and 4096 bits;
/// Miri checks 65 bits in two storages and 129 bits in one, at eight patterns.
#[test]
fn storages_agree_on_the_nonzero_methods() {
  if tier().is_miri() {
    nonzero_agree!(U65, 65, 2; [u8; 9], [u8; 9]; [u64; 2], [u64; 2]);
    nonzero_agree!(U129, 129, 3; [u64; 3], [u64; 3]);
    return;
  }
  nonzero_agree!(U24, 24, 1; u32, i32; [u8; 3], [u8; 3]; [u16; 2], [u16; 2]; [u64; 1], [u64; 1]);
  nonzero_agree!(
    U65, 65, 2;
    u128, i128; [u8; 9], [u8; 9]; [u16; 5], [u16; 5]; [u32; 3], [u32; 3]; [u64; 2], [u64; 2];
    [u128; 1], [u128; 1]
  );
  nonzero_agree!(
    U129, 129, 3;
    [u8; 17], [u8; 17]; [u16; 9], [u16; 9]; [u32; 5], [u32; 5]; [u64; 3], [u64; 3];
    [u128; 2], [u128; 2]
  );
  nonzero_agree!(
    U200, 200, 4;
    [u8; 25], [u8; 25]; [u16; 13], [u16; 13]; [u32; 7], [u32; 7]; [u64; 4], [u64; 4];
    [u128; 2], [u128; 2]
  );
  if tier().is_full() {
    nonzero_agree!(U1024, 1024, 16; [u32; 32], [u32; 32]; [u64; 16], [u64; 16]);
    nonzero_agree!(U4096, 4096, 64; [u8; 512], [u8; 512]; [u64; 64], [u64; 64]);
  }
}

/// Rows of each table in constant contexts, in a native and an array storage, at one bit, at 7
/// bits and at 24 bits, and the limits in a generic `const fn` through `ConstNonZeroLimits`.
#[test]
fn rows_in_constant_contexts() {
  type Small = NonZeroUint<U7>;
  type Bytes = NonZeroInt<U24, [u8; 3]>;
  type Word = NonZeroUint<U7, [u64; 1]>;
  type I1 = NonZeroInt<U1>;
  const FIVE: Small = Small::new(u7::new(5)).unwrap();
  const COUNTS: [u32; 4] = [
    FIVE.leading_zeros(),
    FIVE.count_ones().get(),
    Bytes::MIN.highest_one(),
    FIVE.ilog2(),
  ];
  const CHECKED: [Option<Small>; 2] = [FIVE.checked_mul(&FIVE), FIVE.checked_add(&u7::new(123))];
  const VALUES: [Small; 2] = [FIVE.isqrt(), FIVE.saturating_pow(3)];
  const ROTATED: Word = Word::MAX.rotate_left(1);
  const CAST: NonZeroInt<U7> = Small::MAX.cast_signed();
  const SIGNED: (bool, Option<Bytes>, Bytes) = (
    Bytes::MIN.is_negative(),
    Bytes::MIN.checked_abs(),
    Bytes::MIN.saturating_abs(),
  );
  const UNSIGNED_ABS: NonZeroUint<U24, [u8; 3]> = Bytes::MIN.unsigned_abs();
  const ONE_BIT: (Option<I1>, [I1; 2], NonZeroUint<U1>) = (
    I1::MIN.checked_neg(),
    [I1::MIN.saturating_neg(), I1::MIN.saturating_mul(&I1::MIN)],
    I1::MIN.cast_unsigned(),
  );
  const fn limits<T: ConstNonZeroLimits>() -> [T; 2] {
    [T::MIN, T::MAX]
  }
  const LIMITS: ([I1; 2], [Word; 2]) = (limits(), limits());

  assert_eq!(COUNTS, [4, 2, 23, 2]);
  assert_eq!(
    CHECKED.map(|x| x.map(Small::get)),
    [Some(u7::new(25)), None]
  );
  assert_eq!(VALUES.map(Small::get), [u7::new(2), u7::new(125)]);
  assert_eq!(ROTATED, Word::MAX);
  assert_eq!(CAST.get(), i7::new(-1));
  assert_eq!(SIGNED, (true, None, Bytes::MAX));
  assert_eq!(UNSIGNED_ABS.get().to_limbs(), [0, 0, 0x80]);
  assert_eq!(ONE_BIT, (None, [I1::MIN; 2], NonZeroUint::<U1>::MIN));
  assert_eq!(LIMITS, ([I1::MIN; 2], [Word::MIN, Word::MAX]));
}

/// The rows of `x`, and of `x` with each of `values`, through the traits, as text, in code over
/// `B: BitWidth`.
fn unsigned_over_the_width<B: BitWidth>(
  x: &NonZeroUint<B>,
  values: &[NonZeroUint<B>],
  amounts: &[u32],
) -> (String, Vec<String>) {
  let rows = format!("{:?}", unsigned_rows_through_traits(x, amounts));
  let pairs = values
    .iter()
    .map(|y| format!("{:?}", unsigned_pair_through_traits(x, &y.get(), Some(y))))
    .collect();
  (rows, pairs)
}

/// The rows of `x`, and of `x` with each of `values`, through the traits, as text, in code over
/// `B: BitWidth`.
fn signed_over_the_width<B: BitWidth>(
  x: &NonZeroInt<B>,
  values: &[NonZeroInt<B>],
  amounts: &[u32],
) -> (String, Vec<String>) {
  let rows = format!("{:?}", signed_rows_through_traits(x, amounts));
  let pairs = values
    .iter()
    .map(|y| format!("{:?}", signed_pair_through_traits(x, y)))
    .collect();
  (rows, pairs)
}

/// The limits through `NonZeroLimits`, in code over `B: BitWidth` with `B: NonZero`.
fn limits_over_the_width<B>() -> [String; 4]
where
  B: BitWidth + typenum::NonZero,
{
  [
    format!("{:?}", NonZeroUint::<B>::min_value()),
    format!("{:?}", NonZeroUint::<B>::max_value()),
    format!("{:?}", NonZeroInt::<B>::min_value()),
    format!("{:?}", NonZeroInt::<B>::max_value()),
  ]
}

/// The non-zero integer of `Uint<B>`, which `NonZeroOf` names, in code over `B: BitWidth`.
fn nonzero_of<B: BitWidth>(value: Uint<B>) -> Option<NonZeroUint<B>> {
  NonZeroOf::<Uint<B>>::new(value)
}

/// Code over `B: BitWidth` reaches every row of `NonZeroUint<B>` and `NonZeroInt<B>` through the
/// traits, which give the results of the inherent `const fn`s of the default storages, the limits
/// with `B: NonZero`, and the non-zero integer of `Uint<B>` through `NonZeroOf`: at widths of each
/// kind of default storage, at the limits and the values next to them. Miri leaves out 1024 bits.
#[test]
fn nonzero_over_the_width() {
  macro_rules! agrees {
    ($($width:ident),+ $(,)?) => {$({
      type U = NonZeroUint<$width>;
      type I = NonZeroInt<$width>;
      let amounts = bigint::bit_amounts(64, U::BITS);
      let unsigned = [U::MIN, U::MAX, U::MAX.isqrt()];
      for x in &unsigned {
        let rows = format!("{:?}", unsigned_rows!(x, *x, &amounts));
        let pairs: Vec<String> = unsigned
          .iter()
          .map(|y| format!("{:?}", unsigned_pair!(x, &y.get(), Some(y))))
          .collect();
        assert_eq!(unsigned_over_the_width(x, &unsigned, &amounts), (rows, pairs), "{x}");
      }
      let signed = [I::MIN, I::MAX, I::MAX.wrapping_neg()];
      for x in &signed {
        let rows = format!("{:?}", signed_rows!(x, *x, &amounts));
        let pairs: Vec<String> = signed
          .iter()
          .map(|y| format!("{:?}", signed_pair!(x, y)))
          .collect();
        assert_eq!(signed_over_the_width(x, &signed, &amounts), (rows, pairs), "{x}");
      }
      let limits = [U::MIN, U::MAX].map(|x| format!("{x:?}"));
      let signed_limits = [I::MIN, I::MAX].map(|x| format!("{x:?}"));
      assert_eq!(
        limits_over_the_width::<$width>(),
        [limits[0].clone(), limits[1].clone(), signed_limits[0].clone(), signed_limits[1].clone()]
      );
      assert_eq!(nonzero_of(U::MAX.get()), Some(U::MAX));
      assert_eq!(nonzero_of(Uint::<$width>::ZERO), None);
    })+};
  }

  agrees!(U1, U7, U8, U64, U65, U128, U129, U200);
  if !tier().is_miri() {
    agrees!(U1024);
  }
}

/// The cast of `x` in code over `B: BitWidth`, whose type is the `NonZeroInt` of the width.
fn cast_signed_over_the_width<B: BitWidth>(x: NonZeroUint<B>) -> NonZeroInt<B> {
  x.cast_signed()
}

/// The cast of `x` in code over `B: BitWidth`, whose type is the `NonZeroUint` of the width.
fn cast_unsigned_over_the_width<B: BitWidth>(x: NonZeroInt<B>) -> NonZeroUint<B> {
  x.cast_unsigned()
}

/// The absolute value of `x` in code over `B: BitWidth`, whose type is the `NonZeroUint` of the
/// width.
fn unsigned_abs_over_the_width<B: BitWidth>(x: NonZeroInt<B>) -> NonZeroUint<B> {
  x.unsigned_abs()
}

/// What code over the width gives for a non-zero value of each sign: the casts, and the limits of
/// both signs.
#[derive(Debug, PartialEq)]
struct OverTheWidth<U, S> {
  /// `cast_signed` of the unsigned value, twice.
  signed: [S; 2],
  /// `cast_unsigned` and `unsigned_abs` of the signed value.
  unsigned: [U; 2],
  /// `MIN` and `MAX` of the unsigned type.
  unsigned_limits: [U; 2],
  /// `MIN` and `MAX` of the signed type.
  signed_limits: [S; 2],
}

/// The [`OverTheWidth`] of `x` and `y`, in code over `B: BitWidth` with `B: NonZero`: each value is
/// used twice by value, which `Copy` allows, and the limits come from `ConstNonZeroLimits`.
fn copies_casts_and_limits<B>(
  x: NonZeroUint<B>,
  y: NonZeroInt<B>,
) -> OverTheWidth<NonZeroUint<B>, NonZeroInt<B>>
where
  B: BitWidth + typenum::NonZero,
{
  OverTheWidth {
    signed: [cast_signed_over_the_width(x), cast_signed_over_the_width(x)],
    unsigned: [
      cast_unsigned_over_the_width(y),
      unsigned_abs_over_the_width(y),
    ],
    unsigned_limits: [
      <NonZeroUint<B> as ConstNonZeroLimits>::MIN,
      <NonZeroUint<B> as ConstNonZeroLimits>::MAX,
    ],
    signed_limits: [
      <NonZeroInt<B> as ConstNonZeroLimits>::MIN,
      <NonZeroInt<B> as ConstNonZeroLimits>::MAX,
    ],
  }
}

/// Code over `B: BitWidth` copies `NonZeroUint<B>` and `NonZeroInt<B>`, casts each to the other
/// with `cast_signed`, `cast_unsigned` and `unsigned_abs`, whose results have the types of the
/// width, and with `B: NonZero` reads their limits through `ConstNonZeroLimits`: the inherent
/// results of the default storages, at widths of each kind of default storage, for the limits and
/// a value between them. Miri leaves out 1024 bits.
#[test]
fn nonzero_copies_casts_and_limits_over_the_width() {
  macro_rules! agrees {
    ($($width:ident),+ $(,)?) => {$({
      type U = NonZeroUint<$width>;
      type I = NonZeroInt<$width>;
      for (x, y) in [(U::MIN, I::MIN), (U::MAX, I::MAX), (U::MAX.isqrt(), I::MAX.wrapping_neg())] {
        let inherent = OverTheWidth {
          signed: [x.cast_signed(); 2],
          unsigned: [y.cast_unsigned(), y.unsigned_abs()],
          unsigned_limits: [U::MIN, U::MAX],
          signed_limits: [I::MIN, I::MAX],
        };
        assert_eq!(copies_casts_and_limits::<$width>(x, y), inherent, "{x} and {y}");
      }
    })+};
  }

  agrees!(U1, U7, U8, U64, U65, U128, U129, U200);
  if !tier().is_miri() {
    agrees!(U1024);
  }
}

/// The table of one bit, row by row: `NonZeroUint` of one bit holds only 1, and `NonZeroInt` of
/// one bit only -1, each in a native and an array storage. A product, a power and a sum that would
/// be 2, or 1 for one signed bit, overflow, and the saturating rows and the negations return the
/// only value; zero bits hold no value that is not zero.
#[test]
fn one_bit_rows() {
  macro_rules! unsigned_bit {
    ($($ty:ty),+) => {$({
      type T = $ty;
      let one = T::MIN;
      assert_eq!(T::MAX, one);
      assert_eq!(
        [
          one.leading_zeros(),
          one.trailing_zeros(),
          one.count_ones().get(),
          one.highest_one(),
          one.lowest_one(),
          one.ilog2(),
          one.ilog10(),
          one.bit_width().get(),
        ],
        [0, 0, 1, 0, 0, 0, 0, 1]
      );
      assert_eq!(
        [
          one.isolate_highest_one(),
          one.isolate_lowest_one(),
          one.reverse_bits(),
          one.rotate_left(1),
          one.rotate_right(u32::MAX),
          one.saturating_mul(&one),
          one.isqrt(),
          one.midpoint(&one),
          one.div_ceil(&one),
        ],
        [one; 9]
      );
      assert_eq!(one.checked_mul(&one), Some(one));
      for exp in EXPONENTS {
        assert_eq!((one.checked_pow(exp), one.saturating_pow(exp)), (Some(one), one), "1^{exp}");
      }
      assert_eq!(one.checked_add(&Uint::ZERO), Some(one));
      assert_eq!(one.checked_add(&Uint::ONE), None);
      assert_eq!(one.saturating_add(&Uint::ONE), one);
      assert_eq!(one.checked_next_power_of_two(), Some(one));
      assert!(one.is_power_of_two());
      assert_eq!(one.cast_signed().get(), Int::MIN);
    })+};
  }
  unsigned_bit!(NonZeroUint<U1, NonZero<u8>>, NonZeroUint<U1, [u8; 1]>);

  macro_rules! signed_bit {
    ($($ty:ty),+) => {$({
      type T = $ty;
      let minus_one = T::MIN;
      assert_eq!(T::MAX, minus_one);
      assert_eq!(
        [
          minus_one.leading_zeros(),
          minus_one.trailing_zeros(),
          minus_one.count_ones().get(),
          minus_one.highest_one(),
          minus_one.lowest_one(),
        ],
        [0, 0, 1, 0, 0]
      );
      assert!(minus_one.is_negative() && !minus_one.is_positive());
      assert_eq!(
        [
          minus_one.isolate_highest_one(),
          minus_one.isolate_lowest_one(),
          minus_one.reverse_bits(),
          minus_one.rotate_left(1),
          minus_one.rotate_right(u32::MAX),
          minus_one.saturating_mul(&minus_one),
          minus_one.wrapping_neg(),
          minus_one.saturating_neg(),
          minus_one.wrapping_abs(),
          minus_one.saturating_abs(),
        ],
        [minus_one; 10]
      );
      assert_eq!(minus_one.checked_mul(&minus_one), None);
      for exp in EXPONENTS {
        let odd = exp % 2 == 1;
        assert_eq!(
          (minus_one.checked_pow(exp), minus_one.saturating_pow(exp)),
          (odd.then_some(minus_one), minus_one),
          "(-1)^{exp}"
        );
      }
      assert_eq!(minus_one.overflowing_neg(), (minus_one, true));
      assert_eq!(minus_one.overflowing_abs(), (minus_one, true));
      assert_eq!((minus_one.checked_neg(), minus_one.checked_abs()), (None, None));
      assert_eq!(minus_one.unsigned_abs().get(), Uint::ONE);
      assert_eq!(minus_one.cast_unsigned().get(), Uint::ONE);
    })+};
  }
  signed_bit!(NonZeroInt<U1, NonZero<i8>>, NonZeroInt<U1, [u16; 1]>);
}

/// `abs`, `-x` and `-&x` of `MIN` overflow as the native `abs` and `-` do: with overflow checks
/// they panic with the message of a native negation and report the caller, through the inherent
/// `const fn` and the trait, and without them they return `MIN`; at 7 bits, at the full width of a
/// native integer, at one bit and at 256 bits.
#[test]
fn negation_and_abs_of_min_overflow() {
  let message = "attempt to negate with overflow".to_string();
  let small = NonZeroInt::<U7>::MIN;
  let full = NonZeroInt::<U8, NonZero<i8>>::MIN;
  let bit = NonZeroInt::<U1, [u8; 1]>::MIN;
  let wide = NonZeroInt::<U256>::MIN;
  overflows_here!(small.abs(), message, small);
  overflows_here!(SignedNonZero::abs(&small), message, small);
  overflows_here!(-small, message, small);
  overflows_here!(-&small, message, small);
  overflows_here!(full.abs(), message, full);
  overflows_here!(SignedNonZero::abs(&full), message, full);
  overflows_here!(-full, message, full);
  overflows_here!(-&full, message, full);
  overflows_here!(bit.abs(), message, bit);
  overflows_here!(SignedNonZero::abs(&bit), message, bit);
  overflows_here!(-bit, message, bit);
  overflows_here!(-&bit, message, bit);
  overflows_here!(wide.abs(), message, wide);
  overflows_here!(SignedNonZero::abs(&wide), message, wide);
  overflows_here!(-wide, message, wide);
  overflows_here!(-&wide, message, wide);
}

/// Checks the operators of the non-zero integer of `$ty`, the integer of the sign `$sign` and of
/// `$bits` bits, whose values `$from` creates from an `i128`, at the pairs of `$samples`, as
/// [`sweep_storage`] passes them, against the methods of the integer: `/` and `%` of the integer by
/// a non-zero divisor and their compound assignments, `|` of two non-zero values and of a non-zero
/// value and an integer either way, `|=` by both, and for `NonZeroInt` the negation of a value
/// that is not `MIN`, in every form of values and references.
macro_rules! sweep_nonzero_operators {
  ($sign:ident; $ty:ty, $bits:expr, $samples:expr, $from:expr, $value:expr, $magnitude:expr) => {{
    let from: fn(i128) -> $ty = $from;
    let (_, pairs): (Vec<i128>, Vec<(i128, i128)>) = $samples;
    for (a, b) in pairs {
      let (i, j) = (from(a), from(b));
      division_by_nonzero!($sign; $ty; i, j, a, b);
      let Some(x) = NonZeroOf::<$ty>::new(i) else {
        continue;
      };
      let or = NonZeroOf::<$ty>::new(i | j).expect("not zero");
      assert_eq!([x | j, x | &j, &x | j, &x | &j], [or; 4], "{a} | {b}");
      assert_eq!([j | x, j | &x, &j | x, &j | &x], [or; 4], "{b} | {a}");
      let mut targets = [x; 2];
      targets[0] |= j;
      targets[1] |= &j;
      assert_eq!(targets, [or; 2], "{a} |= {b}");
      if let Some(y) = NonZeroOf::<$ty>::new(j) {
        assert_eq!([x | y, x | &y, &x | y, &x | &y], [or; 4], "{a} | {b}");
        let mut targets = [x; 2];
        targets[0] |= y;
        targets[1] |= &y;
        assert_eq!(targets, [or; 2], "{a} |= {b}");
      }
      negation_of_nonzero!($sign; x, a);
    }
  }};
}

/// Checks `/`, `%`, `/=` and `%=` of the unsigned integer `$i` of `$ty` by `$j` where it is not
/// zero, the values `$a` and `$b`, against the checked division and remainder of the integer;
/// signed integers have none.
macro_rules! division_by_nonzero {
  (unsigned; $ty:ty; $i:expr, $j:expr, $a:expr, $b:expr) => {{
    let (i, j) = ($i, $j);
    if let Some(d) = NonZeroOf::<$ty>::new(j) {
      let quotient = i.checked_div(&j).expect("not zero");
      let remainder = i.checked_rem(&j).expect("not zero");
      assert_eq!(
        [i / d, i / &d, &i / d, &i / &d],
        [quotient; 4],
        "{} / {}",
        $a,
        $b
      );
      assert_eq!(
        [i % d, i % &d, &i % d, &i % &d],
        [remainder; 4],
        "{} % {}",
        $a,
        $b
      );
      let mut targets = [i; 4];
      targets[0] /= d;
      targets[1] /= &d;
      targets[2] %= d;
      targets[3] %= &d;
      assert_eq!(
        targets,
        [quotient, quotient, remainder, remainder],
        "{} and {}",
        $a,
        $b
      );
    }
  }};
  (signed; $ty:ty; $i:expr, $j:expr, $a:expr, $b:expr) => {{}};
}

/// Checks `-$x` and `-&$x` of a signed value `$x`, `$a`, where it is not `MIN`, against the checked
/// negation; unsigned values have none.
macro_rules! negation_of_nonzero {
  (unsigned; $x:expr, $a:expr) => {{}};
  (signed; $x:expr, $a:expr) => {{
    let x = $x;
    if let Some(negated) = x.checked_neg() {
      assert_eq!([-x, -&x], [negated; 2], "-{}", $a);
    }
  }};
}

/// Checks the operators at the width `B`, `bits` bits, at most 8, at the pairs of
/// [`nonzero_samples`], in the storages of [`nonzero_of_small_widths`].
#[allow(clippy::op_ref, reason = "the operators on references are under test")]
fn operators_of_small_widths<B>(bits: u32)
where
  B: FitsBits<u8> + FitsBits<i8> + FitsBits<u16> + FitsBits<i16> + FitsBits<u128> + FitsBits<i128>,
  B: FitsBits<[u8; 1]> + FitsBits<[u8; 2]> + FitsBits<[u16; 1]>,
  B: FitsBits<[u64; 1]> + FitsBits<[u128; 1]>,
{
  let everywhere = |bits, signed| nonzero_samples(bits, signed, true, true);
  let primary = |bits, signed| nonzero_samples(bits, signed, true, false);
  let secondary = |bits, signed| nonzero_samples(bits, signed, false, false);
  sweep_storage!(sweep_nonzero_operators, everywhere, native B, bits, u8, i8);
  sweep_storage!(sweep_nonzero_operators, everywhere, array B, bits, [u8; 1]);
  sweep_storage!(sweep_nonzero_operators, primary, array B, bits, [u8; 2]);
  sweep_storage!(sweep_nonzero_operators, primary, array B, bits, [u16; 1]);
  sweep_storage!(sweep_nonzero_operators, secondary, native B, bits, u16, i16);
  sweep_storage!(sweep_nonzero_operators, secondary, native B, bits, u128, i128);
  sweep_storage!(sweep_nonzero_operators, secondary, array B, bits, [u64; 1]);
  sweep_storage!(sweep_nonzero_operators, secondary, array B, bits, [u128; 1]);
}

/// Every form of `/`, `%`, `/=` and `%=` of `Uint` by `NonZeroUint`, of `|` and `|=`, and of the
/// negation of `NonZeroInt`, at the pairs of each width up to 8 bits that the tier takes, against
/// the methods of the integers, in native integers and in arrays of one and two limbs. Miri checks
/// 1, 2, 7 and 8 bits.
#[test]
fn nonzero_operators_of_up_to_8_bits() {
  operators_of_small_widths::<U1>(1);
  operators_of_small_widths::<U2>(2);
  operators_of_small_widths::<U7>(7);
  operators_of_small_widths::<U8>(8);
  if !tier().is_miri() {
    operators_of_small_widths::<U3>(3);
    operators_of_small_widths::<U4>(4);
    operators_of_small_widths::<U5>(5);
    operators_of_small_widths::<U6>(6);
  }
}

/// Checks the operators of `$ty`, a non-zero integer of the sign `$sign` and of the width of the
/// native `$native`, whose integer `$integer` creates from a native value and `$value` reads back,
/// against those of `NonZero<$native>` at the pairs `$pairs`: the division and the remainder of the
/// integer by a non-zero divisor and their compound assignments, for the unsigned ones, `|` in the
/// three pairings, `|=` by both, and the negation, for the signed ones, where it does not overflow.
macro_rules! operators_against_std {
  ($sign:ident; $ty:ty, $native:ident; $integer:expr, $value:expr; $pairs:expr) => {{
    let (integer, value) = ($integer, $value);
    let read = |x: $ty| value(&x.get());
    for (v, w) in $pairs.into_iter().map(|(v, w)| (v as $native, w as $native)) {
      let (i, j) = (integer(v), integer(w));
      native_division_by_nonzero!($sign; $ty; i, j, v, w, value);
      let (Some(x), Some(n)) = (<$ty>::new(i), NonZero::<$native>::new(v)) else {
        continue;
      };
      assert_eq!(
        [read(x | j), read(j | x)],
        [(n | w).get(), (w | n).get()],
        "{v} | {w}"
      );
      let (mut target, mut native) = (x, n);
      target |= j;
      native |= w;
      assert_eq!(read(target), native.get(), "{v} |= {w}");
      if let (Some(y), Some(m)) = (<$ty>::new(j), NonZero::<$native>::new(w)) {
        assert_eq!(read(x | y), (n | m).get(), "{v} | {w}");
        let (mut target, mut native) = (x, n);
        target |= y;
        native |= m;
        assert_eq!(read(target), native.get(), "{v} |= {w}");
      }
      native_negation_of_nonzero!($sign; $native; x, n, v, read);
    }
  }};
}

/// Checks `/` and `%` of the unsigned integer `$i` by `$j` where it is not zero, and their compound
/// assignments, against those of the native `$v` by the native `NonZero` of `$w`, reading the
/// results with `$value`; signed integers have none.
macro_rules! native_division_by_nonzero {
  (unsigned; $ty:ty; $i:expr, $j:expr, $v:expr, $w:expr, $value:expr) => {{
    let (i, j, v, w, value) = ($i, $j, $v, $w, $value);
    if let (Some(d), Some(m)) = (<$ty>::new(j), NonZero::new(w)) {
      assert_eq!(
        [value(&(i / d)), value(&(i % d))],
        [v / m, v % m],
        "{v} and {w}"
      );
      let (mut quotient, mut remainder) = (i, i);
      quotient /= d;
      remainder %= d;
      let (mut native_quotient, mut native_remainder) = (v, v);
      native_quotient /= m;
      native_remainder %= m;
      assert_eq!(
        [value(&quotient), value(&remainder)],
        [native_quotient, native_remainder],
        "{v} and {w}"
      );
    }
  }};
  (signed; $ty:ty; $i:expr, $j:expr, $v:expr, $w:expr, $value:expr) => {{}};
}

/// Checks `-$x` and `-&$x` against the negation of `$n`, the native `NonZero` of `$v`, where it
/// is not `MIN` of the native `$native`, reading the results with `$read`; unsigned values have no
/// negation.
macro_rules! native_negation_of_nonzero {
  (unsigned; $native:ident; $x:expr, $n:expr, $v:expr, $read:expr) => {{}};
  (signed; $native:ident; $x:expr, $n:expr, $v:expr, $read:expr) => {{
    let (x, n, v, read) = ($x, $n, $v, $read);
    if v != $native::MIN {
      assert_eq!([read(-x), read(-&x)], [(-n).get(), (-&n).get()], "-{v}");
    }
  }};
}

/// The operators of the non-zero integers of 8 bits in `u8`, `i8` and `[u8; 1]`, and of 16 bits in
/// `u16` and `i16`, against those of the native non-zero integers, at the pairs that the tier
/// takes: as [`nonzero_against_the_native_nonzero`] takes them, every pair of 8 bits in an
/// optimized build, and Miri the ends of 8 bits.
#[test]
fn nonzero_operators_against_the_native_nonzero() {
  let pairs = |signed| {
    if tier().is_miri() {
      let ends = ends(8, signed);
      ends
        .iter()
        .flat_map(|&a| ends.iter().map(move |&b| (a, b)))
        .collect::<Vec<(i128, i128)>>()
    } else {
      small_pairs(8, signed, true)
    }
  };
  operators_against_std!(
    unsigned; NonZeroUint<U8, NonZero<u8>>, u8;
    Uint::<U8, u8>::new, |i: &Uint<U8, u8>| i.value();
    pairs(false)
  );
  operators_against_std!(
    unsigned; NonZeroUint<U8, [u8; 1]>, u8;
    |v| Uint::<U8, [u8; 1]>::from_limbs([v]), |i: &Uint<U8, [u8; 1]>| i.to_limbs()[0];
    pairs(false)
  );
  operators_against_std!(
    signed; NonZeroInt<U8, NonZero<i8>>, i8;
    Int::<U8, i8>::new, |i: &Int<U8, i8>| i.value();
    pairs(true)
  );
  operators_against_std!(
    signed; NonZeroInt<U8, [u8; 1]>, i8;
    |v: i8| Int::<U8, [u8; 1]>::from_limbs([v as u8]), |i: &Int<U8, [u8; 1]>| i.to_limbs()[0] as i8;
    pairs(true)
  );
  if tier().is_miri() {
    return;
  }
  operators_against_std!(
    unsigned; NonZeroUint<U16, NonZero<u16>>, u16;
    Uint::<U16, u16>::new, |i: &Uint<U16, u16>| i.value();
    large_pairs(16, false)
  );
  operators_against_std!(
    signed; NonZeroInt<U16, NonZero<i16>>, i16;
    Int::<U16, i16>::new, |i: &Int<U16, i16>| i.value();
    large_pairs(16, true)
  );
}

/// The operators of code over `B: BitWidth`, on values and on references, each value used again
/// after it is passed by value, which `Copy` allows, as text: `/`, `%`, `/=` and `%=` of `total`
/// by `parts`, `|` of `parts` with `other` and with `total` either way, `|=` by both, `|` of
/// `signed` with `nonzero` either way, and the negation of `nonzero` where it does not overflow.
#[allow(clippy::op_ref, reason = "the operators on references are under test")]
fn operators_over_the_width<B: BitWidth>(
  total: Uint<B>,
  parts: NonZeroUint<B>,
  other: NonZeroUint<B>,
  signed: Int<B>,
  nonzero: NonZeroInt<B>,
) -> [String; 4] {
  let (mut quotient, mut remainder) = (total, total);
  quotient /= parts;
  remainder %= &parts;
  let mut or = parts;
  or |= other;
  or |= &total;
  let negated: Option<[NonZeroInt<B>; 2]> = nonzero.checked_neg().map(|_| [-nonzero, -&nonzero]);
  [
    format!(
      "{:?}",
      [
        total / parts,
        &total / &parts,
        total % parts,
        &total % &parts,
        quotient,
        remainder,
      ]
    ),
    format!(
      "{:?}",
      [
        parts | other,
        &parts | &other,
        parts | total,
        &parts | &total,
        total | parts,
        &total | &parts,
        or,
      ]
    ),
    format!(
      "{:?}",
      [signed | nonzero, &signed | &nonzero, nonzero | signed]
    ),
    format!("{negated:?}"),
  ]
}

/// Code over `B: BitWidth` uses the operators of `NonZeroUint<B>` and `NonZeroInt<B>` on values and
/// on references, which give the results of the methods of the integers of the default storages,
/// at widths of each kind of default storage, for the limits and a value between them. Miri leaves
/// out 1024 bits.
#[test]
fn nonzero_operators_over_the_width() {
  macro_rules! agrees {
    ($($width:ident),+ $(,)?) => {$({
      type U = NonZeroUint<$width>;
      type I = NonZeroInt<$width>;
      let (total, other, signed) = (Uint::<$width>::MAX, U::MAX.isqrt(), Int::<$width>::MIN);
      for (parts, nonzero) in [(U::MIN, I::MIN), (U::MAX, I::MAX), (other, I::MAX.wrapping_neg())] {
        let quotient = total.checked_div(&parts.get()).expect("not zero");
        let remainder = total.checked_rem(&parts.get()).expect("not zero");
        let with = |value: Uint<$width>| U::new(parts.get().bitor(value)).expect("not zero");
        let (with_other, with_total) = (with(other.get()), with(total));
        let assigned = U::new(parts.get().bitor(other.get()).bitor(total)).expect("not zero");
        let signed_or = I::new(signed.bitor(nonzero.get())).expect("not zero");
        let expected = [
          format!("{:?}", [quotient, quotient, remainder, remainder, quotient, remainder]),
          format!(
            "{:?}",
            [with_other, with_other, with_total, with_total, with_total, with_total, assigned]
          ),
          format!("{:?}", [signed_or; 3]),
          format!("{:?}", nonzero.checked_neg().map(|negated| [negated; 2])),
        ];
        assert_eq!(
          operators_over_the_width::<$width>(total, parts, other, signed, nonzero),
          expected,
          "{parts} and {nonzero}"
        );
      }
    })+};
  }

  agrees!(U1, U7, U8, U64, U65, U128, U129, U200);
  if !tier().is_miri() {
    agrees!(U1024);
  }
}
