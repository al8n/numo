//! `quickcheck` support: generation and shrinking for a decimal and a rounding mode.
//!
//! The choices come from `Gen::choose` over an identity table, which is uniform and stable across
//! `quickcheck` 1.x, not from the `Arbitrary` impls of the integers, which return extreme values a
//! tenth of the time. Shrinking returns the boxed iterator that the trait requires, the only
//! allocation in the crate.

use alloc::boxed::Box;

use ::quickcheck::{Arbitrary, Gen};

use crate::{
  Decimal, RoundingMode,
  layout::{Layout, Storage},
  sample::{self, Chain, MODES, Source},
};

/// `0..=255` in order, so that `Gen::choose` picks a uniform value below `n` from `INDEX[..n]`.
const INDEX: [u8; 256] = {
  let mut index = [0; 256];
  let mut i = 0;
  while i < index.len() {
    index[i] = i as u8;
    i += 1;
  }
  index
};

/// A source that draws from a `quickcheck` generator.
struct Choices<'g>(&'g mut Gen);

impl Source for Choices<'_> {
  fn below(&mut self, n: u32) -> u32 {
    // `choose` returns `None` only for an empty slice, and `n` is at least one.
    u32::from(*self.0.choose(&INDEX[..n as usize]).unwrap_or(&0))
  }
}

#[cfg_attr(docsrs, doc(cfg(feature = "quickcheck")))]
impl<I, F, R> Arbitrary for Decimal<I, F, R>
where
  R: Storage + 'static,
  I: 'static,
  F: 'static,
  (I, F): Layout<R>,
{
  /// Draws a decimal as the crate documentation describes. The size of `g` is not used.
  fn arbitrary(g: &mut Gen) -> Self {
    let coeff = sample::coefficient::<I, F, R>(&mut Choices(g));
    // The coefficient has at most the digits of the type.
    Decimal::from_coeff(coeff)
  }

  /// Returns zero, then the magnitude of a negative value, then the value truncated toward zero to
  /// its first digit, its first two digits, and so on: each simpler than the value, which is never
  /// among them. The iterator is boxed, as the trait requires.
  fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
    Box::new(Chain::<I, F, R>::new(self.coeff()))
  }
}

#[cfg_attr(docsrs, doc(cfg(feature = "quickcheck")))]
impl Arbitrary for RoundingMode {
  /// Chooses a mode uniformly.
  fn arbitrary(g: &mut Gen) -> Self {
    sample::mode(&mut Choices(g))
  }

  /// Returns the modes before this one in the order `Down`, `Up`, `Ceiling`, `Floor`, `HalfUp`,
  /// `HalfDown`, `HalfEven`, so shrinking moves toward [`Down`](RoundingMode::Down).
  fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
    let mode = *self;
    let before = MODES
      .iter()
      .copied()
      .take_while(move |&other| other != mode);
    Box::new(before)
  }
}

#[cfg(test)]
mod tests;
