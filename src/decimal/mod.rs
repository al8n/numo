//! Fixed-point decimal numbers.

use core::{
  cmp::Ordering,
  fmt,
  hash::{Hash, Hasher},
  marker::PhantomData,
  ops::{Add, BitAnd, Sub},
};

use typenum::{And, Diff, Max, Maximum, Sum};

pub use error::{ConvertError, DivideError, ParseError, ParseErrorKind, QuantizeError};

use crate::{
  int,
  layout::{
    DigitCount, DivIntermediate, Layout, LayoutDigits, Pick, Storage, StorageCore, Token, Widen,
    WidenRepr,
  },
  rounding::RoundingMode,
};

mod error;
mod format;
mod parse;
mod storage;

#[cfg(test)]
mod tests;

/// An exact fixed-point decimal number with at most `Integral` integer digits and exactly
/// `Fraction` fraction digits, stored as a signed or unsigned integer coefficient of type `Repr`.
///
/// The value is `c / 10^Fraction`, where the coefficient `c` is the integer stored in `Repr` and
/// `|c| <= 10^(Integral + Fraction) - 1`. With a signed `Repr`, the range is symmetric
/// ([`MIN`](Self::MIN) is `-MAX`); with an unsigned `Repr`, it runs from zero to `MAX`, and `MIN`
/// is zero. There is no NaN, infinity or negative zero. The type corresponds to SQL
/// `NUMERIC(Integral + Fraction, Fraction)`. Because the number of fraction digits is part of the
/// type, every value has exactly one coefficient, and equality, ordering and hashing compare
/// coefficients.
///
/// `Integral` and `Fraction` are [`typenum`] numbers such as `U8` or `U<8>`. `Repr` defaults to
/// the smallest signed integer that holds `Integral + Fraction` digits (see [`DigitCount`]): a
/// native integer up to 38 digits, and beyond that the wide integers [`i192`](crate::i192),
/// [`i256`](crate::i256), [`i320`](crate::i320), [`i384`](crate::i384), [`i448`](crate::i448) and
/// [`i512`](crate::i512), which hold 57, 76, 96, 115, 134 and 153 digits. Any other storage that
/// holds the digits can be named explicitly: a wider one, or an unsigned one for values that are
/// never negative. An unsigned `Repr` holds as many digits as the signed one of the same width,
/// except that `u64` holds 19 digits and `i64` 18, [`u256`](crate::u256) holds 77 and `i256` 76,
/// and [`u512`](crate::u512) holds 154 and `i512` 153. Only an explicit `u512` holds 154 digits.
/// The type has the size, alignment and ABI of `Repr`.
///
/// ```
/// use numo::{
///   Decimal,
///   typenum::{U8, U9, U10, U12, U18, U60},
///   u320,
/// };
///
/// type Price = Decimal<U10, U8>; // 18 digits, stored in an `i64`
/// type Quantity = Decimal<U12, U8>; // 20 digits, stored in an `i128`
/// type WidePrice = Decimal<U10, U8, i128>; // 18 digits in an explicitly chosen `i128`
/// type Volume = Decimal<U10, U9, u64>; // 19 digits, never negative, in a `u64`
/// type TokenAmount = Decimal<U60, U18, u320>; // 78 digits, never negative, in a `u320`
///
/// assert_eq!(size_of::<Price>(), 8);
/// assert_eq!(size_of::<Quantity>(), 16);
/// assert_eq!(size_of::<WidePrice>(), 16);
/// assert_eq!(size_of::<Volume>(), 8);
/// assert_eq!(size_of::<TokenAmount>(), 40);
/// ```
///
/// A combination that does not fit is rejected during type checking: zero digits, more than 154
/// digits, or a `Repr` too small for `Integral + Fraction` digits. With the default `Repr`, zero
/// digits or more than 153 digits fail wherever the type is written, because the default storage
/// must be computed. With an explicit `Repr`, a combination that does not fit can be written, but
/// its constants and constructors fail to compile, so no value of the type can be created.
///
/// ```compile_fail
/// use numo::{
///   Decimal,
///   typenum::{U8, U10},
/// };
///
/// let _ = Decimal::<U10, U8, i32>::ZERO; // 18 digits do not fit in an `i32`
/// ```
///
/// Types with different `Repr` are distinct; [`widen`](Self::widen) and
/// [`try_convert`](Self::try_convert) convert between decimal types. The default `Repr` is meant
/// for concrete types, and generic code names `Repr`. `Decimal` itself has no bounds, so generic
/// code can store and pass `Decimal<I, F, R>` values without any. It needs `R: Storage` to copy,
/// compare or hash them, and adds `(I, F): Layout<R>` only where it uses items that depend on the
/// digit counts, such as formatting and conversions (see [`Layout`]).
///
/// # Operations
///
/// Every operation is exact: it returns the exact result or reports failure, and nothing rounds,
/// truncates or wraps unless asked to. The operators `+`, `-`, `*` and `/` are not implemented;
/// use [`checked_add`](Self::checked_add), [`checked_sub`](Self::checked_sub),
/// [`exact_mul`](Self::exact_mul) and [`div_round`](Self::div_round). With a signed `Repr`,
/// negation always succeeds because the range is symmetric, so [`Neg`](core::ops::Neg) is
/// implemented, along with [`neg`](Self::neg), [`abs`](Self::abs) and
/// [`is_negative`](Self::is_negative). An unsigned `Repr` has none of them, and its `checked_sub`
/// returns `None` below zero.
///
/// Rounding happens only with an explicit [`RoundingMode`]: [`round_to`](Self::round_to) converts
/// to a type with fewer fraction digits, [`div_round`](Self::div_round) divides and rounds the
/// quotient to a type of the caller's choice, and [`quantize`](Self::quantize) rounds to a
/// multiple of a step, such as a price to a tick size. [`is_multiple_of`](Self::is_multiple_of)
/// checks a value against a step without rounding it.
///
/// Conversions keep track of the sign. [`exact_mul`](Self::exact_mul) stores the product of two
/// unsigned storages in an unsigned storage and any other product in a signed one;
/// [`widen`](Self::widen) converts an unsigned storage to any storage but a signed storage only to
/// a signed one; and [`try_convert`](Self::try_convert) reports a negative value converted to an
/// unsigned storage as out of range, as does [`round_to`](Self::round_to) unless the value rounds
/// to zero.
///
/// A wide coefficient is built from 64-bit limbs, least significant first, and
/// [`from_repr`](Self::from_repr) checks its range. Any uint256 amount, which has at most 78
/// digits, fits a [`u320`](crate::u320):
///
/// ```
/// use numo::{
///   Decimal,
///   typenum::{U18, U60},
///   u320,
/// };
///
/// type TokenAmount = Decimal<U60, U18, u320>;
///
/// // The raw amount 1.5 * 10^18 as a uint256.
/// let raw: [u64; 4] = [1_500_000_000_000_000_000, 0, 0, 0];
/// let [l0, l1, l2, l3] = raw;
/// let amount = TokenAmount::from_repr(u320::from_limbs([l0, l1, l2, l3, 0])).unwrap();
/// assert_eq!(amount.to_string(), "1.5");
///
/// // The same amount from its decimal digits and an exponent.
/// assert_eq!("1500000000000000000e-18".parse::<TokenAmount>()?, amount);
/// # Ok::<(), numo::ParseError>(())
/// ```
///
/// Methods whose behavior depends only on the storage are `const fn`, generated for each storage,
/// so values can be computed at compile time:
///
/// ```
/// use numo::{
///   Decimal,
///   typenum::{U8, U10},
/// };
///
/// type Price = Decimal<U10, U8>;
///
/// const TICK: Price = match Price::parse("0.01") {
///   Ok(tick) => tick,
///   Err(_) => panic!("invalid tick"),
/// };
/// const TWO_TICKS: Option<Price> = TICK.checked_add(TICK);
///
/// assert_eq!(TWO_TICKS, Price::from_repr(2_000_000));
/// assert_eq!(Price::MAX.checked_add(TICK), None);
/// ```
///
/// # Text
///
/// [`parse`](Self::parse) and [`FromStr`](core::str::FromStr) accept ASCII text with an optional
/// sign, point and exponent, and `_` separators as in Rust literals: the numeric strings of the
/// General Decimal Arithmetic specification and every form that [`f64`] parses, such as `-1.5`,
/// `+.5`, `5.`, `2.5E-3` and `1_000`. An unsigned `Repr` rejects a leading `-`. Infinity and NaN
/// are recognized and reported as [`NotFinite`](crate::ParseErrorKind::NotFinite), because a
/// decimal is always finite. The value must fit exactly: once the exponent has moved the point,
/// digits beyond `Integral` and `Fraction` are accepted only when they are leading or trailing
/// zeros; anything else is an error, never a rounded value. See [`parse`](Self::parse) for the
/// grammar.
///
/// [`Display`](fmt::Display) writes the shortest exact form: an optional `-`, the integral part
/// without leading zeros (at least one digit), then `.` and the fraction digits without trailing
/// zeros, omitted when they are all zero. It never uses exponent notation, and the text depends
/// only on the value, not on `Repr` or the build configuration. [`Debug`](fmt::Debug) writes the
/// same text. Text written for one type parses back into any type with at least as many integral
/// and fraction digits, except that a negative value needs a signed `Repr`.
///
/// [`LowerExp`](fmt::LowerExp) and [`UpperExp`](fmt::UpperExp), the `{:e}` and `{:E}` formats,
/// write scientific notation instead: an optional `-`, the significant digits with the point after
/// the first, omitted when only one digit remains, then `e` or `E` and the exponent of the first
/// digit, with `-` when negative and without `+` or leading zeros. `64250.5` is `6.42505e4`,
/// `0.00012` is `1.2e-4`, `-3` is `-3e0` and zero is `0e0`. The text depends only on the value, and
/// parses back into any type with at least as many integral and fraction digits, except that a
/// negative value needs a signed `Repr`.
///
/// The width, fill, alignment, `+` and `0` flags behave as they do for integers, and the width
/// counts the whole text, exponent included. The precision is the minimum number of fraction
/// digits, those of the mantissa for `{:e}` and `{:E}`: shorter fractions are padded with zeros,
/// even beyond `Fraction`.
///
/// <div class="warning">
///
/// Precision never rounds or truncates. Unlike floating-point formatting, `{:.1}` writes `1.25`
/// as `1.25`, not `1.2` or `1.3`, and `{:.1e}` writes it as `1.25e0`, not `1.2e0` or `1.3e0`.
///
/// </div>
///
/// ```
/// use numo::{
///   Decimal,
///   typenum::{U4, U6},
/// };
///
/// type Amount = Decimal<U6, U4>;
///
/// let amount: Amount = "-001.2500".parse()?;
/// assert_eq!(amount, "-1_25e-2".parse()?);
/// assert_eq!(amount.to_string(), "-1.25");
/// assert_eq!(format!("{amount:.3}"), "-1.250");
/// assert_eq!(format!("{amount:.1}"), "-1.25");
/// assert_eq!(format!("{amount:>8}"), "   -1.25");
/// assert_eq!(format!("{amount:08.3}"), "-001.250");
/// assert_eq!(format!("{:+}", amount.abs()), "+1.25");
/// # Ok::<(), numo::ParseError>(())
/// ```
///
/// Scientific notation takes the same flags and precision:
///
/// ```
/// use numo::{
///   Decimal,
///   typenum::{U8, U10},
/// };
///
/// type Price = Decimal<U10, U8>;
///
/// let price: Price = "64250.5".parse()?;
/// assert_eq!(format!("{price:e}"), "6.42505e4");
/// assert_eq!(format!("{price:E}"), "6.42505E4");
/// assert_eq!(format!("{:e}", "0.00012".parse::<Price>()?), "1.2e-4");
/// assert_eq!(format!("{:e}", Price::ZERO), "0e0");
/// assert_eq!(format!("{price:.8e}"), "6.42505000e4");
/// assert_eq!(format!("{price:.2e}"), "6.42505e4");
/// assert_eq!(format!("{:012e}", -price), "-006.42505e4");
/// assert_eq!(format!("{price:>12E}"), "   6.42505E4");
/// assert_eq!(format!("{price:e}").parse::<Price>()?, price);
/// # Ok::<(), numo::ParseError>(())
/// ```
#[repr(transparent)]
pub struct Decimal<Integral, Fraction, Repr = <Sum<Integral, Fraction> as DigitCount>::Signed> {
  repr: Repr,
  _digits: PhantomData<(Integral, Fraction)>,
}

impl<I, F, R> Decimal<I, F, R> {
  /// Wraps a coefficient without checking it. Callers ensure that the layout of the type holds
  /// and that `|repr| <= 10^(Integral + Fraction) - 1`.
  pub(crate) const fn from_coeff(repr: R) -> Self {
    Self {
      repr,
      _digits: PhantomData,
    }
  }
}

#[cfg(feature = "quickcheck")]
impl<I, F, R> Decimal<I, F, R>
where
  R: Copy,
{
  /// Returns the coefficient. Storage-generic code calls this, because `repr` is defined for each
  /// storage.
  pub(crate) const fn coeff(self) -> R {
    self.repr
  }
}

impl<I, F, R> Decimal<I, F, R>
where
  (I, F): Layout<R>,
{
  const INTEGRAL: u32 = <(I, F) as LayoutDigits<R>>::INTEGRAL;
  const FRACTION: u32 = <(I, F) as LayoutDigits<R>>::FRACTION;
  const DIGITS: u32 = <(I, F) as LayoutDigits<R>>::DIGITS;
}

// Not `const`: storage-generic code calls trait methods, which cannot be `const` (E0015).
impl<I, F, R> Decimal<I, F, R>
where
  R: Storage,
{
  /// Multiplies two decimals exactly.
  ///
  /// The result has `Integral + I2` integral digits and `Fraction + F2` fraction digits, so the
  /// product always fits and is never rounded. It is stored in the smallest unsigned storage for
  /// its digits if both operands have unsigned storages, and in the smallest signed storage for
  /// its digits otherwise (see [`DigitCount`] and [`Pick`]). A product type with more than 153
  /// digits fails to compile, whatever the signs of the operands;
  /// [`try_convert`](Self::try_convert) can narrow an operand first.
  ///
  /// ```
  /// use numo::{
  ///   Decimal, i256,
  ///   typenum::{U8, U9, U10, U11, U12, U16, U17, U19, U21, U22, U38},
  /// };
  ///
  /// let price: Decimal<U10, U8> = "64250.5".parse()?;
  /// let quantity: Decimal<U12, U8> = "-0.0015".parse()?;
  /// let notional: Decimal<U22, U16> = price.exact_mul(quantity);
  /// assert_eq!(notional.to_string(), "-96.37575");
  ///
  /// // The product of two unsigned storages is unsigned.
  /// let price: Decimal<U10, U9, u64> = "64250.5".parse()?;
  /// let quantity: Decimal<U11, U8, u64> = "0.0015".parse()?;
  /// let notional: Decimal<U21, U17, u128> = price.exact_mul(quantity);
  /// assert_eq!(notional.to_string(), "96.37575");
  ///
  /// // Two 38-digit operands give a 76-digit product in a wide storage.
  /// let large: Decimal<U19, U19> = "-1e18".parse()?;
  /// let square: Decimal<U38, U38, i256> = large.exact_mul(large);
  /// assert_eq!(square.to_string(), "1000000000000000000000000000000000000");
  /// # Ok::<(), numo::ParseError>(())
  /// ```
  #[must_use]
  #[expect(
    clippy::type_complexity,
    reason = "the product storage is computed from the operand storages and the digit count"
  )]
  pub fn exact_mul<I2, F2, R2>(
    self,
    rhs: Decimal<I2, F2, R2>,
  ) -> Decimal<
    Sum<I, I2>,
    Sum<F, F2>,
    <And<R::IsUnsigned, R2::IsUnsigned> as Pick<Sum<Sum<I, I2>, Sum<F, F2>>>>::Storage,
  >
  where
    R2: Storage,
    I: Add<I2>,
    F: Add<F2>,
    Sum<I, I2>: Add<Sum<F, F2>>,
    R::IsUnsigned: BitAnd<R2::IsUnsigned>,
    And<R::IsUnsigned, R2::IsUnsigned>: Pick<Sum<Sum<I, I2>, Sum<F, F2>>>,
  {
    // Every value has at most the digits of its type: the other constructors require the layout
    // of the type they create. The product type has at least as many digits as either operand
    // type, and its storage, which `Pick` chooses, is unsigned only if both operand storages are,
    // so both operands convert to it exactly. Their product has at most as many digits as the
    // operands together, which is the digit count of its type, so it fits that storage as well.
    // Between native storages, the conversions are casts and the multiplication is native.
    let lhs = self.repr.convert(Token::new());
    let rhs = rhs.repr.convert(Token::new());
    Decimal::from_coeff(StorageCore::mul(lhs, rhs, Token::new()))
  }
}

// Not `const`: storage-generic code calls trait methods, which cannot be `const` (E0015).
impl<I, F, R> Decimal<I, F, R>
where
  R: Storage,
  (I, F): Layout<R>,
{
  /// Converts to a type with at least as many integral and fraction digits, and any storage that
  /// holds them, as long as a signed storage stays signed. Never fails.
  ///
  /// Converting to fewer integral or fraction digits fails to compile, and so does converting
  /// from a signed to an unsigned storage, which has no negative values; use
  /// [`try_convert`](Self::try_convert) instead. An unsigned storage converts to any storage.
  ///
  /// ```
  /// use numo::{
  ///   Decimal,
  ///   typenum::{U2, U4, U8, U10},
  /// };
  ///
  /// let small: Decimal<U2, U4> = "-12.5".parse()?;
  /// let wide: Decimal<U10, U8, i128> = small.widen();
  /// assert_eq!(wide.repr(), -1_250_000_000);
  ///
  /// let unsigned: Decimal<U2, U4, u32> = "12.5".parse()?;
  /// let signed: Decimal<U10, U8> = unsigned.widen();
  /// assert_eq!(signed.repr(), 1_250_000_000);
  /// # Ok::<(), numo::ParseError>(())
  /// ```
  #[must_use]
  pub fn widen<I2, F2, R2>(self) -> Decimal<I2, F2, R2>
  where
    R: WidenRepr<R2>,
    R2: Storage,
    (I2, F2): Layout<R2>,
    (I, F): Widen<(I2, F2)>,
  {
    // `I2 >= I` and `F2 >= F`, so `R2` holds the coefficient, and the scaled coefficient has at
    // most `I + F2 <= I2 + F2` digits. `R2` is unsigned only if `R` is, and then the coefficient
    // is not negative.
    let scale = Decimal::<I2, F2, R2>::FRACTION - Self::FRACTION;
    let coeff: R2 = self.repr.convert(Token::new());
    Decimal::from_coeff(coeff.mul_pow10(scale, Token::new()))
  }

  /// Converts to any decimal type, failing instead of losing digits.
  ///
  /// # Errors
  ///
  /// Returns [`ConvertError::OutOfRange`] if the value is outside the range of the target type,
  /// that is, if its integral part has more than `I2` digits, or if it is negative and `R2` is
  /// unsigned. Otherwise returns [`ConvertError::Inexact`] if the target has fewer fraction digits
  /// and a dropped digit is not zero. When both apply, the error is `OutOfRange`: a negative value
  /// converted to an unsigned storage is out of range even if dropping digits would leave zero,
  /// as for `-0.5` with no fraction digits.
  ///
  /// ```
  /// use numo::{
  ///   ConvertError, Decimal,
  ///   typenum::{U0, U2, U3, U4, U10},
  /// };
  ///
  /// let value: Decimal<U10, U4> = "-12.5000".parse()?;
  /// let narrow: Decimal<U2, U2> = value.try_convert().unwrap();
  /// assert_eq!(narrow.to_string(), "-12.5");
  /// assert_eq!(
  ///   value.try_convert::<U2, U2, u16>(),
  ///   Err(ConvertError::OutOfRange)
  /// );
  ///
  /// let half: Decimal<U10, U4> = "-0.5".parse()?;
  /// assert_eq!(
  ///   half.try_convert::<U2, U0, u8>(),
  ///   Err(ConvertError::OutOfRange)
  /// );
  ///
  /// let too_large: Decimal<U10, U4> = "123".parse()?;
  /// assert_eq!(
  ///   too_large.try_convert::<U2, U3, i32>(),
  ///   Err(ConvertError::OutOfRange)
  /// );
  ///
  /// let precise: Decimal<U10, U4> = "1.2345".parse()?;
  /// assert_eq!(
  ///   precise.try_convert::<U2, U2, i16>(),
  ///   Err(ConvertError::Inexact)
  /// );
  /// # Ok::<(), numo::ParseError>(())
  /// ```
  pub fn try_convert<I2, F2, R2>(self) -> Result<Decimal<I2, F2, R2>, ConvertError>
  where
    R2: Storage,
    (I2, F2): Layout<R2>,
  {
    // Checked before scaling, so that a negative value that truncates to zero, such as `-0.5`,
    // is out of range rather than inexact. A value that remains is never negative for an unsigned
    // target.
    if self.repr.is_negative(Token::new()) && !R2::SIGNED {
      return Err(ConvertError::OutOfRange);
    }
    let from = Self::FRACTION;
    let to = Decimal::<I2, F2, R2>::FRACTION;
    let digits = Decimal::<I2, F2, R2>::DIGITS;
    // The range is checked in `R2`: `10^(I2 + F2) - 1` may not fit `R`, as for a 6-digit `i32`
    // source and a 40-digit `i192` target. The powers of ten that the scaling uses fit their
    // storages, because a storage that holds `d` digits also holds `10^d`, which is not a power
    // of two.
    let converted = if to >= from {
      // Scaled in `R2`, which holds every scaled value in range, while `R` may not. A value that
      // `R2` cannot hold is out of range even before scaling.
      match self.repr.checked_convert::<R2>(Token::new()) {
        Some(coeff) => match coeff.checked_mul_pow10(to - from, Token::new()) {
          Some(scaled) if scaled.within_digits(digits, Token::new()) => scaled,
          _ => return Err(ConvertError::OutOfRange),
        },
        None => return Err(ConvertError::OutOfRange),
      }
    } else {
      // Divided in `R`, which holds `10^(from - to)`. The truncated quotient is in range exactly
      // when the value's integral part has at most `I2` digits, so range is decided before
      // exactness.
      let (quotient, remainder) = self.repr.div_rem_pow10(from - to, Token::new());
      match quotient.checked_convert::<R2>(Token::new()) {
        Some(quotient) if quotient.within_digits(digits, Token::new()) => {
          if !remainder.is_zero(Token::new()) {
            return Err(ConvertError::Inexact);
          }
          quotient
        }
        _ => return Err(ConvertError::OutOfRange),
      }
    };
    Ok(Decimal::from_coeff(converted))
  }

  /// Converts to any decimal type, rounding with `mode` where the target has fewer fraction
  /// digits.
  ///
  /// This is [`try_convert`](Self::try_convert) with dropped digits rounded instead of reported.
  /// If `F2` is at least `Fraction`, nothing is dropped, and the result is that of `try_convert`.
  /// Otherwise the value is rounded to `F2` fraction digits as [`RoundingMode`] describes, and the
  /// rounded value must fit the target. A negative value that rounds to zero gives zero, which has
  /// no sign.
  ///
  /// # Errors
  ///
  /// Returns [`ConvertError::OutOfRange`] if the rounded value is outside the range of the target
  /// type: if its integral part has more than `I2` digits, as when `9.99` rounds up to `10.0` for
  /// one integral digit, or if it is negative and `R2` is unsigned. Never returns
  /// [`ConvertError::Inexact`].
  ///
  /// Unlike `try_convert`, which reports every negative value converted to an unsigned storage as
  /// out of range, the range check applies to the rounded value, so a negative value that rounds to
  /// zero converts. For no fraction digits and a `u8` storage, `-0.4` with
  /// [`HalfUp`](RoundingMode::HalfUp) gives zero and `-0.6` is out of range, and `-0.9` with
  /// [`Ceiling`](RoundingMode::Ceiling) gives zero and with [`Floor`](RoundingMode::Floor) is out
  /// of range.
  ///
  /// ```
  /// use numo::{
  ///   ConvertError, Decimal, RoundingMode,
  ///   typenum::{U0, U1, U2, U4, U10},
  /// };
  ///
  /// let price: Decimal<U10, U4> = "2.345".parse()?;
  /// let cents: Decimal<U10, U2> = price.round_to(RoundingMode::HalfEven)?;
  /// assert_eq!(cents.to_string(), "2.34");
  /// let cents: Decimal<U10, U2> = price.round_to(RoundingMode::HalfUp)?;
  /// assert_eq!(cents.to_string(), "2.35");
  /// let cents: Decimal<U10, U2> = price.neg().round_to(RoundingMode::Floor)?;
  /// assert_eq!(cents.to_string(), "-2.35");
  ///
  /// // `9.99` rounds up to `10.0`, which has two integral digits.
  /// let nines: Decimal<U10, U4> = "9.99".parse()?;
  /// assert_eq!(
  ///   nines.round_to::<U1, U1, i8>(RoundingMode::HalfUp),
  ///   Err(ConvertError::OutOfRange)
  /// );
  ///
  /// // A negative value converts to an unsigned storage if it rounds to zero.
  /// let small: Decimal<U10, U4> = "-0.4".parse()?;
  /// let zero: Decimal<U1, U0, u8> = small.round_to(RoundingMode::HalfUp)?;
  /// assert!(zero.is_zero());
  /// assert_eq!(
  ///   small.round_to::<U1, U0, u8>(RoundingMode::Floor),
  ///   Err(ConvertError::OutOfRange)
  /// );
  /// # Ok::<(), Box<dyn core::error::Error>>(())
  /// ```
  #[must_use = "this returns the rounded value, without modifying the original"]
  pub fn round_to<I2, F2, R2>(self, mode: RoundingMode) -> Result<Decimal<I2, F2, R2>, ConvertError>
  where
    R2: Storage,
    (I2, F2): Layout<R2>,
  {
    let from = Self::FRACTION;
    let to = Decimal::<I2, F2, R2>::FRACTION;
    if to >= from {
      return self.try_convert();
    }
    // Rounded in `R`, which holds `10^(from - to)` and the rounded quotient: its magnitude is at
    // most `10^(Integral + to)`. The range is then checked in `R2`, as `try_convert` does.
    let rounded = self.repr.round_div_pow10(from - to, mode, Token::new());
    match rounded.checked_convert::<R2>(Token::new()) {
      Some(coeff) if coeff.within_digits(Decimal::<I2, F2, R2>::DIGITS, Token::new()) => {
        Ok(Decimal::from_coeff(coeff))
      }
      _ => Err(ConvertError::OutOfRange),
    }
  }

  /// Divides by `rhs`, rounding the quotient with `mode` to the fraction digits of any decimal
  /// type.
  ///
  /// The target type `Decimal<I3, F3, R3>` is chosen by inference or with a turbofish, whose first
  /// parameters are the target's, as in `a.div_round::<U12, U8, i128, _, _, _>(b, mode)`. The exact
  /// quotient is rounded once, to `F3` fraction digits, as [`RoundingMode`] describes, so the
  /// result is never inexact. A negative quotient that rounds to zero gives zero, which has no
  /// sign. [`Down`](RoundingMode::Down) rounds toward zero, which for a negative quotient differs
  /// from [`Floor`](RoundingMode::Floor), and with [`HalfEven`](RoundingMode::HalfEven) a tie goes
  /// to the quotient whose last digit is even.
  ///
  /// The division is exact up to the rounding. It is computed in an intermediate storage, after
  /// multiplying the dividend by `10^(F3 + F2 - Fraction)` or, when that exponent would be
  /// negative, the divisor by `10^(Fraction - F2 - F3)`, so the intermediate has
  /// `max(Integral + max(Fraction, F2 + F3), I2 + max(F2 + F3, Fraction) - F3)` digits. It is the
  /// [`DivIntermediate`] storage for those digits, unsigned only if both operand storages are, and
  /// it can be wider than both operands, as in the last example below. An intermediate of more than
  /// 153 digits fails to compile; 154 is allowed only when both storages are unsigned.
  ///
  /// To round a quotient to a step, such as a quantity to a lot size, [`quantize`](Self::quantize)
  /// it afterwards. `quantize` takes a step of the quotient's type, so
  /// [`try_convert`](Self::try_convert) a step of another type first. A directed mode, `Down`,
  /// `Up`, `Floor` or `Ceiling`, followed by `quantize` in the same mode gives the exact quotient
  /// quantized, because the step is a multiple of `10^-F3`; the half modes can round twice.
  ///
  /// # Errors
  ///
  /// Returns [`DivideError::DivisionByZero`] if `rhs` is zero, which is checked first. Otherwise
  /// returns [`DivideError::OutOfRange`] if the rounded quotient is outside the range of the target
  /// type: if its integral part has more than `I3` digits, or if it is negative and `R3` is
  /// unsigned, which a quotient that rounds to zero is not.
  ///
  /// ```
  /// use numo::{
  ///   Decimal, DivideError, RoundingMode,
  ///   typenum::{U0, U1, U2, U4, U8, U10, U12, U19, U20},
  /// };
  ///
  /// // The quantity that a budget buys at a price, rounded down to the quantity's fraction digits.
  /// let budget: Decimal<U12, U2> = "1000".parse()?;
  /// let price: Decimal<U10, U8> = "64250.5".parse()?;
  /// let quantity: Decimal<U12, U8> = budget.div_round(price, RoundingMode::Down)?;
  /// assert_eq!(quantity.to_string(), "0.01556408");
  /// let up = budget.div_round::<U12, U8, i128, _, _, _>(price, RoundingMode::Up)?;
  /// assert_eq!(up.to_string(), "0.01556409");
  ///
  /// // Then down to a lot size, converted to the quantity's type first.
  /// let lot: Decimal<U1, U4> = "0.0001".parse()?;
  /// let lots = quantity.quantize(lot.try_convert()?, RoundingMode::Down)?;
  /// assert_eq!(lots.to_string(), "0.0155");
  ///
  /// // A negative quotient goes into an unsigned storage only if it rounds to zero.
  /// let change: Decimal<U2, U2> = "-0.1".parse()?;
  /// let four: Decimal<U1, U0> = "4".parse()?;
  /// let zero: Decimal<U1, U1, u8> = change.div_round(four, RoundingMode::HalfUp)?;
  /// assert!(zero.is_zero());
  /// assert_eq!(
  ///   change.div_round::<U1, U1, u8, _, _, _>(four, RoundingMode::Floor),
  ///   Err(DivideError::OutOfRange)
  /// );
  ///
  /// // Division by zero is an error, never zero.
  /// assert_eq!(
  ///   budget.div_round::<U12, U8, i128, _, _, _>(Decimal::<U10, U8>::ZERO, RoundingMode::Down),
  ///   Err(DivideError::DivisionByZero)
  /// );
  ///
  /// // A quotient with more integral digits than the target holds.
  /// let tick: Decimal<U10, U8> = "0.00000001".parse()?;
  /// assert_eq!(
  ///   budget.div_round::<U10, U8, i64, _, _, _>(tick, RoundingMode::Down),
  ///   Err(DivideError::OutOfRange)
  /// );
  ///
  /// // Two 38-digit native operands, whose intermediate has 39 digits in an `i192`.
  /// let divisor: Decimal<U1, U1> = "0.7".parse()?;
  /// let quotient: Decimal<U20, U19> =
  ///   Decimal::<U19, U19>::MAX.div_round(divisor, RoundingMode::HalfEven)?;
  /// assert_eq!(quotient.to_string(), "14285714285714285714.2857142857142857141");
  /// # Ok::<(), Box<dyn core::error::Error>>(())
  /// ```
  #[must_use = "this returns the rounded quotient, without modifying the original"]
  pub fn div_round<I3, F3, R3, I2, F2, R2>(
    self,
    rhs: Decimal<I2, F2, R2>,
    mode: RoundingMode,
  ) -> Result<Decimal<I3, F3, R3>, DivideError>
  where
    R2: Storage,
    (I2, F2): Layout<R2>,
    R3: Storage,
    (I3, F3): Layout<R3>,
    F2: Add<F3>,
    Sum<F2, F3>: Max<F>,
    I: Add<Maximum<Sum<F2, F3>, F>>,
    Maximum<Sum<F2, F3>, F>: Sub<F3>,
    I2: Add<Diff<Maximum<Sum<F2, F3>, F>, F3>>,
    Sum<I, Maximum<Sum<F2, F3>, F>>: Max<Sum<I2, Diff<Maximum<Sum<F2, F3>, F>, F3>>>,
    R::IsUnsigned: BitAnd<R2::IsUnsigned>,
    Maximum<Sum<I, Maximum<Sum<F2, F3>, F>>, Sum<I2, Diff<Maximum<Sum<F2, F3>, F>, F3>>>:
      DivIntermediate<And<R::IsUnsigned, R2::IsUnsigned>>,
  {
    if rhs.repr.is_zero(Token::new()) {
      return Err(DivideError::DivisionByZero);
    }
    let from = Self::FRACTION;
    let by = Decimal::<I2, F2, R2>::FRACTION;
    let to = Decimal::<I3, F3, R3>::FRACTION;
    // In units of `10^-F3`, the quotient is `N / D`: `N = c_a * 10^(F3 + F2 - F)` and `D = c_b`, or
    // `N = c_a` and `D = c_b * 10^(F - F2 - F3)` when that exponent is positive. The intermediate
    // holds the digits of both and is unsigned only if both operand storages are, so the
    // coefficients convert to it exactly and the scaling does not overflow.
    let numerator = self.repr.convert::<<Maximum<
      Sum<I, Maximum<Sum<F2, F3>, F>>,
      Sum<I2, Diff<Maximum<Sum<F2, F3>, F>, F3>>,
    > as DivIntermediate<And<R::IsUnsigned, R2::IsUnsigned>>>::Storage>(
      Token::new()
    );
    let denominator = rhs.repr.convert(Token::new());
    let (numerator, denominator) = if to + by >= from {
      (
        StorageCore::mul_pow10(numerator, to + by - from, Token::new()),
        denominator,
      )
    } else {
      (
        numerator,
        StorageCore::mul_pow10(denominator, from - to - by, Token::new()),
      )
    };
    // `|N|, |D| <= 10^W - 1 <= MAX < |MIN|` in the intermediate, so neither operand is `MIN`. The
    // rounded quotient is at most `|N|`: rounding away from zero needs a remainder, so `|D| >= 2`.
    let quotient = numerator.round_div(denominator, mode, Token::new());
    match quotient.checked_convert::<R3>(Token::new()) {
      Some(coeff) if coeff.within_digits(Decimal::<I3, F3, R3>::DIGITS, Token::new()) => {
        Ok(Decimal::from_coeff(coeff))
      }
      _ => Err(DivideError::OutOfRange),
    }
  }
}

impl<I, F, R> Clone for Decimal<I, F, R>
where
  R: Copy,
{
  #[inline]
  fn clone(&self) -> Self {
    *self
  }
}

impl<I, F, R> Copy for Decimal<I, F, R> where R: Copy {}

impl<I, F, R> PartialEq for Decimal<I, F, R>
where
  R: PartialEq,
{
  #[inline]
  fn eq(&self, other: &Self) -> bool {
    self.repr == other.repr
  }
}

impl<I, F, R> Eq for Decimal<I, F, R> where R: Eq {}

impl<I, F, R> PartialOrd for Decimal<I, F, R>
where
  R: Ord,
{
  #[inline]
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl<I, F, R> Ord for Decimal<I, F, R>
where
  R: Ord,
{
  #[inline]
  fn cmp(&self, other: &Self) -> Ordering {
    self.repr.cmp(&other.repr)
  }
}

impl<I, F, R> Hash for Decimal<I, F, R>
where
  R: Hash,
{
  #[inline]
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.repr.hash(state);
  }
}

impl<I, F, R> fmt::Display for Decimal<I, F, R>
where
  R: Storage,
  (I, F): Layout<R>,
{
  /// Writes the shortest exact form, honoring width, fill, alignment, `+`, `0` and precision as
  /// described on [`Decimal`]. Precision never rounds or truncates.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut buf = [0; int::DIGIT_BUF_LEN];
    let start = self.repr.write_digits(&mut buf, Token::new());
    format::write(
      f,
      self.repr.is_negative(Token::new()),
      &buf[start..],
      Self::FRACTION as usize,
    )
  }
}

impl<I, F, R> fmt::Debug for Decimal<I, F, R>
where
  R: Storage,
  (I, F): Layout<R>,
{
  /// Writes the same text as [`Display`](fmt::Display).
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Display::fmt(self, f)
  }
}

impl<I, F, R> fmt::LowerExp for Decimal<I, F, R>
where
  R: Storage,
  (I, F): Layout<R>,
{
  /// Writes scientific notation with `e`, honoring width, fill, alignment, `+`, `0` and precision
  /// as described on [`Decimal`]. Precision never rounds or truncates: unlike the primitive types,
  /// `{:.1e}` writes `1.25` as `1.25e0`, not `1.2e0`. Use [`round_to`](Self::round_to) or
  /// [`quantize`](Self::quantize) to round.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut buf = [0; int::DIGIT_BUF_LEN];
    let start = self.repr.write_digits(&mut buf, Token::new());
    format::write_exp(
      f,
      self.repr.is_negative(Token::new()),
      &buf[start..],
      Self::FRACTION as usize,
      b'e',
    )
  }
}

impl<I, F, R> fmt::UpperExp for Decimal<I, F, R>
where
  R: Storage,
  (I, F): Layout<R>,
{
  /// Writes scientific notation with `E`, honoring width, fill, alignment, `+`, `0` and precision
  /// as described on [`Decimal`]. Precision never rounds or truncates: unlike the primitive types,
  /// `{:.1E}` writes `1.25` as `1.25E0`, not `1.2E0`. Use [`round_to`](Self::round_to) or
  /// [`quantize`](Self::quantize) to round.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut buf = [0; int::DIGIT_BUF_LEN];
    let start = self.repr.write_digits(&mut buf, Token::new());
    format::write_exp(
      f,
      self.repr.is_negative(Token::new()),
      &buf[start..],
      Self::FRACTION as usize,
      b'E',
    )
  }
}
