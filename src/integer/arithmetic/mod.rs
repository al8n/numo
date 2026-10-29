//! The arithmetic of [`Uint`](crate::Uint) and [`Int`](crate::Int): the method tables of the
//! overflowing, checked, wrapping, saturating and strict addition, subtraction, multiplication and
//! negation, of the carrying addition and the borrowing subtraction of `Uint`, and of the absolute
//! value and the sign of `Int`, the panics of the strict methods, and the operators, `Sum` and
//! `Product`.
//!
//! Each storage computes the overflowing methods at the width `Bits`, and most other methods are
//! derived from them. The tables pass their rows to the callbacks of `tables`, for the inherent
//! `const fn`s of each storage and for the generic implementations of the traits, so their bodies
//! name the panics by the full path `$crate::integer::arithmetic::overflow`. The operators are
//! implemented once for every storage, over `OverflowingOps`, and repeat an overflow in a native
//! operation, so that `overflow-checks` decides whether they panic, as for the native integers.

use core::{
  iter::{Product, Sum},
  ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use typenum::{IsGreater, NonZero, True, U1};

use super::{Int, OverflowingOps, Uint};
use crate::{
  layout::Token,
  width::{FitsBits, InlineIntRepr, InlineUintRepr, IntRepr, IntReprMut, UintRepr, UintReprMut},
};

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
      /// except at zero bits, where `MIN` is zero.
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
      /// unless `self` is zero, and for `Int` if `self` is `MIN`, except at zero bits, where `MIN`
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
      /// zero, and for `Int` if `self` is `MIN`, except at zero bits, where `MIN` is zero.
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

/// Passes the carrying addition and the borrowing subtraction of `Uint` to `$callback`, after the
/// tokens of `$args`.
macro_rules! uint_sign_table {
  ($callback:ident! $args:tt) => {
    $callback! { $args
      /// Addition with a carry: `self + rhs + carry` wrapped to the range of the type, and whether
      /// the exact sum is outside that range.
      fn carrying_add(&self, rhs: &Self, carry: bool) -> (Self, bool) {
        let (sum, overflow) = self.overflowing_add(rhs);
        if carry {
          // Adding one subtracts `MAX`, which is `-1` modulo `2^Bits`, and overflows exactly when
          // the sum is `MAX`, which is when the subtraction does not borrow. At zero bits, which
          // hold no one, a carry always overflows.
          let (sum, borrow) = sum.overflowing_sub(&Self::MAX);
          (sum, overflow || !borrow)
        } else {
          (sum, overflow)
        }
      }

      /// Subtraction with a borrow: `self - rhs - borrow` wrapped to the range of the type, and
      /// whether the exact difference is negative.
      fn borrowing_sub(&self, rhs: &Self, borrow: bool) -> (Self, bool) {
        let (difference, overflow) = self.overflowing_sub(rhs);
        if borrow {
          // Subtracting one adds `MAX`, which is `-1` modulo `2^Bits`, and overflows exactly when
          // the difference is zero, which is when the addition does not carry. At zero bits, which
          // hold no one, a borrow always overflows.
          let (difference, carry) = difference.overflowing_add(&Self::MAX);
          (difference, overflow || !carry)
        } else {
          (difference, overflow)
        }
      }
    }
  };
}

/// Passes the absolute value, the sign and the saturating negation of `Int` to `$callback`, after
/// the tokens of `$args`.
macro_rules! int_sign_table {
  ($callback:ident! $args:tt) => {
    $callback! { $args
      /// Saturating negation: `-self`, or `MAX` if the negation is outside the range of the type,
      /// as it is for `MIN`, except at zero bits, where `MIN` is zero.
      fn saturating_neg(&self) -> Self {
        match self.overflowing_neg() {
          (negated, false) => negated,
          (_, true) => Self::MAX,
        }
      }

      /// Overflowing absolute value: `|self|` wrapped to the range of the type, and whether it is
      /// outside that range, as it is for `MIN`, except at zero bits, where `MIN` is zero. `MIN`
      /// wraps to itself.
      fn overflowing_abs(&self) -> (Self, bool) {
        if self.is_negative() {
          self.overflowing_neg()
        } else {
          (*self, false)
        }
      }

      /// Checked absolute value: `|self|`, or `None` if it is outside the range of the type, as it
      /// is for `MIN`, except at zero bits, where `MIN` is zero.
      fn checked_abs(&self) -> Option<Self> {
        match self.overflowing_abs() {
          (abs, false) => Some(abs),
          (_, true) => None,
        }
      }

      /// Wrapping absolute value: `|self|` modulo `2^Bits`, within the range of the type; `MIN`
      /// wraps to itself.
      fn wrapping_abs(&self) -> Self {
        self.overflowing_abs().0
      }

      /// Saturating absolute value: `|self|`, or `MAX` if it is outside the range of the type, as
      /// it is for `MIN`, except at zero bits, where `MIN` is zero.
      fn saturating_abs(&self) -> Self {
        match self.overflowing_abs() {
          (abs, false) => abs,
          (_, true) => Self::MAX,
        }
      }

      /// Strict absolute value: `|self|`, which panics on overflow whatever the `overflow-checks`
      /// setting.
      ///
      /// # Panics
      ///
      /// Panics if the absolute value is outside the range of the type, as it is for `MIN`,
      /// except at zero bits, where `MIN` is zero.
      #[track_caller]
      fn strict_abs(&self) -> Self {
        match self.overflowing_abs() {
          (abs, false) => abs,
          (_, true) => $crate::integer::arithmetic::overflow::neg(),
        }
      }

      /// The absolute value, `|self|`. The absolute value of `MIN` is outside the range of the
      /// type, except at zero bits, where `MIN` is zero: the method then panics or returns `MIN`, as
      /// `overflow-checks` decides for the native integers (see [Overflow](crate::Int#overflow)).
      #[track_caller]
      fn abs(&self) -> Self {
        match self.overflowing_abs() {
          (abs, false) => abs,
          (abs, true) => {
            $crate::integer::arithmetic::overflow::forward_neg();
            abs
          }
        }
      }

      /// The sign of the value: -1 if it is negative, zero if it is zero, and one if it is
      /// positive.
      fn signum(&self) -> Self = prim value signum(self);

      /// The absolute value, `|self|`, as the unsigned integer of the same width, which also holds
      /// the absolute value of `MIN`.
      fn unsigned_abs(&self) -> Self::Unsigned = prim unsigned unsigned_abs(self);
    }
  };
}

/// The panics of the strict methods, with the messages of the native integers, and the overflow of
/// a native operation, which follows the `overflow-checks` setting.
pub(super) mod overflow {
  use core::hint::black_box;

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

  /// Repeats the overflow of a native addition, which panics with the message of the native
  /// integers when `overflow-checks` is on and does nothing otherwise.
  #[inline]
  #[track_caller]
  pub(crate) const fn forward_add() {
    let _ = black_box(u8::MAX) + 1;
  }

  /// Repeats the overflow of a native subtraction, as [`forward_add`] does.
  #[inline]
  #[track_caller]
  pub(crate) const fn forward_sub() {
    let _ = black_box(0u8) - 1;
  }

  /// Repeats the overflow of a native multiplication, as [`forward_add`] does.
  #[inline]
  #[track_caller]
  pub(crate) const fn forward_mul() {
    let _ = black_box(u8::MAX) * 2;
  }

  /// Repeats the overflow of a native negation, which panics with the message of the native
  /// integers when `overflow-checks` is on and does nothing otherwise.
  #[inline]
  #[track_caller]
  pub(crate) const fn forward_neg() {
    let _ = -black_box(i8::MIN);
  }
}

/// Implements the binary operator `$op` in its four forms of values and references, and its
/// compound assignment `$assign` for a value and a reference. The form on two references computes
/// over `OverflowingOps` for every storage, `$repr`; the forms with an owned operand, for a storage
/// that holds the value inline, `$inline`, pass references to it; and the compound assignments, for
/// a storage that holds the value in place, `$repr_mut`, compute with the sealed storage method
/// `$primitive` and store the result. On overflow, each repeats a native overflow with `$forward`,
/// which panics or not as `overflow-checks` decides; without a panic, the result is the wrapped
/// value. An assignment stores the result after `$forward`, so that a panic leaves its target
/// unchanged, as for the native integers.
macro_rules! binary_operator {
  (
    $type:ident: $repr:ident, $inline:ident, $repr_mut:ident;
    $op:ident $method:ident,
    $assign:ident $assign_method:ident,
    $overflowing:ident, $primitive:ident, $forward:ident
  ) => {
    impl<B: FitsBits<R>, R: $inline> $op for $type<B, R> {
      type Output = Self;

      #[inline]
      #[track_caller]
      fn $method(self, rhs: Self) -> Self {
        $op::$method(&self, &rhs)
      }
    }

    impl<B: FitsBits<R>, R: $inline> $op<&$type<B, R>> for $type<B, R> {
      type Output = Self;

      #[inline]
      #[track_caller]
      fn $method(self, rhs: &Self) -> Self {
        $op::$method(&self, rhs)
      }
    }

    impl<B: FitsBits<R>, R: $inline> $op<$type<B, R>> for &$type<B, R> {
      type Output = $type<B, R>;

      #[inline]
      #[track_caller]
      fn $method(self, rhs: $type<B, R>) -> $type<B, R> {
        $op::$method(self, &rhs)
      }
    }

    impl<B: FitsBits<R>, R: $repr> $op<&$type<B, R>> for &$type<B, R> {
      type Output = $type<B, R>;

      #[inline]
      #[track_caller]
      fn $method(self, rhs: &$type<B, R>) -> $type<B, R> {
        let (value, overflow) = OverflowingOps::$overflowing(self, rhs);
        if overflow {
          overflow::$forward();
        }
        value
      }
    }

    impl<B: FitsBits<R>, R: $repr_mut> $assign for $type<B, R> {
      #[inline]
      #[track_caller]
      fn $assign_method(&mut self, rhs: Self) {
        $assign::$assign_method(self, &rhs);
      }
    }

    impl<B: FitsBits<R>, R: $repr_mut> $assign<&$type<B, R>> for $type<B, R> {
      #[inline]
      #[track_caller]
      fn $assign_method(&mut self, rhs: &Self) {
        let (value, overflow) = self.repr.$primitive(&rhs.repr, Self::BITS, Token::new());
        if overflow {
          overflow::$forward();
        }
        self.repr = value;
      }
    }
  };
}

binary_operator!(
  Uint: UintRepr, InlineUintRepr, UintReprMut;
  Add add, AddAssign add_assign, overflowing_add, overflowing_add_bits, forward_add
);
binary_operator!(
  Uint: UintRepr, InlineUintRepr, UintReprMut;
  Sub sub, SubAssign sub_assign, overflowing_sub, overflowing_sub_bits, forward_sub
);
binary_operator!(
  Uint: UintRepr, InlineUintRepr, UintReprMut;
  Mul mul, MulAssign mul_assign, overflowing_mul, overflowing_mul_bits, forward_mul
);
binary_operator!(
  Int: IntRepr, InlineIntRepr, IntReprMut;
  Add add, AddAssign add_assign, overflowing_add, overflowing_add_bits, forward_add
);
binary_operator!(
  Int: IntRepr, InlineIntRepr, IntReprMut;
  Sub sub, SubAssign sub_assign, overflowing_sub, overflowing_sub_bits, forward_sub
);
binary_operator!(
  Int: IntRepr, InlineIntRepr, IntReprMut;
  Mul mul, MulAssign mul_assign, overflowing_mul, overflowing_mul_bits, forward_mul
);

// As for the binary operators, the negation of a reference computes for every storage, and the
// negation of a value, for a storage that holds the value inline, passes a reference to it.
impl<B: FitsBits<R>, R: InlineIntRepr> Neg for Int<B, R> {
  type Output = Self;

  #[inline]
  #[track_caller]
  fn neg(self) -> Self {
    Neg::neg(&self)
  }
}

impl<B: FitsBits<R>, R: IntRepr> Neg for &Int<B, R> {
  type Output = Int<B, R>;

  #[inline]
  #[track_caller]
  fn neg(self) -> Int<B, R> {
    let (value, overflow) = OverflowingOps::overflowing_neg(self);
    if overflow {
      overflow::forward_neg();
    }
    value
  }
}

/// Implements `Sum`, which adds from `ZERO` with `+`, and `Product`, which multiplies from `ONE`
/// with `*` for a width bounded by `$one`, for values and references.
macro_rules! folds {
  ($type:ident: $repr:ident, $($one:tt)+) => {
    impl<B: FitsBits<R>, R: $repr> Sum for $type<B, R> {
      fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |sum, value| sum + value)
      }
    }

    impl<'a, B: FitsBits<R>, R: $repr> Sum<&'a $type<B, R>> for $type<B, R> {
      fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |sum, value| sum + value)
      }
    }

    impl<B: FitsBits<R> + $($one)+, R: $repr> Product for $type<B, R> {
      fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ONE, |product, value| product * value)
      }
    }

    impl<'a, B: FitsBits<R> + $($one)+, R: $repr> Product<&'a $type<B, R>> for $type<B, R> {
      fn product<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        iter.fold(Self::ONE, |product, value| product * value)
      }
    }
  };
}

folds!(Uint: InlineUintRepr, NonZero);
folds!(Int: InlineIntRepr, IsGreater<U1, Output = True>);

#[cfg(test)]
mod tests;
