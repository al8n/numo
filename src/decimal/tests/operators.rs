//! The remainder and the saturating sum and difference of `Decimal`: `checked_rem`,
//! `saturating_add` and `saturating_sub` of every storage against the coefficients in
//! `num-bigint`, at the limits and in constant contexts. Each type checks in a function of its
//! own, whose frame holds the values of that type alone: the function is `#[inline(never)]`, since
//! an optimized build would inline a function that has one call site, which would put every type
//! in the frame of the test, beyond the stack of a test thread.

use std::{
  format,
  string::{String, ToString},
  vec::Vec,
};

use num_bigint::BigInt;
use typenum::{
  U0, U1, U2, U4, U5, U8, U9, U10, U18, U19, U39, U58, U77, U78, U96, U97, U116, U135, U154,
};

use crate::{
  Decimal, FixedDecimal, Layout, Storage, bigint::tier, i192, i256, i320, i384, i448, i512, u192,
  u256, u320, u384, u448, u512,
};

// One type of each storage, most of them at the digit limit of the storage, where a sum can leave
// the storage itself: among them types without integral digits and types without fraction digits.
type Di8 = Decimal<U1, U1, i8>;
type Di16 = Decimal<U0, U4, i16>;
type Di32 = Decimal<U5, U4, i32>;
type Di64 = Decimal<U10, U8, i64>;
type Di128 = Decimal<U19, U19, i128>;
type Di192 = Decimal<U39, U18, i192>;
type Di256 = Decimal<U58, U18, i256>;
type Di320 = Decimal<U96, U0, i320>;
type Di384 = Decimal<U97, U18, i384>;
type Di448 = Decimal<U116, U18, i448>;
type Di512 = Decimal<U135, U18, i512>;
type Du8 = Decimal<U1, U1, u8>;
type Du16 = Decimal<U2, U2, u16>;
type Du32 = Decimal<U0, U9, u32>;
type Du64 = Decimal<U10, U9, u64>;
type Du128 = Decimal<U19, U19, u128>;
type Du192 = Decimal<U39, U18, u192>;
type Du256 = Decimal<U77, U0, u256>;
type Du320 = Decimal<U78, U18, u320>;
type Du384 = Decimal<U97, U18, u384>;
type Du448 = Decimal<U116, U18, u448>;
type Du512 = Decimal<U154, U0, u512>;

/// The values at which a type is checked: zero, the limits, one, two and 25 units of the last
/// digit, negated too for a signed storage, and the values one unit inside the limits, each once;
/// under Miri the first five.
fn samples<T: FixedDecimal>() -> Vec<T> {
  let units = |count: i32| -> T {
    format!("{count}e-{}", T::FRACTION)
      .parse()
      .expect("in range")
  };
  let mut candidates = Vec::from([T::zero(), T::min_value(), T::max_value()]);
  for count in [1, 2, 25] {
    candidates.push(units(count));
    if T::SIGNED {
      candidates.push(units(-count));
    }
  }
  let unit = units(1);
  candidates.push(T::max_value().checked_sub(&unit).expect("in range"));
  candidates.push(T::min_value().checked_add(&unit).expect("in range"));
  // The smallest value of an unsigned storage is zero, and the value beside it one unit.
  let mut values = Vec::new();
  for value in candidates {
    if !values.contains(&value) {
      values.push(value);
    }
  }
  if tier().is_miri() {
    values.truncate(5);
  }
  values
}

/// The coefficient of `value`: the integer that its text with every fraction digit of the type
/// writes, without the point.
fn coefficient<I, F, R>(value: &Decimal<I, F, R>) -> BigInt
where
  R: Storage,
  (I, F): Layout<R>,
{
  let digits = Decimal::<I, F, R>::FRACTION as usize;
  format!("{value:.digits$}")
    .replace('.', "")
    .parse()
    .expect("an integer")
}

/// The value of `T` whose coefficient is `coefficient`, which must be in the range of `T`.
fn of_coefficient<T: FixedDecimal>(coefficient: &BigInt) -> T {
  format!("{coefficient}e-{}", T::FRACTION)
    .parse()
    .expect("in range")
}

/// Checks `checked_rem`, `saturating_add` and `saturating_sub`, the inherent `const fn`s, of each
/// type of the list at every pair of its [`samples`] against the coefficients in `num-bigint`: the
/// remainder, whose `%` truncates as that of the native integers does, and the sum and the
/// difference, moved to the limit that they pass. Then a divisor of one unit of either sign, and
/// the limits as dividends. Each type checks in a function of its own.
macro_rules! methods_agree {
  ($($ty:ty),+ $(,)?) => {$({
    #[inline(never)]
    fn storage() {
      type T = $ty;
      let values = samples::<T>();
      let coefficients: Vec<BigInt> = values.iter().map(coefficient).collect();
      let (min, max) = (coefficient(&T::MIN), coefficient(&T::MAX));
      for (a, x) in values.iter().zip(&coefficients) {
        for (b, y) in values.iter().zip(&coefficients) {
          let remainder = (*y != BigInt::ZERO).then(|| of_coefficient::<T>(&(x % y)));
          assert_eq!(a.checked_rem(b), remainder, "{a} % {b}");
          assert_eq!(a.checked_rem(b).is_none(), b.is_zero(), "{a} % {b}");
          let sum = of_coefficient::<T>(&(x + y).clamp(min.clone(), max.clone()));
          assert_eq!(a.saturating_add(b), sum, "{a} + {b}");
          let difference = of_coefficient::<T>(&(x - y).clamp(min.clone(), max.clone()));
          assert_eq!(a.saturating_sub(b), difference, "{a} - {b}");
        }
      }

      // Every value is a whole number of units of the last digit, and the largest coefficient,
      // `10^DIGITS - 1`, is odd.
      let unit = of_coefficient::<T>(&BigInt::from(1));
      let two_units = of_coefficient::<T>(&BigInt::from(2));
      for a in &values {
        assert_eq!(a.checked_rem(&unit), Some(T::ZERO), "{a}");
      }
      assert_eq!(T::MAX.checked_rem(&T::MAX), Some(T::ZERO));
      assert_eq!(T::MIN.checked_rem(&T::MAX), Some(T::ZERO));
      assert_eq!(T::MAX.checked_rem(&two_units), Some(unit));
      if T::SIGNED {
        // The remainder has the sign of the dividend, whatever the sign of the divisor.
        let minus_unit = of_coefficient::<T>(&BigInt::from(-1));
        for a in &values {
          assert_eq!(a.checked_rem(&minus_unit), Some(T::ZERO), "{a}");
        }
        assert_eq!(T::MAX.checked_rem(&T::MIN), Some(T::ZERO));
        assert_eq!(T::MIN.checked_rem(&two_units), Some(minus_unit));
        assert_eq!(T::MAX.checked_rem(&of_coefficient::<T>(&BigInt::from(-2))), Some(unit));
      }
    }
    storage();
  })+};
}

/// `checked_rem`, `saturating_add` and `saturating_sub` of one type of each storage against the
/// coefficients in `num-bigint`, at every pair of the samples; under Miri, of a signed native, an
/// unsigned native and a wide storage.
#[test]
fn methods_match_the_coefficients() {
  methods_agree!(Di8, Du64, Di256);
  if tier().is_miri() {
    return;
  }
  methods_agree!(Di16, Di32, Di64, Di128, Di192, Di320, Di384, Di448, Di512);
  methods_agree!(
    Du8, Du16, Du32, Du128, Du192, Du256, Du320, Du384, Du448, Du512
  );
}

/// The sum, the difference and the remainder of the limits with each other, and the remainder by
/// zero, through the checked and the saturating methods, for a signed storage and for an unsigned
/// one, whose smallest value is zero.
#[test]
fn limits_give_the_checked_and_the_saturated_results() {
  let (zero, min, max) = (Di64::ZERO, Di64::MIN, Di64::MAX);
  assert_eq!(max.checked_add(&max), None);
  assert_eq!(max.checked_add(&min), Some(zero));
  assert_eq!(max.checked_rem(&zero), None);
  assert_eq!(max.checked_sub(&max), Some(zero));
  assert_eq!(max.checked_sub(&min), None);
  assert_eq!(min.checked_add(&max), Some(zero));
  assert_eq!(min.checked_add(&min), None);
  assert_eq!(min.checked_rem(&zero), None);
  assert_eq!(min.checked_sub(&max), None);
  assert_eq!(min.checked_sub(&min), Some(zero));

  assert_eq!(max.saturating_add(&max), max);
  assert_eq!(max.saturating_add(&min), zero);
  assert_eq!(max.saturating_sub(&max), zero);
  assert_eq!(max.saturating_sub(&min), max);
  assert_eq!(min.saturating_add(&max), zero);
  assert_eq!(min.saturating_add(&min), min);
  assert_eq!(min.saturating_sub(&max), min);
  assert_eq!(min.saturating_sub(&min), zero);

  let (zero, max) = (Du64::ZERO, Du64::MAX);
  assert_eq!(Du64::MIN, zero);
  assert_eq!(max.checked_add(&max), None);
  assert_eq!(max.checked_add(&zero), Some(max));
  assert_eq!(max.checked_rem(&zero), None);
  assert_eq!(max.checked_sub(&max), Some(zero));
  assert_eq!(max.checked_sub(&zero), Some(max));
  assert_eq!(zero.checked_add(&max), Some(max));
  assert_eq!(zero.checked_add(&zero), Some(zero));
  assert_eq!(zero.checked_rem(&zero), None);
  assert_eq!(zero.checked_sub(&max), None);
  assert_eq!(zero.checked_sub(&zero), Some(zero));

  assert_eq!(max.saturating_add(&max), max);
  assert_eq!(max.saturating_add(&zero), max);
  assert_eq!(max.saturating_sub(&max), zero);
  assert_eq!(max.saturating_sub(&zero), max);
  assert_eq!(zero.saturating_add(&max), max);
  assert_eq!(zero.saturating_add(&zero), zero);
  assert_eq!(zero.saturating_sub(&max), zero);
  assert_eq!(zero.saturating_sub(&zero), zero);
}

/// An unsigned difference below zero saturates to zero, the limit that it passes, although both
/// operands are positive or zero, and an unsigned sum above the range to the largest value: in a
/// native storage, where the difference leaves the storage itself, and in a wide one.
#[test]
fn unsigned_results_saturate_to_the_limit_that_they_pass() {
  let half: Du64 = "0.5".parse().unwrap();
  assert_eq!(half.saturating_sub(&Du64::ONE), Du64::ZERO);
  assert_eq!(Du64::ZERO.saturating_sub(&Du64::MAX), Du64::ZERO);
  assert_eq!(Du64::ONE.saturating_sub(&half), half);
  assert_eq!(Du64::MAX.saturating_add(&half), Du64::MAX);
  assert_eq!(half.saturating_add(&Du64::MAX), Du64::MAX);

  let half: Du320 = "0.5".parse().unwrap();
  assert_eq!(half.saturating_sub(&Du320::ONE), Du320::ZERO);
  assert_eq!(Du320::ZERO.saturating_sub(&Du320::MAX), Du320::ZERO);
  assert_eq!(Du320::ONE.saturating_sub(&half), half);
  assert_eq!(Du320::MAX.saturating_add(&half), Du320::MAX);
  assert_eq!(half.saturating_add(&Du320::MAX), Du320::MAX);
}

/// The three methods in constant contexts, in a signed native storage and in an unsigned wide one.
#[test]
fn methods_in_constant_contexts() {
  const fn price(text: &str) -> Di64 {
    match Di64::parse(text) {
      Ok(price) => price,
      Err(_) => panic!("a price"),
    }
  }
  const DIVIDEND: Di64 = price("-7.5");
  const DIVISOR: Di64 = price("2");
  const REMAINDERS: [Option<Di64>; 3] = [
    DIVIDEND.checked_rem(&DIVISOR),
    DIVIDEND.checked_rem(&Di64::ZERO),
    Di64::MAX.checked_rem(&Di64::MIN),
  ];
  const SUMS: [Di64; 3] = [
    DIVIDEND.saturating_add(&DIVISOR),
    Di64::MAX.saturating_add(&DIVISOR),
    Di64::MIN.saturating_add(&DIVIDEND),
  ];
  const DIFFERENCES: [Di64; 3] = [
    DIVIDEND.saturating_sub(&DIVISOR),
    Di64::MIN.saturating_sub(&DIVISOR),
    Di64::MAX.saturating_sub(&DIVIDEND),
  ];
  assert_eq!(REMAINDERS, [Some(price("-1.5")), None, Some(Di64::ZERO)]);
  assert_eq!(SUMS, [price("-5.5"), Di64::MAX, Di64::MIN]);
  assert_eq!(DIFFERENCES, [price("-9.5"), Di64::MIN, Di64::MAX]);

  const UNIT: Du320 = match Du320::parse("1e-18") {
    Ok(unit) => unit,
    Err(_) => panic!("an amount"),
  };
  const REMAINDER: Option<Du320> = Du320::MAX.checked_rem(&Du320::ONE);
  const SUM: Du320 = Du320::MAX.saturating_add(&UNIT);
  const DIFFERENCE: Du320 = Du320::ZERO.saturating_sub(&UNIT);
  let fraction: Option<String> = REMAINDER.map(|remainder| remainder.to_string());
  assert_eq!(fraction.as_deref(), Some("0.999999999999999999"));
  assert_eq!((SUM, DIFFERENCE), (Du320::MAX, Du320::ZERO));
}
