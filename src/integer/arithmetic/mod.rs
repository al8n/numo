//! The arithmetic of [`Uint`](crate::Uint) and [`Int`](crate::Int): the method tables of the
//! overflowing, checked, wrapping, saturating and strict addition, subtraction, multiplication and
//! negation, and the panics of the strict methods.
//!
//! Each storage computes the overflowing methods at the width `Bits`, and the other families are
//! derived from them. The tables pass their rows to the callbacks of `tables`, for the inherent
//! `const fn`s of each storage and for the generic implementations of the traits, so their bodies
//! name the panics by the full path `$crate::integer::arithmetic::overflow`.

/// Passes the overflowing methods of `Uint` and `Int` to `$callback`, after the tokens of `$args`.
macro_rules! overflowing_table {
  ($callback:ident! $args:tt) => {
    $callback! { $args
      /// Overflowing addition: `self + rhs` wrapped to the range of the type, and whether the exact
      /// sum is outside that range.
      fn overflowing_add(&self, rhs: &Self) -> (Self, bool)
        = prim pair overflowing_add_bits(self, rhs);

      /// Overflowing subtraction: `self - rhs` wrapped to the range of the type, and whether the
      /// exact difference is outside that range.
      fn overflowing_sub(&self, rhs: &Self) -> (Self, bool)
        = prim pair overflowing_sub_bits(self, rhs);

      /// Overflowing multiplication: `self * rhs` wrapped to the range of the type, and whether the
      /// exact product is outside that range.
      fn overflowing_mul(&self, rhs: &Self) -> (Self, bool)
        = prim pair overflowing_mul_bits(self, rhs);

      /// Overflowing negation: `-self` wrapped to the range of the type, and whether the exact
      /// negation is outside that range: for `Uint` every value but zero, and for `Int` only `MIN`,
      /// unless `Bits` is zero and `MIN` is zero.
      fn overflowing_neg(&self) -> (Self, bool) = prim pair overflowing_neg_bits(self);
    }
  };
}

/// Passes the checked methods of `Uint` and `Int` to `$callback`, after the tokens of `$args`.
macro_rules! checked_table {
  ($callback:ident! $args:tt) => {
    $callback! { $args
      /// Checked addition: `self + rhs`, or `None` if the sum is outside the range of the type.
      fn checked_add(&self, rhs: &Self) -> Option<Self> {
        match self.overflowing_add(rhs) {
          (sum, false) => Some(sum),
          (_, true) => None,
        }
      }

      /// Checked subtraction: `self - rhs`, or `None` if the difference is outside the range of
      /// the type.
      fn checked_sub(&self, rhs: &Self) -> Option<Self> {
        match self.overflowing_sub(rhs) {
          (difference, false) => Some(difference),
          (_, true) => None,
        }
      }

      /// Checked multiplication: `self * rhs`, or `None` if the product is outside the range of
      /// the type.
      fn checked_mul(&self, rhs: &Self) -> Option<Self> {
        match self.overflowing_mul(rhs) {
          (product, false) => Some(product),
          (_, true) => None,
        }
      }

      /// Checked negation: `-self`, or `None` if it is outside the range of the type: for `Uint`
      /// unless `self` is zero, and for `Int` if `self` is `MIN`, unless `Bits` is zero and `MIN`
      /// is zero.
      fn checked_neg(&self) -> Option<Self> {
        match self.overflowing_neg() {
          (negated, false) => Some(negated),
          (_, true) => None,
        }
      }
    }
  };
}

/// Passes the wrapping methods of `Uint` and `Int` to `$callback`, after the tokens of `$args`.
macro_rules! wrapping_table {
  ($callback:ident! $args:tt) => {
    $callback! { $args
      /// Wrapping addition: `self + rhs` modulo `2^Bits`, within the range of the type.
      fn wrapping_add(&self, rhs: &Self) -> Self {
        self.overflowing_add(rhs).0
      }

      /// Wrapping subtraction: `self - rhs` modulo `2^Bits`, within the range of the type.
      fn wrapping_sub(&self, rhs: &Self) -> Self {
        self.overflowing_sub(rhs).0
      }

      /// Wrapping multiplication: `self * rhs` modulo `2^Bits`, within the range of the type.
      fn wrapping_mul(&self, rhs: &Self) -> Self {
        self.overflowing_mul(rhs).0
      }

      /// Wrapping negation: `-self` modulo `2^Bits`, within the range of the type; `MIN` of `Int`
      /// negates to itself.
      fn wrapping_neg(&self) -> Self {
        self.overflowing_neg().0
      }
    }
  };
}

/// Passes the saturating methods of `Uint`, written `saturating_table!(Uint, ..)`, or of `Int`,
/// written `saturating_table!(Int, ..)`, to `$callback`, after the tokens of `$args`. Both have the
/// same documentation and signatures, and differ in the bound that an overflow saturates to.
macro_rules! saturating_table {
  (Uint, $callback:ident! $args:tt) => {
    $callback! { $args
      /// Saturating addition: `self + rhs`, or the bound of the range of the type that the sum
      /// passes.
      fn saturating_add(&self, rhs: &Self) -> Self {
        match self.overflowing_add(rhs) {
          (sum, false) => sum,
          (_, true) => Self::MAX,
        }
      }

      /// Saturating subtraction: `self - rhs`, or the bound of the range of the type that the
      /// difference passes.
      fn saturating_sub(&self, rhs: &Self) -> Self {
        match self.overflowing_sub(rhs) {
          (difference, false) => difference,
          (_, true) => Self::MIN,
        }
      }

      /// Saturating multiplication: `self * rhs`, or the bound of the range of the type that the
      /// product passes.
      fn saturating_mul(&self, rhs: &Self) -> Self {
        match self.overflowing_mul(rhs) {
          (product, false) => product,
          (_, true) => Self::MAX,
        }
      }
    }
  };
  (Int, $callback:ident! $args:tt) => {
    $callback! { $args
      /// Saturating addition: `self + rhs`, or the bound of the range of the type that the sum
      /// passes.
      fn saturating_add(&self, rhs: &Self) -> Self {
        match self.overflowing_add(rhs) {
          (sum, false) => sum,
          // The sum passes `MIN` exactly when `rhs` is negative.
          (_, true) => {
            if rhs.is_negative() {
              Self::MIN
            } else {
              Self::MAX
            }
          }
        }
      }

      /// Saturating subtraction: `self - rhs`, or the bound of the range of the type that the
      /// difference passes.
      fn saturating_sub(&self, rhs: &Self) -> Self {
        match self.overflowing_sub(rhs) {
          (difference, false) => difference,
          // The difference passes `MAX` exactly when `rhs` is negative.
          (_, true) => {
            if rhs.is_negative() {
              Self::MAX
            } else {
              Self::MIN
            }
          }
        }
      }

      /// Saturating multiplication: `self * rhs`, or the bound of the range of the type that the
      /// product passes.
      fn saturating_mul(&self, rhs: &Self) -> Self {
        match self.overflowing_mul(rhs) {
          (product, false) => product,
          // An overflowing product passes `MIN` exactly when the operands differ in sign.
          (_, true) => {
            if self.is_negative() != rhs.is_negative() {
              Self::MIN
            } else {
              Self::MAX
            }
          }
        }
      }
    }
  };
}

/// Passes the strict methods of `Uint` and `Int` to `$callback`, after the tokens of `$args`.
macro_rules! strict_table {
  ($callback:ident! $args:tt) => {
    $callback! { $args
      /// Strict addition: `self + rhs`, which panics on overflow whatever the `overflow-checks`
      /// setting.
      ///
      /// # Panics
      ///
      /// Panics if the sum is outside the range of the type.
      #[track_caller]
      fn strict_add(&self, rhs: &Self) -> Self {
        match self.overflowing_add(rhs) {
          (sum, false) => sum,
          (_, true) => $crate::integer::arithmetic::overflow::add(),
        }
      }

      /// Strict subtraction: `self - rhs`, which panics on overflow whatever the
      /// `overflow-checks` setting.
      ///
      /// # Panics
      ///
      /// Panics if the difference is outside the range of the type.
      #[track_caller]
      fn strict_sub(&self, rhs: &Self) -> Self {
        match self.overflowing_sub(rhs) {
          (difference, false) => difference,
          (_, true) => $crate::integer::arithmetic::overflow::sub(),
        }
      }

      /// Strict multiplication: `self * rhs`, which panics on overflow whatever the
      /// `overflow-checks` setting.
      ///
      /// # Panics
      ///
      /// Panics if the product is outside the range of the type.
      #[track_caller]
      fn strict_mul(&self, rhs: &Self) -> Self {
        match self.overflowing_mul(rhs) {
          (product, false) => product,
          (_, true) => $crate::integer::arithmetic::overflow::mul(),
        }
      }

      /// Strict negation: `-self`, which panics on overflow whatever the `overflow-checks`
      /// setting.
      ///
      /// # Panics
      ///
      /// Panics if the negation is outside the range of the type: for `Uint` if `self` is not
      /// zero, and for `Int` if `self` is `MIN`, unless `Bits` is zero and `MIN` is zero.
      #[track_caller]
      fn strict_neg(&self) -> Self {
        match self.overflowing_neg() {
          (negated, false) => negated,
          (_, true) => $crate::integer::arithmetic::overflow::neg(),
        }
      }
    }
  };
}

/// The panics of the strict methods, with the messages of the native integers.
pub(super) mod overflow {
  /// Panics as the native `strict_add` does.
  #[cold]
  #[track_caller]
  pub(crate) const fn add() -> ! {
    panic!("attempt to add with overflow")
  }

  /// Panics as the native `strict_sub` does.
  #[cold]
  #[track_caller]
  pub(crate) const fn sub() -> ! {
    panic!("attempt to subtract with overflow")
  }

  /// Panics as the native `strict_mul` does.
  #[cold]
  #[track_caller]
  pub(crate) const fn mul() -> ! {
    panic!("attempt to multiply with overflow")
  }

  /// Panics as the native `strict_neg` does.
  #[cold]
  #[track_caller]
  pub(crate) const fn neg() -> ! {
    panic!("attempt to negate with overflow")
  }
}

#[cfg(test)]
mod tests;
