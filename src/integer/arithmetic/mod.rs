//! The arithmetic of [`Uint`](crate::Uint) and [`Int`](crate::Int): the method tables of the
//! overflowing, checked, wrapping, saturating and strict addition, subtraction, multiplication,
//! negation, division, remainder and power, and of the overflowing, checked, wrapping and strict
//! shifts, of the carrying addition and the borrowing subtraction of `Uint`, of the multiplication
//! with a carry and with a carry and an addend of `Uint` and of `Int`, with the carrying addition
//! and the borrowing subtraction of `Int`, of the absolute value and the sign of `Int`, of the
//! unchecked arithmetic of the traits, and of the integer functions without an overflow prefix,
//! such as the Euclidean and rounded division, the roots and the logarithms; the panics; and the
//! operators, `Sum` and `Product`.
//!
//! Each storage computes the overflowing methods at the width `Bits`. The checked methods compute
//! the same on the storage, and build a value only where it is in range; the wrapping methods are
//! derived from the overflowing ones, and the saturating and the strict ones from the checked ones
//! or, where a zero divisor must panic, from the division of the storage, so that a method builds
//! only the integers that it returns. The tables pass their rows to the callbacks of `tables`, for
//! the inherent `const fn`s of each storage and for the generic implementations of the traits, so
//! their bodies name the panics by the full paths `$crate::integer::arithmetic::overflow` and
//! `$crate::integer::arithmetic::domain`. The unchecked tables serve only the traits, whose bodies
//! compute the overflowing or the checked method and check it with a debug assertion: they never
//! call the inherent `unchecked_*` methods, which each storage writes out with their unsafe blocks.
//! The operators are implemented once for every storage. `+`, `-`, `*` and the negation compute
//! with the sealed storage methods on the inline storages that the operands lend, and repeat an
//! overflow in a native operation, so that `overflow-checks` decides whether they panic: on two
//! references they then allocate the result, and with an owned operand they store it in that
//! operand, as a compound assignment stores it in its target. `/` and `%` on two references are
//! the methods of `StrictOps`, which panic in every build, as for the native integers, and with an
//! owned operand they compute the same on the inline storages and store the result in that
//! operand.

use core::{
  iter::{Product, Sum},
  ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign},
};

use typenum::{IsGreater, NonZero, True, U1};

use super::{Int, StrictOps, Uint, nonzero};
use crate::{
  layout::Token,
  width::{FitsBits, IntCore, IntRepr, IntReprMut, UintCore, UintRepr, UintReprMut},
};

/// Passes the overflowing methods of `Uint` and `Int` to `$callback`, after the tokens of `$args`.
// The rows of `checked_table!` and the inherent `unchecked_*` methods make the calls of these
// rows, so that `unchecked_*` returns the value of `wrapping_*`; the safety of `unchecked_*` rests
// on the flags of those calls, as `checked_table!` explains.
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

      /// Overflowing division: `self / rhs` rounded toward zero, wrapped to the range of the type,
      /// and whether the exact quotient is outside that range, as it is only for `Int` when `self`
      /// is `MIN` and `rhs` is -1, where the quotient wraps to `MIN`.
      ///
      /// # Panics
      ///
      /// Panics if `rhs` is zero.
      #[track_caller]
      fn overflowing_div(&self, rhs: &Self) -> (Self, bool)
        = prim div_rem_bits(self, rhs; Self::BITS) as division {
          match division {
            Some((quotient, _, overflow)) => (Self::of(quotient), overflow),
            None => $crate::integer::arithmetic::domain::div_by_zero(),
          }
        }

      /// Overflowing remainder: `self % rhs`, the remainder of the division rounded toward zero,
      /// which has the sign of `self`, and whether the quotient is outside the range of the type,
      /// as it is only for `Int` when `self` is `MIN` and `rhs` is -1, where the remainder is zero.
      ///
      /// # Panics
      ///
      /// Panics if `rhs` is zero.
      #[track_caller]
      fn overflowing_rem(&self, rhs: &Self) -> (Self, bool)
        = prim div_rem_bits(self, rhs; Self::BITS) as division {
          match division {
            Some((_, remainder, overflow)) => (Self::of(remainder), overflow),
            None => $crate::integer::arithmetic::domain::rem_by_zero(),
          }
        }

      /// Overflowing Euclidean division: the quotient `q` of `self = q * rhs + r` with
      /// `0 <= r < |rhs|`, wrapped to the range of the type, and whether the exact quotient is
      /// outside that range, as it is only for `Int` when `self` is `MIN` and `rhs` is -1, where
      /// the quotient wraps to `MIN`. For `Uint` it is the division rounded toward zero.
      ///
      /// # Panics
      ///
      /// Panics if `rhs` is zero.
      #[track_caller]
      fn overflowing_div_euclid(&self, rhs: &Self) -> (Self, bool)
        = prim div_rem_euclid_bits(self, rhs; Self::BITS) as division {
          match division {
            Some((quotient, _, overflow)) => (Self::of(quotient), overflow),
            None => $crate::integer::arithmetic::domain::div_by_zero(),
          }
        }

      /// Overflowing Euclidean remainder: the remainder `r` of `self = q * rhs + r` with
      /// `0 <= r < |rhs|`, and whether the quotient is outside the range of the type, as it is only
      /// for `Int` when `self` is `MIN` and `rhs` is -1, where the remainder is zero.
      ///
      /// # Panics
      ///
      /// Panics if `rhs` is zero.
      #[track_caller]
      fn overflowing_rem_euclid(&self, rhs: &Self) -> (Self, bool)
        = prim div_rem_euclid_bits(self, rhs; Self::BITS) as division {
          match division {
            Some((_, remainder, overflow)) => (Self::of(remainder), overflow),
            None => $crate::integer::arithmetic::domain::rem_by_zero(),
          }
        }

      /// Overflowing exponentiation: `self^exp` wrapped to the range of the type, and whether the
      /// exact power is outside that range. A power that is one, as `self^0` is, overflows where
      /// one is outside the range: at zero bits, where it wraps to zero, and for `Int` of one bit,
      /// where it wraps to -1, as `(-1)^2` does, while `(-1)^3` is -1.
      fn overflowing_pow(&self, exp: u32) -> (Self, bool)
        = prim pair overflowing_pow_bits(self; exp);

      /// Overflowing shift left: `self << rhs` within `Bits` bits, with `rhs` taken modulo `Bits`,
      /// and whether `rhs` is `Bits` or more, as the native method, which takes `rhs` modulo the
      /// width of the type. Zero bits hold only zero, and every amount overflows there.
      fn overflowing_shl(&self, rhs: u32) -> (Self, bool)
        = prim pair overflowing_shl_bits(self; rhs);

      /// Overflowing shift right: `self >> rhs`, arithmetic for `Int`, with `rhs` taken modulo
      /// `Bits`, and whether `rhs` is `Bits` or more. Zero bits hold only zero, and every amount
      /// overflows there.
      fn overflowing_shr(&self, rhs: u32) -> (Self, bool)
        = prim pair overflowing_shr_bits(self; rhs);
    }
  };
}

/// Passes the checked methods of `Uint` and `Int` to `$callback`, after the tokens of `$args`.
// The unsafe blocks of the inherent `unchecked_*` methods, in `integer`, rely on the shape of
// these rows. `checked_add`, `checked_sub`, `checked_mul`, `checked_neg`, `checked_shl` and
// `checked_shr` call `overflowing_*_bits` of the storage on the operands, with the amount of a
// shift and then `Self::BITS`, and return `None` exactly when its flag is set; for the shifts the
// flag is `rhs >= Bits`, set for every amount at zero bits. `unchecked_*` makes the same call on
// the same storage and passes that flag to `assert_unchecked`, or at the full width of a native
// integer calls the native `unchecked_*`, whose precondition is then that flag: either way it
// assumes exactly what `checked_*` tests. `checked_div_exact` returns `Some` exactly when
// `checked_div_exact_bits` does, and `unchecked_div_exact` unwraps that value. A change to the
// call or to the flag of one of these rows requires revisiting those unsafe blocks.
macro_rules! checked_table {
  ($callback:ident! $args:tt) => {
    $callback! { $args
      /// Checked addition: `self + rhs`, or `None` if the sum is outside the range of the type.
      fn checked_add(&self, rhs: &Self) -> Option<Self>
        = prim overflowing_add_bits(self, rhs; Self::BITS) as sum {
          match sum {
            (sum, false) => Some(Self::of(sum)),
            (_, true) => None,
          }
        }

      /// Checked subtraction: `self - rhs`, or `None` if the difference is outside the range of
      /// the type.
      fn checked_sub(&self, rhs: &Self) -> Option<Self>
        = prim overflowing_sub_bits(self, rhs; Self::BITS) as difference {
          match difference {
            (difference, false) => Some(Self::of(difference)),
            (_, true) => None,
          }
        }

      /// Checked multiplication: `self * rhs`, or `None` if the product is outside the range of
      /// the type.
      fn checked_mul(&self, rhs: &Self) -> Option<Self>
        = prim overflowing_mul_bits(self, rhs; Self::BITS) as product {
          match product {
            (product, false) => Some(Self::of(product)),
            (_, true) => None,
          }
        }

      /// Checked negation: `-self`, or `None` if it is outside the range of the type: for `Uint`
      /// unless `self` is zero, and for `Int` if `self` is `MIN`, except at zero bits, where `MIN`
      /// is zero.
      fn checked_neg(&self) -> Option<Self>
        = prim overflowing_neg_bits(self; Self::BITS) as negated {
          match negated {
            (negated, false) => Some(Self::of(negated)),
            (_, true) => None,
          }
        }

      /// Checked division: `self / rhs` rounded toward zero, or `None` if `rhs` is zero or the
      /// quotient is outside the range of the type, as it is only for `Int` when `self` is `MIN`
      /// and `rhs` is -1.
      fn checked_div(&self, rhs: &Self) -> Option<Self>
        = prim div_rem_bits(self, rhs; Self::BITS) as division {
          match division {
            Some((quotient, _, false)) => Some(Self::of(quotient)),
            _ => None,
          }
        }

      /// Checked remainder: `self % rhs`, which has the sign of `self`, or `None` if `rhs` is zero
      /// or the quotient is outside the range of the type, as it is only for `Int` when `self` is
      /// `MIN` and `rhs` is -1.
      fn checked_rem(&self, rhs: &Self) -> Option<Self>
        = prim div_rem_bits(self, rhs; Self::BITS) as division {
          match division {
            Some((_, remainder, false)) => Some(Self::of(remainder)),
            _ => None,
          }
        }

      /// Checked Euclidean division: the quotient `q` of `self = q * rhs + r` with
      /// `0 <= r < |rhs|`, or `None` if `rhs` is zero or the quotient is outside the range of the
      /// type, as it is only for `Int` when `self` is `MIN` and `rhs` is -1.
      fn checked_div_euclid(&self, rhs: &Self) -> Option<Self>
        = prim div_rem_euclid_bits(self, rhs; Self::BITS) as division {
          match division {
            Some((quotient, _, false)) => Some(Self::of(quotient)),
            _ => None,
          }
        }

      /// Checked Euclidean remainder: the remainder `r` of `self = q * rhs + r` with
      /// `0 <= r < |rhs|`, or `None` if `rhs` is zero or the quotient is outside the range of the
      /// type, as it is only for `Int` when `self` is `MIN` and `rhs` is -1.
      fn checked_rem_euclid(&self, rhs: &Self) -> Option<Self>
        = prim div_rem_euclid_bits(self, rhs; Self::BITS) as division {
          match division {
            Some((_, remainder, false)) => Some(Self::of(remainder)),
            _ => None,
          }
        }

      /// Checked exponentiation: `self^exp`, or `None` if the power is outside the range of the
      /// type, as `self^0`, one, is at zero bits and for `Int` of one bit.
      fn checked_pow(&self, exp: u32) -> Option<Self>
        = prim overflowing_pow_bits(self; exp, Self::BITS) as power {
          match power {
            (power, false) => Some(Self::of(power)),
            (_, true) => None,
          }
        }

      /// Checked shift left: `self << rhs`, or `None` if `rhs` is `Bits` or more, as every amount
      /// is at zero bits. The bits shifted out at the top are lost, as for the native method;
      /// [`shl_exact`](crate::IntegerBits::shl_exact) returns `None` for them instead.
      fn checked_shl(&self, rhs: u32) -> Option<Self>
        = prim overflowing_shl_bits(self; rhs, Self::BITS) as shifted {
          match shifted {
            (shifted, false) => Some(Self::of(shifted)),
            (_, true) => None,
          }
        }

      /// Checked shift right: `self >> rhs`, arithmetic for `Int`, or `None` if `rhs` is `Bits`
      /// or more, as every amount is at zero bits.
      fn checked_shr(&self, rhs: u32) -> Option<Self>
        = prim overflowing_shr_bits(self; rhs, Self::BITS) as shifted {
          match shifted {
            (shifted, false) => Some(Self::of(shifted)),
            (_, true) => None,
          }
        }

      /// Checked next multiple: for a positive `rhs`, the smallest multiple of `rhs` at least
      /// `self`, and for a negative `rhs`, which only `Int` has, the largest multiple of `rhs` at
      /// most `self`; or `None` if `rhs` is zero or that multiple is outside the range of the type.
      fn checked_next_multiple_of(&self, rhs: &Self) -> Option<Self>
        = prim next_multiple_of_bits(self, rhs; Self::BITS) as multiple {
          match multiple {
            Some((multiple, false)) => Some(Self::of(multiple)),
            _ => None,
          }
        }

      /// Checked exact division: `self / rhs` if `rhs` divides `self`, or `None` if `rhs` is
      /// zero, the division leaves a remainder, or the quotient is outside the range of the type,
      /// as it is only for `Int` when `self` is `MIN` and `rhs` is -1.
      fn checked_div_exact(&self, rhs: &Self) -> Option<Self>
        = prim checked_div_exact_bits(self, rhs; Self::BITS) as quotient {
          match quotient {
            Some(quotient) => Some(Self::of(quotient)),
            None => None,
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

      /// Wrapping division: `self / rhs` rounded toward zero, modulo `2^Bits`, within the range of
      /// the type; for `Int`, `MIN / -1` wraps to `MIN`.
      ///
      /// # Panics
      ///
      /// Panics if `rhs` is zero.
      #[track_caller]
      fn wrapping_div(&self, rhs: &Self) -> Self {
        self.overflowing_div(rhs).0
      }

      /// Wrapping remainder: `self % rhs`, which has the sign of `self`; for `Int`, `MIN % -1` is
      /// zero.
      ///
      /// # Panics
      ///
      /// Panics if `rhs` is zero.
      #[track_caller]
      fn wrapping_rem(&self, rhs: &Self) -> Self {
        self.overflowing_rem(rhs).0
      }

      /// Wrapping Euclidean division: the quotient `q` of `self = q * rhs + r` with
      /// `0 <= r < |rhs|`, modulo `2^Bits`, within the range of the type; for `Int`, `MIN / -1`
      /// wraps to `MIN`.
      ///
      /// # Panics
      ///
      /// Panics if `rhs` is zero.
      #[track_caller]
      fn wrapping_div_euclid(&self, rhs: &Self) -> Self {
        self.overflowing_div_euclid(rhs).0
      }

      /// Wrapping Euclidean remainder: the remainder `r` of `self = q * rhs + r` with
      /// `0 <= r < |rhs|`; for `Int`, the remainder of `MIN` by -1 is zero.
      ///
      /// # Panics
      ///
      /// Panics if `rhs` is zero.
      #[track_caller]
      fn wrapping_rem_euclid(&self, rhs: &Self) -> Self {
        self.overflowing_rem_euclid(rhs).0
      }

      /// Wrapping exponentiation: `self^exp` modulo `2^Bits`, within the range of the type.
      fn wrapping_pow(&self, exp: u32) -> Self {
        self.overflowing_pow(exp).0
      }

      /// Wrapping shift left: `self << (rhs % Bits)` within `Bits` bits. Unlike the other wrapping
      /// methods, it takes the amount modulo the width rather than the result, as the native
      /// method does; [`unbounded_shl`](crate::IntegerBits::unbounded_shl) shifts every bit out
      /// instead. Zero bits hold only zero, which it returns for every amount.
      fn wrapping_shl(&self, rhs: u32) -> Self {
        self.overflowing_shl(rhs).0
      }

      /// Wrapping shift right: `self >> (rhs % Bits)`, arithmetic for `Int`, taking the amount
      /// modulo the width as [`wrapping_shl`](Self::wrapping_shl) does;
      /// [`unbounded_shr`](crate::IntegerBits::unbounded_shr) shifts every bit out instead.
      fn wrapping_shr(&self, rhs: u32) -> Self {
        self.overflowing_shr(rhs).0
      }
    }
  };
}

/// Passes the saturating methods of `Uint`, written `saturating_table!(Uint, ..)`, or of `Int`,
/// written `saturating_table!(Int, ..)`, to `$callback`, after the tokens of `$args`. Both have the
/// same documentation and signatures, and differ in the bound that an overflow saturates to.
// The rows of `Int` read the sign of a value with `bit`, as the casts of `int_bits_table!` do, so
// that they compile for every storage: `is_negative` reaches generic code only through
// `SignedFixedInteger`.
macro_rules! saturating_table {
  (Uint, $callback:ident! $args:tt) => {
    $callback! { $args
      /// Saturating addition: `self + rhs`, or the bound of the range of the type that the sum
      /// passes.
      fn saturating_add(&self, rhs: &Self) -> Self {
        match self.checked_add(rhs) {
          Some(sum) => sum,
          None => Self::bound(true),
        }
      }

      /// Saturating subtraction: `self - rhs`, or the bound of the range of the type that the
      /// difference passes.
      fn saturating_sub(&self, rhs: &Self) -> Self {
        match self.checked_sub(rhs) {
          Some(difference) => difference,
          None => Self::bound(false),
        }
      }

      /// Saturating multiplication: `self * rhs`, or the bound of the range of the type that the
      /// product passes.
      fn saturating_mul(&self, rhs: &Self) -> Self {
        match self.checked_mul(rhs) {
          Some(product) => product,
          None => Self::bound(true),
        }
      }

      /// Saturating division: `self / rhs` rounded toward zero, or `MAX` if the quotient is
      /// outside the range of the type, as it is only for `Int` when `self` is `MIN` and `rhs` is
      /// -1.
      ///
      /// # Panics
      ///
      /// Panics if `rhs` is zero.
      #[track_caller]
      fn saturating_div(&self, rhs: &Self) -> Self
        = prim div_rem_bits(self, rhs; Self::BITS) as division {
          match division {
            Some((quotient, _, false)) => Self::of(quotient),
            Some((_, _, true)) => Self::bound(true),
            None => $crate::integer::arithmetic::domain::div_by_zero(),
          }
        }

      /// Saturating exponentiation: `self^exp`, or the bound of the range of the type that the
      /// power passes.
      fn saturating_pow(&self, exp: u32) -> Self {
        match self.checked_pow(exp) {
          Some(power) => power,
          None => Self::bound(true),
        }
      }
    }
  };
  (Int, $callback:ident! $args:tt) => {
    $callback! { $args
      /// Saturating addition: `self + rhs`, or the bound of the range of the type that the sum
      /// passes.
      fn saturating_add(&self, rhs: &Self) -> Self {
        match self.checked_add(rhs) {
          Some(sum) => sum,
          // The sum passes `MIN` exactly when `rhs` is negative.
          None => {
            if rhs.bit(Self::BITS.wrapping_sub(1)) {
              Self::bound(false)
            } else {
              Self::bound(true)
            }
          }
        }
      }

      /// Saturating subtraction: `self - rhs`, or the bound of the range of the type that the
      /// difference passes.
      fn saturating_sub(&self, rhs: &Self) -> Self {
        match self.checked_sub(rhs) {
          Some(difference) => difference,
          // The difference passes `MAX` exactly when `rhs` is negative.
          None => {
            if rhs.bit(Self::BITS.wrapping_sub(1)) {
              Self::bound(true)
            } else {
              Self::bound(false)
            }
          }
        }
      }

      /// Saturating multiplication: `self * rhs`, or the bound of the range of the type that the
      /// product passes.
      fn saturating_mul(&self, rhs: &Self) -> Self {
        match self.checked_mul(rhs) {
          Some(product) => product,
          // An overflowing product passes `MIN` exactly when the operands differ in sign.
          None => {
            let sign = Self::BITS.wrapping_sub(1);
            if self.bit(sign) != rhs.bit(sign) {
              Self::bound(false)
            } else {
              Self::bound(true)
            }
          }
        }
      }

      /// Saturating division: `self / rhs` rounded toward zero, or `MAX` if the quotient is
      /// outside the range of the type, as it is only for `Int` when `self` is `MIN` and `rhs` is
      /// -1.
      ///
      /// # Panics
      ///
      /// Panics if `rhs` is zero.
      #[track_caller]
      fn saturating_div(&self, rhs: &Self) -> Self
        = prim div_rem_bits(self, rhs; Self::BITS) as division {
          match division {
            Some((quotient, _, false)) => Self::of(quotient),
            Some((_, _, true)) => Self::bound(true),
            None => $crate::integer::arithmetic::domain::div_by_zero(),
          }
        }

      /// Saturating exponentiation: `self^exp`, or the bound of the range of the type that the
      /// power passes.
      fn saturating_pow(&self, exp: u32) -> Self {
        match self.checked_pow(exp) {
          Some(power) => power,
          // The power passes `MIN` exactly when it is negative: an odd power of a negative value.
          // At one bit `(-1)^0`, one, passes `MAX`, which is zero.
          None => {
            if self.bit(Self::BITS.wrapping_sub(1)) && exp % 2 == 1 {
              Self::bound(false)
            } else {
              Self::bound(true)
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
        match self.checked_add(rhs) {
          Some(sum) => sum,
          None => $crate::integer::arithmetic::overflow::add(),
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
        match self.checked_sub(rhs) {
          Some(difference) => difference,
          None => $crate::integer::arithmetic::overflow::sub(),
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
        match self.checked_mul(rhs) {
          Some(product) => product,
          None => $crate::integer::arithmetic::overflow::mul(),
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
        match self.checked_neg() {
          Some(negated) => negated,
          None => $crate::integer::arithmetic::overflow::neg(),
        }
      }

      /// Strict division: `self / rhs` rounded toward zero, which panics on overflow whatever the
      /// `overflow-checks` setting.
      ///
      /// # Panics
      ///
      /// Panics if `rhs` is zero, or if the quotient is outside the range of the type, as it is
      /// only for `Int` when `self` is `MIN` and `rhs` is -1.
      #[track_caller]
      fn strict_div(&self, rhs: &Self) -> Self
        = prim div_rem_bits(self, rhs; Self::BITS) as division {
          match division {
            Some((quotient, _, false)) => Self::of(quotient),
            Some((_, _, true)) => $crate::integer::arithmetic::overflow::div(),
            None => $crate::integer::arithmetic::domain::div_by_zero(),
          }
        }

      /// Strict remainder: `self % rhs`, which has the sign of `self` and panics on overflow
      /// whatever the `overflow-checks` setting.
      ///
      /// # Panics
      ///
      /// Panics if `rhs` is zero, or if the quotient is outside the range of the type, as it is
      /// only for `Int` when `self` is `MIN` and `rhs` is -1.
      #[track_caller]
      fn strict_rem(&self, rhs: &Self) -> Self
        = prim div_rem_bits(self, rhs; Self::BITS) as division {
          match division {
            Some((_, remainder, false)) => Self::of(remainder),
            Some((_, _, true)) => $crate::integer::arithmetic::overflow::rem(),
            None => $crate::integer::arithmetic::domain::rem_by_zero(),
          }
        }

      /// Strict Euclidean division: the quotient `q` of `self = q * rhs + r` with
      /// `0 <= r < |rhs|`, which panics on overflow whatever the `overflow-checks` setting.
      ///
      /// # Panics
      ///
      /// Panics if `rhs` is zero, or if the quotient is outside the range of the type, as it is
      /// only for `Int` when `self` is `MIN` and `rhs` is -1.
      #[track_caller]
      fn strict_div_euclid(&self, rhs: &Self) -> Self
        = prim div_rem_euclid_bits(self, rhs; Self::BITS) as division {
          match division {
            Some((quotient, _, false)) => Self::of(quotient),
            Some((_, _, true)) => $crate::integer::arithmetic::overflow::div(),
            None => $crate::integer::arithmetic::domain::div_by_zero(),
          }
        }

      /// Strict Euclidean remainder: the remainder `r` of `self = q * rhs + r` with
      /// `0 <= r < |rhs|`, which panics on overflow whatever the `overflow-checks` setting.
      ///
      /// # Panics
      ///
      /// Panics if `rhs` is zero, or if the quotient is outside the range of the type, as it is
      /// only for `Int` when `self` is `MIN` and `rhs` is -1.
      #[track_caller]
      fn strict_rem_euclid(&self, rhs: &Self) -> Self
        = prim div_rem_euclid_bits(self, rhs; Self::BITS) as division {
          match division {
            Some((_, remainder, false)) => Self::of(remainder),
            Some((_, _, true)) => $crate::integer::arithmetic::overflow::rem(),
            None => $crate::integer::arithmetic::domain::rem_by_zero(),
          }
        }

      /// Strict exponentiation: `self^exp`, which panics on overflow whatever the
      /// `overflow-checks` setting.
      ///
      /// # Panics
      ///
      /// Panics if the power is outside the range of the type, as `self^0`, one, is at zero bits
      /// and for `Int` of one bit.
      #[track_caller]
      fn strict_pow(&self, exp: u32) -> Self {
        match self.checked_pow(exp) {
          Some(power) => power,
          None => $crate::integer::arithmetic::overflow::pow(),
        }
      }

      /// Strict shift left: `self << rhs`, which panics if `rhs` is `Bits` or more whatever the
      /// `overflow-checks` setting.
      ///
      /// # Panics
      ///
      /// Panics if `rhs` is `Bits` or more, as every amount is at zero bits.
      #[track_caller]
      fn strict_shl(&self, rhs: u32) -> Self {
        match self.checked_shl(rhs) {
          Some(shifted) => shifted,
          None => $crate::integer::arithmetic::overflow::shl(),
        }
      }

      /// Strict shift right: `self >> rhs`, arithmetic for `Int`, which panics if `rhs` is `Bits`
      /// or more whatever the `overflow-checks` setting.
      ///
      /// # Panics
      ///
      /// Panics if `rhs` is `Bits` or more, as every amount is at zero bits.
      #[track_caller]
      fn strict_shr(&self, rhs: u32) -> Self {
        match self.checked_shr(rhs) {
          Some(shifted) => shifted,
          None => $crate::integer::arithmetic::overflow::shr(),
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
      fn carrying_add(&self, rhs: &Self, carry: bool) -> (Self, bool)
        = prim pair carrying_add_bits(self, rhs; carry);

      /// Subtraction with a borrow: `self - rhs - borrow` wrapped to the range of the type, and
      /// whether the exact difference is negative.
      fn borrowing_sub(&self, rhs: &Self, borrow: bool) -> (Self, bool)
        = prim pair borrowing_sub_bits(self, rhs; borrow);
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
        match self.checked_neg() {
          Some(negated) => negated,
          None => Self::bound(true),
        }
      }

      /// Overflowing absolute value: `|self|` wrapped to the range of the type, and whether it is
      /// outside that range, as it is for `MIN`, except at zero bits, where `MIN` is zero. `MIN`
      /// wraps to itself.
      fn overflowing_abs(&self) -> (Self, bool) {
        if self.is_negative() {
          self.overflowing_neg()
        } else {
          (self.duplicate(), false)
        }
      }

      /// Checked absolute value: `|self|`, or `None` if it is outside the range of the type, as it
      /// is for `MIN`, except at zero bits, where `MIN` is zero.
      fn checked_abs(&self) -> Option<Self> {
        if self.is_negative() {
          self.checked_neg()
        } else {
          Some(self.duplicate())
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
        match self.checked_abs() {
          Some(abs) => abs,
          None => Self::bound(true),
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
        match self.checked_abs() {
          Some(abs) => abs,
          None => $crate::integer::arithmetic::overflow::neg(),
        }
      }

      /// The absolute value, `|self|`. The absolute value of `MIN` is outside the range of the
      /// type, except at zero bits, where `MIN` is zero: the method then panics or returns `MIN`, as
      /// `overflow-checks` decides for the native integers (see [Overflow](crate::Int#overflow)).
      #[track_caller]
      fn abs(&self) -> Self {
        match self.checked_abs() {
          Some(abs) => abs,
          // Only `MIN` overflows, and its wrapped absolute value is `MIN` itself.
          None => {
            $crate::integer::arithmetic::overflow::forward_neg();
            self.duplicate()
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

/// Passes the multiplication with a carry and the multiplication with a carry and an addend of
/// `Uint` to `$callback`, after the tokens of `$args`.
macro_rules! uint_wide_table {
  ($callback:ident! $args:tt) => {
    $callback! { $args
      /// Multiplication with a carry: the low and the high half of `self * rhs + carry`, the
      /// integers `low` and `high` of `Bits` bits with `self * rhs + carry = low + high * 2^Bits`.
      /// Nothing overflows: the exact value is at most `(2^Bits - 1)^2 + 2^Bits - 1`, below
      /// `2^(2 * Bits)`, which the two halves hold. A digit of the product of a number of several
      /// digits of `Bits` bits by one digit is such a low half, and its high half is the carry into
      /// the next digit.
      fn carrying_mul(&self, rhs: &Self, carry: &Self) -> (Self, Self)
        = prim carrying_mul_bits(self, rhs, carry; Self::BITS) as halves {
          let (low, high) = halves;
          (Self::of(low), Self::of(high))
        }

      /// Multiplication with a carry and an addend: the low and the high half of
      /// `self * rhs + carry + add`, the integers `low` and `high` of `Bits` bits with
      /// `self * rhs + carry + add = low + high * 2^Bits`. Nothing overflows: the exact value is at
      /// most `(2^Bits - 1)^2 + 2 * (2^Bits - 1) = 2^(2 * Bits) - 1`, which the two halves hold.
      /// The addend is a digit of the product so far, as in a product of two numbers of several
      /// digits.
      fn carrying_mul_add(&self, rhs: &Self, carry: &Self, add: &Self) -> (Self, Self)
        = prim carrying_mul_add_bits(self, rhs, carry, add; Self::BITS) as halves {
          let (low, high) = halves;
          (Self::of(low), Self::of(high))
        }
    }
  };
}

/// Passes the carrying addition and the borrowing subtraction of `Int`, and its multiplication with
/// a carry and with a carry and an addend, to `$callback`, after the tokens of `$args`. The low
/// half of a product is the `Uint` of the same width and storage size, which an inherent block
/// passes last in `$args`, for `cast_other!`.
macro_rules! int_wide_table {
  ($callback:ident! $args:tt) => {
    $callback! { $args
      /// Addition with a carry: `self + rhs + carry` wrapped to the range of the type, and whether
      /// the exact sum is outside that range. As for the native signed integers, the flag is an
      /// overflow of the range, not a carry out of the top bit: the method adds the most
      /// significant digits of two numbers of several digits, whose lower digits are `Uint`s that
      /// pass it their carry, and its flag says whether the sum of the numbers overflows.
      fn carrying_add(&self, rhs: &Self, carry: bool) -> (Self, bool)
        = prim pair carrying_add_bits(self, rhs; carry);

      /// Subtraction with a borrow: `self - rhs - borrow` wrapped to the range of the type, and
      /// whether the exact difference is outside that range. As for
      /// [`carrying_add`](Self::carrying_add), the flag is an overflow of the range, not a borrow
      /// past the top bit, so that the method subtracts the most significant digits of two numbers
      /// of several digits.
      fn borrowing_sub(&self, rhs: &Self, borrow: bool) -> (Self, bool)
        = prim pair borrowing_sub_bits(self, rhs; borrow);

      /// Multiplication with a carry: the low and the high half of `self * rhs + carry`, with
      /// `self * rhs + carry = low + high * 2^Bits`. The low half is the `Uint` of the same width
      /// in a storage of the same size, from zero to `2^Bits - 1`, and the high half is negative
      /// exactly when the exact value is. Nothing overflows: the exact value is from
      /// `MIN * MAX + MIN` to `MIN * MIN + MAX`, within the range of `2 * Bits` bits, which the two
      /// halves hold.
      fn carrying_mul(&self, rhs: &Self, carry: &Self) -> (cast_other!($args, Unsigned), Self)
        = prim storage carrying_mul_bits(self, rhs, carry; Self::BITS) as halves {
          let (low, high) = halves;
          (Uint::wrap(low), Self::of(high))
        }

      /// Multiplication with a carry and an addend: the low and the high half of
      /// `self * rhs + carry + add`, with `self * rhs + carry + add = low + high * 2^Bits`, as
      /// [`carrying_mul`](Self::carrying_mul) gives them. Nothing overflows: the exact value is
      /// from `MIN * MAX + 2 * MIN` to `MIN * MIN + 2 * MAX`, within the range of `2 * Bits` bits
      /// at every width, which the two halves hold.
      fn carrying_mul_add(
        &self,
        rhs: &Self,
        carry: &Self,
        add: &Self
      ) -> (cast_other!($args, Unsigned), Self)
        = prim storage carrying_mul_add_bits(self, rhs, carry, add; Self::BITS) as halves {
          let (low, high) = halves;
          (Uint::wrap(low), Self::of(high))
        }
    }
  };
}

/// Passes the integer functions of `Uint` and `Int` that have no overflow prefix to `$callback`,
/// after the tokens of `$args`: the power, the Euclidean, rounded and exact division, the next
/// multiple, the roots, the logarithms, the midpoint and the absolute difference.
macro_rules! integer_math_table {
  ($callback:ident! $args:tt) => {
    $callback! { $args
      /// Exponentiation: `self^exp`. A power outside the range of the type overflows: the method
      /// then panics or returns the wrapped power, as `overflow-checks` decides for the native
      /// integers (see [Overflow](crate::Uint#overflow)), and in a constant it fails to compile in
      /// every build.
      fn pow(&self, exp: u32) -> Self
        = prim overflowing_pow_bits(self; exp, Self::BITS) as power {
          match power {
            (power, false) => Self::of(power),
            (power, true) => {
              $crate::integer::arithmetic::overflow::forward_pow();
              Self::of(power)
            }
          }
        }

      /// Euclidean division: the quotient `q` of `self = q * rhs + r` with `0 <= r < |rhs|`, which
      /// for `Uint` is the quotient rounded toward zero.
      ///
      /// # Panics
      ///
      /// Panics if `rhs` is zero, or if the quotient is outside the range of the type, as it is
      /// only for `Int` when `self` is `MIN` and `rhs` is -1, in every build, as the native
      /// method does.
      #[track_caller]
      fn div_euclid(&self, rhs: &Self) -> Self {
        self.strict_div_euclid(rhs)
      }

      /// Euclidean remainder: the remainder `r` of `self = q * rhs + r` with `0 <= r < |rhs|`,
      /// which for `Uint` is the remainder of the division rounded toward zero.
      ///
      /// # Panics
      ///
      /// Panics if `rhs` is zero, or if the quotient is outside the range of the type, as it is
      /// only for `Int` when `self` is `MIN` and `rhs` is -1, in every build, as the native
      /// method does.
      #[track_caller]
      fn rem_euclid(&self, rhs: &Self) -> Self {
        self.strict_rem_euclid(rhs)
      }

      /// Division rounded down: `self / rhs` rounded toward negative infinity, which for `Uint` is
      /// the quotient rounded toward zero.
      ///
      /// # Panics
      ///
      /// Panics if `rhs` is zero, or if the quotient is outside the range of the type, as it is
      /// only for `Int` when `self` is `MIN` and `rhs` is -1, in every build.
      #[track_caller]
      fn div_floor(&self, rhs: &Self) -> Self
        = prim div_floor_bits(self, rhs; Self::BITS) as division {
          match division {
            Some((quotient, false)) => Self::of(quotient),
            Some((_, true)) => $crate::integer::arithmetic::overflow::div(),
            None => $crate::integer::arithmetic::domain::div_by_zero(),
          }
        }

      /// Division rounded up: `self / rhs` rounded toward positive infinity.
      ///
      /// # Panics
      ///
      /// Panics if `rhs` is zero, or if the quotient is outside the range of the type, as it is
      /// only for `Int` when `self` is `MIN` and `rhs` is -1, in every build.
      #[track_caller]
      fn div_ceil(&self, rhs: &Self) -> Self
        = prim div_ceil_bits(self, rhs; Self::BITS) as division {
          match division {
            Some((quotient, false)) => Self::of(quotient),
            Some((_, true)) => $crate::integer::arithmetic::overflow::div(),
            None => $crate::integer::arithmetic::domain::div_by_zero(),
          }
        }

      /// Next multiple: for a positive `rhs`, the smallest multiple of `rhs` at least `self`, and
      /// for a negative `rhs`, which only `Int` has, the largest multiple of `rhs` at most `self`.
      /// A multiple outside the range of the type overflows: the method then panics or returns the
      /// wrapped multiple, as `overflow-checks` decides for the native integers (see
      /// [Overflow](crate::Uint#overflow)).
      ///
      /// # Panics
      ///
      /// Panics if `rhs` is zero, with the message of the native remainder, which the native
      /// method computes first, in every build.
      #[track_caller]
      fn next_multiple_of(&self, rhs: &Self) -> Self
        = prim next_multiple_of_bits(self, rhs; Self::BITS) as multiple {
          match multiple {
            Some((multiple, false)) => Self::of(multiple),
            Some((multiple, true)) => {
              $crate::integer::arithmetic::overflow::forward_add();
              Self::of(multiple)
            }
            None => $crate::integer::arithmetic::domain::rem_by_zero(),
          }
        }

      /// Exact division: `self / rhs` if `rhs` divides `self`, or `None` if the division leaves a
      /// remainder.
      ///
      /// # Panics
      ///
      /// Panics if `rhs` is zero, or if `self` is `MIN` and `rhs` is -1 for `Int`, in every build,
      /// with the messages of the native remainder. The native `div_exact` is unstable
      /// ([#139911](https://github.com/rust-lang/rust/issues/139911)); this method follows what it
      /// does, which computes `self % rhs` first, rather than its documentation.
      #[track_caller]
      fn div_exact(&self, rhs: &Self) -> Option<Self>
        = prim div_rem_bits(self, rhs; Self::BITS) as division {
          match division {
            Some((quotient, remainder, false)) => {
              if Self::holds_zero(&remainder) {
                Some(Self::of(quotient))
              } else {
                None
              }
            }
            // The remainder of `MIN / -1` and of a zero divisor panics, in every build.
            Some((_, _, true)) => $crate::integer::arithmetic::overflow::rem(),
            None => $crate::integer::arithmetic::domain::rem_by_zero(),
          }
        }

      /// Square root: `self` to the power one half, rounded down.
      ///
      /// # Panics
      ///
      /// Panics if `self` is negative, which only `Int` can be.
      #[track_caller]
      fn isqrt(&self) -> Self
        = prim checked_isqrt(self) as root {
          match root {
            Some(root) => Self::of(root),
            // Every unsigned value has a square root.
            None => $crate::integer::arithmetic::domain::sqrt_of_negative(),
          }
        }

      /// Checked root: the `n`th root of `self`, rounded toward zero, or `None` if `n` is zero or
      /// `self` is negative and `n` even.
      fn checked_nth_root(&self, n: u32) -> Option<Self>
        = prim checked_nth_root(self; n) as root {
          match root {
            Some(root) => Some(Self::of(root)),
            None => None,
          }
        }

      /// Root: the `n`th root of `self`, rounded toward zero: the largest `r` with `r^n <= self`
      /// for a value at least zero, and the negation of the root of `-self` for a negative value
      /// and an odd `n`.
      ///
      /// # Panics
      ///
      /// Panics if `n` is zero, and otherwise if `self` is negative and `n` even, which only `Int`
      /// can be.
      #[track_caller]
      fn nth_root(&self, n: u32) -> Self {
        if n == 0 {
          $crate::integer::arithmetic::domain::root_degree();
        }
        match self.checked_nth_root(n) {
          Some(root) => root,
          None => $crate::integer::arithmetic::domain::root_of_negative(),
        }
      }

      /// Checked logarithm: the logarithm of `self` to the base `base`, rounded down, or `None` if
      /// `self` is not positive or `base` is below 2.
      fn checked_ilog(&self, base: &Self) -> Option<u32> = prim checked_ilog(self, base);

      /// Checked base-2 logarithm: the logarithm of `self` to the base 2, rounded down, or `None`
      /// if `self` is not positive.
      fn checked_ilog2(&self) -> Option<u32> = prim checked_ilog2(self);

      /// Checked base-10 logarithm: the logarithm of `self` to the base 10, rounded down, or
      /// `None` if `self` is not positive.
      fn checked_ilog10(&self) -> Option<u32> = prim checked_ilog10(self);

      /// Logarithm: the logarithm of `self` to the base `base`, rounded down.
      ///
      /// # Panics
      ///
      /// Panics if `base` is below 2, and otherwise if `self` is not positive, as the native
      /// method does.
      #[track_caller]
      fn ilog(&self, base: &Self) -> u32 {
        // A base below 2 has no base-2 logarithm of at least 1.
        if !matches!(base.checked_ilog2(), Some(1..)) {
          $crate::integer::arithmetic::domain::log_base();
        }
        match self.checked_ilog(base) {
          Some(log) => log,
          None => $crate::integer::arithmetic::domain::log_of_nonpositive(),
        }
      }

      /// Base-2 logarithm: the logarithm of `self` to the base 2, rounded down, the index of its
      /// highest set bit.
      ///
      /// # Panics
      ///
      /// Panics if `self` is not positive.
      #[track_caller]
      fn ilog2(&self) -> u32 {
        match self.checked_ilog2() {
          Some(log) => log,
          None => $crate::integer::arithmetic::domain::log_of_nonpositive(),
        }
      }

      /// Base-10 logarithm: the logarithm of `self` to the base 10, rounded down, one less than
      /// the number of its decimal digits.
      ///
      /// # Panics
      ///
      /// Panics if `self` is not positive.
      #[track_caller]
      fn ilog10(&self) -> u32 {
        match self.checked_ilog10() {
          Some(log) => log,
          None => $crate::integer::arithmetic::domain::log_of_nonpositive(),
        }
      }

      /// Midpoint: `(self + rhs) / 2` rounded toward zero, computed without overflow.
      fn midpoint(&self, rhs: &Self) -> Self = prim value midpoint(self, rhs);

      /// Absolute difference: `|self - other|`, as the unsigned integer of the same width, which
      /// holds it: `Self` for `Uint`, and for `Int` the `Uint` of the same width in a storage of
      /// the same size.
      fn abs_diff(&self, other: &Self) -> Self::Magnitude = prim unsigned abs_diff(self, other);
    }
  };
}

/// Passes the functions of `Uint` alone that have no overflow prefix to `$callback`, after the
/// tokens of `$args`: `is_multiple_of`, `is_power_of_two` and the next power of two.
macro_rules! uint_math_table {
  ($callback:ident! $args:tt) => {
    $callback! { $args
      /// Returns `true` if `self` is a multiple of `rhs`: if `self / rhs` leaves no remainder, or,
      /// for a zero `rhs`, if `self` is zero.
      fn is_multiple_of(&self, rhs: &Self) -> bool = prim is_multiple_of(self, rhs);

      /// Returns `true` if `self` is a power of two: if it has exactly one set bit. Zero bits hold
      /// only zero, which is not.
      fn is_power_of_two(&self) -> bool {
        self.count_ones() == 1
      }

      /// Next power of two: the smallest power of two at least `self`. A power outside the range
      /// of the type overflows, as one does at zero bits: the method then panics or returns zero,
      /// as `overflow-checks` decides for the native integers (see
      /// [Overflow](crate::Uint#overflow)).
      #[track_caller]
      fn next_power_of_two(&self) -> Self
        = prim next_power_of_two_bits(self; Self::BITS) as power {
          match power {
            (power, false) => Self::of(power),
            (power, true) => {
              $crate::integer::arithmetic::overflow::forward_add();
              Self::of(power)
            }
          }
        }

      /// Checked next power of two: the smallest power of two at least `self`, or `None` if it is
      /// outside the range of the type, as one is at zero bits.
      fn checked_next_power_of_two(&self) -> Option<Self>
        = prim next_power_of_two_bits(self; Self::BITS) as power {
          match power {
            (power, false) => Some(Self::of(power)),
            (_, true) => None,
          }
        }

      /// Wrapping next power of two: the smallest power of two at least `self`, or zero if it is
      /// outside the range of the type.
      fn wrapping_next_power_of_two(&self) -> Self
        = prim next_power_of_two_bits(self; Self::BITS) as power {
          Self::of(power.0)
        }
    }
  };
}

/// Passes the functions of `Int` alone that have no overflow prefix to `$callback`, after the
/// tokens of `$args`: the checked square root.
macro_rules! int_math_table {
  ($callback:ident! $args:tt) => {
    $callback! { $args
      /// Checked square root: `self` to the power one half, rounded down, or `None` if `self` is
      /// negative.
      fn checked_isqrt(&self) -> Option<Self>
        = prim checked_isqrt(self) as root {
          match root {
            Some(root) => Some(Self::of(root)),
            None => None,
          }
        }
    }
  };
}

/// Passes the unchecked addition, subtraction, multiplication, exact division and shifts of `Uint`
/// and `Int` to `$callback`, after the tokens of `$args`, for the traits alone: each storage writes
/// out its inherent `unchecked_*` methods.
macro_rules! unchecked_table {
  ($callback:ident! $args:tt) => {
    $callback! { $args
      /// Unchecked addition: `self + rhs`, assuming it cannot overflow.
      ///
      /// # Safety
      ///
      /// The sum must be at most `MAX` for `Uint` and within `MIN..=MAX` for `Int`, that is,
      /// [`checked_add`](crate::CheckedOps::checked_add) would return `Some`. Otherwise the
      /// behavior is undefined.
      #[track_caller]
      unsafe fn unchecked_add(&self, rhs: &Self) -> Self
        = unchecked overflowing_add(self, rhs), "attempt to add with overflow";

      /// Unchecked subtraction: `self - rhs`, assuming it cannot overflow.
      ///
      /// # Safety
      ///
      /// For `Uint`, `rhs` must be at most `self`, and for `Int`, the difference must be within
      /// `MIN..=MAX`, that is, [`checked_sub`](crate::CheckedOps::checked_sub) would return `Some`.
      /// Otherwise the behavior is undefined.
      #[track_caller]
      unsafe fn unchecked_sub(&self, rhs: &Self) -> Self
        = unchecked overflowing_sub(self, rhs), "attempt to subtract with overflow";

      /// Unchecked multiplication: `self * rhs`, assuming it cannot overflow.
      ///
      /// # Safety
      ///
      /// The product must be at most `MAX` for `Uint` and within `MIN..=MAX` for `Int`, that is,
      /// [`checked_mul`](crate::CheckedOps::checked_mul) would return `Some`. Otherwise the
      /// behavior is undefined.
      #[track_caller]
      unsafe fn unchecked_mul(&self, rhs: &Self) -> Self
        = unchecked overflowing_mul(self, rhs), "attempt to multiply with overflow";

      /// Unchecked exact division: `self / rhs`, assuming that `rhs` divides `self`.
      ///
      /// # Safety
      ///
      /// `rhs` must not be zero, the division must leave no remainder, and the quotient must be
      /// within the range of the type, so that for `Int` `self` is not `MIN` with `rhs` -1: that
      /// is, [`checked_div_exact`](crate::CheckedOps::checked_div_exact) would return `Some`.
      /// Otherwise the behavior is undefined. Zero bits hold no divisor but zero, so no call is
      /// sound there.
      #[track_caller]
      unsafe fn unchecked_div_exact(&self, rhs: &Self) -> Self {
        let quotient = self.checked_div_exact(rhs);
        debug_assert!(
          quotient.is_some(),
          "attempt to divide exactly with a zero divisor, with overflow or with a remainder"
        );
        // Without debug assertions, a call that breaks the contract gets the wrapped quotient,
        // or the panic of a zero divisor.
        match quotient {
          Some(quotient) => quotient,
          None => self.wrapping_div(rhs),
        }
      }

      /// Unchecked shift left: `self << rhs`, assuming that `rhs` is below `Bits`.
      ///
      /// # Safety
      ///
      /// `rhs` must be below `Bits`, that is, [`checked_shl`](crate::CheckedOps::checked_shl)
      /// would return `Some`. Otherwise the behavior is undefined. Zero bits have no amount below
      /// `Bits`, so no call is sound there.
      #[track_caller]
      unsafe fn unchecked_shl(&self, rhs: u32) -> Self
        = unchecked overflowing_shl(self, rhs), "attempt to shift left with overflow";

      /// Unchecked shift right: `self >> rhs`, arithmetic for `Int`, assuming that `rhs` is below
      /// `Bits`.
      ///
      /// # Safety
      ///
      /// `rhs` must be below `Bits`, that is, [`checked_shr`](crate::CheckedOps::checked_shr)
      /// would return `Some`. Otherwise the behavior is undefined. Zero bits have no amount below
      /// `Bits`, so no call is sound there.
      #[track_caller]
      unsafe fn unchecked_shr(&self, rhs: u32) -> Self
        = unchecked overflowing_shr(self, rhs), "attempt to shift right with overflow";
    }
  };
}

/// Passes the unchecked negation of `Int` to `$callback`, after the tokens of `$args`, for the
/// trait of its sign alone, as `unchecked_table!` does.
macro_rules! signed_unchecked_table {
  ($callback:ident! $args:tt) => {
    $callback! { $args
      /// Unchecked negation: `-self`, assuming it cannot overflow.
      ///
      /// # Safety
      ///
      /// The negation must be within `MIN..=MAX`, that is,
      /// [`checked_neg`](crate::CheckedOps::checked_neg) would return `Some`: `self` must not be
      /// `MIN`, except at zero bits, where `MIN` is zero. Otherwise the behavior is undefined.
      #[track_caller]
      unsafe fn unchecked_neg(&self) -> Self
        = unchecked overflowing_neg(self), "attempt to negate with overflow";
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

  /// Panics as the native `strict_div` does on overflow, and the native `/` in every build.
  #[cold]
  #[track_caller]
  pub(crate) const fn div() -> ! {
    panic!("attempt to divide with overflow")
  }

  /// Panics as the native `strict_rem` does on overflow, and the native `%` in every build.
  #[cold]
  #[track_caller]
  pub(crate) const fn rem() -> ! {
    panic!("attempt to calculate the remainder with overflow")
  }

  /// Panics as the native `strict_pow` does.
  #[cold]
  #[track_caller]
  pub(crate) const fn pow() -> ! {
    panic!("attempt to exponentiate with overflow")
  }

  /// Panics as the native `strict_shl` does.
  #[cold]
  #[track_caller]
  pub(crate) const fn shl() -> ! {
    panic!("attempt to shift left with overflow")
  }

  /// Panics as the native `strict_shr` does.
  #[cold]
  #[track_caller]
  pub(crate) const fn shr() -> ! {
    panic!("attempt to shift right with overflow")
  }

  /// Panics as the native `strict_cast` does.
  #[cold]
  #[track_caller]
  pub(crate) const fn cast() -> ! {
    panic!("attempt to cast integer with overflow")
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

  /// Repeats the overflow of a native shift left, as [`forward_add`] does: a shift by the width of
  /// the native integer, which the native operator reports with the message of a shift left.
  #[inline]
  #[track_caller]
  pub(crate) const fn forward_shl() {
    let _ = black_box(1u8) << black_box(8u32);
  }

  /// Repeats the overflow of a native shift right, as [`forward_shl`] does for a shift left.
  #[inline]
  #[track_caller]
  pub(crate) const fn forward_shr() {
    let _ = black_box(1u8) >> black_box(8u32);
  }

  /// Repeats the overflow of a native exponentiation, as [`forward_add`] does. The native `pow`
  /// is not `#[track_caller]`, so the panic reports a line in `core`, as that of the native `pow`
  /// does. It reads `overflow-checks` where the native `pow` is instantiated, so the setting of the
  /// crate that calls this decides, also in an unoptimized build, and a constant evaluates it
  /// with the checks on.
  #[inline]
  pub(crate) const fn forward_pow() {
    let _ = black_box(u8::MAX).pow(2);
  }
}

/// The panics of an operand outside the domain of an operation, such as a zero divisor, with the
/// messages of the native integers. They are not overflows: every build panics on them, whatever
/// the `overflow-checks` setting.
pub(super) mod domain {
  /// Panics as the native division does on a zero divisor.
  #[cold]
  #[track_caller]
  pub(crate) const fn div_by_zero() -> ! {
    panic!("attempt to divide by zero")
  }

  /// Panics as the native remainder does on a zero divisor.
  #[cold]
  #[track_caller]
  pub(crate) const fn rem_by_zero() -> ! {
    panic!("attempt to calculate the remainder with a divisor of zero")
  }

  /// Panics as the native `isqrt` does on a negative value.
  #[cold]
  #[track_caller]
  pub(crate) const fn sqrt_of_negative() -> ! {
    panic!("argument of integer square root cannot be negative")
  }

  /// Panics as the native logarithms do on a value that is not positive.
  #[cold]
  #[track_caller]
  pub(crate) const fn log_of_nonpositive() -> ! {
    panic!("argument of integer logarithm must be positive")
  }

  /// Panics as the native `ilog` does on a base below 2.
  #[cold]
  #[track_caller]
  pub(crate) const fn log_base() -> ! {
    panic!("base of integer logarithm must be at least 2")
  }

  /// Panics on a root of degree zero, in the words of the other panics.
  #[cold]
  #[track_caller]
  pub(crate) const fn root_degree() -> ! {
    panic!("degree of integer root must be at least 1")
  }

  /// Panics on a root of even degree of a negative value, in the words of the other panics.
  #[cold]
  #[track_caller]
  pub(crate) const fn root_of_negative() -> ! {
    panic!("argument of integer root of even degree cannot be negative")
  }

  /// Panics on a bit index of `Bits` or more, where no bit of the value can be set or cleared.
  #[cold]
  #[track_caller]
  pub(crate) const fn bit_index() -> ! {
    panic!("bit index out of range")
  }
}

/// Implements the binary operator `$op` in its four forms of values and references, and its
/// compound assignment `$assign` for a value and a reference, for every storage, `$repr`, and the
/// assignments for a storage that holds the value in place, `$repr_mut`. Every form computes with
/// the sealed storage method `$primitive` on the inline storages that the operands lend, and on
/// overflow repeats a native overflow with `$forward`, which panics or not as `overflow-checks`
/// decides; without a panic, the result is the wrapped value. The form on two references then
/// allocates the result, the forms with an owned operand store it in that operand, the left one
/// when both are owned, and the compound assignments store it in their target. Each builds or
/// stores the result after `$forward`, so that a panic builds nothing and leaves the target of an
/// assignment unchanged, as for the native integers.
macro_rules! binary_operator {
  // The value of `$primitive` on the inline storages that `$lhs` and `$rhs` lend, at `$bits` bits,
  // after the overflow, which `$forward` repeats.
  (@compute $lhs:expr, $rhs:expr, $bits:expr, $primitive:ident, $forward:ident) => {{
    let (value, overflow) =
      $lhs.inline(Token::new()).$primitive($rhs.inline(Token::new()), $bits, Token::new());
    if overflow {
      overflow::$forward();
    }
    value
  }};
  (
    $type:ident: $repr:ident, $repr_mut:ident;
    $op:ident $method:ident,
    $assign:ident $assign_method:ident,
    $primitive:ident, $forward:ident
  ) => {
    impl<B: FitsBits<R>, R: $repr> $op for $type<B, R> {
      type Output = Self;

      #[inline]
      #[track_caller]
      fn $method(mut self, rhs: Self) -> Self {
        let value =
          binary_operator!(@compute self.repr, rhs.repr, Self::BITS, $primitive, $forward);
        self.repr.set_inline(value, Token::new());
        self
      }
    }

    impl<B: FitsBits<R>, R: $repr> $op<&$type<B, R>> for $type<B, R> {
      type Output = Self;

      #[inline]
      #[track_caller]
      fn $method(mut self, rhs: &Self) -> Self {
        let value =
          binary_operator!(@compute self.repr, rhs.repr, Self::BITS, $primitive, $forward);
        self.repr.set_inline(value, Token::new());
        self
      }
    }

    impl<B: FitsBits<R>, R: $repr> $op<$type<B, R>> for &$type<B, R> {
      type Output = $type<B, R>;

      #[inline]
      #[track_caller]
      fn $method(self, mut rhs: $type<B, R>) -> $type<B, R> {
        let bits = $type::<B, R>::BITS;
        let value = binary_operator!(@compute self.repr, rhs.repr, bits, $primitive, $forward);
        rhs.repr.set_inline(value, Token::new());
        rhs
      }
    }

    impl<B: FitsBits<R>, R: $repr> $op<&$type<B, R>> for &$type<B, R> {
      type Output = $type<B, R>;

      #[inline]
      #[track_caller]
      fn $method(self, rhs: &$type<B, R>) -> $type<B, R> {
        let bits = $type::<B, R>::BITS;
        let value = binary_operator!(@compute self.repr, rhs.repr, bits, $primitive, $forward);
        $type::wrap(R::from_inline(value, Token::new()))
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
        let value =
          binary_operator!(@compute self.repr, rhs.repr, Self::BITS, $primitive, $forward);
        self.repr.set_inline(value, Token::new());
      }
    }
  };
}

binary_operator!(
  Uint: UintRepr, UintReprMut;
  Add add, AddAssign add_assign, overflowing_add_bits, forward_add
);
binary_operator!(
  Uint: UintRepr, UintReprMut;
  Sub sub, SubAssign sub_assign, overflowing_sub_bits, forward_sub
);
binary_operator!(
  Uint: UintRepr, UintReprMut;
  Mul mul, MulAssign mul_assign, overflowing_mul_bits, forward_mul
);
binary_operator!(
  Int: IntRepr, IntReprMut;
  Add add, AddAssign add_assign, overflowing_add_bits, forward_add
);
binary_operator!(
  Int: IntRepr, IntReprMut;
  Sub sub, SubAssign sub_assign, overflowing_sub_bits, forward_sub
);
binary_operator!(
  Int: IntRepr, IntReprMut;
  Mul mul, MulAssign mul_assign, overflowing_mul_bits, forward_mul
);

/// Implements the division operator `$op`, `/` or `%`, in its four forms of values and references,
/// and its compound assignment `$assign` for a value and a reference: as for the native integers, a
/// zero divisor and an overflow panic whatever the `overflow-checks` setting, and nothing wraps.
/// The form on two references is the strict method `$strict` of the same operation, for every
/// storage, `$repr`. The forms with an owned operand, for every storage too, compute the quotient
/// and the remainder on the inline storages that the operands lend, with the panics of `$strict`,
/// `$by_zero` and `$overflow`, and store the element `$part` of them in the owned operand, the left
/// one when both are owned; the compound assignments, for a storage that holds the value in place,
/// `$repr_mut`, store it in their target. Each stores after the division, so that a panic leaves
/// the target of an assignment unchanged.
macro_rules! division_operator {
  // The element `$part` of the quotient and the remainder of the inline storages that `$lhs` and
  // `$rhs` lend, at `$bits` bits: `$by_zero` panics on a zero divisor and `$overflow` on overflow,
  // in that order, as the strict methods panic.
  (@part $lhs:expr, $rhs:expr, $bits:expr, $part:tt, $by_zero:ident, $overflow:ident) => {
    match $lhs.inline(Token::new()).div_rem_bits($rhs.inline(Token::new()), $bits, Token::new()) {
      Some((quotient, remainder, false)) => (quotient, remainder).$part,
      Some((_, _, true)) => overflow::$overflow(),
      None => domain::$by_zero(),
    }
  };
  (
    $type:ident: $repr:ident, $repr_mut:ident;
    $op:ident $method:ident,
    $assign:ident $assign_method:ident,
    $strict:ident; $part:tt, $by_zero:ident, $overflow:ident
  ) => {
    impl<B: FitsBits<R>, R: $repr> $op for $type<B, R> {
      type Output = Self;

      #[inline]
      #[track_caller]
      fn $method(mut self, rhs: Self) -> Self {
        let value = division_operator!(
          @part self.repr, rhs.repr, Self::BITS, $part, $by_zero, $overflow
        );
        self.repr.set_inline(value, Token::new());
        self
      }
    }

    impl<B: FitsBits<R>, R: $repr> $op<&$type<B, R>> for $type<B, R> {
      type Output = Self;

      #[inline]
      #[track_caller]
      fn $method(mut self, rhs: &Self) -> Self {
        let value = division_operator!(
          @part self.repr, rhs.repr, Self::BITS, $part, $by_zero, $overflow
        );
        self.repr.set_inline(value, Token::new());
        self
      }
    }

    impl<B: FitsBits<R>, R: $repr> $op<$type<B, R>> for &$type<B, R> {
      type Output = $type<B, R>;

      #[inline]
      #[track_caller]
      fn $method(self, mut rhs: $type<B, R>) -> $type<B, R> {
        let bits = $type::<B, R>::BITS;
        let value =
          division_operator!(@part self.repr, rhs.repr, bits, $part, $by_zero, $overflow);
        rhs.repr.set_inline(value, Token::new());
        rhs
      }
    }

    impl<B: FitsBits<R>, R: $repr> $op<&$type<B, R>> for &$type<B, R> {
      type Output = $type<B, R>;

      #[inline]
      #[track_caller]
      fn $method(self, rhs: &$type<B, R>) -> $type<B, R> {
        StrictOps::$strict(self, rhs)
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
        let value = division_operator!(
          @part self.repr, rhs.repr, Self::BITS, $part, $by_zero, $overflow
        );
        self.repr.set_inline(value, Token::new());
      }
    }
  };
}

division_operator!(
  Uint: UintRepr, UintReprMut;
  Div div, DivAssign div_assign, strict_div; 0, div_by_zero, div
);
division_operator!(
  Uint: UintRepr, UintReprMut;
  Rem rem, RemAssign rem_assign, strict_rem; 1, rem_by_zero, rem
);
division_operator!(
  Int: IntRepr, IntReprMut;
  Div div, DivAssign div_assign, strict_div; 0, div_by_zero, div
);
division_operator!(
  Int: IntRepr, IntReprMut;
  Rem rem, RemAssign rem_assign, strict_rem; 1, rem_by_zero, rem
);

// As for the binary operators, the negation computes on the inline storage that its operand lends
// and repeats an overflow; the negation of a reference then allocates the result, and the negation
// of a value stores it in that value.
impl<B: FitsBits<R>, R: IntRepr> Neg for Int<B, R> {
  type Output = Self;

  #[inline]
  #[track_caller]
  fn neg(mut self) -> Self {
    let (value, overflow) = self
      .repr
      .inline(Token::new())
      .overflowing_neg_bits(Self::BITS, Token::new());
    if overflow {
      overflow::forward_neg();
    }
    self.repr.set_inline(value, Token::new());
    self
  }
}

impl<B: FitsBits<R>, R: IntRepr> Neg for &Int<B, R> {
  type Output = Int<B, R>;

  #[inline]
  #[track_caller]
  fn neg(self) -> Int<B, R> {
    let (value, overflow) = self
      .repr
      .inline(Token::new())
      .overflowing_neg_bits(Int::<B, R>::BITS, Token::new());
    if overflow {
      overflow::forward_neg();
    }
    Int::wrap(R::from_inline(value, Token::new()))
  }
}

/// One in the inline storage of `R`: bit 0 alone, set in the inline zero, which `Product` starts
/// from. The width holds one, so that bit exists.
#[inline]
fn unsigned_inline_one<B: FitsBits<R>, R: UintRepr>() -> R::Inline {
  match <R::Inline as UintCore>::zero(Token::new()).with_bit(0, true, B::U32, Token::new()) {
    Some(one) => one,
    None => nonzero::zero(),
  }
}

/// One in the inline storage of `R`, as [`unsigned_inline_one`] gives it: the width has two bits
/// or more, so that bit 0 alone is one, not -1.
#[inline]
fn signed_inline_one<B: FitsBits<R>, R: IntRepr>() -> R::Inline {
  match <R::Inline as IntCore>::zero(Token::new()).with_bit(0, true, B::U32, Token::new()) {
    Some(one) => one,
    None => nonzero::zero(),
  }
}

/// One in the storage `R`, which `IntegerOne` returns and `Product` gives for no item: the inline
/// one, stored once.
#[inline]
pub(super) fn unsigned_one<B: FitsBits<R>, R: UintRepr>() -> Uint<B, R> {
  Uint::wrap(R::from_inline(unsigned_inline_one::<B, R>(), Token::new()))
}

/// One in the storage `R`, as [`unsigned_one`] gives it.
#[inline]
pub(super) fn signed_one<B: FitsBits<R>, R: IntRepr>() -> Int<B, R> {
  Int::wrap(R::from_inline(signed_inline_one::<B, R>(), Token::new()))
}

/// Implements `Sum`, which adds from zero, and `Product`, which multiplies from one for a width
/// bounded by `$width`, for values and references, for every storage, `$repr`, whose sealed trait
/// is `$core`. Each computes on the inline storages that the items lend, with the sealed storage
/// method of the operator, and repeats the overflow of a step as the operator does. A fold of
/// values stores the result in the first item, and gives the sealed `zero`, or `$one()`, for no
/// item; a fold of references builds the result once, from the inline zero or `$inline_one()`.
macro_rules! folds {
  // The running value `$total`, an inline storage, combined with the value that `$repr` lends by
  // the sealed storage method `$primitive`, at `$bits` bits, after the overflow, which `$forward`
  // repeats.
  (@step $total:expr, $repr:expr, $bits:expr, $primitive:ident, $forward:ident) => {{
    let (total, overflow) = $total.$primitive($repr.inline(Token::new()), $bits, Token::new());
    if overflow {
      overflow::$forward();
    }
    total
  }};
  ($type:ident: $repr:ident, $core:ident, $one:ident, $inline_one:ident, $($width:tt)+) => {
    impl<B: FitsBits<R>, R: $repr> Sum for $type<B, R> {
      fn sum<I: Iterator<Item = Self>>(mut iter: I) -> Self {
        let Some(mut first) = iter.next() else {
          return Self::wrap(R::zero(Token::new()));
        };
        // Zero takes the value of the first item in a step that cannot overflow: the inline
        // storage is not known to be `Copy`.
        let zero = <R::Inline as $core>::zero(Token::new());
        let mut total =
          folds!(@step zero, first.repr, Self::BITS, overflowing_add_bits, forward_add);
        for value in iter {
          total = folds!(@step total, value.repr, Self::BITS, overflowing_add_bits, forward_add);
        }
        first.repr.set_inline(total, Token::new());
        first
      }
    }

    impl<'a, B: FitsBits<R>, R: $repr> Sum<&'a $type<B, R>> for $type<B, R> {
      fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        let mut total = <R::Inline as $core>::zero(Token::new());
        for value in iter {
          total = folds!(@step total, value.repr, Self::BITS, overflowing_add_bits, forward_add);
        }
        Self::wrap(R::from_inline(total, Token::new()))
      }
    }

    impl<B: FitsBits<R> + $($width)+, R: $repr> Product for $type<B, R> {
      fn product<I: Iterator<Item = Self>>(mut iter: I) -> Self {
        let Some(mut first) = iter.next() else {
          return $one();
        };
        // One takes the value of the first item, as zero does in the sum.
        let one = $inline_one::<B, R>();
        let mut total =
          folds!(@step one, first.repr, Self::BITS, overflowing_mul_bits, forward_mul);
        for value in iter {
          total = folds!(@step total, value.repr, Self::BITS, overflowing_mul_bits, forward_mul);
        }
        first.repr.set_inline(total, Token::new());
        first
      }
    }

    impl<'a, B: FitsBits<R> + $($width)+, R: $repr> Product<&'a $type<B, R>> for $type<B, R> {
      fn product<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        let mut total = $inline_one::<B, R>();
        for value in iter {
          total = folds!(@step total, value.repr, Self::BITS, overflowing_mul_bits, forward_mul);
        }
        Self::wrap(R::from_inline(total, Token::new()))
      }
    }
  };
}

folds!(Uint: UintRepr, UintCore, unsigned_one, unsigned_inline_one, NonZero);
folds!(Int: IntRepr, IntCore, signed_one, signed_inline_one, IsGreater<U1, Output = True>);

#[cfg(test)]
mod tests;
