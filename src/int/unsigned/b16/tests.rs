use crate::int::limb_layer::tests::{limb_layer_tests, native_oracle, other_limb_tests};

// The tests that every instance shares, with their helpers and the imports that the tests below
// use as well.
limb_layer_tests!(unsigned);

// The helpers of the tests below, which compare the layer with native integers and with the layer
// for `[u64; N]`.
other_limb_tests!(unsigned);

native_oracle!(unsigned one_limb_matches_u16, u16, 1);
native_oracle!(unsigned two_limbs_match_u32, u32, 2);
native_oracle!(unsigned four_limbs_match_u64, u64, 4);
native_oracle!(unsigned eight_limbs_match_u128, u128, 8);

/// Steps whose window has the top divisor limb on top, where the quotient of the window's top two
/// limbs by that limb is `2^16` or `2^16 + 1`.
#[test]
fn division_estimates_reach_two_pow_16() {
  division_with_equal_top_limbs();
}

#[test]
fn groups_of_four_limbs_match_the_u64_layer() {
  matches_the_u64_layer::<4, 1>();
  matches_the_u64_layer::<8, 2>();
  matches_the_u64_layer::<12, 3>();
  matches_the_u64_layer::<16, 4>();
  matches_the_u64_layer::<32, 8>();
}
