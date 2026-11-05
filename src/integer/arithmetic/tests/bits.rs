//! The bit operations of `Uint` and `Int`: the counts, the highest and the lowest one and their
//! isolation, the bit reversal, the rotations, the bit at an index, read, set and cleared, the bit
//! width, `is_power_of_two` and the casts between the signs. Each through the inherent
//! `const fn`s and the traits, against the bit model at every width up to 16 bits and at wider
//! ones; the panic of `set_bit`; their constants; code generic over the width; and the table of
//! zero bits and one bit.

use core::marker::PhantomData;

use typenum::{U3, U4, U5, U6, U10, U11, U12, U13, U14, U15};

use super::*;
use crate::{IntRepr, UintRepr};

/// What the methods of `IntegerBits` return for a value, each value read as `V`: the counts, the
/// indexes of the highest and the lowest one, the isolated ones and the reversed bits, at each
/// amount of a list the rotations left and right, and at each index of the same list the bit, and
/// the value with it set and cleared where the index is below the width.
#[derive(Debug, PartialEq)]
struct Bits<V> {
  /// `count_ones`, `count_zeros`, `leading_zeros`, `trailing_zeros`, `leading_ones` and
  /// `trailing_ones`.
  counts: [u32; 6],
  /// `highest_one` and `lowest_one`.
  ones: [Option<u32>; 2],
  /// `isolate_highest_one`, `isolate_lowest_one` and `reverse_bits`.
  values: [V; 3],
  /// `rotate_left` and `rotate_right` at each amount.
  rotated: Vec<[V; 2]>,
  /// `bit` at each index, and `set_bit` with `true` and with `false` where it does not panic.
  indexed: Vec<(bool, Option<[V; 2]>)>,
}

/// The [`Bits`] of `$x`, a reference to a value of `$bits` bits, at each amount and index of
/// `$amounts`, which `$value` reads, through the methods that the context reaches: the inherent
/// `const fn`s for a concrete storage, and the traits in generic code.
macro_rules! bit_results {
  ($x:expr, $bits:expr, $amounts:expr, $value:expr) => {{
    let (x, bits, amounts, value) = ($x, $bits, $amounts, $value);
    Bits {
      counts: [
        x.count_ones(),
        x.count_zeros(),
        x.leading_zeros(),
        x.trailing_zeros(),
        x.leading_ones(),
        x.trailing_ones(),
      ],
      ones: [x.highest_one(), x.lowest_one()],
      values: [
        value(&x.isolate_highest_one()),
        value(&x.isolate_lowest_one()),
        value(&x.reverse_bits()),
      ],
      rotated: amounts
        .iter()
        .map(|&n| [value(&x.rotate_left(n)), value(&x.rotate_right(n))])
        .collect(),
      indexed: amounts
        .iter()
        .map(|&index| {
          let set = (index < bits).then(|| {
            let (mut one, mut zero) = (*x, *x);
            one.set_bit(index, true);
            zero.set_bit(index, false);
            [value(&one), value(&zero)]
          });
          (x.bit(index), set)
        })
        .collect(),
    }
  }};
}

/// The [`Bits`] of `x`, of `bits` bits, through the traits.
fn bits_through_traits<T: IntegerBits + Copy, V>(
  x: &T,
  bits: u32,
  amounts: &[u32],
  value: fn(&T) -> V,
) -> Bits<V> {
  bit_results!(x, bits, amounts, value)
}

/// The [`Bits`] that the methods return for `p`, the pattern of a value, at each amount and index
/// of `amounts`, computed with the bit model: `read` reads each resulting pattern, extending it
/// with zeros or with copies of its sign to the value it stands for.
fn expected_bits<V>(p: &[bool], amounts: &[u32], read: impl Fn(&[bool]) -> V) -> Bits<V> {
  let (bits, ones) = (model::width(p), model::count_ones(p));
  let (highest, lowest) = (model::highest_one(p), model::lowest_one(p));
  // Every bit below the width is set or cleared, and none from the width up.
  let set = |index: u32, bit: bool| model::with_bit(p, index, bit).map(|changed| read(&changed));
  Bits {
    counts: [
      ones,
      bits - ones,
      model::leading_zeros(p),
      model::trailing_zeros(p),
      model::leading_ones(p),
      model::trailing_ones(p),
    ],
    ones: [highest, lowest],
    values: [
      read(&model::only(bits, highest)),
      read(&model::only(bits, lowest)),
      read(&model::reverse(p)),
    ],
    rotated: amounts
      .iter()
      .map(|&n| {
        [
          read(&model::rotate_left(p, n)),
          read(&model::rotate_right(p, n)),
        ]
      })
      .collect(),
    indexed: amounts
      .iter()
      .map(|&index| {
        let changed = set(index, true).zip(set(index, false));
        (model::bit(p, index), changed.map(|(one, zero)| [one, zero]))
      })
      .collect(),
  }
}

/// Checks the methods of `UnsignedBits`, `SignedBits` and `is_power_of_two`, for the sign token
/// `$sign`, on `$x`, the value `$a` of `$ty` of `$bits` bits, through the inherent `const fn`s and
/// the traits: the bit width, `is_power_of_two` and the cast to `Int` for `Uint`, and for `Int`
/// the cast to `Uint`, whose result `$magnitude` reads. A cast back gives the value again.
macro_rules! sign_bits {
  (unsigned, $ty:ty, $x:expr, $a:expr, $bits:expr, $magnitude:expr) => {{
    let (x, a, bits) = ($x, $a, $bits);
    let p = pattern_of(a, bits);
    assert_eq!(
      both!(UnsignedBits::bit_width(x)),
      bits - model::leading_zeros(&p),
      "{a}"
    );
    assert_eq!(
      both!(UnsignedMath::is_power_of_two(x)),
      model::count_ones(&p) == 1,
      "{a}"
    );
    let signed = both!(UnsignedBits::cast_signed(*x));
    // The value of the cast, which `Display` writes in every storage.
    assert_eq!(
      signed.to_string(),
      value_of(&p, true).to_string(),
      "{a} read in two's complement"
    );
    assert_eq!(SignedBits::cast_unsigned(signed), *x, "{a} cast back");
  }};
  (signed, $ty:ty, $x:expr, $a:expr, $bits:expr, $magnitude:expr) => {{
    let (x, a, bits) = ($x, $a, $bits);
    let magnitude: fn(&<$ty as IntegerMath>::Magnitude) -> i128 = $magnitude;
    let p = pattern_of(a, bits);
    let unsigned = both!(SignedBits::cast_unsigned(*x));
    assert_eq!(
      magnitude(&unsigned),
      value_of(&p, false),
      "{a} read unsigned"
    );
    assert_eq!(UnsignedBits::cast_signed(unsigned), *x, "{a} cast back");
  }};
}

/// Checks the bit methods of `$ty`, of the sign `$sign` and of `$bits` bits, whose values `$from`
/// creates from an `i128` and `$value` and `$magnitude` read back, at each value of `$values`,
/// against the bit model, through the inherent `const fn`s and the traits, at the amounts and
/// indexes of [`shift_amounts`].
macro_rules! sweep_bits {
  ($sign:ident; $ty:ty, $bits:expr, $values:expr, $from:expr, $value:expr, $magnitude:expr) => {{
    let (bits, signed) = ($bits, is_signed!($sign));
    let from: fn(i128) -> $ty = $from;
    let value: fn(&$ty) -> i128 = $value;
    let amounts = shift_amounts(bits);
    for a in $values {
      let x = from(a);
      let expected = expected_bits(&pattern_of(a, bits), &amounts, |q| value_of(q, signed));
      assert_eq!(
        bit_results!(&x, bits, &amounts, value),
        expected,
        "{a} in {bits} bits"
      );
      assert_eq!(
        bits_through_traits(&x, bits, &amounts, value),
        expected,
        "{a} in {bits} bits, through the traits"
      );
      sign_bits!($sign, $ty, &x, a, bits, $magnitude);
    }
  }};
}

/// Checks the bit methods at the width `B`, `bits` bits, at most 8, in `u8` and `i8`, `[u8; 1]`,
/// `[u8; 2]` and `[u16; 1]` at the values of [`bit_values`] for a storage that every tier checks,
/// and in `u16` and `i16`, `u128` and `i128`, `[u64; 1]` and `[u128; 1]` at those for another.
fn bits_of_small_widths<B>(bits: u32)
where
  B: FitsBits<u8> + FitsBits<i8> + FitsBits<u16> + FitsBits<i16> + FitsBits<u128> + FitsBits<i128>,
  B: FitsBits<[u8; 1]> + FitsBits<[u8; 2]> + FitsBits<[u16; 1]>,
  B: FitsBits<[u64; 1]> + FitsBits<[u128; 1]>,
{
  let primary = |bits, signed| bit_values(bits, signed, true);
  let secondary = |bits, signed| bit_values(bits, signed, false);
  sweep_storage!(sweep_bits, primary, native B, bits, u8, i8);
  sweep_storage!(sweep_bits, primary, array B, bits, [u8; 1]);
  sweep_storage!(sweep_bits, primary, array B, bits, [u8; 2]);
  sweep_storage!(sweep_bits, primary, array B, bits, [u16; 1]);
  sweep_storage!(sweep_bits, secondary, native B, bits, u16, i16);
  sweep_storage!(sweep_bits, secondary, native B, bits, u128, i128);
  sweep_storage!(sweep_bits, secondary, array B, bits, [u64; 1]);
  sweep_storage!(sweep_bits, secondary, array B, bits, [u128; 1]);
}

/// Checks the bit methods at the width `B`, `bits` bits, from 9 to 16, in `u16` and `i16`,
/// `[u8; 2]` and `[u16; 1]`, at the values of [`bit_values`].
fn bits_of_large_widths<B>(bits: u32)
where
  B: FitsBits<u16> + FitsBits<i16> + FitsBits<[u8; 2]> + FitsBits<[u16; 1]>,
{
  let values = |bits, signed| bit_values(bits, signed, true);
  sweep_storage!(sweep_bits, values, native B, bits, u16, i16);
  sweep_storage!(sweep_bits, values, array B, bits, [u8; 2]);
  sweep_storage!(sweep_bits, values, array B, bits, [u16; 1]);
}

/// Every value of each width up to 8 bits, or the values that the tier takes, in native integers
/// and in arrays of one and two limbs, through the inherent `const fn`s and the traits, against
/// the bit model: zero bits, one signed bit, and partial and full widths. Miri checks 0, 1, 2, 7
/// and 8 bits.
#[test]
fn bit_methods_of_up_to_8_bits() {
  bits_of_small_widths::<U0>(0);
  bits_of_small_widths::<U1>(1);
  bits_of_small_widths::<U2>(2);
  bits_of_small_widths::<U7>(7);
  bits_of_small_widths::<U8>(8);
  if !tier().is_miri() {
    bits_of_small_widths::<U3>(3);
    bits_of_small_widths::<U4>(4);
    bits_of_small_widths::<U5>(5);
    bits_of_small_widths::<U6>(6);
  }
}

/// The values that the tier takes of widths from 9 to 16 bits, with the ends, in `u16` and `i16`,
/// in two limbs of 8 bits and in one of 16 bits: every width in an optimized build, 9, 12, 15 and
/// 16 bits in an unoptimized one, and 9 and 16 under Miri.
#[test]
fn bit_methods_of_9_to_16_bits() {
  bits_of_large_widths::<U9>(9);
  bits_of_large_widths::<U16>(16);
  if !tier().is_miri() {
    bits_of_large_widths::<U12>(12);
    bits_of_large_widths::<U15>(15);
  }
  if tier().is_full() {
    bits_of_large_widths::<U10>(10);
    bits_of_large_widths::<U11>(11);
    bits_of_large_widths::<U13>(13);
    bits_of_large_widths::<U14>(14);
  }
}

/// Checks the methods of `IntegerBits` of `T`, of `bits` bits, through the traits against the bit
/// model at each pattern of `patterns` and each amount and index of `amounts`, reading the whole
/// storage of every result, so that its bits above the width are checked too.
fn check_bits_against_the_model<T>(bits: u32, patterns: &[Vec<bool>], amounts: &[u32])
where
  T: Patterned + IntegerBits,
{
  let storage = |q: &[bool]| model::extend(q, T::STORAGE, T::SIGNED);
  for p in patterns {
    let x = T::from_pattern(p);
    assert_eq!(
      bits_through_traits(&x, bits, amounts, T::storage_bits),
      expected_bits(p, amounts, storage),
      "{} in {bits} bits",
      model::text(p)
    );
  }
}

/// Checks the bit width, `is_power_of_two` and the casts of `Uint` of `T` and of its `Int` through
/// the traits against the bit model at each pattern of `patterns`, and that each cast back gives
/// the value again.
fn check_casts_against_the_model<T>(patterns: &[Vec<bool>])
where
  T: Patterned + IntegerBits + UnsignedBits + UnsignedMath,
  T::Signed: Patterned + SignedBits + IntegerMath<Magnitude = T>,
{
  let width = |p: &[bool]| model::width(p);
  for p in patterns {
    let (x, y) = (T::from_pattern(p), T::Signed::from_pattern(p));
    assert_eq!(x.bit_width(), width(p) - model::leading_zeros(p));
    assert_eq!(x.is_power_of_two(), model::count_ones(p) == 1);
    assert_eq!(
      x.cast_signed().storage_bits(),
      model::extend(p, T::Signed::STORAGE, true),
      "{}",
      model::text(p)
    );
    assert_eq!(
      y.cast_unsigned().storage_bits(),
      model::extend(p, T::STORAGE, false),
      "{}",
      model::text(p)
    );
    assert_eq!((x.cast_signed(), y.cast_unsigned()), (y, x));
  }
}

/// Checks that the inherent `const fn`s of the bit methods of `$ty`, of `$bits` bits, agree with
/// the traits at every pattern of `$patterns` and every amount and index of `$amounts`.
macro_rules! check_bit_paths {
  ($ty:ty, $bits:expr, $patterns:expr, $amounts:expr) => {{
    let value: fn(&$ty) -> Vec<bool> = |x| x.storage_bits();
    for p in $patterns {
      let x = <$ty>::from_pattern(p);
      assert_eq!(
        bit_results!(&x, $bits, $amounts, value),
        bits_through_traits(&x, $bits, $amounts, value),
        "{}",
        model::text(p)
      );
    }
  }};
}

/// Checks the bit methods of `Uint` and `Int` of `$bits` bits, the width `$width`, in each storage
/// of `$storages`, written `unsigned, signed;`, through the traits against the bit model and
/// through the inherent `const fn`s against the traits, at the patterns of [`wide_patterns`] in
/// `$words` words of 64 bits and the amounts of `bigint::bit_amounts`.
macro_rules! bits_agree {
  ($width:ty, $bits:literal, $words:literal; $($uint:ty, $int:ty);+ $(;)?) => {{
    let patterns = wide_patterns::<$words>($bits);
    let amounts = bigint::bit_amounts(64, $bits);
    $(
      check_bits_against_the_model::<Uint<$width, $uint>>($bits, &patterns, &amounts);
      check_bits_against_the_model::<Int<$width, $int>>($bits, &patterns, &amounts);
      check_casts_against_the_model::<Uint<$width, $uint>>(&patterns);
      check_bit_paths!(Uint<$width, $uint>, $bits, &patterns, &amounts);
      check_bit_paths!(Int<$width, $int>, $bits, &patterns, &amounts);
    )+
  }};
}

/// The bit patterns of 24 to 4096 bits in storages of each kind that hold them: the bit methods
/// agree with the bit model through the traits, and the inherent `const fn`s agree with the traits.
/// An optimized build adds 1024 and 4096 bits; Miri checks 65 and 129 bits, in three storages
/// each.
#[test]
fn storages_agree_on_the_bit_methods() {
  bits_agree!(U65, 65, 2; u128, i128; [u8; 9], [u8; 9]; [u64; 2], [u64; 2]);
  bits_agree!(U129, 129, 3; [u8; 17], [u8; 17]; [u64; 3], [u64; 3]; [u128; 2], [u128; 2]);
  if !tier().is_miri() {
    bits_agree!(U24, 24, 1; u32, i32; [u8; 3], [u8; 3]; [u16; 2], [u16; 2]; [u64; 1], [u64; 1]);
    bits_agree!(U65, 65, 2; [u16; 5], [u16; 5]; [u32; 3], [u32; 3]; [u128; 1], [u128; 1]);
    bits_agree!(U129, 129, 3; [u16; 9], [u16; 9]; [u32; 5], [u32; 5]);
    bits_agree!(U200, 200, 4; [u8; 25], [u8; 25]; [u16; 13], [u16; 13]; [u32; 7], [u32; 7]);
    bits_agree!(U200, 200, 4; [u64; 4], [u64; 4]; [u128; 2], [u128; 2]);
  }
  if tier().is_full() {
    bits_agree!(U1024, 1024, 16; [u32; 32], [u32; 32]; [u64; 16], [u64; 16]);
    bits_agree!(U4096, 4096, 64; [u8; 512], [u8; 512]; [u16; 256], [u16; 256]);
    bits_agree!(U4096, 4096, 64; [u64; 64], [u64; 64]; [u128; 32], [u128; 32]);
  }
}

/// Asserts that `set_bit`, inherent and through the trait, at every index of `$ty` below its
/// width, on each value of `$values`, sets or clears that bit and no other, and that the result
/// holds a value of the width, with the copies of the sign of `Int`; and that at the width and at
/// `u32::MAX` it panics with the message of `$message` and reports the caller.
macro_rules! set_every_bit {
  ($ty:ty, $message:expr; $($value:expr),+ $(,)?) => {{
    let bits = <$ty>::BITS;
    for x in [$($value),+] {
      let x: $ty = x;
      let storage = x.storage_bits();
      let p = model::pattern(bits, |i| storage[i as usize]);
      for index in 0..bits {
        for bit in [false, true] {
          let expected = <$ty>::from_pattern(&model::with_bit(&p, index, bit).expect("in range"));
          let (mut inherent, mut through_trait) = (x, x);
          inherent.set_bit(index, bit);
          IntegerBits::set_bit(&mut through_trait, index, bit);
          assert_eq!(
            (inherent, through_trait),
            (expected, expected),
            "bit {index} of {x:?} set to {bit}"
          );
        }
      }
      for index in [bits, u32::MAX] {
        let mut target = x;
        panics_here!(target.set_bit(index, true), $message);
        panics_here!(IntegerBits::set_bit(&mut target, index, false), $message);
        assert_eq!(target, x);
      }
    }
  }};
}

/// `set_bit` writes every index below the width and panics from the width up, through the inherent
/// `const fn`s and the trait, reporting the caller: at zero bits, where every index panics, at one
/// signed bit, whose only bit is the sign, and at partial and full widths of native integers and
/// arrays, where setting or clearing the sign bit of `Int` changes the copies of the sign.
#[test]
fn set_bit_writes_every_index_of_the_width() {
  let message = "bit index out of range".to_string();
  type U0x8 = Uint<U0, u8>;
  type I0x0 = Int<U0, [u8; 0]>;
  type I1x8 = Int<U1, i8>;
  type I1x16 = Int<U1, [u16; 1]>;
  type I7x64 = Int<U7, [u64; 1]>;
  type U200x4 = Uint<U200, [u64; 4]>;
  type U8x8 = Uint<U8, u8>;
  type I8x8 = Int<U8, i8>;
  set_every_bit!(U0x8, message; U0x8::ZERO);
  set_every_bit!(I0x0, message; I0x0::ZERO);
  set_every_bit!(I1x8, message; I1x8::MIN, I1x8::ZERO);
  set_every_bit!(I1x16, message; I1x16::MIN, I1x16::ZERO);
  set_every_bit!(u7, message; u7::ZERO, u7::MAX, u7::new(0b101_0101));
  set_every_bit!(i7, message; i7::MIN, i7::new(-1), i7::ZERO, i7::MAX);
  set_every_bit!(I7x64, message; I7x64::MIN, I7x64::ZERO, I7x64::MAX);
  set_every_bit!(U200x4, message; U200x4::ZERO, U200x4::MAX);
  set_every_bit!(U8x8, message; U8x8::ZERO, U8x8::MAX);
  set_every_bit!(I8x8, message; I8x8::MIN, I8x8::ZERO, I8x8::MAX);
  set_every_bit!(i256, message; i256::MIN, i256::ZERO, i256::MAX);
}

/// The bit methods in constant contexts, in a native integer and in an array, at zero bits and at
/// one signed bit, with `set_bit` on a local value.
#[test]
fn bit_methods_in_constant_contexts() {
  type Bytes = Int<U24, [u8; 3]>;
  type Word = Uint<U7, [u64; 1]>;
  type U0x8 = Uint<U0, u8>;
  type I0x0 = Int<U0, [u8; 0]>;
  type I1 = Int<U1, i8>;
  const COUNTS: [u32; 6] = [
    u7::new(0b101_0000).count_ones(),
    u7::new(0b101_0000).count_zeros(),
    u7::new(0b101_0000).leading_zeros(),
    u7::new(0b101_0000).trailing_zeros(),
    Bytes::MIN.leading_ones(),
    Bytes::MAX.trailing_ones(),
  ];
  const ONES: [Option<u32>; 4] = [
    Word::MAX.highest_one(),
    Word::ZERO.lowest_one(),
    Bytes::MIN.highest_one(),
    Bytes::MIN.lowest_one(),
  ];
  const ISOLATED: [i7; 3] = [
    i7::new(-6).isolate_highest_one(),
    i7::new(-6).isolate_lowest_one(),
    i7::new(0b001_0110).reverse_bits(),
  ];
  const ROTATED: [Word; 2] = [Word::ONE.rotate_right(1), Word::MAX.rotate_left(u32::MAX)];
  const INDEXED: [bool; 3] = [i256::MIN.bit(255), i256::MIN.bit(256), Bytes::MIN.bit(23)];
  const SET: (i7, Bytes) = {
    let (mut small, mut wide) = (i7::ZERO, Bytes::MIN);
    small.set_bit(6, true);
    wide.set_bit(23, false);
    (small, wide)
  };
  const UNSIGNED: (u32, bool, bool, i7) = (
    u7::new(5).bit_width(),
    u7::new(64).is_power_of_two(),
    Word::ZERO.is_power_of_two(),
    u7::MAX.cast_signed(),
  );
  const SIGNED: (u7, Uint<U24, [u8; 3]>) = (i7::MIN.cast_unsigned(), Bytes::MIN.cast_unsigned());
  const ZERO_BITS: (u32, u32, Option<u32>, U0x8, I0x0) = (
    U0x8::ZERO.count_zeros(),
    I0x0::ZERO.leading_zeros(),
    U0x8::ZERO.highest_one(),
    U0x8::ZERO.rotate_left(3),
    I0x0::ZERO.reverse_bits(),
  );
  const ONE_BIT: (u32, Option<u32>, I1, Uint<U1, u8>) = (
    I1::MIN.leading_ones(),
    I1::MIN.highest_one(),
    I1::MIN.isolate_lowest_one(),
    I1::MIN.cast_unsigned(),
  );

  assert_eq!(COUNTS, [2, 5, 0, 4, 1, 23]);
  assert_eq!(ONES, [Some(6), None, Some(23), Some(23)]);
  assert_eq!(ISOLATED, [i7::MIN, i7::new(2), i7::new(0b011_0100)]);
  assert_eq!(
    ROTATED,
    [Word::checked_from_limbs([64]).expect("7 bits"), Word::MAX]
  );
  assert_eq!(INDEXED, [true, false, true]);
  assert_eq!(SET, (i7::MIN, Bytes::ZERO));
  assert_eq!(UNSIGNED, (3, true, false, i7::new(-1)));
  assert_eq!(
    SIGNED,
    (u7::new(64), Uint::<U24, [u8; 3]>::from_limbs([0, 0, 0x80]))
  );
  assert_eq!(ZERO_BITS, (0, 0, None, U0x8::ZERO, I0x0::ZERO));
  assert_eq!(ONE_BIT, (1, Some(0), I1::MIN, Uint::<U1, u8>::ONE));
}

/// The bit results of `a` through the traits, as text, in code over `B: BitWidth`, and the casts
/// there: `Uint<B>` casts to `Int<B>` and back.
fn unsigned_bits_over_the_width<B: BitWidth>(a: Uint<B>, amounts: &[u32]) -> Vec<String> {
  let signed: Int<B> = a.cast_signed();
  let back: Uint<B> = signed.cast_unsigned();
  let results = bits_through_traits(&a, Uint::<B>::BITS, amounts, |x| format!("{x:?}"));
  vec![
    format!("{results:?}"),
    format!("{:?}", (a.bit_width(), a.is_power_of_two(), signed, back)),
  ]
}

/// The bit results of `a` through the traits, as text, in code over `B: BitWidth`, and the casts
/// there: `Int<B>` casts to `Uint<B>` and back.
fn signed_bits_over_the_width<B: BitWidth>(a: Int<B>, amounts: &[u32]) -> Vec<String> {
  let unsigned: Uint<B> = a.cast_unsigned();
  let back: Int<B> = unsigned.cast_signed();
  let results = bits_through_traits(&a, Int::<B>::BITS, amounts, |x| format!("{x:?}"));
  vec![format!("{results:?}"), format!("{:?}", (unsigned, back))]
}

/// Code over `B: BitWidth` reaches every bit method of `Uint<B>` and `Int<B>`, which gives the
/// results of the inherent `const fn`s of the default storages, and casts `Uint<B>` to `Int<B>`
/// and back: at widths of each kind of default storage, for zero, the limits and the values that
/// wrap from them.
#[test]
fn bit_methods_over_the_width() {
  macro_rules! agrees {
    ($($width:ident),+ $(,)?) => {$({
      type U = Uint<$width>;
      type I = Int<$width>;
      let amounts = bigint::bit_amounts(64, U::BITS);
      let text: fn(&U) -> String = |x| format!("{x:?}");
      for a in [U::ZERO, U::MAX, U::MAX.wrapping_add(&U::MAX), U::MAX.wrapping_mul(&U::MAX)] {
        let inherent = vec![
          format!("{:?}", bit_results!(&a, U::BITS, &amounts, text)),
          format!(
            "{:?}",
            (a.bit_width(), a.is_power_of_two(), a.cast_signed(), a.cast_signed().cast_unsigned())
          ),
        ];
        assert_eq!(unsigned_bits_over_the_width::<$width>(a, &amounts), inherent, "{a}");
      }
      let text: fn(&I) -> String = |x| format!("{x:?}");
      for a in [
        I::ZERO,
        I::MIN,
        I::MAX,
        I::MIN.wrapping_sub(&I::MAX),
        I::MAX.wrapping_add(&I::MAX),
      ] {
        let inherent = vec![
          format!("{:?}", bit_results!(&a, I::BITS, &amounts, text)),
          format!("{:?}", (a.cast_unsigned(), a.cast_unsigned().cast_signed())),
        ];
        assert_eq!(signed_bits_over_the_width::<$width>(a, &amounts), inherent, "{a}");
      }
    })+};
  }

  agrees!(U0, U1, U7, U8, U64, U65, U128, U129, U200, U1024);
}

/// `IntegerBits`, `UnsignedBits` and `SignedBits` need only the storage trait, `UintRepr` or
/// `IntRepr`, not a storage that holds the value inline: code bounded by it uses them, in a native
/// integer and in an array. A cast's result needs the width relation of its own storage, and the
/// storage of `Int` of the same size as a storage of `Uint` is named in generic code.
#[test]
fn bit_traits_need_only_the_storage_trait() {
  fn unsigned<B: FitsBits<R>, R: UintRepr>(a: &Uint<B, R>) -> String {
    format!(
      "{:?}",
      (
        a.count_ones(),
        a.highest_one(),
        a.rotate_left(3),
        a.bit(0),
        a.bit_width(),
        a.is_power_of_two(),
      )
    )
  }

  fn signed<B: FitsBits<R>, R: IntRepr>(a: &Int<B, R>) -> String {
    format!(
      "{:?}",
      (
        a.leading_ones(),
        a.isolate_lowest_one(),
        a.reverse_bits(),
        a.unbounded_shr(1)
      )
    )
  }

  fn round_trip<B, R>(a: Uint<B, R>) -> Uint<B, R>
  where
    B: FitsBits<R> + FitsBits<<R as UintRepr>::Signed>,
    R: UintRepr,
  {
    a.cast_signed().cast_unsigned()
  }

  fn set<B: FitsBits<R>, R: IntRepr>(mut a: Int<B, R>, index: u32) -> Int<B, R> {
    a.set_bit(index, true);
    a
  }

  fn signed_of<R: UintRepr>() -> PhantomData<R::Signed> {
    PhantomData
  }

  // The inherent results that the generic code must give.
  macro_rules! unsigned_inherent {
    ($a:expr) => {{
      let a = $a;
      format!(
        "{:?}",
        (
          a.count_ones(),
          a.highest_one(),
          a.rotate_left(3),
          a.bit(0),
          a.bit_width(),
          a.is_power_of_two(),
        )
      )
    }};
  }
  macro_rules! signed_inherent {
    ($a:expr) => {{
      let a = $a;
      format!(
        "{:?}",
        (
          a.leading_ones(),
          a.isolate_lowest_one(),
          a.reverse_bits(),
          a.unbounded_shr(1)
        )
      )
    }};
  }
  assert_eq!(unsigned(&u7::new(5)), "(2, Some(2), 40, true, 3, false)");
  let four = u256::from_limbs([4, 0, 0, 0]);
  assert_eq!(unsigned(&four), unsigned_inherent!(four));
  assert_eq!(signed(&i7::new(-8)), "(4, 8, 15, -4)");
  let minus_eight = i256::from_limbs([u64::MAX - 7, u64::MAX, u64::MAX, u64::MAX]);
  assert_eq!(signed(&minus_eight), signed_inherent!(minus_eight));
  assert_eq!(unsigned(&u7::MAX), unsigned_inherent!(u7::MAX));
  assert_eq!(signed(&i7::MIN), signed_inherent!(i7::MIN));
  assert_eq!(round_trip(u7::MAX), u7::MAX);
  assert_eq!(round_trip(u256::MAX), u256::MAX);
  assert_eq!(set(i7::ZERO, 6), i7::MIN);
  assert_eq!(set(i256::ZERO, 255), i256::MIN);
  let _: PhantomData<i8> = signed_of::<u8>();
  let _: PhantomData<[u64; 4]> = signed_of::<[u64; 4]>();
}

/// Asserts the rows of the table of zero bits and one bit that hold for every value `$x` of the
/// integer `$ty`, `u0`, `i0`, `u1` or `i1` in some storage, with the results that depend on the
/// value as `$name = $expected`, where `$expected` may read `x`.
macro_rules! zero_and_one_bit {
  ($ty:ty; $x:expr; $($name:ident($($arg:expr),*) = $expected:expr),+ $(,)?) => {{
    let x: $ty = $x;
    $(
      assert_eq!(
        x.$name($($arg),*),
        $expected,
        "{}({}) of {x:?}",
        stringify!($name),
        stringify!($($arg),*)
      );
    )+
  }};
}

/// The table of zero bits and one bit, row by row: `u0` and `i0`, which hold only zero, `u1`,
/// which holds 0 and 1, and `i1`, which holds -1 and 0, each in a native integer and in an array.
/// Every amount overflows zero bits and takes no remainder by zero, and zero is its own complement
/// there; every amount but zero overflows one bit, whose rotations, reversal and exact shifts by
/// zero return the value, and whose bitwise operations are those of booleans.
#[test]
fn zero_bits_and_one_bit() {
  macro_rules! zero_bits {
    ($($ty:ty),+) => {$({
      type T = $ty;
      zero_and_one_bit!(T; T::ZERO;
        count_ones() = 0, count_zeros() = 0, leading_zeros() = 0, trailing_zeros() = 0,
        leading_ones() = 0, trailing_ones() = 0, highest_one() = None, lowest_one() = None,
        isolate_highest_one() = T::ZERO, isolate_lowest_one() = T::ZERO,
        reverse_bits() = T::ZERO, rotate_left(0) = T::ZERO, rotate_left(5) = T::ZERO,
        rotate_right(u32::MAX) = T::ZERO, overflowing_shl(0) = (T::ZERO, true),
        overflowing_shr(5) = (T::ZERO, true), checked_shl(0) = None, checked_shr(0) = None,
        wrapping_shl(0) = T::ZERO, wrapping_shr(u32::MAX) = T::ZERO,
        unbounded_shl(0) = T::ZERO, unbounded_shr(1) = T::ZERO, shl_exact(0) = None,
        shr_exact(0) = None, bit(0) = false, not() = T::ZERO, bitand(T::ZERO) = T::ZERO,
        bitor(T::ZERO) = T::ZERO, bitxor(T::ZERO) = T::ZERO,
      );
      assert_eq!(
        [!T::ZERO, T::ZERO & T::ZERO, T::ZERO | T::ZERO, T::ZERO ^ T::ZERO],
        [T::ZERO; 4]
      );
    })+};
  }
  zero_bits!(Uint<U0, u8>, Uint<U0, [u64; 1]>, Int<U0, i8>, Int<U0, [u8; 0]>);
  let zero = Uint::<U0, u8>::ZERO;
  assert_eq!((zero.bit_width(), zero.is_power_of_two()), (0, false));
  assert_eq!(zero.cast_signed(), Int::<U0, i8>::ZERO);
  assert_eq!(
    Int::<U0, [u8; 0]>::ZERO.cast_unsigned(),
    Uint::<U0, [u8; 0]>::ZERO
  );

  macro_rules! one_bit {
    ($($ty:ty: $one:expr),+) => {$({
      type T = $ty;
      for (x, one) in [(T::ZERO, false), ($one, true)] {
        // `x` again for every amount of a rotation, and for a shift by zero. The bitwise
        // operations are those of booleans, the complement flips the only bit.
        let ones = u32::from(one);
        let flipped = if one { T::ZERO } else { $one };
        zero_and_one_bit!(T; x;
          count_ones() = ones, count_zeros() = 1 - ones, leading_zeros() = 1 - ones,
          trailing_zeros() = 1 - ones, leading_ones() = ones, trailing_ones() = ones,
          highest_one() = one.then_some(0), lowest_one() = one.then_some(0),
          isolate_highest_one() = x, isolate_lowest_one() = x, reverse_bits() = x,
          rotate_left(1) = x, rotate_right(u32::MAX) = x, overflowing_shl(0) = (x, false),
          overflowing_shl(1) = (x, true), overflowing_shr(u32::MAX) = (x, true),
          checked_shl(1) = None, wrapping_shl(1) = x, unbounded_shl(0) = x,
          unbounded_shl(1) = T::ZERO, shl_exact(0) = Some(x), shl_exact(1) = None,
          shr_exact(0) = Some(x), shr_exact(1) = None, bit(0) = one, bit(1) = false,
          not() = flipped, bitand($one) = x, bitor(T::ZERO) = x, bitxor($one) = flipped,
        );
        assert_eq!([!x, x & $one, x | T::ZERO, x ^ $one], [flipped, x, x, flipped]);
      }
    })+};
  }
  one_bit!(Uint<U1, u8>: Uint::<U1, u8>::ONE, Uint<U1, [u8; 1]>: Uint::<U1, [u8; 1]>::ONE);
  one_bit!(Int<U1, i8>: Int::<U1, i8>::MIN, Int<U1, [u16; 1]>: Int::<U1, [u16; 1]>::MIN);
  // The right shift of `Uint` brings in zeros and that of `Int` copies of the sign.
  assert_eq!(Uint::<U1, u8>::ONE.unbounded_shr(1), Uint::<U1, u8>::ZERO);
  assert_eq!(Int::<U1, i8>::MIN.unbounded_shr(1), Int::<U1, i8>::MIN);
  assert_eq!(
    Int::<U1, [u16; 1]>::MIN.unbounded_shr(u32::MAX),
    Int::<U1, [u16; 1]>::MIN
  );
  // One bit of `Uint` is 1 and of `Int` -1.
  assert_eq!(Uint::<U1, u8>::ONE.cast_signed(), Int::<U1, i8>::MIN);
  assert_eq!(
    Int::<U1, [u16; 1]>::MIN.cast_unsigned(),
    Uint::<U1, [u16; 1]>::ONE
  );
  assert_eq!(
    (
      Uint::<U1, u8>::ONE.bit_width(),
      Uint::<U1, u8>::ONE.is_power_of_two()
    ),
    (1, true)
  );
}

/// Checks `&`, `|`, `^` and `!` of `$ty`, of the sign `$sign` and of `$bits` bits, whose values
/// `$from` creates from an `i128`, at each pair of `$pairs`: the bitwise `const fn`s give the bit
/// model's results, and so do the operators in their four forms of values and references, their
/// compound assignments with a value and a reference, and `!` on a value and a reference.
macro_rules! sweep_bitwise {
  ($sign:ident; $ty:ty, $bits:expr, $pairs:expr, $from:expr, $value:expr, $magnitude:expr) => {{
    let (bits, signed) = ($bits, is_signed!($sign));
    let from: fn(i128) -> $ty = $from;
    let read = |pattern: &[bool]| from(value_of(pattern, signed));
    for (a, b) in $pairs {
      let (x, y) = (from(a), from(b));
      let (p, q) = (pattern_of(a, bits), pattern_of(b, bits));
      let (and, or, xor) = (
        read(&model::and(&p, &q)),
        read(&model::or(&p, &q)),
        read(&model::xor(&p, &q)),
      );
      assert_eq!(
        [x.bitand(y), x.bitor(y), x.bitxor(y)],
        [and, or, xor],
        "{a} and {b} in {bits} bits"
      );
      assert_eq!([x & y, x & &y, &x & y, &x & &y], [and; 4], "{a} & {b}");
      assert_eq!([x | y, x | &y, &x | y, &x | &y], [or; 4], "{a} | {b}");
      assert_eq!([x ^ y, x ^ &y, &x ^ y, &x ^ &y], [xor; 4], "{a} ^ {b}");
      let mut targets = [x; 6];
      targets[0] &= y;
      targets[1] &= &y;
      targets[2] |= y;
      targets[3] |= &y;
      targets[4] ^= y;
      targets[5] ^= &y;
      assert_eq!(targets, [and, and, or, or, xor, xor], "{a} and {b}");
      let not = read(&model::not(&p));
      assert_eq!([x.not(), !x, !&x], [not; 3], "!{a} in {bits} bits");
    }
  }};
}

/// Checks the bitwise operations at the width `B`, `bits` bits, at most 8, in `u8` and `i8`,
/// `[u8; 1]`, `[u8; 2]` and `[u16; 1]` at the pairs of [`small_pairs`] for a storage that every
/// tier checks, and in `u16` and `i16`, `u128` and `i128`, `[u64; 1]` and `[u128; 1]` at those for
/// another.
#[allow(clippy::op_ref, reason = "the operators on references are under test")]
fn bitwise_of_small_widths<B>(bits: u32)
where
  B: FitsBits<u8> + FitsBits<i8> + FitsBits<u16> + FitsBits<i16> + FitsBits<u128> + FitsBits<i128>,
  B: FitsBits<[u8; 1]> + FitsBits<[u8; 2]> + FitsBits<[u16; 1]>,
  B: FitsBits<[u64; 1]> + FitsBits<[u128; 1]>,
{
  let primary = |bits, signed| small_pairs(bits, signed, true);
  let secondary = |bits, signed| small_pairs(bits, signed, false);
  sweep_storage!(sweep_bitwise, primary, native B, bits, u8, i8);
  sweep_storage!(sweep_bitwise, primary, array B, bits, [u8; 1]);
  sweep_storage!(sweep_bitwise, primary, array B, bits, [u8; 2]);
  sweep_storage!(sweep_bitwise, primary, array B, bits, [u16; 1]);
  sweep_storage!(sweep_bitwise, secondary, native B, bits, u16, i16);
  sweep_storage!(sweep_bitwise, secondary, native B, bits, u128, i128);
  sweep_storage!(sweep_bitwise, secondary, array B, bits, [u64; 1]);
  sweep_storage!(sweep_bitwise, secondary, array B, bits, [u128; 1]);
}

/// Checks the bitwise operations at the width `B`, `bits` bits, from 9 to 16, in `u16` and `i16`,
/// `[u8; 2]` and `[u16; 1]`, at the pairs of [`large_pairs`].
#[allow(clippy::op_ref, reason = "the operators on references are under test")]
fn bitwise_of_large_widths<B>(bits: u32)
where
  B: FitsBits<u16> + FitsBits<i16> + FitsBits<[u8; 2]> + FitsBits<[u16; 1]>,
{
  sweep_storage!(sweep_bitwise, large_pairs, native B, bits, u16, i16);
  sweep_storage!(sweep_bitwise, large_pairs, array B, bits, [u8; 2]);
  sweep_storage!(sweep_bitwise, large_pairs, array B, bits, [u16; 1]);
}

/// Every pair of values of each width up to 8 bits, or the pairs that the tier takes, and the
/// pairs that the tier takes of widths from 9 to 16 bits: `&`, `|`, `^` and `!` and the bitwise
/// `const fn`s against the bit model, in native integers and in arrays, at zero bits, at one
/// signed bit, and at partial and full widths. Miri checks 0, 1, 2, 7, 8, 9 and 16 bits.
#[test]
fn bitwise_operations_of_up_to_16_bits() {
  bitwise_of_small_widths::<U0>(0);
  bitwise_of_small_widths::<U1>(1);
  bitwise_of_small_widths::<U2>(2);
  bitwise_of_small_widths::<U7>(7);
  bitwise_of_small_widths::<U8>(8);
  bitwise_of_large_widths::<U9>(9);
  bitwise_of_large_widths::<U16>(16);
  if !tier().is_miri() {
    bitwise_of_small_widths::<U3>(3);
    bitwise_of_small_widths::<U4>(4);
    bitwise_of_small_widths::<U5>(5);
    bitwise_of_small_widths::<U6>(6);
    bitwise_of_large_widths::<U12>(12);
    bitwise_of_large_widths::<U15>(15);
  }
  if tier().is_full() {
    bitwise_of_large_widths::<U10>(10);
    bitwise_of_large_widths::<U11>(11);
    bitwise_of_large_widths::<U13>(13);
    bitwise_of_large_widths::<U14>(14);
  }
}

/// `&`, `|`, `^` and `!` on references need only the storage trait, `UintRepr` or `IntRepr`, not a
/// storage that holds the value inline, and the operators on values, with their compound
/// assignments, reach code bounded by `FixedInteger` alone: in a native integer and in an array,
/// they give the results of the bitwise `const fn`s.
#[test]
fn bitwise_operators_need_only_the_storage_trait() {
  fn unsigned<B: FitsBits<R>, R: UintRepr>(a: &Uint<B, R>, b: &Uint<B, R>) -> [Uint<B, R>; 4] {
    [a & b, a | b, a ^ b, !a]
  }

  fn signed<B: FitsBits<R>, R: IntRepr>(a: &Int<B, R>, b: &Int<B, R>) -> [Int<B, R>; 4] {
    [a & b, a | b, a ^ b, !a]
  }

  fn fixed<T: FixedInteger>(a: T, b: T) -> [T; 4] {
    let mut assigned = a.clone();
    assigned &= b.clone();
    assigned |= !b.clone();
    assigned ^= a.clone();
    [
      a.clone() & b.clone(),
      a.clone() | b.clone(),
      a ^ b,
      assigned,
    ]
  }

  // The generic function `$results` gives the results of the bitwise `const fn`s.
  macro_rules! check {
    ($results:ident, $a:expr, $b:expr) => {{
      let (a, b) = ($a, $b);
      let expected = [a.bitand(b), a.bitor(b), a.bitxor(b), a.not()];
      assert_eq!($results(&a, &b), expected, "{a} and {b}");
      let assigned = a.bitand(b).bitor(b.not()).bitxor(a);
      assert_eq!(
        fixed(a, b),
        [expected[0], expected[1], expected[2], assigned],
        "{a} and {b}"
      );
    }};
  }

  check!(unsigned, u7::new(0b101_1010), u7::new(0b011_0110));
  check!(unsigned, u256::from_limbs([5, 0, 0, 1 << 63]), u256::MAX);
  check!(signed, i7::MIN, i7::new(-6));
  check!(signed, i256::MIN, i256::ONE);
  check!(unsigned, Uint::<U0, u8>::ZERO, Uint::<U0, u8>::ZERO);
  check!(signed, Int::<U1, [u8; 1]>::MIN, Int::<U1, [u8; 1]>::ZERO);
}

/// The bitwise `const fn`s take their operands by value, as the methods of the operator traits
/// do, so that method syntax on a value finds them first, also where those traits are in scope:
/// constants built with them evaluate, in a native integer and in an array, and equal the results
/// of the operators. On a reference, method syntax reaches the methods of the traits.
mod with_the_operator_traits {
  use core::ops::{BitAnd, BitOr, BitXor, Not};

  use typenum::U24;

  use crate::{Int, i7, u7};

  #[test]
  fn bitwise_const_fns_come_first() {
    type Bytes = Int<U24, [u8; 3]>;
    const NATIVE: [u7; 4] = [
      u7::MAX.bitand(u7::new(5)),
      u7::new(64).bitor(u7::new(5)),
      u7::MAX.bitxor(u7::new(5)),
      u7::new(5).not(),
    ];
    const SIGNED: [i7; 3] = [
      i7::MIN.bitor(i7::new(5)),
      i7::new(-1).bitxor(i7::MIN),
      i7::new(-6).not(),
    ];
    const ARRAY: [Bytes; 4] = [
      Bytes::MIN.bitand(Bytes::MAX),
      Bytes::MIN.bitor(Bytes::MAX),
      Bytes::MIN.bitxor(Bytes::MIN),
      Bytes::ZERO.not(),
    ];

    assert_eq!(
      NATIVE,
      [
        u7::MAX & u7::new(5),
        u7::new(64) | u7::new(5),
        u7::MAX ^ u7::new(5),
        !u7::new(5)
      ]
    );
    assert_eq!(
      NATIVE,
      [u7::new(5), u7::new(69), u7::new(122), u7::new(122)]
    );
    assert_eq!(
      SIGNED,
      [i7::MIN | i7::new(5), i7::new(-1) ^ i7::MIN, !i7::new(-6)]
    );
    assert_eq!(SIGNED, [i7::new(-59), i7::MAX, i7::new(5)]);
    assert_eq!(
      ARRAY,
      [
        Bytes::MIN & Bytes::MAX,
        Bytes::MIN | Bytes::MAX,
        Bytes::MIN ^ Bytes::MIN,
        !Bytes::ZERO
      ]
    );
    assert_eq!(ARRAY, [Bytes::ZERO, -Bytes::ONE, Bytes::ZERO, -Bytes::ONE]);

    // On references, the methods of the traits.
    let (five, max) = (u7::new(5), u7::MAX);
    assert_eq!((&max).bitand(&five), five);
    assert_eq!((&five).bitor(&u7::new(64)), u7::new(69));
    assert_eq!((&max).bitxor(&five), u7::new(122));
    assert_eq!((&five).not(), u7::new(122));
  }
}
