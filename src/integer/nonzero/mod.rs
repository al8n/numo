//! The non-zero integers [`NonZeroUint`](crate::NonZeroUint) and [`NonZeroInt`](crate::NonZeroInt):
//! their constants, the constructor and the value of each storage, and the standard traits, which
//! compare, hash and format a non-zero integer as the integer of the same value.
//!
//! A non-zero integer holds a [`Uint`] or an [`Int`] over its storage, the `NonZero` of a native
//! integer or an array of limbs, and lends the value to the integer: an array as itself, and a
//! native `NonZero` as a copy of its integer.

use core::{
  cmp::Ordering,
  fmt,
  hash::{Hash, Hasher},
};

use typenum::NonZero;

use super::{Int, NonZeroInt, NonZeroUint, Uint};
use crate::{
  int,
  layout::Token,
  width::{
    FitsBits, InlineIntConsts, InlineIntRepr, InlineUintConsts, InlineUintRepr, IntCore, IntWidth,
    NonZeroIntRepr, NonZeroUintRepr, UintCore, UintWidth,
  },
};

// The constants are generic over the storage, as those of `Uint` and `Int` are. `BITS` needs only
// the width relation; the limits need a width that holds a value that is not zero, and an integer
// storage that holds the value inline, whose sealed trait carries them for each width.
impl<B: FitsBits<R::Integer>, R: NonZeroUintRepr> NonZeroUint<B, R> {
  /// The number of bits, `Bits`.
  pub const BITS: u32 = B::U32;
}

impl<B: FitsBits<R::Integer> + NonZero, R: NonZeroUintRepr> NonZeroUint<B, R>
where
  R::Integer: InlineUintRepr,
{
  /// The smallest value, one. Zero bits hold no value but zero, so `NonZeroUint` of zero bits has
  /// no `MIN`.
  pub const MIN: Self = Self {
    value: Uint::wrap(
      <<R::Integer as InlineUintConsts>::Width<B> as UintWidth<R::Integer>>::NONZERO_MIN,
    ),
  };

  /// The largest value, `2^Bits - 1`. Zero bits have no `MAX`.
  pub const MAX: Self = Self {
    value: Uint::wrap(
      <<R::Integer as InlineUintConsts>::Width<B> as UintWidth<R::Integer>>::NONZERO_MAX,
    ),
  };
}

impl<B: FitsBits<R::Integer>, R: NonZeroIntRepr> NonZeroInt<B, R> {
  /// The number of bits, `Bits`, including the sign bit.
  pub const BITS: u32 = B::U32;
}

impl<B: FitsBits<R::Integer> + NonZero, R: NonZeroIntRepr> NonZeroInt<B, R>
where
  R::Integer: InlineIntRepr,
{
  /// The smallest value, `-2^(Bits - 1)`. Zero bits hold no value but zero, so `NonZeroInt` of
  /// zero bits has no `MIN`.
  pub const MIN: Self = Self {
    value: Int::wrap(
      <<R::Integer as InlineIntConsts>::Width<B> as IntWidth<R::Integer>>::NONZERO_MIN,
    ),
  };

  /// The largest value: `2^(Bits - 1) - 1` from two bits, and at one bit -1, which is also `MIN`,
  /// since one bit holds no other value that is not zero. Zero bits have no `MAX`.
  pub const MAX: Self = Self {
    value: Int::wrap(
      <<R::Integer as InlineIntConsts>::Width<B> as IntWidth<R::Integer>>::NONZERO_MAX,
    ),
  };
}

/// Generates the constructor and the value of [`NonZeroUint`] and [`NonZeroInt`] over the arrays of
/// each limb type, written `limb, lower, upper;`, where `lower` and `upper` are the modules of its
/// limb layers.
macro_rules! array_nonzero {
  ($($limb:ident, $lower:ident, $upper:ident);+ $(;)?) => {$(
    impl<B, const N: usize> NonZeroUint<B, [$limb; N]> {
      /// Creates a non-zero integer of the value `value`, or returns `None` if it is zero, as
      /// every value of zero bits is.
      #[must_use]
      #[inline]
      pub const fn new(value: Uint<B, [$limb; N]>) -> Option<Self> {
        if int::unsigned::$lower::is_zero(&value.repr) {
          None
        } else {
          Some(Self { value })
        }
      }

      /// Returns the value, as the integer of the same width and storage that may be zero.
      #[must_use]
      #[inline]
      pub const fn get(self) -> Uint<B, [$limb; N]> {
        self.value
      }
    }

    impl<B, const N: usize> NonZeroInt<B, [$limb; N]> {
      /// Creates a non-zero integer of the value `value`, or returns `None` if it is zero, as
      /// every value of zero bits is.
      #[must_use]
      #[inline]
      pub const fn new(value: Int<B, [$limb; N]>) -> Option<Self> {
        if int::signed::$upper::is_zero(&value.repr) {
          None
        } else {
          Some(Self { value })
        }
      }

      /// Returns the value, as the integer of the same width and storage that may be zero.
      #[must_use]
      #[inline]
      pub const fn get(self) -> Int<B, [$limb; N]> {
        self.value
      }
    }
  )+};
}

array_nonzero!(
  u64, b64, b64;
  u8, b8, b8;
  u16, b16, b16;
  u32, b32, b32;
  u128, b128, b128;
);

/// Generates the constructor and the value of [`NonZeroUint`] and [`NonZeroInt`] over the `NonZero`
/// of each native integer, written `unsigned, signed;`.
macro_rules! native_nonzero {
  ($($uint:ident, $int:ident);+ $(;)?) => {$(
    impl<B> NonZeroUint<B, core::num::NonZero<$uint>> {
      /// Creates a non-zero integer of the value `value`, or returns `None` if it is zero, as
      /// every value of zero bits is.
      #[must_use]
      #[inline]
      pub const fn new(value: Uint<B, $uint>) -> Option<Self> {
        match core::num::NonZero::new(value.repr) {
          Some(repr) => Some(Self { value: Uint::wrap(repr) }),
          None => None,
        }
      }

      /// Returns the value, as the integer of the same width that may be zero.
      #[must_use]
      #[inline]
      pub const fn get(self) -> Uint<B, $uint> {
        Uint::wrap(self.value.repr.get())
      }
    }

    impl<B> NonZeroInt<B, core::num::NonZero<$int>> {
      /// Creates a non-zero integer of the value `value`, or returns `None` if it is zero, as
      /// every value of zero bits is.
      #[must_use]
      #[inline]
      pub const fn new(value: Int<B, $int>) -> Option<Self> {
        match core::num::NonZero::new(value.repr) {
          Some(repr) => Some(Self { value: Int::wrap(repr) }),
          None => None,
        }
      }

      /// Returns the value, as the integer of the same width that may be zero.
      #[must_use]
      #[inline]
      pub const fn get(self) -> Int<B, $int> {
        Int::wrap(self.value.repr.get())
      }
    }
  )+};
}

native_nonzero!(
  u8, i8;
  u16, i16;
  u32, i32;
  u64, i64;
  u128, i128;
);

/// Implements the standard traits of the non-zero integer `$name`, over the storages of `$repr`,
/// whose integer `$integer` stores the value in the storages of `$core`. The value compares,
/// hashes and formats as the integer of the same value, which the storage lends.
// Trait methods cannot be `const`.
macro_rules! nonzero_traits {
  ($name:ident, $integer:ident, $repr:ident, $core:ident) => {
    impl<B, R> Clone for $name<B, R>
    where
      R: Clone,
    {
      #[inline]
      fn clone(&self) -> Self {
        Self {
          value: self.value.clone(),
        }
      }
    }

    impl<B, R> Copy for $name<B, R> where R: Copy {}

    impl<B, R> PartialEq for $name<B, R>
    where
      R: PartialEq,
    {
      #[inline]
      fn eq(&self, other: &Self) -> bool {
        self.value == other.value
      }
    }

    impl<B, R> Eq for $name<B, R> where R: Eq {}

    impl<B, R> PartialOrd for $name<B, R>
    where
      R: $repr,
    {
      #[inline]
      fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
      }
    }

    impl<B, R> Ord for $name<B, R>
    where
      R: $repr,
    {
      /// Compares the values, as the integers of the same values compare.
      #[inline]
      fn cmp(&self, other: &Self) -> Ordering {
        <R::Integer as $core>::with_nonzero(
          &self.value.repr,
          |lhs| {
            <R::Integer as $core>::with_nonzero(
              &other.value.repr,
              |rhs| lhs.compare(rhs, Token::new()),
              Token::new(),
            )
          },
          Token::new(),
        )
      }
    }

    impl<B, R> Hash for $name<B, R>
    where
      R: $repr,
      B: FitsBits<R::Integer>,
    {
      /// Writes what the integer of the same value writes, so that the hash depends only on the
      /// value and `Bits`, whatever the storage, as the hash of a native `NonZero` is that of its
      /// integer.
      #[inline]
      fn hash<H: Hasher>(&self, state: &mut H) {
        <R::Integer as $core>::with_nonzero(
          &self.value.repr,
          |integer| integer.hash_limbs(B::U32, state, Token::new()),
          Token::new(),
        );
      }
    }

    impl<B, R> fmt::Display for $name<B, R>
    where
      R: $repr,
    {
      /// Writes the value in decimal, as the integer of the same value does.
      #[inline]
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <R::Integer as $core>::with_nonzero(
          &self.value.repr,
          |integer| integer.fmt_decimal(f, Token::new()),
          Token::new(),
        )
      }
    }

    impl<B, R> fmt::Debug for $name<B, R>
    where
      R: $repr,
    {
      /// Writes the same text as [`Display`](fmt::Display): decimal for every storage, as the
      /// integers write it.
      #[inline]
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
      }
    }

    impl<B, R> From<$name<B, R>> for $integer<B, R::Integer>
    where
      R: $repr,
    {
      /// Returns the value, as the integer of the same width and storage that may be zero.
      #[inline]
      fn from(value: $name<B, R>) -> Self {
        $integer::wrap(<R::Integer as $core>::from_nonzero(
          value.value.repr,
          Token::new(),
        ))
      }
    }
  };
}

nonzero_traits!(NonZeroUint, Uint, NonZeroUintRepr, UintCore);
nonzero_traits!(NonZeroInt, Int, NonZeroIntRepr, IntCore);
