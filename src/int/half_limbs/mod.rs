//! The two primitives of `limb_layer!` for a limb type with no native integer of twice its width.
//!
//! A limb of `B` bits is split into two digits of `H = B / 2` bits, each held in the limb type, so
//! that every product of two digits and every quotient by a digit is an operation of the limb type.
//! `unsigned::b128` uses it for `u128`; the tests instantiate it for `u8` to `u64` and check it
//! against native integers of twice the width.

/// Defines, in the module that invokes it, the primitives `carrying_mul_add` and `div_rem_wide` of
/// `limb_layer!` for limbs of type `$limb`, with the contracts that `limb_layer!` states, and the
/// constants and the division step that they use.
// The items name `$limb`, never `Limb`, which `limb_layer!` defines in the same module.
macro_rules! half_limbs {
  ($limb:ident) => {
    /// `H = B / 2`, the number of bits of a digit.
    const HALF: u32 = $limb::BITS / 2;

    /// `2^H - 1`, the largest digit, and the mask of a limb's low half.
    const DIGIT_MAX: $limb = $limb::MAX >> HALF;

    /// `a * b + carry + add` as two limbs, the low one first. It is at most
    /// `(2^B - 1)^2 + 2 * (2^B - 1) = 2^(2B) - 1`, so it does not overflow.
    #[inline]
    const fn carrying_mul_add(a: $limb, b: $limb, carry: $limb, add: $limb) -> ($limb, $limb) {
      let (a0, a1) = (a & DIGIT_MAX, a >> HALF);
      let (b0, b1) = (b & DIGIT_MAX, b >> HALF);
      // Each product of two digits is below `2^B`.
      let cross01 = a0 * b1;
      let cross10 = a1 * b0;
      // Bits 0 to H - 1: at most `(2^H - 1)^2 + 2 * (2^H - 1) = 2^B - 1`.
      let column0 = a0 * b0 + (carry & DIGIT_MAX) + (add & DIGIT_MAX);
      // Bits H to B - 1, with the carry of `column0`: at most `5 * (2^H - 1)`.
      let column1 = (column0 >> HALF)
        + (cross01 & DIGIT_MAX)
        + (cross10 & DIGIT_MAX)
        + (carry >> HALF)
        + (add >> HALF);
      // Bits B and up: below `2^B`, because the whole sum is below `2^(2B)`.
      let high = a1 * b1 + (cross01 >> HALF) + (cross10 >> HALF) + (column1 >> HALF);
      ((column1 << HALF) | (column0 & DIGIT_MAX), high)
    }

    /// Quotient and remainder of `top * 2^H + digit` by `divisor`: three digits by two, giving
    /// one digit. Requires `top < divisor`, `digit <= DIGIT_MAX` and the top bit of `divisor`
    /// set.
    ///
    /// The estimate from the top digit of `divisor` exceeds the quotient by at most two. With a
    /// divisor of two digits, the test with its low digit is exact, so the corrected estimate is
    /// the quotient and nothing is added back.
    #[inline]
    const fn div_rem_step(top: $limb, digit: $limb, divisor: $limb) -> ($limb, $limb) {
      let (divisor1, divisor0) = (divisor >> HALF, divisor & DIGIT_MAX);
      // `divisor1 >= 2^(H - 1)`, so the estimate is at most `2^H + 1`, and `rest` is below
      // `divisor1`.
      let mut estimate = top / divisor1;
      let mut rest = top - estimate * divisor1;
      // Corrects while the estimate is not a digit, or `estimate * divisor > top * 2^H + digit`,
      // which is `estimate * divisor0 > rest * 2^H + digit`. Once `rest` exceeds a digit, that
      // cannot hold.
      while estimate > DIGIT_MAX || estimate * divisor0 > (rest << HALF | digit) {
        estimate -= 1;
        rest += divisor1;
        if rest > DIGIT_MAX {
          break;
        }
      }
      // The remainder is below `divisor`, so computing it modulo `2^B` is exact.
      (
        estimate,
        (top << HALF | digit).wrapping_sub(estimate.wrapping_mul(divisor)),
      )
    }

    /// Quotient and remainder of `high * 2^B + low` by `divisor`. Requires `high < divisor`, so
    /// that the quotient fits a limb. A zero divisor fails as native division does.
    #[inline]
    const fn div_rem_wide(high: $limb, low: $limb, divisor: $limb) -> ($limb, $limb) {
      debug_assert!(
        divisor == 0 || high < divisor,
        "the quotient overflows a limb"
      );
      // Native division handles a high limb of zero, and fails on a zero divisor whatever `high`
      // is.
      if high == 0 || divisor == 0 {
        return (low / divisor, low % divisor);
      }
      // `high < divisor`, so the shift loses no bit of `high`, and the top bits of `low` move into
      // it. A shift of zero would shift `low` right by `B` bits, which overflows.
      let shift = divisor.leading_zeros();
      let divisor = divisor << shift;
      let (high, low) = if shift == 0 {
        (high, low)
      } else {
        (high << shift | low >> ($limb::BITS - shift), low << shift)
      };
      let (quotient1, rest) = div_rem_step(high, low >> HALF, divisor);
      let (quotient0, rest) = div_rem_step(rest, low & DIGIT_MAX, divisor);
      (quotient1 << HALF | quotient0, rest >> shift)
    }
  };
}

pub(crate) use half_limbs;

#[cfg(test)]
pub(super) mod tests;
