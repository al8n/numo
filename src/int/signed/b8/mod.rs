//! Upper layer for `[u8; N]`: two's complement integers of `N` limbs of `B = 8` bits, least
//! significant first.
//!
//! `limb_layer!` generates the layer, over `unsigned::b8`.

use super::limb_layer::limb_layer;

limb_layer!(signed limb: u8, signed: i8, unsigned: b8);

#[cfg(test)]
mod tests;
