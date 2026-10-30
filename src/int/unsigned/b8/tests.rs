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
/// layer for `u8`, or under Miri, which runs them slowly, every 61st pair and every 7th value, with
/// the limits and the values next to them, which a stride may skip, and their pairs.
#[test]
fn every_one_limb_value_matches_u8() {
  let (pair_step, value_step) = if cfg!(miri) { (61, 7) } else { (1, 1) };
  let ends = [0, 1, u8::MAX - 1, u8::MAX];
  let end_pairs = ends
    .iter()
    .flat_map(|&a| ends.iter().map(move |&b| u16::from_le_bytes([a, b])));
  for pair in bigint::with_ends((0..=u16::MAX).step_by(pair_step), end_pairs) {
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
  for a in bigint::with_ends((0..=u8::MAX).step_by(value_step), ends) {
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

/// `value`, below `2^16`, in two limbs, least significant first.
fn two_limbs(value: u32) -> [u8; 2] {
  (value as u16).to_le_bytes()
}

/// `exact` modulo `2^bits`, and whether it is outside the range of `bits` bits.
fn wrapped(exact: i64, bits: u32) -> (u32, bool) {
  let modulus = 1 << bits;
  (
    exact.rem_euclid(modulus) as u32,
    !(0..modulus).contains(&exact),
  )
}

/// Checks the arithmetic of `bits` bits, at most 16, on `a` and `b` against `i64`: in one limb
/// when `bits` is at most 8, and in two limbs.
fn check_arithmetic_bits(a: u32, b: u32, bits: u32) {
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
fn check_negation_bits(a: u32, bits: u32) {
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

/// The values of `bits` bits, at most 16, that a stride may skip: the limits and the values next
/// to them.
fn ends_of(bits: u32) -> Vec<u32> {
  let max: u32 = (1 << bits) - 1;
  [0, 1, max.saturating_sub(1), max]
    .into_iter()
    .filter(|&value| value <= max)
    .collect()
}

/// Every pair of values of each width from zero to 8 bits, in one and in two limbs, against `i64`,
/// or under Miri every 61st pair and the pairs of the ends.
#[test]
fn arithmetic_of_every_pair_up_to_8_bits() {
  let step = if cfg!(miri) { 61 } else { 1 };
  for bits in 0..=8 {
    let count = 1 << bits;
    for a in 0..count {
      check_negation_bits(a, bits);
    }
    let ends = ends_of(bits);
    let end_pairs = ends
      .iter()
      .flat_map(|&a| ends.iter().map(move |&b| a * count + b));
    for pair in bigint::with_ends((0..count * count).step_by(step), end_pairs) {
      check_arithmetic_bits(pair / count, pair % count, bits);
    }
  }
}

/// Every value of each width from 9 to 16 bits, in two limbs, against the structured operands of
/// that width, against `i64`, or under Miri every 61st value and the ends at 9 and 16 bits.
#[test]
fn arithmetic_of_9_to_16_bits_against_structured_operands() {
  let (widths, step) = if cfg!(miri) {
    (vec![9, 16], 61)
  } else {
    ((9..=16).collect(), 1)
  };
  for bits in widths {
    let operands = bigint::structured_patterns(bits);
    for a in bigint::with_ends((0..1 << bits).step_by(step), ends_of(bits)) {
      check_negation_bits(a, bits);
      for &b in &operands {
        check_arithmetic_bits(a, b as u32, bits);
      }
    }
  }
}

/// `is_one` of every value of one and of two limbs, or under Miri of every 7th and of the ends.
#[test]
fn one_of_every_value() {
  let step = if cfg!(miri) { 7 } else { 1 };
  let ends = ends_of(16).into_iter().map(|value| value as u16);
  for value in bigint::with_ends((0..=u16::MAX).step_by(step), ends) {
    assert_eq!(is_one(&value.to_le_bytes()), value == 1, "{value}");
    if let Ok(limb) = u8::try_from(value) {
      assert_eq!(is_one(&[limb]), value == 1, "{value}");
    }
  }
  assert!(!is_one::<0>(&[]));
}
