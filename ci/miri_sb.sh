#!/bin/bash
set -e

# Check if TARGET and CONFIG_FLAGS are provided, otherwise panic
if [ -z "$1" ]; then
  echo "Error: TARGET is not provided"
  exit 1
fi

if [ -z "$2" ]; then
  echo "Error: CONFIG_FLAGS are not provided"
  exit 1
fi

TARGET=$1
CONFIG_FLAGS=$2

rustup toolchain install nightly --component miri
rustup override set nightly
cargo miri setup

export MIRIFLAGS="-Zmiri-strict-provenance -Zmiri-disable-isolation -Zmiri-symbolic-alignment-check"
export RUSTFLAGS="--cfg test_$CONFIG_FLAGS"

# Libtest runs each test on a thread of its own, and Miri rarely lets a thread reuse the addresses
# that another thread freed. On a 32-bit target, the lib tests then use up the address space of one
# process, so they run in several processes there, each with a fresh address space: one for each
# group of modules below, and a last one for every test outside the groups.
if rustc --print cfg --target "$TARGET" | grep -qx 'target_pointer_width="32"'; then
  filters=(
    decimal::
    int::tests::
    int::half_limbs::
    int::unsigned::b64::
    int::unsigned::b8::
    int::unsigned::b16::
    int::unsigned::b32::
    int::unsigned::b128::
    int::signed::b64::
    int::signed::b8::
    int::signed::b16::
    int::signed::b32::
    int::signed::b128::
    integer::
  )
  skips=()
  for filter in "${filters[@]}"; do
    cargo miri test --tests --target "$TARGET" --lib -- "$filter"
    skips+=(--skip "$filter")
  done
  cargo miri test --tests --target "$TARGET" --lib -- "${skips[@]}"
else
  cargo miri test --tests --target "$TARGET" --lib
fi
