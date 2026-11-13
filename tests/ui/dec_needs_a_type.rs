use numo::{Decimal, FixedDecimal, dec, typenum::U1};

type D = Decimal<U1, U1>;

// A literal takes its type from its context, and these contexts give it none: no expected type,
// the left operand of `+` and of `<`, and code generic over the decimal. Each is in a function of
// its own, because the compiler reports only the first unknown type of a function.
fn no_expected_type() {
  let _value = dec!(0.5);
}

fn left_operand(value: D) -> D {
  dec!(0.5) + value
}

fn left_of_a_comparison(value: D) -> bool {
  dec!(0.5) < value
}

fn generic<T: FixedDecimal>() -> T {
  dec!(0.5)
}

// The same literal compiles on the right of a value, and with its type named on the left.
fn typed(value: D) -> bool {
  value + dec!(0.5) == dec!(0.5 as D) + value && dec!(0.5 as D) < value
}

fn main() {
  no_expected_type();
  let _ = left_operand(D::ZERO);
  let _ = left_of_a_comparison(D::ZERO);
  let _: D = generic();
  let _ = typed(D::ZERO);
}
