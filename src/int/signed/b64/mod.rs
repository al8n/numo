//! Upper layer for `[u64; N]`: two's complement integers of `N` limbs of `B = 64` bits, least
//! significant first.
//!
//! The top bit of the top limb is the sign. The functions mirror the native upper layer, generic
//! over `N >= 1`: they have the same names and contracts, except that some preconditions are
//! weaker, as each function states, and they handle the sign and pass magnitude work to the lower
//! layer through `unsigned_abs`. "Representable" means within the signed range, from
//! `MIN = -2^(B * N - 1)` to `2^(B * N - 1) - 1`.
//!
//! `limb_layer!` generates the layer, over `unsigned::b64`.

use super::limb_layer::limb_layer;

limb_layer!(signed limb: u64, signed: i64, unsigned: b64);

#[cfg(test)]
mod tests;
