// The modules generated here shadow the primitive type names in this module, so primitive types
// are named only inside them.

macro_rules! core_tests {
  ($($sign:ident $int:ident),+ $(,)?) => {$(
    mod $int {
      use num_bigint::BigInt;
      use std::{string::ToString, vec::Vec};

      use crate::{
        bigint,
        int::{$int as native, DIGIT_BUF_LEN},
      };

      /// Whether `10^exp` is representable.
      fn representable(exp: u32) -> bool {
        10u128.checked_pow(exp).is_some_and(|power| power <= <$int>::MAX as u128)
      }

      /// The largest `d` such that `10^d` is representable.
      fn max_exp() -> u32 {
        let mut exp = 0;
        while representable(exp + 1) {
          exp += 1;
        }
        exp
      }

      /// Values around zero, the decimal limits and the native limits.
      fn samples() -> Vec<$int> {
        core_tests!(@samples $sign $int)
      }

      #[test]
      fn pow10_and_max_magnitude() {
        assert!(!representable(max_exp() + 1));
        for exp in 0..=max_exp() {
          assert_eq!(native::pow10(exp) as u128, 10u128.pow(exp));
          assert_eq!(native::max_magnitude(exp) as u128, 10u128.pow(exp) - 1);
        }
      }

      #[test]
      fn predicates_and_comparison() {
        for a in samples() {
          assert_eq!(native::is_zero(a), a == 0);
          assert_eq!(native::is_positive(a), a > 0);
          core_tests!(@is_negative $sign a);
          for b in samples() {
            assert_eq!(native::le(a, b), a <= b);
          }
        }
      }

      #[test]
      fn checked_arithmetic() {
        for a in samples() {
          for b in samples() {
            assert_eq!(native::checked_add(a, b), a.checked_add(b));
            assert_eq!(native::checked_sub(a, b), a.checked_sub(b));
          }
        }
      }

      #[test]
      fn multiplication() {
        for a in samples() {
          for b in samples() {
            if let Some(product) = a.checked_mul(b) {
              assert_eq!(native::mul(a, b), product);
            }
          }
        }
      }

      #[test]
      fn powers_of_ten() {
        for a in samples() {
          for exp in 0..=max_exp() {
            let power = native::pow10(exp);
            assert_eq!(native::checked_mul_pow10(a, exp), a.checked_mul(power));
            if let Some(product) = a.checked_mul(power) {
              assert_eq!(native::mul_pow10(a, exp), product);
            }
            // As with `/` and `%`, the quotient and the remainder have the sign of `a`.
            assert_eq!(native::div_rem_pow10(a, exp), (a / power, a % power));
          }
        }
      }

      #[test]
      fn division_and_rounding() {
        let samples = bigint::thin(samples());
        for &a in &samples {
          // As for `/` and `%`, a zero divisor and `MIN / -1` are excluded.
          for &b in samples.iter().filter(|&&b| a.checked_div(b).is_some()) {
            assert_eq!(native::div_rem(a, b), (a / b, a % b), "{a} / {b}");
          }
        }
        // Rounding by positive divisors, for fewer pairs under Miri, which runs big integers
        // slowly.
        let stride = if cfg!(miri) { 4 } else { 1 };
        let divisors: Vec<$int> =
          samples.iter().copied().filter(|&b| b > 0).step_by(stride).collect();
        for &a in samples.iter().step_by(stride) {
          for &b in &divisors {
            let (big_a, big_b) = (BigInt::from(a), BigInt::from(b));
            for mode in bigint::MODES {
              let (rounded, remainder, away) = native::round_div(a, b, mode);
              let expected = bigint::round(&big_a, &big_b, mode);
              assert_eq!(BigInt::from(rounded), expected, "{a} / {b}, {mode:?}");
              assert_eq!(remainder, a % b);
              assert_eq!(away, rounded != a / b);
            }
          }
        }
      }

      #[test]
      fn decimal_digits() {
        for a in samples() {
          let mut buf = [0xff; DIGIT_BUF_LEN];
          let start = native::write_digits(a, &mut buf);
          let expected = core_tests!(@magnitude $sign a).to_string();
          assert_eq!(&buf[start..], expected.as_bytes());

          if core_tests!(@is_not_negative $sign a) {
            let digits = expected.as_bytes();
            assert_eq!(native::push_digits(native::ZERO, digits, 0, digits.len()), a);
            // Appending to a prefix continues the same number.
            let split = digits.len() / 2;
            let prefix = native::push_digits(native::ZERO, digits, 0, split);
            assert_eq!(native::push_digits(prefix, digits, split, digits.len()), a);
            // Separators between the digits are skipped.
            let mut separated = Vec::new();
            for (i, &digit) in digits.iter().enumerate() {
              if i == split {
                separated.push(b'.');
              }
              separated.extend([digit, b'_']);
            }
            assert_eq!(native::push_digits(native::ZERO, &separated, 0, separated.len()), a);
          }
        }
      }

      #[test]
      fn limbs() {
        for a in samples() {
          let limbs = native::to_limbs(a);
          // Extended to 128 bits by the sign of the type.
          assert_eq!(
            limbs[0] as u128 | (limbs[1] as u128) << 64,
            core_tests!(@to_u128 $sign a)
          );
          assert_eq!(native::from_limbs(limbs), a);
          assert_eq!(native::checked_from_limbs(limbs), Some(a));
        }
        // Two limbs that the type cannot hold, read with the sign of the type, are rejected.
        for wide in core_tests!(@wide_samples $sign) {
          let limbs = [wide as u64, (wide >> 64) as u64];
          assert_eq!(native::checked_from_limbs(limbs), <$int>::try_from(wide).ok());
        }
      }

      #[test]
      fn bit_functions() {
        for bits in 0..=<$int>::BITS {
          core_tests!(@bit_functions $sign $int, bits);
        }
      }

      core_tests!(@sign_tests $sign $int);
    }
  )+};

  // `max_of_bits`, `min_of_bits` and `fits_bits` at one width, against the limits computed in 128
  // bits, for the values next to the limits and, for a type of 8 or 16 bits, for every value, or
  // under Miri every 61st value of 16 bits.
  (@bit_functions unsigned $int:ident, $bits:ident) => {{
    let max = if $bits == 128 { u128::MAX } else { (1u128 << $bits) - 1 };
    assert_eq!(native::max_of_bits($bits) as u128, max, "{} bits", $bits);
    let check = |value: $int| {
      let fits = value as u128 <= max;
      assert_eq!(native::fits_bits(value, $bits), fits, "{value} in {} bits", $bits);
    };
    let edges = [0, 1, max.saturating_sub(1), max, max.saturating_add(1), <$int>::MAX as u128];
    edges.into_iter().filter_map(|value| <$int>::try_from(value).ok()).for_each(check);
    if <$int>::BITS <= 16 {
      let step = if cfg!(miri) && <$int>::BITS == 16 { 61 } else { 1 };
      (0..=<$int>::MAX).step_by(step).for_each(check);
    }
  }};
  (@bit_functions signed $int:ident, $bits:ident) => {{
    // Zero bits hold only zero.
    let (min, max) = match $bits {
      0 => (0, 0),
      128 => (i128::MIN, i128::MAX),
      _ => (-(1i128 << ($bits - 1)), (1i128 << ($bits - 1)) - 1),
    };
    assert_eq!(native::max_of_bits($bits) as i128, max, "{} bits", $bits);
    assert_eq!(native::min_of_bits($bits) as i128, min, "{} bits", $bits);
    let check = |value: $int| {
      let fits = min <= value as i128 && value as i128 <= max;
      assert_eq!(native::fits_bits(value, $bits), fits, "{value} in {} bits", $bits);
    };
    let edges = [
      min.saturating_sub(1),
      min,
      min + 1,
      -1,
      0,
      1,
      max - 1,
      max,
      max.saturating_add(1),
      <$int>::MIN as i128,
      <$int>::MAX as i128,
    ];
    edges.into_iter().filter_map(|value| <$int>::try_from(value).ok()).for_each(check);
    if <$int>::BITS <= 16 {
      let step = if cfg!(miri) && <$int>::BITS == 16 { 61 } else { 1 };
      (<$int>::MIN..=<$int>::MAX).step_by(step).for_each(check);
    }
  }};

  (@samples signed $int:ident) => {{
    let mut values = std::vec![0, 1, -1, 9, -9, 10, -10, <$int>::MAX, <$int>::MIN];
    for exp in 1..=max_exp() {
      let power = native::pow10(exp);
      values.extend([power, power - 1, power + 1, -power, -(power - 1), -(power + 1)]);
    }
    values
  }};
  (@samples unsigned $int:ident) => {{
    let mut values = std::vec![0, 1, 9, 10, <$int>::MAX - 1, <$int>::MAX];
    for exp in 1..=max_exp() {
      let power = native::pow10(exp);
      values.extend([power, power - 1, power + 1]);
    }
    values
  }};

  (@is_negative signed $a:ident) => {
    assert_eq!(native::is_negative($a), $a < 0)
  };
  (@is_negative unsigned $a:ident) => {};

  (@magnitude signed $a:ident) => {
    $a.unsigned_abs()
  };
  (@magnitude unsigned $a:ident) => {
    $a
  };

  (@is_not_negative signed $a:ident) => {
    $a >= 0
  };
  (@is_not_negative unsigned $a:ident) => {
    true
  };

  (@to_u128 signed $a:ident) => {
    $a as i128 as u128
  };
  (@to_u128 unsigned $a:ident) => {
    $a as u128
  };

  // 128-bit values at and beyond the limits of every narrower type.
  (@wide_samples signed) => {{
    let mut values = std::vec![0i128, 1, -1, i128::MIN, i128::MAX];
    for bits in [8, 16, 32, 64] {
      let max = (1i128 << (bits - 1)) - 1;
      values.extend([max, max + 1, -max - 1, -max - 2]);
    }
    values
  }};
  (@wide_samples unsigned) => {{
    let mut values = std::vec![0u128, 1, u128::MAX];
    for bits in [8, 16, 32, 64] {
      let max = (1u128 << bits) - 1;
      values.extend([max, max + 1]);
    }
    values
  }};

  (@sign_tests signed $int:ident) => {
    #[test]
    fn neg_and_abs() {
      for a in samples().into_iter().filter(|&a| a != <$int>::MIN) {
        assert_eq!(native::neg(a), -a);
        assert_eq!(native::abs(a), a.abs());
      }
    }
  };
  (@sign_tests unsigned $int:ident) => {};
}

core_tests!(
  unsigned u8,
  unsigned u16,
  unsigned u32,
  unsigned u64,
  unsigned u128,
  signed i8,
  signed i16,
  signed i32,
  signed i64,
  signed i128,
);

/// Tests of the modules for the wide storages against `num-bigint`. Each storage is listed with its
/// number of limbs, the most digits it holds, and whether twice the largest coefficient of that many
/// digits still fits it.
macro_rules! wide_tests {
  ($($sign:ident $int:ident: $limbs:literal, $digits:literal, $sums_fit:literal),+ $(,)?) => {$(
    mod $int {
      use num_bigint::{BigInt, Sign};
      use std::{string::ToString, vec, vec::Vec};

      use crate::{
        $int as Wide, bigint,
        int::{$int as wide, DIGIT_BUF_LEN},
      };

      const SIGNED: bool = wide_tests!(@signed $sign);
      const BITS: u32 = 64 * $limbs;

      fn big(value: Wide) -> BigInt {
        bigint::from_limbs(&value.to_limbs(), SIGNED)
      }

      fn from_big(value: &BigInt) -> Option<Wide> {
        bigint::to_limbs::<$limbs>(value, SIGNED).map(Wide::from_limbs)
      }

      fn max() -> BigInt {
        bigint::max(BITS, SIGNED)
      }

      fn min() -> BigInt {
        bigint::min(BITS, SIGNED)
      }

      /// Each value with its integer, computed once for the pairwise tests.
      fn with_big(values: Vec<Wide>) -> Vec<(Wide, BigInt)> {
        values.into_iter().map(|value| (value, big(value))).collect()
      }

      /// The values among `candidates`, and their negations for a signed type, that the type
      /// holds, without duplicates.
      fn representable(candidates: Vec<BigInt>) -> Vec<Wide> {
        let mut values: Vec<BigInt> = Vec::new();
        for candidate in candidates {
          let negated = -&candidate;
          for value in [candidate, negated] {
            if from_big(&value).is_some() && !values.contains(&value) {
              values.push(value);
            }
          }
        }
        values.iter().map(|value| from_big(value).expect("representable")).collect()
      }

      /// Values at the limb boundaries, with all-ones limbs, at the top limb's sign bit, around
      /// the square root of `MAX` and at the limits of the type.
      fn edges() -> Vec<Wide> {
        let one = BigInt::from(1u8);
        let mut candidates = vec![BigInt::ZERO, one.clone(), BigInt::from(9u8), max(), min()];
        candidates.extend([max() - 1u8, min() + 1u8]);
        for k in 1u32..$limbs {
          let boundary = &one << (64 * k);
          candidates.extend([&boundary - 1u8, boundary.clone(), &boundary + 1u8]);
        }
        for k in 0..$limbs {
          // All limbs but `k` set to ones, and only limb `k` set to ones.
          let mut limbs = [u64::MAX; $limbs];
          limbs[k] = 0;
          candidates.push(bigint::from_limbs(&limbs, SIGNED));
          let mut limbs = [0; $limbs];
          limbs[k] = u64::MAX;
          candidates.push(bigint::from_limbs(&limbs, SIGNED));
        }
        let top = &one << (BITS - 1);
        candidates.extend([&top - 1u8, top.clone(), &top + 1u8]);
        let root = max().sqrt();
        candidates.extend([&root - 1u8, root.clone(), &root + 1u8]);
        let max_power = bigint::pow10(bigint::max_exp(&max()));
        candidates.extend([&max_power - 1u8, max_power]);
        bigint::thin(representable(candidates))
      }

      /// Values around every power of ten that the type holds.
      fn powers() -> Vec<Wide> {
        let mut candidates = Vec::new();
        for exp in 0..=bigint::max_exp(&max()) {
          let power = bigint::pow10(exp);
          candidates.extend([&power - 1u8, power.clone(), power + 1u8]);
        }
        bigint::thin(representable(candidates))
      }

      #[test]
      fn pow10_and_max_magnitude() {
        let max_exp = bigint::max_exp(&max());
        for exp in bigint::exponents(max_exp) {
          assert_eq!(big(wide::pow10(exp)), bigint::pow10(exp));
          assert_eq!(big(wide::max_magnitude(exp)), bigint::pow10(exp) - 1u8);
        }
        assert!(bigint::pow10(max_exp + 1) > max());
      }

      #[test]
      fn predicates_and_comparison() {
        let samples = with_big(edges());
        for &(a, ref big_a) in &samples {
          assert_eq!(wide::is_zero(a), big_a.sign() == Sign::NoSign);
          assert_eq!(wide::is_positive(a), big_a.sign() == Sign::Plus);
          wide_tests!(@is_negative $sign a, big_a);
          for &(b, ref big_b) in &samples {
            assert_eq!(wide::le(a, b), big_a <= big_b);
          }
        }
      }

      #[test]
      fn checked_arithmetic() {
        let samples = with_big(edges());
        for &(a, ref big_a) in &samples {
          for &(b, ref big_b) in &samples {
            assert_eq!(wide::checked_add(a, b), from_big(&(big_a + big_b)));
            assert_eq!(wide::checked_sub(a, b), from_big(&(big_a - big_b)));
          }
        }
      }

      #[test]
      fn sums_of_the_largest_coefficient() {
        assert_eq!(bigint::max_exp(&max()), $digits);
        let largest = wide::max_magnitude($digits);
        assert_eq!(big(largest), bigint::pow10($digits) - 1u8);
        let sum = wide::checked_add(largest, largest);
        assert_eq!(sum.is_some(), $sums_fit);
        assert_eq!(sum, from_big(&(big(largest) * 2u8)));
      }

      #[test]
      fn multiplication() {
        let samples = with_big(edges());
        for &(a, ref big_a) in &samples {
          for &(b, ref big_b) in &samples {
            if let Some(product) = from_big(&(big_a * big_b)) {
              assert_eq!(wide::mul(a, b), product, "{a:?} * {b:?}");
            }
          }
        }
      }

      #[test]
      fn division_and_rounding() {
        let samples = with_big(edges());
        let minus_one = BigInt::from(-1);
        for &(a, ref big_a) in &samples {
          // As for `/` and `%`, a zero divisor and `MIN / -1` are excluded.
          for &(b, ref big_b) in &samples {
            if big_b.sign() == Sign::NoSign || (*big_a == min() && *big_b == minus_one) {
              continue;
            }
            let (quotient, remainder) = wide::div_rem(a, b);
            assert_eq!(big(quotient), big_a / big_b, "{a:?} / {b:?}");
            assert_eq!(big(remainder), big_a % big_b, "{a:?} % {b:?}");
          }
        }
        // Rounding by positive divisors, for fewer pairs under Miri, which runs big integers
        // slowly.
        let stride = if cfg!(miri) { 3 } else { 1 };
        for &(a, ref big_a) in samples.iter().step_by(stride) {
          let divisors = samples.iter().filter(|(_, big_b)| big_b.sign() == Sign::Plus);
          for &(b, ref big_b) in divisors.step_by(stride) {
            let truncated = big_a / big_b;
            for mode in bigint::MODES {
              let (rounded, remainder, away) = wide::round_div(a, b, mode);
              let expected = bigint::round(big_a, big_b, mode);
              assert_eq!(big(rounded), expected, "{a:?} / {b:?}, {mode:?}");
              assert_eq!(big(remainder), big_a % big_b);
              assert_eq!(away, big(rounded) != truncated);
            }
          }
        }
      }

      /// The edges and, except under Miri, the values around the powers of ten.
      fn samples() -> Vec<Wide> {
        let mut samples = edges();
        if !cfg!(miri) {
          samples.extend(powers());
        }
        samples
      }

      #[test]
      fn powers_of_ten() {
        // Under Miri, only the edges, scaled by the exponents at the edges of the scaling steps.
        let powers: Vec<(u32, BigInt)> = bigint::exponents(bigint::max_exp(&max()))
          .into_iter()
          .map(|exp| (exp, bigint::pow10(exp)))
          .collect();
        for (a, big_a) in with_big(samples()) {
          for &(exp, ref power) in &powers {
            let product = from_big(&(&big_a * power));
            assert_eq!(wide::checked_mul_pow10(a, exp), product);
            if let Some(product) = product {
              assert_eq!(wide::mul_pow10(a, exp), product);
            }
            // As with `/` and `%`, the quotient and the remainder have the sign of `a`.
            let (quotient, remainder) = wide::div_rem_pow10(a, exp);
            assert_eq!(big(quotient), &big_a / power);
            assert_eq!(big(remainder), &big_a % power);
          }
        }
      }

      #[test]
      fn decimal_digits() {
        for a in samples() {
          let mut buf = [0xff; DIGIT_BUF_LEN];
          let start = wide::write_digits(a, &mut buf);
          let expected = big(a).magnitude().to_string();
          assert_eq!(&buf[start..], expected.as_bytes());

          if big(a).sign() != Sign::Minus {
            let digits = expected.as_bytes();
            assert_eq!(wide::push_digits(wide::ZERO, digits, 0, digits.len()), a);
            // Appending to a prefix continues the same number.
            let split = digits.len() / 2;
            let prefix = wide::push_digits(wide::ZERO, digits, 0, split);
            assert_eq!(wide::push_digits(prefix, digits, split, digits.len()), a);
            // Separators between the digits are skipped.
            let mut separated = Vec::new();
            for (i, &digit) in digits.iter().enumerate() {
              if i == split {
                separated.push(b'.');
              }
              separated.extend([digit, b'_']);
            }
            assert_eq!(wide::push_digits(wide::ZERO, &separated, 0, separated.len()), a);
          }
        }
      }

      #[test]
      fn limbs() {
        for a in edges() {
          let limbs = wide::to_limbs(a);
          assert_eq!(limbs, a.to_limbs());
          assert_eq!(wide::from_limbs(limbs), a);
          assert_eq!(wide::checked_from_limbs(limbs), Some(a));
        }
        assert_eq!(big(wide::ZERO), BigInt::ZERO);
      }

      wide_tests!(@sign_tests $sign);
    }
  )+};

  (@signed signed) => {
    true
  };
  (@signed unsigned) => {
    false
  };

  (@is_negative signed $a:ident, $big:ident) => {
    assert_eq!(wide::is_negative($a), $big.sign() == Sign::Minus)
  };
  (@is_negative unsigned $a:ident, $big:ident) => {};

  (@sign_tests signed) => {
    #[test]
    fn neg_and_abs() {
      for a in edges().into_iter().filter(|&a| big(a) != min()) {
        assert_eq!(big(wide::neg(a)), -big(a));
        assert_eq!(big(wide::abs(a)), BigInt::from(big(a).magnitude().clone()));
      }
    }
  };
  (@sign_tests unsigned) => {};
}

wide_tests!(
  unsigned u192: 3, 57, true,
  unsigned u256: 4, 77, false,
  unsigned u320: 5, 96, true,
  unsigned u384: 6, 115, true,
  unsigned u448: 7, 134, true,
  unsigned u512: 8, 154, false,
  signed i192: 3, 57, true,
  signed i256: 4, 76, true,
  signed i320: 5, 96, false,
  signed i384: 6, 115, false,
  signed i448: 7, 134, true,
  signed i512: 8, 153, true,
);

/// The arithmetic of a width in the native layers: every pair of values of up to 8 bits in `u8`,
/// `i8`, `u16` and `i16`, every value of 9 to 16 bits in `u16` and `i16` against the structured
/// operands of its width, and in the wider types the structured operands of a few widths, against
/// wider native integers or, for `u128` and `i128`, against `num-bigint`.
mod arithmetic_bits {
  use num_bigint::BigInt;
  use std::{vec, vec::Vec};

  use crate::{bigint, int};

  /// The range of `bits` bits, unsigned or in two's complement.
  fn range(bits: u32, signed: bool) -> (i128, i128) {
    match (bits, signed) {
      (0, _) => (0, 0),
      (_, false) => (0, (1 << bits) - 1),
      (_, true) => (-(1 << (bits - 1)), (1 << (bits - 1)) - 1),
    }
  }

  /// `exact` wrapped to the range of `bits` bits, at most 64, and whether it is outside that
  /// range.
  fn wrapped(exact: i128, bits: u32, signed: bool) -> (i128, bool) {
    let (min, max) = range(bits, signed);
    let result = (exact - min).rem_euclid(1 << bits) + min;
    (result, !(min..=max).contains(&exact))
  }

  /// The unsigned results of `bits` bits, at most 64, for `a` and `b` below `2^bits`, computed
  /// modulo `2^128` and reduced to `bits` bits: the sum, the difference, the product and the
  /// negation of `a`, each with whether the exact result is outside the range of `bits` bits.
  fn unsigned_results(a: u128, b: u128, bits: u32) -> [(u128, bool); 4] {
    let mask = (1 << bits) - 1;
    // `a` and `b` are below `2^64`, so neither the sum nor the product overflows `u128`.
    let (sum, product) = (a + b, a * b);
    [
      (sum & mask, sum > mask),
      (a.wrapping_sub(b) & mask, a < b),
      (product & mask, product > mask),
      (a.wrapping_neg() & mask, a != 0),
    ]
  }

  /// Checks the arithmetic of each width of `$widths` in the unsigned layer `$uint` and the signed
  /// layer `$int`, for `bits` bits at most 64: every value of up to `$every` bits against every
  /// other, or under Miri every 61st pair, and otherwise every value against the structured
  /// operands of the width, or under Miri every 61st value against them.
  macro_rules! check_native {
    ($uint:ident, $int:ident, $widths:expr, $every:expr) => {{
      let step = if cfg!(miri) { 61 } else { 1 };
      for bits in $widths {
        // Every value, or the structured operands.
        let lefts: Vec<u128> = if bits <= $every {
          (0..1u128 << bits).collect()
        } else {
          bigint::structured_patterns(bits)
        };
        let rights: Vec<u128> = if bits <= $every {
          lefts.clone()
        } else {
          bigint::structured_patterns(bits)
        };
        for &a in lefts.iter().step_by(step) {
          let signed = bigint::to_signed(a, bits);
          assert_eq!(int::$uint::is_one(a as $uint), a == 1);
          assert_eq!(int::$int::is_one(signed as $int), signed == 1);
          assert_eq!(int::$int::signum(signed as $int), signed.signum() as $int);
          assert_eq!(
            int::$int::unsigned_abs(signed as $int),
            signed.unsigned_abs() as $uint
          );
          for &b in &rights {
            let [sum, difference, product, negated] = unsigned_results(a, b, bits);
            let (x, y) = (a as $uint, b as $uint);
            let native = |(value, overflow): (u128, bool)| (value as $uint, overflow);
            assert_eq!(int::$uint::overflowing_add_bits(x, y, bits), native(sum));
            assert_eq!(
              int::$uint::overflowing_sub_bits(x, y, bits),
              native(difference)
            );
            assert_eq!(
              int::$uint::overflowing_mul_bits(x, y, bits),
              native(product)
            );
            assert_eq!(int::$uint::overflowing_neg_bits(x, bits), native(negated));

            let (a, b) = (bigint::to_signed(a, bits), bigint::to_signed(b, bits));
            let (x, y) = (a as $int, b as $int);
            for (exact, op, result) in [
              (a + b, '+', int::$int::overflowing_add_bits(x, y, bits)),
              (a - b, '-', int::$int::overflowing_sub_bits(x, y, bits)),
              (a * b, '*', int::$int::overflowing_mul_bits(x, y, bits)),
            ] {
              let (value, overflow) = wrapped(exact, bits, true);
              assert_eq!(
                result,
                (value as $int, overflow),
                "{a} {op} {b} in {bits} bits"
              );
            }
            let (value, overflow) = wrapped(-a, bits, true);
            assert_eq!(
              int::$int::overflowing_neg_bits(x, bits),
              (value as $int, overflow),
              "-{a} in {bits} bits"
            );
          }
        }
      }
    }};
  }

  #[test]
  fn u8_and_i8_match_wider_integers() {
    check_native!(u8, i8, 0..=8, 8);
  }

  #[test]
  fn u16_and_i16_match_wider_integers() {
    let widths: Vec<u32> = if cfg!(miri) {
      vec![0, 1, 7, 8, 9, 16]
    } else {
      (0..=16).collect()
    };
    check_native!(u16, i16, widths, 8);
  }

  #[test]
  fn u32_to_i64_match_wider_integers() {
    check_native!(u32, i32, [0, 1, 2, 16, 31, 32], 0);
    check_native!(u64, i64, [0, 1, 2, 32, 63, 64], 0);
  }

  /// The structured operands of 128 bits and of a few narrower widths in `u128` and `i128`, against
  /// `num-bigint`.
  #[test]
  #[cfg_attr(
    all(miri, target_pointer_width = "32"),
    ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
  )]
  fn u128_and_i128_match_big_integers() {
    for bits in [0, 1, 2, 64, 65, 127, 128] {
      for signed in [false, true] {
        let (min, max) = (bigint::min(bits, signed), bigint::max(bits, signed));
        let mut patterns = bigint::structured_patterns(bits).into_iter().cycle();
        let samples = bigint::arithmetic_samples(bits, 64, &min, &max, || {
          BigInt::from(patterns.next().expect("a pattern"))
        });
        // `exact` is `lhs op rhs`, and `computed` its result in the layer, read as an integer.
        let check = |exact: BigInt, computed: (BigInt, bool), lhs: &BigInt, op, rhs: &BigInt| {
          let expected = (bigint::wrap(&exact, bits, &min), exact < min || max < exact);
          assert_eq!(computed, expected, "{lhs} {op} {rhs} in {bits} bits");
        };
        let zero = BigInt::ZERO;
        for a in &samples {
          if signed {
            let x = i128::try_from(a).expect("in range");
            assert_eq!(int::i128::is_one(x), x == 1);
            assert_eq!(int::i128::signum(x), x.signum());
            assert_eq!(int::i128::unsigned_abs(x), x.unsigned_abs());
            let big = |(value, overflow): (i128, bool)| (BigInt::from(value), overflow);
            check(
              -a,
              big(int::i128::overflowing_neg_bits(x, bits)),
              &zero,
              '-',
              a,
            );
            for b in &samples {
              let y = i128::try_from(b).expect("in range");
              for (exact, op, computed) in [
                (a + b, '+', int::i128::overflowing_add_bits(x, y, bits)),
                (a - b, '-', int::i128::overflowing_sub_bits(x, y, bits)),
                (a * b, '*', int::i128::overflowing_mul_bits(x, y, bits)),
              ] {
                check(exact, big(computed), a, op, b);
              }
            }
          } else {
            let x = u128::try_from(a).expect("in range");
            assert_eq!(int::u128::is_one(x), x == 1);
            let big = |(value, overflow): (u128, bool)| (BigInt::from(value), overflow);
            check(
              -a,
              big(int::u128::overflowing_neg_bits(x, bits)),
              &zero,
              '-',
              a,
            );
            for b in &samples {
              let y = u128::try_from(b).expect("in range");
              for (exact, op, computed) in [
                (a + b, '+', int::u128::overflowing_add_bits(x, y, bits)),
                (a - b, '-', int::u128::overflowing_sub_bits(x, y, bits)),
                (a * b, '*', int::u128::overflowing_mul_bits(x, y, bits)),
              ] {
                check(exact, big(computed), a, op, b);
              }
            }
          }
        }
      }
    }
  }
}
