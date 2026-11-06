//! The conversions of the non-zero integers `NonZeroUint` and `NonZeroInt`: `TryFrom` of the
//! integer of the same width and storage, which returns the error of the kind `Zero` for zero, at
//! every value of 8 bits against the native `NonZero` integers, and at the values of the casts in
//! every kind of storage against the bit model. Each type checks in an instantiation of a generic
//! function of its own, whose frame holds its values alone.

use core::{
  any::type_name,
  num::{NonZero, TryFromIntError},
};

use typenum::{U12, U33};

use super::{
  casts::{Model, samples},
  *,
};
use crate::{NonZeroInt, NonZeroUint};

/// The error of the native conversions for zero into a non-zero integer, of the kind `Zero`.
fn zero_error() -> TryFromIntError {
  NonZero::<u8>::try_from(0u8).unwrap_err()
}

/// Checks `TryFrom` into the non-zero integer `T` of 8 bits of every value of its integer `I`, as
/// the tier takes them, against the native `TryFrom` into `NonZero<u8>`, or into `NonZero<i8>` for
/// a signed `T`: the same value, or the same error, of the kind `Zero`, for zero.
fn check_try_from_against_native<T, I>()
where
  T: TryFrom<I, Error = TryFromIntError>,
  I: Model + From<T>,
{
  for value in small_values(8, I::SIGNED, true) {
    let native = if I::SIGNED {
      NonZero::<i8>::try_from(value as i8).map(|nonzero| pattern_of(i128::from(nonzero.get()), 8))
    } else {
      NonZero::<u8>::try_from(value as u8).map(|nonzero| pattern_of(i128::from(nonzero.get()), 8))
    };
    let integer = I::of_pattern(&pattern_of(value, 8));
    assert_eq!(
      T::try_from(integer).map(|nonzero| I::from(nonzero).pattern()),
      native,
      "{value} into {}",
      type_name::<T>()
    );
  }
}

/// Checks `TryFrom` into the non-zero integer `T` of each value of `samples` of its integer `I`,
/// against the bit model: the value, unless it is zero, and the error of the native conversions of
/// the kind `Zero` for zero.
fn check_try_from<T, I>(samples: &[I])
where
  T: TryFrom<I, Error = TryFromIntError>,
  I: Model + From<T>,
{
  let zero = zero_error();
  for &integer in samples {
    let expected = if integer.pattern().contains(&true) {
      Ok(integer)
    } else {
      Err(zero)
    };
    assert_eq!(
      T::try_from(integer).map(I::from),
      expected,
      "{integer:?} into {}",
      type_name::<T>()
    );
  }
}

/// `TryFrom` of every integer of 8 bits into its non-zero integer, in a native storage and in
/// arrays, of both signs, gives what the native `TryFrom` of `NonZero<u8>` and `NonZero<i8>`
/// gives: the value, or for zero the error of the kind `Zero`, equal to the native one by value and
/// by text.
#[test]
fn try_from_the_integer_matches_the_native_nonzero_at_8_bits() {
  check_try_from_against_native::<NonZeroUint<U8, NonZero<u8>>, Uint<U8, u8>>();
  check_try_from_against_native::<NonZeroUint<U8, [u8; 1]>, Uint<U8, [u8; 1]>>();
  check_try_from_against_native::<NonZeroUint<U8, [u64; 1]>, Uint<U8, [u64; 1]>>();
  check_try_from_against_native::<NonZeroInt<U8, NonZero<i8>>, Int<U8, i8>>();
  check_try_from_against_native::<NonZeroInt<U8, [u16; 1]>, Int<U8, [u16; 1]>>();
  check_try_from_against_native::<NonZeroInt<U8, [u128; 1]>, Int<U8, [u128; 1]>>();

  let zero = zero_error();
  assert_eq!(NonZeroUint::<U8>::try_from(Uint::<U8>::ZERO), Err(zero));
  assert_eq!(NonZeroInt::<U200>::try_from(Int::<U200>::ZERO), Err(zero));
  assert_eq!(zero, NonZero::<i64>::try_from(0i64).unwrap_err());
  assert_eq!(zero.to_string(), "number would be zero for non-zero type");
  assert_ne!(zero, u8::try_from(u16::MAX).unwrap_err());
  assert_ne!(zero, u8::try_from(-1i8).unwrap_err());
}

/// Checks `TryFrom` into each non-zero integer of `$nonzero => $integer`, from its integer, at the
/// values of [`samples`] of the integer, each pair in an instantiation of [`check_try_from`] of its
/// own.
macro_rules! try_from_integers {
  ($($nonzero:ty => $integer:ty),+ $(,)?) => {$(
    check_try_from::<$nonzero, $integer>(&samples::<$integer>());
  )+};
}

/// `TryFrom` of the integer into its non-zero integer, at the values of the casts, in native
/// storages and arrays of each limb type, of both signs, at zero bits, where every value is zero,
/// at one signed bit and up to 200 bits, against the bit model.
#[test]
fn try_from_the_integer_in_every_kind_of_storage() {
  try_from_integers!(
    NonZeroUint<U0> => Uint<U0>,
    NonZeroInt<U0, [u8; 0]> => Int<U0, [u8; 0]>,
    NonZeroInt<U1, NonZero<i8>> => Int<U1, i8>,
    NonZeroUint<U7, NonZero<u8>> => Uint<U7, u8>,
    NonZeroInt<U7, NonZero<i8>> => Int<U7, i8>,
    NonZeroUint<U8, [u8; 1]> => Uint<U8, [u8; 1]>,
    NonZeroInt<U12, [u16; 1]> => Int<U12, [u16; 1]>,
    NonZeroUint<U33, [u32; 2]> => Uint<U33, [u32; 2]>,
    NonZeroInt<U64, NonZero<i64>> => Int<U64, i64>,
    NonZeroUint<U65, [u64; 2]> => Uint<U65, [u64; 2]>,
    NonZeroInt<U100, [u128; 1]> => Int<U100, [u128; 1]>,
    NonZeroUint<U128, NonZero<u128>> => Uint<U128, u128>,
    NonZeroInt<U129, [u64; 3]> => Int<U129, [u64; 3]>,
    NonZeroUint<U200, [u8; 25]> => Uint<U200, [u8; 25]>,
  );
}
