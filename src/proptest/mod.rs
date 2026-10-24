//! `proptest` support: [`Arbitrary`] for [`Decimal`] and [`RoundingMode`], and the strategy and
//! value tree of a decimal.
//!
//! `any::<Decimal<I, F, R>>()` returns a [`DecimalStrategy`], which draws values from the runner's
//! random number generator as the [crate documentation](crate#arbitrary-proptest-and-quickcheck)
//! describes. Its [`DecimalValueTree`] shrinks a failing value by decimal truncation: it tries
//! zero, then the magnitude of a negative value, then the value truncated toward zero to its first
//! digit, its first two digits, and so on, and continues from the first candidate that fails.
//! Shrinking stops at a failing value none of whose candidates fail, which is not necessarily the
//! smallest failing value. `any::<RoundingMode>()` chooses a mode uniformly and shrinks toward
//! [`Down`](RoundingMode::Down).
//!
//! The feature enables the `std` feature of `proptest`.

use core::{fmt, marker::PhantomData};

use ::proptest::{
  arbitrary::Arbitrary,
  sample::{Select, select},
  strategy::{NewTree, Strategy, ValueTree},
  test_runner::{Reason, TestRunner},
};

use crate::{
  Decimal, RoundingMode,
  layout::{Layout, Storage},
  sample::{self, Chain, DecimalMarker, MODES, Source},
};

/// The strategy of [`Decimal<I, F, R>`](Decimal), which `any::<Decimal<I, F, R>>()` returns.
///
/// It draws values as the [crate documentation](crate#arbitrary-proptest-and-quickcheck)
/// describes, and its value tree is a [`DecimalValueTree`].
#[must_use = "strategies do nothing unless used"]
pub struct DecimalStrategy<I, F, R> {
  decimal: DecimalMarker<I, F, R>,
}

impl<I, F, R> fmt::Debug for DecimalStrategy<I, F, R> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("DecimalStrategy")
  }
}

impl<I, F, R> Clone for DecimalStrategy<I, F, R> {
  fn clone(&self) -> Self {
    *self
  }
}

impl<I, F, R> Copy for DecimalStrategy<I, F, R> {}

impl<I, F, R> Strategy for DecimalStrategy<I, F, R>
where
  R: Storage,
  (I, F): Layout<R>,
{
  type Tree = DecimalValueTree<I, F, R>;
  type Value = Decimal<I, F, R>;

  fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
    let mut source = Runner {
      runner,
      reason: None,
    };
    let coeff = sample::coefficient::<I, F, R>(&mut source);
    match source.reason {
      Some(reason) => Err(reason),
      None => Ok(DecimalValueTree::new(coeff)),
    }
  }
}

/// A source that draws from a runner through integer range strategies, whose interface is stable
/// across `proptest` 1.x, unlike the random number generator traits beneath them. It keeps the
/// first reason that drawing reports, which the strategy returns instead of a tree.
struct Runner<'a> {
  runner: &'a mut TestRunner,
  reason: Option<Reason>,
}

impl Source for Runner<'_> {
  fn below(&mut self, n: u32) -> u32 {
    match (0..n as usize).new_tree(self.runner) {
      Ok(tree) => tree.current() as u32,
      Err(reason) => {
        if self.reason.is_none() {
          self.reason = Some(reason);
        }
        0
      }
    }
  }
}

/// The value tree of [`Decimal<I, F, R>`](Decimal), which shrinks by decimal truncation.
///
/// The candidates for a value `v` with `d` digits in its coefficient are, in order, zero, then the
/// magnitude of `v` if `v` is negative, then `v` truncated toward zero to its first digit, its
/// first two digits, and so on up to `d - 1` digits, each simpler than `v`. The tree tries them in
/// order, and when one fails, it continues from that candidate, skipping zero, which it has
/// already tried. It stops at a failing value none of whose candidates fail, which is not
/// necessarily the smallest failing value.
pub struct DecimalValueTree<I, F, R> {
  /// The last value known to fail.
  parent: R,
  current: R,
  /// The index of `current` among the candidates for `parent`, or `None` when `current` is
  /// `parent`.
  cursor: Option<u8>,
  /// `true` once every candidate for `parent` has passed.
  exhausted: bool,
  decimal: DecimalMarker<I, F, R>,
}

impl<I, F, R> DecimalValueTree<I, F, R>
where
  R: Copy,
{
  /// Returns a tree that starts from `repr`, a coefficient in the range of `Decimal<I, F, R>`.
  pub(crate) const fn new(repr: R) -> Self {
    Self {
      parent: repr,
      current: repr,
      cursor: None,
      exhausted: false,
      decimal: PhantomData,
    }
  }
}

impl<I, F, R> DecimalValueTree<I, F, R>
where
  R: Storage,
{
  /// Moves to the candidate for `parent` at `index` and returns `true`, or, past the last
  /// candidate, back to `parent`, which is then exhausted, and returns `false`.
  fn move_to(&mut self, index: u8) -> bool {
    match Chain::<I, F, R>::new(self.parent).get(u32::from(index)) {
      Some(candidate) => {
        self.current = candidate;
        self.cursor = Some(index);
        true
      }
      None => {
        self.current = self.parent;
        self.cursor = None;
        self.exhausted = true;
        false
      }
    }
  }
}

impl<I, F, R> fmt::Debug for DecimalValueTree<I, F, R>
where
  R: fmt::Debug,
{
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("DecimalValueTree")
      .field("parent", &self.parent)
      .field("current", &self.current)
      .field("cursor", &self.cursor)
      .field("exhausted", &self.exhausted)
      .finish()
  }
}

impl<I, F, R> Clone for DecimalValueTree<I, F, R>
where
  R: Copy,
{
  fn clone(&self) -> Self {
    *self
  }
}

impl<I, F, R> Copy for DecimalValueTree<I, F, R> where R: Copy {}

impl<I, F, R> ValueTree for DecimalValueTree<I, F, R>
where
  R: Storage,
  (I, F): Layout<R>,
{
  type Value = Decimal<I, F, R>;

  fn current(&self) -> Decimal<I, F, R> {
    // Every candidate has at most the digits of the value it was made from.
    Decimal::from_coeff(self.current)
  }

  fn simplify(&mut self) -> bool {
    let first = if self.cursor.is_some() {
      // The candidate failed, so the next level shrinks it. Zero comes first among the
      // candidates of every value, and the first level has tried it.
      self.parent = self.current;
      self.exhausted = false;
      1
    } else if self.exhausted {
      return false;
    } else {
      0
    };
    self.move_to(first)
  }

  fn complicate(&mut self) -> bool {
    match self.cursor {
      Some(cursor) => {
        // Past the last candidate, this returns to the parent, which has failed, so the state
        // changes either way.
        self.move_to(cursor + 1);
        true
      }
      None => false,
    }
  }
}

#[cfg_attr(docsrs, doc(cfg(feature = "proptest")))]
impl<I, F, R> Arbitrary for Decimal<I, F, R>
where
  R: Storage,
  (I, F): Layout<R>,
{
  type Parameters = ();
  type Strategy = DecimalStrategy<I, F, R>;

  fn arbitrary_with(_: ()) -> DecimalStrategy<I, F, R> {
    DecimalStrategy {
      decimal: PhantomData,
    }
  }
}

#[cfg_attr(docsrs, doc(cfg(feature = "proptest")))]
impl Arbitrary for RoundingMode {
  type Parameters = ();
  type Strategy = Select<RoundingMode>;

  /// Chooses a mode uniformly, shrinking toward [`Down`](RoundingMode::Down) in the order `Down`,
  /// `Up`, `Ceiling`, `Floor`, `HalfUp`, `HalfDown`, `HalfEven`.
  fn arbitrary_with(_: ()) -> Select<RoundingMode> {
    select(&MODES[..])
  }
}

#[cfg(test)]
mod tests;
