//! The operators of [`Decimal`]: `+` and `-` on two values and on two references, `+=` and `-=`,
//! and `Sum` over values and over references.
//!
//! Each has one implementation for every storage, through [`FixedDecimal`], whose `checked_add`,
//! `checked_sub` and `zero` each storage implements. An operator returns what the checked method
//! returns, or panics where that method returns `None`. It computes nothing itself, so no build
//! setting changes what it does.

use core::{
  iter::Sum,
  ops::{Add, AddAssign, Sub, SubAssign},
};

use super::{Decimal, FixedDecimal};

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
  Self: FixedDecimal,
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
  fn add(self, rhs: Self) -> Self {
    match FixedDecimal::checked_add(&self, &rhs) {
      Some(sum) => sum,
      None => overflow::add(),
    }
  }
}

impl<I, F, R> Add<&Decimal<I, F, R>> for &Decimal<I, F, R>
where
  Decimal<I, F, R>: FixedDecimal,
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
    match FixedDecimal::checked_add(self, rhs) {
      Some(sum) => sum,
      None => overflow::add(),
    }
  }
}

impl<I, F, R> AddAssign for Decimal<I, F, R>
where
  Self: FixedDecimal,
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
    match FixedDecimal::checked_add(self, &rhs) {
      Some(sum) => *self = sum,
      None => overflow::add(),
    }
  }
}

impl<I, F, R> Sub for Decimal<I, F, R>
where
  Self: FixedDecimal,
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
  fn sub(self, rhs: Self) -> Self {
    match FixedDecimal::checked_sub(&self, &rhs) {
      Some(difference) => difference,
      None => overflow::sub(),
    }
  }
}

impl<I, F, R> Sub<&Decimal<I, F, R>> for &Decimal<I, F, R>
where
  Decimal<I, F, R>: FixedDecimal,
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
    match FixedDecimal::checked_sub(self, rhs) {
      Some(difference) => difference,
      None => overflow::sub(),
    }
  }
}

impl<I, F, R> SubAssign for Decimal<I, F, R>
where
  Self: FixedDecimal,
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
    match FixedDecimal::checked_sub(self, &rhs) {
      Some(difference) => *self = difference,
      None => overflow::sub(),
    }
  }
}

impl<I, F, R> Sum for Decimal<I, F, R>
where
  Self: FixedDecimal,
{
  /// Adds the values, from zero.
  ///
  /// # Panics
  ///
  /// Panics if a partial sum is outside the range of the type, in every build, whatever
  /// `overflow-checks` is, also where the whole sum would be in range. A fold with
  /// [`checked_add`](Decimal::checked_add) returns `None` instead.
  fn sum<T: Iterator<Item = Self>>(iter: T) -> Self {
    iter.fold(Self::zero(), |sum, value| sum + value)
  }
}

impl<'a, I, F, R> Sum<&'a Decimal<I, F, R>> for Decimal<I, F, R>
where
  Self: FixedDecimal,
{
  /// Adds the values, from zero.
  ///
  /// # Panics
  ///
  /// Panics if a partial sum is outside the range of the type, in every build, whatever
  /// `overflow-checks` is, also where the whole sum would be in range. A fold with
  /// [`checked_add`](Decimal::checked_add) returns `None` instead.
  fn sum<T: Iterator<Item = &'a Self>>(iter: T) -> Self {
    iter.fold(Self::zero(), |sum, value| &sum + value)
  }
}
