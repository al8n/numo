use numo::u7;

// A radix above 256 panics, and in a constant the panic fails to compile.
const DIGITS: Option<u7> = u7::checked_from_radix_be(&[1], 257);

fn main() {
  let _ = DIGITS;
}
