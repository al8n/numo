//! Lower layer for `[u128; N]`: unsigned integers of `N` limbs of `B = 128` bits, least
//! significant first.
//!
//! No native integer has 256 bits, so `half_limbs!` computes the two functions that `limb_layer!`
//! takes from the 64-bit halves of a limb.

use super::{half_limbs::half_limbs, limb_layer::limb_layer};

half_limbs!(u128);

limb_layer!(unsigned limb: u128, carrying_mul_add: carrying_mul_add, div_rem_wide: div_rem_wide);

// Pins the derived step to 38 digits, the most that a `u128` holds, and the bound on the digits of
// a limb to 39. A smaller step or a larger bound would give the same results with more limb
// operations or more stack, so no test would notice it.
const _: () = assert!(
  MAX_STEP == 38
    && TEN_POW == 100_000_000_000_000_000_000_000_000_000_000_000_000
    && DIGITS_PER_LIMB == 39
);

#[cfg(test)]
mod tests;
