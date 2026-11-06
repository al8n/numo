//! The conversions of the non-zero integers `NonZeroUint` and `NonZeroInt`: `TryFrom` of the
//! integer of the same width and storage, which returns the error of the kind `Zero` for zero, at
//! every value of 8 bits against the native `NonZero` integers, and at the values of the casts in
//! every kind of storage against the bit model; the conversions with the native `NonZero`
//! integers, both ways, against the bit model and, at 8 and 16 bits, against the conversions
//! between the native `NonZero` integers; and the casts with the non-zero integers and the native
//! `NonZero` integers as sources, into every kind of integer, and as targets, from every kind of
//! source, zero included, `widen` and `truncate` and the inherent methods, against the bit model,
//! with the panics of the strict cast. Each type or pair checks in an instantiation of a generic
//! function of its own, whose frame holds its values alone: the function is `#[inline(never)]`,
//! since an optimized build would inline a function that has one call site, which would put every
//! pair in the frame of the test, beyond the stack of a test thread.

use core::{
  any::type_name,
  convert::Infallible,
  num::{NonZero, TryFromIntError},
};

use typenum::{U3, U12, U17, U33, U150};

use super::{
  casts::{Model, check_cast, check_truncate, check_widen, errors, expected, samples},
  *,
};
use crate::{
  BoundedCastFrom, CheckedCastFrom, NonZeroInt, NonZeroIntRepr, NonZeroUint, NonZeroUintRepr,
  WidenFrom, integer::cast::try_cast,
};

/// The error of the native conversions for zero into a non-zero integer, of the kind `Zero`.
fn zero_error() -> TryFromIntError {
  NonZero::<u8>::try_from(0u8).unwrap_err()
}

/// A non-zero integer as the tests read it: the [`Model`] of its value, whose pattern is not zero,
/// and the integer of the same width and sign.
trait NonZeroModel: Model {
  /// The integer of the same width and sign, which may be zero.
  type Integer: Model;

  /// The non-zero integer of `integer`, or `None` if it is zero.
  fn of_integer(integer: Self::Integer) -> Option<Self>;
}

/// Implements [`Model`] and [`NonZeroModel`] for the non-zero integers over the storages of their
/// storage traits, written `Name<Integer>: Repr, signed;`, through the model of the integer.
macro_rules! nonzero_model {
  ($($name:ident<$integer:ident>: $repr:ident, $signed:literal;)+) => {$(
    impl<B: FitsBits<R::Integer>, R: $repr + Copy> Model for $name<B, R>
    where
      $integer<B, R::Integer>: Model,
    {
      const SIGNED: bool = $signed;
      const WIDTH: u32 = B::U32;

      fn pattern(self) -> Vec<bool> {
        $integer::from(self).pattern()
      }

      fn of_pattern(pattern: &[bool]) -> Self {
        Self::of_integer(<$integer<B, R::Integer>>::of_pattern(pattern)).expect("not zero")
      }
    }

    impl<B: FitsBits<R::Integer>, R: $repr + Copy> NonZeroModel for $name<B, R>
    where
      $integer<B, R::Integer>: Model,
    {
      type Integer = $integer<B, R::Integer>;

      fn of_integer(integer: $integer<B, R::Integer>) -> Option<Self> {
        Self::try_from(integer).ok()
      }
    }
  )+};
}

nonzero_model!(
  NonZeroUint<Uint>: NonZeroUintRepr, false;
  NonZeroInt<Int>: NonZeroIntRepr, true;
);

/// Implements [`Model`] and [`NonZeroModel`] for the native `NonZero` of each native integer,
/// through the model of the native integer.
macro_rules! native_nonzero_model {
  ($($native:ident),+ $(,)?) => {$(
    impl Model for NonZero<$native> {
      const SIGNED: bool = <$native as Model>::SIGNED;
      const WIDTH: u32 = $native::BITS;

      fn pattern(self) -> Vec<bool> {
        self.get().pattern()
      }

      fn of_pattern(pattern: &[bool]) -> Self {
        NonZero::new(<$native as Model>::of_pattern(pattern)).expect("not zero")
      }
    }

    impl NonZeroModel for NonZero<$native> {
      type Integer = $native;

      fn of_integer(integer: $native) -> Option<Self> {
        NonZero::new(integer)
      }
    }
  )+};
}

native_nonzero_model!(
  u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
);

/// The values of the non-zero integer `T` from which the conversions are checked: those of
/// [`samples`] of its integer but zero.
fn nonzero_samples<T: NonZeroModel>() -> Vec<T> {
  samples::<T::Integer>()
    .into_iter()
    .filter_map(T::of_integer)
    .collect()
}

/// Checks the pairs of `$source => $targets;`, at the values that `$sampler`, [`samples`] or
/// [`nonzero_samples`], gives for the source, each pair in an instantiation of `$check` of its own.
macro_rules! pairs {
  ($sampler:ident, $check:ident: $($source:ty => $($target:ty),+;)+) => {$({
    let samples = $sampler::<$source>();
    $($check::<$source, $target>(&samples);)+
  })+};
}

/// Checks `TryFrom` into the non-zero integer `T` of 8 bits of every value of its integer `I`, as
/// the tier takes them, against the native `TryFrom` into `NonZero<u8>`, or into `NonZero<i8>` for
/// a signed `T`: the same value, or the same error, of the kind `Zero`, for zero.
#[inline(never)]
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
#[inline(never)]
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

/// Checks `From` of each value of `samples` of the non-zero integer `S` into the non-zero integer
/// `T`, whose width holds every value of `S`, against the bit model: the value.
#[inline(never)]
fn check_from_nonzero<S: NonZeroModel, T: NonZeroModel + From<S>>(samples: &[S]) {
  for &value in samples {
    let expected = expected::<S, T::Integer>(value);
    let name = || format!("{value:?} into {}", type_name::<T>());
    assert!(expected.fits, "{}", name());
    assert_eq!(
      Some(T::from(value)),
      T::of_integer(expected.wrapped),
      "{}",
      name()
    );
  }
}

/// Checks `TryFrom` of each value of `samples` of the non-zero integer `S` into the non-zero
/// integer `T` against the bit model: the value, or the error that the native conversions return
/// for a value above the range of `T` or below it.
#[inline(never)]
fn check_try_from_nonzero<S, T>(samples: &[S])
where
  S: NonZeroModel,
  T: NonZeroModel + TryFrom<S, Error = TryFromIntError>,
{
  let (above, below) = errors();
  for &value in samples {
    let expected = expected::<S, T::Integer>(value);
    let converted = if expected.fits {
      Ok(T::of_integer(expected.wrapped).expect("not zero"))
    } else if S::SIGNED && model::sign(&value.pattern()) {
      Err(below)
    } else {
      Err(above)
    };
    assert_eq!(
      T::try_from(value),
      converted,
      "{value:?} into {}",
      type_name::<T>()
    );
  }
}

/// `From` of the native `NonZero` integers into the non-zero integers whose width holds every value
/// of them, in native storages and arrays: at least their width for the same sign, and one bit more
/// for an unsigned native integer into `NonZeroInt`, against the bit model.
#[test]
fn native_nonzero_integers_convert_into_nonzero_integers_where_lossless() {
  pairs!(nonzero_samples, check_from_nonzero:
    NonZero<u8> => NonZeroUint<U8, NonZero<u8>>, NonZeroUint<U8, [u8; 1]>,
      NonZeroUint<U200, [u64; 4]>, NonZeroInt<U9, NonZero<i16>>, NonZeroInt<U9, [u16; 1]>;
    NonZero<u16> => NonZeroUint<U16, [u8; 2]>, NonZeroUint<U33, [u32; 2]>,
      NonZeroInt<U17, NonZero<i32>>;
    NonZero<u32> => NonZeroUint<U32, NonZero<u32>>, NonZeroInt<U33, [u32; 2]>;
    NonZero<u64> => NonZeroUint<U64, NonZero<u64>>, NonZeroUint<U65, [u64; 2]>,
      NonZeroInt<U65, NonZero<i128>>;
    NonZero<u128> => NonZeroUint<U128, NonZero<u128>>, NonZeroInt<U129, [u64; 3]>;
    NonZero<i8> => NonZeroInt<U8, NonZero<i8>>, NonZeroInt<U8, [u8; 1]>,
      NonZeroInt<U100, [u128; 1]>;
    NonZero<i16> => NonZeroInt<U16, NonZero<i16>>, NonZeroInt<U24, [u8; 3]>;
    NonZero<i32> => NonZeroInt<U32, NonZero<i32>>, NonZeroInt<U33, [u32; 2]>;
    NonZero<i64> => NonZeroInt<U64, NonZero<i64>>, NonZeroInt<U200, [u64; 4]>;
    NonZero<i128> => NonZeroInt<U128, NonZero<i128>>, NonZeroInt<U129, [u64; 3]>;
  );
}

/// `TryFrom` of the native `NonZero` of `usize` and `isize` into the non-zero integers of every
/// width, zero bits included, which take no value, and of the non-zero integers of widths from one
/// bit to 200 and of every kind of storage into the native `NonZero` of each native integer,
/// against the bit model, with the errors of the native conversions.
#[test]
fn nonzero_and_native_nonzero_integers_try_from_each_other() {
  pairs!(nonzero_samples, check_try_from_nonzero:
    NonZero<usize> => NonZeroUint<U0>, NonZeroUint<U7, NonZero<u8>>, NonZeroUint<U16, [u8; 2]>,
      NonZeroUint<U64, NonZero<u64>>, NonZeroUint<U200, [u64; 4]>, NonZeroInt<U0, [u8; 0]>,
      NonZeroInt<U7, NonZero<i8>>, NonZeroInt<U64, NonZero<i64>>, NonZeroInt<U65, [u64; 2]>;
    NonZero<isize> => NonZeroUint<U0>, NonZeroUint<U7, NonZero<u8>>, NonZeroUint<U64, NonZero<u64>>,
      NonZeroUint<U200, [u64; 4]>, NonZeroInt<U1, NonZero<i8>>, NonZeroInt<U7, NonZero<i8>>,
      NonZeroInt<U33, [u32; 2]>, NonZeroInt<U64, NonZero<i64>>, NonZeroInt<U129, [u64; 3]>;
  );
  // Each non-zero integer of `$sources` into the native `NonZero` of each native integer.
  macro_rules! into_native_nonzeros {
    ($($source:ty),+ $(,)?) => {$(
      pairs!(nonzero_samples, check_try_from_nonzero:
        $source => NonZero<u8>, NonZero<u16>, NonZero<u32>, NonZero<u64>, NonZero<u128>,
          NonZero<usize>, NonZero<i8>, NonZero<i16>, NonZero<i32>, NonZero<i64>, NonZero<i128>,
          NonZero<isize>;
      );
    )+};
  }
  into_native_nonzeros!(
    NonZeroInt<U1, NonZero<i8>>,
    NonZeroUint<U7, NonZero<u8>>,
    NonZeroInt<U7, NonZero<i8>>,
    NonZeroInt<U12, [u16; 1]>,
    NonZeroUint<U65, [u64; 2]>,
    NonZeroInt<U129, [u64; 3]>,
    NonZeroUint<U200, [u8; 25]>,
  );
}

/// The error of a native conversion: a `TryFromIntError`, or `Infallible` for a conversion that
/// has `From`, whose `TryFrom` cannot fail.
trait NativeError {
  /// The error, as the `TryFromIntError` that it is.
  fn into_error(self) -> TryFromIntError;
}

impl NativeError for TryFromIntError {
  fn into_error(self) -> TryFromIntError {
    self
  }
}

impl NativeError for Infallible {
  fn into_error(self) -> TryFromIntError {
    match self {}
  }
}

/// Checks the conversions of each value of `samples` of the native `NonZero` integer `N` through
/// the non-zero integer `T` of the same width and sign into the native `NonZero` integer `Target`,
/// against the native conversion of `N` into `Target`: `From` into `T` and then `TryFrom` into
/// `Target` give what the native `TryFrom` gives, or the native `From` where it exists.
#[inline(never)]
fn check_against_native<N, T, Target>(samples: &[N])
where
  N: Copy + Debug,
  T: From<N>,
  Target: TryFrom<T, Error = TryFromIntError> + TryFrom<N> + Debug + PartialEq,
  <Target as TryFrom<N>>::Error: NativeError,
{
  for &value in samples {
    assert_eq!(
      Target::try_from(T::from(value)),
      Target::try_from(value).map_err(NativeError::into_error),
      "{value:?} into {}",
      type_name::<Target>()
    );
  }
}

/// Checks the conversions of each value of [`nonzero_samples`] of each native `NonZero` integer of
/// `$native => $nonzero;` through the non-zero integer `$nonzero` into the native `NonZero` of each
/// native integer, each target in an instantiation of [`check_against_native`] of its own.
macro_rules! against_native {
  ($($native:ty => $nonzero:ty;)+) => {$({
    let samples = nonzero_samples::<$native>();
    against_native!(
      @targets $native, $nonzero, samples;
      u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
    );
  })+};
  (@targets $native:ty, $nonzero:ty, $samples:ident; $($target:ident),+) => {
    $(check_against_native::<$native, $nonzero, NonZero<$target>>(&$samples);)+
  };
}

/// The non-zero integers of 8 and 16 bits, in a native storage and in arrays, of both signs,
/// convert with the native `NonZero` integers as the native `NonZero` integer of the same width
/// does: `From` of it into the non-zero integer and then `TryFrom` into the `NonZero` of each
/// native integer give what the native conversion between the `NonZero` integers gives.
#[test]
fn nonzero_integers_convert_as_the_native_nonzero_at_8_and_16_bits() {
  against_native!(
    NonZero<u8> => NonZeroUint<U8, NonZero<u8>>;
    NonZero<u8> => NonZeroUint<U8, [u8; 1]>;
    NonZero<i8> => NonZeroInt<U8, NonZero<i8>>;
    NonZero<i8> => NonZeroInt<U8, [u16; 1]>;
    NonZero<u16> => NonZeroUint<U16, NonZero<u16>>;
    NonZero<u16> => NonZeroUint<U16, [u8; 2]>;
    NonZero<i16> => NonZeroInt<U16, NonZero<i16>>;
    NonZero<i16> => NonZeroInt<U16, [u64; 1]>;
  );
}

/// Checks `TryFrom` of each value of `samples` of the native `NonZero` integer `N` into the
/// non-zero integer `T` against `TryFrom` into the native `NonZero` integer `Native` of the same
/// width and sign: the same value, or the same error.
#[inline(never)]
fn check_as_native<N, T, Native>(samples: &[N])
where
  N: Copy + Debug,
  T: Model + TryFrom<N, Error = TryFromIntError>,
  Native: Model + TryFrom<N, Error = TryFromIntError>,
{
  for &value in samples {
    assert_eq!(
      T::try_from(value).map(Model::pattern),
      Native::try_from(value).map(Model::pattern),
      "{value:?} into {}",
      type_name::<T>()
    );
  }
}

/// The conversions of the native `NonZero` of `usize` and `isize` into the non-zero integers of 8
/// and 16 bits, in native storages and arrays, give what those into the native `NonZero` integers
/// of the same width and sign give: the value, or the same error.
#[test]
fn pointer_sized_nonzero_integers_convert_as_the_native_nonzero() {
  let unsigned = nonzero_samples::<NonZero<usize>>();
  check_as_native::<_, NonZeroUint<U8, NonZero<u8>>, NonZero<u8>>(&unsigned);
  check_as_native::<_, NonZeroUint<U16, [u16; 1]>, NonZero<u16>>(&unsigned);
  check_as_native::<_, NonZeroInt<U8, [u8; 1]>, NonZero<i8>>(&unsigned);
  let signed = nonzero_samples::<NonZero<isize>>();
  check_as_native::<_, NonZeroUint<U8, [u64; 1]>, NonZero<u8>>(&signed);
  check_as_native::<_, NonZeroInt<U8, NonZero<i8>>, NonZero<i8>>(&signed);
  check_as_native::<_, NonZeroInt<U16, NonZero<i16>>, NonZero<i16>>(&signed);
}

/// The casts from the non-zero integers, of widths from one bit to 200 and of every kind of
/// storage, and from the native `NonZero` integers, into integers of each kind, the native integers
/// and `Uint` and `Int` of zero bits up to 200, against the bit model, and in an optimized build
/// also those of 256 and 1024 bits.
#[test]
fn nonzero_integers_cast_into_integers() {
  pairs!(nonzero_samples, check_cast:
    NonZeroInt<U1, NonZero<i8>> => u8, i8, Uint<U0>, Int<U1, [u8; 1]>;
    NonZeroUint<U7, NonZero<u8>> => u8, i8, usize, Int<U7, i8>, Uint<U3, [u16; 1]>;
    NonZeroInt<U7, NonZero<i8>> => u16, i8, isize, Uint<U7, u8>, Int<U200, [u64; 4]>;
    NonZeroUint<U8, [u8; 1]> => u8, i16, Int<U8, [u8; 1]>, Uint<U0>;
    NonZeroInt<U12, [u16; 1]> => u8, i8, i32, Int<U9, [u16; 1]>, Uint<U12, u16>;
    NonZeroUint<U33, [u32; 2]> => u32, i64, Int<U33, i64>, Uint<U12, [u8; 2]>;
    NonZeroInt<U64, NonZero<i64>> => u64, i64, usize, Int<U63, [u32; 2]>, Uint<U65, [u64; 2]>;
    NonZeroUint<U65, [u64; 2]> => u64, i128, Uint<U65, u128>, Int<U64, i64>;
    NonZeroInt<U100, [u128; 1]> => u128, i64, Int<U100, i128>, Uint<U129, [u64; 3]>;
    NonZeroUint<U128, NonZero<u128>> => u128, i128, isize, Int<U129, [u64; 3]>;
    NonZeroInt<U129, [u64; 3]> => i128, u64, Int<U129, [u8; 17]>, Uint<U128, u128>;
    NonZeroUint<U200, [u8; 25]> => u8, u128, Uint<U200, [u64; 4]>, Int<U150, [u64; 3]>;
    NonZero<u8> => u8, i8, Uint<U7, u8>, Int<U9, i16>;
    NonZero<i16> => i8, u16, Int<U12, [u16; 1]>, Uint<U16, [u8; 2]>;
    NonZero<u64> => u32, i64, Uint<U65, [u64; 2]>;
    NonZero<i128> => u128, i64, Int<U100, [u128; 1]>;
    NonZero<usize> => u16, isize, Uint<U33, [u32; 2]>;
    NonZero<isize> => i16, usize, Int<U64, [u16; 4]>;
  );
  if tier().is_full() {
    pairs!(nonzero_samples, check_cast:
      NonZeroUint<U1024, [u64; 16]> => Int<U256, [u8; 32]>, u64;
      NonZeroInt<U256, [u32; 8]> => Uint<U1024, [u128; 8]>, i8;
    );
  }
}

/// Checks the checked, the strict and the unchecked cast into the non-zero integer `T` of each
/// value of `samples` against the bit model: the value, if `T` holds it and it is not zero, and
/// otherwise `None`, where the strict cast panics, which
/// [`strict_casts_into_nonzero_integers_panic`] checks, and the unchecked one is not called.
#[inline(never)]
fn check_nonzero_target<S: Model, T: NonZeroModel + CheckedCastFrom<S>>(samples: &[S]) {
  for &value in samples {
    let expected = expected::<S, T::Integer>(value);
    let cast = if expected.fits {
      T::of_integer(expected.wrapped)
    } else {
      None
    };
    let name = || format!("{value:?} into {}", type_name::<T>());
    assert_eq!(T::checked_cast_from(value), cast, "{}", name());
    if let Some(cast) = cast {
      assert_eq!(T::strict_cast_from(value), cast, "{}", name());
      // SAFETY: `checked_cast_from` returns `Some`, as asserted above, which is the precondition of
      // `unchecked_cast_from`.
      let unchecked = unsafe { T::unchecked_cast_from(value) };
      assert_eq!(unchecked, cast, "{}", name());
    }
  }
}

/// The checked and the strict cast into the non-zero integers, of widths from zero bits, which take
/// no value, to 200 and of every kind of storage, and into the native `NonZero` integers, from
/// integers, which hold zero, non-zero integers and native integers and their `NonZero`, of both
/// signs, against the bit model.
#[test]
fn casts_into_nonzero_integers() {
  pairs!(samples, check_nonzero_target:
    Uint<U0> => NonZeroUint<U7, NonZero<u8>>, NonZero<u8>;
    Int<U1, i8> => NonZeroInt<U1, [u8; 1]>, NonZeroUint<U1, NonZero<u8>>, NonZero<i8>;
    Uint<U7, u8> => NonZeroUint<U7, [u8; 1]>, NonZeroInt<U7, NonZero<i8>>, NonZero<u8>,
      NonZero<i8>;
    Int<U12, [u16; 1]> => NonZeroInt<U9, [u16; 1]>, NonZeroUint<U16, [u8; 2]>, NonZero<i16>,
      NonZero<u8>;
    Uint<U65, [u64; 2]> => NonZeroUint<U65, NonZero<u128>>, NonZeroInt<U64, NonZero<i64>>,
      NonZero<u64>;
    Int<U129, [u64; 3]> => NonZeroInt<U129, [u8; 17]>, NonZeroUint<U128, [u64; 2]>, NonZero<i128>;
    Uint<U200, [u8; 25]> => NonZeroUint<U200, [u64; 4]>, NonZeroInt<U0, [u8; 0]>, NonZero<usize>;
    u8 => NonZeroUint<U8, NonZero<u8>>, NonZeroInt<U8, [u8; 1]>, NonZero<i8>, NonZero<u8>;
    i16 => NonZeroInt<U12, [u16; 1]>, NonZeroUint<U16, NonZero<u16>>, NonZero<isize>;
    u64 => NonZeroUint<U33, [u32; 2]>, NonZero<u32>, NonZero<i64>;
    i128 => NonZeroInt<U100, [u128; 1]>, NonZero<u128>;
    usize => NonZeroUint<U16, [u8; 2]>, NonZero<usize>, NonZero<i32>;
    isize => NonZeroInt<U64, NonZero<i64>>, NonZero<isize>;
  );
  pairs!(nonzero_samples, check_nonzero_target:
    NonZeroInt<U1, NonZero<i8>> => NonZeroInt<U1, [u8; 1]>, NonZeroUint<U1, NonZero<u8>>,
      NonZero<i8>;
    NonZeroUint<U7, NonZero<u8>> => NonZeroUint<U3, NonZero<u8>>, NonZeroInt<U7, [u8; 1]>,
      NonZero<u8>, NonZero<i8>;
    NonZeroInt<U12, [u16; 1]> => NonZeroInt<U8, NonZero<i8>>, NonZeroUint<U12, [u16; 1]>,
      NonZero<u16>;
    NonZeroUint<U65, [u64; 2]> => NonZeroUint<U64, NonZero<u64>>, NonZeroInt<U200, [u64; 4]>,
      NonZero<u128>;
    NonZeroInt<U129, [u64; 3]> => NonZeroInt<U65, [u32; 3]>, NonZeroUint<U129, [u16; 9]>,
      NonZero<i64>;
    NonZeroUint<U200, [u8; 25]> => NonZeroUint<U200, [u64; 4]>, NonZero<u8>;
    NonZero<u8> => NonZeroUint<U7, NonZero<u8>>, NonZeroInt<U8, NonZero<i8>>, NonZero<i8>;
    NonZero<i64> => NonZeroInt<U33, [u32; 2]>, NonZeroUint<U64, [u8; 8]>, NonZero<u16>;
    NonZero<usize> => NonZeroUint<U16, [u8; 2]>, NonZero<u8>;
    NonZero<isize> => NonZeroInt<U8, NonZero<i8>>, NonZero<usize>;
  );
}

/// Checks `widen` of each value of `samples` of the non-zero integer `S` into the non-zero integer
/// `T`, which holds every value of `S`, against the bit model: the value.
#[inline(never)]
fn check_nonzero_widen<S: NonZeroModel, T: NonZeroModel + WidenFrom<S>>(samples: &[S]) {
  for &value in samples {
    let expected = expected::<S, T::Integer>(value);
    let name = || format!("{value:?} into {}", type_name::<T>());
    assert!(expected.fits, "{}", name());
    assert_eq!(
      Some(T::widen_from(value)),
      T::of_integer(expected.wrapped),
      "{}",
      name()
    );
  }
}

/// `widen` from the non-zero integers and the native `NonZero` integers into the non-zero integers
/// of the same sign that are at least as wide, as the native `From` converts between the `NonZero`
/// integers, and into the integers of the same sign that are at least as wide, `usize` and
/// `isize` and their `NonZero` among them as 16 bits and as their own identities; and `truncate`
/// with its checked and saturating forms from them into the integers of the same sign that are at
/// most as wide; against the bit model.
#[test]
fn nonzero_integers_widen_and_truncate() {
  pairs!(nonzero_samples, check_nonzero_widen:
    NonZeroUint<U1, NonZero<u8>> => NonZeroUint<U1, [u8; 1]>, NonZero<u8>;
    NonZeroUint<U7, NonZero<u8>> => NonZeroUint<U8, [u8; 1]>, NonZero<u8>, NonZero<usize>,
      NonZeroUint<U200, [u64; 4]>;
    NonZeroInt<U7, NonZero<i8>> => NonZeroInt<U7, [u64; 1]>, NonZero<i8>, NonZero<isize>,
      NonZeroInt<U129, [u64; 3]>;
    NonZeroUint<U65, [u64; 2]> => NonZero<u128>, NonZeroUint<U65, NonZero<u128>>;
    NonZeroInt<U100, [u128; 1]> => NonZero<i128>, NonZeroInt<U200, [u8; 25]>;
    NonZero<u8> => NonZero<u16>, NonZero<usize>, NonZeroUint<U8, NonZero<u8>>,
      NonZeroUint<U33, [u32; 2]>;
    NonZero<i16> => NonZero<i32>, NonZero<isize>, NonZeroInt<U16, [u8; 2]>;
    NonZero<u64> => NonZero<u128>, NonZeroUint<U64, [u16; 4]>;
    NonZero<usize> => NonZero<usize>;
    NonZero<isize> => NonZero<isize>;
  );
  pairs!(nonzero_samples, check_widen:
    NonZeroUint<U7, NonZero<u8>> => u8, usize, Uint<U7, u8>, Uint<U200, [u8; 25]>;
    NonZeroInt<U12, [u16; 1]> => i16, isize, Int<U12, i16>;
    NonZeroUint<U128, NonZero<u128>> => u128, Uint<U129, [u64; 3]>;
    NonZero<u16> => u16, usize, Uint<U16, [u8; 2]>;
    NonZero<i64> => i64, i128, Int<U64, [u32; 2]>;
    NonZero<usize> => usize;
    NonZero<isize> => isize;
  );
  pairs!(nonzero_samples, check_truncate:
    NonZeroUint<U200, [u8; 25]> => u8, u64, Uint<U65, [u64; 2]>, Uint<U200, [u64; 4]>;
    NonZeroInt<U129, [u64; 3]> => i8, i128, Int<U7, i8>;
    NonZeroUint<U16, NonZero<u16>> => u8, u16, Uint<U0>;
    NonZero<u64> => u8, u64, Uint<U33, [u32; 2]>;
    NonZero<i32> => i8, i32, Int<U12, [u16; 1]>;
    NonZero<usize> => u8, u16, usize, Uint<U16, [u8; 2]>;
    NonZero<isize> => i8, i16, isize;
  );
}

/// The inherent casts of `NonZeroUint` and `NonZeroInt` give what the traits of the target give:
/// into integers and native integers, which hold zero, and into non-zero integers and the native
/// `NonZero` integers; and they infer the target from the expected type.
#[test]
fn inherent_nonzero_casts_agree_with_the_traits() {
  // The casts of `$x`, a value of `$ty`, inherent and through the traits: into each integer of
  // `$integers`, and the checked and the strict cast into each non-zero integer of `$nonzeros`,
  // where the checked one returns a value; the unchecked cast into both where the checked one
  // returns a value; each value in an `#[inline(never)]` function of its own, for the reason that
  // the module gives.
  macro_rules! agree {
    ($ty:ty = $x:expr => [$($integer:ty),+], [$($nonzero:ty),+]) => {{
      #[inline(never)]
      fn check(x: $ty) {
        $(
          assert_eq!(
            (
              x.checked_cast::<$integer>(),
              x.wrapping_cast::<$integer>(),
              x.saturating_cast::<$integer>(),
            ),
            (
              <$integer>::checked_cast_from(x),
              <$integer>::wrapping_cast_from(x),
              <$integer>::saturating_cast_from(x),
            ),
            "{x:?} into {}",
            type_name::<$integer>()
          );
          if let Some(checked) = x.checked_cast::<$integer>() {
            // SAFETY: `checked_cast` returns `Some`, which is the precondition of `unchecked_cast`.
            let unchecked = unsafe { x.unchecked_cast::<$integer>() };
            assert_eq!(unchecked, checked, "{x:?} into {}", type_name::<$integer>());
          }
        )+
        $(
          let checked = x.checked_cast::<$nonzero>();
          let name = || format!("{x:?} into {}", type_name::<$nonzero>());
          assert_eq!(checked, <$nonzero>::checked_cast_from(x), "{}", name());
          if let Some(checked) = checked {
            assert_eq!(x.strict_cast::<$nonzero>(), checked, "{}", name());
            // SAFETY: `checked_cast` returns `Some`, as asserted above, which is the precondition
            // of `unchecked_cast`.
            let unchecked = unsafe { x.unchecked_cast::<$nonzero>() };
            assert_eq!(unchecked, checked, "{}", name());
          }
        )+
      }
      check($x);
    }};
  }
  type Wide = NonZeroUint<U200, [u8; 25]>;
  agree!(NonZeroUint<U7> = NonZeroUint::<U7>::MAX =>
    [u8, i8, u7, i7, Uint<U3, u8>, isize],
    [NonZeroUint<U8>, NonZeroInt<U7>, NonZeroUint<U3>, NonZero<u8>, NonZero<i8>]);
  agree!(NonZeroInt<U7> = NonZeroInt::<U7>::MIN =>
    [u8, i8, u7, Int<U3, i8>, u256, usize],
    [NonZeroInt<U8, [u8; 1]>, NonZeroUint<U7>, NonZero<i8>, NonZero<u16>]);
  agree!(Wide = Wide::MAX =>
    [u128, i128, u7, i256, Uint<U200, [u64; 4]>],
    [NonZeroUint<U256>, NonZeroInt<U200>, NonZero<u128>]);
  agree!(NonZeroInt<U200> = NonZeroInt::<U200>::MIN =>
    [i128, u128, i7, i256],
    [NonZeroInt<U256>, NonZeroUint<U200>, NonZero<i64>]);

  let five = NonZeroUint::<U7>::new(u7::new(5)).expect("not zero");
  let wide: NonZeroUint<U16> = five.widen();
  assert_eq!(wide.get(), Uint::<U16>::new(5));
  let native: NonZero<u64> = five.widen();
  assert_eq!(native.get(), 5);
  let integer: u8 = five.widen();
  assert_eq!(integer, 5);
  assert_eq!(five.truncate::<Uint<U3, u8>>(), Uint::<U3, u8>::new(5));
  assert_eq!(NonZeroUint::<U16>::MAX.truncate::<u8>(), u8::MAX);
  assert_eq!(NonZeroUint::<U16>::MAX.checked_truncate::<u8>(), None);
  assert_eq!(NonZeroInt::<U16>::MIN.saturating_truncate::<i8>(), i8::MIN);
  assert_eq!(
    NonZeroInt::<U16>::MIN.truncate::<Int<U3, i8>>(),
    Int::<U3, i8>::ZERO
  );
}

/// The strict cast, inherent and through `CheckedCastFrom`, panics in every build on zero into a
/// non-zero integer, at zero bits too, with the text of the error of the kind `Zero` of the native
/// conversions, and on a value outside the range of the target, with the message of the native
/// `strict_cast`, and reports the caller: from integers, non-zero integers and native integers and
/// their `NonZero`, into non-zero integers and native `NonZero` integers, and from non-zero
/// integers into integers.
#[test]
fn strict_casts_into_nonzero_integers_panic() {
  let zero = "number would be zero for non-zero type".to_string();
  let overflow = "attempt to cast integer with overflow".to_string();
  type Byte = NonZeroUint<U8>;
  type Wide = NonZeroInt<U200, [u8; 25]>;
  panics_here!(u7::ZERO.strict_cast::<Byte>(), zero);
  panics_here!(i256::ZERO.strict_cast::<NonZero<i8>>(), zero);
  panics_here!(Uint::<U0>::ZERO.strict_cast::<NonZeroUint<U0>>(), zero);
  panics_here!(Byte::strict_cast_from(0u64), zero);
  panics_here!(NonZero::<u8>::strict_cast_from(0isize), zero);
  // A short argument keeps the invocation on one line, as `panics_here!` requires.
  let x = i7::ZERO;
  panics_here!(<Wide as CheckedCastFrom<i7>>::strict_cast_from(x), zero);
  panics_here!(u256::MAX.strict_cast::<Byte>(), overflow);
  panics_here!(i7::new(-1).strict_cast::<Byte>(), overflow);
  panics_here!(NonZero::<u8>::strict_cast_from(300u16), overflow);
  panics_here!(Wide::MIN.strict_cast::<NonZero<i128>>(), overflow);
  panics_here!(Byte::MAX.strict_cast::<NonZeroInt<U8>>(), overflow);
  panics_here!(NonZeroInt::<U7>::MIN.strict_cast::<u8>(), overflow);
  panics_here!(u8::strict_cast_from(NonZero::<i8>::MIN), overflow);
}

/// The unchecked cast into a non-zero integer, inherent and through `CheckedCastFrom`: the value
/// where the checked cast returns one, and on zero, which breaks its contract, a panic in every
/// build that reports the caller, as no non-zero integer of zero exists to return: from integers
/// of both signs into non-zero integers of every kind of storage, zero bits included, and into the
/// native `NonZero` integers.
#[test]
fn unchecked_casts_into_nonzero_integers() {
  // The unchecked cast of `value` into `T` through the trait, whose panic reports the caller of
  // this function.
  #[track_caller]
  fn through_the_trait<S, T: CheckedCastFrom<S>>(value: S) -> T {
    // SAFETY: a non-zero target checks for zero in every build and panics, and every other value
    // that this test passes is in the range of `T`, so no call has undefined behavior: the test
    // breaks the contract with zero to check the panic.
    unsafe { T::unchecked_cast_from(value) }
  }

  // The inherent unchecked cast of `value` into `NonZeroUint<U8>`, whose panic reports the caller
  // of this function.
  #[track_caller]
  fn from_u7(value: u7) -> NonZeroUint<U8> {
    // SAFETY: as for `through_the_trait`.
    unsafe { value.unchecked_cast() }
  }

  // The inherent unchecked cast of `value` into `NonZero<i8>`, whose panic reports the caller of
  // this function.
  #[track_caller]
  fn from_i256(value: i256) -> NonZero<i8> {
    // SAFETY: as for `through_the_trait`.
    unsafe { value.unchecked_cast() }
  }

  type Byte = NonZeroUint<U8>;
  type Wide = NonZeroInt<U200, [u8; 25]>;
  // A non-zero integer of zero bits, which holds no value, of each sign, and one in an array of
  // limbs; short names keep each invocation of `panics_here!` on one line, as it requires.
  type Empty = NonZeroUint<U0>;
  type EmptySigned = NonZeroInt<U0, [u8; 0]>;
  type Limbs = NonZeroUint<U200, [u64; 4]>;
  assert_eq!(Some(from_u7(u7::new(5))), Byte::new(Uint::<U8>::new(5)));
  assert_eq!(Some(from_i256(i256::from(-128))), NonZero::new(-128));
  assert_eq!(
    Some(through_the_trait::<i7, Wide>(i7::MIN)),
    Wide::checked_cast_from(i7::MIN)
  );
  assert_eq!(
    through_the_trait::<u64, NonZero<u16>>(65535),
    NonZero::<u16>::MAX
  );

  let zero = "`unchecked_cast` requires a value that is not zero for a non-zero target".to_string();
  panics_here!(from_u7(u7::ZERO), zero);
  panics_here!(from_i256(i256::ZERO), zero);
  panics_here!(through_the_trait::<u8, Byte>(0), zero);
  panics_here!(through_the_trait::<i7, Wide>(i7::ZERO), zero);
  panics_here!(through_the_trait::<isize, NonZero<i64>>(0), zero);
  panics_here!(through_the_trait::<Uint<U0>, Empty>(Uint::ZERO), zero);
  panics_here!(through_the_trait::<u8, Limbs>(0), zero);
  panics_here!(through_the_trait::<i64, NonZeroInt<U7>>(0), zero);
  panics_here!(through_the_trait::<u16, NonZero<u8>>(0), zero);
  panics_here!(through_the_trait::<i128, NonZero<usize>>(0), zero);
  panics_here!(through_the_trait::<u8, EmptySigned>(0), zero);
}

/// `try_cast` into a non-zero integer returns the error of the kind `Zero` for zero, at zero bits
/// too, as the native conversions do, and those of the kinds `PosOverflow` and `NegOverflow` for a
/// value outside the range, which it checks first.
#[test]
fn try_cast_into_nonzero_integers() {
  let (above, below) = errors();
  let zero = zero_error();
  let one = NonZeroUint::<U200>::new(Uint::<U200>::ONE);
  assert_eq!(try_cast::<_, NonZeroUint<U8>>(Uint::<U8>::ZERO), Err(zero));
  assert_eq!(try_cast::<_, NonZero<i64>>(0u8), Err(zero));
  assert_eq!(try_cast::<_, NonZeroUint<U0>>(Uint::<U0>::ZERO), Err(zero));
  assert_eq!(try_cast::<_, NonZeroInt<U7>>(64i64), Err(above));
  assert_eq!(try_cast::<_, NonZero<u8>>(-1i8), Err(below));
  assert_eq!(try_cast::<_, NonZero<u16>>(NonZero::<u32>::MAX), Err(above));
  assert_eq!(try_cast::<_, NonZeroUint<U200>>(1u8).ok(), one);
}

/// The checked cast of `x` into a `u64`, in code generic over the width alone.
fn nonzero_into_u64<B: BitWidth>(x: NonZeroUint<B>) -> Option<u64> {
  x.checked_cast()
}

/// The checked cast of `n` into `NonZeroInt<B>`, in code generic over the width alone.
fn nonzero_from_i64<B: BitWidth>(n: i64) -> Option<NonZeroInt<B>> {
  NonZeroInt::<B>::checked_cast_from(n)
}

/// The casts in code generic over the width, whose bound `B: BitWidth` alone gives
/// `NonZeroUint<B>` and `NonZeroInt<B>` the casts with the native integers, at widths from zero
/// bits, which take no value, to 200.
#[test]
fn nonzero_casts_in_code_generic_over_the_width() {
  assert_eq!(nonzero_into_u64(NonZeroUint::<U1>::MIN), Some(1));
  assert_eq!(nonzero_into_u64(NonZeroUint::<U64>::MAX), Some(u64::MAX));
  assert_eq!(nonzero_into_u64(NonZeroUint::<U65>::MAX), None);
  assert_eq!(nonzero_into_u64(NonZeroUint::<U200>::MIN), Some(1));
  assert_eq!(nonzero_from_i64::<U0>(5), None);
  assert_eq!(nonzero_from_i64::<U1>(-1), Some(NonZeroInt::<U1>::MIN));
  assert_eq!(nonzero_from_i64::<U7>(0), None);
  assert_eq!(nonzero_from_i64::<U7>(-64), Some(NonZeroInt::<U7>::MIN));
  assert_eq!(nonzero_from_i64::<U7>(64), None);
  assert_eq!(
    nonzero_from_i64::<U200>(i64::MIN).map(Int::from),
    Some(Int::<U200>::from(i64::MIN))
  );
}
