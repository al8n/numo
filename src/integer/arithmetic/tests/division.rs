//! The division of `Uint` and `Int`: the overflowing, checked, wrapping, saturating and strict
//! division and remainder of the families, rounded toward zero and Euclidean, the checked next
//! multiple and the exact division, and the functions of `IntegerMath` and `UnsignedMath` that
//! take two values: the Euclidean, rounded and exact division, the next multiple, the midpoint, the
//! absolute difference, the logarithm to any base and `is_multiple_of`. Each through the inherent
//! `const fn`s and the traits, against `i128` at every width up to 16 bits and against
//! `num-bigint` at wider ones; their panics; and their constants.

use typenum::{U3, U4, U5, U6, U10, U11, U12, U13, U14, U15};

use super::*;

/// What the division methods of the families return for a pair of values, each value read as `V`:
/// the overflowing, checked, wrapping and strict division and remainder, rounded toward zero and
/// Euclidean, in that order, then `saturating_div`, `checked_next_multiple_of` and
/// `checked_div_exact`. A method that would panic, on a zero divisor or on overflow, is not called
/// and gives `None`.
#[derive(Debug, PartialEq)]
struct Division<V> {
  overflowing: [Option<(V, bool)>; 4],
  checked: [Option<V>; 4],
  wrapping: [Option<V>; 4],
  strict: [Option<V>; 4],
  saturating: Option<V>,
  next_multiple: Option<V>,
  exact: Option<V>,
}

impl<V> Division<V> {
  /// What the methods return for a zero divisor: `None` from the checked methods, and no call of
  /// the others.
  fn by_zero() -> Self {
    Self {
      overflowing: [None, None, None, None],
      checked: [None, None, None, None],
      wrapping: [None, None, None, None],
      strict: [None, None, None, None],
      saturating: None,
      next_multiple: None,
      exact: None,
    }
  }
}

/// The [`Division`] of `$x` and `$y`, references to values of one type, which `$value` reads,
/// through the methods that the context reaches: the inherent `const fn`s for a concrete storage,
/// and the traits in generic code. A method that can panic is called only for a nonzero divisor,
/// and a strict one only where the checked method of the same operation returns a value.
macro_rules! division_results {
  ($x:expr, $y:expr, $value:expr) => {{
    let (x, y, value) = ($x, $y, $value);
    let divides = !y.is_zero();
    let read = |(result, overflow)| (value(&result), overflow);
    Division {
      overflowing: [
        divides.then(|| read(x.overflowing_div(y))),
        divides.then(|| read(x.overflowing_rem(y))),
        divides.then(|| read(x.overflowing_div_euclid(y))),
        divides.then(|| read(x.overflowing_rem_euclid(y))),
      ],
      checked: [
        x.checked_div(y).map(|result| value(&result)),
        x.checked_rem(y).map(|result| value(&result)),
        x.checked_div_euclid(y).map(|result| value(&result)),
        x.checked_rem_euclid(y).map(|result| value(&result)),
      ],
      wrapping: [
        divides.then(|| value(&x.wrapping_div(y))),
        divides.then(|| value(&x.wrapping_rem(y))),
        divides.then(|| value(&x.wrapping_div_euclid(y))),
        divides.then(|| value(&x.wrapping_rem_euclid(y))),
      ],
      strict: [
        x.checked_div(y).map(|_| value(&x.strict_div(y))),
        x.checked_rem(y).map(|_| value(&x.strict_rem(y))),
        x.checked_div_euclid(y)
          .map(|_| value(&x.strict_div_euclid(y))),
        x.checked_rem_euclid(y)
          .map(|_| value(&x.strict_rem_euclid(y))),
      ],
      saturating: divides.then(|| value(&x.saturating_div(y))),
      next_multiple: x.checked_next_multiple_of(y).map(|result| value(&result)),
      exact: x.checked_div_exact(y).map(|result| value(&result)),
    }
  }};
}

/// The [`Division`] of `x` and `y` through the traits.
fn division_through_traits<T, V>(x: &T, y: &T, value: fn(&T) -> V) -> Division<V>
where
  T: FixedInteger + OverflowingOps + CheckedOps + WrappingOps + SaturatingOps + StrictOps,
{
  division_results!(x, y, value)
}

/// The [`Division`] that the methods return for `a` and `b`, values of `bits` bits, signed if
/// `signed`, computed in `i128`.
fn expected_division(a: i128, b: i128, bits: u32, signed: bool) -> Division<i128> {
  if b == 0 {
    return Division::by_zero();
  }
  let (min, max) = range(bits, signed);
  // Only `MIN / -1` overflows, whose quotient, rounded either way, is `-MIN`.
  let (quotient, overflow) = wrap(a / b, bits, signed);
  let (euclid, euclid_overflow) = wrap(a.div_euclid(b), bits, signed);
  let overflowing = [
    (quotient, overflow),
    (a % b, overflow),
    (euclid, euclid_overflow),
    (a.rem_euclid(b), euclid_overflow),
  ];
  let checked = overflowing.map(|(result, overflow)| (!overflow).then_some(result));
  let next = bigint::next_multiple(a, b);
  Division {
    overflowing: overflowing.map(Some),
    checked,
    wrapping: overflowing.map(|(result, _)| Some(result)),
    strict: checked,
    saturating: Some(if overflow { max } else { quotient }),
    next_multiple: (min..=max).contains(&next).then_some(next),
    exact: (a % b == 0 && !overflow).then_some(quotient),
  }
}

/// The [`Division`] that the methods return for `a` and `b`, values of `bits` bits, signed if
/// `signed`, computed with `num-bigint`.
fn expected_big_division(a: &BigInt, b: &BigInt, bits: u32, signed: bool) -> Division<BigInt> {
  if *b == BigInt::ZERO {
    return Division::by_zero();
  }
  let (min, max) = (bigint::min(bits, signed), bigint::max(bits, signed));
  let wrapped = |exact: &BigInt| {
    (
      bigint::wrap(exact, bits, &min),
      *exact < min || max < *exact,
    )
  };
  let (quotient, overflow) = wrapped(&(a / b));
  let remainder = a % b;
  let (euclid_quotient, euclid_remainder) = bigint::div_rem_euclid(a, b);
  let (euclid, euclid_overflow) = wrapped(&euclid_quotient);
  let overflowing = [
    (quotient.clone(), overflow),
    (remainder.clone(), overflow),
    (euclid, euclid_overflow),
    (euclid_remainder, euclid_overflow),
  ];
  let checked = overflowing
    .clone()
    .map(|(result, overflow)| (!overflow).then_some(result));
  let next = bigint::next_multiple_of(a, b);
  Division {
    overflowing: overflowing.clone().map(Some),
    checked: checked.clone(),
    wrapping: overflowing.map(|(result, _)| Some(result)),
    strict: checked,
    saturating: Some(if overflow {
      max.clone()
    } else {
      quotient.clone()
    }),
    next_multiple: (min <= next && next <= max).then_some(next),
    exact: (remainder == BigInt::ZERO && !overflow).then_some(quotient),
  }
}

/// What the functions of `IntegerMath` that take two values return for a pair, each value read as
/// `V`: the Euclidean division and remainder, the division rounded down and up, and the exact
/// division where they do not panic, on a zero divisor or on `MIN / -1`; the next multiple where it
/// does not overflow; the midpoint; the absolute difference; and the logarithm to the base of the
/// second value, checked, and where it does not panic.
#[derive(Debug, PartialEq)]
struct Math<V> {
  euclid: [Option<V>; 2],
  rounded: [Option<V>; 2],
  exact: Option<Option<V>>,
  next_multiple: Option<V>,
  midpoint: V,
  abs_diff: V,
  log: [Option<u32>; 2],
}

/// The [`Math`] of `$x` and `$y`, references to values of one type, which `$value` reads, with the
/// absolute difference read by `$magnitude`, through the methods that the context reaches, as
/// [`division_results!`] does.
macro_rules! math_results {
  ($x:expr, $y:expr, $value:expr, $magnitude:expr) => {{
    let (x, y, value, magnitude) = ($x, $y, $value, $magnitude);
    // Each of these methods panics where the checked division returns `None`.
    let divides = x.checked_div(y).is_some();
    Math {
      euclid: [
        divides.then(|| value(&x.div_euclid(y))),
        divides.then(|| value(&x.rem_euclid(y))),
      ],
      rounded: [
        divides.then(|| value(&x.div_floor(y))),
        divides.then(|| value(&x.div_ceil(y))),
      ],
      exact: divides.then(|| x.div_exact(y).map(|quotient| value(&quotient))),
      next_multiple: x
        .checked_next_multiple_of(y)
        .map(|_| value(&x.next_multiple_of(y))),
      midpoint: value(&x.midpoint(y)),
      abs_diff: magnitude(&x.abs_diff(y)),
      log: [x.checked_ilog(y), x.checked_ilog(y).map(|_| x.ilog(y))],
    }
  }};
}

/// The [`Math`] of `x` and `y` through the traits.
fn math_through_traits<T, V>(
  x: &T,
  y: &T,
  value: fn(&T) -> V,
  magnitude: fn(&T::Magnitude) -> V,
) -> Math<V>
where
  T: CheckedOps + IntegerMath,
{
  math_results!(x, y, value, magnitude)
}

/// The [`Math`] that the functions return for `a` and `b`, values of `bits` bits, signed if
/// `signed`, computed in `i128`.
fn expected_math(a: i128, b: i128, bits: u32, signed: bool) -> Math<i128> {
  let (min, max) = range(bits, signed);
  // Only `MIN / -1` overflows, whose quotient, rounded any way, is `-MIN`.
  let divides = b != 0 && a / b <= max;
  let next = (b != 0).then(|| bigint::next_multiple(a, b));
  Math {
    euclid: [
      divides.then(|| a.div_euclid(b)),
      divides.then(|| a.rem_euclid(b)),
    ],
    rounded: [
      divides.then(|| bigint::floor_div(a, b)),
      divides.then(|| bigint::ceil_div(a, b)),
    ],
    exact: divides.then(|| (a % b == 0).then_some(a / b)),
    next_multiple: next.filter(|next| (min..=max).contains(next)),
    midpoint: (a + b) / 2,
    abs_diff: (a - b).abs(),
    log: [a.checked_ilog(b), a.checked_ilog(b)],
  }
}

/// The [`Math`] that the functions return for `a` and `b`, values of `bits` bits, signed if
/// `signed`, computed with `num-bigint`.
fn expected_big_math(a: &BigInt, b: &BigInt, bits: u32, signed: bool) -> Math<BigInt> {
  let (min, max) = (bigint::min(bits, signed), bigint::max(bits, signed));
  let zero = BigInt::ZERO;
  let divides = *b != zero && a / b <= max;
  let (euclid_quotient, euclid_remainder) = if *b == zero {
    (zero.clone(), zero.clone())
  } else {
    bigint::div_rem_euclid(a, b)
  };
  let next = (*b != zero).then(|| bigint::next_multiple_of(a, b));
  let log = bigint::ilog(a, b);
  Math {
    euclid: [
      divides.then_some(euclid_quotient),
      divides.then_some(euclid_remainder),
    ],
    rounded: [
      divides.then(|| bigint::div_floor(a, b)),
      divides.then(|| bigint::div_ceil(a, b)),
    ],
    exact: divides.then(|| (a % b == zero).then(|| a / b)),
    next_multiple: next.filter(|next| min <= *next && *next <= max),
    midpoint: (a + b) / 2u8,
    abs_diff: if a < b { b - a } else { a - b },
    log: [log, log],
  }
}

/// Checks the functions of `UnsignedMath` and `SignedMath` that take two values, for the sign
/// token `$sign`, on `$x` and `$y`, the values `$a` and `$b`, through the inherent `const fn`s and
/// the traits: `is_multiple_of` for `Uint`, and nothing for `Int`.
macro_rules! sign_division {
  (unsigned, $x:expr, $y:expr, $a:expr, $b:expr) => {{
    let (a, b) = ($a, $b);
    let expected = if b == 0 { a == 0 } else { a % b == 0 };
    assert_eq!(
      both!(UnsignedMath::is_multiple_of($x, $y)),
      expected,
      "{a} % {b}"
    );
  }};
  (signed, $x:expr, $y:expr, $a:expr, $b:expr) => {{}};
}

/// Checks the division of `$ty`, of the sign `$sign` and of `$bits` bits, whose values `$from`
/// creates from an `i128` and `$value` and `$magnitude` read back, at each pair of `$pairs`: the
/// inherent `const fn`s and the traits return the [`expected_division`] and the [`expected_math`],
/// and so does `unchecked_div_exact`, inherent and through the trait, wherever `checked_div_exact`
/// returns a value. Every width but zero bits, where every divisor is zero, divides 0 by 1, or at
/// one signed bit by -1, which the pairs of the ends include, so unless the tier checks no pair of
/// the storage, `unchecked_div_exact` is called.
macro_rules! sweep_division {
  ($sign:ident; $ty:ty, $bits:expr, $pairs:expr, $from:expr, $value:expr, $magnitude:expr) => {{
    let (bits, signed, pairs) = ($bits, is_signed!($sign), $pairs);
    let from: fn(i128) -> $ty = $from;
    let value: fn(&$ty) -> i128 = $value;
    let magnitude: fn(&<$ty as IntegerMath>::Magnitude) -> i128 = $magnitude;
    let checks_pairs = !pairs.is_empty();
    let mut exact_calls = 0;
    for (a, b) in pairs {
      let (x, y) = (from(a), from(b));
      let expected = expected_division(a, b, bits, signed);
      assert_eq!(
        division_results!(&x, &y, value),
        expected,
        "{a} and {b} in {bits} bits"
      );
      assert_eq!(
        division_through_traits(&x, &y, value),
        expected,
        "{a} and {b} in {bits} bits, through the traits"
      );
      let math = expected_math(a, b, bits, signed);
      assert_eq!(
        math_results!(&x, &y, value, magnitude),
        math,
        "{a} and {b} in {bits} bits"
      );
      assert_eq!(
        math_through_traits(&x, &y, value, magnitude),
        math,
        "{a} and {b} in {bits} bits, through the traits"
      );
      sign_division!($sign, &x, &y, a, b);
      if let Some(quotient) = expected.exact {
        // SAFETY: the assertions above show that `checked_div_exact` returns `Some`, the
        // precondition of `unchecked_div_exact`.
        let (inherent, through_trait) = unsafe {
          (
            x.unchecked_div_exact(&y),
            UncheckedOps::unchecked_div_exact(&x, &y),
          )
        };
        assert_eq!(
          (value(&inherent), value(&through_trait)),
          (quotient, quotient),
          "{a} / {b} exactly in {bits} bits"
        );
        exact_calls += 1;
      }
    }
    if bits > 0 && checks_pairs {
      assert!(exact_calls > 0, "no exact division in {bits} bits");
    }
  }};
}

/// Checks the division at the width `B`, `bits` bits, at most 8, in `u8` and `i8`, `[u8; 1]`,
/// `[u8; 2]` and `[u16; 1]` at the pairs of [`small_pairs`] for a storage that every tier checks,
/// and in `u16` and `i16`, `u128` and `i128`, `[u64; 1]` and `[u128; 1]` at those for another.
fn division_of_small_widths<B>(bits: u32)
where
  B: FitsBits<u8> + FitsBits<i8> + FitsBits<u16> + FitsBits<i16> + FitsBits<u128> + FitsBits<i128>,
  B: FitsBits<[u8; 1]> + FitsBits<[u8; 2]> + FitsBits<[u16; 1]>,
  B: FitsBits<[u64; 1]> + FitsBits<[u128; 1]>,
{
  let primary = |bits, signed| small_pairs(bits, signed, true);
  let secondary = |bits, signed| small_pairs(bits, signed, false);
  sweep_storage!(sweep_division, primary, native B, bits, u8, i8);
  sweep_storage!(sweep_division, primary, array B, bits, [u8; 1]);
  sweep_storage!(sweep_division, primary, array B, bits, [u8; 2]);
  sweep_storage!(sweep_division, primary, array B, bits, [u16; 1]);
  sweep_storage!(sweep_division, secondary, native B, bits, u16, i16);
  sweep_storage!(sweep_division, secondary, native B, bits, u128, i128);
  sweep_storage!(sweep_division, secondary, array B, bits, [u64; 1]);
  sweep_storage!(sweep_division, secondary, array B, bits, [u128; 1]);
}

/// Checks the division at the width `B`, `bits` bits, from 9 to 16, in `u16` and `i16`, `[u8; 2]`
/// and `[u16; 1]`, at the pairs of [`large_pairs`].
fn division_of_large_widths<B>(bits: u32)
where
  B: FitsBits<u16> + FitsBits<i16> + FitsBits<[u8; 2]> + FitsBits<[u16; 1]>,
{
  sweep_storage!(sweep_division, large_pairs, native B, bits, u16, i16);
  sweep_storage!(sweep_division, large_pairs, array B, bits, [u8; 2]);
  sweep_storage!(sweep_division, large_pairs, array B, bits, [u16; 1]);
}

/// Every pair of values of each width up to 8 bits, or the pairs that the tier takes, in native
/// integers and in arrays of one and two limbs, through the inherent `const fn`s and the traits,
/// against `i128`: the division rounded toward zero and Euclidean, by zero, by -1 and at
/// `MIN / -1`, at partial and full widths.
#[test]
fn families_divide_values_of_up_to_8_bits() {
  division_of_small_widths::<U0>(0);
  division_of_small_widths::<U1>(1);
  division_of_small_widths::<U2>(2);
  division_of_small_widths::<U3>(3);
  division_of_small_widths::<U4>(4);
  division_of_small_widths::<U5>(5);
  division_of_small_widths::<U6>(6);
  division_of_small_widths::<U7>(7);
  division_of_small_widths::<U8>(8);
}

/// The pairs that the tier takes of each width from 9 to 16 bits, with the pairs of the ends, in
/// `u16` and `i16`, in two limbs of 8 bits and in one of 16 bits. Miri checks 9, 12 and 16 bits.
#[test]
fn families_divide_values_of_9_to_16_bits() {
  division_of_large_widths::<U9>(9);
  division_of_large_widths::<U12>(12);
  division_of_large_widths::<U16>(16);
  if !tier().is_miri() {
    division_of_large_widths::<U10>(10);
    division_of_large_widths::<U11>(11);
    division_of_large_widths::<U13>(13);
    division_of_large_widths::<U14>(14);
    division_of_large_widths::<U15>(15);
  }
}

/// Checks the division methods of `T`, of `bits` bits, and its functions of `IntegerMath` that
/// take two values, through the traits at every pair of `samples` against `num-bigint`, and
/// `unchecked_div_exact` through the trait wherever `checked_div_exact` returns a value.
fn check_division_against_big_integers<T>(bits: u32, samples: &[BigInt])
where
  T: Big + FixedInteger + OverflowingOps + CheckedOps + WrappingOps + SaturatingOps + StrictOps,
  T: UncheckedOps + IntegerMath,
  T::Magnitude: Big,
{
  let values: Vec<T> = samples.iter().map(T::from_big).collect();
  for (a, x) in samples.iter().zip(&values) {
    for (b, y) in samples.iter().zip(&values) {
      let expected = expected_big_division(a, b, bits, T::SIGNED);
      assert_eq!(
        division_through_traits(x, y, |value| value.to_big()),
        expected,
        "{a} and {b} in {bits} bits"
      );
      assert_eq!(
        math_through_traits(x, y, |value| value.to_big(), |value| value.to_big()),
        expected_big_math(a, b, bits, T::SIGNED),
        "{a} and {b} in {bits} bits"
      );
      if let Some(quotient) = expected.exact {
        // SAFETY: the assertions above show that `checked_div_exact` returns `Some`, the
        // precondition of `unchecked_div_exact`.
        let result = unsafe { UncheckedOps::unchecked_div_exact(x, y) };
        assert_eq!(
          result.to_big(),
          quotient,
          "{a} / {b} exactly in {bits} bits"
        );
      }
    }
  }
}

/// Checks `is_multiple_of` of `T`, unsigned, through the trait at every pair of `samples` against
/// `num-bigint`.
fn check_multiples_against_big_integers<T: Big + UnsignedMath>(samples: &[BigInt]) {
  let zero = BigInt::ZERO;
  for a in samples {
    for b in samples {
      let expected = if *b == zero {
        *a == zero
      } else {
        a % b == zero
      };
      let multiple = T::from_big(a).is_multiple_of(&T::from_big(b));
      assert_eq!(multiple, expected, "{a} % {b}");
    }
  }
}

/// Checks that the inherent `const fn`s of the division of `$ty` and of its functions of
/// `IntegerMath` that take two values agree with the traits at every pair of `$samples`.
macro_rules! check_division_paths {
  ($ty:ty, $samples:expr) => {{
    let values: Vec<$ty> = $samples.iter().map(<$ty>::from_big).collect();
    let value: fn(&$ty) -> BigInt = |value| value.to_big();
    let magnitude: fn(&<$ty as IntegerMath>::Magnitude) -> BigInt = |value| value.to_big();
    for x in &values {
      for y in &values {
        assert_eq!(
          division_results!(x, y, value),
          division_through_traits(x, y, value),
          "{x} and {y}"
        );
        assert_eq!(
          math_results!(x, y, value, magnitude),
          math_through_traits(x, y, value, magnitude),
          "{x} and {y}"
        );
      }
    }
  }};
}

/// Checks the division of `Uint` and `Int` of `$bits` bits, the width `$width`, in each pair of
/// storages written `unsigned, signed;`, against `num-bigint`, with `$count` random samples.
macro_rules! division_agrees {
  ($width:ty, $bits:literal, $count:literal; $($uint:ty, $int:ty);+ $(;)?) => {{
    let unsigned = big_samples($bits, false, $count);
    let signed = big_samples($bits, true, $count);
    $(
      check_division_against_big_integers::<Uint<$width, $uint>>($bits, &unsigned);
      check_multiples_against_big_integers::<Uint<$width, $uint>>(&unsigned);
      check_division_paths!(Uint<$width, $uint>, unsigned);
      check_division_against_big_integers::<Int<$width, $int>>($bits, &signed);
      check_division_paths!(Int<$width, $int>, signed);
    )+
  }};
}

/// The edge and random values of 24 to 4096 bits in storages of each kind that hold them: the
/// division agrees with `num-bigint` through the traits, and the inherent `const fn`s agree with
/// the traits. An unoptimized build leaves out the widths above 200 bits, and Miri also 64, 128 and
/// 200 bits.
#[test]
#[cfg_attr(
  all(miri, target_pointer_width = "32"),
  ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
)]
fn storages_agree_on_the_division() {
  division_agrees!(U24, 24, 4; u32, i32; [u8; 3], [u8; 3]; [u16; 2], [u16; 2]);
  division_agrees!(U65, 65, 4; u128, i128; [u8; 9], [u8; 9]; [u64; 2], [u64; 2]);
  division_agrees!(U129, 129, 4; [u8; 17], [u8; 17]; [u64; 3], [u64; 3]; [u128; 2], [u128; 2]);
  if !tier().is_miri() {
    division_agrees!(U64, 64, 4; u64, i64; [u32; 2], [u32; 2]; [u64; 1], [u64; 1]);
    division_agrees!(U128, 128, 4; u128, i128; [u16; 8], [u16; 8]; [u128; 1], [u128; 1]);
    division_agrees!(U200, 200, 4; [u32; 7], [u32; 7]; [u64; 4], [u64; 4]);
  }
  if tier().is_full() {
    division_agrees!(U1024, 1024, 2; [u32; 32], [u32; 32]; [u64; 16], [u64; 16]);
    division_agrees!(U4096, 4096, 2; [u8; 512], [u8; 512]; [u64; 64], [u64; 64]);
    division_agrees!(U4096, 4096, 2; [u128; 32], [u128; 32]);
  }
}

/// The messages of the native integers for a zero divisor and for `MIN / -1`, in the division and
/// in the remainder, in that order, which every build panics with.
fn native_division_messages() -> [String; 4] {
  let message = |panic: Option<Panic>| panic.expect("a panic").0;
  [
    message(panic_of(|| black_box(1u8) / black_box(0))),
    message(panic_of(|| black_box(1u8) % black_box(0))),
    message(panic_of(|| black_box(i8::MIN) / black_box(-1))),
    message(panic_of(|| black_box(i8::MIN) % black_box(-1))),
  ]
}

/// Asserts that the division methods of the families that can panic panic on the divisor `$zero`,
/// zero, of `$x`, with the messages of `$messages`, through the inherent `const fn`s and the
/// traits, and report the caller.
macro_rules! division_by_zero_panics {
  ($x:expr, $zero:expr, $messages:expr) => {{
    let (x, zero) = ($x, $zero);
    let [div, rem, _, _] = $messages;
    panics_here!(x.overflowing_div(&zero), div);
    panics_here!(OverflowingOps::overflowing_div(&x, &zero), div);
    panics_here!(x.overflowing_rem(&zero), rem);
    panics_here!(OverflowingOps::overflowing_rem(&x, &zero), rem);
    panics_here!(x.overflowing_div_euclid(&zero), div);
    panics_here!(OverflowingOps::overflowing_div_euclid(&x, &zero), div);
    panics_here!(x.overflowing_rem_euclid(&zero), rem);
    panics_here!(OverflowingOps::overflowing_rem_euclid(&x, &zero), rem);
    panics_here!(x.wrapping_div(&zero), div);
    panics_here!(WrappingOps::wrapping_div(&x, &zero), div);
    panics_here!(x.wrapping_rem(&zero), rem);
    panics_here!(WrappingOps::wrapping_rem(&x, &zero), rem);
    panics_here!(x.wrapping_div_euclid(&zero), div);
    panics_here!(WrappingOps::wrapping_div_euclid(&x, &zero), div);
    panics_here!(x.wrapping_rem_euclid(&zero), rem);
    panics_here!(WrappingOps::wrapping_rem_euclid(&x, &zero), rem);
    panics_here!(x.saturating_div(&zero), div);
    panics_here!(SaturatingOps::saturating_div(&x, &zero), div);
    panics_here!(x.strict_div(&zero), div);
    panics_here!(StrictOps::strict_div(&x, &zero), div);
    panics_here!(x.strict_rem(&zero), rem);
    panics_here!(StrictOps::strict_rem(&x, &zero), rem);
    panics_here!(x.strict_div_euclid(&zero), div);
    panics_here!(StrictOps::strict_div_euclid(&x, &zero), div);
    panics_here!(x.strict_rem_euclid(&zero), rem);
    panics_here!(StrictOps::strict_rem_euclid(&x, &zero), rem);
  }};
}

/// Asserts that the strict division methods panic on `$min / $minus_one`, which overflows, with the
/// messages of `$messages`, through the inherent `const fn`s and the traits, and report the caller,
/// while the others wrap or saturate.
macro_rules! division_overflow_panics {
  ($min:expr, $minus_one:expr, $max:expr, $messages:expr) => {{
    let (min, minus_one) = ($min, $minus_one);
    let [_, _, div, rem] = $messages;
    panics_here!(min.strict_div(&minus_one), div);
    panics_here!(StrictOps::strict_div(&min, &minus_one), div);
    panics_here!(min.strict_rem(&minus_one), rem);
    panics_here!(StrictOps::strict_rem(&min, &minus_one), rem);
    panics_here!(min.strict_div_euclid(&minus_one), div);
    panics_here!(StrictOps::strict_div_euclid(&min, &minus_one), div);
    panics_here!(min.strict_rem_euclid(&minus_one), rem);
    panics_here!(StrictOps::strict_rem_euclid(&min, &minus_one), rem);
    assert_eq!(min.overflowing_div(&minus_one), (min, true));
    assert!(min.overflowing_rem(&minus_one).1);
    assert_eq!(min.wrapping_div_euclid(&minus_one), min);
    assert_eq!(min.saturating_div(&minus_one), $max);
    assert_eq!(min.checked_div(&minus_one), None);
    assert_eq!(min.checked_rem_euclid(&minus_one), None);
  }};
}

/// The division methods that can panic panic on a zero divisor with the messages of the native
/// division and remainder, and the strict ones also on `MIN / -1` with those of the native
/// overflow, in every build, through the inherent `const fn`s and the traits, and report the
/// caller where the native methods sometimes report a line in `core`: at a partial and at the full
/// width of a native integer and of an array, at zero bits, whose only divisor is zero, and at one
/// signed bit, where `-1 / -1` overflows.
#[test]
fn division_panics_report_the_caller() {
  let messages = native_division_messages();
  type I7 = Int<U7, i8>;
  type I8 = Int<U8, i8>;
  type I200 = Int<U200, [u64; 4]>;
  type I1 = Int<U1, i8>;
  type I1Array = Int<U1, [u8; 1]>;
  division_by_zero_panics!(u7::MAX, u7::ZERO, messages.clone());
  division_by_zero_panics!(Uint::<U8, u8>::MAX, Uint::<U8, u8>::ZERO, messages.clone());
  division_by_zero_panics!(Uint::<U200>::MAX, Uint::<U200>::ZERO, messages.clone());
  division_by_zero_panics!(u256::MAX, u256::ZERO, messages.clone());
  division_by_zero_panics!(I7::MIN, I7::ZERO, messages.clone());
  division_by_zero_panics!(I8::MAX, I8::ZERO, messages.clone());
  division_by_zero_panics!(I200::MIN, I200::ZERO, messages.clone());
  division_by_zero_panics!(i256::MAX, i256::ZERO, messages.clone());
  division_by_zero_panics!(Uint::<U0, u8>::ZERO, Uint::<U0, u8>::ZERO, messages.clone());
  division_by_zero_panics!(
    Int::<U0, [u8; 0]>::ZERO,
    Int::<U0, [u8; 0]>::ZERO,
    messages.clone()
  );
  division_by_zero_panics!(I1::MIN, I1::ZERO, messages.clone());
  division_overflow_panics!(I7::MIN, -I7::ONE, I7::MAX, messages.clone());
  division_overflow_panics!(I8::MIN, -I8::ONE, I8::MAX, messages.clone());
  division_overflow_panics!(I200::MIN, -I200::ONE, I200::MAX, messages.clone());
  division_overflow_panics!(i256::MIN, -i256::ONE, i256::MAX, messages.clone());
  // One bit holds only -1 and 0, so -1 is `MIN`.
  division_overflow_panics!(I1::MIN, I1::MIN, I1::MAX, messages.clone());
  division_overflow_panics!(I1Array::MIN, I1Array::MIN, I1Array::MAX, messages.clone());
}

/// The division of the families in constant contexts, in a native integer and in an array, at zero
/// bits and at one signed bit, with inputs that do not panic, and `unchecked_div_exact` with inputs
/// that meet its contract.
#[test]
fn division_in_constant_contexts() {
  type Bytes = Int<U24, [u8; 3]>;
  type Word = Uint<U7, [u64; 1]>;
  const OVERFLOWING: [(i7, bool); 4] = [
    i7::MIN.overflowing_div(&i7::new(-1)),
    i7::new(-7).overflowing_rem(&i7::new(2)),
    i7::new(-7).overflowing_div_euclid(&i7::new(2)),
    i7::MIN.overflowing_rem_euclid(&i7::new(-1)),
  ];
  const CHECKED: [Option<Bytes>; 4] = [
    Bytes::MIN.checked_div(&Bytes::MIN),
    Bytes::MIN.checked_rem(&Bytes::ZERO),
    Bytes::MAX.checked_div_euclid(&Bytes::MIN),
    Bytes::MIN.checked_rem_euclid(&Bytes::MAX),
  ];
  const WRAPPING: [Word; 4] = [
    Word::MAX.wrapping_div(&Word::ONE),
    Word::MAX.wrapping_rem(&Word::MAX),
    Word::MAX.wrapping_div_euclid(&Word::MAX),
    Word::ONE.wrapping_rem_euclid(&Word::MAX),
  ];
  const SATURATING: [i256; 2] = [
    i256::MIN.saturating_div(&i256::MIN),
    i256::MIN.saturating_div(&i256::MIN.wrapping_add(&i256::MAX)),
  ];
  const STRICT: [i7; 4] = [
    i7::new(-7).strict_div(&i7::new(2)),
    i7::new(-7).strict_rem(&i7::new(2)),
    i7::new(-7).strict_div_euclid(&i7::new(2)),
    i7::new(-7).strict_rem_euclid(&i7::new(2)),
  ];
  const MULTIPLES: [Option<Bytes>; 4] = [
    Bytes::ONE.checked_next_multiple_of(&Bytes::MAX),
    Bytes::ONE
      .wrapping_neg()
      .checked_next_multiple_of(&Bytes::MIN),
    Bytes::MAX.checked_next_multiple_of(&Bytes::ONE.wrapping_add(&Bytes::ONE)),
    Bytes::MIN.checked_next_multiple_of(&Bytes::ZERO),
  ];
  const EXACT: [Option<u7>; 3] = [
    u7::new(126).checked_div_exact(&u7::new(7)),
    u7::new(127).checked_div_exact(&u7::new(7)),
    u7::new(1).checked_div_exact(&u7::ZERO),
  ];
  // Every divisor of zero bits is zero.
  const ZERO_BITS: [Option<Uint<U0, u8>>; 3] = [
    Uint::<U0, u8>::ZERO.checked_div(&Uint::<U0, u8>::ZERO),
    Uint::<U0, u8>::ZERO.checked_next_multiple_of(&Uint::<U0, u8>::ZERO),
    Uint::<U0, u8>::ZERO.checked_div_exact(&Uint::<U0, u8>::ZERO),
  ];
  // The only nonzero divisor of one signed bit is -1, `MIN`.
  const ONE_BIT: [(Int<U1, i8>, bool); 2] = [
    Int::<U1, i8>::MIN.overflowing_div(&Int::<U1, i8>::MIN),
    Int::<U1, i8>::ZERO.overflowing_div(&Int::<U1, i8>::MIN),
  ];
  const ONE_BIT_EXACT: [Option<Int<U1, i8>>; 2] = [
    Int::<U1, i8>::MIN.checked_div_exact(&Int::<U1, i8>::MIN),
    Int::<U1, i8>::ZERO.checked_div_exact(&Int::<U1, i8>::MIN),
  ];
  // SAFETY: every divisor is nonzero and divides its value, and no value is `MIN` divided by -1, so
  // `checked_div_exact` would return every quotient.
  const UNCHECKED_EXACT: (u7, Int<U8, i8>, Bytes, Int<U1, i8>, i256) = unsafe {
    (
      u7::new(126).unchecked_div_exact(&u7::new(7)),
      Int::<U8, i8>::MIN.unchecked_div_exact(&Int::<U8, i8>::new(2)),
      Bytes::MIN.unchecked_div_exact(&Bytes::MIN),
      Int::<U1, i8>::ZERO.unchecked_div_exact(&Int::<U1, i8>::MIN),
      i256::MAX.unchecked_div_exact(&i256::MAX.wrapping_neg()),
    )
  };

  assert_eq!(
    OVERFLOWING,
    [
      (i7::MIN, true),
      (i7::new(-1), false),
      (i7::new(-4), false),
      (i7::ZERO, true)
    ]
  );
  assert_eq!(
    CHECKED,
    [
      Some(Bytes::ONE),
      None,
      Some(Bytes::ZERO),
      Some(Bytes::MAX.wrapping_sub(&Bytes::ONE))
    ]
  );
  assert_eq!(WRAPPING, [Word::MAX, Word::ZERO, Word::ONE, Word::ONE]);
  assert_eq!(SATURATING, [i256::ONE, i256::MAX]);
  assert_eq!(STRICT, [i7::new(-3), i7::new(-1), i7::new(-4), i7::ONE]);
  assert_eq!(EXACT, [Some(u7::new(18)), None, None]);
  assert_eq!(ZERO_BITS, [None, None, None]);
  assert_eq!(
    ONE_BIT,
    [(Int::<U1, i8>::MIN, true), (Int::<U1, i8>::ZERO, false)]
  );
  assert_eq!(ONE_BIT_EXACT, [None, Some(Int::<U1, i8>::ZERO)]);
  assert_eq!(MULTIPLES, [Some(Bytes::MAX), Some(Bytes::MIN), None, None]);
  assert_eq!(
    UNCHECKED_EXACT,
    (
      u7::new(18),
      Int::<U8, i8>::new(-64),
      Bytes::ONE,
      Int::<U1, i8>::ZERO,
      -i256::ONE
    )
  );
}

/// Asserts that the functions of `IntegerMath` that divide panic on the divisor `$zero`, zero, of
/// `$x`, with the messages of `$messages`, through the inherent `const fn`s and the traits, and
/// report the caller: the division ones with that of the native division, and the remainder,
/// `next_multiple_of` and `div_exact`, which computes the remainder first, with that of the
/// native remainder.
macro_rules! math_by_zero_panics {
  ($x:expr, $zero:expr, $messages:expr) => {{
    let (x, zero) = ($x, $zero);
    let [div, rem, _, _] = $messages;
    panics_here!(x.div_euclid(&zero), div);
    panics_here!(IntegerMath::div_euclid(&x, &zero), div);
    panics_here!(x.rem_euclid(&zero), rem);
    panics_here!(IntegerMath::rem_euclid(&x, &zero), rem);
    panics_here!(x.div_floor(&zero), div);
    panics_here!(IntegerMath::div_floor(&x, &zero), div);
    panics_here!(x.div_ceil(&zero), div);
    panics_here!(IntegerMath::div_ceil(&x, &zero), div);
    panics_here!(x.next_multiple_of(&zero), rem);
    panics_here!(IntegerMath::next_multiple_of(&x, &zero), rem);
    panics_here!(x.div_exact(&zero), rem);
    panics_here!(IntegerMath::div_exact(&x, &zero), rem);
  }};
}

/// Asserts that the functions of `IntegerMath` that divide panic on `$min / $minus_one`, which
/// overflows, with the messages of `$messages`, in every build, through the inherent `const fn`s
/// and the traits, and report the caller: `div_exact` with that of the remainder, which it computes
/// first. Every value is a multiple of -1, so `next_multiple_of` returns `$min`.
macro_rules! math_overflow_panics {
  ($min:expr, $minus_one:expr, $messages:expr) => {{
    let (min, minus_one) = ($min, $minus_one);
    let [_, _, div, rem] = $messages;
    panics_here!(min.div_euclid(&minus_one), div);
    panics_here!(IntegerMath::div_euclid(&min, &minus_one), div);
    panics_here!(min.rem_euclid(&minus_one), rem);
    panics_here!(IntegerMath::rem_euclid(&min, &minus_one), rem);
    panics_here!(min.div_floor(&minus_one), div);
    panics_here!(IntegerMath::div_floor(&min, &minus_one), div);
    panics_here!(min.div_ceil(&minus_one), div);
    panics_here!(IntegerMath::div_ceil(&min, &minus_one), div);
    panics_here!(min.div_exact(&minus_one), rem);
    panics_here!(IntegerMath::div_exact(&min, &minus_one), rem);
    assert_eq!(min.next_multiple_of(&minus_one), min);
    assert_eq!(min.checked_div_exact(&minus_one), None);
  }};
}

/// The functions of `IntegerMath` that divide panic on a zero divisor and on `MIN / -1` in every
/// build, as the native methods do, through the inherent `const fn`s and the traits, and report the
/// caller, at a partial and at the full width of a native integer and of an array, at zero bits and
/// at one signed bit. `div_exact` panics with the messages of the remainder, as the native method
/// does, which computes it first, although its documentation describes otherwise; the native
/// `next_multiple_of` also computes the remainder of a zero divisor first.
#[test]
fn math_division_panics_report_the_caller() {
  let mut messages = native_division_messages();
  messages[1] = panic_of(|| black_box(1u8).next_multiple_of(black_box(0)))
    .expect("a panic")
    .0;
  let exact = [
    messages[0].clone(),
    panic_of(|| black_box(1i8) % black_box(0))
      .expect("a panic")
      .0,
    messages[2].clone(),
    panic_of(|| black_box(i8::MIN) % black_box(-1))
      .expect("a panic")
      .0,
  ];
  assert_eq!(messages[1], exact[1]);
  type I7 = Int<U7, i8>;
  type I8 = Int<U8, i8>;
  type I200 = Int<U200, [u64; 4]>;
  type I1 = Int<U1, [u8; 1]>;
  math_by_zero_panics!(u7::MAX, u7::ZERO, messages.clone());
  math_by_zero_panics!(Uint::<U200>::MAX, Uint::<U200>::ZERO, messages.clone());
  math_by_zero_panics!(u256::ONE, u256::ZERO, messages.clone());
  math_by_zero_panics!(I7::MIN, I7::ZERO, messages.clone());
  math_by_zero_panics!(I8::MAX, I8::ZERO, messages.clone());
  math_by_zero_panics!(I200::MIN, I200::ZERO, messages.clone());
  math_by_zero_panics!(i256::MAX, i256::ZERO, messages.clone());
  math_by_zero_panics!(Uint::<U0, u8>::ZERO, Uint::<U0, u8>::ZERO, messages.clone());
  math_by_zero_panics!(Int::<U0, i16>::ZERO, Int::<U0, i16>::ZERO, messages.clone());
  math_by_zero_panics!(I1::MIN, I1::ZERO, messages.clone());
  math_overflow_panics!(I7::MIN, -I7::ONE, exact.clone());
  math_overflow_panics!(I8::MIN, -I8::ONE, exact.clone());
  math_overflow_panics!(I200::MIN, -I200::ONE, exact.clone());
  math_overflow_panics!(i256::MIN, -i256::ONE, exact.clone());
  // One bit holds only -1 and 0, so -1 is `MIN`.
  math_overflow_panics!(I1::MIN, I1::MIN, exact.clone());
  // A remainder that is not zero gives `None` and no panic.
  assert_eq!(I7::MIN.div_exact(&I7::new(3)), None);
  assert_eq!(i256::MAX.div_exact(&-i256::ONE), Some(-i256::MAX));
}

/// `next_multiple_of` overflows as a native addition does, which is how the native method
/// overflows: with overflow checks it panics with the message of the native addition and reports
/// the caller, and without them it returns the wrapped multiple, at a partial and at the full width
/// of a native integer and of an array.
#[test]
fn next_multiple_of_follows_overflow_checks() {
  // The message of a native addition that overflows, which only a build with checks has.
  let message = panic_of(|| black_box(u8::MAX) + 1).map_or_else(String::new, |panic| panic.0);
  type U200 = Uint<typenum::U200, [u64; 4]>;
  type U8 = Uint<typenum::U8, u8>;
  let two = U200::ONE + U200::ONE;
  overflows_here!(u7::MAX.next_multiple_of(&u7::new(2)), message, u7::ZERO);
  overflows_here!(U8::MAX.next_multiple_of(&U8::new(2)), message, U8::ZERO);
  // The next multiple of `MAX - 1` from `MAX` is twice it, `MAX - 3` modulo `2^200`.
  let below = U200::MAX - U200::ONE;
  overflows_here!(U200::MAX.next_multiple_of(&below), message, below - two);
  overflows_here!(i7::MIN.next_multiple_of(&i7::new(-3)), message, i7::new(62));
  let even = i256::ONE + i256::ONE;
  overflows_here!(i256::MAX.next_multiple_of(&even), message, i256::MIN);
}

/// The functions of `IntegerMath` and `UnsignedMath` that take two values in constant contexts, in
/// a native integer and in an array, at zero bits and at one signed bit.
#[test]
fn math_division_in_constant_contexts() {
  type Bytes = Int<U24, [u8; 3]>;
  type Word = Uint<U7, [u64; 1]>;
  type U0 = Uint<typenum::U0, u8>;
  type I1 = Int<U1, i8>;
  const EUCLID: [i7; 2] = [
    i7::new(-7).div_euclid(&i7::new(2)),
    i7::new(-7).rem_euclid(&i7::new(-2)),
  ];
  const ROUNDED: [Bytes; 2] = [
    Bytes::MIN.div_floor(&Bytes::MAX),
    Bytes::MIN.div_ceil(&Bytes::MAX),
  ];
  const EXACT: [Option<Word>; 2] = [
    Word::MAX.div_exact(&Word::MAX),
    Word::MAX.div_exact(&Word::ONE.wrapping_add(&Word::ONE)),
  ];
  const MULTIPLES: [u7; 2] = [
    u7::new(100).next_multiple_of(&u7::new(7)),
    u7::ZERO.next_multiple_of(&u7::MAX),
  ];
  const MIDPOINTS: [i256; 2] = [
    i256::MIN.midpoint(&i256::MAX),
    i256::MIN.midpoint(&i256::ZERO),
  ];
  const DIFFERENCES: (u7, Uint<U24, [u8; 3]>) =
    (i7::MIN.abs_diff(&i7::MAX), Bytes::MIN.abs_diff(&Bytes::MAX));
  const LOGS: [Option<u32>; 3] = [
    u7::new(81).checked_ilog(&u7::new(3)),
    Bytes::MAX.checked_ilog(&Bytes::ONE),
    Word::MAX.checked_ilog(&Word::ONE.wrapping_add(&Word::ONE)),
  ];
  const LOG: u32 = Bytes::MAX.ilog(&Bytes::MAX);
  const DIVIDES: [bool; 3] = [
    u7::new(42).is_multiple_of(&u7::new(7)),
    Word::ZERO.is_multiple_of(&Word::ZERO),
    Word::ONE.is_multiple_of(&Word::ZERO),
  ];
  const ZERO_BITS: (U0, U0, bool) = (
    U0::ZERO.midpoint(&U0::ZERO),
    U0::ZERO.abs_diff(&U0::ZERO),
    U0::ZERO.is_multiple_of(&U0::ZERO),
  );
  const ONE_BIT: (I1, Uint<U1, u8>, Option<I1>) = (
    I1::MIN.midpoint(&I1::ZERO),
    I1::MIN.abs_diff(&I1::ZERO),
    I1::ZERO.div_exact(&I1::MIN),
  );

  let minus = |count: i8| Bytes::checked_from_limbs(limbs_of(count.into())).expect("in range");
  assert_eq!(EUCLID, [i7::new(-4), i7::ONE]);
  assert_eq!(ROUNDED, [minus(-2), minus(-1)]);
  assert_eq!(EXACT, [Some(Word::ONE), None]);
  assert_eq!(MULTIPLES, [u7::new(105), u7::ZERO]);
  let half_min = i256::MIN.checked_div(&(i256::ONE + i256::ONE));
  assert_eq!(MIDPOINTS, [i256::ZERO, half_min.expect("in range")]);
  assert_eq!(DIFFERENCES, (u7::MAX, Uint::<U24, [u8; 3]>::MAX));
  assert_eq!(LOGS, [Some(4), None, Some(6)]);
  assert_eq!(LOG, 1);
  assert_eq!(DIVIDES, [true, true, false]);
  assert_eq!(ZERO_BITS, (U0::ZERO, U0::ZERO, true));
  assert_eq!(ONE_BIT, (I1::ZERO, Uint::<U1, u8>::ONE, Some(I1::ZERO)));
}

/// Checks `/` and `%` of `$ty`, whose values `$from` creates from an `i128`, at each pair of
/// `$pairs` where the checked division returns a value: the operators in their four forms of
/// values and references and their compound assignments with a value and with a reference give
/// the results of the checked division and remainder.
macro_rules! sweep_operators {
  ($sign:ident; $ty:ty, $bits:expr, $pairs:expr, $from:expr, $value:expr, $magnitude:expr) => {{
    let from: fn(i128) -> $ty = $from;
    for (a, b) in $pairs {
      let (x, y) = (from(a), from(b));
      if let (Some(quotient), Some(remainder)) = (x.checked_div(&y), x.checked_rem(&y)) {
        assert_eq!([x / y, x / &y, &x / y, &x / &y], [quotient; 4], "{a} / {b}");
        assert_eq!(
          [x % y, x % &y, &x % y, &x % &y],
          [remainder; 4],
          "{a} % {b}"
        );
        let (mut by_value, mut by_reference) = (x, x);
        by_value /= y;
        by_reference /= &y;
        assert_eq!([by_value, by_reference], [quotient; 2], "{a} /= {b}");
        let (mut by_value, mut by_reference) = (x, x);
        by_value %= y;
        by_reference %= &y;
        assert_eq!([by_value, by_reference], [remainder; 2], "{a} %= {b}");
      }
    }
  }};
}

/// Checks the division operators at the width `B`, `bits` bits, at most 8, in `u8` and `i8` and in
/// `[u8; 2]`, at the pairs of [`small_pairs`] for a storage that every tier checks.
#[allow(clippy::op_ref, reason = "the operators on references are under test")]
fn operators_of_small_widths<B>(bits: u32)
where
  B: FitsBits<u8> + FitsBits<i8> + FitsBits<[u8; 2]>,
{
  let pairs = |bits, signed| small_pairs(bits, signed, true);
  sweep_storage!(sweep_operators, pairs, native B, bits, u8, i8);
  sweep_storage!(sweep_operators, pairs, array B, bits, [u8; 2]);
}

/// Wherever the checked division returns a value, `/`, `%`, `/=` and `%=` in every form give its
/// results, at every width up to 8 bits in a native integer and in an array.
#[test]
fn division_operators_match_the_checked_methods() {
  operators_of_small_widths::<U0>(0);
  operators_of_small_widths::<U1>(1);
  operators_of_small_widths::<U2>(2);
  operators_of_small_widths::<U7>(7);
  operators_of_small_widths::<U8>(8);
}

/// `/`, `%`, `/=` and `%=` panic on a zero divisor and on `MIN / -1` in every build, with the
/// messages of the native operators, through every form, and report the caller, as the native
/// operators do; a panicking compound assignment leaves its target unchanged. At a partial and at
/// the full width of a native integer and of an array, at zero bits and at one signed bit.
#[test]
#[allow(clippy::op_ref, reason = "the operators on references are under test")]
fn division_operators_panic_in_every_build() {
  let messages = native_division_messages();
  // `$x / $y` and `$x % $y` panic with `$div` and `$rem`.
  macro_rules! operator_panics {
    ($x:expr, $y:expr, $div:expr, $rem:expr) => {{
      let (x, y, div, rem) = ($x, $y, &$div, &$rem);
      panics_here!(x / y, div);
      panics_here!(x / &y, div);
      panics_here!(&x / y, div);
      panics_here!(&x / &y, div);
      panics_here!(x % y, rem);
      panics_here!(x % &y, rem);
      panics_here!(&x % y, rem);
      panics_here!(&x % &y, rem);
      let mut target = x;
      panics_here!(target /= y, div);
      panics_here!(target /= &y, div);
      panics_here!(target %= y, rem);
      panics_here!(target %= &y, rem);
      assert_eq!(target, x);
    }};
  }
  let [div_by_zero, rem_by_zero, div_overflow, rem_overflow] = messages;
  type I7 = Int<U7, i8>;
  type I200 = Int<U200, [u64; 4]>;
  type I1 = Int<U1, [u8; 1]>;
  operator_panics!(u7::MAX, u7::ZERO, div_by_zero, rem_by_zero);
  operator_panics!(u256::MAX, u256::ZERO, div_by_zero, rem_by_zero);
  operator_panics!(I7::MIN, I7::ZERO, div_by_zero, rem_by_zero);
  operator_panics!(
    Int::<U8, i8>::MAX,
    Int::<U8, i8>::ZERO,
    div_by_zero,
    rem_by_zero
  );
  operator_panics!(I200::MAX, I200::ZERO, div_by_zero, rem_by_zero);
  operator_panics!(
    Uint::<U0, u8>::ZERO,
    Uint::<U0, u8>::ZERO,
    div_by_zero,
    rem_by_zero
  );
  operator_panics!(I7::MIN, -I7::ONE, div_overflow, rem_overflow);
  operator_panics!(
    Int::<U8, i8>::MIN,
    -Int::<U8, i8>::ONE,
    div_overflow,
    rem_overflow
  );
  operator_panics!(I200::MIN, -I200::ONE, div_overflow, rem_overflow);
  operator_panics!(i256::MIN, -i256::ONE, div_overflow, rem_overflow);
  // One bit holds only -1 and 0, so -1 is `MIN`.
  operator_panics!(I1::MIN, I1::MIN, div_overflow, rem_overflow);
}
