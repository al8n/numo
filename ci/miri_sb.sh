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
# group of modules below, in order, and a last one for every test outside the groups. Each group
# skips the groups before it, so a later group may be a prefix of an earlier one: `integer::tests::`
# runs the tests that `integer::tests::int_` and `integer::tests::uint_` did not, and `integer::`
# those that the groups before it did not. The arithmetic groups run first, so that their results,
# and the message of a failure in them, arrive early in the job: the division and the powers in
# processes of their own, and then the rest of the arithmetic.
if rustc --print cfg --target "$TARGET" | grep -qx 'target_pointer_width="32"'; then
  filters=(
    integer::arithmetic::tests::division::
    integer::arithmetic::tests::powers::
    integer::arithmetic::
    integer::tests::int_
    integer::tests::uint_
    integer::tests::
    integer::
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
  )
  skips=()
  for filter in "${filters[@]}"; do
    cargo miri test --tests --target "$TARGET" --lib -- "$filter" "${skips[@]}"
    skips+=(--skip "$filter")
  done
  cargo miri test --tests --target "$TARGET" --lib -- "${skips[@]}"
else
  cargo miri test --tests --target "$TARGET" --lib
fi
