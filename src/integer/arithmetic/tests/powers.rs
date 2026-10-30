//! The powers of `Uint` and `Int` in the arithmetic families: the overflowing, checked, wrapping,
//! saturating and strict power, through the inherent `const fn`s and the traits, against `i128` at
//! every width up to 16 bits and against `num-bigint` at wider ones; their panics; and their
//! constants.

use typenum::{U3, U4, U5, U6, U10, U11, U12, U13, U14, U15};

use super::*;

/// What the power methods of the families return for a value and an exponent, each value read as
/// `V`: the overflowing, checked, wrapping and saturating power, and the strict power where the
/// checked one returns a value, and `None` where it would panic.
#[derive(Debug, PartialEq)]
struct Power<V> {
  overflowing: (V, bool),
  checked: Option<V>,
  wrapping: V,
  saturating: V,
  strict: Option<V>,
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
    }
  }};
}

/// The [`Power`] of `x` and `exp` through the traits.
fn power_through_traits<T, V>(x: &T, exp: u32, value: fn(&T) -> V) -> Power<V>
where
  T: OverflowingOps + CheckedOps + WrappingOps + SaturatingOps + StrictOps,
{
  power_results!(x, exp, value)
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
    strict: exact,
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

/// Checks the power methods of `$ty`, of `$bits` bits, signed if `$signed`, whose values `$from`
/// creates from an `i128` and `$value` reads back, at each value of `$values` and each exponent of
/// [`exponents`]: the inherent `const fn`s and the traits return the [`expected_power`].
macro_rules! sweep_powers {
  ($ty:ty, $bits:expr, $signed:expr, $values:expr, $from:expr, $value:expr) => {{
    let (bits, signed) = ($bits, $signed);
    let from: fn(i128) -> $ty = $from;
    let value: fn(&$ty) -> i128 = $value;
    let exps = exponents(bits);
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
/// `i16`, in two limbs of 8 bits and in one of 16 bits.
#[test]
fn families_raise_values_of_9_to_16_bits() {
  powers_of_large_widths::<U9>(9);
  powers_of_large_widths::<U10>(10);
  powers_of_large_widths::<U11>(11);
  powers_of_large_widths::<U12>(12);
  powers_of_large_widths::<U13>(13);
  powers_of_large_widths::<U14>(14);
  powers_of_large_widths::<U15>(15);
  powers_of_large_widths::<U16>(16);
}

/// The exponents at which the powers of `bits` bits are checked against `num-bigint`.
fn big_exponents(bits: u32) -> [u32; 10] {
  [
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

/// Checks the power methods of `T`, of `bits` bits, through the traits at every value of `samples`
/// and every exponent of [`big_exponents`], against `num-bigint`.
fn check_powers_against_big_integers<T>(bits: u32, samples: &[BigInt])
where
  T: Big + OverflowingOps + CheckedOps + WrappingOps + SaturatingOps + StrictOps,
{
  for a in samples {
    let x = T::from_big(a);
    for exp in big_exponents(bits) {
      assert_eq!(
        power_through_traits(&x, exp, |value| value.to_big()),
        expected_big_power(a, exp, bits, T::SIGNED),
        "{a}^{exp} in {bits} bits"
      );
    }
  }
}

/// Checks that the inherent `const fn`s of the powers of `$ty`, of `$bits` bits, agree with the
/// traits at every value of `$samples` and every exponent of [`big_exponents`].
macro_rules! check_power_paths {
  ($ty:ty, $bits:expr, $samples:expr) => {{
    let value: fn(&$ty) -> BigInt = |value| value.to_big();
    for a in $samples {
      let x = <$ty>::from_big(a);
      for exp in big_exponents($bits) {
        assert_eq!(
          power_results!(&x, exp, value),
          power_through_traits(&x, exp, value),
          "{a}^{exp}"
        );
      }
    }
  }};
}

/// Checks the powers of `Uint` and `Int` of `$bits` bits, the width `$width`, in each pair of
/// storages written `unsigned, signed;`, against `num-bigint`, with `$count` random samples.
macro_rules! powers_agree {
  ($width:ty, $bits:literal, $count:literal; $($uint:ty, $int:ty);+ $(;)?) => {{
    let unsigned = big_samples($bits, false, $count);
    let signed = big_samples($bits, true, $count);
    $(
      check_powers_against_big_integers::<Uint<$width, $uint>>($bits, &unsigned);
      check_power_paths!(Uint<$width, $uint>, $bits, &unsigned);
      check_powers_against_big_integers::<Int<$width, $int>>($bits, &signed);
      check_power_paths!(Int<$width, $int>, $bits, &signed);
    )+
  }};
}

/// The edge and random values of 17 to 4096 bits in every storage that holds them: the powers
/// agree with `num-bigint` through the traits, and the inherent `const fn`s agree with the traits.
/// Under Miri, the widths above 256 bits are left out.
#[test]
#[cfg_attr(
  all(miri, target_pointer_width = "32"),
  ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
)]
fn storages_agree_on_the_powers() {
  powers_agree!(U24, 24, 8; u32, i32; [u8; 3], [u8; 3]; [u16; 2], [u16; 2]);
  powers_agree!(U64, 64, 8; u64, i64; [u16; 4], [u16; 4]; [u64; 1], [u64; 1]);
  powers_agree!(U65, 65, 8; u128, i128; [u8; 9], [u8; 9]; [u64; 2], [u64; 2]);
  powers_agree!(U128, 128, 8; u128, i128; [u32; 4], [u32; 4]; [u128; 1], [u128; 1]);
  powers_agree!(U129, 129, 8; [u8; 17], [u8; 17]; [u64; 3], [u64; 3]; [u128; 2], [u128; 2]);
  powers_agree!(U256, 256, 8; [u16; 16], [u16; 16]; [u64; 4], [u64; 4]; [u128; 2], [u128; 2]);
  if !cfg!(miri) {
    powers_agree!(U1024, 1024, 4; [u32; 32], [u32; 32]; [u64; 16], [u64; 16]);
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
