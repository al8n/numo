//! Lower layer for `[u32; N]`: unsigned integers of `N` limbs of `B = 32` bits, least significant
//! first.
//!
//! `limb_layer!` generates the layer from the two functions below, which compute in `u64`.

use super::limb_layer::limb_layer;

/// `a * b + carry + add` as two limbs, the low one first. It is at most
/// `(2^32 - 1)^2 + 2 * (2^32 - 1) = 2^64 - 1`, so it does not overflow.
#[inline]
const fn carrying_mul_add(a: u32, b: u32, carry: u32, add: u32) -> (u32, u32) {
  let wide = a as u64 * b as u64 + carry as u64 + add as u64;
  (wide as u32, (wide >> 32) as u32)
}

/// Quotient and remainder of `high * 2^32 + low` by `divisor`. Requires `high < divisor`, so that
/// the quotient fits a limb. A zero divisor fails as native division does.
#[inline]
const fn div_rem_wide(high: u32, low: u32, divisor: u32) -> (u32, u32) {
  debug_assert!(
    divisor == 0 || high < divisor,
    "the quotient overflows a limb"
  );
  let wide = (high as u64) << 32 | low as u64;
  (
    (wide / divisor as u64) as u32,
    (wide % divisor as u64) as u32,
  )
}

limb_layer!(unsigned limb: u32, carrying_mul_add: carrying_mul_add, div_rem_wide: div_rem_wide);

// Pins the derived step to 9 digits, the most that a `u32` holds, and the bound on the digits of a
// limb to 10. A smaller step or a larger bound would give the same results with more limb
// operations or more stack, so no test would notice it.
const _: () = assert!(MAX_STEP == 9 && TEN_POW == 1_000_000_000 && DIGITS_PER_LIMB == 10);

#[cfg(test)]
mod tests;
