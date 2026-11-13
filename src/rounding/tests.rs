use core::cmp::Ordering;

use super::{RoundingMode, increments};

const MODES: [RoundingMode; 8] = [
  RoundingMode::Up,
  RoundingMode::Down,
  RoundingMode::Ceiling,
  RoundingMode::Floor,
  RoundingMode::HalfUp,
  RoundingMode::HalfDown,
  RoundingMode::HalfEven,
  RoundingMode::Up05,
];

/// Rounds `quarters / 4` to an integer with `mode`, choosing between the integers below and above
/// it by their distances, independently of `increments`. `Up05` has no distance: it reads the last
/// digit of the integer toward zero.
fn round_quarters(quarters: i64, mode: RoundingMode) -> i64 {
  let floor = quarters.div_euclid(4);
  if quarters.rem_euclid(4) == 0 {
    return floor;
  }
  let ceiling = floor + 1;
  let (toward_zero, away_from_zero) = if quarters < 0 {
    (ceiling, floor)
  } else {
    (floor, ceiling)
  };
  let below = quarters - 4 * floor;
  let above = 4 * ceiling - quarters;
  match mode {
    RoundingMode::Up => away_from_zero,
    RoundingMode::Down => toward_zero,
    RoundingMode::Ceiling => ceiling,
    RoundingMode::Floor => floor,
    RoundingMode::HalfUp | RoundingMode::HalfDown | RoundingMode::HalfEven => {
      match below.cmp(&above) {
        Ordering::Less => floor,
        Ordering::Greater => ceiling,
        Ordering::Equal => match mode {
          RoundingMode::HalfUp => away_from_zero,
          RoundingMode::HalfDown => toward_zero,
          _ if floor % 2 == 0 => floor,
          _ => ceiling,
        },
      }
    }
    RoundingMode::Up05 if toward_zero % 5 == 0 => away_from_zero,
    RoundingMode::Up05 => toward_zero,
  }
}

/// Every combination of inputs, against values a quarter, a half and three quarters past each
/// quotient from 0 to 9, which takes every parity and every last digit, of either sign.
#[test]
fn increments_matches_rounding_by_distance() {
  for mode in MODES {
    for negative in [false, true] {
      for quotient in 0i64..10 {
        for (quarters, half) in [
          (1, Ordering::Less),
          (2, Ordering::Equal),
          (3, Ordering::Greater),
        ] {
          let magnitude = 4 * quotient + quarters;
          let value = if negative { -magnitude } else { magnitude };
          let away = round_quarters(value, mode).abs() > quotient;
          assert_eq!(
            increments(
              mode,
              negative,
              false,
              half,
              quotient % 2 == 1,
              quotient % 5 == 0
            ),
            away,
            "{mode:?}, value {value}/4"
          );
        }
        // An exact quotient never moves, whatever `half` and the last digit say.
        for half in [Ordering::Less, Ordering::Equal, Ordering::Greater] {
          for odd in [false, true] {
            for ends_in_0_or_5 in [false, true] {
              assert!(
                !increments(mode, negative, true, half, odd, ends_in_0_or_5),
                "{mode:?}"
              );
            }
          }
        }
      }
    }
  }
}

/// The decision of each mode, written out case by case.
#[test]
fn increments_per_mode() {
  use Ordering::{Equal, Greater, Less};
  use RoundingMode::{Ceiling, Down, Floor, HalfDown, HalfEven, HalfUp, Up};

  // No mode but `Up05` reads the last digit.
  for ends in [false, true] {
    for negative in [false, true] {
      for half in [Less, Equal, Greater] {
        for odd in [false, true] {
          assert!(increments(Up, negative, false, half, odd, ends));
          assert!(!increments(Down, negative, false, half, odd, ends));
          assert_eq!(
            increments(Ceiling, negative, false, half, odd, ends),
            !negative
          );
          assert_eq!(
            increments(Floor, negative, false, half, odd, ends),
            negative
          );
        }
        // The half modes ignore the sign; only a tie looks further.
        assert!(!increments(HalfUp, negative, false, Less, false, ends));
        assert!(increments(HalfUp, negative, false, Equal, false, ends));
        assert!(increments(HalfUp, negative, false, Greater, false, ends));
        assert!(!increments(HalfDown, negative, false, Less, true, ends));
        assert!(!increments(HalfDown, negative, false, Equal, true, ends));
        assert!(increments(HalfDown, negative, false, Greater, true, ends));
        assert!(!increments(HalfEven, negative, false, Less, true, ends));
        assert!(!increments(HalfEven, negative, false, Equal, false, ends));
        assert!(increments(HalfEven, negative, false, Equal, true, ends));
        assert!(increments(HalfEven, negative, false, Greater, false, ends));
      }
    }
  }
}

#[test]
fn modes_are_plain_values() {
  fn assert_traits<T: Copy + Eq + core::hash::Hash + core::fmt::Debug + Send + Sync + 'static>() {}
  assert_traits::<RoundingMode>();
  assert_eq!(size_of::<RoundingMode>(), 1);
  assert_eq!(std::format!("{:?}", RoundingMode::HalfEven), "HalfEven");
}
