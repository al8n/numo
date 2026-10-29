use typenum::{U0, U7, U8};

use crate::{CheckedOps, Int, OverflowingOps, SaturatingOps, StrictOps, Uint, WrappingOps};

/// The smallest and the largest value of `bits` bits, unsigned or in two's complement.
fn range(bits: u32, signed: bool) -> (i128, i128) {
  match (bits, signed) {
    (0, _) => (0, 0),
    (_, false) => (0, (1 << bits) - 1),
    (_, true) => (-(1 << (bits - 1)), (1 << (bits - 1)) - 1),
  }
}

/// `exact` wrapped to the range of `bits` bits, and whether it is outside that range.
fn wrap(exact: i128, bits: u32, signed: bool) -> (i128, bool) {
  let (min, max) = range(bits, signed);
  let wrapped = (exact - min).rem_euclid(1 << bits) + min;
  (wrapped, exact < min || max < exact)
}

/// The result of a method through the inherent `const fn` of the storage and through the trait,
/// which must agree.
macro_rules! both {
  ($trait:ident::$method:ident($receiver:expr $(, $arg:expr)*)) => {{
    let inherent = $receiver.$method($($arg),*);
    let generic = $trait::$method($receiver $(, $arg)*);
    assert_eq!(inherent, generic, "{} through the trait", stringify!($method));
    inherent
  }};
}

/// Checks the values of `$ty`, of `$bits` bits, created by `$from` from their value: the
/// overflowing, checked, wrapping, saturating and strict methods, through the inherent `const fn`s
/// and the traits, against the exact results wrapped to the width. Every value and every `$step`th
/// pair, or under Miri every 7th value and every 61st pair.
macro_rules! check_every_pair {
  ($ty:ty, $bits:expr, $signed:expr, $step:expr, $from:expr) => {{
    let from: fn(i128) -> $ty = $from;
    let (bits, signed) = ($bits, $signed);
    let (min, max) = range(bits, signed);
    let (value_step, pair_step) = if cfg!(miri) { (7, 61) } else { (1, $step) };
    for a in (min..=max).step_by(value_step) {
      let x = from(a);
      let (negated, overflow) = wrap(-a, bits, signed);
      assert_eq!(
        both!(OverflowingOps::overflowing_neg(&x)),
        (from(negated), overflow),
        "-{a} in {bits} bits"
      );
      assert_eq!(
        both!(CheckedOps::checked_neg(&x)),
        (!overflow).then(|| from(negated)),
        "-{a} in {bits} bits"
      );
      assert_eq!(
        both!(WrappingOps::wrapping_neg(&x)),
        from(negated),
        "-{a} in {bits} bits"
      );
      if !overflow {
        assert_eq!(
          both!(StrictOps::strict_neg(&x)),
          from(negated),
          "-{a} in {bits} bits"
        );
      }
    }
    let count = max - min + 1;
    for pair in (0..count * count).step_by(pair_step) {
      let (a, b) = (min + pair / count, min + pair % count);
      let (x, y) = (from(a), from(b));
      for (exact, (value, overflow), checked, wrapping, saturating) in [
        (
          a + b,
          both!(OverflowingOps::overflowing_add(&x, &y)),
          both!(CheckedOps::checked_add(&x, &y)),
          both!(WrappingOps::wrapping_add(&x, &y)),
          both!(SaturatingOps::saturating_add(&x, &y)),
        ),
        (
          a - b,
          both!(OverflowingOps::overflowing_sub(&x, &y)),
          both!(CheckedOps::checked_sub(&x, &y)),
          both!(WrappingOps::wrapping_sub(&x, &y)),
          both!(SaturatingOps::saturating_sub(&x, &y)),
        ),
        (
          a * b,
          both!(OverflowingOps::overflowing_mul(&x, &y)),
          both!(CheckedOps::checked_mul(&x, &y)),
          both!(WrappingOps::wrapping_mul(&x, &y)),
          both!(SaturatingOps::saturating_mul(&x, &y)),
        ),
      ] {
        let (wrapped, overflowed) = wrap(exact, bits, signed);
        assert_eq!(
          (value, overflow),
          (from(wrapped), overflowed),
          "{a} and {b} in {bits} bits"
        );
        assert_eq!(
          checked,
          (!overflowed).then(|| from(wrapped)),
          "{a} and {b} in {bits} bits"
        );
        assert_eq!(wrapping, from(wrapped), "{a} and {b} in {bits} bits");
        assert_eq!(
          saturating,
          from(exact.clamp(min, max)),
          "{a} and {b} in {bits} bits"
        );
      }
      if !wrap(a + b, bits, signed).1 {
        assert_eq!(
          both!(StrictOps::strict_add(&x, &y)),
          from(a + b),
          "{a} + {b}"
        );
      }
      if !wrap(a - b, bits, signed).1 {
        assert_eq!(
          both!(StrictOps::strict_sub(&x, &y)),
          from(a - b),
          "{a} - {b}"
        );
      }
      if !wrap(a * b, bits, signed).1 {
        assert_eq!(
          both!(StrictOps::strict_mul(&x, &y)),
          from(a * b),
          "{a} * {b}"
        );
      }
    }
  }};
}

/// Every pair of 7 and 8 bits in a native integer and of 7 bits in an array of one byte, and the
/// only value of zero bits in a native integer and in arrays of no limbs and of one limb, through
/// the inherent `const fn`s and the traits.
#[test]
fn families_match_the_exact_results() {
  check_every_pair!(Uint<U7, u8>, 7, false, 1, |v| Uint::<U7, u8>::new(v as u8));
  check_every_pair!(Uint<U8, u8>, 8, false, 1, |v| Uint::<U8, u8>::new(v as u8));
  check_every_pair!(Int<U7, i8>, 7, true, 1, |v| Int::<U7, i8>::new(v as i8));
  check_every_pair!(Int<U8, i8>, 8, true, 1, |v| Int::<U8, i8>::new(v as i8));
  check_every_pair!(Uint<U7, [u8; 1]>, 7, false, 1, |v| {
    Uint::<U7, [u8; 1]>::checked_from_limbs([v as u8]).expect("7 bits")
  });
  check_every_pair!(Int<U7, [u8; 1]>, 7, true, 1, |v| {
    Int::<U7, [u8; 1]>::checked_from_limbs([v as u8]).expect("7 bits")
  });
  check_every_pair!(Uint<U0, u8>, 0, false, 1, |_| Uint::<U0, u8>::ZERO);
  check_every_pair!(Int<U0, i8>, 0, true, 1, |_| Int::<U0, i8>::ZERO);
  check_every_pair!(Uint<U0, [u64; 0]>, 0, false, 1, |_| Uint::<U0, [u64; 0]>::ZERO);
  check_every_pair!(Int<U0, [u64; 0]>, 0, true, 1, |_| Int::<U0, [u64; 0]>::ZERO);
  check_every_pair!(Uint<U0, [u8; 1]>, 0, false, 1, |_| Uint::<U0, [u8; 1]>::ZERO);
  check_every_pair!(Int<U0, [u8; 1]>, 0, true, 1, |_| Int::<U0, [u8; 1]>::ZERO);
}
