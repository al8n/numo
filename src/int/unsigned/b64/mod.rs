//! Lower layer for `[u64; N]`: unsigned integers of `N` limbs of `B = 64` bits, least significant
//! first.
//!
//! The functions mirror the native lower layer, generic over `N >= 1`: they have the same names
//! and contracts, except that some preconditions are weaker, as each function states. Where native
//! arithmetic would overflow on a violated precondition, they wrap modulo `2^(B * N)` and fail a
//! debug assertion.
//!
//! `limb_layer!` generates the layer from the two functions below, which compute in `u128`.

use super::limb_layer::limb_layer;

/// `a * b + carry + add` as two limbs, the low one first. It is at most
/// `(2^64 - 1)^2 + 2 * (2^64 - 1) = 2^128 - 1`, so it does not overflow.
#[inline]
const fn carrying_mul_add(a: u64, b: u64, carry: u64, add: u64) -> (u64, u64) {
  let wide = a as u128 * b as u128 + carry as u128 + add as u128;
  (wide as u64, (wide >> 64) as u64)
}

/// Quotient and remainder of `high * 2^64 + low` by `divisor`. Requires `high < divisor`, so that
/// the quotient fits a limb. A zero divisor fails as native division does.
#[inline]
const fn div_rem_wide(high: u64, low: u64, divisor: u64) -> (u64, u64) {
  debug_assert!(
    divisor == 0 || high < divisor,
    "the quotient overflows a limb"
  );
  let wide = (high as u128) << 64 | low as u128;
  (
    (wide / divisor as u128) as u64,
    (wide % divisor as u128) as u64,
  )
}

limb_layer!(unsigned limb: u64, carrying_mul_add: carrying_mul_add, div_rem_wide: div_rem_wide);

// Pins the derived step to 19 digits, the most that a `u64` holds, and the bound on the digits of
// a limb to 20. A smaller step or a larger bound would give the same results with more limb
// operations or more stack, so no test would notice it.
const _: () =
  assert!(MAX_STEP == 19 && TEN_POW == 10_000_000_000_000_000_000 && DIGITS_PER_LIMB == 20);

#[cfg(test)]
mod tests;
