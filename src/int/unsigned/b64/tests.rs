use crate::int::limb_layer::tests::limb_layer_tests;

// The tests that every instance shares, with their helpers and the imports that the tests below
// use as well.
limb_layer_tests!(unsigned);

fn from_u128(value: u128) -> [u64; 2] {
  [value as u64, (value >> 64) as u64]
}

fn to_u128(limbs: [u64; 2]) -> u128 {
  limbs[0] as u128 | (limbs[1] as u128) << 64
}

/// Values around zero, the limb boundary, the powers of ten and the limit of `u128`.
fn samples() -> Vec<u128> {
  let mut values = vec![0, 1, 2, 9, 10, u128::MAX, u128::MAX - 1];
  for boundary in [1u128 << 64, 1 << 127] {
    values.extend([boundary - 1, boundary, boundary + 1]);
  }
  for exp in 1..=38 {
    let power = 10u128.pow(exp);
    values.extend([power - 1, power, power + 1]);
  }
  values
}

#[test]
fn one_limb_matches_u64() {
  let samples: Vec<u64> = samples()
    .into_iter()
    .filter_map(|value| value.try_into().ok())
    .collect();
  for &a in &samples {
    assert_eq!(is_zero(&[a]), a == 0);
    assert_eq!(is_positive(&[a]), a > 0);
    for &b in &samples {
      assert_eq!(cmp(&[a], &[b]), a.cmp(&b));
      assert_eq!(le(&[a], &[b]), a <= b);
      let (sum, carry) = a.overflowing_add(b);
      assert_eq!(overflowing_add(&[a], &[b]), ([sum], carry));
      let (difference, borrow) = a.overflowing_sub(b);
      assert_eq!(overflowing_sub(&[a], &[b]), ([difference], borrow));
      if let Some(product) = a.checked_mul(b) {
        assert_eq!(mul(&[a], &[b]), [product]);
      }
    }
    for exp in 0..=25 {
      let power = 10u128.pow(exp);
      let product = (a as u128)
        .checked_mul(power)
        .and_then(|product| u64::try_from(product).ok());
      assert_eq!(
        checked_mul_pow10(&[a], exp),
        product.map(|product| [product])
      );
      if let Some(product) = product {
        assert_eq!(mul_pow10(&[a], exp), [product]);
      }
      let quotient = (a as u128 / power) as u64;
      let remainder = (a as u128 % power) as u64;
      assert_eq!(div_rem_pow10(&[a], exp), ([quotient], [remainder]));
    }
  }
}

#[test]
fn two_limbs_match_u128() {
  for a in samples() {
    let limbs_a = from_u128(a);
    assert_eq!(is_zero(&limbs_a), a == 0);
    for b in samples() {
      let limbs_b = from_u128(b);
      assert_eq!(cmp(&limbs_a, &limbs_b), a.cmp(&b));
      assert_eq!(le(&limbs_a, &limbs_b), a <= b);
      let (sum, carry) = a.overflowing_add(b);
      assert_eq!(overflowing_add(&limbs_a, &limbs_b), (from_u128(sum), carry));
      let (difference, borrow) = a.overflowing_sub(b);
      assert_eq!(
        overflowing_sub(&limbs_a, &limbs_b),
        (from_u128(difference), borrow)
      );
      assert_eq!(
        checked_add(&limbs_a, &limbs_b),
        a.checked_add(b).map(from_u128)
      );
      assert_eq!(
        checked_sub(&limbs_a, &limbs_b),
        a.checked_sub(b).map(from_u128)
      );
      if let Some(product) = a.checked_mul(b) {
        assert_eq!(mul(&limbs_a, &limbs_b), from_u128(product));
      }
    }
    for exp in 0..=40 {
      // Zero times any power is zero; otherwise the power itself must fit.
      let product = match a {
        0 => Some(0),
        _ => 10u128
          .checked_pow(exp)
          .and_then(|power| a.checked_mul(power)),
      };
      assert_eq!(checked_mul_pow10(&limbs_a, exp), product.map(from_u128));
      if let Some(product) = product {
        assert_eq!(mul_pow10(&limbs_a, exp), from_u128(product));
      }
      let (quotient, remainder) = div_rem_pow10(&limbs_a, exp);
      match 10u128.checked_pow(exp) {
        Some(power) => {
          assert_eq!(to_u128(quotient), a / power);
          assert_eq!(to_u128(remainder), a % power);
        }
        None => assert_eq!((quotient, remainder), ([0; 2], limbs_a)),
      }
    }

    let mut buf = [0xff; DIGIT_BUF_LEN];
    let start = write_digits(&limbs_a, &mut buf);
    assert_eq!(&buf[start..], a.to_string().as_bytes());
  }
  for exp in 0..=38 {
    assert_eq!(to_u128(pow10(exp)), 10u128.pow(exp));
    assert_eq!(to_u128(max_magnitude(exp)), 10u128.pow(exp) - 1);
  }
}

#[test]
fn division_matches_u64_and_u128() {
  let samples = bigint::thin(samples());
  for &a in &samples {
    for &b in samples.iter().filter(|&&b| b != 0) {
      if let (Ok(a), Ok(b)) = (u64::try_from(a), u64::try_from(b)) {
        assert_eq!(div_rem(&[a], &[b]), ([a / b], [a % b]), "{a} / {b}");
      }
      assert_eq!(
        div_rem(&from_u128(a), &from_u128(b)),
        (from_u128(a / b), from_u128(a % b)),
        "{a} / {b}"
      );
    }
  }
}

/// Steps whose window has the top divisor limb on top, where the quotient of the window's top two
/// limbs by that limb is `2^64` or `2^64 + 1`, and a step whose estimate stops correcting once its
/// remainder overflows a limb.
#[test]
fn division_estimates_reach_two_pow_64() {
  const MAX: u64 = u64::MAX;
  division_with_equal_top_limbs();
  // The remainder of the estimate overflows after one correction, so the loop stops before
  // comparing a truncated value.
  let a = [0x9a86_1c5d_6121_668a, 1, 0, 0x8000_0000_0000_0001];
  let b = [MAX, MAX - 1, 0, 0];
  assert_eq!(
    div_rem(&a, &b),
    (
      [0x8000_0000_0000_0002, 0x8000_0000_0000_0001, 0, 0],
      [0x1a86_1c5d_6121_668c, 5, 0, 0]
    )
  );
  check_division(&a, &b);
}
