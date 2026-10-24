use numo::{
  Limbs,
  typenum::{Sum, U1, U200, U65536},
};

// 65537 bits take 1025 limbs of 64 bits, more than the lengths that have an array.
fn too_many_limbs(_: Limbs<Sum<U65536, U1>>) {}

// `i8` is not a limb type.
fn not_a_limb(_: Limbs<U200, i8>) {}

fn main() {}
