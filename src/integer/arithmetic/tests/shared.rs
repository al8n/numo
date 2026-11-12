//! The checks that every heap storage of `Uint`, `Int`, `NonZeroUint` and `NonZeroInt` shares,
//! generic over the pointer through [`Heap`], with the oracle its inline twin, the array that it
//! holds; and the shared storages `Rc<[L; N]>`, `Arc<[L; N]>` and `triomphe::Arc<[L; N]>`, against
//! their twins: every row of the storage cores, the rows that never allocate keeping the pointer
//! and the others giving a value of its own to a new allocation; values that every owner keeps;
//! each way of building one; every trait and operator but the compound assignments, which a shared
//! storage does not have; the non-zero integers; the layout and the auto traits; serde; and the
//! zero of `triomphe::Arc`, which is moved into its allocation.

use core::{
  cmp::Ordering,
  fmt,
  hash::{Hash, Hasher},
  ops::Shl,
};
#[cfg(any(feature = "std", feature = "alloc"))]
use std::rc::Rc;
#[cfg(all(any(feature = "std", feature = "alloc"), target_has_atomic = "ptr"))]
use std::sync::Arc;

#[cfg(feature = "serde")]
use serde_core::{Serialize, de::DeserializeOwned};
#[cfg(all(feature = "triomphe", target_has_atomic = "ptr"))]
use triomphe::Arc as TriompheArc;
use typenum::{U20, U300};

use super::*;
use crate::{
  ArrayInteger, BoundedCastFrom, CheckedCastFrom, FillsRepr, FixedNonZero, FromFloatError,
  FromLimbs, IntRepr, IntegerBytes, IntegerFloat, NonZeroInt, NonZeroIntRepr, NonZeroLimits,
  NonZeroUint, NonZeroUintRepr, SignedNonZero, UintRepr, UnsignedNonZero, WidenFrom,
  bigint::floats,
  int::float::{FloatFormat, FloatParts, float_parts_f64},
  layout::Token,
  width::{ArrayLimb, ArrayRepr, IntCore, ReprLimb, UintCore, View},
};

/// A heap storage of the array `[L; N]`, as the checks build and read it: `Box`, `Rc`, `Arc` and
/// `triomphe::Arc`. The supertraits are the storage traits of the pointer, whose associated
/// storages are the pointer itself, as those of the array are the array.
pub(super) trait Heap<L, const N: usize>:
  Clone
  + Debug
  + Unbox<Inline = [L; N]>
  + ReprLimb<Limb = L>
  + UintRepr<Signed = Self, NonZero = Self>
  + IntRepr<Unsigned = Self, NonZero = Self>
  + UintCore<Inline = [L; N], Complement = Self, NonZeroStorage = Self>
  + IntCore<Inline = [L; N], Magnitude = Self, NonZeroStorage = Self>
  + NonZeroUintRepr<Integer = Self, Signed = Self>
  + NonZeroIntRepr<Integer = Self, Unsigned = Self>
{
  /// Whether the pointer holds its value in place: a value stored in it and the casts between the
  /// signs keep its allocation. A shared pointer never writes the value that it holds, and each of
  /// them gives a value of its own to a new allocation.
  const IN_PLACE: bool;

  /// `array` in an allocation of its own.
  fn new(array: [L; N]) -> Self;

  /// The array that the pointer holds.
  fn array(&self) -> &[L; N];
}

/// Implements [`Heap`] and [`Unbox`] for the arrays of limbs in the pointer `$pointer`, which holds
/// its value in place if `$in_place`.
macro_rules! heap {
  ($pointer:ident, $in_place:literal) => {
    impl<L: ArrayLimb + Debug, const N: usize> Heap<L, N> for $pointer<[L; N]>
    where
      [L; N]: ArrayRepr,
    {
      const IN_PLACE: bool = $in_place;

      fn new(array: [L; N]) -> Self {
        $pointer::new(array)
      }

      fn array(&self) -> &[L; N] {
        self
      }
    }

    impl<L: Copy, const N: usize> Unbox for $pointer<[L; N]> {
      type Inline = [L; N];

      fn unbox(self) -> [L; N] {
        *self
      }
    }
  };
}

#[cfg(any(feature = "std", feature = "alloc"))]
heap!(Box, true);
#[cfg(any(feature = "std", feature = "alloc"))]
heap!(Rc, false);
#[cfg(all(any(feature = "std", feature = "alloc"), target_has_atomic = "ptr"))]
heap!(Arc, false);
#[cfg(all(feature = "triomphe", target_has_atomic = "ptr"))]
heap!(TriompheArc, false);

/// A result of a row of a heap storage, as the result of the row of its array: each storage in it
/// replaced by the array that it holds.
pub(super) trait Unbox {
  /// The result of the row of the array.
  type Inline;

  /// The result with each storage replaced by its array.
  fn unbox(self) -> Self::Inline;
}

/// Implements [`Unbox`] for the results that hold no storage, which are their own.
macro_rules! unbox_as_is {
  ($($ty:ty),+ $(,)?) => {$(
    impl Unbox for $ty {
      type Inline = Self;

      fn unbox(self) -> Self {
        self
      }
    }
  )+};
}

unbox_as_is!(bool, u8, u32, u64, Ordering);

impl<T: Unbox> Unbox for Option<T> {
  type Inline = Option<T::Inline>;

  fn unbox(self) -> Option<T::Inline> {
    self.map(T::unbox)
  }
}

impl<T: Unbox, E> Unbox for Result<T, E> {
  type Inline = Result<T::Inline, E>;

  fn unbox(self) -> Result<T::Inline, E> {
    self.map(T::unbox)
  }
}

impl<A: Unbox, B: Unbox> Unbox for (A, B) {
  type Inline = (A::Inline, B::Inline);

  fn unbox(self) -> Self::Inline {
    (self.0.unbox(), self.1.unbox())
  }
}

impl<A: Unbox, B: Unbox, C: Unbox> Unbox for (A, B, C) {
  type Inline = (A::Inline, B::Inline, C::Inline);

  fn unbox(self) -> Self::Inline {
    (self.0.unbox(), self.1.unbox(), self.2.unbox())
  }
}

/// Asserts that the row `$core::$row` gives on the heap storage what it gives on the array, with
/// each storage of its result replaced by its array. A row with a receiver is written with its
/// operands as `heap => array`, then `;` and its other arguments; a row without one with its
/// arguments alone, and it is called on the heap storage `P` and on `[L; N]`.
macro_rules! row {
  ($core:ident::$row:ident($($heap:expr => $array:expr),+ $(; $($arg:expr),+)?)) => {
    assert_eq!(
      Unbox::unbox($core::$row($($heap,)+ $($($arg,)+)? Token::new())),
      $core::$row($($array,)+ $($($arg,)+)? Token::new()),
      "{} of {:?} with {:?}",
      stringify!($row),
      ($($array,)+),
      ($($(&$arg,)+)?),
    )
  };
  ($core:ident::$row:ident($($arg:expr),*)) => {
    assert_eq!(
      Unbox::unbox(<P as $core>::$row($($arg,)* Token::new())),
      <[L; N] as $core>::$row($($arg,)* Token::new()),
      "{} with {:?}",
      stringify!($row),
      ($(&$arg,)*),
    )
  };
}

/// Formats with `write`, as a format argument.
pub(super) struct Text<F>(F);

impl<F: Fn(&mut fmt::Formatter<'_>) -> fmt::Result> fmt::Display for Text<F> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    (self.0)(f)
  }
}

/// The texts that `write` gives with no flags and with the flags `+`, `#`, a fill and a width, and
/// `0` with a precision, each of which the rows of the formats honor as the integers do.
pub(super) fn texts(write: impl Fn(&mut fmt::Formatter<'_>) -> fmt::Result) -> [String; 5] {
  let text = Text(write);
  [
    format!("{text}"),
    format!("{text:+}"),
    format!("{text:#}"),
    format!("{text:_>40}"),
    format!("{text:010.3}"),
  ]
}

/// A hasher that keeps the bytes written to it, so that two writers compare by what they wrote.
#[derive(Default)]
pub(super) struct Written(Vec<u8>);

impl Hasher for Written {
  fn finish(&self) -> u64 {
    0
  }

  fn write(&mut self, bytes: &[u8]) {
    self.0.extend_from_slice(bytes);
  }
}

/// The address of `array`, which a heap storage lends to it: a row that never allocates keeps the
/// address of the array in its pointer.
pub(super) fn address<L, const N: usize>(array: &[L; N]) -> *const [L; N] {
  array
}

/// A second owner of the value of `storage`, for a shared pointer, which then keeps that value when
/// a write gives `storage` a new allocation.
pub(super) fn witness<P: Heap<L, N>, L, const N: usize>(storage: &P) -> Option<P> {
  (!P::IN_PLACE).then(|| storage.clone())
}

/// Asserts that `result`, what a write of `value` to a storage at `before` left, holds `value`: at
/// `before` where the pointer holds its value in place, and otherwise in a new allocation, while
/// `other`, the [`witness`] of the storage, keeps the old value `old` at `before`.
pub(super) fn written<P, L, const N: usize>(
  result: &P,
  value: &[L; N],
  before: *const [L; N],
  other: Option<P>,
  old: &[L; N],
  what: &str,
) where
  P: Heap<L, N>,
  L: Debug + PartialEq,
{
  match other {
    None => assert_eq!(
      (result.array(), address(result.array())),
      (value, before),
      "{what}"
    ),
    Some(other) => {
      assert_eq!(result.array(), value, "{what}");
      assert_ne!(
        address(result.array()),
        before,
        "{what} makes a new allocation"
      );
      let kept = (other.array(), address(other.array()));
      assert_eq!(kept, (old, before), "{what} leaves the other owner");
    }
  }
}

/// The patterns of `bits` bits that every sample of the rows keeps: zero, one, the top bit alone,
/// every bit but the top one and every bit, the limits of both signs among them.
pub(super) fn pattern_ends(bits: u32) -> Vec<Vec<bool>> {
  vec![
    model::pattern(bits, |_| false),
    model::pattern(bits, |i| i == 0),
    model::pattern(bits, |i| i + 1 == bits),
    model::pattern(bits, |i| i + 1 < bits),
    model::pattern(bits, |_| true),
  ]
}

/// The patterns of `bits` bits at which the rows are checked: those of [`wide_patterns`] in `K`
/// words of 64 bits, or up to 16 bits those of the values of [`small_values`], thinned to as many
/// as the tier takes, 48 in an optimized build, 12 in an unoptimized one and 8 under Miri, at a
/// stride from the first, with the [`pattern_ends`].
pub(super) fn row_patterns<const K: usize>(bits: u32, signed: bool) -> Vec<Vec<bool>> {
  let count = match tier() {
    Tier::Full => 48,
    Tier::Debug => 12,
    Tier::Miri => 8,
  };
  let patterns: Vec<Vec<bool>> = if bits <= 16 {
    small_values(bits, signed, true)
      .into_iter()
      .map(|value| pattern_of(value, bits))
      .collect()
  } else {
    wide_patterns::<K>(bits)
  };
  let step = patterns.len().div_ceil(count).max(1);
  bigint::with_ends(patterns.into_iter().step_by(step), pattern_ends(bits))
}

/// The pairs of `values` at which the rows of two operands are checked: every pair where there are
/// at most as many as the tier takes, 256 in an optimized build, 24 in an unoptimized one and 8
/// under Miri, and otherwise that many at an odd step through the pairs near their number divided
/// by the golden ratio, as [`large_pairs`] takes them, so that both operands spread over the
/// values.
pub(super) fn row_pairs<T: Copy>(values: &[T]) -> Vec<(T, T)> {
  let count = match tier() {
    Tier::Full => 256,
    Tier::Debug => 24,
    Tier::Miri => 8,
  };
  let size = values.len();
  let pairs = size * size;
  // `pairs * (sqrt(5) - 1) / 2`, from the fraction in 64 bits.
  let step = if pairs <= count {
    1
  } else {
    ((pairs as u128 * 0x9e37_79b9_7f4a_7c15) >> 64) as usize | 1
  };
  (0..count.min(pairs))
    .map(|k| {
      let pair = k * step % pairs;
      (values[pair / size], values[pair % size])
    })
    .collect()
}

/// The shift amounts and the bit indexes at which the rows of `bits` bits are checked: 0, 1, the
/// middle, `bits - 1`, `bits` and `bits + 1`.
pub(super) fn row_amounts(bits: u32) -> Vec<u32> {
  let mut amounts = vec![0, 1, bits / 2, bits.saturating_sub(1), bits, bits + 1];
  amounts.sort_unstable();
  amounts.dedup();
  amounts
}

/// The exponents of the powers and the degrees of the roots at which the rows are checked.
pub(super) const EXPONENTS: [u32; 5] = [0, 1, 2, 3, 7];

/// The parts of the floats at which `from_float` is checked at `bits` bits: zero of both signs, a
/// few halves and integers, `2^(bits - 1)` and `2^bits` and their negations where they are finite,
/// and the largest float.
pub(super) fn row_floats(bits: u32) -> Vec<FloatParts> {
  let mut values = vec![0.0, -0.0, 0.5, -1.5, 2.5, 7.0, -1e19, 1e30, f64::MAX];
  for exponent in [bits.saturating_sub(1), bits] {
    let power = floats::pow2(exponent as i32);
    values.extend([power, -power]);
  }
  values
    .into_iter()
    .filter_map(|value| float_parts_f64(value).ok())
    .collect()
}

/// The operands at which the rows of a storage of `N` limbs of type `L` are checked at `bits` bits,
/// unsigned or in two's complement.
pub(super) struct Samples<L, const N: usize> {
  /// The values of the patterns of [`row_patterns`].
  values: Vec<[L; N]>,
  /// The pairs of the values of [`row_pairs`].
  pairs: Vec<([L; N], [L; N])>,
  /// The patterns of [`pattern_ends`], from which the slices of bytes and of limbs are cut.
  ends: Vec<Vec<bool>>,
  /// The amounts of [`row_amounts`].
  amounts: Vec<u32>,
}

impl<L: bigint::Limb, const N: usize> Samples<L, N> {
  /// The operands of `bits` bits, read in two's complement if `signed`, with the patterns of
  /// [`row_patterns`] in `K` words of 64 bits.
  fn of<const K: usize>(bits: u32, signed: bool) -> Self {
    let values: Vec<[L; N]> = row_patterns::<K>(bits, signed)
      .iter()
      .map(|pattern| bigint::limbs_of_pattern(pattern, signed))
      .collect();
    Self {
      pairs: row_pairs(&values),
      values,
      ends: pattern_ends(bits),
      amounts: row_amounts(bits),
    }
  }
}

/// Checks the rows that `UintCore` and `IntCore` share, through the core `$core`, on heap storages
/// `P` of the values of `$samples`, of `$bits` bits and signed if `$signed`, against the arrays:
/// the protocol of the inline storage, the rows of one operand at each value with the amounts, the
/// exponents and the formats, the rows of two operands at each pair, the readers of bytes, of
/// limbs, of words, of text and of floats, and the conversions with the non-zero storage, which
/// keep the pointer.
macro_rules! common_rows {
  ($core:ident, $samples:expr, $bits:expr, $signed:expr) => {{
    let (samples, bits, signed): (&Samples<L, N>, u32, bool) = (&$samples, $bits, $signed);
    let storage_bits = <L as bigint::Limb>::BITS * N as u32;
    let storage_bytes = size_of::<[L; N]>();
    row!($core::zero());
    for a in &samples.values {
      let x = P::new(*a);
      // The protocol lends the array in the pointer, puts a value in an allocation of its own, and
      // stores one: in place, or for a shared pointer in a new allocation.
      assert!(core::ptr::eq($core::inline(&x, Token::new()), address(x.array())));
      assert_eq!(*<P as $core>::from_inline(*a, Token::new()).array(), *a);
      let old = samples.values[0];
      let mut target = <P as $core>::from_inline(old, Token::new());
      let (before, other) = (address(target.array()), witness(&target));
      $core::set_inline(&mut target, *a, Token::new());
      written(&target, a, before, other, &old, &format!("set_inline of {a:?}"));
      row!($core::compare_u128(&x => a; 0));
      row!($core::compare_u128(&x => a; 1));
      row!($core::compare_u128(&x => a; u128::MAX));
      row!($core::compare_i128(&x => a; -1));
      row!($core::compare_i128(&x => a; i128::MIN));
      row!($core::compare_i128(&x => a; i128::MAX));
      let own = u128::from($core::word(a, 0, Token::new()))
        | u128::from($core::word(a, 1, Token::new())) << 64;
      row!($core::compare_u128(&x => a; own));
      row!($core::compare_i128(&x => a; own as i128));
      let (mut heap, mut array) = (Written::default(), Written::default());
      $core::hash_limbs(&x, bits, &mut heap, Token::new());
      $core::hash_limbs(a, bits, &mut array, Token::new());
      assert_eq!(heap.0, array.0, "hash_limbs of {a:?}");
      assert_eq!(
        texts(|f| $core::fmt_decimal(&x, f, Token::new())),
        texts(|f| $core::fmt_decimal(a, f, Token::new())),
        "fmt_decimal of {a:?}"
      );
      for upper in [false, true] {
        assert_eq!(
          texts(|f| $core::fmt_exp(&x, upper, f, Token::new())),
          texts(|f| $core::fmt_exp(a, upper, f, Token::new())),
          "fmt_exp of {a:?}"
        );
      }
      for len in [0, 1, storage_bytes / 2, storage_bytes] {
        if len > storage_bytes {
          continue;
        }
        for big_endian in [false, true] {
          let (mut heap, mut array) = (vec![0; len], vec![0; len]);
          $core::write_bytes(&x, &mut heap, big_endian, Token::new());
          $core::write_bytes(a, &mut array, big_endian, Token::new());
          assert_eq!(heap, array, "write_bytes of {a:?} into {len} bytes");
        }
      }
      for index in [0, 1, storage_bytes / 2, storage_bytes.saturating_sub(1)] {
        if index < storage_bytes {
          row!($core::byte_at(&x => a; index));
        }
      }
      if bits % 8 == 0 {
        row!($core::swap_bytes_bits(&x => a; bits));
      }
      row!($core::is_zero(&x => a));
      row!($core::is_one(&x => a));
      for &width in &samples.amounts {
        if width <= storage_bits {
          row!($core::fits_bits(&x => a; width));
        }
      }
      row!($core::overflowing_neg_bits(&x => a; bits));
      for exp in EXPONENTS {
        row!($core::overflowing_pow_bits(&x => a; exp, bits));
        row!($core::checked_nth_root(&x => a; exp));
      }
      row!($core::checked_isqrt(&x => a));
      row!($core::checked_ilog2(&x => a));
      row!($core::checked_ilog10(&x => a));
      row!($core::not_bits(&x => a; bits));
      row!($core::count_ones_bits(&x => a; bits));
      row!($core::leading_zeros_bits(&x => a; bits));
      row!($core::trailing_zeros_bits(&x => a; bits));
      row!($core::leading_ones_bits(&x => a; bits));
      row!($core::trailing_ones_bits(&x => a; bits));
      row!($core::highest_one_bits(&x => a; bits));
      row!($core::lowest_one(&x => a));
      row!($core::isolate_highest_one_bits(&x => a; bits));
      row!($core::isolate_lowest_one_bits(&x => a; bits));
      row!($core::reverse_bits_bits(&x => a; bits));
      for &n in &samples.amounts {
        row!($core::rotate_left_bits(&x => a; n, bits));
        row!($core::rotate_right_bits(&x => a; n, bits));
        row!($core::overflowing_shl_bits(&x => a; n, bits));
        row!($core::overflowing_shr_bits(&x => a; n, bits));
        row!($core::unbounded_shl_bits(&x => a; n, bits));
        row!($core::unbounded_shr_bits(&x => a; n, bits));
        row!($core::shl_exact_bits(&x => a; n, bits));
        row!($core::shr_exact_bits(&x => a; n, bits));
        row!($core::bit(&x => a; n, bits));
        for set in [false, true] {
          row!($core::with_bit(&x => a; n, set, bits));
        }
      }
      for index in 0..=storage_bits.div_ceil(64) as usize {
        row!($core::word(&x => a; index));
      }
      for format in [FloatFormat::F64, FloatFormat::F32] {
        row!($core::float_bits(&x => a; format));
      }
      // The conversions with the non-zero storage, which is the pointer itself, keep it.
      let y = x.clone();
      let before = address(y.array());
      let nonzero = $core::to_nonzero(y, Token::new());
      assert_eq!(
        nonzero
          .as_ref()
          .map(|nonzero| (*nonzero.array(), address(nonzero.array()))),
        $core::to_nonzero(*a, Token::new()).map(|nonzero| (nonzero, before)),
        "to_nonzero of {a:?}"
      );
      if let Some(nonzero) = nonzero {
        let lent =
          <P as $core>::with_nonzero(&nonzero, |lent| address(lent.array()), Token::new());
        assert_eq!(lent, before, "with_nonzero of {a:?}");
        let integer = <P as $core>::from_nonzero(nonzero, Token::new());
        let read = (*integer.array(), address(integer.array()));
        assert_eq!(read, (*a, before), "from_nonzero of {a:?}");
        let known = $core::known_nonzero(integer, Token::new());
        let read = (*known.array(), address(known.array()));
        assert_eq!(read, (*a, before), "known_nonzero of {a:?}");
      }
      // The text of the value, in decimal, also with a digit more, which overflows the width where
      // the value is not zero.
      let decimal = |f: &mut fmt::Formatter<'_>| $core::fmt_decimal(a, f, Token::new());
      let text = format!("{x}", x = Text(decimal));
      for src in [text.clone(), format!("{text}0"), format!("+{text}")] {
        row!($core::from_str_radix_bits(src.as_bytes(), 10, bits));
      }
    }
    for (a, b) in &samples.pairs {
      let (x, y) = (P::new(*a), P::new(*b));
      row!($core::compare(&x => a, &y => b));
      row!($core::overflowing_add_bits(&x => a, &y => b; bits));
      row!($core::overflowing_sub_bits(&x => a, &y => b; bits));
      row!($core::overflowing_mul_bits(&x => a, &y => b; bits));
      row!($core::div_rem_bits(&x => a, &y => b; bits));
      row!($core::div_rem_euclid_bits(&x => a, &y => b; bits));
      row!($core::div_floor_bits(&x => a, &y => b; bits));
      row!($core::div_ceil_bits(&x => a, &y => b; bits));
      row!($core::next_multiple_of_bits(&x => a, &y => b; bits));
      row!($core::checked_div_exact_bits(&x => a, &y => b; bits));
      row!($core::checked_ilog(&x => a, &y => b));
      row!($core::midpoint(&x => a, &y => b));
      row!($core::and(&x => a, &y => b));
      row!($core::or(&x => a, &y => b));
      row!($core::xor(&x => a, &y => b));
    }
    for pattern in &samples.ends {
      for bytes in bigint::byte_slices(pattern, signed) {
        let reversed: Vec<u8> = bytes.iter().rev().copied().collect();
        for (slice, big_endian) in [(&bytes, false), (&reversed, true)] {
          let slice: &[u8] = slice;
          row!($core::checked_read_bytes(slice, big_endian, bits));
          row!($core::wrapping_read_bytes(slice, big_endian, bits));
          row!($core::saturating_read_bytes(slice, big_endian, bits));
        }
      }
      for limbs in bigint::limb_slices::<L>(pattern, signed, N) {
        let limbs: &[L] = &limbs;
        row!($core::checked_read_limbs(limbs, bits));
        row!($core::wrapping_read_limbs(limbs, bits));
        row!($core::saturating_read_limbs(limbs, bits));
      }
      // The words of the pattern, and words that run past every width, which the row reduces.
      let words = |index: usize| model::word(pattern, index, signed);
      assert_eq!(
        *<P as $core>::from_words(words, bits, Token::new()).array(),
        <[L; N] as $core>::from_words(words, bits, Token::new()),
        "from_words of {}",
        model::text(pattern)
      );
    }
    let scrambled = |index: usize| 0x9e37_79b9_7f4a_7c15_u64.rotate_left(index as u32 * 7);
    assert_eq!(
      *<P as $core>::from_words(scrambled, bits, Token::new()).array(),
      <[L; N] as $core>::from_words(scrambled, bits, Token::new()),
      "from_words of scrambled words"
    );
    for src in ["", "+", "-", "-0", "0", "1", "-1", "z", "1_0", "-zz"] {
      for radix in [2, 16, 36] {
        row!($core::from_str_radix_bits(src.as_bytes(), radix, bits));
      }
    }
    for parts in row_floats(bits) {
      for mode in bigint::MODES {
        row!($core::from_float(parts, mode, bits));
      }
    }
  }};
}

/// Checks every row of `UintCore` on the heap storage `P` of `[L; N]` against the row of the same
/// name on `[L; N]`, at the unsigned [`Samples`] of `bits` bits, with the patterns in `K` words of
/// 64 bits: the rows of [`common_rows!`], then those of `Uint` alone, with the addition with a
/// carry and the subtraction with a borrow, the casts to the signed storage, the saturating one
/// among them, which store their result in the allocation that they consume, or for a shared
/// pointer in a new one, and the lent non-zero integer. The function is `#[inline(never)]`, so that
/// its frame holds the values of this storage alone.
#[inline(never)]
pub(super) fn uint_rows<P, L, const N: usize, const K: usize>(bits: u32)
where
  P: Heap<L, N>,
  L: ArrayLimb + bigint::Limb + Debug + Ord + Shl<u32, Output = L>,
  [L; N]: ArrayRepr,
{
  let samples = Samples::<L, N>::of::<K>(bits, false);
  common_rows!(UintCore, samples, bits, false);
  for a in &samples.values {
    let x = P::new(*a);
    for (shift, upper) in [(1, false), (3, false), (4, false), (4, true)] {
      assert_eq!(
        texts(|f| UintCore::fmt_pow2(&x, shift, upper, f, Token::new())),
        texts(|f| UintCore::fmt_pow2(a, shift, upper, f, Token::new())),
        "fmt_pow2 of {a:?}"
      );
    }
    row!(UintCore::next_power_of_two_bits(&x => a; bits));
    let (before, other) = (address(x.array()), witness(&x));
    let cast = UintCore::cast_signed_bits(x, bits, Token::new());
    let expected = UintCore::cast_signed_bits(*a, bits, Token::new());
    written(
      &cast,
      &expected,
      before,
      other,
      a,
      &format!("cast_signed_bits of {a:?}"),
    );
    let x = P::new(*a);
    let (before, other) = (address(x.array()), witness(&x));
    let cast = UintCore::saturating_cast_signed_bits(x, bits, Token::new());
    let expected = UintCore::saturating_cast_signed_bits(*a, bits, Token::new());
    let what = format!("saturating_cast_signed_bits of {a:?}");
    written(&cast, &expected, before, other, a, &what);
    // The width of the integer does not matter to the lend.
    let integer = Uint::<(), P>::wrap(P::new(*a));
    let View::Lent(lent) = <P as UintCore>::lend_nonzero(&integer, Token::new()) else {
      panic!("lend_nonzero of {a:?} returns a copy");
    };
    assert!(core::ptr::eq(lent, &integer), "lend_nonzero of {a:?}");
  }
  for (a, b) in &samples.pairs {
    let (x, y) = (P::new(*a), P::new(*b));
    row!(UintCore::abs_diff(&x => a, &y => b));
    row!(UintCore::is_multiple_of(&x => a, &y => b));
    for carry in [false, true] {
      row!(UintCore::carrying_add_bits(&x => a, &y => b; carry, bits));
      row!(UintCore::borrowing_sub_bits(&x => a, &y => b; carry, bits));
    }
  }
}

/// Checks every row of `IntCore` on the heap storage `P` of `[L; N]` against the row of the same
/// name on `[L; N]`, as [`uint_rows`] does for `UintCore`, at the signed [`Samples`] of `bits`
/// bits: the rows of [`common_rows!`], then those of `Int` alone, with the casts to the unsigned
/// storage, the saturating one among them, which store their result in the allocation that they
/// consume, or for a shared pointer in a new one, and the lent non-zero integer.
#[inline(never)]
pub(super) fn int_rows<P, L, const N: usize, const K: usize>(bits: u32)
where
  P: Heap<L, N>,
  L: ArrayLimb + bigint::Limb + Debug + Ord + Shl<u32, Output = L>,
  [L; N]: ArrayRepr,
{
  let samples = Samples::<L, N>::of::<K>(bits, true);
  common_rows!(IntCore, samples, bits, true);
  for a in &samples.values {
    let x = P::new(*a);
    for (shift, upper) in [(1, false), (3, false), (4, false), (4, true)] {
      assert_eq!(
        texts(|f| IntCore::fmt_pow2(&x, bits, shift, upper, f, Token::new())),
        texts(|f| IntCore::fmt_pow2(a, bits, shift, upper, f, Token::new())),
        "fmt_pow2 of {a:?}"
      );
    }
    row!(IntCore::is_negative(&x => a));
    row!(IntCore::is_positive(&x => a));
    row!(IntCore::signum(&x => a));
    row!(IntCore::unsigned_abs(&x => a));
    let (before, other) = (address(x.array()), witness(&x));
    let cast = IntCore::cast_unsigned_bits(x, bits, Token::new());
    let expected = IntCore::cast_unsigned_bits(*a, bits, Token::new());
    written(
      &cast,
      &expected,
      before,
      other,
      a,
      &format!("cast_unsigned_bits of {a:?}"),
    );
    let x = P::new(*a);
    let (before, other) = (address(x.array()), witness(&x));
    let cast = IntCore::saturating_cast_unsigned_bits(x, bits, Token::new());
    let expected = IntCore::saturating_cast_unsigned_bits(*a, bits, Token::new());
    let what = format!("saturating_cast_unsigned_bits of {a:?}");
    written(&cast, &expected, before, other, a, &what);
    // The width of the integer does not matter to the lend.
    let integer = Int::<(), P>::wrap(P::new(*a));
    let View::Lent(lent) = <P as IntCore>::lend_nonzero(&integer, Token::new()) else {
      panic!("lend_nonzero of {a:?} returns a copy");
    };
    assert!(core::ptr::eq(lent, &integer), "lend_nonzero of {a:?}");
  }
  for (a, b) in &samples.pairs {
    let (x, y) = (P::new(*a), P::new(*b));
    row!(IntCore::abs_diff(&x => a, &y => b));
  }
}

/// The size and the alignment of `T` and of `Option<T>`.
pub(super) fn layout<T>() -> [usize; 4] {
  [
    size_of::<T>(),
    align_of::<T>(),
    size_of::<Option<T>>(),
    align_of::<Option<T>>(),
  ]
}

/// Asserts that `T` is `Send` and `Sync`.
pub(super) fn send_and_sync<T: Send + Sync>() {}

/// The results of the conversions of an integer from a slice, as the limbs of the integers: the
/// checked, the wrapping, the overflowing and the saturating conversion, and the strict one where
/// the checked one returns a value.
pub(super) type Reads<A> = (Option<A>, A, (A, bool), A, Option<A>);

/// The [`Reads`] of `T` from the limbs of `slice`, through `ArrayInteger`.
pub(super) fn limb_reads<T: ArrayInteger>(slice: &[T::Limb]) -> Reads<T::Array> {
  let checked = T::checked_from_limbs_slice(slice).map(T::into_limbs);
  let strict = checked
    .as_ref()
    .map(|_| T::strict_from_limbs_slice(slice).into_limbs());
  let (wrapped, overflow) = T::overflowing_from_limbs_slice(slice);
  (
    checked,
    T::wrapping_from_limbs_slice(slice).into_limbs(),
    (wrapped.into_limbs(), overflow),
    T::saturating_from_limbs_slice(slice).into_limbs(),
    strict,
  )
}

/// The [`Reads`] of `T` from `bytes`, least significant first, and from them in reverse order,
/// most significant first, through `IntegerBytes`.
pub(super) fn byte_reads<T: IntegerBytes + ArrayInteger>(bytes: &[u8]) -> [Reads<T::Array>; 2] {
  let reversed: Vec<u8> = bytes.iter().rev().copied().collect();
  let little = T::checked_from_le_slice(bytes).map(T::into_limbs);
  let little_strict = little
    .as_ref()
    .map(|_| T::strict_from_le_slice(bytes).into_limbs());
  let (little_wrapped, little_overflow) = T::overflowing_from_le_slice(bytes);
  let big = T::checked_from_be_slice(&reversed).map(T::into_limbs);
  let big_strict = big
    .as_ref()
    .map(|_| T::strict_from_be_slice(&reversed).into_limbs());
  let (big_wrapped, big_overflow) = T::overflowing_from_be_slice(&reversed);
  [
    (
      little,
      T::wrapping_from_le_slice(bytes).into_limbs(),
      (little_wrapped.into_limbs(), little_overflow),
      T::saturating_from_le_slice(bytes).into_limbs(),
      little_strict,
    ),
    (
      big,
      T::wrapping_from_be_slice(&reversed).into_limbs(),
      (big_wrapped.into_limbs(), big_overflow),
      T::saturating_from_be_slice(&reversed).into_limbs(),
      big_strict,
    ),
  ]
}

/// The results of the conversions of an integer from a float, as the limbs of the integers: the
/// checked and the saturating conversion, the strict one where the checked one returns a value,
/// the exact one, the checked and the exact one from the nearest `f32`, and the one rounded with
/// each rounding mode.
pub(super) type FloatReads<A> = (
  Option<A>,
  A,
  Option<A>,
  Result<A, FromFloatError>,
  Option<A>,
  Result<A, FromFloatError>,
  Vec<Result<A, FromFloatError>>,
);

/// The [`FloatReads`] of `T` from `value`, through `IntegerFloat`.
pub(super) fn float_reads<T: IntegerFloat + ArrayInteger>(value: f64) -> FloatReads<T::Array> {
  let checked = T::checked_from_f64(value).map(T::into_limbs);
  let strict = checked
    .as_ref()
    .map(|_| T::strict_from_f64(value).into_limbs());
  let rounded = bigint::MODES
    .iter()
    .map(|&mode| T::round_from_f64(value, mode).map(T::into_limbs))
    .collect();
  (
    checked,
    T::saturating_from_f64(value).into_limbs(),
    strict,
    T::exact_from_f64(value).map(T::into_limbs),
    T::checked_from_f32(value as f32).map(T::into_limbs),
    T::exact_from_f32(value as f32).map(T::into_limbs),
    rounded,
  )
}

/// The floats at which the conversions from the floats are checked at `bits` bits: zero of both
/// signs, a few halves and integers, `2^(bits - 1)` and `2^bits` and their negations, the largest
/// float, an infinity and NaN.
pub(super) fn float_values(bits: u32) -> Vec<f64> {
  let mut values = vec![0.0, -0.0, 0.5, -1.5, 2.5, 7.0, -1e19, 1e30, f64::MAX];
  for exponent in [bits.saturating_sub(1), bits] {
    let power = floats::pow2(exponent as i32);
    values.extend([power, -power]);
  }
  values.extend([f64::INFINITY, f64::NAN]);
  values
}

/// Emits a function that checks the construction of `$integer` of `B` bits in the heap storage `P`
/// of `[L; N]`, read in two's complement if `$signed`, against its twin in `[L; N]`, through
/// `ArrayInteger`, which compares the limbs of the two: zero through `Default` and
/// `FixedInteger::zero`, and the limits through `FixedInteger`; at the patterns of
/// [`row_patterns`] in `K` words, the limbs through `ArrayInteger::checked_from_limbs`, the slices
/// of limbs and of bytes cut from them through `ArrayInteger` and `IntegerBytes`, the decimal and
/// hexadecimal texts of the twin through `FromStr` and `FixedInteger::from_str_radix`, the casts
/// from the twin into the heap storage and from a borrowed heap integer into the twin and into the
/// native integers, and the conversions to the floats;
/// limbs above the width, which `checked_from_limbs` refuses; and the casts and `TryFrom` of the
/// native integers and the conversions from the floats of [`float_values`]. The function is
/// `#[inline(never)]`, so that its frame holds the values of this storage alone.
macro_rules! construction {
  ($name:ident, $integer:ident, $signed:literal) => {
    #[inline(never)]
    pub(super) fn $name<P, B, L, const N: usize, const K: usize>()
    where
      P: Heap<L, N>,
      B: FitsBits<[L; N]> + FitsBits<P>,
      L: ArrayLimb + bigint::Limb + Debug + Ord + Shl<u32, Output = L>,
      [L; N]: ArrayRepr,
      $integer<B, P>: ArrayInteger<Array = [L; N], Limb = L>,
    {
      let bits = B::U32;
      let zero = $integer::<B, [L; N]>::ZERO.into_limbs();
      assert_eq!($integer::<B, P>::default().into_limbs(), zero);
      assert_eq!(<$integer<B, P> as FixedInteger>::zero().into_limbs(), zero);
      assert_eq!(
        <$integer<B, P> as FixedInteger>::min_value().into_limbs(),
        $integer::<B, [L; N]>::MIN.into_limbs()
      );
      assert_eq!(
        <$integer<B, P> as FixedInteger>::max_value().into_limbs(),
        $integer::<B, [L; N]>::MAX.into_limbs()
      );
      for pattern in row_patterns::<K>(bits, $signed) {
        let limbs: [L; N] = bigint::limbs_of_pattern(&pattern, $signed);
        let twin = <$integer<B, [L; N]> as ArrayInteger>::checked_from_limbs(limbs);
        let twin = twin.expect("a value of the width");
        let heap = <$integer<B, P> as ArrayInteger>::checked_from_limbs(limbs);
        let heap = heap.expect("a value of the width");
        assert_eq!((*heap.as_limbs(), heap.to_limbs()), (limbs, limbs));
        for slice in bigint::limb_slices::<L>(&pattern, $signed, N) {
          assert_eq!(
            limb_reads::<$integer<B, P>>(&slice),
            limb_reads::<$integer<B, [L; N]>>(&slice),
            "{slice:?} into {bits} bits"
          );
        }
        for bytes in bigint::byte_slices(&pattern, $signed) {
          assert_eq!(
            byte_reads::<$integer<B, P>>(&bytes),
            byte_reads::<$integer<B, [L; N]>>(&bytes),
            "{bytes:?} into {bits} bits"
          );
        }
        let text = twin.to_string();
        let parsed = text.parse::<$integer<B, P>>();
        assert_eq!(parsed.map(ArrayInteger::into_limbs), Ok(limbs), "{text}");
        let hex = format!("{twin:x}");
        assert_eq!(
          <$integer<B, P> as FixedInteger>::from_str_radix(&hex, 16).map(ArrayInteger::into_limbs),
          <$integer<B, [L; N]> as FixedInteger>::from_str_radix(&hex, 16)
            .map(ArrayInteger::into_limbs),
          "{hex}"
        );
        let cast = $integer::<B, P>::checked_cast_from(twin);
        assert_eq!(cast.map(ArrayInteger::into_limbs), Some(limbs));
        let cast = $integer::<B, [L; N]>::checked_cast_from(&heap);
        assert_eq!(cast.map(ArrayInteger::into_limbs), Some(limbs));
        assert_eq!(u64::checked_cast_from(&heap), u64::checked_cast_from(twin));
        assert_eq!(
          i8::saturating_cast_from(&heap),
          i8::saturating_cast_from(twin)
        );
        assert_eq!(
          u128::wrapping_cast_from(&heap),
          u128::wrapping_cast_from(twin)
        );
        assert_eq!(heap.to_f64().to_bits(), twin.to_f64().to_bits());
        assert_eq!(heap.to_f32().to_bits(), twin.to_f32().to_bits());
      }
      // Limbs with the bit at the width set hold a value outside the width, unless the width fills
      // them; for `Int`, a value whose sign is not copied above the width.
      let storage_bits = <L as bigint::Limb>::BITS * N as u32;
      if bits < storage_bits {
        let above = model::pattern(storage_bits, |i| i == bits);
        let limbs: [L; N] = bigint::limbs_of_pattern(&above, false);
        let heap = <$integer<B, P> as ArrayInteger>::checked_from_limbs(limbs);
        assert!(heap.is_none(), "{limbs:?} into {bits} bits");
      }
      for value in [0, 1, u64::MAX.into(), u128::MAX] {
        let heap = $integer::<B, P>::checked_cast_from(value);
        let twin = $integer::<B, [L; N]>::checked_cast_from(value);
        assert_eq!(
          heap.map(ArrayInteger::into_limbs),
          twin.map(ArrayInteger::into_limbs)
        );
        let heap = $integer::<B, P>::wrapping_cast_from(value).into_limbs();
        assert_eq!(
          heap,
          $integer::<B, [L; N]>::wrapping_cast_from(value).into_limbs()
        );
      }
      for value in [i128::MIN, -1, 0, i128::MAX] {
        let heap = $integer::<B, P>::saturating_cast_from(value).into_limbs();
        assert_eq!(
          heap,
          $integer::<B, [L; N]>::saturating_cast_from(value).into_limbs()
        );
      }
      assert_eq!(
        $integer::<B, P>::try_from(usize::MAX).map(ArrayInteger::into_limbs),
        $integer::<B, [L; N]>::try_from(usize::MAX).map(ArrayInteger::into_limbs)
      );
      assert_eq!(
        $integer::<B, P>::try_from(isize::MIN).map(ArrayInteger::into_limbs),
        $integer::<B, [L; N]>::try_from(isize::MIN).map(ArrayInteger::into_limbs)
      );
      for value in float_values(bits) {
        assert_eq!(
          float_reads::<$integer<B, P>>(value),
          float_reads::<$integer<B, [L; N]>>(value),
          "{value:e} into {bits} bits"
        );
      }
    }
  };
}

construction!(uint_construction, Uint, false);
construction!(int_construction, Int, true);

/// Checks `FromLimbs` and `From` of the pointer into `Uint` and `Int` of `B` bits in the heap
/// storage `P` of `[L; N]`, a width that fills the array, and `From` of the integers into the
/// pointer, at the patterns of [`row_patterns`] in `K` words: each gives the limbs that it takes.
/// The function is `#[inline(never)]`, so that its frame holds the values of this storage alone.
#[inline(never)]
pub(super) fn filled_construction<P, B, L, const N: usize, const K: usize>()
where
  P: Heap<L, N> + From<Uint<B, P>> + From<Int<B, P>>,
  B: FillsRepr<[L; N]> + FillsRepr<P>,
  L: ArrayLimb + bigint::Limb + Debug,
  [L; N]: ArrayRepr,
  Uint<B, P>: FromLimbs<Array = [L; N]> + From<P>,
  Int<B, P>: FromLimbs<Array = [L; N]> + From<P>,
{
  for signed in [false, true] {
    for pattern in row_patterns::<K>(B::U32, signed) {
      let limbs: [L; N] = bigint::limbs_of_pattern(&pattern, signed);
      let uint = <Uint<B, P> as FromLimbs>::from_limbs(limbs);
      assert_eq!(uint.into_limbs(), limbs);
      let int = <Int<B, P> as FromLimbs>::from_limbs(limbs);
      assert_eq!(int.into_limbs(), limbs);
      let uint = Uint::<B, P>::from(P::new(limbs));
      assert_eq!(*P::from(uint).array(), limbs);
      let int = Int::<B, P>::from(P::new(limbs));
      assert_eq!(*P::from(int).array(), limbs);
    }
  }
}

/// The limbs of `value`, which compare an integer in a heap storage with its inline twin.
pub(super) fn limbs<T: ArrayInteger>(value: T) -> T::Array {
  value.into_limbs()
}

/// The limbs of the integer of a pair, and its flag.
pub(super) fn flagged<T: ArrayInteger>((value, flag): (T, bool)) -> (T::Array, bool) {
  (value.into_limbs(), flag)
}

/// The results of the families of overflowing, checked, wrapping, saturating and strict arithmetic
/// on `a` and `b`, and on `a` with the exponent and the amount `n`, as the limbs of the integers
/// that they return, each where it does not panic: a division where `b` is not zero, and a strict
/// method where the checked one returns a value.
#[inline(never)]
pub(super) fn arithmetic_families<T>(a: &T, b: &T, n: u32) -> Vec<String>
where
  T: FixedInteger
    + OverflowingOps
    + CheckedOps
    + WrappingOps
    + SaturatingOps
    + StrictOps
    + ArrayInteger<Array: Debug>,
{
  let divides = !b.is_zero();
  vec![
    format!(
      "{:?}",
      [
        flagged(a.overflowing_add(b)),
        flagged(a.overflowing_sub(b)),
        flagged(a.overflowing_mul(b)),
        flagged(a.overflowing_neg()),
        flagged(a.overflowing_pow(n)),
        flagged(a.overflowing_shl(n)),
        flagged(a.overflowing_shr(n)),
      ]
    ),
    format!(
      "{:?}",
      divides.then(|| [
        flagged(a.overflowing_div(b)),
        flagged(a.overflowing_rem(b)),
        flagged(a.overflowing_div_euclid(b)),
        flagged(a.overflowing_rem_euclid(b)),
      ])
    ),
    format!(
      "{:?}",
      [
        a.checked_add(b).map(limbs),
        a.checked_sub(b).map(limbs),
        a.checked_mul(b).map(limbs),
        a.checked_neg().map(limbs),
        a.checked_div(b).map(limbs),
        a.checked_rem(b).map(limbs),
        a.checked_div_euclid(b).map(limbs),
        a.checked_rem_euclid(b).map(limbs),
        a.checked_pow(n).map(limbs),
        a.checked_shl(n).map(limbs),
        a.checked_shr(n).map(limbs),
        a.checked_next_multiple_of(b).map(limbs),
        a.checked_div_exact(b).map(limbs),
      ]
    ),
    format!(
      "{:?}",
      [
        limbs(a.wrapping_add(b)),
        limbs(a.wrapping_sub(b)),
        limbs(a.wrapping_mul(b)),
        limbs(a.wrapping_neg()),
        limbs(a.wrapping_pow(n)),
        limbs(a.wrapping_shl(n)),
        limbs(a.wrapping_shr(n)),
        limbs(a.saturating_add(b)),
        limbs(a.saturating_sub(b)),
        limbs(a.saturating_mul(b)),
        limbs(a.saturating_pow(n)),
      ]
    ),
    format!(
      "{:?}",
      divides.then(|| [
        limbs(a.wrapping_div(b)),
        limbs(a.wrapping_rem(b)),
        limbs(a.wrapping_div_euclid(b)),
        limbs(a.wrapping_rem_euclid(b)),
        limbs(a.saturating_div(b)),
      ])
    ),
    format!(
      "{:?}",
      [
        a.checked_add(b).map(|_| limbs(a.strict_add(b))),
        a.checked_sub(b).map(|_| limbs(a.strict_sub(b))),
        a.checked_mul(b).map(|_| limbs(a.strict_mul(b))),
        a.checked_neg().map(|_| limbs(a.strict_neg())),
        a.checked_div(b).map(|_| limbs(a.strict_div(b))),
        a.checked_rem(b).map(|_| limbs(a.strict_rem(b))),
        a.checked_div_euclid(b)
          .map(|_| limbs(a.strict_div_euclid(b))),
        a.checked_rem_euclid(b)
          .map(|_| limbs(a.strict_rem_euclid(b))),
        a.checked_pow(n).map(|_| limbs(a.strict_pow(n))),
        a.checked_shl(n).map(|_| limbs(a.strict_shl(n))),
        a.checked_shr(n).map(|_| limbs(a.strict_shr(n))),
      ]
    ),
  ]
}

/// The results of the functions of `IntegerMath` on `a` and `b`, and on `a` with the exponent and
/// the degree `n`, as the limbs of the integers that they return, each where it does not panic,
/// which the checked method of the same operation decides.
#[inline(never)]
pub(super) fn math<T>(a: &T, b: &T, n: u32) -> Vec<String>
where
  T: CheckedOps + IntegerMath<Magnitude: ArrayInteger<Array: Debug>> + ArrayInteger<Array: Debug>,
{
  vec![
    format!("{:?}", a.checked_pow(n).map(|_| limbs(a.pow(n)))),
    format!(
      "{:?}",
      [
        a.checked_div_euclid(b).map(|_| limbs(a.div_euclid(b))),
        a.checked_rem_euclid(b).map(|_| limbs(a.rem_euclid(b))),
        a.checked_div(b).map(|_| limbs(a.div_floor(b))),
        a.checked_div(b).map(|_| limbs(a.div_ceil(b))),
        a.checked_next_multiple_of(b)
          .map(|_| limbs(a.next_multiple_of(b))),
        a.checked_nth_root(2).map(|_| limbs(a.isqrt())),
        a.checked_nth_root(n).map(limbs),
        a.checked_nth_root(n).map(|_| limbs(a.nth_root(n))),
        Some(limbs(a.midpoint(b))),
      ]
    ),
    format!("{:?}", a.checked_div(b).map(|_| a.div_exact(b).map(limbs))),
    format!(
      "{:?}",
      (
        a.checked_ilog(b),
        a.checked_ilog2(),
        a.checked_ilog10(),
        a.checked_ilog(b).map(|_| a.ilog(b)),
        a.checked_ilog2().map(|_| (a.ilog2(), a.ilog10())),
      )
    ),
    format!("{:?}", limbs(a.abs_diff(b))),
  ]
}

/// The results of the bit operations of `IntegerBits` on `a` with the amount and the index `n`, as
/// the limbs of the integers that they return, with `set_bit` where `n` is a bit of the width.
#[inline(never)]
pub(super) fn bit_operations<T>(a: &T, n: u32) -> Vec<String>
where
  T: FixedInteger + IntegerBits + ArrayInteger<Array: Debug>,
{
  let set = (n < T::BITS).then(|| {
    let (mut set, mut clear) = (a.clone(), a.clone());
    set.set_bit(n, true);
    clear.set_bit(n, false);
    (limbs(set), limbs(clear))
  });
  vec![
    format!(
      "{:?}",
      (
        a.count_ones(),
        a.count_zeros(),
        a.leading_zeros(),
        a.trailing_zeros(),
        a.leading_ones(),
        a.trailing_ones(),
        a.highest_one(),
        a.lowest_one(),
        a.bit(n),
      )
    ),
    format!(
      "{:?}",
      [
        limbs(a.isolate_highest_one()),
        limbs(a.isolate_lowest_one()),
        limbs(a.reverse_bits()),
        limbs(a.rotate_left(n)),
        limbs(a.rotate_right(n)),
        limbs(a.unbounded_shl(n)),
        limbs(a.unbounded_shr(n)),
      ]
    ),
    format!(
      "{:?}",
      (a.shl_exact(n).map(limbs), a.shr_exact(n).map(limbs), set)
    ),
  ]
}

/// The results of the methods of `Uint` alone, of `UnsignedFixedInteger`, `UnsignedMath` and
/// `UnsignedBits`, on `a` and `b`, as the limbs of the integers that they return, each where it does
/// not panic.
#[inline(never)]
pub(super) fn unsigned_methods<T>(a: &T, b: &T) -> Vec<String>
where
  T: FixedInteger
    + UnsignedFixedInteger
    + UnsignedMath
    + UnsignedBits<Signed: ArrayInteger<Array: Debug>>
    + ArrayInteger<Array: Debug>,
{
  vec![
    format!(
      "{:?}",
      [
        flagged(a.carrying_add(b, false)),
        flagged(a.carrying_add(b, true)),
        flagged(a.borrowing_sub(b, false)),
        flagged(a.borrowing_sub(b, true)),
      ]
    ),
    format!(
      "{:?}",
      (
        a.is_multiple_of(b),
        a.is_power_of_two(),
        a.bit_width(),
        a.checked_next_power_of_two().map(limbs),
        limbs(a.wrapping_next_power_of_two()),
        a.checked_next_power_of_two()
          .map(|_| limbs(a.next_power_of_two())),
      )
    ),
    format!(
      "{:?}",
      (
        limbs(a.clone().cast_signed()),
        a.clone().checked_cast_signed().map(limbs),
        limbs(a.clone().saturating_cast_signed()),
        a.clone()
          .checked_cast_signed()
          .map(|_| limbs(a.clone().strict_cast_signed())),
      )
    ),
  ]
}

/// The results of the methods of `Int` alone, of `SignedFixedInteger`, `SignedMath` and
/// `SignedBits`, and of the negation, on `a`, as the limbs of the integers that they return, each
/// where it does not panic.
#[inline(never)]
pub(super) fn signed_methods<T>(a: &T) -> Vec<String>
where
  T: FixedInteger
    + SignedFixedInteger<Unsigned: ArrayInteger<Array: Debug>>
    + SignedMath
    + SignedBits<Magnitude: ArrayInteger<Array: Debug>>
    + CheckedOps
    + ArrayInteger<Array: Debug>,
{
  vec![
    format!(
      "{:?}",
      (
        flagged(a.overflowing_abs()),
        a.checked_abs().map(limbs),
        limbs(a.wrapping_abs()),
        limbs(a.saturating_abs()),
        limbs(a.saturating_neg()),
        limbs(a.signum()),
      )
    ),
    format!(
      "{:?}",
      (
        limbs(a.unsigned_abs()),
        a.is_negative(),
        a.is_positive(),
        a.checked_isqrt().map(limbs),
        a.checked_abs()
          .map(|_| (limbs(a.strict_abs()), limbs(a.abs()))),
        a.checked_neg().map(|_| limbs(-a.clone())),
      )
    ),
    format!(
      "{:?}",
      (
        limbs(a.clone().cast_unsigned()),
        a.clone().checked_cast_unsigned().map(limbs),
        limbs(a.clone().saturating_cast_unsigned()),
        a.clone()
          .checked_cast_unsigned()
          .map(|_| limbs(a.clone().strict_cast_unsigned())),
      )
    ),
  ]
}

/// The text of `a` in each format of `FixedInteger`, with flags, and the limbs of the integers
/// that `FromStr` and `from_str_radix` read from its decimal and hexadecimal text.
#[inline(never)]
pub(super) fn text<T: FixedInteger + ArrayInteger<Array: Debug>>(a: &T) -> Vec<String> {
  let decimal = a.to_string();
  let hex = format!("{a:x}");
  vec![
    format!("{a} {a:+} {a:_>40} {a:?} {a:b} {a:o} {a:x} {a:X} {a:#x} {a:#010b}"),
    format!("{a:e} {a:E} {a:.3e} {a:+.1E} {a:>30e}"),
    format!("{:?}", decimal.parse::<T>().map(limbs)),
    format!("{:?}", T::from_str_radix(&hex, 16).map(limbs)),
    format!("{:?}", T::from_str_radix(&decimal, 10).map(limbs)),
  ]
}

/// The bytes of `a` through `IntegerBytes`: each byte, and the bytes copied to buffers of each
/// length up to two more than the width needs, in both orders.
#[inline(never)]
pub(super) fn bytes<T: IntegerBytes>(a: &T) -> Vec<String> {
  let mut results = vec![format!(
    "{:?}",
    (0..T::BYTES)
      .map(|index| a.byte(index))
      .collect::<Vec<u8>>()
  )];
  for len in 0..=T::BYTES + 2 {
    let (mut little, mut big) = (vec![0xa5; len], vec![0xa5; len]);
    let written = (
      a.checked_copy_le_bytes_to(&mut little),
      a.checked_copy_be_bytes_to(&mut big),
    );
    results.push(format!("{written:?} {little:?} {big:?}"));
  }
  results
}

/// Emits a function that gives the results of the casts of `$integer<B, R>` from a borrowed value
/// into the native integers and into inline integers of other widths and signs, of the conversions
/// to the floats, of `TryFrom` into the native integers, of the comparisons with the native
/// integers both ways and with a second value, and the words that its hash writes.
macro_rules! conversions {
  ($name:ident, $integer:ident, $repr:ident) => {
    #[inline(never)]
    pub(super) fn $name<B: FitsBits<R>, R: $repr + Clone>(
      a: &$integer<B, R>,
      b: &$integer<B, R>,
    ) -> Vec<String> {
      let mut hash = Written::default();
      a.hash(&mut hash);
      vec![
        format!(
          "{:?}",
          (
            u8::checked_cast_from(a),
            i16::wrapping_cast_from(a),
            u64::saturating_cast_from(a),
            i128::checked_cast_from(a),
            u128::wrapping_cast_from(a),
          )
        ),
        format!(
          "{:?}",
          (
            Uint::<U100, [u32; 4]>::checked_cast_from(a),
            Int::<U300, [u64; 5]>::wrapping_cast_from(a),
            Int::<U7, i8>::saturating_cast_from(a),
          )
        ),
        format!("{:?}", (a.to_f64().to_bits(), a.to_f32().to_bits())),
        format!("{:?}", (u8::try_from(a.clone()), i128::try_from(a.clone()))),
        format!(
          "{:?}",
          (
            a.partial_cmp(&0u8),
            a.partial_cmp(&-1i32),
            a.partial_cmp(&u128::MAX),
            i64::MIN.partial_cmp(a),
            *a == 1u64,
            u16::MAX == *a,
          )
        ),
        format!("{:?}", (a.cmp(b), a == b, hash.0)),
      ]
    }
  };
}

conversions!(uint_conversions, Uint, UintRepr);
conversions!(int_conversions, Int, IntRepr);

/// The limbs of the results of the binary operator `$op` on the references `$a` and `$b`, with
/// values and with references in each of the four forms, where `$a.$checked($b)` returns a value:
/// the operators panic or wrap on overflow as `overflow-checks` decides.
macro_rules! forms {
  ($a:expr, $b:expr, $checked:ident, $op:tt) => {{
    let (a, b) = ($a, $b);
    let forms = a.$checked(b).map(|_| {
      [
        limbs(a.clone() $op b.clone()),
        limbs(a.clone() $op b),
        limbs(a $op b.clone()),
        limbs(a $op b),
      ]
    });
    format!("{forms:?}")
  }};
}

/// Emits a function that gives the results of the operators of `$integer<B, R>` on `a` and `b`, and
/// on `a` with the amount `n`, in the forms of [`forms!`], of the shifts, `!` and `Sum` of values
/// and of references, as the limbs of the integers, each where the checked method of the
/// operation returns a value.
macro_rules! operators {
  ($name:ident, $integer:ident, $repr:ident) => {
    #[inline(never)]
    pub(super) fn $name<B, R>(a: &$integer<B, R>, b: &$integer<B, R>, n: u32) -> Vec<String>
    where
      B: FitsBits<R>,
      R: $repr + Clone,
      $integer<B, R>: ArrayInteger<Array: Debug>,
    {
      let shifts = |shl: bool| {
        if shl {
          a.checked_shl(n)
            .map(|_| [limbs(a.clone() << n), limbs(a << n)])
        } else {
          a.checked_shr(n)
            .map(|_| [limbs(a.clone() >> n), limbs(a >> n)])
        }
      };
      vec![
        forms!(a, b, checked_add, +),
        forms!(a, b, checked_sub, -),
        forms!(a, b, checked_mul, *),
        forms!(a, b, checked_div, /),
        forms!(a, b, checked_rem, %),
        format!(
          "{:?}",
          [
            limbs(a.clone() & b.clone()),
            limbs(a.clone() | b),
            limbs(a ^ b.clone()),
            limbs(a & b),
            limbs(!a.clone()),
            limbs(!a),
          ]
        ),
        format!("{:?}", (shifts(true), shifts(false))),
        format!(
          "{:?}",
          a.checked_add(b).map(|_| [
            limbs([a.clone(), b.clone()].into_iter().sum::<$integer<B, R>>()),
            limbs([a, b].into_iter().sum::<$integer<B, R>>()),
          ])
        ),
      ]
    }
  };
}

operators!(uint_operators, Uint, UintRepr);
operators!(int_operators, Int, IntRepr);

/// The limbs of one through `IntegerOne`, and of `Product` of `a` and `b`, of values and of
/// references, where the checked product returns a value.
#[inline(never)]
pub(super) fn products<T>(a: &T, b: &T) -> Vec<String>
where
  T: IntegerOne + CheckedOps + for<'x> Product<&'x T> + ArrayInteger<Array: Debug>,
{
  vec![format!(
    "{:?}",
    (
      limbs(T::one()),
      a.checked_mul(b).map(|_| [
        limbs([a.clone(), b.clone()].into_iter().product::<T>()),
        limbs([a, b].into_iter().product::<T>()),
      ])
    )
  )]
}

/// The results of every trait and operator of `Uint` but the compound assignments on `$a` and
/// `$b`, with the amount `$n`, as the functions above give them; `$repr` is the storage, which
/// the calls name so that a bound of the caller on another storage does not choose it.
macro_rules! uint_results {
  ($a:expr, $b:expr, $n:expr, $repr:ty) => {{
    let (a, b, n) = ($a, $b, $n);
    let mut results = arithmetic_families(a, b, n);
    results.extend(math(a, b, n));
    results.extend(bit_operations(a, n));
    results.extend(unsigned_methods(a, b));
    results.extend(text(a));
    results.extend(bytes(a));
    results.extend(uint_conversions(a, b));
    results.extend(uint_operators::<_, $repr>(a, b, n));
    results
  }};
}
#[cfg(any(feature = "std", feature = "alloc"))]
pub(super) use uint_results;

/// The results of every trait and operator of `Int` but the compound assignments on `$a` and `$b`,
/// with the amount `$n`, as the functions above give them; `$repr` is the storage, as for
/// [`uint_results!`].
macro_rules! int_results {
  ($a:expr, $b:expr, $n:expr, $repr:ty) => {{
    let (a, b, n) = ($a, $b, $n);
    let mut results = arithmetic_families(a, b, n);
    results.extend(math(a, b, n));
    results.extend(bit_operations(a, n));
    results.extend(signed_methods(a));
    results.extend(text(a));
    results.extend(bytes(a));
    results.extend(int_conversions(a, b));
    results.extend(int_operators::<_, $repr>(a, b, n));
    results
  }};
}
#[cfg(any(feature = "std", feature = "alloc"))]
pub(super) use int_results;

/// Emits a function that checks `$integer` of `B` bits in the heap storage `P` of `[L; N]`, which
/// also implements the traits `$bound`, against its twin in `[L; N]`, read in two's complement if
/// `$signed`, through every trait and operator that `$results!` gives, at the pairs of
/// [`row_pairs`] of the values of [`row_patterns`] in `K` words, each pair with the next of the
/// amounts of [`row_amounts`]. The function is `#[inline(never)]`, so that its frame holds the
/// values of this storage alone.
macro_rules! traits_agree {
  ($name:ident, $integer:ident, $signed:literal, $results:ident $(, $bound:path)* $(,)?) => {
    #[inline(never)]
    pub(super) fn $name<P, B, L, const N: usize, const K: usize>()
    where
      P: Heap<L, N> $(+ $bound)*,
      B: FitsBits<[L; N]> + FitsBits<P>,
      L: ArrayLimb + bigint::Limb + Debug,
      [L; N]: ArrayRepr,
      Uint<B, P>: ArrayInteger<Array = [L; N], Limb = L>,
      Int<B, P>: ArrayInteger<Array = [L; N], Limb = L>,
    {
      let bits = B::U32;
      let values: Vec<[L; N]> = row_patterns::<K>(bits, $signed)
        .iter()
        .map(|pattern| bigint::limbs_of_pattern(pattern, $signed))
        .collect();
      let amounts = row_amounts(bits);
      for (k, (x, y)) in row_pairs(&values).into_iter().enumerate() {
        let n = amounts[k % amounts.len()];
        let twin = |array| <$integer<B, [L; N]> as ArrayInteger>::checked_from_limbs(array);
        let heap = |array| <$integer<B, P> as ArrayInteger>::checked_from_limbs(array);
        let (a, b) = (twin(x).expect("in range"), twin(y).expect("in range"));
        let (c, d) = (heap(x).expect("in range"), heap(y).expect("in range"));
        assert_eq!(
          $results!(&c, &d, n, P),
          $results!(&a, &b, n, [L; N]),
          "{x:?} and {y:?} at {bits} bits with {n}"
        );
      }
    }
  };
}
#[cfg(any(feature = "std", feature = "alloc"))]
pub(super) use traits_agree;

traits_agree!(uint_traits_agree, Uint, false, uint_results);
traits_agree!(int_traits_agree, Int, true, int_results);

/// Checks one and `Product` of `Uint` and `Int` of `B` bits in the heap storage `P` of `[L; N]`
/// against their twins in `[L; N]`, at the pairs of [`row_pairs`] of the values of
/// [`row_patterns`] in `K` words; `B` has two bits or more, which `Int` needs for one.
#[inline(never)]
pub(super) fn products_agree<P, B, L, const N: usize, const K: usize>()
where
  P: Heap<L, N>,
  B: FitsBits<[L; N]> + FitsBits<P> + NonZero + IsGreater<U1, Output = True>,
  L: ArrayLimb + bigint::Limb + Debug,
  [L; N]: ArrayRepr,
  Uint<B, P>: ArrayInteger<Array = [L; N], Limb = L>,
  Int<B, P>: ArrayInteger<Array = [L; N], Limb = L>,
{
  for signed in [false, true] {
    let values: Vec<[L; N]> = row_patterns::<K>(B::U32, signed)
      .iter()
      .map(|pattern| bigint::limbs_of_pattern(pattern, signed))
      .collect();
    for (x, y) in row_pairs(&values) {
      if signed {
        let twin = |array| <Int<B, [L; N]> as ArrayInteger>::checked_from_limbs(array);
        let heap = |array| <Int<B, P> as ArrayInteger>::checked_from_limbs(array);
        let (a, b) = (twin(x).expect("in range"), twin(y).expect("in range"));
        let (c, d) = (heap(x).expect("in range"), heap(y).expect("in range"));
        assert_eq!(products(&c, &d), products(&a, &b), "{x:?} and {y:?}");
      } else {
        let twin = |array| <Uint<B, [L; N]> as ArrayInteger>::checked_from_limbs(array);
        let heap = |array| <Uint<B, P> as ArrayInteger>::checked_from_limbs(array);
        let (a, b) = (twin(x).expect("in range"), twin(y).expect("in range"));
        let (c, d) = (heap(x).expect("in range"), heap(y).expect("in range"));
        assert_eq!(products(&c, &d), products(&a, &b), "{x:?} and {y:?}");
      }
    }
  }
}

/// The address of the limbs of `value`, which an operation that stores in its allocation keeps, and
/// an operation on a shared value leaves to every other owner.
pub(super) fn place<T: ArrayInteger>(value: &T) -> *const T::Array {
  value.as_limbs()
}

/// The results of the methods of `FixedNonZero` on `a` and `b`, and on `a` with the exponent and
/// the amount `n`, as the limbs of the integers of the non-zero integers that they return, with the
/// formats, the parsing and the order.
#[inline(never)]
pub(super) fn nonzero_methods<T>(a: &T, b: &T, n: u32) -> Vec<String>
where
  T: FixedNonZero<Integer: ArrayInteger<Array: Debug>>,
{
  let value = |x: T| limbs(x.get());
  vec![
    format!(
      "{:?}",
      [
        value(a.saturating_mul(b)),
        value(a.saturating_pow(n)),
        value(a.isolate_highest_one()),
        value(a.isolate_lowest_one()),
        value(a.reverse_bits()),
        value(a.rotate_left(n)),
        value(a.rotate_right(n)),
      ]
    ),
    format!(
      "{:?}",
      (a.checked_mul(b).map(value), a.checked_pow(n).map(value))
    ),
    format!(
      "{:?}",
      (
        a.leading_zeros(),
        a.trailing_zeros(),
        a.count_ones(),
        a.highest_one(),
        a.lowest_one(),
      )
    ),
    format!("{a} {a:+} {a:x} {a:#b} {a:o} {a:X} {a:e} {a:E} {a:?}"),
    format!(
      "{:?}",
      (
        a.to_string().parse::<T>().map(value),
        T::from_str_radix(&format!("{a:x}"), 16).map(value),
        a.cmp(b),
        a == b,
      )
    ),
  ]
}

/// The results of the methods of `UnsignedNonZero` on `a` and `b`, as the limbs of the integers of
/// the non-zero integers that they return.
#[inline(never)]
pub(super) fn unsigned_nonzero_methods<T>(a: &T, b: &T) -> Vec<String>
where
  T: UnsignedNonZero<
      Integer: ArrayInteger<Array: Debug>,
      Signed: FixedNonZero<Integer: ArrayInteger<Array: Debug>>,
    >,
{
  let value = |x: T| limbs(x.get());
  let integer = b.clone().get();
  vec![
    format!(
      "{:?}",
      (
        a.checked_add(&integer).map(value),
        value(a.saturating_add(&integer)),
        a.checked_next_power_of_two().map(value),
        limbs(a.clone().cast_signed().get()),
      )
    ),
    format!(
      "{:?}",
      (a.ilog2(), a.ilog10(), a.is_power_of_two(), a.bit_width())
    ),
    format!(
      "{:?}",
      [value(a.midpoint(b)), value(a.isqrt()), value(a.div_ceil(b))]
    ),
  ]
}

/// The results of the methods of `SignedNonZero` on `a`, as the limbs of the integers of the
/// non-zero integers that they return, each where it does not panic.
#[inline(never)]
pub(super) fn signed_nonzero_methods<T>(a: &T) -> Vec<String>
where
  T: SignedNonZero<
      Integer: ArrayInteger<Array: Debug>,
      Unsigned: FixedNonZero<Integer: ArrayInteger<Array: Debug>>,
    >,
{
  let value = |x: T| limbs(x.get());
  let pair = |(x, flag): (T, bool)| (limbs(x.get()), flag);
  vec![
    format!(
      "{:?}",
      (
        a.is_negative(),
        a.is_positive(),
        pair(a.overflowing_neg()),
        a.checked_neg().map(value),
        value(a.wrapping_neg()),
        value(a.saturating_neg()),
      )
    ),
    format!(
      "{:?}",
      (
        pair(a.overflowing_abs()),
        a.checked_abs().map(value),
        value(a.wrapping_abs()),
        value(a.saturating_abs()),
        a.checked_abs().map(|_| value(a.abs())),
      )
    ),
    format!(
      "{:?}",
      (
        limbs(a.unsigned_abs().get()),
        limbs(a.clone().cast_unsigned().get())
      )
    ),
  ]
}

/// Emits a function that checks `$nonzero` of `B` bits in the heap storage `P` of `[L; N]` against
/// its twin in `[L; N]`, read in two's complement if `$signed`, through its traits, `$sign` giving
/// the methods of its sign: at the pairs of [`row_pairs`] of the values of [`row_patterns`] in `K`
/// words that are not zero, each pair with the next of the amounts of [`row_amounts`], `TryFrom`
/// of the integer, which rejects zero, and the limits of `NonZeroLimits`. The function is
/// `#[inline(never)]`, so that its frame holds the values of this storage alone.
macro_rules! nonzero_agree {
  ($name:ident, $nonzero:ident, $integer:ident, $signed:literal, $sign:ident) => {
    #[inline(never)]
    pub(super) fn $name<P, B, L, const N: usize, const K: usize>()
    where
      P: Heap<L, N>,
      B: FitsBits<[L; N]> + FitsBits<P> + NonZero,
      L: ArrayLimb + bigint::Limb + Debug,
      [L; N]: ArrayRepr
        + NonZeroUintRepr<Integer = [L; N], Signed = [L; N]>
        + NonZeroIntRepr<Integer = [L; N], Unsigned = [L; N]>,
      Uint<B, P>: ArrayInteger<Array = [L; N], Limb = L>,
      Int<B, P>: ArrayInteger<Array = [L; N], Limb = L>,
    {
      let bits = B::U32;
      let values: Vec<[L; N]> = row_patterns::<K>(bits, $signed)
        .iter()
        .map(|pattern| bigint::limbs_of_pattern(pattern, $signed))
        .collect();
      let amounts = row_amounts(bits);
      let twin = |array| <$integer<B, [L; N]> as ArrayInteger>::checked_from_limbs(array);
      let heap = |array| <$integer<B, P> as ArrayInteger>::checked_from_limbs(array);
      for &x in &values {
        let twin_value = $nonzero::<B, [L; N]>::try_from(twin(x).expect("in range"));
        let heap_value = $nonzero::<B, P>::try_from(heap(x).expect("in range"));
        assert_eq!(
          heap_value.map(|value| limbs(value.get())),
          twin_value.map(|value| limbs(value.get())),
          "{x:?}"
        );
      }
      let nonzero = |array| twin(array).and_then(<$nonzero<B, [L; N]> as FixedNonZero>::new);
      let heap_nonzero = |array| heap(array).and_then(<$nonzero<B, P> as FixedNonZero>::new);
      for (k, (x, y)) in row_pairs(&values).into_iter().enumerate() {
        let n = amounts[k % amounts.len()];
        let (Some(a), Some(b)) = (nonzero(x), nonzero(y)) else {
          continue;
        };
        let (c, d) = (
          heap_nonzero(x).expect("not zero"),
          heap_nonzero(y).expect("not zero"),
        );
        let mut results = nonzero_methods(&c, &d, n);
        results.extend($sign!(&c, &d));
        let mut twin_results = nonzero_methods(&a, &b, n);
        twin_results.extend($sign!(&a, &b));
        assert_eq!(
          results, twin_results,
          "{x:?} and {y:?} at {bits} bits with {n}"
        );
      }
      assert_eq!(
        limbs(<$nonzero<B, P> as NonZeroLimits>::min_value().get()),
        limbs(<$nonzero<B, [L; N]> as NonZeroLimits>::min_value().get())
      );
      assert_eq!(
        limbs(<$nonzero<B, P> as NonZeroLimits>::max_value().get()),
        limbs(<$nonzero<B, [L; N]> as NonZeroLimits>::max_value().get())
      );
    }
  };
}

/// The results of the methods of `UnsignedNonZero` on `$a` and `$b`.
macro_rules! unsigned_nonzero {
  ($a:expr, $b:expr) => {
    unsigned_nonzero_methods($a, $b)
  };
}
#[cfg(any(feature = "std", feature = "alloc"))]
pub(super) use unsigned_nonzero;

/// The results of the methods of `SignedNonZero` on `$a`.
macro_rules! signed_nonzero {
  ($a:expr, $b:expr) => {
    signed_nonzero_methods($a)
  };
}
#[cfg(any(feature = "std", feature = "alloc"))]
pub(super) use signed_nonzero;

nonzero_agree!(
  nonzero_uint_agree,
  NonZeroUint,
  Uint,
  false,
  unsigned_nonzero
);
nonzero_agree!(nonzero_int_agree, NonZeroInt, Int, true, signed_nonzero);

/// Asserts that `heap` writes in JSON and in postcard what `twin`, its inline twin, writes, and
/// that each form reads back into the value of `heap`.
#[cfg(feature = "serde")]
#[inline(never)]
pub(super) fn serde_agrees<T, U>(heap: &T, twin: &U)
where
  T: Serialize + DeserializeOwned + Debug + PartialEq,
  U: Serialize,
{
  let json = serde_json::to_string(heap).expect("JSON");
  assert_eq!(json, serde_json::to_string(twin).expect("JSON"));
  assert_eq!(serde_json::from_str::<T>(&json).expect(&json), *heap);
  let bytes = postcard::to_stdvec(heap).expect("postcard");
  assert_eq!(bytes, postcard::to_stdvec(twin).expect("postcard"));
  assert_eq!(
    postcard::from_bytes::<T>(&bytes).expect("postcard"),
    *heap,
    "{json}"
  );
}

/// Emits a function that checks the serde of `$integer` of `B` bits in the heap storage `P` of
/// `[L; N]`, read in two's complement if `$signed`, and of its non-zero integer `$nonzero`, against
/// their twins in `[L; N]` with [`serde_agrees`], at the values of the patterns of
/// [`row_patterns`] in `K` words. The function is `#[inline(never)]`, so that its frame holds the
/// values of this storage alone.
#[cfg(feature = "serde")]
macro_rules! serde_of {
  ($name:ident, $integer:ident, $nonzero:ident, $signed:literal) => {
    #[inline(never)]
    pub(super) fn $name<P, B, L, const N: usize, const K: usize>()
    where
      P: Heap<L, N>,
      B: FitsBits<[L; N]> + FitsBits<P>,
      L: ArrayLimb + bigint::Limb + Debug,
      [L; N]: ArrayRepr
        + NonZeroUintRepr<Integer = [L; N], Signed = [L; N]>
        + NonZeroIntRepr<Integer = [L; N], Unsigned = [L; N]>,
      $integer<B, P>: ArrayInteger<Array = [L; N], Limb = L>,
    {
      for pattern in row_patterns::<K>(B::U32, $signed) {
        let limbs: [L; N] = bigint::limbs_of_pattern(&pattern, $signed);
        let twin = <$integer<B, [L; N]> as ArrayInteger>::checked_from_limbs(limbs);
        let heap = <$integer<B, P> as ArrayInteger>::checked_from_limbs(limbs);
        let (twin, heap) = (twin.expect("in range"), heap.expect("in range"));
        serde_agrees(&heap, &twin);
        let twin = <$nonzero<B, [L; N]> as FixedNonZero>::new(twin);
        let heap = <$nonzero<B, P> as FixedNonZero>::new(heap);
        if let (Some(twin), Some(heap)) = (twin, heap) {
          serde_agrees(&heap, &twin);
        }
      }
    }
  };
}

#[cfg(feature = "serde")]
serde_of!(uint_serde, Uint, NonZeroUint, false);
#[cfg(feature = "serde")]
serde_of!(int_serde, Int, NonZeroInt, true);

/// Asserts that `$expr`, an operation on owners of the values `$owner`, leaves each owner its value
/// and its allocation, and gives its result, an integer, an allocation of its own.
macro_rules! apart {
  ($expr:expr; $($owner:ident),+ $(,)?) => {{
    let before = [$((place(&$owner), $owner.to_limbs())),+];
    let result = $expr;
    let after = [$((place(&$owner), $owner.to_limbs())),+];
    assert_eq!(after, before, "{}", stringify!($expr));
    $(assert_ne!(
      place(&result) as *const (),
      place(&$owner) as *const (),
      "{}",
      stringify!($expr)
    );)+
  }};
}

/// Emits the tests of the shared heap storage `$pointer<[L; N]>`, each against the inline twin, in
/// the module of the pointer.
macro_rules! shared_tests {
  ($pointer:ident) => {
    /// Every row of `UintCore` and `IntCore` on the pointer gives what the row of the same name
    /// gives on the array it holds, a value stored and a cast between the signs going to a new
    /// allocation while the other owner keeps the old one: `[u64; N]` at 0 bits in no limbs and at
    /// 65 in two, three bytes at 20 bits, and four 64-bit limbs at 200 bits and, in an optimized
    /// build, at 256. Under Miri, no limbs and two 64-bit limbs at 65 bits.
    #[test]
    fn rows_agree_with_the_array_rows() {
      uint_rows::<$pointer<[u64; 0]>, u64, 0, 1>(0);
      int_rows::<$pointer<[u64; 0]>, u64, 0, 1>(0);
      uint_rows::<$pointer<[u64; 2]>, u64, 2, 2>(65);
      int_rows::<$pointer<[u64; 2]>, u64, 2, 2>(65);
      if tier().is_miri() {
        return;
      }
      uint_rows::<$pointer<[u8; 3]>, u8, 3, 1>(20);
      int_rows::<$pointer<[u8; 3]>, u8, 3, 1>(20);
      uint_rows::<$pointer<[u64; 4]>, u64, 4, 4>(200);
      int_rows::<$pointer<[u64; 4]>, u64, 4, 4>(200);
      if tier().is_full() {
        uint_rows::<$pointer<[u64; 4]>, u64, 4, 4>(256);
        int_rows::<$pointer<[u64; 4]>, u64, 4, 4>(256);
      }
    }

    /// A clone shares the allocation of its source, and so do the non-zero `new` and `get`; every
    /// operation that returns an integer, the operators with owned operands, `set_bit`, the casts
    /// between the signs and `Sum` among them, leaves every other owner of an operand its value and
    /// its allocation, and gives its result an allocation of its own.
    #[test]
    fn shared_values_are_never_written() {
      type U = Uint<U256, $pointer<[u64; 4]>>;
      type I = Int<U200, $pointer<[u64; 4]>>;
      type Nz = NonZeroUint<U256, $pointer<[u64; 4]>>;
      let (a, b) = (U::from(1_000_000u64), U::from(7u64));
      let i = I::from(-1_000_000i64);
      let divisor = Nz::new(U::from(3u64)).expect("not zero");
      let d = divisor.clone().get();
      assert_eq!(place(&a.clone()), place(&a));
      let nonzero = Nz::new(a.clone()).expect("not zero");
      assert_eq!(place(&nonzero.get()), place(&a));
      apart!(a.clone() + b.clone(); a, b);
      apart!(a.clone() + &b; a, b);
      apart!(&a + b.clone(); a, b);
      apart!(&a + &b; a, b);
      apart!(a.clone() - &b; a, b);
      apart!(a.clone() * b.clone(); a, b);
      apart!(a.clone() / b.clone(); a, b);
      apart!(&a % b.clone(); a, b);
      apart!(a.clone() & b.clone(); a, b);
      apart!(a.clone() | &b; a, b);
      apart!(&a ^ b.clone(); a, b);
      apart!(!a.clone(); a);
      apart!(a.clone() << 3u32; a);
      apart!(a.clone() >> 3u32; a);
      apart!(-i.clone(); i);
      apart!(a.clone() / divisor.clone(); a, d);
      apart!(&a / divisor.clone(); a, d);
      apart!(a.clone() % &divisor; a, d);
      apart!((Nz::new(a.clone()).expect("not zero") | b.clone()).get(); a, b);
      apart!(a.clone().cast_signed(); a);
      apart!(i.clone().cast_unsigned(); i);
      apart!(Nz::new(a.clone()).expect("not zero").cast_signed().get(); a);
      apart!([a.clone(), b.clone()].into_iter().sum::<U>(); a, b);
      apart!([&a, &b].into_iter().sum::<U>(); a, b);
      apart!([a.clone(), b.clone()].into_iter().product::<U>(); a, b);
      apart!(a.clone().wrapping_add(&b); a, b);
      apart!(
        {
          let mut target = a.clone();
          target.set_bit(100, true);
          target
        };
        a
      );
      let mut target = a.clone();
      target.set_bit(100, true);
      assert_eq!(target.to_limbs(), [1_000_000, 1 << 36, 0, 0]);
      assert_eq!(a.to_limbs(), [1_000_000, 0, 0, 0]);
    }

    /// Every trait and operator of `Uint` and `Int` in the pointer but the compound assignments
    /// gives what it gives on the inline twin, compared through the limbs of the results, with
    /// `FixedInteger`, `IntegerOne`, `Sum` and `Product`, in no limbs at 0 bits, two 64-bit limbs
    /// at 65, three bytes at 20 and four 64-bit limbs at 200 and, in an optimized build, at 256;
    /// under Miri, at 0 and 65 bits.
    #[test]
    fn integers_agree_with_inline_integers() {
      uint_traits_agree::<$pointer<[u64; 0]>, U0, u64, 0, 1>();
      int_traits_agree::<$pointer<[u64; 0]>, U0, u64, 0, 1>();
      uint_traits_agree::<$pointer<[u64; 2]>, U65, u64, 2, 2>();
      int_traits_agree::<$pointer<[u64; 2]>, U65, u64, 2, 2>();
      products_agree::<$pointer<[u64; 2]>, U65, u64, 2, 2>();
      if tier().is_miri() {
        return;
      }
      uint_traits_agree::<$pointer<[u8; 3]>, U20, u8, 3, 1>();
      int_traits_agree::<$pointer<[u8; 3]>, U20, u8, 3, 1>();
      products_agree::<$pointer<[u8; 3]>, U20, u8, 3, 1>();
      uint_traits_agree::<$pointer<[u64; 4]>, U200, u64, 4, 4>();
      int_traits_agree::<$pointer<[u64; 4]>, U200, u64, 4, 4>();
      products_agree::<$pointer<[u64; 4]>, U200, u64, 4, 4>();
      if tier().is_full() {
        uint_traits_agree::<$pointer<[u64; 4]>, U256, u64, 4, 4>();
        int_traits_agree::<$pointer<[u64; 4]>, U256, u64, 4, 4>();
      }
    }

    /// Each way of building an integer in the pointer gives the limbs of its inline twin, as for
    /// `Box`: zero, the limits, the limbs, the slices, the text, the casts, the native integers and
    /// the floats, in no limbs at 0 bits, two 64-bit limbs at 65 and three bytes at 20; `FromLimbs`
    /// and `From` of the pointer at widths that fill the array; and, at 256 bits, `From` of the
    /// native integers, `IntegerOne` and `widen` both ways. Under Miri, at 0 and 65 bits and at the
    /// filled widths of 64-bit limbs.
    #[test]
    fn construction_agrees_with_inline_integers() {
      uint_construction::<$pointer<[u64; 0]>, U0, u64, 0, 1>();
      int_construction::<$pointer<[u64; 0]>, U0, u64, 0, 1>();
      uint_construction::<$pointer<[u64; 2]>, U65, u64, 2, 2>();
      int_construction::<$pointer<[u64; 2]>, U65, u64, 2, 2>();
      filled_construction::<$pointer<[u64; 0]>, U0, u64, 0, 1>();
      filled_construction::<$pointer<[u64; 4]>, U256, u64, 4, 4>();
      if !tier().is_miri() {
        uint_construction::<$pointer<[u8; 3]>, U20, u8, 3, 1>();
        int_construction::<$pointer<[u8; 3]>, U20, u8, 3, 1>();
        filled_construction::<$pointer<[u8; 3]>, U24, u8, 3, 1>();
      }

      type U = Uint<U256, $pointer<[u64; 4]>>;
      type I = Int<U256, $pointer<[u64; 4]>>;
      assert_eq!(
        U::from(u64::MAX).into_limbs(),
        u256::from(u64::MAX).into_limbs()
      );
      assert_eq!(
        I::from(i64::MIN).into_limbs(),
        i256::from(i64::MIN).into_limbs()
      );
      assert_eq!(
        <U as IntegerOne>::one().into_limbs(),
        u256::ONE.into_limbs()
      );
      assert_eq!(
        <I as IntegerOne>::one().into_limbs(),
        i256::ONE.into_limbs()
      );
      let widened: I = i7::new(-50).widen();
      assert_eq!(widened.into_limbs(), i256::from(-50i8).into_limbs());
      let shared = U::from(u64::MAX);
      let wide = Uint::<U300, [u64; 5]>::widen_from(&shared);
      assert_eq!(wide.to_limbs(), [u64::MAX, 0, 0, 0, 0]);
      assert_eq!(shared.to_limbs(), [u64::MAX, 0, 0, 0]);
    }

    /// The non-zero integers over the pointer give what their twins over the array give, through
    /// their traits, `TryFrom` of the integer and the limits, in two 64-bit limbs at 65 bits and,
    /// but under Miri, in three bytes at 20 bits; the division and the remainder of an integer by a
    /// non-zero integer and `|` give the values of the twins; and `Option` of a non-zero integer
    /// has the size of a pointer.
    #[test]
    fn nonzero_integers_agree_with_inline_integers() {
      nonzero_uint_agree::<$pointer<[u64; 2]>, U65, u64, 2, 2>();
      nonzero_int_agree::<$pointer<[u64; 2]>, U65, u64, 2, 2>();
      if !tier().is_miri() {
        nonzero_uint_agree::<$pointer<[u8; 3]>, U20, u8, 3, 1>();
        nonzero_int_agree::<$pointer<[u8; 3]>, U20, u8, 3, 1>();
      }

      type U = Uint<U256, $pointer<[u64; 4]>>;
      type Nz = NonZeroUint<U256, $pointer<[u64; 4]>>;
      type Twin = NonZeroUint<U256, [u64; 4]>;
      let (x, twin_x) = (U::from(1_000_003u64), u256::from(1_000_003u64));
      let a = Nz::new(U::from(7u64)).expect("not zero");
      let twin_a = Twin::new(u256::from(7u64)).expect("not zero");
      assert_eq!(
        [
          limbs(x.clone() / a.clone()),
          limbs(&x / &a),
          limbs(x.clone() % &a),
          limbs((a.clone() | &x).get()),
        ],
        [
          (twin_x / twin_a).to_limbs(),
          (twin_x / twin_a).to_limbs(),
          (twin_x % twin_a).to_limbs(),
          (twin_a | twin_x).get().to_limbs(),
        ]
      );
      assert_eq!(size_of::<Option<Nz>>(), size_of::<usize>());
    }

    /// An integer in the pointer has the size and the alignment of a pointer, and so has its
    /// `Option`, which takes the niche of the pointer, in limbs of each type and in no limbs, and
    /// so have the non-zero integers.
    #[test]
    fn integers_have_the_layout_of_a_pointer() {
      let (size, align) = (size_of::<usize>(), align_of::<usize>());
      let pointer = [size, align, size, align];
      assert_eq!(layout::<Uint<U256, $pointer<[u64; 4]>>>(), pointer);
      assert_eq!(layout::<Int<U20, $pointer<[u8; 3]>>>(), pointer);
      assert_eq!(layout::<Uint<U129, $pointer<[u128; 2]>>>(), pointer);
      assert_eq!(layout::<Int<U0, $pointer<[u64; 0]>>>(), pointer);
      assert_eq!(layout::<NonZeroUint<U256, $pointer<[u64; 4]>>>(), pointer);
      assert_eq!(layout::<NonZeroInt<U20, $pointer<[u8; 3]>>>(), pointer);
    }

    /// An integer in the pointer writes in JSON and in postcard what its inline twin writes, and
    /// reads back from both, at 7, 64 and 200 bits and at zero bits, in both signs, and so do the
    /// non-zero integers.
    #[cfg(feature = "serde")]
    #[test]
    fn integers_serde_as_inline_integers() {
      uint_serde::<$pointer<[u64; 0]>, U0, u64, 0, 1>();
      int_serde::<$pointer<[u64; 0]>, U0, u64, 0, 1>();
      uint_serde::<$pointer<[u8; 1]>, U7, u8, 1, 1>();
      int_serde::<$pointer<[u8; 1]>, U7, u8, 1, 1>();
      uint_serde::<$pointer<[u64; 1]>, U64, u64, 1, 1>();
      int_serde::<$pointer<[u64; 1]>, U64, u64, 1, 1>();
      uint_serde::<$pointer<[u64; 4]>, U200, u64, 4, 4>();
      int_serde::<$pointer<[u64; 4]>, U200, u64, 4, 4>();
    }
  };
}

/// `Rc<[L; N]>`, which is neither `Send` nor `Sync`.
#[cfg(any(feature = "std", feature = "alloc"))]
mod rc {
  use super::*;

  shared_tests!(Rc);
}

/// `Arc<[L; N]>`.
#[cfg(all(any(feature = "std", feature = "alloc"), target_has_atomic = "ptr"))]
mod arc {
  use super::*;

  shared_tests!(Arc);

  /// The integers and the non-zero integers in an `Arc` are `Send` and `Sync`, as the `Arc` is.
  #[test]
  fn integers_are_send_and_sync() {
    send_and_sync::<Uint<U256, Arc<[u64; 4]>>>();
    send_and_sync::<Int<U20, Arc<[u8; 3]>>>();
    send_and_sync::<NonZeroUint<U256, Arc<[u64; 4]>>>();
    send_and_sync::<NonZeroInt<U20, Arc<[u8; 3]>>>();
  }
}

/// `triomphe::Arc<[L; N]>`.
#[cfg(all(feature = "triomphe", target_has_atomic = "ptr"))]
mod triomphe_arc {
  use typenum::U65536;

  use super::*;

  shared_tests!(TriompheArc);

  /// The integers and the non-zero integers in a `triomphe::Arc` are `Send` and `Sync`, as the
  /// pointer is.
  #[test]
  fn integers_are_send_and_sync() {
    send_and_sync::<Uint<U256, TriompheArc<[u64; 4]>>>();
    send_and_sync::<Int<U20, TriompheArc<[u8; 3]>>>();
    send_and_sync::<NonZeroUint<U256, TriompheArc<[u64; 4]>>>();
    send_and_sync::<NonZeroInt<U20, TriompheArc<[u8; 3]>>>();
  }

  /// Checks that zero of `B` bits in `N` 64-bit limbs in a `triomphe::Arc`, through
  /// `FixedInteger::zero` and `Default`, has zero limbs, in both signs. The function is
  /// `#[inline(never)]`, so that its frame holds the values of this width alone.
  #[inline(never)]
  fn zero_of<B: FitsBits<TriompheArc<[u64; N]>>, const N: usize>() {
    let uint = <Uint<B, TriompheArc<[u64; N]>> as FixedInteger>::zero();
    assert_eq!(uint.as_limbs(), &[0; N]);
    assert_eq!(Uint::<B, TriompheArc<[u64; N]>>::default(), uint);
    let int = <Int<B, TriompheArc<[u64; N]>> as FixedInteger>::zero();
    assert_eq!(int.as_limbs(), &[0; N]);
    assert_eq!(Int::<B, TriompheArc<[u64; N]>>::default(), int);
  }

  /// The zero of a `triomphe::Arc`, the one zero of a heap storage that is built as an array and
  /// moved into its allocation, is zero in no limbs, at 256 and 4096 bits and, in an optimized
  /// build, at 65536 bits, each width in a frame of its own.
  #[test]
  fn zero_is_moved_into_its_allocation() {
    zero_of::<U0, 0>();
    zero_of::<U256, 4>();
    zero_of::<U4096, 64>();
    if tier().is_full() {
      zero_of::<U65536, 1024>();
    }
  }
}
