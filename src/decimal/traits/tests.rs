use core::hash::BuildHasher;
use std::{collections::hash_map::RandomState, format, string::ToString, vec::Vec};

use typenum::{
  U0, U1, U2, U4, U5, U8, U9, U10, U11, U12, U16, U17, U18, U19, U20, U21, U22, U26, U38, U40, U50,
  U58, U59, U60, U78, U96, U97, U116, U135, U154, Unsigned,
};

use super::{
  ConstDecimalOne, ConstFixedDecimal, ConvertTo, DecimalOne, DivRound, ExactMul, FixedDecimal,
  SignedFixedDecimal, UnsignedFixedDecimal, WidenTo,
};
use crate::{
  ConvertError, Decimal, DivideError, ParseErrorKind, QuantizeError, RoundingMode,
  bigint::MODES,
  i192, i256, i320, i384, i448, i512,
  layout::{StorageCore, Token},
  u192, u256, u320, u384, u448, u512,
};

/// Zero, the limits, and one, two and 25 units in the last place, also negated for a signed
/// storage.
fn samples<T: FixedDecimal>() -> Vec<T> {
  let mut values = Vec::from([T::zero(), T::min_value(), T::max_value()]);
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
/// ones for each type, on the values of [`samples`], that its functions `zero`, `max_value` and
/// `min_value` and the constants of [`ConstFixedDecimal`] are the inherent constants, that the
/// default is zero, and that the digit counts are those of the type.
macro_rules! assert_fixed {
  ($(Decimal<$integral:ty, $fraction:ty, $repr:ty>),+ $(,)?) => {$({
    type T = Decimal<$integral, $fraction, $repr>;
    same_repr::<T, $repr>();
    assert_eq!(<T as FixedDecimal>::INTEGRAL, T::INTEGRAL);
    assert_eq!(<T as FixedDecimal>::FRACTION, T::FRACTION);
    assert_eq!(<T as FixedDecimal>::DIGITS, T::DIGITS);
    assert_eq!(<T as FixedDecimal>::SIGNED, T::SIGNED);
    assert_eq!(<T as FixedDecimal>::zero(), T::ZERO);
    assert_eq!(<T as FixedDecimal>::max_value(), T::MAX);
    assert_eq!(<T as FixedDecimal>::min_value(), T::MIN);
    assert_eq!(<T as ConstFixedDecimal>::ZERO, T::ZERO);
    assert_eq!(<T as ConstFixedDecimal>::MAX, T::MAX);
    assert_eq!(<T as ConstFixedDecimal>::MIN, T::MIN);
    assert_eq!(T::default(), <T as FixedDecimal>::zero());
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

/// Asserts that [`DecimalOne::one`] and [`ConstDecimalOne::ONE`] are the inherent `ONE`, which is
/// one, for each type.
macro_rules! assert_one {
  ($($ty:ty),+ $(,)?) => {$(
    assert_eq!(<$ty as DecimalOne>::one(), <$ty>::ONE);
    assert_eq!(<$ty as ConstDecimalOne>::ONE, <$ty>::ONE);
    assert_eq!(<$ty as DecimalOne>::one().to_string(), "1");
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

/// Uses only what the bound `T: FixedDecimal` gives: `Clone`, `Default`, `FromStr`, `Display`,
/// `Debug`, `Hash`, `Ord`, zero and the limits. A value that is used again is borrowed or cloned,
/// never copied.
fn check_supertraits<T: FixedDecimal>() {
  assert_eq!(T::default(), T::zero());
  let value: T = "1.5".parse().unwrap();
  assert_eq!(format!("{value:.3}"), "1.500");
  assert_eq!(format!("{value:?}"), "1.5");
  assert_eq!(
    "1,5".parse::<T>().map_err(|err| err.kind()),
    Err(ParseErrorKind::InvalidSyntax)
  );
  let state = RandomState::new();
  assert_eq!(
    state.hash_one(&value),
    state.hash_one("1.50".parse::<T>().unwrap())
  );
  assert!(T::min_value() <= T::zero() && T::zero() < value && value < T::max_value());
  assert_eq!(value.clone().max(T::zero()), value);
}

#[test]
fn supertraits_through_the_bound() {
  check_supertraits::<Decimal<U10, U8>>();
  check_supertraits::<Decimal<U10, U9, u64>>();
  check_supertraits::<Decimal<U38, U38, i256>>();
  check_supertraits::<Decimal<U60, U18, u320>>();
}

/// Uses the general layer through the bound `T: FixedDecimal` alone and clones each value that it
/// uses again, so it compiles only because the general layer does not require `Copy`. Returns the
/// sum, the smallest and the largest of the values, and the largest rebuilt from its coefficient.
fn without_copy<T: FixedDecimal>(values: &[T]) -> (Option<T>, T, T, Option<T>) {
  let sum = values
    .iter()
    .try_fold(T::zero(), |sum, value| sum.checked_add(value));
  let smallest = values.iter().fold(T::max_value(), |smallest, value| {
    smallest.min(value.clone())
  });
  let largest = values
    .iter()
    .fold(T::min_value(), |largest, value| largest.max(value.clone()));
  let rebuilt = T::from_repr(largest.clone().repr());
  (sum, smallest, largest, rebuilt)
}

#[test]
fn general_layer_without_copy() {
  type Price = Decimal<U10, U8>;
  type Amount = Decimal<U10, U9, u64>;
  type Wide = Decimal<U38, U38, i256>;

  let prices: [Price; 3] = ["1.5", "-0.25", "64250.5"].map(|text| text.parse().unwrap());
  assert_eq!(
    without_copy(&prices),
    (
      Some("64251.75".parse().unwrap()),
      prices[1],
      prices[2],
      Some(prices[2])
    )
  );
  // Without values, the sum is zero, the smallest is `MAX` and the largest is `MIN`, which is zero
  // for an unsigned storage.
  assert_eq!(
    without_copy::<Amount>(&[]),
    (
      Some(Amount::ZERO),
      Amount::MAX,
      Amount::ZERO,
      Some(Amount::ZERO)
    )
  );
  assert_eq!(
    without_copy(&[Wide::MAX; 2]),
    (None, Wide::MAX, Wide::MAX, Some(Wide::MAX))
  );
}

/// Reads the constants of the const layer in a generic `const fn`, which cannot call the functions
/// of the general layer.
const fn constants<T: ConstDecimalOne>() -> [T; 4] {
  [T::MIN, T::ZERO, T::ONE, T::MAX]
}

#[test]
fn const_layer_in_constant_contexts() {
  type Price = Decimal<U10, U8>;
  type Supply = Decimal<U59, U18, u256>;

  const PRICES: [Price; 4] = constants();
  const SUPPLIES: [Supply; 4] = constants();
  assert_eq!(PRICES, [Price::MIN, Price::ZERO, Price::ONE, Price::MAX]);
  assert_eq!(
    SUPPLIES,
    [Supply::ZERO, Supply::ZERO, Supply::ONE, Supply::MAX]
  );
}

/// In code bounded by [`SignedFixedDecimal`], `-x` and `x.neg()` on a value call `Neg::neg`, while
/// `x.neg()` on a reference and the path through the trait call [`SignedFixedDecimal::neg`]. The
/// bound does not require `Copy`, so the value is cloned for the first negation that takes it.
fn negations<T: SignedFixedDecimal>(x: T) -> [T; 4] {
  let by_reference = &x;
  let (on_reference, through_trait) = (by_reference.neg(), SignedFixedDecimal::neg(&x));
  [-x.clone(), x.neg(), on_reference, through_trait]
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

/// Compiles only if the product of `P` and `Q` is `O`.
fn product<P: ExactMul<Q, Output = O>, Q, O>() {}

/// The total of the products of the pairs, through the bound `FixedDecimal` of `Output`.
fn total<P: ExactMul<Q>, Q>(pairs: &[(P, Q)]) -> Option<P::Output> {
  pairs.iter().try_fold(P::Output::zero(), |sum, (lhs, rhs)| {
    sum.checked_add(&lhs.exact_mul(rhs))
  })
}

#[test]
fn exact_mul_is_the_inherent_one() {
  type Price = Decimal<U10, U8>;
  type Quantity = Decimal<U12, U8>;
  type Volume = Decimal<U10, U9, u64>;
  type Lots = Decimal<U11, U8, u64>;
  type Large = Decimal<U19, U19>;
  type Supply = Decimal<U40, U17, u192>;

  // The product of two unsigned storages is unsigned, and any other product is signed.
  product::<Price, Quantity, Decimal<U22, U16>>();
  product::<Volume, Lots, Decimal<U21, U17, u128>>();
  product::<Volume, Quantity, Decimal<U22, U17, i192>>();
  product::<Quantity, Volume, Decimal<U22, U17, i192>>();
  product::<Large, Large, Decimal<U38, U38, i256>>();
  product::<Supply, Volume, Decimal<U50, U26, u256>>();

  for lhs in [
    Price::MIN,
    Price::ZERO,
    Price::MAX,
    "64250.5".parse().unwrap(),
  ] {
    for rhs in [Quantity::MIN, Quantity::MAX, "-0.0015".parse().unwrap()] {
      assert_eq!(
        ExactMul::exact_mul(&lhs, &rhs),
        lhs.exact_mul(&rhs),
        "{lhs} * {rhs}"
      );
    }
  }
  for lhs in [Volume::ZERO, Volume::MAX, "64250.5".parse().unwrap()] {
    for rhs in [Lots::ZERO, Lots::MAX, "0.0015".parse().unwrap()] {
      assert_eq!(
        ExactMul::exact_mul(&lhs, &rhs),
        lhs.exact_mul(&rhs),
        "{lhs} * {rhs}"
      );
    }
    for rhs in [Quantity::MIN, Quantity::MAX] {
      assert_eq!(
        ExactMul::exact_mul(&lhs, &rhs),
        lhs.exact_mul(&rhs),
        "{lhs} * {rhs}"
      );
    }
  }
  for lhs in [Large::MIN, Large::MAX, "-1e18".parse().unwrap()] {
    for rhs in [Large::MIN, Large::MAX, "1e-19".parse().unwrap()] {
      assert_eq!(
        ExactMul::exact_mul(&lhs, &rhs),
        lhs.exact_mul(&rhs),
        "{lhs} * {rhs}"
      );
    }
  }
  for lhs in [Supply::MAX, "1.5".parse().unwrap()] {
    assert_eq!(
      ExactMul::exact_mul(&lhs, &Volume::MAX),
      lhs.exact_mul(&Volume::MAX),
      "{lhs}"
    );
  }

  let fills: [(Price, Quantity); 2] = [
    ("64250.5".parse().unwrap(), "0.0015".parse().unwrap()),
    ("64251".parse().unwrap(), "0.001".parse().unwrap()),
  ];
  assert_eq!(total(&fills), Some("160.62675".parse().unwrap()));
  assert_eq!(total(&[(Large::MAX, Large::MAX); 2]), None);
}

#[test]
fn widen_is_the_inherent_one() {
  type Small = Decimal<U2, U4>;
  type Fee = Decimal<U2, U4, u32>;
  type Large = Decimal<U38, U38, i256>;
  type Supply = Decimal<U60, U18, u320>;

  for value in [
    Small::MIN,
    Small::ZERO,
    Small::MAX,
    "-12.5".parse().unwrap(),
  ] {
    let widened: Decimal<U10, U8, i128> = WidenTo::widen(value);
    assert_eq!(widened, value.widen(), "{value}");
  }
  // An unsigned storage widens to a signed one.
  for value in [Fee::ZERO, Fee::MAX, "12.5".parse().unwrap()] {
    let widened: Decimal<U10, U8> = WidenTo::widen(value);
    assert_eq!(widened, value.widen(), "{value}");
  }
  for value in [Large::MIN, Large::MAX, "-1e-38".parse().unwrap()] {
    let widened: Decimal<U58, U38, i320> = WidenTo::widen(value);
    assert_eq!(widened, value.widen(), "{value}");
  }
  for value in [Supply::ZERO, Supply::MAX, "1.5".parse().unwrap()] {
    let widened: Decimal<U96, U18, u384> = WidenTo::widen(value);
    assert_eq!(widened, value.widen(), "{value}");
  }
}

#[test]
fn convert_to_is_the_inherent_one() {
  type Amount = Decimal<U10, U4>;
  type Narrow = Decimal<U2, U2>;
  type Large = Decimal<U38, U38, i256>;
  type Price = Decimal<U10, U8>;
  type Supply = Decimal<U60, U18, u320>;

  for value in [
    Amount::MIN,
    Amount::ZERO,
    Amount::MAX,
    "-12.5".parse().unwrap(),
    "1.2345".parse().unwrap(),
    "-0.005".parse().unwrap(),
    "99.995".parse().unwrap(),
  ] {
    let converted: Result<Narrow, _> = ConvertTo::try_convert(value);
    assert_eq!(converted, value.try_convert(), "{value}");
    for mode in MODES {
      let rounded: Result<Narrow, _> = ConvertTo::round_to(value, mode);
      assert_eq!(rounded, value.round_to(mode), "{value}, {mode:?}");
    }
  }
  for value in [
    Large::MIN,
    Large::MAX,
    "-64250.5".parse().unwrap(),
    "1e-30".parse().unwrap(),
  ] {
    let converted: Result<Price, _> = ConvertTo::try_convert(value);
    assert_eq!(converted, value.try_convert(), "{value}");
    let unsigned: Result<Supply, _> = ConvertTo::try_convert(value);
    assert_eq!(unsigned, value.try_convert(), "{value}");
    for mode in MODES {
      let rounded: Result<Price, _> = ConvertTo::round_to(value, mode);
      assert_eq!(rounded, value.round_to(mode), "{value}, {mode:?}");
    }
  }

  let amount: Amount = "123".parse().unwrap();
  assert_eq!(
    ConvertTo::<Narrow>::try_convert(amount),
    Err(ConvertError::OutOfRange)
  );
  let amount: Amount = "1.2345".parse().unwrap();
  assert_eq!(
    ConvertTo::<Narrow>::try_convert(amount),
    Err(ConvertError::Inexact)
  );
  assert_eq!(
    ConvertTo::<Narrow>::round_to(amount, RoundingMode::Up),
    Ok("1.24".parse().unwrap())
  );
  let negative: Large = "-0.5".parse().unwrap();
  assert_eq!(
    ConvertTo::<Supply>::try_convert(negative),
    Err(ConvertError::OutOfRange)
  );
}

#[test]
fn div_round_is_the_inherent_one() {
  type Budget = Decimal<U12, U2>;
  type Price = Decimal<U10, U8>;
  type Quantity = Decimal<U12, U8, i128>;
  type Large = Decimal<U19, U19>;
  type Wide = Decimal<U38, U38, i256>;

  for budget in [
    Budget::MIN,
    Budget::ZERO,
    Budget::MAX,
    "1000".parse().unwrap(),
  ] {
    for price in [
      Price::ZERO,
      Price::MIN,
      Price::MAX,
      "64250.5".parse().unwrap(),
      "-0.00000001".parse().unwrap(),
    ] {
      for mode in MODES {
        let quotient: Result<Quantity, _> = DivRound::div_round(&budget, &price, mode);
        assert_eq!(
          quotient,
          budget.div_round(&price, mode),
          "{budget} / {price}, {mode:?}"
        );
      }
    }
  }
  // An intermediate of 39 digits, in an `i192`, and one of 76 digits, in an `i256`.
  let divisor: Decimal<U1, U1> = "0.7".parse().unwrap();
  for value in [Large::MIN, Large::MAX, "1e-19".parse().unwrap()] {
    for mode in MODES {
      let quotient: Result<Decimal<U20, U19>, _> = DivRound::div_round(&value, &divisor, mode);
      assert_eq!(
        quotient,
        value.div_round(&divisor, mode),
        "{value}, {mode:?}"
      );
    }
  }
  for value in [Wide::MIN, Wide::MAX, "-1e-38".parse().unwrap()] {
    for price in [Price::MAX, "0.00000001".parse().unwrap()] {
      for mode in MODES {
        let quotient: Result<Decimal<U40, U20, i256>, _> =
          DivRound::div_round(&value, &price, mode);
        assert_eq!(
          quotient,
          value.div_round(&price, mode),
          "{value} / {price}, {mode:?}"
        );
      }
    }
  }

  let budget: Budget = "1000".parse().unwrap();
  assert_eq!(
    DivRound::<Price, Quantity>::div_round(&budget, &Price::ZERO, RoundingMode::Down),
    Err(DivideError::DivisionByZero)
  );
  let tick: Price = "0.00000001".parse().unwrap();
  assert_eq!(
    DivRound::<Price, Price>::div_round(&budget, &tick, RoundingMode::Down),
    Err(DivideError::OutOfRange)
  );
}

/// Every trait of both families and `Neg` are in scope at once: no call below is ambiguous
/// (E0034), on concrete types or in code bounded by either family, with the decimal traits of both
/// layers in one bound.
mod collisions {
  use core::ops::Neg;

  use typenum::{U8, U10};

  #[allow(
    unused_imports,
    reason = "every trait is in scope, as in code that imports all of them"
  )]
  use crate::{
    ArrayInteger, CheckedOps, ConstDecimalOne, ConstFixedDecimal, ConvertTo, Decimal, DecimalOne,
    DivRound, ExactMul, FixedDecimal, FixedInteger, FromLimbs, HasOne, NativeInteger,
    OverflowingOps, SaturatingOps, SignedFixedDecimal, SignedFixedInteger, StrictOps, UncheckedOps,
    UnsignedFixedDecimal, UnsignedFixedInteger, WidenTo, WrappingOps, i256, u256,
  };

  type Price = Decimal<U10, U8>;
  type Amount = Decimal<U10, U8, u64>;

  /// Zero, the limits and one through the functions and the constants, and the default.
  fn decimal_values<T>() -> [T; 9]
  where
    T: FixedDecimal + ConstFixedDecimal + DecimalOne + ConstDecimalOne,
  {
    [
      T::zero(),
      T::ZERO,
      T::min_value(),
      T::MIN,
      T::max_value(),
      T::MAX,
      T::one(),
      T::ONE,
      T::default(),
    ]
  }

  fn signed_decimal<T>(x: &T) -> (Option<T>, Option<T>, bool, bool, T, T)
  where
    T: FixedDecimal + ConstFixedDecimal + SignedFixedDecimal + DecimalOne + ConstDecimalOne,
  {
    (
      x.checked_add(&T::one()),
      x.checked_add(&T::ONE),
      x.is_zero(),
      x.is_negative(),
      x.neg(),
      x.abs(),
    )
  }

  fn unsigned_decimal<T>(x: &T) -> (Option<T>, Option<T>, bool)
  where
    T: FixedDecimal + ConstFixedDecimal + UnsignedFixedDecimal + DecimalOne + ConstDecimalOne,
  {
    (
      x.checked_add(&T::one()),
      x.checked_add(&T::ONE),
      x.is_zero(),
    )
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
      price.checked_add(&Price::one()),
      price.checked_add(&Price::ONE),
      price.is_zero(),
      price.is_negative(),
      price.neg(),
      price.abs(),
    );
    let next = Some("-0.5".parse().unwrap());
    assert_eq!(concrete, (next, next, false, true, -price, -price));
    assert_eq!(signed_decimal(&price), concrete);

    let concrete = [
      Price::zero(),
      Price::ZERO,
      Price::min_value(),
      Price::MIN,
      Price::max_value(),
      Price::MAX,
      Price::one(),
      Price::ONE,
      Price::default(),
    ];
    let (zero, min, max, one) = (Price::ZERO, Price::MIN, Price::MAX, Price::ONE);
    assert_eq!(concrete, [zero, zero, min, min, max, max, one, one, zero]);
    assert_eq!(decimal_values::<Price>(), concrete);

    let amount: Amount = "1.5".parse().unwrap();
    let concrete = (
      amount.checked_add(&Amount::one()),
      amount.checked_add(&Amount::ONE),
      amount.is_zero(),
    );
    let next = Some("2.5".parse().unwrap());
    assert_eq!(concrete, (next, next, false));
    assert_eq!(unsigned_decimal(&amount), concrete);

    let (zero, max, one) = (Amount::ZERO, Amount::MAX, Amount::ONE);
    assert_eq!(
      decimal_values::<Amount>(),
      [zero, zero, zero, zero, max, max, one, one, zero]
    );

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
