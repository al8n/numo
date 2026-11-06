//! The standard conversions of [`Uint`] and [`Int`] with the native integers and `bool`, by
//! direction: `From` from `u8` to `u128` and `i8` to `i128` where the width holds every value of the
//! native integer, which the sealed marker `HoldsEvery` checks, and `TryFrom` from `usize` and
//! `isize` at every width; `TryFrom` into each native integer at every width; and `From` from
//! `bool` where the width holds one, and `TryFrom` into `bool`. `TryFrom` converts an integer into
//! the non-zero integer of the same width and storage, [`NonZeroUint`] or [`NonZeroInt`], with the
//! error of the kind `Zero` for zero, as the native `NonZero` integers convert from their integers.
//! The non-zero integers convert with the native `NonZero` integers by the directions of the
//! integers with the native integers, through the conversion of the integer of the same value,
//! whose result is not zero either.
//!
//! Each conversion goes through the casts, and the errors are the values of
//! `core::num::TryFromIntError` that the native conversions return: the type has no public
//! constructor and its conversions are not `const`, so cold functions make them at run time from
//! the native conversions themselves. Between numo integers, the casts are the conversions: `From`
//! and `TryFrom` between them would overlap the implementations of the standard library from a type
//! to itself and from every `From`.

use core::num::TryFromIntError;

use typenum::{Gr, GrEq, IsGreater, IsGreaterOrEqual, NonZero, True, U1, U8, U16, U32, U64, U128};

use sealed::HoldsEvery;

use super::{BoundedCastFrom, Int, NonZeroInt, NonZeroUint, Uint, cast, nonzero::Parts};
use crate::{
  layout::Token,
  width::{FitsBits, IntCore, IntRepr, NonZeroIntRepr, NonZeroUintRepr, UintCore, UintRepr},
};

mod sealed {
  /// Implemented only for `B1`, the result of the comparison of `Bits` with the width of the native
  /// integer that a `From` converts from: at least that width for an integer of the same sign, and
  /// one bit more for an unsigned native integer into `Int`.
  #[diagnostic::on_unimplemented(
    message = "the width cannot hold every value of the integer converted from",
    label = "`Bits` is too narrow for a lossless conversion",
    note = "a narrower width converts with `checked_cast_from` and the other casts"
  )]
  pub trait HoldsEvery {}

  impl HoldsEvery for typenum::B1 {}
}

/// The error of a value above the largest value of the target, of the kind `PosOverflow`, as the
/// native conversions return it.
#[cold]
pub(crate) fn pos_overflow() -> TryFromIntError {
  match u8::try_from(u16::MAX) {
    Err(error) => error,
    Ok(_) => unreachable!("`u16::MAX` fits in a `u8`"),
  }
}

/// The error of a value below the smallest value of the target, of the kind `NegOverflow`, as the
/// native conversions return it: a negative value for an unsigned target.
#[cold]
pub(crate) fn neg_overflow() -> TryFromIntError {
  match u8::try_from(-1i8) {
    Err(error) => error,
    Ok(_) => unreachable!("-1 fits in a `u8`"),
  }
}

/// The error of zero for a non-zero target, of the kind `Zero`, as the native conversions return
/// it.
#[cold]
pub(crate) fn zero() -> TryFromIntError {
  match core::num::NonZero::<u8>::try_from(0u8) {
    Err(error) => error,
    Ok(_) => unreachable!("0 converts to a `NonZero<u8>`"),
  }
}

/// Implements `From` of the native integers of `$natives`, written `native: Bits`, into `$integer`
/// over every storage of `$repr` whose width compares with `Bits` as `$compare`, of which `$result`
/// names the result: at least the width for an integer of the same sign, and greater for an
/// unsigned native integer into `Int`, which also holds the sign.
macro_rules! from_natives {
  (
    $integer:ident: $repr:ident, $compare:ident, $result:ident;
    $($native:ident: $bits:ident),+
  ) => {$(
    impl<B, R> From<$native> for $integer<B, R>
    where
      R: $repr,
      B: FitsBits<R> + $compare<$bits>,
      $result<B, $bits>: HoldsEvery,
    {
      /// Converts `value`, which the width holds, as every value of the native integer.
      #[inline]
      fn from(value: $native) -> Self {
        Self::wrapping_cast_from(value)
      }
    }
  )+};
}

from_natives!(
  Uint: UintRepr, IsGreaterOrEqual, GrEq;
  u8: U8, u16: U16, u32: U32, u64: U64, u128: U128
);
from_natives!(
  Int: IntRepr, IsGreater, Gr;
  u8: U8, u16: U16, u32: U32, u64: U64, u128: U128
);
from_natives!(
  Int: IntRepr, IsGreaterOrEqual, GrEq;
  i8: U8, i16: U16, i32: U32, i64: U64, i128: U128
);

/// Implements `TryFrom` of `usize` and `isize` into `$integer` over every storage of `$repr`, at
/// every width, as the native integers convert from them.
macro_rules! try_from_pointer_sized {
  ($($integer:ident: $repr:ident),+) => {$(
    impl<B: FitsBits<R>, R: $repr> TryFrom<usize> for $integer<B, R> {
      type Error = TryFromIntError;

      /// Converts `value`, or returns the error of the native conversions, of the kind
      /// `PosOverflow`, if it is above `MAX`.
      #[inline]
      fn try_from(value: usize) -> Result<Self, TryFromIntError> {
        cast::try_cast(value)
      }
    }

    impl<B: FitsBits<R>, R: $repr> TryFrom<isize> for $integer<B, R> {
      type Error = TryFromIntError;

      /// Converts `value`, or returns the error of the native conversions if it is outside the
      /// range of `Bits` bits: of the kind `PosOverflow` above `MAX`, and `NegOverflow` below
      /// `MIN`.
      #[inline]
      fn try_from(value: isize) -> Result<Self, TryFromIntError> {
        cast::try_cast(value)
      }
    }
  )+};
}

try_from_pointer_sized!(Uint: UintRepr, Int: IntRepr);

/// Implements `TryFrom` of [`Uint`] and [`Int`] over every storage into each native integer of
/// `$natives`, at every width, as the native integers convert into one another.
macro_rules! natives_try_from {
  ($($native:ident),+) => {$(
    impl<B: FitsBits<R>, R: UintRepr> TryFrom<Uint<B, R>> for $native {
      type Error = TryFromIntError;

      /// Converts `value`, or returns the error of the native conversions, of the kind
      /// `PosOverflow`, if it is above the largest value of the native integer.
      #[inline]
      fn try_from(value: Uint<B, R>) -> Result<Self, TryFromIntError> {
        cast::try_cast(value)
      }
    }

    impl<B: FitsBits<R>, R: IntRepr> TryFrom<Int<B, R>> for $native {
      type Error = TryFromIntError;

      /// Converts `value`, or returns the error of the native conversions if it is outside the
      /// range of the native integer: of the kind `PosOverflow` above it, and `NegOverflow` below
      /// it, as every negative value is for an unsigned integer.
      #[inline]
      fn try_from(value: Int<B, R>) -> Result<Self, TryFromIntError> {
        cast::try_cast(value)
      }
    }
  )+};
}

natives_try_from!(
  u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
);

// `bool` converts to zero and one, which `Uint` holds from one bit and `Int` from two, as `ONE`
// exists.
impl<B: FitsBits<R> + NonZero, R: UintRepr> From<bool> for Uint<B, R> {
  /// Converts `false` to zero and `true` to one, as the native integers do.
  #[inline]
  fn from(value: bool) -> Self {
    Self::wrapping_cast_from(u8::from(value))
  }
}

impl<B: FitsBits<R> + IsGreater<U1, Output = True>, R: IntRepr> From<bool> for Int<B, R> {
  /// Converts `false` to zero and `true` to one, as the native integers do.
  #[inline]
  fn from(value: bool) -> Self {
    Self::wrapping_cast_from(u8::from(value))
  }
}

/// Implements `TryFrom` of `$integer` over every storage of `$repr` into `bool`, at every width, as
/// `bool` converts from the native integers.
macro_rules! bool_try_from {
  ($($integer:ident: $repr:ident),+) => {$(
    impl<B: FitsBits<R>, R: $repr> TryFrom<$integer<B, R>> for bool {
      type Error = TryFromIntError;

      /// Converts zero to `false` and one to `true`, or returns the error of the native
      /// conversions: of the kind `PosOverflow` above one, and `NegOverflow` below zero.
      #[inline]
      fn try_from(value: $integer<B, R>) -> Result<Self, TryFromIntError> {
        match cast::try_cast::<_, u8>(value)? {
          0 => Ok(false),
          1 => Ok(true),
          _ => Err(pos_overflow()),
        }
      }
    }
  )+};
}

bool_try_from!(Uint: UintRepr, Int: IntRepr);

/// Implements `TryFrom` of the integer `$integer`, whose storages implement `$core`, into the
/// non-zero integer `$name` of the same width over every storage of `$repr`, written
/// `$name: $repr, $integer, $core;`, as the native `NonZero` integers convert from their integers.
/// The storage of the value becomes that of the non-zero integer once its zero test passes, so no
/// width is read, as for `From` back to the integer.
macro_rules! nonzero_try_from_integer {
  ($($name:ident: $repr:ident, $integer:ident, $core:ident);+ $(;)?) => {$(
    impl<B, R: $repr> TryFrom<$integer<B, R::Integer>> for $name<B, R> {
      type Error = TryFromIntError;

      /// Converts `value`, or returns the error of the native conversions, of the kind `Zero`, if
      /// it is zero.
      #[inline]
      fn try_from(value: $integer<B, R::Integer>) -> Result<Self, TryFromIntError> {
        match <R::Integer as $core>::to_nonzero(value.repr, Token::new()) {
          Some(repr) => Ok(Self {
            value: $integer::wrap(repr),
          }),
          None => Err(zero()),
        }
      }
    }
  )+};
}

nonzero_try_from_integer!(
  NonZeroUint: NonZeroUintRepr, Uint, UintCore;
  NonZeroInt: NonZeroIntRepr, Int, IntCore;
);

/// Implements `From` of the native `NonZero` of the native integers of `$natives`, written
/// `native: Bits`, into the non-zero integer `$name` of the integer `$integer` over every storage
/// of `$repr` whose width compares with `Bits` as `$compare`, of which `$result` names the result,
/// as `From` of the native integers converts into `$integer`: the integer of the value, which is
/// not zero.
macro_rules! nonzero_from_natives {
  (
    $name:ident, $integer:ident: $repr:ident, $compare:ident, $result:ident;
    $($native:ident: $bits:ident),+
  ) => {$(
    impl<B, R> From<core::num::NonZero<$native>> for $name<B, R>
    where
      R: $repr,
      B: FitsBits<R::Integer> + $compare<$bits>,
      $result<B, $bits>: HoldsEvery,
    {
      /// Converts `value`, which the width holds, as every value of the native integer.
      #[inline]
      fn from(value: core::num::NonZero<$native>) -> Self {
        Self::known(<$integer<B, R::Integer>>::from(value.get()))
      }
    }
  )+};
}

nonzero_from_natives!(
  NonZeroUint, Uint: NonZeroUintRepr, IsGreaterOrEqual, GrEq;
  u8: U8, u16: U16, u32: U32, u64: U64, u128: U128
);
nonzero_from_natives!(
  NonZeroInt, Int: NonZeroIntRepr, IsGreater, Gr;
  u8: U8, u16: U16, u32: U32, u64: U64, u128: U128
);
nonzero_from_natives!(
  NonZeroInt, Int: NonZeroIntRepr, IsGreaterOrEqual, GrEq;
  i8: U8, i16: U16, i32: U32, i64: U64, i128: U128
);

/// Implements `TryFrom` of the native `NonZero` of `usize` and `isize` into the non-zero integer
/// `$name` of the integer `$integer` over every storage of `$repr`, written
/// `$name: $repr, $integer;`, at every width, as `TryFrom` of `usize` and `isize` converts into
/// `$integer`: the integer of the value, which is not zero.
macro_rules! nonzero_try_from_pointer_sized {
  ($($name:ident: $repr:ident, $integer:ident);+ $(;)?) => {$(
    impl<B: FitsBits<R::Integer>, R: $repr> TryFrom<core::num::NonZero<usize>> for $name<B, R> {
      type Error = TryFromIntError;

      /// Converts `value`, or returns the error of the native conversions, of the kind
      /// `PosOverflow`, if it is above `MAX`.
      #[inline]
      fn try_from(value: core::num::NonZero<usize>) -> Result<Self, TryFromIntError> {
        <$integer<B, R::Integer>>::try_from(value.get()).map(Self::known)
      }
    }

    impl<B: FitsBits<R::Integer>, R: $repr> TryFrom<core::num::NonZero<isize>> for $name<B, R> {
      type Error = TryFromIntError;

      /// Converts `value`, or returns the error of the native conversions if it is outside the
      /// range of `Bits` bits: of the kind `PosOverflow` above `MAX`, and `NegOverflow` below
      /// `MIN`.
      #[inline]
      fn try_from(value: core::num::NonZero<isize>) -> Result<Self, TryFromIntError> {
        <$integer<B, R::Integer>>::try_from(value.get()).map(Self::known)
      }
    }
  )+};
}

nonzero_try_from_pointer_sized!(
  NonZeroUint: NonZeroUintRepr, Uint;
  NonZeroInt: NonZeroIntRepr, Int;
);

/// Implements `TryFrom` of [`NonZeroUint`] and [`NonZeroInt`] over every storage into the native
/// `NonZero` of each native integer of `$natives`, at every width, as `TryFrom` of the integers
/// converts into the native integers: the native integer of the integer of the value, which is
/// not zero.
macro_rules! native_nonzeros_try_from {
  ($($native:ident),+) => {$(
    impl<B: FitsBits<R::Integer>, R: NonZeroUintRepr> TryFrom<NonZeroUint<B, R>>
      for core::num::NonZero<$native>
    {
      type Error = TryFromIntError;

      /// Converts `value`, or returns the error of the native conversions, of the kind
      /// `PosOverflow`, if it is above the largest value of the native integer.
      #[inline]
      fn try_from(value: NonZeroUint<B, R>) -> Result<Self, TryFromIntError> {
        $native::try_from(Uint::from(value)).and_then(core::num::NonZero::try_from)
      }
    }

    impl<B: FitsBits<R::Integer>, R: NonZeroIntRepr> TryFrom<NonZeroInt<B, R>>
      for core::num::NonZero<$native>
    {
      type Error = TryFromIntError;

      /// Converts `value`, or returns the error of the native conversions if it is outside the
      /// range of the native integer: of the kind `PosOverflow` above it, and `NegOverflow` below
      /// it, as every negative value is for an unsigned integer.
      #[inline]
      fn try_from(value: NonZeroInt<B, R>) -> Result<Self, TryFromIntError> {
        $native::try_from(Int::from(value)).and_then(core::num::NonZero::try_from)
      }
    }
  )+};
}

native_nonzeros_try_from!(
  u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
);
