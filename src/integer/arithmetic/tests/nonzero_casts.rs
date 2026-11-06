//! The conversions of the non-zero integers `NonZeroUint` and `NonZeroInt`: `TryFrom` of the
//! integer of the same width and storage, which returns the error of the kind `Zero` for zero, at
//! every value of 8 bits against the native `NonZero` integers, and at the values of the casts in
//! every kind of storage against the bit model; and the conversions with the native `NonZero`
//! integers, both ways, against the bit model and, at 8 and 16 bits, against the conversions
//! between the native `NonZero` integers. Each type or pair checks in an instantiation of a generic
//! function of its own, whose frame holds its values alone.

use core::{
  any::type_name,
  convert::Infallible,
  num::{NonZero, TryFromIntError},
};

use typenum::{U12, U17, U33};

use super::{
  casts::{Model, errors, expected, samples},
  *,
};
use crate::{NonZeroInt, NonZeroIntRepr, NonZeroUint, NonZeroUintRepr};

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

/// Checks `From` of each value of `samples` of the non-zero integer `S` into the non-zero integer
/// `T`, whose width holds every value of `S`, against the bit model: the value.
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
