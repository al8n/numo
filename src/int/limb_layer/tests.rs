//! The unit tests that every instance of `limb_layer!` shares.
//!
//! `limb_layer_tests!` writes them once, in terms of the instance's `Limb` and its number of bits
//! `B`, into the `tests` module of each instance, so that every test keeps the path of its
//! instance. The inputs depend on the limb type only through `B`: the `N`, the seeds and the order
//! of the random draws are the same for every instance, except that a random limb of more than 64
//! bits takes two draws. Each instance adds the tests that compare it with native integers, whose
//! widths differ from one limb type to another.
//!
//! The instances for the limb types other than `u64` share those tests as well: `other_limb_tests!`
//! writes their helpers, and `native_oracle!` writes each test against a native integer.

/// Defines, in the `tests` module of an instance of `limb_layer!`, the tests that every instance
/// shares, their helpers, and the imports that they and the instance's own tests use:
/// `limb_layer_tests!(unsigned)` for a lower layer, and `limb_layer_tests!(signed unsigned: m)`
/// for an upper layer over the lower layer `m`.
macro_rules! limb_layer_tests {
  (unsigned) => {
    use core::cmp::Ordering;
    use std::{string::ToString, vec, vec::Vec};

    use num_bigint::BigInt;

    use super::*;
    use crate::{bigint, int::DIGIT_BUF_LEN};

    /// The number of bits of a limb.
    const B: u32 = Limb::BITS;

    /// The limb with only its top bit set.
    const HIGH: Limb = 1 << (B - 1);

    /// The number of bits of the widest values that the tests of digits write: 4096, or under Miri,
    /// which divides many narrow limbs slowly, 512 for limbs narrower than 32 bits.
    const WIDEST: u32 = if B < 32 && cfg!(miri) { 512 } else { 4096 };

    /// `2^(B * k)`.
    fn boundary<const N: usize>(k: usize) -> [Limb; N] {
      let mut value = [0; N];
      value[k] = 1;
      value
    }

    /// `2^(B * k) - 1`: the `k` lowest limbs all ones.
    fn ones_below<const N: usize>(k: usize) -> [Limb; N] {
      let mut value = [0; N];
      value[..k].fill(Limb::MAX);
      value
    }

    fn carries_and_borrows<const N: usize>() {
      let one = boundary::<N>(0);
      for k in 1..N {
        // A carry and a borrow across the `k` lowest limbs.
        assert_eq!(
          checked_add(&ones_below::<N>(k), &one),
          Some(boundary::<N>(k))
        );
        assert_eq!(
          checked_sub(&boundary::<N>(k), &one),
          Some(ones_below::<N>(k))
        );
        // And across one limb, from limb `k - 1` into limb `k`.
        let mut low = [0; N];
        low[k - 1] = Limb::MAX;
        let mut high = [0; N];
        high[k - 1] = HIGH;
        let mut carried = [0; N];
        carried[k - 1] = HIGH - 1;
        carried[k] = 1;
        assert_eq!(checked_add(&low, &high), Some(carried));
        assert_eq!(checked_sub(&carried, &high), Some(low));
      }
      // Out of the top limb.
      assert_eq!(overflowing_add(&[Limb::MAX; N], &one), ([0; N], true));
      assert_eq!(checked_add(&[Limb::MAX; N], &one), None);
      assert_eq!(checked_add(&boundary::<N>(N - 1), &[Limb::MAX; N]), None);
      assert_eq!(overflowing_sub(&[0; N], &one), ([Limb::MAX; N], true));
      assert_eq!(checked_sub(&[0; N], &one), None);
      assert_eq!(
        checked_sub(&ones_below::<N>(N - 1), &boundary::<N>(N - 1)),
        None
      );
      assert_eq!(checked_sub(&[Limb::MAX; N], &[Limb::MAX; N]), Some([0; N]));
    }

    #[test]
    fn carries_and_borrows_cross_every_limb() {
      carries_and_borrows::<3>();
      carries_and_borrows::<4>();
      carries_and_borrows::<5>();
      carries_and_borrows::<6>();
      carries_and_borrows::<7>();
      carries_and_borrows::<8>();
    }

    fn numeric_order<const N: usize>() {
      // The array order compares limb 0 first and would put 1 above `2^B`.
      let one = boundary::<N>(0);
      let two_pow_b = boundary::<N>(1);
      assert!(one > two_pow_b);
      assert_eq!(cmp(&one, &two_pow_b), Ordering::Less);
      assert!(le(&one, &two_pow_b) && !le(&two_pow_b, &one));
      assert_eq!(
        cmp(&ones_below::<N>(N - 1), &boundary::<N>(N - 1)),
        Ordering::Less
      );
      assert_eq!(cmp(&[Limb::MAX; N], &[Limb::MAX; N]), Ordering::Equal);
    }

    #[test]
    fn comparison_is_numeric() {
      numeric_order::<3>();
      numeric_order::<4>();
      numeric_order::<5>();
      numeric_order::<6>();
      numeric_order::<7>();
      numeric_order::<8>();
    }

    fn maximal_products<const N: usize>() {
      let max = bigint::from_limbs(&[Limb::MAX; N], false);
      let root = max.sqrt();
      let operands = [
        BigInt::ZERO,
        BigInt::from(1u8),
        root.clone(),
        &root + 1u8,
        max.clone(),
        BigInt::from(Limb::MAX),
        bigint::from_limbs(&ones_below::<N>(N - 1), false),
        bigint::from_limbs(&boundary::<N>(N - 1), false),
      ];
      for a in &operands {
        for b in &operands {
          let product = a * b;
          let (Some(limbs_a), Some(limbs_b)) = (
            bigint::to_limbs_of::<Limb, N>(a, false),
            bigint::to_limbs_of(b, false),
          ) else {
            panic!("operands fit");
          };
          match bigint::to_limbs_of::<Limb, N>(&product, false) {
            Some(expected) => assert_eq!(mul(&limbs_a, &limbs_b), expected, "{a} * {b}"),
            // The maximal operands whose product fits are around the square root.
            None => assert!(a > &root || b > &root),
          }
        }
      }
      // The largest square is `root^2`.
      let square = bigint::to_limbs_of::<Limb, N>(&(&root * &root), false).expect("fits");
      let root_limbs = bigint::to_limbs_of::<Limb, N>(&root, false).expect("fits");
      assert_eq!(mul(&root_limbs, &root_limbs), square);
      assert!(bigint::to_limbs_of::<Limb, N>(&((&root + 1u8) * (&root + 1u8)), false).is_none());
    }

    #[test]
    fn multiplication_with_maximal_operands() {
      maximal_products::<3>();
      maximal_products::<4>();
      maximal_products::<5>();
      maximal_products::<6>();
      maximal_products::<7>();
      maximal_products::<8>();
    }

    /// Checks `overflowing_mul` at the full width of `N` limbs against `num-bigint`, with the
    /// operands of `maximal_products` and seeded random ones.
    fn overflowing_products<const N: usize>() {
      let max = bigint::from_limbs(&[Limb::MAX; N], false);
      let root = max.sqrt();
      let mut operands = vec![
        BigInt::ZERO,
        BigInt::from(1u8),
        root.clone(),
        &root + 1u8,
        max.clone(),
        BigInt::from(Limb::MAX),
        bigint::from_limbs(&ones_below::<N>(N - 1), false),
        bigint::from_limbs(&boundary::<N>(N - 1), false),
      ];
      let mut rng = Rng(0x6f7e_7f10 ^ N as u64);
      let count = if cfg!(miri) { 4 } else { 64 };
      operands.extend((0..count).map(|_| bigint::from_limbs(&rng.limbs::<N>(), false)));
      let modulus = &max + 1u8;
      for a in &operands {
        for b in &operands {
          let product = a * b;
          let (Some(limbs_a), Some(limbs_b)) = (
            bigint::to_limbs_of::<Limb, N>(a, false),
            bigint::to_limbs_of(b, false),
          ) else {
            panic!("operands fit");
          };
          let (wrapped, overflow) = overflowing_mul(&limbs_a, &limbs_b);
          assert_eq!(
            bigint::from_limbs(&wrapped, false),
            &product % &modulus,
            "{a} * {b}"
          );
          assert_eq!(overflow, product > max, "{a} * {b}");
        }
      }
    }

    #[test]
    #[cfg_attr(
      all(miri, target_pointer_width = "32"),
      ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
    )]
    fn overflowing_mul_matches_big_integers() {
      overflowing_products::<1>();
      overflowing_products::<2>();
      overflowing_products::<3>();
      overflowing_products::<4>();
      overflowing_products::<5>();
      overflowing_products::<6>();
      overflowing_products::<7>();
      overflowing_products::<8>();
    }

    /// `2^(B * i) * 2^(B * j)` carries out of no limb, so only the partial products past the top
    /// limb report its overflow: the product is `2^(B * (i + j))`, which overflows exactly when
    /// `i + j >= N`, and then wraps to zero.
    fn boundary_products<const N: usize>() {
      for i in 0..N {
        for j in 0..N {
          let expected = if i + j < N {
            (boundary::<N>(i + j), false)
          } else {
            ([0; N], true)
          };
          assert_eq!(
            overflowing_mul(&boundary::<N>(i), &boundary::<N>(j)),
            expected,
            "2^({B} * {i}) * 2^({B} * {j})"
          );
        }
      }
    }

    #[test]
    fn boundary_products_overflow_past_the_top_limb() {
      boundary_products::<1>();
      boundary_products::<2>();
      boundary_products::<3>();
      boundary_products::<4>();
      boundary_products::<5>();
      boundary_products::<6>();
      boundary_products::<7>();
      boundary_products::<8>();
      // No limbs hold only zero, whose square does not overflow.
      assert_eq!(overflowing_mul::<0>(&[], &[]), ([], false));
    }

    fn division_by_every_power<const N: usize>() {
      let max = bigint::from_limbs(&[Limb::MAX; N], false);
      for value in [
        max.clone(),
        bigint::pow10(bigint::max_exp(&max)),
        BigInt::from(1u8),
        BigInt::ZERO,
      ] {
        let limbs = bigint::to_limbs_of::<Limb, N>(&value, false).expect("fits");
        // Beyond the largest power of ten that fits, the quotient is zero.
        for exp in 0..=bigint::max_exp(&max) + 2 {
          let power = bigint::pow10(exp);
          let (quotient, remainder) = div_rem_pow10(&limbs, exp);
          assert_eq!(bigint::from_limbs(&quotient, false), &value / &power);
          assert_eq!(bigint::from_limbs(&remainder, false), &value % &power);
        }
      }
    }

    #[test]
    fn div_rem_pow10_for_every_exponent() {
      division_by_every_power::<3>();
      division_by_every_power::<4>();
      division_by_every_power::<5>();
      division_by_every_power::<6>();
      division_by_every_power::<7>();
      division_by_every_power::<8>();
    }

    fn digits_of_the_limits<const N: usize>() {
      for value in [[0; N], [Limb::MAX; N], boundary::<N>(N - 1)] {
        let expected = bigint::from_limbs(&value, false).to_string();
        // `write_digits` requires at most 512 bits, whose digits fit `DIGIT_BUF_LEN` bytes, and
        // `DIGITS_PER_LIMB * N` bytes hold the digits of any `N` limbs.
        if B as usize * N <= 512 {
          let mut buf = [0xff; DIGIT_BUF_LEN];
          let start = write_digits(&value, &mut buf);
          assert_eq!(&buf[start..], expected.as_bytes());
        }
        let mut buf = vec![0xff; DIGITS_PER_LIMB * N];
        let start = write_digits_into(&value, &mut buf);
        assert_eq!(&buf[start..], expected.as_bytes());

        // The digits read back, also with separators among them.
        let digits = expected.as_bytes();
        assert_eq!(push_digits(&[0; N], digits, 0, digits.len()), value);
        let separated: Vec<u8> = digits
          .iter()
          .flat_map(|&digit| [b'_', digit, b'.'])
          .collect();
        assert_eq!(push_digits(&[0; N], &separated, 0, separated.len()), value);
      }
    }

    #[test]
    fn digits_of_zero_and_max() {
      digits_of_the_limits::<3>();
      digits_of_the_limits::<4>();
      digits_of_the_limits::<5>();
      digits_of_the_limits::<6>();
      digits_of_the_limits::<7>();
      digits_of_the_limits::<8>();
      let mut buf = [0; DIGIT_BUF_LEN];
      assert_eq!(
        write_digits(&[Limb::MAX; 512 / B as usize], &mut buf),
        0,
        "u512::MAX fills the buffer"
      );
    }

    #[test]
    fn resizing() {
      let value = [1, 2, 3];
      assert_eq!(resize::<3, 5>(&value), [1, 2, 3, 0, 0]);
      assert_eq!(checked_resize::<3, 5>(&value), Some([1, 2, 3, 0, 0]));
      assert_eq!(resize::<3, 2>(&value), [1, 2]);
      assert_eq!(checked_resize::<3, 2>(&value), None);
      assert_eq!(checked_resize::<3, 2>(&[1, 2, 0]), Some([1, 2]));
      assert_eq!(checked_resize::<5, 3>(&[1, 2, 3, 0, 0]), Some(value));
      assert_eq!(checked_resize::<5, 3>(&[1, 2, 3, 0, 1]), None);
      assert_eq!(
        checked_resize::<3, 3>(&[Limb::MAX; 3]),
        Some([Limb::MAX; 3])
      );
      // Unsigned limbs extend with zeros, whatever the top bit.
      assert_eq!(resize::<2, 3>(&[0, HIGH]), [0, HIGH, 0]);
    }

    /// A deterministic xorshift generator, so that a failure replays from the seed.
    struct Rng(u64);

    impl Rng {
      fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
      }

      /// A random limb: one draw, or two for a limb of more than 64 bits, the high half first.
      fn limb(&mut self) -> Limb {
        if B > 64 {
          ((self.next() as u128) << 64 | self.next() as u128) as Limb
        } else {
          self.next() as Limb
        }
      }

      /// A nonzero value with a random number of significant limbs, each random, zero or all
      /// ones, and a top limb of random bit length, which gives every normalization shift.
      fn limbs<const N: usize>(&mut self) -> [Limb; N] {
        let mut value = [0; N];
        let count = (self.next() % N as u64) as usize + 1;
        for limb in &mut value[..count] {
          *limb = match self.next() % 4 {
            0 => 0,
            1 => Limb::MAX,
            _ => self.limb(),
          };
        }
        value[count - 1] = (value[count - 1] >> (self.next() % u64::from(B))).max(1);
        value
      }
    }

    /// Checks `div_rem(a, b)` against `num-bigint`.
    fn check_division<const N: usize>(a: &[Limb; N], b: &[Limb; N]) {
      let (quotient, remainder) = div_rem(a, b);
      let (big_a, big_b) = (bigint::from_limbs(a, false), bigint::from_limbs(b, false));
      assert_eq!(
        bigint::from_limbs(&quotient, false),
        &big_a / &big_b,
        "{a:x?} / {b:x?}"
      );
      assert_eq!(
        bigint::from_limbs(&remainder, false),
        &big_a % &big_b,
        "{a:x?} % {b:x?}"
      );
    }

    fn division_against_big_integers<const N: usize>() {
      // Structured operands: limb boundaries, all-ones runs, single limbs, and a top limb with
      // the high bit set or with only the low bit.
      let mut operands = vec![[0; N], boundary::<N>(0), [Limb::MAX; N]];
      for k in 1..N {
        operands.extend([boundary::<N>(k), ones_below::<N>(k)]);
        let mut top = ones_below::<N>(k);
        top[k] = HIGH;
        operands.push(top);
      }
      let mut seven = [0; N];
      seven[0] = 7;
      operands.push(seven);
      let operands = bigint::thin(operands);
      for a in &operands {
        for b in operands.iter().filter(|b| !is_zero(b)) {
          check_division(a, b);
        }
      }
      let mut rng = Rng(0x9e37_79b9_7f4a_7c15 ^ N as u64);
      let count = if cfg!(miri) { 8 } else { 3000 };
      for _ in 0..count {
        let b = rng.limbs::<N>();
        let a = match rng.next() % 4 {
          0 => [Limb::MAX; N],
          _ => rng.limbs::<N>(),
        };
        check_division(&a, &b);
      }
    }

    #[test]
    fn division_against_big_integers_for_every_width() {
      division_against_big_integers::<1>();
      division_against_big_integers::<2>();
      division_against_big_integers::<3>();
      division_against_big_integers::<4>();
      division_against_big_integers::<5>();
      division_against_big_integers::<6>();
      division_against_big_integers::<7>();
      division_against_big_integers::<8>();
    }

    /// The add-back step of Algorithm D, each reached exactly once and past the shortcut for a
    /// dividend below the divisor. Both divisors have three significant limbs: with two, the
    /// correction by the next divisor limb is exact and nothing is ever added back.
    #[test]
    fn division_adds_back() {
      const MAX: Limb = Limb::MAX;
      // Two steps with shift 0. The second estimates 8, which passes the test with the second
      // divisor limb, zero, but the third divisor limb makes the subtraction borrow.
      let a = [MAX - 9, 7, 0, 4];
      let b = [MAX, 0, HIGH, 0];
      assert_eq!(div_rem(&a, &b), ([7, 0, 0, 0], [MAX - 2, 0, HIGH, 0]));
      check_division(&a, &b);
      // A divisor as wide as the dividend: one step, with shift `B - 1`, which estimates 2.
      let a = [MAX - 2, 1, 2];
      let b = [MAX, 0, 1];
      assert_eq!(div_rem(&a, &b), ([1, 0, 0], [MAX - 1, 0, 1]));
      check_division(&a, &b);
    }

    /// Steps whose window has the top divisor limb on top, so that the quotient of the window's
    /// top two limbs by that limb is `2^B` or `2^B + 1`, and the estimate starts at `2^B - 1`.
    fn division_with_equal_top_limbs() {
      const MAX: Limb = Limb::MAX;
      // The first step estimates 1 and corrects it to 0, leaving the top limb `HIGH`: in the
      // second step, the quotient of the top two limbs is `2^B`.
      let a = [0, 0, HIGH];
      let b = [MAX, HIGH, 0];
      assert_eq!(div_rem(&a, &b), ([MAX - 1, 0, 0], [MAX - 1, 2, 0]));
      check_division(&a, &b);
      // The same with `HIGH + 5` below the top limb: the quotient is `2^B + 1`.
      let a = [7, HIGH + 5, HIGH];
      assert_eq!(div_rem(&a, &b), ([MAX, 0, 0], [6, 7, 0]));
      check_division(&a, &b);
    }

    /// Divisors whose top limb has 0, 1, `B / 2 - 1`, `B / 2`, `B - 2` and `B - 1` leading
    /// zeros, including the shift of zero, which shifts by nothing.
    #[test]
    fn division_for_every_normalization_shift() {
      for shift in [0, 1, B / 2 - 1, B / 2, B - 2, B - 1] {
        let top = Limb::MAX >> shift;
        let b = [Limb::MAX, 0, top];
        for a in [
          [Limb::MAX; 3],
          b,
          [0, 0, top],
          [1, 0, top],
          [Limb::MAX, Limb::MAX, top],
        ] {
          check_division(&a, &b);
        }
        let mut wide = [Limb::MAX; 8];
        wide[7] = top;
        check_division(&[Limb::MAX; 8], &wide);
        check_division(&[Limb::MAX; 8], &[Limb::MAX, top, 0, 0, 0, 0, 0, 0]);
      }
    }

    fn division_edges<const N: usize>() {
      let one = boundary::<N>(0);
      let max = [Limb::MAX; N];
      assert_eq!(div_rem(&max, &one), (max, [0; N]));
      assert_eq!(div_rem(&max, &max), (one, [0; N]));
      assert_eq!(div_rem(&[0; N], &max), ([0; N], [0; N]));
      // A dividend below the divisor, and equal to it, for a divisor as wide as the dividend.
      let mut divisor = ones_below::<N>(N - 1);
      divisor[N - 1] = 1;
      let mut below = divisor;
      below[0] -= 1;
      assert_eq!(div_rem(&below, &divisor), ([0; N], below));
      assert_eq!(div_rem(&divisor, &divisor), (one, [0; N]));
      // A divisor with leading zero limbs and one significant limb takes the single-limb
      // division.
      let mut seven = [0; N];
      seven[0] = 7;
      check_division(&max, &seven);
      // A divisor as wide as the dividend leaves one quotient limb.
      let mut wide = max;
      wide[N - 1] = 1;
      check_division(&max, &wide);
    }

    #[test]
    fn division_edges_for_every_width() {
      division_edges::<2>();
      division_edges::<3>();
      division_edges::<4>();
      division_edges::<5>();
      division_edges::<6>();
      division_edges::<7>();
      division_edges::<8>();
    }

    /// Checks `round_div(value, divisor, mode)` against rounding by distance.
    fn check_rounding<const N: usize>(value: &[Limb; N], divisor: &[Limb; N]) {
      let (big_value, big_divisor) = (
        bigint::from_limbs(value, false),
        bigint::from_limbs(divisor, false),
      );
      let truncated = &big_value / &big_divisor;
      for mode in bigint::MODES {
        let (rounded, remainder, away) = round_div(value, divisor, mode);
        let expected = bigint::round(&big_value, &big_divisor, mode);
        let rounded = bigint::from_limbs(&rounded, false);
        assert_eq!(rounded, expected, "{value:x?} / {divisor:x?}, {mode:?}");
        assert_eq!(
          bigint::from_limbs(&remainder, false),
          &big_value % &big_divisor
        );
        assert_eq!(away, rounded != truncated);
      }
    }

    fn rounding_division<const N: usize>() {
      let mut rng = Rng(0x5eed_f00d ^ N as u64);
      let count = if cfg!(miri) { 4 } else { 400 };
      for _ in 0..count {
        let divisor = rng.limbs::<N>();
        // Values at and around the multiples of the divisor and the halfway points between
        // them.
        let big_divisor = bigint::from_limbs(&divisor, false);
        let quotient = bigint::from_limbs(&rng.limbs::<N>(), false) >> (B * N as u32 / 2);
        let base = &quotient * &big_divisor;
        let half = &big_divisor / 2u8;
        for offset in [
          BigInt::ZERO,
          BigInt::from(1u8),
          &half - 1u8,
          half.clone(),
          &half + 1u8,
          &big_divisor - 1u8,
        ] {
          if let Some(value) = bigint::to_limbs_of::<Limb, N>(&(&base + offset), false) {
            check_rounding(&value, &divisor);
          }
        }
        check_rounding(&rng.limbs::<N>(), &divisor);
      }
      // Even divisors, whose halfway points are ties, with an even and an odd quotient.
      let mut ten = [0; N];
      ten[0] = 10;
      for value in [5, 15, 25, 4, 6, 14, 16] {
        let mut limbs = [0; N];
        limbs[0] = value;
        check_rounding(&limbs, &ten);
      }
      check_rounding(&[Limb::MAX; N], &boundary::<N>(N - 1));
    }

    #[test]
    fn rounding_division_matches_rounding_by_distance() {
      rounding_division::<1>();
      rounding_division::<2>();
      rounding_division::<3>();
      rounding_division::<5>();
      rounding_division::<8>();
    }

    /// The widths at which the functions of a width are checked: 0, 1, `B - 1`, `B`, `B + 1`,
    /// `B * N - 1` and `B * N`, those that `N` limbs hold.
    fn widths<const N: usize>() -> Vec<u32> {
      let top = B * N as u32;
      let mut widths: Vec<u32> = [0, 1, B - 1, B, B + 1, top.saturating_sub(1), top]
        .into_iter()
        .filter(|&width| width <= top)
        .collect();
      widths.sort_unstable();
      widths.dedup();
      widths
    }

    fn widths_against_big_integers<const N: usize>() {
      // Above 4096 bits, which only limbs of more than 64 bits reach here, the values take no
      // path that narrower ones do not, and the time grows faster than the width.
      if B as usize * N > 4096 {
        return;
      }
      let widths = widths::<N>();
      let patterns = bigint::bit_patterns_of::<Limb, N>(&widths);
      let values: Vec<BigInt> = patterns
        .iter()
        .map(|pattern| bigint::from_limbs(pattern, false))
        .collect();
      for &bits in &widths {
        let max = bigint::max(bits, false);
        assert_eq!(bigint::from_limbs(&max_of_bits::<N>(bits), false), max);
        assert_eq!(
          bigint::to_limbs_of(&max, false),
          Some(max_of_bits::<N>(bits))
        );
        for (pattern, value) in patterns.iter().zip(&values) {
          assert_eq!(
            fits_bits(pattern, bits),
            *value <= max,
            "{pattern:x?} in {bits} bits"
          );
        }
      }
      assert_eq!(max_of_bits::<N>(0), [0; N]);
    }

    #[test]
    #[cfg_attr(
      all(miri, target_pointer_width = "32"),
      ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
    )]
    fn widths_match_big_integers() {
      widths_against_big_integers::<0>();
      widths_against_big_integers::<1>();
      widths_against_big_integers::<2>();
      widths_against_big_integers::<3>();
      widths_against_big_integers::<4>();
      widths_against_big_integers::<8>();
      widths_against_big_integers::<16>();
      widths_against_big_integers::<64>();
    }

    /// Checks the arithmetic of each width of `N` limbs against `num-bigint`: the result wrapped
    /// to the width, the overflow flag, and that the result has the width.
    fn arithmetic_bits_against_big_integers<const N: usize>() {
      let mut rng = Rng(0xa817_5eed ^ N as u64);
      for bits in bigint::arithmetic_widths(B, N) {
        let (min, max) = (bigint::min(bits, false), bigint::max(bits, false));
        // No limbs hold only zero, from which nothing random is drawn.
        let samples = bigint::arithmetic_samples(bits, B, &min, &max, || match N {
          0 => BigInt::ZERO,
          _ => bigint::from_limbs(&rng.limbs::<N>(), false),
        });
        let limbs: Vec<[Limb; N]> = samples
          .iter()
          .map(|value| bigint::to_limbs_of(value, false).expect("in range"))
          .collect();
        // `exact` is `lhs op rhs`, and `computed` its result in the layer.
        let check = |exact: BigInt, computed: ([Limb; N], bool), lhs: &BigInt, op, rhs: &BigInt| {
          let (result, overflow) = computed;
          assert_eq!(
            bigint::from_limbs(&result, false),
            bigint::wrap(&exact, bits, &min),
            "{lhs} {op} {rhs} in {bits} bits"
          );
          assert_eq!(
            overflow,
            exact < min || max < exact,
            "{lhs} {op} {rhs} in {bits} bits"
          );
          assert!(fits_bits(&result, bits), "{lhs} {op} {rhs} in {bits} bits");
        };
        let (zero, one) = (BigInt::ZERO, BigInt::from(1u8));
        for (a, limbs_a) in samples.iter().zip(&limbs) {
          assert_eq!(is_one(limbs_a), *a == one, "{a}");
          check(-a, overflowing_neg_bits(limbs_a, bits), &zero, '-', a);
          for (b, limbs_b) in samples.iter().zip(&limbs) {
            for (exact, op, computed) in [
              (a + b, '+', overflowing_add_bits(limbs_a, limbs_b, bits)),
              (a - b, '-', overflowing_sub_bits(limbs_a, limbs_b, bits)),
              (a * b, '*', overflowing_mul_bits(limbs_a, limbs_b, bits)),
            ] {
              check(exact, computed, a, op, b);
            }
          }
        }
      }
    }

    #[test]
    #[cfg_attr(
      all(miri, target_pointer_width = "32"),
      ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
    )]
    fn arithmetic_bits_match_big_integers() {
      arithmetic_bits_against_big_integers::<0>();
      arithmetic_bits_against_big_integers::<1>();
      arithmetic_bits_against_big_integers::<2>();
      arithmetic_bits_against_big_integers::<3>();
    }

    fn digits_into_a_slice<const N: usize>() {
      // Above 4096 bits, which only limbs of more than 64 bits reach here, the values take no
      // path that narrower ones do not, and the time grows faster than the width.
      if B as usize * N > 4096 {
        return;
      }
      let mut patterns = vec![[0; N], [Limb::MAX; N]];
      // Each limb boundary, or under Miri, which divides many limbs slowly, only the top one.
      let boundaries = if cfg!(miri) { N - 1..N } else { 0..N };
      for k in boundaries {
        patterns.extend([boundary::<N>(k), ones_below::<N>(k + 1)]);
      }
      for pattern in patterns {
        let expected = bigint::from_limbs(&pattern, false).to_string();
        // `(MAX_STEP + 1) * N` bytes hold every value, and so do exactly as many bytes as there
        // are digits.
        let mut buf = vec![0xff; (MAX_STEP as usize + 1) * N];
        let start = write_digits_into(&pattern, &mut buf);
        assert_eq!(&buf[start..], expected.as_bytes());
        let mut exact = vec![0xff; expected.len()];
        assert_eq!(write_digits_into(&pattern, &mut exact), 0);
        assert_eq!(exact, expected.as_bytes());
      }
    }

    #[test]
    #[cfg_attr(
      all(miri, target_pointer_width = "32"),
      ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
    )]
    fn digits_into_a_slice_for_every_width() {
      digits_into_a_slice::<1>();
      digits_into_a_slice::<2>();
      digits_into_a_slice::<3>();
      digits_into_a_slice::<4>();
      digits_into_a_slice::<8>();
      digits_into_a_slice::<16>();
      digits_into_a_slice::<64>();
      // `2^4096 - 1` has 1234 digits, and `2^512 - 1` has 155.
      let widest = [Limb::MAX; (WIDEST / B) as usize];
      let expected = bigint::from_limbs(&widest, false).to_string();
      assert_eq!(expected.len(), if WIDEST == 4096 { 1234 } else { 155 });
      let mut buf = vec![0; expected.len()];
      assert_eq!(write_digits_into(&widest, &mut buf), 0);
      assert_eq!(buf, expected.as_bytes());
    }

    /// A zero divisor fails as native division does, in the single-limb division that it takes.
    #[test]
    #[should_panic(expected = "attempt to divide by zero")]
    fn division_by_zero_panics() {
      let _ = div_rem(&[1, 2, 3], &[0; 3]);
    }

    /// Checks the functions of a width that divide, round a quotient or relate two values, on `a`
    /// and `b` of `bits` bits, in `N` limbs `x` and `y`, against `num-bigint`: each result is the
    /// exact one, or that wrapped to the width with its overflow flag, and has the width. A zero
    /// divisor gives `None`.
    fn check_division_bits<const N: usize>(
      a: &BigInt,
      x: &[Limb; N],
      b: &BigInt,
      y: &[Limb; N],
      bits: u32,
    ) {
      let (min, max) = (bigint::min(bits, false), bigint::max(bits, false));
      let big = |limbs: &[Limb; N]| bigint::from_limbs(limbs, false);
      let zero = BigInt::ZERO;
      assert_eq!(
        big(&midpoint(x, y)),
        (a + b) / 2u8,
        "midpoint of {a} and {b}"
      );
      let difference = if a < b { b - a } else { a - b };
      assert_eq!(big(&abs_diff(x, y)), difference, "|{a} - {b}|");
      let multiple = if *b == zero {
        *a == zero
      } else {
        a % b == zero
      };
      assert_eq!(is_multiple_of(x, y), multiple, "{a} % {b}");
      if *b == zero {
        assert_eq!(div_rem_bits(x, y, bits), None);
        assert_eq!(div_rem_euclid_bits(x, y, bits), None);
        assert_eq!(div_floor_bits(x, y, bits), None);
        assert_eq!(div_ceil_bits(x, y, bits), None);
        assert_eq!(next_multiple_of_bits(x, y, bits), None);
        assert_eq!(checked_div_exact_bits(x, y, bits), None);
        return;
      }
      let (quotient, remainder) = (a / b, a % b);
      let division = div_rem_bits(x, y, bits).expect("a nonzero divisor");
      assert_eq!(
        (big(&division.0), big(&division.1), division.2),
        (quotient.clone(), remainder.clone(), false),
        "{a} / {b} in {bits} bits"
      );
      assert_eq!(div_rem_euclid_bits(x, y, bits), Some(division));
      let rounded =
        |result: Option<([Limb; N], bool)>| result.map(|(value, flag)| (big(&value), flag));
      assert_eq!(
        rounded(div_floor_bits(x, y, bits)),
        Some((bigint::div_floor(a, b), false)),
        "floor({a} / {b})"
      );
      assert_eq!(
        rounded(div_ceil_bits(x, y, bits)),
        Some((bigint::div_ceil(a, b), false)),
        "ceil({a} / {b})"
      );
      let next = bigint::next_multiple_of(a, b);
      let (value, overflow) = next_multiple_of_bits(x, y, bits).expect("a nonzero divisor");
      assert_eq!(
        (big(&value), overflow),
        (bigint::wrap(&next, bits, &min), next > max),
        "the multiple of {b} from {a} in {bits} bits"
      );
      assert!(fits_bits(&value, bits), "{value:x?} in {bits} bits");
      assert_eq!(
        checked_div_exact_bits(x, y, bits).map(|exact| big(&exact)),
        (remainder == zero).then_some(quotient),
        "{a} / {b} exactly"
      );
    }

    /// The bases at which the logarithms of `N` limbs are checked: those below 2, which have none,
    /// powers of two, ten, bases of one limb and of two, and the largest value.
    fn bases<const N: usize>() -> Vec<(BigInt, [Limb; N])> {
      let one = BigInt::from(1u8);
      let limb = &one << B;
      let candidates = [
        BigInt::ZERO,
        one.clone(),
        BigInt::from(2u8),
        BigInt::from(3u8),
        BigInt::from(7u8),
        BigInt::from(10u8),
        BigInt::from(16u8),
        BigInt::from(259u16),
        BigInt::from(1000u16),
        &limb - 1u8,
        limb.clone(),
        &limb + 3u8,
        bigint::from_limbs(&[Limb::MAX; N], false),
      ];
      candidates
        .into_iter()
        .filter_map(|base| bigint::to_limbs_of(&base, false).map(|limbs| (base, limbs)))
        .collect()
    }

    /// Checks the powers, roots and logarithms of `a`, of `bits` bits, in `N` limbs `x`, and its
    /// next power of two, against `num-bigint`.
    fn check_powers_bits<const N: usize>(a: &BigInt, x: &[Limb; N], bits: u32) {
      let (min, max) = (bigint::min(bits, false), bigint::max(bits, false));
      let big = |limbs: &[Limb; N]| bigint::from_limbs(limbs, false);
      let exps = [
        0,
        1,
        2,
        3,
        bits.saturating_sub(1),
        bits,
        bits + 1,
        2 * bits + 1,
      ];
      for exp in exps.into_iter().chain([u32::MAX - 1, u32::MAX]) {
        let (value, overflow) = overflowing_pow_bits(x, exp, bits);
        assert_eq!(
          (big(&value), overflow),
          bigint::pow_bits(a, exp, bits, &min, &max),
          "{a}^{exp} in {bits} bits"
        );
        assert!(fits_bits(&value, bits), "{a}^{exp} in {bits} bits");
      }
      assert_eq!(big(&isqrt(x)), a.sqrt(), "the square root of {a}");
      assert_eq!(checked_isqrt(x), Some(isqrt(x)));
      assert_eq!(checked_nth_root(x, 0), None);
      for n in [1, 2, 3, 4, 7, bits, bits + 1, u32::MAX] {
        if n > 0 {
          assert_eq!(big(&nth_root(x, n)), a.nth_root(n), "the root {n} of {a}");
          assert_eq!(checked_nth_root(x, n), Some(nth_root(x, n)));
        }
      }
      let log2 = (*a > BigInt::ZERO).then(|| u32::try_from(a.bits() - 1).expect("a width"));
      assert_eq!(checked_ilog2(x), log2, "log2({a})");
      assert_eq!(
        checked_ilog10(x),
        bigint::ilog(a, &BigInt::from(10u8)),
        "log10({a})"
      );
      for (base, limbs) in bases::<N>() {
        assert_eq!(
          checked_ilog(x, &limbs),
          bigint::ilog(a, &base),
          "log({a}) to the base {base}"
        );
      }
      let power = bigint::next_power_of_two(a);
      let (value, overflow) = next_power_of_two_bits(x, bits);
      assert_eq!(
        (big(&value), overflow),
        (bigint::wrap(&power, bits, &min), power > max),
        "the power of two from {a} in {bits} bits"
      );
    }

    /// Checks the division, the powers, the roots and the logarithms of each width of `N` limbs
    /// against `num-bigint`, at the values of `bigint::arithmetic_samples` and their pairs.
    fn division_and_powers_against_big_integers<const N: usize>() {
      let mut rng = Rng(0xd1f1_5eed ^ N as u64);
      for bits in widths::<N>() {
        let (min, max) = (bigint::min(bits, false), bigint::max(bits, false));
        // No limbs hold only zero, from which nothing random is drawn.
        let samples = bigint::arithmetic_samples(bits, B, &min, &max, || match N {
          0 => BigInt::ZERO,
          _ => bigint::from_limbs(&rng.limbs::<N>(), false),
        });
        let limbs: Vec<[Limb; N]> = samples
          .iter()
          .map(|value| bigint::to_limbs_of(value, false).expect("in range"))
          .collect();
        for (a, x) in samples.iter().zip(&limbs) {
          check_powers_bits(a, x, bits);
          for (b, y) in samples.iter().zip(&limbs) {
            check_division_bits(a, x, b, y, bits);
          }
        }
      }
    }

    #[test]
    #[cfg_attr(
      all(miri, target_pointer_width = "32"),
      ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
    )]
    fn division_and_powers_of_a_width_match_big_integers() {
      division_and_powers_against_big_integers::<0>();
      division_and_powers_against_big_integers::<1>();
      division_and_powers_against_big_integers::<2>();
      division_and_powers_against_big_integers::<3>();
    }

    /// Checks the roots of `[Limb::MAX; N]`, which take the most Newton steps of `N` limbs, and of
    /// the perfect squares, cubes and fifth powers of `N` limbs with their neighbors, against
    /// `num-bigint`.
    fn roots_against_big_integers<const N: usize>() {
      // Above 4096 bits, which only limbs of more than 64 bits reach here, the values take no
      // path that narrower ones do not, and the time grows faster than the width.
      if B as usize * N > 4096 {
        return;
      }
      let largest = bigint::from_limbs(&[Limb::MAX; N], false);
      let one = BigInt::from(1u8);
      let mut values = vec![largest.clone()];
      for root in [
        BigInt::from(3u8),
        BigInt::from(10u8),
        BigInt::from(Limb::MAX),
        &one << B,
      ] {
        for n in [2, 3, 5] {
          let power = root.pow(n);
          values.extend([&power - 1u8, power.clone(), &power + 1u8]);
        }
      }
      for value in values.into_iter().filter(|value| *value <= largest) {
        let limbs: [Limb; N] = bigint::to_limbs_of(&value, false).expect("in range");
        assert_eq!(
          bigint::from_limbs(&isqrt(&limbs), false),
          value.sqrt(),
          "{value}"
        );
        // Degrees of more than one limb of 8 bits, which `nth_root` takes in limbs, and degrees
        // above the logarithm, whose root is one.
        for n in [2, 3, 5, 64, 255, 256, 300, 511, 1000, u32::MAX] {
          assert_eq!(
            bigint::from_limbs(&nth_root(&limbs, n), false),
            value.nth_root(n),
            "the root {n} of {value}"
          );
        }
      }
    }

    #[test]
    #[cfg_attr(
      all(miri, target_pointer_width = "32"),
      ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
    )]
    fn roots_match_big_integers() {
      roots_against_big_integers::<1>();
      roots_against_big_integers::<2>();
      roots_against_big_integers::<3>();
      roots_against_big_integers::<4>();
      roots_against_big_integers::<8>();
      roots_against_big_integers::<64>();
    }

    /// Checks the logarithms of `N` limbs at the powers of ten and the values below them, and at
    /// `[Limb::MAX; N]` to the bases of [`bases`], against `num-bigint`.
    fn logarithms_against_big_integers<const N: usize>() {
      let largest = bigint::from_limbs(&[Limb::MAX; N], false);
      let ten: [Limb; N] = bigint::to_limbs_of(&BigInt::from(10u8), false).expect("ten");
      for exp in bigint::exponents_by(bigint::max_exp(&largest), Limb::MAX.ilog10()) {
        let power: [Limb; N] = bigint::to_limbs_of(&bigint::pow10(exp), false).expect("in range");
        assert_eq!(checked_ilog10(&power), Some(exp), "log10(10^{exp})");
        assert_eq!(checked_ilog(&power, &ten), Some(exp), "log10(10^{exp})");
        if exp > 0 {
          let below = overflowing_sub(&power, &one()).0;
          assert_eq!(checked_ilog10(&below), Some(exp - 1), "log10(10^{exp} - 1)");
        }
      }
      for (base, limbs) in bases::<N>() {
        assert_eq!(
          checked_ilog(&[Limb::MAX; N], &limbs),
          bigint::ilog(&largest, &base),
          "log({largest}) to the base {base}"
        );
      }
    }

    #[test]
    #[cfg_attr(
      all(miri, target_pointer_width = "32"),
      ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
    )]
    fn logarithms_match_big_integers() {
      logarithms_against_big_integers::<1>();
      logarithms_against_big_integers::<2>();
      logarithms_against_big_integers::<3>();
      logarithms_against_big_integers::<8>();
    }

    /// No limbs hold only zero: every divisor is zero, so every division gives `None`; zero is a
    /// multiple of zero; `0^0`, one, and the power of two of zero overflow zero bits; and zero has
    /// no logarithm.
    #[test]
    fn no_limbs_divide_nothing() {
      const NONE: [Limb; 0] = [];
      assert_eq!(div_rem_bits(&NONE, &NONE, 0), None);
      assert_eq!(div_rem_euclid_bits(&NONE, &NONE, 0), None);
      assert_eq!(div_floor_bits(&NONE, &NONE, 0), None);
      assert_eq!(div_ceil_bits(&NONE, &NONE, 0), None);
      assert_eq!(next_multiple_of_bits(&NONE, &NONE, 0), None);
      assert_eq!(checked_div_exact_bits(&NONE, &NONE, 0), None);
      assert!(is_multiple_of(&NONE, &NONE));
      assert_eq!(overflowing_pow_bits(&NONE, 0, 0), (NONE, true));
      assert_eq!(overflowing_pow_bits(&NONE, 5, 0), (NONE, false));
      assert_eq!(next_power_of_two_bits(&NONE, 0), (NONE, true));
      assert_eq!((isqrt(&NONE), nth_root(&NONE, 3)), (NONE, NONE));
      assert_eq!(
        (midpoint(&NONE, &NONE), abs_diff(&NONE, &NONE)),
        (NONE, NONE)
      );
      assert_eq!(checked_ilog2(&NONE), None);
      assert_eq!(checked_ilog10(&NONE), None);
      assert_eq!(checked_ilog(&NONE, &NONE), None);
    }
  };
  (signed unsigned: $unsigned:ident) => {
    use core::cmp::Ordering;
    use std::{string::ToString, vec, vec::Vec};

    use num_bigint::BigInt;

    use super::*;
    use crate::{bigint, int::DIGIT_BUF_LEN};

    /// The number of bits of a limb.
    const B: u32 = Limb::BITS;

    /// The limb with only its top bit set.
    const HIGH: Limb = 1 << (B - 1);

    /// The number of bits of the widest values that the tests of digits write: 4096, or under Miri,
    /// which divides many narrow limbs slowly, 512 for limbs narrower than 32 bits.
    const WIDEST: u32 = if B < 32 && cfg!(miri) { 512 } else { 4096 };

    /// `MIN`, with only the sign bit set.
    fn min<const N: usize>() -> [Limb; N] {
      let mut min = [0; N];
      min[N - 1] = HIGH;
      min
    }

    /// `MAX`, with every bit but the sign bit set.
    fn max<const N: usize>() -> [Limb; N] {
      let mut max = [Limb::MAX; N];
      max[N - 1] = HIGH - 1;
      max
    }

    fn sign_transitions<const N: usize>() {
      let one = {
        let mut one = [0; N];
        one[0] = 1;
        one
      };
      let minus_one = [Limb::MAX; N];
      // The top limb's sign bit flips exactly at `MAX + 1` and `MIN - 1`.
      assert!(!is_negative(&max::<N>()) && is_negative(&min::<N>()));
      assert_eq!(checked_add(&max::<N>(), &one), None);
      assert_eq!(checked_sub(&min::<N>(), &one), None);
      assert_eq!(checked_add(&min::<N>(), &minus_one), None);
      assert_eq!(checked_sub(&max::<N>(), &minus_one), None);
      assert_eq!(checked_add(&minus_one, &one), Some([0; N]));
      assert_eq!(checked_add(&max::<N>(), &min::<N>()), Some(minus_one));
      assert_eq!(checked_sub(&min::<N>(), &min::<N>()), Some([0; N]));
      assert_eq!(
        checked_sub(&[0; N], &max::<N>()),
        Some({
          let mut negated = min::<N>();
          negated[0] = 1;
          negated
        })
      );
      assert_eq!(checked_sub(&[0; N], &min::<N>()), None);

      // Numeric order, where the unsigned order of the limbs puts the negative values on top.
      assert_eq!(cmp(&min::<N>(), &minus_one), Ordering::Less);
      assert_eq!(cmp(&minus_one, &[0; N]), Ordering::Less);
      assert_eq!(cmp(&[0; N], &one), Ordering::Less);
      assert_eq!(cmp(&one, &max::<N>()), Ordering::Less);
      assert_eq!(cmp(&minus_one, &max::<N>()), Ordering::Less);
      assert_eq!(
        super::$unsigned::cmp(&minus_one, &max::<N>()),
        Ordering::Greater
      );

      // `MIN` has no negation, but its magnitude is unsigned.
      assert_eq!(unsigned_abs(&min::<N>()), min::<N>());
      assert_eq!(neg(&max::<N>()), {
        let mut negated = min::<N>();
        negated[0] = 1;
        negated
      });
      assert_eq!(abs(&minus_one), one);

      // Scaling reaches the limits on both sides, and one step beyond them fails; the magnitude
      // of `MIN` itself scales by `10^0`.
      assert_eq!(checked_mul_pow10(&min::<N>(), 0), Some(min::<N>()));
      assert_eq!(mul_pow10(&min::<N>(), 0), min::<N>());
      for limit in [min::<N>(), max::<N>()] {
        let limit = bigint::from_limbs(&limit, true);
        let inside = &limit / 10u8;
        let outside = if limit < BigInt::ZERO {
          &inside - 1u8
        } else {
          &inside + 1u8
        };
        let scaled = bigint::to_limbs_of::<Limb, N>(&(&inside * 10u8), true);
        assert!(scaled.is_some());
        let inside = bigint::to_limbs_of::<Limb, N>(&inside, true).expect("fits");
        let outside = bigint::to_limbs_of::<Limb, N>(&outside, true).expect("fits");
        assert_eq!(checked_mul_pow10(&inside, 1), scaled);
        assert_eq!(checked_mul_pow10(&outside, 1), None);
      }
    }

    #[test]
    fn sign_transitions_of_the_top_limb() {
      sign_transitions::<3>();
      sign_transitions::<4>();
      sign_transitions::<5>();
      sign_transitions::<6>();
      sign_transitions::<7>();
      sign_transitions::<8>();
    }

    fn digits_of_min_and_max<const N: usize>() {
      for value in [min::<N>(), max::<N>(), [Limb::MAX; N], [0; N]] {
        let expected = bigint::from_limbs(&value, true).magnitude().to_string();
        // `write_digits` requires at most 512 bits, whose digits fit `DIGIT_BUF_LEN` bytes, and
        // `DIGITS_PER_LIMB * N` bytes hold the digits of any `N` limbs.
        if B as usize * N <= 512 {
          let mut buf = [0xff; DIGIT_BUF_LEN];
          let start = write_digits(&value, &mut buf);
          assert_eq!(&buf[start..], expected.as_bytes());
        }
        let mut buf = vec![0xff; crate::int::unsigned::$unsigned::DIGITS_PER_LIMB * N];
        let start =
          crate::int::unsigned::$unsigned::write_digits_into(&unsigned_abs(&value), &mut buf);
        assert_eq!(&buf[start..], expected.as_bytes());
      }
    }

    #[test]
    fn digits_of_zero_min_and_max() {
      digits_of_min_and_max::<3>();
      digits_of_min_and_max::<4>();
      digits_of_min_and_max::<5>();
      digits_of_min_and_max::<6>();
      digits_of_min_and_max::<7>();
      digits_of_min_and_max::<8>();
      let mut buf = [0; DIGIT_BUF_LEN];
      assert_eq!(
        write_digits(&min::<{ 512 / B as usize }>(), &mut buf),
        1,
        "|i512::MIN| has one digit fewer than the buffer"
      );
    }

    /// Every conversion between two and three limbs of either sign, against `num-bigint`.
    #[test]
    fn conversions_between_signs_and_widths() {
      let mut values: Vec<BigInt> = vec![BigInt::ZERO, BigInt::from(1u8), BigInt::from(-1)];
      for bits in [B - 1, B, 2 * B - 1, 2 * B, 3 * B - 1, 3 * B] {
        let power = BigInt::from(1u8) << bits;
        values.extend([&power - 1u8, power.clone(), &power + 1u8]);
        values.extend([1u8 - &power, -&power, -&power - 1u8]);
      }
      for value in &values {
        let two_signed = bigint::to_limbs_of::<Limb, 2>(value, true);
        let two_unsigned = bigint::to_limbs_of::<Limb, 2>(value, false);
        let three_signed = bigint::to_limbs_of::<Limb, 3>(value, true);
        let three_unsigned = bigint::to_limbs_of::<Limb, 3>(value, false);

        if let Some(limbs) = two_signed {
          assert_eq!(checked_resize::<2, 3>(&limbs), three_signed);
          assert_eq!(resize::<2, 3>(&limbs), three_signed.expect("wider"));
          assert_eq!(checked_to_unsigned::<2, 3>(&limbs), three_unsigned);
          assert_eq!(checked_to_unsigned::<2, 2>(&limbs), two_unsigned);
        }
        if let Some(limbs) = three_signed {
          assert_eq!(checked_resize::<3, 2>(&limbs), two_signed);
          assert_eq!(checked_to_unsigned::<3, 2>(&limbs), two_unsigned);
          if let Some(narrow) = two_signed {
            assert_eq!(resize::<3, 2>(&limbs), narrow);
          }
        }
        if let Some(limbs) = two_unsigned {
          assert_eq!(checked_from_unsigned::<2, 3>(&limbs), three_signed);
          assert_eq!(checked_from_unsigned::<2, 2>(&limbs), two_signed);
        }
        if let Some(limbs) = three_unsigned {
          assert_eq!(checked_from_unsigned::<3, 2>(&limbs), two_signed);
          assert_eq!(checked_from_unsigned::<3, 3>(&limbs), three_signed);
        }
      }
    }

    /// The value `value`, which must fit `N` limbs.
    fn limbs<const N: usize>(value: i64) -> [Limb; N] {
      bigint::to_limbs_of(&BigInt::from(value), true).expect("fits")
    }

    fn division_with_min<const N: usize>() {
      let min = min::<N>();
      let one = limbs::<N>(1);
      let mut half_min = [0; N];
      half_min[N - 1] = HIGH | HIGH >> 1;
      assert_eq!(div_rem(&min, &one), (min, [0; N]));
      assert_eq!(div_rem(&min, &limbs(2)), (half_min, [0; N]));
      assert_eq!(div_rem(&min, &min), (one, [0; N]));
      // `2^(B * N - 1) = 2 (mod 3)`, an odd power of two, so the remainder is `-2`.
      let (quotient, remainder) = div_rem(&min, &limbs(3));
      assert_eq!(remainder, limbs(-2));
      assert_eq!(
        bigint::from_limbs(&quotient, true),
        bigint::min(B * N as u32, true) / 3
      );
      let min_plus_one =
        bigint::to_limbs_of(&(bigint::min(B * N as u32, true) + 1u8), true).expect("fits");
      assert_eq!(div_rem(&min_plus_one, &limbs(-1)), (max::<N>(), [0; N]));
      assert_eq!(div_rem(&limbs::<N>(-7), &limbs(2)), (limbs(-3), limbs(-1)));
      assert_eq!(div_rem(&limbs::<N>(7), &limbs(-2)), (limbs(-3), limbs(1)));
      assert_eq!(div_rem(&limbs::<N>(-7), &limbs(-2)), (limbs(3), limbs(-1)));
    }

    #[test]
    fn division_with_min_for_every_width() {
      division_with_min::<1>();
      division_with_min::<2>();
      division_with_min::<3>();
      division_with_min::<4>();
      division_with_min::<5>();
      division_with_min::<6>();
      division_with_min::<7>();
      division_with_min::<8>();
    }

    /// A deterministic xorshift generator, so that a failure replays from the seed.
    struct Rng(u64);

    impl Rng {
      fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
      }

      /// A random limb: one draw, or two for a limb of more than 64 bits, the high half first.
      fn limb(&mut self) -> Limb {
        if B > 64 {
          ((self.next() as u128) << 64 | self.next() as u128) as Limb
        } else {
          self.next() as Limb
        }
      }

      /// A value of either sign whose magnitude has a random number of significant limbs.
      fn signed<const N: usize>(&mut self) -> [Limb; N] {
        let mut magnitude = [0; N];
        let count = (self.next() % N as u64) as usize + 1;
        for limb in &mut magnitude[..count] {
          *limb = self.limb();
        }
        magnitude[count - 1] >>= self.next() % u64::from(B);
        magnitude[N - 1] &= Limb::MAX >> 1;
        if self.next().is_multiple_of(2) {
          magnitude
        } else {
          neg(&magnitude)
        }
      }
    }

    fn division_against_big_integers<const N: usize>() {
      let mut rng = Rng(0x0dd_ba11 ^ N as u64);
      let count = if cfg!(miri) { 8 } else { 2000 };
      for _ in 0..count {
        let (a, b) = (rng.signed::<N>(), rng.signed::<N>());
        if is_zero(&b) {
          continue;
        }
        let (quotient, remainder) = div_rem(&a, &b);
        let (big_a, big_b) = (bigint::from_limbs(&a, true), bigint::from_limbs(&b, true));
        // `num-bigint` rounds toward zero, as `/` and `%` do.
        assert_eq!(bigint::from_limbs(&quotient, true), &big_a / &big_b);
        assert_eq!(bigint::from_limbs(&remainder, true), &big_a % &big_b);
      }
    }

    #[test]
    fn division_against_big_integers_for_every_width() {
      division_against_big_integers::<3>();
      division_against_big_integers::<4>();
      division_against_big_integers::<8>();
    }

    /// Checks `round_div(value, divisor, mode)` against rounding by distance.
    fn check_rounding<const N: usize>(value: &[Limb; N], divisor: &[Limb; N]) {
      let (big_value, big_divisor) = (
        bigint::from_limbs(value, true),
        bigint::from_limbs(divisor, true),
      );
      let truncated = &big_value / &big_divisor;
      for mode in bigint::MODES {
        let (rounded, remainder, away) = round_div(value, divisor, mode);
        let rounded = bigint::from_limbs(&rounded, true);
        assert_eq!(
          rounded,
          bigint::round(&big_value, &big_divisor, mode),
          "{big_value} / {big_divisor}, {mode:?}"
        );
        assert_eq!(
          bigint::from_limbs(&remainder, true),
          &big_value % &big_divisor
        );
        assert_eq!(away, rounded != truncated);
      }
    }

    fn rounding_division<const N: usize>() {
      // Values below the divisor in magnitude, whose quotient is zero, and ties of each parity.
      for value in [-9, -6, -5, -4, -1, 0, 1, 4, 5, 6, 9, -15, -25, 15, 25] {
        check_rounding(&limbs::<N>(value), &limbs(10));
      }
      check_rounding(&min::<N>(), &limbs(3));
      check_rounding(&max::<N>(), &limbs(2));
      let mut rng = Rng(0xfee1_600d ^ N as u64);
      let count = if cfg!(miri) { 4 } else { 600 };
      for _ in 0..count {
        let divisor = unsigned_abs(&rng.signed::<N>());
        if is_zero(&divisor) {
          continue;
        }
        check_rounding(&rng.signed::<N>(), &divisor);
      }
    }

    #[test]
    fn rounding_division_matches_rounding_by_distance() {
      rounding_division::<1>();
      rounding_division::<2>();
      rounding_division::<3>();
      rounding_division::<8>();
    }

    /// The widths at which the functions of a width are checked: 0, 1, `B - 1`, `B`, `B + 1`,
    /// `B * N - 1` and `B * N`, those that `N` limbs hold.
    fn widths<const N: usize>() -> Vec<u32> {
      let top = B * N as u32;
      let mut widths: Vec<u32> = [0, 1, B - 1, B, B + 1, top.saturating_sub(1), top]
        .into_iter()
        .filter(|&width| width <= top)
        .collect();
      widths.sort_unstable();
      widths.dedup();
      widths
    }

    fn widths_against_big_integers<const N: usize>() {
      // Above 4096 bits, which only limbs of more than 64 bits reach here, the values take no
      // path that narrower ones do not, and the time grows faster than the width.
      if B as usize * N > 4096 {
        return;
      }
      let widths = widths::<N>();
      let patterns = bigint::bit_patterns_of::<Limb, N>(&widths);
      let values: Vec<(BigInt, BigInt)> = patterns
        .iter()
        .map(|pattern| {
          (
            bigint::from_limbs(pattern, true),
            bigint::from_limbs(pattern, false),
          )
        })
        .collect();
      for &bits in &widths {
        let (min, max) = (bigint::min(bits, true), bigint::max(bits, true));
        assert_eq!(bigint::from_limbs(&max_of_bits::<N>(bits), true), max);
        assert_eq!(bigint::from_limbs(&min_of_bits::<N>(bits), true), min);
        assert_eq!(
          bigint::to_limbs_of(&max, true),
          Some(max_of_bits::<N>(bits))
        );
        assert_eq!(
          bigint::to_limbs_of(&min, true),
          Some(min_of_bits::<N>(bits))
        );
        let modulus = BigInt::from(1u8) << bits;
        for (pattern, (value, unsigned)) in patterns.iter().zip(&values) {
          assert_eq!(
            fits_bits(pattern, bits),
            min <= *value && *value <= max,
            "{pattern:x?} in {bits} bits"
          );
          // The two's complement integer in the low `bits` bits.
          let low = unsigned % &modulus;
          let expected = if low > max { low - &modulus } else { low };
          assert_eq!(
            bigint::from_limbs(&sign_extend(pattern, bits), true),
            expected,
            "{pattern:x?} from {bits} bits"
          );
        }
      }
    }

    #[test]
    #[cfg_attr(
      all(miri, target_pointer_width = "32"),
      ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
    )]
    fn widths_match_big_integers() {
      widths_against_big_integers::<0>();
      widths_against_big_integers::<1>();
      widths_against_big_integers::<2>();
      widths_against_big_integers::<3>();
      widths_against_big_integers::<4>();
      widths_against_big_integers::<8>();
      widths_against_big_integers::<16>();
      widths_against_big_integers::<64>();
    }

    /// Checks the arithmetic of each width of `N` limbs against `num-bigint`: the result wrapped
    /// to the width, the overflow flag, and that the result has the width.
    fn arithmetic_bits_against_big_integers<const N: usize>() {
      let mut rng = Rng(0x5167_5eed ^ N as u64);
      for bits in bigint::arithmetic_widths(B, N) {
        let (min, max) = (bigint::min(bits, true), bigint::max(bits, true));
        // No limbs hold only zero, from which nothing random is drawn.
        let samples = bigint::arithmetic_samples(bits, B, &min, &max, || match N {
          0 => BigInt::ZERO,
          _ => bigint::from_limbs(&rng.signed::<N>(), true),
        });
        let limbs: Vec<[Limb; N]> = samples
          .iter()
          .map(|value| bigint::to_limbs_of(value, true).expect("in range"))
          .collect();
        // `exact` is `lhs op rhs`, and `computed` its result in the layer.
        let check = |exact: BigInt, computed: ([Limb; N], bool), lhs: &BigInt, op, rhs: &BigInt| {
          let (result, overflow) = computed;
          assert_eq!(
            bigint::from_limbs(&result, true),
            bigint::wrap(&exact, bits, &min),
            "{lhs} {op} {rhs} in {bits} bits"
          );
          assert_eq!(
            overflow,
            exact < min || max < exact,
            "{lhs} {op} {rhs} in {bits} bits"
          );
          assert!(fits_bits(&result, bits), "{lhs} {op} {rhs} in {bits} bits");
        };
        let (zero, one) = (BigInt::ZERO, BigInt::from(1u8));
        for (a, limbs_a) in samples.iter().zip(&limbs) {
          assert_eq!(is_one(limbs_a), *a == one, "{a}");
          let sign = if *a < zero { -1 } else { i8::from(*a > zero) };
          assert_eq!(
            bigint::from_limbs(&signum(limbs_a), true),
            BigInt::from(sign),
            "{a}"
          );
          check(-a, overflowing_neg_bits(limbs_a, bits), &zero, '-', a);
          for (b, limbs_b) in samples.iter().zip(&limbs) {
            for (exact, op, computed) in [
              (a + b, '+', overflowing_add_bits(limbs_a, limbs_b, bits)),
              (a - b, '-', overflowing_sub_bits(limbs_a, limbs_b, bits)),
              (a * b, '*', overflowing_mul_bits(limbs_a, limbs_b, bits)),
            ] {
              check(exact, computed, a, op, b);
            }
          }
        }
      }
    }

    #[test]
    #[cfg_attr(
      all(miri, target_pointer_width = "32"),
      ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
    )]
    fn arithmetic_bits_match_big_integers() {
      arithmetic_bits_against_big_integers::<0>();
      arithmetic_bits_against_big_integers::<1>();
      arithmetic_bits_against_big_integers::<2>();
      arithmetic_bits_against_big_integers::<3>();
    }

    /// The magnitude of `-2^4095`, the smallest value of 4096 bits, has 1233 digits, and that of
    /// `-2^511` has 154.
    #[test]
    #[cfg_attr(
      all(miri, target_pointer_width = "32"),
      ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
    )]
    fn digits_of_the_smallest_value_of_64_limbs() {
      let min = min_of_bits::<{ (WIDEST / B) as usize }>(WIDEST);
      let expected = (-bigint::from_limbs(&min, true)).to_string();
      assert_eq!(expected.len(), if WIDEST == 4096 { 1233 } else { 154 });
      let mut buf = vec![0; expected.len()];
      assert_eq!(
        crate::int::unsigned::$unsigned::write_digits_into(&unsigned_abs(&min), &mut buf),
        0
      );
      assert_eq!(buf, expected.as_bytes());
    }

    fn powers_of_ten<const N: usize>() {
      let max_exp = bigint::max_exp(&bigint::from_limbs(&max::<N>(), true));
      // Under Miri, the exponents around one and two scaling steps of the lower layer.
      for exp in bigint::exponents_by(max_exp, Limb::MAX.ilog10()) {
        assert_eq!(
          bigint::from_limbs(&pow10::<N>(exp), true),
          bigint::pow10(exp)
        );
        assert_eq!(
          bigint::from_limbs(&max_magnitude::<N>(exp), true),
          bigint::pow10(exp) - 1u8
        );
      }
    }

    #[test]
    fn pow10_and_max_magnitude() {
      powers_of_ten::<1>();
      powers_of_ten::<2>();
      powers_of_ten::<3>();
      powers_of_ten::<8>();
    }

    /// Checks the functions of a width that divide, round a quotient or relate two values, on `a`
    /// and `b` of `bits` bits, in `N` limbs `x` and `y` of two's complement, against `num-bigint`:
    /// each result is the exact one, or that wrapped to the width with its overflow flag, as for
    /// `MIN / -1`, and has the width; the absolute difference is unsigned. A zero divisor gives
    /// `None`.
    fn check_division_bits<const N: usize>(
      a: &BigInt,
      x: &[Limb; N],
      b: &BigInt,
      y: &[Limb; N],
      bits: u32,
    ) {
      let (min, max) = (bigint::min(bits, true), bigint::max(bits, true));
      let big = |limbs: &[Limb; N]| bigint::from_limbs(limbs, true);
      // `exact` wrapped to the width, and whether it is outside the range.
      let wrapped = |exact: &BigInt| {
        (
          bigint::wrap(exact, bits, &min),
          *exact < min || max < *exact,
        )
      };
      let zero = BigInt::ZERO;
      // `/` rounds toward zero.
      assert_eq!(
        big(&midpoint(x, y)),
        (a + b) / 2u8,
        "midpoint of {a} and {b}"
      );
      let difference = if a < b { b - a } else { a - b };
      assert_eq!(
        bigint::from_limbs(&abs_diff(x, y), false),
        difference,
        "|{a} - {b}|"
      );
      if *b == zero {
        assert_eq!(div_rem_bits(x, y, bits), None);
        assert_eq!(div_rem_euclid_bits(x, y, bits), None);
        assert_eq!(div_floor_bits(x, y, bits), None);
        assert_eq!(div_ceil_bits(x, y, bits), None);
        assert_eq!(next_multiple_of_bits(x, y, bits), None);
        assert_eq!(checked_div_exact_bits(x, y, bits), None);
        return;
      }
      let (quotient, remainder) = (a / b, a % b);
      let (truncated, overflow) = wrapped(&quotient);
      let division = div_rem_bits(x, y, bits).expect("a nonzero divisor");
      assert_eq!(
        (big(&division.0), big(&division.1), division.2),
        (truncated, remainder.clone(), overflow),
        "{a} / {b} in {bits} bits"
      );
      assert!(fits_bits(&division.0, bits), "{a} / {b} in {bits} bits");
      let (euclid_quotient, euclid_remainder) = bigint::div_rem_euclid(a, b);
      let (euclid, euclid_overflow) = wrapped(&euclid_quotient);
      let division = div_rem_euclid_bits(x, y, bits).expect("a nonzero divisor");
      assert_eq!(
        (big(&division.0), big(&division.1), division.2),
        (euclid, euclid_remainder, euclid_overflow),
        "{a} / {b} in {bits} bits, Euclidean"
      );
      assert!(fits_bits(&division.0, bits), "{a} / {b} in {bits} bits");
      for (name, result, exact) in [
        ("floor", div_floor_bits(x, y, bits), bigint::div_floor(a, b)),
        ("ceiling", div_ceil_bits(x, y, bits), bigint::div_ceil(a, b)),
        (
          "next multiple",
          next_multiple_of_bits(x, y, bits),
          bigint::next_multiple_of(a, b),
        ),
      ] {
        let (value, flag) = result.expect("a nonzero divisor");
        assert_eq!(
          (big(&value), flag),
          wrapped(&exact),
          "the {name} of {a} and {b} in {bits} bits"
        );
        assert!(fits_bits(&value, bits), "the {name} of {a} and {b}");
      }
      assert_eq!(
        checked_div_exact_bits(x, y, bits).map(|exact| big(&exact)),
        (remainder == zero && !overflow).then_some(quotient),
        "{a} / {b} exactly"
      );
    }

    /// The bases at which the logarithms of `N` limbs are checked: negative ones and those below 2,
    /// which have none, powers of two, ten, bases of one limb and of two, and the largest value.
    fn bases<const N: usize>() -> Vec<(BigInt, [Limb; N])> {
      let one = BigInt::from(1u8);
      let limb = &one << B;
      let candidates = [
        BigInt::from(-2),
        BigInt::from(-1),
        BigInt::ZERO,
        one.clone(),
        BigInt::from(2u8),
        BigInt::from(3u8),
        BigInt::from(10u8),
        BigInt::from(16u8),
        BigInt::from(259u16),
        BigInt::from(1000u16),
        &limb - 1u8,
        limb.clone(),
        &limb + 3u8,
        bigint::from_limbs(&max_of_bits::<N>(B * N as u32), true),
      ];
      candidates
        .into_iter()
        .filter_map(|base| bigint::to_limbs_of(&base, true).map(|limbs| (base, limbs)))
        .collect()
    }

    /// Checks the powers, roots and logarithms of `a`, of `bits` bits, in `N` limbs `x` of two's
    /// complement, against `num-bigint`: a negative value has a root of every odd degree, rounded
    /// toward zero, and no square root or logarithm.
    fn check_powers_bits<const N: usize>(a: &BigInt, x: &[Limb; N], bits: u32) {
      let (min, max) = (bigint::min(bits, true), bigint::max(bits, true));
      let big = |limbs: &[Limb; N]| bigint::from_limbs(limbs, true);
      let exps = [
        0,
        1,
        2,
        3,
        bits.saturating_sub(1),
        bits,
        bits + 1,
        2 * bits + 1,
      ];
      for exp in exps.into_iter().chain([u32::MAX - 1, u32::MAX]) {
        let (value, overflow) = overflowing_pow_bits(x, exp, bits);
        assert_eq!(
          (big(&value), overflow),
          bigint::pow_bits(a, exp, bits, &min, &max),
          "{a}^{exp} in {bits} bits"
        );
        assert!(fits_bits(&value, bits), "{a}^{exp} in {bits} bits");
      }
      let negative = *a < BigInt::ZERO;
      assert_eq!(
        checked_isqrt(x).map(|root| big(&root)),
        (!negative).then(|| a.sqrt()),
        "the square root of {a}"
      );
      assert_eq!(checked_nth_root(x, 0), None);
      for n in [1, 2, 3, 4, 7, bits, bits + 1, u32::MAX] {
        if n > 0 {
          assert_eq!(
            checked_nth_root(x, n).map(|root| big(&root)),
            (!negative || n % 2 == 1).then(|| a.nth_root(n)),
            "the root {n} of {a}"
          );
        }
      }
      let log2 = (*a > BigInt::ZERO).then(|| u32::try_from(a.bits() - 1).expect("a width"));
      assert_eq!(checked_ilog2(x), log2, "log2({a})");
      assert_eq!(
        checked_ilog10(x),
        bigint::ilog(a, &BigInt::from(10u8)),
        "log10({a})"
      );
      for (base, limbs) in bases::<N>() {
        assert_eq!(
          checked_ilog(x, &limbs),
          bigint::ilog(a, &base),
          "log({a}) to the base {base}"
        );
      }
    }

    /// Checks the division, the powers, the roots and the logarithms of each width of `N` limbs
    /// against `num-bigint`, at the values of `bigint::arithmetic_samples` and their pairs.
    fn division_and_powers_against_big_integers<const N: usize>() {
      let mut rng = Rng(0xd1f1_51a1 ^ N as u64);
      for bits in widths::<N>() {
        let (min, max) = (bigint::min(bits, true), bigint::max(bits, true));
        // No limbs hold only zero, from which nothing random is drawn.
        let samples = bigint::arithmetic_samples(bits, B, &min, &max, || match N {
          0 => BigInt::ZERO,
          _ => bigint::from_limbs(&rng.signed::<N>(), true),
        });
        let limbs: Vec<[Limb; N]> = samples
          .iter()
          .map(|value| bigint::to_limbs_of(value, true).expect("in range"))
          .collect();
        for (a, x) in samples.iter().zip(&limbs) {
          check_powers_bits(a, x, bits);
          for (b, y) in samples.iter().zip(&limbs) {
            check_division_bits(a, x, b, y, bits);
          }
        }
      }
    }

    #[test]
    #[cfg_attr(
      all(miri, target_pointer_width = "32"),
      ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
    )]
    fn division_and_powers_of_a_width_match_big_integers() {
      division_and_powers_against_big_integers::<0>();
      division_and_powers_against_big_integers::<1>();
      division_and_powers_against_big_integers::<2>();
      division_and_powers_against_big_integers::<3>();
    }

    /// `MIN / -1` overflows at the full width, where the division of the magnitudes would not
    /// accept it, and at a partial width, where the storage holds the quotient: both wrap to `MIN`
    /// with the remainder zero. One bit holds only -1 and 0, so `-1 / -1` overflows, `-1` to an
    /// even power overflows, and every other division by -1 is exact.
    fn division_by_minus_one<const N: usize>() {
      let minus_one = [Limb::MAX; N];
      for bits in [1, B * N as u32 - 1, B * N as u32] {
        let min = min_of_bits::<N>(bits);
        assert_eq!(
          div_rem_bits(&min, &minus_one, bits),
          Some((min, [0; N], true))
        );
        assert_eq!(
          div_rem_euclid_bits(&min, &minus_one, bits),
          Some((min, [0; N], true))
        );
        assert_eq!(div_floor_bits(&min, &minus_one, bits), Some((min, true)));
        assert_eq!(div_ceil_bits(&min, &minus_one, bits), Some((min, true)));
        assert_eq!(
          next_multiple_of_bits(&min, &minus_one, bits),
          Some((min, false))
        );
        assert_eq!(checked_div_exact_bits(&min, &minus_one, bits), None);
        let above = overflowing_add_bits(&min, &super::$unsigned::one(), bits).0;
        if bits > 1 {
          let quotient = Some((max_of_bits::<N>(bits), [0; N], false));
          assert_eq!(div_rem_bits(&above, &minus_one, bits), quotient);
        }
      }
      assert_eq!(overflowing_pow_bits(&minus_one, 0, 1), (minus_one, true));
      assert_eq!(overflowing_pow_bits(&minus_one, 2, 1), (minus_one, true));
      assert_eq!(overflowing_pow_bits(&minus_one, 3, 1), (minus_one, false));
      assert_eq!(
        div_rem_bits(&[0; N], &minus_one, 1),
        Some(([0; N], [0; N], false))
      );
      assert_eq!(checked_div_exact_bits(&[0; N], &minus_one, 1), Some([0; N]));
    }

    #[test]
    fn division_by_minus_one_for_every_width() {
      division_by_minus_one::<1>();
      division_by_minus_one::<2>();
      division_by_minus_one::<3>();
      division_by_minus_one::<8>();
    }
  };
}

pub(crate) use limb_layer_tests;

/// Defines, in the `tests` module of an instance of `limb_layer!` for a limb type other than `u64`,
/// after `limb_layer_tests!`, the helpers of the tests that compare the instance with native
/// integers and with the layer for `[u64; N]`: `other_limb_tests!(unsigned)` for a lower layer,
/// and `other_limb_tests!(signed)` for an upper layer. The samples depend on the limb type only
/// through `B`.
macro_rules! other_limb_tests {
  (unsigned) => {
    /// Values around zero, the limb boundaries, the powers of ten and the limit of `u128`.
    fn samples() -> Vec<u128> {
      let mut values = vec![0, 1, 2, 9, 10, u128::MAX, u128::MAX - 1];
      // `2^(B * k)` for each `k` from 1 below `128 / B`, and `2^127`.
      for boundary in (1..128 / B).map(|k| 1u128 << (B * k)).chain([1 << 127]) {
        values.extend([boundary - 1, boundary, boundary + 1]);
      }
      for exp in 1..=38 {
        let power = 10u128.pow(exp);
        values.extend([power - 1, power, power + 1]);
      }
      values
    }

    /// The samples that `T` holds, or under Miri every third of them, as `bigint::thin` keeps them.
    fn samples_of<T: TryFrom<u128> + bigint::Sample>() -> Vec<T> {
      bigint::thin(
        samples()
          .into_iter()
          .filter_map(|value| T::try_from(value).ok())
          .collect(),
      )
    }

    /// `value` in `N` limbs, least significant first. Requires `B * N <= 128` and
    /// `value < 2^(B * N)`.
    fn limbs_of<T: Into<u128>, const N: usize>(value: T) -> [Limb; N] {
      let value = value.into();
      core::array::from_fn(|i| (value >> (B as usize * i)) as Limb)
    }

    /// The integer in `value` in `N` limbs of 64 bits, least significant first: each run of
    /// `64 / B` limbs joined into one, or each limb of 128 bits split into two, the low half
    /// first. Requires `B * M == 64 * N`.
    const fn to_u64_limbs<const M: usize, const N: usize>(value: &[Limb; M]) -> [u64; N] {
      const { assert!(B as usize * M == 64 * N) };
      // Both arrays hold the integer in the same little-endian bytes.
      let mut bytes = [[0; 8]; N];
      let flat = bytes.as_flattened_mut();
      let mut i = 0;
      while i < M {
        let limb = value[i].to_le_bytes();
        let mut k = 0;
        while k < limb.len() {
          flat[limb.len() * i + k] = limb[k];
          k += 1;
        }
        i += 1;
      }
      let mut words = [0; N];
      let mut i = 0;
      while i < N {
        words[i] = u64::from_le_bytes(bytes[i]);
        i += 1;
      }
      words
    }

    /// Checks the layer for `[Limb; M]` against the layer for `[u64; N]`, which holds the same
    /// integers in limbs of 64 bits. It allocates nothing, so that it also runs on 32-bit Miri,
    /// where the tests against `num-bigint` run out of addresses.
    fn matches_the_u64_layer<const M: usize, const N: usize>() {
      use crate::int::unsigned::b64;

      const { assert!(B as usize * M == 64 * N) };
      let mut rng = Rng(0x5eed_3264 ^ N as u64);
      let count = if cfg!(miri) { 16 } else { 2000 };
      for _ in 0..count {
        // Nonzero values, whose limbs have every normalization shift of a limb.
        let (a, b) = (rng.limbs::<M>(), rng.limbs::<M>());
        let (wide_a, wide_b): ([u64; N], [u64; N]) = (to_u64_limbs(&a), to_u64_limbs(&b));
        assert_eq!(cmp(&a, &b), b64::cmp(&wide_a, &wide_b));
        assert_eq!(le(&a, &b), b64::le(&wide_a, &wide_b));
        let (sum, carry) = overflowing_add(&a, &b);
        assert_eq!(
          (to_u64_limbs(&sum), carry),
          b64::overflowing_add(&wide_a, &wide_b)
        );
        let (difference, borrow) = overflowing_sub(&a, &b);
        assert_eq!(
          (to_u64_limbs(&difference), borrow),
          b64::overflowing_sub(&wide_a, &wide_b)
        );
        let (quotient, remainder) = div_rem(&a, &b);
        assert_eq!(
          (to_u64_limbs(&quotient), to_u64_limbs(&remainder)),
          b64::div_rem(&wide_a, &wide_b),
          "{a:x?} / {b:x?}"
        );
        for mode in bigint::MODES {
          let (rounded, remainder, away) = round_div(&a, &b, mode);
          assert_eq!(
            (to_u64_limbs(&rounded), to_u64_limbs(&remainder), away),
            b64::round_div(&wide_a, &wide_b, mode)
          );
        }

        // The low `B * M / 2` bits, whose product is representable: the low half of the limbs,
        // or for an odd `M`, also the low half of the middle limb.
        let half = B as usize * M / 2;
        let low = |value: &[Limb; M]| -> [Limb; M] {
          core::array::from_fn(|i| match half.saturating_sub(B as usize * i) {
            0 => 0,
            kept if kept >= B as usize => value[i],
            kept => value[i] & (Limb::MAX >> (B as usize - kept)),
          })
        };
        let (low_a, low_b) = (low(&a), low(&b));
        let (wide_low_a, wide_low_b): ([u64; N], [u64; N]) =
          (to_u64_limbs(&low_a), to_u64_limbs(&low_b));
        assert_eq!(
          to_u64_limbs(&mul(&low_a, &low_b)),
          b64::mul(&wide_low_a, &wide_low_b)
        );

        let exp = (rng.next() % (20 * N as u64 + 2)) as u32;
        let product = checked_mul_pow10(&a, exp);
        assert_eq!(
          product.map(|product| to_u64_limbs(&product)),
          b64::checked_mul_pow10(&wide_a, exp)
        );
        if product.is_some() {
          assert_eq!(
            to_u64_limbs(&mul_pow10(&a, exp)),
            b64::mul_pow10(&wide_a, exp)
          );
        }
        let (quotient, remainder) = div_rem_pow10(&a, exp);
        assert_eq!(
          (to_u64_limbs(&quotient), to_u64_limbs(&remainder)),
          b64::div_rem_pow10(&wide_a, exp)
        );

        let bits = (rng.next() % (64 * N as u64 + 1)) as u32;
        assert_eq!(fits_bits(&a, bits), b64::fits_bits(&wide_a, bits));
        assert_eq!(
          to_u64_limbs(&max_of_bits::<M>(bits)),
          b64::max_of_bits::<N>(bits)
        );

        // The digits agree and read back. Both layers need `20 * N` bytes at most.
        let (mut buf, mut wide_buf) = ([0; 160], [0; 160]);
        let start = write_digits_into(&a, &mut buf);
        let wide_start = b64::write_digits_into(&wide_a, &mut wide_buf);
        assert_eq!(&buf[start..], &wide_buf[wide_start..]);
        assert_eq!(push_digits(&[0; M], &buf, start, buf.len()), a);
      }
    }
  };
  (signed) => {
    /// Values around zero, the limb boundaries, the powers of ten and the limits of `i128`.
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
      // `2^(B * k - 1)` and `2^(B * k)` for each `k` from 1 below `128 / B`.
      for shift in (1..128 / B).map(|k| B * k) {
        for boundary in [1i128 << (shift - 1), 1 << shift] {
          values.extend([boundary - 1, boundary, boundary + 1]);
          values.extend([1 - boundary, -boundary, -boundary - 1]);
        }
      }
      for exp in 1..=38 {
        let power = 10i128.pow(exp);
        values.extend([power - 1, power, power + 1, 1 - power, -power, -power - 1]);
      }
      values
    }

    /// The samples that `T` holds, or under Miri every third of them, as `bigint::thin` keeps them.
    fn samples_of<T: TryFrom<i128> + bigint::Sample>() -> Vec<T> {
      bigint::thin(
        samples()
          .into_iter()
          .filter_map(|value| T::try_from(value).ok())
          .collect(),
      )
    }

    /// `value` in `N` limbs of two's complement, least significant first. Requires `B * N <= 128`
    /// and `value` to be within the range of `B * N` bits.
    fn limbs_of<T: Into<i128>, const N: usize>(value: T) -> [Limb; N] {
      let value = value.into();
      core::array::from_fn(|i| (value >> (B as usize * i)) as Limb)
    }

    /// The unsigned integer `value` in `N` limbs, least significant first. Requires
    /// `B * N <= 128` and `value < 2^(B * N)`.
    fn unsigned_limbs_of<T: Into<u128>, const N: usize>(value: T) -> [Limb; N] {
      let value = value.into();
      core::array::from_fn(|i| (value >> (B as usize * i)) as Limb)
    }
  };
}

pub(crate) use other_limb_tests;

/// Defines a test that compares `N` limbs with the native integer of `B * N` bits:
/// `native_oracle!(unsigned name, native, N)` in the `tests` module of a lower layer, and
/// `native_oracle!(signed name, native, N)` in that of an upper layer, after
/// `other_limb_tests!`.
macro_rules! native_oracle {
  (unsigned $name:ident, $native:ident, $n:literal) => {
    #[test]
    fn $name() {
      let samples: Vec<$native> = samples_of();
      let max_exp = $native::MAX.ilog10();
      // At the full width, the arithmetic of a width is the native arithmetic.
      let bits = $native::BITS;
      for &a in &samples {
        let limbs_a: [Limb; $n] = limbs_of(a);
        assert_eq!(is_zero(&limbs_a), a == 0);
        assert_eq!(is_positive(&limbs_a), a > 0);
        assert_eq!(is_one(&limbs_a), a == 1);
        let (negated, borrow) = a.overflowing_neg();
        assert_eq!(
          overflowing_neg_bits(&limbs_a, bits),
          (limbs_of(negated), borrow)
        );
        for &b in &samples {
          let limbs_b: [Limb; $n] = limbs_of(b);
          assert_eq!(cmp(&limbs_a, &limbs_b), a.cmp(&b));
          assert_eq!(le(&limbs_a, &limbs_b), a <= b);
          let (sum, carry) = a.overflowing_add(b);
          assert_eq!(overflowing_add(&limbs_a, &limbs_b), (limbs_of(sum), carry));
          assert_eq!(
            overflowing_add_bits(&limbs_a, &limbs_b, bits),
            (limbs_of(sum), carry)
          );
          let (difference, borrow) = a.overflowing_sub(b);
          assert_eq!(
            overflowing_sub(&limbs_a, &limbs_b),
            (limbs_of(difference), borrow)
          );
          assert_eq!(
            overflowing_sub_bits(&limbs_a, &limbs_b, bits),
            (limbs_of(difference), borrow)
          );
          let (product, overflow) = a.overflowing_mul(b);
          assert_eq!(
            overflowing_mul(&limbs_a, &limbs_b),
            (limbs_of(product), overflow)
          );
          assert_eq!(
            overflowing_mul_bits(&limbs_a, &limbs_b, bits),
            (limbs_of(product), overflow)
          );
          assert_eq!(
            checked_add(&limbs_a, &limbs_b),
            a.checked_add(b).map(limbs_of)
          );
          assert_eq!(
            checked_sub(&limbs_a, &limbs_b),
            a.checked_sub(b).map(limbs_of)
          );
          if let Some(product) = a.checked_mul(b) {
            assert_eq!(mul(&limbs_a, &limbs_b), limbs_of(product));
          }
          if b != 0 {
            assert_eq!(
              div_rem(&limbs_a, &limbs_b),
              (limbs_of(a / b), limbs_of(a % b)),
              "{a} / {b}"
            );
          }
        }
        // Up to two exponents beyond the largest power of ten that the type holds.
        for exp in 0..=max_exp + 2 {
          let power = $native::checked_pow(10, exp);
          // Zero times any power is zero; otherwise the power itself must fit.
          let product = match a {
            0 => Some(0),
            _ => power.and_then(|power| a.checked_mul(power)),
          };
          assert_eq!(checked_mul_pow10(&limbs_a, exp), product.map(limbs_of));
          if let Some(product) = product {
            assert_eq!(mul_pow10(&limbs_a, exp), limbs_of(product));
          }
          let (quotient, remainder) = div_rem_pow10(&limbs_a, exp);
          match power {
            Some(power) => assert_eq!(
              (quotient, remainder),
              (limbs_of(a / power), limbs_of(a % power))
            ),
            None => assert_eq!((quotient, remainder), ([0; $n], limbs_a)),
          }
        }

        let mut buf = [0xff; DIGIT_BUF_LEN];
        let start = write_digits(&limbs_a, &mut buf);
        assert_eq!(&buf[start..], a.to_string().as_bytes());
      }
      for exp in 0..=max_exp {
        assert_eq!(pow10::<$n>(exp), limbs_of($native::pow(10, exp)));
        assert_eq!(
          max_magnitude::<$n>(exp),
          limbs_of($native::pow(10, exp) - 1)
        );
      }
    }
  };
  (signed $name:ident, $native:ident, $n:literal) => {
    #[test]
    fn $name() {
      let samples: Vec<$native> = samples_of();
      let max_exp = $native::MAX.ilog10();
      // At the full width, the arithmetic of a width is the native arithmetic.
      let bits = $native::BITS;
      for &a in &samples {
        let limbs_a: [Limb; $n] = limbs_of(a);
        assert_eq!(is_negative(&limbs_a), a < 0);
        assert_eq!(is_positive(&limbs_a), a > 0);
        assert_eq!(is_zero(&limbs_a), a == 0);
        assert_eq!(unsigned_abs(&limbs_a), unsigned_limbs_of(a.unsigned_abs()));
        if a != $native::MIN {
          assert_eq!(neg(&limbs_a), limbs_of(-a));
          assert_eq!(abs(&limbs_a), limbs_of(a.abs()));
        }
        assert_eq!(is_one(&limbs_a), a == 1);
        assert_eq!(signum(&limbs_a), limbs_of(a.signum()));
        let (negated, overflow) = a.overflowing_neg();
        assert_eq!(
          overflowing_neg_bits(&limbs_a, bits),
          (limbs_of(negated), overflow)
        );
        for &b in &samples {
          let limbs_b: [Limb; $n] = limbs_of(b);
          assert_eq!(cmp(&limbs_a, &limbs_b), a.cmp(&b));
          assert_eq!(le(&limbs_a, &limbs_b), a <= b);
          let (sum, overflow) = a.overflowing_add(b);
          assert_eq!(
            overflowing_add_bits(&limbs_a, &limbs_b, bits),
            (limbs_of(sum), overflow)
          );
          let (difference, overflow) = a.overflowing_sub(b);
          assert_eq!(
            overflowing_sub_bits(&limbs_a, &limbs_b, bits),
            (limbs_of(difference), overflow)
          );
          let (product, overflow) = a.overflowing_mul(b);
          assert_eq!(
            overflowing_mul_bits(&limbs_a, &limbs_b, bits),
            (limbs_of(product), overflow)
          );
          assert_eq!(
            checked_add(&limbs_a, &limbs_b),
            a.checked_add(b).map(limbs_of)
          );
          assert_eq!(
            checked_sub(&limbs_a, &limbs_b),
            a.checked_sub(b).map(limbs_of)
          );
          if let Some(product) = a.checked_mul(b) {
            assert_eq!(mul(&limbs_a, &limbs_b), limbs_of(product));
          }
          // As for `/` and `%`, a zero divisor and `MIN / -1` are excluded.
          if b != 0 && (a, b) != ($native::MIN, -1) {
            assert_eq!(
              div_rem(&limbs_a, &limbs_b),
              (limbs_of(a / b), limbs_of(a % b)),
              "{a} / {b}"
            );
          }
        }
        for exp in 0..=max_exp {
          let power = $native::pow(10, exp);
          assert_eq!(
            checked_mul_pow10(&limbs_a, exp),
            a.checked_mul(power).map(limbs_of)
          );
          if let Some(product) = a.checked_mul(power) {
            assert_eq!(mul_pow10(&limbs_a, exp), limbs_of(product));
          }
          // As with `/` and `%`, the quotient and the remainder have the sign of `a`.
          assert_eq!(
            div_rem_pow10(&limbs_a, exp),
            (limbs_of(a / power), limbs_of(a % power))
          );
        }

        let mut buf = [0xff; DIGIT_BUF_LEN];
        let start = write_digits(&limbs_a, &mut buf);
        assert_eq!(&buf[start..], a.unsigned_abs().to_string().as_bytes());
        if a >= 0 {
          let digits = a.to_string();
          assert_eq!(
            push_digits(&[0; $n], digits.as_bytes(), 0, digits.len()),
            limbs_a
          );
        }
      }
    }
  };
}

pub(crate) use native_oracle;
