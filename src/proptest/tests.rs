//! Tests of the `proptest` impls: values drawn by the strategy, the shrinking protocol of the
//! value tree, a failing run, and the strategy of a rounding mode.

use core::{fmt::Debug, marker::PhantomData};
use std::{
  format,
  string::{String, ToString},
  vec::Vec,
};

use ::proptest::{
  arbitrary::any,
  prop_assert,
  strategy::{Strategy, ValueTree},
  test_runner::{Config, RngAlgorithm, TestError, TestRng, TestRunner},
};
use num_bigint::{BigInt, Sign};
use typenum::{U0, U8, U10, U18};

use super::{DecimalStrategy, DecimalValueTree};
use crate::{
  Decimal, RoundingMode,
  layout::{Layout, LayoutDigits, Storage},
  sample::{
    Chain, MODES,
    tests::{SAMPLES, big, candidates, check_values, for_each_type},
  },
};

/// A runner with a fixed seed that writes no failure to a file.
fn runner() -> TestRunner {
  TestRunner::new_with_rng(
    Config {
      failure_persistence: None,
      ..Config::default()
    },
    TestRng::deterministic_rng(RngAlgorithm::ChaCha),
  )
}

macro_rules! check_type {
  ($t:ty) => {{
    let mut runner = runner();
    let strategy = any::<$t>();
    let values: Vec<$t> = (0..SAMPLES)
      .map(|_| strategy.new_tree(&mut runner).expect("a tree").current())
      .collect();
    check_values(stringify!($t), &values, <$t>::repr, <$t>::from_repr);
  }};
}

#[test]
fn drawn_values_cover_digits_signs_and_edges() {
  for_each_type!(check_type);
}

/// The most tests that shrinking a value of `Decimal<I, F, R>` takes, `(D + 2)^2`: the value
/// becomes simpler at most `D` times, by losing its sign once or digits otherwise, so there are at
/// most `D + 1` levels; each tries at most `D + 1` candidates; and the tree also tests the start
/// and the value it returns to at the end. The shrinking loops below check the bound as they go,
/// so that a chain that never ends fails instead of hanging.
fn max_tests<I, F, R>() -> usize
where
  (I, F): Layout<R>,
{
  let digits = <(I, F) as LayoutDigits<R>>::DIGITS as usize;
  (digits + 2).pow(2)
}

/// Drives `tree` as the runner does, testing `fails` on each value it moves to, and returns the
/// value it ends on with the number of tests.
fn shrink_tree<I, F, R>(
  mut tree: DecimalValueTree<I, F, R>,
  fails: fn(&BigInt) -> bool,
) -> (R, usize)
where
  R: Storage,
  (I, F): Layout<R>,
{
  let mut tests = 0;
  loop {
    tests += 1;
    assert!(tests <= max_tests::<I, F, R>(), "{tests} tests");
    assert_eq!(tree.current(), Decimal::from_coeff(tree.current));
    if fails(&big(tree.current)) {
      if !tree.simplify() {
        break;
      }
    } else if !tree.complicate() {
      break;
    }
  }
  (tree.current, tests)
}

/// Shrinks `start` as `quickcheck` does: tries the candidates of the value in order, and moves to
/// the first that fails, starting over from its candidates.
fn shrink_greedy<I, F, R>(start: R, fails: fn(&BigInt) -> bool) -> R
where
  R: Storage,
  (I, F): Layout<R>,
{
  let mut value = start;
  let mut index = 0;
  let mut tests = 0;
  while let Some(candidate) = Chain::<I, F, R>::new(value).get(index) {
    tests += 1;
    assert!(tests <= max_tests::<I, F, R>(), "{tests} tests");
    if fails(&big(candidate)) {
      value = candidate;
      index = 0;
    } else {
      index += 1;
    }
  }
  value
}

/// Shrinks a failing value with the tree: it ends on a failing value, every candidate of which
/// passes, within `(D + 2)^2` tests, and on the same value as the greedy shrinking of
/// `quickcheck`.
fn check_shrinking<I, F, R>(tree: DecimalValueTree<I, F, R>, fails: fn(&BigInt) -> bool)
where
  R: Storage,
  (I, F): Layout<R>,
{
  let start = tree.current;
  assert!(fails(&big(start)));
  let (end, _) = shrink_tree(tree, fails);
  let end = big(end);
  assert!(fails(&end), "{end}, shrunk from {}, passes", big(start));
  for candidate in candidates(&end) {
    assert!(
      !fails(&candidate),
      "{end} has the failing candidate {candidate}"
    );
  }
  assert_eq!(big(shrink_greedy::<I, F, R>(start, fails)), end);
}

/// Properties that fail on some values, to shrink against.
const FAILURES: [fn(&BigInt) -> bool; 5] = [
  |value| value.magnitude() >= &100_000u32.into(),
  |value| value.sign() == Sign::Minus,
  |value| value % 2u8 != BigInt::ZERO,
  |value| value.to_string().contains('7'),
  |value| value.magnitude() > &1_000u32.into() && value % 3u8 == BigInt::ZERO,
];

macro_rules! check_type_shrinking {
  ($t:ty) => {{
    let mut runner = runner();
    let strategy = any::<$t>();
    for _ in 0..256 {
      let tree = strategy.new_tree(&mut runner).expect("a tree");
      for fails in FAILURES {
        if fails(&big(tree.current)) {
          check_shrinking(tree, fails);
        }
      }
    }
  }};
}

#[test]
fn shrinking_ends_on_a_value_whose_candidates_pass() {
  for_each_type!(check_type_shrinking);
}

/// Fails iff `|c| >= 10^5`.
fn large(value: &BigInt) -> bool {
  FAILURES[0](value)
}

#[test]
fn shrinking_examples() {
  type D = Decimal<U18, U0, i64>;
  let shrink = |start: i64, fails: fn(&BigInt) -> bool| {
    check_shrinking(DecimalValueTree::<U18, U0, i64>::new(start), fails);
    shrink_tree(DecimalValueTree::<U18, U0, i64>::new(start), fails).0
  };
  assert_eq!(shrink(123_456_789, large), 123_456);
  assert_eq!(shrink(-123_456_789, large), 123_456);
  assert_eq!(shrink(-123, FAILURES[1]), -1);
  assert_eq!(shrink(D::MAX.repr(), large), 999_999);
}

/// The values that the tree moves to while `fails` holds, from `start`.
fn visits(start: i64, fails: fn(i64) -> bool) -> Vec<i64> {
  let mut tree = DecimalValueTree::<U18, U0, i64>::new(start);
  let mut visited = Vec::new();
  loop {
    visited.push(tree.current);
    assert!(visited.len() <= max_tests::<U18, U0, i64>(), "{visited:?}");
    if fails(tree.current) {
      if !tree.simplify() {
        break;
      }
    } else if !tree.complicate() {
      break;
    }
  }
  visited
}

/// After the first level the tree skips zero, which it has tried, and when every candidate passes
/// it returns to the parent, which fails.
#[test]
fn the_tree_visits_candidates_in_order() {
  assert_eq!(visits(-123, |value| value != 0), [-123, 0, 123, 1]);
  assert_eq!(
    visits(12_345, |value| value.abs() >= 100),
    [12_345, 0, 1, 12, 123, 1, 12, 123]
  );
  assert_eq!(visits(-5, |value| value < 0), [-5, 0, 5, -5]);
  assert_eq!(visits(0, |_| true), [0]);
}

/// The runner, when it gives up, calls `complicate` until it returns `false`, which must leave the
/// last failing value.
#[test]
fn complicating_until_false_returns_to_the_failing_value() {
  let mut tree = DecimalValueTree::<U18, U0, i64>::new(123_456_789);
  assert!(tree.simplify());
  assert_eq!(tree.current, 0);
  assert!(tree.complicate());
  assert_eq!(tree.current, 1);
  let mut steps = 0;
  while tree.complicate() {
    steps += 1;
    assert!(steps <= 9, "complicate ends");
  }
  assert_eq!(tree.current, 123_456_789);
  assert!(!tree.simplify());
  assert!(!tree.complicate());
  assert_eq!(tree.current, 123_456_789);
}

/// `simplify` and `complicate` may be called again after returning `false`.
#[test]
fn exhausted_trees_stay_put() {
  let mut zero = DecimalValueTree::<U18, U0, i64>::new(0);
  for _ in 0..3 {
    assert!(!zero.simplify());
    assert!(!zero.complicate());
    assert_eq!(zero.current, 0);
  }
  let mut tree = DecimalValueTree::<U18, U0, i64>::new(7);
  assert!(tree.simplify());
  assert_eq!(tree.current, 0);
  // Zero fails, so it becomes the value, which has no candidates.
  assert!(!tree.simplify());
  assert!(!tree.simplify());
  assert!(!tree.complicate());
  assert_eq!(tree.current, 0);
}

#[test]
fn a_failing_run_shrinks_to_six_digits() {
  type D = Decimal<U10, U8, i64>;
  let result = runner().run(&any::<D>(), |value| {
    prop_assert!(value.repr().unsigned_abs() < 100_000);
    Ok(())
  });
  match result {
    // A candidate of a negative value is its magnitude, and one of a value with seven or more
    // digits has six, both of which fail.
    Err(TestError::Fail(_, value)) => {
      assert!((100_000..1_000_000).contains(&value.repr()), "{value:?}");
    }
    result => panic!("expected a failure, got {result:?}"),
  }
}

/// A type with none of the traits that derived impls would require of a parameter.
struct Bare;

/// Clones `value` through `Clone` and formats the clone.
fn clone_and_format<T: Clone + Copy + Debug>(value: &T) -> String {
  format!("{:?}", value.clone())
}

#[test]
fn strategy_and_tree_impls_need_no_bounds_on_the_digit_counts() {
  let strategy = DecimalStrategy::<Bare, Bare, Bare> {
    decimal: PhantomData,
  };
  assert_eq!(clone_and_format(&strategy), "DecimalStrategy");
  let tree = DecimalValueTree::<Bare, Bare, i64>::new(-5);
  assert_eq!(
    clone_and_format(&tree),
    "DecimalValueTree { parent: -5, current: -5, cursor: None, exhausted: false }"
  );
}

#[test]
fn modes_are_uniform_choices_that_shrink_to_down() {
  let mut runner = runner();
  let strategy = any::<RoundingMode>();
  let modes: Vec<RoundingMode> = (0..SAMPLES)
    .map(|_| strategy.new_tree(&mut runner).expect("a tree").current())
    .collect();
  assert!(MODES.iter().all(|mode| modes.contains(mode)));
  for _ in 0..64 {
    let mut tree = strategy.new_tree(&mut runner).expect("a tree");
    while tree.simplify() {}
    assert_eq!(tree.current(), RoundingMode::Down);
  }
}

/// The tree and the `quickcheck` impl, shrinking greedily through `shrink`, end on the same value.
#[cfg(feature = "quickcheck")]
#[test]
fn the_tree_ends_where_quickcheck_shrinking_ends() {
  type D = Decimal<U10, U8, i64>;
  let mut runner = runner();
  let strategy = any::<D>();
  let mut compared = [0; FAILURES.len()];
  for _ in 0..SAMPLES {
    let tree = strategy.new_tree(&mut runner).expect("a tree");
    for (index, fails) in FAILURES.into_iter().enumerate() {
      let start = tree.current();
      if !fails(&big(start.repr())) {
        continue;
      }
      let mut value = start;
      let mut candidates = ::quickcheck::Arbitrary::shrink(&value);
      let mut tests = 0;
      while let Some(candidate) = candidates.next() {
        tests += 1;
        assert!(tests <= max_tests::<U10, U8, i64>(), "{tests} tests");
        if fails(&big(candidate.repr())) {
          value = candidate;
          candidates = ::quickcheck::Arbitrary::shrink(&value);
        }
      }
      assert_eq!(shrink_tree(tree, fails).0, value.repr());
      compared[index] += 1;
    }
  }
  assert!(compared.iter().all(|&count| count > 0), "{compared:?}");
}
