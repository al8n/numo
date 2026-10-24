use numo::{i7, u7};

// `new` panics for a value that does not fit, and in a constant the panic fails to compile.
const TOO_LARGE: u7 = u7::new(128);
const TOO_SMALL: i7 = i7::new(-65);

fn main() {
  let _ = (TOO_LARGE, TOO_SMALL);
}
