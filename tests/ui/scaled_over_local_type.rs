use numo::{FixedDecimal, Scaled};

// A `Scaled` over a type of another crate is not a decimal: the type is not one, and nothing gives
// the `Scaled` the `FromStr` that a decimal needs.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
struct Local;

fn total<T: FixedDecimal>(values: &[T]) -> Option<T> {
  values.iter().try_fold(T::zero(), |sum, value| sum.checked_add(value))
}

fn main() {
  let _ = total::<Scaled<Local>>(&[]);
}
