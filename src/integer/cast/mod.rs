//! The casts between integers of any width and sign: [`CheckedCastFrom`], [`BoundedCastFrom`],
//! [`WidenFrom`] and [`TruncateFrom`], which the target implements, for [`Uint`], [`Int`], the
//! non-zero integers [`NonZeroUint`] and [`NonZeroInt`], and the native integers and their
//! `NonZero`, with the semantics of the native casts, still unstable.
//!
//! Each trait has one blanket implementation over the sealed traits of this module, so that every
//! pair of participants converts through the same code. A source gives the 64-bit words of its
//! value in two's complement, `CastSource`, and a target builds a value from such words,
//! `CheckedTarget` and `BoundedTarget`: [`Uint`] and [`Int`] through the sealed `word` and
//! `from_words` of their storage, the non-zero integers through those of the storage of their
//! integer, which `with_nonzero` lends, and of its inline storage, which holds the words until
//! they are known not to make zero, before a storage is built, and the native integers
//! and their `NonZero` through `u128` and `i128`. A non-zero integer is a checked target, which
//! takes no value of zero, and not a bounded one, as a wrapped or saturated value may be zero. A
//! value is in the range of a target when it is not negative or the target is signed, and its bits
//! from the sign bit of the target up, or from the width of an unsigned target up, copy its sign:
//! the check reads the words of the source from the first that the target does not hold, with no
//! allocation and no constant. `MaxWidth` and `MinWidth` give the width and the sign of each
//! participant, and whether it holds only values that are not zero, as `typenum` numbers, which the
//! bounds of `widen` and `truncate` compare at compile time, and the sealed markers carry the
//! diagnostics of a pair that fails them. The sealed traits are public in a private module, as the
//! other sealed traits of the crate are, so that the public implementations name no private item.
//! A reference to a source is a source of the same words and widths, so that every cast function
//! reads a borrowed value, through any number of references. The identities of `usize`, `isize`
//! and their `NonZero`, which are written out rather than derived from the widths, take a value or
//! one reference.

use core::num::TryFromIntError;

use typenum::{B0, B1, IsLessOrEqual, LeEq, U8, U16, U32, U64, U128};

use sealed::{
  BoundedTarget, CastPair, KindWidens, MaxWidth, MinWidth, SameSign, TruncateBits, WidenBits,
};
pub(super) use sealed::{CastSource, CheckedTarget};

use super::{Int, NonZeroInt, NonZeroUint, Uint, arithmetic::overflow, convert, nonzero};
use crate::{
  int,
  layout::Token,
  width::{self, FitsBits, IntCore, IntRepr, NonZeroIntRepr, NonZeroUintRepr, UintCore, UintRepr},
};

mod sealed {
  use typenum::{B0, B1};

  /// Seals the cast traits: implemented for every target of the casts with every source, `S`, so
  /// that no other crate can implement them.
  pub trait CastPair<S> {}

  /// What a cast reads of its source: [`Uint`](crate::Uint), [`Int`](crate::Int), the non-zero
  /// integers, and the native integers and their `NonZero`.
  pub trait CastSource {
    /// The number of bits of the type: `Bits`, or the `BITS` of a native integer.
    const BITS: u32;

    /// Whether the type is signed.
    const SIGNED: bool;

    /// The 64-bit word of the value at `index`, its bits from `64 * index` to `64 * index + 63`, in
    /// two's complement for a signed type: zero, or copies of the sign, beyond the type.
    fn word(&self, index: usize) -> u64;
  }

  /// What a checked cast builds: [`Uint`](crate::Uint), [`Int`](crate::Int), the non-zero
  /// integers, and the native integers and their `NonZero`.
  pub trait CheckedTarget: Sized {
    /// The number of bits of the type: `Bits`, or the `BITS` of a native integer.
    const BITS: u32;

    /// Whether the type is signed.
    const SIGNED: bool;

    /// The value whose 64-bit words in two's complement, least significant first, `word` gives,
    /// which is within the range of `Self`. Every value of an integer that may be zero is `Some`.
    fn from_words(word: impl Fn(usize) -> u64) -> Option<Self>;

    /// The value that [`from_words`](Self::from_words) returns, which the caller of
    /// `unchecked_cast_from` guarantees to be `Some`: `wrap_words` for an integer that may be zero.
    /// The function is safe, so it cannot rely on the guarantee: a type whose `from_words` can
    /// return `None` checks in every build and panics there.
    #[track_caller]
    fn assume_from_words(word: impl Fn(usize) -> u64) -> Self;
  }

  /// What a wrapping or saturating cast builds: a [`CheckedTarget`] that takes every value modulo
  /// `2^BITS`.
  pub trait BoundedTarget: CheckedTarget {
    /// The value whose 64-bit words in two's complement, least significant first, `word` gives,
    /// modulo `2^BITS`, read in two's complement for a signed type.
    fn wrap_words(word: impl Fn(usize) -> u64) -> Self;
  }

  /// The most bits of a value of the type, `Bits`, and its sign, `B0` or `B1`, as `typenum` types:
  /// what `widen` reads of a source and `truncate` of a target. `usize` and `isize`, whose width
  /// has no fixed upper bound, have no implementation.
  pub trait MaxWidth {
    /// `B1` for a signed type, and `B0` otherwise.
    type Sign;

    /// The width.
    type Bits;
  }

  /// The fewest bits that the type holds, `Bits`, its sign, and whether it holds only values that
  /// are not zero, `NonZero`, as `typenum` types: what `widen` reads of a target and `truncate` of
  /// a source. `usize` and `isize` hold at least 16 bits.
  pub trait MinWidth {
    /// `B1` for a signed type, and `B0` otherwise.
    type Sign;

    /// The width, or 16 for `usize` and `isize`.
    type Bits;

    /// `B1` for a type of values that are not zero, and `B0` for one that holds zero.
    type NonZero;
  }

  /// Implemented only for `B1`, the result of `Bits <= Bits` of the source and the target of
  /// `widen`.
  // No message names `Self`, which is a `typenum` bit.
  #[diagnostic::on_unimplemented(message = "`widen` needs a target at least as wide as the source")]
  pub trait WidenBits {}

  impl WidenBits for B1 {}

  /// Implemented only for `B1`, the result of `Bits <= Bits` of the target and the source of
  /// `truncate`.
  #[diagnostic::on_unimplemented(
    message = "`truncate` needs a target at most as wide as the source"
  )]
  pub trait TruncateBits {}

  impl TruncateBits for B1 {}

  /// Implemented for the pairs of equal signs, `B0` and `B0` and `B1` and `B1`, of the source and
  /// the target of `widen` and `truncate`.
  #[diagnostic::on_unimplemented(
    message = "`widen` and `truncate` keep the sign",
    note = "`checked_cast` and the other casts convert between the signs"
  )]
  pub trait SameSign<Target> {}

  impl SameSign<B0> for B0 {}

  impl SameSign<B1> for B1 {}

  /// Implemented for the pairs of a source and a target of `widen` whose kinds, `B1` for values
  /// that are not zero, allow it: any source into a type that holds zero, and only a source of
  /// values that are not zero into a type of such values.
  #[diagnostic::on_unimplemented(
    message = "`widen` into a non-zero integer needs a non-zero source",
    note = "`checked_cast` returns `None` for zero"
  )]
  pub trait KindWidens<Target> {}

  impl KindWidens<B0> for B0 {}

  impl KindWidens<B0> for B1 {}

  impl KindWidens<B1> for B1 {}
}

/// Conversions to `Self` from the integer `S` that check the range of `Self`, or assume it: the
/// checked, the strict and the unchecked cast of the native integers, still unstable
/// ([#157388](https://github.com/rust-lang/rust/issues/157388)).
///
/// The target implements this trait, as it implements `TryFrom`, so that a native integer converts
/// to a numo integer and back alike, and generic code bounded by `T: CheckedCastFrom<S>` takes
/// every pair:
///
/// | Target | Source | Checked at compile time |
/// |---|---|---|
/// | [`Uint`] and [`Int`] of every width and storage, and the native integers, `u8` to `u128`, `i8` to `i128`, `usize` and `isize` | the same, [`NonZeroUint`] and [`NonZeroInt`] of every width and storage, and the native `NonZero` integers | nothing |
/// | [`NonZeroUint`] and [`NonZeroInt`] of every width and storage, and the native `NonZero` integers | the same as above | nothing |
///
/// A value converts if it is in the range of `Self`: from zero to `2^Bits - 1` for [`Uint`], from
/// `-2^(Bits - 1)` to `2^(Bits - 1) - 1` for [`Int`], with zero bits holding only zero and one
/// signed bit -1 and 0, and the range of a native integer. A non-zero integer has the range of its
/// integer but takes no zero, for which the strict cast panics with the text of the error of the
/// kind `Zero` of the native conversions. A path through a native integer, such as
/// `u8::checked_cast_from(x)`, finds this trait while the trait of the native casts, still
/// unstable, is not in scope; once it is stable and both are in scope, the path names this trait,
/// as in `<u8 as CheckedCastFrom<_>>::checked_cast_from(x)`. A reference to a source is a source
/// too, as in `u8::checked_cast_from(&x)`, which leaves `x` to its owner.
///
/// ```
/// use numo::{
///   CheckedCastFrom, Int, NonZeroUint, i7,
///   typenum::{U7, U200},
///   u7,
/// };
///
/// // A native integer converts to a numo integer and back.
/// assert_eq!(u7::checked_cast_from(100u64), Some(u7::new(100)));
/// assert_eq!(u7::checked_cast_from(128u64), None);
/// assert_eq!(u8::checked_cast_from(i7::new(-1)), None);
/// assert_eq!(i64::checked_cast_from(i7::MIN), Some(-64));
/// assert_eq!(u8::strict_cast_from(u7::MAX), 127);
/// // SAFETY: 100 is in the range of `u7`, so `checked_cast_from` would return `Some`.
/// assert_eq!(unsafe { u7::unchecked_cast_from(100u64) }, u7::new(100));
///
/// // Numo integers convert between any widths and signs, also from a reference.
/// let wide = Int::<U200>::checked_cast_from(u7::MAX).unwrap();
/// assert_eq!(wide.to_string(), "127");
/// assert_eq!(u7::checked_cast_from(&wide), Some(u7::MAX));
/// assert_eq!(u7::checked_cast_from(wide), Some(u7::MAX));
/// assert_eq!(u7::checked_cast_from(-wide), None);
///
/// // A non-zero integer takes no zero.
/// assert_eq!(NonZeroUint::<U7>::checked_cast_from(0u8), None);
/// assert_eq!(u8::checked_cast_from(NonZeroUint::<U7>::MAX), Some(127));
///
/// // Generic code bounds the target.
/// fn narrow<T: CheckedCastFrom<u64>>(values: &[u64]) -> Option<Vec<T>> {
///   values.iter().map(|&value| T::checked_cast_from(value)).collect()
/// }
///
/// assert_eq!(narrow::<u7>(&[1, 127]), Some(vec![u7::new(1), u7::MAX]));
/// assert_eq!(narrow::<u7>(&[1, 128]), None);
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait CheckedCastFrom<S>: CastPair<S> + Sized {
  /// Converts `value` to `Self`, or returns `None` if it is outside the range of `Self`, or if it
  /// is zero and `Self` is a non-zero integer.
  #[must_use]
  fn checked_cast_from(value: S) -> Option<Self>;

  /// Converts `value` to `Self`.
  ///
  /// # Panics
  ///
  /// Panics in every build, reporting the caller, if `value` is outside the range of `Self`, with
  /// the message of the native `strict_cast`, and if it is zero and `Self` is a non-zero integer,
  /// with the text of the error of the kind `Zero` of the native conversions.
  #[must_use]
  #[track_caller]
  fn strict_cast_from(value: S) -> Self;

  /// Converts `value` to `Self`, assuming that it is in the range of `Self`, and not zero for a
  /// non-zero integer.
  ///
  /// # Safety
  ///
  /// `value` must be in the range of `Self`, from its `MIN` to its `MAX`, and not zero if `Self` is
  /// a non-zero integer, that is, [`checked_cast_from`](Self::checked_cast_from) would return
  /// `Some`. Otherwise the behavior is undefined. A non-zero `Self` checks for zero in every build
  /// and panics there, reporting the caller, as no non-zero integer of zero exists to return.
  #[must_use]
  #[track_caller]
  unsafe fn unchecked_cast_from(value: S) -> Self;
}

/// Conversions to `Self` from the integer `S` for every value: wrapping, as `as` casts the native
/// integers, or saturating, as the wrapping and the saturating cast of the native integers, still
/// unstable ([#157388](https://github.com/rust-lang/rust/issues/157388)).
///
/// The target implements this trait, as it implements [`CheckedCastFrom`], whose checks every
/// target of a wrapping cast also has:
///
/// | Target | Source | Checked at compile time |
/// |---|---|---|
/// | [`Uint`] and [`Int`] of every width and storage, and the native integers, `u8` to `u128`, `i8` to `i128`, `usize` and `isize` | the same, [`NonZeroUint`] and [`NonZeroInt`] of every width and storage, and the native `NonZero` integers | nothing |
///
/// The wrapping cast takes the value modulo `2^Bits` of `Self` and reads it in two's complement for
/// a signed `Self`, at every width: -1 of 7 bits wraps to 255 in a `u8`, and 300 to 44 in 7 bits.
/// The saturating cast takes the value, or the bound of the range of `Self` that it passes. Either
/// may give zero, so a non-zero integer is not a target.
///
/// ```
/// use core::num::NonZero;
///
/// use numo::{BoundedCastFrom, Int, i7, typenum::U12, u7};
///
/// // A wrapping cast keeps the low bits of the value, which may all be zero.
/// assert_eq!(u7::wrapping_cast_from(300u16), u7::new(44));
/// assert_eq!(u8::wrapping_cast_from(i7::new(-1)), 255);
/// assert_eq!(i7::wrapping_cast_from(64u8), i7::MIN);
/// assert_eq!(u8::wrapping_cast_from(NonZero::<u16>::new(256).unwrap()), 0);
///
/// // A saturating cast takes the bound of the range that the value passes.
/// assert_eq!(u7::saturating_cast_from(-5i32), u7::ZERO);
/// assert_eq!(u7::saturating_cast_from(300u16), u7::MAX);
/// assert_eq!(i8::saturating_cast_from(Int::<U12>::new(-2000)), i8::MIN);
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
#[diagnostic::on_unimplemented(
  message = "the target of a wrapping or saturating cast must take every value it wraps to",
  note = "a non-zero integer is a target of `checked_cast` and the other casts that check, not of the wrapping or saturating ones"
)]
pub trait BoundedCastFrom<S>: CheckedCastFrom<S> {
  /// Converts `value` modulo `2^Bits` of `Self`, read in two's complement for a signed `Self`, as
  /// `as` casts the native integers.
  #[must_use]
  fn wrapping_cast_from(value: S) -> Self;

  /// Converts `value`, or the bound of the range of `Self` that it passes: `MIN` below the range,
  /// and `MAX` above it.
  #[must_use]
  fn saturating_cast_from(value: S) -> Self;
}

/// The lossless conversion to `Self` from an integer `S` of the same sign that `Self` is at least
/// as wide as, which the compiler checks: the widening of the native integers, still unstable
/// ([#154330](https://github.com/rust-lang/rust/issues/154330)).
///
/// The target implements this trait for the pairs whose widths, signs and kinds the bounds accept:
///
/// | Target | Source | Checked at compile time |
/// |---|---|---|
/// | [`Uint`], `u8` to `u128` | [`Uint`], [`NonZeroUint`], `u8` to `u128` and their `NonZero` | the target is at least as wide as the source |
/// | [`Int`], `i8` to `i128` | [`Int`], [`NonZeroInt`], `i8` to `i128` and their `NonZero` | the target is at least as wide as the source |
/// | [`NonZeroUint`], `NonZero<u8>` to `NonZero<u128>` | [`NonZeroUint`], `NonZero<u8>` to `NonZero<u128>` | the target is at least as wide as the source |
/// | [`NonZeroInt`], `NonZero<i8>` to `NonZero<i128>` | [`NonZeroInt`], `NonZero<i8>` to `NonZero<i128>` | the target is at least as wide as the source |
/// | `usize` | [`Uint`], [`NonZeroUint`], and `u8` to `u128` and their `NonZero`, of at most 16 bits, `usize` and `NonZero<usize>` | the source has at most 16 bits |
/// | `isize` | [`Int`], [`NonZeroInt`], and `i8` to `i128` and their `NonZero`, of at most 16 bits, `isize` and `NonZero<isize>` | the source has at most 16 bits |
/// | `NonZero<usize>` | [`NonZeroUint`] and `NonZero<u8>` to `NonZero<u128>` of at most 16 bits, and `NonZero<usize>` | the source has at most 16 bits |
/// | `NonZero<isize>` | [`NonZeroInt`] and `NonZero<i8>` to `NonZero<i128>` of at most 16 bits, and `NonZero<isize>` | the source has at most 16 bits |
///
/// `usize` and `isize` have at least 16 bits, and no fixed upper bound: as targets they count as
/// 16 bits, and as sources they widen only into themselves, as for the native `widen`; their
/// `NonZero` do the same, and also widen into `usize` and `isize`. A non-zero integer takes only a
/// non-zero source, as the native `From` converts between the `NonZero` integers. A pair of another
/// width, sign or kind fails to compile; the casts of [`CheckedCastFrom`] convert it. A reference
/// to a source widens as the source does, through any number of references, but the identities of
/// `usize`, `isize` and their `NonZero` take a value or one reference.
///
/// ```
/// use core::num::NonZero;
///
/// use numo::{
///   Int, Uint, WidenFrom, i7,
///   typenum::{U16, U200},
///   u7,
/// };
///
/// assert_eq!(u8::widen_from(u7::MAX), 127);
/// assert_eq!(i16::widen_from(i7::MIN), -64);
/// assert_eq!(Int::<U200>::widen_from(-5i64).to_string(), "-5");
/// // `usize` holds at least 16 bits.
/// assert_eq!(usize::widen_from(Uint::<U16>::MAX), 65535);
/// // A non-zero integer widens into a non-zero integer, as the native `From` converts it, and
/// // into an integer.
/// let max = NonZero::<u8>::MAX;
/// assert_eq!(NonZero::<u16>::widen_from(max), NonZero::<u16>::from(max));
/// assert_eq!(u16::widen_from(max), 255);
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait WidenFrom<S>: CastPair<S> + Sized {
  /// Converts `value`, which `Self` holds, as it is at least as wide as `S` and of the same sign.
  #[must_use]
  fn widen_from(value: S) -> Self;
}

/// The conversions to `Self` from an integer `S` of the same sign that `Self` is at most as wide
/// as, which the compiler checks: wrapping, checked and saturating, as the truncation of the native
/// integers, still unstable ([#154330](https://github.com/rust-lang/rust/issues/154330)).
///
/// The target implements this trait for the pairs whose widths and signs the bounds accept:
///
/// | Target | Source | Checked at compile time |
/// |---|---|---|
/// | [`Uint`], `u8` to `u128` | [`Uint`], [`NonZeroUint`], `u8` to `u128` and `usize`, and their `NonZero` | the target is at most as wide as the source, `usize` counting as 16 bits |
/// | [`Int`], `i8` to `i128` | [`Int`], [`NonZeroInt`], `i8` to `i128` and `isize`, and their `NonZero` | the target is at most as wide as the source, `isize` counting as 16 bits |
/// | `usize` | `usize` and `NonZero<usize>` | nothing |
/// | `isize` | `isize` and `NonZero<isize>` | nothing |
///
/// `usize` and `isize` have at least 16 bits, and no fixed upper bound: as sources they count as 16
/// bits, and as targets they take only themselves, as for the native `truncate`, and their
/// `NonZero`. A non-zero integer is a source and not a target, as a truncated value may be zero.
/// The methods are those of [`BoundedCastFrom`] and [`CheckedCastFrom`] restricted to these pairs.
///
/// ```
/// use core::num::NonZero;
///
/// use numo::{Int, TruncateFrom, typenum::U12, u7};
///
/// assert_eq!(u7::truncate_from(300u16), u7::new(44));
/// assert_eq!(u7::checked_truncate_from(300u16), None);
/// assert_eq!(u7::saturating_truncate_from(300u16), u7::MAX);
/// assert_eq!(i8::truncate_from(Int::<U12>::new(-2000)), 48);
/// assert_eq!(u8::truncate_from(300usize), 44);
/// assert_eq!(u8::checked_truncate_from(NonZero::<u16>::MAX), None);
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait TruncateFrom<S>: BoundedCastFrom<S> {
  /// Converts `value` modulo `2^Bits` of `Self`, as
  /// [`wrapping_cast_from`](BoundedCastFrom::wrapping_cast_from) does.
  #[must_use]
  fn truncate_from(value: S) -> Self;

  /// Converts `value`, or returns `None` if it is outside the range of `Self`, as
  /// [`checked_cast_from`](CheckedCastFrom::checked_cast_from) does.
  #[must_use]
  fn checked_truncate_from(value: S) -> Option<Self>;

  /// Converts `value`, or the bound of the range of `Self` that it passes, as
  /// [`saturating_cast_from`](BoundedCastFrom::saturating_cast_from) does.
  #[must_use]
  fn saturating_truncate_from(value: S) -> Self;
}

/// The number of 64-bit words that hold `bits` bits.
#[inline]
const fn words(bits: u32) -> usize {
  bits.div_ceil(64) as usize
}

/// The word of every bit of `value` beyond its own words: copies of its sign, which only a negative
/// value sets.
#[inline]
fn fill<S: CastSource>(value: &S) -> u64 {
  if S::SIGNED {
    value.word(words(S::BITS))
  } else {
    0
  }
}

/// Returns `true` if the value of `source` is in the range of `T`: a negative value only for a
/// signed `T`, and every bit of the value from the sign bit of `T` up, or from the width of an
/// unsigned `T` up, a copy of its sign. Zero bits hold only zero. The bits beyond the words of the
/// source copy its sign, so only those words are read.
fn in_range<S: CastSource, T: CheckedTarget>(source: &S) -> bool {
  let fill = fill(source);
  // The lowest bit that must copy the sign of the value.
  let from = match (T::SIGNED, T::BITS) {
    (true, bits @ 1..) => bits - 1,
    // An unsigned target and zero bits hold no negative value.
    (_, bits) if fill == 0 => bits,
    _ => return false,
  };
  let first = (from / 64) as usize;
  (first..words(S::BITS)).all(|index| {
    // The bits of the first word below `from` take any value.
    let mask = if index == first {
      u64::MAX << (from % 64)
    } else {
      u64::MAX
    };
    (source.word(index) ^ fill) & mask == 0
  })
}

/// The value of `source` in `T`, if it is in the range of `T`.
#[inline]
fn checked<S: CastSource, T: CheckedTarget>(source: &S) -> Option<T> {
  if in_range::<S, T>(source) {
    T::from_words(|index| source.word(index))
  } else {
    None
  }
}

/// The value of `source` modulo `2^Bits` of `T`, read in two's complement for a signed `T`.
#[inline]
fn wrapping<S: CastSource, T: BoundedTarget>(source: &S) -> T {
  T::wrap_words(|index| source.word(index))
}

/// The value of `source` in `T`, or the bound of the range of `T` that it passes: the smallest
/// value, which only a negative value passes, or the largest.
#[inline]
fn saturating<S: CastSource, T: BoundedTarget>(source: &S) -> T {
  if in_range::<S, T>(source) {
    wrapping(source)
  } else {
    limit(fill(source) == 0)
  }
}

/// The value of `source` in `T`, or the error of the native conversions: for a value outside the
/// range of `T`, of the kind `NegOverflow` below it, which only a negative value is, and
/// `PosOverflow` above it, and for zero into a non-zero `T`, of the kind `Zero`.
pub(super) fn try_cast<S: CastSource, T: CheckedTarget>(source: S) -> Result<T, TryFromIntError> {
  if in_range::<S, T>(&source) {
    // In the range, only zero has no value, in a non-zero `T`.
    T::from_words(|index| source.word(index)).ok_or_else(convert::zero)
  } else if fill(&source) != 0 {
    Err(convert::neg_overflow())
  } else {
    Err(convert::pos_overflow())
  }
}

/// Panics on the strict cast of zero into a non-zero integer, with the text of the error of the
/// kind `Zero` of the native conversions, as no native strict cast has a non-zero target, reporting
/// the caller.
#[cold]
#[track_caller]
const fn strict_zero() -> ! {
  panic!("number would be zero for non-zero type")
}

/// The largest value of `T` if `max`, and its smallest otherwise, from the words of their bits,
/// without a constant: `2^Bits - 1` and zero for an unsigned `T`, and for a signed one the bits
/// below the sign bit and the bits from the sign bit up.
pub(super) fn limit<T: BoundedTarget>(max: bool) -> T {
  // The bits of the largest value, those below the sign bit or below the width.
  let magnitude = if T::SIGNED {
    T::BITS.saturating_sub(1)
  } else {
    T::BITS
  };
  T::wrap_words(|index| {
    let ones = ones_below(magnitude, index);
    match (max, T::SIGNED) {
      (true, _) => ones,
      (false, true) => !ones,
      (false, false) => 0,
    }
  })
}

/// The word at `index` of `2^bits - 1`, whose bits below `bits` are set.
#[inline]
fn ones_below(bits: u32, index: usize) -> u64 {
  // The index of the first bit of the word, which saturates far beyond every width.
  let low = (index as u64).saturating_mul(64);
  match u64::from(bits).saturating_sub(low) {
    64.. => u64::MAX,
    set => (1 << set) - 1,
  }
}

impl<S: CastSource, T: CheckedTarget> CastPair<S> for T {}

// A reference to a source reads the words and has the widths of the source, so that a cast function
// takes a borrowed value, as in `u64::checked_cast_from(&x)`, which neither takes nor clones `x`. A
// call of `word` on a reference resolves to the source's own method, which matches before the
// autoref of this one.
impl<S: CastSource> CastSource for &S {
  const BITS: u32 = S::BITS;
  const SIGNED: bool = S::SIGNED;

  #[inline]
  fn word(&self, index: usize) -> u64 {
    S::word(self, index)
  }
}

impl<S: MaxWidth> MaxWidth for &S {
  type Sign = S::Sign;
  type Bits = S::Bits;
}

impl<S: MinWidth> MinWidth for &S {
  type Sign = S::Sign;
  type Bits = S::Bits;
  type NonZero = S::NonZero;
}

impl<S: CastSource, T: CheckedTarget> CheckedCastFrom<S> for T {
  #[inline]
  fn checked_cast_from(value: S) -> Option<Self> {
    checked(&value)
  }

  #[inline]
  #[track_caller]
  fn strict_cast_from(value: S) -> Self {
    if in_range::<S, T>(&value) {
      match T::from_words(|index| value.word(index)) {
        Some(cast) => cast,
        // In the range, only zero has no value, in a non-zero `T`.
        None => strict_zero(),
      }
    } else {
      overflow::cast()
    }
  }

  // The body is safe: a debug build checks the caller's guarantee, and the value of the words is
  // the cast when the guarantee holds.
  #[inline]
  #[track_caller]
  unsafe fn unchecked_cast_from(value: S) -> Self {
    debug_assert!(
      in_range::<S, T>(&value),
      "attempt to cast integer with overflow"
    );
    T::assume_from_words(|index| value.word(index))
  }
}

impl<S: CastSource, T: BoundedTarget> BoundedCastFrom<S> for T {
  #[inline]
  fn wrapping_cast_from(value: S) -> Self {
    wrapping(&value)
  }

  #[inline]
  fn saturating_cast_from(value: S) -> Self {
    saturating(&value)
  }
}

// The widths and the signs are compared as types, and each failing comparison has a marker of its
// own, with its diagnostic.
impl<S, T> WidenFrom<S> for T
where
  S: CastSource + MaxWidth + MinWidth,
  T: CheckedTarget + MinWidth,
  <S as MaxWidth>::Sign: SameSign<<T as MinWidth>::Sign>,
  <S as MaxWidth>::Bits: IsLessOrEqual<<T as MinWidth>::Bits>,
  LeEq<<S as MaxWidth>::Bits, <T as MinWidth>::Bits>: WidenBits,
  <S as MinWidth>::NonZero: KindWidens<<T as MinWidth>::NonZero>,
{
  #[inline]
  fn widen_from(value: S) -> Self {
    // `T` holds every value of `S`, and a target of values that are not zero takes only such a
    // source, so the words always make a value.
    match T::from_words(|index| value.word(index)) {
      Some(widened) => widened,
      None => nonzero::zero(),
    }
  }
}

impl<S, T> TruncateFrom<S> for T
where
  S: CastSource + MinWidth,
  T: BoundedTarget + MaxWidth,
  <S as MinWidth>::Sign: SameSign<<T as MaxWidth>::Sign>,
  <T as MaxWidth>::Bits: IsLessOrEqual<<S as MinWidth>::Bits>,
  LeEq<<T as MaxWidth>::Bits, <S as MinWidth>::Bits>: TruncateBits,
{
  #[inline]
  fn truncate_from(value: S) -> Self {
    wrapping(&value)
  }

  #[inline]
  fn checked_truncate_from(value: S) -> Option<Self> {
    checked(&value)
  }

  #[inline]
  fn saturating_truncate_from(value: S) -> Self {
    saturating(&value)
  }
}

/// Implements the identities of `usize` and `isize`, whose width has no fixed upper bound, from a
/// value and from a reference to one: the native `widen` and `truncate` take them only from
/// themselves, which the bounds of the blanket implementations, over [`MaxWidth`], leave out.
macro_rules! pointer_sized_identities {
  ($($native:ident),+) => {$(
    impl WidenFrom<$native> for $native {
      #[inline]
      fn widen_from(value: $native) -> $native {
        value
      }
    }

    impl TruncateFrom<$native> for $native {
      #[inline]
      fn truncate_from(value: $native) -> $native {
        value
      }

      #[inline]
      fn checked_truncate_from(value: $native) -> Option<$native> {
        Some(value)
      }

      #[inline]
      fn saturating_truncate_from(value: $native) -> $native {
        value
      }
    }

    impl<'a> WidenFrom<&'a $native> for $native {
      #[inline]
      fn widen_from(value: &'a $native) -> $native {
        *value
      }
    }

    impl<'a> TruncateFrom<&'a $native> for $native {
      #[inline]
      fn truncate_from(value: &'a $native) -> $native {
        *value
      }

      #[inline]
      fn checked_truncate_from(value: &'a $native) -> Option<$native> {
        Some(*value)
      }

      #[inline]
      fn saturating_truncate_from(value: &'a $native) -> $native {
        *value
      }
    }
  )+};
}

pointer_sized_identities!(usize, isize);

/// Implements the sealed traits of the casts for [`Uint`] and [`Int`] over every storage of their
/// storage traits, written `Integer: Repr, Sign, signed;`, through the sealed `word` and
/// `from_words` of the storage.
macro_rules! integer_participants {
  ($($integer:ident: $repr:ident, $sign:ident, $signed:literal;)+) => {$(
    impl<B: FitsBits<R>, R: $repr> CastSource for $integer<B, R> {
      const BITS: u32 = B::U32;
      const SIGNED: bool = $signed;

      #[inline]
      fn word(&self, index: usize) -> u64 {
        self.repr.word(index, Token::new())
      }
    }

    impl<B: FitsBits<R>, R: $repr> CheckedTarget for $integer<B, R> {
      const BITS: u32 = B::U32;
      const SIGNED: bool = $signed;

      #[inline]
      fn from_words(word: impl Fn(usize) -> u64) -> Option<Self> {
        Some(Self::wrap_words(word))
      }

      #[inline]
      #[track_caller]
      fn assume_from_words(word: impl Fn(usize) -> u64) -> Self {
        Self::wrap_words(word)
      }
    }

    impl<B: FitsBits<R>, R: $repr> BoundedTarget for $integer<B, R> {
      #[inline]
      fn wrap_words(word: impl Fn(usize) -> u64) -> Self {
        Self::wrap(R::from_words(word, B::U32, Token::new()))
      }
    }

    // Facts of the type alone, which need no bound.
    impl<B, R> MaxWidth for $integer<B, R> {
      type Sign = $sign;
      type Bits = B;
    }

    impl<B, R> MinWidth for $integer<B, R> {
      type Sign = $sign;
      type Bits = B;
      type NonZero = B0;
    }
  )+};
}

integer_participants! {
  Uint: UintRepr, B0, false;
  Int: IntRepr, B1, true;
}

// The words of `usize` and `isize` pass through `u128` and `i128`, which hold them on every target,
// where they have 16, 32 or 64 bits.
const _: () = assert!(usize::BITS <= 128, "`usize` has more than 128 bits");

/// Implements the sealed traits of the casts for the native integers, written
/// `native: signed, layer;`, where `layer`, `u128` or `i128`, is the native layer of the widest
/// integer of the sign, through which every native integer of that sign reads its words, and the
/// widths of the native integers but `usize` and `isize`, written `widths native: Sign, Bits;`.
macro_rules! native_participants {
  ($($native:ident: $signed:literal, $layer:ident;)+) => {$(
    impl CastSource for $native {
      const BITS: u32 = $native::BITS;
      const SIGNED: bool = $signed;

      #[inline]
      fn word(&self, index: usize) -> u64 {
        int::$layer::word(*self as $layer, index)
      }
    }

    impl CheckedTarget for $native {
      const BITS: u32 = $native::BITS;
      const SIGNED: bool = $signed;

      #[inline]
      fn from_words(word: impl Fn(usize) -> u64) -> Option<Self> {
        Some(Self::wrap_words(word))
      }

      #[inline]
      #[track_caller]
      fn assume_from_words(word: impl Fn(usize) -> u64) -> Self {
        Self::wrap_words(word)
      }
    }

    impl BoundedTarget for $native {
      #[inline]
      fn wrap_words(word: impl Fn(usize) -> u64) -> Self {
        // `as` keeps the low bits, which read in two's complement for a signed integer.
        width::u128_of_words(word, $native::BITS) as $native
      }
    }
  )+};
  (widths $($native:ident: $sign:ident, $bits:ident;)+) => {$(
    impl MaxWidth for $native {
      type Sign = $sign;
      type Bits = $bits;
    }

    impl MinWidth for $native {
      type Sign = $sign;
      type Bits = $bits;
      type NonZero = B0;
    }
  )+};
}

native_participants! {
  u8: false, u128;
  u16: false, u128;
  u32: false, u128;
  u64: false, u128;
  u128: false, u128;
  usize: false, u128;
  i8: true, i128;
  i16: true, i128;
  i32: true, i128;
  i64: true, i128;
  i128: true, i128;
  isize: true, i128;
}

native_participants! {
  widths
  u8: B0, U8;
  u16: B0, U16;
  u32: B0, U32;
  u64: B0, U64;
  u128: B0, U128;
  i8: B1, U8;
  i16: B1, U16;
  i32: B1, U32;
  i64: B1, U64;
  i128: B1, U128;
}

// `usize` and `isize` have at least 16 bits, and no fixed upper bound.
impl MinWidth for usize {
  type Sign = B0;
  type Bits = U16;
  type NonZero = B0;
}

impl MinWidth for isize {
  type Sign = B1;
  type Bits = U16;
  type NonZero = B0;
}

/// Implements the sealed traits of the casts for [`NonZeroUint`] and [`NonZeroInt`] over every
/// storage of their storage traits, written `Name: Repr, Integer, Core, Sign, signed;`, through the
/// sealed methods of the storage of their integer, `Core`: a source reads the words of the integer
/// that `with_nonzero` lends, and a target computes the inline storage of the words with
/// `from_words`, and builds its storage from it unless it is zero.
macro_rules! nonzero_participants {
  ($($name:ident: $repr:ident, $integer:ident, $core:ident, $sign:ident, $signed:literal;)+) => {$(
    impl<B: FitsBits<R::Integer>, R: $repr> CastSource for $name<B, R> {
      const BITS: u32 = B::U32;
      const SIGNED: bool = $signed;

      #[inline]
      fn word(&self, index: usize) -> u64 {
        <R::Integer as $core>::with_nonzero(
          &self.value.repr,
          |integer| <R::Integer as $core>::word(integer, index, Token::new()),
          Token::new(),
        )
      }
    }

    impl<B: FitsBits<R::Integer>, R: $repr> CheckedTarget for $name<B, R> {
      const BITS: u32 = B::U32;
      const SIGNED: bool = $signed;

      #[inline]
      fn from_words(word: impl Fn(usize) -> u64) -> Option<Self> {
        let integer =
          <<R::Integer as $core>::Inline as $core>::from_words(word, B::U32, Token::new());
        if integer.is_zero(Token::new()) {
          return None;
        }
        let repr = <R::Integer as $core>::from_inline(integer, Token::new());
        Some(Self {
          value: $integer::wrap(repr.known_nonzero(Token::new())),
        })
      }

      #[inline]
      #[track_caller]
      fn assume_from_words(word: impl Fn(usize) -> u64) -> Self {
        match Self::from_words(word) {
          Some(value) => value,
          None => nonzero::zero_cast_value(),
        }
      }
    }

    // Facts of the type alone, which need no bound.
    impl<B, R> MaxWidth for $name<B, R> {
      type Sign = $sign;
      type Bits = B;
    }

    impl<B, R> MinWidth for $name<B, R> {
      type Sign = $sign;
      type Bits = B;
      type NonZero = B1;
    }
  )+};
}

nonzero_participants! {
  NonZeroUint: NonZeroUintRepr, Uint, UintCore, B0, false;
  NonZeroInt: NonZeroIntRepr, Int, IntCore, B1, true;
}

/// Implements the sealed traits of the casts for the native `NonZero` of each native integer,
/// written `native;`, through those of the native integer: a source reads its words, and a target
/// builds it from the words unless they make zero; and the widths of the native `NonZero`
/// integers but those of `usize` and `isize`, written `widths native: Sign, Bits;`.
macro_rules! native_nonzero_participants {
  ($($native:ident;)+) => {$(
    impl CastSource for core::num::NonZero<$native> {
      const BITS: u32 = $native::BITS;
      const SIGNED: bool = <$native as CastSource>::SIGNED;

      #[inline]
      fn word(&self, index: usize) -> u64 {
        <$native as CastSource>::word(&self.get(), index)
      }
    }

    impl CheckedTarget for core::num::NonZero<$native> {
      const BITS: u32 = $native::BITS;
      const SIGNED: bool = <$native as CheckedTarget>::SIGNED;

      #[inline]
      fn from_words(word: impl Fn(usize) -> u64) -> Option<Self> {
        Self::new(<$native as BoundedTarget>::wrap_words(word))
      }

      #[inline]
      #[track_caller]
      fn assume_from_words(word: impl Fn(usize) -> u64) -> Self {
        match Self::from_words(word) {
          Some(value) => value,
          None => nonzero::zero_cast_value(),
        }
      }
    }
  )+};
  (widths $($native:ident: $sign:ident, $bits:ident;)+) => {$(
    impl MaxWidth for core::num::NonZero<$native> {
      type Sign = $sign;
      type Bits = $bits;
    }

    impl MinWidth for core::num::NonZero<$native> {
      type Sign = $sign;
      type Bits = $bits;
      type NonZero = B1;
    }
  )+};
}

native_nonzero_participants! {
  u8;
  u16;
  u32;
  u64;
  u128;
  usize;
  i8;
  i16;
  i32;
  i64;
  i128;
  isize;
}

native_nonzero_participants! {
  widths
  u8: B0, U8;
  u16: B0, U16;
  u32: B0, U32;
  u64: B0, U64;
  u128: B0, U128;
  i8: B1, U8;
  i16: B1, U16;
  i32: B1, U32;
  i64: B1, U64;
  i128: B1, U128;
}

// The native `NonZero` of `usize` and `isize` have at least 16 bits and no fixed upper bound, as
// `usize` and `isize` do.
impl MinWidth for core::num::NonZero<usize> {
  type Sign = B0;
  type Bits = U16;
  type NonZero = B1;
}

impl MinWidth for core::num::NonZero<isize> {
  type Sign = B1;
  type Bits = U16;
  type NonZero = B1;
}

/// Implements the identities of the native `NonZero` of `usize` and `isize`, whose width has no
/// fixed upper bound, from a value and from a reference to one, as `pointer_sized_identities!` does
/// for `usize` and `isize`: `widen` into the `NonZero` itself and into its integer, which the
/// native `From` converts it into, and `truncate` into its integer. The bounds of the blanket
/// implementations, over [`MaxWidth`], leave them out.
macro_rules! pointer_sized_nonzero_identities {
  ($($native:ident),+) => {$(
    impl WidenFrom<core::num::NonZero<$native>> for core::num::NonZero<$native> {
      #[inline]
      fn widen_from(value: core::num::NonZero<$native>) -> core::num::NonZero<$native> {
        value
      }
    }

    impl WidenFrom<core::num::NonZero<$native>> for $native {
      #[inline]
      fn widen_from(value: core::num::NonZero<$native>) -> $native {
        value.get()
      }
    }

    impl TruncateFrom<core::num::NonZero<$native>> for $native {
      #[inline]
      fn truncate_from(value: core::num::NonZero<$native>) -> $native {
        value.get()
      }

      #[inline]
      fn checked_truncate_from(value: core::num::NonZero<$native>) -> Option<$native> {
        Some(value.get())
      }

      #[inline]
      fn saturating_truncate_from(value: core::num::NonZero<$native>) -> $native {
        value.get()
      }
    }

    impl<'a> WidenFrom<&'a core::num::NonZero<$native>> for core::num::NonZero<$native> {
      #[inline]
      fn widen_from(value: &'a core::num::NonZero<$native>) -> core::num::NonZero<$native> {
        *value
      }
    }

    impl<'a> WidenFrom<&'a core::num::NonZero<$native>> for $native {
      #[inline]
      fn widen_from(value: &'a core::num::NonZero<$native>) -> $native {
        value.get()
      }
    }

    impl<'a> TruncateFrom<&'a core::num::NonZero<$native>> for $native {
      #[inline]
      fn truncate_from(value: &'a core::num::NonZero<$native>) -> $native {
        value.get()
      }

      #[inline]
      fn checked_truncate_from(value: &'a core::num::NonZero<$native>) -> Option<$native> {
        Some(value.get())
      }

      #[inline]
      fn saturating_truncate_from(value: &'a core::num::NonZero<$native>) -> $native {
        value.get()
      }
    }
  )+};
}

pointer_sized_nonzero_identities!(usize, isize);

/// Implements the casts of [`Uint`] and [`Int`] over every storage of their storage traits, written
/// `Integer: Repr`, which call the traits of the target, so that a value names its target with
/// method syntax, as the native integers do.
macro_rules! cast_methods {
  ($($integer:ident: $repr:ident),+ $(,)?) => {$(
    impl<B: FitsBits<R>, R: $repr> $integer<B, R> {
      /// Checked cast: converts the value to the integer `T`, or returns `None` if it is outside
      /// the range of `T`, as the native `checked_cast` does, still unstable
      /// ([#157388](https://github.com/rust-lang/rust/issues/157388)), or if it is zero and `T` is
      /// a non-zero integer. `T` is [`Uint`], [`Int`], [`NonZeroUint`] or [`NonZeroInt`] of any
      /// width and storage, or a native integer or its `NonZero`, as [`CheckedCastFrom`] lists
      /// them.
      #[must_use]
      #[inline]
      pub fn checked_cast<T: CheckedCastFrom<Self>>(self) -> Option<T> {
        T::checked_cast_from(self)
      }

      /// Wrapping cast: converts the value to the integer `T` modulo `2^Bits` of `T`, read in
      /// two's complement for a signed `T`, as `as` casts the native integers and the native
      /// `wrapping_cast` does, still unstable
      /// ([#157388](https://github.com/rust-lang/rust/issues/157388)).
      #[must_use]
      #[inline]
      pub fn wrapping_cast<T: BoundedCastFrom<Self>>(self) -> T {
        T::wrapping_cast_from(self)
      }

      /// Saturating cast: converts the value to the integer `T`, or the bound of the range of `T`
      /// that it passes, `MIN` below the range and `MAX` above it, as the native
      /// `saturating_cast` does, still unstable
      /// ([#157388](https://github.com/rust-lang/rust/issues/157388)).
      #[must_use]
      #[inline]
      pub fn saturating_cast<T: BoundedCastFrom<Self>>(self) -> T {
        T::saturating_cast_from(self)
      }

      /// Strict cast: converts the value to the integer `T`, as the native `strict_cast` does,
      /// still unstable ([#157388](https://github.com/rust-lang/rust/issues/157388)).
      ///
      /// # Panics
      ///
      /// Panics in every build, reporting the caller, if the value is outside the range of `T`,
      /// with the message of the native `strict_cast`, and if it is zero and `T` is a non-zero
      /// integer, with the text of the error of the kind `Zero` of the native conversions.
      #[must_use]
      #[inline]
      #[track_caller]
      pub fn strict_cast<T: CheckedCastFrom<Self>>(self) -> T {
        T::strict_cast_from(self)
      }

      /// Unchecked cast: converts the value to the integer `T`, assuming that it is in the range
      /// of `T`, as the native `unchecked_cast` does, still unstable
      /// ([#157388](https://github.com/rust-lang/rust/issues/157388)), and not zero for a non-zero
      /// `T`.
      ///
      /// # Safety
      ///
      /// The value must be in the range of `T`, from its `MIN` to its `MAX`, and not zero if `T` is
      /// a non-zero integer, that is, [`checked_cast`](Self::checked_cast) would return `Some`.
      /// Otherwise the behavior is undefined. A non-zero `T` checks for zero in every build and
      /// panics there, reporting the caller, as no non-zero integer of zero exists to return.
      #[must_use]
      #[inline]
      #[track_caller]
      pub unsafe fn unchecked_cast<T: CheckedCastFrom<Self>>(self) -> T {
        // SAFETY: the caller guarantees that `checked_cast` would return `Some`, the precondition
        // of `unchecked_cast_from`.
        unsafe { T::unchecked_cast_from(self) }
      }

      /// Converts the value to the integer `T` of the same sign, which is at least as wide, as the
      /// native `widen` does, still unstable
      /// ([#154330](https://github.com/rust-lang/rust/issues/154330)). It never fails: a `T` of
      /// the other sign or of fewer bits fails to compile, and so does a non-zero `T`, which takes
      /// no zero, as [`WidenFrom`] lists the pairs.
      #[must_use]
      #[inline]
      pub fn widen<T: WidenFrom<Self>>(self) -> T {
        T::widen_from(self)
      }

      /// Converts the value to the integer `T` of the same sign, which is at most as wide, modulo
      /// `2^Bits` of `T`, as the native `truncate` does, still unstable
      /// ([#154330](https://github.com/rust-lang/rust/issues/154330)), and as
      /// [`wrapping_cast`](Self::wrapping_cast) does. A `T` of the other sign or of more bits
      /// fails to compile, as [`TruncateFrom`] lists the pairs.
      #[must_use]
      #[inline]
      pub fn truncate<T: TruncateFrom<Self>>(self) -> T {
        T::truncate_from(self)
      }

      /// Converts the value to the integer `T` of the same sign, which is at most as wide, or
      /// returns `None` if it is outside the range of `T`, as the native `checked_truncate` does,
      /// still unstable ([#154330](https://github.com/rust-lang/rust/issues/154330)).
      #[must_use]
      #[inline]
      pub fn checked_truncate<T: TruncateFrom<Self>>(self) -> Option<T> {
        T::checked_truncate_from(self)
      }

      /// Converts the value to the integer `T` of the same sign, which is at most as wide, or the
      /// bound of the range of `T` that it passes, as the native `saturating_truncate` does,
      /// still unstable ([#154330](https://github.com/rust-lang/rust/issues/154330)).
      #[must_use]
      #[inline]
      pub fn saturating_truncate<T: TruncateFrom<Self>>(self) -> T {
        T::saturating_truncate_from(self)
      }
    }
  )+};
}

cast_methods!(Uint: UintRepr, Int: IntRepr);

/// Implements the casts of [`NonZeroUint`] and [`NonZeroInt`] over every storage of their storage
/// traits, written `Name: Repr`, which call the traits of the target, as `cast_methods!` does for
/// the integers: the value converts as the integer of the same value does, and it is not zero, so
/// a non-zero target takes it wherever its range does, while the wrapping, saturating and
/// truncating casts, which may give zero, take a target that holds zero.
macro_rules! nonzero_cast_methods {
  ($($name:ident: $repr:ident),+ $(,)?) => {$(
    impl<B: FitsBits<R::Integer>, R: $repr> $name<B, R> {
      /// Checked cast: converts the value to the integer `T`, or returns `None` if it is outside
      /// the range of `T`, as `checked_cast` of the integer of the same value does. `T` is
      /// [`Uint`], [`Int`], [`NonZeroUint`] or [`NonZeroInt`] of any width and storage, or a
      /// native integer or its `NonZero`, as [`CheckedCastFrom`] lists them.
      #[must_use]
      #[inline]
      pub fn checked_cast<T: CheckedCastFrom<Self>>(self) -> Option<T> {
        T::checked_cast_from(self)
      }

      /// Wrapping cast: converts the value to the integer `T` modulo `2^Bits` of `T`, read in
      /// two's complement for a signed `T`, as `wrapping_cast` of the integer of the same value
      /// does. `T` holds zero, which a wrapped value may be: it is one of the integers that
      /// [`BoundedCastFrom`] lists, not a non-zero integer.
      #[must_use]
      #[inline]
      pub fn wrapping_cast<T: BoundedCastFrom<Self>>(self) -> T {
        T::wrapping_cast_from(self)
      }

      /// Saturating cast: converts the value to the integer `T`, or the bound of the range of `T`
      /// that it passes, `MIN` below the range and `MAX` above it, as `saturating_cast` of the
      /// integer of the same value does. `T` holds zero, the `MIN` of an unsigned integer: it is
      /// one of the integers that [`BoundedCastFrom`] lists, not a non-zero integer.
      #[must_use]
      #[inline]
      pub fn saturating_cast<T: BoundedCastFrom<Self>>(self) -> T {
        T::saturating_cast_from(self)
      }

      /// Strict cast: converts the value to the integer `T`, as `strict_cast` of the integer of the
      /// same value does.
      ///
      /// # Panics
      ///
      /// Panics if the value is outside the range of `T`, in every build, with the message of the
      /// native `strict_cast`, reporting the caller.
      #[must_use]
      #[inline]
      #[track_caller]
      pub fn strict_cast<T: CheckedCastFrom<Self>>(self) -> T {
        T::strict_cast_from(self)
      }

      /// Unchecked cast: converts the value to the integer `T`, assuming that it is in the range
      /// of `T`, as `unchecked_cast` of the integer of the same value does. The value is not zero,
      /// so a non-zero `T` takes it wherever its range does.
      ///
      /// # Safety
      ///
      /// The value must be in the range of `T`, from its `MIN` to its `MAX`, that is,
      /// [`checked_cast`](Self::checked_cast) would return `Some`. Otherwise the behavior is
      /// undefined.
      #[must_use]
      #[inline]
      #[track_caller]
      pub unsafe fn unchecked_cast<T: CheckedCastFrom<Self>>(self) -> T {
        // SAFETY: the caller guarantees that `checked_cast` would return `Some`, the precondition
        // of `unchecked_cast_from`.
        unsafe { T::unchecked_cast_from(self) }
      }

      /// Converts the value to the integer `T` of the same sign, which is at least as wide, as
      /// `widen` of the integer of the same value does, or to such a non-zero integer, as the
      /// native `From` converts between the `NonZero` integers. It never fails: a `T` of the other
      /// sign or of fewer bits fails to compile, as [`WidenFrom`] lists the pairs.
      #[must_use]
      #[inline]
      pub fn widen<T: WidenFrom<Self>>(self) -> T {
        T::widen_from(self)
      }

      /// Converts the value to the integer `T` of the same sign, which is at most as wide, modulo
      /// `2^Bits` of `T`, as `truncate` of the integer of the same value does. `T` holds zero,
      /// which a truncated value may be, and a `T` of the other sign or of more bits fails to
      /// compile, as [`TruncateFrom`] lists the pairs.
      #[must_use]
      #[inline]
      pub fn truncate<T: TruncateFrom<Self>>(self) -> T {
        T::truncate_from(self)
      }

      /// Converts the value to the integer `T` of the same sign, which is at most as wide, or
      /// returns `None` if it is outside the range of `T`, as `checked_truncate` of the integer of
      /// the same value does.
      #[must_use]
      #[inline]
      pub fn checked_truncate<T: TruncateFrom<Self>>(self) -> Option<T> {
        T::checked_truncate_from(self)
      }

      /// Converts the value to the integer `T` of the same sign, which is at most as wide, or the
      /// bound of the range of `T` that it passes, as `saturating_truncate` of the integer of the
      /// same value does.
      #[must_use]
      #[inline]
      pub fn saturating_truncate<T: TruncateFrom<Self>>(self) -> T {
        T::saturating_truncate_from(self)
      }
    }
  )+};
}

nonzero_cast_methods!(NonZeroUint: NonZeroUintRepr, NonZeroInt: NonZeroIntRepr);
