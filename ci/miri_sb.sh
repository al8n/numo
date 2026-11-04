#!/bin/bash
set -e

# Check if TARGET, CONFIG_FLAGS and SHARD are provided, otherwise panic
if [ -z "$1" ]; then
  echo "Error: TARGET is not provided"
  exit 1
fi

if [ -z "$2" ]; then
  echo "Error: CONFIG_FLAGS are not provided"
  exit 1
fi

if [ -z "$3" ]; then
  echo "Error: SHARD is not provided"
  exit 1
fi

TARGET=$1
CONFIG_FLAGS=$2
SHARD=$3

# shellcheck source-path=SCRIPTDIR source=miri_groups.sh
source "$(dirname "${BASH_SOURCE[0]}")/miri_groups.sh"
miri_check_shard "$SHARD"

rustup toolchain install nightly --component miri
rustup override set nightly
cargo miri setup

export MIRIFLAGS="-Zmiri-strict-provenance -Zmiri-disable-isolation -Zmiri-symbolic-alignment-check"
export RUSTFLAGS="--cfg test_$CONFIG_FLAGS"

# Every target runs its lib tests in groups, each group in a process of its own: a 32-bit target
# runs out of address space in one process, and the lib tests of any target take longer than the six
# hours of a job. A job runs one shard of the groups. The groups, their order and the shard of each
# group are in `miri_groups.sh`, which explains them.
miri_run_shard "$TARGET" "$SHARD"
