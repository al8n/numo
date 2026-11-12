//! The heap storage `Box<[L; N]>` of `Uint`, `Int`, `NonZeroUint` and `NonZeroInt`, against its
//! inline twin, the array that it holds: every row of the storage cores on a box against the row of
//! the same name on its array, with the pointers of the rows that never allocate; the layout and
//! the auto traits of the boxed integers; and each way of building one.

use core::{cmp::Ordering, fmt, hash::Hasher, ops::Shl};

use typenum::{U20, U300};

use super::*;
use crate::{
  ArrayInteger, BoundedCastFrom, ByteArray, CheckedCastFrom, FillsRepr, FromByteArray,
  FromFloatError, FromLimbs, IntegerBytes, IntegerFloat, NonZeroInt, NonZeroUint, WidenFrom,
  bigint::floats,
  int::float::{FloatFormat, FloatParts, float_parts_f64},
  layout::Token,
  width::{ArrayLimb, ArrayRepr, IntCore, UintCore, View},
};

/// A result of a row of a heap storage, as the result of the row of its array: each storage in it
/// replaced by the array that it holds.
trait Unbox {
  /// The result of the row of the array.
  type Inline;

  /// The result with each storage replaced by its array.
  fn unbox(self) -> Self::Inline;
}

impl<L, const N: usize> Unbox for Box<[L; N]> {
  type Inline = [L; N];

  fn unbox(self) -> [L; N] {
    *self
  }
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
/// arguments alone, and it is called on `Box<[L; N]>` and on `[L; N]`.
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
      Unbox::unbox(<Box<[L; N]> as $core>::$row($($arg,)* Token::new())),
      <[L; N] as $core>::$row($($arg,)* Token::new()),
      "{} with {:?}",
      stringify!($row),
      ($(&$arg,)*),
    )
  };
}

/// Formats with `write`, as a format argument.
struct Text<F>(F);

impl<F: Fn(&mut fmt::Formatter<'_>) -> fmt::Result> fmt::Display for Text<F> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    (self.0)(f)
  }
}

/// The texts that `write` gives with no flags and with the flags `+`, `#`, a fill and a width, and
/// `0` with a precision, each of which the rows of the formats honor as the integers do.
fn texts(write: impl Fn(&mut fmt::Formatter<'_>) -> fmt::Result) -> [String; 5] {
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
struct Written(Vec<u8>);

impl Hasher for Written {
  fn finish(&self) -> u64 {
    0
  }

  fn write(&mut self, bytes: &[u8]) {
    self.0.extend_from_slice(bytes);
  }
}

/// The address of `array`, which a box lends to it: a row that never allocates keeps the address
/// of the array in its box.
fn address<L, const N: usize>(array: &[L; N]) -> *const [L; N] {
  array
}

/// The patterns of `bits` bits that every sample of the rows keeps: zero, one, the top bit alone,
/// every bit but the top one and every bit, the limits of both signs among them.
fn pattern_ends(bits: u32) -> Vec<Vec<bool>> {
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
fn row_patterns<const K: usize>(bits: u32, signed: bool) -> Vec<Vec<bool>> {
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
fn row_pairs<T: Copy>(values: &[T]) -> Vec<(T, T)> {
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
fn row_amounts(bits: u32) -> Vec<u32> {
  let mut amounts = vec![0, 1, bits / 2, bits.saturating_sub(1), bits, bits + 1];
  amounts.sort_unstable();
  amounts.dedup();
  amounts
}

/// The exponents of the powers and the degrees of the roots at which the rows are checked.
const EXPONENTS: [u32; 5] = [0, 1, 2, 3, 7];

/// The parts of the floats at which `from_float` is checked at `bits` bits: zero of both signs, a
/// few halves and integers, `2^(bits - 1)` and `2^bits` and their negations where they are finite,
/// and the largest float.
fn row_floats(bits: u32) -> Vec<FloatParts> {
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
struct Samples<L, const N: usize> {
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

/// Checks the rows that `UintCore` and `IntCore` share, through the core `$core`, on boxes of the
/// values of `$samples`, of `$bits` bits and signed if `$signed`, against the arrays: the protocol
/// of the inline storage, the rows of one operand at each value with the amounts, the exponents and
/// the formats, the rows of two operands at each pair, the readers of bytes, of limbs, of words, of
/// text and of floats, and the conversions with the non-zero storage, which keep the pointer.
macro_rules! shared_rows {
  ($core:ident, $samples:expr, $bits:expr, $signed:expr) => {{
    let (samples, bits, signed): (&Samples<L, N>, u32, bool) = (&$samples, $bits, $signed);
    let storage_bits = <L as bigint::Limb>::BITS * N as u32;
    let storage_bytes = size_of::<[L; N]>();
    row!($core::zero());
    for a in &samples.values {
      let x = Box::new(*a);
      // The protocol lends the array in the box, boxes a value, and stores one in place.
      assert!(core::ptr::eq($core::inline(&x, Token::new()), address(&x)));
      assert_eq!(*<Box<[L; N]> as $core>::from_inline(*a, Token::new()), *a);
      let mut target = <Box<[L; N]> as $core>::from_inline(samples.values[0], Token::new());
      let before = address(&target);
      $core::set_inline(&mut target, *a, Token::new());
      assert_eq!((*target, address(&target)), (*a, before), "set_inline of {a:?}");
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
      // The conversions with the non-zero storage, which is the box itself, keep the pointer.
      let y = x.clone();
      let before = address(&y);
      let nonzero = $core::to_nonzero(y, Token::new());
      assert_eq!(
        nonzero.as_ref().map(|nonzero| (**nonzero, address(nonzero))),
        $core::to_nonzero(*a, Token::new()).map(|nonzero| (nonzero, before)),
        "to_nonzero of {a:?}"
      );
      if let Some(nonzero) = nonzero {
        let lent =
          <Box<[L; N]> as $core>::with_nonzero(&nonzero, |lent| address(lent), Token::new());
        assert_eq!(lent, before, "with_nonzero of {a:?}");
        let integer = <Box<[L; N]> as $core>::from_nonzero(nonzero, Token::new());
        assert_eq!((*integer, address(&integer)), (*a, before), "from_nonzero of {a:?}");
        let known = $core::known_nonzero(integer, Token::new());
        assert_eq!((*known, address(&known)), (*a, before), "known_nonzero of {a:?}");
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
      let (x, y) = (Box::new(*a), Box::new(*b));
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
        *<Box<[L; N]> as $core>::from_words(words, bits, Token::new()),
        <[L; N] as $core>::from_words(words, bits, Token::new()),
        "from_words of {}",
        model::text(pattern)
      );
    }
    let scrambled = |index: usize| 0x9e37_79b9_7f4a_7c15_u64.rotate_left(index as u32 * 7);
    assert_eq!(
      *<Box<[L; N]> as $core>::from_words(scrambled, bits, Token::new()),
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

/// Checks every row of `UintCore` on `Box<[L; N]>` against the row of the same name on `[L; N]`,
/// at the unsigned [`Samples`] of `bits` bits, with the patterns in `K` words of 64 bits: the rows
/// of [`shared_rows!`], then those of `Uint` alone, with the cast to the signed storage, which
/// stores its result in the allocation that it consumes, and the lent non-zero integer. The
/// function is `#[inline(never)]`, so that its frame holds the values of this storage alone.
#[inline(never)]
fn uint_rows<L, const N: usize, const K: usize>(bits: u32)
where
  L: ArrayLimb + bigint::Limb + Debug + Ord + Shl<u32, Output = L>,
  [L; N]: ArrayRepr,
{
  let samples = Samples::<L, N>::of::<K>(bits, false);
  shared_rows!(UintCore, samples, bits, false);
  for a in &samples.values {
    let x = Box::new(*a);
    for (shift, upper) in [(1, false), (3, false), (4, false), (4, true)] {
      assert_eq!(
        texts(|f| UintCore::fmt_pow2(&x, shift, upper, f, Token::new())),
        texts(|f| UintCore::fmt_pow2(a, shift, upper, f, Token::new())),
        "fmt_pow2 of {a:?}"
      );
    }
    row!(UintCore::next_power_of_two_bits(&x => a; bits));
    let before = address(&x);
    let cast = UintCore::cast_signed_bits(x, bits, Token::new());
    assert_eq!(
      (*cast, address(&cast)),
      (UintCore::cast_signed_bits(*a, bits, Token::new()), before),
      "cast_signed_bits of {a:?}"
    );
    // The width of the integer does not matter to the lend.
    let integer = Uint::<(), Box<[L; N]>>::wrap(Box::new(*a));
    let View::Lent(lent) = <Box<[L; N]> as UintCore>::lend_nonzero(&integer, Token::new()) else {
      panic!("lend_nonzero of {a:?} returns a copy");
    };
    assert!(core::ptr::eq(lent, &integer), "lend_nonzero of {a:?}");
  }
  for (a, b) in &samples.pairs {
    let (x, y) = (Box::new(*a), Box::new(*b));
    row!(UintCore::abs_diff(&x => a, &y => b));
    row!(UintCore::is_multiple_of(&x => a, &y => b));
  }
}

/// Checks every row of `IntCore` on `Box<[L; N]>` against the row of the same name on `[L; N]`, as
/// [`uint_rows`] does for `UintCore`, at the signed [`Samples`] of `bits` bits: the rows of
/// [`shared_rows!`], then those of `Int` alone, with the cast to the unsigned storage, which stores
/// its result in the allocation that it consumes, and the lent non-zero integer.
#[inline(never)]
fn int_rows<L, const N: usize, const K: usize>(bits: u32)
where
  L: ArrayLimb + bigint::Limb + Debug + Ord + Shl<u32, Output = L>,
  [L; N]: ArrayRepr,
{
  let samples = Samples::<L, N>::of::<K>(bits, true);
  shared_rows!(IntCore, samples, bits, true);
  for a in &samples.values {
    let x = Box::new(*a);
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
    let before = address(&x);
    let cast = IntCore::cast_unsigned_bits(x, bits, Token::new());
    assert_eq!(
      (*cast, address(&cast)),
      (IntCore::cast_unsigned_bits(*a, bits, Token::new()), before),
      "cast_unsigned_bits of {a:?}"
    );
    // The width of the integer does not matter to the lend.
    let integer = Int::<(), Box<[L; N]>>::wrap(Box::new(*a));
    let View::Lent(lent) = <Box<[L; N]> as IntCore>::lend_nonzero(&integer, Token::new()) else {
      panic!("lend_nonzero of {a:?} returns a copy");
    };
    assert!(core::ptr::eq(lent, &integer), "lend_nonzero of {a:?}");
  }
  for (a, b) in &samples.pairs {
    let (x, y) = (Box::new(*a), Box::new(*b));
    row!(IntCore::abs_diff(&x => a, &y => b));
  }
}

/// Every row of `UintCore` and `IntCore` on `Box<[L; N]>` gives what the row of the same name gives
/// on `[L; N]`, so that a row that forwards to the wrong row, or passes the wrong operand, changes a
/// result here, whatever calls it: `[u64; N]` at 0 bits in no limbs, at 1, 63 and 64 bits in one,
/// at 65 and 128 in two, at 200 and 256 in four and, in an optimized build, at 1024 in sixteen;
/// three bytes at 20 bits, two 16-bit limbs at 24, three 32-bit limbs at 65, two 128-bit limbs at
/// 129 and 256, and 25 bytes at 200. Under Miri, no limbs, two 64-bit limbs at 65 bits and three
/// bytes at 20.
#[test]
fn box_rows_agree_with_the_array_rows() {
  uint_rows::<u64, 0, 1>(0);
  int_rows::<u64, 0, 1>(0);
  uint_rows::<u64, 2, 2>(65);
  int_rows::<u64, 2, 2>(65);
  uint_rows::<u8, 3, 1>(20);
  int_rows::<u8, 3, 1>(20);
  if tier().is_miri() {
    return;
  }
  for bits in [1, 63, 64] {
    uint_rows::<u64, 1, 1>(bits);
    int_rows::<u64, 1, 1>(bits);
  }
  uint_rows::<u64, 2, 2>(128);
  int_rows::<u64, 2, 2>(128);
  for bits in [200, 256] {
    uint_rows::<u64, 4, 4>(bits);
    int_rows::<u64, 4, 4>(bits);
  }
  uint_rows::<u16, 2, 1>(24);
  int_rows::<u16, 2, 1>(24);
  uint_rows::<u32, 3, 2>(65);
  int_rows::<u32, 3, 2>(65);
  for bits in [129, 256] {
    uint_rows::<u128, 2, 4>(bits);
    int_rows::<u128, 2, 4>(bits);
  }
  uint_rows::<u8, 25, 4>(200);
  int_rows::<u8, 25, 4>(200);
  if tier().is_full() {
    uint_rows::<u64, 16, 16>(1024);
    int_rows::<u64, 16, 16>(1024);
  }
}

/// The size and the alignment of `T` and of `Option<T>`.
fn layout<T>() -> [usize; 4] {
  [
    size_of::<T>(),
    align_of::<T>(),
    size_of::<Option<T>>(),
    align_of::<Option<T>>(),
  ]
}

/// Asserts that `T` is `Send` and `Sync`.
fn send_and_sync<T: Send + Sync>() {}

/// A boxed integer has the size and the alignment of a pointer, and so has its `Option`, which
/// takes the niche of the pointer, in limbs of each type and in no limbs, and so have the non-zero
/// integers; and each is `Send` and `Sync`, as the box is.
#[test]
fn boxed_integers_have_the_layout_of_a_pointer() {
  let (size, align) = (size_of::<usize>(), align_of::<usize>());
  let pointer = [size, align, size, align];
  assert_eq!(layout::<Uint<U256, Box<[u64; 4]>>>(), pointer);
  assert_eq!(layout::<Int<U256, Box<[u64; 4]>>>(), pointer);
  assert_eq!(layout::<Uint<U20, Box<[u8; 3]>>>(), pointer);
  assert_eq!(layout::<Int<U20, Box<[u8; 3]>>>(), pointer);
  assert_eq!(layout::<Uint<U129, Box<[u128; 2]>>>(), pointer);
  assert_eq!(layout::<Uint<U0, Box<[u64; 0]>>>(), pointer);
  assert_eq!(layout::<Int<U0, Box<[u64; 0]>>>(), pointer);
  assert_eq!(layout::<NonZeroUint<U256, Box<[u64; 4]>>>(), pointer);
  assert_eq!(layout::<NonZeroInt<U256, Box<[u64; 4]>>>(), pointer);
  assert_eq!(layout::<NonZeroUint<U20, Box<[u8; 3]>>>(), pointer);
  assert_eq!(layout::<NonZeroInt<U0, Box<[u64; 0]>>>(), pointer);
  send_and_sync::<Uint<U256, Box<[u64; 4]>>>();
  send_and_sync::<Int<U20, Box<[u8; 3]>>>();
  send_and_sync::<NonZeroUint<U256, Box<[u64; 4]>>>();
  send_and_sync::<NonZeroInt<U20, Box<[u8; 3]>>>();
}

/// The results of the conversions of an integer from a slice, as the limbs of the integers: the
/// checked, the wrapping, the overflowing and the saturating conversion, and the strict one where
/// the checked one returns a value.
type Reads<A> = (Option<A>, A, (A, bool), A, Option<A>);

/// The [`Reads`] of `T` from the limbs of `slice`, through `ArrayInteger`.
fn limb_reads<T: ArrayInteger>(slice: &[T::Limb]) -> Reads<T::Array> {
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
fn byte_reads<T: IntegerBytes + ArrayInteger>(bytes: &[u8]) -> [Reads<T::Array>; 2] {
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
/// the exact one, the checked one from the nearest `f32`, and the one rounded with each rounding
/// mode.
type FloatReads<A> = (
  Option<A>,
  A,
  Option<A>,
  Result<A, FromFloatError>,
  Option<A>,
  Vec<Result<A, FromFloatError>>,
);

/// The [`FloatReads`] of `T` from `value`, through `IntegerFloat`.
fn float_reads<T: IntegerFloat + ArrayInteger>(value: f64) -> FloatReads<T::Array> {
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
    rounded,
  )
}

/// The floats at which the conversions from the floats are checked at `bits` bits: zero of both
/// signs, a few halves and integers, `2^(bits - 1)` and `2^bits` and their negations, the largest
/// float, an infinity and NaN.
fn float_values(bits: u32) -> Vec<f64> {
  let mut values = vec![0.0, -0.0, 0.5, -1.5, 2.5, 7.0, -1e19, 1e30, f64::MAX];
  for exponent in [bits.saturating_sub(1), bits] {
    let power = floats::pow2(exponent as i32);
    values.extend([power, -power]);
  }
  values.extend([f64::INFINITY, f64::NAN]);
  values
}

/// Emits a function that checks the construction of `$integer` of `B` bits in `Box<[L; N]>`, read
/// in two's complement if `$signed`, against its twin in `[L; N]`, through `ArrayInteger`, which
/// compares the limbs of the two: zero through `Default` and `FixedInteger::zero`, and the limits
/// through `FixedInteger`; at the patterns of [`row_patterns`] in `K` words, the limbs through
/// `ArrayInteger::checked_from_limbs`, the slices of limbs and of bytes cut from them through
/// `ArrayInteger` and `IntegerBytes`, the decimal and hexadecimal texts of the twin through
/// `FromStr` and `FixedInteger::from_str_radix`, the casts from the twin into a box and from a
/// borrowed box into the twin and into the native integers, and the conversions to the floats;
/// limbs above the width, which `checked_from_limbs` refuses; and the casts and `TryFrom` of the
/// native integers and the conversions from the floats of [`float_values`]. The function is
/// `#[inline(never)]`, so that its frame holds the values of this storage alone.
macro_rules! construction {
  ($name:ident, $integer:ident, $signed:literal) => {
    #[inline(never)]
    fn $name<B, L, const N: usize, const K: usize>()
    where
      B: FitsBits<[L; N]>,
      L: ArrayLimb + bigint::Limb + Debug + Ord + Shl<u32, Output = L>,
      [L; N]: ArrayRepr,
    {
      let bits = B::U32;
      let zero = $integer::<B, [L; N]>::ZERO.into_limbs();
      assert_eq!($integer::<B, Box<[L; N]>>::default().into_limbs(), zero);
      assert_eq!(
        <$integer<B, Box<[L; N]>> as FixedInteger>::zero().into_limbs(),
        zero
      );
      assert_eq!(
        <$integer<B, Box<[L; N]>> as FixedInteger>::min_value().into_limbs(),
        $integer::<B, [L; N]>::MIN.into_limbs()
      );
      assert_eq!(
        <$integer<B, Box<[L; N]>> as FixedInteger>::max_value().into_limbs(),
        $integer::<B, [L; N]>::MAX.into_limbs()
      );
      for pattern in row_patterns::<K>(bits, $signed) {
        let limbs: [L; N] = bigint::limbs_of_pattern(&pattern, $signed);
        let twin = <$integer<B, [L; N]> as ArrayInteger>::checked_from_limbs(limbs);
        let twin = twin.expect("a value of the width");
        let boxed = <$integer<B, Box<[L; N]>> as ArrayInteger>::checked_from_limbs(limbs);
        let boxed = boxed.expect("a value of the width");
        assert_eq!((*boxed.as_limbs(), boxed.to_limbs()), (limbs, limbs));
        for slice in bigint::limb_slices::<L>(&pattern, $signed, N) {
          assert_eq!(
            limb_reads::<$integer<B, Box<[L; N]>>>(&slice),
            limb_reads::<$integer<B, [L; N]>>(&slice),
            "{slice:?} into {bits} bits"
          );
        }
        for bytes in bigint::byte_slices(&pattern, $signed) {
          assert_eq!(
            byte_reads::<$integer<B, Box<[L; N]>>>(&bytes),
            byte_reads::<$integer<B, [L; N]>>(&bytes),
            "{bytes:?} into {bits} bits"
          );
        }
        let text = twin.to_string();
        let parsed = text.parse::<$integer<B, Box<[L; N]>>>();
        assert_eq!(parsed.map(ArrayInteger::into_limbs), Ok(limbs), "{text}");
        let hex = format!("{twin:x}");
        assert_eq!(
          <$integer<B, Box<[L; N]>> as FixedInteger>::from_str_radix(&hex, 16)
            .map(ArrayInteger::into_limbs),
          <$integer<B, [L; N]> as FixedInteger>::from_str_radix(&hex, 16)
            .map(ArrayInteger::into_limbs),
          "{hex}"
        );
        let cast = $integer::<B, Box<[L; N]>>::checked_cast_from(twin);
        assert_eq!(cast.map(ArrayInteger::into_limbs), Some(limbs));
        let cast = $integer::<B, [L; N]>::checked_cast_from(&boxed);
        assert_eq!(cast.map(ArrayInteger::into_limbs), Some(limbs));
        assert_eq!(u64::checked_cast_from(&boxed), u64::checked_cast_from(twin));
        assert_eq!(
          i8::saturating_cast_from(&boxed),
          i8::saturating_cast_from(twin)
        );
        assert_eq!(
          u128::wrapping_cast_from(&boxed),
          u128::wrapping_cast_from(twin)
        );
        assert_eq!(boxed.to_f64().to_bits(), twin.to_f64().to_bits());
        assert_eq!(boxed.to_f32().to_bits(), twin.to_f32().to_bits());
      }
      // Limbs with the bit at the width set hold a value outside the width, unless the width fills
      // them; for `Int`, a value whose sign is not copied above the width.
      let storage_bits = <L as bigint::Limb>::BITS * N as u32;
      if bits < storage_bits {
        let above = model::pattern(storage_bits, |i| i == bits);
        let limbs: [L; N] = bigint::limbs_of_pattern(&above, false);
        let boxed = <$integer<B, Box<[L; N]>> as ArrayInteger>::checked_from_limbs(limbs);
        assert!(boxed.is_none(), "{limbs:?} into {bits} bits");
      }
      for value in [0, 1, u64::MAX.into(), u128::MAX] {
        let boxed = $integer::<B, Box<[L; N]>>::checked_cast_from(value);
        let twin = $integer::<B, [L; N]>::checked_cast_from(value);
        assert_eq!(
          boxed.map(ArrayInteger::into_limbs),
          twin.map(ArrayInteger::into_limbs)
        );
        let boxed = $integer::<B, Box<[L; N]>>::wrapping_cast_from(value).into_limbs();
        assert_eq!(
          boxed,
          $integer::<B, [L; N]>::wrapping_cast_from(value).into_limbs()
        );
      }
      for value in [i128::MIN, -1, 0, i128::MAX] {
        let boxed = $integer::<B, Box<[L; N]>>::saturating_cast_from(value).into_limbs();
        assert_eq!(
          boxed,
          $integer::<B, [L; N]>::saturating_cast_from(value).into_limbs()
        );
      }
      assert_eq!(
        $integer::<B, Box<[L; N]>>::try_from(usize::MAX).map(ArrayInteger::into_limbs),
        $integer::<B, [L; N]>::try_from(usize::MAX).map(ArrayInteger::into_limbs)
      );
      assert_eq!(
        $integer::<B, Box<[L; N]>>::try_from(isize::MIN).map(ArrayInteger::into_limbs),
        $integer::<B, [L; N]>::try_from(isize::MIN).map(ArrayInteger::into_limbs)
      );
      for value in float_values(bits) {
        assert_eq!(
          float_reads::<$integer<B, Box<[L; N]>>>(value),
          float_reads::<$integer<B, [L; N]>>(value),
          "{value:e} into {bits} bits"
        );
      }
    }
  };
}

construction!(uint_construction, Uint, false);
construction!(int_construction, Int, true);

/// Checks `FromLimbs` and `From` of the pointer into `Uint` and `Int` of `B` bits in `Box<[L; N]>`,
/// a width that fills the array, at the patterns of [`row_patterns`] in `K` words: each gives the
/// limbs that it takes. The function is `#[inline(never)]`, so that its frame holds the values of
/// this storage alone.
#[inline(never)]
fn filled_construction<B, L, const N: usize, const K: usize>()
where
  B: FillsRepr<[L; N]>,
  L: ArrayLimb + bigint::Limb + Debug,
  [L; N]: ArrayRepr,
{
  for signed in [false, true] {
    for pattern in row_patterns::<K>(B::U32, signed) {
      let limbs: [L; N] = bigint::limbs_of_pattern(&pattern, signed);
      let uint = <Uint<B, Box<[L; N]>> as FromLimbs>::from_limbs(limbs);
      assert_eq!(uint.into_limbs(), limbs);
      let int = <Int<B, Box<[L; N]>> as FromLimbs>::from_limbs(limbs);
      assert_eq!(int.into_limbs(), limbs);
      let uint = Uint::<B, Box<[L; N]>>::from(Box::new(limbs));
      assert_eq!(*Box::<[L; N]>::from(uint), limbs);
      let int = Int::<B, Box<[L; N]>>::from(Box::new(limbs));
      assert_eq!(*Box::<[L; N]>::from(int), limbs);
    }
  }
}

/// Each way of building a boxed integer gives the limbs of its inline twin: zero, the limits, the
/// limbs, the slices of limbs and of bytes, the text, the casts from an inline integer, also
/// borrowed and into a native integer, the native integers and the floats, in the storages of
/// [`box_rows_agree_with_the_array_rows`], fewer under Miri; `FromLimbs` and `From` of the pointer
/// at widths that fill the array; and, at 256 bits in four 64-bit limbs, `From` of the native
/// integers, `widen` both ways, `IntegerOne`, and the arrays of bytes.
#[test]
fn construction_of_boxed_integers() {
  uint_construction::<U0, u64, 0, 1>();
  int_construction::<U0, u64, 0, 1>();
  uint_construction::<U65, u64, 2, 2>();
  int_construction::<U65, u64, 2, 2>();
  uint_construction::<U20, u8, 3, 1>();
  int_construction::<U20, u8, 3, 1>();
  filled_construction::<U0, u64, 0, 1>();
  filled_construction::<U256, u64, 4, 4>();
  if !tier().is_miri() {
    uint_construction::<U1, u64, 1, 1>();
    int_construction::<U1, u64, 1, 1>();
    uint_construction::<U200, u64, 4, 4>();
    int_construction::<U200, u64, 4, 4>();
    uint_construction::<U24, u16, 2, 1>();
    int_construction::<U24, u16, 2, 1>();
    uint_construction::<U129, u128, 2, 4>();
    int_construction::<U129, u128, 2, 4>();
    filled_construction::<U64, u64, 1, 1>();
    filled_construction::<U24, u8, 3, 1>();
    filled_construction::<U256, u128, 2, 4>();
    filled_construction::<U200, u8, 25, 4>();
  }

  type U = Uint<U256, Box<[u64; 4]>>;
  type I = Int<U256, Box<[u64; 4]>>;
  assert_eq!(
    U::from(u64::MAX).into_limbs(),
    u256::from(u64::MAX).into_limbs()
  );
  assert_eq!(U::from(true).into_limbs(), u256::ONE.into_limbs());
  assert_eq!(
    I::from(i64::MIN).into_limbs(),
    i256::from(i64::MIN).into_limbs()
  );
  assert_eq!(
    I::from(u128::MAX).into_limbs(),
    i256::from(u128::MAX).into_limbs()
  );
  assert_eq!(
    <U as IntegerOne>::one().into_limbs(),
    u256::ONE.into_limbs()
  );
  assert_eq!(
    <I as IntegerOne>::one().into_limbs(),
    i256::ONE.into_limbs()
  );
  let widened: U = u7::new(100).widen();
  assert_eq!(widened.into_limbs(), [100, 0, 0, 0]);
  let widened: I = i7::new(-50).widen();
  assert_eq!(widened.into_limbs(), i256::from(-50i8).into_limbs());
  let boxed = U::from(u64::MAX);
  let wide = Uint::<U300, [u64; 5]>::widen_from(&boxed);
  assert_eq!(wide.to_limbs(), [u64::MAX, 0, 0, 0, 0]);
  assert_eq!(
    boxed.widen::<Uint<U300, Box<[u64; 5]>>>().into_limbs(),
    wide.to_limbs()
  );
  let bytes: [u8; 32] = core::array::from_fn(|i| i as u8 * 7);
  assert_eq!(
    <U as FromByteArray>::from_le_bytes(bytes).into_limbs(),
    u256::from_le_bytes(bytes).into_limbs()
  );
  assert_eq!(
    <I as FromByteArray>::from_be_bytes(bytes).into_limbs(),
    i256::from_be_bytes(bytes).into_limbs()
  );
  assert_eq!(
    U::from_le_bytes(bytes).to_be_bytes(),
    u256::from_le_bytes(bytes).to_be_bytes()
  );
}
