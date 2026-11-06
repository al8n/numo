//! The conversions of `Uint` and `Int` from slices of limbs: the checked, wrapping, overflowing,
//! saturating and strict ones, through the inherent `const fn`s against the bit model, and through
//! `ArrayInteger` and from the slices of bytes of the same bytes against the inherent `const fn`s,
//! in two limbs of each limb type at the widths of one and two limbs and next to them, and in no
//! limbs; the panics of the strict one; code generic over the limb type; and constant contexts.

use core::ops::{Not, Shl, Sub};

use typenum::{U12, U15, U17, U31, U33, U255};

use super::*;
use crate::{ArrayInteger, InlineIntRepr, InlineUintRepr, IntegerBytes};

/// What the conversions from a slice give: through the inherent `const fn`s in `limb_reads!`,
/// through `ArrayInteger` in [`limbs_through_traits`] and through `IntegerBytes` in
/// [`bytes_through_traits`], which must agree, and from the bit model.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Reads<T> {
  /// The checked conversion.
  checked: Option<T>,
  /// The wrapping conversion.
  wrapping: T,
  /// The overflowing conversion.
  overflowing: (T, bool),
  /// The saturating conversion.
  saturating: T,
  /// The strict conversion where the checked one returns a value, and `None` where it panics.
  strict: Option<T>,
}

/// The [`Reads`] of `$ty` from the limbs of `$slice` through the inherent `const fn`s.
macro_rules! limb_reads {
  ($ty:ty, $slice:expr) => {{
    let slice: &[_] = $slice;
    let checked = <$ty>::checked_from_limbs_slice(slice);
    Reads {
      checked,
      wrapping: <$ty>::wrapping_from_limbs_slice(slice),
      overflowing: <$ty>::overflowing_from_limbs_slice(slice),
      saturating: <$ty>::saturating_from_limbs_slice(slice),
      strict: checked.map(|_| <$ty>::strict_from_limbs_slice(slice)),
    }
  }};
}

/// The [`Reads`] of `T` from the limbs of `slice` through `ArrayInteger`.
fn limbs_through_traits<T: ArrayInteger + Copy>(slice: &[T::Limb]) -> Reads<T> {
  let checked = T::checked_from_limbs_slice(slice);
  Reads {
    checked,
    wrapping: T::wrapping_from_limbs_slice(slice),
    overflowing: T::overflowing_from_limbs_slice(slice),
    saturating: T::saturating_from_limbs_slice(slice),
    strict: checked.map(|_| T::strict_from_limbs_slice(slice)),
  }
}

/// The [`Reads`] of `T` from `bytes`, least significant first, and from them in reverse order, most
/// significant first, through `IntegerBytes`.
fn bytes_through_traits<T: IntegerBytes + Copy>(bytes: &[u8]) -> [Reads<T>; 2] {
  let reversed: Vec<u8> = bytes.iter().rev().copied().collect();
  let (little, big) = (
    T::checked_from_le_slice(bytes),
    T::checked_from_be_slice(&reversed),
  );
  [
    Reads {
      checked: little,
      wrapping: T::wrapping_from_le_slice(bytes),
      overflowing: T::overflowing_from_le_slice(bytes),
      saturating: T::saturating_from_le_slice(bytes),
      strict: little.map(|_| T::strict_from_le_slice(bytes)),
    },
    Reads {
      checked: big,
      wrapping: T::wrapping_from_be_slice(&reversed),
      overflowing: T::overflowing_from_be_slice(&reversed),
      saturating: T::saturating_from_be_slice(&reversed),
      strict: big.map(|_| T::strict_from_be_slice(&reversed)),
    },
  ]
}

/// Checks the conversions of `$ty`, an array of `$n` limbs of type `$limb`, from the slices of
/// `bigint::limb_slices` of `$p`, a pattern of the width of `$ty`, read in two's complement if
/// `$signed`: through the inherent `const fn`s against the bit model, where the wrapping one returns
/// the value of the checked one wherever that returns one and the saturating one the bound of the
/// range that the integer passes; and through `ArrayInteger` and from the bytes of the slice, in
/// both orders, against the inherent `const fn`s.
macro_rules! check_limb_slices {
  ($ty:ty, $limb:ty, $n:expr, $p:expr, $signed:expr) => {{
    let (p, signed): (&[bool], bool) = ($p, $signed);
    let width = model::width(p);
    for slice in bigint::limb_slices::<$limb>(p, signed, $n) {
      let q = bigint::pattern_of_limbs(&slice, <$limb>::BITS * slice.len() as u32);
      let checked = model::fit(&q, width, signed).map(|r| <$ty>::from_pattern(&r));
      let wrapped = <$ty>::from_pattern(&model::wrap(&q, width, signed));
      let negative = signed && model::sign(&q);
      let bound = || <$ty>::from_pattern(&bound_pattern(width, signed, negative));
      let expected = Reads {
        checked,
        wrapping: wrapped,
        overflowing: (wrapped, checked.is_none()),
        saturating: checked.unwrap_or_else(bound),
        strict: checked,
      };
      let inherent = limb_reads!($ty, &slice);
      assert_eq!(inherent, expected, "{slice:x?} at {width} bits");
      if let Some(value) = inherent.checked {
        assert_eq!(inherent.wrapping, value, "{slice:x?} at {width} bits");
      }
      assert_eq!(
        limbs_through_traits::<$ty>(&slice),
        inherent,
        "{slice:x?} at {width} bits"
      );
      assert_eq!(
        bytes_through_traits::<$ty>(&bigint::bytes_of_limbs(&slice)),
        [inherent; 2],
        "{slice:x?} at {width} bits"
      );
    }
  }};
}

/// The patterns of `bits` bits at which the conversions from slices into `N` limbs of type `L` are
/// checked: those of `bigint::bit_patterns_by` at the reduced indexes, cut to the width, or under
/// Miri those of `model::few_patterns`.
fn limb_patterns<L, const N: usize>(bits: u32) -> Vec<Vec<bool>>
where
  L: bigint::Limb + Not<Output = L> + Shl<u32, Output = L> + Sub<Output = L>,
{
  let mut patterns: Vec<Vec<bool>> = if tier().is_miri() {
    model::few_patterns(bits, L::BITS)
  } else {
    bigint::bit_patterns_by::<L, N>(&[bits], true)
      .iter()
      .map(|limbs| bigint::pattern_of_limbs(limbs, bits))
      .collect()
  };
  patterns.sort_unstable();
  patterns.dedup();
  patterns
}

/// Checks the conversions from slices of limbs of `Uint` and `Int` in `$n` limbs of type `$limb` at
/// each width of the list, written `Bits, bits;`, at the patterns of [`limb_patterns`], each
/// storage in a function of its own, whose frame holds the values of that storage alone: the
/// function is `#[inline(never)]`, since an optimized build would inline a function that has one
/// call site, which would put every storage in the frame of the test.
macro_rules! limbs_agree {
  ($limb:ident, $n:literal; $($width:ty, $bits:literal);+ $(;)?) => {$({
    #[inline(never)]
    fn storage() {
      type U = Uint<$width, [$limb; $n]>;
      type I = Int<$width, [$limb; $n]>;
      for p in &limb_patterns::<$limb, $n>($bits) {
        check_limb_slices!(U, $limb, $n, p, false);
        check_limb_slices!(I, $limb, $n, p, true);
      }
    }
    storage();
  })+};
}

/// The conversions from slices of limbs into two limbs of each limb type, at zero bits, one, one
/// less than a limb, a limb, one more, one less than two limbs and two limbs, and into no limbs,
/// against the bit model and the slices of bytes. Under Miri, into no limbs and into two limbs of
/// 8 and 64 bits at a limb and two limbs.
#[test]
fn limb_slices_match_the_bit_model() {
  limbs_agree!(u64, 0; U0, 0);
  limbs_agree!(u8, 2; U8, 8; U16, 16);
  limbs_agree!(u64, 2; U64, 64; U128, 128);
  if tier().is_miri() {
    return;
  }
  limbs_agree!(u8, 2; U0, 0; U1, 1; U7, 7; U9, 9; U15, 15);
  limbs_agree!(u16, 2; U0, 0; U1, 1; U15, 15; U16, 16; U17, 17; U31, 31; U32, 32);
  limbs_agree!(u32, 2; U0, 0; U1, 1; U31, 31; U32, 32; U33, 33; U63, 63; U64, 64);
  limbs_agree!(u64, 2; U0, 0; U1, 1; U63, 63; U65, 65; U127, 127);
  limbs_agree!(u128, 2; U0, 0; U1, 1; U127, 127; U128, 128; U129, 129; U255, 255; U256, 256);
}

/// `strict_from_limbs_slice`, inherent and through `ArrayInteger`, panics in every build on limbs
/// that hold an integer outside the range of the width, with its message, and reports the caller:
/// in limbs of 8 and 64 bits, above the range and below it, and at zero bits.
#[test]
fn strict_limb_slices_panic_outside_the_range() {
  let m = "strict_from_limbs_slice: the limbs hold a value outside the range of the integer";
  let m = m.to_string();
  type U = Uint<U12, [u8; 2]>;
  type I = Int<U12, [u8; 2]>;
  type W = i256;
  panics_here!(U::strict_from_limbs_slice(&[0, 0x10]), m);
  panics_here!(<U as ArrayInteger>::strict_from_limbs_slice(&[0, 0, 1]), m);
  panics_here!(I::strict_from_limbs_slice(&[0xff, 0xf7]), m);
  panics_here!(<I as ArrayInteger>::strict_from_limbs_slice(&[0, 0x08]), m);
  panics_here!(u256::strict_from_limbs_slice(&[0, 0, 0, 0, 1]), m);
  panics_here!(<W as ArrayInteger>::strict_from_limbs_slice(&[1; 5]), m);
  panics_here!(Int::<U0, [u8; 0]>::strict_from_limbs_slice(&[0x80]), m);
}

/// The checked and the overflowing conversions from a slice into `Uint<B, [L; N]>`, in code
/// generic over the limb type, whose bounds are those of an array storage alone, through
/// `ArrayInteger`.
fn uint_from_limbs<B, L, const N: usize>(limbs: &[L]) -> (Option<Uint<B, [L; N]>>, bool)
where
  B: FitsBits<[L; N]>,
  [L; N]: InlineUintRepr,
{
  let (_, overflow): (Uint<B, [L; N]>, bool) = ArrayInteger::overflowing_from_limbs_slice(limbs);
  (ArrayInteger::checked_from_limbs_slice(limbs), overflow)
}

/// The wrapping and the saturating conversions from a slice into `Int<B, [L; N]>`, in code generic
/// over the limb type, as for [`uint_from_limbs`].
fn int_from_limbs<B, L, const N: usize>(limbs: &[L]) -> (Int<B, [L; N]>, Int<B, [L; N]>)
where
  B: FitsBits<[L; N]>,
  [L; N]: InlineIntRepr,
{
  (
    ArrayInteger::wrapping_from_limbs_slice(limbs),
    ArrayInteger::saturating_from_limbs_slice(limbs),
  )
}

/// Code generic over the limb type and the number of limbs reaches the conversions from slices
/// through `ArrayInteger` with the bounds of an array storage alone: its `Limb` is the limb type of
/// the array.
#[test]
fn limb_slices_in_code_generic_over_the_limb_type() {
  type U12x2 = Uint<U12, [u8; 2]>;
  type I12x2 = Int<U12, [u8; 2]>;
  type I129x2 = Int<U129, [u128; 2]>;
  assert_eq!(
    uint_from_limbs::<U12, u8, 2>(&[0xff, 0x0f, 0]),
    (Some(U12x2::MAX), false)
  );
  assert_eq!(uint_from_limbs::<U12, u8, 2>(&[0, 0x10]), (None, true));
  assert_eq!(
    uint_from_limbs::<U200, u64, 4>(&[5, 1]),
    (
      Uint::<U200, [u64; 4]>::checked_from_limbs([5, 1, 0, 0]),
      false
    )
  );
  // -32768 and 2^256 are outside the range: they wrap to zero and saturate to the bounds.
  assert_eq!(
    int_from_limbs::<U12, u8, 2>(&[0, 0x80]),
    (I12x2::ZERO, I12x2::MIN)
  );
  assert_eq!(
    int_from_limbs::<U129, u128, 2>(&[0, 0, 1]),
    (I129x2::ZERO, I129x2::MAX)
  );
}

/// The conversions from slices of limbs in constant contexts, unsigned and in two's complement, of
/// a slice shorter than the array, one longer and an empty one, and at zero bits.
#[test]
fn limb_slices_in_constant_contexts() {
  type U12x2 = Uint<U12, [u8; 2]>;
  type I12x2 = Int<U12, [u8; 2]>;
  type U0x0 = Uint<U0, [u64; 0]>;
  const UNSIGNED: (Option<U12x2>, U12x2, (U12x2, bool), U12x2, U12x2) = (
    U12x2::checked_from_limbs_slice(&[0xff, 0x0f, 0, 0]),
    U12x2::wrapping_from_limbs_slice(&[0xff, 0xff]),
    U12x2::overflowing_from_limbs_slice(&[1, 0x10]),
    U12x2::saturating_from_limbs_slice(&[0, 0, 1]),
    U12x2::strict_from_limbs_slice(&[0x34]),
  );
  const SIGNED: (Option<I12x2>, I12x2, (I12x2, bool), I12x2, I12x2) = (
    I12x2::checked_from_limbs_slice(&[0x80]),
    I12x2::wrapping_from_limbs_slice(&[0xff, 0x0f]),
    I12x2::overflowing_from_limbs_slice(&[]),
    I12x2::saturating_from_limbs_slice(&[0, 0xf0, 0xff]),
    I12x2::strict_from_limbs_slice(&[0, 0xf8, 0xff, 0xff]),
  );
  const ZERO_BITS: (Option<U0x0>, U0x0) = (
    U0x0::checked_from_limbs_slice(&[0, 0]),
    U0x0::saturating_from_limbs_slice(&[1]),
  );

  let u12 = |limbs| U12x2::checked_from_limbs(limbs).expect("in range");
  let i12 = |limbs| I12x2::checked_from_limbs(limbs).expect("in range");
  assert_eq!(
    UNSIGNED,
    (
      Some(U12x2::MAX),
      U12x2::MAX,
      (u12([1, 0]), true),
      U12x2::MAX,
      u12([0x34, 0])
    )
  );
  assert_eq!(
    SIGNED,
    (
      Some(i12([0x80, 0xff])),
      i12([0xff, 0xff]),
      (I12x2::ZERO, false),
      I12x2::MIN,
      I12x2::MIN
    )
  );
  assert_eq!(ZERO_BITS, (Some(U0x0::ZERO), U0x0::ZERO));
}
