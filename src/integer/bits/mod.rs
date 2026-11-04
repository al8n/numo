//! The bit operations of [`Uint`](crate::Uint) and [`Int`](crate::Int): the method tables of the
//! bit counts, the highest and the lowest one, the bit reversal, the rotations, the unbounded and
//! exact shifts, the bit at an index, the casts between the signs and the bitwise `const fn`s; and
//! the operators `!`, `&`, `|`, `^`, `<<` and `>>` with their compound assignments.
//!
//! The tables pass their rows to the callbacks of `tables`, as those of `arithmetic` do. Each
//! storage computes the bits of a value at the width `Bits`: those of `Uint` above the width are
//! zero, and those of `Int` copy the sign, so that a result has the bits that the native integer of
//! `Bits` bits would have. The overflowing, checked, wrapping, strict and unchecked shifts are rows
//! of the families, in `arithmetic`.
//!
//! The operators are implemented once for every storage, as those of `arithmetic` are. The bitwise
//! ones cannot overflow. A shift takes an amount of any primitive integer type; one below `Bits`
//! shifts by itself, and any other, negative or `Bits` or more, overflows: the operator repeats
//! the overflow of a native shift, so that `overflow-checks` decides whether it panics, and
//! otherwise shifts by the amount modulo `Bits`, as a mathematical integer.

use core::ops::{
  BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Shl, ShlAssign, Shr,
  ShrAssign,
};

use super::{Int, Uint, WrappingOps, arithmetic::overflow};
use crate::{
  layout::Token,
  width::{FitsBits, InlineIntRepr, InlineUintRepr, IntRepr, IntReprMut, UintRepr, UintReprMut},
};

/// Passes the bit methods of `Uint` and `Int` to `$callback`, after the tokens of `$args`.
macro_rules! integer_bits_table {
  ($callback:ident! $args:tt) => {
    $callback! { $args
      /// The number of ones in the binary representation of `self`, its `Bits` bits: for `Int`,
      /// the bits of its two's complement, without the copies of the sign that the storage holds
      /// above them.
      fn count_ones(&self) -> u32 = prim count_ones_bits(self; Self::BITS);

      /// The number of zeros in the `Bits` bits of `self`.
      fn count_zeros(&self) -> u32 {
        Self::BITS - self.count_ones()
      }

      /// The number of leading zeros in the `Bits` bits of `self`, from bit `Bits - 1` down: `Bits`
      /// for zero, and none for a negative `Int`.
      fn leading_zeros(&self) -> u32 = prim leading_zeros_bits(self; Self::BITS);

      /// The number of trailing zeros of `self`, from bit 0 up: `Bits` for zero.
      fn trailing_zeros(&self) -> u32 = prim trailing_zeros_bits(self; Self::BITS);

      /// The number of leading ones in the `Bits` bits of `self`, from bit `Bits - 1` down: none
      /// for an `Int` at least zero, and `Bits` for `MAX` of `Uint` and for -1.
      fn leading_ones(&self) -> u32 = prim leading_ones_bits(self; Self::BITS);

      /// The number of trailing ones of `self`, from bit 0 up: `Bits` for `MAX` of `Uint` and for
      /// -1.
      fn trailing_ones(&self) -> u32 = prim trailing_ones_bits(self; Self::BITS);

      /// The index of the highest one in the `Bits` bits of `self`, or `None` if `self` is zero:
      /// [`checked_ilog2`](crate::IntegerMath::checked_ilog2) for a value at least zero, and the
      /// sign bit, `Bits - 1`, for a negative `Int`.
      fn highest_one(&self) -> Option<u32> = prim highest_one_bits(self; Self::BITS);

      /// The index of the lowest one of `self`, or `None` if `self` is zero.
      fn lowest_one(&self) -> Option<u32> = prim lowest_one(self);

      /// `self` with only its highest one, or zero for zero: `MIN` for a negative `Int`.
      fn isolate_highest_one(&self) -> Self = prim value isolate_highest_one_bits(self; Self::BITS);

      /// `self` with only its lowest one, or zero for zero: `MIN` for `MIN` of `Int`.
      fn isolate_lowest_one(&self) -> Self = prim value isolate_lowest_one_bits(self; Self::BITS);

      /// The `Bits` bits of `self` in reverse order: bit `i` moves to bit `Bits - 1 - i`, at every
      /// width, also one that is not a whole number of bytes. For `Int`, bit 0 becomes the sign.
      fn reverse_bits(&self) -> Self = prim value reverse_bits_bits(self; Self::BITS);

      /// `self` rotated left by `n` places within its `Bits` bits: the bits shifted out at the top
      /// come back at the bottom. The rotation is by `n` modulo `Bits`, so by `Bits` it returns
      /// `self`; zero bits hold only zero, which every rotation returns. For `Int`, the rotation
      /// moves the sign bit like any other, so it can change the sign.
      fn rotate_left(&self, n: u32) -> Self = prim value rotate_left_bits(self; n, Self::BITS);

      /// `self` rotated right by `n` places within its `Bits` bits, as
      /// [`rotate_left`](Self::rotate_left) rotates left.
      fn rotate_right(&self, n: u32) -> Self = prim value rotate_right_bits(self; n, Self::BITS);

      /// Unbounded shift left: `self << rhs` within `Bits` bits, or zero if `rhs` is `Bits` or
      /// more, which shifts every bit out.
      fn unbounded_shl(&self, rhs: u32) -> Self
        = prim value unbounded_shl_bits(self; rhs, Self::BITS);

      /// Unbounded shift right: `self >> rhs`, arithmetic for `Int`. An amount of `Bits` or more
      /// shifts every bit out, which leaves zero, or -1 for a negative `Int`.
      fn unbounded_shr(&self, rhs: u32) -> Self
        = prim value unbounded_shr_bits(self; rhs, Self::BITS);

      /// Exact shift left: `self << rhs`, or `None` if `rhs` is `Bits` or more or the shift loses
      /// a bit: for `Uint` if it shifts out a one, and for `Int` if a bit that it shifts out, or
      /// the new sign bit, differs from the sign, so that `>> rhs` would not give `self` back. The
      /// native method is unstable ([#144336](https://github.com/rust-lang/rust/issues/144336)),
      /// and this one follows it.
      fn shl_exact(&self, rhs: u32) -> Option<Self>
        = prim shl_exact_bits(self; rhs, Self::BITS) as shifted {
          match shifted {
            Some(shifted) => Some(Self::wrap(shifted)),
            None => None,
          }
        }

      /// Exact shift right: `self >> rhs`, arithmetic for `Int`, or `None` if `rhs` is `Bits` or
      /// more or the shift loses a one, so that `<< rhs` would not give `self` back. The native
      /// method is unstable ([#144336](https://github.com/rust-lang/rust/issues/144336)), and this
      /// one follows it.
      fn shr_exact(&self, rhs: u32) -> Option<Self>
        = prim shr_exact_bits(self; rhs, Self::BITS) as shifted {
          match shifted {
            Some(shifted) => Some(Self::wrap(shifted)),
            None => None,
          }
        }

      /// Returns the bit at `index` of `self`, one of its `Bits` bits: `false` for `index` of
      /// `Bits` or more, also for a negative `Int`, whose storage holds copies of the sign above
      /// the width.
      fn bit(&self, index: u32) -> bool = prim bit(self; index, Self::BITS);

      /// Sets the bit at `index` of `self` if `value` is `true`, and clears it otherwise. For
      /// `Int`, the bit at `Bits - 1` is the sign, so setting or clearing it changes the sign.
      ///
      /// # Panics
      ///
      /// Panics if `index` is `Bits` or more, as every index is at zero bits: a bit outside the
      /// width cannot be written.
      #[track_caller]
      fn set_bit(&mut self, index: u32, value: bool)
        = prim with_bit(self; index, value, Self::BITS) as changed {
          match changed {
            Some(changed) => self.repr = changed,
            None => $crate::integer::arithmetic::domain::bit_index(),
          }
        }
    }
  };
}

/// Passes the bit methods of `Uint` alone to `$callback`, after the tokens of `$args`: the bit
/// width and the cast to `Int`.
macro_rules! uint_bits_table {
  ($callback:ident! $args:tt) => {
    $callback! { $args
      /// The number of bits that `self` needs, `Bits` less its leading zeros: zero for zero.
      fn bit_width(&self) -> u32 {
        Self::BITS - self.leading_zeros()
      }

      /// The bits of `self` read as the `Int` of the same width in a storage of the same size, its
      /// two's complement, as `as` casts a native integer: `MAX` becomes -1.
      fn cast_signed(self) -> Self::Signed = prim signed cast_signed_bits(self; Self::BITS);
    }
  };
}

/// Passes the bit methods of `Int` alone to `$callback`, after the tokens of `$args`: the cast to
/// `Uint`.
macro_rules! int_bits_table {
  ($callback:ident! $args:tt) => {
    $callback! { $args
      /// The `Bits` bits of `self`, its two's complement, as the `Uint` of the same width in a
      /// storage of the same size, as `as` casts a native integer: -1 becomes `MAX`.
      fn cast_unsigned(self) -> Self::Magnitude
        = prim unsigned cast_unsigned_bits(self; Self::BITS);
    }
  };
}

/// Passes the bitwise `const fn`s of `Uint` and `Int` to `$callback`, after the tokens of `$args`,
/// for the inherent methods alone: generic code uses the operators, whose traits are the home of
/// these methods.
// The methods take the operands by value, as the methods of the operator traits do, rather than
// by reference: method syntax on a value then finds them before the methods of the traits, also
// where the traits are in scope, so that they serve constant contexts there too.
macro_rules! bitwise_table {
  ($callback:ident! $args:tt) => {
    $callback! { $args
      /// Bitwise and, `self & rhs`, as a `const fn` for constant contexts; other code uses the
      /// operator `&`. It takes the operands by value, as the method of `BitAnd` does, so that
      /// `a.bitand(b)` calls this method also where `core::ops::BitAnd` is in scope.
      fn bitand(self, rhs: Self) -> Self = prim value and(self, rhs);

      /// Bitwise or, `self | rhs`, as a `const fn` for constant contexts; other code uses the
      /// operator `|`. It takes the operands by value, as [`bitand`](Self::bitand) does.
      fn bitor(self, rhs: Self) -> Self = prim value or(self, rhs);

      /// Bitwise exclusive or, `self ^ rhs`, as a `const fn` for constant contexts; other code
      /// uses the operator `^`. It takes the operands by value, as [`bitand`](Self::bitand) does.
      fn bitxor(self, rhs: Self) -> Self = prim value xor(self, rhs);

      /// Bitwise complement, `!self`, as a `const fn` for constant contexts; other code uses the
      /// operator `!`. For `Uint`, every bit below `Bits` flips and the bits above stay zero; for
      /// `Int`, every bit flips, the copies of the sign included, so that `!x` is `-x - 1`. Zero
      /// bits hold only zero, its own complement. It takes the value, as `Not::not` does.
      fn not(self) -> Self = prim value not_bits(self; Self::BITS);
    }
  };
}

/// Implements the bitwise operator `$op`, `&`, `|` or `^`, in its four forms of values and
/// references, and its compound assignment `$assign` for a value and a reference. The form on two
/// references combines the storages with the sealed storage method `$primitive`, for every
/// storage, `$repr`; the forms with an owned operand, for a storage that holds the value inline,
/// `$inline`, pass references to it; and the compound assignments, for a storage that holds the
/// value in place, `$repr_mut`, store the result. Nothing overflows, so nothing panics, and as for
/// the native integers the methods are not `#[track_caller]`.
macro_rules! bit_operator {
  (
    $type:ident: $repr:ident, $inline:ident, $repr_mut:ident;
    $op:ident $method:ident, $assign:ident $assign_method:ident, $primitive:ident
  ) => {
    impl<B: FitsBits<R>, R: $inline> $op for $type<B, R> {
      type Output = Self;

      #[inline]
      fn $method(self, rhs: Self) -> Self {
        $op::$method(&self, &rhs)
      }
    }

    impl<B: FitsBits<R>, R: $inline> $op<&$type<B, R>> for $type<B, R> {
      type Output = Self;

      #[inline]
      fn $method(self, rhs: &Self) -> Self {
        $op::$method(&self, rhs)
      }
    }

    impl<B: FitsBits<R>, R: $inline> $op<$type<B, R>> for &$type<B, R> {
      type Output = $type<B, R>;

      #[inline]
      fn $method(self, rhs: $type<B, R>) -> $type<B, R> {
        $op::$method(self, &rhs)
      }
    }

    impl<B: FitsBits<R>, R: $repr> $op<&$type<B, R>> for &$type<B, R> {
      type Output = $type<B, R>;

      #[inline]
      fn $method(self, rhs: &$type<B, R>) -> $type<B, R> {
        $type::wrap(self.repr.$primitive(&rhs.repr, Token::new()))
      }
    }

    impl<B: FitsBits<R>, R: $repr_mut> $assign for $type<B, R> {
      #[inline]
      fn $assign_method(&mut self, rhs: Self) {
        $assign::$assign_method(self, &rhs);
      }
    }

    impl<B: FitsBits<R>, R: $repr_mut> $assign<&$type<B, R>> for $type<B, R> {
      #[inline]
      fn $assign_method(&mut self, rhs: &Self) {
        self.repr = self.repr.$primitive(&rhs.repr, Token::new());
      }
    }
  };
}

bit_operator!(
  Uint: UintRepr, InlineUintRepr, UintReprMut;
  BitAnd bitand, BitAndAssign bitand_assign, and
);
bit_operator!(
  Uint: UintRepr, InlineUintRepr, UintReprMut;
  BitOr bitor, BitOrAssign bitor_assign, or
);
bit_operator!(
  Uint: UintRepr, InlineUintRepr, UintReprMut;
  BitXor bitxor, BitXorAssign bitxor_assign, xor
);
bit_operator!(
  Int: IntRepr, InlineIntRepr, IntReprMut;
  BitAnd bitand, BitAndAssign bitand_assign, and
);
bit_operator!(
  Int: IntRepr, InlineIntRepr, IntReprMut;
  BitOr bitor, BitOrAssign bitor_assign, or
);
bit_operator!(
  Int: IntRepr, InlineIntRepr, IntReprMut;
  BitXor bitxor, BitXorAssign bitxor_assign, xor
);

/// Implements `!` for a reference, for every storage, `$repr`, with the sealed storage method
/// `not_bits`, and for a value, for a storage that holds it inline, `$inline`, which passes a
/// reference.
macro_rules! not_operator {
  ($type:ident: $repr:ident, $inline:ident) => {
    impl<B: FitsBits<R>, R: $inline> Not for $type<B, R> {
      type Output = Self;

      #[inline]
      fn not(self) -> Self {
        Not::not(&self)
      }
    }

    impl<B: FitsBits<R>, R: $repr> Not for &$type<B, R> {
      type Output = $type<B, R>;

      #[inline]
      fn not(self) -> $type<B, R> {
        $type::wrap(self.repr.not_bits($type::<B, R>::BITS, Token::new()))
      }
    }
  };
}

not_operator!(Uint: UintRepr, InlineUintRepr);
not_operator!(Int: IntRepr, InlineIntRepr);

/// The amount `rhs` of a shift of `bits` bits, an unsigned primitive integer converted to `u128`,
/// modulo `bits`, and whether it is `bits` or more. Zero bits take no remainder: every amount
/// overflows them, and shifts their only value, zero, by nothing.
#[inline]
const fn unsigned_amount(rhs: u128, bits: u32) -> (u32, bool) {
  if rhs < bits as u128 {
    (rhs as u32, false)
  } else if bits == 0 {
    (0, true)
  } else {
    ((rhs % bits as u128) as u32, true)
  }
}

/// The amount `rhs` of a shift of `bits` bits, a signed primitive integer converted to `i128`,
/// modulo `bits`, the remainder of the Euclidean division, which is never negative, and whether it
/// is negative or `bits` or more, as [`unsigned_amount`] gives them.
#[inline]
const fn signed_amount(rhs: i128, bits: u32) -> (u32, bool) {
  if 0 <= rhs && rhs < bits as i128 {
    (rhs as u32, false)
  } else if bits == 0 {
    (0, true)
  } else {
    (rhs.rem_euclid(bits as i128) as u32, true)
  }
}

/// Implements the shift operator `$op`, `<<` or `>>`, with each primitive integer of `$amount` as
/// the amount, by value and by reference, on values and references, and its compound assignment
/// `$assign` with an amount and a reference to one. The form on a reference computes, for every
/// storage, `$repr`: `$reduce` takes the amount, converted to `$wide`, modulo `Bits`, and an
/// amount that is negative or `Bits` or more repeats the overflow of a native shift with
/// `$forward`, which panics or not as `overflow-checks` decides, before the wrapping shift
/// `$wrapping` by the remainder. The forms on a value, for a storage that holds it inline,
/// `$inline`, pass a reference, and the compound assignments, for a storage that holds the value in
/// place, `$repr_mut`, store the result after the shift, so that a panic leaves their target
/// unchanged. Each form reports its caller, as the native operators do.
macro_rules! shift_operator {
  (
    $type:ident: $repr:ident, $inline:ident, $repr_mut:ident;
    $op:ident $method:ident, $assign:ident $assign_method:ident, $wrapping:ident, $forward:ident;
    $reduce:ident as $wide:ty: $($amount:ty),+ $(,)?
  ) => {$(
    impl<B: FitsBits<R>, R: $repr> $op<$amount> for &$type<B, R> {
      type Output = $type<B, R>;

      #[inline]
      #[track_caller]
      fn $method(self, rhs: $amount) -> $type<B, R> {
        let (amount, overflow) = $reduce(rhs as $wide, $type::<B, R>::BITS);
        if overflow {
          overflow::$forward();
        }
        WrappingOps::$wrapping(self, amount)
      }
    }

    impl<B: FitsBits<R>, R: $repr> $op<&$amount> for &$type<B, R> {
      type Output = $type<B, R>;

      #[inline]
      #[track_caller]
      fn $method(self, rhs: &$amount) -> $type<B, R> {
        $op::$method(self, *rhs)
      }
    }

    impl<B: FitsBits<R>, R: $inline> $op<$amount> for $type<B, R> {
      type Output = Self;

      #[inline]
      #[track_caller]
      fn $method(self, rhs: $amount) -> Self {
        $op::$method(&self, rhs)
      }
    }

    impl<B: FitsBits<R>, R: $inline> $op<&$amount> for $type<B, R> {
      type Output = Self;

      #[inline]
      #[track_caller]
      fn $method(self, rhs: &$amount) -> Self {
        $op::$method(&self, *rhs)
      }
    }

    impl<B: FitsBits<R>, R: $repr_mut> $assign<$amount> for $type<B, R> {
      #[inline]
      #[track_caller]
      fn $assign_method(&mut self, rhs: $amount) {
        let shifted = $op::$method(&*self, rhs);
        *self = shifted;
      }
    }

    impl<B: FitsBits<R>, R: $repr_mut> $assign<&$amount> for $type<B, R> {
      #[inline]
      #[track_caller]
      fn $assign_method(&mut self, rhs: &$amount) {
        $assign::$assign_method(self, *rhs);
      }
    }
  )+};
}

/// Implements `<<` and `>>` and their compound assignments for `$type` over the storages of
/// `$repr`, `$inline` and `$repr_mut`, with every primitive integer as the amount.
macro_rules! shift_operators {
  ($type:ident: $repr:ident, $inline:ident, $repr_mut:ident) => {
    shift_operator!(
      $type: $repr, $inline, $repr_mut;
      Shl shl, ShlAssign shl_assign, wrapping_shl, forward_shl;
      unsigned_amount as u128: u8, u16, u32, u64, u128, usize
    );
    shift_operator!(
      $type: $repr, $inline, $repr_mut;
      Shl shl, ShlAssign shl_assign, wrapping_shl, forward_shl;
      signed_amount as i128: i8, i16, i32, i64, i128, isize
    );
    shift_operator!(
      $type: $repr, $inline, $repr_mut;
      Shr shr, ShrAssign shr_assign, wrapping_shr, forward_shr;
      unsigned_amount as u128: u8, u16, u32, u64, u128, usize
    );
    shift_operator!(
      $type: $repr, $inline, $repr_mut;
      Shr shr, ShrAssign shr_assign, wrapping_shr, forward_shr;
      signed_amount as i128: i8, i16, i32, i64, i128, isize
    );
  };
}

shift_operators!(Uint: UintRepr, InlineUintRepr, UintReprMut);
shift_operators!(Int: IntRepr, InlineIntRepr, IntReprMut);
