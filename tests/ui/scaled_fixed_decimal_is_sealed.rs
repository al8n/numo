use numo::{
  Decimal, FixedDecimal, Scaled,
  typenum::{U0, U1},
};

// A `Scaled` over a type of another crate cannot be given the decimal traits by that crate: the
// orphan rule stops the impl before the seal does.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
struct Local;

impl FixedDecimal for Scaled<Local> {
  const INTEGRAL: u32 = 1;
  const FRACTION: u32 = 0;
  const DIGITS: u32 = 1;
  const SIGNED: bool = false;

  type Repr = u8;

  type Inline = Scaled<Decimal<U1, U0, i8>>;

  fn to_inline(&self) -> Scaled<Decimal<U1, U0, i8>> {
    Scaled::new(Decimal::<U1, U0, i8>::ZERO, 0)
  }

  fn from_inline(_: Scaled<Decimal<U1, U0, i8>>) -> Self {
    Scaled::new(Local, 0)
  }

  fn repr(self) -> u8 {
    0
  }

  fn from_repr(_: u8) -> Option<Self> {
    None
  }

  fn zero() -> Self {
    Scaled::new(Local, 0)
  }

  fn max_value() -> Self {
    Scaled::new(Local, 0)
  }

  fn min_value() -> Self {
    Scaled::new(Local, 0)
  }

  fn is_zero(&self) -> bool {
    true
  }

  fn is_positive(&self) -> bool {
    false
  }

  fn checked_add(&self, _: &Self) -> Option<Self> {
    None
  }

  fn checked_sub(&self, _: &Self) -> Option<Self> {
    None
  }

  fn quantize(&self, _: &Self, _: numo::RoundingMode) -> Result<Self, numo::QuantizeError> {
    Err(numo::QuantizeError::InvalidStep)
  }

  fn is_multiple_of(&self, _: &Self) -> bool {
    true
  }
}

fn main() {}
