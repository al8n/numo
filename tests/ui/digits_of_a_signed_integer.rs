use numo::{UnsignedDigits, i7};

// The digits in a base are those of an unsigned integer: an `Int` does not implement the trait.
fn digits<T: UnsignedDigits>(value: &T) -> Vec<u64> {
  value.to_base_le(10).collect()
}

fn main() {
  let _ = digits(&i7::new(-5));
}
