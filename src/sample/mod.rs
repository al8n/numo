//! The generator and the shrinking chain that the `arbitrary`, `proptest` and `quickcheck` features
//! share. The features differ only in where the random choices come from, a [`Source`].
//!
//! A coefficient is an edge value with probability 1/8. Otherwise it is built digit by digit: its
//! number of digits is uniform in `1..=D`, where `D = Integral + Fraction`, its first digit is
//! uniform in `1..=9`, or in `0..=9` for a single digit, and each other digit is uniform in
//! `0..=9`, so the coefficient is uniform among the numbers with that many digits. A uniform
//! coefficient would instead almost always have `D` digits. With probability 1/4, a uniform number
//! of its trailing digits, fewer than all of them, is then set to zero, which makes values on a
//! coarser step, such as `64250.50`, and exact multiples frequent. A signed storage gives a
//! nonzero coefficient either sign.

#[cfg(any(feature = "proptest", feature = "quickcheck"))]
use core::marker::PhantomData;

#[cfg(any(feature = "proptest", feature = "quickcheck"))]
use crate::Decimal;
use crate::{
  int::{DIGIT_BUF_LEN, DigitBuf},
  layout::{Layout, LayoutDigits, Storage, StorageCore, Token},
  rounding::RoundingMode,
};

/// A source of random choices: the bytes of `arbitrary`, or the random number generator of
/// `proptest` or `quickcheck`.
pub(crate) trait Source {
  /// Returns a value in `0..n`. Requires `1 <= n <= 256`.
  fn below(&mut self, n: u32) -> u32;
}

/// Every rounding mode. `Down` comes first, so that shrinking moves toward it: it truncates, which
/// separates a failure of the division or quantization from one of the rounding rule.
pub(crate) static MODES: [RoundingMode; 7] = [
  RoundingMode::Down,
  RoundingMode::Up,
  RoundingMode::Ceiling,
  RoundingMode::Floor,
  RoundingMode::HalfUp,
  RoundingMode::HalfDown,
  RoundingMode::HalfEven,
];

/// Returns a rounding mode chosen uniformly.
#[cfg(any(feature = "arbitrary", feature = "quickcheck"))]
pub(crate) fn mode(source: &mut impl Source) -> RoundingMode {
  MODES[source.below(MODES.len() as u32) as usize]
}

/// Returns a coefficient in the range of `Decimal<I, F, R>`, drawn as the module describes.
pub(crate) fn coefficient<I, F, R>(source: &mut impl Source) -> R
where
  R: Storage,
  (I, F): Layout<R>,
{
  generate(
    source,
    <(I, F) as LayoutDigits<R>>::DIGITS,
    <(I, F) as LayoutDigits<R>>::FRACTION,
  )
}

/// Returns a coefficient of at most `digits` digits, `fraction` of them fraction digits. Requires
/// `fraction <= digits` and `R` to hold `10^digits`, as the storage of a layout does.
fn generate<R: Storage>(source: &mut impl Source, digits: u32, fraction: u32) -> R {
  let magnitude: R = if source.below(8) == 0 {
    edge(source, digits, fraction)
  } else {
    digit_wise(source, digits)
  };
  // Only a signed storage negates a nonzero value, so only a signed storage draws a sign.
  match magnitude.checked_neg(Token::new()) {
    Some(negated) if !magnitude.is_zero(Token::new()) && source.below(2) == 1 => negated,
    _ => magnitude,
  }
}

/// The kinds of edge value, as magnitudes in units of `10^-Fraction`. A kind with an exponent `k`
/// draws it after the kind.
#[derive(Clone, Copy)]
enum Edge {
  /// Zero.
  Zero,
  /// One unit in the last place.
  Ulp,
  /// One, `10^Fraction`, for a layout with an integral digit.
  One,
  /// The largest magnitude, `10^D - 1`.
  Max,
  /// `10^k`, for `k` in `0..D`.
  Power,
  /// `10^k - 1`, which is `k` nines, for `k` in `0..D`.
  Nines,
  /// `5 * 10^k`, for `k` in `0..D`: halfway between the multiples zero and `10^(k + 1)`, so a tie
  /// whose quotient toward zero is even.
  Five,
  /// `15 * 10^k`, for `k` in `0..D - 1`: halfway between `10^(k + 1)` and `2 * 10^(k + 1)`, a tie
  /// whose quotient toward zero is odd, which `HalfEven` rounds away from zero.
  Fifteen,
}

impl Edge {
  /// Every kind, in the order in which a choice indexes the kinds that apply.
  const ALL: [Self; 8] = [
    Self::Zero,
    Self::Ulp,
    Self::One,
    Self::Max,
    Self::Power,
    Self::Nines,
    Self::Five,
    Self::Fifteen,
  ];

  /// Returns `true` if the values of the kind fit `digits` digits, `fraction` of them fraction
  /// digits.
  const fn applies(self, digits: u32, fraction: u32) -> bool {
    match self {
      Self::One => fraction < digits,
      Self::Fifteen => digits >= 2,
      _ => true,
    }
  }
}

/// Returns an edge magnitude: a kind chosen uniformly among those that apply, then its exponent
/// chosen uniformly.
fn edge<R: Storage>(source: &mut impl Source, digits: u32, fraction: u32) -> R {
  let mut kinds = [Edge::Zero; Edge::ALL.len()];
  let mut count = 0;
  for kind in Edge::ALL {
    if kind.applies(digits, fraction) {
      kinds[count] = kind;
      count += 1;
    }
  }
  match kinds[source.below(count as u32) as usize] {
    Edge::Zero => small(0),
    Edge::Ulp => small(1),
    Edge::One => small::<R>(1).mul_pow10(fraction, Token::new()),
    Edge::Max => nines(digits),
    Edge::Power => small::<R>(1).mul_pow10(source.below(digits), Token::new()),
    Edge::Nines => nines(source.below(digits)),
    Edge::Five => small::<R>(5).mul_pow10(source.below(digits), Token::new()),
    Edge::Fifteen => small::<R>(15).mul_pow10(source.below(digits - 1), Token::new()),
  }
}

/// Returns a magnitude whose number of digits is uniform in `1..=digits` and which is uniform among
/// the numbers with that many digits, with a uniform number of its trailing digits, fewer than
/// all, set to zero with probability 1/4.
fn digit_wise<R: Storage>(source: &mut impl Source, digits: u32) -> R {
  let len = 1 + source.below(digits) as usize;
  let mut buf: DigitBuf = [b'0'; DIGIT_BUF_LEN];
  // A first digit of zero would leave a longer number fewer digits.
  let first = if len >= 2 {
    1 + source.below(9)
  } else {
    source.below(10)
  };
  buf[0] = b'0' + first as u8;
  for digit in &mut buf[1..len] {
    *digit = b'0' + source.below(10) as u8;
  }
  if source.below(4) == 0 {
    // At most `len - 1` zeros, which keep the first digit and so the number of digits.
    let zeros = source.below(len as u32) as usize;
    buf[len - zeros..len].fill(b'0');
  }
  small::<R>(0).push_digits(&buf, 0, len, Token::new())
}

/// `value` in the storage `R`.
fn small<R: Storage>(value: u8) -> R {
  value.convert(Token::new())
}

/// `10^count - 1`, which is `count` nines. Requires `count <= DIGIT_BUF_LEN` and `R` to hold it.
fn nines<R: Storage>(count: u32) -> R {
  const NINES: DigitBuf = [b'9'; DIGIT_BUF_LEN];
  small::<R>(0).push_digits(&NINES, 0, count as usize, Token::new())
}

/// Marks a type that produces `Decimal<I, F, R>` values without holding one. The marker of a
/// function returning the decimal is `Send`, `Sync` and covariant whatever `I`, `F` and `R` are.
#[cfg(any(feature = "proptest", feature = "quickcheck"))]
pub(crate) type DecimalMarker<I, F, R> = PhantomData<fn() -> Decimal<I, F, R>>;

/// The candidates that shrinking tries in place of a coefficient `v = ±m` of `d` digits, each
/// strictly simpler by `(nonzero, negative, digits)`: zero, then `m` if `v` is negative, then `v`
/// truncated toward zero to its first digit, its first two digits, and so on up to `d - 1` digits.
/// Zero has none. As an iterator, it yields the candidates as decimals.
#[cfg(any(feature = "proptest", feature = "quickcheck"))]
pub(crate) struct Chain<I, F, R> {
  value: R,
  digits: u32,
  next: u32,
  decimal: DecimalMarker<I, F, R>,
}

#[cfg(any(feature = "proptest", feature = "quickcheck"))]
impl<I, F, R> Chain<I, F, R>
where
  R: Storage,
{
  /// Returns the candidates for `value`, a coefficient in the range of `Decimal<I, F, R>`.
  pub(crate) fn new(value: R) -> Self {
    let mut buf = [0; DIGIT_BUF_LEN];
    let start = value.write_digits(&mut buf, Token::new());
    Self {
      value,
      digits: (DIGIT_BUF_LEN - start) as u32,
      next: 0,
      decimal: PhantomData,
    }
  }

  /// Returns the candidate at `index`, or `None` past the last one.
  pub(crate) fn get(&self, index: u32) -> Option<R> {
    if self.value.is_zero(Token::new()) {
      return None;
    }
    if index == 0 {
      return Some(small(0));
    }
    let mut truncations = index - 1;
    if self.value.is_negative(Token::new()) {
      if truncations == 0 {
        return self.value.checked_neg(Token::new());
      }
      truncations -= 1;
    }
    // The truncations keep `truncations + 1` digits: `v / 10^k` for `k = d - 1, ..., 1`.
    if truncations + 1 < self.digits {
      Some(
        self
          .value
          .div_rem_pow10(self.digits - 1 - truncations, Token::new())
          .0,
      )
    } else {
      None
    }
  }
}

#[cfg(any(feature = "proptest", feature = "quickcheck"))]
impl<I, F, R> Clone for Chain<I, F, R>
where
  R: Copy,
{
  fn clone(&self) -> Self {
    *self
  }
}

#[cfg(any(feature = "proptest", feature = "quickcheck"))]
impl<I, F, R> Copy for Chain<I, F, R> where R: Copy {}

#[cfg(any(feature = "proptest", feature = "quickcheck"))]
impl<I, F, R> Iterator for Chain<I, F, R>
where
  R: Storage,
{
  type Item = Decimal<I, F, R>;

  fn next(&mut self) -> Option<Decimal<I, F, R>> {
    let candidate = self.get(self.next)?;
    self.next += 1;
    // A candidate has at most the digits of the value, so it is in the range of the type.
    Some(Decimal::from_coeff(candidate))
  }
}

#[cfg(test)]
pub(crate) mod tests;
