//! The width relation between a type-level number of bits and an integer storage, the storages of
//! [`Uint`](crate::Uint) and [`Int`](crate::Int), and their default storage.

use core::{
  cmp::Ordering,
  fmt,
  hash::Hasher,
  num::ParseIntError,
  ops::{Add, Shl, Shr},
};

use typenum::{
  B0, B1, Const, IsEqual, IsLess, IsLessOrEqual, Le, LeEq, Shleft, Shright, Sum, ToUInt, U, U3, U4,
  U5, U6, U7, U8, U15, U16, U31, U32, U63, U64, U127, U128, U65536, U4294967296, UInt, UTerm,
  Unsigned,
};

pub use array_size::ArraySize;
pub(crate) use array_size::{Array, ArrayLength};
#[cfg(any(
  feature = "std",
  feature = "alloc",
  all(feature = "triomphe", target_has_atomic = "ptr")
))]
pub(crate) use heap::{ArrayLimb, ArrayRepr};
pub(crate) use sealed::{
  InlineIntConsts, InlineUintConsts, IntCore, IntWidth, ReprLimb, TypenumNumber, UintCore,
  UintWidth, View,
};

use sealed::{
  BitsBelow2Pow32, BitsFill, BitsFit, BitsOf, Canonical, DefaultArray, DefaultRepr, LimbFor,
  MutableRepr, NativeBitsFit, NativeReprFilled, NonZeroRepr, ReprFilled, Width,
};

use crate::{
  FromFloatError, Int, RoundingMode, Uint, format,
  int::{
    self,
    float::{FloatFormat, FloatParts},
    signed, unsigned,
  },
  layout::Token,
};

mod array_size;
#[cfg(any(
  feature = "std",
  feature = "alloc",
  all(feature = "triomphe", target_has_atomic = "ptr")
))]
mod heap;

mod sealed {
  use core::{
    cmp::Ordering, fmt, hash::Hasher, marker::PhantomData, num::ParseIntError, ops::Deref,
  };

  use typenum::{B1, UInt, UTerm};

  use super::{FitsBits, InlineIntRepr, InlineUintRepr, IntRepr, UintRepr};
  use crate::{
    FromFloatError, Int, RoundingMode, Uint,
    int::float::{FloatFormat, FloatParts},
    layout::Token,
  };

  /// A `typenum` number in canonical form: `UTerm`, or a `UInt` whose most significant bit is `B1`.
  /// `typenum` forbids leading zero bits but does not enforce the rule, and its comparisons are
  /// structural, so a number written with leading zeros compares as longer, not as its value.
  #[diagnostic::on_unimplemented(
    message = "`{Self}` is not a `typenum` number",
    label = "the width has leading zero bits",
    note = "a `typenum` number is `UTerm`, or a `UInt` whose most significant bit is `B1`; `UInt<UTerm, B0>` is not a number"
  )]
  pub trait Canonical {}

  impl Canonical for UTerm {}

  impl Canonical for UInt<UTerm, B1> {}

  impl<U, B, X> Canonical for UInt<UInt<U, B>, X> where UInt<U, B>: Canonical {}

  /// Seals [`ArraySize`](super::ArraySize), [`BitWidth`](super::BitWidth) and
  /// [`DigitCount`](crate::DigitCount), the traits that only `typenum` numbers implement. Two impls,
  /// for `UTerm` and for every `UInt`, cover every number, and the orphan rules keep other crates
  /// from implementing a trait of this crate for a type of `typenum`, so no other crate can
  /// implement this trait and none can implement the traits it seals.
  pub trait TypenumNumber {}

  impl TypenumNumber for UTerm {}

  impl<U, B> TypenumNumber for UInt<U, B> {}

  /// Seals [`FitsBits`](super::FitsBits).
  pub trait BitsOf<Repr> {}

  /// Seals [`FillsRepr`](super::FillsRepr).
  pub trait BitsFill<Repr> {}

  /// Implemented only for `B1`, the result of `Bits <= B * N` when `[L; N]`, of limbs of `B` bits,
  /// holds `Bits` bits.
  // The message and label name no storage, so that every storage can share them.
  #[diagnostic::on_unimplemented(
    message = "the storage cannot hold `Bits` bits",
    label = "`Bits` is greater than the bits of the storage",
    note = "`[u8; N]`, `[u16; N]`, `[u32; N]`, `[u64; N]` and `[u128; N]` hold `8 * N`, `16 * N`, `32 * N`, `64 * N` and `128 * N` bits"
  )]
  pub trait BitsFit {}

  impl BitsFit for typenum::B1 {}

  /// Implemented only for `B1`, the result of `Bits < 2^32`, below which `typenum`'s `U32` is
  /// exact.
  #[diagnostic::on_unimplemented(
    message = "`Bits` must be below 2^32",
    label = "`Bits` is 2^32 or more",
    note = "the number of bits is a `u32`, as the constant `BITS` is"
  )]
  pub trait BitsBelow2Pow32 {}

  impl BitsBelow2Pow32 for typenum::B1 {}

  /// Implemented only for `B1`, the result of `Bits == B * N` when `Bits` fills `[L; N]`, of limbs
  /// of `B` bits.
  #[diagnostic::on_unimplemented(
    message = "`Bits` does not fill the storage",
    label = "`Bits` is less than the bits of the storage",
    note = "`[u8; N]`, `[u16; N]`, `[u32; N]`, `[u64; N]` and `[u128; N]` hold `8 * N`, `16 * N`, `32 * N`, `64 * N` and `128 * N` bits",
    note = "`checked_from_limbs` takes the limbs of any width that fits and checks the value"
  )]
  pub trait ReprFilled {}

  impl ReprFilled for typenum::B1 {}

  /// Implemented only for `B1`, the result of `Bits <= B` when a native integer of `B` bits holds
  /// `Bits` bits.
  #[diagnostic::on_unimplemented(
    message = "the storage cannot hold `Bits` bits",
    label = "`Bits` is greater than the bits of the storage",
    note = "`u8` and `i8` hold 8 bits, `u16` and `i16` 16, `u32` and `i32` 32, `u64` and `i64` 64, and `u128` and `i128` 128"
  )]
  pub trait NativeBitsFit {}

  impl NativeBitsFit for typenum::B1 {}

  /// Implemented only for `B1`, the result of `Bits == B` when `Bits` fills a native integer of `B`
  /// bits.
  #[diagnostic::on_unimplemented(
    message = "`Bits` does not fill the storage",
    label = "`Bits` is less than the bits of the storage",
    note = "`u8` and `i8` hold 8 bits, `u16` and `i16` 16, `u32` and `i32` 32, `u64` and `i64` 64, and `u128` and `i128` 128"
  )]
  pub trait NativeReprFilled {}

  impl NativeReprFilled for typenum::B1 {}

  /// Seals [`UintReprMut`](super::UintReprMut) and [`IntReprMut`](super::IntReprMut): the
  /// storages that hold the value in place, which every native integer and every array of limbs
  /// does.
  pub trait MutableRepr {}

  /// Seals [`LimbArray`](super::LimbArray). Each impl requires `Bits: Unsigned`, which `typenum`
  /// seals, so that no other crate can implement `LimbArray` for a limb type with a width of its
  /// own type.
  pub trait LimbFor<Bits> {}

  /// Seals [`NonZeroUintRepr`](super::NonZeroUintRepr) and
  /// [`NonZeroIntRepr`](super::NonZeroIntRepr): the native `NonZero` integers and the arrays of
  /// limbs.
  pub trait NonZeroRepr {}

  /// Seals [`InlineUintRepr`](super::InlineUintRepr) and carries the constants of `Uint`.
  pub trait InlineUintConsts: UintRepr + Copy {
    /// Zero.
    const ZERO: Self;

    /// One. `Uint` reads it only for a width of at least one bit, which an array of no limbs does
    /// not hold, and whose `ONE` fails to evaluate.
    const ONE: Self;

    /// The constants of `Uint<B, Self>` that depend on the width `B`.
    type Width<B: FitsBits<Self>>: UintWidth<Self>;
  }

  /// Seals [`InlineIntRepr`](super::InlineIntRepr) and carries the constants of `Int`.
  pub trait InlineIntConsts: IntRepr + Copy {
    /// Zero.
    const ZERO: Self;

    /// One. `Int` reads it only for a width of at least two bits, which an array of no limbs does
    /// not hold, and whose `ONE` fails to evaluate.
    const ONE: Self;

    /// The constants of `Int<B, Self>` that depend on the width `B`.
    type Width<B: FitsBits<Self>>: IntWidth<Self>;
  }

  /// The constants of `Uint<B, R>` that depend on the width `B`, and those of the non-zero unsigned
  /// integer of `B` bits in the non-zero storage of `R`.
  pub trait UintWidth<R: UintCore> {
    /// The largest value, `2^B - 1`.
    const MAX: R;

    /// The smallest non-zero value, one. It is read only from one bit up.
    const NONZERO_MIN: R::NonZeroStorage;

    /// The largest non-zero value, `MAX`. It is read only from one bit up: at zero bits, `MAX` is
    /// zero.
    const NONZERO_MAX: R::NonZeroStorage;
  }

  /// The constants of `Int<B, R>` that depend on the width `B`, and those of the non-zero signed
  /// integer of `B` bits in the non-zero storage of `R`.
  pub trait IntWidth<R: IntCore> {
    /// The smallest value, `-2^(B - 1)`, or zero for zero bits.
    const MIN: R;

    /// The largest value, `2^(B - 1) - 1`, or zero for zero bits.
    const MAX: R;

    /// The smallest non-zero value, `MIN`. It is read only from one bit up: at zero bits, `MIN` is
    /// zero.
    const NONZERO_MIN: R::NonZeroStorage;

    /// The largest non-zero value: `MAX` from two bits, and `MIN`, -1, at one bit, whose `MAX` is
    /// zero. It is read only from one bit up.
    const NONZERO_MAX: R::NonZeroStorage;
  }

  /// Carries the constants of the width `B` in the inline storage `R`, as its impls of
  /// [`UintWidth`] and [`IntWidth`]. A generic constant cannot call a trait method, but it can read
  /// the constants of an associated type, which each storage computes with its own `const fn`s.
  pub struct Width<B, R>(PhantomData<(B, R)>);

  /// The default storages of `Self` bits, chosen by `Small`, the result of `Self <= 128`: `B1` reads
  /// the native table and `B0` takes `Limbs<Self>`. The supertraits make the compiler check each
  /// row of the table.
  // Only `BitWidth` asks for this trait, for a `UInt`. The table has every number from 1 to 128 and
  // one impl covers `B0`, so a width finds no impl only when it compares as at most 128 without
  // being in the table: when it has leading zero bits.
  #[diagnostic::on_unimplemented(
    message = "`{Self}` is not a `typenum` number",
    label = "the width has leading zero bits",
    note = "a `typenum` number is `UTerm`, or a `UInt` whose most significant bit is `B1`; `UInt<UTerm, B0>` is not a number"
  )]
  pub trait DefaultRepr<Small>:
    FitsBits<<Self as DefaultRepr<Small>>::Unsigned>
    + FitsBits<<Self as DefaultRepr<Small>>::Signed>
  {
    /// The default storage of `Uint<Self>`.
    type Unsigned: InlineUintRepr
      + UintRepr<Signed = <Self as DefaultRepr<Small>>::Signed, NonZero: Copy>;

    /// The default storage of `Int<Self>`.
    type Signed: InlineIntRepr
      + IntRepr<Unsigned = <Self as DefaultRepr<Small>>::Unsigned, NonZero: Copy>;
  }

  /// Implemented only for `B1`, the result of `Bits <= 65536`, with the array of 64-bit limbs that
  /// holds `Bits` bits.
  // A check of its own keeps a wider width from looking up the array, which would report a second
  // error, and keeps the range of the default from following the lengths that `ArraySize` covers.
  #[diagnostic::on_unimplemented(
    message = "`Uint` and `Int` have a default storage only for 0 to 65536 bits",
    label = "`Bits` is greater than 65536",
    note = "a wider integer names its storage, as in `Uint<U100000, [u64; 2048]>`"
  )]
  pub trait DefaultArray<Bits> {
    /// `Limbs<Bits>`.
    type Array: InlineUintRepr
      + InlineIntRepr
      + IntRepr<Unsigned = Self::Array, NonZero = Self::Array>
      + UintRepr<Signed = Self::Array, NonZero = Self::Array>;
  }

  /// The type of the limbs of a storage, which the slices of limbs that it reads hold: `L` for an
  /// array `[L; N]`, and for a native integer the unsigned integer of its width, of which it is one
  /// limb, as its hash reads it.
  pub trait ReprLimb {
    /// The type of a limb.
    type Limb;
  }

  // One implementation for every array, so that `<[L; N] as ReprLimb>::Limb` is `L` also in code
  // generic over the limb type.
  impl<L, const N: usize> ReprLimb for [L; N] {
    type Limb = L;
  }

  /// The integer of a non-zero integer, as the storage cores give it to the rows of the non-zero
  /// integers: lent where the non-zero storage is the storage of the integer, as for an array, and
  /// a copy of it for a native `NonZero`. It dereferences to the integer.
  pub enum View<'a, T> {
    /// The integer itself, lent.
    Lent(&'a T),
    /// A copy of the integer.
    Owned(T),
  }

  impl<T> Deref for View<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
      match self {
        Self::Lent(integer) => integer,
        Self::Owned(integer) => integer,
      }
    }
  }

  /// Seals [`UintRepr`](super::UintRepr) and gives the trait implementations of
  /// [`Uint`](crate::Uint) access to the integer core.
  ///
  /// A bound `R: UintRepr` makes these methods callable from any crate, so each takes a [`Token`].
  /// They rely on the width invariant of `Uint`: the representation, read as an unsigned integer,
  /// equals the value.
  pub trait UintCore: Sized {
    /// The storage of the two's complement integer of the same size, which holds the bits of a
    /// value read with a sign: [`UintRepr::Signed`](super::UintRepr::Signed).
    type Complement;

    /// The storage of a value that is not zero: [`UintRepr::NonZero`](super::UintRepr::NonZero),
    /// the native `NonZero` of a native integer, whose niche gives `Option` the size of the
    /// integer, and the array itself for an array of limbs.
    type NonZeroStorage;

    /// The storage that computes the values of this storage and holds one inline: the storage
    /// itself for a native integer or an array of limbs, and the array of a heap storage.
    type Inline: UintCore;

    /// Lends the value, as its inline storage.
    fn inline(&self, token: Token) -> &Self::Inline;

    /// A storage of the value `value`: `value` itself, or a new allocation that holds it.
    fn from_inline(value: Self::Inline, token: Token) -> Self;

    /// Stores the value `value`: in place where the storage holds its value in place, and
    /// otherwise in a new allocation, which replaces the storage.
    fn set_inline(&mut self, value: Self::Inline, token: Token);

    /// Zero.
    fn zero(token: Token) -> Self;

    /// Compares the values of `self` and `other`.
    fn compare(&self, other: &Self, token: Token) -> Ordering;

    /// Compares the value with `rhs`, as integers.
    fn compare_u128(&self, rhs: u128, token: Token) -> Ordering;

    /// Compares the value with `rhs`, as integers: the value is above every negative `rhs`.
    fn compare_i128(&self, rhs: i128, token: Token) -> Ordering;

    /// Writes the lowest `ceil(bits / 64)` 64-bit words of the value to `state`, least significant
    /// first, extended by zeros beyond the storage. Requires the value to have `bits` bits.
    fn hash_limbs<H: Hasher>(&self, bits: u32, state: &mut H, token: Token);

    /// Writes the value in decimal, honoring the flags of `f` as integers do.
    fn fmt_decimal(&self, f: &mut fmt::Formatter<'_>, token: Token) -> fmt::Result;

    /// Writes the value in the radix `2^shift`, binary for 1, octal for 3 and hexadecimal for 4,
    /// in uppercase if `upper`, honoring the flags of `f` as the radix formats of integers do.
    fn fmt_pow2(
      &self,
      shift: u32,
      upper: bool,
      f: &mut fmt::Formatter<'_>,
      token: Token,
    ) -> fmt::Result;

    /// Writes the value in scientific notation, with `E` if `upper` and `e` otherwise, honoring
    /// the flags of `f` as integers do: a precision below the digits after the first rounds them
    /// half to even.
    fn fmt_exp(&self, upper: bool, f: &mut fmt::Formatter<'_>, token: Token) -> fmt::Result;

    /// The value of `src`, an optional `+` and the digits of `radix`, as an integer of `bits` bits,
    /// or the error of the native `from_str_radix`. Requires `2 <= radix <= 36` and `bits` to be at
    /// most the bits of the storage.
    fn from_str_radix_bits(
      src: &[u8],
      radix: u32,
      bits: u32,
      token: Token,
    ) -> Result<Self, ParseIntError>;

    /// Writes the low `out.len()` bytes of the storage to `out`, least significant first, or most
    /// significant first if `big_endian`. Requires `out.len()` to be at most the bytes of the
    /// storage.
    fn write_bytes(&self, out: &mut [u8], big_endian: bool, token: Token);

    /// The integer in `bytes`, least significant byte first, or most significant first if
    /// `big_endian`, unsigned, as an integer of `bits` bits, or `None` if it is `2^bits` or more.
    /// Requires `bits` to be at most the bits of the storage.
    fn checked_read_bytes(bytes: &[u8], big_endian: bool, bits: u32, token: Token) -> Option<Self>;

    /// The integer whose digits in `radix` are `digits`, most significant first if `big_endian` and
    /// least significant first otherwise, as an integer of `bits` bits, or `None` if a digit is not
    /// below `radix` or the integer is `2^bits` or more. No digits are zero. Requires
    /// `2 <= radix <= 256` and `bits` to be at most the bits of the storage.
    fn checked_read_digits(
      digits: &[u8],
      big_endian: bool,
      radix: u32,
      bits: u32,
      token: Token,
    ) -> Option<Self>;

    /// `self * factor + addend` if it has `bits` bits, or `None` otherwise. Requires `bits` to be
    /// at most the bits of the storage.
    fn checked_mul_add_u64(
      &self,
      factor: u64,
      addend: u64,
      bits: u32,
      token: Token,
    ) -> Option<Self>;

    /// The quotient and the remainder of the value divided by `divisor`. Requires `divisor != 0`.
    fn div_rem_u64(&self, divisor: u64, token: Token) -> (Self, u64);

    /// The integer in `bytes`, read as [`checked_read_bytes`](Self::checked_read_bytes) reads it,
    /// modulo `2^bits`. Requires `bits` to be at most the bits of the storage.
    fn wrapping_read_bytes(bytes: &[u8], big_endian: bool, bits: u32, token: Token) -> Self;

    /// The integer in `bytes`, read as [`checked_read_bytes`](Self::checked_read_bytes) reads it,
    /// or `2^bits - 1` if it is above that. Requires `bits` to be at most the bits of the storage.
    fn saturating_read_bytes(bytes: &[u8], big_endian: bool, bits: u32, token: Token) -> Self;

    /// The byte of the storage at `index`, its bits from `8 * index` to `8 * index + 7`. Requires
    /// `index` to be below the bytes of the storage.
    fn byte_at(&self, index: usize, token: Token) -> u8;

    /// The low `bits / 8` bytes of the value in reverse order. Requires `bits` to be a multiple of
    /// 8 and at most the bits of the storage, and the value to have `bits` bits.
    fn swap_bytes_bits(&self, bits: u32, token: Token) -> Self;

    /// Returns `true` if the value is zero.
    fn is_zero(&self, token: Token) -> bool;

    /// Returns `true` if the value has `bits` bits: every bit at index `bits` or above is zero.
    /// Requires `bits` to be at most the bits of the storage.
    fn fits_bits(&self, bits: u32, token: Token) -> bool;

    /// `self + rhs` modulo `2^bits`, and whether the sum is `2^bits` or more. Requires `bits` to
    /// be at most the bits of the storage and both values to have `bits` bits.
    fn overflowing_add_bits(&self, rhs: &Self, bits: u32, token: Token) -> (Self, bool);

    /// `self - rhs` modulo `2^bits`, and whether `rhs` is greater than `self`. Requires `bits` to
    /// be at most the bits of the storage and both values to have `bits` bits.
    fn overflowing_sub_bits(&self, rhs: &Self, bits: u32, token: Token) -> (Self, bool);

    /// `self * rhs` modulo `2^bits`, and whether the product is `2^bits` or more. Requires `bits`
    /// to be at most the bits of the storage and both values to have `bits` bits.
    fn overflowing_mul_bits(&self, rhs: &Self, bits: u32, token: Token) -> (Self, bool);

    /// `-self` modulo `2^bits`, and whether `self` is not zero. Requires `bits` to be at most the
    /// bits of the storage and the value to have `bits` bits.
    fn overflowing_neg_bits(&self, bits: u32, token: Token) -> (Self, bool);

    /// `self + rhs + carry` modulo `2^bits`, and whether the sum is `2^bits` or more: a row of its
    /// own, so that a storage builds only the sum that it returns. Requires `bits` to be at most
    /// the bits of the storage and both values to have `bits` bits.
    fn carrying_add_bits(&self, rhs: &Self, carry: bool, bits: u32, token: Token) -> (Self, bool);

    /// `self - rhs - borrow` modulo `2^bits`, and whether the difference is negative: a row of its
    /// own, as [`carrying_add_bits`](Self::carrying_add_bits) is. Requires `bits` to be at most
    /// the bits of the storage and both values to have `bits` bits.
    fn borrowing_sub_bits(&self, rhs: &Self, borrow: bool, bits: u32, token: Token)
    -> (Self, bool);

    /// Returns `true` if the value is one.
    fn is_one(&self, token: Token) -> bool;

    /// The quotient and the remainder of `self / rhs`, and `false`, which no unsigned quotient
    /// overflows, or `None` if `rhs` is zero. Requires both values to have `bits` bits.
    fn div_rem_bits(&self, rhs: &Self, bits: u32, token: Token) -> Option<(Self, Self, bool)>;

    /// The Euclidean quotient and remainder of `self / rhs`, which for unsigned values are those
    /// of [`div_rem_bits`](Self::div_rem_bits).
    fn div_rem_euclid_bits(
      &self,
      rhs: &Self,
      bits: u32,
      token: Token,
    ) -> Option<(Self, Self, bool)>;

    /// `self / rhs` rounded down, and `false`, or `None` if `rhs` is zero. Requires both values to
    /// have `bits` bits.
    fn div_floor_bits(&self, rhs: &Self, bits: u32, token: Token) -> Option<(Self, bool)>;

    /// `self / rhs` rounded up, and `false`, or `None` if `rhs` is zero. Requires both values to
    /// have `bits` bits.
    fn div_ceil_bits(&self, rhs: &Self, bits: u32, token: Token) -> Option<(Self, bool)>;

    /// The smallest multiple of `rhs` at least `self`, modulo `2^bits`, and whether it is
    /// `2^bits` or more, or `None` if `rhs` is zero. Requires `bits` to be at most the bits of the
    /// storage and both values to have `bits` bits.
    fn next_multiple_of_bits(&self, rhs: &Self, bits: u32, token: Token) -> Option<(Self, bool)>;

    /// `self / rhs` if `rhs` divides `self`, or `None` if `rhs` is zero or the division leaves a
    /// remainder. Requires both values to have `bits` bits.
    fn checked_div_exact_bits(&self, rhs: &Self, bits: u32, token: Token) -> Option<Self>;

    /// `self^exp` modulo `2^bits`, and whether it is `2^bits` or more. Requires `bits` to be at
    /// most the bits of the storage and the value to have `bits` bits.
    fn overflowing_pow_bits(&self, exp: u32, bits: u32, token: Token) -> (Self, bool);

    /// The square root of the value, rounded down, which every unsigned value has.
    fn checked_isqrt(&self, token: Token) -> Option<Self>;

    /// The `n`th root of the value, rounded down, or `None` if `n` is zero.
    fn checked_nth_root(&self, n: u32, token: Token) -> Option<Self>;

    /// The logarithm of the value to the base `base`, rounded down, or `None` if the value is
    /// zero or `base` is below 2.
    fn checked_ilog(&self, base: &Self, token: Token) -> Option<u32>;

    /// The base-2 logarithm of the value, rounded down, or `None` if the value is zero.
    fn checked_ilog2(&self, token: Token) -> Option<u32>;

    /// The base-10 logarithm of the value, rounded down, or `None` if the value is zero.
    fn checked_ilog10(&self, token: Token) -> Option<u32>;

    /// `(self + rhs) / 2` rounded down.
    fn midpoint(&self, rhs: &Self, token: Token) -> Self;

    /// `|self - rhs|`.
    fn abs_diff(&self, rhs: &Self, token: Token) -> Self;

    /// Returns `true` if the value is a multiple of `rhs`: if `self / rhs` leaves no remainder, or
    /// if both are zero.
    fn is_multiple_of(&self, rhs: &Self, token: Token) -> bool;

    /// The smallest power of two at least the value, modulo `2^bits`, and whether it is `2^bits`
    /// or more. Requires `bits` to be at most the bits of the storage and the value to have `bits`
    /// bits.
    fn next_power_of_two_bits(&self, bits: u32, token: Token) -> (Self, bool);

    /// `self & rhs`.
    fn and(&self, rhs: &Self, token: Token) -> Self;

    /// `self | rhs`.
    fn or(&self, rhs: &Self, token: Token) -> Self;

    /// `self ^ rhs`.
    fn xor(&self, rhs: &Self, token: Token) -> Self;

    /// `!self` modulo `2^bits`: every bit below index `bits` flipped. Requires `bits` to be at most
    /// the bits of the storage and the value to have `bits` bits.
    fn not_bits(&self, bits: u32, token: Token) -> Self;

    /// The number of ones of the value, which has `bits` bits.
    fn count_ones_bits(&self, bits: u32, token: Token) -> u32;

    /// The number of zeros of the value from bit `bits - 1` down to its highest one, or `bits` for
    /// zero. Requires `bits` to be at most the bits of the storage and the value to have `bits`
    /// bits.
    fn leading_zeros_bits(&self, bits: u32, token: Token) -> u32;

    /// The number of zeros of the value below its lowest one, or `bits` for zero. Requires the
    /// value to have `bits` bits.
    fn trailing_zeros_bits(&self, bits: u32, token: Token) -> u32;

    /// The number of ones of the value from bit `bits - 1` down. Requires `bits` to be at most the
    /// bits of the storage and the value to have `bits` bits.
    fn leading_ones_bits(&self, bits: u32, token: Token) -> u32;

    /// The number of ones of the value from bit 0 up. Requires `bits` to be at most the bits of the
    /// storage and the value to have `bits` bits.
    fn trailing_ones_bits(&self, bits: u32, token: Token) -> u32;

    /// The index of the highest one of the value, which has `bits` bits, or `None` for zero.
    fn highest_one_bits(&self, bits: u32, token: Token) -> Option<u32>;

    /// The index of the lowest one of the value, or `None` for zero.
    fn lowest_one(&self, token: Token) -> Option<u32>;

    /// The value with only its highest one, or zero for zero.
    fn isolate_highest_one_bits(&self, bits: u32, token: Token) -> Self;

    /// The value with only its lowest one, or zero for zero.
    fn isolate_lowest_one_bits(&self, bits: u32, token: Token) -> Self;

    /// The bits of the value below index `bits` in reverse order. Requires `bits` to be at most
    /// the bits of the storage and the value to have `bits` bits.
    fn reverse_bits_bits(&self, bits: u32, token: Token) -> Self;

    /// The value rotated left by `n` places within `bits` bits. Requires `bits` to be at most the
    /// bits of the storage and the value to have `bits` bits.
    fn rotate_left_bits(&self, n: u32, bits: u32, token: Token) -> Self;

    /// The value rotated right by `n` places within `bits` bits. Requires `bits` to be at most the
    /// bits of the storage and the value to have `bits` bits.
    fn rotate_right_bits(&self, n: u32, bits: u32, token: Token) -> Self;

    /// `self << (rhs % bits)` modulo `2^bits`, and whether `rhs >= bits`, as it always is at zero
    /// bits. Requires `bits` to be at most the bits of the storage and the value to have `bits`
    /// bits.
    fn overflowing_shl_bits(&self, rhs: u32, bits: u32, token: Token) -> (Self, bool);

    /// `self >> (rhs % bits)`, and whether `rhs >= bits`, as it always is at zero bits. Requires
    /// `bits` to be at most the bits of the storage and the value to have `bits` bits.
    fn overflowing_shr_bits(&self, rhs: u32, bits: u32, token: Token) -> (Self, bool);

    /// `self << rhs` modulo `2^bits`, which is zero for `rhs >= bits`. Requires `bits` to be at
    /// most the bits of the storage.
    fn unbounded_shl_bits(&self, rhs: u32, bits: u32, token: Token) -> Self;

    /// `self >> rhs`, which is zero for `rhs >= bits`. Requires the value to have `bits` bits.
    fn unbounded_shr_bits(&self, rhs: u32, bits: u32, token: Token) -> Self;

    /// `self << rhs` if `rhs < bits` and the shift moves no one out of `bits` bits, or `None`
    /// otherwise. Requires `bits` to be at most the bits of the storage and the value to have
    /// `bits` bits.
    fn shl_exact_bits(&self, rhs: u32, bits: u32, token: Token) -> Option<Self>;

    /// `self >> rhs` if `rhs < bits` and the shift moves no one out, or `None` otherwise. Requires
    /// the value to have `bits` bits.
    fn shr_exact_bits(&self, rhs: u32, bits: u32, token: Token) -> Option<Self>;

    /// The bit at `index` of the value, or `false` for `index >= bits`. Requires `bits` to be at
    /// most the bits of the storage.
    fn bit(&self, index: u32, bits: u32, token: Token) -> bool;

    /// The value with the bit at `index` set if `set`, and cleared otherwise, or `None` for
    /// `index >= bits`. Requires `bits` to be at most the bits of the storage.
    fn with_bit(&self, index: u32, set: bool, bits: u32, token: Token) -> Option<Self>;

    /// The bits of the value read in two's complement, as the signed storage of the same size,
    /// sign-extended from bit `bits - 1`, or zero for zero bits. Requires `bits` to be at most the
    /// bits of the storage and the value to have `bits` bits.
    fn cast_signed_bits(self, bits: u32, token: Token) -> Self::Complement;

    /// The value as the signed storage of the same size, or the largest value of `bits` bits in
    /// two's complement, `2^(bits - 1) - 1`, if the value is above it: a row of its own, so that a
    /// storage builds no bound that it drops, and stores the result where it can. Requires `bits`
    /// to be at most the bits of the storage and the value to have `bits` bits.
    fn saturating_cast_signed_bits(self, bits: u32, token: Token) -> Self::Complement;

    /// The non-zero storage of the value, or `None` if it is zero.
    fn to_nonzero(self, token: Token) -> Option<Self::NonZeroStorage>;

    /// The value of a non-zero storage.
    fn from_nonzero(nonzero: Self::NonZeroStorage, token: Token) -> Self;

    /// Calls `f` with the value of a non-zero storage, which an array lends and a native integer
    /// copies.
    fn with_nonzero<T>(
      nonzero: &Self::NonZeroStorage,
      f: impl FnOnce(&Self) -> T,
      token: Token,
    ) -> T;

    /// The integer of `nonzero`, the value of a non-zero integer: the integer itself where the
    /// non-zero storage is this storage, as an array lends it, and a copy for a native integer.
    fn lend_nonzero<B>(
      nonzero: &Uint<B, Self::NonZeroStorage>,
      token: Token,
    ) -> View<'_, Uint<B, Self>>;

    /// The non-zero storage of a value that is not zero, as every non-zero result of the
    /// arithmetic is. A zero is a bug of this crate: it panics in a native integer, whose non-zero
    /// storage cannot hold it, and fails a debug assertion in an array.
    fn known_nonzero(self, token: Token) -> Self::NonZeroStorage;

    /// The 64-bit word of the value at `index`, its bits from `64 * index` to `64 * index + 63`:
    /// zero beyond the storage.
    fn word(&self, index: usize, token: Token) -> u64;

    /// The integer whose 64-bit words, least significant first, `word` gives, modulo `2^bits`: the
    /// storage of the words, with every bit at index `bits` or above cleared. Requires `bits` to be
    /// at most the bits of the storage.
    fn from_words(word: impl Fn(usize) -> u64, bits: u32, token: Token) -> Self;

    /// The bits of the float of `format` nearest to the value, ties to even, or of the infinity
    /// above the range of the float, as `as` converts a native integer to a float.
    fn float_bits(&self, format: FloatFormat, token: Token) -> u64;

    /// The float of `parts` rounded to an integer with `mode`, as an integer of `bits` bits, or the
    /// error of a value outside their range: `NegOverflow` for a negative value but zero, and
    /// `PosOverflow` for a magnitude of more than `bits` bits. Requires `bits` to be at most the
    /// bits of the storage.
    fn from_float(
      parts: FloatParts,
      mode: RoundingMode,
      bits: u32,
      token: Token,
    ) -> Result<Self, FromFloatError>;

    // `ReprLimb` bounds the readers of limbs rather than being a supertrait, which would put the
    // bound `[L; N]: ReprLimb` into code bounded by `[L; N]: UintRepr` and keep the limb type of
    // the array there from normalizing to `L`.

    /// The integer in `limbs`, least significant limb first, unsigned, as an integer of `bits`
    /// bits, or `None` if it is `2^bits` or more, whatever the number of limbs. No limbs are zero.
    /// Requires `bits` to be at most the bits of the storage.
    fn checked_read_limbs(
      limbs: &[<Self as ReprLimb>::Limb],
      bits: u32,
      token: Token,
    ) -> Option<Self>
    where
      Self: ReprLimb;

    /// The integer in `limbs`, read as [`checked_read_limbs`](Self::checked_read_limbs) reads it,
    /// modulo `2^bits`. Requires `bits` to be at most the bits of the storage.
    fn wrapping_read_limbs(limbs: &[<Self as ReprLimb>::Limb], bits: u32, token: Token) -> Self
    where
      Self: ReprLimb;

    /// The integer in `limbs`, read as [`checked_read_limbs`](Self::checked_read_limbs) reads it,
    /// or `2^bits - 1` if it is above that. Requires `bits` to be at most the bits of the storage.
    fn saturating_read_limbs(limbs: &[<Self as ReprLimb>::Limb], bits: u32, token: Token) -> Self
    where
      Self: ReprLimb;
  }

  /// Seals [`IntRepr`](super::IntRepr) and gives the trait implementations of [`Int`](crate::Int)
  /// access to the integer core.
  ///
  /// As for [`UintCore`], each method takes a [`Token`]. They rely on the width invariant of
  /// `Int`: the representation, read as a two's complement integer, equals the value.
  pub trait IntCore: Sized {
    /// The storage of the absolute value of a value as an unsigned integer, which also holds the
    /// absolute value of the smallest value and the two's complement bits of every value:
    /// [`IntRepr::Unsigned`](super::IntRepr::Unsigned).
    type Magnitude;

    /// The storage of a value that is not zero: [`IntRepr::NonZero`](super::IntRepr::NonZero), as
    /// for [`UintCore::NonZeroStorage`].
    type NonZeroStorage;

    /// The storage that computes the values of this storage and holds one inline, as for
    /// [`UintCore::Inline`]. Its magnitude is an unsigned storage, so generic code can compute
    /// the magnitude of a lent value without a storage of its own.
    type Inline: IntCore<Magnitude: UintCore>;

    /// Lends the value, as its inline storage.
    fn inline(&self, token: Token) -> &Self::Inline;

    /// A storage of the value `value`, as for [`UintCore::from_inline`].
    fn from_inline(value: Self::Inline, token: Token) -> Self;

    /// Stores the value `value`, as for [`UintCore::set_inline`].
    fn set_inline(&mut self, value: Self::Inline, token: Token);

    /// Zero.
    fn zero(token: Token) -> Self;

    /// Compares the values of `self` and `other`.
    fn compare(&self, other: &Self, token: Token) -> Ordering;

    /// Compares the value with `rhs`, as integers: a negative value is below every `rhs`.
    fn compare_u128(&self, rhs: u128, token: Token) -> Ordering;

    /// Compares the value with `rhs`, as integers.
    fn compare_i128(&self, rhs: i128, token: Token) -> Ordering;

    /// Writes the lowest `ceil(bits / 64)` 64-bit words of the value in two's complement to
    /// `state`, least significant first, extended by copies of the sign beyond the storage.
    /// Requires the value to have `bits` bits.
    fn hash_limbs<H: Hasher>(&self, bits: u32, state: &mut H, token: Token);

    /// Writes the value in decimal, honoring the flags of `f` as integers do.
    fn fmt_decimal(&self, f: &mut fmt::Formatter<'_>, token: Token) -> fmt::Result;

    /// Writes the `bits` bits of the value, its two's complement, in the radix `2^shift`, binary
    /// for 1, octal for 3 and hexadecimal for 4, in uppercase if `upper`, honoring the flags of `f`
    /// as the radix formats of integers do. Requires `bits` to be at most the bits of the storage.
    fn fmt_pow2(
      &self,
      bits: u32,
      shift: u32,
      upper: bool,
      f: &mut fmt::Formatter<'_>,
      token: Token,
    ) -> fmt::Result;

    /// Writes the value in scientific notation, with `E` if `upper` and `e` otherwise, honoring
    /// the flags of `f` as integers do: a precision below the digits after the first rounds them
    /// half to even.
    fn fmt_exp(&self, upper: bool, f: &mut fmt::Formatter<'_>, token: Token) -> fmt::Result;

    /// The value of `src`, an optional `+` or `-` and the digits of `radix`, as an integer of
    /// `bits` bits in two's complement, or the error of the native `from_str_radix`. Requires
    /// `2 <= radix <= 36` and `bits` to be at most the bits of the storage.
    fn from_str_radix_bits(
      src: &[u8],
      radix: u32,
      bits: u32,
      token: Token,
    ) -> Result<Self, ParseIntError>;

    /// Writes the low `out.len()` bytes of the storage to `out`, least significant first, or most
    /// significant first if `big_endian`: the two's complement of the value, extended by copies of
    /// its sign. Requires `out.len()` to be at most the bytes of the storage.
    fn write_bytes(&self, out: &mut [u8], big_endian: bool, token: Token);

    /// The integer in `bytes`, least significant byte first, or most significant first if
    /// `big_endian`, in two's complement of `8 * bytes.len()` bits, as an integer of `bits` bits,
    /// or `None` if it is outside their range. Requires `bits` to be at most the bits of the
    /// storage.
    fn checked_read_bytes(bytes: &[u8], big_endian: bool, bits: u32, token: Token) -> Option<Self>;

    /// The integer in `bytes`, read as [`checked_read_bytes`](Self::checked_read_bytes) reads it,
    /// reduced modulo `2^bits` into the range of `bits` bits. Requires `bits` to be at most the
    /// bits of the storage.
    fn wrapping_read_bytes(bytes: &[u8], big_endian: bool, bits: u32, token: Token) -> Self;

    /// The integer in `bytes`, read as [`checked_read_bytes`](Self::checked_read_bytes) reads it,
    /// or the bound of the range of `bits` bits that it passes: the smallest value below the range,
    /// which only a negative integer is, and the largest above it. Requires `bits` to be at most
    /// the bits of the storage.
    fn saturating_read_bytes(bytes: &[u8], big_endian: bool, bits: u32, token: Token) -> Self;

    /// The byte of the storage at `index`, its bits from `8 * index` to `8 * index + 7`, which
    /// copy the sign from the width up. Requires `index` to be below the bytes of the storage.
    fn byte_at(&self, index: usize, token: Token) -> u8;

    /// The low `bits / 8` bytes of the value in reverse order, read in two's complement. Requires
    /// `bits` to be a multiple of 8 and at most the bits of the storage, and the value to have
    /// `bits` bits.
    fn swap_bytes_bits(&self, bits: u32, token: Token) -> Self;

    /// Returns `true` if the value is zero.
    fn is_zero(&self, token: Token) -> bool;

    /// Returns `true` if the value has `bits` bits, from `-2^(bits - 1)` to `2^(bits - 1) - 1`:
    /// every bit at index `bits - 1` or above equals the top bit of the storage, and for zero bits
    /// every bit is zero. Requires `bits` to be at most the bits of the storage.
    fn fits_bits(&self, bits: u32, token: Token) -> bool;

    /// Returns `true` if the value is negative.
    fn is_negative(&self, token: Token) -> bool;

    /// Returns `true` if the value is positive.
    fn is_positive(&self, token: Token) -> bool;

    /// `self + rhs` wrapped to `bits` bits, and whether the sum is outside the range of `bits`
    /// bits. Requires `bits` to be at most the bits of the storage and both values to have `bits`
    /// bits.
    fn overflowing_add_bits(&self, rhs: &Self, bits: u32, token: Token) -> (Self, bool);

    /// `self - rhs` wrapped to `bits` bits, and whether the difference is outside the range of
    /// `bits` bits. Requires `bits` to be at most the bits of the storage and both values to have
    /// `bits` bits.
    fn overflowing_sub_bits(&self, rhs: &Self, bits: u32, token: Token) -> (Self, bool);

    /// `self * rhs` wrapped to `bits` bits, and whether the product is outside the range of `bits`
    /// bits. Requires `bits` to be at most the bits of the storage and both values to have `bits`
    /// bits.
    fn overflowing_mul_bits(&self, rhs: &Self, bits: u32, token: Token) -> (Self, bool);

    /// `-self` wrapped to `bits` bits, and whether the negation is outside the range of `bits`
    /// bits, as it is only for the smallest value. Requires `bits` to be at most the bits of the
    /// storage and the value to have `bits` bits.
    fn overflowing_neg_bits(&self, bits: u32, token: Token) -> (Self, bool);

    /// Returns `true` if the value is one.
    fn is_one(&self, token: Token) -> bool;

    /// `-1`, `0` or `1` as the sign of the value.
    fn signum(&self, token: Token) -> Self;

    /// The absolute value as an unsigned integer, which has the bits of the value.
    fn unsigned_abs(&self, token: Token) -> Self::Magnitude;

    /// The quotient and the remainder of `self / rhs` rounded toward zero, the remainder with the
    /// sign of `self`, and whether the quotient is outside the range of `bits` bits, as it is only
    /// for the smallest value divided by -1, whose quotient wraps to the smallest value; or `None`
    /// if `rhs` is zero. Requires `bits` to be at most the bits of the storage and both values to
    /// have `bits` bits.
    fn div_rem_bits(&self, rhs: &Self, bits: u32, token: Token) -> Option<(Self, Self, bool)>;

    /// The Euclidean quotient and remainder of `self / rhs`, the remainder never negative, and
    /// whether the quotient is outside the range of `bits` bits, as for
    /// [`div_rem_bits`](Self::div_rem_bits).
    fn div_rem_euclid_bits(
      &self,
      rhs: &Self,
      bits: u32,
      token: Token,
    ) -> Option<(Self, Self, bool)>;

    /// `self / rhs` rounded down, and whether it is outside the range of `bits` bits, or `None` if
    /// `rhs` is zero, as for [`div_rem_bits`](Self::div_rem_bits).
    fn div_floor_bits(&self, rhs: &Self, bits: u32, token: Token) -> Option<(Self, bool)>;

    /// `self / rhs` rounded up, and whether it is outside the range of `bits` bits, or `None` if
    /// `rhs` is zero, as for [`div_rem_bits`](Self::div_rem_bits).
    fn div_ceil_bits(&self, rhs: &Self, bits: u32, token: Token) -> Option<(Self, bool)>;

    /// For a positive `rhs` the smallest multiple of `rhs` at least `self`, and for a negative
    /// `rhs` the largest at most `self`, wrapped to `bits` bits, and whether it is outside the
    /// range of `bits` bits; or `None` if `rhs` is zero. Requires `bits` to be at most the bits of
    /// the storage and both values to have `bits` bits.
    fn next_multiple_of_bits(&self, rhs: &Self, bits: u32, token: Token) -> Option<(Self, bool)>;

    /// `self / rhs` if `rhs` divides `self` and the quotient is within the range of `bits` bits,
    /// or `None` otherwise. Requires `bits` to be at most the bits of the storage and both values
    /// to have `bits` bits.
    fn checked_div_exact_bits(&self, rhs: &Self, bits: u32, token: Token) -> Option<Self>;

    /// `self^exp` wrapped to `bits` bits, and whether it is outside the range of `bits` bits.
    /// Requires `bits` to be at most the bits of the storage and the value to have `bits` bits.
    fn overflowing_pow_bits(&self, exp: u32, bits: u32, token: Token) -> (Self, bool);

    /// The square root of the value, rounded down, or `None` if the value is negative.
    fn checked_isqrt(&self, token: Token) -> Option<Self>;

    /// The `n`th root of the value, rounded toward zero, or `None` if `n` is zero or the value is
    /// negative and `n` even.
    fn checked_nth_root(&self, n: u32, token: Token) -> Option<Self>;

    /// The logarithm of the value to the base `base`, rounded down, or `None` if the value is not
    /// positive or `base` is below 2.
    fn checked_ilog(&self, base: &Self, token: Token) -> Option<u32>;

    /// The base-2 logarithm of the value, rounded down, or `None` if the value is not positive.
    fn checked_ilog2(&self, token: Token) -> Option<u32>;

    /// The base-10 logarithm of the value, rounded down, or `None` if the value is not positive.
    fn checked_ilog10(&self, token: Token) -> Option<u32>;

    /// `(self + rhs) / 2` rounded toward zero.
    fn midpoint(&self, rhs: &Self, token: Token) -> Self;

    /// `|self - rhs|` as an unsigned integer, which has the bits of the values.
    fn abs_diff(&self, rhs: &Self, token: Token) -> Self::Magnitude;

    /// `self & rhs`, which keeps the copies of the sign as the signs combine.
    fn and(&self, rhs: &Self, token: Token) -> Self;

    /// `self | rhs`, which keeps the copies of the sign as the signs combine.
    fn or(&self, rhs: &Self, token: Token) -> Self;

    /// `self ^ rhs`, which keeps the copies of the sign as the signs combine.
    fn xor(&self, rhs: &Self, token: Token) -> Self;

    /// `!self`: every bit flipped, or zero for zero bits. Requires the value to have `bits` bits.
    fn not_bits(&self, bits: u32, token: Token) -> Self;

    /// The number of ones of the value in `bits` bits, without the copies of the sign above them.
    /// Requires `bits` to be at most the bits of the storage.
    fn count_ones_bits(&self, bits: u32, token: Token) -> u32;

    /// The number of zeros of the value from bit `bits - 1` down to its highest one: none for a
    /// negative value, and `bits` for zero. Requires `bits` to be at most the bits of the storage
    /// and the value to have `bits` bits.
    fn leading_zeros_bits(&self, bits: u32, token: Token) -> u32;

    /// The number of zeros of the value below its lowest one, or `bits` for zero. Requires the
    /// value to have `bits` bits.
    fn trailing_zeros_bits(&self, bits: u32, token: Token) -> u32;

    /// The number of ones of the value from bit `bits - 1` down: none for a value at least zero.
    /// Requires `bits` to be at most the bits of the storage and the value to have `bits` bits.
    fn leading_ones_bits(&self, bits: u32, token: Token) -> u32;

    /// The number of ones of the value from bit 0 up: `bits` for -1. Requires `bits` to be at most
    /// the bits of the storage and the value to have `bits` bits.
    fn trailing_ones_bits(&self, bits: u32, token: Token) -> u32;

    /// The index of the highest one of the value in `bits` bits: `bits - 1` for a negative value,
    /// or `None` for zero. Requires the value to have `bits` bits.
    fn highest_one_bits(&self, bits: u32, token: Token) -> Option<u32>;

    /// The index of the lowest one of the value, or `None` for zero.
    fn lowest_one(&self, token: Token) -> Option<u32>;

    /// The value with only its highest one in `bits` bits: the smallest value for a negative
    /// value, or zero for zero. Requires `bits` to be at most the bits of the storage and the
    /// value to have `bits` bits.
    fn isolate_highest_one_bits(&self, bits: u32, token: Token) -> Self;

    /// The value with only its lowest one, sign-extended from `bits` bits: the smallest value for
    /// the smallest value, or zero for zero. Requires `bits` to be at most the bits of the storage
    /// and the value to have `bits` bits.
    fn isolate_lowest_one_bits(&self, bits: u32, token: Token) -> Self;

    /// The `bits` bits of the value in reverse order, read in two's complement. Requires `bits` to
    /// be at most the bits of the storage and the value to have `bits` bits.
    fn reverse_bits_bits(&self, bits: u32, token: Token) -> Self;

    /// The `bits` bits of the value rotated left by `n` places, read in two's complement. Requires
    /// `bits` to be at most the bits of the storage and the value to have `bits` bits.
    fn rotate_left_bits(&self, n: u32, bits: u32, token: Token) -> Self;

    /// The `bits` bits of the value rotated right by `n` places, read in two's complement.
    /// Requires `bits` to be at most the bits of the storage and the value to have `bits` bits.
    fn rotate_right_bits(&self, n: u32, bits: u32, token: Token) -> Self;

    /// `self << (rhs % bits)` wrapped to `bits` bits, and whether `rhs >= bits`, as it always is
    /// at zero bits. Requires `bits` to be at most the bits of the storage and the value to have
    /// `bits` bits.
    fn overflowing_shl_bits(&self, rhs: u32, bits: u32, token: Token) -> (Self, bool);

    /// `self >> (rhs % bits)`, an arithmetic shift, and whether `rhs >= bits`, as it always is at
    /// zero bits. Requires the value to have `bits` bits.
    fn overflowing_shr_bits(&self, rhs: u32, bits: u32, token: Token) -> (Self, bool);

    /// `self << rhs` wrapped to `bits` bits, which is zero for `rhs >= bits`. Requires `bits` to
    /// be at most the bits of the storage.
    fn unbounded_shl_bits(&self, rhs: u32, bits: u32, token: Token) -> Self;

    /// `self >> rhs`, an arithmetic shift, which is 0 or -1 for `rhs >= bits`. Requires the value
    /// to have `bits` bits.
    fn unbounded_shr_bits(&self, rhs: u32, bits: u32, token: Token) -> Self;

    /// `self << rhs` if every bit that the shift moves out of `bits` bits, and the new bit
    /// `bits - 1`, equal the sign, or `None` otherwise. Requires `bits` to be at most the bits of
    /// the storage and the value to have `bits` bits.
    fn shl_exact_bits(&self, rhs: u32, bits: u32, token: Token) -> Option<Self>;

    /// `self >> rhs`, an arithmetic shift, if `rhs < bits` and the shift moves no one out, or
    /// `None` otherwise. Requires the value to have `bits` bits.
    fn shr_exact_bits(&self, rhs: u32, bits: u32, token: Token) -> Option<Self>;

    /// The bit at `index` of the value, or `false` for `index >= bits`. Requires `bits` to be at
    /// most the bits of the storage.
    fn bit(&self, index: u32, bits: u32, token: Token) -> bool;

    /// The value with the bit at `index` set if `set`, and cleared otherwise, sign-extended from
    /// `bits` bits, or `None` for `index >= bits`. Requires `bits` to be at most the bits of the
    /// storage and the value to have `bits` bits.
    fn with_bit(&self, index: u32, set: bool, bits: u32, token: Token) -> Option<Self>;

    /// The `bits` bits of the value, its two's complement, as the unsigned storage of the same
    /// size. Requires `bits` to be at most the bits of the storage.
    fn cast_unsigned_bits(self, bits: u32, token: Token) -> Self::Magnitude;

    /// The value as the unsigned storage of the same size, or zero if the value is negative: a
    /// row of its own, as [`UintCore::saturating_cast_signed_bits`] is. Requires `bits` to be at
    /// most the bits of the storage and the value to have `bits` bits.
    fn saturating_cast_unsigned_bits(self, bits: u32, token: Token) -> Self::Magnitude;

    /// The non-zero storage of the value, or `None` if it is zero.
    fn to_nonzero(self, token: Token) -> Option<Self::NonZeroStorage>;

    /// The value of a non-zero storage.
    fn from_nonzero(nonzero: Self::NonZeroStorage, token: Token) -> Self;

    /// Calls `f` with the value of a non-zero storage, which an array lends and a native integer
    /// copies.
    fn with_nonzero<T>(
      nonzero: &Self::NonZeroStorage,
      f: impl FnOnce(&Self) -> T,
      token: Token,
    ) -> T;

    /// The integer of `nonzero`, the value of a non-zero integer, as for
    /// [`UintCore::lend_nonzero`].
    fn lend_nonzero<B>(
      nonzero: &Int<B, Self::NonZeroStorage>,
      token: Token,
    ) -> View<'_, Int<B, Self>>;

    /// The non-zero storage of a value that is not zero, as for
    /// [`UintCore::known_nonzero`].
    fn known_nonzero(self, token: Token) -> Self::NonZeroStorage;

    /// The 64-bit word of the value at `index` in two's complement, its bits from `64 * index` to
    /// `64 * index + 63`: copies of the sign beyond the storage.
    fn word(&self, index: usize, token: Token) -> u64;

    /// The integer whose 64-bit words in two's complement, least significant first, `word` gives,
    /// reduced modulo `2^bits` into the range of `bits` bits: the storage of the words, with every
    /// bit at index `bits - 1` or above set to bit `bits - 1`, or zero for zero bits. Requires
    /// `bits` to be at most the bits of the storage.
    fn from_words(word: impl Fn(usize) -> u64, bits: u32, token: Token) -> Self;

    /// The bits of the float of `format` nearest to the value, ties to even, or of the infinity of
    /// its sign beyond the range of the float, as `as` converts a native integer to a float: zero
    /// is `+0.0`.
    fn float_bits(&self, format: FloatFormat, token: Token) -> u64;

    /// The float of `parts` rounded to an integer with `mode`, as an integer of `bits` bits in
    /// two's complement, or the error of a value outside their range: `NegOverflow` below
    /// `-2^(bits - 1)`, and `PosOverflow` above `2^(bits - 1) - 1`. Requires `bits` to be at most
    /// the bits of the storage.
    fn from_float(
      parts: FloatParts,
      mode: RoundingMode,
      bits: u32,
      token: Token,
    ) -> Result<Self, FromFloatError>;

    // `ReprLimb` bounds the readers of limbs, as for `UintCore`.

    /// The integer in `limbs`, least significant limb first, in two's complement of the bits of
    /// its limbs, as an integer of `bits` bits, or `None` if it is outside their range, whatever
    /// the number of limbs. No limbs are zero. Requires `bits` to be at most the bits of the
    /// storage.
    fn checked_read_limbs(
      limbs: &[<Self as ReprLimb>::Limb],
      bits: u32,
      token: Token,
    ) -> Option<Self>
    where
      Self: ReprLimb;

    /// The integer in `limbs`, read as [`checked_read_limbs`](Self::checked_read_limbs) reads it,
    /// reduced modulo `2^bits` into the range of `bits` bits. Requires `bits` to be at most the
    /// bits of the storage.
    fn wrapping_read_limbs(limbs: &[<Self as ReprLimb>::Limb], bits: u32, token: Token) -> Self
    where
      Self: ReprLimb;

    /// The integer in `limbs`, read as [`checked_read_limbs`](Self::checked_read_limbs) reads it,
    /// or the bound of the range of `bits` bits that it passes: the smallest value below the range,
    /// which only a negative integer is, and the largest above it. Requires `bits` to be at most
    /// the bits of the storage.
    fn saturating_read_limbs(limbs: &[<Self as ReprLimb>::Limb], bits: u32, token: Token) -> Self
    where
      Self: ReprLimb;
  }
}

/// Marks that `Repr` holds every value of `Self` bits.
///
/// `Self` is a [`typenum`] number from 0 to `2^32 - 1`, so that the constant `BITS`, a `u32`, holds
/// it, and is in canonical form: `UTerm`, or a `UInt` whose most significant bit is `B1`, as every
/// `typenum` constant and operator result is. The native integers `u8` and `i8` hold up to 8 bits,
/// `u16` and `i16` up to 16, `u32` and `i32` up to 32, `u64` and `i64` up to 64, and `u128` and
/// `i128` up to 128. The arrays `[u8; N]`, `[u16; N]`, `[u32; N]`, `[u64; N]` and `[u128; N]` hold
/// up to `8 * N`, `16 * N`, `32 * N`, `64 * N` and `128 * N` bits, for every `N` for which
/// `typenum` implements `ToUInt` for `Const<N>`: each `N` up to 1024, and larger ones such as the
/// powers of 2 and of 10. An array of any other length, such as `[u8; 1025]`, holds no width, so
/// the integers stored in it have no constants, constructors or `Default`, and no value of them can
/// be made. A storage may have more bits than needed, as `u16` has for 9 bits and `[u64; 4]` for
/// 200. [`Uint`](crate::Uint) and [`Int`](crate::Int) require this bound on their constants,
/// constructors and `Default`, not on the types themselves, so generic code adds it where it uses
/// them:
///
/// ```
/// use numo::{FitsBits, Uint};
///
/// fn largest<B: FitsBits<[u64; 2]>>() -> Uint<B, [u64; 2]> {
///   Uint::<B, [u64; 2]>::MAX
/// }
///
/// assert_eq!(largest::<numo::typenum::U100>().to_string(), "1267650600228229401496703205375");
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
// The compiler shows this text to a generic caller whose bounds do not prove the relation; for a
// concrete width, it shows the text of the comparison instead.
#[diagnostic::on_unimplemented(
  message = "the storage cannot hold `Bits` bits",
  label = "`Bits` is not known to fit in the storage",
  note = "add a `FitsBits<Repr>` bound on `Bits`"
)]
pub trait FitsBits<Repr>: Unsigned + Canonical + BitsOf<Repr> {}

impl<B, R> FitsBits<R> for B where B: Unsigned + Canonical + BitsOf<R> {}

/// Marks that `Self` bits fill `Repr` exactly, so that every bit pattern of `Repr` is a value.
///
/// 8, 16, 32, 64 and 128 bits fill the native integers of that width, and `8 * N`, `16 * N`,
/// `32 * N`, `64 * N` and `128 * N` bits fill the arrays `[u8; N]`, `[u16; N]`, `[u32; N]`,
/// `[u64; N]` and `[u128; N]`. The constructor `from_limbs` of
/// [`Uint`](crate::Uint::from_limbs) and [`Int`](crate::Int::from_limbs) requires this bound and
/// never fails; `checked_from_limbs` takes the limbs of any width that fits and checks the value.
/// This trait is sealed and cannot be implemented outside this crate.
#[diagnostic::on_unimplemented(
  message = "`Bits` must fill the storage exactly",
  label = "`Bits` is not known to fill the storage",
  note = "add a `FillsRepr<Repr>` bound on `Bits`, or use `checked_from_limbs`, which checks the value"
)]
pub trait FillsRepr<Repr>: FitsBits<Repr> + BitsFill<Repr> {}

impl<B, R> FillsRepr<R> for B where B: FitsBits<R> + BitsFill<R> {}

/// Marks a width of whole bytes: `Self`, a [`typenum`] number, is a multiple of 8, zero included.
///
/// The byte order of [`Uint`](crate::Uint) and [`Int`](crate::Int), `swap_bytes`, `to_be`,
/// `to_le`, `from_be` and `from_le`, and their conversions from a byte array, `from_le_bytes` and
/// the others, exist at such widths, where every array of bytes holds a value; the conversions
/// `checked_from_le_bytes` and the others, and those from a slice, read any width. A `typenum`
/// number lists its bits, and the three lowest of a multiple of 8 are zeros, which is what the
/// implementations match. This trait is sealed and cannot be implemented outside this crate.
#[diagnostic::on_unimplemented(
  message = "`Bits` is not a whole number of bytes",
  label = "the width is not a multiple of 8",
  note = "the byte order and the conversions from a byte array exist for widths of whole bytes; `checked_from_le_bytes` and `checked_from_le_slice` read any width"
)]
pub trait WholeBytes: Unsigned {}

impl WholeBytes for UTerm {}

impl<U: Unsigned> WholeBytes for UInt<UInt<UInt<U, B0>, B0>, B0> {}

/// A storage of [`Uint`](crate::Uint): a native unsigned integer, `u8`, `u16`, `u32`, `u64` or
/// `u128`, holds the value itself, and the array `[u8; N]`, `[u16; N]`, `[u32; N]`, `[u64; N]` or
/// `[u128; N]` holds it in `N` limbs of 8, 16, 32, 64 or 128 bits, least significant first. With
/// the `std` or the `alloc` feature, such an array in a `Box`, an `Rc` or an `Arc`, as in
/// `Box<[L; N]>`, holds it in its allocation, and so does such an array in a `triomphe::Arc` with
/// the `triomphe` feature (see [Heap storages](crate::Uint#heap-storages)).
///
/// `Uint<Bits, Repr>` implements `Ord`, `Hash`, `Display`, `Debug` and `Default`, the arithmetic
/// traits [`OverflowingOps`](crate::OverflowingOps), [`CheckedOps`](crate::CheckedOps),
/// [`WrappingOps`](crate::WrappingOps), [`SaturatingOps`](crate::SaturatingOps),
/// [`StrictOps`](crate::StrictOps) and [`UncheckedOps`](crate::UncheckedOps), the math traits
/// [`IntegerMath`](crate::IntegerMath) and [`UnsignedMath`](crate::UnsignedMath), the bit traits
/// [`IntegerBits`](crate::IntegerBits) and [`UnsignedBits`](crate::UnsignedBits),
/// [`UnsignedFixedInteger`](crate::UnsignedFixedInteger), the operators `+`, `-`, `*`, `/`, `%`,
/// `&`, `|` and `^` in every form of values and references, as in `a + &b`, `!`, `<<` and `>>`,
/// `Sum`, and from one bit `Product`, for every `Repr` that implements this trait, and where `Repr`
/// is `Clone`, as every storage of this crate is, [`FixedInteger`](crate::FixedInteger) and, from
/// one bit, [`IntegerOne`](crate::IntegerOne). So does [`IntegerNonZero`](crate::IntegerNonZero),
/// whose non-zero integer is the [`NonZeroUint`](crate::NonZeroUint) over the
/// [`NonZero`](Self::NonZero) of `Repr`: `Uint<Bits, Repr>` divides by it with `/` and `%`, which
/// cannot panic, and combines with it by `|`, in every form. This trait is sealed and cannot be
/// implemented outside this crate.
#[diagnostic::on_unimplemented(
  message = "`{Self}` is not a storage of `Uint`",
  note = "`Uint` stores an unsigned native integer, `u8` to `u128`, an array `[u8; N]` to `[u128; N]`, or such an array in a `Box`, an `Rc` or an `Arc`, with the `std` or the `alloc` feature, or in a `triomphe::Arc`, with the `triomphe` feature"
)]
pub trait UintRepr:
  Eq + UintCore<Complement = <Self as UintRepr>::Signed, NonZeroStorage = <Self as UintRepr>::NonZero>
{
  /// The storage of [`Int`](crate::Int) of the same size: `i8` for `u8`, `i16` for `u16`, and so
  /// on, and the same storage for an array of limbs and for a heap storage. It holds the bits of a
  /// value read in two's complement, as [`cast_signed`](crate::UnsignedBits::cast_signed) reads
  /// them, and its [`Unsigned`](IntRepr::Unsigned) is this storage again, so that generic code over
  /// the width casts `Uint<B>` to `Int<B>` and back.
  type Signed: IntRepr<Unsigned = Self>;

  /// The storage of the values of this storage that are not zero: `core::num::NonZero<u8>` for
  /// `u8`, and so on, and the same storage for an array of limbs and for a heap storage. Its
  /// [`Integer`](NonZeroUintRepr::Integer) is this storage again, and its
  /// [`Signed`](NonZeroUintRepr::Signed), the storage of a cast, is the
  /// [`NonZero`](IntRepr::NonZero) of [`Signed`](Self::Signed).
  type NonZero: NonZeroUintRepr<Integer = Self, Signed = <Self::Signed as IntRepr>::NonZero>;
}

/// A storage of [`Int`](crate::Int): a native signed integer, `i8`, `i16`, `i32`, `i64` or `i128`,
/// holds the value itself, and the array `[u8; N]`, `[u16; N]`, `[u32; N]`, `[u64; N]` or
/// `[u128; N]` holds it in `N` limbs of 8, 16, 32, 64 or 128 bits, least significant first, in
/// two's complement. With the `std` or the `alloc` feature, such an array in a `Box`, an `Rc` or
/// an `Arc`, as in `Box<[L; N]>`, holds it in its allocation, and so does such an array in a
/// `triomphe::Arc` with the `triomphe` feature (see [Heap storages](crate::Uint#heap-storages)).
///
/// `Int<Bits, Repr>` implements `Ord`, `Hash`, `Display`, `Debug` and `Default`, the arithmetic
/// traits [`OverflowingOps`](crate::OverflowingOps), [`CheckedOps`](crate::CheckedOps),
/// [`WrappingOps`](crate::WrappingOps), [`SaturatingOps`](crate::SaturatingOps),
/// [`StrictOps`](crate::StrictOps) and [`UncheckedOps`](crate::UncheckedOps), the math traits
/// [`IntegerMath`](crate::IntegerMath) and [`SignedMath`](crate::SignedMath), the bit traits
/// [`IntegerBits`](crate::IntegerBits) and [`SignedBits`](crate::SignedBits), the operators `+`,
/// `-`, `*`, `/`, `%`, `&`, `|` and `^` in every form of values and references, as in `a + &b`,
/// the negation `-`, `!`, `<<` and `>>`, `Sum`, and from two bits `Product`, for every `Repr` that
/// implements this trait, and where `Repr` is `Clone`, as every storage of this crate is,
/// [`FixedInteger`](crate::FixedInteger), [`SignedFixedInteger`](crate::SignedFixedInteger) and,
/// from two bits, [`IntegerOne`](crate::IntegerOne). So does
/// [`IntegerNonZero`](crate::IntegerNonZero), whose non-zero integer is the
/// [`NonZeroInt`](crate::NonZeroInt) over the [`NonZero`](Self::NonZero) of `Repr`, with which
/// `Int<Bits, Repr>` combines by `|` in every form. This trait is sealed and cannot be implemented
/// outside this crate.
#[diagnostic::on_unimplemented(
  message = "`{Self}` is not a storage of `Int`",
  note = "`Int` stores a signed native integer, `i8` to `i128`, an array `[u8; N]` to `[u128; N]` of unsigned limbs in two's complement, or such an array in a `Box`, an `Rc` or an `Arc`, with the `std` or the `alloc` feature, or in a `triomphe::Arc`, with the `triomphe` feature"
)]
pub trait IntRepr:
  Eq + IntCore<Magnitude = <Self as IntRepr>::Unsigned, NonZeroStorage = <Self as IntRepr>::NonZero>
{
  /// The storage of [`Uint`](crate::Uint) of the same size: `u8` for `i8`, `u16` for `i16`, and so
  /// on, and the same storage for an array of limbs and for a heap storage.
  type Unsigned: UintRepr;

  /// The storage of the values of this storage that are not zero: `core::num::NonZero<i8>` for
  /// `i8`, and so on, and the same storage for an array of limbs and for a heap storage. Its
  /// [`Integer`](NonZeroIntRepr::Integer) is this storage again, and its
  /// [`Unsigned`](NonZeroIntRepr::Unsigned), the storage of a cast, is the
  /// [`NonZero`](UintRepr::NonZero) of [`Unsigned`](Self::Unsigned).
  type NonZero: NonZeroIntRepr<Integer = Self, Unsigned = <Self::Unsigned as UintRepr>::NonZero>;
}

/// A storage of the non-zero unsigned integers: `core::num::NonZero<u8>`, `NonZero<u16>`,
/// `NonZero<u32>`, `NonZero<u64>` or `NonZero<u128>`, which holds a value that is not zero, or an
/// array `[u8; N]`, `[u16; N]`, `[u32; N]`, `[u64; N]` or `[u128; N]`, which holds it in `N` limbs,
/// least significant first, as for [`Uint`](crate::Uint), and with the `std` or the `alloc`
/// feature such an array in a `Box`, an `Rc` or an `Arc`, or with the `triomphe` feature in a
/// `triomphe::Arc`.
///
/// Each is the [`NonZero`](UintRepr::NonZero) of a storage of `Uint`, its
/// [`Integer`](Self::Integer). `NonZeroUint<Bits, Repr>` implements `Clone`, `Eq`, `Ord`, `Hash`,
/// `Display` and `Debug`, and `Copy` where `Repr` is `Copy`, for every `Repr` that implements this
/// trait, and with the width relation of its integer, `Bits: FitsBits<Repr::Integer>`,
/// [`FixedNonZero`](crate::FixedNonZero) and [`UnsignedNonZero`](crate::UnsignedNonZero), from one
/// bit [`NonZeroLimits`](crate::NonZeroLimits), and `|` in every form, with itself and with its
/// integer either way. `From` converts it to its integer. This trait is sealed and cannot be
/// implemented outside this crate.
#[diagnostic::on_unimplemented(
  message = "`{Self}` is not a storage of `NonZeroUint`",
  note = "`NonZeroUint` stores `core::num::NonZero<u8>` to `NonZero<u128>`, an array `[u8; N]` to `[u128; N]`, or such an array in a `Box`, an `Rc` or an `Arc`, with the `std` or the `alloc` feature, or in a `triomphe::Arc`, with the `triomphe` feature"
)]
pub trait NonZeroUintRepr: Eq + Clone + NonZeroRepr {
  /// The storage of the integer that may be zero: `u8` for `core::num::NonZero<u8>`, and so on,
  /// and the storage itself for an array of limbs and for a heap storage. Its
  /// [`NonZero`](UintRepr::NonZero) is this storage again.
  type Integer: UintRepr<NonZero = Self>;

  /// The non-zero storage of the signed integer of the same size: `core::num::NonZero<i8>` for
  /// `NonZero<u8>`, and so on, and the storage itself for an array of limbs and for a heap storage.
  /// Its [`Integer`](NonZeroIntRepr::Integer) is the [`Signed`](UintRepr::Signed) of this
  /// storage's `Integer`, which holds the bits of a cast, and it is the
  /// [`NonZero`](IntRepr::NonZero) of that storage, as [`UintRepr::NonZero`] states.
  type Signed: NonZeroIntRepr<Integer = <Self::Integer as UintRepr>::Signed>;
}

/// A storage of the non-zero signed integers: `core::num::NonZero<i8>`, `NonZero<i16>`,
/// `NonZero<i32>`, `NonZero<i64>` or `NonZero<i128>`, which holds a value that is not zero, or an
/// array `[u8; N]`, `[u16; N]`, `[u32; N]`, `[u64; N]` or `[u128; N]`, which holds it in `N` limbs,
/// least significant first, in two's complement, as for [`Int`](crate::Int), and with the `std` or
/// the `alloc` feature such an array in a `Box`, an `Rc` or an `Arc`, or with the `triomphe`
/// feature in a `triomphe::Arc`.
///
/// Each is the [`NonZero`](IntRepr::NonZero) of a storage of `Int`, its
/// [`Integer`](Self::Integer). `NonZeroInt<Bits, Repr>` implements the traits of
/// [`NonZeroUintRepr`] for `NonZeroUint`, with [`SignedNonZero`](crate::SignedNonZero) for
/// [`UnsignedNonZero`](crate::UnsignedNonZero), and the negation `-` on a value and on a reference.
/// This trait is sealed and cannot be implemented outside this crate.
#[diagnostic::on_unimplemented(
  message = "`{Self}` is not a storage of `NonZeroInt`",
  note = "`NonZeroInt` stores `core::num::NonZero<i8>` to `NonZero<i128>`, an array `[u8; N]` to `[u128; N]` of unsigned limbs in two's complement, or such an array in a `Box`, an `Rc` or an `Arc`, with the `std` or the `alloc` feature, or in a `triomphe::Arc`, with the `triomphe` feature"
)]
pub trait NonZeroIntRepr: Eq + Clone + NonZeroRepr {
  /// The storage of the integer that may be zero: `i8` for `core::num::NonZero<i8>`, and so on,
  /// and the storage itself for an array of limbs and for a heap storage. Its
  /// [`NonZero`](IntRepr::NonZero) is this storage again.
  type Integer: IntRepr<NonZero = Self>;

  /// The non-zero storage of the unsigned integer of the same size: `core::num::NonZero<u8>` for
  /// `NonZero<i8>`, and so on, and the storage itself for an array of limbs and for a heap storage.
  /// Its [`Integer`](NonZeroUintRepr::Integer) is the [`Unsigned`](IntRepr::Unsigned) of this
  /// storage's `Integer`, which holds the bits of a cast, and it is the
  /// [`NonZero`](UintRepr::NonZero) of that storage, as [`IntRepr::NonZero`] states.
  type Unsigned: NonZeroUintRepr<Integer = <Self::Integer as IntRepr>::Unsigned>;
}

/// A storage of [`Uint`](crate::Uint) that holds the value in place, so that the integers have the
/// compound assignment operators `+=`, `-=`, `*=`, `/=`, `%=`, `&=`, `|=`, `^=`, `<<=` and `>>=`.
///
/// The native unsigned integers, the arrays of limbs and the boxes of them implement it, and the
/// shared heap storages, an `Rc`, an `Arc` or a `triomphe::Arc` of an array, do not. With it,
/// `Uint` also has `/=` and `%=` by a [`NonZeroUint`](crate::NonZeroUint), the `NonZeroUint` over
/// its [`NonZero`](UintRepr::NonZero) has `|=`, and where the storage is `Clone`, as every storage
/// of this crate is, `Uint` implements [`IntegerAssign`](crate::IntegerAssign). This trait is
/// sealed and cannot be implemented outside this crate.
pub trait UintReprMut: UintRepr + MutableRepr {}

/// A storage of [`Int`](crate::Int) that holds the value in place, so that the integers have the
/// compound assignment operators `+=`, `-=`, `*=`, `/=`, `%=`, `&=`, `|=`, `^=`, `<<=` and `>>=`.
///
/// The native signed integers, the arrays of limbs and the boxes of them implement it, and the
/// shared heap storages, an `Rc`, an `Arc` or a `triomphe::Arc` of an array, do not. With it, the
/// [`NonZeroInt`](crate::NonZeroInt) over its [`NonZero`](IntRepr::NonZero) has `|=`, and where
/// the storage is `Clone`, as every storage of this crate is, `Int` implements
/// [`IntegerAssign`](crate::IntegerAssign). This trait is sealed and cannot be implemented outside
/// this crate.
pub trait IntReprMut: IntRepr + MutableRepr {}

/// A storage of [`Uint`](crate::Uint) that holds the value inline, so that the integer is `Copy`: a
/// native unsigned integer, `u8` to `u128`, or an array `[u8; N]` to `[u128; N]`. A heap storage
/// does not implement it.
///
/// `Uint<Bits, Repr>` has the constants `ZERO`, `MIN` and `MAX` and implements
/// [`ConstFixedInteger`](crate::ConstFixedInteger), and from one bit it has the constant `ONE` and
/// implements [`ConstIntegerOne`](crate::ConstIntegerOne), for every `Repr` that implements this
/// trait; the traits that need no constant are those of [`UintRepr`] and [`UintReprMut`]. The
/// [`NonZeroUint`](crate::NonZeroUint) over its [`NonZero`](UintRepr::NonZero) has, from one bit,
/// the constants `MIN` and `MAX`, with [`ConstNonZeroLimits`](crate::ConstNonZeroLimits) where that
/// storage is `Copy`. The default storages of [`BitWidth`] are inline, and their non-zero storages
/// are `Copy`. This trait is sealed and cannot be implemented outside this crate.
pub trait InlineUintRepr: UintReprMut + Copy + InlineUintConsts {}

/// A storage of [`Int`](crate::Int) that holds the value inline, so that the integer is `Copy`: a
/// native signed integer, `i8` to `i128`, or an array `[u8; N]` to `[u128; N]`. A heap storage
/// does not implement it.
///
/// `Int<Bits, Repr>` has the constants `ZERO`, `MIN` and `MAX` and implements
/// [`ConstFixedInteger`](crate::ConstFixedInteger), and from two bits it has the constant `ONE` and
/// implements [`ConstIntegerOne`](crate::ConstIntegerOne), for every `Repr` that implements this
/// trait; the traits that need no constant are those of [`IntRepr`] and [`IntReprMut`]. The
/// [`NonZeroInt`](crate::NonZeroInt) over its [`NonZero`](IntRepr::NonZero) has, from one bit, the
/// constants `MIN` and `MAX`, with [`ConstNonZeroLimits`](crate::ConstNonZeroLimits) where that
/// storage is `Copy`. The default storages of [`BitWidth`] are inline, and their non-zero storages
/// are `Copy`. This trait is sealed and cannot be implemented outside this crate.
pub trait InlineIntRepr: IntReprMut + Copy + InlineIntConsts {}

/// Implements [`UintRepr`], [`IntRepr`], [`UintReprMut`], [`IntReprMut`], [`InlineUintRepr`],
/// [`InlineIntRepr`], [`NonZeroUintRepr`] and [`NonZeroIntRepr`] for the arrays of each limb type,
/// and their width relations, written `limb: BitsLog2, lower, upper`, where `BitsLog2` is the
/// base-2 logarithm of the number of bits of a limb, so that `[limb; N]` holds `N << BitsLog2`
/// bits, and `lower` and `upper` are the modules of its limb layers.
// Each limb type has width impls of its own. One impl over every array, bounded by a trait of the
// limb types, would compute with the logarithm of that trait, which for an array of a type that is
// not a limb reaches the recursive `Shl` impls of `typenum` and overflows the trait solver, instead
// of failing with the diagnostic of the storage traits. The logarithm is written out, because rustc
// does not match a where-clause that names it through the limb type against its normalized form.
macro_rules! array_storage {
  ($($limb:ident: $bits_log2:ident, $lower:ident, $upper:ident);+ $(;)?) => {$(
    // The bound below 2^32 is a comparison of types, because `typenum`'s `U32` and `U64` wrap: for
    // `64 * 2^58` bits, both are zero.
    impl<B, const N: usize> BitsOf<[$limb; N]> for B
    where
      B: IsLessOrEqual<Shleft<U<N>, $bits_log2>> + IsLess<U4294967296>,
      Const<N>: ToUInt,
      U<N>: Shl<$bits_log2>,
      LeEq<B, Shleft<U<N>, $bits_log2>>: BitsFit,
      Le<B, U4294967296>: BitsBelow2Pow32,
    {
    }

    // `typenum`'s alias for `IsEqual::Output` is named `Eq`, as the trait of `core` is.
    impl<B, const N: usize> BitsFill<[$limb; N]> for B
    where
      B: IsEqual<Shleft<U<N>, $bits_log2>>,
      Const<N>: ToUInt,
      U<N>: Shl<$bits_log2>,
      <B as IsEqual<Shleft<U<N>, $bits_log2>>>::Output: ReprFilled,
    {
    }

    impl<const N: usize> UintRepr for [$limb; N] {
      type Signed = Self;
      type NonZero = Self;
    }

    impl<const N: usize> UintCore for [$limb; N] {
      type Complement = Self;
      type NonZeroStorage = Self;
      type Inline = Self;

      #[inline(always)]
      fn inline(&self, _: Token) -> &Self {
        self
      }

      #[inline(always)]
      fn from_inline(value: Self, _: Token) -> Self {
        value
      }

      #[inline(always)]
      fn set_inline(&mut self, value: Self, _: Token) {
        *self = value;
      }

      #[inline]
      fn zero(_: Token) -> Self {
        [0; N]
      }

      #[inline]
      fn compare(&self, other: &Self, _: Token) -> Ordering {
        unsigned::$lower::cmp(self, other)
      }

      #[inline]
      fn compare_u128(&self, rhs: u128, _: Token) -> Ordering {
        unsigned::$lower::compare_u128(self, rhs)
      }

      #[inline]
      fn compare_i128(&self, rhs: i128, _: Token) -> Ordering {
        unsigned::$lower::compare_i128(self, rhs)
      }

      #[inline]
      fn hash_limbs<H: Hasher>(&self, bits: u32, state: &mut H, _: Token) {
        hash_words(self, bits, 0, state);
      }

      #[inline]
      fn fmt_decimal(&self, f: &mut fmt::Formatter<'_>, _: Token) -> fmt::Result {
        fmt_magnitude::<_, N, { unsigned::$lower::DIGITS_PER_LIMB }>(self, unsigned::$lower::write_digits_into, true, f)
      }

      #[inline]
      fn fmt_pow2(
        &self,
        shift: u32,
        upper: bool,
        f: &mut fmt::Formatter<'_>,
        _: Token,
      ) -> fmt::Result {
        format::write_pow2(self, shift, upper, f)
      }

      #[inline]
      fn fmt_exp(&self, upper: bool, f: &mut fmt::Formatter<'_>, _: Token) -> fmt::Result {
        fmt_magnitude_exp::<_, N, { unsigned::$lower::DIGITS_PER_LIMB }>(
          self,
          unsigned::$lower::write_digits_into,
          true,
          upper,
          f,
        )
      }

      #[inline]
      fn from_str_radix_bits(
        src: &[u8],
        radix: u32,
        bits: u32,
        _: Token,
      ) -> Result<Self, ParseIntError> {
        unsigned::$lower::from_str_radix_bits(src, radix, bits)
      }

      #[inline]
      fn write_bytes(&self, out: &mut [u8], big_endian: bool, _: Token) {
        unsigned::$lower::write_bytes(self, out, big_endian);
      }

      #[inline]
      fn checked_read_bytes(
        bytes: &[u8],
        big_endian: bool,
        bits: u32,
        _: Token,
      ) -> Option<Self> {
        unsigned::$lower::checked_read_bytes(bytes, big_endian, bits)
      }

      #[inline]
      fn checked_read_digits(
        digits: &[u8],
        big_endian: bool,
        radix: u32,
        bits: u32,
        _: Token,
      ) -> Option<Self> {
        unsigned::$lower::checked_read_digits(digits, big_endian, radix, bits)
      }

      #[inline]
      fn checked_mul_add_u64(&self, factor: u64, addend: u64, bits: u32, _: Token) -> Option<Self> {
        unsigned::$lower::append_chunk(self, factor, addend, bits, false)
      }

      #[inline]
      fn div_rem_u64(&self, divisor: u64, _: Token) -> (Self, u64) {
        unsigned::$lower::div_rem_chunk(self, divisor)
      }

      #[inline]
      fn wrapping_read_bytes(bytes: &[u8], big_endian: bool, bits: u32, _: Token) -> Self {
        unsigned::$lower::wrapping_read_bytes(bytes, big_endian, bits)
      }

      #[inline]
      fn saturating_read_bytes(bytes: &[u8], big_endian: bool, bits: u32, _: Token) -> Self {
        unsigned::$lower::saturating_read_bytes(bytes, big_endian, bits)
      }

      #[inline]
      fn byte_at(&self, index: usize, _: Token) -> u8 {
        unsigned::$lower::byte_at(self, index)
      }

      #[inline]
      fn swap_bytes_bits(&self, bits: u32, _: Token) -> Self {
        unsigned::$lower::swap_bytes_bits(self, bits)
      }

      #[inline]
      fn is_zero(&self, _: Token) -> bool {
        unsigned::$lower::is_zero(self)
      }

      #[inline]
      fn fits_bits(&self, bits: u32, _: Token) -> bool {
        unsigned::$lower::fits_bits(self, bits)
      }

      #[inline]
      fn overflowing_add_bits(&self, rhs: &Self, bits: u32, _: Token) -> (Self, bool) {
        unsigned::$lower::overflowing_add_bits(self, rhs, bits)
      }

      #[inline]
      fn overflowing_sub_bits(&self, rhs: &Self, bits: u32, _: Token) -> (Self, bool) {
        unsigned::$lower::overflowing_sub_bits(self, rhs, bits)
      }

      #[inline]
      fn overflowing_mul_bits(&self, rhs: &Self, bits: u32, _: Token) -> (Self, bool) {
        unsigned::$lower::overflowing_mul_bits(self, rhs, bits)
      }

      #[inline]
      fn overflowing_neg_bits(&self, bits: u32, _: Token) -> (Self, bool) {
        unsigned::$lower::overflowing_neg_bits(self, bits)
      }

      #[inline]
      fn carrying_add_bits(&self, rhs: &Self, carry: bool, bits: u32, _: Token) -> (Self, bool) {
        unsigned::$lower::carrying_add_bits(self, rhs, carry, bits)
      }

      #[inline]
      fn borrowing_sub_bits(&self, rhs: &Self, borrow: bool, bits: u32, _: Token) -> (Self, bool) {
        unsigned::$lower::borrowing_sub_bits(self, rhs, borrow, bits)
      }

      #[inline]
      fn is_one(&self, _: Token) -> bool {
        unsigned::$lower::is_one(self)
      }

      #[inline]
      fn div_rem_bits(&self, rhs: &Self, bits: u32, _: Token) -> Option<(Self, Self, bool)> {
        unsigned::$lower::div_rem_bits(self, rhs, bits)
      }

      #[inline]
      fn div_rem_euclid_bits(&self, rhs: &Self, bits: u32, _: Token) -> Option<(Self, Self, bool)> {
        unsigned::$lower::div_rem_euclid_bits(self, rhs, bits)
      }

      #[inline]
      fn div_floor_bits(&self, rhs: &Self, bits: u32, _: Token) -> Option<(Self, bool)> {
        unsigned::$lower::div_floor_bits(self, rhs, bits)
      }

      #[inline]
      fn div_ceil_bits(&self, rhs: &Self, bits: u32, _: Token) -> Option<(Self, bool)> {
        unsigned::$lower::div_ceil_bits(self, rhs, bits)
      }

      #[inline]
      fn next_multiple_of_bits(&self, rhs: &Self, bits: u32, _: Token) -> Option<(Self, bool)> {
        unsigned::$lower::next_multiple_of_bits(self, rhs, bits)
      }

      #[inline]
      fn checked_div_exact_bits(&self, rhs: &Self, bits: u32, _: Token) -> Option<Self> {
        unsigned::$lower::checked_div_exact_bits(self, rhs, bits)
      }

      #[inline]
      fn overflowing_pow_bits(&self, exp: u32, bits: u32, _: Token) -> (Self, bool) {
        unsigned::$lower::overflowing_pow_bits(self, exp, bits)
      }

      #[inline]
      fn checked_isqrt(&self, _: Token) -> Option<Self> {
        unsigned::$lower::checked_isqrt(self)
      }

      #[inline]
      fn checked_nth_root(&self, n: u32, _: Token) -> Option<Self> {
        unsigned::$lower::checked_nth_root(self, n)
      }

      #[inline]
      fn checked_ilog(&self, base: &Self, _: Token) -> Option<u32> {
        unsigned::$lower::checked_ilog(self, base)
      }

      #[inline]
      fn checked_ilog2(&self, _: Token) -> Option<u32> {
        unsigned::$lower::checked_ilog2(self)
      }

      #[inline]
      fn checked_ilog10(&self, _: Token) -> Option<u32> {
        unsigned::$lower::checked_ilog10(self)
      }

      #[inline]
      fn midpoint(&self, rhs: &Self, _: Token) -> Self {
        unsigned::$lower::midpoint(self, rhs)
      }

      #[inline]
      fn abs_diff(&self, rhs: &Self, _: Token) -> Self {
        unsigned::$lower::abs_diff(self, rhs)
      }

      #[inline]
      fn is_multiple_of(&self, rhs: &Self, _: Token) -> bool {
        unsigned::$lower::is_multiple_of(self, rhs)
      }

      #[inline]
      fn next_power_of_two_bits(&self, bits: u32, _: Token) -> (Self, bool) {
        unsigned::$lower::next_power_of_two_bits(self, bits)
      }

      #[inline]
      fn and(&self, rhs: &Self, _: Token) -> Self {
        unsigned::$lower::and(self, rhs)
      }

      #[inline]
      fn or(&self, rhs: &Self, _: Token) -> Self {
        unsigned::$lower::or(self, rhs)
      }

      #[inline]
      fn xor(&self, rhs: &Self, _: Token) -> Self {
        unsigned::$lower::xor(self, rhs)
      }

      #[inline]
      fn not_bits(&self, bits: u32, _: Token) -> Self {
        unsigned::$lower::not_bits(self, bits)
      }

      #[inline]
      fn count_ones_bits(&self, bits: u32, _: Token) -> u32 {
        unsigned::$lower::count_ones_bits(self, bits)
      }

      #[inline]
      fn leading_zeros_bits(&self, bits: u32, _: Token) -> u32 {
        unsigned::$lower::leading_zeros_bits(self, bits)
      }

      #[inline]
      fn trailing_zeros_bits(&self, bits: u32, _: Token) -> u32 {
        unsigned::$lower::trailing_zeros_bits(self, bits)
      }

      #[inline]
      fn leading_ones_bits(&self, bits: u32, _: Token) -> u32 {
        unsigned::$lower::leading_ones_bits(self, bits)
      }

      #[inline]
      fn trailing_ones_bits(&self, bits: u32, _: Token) -> u32 {
        unsigned::$lower::trailing_ones_bits(self, bits)
      }

      #[inline]
      fn highest_one_bits(&self, bits: u32, _: Token) -> Option<u32> {
        unsigned::$lower::highest_one_bits(self, bits)
      }

      #[inline]
      fn lowest_one(&self, _: Token) -> Option<u32> {
        unsigned::$lower::lowest_one(self)
      }

      #[inline]
      fn isolate_highest_one_bits(&self, bits: u32, _: Token) -> Self {
        unsigned::$lower::isolate_highest_one_bits(self, bits)
      }

      #[inline]
      fn isolate_lowest_one_bits(&self, bits: u32, _: Token) -> Self {
        unsigned::$lower::isolate_lowest_one_bits(self, bits)
      }

      #[inline]
      fn reverse_bits_bits(&self, bits: u32, _: Token) -> Self {
        unsigned::$lower::reverse_bits_bits(self, bits)
      }

      #[inline]
      fn rotate_left_bits(&self, n: u32, bits: u32, _: Token) -> Self {
        unsigned::$lower::rotate_left_bits(self, n, bits)
      }

      #[inline]
      fn rotate_right_bits(&self, n: u32, bits: u32, _: Token) -> Self {
        unsigned::$lower::rotate_right_bits(self, n, bits)
      }

      #[inline]
      fn overflowing_shl_bits(&self, rhs: u32, bits: u32, _: Token) -> (Self, bool) {
        unsigned::$lower::overflowing_shl_bits(self, rhs, bits)
      }

      #[inline]
      fn overflowing_shr_bits(&self, rhs: u32, bits: u32, _: Token) -> (Self, bool) {
        unsigned::$lower::overflowing_shr_bits(self, rhs, bits)
      }

      #[inline]
      fn unbounded_shl_bits(&self, rhs: u32, bits: u32, _: Token) -> Self {
        unsigned::$lower::unbounded_shl_bits(self, rhs, bits)
      }

      #[inline]
      fn unbounded_shr_bits(&self, rhs: u32, bits: u32, _: Token) -> Self {
        unsigned::$lower::unbounded_shr_bits(self, rhs, bits)
      }

      #[inline]
      fn shl_exact_bits(&self, rhs: u32, bits: u32, _: Token) -> Option<Self> {
        unsigned::$lower::shl_exact_bits(self, rhs, bits)
      }

      #[inline]
      fn shr_exact_bits(&self, rhs: u32, bits: u32, _: Token) -> Option<Self> {
        unsigned::$lower::shr_exact_bits(self, rhs, bits)
      }

      #[inline]
      fn bit(&self, index: u32, bits: u32, _: Token) -> bool {
        unsigned::$lower::bit(self, index, bits)
      }

      #[inline]
      fn with_bit(&self, index: u32, set: bool, bits: u32, _: Token) -> Option<Self> {
        unsigned::$lower::with_bit(self, index, set, bits)
      }

      #[inline]
      fn cast_signed_bits(self, bits: u32, _: Token) -> Self {
        unsigned::$lower::cast_signed_bits(&self, bits)
      }

      #[inline]
      fn saturating_cast_signed_bits(self, bits: u32, _: Token) -> Self {
        unsigned::$lower::saturating_cast_signed_bits(&self, bits)
      }

      #[inline]
      fn to_nonzero(self, _: Token) -> Option<Self> {
        if unsigned::$lower::is_zero(&self) {
          None
        } else {
          Some(self)
        }
      }

      #[inline]
      fn from_nonzero(nonzero: Self, _: Token) -> Self {
        nonzero
      }

      #[inline]
      fn with_nonzero<T>(nonzero: &Self, f: impl FnOnce(&Self) -> T, _: Token) -> T {
        f(nonzero)
      }

      #[inline]
      fn lend_nonzero<B>(nonzero: &Uint<B, Self>, _: Token) -> View<'_, Uint<B, Self>> {
        View::Lent(nonzero)
      }

      #[inline]
      fn known_nonzero(self, _: Token) -> Self {
        debug_assert!(!unsigned::$lower::is_zero(&self), "a non-zero result is zero");
        self
      }

      #[inline]
      fn word(&self, index: usize, _: Token) -> u64 {
        unsigned::$lower::word(self, index)
      }

      #[inline]
      fn from_words(word: impl Fn(usize) -> u64, bits: u32, _: Token) -> Self {
        unsigned::$lower::zero_extend(&limbs_of_words(word, |low| low as $limb), bits)
      }

      #[inline]
      fn float_bits(&self, format: FloatFormat, _: Token) -> u64 {
        unsigned::$lower::float_bits(self, format)
      }

      #[inline]
      fn from_float(
        parts: FloatParts,
        mode: RoundingMode,
        bits: u32,
        _: Token,
      ) -> Result<Self, FromFloatError> {
        unsigned::$lower::place_magnitude(int::float::round_magnitude(parts, mode), bits)
      }

      #[inline]
      fn checked_read_limbs(
        limbs: &[<Self as ReprLimb>::Limb],
        bits: u32,
        _: Token,
      ) -> Option<Self> {
        unsigned::$lower::checked_read_limbs(limbs, bits)
      }

      #[inline]
      fn wrapping_read_limbs(limbs: &[<Self as ReprLimb>::Limb], bits: u32, _: Token) -> Self {
        unsigned::$lower::wrapping_read_limbs(limbs, bits)
      }

      #[inline]
      fn saturating_read_limbs(limbs: &[<Self as ReprLimb>::Limb], bits: u32, _: Token) -> Self {
        unsigned::$lower::saturating_read_limbs(limbs, bits)
      }
    }

    impl<const N: usize> IntRepr for [$limb; N] {
      type Unsigned = Self;
      type NonZero = Self;
    }

    impl<const N: usize> IntCore for [$limb; N] {
      type Magnitude = Self;
      type NonZeroStorage = Self;
      type Inline = Self;

      #[inline(always)]
      fn inline(&self, _: Token) -> &Self {
        self
      }

      #[inline(always)]
      fn from_inline(value: Self, _: Token) -> Self {
        value
      }

      #[inline(always)]
      fn set_inline(&mut self, value: Self, _: Token) {
        *self = value;
      }

      #[inline]
      fn zero(_: Token) -> Self {
        [0; N]
      }

      #[inline]
      fn compare(&self, other: &Self, _: Token) -> Ordering {
        signed::$upper::cmp(self, other)
      }

      #[inline]
      fn compare_u128(&self, rhs: u128, _: Token) -> Ordering {
        signed::$upper::compare_u128(self, rhs)
      }

      #[inline]
      fn compare_i128(&self, rhs: i128, _: Token) -> Ordering {
        signed::$upper::compare_i128(self, rhs)
      }

      #[inline]
      fn hash_limbs<H: Hasher>(&self, bits: u32, state: &mut H, _: Token) {
        // The limbs above the sign bit hold its copies, so the lowest limbs hold the two's
        // complement of the value, and the limbs beyond a storage narrower than the words read as
        // further copies.
        let fill = if signed::$upper::is_negative(self) { $limb::MAX } else { 0 };
        hash_words(self, bits, fill, state);
      }

      #[inline]
      fn fmt_decimal(&self, f: &mut fmt::Formatter<'_>, _: Token) -> fmt::Result {
        fmt_magnitude::<_, N, { unsigned::$lower::DIGITS_PER_LIMB }>(
          &signed::$upper::unsigned_abs(self),
          unsigned::$lower::write_digits_into,
          !signed::$upper::is_negative(self),
          f,
        )
      }

      #[inline]
      fn fmt_pow2(
        &self,
        bits: u32,
        shift: u32,
        upper: bool,
        f: &mut fmt::Formatter<'_>,
        _: Token,
      ) -> fmt::Result {
        format::write_pow2(&signed::$upper::cast_unsigned_bits(self, bits), shift, upper, f)
      }

      #[inline]
      fn fmt_exp(&self, upper: bool, f: &mut fmt::Formatter<'_>, _: Token) -> fmt::Result {
        fmt_magnitude_exp::<_, N, { unsigned::$lower::DIGITS_PER_LIMB }>(
          &signed::$upper::unsigned_abs(self),
          unsigned::$lower::write_digits_into,
          !signed::$upper::is_negative(self),
          upper,
          f,
        )
      }

      #[inline]
      fn from_str_radix_bits(
        src: &[u8],
        radix: u32,
        bits: u32,
        _: Token,
      ) -> Result<Self, ParseIntError> {
        signed::$upper::from_str_radix_bits(src, radix, bits)
      }

      // The limbs hold the two's complement of the value and the copies of its sign, which the
      // lower layer writes and reads byte by byte.
      #[inline]
      fn write_bytes(&self, out: &mut [u8], big_endian: bool, _: Token) {
        unsigned::$lower::write_bytes(self, out, big_endian);
      }

      #[inline]
      fn checked_read_bytes(
        bytes: &[u8],
        big_endian: bool,
        bits: u32,
        _: Token,
      ) -> Option<Self> {
        signed::$upper::checked_read_bytes(bytes, big_endian, bits)
      }

      #[inline]
      fn wrapping_read_bytes(bytes: &[u8], big_endian: bool, bits: u32, _: Token) -> Self {
        signed::$upper::wrapping_read_bytes(bytes, big_endian, bits)
      }

      #[inline]
      fn saturating_read_bytes(bytes: &[u8], big_endian: bool, bits: u32, _: Token) -> Self {
        signed::$upper::saturating_read_bytes(bytes, big_endian, bits)
      }

      #[inline]
      fn byte_at(&self, index: usize, _: Token) -> u8 {
        unsigned::$lower::byte_at(self, index)
      }

      #[inline]
      fn swap_bytes_bits(&self, bits: u32, _: Token) -> Self {
        signed::$upper::swap_bytes_bits(self, bits)
      }

      #[inline]
      fn is_zero(&self, _: Token) -> bool {
        signed::$upper::is_zero(self)
      }

      #[inline]
      fn fits_bits(&self, bits: u32, _: Token) -> bool {
        signed::$upper::fits_bits(self, bits)
      }

      #[inline]
      fn is_negative(&self, _: Token) -> bool {
        signed::$upper::is_negative(self)
      }

      #[inline]
      fn is_positive(&self, _: Token) -> bool {
        signed::$upper::is_positive(self)
      }

      #[inline]
      fn overflowing_add_bits(&self, rhs: &Self, bits: u32, _: Token) -> (Self, bool) {
        signed::$upper::overflowing_add_bits(self, rhs, bits)
      }

      #[inline]
      fn overflowing_sub_bits(&self, rhs: &Self, bits: u32, _: Token) -> (Self, bool) {
        signed::$upper::overflowing_sub_bits(self, rhs, bits)
      }

      #[inline]
      fn overflowing_mul_bits(&self, rhs: &Self, bits: u32, _: Token) -> (Self, bool) {
        signed::$upper::overflowing_mul_bits(self, rhs, bits)
      }

      #[inline]
      fn overflowing_neg_bits(&self, bits: u32, _: Token) -> (Self, bool) {
        signed::$upper::overflowing_neg_bits(self, bits)
      }

      #[inline]
      fn is_one(&self, _: Token) -> bool {
        signed::$upper::is_one(self)
      }

      #[inline]
      fn signum(&self, _: Token) -> Self {
        signed::$upper::signum(self)
      }

      #[inline]
      fn unsigned_abs(&self, _: Token) -> Self {
        signed::$upper::unsigned_abs(self)
      }

      #[inline]
      fn div_rem_bits(&self, rhs: &Self, bits: u32, _: Token) -> Option<(Self, Self, bool)> {
        signed::$upper::div_rem_bits(self, rhs, bits)
      }

      #[inline]
      fn div_rem_euclid_bits(&self, rhs: &Self, bits: u32, _: Token) -> Option<(Self, Self, bool)> {
        signed::$upper::div_rem_euclid_bits(self, rhs, bits)
      }

      #[inline]
      fn div_floor_bits(&self, rhs: &Self, bits: u32, _: Token) -> Option<(Self, bool)> {
        signed::$upper::div_floor_bits(self, rhs, bits)
      }

      #[inline]
      fn div_ceil_bits(&self, rhs: &Self, bits: u32, _: Token) -> Option<(Self, bool)> {
        signed::$upper::div_ceil_bits(self, rhs, bits)
      }

      #[inline]
      fn next_multiple_of_bits(&self, rhs: &Self, bits: u32, _: Token) -> Option<(Self, bool)> {
        signed::$upper::next_multiple_of_bits(self, rhs, bits)
      }

      #[inline]
      fn checked_div_exact_bits(&self, rhs: &Self, bits: u32, _: Token) -> Option<Self> {
        signed::$upper::checked_div_exact_bits(self, rhs, bits)
      }

      #[inline]
      fn overflowing_pow_bits(&self, exp: u32, bits: u32, _: Token) -> (Self, bool) {
        signed::$upper::overflowing_pow_bits(self, exp, bits)
      }

      #[inline]
      fn checked_isqrt(&self, _: Token) -> Option<Self> {
        signed::$upper::checked_isqrt(self)
      }

      #[inline]
      fn checked_nth_root(&self, n: u32, _: Token) -> Option<Self> {
        signed::$upper::checked_nth_root(self, n)
      }

      #[inline]
      fn checked_ilog(&self, base: &Self, _: Token) -> Option<u32> {
        signed::$upper::checked_ilog(self, base)
      }

      #[inline]
      fn checked_ilog2(&self, _: Token) -> Option<u32> {
        signed::$upper::checked_ilog2(self)
      }

      #[inline]
      fn checked_ilog10(&self, _: Token) -> Option<u32> {
        signed::$upper::checked_ilog10(self)
      }

      #[inline]
      fn midpoint(&self, rhs: &Self, _: Token) -> Self {
        signed::$upper::midpoint(self, rhs)
      }

      #[inline]
      fn abs_diff(&self, rhs: &Self, _: Token) -> Self {
        signed::$upper::abs_diff(self, rhs)
      }

      // The limbs combine as unsigned ones do, and so do the copies of the signs.
      #[inline]
      fn and(&self, rhs: &Self, _: Token) -> Self {
        unsigned::$lower::and(self, rhs)
      }

      #[inline]
      fn or(&self, rhs: &Self, _: Token) -> Self {
        unsigned::$lower::or(self, rhs)
      }

      #[inline]
      fn xor(&self, rhs: &Self, _: Token) -> Self {
        unsigned::$lower::xor(self, rhs)
      }

      #[inline]
      fn not_bits(&self, bits: u32, _: Token) -> Self {
        signed::$upper::not_bits(self, bits)
      }

      #[inline]
      fn count_ones_bits(&self, bits: u32, _: Token) -> u32 {
        signed::$upper::count_ones_bits(self, bits)
      }

      #[inline]
      fn leading_zeros_bits(&self, bits: u32, _: Token) -> u32 {
        signed::$upper::leading_zeros_bits(self, bits)
      }

      #[inline]
      fn trailing_zeros_bits(&self, bits: u32, _: Token) -> u32 {
        signed::$upper::trailing_zeros_bits(self, bits)
      }

      #[inline]
      fn leading_ones_bits(&self, bits: u32, _: Token) -> u32 {
        signed::$upper::leading_ones_bits(self, bits)
      }

      #[inline]
      fn trailing_ones_bits(&self, bits: u32, _: Token) -> u32 {
        signed::$upper::trailing_ones_bits(self, bits)
      }

      #[inline]
      fn highest_one_bits(&self, bits: u32, _: Token) -> Option<u32> {
        signed::$upper::highest_one_bits(self, bits)
      }

      #[inline]
      fn lowest_one(&self, _: Token) -> Option<u32> {
        signed::$upper::lowest_one(self)
      }

      #[inline]
      fn isolate_highest_one_bits(&self, bits: u32, _: Token) -> Self {
        signed::$upper::isolate_highest_one_bits(self, bits)
      }

      #[inline]
      fn isolate_lowest_one_bits(&self, bits: u32, _: Token) -> Self {
        signed::$upper::isolate_lowest_one_bits(self, bits)
      }

      #[inline]
      fn reverse_bits_bits(&self, bits: u32, _: Token) -> Self {
        signed::$upper::reverse_bits_bits(self, bits)
      }

      #[inline]
      fn rotate_left_bits(&self, n: u32, bits: u32, _: Token) -> Self {
        signed::$upper::rotate_left_bits(self, n, bits)
      }

      #[inline]
      fn rotate_right_bits(&self, n: u32, bits: u32, _: Token) -> Self {
        signed::$upper::rotate_right_bits(self, n, bits)
      }

      #[inline]
      fn overflowing_shl_bits(&self, rhs: u32, bits: u32, _: Token) -> (Self, bool) {
        signed::$upper::overflowing_shl_bits(self, rhs, bits)
      }

      #[inline]
      fn overflowing_shr_bits(&self, rhs: u32, bits: u32, _: Token) -> (Self, bool) {
        signed::$upper::overflowing_shr_bits(self, rhs, bits)
      }

      #[inline]
      fn unbounded_shl_bits(&self, rhs: u32, bits: u32, _: Token) -> Self {
        signed::$upper::unbounded_shl_bits(self, rhs, bits)
      }

      #[inline]
      fn unbounded_shr_bits(&self, rhs: u32, bits: u32, _: Token) -> Self {
        signed::$upper::unbounded_shr_bits(self, rhs, bits)
      }

      #[inline]
      fn shl_exact_bits(&self, rhs: u32, bits: u32, _: Token) -> Option<Self> {
        signed::$upper::shl_exact_bits(self, rhs, bits)
      }

      #[inline]
      fn shr_exact_bits(&self, rhs: u32, bits: u32, _: Token) -> Option<Self> {
        signed::$upper::shr_exact_bits(self, rhs, bits)
      }

      #[inline]
      fn bit(&self, index: u32, bits: u32, _: Token) -> bool {
        signed::$upper::bit(self, index, bits)
      }

      #[inline]
      fn with_bit(&self, index: u32, set: bool, bits: u32, _: Token) -> Option<Self> {
        signed::$upper::with_bit(self, index, set, bits)
      }

      #[inline]
      fn cast_unsigned_bits(self, bits: u32, _: Token) -> Self {
        signed::$upper::cast_unsigned_bits(&self, bits)
      }

      #[inline]
      fn saturating_cast_unsigned_bits(self, bits: u32, _: Token) -> Self {
        signed::$upper::saturating_cast_unsigned_bits(&self, bits)
      }

      #[inline]
      fn to_nonzero(self, _: Token) -> Option<Self> {
        if signed::$upper::is_zero(&self) {
          None
        } else {
          Some(self)
        }
      }

      #[inline]
      fn from_nonzero(nonzero: Self, _: Token) -> Self {
        nonzero
      }

      #[inline]
      fn with_nonzero<T>(nonzero: &Self, f: impl FnOnce(&Self) -> T, _: Token) -> T {
        f(nonzero)
      }

      #[inline]
      fn lend_nonzero<B>(nonzero: &Int<B, Self>, _: Token) -> View<'_, Int<B, Self>> {
        View::Lent(nonzero)
      }

      #[inline]
      fn known_nonzero(self, _: Token) -> Self {
        debug_assert!(!signed::$upper::is_zero(&self), "a non-zero result is zero");
        self
      }

      #[inline]
      fn word(&self, index: usize, _: Token) -> u64 {
        signed::$upper::word(self, index)
      }

      #[inline]
      fn from_words(word: impl Fn(usize) -> u64, bits: u32, _: Token) -> Self {
        signed::$upper::sign_extend(&limbs_of_words(word, |low| low as $limb), bits)
      }

      #[inline]
      fn float_bits(&self, format: FloatFormat, _: Token) -> u64 {
        signed::$upper::float_bits(self, format)
      }

      #[inline]
      fn from_float(
        parts: FloatParts,
        mode: RoundingMode,
        bits: u32,
        _: Token,
      ) -> Result<Self, FromFloatError> {
        signed::$upper::place_magnitude(int::float::round_magnitude(parts, mode), bits)
      }

      #[inline]
      fn checked_read_limbs(
        limbs: &[<Self as ReprLimb>::Limb],
        bits: u32,
        _: Token,
      ) -> Option<Self> {
        signed::$upper::checked_read_limbs(limbs, bits)
      }

      #[inline]
      fn wrapping_read_limbs(limbs: &[<Self as ReprLimb>::Limb], bits: u32, _: Token) -> Self {
        signed::$upper::wrapping_read_limbs(limbs, bits)
      }

      #[inline]
      fn saturating_read_limbs(limbs: &[<Self as ReprLimb>::Limb], bits: u32, _: Token) -> Self {
        signed::$upper::saturating_read_limbs(limbs, bits)
      }
    }

    impl<const N: usize> NonZeroRepr for [$limb; N] {}

    impl<const N: usize> NonZeroUintRepr for [$limb; N] {
      type Integer = Self;
      type Signed = Self;
    }

    impl<const N: usize> NonZeroIntRepr for [$limb; N] {
      type Integer = Self;
      type Unsigned = Self;
    }

    impl<const N: usize> MutableRepr for [$limb; N] {}

    impl<const N: usize> UintReprMut for [$limb; N] {}

    impl<const N: usize> IntReprMut for [$limb; N] {}

    impl<const N: usize> InlineUintRepr for [$limb; N] {}

    impl<const N: usize> InlineUintConsts for [$limb; N] {
      const ZERO: Self = [0; N];
      const ONE: Self = unsigned::$lower::one();

      type Width<B: FitsBits<Self>> = Width<B, Self>;
    }

    impl<B: FitsBits<[$limb; N]>, const N: usize> UintWidth<[$limb; N]> for Width<B, [$limb; N]> {
      const MAX: [$limb; N] = unsigned::$lower::max_of_bits(B::U32);
      const NONZERO_MIN: [$limb; N] = unsigned::$lower::one();
      const NONZERO_MAX: [$limb; N] = unsigned::$lower::max_of_bits(B::U32);
    }

    impl<const N: usize> InlineIntRepr for [$limb; N] {}

    impl<const N: usize> InlineIntConsts for [$limb; N] {
      const ZERO: Self = [0; N];
      const ONE: Self = unsigned::$lower::one();

      type Width<B: FitsBits<Self>> = Width<B, Self>;
    }

    impl<B: FitsBits<[$limb; N]>, const N: usize> IntWidth<[$limb; N]> for Width<B, [$limb; N]> {
      const MIN: [$limb; N] = signed::$upper::min_of_bits(B::U32);
      const MAX: [$limb; N] = signed::$upper::max_of_bits(B::U32);
      const NONZERO_MIN: [$limb; N] = signed::$upper::min_of_bits(B::U32);
      // One bit holds -1 and 0, so its largest non-zero value is -1.
      const NONZERO_MAX: [$limb; N] = if B::U32 == 1 {
        signed::$upper::min_of_bits(B::U32)
      } else {
        signed::$upper::max_of_bits(B::U32)
      };
    }
  )+};
}

array_storage!(
  u64: U6, b64, b64;
  u8: U3, b8, b8;
  u16: U4, b16, b16;
  u32: U5, b32, b32;
  u128: U7, b128, b128;
);

/// Implements [`UintRepr`], [`IntRepr`], [`UintReprMut`], [`IntReprMut`], [`InlineUintRepr`] and
/// [`InlineIntRepr`] for the native integers, and their width relations, and [`NonZeroUintRepr`]
/// and [`NonZeroIntRepr`] for their `NonZero`, written `unsigned, signed: Bits, layer`, where
/// `Bits` is the number of bits of both and `layer` the module of the limb layers of that width,
/// through which they read a slice of limbs as an array of one limb.
// Each native type has width impls of its own, as each array type constructor has: one impl over
// every storage, bounded by a trait of the storages, would degrade the diagnostics of the arrays.
macro_rules! native_storage {
  ($($uint:ident, $int:ident: $bits:ident, $layer:ident);+ $(;)?) => {$(
    native_storage!(@width $uint, $bits);
    native_storage!(@width $int, $bits);

    impl UintRepr for $uint {
      type Signed = $int;
      type NonZero = core::num::NonZero<$uint>;
    }

    impl UintCore for $uint {
      type Complement = $int;
      type NonZeroStorage = core::num::NonZero<$uint>;
      type Inline = Self;

      #[inline(always)]
      fn inline(&self, _: Token) -> &Self {
        self
      }

      #[inline(always)]
      fn from_inline(value: Self, _: Token) -> Self {
        value
      }

      #[inline(always)]
      fn set_inline(&mut self, value: Self, _: Token) {
        *self = value;
      }

      #[inline]
      fn zero(_: Token) -> Self {
        0
      }

      #[inline]
      fn compare(&self, other: &Self, _: Token) -> Ordering {
        self.cmp(other)
      }

      #[inline]
      fn compare_u128(&self, rhs: u128, _: Token) -> Ordering {
        int::$uint::compare_u128(*self, rhs)
      }

      #[inline]
      fn compare_i128(&self, rhs: i128, _: Token) -> Ordering {
        int::$uint::compare_i128(*self, rhs)
      }

      #[inline]
      fn hash_limbs<H: Hasher>(&self, bits: u32, state: &mut H, _: Token) {
        hash_words(&[*self], bits, 0, state);
      }

      #[inline]
      fn fmt_decimal(&self, f: &mut fmt::Formatter<'_>, _: Token) -> fmt::Result {
        fmt::Display::fmt(self, f)
      }

      #[inline]
      fn fmt_pow2(
        &self,
        shift: u32,
        upper: bool,
        f: &mut fmt::Formatter<'_>,
        _: Token,
      ) -> fmt::Result {
        fmt_native_pow2(self, shift, upper, f)
      }

      #[inline]
      fn fmt_exp(&self, upper: bool, f: &mut fmt::Formatter<'_>, _: Token) -> fmt::Result {
        if upper {
          fmt::UpperExp::fmt(self, f)
        } else {
          fmt::LowerExp::fmt(self, f)
        }
      }

      #[inline]
      fn from_str_radix_bits(
        src: &[u8],
        radix: u32,
        bits: u32,
        _: Token,
      ) -> Result<Self, ParseIntError> {
        int::$uint::from_str_radix_bits(src, radix, bits)
      }

      #[inline]
      fn write_bytes(&self, out: &mut [u8], big_endian: bool, _: Token) {
        int::$uint::write_bytes(*self, out, big_endian);
      }

      #[inline]
      fn checked_read_bytes(
        bytes: &[u8],
        big_endian: bool,
        bits: u32,
        _: Token,
      ) -> Option<Self> {
        int::$uint::checked_read_bytes(bytes, big_endian, bits)
      }

      #[inline]
      fn checked_read_digits(
        digits: &[u8],
        big_endian: bool,
        radix: u32,
        bits: u32,
        _: Token,
      ) -> Option<Self> {
        int::$uint::checked_read_digits(digits, big_endian, radix, bits)
      }

      #[inline]
      fn checked_mul_add_u64(&self, factor: u64, addend: u64, bits: u32, _: Token) -> Option<Self> {
        int::$uint::checked_mul_add_chunk(*self, factor, addend, bits)
      }

      #[inline]
      fn div_rem_u64(&self, divisor: u64, _: Token) -> (Self, u64) {
        int::$uint::div_rem_chunk(*self, divisor)
      }

      #[inline]
      fn wrapping_read_bytes(bytes: &[u8], big_endian: bool, bits: u32, _: Token) -> Self {
        int::$uint::wrapping_read_bytes(bytes, big_endian, bits)
      }

      #[inline]
      fn saturating_read_bytes(bytes: &[u8], big_endian: bool, bits: u32, _: Token) -> Self {
        int::$uint::saturating_read_bytes(bytes, big_endian, bits)
      }

      #[inline]
      fn byte_at(&self, index: usize, _: Token) -> u8 {
        int::$uint::byte_at(*self, index)
      }

      #[inline]
      fn swap_bytes_bits(&self, bits: u32, _: Token) -> Self {
        int::$uint::swap_bytes_bits(*self, bits)
      }

      #[inline]
      fn is_zero(&self, _: Token) -> bool {
        int::$uint::is_zero(*self)
      }

      #[inline]
      fn fits_bits(&self, bits: u32, _: Token) -> bool {
        int::$uint::fits_bits(*self, bits)
      }

      #[inline]
      fn overflowing_add_bits(&self, rhs: &Self, bits: u32, _: Token) -> (Self, bool) {
        int::$uint::overflowing_add_bits(*self, *rhs, bits)
      }

      #[inline]
      fn overflowing_sub_bits(&self, rhs: &Self, bits: u32, _: Token) -> (Self, bool) {
        int::$uint::overflowing_sub_bits(*self, *rhs, bits)
      }

      #[inline]
      fn overflowing_mul_bits(&self, rhs: &Self, bits: u32, _: Token) -> (Self, bool) {
        int::$uint::overflowing_mul_bits(*self, *rhs, bits)
      }

      #[inline]
      fn overflowing_neg_bits(&self, bits: u32, _: Token) -> (Self, bool) {
        int::$uint::overflowing_neg_bits(*self, bits)
      }

      #[inline]
      fn carrying_add_bits(&self, rhs: &Self, carry: bool, bits: u32, _: Token) -> (Self, bool) {
        int::$uint::carrying_add_bits(*self, *rhs, carry, bits)
      }

      #[inline]
      fn borrowing_sub_bits(&self, rhs: &Self, borrow: bool, bits: u32, _: Token) -> (Self, bool) {
        int::$uint::borrowing_sub_bits(*self, *rhs, borrow, bits)
      }

      #[inline]
      fn is_one(&self, _: Token) -> bool {
        int::$uint::is_one(*self)
      }

      // The layer is named by its path, as in `IntCore` below: method syntax on `self`, a
      // `&$uint`, would find these methods before the inherent methods of the same names.
      #[inline]
      fn div_rem_bits(&self, rhs: &Self, bits: u32, _: Token) -> Option<(Self, Self, bool)> {
        int::$uint::div_rem_bits(*self, *rhs, bits)
      }

      #[inline]
      fn div_rem_euclid_bits(&self, rhs: &Self, bits: u32, _: Token) -> Option<(Self, Self, bool)> {
        int::$uint::div_rem_euclid_bits(*self, *rhs, bits)
      }

      #[inline]
      fn div_floor_bits(&self, rhs: &Self, bits: u32, _: Token) -> Option<(Self, bool)> {
        int::$uint::div_floor_bits(*self, *rhs, bits)
      }

      #[inline]
      fn div_ceil_bits(&self, rhs: &Self, bits: u32, _: Token) -> Option<(Self, bool)> {
        int::$uint::div_ceil_bits(*self, *rhs, bits)
      }

      #[inline]
      fn next_multiple_of_bits(&self, rhs: &Self, bits: u32, _: Token) -> Option<(Self, bool)> {
        int::$uint::next_multiple_of_bits(*self, *rhs, bits)
      }

      #[inline]
      fn checked_div_exact_bits(&self, rhs: &Self, bits: u32, _: Token) -> Option<Self> {
        int::$uint::checked_div_exact_bits(*self, *rhs, bits)
      }

      #[inline]
      fn overflowing_pow_bits(&self, exp: u32, bits: u32, _: Token) -> (Self, bool) {
        int::$uint::overflowing_pow_bits(*self, exp, bits)
      }

      #[inline]
      fn checked_isqrt(&self, _: Token) -> Option<Self> {
        int::$uint::checked_isqrt(*self)
      }

      #[inline]
      fn checked_nth_root(&self, n: u32, _: Token) -> Option<Self> {
        int::$uint::checked_nth_root(*self, n)
      }

      #[inline]
      fn checked_ilog(&self, base: &Self, _: Token) -> Option<u32> {
        int::$uint::checked_ilog(*self, *base)
      }

      #[inline]
      fn checked_ilog2(&self, _: Token) -> Option<u32> {
        int::$uint::checked_ilog2(*self)
      }

      #[inline]
      fn checked_ilog10(&self, _: Token) -> Option<u32> {
        int::$uint::checked_ilog10(*self)
      }

      #[inline]
      fn midpoint(&self, rhs: &Self, _: Token) -> Self {
        int::$uint::midpoint(*self, *rhs)
      }

      #[inline]
      fn abs_diff(&self, rhs: &Self, _: Token) -> Self {
        int::$uint::abs_diff(*self, *rhs)
      }

      #[inline]
      fn is_multiple_of(&self, rhs: &Self, _: Token) -> bool {
        int::$uint::is_multiple_of(*self, *rhs)
      }

      #[inline]
      fn next_power_of_two_bits(&self, bits: u32, _: Token) -> (Self, bool) {
        int::$uint::next_power_of_two_bits(*self, bits)
      }

      #[inline]
      fn and(&self, rhs: &Self, _: Token) -> Self {
        int::$uint::and(*self, *rhs)
      }

      #[inline]
      fn or(&self, rhs: &Self, _: Token) -> Self {
        int::$uint::or(*self, *rhs)
      }

      #[inline]
      fn xor(&self, rhs: &Self, _: Token) -> Self {
        int::$uint::xor(*self, *rhs)
      }

      #[inline]
      fn not_bits(&self, bits: u32, _: Token) -> Self {
        int::$uint::not_bits(*self, bits)
      }

      #[inline]
      fn count_ones_bits(&self, bits: u32, _: Token) -> u32 {
        int::$uint::count_ones_bits(*self, bits)
      }

      #[inline]
      fn leading_zeros_bits(&self, bits: u32, _: Token) -> u32 {
        int::$uint::leading_zeros_bits(*self, bits)
      }

      #[inline]
      fn trailing_zeros_bits(&self, bits: u32, _: Token) -> u32 {
        int::$uint::trailing_zeros_bits(*self, bits)
      }

      #[inline]
      fn leading_ones_bits(&self, bits: u32, _: Token) -> u32 {
        int::$uint::leading_ones_bits(*self, bits)
      }

      #[inline]
      fn trailing_ones_bits(&self, bits: u32, _: Token) -> u32 {
        int::$uint::trailing_ones_bits(*self, bits)
      }

      #[inline]
      fn highest_one_bits(&self, bits: u32, _: Token) -> Option<u32> {
        int::$uint::highest_one_bits(*self, bits)
      }

      #[inline]
      fn lowest_one(&self, _: Token) -> Option<u32> {
        int::$uint::lowest_one(*self)
      }

      #[inline]
      fn isolate_highest_one_bits(&self, bits: u32, _: Token) -> Self {
        int::$uint::isolate_highest_one_bits(*self, bits)
      }

      #[inline]
      fn isolate_lowest_one_bits(&self, bits: u32, _: Token) -> Self {
        int::$uint::isolate_lowest_one_bits(*self, bits)
      }

      #[inline]
      fn reverse_bits_bits(&self, bits: u32, _: Token) -> Self {
        int::$uint::reverse_bits_bits(*self, bits)
      }

      #[inline]
      fn rotate_left_bits(&self, n: u32, bits: u32, _: Token) -> Self {
        int::$uint::rotate_left_bits(*self, n, bits)
      }

      #[inline]
      fn rotate_right_bits(&self, n: u32, bits: u32, _: Token) -> Self {
        int::$uint::rotate_right_bits(*self, n, bits)
      }

      #[inline]
      fn overflowing_shl_bits(&self, rhs: u32, bits: u32, _: Token) -> (Self, bool) {
        int::$uint::overflowing_shl_bits(*self, rhs, bits)
      }

      #[inline]
      fn overflowing_shr_bits(&self, rhs: u32, bits: u32, _: Token) -> (Self, bool) {
        int::$uint::overflowing_shr_bits(*self, rhs, bits)
      }

      #[inline]
      fn unbounded_shl_bits(&self, rhs: u32, bits: u32, _: Token) -> Self {
        int::$uint::unbounded_shl_bits(*self, rhs, bits)
      }

      #[inline]
      fn unbounded_shr_bits(&self, rhs: u32, bits: u32, _: Token) -> Self {
        int::$uint::unbounded_shr_bits(*self, rhs, bits)
      }

      #[inline]
      fn shl_exact_bits(&self, rhs: u32, bits: u32, _: Token) -> Option<Self> {
        int::$uint::shl_exact_bits(*self, rhs, bits)
      }

      #[inline]
      fn shr_exact_bits(&self, rhs: u32, bits: u32, _: Token) -> Option<Self> {
        int::$uint::shr_exact_bits(*self, rhs, bits)
      }

      #[inline]
      fn bit(&self, index: u32, bits: u32, _: Token) -> bool {
        int::$uint::bit(*self, index, bits)
      }

      #[inline]
      fn with_bit(&self, index: u32, set: bool, bits: u32, _: Token) -> Option<Self> {
        int::$uint::with_bit(*self, index, set, bits)
      }

      #[inline]
      fn cast_signed_bits(self, bits: u32, _: Token) -> $int {
        int::$uint::cast_signed_bits(self, bits)
      }

      #[inline]
      fn saturating_cast_signed_bits(self, bits: u32, _: Token) -> $int {
        int::$uint::saturating_cast_signed_bits(self, bits)
      }

      #[inline]
      fn to_nonzero(self, _: Token) -> Option<core::num::NonZero<$uint>> {
        core::num::NonZero::new(self)
      }

      #[inline]
      fn from_nonzero(nonzero: core::num::NonZero<$uint>, _: Token) -> Self {
        nonzero.get()
      }

      #[inline]
      fn with_nonzero<T>(
        nonzero: &core::num::NonZero<$uint>,
        f: impl FnOnce(&Self) -> T,
        _: Token,
      ) -> T {
        f(&nonzero.get())
      }

      #[inline]
      fn lend_nonzero<B>(
        nonzero: &Uint<B, core::num::NonZero<$uint>>,
        _: Token,
      ) -> View<'_, Uint<B, Self>> {
        View::Owned(Uint::wrap(nonzero.repr.get()))
      }

      #[inline]
      fn known_nonzero(self, _: Token) -> core::num::NonZero<$uint> {
        int::$uint::known_nonzero(self)
      }

      #[inline]
      fn word(&self, index: usize, _: Token) -> u64 {
        int::$uint::word(*self, index)
      }

      #[inline]
      fn from_words(word: impl Fn(usize) -> u64, bits: u32, _: Token) -> Self {
        int::$uint::zero_extend(u128_of_words(word, $uint::BITS) as $uint, bits)
      }

      #[inline]
      fn float_bits(&self, format: FloatFormat, _: Token) -> u64 {
        int::$uint::float_bits(*self, format)
      }

      #[inline]
      fn from_float(
        parts: FloatParts,
        mode: RoundingMode,
        bits: u32,
        _: Token,
      ) -> Result<Self, FromFloatError> {
        int::$uint::place_magnitude(int::float::round_magnitude(parts, mode), bits)
      }

      #[inline]
      fn checked_read_limbs(
        limbs: &[<Self as ReprLimb>::Limb],
        bits: u32,
        _: Token,
      ) -> Option<Self> {
        unsigned::$layer::checked_read_limbs::<1>(limbs, bits).map(|[value]| value)
      }

      #[inline]
      fn wrapping_read_limbs(limbs: &[<Self as ReprLimb>::Limb], bits: u32, _: Token) -> Self {
        let [value] = unsigned::$layer::wrapping_read_limbs::<1>(limbs, bits);
        value
      }

      #[inline]
      fn saturating_read_limbs(limbs: &[<Self as ReprLimb>::Limb], bits: u32, _: Token) -> Self {
        let [value] = unsigned::$layer::saturating_read_limbs::<1>(limbs, bits);
        value
      }
    }

    impl IntRepr for $int {
      type Unsigned = $uint;
      type NonZero = core::num::NonZero<$int>;
    }

    impl IntCore for $int {
      type Magnitude = $uint;
      type NonZeroStorage = core::num::NonZero<$int>;
      type Inline = Self;

      #[inline(always)]
      fn inline(&self, _: Token) -> &Self {
        self
      }

      #[inline(always)]
      fn from_inline(value: Self, _: Token) -> Self {
        value
      }

      #[inline(always)]
      fn set_inline(&mut self, value: Self, _: Token) {
        *self = value;
      }

      #[inline]
      fn zero(_: Token) -> Self {
        0
      }

      #[inline]
      fn compare(&self, other: &Self, _: Token) -> Ordering {
        self.cmp(other)
      }

      #[inline]
      fn compare_u128(&self, rhs: u128, _: Token) -> Ordering {
        int::$int::compare_u128(*self, rhs)
      }

      #[inline]
      fn compare_i128(&self, rhs: i128, _: Token) -> Ordering {
        int::$int::compare_i128(*self, rhs)
      }

      #[inline]
      fn hash_limbs<H: Hasher>(&self, bits: u32, state: &mut H, _: Token) {
        // The bits above the integer read as copies of its sign, as in an array storage.
        let fill = if int::$int::is_negative(*self) { $uint::MAX } else { 0 };
        hash_words(&[self.cast_unsigned()], bits, fill, state);
      }

      #[inline]
      fn fmt_decimal(&self, f: &mut fmt::Formatter<'_>, _: Token) -> fmt::Result {
        fmt::Display::fmt(self, f)
      }

      #[inline]
      fn fmt_pow2(
        &self,
        bits: u32,
        shift: u32,
        upper: bool,
        f: &mut fmt::Formatter<'_>,
        _: Token,
      ) -> fmt::Result {
        fmt_native_pow2(&int::$int::cast_unsigned_bits(*self, bits), shift, upper, f)
      }

      #[inline]
      fn fmt_exp(&self, upper: bool, f: &mut fmt::Formatter<'_>, _: Token) -> fmt::Result {
        if upper {
          fmt::UpperExp::fmt(self, f)
        } else {
          fmt::LowerExp::fmt(self, f)
        }
      }

      #[inline]
      fn from_str_radix_bits(
        src: &[u8],
        radix: u32,
        bits: u32,
        _: Token,
      ) -> Result<Self, ParseIntError> {
        int::$int::from_str_radix_bits(src, radix, bits)
      }

      #[inline]
      fn write_bytes(&self, out: &mut [u8], big_endian: bool, _: Token) {
        int::$int::write_bytes(*self, out, big_endian);
      }

      #[inline]
      fn checked_read_bytes(
        bytes: &[u8],
        big_endian: bool,
        bits: u32,
        _: Token,
      ) -> Option<Self> {
        int::$int::checked_read_bytes(bytes, big_endian, bits)
      }

      #[inline]
      fn wrapping_read_bytes(bytes: &[u8], big_endian: bool, bits: u32, _: Token) -> Self {
        int::$int::wrapping_read_bytes(bytes, big_endian, bits)
      }

      #[inline]
      fn saturating_read_bytes(bytes: &[u8], big_endian: bool, bits: u32, _: Token) -> Self {
        int::$int::saturating_read_bytes(bytes, big_endian, bits)
      }

      #[inline]
      fn byte_at(&self, index: usize, _: Token) -> u8 {
        int::$int::byte_at(*self, index)
      }

      #[inline]
      fn swap_bytes_bits(&self, bits: u32, _: Token) -> Self {
        int::$int::swap_bytes_bits(*self, bits)
      }

      #[inline]
      fn is_zero(&self, _: Token) -> bool {
        int::$int::is_zero(*self)
      }

      #[inline]
      fn fits_bits(&self, bits: u32, _: Token) -> bool {
        int::$int::fits_bits(*self, bits)
      }

      #[inline]
      fn is_negative(&self, _: Token) -> bool {
        int::$int::is_negative(*self)
      }

      #[inline]
      fn is_positive(&self, _: Token) -> bool {
        int::$int::is_positive(*self)
      }

      #[inline]
      fn overflowing_add_bits(&self, rhs: &Self, bits: u32, _: Token) -> (Self, bool) {
        int::$int::overflowing_add_bits(*self, *rhs, bits)
      }

      #[inline]
      fn overflowing_sub_bits(&self, rhs: &Self, bits: u32, _: Token) -> (Self, bool) {
        int::$int::overflowing_sub_bits(*self, *rhs, bits)
      }

      #[inline]
      fn overflowing_mul_bits(&self, rhs: &Self, bits: u32, _: Token) -> (Self, bool) {
        int::$int::overflowing_mul_bits(*self, *rhs, bits)
      }

      #[inline]
      fn overflowing_neg_bits(&self, bits: u32, _: Token) -> (Self, bool) {
        int::$int::overflowing_neg_bits(*self, bits)
      }

      #[inline]
      fn is_one(&self, _: Token) -> bool {
        int::$int::is_one(*self)
      }

      // The layer is named by its path: method syntax on `self`, a `&$int`, would find these
      // methods, which take `&self`, before the inherent methods of the same names.
      #[inline]
      fn signum(&self, _: Token) -> Self {
        int::$int::signum(*self)
      }

      #[inline]
      fn unsigned_abs(&self, _: Token) -> $uint {
        int::$int::unsigned_abs(*self)
      }

      #[inline]
      fn div_rem_bits(&self, rhs: &Self, bits: u32, _: Token) -> Option<(Self, Self, bool)> {
        int::$int::div_rem_bits(*self, *rhs, bits)
      }

      #[inline]
      fn div_rem_euclid_bits(&self, rhs: &Self, bits: u32, _: Token) -> Option<(Self, Self, bool)> {
        int::$int::div_rem_euclid_bits(*self, *rhs, bits)
      }

      #[inline]
      fn div_floor_bits(&self, rhs: &Self, bits: u32, _: Token) -> Option<(Self, bool)> {
        int::$int::div_floor_bits(*self, *rhs, bits)
      }

      #[inline]
      fn div_ceil_bits(&self, rhs: &Self, bits: u32, _: Token) -> Option<(Self, bool)> {
        int::$int::div_ceil_bits(*self, *rhs, bits)
      }

      #[inline]
      fn next_multiple_of_bits(&self, rhs: &Self, bits: u32, _: Token) -> Option<(Self, bool)> {
        int::$int::next_multiple_of_bits(*self, *rhs, bits)
      }

      #[inline]
      fn checked_div_exact_bits(&self, rhs: &Self, bits: u32, _: Token) -> Option<Self> {
        int::$int::checked_div_exact_bits(*self, *rhs, bits)
      }

      #[inline]
      fn overflowing_pow_bits(&self, exp: u32, bits: u32, _: Token) -> (Self, bool) {
        int::$int::overflowing_pow_bits(*self, exp, bits)
      }

      #[inline]
      fn checked_isqrt(&self, _: Token) -> Option<Self> {
        int::$int::checked_isqrt(*self)
      }

      #[inline]
      fn checked_nth_root(&self, n: u32, _: Token) -> Option<Self> {
        int::$int::checked_nth_root(*self, n)
      }

      #[inline]
      fn checked_ilog(&self, base: &Self, _: Token) -> Option<u32> {
        int::$int::checked_ilog(*self, *base)
      }

      #[inline]
      fn checked_ilog2(&self, _: Token) -> Option<u32> {
        int::$int::checked_ilog2(*self)
      }

      #[inline]
      fn checked_ilog10(&self, _: Token) -> Option<u32> {
        int::$int::checked_ilog10(*self)
      }

      #[inline]
      fn midpoint(&self, rhs: &Self, _: Token) -> Self {
        int::$int::midpoint(*self, *rhs)
      }

      #[inline]
      fn abs_diff(&self, rhs: &Self, _: Token) -> $uint {
        int::$int::abs_diff(*self, *rhs)
      }

      #[inline]
      fn and(&self, rhs: &Self, _: Token) -> Self {
        int::$int::and(*self, *rhs)
      }

      #[inline]
      fn or(&self, rhs: &Self, _: Token) -> Self {
        int::$int::or(*self, *rhs)
      }

      #[inline]
      fn xor(&self, rhs: &Self, _: Token) -> Self {
        int::$int::xor(*self, *rhs)
      }

      #[inline]
      fn not_bits(&self, bits: u32, _: Token) -> Self {
        int::$int::not_bits(*self, bits)
      }

      #[inline]
      fn count_ones_bits(&self, bits: u32, _: Token) -> u32 {
        int::$int::count_ones_bits(*self, bits)
      }

      #[inline]
      fn leading_zeros_bits(&self, bits: u32, _: Token) -> u32 {
        int::$int::leading_zeros_bits(*self, bits)
      }

      #[inline]
      fn trailing_zeros_bits(&self, bits: u32, _: Token) -> u32 {
        int::$int::trailing_zeros_bits(*self, bits)
      }

      #[inline]
      fn leading_ones_bits(&self, bits: u32, _: Token) -> u32 {
        int::$int::leading_ones_bits(*self, bits)
      }

      #[inline]
      fn trailing_ones_bits(&self, bits: u32, _: Token) -> u32 {
        int::$int::trailing_ones_bits(*self, bits)
      }

      #[inline]
      fn highest_one_bits(&self, bits: u32, _: Token) -> Option<u32> {
        int::$int::highest_one_bits(*self, bits)
      }

      #[inline]
      fn lowest_one(&self, _: Token) -> Option<u32> {
        int::$int::lowest_one(*self)
      }

      #[inline]
      fn isolate_highest_one_bits(&self, bits: u32, _: Token) -> Self {
        int::$int::isolate_highest_one_bits(*self, bits)
      }

      #[inline]
      fn isolate_lowest_one_bits(&self, bits: u32, _: Token) -> Self {
        int::$int::isolate_lowest_one_bits(*self, bits)
      }

      #[inline]
      fn reverse_bits_bits(&self, bits: u32, _: Token) -> Self {
        int::$int::reverse_bits_bits(*self, bits)
      }

      #[inline]
      fn rotate_left_bits(&self, n: u32, bits: u32, _: Token) -> Self {
        int::$int::rotate_left_bits(*self, n, bits)
      }

      #[inline]
      fn rotate_right_bits(&self, n: u32, bits: u32, _: Token) -> Self {
        int::$int::rotate_right_bits(*self, n, bits)
      }

      #[inline]
      fn overflowing_shl_bits(&self, rhs: u32, bits: u32, _: Token) -> (Self, bool) {
        int::$int::overflowing_shl_bits(*self, rhs, bits)
      }

      #[inline]
      fn overflowing_shr_bits(&self, rhs: u32, bits: u32, _: Token) -> (Self, bool) {
        int::$int::overflowing_shr_bits(*self, rhs, bits)
      }

      #[inline]
      fn unbounded_shl_bits(&self, rhs: u32, bits: u32, _: Token) -> Self {
        int::$int::unbounded_shl_bits(*self, rhs, bits)
      }

      #[inline]
      fn unbounded_shr_bits(&self, rhs: u32, bits: u32, _: Token) -> Self {
        int::$int::unbounded_shr_bits(*self, rhs, bits)
      }

      #[inline]
      fn shl_exact_bits(&self, rhs: u32, bits: u32, _: Token) -> Option<Self> {
        int::$int::shl_exact_bits(*self, rhs, bits)
      }

      #[inline]
      fn shr_exact_bits(&self, rhs: u32, bits: u32, _: Token) -> Option<Self> {
        int::$int::shr_exact_bits(*self, rhs, bits)
      }

      #[inline]
      fn bit(&self, index: u32, bits: u32, _: Token) -> bool {
        int::$int::bit(*self, index, bits)
      }

      #[inline]
      fn with_bit(&self, index: u32, set: bool, bits: u32, _: Token) -> Option<Self> {
        int::$int::with_bit(*self, index, set, bits)
      }

      #[inline]
      fn cast_unsigned_bits(self, bits: u32, _: Token) -> $uint {
        int::$int::cast_unsigned_bits(self, bits)
      }

      #[inline]
      fn saturating_cast_unsigned_bits(self, bits: u32, _: Token) -> $uint {
        int::$int::saturating_cast_unsigned_bits(self, bits)
      }

      #[inline]
      fn to_nonzero(self, _: Token) -> Option<core::num::NonZero<$int>> {
        core::num::NonZero::new(self)
      }

      #[inline]
      fn from_nonzero(nonzero: core::num::NonZero<$int>, _: Token) -> Self {
        nonzero.get()
      }

      #[inline]
      fn with_nonzero<T>(
        nonzero: &core::num::NonZero<$int>,
        f: impl FnOnce(&Self) -> T,
        _: Token,
      ) -> T {
        f(&nonzero.get())
      }

      #[inline]
      fn lend_nonzero<B>(
        nonzero: &Int<B, core::num::NonZero<$int>>,
        _: Token,
      ) -> View<'_, Int<B, Self>> {
        View::Owned(Int::wrap(nonzero.repr.get()))
      }

      #[inline]
      fn known_nonzero(self, _: Token) -> core::num::NonZero<$int> {
        int::$int::known_nonzero(self)
      }

      #[inline]
      fn word(&self, index: usize, _: Token) -> u64 {
        int::$int::word(*self, index)
      }

      #[inline]
      fn from_words(word: impl Fn(usize) -> u64, bits: u32, _: Token) -> Self {
        int::$int::sign_extend(u128_of_words(word, $int::BITS) as $int, bits)
      }

      #[inline]
      fn float_bits(&self, format: FloatFormat, _: Token) -> u64 {
        int::$int::float_bits(*self, format)
      }

      #[inline]
      fn from_float(
        parts: FloatParts,
        mode: RoundingMode,
        bits: u32,
        _: Token,
      ) -> Result<Self, FromFloatError> {
        int::$int::place_magnitude(int::float::round_magnitude(parts, mode), bits)
      }

      // The limb holds the bits of the two's complement, which the cast reads with the sign.
      #[inline]
      fn checked_read_limbs(
        limbs: &[<Self as ReprLimb>::Limb],
        bits: u32,
        _: Token,
      ) -> Option<Self> {
        signed::$layer::checked_read_limbs::<1>(limbs, bits).map(|[limb]| limb.cast_signed())
      }

      #[inline]
      fn wrapping_read_limbs(limbs: &[<Self as ReprLimb>::Limb], bits: u32, _: Token) -> Self {
        let [limb] = signed::$layer::wrapping_read_limbs::<1>(limbs, bits);
        limb.cast_signed()
      }

      #[inline]
      fn saturating_read_limbs(limbs: &[<Self as ReprLimb>::Limb], bits: u32, _: Token) -> Self {
        let [limb] = signed::$layer::saturating_read_limbs::<1>(limbs, bits);
        limb.cast_signed()
      }
    }

    impl ReprLimb for $uint {
      type Limb = $uint;
    }

    impl ReprLimb for $int {
      type Limb = $uint;
    }

    impl NonZeroRepr for core::num::NonZero<$uint> {}

    impl NonZeroRepr for core::num::NonZero<$int> {}

    impl NonZeroUintRepr for core::num::NonZero<$uint> {
      type Integer = $uint;
      type Signed = core::num::NonZero<$int>;
    }

    impl NonZeroIntRepr for core::num::NonZero<$int> {
      type Integer = $int;
      type Unsigned = core::num::NonZero<$uint>;
    }

    impl MutableRepr for $uint {}

    impl MutableRepr for $int {}

    impl UintReprMut for $uint {}

    impl IntReprMut for $int {}

    impl InlineUintRepr for $uint {}

    impl InlineUintConsts for $uint {
      const ZERO: Self = 0;
      const ONE: Self = 1;

      type Width<B: FitsBits<Self>> = Width<B, Self>;
    }

    impl<B: FitsBits<$uint>> UintWidth<$uint> for Width<B, $uint> {
      const MAX: $uint = int::$uint::max_of_bits(B::U32);
      const NONZERO_MIN: core::num::NonZero<$uint> = core::num::NonZero::<$uint>::MIN;
      const NONZERO_MAX: core::num::NonZero<$uint> =
        int::$uint::known_nonzero(int::$uint::max_of_bits(B::U32));
    }

    impl InlineIntRepr for $int {}

    impl InlineIntConsts for $int {
      const ZERO: Self = 0;
      const ONE: Self = 1;

      type Width<B: FitsBits<Self>> = Width<B, Self>;
    }

    impl<B: FitsBits<$int>> IntWidth<$int> for Width<B, $int> {
      const MIN: $int = int::$int::min_of_bits(B::U32);
      const MAX: $int = int::$int::max_of_bits(B::U32);
      const NONZERO_MIN: core::num::NonZero<$int> =
        int::$int::known_nonzero(int::$int::min_of_bits(B::U32));
      // One bit holds -1 and 0, so its largest non-zero value is -1.
      const NONZERO_MAX: core::num::NonZero<$int> = int::$int::known_nonzero(if B::U32 == 1 {
        int::$int::min_of_bits(B::U32)
      } else {
        int::$int::max_of_bits(B::U32)
      });
    }
  )+};
  (@width $native:ident, $bits:ident) => {
    impl<B> BitsOf<$native> for B
    where
      B: IsLessOrEqual<$bits>,
      LeEq<B, $bits>: NativeBitsFit,
    {
    }

    impl<B> BitsFill<$native> for B
    where
      B: IsEqual<$bits>,
      <B as IsEqual<$bits>>::Output: NativeReprFilled,
    {
    }
  };
}

native_storage!(
  u8, i8: U8, b8;
  u16, i16: U16, b16;
  u32, i32: U32, b32;
  u64, i64: U64, b64;
  u128, i128: U128, b128;
);

/// The array of limbs of type `Self` that holds `Bits` bits: `ceil(Bits / Self::BITS)` limbs.
///
/// Implemented by `u8`, `u16`, `u32`, `u64` and `u128` for every width whose number of limbs has an
/// [`ArraySize`], from 0 bits up to 8192, 16384, 32768, 65536 and 131072 bits. [`Limbs`] names the
/// array. This trait is sealed and cannot be implemented outside this crate.
#[diagnostic::on_unimplemented(
  message = "`{Self}` is not a limb type of an array storage",
  label = "not a limb type",
  note = "the limb types are `u8`, `u16`, `u32`, `u64` and `u128`"
)]
pub trait LimbArray<Bits>: LimbFor<Bits> {
  /// The number of limbs, `ceil(Bits / Self::BITS)`.
  type Count: ArraySize;

  /// The array, `[Self; Count]`.
  type Array;
}

/// The array of limbs of type `L` that holds `Bits` bits, `[L; ceil(Bits / L::BITS)]`:
/// `Limbs<U200>` is `[u64; 4]` and `Limbs<U24, u8>` is `[u8; 3]`.
pub type Limbs<Bits, L = u64> = <L as LimbArray<Bits>>::Array;

/// Implements [`LimbArray`] for each limb type, written `limb: BitsLog2, mask`, where `mask` is
/// `2^BitsLog2 - 1`: the number of limbs is `(Bits + mask) >> BitsLog2`.
// Each limb type has an impl of its own, as it has width impls of its own in `array_storage!`. One
// impl over a trait of the limb types would compute with the logarithm of their bits, which for a
// type that is not a limb reaches the recursive `Shl` impls of `typenum` and overflows the trait
// solver, instead of failing with the diagnostic of this trait. The masks are written out, because
// rustc does not match a where-clause such as `Bits: Add<Sub1<Shleft<U1, U6>>>` against the
// normalized `Bits: Add<U63>`.
macro_rules! limb_arrays {
  ($($limb:ident: $bits_log2:ident, $mask:ident);+ $(;)?) => {$(
    impl<Bits: Unsigned> LimbFor<Bits> for $limb {}

    impl<Bits> LimbArray<Bits> for $limb
    where
      Bits: Unsigned + Canonical + Add<$mask>,
      Sum<Bits, $mask>: Shr<$bits_log2>,
      Shright<Sum<Bits, $mask>, $bits_log2>: ArraySize,
    {
      type Count = Shright<Sum<Bits, $mask>, $bits_log2>;
      type Array = <Shright<Sum<Bits, $mask>, $bits_log2> as ArraySize>::ArrayType<$limb>;
    }
  )+};
}

limb_arrays!(
  u8: U3, U7;
  u16: U4, U15;
  u32: U5, U31;
  u64: U6, U63;
  u128: U7, U127;
);

/// Maps a [`typenum`] number of bits to the default storages of [`Uint`](crate::Uint) and
/// [`Int`](crate::Int): the zero-sized `[u64; 0]` for zero bits, the smallest native integer that
/// holds 1 to 128 bits, and above 128 bits an array of 64-bit limbs.
///
/// | Bits | `Unsigned` | `Signed` |
/// |---|---|---|
/// | 0 | `[u64; 0]` | `[u64; 0]` |
/// | 1-8 | `u8` | `i8` |
/// | 9-16 | `u16` | `i16` |
/// | 17-32 | `u32` | `i32` |
/// | 33-64 | `u64` | `i64` |
/// | 65-128 | `u128` | `i128` |
/// | 129-65536 | [`Limbs<Bits>`](Limbs), `[u64; ceil(Bits / 64)]` | the same array |
///
/// Implemented for every number from 0 to 65536, written as a constant such as `U200`, as `U<200>`,
/// or as an expression such as `Sum<U1024, U1>`; `typenum` names each number up to 1024 and some
/// larger ones. `Uint<U7>` is `Uint<U7, u8>` and `Uint<U200>` is `Uint<U200, [u64; 4]>`. The
/// default is stored inline, so `Uint<U65536>` takes 8 KiB and `Uint<U0>` takes no space. More than
/// 65536 bits have no default; such a type names its storage. `NonZeroUnsigned` and
/// `NonZeroSigned` are the [`NonZero`](UintRepr::NonZero) storages of `Unsigned` and `Signed`, the
/// defaults of [`NonZeroUint`](crate::NonZeroUint) and [`NonZeroInt`](crate::NonZeroInt):
/// `core::num::NonZero<u8>` for `u8`, and so on, and the same array, so `NonZeroUint<U7>` is
/// `NonZeroUint<U7, NonZero<u8>>`.
///
/// Generic code with the bound `B: BitWidth` gets `Copy`, `Clone`, `Eq`, `Ord`, `Hash`, `Debug`,
/// `Display` and `Default` on `Uint<B>` and `Int<B>`, their constants `BITS`, `ZERO`, `MIN` and
/// `MAX`, as in `Uint::<B>::MAX`, the operators, and the integer traits, such as
/// [`FixedInteger`](crate::FixedInteger), [`ConstFixedInteger`](crate::ConstFixedInteger) and
/// [`CheckedOps`](crate::CheckedOps), whose methods it calls with the trait in scope. `ONE` and
/// the traits [`IntegerOne`](crate::IntegerOne) and [`ConstIntegerOne`](crate::ConstIntegerOne)
/// need a width that holds one: `B: NonZero` for `Uint<B>`, and `B: IsGreater<U1, Output = True>`
/// for `Int<B>`. On `NonZeroUint<B>` and `NonZeroInt<B>` it gets `Copy`, `Clone`, `Eq`, `Ord`,
/// `Hash`, `Debug` and `Display`, `BITS`, the operators, [`FixedNonZero`](crate::FixedNonZero)
/// and the trait of each sign, whose casts turn `NonZeroUint<B>` into `NonZeroInt<B>` and back,
/// and with `B: NonZero` the constants `MIN` and `MAX`, [`NonZeroLimits`](crate::NonZeroLimits)
/// and [`ConstNonZeroLimits`](crate::ConstNonZeroLimits). The methods of the traits are not
/// `const`, and the inherent `const fn`s belong to each storage, so a constant context calls them
/// on a concrete type, such as `Uint<U7>`, or an alias. This trait is sealed and cannot be
/// implemented outside this crate.
///
/// ```
/// use numo::{
///   BitWidth, Int, Uint,
///   typenum::{Sum, U0, U1, U7, U24, U200, U1024},
/// };
///
/// assert_eq!(Uint::<U7>::MAX, Uint::<U7, u8>::MAX);
/// assert_eq!(size_of::<Int<U24>>(), 4);
/// assert_eq!(size_of::<Uint<U0>>(), 0);
/// assert_eq!(Uint::<U200>::MAX.to_limbs(), [u64::MAX, u64::MAX, u64::MAX, 0xff]);
/// assert_eq!(size_of::<Uint<Sum<U1024, U1>>>(), 17 * 8);
///
/// fn largest<B: BitWidth>(values: &[Uint<B>]) -> Uint<B> {
///   values.iter().copied().fold(Uint::<B>::MIN, Ord::max)
/// }
///
/// let values = [Uint::<U200>::MAX, Uint::<U200>::default()];
/// assert_eq!(largest(&values), Uint::<U200>::MAX);
/// ```
#[diagnostic::on_unimplemented(
  message = "`Uint` and `Int` have a default storage only for 0 to 65536 bits",
  label = "`Bits` is not known to be from 0 to 65536",
  note = "generic code adds the bound `B: BitWidth`, as in `fn f<B: BitWidth>(x: Uint<B>)`, or names the storage, as in `Uint<B, [u64; 4]>`"
)]
pub trait BitWidth:
  TypenumNumber + FitsBits<<Self as BitWidth>::Unsigned> + FitsBits<<Self as BitWidth>::Signed>
{
  /// The default storage of `Uint<Self>`.
  type Unsigned: InlineUintRepr
    + UintRepr<Signed = <Self as BitWidth>::Signed, NonZero = <Self as BitWidth>::NonZeroUnsigned>;

  /// The default storage of `Int<Self>`, of the same size as `Unsigned`.
  type Signed: InlineIntRepr
    + IntRepr<Unsigned = <Self as BitWidth>::Unsigned, NonZero = <Self as BitWidth>::NonZeroSigned>;

  /// The default storage of the non-zero unsigned integer of `Self` bits, the
  /// [`NonZero`](UintRepr::NonZero) of `Unsigned`: `[u64; 0]` for zero bits,
  /// `core::num::NonZero<u8>` to `NonZero<u128>` for 1 to 128 bits, and the array of `Unsigned`
  /// above. It is `Copy`, and its [`Signed`](NonZeroUintRepr::Signed) is `NonZeroSigned`, whose
  /// [`Unsigned`](NonZeroIntRepr::Unsigned) is this storage again, so that generic code over the
  /// width casts between the two.
  type NonZeroUnsigned: NonZeroUintRepr<
      Integer = <Self as BitWidth>::Unsigned,
      Signed = <Self as BitWidth>::NonZeroSigned,
    > + Copy;

  /// The default storage of the non-zero signed integer of `Self` bits, the
  /// [`NonZero`](IntRepr::NonZero) of `Signed`: `[u64; 0]` for zero bits,
  /// `core::num::NonZero<i8>` to `NonZero<i128>` for 1 to 128 bits, and the array of `Signed`
  /// above. It is `Copy`, and its [`Unsigned`](NonZeroIntRepr::Unsigned) is `NonZeroUnsigned`.
  type NonZeroSigned: NonZeroIntRepr<
      Integer = <Self as BitWidth>::Signed,
      Unsigned = <Self as BitWidth>::NonZeroUnsigned,
    > + Copy;
}

// The impls are on the numbers of `typenum`, `UTerm` and `UInt`, rather than over every type, so
// that a generic width without the bound reports the diagnostic of this trait and the suggestion to
// add it, not an unsatisfied comparison.
impl BitWidth for UTerm {
  type Unsigned = [u64; 0];
  type Signed = [u64; 0];
  type NonZeroUnsigned = [u64; 0];
  type NonZeroSigned = [u64; 0];
}

impl<U, X> BitWidth for UInt<U, X>
where
  Self: Canonical + IsLessOrEqual<U128> + DefaultRepr<LeEq<Self, U128>>,
{
  type Unsigned = <Self as DefaultRepr<LeEq<Self, U128>>>::Unsigned;
  type Signed = <Self as DefaultRepr<LeEq<Self, U128>>>::Signed;
  type NonZeroUnsigned = <<Self as DefaultRepr<LeEq<Self, U128>>>::Unsigned as UintRepr>::NonZero;
  type NonZeroSigned = <<Self as DefaultRepr<LeEq<Self, U128>>>::Signed as IntRepr>::NonZero;
}

impl<B> DefaultRepr<B0> for B
where
  B: IsLessOrEqual<U65536>,
  LeEq<B, U65536>: DefaultArray<B>,
  B: FitsBits<<LeEq<B, U65536> as DefaultArray<B>>::Array>,
{
  type Unsigned = <LeEq<B, U65536> as DefaultArray<B>>::Array;
  type Signed = <LeEq<B, U65536> as DefaultArray<B>>::Array;
}

impl<B> DefaultArray<B> for B1
where
  u64: LimbArray<B>,
  Limbs<B>: InlineUintRepr
    + InlineIntRepr
    + IntRepr<Unsigned = Limbs<B>, NonZero = Limbs<B>>
    + UintRepr<Signed = Limbs<B>, NonZero = Limbs<B>>,
{
  type Array = Limbs<B>;
}

/// Passes the native rows of the default storages to `$callback`, written
/// `Bits = value => unsigned, signed`, where `unsigned` and `signed` are the smallest native
/// integers that hold `Bits` bits.
macro_rules! bit_table {
  ($callback:ident) => {
    $callback! {
      U1 = 1 => u8, i8,
      U2 = 2 => u8, i8,
      U3 = 3 => u8, i8,
      U4 = 4 => u8, i8,
      U5 = 5 => u8, i8,
      U6 = 6 => u8, i8,
      U7 = 7 => u8, i8,
      U8 = 8 => u8, i8,
      U9 = 9 => u16, i16,
      U10 = 10 => u16, i16,
      U11 = 11 => u16, i16,
      U12 = 12 => u16, i16,
      U13 = 13 => u16, i16,
      U14 = 14 => u16, i16,
      U15 = 15 => u16, i16,
      U16 = 16 => u16, i16,
      U17 = 17 => u32, i32,
      U18 = 18 => u32, i32,
      U19 = 19 => u32, i32,
      U20 = 20 => u32, i32,
      U21 = 21 => u32, i32,
      U22 = 22 => u32, i32,
      U23 = 23 => u32, i32,
      U24 = 24 => u32, i32,
      U25 = 25 => u32, i32,
      U26 = 26 => u32, i32,
      U27 = 27 => u32, i32,
      U28 = 28 => u32, i32,
      U29 = 29 => u32, i32,
      U30 = 30 => u32, i32,
      U31 = 31 => u32, i32,
      U32 = 32 => u32, i32,
      U33 = 33 => u64, i64,
      U34 = 34 => u64, i64,
      U35 = 35 => u64, i64,
      U36 = 36 => u64, i64,
      U37 = 37 => u64, i64,
      U38 = 38 => u64, i64,
      U39 = 39 => u64, i64,
      U40 = 40 => u64, i64,
      U41 = 41 => u64, i64,
      U42 = 42 => u64, i64,
      U43 = 43 => u64, i64,
      U44 = 44 => u64, i64,
      U45 = 45 => u64, i64,
      U46 = 46 => u64, i64,
      U47 = 47 => u64, i64,
      U48 = 48 => u64, i64,
      U49 = 49 => u64, i64,
      U50 = 50 => u64, i64,
      U51 = 51 => u64, i64,
      U52 = 52 => u64, i64,
      U53 = 53 => u64, i64,
      U54 = 54 => u64, i64,
      U55 = 55 => u64, i64,
      U56 = 56 => u64, i64,
      U57 = 57 => u64, i64,
      U58 = 58 => u64, i64,
      U59 = 59 => u64, i64,
      U60 = 60 => u64, i64,
      U61 = 61 => u64, i64,
      U62 = 62 => u64, i64,
      U63 = 63 => u64, i64,
      U64 = 64 => u64, i64,
      U65 = 65 => u128, i128,
      U66 = 66 => u128, i128,
      U67 = 67 => u128, i128,
      U68 = 68 => u128, i128,
      U69 = 69 => u128, i128,
      U70 = 70 => u128, i128,
      U71 = 71 => u128, i128,
      U72 = 72 => u128, i128,
      U73 = 73 => u128, i128,
      U74 = 74 => u128, i128,
      U75 = 75 => u128, i128,
      U76 = 76 => u128, i128,
      U77 = 77 => u128, i128,
      U78 = 78 => u128, i128,
      U79 = 79 => u128, i128,
      U80 = 80 => u128, i128,
      U81 = 81 => u128, i128,
      U82 = 82 => u128, i128,
      U83 = 83 => u128, i128,
      U84 = 84 => u128, i128,
      U85 = 85 => u128, i128,
      U86 = 86 => u128, i128,
      U87 = 87 => u128, i128,
      U88 = 88 => u128, i128,
      U89 = 89 => u128, i128,
      U90 = 90 => u128, i128,
      U91 = 91 => u128, i128,
      U92 = 92 => u128, i128,
      U93 = 93 => u128, i128,
      U94 = 94 => u128, i128,
      U95 = 95 => u128, i128,
      U96 = 96 => u128, i128,
      U97 = 97 => u128, i128,
      U98 = 98 => u128, i128,
      U99 = 99 => u128, i128,
      U100 = 100 => u128, i128,
      U101 = 101 => u128, i128,
      U102 = 102 => u128, i128,
      U103 = 103 => u128, i128,
      U104 = 104 => u128, i128,
      U105 = 105 => u128, i128,
      U106 = 106 => u128, i128,
      U107 = 107 => u128, i128,
      U108 = 108 => u128, i128,
      U109 = 109 => u128, i128,
      U110 = 110 => u128, i128,
      U111 = 111 => u128, i128,
      U112 = 112 => u128, i128,
      U113 = 113 => u128, i128,
      U114 = 114 => u128, i128,
      U115 = 115 => u128, i128,
      U116 = 116 => u128, i128,
      U117 = 117 => u128, i128,
      U118 = 118 => u128, i128,
      U119 = 119 => u128, i128,
      U120 = 120 => u128, i128,
      U121 = 121 => u128, i128,
      U122 = 122 => u128, i128,
      U123 = 123 => u128, i128,
      U124 = 124 => u128, i128,
      U125 = 125 => u128, i128,
      U126 = 126 => u128, i128,
      U127 = 127 => u128, i128,
      U128 = 128 => u128, i128,
    }
  };
}

macro_rules! impl_bits {
  ($($bits:ident = $value:literal => $uint:ident, $int:ident),+ $(,)?) => {$(
    impl DefaultRepr<B1> for typenum::consts::$bits {
      type Unsigned = $uint;
      type Signed = $int;
    }
  )+};
}

bit_table!(impl_bits);

/// Writes the lowest `ceil(bits / 64)` 64-bit words of the integer in `limbs` to `state`, least
/// significant first, without the length prefix of a slice. For limbs of `B <= 64` bits, each word
/// joins `64 / B` limbs, the least significant first, and a limb at index `limbs.len()` or above
/// reads as `fill`. A limb of 128 bits holds two words, the low one first, and `N` such limbs hold
/// every word of `bits <= 128 * N` bits, so they never read `fill`.
///
/// Every storage of a value of `bits` bits writes the same words, so the hash does not depend on the
/// limb type or the number of limbs.
fn hash_words<L: Copy + Into<u128>, H: Hasher>(limbs: &[L], bits: u32, fill: L, state: &mut H) {
  let limb_bits = 8 * size_of::<L>() as u32;
  for word in 0..bits.div_ceil(64) as usize {
    // A 128-bit limb comes first, since `64 / 128` limbs per word would write zero words.
    let value = if limb_bits == 128 {
      limbs[word / 2].into() >> (64 * (word % 2))
    } else {
      let per_word = (u64::BITS / limb_bits) as usize;
      let mut value = 0;
      for k in 0..per_word {
        let limb = limbs.get(word * per_word + k).copied().unwrap_or(fill);
        value |= limb.into() << (limb_bits * k as u32);
      }
      value
    };
    // The low 64 bits: a word of narrower limbs has no others, and the high half of a 128-bit limb
    // is the next word.
    state.write_u64(value as u64);
  }
}

/// The `N` limbs of type `L`, least significant first, of the integer whose 64-bit words `word`
/// gives, least significant first, as [`hash_words`] joins the limbs into words: for limbs of
/// `B <= 64` bits, limb `i` holds the bits of word `i / (64 / B)` from `B * (i % (64 / B))` up,
/// and a limb of 128 bits joins two words, the low one first. `limb` keeps the low bits of its
/// argument as a limb.
fn limbs_of_words<L, const N: usize>(
  word: impl Fn(usize) -> u64,
  limb: impl Fn(u128) -> L,
) -> [L; N] {
  let limb_bits = 8 * size_of::<L>();
  core::array::from_fn(|i| {
    if limb_bits == 128 {
      limb(u128::from(word(2 * i)) | u128::from(word(2 * i + 1)) << 64)
    } else {
      let per_word = 64 / limb_bits;
      limb(u128::from(
        word(i / per_word) >> (limb_bits * (i % per_word)),
      ))
    }
  })
}

/// The integer whose 64-bit words `word` gives, least significant first, cut to the 128 bits of a
/// `u128`, of which a native integer of `bits` bits keeps the low ones: the first word, and above
/// 64 bits the second.
pub(crate) fn u128_of_words(word: impl Fn(usize) -> u64, bits: u32) -> u128 {
  let low = u128::from(word(0));
  if bits > 64 {
    low | u128::from(word(1)) << 64
  } else {
    low
  }
}

/// The result of `write` on the decimal digits of `magnitude`, without leading zeros (zero is a
/// single `0`), which `write_digits_into` writes to a buffer on the stack of `DIGITS` bytes for
/// each limb. Requires each limb to add at most `DIGITS` digits.
fn with_digits<L, const N: usize, const DIGITS: usize, T>(
  magnitude: &[L; N],
  write_digits_into: impl FnOnce(&[L; N], &mut [u8]) -> usize,
  write: impl FnOnce(&[u8]) -> T,
) -> T {
  // No limbs hold only zero, and leave no room in the buffer for its digit.
  if N == 0 {
    return write(b"0");
  }
  let mut buf = [[0; DIGITS]; N];
  let buf = buf.as_flattened_mut();
  let start = write_digits_into(magnitude, buf);
  write(&buf[start..])
}

/// Writes `magnitude` in decimal with `write_digits_into`, negative unless `is_nonnegative`,
/// honoring the flags of `f` as integers do. Requires each limb to add at most `DIGITS` digits.
fn fmt_magnitude<L, const N: usize, const DIGITS: usize>(
  magnitude: &[L; N],
  write_digits_into: impl FnOnce(&[L; N], &mut [u8]) -> usize,
  is_nonnegative: bool,
  f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
  with_digits::<L, N, DIGITS, _>(magnitude, write_digits_into, |digits| {
    f.pad_integral(is_nonnegative, "", format::ascii(digits)?)
  })
}

/// Writes `magnitude`, whose decimal digits `write_digits_into` writes, negative unless
/// `is_nonnegative`, in scientific notation with `E` if `upper` and `e` otherwise, honoring the
/// flags of `f` as integers do. Requires each limb to add at most `DIGITS` digits.
fn fmt_magnitude_exp<L, const N: usize, const DIGITS: usize>(
  magnitude: &[L; N],
  write_digits_into: impl FnOnce(&[L; N], &mut [u8]) -> usize,
  is_nonnegative: bool,
  upper: bool,
  f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
  let marker = if upper { b'E' } else { b'e' };
  with_digits::<L, N, DIGITS, _>(magnitude, write_digits_into, |digits| {
    format::write_integer_exp(f, !is_nonnegative, digits, marker)
  })
}

/// Writes the native unsigned integer `value` in the radix `2^shift` with its own radix format,
/// binary for 1, octal for 3 and hexadecimal for 4, in uppercase if `upper`.
fn fmt_native_pow2<T>(value: &T, shift: u32, upper: bool, f: &mut fmt::Formatter<'_>) -> fmt::Result
where
  T: fmt::Binary + fmt::Octal + fmt::LowerHex + fmt::UpperHex,
{
  match (shift, upper) {
    (1, _) => fmt::Binary::fmt(value, f),
    (3, _) => fmt::Octal::fmt(value, f),
    (_, false) => fmt::LowerHex::fmt(value, f),
    (_, true) => fmt::UpperHex::fmt(value, f),
  }
}

#[cfg(test)]
mod tests;
