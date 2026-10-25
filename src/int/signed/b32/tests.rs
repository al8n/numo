use crate::int::limb_layer::tests::{limb_layer_tests, native_oracle, other_limb_tests};

// The tests that every instance shares, with their helpers and the imports that the tests below
// use as well.
limb_layer_tests!(signed unsigned: b32);

// The helpers of the tests below, which compare the layer with native integers.
other_limb_tests!(signed);

native_oracle!(signed one_limb_matches_i32, i32, 1);
native_oracle!(signed two_limbs_match_i64, i64, 2);
native_oracle!(signed four_limbs_match_i128, i128, 4);
