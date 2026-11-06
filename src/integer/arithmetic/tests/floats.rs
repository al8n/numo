//! The conversions of `Uint` and `Int` with the floats, through the inherent `const fn`s of every
//! storage and through `IntegerFloat`, which must agree: to `f64` and `f32` against what `as` gives
//! up to 128 bits and the rounding to odd in 64 bits above them; from the floats of both formats,
//! near the bounds of each width, in every rounding mode against the rounding functions of std,
//! truncated against the rule of the native `to_int_checked` and saturated as `as` converts, and
//! exactly, with the order of its errors; and the conversions in constant contexts and the panics
//! of the strict ones. Each type checks in a function of its own, whose frame holds its values
//! alone.

use core::any::type_name;

use typenum::{Sum, U25, U53, U54};

use super::{
  casts::{Model, samples},
  *,
};
use crate::{
  FromFloatError, IntegerFloat, RoundingMode,
  bigint::{
    MODES,
    floats::{self, TestFloat},
  },
  i0, u0,
};

/// The result of a conversion of a float into `$ty`, through the inherent `const fn` of the
/// storage and through `IntegerFloat`, which must agree.
macro_rules! both_from {
  ($ty:ident::$method:ident($($arg:expr),*)) => {{
    let inherent = $ty::$method($($arg),*);
    let generic = <$ty as IntegerFloat>::$method($($arg),*);
    assert_eq!(inherent, generic, "{} through the trait", stringify!($method));
    inherent
  }};
}

/// The integer of `T` whose value is `value`, an integer in the range of `T`.
fn of_value<T: Model>(value: f64) -> T {
  T::of_pattern(&floats::pattern_of(value, T::WIDTH))
}

/// What `round_from_f64` of `x` with `mode` gives in `T`, from the rounding functions of std: the
/// error of NaN and of an infinity, the rounded value where `T` holds it, and otherwise the error
/// of the side of the range that it passes.
fn rounded<T: Model>(x: f64, mode: RoundingMode) -> Result<T, FromFloatError> {
  if x.is_nan() {
    return Err(FromFloatError::Nan);
  }
  if x.is_infinite() {
    return Err(FromFloatError::Infinite);
  }
  let value = floats::round(x, mode);
  match (floats::in_range(value, T::WIDTH, T::SIGNED), value < 0.0) {
    (true, _) => Ok(of_value(value)),
    (false, true) => Err(FromFloatError::NegOverflow),
    (false, false) => Err(FromFloatError::PosOverflow),
  }
}

/// The pattern of the bound of the range of `bits` bits that a value outside it passes: zero below
/// the range of an unsigned integer, the smallest value below that of a signed one, and the
/// largest value above both.
fn bound(bits: u32, signed: bool, negative: bool) -> Vec<bool> {
  model::pattern(bits, |i| match (signed, negative) {
    (false, negative) => !negative,
    (true, negative) => (i + 1 == bits) == negative,
  })
}

/// What `saturating_from_f64` of `x` gives in `T`, as `as` converts a float to the native
/// integers: zero for NaN, the value truncated toward zero where `T` holds it, and otherwise the
/// bound of the range of the sign of `x`, which an infinity passes too.
fn saturated<T: Model>(x: f64) -> T {
  match rounded::<T>(x, RoundingMode::Down) {
    Ok(integer) => integer,
    Err(FromFloatError::Nan) => of_value(0.0),
    Err(_) => T::of_pattern(&bound(T::WIDTH, T::SIGNED, x.is_sign_negative())),
  }
}

/// What `exact_from_f64` of `x` gives in `T`, from the functions of std: the error of NaN and of an
/// infinity; then that of a value outside the range, which is above the largest integer of the
/// range exactly when the integer above it, `ceil`, is, and below the smallest exactly when the
/// integer below it, `floor`, is; then that of a fraction, which `trunc` drops.
fn exact<T: Model>(x: f64) -> Result<T, FromFloatError> {
  let (width, signed) = (T::WIDTH, T::SIGNED);
  if x.is_nan() {
    Err(FromFloatError::Nan)
  } else if x.is_infinite() {
    Err(FromFloatError::Infinite)
  } else if x > 0.0 && !floats::in_range(x.ceil(), width, signed) {
    Err(FromFloatError::PosOverflow)
  } else if x < 0.0 && !floats::in_range(x.floor(), width, signed) {
    Err(FromFloatError::NegOverflow)
  } else if x != x.trunc() {
    Err(FromFloatError::Inexact)
  } else {
    Ok(of_value(x))
  }
}

/// The floats of `F` at which a type of `bits` bits is checked: the structured floats of
/// `floats::samples` near its bounds, with as many floats of random magnitude as the tier takes,
/// or under Miri every seventh of the structured floats, with NaN, the zeros and the infinities.
fn float_samples<F: TestFloat>(bits: u32) -> Vec<F> {
  let random = match tier() {
    Tier::Full => 256,
    Tier::Debug => 16,
    Tier::Miri => 0,
  };
  let samples = floats::samples::<F>(&[bits], random);
  bigint::thin_by(samples, 7, |x| {
    let wide = x.wide();
    !wide.is_finite() || wide == 0.0
  })
}

/// Checks the conversions of each type of `$ty` with the floats, each in a function of its own:
/// from the floats of [`float_samples`] of its width, of both formats, through the inherent
/// `const fn`s and `IntegerFloat`, against [`rounded`] in every mode, against the truncated value
/// of `to_int_checked` and the strict conversion where it converts, against [`saturated`] and
/// against [`exact`], the conversions of an `f32` against the models of the `f64` of the same
/// value; and to the floats of both formats, at the values of [`samples`], against
/// `floats::float_of_pattern`.
macro_rules! check_floats {
  ($($ty:ty),+ $(,)?) => {$({
    #[inline(never)]
    fn check(f64s: &[f64], f32s: &[f32]) {
      type T = $ty;
      let name = type_name::<T>();
      for &x in f64s {
        for mode in MODES {
          assert_eq!(
            both_from!(T::round_from_f64(x, mode)),
            rounded::<T>(x, mode),
            "{x:?} {mode:?} into {name}"
          );
        }
        let checked = rounded::<T>(x, RoundingMode::Down).ok();
        assert_eq!(both_from!(T::checked_from_f64(x)), checked, "{x:?} into {name}");
        if let Some(checked) = checked {
          assert_eq!(both_from!(T::strict_from_f64(x)), checked, "{x:?} into {name}");
        }
        assert_eq!(
          both_from!(T::saturating_from_f64(x)),
          saturated::<T>(x),
          "{x:?} into {name}"
        );
        assert_eq!(both_from!(T::exact_from_f64(x)), exact::<T>(x), "{x:?} into {name}");
      }
      for &y in f32s {
        // Every `f32` is an `f64`.
        let x = f64::from(y);
        for mode in MODES {
          assert_eq!(
            both_from!(T::round_from_f32(y, mode)),
            rounded::<T>(x, mode),
            "{y:?} {mode:?} into {name}"
          );
        }
        let checked = rounded::<T>(x, RoundingMode::Down).ok();
        assert_eq!(both_from!(T::checked_from_f32(y)), checked, "{y:?} into {name}");
        if let Some(checked) = checked {
          assert_eq!(both_from!(T::strict_from_f32(y)), checked, "{y:?} into {name}");
        }
        assert_eq!(
          both_from!(T::saturating_from_f32(y)),
          saturated::<T>(x),
          "{y:?} into {name}"
        );
        assert_eq!(both_from!(T::exact_from_f32(y)), exact::<T>(x), "{y:?} into {name}");
      }
      for x in samples::<T>() {
        let p = x.pattern();
        assert_eq!(
          (
            both!(IntegerFloat::to_f64(&x)).to_bits(),
            both!(IntegerFloat::to_f32(&x)).to_bits(),
          ),
          (
            floats::float_of_pattern::<f64>(&p, <T as Model>::SIGNED).to_bits(),
            floats::float_of_pattern::<f32>(&p, <T as Model>::SIGNED).to_bits(),
          ),
          "{x:?} into the floats"
        );
      }
    }
    let bits = <$ty as Model>::WIDTH;
    check(&float_samples(bits), &float_samples(bits));
  })+};
}

/// The conversions with the floats of zero bits, one bit, 7 and 8 bits, unsigned and signed, in
/// native integers and in arrays; under Miri, two of them.
#[test]
fn floats_of_up_to_8_bits() {
  check_floats!(Uint<U0>, Int<U8, [u16; 1]>);
  if !tier().is_miri() {
    check_floats!(
      Int<U1, i8>,
      u7,
      Int<U0, [u8; 0]>,
      Uint<U0, u8>,
      Int<U0, i8>,
      Uint<U1, u8>,
      Uint<U1, [u16; 1]>,
      Int<U1, [u64; 1]>,
      i7,
      Uint<U7, [u8; 1]>,
      Int<U7, [u32; 1]>,
      Uint<U8, u8>,
      Int<U8, i8>,
      Uint<U8, [u8; 1]>,
    );
  }
}

/// The conversions with the floats of 24 and 25 bits, next to the precision of `f32`, and of 53
/// and 54 bits, next to that of `f64`, unsigned and signed, in native integers and in arrays;
/// under Miri, two of them.
#[test]
fn floats_of_the_precisions_of_the_floats() {
  check_floats!(Uint<U24, [u8; 3]>, Int<U54, i64>);
  if !tier().is_miri() {
    check_floats!(
      Uint<U24, u32>,
      Int<U24, i32>,
      Int<U24, [u8; 3]>,
      Uint<U25, u32>,
      Int<U25, i32>,
      Uint<U25, [u16; 2]>,
      Int<U25, [u64; 1]>,
      Uint<U53, u64>,
      Int<U53, i64>,
      Uint<U53, [u32; 2]>,
      Int<U53, [u8; 7]>,
      Uint<U54, u64>,
      Uint<U54, [u64; 1]>,
      Int<U54, [u16; 4]>,
    );
  }
}

/// The conversions with the floats of 64, 65 and 128 bits, unsigned and signed, in native integers
/// and in arrays; under Miri, one of them.
#[test]
fn floats_of_64_to_128_bits() {
  check_floats!(Uint<U65, [u64; 2]>);
  if !tier().is_miri() {
    check_floats!(
      Int<U128, i128>,
      Uint<U64, u64>,
      Int<U64, i64>,
      Uint<U64, [u32; 2]>,
      Int<U64, [u8; 8]>,
      Uint<U65, u128>,
      Int<U65, i128>,
      Int<U65, [u32; 3]>,
      Uint<U128, u128>,
      Uint<U128, [u64; 2]>,
      Int<U128, [u128; 1]>,
    );
  }
}

/// The conversions with the floats of 129, 200 and 1024 bits, and of 1025, the first width of
/// `Int` that holds the largest `f64`, unsigned and signed, in arrays of each limb type; under
/// Miri, one of them.
#[test]
fn floats_of_wide_widths() {
  check_floats!(Int<U129, [u8; 17]>);
  if !tier().is_miri() {
    check_floats!(
      Uint<U129, [u64; 3]>,
      Uint<U200, [u128; 2]>,
      Int<U200, [u64; 4]>,
      Uint<U1024, [u64; 16]>,
      Int<U1024, [u32; 32]>,
      Uint<Sum<U1024, U1>, [u64; 17]>,
      Int<Sum<U1024, U1>, [u64; 17]>,
    );
  }
}

/// At the full width of each native storage, the conversions give what `as` gives between the
/// native integer and the floats, both ways: the saturating conversion from the floats near the
/// bounds of both signs, and the conversions to the floats at the values of [`samples`].
#[test]
fn native_storages_at_their_full_width_convert_as_as_does() {
  // Checks `Uint` and `Int` of `$bits` bits in `$uint` and `$int`, in a function of its own.
  macro_rules! as_converts {
    ($($bits:ident: $uint:ident, $int:ident);+ $(;)?) => {$({
      #[inline(never)]
      fn check(f64s: &[f64], f32s: &[f32]) {
        type U = Uint<$bits, $uint>;
        type I = Int<$bits, $int>;
        for &x in f64s {
          assert_eq!(
            (U::saturating_from_f64(x).value(), I::saturating_from_f64(x).value()),
            (x as $uint, x as $int),
            "{x:?}"
          );
        }
        for &y in f32s {
          assert_eq!(
            (U::saturating_from_f32(y).value(), I::saturating_from_f32(y).value()),
            (y as $uint, y as $int),
            "{y:?}"
          );
        }
        for x in samples::<U>() {
          let value = x.value();
          assert_eq!(
            (x.to_f64().to_bits(), x.to_f32().to_bits()),
            ((value as f64).to_bits(), (value as f32).to_bits()),
            "{value}"
          );
        }
        for x in samples::<I>() {
          let value = x.value();
          assert_eq!(
            (x.to_f64().to_bits(), x.to_f32().to_bits()),
            ((value as f64).to_bits(), (value as f32).to_bits()),
            "{value}"
          );
        }
      }
      check(&float_samples($uint::BITS), &float_samples($uint::BITS));
    })+};
  }
  as_converts!(
    U8: u8, i8;
    U16: u16, i16;
    U32: u32, i32;
    U64: u64, i64;
    U128: u128, i128;
  );
}

/// `exact_from_f64` and `exact_from_f32` convert an integer of the range, `-0.0` included, and
/// report the first error in their order: NaN, an infinity, a value outside the range, judged on
/// the value itself, so that a fraction just beyond `MAX` or `MIN` passes it, and then a fraction
/// in the range; at zero bits, one signed bit, 7 and 8 bits and 256 bits.
#[test]
fn exact_conversions_report_the_first_error() {
  use FromFloatError::{Inexact, Infinite, Nan, NegOverflow, PosOverflow};
  type U8x1 = Uint<U8, [u8; 1]>;
  type I1 = Int<U1, i8>;
  assert_eq!(u7::exact_from_f64(127.0), Ok(u7::MAX));
  assert_eq!(u7::exact_from_f64(-0.0), Ok(u7::ZERO));
  assert_eq!(u7::exact_from_f64(126.5), Err(Inexact));
  assert_eq!(u7::exact_from_f64(127.5), Err(PosOverflow));
  assert_eq!(u7::exact_from_f64(-0.5), Err(NegOverflow));
  assert_eq!(u7::exact_from_f64(-1.0), Err(NegOverflow));
  assert_eq!(u7::exact_from_f64(f64::NAN), Err(Nan));
  assert_eq!(u7::exact_from_f64(f64::INFINITY), Err(Infinite));
  assert_eq!(u7::exact_from_f64(f64::NEG_INFINITY), Err(Infinite));
  assert_eq!(i7::exact_from_f64(-64.0), Ok(i7::MIN));
  assert_eq!(i7::exact_from_f64(-64.5), Err(NegOverflow));
  assert_eq!(i7::exact_from_f64(63.5), Err(PosOverflow));
  assert_eq!(i7::exact_from_f64(-63.5), Err(Inexact));
  assert_eq!(i7::exact_from_f64(-0.5), Err(Inexact));
  assert_eq!(Uint::<U8>::exact_from_f64(255.5), Err(PosOverflow));
  assert_eq!(U8x1::exact_from_f32(255.0), Ok(U8x1::MAX));
  assert_eq!(U8x1::exact_from_f32(-0.25), Err(NegOverflow));
  assert_eq!(I1::exact_from_f64(-1.0), Ok(I1::MIN));
  assert_eq!(I1::exact_from_f64(-0.5), Err(Inexact));
  assert_eq!(I1::exact_from_f64(0.5), Err(PosOverflow));
  assert_eq!(u0::exact_from_f64(0.25), Err(PosOverflow));
  assert_eq!(i0::exact_from_f64(-0.25), Err(NegOverflow));
  assert_eq!(u0::exact_from_f32(-0.0), Ok(u0::ZERO));
  let power = floats::pow2(255);
  assert_eq!(i256::exact_from_f64(-power), Ok(i256::MIN));
  assert_eq!(i256::exact_from_f64(power), Err(PosOverflow));
  assert_eq!(u256::exact_from_f64(power), Ok(u256::ONE << 255u32));
  assert_eq!(u256::exact_from_f64(floats::pow2(256)), Err(PosOverflow));
  assert_eq!(
    <u256 as IntegerFloat>::exact_from_f32(-0.5),
    Err(NegOverflow)
  );
  assert_eq!(<i256 as IntegerFloat>::exact_from_f32(-0.5), Err(Inexact));
  assert_eq!(<i256 as IntegerFloat>::exact_from_f64(f64::NAN), Err(Nan));
}

/// The errors write the texts of their kinds, those of the native conversions for a value outside
/// the range, and have a predicate for each kind.
#[test]
fn errors_write_their_kinds() {
  use FromFloatError::{Inexact, Infinite, Nan, NegOverflow, PosOverflow};
  let (above, below) = super::casts::errors();
  assert_eq!(Nan.to_string(), "NaN has no integer value");
  assert_eq!(Infinite.to_string(), "infinity has no integer value");
  assert_eq!(PosOverflow.to_string(), above.to_string());
  assert_eq!(NegOverflow.to_string(), below.to_string());
  assert_eq!(Inexact.to_string(), "the value is not an integer");
  assert!(Nan.is_nan() && Infinite.is_infinite() && Inexact.is_inexact());
  assert!(PosOverflow.is_pos_overflow() && NegOverflow.is_neg_overflow());
  assert!(!Nan.is_infinite() && !PosOverflow.is_neg_overflow());
}

/// Every conversion with the floats evaluates in constant contexts, in native integers and arrays.
#[test]
fn conversions_with_the_floats_in_constant_contexts() {
  type Bytes = Uint<U24, [u8; 3]>;
  const TO: (f64, f32, f64, f32) = (
    u7::MAX.to_f64(),
    i7::MIN.to_f32(),
    u256::MAX.to_f64(),
    Bytes::MAX.to_f32(),
  );
  const CHECKED: (Option<u7>, Option<i256>, Option<u7>) = (
    u7::checked_from_f64(127.9),
    i256::checked_from_f32(-1.5),
    u7::checked_from_f64(f64::NAN),
  );
  const SATURATING: (u7, i7, Bytes) = (
    u7::saturating_from_f64(f64::INFINITY),
    i7::saturating_from_f32(f32::NEG_INFINITY),
    Bytes::saturating_from_f64(-1.0),
  );
  const STRICT: (i7, Bytes) = (
    i7::strict_from_f64(-64.9),
    Bytes::strict_from_f32(16_777_215.0),
  );
  const ROUND: (Result<u256, FromFloatError>, Result<i7, FromFloatError>) = (
    u256::round_from_f64(2.5, RoundingMode::HalfEven),
    i7::round_from_f32(-0.5, RoundingMode::Floor),
  );
  const EXACT: (Result<u7, FromFloatError>, Result<i256, FromFloatError>) =
    (u7::exact_from_f64(-0.0), i256::exact_from_f32(0.5));
  assert_eq!(TO, (127.0, -64.0, floats::pow2(256), 16_777_215.0));
  assert_eq!(CHECKED, (Some(u7::MAX), Some(-i256::ONE), None));
  assert_eq!(SATURATING, (u7::MAX, i7::MIN, Bytes::ZERO));
  assert_eq!(STRICT, (i7::MIN, Bytes::MAX));
  assert_eq!(ROUND, (Ok(u256::from(2u64)), Ok(-i7::ONE)));
  assert_eq!(EXACT, (Ok(u7::ZERO), Err(FromFloatError::Inexact)));
}

/// `strict_from_f64` and `strict_from_f32`, inherent and through `IntegerFloat`, panic in every
/// build on NaN, an infinity and a value whose truncation is outside the range, with the message of
/// the native `to_int_strict`, and report the caller.
#[test]
fn strict_conversions_panic_outside_the_range() {
  let message = "the value cannot be represented in the target integer type".to_string();
  let (nan, infinity) = (f64::NAN, f32::INFINITY);
  type Wide = Int<U200, [u8; 25]>;
  panics_here!(u7::strict_from_f64(nan), message);
  panics_here!(u7::strict_from_f64(128.0), message);
  panics_here!(u7::strict_from_f64(-1.0), message);
  panics_here!(i7::strict_from_f32(-65.0), message);
  panics_here!(i7::strict_from_f32(infinity), message);
  panics_here!(u256::strict_from_f64(-1.0), message);
  panics_here!(Wide::strict_from_f64(f64::MAX), message);
  panics_here!(u0::strict_from_f64(1.0), message);
  panics_here!(<u7 as IntegerFloat>::strict_from_f64(nan), message);
  panics_here!(<Wide as IntegerFloat>::strict_from_f32(infinity), message);
  // A value that truncates into the range converts.
  assert_eq!(u7::strict_from_f64(-0.9), u7::ZERO);
  assert_eq!(<Wide as IntegerFloat>::strict_from_f32(-0.9), Wide::ZERO);
}

/// The conversions with the floats in code generic over the width, whose bound `B: BitWidth`
/// alone gives `Uint<B>` and `Int<B>` `IntegerFloat`, at widths from zero bits to 1024.
#[test]
fn conversions_with_the_floats_in_code_generic_over_the_width() {
  // The value of `x` as an `f64`, and `x` rounded to the nearest integer of `Int<B>`.
  fn to_float<B: BitWidth>(x: Uint<B>) -> f64 {
    x.to_f64()
  }
  fn nearest<B: BitWidth>(x: f64) -> Result<Int<B>, FromFloatError> {
    Int::<B>::round_from_f64(x, RoundingMode::HalfEven)
  }
  assert_eq!(to_float(Uint::<U0>::ZERO), 0.0);
  assert_eq!(to_float(u7::MAX), 127.0);
  assert_eq!(to_float(Uint::<U64>::MAX), floats::pow2(64));
  assert_eq!(to_float(Uint::<U1024>::MAX), f64::INFINITY);
  assert_eq!(nearest::<U0>(0.5), Ok(Int::<U0>::ZERO));
  assert_eq!(nearest::<U1>(-0.5), Ok(Int::<U1>::ZERO));
  assert_eq!(nearest::<U1>(-1.5), Err(FromFloatError::NegOverflow));
  assert_eq!(nearest::<U7>(63.5), Err(FromFloatError::PosOverflow));
  assert_eq!(nearest::<U128>(-0.5), Ok(Int::<U128>::ZERO));
  assert_eq!(nearest::<U200>(-2.5), Ok(-Int::<U200>::from(2i8)));
}
