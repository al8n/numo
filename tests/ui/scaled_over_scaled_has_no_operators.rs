use numo::{Decimal, Scaled, typenum::U1};

type D = Decimal<U1, U1>;

// A `Scaled` over a decimal has the operators of the decimal.
fn add(a: Scaled<D>, b: Scaled<D>) -> Scaled<D> {
  a + b
}

// A `Scaled` over a `Scaled` is not a decimal and has none of them.
fn add_nested(a: Scaled<Scaled<D>>, b: Scaled<Scaled<D>>) -> Scaled<Scaled<D>> {
  a + b
}

fn main() {}
