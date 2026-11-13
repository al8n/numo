use core::{
  fmt,
  iter::Sum,
  ops::{Add, Sub},
  str::FromStr,
};

use numo::{ConvertError, FixedDecimal, ParseError, QuantizeError, RoundingMode};

// The seal of `FixedDecimal` holds even for a type with every supertrait and item.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
struct Local;

impl fmt::Display for Local {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("0")
  }
}

impl FromStr for Local {
  type Err = ParseError;

  fn from_str(_: &str) -> Result<Self, ParseError> {
    Ok(Self)
  }
}

impl Add for Local {
  type Output = Self;

  fn add(self, _: Self) -> Self {
    Self
  }
}

impl Sub for Local {
  type Output = Self;

  fn sub(self, _: Self) -> Self {
    Self
  }
}

impl Sum for Local {
  fn sum<I: Iterator<Item = Self>>(_: I) -> Self {
    Self
  }
}

impl FixedDecimal for Local {
  const INTEGRAL: u32 = 1;
  const FRACTION: u32 = 0;
  const DIGITS: u32 = 1;
  const SIGNED: bool = false;

  type Repr = u8;

  fn repr(self) -> u8 {
    0
  }

  fn from_repr(_: u8) -> Option<Self> {
    Some(Self)
  }

  fn zero() -> Self {
    Self
  }

  fn max_value() -> Self {
    Self
  }

  fn min_value() -> Self {
    Self
  }

  fn is_zero(&self) -> bool {
    true
  }

  fn is_positive(&self) -> bool {
    false
  }

  fn checked_add(&self, _: &Self) -> Option<Self> {
    Some(Self)
  }

  fn checked_sub(&self, _: &Self) -> Option<Self> {
    Some(Self)
  }

  fn checked_rem(&self, _: &Self) -> Option<Self> {
    Some(Self)
  }

  fn saturating_add(&self, _: &Self) -> Self {
    Self
  }

  fn saturating_sub(&self, _: &Self) -> Self {
    Self
  }

  fn quantize(&self, _: &Self, _: RoundingMode) -> Result<Self, QuantizeError> {
    Ok(Self)
  }

  fn is_multiple_of(&self, _: &Self) -> bool {
    true
  }

  fn trunc(&self) -> Self {
    Self
  }

  fn fract(&self) -> Self {
    Self
  }

  fn floor(&self) -> Result<Self, ConvertError> {
    Ok(Self)
  }

  fn ceil(&self) -> Result<Self, ConvertError> {
    Ok(Self)
  }

  fn is_integer(&self) -> bool {
    true
  }

  fn order_of_magnitude(&self) -> i32 {
    0
  }

  fn significant_digits(&self) -> u32 {
    1
  }

  fn fraction_digits(&self) -> u32 {
    0
  }

  fn round_significant(
    &self,
    _: core::num::NonZero<u32>,
    _: RoundingMode,
  ) -> Result<Self, ConvertError> {
    Ok(Self)
  }
}

fn main() {}
