# shellcheck shell=bash
#
# The groups of lib tests that the Miri scripts run, shared by `miri_tb.sh` and `miri_sb.sh`.
#
# Every target runs its lib tests as a series of groups, each group in a process of its own:
#
# - Libtest runs each test on a thread of its own, and Miri rarely lets a thread reuse the addresses
#   that another thread freed. On a 32-bit target, the lib tests then use up the address space of
#   one process, and a fresh process starts with a fresh address space.
# - Miri runs the lib tests of one target for far longer than the six hours that a GitHub job may
#   take. The groups are spread over shards, one job for each shard and target, and a job runs only
#   the groups of its shard.
#
# The order of `miri_groups` is global, whichever shard a group belongs to. Each group skips every
# group before it, so a later group may be a prefix of an earlier one: `integer::tests::` runs the
# tests that `integer::tests::int_` and `integer::tests::uint_` did not, and `integer::` those that
# the groups before it did not. A last process, which runs in the shard `miri_rest_shard`, runs
# every test outside the groups, with every group skipped.
#
# The shard of a group balances the shards by the time that Miri takes for the groups. Rebalance the
# map when a shard approaches the six hours of a job: moving a group to another shard changes
# neither the order nor the skips of any group.

# The number of shards, numbered from 1.
miri_shards=10

# The shard that also runs the tests outside the groups.
miri_rest_shard=3

# One group in each element, as `<shard> <filter>`, in the global order.
miri_groups=(
  "1 integer::arithmetic::tests::division::"
  "1 integer::arithmetic::tests::powers::"
  "6 integer::arithmetic::tests::bits::"
  "6 integer::arithmetic::tests::shifts::"
  "10 integer::arithmetic::tests::nonzero::"
  "5 integer::arithmetic::tests::text::"
  "3 integer::arithmetic::tests::bytes::"
  "3 integer::arithmetic::tests::casts::"
  "1 integer::arithmetic::"
  "2 integer::tests::int_"
  "2 integer::tests::uint_"
  "2 integer::tests::"
  "3 integer::"
  "3 decimal::"
  "4 int::tests::"
  "3 int::half_limbs::"
  "5 int::unsigned::b64::"
  "6 int::unsigned::b8::"
  "6 int::unsigned::b16::"
  "7 int::unsigned::b32::"
  "8 int::unsigned::b128::"
  "9 int::signed::b64::"
  "7 int::signed::b8::"
  "9 int::signed::b16::"
  "10 int::signed::b32::"
  "10 int::signed::b128::"
  # The tests of `tests/oracle.rs` have no module path of the crate, so these groups match parts of
  # their names. They come last, so they take only the tests that no group before them matched.
  "4 ::arithmetic"
  "5 ::parse"
  "7 convert_"
  "8 ::exp"
  "8 ::text"
  "9 round_"
)

# Exit with an error unless the argument is a shard number, from 1 to `miri_shards`.
miri_check_shard() {
  local shard=$1 number
  for ((number = 1; number <= miri_shards; number++)); do
    if [ "$shard" = "$number" ]; then
      return 0
    fi
  done
  echo "Error: SHARD must be a number from 1 to $miri_shards, got '$shard'"
  exit 1
}

# Run the groups of one shard on one target. The Miri flags and the rustflags come from the caller.
miri_run_shard() {
  local target=$1 shard=$2 group group_shard filter
  local skips=()
  for group in "${miri_groups[@]}"; do
    read -r group_shard filter <<<"$group"
    if [ "$group_shard" = "$shard" ]; then
      cargo miri test --tests --target "$target" --lib -- "$filter" "${skips[@]}"
    fi
    skips+=(--skip "$filter")
  done
  if [ "$shard" = "$miri_rest_shard" ]; then
    cargo miri test --tests --target "$target" --lib -- "${skips[@]}"
  fi
}
