//! Lower layer for `[u8; N]`: unsigned integers of `N` limbs of `B = 8` bits, least significant
//! first.
//!
//! `limb_layer!` generates the layer from the two functions below, which compute in `u16`.

use super::limb_layer::limb_layer;

/// `a * b + carry + add` as two limbs, the low one first. It is at most
/// `(2^8 - 1)^2 + 2 * (2^8 - 1) = 2^16 - 1`, so it does not overflow.
#[inline]
const fn carrying_mul_add(a: u8, b: u8, carry: u8, add: u8) -> (u8, u8) {
  let wide = a as u16 * b as u16 + carry as u16 + add as u16;
  (wide as u8, (wide >> 8) as u8)
}

/// Quotient and remainder of `high * 2^8 + low` by `divisor`. Requires `high < divisor`, so that
/// the quotient fits a limb. A zero divisor fails as native division does.
#[inline]
const fn div_rem_wide(high: u8, low: u8, divisor: u8) -> (u8, u8) {
  debug_assert!(
    divisor == 0 || high < divisor,
    "the quotient overflows a limb"
  );
  let wide = (high as u16) << 8 | low as u16;
  ((wide / divisor as u16) as u8, (wide % divisor as u16) as u8)
}

limb_layer!(unsigned limb: u8, carrying_mul_add: carrying_mul_add, div_rem_wide: div_rem_wide);

// Pins the derived step to 2 digits, the most that a `u8` holds, and the bound on the digits of a
// limb to 3. A smaller step or a larger bound would give the same results with more limb operations
// or more stack, so no test would notice it.
const _: () = assert!(MAX_STEP == 2 && TEN_POW == 100 && DIGITS_PER_LIMB == 3);

#[cfg(test)]
mod tests;
