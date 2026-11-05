//! The bytes and the byte order of `Uint` and `Int`: the arrays of bytes in each order and the
//! conversions back from them, checked and at whole bytes, the checked and wrapping conversions
//! from slices, the copies to a buffer and the byte at an index, against the bit model at every
//! width up to 24 bits and at wider ones; the reversal of the bytes and the conversions of the byte
//! order at widths of whole bytes; the panics of a short buffer and of a byte index; constant
//! contexts; and zero bits and one signed bit.

use typenum::{
  U3, U4, U5, U6, U10, U11, U12, U13, U14, U15, U17, U18, U19, U20, U21, U22, U23, U201, U8192,
  U16384,
};

use super::*;
use crate::{LimbArray, WholeBytes, i0, i1, u0};

/// Checks the arrays of bytes of `$x`, a value of `$ty` whose pattern is `$p`, of the width of
/// `$ty`, read in two's complement if `$signed`, through the inherent `const fn`s, against the bit
/// model: `to_le_bytes`, `to_be_bytes` and `to_ne_bytes`, whose order `cfg!(target_endian)`
/// decides; the checked conversions back from them; and the checked conversions from the array of
/// the value with its last byte replaced by `0x00`, `0xff`, `0x01` or `0x80`, in both orders, of
/// which `$read` reads the patterns that hold a value of the width.
macro_rules! check_arrays {
  ($ty:ty, $x:expr, $p:expr, $signed:expr, $read:expr) => {{
    let (x, p, signed): ($ty, &[bool], bool) = ($x, $p, $signed);
    let read = $read;
    let width = model::width(p);
    let size = <$ty>::BYTES;
    let le = model::le_bytes(p, size, signed);
    let be: Vec<u8> = le.iter().rev().copied().collect();
    let ne = if cfg!(target_endian = "big") {
      be.clone()
    } else {
      le.clone()
    };
    let arrays = (x.to_le_bytes(), x.to_be_bytes(), x.to_ne_bytes());
    assert_eq!(
      (arrays.0.to_vec(), arrays.1.to_vec(), arrays.2.to_vec()),
      (le, be, ne),
      "{}",
      model::text(p)
    );
    assert_eq!(
      (
        <$ty>::checked_from_le_bytes(arrays.0),
        <$ty>::checked_from_be_bytes(arrays.1),
        <$ty>::checked_from_ne_bytes(arrays.2),
      ),
      (Some(x), Some(x), Some(x)),
      "{}",
      model::text(p)
    );
    if size > 0 {
      for high in [0x00, 0xff, 0x01, 0x80] {
        let mut little = arrays.0;
        little[size - 1] = high;
        let mut big = little;
        big.reverse();
        let expected = model::fit(&model::of_le_bytes(&little), width, signed).map(|q| read(&q));
        assert_eq!(
          (
            <$ty>::checked_from_le_bytes(little),
            <$ty>::checked_from_be_bytes(big)
          ),
          (expected, expected),
          "{little:x?} at {width} bits"
        );
      }
    }
  }};
}

/// Checks the slices, the copies and the bytes of `$x`, a value of `$ty` whose pattern is `$p`,
/// of the width of `$ty`, read in two's complement if `$signed`, through the inherent `const fn`s,
/// against the bit model: the checked and wrapping conversions from each slice of
/// `bigint::byte_slices`, least significant byte first and reversed, whose results `$read` reads,
/// the wrapping one returning the value of the checked one wherever that returns one; the copies
/// to buffers of the bytes of the value, two more and one less, whose other bytes stay as they
/// were; and the byte at each index.
macro_rules! check_slices {
  ($ty:ty, $x:expr, $p:expr, $signed:expr, $read:expr) => {{
    let (x, p, signed): ($ty, &[bool], bool) = ($x, $p, $signed);
    let read = $read;
    let width = model::width(p);
    let size = <$ty>::BYTES;
    for slice in bigint::byte_slices(p, signed) {
      let q = model::of_le_bytes(&slice);
      let checked = model::fit(&q, width, signed).map(|r| read(&r));
      let wrapped = read(&model::wrap(&q, width, signed));
      let reversed: Vec<u8> = slice.iter().rev().copied().collect();
      let reads = [
        (
          <$ty>::checked_from_le_slice(&slice),
          <$ty>::wrapping_from_le_slice(&slice),
        ),
        (
          <$ty>::checked_from_be_slice(&reversed),
          <$ty>::wrapping_from_be_slice(&reversed),
        ),
      ];
      assert_eq!(reads, [(checked, wrapped); 2], "{slice:x?} at {width} bits");
      for (checked, wrapped) in reads {
        if let Some(value) = checked {
          assert_eq!(wrapped, value, "{slice:x?} at {width} bits");
        }
      }
    }
    let le = model::le_bytes(p, size, signed);
    let be: Vec<u8> = le.iter().rev().copied().collect();
    for len in [size, size + 2].into_iter().chain(size.checked_sub(1)) {
      let fits = len >= size;
      for (big_endian, bytes) in [(false, &le), (true, &be)] {
        let mut expected = vec![0xa5; len];
        if fits {
          expected[..size].copy_from_slice(bytes);
        }
        let mut buf = vec![0xa5; len];
        let written = if big_endian {
          x.checked_copy_be_bytes_to(&mut buf)
        } else {
          x.checked_copy_le_bytes_to(&mut buf)
        };
        assert_eq!(
          (written, &buf),
          (fits.then_some(size), &expected),
          "{} into {len} bytes",
          model::text(p)
        );
        if fits {
          let mut buf = vec![0xa5; len];
          let written = if big_endian {
            x.copy_be_bytes_to(&mut buf)
          } else {
            x.copy_le_bytes_to(&mut buf)
          };
          assert_eq!(
            (written, buf),
            (size, expected),
            "{} into {len} bytes",
            model::text(p)
          );
        }
      }
    }
    for (index, &byte) in le.iter().enumerate() {
      assert_eq!(x.byte(index), byte, "byte {index} of {}", model::text(p));
    }
  }};
}

/// Checks the conversions from arrays of whole bytes of `$x`, a value of `$ty` of a width of whole
/// bytes, through the inherent `const fn`s: its arrays in each order read back as the value, and
/// so do those with the last byte replaced by `0x00`, `0xff`, `0x01` or `0x80`, as `$read` reads
/// their patterns, since every array holds a value.
macro_rules! check_whole_arrays {
  ($ty:ty, $x:expr, $read:expr) => {{
    let x: $ty = $x;
    let read = $read;
    let size = <$ty>::BYTES;
    let arrays = (x.to_le_bytes(), x.to_be_bytes(), x.to_ne_bytes());
    assert_eq!(
      (
        <$ty>::from_le_bytes(arrays.0),
        <$ty>::from_be_bytes(arrays.1),
        <$ty>::from_ne_bytes(arrays.2),
      ),
      (x, x, x),
      "{x:?}"
    );
    if size > 0 {
      for high in [0x00, 0xff, 0x01, 0x80] {
        let mut little = arrays.0;
        little[size - 1] = high;
        let mut big = little;
        big.reverse();
        let expected = read(&model::of_le_bytes(&little));
        assert_eq!(
          (<$ty>::from_le_bytes(little), <$ty>::from_be_bytes(big)),
          (expected, expected),
          "{little:x?}"
        );
      }
    }
  }};
}

/// Checks the byte order of `$x`, a value of `$ty` of a width of whole bytes whose pattern is
/// `$p`, through the inherent `const fn`s, against the bit model: `swap_bytes` reverses its bytes,
/// as `$read` reads the reversed pattern, and twice gives the value back, and `to_be`, `to_le`,
/// `from_be` and `from_le` give the value or its reversal, as `cfg!(target_endian)` decides.
macro_rules! check_byte_order {
  ($ty:ty, $x:expr, $p:expr, $read:expr) => {{
    let (x, p): ($ty, &[bool]) = ($x, $p);
    let read = $read;
    let swapped = read(&model::swap_bytes(p));
    let (to_big, to_little) = if cfg!(target_endian = "big") {
      (x, swapped)
    } else {
      (swapped, x)
    };
    assert_eq!(
      (
        x.swap_bytes(),
        x.swap_bytes().swap_bytes(),
        x.to_be(),
        x.to_le(),
        <$ty>::from_be(x),
        <$ty>::from_le(x),
      ),
      (swapped, x, to_big, to_little, to_big, to_little),
      "{}",
      model::text(p)
    );
  }};
}

/// Checks the arrays, the slices, the copies and the bytes of `$ty`, of the sign `$sign` and of
/// `$bits` bits, whose values `$from` creates from an `i128`, at each value of `$values`.
macro_rules! sweep_bytes {
  ($sign:ident; $ty:ty, $bits:expr, $values:expr, $from:expr, $value:expr, $magnitude:expr) => {{
    let (bits, signed) = ($bits, is_signed!($sign));
    let from: fn(i128) -> $ty = $from;
    let read = |q: &[bool]| from(value_of(q, signed));
    for a in $values {
      let p = pattern_of(a, bits);
      check_arrays!($ty, from(a), &p, signed, read);
      check_slices!($ty, from(a), &p, signed, read);
    }
  }};
}

/// Checks the conversions from arrays of whole bytes and the byte order of `$ty`, of the sign
/// `$sign` and of `$bits` bits, a whole number of bytes, whose values `$from` creates from an
/// `i128`, at each value of `$values`.
macro_rules! sweep_whole_bytes {
  ($sign:ident; $ty:ty, $bits:expr, $values:expr, $from:expr, $value:expr, $magnitude:expr) => {{
    let (bits, signed) = ($bits, is_signed!($sign));
    let from: fn(i128) -> $ty = $from;
    let read = |q: &[bool]| from(value_of(q, signed));
    for a in $values {
      check_whole_arrays!($ty, from(a), read);
      check_byte_order!($ty, from(a), &pattern_of(a, bits), read);
    }
  }};
}

/// The values of up to 24 bits at which the bytes of a storage are checked: those of
/// [`small_values`] up to 8 bits, and above those of the patterns of [`wide_patterns`] in one word
/// with the [`ends`]; or under Miri the ends in a storage that every tier checks, `primary`, and
/// none in another.
fn byte_values(bits: u32, signed: bool, primary: bool) -> Vec<i128> {
  match tier() {
    Tier::Miri if primary => ends(bits, signed),
    Tier::Miri => Vec::new(),
    _ if bits <= 8 => small_values(bits, signed, primary),
    _ => bigint::with_ends(
      wide_patterns::<1>(bits)
        .iter()
        .map(|pattern| value_of(pattern, signed)),
      ends(bits, signed),
    ),
  }
}

/// Checks the arrays, the slices, the copies and the bytes at the width `B`, `bits` bits, at most
/// 8, of `M` bytes, in `u8` and `i8` and `[u8; 2]` at the values of [`byte_values`] for a storage
/// that every tier checks, and in `[u8; 1]`, `[u16; 1]`, `u16` and `i16`, `u128` and `i128`,
/// `[u64; 1]` and `[u128; 1]` at those for another. Under Miri, in `u8`, `i8` and `[u8; 2]`.
fn bytes_of_small_widths<B, const M: usize>(bits: u32)
where
  B: FitsBits<u8> + FitsBits<i8> + FitsBits<u16> + FitsBits<i16> + FitsBits<u128> + FitsBits<i128>,
  B: FitsBits<[u8; 1]> + FitsBits<[u8; 2]> + FitsBits<[u16; 1]>,
  B: FitsBits<[u64; 1]> + FitsBits<[u128; 1]>,
  u8: LimbArray<B, Array = [u8; M]>,
{
  let primary = |bits, signed| byte_values(bits, signed, true);
  let secondary = |bits, signed| byte_values(bits, signed, false);
  sweep_storage!(sweep_bytes, primary, native B, bits, u8, i8);
  sweep_storage!(sweep_bytes, primary, array B, bits, [u8; 2]);
  if !tier().is_miri() {
    sweep_storage!(sweep_bytes, secondary, array B, bits, [u8; 1]);
    sweep_storage!(sweep_bytes, secondary, array B, bits, [u16; 1]);
    sweep_storage!(sweep_bytes, secondary, native B, bits, u16, i16);
    sweep_storage!(sweep_bytes, secondary, native B, bits, u128, i128);
    sweep_storage!(sweep_bytes, secondary, array B, bits, [u64; 1]);
    sweep_storage!(sweep_bytes, secondary, array B, bits, [u128; 1]);
  }
}

/// Checks the conversions from arrays of whole bytes and the byte order at the width `B`, `bits`
/// bits, 0 or 8, of `M` bytes, in the storages and at the values of [`bytes_of_small_widths`].
fn whole_bytes_of_small_widths<B, const M: usize>(bits: u32)
where
  B: FitsBits<u8> + FitsBits<i8> + FitsBits<u16> + FitsBits<i16> + FitsBits<u128> + FitsBits<i128>,
  B: FitsBits<[u8; 1]> + FitsBits<[u8; 2]> + FitsBits<[u16; 1]>,
  B: FitsBits<[u64; 1]> + FitsBits<[u128; 1]>,
  B: WholeBytes,
  u8: LimbArray<B, Array = [u8; M]>,
{
  let primary = |bits, signed| byte_values(bits, signed, true);
  let secondary = |bits, signed| byte_values(bits, signed, false);
  sweep_storage!(sweep_whole_bytes, primary, native B, bits, u8, i8);
  sweep_storage!(sweep_whole_bytes, primary, array B, bits, [u8; 2]);
  if !tier().is_miri() {
    sweep_storage!(sweep_whole_bytes, secondary, array B, bits, [u8; 1]);
    sweep_storage!(sweep_whole_bytes, secondary, array B, bits, [u16; 1]);
    sweep_storage!(sweep_whole_bytes, secondary, native B, bits, u16, i16);
    sweep_storage!(sweep_whole_bytes, secondary, native B, bits, u128, i128);
    sweep_storage!(sweep_whole_bytes, secondary, array B, bits, [u64; 1]);
    sweep_storage!(sweep_whole_bytes, secondary, array B, bits, [u128; 1]);
  }
}

/// Checks the bytes at the width `B`, `bits` bits, from 9 to 16, of `M` bytes, in `[u8; 2]`, `u16`
/// and `i16` and `[u16; 1]`, at the values of [`byte_values`]. Under Miri, in `[u8; 2]`.
fn bytes_of_large_widths<B, const M: usize>(bits: u32)
where
  B: FitsBits<u16> + FitsBits<i16> + FitsBits<[u8; 2]> + FitsBits<[u16; 1]>,
  u8: LimbArray<B, Array = [u8; M]>,
{
  let values = |bits, signed| byte_values(bits, signed, true);
  sweep_storage!(sweep_bytes, values, array B, bits, [u8; 2]);
  if !tier().is_miri() {
    sweep_storage!(sweep_bytes, values, native B, bits, u16, i16);
    sweep_storage!(sweep_bytes, values, array B, bits, [u16; 1]);
  }
}

/// Checks the conversions from arrays of whole bytes and the byte order at the width `B`, 16 bits,
/// of `M` bytes, in the storages and at the values of [`bytes_of_large_widths`].
fn whole_bytes_of_large_widths<B, const M: usize>(bits: u32)
where
  B: FitsBits<u16> + FitsBits<i16> + FitsBits<[u8; 2]> + FitsBits<[u16; 1]>,
  B: WholeBytes,
  u8: LimbArray<B, Array = [u8; M]>,
{
  let values = |bits, signed| byte_values(bits, signed, true);
  sweep_storage!(sweep_whole_bytes, values, array B, bits, [u8; 2]);
  if !tier().is_miri() {
    sweep_storage!(sweep_whole_bytes, values, native B, bits, u16, i16);
    sweep_storage!(sweep_whole_bytes, values, array B, bits, [u16; 1]);
  }
}

/// Checks the bytes at the width `B`, `bits` bits, from 17 to 24, of `M` bytes, in `[u8; 3]`,
/// `u32` and `i32`, `[u16; 2]` and `[u64; 1]`, at the values of [`byte_values`]. Under Miri, in
/// `[u8; 3]`.
fn bytes_of_wider_widths<B, const M: usize>(bits: u32)
where
  B: FitsBits<u32> + FitsBits<i32> + FitsBits<[u8; 3]> + FitsBits<[u16; 2]> + FitsBits<[u64; 1]>,
  u8: LimbArray<B, Array = [u8; M]>,
{
  let values = |bits, signed| byte_values(bits, signed, true);
  sweep_storage!(sweep_bytes, values, array B, bits, [u8; 3]);
  if !tier().is_miri() {
    sweep_storage!(sweep_bytes, values, native B, bits, u32, i32);
    sweep_storage!(sweep_bytes, values, array B, bits, [u16; 2]);
    sweep_storage!(sweep_bytes, values, array B, bits, [u64; 1]);
  }
}

/// Checks the conversions from arrays of whole bytes and the byte order at the width `B`, 24 bits,
/// of `M` bytes, in the storages and at the values of [`bytes_of_wider_widths`].
fn whole_bytes_of_wider_widths<B, const M: usize>(bits: u32)
where
  B: FitsBits<u32> + FitsBits<i32> + FitsBits<[u8; 3]> + FitsBits<[u16; 2]> + FitsBits<[u64; 1]>,
  B: WholeBytes,
  u8: LimbArray<B, Array = [u8; M]>,
{
  let values = |bits, signed| byte_values(bits, signed, true);
  sweep_storage!(sweep_whole_bytes, values, array B, bits, [u8; 3]);
  if !tier().is_miri() {
    sweep_storage!(sweep_whole_bytes, values, native B, bits, u32, i32);
    sweep_storage!(sweep_whole_bytes, values, array B, bits, [u16; 2]);
    sweep_storage!(sweep_whole_bytes, values, array B, bits, [u64; 1]);
  }
}

/// Every value of each width up to 8 bits, or the values that the tier takes, in native integers
/// and in arrays of one and two limbs, against the bit model: the arrays, the slices, the copies
/// and the bytes, and at 0 and 8 bits the conversions from arrays of whole bytes and the byte
/// order. Miri checks 0, 1, 7 and 8 bits at the ends, in `u8`, `i8` and `[u8; 2]`.
#[test]
fn bytes_of_up_to_8_bits() {
  bytes_of_small_widths::<U0, 0>(0);
  bytes_of_small_widths::<U1, 1>(1);
  bytes_of_small_widths::<U7, 1>(7);
  bytes_of_small_widths::<U8, 1>(8);
  whole_bytes_of_small_widths::<U0, 0>(0);
  whole_bytes_of_small_widths::<U8, 1>(8);
  if !tier().is_miri() {
    bytes_of_small_widths::<U2, 1>(2);
    bytes_of_small_widths::<U3, 1>(3);
    bytes_of_small_widths::<U4, 1>(4);
    bytes_of_small_widths::<U5, 1>(5);
    bytes_of_small_widths::<U6, 1>(6);
  }
}

/// The values that the tier takes of widths from 9 to 16 bits, with the ends, in two limbs of 8
/// bits, in `u16` and `i16` and in one limb of 16 bits, and at 16 bits the conversions from arrays
/// of whole bytes and the byte order: every width in an optimized build, 9, 12, 15 and 16 bits in
/// an unoptimized one, and 12 and 16 bits in two limbs of 8 bits under Miri.
#[test]
fn bytes_of_9_to_16_bits() {
  bytes_of_large_widths::<U12, 2>(12);
  bytes_of_large_widths::<U16, 2>(16);
  whole_bytes_of_large_widths::<U16, 2>(16);
  if !tier().is_miri() {
    bytes_of_large_widths::<U9, 2>(9);
    bytes_of_large_widths::<U15, 2>(15);
  }
  if tier().is_full() {
    bytes_of_large_widths::<U10, 2>(10);
    bytes_of_large_widths::<U11, 2>(11);
    bytes_of_large_widths::<U13, 2>(13);
    bytes_of_large_widths::<U14, 2>(14);
  }
}

/// The values that the tier takes of widths from 17 to 24 bits, with the ends, in three limbs of 8
/// bits, in `u32` and `i32`, in two limbs of 16 bits and in one of 64, and at 24 bits the
/// conversions from arrays of whole bytes and the byte order: every width in an optimized build,
/// 17, 20, 23 and 24 bits in an unoptimized one, and 24 bits in three limbs of 8 bits under Miri.
#[test]
fn bytes_of_17_to_24_bits() {
  bytes_of_wider_widths::<U24, 3>(24);
  whole_bytes_of_wider_widths::<U24, 3>(24);
  if !tier().is_miri() {
    bytes_of_wider_widths::<U17, 3>(17);
    bytes_of_wider_widths::<U20, 3>(20);
    bytes_of_wider_widths::<U23, 3>(23);
  }
  if tier().is_full() {
    bytes_of_wider_widths::<U18, 3>(18);
    bytes_of_wider_widths::<U19, 3>(19);
    bytes_of_wider_widths::<U21, 3>(21);
    bytes_of_wider_widths::<U22, 3>(22);
  }
}

/// The patterns of `bits` bits, above 24, at which the bytes of a wide width are checked: those of
/// `bigint::bit_patterns_by` in `K` words of 64 bits at the reduced indexes, cut to the width, or
/// under Miri four: zero, every bit, the top bit and one.
fn byte_patterns<const K: usize>(bits: u32) -> Vec<Vec<bool>> {
  let mut patterns: Vec<Vec<bool>> = if tier().is_miri() {
    vec![
      model::pattern(bits, |_| false),
      model::pattern(bits, |_| true),
      model::pattern(bits, |i| i + 1 == bits),
      model::pattern(bits, |i| i == 0),
    ]
  } else {
    bigint::bit_patterns_by::<u64, K>(&[bits], true)
      .iter()
      .map(|words| bigint::pattern_of_limbs(words, bits))
      .collect()
  };
  patterns.sort_unstable();
  patterns.dedup();
  patterns
}

/// Checks the bytes of `Uint` and `Int` of `$bits` bits, the width `$width`, in each storage of
/// `$storages`, written `unsigned, signed;`, at the patterns of [`byte_patterns`] in `$words` words
/// of 64 bits: the arrays, the slices, the copies and the bytes, and with `whole`, at a width of
/// whole bytes, the conversions from arrays of whole bytes and the byte order.
macro_rules! bytes_agree {
  (whole $width:ty, $bits:literal, $words:literal; $($uint:ty, $int:ty);+ $(;)?) => {{
    bytes_agree!($width, $bits, $words; $($uint, $int);+);
    let patterns = byte_patterns::<$words>($bits);
    $({
      type U = Uint<$width, $uint>;
      type I = Int<$width, $int>;
      for p in &patterns {
        let (unsigned, signed) = (U::from_pattern(p), I::from_pattern(p));
        check_whole_arrays!(U, unsigned, |q: &[bool]| U::from_pattern(q));
        check_whole_arrays!(I, signed, |q: &[bool]| I::from_pattern(q));
        check_byte_order!(U, unsigned, p, |q: &[bool]| U::from_pattern(q));
        check_byte_order!(I, signed, p, |q: &[bool]| I::from_pattern(q));
      }
    })+
  }};
  ($width:ty, $bits:literal, $words:literal; $($uint:ty, $int:ty);+ $(;)?) => {{
    let patterns = byte_patterns::<$words>($bits);
    $({
      type U = Uint<$width, $uint>;
      type I = Int<$width, $int>;
      for p in &patterns {
        let (unsigned, signed) = (U::from_pattern(p), I::from_pattern(p));
        check_arrays!(U, unsigned, p, false, |q: &[bool]| U::from_pattern(q));
        check_arrays!(I, signed, p, true, |q: &[bool]| I::from_pattern(q));
        check_slices!(U, unsigned, p, false, |q: &[bool]| U::from_pattern(q));
        check_slices!(I, signed, p, true, |q: &[bool]| I::from_pattern(q));
      }
    })+
  }};
}

/// The bit patterns of 200 to 8192 bits, the widest width with an array of bytes, in arrays of
/// limbs of 8, 64 and 128 bits: the arrays, the slices, the copies and the bytes against the bit
/// model, and at the widths of whole bytes the conversions from arrays and the byte order. 200,
/// 201, which is no whole number of bytes, 256 and 1024 bits in an unoptimized build, and also
/// 4096 and 8192 bits in an optimized one; Miri checks 200 and 256 bits, at four patterns, in limbs
/// of 8 and 64 bits.
#[test]
fn bytes_of_wide_widths() {
  bytes_agree!(whole U200, 200, 4; [u8; 25], [u8; 25]; [u64; 4], [u64; 4]);
  bytes_agree!(whole U256, 256, 4; [u8; 32], [u8; 32]; [u64; 4], [u64; 4]);
  if !tier().is_miri() {
    bytes_agree!(whole U200, 200, 4; [u128; 2], [u128; 2]);
    bytes_agree!(U201, 201, 4; [u8; 26], [u8; 26]; [u64; 4], [u64; 4]; [u128; 2], [u128; 2]);
    bytes_agree!(whole U256, 256, 4; [u128; 2], [u128; 2]);
    bytes_agree!(whole U1024, 1024, 16; [u8; 128], [u8; 128]; [u64; 16], [u64; 16]);
    bytes_agree!(whole U1024, 1024, 16; [u128; 8], [u128; 8]);
  }
  if tier().is_full() {
    bytes_agree!(whole U4096, 4096, 64; [u8; 512], [u8; 512]; [u64; 64], [u64; 64]);
    bytes_agree!(whole U4096, 4096, 64; [u128; 32], [u128; 32]);
    bytes_agree!(whole U8192, 8192, 128; [u8; 1024], [u8; 1024]; [u64; 128], [u64; 128]);
    bytes_agree!(whole U8192, 8192, 128; [u128; 64], [u128; 64]);
  }
}

/// 16384 bits, wider than every array of bytes, in arrays of limbs of 8, 64 and 128 bits, at the
/// patterns of [`byte_patterns`]: the slices, the copies, the bytes and the byte order against the
/// bit model, in an optimized build.
#[test]
fn bytes_of_16384_bits() {
  if !tier().is_full() {
    return;
  }
  macro_rules! check {
    ($($uint:ty, $int:ty);+ $(;)?) => {$({
      for p in &byte_patterns::<256>(16384) {
        let (unsigned, signed) = (<$uint>::from_pattern(p), <$int>::from_pattern(p));
        check_slices!($uint, unsigned, p, false, |q: &[bool]| <$uint>::from_pattern(q));
        check_slices!($int, signed, p, true, |q: &[bool]| <$int>::from_pattern(q));
        check_byte_order!($uint, unsigned, p, |q: &[bool]| <$uint>::from_pattern(q));
        check_byte_order!($int, signed, p, |q: &[bool]| <$int>::from_pattern(q));
      }
    })+};
  }
  check!(
    Uint<U16384, [u8; 2048]>, Int<U16384, [u8; 2048]>;
    Uint<U16384, [u64; 256]>, Int<U16384, [u64; 256]>;
    Uint<U16384, [u128; 128]>, Int<U16384, [u128; 128]>;
  );
}

/// `copy_le_bytes_to` and `copy_be_bytes_to` into a buffer shorter than the bytes of the value, and
/// `byte` at an index of the number of bytes or more, panic in every build with their messages and
/// report the caller, leaving the buffer unchanged, in native integers and arrays, at zero bits,
/// where every index panics and every buffer is long enough, and above 8192 bits.
#[test]
fn short_buffers_and_byte_indexes_panic() {
  let le = "copy_le_bytes_to: the buffer is shorter than the bytes of the value".to_string();
  let be = "copy_be_bytes_to: the buffer is shorter than the bytes of the value".to_string();
  let index = "byte index out of range".to_string();
  // A value, and a buffer one byte shorter than its bytes.
  macro_rules! short {
    ($ty:ty, $x:expr) => {{
      let x: $ty = $x;
      (x, vec![0xa5; <$ty>::BYTES - 1])
    }};
  }
  type Bytes = Int<U24, [u8; 3]>;
  type Wide = Uint<U16384, [u64; 256]>;
  let (x, mut buf) = short!(u7, u7::MAX);
  panics_here!(x.copy_le_bytes_to(&mut buf), le);
  panics_here!(x.copy_be_bytes_to(&mut buf), be);
  assert!(buf.is_empty());
  let (x, mut buf) = short!(Bytes, Bytes::MIN);
  panics_here!(x.copy_le_bytes_to(&mut buf), le);
  panics_here!(x.copy_be_bytes_to(&mut buf), be);
  assert_eq!(buf, [0xa5; 2]);
  let (x, mut buf) = short!(i256, i256::MIN);
  panics_here!(x.copy_le_bytes_to(&mut buf), le);
  panics_here!(x.copy_be_bytes_to(&mut buf), be);
  assert_eq!(buf, [0xa5; 31]);
  let (x, mut buf) = short!(Wide, Wide::MAX);
  panics_here!(x.copy_le_bytes_to(&mut buf), le);
  panics_here!(x.copy_be_bytes_to(&mut buf), be);
  assert_eq!(buf, vec![0xa5; 2047]);
  for k in [0, 1, usize::MAX - 1] {
    panics_here!(u7::MAX.byte(u7::BYTES.saturating_add(k)), index);
    panics_here!(Bytes::MIN.byte(Bytes::BYTES.saturating_add(k)), index);
    panics_here!(i256::MIN.byte(i256::BYTES.saturating_add(k)), index);
    panics_here!(Wide::MAX.byte(Wide::BYTES.saturating_add(k)), index);
    panics_here!(u0::ZERO.byte(k), index);
    panics_here!(Int::<U0, [u16; 1]>::ZERO.byte(k), index);
  }
}

/// The byte methods in constant contexts, in native integers and arrays, at zero bits and at one
/// signed bit: the arrays and the conversions from them, the slices, a copy to a local buffer, the
/// byte at an index and the byte order.
#[test]
fn bytes_in_constant_contexts() {
  type Bytes = Int<U24, [u8; 3]>;
  type Word = Uint<U12, [u64; 1]>;
  type I12 = Int<U12>;
  type U16x2 = Uint<U16, [u8; 2]>;
  const SIZES: [usize; 5] = [u0::BYTES, u7::BYTES, Word::BYTES, Bytes::BYTES, i256::BYTES];
  const ARRAYS: ([u8; 1], [u8; 2], [u8; 3], [u8; 0]) = (
    i7::MIN.to_le_bytes(),
    Word::MAX.to_be_bytes(),
    Bytes::MIN.to_le_bytes(),
    u0::ZERO.to_ne_bytes(),
  );
  const CHECKED: (Option<u7>, Option<u7>, Option<Word>) = (
    u7::checked_from_le_bytes([0x7f]),
    u7::checked_from_be_bytes([0x80]),
    Word::checked_from_be_bytes([0x0f, 0xff]),
  );
  const WHOLE: (Bytes, Bytes, Bytes) = (
    Bytes::from_le_bytes([0, 0, 0x80]),
    Bytes::from_be_bytes([0x80, 0, 0]),
    Bytes::from_ne_bytes(Bytes::MIN.to_ne_bytes()),
  );
  const SLICES: (Option<I12>, Option<I12>, I12, I12) = (
    I12::checked_from_le_slice(&[0x80]),
    I12::checked_from_be_slice(&[0x08, 0x00]),
    I12::wrapping_from_le_slice(&[0x80]),
    I12::wrapping_from_le_slice(&[0xff, 0x0f]),
  );
  const COPY: ([u8; 4], usize, Option<usize>) = {
    let mut buf = [0xa5; 4];
    let written = Bytes::MIN.copy_le_bytes_to(&mut buf);
    let mut short = [0; 2];
    (
      buf,
      written,
      Bytes::MIN.checked_copy_be_bytes_to(&mut short),
    )
  };
  const BYTE: [u8; 3] = [Bytes::MIN.byte(2), i256::MIN.byte(31), u7::MAX.byte(0)];
  const ORDER: (Int<U24, i32>, U16x2, U16x2) = (
    Int::<U24, i32>::new(0x01_0203).swap_bytes(),
    U16x2::from_limbs([1, 2]).to_be(),
    U16x2::from_be(U16x2::from_limbs([1, 2])),
  );
  const ZERO_BITS: (u0, Option<i0>) = (u0::from_le_bytes([]), i0::checked_from_le_slice(&[0, 0]));
  const ONE_BIT: ([u8; 1], Option<i1>) = (i1::MIN.to_le_bytes(), i1::checked_from_le_bytes([1]));

  assert_eq!(SIZES, [0, 1, 2, 3, 32]);
  assert_eq!(ARRAYS, ([0xc0], [0x0f, 0xff], [0, 0, 0x80], []));
  assert_eq!(CHECKED, (Some(u7::MAX), None, Some(Word::MAX)));
  assert_eq!(WHOLE, (Bytes::MIN, Bytes::MIN, Bytes::MIN));
  assert_eq!(
    SLICES,
    (Some(I12::new(-128)), None, I12::new(-128), I12::new(-1))
  );
  assert_eq!(COPY, ([0, 0, 0x80, 0xa5], 3, None));
  assert_eq!(BYTE, [0x80, 0x80, 0x7f]);
  // The bytes 1 and 2 swapped, which is the big-endian value on a little-endian target.
  let to_big = if cfg!(target_endian = "big") {
    U16x2::from_limbs([1, 2])
  } else {
    U16x2::from_limbs([2, 1])
  };
  assert_eq!(ORDER, (Int::<U24, i32>::new(0x03_0201), to_big, to_big));
  assert_eq!(ZERO_BITS, (u0::ZERO, Some(i0::ZERO)));
  assert_eq!(ONE_BIT, ([0xff], None));
}

/// Zero bits hold only zero: no bytes, an array of none, from which zero reads back, a reversal
/// that gives zero, copies that write nothing, empty and zero slices that read as zero, and slices
/// of another value, 1 or -1, that do not fit, in native integers and arrays. One signed bit holds
/// -1 and 0: -1 is `[0xff]`, `[0x01]` holds no value of it, and the wrapping slices reduce modulo
/// 2 into the range.
#[test]
fn zero_bits_and_one_signed_bit() {
  macro_rules! zero_bits {
    ($($ty:ty),+ $(,)?) => {$({
      type T = $ty;
      let zero = T::ZERO;
      assert_eq!(T::BYTES, 0);
      assert_eq!(
        (zero.to_le_bytes(), zero.to_be_bytes(), zero.to_ne_bytes()),
        ([], [], [])
      );
      assert_eq!(
        (
          T::checked_from_le_bytes([]),
          T::from_le_bytes([]),
          T::from_be_bytes([]),
          T::from_ne_bytes([]),
        ),
        (Some(zero), zero, zero, zero)
      );
      assert_eq!(
        (zero.swap_bytes(), zero.to_be(), zero.to_le(), T::from_be(zero), T::from_le(zero)),
        (zero, zero, zero, zero, zero)
      );
      for slice in [&[][..], &[0][..], &[0, 0][..]] {
        assert_eq!(
          (T::checked_from_le_slice(slice), T::wrapping_from_be_slice(slice)),
          (Some(zero), zero),
          "{slice:x?}"
        );
      }
      for slice in [&[1][..], &[0xff][..], &[0, 0x80][..]] {
        assert_eq!(
          (T::checked_from_le_slice(slice), T::wrapping_from_le_slice(slice)),
          (None, zero),
          "{slice:x?}"
        );
      }
      let mut buf = [0xa5; 2];
      let written = (zero.copy_le_bytes_to(&mut buf), zero.checked_copy_be_bytes_to(&mut []));
      assert_eq!((written, buf), ((0, Some(0)), [0xa5; 2]));
    })+};
  }
  zero_bits!(
    u0,
    Uint<U0, u8>,
    Uint<U0, [u16; 1]>,
    i0,
    Int<U0, i8>,
    Int<U0, [u8; 1]>
  );

  macro_rules! one_signed_bit {
    ($($ty:ty),+ $(,)?) => {$({
      type T = $ty;
      let (min, zero) = (T::MIN, T::ZERO);
      assert_eq!(T::BYTES, 1);
      assert_eq!(
        (min.to_le_bytes(), min.to_be_bytes(), zero.to_le_bytes()),
        ([0xff], [0xff], [0])
      );
      for (bytes, expected) in [
        ([0xff], Some(min)),
        ([0x00], Some(zero)),
        ([0x01], None),
        ([0x7f], None),
        ([0xfe], None),
        ([0x80], None),
      ] {
        assert_eq!(T::checked_from_le_bytes(bytes), expected, "{bytes:x?}");
      }
      for (slice, wrapped) in [
        (&[0xff, 0xff][..], min),
        (&[0x01][..], min),
        (&[0x02][..], zero),
        (&[][..], zero),
      ] {
        assert_eq!(T::wrapping_from_le_slice(slice), wrapped, "{slice:x?}");
      }
      assert_eq!(
        (T::checked_from_le_slice(&[0xff, 0xff]), T::checked_from_be_slice(&[0, 1])),
        (Some(min), None)
      );
      assert_eq!((min.byte(0), zero.byte(0)), (0xff, 0));
    })+};
  }
  one_signed_bit!(i1, Int<U1, [u16; 1]>, Int<U1, [u64; 1]>);
}
