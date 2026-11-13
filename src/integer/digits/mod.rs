//! The digits of [`Uint`] in a base: the trait [`UnsignedDigits`], the constructors from digits in
//! bytes that `digit_methods!` emits as inherent `const fn`s of every inline storage, and the
//! methods over every storage, which read digits in `u64` from an iterator and write them.
//!
//! Two stems, each with one role. A `radix`, from 2 to 256, has its digits in bytes: the
//! constructors read them from a slice, also in constant contexts. A `base`, from 2 up, has its
//! digits in `u64`: the constructors read them from an iterator, and `to_base_le` writes them as
//! one, without allocation. A constructor returns `None` for a digit that is not below its base and
//! for a value outside the width; no digits are zero, and zeros above the value are read. A radix
//! or a base outside its range panics, reporting the caller, before any digit is read.
//!
//! Every value is computed on the inline storage, a native integer or an array of limbs: the
//! iterator of the digits keeps the quotient of each division of the lent value, and a reader
//! stores its value only once it is known, so that a heap storage allocates only the integer that
//! a constructor returns.

use core::iter::FusedIterator;

use super::{Uint, traits::Sealed};
use crate::{
  layout::Token,
  width::{FitsBits, UintCore, UintRepr},
};

/// Panics with `message` unless `radix` is from 2 to `max`, reporting the caller.
#[inline]
#[track_caller]
pub(crate) const fn check_radix(radix: u32, max: u32, message: &str) {
  if radix < 2 || radix > max {
    panic!("{}", message);
  }
}

/// Panics with `message` unless `base` is at least 2, reporting the caller.
#[inline]
#[track_caller]
pub(crate) const fn check_base(base: u64, message: &str) {
  if base < 2 {
    panic!("{}", message);
  }
}

/// The largest power of `base` in a `u64`, and its exponent: the digits that one division by it
/// gives. Requires `base >= 2`.
#[inline]
const fn chunk_of(base: u64) -> (u64, u32) {
  let per_chunk = u64::MAX.ilog(base);
  (base.pow(per_chunk), per_chunk)
}

/// The number of digits of `base` that a writer reserves for a value of `bit_len` significant
/// bits, `ceil(bit_len / floor(log2(base)))`: at least the number of its digits, as `base^count`
/// is at least `2^bit_len`. Requires `base >= 2`.
#[cfg(any(feature = "std", feature = "alloc"))]
#[inline]
fn digit_capacity(bit_len: u32, base: u64) -> usize {
  // A count that `usize` does not hold belongs to a value that no memory holds, and the
  // allocation then fails as it does for any such capacity.
  usize::try_from(bit_len.div_ceil(base.ilog2())).unwrap_or(usize::MAX)
}

/// The digits of a value in a base, least significant first, without the zeros above the highest
/// one: what is left of the value is divided by the largest power of the base in a `u64`, and the
/// digits of a chunk are taken from the remainder.
struct BaseDigits<I> {
  /// What is left of the value above the chunks that were divided off.
  rest: I,
  /// The base.
  base: u64,
  /// The largest power of `base` in a `u64`.
  power: u64,
  /// Its exponent: the digits of a chunk.
  per_chunk: u32,
  /// The digits of the current chunk that were not given yet.
  chunk: u64,
  /// Their number, if the chunk is not the last.
  left: u32,
  /// Whether `rest` is zero, so that the current chunk ends at its highest digit.
  last: bool,
}

impl<I: UintCore> BaseDigits<I> {
  /// The digits of `value` in `base`, from the first division of the lent value, whose quotient
  /// it keeps. Requires `base >= 2`.
  #[inline]
  fn new(value: &I, base: u64) -> Self {
    let (power, per_chunk) = chunk_of(base);
    let (rest, chunk) = value.div_rem_u64(power, Token::new());
    let last = rest.is_zero(Token::new());
    Self {
      rest,
      base,
      power,
      per_chunk,
      chunk,
      left: per_chunk,
      last,
    }
  }
}

impl<I: UintCore> Iterator for BaseDigits<I> {
  type Item = u64;

  #[inline]
  fn next(&mut self) -> Option<u64> {
    if self.left == 0 {
      if self.last {
        return None;
      }
      (self.rest, self.chunk) = self.rest.div_rem_u64(self.power, Token::new());
      self.last = self.rest.is_zero(Token::new());
      self.left = self.per_chunk;
    }
    // A chunk below others has all its digits, its zeros among them; the last one ends at its
    // highest digit that is not zero, and zero has none.
    if self.last && self.chunk == 0 {
      self.left = 0;
      return None;
    }
    let digit = self.chunk % self.base;
    self.chunk /= self.base;
    self.left -= 1;
    Some(digit)
  }
}

// After its first `None`, `left` is zero and `last` is set, so it returns `None` for ever.
impl<I: UintCore> FusedIterator for BaseDigits<I> {}

/// The value of `digits` in `base`, most significant first, as an integer of `bits` bits, or
/// `None` if a digit is not below `base` or the value is `2^bits` or more. Requires `base >= 2`.
fn read_be<I: UintCore>(digits: impl Iterator<Item = u64>, base: u64, bits: u32) -> Option<I> {
  let (_, per_chunk) = chunk_of(base);
  let mut value = I::zero(Token::new());
  // The digits since the last chunk was added, `base` to their number, and their number: the first
  // two stay at most `base^per_chunk <= u64::MAX`.
  let (mut chunk, mut scale, mut count) = (0u64, 1u64, 0u32);
  for digit in digits {
    if digit >= base {
      return None;
    }
    chunk = chunk * base + digit;
    scale *= base;
    count += 1;
    if count == per_chunk {
      value = value.checked_mul_add_u64(scale, chunk, bits, Token::new())?;
      (chunk, scale, count) = (0, 1, 0);
    }
  }
  if count > 0 {
    value = value.checked_mul_add_u64(scale, chunk, bits, Token::new())?;
  }
  Some(value)
}

/// The value of `digits` in `base`, least significant first, as an integer of `bits` bits, or
/// `None` if a digit is not below `base` or the value is `2^bits` or more. Requires `base >= 2`.
fn read_le<I: UintCore>(digits: impl Iterator<Item = u64>, base: u64, bits: u32) -> Option<I> {
  let (_, per_chunk) = chunk_of(base);
  let mut value = I::zero(Token::new());
  // The weight of the next chunk, or `None` once it is `2^bits` or more, as one is at zero bits.
  let mut weight = I::zero(Token::new()).checked_mul_add_u64(0, 1, bits, Token::new());
  // The digits since the last chunk was added, weighted by `base` to their position in it, the
  // weight of the next one, and their number: the first two stay at most `base^per_chunk`.
  let (mut chunk, mut scale, mut count) = (0u64, 1u64, 0u32);
  for digit in digits {
    if digit >= base {
      return None;
    }
    chunk += digit * scale;
    count += 1;
    if count == per_chunk {
      add_chunk(&mut value, &mut weight, chunk, base, count, bits)?;
      (chunk, scale, count) = (0, 1, 0);
    } else {
      scale *= base;
    }
  }
  add_chunk(&mut value, &mut weight, chunk, base, count, bits)?;
  Some(value)
}

/// Adds `chunk` times `weight` to `value`, and scales `weight` by `base^count`, or returns `None`
/// if the sum is `2^bits` or more. A `weight` of `None` is `2^bits` or more, which only a zero
/// chunk leaves the sum below. Requires `count <= chunk_of(base).1`.
fn add_chunk<I: UintCore>(
  value: &mut I,
  weight: &mut Option<I>,
  chunk: u64,
  base: u64,
  count: u32,
  bits: u32,
) -> Option<()> {
  if chunk != 0 {
    let term = weight
      .as_ref()?
      .checked_mul_add_u64(chunk, 0, bits, Token::new())?;
    let (sum, overflow) = value.overflowing_add_bits(&term, bits, Token::new());
    if overflow {
      return None;
    }
    *value = sum;
  }
  if count > 0
    && let Some(current) = weight.as_ref()
  {
    *weight = current.checked_mul_add_u64(base.pow(count), 0, bits, Token::new());
  }
  Some(())
}

/// The digits of `value`, of `bits` bits, in `radix` as bytes, most significant first if
/// `big_endian`: the one digit 0 for zero. Requires `2 <= radix <= 256`.
#[cfg(any(feature = "std", feature = "alloc"))]
fn radix_digits<I: UintCore>(
  value: &I,
  radix: u32,
  bits: u32,
  big_endian: bool,
) -> std::vec::Vec<u8> {
  let bit_len = bits - value.leading_zeros_bits(bits, Token::new());
  let mut digits = std::vec::Vec::with_capacity(digit_capacity(bit_len, radix.into()).max(1));
  // Each digit is below `radix`, at most 256, so it fits a byte.
  digits.extend(BaseDigits::new(value, radix.into()).map(|digit| digit as u8));
  if digits.is_empty() {
    digits.push(0);
  }
  if big_endian {
    digits.reverse();
  }
  digits
}

/// The text of the magnitude `magnitude`, of `bits` bits, in `radix` with lowercase letters, after
/// `-` if `negative`: `"0"` for zero. Requires `2 <= radix <= 36`.
#[cfg(any(feature = "std", feature = "alloc"))]
pub(crate) fn write_text<I: UintCore>(
  magnitude: &I,
  negative: bool,
  radix: u32,
  bits: u32,
) -> std::string::String {
  const ALPHABET: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
  let bit_len = bits - magnitude.leading_zeros_bits(bits, Token::new());
  let capacity = digit_capacity(bit_len, radix.into())
    .max(1)
    .saturating_add(usize::from(negative));
  let mut text = std::vec::Vec::with_capacity(capacity);
  // Each digit is below `radix`, at most 36, so it indexes the alphabet.
  text.extend(BaseDigits::new(magnitude, radix.into()).map(|digit| ALPHABET[digit as usize]));
  if text.is_empty() {
    text.push(b'0');
  }
  if negative {
    text.push(b'-');
  }
  text.reverse();
  match std::string::String::from_utf8(text) {
    Ok(text) => text,
    Err(_) => not_ascii(),
  }
}

/// Panics on a text that is not ASCII: a bug of this crate, as every byte of the text comes from
/// the alphabet or is a sign.
#[cfg(any(feature = "std", feature = "alloc"))]
#[cold]
fn not_ascii() -> ! {
  panic!("the digits of a text are ASCII")
}

/// Emits the constructors of `Uint` from digits in bytes as inherent `const fn`s of one inline
/// storage, in an impl block whose width relation holds: `$args` is `[layer]` for a native integer
/// and `[group, layer, &]` for an array, as for `byte_methods!`.
// A `const fn` cannot call the rows of a trait, so the functions are written here once for every
// storage.
macro_rules! digit_methods {
  (@layer [$group:ident, $layer:ident, &] $f:ident($($arg:expr),*)) => {
    int::$group::$layer::$f($($arg),*)
  };
  (@layer [$layer:ident] $f:ident($($arg:expr),*)) => {
    int::$layer::$f($($arg),*)
  };
  ($args:tt) => {
    /// Creates an integer from its digits in `radix`, a byte each, most significant first, or
    /// returns `None` if a digit is not below `radix` or the value is above `MAX`. No digits are
    /// zero, and zeros above the value are read, whatever their number. In the radix 256 the
    /// digits are the bytes of the value, as
    /// [`checked_from_be_slice`](Self::checked_from_be_slice) reads them.
    ///
    /// # Panics
    ///
    /// Panics if `radix` is not from 2 to 256, reporting the caller, so in a constant such a
    /// radix fails to compile.
    #[must_use]
    #[inline]
    #[track_caller]
    pub const fn checked_from_radix_be(digits: &[u8], radix: u32) -> Option<Self> {
      digits::check_radix(
        radix,
        256,
        "checked_from_radix_be: radix must lie in the range `[2, 256]`",
      );
      match digit_methods!(@layer $args checked_read_digits(digits, true, radix, Self::BITS)) {
        Some(repr) => Some(Self::wrap(repr)),
        None => None,
      }
    }

    /// Creates an integer from its digits in `radix`, a byte each, least significant first, as
    /// [`checked_from_radix_be`](Self::checked_from_radix_be) does from them in reverse order.
    ///
    /// # Panics
    ///
    /// Panics if `radix` is not from 2 to 256, reporting the caller, so in a constant such a
    /// radix fails to compile.
    #[must_use]
    #[inline]
    #[track_caller]
    pub const fn checked_from_radix_le(digits: &[u8], radix: u32) -> Option<Self> {
      digits::check_radix(
        radix,
        256,
        "checked_from_radix_le: radix must lie in the range `[2, 256]`",
      );
      match digit_methods!(@layer $args checked_read_digits(digits, false, radix, Self::BITS)) {
        Some(repr) => Some(Self::wrap(repr)),
        None => None,
      }
    }
  };
}

/// The digits of an unsigned integer in a base: [`Uint`] over every storage.
///
/// The methods are the inherent items of the same names, which a concrete type reaches without this
/// trait; the constructors from digits in bytes are also `const fn`s of each inline storage. Two
/// stems, each with one role: a `radix`, from 2 to 256, has its digits in bytes, and a `base`, from
/// 2 up, has its digits in `u64`, read from an iterator and written as one. A constructor returns
/// `None` for a digit that is not below its base and for a value above `MAX`; no digits are zero.
/// The digits of a base have no digit for zero, and those of a radix the one digit 0. A radix or a
/// base outside its range panics, reporting the caller.
///
/// ```
/// use numo::{BitWidth, Uint, UnsignedDigits, u7};
///
/// // The value of digits in a base, read back from those of the value.
/// fn again<T: UnsignedDigits>(value: &T, base: u64) -> Option<T> {
///   T::checked_from_base_le(value.to_base_le(base), base)
/// }
///
/// assert_eq!(again(&u7::new(100), 7), Some(u7::new(100)));
/// assert_eq!(u7::new(100).to_base_le(7).collect::<Vec<u64>>(), [2, 0, 2]);
/// assert_eq!(<u7 as UnsignedDigits>::checked_from_radix_be(&[1, 2, 7], 10), Some(u7::new(127)));
/// assert_eq!(<u7 as UnsignedDigits>::checked_from_radix_be(&[1, 2, 8], 10), None);
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
#[diagnostic::on_unimplemented(
  message = "the digits in a base are those of an unsigned integer",
  label = "not a `Uint`",
  note = "an `Int` gives the digits of its magnitude through `unsigned_abs`, and those of its bits through `cast_unsigned`"
)]
pub trait UnsignedDigits: Sealed + Sized {
  /// Creates an integer from its digits in `radix`, a byte each, most significant first, or
  /// returns `None` if a digit is not below `radix` or the value is above `MAX`. No digits are
  /// zero, and zeros above the value are read.
  ///
  /// # Panics
  ///
  /// Panics if `radix` is not from 2 to 256, reporting the caller.
  #[must_use]
  #[track_caller]
  fn checked_from_radix_be(digits: &[u8], radix: u32) -> Option<Self>;

  /// Creates an integer from its digits in `radix`, a byte each, least significant first, as
  /// [`checked_from_radix_be`](Self::checked_from_radix_be) does from them in reverse order.
  ///
  /// # Panics
  ///
  /// Panics if `radix` is not from 2 to 256, reporting the caller.
  #[must_use]
  #[track_caller]
  fn checked_from_radix_le(digits: &[u8], radix: u32) -> Option<Self>;

  /// Creates an integer from its digits in `base`, most significant first, read to the end of the
  /// iterator, or returns `None` if a digit is not below `base` or the value is above `MAX`. No
  /// digits are zero, and zeros above the value are read.
  ///
  /// # Panics
  ///
  /// Panics if `base` is below 2, reporting the caller.
  #[must_use]
  #[track_caller]
  fn checked_from_base_be<I: IntoIterator<Item = u64>>(digits: I, base: u64) -> Option<Self>;

  /// Creates an integer from its digits in `base`, least significant first, as
  /// [`checked_from_base_be`](Self::checked_from_base_be) does from them in reverse order.
  ///
  /// # Panics
  ///
  /// Panics if `base` is below 2, reporting the caller.
  #[must_use]
  #[track_caller]
  fn checked_from_base_le<I: IntoIterator<Item = u64>>(digits: I, base: u64) -> Option<Self>;

  /// The digits of the value in `base`, least significant first, without the zeros above the
  /// highest one, so zero has none. The iterator holds what is left of the value, not a borrow of
  /// it, and allocates nothing.
  ///
  /// # Panics
  ///
  /// Panics if `base` is below 2, reporting the caller, when it is called.
  #[track_caller]
  fn to_base_le(&self, base: u64) -> impl FusedIterator<Item = u64> + use<Self>;

  /// The digits of the value in `base`, most significant first, without the zeros above the
  /// highest one, so zero has none. They are collected in a `Vec`, which a value that is not zero
  /// allocates.
  ///
  /// # Panics
  ///
  /// Panics if `base` is below 2, reporting the caller, when it is called.
  #[cfg(any(feature = "std", feature = "alloc"))]
  #[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
  #[track_caller]
  fn to_base_be(&self, base: u64) -> impl FusedIterator<Item = u64> + use<Self>;

  /// The digits of the value in `radix`, a byte each, most significant first: the one digit 0 for
  /// zero.
  ///
  /// # Panics
  ///
  /// Panics if `radix` is not from 2 to 256, reporting the caller.
  #[cfg(any(feature = "std", feature = "alloc"))]
  #[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
  #[must_use]
  #[track_caller]
  fn to_radix_be(&self, radix: u32) -> std::vec::Vec<u8>;

  /// The digits of the value in `radix`, a byte each, least significant first: the one digit 0
  /// for zero.
  ///
  /// # Panics
  ///
  /// Panics if `radix` is not from 2 to 256, reporting the caller.
  #[cfg(any(feature = "std", feature = "alloc"))]
  #[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
  #[must_use]
  #[track_caller]
  fn to_radix_le(&self, radix: u32) -> std::vec::Vec<u8>;
}

// The constructors from a slice call the row of the storage, which a heap storage answers with one
// allocation for a value and none for `None`; the other methods are the inherent ones.
impl<B: FitsBits<R>, R: UintRepr> UnsignedDigits for Uint<B, R> {
  #[inline]
  #[track_caller]
  fn checked_from_radix_be(digits: &[u8], radix: u32) -> Option<Self> {
    check_radix(
      radix,
      256,
      "checked_from_radix_be: radix must lie in the range `[2, 256]`",
    );
    R::checked_read_digits(digits, true, radix, B::U32, Token::new()).map(Self::wrap)
  }

  #[inline]
  #[track_caller]
  fn checked_from_radix_le(digits: &[u8], radix: u32) -> Option<Self> {
    check_radix(
      radix,
      256,
      "checked_from_radix_le: radix must lie in the range `[2, 256]`",
    );
    R::checked_read_digits(digits, false, radix, B::U32, Token::new()).map(Self::wrap)
  }

  #[inline]
  #[track_caller]
  fn checked_from_base_be<I: IntoIterator<Item = u64>>(digits: I, base: u64) -> Option<Self> {
    Self::checked_from_base_be(digits, base)
  }

  #[inline]
  #[track_caller]
  fn checked_from_base_le<I: IntoIterator<Item = u64>>(digits: I, base: u64) -> Option<Self> {
    Self::checked_from_base_le(digits, base)
  }

  #[inline]
  #[track_caller]
  fn to_base_le(&self, base: u64) -> impl FusedIterator<Item = u64> + use<B, R> {
    Self::to_base_le(self, base)
  }

  #[cfg(any(feature = "std", feature = "alloc"))]
  #[inline]
  #[track_caller]
  fn to_base_be(&self, base: u64) -> impl FusedIterator<Item = u64> + use<B, R> {
    Self::to_base_be(self, base)
  }

  #[cfg(any(feature = "std", feature = "alloc"))]
  #[inline]
  #[track_caller]
  fn to_radix_be(&self, radix: u32) -> std::vec::Vec<u8> {
    Self::to_radix_be(self, radix)
  }

  #[cfg(any(feature = "std", feature = "alloc"))]
  #[inline]
  #[track_caller]
  fn to_radix_le(&self, radix: u32) -> std::vec::Vec<u8> {
    Self::to_radix_le(self, radix)
  }
}

// One item for every storage: a method that cannot be `const` gains nothing from a copy for each
// storage, and a heap storage has it too. Each computes on the inline storage, and a constructor
// stores the value that it returns, after its last check.
impl<B: FitsBits<R>, R: UintRepr> Uint<B, R> {
  /// Creates an integer from its digits in `base`, most significant first, read to the end of the
  /// iterator, or returns `None` if a digit is not below `base` or the value is above `MAX`. No
  /// digits are zero, and zeros above the value are read, whatever their number. The digits are
  /// gathered in runs that a `u64` holds, so a value of `n` limbs costs about `n` limb operations
  /// for each such run.
  ///
  /// ```
  /// use numo::u7;
  ///
  /// assert_eq!(u7::checked_from_base_be([1, 2, 7], 10), Some(u7::new(127)));
  /// assert_eq!(u7::checked_from_base_be([0, 0, 1, 2, 7], 10), Some(u7::new(127)));
  /// assert_eq!(u7::checked_from_base_be([1, 2, 8], 10), None);
  /// assert_eq!(u7::checked_from_base_be([1, 10], 10), None);
  /// assert_eq!(u7::checked_from_base_be([], 10), Some(u7::ZERO));
  /// ```
  ///
  /// # Panics
  ///
  /// Panics if `base` is below 2, reporting the caller.
  #[must_use]
  #[inline]
  #[track_caller]
  pub fn checked_from_base_be<I: IntoIterator<Item = u64>>(digits: I, base: u64) -> Option<Self> {
    check_base(base, "checked_from_base_be: base must be at least 2");
    let value = read_be::<R::Inline>(digits.into_iter(), base, B::U32)?;
    Some(Self::wrap(R::from_inline(value, Token::new())))
  }

  /// Creates an integer from its digits in `base`, least significant first, as
  /// [`checked_from_base_be`](Self::checked_from_base_be) does from them in reverse order. The
  /// digits are read in the order of the iterator, which needs no end at either side, so
  /// [`to_base_le`](Self::to_base_le) reads back.
  ///
  /// ```
  /// use numo::u7;
  ///
  /// assert_eq!(u7::checked_from_base_le([7, 2, 1], 10), Some(u7::new(127)));
  /// assert_eq!(u7::checked_from_base_le([7, 2, 1, 0, 0], 10), Some(u7::new(127)));
  /// assert_eq!(u7::checked_from_base_le(u7::new(100).to_base_le(3), 3), Some(u7::new(100)));
  /// ```
  ///
  /// # Panics
  ///
  /// Panics if `base` is below 2, reporting the caller.
  #[must_use]
  #[inline]
  #[track_caller]
  pub fn checked_from_base_le<I: IntoIterator<Item = u64>>(digits: I, base: u64) -> Option<Self> {
    check_base(base, "checked_from_base_le: base must be at least 2");
    let value = read_le::<R::Inline>(digits.into_iter(), base, B::U32)?;
    Some(Self::wrap(R::from_inline(value, Token::new())))
  }

  /// The digits of the value in `base`, least significant first, without the zeros above the
  /// highest one, so zero has none, as an iterator that allocates nothing. It holds what is left
  /// of the value, not a borrow of it, so a function can return the iterator of a local value, and
  /// it divides the value by the largest power of `base` in a `u64` once for each run of that many
  /// digits.
  ///
  /// ```
  /// use numo::{Uint, typenum::U100, u7};
  ///
  /// assert_eq!(u7::new(100).to_base_le(10).collect::<Vec<u64>>(), [0, 0, 1]);
  /// assert_eq!(u7::ZERO.to_base_le(10).count(), 0);
  /// // The 32-bit words of a value of 100 bits, least significant first.
  /// let words: Vec<u64> = Uint::<U100>::MAX.to_base_le(1 << 32).collect();
  /// assert_eq!(words, [0xffff_ffff, 0xffff_ffff, 0xffff_ffff, 0xf]);
  /// ```
  ///
  /// # Panics
  ///
  /// Panics if `base` is below 2, reporting the caller, when it is called.
  #[inline]
  #[track_caller]
  pub fn to_base_le(&self, base: u64) -> impl FusedIterator<Item = u64> + use<B, R> {
    check_base(base, "to_base_le: base must be at least 2");
    BaseDigits::new(self.repr.inline(Token::new()), base)
  }

  /// The digits of the value in `base`, most significant first, without the zeros above the
  /// highest one, so zero has none: those of [`to_base_le`](Self::to_base_le) in reverse order,
  /// collected in a `Vec` of the capacity that they need, which zero does not allocate. Without
  /// the `std` and the `alloc` features, a caller fills a buffer from `to_base_le` and reads it
  /// backwards.
  ///
  /// ```
  /// use numo::u7;
  ///
  /// assert_eq!(u7::new(100).to_base_be(10).collect::<Vec<u64>>(), [1, 0, 0]);
  /// assert_eq!(u7::ZERO.to_base_be(10).count(), 0);
  /// ```
  ///
  /// # Panics
  ///
  /// Panics if `base` is below 2, reporting the caller, when it is called.
  #[cfg(any(feature = "std", feature = "alloc"))]
  #[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
  #[inline]
  #[track_caller]
  pub fn to_base_be(&self, base: u64) -> impl FusedIterator<Item = u64> + use<B, R> {
    check_base(base, "to_base_be: base must be at least 2");
    let value = self.repr.inline(Token::new());
    let bit_len = B::U32 - value.leading_zeros_bits(B::U32, Token::new());
    let mut digits = std::vec::Vec::with_capacity(digit_capacity(bit_len, base));
    digits.extend(BaseDigits::new(value, base));
    digits.into_iter().rev()
  }

  /// The digits of the value in `radix`, a byte each, most significant first: the one digit 0 for
  /// zero. Mapped through an alphabet of `radix` symbols they are the text of the value, such as
  /// base 58; in the radix 256 they are the bytes of a value that is not zero, without the zeros
  /// above it. The `Vec` is allocated once, with the capacity that the digits need.
  ///
  /// ```
  /// use numo::u7;
  ///
  /// assert_eq!(u7::new(100).to_radix_be(10), [1, 0, 0]);
  /// assert_eq!(u7::ZERO.to_radix_be(10), [0]);
  /// assert_eq!(u7::checked_from_radix_be(&u7::new(100).to_radix_be(58), 58), Some(u7::new(100)));
  /// ```
  ///
  /// # Panics
  ///
  /// Panics if `radix` is not from 2 to 256, reporting the caller.
  #[cfg(any(feature = "std", feature = "alloc"))]
  #[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
  #[must_use]
  #[inline]
  #[track_caller]
  pub fn to_radix_be(&self, radix: u32) -> std::vec::Vec<u8> {
    check_radix(
      radix,
      256,
      "to_radix_be: radix must lie in the range `[2, 256]`",
    );
    radix_digits(self.repr.inline(Token::new()), radix, B::U32, true)
  }

  /// The digits of the value in `radix`, a byte each, least significant first: the one digit 0
  /// for zero, and those of [`to_radix_be`](Self::to_radix_be) in reverse order.
  ///
  /// ```
  /// use numo::u7;
  ///
  /// assert_eq!(u7::new(100).to_radix_le(10), [0, 0, 1]);
  /// assert_eq!(u7::ZERO.to_radix_le(256), [0]);
  /// ```
  ///
  /// # Panics
  ///
  /// Panics if `radix` is not from 2 to 256, reporting the caller.
  #[cfg(any(feature = "std", feature = "alloc"))]
  #[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
  #[must_use]
  #[inline]
  #[track_caller]
  pub fn to_radix_le(&self, radix: u32) -> std::vec::Vec<u8> {
    check_radix(
      radix,
      256,
      "to_radix_le: radix must lie in the range `[2, 256]`",
    );
    radix_digits(self.repr.inline(Token::new()), radix, B::U32, false)
  }
}
