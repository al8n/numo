use numo::{
  Int, Uint,
  typenum::{U0, U100000},
};

// 100000 bits have no default storage, and nor do zero bits.
fn too_wide(_: Uint<U100000>) {}

fn zero(_: Int<U0>) {}

// Nor does a generic width without the bound `B: BitCount`.
fn generic<B>(_: Uint<B>) {}

fn main() {}
