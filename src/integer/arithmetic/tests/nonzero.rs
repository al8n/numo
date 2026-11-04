//! The non-zero integers `NonZeroUint` and `NonZeroInt`: their layout and the niche of a native
//! storage, the constructor, the value and the constants, also in constant contexts, the standard
//! traits against the integer of the same value, and the table of zero bits and one bit.

use core::{
  hash::{Hash, Hasher},
  num::NonZero,
};
use std::hash::DefaultHasher;

use super::*;
use crate::{NonZeroInt, NonZeroUint};

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
