#!/bin/bash

set -ex

export ASAN_OPTIONS="detect_odr_violation=0 detect_leaks=0"

# Run address sanitizer
RUSTFLAGS="-Z sanitizer=address --cfg all_tests" \
cargo test --tests --target x86_64-unknown-linux-gnu --all-features

# Run leak sanitizer
RUSTFLAGS="-Z sanitizer=leak --cfg all_tests" \
cargo test --tests --target x86_64-unknown-linux-gnu --all-features

# The memory and thread sanitizers build the standard library too, and run on optimized code: built
# without optimization, the memory sanitizer's build of the long exhaustive arithmetic tests uses
# ever more stack as they run, and exhausts even a 64 MiB thread stack. Optimization level 1 keeps
# the debug assertions and the overflow checks of the test profile.
export CARGO_PROFILE_DEV_OPT_LEVEL=1 CARGO_PROFILE_TEST_OPT_LEVEL=1

# Run memory sanitizer
RUSTFLAGS="-Z sanitizer=memory --cfg all_tests" \
cargo -Zbuild-std test --tests --target x86_64-unknown-linux-gnu --all-features

# Run thread sanitizer
RUSTFLAGS="-Z sanitizer=thread --cfg all_tests" \
cargo -Zbuild-std test --tests --target x86_64-unknown-linux-gnu --all-features
