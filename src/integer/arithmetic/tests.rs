use std::string::ToString;

use typenum::{IsGreater, NonZero, True, U0, U1, U2, U7, U8, U64, U65, U128, U129, U200, U1024};

use crate::{
  BitWidth, CheckedOps, FixedInteger, HasOne, Int, OverflowingOps, SaturatingOps,
  SignedFixedInteger, StrictOps, Uint, UnsignedFixedInteger, WrappingOps,
};

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

/// Checks the values of `$ty`, unsigned, of `$bits` bits, created by `$from` from their value:
/// `is_one`, and the carrying addition and the borrowing subtraction with and without a carry,
/// through the inherent `const fn`s and the traits, against the exact results wrapped to the
/// width. Every value and every `$step`th pair, or under Miri every 7th value and every 61st pair.
macro_rules! check_unsigned_methods {
  ($ty:ty, $bits:expr, $step:expr, $from:expr) => {{
    let from: fn(i128) -> $ty = $from;
    let bits = $bits;
    let (min, max) = range(bits, false);
    let (value_step, pair_step) = if cfg!(miri) { (7, 61) } else { (1, $step) };
    for a in (min..=max).step_by(value_step) {
      assert_eq!(both!(FixedInteger::is_one(&from(a))), a == 1, "{a}");
    }
    let count = max - min + 1;
    for pair in (0..count * count).step_by(pair_step) {
      let (a, b) = (min + pair / count, min + pair % count);
      let (x, y) = (from(a), from(b));
      for carry in [false, true] {
        let c = i128::from(carry);
        let (sum, overflow) = wrap(a + b + c, bits, false);
        assert_eq!(
          both!(UnsignedFixedInteger::carrying_add(&x, &y, carry)),
          (from(sum), overflow),
          "{a} + {b} + {c} in {bits} bits"
        );
        let (difference, overflow) = wrap(a - b - c, bits, false);
        assert_eq!(
          both!(UnsignedFixedInteger::borrowing_sub(&x, &y, carry)),
          (from(difference), overflow),
          "{a} - {b} - {c} in {bits} bits"
        );
      }
    }
  }};
}

/// Checks every value of `$ty`, signed, of `$bits` bits, created by `$from` from its value, or
/// under Miri every 7th value: `is_one`, the absolute value in every form, the saturating
/// negation, `signum` and `unsigned_abs`, whose result `$unsigned` creates, through the inherent
/// `const fn`s and the traits, against the exact results wrapped to the width.
macro_rules! check_signed_methods {
  ($ty:ty, $bits:expr, $from:expr, $unsigned:expr) => {{
    let from: fn(i128) -> $ty = $from;
    let bits = $bits;
    let (min, max) = range(bits, true);
    let step = if cfg!(miri) { 7 } else { 1 };
    for a in (min..=max).step_by(step) {
      let x = from(a);
      assert_eq!(both!(FixedInteger::is_one(&x)), a == 1, "{a}");
      let (abs, overflow) = wrap(a.abs(), bits, true);
      assert_eq!(
        both!(SignedFixedInteger::overflowing_abs(&x)),
        (from(abs), overflow),
        "|{a}| in {bits} bits"
      );
      assert_eq!(
        both!(SignedFixedInteger::checked_abs(&x)),
        (!overflow).then(|| from(abs)),
        "|{a}| in {bits} bits"
      );
      assert_eq!(
        both!(SignedFixedInteger::wrapping_abs(&x)),
        from(abs),
        "|{a}| in {bits} bits"
      );
      assert_eq!(
        both!(SignedFixedInteger::saturating_abs(&x)),
        from(a.abs().min(max)),
        "|{a}| in {bits} bits"
      );
      if !overflow {
        assert_eq!(
          both!(SignedFixedInteger::strict_abs(&x)),
          from(abs),
          "|{a}|"
        );
        assert_eq!(both!(SignedFixedInteger::abs(&x)), from(abs), "|{a}|");
      }
      assert_eq!(
        both!(SignedFixedInteger::saturating_neg(&x)),
        from((-a).clamp(min, max)),
        "-{a} in {bits} bits"
      );
      assert_eq!(
        both!(SignedFixedInteger::signum(&x)),
        from(a.signum()),
        "{a}"
      );
      assert_eq!(
        both!(SignedFixedInteger::unsigned_abs(&x)),
        $unsigned(a.abs()),
        "{a}"
      );
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

/// The methods of `Uint` alone and of `Int` alone, for every value of 7 and 8 bits in a native
/// integer and in an array of one byte, of one bit, which `Int` holds only -1 and 0 in, and of zero
/// bits, through the inherent `const fn`s and the traits.
#[test]
fn sign_methods_match_the_exact_results() {
  check_unsigned_methods!(Uint<U7, u8>, 7, 1, |v| Uint::<U7, u8>::new(v as u8));
  check_unsigned_methods!(Uint<U8, u8>, 8, 1, |v| Uint::<U8, u8>::new(v as u8));
  check_unsigned_methods!(Uint<U1, u8>, 1, 1, |v| Uint::<U1, u8>::new(v as u8));
  check_unsigned_methods!(Uint<U7, [u8; 1]>, 7, 1, |v| {
    Uint::<U7, [u8; 1]>::checked_from_limbs([v as u8]).expect("7 bits")
  });
  check_unsigned_methods!(Uint<U0, u8>, 0, 1, |_| Uint::<U0, u8>::ZERO);
  check_unsigned_methods!(Uint<U0, [u64; 0]>, 0, 1, |_| Uint::<U0, [u64; 0]>::ZERO);
  check_unsigned_methods!(Uint<U0, [u8; 1]>, 0, 1, |_| Uint::<U0, [u8; 1]>::ZERO);

  check_signed_methods!(
    Int<U7, i8>,
    7,
    |v| Int::<U7, i8>::new(v as i8),
    |v| Uint::<U7, u8>::new(v as u8)
  );
  check_signed_methods!(
    Int<U8, i8>,
    8,
    |v| Int::<U8, i8>::new(v as i8),
    |v| Uint::<U8, u8>::new(v as u8)
  );
  check_signed_methods!(
    Int<U1, i8>,
    1,
    |v| Int::<U1, i8>::new(v as i8),
    |v| Uint::<U1, u8>::new(v as u8)
  );
  check_signed_methods!(
    Int<U7, [u8; 1]>,
    7,
    |v| Int::<U7, [u8; 1]>::checked_from_limbs([v as u8]).expect("7 bits"),
    |v| Uint::<U7, [u8; 1]>::checked_from_limbs([v as u8]).expect("7 bits")
  );
  check_signed_methods!(
    Int<U0, i8>,
    0,
    |_| Int::<U0, i8>::ZERO,
    |_| Uint::<U0, u8>::ZERO
  );
  check_signed_methods!(
    Int<U0, [u64; 0]>,
    0,
    |_| Int::<U0, [u64; 0]>::ZERO,
    |_| Uint::<U0, [u64; 0]>::ZERO
  );
  check_signed_methods!(
    Int<U0, [u8; 1]>,
    0,
    |_| Int::<U0, [u8; 1]>::ZERO,
    |_| Uint::<U0, [u8; 1]>::ZERO
  );
}

/// `ONE` is the storage's one, and the constant of `HasOne` is the inherent one, from one bit for
/// `Uint` and from two bits for `Int`. Zero bits, which have no `ONE`, overflow on a carry and on a
/// borrow, and negate `MIN`, which is zero, to itself.
#[test]
fn one_and_zero_bits() {
  assert_eq!(Uint::<U1, u8>::ONE.value(), 1);
  assert_eq!(Uint::<U7, u128>::ONE.value(), 1);
  assert_eq!(Uint::<U1, [u64; 1]>::ONE.to_limbs(), [1]);
  assert_eq!(Uint::<U7, [u8; 2]>::ONE.to_limbs(), [1, 0]);
  assert_eq!(Int::<U2, i8>::ONE.value(), 1);
  assert_eq!(Int::<U7, i128>::ONE.value(), 1);
  assert_eq!(Int::<U2, [u16; 1]>::ONE.to_limbs(), [1]);
  assert_eq!(Int::<U8, [u8; 3]>::ONE.to_limbs(), [1, 0, 0]);
  assert_eq!(<Uint<U1, u8> as HasOne>::ONE, Uint::<U1, u8>::ONE);
  assert_eq!(<Uint<U7, [u8; 2]> as HasOne>::ONE, Uint::<U7, [u8; 2]>::ONE);
  assert_eq!(<Int<U2, i8> as HasOne>::ONE, Int::<U2, i8>::ONE);
  assert_eq!(<Int<U8, [u8; 3]> as HasOne>::ONE, Int::<U8, [u8; 3]>::ONE);
  assert!(Uint::<U7, u8>::ONE.is_one() && Int::<U7, [u8; 1]>::ONE.is_one());

  type U = Uint<U0, u8>;
  type I = Int<U0, [u64; 0]>;
  assert_eq!(U::ZERO.carrying_add(&U::ZERO, true), (U::ZERO, true));
  assert_eq!(U::ZERO.borrowing_sub(&U::ZERO, true), (U::ZERO, true));
  assert_eq!(U::ZERO.carrying_add(&U::ZERO, false), (U::ZERO, false));
  assert_eq!(I::MIN.checked_neg(), Some(I::ZERO));
  assert_eq!(I::MIN.overflowing_abs(), (I::ZERO, false));
  assert!(!U::ZERO.is_one() && !I::ZERO.is_one());
  // One bit holds no positive value, so `signum` is never one.
  assert_eq!(Int::<U1, i8>::MIN.signum(), Int::<U1, i8>::MIN);
}

/// Checks every pair of values of `$ty`, of `$bits` bits, created by `$from` from their value, or
/// under Miri every 61st pair: where the sum, the difference or the product is in range, the
/// operator in its four forms of values and references and its compound assignment with a value
/// and with a reference give the result of the checked method.
macro_rules! check_operators {
  ($ty:ty, $bits:expr, $signed:expr, $from:expr) => {{
    let from: fn(i128) -> $ty = $from;
    let (min, max) = range($bits, $signed);
    let step = if cfg!(miri) { 61 } else { 1 };
    let count = max - min + 1;
    for pair in (0..count * count).step_by(step) {
      let (x, y) = (from(min + pair / count), from(min + pair % count));
      check_operators!(@op x, y, +, +=, x.checked_add(&y));
      check_operators!(@op x, y, -, -=, x.checked_sub(&y));
      check_operators!(@op x, y, *, *=, x.checked_mul(&y));
    }
  }};
  (@op $x:ident, $y:ident, $op:tt, $assign:tt, $checked:expr) => {
    if let Some(expected) = $checked {
      assert_eq!($x $op $y, expected, "{} {} {}", $x, stringify!($op), $y);
      assert_eq!($x $op &$y, expected);
      assert_eq!(&$x $op $y, expected);
      assert_eq!(&$x $op &$y, expected);
      let mut target = $x;
      target $assign $y;
      assert_eq!(target, expected);
      let mut target = $x;
      target $assign &$y;
      assert_eq!(target, expected);
    }
  };
}

/// The operators give the results of the checked methods where these are `Some`, in a native
/// integer and in an array of one byte, at 7 bits and at zero bits, and so does `-` of `Int`.
#[test]
#[allow(clippy::op_ref, reason = "the operators on references are under test")]
fn operators_match_the_checked_methods() {
  check_operators!(Uint<U7, u8>, 7, false, |v| Uint::<U7, u8>::new(v as u8));
  check_operators!(Int<U7, i8>, 7, true, |v| Int::<U7, i8>::new(v as i8));
  check_operators!(Uint<U7, [u8; 1]>, 7, false, |v| {
    Uint::<U7, [u8; 1]>::checked_from_limbs([v as u8]).expect("7 bits")
  });
  check_operators!(Int<U7, [u8; 1]>, 7, true, |v| {
    Int::<U7, [u8; 1]>::checked_from_limbs([v as u8]).expect("7 bits")
  });
  check_operators!(Uint<U0, u8>, 0, false, |_| Uint::<U0, u8>::ZERO);
  check_operators!(Int<U0, [u64; 0]>, 0, true, |_| Int::<U0, [u64; 0]>::ZERO);
  for value in -64..=63 {
    let native = Int::<U7, i8>::new(value);
    let array = Int::<U7, [u8; 1]>::checked_from_limbs([value as u8]).expect("7 bits");
    if let Some(negated) = native.checked_neg() {
      assert_eq!(-native, negated);
      assert_eq!(-&native, negated);
      assert_eq!((-array).to_limbs(), [negated.value() as u8]);
      assert_eq!((-&array).to_limbs(), [negated.value() as u8]);
    }
  }
  assert_eq!(-Int::<U0, i8>::MIN, Int::<U0, i8>::ZERO);
}

/// Uses the operators, `Sum` and `Product` that the bound `B: BitWidth` gives `Uint<B>` and
/// `Int<B>`, with `ONE` from the bounds on the width, which is at least 7 bits.
#[allow(clippy::op_ref, reason = "the operators on references are under test")]
fn generic_operators<B: BitWidth + NonZero + IsGreater<U1, Output = True>>() {
  let one = Uint::<B>::ONE;
  let (two, three) = (one + one, one + &one + one);
  let mut six = two;
  six *= three;
  assert_eq!(six.to_string(), "6");
  assert_eq!((six - &three).to_string(), "3");
  assert_eq!((&six * &two).to_string(), "12");
  assert_eq!([one, two, three].iter().sum::<Uint<B>>(), six);
  assert_eq!([two, three].into_iter().product::<Uint<B>>(), six);

  let one = Int::<B>::ONE;
  let minus_three = -(one + one + one);
  let mut value = minus_three;
  value -= one;
  value += &one;
  value *= -&one;
  assert_eq!(value.to_string(), "3");
  assert_eq!((minus_three * minus_three).to_string(), "9");
  assert_eq!([minus_three, one, one].into_iter().sum::<Int<B>>(), -one);
  assert_eq!([minus_three, -one].iter().product::<Int<B>>(), value);
}

#[test]
fn bit_width_gives_the_operators() {
  generic_operators::<U7>();
  generic_operators::<U8>();
  generic_operators::<U64>();
  generic_operators::<U65>();
  generic_operators::<U128>();
  generic_operators::<U129>();
  generic_operators::<U200>();
  generic_operators::<U1024>();
}

/// The operators on two references and the negation of a reference need only the storage trait,
/// `UintRepr` or `IntRepr`, not a storage that holds the value inline: code bounded by it uses
/// them, in a native integer and in an array.
#[test]
fn reference_operators_need_only_the_storage_trait() {
  use crate::{FitsBits, IntRepr, UintRepr, i7, i256, u7, u256};

  fn unsigned<B: FitsBits<R>, R: UintRepr>(a: Uint<B, R>, b: Uint<B, R>) -> [Uint<B, R>; 3] {
    [&a + &b, &a - &b, &a * &b]
  }

  fn signed<B: FitsBits<R>, R: IntRepr>(a: Int<B, R>, b: Int<B, R>) -> [Int<B, R>; 4] {
    [&a + &b, &a - &b, &a * &b, -&a]
  }

  assert_eq!(
    unsigned(u7::new(10), u7::new(3)),
    [u7::new(13), u7::new(7), u7::new(30)]
  );
  let (ten, three) = (
    u256::from_limbs([10, 0, 0, 0]),
    u256::from_limbs([3, 0, 0, 0]),
  );
  assert_eq!(
    unsigned(ten, three).map(|value| value.to_string()),
    ["13", "7", "30"]
  );
  assert_eq!(
    signed(i7::new(-10), i7::new(3)),
    [i7::new(-7), i7::new(-13), i7::new(-30), i7::new(10)]
  );
  let (minus_ten, three) = (
    -i256::from_limbs([10, 0, 0, 0]),
    i256::from_limbs([3, 0, 0, 0]),
  );
  assert_eq!(
    signed(minus_ten, three).map(|value| value.to_string()),
    ["-7", "-13", "-30", "10"]
  );
}
