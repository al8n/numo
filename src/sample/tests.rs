//! Tests of the generator, the chain and the table of rounding modes, and the helpers that the
//! tests of each feature share: a deterministic source, the tested types and the check of drawn
//! values.
//!
//! The generator is checked exhaustively: every path of choices through the edge branch for types
//! of every size, and every path through the digit-wise branch for types of up to four digits,
//! with the exact probability of each value. Values drawn from a seeded source check the larger
//! types, as the tests of each feature do with that feature's source.

use std::{
  collections::{BTreeMap, BTreeSet},
  string::ToString,
  vec,
  vec::Vec,
};

use num_bigint::{BigInt, Sign};

#[cfg(any(feature = "proptest", feature = "quickcheck"))]
use super::Chain;
use super::{MODES, Source, coefficient, generate};
use crate::{
  Decimal, RoundingMode, bigint, i256, i512,
  layout::{Layout, LayoutDigits, Storage, Token},
  u256, u512,
};

/// The number of values that each test draws for each type.
pub(crate) const SAMPLES: usize = 4096;

/// Calls `$check!(Type)` for each decimal type that the tests of every feature draw: native and
/// wide storages, signed and unsigned, of 2, 18, 19, 38, 76, 77, 153 and 154 digits, among them
/// layouts with only fraction digits and with none.
macro_rules! for_each_type {
  ($check:ident) => {
    $check!($crate::Decimal<::typenum::U1, ::typenum::U1, i8>);
    $check!($crate::Decimal<::typenum::U0, ::typenum::U2, i8>);
    $check!($crate::Decimal<::typenum::U2, ::typenum::U0, i8>);
    $check!($crate::Decimal<::typenum::U1, ::typenum::U1, u8>);
    $check!($crate::Decimal<::typenum::U0, ::typenum::U2, u8>);
    $check!($crate::Decimal<::typenum::U2, ::typenum::U0, u8>);
    $check!($crate::Decimal<::typenum::U10, ::typenum::U8, i64>);
    $check!($crate::Decimal<::typenum::U10, ::typenum::U9, u64>);
    $check!($crate::Decimal<::typenum::U19, ::typenum::U19, i128>);
    $check!($crate::Decimal<::typenum::U38, ::typenum::U0, u128>);
    $check!($crate::Decimal<::typenum::U38, ::typenum::U38, $crate::i256>);
    $check!($crate::Decimal<::typenum::U59, ::typenum::U18, $crate::u256>);
    $check!($crate::Decimal<::typenum::U0, ::typenum::U153, $crate::i512>);
    $check!($crate::Decimal<::typenum::U154, ::typenum::U0, $crate::u512>);
  };
}

pub(crate) use for_each_type;

/// SplitMix64, a small deterministic source for the tests.
pub(crate) struct SplitMix(pub(crate) u64);

impl SplitMix {
  pub(crate) fn next_u64(&mut self) -> u64 {
    self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = self.0;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
  }
}

impl Source for SplitMix {
  fn below(&mut self, n: u32) -> u32 {
    // The top 32 bits scaled to `0..n`.
    (((self.next_u64() >> 32) * u64::from(n)) >> 32) as u32
  }
}

/// Answers `below` from a list, checking each answer against its bound.
struct Script<'a> {
  answers: &'a [u32],
  next: usize,
}

impl Source for Script<'_> {
  fn below(&mut self, n: u32) -> u32 {
    assert!(
      self.next < self.answers.len(),
      "an unexpected choice among {n}"
    );
    let answer = self.answers[self.next];
    assert!(
      answer < n,
      "answer {answer} at {} is not below {n}",
      self.next
    );
    self.next += 1;
    answer
  }
}

/// The coefficient that `answers` give for `digits` digits, `fraction` of them fraction digits.
/// Every answer must be drawn.
fn scripted<R: Storage>(digits: u32, fraction: u32, answers: &[u32]) -> BigInt {
  let mut script = Script { answers, next: 0 };
  let coeff: R = generate(&mut script, digits, fraction);
  assert_eq!(script.next, answers.len(), "every answer is drawn");
  big(coeff)
}

/// Answers `below` along one path of choices, recording the bound of each choice, so that
/// `leaves` can step to the next path.
struct Odometer<'p> {
  path: &'p mut Vec<(u32, u32)>,
  next: usize,
}

impl Source for Odometer<'_> {
  fn below(&mut self, n: u32) -> u32 {
    assert!((1..=256).contains(&n), "a choice among {n} values");
    let answer = match self.path.get_mut(self.next) {
      Some((answer, bound)) => {
        // The same answers lead to the same choices; a prefix records its bounds on the first
        // path.
        assert!(*bound == 0 || *bound == n);
        *bound = n;
        *answer
      }
      None => {
        self.path.push((0, n));
        0
      }
    };
    assert!(answer < n, "answer {answer} is not below {n}");
    self.next += 1;
    answer
  }
}

/// Calls `visit` with every path of choices, as `(answer, bound)` pairs, that starts with the
/// answers `prefix`, and with the coefficient that the path gives for `digits` digits, `fraction`
/// of them fraction digits.
fn leaves<R: Storage>(
  digits: u32,
  fraction: u32,
  prefix: &[u32],
  mut visit: impl FnMut(&[(u32, u32)], R),
) {
  let mut path: Vec<(u32, u32)> = prefix.iter().map(|&answer| (answer, 0)).collect();
  loop {
    let mut source = Odometer {
      path: &mut path,
      next: 0,
    };
    let coeff = generate(&mut source, digits, fraction);
    let drawn = source.next;
    assert_eq!(drawn, path.len(), "every recorded choice is drawn");
    visit(&path, coeff);
    // The last choice after the prefix that has another answer takes it, and the choices after
    // it are drawn again.
    loop {
      if path.len() <= prefix.len() {
        return;
      }
      let (answer, bound) = path.last_mut().expect("the path is longer than its prefix");
      if *answer + 1 < *bound {
        *answer += 1;
        break;
      }
      path.pop();
    }
  }
}

/// `coeff` as a big integer, read through the widest storage of its sign.
pub(crate) fn big<R: Storage>(coeff: R) -> BigInt {
  if R::SIGNED {
    bigint::from_limbs(&coeff.convert::<i512>(Token::new()).to_limbs(), true)
  } else {
    bigint::from_limbs(&coeff.convert::<u512>(Token::new()).to_limbs(), false)
  }
}

/// The number of decimal digits of `|value|`, one for zero.
fn digits_of(value: &BigInt) -> u32 {
  value.magnitude().to_string().len() as u32
}

/// A decimal drawn from `source` by the generator.
fn draw<I, F, R>(source: &mut impl Source) -> Decimal<I, F, R>
where
  R: Storage,
  (I, F): Layout<R>,
{
  Decimal::from_coeff(coefficient::<I, F, R>(source))
}

/// The magnitudes of each edge kind that applies to `digits` digits, `fraction` of them fraction
/// digits, in the order of the choice of kind: zero, one unit, one, the maximum, `10^k`,
/// `10^k - 1`, `5 * 10^k` and `15 * 10^k`, each with the values for every exponent.
fn edge_kinds(digits: u32, fraction: u32) -> Vec<Vec<BigInt>> {
  let pow10 = bigint::pow10;
  let mut kinds = vec![vec![BigInt::ZERO], vec![BigInt::from(1u8)]];
  if fraction < digits {
    kinds.push(vec![pow10(fraction)]);
  }
  kinds.push(vec![pow10(digits) - 1u8]);
  kinds.push((0..digits).map(pow10).collect());
  kinds.push((0..digits).map(|k| pow10(k) - 1u8).collect());
  kinds.push((0..digits).map(|k| pow10(k) * 5u8).collect());
  if digits >= 2 {
    kinds.push((0..digits - 1).map(|k| pow10(k) * 15u8).collect());
  }
  kinds
}

/// Checks the values drawn for `Decimal<I, F, R>`: each is in range and a valid coefficient,
/// every number of digits from one to `Integral + Fraction` occurs, negative values occur exactly
/// for a signed storage, and every kind of edge value occurs, with every exponent for up to nine
/// digits.
pub(crate) fn check_values<I, F, R>(
  name: &str,
  values: &[Decimal<I, F, R>],
  repr: fn(Decimal<I, F, R>) -> R,
  from_repr: fn(R) -> Option<Decimal<I, F, R>>,
) where
  R: Storage,
  (I, F): Layout<R>,
{
  let digits = <(I, F) as LayoutDigits<R>>::DIGITS;
  let fraction = <(I, F) as LayoutDigits<R>>::FRACTION;
  assert_eq!(values.len(), SAMPLES, "{name}");
  let mut lengths = vec![0usize; digits as usize + 1];
  let mut seen = BTreeSet::new();
  let (mut negative, mut positive) = (false, false);
  for &value in values {
    let coeff = repr(value);
    assert!(
      coeff.within_digits(digits, Token::new()),
      "{name}: {value:?} is out of range"
    );
    assert_eq!(from_repr(coeff), Some(value), "{name}");
    let coeff = big(coeff);
    negative |= coeff.sign() == Sign::Minus;
    positive |= coeff.sign() == Sign::Plus;
    lengths[digits_of(&coeff) as usize] += 1;
    seen.insert(coeff);
  }
  assert!(
    lengths[1..].iter().all(|&count| count > 0),
    "{name}: values by number of digits {lengths:?}"
  );
  assert!(positive, "{name}: no positive value");
  assert_eq!(negative, R::SIGNED, "{name}: negative values");
  let occurs = |magnitude: &BigInt| seen.contains(magnitude) || seen.contains(&-magnitude);
  for (kind, magnitudes) in edge_kinds(digits, fraction).iter().enumerate() {
    if digits <= 9 {
      for magnitude in magnitudes {
        assert!(occurs(magnitude), "{name}: edge kind {kind}, {magnitude}");
      }
    } else {
      assert!(magnitudes.iter().any(occurs), "{name}: edge kind {kind}");
    }
  }
}

/// The candidates for `value` from their definition: zero, then `|value|` if `value` is negative,
/// then `value` truncated toward zero to `1, 2, ..., d - 1` of its `d` digits.
#[cfg(any(feature = "proptest", feature = "quickcheck"))]
pub(crate) fn candidates(value: &BigInt) -> Vec<BigInt> {
  if value.sign() == Sign::NoSign {
    return Vec::new();
  }
  let mut candidates = vec![BigInt::ZERO];
  if value.sign() == Sign::Minus {
    candidates.push(-value);
  }
  let digits = digits_of(value);
  candidates.extend((1..digits).rev().map(|k| value / bigint::pow10(k)));
  candidates
}

/// `(nonzero, negative, digits)`, by which every candidate is simpler than its value.
#[cfg(any(feature = "proptest", feature = "quickcheck"))]
pub(crate) fn simplicity(value: &BigInt) -> (bool, bool, u32) {
  (
    value.sign() != Sign::NoSign,
    value.sign() == Sign::Minus,
    digits_of(value),
  )
}

/// The most choices among two or more values on any path for `digits` digits, which is the
/// number of bytes that `arbitrary` reads at most: the branch, the number of digits, the digits,
/// whether to zero trailing digits, how many, and the sign of a signed storage.
fn longest_path(digits: u32, signed: bool) -> usize {
  let several = usize::from(digits >= 2);
  1 + several + digits as usize + 1 + several + usize::from(signed)
}

/// Visits every path through the edge branch. Each gives a value of the kind it chose, the kinds
/// that apply are equally likely, and so are the magnitudes of a kind; every edge value, with
/// either sign for a signed storage, is reached. Returns the most choices among two or more
/// values on a path.
fn check_edges<R: Storage>(digits: u32, fraction: u32) -> usize {
  let kinds = edge_kinds(digits, fraction);
  // A multiple of the product of the bounds on every path after the branch: the kind, an
  // exponent below `digits` or `digits - 1`, and a sign.
  let scale = kinds.len() as u128 * u128::from(digits) * u128::from(digits.max(2) - 1) * 2;
  let mut reached = vec![BTreeSet::new(); kinds.len()];
  let mut mass: Vec<BTreeMap<BigInt, u128>> = vec![BTreeMap::new(); kinds.len()];
  let mut longest = 0;
  leaves::<R>(digits, fraction, &[0], |path, coeff| {
    let (kind, count) = path[1];
    assert_eq!(count as usize, kinds.len(), "the kinds that apply");
    let value = big(coeff);
    let magnitude = BigInt::from(value.magnitude().clone());
    assert!(
      kinds[kind as usize].contains(&magnitude),
      "kind {kind} gave {value}"
    );
    let product: u128 = path[1..]
      .iter()
      .map(|&(_, bound)| u128::from(bound))
      .product();
    assert_eq!(scale % product, 0);
    *mass[kind as usize].entry(magnitude).or_default() += scale / product;
    reached[kind as usize].insert(value);
    longest = longest.max(path.iter().filter(|&&(_, bound)| bound >= 2).count());
  });
  for (kind, magnitudes) in kinds.iter().enumerate() {
    let mut expected = BTreeSet::new();
    for magnitude in magnitudes {
      if R::SIGNED && magnitude.sign() != Sign::NoSign {
        expected.insert(-magnitude);
      }
      expected.insert(magnitude.clone());
    }
    assert_eq!(reached[kind], expected, "kind {kind}");
    let per_magnitude = scale / kinds.len() as u128 / mass[kind].len() as u128;
    assert!(
      mass[kind].values().all(|&each| each == per_magnitude),
      "kind {kind} is chosen with probability 1/{}, its magnitudes uniformly: {:?}",
      kinds.len(),
      mass[kind]
    );
  }
  longest
}

fn gcd(a: u128, b: u128) -> u128 {
  if b == 0 { a } else { gcd(b, a % b) }
}

/// Visits every path through the digit-wise branch, for up to four digits, and checks the
/// probability of each value given the branch against the design: the number of digits `d` is
/// uniform in `1..=digits`, the magnitude uniform among the numbers of `d` digits, then with
/// probability 1/4 its last `t` digits are zeroed for `t` uniform in `0..d`, and a nonzero value
/// takes either sign for a signed storage. Every value in range is reached. Returns the most
/// choices among two or more values on a path.
fn check_digit_wise<R: Storage>(digits: u32, fraction: u32) -> usize {
  // A multiple of the product of the bounds on every path after the branch: the number of digits,
  // the first digit among 9 or 10, the other digits, whether to zero trailing digits, how many,
  // and the sign.
  let lcm = (1..=u128::from(digits)).fold(1, |lcm, d| lcm / gcd(lcm, d) * d);
  let scale = u128::from(digits) * 90 * 10u128.pow(digits - 1) * 4 * lcm * 2;
  let mut mass: BTreeMap<BigInt, u128> = BTreeMap::new();
  let mut longest = 0;
  leaves::<R>(digits, fraction, &[1], |path, coeff| {
    let product: u128 = path[1..]
      .iter()
      .map(|&(_, bound)| u128::from(bound))
      .product();
    assert_eq!(scale % product, 0);
    *mass.entry(big(coeff)).or_default() += scale / product;
    longest = longest.max(path.iter().filter(|&&(_, bound)| bound >= 2).count());
  });
  assert_eq!(mass.values().sum::<u128>(), scale);
  let max = 10i128.pow(digits) - 1;
  let min = if R::SIGNED { -max } else { 0 };
  assert_eq!(mass.len() as i128, max - min + 1, "every value in range");
  for (value, &got) in &mass {
    let magnitude = u128::try_from(value.magnitude()).expect("at most four digits");
    let d = digits_of(value);
    let numbers = if d == 1 { 10 } else { 9 * 10u128.pow(d - 1) };
    // P = (1/digits) (1/numbers) (3/4 + 1/(4d) * the sum of 10^t over `t < d` with 10^t | m): an
    // untouched number, or one of the 10^t numbers whose last `t` digits were zeroed to `m`.
    let zeroed: u128 = (0..d)
      .map(|t| 10u128.pow(t))
      .filter(|&power| magnitude % power == 0)
      .sum();
    let numerator = 3 * u128::from(d) + zeroed;
    let mut denominator = u128::from(digits) * numbers * 4 * u128::from(d);
    if R::SIGNED && magnitude != 0 {
      denominator *= 2;
    }
    assert_eq!(
      got * denominator,
      numerator * scale,
      "the probability of {value} with {digits} digits"
    );
  }
  longest
}

#[test]
fn every_edge_path_gives_an_edge_value_of_its_kind() {
  let edges = [
    check_edges::<i8>(1, 0),
    check_edges::<i8>(1, 1),
    check_edges::<i8>(2, 0),
    check_edges::<i8>(2, 1),
    check_edges::<i8>(2, 2),
    check_edges::<u8>(1, 0),
    check_edges::<u8>(2, 2),
    check_edges::<i64>(18, 8),
    check_edges::<u64>(19, 9),
    check_edges::<u64>(19, 19),
    check_edges::<i128>(38, 0),
    check_edges::<u128>(38, 38),
    check_edges::<i256>(76, 38),
    check_edges::<u256>(77, 18),
    check_edges::<i512>(153, 153),
    check_edges::<u512>(154, 0),
  ];
  // The branch, the kind, an exponent and a sign at most: never the longest path.
  assert!(edges.iter().all(|&longest| longest <= 4));
}

#[test]
fn digit_wise_paths_have_the_designed_distribution() {
  for (digits, fraction) in [(1, 0), (1, 1), (2, 1), (2, 2), (3, 0), (4, 2)] {
    assert_eq!(
      check_digit_wise::<i16>(digits, fraction),
      longest_path(digits, true)
    );
    assert_eq!(
      check_digit_wise::<u16>(digits, fraction),
      longest_path(digits, false)
    );
  }
}

/// The longest paths of the widest types: all the digits, nines, then whether to zero trailing
/// digits, none of them, and a sign for a signed storage.
#[test]
fn widest_longest_paths() {
  let nines = |digits: u32, signed: bool| {
    let mut answers = vec![1, digits - 1, 8];
    answers.extend(core::iter::repeat_n(9, digits as usize - 1));
    answers.extend([0, 0]);
    if signed {
      answers.push(1);
    }
    assert_eq!(answers.len(), longest_path(digits, signed));
    answers
  };
  let max = |digits| bigint::pow10(digits) - 1u8;
  assert_eq!(scripted::<u512>(154, 0, &nines(154, false)), max(154));
  assert_eq!(scripted::<i512>(153, 9, &nines(153, true)), -max(153));
  assert_eq!(scripted::<u256>(77, 77, &nines(77, false)), max(77));
  assert_eq!(scripted::<i256>(76, 0, &nines(76, true)), -max(76));
}

/// Trailing zeros keep the first digit, and with it the number of digits.
#[test]
fn trailing_zeros_keep_the_number_of_digits() {
  let mut answers = vec![1, 153, 0];
  answers.extend(core::iter::repeat_n(9, 153));
  answers.extend([0, 153]);
  assert_eq!(scripted::<u512>(154, 0, &answers), bigint::pow10(153));

  let mut answers = vec![1, 17, 4];
  answers.extend(core::iter::repeat_n(3, 17));
  answers.extend([0, 5, 1]);
  assert_eq!(
    scripted::<i64>(18, 8, &answers),
    BigInt::from(-533_333_333_333_300_000i64)
  );

  // Any answer but 0 leaves the digits as drawn.
  let mut answers = vec![1, 17, 4];
  answers.extend(core::iter::repeat_n(3, 17));
  answers.extend([3, 0]);
  assert_eq!(
    scripted::<i64>(18, 8, &answers),
    BigInt::from(533_333_333_333_333_333i64)
  );
}

macro_rules! check_draws {
  ($t:ty) => {{
    let mut source = SplitMix(0x5eed_0001);
    let values: Vec<$t> = (0..SAMPLES).map(|_| draw(&mut source)).collect();
    check_values(stringify!($t), &values, <$t>::repr, <$t>::from_repr);
  }};
}

#[test]
fn drawn_values_cover_digits_signs_and_edges() {
  for_each_type!(check_draws);
}

#[cfg(any(feature = "proptest", feature = "quickcheck"))]
fn check_chain<I, F, R>(coeff: R, repr: fn(Decimal<I, F, R>) -> R)
where
  R: Storage,
{
  let value = big(coeff);
  let chain = Chain::<I, F, R>::new(coeff);
  let indexed: Vec<BigInt> = (0..).map_while(|index| chain.get(index)).map(big).collect();
  assert_eq!(indexed, candidates(&value), "the candidates for {value}");
  let iterated: Vec<BigInt> = chain.map(|candidate| big(repr(candidate))).collect();
  assert_eq!(iterated, indexed);
  assert!(indexed.len() <= digits_of(&value) as usize + 1);
  for candidate in &indexed {
    assert!(
      simplicity(candidate) < simplicity(&value),
      "{candidate} is not simpler than {value}"
    );
  }
  assert_eq!(
    indexed.iter().collect::<BTreeSet<_>>().len(),
    indexed.len(),
    "the candidates for {value} are distinct"
  );
}

#[cfg(any(feature = "proptest", feature = "quickcheck"))]
macro_rules! check_chains {
  ($t:ty) => {{
    let mut source = SplitMix(0x5eed_0002);
    for _ in 0..SAMPLES {
      let value: $t = draw(&mut source);
      check_chain(value.repr(), <$t>::repr);
    }
    check_chain(<$t>::ZERO.repr(), <$t>::repr);
    check_chain(<$t>::MAX.repr(), <$t>::repr);
    check_chain(<$t>::MIN.repr(), <$t>::repr);
  }};
}

#[cfg(any(feature = "proptest", feature = "quickcheck"))]
#[test]
fn chains_hold_simpler_candidates_in_order() {
  for_each_type!(check_chains);
}

#[cfg(any(feature = "proptest", feature = "quickcheck"))]
#[test]
fn chain_examples() {
  use typenum::{U0, U18};

  let chain = |coeff: i64| -> Vec<i64> {
    let chain = Chain::<U18, U0, i64>::new(coeff);
    (0..).map_while(|index| chain.get(index)).collect()
  };
  assert_eq!(chain(-123), [0, 123, -1, -12]);
  assert_eq!(
    chain(123_456_789),
    [0, 1, 12, 123, 1_234, 12_345, 123_456, 1_234_567, 12_345_678]
  );
  assert_eq!(chain(-90), [0, 90, -9]);
  assert_eq!(chain(7), [0]);
  assert_eq!(chain(-7), [0, 7]);
  assert!(chain(0).is_empty());
}

/// `MODES` lists every mode once, `Down` first; a new variant fails to compile here.
#[test]
fn modes_list_every_mode_down_first() {
  let position = |mode| match mode {
    RoundingMode::Down => 0,
    RoundingMode::Up => 1,
    RoundingMode::Ceiling => 2,
    RoundingMode::Floor => 3,
    RoundingMode::HalfUp => 4,
    RoundingMode::HalfDown => 5,
    RoundingMode::HalfEven => 6,
  };
  for (index, &mode) in MODES.iter().enumerate() {
    assert_eq!(position(mode), index, "{mode:?}");
  }
  assert_eq!(MODES[0], RoundingMode::Down);
}

#[cfg(any(feature = "arbitrary", feature = "quickcheck"))]
#[test]
fn a_mode_is_one_choice_among_seven() {
  for (index, &mode) in MODES.iter().enumerate() {
    let mut script = Script {
      answers: &[index as u32],
      next: 0,
    };
    assert_eq!(super::mode(&mut script), mode);
    assert_eq!(script.next, 1);
  }
}
