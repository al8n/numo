//! Upper layer for `[u16; N]`: two's complement integers of `N` limbs of `B = 16` bits, least
//! significant first.
//!
//! `limb_layer!` generates the layer, over `unsigned::b16`.

use super::limb_layer::limb_layer;

limb_layer!(signed limb: u16, signed: i16, unsigned: b16);

#[cfg(test)]
mod tests;
