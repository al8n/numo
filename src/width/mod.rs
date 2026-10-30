//! The width relation between a type-level number of bits and an integer storage, the storages of
//! [`Uint`](crate::Uint) and [`Int`](crate::Int), and their default storage.

use core::{
  cmp::Ordering,
  fmt,
  hash::Hasher,
  ops::{Add, Shl, Shr},
};

use typenum::{
  B0, B1, Const, IsEqual, IsLess, IsLessOrEqual, Le, LeEq, Shleft, Shright, Sum, ToUInt, U, U3, U4,
  U5, U6, U7, U8, U15, U16, U31, U32, U63, U64, U127, U128, U65536, U4294967296, UInt, UTerm,
  Unsigned,
};

pub use array_size::ArraySize;
pub(crate) use sealed::{IntCore, IntWidth, TypenumNumber, UintCore, UintWidth};

use sealed::{
  ArrayLimb, BitsBelow2Pow32, BitsFill, BitsFit, BitsOf, Canonical, DefaultArray, DefaultRepr,
  InlineIntConsts, InlineUintConsts, LimbFor, MutableRepr, NativeBitsFit, NativeReprFilled,
  ReprFilled, Width,
};

use crate::{
  int::{self, signed, unsigned},
  layout::Token,
};

mod array_size;

mod sealed {
  use core::{cmp::Ordering, fmt, hash::Hasher, marker::PhantomData};

  use typenum::{B1, UInt, UTerm};

  use super::{FitsBits, InlineIntRepr, InlineUintRepr, IntRepr, UintRepr};
  use crate::layout::Token;

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

  /// The constants of `Uint<B, R>` that depend on the width `B`.
  pub trait UintWidth<R> {
    /// The largest value, `2^B - 1`.
    const MAX: R;
  }

  /// The constants of `Int<B, R>` that depend on the width `B`.
  pub trait IntWidth<R> {
    /// The smallest value, `-2^(B - 1)`, or zero for zero bits.
    const MIN: R;

    /// The largest value, `2^(B - 1) - 1`, or zero for zero bits.
    const MAX: R;
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
    type Unsigned: InlineUintRepr;

    /// The default storage of `Int<Self>`.
    type Signed: InlineIntRepr + IntRepr<Unsigned = <Self as DefaultRepr<Small>>::Unsigned>;
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
    type Array: InlineUintRepr + InlineIntRepr + IntRepr<Unsigned = Self::Array>;
  }

  /// A limb type of the array storages: `[L; N]` holds `N << L::BitsLog2` bits.
  pub trait ArrayLimb {
    /// The base-2 logarithm of the number of bits of a limb.
    type BitsLog2;
  }

  /// Seals [`UintRepr`](super::UintRepr) and gives the trait implementations of
  /// [`Uint`](crate::Uint) access to the integer core.
  ///
  /// A bound `R: UintRepr` makes these methods callable from any crate, so each takes a [`Token`].
  /// They rely on the width invariant of `Uint`: the representation, read as an unsigned integer,
  /// equals the value.
  pub trait UintCore: Sized {
    /// Zero.
    fn zero(token: Token) -> Self;

    /// Compares the values of `self` and `other`.
    fn compare(&self, other: &Self, token: Token) -> Ordering;

    /// Writes the lowest `ceil(bits / 64)` 64-bit words of the value to `state`, least significant
    /// first, extended by zeros beyond the storage. Requires the value to have `bits` bits.
    fn hash_limbs<H: Hasher>(&self, bits: u32, state: &mut H, token: Token);

    /// Writes the value in decimal, honoring the flags of `f` as integers do.
    fn fmt_decimal(&self, f: &mut fmt::Formatter<'_>, token: Token) -> fmt::Result;

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

    /// Returns `true` if the value is one.
    fn is_one(&self, token: Token) -> bool;
  }

  /// Seals [`IntRepr`](super::IntRepr) and gives the trait implementations of [`Int`](crate::Int)
  /// access to the integer core.
  ///
  /// As for [`UintCore`], each method takes a [`Token`]. They rely on the width invariant of
  /// `Int`: the representation, read as a two's complement integer, equals the value.
  pub trait IntCore: Sized {
    /// The storage of the absolute value of a value as an unsigned integer, which also holds the
    /// absolute value of the smallest value: [`IntRepr::Unsigned`](super::IntRepr::Unsigned).
    type Magnitude;

    /// Zero.
    fn zero(token: Token) -> Self;

    /// Compares the values of `self` and `other`.
    fn compare(&self, other: &Self, token: Token) -> Ordering;

    /// Writes the lowest `ceil(bits / 64)` 64-bit words of the value in two's complement to
    /// `state`, least significant first, extended by copies of the sign beyond the storage.
    /// Requires the value to have `bits` bits.
    fn hash_limbs<H: Hasher>(&self, bits: u32, state: &mut H, token: Token);

    /// Writes the value in decimal, honoring the flags of `f` as integers do.
    fn fmt_decimal(&self, f: &mut fmt::Formatter<'_>, token: Token) -> fmt::Result;

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

/// `N * B` for limbs of `B` bits, the number of bits of `[L; N]`.
type LimbBits<L, const N: usize> = Shleft<U<N>, <L as ArrayLimb>::BitsLog2>;

// The bound below 2^32 is a comparison of types, because `typenum`'s `U32` and `U64` wrap: for
// `64 * 2^58` bits, both are zero.
impl<B, L: ArrayLimb, const N: usize> BitsOf<[L; N]> for B
where
  B: IsLessOrEqual<LimbBits<L, N>> + IsLess<U4294967296>,
  Const<N>: ToUInt,
  U<N>: Shl<L::BitsLog2>,
  LeEq<B, LimbBits<L, N>>: BitsFit,
  Le<B, U4294967296>: BitsBelow2Pow32,
{
}

// `typenum`'s alias for `IsEqual::Output` is named `Eq`, as the trait of `core` is.
impl<B, L: ArrayLimb, const N: usize> BitsFill<[L; N]> for B
where
  B: IsEqual<LimbBits<L, N>>,
  Const<N>: ToUInt,
  U<N>: Shl<L::BitsLog2>,
  <B as IsEqual<LimbBits<L, N>>>::Output: ReprFilled,
{
}

/// A storage of [`Uint`](crate::Uint): a native unsigned integer, `u8`, `u16`, `u32`, `u64` or
/// `u128`, holds the value itself, and the array `[u8; N]`, `[u16; N]`, `[u32; N]`, `[u64; N]` or
/// `[u128; N]` holds it in `N` limbs of 8, 16, 32, 64 or 128 bits, least significant first.
///
/// `Uint<Bits, Repr>` implements `Ord`, `Hash`, `Display`, `Debug` and `Default`, the arithmetic
/// traits [`OverflowingOps`](crate::OverflowingOps), [`CheckedOps`](crate::CheckedOps),
/// [`WrappingOps`](crate::WrappingOps), [`StrictOps`](crate::StrictOps) and
/// [`UncheckedOps`](crate::UncheckedOps), and the operators `+`, `-` and `*` on two references, as
/// in `&a + &b`, for every `Repr` that implements this trait. This trait is sealed and cannot be
/// implemented outside this crate.
#[diagnostic::on_unimplemented(
  message = "`{Self}` is not a storage of `Uint`",
  note = "`Uint` stores an unsigned native integer, `u8` to `u128`, or an array `[u8; N]` to `[u128; N]`"
)]
pub trait UintRepr: Eq + UintCore {}

/// A storage of [`Int`](crate::Int): a native signed integer, `i8`, `i16`, `i32`, `i64` or `i128`,
/// holds the value itself, and the array `[u8; N]`, `[u16; N]`, `[u32; N]`, `[u64; N]` or
/// `[u128; N]` holds it in `N` limbs of 8, 16, 32, 64 or 128 bits, least significant first, in
/// two's complement.
///
/// `Int<Bits, Repr>` implements `Ord`, `Hash`, `Display`, `Debug` and `Default`, the arithmetic
/// traits [`OverflowingOps`](crate::OverflowingOps), [`CheckedOps`](crate::CheckedOps),
/// [`WrappingOps`](crate::WrappingOps), [`StrictOps`](crate::StrictOps) and
/// [`UncheckedOps`](crate::UncheckedOps), the operators `+`, `-` and `*` on two references, as in
/// `&a + &b`, and the negation `-` of a reference, for every `Repr` that implements this trait.
/// This trait is sealed and cannot be implemented outside this crate.
#[diagnostic::on_unimplemented(
  message = "`{Self}` is not a storage of `Int`",
  note = "`Int` stores a signed native integer, `i8` to `i128`, or an array `[u8; N]` to `[u128; N]` of unsigned limbs in two's complement"
)]
pub trait IntRepr: Eq + IntCore<Magnitude = <Self as IntRepr>::Unsigned> {
  /// The storage of [`Uint`](crate::Uint) of the same size: `u8` for `i8`, `u16` for `i16`, and so
  /// on, and the same array for an array of limbs.
  type Unsigned: UintRepr;
}

/// A storage of [`Uint`](crate::Uint) that holds the value in place, so that the integers have the
/// compound assignment operators `+=`, `-=` and `*=`.
///
/// Every storage of this crate implements it: the native unsigned integers and the arrays of
/// limbs. This trait is sealed and cannot be implemented outside this crate.
pub trait UintReprMut: UintRepr + MutableRepr {}

/// A storage of [`Int`](crate::Int) that holds the value in place, so that the integers have the
/// compound assignment operators `+=`, `-=` and `*=`.
///
/// Every storage of this crate implements it: the native signed integers and the arrays of limbs.
/// This trait is sealed and cannot be implemented outside this crate.
pub trait IntReprMut: IntRepr + MutableRepr {}

/// A storage of [`Uint`](crate::Uint) that holds the value inline, so that the integer is `Copy`: a
/// native unsigned integer, `u8` to `u128`, or an array `[u8; N]` to `[u128; N]`.
///
/// `Uint<Bits, Repr>` has the constants `ZERO`, `MIN` and `MAX`, and implements
/// [`FixedInteger`](crate::FixedInteger) and [`ConstFixedInteger`](crate::ConstFixedInteger), with
/// the operators `+`, `-` and `*` and `Sum`, [`UnsignedFixedInteger`](crate::UnsignedFixedInteger)
/// and [`SaturatingOps`](crate::SaturatingOps), and from one bit it has the constant `ONE` and
/// implements [`IntegerOne`](crate::IntegerOne) and [`ConstIntegerOne`](crate::ConstIntegerOne),
/// with `Product`, for every `Repr` that implements this trait. The default storages of
/// [`BitWidth`] are inline. This trait is sealed and cannot be implemented outside this crate.
pub trait InlineUintRepr: UintReprMut + Copy + InlineUintConsts {}

/// A storage of [`Int`](crate::Int) that holds the value inline, so that the integer is `Copy`: a
/// native signed integer, `i8` to `i128`, or an array `[u8; N]` to `[u128; N]`.
///
/// `Int<Bits, Repr>` has the constants `ZERO`, `MIN` and `MAX`, and implements
/// [`FixedInteger`](crate::FixedInteger) and [`ConstFixedInteger`](crate::ConstFixedInteger), with
/// the operators `+`, `-` and `*` and `Sum`, [`SignedFixedInteger`](crate::SignedFixedInteger),
/// with the operator `-`, and [`SaturatingOps`](crate::SaturatingOps), and from two bits it has the
/// constant `ONE` and implements [`IntegerOne`](crate::IntegerOne) and
/// [`ConstIntegerOne`](crate::ConstIntegerOne), with `Product`, for every `Repr` that implements
/// this trait. The default storages of [`BitWidth`] are inline. This trait is sealed and cannot be
/// implemented outside this crate.
pub trait InlineIntRepr: IntReprMut + Copy + InlineIntConsts {}

/// Implements [`ArrayLimb`], [`UintRepr`], [`IntRepr`], [`UintReprMut`], [`IntReprMut`],
/// [`InlineUintRepr`] and [`InlineIntRepr`] for the arrays of each limb type, written
/// `limb: BitsLog2, lower, upper`, where `lower` and `upper` are the modules of its limb layers.
macro_rules! array_storage {
  ($($limb:ident: $bits_log2:ident, $lower:ident, $upper:ident);+ $(;)?) => {$(
    impl ArrayLimb for $limb {
      type BitsLog2 = $bits_log2;
    }

    impl<const N: usize> UintRepr for [$limb; N] {}

    impl<const N: usize> UintCore for [$limb; N] {
      #[inline]
      fn zero(_: Token) -> Self {
        [0; N]
      }

      #[inline]
      fn compare(&self, other: &Self, _: Token) -> Ordering {
        unsigned::$lower::cmp(self, other)
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
      fn is_one(&self, _: Token) -> bool {
        unsigned::$lower::is_one(self)
      }
    }

    impl<const N: usize> IntRepr for [$limb; N] {
      type Unsigned = Self;
    }

    impl<const N: usize> IntCore for [$limb; N] {
      type Magnitude = Self;

      #[inline]
      fn zero(_: Token) -> Self {
        [0; N]
      }

      #[inline]
      fn compare(&self, other: &Self, _: Token) -> Ordering {
        signed::$upper::cmp(self, other)
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
/// [`InlineIntRepr`] for the native integers, and their width relations, written
/// `unsigned, signed: Bits`, where `Bits` is the number of bits of both.
// Each native type has width impls of its own, as each array type constructor has: one impl over
// every storage, bounded by a trait of the storages, would degrade the diagnostics of the arrays.
macro_rules! native_storage {
  ($($uint:ident, $int:ident: $bits:ident);+ $(;)?) => {$(
    native_storage!(@width $uint, $bits);
    native_storage!(@width $int, $bits);

    impl UintRepr for $uint {}

    impl UintCore for $uint {
      #[inline]
      fn zero(_: Token) -> Self {
        0
      }

      #[inline]
      fn compare(&self, other: &Self, _: Token) -> Ordering {
        self.cmp(other)
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
      fn is_one(&self, _: Token) -> bool {
        int::$uint::is_one(*self)
      }
    }

    impl IntRepr for $int {
      type Unsigned = $uint;
    }

    impl IntCore for $int {
      type Magnitude = $uint;

      #[inline]
      fn zero(_: Token) -> Self {
        0
      }

      #[inline]
      fn compare(&self, other: &Self, _: Token) -> Ordering {
        self.cmp(other)
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
  u8, i8: U8;
  u16, i16: U16;
  u32, i32: U32;
  u64, i64: U64;
  u128, i128: U128;
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
// Each limb type has an impl of its own. One impl over `ArrayLimb` would compute with `BitsLog2`,
// which for a type that is not a limb reaches the recursive `Shl` impls of `typenum` and overflows
// the trait solver, instead of failing with the diagnostic of this trait. The masks are written
// out, because rustc does not match a where-clause such as `Bits: Add<Sub1<Shleft<U1, U6>>>`
// against the normalized `Bits: Add<U63>`.
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
/// 65536 bits have no default; such a type names its storage.
///
/// Generic code with the bound `B: BitWidth` gets `Copy`, `Clone`, `Eq`, `Ord`, `Hash`, `Debug`,
/// `Display` and `Default` on `Uint<B>` and `Int<B>`, their constants `BITS`, `ZERO`, `MIN` and
/// `MAX`, as in `Uint::<B>::MAX`, the operators, and the integer traits, such as
/// [`FixedInteger`](crate::FixedInteger), [`ConstFixedInteger`](crate::ConstFixedInteger) and
/// [`CheckedOps`](crate::CheckedOps), whose methods it calls with the trait in scope. `ONE` and
/// the traits [`IntegerOne`](crate::IntegerOne) and [`ConstIntegerOne`](crate::ConstIntegerOne)
/// need a width that holds one: `B: NonZero` for `Uint<B>`, and `B: IsGreater<U1, Output = True>`
/// for `Int<B>`. The methods of the traits are not `const`, and the inherent `const fn`s belong to
/// each storage, so a constant context calls them on a concrete type, such as `Uint<U7>`, or an
/// alias. This trait is sealed and cannot be implemented outside this crate.
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
  type Unsigned: InlineUintRepr;

  /// The default storage of `Int<Self>`, of the same size as `Unsigned`.
  type Signed: InlineIntRepr + IntRepr<Unsigned = <Self as BitWidth>::Unsigned>;
}

// The impls are on the numbers of `typenum`, `UTerm` and `UInt`, rather than over every type, so
// that a generic width without the bound reports the diagnostic of this trait and the suggestion to
// add it, not an unsatisfied comparison.
impl BitWidth for UTerm {
  type Unsigned = [u64; 0];
  type Signed = [u64; 0];
}

impl<U, X> BitWidth for UInt<U, X>
where
  Self: Canonical + IsLessOrEqual<U128> + DefaultRepr<LeEq<Self, U128>>,
{
  type Unsigned = <Self as DefaultRepr<LeEq<Self, U128>>>::Unsigned;
  type Signed = <Self as DefaultRepr<LeEq<Self, U128>>>::Signed;
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
  Limbs<B>: InlineUintRepr + InlineIntRepr + IntRepr<Unsigned = Limbs<B>>,
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

/// Writes `magnitude` in decimal with `write_digits_into`, negative unless `is_nonnegative`,
/// honoring the flags of `f` as integers do. Requires each limb to add at most `DIGITS` digits.
fn fmt_magnitude<L, const N: usize, const DIGITS: usize>(
  magnitude: &[L; N],
  write_digits_into: impl FnOnce(&[L; N], &mut [u8]) -> usize,
  is_nonnegative: bool,
  f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
  // No limbs hold only zero, and leave no room in the buffer for its digit.
  if N == 0 {
    return f.pad_integral(is_nonnegative, "", "0");
  }
  let mut buf = [[0; DIGITS]; N];
  let buf = buf.as_flattened_mut();
  let start = write_digits_into(magnitude, buf);
  f.pad_integral(is_nonnegative, "", ascii(&buf[start..])?)
}

/// Views digits produced by the integer core, which are always ASCII, as a string.
const fn ascii(digits: &[u8]) -> Result<&str, fmt::Error> {
  match core::str::from_utf8(digits) {
    Ok(text) => Ok(text),
    Err(_) => Err(fmt::Error),
  }
}

#[cfg(test)]
mod tests;
