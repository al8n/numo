use crate::int::limb_layer::tests::{limb_layer_tests, native_oracle, other_limb_tests};

// The tests that every instance shares, with their helpers and the imports that the tests below
// use as well.
limb_layer_tests!(unsigned);

// The helpers of the tests below, which compare the layer with native integers and with the layer
// for `[u64; N]`.
other_limb_tests!(unsigned);

native_oracle!(unsigned one_limb_matches_u128, u128, 1);

/// Steps whose window has the top divisor limb on top, where the quotient of the window's top two
/// limbs by that limb is `2^128` or `2^128 + 1`.
#[test]
fn division_estimates_reach_two_pow_128() {
  division_with_equal_top_limbs();
}

#[test]
fn split_limbs_match_the_u64_layer() {
  matches_the_u64_layer::<1, 2>();
  matches_the_u64_layer::<2, 4>();
  matches_the_u64_layer::<3, 6>();
  matches_the_u64_layer::<4, 8>();
}

/// The primitives that `half_limbs!` computes for `u128`: the tests that every instance shares, and
/// tests against `num-bigint` and against the layer for `[u64; N]`.
mod half_limbs {
  use super::*;
  use crate::int::half_limbs::tests::half_limbs_tests;

  half_limbs_tests!(u128);

  /// Full-width operands, and divisors with every number of leading zeros, against `num-bigint`.
  #[test]
  #[cfg_attr(
    all(miri, target_pointer_width = "32"),
    ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
  )]
  fn primitives_match_big_integers() {
    let mut rng = Rng(0x3c6e_f372_fe94_f82b);
    let count = if cfg!(miri) { 1 } else { 200 };
    for shift in 0..B {
      for _ in 0..count {
        let (a, b, carry, add) = (rng.limb(), rng.limb(), rng.limb(), rng.limb());
        let (low, high) = carrying_mul_add(a, b, carry, add);
        assert_eq!(
          bigint::from_limbs(&[low, high], false),
          BigInt::from(a) * BigInt::from(b) + BigInt::from(carry) + BigInt::from(add),
          "{a:#x} * {b:#x} + {carry:#x} + {add:#x}"
        );

        let divisor = (rng.limb() | HIGH) >> shift;
        let (high, low) = (rng.limb() % divisor, rng.limb());
        let (quotient, remainder) = div_rem_wide(high, low, divisor);
        let (dividend, big_divisor) = (
          bigint::from_limbs(&[low, high], false),
          BigInt::from(divisor),
        );
        assert_eq!(
          (BigInt::from(quotient), BigInt::from(remainder)),
          (&dividend / &big_divisor, &dividend % &big_divisor),
          "{dividend:#x} / {divisor:#x}"
        );
      }
    }
  }

  /// Full-width operands, and divisors with every number of leading zeros, against the layer for
  /// `[u64; N]`: a product and two sums for `carrying_mul_add`, and a division of four limbs by
  /// two for `div_rem_wide`. It allocates nothing, so that it also runs on 32-bit Miri.
  #[test]
  fn primitives_match_the_u64_layer() {
    use crate::int::unsigned::b64;

    /// `value` in four limbs of 64 bits, the top two zero.
    fn split(value: u128) -> [u64; 4] {
      [value as u64, (value >> 64) as u64, 0, 0]
    }

    let mut rng = Rng(0xa54f_f53a_5f1d_36f1);
    let count = if cfg!(miri) { 1 } else { 200 };
    for shift in 0..B {
      for _ in 0..count {
        let (a, b, carry, add) = (rng.limb(), rng.limb(), rng.limb(), rng.limb());
        let (low, high) = carrying_mul_add(a, b, carry, add);
        let ([low0, low1, ..], [high0, high1, ..]) = (split(low), split(high));
        // The sum is below `2^256`, so neither addition carries out of four limbs.
        let sum = b64::checked_add(&b64::mul(&split(a), &split(b)), &split(carry))
          .and_then(|sum| b64::checked_add(&sum, &split(add)));
        assert_eq!(
          sum,
          Some([low0, low1, high0, high1]),
          "{a:#x} * {b:#x} + {carry:#x} + {add:#x}"
        );

        let divisor = (rng.limb() | HIGH) >> shift;
        let (high, low) = (rng.limb() % divisor, rng.limb());
        let (quotient, remainder) = div_rem_wide(high, low, divisor);
        let ([low0, low1, ..], [high0, high1, ..]) = (split(low), split(high));
        assert_eq!(
          b64::div_rem(&[low0, low1, high0, high1], &split(divisor)),
          (split(quotient), split(remainder)),
          "{high:#x}:{low:#x} / {divisor:#x}"
        );
      }
    }
  }
}
