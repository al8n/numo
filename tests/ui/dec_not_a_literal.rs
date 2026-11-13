use numo::{Decimal, dec, typenum::U1};

type D = Decimal<U1, U1>;

// The macro reads one Rust number token with an optional `-` before it: not a `+`, which is not
// part of a number token, and not a second point.
const _: D = dec!(+1.5);
const _: D = dec!(1.5.5);

fn main() {}
