use core::hash::BuildHasher;
use std::{collections::hash_map::RandomState, format, string::ToString, vec::Vec};

use typenum::{
  U0, U1, U2, U4, U5, U8, U9, U10, U17, U18, U19, U38, U40, U59, U60, U78, U97, U116, U135, U154,
  Unsigned,
};

use super::{FixedDecimal, HasDecimalOne, SignedFixedDecimal, UnsignedFixedDecimal};
use crate::{
  Decimal, ParseErrorKind, QuantizeError, RoundingMode,
  bigint::MODES,
  i192, i256, i320, i384, i448, i512,
  layout::{StorageCore, Token},
  u192, u256, u320, u384, u448, u512,
};

/// Zero, the limits, and one, two and 25 units in the last place, also negated for a signed
/// storage.
fn samples<T: FixedDecimal>() -> Vec<T> {
  let mut values = Vec::from([T::ZERO, T::MIN, T::MAX]);
  for units in [1, 2, 25] {
    values.push(format!("{units}e-{}", T::FRACTION).parse().unwrap());
    if T::SIGNED {
      values.push(format!("-{units}e-{}", T::FRACTION).parse().unwrap());
    }
  }
  values
}

/// Compiles only if the storage of `T` is `R`.
fn same_repr<T: FixedDecimal<Repr = R>, R>() {}

fn signed<T: SignedFixedDecimal>() {}

fn unsigned<T: UnsignedFixedDecimal>() {}

/// Asserts that the constants, the storage and the methods of [`FixedDecimal`] are the inherent
/// ones for each type, on the values of [`samples`], and that the digit counts are those of the
/// type.
macro_rules! assert_fixed {
  ($(Decimal<$integral:ty, $fraction:ty, $repr:ty>),+ $(,)?) => {$({
    type T = Decimal<$integral, $fraction, $repr>;
    same_repr::<T, $repr>();
    assert_eq!(<T as FixedDecimal>::INTEGRAL, T::INTEGRAL);
    assert_eq!(<T as FixedDecimal>::FRACTION, T::FRACTION);
    assert_eq!(<T as FixedDecimal>::DIGITS, T::DIGITS);
    assert_eq!(<T as FixedDecimal>::SIGNED, T::SIGNED);
    assert_eq!(<T as FixedDecimal>::ZERO, T::ZERO);
    assert_eq!(<T as FixedDecimal>::MAX, T::MAX);
    assert_eq!(<T as FixedDecimal>::MIN, T::MIN);
    assert_eq!(T::INTEGRAL, <$integral as Unsigned>::U32);
    assert_eq!(T::FRACTION, <$fraction as Unsigned>::U32);
    assert_eq!(T::DIGITS, T::INTEGRAL + T::FRACTION);

    let values = samples::<T>();
    for value in &values {
      assert_eq!(FixedDecimal::is_zero(value), value.is_zero(), "{value}");
      assert_eq!(FixedDecimal::is_positive(value), value.is_positive(), "{value}");
      assert_eq!(FixedDecimal::repr(*value), value.repr(), "{value}");
      assert_eq!(<T as FixedDecimal>::from_repr(value.repr()), Some(*value), "{value}");
      for rhs in &values {
        assert_eq!(
          FixedDecimal::checked_add(value, rhs),
          value.checked_add(rhs),
          "{value} + {rhs}"
        );
        assert_eq!(
          FixedDecimal::checked_sub(value, rhs),
          value.checked_sub(rhs),
          "{value} - {rhs}"
        );
        assert_eq!(
          FixedDecimal::is_multiple_of(value, rhs),
          value.is_multiple_of(rhs),
          "{value} on {rhs}"
        );
        for mode in MODES {
          assert_eq!(
            FixedDecimal::quantize(value, rhs, mode),
            value.quantize(rhs, mode),
            "{value} to {rhs}, {mode:?}"
          );
        }
      }
    }

    // One unit in the last place leaves the range at either limit. `MAX` has an odd coefficient,
    // so rounding it up to a step of two units gives `10^DIGITS`, which is out of range, both as a
    // value and as a coefficient.
    let unit = T::parse(&format!("1e-{}", T::FRACTION)).unwrap();
    let two_units = unit.checked_add(&unit).unwrap();
    assert_eq!(FixedDecimal::checked_add(&T::MAX, &unit), None);
    assert_eq!(FixedDecimal::checked_sub(&T::MIN, &unit), None);
    assert_eq!(
      FixedDecimal::quantize(&T::MAX, &two_units, RoundingMode::Up),
      Err(QuantizeError::OutOfRange)
    );
    assert_eq!(
      FixedDecimal::quantize(&unit, &T::ZERO, RoundingMode::Up),
      Err(QuantizeError::InvalidStep)
    );
    assert!(FixedDecimal::is_multiple_of(&T::ZERO, &T::ZERO));
    assert!(!FixedDecimal::is_multiple_of(&unit, &T::ZERO));
    let beyond = StorageCore::checked_mul_pow10(&unit.repr(), T::DIGITS, Token::new()).unwrap();
    assert_eq!(<T as FixedDecimal>::from_repr(beyond), None);
    assert_eq!(T::from_repr(beyond), None);
  })+};
}

/// As [`assert_fixed!`], and the methods of [`SignedFixedDecimal`] are the inherent ones.
macro_rules! assert_signed {
  ($(Decimal<$integral:ty, $fraction:ty, $repr:ty>),+ $(,)?) => {$({
    assert_fixed!(Decimal<$integral, $fraction, $repr>);
    type T = Decimal<$integral, $fraction, $repr>;
    signed::<T>();
    assert!(T::SIGNED);
    for value in samples::<T>() {
      assert_eq!(SignedFixedDecimal::is_negative(&value), value.is_negative(), "{value}");
      assert_eq!(SignedFixedDecimal::neg(&value), value.neg(), "{value}");
      assert_eq!(SignedFixedDecimal::abs(&value), value.abs(), "{value}");
    }

    // A negative step is invalid, and the coefficient `-10^DIGITS` is below `MIN`.
    let unit = T::parse(&format!("1e-{}", T::FRACTION)).unwrap();
    assert_eq!(
      FixedDecimal::quantize(&unit, &unit.neg(), RoundingMode::Up),
      Err(QuantizeError::InvalidStep)
    );
    let beyond = StorageCore::checked_mul_pow10(&unit.repr(), T::DIGITS, Token::new()).unwrap();
    let below = StorageCore::checked_neg(&beyond, Token::new()).unwrap();
    assert_eq!(<T as FixedDecimal>::from_repr(below), None);
  })+};
}

/// As [`assert_fixed!`], for a type with an unsigned storage.
macro_rules! assert_unsigned {
  ($(Decimal<$integral:ty, $fraction:ty, $repr:ty>),+ $(,)?) => {$({
    assert_fixed!(Decimal<$integral, $fraction, $repr>);
    unsigned::<Decimal<$integral, $fraction, $repr>>();
    assert!(!Decimal::<$integral, $fraction, $repr>::SIGNED);
  })+};
}

/// Asserts that [`HasDecimalOne::ONE`] is the inherent `ONE`, which is one, for each type.
macro_rules! assert_one {
  ($($ty:ty),+ $(,)?) => {$(
    assert_eq!(<$ty as HasDecimalOne>::ONE, <$ty>::ONE);
    assert_eq!(<$ty as HasDecimalOne>::ONE.to_string(), "1");
  )+};
}

#[test]
fn signed_items_are_the_inherent_ones() {
  assert_signed!(
    Decimal<U1, U1, i8>,
    Decimal<U0, U2, i8>,
    Decimal<U2, U2, i16>,
    Decimal<U5, U4, i32>,
    Decimal<U10, U8, i64>,
    Decimal<U19, U19, i128>,
    Decimal<U40, U17, i192>,
    Decimal<U38, U38, i256>,
    Decimal<U78, U18, i320>,
    Decimal<U97, U18, i384>,
    Decimal<U116, U18, i448>,
    Decimal<U135, U18, i512>,
  );
}

#[test]
fn unsigned_items_are_the_inherent_ones() {
  assert_unsigned!(
    Decimal<U1, U1, u8>,
    Decimal<U0, U2, u8>,
    Decimal<U2, U2, u16>,
    Decimal<U5, U4, u32>,
    Decimal<U10, U9, u64>,
    Decimal<U19, U19, u128>,
    Decimal<U40, U17, u192>,
    Decimal<U59, U18, u256>,
    Decimal<U78, U18, u320>,
    Decimal<U97, U18, u384>,
    Decimal<U116, U18, u448>,
    Decimal<U154, U0, u512>,
  );
}

#[test]
fn one_is_the_inherent_one() {
  assert_one!(
    Decimal<U1, U1, i8>,
    Decimal<U2, U2, i16>,
    Decimal<U5, U4, i32>,
    Decimal<U10, U8, i64>,
    Decimal<U19, U19, i128>,
    Decimal<U40, U17, i192>,
    Decimal<U38, U38, i256>,
    Decimal<U78, U18, i320>,
    Decimal<U97, U18, i384>,
    Decimal<U116, U18, i448>,
    Decimal<U135, U18, i512>,
    Decimal<U1, U1, u8>,
    Decimal<U2, U2, u16>,
    Decimal<U5, U4, u32>,
    Decimal<U10, U9, u64>,
    Decimal<U19, U19, u128>,
    Decimal<U40, U17, u192>,
    Decimal<U59, U18, u256>,
    Decimal<U78, U18, u320>,
    Decimal<U97, U18, u384>,
    Decimal<U116, U18, u448>,
    Decimal<U154, U0, u512>,
  );
}

/// Uses only what the bound `T: FixedDecimal` gives: `Default`, `FromStr`, `Display`, `Debug`,
/// `Hash` and `Ord`.
fn check_supertraits<T: FixedDecimal>() {
  assert_eq!(T::default(), T::ZERO);
  let value: T = "1.5".parse().unwrap();
  assert_eq!(format!("{value:.3}"), "1.500");
  assert_eq!(format!("{value:?}"), "1.5");
  assert_eq!(
    "1,5".parse::<T>().map_err(|err| err.kind()),
    Err(ParseErrorKind::InvalidSyntax)
  );
  let state = RandomState::new();
  assert_eq!(
    state.hash_one(value),
    state.hash_one("1.50".parse::<T>().unwrap())
  );
  assert!(T::MIN <= T::ZERO && T::ZERO < value && value < T::MAX);
  assert_eq!(value.max(T::ZERO), value);
}

#[test]
fn supertraits_through_the_bound() {
  check_supertraits::<Decimal<U10, U8>>();
  check_supertraits::<Decimal<U10, U9, u64>>();
  check_supertraits::<Decimal<U38, U38, i256>>();
  check_supertraits::<Decimal<U60, U18, u320>>();
}

/// In code bounded by [`SignedFixedDecimal`], `-x` and `x.neg()` on a value call `Neg::neg`, while
/// `x.neg()` on a reference and the path through the trait call [`SignedFixedDecimal::neg`].
fn negations<T: SignedFixedDecimal>(x: T) -> [T; 4] {
  let by_reference = &x;
  [-x, x.neg(), by_reference.neg(), SignedFixedDecimal::neg(&x)]
}

#[test]
fn negations_agree() {
  for text in ["0", "1.5", "-2.25"] {
    let price: Decimal<U10, U8> = text.parse().unwrap();
    assert_eq!(negations(price), [price.neg(); 4], "{text}");
    let wide: Decimal<U38, U38, i256> = text.parse().unwrap();
    assert_eq!(negations(wide), [wide.neg(); 4], "{text}");
  }
}

/// Every trait of both families and `Neg` are in scope at once: no call below is ambiguous
/// (E0034), on concrete types or in code bounded by either family.
mod collisions {
  use core::ops::Neg;

  use typenum::{U8, U10};

  #[allow(
    unused_imports,
    reason = "every trait is in scope, as in code that imports all of them"
  )]
  use crate::{
    ArrayInteger, CheckedOps, Decimal, FixedDecimal, FixedInteger, FromLimbs, HasDecimalOne,
    HasOne, NativeInteger, OverflowingOps, SaturatingOps, SignedFixedDecimal, SignedFixedInteger,
    StrictOps, UncheckedOps, UnsignedFixedDecimal, UnsignedFixedInteger, WrappingOps, i256, u256,
  };

  type Price = Decimal<U10, U8>;
  type Amount = Decimal<U10, U8, u64>;

  fn signed_decimal<T>(x: &T) -> (Option<T>, bool, bool, T, T)
  where
    T: FixedDecimal + SignedFixedDecimal + HasDecimalOne,
  {
    (
      x.checked_add(&T::ONE),
      x.is_zero(),
      x.is_negative(),
      x.neg(),
      x.abs(),
    )
  }

  fn unsigned_decimal<T>(x: &T) -> (Option<T>, bool)
  where
    T: FixedDecimal + UnsignedFixedDecimal + HasDecimalOne,
  {
    (x.checked_add(&T::ONE), x.is_zero())
  }

  fn signed_integer<T>(x: &T) -> (Option<T>, bool, bool, T, T)
  where
    T: FixedInteger
      + SignedFixedInteger
      + HasOne
      + OverflowingOps
      + CheckedOps
      + WrappingOps
      + SaturatingOps
      + StrictOps
      + UncheckedOps
      + ArrayInteger
      + FromLimbs,
  {
    (
      x.checked_add(&T::ONE),
      x.is_zero(),
      x.is_negative(),
      x.neg(),
      x.abs(),
    )
  }

  fn unsigned_integer<T>(x: &T) -> (Option<T>, bool)
  where
    T: FixedInteger
      + UnsignedFixedInteger
      + HasOne
      + OverflowingOps
      + CheckedOps
      + WrappingOps
      + SaturatingOps
      + StrictOps
      + UncheckedOps
      + ArrayInteger
      + FromLimbs,
  {
    (x.checked_add(&T::ONE), x.is_zero())
  }

  #[test]
  fn shared_names_resolve() {
    let price: Price = "-1.5".parse().unwrap();
    let concrete = (
      price.checked_add(&Price::ONE),
      price.is_zero(),
      price.is_negative(),
      price.neg(),
      price.abs(),
    );
    assert_eq!(
      concrete,
      (Some("-0.5".parse().unwrap()), false, true, -price, -price)
    );
    assert_eq!(signed_decimal(&price), concrete);

    let amount: Amount = "1.5".parse().unwrap();
    let concrete = (amount.checked_add(&Amount::ONE), amount.is_zero());
    assert_eq!(concrete, (Some("2.5".parse().unwrap()), false));
    assert_eq!(unsigned_decimal(&amount), concrete);

    let int = i256::ONE.neg();
    let concrete = (
      int.checked_add(&i256::ONE),
      int.is_zero(),
      int.is_negative(),
      int.neg(),
      int.abs(),
    );
    assert_eq!(
      concrete,
      (Some(i256::ZERO), false, true, i256::ONE, i256::ONE)
    );
    assert_eq!(signed_integer(&int), concrete);

    let uint = u256::ONE;
    let concrete = (uint.checked_add(&u256::ONE), uint.is_zero());
    assert_eq!(concrete, (Some(u256::from_limbs([2, 0, 0, 0])), false));
    assert_eq!(unsigned_integer(&uint), concrete);
  }
}
