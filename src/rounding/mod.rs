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
/// conversion such as [`try_convert`](crate::Decimal::try_convert) plays, and those of the General
/// Decimal Arithmetic specification without `ROUND_05UP`. Rounded to no fraction digits:
///
/// | Value | `Up` | `Down` | `Ceiling` | `Floor` | `HalfUp` | `HalfDown` | `HalfEven` |
/// |---|---|---|---|---|---|---|---|
/// | 5.5 | 6 | 5 | 6 | 5 | 6 | 5 | 6 |
/// | 2.5 | 3 | 2 | 3 | 2 | 3 | 2 | 2 |
/// | 1.6 | 2 | 1 | 2 | 1 | 2 | 2 | 2 |
/// | 1.1 | 2 | 1 | 2 | 1 | 1 | 1 | 1 |
/// | 1.0 | 1 | 1 | 1 | 1 | 1 | 1 | 1 |
/// | -1.0 | -1 | -1 | -1 | -1 | -1 | -1 | -1 |
/// | -1.1 | -2 | -1 | -1 | -2 | -1 | -1 | -1 |
/// | -1.6 | -2 | -1 | -1 | -2 | -2 | -2 | -2 |
/// | -2.5 | -3 | -2 | -2 | -3 | -3 | -2 | -2 |
/// | -5.5 | -6 | -5 | -5 | -6 | -6 | -5 | -6 |
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
}

/// Returns `true` if `mode` rounds a quotient one step away from zero.
///
/// The quotient is that of a division rounded toward zero, so the exact value lies between it and
/// the next quotient away from zero. `negative` is the sign of the dividend, which the quotient
/// loses when it is zero. `remainder_is_zero` tells whether the division is exact. `half` compares
/// the magnitude of the remainder `r` with its distance to the divisor `d`, `|r|` with `d - |r|`,
/// which is twice the remainder against the divisor without overflow: `Less` puts the value nearer
/// the quotient, `Greater` nearer the next one, and `Equal` halfway between. `quotient_is_odd` is
/// the parity of the quotient's magnitude.
///
/// For a divisor `10^k`, `half` is what decimal implementations read from the first dropped digit
/// and whether any later dropped digit is nonzero; the comparison works for any divisor, which
/// `quantize` needs.
pub(crate) const fn increments(
  mode: RoundingMode,
  negative: bool,
  remainder_is_zero: bool,
  half: Ordering,
  quotient_is_odd: bool,
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
  }
}

#[cfg(test)]
mod tests;
