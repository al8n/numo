//! The bytes and the byte order of `Uint` and `Int`: the arrays of bytes in each order and the
//! conversions back from them, checked and at whole bytes, the checked, wrapping, overflowing,
//! saturating and strict conversions from slices, the copies to a buffer and the byte at an index,
//! against the bit model at every width up to 24 bits and at wider ones; the number of bytes up to
//! the widest widths; the reversal of the bytes and the conversions of the byte order at widths of
//! whole bytes; the panics of a short buffer, of a byte index and of a strict slice; constant
//! contexts; and zero bits and one signed bit.

use typenum::{
  U3, U4, U5, U6, U10, U11, U12, U13, U14, U15, U17, U18, U19, U20, U21, U22, U23, U201, U8192,
  U16384, U65536,
};

use super::*;
use crate::{
  ByteArray, FromByteArray, IntRepr, IntegerByteOrder, IntegerBytes, LimbArray, NonZeroInt,
  NonZeroOf, NonZeroUint, UintRepr, WholeBytes, i0, i1, u0,
};

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
/// against the bit model: the conversions from each slice of `bigint::byte_slices`, least
/// significant byte first and reversed, whose results `$read` reads: the checked and the wrapping
/// ones, the wrapping one returning the value of the checked one wherever that returns one, and the
/// overflowing, saturating and strict ones, which return the wrapped value with whether the checked
/// one returns `None`, the checked value or the bound of the range that the integer passes, and
/// the checked value where there is one; the copies to buffers of the bytes of the value, two more
/// and one less, whose other bytes stay as they were; and the byte at each index.
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
      let negative = signed && model::sign(&q);
      let saturated = checked.unwrap_or_else(|| read(&bound_pattern(width, signed, negative)));
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
      assert_eq!(
        [
          (
            <$ty>::overflowing_from_le_slice(&slice),
            <$ty>::saturating_from_le_slice(&slice),
          ),
          (
            <$ty>::overflowing_from_be_slice(&reversed),
            <$ty>::saturating_from_be_slice(&reversed),
          ),
        ],
        [((wrapped, checked.is_none()), saturated); 2],
        "{slice:x?} at {width} bits"
      );
      if let Some(value) = checked {
        assert_eq!(
          (
            <$ty>::strict_from_le_slice(&slice),
            <$ty>::strict_from_be_slice(&reversed),
          ),
          (value, value),
          "{slice:x?} at {width} bits"
        );
      }
    }
    let le = model::le_bytes(p, size, signed);
    let be: Vec<u8> = le.iter().rev().copied().collect();
    for len in buffer_lengths(size) {
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

/// The lengths of the buffers that the bytes of a value of `size` bytes are copied to: `size`, two
/// more, and one less, which is too short.
fn buffer_lengths(size: usize) -> Vec<usize> {
  [size, size + 2]
    .into_iter()
    .chain(size.checked_sub(1))
    .collect()
}

/// What the arrays of bytes of a value give, through the inherent `const fn`s in `array_results!`
/// and through `ByteArray` in [`arrays_through_traits`], which must agree.
#[derive(Debug, PartialEq)]
struct ArrayResults<T> {
  /// `to_le_bytes`, `to_be_bytes` and `to_ne_bytes`.
  arrays: [Vec<u8>; 3],
  /// `checked_from_le_bytes`, `checked_from_be_bytes` and `checked_from_ne_bytes` of those arrays.
  from_arrays: [Option<T>; 3],
}

/// What the overflowing, the saturating and the strict conversion from a slice give, the last only
/// where the checked conversion returns a value, as it panics elsewhere.
type Forms<T> = ((T, bool), T, Option<T>);

/// What the slices, the copies and the bytes of a value give at some slices and at the buffers of
/// [`buffer_lengths`], through the inherent `const fn`s in `slice_results!` and through
/// `IntegerBytes` in [`slices_through_traits`], which must agree.
#[derive(Debug, PartialEq)]
struct SliceResults<T> {
  /// At each slice, `checked_from_le_slice` and `wrapping_from_le_slice`, and the same of the
  /// reversed slice from big-endian.
  reads: Vec<[(Option<T>, T); 2]>,
  /// At each slice, the [`Forms`] of `overflowing_from_le_slice`, `saturating_from_le_slice` and
  /// `strict_from_le_slice`, and the same of the reversed slice from big-endian.
  forms: Vec<[Forms<T>; 2]>,
  /// At each buffer, `checked_copy_le_bytes_to` and `checked_copy_be_bytes_to`, with the buffers
  /// that they leave.
  copies: Vec<[(Option<usize>, Vec<u8>); 2]>,
  /// `byte` at each index below `BYTES`.
  bytes: Vec<u8>,
}

/// The [`ArrayResults`] of `$x`, a value of `$ty`, through the inherent `const fn`s.
macro_rules! array_results {
  ($ty:ty, $x:expr) => {{
    let x: $ty = $x;
    let arrays = (x.to_le_bytes(), x.to_be_bytes(), x.to_ne_bytes());
    ArrayResults {
      arrays: [arrays.0.to_vec(), arrays.1.to_vec(), arrays.2.to_vec()],
      from_arrays: [
        <$ty>::checked_from_le_bytes(arrays.0),
        <$ty>::checked_from_be_bytes(arrays.1),
        <$ty>::checked_from_ne_bytes(arrays.2),
      ],
    }
  }};
}

/// The [`ArrayResults`] of `x` through `ByteArray`.
fn arrays_through_traits<T: ByteArray>(x: &T) -> ArrayResults<T> {
  let arrays = (x.to_le_bytes(), x.to_be_bytes(), x.to_ne_bytes());
  ArrayResults {
    arrays: [
      arrays.0.as_ref().to_vec(),
      arrays.1.as_ref().to_vec(),
      arrays.2.as_ref().to_vec(),
    ],
    from_arrays: [
      T::checked_from_le_bytes(arrays.0),
      T::checked_from_be_bytes(arrays.1),
      T::checked_from_ne_bytes(arrays.2),
    ],
  }
}

/// The [`SliceResults`] of `$x`, a value of `$ty`, at the slices of `$slices`, through the
/// inherent `const fn`s.
macro_rules! slice_results {
  ($ty:ty, $x:expr, $slices:expr) => {{
    let (x, slices): ($ty, &[Vec<u8>]) = ($x, $slices);
    SliceResults {
      reads: slices
        .iter()
        .map(|slice| {
          let reversed: Vec<u8> = slice.iter().rev().copied().collect();
          [
            (
              <$ty>::checked_from_le_slice(slice),
              <$ty>::wrapping_from_le_slice(slice),
            ),
            (
              <$ty>::checked_from_be_slice(&reversed),
              <$ty>::wrapping_from_be_slice(&reversed),
            ),
          ]
        })
        .collect(),
      forms: slices
        .iter()
        .map(|slice| {
          let reversed: Vec<u8> = slice.iter().rev().copied().collect();
          let fits = <$ty>::checked_from_le_slice(slice).is_some();
          [
            (
              <$ty>::overflowing_from_le_slice(slice),
              <$ty>::saturating_from_le_slice(slice),
              fits.then(|| <$ty>::strict_from_le_slice(slice)),
            ),
            (
              <$ty>::overflowing_from_be_slice(&reversed),
              <$ty>::saturating_from_be_slice(&reversed),
              fits.then(|| <$ty>::strict_from_be_slice(&reversed)),
            ),
          ]
        })
        .collect(),
      copies: buffer_lengths(<$ty>::BYTES)
        .into_iter()
        .map(|len| {
          let (mut little, mut big) = (vec![0xa5; len], vec![0xa5; len]);
          [
            (x.checked_copy_le_bytes_to(&mut little), little),
            (x.checked_copy_be_bytes_to(&mut big), big),
          ]
        })
        .collect(),
      bytes: (0..<$ty>::BYTES).map(|index| x.byte(index)).collect(),
    }
  }};
}

/// The [`SliceResults`] of `x` at the slices of `slices` through `IntegerBytes`.
fn slices_through_traits<T: IntegerBytes>(x: &T, slices: &[Vec<u8>]) -> SliceResults<T> {
  SliceResults {
    reads: slices
      .iter()
      .map(|slice| {
        let reversed: Vec<u8> = slice.iter().rev().copied().collect();
        [
          (
            T::checked_from_le_slice(slice),
            T::wrapping_from_le_slice(slice),
          ),
          (
            T::checked_from_be_slice(&reversed),
            T::wrapping_from_be_slice(&reversed),
          ),
        ]
      })
      .collect(),
    forms: slices
      .iter()
      .map(|slice| {
        let reversed: Vec<u8> = slice.iter().rev().copied().collect();
        let fits = T::checked_from_le_slice(slice).is_some();
        [
          (
            T::overflowing_from_le_slice(slice),
            T::saturating_from_le_slice(slice),
            fits.then(|| T::strict_from_le_slice(slice)),
          ),
          (
            T::overflowing_from_be_slice(&reversed),
            T::saturating_from_be_slice(&reversed),
            fits.then(|| T::strict_from_be_slice(&reversed)),
          ),
        ]
      })
      .collect(),
    copies: buffer_lengths(T::BYTES)
      .into_iter()
      .map(|len| {
        let (mut little, mut big) = (vec![0xa5; len], vec![0xa5; len]);
        [
          (x.checked_copy_le_bytes_to(&mut little), little),
          (x.checked_copy_be_bytes_to(&mut big), big),
        ]
      })
      .collect(),
    bytes: (0..T::BYTES).map(|index| x.byte(index)).collect(),
  }
}

/// `from_le_bytes`, `from_be_bytes` and `from_ne_bytes` of the arrays of `$x`, a value of `$ty`
/// of a width of whole bytes, and `swap_bytes`, `to_be`, `to_le`, `from_be` and `from_le` of it,
/// through the inherent `const fn`s.
macro_rules! whole_results {
  ($ty:ty, $x:expr) => {{
    let x: $ty = $x;
    (
      [
        <$ty>::from_le_bytes(x.to_le_bytes()),
        <$ty>::from_be_bytes(x.to_be_bytes()),
        <$ty>::from_ne_bytes(x.to_ne_bytes()),
      ],
      order_results!($ty, x),
    )
  }};
}

/// `swap_bytes`, `to_be`, `to_le`, `from_be` and `from_le` of `$x`, a value of `$ty` of a width of
/// whole bytes, through the inherent `const fn`s.
macro_rules! order_results {
  ($ty:ty, $x:expr) => {{
    let x: $ty = $x;
    [
      x.swap_bytes(),
      x.to_be(),
      x.to_le(),
      <$ty>::from_be(x),
      <$ty>::from_le(x),
    ]
  }};
}

/// What `whole_results!` gives for `x`, through `FromByteArray` and `IntegerByteOrder`.
fn whole_through_traits<T: FromByteArray + IntegerByteOrder + Copy>(x: &T) -> ([T; 3], [T; 5]) {
  (
    [
      T::from_le_bytes(x.to_le_bytes()),
      T::from_be_bytes(x.to_be_bytes()),
      T::from_ne_bytes(x.to_ne_bytes()),
    ],
    order_through_traits(x),
  )
}

/// What `order_results!` gives for `x`, through `IntegerByteOrder`.
fn order_through_traits<T: IntegerByteOrder + Copy>(x: &T) -> [T; 5] {
  [
    x.swap_bytes(),
    x.to_be(),
    x.to_le(),
    T::from_be(*x),
    T::from_le(*x),
  ]
}

/// Asserts that the arrays, the slices, the copies and the bytes of `$x`, a value of `$ty` whose
/// pattern is `$p`, read in two's complement if `$signed`, through `ByteArray` and `IntegerBytes`
/// give what the inherent `const fn`s give, at the slices of `bigint::byte_slices`.
macro_rules! traits_agree {
  ($ty:ty, $x:expr, $p:expr, $signed:expr) => {{
    let (x, p): ($ty, &[bool]) = ($x, $p);
    let slices = bigint::byte_slices(p, $signed);
    assert_eq!(
      array_results!($ty, x),
      arrays_through_traits(&x),
      "{}",
      model::text(p)
    );
    assert_eq!(
      slice_results!($ty, x, &slices),
      slices_through_traits(&x, &slices),
      "{}",
      model::text(p)
    );
  }};
}

/// Checks the arrays, the slices, the copies and the bytes of `$ty`, of the sign `$sign` and of
/// `$bits` bits, whose values `$from` creates from an `i128`, at each value of `$values`, through
/// the inherent `const fn`s against the bit model, and at the ends of the width through the traits
/// against the inherent `const fn`s.
macro_rules! sweep_bytes {
  ($sign:ident; $ty:ty, $bits:expr, $values:expr, $from:expr, $value:expr, $magnitude:expr) => {{
    let (bits, signed) = ($bits, is_signed!($sign));
    let from: fn(i128) -> $ty = $from;
    let read = |q: &[bool]| from(value_of(q, signed));
    let ends = ends(bits, signed);
    for a in $values {
      let (x, p) = (from(a), pattern_of(a, bits));
      check_arrays!($ty, x, &p, signed, read);
      check_slices!($ty, x, &p, signed, read);
      if ends.contains(&a) {
        traits_agree!($ty, x, &p, signed);
      }
    }
  }};
}

/// Asserts that the byte order of `$x`, a value of the non-zero integer `$nonzero` of the integer
/// `$integer`, through the inherent `const fn`s gives what the byte order of the integer gives on
/// its value, and through `IntegerByteOrder` what the inherent `const fn`s give.
macro_rules! nonzero_order_agrees {
  ($integer:ty, $nonzero:ty, $x:expr) => {{
    let x: $nonzero = $x;
    let value = x.get();
    let inherent = [
      x.swap_bytes(),
      x.to_be(),
      x.to_le(),
      <$nonzero>::from_be(x),
      <$nonzero>::from_le(x),
    ];
    assert_eq!(
      inherent.map(|y| y.get()),
      order_results!($integer, value),
      "{value:?}"
    );
    assert_eq!(order_through_traits(&x), inherent, "{value:?}");
  }};
}

/// Checks the conversions from arrays of whole bytes and the byte order of `$ty`, of the sign
/// `$sign` and of `$bits` bits, a whole number of bytes, whose values `$from` creates from an
/// `i128`, at each value of `$values`, through the inherent `const fn`s against the bit model, at
/// the ends of the width through the traits against the inherent `const fn`s, and the byte order
/// of the non-zero integer of each value that is not zero, or under Miri of the first one, against
/// that of the integer.
macro_rules! sweep_whole_bytes {
  ($sign:ident; $ty:ty, $bits:expr, $values:expr, $from:expr, $value:expr, $magnitude:expr) => {{
    let (bits, signed) = ($bits, is_signed!($sign));
    let from: fn(i128) -> $ty = $from;
    let read = |q: &[bool]| from(value_of(q, signed));
    let ends = ends(bits, signed);
    let mut nonzero_left = if tier().is_miri() { 1 } else { usize::MAX };
    for a in $values {
      let x = from(a);
      check_whole_arrays!($ty, x, read);
      check_byte_order!($ty, x, &pattern_of(a, bits), read);
      if ends.contains(&a) {
        assert_eq!(
          whole_results!($ty, x),
          whole_through_traits(&x),
          "{a} in {bits} bits"
        );
      }
      if nonzero_left > 0
        && let Some(nonzero) = <NonZeroOf<$ty>>::new(x)
      {
        nonzero_order_agrees!($ty, NonZeroOf<$ty>, nonzero);
        nonzero_left -= 1;
      }
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
/// whole bytes, the conversions from arrays of whole bytes and the byte order. Each storage checks
/// in a frame of its own, because the temporaries of every width in one frame exceed the stack of a
/// test thread in an unoptimized build.
macro_rules! bytes_agree {
  (whole $width:ty, $bits:literal, $words:literal; $($uint:ty, $int:ty);+ $(;)?) => {{
    bytes_agree!($width, $bits, $words; $($uint, $int);+);
    let patterns = byte_patterns::<$words>($bits);
    $({
      #[inline(never)]
      fn storage(patterns: &[Vec<bool>]) {
        type U = Uint<$width, $uint>;
        type I = Int<$width, $int>;
        for p in patterns {
          let (unsigned, signed) = (U::from_pattern(p), I::from_pattern(p));
          check_whole_arrays!(U, unsigned, |q: &[bool]| U::from_pattern(q));
          check_whole_arrays!(I, signed, |q: &[bool]| I::from_pattern(q));
          check_byte_order!(U, unsigned, p, |q: &[bool]| U::from_pattern(q));
          check_byte_order!(I, signed, p, |q: &[bool]| I::from_pattern(q));
          assert_eq!(whole_results!(U, unsigned), whole_through_traits(&unsigned));
          assert_eq!(whole_results!(I, signed), whole_through_traits(&signed));
        }
      }
      storage(&patterns);
    })+
  }};
  ($width:ty, $bits:literal, $words:literal; $($uint:ty, $int:ty);+ $(;)?) => {{
    let patterns = byte_patterns::<$words>($bits);
    $({
      #[inline(never)]
      fn storage(patterns: &[Vec<bool>]) {
        type U = Uint<$width, $uint>;
        type I = Int<$width, $int>;
        for p in patterns {
          let (unsigned, signed) = (U::from_pattern(p), I::from_pattern(p));
          check_arrays!(U, unsigned, p, false, |q: &[bool]| U::from_pattern(q));
          check_arrays!(I, signed, p, true, |q: &[bool]| I::from_pattern(q));
          check_slices!(U, unsigned, p, false, |q: &[bool]| U::from_pattern(q));
          check_slices!(I, signed, p, true, |q: &[bool]| I::from_pattern(q));
          traits_agree!(U, unsigned, p, false);
          traits_agree!(I, signed, p, true);
        }
      }
      storage(&patterns);
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
  // Each storage checks in a frame of its own, to keep every frame within the stack of a test
  // thread in an unoptimized build.
  macro_rules! check {
    ($($uint:ty, $int:ty);+ $(;)?) => {$({
      #[inline(never)]
      fn storage() {
        for p in &byte_patterns::<256>(16384) {
          let (unsigned, signed) = (<$uint>::from_pattern(p), <$int>::from_pattern(p));
          check_slices!($uint, unsigned, p, false, |q: &[bool]| <$uint>::from_pattern(q));
          check_slices!($int, signed, p, true, |q: &[bool]| <$int>::from_pattern(q));
          check_byte_order!($uint, unsigned, p, |q: &[bool]| <$uint>::from_pattern(q));
          check_byte_order!($int, signed, p, |q: &[bool]| <$int>::from_pattern(q));
          let slices = bigint::byte_slices(p, false);
          assert_eq!(
            slice_results!($uint, unsigned, &slices),
            slices_through_traits(&unsigned, &slices)
          );
          let slices = bigint::byte_slices(p, true);
          assert_eq!(
            slice_results!($int, signed, &slices),
            slices_through_traits(&signed, &slices)
          );
          assert_eq!(order_results!($uint, unsigned), order_through_traits(&unsigned));
          assert_eq!(order_results!($int, signed), order_through_traits(&signed));
        }
      }
      storage();
    })+};
  }
  check!(
    Uint<U16384, [u8; 2048]>, Int<U16384, [u8; 2048]>;
    Uint<U16384, [u64; 256]>, Int<U16384, [u64; 256]>;
    Uint<U16384, [u128; 128]>, Int<U16384, [u128; 128]>;
  );
}

/// The number of bytes of a width is counted in `u32`, which holds every width, and then narrowed to
/// `usize`: it rounds up to whole bytes from zero bits to `u32::MAX` bits, the widest width.
#[test]
fn byte_counts_round_up_in_u32() {
  use crate::integer::bytes::byte_count;

  for (bits, bytes) in [
    (0, 0),
    (1, 1),
    (7, 1),
    (8, 1),
    (9, 2),
    (65535, 8192),
    (65536, 8192),
    (65537, 8193),
  ] {
    assert_eq!(byte_count(bits), bytes, "{bits} bits");
  }
  // A 16-bit `usize` cannot hold the number of bytes of the widest width, where the count fails.
  #[cfg(not(target_pointer_width = "16"))]
  assert_eq!(byte_count(u32::MAX), 1 << 29);
}

/// The widest default storage, 65536 bits in `[u64; 1024]`, has 8192 bytes, inherent and through
/// `IntegerBytes`, for `Uint` and `Int`: the byte at the last index, and checked copies into a
/// buffer of 8192 bytes, which write them all, while a buffer one byte shorter takes none. The
/// copies, of 8192 bytes each, are left out under Miri.
#[test]
fn bytes_of_the_widest_default_storage() {
  type Wide = Uint<U65536, [u64; 1024]>;
  type WideInt = Int<U65536, [u64; 1024]>;
  assert_eq!(
    (
      Wide::BYTES,
      WideInt::BYTES,
      <Wide as IntegerBytes>::BYTES,
      <WideInt as IntegerBytes>::BYTES,
    ),
    (8192, 8192, 8192, 8192)
  );
  assert_eq!(
    (
      Wide::MAX.byte(8191),
      WideInt::MIN.byte(8191),
      WideInt::MIN.byte(8190),
      IntegerBytes::byte(&WideInt::MIN, 8191),
    ),
    (0xff, 0x80, 0, 0x80)
  );
  if tier().is_miri() {
    return;
  }
  let mut buf = vec![0; 8192];
  assert_eq!(Wide::MAX.checked_copy_le_bytes_to(&mut buf), Some(8192));
  assert!(buf.iter().all(|&byte| byte == 0xff));
  let mut buf = vec![0xa5; 8192];
  assert_eq!(WideInt::MIN.checked_copy_be_bytes_to(&mut buf), Some(8192));
  assert_eq!(
    (buf[0], buf[1..].iter().all(|&byte| byte == 0)),
    (0x80, true)
  );
  let mut buf = vec![0xa5; 8192];
  assert_eq!(
    IntegerBytes::checked_copy_le_bytes_to(&WideInt::MIN, &mut buf),
    Some(8192)
  );
  assert_eq!(
    (buf[8191], buf[..8191].iter().all(|&byte| byte == 0)),
    (0x80, true)
  );
  let mut short = vec![0xa5; 8191];
  assert_eq!(
    (
      Wide::MAX.checked_copy_le_bytes_to(&mut short),
      IntegerBytes::checked_copy_be_bytes_to(&WideInt::MIN, &mut short),
    ),
    (None, None)
  );
  assert!(short.iter().all(|&byte| byte == 0xa5));
}

/// `copy_le_bytes_to` and `copy_be_bytes_to` into a buffer shorter than the bytes of the value, and
/// `byte` at an index of the number of bytes or more, panic in every build with their messages and
/// report the caller, through the inherent `const fn`s and `IntegerBytes`, leaving the buffer
/// unchanged, in native integers and arrays, at zero bits, where every index panics, and above 8192
/// bits.
#[test]
fn short_buffers_and_byte_indexes_panic() {
  let le = "copy_le_bytes_to: the buffer is shorter than the bytes of the value".to_string();
  let be = "copy_be_bytes_to: the buffer is shorter than the bytes of the value".to_string();
  let index = "byte index out of range".to_string();
  // The copies of `$x`, a value of `$ty`, to a buffer one byte shorter than its bytes, and its byte
  // at its number of bytes, one more and `usize::MAX`.
  macro_rules! panics {
    ($ty:ty, $x:expr) => {{
      let x: $ty = $x;
      let mut buf = vec![0xa5; <$ty>::BYTES - 1];
      panics_here!(x.copy_le_bytes_to(&mut buf), le);
      panics_here!(x.copy_be_bytes_to(&mut buf), be);
      panics_here!(IntegerBytes::copy_le_bytes_to(&x, &mut buf), le);
      panics_here!(IntegerBytes::copy_be_bytes_to(&x, &mut buf), be);
      assert_eq!(buf, vec![0xa5; <$ty>::BYTES - 1]);
      for k in [0, 1, usize::MAX - <$ty>::BYTES] {
        panics_here!(x.byte(<$ty>::BYTES + k), index);
        panics_here!(IntegerBytes::byte(&x, <$ty>::BYTES + k), index);
      }
    }};
  }
  type Bytes = Int<U24, [u8; 3]>;
  type Wide = Uint<U16384, [u64; 256]>;
  panics!(u7, u7::MAX);
  panics!(Bytes, Bytes::MIN);
  panics!(i256, i256::MIN);
  panics!(Wide, Wide::MAX);
  for k in [0, 1, usize::MAX] {
    panics_here!(u0::ZERO.byte(k), index);
    panics_here!(IntegerBytes::byte(&Int::<U0, [u16; 1]>::ZERO, k), index);
  }
}

/// `strict_from_le_slice` and `strict_from_be_slice`, inherent and through `IntegerBytes`, panic in
/// every build on bytes that hold an integer outside the range of the width, with their messages,
/// and report the caller: above the range and below it, in native integers and arrays, and at zero
/// bits.
#[test]
fn strict_slices_panic_outside_the_range() {
  let le = "strict_from_le_slice: the bytes hold a value outside the range of the integer";
  let be = "strict_from_be_slice: the bytes hold a value outside the range of the integer";
  let (le, be) = (le.to_string(), be.to_string());
  type I24 = Int<U24, [u8; 3]>;
  type W = u256;
  panics_here!(u7::strict_from_le_slice(&[0x80]), le);
  panics_here!(u7::strict_from_be_slice(&[1, 0]), be);
  panics_here!(<u7 as IntegerBytes>::strict_from_le_slice(&[0, 1]), le);
  panics_here!(<u7 as IntegerBytes>::strict_from_be_slice(&[0xff]), be);
  panics_here!(I24::strict_from_le_slice(&[0xff, 0xff, 0x7f, 0xff]), le);
  panics_here!(<I24 as IntegerBytes>::strict_from_be_slice(&[1; 4]), be);
  panics_here!(i256::strict_from_be_slice(&[0x80; 33]), be);
  panics_here!(<W as IntegerBytes>::strict_from_le_slice(&[1; 33]), le);
  panics_here!(u0::strict_from_le_slice(&[1]), le);
  panics_here!(<i0 as IntegerBytes>::strict_from_be_slice(&[0x80]), be);
}

/// The byte methods in constant contexts, in native integers and arrays, at zero bits and at one
/// signed bit: the arrays and the conversions from them, the slices in every form, a copy to a
/// local buffer, the byte at an index and the byte order.
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
  const FORMS: ((I12, bool), I12, I12, (u7, bool), u7, u7) = (
    I12::overflowing_from_le_slice(&[0xff, 0x0f]),
    I12::saturating_from_be_slice(&[0x80, 0]),
    I12::strict_from_le_slice(&[0x80]),
    u7::overflowing_from_be_slice(&[1, 0]),
    u7::saturating_from_le_slice(&[0x80]),
    u7::strict_from_be_slice(&[0, 0x7f]),
  );

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
  // 4095 wraps to -1 in 12 bits, -32768 saturates to `MIN`, and 256 wraps to zero in 7 bits.
  assert_eq!(
    FORMS,
    (
      (I12::new(-1), true),
      I12::MIN,
      I12::new(-128),
      (u7::ZERO, true),
      u7::MAX,
      u7::MAX
    )
  );
}

/// Zero bits hold only zero: no bytes, an array of none, from which zero reads back, a reversal
/// that gives zero, copies that write nothing, empty and zero slices that read as zero, and slices
/// of another value, 1 or -1, that do not fit and saturate to zero, in native integers and arrays.
/// One signed bit holds -1 and 0: -1 is `[0xff]`, `[0x01]` holds no value of it, the wrapping and
/// overflowing slices reduce modulo 2 into the range, and the saturating ones give -1 below it and
/// zero above it.
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
        assert_eq!(
          (
            T::overflowing_from_be_slice(slice),
            T::saturating_from_le_slice(slice),
            T::strict_from_be_slice(slice),
          ),
          ((zero, false), zero, zero),
          "{slice:x?}"
        );
      }
      for slice in [&[1][..], &[0xff][..], &[0, 0x80][..]] {
        assert_eq!(
          (T::checked_from_le_slice(slice), T::wrapping_from_le_slice(slice)),
          (None, zero),
          "{slice:x?}"
        );
        // Zero bits hold only zero, which is both bounds.
        assert_eq!(
          (
            T::overflowing_from_le_slice(slice),
            T::saturating_from_be_slice(slice),
          ),
          ((zero, true), zero),
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
      // 1 and 2 are above the range, whose largest value is zero, and -128 below it.
      for (slice, overflowing, saturated) in [
        (&[0xff, 0xff][..], (min, false), min),
        (&[0x01][..], (min, true), zero),
        (&[0x02][..], (zero, true), zero),
        (&[0x80][..], (zero, true), min),
        (&[][..], (zero, false), zero),
      ] {
        assert_eq!(
          (
            T::overflowing_from_le_slice(slice),
            T::saturating_from_le_slice(slice),
          ),
          (overflowing, saturated),
          "{slice:x?}"
        );
      }
      assert_eq!(
        (T::strict_from_le_slice(&[0xff]), T::strict_from_be_slice(&[0, 0])),
        (min, zero)
      );
      assert_eq!((min.byte(0), zero.byte(0)), (0xff, 0));
    })+};
  }
  one_signed_bit!(i1, Int<U1, [u16; 1]>, Int<U1, [u64; 1]>);
}

/// `IntegerBytes` needs only the storage trait, `UintRepr` or `IntRepr`, and `IntegerByteOrder` a
/// storage that is also `Clone`, not one that holds the value inline, and `ByteArray` and
/// `FromByteArray` add the bound of the array, whose `Bytes` reads as a slice: code bounded by them
/// gives the results of the inherent `const fn`s, in native integers and arrays.
#[test]
fn byte_traits_need_no_inline_storage() {
  fn unsigned<B: FitsBits<R>, R: UintRepr>(a: &Uint<B, R>) -> String {
    let mut buf = [0xa5; 34];
    let written = a.copy_le_bytes_to(&mut buf);
    format!(
      "{:?}",
      (
        a.byte(0),
        Uint::<B, R>::checked_from_be_slice(&[1, 2]),
        Uint::<B, R>::wrapping_from_le_slice(&[0xff; 40]),
        Uint::<B, R>::saturating_from_le_slice(&[0xff; 40]),
        written,
        buf,
      )
    )
  }

  fn signed<B: FitsBits<R> + WholeBytes, R: IntRepr + Clone>(a: &Int<B, R>) -> String {
    format!(
      "{:?}",
      (a.swap_bytes(), a.to_be(), Int::<B, R>::from_le(a.to_le()))
    )
  }

  fn top<T: ByteArray>(a: &T) -> u8 {
    a.to_be_bytes().as_ref()[0]
  }

  // The value with the lowest bit of its first byte flipped.
  fn flipped<T: FromByteArray>(a: &T) -> T {
    let mut bytes = a.to_le_bytes();
    bytes.as_mut()[0] ^= 1;
    T::from_le_bytes(bytes)
  }

  // The inherent results that the generic code must give.
  macro_rules! unsigned_inherent {
    ($ty:ty, $a:expr) => {{
      let a: $ty = $a;
      let mut buf = [0xa5; 34];
      let written = a.copy_le_bytes_to(&mut buf);
      format!(
        "{:?}",
        (
          a.byte(0),
          <$ty>::checked_from_be_slice(&[1, 2]),
          <$ty>::wrapping_from_le_slice(&[0xff; 40]),
          <$ty>::saturating_from_le_slice(&[0xff; 40]),
          written,
          buf,
        )
      )
    }};
  }
  macro_rules! signed_inherent {
    ($ty:ty, $a:expr) => {{
      let a: $ty = $a;
      format!(
        "{:?}",
        (a.swap_bytes(), a.to_be(), <$ty>::from_le(a.to_le()))
      )
    }};
  }
  type Bytes = Int<U24, i32>;
  let four = u256::from_limbs([4, 0, 0, 0]);
  assert_eq!(unsigned(&u7::new(5)), unsigned_inherent!(u7, u7::new(5)));
  assert_eq!(unsigned(&four), unsigned_inherent!(u256, four));
  assert_eq!(
    signed(&Bytes::new(-2)),
    signed_inherent!(Bytes, Bytes::new(-2))
  );
  assert_eq!(signed(&i256::MIN), signed_inherent!(i256, i256::MIN));
  assert_eq!(
    (top(&Uint::<U24>::new(0x01_0203)), top(&i7::new(-1))),
    (1, 0xff)
  );
  assert_eq!(
    flipped(&Uint::<U24>::new(0x01_0203)),
    Uint::<U24>::new(0x01_0202)
  );
  assert_eq!(
    flipped(&Int::<U16, [u8; 2]>::from_limbs([0, 0x80])),
    Int::<U16, [u8; 2]>::from_limbs([1, 0x80])
  );
}

/// The slices at which code over the width reads its values.
const OVER_THE_WIDTH: [&[u8]; 3] = [&[], &[0x80], &[0xff, 0x7f, 1]];

/// The bytes of `a` and `b` through `IntegerBytes` and `ByteArray`, as text, in code over
/// `B: BitWidth` with the bound of the arrays, which every width up to 8192 bits meets.
fn bytes_over_the_width<B: BitWidth>(a: Uint<B>, b: Int<B>) -> Vec<String>
where
  Uint<B>: ByteArray,
  Int<B>: ByteArray,
{
  let mut buf = [0xa5; 130];
  let written = (
    a.copy_be_bytes_to(&mut buf),
    b.checked_copy_le_bytes_to(&mut buf[1..]),
  );
  vec![
    format!(
      "{:?}",
      (
        a.to_le_bytes().as_ref(),
        a.to_be_bytes().as_ref(),
        b.to_le_bytes().as_ref(),
        b.to_ne_bytes().as_ref(),
      )
    ),
    format!(
      "{:?}",
      (
        Uint::<B>::checked_from_le_bytes(a.to_le_bytes()),
        Int::<B>::checked_from_be_bytes(b.to_be_bytes()),
      )
    ),
    format!(
      "{:?}",
      OVER_THE_WIDTH.map(|slice| (
        Uint::<B>::checked_from_le_slice(slice),
        Int::<B>::wrapping_from_be_slice(slice)
      ))
    ),
    format!("{written:?} {buf:?}"),
    format!(
      "{:?}",
      (0..Uint::<B>::BYTES)
        .map(|index| a.byte(index))
        .collect::<Vec<_>>()
    ),
  ]
}

/// The byte order of `a` and `b` through `IntegerByteOrder`, as text, in code over `B: BitWidth`
/// at a width of whole bytes.
fn order_over_the_width<B: BitWidth + WholeBytes>(a: Uint<B>, b: Int<B>) -> String {
  format!(
    "{:?}",
    (
      a.swap_bytes(),
      a.to_be(),
      b.to_le(),
      Uint::<B>::from_be(a),
      Int::<B>::from_le(b),
    )
  )
}

/// Code over `B: BitWidth` reaches the bytes of `Uint<B>` and `Int<B>` through `IntegerBytes`,
/// their arrays through `ByteArray` with the bound of the arrays, and at a width of whole bytes
/// their byte order through `IntegerByteOrder` with `B: WholeBytes`: they give the results of the
/// inherent `const fn`s of the default storages, at widths of each kind of default storage, at
/// zero, the limits and values that wrap from them. Under Miri, at 0, 7, 65 and 129 bits.
#[test]
fn bytes_over_the_width_match_the_default_storages() {
  macro_rules! agrees {
    ($($width:ident),+ $(,)?) => {$({
      type U = Uint<$width>;
      type I = Int<$width>;
      for (a, b) in [
        (U::ZERO, I::ZERO),
        (U::MAX, I::MIN),
        (U::MAX.wrapping_mul(&U::MAX), I::MAX),
        (U::MAX.wrapping_add(&U::MAX), I::MIN.wrapping_sub(&I::MAX)),
      ] {
        let mut buf = [0xa5; 130];
        let written = (
          a.copy_be_bytes_to(&mut buf),
          b.checked_copy_le_bytes_to(&mut buf[1..]),
        );
        let inherent = vec![
          format!(
            "{:?}",
            (
              &a.to_le_bytes()[..],
              &a.to_be_bytes()[..],
              &b.to_le_bytes()[..],
              &b.to_ne_bytes()[..],
            )
          ),
          format!(
            "{:?}",
            (
              U::checked_from_le_bytes(a.to_le_bytes()),
              I::checked_from_be_bytes(b.to_be_bytes()),
            )
          ),
          format!(
            "{:?}",
            OVER_THE_WIDTH.map(|slice| (
              U::checked_from_le_slice(slice),
              I::wrapping_from_be_slice(slice)
            ))
          ),
          format!("{written:?} {buf:?}"),
          format!(
            "{:?}",
            (0..U::BYTES).map(|index| a.byte(index)).collect::<Vec<_>>()
          ),
        ];
        assert_eq!(bytes_over_the_width::<$width>(a, b), inherent, "{a} and {b}");
      }
    })+};
  }
  // The byte order at a width of whole bytes.
  macro_rules! order_agrees {
    ($($width:ident),+ $(,)?) => {$({
      type U = Uint<$width>;
      type I = Int<$width>;
      for (a, b) in [(U::ZERO, I::ZERO), (U::MAX, I::MIN), (U::MAX.wrapping_mul(&U::MAX), I::MAX)] {
        let inherent = format!(
          "{:?}",
          (a.swap_bytes(), a.to_be(), b.to_le(), U::from_be(a), I::from_le(b))
        );
        assert_eq!(order_over_the_width::<$width>(a, b), inherent, "{a} and {b}");
      }
    })+};
  }

  agrees!(U0, U7, U65, U129);
  order_agrees!(U0, U8, U128);
  if !tier().is_miri() {
    agrees!(U1, U8, U64, U128, U200, U1024);
    order_agrees!(U64, U200, U1024);
  }
}

/// The byte order of the non-zero integers of the native widths, in their native `NonZero`
/// storages, gives what the byte order of the native integers gives, since the byte methods of the
/// native `NonZero` integers are unstable: at the structured patterns of each width that are not
/// zero, through the inherent `const fn`s and `IntegerByteOrder`, or under Miri at one value of
/// each storage.
#[test]
fn nonzero_byte_order_matches_the_native_integers() {
  macro_rules! native {
    ($($width:ident: $uint:ident, $int:ident);+ $(;)?) => {$({
      type U = NonZeroUint<$width, core::num::NonZero<$uint>>;
      type I = NonZeroInt<$width, core::num::NonZero<$int>>;
      let patterns = if tier().is_miri() {
        vec![0x0f0e_0d0c_0b0a_0908_0706_0504_0302_0180]
      } else {
        bigint::structured_patterns($uint::BITS)
      };
      for pattern in patterns {
        let a = pattern as $uint;
        let b = a.cast_signed();
        let (Some(x), Some(y)) = (
          U::new(Uint::<$width, $uint>::new(a)),
          I::new(Int::<$width, $int>::new(b)),
        ) else {
          continue;
        };
        let unsigned = [x.swap_bytes(), x.to_be(), x.to_le(), U::from_be(x), U::from_le(x)];
        let signed = [y.swap_bytes(), y.to_be(), y.to_le(), I::from_be(y), I::from_le(y)];
        assert_eq!(
          unsigned.map(|z| z.get().value()),
          [a.swap_bytes(), a.to_be(), a.to_le(), $uint::from_be(a), $uint::from_le(a)],
          "{a:#x}"
        );
        assert_eq!(
          signed.map(|z| z.get().value()),
          [b.swap_bytes(), b.to_be(), b.to_le(), $int::from_be(b), $int::from_le(b)],
          "{b:#x}"
        );
        assert_eq!(
          (order_through_traits(&x), order_through_traits(&y)),
          (unsigned, signed),
          "{a:#x}"
        );
      }
    })+};
  }

  native!(
    U8: u8, i8;
    U16: u16, i16;
    U32: u32, i32;
    U64: u64, i64;
    U128: u128, i128;
  );
}

/// The byte order of the non-zero integers in constant contexts, in a native storage and in an
/// array.
#[test]
fn nonzero_byte_order_in_constant_contexts() {
  type Native = NonZeroUint<U16>;
  type Bytes = NonZeroInt<U24, [u8; 3]>;
  const NATIVE: (Native, Native) = {
    let x = Native::new(Uint::<U16>::new(0x0102)).expect("not zero");
    (x.swap_bytes(), Native::from_le(x))
  };
  const BYTES: (Bytes, Bytes) = {
    let x = Bytes::new(Int::<U24, [u8; 3]>::from_limbs([0x80, 0, 0x01])).expect("not zero");
    (x.swap_bytes(), x.to_be())
  };

  let nonzero = |value| Native::new(Uint::<U16>::new(value)).expect("not zero");
  let (value, swapped) = (nonzero(0x0102), nonzero(0x0201));
  let in_little = if cfg!(target_endian = "big") {
    swapped
  } else {
    value
  };
  assert_eq!(NATIVE, (swapped, in_little));
  let nonzero = |limbs| Bytes::new(Int::<U24, [u8; 3]>::from_limbs(limbs)).expect("not zero");
  let (value, swapped) = (nonzero([0x80, 0, 0x01]), nonzero([0x01, 0, 0x80]));
  let in_big = if cfg!(target_endian = "big") {
    value
  } else {
    swapped
  };
  assert_eq!(BYTES, (swapped, in_big));
}
