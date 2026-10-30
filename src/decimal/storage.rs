use core::{ops::Neg, str::FromStr};

use typenum::NonZero;

use super::{
  ConstDecimalOne, ConstFixedDecimal, Decimal, DecimalOne, FixedDecimal, ParseError, QuantizeError,
  SignedFixedDecimal, UnsignedFixedDecimal, parse,
};
use crate::{
  i192, i256, i320, i384, i448, i512, int, layout::Layout, rounding::RoundingMode, u192, u256,
  u320, u384, u448, u512,
};

/// Implements the constants, `const fn` methods and traits that depend only on the storage, for
/// each listed storage with its sign, `signed` or `unsigned`, and its kind, `native` or `wide`.
/// The kind selects how a coefficient goes to the storage's module in `int`, and how the examples
/// import the storage and spell its coefficients.
macro_rules! storage_decimal {
  ($($repr:ident: $sign:ident $kind:ident),+ $(,)?) => {$(
    storage_decimal!(@by $sign $repr $kind);

    impl<I, F> Decimal<I, F, $repr>
    where
      I: NonZero,
      (I, F): Layout<$repr>,
    {
      /// One. Defined only when `Integral` is nonzero, because one is out of range otherwise.
      pub const ONE: Self = Self::from_coeff(Self::SCALE);
    }

    impl<I, F> Default for Decimal<I, F, $repr>
    where
      (I, F): Layout<$repr>,
    {
      /// Returns zero.
      #[inline]
      fn default() -> Self {
        Self::ZERO
      }
    }

    impl<I, F> FromStr for Decimal<I, F, $repr>
    where
      (I, F): Layout<$repr>,
    {
      type Err = ParseError;

      /// Parses a decimal from text; see [`parse`](Self::parse).
      #[inline]
      fn from_str(s: &str) -> Result<Self, ParseError> {
        Self::parse(s)
      }
    }

    // The constants and methods are the inherent ones, which `Self::` names first, and the
    // functions without an inherent counterpart return the inherent constants.
    impl<I, F> FixedDecimal for Decimal<I, F, $repr>
    where
      (I, F): Layout<$repr>,
    {
      const INTEGRAL: u32 = Self::INTEGRAL;
      const FRACTION: u32 = Self::FRACTION;
      const DIGITS: u32 = Self::DIGITS;
      const SIGNED: bool = Self::SIGNED;

      type Repr = $repr;

      #[inline]
      fn repr(self) -> $repr {
        Self::repr(self)
      }

      #[inline]
      fn from_repr(repr: $repr) -> Option<Self> {
        Self::from_repr(repr)
      }

      #[inline]
      fn zero() -> Self {
        Self::ZERO
      }

      #[inline]
      fn max_value() -> Self {
        Self::MAX
      }

      #[inline]
      fn min_value() -> Self {
        Self::MIN
      }

      #[inline]
      fn is_zero(&self) -> bool {
        Self::is_zero(self)
      }

      #[inline]
      fn is_positive(&self) -> bool {
        Self::is_positive(self)
      }

      #[inline]
      fn checked_add(&self, rhs: &Self) -> Option<Self> {
        Self::checked_add(self, rhs)
      }

      #[inline]
      fn checked_sub(&self, rhs: &Self) -> Option<Self> {
        Self::checked_sub(self, rhs)
      }

      #[inline]
      fn quantize(&self, step: &Self, mode: RoundingMode) -> Result<Self, QuantizeError> {
        Self::quantize(self, step, mode)
      }

      #[inline]
      fn is_multiple_of(&self, step: &Self) -> bool {
        Self::is_multiple_of(self, step)
      }
    }

    impl<I, F> ConstFixedDecimal for Decimal<I, F, $repr>
    where
      (I, F): Layout<$repr>,
    {
      const ZERO: Self = Self::ZERO;
      const MAX: Self = Self::MAX;
      const MIN: Self = Self::MIN;
    }

    impl<I, F> DecimalOne for Decimal<I, F, $repr>
    where
      I: NonZero,
      (I, F): Layout<$repr>,
    {
      #[inline]
      fn one() -> Self {
        Self::ONE
      }
    }

    impl<I, F> ConstDecimalOne for Decimal<I, F, $repr>
    where
      I: NonZero,
      (I, F): Layout<$repr>,
    {
      const ONE: Self = Self::ONE;
    }

    #[cfg(feature = "serde")]
    #[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
    impl<'de, I, F> serde_core::Deserialize<'de> for Decimal<I, F, $repr>
    where
      (I, F): Layout<$repr>,
    {
      /// Reads a decimal from a string in the syntax of [`parse`](Self::parse), in every format.
      /// Any other input, such as a number, is rejected, so a decimal never passes through a
      /// binary floating-point value, with a fixed message that names nothing of the input; a
      /// string that does not parse reports its [`ParseError`](crate::ParseError).
      fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
      where
        D: serde_core::Deserializer<'de>,
      {
        crate::serde::deserialize_text(deserializer)
      }
    }
  )+};

  // The module of a native storage takes a coefficient by value, and that of a wide storage by
  // reference: the arms put `$($by)*` before each coefficient that they pass to it.
  (@by $sign:ident $repr:ident native) => {
    storage_decimal!(@$sign $repr native []);
  };
  (@by $sign:ident $repr:ident wide) => {
    storage_decimal!(@$sign $repr wide [&]);
  };

  (@signed $repr:ident $kind:ident [$($by:tt)*]) => {
    impl<I, F> Decimal<I, F, $repr>
    where
      (I, F): Layout<$repr>,
    {
      /// The largest coefficient, `10^(Integral + Fraction) - 1`.
      const MAX_COEFF: $repr = int::$repr::max_magnitude(Self::DIGITS);

      /// The smallest coefficient, `-MAX_COEFF`.
      const MIN_COEFF: $repr = int::$repr::neg($($by)* Self::MAX_COEFF);

      /// The coefficient of one, `10^Fraction`.
      const SCALE: $repr = int::$repr::pow10(Self::FRACTION);

      /// Zero.
      pub const ZERO: Self = Self::from_coeff(int::$repr::ZERO);

      /// The largest value, `10^Integral - 10^-Fraction`: every digit is `9`.
      pub const MAX: Self = Self::from_coeff(Self::MAX_COEFF);

      /// The smallest value, `-MAX`.
      pub const MIN: Self = Self::from_coeff(Self::MIN_COEFF);

      /// Whether the storage is signed: `true`, so values can be negative.
      pub const SIGNED: bool = true;

      const fn in_range(coeff: $($by)* $repr) -> bool {
        int::$repr::le($($by)* Self::MIN_COEFF, coeff)
          && int::$repr::le(coeff, $($by)* Self::MAX_COEFF)
      }

      /// Returns `true` if the value is zero.
      #[must_use]
      #[inline]
      pub const fn is_zero(&self) -> bool {
        int::$repr::is_zero($($by)* self.repr)
      }

      /// Returns `true` if the value is less than zero.
      #[must_use]
      #[inline]
      pub const fn is_negative(&self) -> bool {
        int::$repr::is_negative($($by)* self.repr)
      }

      /// Returns `true` if the value is greater than zero.
      #[must_use]
      #[inline]
      pub const fn is_positive(&self) -> bool {
        int::$repr::is_positive($($by)* self.repr)
      }

      /// Returns the coefficient: the stored integer `c` such that the value is
      /// `c / 10^Fraction`.
      #[must_use]
      #[inline]
      pub const fn repr(self) -> $repr {
        self.repr
      }

      /// Creates a value from its coefficient `repr`, meaning `repr / 10^Fraction`.
      ///
      /// Returns `None` if `|repr| > 10^(Integral + Fraction) - 1`.
      #[must_use]
      #[inline]
      pub const fn from_repr(repr: $repr) -> Option<Self> {
        if Self::in_range($($by)* repr) {
          Some(Self::from_coeff(repr))
        } else {
          None
        }
      }

      /// Parses a decimal from text, exactly.
      ///
      /// The input is ASCII text in the following grammar, in which letters match in either case.
      /// It accepts the numeric strings of the General Decimal Arithmetic specification, every
      /// form that [`f64`] parses with [`FromStr`], and `_` separators as in Rust literals.
      ///
      /// ```text
      /// number     = [sign] (finite / nonfinite)
      /// sign       = "+" / "-"
      /// finite     = (digits ["." [digits]] / "." digits) [exponent]
      /// exponent   = ("e" / "E") [sign] exp-digits
      /// digits     = DIGIT *(DIGIT / "_")
      /// exp-digits = *"_" DIGIT *(DIGIT / "_")
      /// nonfinite  = "inf" / "infinity" / ["s"] "nan" *DIGIT
      /// ```
      ///
      /// The integral and fraction digits each start with a digit, after which `_` may appear
      /// anywhere, also repeated or at the end. The exponent's digits may have `_` anywhere after
      /// its sign, also first, but need at least one digit. So `1_000.000_1`, `1__0`, `5_.5`,
      /// `1e_1_0`, `1e+_5`, `+.5`, `5.` and `.5e3` are valid, while `_1`, `._5`, `1._5`, `1e_`,
      /// `1e_+5`, `.`, `+` and `e5` are not. Whitespace, `,`, hexadecimal and non-ASCII digits are
      /// never accepted.
      ///
      /// The value is the mantissa times ten to the power of the exponent, and it must fit the
      /// type exactly: once the exponent has moved the point, the integral part may have at most
      /// `Integral` digits after its leading zeros, and the fraction digits beyond the first
      /// `Fraction` must all be `0`. Nothing is rounded. The exponent may have any number of
      /// digits and is applied exactly, so `1000e-3` and `0.0001e4` are both one, and zero parses
      /// with any exponent, as in `0e999999999`. `-0` parses as zero. A decimal is always finite,
      /// so an infinity or a NaN, including a signaling `sNaN` and a NaN with diagnostic digits
      /// such as `NaN123`, is recognized but reported as an error.
      ///
      /// Runs in linear time and constant space.
      ///
      /// # Errors
      ///
      /// The first condition that applies, in this order, determines the error kind:
      ///
      /// 1. [`Empty`](crate::ParseErrorKind::Empty): the input is empty. Whitespace alone is
      ///    invalid syntax.
      /// 2. [`InvalidSyntax`](crate::ParseErrorKind::InvalidSyntax): the input does not match
      ///    the grammar. The whole input is checked before its value, so `1e999_x` is invalid
      ///    syntax rather than out of range.
      /// 3. [`NotFinite`](crate::ParseErrorKind::NotFinite): the input is an infinity or a NaN.
      /// 4. [`OutOfRange`](crate::ParseErrorKind::OutOfRange): once the exponent has moved the
      ///    point, the integral part has more than `Integral` digits after its leading zeros.
      /// 5. [`Inexact`](crate::ParseErrorKind::Inexact): once the exponent has moved the point,
      ///    a fraction digit beyond the first `Fraction` is not `0`.
      ///
      /// # Examples
      ///
      /// ```
      #[doc = storage_decimal!(@use "Decimal, ParseErrorKind", signed $kind $repr)]
      ///
      #[doc = concat!("type D = Decimal<U1, U1, ", stringify!($repr), ">;")]
      ///
      /// for text in ["2.5", "+2.5", "02.50", "25e-1", "0.25E+1", ".25e1", "2_5e-1", "250_000e-5"] {
      #[doc = concat!("  assert_eq!(D::parse(text)?", storage_decimal!(@coeff $kind "25"), ");")]
      /// }
      #[doc = concat!("assert_eq!(D::parse(\"-5.\")?", storage_decimal!(@coeff $kind "-50"), ");")]
      /// assert_eq!(D::parse("-0e5")?, D::ZERO);
      ///
      /// let kind = |text| D::parse(text).map_err(|err| err.kind());
      /// assert_eq!(kind(""), Err(ParseErrorKind::Empty));
      /// assert_eq!(kind(" 1"), Err(ParseErrorKind::InvalidSyntax));
      /// assert_eq!(kind("1e9_x"), Err(ParseErrorKind::InvalidSyntax));
      /// assert_eq!(kind("-Infinity"), Err(ParseErrorKind::NotFinite));
      /// assert_eq!(kind("NaN"), Err(ParseErrorKind::NotFinite));
      /// assert_eq!(kind("1e1"), Err(ParseErrorKind::OutOfRange));
      /// assert_eq!(kind("0.05"), Err(ParseErrorKind::Inexact));
      /// // Out of range and inexact at once.
      /// assert_eq!(kind("10.05"), Err(ParseErrorKind::OutOfRange));
      /// # Ok::<(), numo::ParseError>(())
      /// ```
      pub const fn parse(s: &str) -> Result<Self, ParseError> {
        let input = s.as_bytes();
        let digits = match parse::scan(input, Self::INTEGRAL, Self::FRACTION, true) {
          Ok(digits) => digits,
          Err(err) => return Err(err),
        };
        let mut coeff =
          int::$repr::push_digits($($by)* int::$repr::ZERO, input, digits.start, digits.end);
        coeff = int::$repr::mul_pow10($($by)* coeff, digits.scale);
        if digits.negative {
          coeff = int::$repr::neg($($by)* coeff);
        }
        Ok(Self::from_coeff(coeff))
      }

      /// Adds two values, returning `None` if the sum is outside the range of the type.
      #[must_use]
      #[inline]
      pub const fn checked_add(&self, rhs: &Self) -> Option<Self> {
        match int::$repr::checked_add($($by)* self.repr, $($by)* rhs.repr) {
          Some(sum) if Self::in_range($($by)* sum) => Some(Self::from_coeff(sum)),
          _ => None,
        }
      }

      /// Subtracts `rhs` from `self`, returning `None` if the difference is outside the range of
      /// the type.
      #[must_use]
      #[inline]
      pub const fn checked_sub(&self, rhs: &Self) -> Option<Self> {
        match int::$repr::checked_sub($($by)* self.repr, $($by)* rhs.repr) {
          Some(difference) if Self::in_range($($by)* difference) => {
            Some(Self::from_coeff(difference))
          }
          _ => None,
        }
      }

      /// Rounds to a multiple of `step` with `mode`: returns the multiple of `step` that `mode`
      /// selects among the two multiples bracketing `self`, as [`RoundingMode`] describes. A
      /// multiple of `step` is returned unchanged, and a negative value that rounds to zero gives
      /// zero, which has no sign. With [`HalfEven`](RoundingMode::HalfEven), a tie goes to the
      /// multiple with the even quotient.
      ///
      /// `step` has the type of `self`; [`try_convert`](Self::try_convert) a step of another type
      /// first. To round a quantity that may be negative down to a step, use
      /// [`Down`](RoundingMode::Down), which rounds toward zero, not
      /// [`Floor`](RoundingMode::Floor), which rounds a negative value away from zero.
      ///
      /// # Errors
      ///
      /// Returns [`QuantizeError::InvalidStep`] if `step` is zero or negative. Zero is a step
      /// like any other, of which only zero is a multiple, so no other value has a multiple to
      /// round to. A negative step is rejected rather than taken as its magnitude, unlike in
      /// [`is_multiple_of`](Self::is_multiple_of), which has no error to report. Returns
      /// [`QuantizeError::OutOfRange`] if the selected multiple is outside the range of the type,
      /// which only rounding away from zero can cause.
      ///
      /// # Examples
      ///
      /// ```
      #[doc = storage_decimal!(@use "Decimal, QuantizeError, RoundingMode", signed $kind $repr)]
      ///
      #[doc = concat!("type D = Decimal<U1, U1, ", stringify!($repr), ">;")]
      ///
      /// let step = D::parse("0.5")?;
      /// let value = D::parse("2.3")?;
      /// assert_eq!(value.quantize(&step, RoundingMode::Up)?, D::parse("2.5")?);
      /// assert_eq!(value.quantize(&step, RoundingMode::Down)?, D::parse("2")?);
      /// assert_eq!(value.quantize(&step, RoundingMode::HalfEven)?, D::parse("2.5")?);
      /// assert!(!value.is_multiple_of(&step));
      ///
      /// // 2.1 lies halfway between 2 and 2.2, which are 10 and 11 steps of 0.2.
      /// let tick = D::parse("0.2")?;
      /// let tie = D::parse("2.1")?;
      /// assert_eq!(tie.quantize(&tick, RoundingMode::HalfEven)?, D::parse("2")?);
      /// assert_eq!(tie.quantize(&tick, RoundingMode::HalfUp)?, D::parse("2.2")?);
      ///
      /// // A negative quantity rounds down to a step with `Down`.
      /// let short = D::parse("-2.3")?;
      /// assert_eq!(short.quantize(&step, RoundingMode::Down)?, D::parse("-2")?);
      /// assert_eq!(short.quantize(&step, RoundingMode::Floor)?, D::parse("-2.5")?);
      ///
      /// assert_eq!(value.quantize(&D::ZERO, RoundingMode::Up), Err(QuantizeError::InvalidStep));
      /// assert_eq!(value.quantize(&-step, RoundingMode::Up), Err(QuantizeError::InvalidStep));
      /// // 9.9 rounds up to 10, which has two integral digits.
      /// assert_eq!(D::MAX.quantize(&step, RoundingMode::Up), Err(QuantizeError::OutOfRange));
      /// # Ok::<(), Box<dyn core::error::Error>>(())
      /// ```
      #[must_use = "this returns the rounded value, without modifying the original"]
      pub const fn quantize(&self, step: &Self, mode: RoundingMode) -> Result<Self, QuantizeError> {
        if !int::$repr::is_positive($($by)* step.repr) {
          return Err(QuantizeError::InvalidStep);
        }
        let (_, remainder, away) =
          int::$repr::round_div($($by)* self.repr, $($by)* step.repr, mode);
        // The remainder has the sign of `self` and a smaller magnitude, so the multiple toward
        // zero, `self - remainder`, exists and is in range. Only the step away from zero can leave
        // the storage or the decimal range.
        let rounded = match int::$repr::checked_sub($($by)* self.repr, $($by)* remainder) {
          Some(toward_zero) if away => {
            if int::$repr::is_negative($($by)* self.repr) {
              int::$repr::checked_sub($($by)* toward_zero, $($by)* step.repr)
            } else {
              int::$repr::checked_add($($by)* toward_zero, $($by)* step.repr)
            }
          }
          toward_zero => toward_zero,
        };
        match rounded {
          Some(coeff) if Self::in_range($($by)* coeff) => Ok(Self::from_coeff(coeff)),
          _ => Err(QuantizeError::OutOfRange),
        }
      }

      /// Returns `true` if `self` is a multiple of `step`, an integer number of steps, without
      /// rounding anything: for example, whether a price is on a tick.
      ///
      /// Zero is a step like any other, of which only zero is a multiple. A negative step counts
      /// as its magnitude, because this method has no error to report, while
      /// [`quantize`](Self::quantize) rejects a step that is zero or negative.
      #[must_use]
      #[inline]
      pub const fn is_multiple_of(&self, step: &Self) -> bool {
        if int::$repr::is_zero($($by)* step.repr) {
          return int::$repr::is_zero($($by)* self.repr);
        }
        // A coefficient is never `MIN`, so the division does not overflow, even for a step of
        // `-1`.
        let (_, remainder) = int::$repr::div_rem($($by)* self.repr, $($by)* step.repr);
        int::$repr::is_zero($($by)* remainder)
      }

      /// Returns `-self`. Never fails, because the range is symmetric.
      #[must_use]
      #[inline]
      pub const fn neg(&self) -> Self {
        Self::from_coeff(int::$repr::neg($($by)* self.repr))
      }

      /// Returns the absolute value. Never fails, because the range is symmetric.
      #[must_use]
      #[inline]
      pub const fn abs(&self) -> Self {
        Self::from_coeff(int::$repr::abs($($by)* self.repr))
      }
    }

    impl<I, F> Neg for Decimal<I, F, $repr>
    where
      (I, F): Layout<$repr>,
    {
      type Output = Self;

      /// Returns `-self`. Never fails, because the range is symmetric.
      #[inline]
      fn neg(self) -> Self {
        Self::neg(&self)
      }
    }

    impl<I, F> SignedFixedDecimal for Decimal<I, F, $repr>
    where
      (I, F): Layout<$repr>,
    {
      #[inline]
      fn is_negative(&self) -> bool {
        Self::is_negative(self)
      }

      #[inline]
      fn neg(&self) -> Self {
        Self::neg(self)
      }

      #[inline]
      fn abs(&self) -> Self {
        Self::abs(self)
      }
    }
  };

  (@unsigned $repr:ident $kind:ident [$($by:tt)*]) => {
    impl<I, F> Decimal<I, F, $repr>
    where
      (I, F): Layout<$repr>,
    {
      /// The largest coefficient, `10^(Integral + Fraction) - 1`.
      const MAX_COEFF: $repr = int::$repr::max_magnitude(Self::DIGITS);

      /// The coefficient of one, `10^Fraction`.
      const SCALE: $repr = int::$repr::pow10(Self::FRACTION);

      /// Zero.
      pub const ZERO: Self = Self::from_coeff(int::$repr::ZERO);

      /// The largest value, `10^Integral - 10^-Fraction`: every digit is `9`.
      pub const MAX: Self = Self::from_coeff(Self::MAX_COEFF);

      /// The smallest value, zero.
      pub const MIN: Self = Self::ZERO;

      /// Whether the storage is signed: `false`, so values are never negative.
      pub const SIGNED: bool = false;

      const fn in_range(coeff: $($by)* $repr) -> bool {
        int::$repr::le(coeff, $($by)* Self::MAX_COEFF)
      }

      /// Returns `true` if the value is zero.
      #[must_use]
      #[inline]
      pub const fn is_zero(&self) -> bool {
        int::$repr::is_zero($($by)* self.repr)
      }

      /// Returns `true` if the value is greater than zero, that is, not zero.
      #[must_use]
      #[inline]
      pub const fn is_positive(&self) -> bool {
        int::$repr::is_positive($($by)* self.repr)
      }

      /// Returns the coefficient: the stored integer `c` such that the value is
      /// `c / 10^Fraction`.
      #[must_use]
      #[inline]
      pub const fn repr(self) -> $repr {
        self.repr
      }

      /// Creates a value from its coefficient `repr`, meaning `repr / 10^Fraction`.
      ///
      /// Returns `None` if `repr > 10^(Integral + Fraction) - 1`.
      #[must_use]
      #[inline]
      pub const fn from_repr(repr: $repr) -> Option<Self> {
        if Self::in_range($($by)* repr) {
          Some(Self::from_coeff(repr))
        } else {
          None
        }
      }

      /// Parses a decimal from text, exactly.
      ///
      /// The input is ASCII text in the following grammar, in which letters match in either case.
      /// It accepts the numeric strings of the General Decimal Arithmetic specification, every
      /// form that [`f64`] parses with [`FromStr`], and `_` separators as in Rust literals. The
      /// storage holds no negative value, so a number with a leading `-` is rejected as
      /// [`Negative`](crate::ParseErrorKind::Negative), even `-0`, while the sign of the exponent
      /// may be `-`.
      ///
      /// ```text
      /// number     = [sign] (finite / nonfinite)
      /// sign       = "+" / "-"
      /// finite     = (digits ["." [digits]] / "." digits) [exponent]
      /// exponent   = ("e" / "E") [sign] exp-digits
      /// digits     = DIGIT *(DIGIT / "_")
      /// exp-digits = *"_" DIGIT *(DIGIT / "_")
      /// nonfinite  = "inf" / "infinity" / ["s"] "nan" *DIGIT
      /// ```
      ///
      /// The integral and fraction digits each start with a digit, after which `_` may appear
      /// anywhere, also repeated or at the end. The exponent's digits may have `_` anywhere after
      /// its sign, also first, but need at least one digit. So `1_000.000_1`, `1__0`, `5_.5`,
      /// `1e_1_0`, `1e+_5`, `+.5`, `5.` and `.5e3` are valid, while `_1`, `._5`, `1._5`, `1e_`,
      /// `1e_+5`, `.`, `+` and `e5` are not. Whitespace, `,`, hexadecimal and non-ASCII digits are
      /// never accepted.
      ///
      /// The value is the mantissa times ten to the power of the exponent, and it must fit the
      /// type exactly: once the exponent has moved the point, the integral part may have at most
      /// `Integral` digits after its leading zeros, and the fraction digits beyond the first
      /// `Fraction` must all be `0`. Nothing is rounded. The exponent may have any number of
      /// digits and is applied exactly, so `1000e-3` and `0.0001e4` are both one, and zero parses
      /// with any exponent, as in `0e999999999`. A decimal is always finite, so an infinity or a
      /// NaN, including a signaling `sNaN` and a NaN with diagnostic digits such as `NaN123`, is
      /// recognized but reported as an error.
      ///
      /// Runs in linear time and constant space.
      ///
      /// # Errors
      ///
      /// The first condition that applies, in this order, determines the error kind:
      ///
      /// 1. [`Empty`](crate::ParseErrorKind::Empty): the input is empty. Whitespace alone is
      ///    invalid syntax.
      /// 2. [`InvalidSyntax`](crate::ParseErrorKind::InvalidSyntax): the input does not match
      ///    the grammar. The whole input is checked before its value, so `1e999_x` is invalid
      ///    syntax rather than out of range, and `-1e999_x` rather than negative.
      /// 3. [`NotFinite`](crate::ParseErrorKind::NotFinite): the input is an infinity or a NaN,
      ///    with either sign.
      /// 4. [`Negative`](crate::ParseErrorKind::Negative): the input starts with `-`, whatever
      ///    the magnitude, so `-0` and `-1e999` are both negative.
      /// 5. [`OutOfRange`](crate::ParseErrorKind::OutOfRange): once the exponent has moved the
      ///    point, the integral part has more than `Integral` digits after its leading zeros.
      /// 6. [`Inexact`](crate::ParseErrorKind::Inexact): once the exponent has moved the point,
      ///    a fraction digit beyond the first `Fraction` is not `0`.
      ///
      /// # Examples
      ///
      /// ```
      #[doc = storage_decimal!(@use "Decimal, ParseErrorKind", unsigned $kind $repr)]
      ///
      #[doc = concat!("type D = Decimal<U1, U1, ", stringify!($repr), ">;")]
      ///
      /// for text in ["2.5", "+2.5", "02.50", "25e-1", "0.25E+1", ".25e1", "2_5e-1", "250_000e-5"] {
      #[doc = concat!("  assert_eq!(D::parse(text)?", storage_decimal!(@coeff $kind "25"), ");")]
      /// }
      /// assert_eq!(D::parse("+0e5")?, D::ZERO);
      ///
      /// let kind = |text| D::parse(text).map_err(|err| err.kind());
      /// assert_eq!(kind(""), Err(ParseErrorKind::Empty));
      /// assert_eq!(kind("1e9_x"), Err(ParseErrorKind::InvalidSyntax));
      /// assert_eq!(kind("--1"), Err(ParseErrorKind::InvalidSyntax));
      /// assert_eq!(kind("inf"), Err(ParseErrorKind::NotFinite));
      /// assert_eq!(kind("-inf"), Err(ParseErrorKind::NotFinite));
      /// assert_eq!(kind("-0"), Err(ParseErrorKind::Negative));
      /// assert_eq!(kind("-1e9"), Err(ParseErrorKind::Negative));
      /// assert_eq!(kind("1e1"), Err(ParseErrorKind::OutOfRange));
      /// assert_eq!(kind("0.05"), Err(ParseErrorKind::Inexact));
      /// // Out of range and inexact at once.
      /// assert_eq!(kind("10.05"), Err(ParseErrorKind::OutOfRange));
      /// # Ok::<(), numo::ParseError>(())
      /// ```
      pub const fn parse(s: &str) -> Result<Self, ParseError> {
        let input = s.as_bytes();
        let digits = match parse::scan(input, Self::INTEGRAL, Self::FRACTION, false) {
          Ok(digits) => digits,
          Err(err) => return Err(err),
        };
        let coeff =
          int::$repr::push_digits($($by)* int::$repr::ZERO, input, digits.start, digits.end);
        Ok(Self::from_coeff(int::$repr::mul_pow10($($by)* coeff, digits.scale)))
      }

      /// Adds two values, returning `None` if the sum is outside the range of the type.
      #[must_use]
      #[inline]
      pub const fn checked_add(&self, rhs: &Self) -> Option<Self> {
        match int::$repr::checked_add($($by)* self.repr, $($by)* rhs.repr) {
          Some(sum) if Self::in_range($($by)* sum) => Some(Self::from_coeff(sum)),
          _ => None,
        }
      }

      /// Subtracts `rhs` from `self`, returning `None` if the difference is negative.
      #[must_use]
      #[inline]
      pub const fn checked_sub(&self, rhs: &Self) -> Option<Self> {
        // A difference that is not negative is at most `self`, so it is in range.
        match int::$repr::checked_sub($($by)* self.repr, $($by)* rhs.repr) {
          Some(difference) => Some(Self::from_coeff(difference)),
          None => None,
        }
      }

      /// Rounds to a multiple of `step` with `mode`: returns the multiple of `step` that `mode`
      /// selects among the two multiples bracketing `self`, as [`RoundingMode`] describes. A
      /// multiple of `step` is returned unchanged. With [`HalfEven`](RoundingMode::HalfEven), a tie
      /// goes to the multiple with the even quotient. The value is never negative, so
      /// [`Ceiling`](RoundingMode::Ceiling) rounds as [`Up`](RoundingMode::Up) does and
      /// [`Floor`](RoundingMode::Floor) as [`Down`](RoundingMode::Down) does.
      ///
      /// `step` has the type of `self`; [`try_convert`](Self::try_convert) a step of another type
      /// first.
      ///
      /// # Errors
      ///
      /// Returns [`QuantizeError::InvalidStep`] if `step` is zero. Zero is a step like any other,
      /// of which only zero is a multiple, so no other value has a multiple to round to; unlike
      /// [`is_multiple_of`](Self::is_multiple_of), which has no error to report, this rejects it.
      /// Returns [`QuantizeError::OutOfRange`] if the selected multiple is outside the range of
      /// the type, which only rounding up can cause.
      ///
      /// # Examples
      ///
      /// ```
      #[doc = storage_decimal!(@use "Decimal, QuantizeError, RoundingMode", unsigned $kind $repr)]
      ///
      #[doc = concat!("type D = Decimal<U1, U1, ", stringify!($repr), ">;")]
      ///
      /// let step = D::parse("0.5")?;
      /// let value = D::parse("2.3")?;
      /// assert_eq!(value.quantize(&step, RoundingMode::Up)?, D::parse("2.5")?);
      /// assert_eq!(value.quantize(&step, RoundingMode::Down)?, D::parse("2")?);
      /// assert_eq!(value.quantize(&step, RoundingMode::HalfEven)?, D::parse("2.5")?);
      /// assert!(!value.is_multiple_of(&step));
      ///
      /// // 2.1 lies halfway between 2 and 2.2, which are 10 and 11 steps of 0.2.
      /// let tick = D::parse("0.2")?;
      /// let tie = D::parse("2.1")?;
      /// assert_eq!(tie.quantize(&tick, RoundingMode::HalfEven)?, D::parse("2")?);
      /// assert_eq!(tie.quantize(&tick, RoundingMode::HalfUp)?, D::parse("2.2")?);
      ///
      /// assert_eq!(value.quantize(&D::ZERO, RoundingMode::Up), Err(QuantizeError::InvalidStep));
      /// // 9.9 rounds up to 10, which has two integral digits.
      /// assert_eq!(D::MAX.quantize(&step, RoundingMode::Up), Err(QuantizeError::OutOfRange));
      /// # Ok::<(), Box<dyn core::error::Error>>(())
      /// ```
      #[must_use = "this returns the rounded value, without modifying the original"]
      pub const fn quantize(&self, step: &Self, mode: RoundingMode) -> Result<Self, QuantizeError> {
        if !int::$repr::is_positive($($by)* step.repr) {
          return Err(QuantizeError::InvalidStep);
        }
        let (_, remainder, away) =
          int::$repr::round_div($($by)* self.repr, $($by)* step.repr, mode);
        // The remainder is at most `self`, so the multiple below, `self - remainder`, exists and is
        // in range. Only the step up can leave the storage or the decimal range.
        let rounded = match int::$repr::checked_sub($($by)* self.repr, $($by)* remainder) {
          Some(below) if away => int::$repr::checked_add($($by)* below, $($by)* step.repr),
          below => below,
        };
        match rounded {
          Some(coeff) if Self::in_range($($by)* coeff) => Ok(Self::from_coeff(coeff)),
          _ => Err(QuantizeError::OutOfRange),
        }
      }

      /// Returns `true` if `self` is a multiple of `step`, an integer number of steps, without
      /// rounding anything: for example, whether a quantity is a whole number of lots.
      ///
      /// Zero is a step like any other, of which only zero is a multiple, while
      /// [`quantize`](Self::quantize) rejects a zero step.
      #[must_use]
      #[inline]
      pub const fn is_multiple_of(&self, step: &Self) -> bool {
        if int::$repr::is_zero($($by)* step.repr) {
          return int::$repr::is_zero($($by)* self.repr);
        }
        let (_, remainder) = int::$repr::div_rem($($by)* self.repr, $($by)* step.repr);
        int::$repr::is_zero($($by)* remainder)
      }
    }

    impl<I, F> UnsignedFixedDecimal for Decimal<I, F, $repr> where (I, F): Layout<$repr> {}
  };

  // The imports of an example, the listed items of the crate and the storage, in `rustfmt` order.
  (@use $items:literal, $sign:ident native $repr:ident) => {
    concat!("use numo::{", $items, ", typenum::U1};")
  };
  (@use $items:literal, signed wide $repr:ident) => {
    concat!("use numo::{", $items, ", ", stringify!($repr), ", typenum::U1};")
  };
  (@use $items:literal, unsigned wide $repr:ident) => {
    concat!("use numo::{", $items, ", typenum::U1, ", stringify!($repr), "};")
  };

  // A native coefficient compares with an integer literal, a wide one through its text.
  (@coeff native $coeff:literal) => {
    concat!(".repr(), ", $coeff)
  };
  (@coeff wide $coeff:literal) => {
    concat!(".repr().to_string(), \"", $coeff, "\"")
  };
}

storage_decimal!(
  i8: signed native,
  i16: signed native,
  i32: signed native,
  i64: signed native,
  i128: signed native,
  i192: signed wide,
  i256: signed wide,
  i320: signed wide,
  i384: signed wide,
  i448: signed wide,
  i512: signed wide,
  u8: unsigned native,
  u16: unsigned native,
  u32: unsigned native,
  u64: unsigned native,
  u128: unsigned native,
  u192: unsigned wide,
  u256: unsigned wide,
  u320: unsigned wide,
  u384: unsigned wide,
  u448: unsigned wide,
  u512: unsigned wide,
);
