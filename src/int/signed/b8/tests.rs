use crate::int::limb_layer::tests::{limb_layer_tests, native_oracle, other_limb_tests};

// The tests that every instance shares, with their helpers and the imports that the tests below
// use as well.
limb_layer_tests!(signed unsigned: b8);

// The helpers of the tests below, which compare the layer with native integers.
other_limb_tests!(signed);

native_oracle!(signed one_limb_matches_i8, i8, 1);
native_oracle!(signed two_limbs_match_i16, i16, 2);
native_oracle!(signed four_limbs_match_i32, i32, 4);
native_oracle!(signed eight_limbs_match_i64, i64, 8);
native_oracle!(signed sixteen_limbs_match_i128, i128, 16);

/// Every pair of one-limb values against `i8`, and every value against `i8` and the native upper
/// layer for `i8`, or under Miri, which runs them slowly, every 61st pair and every 7th value.
#[test]
fn every_one_limb_value_matches_i8() {
  let (pair_step, value_step) = if cfg!(miri) { (61, 7) } else { (1, 1) };
  for pair in (0..=u16::MAX).step_by(pair_step) {
    let [limb_a, limb_b] = pair.to_le_bytes();
    let (a, b) = (limb_a as i8, limb_b as i8);
    assert_eq!(cmp(&[limb_a], &[limb_b]), a.cmp(&b));
    assert_eq!(le(&[limb_a], &[limb_b]), a <= b);
    assert_eq!(
      checked_add(&[limb_a], &[limb_b]),
      a.checked_add(b).map(|sum| [sum as u8])
    );
    assert_eq!(
      checked_sub(&[limb_a], &[limb_b]),
      a.checked_sub(b).map(|difference| [difference as u8])
    );
    if let Some(product) = a.checked_mul(b) {
      assert_eq!(mul(&[limb_a], &[limb_b]), [product as u8]);
    }
    // As for `/` and `%`, a zero divisor and `MIN / -1` are excluded.
    if b != 0 && (a, b) != (i8::MIN, -1) {
      assert_eq!(
        div_rem(&[limb_a], &[limb_b]),
        ([(a / b) as u8], [(a % b) as u8]),
        "{a} / {b}"
      );
    }
    if b > 0 {
      for mode in bigint::MODES {
        let (rounded, remainder, away) = crate::int::i8::round_div(a, b, mode);
        assert_eq!(
          round_div(&[limb_a], &[limb_b], mode),
          ([rounded as u8], [remainder as u8], away),
          "{a} / {b}, {mode:?}"
        );
      }
    }
  }
  for a in (i8::MIN..=i8::MAX).step_by(value_step) {
    let limbs_a = [a as u8];
    assert_eq!(is_negative(&limbs_a), a < 0);
    assert_eq!(is_positive(&limbs_a), a > 0);
    assert_eq!(is_zero(&limbs_a), a == 0);
    assert_eq!(unsigned_abs(&limbs_a), [a.unsigned_abs()]);
    if a != i8::MIN {
      assert_eq!(neg(&limbs_a), [(-a) as u8]);
      assert_eq!(abs(&limbs_a), [a.unsigned_abs()]);
    }
    for exp in 0..=2 {
      let power = 10i8.pow(exp);
      let product = a.checked_mul(power);
      assert_eq!(
        checked_mul_pow10(&limbs_a, exp),
        product.map(|product| [product as u8])
      );
      if let Some(product) = product {
        assert_eq!(mul_pow10(&limbs_a, exp), [product as u8]);
      }
      // As with `/` and `%`, the quotient and the remainder have the sign of `a`.
      assert_eq!(
        div_rem_pow10(&limbs_a, exp),
        ([(a / power) as u8], [(a % power) as u8])
      );
    }
    // Every width from one bit to the limb.
    for bits in 1..=8 {
      let (min, max) = (-(1i16 << (bits - 1)), (1i16 << (bits - 1)) - 1);
      assert_eq!(
        fits_bits(&limbs_a, bits),
        (min..=max).contains(&i16::from(a)),
        "{a} in {bits} bits"
      );
      // The two's complement integer in the low `bits` bits.
      let low = i16::from(a as u8) & ((1 << bits) - 1);
      let extended = if low > max { low - (1 << bits) } else { low };
      assert_eq!(
        sign_extend(&limbs_a, bits),
        [extended as u8],
        "{a} from {bits} bits"
      );
    }
    let mut buf = [0xff; DIGIT_BUF_LEN];
    let start = write_digits(&limbs_a, &mut buf);
    assert_eq!(&buf[start..], a.unsigned_abs().to_string().as_bytes());
    if a >= 0 {
      assert_eq!(push_digits(&[0], &buf, start, DIGIT_BUF_LEN), limbs_a);
    }
  }
  for bits in 1..=8 {
    assert_eq!(max_of_bits::<1>(bits), [((1i16 << (bits - 1)) - 1) as u8]);
    assert_eq!(min_of_bits::<1>(bits), [(-(1i16 << (bits - 1))) as u8]);
  }
}
