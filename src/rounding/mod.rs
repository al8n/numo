//! Rounding modes and the rule that applies them.

use core::cmp::Ordering;

/// How to round a value that lies between two representable results.
///
/// [`Decimal::round_to`](crate::Decimal::round_to) rounds to fewer fraction digits,
/// [`Decimal::quantize`](crate::Decimal::quantize) rounds to a multiple of a step, and
/// [`Decimal::div_round`](crate::Decimal::div_round) rounds a quotient. Each takes the mode as an
/// argument; there is no default mode. In every mode, a value that is already representable is
/// returned unchanged, and a negative value that rounds to zero gives zero, which has no sign.
///
/// The modes are those of Java's `RoundingMode` without `UNNECESSARY`, whose part an exact
/// conversion such as [`try_convert`](crate::Decimal::try_convert) plays, and all eight of the
/// General Decimal Arithmetic specification. Rounded to no fraction digits:
///
/// | Value | `Up` | `Down` | `Ceiling` | `Floor` | `HalfUp` | `HalfDown` | `HalfEven` | `Up05` |
/// |---|---|---|---|---|---|---|---|---|
/// | 5.5 | 6 | 5 | 6 | 5 | 6 | 5 | 6 | 6 |
/// | 2.5 | 3 | 2 | 3 | 2 | 3 | 2 | 2 | 2 |
/// | 1.6 | 2 | 1 | 2 | 1 | 2 | 2 | 2 | 1 |
/// | 1.1 | 2 | 1 | 2 | 1 | 1 | 1 | 1 | 1 |
/// | 1.0 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 |
/// | 0.1 | 1 | 0 | 1 | 0 | 0 | 0 | 0 | 1 |
/// | -0.1 | -1 | 0 | 0 | -1 | 0 | 0 | 0 | -1 |
/// | -1.0 | -1 | -1 | -1 | -1 | -1 | -1 | -1 | -1 |
/// | -1.1 | -2 | -1 | -1 | -2 | -1 | -1 | -1 | -1 |
/// | -1.6 | -2 | -1 | -1 | -2 | -2 | -2 | -2 | -1 |
/// | -2.5 | -3 | -2 | -2 | -3 | -3 | -2 | -2 | -2 |
/// | -5.5 | -6 | -5 | -5 | -6 | -6 | -5 | -6 | -6 |
///
/// ```
/// use numo::{
///   Decimal, RoundingMode,
///   typenum::{U0, U1},
/// };
///
/// let value: Decimal<U1, U1> = "-2.5".parse()?;
/// let rounded: Decimal<U1, U0> = value.round_to(RoundingMode::HalfEven)?;
/// assert_eq!(rounded.to_string(), "-2");
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
///
/// Rounding twice can give another result than rounding once, unless the first rounding is
/// [`Up05`](Self::Up05):
///
/// ```
/// use numo::{
///   Decimal, RoundingMode,
///   typenum::{U1, U2, U3},
/// };
///
/// let value: Decimal<U1, U3> = "2.449".parse()?;
/// let twice: Decimal<U1, U2> = value.round_to(RoundingMode::HalfUp)?;
/// let twice: Decimal<U1, U1> = twice.round_to(RoundingMode::HalfUp)?;
/// assert_eq!(twice.to_string(), "2.5");
///
/// let first: Decimal<U1, U2> = value.round_to(RoundingMode::Up05)?;
/// assert_eq!(first.to_string(), "2.44");
/// let then: Decimal<U1, U1> = first.round_to(RoundingMode::HalfUp)?;
/// let once: Decimal<U1, U1> = value.round_to(RoundingMode::HalfUp)?;
/// assert_eq!(then, once);
/// assert_eq!(once.to_string(), "2.4");
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::IsVariant, derive_more::Display)]
#[display("{}", self.as_str())]
pub enum RoundingMode {
  /// Away from zero.
  Up,
  /// Toward zero, which truncates.
  Down,
  /// Toward positive infinity.
  Ceiling,
  /// Toward negative infinity.
  Floor,
  /// To the nearer result; a tie goes away from zero.
  HalfUp,
  /// To the nearer result; a tie goes toward zero.
  HalfDown,
  /// To the nearer result; a tie goes to the even one, which is the result whose last digit is
  /// even for [`round_to`](crate::Decimal::round_to) and [`div_round`](crate::Decimal::div_round),
  /// and the multiple with the even quotient for [`quantize`](crate::Decimal::quantize). Also known
  /// as banker's rounding.
  HalfEven,
  /// Away from zero if the last digit of the result rounded toward zero is 0 or 5, and toward zero
  /// otherwise: `ROUND_05UP` of the General Decimal Arithmetic specification. The digit is the last
  /// kept one for [`round_to`](crate::Decimal::round_to), the last digit of the quotient for
  /// [`div_round`](crate::Decimal::div_round), and the last decimal digit of the number of steps
  /// for [`quantize`](crate::Decimal::quantize).
  ///
  /// A result that is not exact thus never ends in 0 or 5, so rounding it again to fewer digits, in
  /// any mode, gives what rounding the exact value once gives. This holds for the roundings at a
  /// power of ten: `round_to`, `div_round`, and `quantize` with a step `10^-k`. This makes it the
  /// mode for an intermediate result that a later step rounds again.
  #[is_variant(ignore)]
  Up05,
}

impl RoundingMode {
  /// Returns the string representation of the rounding mode.
  #[inline(always)]
  pub const fn as_str(&self) -> &'static str {
    match self {
      Self::Up => "up",
      Self::Down => "down",
      Self::Ceiling => "ceiling",
      Self::Floor => "floor",
      Self::HalfUp => "halfup",
      Self::HalfDown => "halfdown",
      Self::HalfEven => "halfeven",
      Self::Up05 => "up05",
    }
  }

  /// Returns `true` if this value is of type `Up05`. Returns `false` otherwise
  #[must_use]
  #[inline]
  pub const fn is_up05(&self) -> bool {
    matches!(self, Self::Up05)
  }
}

/// Returns `true` if `mode` rounds a quotient one step away from zero.
///
/// The quotient is that of a division rounded toward zero, so the exact value lies between it and
/// the next quotient away from zero. `negative` is the sign of the dividend, which the quotient
/// loses when it is zero. `remainder_is_zero` tells whether the division is exact. `half` compares
/// the magnitude of the remainder `r` with its distance to the divisor `d`, `|r|` with `d - |r|`,
/// which is twice the remainder against the divisor without overflow: `Less` puts the value nearer
/// the quotient, `Greater` nearer the next one, and `Equal` halfway between. `quotient_is_odd` is
/// the parity of the quotient's magnitude. `quotient_ends_in_0_or_5` tells whether the last decimal
/// digit of the quotient's magnitude is 0 or 5; only [`RoundingMode::Up05`] reads it.
///
/// For a divisor `10^k`, `half` is what decimal implementations read from the first dropped digit
/// and whether any later dropped digit is nonzero; the comparison works for any divisor, which
/// `quantize` needs.
#[inline(always)]
pub(crate) const fn increments(
  mode: RoundingMode,
  negative: bool,
  remainder_is_zero: bool,
  half: Ordering,
  quotient_is_odd: bool,
  quotient_ends_in_0_or_5: bool,
) -> bool {
  if remainder_is_zero {
    return false;
  }
  match mode {
    RoundingMode::Up => true,
    RoundingMode::Down => false,
    RoundingMode::Ceiling => !negative,
    RoundingMode::Floor => negative,
    RoundingMode::HalfUp => !matches!(half, Ordering::Less),
    RoundingMode::HalfDown => matches!(half, Ordering::Greater),
    RoundingMode::HalfEven => match half {
      Ordering::Less => false,
      Ordering::Equal => quotient_is_odd,
      Ordering::Greater => true,
    },
    RoundingMode::Up05 => quotient_ends_in_0_or_5,
  }
}

#[cfg(test)]
mod tests;
