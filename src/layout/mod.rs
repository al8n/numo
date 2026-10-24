//! Type-level digit counts and the storage types that hold them.

use core::{hash::Hash, ops::Add};

use typenum::{B0, B1, IsLessOrEqual, LeEq, Sum, Unsigned};

use sealed::{
  ConvertFrom, FractionNotReduced, IntegralNotReduced, SealedDigitCount, SealedDivIntermediate,
  SealedFitsDigits, SealedPick, SealedWiden, SealedWidenRepr,
};
pub(crate) use sealed::{SealedLayout, SealedStorage, Token};

use crate::{
  i192, i256, i320, i384, i448, i512,
  int::{self, DigitBuf},
  rounding::RoundingMode,
  u192, u256, u320, u384, u448, u512,
};

mod sealed {
  use crate::{
    i192, i256, i320, i384, i448, i512, int::DigitBuf, rounding::RoundingMode, u192, u256, u320,
    u384, u448, u512,
  };

  /// Taken by every method of [`SealedStorage`] and [`ConvertFrom`]. A bound `R: Storage` makes the
  /// methods of its supertrait callable from any crate, although the trait cannot be named, and
  /// several have preconditions. Only this crate can make a `Token`, which implements no trait that
  /// could make one, such as `Default`.
  pub struct Token(());

  impl Token {
    pub(crate) const fn new() -> Self {
      Self(())
    }
  }

  /// Seals [`Storage`](super::Storage) and gives storage-generic code access to the integer core.
  ///
  /// Every storage converts to every storage: `convert` and `checked_convert`, implemented for
  /// each source storage, call the `ConvertFrom` implementation of the target, which this trait
  /// requires for every source. Storage-generic code is not `const`, because trait methods cannot
  /// be.
  // clippy's `wrong_self_convention` requires `Copy` for `is_*` methods that take `self`.
  pub trait SealedStorage:
    Copy
    + ConvertFrom<i8>
    + ConvertFrom<i16>
    + ConvertFrom<i32>
    + ConvertFrom<i64>
    + ConvertFrom<i128>
    + ConvertFrom<i192>
    + ConvertFrom<i256>
    + ConvertFrom<i320>
    + ConvertFrom<i384>
    + ConvertFrom<i448>
    + ConvertFrom<i512>
    + ConvertFrom<u8>
    + ConvertFrom<u16>
    + ConvertFrom<u32>
    + ConvertFrom<u64>
    + ConvertFrom<u128>
    + ConvertFrom<u192>
    + ConvertFrom<u256>
    + ConvertFrom<u320>
    + ConvertFrom<u384>
    + ConvertFrom<u448>
    + ConvertFrom<u512>
  {
    /// `B1` for an unsigned storage, `B0` for a signed one.
    type IsUnsigned: typenum::Bit;

    /// `true` for a signed storage.
    const SIGNED: bool = !<Self::IsUnsigned as typenum::Bit>::BOOL;

    /// Returns `true` if the value is negative, which an unsigned value never is.
    fn is_negative(self, token: Token) -> bool;

    /// Returns `true` if the value is zero.
    fn is_zero(self, token: Token) -> bool;

    /// Returns `true` if `|self| <= 10^digits - 1`. Requires `10^digits` to be representable.
    fn within_digits(self, digits: u32, token: Token) -> bool;

    /// `self * rhs`. Requires the product to be representable.
    fn mul(self, rhs: Self, token: Token) -> Self;

    /// `self * 10^exp`. Requires the product to be representable.
    fn mul_pow10(self, exp: u32, token: Token) -> Self;

    /// `self * 10^exp`, or `None` if the product is not representable. Requires `10^exp` to be
    /// representable.
    fn checked_mul_pow10(self, exp: u32, token: Token) -> Option<Self>;

    /// Quotient and remainder of `self / 10^exp`, rounding toward zero; both have the sign of
    /// `self`. Requires `10^exp` to be representable.
    fn div_rem_pow10(self, exp: u32, token: Token) -> (Self, Self);

    /// `self / 10^exp` rounded to an integer with `mode`. Requires `10^exp` to be representable.
    fn round_div_pow10(self, exp: u32, mode: RoundingMode, token: Token) -> Self;

    /// `self / divisor` rounded to an integer with `mode`. Requires `divisor != 0`, and for a
    /// signed storage that neither operand is `MIN`.
    fn round_div(self, divisor: Self, mode: RoundingMode, token: Token) -> Self;

    /// Writes the decimal digits of the magnitude to the end of `buf` and returns the index of
    /// the first digit.
    fn write_digits(self, buf: &mut DigitBuf, token: Token) -> usize;

    /// Appends the ASCII digits in `bytes[start..end]` to `self`, computing `self * 10 + d` for
    /// each digit `d` and skipping every other byte. Requires `self` not to be negative and a
    /// representable result.
    fn push_digits(self, bytes: &[u8], start: usize, end: usize, token: Token) -> Self;

    /// `-self`, or `None` if it is not representable, which for an unsigned storage is every value
    /// but zero. Requires, for a signed storage, that `self` is not `MIN`, which no coefficient is.
    fn checked_neg(self, token: Token) -> Option<Self>;

    /// Converts the value to `Target`. Requires it to be representable there, so not negative for
    /// an unsigned `Target`.
    fn convert<Target: SealedStorage>(self, token: Token) -> Target;

    /// Converts the value to `Target`, or returns `None` if it is not representable there.
    fn checked_convert<Target: SealedStorage>(self, token: Token) -> Option<Target>;
  }

  /// Converts values of the storage `Source` to this storage. Implemented for every ordered pair
  /// of storages.
  pub trait ConvertFrom<Source>: Sized {
    /// Converts `value`. Requires it to be representable in this storage, so not negative for an
    /// unsigned one.
    fn convert_from(value: Source, token: Token) -> Self;

    /// Converts `value`, or returns `None` if it is not representable in this storage.
    fn checked_convert_from(value: Source, token: Token) -> Option<Self>;
  }

  /// Seals [`DigitCount`](super::DigitCount).
  pub trait SealedDigitCount {}

  /// Seals [`FitsDigits`](super::FitsDigits).
  pub trait SealedFitsDigits<Repr> {}

  /// Seals [`Layout`](super::Layout) and carries the digit counts as constants.
  pub trait SealedLayout<Repr> {
    /// The number of integral digits.
    const INTEGRAL: u32;
    /// The number of fraction digits.
    const FRACTION: u32;
    /// The total number of digits, `INTEGRAL + FRACTION`.
    const DIGITS: u32;
  }

  /// Seals [`Pick`](super::Pick).
  pub trait SealedPick<Digits> {}

  /// Seals [`DivIntermediate`](super::DivIntermediate).
  pub trait SealedDivIntermediate<IsUnsigned> {}

  /// Seals [`Widen`](super::Widen).
  pub trait SealedWiden<Target> {}

  /// Seals [`WidenRepr`](super::WidenRepr).
  pub trait SealedWidenRepr<Target> {}

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
}

/// An integer type that can store the coefficient of a [`Decimal`](crate::Decimal).
///
/// Implemented for the native signed integers `i8`, `i16`, `i32`, `i64` and `i128`, the native
/// unsigned integers `u8`, `u16`, `u32`, `u64` and `u128`, and the wide integers
/// [`i192`](crate::i192), [`i256`](crate::i256), [`i320`](crate::i320), [`i384`](crate::i384),
/// [`i448`](crate::i448), [`i512`](crate::i512), [`u192`](crate::u192), [`u256`](crate::u256),
/// [`u320`](crate::u320), [`u384`](crate::u384), [`u448`](crate::u448) and [`u512`](crate::u512).
/// A decimal with an unsigned storage is never negative. This trait is sealed and cannot be
/// implemented outside this crate.
pub trait Storage: SealedStorage + Copy + Ord + Hash {}

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
pub trait DigitCount: SealedDigitCount {
  /// The smallest signed integer type that holds every value with `Self` decimal digits.
  type Signed: Storage;

  /// The smallest unsigned integer type that holds every value with `Self` decimal digits.
  type Unsigned: Storage;
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
pub trait FitsDigits<Repr>: SealedFitsDigits<Repr> {}

/// Requires that `Repr` holds `Integral + Fraction` decimal digits, implemented for the pair
/// `(Integral, Fraction)` of type-level numbers.
///
/// [`Decimal`](crate::Decimal) requires `(Integral, Fraction): Layout<Repr>` on its constants,
/// constructors, `const fn` methods, formatting and conversions, not on the type itself. Generic
/// code needs no bounds to store and pass `Decimal<I, F, R>` values and only `R: Storage` to copy,
/// compare and hash them; it adds `(I, F): Layout<R>` where it uses the other items:
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
pub trait Layout<Repr>: SealedLayout<Repr> {}

impl<I, F, R> SealedLayout<R> for (I, F)
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
pub trait Pick<Digits>: SealedPick<Digits> {
  /// The smallest storage of the picked sign that holds every value with `Digits` decimal digits.
  type Storage: Storage;
}

impl<D: DigitCount> SealedPick<D> for B0 {}

impl<D: DigitCount> Pick<D> for B0 {
  type Storage = D::Signed;
}

impl<D: DigitCount> SealedPick<D> for B1 {}

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
pub trait DivIntermediate<IsUnsigned>: SealedDivIntermediate<IsUnsigned> {
  /// The smallest storage of the picked sign that holds every value with `Self` decimal digits.
  type Storage: Storage;
}

/// Requires that a decimal with the digit counts `Self = (Integral, Fraction)` converts to one with
/// the digit counts `Target = (I2, F2)` without losing digits: `Integral <= I2` and
/// `Fraction <= F2`.
///
/// Used by [`Decimal::widen`](crate::Decimal::widen). This trait is sealed and cannot be
/// implemented outside this crate.
pub trait Widen<Target>: SealedWiden<Target> {}

impl<I, F, I2, F2> SealedWiden<(I2, F2)> for (I, F)
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
// As for `FitsDigits`, the diagnostic does not print the storages.
#[diagnostic::on_unimplemented(
  message = "`widen` cannot convert from a signed storage to an unsigned storage",
  note = "use `try_convert` for a conversion that can fail",
  note = "the compiler writes `u192`, `u256`, `u320`, `u384`, `u448` and `u512` as `Uint` with the limbs `[u64; 3]`, `[u64; 4]`, `[u64; 5]`, `[u64; 6]`, `[u64; 7]` and `[u64; 8]`, and `i192`, `i256`, `i320`, `i384`, `i448` and `i512` as `Int` with the same limbs"
)]
pub trait WidenRepr<Target>: SealedWidenRepr<Target> {}

macro_rules! impl_storage {
  (signed: $($int:ident),+; unsigned: $($uint:ident),+ $(,)?) => {
    $(
      impl SealedStorage for $int {
        type IsUnsigned = B0;

        #[inline]
        fn is_negative(self, _: Token) -> bool {
          int::$int::is_negative(self)
        }

        #[inline]
        fn within_digits(self, digits: u32, _: Token) -> bool {
          let max = int::$int::max_magnitude(digits);
          int::$int::le(int::$int::neg(max), self) && int::$int::le(self, max)
        }

        #[inline]
        fn round_div(self, divisor: Self, mode: RoundingMode, _: Token) -> Self {
          // The rounding step requires a positive divisor: it reads the divisor as its magnitude
          // and the sign of the exact quotient from the dividend, because a quotient of zero has
          // none. Negating both operands keeps the quotient and moves its sign to the dividend.
          if int::$int::is_negative(divisor) {
            int::$int::round_div(int::$int::neg(self), int::$int::neg(divisor), mode).0
          } else {
            int::$int::round_div(self, divisor, mode).0
          }
        }

        #[inline]
        fn checked_neg(self, _: Token) -> Option<Self> {
          Some(int::$int::neg(self))
        }

        impl_storage!(@shared $int);
      }

      impl Storage for $int {}
    )+
    $(
      impl SealedStorage for $uint {
        type IsUnsigned = B1;

        #[inline]
        fn is_negative(self, _: Token) -> bool {
          false
        }

        #[inline]
        fn within_digits(self, digits: u32, _: Token) -> bool {
          int::$uint::le(self, int::$uint::max_magnitude(digits))
        }

        #[inline]
        fn round_div(self, divisor: Self, mode: RoundingMode, _: Token) -> Self {
          int::$uint::round_div(self, divisor, mode).0
        }

        #[inline]
        fn checked_neg(self, _: Token) -> Option<Self> {
          if int::$uint::is_zero(self) {
            Some(self)
          } else {
            None
          }
        }

        impl_storage!(@shared $uint);
      }

      impl Storage for $uint {}
    )+
  };
  (@shared $repr:ident) => {
    #[inline]
    fn is_zero(self, _: Token) -> bool {
      int::$repr::is_zero(self)
    }

    #[inline]
    fn mul(self, rhs: Self, _: Token) -> Self {
      int::$repr::mul(self, rhs)
    }

    #[inline]
    fn mul_pow10(self, exp: u32, _: Token) -> Self {
      int::$repr::mul_pow10(self, exp)
    }

    #[inline]
    fn checked_mul_pow10(self, exp: u32, _: Token) -> Option<Self> {
      int::$repr::checked_mul_pow10(self, exp)
    }

    #[inline]
    fn div_rem_pow10(self, exp: u32, _: Token) -> (Self, Self) {
      int::$repr::div_rem_pow10(self, exp)
    }

    #[inline]
    fn round_div_pow10(self, exp: u32, mode: RoundingMode, _: Token) -> Self {
      // `10^exp` is positive, as the rounding step requires of the divisor.
      int::$repr::round_div(self, int::$repr::pow10(exp), mode).0
    }

    #[inline]
    fn write_digits(self, buf: &mut DigitBuf, _: Token) -> usize {
      int::$repr::write_digits(self, buf)
    }

    #[inline]
    fn push_digits(self, bytes: &[u8], start: usize, end: usize, _: Token) -> Self {
      int::$repr::push_digits(self, bytes, start, end)
    }

    #[inline]
    fn convert<Target: SealedStorage>(self, token: Token) -> Target {
      <Target as ConvertFrom<$repr>>::convert_from(self, token)
    }

    #[inline]
    fn checked_convert<Target: SealedStorage>(self, token: Token) -> Option<Target> {
      <Target as ConvertFrom<$repr>>::checked_convert_from(self, token)
    }
  };
}

impl_storage!(
  signed: i8, i16, i32, i64, i128, i192, i256, i320, i384, i448, i512;
  unsigned: u8, u16, u32, u64, u128, u192, u256, u320, u384, u448, u512,
);

/// Implements [`ConvertFrom`] for every ordered pair of the listed storages, each written as
/// `(name sign kind)` with the sign `signed` or `unsigned` and the kind `native` or `wide`.
///
/// Between native storages, a conversion is a cast or a native `TryFrom`. Any other conversion
/// goes through limbs: the source's limbs are extended by the source's sign, or truncated, to the
/// target's limb count. The checked conversion fails when that changes the value or, between signs,
/// when the value is negative or reaches the sign bit of a signed target; the target then checks
/// its own range, which a native storage narrower than two limbs needs.
macro_rules! impl_convert {
  ([$($storage:tt),+ $(,)?]) => {
    impl_convert!(@sources [$($storage),+] to [$($storage),+]);
  };
  (@sources [$($source:tt),+] to $targets:tt) => {
    $(impl_convert!(@pairs $source to $targets);)+
  };
  (@pairs $source:tt to [$($target:tt),+]) => {
    $(impl_convert!(@pair $source to $target);)+
  };
  (@pair
    ($source:ident $source_sign:ident native) to ($target:ident $target_sign:ident native)
  ) => {
    impl ConvertFrom<$source> for $target {
      #[inline]
      fn convert_from(value: $source, _: Token) -> Self {
        value as $target
      }

      #[inline]
      fn checked_convert_from(value: $source, _: Token) -> Option<Self> {
        <$target>::try_from(value).ok()
      }
    }
  };
  (@pair
    ($source:ident $source_sign:ident $source_kind:ident)
    to ($target:ident $target_sign:ident $target_kind:ident)
  ) => {
    impl ConvertFrom<$source> for $target {
      #[inline]
      fn convert_from(value: $source, _: Token) -> Self {
        int::$target::from_limbs(impl_convert!(@resize $source_sign &int::$source::to_limbs(value)))
      }

      #[inline]
      fn checked_convert_from(value: $source, _: Token) -> Option<Self> {
        let limbs = int::$source::to_limbs(value);
        match impl_convert!(@checked_resize $source_sign $target_sign &limbs) {
          Some(limbs) => int::$target::checked_from_limbs(limbs),
          None => None,
        }
      }
    }
  };
  (@resize unsigned $limbs:expr) => {
    int::limbs::resize($limbs)
  };
  (@resize signed $limbs:expr) => {
    int::signed_limbs::resize($limbs)
  };
  (@checked_resize unsigned unsigned $limbs:expr) => {
    int::limbs::checked_resize($limbs)
  };
  (@checked_resize signed signed $limbs:expr) => {
    int::signed_limbs::checked_resize($limbs)
  };
  (@checked_resize signed unsigned $limbs:expr) => {
    int::signed_limbs::checked_to_unsigned($limbs)
  };
  (@checked_resize unsigned signed $limbs:expr) => {
    int::signed_limbs::checked_from_unsigned($limbs)
  };
}

impl_convert!([
  (i8 signed native),
  (i16 signed native),
  (i32 signed native),
  (i64 signed native),
  (i128 signed native),
  (u8 unsigned native),
  (u16 unsigned native),
  (u32 unsigned native),
  (u64 unsigned native),
  (u128 unsigned native),
  (i192 signed wide),
  (i256 signed wide),
  (i320 signed wide),
  (i384 signed wide),
  (i448 signed wide),
  (i512 signed wide),
  (u192 unsigned wide),
  (u256 unsigned wide),
  (u320 unsigned wide),
  (u384 unsigned wide),
  (u448 unsigned wide),
  (u512 unsigned wide),
]);

/// Implements [`WidenRepr`] from every unsigned storage to every storage, and from every signed
/// storage to every signed storage.
macro_rules! impl_widen_repr {
  (signed: [$($int:ident),+], unsigned: [$($uint:ident),+] $(,)?) => {
    impl_widen_repr!(@from [$($uint),+] to [$($int,)+ $($uint),+]);
    impl_widen_repr!(@from [$($int),+] to [$($int),+]);
  };
  (@from [$($source:ident),+] to $targets:tt) => {
    $(impl_widen_repr!(@pairs $source to $targets);)+
  };
  (@pairs $source:ident to [$($target:ident),+]) => {
    $(
      impl SealedWidenRepr<$target> for $source {}

      impl WidenRepr<$target> for $source {}
    )+
  };
}

impl_widen_repr!(
  signed: [i8, i16, i32, i64, i128, i192, i256, i320, i384, i448, i512],
  unsigned: [u8, u16, u32, u64, u128, u192, u256, u320, u384, u448, u512],
);

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
    impl SealedDigitCount for typenum::consts::$digits {}

    impl DigitCount for typenum::consts::$digits {
      type Signed = $signed;
      type Unsigned = $unsigned;
    }

    impl SealedDivIntermediate<B0> for typenum::consts::$digits {}

    impl DivIntermediate<B0> for typenum::consts::$digits {
      type Storage = $signed;
    }

    impl SealedDivIntermediate<B1> for typenum::consts::$digits {}

    impl DivIntermediate<B1> for typenum::consts::$digits {
      type Storage = $unsigned;
    }

    impl_digits!(@fits $digits: $signed $(, $wider_signed)*, $unsigned $(, $wider_unsigned)*);
  )+};
  (@fits $digits:ident: $($repr:ident),+) => {$(
    impl SealedFitsDigits<$repr> for typenum::consts::$digits {}

    impl FitsDigits<$repr> for typenum::consts::$digits {}
  )+};
}

digit_table!(impl_digits);

// Only a `u512` holds 154 digits and no signed storage does, so there is no `DigitCount`, and only
// a division of two unsigned storages has an intermediate of 154 digits.
impl SealedFitsDigits<u512> for typenum::consts::U154 {}

impl FitsDigits<u512> for typenum::consts::U154 {}

impl SealedDivIntermediate<B1> for typenum::consts::U154 {}

impl DivIntermediate<B1> for typenum::consts::U154 {
  type Storage = u512;
}

#[cfg(test)]
mod tests;
