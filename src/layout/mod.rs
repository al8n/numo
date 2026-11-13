//! Type-level digit counts and the storage types that hold them.

use core::{
  hash::Hash,
  ops::{Add, BitOr, Not},
};

use typenum::{B0, B1, IsLessOrEqual, LeEq, Or, Sum, Unsigned};

use sealed::{
  ConvertFrom, DigitsOf, FractionNotReduced, HasDivIntermediate, InPlace, IntegralNotReduced,
  ReprWidensTo, SignKept, SignPick, WidensTo,
};
pub(crate) use sealed::{InlineCore, LayoutDigits, StorageCore, Token};

use crate::{
  Int, Uint, i192, i256, i320, i384, i448, i512, int,
  rounding::RoundingMode,
  u192, u256, u320, u384, u448, u512,
  width::{Array, ArraySize, FillsRepr, TypenumNumber},
};

/// The digit buffer of the storage `R`, of the length of the digits of its widest magnitude: the
/// buffer that its row `write_digits` takes.
pub(crate) type DigitBuf<R> = Array<u8, <R as InlineCore>::DigitLen>;

/// The float scratch of the storage `R`: the limbs in which its conversions with the floats
/// compute.
pub(crate) type FloatScratch<R> = <<R as InlineCore>::FloatLen as ArraySize>::ArrayType<u64>;

mod sealed {
  use crate::{
    rounding::RoundingMode,
    width::{Array, ArrayLength, ArraySize},
  };

  /// Taken by every method of [`StorageCore`], [`InlineCore`] and [`ConvertFrom`]. A bound
  /// `R: Storage` or `R: InlineStorage` makes the methods of their sealed supertraits callable from
  /// any crate, although the traits cannot be named, and several have preconditions. Only this
  /// crate can make a `Token`, which implements no trait that could make one, such as `Default`.
  pub struct Token(());

  impl Token {
    pub(crate) const fn new() -> Self {
      Self(())
    }
  }

  /// Seals [`Storage`](super::Storage) and gives every storage its twin: the inline storage of the
  /// same width and sign, which computes the coefficients of the storage and holds one.
  pub trait StorageCore: Sized {
    /// `B1` for an unsigned storage, `B0` for a signed one.
    type IsUnsigned: typenum::Bit;

    /// `true` for a signed storage.
    const SIGNED: bool = !<Self::IsUnsigned as typenum::Bit>::BOOL;

    /// The inline storage that computes the coefficients of this storage and holds one: the storage
    /// itself for an inline storage.
    type Twin: InlineCore<IsUnsigned = Self::IsUnsigned>;

    /// The coefficient, copied into the inline storage.
    ///
    /// A copy, where the integers lend their inline storage: a storage that holds its limbs behind
    /// a pointer could lend the limbs, but an integer over them only through a cast between
    /// references. A coefficient is at most 64 bytes, and for an inline storage, its own twin, the
    /// copy is the one that passing the coefficient by value made before.
    fn twin(&self, token: Token) -> Self::Twin;

    /// A storage of the coefficient `value`: `value` itself for an inline storage.
    fn from_twin(value: Self::Twin, token: Token) -> Self;

    /// Stores the coefficient `value`, in place for a storage that holds its coefficient in
    /// place.
    fn set_twin(&mut self, value: Self::Twin, token: Token);
  }

  /// The integer core of the decimals, which the inline storages implement: the lengths of their
  /// buffers, the rows that compute with coefficients, and the conversions between storages.
  ///
  /// The rows are on the inline layer alone. Each takes and returns inline storages, so that
  /// storage-generic code computes in the twins and builds the storage of a decimal once, for the
  /// value that it returns, after every check.
  ///
  /// Every storage converts to every storage. A native source calls the `ConvertFrom`
  /// implementation of the target, which this trait requires for every native source, and a wide
  /// source lends its limbs to the rows `read_limbs` and `checked_read_limbs` of the target, so
  /// that no conversion is written for a pair of wide storages. Storage-generic code is not
  /// `const`, because trait methods cannot be.
  pub trait InlineCore:
    StorageCore<Twin = Self>
    + Copy
    + ConvertFrom<i8>
    + ConvertFrom<i16>
    + ConvertFrom<i32>
    + ConvertFrom<i64>
    + ConvertFrom<i128>
    + ConvertFrom<u8>
    + ConvertFrom<u16>
    + ConvertFrom<u32>
    + ConvertFrom<u64>
    + ConvertFrom<u128>
  {
    /// The decimal digits of the widest magnitude of the storage, `|MIN|` for a signed storage and
    /// `MAX` for an unsigned one: the bytes of its digit buffer.
    type DigitLen: ArrayLength;

    /// The bytes of the longest text of a decimal over the storage: its digits, a sign or a zero
    /// before the point, and the point.
    type TextLen: ArrayLength;

    /// The limbs in which the conversions of the storage with the floats compute: the fewest that
    /// hold the limbs of a coefficient, two for a native storage, and `bits(5^d) + 55` bits for the
    /// `d` digits that the storage holds, the bits of the largest power of five that divides a
    /// magnitude, the 53 bits of an `f64` and two more. Only the code of one storage builds the
    /// array, by resizing the limbs of a coefficient.
    type FloatLen: ArraySize;

    /// Returns `true` if the value is negative, which an unsigned value never is.
    fn is_negative(&self, token: Token) -> bool;

    /// Returns `true` if the value is zero.
    fn is_zero(&self, token: Token) -> bool;

    /// Returns `true` if `|self| <= 10^digits - 1`. Requires `10^digits` to be representable.
    fn within_digits(&self, digits: u32, token: Token) -> bool;

    /// `self * rhs`. Requires the product to be representable.
    fn mul(&self, rhs: &Self, token: Token) -> Self;

    /// `self * 10^exp`. Requires the product to be representable.
    fn mul_pow10(&self, exp: u32, token: Token) -> Self;

    /// `self * 10^exp`, or `None` if the product is not representable. Requires `10^exp` to be
    /// representable.
    fn checked_mul_pow10(&self, exp: u32, token: Token) -> Option<Self>;

    /// Quotient and remainder of `self / 10^exp`, rounding toward zero; both have the sign of
    /// `self`. Requires `10^exp` to be representable.
    fn div_rem_pow10(&self, exp: u32, token: Token) -> (Self, Self);

    /// `self / 10^exp` rounded to an integer with `mode`. Requires `10^exp` to be representable.
    fn round_div_pow10(&self, exp: u32, mode: RoundingMode, token: Token) -> Self;

    /// `self / divisor` rounded to an integer with `mode`. Requires `divisor != 0`, and for a
    /// signed storage that neither operand is `MIN`.
    fn round_div(&self, divisor: &Self, mode: RoundingMode, token: Token) -> Self;

    /// Writes the decimal digits of the magnitude to the end of `buf` and returns the index of
    /// the first digit. The buffer of the storage holds the digits of every value.
    fn write_digits(&self, buf: &mut Array<u8, Self::DigitLen>, token: Token) -> usize;

    /// Appends the ASCII digits in `bytes[start..end]` to `self`, computing `self * 10 + d` for
    /// each digit `d` and skipping every other byte. Requires `self` not to be negative and a
    /// representable result.
    fn push_digits(&self, bytes: &[u8], start: usize, end: usize, token: Token) -> Self;

    /// `-self`, or `None` if it is not representable, which for an unsigned storage is every value
    /// but zero. Requires, for a signed storage, that `self` is not `MIN`, which no coefficient is.
    fn checked_neg(&self, token: Token) -> Option<Self>;

    /// Converts the value to `Target`. Requires it to be representable there, so not negative for
    /// an unsigned `Target`.
    fn convert<Target: InlineCore>(&self, token: Token) -> Target;

    /// Converts the value to `Target`, or returns `None` if it is not representable there.
    fn checked_convert<Target: InlineCore>(&self, token: Token) -> Option<Target>;

    /// The integer in `limbs`, least significant first, in two's complement if `signed`, as a
    /// value of this storage. Requires it to be representable here.
    fn read_limbs<const N: usize>(limbs: &[u64; N], signed: bool, token: Token) -> Self;

    /// The integer in `limbs`, least significant first, in two's complement if `signed`, as a
    /// value of this storage, or `None` if it is not representable here.
    fn checked_read_limbs<const N: usize>(
      limbs: &[u64; N],
      signed: bool,
      token: Token,
    ) -> Option<Self>;
  }

  /// Converts values of the native storage `Source` to this storage. Implemented between every
  /// two native storages, and from every native storage to every array storage of either sign
  /// whose width fills its array.
  pub trait ConvertFrom<Source>: Sized {
    /// Converts `value`. Requires it to be representable in this storage, so not negative for an
    /// unsigned one.
    fn convert_from(value: &Source, token: Token) -> Self;

    /// Converts `value`, or returns `None` if it is not representable in this storage.
    fn checked_convert_from(value: &Source, token: Token) -> Option<Self>;
  }

  /// Seals [`MutableStorage`](super::MutableStorage): the storages that hold the coefficient in
  /// place, which every inline storage does.
  pub trait InPlace {}

  /// Seals [`FitsDigits`](super::FitsDigits).
  pub trait DigitsOf<Repr> {}

  /// Seals [`Layout`](super::Layout) and carries the digit counts as constants.
  pub trait LayoutDigits<Repr> {
    /// The number of integral digits.
    const INTEGRAL: u32;
    /// The number of fraction digits.
    const FRACTION: u32;
    /// The total number of digits, `INTEGRAL + FRACTION`.
    const DIGITS: u32;
  }

  /// Seals [`Pick`](super::Pick).
  pub trait SignPick<Digits> {}

  /// Seals [`DivIntermediate`](super::DivIntermediate).
  pub trait HasDivIntermediate<IsUnsigned> {}

  /// Seals [`Widen`](super::Widen).
  pub trait WidensTo<Target> {}

  /// Seals [`WidenRepr`](super::WidenRepr).
  pub trait ReprWidensTo<Target> {}

  /// Implemented only for `B1`, the result of `Integral <= I2` when widening is possible.
  #[diagnostic::on_unimplemented(
    message = "`widen` cannot reduce the number of integral digits",
    label = "the target `Integral` is smaller than the source `Integral`",
    note = "use `try_convert` for a conversion that can fail"
  )]
  pub trait IntegralNotReduced {}

  impl IntegralNotReduced for typenum::B1 {}

  /// Implemented only for `B1`, the result of `Fraction <= F2` when widening is possible.
  #[diagnostic::on_unimplemented(
    message = "`widen` cannot reduce the number of fraction digits",
    label = "the target `Fraction` is smaller than the source `Fraction`",
    note = "use `try_convert` for a conversion that can fail"
  )]
  pub trait FractionNotReduced {}

  impl FractionNotReduced for typenum::B1 {}

  /// Implemented only for `B1`, the result of "the source storage is unsigned, or the target
  /// storage is signed" when widening keeps the sign of every value.
  #[diagnostic::on_unimplemented(
    message = "`widen` cannot convert from a signed storage to an unsigned storage",
    label = "the source storage is signed and the target storage is unsigned",
    note = "use `try_convert` for a conversion that can fail",
    note = "the compiler writes `u192`, `u256`, `u320`, `u384`, `u448` and `u512` as `Uint` with the limbs `[u64; 3]`, `[u64; 4]`, `[u64; 5]`, `[u64; 6]`, `[u64; 7]` and `[u64; 8]`, and `i192`, `i256`, `i320`, `i384`, `i448` and `i512` as `Int` with the same limbs"
  )]
  pub trait SignKept {}

  impl SignKept for typenum::B1 {}
}

/// An integer type that can store the coefficient of a [`Decimal`](crate::Decimal).
///
/// Implemented for the native signed integers `i8`, `i16`, `i32`, `i64` and `i128`, the native
/// unsigned integers `u8`, `u16`, `u32`, `u64` and `u128`, and the wide integers
/// [`i192`](crate::i192), [`i256`](crate::i256), [`i320`](crate::i320), [`i384`](crate::i384),
/// [`i448`](crate::i448), [`i512`](crate::i512), [`u192`](crate::u192), [`u256`](crate::u256),
/// [`u320`](crate::u320), [`u384`](crate::u384), [`u448`](crate::u448) and [`u512`](crate::u512).
/// A decimal with an unsigned storage is never negative.
///
/// `R: Storage` gives generic code `Clone`, the comparisons and the hash of `Decimal<I, F, R>`.
/// Each of these storages holds the coefficient inline and in place, and is its own
/// [`Inline`](Self::Inline) storage: it implements [`InlineStorage`], which gives `Copy` too, and
/// [`MutableStorage`]. This trait is sealed and cannot be implemented outside this crate.
#[diagnostic::on_unimplemented(
  message = "`{Self}` is not a storage of `Decimal`",
  note = "`Decimal` stores a native integer, `i8`, `i16`, `i32`, `i64`, `i128`, `u8`, `u16`, `u32`, `u64` or `u128`, or a wide integer, `i192`, `i256`, `i320`, `i384`, `i448`, `i512`, `u192`, `u256`, `u320`, `u384`, `u448` or `u512`"
)]
pub trait Storage: StorageCore<Twin = <Self as Storage>::Inline> + Clone + Ord + Hash {
  /// The inline storage of the same width and sign, in which the coefficients of this storage are
  /// computed: the storage itself for an inline storage.
  type Inline: InlineStorage;
}

/// A [`Storage`] that holds the coefficient inline, so that the decimal is `Copy`.
///
/// Implemented for every storage of `Storage`: the native integers and the wide integers. It
/// implies [`MutableStorage`], so that generic code that bounds `R: InlineStorage` has the
/// assignments too. This trait is sealed and cannot be implemented outside this crate.
#[diagnostic::on_unimplemented(
  message = "`{Self}` is not an inline storage of `Decimal`",
  note = "`Decimal` stores a native integer, `i8`, `i16`, `i32`, `i64`, `i128`, `u8`, `u16`, `u32`, `u64` or `u128`, or a wide integer, `i192`, `i256`, `i320`, `i384`, `i448`, `i512`, `u192`, `u256`, `u320`, `u384`, `u448` or `u512`"
)]
pub trait InlineStorage: MutableStorage + Storage<Inline = Self> + InlineCore + Copy {}

/// A [`Storage`] that holds the coefficient in place, so that the decimal has `+=` and `-=`.
///
/// Implemented for every storage of `Storage`, each of which holds its coefficient inline. With
/// it, the decimal implements [`DecimalAssign`](crate::DecimalAssign), the assignments of generic
/// code. This trait is sealed and cannot be implemented outside this crate.
pub trait MutableStorage: Storage + InPlace {}

/// Maps a type-level number of decimal digits to the smallest signed and the smallest unsigned
/// integer types that hold every value with that many digits.
///
/// | Digits | `Signed` | `Unsigned` |
/// |---|---|---|
/// | 1-2 | `i8` | `u8` |
/// | 3-4 | `i16` | `u16` |
/// | 5-9 | `i32` | `u32` |
/// | 10-18 | `i64` | `u64` |
/// | 19 | `i128` | `u64` |
/// | 20-38 | `i128` | `u128` |
/// | 39-57 | [`i192`](crate::i192) | [`u192`](crate::u192) |
/// | 58-76 | [`i256`](crate::i256) | [`u256`](crate::u256) |
/// | 77 | [`i320`](crate::i320) | [`u256`](crate::u256) |
/// | 78-96 | [`i320`](crate::i320) | [`u320`](crate::u320) |
/// | 97-115 | [`i384`](crate::i384) | [`u384`](crate::u384) |
/// | 116-134 | [`i448`](crate::i448) | [`u448`](crate::u448) |
/// | 135-153 | [`i512`](crate::i512) | [`u512`](crate::u512) |
///
/// Implemented for `U1` through `U153`. [`Decimal`](crate::Decimal) uses `Signed` as its default
/// storage, and [`exact_mul`](crate::Decimal::exact_mul) stores a product of two unsigned storages
/// in `Unsigned`. No signed storage holds 154 digits, which only a [`u512`](crate::u512) does, so
/// `U154` has no `DigitCount`: such a type names its storage, as in `Decimal<U154, U0, u512>`, and
/// a product of 154 digits does not compile. This trait is sealed and cannot be implemented
/// outside this crate.
#[diagnostic::on_unimplemented(
  message = "a `Decimal` must have between 1 and 153 digits in total",
  label = "`Integral + Fraction` is zero or greater than 153",
  note = "an explicit `u512` storage holds up to 154 digits, as in `Decimal<I, F, u512>`"
)]
pub trait DigitCount: TypenumNumber {
  /// The smallest signed integer type that holds every value with `Self` decimal digits.
  type Signed: InlineStorage;

  /// The smallest unsigned integer type that holds every value with `Self` decimal digits.
  type Unsigned: InlineStorage;
}

/// Marks that `Repr` holds every value with `Self` decimal digits.
///
/// `Self` is a type-level number of digits. Implemented for every pair that fits: `i8` and `u8`
/// hold up to 2 digits, `i16` and `u16` up to 4, `i32` and `u32` up to 9, `i64` up to 18, `u64` up
/// to 19, `i128` and `u128` up to 38, `i192` and `u192` up to 57, `i256` up to 76, `u256` up to
/// 77, `i320` and `u320` up to 96, `i384` and `u384` up to 115, `i448` and `u448` up to 134, `i512`
/// up to 153 and `u512` up to 154. This trait is sealed and cannot be implemented outside this
/// crate.
// The diagnostic does not print `{Repr}`, which spells the wide storages out as `Uint` or `Int`
// with `typenum` bits and a limb array.
#[diagnostic::on_unimplemented(
  message = "the storage `Repr` cannot hold `Integral + Fraction` decimal digits",
  label = "`Integral + Fraction` does not fit in `Repr`",
  note = "`Integral + Fraction` must be at least 1; `i8` and `u8` hold up to 2 digits, `i16` and `u16` up to 4, `i32` and `u32` up to 9, `i64` up to 18, `u64` up to 19, `i128` and `u128` up to 38, `i192` and `u192` up to 57, `i256` up to 76, `u256` up to 77, `i320` and `u320` up to 96, `i384` and `u384` up to 115, `i448` and `u448` up to 134, `i512` up to 153, and `u512` up to 154",
  note = "the compiler writes `u192`, `u256`, `u320`, `u384`, `u448` and `u512` as `Uint` with the limbs `[u64; 3]`, `[u64; 4]`, `[u64; 5]`, `[u64; 6]`, `[u64; 7]` and `[u64; 8]`, and `i192`, `i256`, `i320`, `i384`, `i448` and `i512` as `Int` with the same limbs"
)]
pub trait FitsDigits<Repr>: DigitsOf<Repr> {}

/// Requires that `Repr` holds `Integral + Fraction` decimal digits, implemented for the pair
/// `(Integral, Fraction)` of type-level numbers.
///
/// [`Decimal`](crate::Decimal) requires `(Integral, Fraction): Layout<Repr>` on its constants,
/// constructors, `const fn` methods, formatting and conversions, not on the type itself. Generic
/// code needs no bounds to store and pass `Decimal<I, F, R>` values, only `R: Storage` to clone,
/// compare and hash them and `R: InlineStorage` to copy them, and adds `(I, F): Layout<R>` where it
/// uses the other items:
///
/// ```
/// use numo::{
///   Decimal, Layout, Storage,
///   typenum::{U2, U4},
/// };
///
/// // Storing a value needs no bounds.
/// struct Entry<I, F, R> {
///   label: &'static str,
///   amount: Decimal<I, F, R>,
/// }
///
/// // Formatting depends on the number of fraction digits, so it needs the layout.
/// fn describe<I, F, R>(entry: &Entry<I, F, R>) -> String
/// where
///   R: Storage,
///   (I, F): Layout<R>,
/// {
///   format!("{}: {}", entry.label, entry.amount)
/// }
///
/// let entry = Entry {
///   label: "fee",
///   amount: "12.5".parse::<Decimal<U2, U4>>().unwrap(),
/// };
/// assert_eq!(describe(&entry), "fee: 12.5");
/// ```
///
/// For `typenum` unsigned numbers `Integral` and `Fraction`, the bound holds exactly when
/// `Integral + Fraction` implements [`FitsDigits<Repr>`]. When it does not, the compiler reports
/// the diagnostic of [`FitsDigits`], or, for a constant or method whose impl requires the bound,
/// that the impl's bounds are not satisfied. This trait is sealed and cannot be implemented
/// outside this crate.
pub trait Layout<Repr>: LayoutDigits<Repr> {}

impl<I, F, R> LayoutDigits<R> for (I, F)
where
  I: Unsigned + Add<F>,
  F: Unsigned,
  Sum<I, F>: FitsDigits<R>,
{
  const INTEGRAL: u32 = I::U32;
  const FRACTION: u32 = F::U32;
  const DIGITS: u32 = I::U32 + F::U32;
}

impl<I, F, R> Layout<R> for (I, F)
where
  I: Unsigned + Add<F>,
  F: Unsigned,
  Sum<I, F>: FitsDigits<R>,
{
}

/// Picks the smallest storage of one sign for a type-level number of decimal digits.
///
/// Implemented for the `typenum` bits: `B0` picks [`DigitCount::Signed`] and `B1` picks
/// [`DigitCount::Unsigned`]. [`Decimal::exact_mul`](crate::Decimal::exact_mul) picks with `B1`
/// when both operands have unsigned storages and with `B0` otherwise. This trait is sealed and
/// cannot be implemented outside this crate.
pub trait Pick<Digits>: SignPick<Digits> {
  /// The smallest storage of the picked sign that holds every value with `Digits` decimal digits.
  type Storage: InlineStorage;
}

impl<D: DigitCount> SignPick<D> for B0 {}

impl<D: DigitCount> Pick<D> for B0 {
  type Storage = D::Signed;
}

impl<D: DigitCount> SignPick<D> for B1 {}

impl<D: DigitCount> Pick<D> for B1 {
  type Storage = D::Unsigned;
}

/// Picks the storage in which [`Decimal::div_round`](crate::Decimal::div_round) divides, for a
/// type-level number of decimal digits.
///
/// `Self` is the number of digits that the scaled dividend and the scaled divisor need, and
/// `IsUnsigned` is `B1` when both operands have unsigned storages and `B0` otherwise. As with
/// [`Pick`], `B0` picks [`DigitCount::Signed`] and `B1` picks [`DigitCount::Unsigned`] for `U1`
/// through `U153`. `U154` has a storage only with `B1`, a [`u512`](crate::u512), so a division
/// whose intermediate has more than 153 digits fails to compile, unless it has 154 and both
/// storages are unsigned. This trait is sealed and cannot be implemented outside this crate.
#[diagnostic::on_unimplemented(
  message = "the intermediate of `div_round` has more than 153 digits",
  label = "`Integral + max(Fraction, F2 + F3)` or `I2 + max(F2 + F3, Fraction) - F3` is greater than 153",
  note = "narrow an operand with `try_convert`, or ask for fewer fraction digits",
  note = "a `u512` intermediate holds up to 154 digits, when both storages are unsigned"
)]
pub trait DivIntermediate<IsUnsigned>: HasDivIntermediate<IsUnsigned> {
  /// The smallest storage of the picked sign that holds every value with `Self` decimal digits.
  type Storage: InlineStorage;
}

/// Requires that a decimal with the digit counts `Self = (Integral, Fraction)` converts to one with
/// the digit counts `Target = (I2, F2)` without losing digits: `Integral <= I2` and
/// `Fraction <= F2`.
///
/// Used by [`Decimal::widen`](crate::Decimal::widen). This trait is sealed and cannot be
/// implemented outside this crate.
pub trait Widen<Target>: WidensTo<Target> {}

impl<I, F, I2, F2> WidensTo<(I2, F2)> for (I, F)
where
  I: IsLessOrEqual<I2>,
  F: IsLessOrEqual<F2>,
  LeEq<I, I2>: IntegralNotReduced,
  LeEq<F, F2>: FractionNotReduced,
{
}

impl<I, F, I2, F2> Widen<(I2, F2)> for (I, F)
where
  I: IsLessOrEqual<I2>,
  F: IsLessOrEqual<F2>,
  LeEq<I, I2>: IntegralNotReduced,
  LeEq<F, F2>: FractionNotReduced,
{
}

/// Requires that a value stored in `Self` converts to the storage `Target` whatever its sign: an
/// unsigned storage converts to any storage, and a signed storage only to a signed one.
///
/// Used by [`Decimal::widen`](crate::Decimal::widen), which never fails. A negative value has no
/// unsigned representation, so a conversion from a signed to an unsigned storage goes through
/// [`Decimal::try_convert`](crate::Decimal::try_convert). This trait covers only the sign;
/// [`Layout`] checks that `Target` holds the digits. This trait is sealed and cannot be implemented
/// outside this crate.
pub trait WidenRepr<Target>: ReprWidensTo<Target> {}

/// Implements the storage traits for the native storages, written `native`, whose rows copy the
/// value to the storage's module in `int`, and for the wide storages, written `wide`, whose rows
/// lend their limbs to the limb layer, `int::signed::b64` or `int::unsigned::b64`. Each storage is
/// written with the `typenum` constants of the length of its digits and of its float scratch, as
/// `i64 [U19, U2]`. Each is an inline storage, its own twin, and implements [`StorageCore`] by
/// copies, the rows of [`InlineCore`], [`Storage`], [`InlineStorage`] and [`MutableStorage`].
macro_rules! impl_storage {
  (
    native signed: $($int:ident [$int_digits:ident, $int_float:ident]),+;
    unsigned: $($uint:ident [$uint_digits:ident, $uint_float:ident]),+ $(,)?
  ) => {
    $(
      impl_storage!(@inline $int B0);

      impl InlineCore for $int {
        impl_storage!(@lengths $int_digits, $int_float);

        #[inline]
        fn is_negative(&self, _: Token) -> bool {
          int::$int::is_negative(*self)
        }

        #[inline]
        fn within_digits(&self, digits: u32, _: Token) -> bool {
          let max = int::$int::max_magnitude(digits);
          int::$int::le(int::$int::neg(max), *self) && int::$int::le(*self, max)
        }

        #[inline]
        fn round_div(&self, divisor: &Self, mode: RoundingMode, _: Token) -> Self {
          // The rounding step requires a positive divisor: it reads the divisor as its magnitude
          // and the sign of the exact quotient from the dividend, because a quotient of zero has
          // none. Negating both operands keeps the quotient and moves its sign to the dividend.
          if int::$int::is_negative(*divisor) {
            int::$int::round_div(int::$int::neg(*self), int::$int::neg(*divisor), mode).0
          } else {
            int::$int::round_div(*self, *divisor, mode).0
          }
        }

        #[inline]
        fn checked_neg(&self, _: Token) -> Option<Self> {
          Some(int::$int::neg(*self))
        }

        impl_storage!(@native signed $int);
      }
    )+
    $(
      impl_storage!(@inline $uint B1);

      impl InlineCore for $uint {
        impl_storage!(@lengths $uint_digits, $uint_float);

        #[inline]
        fn is_negative(&self, _: Token) -> bool {
          false
        }

        #[inline]
        fn within_digits(&self, digits: u32, _: Token) -> bool {
          int::$uint::le(*self, int::$uint::max_magnitude(digits))
        }

        #[inline]
        fn round_div(&self, divisor: &Self, mode: RoundingMode, _: Token) -> Self {
          int::$uint::round_div(*self, *divisor, mode).0
        }

        #[inline]
        fn checked_neg(&self, _: Token) -> Option<Self> {
          if int::$uint::is_zero(*self) {
            Some(*self)
          } else {
            None
          }
        }

        impl_storage!(@native unsigned $uint);
      }
    )+
  };
  (
    wide signed: $($int:ident [$int_digits:ident, $int_float:ident]),+;
    unsigned: $($uint:ident [$uint_digits:ident, $uint_float:ident]),+ $(,)?
  ) => {
    $(
      impl_storage!(@inline $int B0);

      impl InlineCore for $int {
        impl_storage!(@lengths $int_digits, $int_float);

        #[inline]
        fn is_negative(&self, _: Token) -> bool {
          int::signed::b64::is_negative(self.as_limbs())
        }

        #[inline]
        fn within_digits(&self, digits: u32, _: Token) -> bool {
          let max = int::signed::b64::max_magnitude(digits);
          int::signed::b64::le(&int::signed::b64::neg(&max), self.as_limbs())
            && int::signed::b64::le(self.as_limbs(), &max)
        }

        #[inline]
        fn round_div(&self, divisor: &Self, mode: RoundingMode, _: Token) -> Self {
          // As for a native storage, negating both operands makes the divisor positive.
          let (value, divisor) = (self.as_limbs(), divisor.as_limbs());
          let (rounded, _, _) = if int::signed::b64::is_negative(divisor) {
            let (value, divisor) = (int::signed::b64::neg(value), int::signed::b64::neg(divisor));
            int::signed::b64::round_div(&value, &divisor, mode)
          } else {
            int::signed::b64::round_div(value, divisor, mode)
          };
          Self::from_limbs(rounded)
        }

        #[inline]
        fn checked_neg(&self, _: Token) -> Option<Self> {
          Some(Self::from_limbs(int::signed::b64::neg(self.as_limbs())))
        }

        impl_storage!(@wide signed $int);
      }
    )+
    $(
      impl_storage!(@inline $uint B1);

      impl InlineCore for $uint {
        impl_storage!(@lengths $uint_digits, $uint_float);

        #[inline]
        fn is_negative(&self, _: Token) -> bool {
          false
        }

        #[inline]
        fn within_digits(&self, digits: u32, _: Token) -> bool {
          int::unsigned::b64::le(self.as_limbs(), &int::unsigned::b64::max_magnitude(digits))
        }

        #[inline]
        fn round_div(&self, divisor: &Self, mode: RoundingMode, _: Token) -> Self {
          let (rounded, _, _) =
            int::unsigned::b64::round_div(self.as_limbs(), divisor.as_limbs(), mode);
          Self::from_limbs(rounded)
        }

        #[inline]
        fn checked_neg(&self, _: Token) -> Option<Self> {
          if int::unsigned::b64::is_zero(self.as_limbs()) {
            Some(*self)
          } else {
            None
          }
        }

        impl_storage!(@wide unsigned $uint);
      }
    )+
  };
  // An inline storage is its own twin: the protocol is the identity, and the storage holds its
  // coefficient inline and in place.
  (@inline $repr:ident $is_unsigned:ident) => {
    impl StorageCore for $repr {
      type IsUnsigned = $is_unsigned;
      type Twin = Self;

      #[inline(always)]
      fn twin(&self, _: Token) -> Self {
        *self
      }

      #[inline(always)]
      fn from_twin(value: Self, _: Token) -> Self {
        value
      }

      #[inline(always)]
      fn set_twin(&mut self, value: Self, _: Token) {
        *self = value;
      }
    }

    impl InPlace for $repr {}

    impl Storage for $repr {
      type Inline = Self;
    }

    impl InlineStorage for $repr {}

    impl MutableStorage for $repr {}
  };
  // The lengths of the buffers of a storage: its digits, its text, which has two bytes more, and
  // its float scratch.
  (@lengths $digits:ident, $float:ident) => {
    type DigitLen = typenum::consts::$digits;
    type TextLen = typenum::Sum<typenum::consts::$digits, typenum::consts::U2>;
    type FloatLen = typenum::consts::$float;
  };
  (@native $sign:ident $repr:ident) => {
    #[inline]
    fn is_zero(&self, _: Token) -> bool {
      int::$repr::is_zero(*self)
    }

    #[inline]
    fn mul(&self, rhs: &Self, _: Token) -> Self {
      int::$repr::mul(*self, *rhs)
    }

    #[inline]
    fn mul_pow10(&self, exp: u32, _: Token) -> Self {
      int::$repr::mul_pow10(*self, exp)
    }

    #[inline]
    fn checked_mul_pow10(&self, exp: u32, _: Token) -> Option<Self> {
      int::$repr::checked_mul_pow10(*self, exp)
    }

    #[inline]
    fn div_rem_pow10(&self, exp: u32, _: Token) -> (Self, Self) {
      int::$repr::div_rem_pow10(*self, exp)
    }

    #[inline]
    fn round_div_pow10(&self, exp: u32, mode: RoundingMode, _: Token) -> Self {
      // `10^exp` is positive, as the rounding step requires of the divisor.
      int::$repr::round_div(*self, int::$repr::pow10(exp), mode).0
    }

    #[inline]
    fn write_digits(&self, buf: &mut Array<u8, Self::DigitLen>, _: Token) -> usize {
      int::$repr::write_digits(*self, buf)
    }

    #[inline]
    fn push_digits(&self, bytes: &[u8], start: usize, end: usize, _: Token) -> Self {
      int::$repr::push_digits(*self, bytes, start, end)
    }

    impl_storage!(@convert native $repr);
    impl_storage!(@read $sign $repr);
  };
  (@wide $sign:ident $repr:ident) => {
    #[inline]
    fn is_zero(&self, _: Token) -> bool {
      int::$sign::b64::is_zero(self.as_limbs())
    }

    #[inline]
    fn mul(&self, rhs: &Self, _: Token) -> Self {
      Self::from_limbs(int::$sign::b64::mul(self.as_limbs(), rhs.as_limbs()))
    }

    #[inline]
    fn mul_pow10(&self, exp: u32, _: Token) -> Self {
      Self::from_limbs(int::$sign::b64::mul_pow10(self.as_limbs(), exp))
    }

    #[inline]
    fn checked_mul_pow10(&self, exp: u32, _: Token) -> Option<Self> {
      int::$sign::b64::checked_mul_pow10(self.as_limbs(), exp).map(Self::from_limbs)
    }

    #[inline]
    fn div_rem_pow10(&self, exp: u32, _: Token) -> (Self, Self) {
      let (quotient, remainder) = int::$sign::b64::div_rem_pow10(self.as_limbs(), exp);
      (Self::from_limbs(quotient), Self::from_limbs(remainder))
    }

    #[inline]
    fn round_div_pow10(&self, exp: u32, mode: RoundingMode, _: Token) -> Self {
      // `10^exp` is positive, as the rounding step requires of the divisor.
      let divisor = int::$sign::b64::pow10(exp);
      let (rounded, _, _) = int::$sign::b64::round_div(self.as_limbs(), &divisor, mode);
      Self::from_limbs(rounded)
    }

    #[inline]
    fn write_digits(&self, buf: &mut Array<u8, Self::DigitLen>, _: Token) -> usize {
      int::$sign::b64::write_digits_into(self.as_limbs(), buf)
    }

    #[inline]
    fn push_digits(&self, bytes: &[u8], start: usize, end: usize, _: Token) -> Self {
      Self::from_limbs(int::$sign::b64::push_digits(self.as_limbs(), bytes, start, end))
    }

    impl_storage!(@convert wide $sign);
    impl_storage!(@read $sign $repr);
  };
  // A native source converts through the `ConvertFrom` of the target. A wide source lends its limbs
  // to the rows of the target, which read them as signed for a signed source.
  (@convert native $repr:ident) => {
    #[inline]
    fn convert<Target: InlineCore>(&self, token: Token) -> Target {
      <Target as ConvertFrom<$repr>>::convert_from(self, token)
    }

    #[inline]
    fn checked_convert<Target: InlineCore>(&self, token: Token) -> Option<Target> {
      <Target as ConvertFrom<$repr>>::checked_convert_from(self, token)
    }
  };
  (@convert wide signed) => {
    #[inline]
    fn convert<Target: InlineCore>(&self, token: Token) -> Target {
      Target::read_limbs(self.as_limbs(), true, token)
    }

    #[inline]
    fn checked_convert<Target: InlineCore>(&self, token: Token) -> Option<Target> {
      Target::checked_read_limbs(self.as_limbs(), true, token)
    }
  };
  (@convert wide unsigned) => {
    #[inline]
    fn convert<Target: InlineCore>(&self, token: Token) -> Target {
      Target::read_limbs(self.as_limbs(), false, token)
    }

    #[inline]
    fn checked_convert<Target: InlineCore>(&self, token: Token) -> Option<Target> {
      Target::checked_read_limbs(self.as_limbs(), false, token)
    }
  };
  // The rows that read limbs of either sign into a storage of the sign `$sign`. The limbs are
  // extended by their sign, or truncated, to the limbs of the storage, two for a native one. The
  // checked row fails where that changes the value or, between signs, where the value is negative
  // or reaches the sign bit of a signed storage; the storage then checks its own range, which a
  // native storage narrower than two limbs needs.
  (@read $sign:ident $repr:ident) => {
    #[inline]
    fn read_limbs<const N: usize>(limbs: &[u64; N], signed: bool, _: Token) -> Self {
      let limbs = if signed {
        int::signed::b64::resize(limbs)
      } else {
        int::unsigned::b64::resize(limbs)
      };
      int::$repr::from_limbs(limbs)
    }

    #[inline]
    fn checked_read_limbs<const N: usize>(
      limbs: &[u64; N],
      signed: bool,
      _: Token,
    ) -> Option<Self> {
      let limbs = if signed {
        impl_storage!(@checked_resize signed $sign limbs)
      } else {
        impl_storage!(@checked_resize unsigned $sign limbs)
      };
      limbs.and_then(int::$repr::checked_from_limbs)
    }
  };
  // The checked resize of limbs of the first sign into a storage of the second sign.
  (@checked_resize unsigned unsigned $limbs:ident) => {
    int::unsigned::b64::checked_resize($limbs)
  };
  (@checked_resize signed signed $limbs:ident) => {
    int::signed::b64::checked_resize($limbs)
  };
  (@checked_resize signed unsigned $limbs:ident) => {
    int::signed::b64::checked_to_unsigned($limbs)
  };
  (@checked_resize unsigned signed $limbs:ident) => {
    int::signed::b64::checked_from_unsigned($limbs)
  };
}

impl_storage!(
  native signed: i8 [U3, U2], i16 [U5, U2], i32 [U10, U2], i64 [U19, U2], i128 [U39, U3];
  unsigned: u8 [U3, U2], u16 [U5, U2], u32 [U10, U2], u64 [U20, U2], u128 [U39, U3],
);
impl_storage!(
  wide signed: i192 [U58, U3], i256 [U77, U4], i320 [U97, U5], i384 [U116, U6], i448 [U135, U7],
    i512 [U154, U8];
  unsigned: u192 [U58, U3], u256 [U78, U4], u320 [U97, U5], u384 [U116, U6], u448 [U135, U7],
    u512 [U155, U8],
);

/// Implements [`ConvertFrom`] for every ordered pair of the listed native storages, each written as
/// `(name sign)` with the sign `signed` or `unsigned`, and from each of them to the arrays of both
/// signs.
///
/// Between native storages, a conversion is a cast or a native `TryFrom`. Into an array, the two
/// limbs of the native value are extended by its sign, or truncated, to the limbs of the array. The
/// checked conversion fails when that changes the value or, between signs, when the value is
/// negative or reaches the sign bit of a signed target. A wide source converts through the rows
/// `read_limbs` and `checked_read_limbs` of the target, so no pair names a wide storage.
macro_rules! impl_convert {
  ([$($storage:tt),+ $(,)?]) => {
    impl_convert!(@sources [$($storage),+] to [$($storage),+]);
    $(impl_convert!(@arrays $storage);)+
  };
  (@sources [$($source:tt),+] to $targets:tt) => {
    $(impl_convert!(@pairs $source to $targets);)+
  };
  (@pairs $source:tt to [$($target:tt),+]) => {
    $(impl_convert!(@pair $source to $target);)+
  };
  (@pair ($source:ident $source_sign:ident) to ($target:ident $target_sign:ident)) => {
    impl ConvertFrom<$source> for $target {
      #[inline]
      fn convert_from(value: &$source, _: Token) -> Self {
        *value as $target
      }

      #[inline]
      fn checked_convert_from(value: &$source, _: Token) -> Option<Self> {
        <$target>::try_from(*value).ok()
      }
    }
  };
  // These hold for every width that fills its array, a storage of a decimal or not, which is
  // harmless: the trait is sealed. Filling the array is the bound of `from_limbs`.
  (@arrays ($source:ident $sign:ident)) => {
    impl<B, const N: usize> ConvertFrom<$source> for Uint<B, [u64; N]>
    where
      B: FillsRepr<[u64; N]>,
    {
      #[inline]
      fn convert_from(value: &$source, _: Token) -> Self {
        Self::from_limbs(impl_convert!(@resize $sign &int::$source::to_limbs(*value)))
      }

      #[inline]
      fn checked_convert_from(value: &$source, _: Token) -> Option<Self> {
        impl_convert!(@checked_resize $sign unsigned &int::$source::to_limbs(*value))
          .map(Self::from_limbs)
      }
    }

    impl<B, const N: usize> ConvertFrom<$source> for Int<B, [u64; N]>
    where
      B: FillsRepr<[u64; N]>,
    {
      #[inline]
      fn convert_from(value: &$source, _: Token) -> Self {
        Self::from_limbs(impl_convert!(@resize $sign &int::$source::to_limbs(*value)))
      }

      #[inline]
      fn checked_convert_from(value: &$source, _: Token) -> Option<Self> {
        impl_convert!(@checked_resize $sign signed &int::$source::to_limbs(*value))
          .map(Self::from_limbs)
      }
    }
  };
  (@resize unsigned $limbs:expr) => {
    int::unsigned::b64::resize($limbs)
  };
  (@resize signed $limbs:expr) => {
    int::signed::b64::resize($limbs)
  };
  (@checked_resize unsigned unsigned $limbs:expr) => {
    int::unsigned::b64::checked_resize($limbs)
  };
  (@checked_resize signed signed $limbs:expr) => {
    int::signed::b64::checked_resize($limbs)
  };
  (@checked_resize signed unsigned $limbs:expr) => {
    int::signed::b64::checked_to_unsigned($limbs)
  };
  (@checked_resize unsigned signed $limbs:expr) => {
    int::signed::b64::checked_from_unsigned($limbs)
  };
}

impl_convert!([
  (i8 signed),
  (i16 signed),
  (i32 signed),
  (i64 signed),
  (i128 signed),
  (u8 unsigned),
  (u16 unsigned),
  (u32 unsigned),
  (u64 unsigned),
  (u128 unsigned),
]);

// The rule holds from every unsigned storage to every storage, and from every signed storage to
// every signed storage. It is computed as a bit, "the source is unsigned or the target is signed",
// which a marker implemented for `B1` alone requires, as `Widen` requires its comparisons: a bound
// on the bits of the parameters themselves would report the bit that it expected and the one that
// it found, where the marker reports its message.
impl<S, T> ReprWidensTo<T> for S
where
  S: StorageCore,
  T: StorageCore,
  T::IsUnsigned: Not,
  S::IsUnsigned: BitOr<<T::IsUnsigned as Not>::Output>,
  Or<S::IsUnsigned, <T::IsUnsigned as Not>::Output>: SignKept,
{
}

impl<S, T> WidenRepr<T> for S where S: ReprWidensTo<T> {}

/// Passes the digit-count table to `$callback`.
///
/// Each row is `Digits = value => [Signed, Wider...], [Unsigned, Wider...]`: the `typenum`
/// constant for a number of digits, its value, and, for each sign, every storage that holds that
/// many digits, smallest first. The first storage of each list is the `DigitCount` and
/// `DivIntermediate` mapping; every listed storage gets `FitsDigits`.
macro_rules! digit_table {
  ($callback:ident) => {
    $callback! {
      U1 = 1 =>
        [i8, i16, i32, i64, i128, i192, i256, i320, i384, i448, i512],
        [u8, u16, u32, u64, u128, u192, u256, u320, u384, u448, u512],
      U2 = 2 =>
        [i8, i16, i32, i64, i128, i192, i256, i320, i384, i448, i512],
        [u8, u16, u32, u64, u128, u192, u256, u320, u384, u448, u512],
      U3 = 3 =>
        [i16, i32, i64, i128, i192, i256, i320, i384, i448, i512],
        [u16, u32, u64, u128, u192, u256, u320, u384, u448, u512],
      U4 = 4 =>
        [i16, i32, i64, i128, i192, i256, i320, i384, i448, i512],
        [u16, u32, u64, u128, u192, u256, u320, u384, u448, u512],
      U5 = 5 =>
        [i32, i64, i128, i192, i256, i320, i384, i448, i512],
        [u32, u64, u128, u192, u256, u320, u384, u448, u512],
      U6 = 6 =>
        [i32, i64, i128, i192, i256, i320, i384, i448, i512],
        [u32, u64, u128, u192, u256, u320, u384, u448, u512],
      U7 = 7 =>
        [i32, i64, i128, i192, i256, i320, i384, i448, i512],
        [u32, u64, u128, u192, u256, u320, u384, u448, u512],
      U8 = 8 =>
        [i32, i64, i128, i192, i256, i320, i384, i448, i512],
        [u32, u64, u128, u192, u256, u320, u384, u448, u512],
      U9 = 9 =>
        [i32, i64, i128, i192, i256, i320, i384, i448, i512],
        [u32, u64, u128, u192, u256, u320, u384, u448, u512],
      U10 = 10 =>
        [i64, i128, i192, i256, i320, i384, i448, i512],
        [u64, u128, u192, u256, u320, u384, u448, u512],
      U11 = 11 =>
        [i64, i128, i192, i256, i320, i384, i448, i512],
        [u64, u128, u192, u256, u320, u384, u448, u512],
      U12 = 12 =>
        [i64, i128, i192, i256, i320, i384, i448, i512],
        [u64, u128, u192, u256, u320, u384, u448, u512],
      U13 = 13 =>
        [i64, i128, i192, i256, i320, i384, i448, i512],
        [u64, u128, u192, u256, u320, u384, u448, u512],
      U14 = 14 =>
        [i64, i128, i192, i256, i320, i384, i448, i512],
        [u64, u128, u192, u256, u320, u384, u448, u512],
      U15 = 15 =>
        [i64, i128, i192, i256, i320, i384, i448, i512],
        [u64, u128, u192, u256, u320, u384, u448, u512],
      U16 = 16 =>
        [i64, i128, i192, i256, i320, i384, i448, i512],
        [u64, u128, u192, u256, u320, u384, u448, u512],
      U17 = 17 =>
        [i64, i128, i192, i256, i320, i384, i448, i512],
        [u64, u128, u192, u256, u320, u384, u448, u512],
      U18 = 18 =>
        [i64, i128, i192, i256, i320, i384, i448, i512],
        [u64, u128, u192, u256, u320, u384, u448, u512],
      U19 = 19 =>
        [i128, i192, i256, i320, i384, i448, i512],
        [u64, u128, u192, u256, u320, u384, u448, u512],
      U20 = 20 =>
        [i128, i192, i256, i320, i384, i448, i512],
        [u128, u192, u256, u320, u384, u448, u512],
      U21 = 21 =>
        [i128, i192, i256, i320, i384, i448, i512],
        [u128, u192, u256, u320, u384, u448, u512],
      U22 = 22 =>
        [i128, i192, i256, i320, i384, i448, i512],
        [u128, u192, u256, u320, u384, u448, u512],
      U23 = 23 =>
        [i128, i192, i256, i320, i384, i448, i512],
        [u128, u192, u256, u320, u384, u448, u512],
      U24 = 24 =>
        [i128, i192, i256, i320, i384, i448, i512],
        [u128, u192, u256, u320, u384, u448, u512],
      U25 = 25 =>
        [i128, i192, i256, i320, i384, i448, i512],
        [u128, u192, u256, u320, u384, u448, u512],
      U26 = 26 =>
        [i128, i192, i256, i320, i384, i448, i512],
        [u128, u192, u256, u320, u384, u448, u512],
      U27 = 27 =>
        [i128, i192, i256, i320, i384, i448, i512],
        [u128, u192, u256, u320, u384, u448, u512],
      U28 = 28 =>
        [i128, i192, i256, i320, i384, i448, i512],
        [u128, u192, u256, u320, u384, u448, u512],
      U29 = 29 =>
        [i128, i192, i256, i320, i384, i448, i512],
        [u128, u192, u256, u320, u384, u448, u512],
      U30 = 30 =>
        [i128, i192, i256, i320, i384, i448, i512],
        [u128, u192, u256, u320, u384, u448, u512],
      U31 = 31 =>
        [i128, i192, i256, i320, i384, i448, i512],
        [u128, u192, u256, u320, u384, u448, u512],
      U32 = 32 =>
        [i128, i192, i256, i320, i384, i448, i512],
        [u128, u192, u256, u320, u384, u448, u512],
      U33 = 33 =>
        [i128, i192, i256, i320, i384, i448, i512],
        [u128, u192, u256, u320, u384, u448, u512],
      U34 = 34 =>
        [i128, i192, i256, i320, i384, i448, i512],
        [u128, u192, u256, u320, u384, u448, u512],
      U35 = 35 =>
        [i128, i192, i256, i320, i384, i448, i512],
        [u128, u192, u256, u320, u384, u448, u512],
      U36 = 36 =>
        [i128, i192, i256, i320, i384, i448, i512],
        [u128, u192, u256, u320, u384, u448, u512],
      U37 = 37 =>
        [i128, i192, i256, i320, i384, i448, i512],
        [u128, u192, u256, u320, u384, u448, u512],
      U38 = 38 =>
        [i128, i192, i256, i320, i384, i448, i512],
        [u128, u192, u256, u320, u384, u448, u512],
      U39 = 39 => [i192, i256, i320, i384, i448, i512], [u192, u256, u320, u384, u448, u512],
      U40 = 40 => [i192, i256, i320, i384, i448, i512], [u192, u256, u320, u384, u448, u512],
      U41 = 41 => [i192, i256, i320, i384, i448, i512], [u192, u256, u320, u384, u448, u512],
      U42 = 42 => [i192, i256, i320, i384, i448, i512], [u192, u256, u320, u384, u448, u512],
      U43 = 43 => [i192, i256, i320, i384, i448, i512], [u192, u256, u320, u384, u448, u512],
      U44 = 44 => [i192, i256, i320, i384, i448, i512], [u192, u256, u320, u384, u448, u512],
      U45 = 45 => [i192, i256, i320, i384, i448, i512], [u192, u256, u320, u384, u448, u512],
      U46 = 46 => [i192, i256, i320, i384, i448, i512], [u192, u256, u320, u384, u448, u512],
      U47 = 47 => [i192, i256, i320, i384, i448, i512], [u192, u256, u320, u384, u448, u512],
      U48 = 48 => [i192, i256, i320, i384, i448, i512], [u192, u256, u320, u384, u448, u512],
      U49 = 49 => [i192, i256, i320, i384, i448, i512], [u192, u256, u320, u384, u448, u512],
      U50 = 50 => [i192, i256, i320, i384, i448, i512], [u192, u256, u320, u384, u448, u512],
      U51 = 51 => [i192, i256, i320, i384, i448, i512], [u192, u256, u320, u384, u448, u512],
      U52 = 52 => [i192, i256, i320, i384, i448, i512], [u192, u256, u320, u384, u448, u512],
      U53 = 53 => [i192, i256, i320, i384, i448, i512], [u192, u256, u320, u384, u448, u512],
      U54 = 54 => [i192, i256, i320, i384, i448, i512], [u192, u256, u320, u384, u448, u512],
      U55 = 55 => [i192, i256, i320, i384, i448, i512], [u192, u256, u320, u384, u448, u512],
      U56 = 56 => [i192, i256, i320, i384, i448, i512], [u192, u256, u320, u384, u448, u512],
      U57 = 57 => [i192, i256, i320, i384, i448, i512], [u192, u256, u320, u384, u448, u512],
      U58 = 58 => [i256, i320, i384, i448, i512], [u256, u320, u384, u448, u512],
      U59 = 59 => [i256, i320, i384, i448, i512], [u256, u320, u384, u448, u512],
      U60 = 60 => [i256, i320, i384, i448, i512], [u256, u320, u384, u448, u512],
      U61 = 61 => [i256, i320, i384, i448, i512], [u256, u320, u384, u448, u512],
      U62 = 62 => [i256, i320, i384, i448, i512], [u256, u320, u384, u448, u512],
      U63 = 63 => [i256, i320, i384, i448, i512], [u256, u320, u384, u448, u512],
      U64 = 64 => [i256, i320, i384, i448, i512], [u256, u320, u384, u448, u512],
      U65 = 65 => [i256, i320, i384, i448, i512], [u256, u320, u384, u448, u512],
      U66 = 66 => [i256, i320, i384, i448, i512], [u256, u320, u384, u448, u512],
      U67 = 67 => [i256, i320, i384, i448, i512], [u256, u320, u384, u448, u512],
      U68 = 68 => [i256, i320, i384, i448, i512], [u256, u320, u384, u448, u512],
      U69 = 69 => [i256, i320, i384, i448, i512], [u256, u320, u384, u448, u512],
      U70 = 70 => [i256, i320, i384, i448, i512], [u256, u320, u384, u448, u512],
      U71 = 71 => [i256, i320, i384, i448, i512], [u256, u320, u384, u448, u512],
      U72 = 72 => [i256, i320, i384, i448, i512], [u256, u320, u384, u448, u512],
      U73 = 73 => [i256, i320, i384, i448, i512], [u256, u320, u384, u448, u512],
      U74 = 74 => [i256, i320, i384, i448, i512], [u256, u320, u384, u448, u512],
      U75 = 75 => [i256, i320, i384, i448, i512], [u256, u320, u384, u448, u512],
      U76 = 76 => [i256, i320, i384, i448, i512], [u256, u320, u384, u448, u512],
      U77 = 77 => [i320, i384, i448, i512], [u256, u320, u384, u448, u512],
      U78 = 78 => [i320, i384, i448, i512], [u320, u384, u448, u512],
      U79 = 79 => [i320, i384, i448, i512], [u320, u384, u448, u512],
      U80 = 80 => [i320, i384, i448, i512], [u320, u384, u448, u512],
      U81 = 81 => [i320, i384, i448, i512], [u320, u384, u448, u512],
      U82 = 82 => [i320, i384, i448, i512], [u320, u384, u448, u512],
      U83 = 83 => [i320, i384, i448, i512], [u320, u384, u448, u512],
      U84 = 84 => [i320, i384, i448, i512], [u320, u384, u448, u512],
      U85 = 85 => [i320, i384, i448, i512], [u320, u384, u448, u512],
      U86 = 86 => [i320, i384, i448, i512], [u320, u384, u448, u512],
      U87 = 87 => [i320, i384, i448, i512], [u320, u384, u448, u512],
      U88 = 88 => [i320, i384, i448, i512], [u320, u384, u448, u512],
      U89 = 89 => [i320, i384, i448, i512], [u320, u384, u448, u512],
      U90 = 90 => [i320, i384, i448, i512], [u320, u384, u448, u512],
      U91 = 91 => [i320, i384, i448, i512], [u320, u384, u448, u512],
      U92 = 92 => [i320, i384, i448, i512], [u320, u384, u448, u512],
      U93 = 93 => [i320, i384, i448, i512], [u320, u384, u448, u512],
      U94 = 94 => [i320, i384, i448, i512], [u320, u384, u448, u512],
      U95 = 95 => [i320, i384, i448, i512], [u320, u384, u448, u512],
      U96 = 96 => [i320, i384, i448, i512], [u320, u384, u448, u512],
      U97 = 97 => [i384, i448, i512], [u384, u448, u512],
      U98 = 98 => [i384, i448, i512], [u384, u448, u512],
      U99 = 99 => [i384, i448, i512], [u384, u448, u512],
      U100 = 100 => [i384, i448, i512], [u384, u448, u512],
      U101 = 101 => [i384, i448, i512], [u384, u448, u512],
      U102 = 102 => [i384, i448, i512], [u384, u448, u512],
      U103 = 103 => [i384, i448, i512], [u384, u448, u512],
      U104 = 104 => [i384, i448, i512], [u384, u448, u512],
      U105 = 105 => [i384, i448, i512], [u384, u448, u512],
      U106 = 106 => [i384, i448, i512], [u384, u448, u512],
      U107 = 107 => [i384, i448, i512], [u384, u448, u512],
      U108 = 108 => [i384, i448, i512], [u384, u448, u512],
      U109 = 109 => [i384, i448, i512], [u384, u448, u512],
      U110 = 110 => [i384, i448, i512], [u384, u448, u512],
      U111 = 111 => [i384, i448, i512], [u384, u448, u512],
      U112 = 112 => [i384, i448, i512], [u384, u448, u512],
      U113 = 113 => [i384, i448, i512], [u384, u448, u512],
      U114 = 114 => [i384, i448, i512], [u384, u448, u512],
      U115 = 115 => [i384, i448, i512], [u384, u448, u512],
      U116 = 116 => [i448, i512], [u448, u512],
      U117 = 117 => [i448, i512], [u448, u512],
      U118 = 118 => [i448, i512], [u448, u512],
      U119 = 119 => [i448, i512], [u448, u512],
      U120 = 120 => [i448, i512], [u448, u512],
      U121 = 121 => [i448, i512], [u448, u512],
      U122 = 122 => [i448, i512], [u448, u512],
      U123 = 123 => [i448, i512], [u448, u512],
      U124 = 124 => [i448, i512], [u448, u512],
      U125 = 125 => [i448, i512], [u448, u512],
      U126 = 126 => [i448, i512], [u448, u512],
      U127 = 127 => [i448, i512], [u448, u512],
      U128 = 128 => [i448, i512], [u448, u512],
      U129 = 129 => [i448, i512], [u448, u512],
      U130 = 130 => [i448, i512], [u448, u512],
      U131 = 131 => [i448, i512], [u448, u512],
      U132 = 132 => [i448, i512], [u448, u512],
      U133 = 133 => [i448, i512], [u448, u512],
      U134 = 134 => [i448, i512], [u448, u512],
      U135 = 135 => [i512], [u512],
      U136 = 136 => [i512], [u512],
      U137 = 137 => [i512], [u512],
      U138 = 138 => [i512], [u512],
      U139 = 139 => [i512], [u512],
      U140 = 140 => [i512], [u512],
      U141 = 141 => [i512], [u512],
      U142 = 142 => [i512], [u512],
      U143 = 143 => [i512], [u512],
      U144 = 144 => [i512], [u512],
      U145 = 145 => [i512], [u512],
      U146 = 146 => [i512], [u512],
      U147 = 147 => [i512], [u512],
      U148 = 148 => [i512], [u512],
      U149 = 149 => [i512], [u512],
      U150 = 150 => [i512], [u512],
      U151 = 151 => [i512], [u512],
      U152 = 152 => [i512], [u512],
      U153 = 153 => [i512], [u512],
    }
  };
}

macro_rules! impl_digits {
  ($(
    $digits:ident = $value:literal =>
      [$signed:ident $(, $wider_signed:ident)*],
      [$unsigned:ident $(, $wider_unsigned:ident)*]
  ),+ $(,)?) => {$(
    impl DigitCount for typenum::consts::$digits {
      type Signed = $signed;
      type Unsigned = $unsigned;
    }

    impl HasDivIntermediate<B0> for typenum::consts::$digits {}

    impl DivIntermediate<B0> for typenum::consts::$digits {
      type Storage = $signed;
    }

    impl HasDivIntermediate<B1> for typenum::consts::$digits {}

    impl DivIntermediate<B1> for typenum::consts::$digits {
      type Storage = $unsigned;
    }

    impl_digits!(@fits $digits: $signed $(, $wider_signed)*, $unsigned $(, $wider_unsigned)*);
  )+};
  (@fits $digits:ident: $($repr:ident),+) => {$(
    impl DigitsOf<$repr> for typenum::consts::$digits {}

    impl FitsDigits<$repr> for typenum::consts::$digits {}
  )+};
}

digit_table!(impl_digits);

// Only a `u512` holds 154 digits and no signed storage does, so there is no `DigitCount`, and only
// a division of two unsigned storages has an intermediate of 154 digits.
impl DigitsOf<u512> for typenum::consts::U154 {}

impl FitsDigits<u512> for typenum::consts::U154 {}

impl HasDivIntermediate<B1> for typenum::consts::U154 {}

impl DivIntermediate<B1> for typenum::consts::U154 {
  type Storage = u512;
}

#[cfg(test)]
mod tests;
