#!/bin/bash

set -ex

export ASAN_OPTIONS="detect_odr_violation=0 detect_leaks=0"

# Run address sanitizer
RUSTFLAGS="-Z sanitizer=address --cfg all_tests" \
cargo test --tests --target x86_64-unknown-linux-gnu --all-features

# Run leak sanitizer
RUSTFLAGS="-Z sanitizer=leak --cfg all_tests" \
cargo test --tests --target x86_64-unknown-linux-gnu --all-features

# The sanitizers below build the standard library with -Zbuild-std, unoptimized and instrumented,
# which enlarges the stack frames of the tests: give each test thread 64 MiB instead of 2 MiB. Under
# the memory sanitizer, a test thread that overflows its stack ends the process with SIGSEGV and no
# report.
# Run memory sanitizer
RUST_MIN_STACK=67108864 \
RUSTFLAGS="-Z sanitizer=memory --cfg all_tests" \
cargo -Zbuild-std test --tests --target x86_64-unknown-linux-gnu --all-features

# Run thread sanitizer
RUST_MIN_STACK=67108864 \
RUSTFLAGS="-Z sanitizer=thread --cfg all_tests" \
cargo -Zbuild-std test --tests --target x86_64-unknown-linux-gnu --all-features
