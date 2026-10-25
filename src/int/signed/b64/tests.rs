use crate::int::limb_layer::tests::limb_layer_tests;

// The tests that every instance shares, with their helpers and the imports that the tests below
// use as well.
limb_layer_tests!(signed unsigned: b64);

fn from_i128(value: i128) -> [u64; 2] {
  [value as u64, (value >> 64) as u64]
}

fn to_i128(limbs: [u64; 2]) -> i128 {
  (limbs[0] as u128 | (limbs[1] as u128) << 64) as i128
}

/// Values around zero, the limb boundary, the powers of ten and the limits of `i128`.
fn samples() -> Vec<i128> {
  let mut values = vec![
    0,
    1,
    -1,
    9,
    -9,
    i128::MAX,
    i128::MIN,
    i128::MAX - 1,
    i128::MIN + 1,
  ];
  for boundary in [1i128 << 63, 1 << 64] {
    values.extend([boundary - 1, boundary, boundary + 1]);
    values.extend([1 - boundary, -boundary, -boundary - 1]);
  }
  for exp in 1..=38 {
    let power = 10i128.pow(exp);
    values.extend([power - 1, power, power + 1, 1 - power, -power, -power - 1]);
  }
  values
}

#[test]
fn one_limb_matches_i64() {
  let samples: Vec<i64> = samples()
    .into_iter()
    .filter_map(|value| value.try_into().ok())
    .collect();
  for &a in &samples {
    let limbs_a = [a as u64];
    assert_eq!(is_negative(&limbs_a), a < 0);
    assert_eq!(is_positive(&limbs_a), a > 0);
    assert_eq!(is_zero(&limbs_a), a == 0);
    assert_eq!(unsigned_abs(&limbs_a), [a.unsigned_abs()]);
    if a != i64::MIN {
      assert_eq!(neg(&limbs_a), [(-a) as u64]);
      assert_eq!(abs(&limbs_a), [a.unsigned_abs()]);
    }
    for &b in &samples {
      let limbs_b = [b as u64];
      assert_eq!(cmp(&limbs_a, &limbs_b), a.cmp(&b));
      assert_eq!(le(&limbs_a, &limbs_b), a <= b);
      assert_eq!(
        checked_add(&limbs_a, &limbs_b),
        a.checked_add(b).map(|sum| [sum as u64])
      );
      assert_eq!(
        checked_sub(&limbs_a, &limbs_b),
        a.checked_sub(b).map(|diff| [diff as u64])
      );
      if let Some(product) = a.checked_mul(b) {
        assert_eq!(mul(&limbs_a, &limbs_b), [product as u64]);
      }
    }
    for exp in 0..=18 {
      let power = 10i64.pow(exp);
      assert_eq!(
        checked_mul_pow10(&limbs_a, exp),
        a.checked_mul(power).map(|product| [product as u64])
      );
      if let Some(product) = a.checked_mul(power) {
        assert_eq!(mul_pow10(&limbs_a, exp), [product as u64]);
      }
      // As with `/` and `%`, the quotient and the remainder have the sign of `a`.
      let (quotient, remainder) = div_rem_pow10(&limbs_a, exp);
      assert_eq!(
        (quotient[0] as i64, remainder[0] as i64),
        (a / power, a % power)
      );
    }
  }
}

#[test]
fn two_limbs_match_i128() {
  for a in samples() {
    let limbs_a = from_i128(a);
    assert_eq!(is_negative(&limbs_a), a < 0);
    assert_eq!(is_positive(&limbs_a), a > 0);
    assert_eq!(unsigned_abs(&limbs_a), {
      let magnitude = a.unsigned_abs();
      [magnitude as u64, (magnitude >> 64) as u64]
    });
    if a != i128::MIN {
      assert_eq!(neg(&limbs_a), from_i128(-a));
      assert_eq!(abs(&limbs_a), from_i128(a.abs()));
    }
    for b in samples() {
      let limbs_b = from_i128(b);
      assert_eq!(cmp(&limbs_a, &limbs_b), a.cmp(&b));
      assert_eq!(le(&limbs_a, &limbs_b), a <= b);
      assert_eq!(
        checked_add(&limbs_a, &limbs_b),
        a.checked_add(b).map(from_i128)
      );
      assert_eq!(
        checked_sub(&limbs_a, &limbs_b),
        a.checked_sub(b).map(from_i128)
      );
      if let Some(product) = a.checked_mul(b) {
        assert_eq!(mul(&limbs_a, &limbs_b), from_i128(product));
      }
    }
    for exp in 0..=38 {
      let power = 10i128.pow(exp);
      assert_eq!(
        checked_mul_pow10(&limbs_a, exp),
        a.checked_mul(power).map(from_i128)
      );
      if let Some(product) = a.checked_mul(power) {
        assert_eq!(mul_pow10(&limbs_a, exp), from_i128(product));
      }
      let (quotient, remainder) = div_rem_pow10(&limbs_a, exp);
      assert_eq!(
        (to_i128(quotient), to_i128(remainder)),
        (a / power, a % power)
      );
    }

    let mut buf = [0xff; DIGIT_BUF_LEN];
    let start = write_digits(&limbs_a, &mut buf);
    assert_eq!(&buf[start..], a.unsigned_abs().to_string().as_bytes());
    if a >= 0 {
      let digits = a.to_string();
      assert_eq!(
        push_digits(&[0; 2], digits.as_bytes(), 0, digits.len()),
        limbs_a
      );
    }
  }
}

#[test]
fn division_matches_i64_and_i128() {
  let samples = bigint::thin(samples());
  for &a in &samples {
    // As for `/` and `%`, a zero divisor and `MIN / -1` are excluded.
    for &b in samples.iter().filter(|&&b| b != 0) {
      if let (Ok(a), Ok(b)) = (i64::try_from(a), i64::try_from(b))
        && (a, b) != (i64::MIN, -1)
      {
        let (quotient, remainder) = div_rem(&[a as u64], &[b as u64]);
        assert_eq!(
          (quotient[0] as i64, remainder[0] as i64),
          (a / b, a % b),
          "{a} / {b}"
        );
      }
      if (a, b) != (i128::MIN, -1) {
        let (quotient, remainder) = div_rem(&from_i128(a), &from_i128(b));
        assert_eq!(
          (to_i128(quotient), to_i128(remainder)),
          (a / b, a % b),
          "{a} / {b}"
        );
      }
    }
  }
}
