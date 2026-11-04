//! The shifts of `Uint` and `Int`: the overflowing, checked, wrapping and strict shifts of the
//! families and the unbounded and exact shifts of `IntegerBits`, left and right, the right shifts
//! arithmetic for `Int`. Each through the inherent `const fn`s and the traits, against the bit
//! model at every width up to 16 bits and at wider ones; the panics of the strict shifts; their
//! constants; and code generic over the width.

use typenum::{U3, U4, U5, U6, U10, U11, U12, U13, U14, U15};

use super::*;
use crate::{IntRepr, UintRepr};

/// What the shift methods return for a value at one amount, each value read as `V`, left and then
/// right: the overflowing, checked, wrapping and strict shifts of the families, the strict ones
/// where the checked ones return a value and `None` where they would panic, and the unbounded and
/// exact shifts of `IntegerBits`.
#[derive(Debug, PartialEq)]
struct Shift<V> {
  overflowing: [(V, bool); 2],
  checked: [Option<V>; 2],
  wrapping: [V; 2],
  strict: [Option<V>; 2],
  unbounded: [V; 2],
  exact: [Option<V>; 2],
}

/// The [`Shift`] of `$x`, a reference to a value, at each amount of `$amounts`, which `$value`
/// reads, through the methods that the context reaches: the inherent `const fn`s for a concrete
/// storage, and the traits in generic code.
macro_rules! shift_results {
  ($x:expr, $amounts:expr, $value:expr) => {{
    let (x, amounts, value) = ($x, $amounts, $value);
    amounts
      .iter()
      .map(|&n| {
        let (left, left_overflow) = x.overflowing_shl(n);
        let (right, right_overflow) = x.overflowing_shr(n);
        Shift {
          overflowing: [
            (value(&left), left_overflow),
            (value(&right), right_overflow),
          ],
          checked: [
            x.checked_shl(n).map(|shifted| value(&shifted)),
            x.checked_shr(n).map(|shifted| value(&shifted)),
          ],
          wrapping: [value(&x.wrapping_shl(n)), value(&x.wrapping_shr(n))],
          strict: [
            x.checked_shl(n).map(|_| value(&x.strict_shl(n))),
            x.checked_shr(n).map(|_| value(&x.strict_shr(n))),
          ],
          unbounded: [value(&x.unbounded_shl(n)), value(&x.unbounded_shr(n))],
          exact: [
            x.shl_exact(n).map(|shifted| value(&shifted)),
            x.shr_exact(n).map(|shifted| value(&shifted)),
          ],
        }
      })
      .collect::<Vec<_>>()
  }};
}

/// The [`Shift`]s of `x` at each amount of `amounts` through the traits.
fn shifts_through_traits<T, V>(x: &T, amounts: &[u32], value: fn(&T) -> V) -> Vec<Shift<V>>
where
  T: OverflowingOps + CheckedOps + WrappingOps + StrictOps + IntegerBits,
{
  shift_results!(x, amounts, value)
}

/// The [`Shift`]s that the methods return for `p`, the pattern of a value, unsigned or in two's
/// complement if `signed`, at each amount of `amounts`, computed with the bit model: `read` reads
/// each resulting pattern, extending it with zeros or with copies of its sign to the value it
/// stands for.
fn expected_shifts<V>(
  p: &[bool],
  signed: bool,
  amounts: &[u32],
  read: impl Fn(&[bool]) -> V,
) -> Vec<Shift<V>> {
  amounts
    .iter()
    .map(|&n| {
      let (left, left_overflow) = model::overflowing_shl(p, n);
      let (right, right_overflow) = model::overflowing_shr(p, n, signed);
      // A shift overflows exactly when it panics in the strict methods and returns `None` in the
      // checked ones.
      let exact = |overflow: bool, shifted: &[bool]| (!overflow).then(|| read(shifted));
      Shift {
        overflowing: [(read(&left), left_overflow), (read(&right), right_overflow)],
        checked: [exact(left_overflow, &left), exact(right_overflow, &right)],
        wrapping: [read(&left), read(&right)],
        strict: [exact(left_overflow, &left), exact(right_overflow, &right)],
        unbounded: [read(&model::shl(p, n)), read(&model::shr(p, n, signed))],
        exact: [
          model::shl_exact(p, n, signed).map(|shifted| read(&shifted)),
          model::shr_exact(p, n, signed).map(|shifted| read(&shifted)),
        ],
      }
    })
    .collect()
}

/// Checks the shifts of `$ty`, of the sign `$sign` and of `$bits` bits, whose values `$from`
/// creates from an `i128` and `$value` reads back, at each value of `$values`, against the bit
/// model, through the inherent `const fn`s and the traits, at the amounts of [`shift_amounts`].
macro_rules! sweep_shifts {
  ($sign:ident; $ty:ty, $bits:expr, $values:expr, $from:expr, $value:expr, $magnitude:expr) => {{
    let (bits, signed) = ($bits, is_signed!($sign));
    let from: fn(i128) -> $ty = $from;
    let value: fn(&$ty) -> i128 = $value;
    let amounts = shift_amounts(bits);
    for a in $values {
      let x = from(a);
      let expected = expected_shifts(&pattern_of(a, bits), signed, &amounts, |q| {
        value_of(q, signed)
      });
      assert_eq!(
        shift_results!(&x, &amounts, value),
        expected,
        "{a} in {bits} bits"
      );
      assert_eq!(
        shifts_through_traits(&x, &amounts, value),
        expected,
        "{a} in {bits} bits, through the traits"
      );
    }
  }};
}

/// Checks the shifts at the width `B`, `bits` bits, at most 8, in `u8` and `i8`, `[u8; 1]`,
/// `[u8; 2]` and `[u16; 1]` at the values of [`bit_values`] for a storage that every tier checks,
/// and in `u16` and `i16`, `u128` and `i128`, `[u64; 1]` and `[u128; 1]` at those for another.
fn shifts_of_small_widths<B>(bits: u32)
where
  B: FitsBits<u8> + FitsBits<i8> + FitsBits<u16> + FitsBits<i16> + FitsBits<u128> + FitsBits<i128>,
  B: FitsBits<[u8; 1]> + FitsBits<[u8; 2]> + FitsBits<[u16; 1]>,
  B: FitsBits<[u64; 1]> + FitsBits<[u128; 1]>,
{
  let primary = |bits, signed| bit_values(bits, signed, true);
  let secondary = |bits, signed| bit_values(bits, signed, false);
  sweep_storage!(sweep_shifts, primary, native B, bits, u8, i8);
  sweep_storage!(sweep_shifts, primary, array B, bits, [u8; 1]);
  sweep_storage!(sweep_shifts, primary, array B, bits, [u8; 2]);
  sweep_storage!(sweep_shifts, primary, array B, bits, [u16; 1]);
  sweep_storage!(sweep_shifts, secondary, native B, bits, u16, i16);
  sweep_storage!(sweep_shifts, secondary, native B, bits, u128, i128);
  sweep_storage!(sweep_shifts, secondary, array B, bits, [u64; 1]);
  sweep_storage!(sweep_shifts, secondary, array B, bits, [u128; 1]);
}

/// Checks the shifts at the width `B`, `bits` bits, from 9 to 16, in `u16` and `i16`, `[u8; 2]`
/// and `[u16; 1]`, at the values of [`bit_values`].
fn shifts_of_large_widths<B>(bits: u32)
where
  B: FitsBits<u16> + FitsBits<i16> + FitsBits<[u8; 2]> + FitsBits<[u16; 1]>,
{
  let values = |bits, signed| bit_values(bits, signed, true);
  sweep_storage!(sweep_shifts, values, native B, bits, u16, i16);
  sweep_storage!(sweep_shifts, values, array B, bits, [u8; 2]);
  sweep_storage!(sweep_shifts, values, array B, bits, [u16; 1]);
}

/// Every value of each width up to 8 bits, or the values that the tier takes, in native integers
/// and in arrays of one and two limbs, through the inherent `const fn`s and the traits, against
/// the bit model: zero bits, where every amount overflows, one signed bit, and partial and full
/// widths. Miri checks 0, 1, 2, 7 and 8 bits.
#[test]
fn shifts_of_up_to_8_bits() {
  shifts_of_small_widths::<U0>(0);
  shifts_of_small_widths::<U1>(1);
  shifts_of_small_widths::<U2>(2);
  shifts_of_small_widths::<U7>(7);
  shifts_of_small_widths::<U8>(8);
  if !tier().is_miri() {
    shifts_of_small_widths::<U3>(3);
    shifts_of_small_widths::<U4>(4);
    shifts_of_small_widths::<U5>(5);
    shifts_of_small_widths::<U6>(6);
  }
}

/// The values that the tier takes of widths from 9 to 16 bits, with the ends, in `u16` and `i16`,
/// in two limbs of 8 bits and in one of 16 bits: every width in an optimized build, 9, 12, 15 and
/// 16 bits in an unoptimized one, and 9 and 16 under Miri.
#[test]
fn shifts_of_9_to_16_bits() {
  shifts_of_large_widths::<U9>(9);
  shifts_of_large_widths::<U16>(16);
  if !tier().is_miri() {
    shifts_of_large_widths::<U12>(12);
    shifts_of_large_widths::<U15>(15);
  }
  if tier().is_full() {
    shifts_of_large_widths::<U10>(10);
    shifts_of_large_widths::<U11>(11);
    shifts_of_large_widths::<U13>(13);
    shifts_of_large_widths::<U14>(14);
  }
}

/// Checks the shifts of `T` through the traits against the bit model at each pattern of
/// `patterns` and each amount of `amounts`, reading the whole storage of every result, so that its
/// bits above the width are checked too.
fn check_shifts_against_the_model<T>(patterns: &[Vec<bool>], amounts: &[u32])
where
  T: Patterned + OverflowingOps + CheckedOps + WrappingOps + StrictOps + IntegerBits,
{
  let storage = |q: &[bool]| model::extend(q, T::STORAGE, T::SIGNED);
  for p in patterns {
    let x = T::from_pattern(p);
    assert_eq!(
      shifts_through_traits(&x, amounts, T::storage_bits),
      expected_shifts(p, T::SIGNED, amounts, storage),
      "{}",
      model::text(p)
    );
  }
}

/// Checks that the inherent `const fn`s of the shifts of `$ty` agree with the traits at every
/// pattern of `$patterns` and every amount of `$amounts`.
macro_rules! check_shift_paths {
  ($ty:ty, $patterns:expr, $amounts:expr) => {{
    let value: fn(&$ty) -> Vec<bool> = |x| x.storage_bits();
    for p in $patterns {
      let x = <$ty>::from_pattern(p);
      assert_eq!(
        shift_results!(&x, $amounts, value),
        shifts_through_traits(&x, $amounts, value),
        "{}",
        model::text(p)
      );
    }
  }};
}

/// Checks the shifts of `Uint` and `Int` of `$bits` bits, the width `$width`, in each storage of
/// `$storages`, written `unsigned, signed;`, through the traits against the bit model and through
/// the inherent `const fn`s against the traits, at the patterns of [`wide_patterns`] in `$words`
/// words of 64 bits and the amounts of `bigint::bit_amounts`.
macro_rules! shifts_agree {
  ($width:ty, $bits:literal, $words:literal; $($uint:ty, $int:ty);+ $(;)?) => {{
    let patterns = wide_patterns::<$words>($bits);
    let amounts = bigint::bit_amounts(64, $bits);
    $(
      check_shifts_against_the_model::<Uint<$width, $uint>>(&patterns, &amounts);
      check_shifts_against_the_model::<Int<$width, $int>>(&patterns, &amounts);
      check_shift_paths!(Uint<$width, $uint>, &patterns, &amounts);
      check_shift_paths!(Int<$width, $int>, &patterns, &amounts);
    )+
  }};
}

/// The bit patterns of 24 to 4096 bits in storages of each kind that hold them: the shifts agree
/// with the bit model through the traits, and the inherent `const fn`s agree with the traits, at
/// the amounts next to the limb boundaries and to the width. An optimized build adds 1024 and 4096
/// bits; Miri checks 65 and 129 bits, in three storages each.
#[test]
fn storages_agree_on_the_shifts() {
  shifts_agree!(U65, 65, 2; u128, i128; [u8; 9], [u8; 9]; [u64; 2], [u64; 2]);
  shifts_agree!(U129, 129, 3; [u8; 17], [u8; 17]; [u64; 3], [u64; 3]; [u128; 2], [u128; 2]);
  if !tier().is_miri() {
    shifts_agree!(U24, 24, 1; u32, i32; [u8; 3], [u8; 3]; [u16; 2], [u16; 2]; [u64; 1], [u64; 1]);
    shifts_agree!(U65, 65, 2; [u16; 5], [u16; 5]; [u32; 3], [u32; 3]; [u128; 1], [u128; 1]);
    shifts_agree!(U129, 129, 3; [u16; 9], [u16; 9]; [u32; 5], [u32; 5]);
    shifts_agree!(U200, 200, 4; [u8; 25], [u8; 25]; [u16; 13], [u16; 13]; [u32; 7], [u32; 7]);
    shifts_agree!(U200, 200, 4; [u64; 4], [u64; 4]; [u128; 2], [u128; 2]);
  }
  if tier().is_full() {
    shifts_agree!(U1024, 1024, 16; [u32; 32], [u32; 32]; [u64; 16], [u64; 16]);
    shifts_agree!(U4096, 4096, 64; [u8; 512], [u8; 512]; [u16; 256], [u16; 256]);
    shifts_agree!(U4096, 4096, 64; [u64; 64], [u64; 64]; [u128; 32], [u128; 32]);
  }
}

/// The strict shifts panic on an amount of `Bits` or more with the messages of the native
/// `strict_shl` and `strict_shr` in every build, through the inherent `const fn`s and the traits,
/// and report the caller, as the native ones do: at a partial and at the full width of a native
/// integer and of an array, and at zero bits, where every amount overflows.
#[test]
fn strict_shift_panics_report_the_caller() {
  let left = panic_of(|| black_box(1u8).strict_shl(black_box(8)))
    .expect("a panic")
    .0;
  let right = panic_of(|| black_box(1u8).strict_shr(black_box(8)))
    .expect("a panic")
    .0;
  macro_rules! shift_panics {
    ($x:expr, $rhs:expr) => {{
      let (x, rhs) = ($x, $rhs);
      panics_here!(x.strict_shl(rhs), left);
      panics_here!(StrictOps::strict_shl(&x, rhs), left);
      panics_here!(x.strict_shr(rhs), right);
      panics_here!(StrictOps::strict_shr(&x, rhs), right);
    }};
  }
  shift_panics!(u7::MAX, 7);
  shift_panics!(Uint::<U8, u8>::MAX, 8);
  shift_panics!(Uint::<U200>::MAX, 200);
  shift_panics!(u256::MAX, u32::MAX);
  shift_panics!(i7::MIN, 7);
  shift_panics!(Int::<U8, i8>::new(-1), 9);
  shift_panics!(Int::<U200>::MIN, 300);
  shift_panics!(i256::MIN, 256);
  shift_panics!(Uint::<U0, u8>::ZERO, 0);
  shift_panics!(Int::<U0, [u8; 0]>::ZERO, 0);
  assert_eq!(u7::MAX.strict_shl(6), u7::new(64));
  assert_eq!(Int::<U1, i8>::MIN.strict_shr(0), Int::<U1, i8>::MIN);
}

/// The shifts in constant contexts, in a native integer and in an array, at zero bits and at one
/// signed bit.
#[test]
fn shifts_in_constant_contexts() {
  type Bytes = Int<U24, [u8; 3]>;
  type Word = Uint<U7, [u64; 1]>;
  type U0x8 = Uint<U0, u8>;
  type I1 = Int<U1, [u8; 1]>;
  const OVERFLOWING: [(u7, bool); 3] = [
    u7::ONE.overflowing_shl(6),
    u7::ONE.overflowing_shl(7),
    u7::MAX.overflowing_shr(8),
  ];
  const CHECKED: [Option<Bytes>; 3] = [
    Bytes::MIN.checked_shr(23),
    Bytes::MIN.checked_shr(24),
    Bytes::ONE.checked_shl(23),
  ];
  const WRAPPING: [Word; 2] = [Word::MAX.wrapping_shl(7), Word::MAX.wrapping_shr(9)];
  const STRICT: [i7; 2] = [i7::new(-3).strict_shl(2), i7::MIN.strict_shr(6)];
  const UNBOUNDED: [i256; 2] = [i256::MIN.unbounded_shr(300), i256::ONE.unbounded_shl(256)];
  const EXACT: [Option<u7>; 3] = [
    u7::new(3).shl_exact(5),
    u7::new(3).shl_exact(6),
    u7::new(12).shr_exact(2),
  ];
  const ZERO_BITS: [(U0x8, bool); 2] = [
    U0x8::ZERO.overflowing_shl(0),
    U0x8::ZERO.overflowing_shr(u32::MAX),
  ];
  const ONE_BIT: [(I1, bool); 2] = [I1::MIN.overflowing_shl(0), I1::MIN.overflowing_shr(1)];

  assert_eq!(
    OVERFLOWING,
    [(u7::new(64), false), (u7::ONE, true), (u7::new(63), true)]
  );
  assert_eq!(CHECKED, [Some(-Bytes::ONE), None, Some(Bytes::MIN)]);
  assert_eq!(
    WRAPPING,
    [Word::MAX, Word::checked_from_limbs([31]).expect("7 bits")]
  );
  assert_eq!(STRICT, [i7::new(-12), i7::new(-1)]);
  assert_eq!(UNBOUNDED, [-i256::ONE, i256::ZERO]);
  assert_eq!(EXACT, [Some(u7::new(96)), None, Some(u7::new(3))]);
  assert_eq!(ZERO_BITS, [(U0x8::ZERO, true), (U0x8::ZERO, true)]);
  assert_eq!(ONE_BIT, [(I1::MIN, false), (I1::MIN, true)]);
}

/// The shifts of `a` through the traits, as text, in code over `B: BitWidth`.
fn unsigned_shifts_over_the_width<B: BitWidth>(a: Uint<B>, amounts: &[u32]) -> String {
  format!(
    "{:?}",
    shifts_through_traits(&a, amounts, |x| format!("{x:?}"))
  )
}

/// The shifts of `a` through the traits, as text, in code over `B: BitWidth`.
fn signed_shifts_over_the_width<B: BitWidth>(a: Int<B>, amounts: &[u32]) -> String {
  format!(
    "{:?}",
    shifts_through_traits(&a, amounts, |x| format!("{x:?}"))
  )
}

/// Code over `B: BitWidth` reaches every shift of `Uint<B>` and `Int<B>`, which gives the results
/// of the inherent `const fn`s of the default storages: at widths of each kind of default storage,
/// for zero, the limits and the values that wrap from them.
#[test]
fn shifts_over_the_width() {
  macro_rules! agrees {
    ($($width:ident),+ $(,)?) => {$({
      type U = Uint<$width>;
      type I = Int<$width>;
      let amounts = bigint::bit_amounts(64, U::BITS);
      let text: fn(&U) -> String = |x| format!("{x:?}");
      for a in [U::ZERO, U::MAX, U::MAX.wrapping_add(&U::MAX), U::MAX.wrapping_mul(&U::MAX)] {
        let inherent = format!("{:?}", shift_results!(&a, &amounts, text));
        assert_eq!(unsigned_shifts_over_the_width::<$width>(a, &amounts), inherent, "{a}");
      }
      let text: fn(&I) -> String = |x| format!("{x:?}");
      for a in [
        I::ZERO,
        I::MIN,
        I::MAX,
        I::MIN.wrapping_sub(&I::MAX),
        I::MAX.wrapping_add(&I::MAX),
      ] {
        let inherent = format!("{:?}", shift_results!(&a, &amounts, text));
        assert_eq!(signed_shifts_over_the_width::<$width>(a, &amounts), inherent, "{a}");
      }
    })+};
  }

  agrees!(U0, U1, U7, U8, U64, U65, U128, U129, U200, U1024);
}

/// The shifts of the families and the unbounded and exact shifts need only the storage trait,
/// `UintRepr` or `IntRepr`, not a storage that holds the value inline: code bounded by it uses
/// them, in a native integer and in an array, and gives the inherent results.
#[test]
fn shifts_need_only_the_storage_trait() {
  fn unsigned<B: FitsBits<R>, R: UintRepr>(a: &Uint<B, R>) -> String {
    format!(
      "{:?}",
      (
        a.overflowing_shl(3),
        a.checked_shr(1),
        a.wrapping_shl(9),
        a.strict_shr(0),
        a.unbounded_shl(2),
        a.shr_exact(1)
      )
    )
  }

  fn signed<B: FitsBits<R>, R: IntRepr>(a: &Int<B, R>) -> String {
    format!(
      "{:?}",
      (
        a.overflowing_shr(3),
        a.checked_shl(1),
        a.wrapping_shr(9),
        a.strict_shl(0),
        a.unbounded_shr(2),
        a.shl_exact(1)
      )
    )
  }

  // The inherent results that the generic code must give.
  macro_rules! inherent {
    (unsigned $a:expr) => {{
      let a = $a;
      format!(
        "{:?}",
        (
          a.overflowing_shl(3),
          a.checked_shr(1),
          a.wrapping_shl(9),
          a.strict_shr(0),
          a.unbounded_shl(2),
          a.shr_exact(1)
        )
      )
    }};
    (signed $a:expr) => {{
      let a = $a;
      format!(
        "{:?}",
        (
          a.overflowing_shr(3),
          a.checked_shl(1),
          a.wrapping_shr(9),
          a.strict_shl(0),
          a.unbounded_shr(2),
          a.shl_exact(1)
        )
      )
    }};
  }

  for a in [u7::ZERO, u7::new(5), u7::MAX] {
    assert_eq!(unsigned(&a), inherent!(unsigned a), "{a}");
  }
  for a in [u256::ZERO, u256::from_limbs([5, 0, 0, 1]), u256::MAX] {
    assert_eq!(unsigned(&a), inherent!(unsigned a), "{a}");
  }
  for a in [i7::MIN, i7::new(-5), i7::MAX] {
    assert_eq!(signed(&a), inherent!(signed a), "{a}");
  }
  for a in [i256::MIN, i256::from_limbs([5, 0, 0, 0]), -i256::ONE] {
    assert_eq!(signed(&a), inherent!(signed a), "{a}");
  }
  assert_eq!(
    unsigned(&u7::new(5)),
    "((40, false), Some(2), 20, 5, 20, None)"
  );
  assert_eq!(
    signed(&i7::new(-5)),
    "((-1, false), Some(-10), -2, -5, -2, Some(-10))"
  );
}

/// Asserts that `$x << n` and `$x >> n`, for each amount `n` of `$amounts` below `Bits` that the
/// primitive integer `$amount` holds, give the checked shifts by `n` in every form: by value and by
/// reference, on a value and a reference, and as compound assignments with the amount and a
/// reference to it.
macro_rules! shifts_by {
  ($x:expr, $amounts:expr; $($amount:ty),+ $(,)?) => {{
    let x = $x;
    for &n in $amounts {
      let (Some(left), Some(right)) = (x.checked_shl(n), x.checked_shr(n)) else {
        continue;
      };
      $(
        if u128::from(n) <= <$amount>::MAX as u128 {
          let amount = n as $amount;
          let name = stringify!($amount);
          assert_eq!(
            [x << amount, x << &amount, &x << amount, &x << &amount],
            [left; 4],
            "{x:?} << {amount}{name}"
          );
          assert_eq!(
            [x >> amount, x >> &amount, &x >> amount, &x >> &amount],
            [right; 4],
            "{x:?} >> {amount}{name}"
          );
          let mut targets = [x; 4];
          targets[0] <<= amount;
          targets[1] <<= &amount;
          targets[2] >>= amount;
          targets[3] >>= &amount;
          assert_eq!(targets, [left, left, right, right], "{x:?} by {amount}{name}");
        }
      )+
    }
  }};
}

/// `<<` and `>>` take every primitive integer as the amount, by value and by reference, on values
/// and references, and so do `<<=` and `>>=`: below `Bits`, every form gives the checked shift, in
/// native integers and arrays, at partial and full widths and at one signed bit.
#[test]
#[allow(clippy::op_ref, reason = "the operators on references are under test")]
fn shift_operators_take_every_amount_type() {
  macro_rules! every_amount_type {
    ($($x:expr),+ $(,)?) => {$({
      let x = $x;
      let amounts: Vec<u32> = (0..x.count_ones() + x.count_zeros()).collect();
      shifts_by!(x, &amounts; u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
    })+};
  }
  every_amount_type!(
    u7::new(0b101_0011),
    Uint::<U8, u8>::MAX,
    Uint::<U16, [u8; 2]>::from_limbs([0x5a, 0xa5]),
    i7::new(-21),
    Int::<U8, i8>::MIN,
    Int::<U1, [u16; 1]>::MIN,
    i256::from_limbs([u64::MAX, 3, 0, 1 << 62]),
    -i256::ONE,
  );
  // A literal amount on a concrete type falls back to `i32`, as for the native integers.
  assert_eq!((u7::ONE << 3, i256::MIN >> 255), (u7::new(8), -i256::ONE));
}

/// An amount that is negative or `Bits` or more overflows `<<` and `>>` as a native shift does:
/// with overflow checks they panic with the messages of the native operators and report the
/// caller, and without them they shift by the amount modulo `Bits`, taken as a mathematical
/// integer, whatever its type. Every amount overflows zero bits, whose shifts give zero. A
/// panicking compound assignment leaves its target unchanged.
#[test]
#[allow(clippy::op_ref, reason = "the operators on references are under test")]
fn shift_operators_follow_overflow_checks() {
  // The messages of the native shifts that overflow, which only a build with checks has.
  let left =
    panic_of(|| black_box(1u8) << black_box(8u32)).map_or_else(String::new, |panic| panic.0);
  let right =
    panic_of(|| black_box(1u8) >> black_box(8u32)).map_or_else(String::new, |panic| panic.0);
  type U0x8 = Uint<U0, u8>;
  type I0x0 = Int<U0, [u8; 0]>;
  type I1 = Int<U1, i8>;
  // Modulo 7, 7 is 0, -1 is 6, -13 is 1, `u128::MAX = 2^128 - 1` is 3 and `i128::MIN = -2^127`
  // is 5.
  overflows_here!(u7::ONE << 7u32, left, u7::ONE);
  overflows_here!(u7::ONE << -1i32, left, u7::new(64));
  overflows_here!(u7::ONE << u128::MAX, left, u7::new(8));
  overflows_here!(&u7::ONE << &-13i64, left, u7::new(2));
  overflows_here!(i7::MIN >> -1i8, right, i7::new(-1));
  overflows_here!(i7::MIN >> i128::MIN, right, i7::new(-2));
  overflows_here!(Uint::<U8, u8>::MAX << 8usize, left, Uint::<U8, u8>::MAX);
  overflows_here!(Int::<U8, i8>::MIN >> isize::MIN, right, Int::<U8, i8>::MIN);
  overflows_here!(U0x8::ZERO << 0u8, left, U0x8::ZERO);
  overflows_here!(I0x0::ZERO >> 0i16, right, I0x0::ZERO);
  overflows_here!(I1::MIN >> 1i64, right, I1::MIN);
  overflows_here!(i256::MIN >> 256u16, right, i256::MIN);
  overflows_here!(u256::ONE << 257u64, left, u256::from_limbs([2, 0, 0, 0]));

  let checks = overflow_checks();
  let mut target = u7::ONE;
  let panic = panic_of(|| target <<= 8u8);
  assert_eq!(panic.is_some(), checks);
  assert_eq!(target, if checks { u7::ONE } else { u7::new(2) });
  let mut target = i256::MIN;
  let panic = panic_of(|| target >>= &-1i32);
  assert_eq!(panic.is_some(), checks);
  assert_eq!(target, if checks { i256::MIN } else { -i256::ONE });
}

/// The shift operators need only the storage trait, `UintRepr` or `IntRepr`, on a reference, and
/// reach code bounded by `FixedInteger` alone on a value, where a literal amount is a `u32`, as in
/// `x << 3`: in a native integer and in an array, they give the results of the checked shifts.
#[test]
#[allow(clippy::op_ref, reason = "the operators on references are under test")]
fn shift_operators_need_only_the_storage_trait() {
  fn unsigned<B: FitsBits<R>, R: UintRepr>(a: &Uint<B, R>) -> [Uint<B, R>; 3] {
    [a << 3u32, a >> 1i8, a << &2usize]
  }

  fn signed<B: FitsBits<R>, R: IntRepr>(a: &Int<B, R>) -> [Int<B, R>; 3] {
    [a << 3u32, a >> 1i8, a << &2usize]
  }

  fn fixed<T: FixedInteger>(a: T) -> [T; 2] {
    let mut assigned = a.clone();
    assigned <<= 2;
    assigned >>= 1;
    [a << 3, assigned]
  }

  // `$results` gives the shifts by 3 places left, one right and 2 left.
  macro_rules! check {
    ($results:ident, $a:expr) => {{
      let a = $a;
      let [three, two] = [3, 2].map(|n| a.checked_shl(n).expect("below the width"));
      let right = a.checked_shr(1).expect("below the width");
      assert_eq!($results(&a), [three, right, two], "{a}");
      let assigned = two.checked_shr(1).expect("below the width");
      assert_eq!(fixed(a), [three, assigned], "{a}");
    }};
  }

  check!(unsigned, u7::new(0b101_0011));
  check!(unsigned, u256::from_limbs([5, 0, 0, 1 << 63]));
  check!(signed, i7::new(-21));
  check!(signed, i256::MIN);
}
