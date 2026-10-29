//! Tests of the `quickcheck` impls: values drawn from a seeded generator, shrinking, and the impl
//! of a rounding mode.

use std::vec::Vec;

use ::quickcheck::{Arbitrary, Gen};
use num_bigint::BigInt;
use typenum::{U0, U18};

use crate::{
  Decimal, RoundingMode,
  layout::{Layout, LayoutDigits, Storage},
  sample::{
    MODES,
    tests::{SAMPLES, big, candidates, check_values, for_each_type},
  },
};

/// A generator with a fixed seed.
fn seeded() -> Gen {
  Gen::from_size_and_seed(100, 0x9c4e_0001)
}

/// `shrink` yields the candidates of the value, a finite number of them, and never the value.
fn check_shrink<I, F, R>(value: Decimal<I, F, R>)
where
  R: Storage + 'static,
  I: 'static,
  F: 'static,
  (I, F): Layout<R>,
{
  let digits = <(I, F) as LayoutDigits<R>>::DIGITS as usize;
  let shrunk: Vec<Decimal<I, F, R>> = value.shrink().take(digits + 2).collect();
  assert!(shrunk.len() <= digits + 1, "{value:?}");
  assert!(!shrunk.contains(&value), "{value:?}");
  let shrunk: Vec<BigInt> = shrunk
    .into_iter()
    .map(|candidate| big(candidate.coeff()))
    .collect();
  assert_eq!(shrunk, candidates(&big(value.coeff())), "{value:?}");
}

macro_rules! check_type {
  ($t:ty) => {{
    let mut g = seeded();
    let values: Vec<$t> = (0..SAMPLES)
      .map(|_| <$t as Arbitrary>::arbitrary(&mut g))
      .collect();
    check_values(stringify!($t), &values, <$t>::repr, <$t>::from_repr);
    for &value in &values {
      check_shrink(value);
    }
  }};
}

#[test]
fn drawn_values_cover_digits_signs_and_edges_and_shrink() {
  for_each_type!(check_type);
}

type D = Decimal<U18, U0, i64>;

/// Shrinks `start` as `quickcheck` does: moves to the first candidate that fails and starts over
/// from its candidates. The value of 18 digits becomes simpler at most 18 times, by losing its sign
/// once or digits otherwise, and each value has at most 19 candidates, so a chain that never ends
/// fails the bound instead of hanging.
fn shrink_greedy(start: D, fails: fn(D) -> bool) -> D {
  let mut value = start;
  let mut candidates = value.shrink();
  let mut tests = 0;
  while let Some(candidate) = candidates.next() {
    tests += 1;
    assert!(tests <= 19 * 19, "{tests} tests from {start:?}");
    if fails(candidate) {
      value = candidate;
      candidates = value.shrink();
    }
  }
  value
}

#[test]
fn greedy_shrinking_ends_on_a_value_whose_candidates_pass() {
  let large: fn(D) -> bool = |value| value.repr().unsigned_abs() >= 100_000;
  let shrink = |start| shrink_greedy(D::from_repr(start).unwrap(), large).repr();
  assert_eq!(shrink(123_456_789), 123_456);
  assert_eq!(shrink(-123_456_789), 123_456);
  assert_eq!(shrink(D::MAX.repr()), 999_999);

  let failures: [fn(D) -> bool; 3] = [
    large,
    |value| value.is_negative(),
    |value| value.repr() % 2 != 0,
  ];
  let mut g = seeded();
  for _ in 0..SAMPLES {
    let start = D::arbitrary(&mut g);
    for fails in failures {
      if fails(start) {
        let end = shrink_greedy(start, fails);
        assert!(fails(end), "{end:?}, shrunk from {start:?}");
        assert!(end.shrink().all(|candidate| !fails(candidate)), "{end:?}");
      }
    }
  }
}

#[test]
fn modes_are_uniform_choices_that_shrink_toward_down() {
  let mut g = seeded();
  let modes: Vec<RoundingMode> = (0..SAMPLES)
    .map(|_| RoundingMode::arbitrary(&mut g))
    .collect();
  assert!(MODES.iter().all(|mode| modes.contains(mode)));
  for (index, &mode) in MODES.iter().enumerate() {
    let shrunk: Vec<RoundingMode> = mode.shrink().collect();
    assert_eq!(shrunk, MODES[..index], "{mode:?}");
  }
}
