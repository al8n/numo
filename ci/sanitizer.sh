#!/bin/bash

set -ex

# The builds of numo's tests fill a standard runner when they keep incremental caches and full debug
# info, so build without the caches and keep only the file and line tables that reports need.
export CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=line-tables-only
export CARGO_PROFILE_TEST_DEBUG=line-tables-only

usage() {
  echo "usage: ci/sanitizer.sh <address|leak|memory|thread>" >&2
  exit 2
}

[ "$#" -eq 1 ] || usage

case "$1" in
  address | leak)
    export ASAN_OPTIONS="detect_odr_violation=0 detect_leaks=0"

    RUSTFLAGS="-Z sanitizer=$1 --cfg all_tests" \
    cargo test --tests --target x86_64-unknown-linux-gnu --all-features
    ;;

  # The memory and thread sanitizers build the standard library too, and run on optimized code: built
  # without optimization, the memory sanitizer's build of the long exhaustive arithmetic tests uses
  # ever more stack as they run, and exhausts even a 64 MiB thread stack. Optimization level 1 keeps
  # the debug assertions and the overflow checks of the test profile.
  memory | thread)
    export CARGO_PROFILE_DEV_OPT_LEVEL=1 CARGO_PROFILE_TEST_OPT_LEVEL=1

    RUSTFLAGS="-Z sanitizer=$1 --cfg all_tests" \
    cargo -Zbuild-std test --tests --target x86_64-unknown-linux-gnu --all-features
    ;;

  *)
    usage
    ;;
esac
