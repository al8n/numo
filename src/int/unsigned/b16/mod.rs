//! Lower layer for `[u16; N]`: unsigned integers of `N` limbs of `B = 16` bits, least significant
//! first.
//!
//! `limb_layer!` generates the layer from the two functions below, which compute in `u32`.

use super::limb_layer::limb_layer;

/// `a * b + carry + add` as two limbs, the low one first. It is at most
/// `(2^16 - 1)^2 + 2 * (2^16 - 1) = 2^32 - 1`, so it does not overflow.
#[inline]
const fn carrying_mul_add(a: u16, b: u16, carry: u16, add: u16) -> (u16, u16) {
  let wide = a as u32 * b as u32 + carry as u32 + add as u32;
  (wide as u16, (wide >> 16) as u16)
}

/// Quotient and remainder of `high * 2^16 + low` by `divisor`. Requires `high < divisor`, so that
/// the quotient fits a limb. A zero divisor fails as native division does.
#[inline]
const fn div_rem_wide(high: u16, low: u16, divisor: u16) -> (u16, u16) {
  debug_assert!(
    divisor == 0 || high < divisor,
    "the quotient overflows a limb"
  );
  let wide = (high as u32) << 16 | low as u32;
  (
    (wide / divisor as u32) as u16,
    (wide % divisor as u32) as u16,
  )
}

limb_layer!(unsigned limb: u16, carrying_mul_add: carrying_mul_add, div_rem_wide: div_rem_wide);

// Pins the derived step to 4 digits, the most that a `u16` holds, and the bound on the digits of a
// limb to 5. A smaller step or a larger bound would give the same results with more limb operations
// or more stack, so no test would notice it.
const _: () = assert!(MAX_STEP == 4 && TEN_POW == 10_000 && DIGITS_PER_LIMB == 5);

#[cfg(test)]
mod tests;
