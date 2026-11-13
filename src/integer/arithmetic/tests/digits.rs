//! The digits of `Uint` in a base: the constructors from digits in bytes and in `u64`, and the
//! digits that a value writes, against a model on `u128` up to 16 bits and `num-bigint` above it,
//! read back through every reader; digits outside the base or the width; the literals of the
//! documented examples and tests of the reference crates; the panics of a radix or a base outside
//! its range; constant contexts; zero and zero bits; and code generic over the trait and the
//! width.

use num_bigint::BigUint;
use typenum::{U3, U4, U5, U6, U12, U512, U65536};

use super::*;
use crate::{IntegerBytes, u0};

/// The radixes at which the digits of a value of up to 16 bits are checked: 2, 3, 10, 16, 36, 58,
/// 255 and 256 in an optimized build, 2, 10 and 256 in an unoptimized one, and 10 and 256 under
/// Miri.
fn small_radixes() -> &'static [u32] {
  match tier() {
    Tier::Full => &[2, 3, 10, 16, 36, 58, 255, 256],
    Tier::Debug => &[2, 10, 256],
    Tier::Miri => &[10, 256],
  }
}

/// The bases at which the digits of a value of up to 16 bits are checked: 7, `2^32 + 1` and
/// `u64::MAX` in an optimized build, 7 and `u64::MAX` in an unoptimized one, and 7 under Miri.
fn small_bases() -> &'static [u64] {
  match tier() {
    Tier::Full => &[7, (1 << 32) + 1, u64::MAX],
    Tier::Debug => &[7, u64::MAX],
    Tier::Miri => &[7],
  }
}

/// The digits of `value` in `base`, least significant first, without the zeros above the highest
/// one: the model on `u128`, which shares no code with the crate.
fn model_digits(mut value: u128, base: u64) -> Vec<u64> {
  let mut digits = Vec::new();
  while value > 0 {
    digits.push((value % u128::from(base)) as u64);
    value /= u128::from(base);
  }
  digits
}

/// The digits of `digits` as bytes. Requires each digit to fit a byte.
fn as_bytes(digits: &[u64]) -> Vec<u8> {
  digits
    .iter()
    .map(|&digit| u8::try_from(digit).expect("a byte"))
    .collect()
}

/// Checks the digits of `$x`, the value `$a` of the unsigned type `$ty`: in each radix of
/// [`small_radixes`] the model's digits read back through the inherent `const fn` and through
/// `UnsignedDigits`, and with the `std` or the `alloc` feature `to_radix_le` and `to_radix_be`
/// give them, with the one digit 0 for zero; in each base of [`small_bases`] `to_base_le` gives
/// the model's digits, through both, which `checked_from_base_le` and `checked_from_base_be` read
/// back, and with a tier `to_base_be` gives them most significant first.
macro_rules! check_small_digits {
  ($ty:ty, $x:expr, $a:expr) => {{
    let (x, a): ($ty, u128) = ($x, $a);
    for &radix in small_radixes() {
      let le = as_bytes(&model_digits(a, radix.into()));
      let be: Vec<u8> = le.iter().rev().copied().collect();
      let what = format!("{a} in radix {radix}");
      let read = [
        <$ty>::checked_from_radix_le(&le, radix),
        <$ty as UnsignedDigits>::checked_from_radix_le(&le, radix),
        <$ty>::checked_from_radix_be(&be, radix),
        <$ty as UnsignedDigits>::checked_from_radix_be(&be, radix),
      ];
      assert_eq!(read, [Some(x); 4], "{what}");
    }
    for &base in small_bases() {
      let le = model_digits(a, base);
      let what = format!("{a} in base {base}");
      assert_eq!(x.to_base_le(base).collect::<Vec<u64>>(), le, "{what}");
      assert_eq!(
        UnsignedDigits::to_base_le(&x, base).collect::<Vec<u64>>(),
        le,
        "{what}"
      );
      let read = [
        <$ty>::checked_from_base_le(le.iter().copied(), base),
        <$ty as UnsignedDigits>::checked_from_base_le(le.iter().copied(), base),
        <$ty>::checked_from_base_be(le.iter().rev().copied(), base),
        <$ty as UnsignedDigits>::checked_from_base_be(le.iter().rev().copied(), base),
      ];
      assert_eq!(read, [Some(x); 4], "{what}");
    }
  }};
}

/// The sweep of `sweep_storage!` for the digits, which `Uint` alone has: [`check_small_digits!`]
/// at each value of the unsigned storage, and nothing for the signed one.
macro_rules! sweep_digits {
  (unsigned; $ty:ty, $bits:expr, $values:expr, $from:expr, $value:expr, $magnitude:expr) => {{
    let from: fn(i128) -> $ty = $from;
    for a in $values {
      check_small_digits!($ty, from(a), a as u128);
    }
  }};
  (signed; $($rest:tt)*) => {{}};
}

/// Checks the digits at the width `B`, `bits` bits, at most 8, in `u8` and `[u8; 1]` at the values
/// of [`small_values`] for a storage that every tier checks, and in `u16`, `u128`, `[u8; 2]`,
/// `[u16; 1]`, `[u64; 1]` and `[u128; 1]` at those for another, which Miri does not check.
#[inline(never)]
fn digits_of_small_widths<B>(bits: u32)
where
  B: FitsBits<u8> + FitsBits<i8> + FitsBits<u16> + FitsBits<i16> + FitsBits<u128> + FitsBits<i128>,
  B: FitsBits<[u8; 1]> + FitsBits<[u8; 2]> + FitsBits<[u16; 1]>,
  B: FitsBits<[u64; 1]> + FitsBits<[u128; 1]>,
{
  let primary = |bits, signed| small_values(bits, signed, true);
  let secondary = |bits, signed| small_values(bits, signed, false);
  sweep_storage!(sweep_digits, primary, native B, bits, u8, i8);
  sweep_storage!(sweep_digits, primary, array B, bits, [u8; 1]);
  sweep_storage!(sweep_digits, secondary, native B, bits, u16, i16);
  sweep_storage!(sweep_digits, secondary, native B, bits, u128, i128);
  sweep_storage!(sweep_digits, secondary, array B, bits, [u8; 2]);
  sweep_storage!(sweep_digits, secondary, array B, bits, [u16; 1]);
  sweep_storage!(sweep_digits, secondary, array B, bits, [u64; 1]);
  sweep_storage!(sweep_digits, secondary, array B, bits, [u128; 1]);
}

/// Checks the digits at the width `B`, `bits` bits, from 9 to 16, in `[u8; 2]`, `u16` and
/// `[u16; 1]`, at the values of [`large_values`]. Under Miri, in `[u8; 2]`.
#[inline(never)]
fn digits_of_large_widths<B>(bits: u32)
where
  B: FitsBits<u16> + FitsBits<i16> + FitsBits<[u8; 2]> + FitsBits<[u16; 1]>,
{
  let values = |bits, signed| large_values(bits, signed);
  sweep_storage!(sweep_digits, values, array B, bits, [u8; 2]);
  if !tier().is_miri() {
    sweep_storage!(sweep_digits, values, native B, bits, u16, i16);
    sweep_storage!(sweep_digits, values, array B, bits, [u16; 1]);
  }
}

/// The digits of every value of each width up to 8 bits, or the values that the tier takes, in
/// native integers and arrays, against the model on `u128`, read back through every reader. The
/// radix 256 on `u8`, whose native integer does not hold the radix, is in every tier.
#[test]
fn digits_of_up_to_8_bits() {
  digits_of_small_widths::<U0>(0);
  digits_of_small_widths::<U1>(1);
  digits_of_small_widths::<U7>(7);
  digits_of_small_widths::<U8>(8);
  if !tier().is_miri() {
    digits_of_small_widths::<U2>(2);
    digits_of_small_widths::<U3>(3);
    digits_of_small_widths::<U4>(4);
    digits_of_small_widths::<U5>(5);
    digits_of_small_widths::<U6>(6);
  }
}

/// The digits of the values that the tier takes of widths from 9 to 16 bits, against the model on
/// `u128`: 9, 12 and 16 bits, or 16 bits under Miri.
#[test]
fn digits_of_9_to_16_bits() {
  digits_of_large_widths::<U16>(16);
  if !tier().is_miri() {
    digits_of_large_widths::<U9>(9);
    digits_of_large_widths::<U12>(12);
  }
}

/// The value of the digit sequence `digits` in `base`, the most significant first if `big_endian`,
/// as `T`: `None` for a digit that is not below `base`, and for a value that `T` does not hold, as
/// its reader of a slice of bytes finds.
fn expected_of<T: IntegerBytes>(digits: &[u64], big_endian: bool, base: u64) -> Option<T> {
  let value = bigint::digits_value(digits, big_endian, base)?;
  T::checked_from_le_slice(&value.to_bytes_le().1)
}

/// Checks the readers of `$ty` at every sequence of the digits of [`rejection_digits`] of up to
/// `bigint::digit_sequence_len()` digits, in both orders, from slices and from iterators, against
/// [`expected_of`].
macro_rules! check_rejections {
  ($ty:ty) => {{
    for radix in [2u32, 10, 256] {
      for digits in bigint::digit_sequences(&rejection_digits(radix), bigint::digit_sequence_len())
      {
        let what = format!("{digits:?} in base {radix}");
        let base = u64::from(radix);
        let iterated = [
          <$ty>::checked_from_base_le(digits.iter().copied(), base),
          <$ty>::checked_from_base_be(digits.iter().copied(), base),
        ];
        let expected = [
          expected_of::<$ty>(&digits, false, base),
          expected_of::<$ty>(&digits, true, base),
        ];
        assert_eq!(iterated, expected, "{what}");
        // A slice holds bytes, which hold no 256.
        if digits.iter().all(|&digit| digit <= 255) {
          let bytes = as_bytes(&digits);
          let read = [
            <$ty>::checked_from_radix_le(&bytes, radix),
            <$ty>::checked_from_radix_be(&bytes, radix),
          ];
          assert_eq!(read, expected, "{what}");
        }
      }
    }
  }};
}

/// The digits of the sequences that the rejections take in `radix`: 0, 1, `radix - 1`, `radix` and
/// 255 in the radixes 2 and 10, and 0, 1, 255 and 256 in the radix 256, whose 256 only an iterator
/// holds.
fn rejection_digits(radix: u32) -> Vec<u64> {
  let radix = u64::from(radix);
  let mut digits = vec![0, 1, radix - 1, radix, 255];
  digits.sort_unstable();
  digits.dedup();
  digits
}

/// Checks the rejections at the width `B` in `u16` and in `[u8; 2]`.
#[inline(never)]
fn rejections_at<B: FitsBits<u16> + FitsBits<[u8; 2]>>() {
  check_rejections!(Uint<B, u16>);
  check_rejections!(Uint<B, [u8; 2]>);
}

/// Every short digit sequence, in both orders, from a slice and from an iterator, at 0, 1, 7, 8, 9
/// and 16 bits in a native integer and an array, against the model: `None` for a digit outside
/// the base and for a value outside the width, and the value for zeros above it and at the limit.
#[test]
fn digits_outside_the_base_or_the_width_are_rejected() {
  rejections_at::<U0>();
  rejections_at::<U1>();
  rejections_at::<U7>();
  rejections_at::<U8>();
  rejections_at::<U9>();
  rejections_at::<U16>();
}

/// The value of the pattern `p`, unsigned, as a big integer.
fn big_of(p: &[bool]) -> BigUint {
  let bytes: Vec<u8> = p
    .chunks(8)
    .map(|bits| {
      bits
        .iter()
        .rev()
        .fold(0, |byte, &bit| byte << 1 | u8::from(bit))
    })
    .collect();
  BigUint::from_bytes_le(&bytes)
}

/// The digits of `value` in `base`, least significant first, without the zeros above the highest
/// one, by repeated division of a big integer.
fn big_digits(value: &BigUint, base: u64) -> Vec<u64> {
  bigint::digits_of(&BigInt::from(value.clone()), base)
}

/// `digits`, least significant first, extended with zeros to `len` digits and then by a top digit
/// 1: a value of more than `len` digits, which overflows a width whose largest value has `len`.
fn one_more_digit<T: Copy + From<u8>>(digits: &[T], len: usize) -> Vec<T> {
  let mut longer = digits.to_vec();
  longer.resize(len, T::from(0));
  longer.push(T::from(1));
  longer
}

/// Checks the digits of `T`, of `bits` bits, at each pattern of `patterns`, against `num-bigint`:
/// in each radix of `radixes` its `to_radix_le` reads back in both orders, and with a tier is what
/// `to_radix_le` and `to_radix_be` give; in each base of `bases` the digits of repeated division
/// are what `to_base_le` gives, and with a tier `to_base_be`, and read back in both orders; and one
/// more digit than the largest value has overflows, from a slice and from an iterator.
#[inline(never)]
fn wide_digits_against_big_integers<T>(
  bits: u32,
  patterns: &[Vec<bool>],
  radixes: &[u32],
  bases: &[u64],
) where
  T: Patterned + UnsignedDigits,
{
  let max = (BigUint::from(1u8) << bits) - 1u8;
  for p in patterns {
    let (x, big) = (T::from_pattern(p), big_of(p));
    for &radix in radixes {
      let le = big.to_radix_le(radix);
      let be: Vec<u8> = le.iter().rev().copied().collect();
      let what = format!("{big} in radix {radix}");
      assert_eq!(T::checked_from_radix_le(&le, radix), Some(x), "{what}");
      assert_eq!(T::checked_from_radix_be(&be, radix), Some(x), "{what}");
      let over = one_more_digit(&le, max.to_radix_le(radix).len());
      assert_eq!(T::checked_from_radix_le(&over, radix), None, "{what}");
    }
    for &base in bases {
      let le = big_digits(&big, base);
      let what = format!("{big} in base {base}");
      assert_eq!(x.to_base_le(base).collect::<Vec<u64>>(), le, "{what}");
      assert_eq!(
        T::checked_from_base_le(le.iter().copied(), base),
        Some(x),
        "{what}"
      );
      assert_eq!(
        T::checked_from_base_be(le.iter().rev().copied(), base),
        Some(x),
        "{what}"
      );
      let over = one_more_digit(&le, big_digits(&max, base).len());
      assert_eq!(T::checked_from_base_le(over, base), None, "{what}");
    }
  }
}

/// The digits of wide values against `num-bigint`, at the patterns of [`wide_patterns`]: 65 bits
/// in `[u64; 2]`, 129 in `[u8; 17]` and 200 in `[u32; 7]` and `[u128; 2]` in the radixes 2, 10, 58
/// and 256 and the bases 10, `2^32 + 1` and `u64::MAX`; in an optimized build also the radixes 3,
/// 16, 36 and 255 and the base `10^19`, and 256, 1024 and 4096 bits in `[u64; N]`, and 4096 bits
/// in `[u8; 512]` in the radixes 10 and 256 alone.
#[test]
#[cfg_attr(miri, ignore = "num-bigint is slow under Miri")]
fn wide_digits_match_big_integers() {
  let mut radixes = vec![2, 10, 58, 256];
  let mut bases = vec![10, (1 << 32) + 1, u64::MAX];
  if tier().is_full() {
    radixes.extend([3, 16, 36, 255]);
    bases.push(10_000_000_000_000_000_000);
  }
  let (radixes, bases) = (&radixes[..], &bases[..]);
  wide_digits_against_big_integers::<Uint<U65, [u64; 2]>>(
    65,
    &wide_patterns::<2>(65),
    radixes,
    bases,
  );
  wide_digits_against_big_integers::<Uint<U129, [u8; 17]>>(
    129,
    &wide_patterns::<3>(129),
    radixes,
    bases,
  );
  wide_digits_against_big_integers::<Uint<U200, [u32; 7]>>(
    200,
    &wide_patterns::<4>(200),
    radixes,
    bases,
  );
  wide_digits_against_big_integers::<Uint<U200, [u128; 2]>>(
    200,
    &wide_patterns::<4>(200),
    radixes,
    bases,
  );
  if tier().is_full() {
    wide_digits_against_big_integers::<Uint<U256, [u64; 4]>>(
      256,
      &wide_patterns::<4>(256),
      radixes,
      bases,
    );
    wide_digits_against_big_integers::<Uint<U1024, [u64; 16]>>(
      1024,
      &wide_patterns::<16>(1024),
      radixes,
      bases,
    );
    wide_digits_against_big_integers::<Uint<U4096, [u64; 64]>>(
      4096,
      &wide_patterns::<64>(4096),
      radixes,
      bases,
    );
    wide_digits_against_big_integers::<Uint<U4096, [u8; 512]>>(
      4096,
      &wide_patterns::<64>(4096),
      &[10, 256],
      &[],
    );
  }
}

/// The bytes of `x`, least significant first, without the zero bytes above the highest one that is
/// not zero.
fn trimmed_bytes<T: IntegerBytes>(x: &T) -> Vec<u8> {
  let mut bytes = vec![0; T::BYTES];
  x.copy_le_bytes_to(&mut bytes);
  while bytes.last() == Some(&0) {
    bytes.pop();
  }
  bytes
}

/// Checks that the digits of `T`, of `bits` bits, at each pattern of [`wide_patterns`] in `K`
/// words, read back: those of the radixes 2, 10 and 256 as bytes and of the bases 10, `2^32 + 1`
/// and `u64::MAX`, in both orders; the digits of the base `2^32` are the 32-bit words of the
/// bytes of the value; and with a tier, `to_radix_be(256)` is the bytes of a value that is not
/// zero, most significant first, without the zeros above it.
#[inline(never)]
fn round_trip_at<T: Patterned + UnsignedDigits + IntegerBytes, const K: usize>(bits: u32) {
  for p in wide_patterns::<K>(bits) {
    let x = T::from_pattern(&p);
    let what = model::text(&p);
    for radix in [2, 10, 256] {
      let le = as_bytes(&x.to_base_le(radix.into()).collect::<Vec<u64>>());
      let be: Vec<u8> = le.iter().rev().copied().collect();
      assert_eq!(T::checked_from_radix_le(&le, radix), Some(x), "{what}");
      assert_eq!(T::checked_from_radix_be(&be, radix), Some(x), "{what}");
    }
    for base in [10, (1 << 32) + 1, u64::MAX] {
      let le: Vec<u64> = x.to_base_le(base).collect();
      assert_eq!(
        T::checked_from_base_le(le.iter().copied(), base),
        Some(x),
        "{what}"
      );
      assert_eq!(
        T::checked_from_base_be(le.iter().rev().copied(), base),
        Some(x),
        "{what}"
      );
    }
    let bytes = trimmed_bytes(&x);
    let words: Vec<u64> = bytes
      .chunks(4)
      .map(|word| {
        word
          .iter()
          .rev()
          .fold(0, |sum, &byte| sum << 8 | u64::from(byte))
      })
      .collect();
    assert_eq!(x.to_base_le(1 << 32).collect::<Vec<u64>>(), words, "{what}");
  }
}

/// The value of 65536 bits with every bit set, in the default storage.
type Widest = Uint<U65536>;

/// The decimal digits of the widest value read back. One operation of its own, which only an
/// optimized build takes.
#[inline(never)]
fn widest_decimal_digits_read_back() {
  let digits = as_bytes(&Widest::MAX.to_base_le(10).collect::<Vec<u64>>());
  assert_eq!(digits.len(), 19729);
  assert_eq!(
    Widest::checked_from_radix_le(&digits, 10),
    Some(Widest::MAX)
  );
}

/// The digits of the radix 256 of the widest value read back, most significant first.
#[inline(never)]
fn widest_bytes_read_back() {
  let mut digits = as_bytes(&Widest::MAX.to_base_le(256).collect::<Vec<u64>>());
  digits.reverse();
  assert_eq!(digits, [0xff; 8192]);
  assert_eq!(
    Widest::checked_from_radix_be(&digits, 256),
    Some(Widest::MAX)
  );
}

/// The digits of 65 and 129 bits in `u8` and `u64` limbs read back, in every tier, and the words
/// and the bytes of a value are its digits in the bases `2^32` and 256; in an optimized build also
/// the digits of the largest value of 65536 bits, and its text.
#[test]
fn wide_digits_round_trip() {
  round_trip_at::<Uint<U65, [u8; 9]>, 2>(65);
  round_trip_at::<Uint<U65, [u64; 2]>, 2>(65);
  round_trip_at::<Uint<U129, [u8; 17]>, 3>(129);
  round_trip_at::<Uint<U129, [u64; 3]>, 3>(129);
  if tier().is_full() {
    widest_decimal_digits_read_back();
    widest_bytes_read_back();
  }
}

/// The documented examples of bnum, at 512 bits, and the tests of ruint, as literals: the digits
/// of the radixes 11, 18, 120, 250 and 36, and of the bases 10 and `10^19`, read and written, and
/// the overflows of zero bits and one bit.
#[test]
fn the_digits_of_the_reference_crates() {
  type Wide = Uint<U512>;
  let from_decimal = |text: &str| Wide::from_str_radix(text, 10).expect("512 bits");
  assert_eq!(
    Wide::checked_from_radix_be(&[4, 3, 2, 1], 11),
    Some(from_decimal("5710"))
  );
  assert_eq!(
    Wide::checked_from_radix_le(&[5, 6, 7, 8], 18),
    Some(from_decimal("49037"))
  );
  let digits = [3, 55, 60, 100, 5, 0, 5, 88];
  let n = Wide::checked_from_radix_be(&digits, 120).expect("512 bits");
  let mut written = as_bytes(&n.to_base_le(120).collect::<Vec<u64>>());
  written.reverse();
  assert_eq!(written, digits);
  let digits = [1, 67, 88, 200, 55, 68, 87, 120, 178];
  let n = Wide::checked_from_radix_le(&digits, 250).expect("512 bits");
  assert_eq!(as_bytes(&n.to_base_le(250).collect::<Vec<u64>>()), digits);
  let letters = "abcdefghijklmnopqrstuvwxyz";
  let n = Wide::from_str_radix(letters, 36).expect("512 bits");
  let mut written: Vec<char> = n
    .to_base_le(36)
    .map(|digit| char::from_digit(digit as u32, 36).expect("a digit"))
    .collect();
  written.reverse();
  assert_eq!(written.into_iter().collect::<String>(), letters);

  let small = Uint::<U64>::new(123_456_789);
  assert_eq!(
    small.to_base_le(10).collect::<Vec<u64>>(),
    [9, 8, 7, 6, 5, 4, 3, 2, 1]
  );
  assert_eq!(
    Uint::<U64>::checked_from_base_le([9, 8, 7, 6, 5, 4, 3, 2, 1], 10),
    Some(small)
  );
  assert_eq!(
    Uint::<U64>::checked_from_base_be([1, 2, 3, 4, 5, 6, 7, 8, 9], 10),
    Some(small)
  );
  // 90630363884335538722706632492458228784305343302099024356772372330524102404852.
  let n = Uint::<U256, [u64; 4]>::from_limbs([
    0xa8ec_9234_4438_aaf4,
    0x9819_ebdb_d1fa_aab1,
    0x573b_1a70_64c1_9c1a,
    0xc85e_f7d7_9691_fe79,
  ]);
  let chunks = [
    2_372_330_524_102_404_852,
    534_330_209_902_435_677,
    7_066_324_924_582_287_843,
    630_363_884_335_538_722,
    9,
  ];
  let base = 10_000_000_000_000_000_000;
  assert_eq!(n.to_base_le(base).collect::<Vec<u64>>(), chunks);
  assert_eq!(
    Uint::<U256, [u64; 4]>::checked_from_base_le(chunks, base),
    Some(n)
  );
  let mut reversed = chunks;
  reversed.reverse();
  assert_eq!(
    Uint::<U256, [u64; 4]>::checked_from_base_be(reversed, base),
    Some(n)
  );
  assert_eq!(u0::checked_from_base_be([], 10), Some(u0::ZERO));
  assert_eq!(u0::checked_from_base_be([0], 10), Some(u0::ZERO));
  assert_eq!(u0::checked_from_base_be([1], 10), None);
  assert_eq!(Uint::<U1>::checked_from_base_be([1, 0, 0], 10), None);
}

/// A radix outside 2 to 256 and a base below 2 panic in every build, with the name of the method,
/// and report the caller, through the inherent methods and through `UnsignedDigits`, on a native
/// integer and an array, and with a tier also on a box; the iterators panic when they are called,
/// not at their first digit. The panics of `to_str_radix` are those of the text.
#[test]
fn a_radix_or_a_base_outside_its_range_panics() {
  type N = u7;
  type A = Uint<U200, [u32; 7]>;
  let be = "checked_from_radix_be: radix must lie in the range `[2, 256]`".to_string();
  let le = "checked_from_radix_le: radix must lie in the range `[2, 256]`".to_string();
  for r in [0, 1, 257, u32::MAX] {
    panics_here!(N::checked_from_radix_be(&[1], r), be);
    panics_here!(N::checked_from_radix_le(&[], r), le);
    panics_here!(A::checked_from_radix_be(&[], r), be);
    panics_here!(A::checked_from_radix_le(&[1], r), le);
    panics_here!(<N as UnsignedDigits>::checked_from_radix_be(&[], r), be);
    panics_here!(<A as UnsignedDigits>::checked_from_radix_le(&[], r), le);
  }
  let from_be = "checked_from_base_be: base must be at least 2".to_string();
  let from_le = "checked_from_base_le: base must be at least 2".to_string();
  let to_le = "to_base_le: base must be at least 2".to_string();
  let (n, a) = (N::new(5), A::MAX);
  for b in [0, 1] {
    panics_here!(N::checked_from_base_be([], b), from_be);
    panics_here!(A::checked_from_base_le([1], b), from_le);
    panics_here!(<N as UnsignedDigits>::checked_from_base_le([], b), from_le);
    panics_here!(<A as UnsignedDigits>::checked_from_base_be([], b), from_be);
    panics_here!(n.to_base_le(b), to_le);
    panics_here!(UnsignedDigits::to_base_le(&a, b), to_le);
  }
}

/// The constructors from digits in bytes in constant contexts, in a native integer, an array and
/// zero bits, with a `None`.
#[test]
fn digits_in_constant_contexts() {
  const NATIVE: Option<u7> = u7::checked_from_radix_be(&[1, 2, 7], 10);
  const ARRAY: Option<Uint<U200, [u64; 4]>> =
    Uint::<U200, [u64; 4]>::checked_from_radix_le(&[0xff; 25], 256);
  const ZERO: Option<u0> = u0::checked_from_radix_le(&[0, 0], 2);
  const NONE: Option<u7> = u7::checked_from_radix_be(&[1, 2, 8], 10);
  assert_eq!(NATIVE, Some(u7::new(127)));
  assert_eq!(ARRAY, Some(Uint::<U200, [u64; 4]>::MAX));
  assert_eq!(ZERO, Some(u0::ZERO));
  assert_eq!(NONE, None);
}

/// Checks the digits of zero in `T`: no digits in any base, which read back, as do no digits and
/// zeros in a radix; with a tier, the one digit 0 in a radix and no digits most significant first.
/// At zero bits every other digit is outside the width.
#[inline(never)]
fn zero_at<T: UnsignedDigits + Debug + PartialEq>(zero: &T, bits: u32) {
  for base in [2, 10, u64::MAX] {
    assert_eq!(zero.to_base_le(base).count(), 0);
    assert_eq!(T::checked_from_base_le([], base).as_ref(), Some(zero));
    assert_eq!(T::checked_from_base_be([0, 0], base).as_ref(), Some(zero));
  }
  for radix in [2, 10, 256] {
    assert_eq!(T::checked_from_radix_be(&[], radix).as_ref(), Some(zero));
    assert_eq!(
      T::checked_from_radix_le(&[0, 0, 0], radix).as_ref(),
      Some(zero)
    );
    assert_eq!(T::checked_from_radix_le(&[1], radix).is_none(), bits == 0);
  }
}

/// Zero at 0, 8 and 200 bits, in native integers and arrays: no digits of a base and the one
/// digit 0 of a radix, and every reader reads it from no digits and from zeros.
#[test]
fn digits_of_zero_and_of_zero_bits() {
  zero_at(&u0::ZERO, 0);
  zero_at(&Uint::<U0, u8>::ZERO, 0);
  zero_at(&Uint::<U8>::ZERO, 8);
  zero_at(&Uint::<U8, [u8; 1]>::ZERO, 8);
  zero_at(&Uint::<U200>::ZERO, 200);
  zero_at(&Uint::<U200, [u8; 25]>::ZERO, 200);
}

/// The digits of a value, least significant first, through a bound on the trait.
fn digits_through<T: UnsignedDigits>(value: &T, base: u64) -> Vec<u64> {
  value.to_base_le(base).collect()
}

/// The digits of a local value through the inherent method: the iterator holds what is left of
/// the value, not a borrow of it.
fn digits_of_a_local(value: u64) -> impl Iterator<Item = u64> {
  let local = Uint::<U64>::new(value);
  local.to_base_le(10)
}

/// The digits of a local value through the trait.
fn digits_of_a_local_through<T: UnsignedDigits>(value: T) -> impl Iterator<Item = u64> {
  let local = value;
  local.to_base_le(10)
}

/// The digits of `value` and back, on `Uint<B>`, which has every method under `B: BitWidth`.
fn again_at_width<B: BitWidth>(value: &Uint<B>) -> Option<Uint<B>> {
  let digits: Vec<u64> = value.to_base_le(7).collect();
  let bytes = as_bytes(&value.to_base_le(256).collect::<Vec<u64>>());
  let read = <Uint<B> as UnsignedDigits>::checked_from_radix_le(&bytes, 256);
  assert_eq!(read.as_ref(), Some(value));
  Uint::<B>::checked_from_base_le(digits, 7)
}

/// The trait in generic code on a native integer and an array, and with a tier on a box; iterators
/// of local values, returned from a function through the inherent method and through the trait;
/// the iterator, once it returns `None`, returns it again; and the methods on `Uint<B>` under
/// `B: BitWidth`.
#[test]
fn digit_trait_surface() {
  assert_eq!(digits_through(&u7::new(100), 7), [2, 0, 2]);
  let wide = Uint::<U200, [u8; 25]>::checked_from_base_le([1], 10).expect("200 bits");
  assert_eq!(digits_through(&wide, 10), [1]);
  assert_eq!(digits_of_a_local(120).collect::<Vec<u64>>(), [0, 2, 1]);
  assert_eq!(
    digits_of_a_local_through(u7::new(120)).collect::<Vec<u64>>(),
    [0, 2, 1]
  );
  let mut digits = u7::new(5).to_base_le(10);
  assert_eq!(
    (digits.next(), digits.next(), digits.next()),
    (Some(5), None, None)
  );
  assert_eq!(again_at_width(&u7::new(100)), Some(u7::new(100)));
  assert_eq!(again_at_width(&Uint::<U200>::MAX), Some(Uint::<U200>::MAX));
}
