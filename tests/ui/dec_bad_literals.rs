use numo::{Decimal, dec, typenum::U1};

type D = Decimal<U1, U1>;
type U = Decimal<U1, U1, u8>;

// A literal that its type does not hold fails to compile with the message of its kind, here in
// constant items, which a check evaluates; the first one compiles. The snapshot quotes the
// constant of the literal and the first and last lines of the list of storages in the library, so
// it changes with them, with the names of the hidden items of the expansion, and with a new Rust
// release. The last two fail alike, and have two types, so that the compiler does not merge their
// errors into one.
const _: D = dec!(9.9);
const _: D = dec!(0.25);
const _: D = dec!(10);
const _: U = dec!(-0.5);
const _: D = dec!(0x10);
const _: U = dec!(1.5f32);

fn main() {}
