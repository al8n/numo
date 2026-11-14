//! The arithmetic of `Decimal` and `Scaled` within one type: `checked_rem`, `saturating_add` and
//! `saturating_sub` of every storage against the coefficients in `num-bigint`, at the limits and
//! in constant contexts, and the operators `+`, `-`, `+=` and `-=` and `Sum` against `checked_add`
//! and `checked_sub`, with their panics on a result outside the range. Each type checks in a
//! function of its own, whose frame holds the values of that type alone: the function is
//! `#[inline(never)]`, since an optimized build would inline a function that has one call site,
//! and the frame of the test would then hold the values of every type.
//!
//! The operators panic in every build, so these tests also run in an optimized build without
//! overflow checks, and nothing that they expect depends on a build setting.

use core::{
  iter::Sum,
  ops::{Add, Sub},
};
use std::{
  boxed::Box,
  cell::RefCell,
  format,
  panic::{self, AssertUnwindSafe},
  path::PathBuf,
  string::{String, ToString},
  sync::Once,
  thread_local,
  vec::Vec,
};

use num_bigint::BigInt;
use typenum::{
  U0, U1, U2, U4, U5, U8, U9, U10, U18, U19, U39, U58, U77, U78, U96, U97, U116, U135, U154,
};

use crate::{
  ConstFixedDecimal, Decimal, FixedDecimal, Layout, Scaled, Storage, bigint::tier, i192, i256,
  i320, i384, i448, i512, u192, u256, u320, u384, u448, u512,
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

/// A panic: its message, and the file and the line that it reports.
type Panic = (String, String, u32);

thread_local! {
  /// The panic of this thread while `panic_of` captures it: `None` outside `panic_of`, and
  /// `Some(None)` inside it until a panic.
  static CAPTURED: RefCell<Option<Option<Panic>>> = const { RefCell::new(None) };
}

/// Runs `f` and returns its panic, or `None` if it returns. The panic hook records the panic of
/// this thread meanwhile instead of printing it, and passes the panics of other threads and of
/// other code to the hook that it replaced.
fn panic_of<T>(f: impl FnOnce() -> T) -> Option<Panic> {
  static HOOK: Once = Once::new();
  HOOK.call_once(|| {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
      let recorded = CAPTURED.with(|captured| match captured.borrow_mut().as_mut() {
        Some(slot) => {
          let message = info.payload_as_str().unwrap_or_default().to_string();
          let (file, line) = info
            .location()
            .map_or((String::new(), 0), |at| (at.file().to_string(), at.line()));
          *slot = Some((message, file, line));
          true
        }
        None => false,
      });
      if !recorded {
        previous(info);
      }
    }));
  });
  CAPTURED.with(|captured| *captured.borrow_mut() = Some(None));
  let result = panic::catch_unwind(AssertUnwindSafe(f));
  let panic = CAPTURED
    .with(|captured| captured.borrow_mut().take())
    .flatten();
  assert_eq!(result.is_err(), panic.is_some(), "a panic and its record");
  panic
}

/// Asserts that `$value` panics with the message `$message` and reports the line of the outermost
/// macro invocation that leads to it, in its file, as `line!` does, so that invocation must fit on
/// one line.
macro_rules! panics_here {
  ($value:expr, $message:expr) => {{
    let expected = Some(($message.clone(), file!().to_string(), line!()));
    assert_eq!(panic_of(|| $value), expected, "{}", stringify!($value));
  }};
}

/// The messages of the native `+` and `-` where they check for overflow, which the operators of a
/// decimal panic with in every build.
fn overflow_messages() -> (String, String) {
  (
    "attempt to add with overflow".to_string(),
    "attempt to subtract with overflow".to_string(),
  )
}

/// Checks the operators of `T` at every pair of its [`samples`] against `checked_add` and
/// `checked_sub`. Where the checked method returns a value, `a + b`, `&a + &b` and `a += b` give
/// it, and `a - b`, `&a - &b` and `a -= b` likewise. Where it returns `None`, as for an unsigned
/// difference below zero, each form panics with the message of the native operator and reports its
/// own line, and the assignment leaves its target unchanged.
#[inline(never)]
#[allow(clippy::op_ref, reason = "the operators on references are under test")]
fn check_operators<T>()
where
  T: ConstFixedDecimal,
  for<'a> &'a T: Add<&'a T, Output = T> + Sub<&'a T, Output = T>,
{
  let (add, sub) = overflow_messages();
  let values = samples::<T>();
  for &a in &values {
    for &b in &values {
      match a.checked_add(&b) {
        Some(sum) => {
          assert_eq!(a + b, sum, "{a} + {b}");
          assert_eq!(&a + &b, sum, "&{a} + &{b}");
          let mut target = a;
          target += b;
          assert_eq!(target, sum, "{a} += {b}");
        }
        None => {
          panics_here!(a + b, add);
          panics_here!(&a + &b, add);
          let mut target = a;
          panics_here!(target += b, add);
          assert_eq!(target, a, "{a} += {b}");
        }
      }
      match a.checked_sub(&b) {
        Some(difference) => {
          assert_eq!(a - b, difference, "{a} - {b}");
          assert_eq!(&a - &b, difference, "&{a} - &{b}");
          let mut target = a;
          target -= b;
          assert_eq!(target, difference, "{a} -= {b}");
        }
        None => {
          panics_here!(a - b, sub);
          panics_here!(&a - &b, sub);
          let mut target = a;
          panics_here!(target -= b, sub);
          assert_eq!(target, a, "{a} -= {b}");
        }
      }
    }
  }
}

/// Checks `Sum` of `T` over values and over references against the fold of `checked_add` from
/// zero: over no item, which gives zero, over one item, which gives the item, over each pair of
/// the [`samples`], and over each pair followed by one of the first three samples. Where the fold
/// returns `None`, at the first partial sum outside the range, the sum panics with the message of
/// the native `+` and reports the fold of the library, in `file`, not its caller.
#[inline(never)]
fn check_sums<T>(file: &str)
where
  T: ConstFixedDecimal + for<'a> Sum<&'a T>,
{
  let (add, _) = overflow_messages();
  let values = samples::<T>();
  let check = |items: &[T]| {
    let fold = items
      .iter()
      .try_fold(T::zero(), |sum, value| sum.checked_add(value));
    match fold {
      Some(total) => {
        assert_eq!(items.iter().copied().sum::<T>(), total, "{items:?}");
        assert_eq!(items.iter().sum::<T>(), total, "{items:?}");
      }
      None => {
        let panics = [
          panic_of(|| items.iter().copied().sum::<T>()),
          panic_of(|| items.iter().sum::<T>()),
        ];
        for panic in panics {
          let (message, at, _) = panic.expect("a panic");
          assert_eq!(message, add, "{items:?}");
          // The location names the file with the separator of the host, and paths compare by
          // component.
          assert_eq!(PathBuf::from(at), PathBuf::from(file), "{items:?}");
        }
      }
    }
  };
  check(&[]);
  assert_eq!(core::iter::empty::<T>().sum::<T>(), T::ZERO);
  assert_eq!(core::iter::empty::<&T>().sum::<T>(), T::ZERO);
  for &a in &values {
    check(&[a]);
    assert_eq!([a].into_iter().sum::<T>(), a, "{a}");
    assert_eq!([a].iter().sum::<T>(), a, "{a}");
    for &b in &values {
      check(&[a, b]);
      for &c in &values[..3] {
        check(&[a, b, c]);
      }
    }
  }
}

/// Checks the operators of each type of the list, a bare decimal, with [`check_operators`].
macro_rules! operators_agree {
  ($($ty:ty),+ $(,)?) => {$(
    check_operators::<$ty>();
  )+};
}

/// Checks the sums of each type of the list, a bare decimal, with [`check_sums`].
macro_rules! sums_agree {
  ($($ty:ty),+ $(,)?) => {$(
    check_sums::<$ty>("src/decimal/ops.rs");
  )+};
}

/// The operators of one type of each storage give the results of `checked_add` and `checked_sub`
/// at every pair of the samples, and panic where those return `None`, at the line of the operator;
/// under Miri, those of a signed native, an unsigned native and a wide storage.
#[test]
fn operators_match_the_checked_methods() {
  operators_agree!(Di8, Du64, Di256);
  if tier().is_miri() {
    return;
  }
  operators_agree!(Di16, Di32, Di64, Di128, Di192, Di320, Di384, Di448, Di512);
  operators_agree!(
    Du8, Du16, Du32, Du128, Du192, Du256, Du320, Du384, Du448, Du512
  );
}

/// `Sum` of one type of each storage, over values and over references, gives the fold of
/// `checked_add` from zero, and panics in the fold of the library where that returns `None`; under
/// Miri, that of a signed native, an unsigned native and a wide storage.
#[test]
fn sums_match_the_fold_of_checked_add() {
  sums_agree!(Di8, Du64, Di256);
  if tier().is_miri() {
    return;
  }
  sums_agree!(Di16, Di32, Di64, Di128, Di192, Di320, Di384, Di448, Di512);
  sums_agree!(
    Du8, Du16, Du32, Du128, Du192, Du256, Du320, Du384, Du448, Du512
  );
}

/// The operators and the sums of a `Scaled` over a signed native, an unsigned native and a wide
/// storage give the results of its `checked_add` and `checked_sub`, and panic where those return
/// `None`, an operator at its own line and a sum in the fold of the library.
#[test]
fn scaled_operators_and_sums_match_the_checked_methods() {
  check_operators::<Scaled<Di64>>();
  check_operators::<Scaled<Du64>>();
  check_operators::<Scaled<Di256>>();
  check_sums::<Scaled<Di64>>("src/decimal/scaled/mod.rs");
  check_sums::<Scaled<Du64>>("src/decimal/scaled/mod.rs");
  check_sums::<Scaled<Di256>>("src/decimal/scaled/mod.rs");
}

/// The operators and the sums on values that are written out, in a signed native storage and in
/// an unsigned wide one.
#[test]
#[allow(clippy::op_ref, reason = "the operators on references are under test")]
fn operators_add_and_subtract() {
  let (a, b): (Di64, Di64) = ("1.5".parse().unwrap(), "-0.25".parse().unwrap());
  assert_eq!((a + b).to_string(), "1.25");
  assert_eq!((a - b).to_string(), "1.75");
  assert_eq!((&a + &b).to_string(), "1.25");
  assert_eq!((&a - &b).to_string(), "1.75");
  let mut total = a;
  total += b;
  assert_eq!(total.to_string(), "1.25");
  total -= a;
  assert_eq!(total, b);
  assert_eq!([a, b, a].into_iter().sum::<Di64>().to_string(), "2.75");
  assert_eq!([a, b, a].iter().sum::<Di64>().to_string(), "2.75");

  let (a, b): (Du320, Du320) = ("1.5".parse().unwrap(), "0.25".parse().unwrap());
  assert_eq!((a + b).to_string(), "1.75");
  assert_eq!((a - b).to_string(), "1.25");
  assert_eq!((&a + &b).to_string(), "1.75");
  assert_eq!((&a - &b).to_string(), "1.25");
  let mut total = a;
  total += b;
  assert_eq!(total.to_string(), "1.75");
  total -= a;
  assert_eq!(total, b);
  assert_eq!([a, b, a].into_iter().sum::<Du320>().to_string(), "3.25");
  assert_eq!([a, b, a].iter().sum::<Du320>().to_string(), "3.25");
}

/// A sum above the range, a difference below it and an unsigned difference below zero panic with
/// the messages of the native operators, at the line of the operator, in each form. A panicking
/// assignment leaves its target, and for a `Scaled` the scale of its target.
#[test]
#[allow(clippy::op_ref, reason = "the operators on references are under test")]
fn results_outside_the_range_panic_at_the_operator() {
  let (add, sub) = overflow_messages();
  let (min, max, one) = (Di64::MIN, Di64::MAX, Di64::ONE);
  panics_here!(max + one, add);
  panics_here!(&max + &one, add);
  panics_here!(min - one, sub);
  panics_here!(&min - &one, sub);
  let mut target = max;
  panics_here!(target += one, add);
  assert_eq!(target, max);
  let mut target = min;
  panics_here!(target -= one, sub);
  assert_eq!(target, min);

  let (half, one): (Du64, Du64) = ("0.5".parse().unwrap(), Du64::ONE);
  panics_here!(half - one, sub);
  panics_here!(&half - &one, sub);
  panics_here!(Du64::ZERO - Du64::MAX, sub);
  panics_here!(Du64::MAX + half, add);
  let mut target = half;
  panics_here!(target -= one, sub);
  assert_eq!(target, half);

  let (min, max) = (Scaled::new(Di64::MIN, 3), Scaled::new(Di64::MAX, 3));
  let one = Scaled::new(Di64::ONE, 7);
  panics_here!(max + one, add);
  panics_here!(&max + &one, add);
  panics_here!(min - one, sub);
  panics_here!(&min - &one, sub);
  let mut target = max;
  panics_here!(target += one, add);
  assert_eq!(target.into_parts(), (Di64::MAX, 3));
  let mut target = min;
  panics_here!(target -= one, sub);
  assert_eq!(target.into_parts(), (Di64::MIN, 3));
}

/// A sum over an iterator panics at the first partial sum outside the range, also where the whole
/// sum is in range, with the message of the native `+`, and reports the fold of the library, not
/// its caller: for a `Decimal` and for a `Scaled`, over values and over references.
#[test]
fn a_partial_sum_outside_the_range_panics_in_the_fold() {
  let (add, _) = overflow_messages();
  let items = [Di64::MAX, Di64::ONE, Di64::MIN];
  let panics = [
    panic_of(|| items.into_iter().sum::<Di64>()),
    panic_of(|| items.iter().sum::<Di64>()),
  ];
  for panic in panics {
    let (message, file, _) = panic.expect("a panic");
    assert_eq!(message, add);
    assert_ne!(file, file!());
    assert_eq!(PathBuf::from(file), PathBuf::from("src/decimal/ops.rs"));
  }
  // In an order whose partial sums are in range, the same values add up.
  let ordered = [Di64::MAX, Di64::MIN, Di64::ONE];
  assert_eq!(ordered.into_iter().sum::<Di64>(), Di64::ONE);
  assert_eq!(ordered.iter().sum::<Di64>(), Di64::ONE);

  let items = items.map(Scaled::from);
  let panics = [
    panic_of(|| items.into_iter().sum::<Scaled<Di64>>()),
    panic_of(|| items.iter().sum::<Scaled<Di64>>()),
  ];
  for panic in panics {
    let (message, file, _) = panic.expect("a panic");
    assert_eq!(message, add);
    assert_ne!(file, file!());
    assert_eq!(
      PathBuf::from(file),
      PathBuf::from("src/decimal/scaled/mod.rs")
    );
  }
}

/// Checks that the sum of no item of `T` is `zero` and the sum of the one item `item` is `item`,
/// over values and over references, comparing what `parts` gives: the value, and for a `Scaled`
/// the scale too.
#[inline(never)]
fn check_sums_of_none_and_one<T, P>(item: T, zero: P, parts: fn(T) -> P)
where
  T: FixedDecimal + for<'a> Sum<&'a T>,
  P: PartialEq + core::fmt::Debug,
{
  assert_eq!(parts(core::iter::empty::<T>().sum::<T>()), zero);
  assert_eq!(parts(core::iter::empty::<&T>().sum::<T>()), zero);
  let expected = parts(item.clone());
  assert_eq!(parts(core::iter::once(item.clone()).sum::<T>()), expected);
  assert_eq!(parts(core::iter::once(&item).sum::<T>()), expected);
}

/// A sum of no item is zero, with scale 0 for a `Scaled`, and a sum of one item is that item, with
/// its scale, over values and over references: for a decimal in a native and in a wide storage,
/// and for a `Scaled` over each.
#[test]
fn sums_of_no_item_and_of_one() {
  let (signed, unsigned): (Di64, Du320) = ("-1.5".parse().unwrap(), "0.25".parse().unwrap());
  check_sums_of_none_and_one(signed, Di64::ZERO, |value| value);
  check_sums_of_none_and_one(unsigned, Du320::ZERO, |value| value);
  check_sums_of_none_and_one(Scaled::new(signed, 3), (Di64::ZERO, 0), Scaled::into_parts);
  check_sums_of_none_and_one(
    Scaled::new(unsigned, 5),
    (Du320::ZERO, 0),
    Scaled::into_parts,
  );
}
