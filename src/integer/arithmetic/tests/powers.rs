//! The powers of `Uint` and `Int`, the overflowing, checked, wrapping, saturating and strict power
//! of the families and `pow`, and the functions of `IntegerMath`, `UnsignedMath` and `SignedMath`
//! that take one value: the roots, the logarithms to the bases 2 and 10 and the next power of two.
//! Each through the inherent `const fn`s and the traits, against `i128` at every width up to 16
//! bits and against `num-bigint` at wider ones; their panics; and their constants.

use typenum::{U3, U4, U5, U6, U10, U11, U12, U13, U14, U15};

use super::*;

/// What the power methods return for a value and an exponent, each value read as `V`: the
/// overflowing, checked, wrapping and saturating power, and the strict power and `pow` where the
/// checked one returns a value, and `None` where they would panic.
#[derive(Debug, PartialEq)]
struct Power<V> {
  overflowing: (V, bool),
  checked: Option<V>,
  wrapping: V,
  saturating: V,
  strict: Option<V>,
  pow: Option<V>,
}

/// The [`Power`] of `$x`, a reference to a value, and `$exp`, which `$value` reads, through the
/// methods that the context reaches: the inherent `const fn`s for a concrete storage, and the
/// traits in generic code.
macro_rules! power_results {
  ($x:expr, $exp:expr, $value:expr) => {{
    let (x, exp, value) = ($x, $exp, $value);
    let (power, overflow) = x.overflowing_pow(exp);
    Power {
      overflowing: (value(&power), overflow),
      checked: x.checked_pow(exp).map(|power| value(&power)),
      wrapping: value(&x.wrapping_pow(exp)),
      saturating: value(&x.saturating_pow(exp)),
      strict: x.checked_pow(exp).map(|_| value(&x.strict_pow(exp))),
      pow: x.checked_pow(exp).map(|_| value(&x.pow(exp))),
    }
  }};
}

/// The [`Power`] of `x` and `exp` through the traits.
fn power_through_traits<T, V>(x: &T, exp: u32, value: fn(&T) -> V) -> Power<V>
where
  T: OverflowingOps + CheckedOps + WrappingOps + SaturatingOps + StrictOps + IntegerMath,
{
  power_results!(x, exp, value)
}

/// What the roots and logarithms return for a value, each value read as `V`: the square root
/// where it does not panic, on a negative value; the checked logarithms to the bases 2 and 10 and
/// the logarithms where they do not panic, on a value that is not positive; and for each degree of
/// a list, the checked root and the root where it does not panic.
#[derive(Debug, PartialEq)]
struct Roots<V> {
  isqrt: Option<V>,
  logs: [Option<u32>; 4],
  roots: Vec<(Option<V>, Option<V>)>,
}

/// The [`Roots`] of `$x`, a reference to a value, at each degree of `$degrees`, which `$value`
/// reads, through the methods that the context reaches, as [`power_results!`] does. A value has a
/// square root exactly when it has a root of degree 2.
macro_rules! root_results {
  ($x:expr, $degrees:expr, $value:expr) => {{
    let (x, value) = ($x, $value);
    Roots {
      isqrt: x.checked_nth_root(2).map(|_| value(&x.isqrt())),
      logs: [
        x.checked_ilog2(),
        x.checked_ilog10(),
        x.checked_ilog2().map(|_| x.ilog2()),
        x.checked_ilog10().map(|_| x.ilog10()),
      ],
      roots: $degrees
        .iter()
        .map(|&n| {
          let checked = x.checked_nth_root(n);
          let root = checked.as_ref().map(|_| value(&x.nth_root(n)));
          (checked.map(|root| value(&root)), root)
        })
        .collect(),
    }
  }};
}

/// The [`Roots`] of `x` at each degree of `degrees` through the traits.
fn roots_through_traits<T: IntegerMath, V>(x: &T, degrees: &[u32], value: fn(&T) -> V) -> Roots<V> {
  root_results!(x, degrees, value)
}

/// The [`Roots`] that the functions return for `a`, of at most 64 bits, at each degree of
/// `degrees`, computed in `i128`.
fn expected_roots(a: i128, degrees: &[u32]) -> Roots<i128> {
  let (log2, log10) = (a.checked_ilog2(), a.checked_ilog10());
  Roots {
    isqrt: (a >= 0).then(|| a.isqrt()),
    logs: [log2, log10, log2, log10],
    roots: degrees
      .iter()
      .map(|&n| {
        let root = (n > 0 && (a >= 0 || n % 2 == 1)).then(|| bigint::nth_root(a, n));
        (root, root)
      })
      .collect(),
  }
}

/// The [`Roots`] that the functions return for `a` at each degree of `degrees`, computed with
/// `num-bigint`.
fn expected_big_roots(a: &BigInt, degrees: &[u32]) -> Roots<BigInt> {
  let negative = *a < BigInt::ZERO;
  let log2 = bigint::ilog(a, &BigInt::from(2u8));
  let log10 = bigint::ilog(a, &BigInt::from(10u8));
  Roots {
    isqrt: (!negative).then(|| a.sqrt()),
    logs: [log2, log10, log2, log10],
    roots: degrees
      .iter()
      .map(|&n| {
        let root = (n > 0 && (!negative || n % 2 == 1)).then(|| a.nth_root(n));
        (root.clone(), root)
      })
      .collect(),
  }
}

/// Checks the functions of `UnsignedMath` and `SignedMath` that take one value, for the sign token
/// `$sign`, on `$x`, the value `$a` of `$bits` bits, which `$value` reads, through the inherent
/// `const fn`s and the traits: the next power of two in its forms for `Uint`, and the checked
/// square root for `Int`.
macro_rules! sign_powers {
  (unsigned, $x:expr, $a:expr, $bits:expr, $value:expr) => {{
    let (x, a, value) = ($x, $a, $value);
    let max = range($bits, false).1;
    let power = a.unsigned_abs().next_power_of_two() as i128;
    let expected = (power <= max).then_some(power);
    let checked = both!(UnsignedMath::checked_next_power_of_two(x));
    assert_eq!(checked.map(|power| value(&power)), expected, "{a}");
    let wrapping = both!(UnsignedMath::wrapping_next_power_of_two(x));
    assert_eq!(value(&wrapping), expected.unwrap_or(0), "{a}");
    if let Some(power) = expected {
      assert_eq!(
        value(&both!(UnsignedMath::next_power_of_two(x))),
        power,
        "{a}"
      );
    }
  }};
  (signed, $x:expr, $a:expr, $bits:expr, $value:expr) => {{
    let (a, value) = ($a, $value);
    let root = both!(SignedMath::checked_isqrt($x));
    assert_eq!(
      root.map(|root| value(&root)),
      (a >= 0).then(|| a.isqrt()),
      "{a}"
    );
  }};
}

/// The [`Power`] that the methods return for `a`, a value of `bits` bits, signed if `signed`, and
/// `exp`, computed in `i128`. A power that saturates passes `MIN` exactly when it is negative, an
/// odd power of a negative value.
fn expected_power(a: i128, exp: u32, bits: u32, signed: bool) -> Power<i128> {
  let (min, max) = range(bits, signed);
  let (power, overflow) = bigint::pow_wrapped(a, exp, bits, min, max);
  let exact = (!overflow).then_some(power);
  let saturated = if a < 0 && exp % 2 == 1 { min } else { max };
  Power {
    overflowing: (power, overflow),
    checked: exact,
    wrapping: power,
    saturating: exact.unwrap_or(saturated),
    strict: exact,
    pow: exact,
  }
}

/// The [`Power`] that the methods return for `a`, a value of `bits` bits, signed if `signed`, and
/// `exp`, computed with `num-bigint`.
fn expected_big_power(a: &BigInt, exp: u32, bits: u32, signed: bool) -> Power<BigInt> {
  let (min, max) = (bigint::min(bits, signed), bigint::max(bits, signed));
  let (power, overflow) = bigint::pow_bits(a, exp, bits, &min, &max);
  let exact = (!overflow).then(|| power.clone());
  let saturated = if *a < BigInt::ZERO && exp % 2 == 1 {
    min
  } else {
    max
  };
  Power {
    overflowing: (power.clone(), overflow),
    checked: exact.clone(),
    wrapping: power,
    saturating: exact.clone().unwrap_or(saturated),
    strict: exact.clone(),
    pow: exact,
  }
}

/// The exponents that a sweep of `bits` bits checks: under Miri 0, 1, 2 and `bits`; otherwise, to
/// 8 bits, every exponent to `2 * bits + 2`, and from 9 bits 0 to 3, `bits - 1` to `bits + 1`; and
/// beyond Miri the two largest exponents, of each parity.
fn exponents(bits: u32) -> Vec<u32> {
  let mut exps: Vec<u32> = match tier() {
    Tier::Miri => vec![0, 1, 2, bits],
    _ if bits <= 8 => (0..=2 * bits + 2).collect(),
    _ => vec![0, 1, 2, 3, bits - 1, bits, bits + 1],
  };
  if !tier().is_miri() {
    exps.extend([u32::MAX - 1, u32::MAX]);
  }
  exps.sort_unstable();
  exps.dedup();
  exps
}

/// The degrees of the roots that a sweep of `bits` bits checks: under Miri 0, 1, 2 and `bits`;
/// otherwise, to 8 bits, every degree to `bits + 2`, and from 9 bits 0 to 3 and `bits - 1` to
/// `bits + 1`; and beyond Miri the largest degree.
fn degrees(bits: u32) -> Vec<u32> {
  let mut degrees: Vec<u32> = match tier() {
    Tier::Miri => vec![0, 1, 2, bits],
    _ if bits <= 8 => (0..=bits + 2).collect(),
    _ => vec![0, 1, 2, 3, bits - 1, bits, bits + 1],
  };
  if !tier().is_miri() {
    degrees.push(u32::MAX);
  }
  degrees.sort_unstable();
  degrees.dedup();
  degrees
}

/// Checks the powers of `$ty`, of the sign `$sign` and of `$bits` bits, whose values `$from`
/// creates from an `i128` and `$value` reads back, at each value of `$values`: at each exponent of
/// [`exponents`], the inherent `const fn`s and the traits return the [`expected_power`]; at each
/// degree of [`degrees`], the [`expected_roots`]; and the functions of the sign.
macro_rules! sweep_powers {
  ($sign:ident; $ty:ty, $bits:expr, $values:expr, $from:expr, $value:expr, $magnitude:expr) => {{
    let (bits, signed) = ($bits, is_signed!($sign));
    let from: fn(i128) -> $ty = $from;
    let value: fn(&$ty) -> i128 = $value;
    let (exps, degrees) = (exponents(bits), degrees(bits));
    for a in $values {
      let x = from(a);
      for &exp in &exps {
        let expected = expected_power(a, exp, bits, signed);
        assert_eq!(
          power_results!(&x, exp, value),
          expected,
          "{a}^{exp} in {bits} bits"
        );
        assert_eq!(
          power_through_traits(&x, exp, value),
          expected,
          "{a}^{exp} in {bits} bits, through the traits"
        );
      }
      let roots = expected_roots(a, &degrees);
      assert_eq!(
        root_results!(&x, degrees, value),
        roots,
        "{a} in {bits} bits"
      );
      assert_eq!(
        roots_through_traits(&x, &degrees, value),
        roots,
        "{a} in {bits} bits, through the traits"
      );
      sign_powers!($sign, &x, a, bits, value);
    }
  }};
}

/// Checks the powers at the width `B`, `bits` bits, at most 8, in `u8` and `i8`, `[u8; 1]`,
/// `[u8; 2]` and `[u16; 1]` at the values of [`small_values`] for a storage that every tier checks,
/// and in `u16` and `i16`, `u128` and `i128`, `[u64; 1]` and `[u128; 1]` at those for another.
fn powers_of_small_widths<B>(bits: u32)
where
  B: FitsBits<u8> + FitsBits<i8> + FitsBits<u16> + FitsBits<i16> + FitsBits<u128> + FitsBits<i128>,
  B: FitsBits<[u8; 1]> + FitsBits<[u8; 2]> + FitsBits<[u16; 1]>,
  B: FitsBits<[u64; 1]> + FitsBits<[u128; 1]>,
{
  let primary = |bits, signed| small_values(bits, signed, true);
  let secondary = |bits, signed| small_values(bits, signed, false);
  sweep_storage!(sweep_powers, primary, native B, bits, u8, i8);
  sweep_storage!(sweep_powers, primary, array B, bits, [u8; 1]);
  sweep_storage!(sweep_powers, primary, array B, bits, [u8; 2]);
  sweep_storage!(sweep_powers, primary, array B, bits, [u16; 1]);
  sweep_storage!(sweep_powers, secondary, native B, bits, u16, i16);
  sweep_storage!(sweep_powers, secondary, native B, bits, u128, i128);
  sweep_storage!(sweep_powers, secondary, array B, bits, [u64; 1]);
  sweep_storage!(sweep_powers, secondary, array B, bits, [u128; 1]);
}

/// Checks the powers at the width `B`, `bits` bits, from 9 to 16, in `u16` and `i16`, `[u8; 2]`
/// and `[u16; 1]`, at the values of [`large_values`].
fn powers_of_large_widths<B>(bits: u32)
where
  B: FitsBits<u16> + FitsBits<i16> + FitsBits<[u8; 2]> + FitsBits<[u16; 1]>,
{
  sweep_storage!(sweep_powers, large_values, native B, bits, u16, i16);
  sweep_storage!(sweep_powers, large_values, array B, bits, [u8; 2]);
  sweep_storage!(sweep_powers, large_values, array B, bits, [u16; 1]);
}

/// Every value of each width up to 8 bits, or the values that the tier takes, in native integers
/// and in arrays of one and two limbs, to every exponent that [`exponents`] gives, through the
/// inherent `const fn`s and the traits, against `i128`: `0^0`, which overflows zero bits, and
/// `(-1)^0` and `(-1)^2`, which overflow one signed bit while `(-1)^3` does not.
#[test]
fn families_raise_values_of_up_to_8_bits() {
  powers_of_small_widths::<U0>(0);
  powers_of_small_widths::<U1>(1);
  powers_of_small_widths::<U2>(2);
  powers_of_small_widths::<U3>(3);
  powers_of_small_widths::<U4>(4);
  powers_of_small_widths::<U5>(5);
  powers_of_small_widths::<U6>(6);
  powers_of_small_widths::<U7>(7);
  powers_of_small_widths::<U8>(8);
}

/// The values that the tier takes of each width from 9 to 16 bits, with the ends, in `u16` and
/// `i16`, in two limbs of 8 bits and in one of 16 bits. Miri checks 9, 12 and 16 bits.
#[test]
fn families_raise_values_of_9_to_16_bits() {
  powers_of_large_widths::<U9>(9);
  powers_of_large_widths::<U12>(12);
  powers_of_large_widths::<U16>(16);
  if !tier().is_miri() {
    powers_of_large_widths::<U10>(10);
    powers_of_large_widths::<U11>(11);
    powers_of_large_widths::<U13>(13);
    powers_of_large_widths::<U14>(14);
    powers_of_large_widths::<U15>(15);
  }
}

/// The exponents at which the powers of `bits` bits are checked against `num-bigint`: under Miri,
/// where the powers of the largest exponents are slow in many limbs, 0, 1, 2 and `bits`.
fn big_exponents(bits: u32) -> Vec<u32> {
  if tier().is_miri() {
    return vec![0, 1, 2, bits];
  }
  vec![
    0,
    1,
    2,
    3,
    bits / 2,
    bits - 1,
    bits,
    bits + 1,
    u32::MAX - 1,
    u32::MAX,
  ]
}

/// The degrees at which the roots of `bits` bits are checked against `num-bigint`: under Miri 0,
/// 1, 2 and `bits`.
fn big_degrees(bits: u32) -> Vec<u32> {
  if tier().is_miri() {
    return vec![0, 1, 2, bits];
  }
  vec![0, 1, 2, 3, 7, bits - 1, bits, bits + 1, u32::MAX]
}

/// Checks the powers, the roots and the logarithms of `T`, of `bits` bits, through the traits at
/// every value of `samples`, every exponent of [`big_exponents`] and every degree of
/// [`big_degrees`], against `num-bigint`.
fn check_powers_against_big_integers<T>(bits: u32, samples: &[BigInt])
where
  T: Big + OverflowingOps + CheckedOps + WrappingOps + SaturatingOps + StrictOps + IntegerMath,
{
  let degrees = big_degrees(bits);
  for a in samples {
    let x = T::from_big(a);
    for exp in big_exponents(bits) {
      assert_eq!(
        power_through_traits(&x, exp, |value| value.to_big()),
        expected_big_power(a, exp, bits, T::SIGNED),
        "{a}^{exp} in {bits} bits"
      );
    }
    assert_eq!(
      roots_through_traits(&x, &degrees, |value| value.to_big()),
      expected_big_roots(a, &degrees),
      "{a} in {bits} bits"
    );
  }
}

/// Checks the next power of two of `T`, unsigned, of `bits` bits, in its forms, through the trait
/// at every value of `samples`, against `num-bigint`.
fn check_next_powers_against_big_integers<T: Big + UnsignedMath>(bits: u32, samples: &[BigInt]) {
  let max = bigint::max(bits, false);
  for a in samples {
    let x = T::from_big(a);
    let power = bigint::next_power_of_two(a);
    let expected = (power <= max).then_some(power);
    assert_eq!(
      x.checked_next_power_of_two().map(T::to_big),
      expected,
      "{a}"
    );
    let wrapped = expected.clone().unwrap_or(BigInt::ZERO);
    assert_eq!(x.wrapping_next_power_of_two().to_big(), wrapped, "{a}");
    if let Some(power) = expected {
      assert_eq!(x.next_power_of_two().to_big(), power, "{a}");
    }
  }
}

/// Checks the checked square root of `T`, signed, through the trait at every value of `samples`,
/// against `num-bigint`.
fn check_square_roots_against_big_integers<T: Big + SignedMath>(samples: &[BigInt]) {
  for a in samples {
    let expected = (*a >= BigInt::ZERO).then(|| a.sqrt());
    assert_eq!(
      T::from_big(a).checked_isqrt().map(T::to_big),
      expected,
      "{a}"
    );
  }
}

/// Checks that the inherent `const fn`s of the powers, the roots and the logarithms of `$ty`, of
/// `$bits` bits, agree with the traits at every value of `$samples`, every exponent of
/// [`big_exponents`] and every degree of [`big_degrees`].
macro_rules! check_power_paths {
  ($ty:ty, $bits:expr, $samples:expr) => {{
    let value: fn(&$ty) -> BigInt = |value| value.to_big();
    let degrees = big_degrees($bits);
    for a in $samples {
      let x = <$ty>::from_big(a);
      for exp in big_exponents($bits) {
        assert_eq!(
          power_results!(&x, exp, value),
          power_through_traits(&x, exp, value),
          "{a}^{exp}"
        );
      }
      assert_eq!(
        root_results!(&x, degrees, value),
        roots_through_traits(&x, &degrees, value),
        "{a}"
      );
    }
  }};
}

/// Checks the powers, the roots and the logarithms of `Uint` and `Int` of `$bits` bits, the width
/// `$width`, in each pair of storages written `unsigned, signed;`, against `num-bigint`, with
/// `$count` random samples.
macro_rules! powers_agree {
  ($width:ty, $bits:literal, $count:literal; $($uint:ty, $int:ty);+ $(;)?) => {{
    let unsigned = big_samples($bits, false, $count);
    let signed = big_samples($bits, true, $count);
    $(
      check_powers_against_big_integers::<Uint<$width, $uint>>($bits, &unsigned);
      check_next_powers_against_big_integers::<Uint<$width, $uint>>($bits, &unsigned);
      check_power_paths!(Uint<$width, $uint>, $bits, &unsigned);
      check_powers_against_big_integers::<Int<$width, $int>>($bits, &signed);
      check_square_roots_against_big_integers::<Int<$width, $int>>(&signed);
      check_power_paths!(Int<$width, $int>, $bits, &signed);
    )+
  }};
}

/// The edge and random values of 24 to 4096 bits in storages of each kind that hold them: the
/// powers, the roots and the logarithms agree with `num-bigint` through the traits, and the
/// inherent `const fn`s agree with the traits. An unoptimized build leaves out the widths above 200
/// bits, and Miri also 64, 128 and 200 bits and every exponent and degree but 0, 1, 2 and the
/// width.
#[test]
#[cfg_attr(
  all(miri, target_pointer_width = "32"),
  ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
)]
fn storages_agree_on_the_powers() {
  powers_agree!(U24, 24, 4; u32, i32; [u8; 3], [u8; 3]; [u16; 2], [u16; 2]);
  powers_agree!(U65, 65, 4; u128, i128; [u8; 9], [u8; 9]; [u64; 2], [u64; 2]);
  powers_agree!(U129, 129, 4; [u8; 17], [u8; 17]; [u64; 3], [u64; 3]; [u128; 2], [u128; 2]);
  if !tier().is_miri() {
    powers_agree!(U64, 64, 4; u64, i64; [u16; 4], [u16; 4]; [u64; 1], [u64; 1]);
    powers_agree!(U128, 128, 4; u128, i128; [u32; 4], [u32; 4]; [u128; 1], [u128; 1]);
    powers_agree!(U200, 200, 4; [u32; 7], [u32; 7]; [u64; 4], [u64; 4]);
  }
  if tier().is_full() {
    powers_agree!(U1024, 1024, 2; [u32; 32], [u32; 32]; [u64; 16], [u64; 16]);
    // Products of 512 limbs of 8 bits would take most of the time of the test.
    powers_agree!(U4096, 4096, 2; [u64; 64], [u64; 64]; [u128; 32], [u128; 32]);
  }
}

/// The strict power panics on overflow with the message of the native `strict_pow` in every build,
/// through the inherent `const fn`s and the traits, and reports the caller: at a partial and at the
/// full width of a native integer and of an array, at zero bits, where `0^0` overflows, and at one
/// signed bit, where `(-1)^0` and `(-1)^2` overflow.
#[test]
fn strict_power_panics_report_the_caller() {
  let message = panic_of(|| black_box(u8::MAX).strict_pow(2))
    .expect("a panic")
    .0;
  macro_rules! power_panics {
    ($x:expr, $exp:expr) => {{
      let (x, exp) = ($x, $exp);
      panics_here!(x.strict_pow(exp), message);
      panics_here!(StrictOps::strict_pow(&x, exp), message);
    }};
  }
  power_panics!(u7::MAX, 2);
  power_panics!(Uint::<U8, u8>::new(16), 2);
  power_panics!(Uint::<U200>::MAX, 2);
  power_panics!(u256::MAX, u32::MAX);
  power_panics!(i7::MIN, 3);
  power_panics!(Int::<U8, i8>::new(-2), 8);
  power_panics!(Int::<U200>::MIN, 2);
  power_panics!(i256::MIN, 3);
  power_panics!(Uint::<U0, u8>::ZERO, 0);
  power_panics!(Int::<U0, [u8; 0]>::ZERO, 0);
  power_panics!(Int::<U1, i8>::MIN, 0);
  power_panics!(Int::<U1, [u8; 1]>::MIN, 2);
  assert_eq!(Int::<U1, i8>::MIN.strict_pow(3), Int::<U1, i8>::MIN);
}

/// The powers of the families in constant contexts, in a native integer and in an array, at zero
/// bits and at one signed bit.
#[test]
fn powers_in_constant_contexts() {
  type Bytes = Int<U24, [u8; 3]>;
  type Word = Uint<U7, [u64; 1]>;
  const OVERFLOWING: [(i7, bool); 3] = [
    i7::new(-2).overflowing_pow(6),
    i7::new(-2).overflowing_pow(7),
    i7::new(3).overflowing_pow(4),
  ];
  const CHECKED: [Option<Bytes>; 3] = [
    Bytes::MIN.checked_pow(1),
    Bytes::ONE.wrapping_neg().checked_pow(u32::MAX),
    Bytes::MIN.checked_pow(2),
  ];
  const WRAPPING: u256 = u256::MAX.wrapping_pow(3);
  const SATURATING: [i7; 3] = [
    i7::new(-2).saturating_pow(7),
    i7::new(-2).saturating_pow(8),
    i7::new(2).saturating_pow(5),
  ];
  const STRICT: [Word; 2] = [
    Word::ONE.wrapping_add(&Word::ONE).strict_pow(6),
    Word::MAX.strict_pow(0),
  ];
  const ZERO_BITS: [(Uint<U0, u8>, bool); 2] = [
    Uint::<U0, u8>::ZERO.overflowing_pow(0),
    Uint::<U0, u8>::ZERO.overflowing_pow(1),
  ];
  const ONE_BIT: [(Int<U1, i8>, bool); 3] = [
    Int::<U1, i8>::MIN.overflowing_pow(0),
    Int::<U1, i8>::MIN.overflowing_pow(2),
    Int::<U1, i8>::MIN.overflowing_pow(3),
  ];
  const ONE_BIT_SATURATING: Int<U1, i8> = Int::<U1, i8>::MIN.saturating_pow(0);

  assert_eq!(
    OVERFLOWING,
    [(i7::new(-64), true), (i7::ZERO, true), (i7::new(-47), true)]
  );
  assert_eq!(
    CHECKED,
    [Some(Bytes::MIN), Some(Bytes::ONE.wrapping_neg()), None]
  );
  assert_eq!(WRAPPING, u256::MAX);
  assert_eq!(SATURATING, [i7::MIN, i7::MAX, i7::new(32)]);
  assert_eq!(
    STRICT.map(|value| value.to_string()),
    ["64".to_string(), "1".to_string()]
  );
  assert_eq!(
    ZERO_BITS,
    [(Uint::<U0, u8>::ZERO, true), (Uint::<U0, u8>::ZERO, false)]
  );
  let minus_one = Int::<U1, i8>::MIN;
  assert_eq!(
    ONE_BIT,
    [(minus_one, true), (minus_one, true), (minus_one, false)]
  );
  assert_eq!(ONE_BIT_SATURATING, Int::<U1, i8>::MAX);
}

/// The roots and the logarithms panic where the native methods do, with their messages, in every
/// build, through the inherent `const fn`s and the traits, and report the caller: the square root
/// and a root of even degree of a negative value, a root of degree zero, checked before the sign,
/// a logarithm of a value that is not positive, and a logarithm to a base below 2, checked before
/// the value. At a partial and at the full width of a native integer and of an array, at zero bits
/// and at one signed bit.
#[test]
fn root_and_logarithm_panics_report_the_caller() {
  let sqrt = panic_of(|| black_box(-1i8).isqrt()).expect("a panic").0;
  let log = panic_of(|| black_box(0u8).ilog2()).expect("a panic").0;
  let base = panic_of(|| black_box(5u8).ilog(black_box(1)))
    .expect("a panic")
    .0;
  // The native integers have no root of other degrees, so the messages are those of this crate.
  let degree = "degree of integer root must be at least 1".to_string();
  let even = "argument of integer root of even degree cannot be negative".to_string();
  // `$zero`, which has no logarithm, `$x`, at least 2, and `$one`, which is no base.
  macro_rules! logarithm_panics {
    ($zero:expr, $one:expr, $x:expr) => {{
      let (zero, one, x) = ($zero, $one, $x);
      panics_here!(zero.ilog2(), log);
      panics_here!(IntegerMath::ilog2(&zero), log);
      panics_here!(zero.ilog10(), log);
      panics_here!(IntegerMath::ilog10(&zero), log);
      panics_here!(zero.ilog(&x), log);
      panics_here!(IntegerMath::ilog(&zero, &x), log);
      panics_here!(x.ilog(&one), base);
      panics_here!(IntegerMath::ilog(&x, &one), base);
      panics_here!(zero.ilog(&one), base);
      panics_here!(x.nth_root(0), degree);
      panics_here!(IntegerMath::nth_root(&x, 0), degree);
    }};
  }
  // `$negative`, which has no square root and no root of even degree.
  macro_rules! negative_panics {
    ($negative:expr) => {{
      let negative = $negative;
      panics_here!(negative.isqrt(), sqrt);
      panics_here!(IntegerMath::isqrt(&negative), sqrt);
      panics_here!(negative.nth_root(2), even);
      panics_here!(IntegerMath::nth_root(&negative, 4), even);
      panics_here!(negative.nth_root(0), degree);
      panics_here!(negative.ilog2(), log);
      panics_here!(negative.ilog10(), log);
    }};
  }
  logarithm_panics!(u7::ZERO, u7::ONE, u7::MAX);
  logarithm_panics!(
    Uint::<U8, u8>::ZERO,
    Uint::<U8, u8>::ONE,
    Uint::<U8, u8>::MAX
  );
  logarithm_panics!(Uint::<U200>::ZERO, Uint::<U200>::ONE, Uint::<U200>::MAX);
  logarithm_panics!(u256::ZERO, u256::ONE, u256::MAX);
  logarithm_panics!(i7::ZERO, i7::ONE, i7::MAX);
  logarithm_panics!(i256::ZERO, i256::ONE, i256::MAX);
  logarithm_panics!(i7::MIN, -i7::ONE, i7::new(2));
  negative_panics!(i7::MIN);
  negative_panics!(Int::<U8, i8>::new(-1));
  negative_panics!(Int::<U200>::MIN);
  negative_panics!(i256::MIN);
  // One signed bit holds only -1 and 0, neither of which is a base, and zero bits only zero.
  negative_panics!(Int::<U1, [u8; 1]>::MIN);
  let one_bit = Int::<U1, i8>::MIN;
  panics_here!(one_bit.ilog(&Int::<U1, i8>::ZERO), base);
  panics_here!(Int::<U1, i8>::ZERO.ilog(&one_bit), base);
  type Zero = Uint<typenum::U0, u8>;
  panics_here!(Zero::ZERO.ilog2(), log);
  panics_here!(Zero::ZERO.ilog(&Zero::ZERO), base);
  panics_here!(Zero::ZERO.nth_root(0), degree);
  assert_eq!(Zero::ZERO.isqrt(), Zero::ZERO);
  assert_eq!(Int::<U1, i8>::MIN.nth_root(3), one_bit);
}

/// `pow` and `next_power_of_two` overflow as the native methods do: with overflow checks they
/// panic with the messages of the native exponentiation and addition, `pow` reporting a line of
/// `core`, as the native `pow` does, and `next_power_of_two` the caller; without them they return
/// the wrapped value. At a partial and at the full width of a native integer and of an array, and
/// at zero bits, where `0^0` and the power of two of zero, both one, overflow, and at one signed
/// bit, where `(-1)^0` overflows to -1.
#[test]
fn pow_and_next_power_of_two_follow_overflow_checks() {
  // The messages of the native operations that overflow, which only a build with checks has.
  let pow = panic_of(|| black_box(u8::MAX).pow(2)).map_or_else(String::new, |panic| panic.0);
  let add = panic_of(|| black_box(u8::MAX) + 1).map_or_else(String::new, |panic| panic.0);
  type U200 = Uint<typenum::U200>;
  type Zero = Uint<typenum::U0, u8>;
  overflows_here!(u7::MAX.pow(2), pow, u7::ONE, in core);
  overflows_here!(IntegerMath::pow(&u7::MAX, 2), pow, u7::ONE, in core);
  overflows_here!(i7::MIN.pow(3), pow, i7::ZERO, in core);
  overflows_here!(U200::MAX.pow(3), pow, U200::MAX, in core);
  overflows_here!(i256::MIN.pow(2), pow, i256::ZERO, in core);
  overflows_here!(Zero::ZERO.pow(0), pow, Zero::ZERO, in core);
  let one_bit = Int::<U1, i8>::MIN;
  overflows_here!(one_bit.pow(0), pow, one_bit, in core);
  overflows_here!(u7::new(65).next_power_of_two(), add, u7::ZERO);
  overflows_here!(UnsignedMath::next_power_of_two(&u7::new(65)), add, u7::ZERO);
  overflows_here!(U200::MAX.next_power_of_two(), add, U200::ZERO);
  overflows_here!(u256::MAX.next_power_of_two(), add, u256::ZERO);
  overflows_here!(Zero::ZERO.next_power_of_two(), add, Zero::ZERO);
}

/// `pow`, the roots, the logarithms and the next power of two in constant contexts, in a native
/// integer and in an array, at zero bits and at one signed bit.
#[test]
fn math_powers_in_constant_contexts() {
  type Bytes = Int<U24, [u8; 3]>;
  type Word = Uint<U7, [u64; 1]>;
  type Zero = Uint<typenum::U0, u8>;
  type I1 = Int<U1, i8>;
  const POWERS: (u7, Bytes) = (u7::new(2).pow(6), Bytes::ONE.wrapping_neg().pow(u32::MAX));
  const ROOTS: [Word; 3] = [
    Word::MAX.isqrt(),
    Word::MAX.nth_root(3),
    Word::MAX.nth_root(u32::MAX),
  ];
  const SIGNED_ROOTS: [Option<i7>; 3] = [
    i7::new(-64).checked_nth_root(3),
    i7::new(-64).checked_nth_root(2),
    i7::new(63).checked_isqrt(),
  ];
  const DEEP: i256 = i256::MIN.nth_root(255);
  const LOGS: [Option<u32>; 3] = [
    Bytes::MAX.checked_ilog2(),
    Bytes::MAX.checked_ilog10(),
    Bytes::MIN.checked_ilog2(),
  ];
  const ILOGS: [u32; 3] = [
    u7::MAX.ilog2(),
    u7::MAX.ilog10(),
    Word::MAX.ilog(&Word::ONE.wrapping_add(&Word::ONE)),
  ];
  const NEXT: [Option<u7>; 2] = [
    u7::new(33).checked_next_power_of_two(),
    u7::new(65).checked_next_power_of_two(),
  ];
  const WRAPPED: [u7; 2] = [
    u7::new(65).wrapping_next_power_of_two(),
    u7::new(33).next_power_of_two(),
  ];
  const ARRAY_NEXT: Word = Word::ONE
    .wrapping_add(&Word::ONE)
    .wrapping_add(&Word::ONE)
    .next_power_of_two();
  const ZERO_BITS: (Zero, Option<Zero>, Zero, Option<u32>) = (
    Zero::ZERO.isqrt(),
    Zero::ZERO.checked_next_power_of_two(),
    Zero::ZERO.nth_root(1),
    Zero::ZERO.checked_ilog2(),
  );
  const ONE_BIT: (Option<I1>, Option<I1>, I1) = (
    I1::MIN.checked_isqrt(),
    I1::MIN.checked_nth_root(3),
    I1::ZERO.isqrt(),
  );

  assert_eq!(POWERS, (u7::new(64), Bytes::ONE.wrapping_neg()));
  assert_eq!(
    ROOTS.map(|root| root.to_string()),
    ["11", "5", "1"].map(String::from)
  );
  assert_eq!(SIGNED_ROOTS, [Some(i7::new(-4)), None, Some(i7::new(7))]);
  assert_eq!(DEEP, -(i256::ONE + i256::ONE));
  assert_eq!(LOGS, [Some(22), Some(6), None]);
  assert_eq!(ILOGS, [6, 2, 6]);
  assert_eq!(NEXT, [Some(u7::new(64)), None]);
  assert_eq!(WRAPPED, [u7::ZERO, u7::new(64)]);
  assert_eq!(ARRAY_NEXT.to_string(), "4");
  assert_eq!(ZERO_BITS, (Zero::ZERO, None, Zero::ZERO, None));
  assert_eq!(ONE_BIT, (None, Some(I1::MIN), I1::ZERO));
}
