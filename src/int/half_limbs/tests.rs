//! The tests of `half_limbs!`.
//!
//! `half_limbs_tests!` writes the tests that every instance shares, which `unsigned::b128` runs for
//! `u128`. Here `half_limbs!` is also instantiated for `u8` to `u64`, and native integers of twice
//! the width check the results: on every input for `u8`, and on inputs weighted to the boundaries
//! for the wider types. The instances differ only in the limb type, so these tests check the text
//! that `unsigned::b128` runs.

/// Defines, in a module that invokes `half_limbs!($limb)`, the tests that every instance shares.
macro_rules! half_limbs_tests {
  ($limb:ident) => {
    /// The limb with every bit set.
    const MAX: $limb = $limb::MAX;

    /// The limb with only its top bit set.
    const TOP: $limb = 1 << ($limb::BITS - 1);

    /// `2^(B - 1) + 2^H - 1`: a divisor with its top bit set and a low digit of all ones.
    const V: $limb = TOP + DIGIT_MAX;

    #[test]
    fn the_largest_multiply_add_fills_two_limbs() {
      // `(2^B - 1)^2 + 2 * (2^B - 1) = 2^(2B) - 1`.
      assert_eq!(carrying_mul_add(MAX, MAX, MAX, MAX), (MAX, MAX));
    }

    /// Divisions through each path of the two steps: an estimate that starts at `2^H + 1` or at
    /// `2^H`, and corrections that stop at the test with the low digit of the divisor or once the
    /// remainder exceeds a digit.
    #[test]
    fn division_takes_every_path() {
      // Both steps start at `2^H + 1`, correct twice and stop on the remainder.
      assert_eq!(div_rem_wide(V - 1, MAX, V), (MAX, V - 1));
      // The first step starts at `2^H` and corrects once.
      let half_digit: $limb = 1 << (HALF - 1);
      assert_eq!(
        div_rem_wide(TOP + half_digit - 1, 0, V),
        (DIGIT_MAX << HALF | 1, (half_digit - 2) << HALF | 1)
      );
      // The first step starts at `2^H + 1`, corrects twice and stops on the remainder, and the
      // second step corrects once.
      assert_eq!(div_rem_wide(V - 1, 0, V), (MAX - 1, (1 << (HALF + 1)) - 2));
      // Both steps start at `2^H`, correct once and stop on the remainder.
      assert_eq!(div_rem_wide(MAX - 1, MAX, MAX), (MAX, MAX - 1));
    }

    /// The largest dividend whose quotient fits a limb, `divisor * 2^B - 1`, for divisors with
    /// every number of leading zeros.
    #[test]
    fn division_for_every_normalization_shift() {
      for shift in 0..$limb::BITS {
        let top = TOP >> shift;
        for divisor in [MAX >> shift, top, top + 1] {
          assert_eq!(
            div_rem_wide(divisor - 1, MAX, divisor),
            (MAX, divisor - 1),
            "{divisor:#x}"
          );
        }
      }
    }

    /// A zero divisor fails as native division does, with a high limb of zero.
    #[test]
    #[should_panic(expected = "attempt to divide by zero")]
    fn division_by_zero_panics() {
      let _ = div_rem_wide(0, 5, 0);
    }

    /// A zero divisor fails as native division does, also with a nonzero high limb.
    #[test]
    #[should_panic(expected = "attempt to divide by zero")]
    fn division_of_a_high_limb_by_zero_panics() {
      let _ = div_rem_wide(1, 0, 0);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "the quotient overflows a limb")]
    fn a_quotient_wider_than_a_limb_panics() {
      let _ = div_rem_wide(3, 0, 3);
    }
  };
}

pub(crate) use half_limbs_tests;

/// `half_limbs!` for `u8`, with digits of 4 bits, against `u16` on every input.
mod u8_limbs {
  half_limbs!(u8);

  half_limbs_tests!(u8);

  /// The step of the sweeps below: every input, or under Miri, which runs them slowly, every
  /// 331st. Each sweep also visits the inputs whose limbs are all of `ENDS`, which a stride may
  /// skip.
  const STEP: usize = if cfg!(miri) { 331 } else { 1 };

  /// The limits of a limb and the values next to them.
  const ENDS: [u8; 4] = [0, 1, u8::MAX - 1, u8::MAX];

  /// Every `(a, b, carry)` with `add` zero or all ones. Every intermediate grows with the digits
  /// of `carry` and `add`, which are largest at all ones, so these include the largest
  /// intermediates of each `(a, b)`.
  #[test]
  fn every_multiply_add_matches_u16() {
    let ends = ENDS.iter().flat_map(|&a| {
      ENDS.iter().flat_map(move |&b| {
        ENDS
          .iter()
          .flat_map(move |&carry| [0, 1].map(|top| u32::from_le_bytes([a, b, carry, top])))
      })
    });
    for index in (0..1 << 25).step_by(STEP).chain(ends) {
      let [a, b, carry, top] = u32::to_le_bytes(index);
      let add = if top == 0 { 0 } else { u8::MAX };
      let wide = u16::from(a) * u16::from(b) + u16::from(carry) + u16::from(add);
      assert_eq!(
        carrying_mul_add(a, b, carry, add),
        (wide as u8, (wide >> 8) as u8),
        "{a} * {b} + {carry} + {add}"
      );
    }
  }

  /// Every `(high, low, divisor)` with `high < divisor`.
  #[test]
  fn every_division_matches_u16() {
    let ends = ENDS.iter().flat_map(|&high| {
      ENDS.iter().flat_map(move |&low| {
        ENDS
          .iter()
          .map(move |&divisor| u32::from_le_bytes([high, low, divisor, 0]))
      })
    });
    for index in (0..1 << 24).step_by(STEP).chain(ends) {
      let [high, low, divisor, _] = u32::to_le_bytes(index);
      if high < divisor {
        let wide = u16::from(high) << 8 | u16::from(low);
        let wide_divisor = u16::from(divisor);
        assert_eq!(
          div_rem_wide(high, low, divisor),
          ((wide / wide_divisor) as u8, (wide % wide_divisor) as u8),
          "{wide} / {divisor}"
        );
      }
    }
  }
}

/// Defines the module `$module` with `half_limbs!($limb)`, the shared tests, and tests against
/// `$wide`, the native integer of twice the width, on random inputs weighted to the boundaries.
macro_rules! sampled_tests {
  ($module:ident: $limb:ident, $wide:ident) => {
    /// `half_limbs!` for one limb type, against the native integer of twice its width on random
    /// inputs.
    mod $module {
      half_limbs!($limb);

      half_limbs_tests!($limb);

      /// The number of random inputs of each test: 65536, or under Miri, which runs them slowly,
      /// 64.
      const COUNT: usize = if cfg!(miri) { 64 } else { 65_536 };

      /// A deterministic xorshift generator, so that a failure replays from the seed.
      struct Rng(u64);

      impl Rng {
        fn next(&mut self) -> u64 {
          self.0 ^= self.0 >> 12;
          self.0 ^= self.0 << 25;
          self.0 ^= self.0 >> 27;
          self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
        }

        /// Zero, all ones, the largest digit, the smallest limb above it, a random limb shifted
        /// right by a random amount, or a random limb.
        fn limb(&mut self) -> $limb {
          let random = self.next() as $limb;
          match self.next() % 8 {
            0 => 0,
            1 => $limb::MAX,
            2 => DIGIT_MAX,
            3 => DIGIT_MAX + 1,
            4 | 5 => random >> (self.next() % u64::from($limb::BITS)),
            _ => random,
          }
        }
      }

      #[test]
      fn multiply_add_matches_the_wide_integer() {
        let mut rng = Rng(0x6a09_e667_f3bc_c908 ^ u64::from($limb::BITS));
        for _ in 0..COUNT {
          let (a, b, carry, add) = (rng.limb(), rng.limb(), rng.limb(), rng.limb());
          let wide = $wide::from(a) * $wide::from(b) + $wide::from(carry) + $wide::from(add);
          assert_eq!(
            carrying_mul_add(a, b, carry, add),
            (wide as $limb, (wide >> $limb::BITS) as $limb),
            "{a:#x} * {b:#x} + {carry:#x} + {add:#x}"
          );
        }
      }

      #[test]
      fn division_matches_the_wide_integer() {
        let mut rng = Rng(0xbb67_ae85_84ca_a73b ^ u64::from($limb::BITS));
        for _ in 0..COUNT {
          let divisor = rng.limb().max(1);
          // A high limb below the divisor: one less, one with the same top digit, or random.
          let high = match rng.next() % 3 {
            0 => divisor - 1,
            1 => match divisor & DIGIT_MAX {
              0 => divisor - 1,
              low_digit => divisor - low_digit + rng.limb() % low_digit,
            },
            _ => rng.limb() % divisor,
          };
          let low = rng.limb();
          let wide = $wide::from(high) << $limb::BITS | $wide::from(low);
          let wide_divisor = $wide::from(divisor);
          assert_eq!(
            div_rem_wide(high, low, divisor),
            (
              (wide / wide_divisor) as $limb,
              (wide % wide_divisor) as $limb
            ),
            "{wide:#x} / {divisor:#x}"
          );
        }
      }
    }
  };
}

sampled_tests!(u16_limbs: u16, u32);
sampled_tests!(u32_limbs: u32, u64);
sampled_tests!(u64_limbs: u64, u128);
