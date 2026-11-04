//! The bit operations of [`Uint`](crate::Uint) and [`Int`](crate::Int): the method tables of the
//! bit counts, the highest and the lowest one, the bit reversal, the rotations, the unbounded and
//! exact shifts, the bit at an index, and the casts between the signs.
//!
//! The tables pass their rows to the callbacks of `tables`, as those of `arithmetic` do. Each
//! storage computes the bits of a value at the width `Bits`: those of `Uint` above the width are
//! zero, and those of `Int` copy the sign, so that a result has the bits that the native integer of
//! `Bits` bits would have. The overflowing, checked, wrapping, strict and unchecked shifts are rows
//! of the families, in `arithmetic`.

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
