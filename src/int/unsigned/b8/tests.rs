use crate::int::limb_layer::tests::{limb_layer_tests, native_oracle, other_limb_tests};

// The tests that every instance shares, with their helpers and the imports that the tests below
// use as well.
limb_layer_tests!(unsigned);

// The helpers of the tests below, which compare the layer with native integers and with the layer
// for `[u64; N]`.
other_limb_tests!(unsigned);

native_oracle!(unsigned one_limb_matches_u8, u8, 1);
native_oracle!(unsigned two_limbs_match_u16, u16, 2);
native_oracle!(unsigned four_limbs_match_u32, u32, 4);
native_oracle!(unsigned eight_limbs_match_u64, u64, 8);
native_oracle!(unsigned sixteen_limbs_match_u128, u128, 16);

/// Steps whose window has the top divisor limb on top, where the quotient of the window's top two
/// limbs by that limb is `2^8` or `2^8 + 1`.
#[test]
fn division_estimates_reach_two_pow_8() {
  division_with_equal_top_limbs();
}

#[test]
fn groups_of_eight_limbs_match_the_u64_layer() {
  matches_the_u64_layer::<8, 1>();
  matches_the_u64_layer::<16, 2>();
  matches_the_u64_layer::<24, 3>();
  matches_the_u64_layer::<32, 4>();
  matches_the_u64_layer::<64, 8>();
}

/// Every pair of one-limb values against `u8`, and every value against `u8` and the native lower
/// layer for `u8`, or under Miri, which runs them slowly, every 61st pair and every 7th value.
#[test]
fn every_one_limb_value_matches_u8() {
  let (pair_step, value_step) = if cfg!(miri) { (61, 7) } else { (1, 1) };
  for pair in (0..=u16::MAX).step_by(pair_step) {
    let [a, b] = pair.to_le_bytes();
    assert_eq!(cmp(&[a], &[b]), a.cmp(&b));
    assert_eq!(le(&[a], &[b]), a <= b);
    let (sum, carry) = a.overflowing_add(b);
    assert_eq!(overflowing_add(&[a], &[b]), ([sum], carry));
    let (difference, borrow) = a.overflowing_sub(b);
    assert_eq!(overflowing_sub(&[a], &[b]), ([difference], borrow));
    assert_eq!(checked_add(&[a], &[b]), a.checked_add(b).map(|sum| [sum]));
    assert_eq!(
      checked_sub(&[a], &[b]),
      a.checked_sub(b).map(|difference| [difference])
    );
    if let Some(product) = a.checked_mul(b) {
      assert_eq!(mul(&[a], &[b]), [product]);
    }
    if b != 0 {
      assert_eq!(div_rem(&[a], &[b]), ([a / b], [a % b]), "{a} / {b}");
      for mode in bigint::MODES {
        let (rounded, remainder, away) = crate::int::u8::round_div(a, b, mode);
        assert_eq!(
          round_div(&[a], &[b], mode),
          ([rounded], [remainder], away),
          "{a} / {b}, {mode:?}"
        );
      }
    }
  }
  for a in (0..=u8::MAX).step_by(value_step) {
    assert_eq!(is_zero(&[a]), a == 0);
    assert_eq!(is_positive(&[a]), a > 0);
    // Up to two exponents beyond the largest power of ten that a `u8` holds, `10^2`.
    for exp in 0..=4 {
      let power = 10u32.pow(exp);
      let product = u8::try_from(u32::from(a) * power).ok();
      assert_eq!(
        checked_mul_pow10(&[a], exp),
        product.map(|product| [product])
      );
      if let Some(product) = product {
        assert_eq!(mul_pow10(&[a], exp), [product]);
      }
      let (quotient, remainder) = (u32::from(a) / power, u32::from(a) % power);
      assert_eq!(
        div_rem_pow10(&[a], exp),
        ([quotient as u8], [remainder as u8])
      );
    }
    let mut buf = [0xff; DIGIT_BUF_LEN];
    let start = write_digits(&[a], &mut buf);
    assert_eq!(&buf[start..], a.to_string().as_bytes());
    assert_eq!(push_digits(&[0], &buf, start, DIGIT_BUF_LEN), [a]);
    // Every width from zero bits to one bit more than the limb.
    for bits in 0..=9 {
      assert_eq!(
        fits_bits(&[a], bits),
        u32::from(a) < 1 << bits,
        "{a} in {bits} bits"
      );
    }
  }
  for bits in 0..=8 {
    assert_eq!(max_of_bits::<1>(bits), [((1u32 << bits) - 1) as u8]);
  }
}
