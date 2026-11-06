//! The non-zero integers [`NonZeroUint`](crate::NonZeroUint) and [`NonZeroInt`](crate::NonZeroInt):
//! their constants, the constructors, the value and the method tables of each storage, the helpers
//! of the generic implementations of their traits, the standard traits, which compare, hash and
//! format a non-zero integer as the integer of the same value, the text, which a non-zero integer
//! parses as its integer does, with the error of the kind `Zero` for zero, and writes as the
//! integer of the same value, and the operators: the division and the remainder of a `Uint` by a
//! `NonZeroUint`, `|` and `|=`, and the negation of a `NonZeroInt`.
//!
//! A non-zero integer holds a [`Uint`] or an [`Int`] over its storage, the `NonZero` of a native
//! integer or an array of limbs, and lends the value to the integer: an array as itself, and a
//! native `NonZero` as a copy of its integer. A row computes the integer's row on the value and
//! wraps the result, which is not zero for an operand that is not: an array checks it with a debug
//! assertion, and a native `NonZero` with its zero test, which panics on a bug instead of making an
//! invalid value. Only `new_unchecked` of a native storage has unsafe code: it leaves the zero test
//! to its caller, as the native `NonZero::new_unchecked` does.
//!
//! The tables pass their rows to the callbacks of `tables`, as those of `arithmetic` and `bits` do,
//! and every row is derived. A row calls `self.integer()`, the rows of the integer, `Self::known`,
//! `Self::saturated`, `Self::duplicate` and other rows, which an inherent block of each storage
//! defines as `const fn`s and the generic implementations of the traits reach through [`Parts`] and
//! [`Saturated`]. The integer of a row and the non-zero integer of the other sign are named through
//! `nonzero_integer!` and `nonzero_other!`, from the `$args` of an inherent block, which holds the
//! two concrete types, or from the associated types of a trait, which passes `[]`.

use core::{
  cmp::Ordering,
  fmt,
  hash::{Hash, Hasher},
  num::ParseIntError,
  ops::{BitOr, BitOrAssign, Div, DivAssign, Neg, Rem, RemAssign},
  str::FromStr,
};

use typenum::NonZero;

use super::{Int, NonZeroInt, NonZeroUint, SignedNonZero, Uint, arithmetic::overflow};
use crate::{
  int::{self, parse_errors},
  layout::Token,
  width::{
    FitsBits, InlineIntConsts, InlineIntRepr, InlineUintConsts, InlineUintRepr, IntCore, IntRepr,
    IntReprMut, IntWidth, NonZeroIntRepr, NonZeroUintRepr, UintCore, UintRepr, UintReprMut,
    UintWidth, WholeBytes,
  },
};

/// Panics on a zero where the arithmetic gives a value that is not zero: a bug of this crate, as
/// no row returns zero for an operand that is not.
#[cold]
pub(crate) const fn zero() -> ! {
  panic!("a non-zero result is zero")
}

/// Panics on a zero passed to `new_unchecked` through the traits, which checks it in every build,
/// reporting the caller.
#[cold]
#[track_caller]
pub(crate) const fn zero_value() -> ! {
  panic!("`new_unchecked` requires a value that is not zero")
}

/// The integer type of a row of the non-zero tables, named by `$args`: in an inherent block of one
/// storage the concrete integer that the table passes, and in a trait `Self::Integer`.
macro_rules! nonzero_integer {
  ([]) => {
    Self::Integer
  };
  ([$integer:ty, $other:ty]) => {
    $integer
  };
}

/// The non-zero integer of the other sign of a row of the non-zero tables, named by `$args`: in an
/// inherent block of one storage the concrete type that the table passes, and in a trait the
/// associated type `$assoc`.
macro_rules! nonzero_other {
  ([], $assoc:ident) => {
    Self::$assoc
  };
  ([$integer:ty, $other:ty], $assoc:ident) => {
    $other
  };
}

/// Passes the rows of both signs of the non-zero integers to `$callback`, after the tokens of
/// `$args`: written `nonzero_table!(Uint, ..)` for `NonZeroUint` and `nonzero_table!(Int, ..)` for
/// `NonZeroInt`. Both have the same documentation and signatures, and differ in the bound that a
/// saturating row returns.
macro_rules! nonzero_table {
  (Uint, $callback:ident! $args:tt) => {
    nonzero_table!(@rows $callback! $args
      /// Saturating multiplication: `self * rhs`, or the bound of the range of the type that the
      /// product passes.
      fn saturating_mul(&self, rhs: &Self) -> Self {
        match self.checked_mul(rhs) {
          Some(product) => product,
          None => Self::saturated(true),
        }
      }

      /// Saturating exponentiation: `self^exp`, or the bound of the range of the type that the
      /// power passes.
      fn saturating_pow(&self, exp: u32) -> Self {
        match self.checked_pow(exp) {
          Some(power) => power,
          None => Self::saturated(true),
        }
      }
    );
  };
  (Int, $callback:ident! $args:tt) => {
    nonzero_table!(@rows $callback! $args
      /// Saturating multiplication: `self * rhs`, or the bound of the range of the type that the
      /// product passes.
      fn saturating_mul(&self, rhs: &Self) -> Self {
        match self.checked_mul(rhs) {
          Some(product) => product,
          // An overflowing product passes `MIN` exactly when the operands differ in sign.
          None => Self::saturated(self.is_negative() == rhs.is_negative()),
        }
      }

      /// Saturating exponentiation: `self^exp`, or the bound of the range of the type that the
      /// power passes.
      fn saturating_pow(&self, exp: u32) -> Self {
        match self.checked_pow(exp) {
          Some(power) => power,
          // The power passes `MIN` exactly when it is negative: an odd power of a negative value.
          // At one bit, `(-1)^0`, one, passes `MAX`, which is -1.
          None => Self::saturated(!(self.is_negative() && exp % 2 == 1)),
        }
      }
    );
  };
  // The saturating rows of the sign, written in its arm with their own `self`, after the rows
  // that both signs share.
  (@rows $callback:ident! $args:tt $($saturating:tt)*) => {
    $callback! { $args
      /// The number of leading zeros in the `Bits` bits of `self`, from bit `Bits - 1` down: none
      /// for a negative `NonZeroInt`.
      fn leading_zeros(&self) -> u32 {
        self.integer().leading_zeros()
      }

      /// The number of trailing zeros of `self`, from bit 0 up.
      fn trailing_zeros(&self) -> u32 {
        self.integer().trailing_zeros()
      }

      /// The number of ones in the `Bits` bits of `self`, which is not zero: for `NonZeroInt`, the
      /// bits of its two's complement, without the copies of the sign that the storage holds above
      /// them.
      fn count_ones(&self) -> core::num::NonZero<u32> {
        match core::num::NonZero::new(self.integer().count_ones()) {
          Some(ones) => ones,
          None => $crate::integer::nonzero::zero(),
        }
      }

      /// The index of the highest one in the `Bits` bits of `self`: the base-2 logarithm for a
      /// positive value, and the sign bit, `Bits - 1`, for a negative `NonZeroInt`.
      fn highest_one(&self) -> u32 {
        match self.integer().highest_one() {
          Some(index) => index,
          None => $crate::integer::nonzero::zero(),
        }
      }

      /// The index of the lowest one of `self`.
      fn lowest_one(&self) -> u32 {
        match self.integer().lowest_one() {
          Some(index) => index,
          None => $crate::integer::nonzero::zero(),
        }
      }

      /// `self` with only its highest one: `MIN` for a negative `NonZeroInt`.
      fn isolate_highest_one(&self) -> Self {
        Self::known(self.integer().isolate_highest_one())
      }

      /// `self` with only its lowest one: `MIN` for `MIN` of `NonZeroInt`.
      fn isolate_lowest_one(&self) -> Self {
        Self::known(self.integer().isolate_lowest_one())
      }

      /// The `Bits` bits of `self` in reverse order: bit `i` moves to bit `Bits - 1 - i`, at every
      /// width, also one that is not a whole number of bytes. For `NonZeroInt`, bit 0 becomes the
      /// sign. The native method is unstable
      /// ([#128281](https://github.com/rust-lang/rust/issues/128281)), and this one follows it.
      fn reverse_bits(&self) -> Self {
        Self::known(self.integer().reverse_bits())
      }

      /// `self` rotated left by `n` places within its `Bits` bits: the bits shifted out at the top
      /// come back at the bottom. The rotation is by `n` modulo `Bits`, so by `Bits` it returns
      /// `self`. For `NonZeroInt`, the rotation moves the sign bit like any other, so it can change
      /// the sign. The native method is unstable
      /// ([#128281](https://github.com/rust-lang/rust/issues/128281)), and this one follows it.
      fn rotate_left(&self, n: u32) -> Self {
        Self::known(self.integer().rotate_left(n))
      }

      /// `self` rotated right by `n` places within its `Bits` bits, as
      /// [`rotate_left`](Self::rotate_left) rotates left. The native method is unstable
      /// ([#128281](https://github.com/rust-lang/rust/issues/128281)), and this one follows it.
      fn rotate_right(&self, n: u32) -> Self {
        Self::known(self.integer().rotate_right(n))
      }

      /// Checked multiplication: `self * rhs`, or `None` if the product is outside the range of
      /// the type. A product of values that are not zero is not zero.
      fn checked_mul(&self, rhs: &Self) -> Option<Self> {
        match self.integer().checked_mul(&rhs.integer()) {
          Some(product) => Some(Self::known(product)),
          None => None,
        }
      }

      /// Checked exponentiation: `self^exp`, or `None` if the power is outside the range of the
      /// type, as `self^0`, one, is for `NonZeroInt` of one bit. A power of a value that is not
      /// zero is not zero.
      fn checked_pow(&self, exp: u32) -> Option<Self> {
        match self.integer().checked_pow(exp) {
          Some(power) => Some(Self::known(power)),
          None => None,
        }
      }

      $($saturating)*
    }
  };
}

/// Passes the rows of `NonZeroUint` alone to `$callback`, after the tokens of `$args`.
macro_rules! nonzero_uint_table {
  ($callback:ident! $args:tt) => {
    $callback! { $args
      /// Checked addition of an integer that may be zero: `self + rhs`, or `None` if the sum is
      /// above `MAX`. The sum of a value that is not zero and one that is not negative is not zero.
      fn checked_add(&self, rhs: &nonzero_integer!($args)) -> Option<Self> {
        match self.integer().checked_add(rhs) {
          Some(sum) => Some(Self::known(sum)),
          None => None,
        }
      }

      /// Saturating addition of an integer that may be zero: `self + rhs`, or `MAX` if the sum is
      /// above it.
      fn saturating_add(&self, rhs: &nonzero_integer!($args)) -> Self {
        match self.checked_add(rhs) {
          Some(sum) => sum,
          None => Self::saturated(true),
        }
      }

      /// Checked next power of two: the smallest power of two at least `self`, or `None` if it is
      /// above `MAX`.
      fn checked_next_power_of_two(&self) -> Option<Self> {
        match self.integer().checked_next_power_of_two() {
          Some(power) => Some(Self::known(power)),
          None => None,
        }
      }

      /// Base-2 logarithm: the logarithm of `self` to the base 2, rounded down, the index of its
      /// highest one. A value that is not zero has one, so this never panics.
      fn ilog2(&self) -> u32 {
        match self.integer().checked_ilog2() {
          Some(log) => log,
          None => $crate::integer::nonzero::zero(),
        }
      }

      /// Base-10 logarithm: the logarithm of `self` to the base 10, rounded down, one less than the
      /// number of its decimal digits. A value that is not zero has one, so this never panics.
      fn ilog10(&self) -> u32 {
        match self.integer().checked_ilog10() {
          Some(log) => log,
          None => $crate::integer::nonzero::zero(),
        }
      }

      /// Midpoint: `(self + rhs) / 2` rounded down, computed without overflow, which is not zero.
      fn midpoint(&self, rhs: &Self) -> Self {
        Self::known(self.integer().midpoint(&rhs.integer()))
      }

      /// Returns `true` if `self` is a power of two: if it has exactly one one.
      fn is_power_of_two(&self) -> bool {
        self.integer().is_power_of_two()
      }

      /// Square root: `self` to the power one half, rounded down, which is not zero.
      fn isqrt(&self) -> Self {
        Self::known(self.integer().isqrt())
      }

      /// Division rounded up: `self / rhs` rounded toward positive infinity, which is not zero. The
      /// divisor is not zero, so this never panics.
      fn div_ceil(&self, rhs: &Self) -> Self {
        Self::known(self.integer().div_ceil(&rhs.integer()))
      }

      /// The number of bits that `self` needs, `Bits` less its leading zeros, which is not zero.
      fn bit_width(&self) -> core::num::NonZero<u32> {
        match core::num::NonZero::new(self.integer().bit_width()) {
          Some(width) => width,
          None => $crate::integer::nonzero::zero(),
        }
      }

      /// The bits of `self` read as the `NonZeroInt` of the same width in a storage of the same
      /// size, its two's complement, as `as` casts a native integer: `MAX` becomes -1.
      fn cast_signed(self) -> nonzero_other!($args, Signed) {
        <nonzero_other!($args, Signed)>::known(self.integer().cast_signed())
      }
    }
  };
}

/// Passes the rows of `NonZeroInt` alone to `$callback`, after the tokens of `$args`.
macro_rules! nonzero_int_table {
  ($callback:ident! $args:tt) => {
    $callback! { $args
      /// Returns `true` if `self` is negative.
      // The sign bit is the leading bit of the `Bits` bits. `Int::is_negative` reaches generic code
      // only through `SignedFixedInteger`, which needs a storage that holds the value inline, and
      // a row compiles for every storage.
      fn is_negative(&self) -> bool {
        self.integer().leading_zeros() == 0
      }

      /// Returns `true` if `self` is positive, as every value that is not negative is.
      fn is_positive(&self) -> bool {
        !self.is_negative()
      }

      /// Overflowing negation: `-self` wrapped to the range of the type, and whether the exact
      /// negation is outside that range, as it is only for `MIN`, which negates to itself.
      fn overflowing_neg(&self) -> (Self, bool) {
        let (negated, overflow) = self.integer().overflowing_neg();
        (Self::known(negated), overflow)
      }

      /// Checked negation: `-self`, or `None` if `self` is `MIN`.
      fn checked_neg(&self) -> Option<Self> {
        match self.overflowing_neg() {
          (negated, false) => Some(negated),
          (_, true) => None,
        }
      }

      /// Wrapping negation: `-self` modulo `2^Bits`, within the range of the type; `MIN` negates
      /// to itself.
      fn wrapping_neg(&self) -> Self {
        self.overflowing_neg().0
      }

      /// Saturating negation: `-self`, or `MAX` if `self` is `MIN`. One bit holds only -1, which
      /// is its `MAX`.
      fn saturating_neg(&self) -> Self {
        match self.overflowing_neg() {
          (negated, false) => negated,
          (_, true) => Self::saturated(true),
        }
      }

      /// Overflowing absolute value: `|self|` wrapped to the range of the type, and whether it is
      /// outside that range, as it is for `MIN`, which wraps to itself.
      fn overflowing_abs(&self) -> (Self, bool) {
        if self.is_negative() {
          self.overflowing_neg()
        } else {
          (Self::duplicate(self), false)
        }
      }

      /// Checked absolute value: `|self|`, or `None` if `self` is `MIN`.
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

      /// Saturating absolute value: `|self|`, or `MAX` if `self` is `MIN`. One bit holds only -1,
      /// which is its `MAX`.
      fn saturating_abs(&self) -> Self {
        match self.overflowing_abs() {
          (abs, false) => abs,
          (_, true) => Self::saturated(true),
        }
      }

      /// The absolute value, `|self|`. The absolute value of `MIN` is outside the range of the
      /// type: the method then panics or returns `MIN`, as `overflow-checks` decides for the native
      /// integers (see [Overflow](crate::NonZeroInt#overflow)).
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

      /// The absolute value, `|self|`, as the `NonZeroUint` of the same width in a storage of the
      /// same size, which also holds the absolute value of `MIN`.
      fn unsigned_abs(&self) -> nonzero_other!($args, Unsigned) {
        <nonzero_other!($args, Unsigned)>::known(self.wrapping_abs().integer().cast_unsigned())
      }

      /// The `Bits` bits of `self`, its two's complement, as the `NonZeroUint` of the same width in
      /// a storage of the same size, as `as` casts a native integer: -1 becomes `MAX`.
      fn cast_unsigned(self) -> nonzero_other!($args, Unsigned) {
        <nonzero_other!($args, Unsigned)>::known(self.integer().cast_unsigned())
      }
    }
  };
}

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

/// Emits the inherent `from_str_radix` of a non-zero integer in an inherent block of one storage,
/// whose integer of the same width and storage is `$integer`.
macro_rules! nonzero_from_str_radix {
  ($integer:ty) => {
    /// Parses a non-zero integer from the digits of `radix` after an optional sign, `+`, or for
    /// `NonZeroInt` also `-`, as the native `NonZero::from_str_radix` does at the width `Bits`: as
    /// the integer of the same width and storage parses them, then the error of the kind `Zero` for
    /// zero, after the errors of the integer.
    ///
    /// # Errors
    ///
    /// Returns the [`ParseIntError`](core::num::ParseIntError) that `from_str_radix` of the integer
    /// returns, and for a value of zero, which every text that the integer of zero bits reads is,
    /// the error of the kind `Zero`.
    ///
    /// # Panics
    ///
    /// Panics if `radix` is not from 2 to 36, reporting the caller, so in a constant such a radix
    /// fails to compile.
    #[inline]
    #[track_caller]
    pub const fn from_str_radix(src: &str, radix: u32) -> Result<Self, core::num::ParseIntError> {
      match <$integer>::from_str_radix(src, radix) {
        Ok(value) => match Self::new(value) {
          Some(nonzero) => Ok(nonzero),
          None => Err($crate::int::parse_errors::ZERO),
        },
        Err(error) => Err(error),
      }
    }
  };
}

/// Emits the byte order of a non-zero integer in an inherent block of one storage, whose integer of
/// the same width and storage is `$integer`. Each method computes the method of the same name of
/// the integer on the value: a permutation of its bytes, which is not zero, as the bytes of a value
/// that is not zero are not all zeros.
macro_rules! nonzero_byte_order {
  ($integer:ty) => {
    /// The value with its bytes in reverse order, at a width of whole bytes, as `swap_bytes` of the
    /// integer of the same value gives it. Reversing them twice gives the value back.
    #[must_use]
    #[inline]
    pub const fn swap_bytes(&self) -> Self
    where
      B: WholeBytes,
    {
      Self::known(self.integer().swap_bytes())
    }

    /// Converts the value to big-endian order, as `to_be` of the integer of the same value does:
    /// `self` on a big-endian target and [`swap_bytes`](Self::swap_bytes) on a little-endian one.
    /// It converts the value, not a layout: an array keeps its limbs least significant first on
    /// every target.
    #[must_use]
    #[inline]
    pub const fn to_be(&self) -> Self
    where
      B: WholeBytes,
    {
      Self::known(self.integer().to_be())
    }

    /// Converts the value to little-endian order, as `to_le` of the integer of the same value
    /// does: `self` on a little-endian target and [`swap_bytes`](Self::swap_bytes) on a big-endian
    /// one, a conversion of the value, as for [`to_be`](Self::to_be).
    #[must_use]
    #[inline]
    pub const fn to_le(&self) -> Self
    where
      B: WholeBytes,
    {
      Self::known(self.integer().to_le())
    }

    /// Converts `x` from big-endian order, as `from_be` of the integer does: `x.to_be()`.
    #[must_use]
    #[inline]
    pub const fn from_be(x: Self) -> Self
    where
      B: WholeBytes,
    {
      Self::known(<$integer>::from_be(x.get()))
    }

    /// Converts `x` from little-endian order, as `from_le` of the integer does: `x.to_le()`.
    #[must_use]
    #[inline]
    pub const fn from_le(x: Self) -> Self
    where
      B: WholeBytes,
    {
      Self::known(<$integer>::from_le(x.get()))
    }
  };
}

/// Generates the constructor, the value and the rows of [`NonZeroUint`] and [`NonZeroInt`] over the
/// arrays of each limb type, written `limb, lower, upper;`, where `lower` and `upper` are the
/// modules of its limb layers. The array is the integer's storage, which the rows lend.
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

      /// Creates a non-zero integer of the value `value` without checking that it is not zero.
      ///
      /// # Safety
      ///
      /// `value` must not be zero, that is, [`new`](Self::new) would return `Some`. Otherwise the
      /// behavior is undefined. Zero bits hold only zero, so no call is sound there.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn new_unchecked(value: Uint<B, [$limb; N]>) -> Self {
        // An array has no niche: a zero breaks only the logical invariant, on which no unsafe code
        // relies, so the body has no unsafe code and a debug assertion checks the value. The
        // contract is that of the native storages all the same, which leaves room for a niche.
        debug_assert!(
          !int::unsigned::$lower::is_zero(&value.repr),
          "`new_unchecked` requires a value that is not zero"
        );
        Self { value }
      }

      /// Returns the value, as the integer of the same width and storage that may be zero.
      #[must_use]
      #[inline]
      pub const fn get(self) -> Uint<B, [$limb; N]> {
        self.value
      }

      /// The value, lent as the integer.
      #[inline]
      const fn integer(&self) -> &Uint<B, [$limb; N]> {
        &self.value
      }

      /// The non-zero integer of `value`, which is not zero, as every non-zero result of the rows
      /// is; a debug assertion checks it.
      #[inline]
      const fn known(value: Uint<B, [$limb; N]>) -> Self {
        debug_assert!(!int::unsigned::$lower::is_zero(&value.repr), "a non-zero result is zero");
        Self { value }
      }
    }

    impl<B, const N: usize> NonZeroUint<B, [$limb; N]>
    where
      B: FitsBits<[$limb; N]>,
    {
      /// `MAX`, the bound that a saturating row returns.
      #[inline]
      const fn saturated(_: bool) -> Self {
        Self::known(Uint::<B, [$limb; N]>::MAX)
      }

      nonzero_from_str_radix!(Uint<B, [$limb; N]>);
      nonzero_byte_order!(Uint<B, [$limb; N]>);

      nonzero_table!(Uint, inherent_items![Uint<B, [$limb; N]>, NonZeroInt<B, [$limb; N]>]);
      nonzero_uint_table!(inherent_items![Uint<B, [$limb; N]>, NonZeroInt<B, [$limb; N]>]);
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

      /// Creates a non-zero integer of the value `value` without checking that it is not zero.
      ///
      /// # Safety
      ///
      /// `value` must not be zero, that is, [`new`](Self::new) would return `Some`. Otherwise the
      /// behavior is undefined. Zero bits hold only zero, so no call is sound there.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn new_unchecked(value: Int<B, [$limb; N]>) -> Self {
        // An array has no niche: a zero breaks only the logical invariant, on which no unsafe code
        // relies, so the body has no unsafe code and a debug assertion checks the value. The
        // contract is that of the native storages all the same, which leaves room for a niche.
        debug_assert!(
          !int::signed::$upper::is_zero(&value.repr),
          "`new_unchecked` requires a value that is not zero"
        );
        Self { value }
      }

      /// Returns the value, as the integer of the same width and storage that may be zero.
      #[must_use]
      #[inline]
      pub const fn get(self) -> Int<B, [$limb; N]> {
        self.value
      }

      /// The value, lent as the integer.
      #[inline]
      const fn integer(&self) -> &Int<B, [$limb; N]> {
        &self.value
      }

      /// The non-zero integer of `value`, which is not zero, as every non-zero result of the rows
      /// is; a debug assertion checks it.
      #[inline]
      const fn known(value: Int<B, [$limb; N]>) -> Self {
        debug_assert!(!int::signed::$upper::is_zero(&value.repr), "a non-zero result is zero");
        Self { value }
      }

      /// A copy of `self`, for a row that returns its operand.
      #[inline]
      const fn duplicate(&self) -> Self {
        *self
      }
    }

    impl<B, const N: usize> NonZeroInt<B, [$limb; N]>
    where
      B: FitsBits<[$limb; N]>,
    {
      /// The bound that a saturating row returns: `MAX` if `max`, and `MIN` otherwise. One bit
      /// holds only -1, which is both.
      #[inline]
      const fn saturated(max: bool) -> Self {
        if max && Self::BITS > 1 {
          Self::known(Int::<B, [$limb; N]>::MAX)
        } else {
          Self::known(Int::<B, [$limb; N]>::MIN)
        }
      }

      nonzero_from_str_radix!(Int<B, [$limb; N]>);
      nonzero_byte_order!(Int<B, [$limb; N]>);

      nonzero_table!(Int, inherent_items![Int<B, [$limb; N]>, NonZeroUint<B, [$limb; N]>]);
      nonzero_int_table!(inherent_items![Int<B, [$limb; N]>, NonZeroUint<B, [$limb; N]>]);
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

/// Generates the constructor, the value and the rows of [`NonZeroUint`] and [`NonZeroInt`] over the
/// `NonZero` of each native integer, written `unsigned, signed;`. The rows compute on a copy of
/// the integer.
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

      /// Creates a non-zero integer of the value `value` without checking that it is not zero.
      ///
      /// # Safety
      ///
      /// `value` must not be zero, that is, [`new`](Self::new) would return `Some`. Otherwise the
      /// behavior is undefined. Zero bits hold only zero, so no call is sound there.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn new_unchecked(value: Uint<B, $uint>) -> Self {
        // SAFETY: the caller guarantees that `value` is not zero. Its bits are the low `Bits` bits
        // of `repr`, so `repr` is not zero either: the precondition of the native
        // `NonZero::new_unchecked`.
        let repr = unsafe { core::num::NonZero::new_unchecked(value.repr) };
        Self { value: Uint::wrap(repr) }
      }

      /// Returns the value, as the integer of the same width that may be zero.
      #[must_use]
      #[inline]
      pub const fn get(self) -> Uint<B, $uint> {
        Uint::wrap(self.value.repr.get())
      }

      /// The value, as the integer.
      #[inline]
      const fn integer(&self) -> Uint<B, $uint> {
        Uint::wrap(self.value.repr.get())
      }

      /// The non-zero integer of `value`, which is not zero, as every non-zero result of the rows
      /// is; the native `NonZero` panics otherwise.
      #[inline]
      const fn known(value: Uint<B, $uint>) -> Self {
        Self { value: Uint::wrap(int::$uint::known_nonzero(value.repr)) }
      }
    }

    impl<B: FitsBits<$uint>> NonZeroUint<B, core::num::NonZero<$uint>> {
      /// `MAX`, the bound that a saturating row returns.
      #[inline]
      const fn saturated(_: bool) -> Self {
        Self::known(Uint::<B, $uint>::MAX)
      }

      nonzero_from_str_radix!(Uint<B, $uint>);
      nonzero_byte_order!(Uint<B, $uint>);

      nonzero_table!(Uint, inherent_items![Uint<B, $uint>, NonZeroInt<B, core::num::NonZero<$int>>]);
      nonzero_uint_table!(inherent_items![Uint<B, $uint>, NonZeroInt<B, core::num::NonZero<$int>>]);
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

      /// Creates a non-zero integer of the value `value` without checking that it is not zero.
      ///
      /// # Safety
      ///
      /// `value` must not be zero, that is, [`new`](Self::new) would return `Some`. Otherwise the
      /// behavior is undefined. Zero bits hold only zero, so no call is sound there.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn new_unchecked(value: Int<B, $int>) -> Self {
        // SAFETY: the caller guarantees that `value` is not zero. Its bits are the low `Bits` bits
        // of `repr`, so `repr` is not zero either: the precondition of the native
        // `NonZero::new_unchecked`.
        let repr = unsafe { core::num::NonZero::new_unchecked(value.repr) };
        Self { value: Int::wrap(repr) }
      }

      /// Returns the value, as the integer of the same width that may be zero.
      #[must_use]
      #[inline]
      pub const fn get(self) -> Int<B, $int> {
        Int::wrap(self.value.repr.get())
      }

      /// The value, as the integer.
      #[inline]
      const fn integer(&self) -> Int<B, $int> {
        Int::wrap(self.value.repr.get())
      }

      /// The non-zero integer of `value`, which is not zero, as every non-zero result of the rows
      /// is; the native `NonZero` panics otherwise.
      #[inline]
      const fn known(value: Int<B, $int>) -> Self {
        Self { value: Int::wrap(int::$int::known_nonzero(value.repr)) }
      }

      /// A copy of `self`, for a row that returns its operand.
      #[inline]
      const fn duplicate(&self) -> Self {
        *self
      }
    }

    impl<B: FitsBits<$int>> NonZeroInt<B, core::num::NonZero<$int>> {
      /// The bound that a saturating row returns: `MAX` if `max`, and `MIN` otherwise. One bit
      /// holds only -1, which is both.
      #[inline]
      const fn saturated(max: bool) -> Self {
        if max && Self::BITS > 1 {
          Self::known(Int::<B, $int>::MAX)
        } else {
          Self::known(Int::<B, $int>::MIN)
        }
      }

      nonzero_from_str_radix!(Int<B, $int>);
      nonzero_byte_order!(Int<B, $int>);

      nonzero_table!(Int, inherent_items![Int<B, $int>, NonZeroUint<B, core::num::NonZero<$uint>>]);
      nonzero_int_table!(inherent_items![Int<B, $int>, NonZeroUint<B, core::num::NonZero<$uint>>]);
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

/// The non-zero integer of the digits of `radix` in `src`, as the integer of the same width and
/// storage parses them, or the error of the kind `Zero` for zero, as the native
/// `NonZero::from_str_radix` gives it after the errors of its integer. Requires
/// `2 <= radix <= 36`.
pub(crate) fn parse_unsigned<B: FitsBits<R::Integer>, R: NonZeroUintRepr>(
  src: &str,
  radix: u32,
) -> Result<NonZeroUint<B, R>, ParseIntError> {
  let integer =
    <R::Integer as UintCore>::from_str_radix_bits(src.as_bytes(), radix, B::U32, Token::new())?;
  match integer.to_nonzero(Token::new()) {
    Some(repr) => Ok(NonZeroUint {
      value: Uint::wrap(repr),
    }),
    None => Err(parse_errors::ZERO),
  }
}

/// The non-zero integer of the digits of `radix` in `src`, as [`parse_unsigned`] reads them for
/// `NonZeroUint`. Requires `2 <= radix <= 36`.
pub(crate) fn parse_signed<B: FitsBits<R::Integer>, R: NonZeroIntRepr>(
  src: &str,
  radix: u32,
) -> Result<NonZeroInt<B, R>, ParseIntError> {
  let integer =
    <R::Integer as IntCore>::from_str_radix_bits(src.as_bytes(), radix, B::U32, Token::new())?;
  match integer.to_nonzero(Token::new()) {
    Some(repr) => Ok(NonZeroInt {
      value: Int::wrap(repr),
    }),
    None => Err(parse_errors::ZERO),
  }
}

impl<B: FitsBits<R::Integer>, R: NonZeroUintRepr> FromStr for NonZeroUint<B, R> {
  type Err = ParseIntError;

  /// Parses a non-zero integer from decimal digits after an optional `+`, as `from_str_radix` with
  /// the radix 10 does.
  #[inline]
  fn from_str(src: &str) -> Result<Self, ParseIntError> {
    parse_unsigned(src, 10)
  }
}

impl<B: FitsBits<R::Integer>, R: NonZeroIntRepr> FromStr for NonZeroInt<B, R> {
  type Err = ParseIntError;

  /// Parses a non-zero integer from decimal digits after an optional `+` or `-`, as
  /// `from_str_radix` with the radix 10 does.
  #[inline]
  fn from_str(src: &str) -> Result<Self, ParseIntError> {
    parse_signed(src, 10)
  }
}

/// Implements the radix formats `$trait` of the non-zero integers, written `trait: shift, upper;`,
/// and the exponent formats, written `exp trait: upper;`, which write what the integer of the same
/// value writes, lent by the storage, as `Display` does.
// Trait methods cannot be `const`.
macro_rules! nonzero_formats {
  ($($trait:ident: $shift:literal, $upper:literal);+ $(;)?) => {$(
    impl<B, R: NonZeroUintRepr> fmt::$trait for NonZeroUint<B, R> {
      /// Writes what the integer of the same value writes.
      #[inline]
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <R::Integer as UintCore>::with_nonzero(
          &self.value.repr,
          |integer| integer.fmt_pow2($shift, $upper, f, Token::new()),
          Token::new(),
        )
      }
    }

    impl<B: FitsBits<R::Integer>, R: NonZeroIntRepr> fmt::$trait for NonZeroInt<B, R> {
      /// Writes what the integer of the same value writes: the two's complement of its `Bits`
      /// bits.
      #[inline]
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <R::Integer as IntCore>::with_nonzero(
          &self.value.repr,
          |integer| integer.fmt_pow2(B::U32, $shift, $upper, f, Token::new()),
          Token::new(),
        )
      }
    }
  )+};
  ($(exp $trait:ident: $upper:literal);+ $(;)?) => {$(
    impl<B, R: NonZeroUintRepr> fmt::$trait for NonZeroUint<B, R> {
      /// Writes what the integer of the same value writes.
      #[inline]
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <R::Integer as UintCore>::with_nonzero(
          &self.value.repr,
          |integer| integer.fmt_exp($upper, f, Token::new()),
          Token::new(),
        )
      }
    }

    impl<B, R: NonZeroIntRepr> fmt::$trait for NonZeroInt<B, R> {
      /// Writes what the integer of the same value writes.
      #[inline]
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <R::Integer as IntCore>::with_nonzero(
          &self.value.repr,
          |integer| integer.fmt_exp($upper, f, Token::new()),
          Token::new(),
        )
      }
    }
  )+};
}

nonzero_formats!(
  Binary: 1, false;
  Octal: 3, false;
  LowerHex: 4, false;
  UpperHex: 4, true;
);

nonzero_formats!(
  exp LowerExp: false;
  exp UpperExp: true;
);

/// Implements the division `$op` of [`Uint`], `/` or `%`, by the [`NonZeroUint`] of the same width
/// over the non-zero storage of its storage, in the four forms of values and references, and its
/// compound assignment `$assign` for a value and a reference, with the element `$part` of the
/// quotient and the remainder. The divisor is not zero and an unsigned quotient cannot overflow,
/// so nothing panics, and as for the native integers the methods are not `#[track_caller]`. The
/// form on two references divides the storage by the divisor that the non-zero storage lends, for
/// every storage; the forms with an owned operand, for a storage that holds the value inline, pass
/// references to it; and the compound assignments, for a storage that holds the value in place,
/// store the result.
macro_rules! nonzero_division {
  ($op:ident $method:ident, $assign:ident $assign_method:ident, $part:tt) => {
    impl<B: FitsBits<R>, R: InlineUintRepr> $op<NonZeroUint<B, <R as UintRepr>::NonZero>>
      for Uint<B, R>
    {
      type Output = Self;

      #[inline]
      fn $method(self, rhs: NonZeroUint<B, <R as UintRepr>::NonZero>) -> Self {
        $op::$method(&self, &rhs)
      }
    }

    impl<B: FitsBits<R>, R: InlineUintRepr> $op<&NonZeroUint<B, <R as UintRepr>::NonZero>>
      for Uint<B, R>
    {
      type Output = Self;

      #[inline]
      fn $method(self, rhs: &NonZeroUint<B, <R as UintRepr>::NonZero>) -> Self {
        $op::$method(&self, rhs)
      }
    }

    impl<B: FitsBits<R>, R: InlineUintRepr> $op<NonZeroUint<B, <R as UintRepr>::NonZero>>
      for &Uint<B, R>
    {
      type Output = Uint<B, R>;

      #[inline]
      fn $method(self, rhs: NonZeroUint<B, <R as UintRepr>::NonZero>) -> Uint<B, R> {
        $op::$method(self, &rhs)
      }
    }

    impl<B: FitsBits<R>, R: UintRepr> $op<&NonZeroUint<B, <R as UintRepr>::NonZero>>
      for &Uint<B, R>
    {
      type Output = Uint<B, R>;

      #[inline]
      fn $method(self, rhs: &NonZeroUint<B, <R as UintRepr>::NonZero>) -> Uint<B, R> {
        let division = R::with_nonzero(
          &rhs.value.repr,
          |divisor| self.repr.div_rem_bits(divisor, B::U32, Token::new()),
          Token::new(),
        );
        match division {
          Some(parts) => Uint::wrap(parts.$part),
          None => zero(),
        }
      }
    }

    impl<B: FitsBits<R>, R: UintReprMut> $assign<NonZeroUint<B, <R as UintRepr>::NonZero>>
      for Uint<B, R>
    {
      #[inline]
      fn $assign_method(&mut self, rhs: NonZeroUint<B, <R as UintRepr>::NonZero>) {
        $assign::$assign_method(self, &rhs);
      }
    }

    impl<B: FitsBits<R>, R: UintReprMut> $assign<&NonZeroUint<B, <R as UintRepr>::NonZero>>
      for Uint<B, R>
    {
      #[inline]
      fn $assign_method(&mut self, rhs: &NonZeroUint<B, <R as UintRepr>::NonZero>) {
        let result = $op::$method(&*self, rhs);
        *self = result;
      }
    }
  };
}

nonzero_division!(Div div, DivAssign div_assign, 0);
nonzero_division!(Rem rem, RemAssign rem_assign, 1);

/// Implements `|` of the non-zero integer `$name` with itself, with its integer `$integer` and of
/// the integer with it, in the four forms of values and references each, with the non-zero
/// integer as the result, and `|=` of `$name` by both, for a value and a reference. The storages of
/// `$repr` are the non-zero ones, of `$core` the sealed trait of their integer, and of `$integer`,
/// `$inline` and `$repr_mut` the integer's own. A value that is not zero has a one, which `|`
/// keeps, so nothing panics, and as for the native integers the methods are not `#[track_caller]`.
/// The forms on two references combine the storages that the non-zero storages lend, for every
/// storage; the forms with an owned operand, for a storage whose integer holds the value inline,
/// pass references; and the compound assignments, for a storage whose integer holds the value in
/// place, store the result. Each impl is keyed on the storage of its `Self` type.
macro_rules! nonzero_bitor {
  (
    $name:ident, $repr:ident, $core:ident;
    $integer:ident: $integer_repr:ident, $inline:ident, $repr_mut:ident
  ) => {
    impl<B: FitsBits<R::Integer>, R: $repr> BitOr<&$name<B, R>> for &$name<B, R> {
      type Output = $name<B, R>;

      #[inline]
      fn bitor(self, rhs: &$name<B, R>) -> $name<B, R> {
        let or = <R::Integer as $core>::with_nonzero(
          &self.value.repr,
          |lhs| {
            <R::Integer as $core>::with_nonzero(
              &rhs.value.repr,
              |rhs| lhs.or(rhs, Token::new()),
              Token::new(),
            )
          },
          Token::new(),
        );
        $name {
          value: $integer::wrap(or.known_nonzero(Token::new())),
        }
      }
    }

    impl<B: FitsBits<R::Integer>, R: $repr> BitOr<$name<B, R>> for $name<B, R>
    where
      R::Integer: $inline,
    {
      type Output = Self;

      #[inline]
      fn bitor(self, rhs: Self) -> Self {
        BitOr::bitor(&self, &rhs)
      }
    }

    impl<B: FitsBits<R::Integer>, R: $repr> BitOr<&$name<B, R>> for $name<B, R>
    where
      R::Integer: $inline,
    {
      type Output = Self;

      #[inline]
      fn bitor(self, rhs: &Self) -> Self {
        BitOr::bitor(&self, rhs)
      }
    }

    impl<B: FitsBits<R::Integer>, R: $repr> BitOr<$name<B, R>> for &$name<B, R>
    where
      R::Integer: $inline,
    {
      type Output = $name<B, R>;

      #[inline]
      fn bitor(self, rhs: $name<B, R>) -> $name<B, R> {
        BitOr::bitor(self, &rhs)
      }
    }

    impl<B: FitsBits<R::Integer>, R: $repr> BitOr<&$integer<B, R::Integer>> for &$name<B, R> {
      type Output = $name<B, R>;

      #[inline]
      fn bitor(self, rhs: &$integer<B, R::Integer>) -> $name<B, R> {
        let or = <R::Integer as $core>::with_nonzero(
          &self.value.repr,
          |lhs| lhs.or(&rhs.repr, Token::new()),
          Token::new(),
        );
        $name {
          value: $integer::wrap(or.known_nonzero(Token::new())),
        }
      }
    }

    impl<B: FitsBits<R::Integer>, R: $repr> BitOr<$integer<B, R::Integer>> for $name<B, R>
    where
      R::Integer: $inline,
    {
      type Output = Self;

      #[inline]
      fn bitor(self, rhs: $integer<B, R::Integer>) -> Self {
        BitOr::bitor(&self, &rhs)
      }
    }

    impl<B: FitsBits<R::Integer>, R: $repr> BitOr<&$integer<B, R::Integer>> for $name<B, R>
    where
      R::Integer: $inline,
    {
      type Output = Self;

      #[inline]
      fn bitor(self, rhs: &$integer<B, R::Integer>) -> Self {
        BitOr::bitor(&self, rhs)
      }
    }

    impl<B: FitsBits<R::Integer>, R: $repr> BitOr<$integer<B, R::Integer>> for &$name<B, R>
    where
      R::Integer: $inline,
    {
      type Output = $name<B, R>;

      #[inline]
      fn bitor(self, rhs: $integer<B, R::Integer>) -> $name<B, R> {
        BitOr::bitor(self, &rhs)
      }
    }

    impl<B: FitsBits<R>, R: $integer_repr> BitOr<&$name<B, <R as $integer_repr>::NonZero>>
      for &$integer<B, R>
    {
      type Output = $name<B, <R as $integer_repr>::NonZero>;

      #[inline]
      fn bitor(
        self,
        rhs: &$name<B, <R as $integer_repr>::NonZero>,
      ) -> $name<B, <R as $integer_repr>::NonZero> {
        let or = R::with_nonzero(
          &rhs.value.repr,
          |rhs| self.repr.or(rhs, Token::new()),
          Token::new(),
        );
        $name {
          value: $integer::wrap(or.known_nonzero(Token::new())),
        }
      }
    }

    impl<B: FitsBits<R>, R: $inline> BitOr<$name<B, <R as $integer_repr>::NonZero>>
      for $integer<B, R>
    {
      type Output = $name<B, <R as $integer_repr>::NonZero>;

      #[inline]
      fn bitor(
        self,
        rhs: $name<B, <R as $integer_repr>::NonZero>,
      ) -> $name<B, <R as $integer_repr>::NonZero> {
        BitOr::bitor(&self, &rhs)
      }
    }

    impl<B: FitsBits<R>, R: $inline> BitOr<&$name<B, <R as $integer_repr>::NonZero>>
      for $integer<B, R>
    {
      type Output = $name<B, <R as $integer_repr>::NonZero>;

      #[inline]
      fn bitor(
        self,
        rhs: &$name<B, <R as $integer_repr>::NonZero>,
      ) -> $name<B, <R as $integer_repr>::NonZero> {
        BitOr::bitor(&self, rhs)
      }
    }

    impl<B: FitsBits<R>, R: $inline> BitOr<$name<B, <R as $integer_repr>::NonZero>>
      for &$integer<B, R>
    {
      type Output = $name<B, <R as $integer_repr>::NonZero>;

      #[inline]
      fn bitor(
        self,
        rhs: $name<B, <R as $integer_repr>::NonZero>,
      ) -> $name<B, <R as $integer_repr>::NonZero> {
        BitOr::bitor(self, &rhs)
      }
    }

    impl<B: FitsBits<R::Integer>, R: $repr> BitOrAssign<$name<B, R>> for $name<B, R>
    where
      R::Integer: $repr_mut,
    {
      #[inline]
      fn bitor_assign(&mut self, rhs: Self) {
        BitOrAssign::bitor_assign(self, &rhs);
      }
    }

    impl<B: FitsBits<R::Integer>, R: $repr> BitOrAssign<&$name<B, R>> for $name<B, R>
    where
      R::Integer: $repr_mut,
    {
      #[inline]
      fn bitor_assign(&mut self, rhs: &Self) {
        let or = BitOr::bitor(&*self, rhs);
        *self = or;
      }
    }

    impl<B: FitsBits<R::Integer>, R: $repr> BitOrAssign<$integer<B, R::Integer>> for $name<B, R>
    where
      R::Integer: $repr_mut,
    {
      #[inline]
      fn bitor_assign(&mut self, rhs: $integer<B, R::Integer>) {
        BitOrAssign::bitor_assign(self, &rhs);
      }
    }

    impl<B: FitsBits<R::Integer>, R: $repr> BitOrAssign<&$integer<B, R::Integer>> for $name<B, R>
    where
      R::Integer: $repr_mut,
    {
      #[inline]
      fn bitor_assign(&mut self, rhs: &$integer<B, R::Integer>) {
        let or = BitOr::bitor(&*self, rhs);
        *self = or;
      }
    }
  };
}

nonzero_bitor!(NonZeroUint, NonZeroUintRepr, UintCore; Uint: UintRepr, InlineUintRepr, UintReprMut);
nonzero_bitor!(NonZeroInt, NonZeroIntRepr, IntCore; Int: IntRepr, InlineIntRepr, IntReprMut);

// As for `Int`, the negation of a reference computes for every storage, and the negation of a
// value, for a storage whose integer holds it inline, passes a reference. `-MIN` overflows as a
// native negation does: with overflow checks it panics, reporting the caller, and without them it
// returns `MIN`.
impl<B: FitsBits<R::Integer>, R: NonZeroIntRepr> Neg for NonZeroInt<B, R>
where
  R::Integer: InlineIntRepr,
{
  type Output = Self;

  #[inline]
  #[track_caller]
  fn neg(self) -> Self {
    Neg::neg(&self)
  }
}

impl<B: FitsBits<R::Integer>, R: NonZeroIntRepr> Neg for &NonZeroInt<B, R> {
  type Output = NonZeroInt<B, R>;

  #[inline]
  #[track_caller]
  fn neg(self) -> NonZeroInt<B, R> {
    let (negated, overflow) = SignedNonZero::overflowing_neg(self);
    if overflow {
      overflow::forward_neg();
    }
    negated
  }
}

/// The value of a non-zero integer as the integer that may be zero, and the non-zero integer of a
/// value that is not zero, for the generic implementations of the traits, over every storage.
// No width: the casts build the non-zero integer of the other sign, whose storage has a width
// relation of its own, which `FitsBits` of the storage of `Self` does not imply.
pub(crate) trait Parts: Sized {
  /// The integer that may be zero, of the same width and storage.
  type Integer;

  /// The value, as the integer. It clones the storage.
  fn integer(&self) -> Self::Integer;

  /// A clone of `self`.
  fn duplicate(&self) -> Self;

  /// The non-zero integer of `value`, which is not zero, as every non-zero result of the rows is.
  fn known(value: Self::Integer) -> Self;
}

impl<B, R: NonZeroUintRepr> Parts for NonZeroUint<B, R> {
  type Integer = Uint<B, R::Integer>;

  #[inline]
  fn integer(&self) -> Uint<B, R::Integer> {
    Uint::wrap(<R::Integer as UintCore>::from_nonzero(
      self.value.repr.clone(),
      Token::new(),
    ))
  }

  #[inline]
  fn duplicate(&self) -> Self {
    self.clone()
  }

  #[inline]
  fn known(value: Uint<B, R::Integer>) -> Self {
    Self {
      value: Uint::wrap(value.repr.known_nonzero(Token::new())),
    }
  }
}

impl<B, R: NonZeroIntRepr> Parts for NonZeroInt<B, R> {
  type Integer = Int<B, R::Integer>;

  #[inline]
  fn integer(&self) -> Int<B, R::Integer> {
    Int::wrap(<R::Integer as IntCore>::from_nonzero(
      self.value.repr.clone(),
      Token::new(),
    ))
  }

  #[inline]
  fn duplicate(&self) -> Self {
    self.clone()
  }

  #[inline]
  fn known(value: Int<B, R::Integer>) -> Self {
    Self {
      value: Int::wrap(value.repr.known_nonzero(Token::new())),
    }
  }
}

/// The bound of the range that a saturating row of a non-zero integer returns, over every storage.
pub(crate) trait Saturated: Sized {
  /// `MAX` if `max`, and `MIN` otherwise. A saturating row of `NonZeroUint` passes only `MAX`, and
  /// one signed bit holds only -1, which is both.
  fn saturated(max: bool) -> Self;
}

// The limits come from the sealed storage methods, without a constant.
impl<B: FitsBits<R::Integer>, R: NonZeroUintRepr> Saturated for NonZeroUint<B, R> {
  #[inline]
  fn saturated(_: bool) -> Self {
    let max = <R::Integer as UintCore>::zero(Token::new()).not_bits(B::U32, Token::new());
    Self::known(Uint::wrap(max))
  }
}

impl<B: FitsBits<R::Integer>, R: NonZeroIntRepr> Saturated for NonZeroInt<B, R> {
  #[inline]
  fn saturated(max: bool) -> Self {
    let bits = B::U32;
    // The sign bit alone, which sign-extends to `MIN`. A value exists from one bit up.
    let min = match <R::Integer as IntCore>::zero(Token::new()).with_bit(
      bits.wrapping_sub(1),
      true,
      bits,
      Token::new(),
    ) {
      Some(min) => min,
      None => zero(),
    };
    // `MAX` is `!MIN` from two bits; one bit holds only -1 and 0, so its largest non-zero value is
    // -1, `MIN`.
    let bound = if max && bits > 1 {
      min.not_bits(bits, Token::new())
    } else {
      min
    };
    Self::known(Int::wrap(bound))
  }
}
