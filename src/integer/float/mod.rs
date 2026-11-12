//! The conversions of [`Uint`] and [`Int`] with the floats: the methods of every storage, which
//! `float_methods!` emits as inherent `const fn`s, the trait [`IntegerFloat`] that gives them to
//! generic code, the panic of the strict conversions, and `From` into `f64` and `f32` at the widths
//! whose every value the float holds.
//!
//! An integer converts to the nearest float, ties to even, as `as` converts the native integers,
//! through the sealed `float_bits` of its storage, and by `From` where the float holds every value
//! of its width, which the sealed marker `HoldsExactly` checks. A float converts through its
//! parts, its value rounded to an integer with a rounding mode, and the sealed `from_float` of the
//! storage, which checks the range of the width and places the integer: the truncating
//! conversions round with `Down`, as the native ones do, and the exact one judges the range with
//! `Up`, which keeps every value of the range in it and moves every value outside it farther out,
//! before it rejects a fraction.

use typenum::{IsLessOrEqual, LeEq, U24, U25, U53, U54};

use sealed::HoldsExactly;

use super::{Int, Uint, cast, traits::Sealed};
use crate::{
  FromFloatError, RoundingMode,
  int::{self, float::FloatFormat},
  layout::Token,
  width::{FitsBits, IntCore, IntRepr, UintCore, UintRepr},
};

mod sealed {
  /// Implemented only for `B1`, the result of the comparison of `Bits` with the width whose every
  /// value a float holds exactly: the 53 bits of the significand of `f64` and the 24 of `f32` for
  /// `Uint`, and one more for `Int`, whose largest magnitude is `2^(Bits - 1)`.
  #[diagnostic::on_unimplemented(
    message = "the width has more bits than the float holds exactly",
    label = "`Bits` is above 53 for `f64` or 24 for `f32` (54 and 25 for `Int`)",
    note = "`to_f64` and `to_f32` round any width to the nearest float"
  )]
  pub trait HoldsExactly {}

  impl HoldsExactly for typenum::B1 {}
}

/// Panics on a float that a strict conversion cannot convert, with the message of the native
/// `to_int_strict`, reporting the caller.
#[cold]
#[track_caller]
pub(crate) const fn strict_float() -> ! {
  panic!("the value cannot be represented in the target integer type")
}

/// The `f64` whose bits a storage writes.
#[inline]
pub(super) const fn f64_of(bits: u64) -> f64 {
  f64::from_bits(bits)
}

/// The `f32` whose bits a storage writes, in the low 32 bits.
#[inline]
pub(super) const fn f32_of(bits: u64) -> f32 {
  f32::from_bits(bits as u32)
}

/// Emits the conversions of `Uint` and `Int` with the floats as inherent `const fn`s of one
/// storage, in an impl block whose width relation holds: `$args` is `[layer]` for a native integer,
/// which its layer in `int` takes by value, or `[group, layer, &]` for an array, which the layer of
/// its sign takes by reference, as for `inherent_items!`.
// The conversions from a float have no operand of the storage, which the rows of the tables need,
// so the methods are written here once for every storage.
macro_rules! float_methods {
  // The function `$f` of the layer of the sign of the storage.
  (@layer [$group:ident, $layer:ident, &] $f:ident($($arg:expr),*)) => {
    int::$group::$layer::$f($($arg),*)
  };
  (@layer [$layer:ident] $f:ident($($arg:expr),*)) => {
    int::$layer::$f($($arg),*)
  };
  // The function `$f` of that layer on the storage of `$value`.
  (@own [$group:ident, $layer:ident, &] $f:ident($value:expr $(, $arg:expr)*)) => {
    int::$group::$layer::$f(&$value.repr $(, $arg)*)
  };
  (@own [$layer:ident] $f:ident($value:expr $(, $arg:expr)*)) => {
    int::$layer::$f($value.repr $(, $arg)*)
  };
  ($args:tt) => {
    /// The float of `parts` rounded to an integer with `mode`, or the error of a value outside the
    /// range of `Bits` bits.
    #[inline]
    const fn round_parts(
      parts: int::float::FloatParts,
      mode: $crate::RoundingMode,
    ) -> Result<Self, $crate::FromFloatError> {
      let rounded = int::float::round_magnitude(parts, mode);
      match float_methods!(@layer $args place_magnitude(rounded, Self::BITS)) {
        Ok(repr) => Ok(Self::wrap(repr)),
        Err(error) => Err(error),
      }
    }

    /// The float of `parts` if it is an integer in the range of `Bits` bits, or the first error of
    /// a value outside the range, judged on the value itself, and of a fraction.
    #[inline]
    const fn exact_parts(parts: int::float::FloatParts) -> Result<Self, $crate::FromFloatError> {
      // `Up` rounds to the integer next to the value away from zero. The bounds of the range are
      // integers on either side of zero, so the integer of a value of the range is in it, and that
      // of a value outside it is outside it on the same side.
      match Self::round_parts(parts, $crate::RoundingMode::Up) {
        Ok(_) if !parts.is_integer() => Err($crate::FromFloatError::Inexact),
        result => result,
      }
    }

    /// Converts the value to the nearest `f64`, ties to even, as `as` converts the native integers.
    /// A magnitude of at most 53 bits, the precision of `f64`, converts exactly, as every value of
    /// a width of at most 53 bits for `Uint` and 54 for `Int` does. A magnitude that rounds to
    /// `2^1024` or more, which only a width of more than 1023 bits holds, gives the infinity of the
    /// sign of the value, and zero gives `+0.0`.
    #[must_use]
    #[inline]
    pub const fn to_f64(&self) -> f64 {
      float::f64_of(float_methods!(@own $args float_bits(self, int::float::FloatFormat::F64)))
    }

    /// Converts the value to the nearest `f32`, ties to even, as `as` converts the native integers
    /// and as [`to_f64`](Self::to_f64) converts it to an `f64`: a magnitude of at most 24 bits,
    /// the precision of `f32`, converts exactly, as every value of a width of at most 24 bits for
    /// `Uint` and 25 for `Int` does, and a magnitude that rounds to `2^128` or more, as that of
    /// `u128::MAX` does, gives the infinity of the sign of the value.
    #[must_use]
    #[inline]
    pub const fn to_f32(&self) -> f32 {
      float::f32_of(float_methods!(@own $args float_bits(self, int::float::FloatFormat::F32)))
    }

    /// Converts `value`, truncated toward zero, or returns `None` if it is NaN or an infinity, or
    /// if the truncated value is outside the range of `Bits` bits, as the native `to_int_checked`
    /// does, still unstable ([#159913](https://github.com/rust-lang/rust/issues/159913)): `-0.9`
    /// converts to zero, and `255.5` to 255 at 8 bits.
    #[must_use]
    #[inline]
    pub const fn checked_from_f64(value: f64) -> Option<Self> {
      match Self::round_from_f64(value, $crate::RoundingMode::Down) {
        Ok(integer) => Some(integer),
        Err(_) => None,
      }
    }

    /// Converts `value` as [`checked_from_f64`](Self::checked_from_f64) converts the `f64` of the
    /// same value, which every `f32` has.
    #[must_use]
    #[inline]
    pub const fn checked_from_f32(value: f32) -> Option<Self> {
      match Self::round_from_f32(value, $crate::RoundingMode::Down) {
        Ok(integer) => Some(integer),
        Err(_) => None,
      }
    }

    /// Converts `value`, truncated toward zero, or the bound of the range of `Bits` bits that it
    /// passes, as `as` converts a float to the native integers, and as the native
    /// `to_int_saturating` does, still unstable
    /// ([#159913](https://github.com/rust-lang/rust/issues/159913)): [`MAX`](Self::MAX) above the
    /// range and for positive infinity, [`MIN`](Self::MIN) below it and for negative infinity, and
    /// zero for NaN.
    #[must_use]
    #[inline]
    pub const fn saturating_from_f64(value: f64) -> Self {
      match Self::round_from_f64(value, $crate::RoundingMode::Down) {
        Ok(integer) => integer,
        Err($crate::FromFloatError::Nan) => Self::ZERO,
        // An infinity, or a value outside the range, passes the bound of its sign.
        Err(_) if value.is_sign_negative() => Self::MIN,
        Err(_) => Self::MAX,
      }
    }

    /// Converts `value` as [`saturating_from_f64`](Self::saturating_from_f64) converts the `f64`
    /// of the same value, which every `f32` has.
    #[must_use]
    #[inline]
    pub const fn saturating_from_f32(value: f32) -> Self {
      match Self::round_from_f32(value, $crate::RoundingMode::Down) {
        Ok(integer) => integer,
        Err($crate::FromFloatError::Nan) => Self::ZERO,
        // An infinity, or a value outside the range, passes the bound of its sign.
        Err(_) if value.is_sign_negative() => Self::MIN,
        Err(_) => Self::MAX,
      }
    }

    /// Converts `value`, truncated toward zero, as [`checked_from_f64`](Self::checked_from_f64)
    /// does and as the native `to_int_strict` does, still unstable
    /// ([#159913](https://github.com/rust-lang/rust/issues/159913)).
    ///
    /// # Panics
    ///
    /// Panics in every build, reporting the caller, if `value` is NaN or an infinity, or if the
    /// truncated value is outside the range of `Bits` bits, with the message of the native
    /// `to_int_strict`; `checked_from_f64` returns `None` instead.
    #[must_use]
    #[inline]
    #[track_caller]
    pub const fn strict_from_f64(value: f64) -> Self {
      match Self::checked_from_f64(value) {
        Some(integer) => integer,
        None => float::strict_float(),
      }
    }

    /// Converts `value` as [`strict_from_f64`](Self::strict_from_f64) converts the `f64` of the
    /// same value, which every `f32` has.
    ///
    /// # Panics
    ///
    /// Panics in every build, reporting the caller, if `value` is NaN or an infinity, or if the
    /// truncated value is outside the range of `Bits` bits;
    /// [`checked_from_f32`](Self::checked_from_f32) returns `None` instead.
    #[must_use]
    #[inline]
    #[track_caller]
    pub const fn strict_from_f32(value: f32) -> Self {
      match Self::checked_from_f32(value) {
        Some(integer) => integer,
        None => float::strict_float(),
      }
    }

    /// Converts `value` rounded to an integer with `mode`, as a
    /// [`RoundingMode`](crate::RoundingMode) rounds a decimal: `Down` truncates, as the other
    /// conversions do, and `HalfEven` rounds to the nearest integer, ties to even, at every
    /// magnitude, so `0.5` rounds to zero and `1.5` to two. A value that is an integer converts
    /// unchanged, and a negative value that rounds to zero converts to zero.
    ///
    /// # Errors
    ///
    /// Returns [`FromFloatError::Nan`](crate::FromFloatError::Nan) for NaN,
    /// [`Infinite`](crate::FromFloatError::Infinite) for an infinity, and for a rounded value
    /// outside the range of `Bits` bits, [`PosOverflow`](crate::FromFloatError::PosOverflow) above
    /// [`MAX`](Self::MAX) and [`NegOverflow`](crate::FromFloatError::NegOverflow) below
    /// [`MIN`](Self::MIN), as every negative rounded value is for `Uint`.
    #[inline]
    pub const fn round_from_f64(
      value: f64,
      mode: $crate::RoundingMode,
    ) -> Result<Self, $crate::FromFloatError> {
      match int::float::float_parts_f64(value) {
        Ok(parts) => Self::round_parts(parts, mode),
        Err(error) => Err(error),
      }
    }

    /// Converts `value` rounded to an integer with `mode`, as
    /// [`round_from_f64`](Self::round_from_f64) converts the `f64` of the same value, which every
    /// `f32` has.
    ///
    /// # Errors
    ///
    /// Returns the errors of `round_from_f64`.
    #[inline]
    pub const fn round_from_f32(
      value: f32,
      mode: $crate::RoundingMode,
    ) -> Result<Self, $crate::FromFloatError> {
      match int::float::float_parts_f32(value) {
        Ok(parts) => Self::round_parts(parts, mode),
        Err(error) => Err(error),
      }
    }

    /// Converts `value` if it is an integer in the range of `Bits` bits. `-0.0` converts to zero,
    /// as `0.0` does.
    ///
    /// # Errors
    ///
    /// Returns the first error that applies, in this order:
    /// [`FromFloatError::Nan`](crate::FromFloatError::Nan) for NaN,
    /// [`Infinite`](crate::FromFloatError::Infinite) for an infinity, then for a value outside the
    /// range of `Bits` bits, judged on the value itself,
    /// [`PosOverflow`](crate::FromFloatError::PosOverflow) above [`MAX`](Self::MAX) and
    /// [`NegOverflow`](crate::FromFloatError::NegOverflow) below [`MIN`](Self::MIN), so that
    /// `255.5` is above the range of 8 unsigned bits and `-0.5` below that of `Uint`, and then
    /// [`Inexact`](crate::FromFloatError::Inexact) for a value in the range that has a fraction.
    #[inline]
    pub const fn exact_from_f64(value: f64) -> Result<Self, $crate::FromFloatError> {
      match int::float::float_parts_f64(value) {
        Ok(parts) => Self::exact_parts(parts),
        Err(error) => Err(error),
      }
    }

    /// Converts `value` if it is an integer in the range of `Bits` bits, as
    /// [`exact_from_f64`](Self::exact_from_f64) converts the `f64` of the same value, which every
    /// `f32` has.
    ///
    /// # Errors
    ///
    /// Returns the errors of `exact_from_f64`, in its order.
    #[inline]
    pub const fn exact_from_f32(value: f32) -> Result<Self, $crate::FromFloatError> {
      match int::float::float_parts_f32(value) {
        Ok(parts) => Self::exact_parts(parts),
        Err(error) => Err(error),
      }
    }
  };
}

/// The conversions of an integer with the floats: [`Uint`] and [`Int`] over every storage.
///
/// The methods are the inherent `const fn`s of the same names, which a concrete type reaches
/// without this trait, also in constant contexts: [`to_f64`](Self::to_f64) and
/// [`to_f32`](Self::to_f32), which round to the nearest float, ties to even, as `as` converts the
/// native integers; [`checked_from_f64`](Self::checked_from_f64),
/// [`saturating_from_f64`](Self::saturating_from_f64) and
/// [`strict_from_f64`](Self::strict_from_f64), which truncate a float toward zero, as the native
/// `to_int_checked`, `to_int_saturating`, which `as` is, and `to_int_strict` do, still unstable
/// ([#159913](https://github.com/rust-lang/rust/issues/159913));
/// [`round_from_f64`](Self::round_from_f64), which rounds a float with a [`RoundingMode`];
/// [`exact_from_f64`](Self::exact_from_f64), which takes a float only if it is an integer; and the
/// `f32` form of each. The errors are those of
/// [`FromFloatError`], which reports NaN, the infinities, a value above the range and one below it
/// apart. The trait needs neither `Copy` nor a constant of a value, and every storage implements
/// it, so the bound `B: BitWidth` provides it on `Uint<B>` and `Int<B>`; generic code calls the
/// methods with the trait in scope.
///
/// ```
/// use numo::{
///   BitWidth, FromFloatError, Int, IntegerFloat, RoundingMode, Uint,
///   typenum::{U8, U200},
/// };
///
/// // The mean of the values, as an `f64`.
/// fn mean<T: IntegerFloat>(values: &[T]) -> f64 {
///   values.iter().map(T::to_f64).sum::<f64>() / values.len() as f64
/// }
///
/// assert_eq!(mean(&[Uint::<U8>::new(1), Uint::<U8>::new(2)]), 1.5);
/// assert_eq!(mean(&[Int::<U200>::MIN, Int::<U200>::MAX]), 0.0);
///
/// // A measurement rounded to the nearest integer of any width.
/// fn nearest<B: BitWidth>(value: f64) -> Result<Int<B>, FromFloatError> {
///   Int::<B>::round_from_f64(value, RoundingMode::HalfEven)
/// }
///
/// assert_eq!(nearest::<U8>(-2.5), Ok(Int::<U8>::new(-2)));
/// assert_eq!(nearest::<U8>(127.6), Err(FromFloatError::PosOverflow));
/// assert_eq!(nearest::<U200>(f64::NAN), Err(FromFloatError::Nan));
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait IntegerFloat: Sealed + Sized {
  /// Converts the value to the nearest `f64`, ties to even, or to the infinity of its sign if its
  /// magnitude rounds to `2^1024` or more; zero gives `+0.0`.
  #[must_use]
  fn to_f64(&self) -> f64;

  /// Converts the value to the nearest `f32`, ties to even, or to the infinity of its sign if its
  /// magnitude rounds to `2^128` or more; zero gives `+0.0`.
  #[must_use]
  fn to_f32(&self) -> f32;

  /// Converts `value`, truncated toward zero, or returns `None` if it is NaN or an infinity, or if
  /// the truncated value is outside the range of `Bits` bits.
  #[must_use]
  fn checked_from_f64(value: f64) -> Option<Self>;

  /// Converts `value`, truncated toward zero, or returns `None` if it is NaN or an infinity, or if
  /// the truncated value is outside the range of `Bits` bits.
  #[must_use]
  fn checked_from_f32(value: f32) -> Option<Self>;

  /// Converts `value`, truncated toward zero, or the bound of the range of `Bits` bits that it
  /// passes: `MAX` above the range and for positive infinity, `MIN` below it and for negative
  /// infinity, and zero for NaN.
  #[must_use]
  fn saturating_from_f64(value: f64) -> Self;

  /// Converts `value`, truncated toward zero, or the bound of the range of `Bits` bits that it
  /// passes: `MAX` above the range and for positive infinity, `MIN` below it and for negative
  /// infinity, and zero for NaN.
  #[must_use]
  fn saturating_from_f32(value: f32) -> Self;

  /// Converts `value`, truncated toward zero.
  ///
  /// # Panics
  ///
  /// Panics if `value` is NaN or an infinity, or if the truncated value is outside the range of
  /// `Bits` bits, reporting the caller.
  #[must_use]
  #[track_caller]
  fn strict_from_f64(value: f64) -> Self;

  /// Converts `value`, truncated toward zero.
  ///
  /// # Panics
  ///
  /// Panics if `value` is NaN or an infinity, or if the truncated value is outside the range of
  /// `Bits` bits, reporting the caller.
  #[must_use]
  #[track_caller]
  fn strict_from_f32(value: f32) -> Self;

  /// Converts `value` rounded to an integer with `mode`; a negative value that rounds to zero
  /// converts to zero.
  ///
  /// # Errors
  ///
  /// Returns [`FromFloatError::Nan`] for NaN, [`Infinite`](FromFloatError::Infinite) for an
  /// infinity, and for a rounded value outside the range of `Bits` bits,
  /// [`PosOverflow`](FromFloatError::PosOverflow) above it and
  /// [`NegOverflow`](FromFloatError::NegOverflow) below it.
  fn round_from_f64(value: f64, mode: RoundingMode) -> Result<Self, FromFloatError>;

  /// Converts `value` rounded to an integer with `mode`; a negative value that rounds to zero
  /// converts to zero.
  ///
  /// # Errors
  ///
  /// Returns [`FromFloatError::Nan`] for NaN, [`Infinite`](FromFloatError::Infinite) for an
  /// infinity, and for a rounded value outside the range of `Bits` bits,
  /// [`PosOverflow`](FromFloatError::PosOverflow) above it and
  /// [`NegOverflow`](FromFloatError::NegOverflow) below it.
  fn round_from_f32(value: f32, mode: RoundingMode) -> Result<Self, FromFloatError>;

  /// Converts `value` if it is an integer in the range of `Bits` bits; `-0.0` converts to zero.
  ///
  /// # Errors
  ///
  /// Returns the first error that applies, in this order: [`FromFloatError::Nan`] for NaN,
  /// [`Infinite`](FromFloatError::Infinite) for an infinity, then for a value outside the range,
  /// judged on the value itself, [`PosOverflow`](FromFloatError::PosOverflow) above it and
  /// [`NegOverflow`](FromFloatError::NegOverflow) below it, and then
  /// [`Inexact`](FromFloatError::Inexact) for a value in the range that has a fraction.
  fn exact_from_f64(value: f64) -> Result<Self, FromFloatError>;

  /// Converts `value` if it is an integer in the range of `Bits` bits; `-0.0` converts to zero.
  ///
  /// # Errors
  ///
  /// Returns the first error that applies, in this order: [`FromFloatError::Nan`] for NaN,
  /// [`Infinite`](FromFloatError::Infinite) for an infinity, then for a value outside the range,
  /// judged on the value itself, [`PosOverflow`](FromFloatError::PosOverflow) above it and
  /// [`NegOverflow`](FromFloatError::NegOverflow) below it, and then
  /// [`Inexact`](FromFloatError::Inexact) for a value in the range that has a fraction.
  fn exact_from_f32(value: f32) -> Result<Self, FromFloatError>;
}

/// Implements [`IntegerFloat`] for [`Uint`] and [`Int`], written `Integer: Repr, Core`, with the
/// sealed methods of the storage trait `Repr`, which neither need `Copy` nor read a constant of a
/// value: a saturation takes zero from the storage and the bounds of the range from the casts,
/// which build them from their bits, and the exact conversion rounds in the inline storage, through
/// its sealed trait `Core`, so that a fraction builds no storage.
// Trait methods cannot be `const`.
macro_rules! float_traits {
  ($($integer:ident: $repr:ident, $core:ident);+ $(;)?) => {$(
    impl<B: FitsBits<R>, R: $repr> IntegerFloat for $integer<B, R> {
      #[inline]
      fn to_f64(&self) -> f64 {
        f64_of(self.repr.float_bits(FloatFormat::F64, Token::new()))
      }

      #[inline]
      fn to_f32(&self) -> f32 {
        f32_of(self.repr.float_bits(FloatFormat::F32, Token::new()))
      }

      #[inline]
      fn checked_from_f64(value: f64) -> Option<Self> {
        IntegerFloat::round_from_f64(value, RoundingMode::Down).ok()
      }

      #[inline]
      fn checked_from_f32(value: f32) -> Option<Self> {
        IntegerFloat::round_from_f32(value, RoundingMode::Down).ok()
      }

      #[inline]
      fn saturating_from_f64(value: f64) -> Self {
        float_traits!(@saturate IntegerFloat::round_from_f64(value, RoundingMode::Down), value)
      }

      #[inline]
      fn saturating_from_f32(value: f32) -> Self {
        float_traits!(@saturate IntegerFloat::round_from_f32(value, RoundingMode::Down), value)
      }

      #[inline]
      #[track_caller]
      fn strict_from_f64(value: f64) -> Self {
        match IntegerFloat::checked_from_f64(value) {
          Some(integer) => integer,
          None => strict_float(),
        }
      }

      #[inline]
      #[track_caller]
      fn strict_from_f32(value: f32) -> Self {
        match IntegerFloat::checked_from_f32(value) {
          Some(integer) => integer,
          None => strict_float(),
        }
      }

      #[inline]
      fn round_from_f64(value: f64, mode: RoundingMode) -> Result<Self, FromFloatError> {
        let parts = int::float::float_parts_f64(value)?;
        R::from_float(parts, mode, B::U32, Token::new()).map(Self::wrap)
      }

      #[inline]
      fn round_from_f32(value: f32, mode: RoundingMode) -> Result<Self, FromFloatError> {
        let parts = int::float::float_parts_f32(value)?;
        R::from_float(parts, mode, B::U32, Token::new()).map(Self::wrap)
      }

      #[inline]
      fn exact_from_f64(value: f64) -> Result<Self, FromFloatError> {
        float_traits!(@exact $core, int::float::float_parts_f64(value)?)
      }

      #[inline]
      fn exact_from_f32(value: f32) -> Result<Self, FromFloatError> {
        float_traits!(@exact $core, int::float::float_parts_f32(value)?)
      }
    }
  )+};
  // The truncated value of `$rounded`, or the bound of the range of its sign, read from the float
  // `$value`, or zero for NaN.
  (@saturate $rounded:expr, $value:expr) => {
    match $rounded {
      Ok(integer) => integer,
      Err(FromFloatError::Nan) => Self::wrap(R::zero(Token::new())),
      // An infinity, or a value outside the range, passes the bound of its sign.
      Err(_) => cast::limit(!$value.is_sign_negative()),
    }
  };
  // The float of `$parts` if it is an integer in the range, as the inherent `exact_parts` gives it,
  // rounded in the inline storage, whose sealed trait is `$core`, and stored once it is exact.
  (@exact $core:ident, $parts:expr) => {{
    let parts = $parts;
    // `Up` judges the range on the value itself, as the inherent `exact_parts` reads it.
    let integer = <R::Inline as $core>::from_float(parts, RoundingMode::Up, B::U32, Token::new())?;
    if parts.is_integer() {
      Ok(Self::wrap(R::from_inline(integer, Token::new())))
    } else {
      Err(FromFloatError::Inexact)
    }
  }};
}

float_traits!(Uint: UintRepr, UintCore; Int: IntRepr, IntCore);

/// Implements `From` of `$integer` over every storage of `$repr` into each float of the list,
/// written `float: format, Bits, of`, at the widths of at most `Bits` bits, every value of which
/// the float holds exactly; `of` makes the float of the bits that the storage writes. The standard
/// library derives `TryFrom` from `From`, with the error `Infallible`; no other `TryFrom` converts
/// between the integers and the floats.
macro_rules! floats_from {
  ($($integer:ident: $repr:ident; $($float:ident: $format:ident, $bits:ident, $of:ident),+;)+) => {
    $($(
      impl<B, R> From<$integer<B, R>> for $float
      where
        R: $repr,
        B: FitsBits<R> + IsLessOrEqual<$bits>,
        LeEq<B, $bits>: HoldsExactly,
      {
        /// Converts `value`, which the float holds exactly, as it holds every value of `Bits` bits.
        #[inline]
        fn from(value: $integer<B, R>) -> $float {
          $of(value.repr.float_bits(FloatFormat::$format, Token::new()))
        }
      }
    )+)+
  };
}

floats_from!(
  Uint: UintRepr; f64: F64, U53, f64_of, f32: F32, U24, f32_of;
  Int: IntRepr; f64: F64, U54, f64_of, f32: F32, U25, f32_of;
);
