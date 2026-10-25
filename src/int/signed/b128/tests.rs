use crate::int::limb_layer::tests::{limb_layer_tests, native_oracle, other_limb_tests};

// The tests that every instance shares, with their helpers and the imports that the tests below
// use as well.
limb_layer_tests!(signed unsigned: b128);

// The helpers of the tests below, which compare the layer with native integers.
other_limb_tests!(signed);

native_oracle!(signed one_limb_matches_i128, i128, 1);
