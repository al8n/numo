//! The operators of [`Decimal`]: `+` and `-` on two values and on two references, `+=` and `-=`,
//! and `Sum` over values and over references.
//!
//! Each has one implementation for every storage. It computes in the inline twin of its operands,
//! through the [`FixedDecimal`] of the twin, whose `checked_add`, `checked_sub` and `zero` each
//! inline storage implements, and stores the result once. An operator returns what the checked
//! method returns, or panics where that method returns `None`, before it stores anything. It
//! computes nothing itself, so no build setting changes what it does.

use core::{
  iter::Sum,
  ops::{Add, AddAssign, Sub, SubAssign},
};

use super::{Decimal, FixedDecimal};
use crate::layout::{MutableStorage, Storage, Token};

/// The panics of the operators of a decimal, with the messages of the native integers.
pub(super) mod overflow {
  /// Panics as the native `+` does where it checks for overflow.
  #[cold]
  #[track_caller]
  pub(crate) const fn add() -> ! {
    panic!("attempt to add with overflow")
  }

  /// Panics as the native `-` does where it checks for overflow.
  #[cold]
  #[track_caller]
  pub(crate) const fn sub() -> ! {
    panic!("attempt to subtract with overflow")
  }
}

impl<I, F, R> Add for Decimal<I, F, R>
where
  R: Storage,
  Decimal<I, F, R::Inline>: FixedDecimal,
{
  type Output = Self;

  /// Adds two values.
  ///
  /// # Panics
  ///
  /// Panics if the sum is outside the range of the type, in every build, whatever
  /// `overflow-checks` is. [`checked_add`](Decimal::checked_add) returns `None` instead.
  #[inline]
  #[track_caller]
  fn add(mut self, rhs: Self) -> Self {
    let sum = match FixedDecimal::checked_add(&self.to_inline(), &rhs.to_inline()) {
      Some(sum) => sum,
      None => overflow::add(),
    };
    self.repr.set_twin(sum.repr, Token::new());
    self
  }
}

impl<I, F, R> Add<&Decimal<I, F, R>> for &Decimal<I, F, R>
where
  R: Storage,
  Decimal<I, F, R::Inline>: FixedDecimal,
{
  type Output = Decimal<I, F, R>;

  /// Adds two values by reference.
  ///
  /// # Panics
  ///
  /// Panics if the sum is outside the range of the type, in every build, whatever
  /// `overflow-checks` is. [`checked_add`](Decimal::checked_add) returns `None` instead.
  #[inline]
  #[track_caller]
  fn add(self, rhs: &Decimal<I, F, R>) -> Decimal<I, F, R> {
    match FixedDecimal::checked_add(&self.to_inline(), &rhs.to_inline()) {
      Some(sum) => Decimal::from_coeff(R::from_twin(sum.repr, Token::new())),
      None => overflow::add(),
    }
  }
}

// The assignments store into the storage of their target, so they need a storage that holds the
// coefficient in place.
impl<I, F, R> AddAssign for Decimal<I, F, R>
where
  R: MutableStorage,
  Decimal<I, F, R::Inline>: FixedDecimal,
{
  /// Adds `rhs` to `self`.
  ///
  /// # Panics
  ///
  /// Panics if the sum is outside the range of the type, in every build, whatever
  /// `overflow-checks` is, and leaves `self` unchanged.
  /// [`checked_add`](Decimal::checked_add) returns `None` instead.
  #[inline]
  #[track_caller]
  fn add_assign(&mut self, rhs: Self) {
    match FixedDecimal::checked_add(&self.to_inline(), &rhs.to_inline()) {
      Some(sum) => self.repr.set_twin(sum.repr, Token::new()),
      None => overflow::add(),
    }
  }
}

impl<I, F, R> Sub for Decimal<I, F, R>
where
  R: Storage,
  Decimal<I, F, R::Inline>: FixedDecimal,
{
  type Output = Self;

  /// Subtracts `rhs` from `self`.
  ///
  /// # Panics
  ///
  /// Panics if the difference is outside the range of the type, which for an unsigned storage is
  /// every difference below zero, in every build, whatever `overflow-checks` is.
  /// [`checked_sub`](Decimal::checked_sub) returns `None` instead.
  #[inline]
  #[track_caller]
  fn sub(mut self, rhs: Self) -> Self {
    let difference = match FixedDecimal::checked_sub(&self.to_inline(), &rhs.to_inline()) {
      Some(difference) => difference,
      None => overflow::sub(),
    };
    self.repr.set_twin(difference.repr, Token::new());
    self
  }
}

impl<I, F, R> Sub<&Decimal<I, F, R>> for &Decimal<I, F, R>
where
  R: Storage,
  Decimal<I, F, R::Inline>: FixedDecimal,
{
  type Output = Decimal<I, F, R>;

  /// Subtracts `rhs` from `self`, by reference.
  ///
  /// # Panics
  ///
  /// Panics if the difference is outside the range of the type, which for an unsigned storage is
  /// every difference below zero, in every build, whatever `overflow-checks` is.
  /// [`checked_sub`](Decimal::checked_sub) returns `None` instead.
  #[inline]
  #[track_caller]
  fn sub(self, rhs: &Decimal<I, F, R>) -> Decimal<I, F, R> {
    match FixedDecimal::checked_sub(&self.to_inline(), &rhs.to_inline()) {
      Some(difference) => Decimal::from_coeff(R::from_twin(difference.repr, Token::new())),
      None => overflow::sub(),
    }
  }
}

impl<I, F, R> SubAssign for Decimal<I, F, R>
where
  R: MutableStorage,
  Decimal<I, F, R::Inline>: FixedDecimal,
{
  /// Subtracts `rhs` from `self`.
  ///
  /// # Panics
  ///
  /// Panics if the difference is outside the range of the type, which for an unsigned storage is
  /// every difference below zero, in every build, whatever `overflow-checks` is, and leaves
  /// `self` unchanged. [`checked_sub`](Decimal::checked_sub) returns `None` instead.
  #[inline]
  #[track_caller]
  fn sub_assign(&mut self, rhs: Self) {
    match FixedDecimal::checked_sub(&self.to_inline(), &rhs.to_inline()) {
      Some(difference) => self.repr.set_twin(difference.repr, Token::new()),
      None => overflow::sub(),
    }
  }
}

impl<I, F, R> Sum for Decimal<I, F, R>
where
  R: Storage,
  Decimal<I, F, R::Inline>: FixedDecimal,
{
  /// Adds the values, from zero.
  ///
  /// # Panics
  ///
  /// Panics if a partial sum is outside the range of the type, in every build, whatever
  /// `overflow-checks` is, also where the whole sum would be in range. A fold with
  /// [`checked_add`](Decimal::checked_add) returns `None` instead.
  fn sum<T: Iterator<Item = Self>>(mut iter: T) -> Self {
    // The running sum is the twin of the first value, which then stores the total.
    let Some(mut first) = iter.next() else {
      return Self::from_inline(FixedDecimal::zero());
    };
    let total = iter.fold(first.to_inline(), |sum, value| sum + value.to_inline());
    first.repr.set_twin(total.repr, Token::new());
    first
  }
}

impl<'a, I, F, R> Sum<&'a Decimal<I, F, R>> for Decimal<I, F, R>
where
  R: Storage,
  Decimal<I, F, R::Inline>: FixedDecimal,
{
  /// Adds the values, from zero.
  ///
  /// # Panics
  ///
  /// Panics if a partial sum is outside the range of the type, in every build, whatever
  /// `overflow-checks` is, also where the whole sum would be in range. A fold with
  /// [`checked_add`](Decimal::checked_add) returns `None` instead.
  fn sum<T: Iterator<Item = &'a Self>>(iter: T) -> Self {
    // The running sum is a twin, from the zero of the twin, and the total is stored once.
    let zero = <Decimal<I, F, R::Inline> as FixedDecimal>::zero();
    Self::from_inline(iter.fold(zero, |sum, value| sum + value.to_inline()))
  }
}
