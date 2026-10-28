use numo::{Uint, typenum::U100000};

// 100000 bits have no default storage.
fn too_wide(_: Uint<U100000>) {}

// Nor does a generic width without the bound `B: BitCount`.
fn generic<B>(_: Uint<B>) {}

fn main() {}
