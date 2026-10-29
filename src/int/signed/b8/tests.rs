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

/// `value`, from `-2^15` to `2^15 - 1`, in two limbs of two's complement, least significant first.
fn two_limbs(value: i32) -> [u8; 2] {
  (value as i16).to_le_bytes()
}

/// `exact` wrapped to the range of `bits` bits, and whether it is outside that range.
fn wrapped(exact: i64, bits: u32) -> (i32, bool) {
  let (min, max) = match bits {
    0 => (0, 0),
    _ => (-(1 << (bits - 1)), (1 << (bits - 1)) - 1),
  };
  let result = (exact - min).rem_euclid(1 << bits) + min;
  (result as i32, !(min..=max).contains(&exact))
}

/// Checks the arithmetic of `bits` bits, at most 16, on `a` and `b` against `i64`: in one limb
/// when `bits` is at most 8, and in two limbs.
fn check_arithmetic_bits(a: i32, b: i32, bits: u32) {
  type One = fn(&[u8; 1], &[u8; 1], u32) -> ([u8; 1], bool);
  type Two = fn(&[u8; 2], &[u8; 2], u32) -> ([u8; 2], bool);
  let (wide_a, wide_b) = (i64::from(a), i64::from(b));
  let operations: [(i64, char, One, Two); 3] = [
    (
      wide_a + wide_b,
      '+',
      overflowing_add_bits,
      overflowing_add_bits,
    ),
    (
      wide_a - wide_b,
      '-',
      overflowing_sub_bits,
      overflowing_sub_bits,
    ),
    (
      wide_a * wide_b,
      '*',
      overflowing_mul_bits,
      overflowing_mul_bits,
    ),
  ];
  for (exact, op, one, two) in operations {
    let (result, overflow) = wrapped(exact, bits);
    if bits <= 8 {
      assert_eq!(
        one(&[a as u8], &[b as u8], bits),
        ([result as u8], overflow),
        "{a} {op} {b} in {bits} bits"
      );
    }
    assert_eq!(
      two(&two_limbs(a), &two_limbs(b), bits),
      (two_limbs(result), overflow),
      "{a} {op} {b} in {bits} bits"
    );
  }
}

/// Checks the negation of `bits` bits, at most 16, of `a` against `i64`, as
/// [`check_arithmetic_bits`] does.
fn check_negation_bits(a: i32, bits: u32) {
  let (result, overflow) = wrapped(-i64::from(a), bits);
  if bits <= 8 {
    assert_eq!(
      overflowing_neg_bits(&[a as u8], bits),
      ([result as u8], overflow),
      "-{a} in {bits} bits"
    );
  }
  assert_eq!(
    overflowing_neg_bits(&two_limbs(a), bits),
    (two_limbs(result), overflow),
    "-{a} in {bits} bits"
  );
}

/// The values of `bits` bits, at most 16, in order.
fn values_of(bits: u32) -> core::ops::RangeInclusive<i32> {
  match bits {
    0 => 0..=0,
    _ => -(1 << (bits - 1))..=(1 << (bits - 1)) - 1,
  }
}

/// Every pair of values of each width from zero to 8 bits, in one and in two limbs, against `i64`,
/// or under Miri every 61st pair.
#[test]
fn arithmetic_of_every_pair_up_to_8_bits() {
  let step = if cfg!(miri) { 61 } else { 1 };
  for bits in 0..=8 {
    for a in values_of(bits) {
      check_negation_bits(a, bits);
    }
    let pairs = values_of(bits).flat_map(|a| values_of(bits).map(move |b| (a, b)));
    for (a, b) in pairs.step_by(step) {
      check_arithmetic_bits(a, b, bits);
    }
  }
}

/// Every value of each width from 9 to 16 bits, in two limbs, against the structured operands of
/// that width read in two's complement, against `i64`, or under Miri every 61st value at 9 and 16
/// bits.
#[test]
fn arithmetic_of_9_to_16_bits_against_structured_operands() {
  let (widths, step) = if cfg!(miri) {
    (vec![9, 16], 61)
  } else {
    ((9..=16).collect(), 1)
  };
  for bits in widths {
    let operands: Vec<i32> = bigint::structured_patterns(bits)
      .into_iter()
      .map(|pattern| bigint::to_signed(pattern, bits) as i32)
      .collect();
    for a in values_of(bits).step_by(step) {
      check_negation_bits(a, bits);
      for &b in &operands {
        check_arithmetic_bits(a, b, bits);
      }
    }
  }
}
