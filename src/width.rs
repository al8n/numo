//! The width relation between a type-level number of bits and an integer storage, and the storages
//! of [`Uint`](crate::Uint) and [`Int`](crate::Int).

use core::{cmp::Ordering, fmt, hash::Hasher, ops::Shl};

use typenum::{
  Const, IsEqual, IsLess, IsLessOrEqual, Le, LeEq, NonZero, Shleft, ToUInt, U, U3, U4, U5, U6, U7,
  U8, U16, U32, U64, U128, U4294967296, Unsigned,
};

use sealed::{
  ArrayLimb, BitsBelow2Pow32, BitsFit, NativeBitsFit, NativeReprFilled, ReprFilled,
  SealedFillsRepr, SealedFitsBits, SealedIntRepr, SealedUintRepr,
};

use crate::{
  int::{
    self, limbs, limbs8, limbs16, limbs32, limbs128, signed_limbs, signed_limbs8, signed_limbs16,
    signed_limbs32, signed_limbs128,
  },
  layout::Token,
};

mod sealed {
  use core::{cmp::Ordering, fmt, hash::Hasher};

  use crate::layout::Token;

  /// Seals [`FitsBits`](super::FitsBits).
  pub trait SealedFitsBits<Repr> {}

  /// Seals [`FillsRepr`](super::FillsRepr).
  pub trait SealedFillsRepr<Repr> {}

  /// Implemented only for `B1`, the result of `Bits <= B * N` when `[L; N]`, of limbs of `B` bits,
  /// holds `Bits` bits.
  // The message and label name no storage, so that every storage can share them.
  #[diagnostic::on_unimplemented(
    message = "the storage cannot hold `Bits` bits",
    label = "`Bits` is greater than the bits of the storage",
    note = "`[u8; N]`, `[u16; N]`, `[u32; N]`, `[u64; N]` and `[u128; N]` hold `8 * N`, `16 * N`, `32 * N`, `64 * N` and `128 * N` bits"
  )]
  pub trait BitsFit {}

  impl BitsFit for typenum::B1 {}

  /// Implemented only for `B1`, the result of `Bits < 2^32`, below which `typenum`'s `U32` is
  /// exact.
  #[diagnostic::on_unimplemented(
    message = "`Bits` must be below 2^32",
    label = "`Bits` is 2^32 or more",
    note = "the number of bits is a `u32`, as the constant `BITS` is"
  )]
  pub trait BitsBelow2Pow32 {}

  impl BitsBelow2Pow32 for typenum::B1 {}

  /// Implemented only for `B1`, the result of `Bits == B * N` when `Bits` fills `[L; N]`, of limbs
  /// of `B` bits.
  #[diagnostic::on_unimplemented(
    message = "`Bits` does not fill the storage",
    label = "`Bits` is less than the bits of the storage",
    note = "`[u8; N]`, `[u16; N]`, `[u32; N]`, `[u64; N]` and `[u128; N]` hold `8 * N`, `16 * N`, `32 * N`, `64 * N` and `128 * N` bits",
    note = "`checked_from_limbs` takes the limbs of any width that fits and checks the value"
  )]
  pub trait ReprFilled {}

  impl ReprFilled for typenum::B1 {}

  /// Implemented only for `B1`, the result of `Bits <= B` when a native integer of `B` bits holds
  /// `Bits` bits.
  #[diagnostic::on_unimplemented(
    message = "the storage cannot hold `Bits` bits",
    label = "`Bits` is greater than the bits of the storage",
    note = "`u8` and `i8` hold 8 bits, `u16` and `i16` 16, `u32` and `i32` 32, `u64` and `i64` 64, and `u128` and `i128` 128"
  )]
  pub trait NativeBitsFit {}

  impl NativeBitsFit for typenum::B1 {}

  /// Implemented only for `B1`, the result of `Bits == B` when `Bits` fills a native integer of `B`
  /// bits.
  #[diagnostic::on_unimplemented(
    message = "`Bits` does not fill the storage",
    label = "`Bits` is less than the bits of the storage",
    note = "`u8` and `i8` hold 8 bits, `u16` and `i16` 16, `u32` and `i32` 32, `u64` and `i64` 64, and `u128` and `i128` 128"
  )]
  pub trait NativeReprFilled {}

  impl NativeReprFilled for typenum::B1 {}

  /// A limb type of the array storages: `[L; N]` holds `N << L::BitsLog2` bits.
  pub trait ArrayLimb {
    /// The base-2 logarithm of the number of bits of a limb.
    type BitsLog2;
  }

  /// Seals [`UintRepr`](super::UintRepr) and gives the trait implementations of
  /// [`Uint`](crate::Uint) access to the integer core.
  ///
  /// A bound `R: UintRepr` makes these methods callable from any crate, so each takes a [`Token`].
  /// They rely on the width invariant of `Uint`: the representation, read as an unsigned integer,
  /// equals the value.
  pub trait SealedUintRepr: Sized {
    /// Zero.
    fn zero(token: Token) -> Self;

    /// Compares the values of `self` and `other`.
    fn compare(&self, other: &Self, token: Token) -> Ordering;

    /// Writes the lowest `ceil(bits / 64)` 64-bit words of the value to `state`, least significant
    /// first, extended by zeros beyond the storage. Requires the value to have `bits` bits.
    fn hash_limbs<H: Hasher>(&self, bits: u32, state: &mut H, token: Token);

    /// Writes the value in decimal, honoring the flags of `f` as integers do.
    fn fmt_decimal(&self, f: &mut fmt::Formatter<'_>, token: Token) -> fmt::Result;

    /// Returns `true` if the value is zero.
    fn is_zero(&self, token: Token) -> bool;

    /// Returns `true` if the value has `bits` bits: every bit at index `bits` or above is zero.
    /// Requires `bits` to be at least 1 and at most the bits of the storage.
    fn fits_bits(&self, bits: u32, token: Token) -> bool;
  }

  /// Seals [`IntRepr`](super::IntRepr) and gives the trait implementations of [`Int`](crate::Int)
  /// access to the integer core.
  ///
  /// As for [`SealedUintRepr`], each method takes a [`Token`]. They rely on the width invariant of
  /// `Int`: the representation, read as a two's complement integer, equals the value.
  pub trait SealedIntRepr: Sized {
    /// Zero.
    fn zero(token: Token) -> Self;

    /// Compares the values of `self` and `other`.
    fn compare(&self, other: &Self, token: Token) -> Ordering;

    /// Writes the lowest `ceil(bits / 64)` 64-bit words of the value in two's complement to
    /// `state`, least significant first, extended by copies of the sign beyond the storage.
    /// Requires the value to have `bits` bits.
    fn hash_limbs<H: Hasher>(&self, bits: u32, state: &mut H, token: Token);

    /// Writes the value in decimal, honoring the flags of `f` as integers do.
    fn fmt_decimal(&self, f: &mut fmt::Formatter<'_>, token: Token) -> fmt::Result;

    /// Returns `true` if the value is zero.
    fn is_zero(&self, token: Token) -> bool;

    /// Returns `true` if the value has `bits` bits, from `-2^(bits - 1)` to `2^(bits - 1) - 1`:
    /// every bit at index `bits - 1` or above equals the top bit of the storage. Requires `bits` to
    /// be at least 1 and at most the bits of the storage.
    fn fits_bits(&self, bits: u32, token: Token) -> bool;

    /// Returns `true` if the value is negative.
    fn is_negative(&self, token: Token) -> bool;

    /// Returns `true` if the value is positive.
    fn is_positive(&self, token: Token) -> bool;
  }
}

/// Marks that `Repr` holds every value of `Self` bits.
///
/// `Self` is a [`typenum`] number from 1 to `2^32 - 1`, so that the constant `BITS`, a `u32`, holds
/// it. The native integers `u8` and `i8` hold up to 8 bits, `u16` and `i16` up to 16, `u32` and
/// `i32` up to 32, `u64` and `i64` up to 64, and `u128` and `i128` up to 128. The arrays `[u8; N]`,
/// `[u16; N]`, `[u32; N]`, `[u64; N]` and `[u128; N]` hold up to `8 * N`, `16 * N`, `32 * N`,
/// `64 * N` and `128 * N` bits, for every `N` for which `typenum` implements `ToUInt` for
/// `Const<N>`: each `N` up to 1024, and larger ones such as the powers of 2 and of 10. An array of
/// any other length, such as `[u8; 1025]`, holds no width, so the integers stored in it have no
/// constants, constructors or `Default`, and no value of them can be made. A storage may have more
/// bits than needed, as `u16` has for 9 bits and `[u64; 4]` for 200. [`Uint`](crate::Uint) and
/// [`Int`](crate::Int) require this bound on their constants, constructors and `Default`, not on
/// the types themselves, so generic code adds it where it uses them:
///
/// ```
/// use numo::{FitsBits, Uint};
///
/// fn largest<B: FitsBits<[u64; 2]>>() -> Uint<B, [u64; 2]> {
///   Uint::<B, [u64; 2]>::MAX
/// }
///
/// assert_eq!(largest::<numo::typenum::U100>().to_string(), "1267650600228229401496703205375");
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
// The compiler shows this text to a generic caller whose bounds do not prove the relation; for a
// concrete width, it shows the text of the comparison instead.
#[diagnostic::on_unimplemented(
  message = "the storage cannot hold `Bits` bits",
  label = "`Bits` is not known to fit in the storage",
  note = "add a `FitsBits<Repr>` bound on `Bits`"
)]
pub trait FitsBits<Repr>: Unsigned + NonZero + SealedFitsBits<Repr> {}

impl<B, R> FitsBits<R> for B where B: Unsigned + NonZero + SealedFitsBits<R> {}

/// Marks that `Self` bits fill `Repr` exactly, so that every bit pattern of `Repr` is a value.
///
/// 8, 16, 32, 64 and 128 bits fill the native integers of that width, and `8 * N`, `16 * N`,
/// `32 * N`, `64 * N` and `128 * N` bits fill the arrays `[u8; N]`, `[u16; N]`, `[u32; N]`,
/// `[u64; N]` and `[u128; N]`. The constructor `from_limbs` of
/// [`Uint`](crate::Uint::from_limbs) and [`Int`](crate::Int::from_limbs) requires this bound and
/// never fails; `checked_from_limbs` takes the limbs of any width that fits and checks the value.
/// This trait is sealed and cannot be implemented outside this crate.
#[diagnostic::on_unimplemented(
  message = "`Bits` must fill the storage exactly",
  label = "`Bits` is not known to fill the storage",
  note = "add a `FillsRepr<Repr>` bound on `Bits`, or use `checked_from_limbs`, which checks the value"
)]
pub trait FillsRepr<Repr>: FitsBits<Repr> + SealedFillsRepr<Repr> {}

impl<B, R> FillsRepr<R> for B where B: FitsBits<R> + SealedFillsRepr<R> {}

/// `N * B` for limbs of `B` bits, the number of bits of `[L; N]`.
type LimbBits<L, const N: usize> = Shleft<U<N>, <L as ArrayLimb>::BitsLog2>;

// The bound below 2^32 is a comparison of types, because `typenum`'s `U32` and `U64` wrap: for
// `64 * 2^58` bits, both are zero.
impl<B, L: ArrayLimb, const N: usize> SealedFitsBits<[L; N]> for B
where
  B: IsLessOrEqual<LimbBits<L, N>> + IsLess<U4294967296>,
  Const<N>: ToUInt,
  U<N>: Shl<L::BitsLog2>,
  LeEq<B, LimbBits<L, N>>: BitsFit,
  Le<B, U4294967296>: BitsBelow2Pow32,
{
}

// `typenum`'s alias for `IsEqual::Output` is named `Eq`, as the trait of `core` is.
impl<B, L: ArrayLimb, const N: usize> SealedFillsRepr<[L; N]> for B
where
  B: IsEqual<LimbBits<L, N>>,
  Const<N>: ToUInt,
  U<N>: Shl<L::BitsLog2>,
  <B as IsEqual<LimbBits<L, N>>>::Output: ReprFilled,
{
}

/// A storage of [`Uint`](crate::Uint): a native unsigned integer, `u8`, `u16`, `u32`, `u64` or
/// `u128`, holds the value itself, and the array `[u8; N]`, `[u16; N]`, `[u32; N]`, `[u64; N]` or
/// `[u128; N]` holds it in `N` limbs of 8, 16, 32, 64 or 128 bits, least significant first.
///
/// `Uint<Bits, Repr>` implements `Ord`, `Hash`, `Display`, `Debug` and `Default` for every `Repr`
/// that implements this trait. This trait is sealed and cannot be implemented outside this crate.
#[diagnostic::on_unimplemented(
  message = "`{Self}` is not a storage of `Uint`",
  note = "`Uint` stores an unsigned native integer, `u8` to `u128`, or an array `[u8; N]` to `[u128; N]`"
)]
pub trait UintRepr: Eq + SealedUintRepr {}

/// A storage of [`Int`](crate::Int): a native signed integer, `i8`, `i16`, `i32`, `i64` or `i128`,
/// holds the value itself, and the array `[u8; N]`, `[u16; N]`, `[u32; N]`, `[u64; N]` or
/// `[u128; N]` holds it in `N` limbs of 8, 16, 32, 64 or 128 bits, least significant first, in
/// two's complement.
///
/// `Int<Bits, Repr>` implements `Ord`, `Hash`, `Display`, `Debug` and `Default` for every `Repr`
/// that implements this trait. This trait is sealed and cannot be implemented outside this crate.
#[diagnostic::on_unimplemented(
  message = "`{Self}` is not a storage of `Int`",
  note = "`Int` stores a signed native integer, `i8` to `i128`, or an array `[u8; N]` to `[u128; N]` of unsigned limbs in two's complement"
)]
pub trait IntRepr: Eq + SealedIntRepr {
  /// The storage of [`Uint`](crate::Uint) of the same size: `u8` for `i8`, `u16` for `i16`, and so
  /// on, and the same array for an array of limbs.
  type Unsigned: UintRepr;
}

/// Implements [`ArrayLimb`], [`UintRepr`] and [`IntRepr`] for the arrays of each limb type,
/// written `limb: BitsLog2, lower, upper`, where `lower` and `upper` are the modules of its limb
/// layers.
macro_rules! array_storage {
  ($($limb:ident: $bits_log2:ident, $lower:ident, $upper:ident);+ $(;)?) => {$(
    impl ArrayLimb for $limb {
      type BitsLog2 = $bits_log2;
    }

    impl<const N: usize> UintRepr for [$limb; N] {}

    impl<const N: usize> SealedUintRepr for [$limb; N] {
      #[inline]
      fn zero(_: Token) -> Self {
        [0; N]
      }

      #[inline]
      fn compare(&self, other: &Self, _: Token) -> Ordering {
        $lower::cmp(self, other)
      }

      #[inline]
      fn hash_limbs<H: Hasher>(&self, bits: u32, state: &mut H, _: Token) {
        hash_words(self, bits, 0, state);
      }

      #[inline]
      fn fmt_decimal(&self, f: &mut fmt::Formatter<'_>, _: Token) -> fmt::Result {
        fmt_magnitude::<_, N, { $lower::DIGITS_PER_LIMB }>(self, $lower::write_digits_into, true, f)
      }

      #[inline]
      fn is_zero(&self, _: Token) -> bool {
        $lower::is_zero(self)
      }

      #[inline]
      fn fits_bits(&self, bits: u32, _: Token) -> bool {
        $lower::fits_bits(self, bits)
      }
    }

    impl<const N: usize> IntRepr for [$limb; N] {
      type Unsigned = Self;
    }

    impl<const N: usize> SealedIntRepr for [$limb; N] {
      #[inline]
      fn zero(_: Token) -> Self {
        [0; N]
      }

      #[inline]
      fn compare(&self, other: &Self, _: Token) -> Ordering {
        $upper::cmp(self, other)
      }

      #[inline]
      fn hash_limbs<H: Hasher>(&self, bits: u32, state: &mut H, _: Token) {
        // The limbs above the sign bit hold its copies, so the lowest limbs hold the two's
        // complement of the value, and the limbs beyond a storage narrower than the words read as
        // further copies.
        let fill = if $upper::is_negative(self) { $limb::MAX } else { 0 };
        hash_words(self, bits, fill, state);
      }

      #[inline]
      fn fmt_decimal(&self, f: &mut fmt::Formatter<'_>, _: Token) -> fmt::Result {
        fmt_magnitude::<_, N, { $lower::DIGITS_PER_LIMB }>(
          &$upper::unsigned_abs(self),
          $lower::write_digits_into,
          !$upper::is_negative(self),
          f,
        )
      }

      #[inline]
      fn is_zero(&self, _: Token) -> bool {
        $upper::is_zero(self)
      }

      #[inline]
      fn fits_bits(&self, bits: u32, _: Token) -> bool {
        $upper::fits_bits(self, bits)
      }

      #[inline]
      fn is_negative(&self, _: Token) -> bool {
        $upper::is_negative(self)
      }

      #[inline]
      fn is_positive(&self, _: Token) -> bool {
        $upper::is_positive(self)
      }
    }
  )+};
}

array_storage!(
  u64: U6, limbs, signed_limbs;
  u8: U3, limbs8, signed_limbs8;
  u16: U4, limbs16, signed_limbs16;
  u32: U5, limbs32, signed_limbs32;
  u128: U7, limbs128, signed_limbs128;
);

/// Implements [`UintRepr`] and [`IntRepr`] for the native integers, and their width relations,
/// written `unsigned, signed: Bits`, where `Bits` is the number of bits of both.
// Each native type has width impls of its own, as each array type constructor has: one impl over
// every storage, bounded by a trait of the storages, would degrade the diagnostics of the arrays.
macro_rules! native_storage {
  ($($uint:ident, $int:ident: $bits:ident);+ $(;)?) => {$(
    native_storage!(@width $uint, $bits);
    native_storage!(@width $int, $bits);

    impl UintRepr for $uint {}

    impl SealedUintRepr for $uint {
      #[inline]
      fn zero(_: Token) -> Self {
        0
      }

      #[inline]
      fn compare(&self, other: &Self, _: Token) -> Ordering {
        self.cmp(other)
      }

      #[inline]
      fn hash_limbs<H: Hasher>(&self, bits: u32, state: &mut H, _: Token) {
        hash_words(&[*self], bits, 0, state);
      }

      #[inline]
      fn fmt_decimal(&self, f: &mut fmt::Formatter<'_>, _: Token) -> fmt::Result {
        fmt::Display::fmt(self, f)
      }

      #[inline]
      fn is_zero(&self, _: Token) -> bool {
        int::$uint::is_zero(*self)
      }

      #[inline]
      fn fits_bits(&self, bits: u32, _: Token) -> bool {
        int::$uint::fits_bits(*self, bits)
      }
    }

    impl IntRepr for $int {
      type Unsigned = $uint;
    }

    impl SealedIntRepr for $int {
      #[inline]
      fn zero(_: Token) -> Self {
        0
      }

      #[inline]
      fn compare(&self, other: &Self, _: Token) -> Ordering {
        self.cmp(other)
      }

      #[inline]
      fn hash_limbs<H: Hasher>(&self, bits: u32, state: &mut H, _: Token) {
        // The bits above the integer read as copies of its sign, as in an array storage.
        let fill = if int::$int::is_negative(*self) { $uint::MAX } else { 0 };
        hash_words(&[self.cast_unsigned()], bits, fill, state);
      }

      #[inline]
      fn fmt_decimal(&self, f: &mut fmt::Formatter<'_>, _: Token) -> fmt::Result {
        fmt::Display::fmt(self, f)
      }

      #[inline]
      fn is_zero(&self, _: Token) -> bool {
        int::$int::is_zero(*self)
      }

      #[inline]
      fn fits_bits(&self, bits: u32, _: Token) -> bool {
        int::$int::fits_bits(*self, bits)
      }

      #[inline]
      fn is_negative(&self, _: Token) -> bool {
        int::$int::is_negative(*self)
      }

      #[inline]
      fn is_positive(&self, _: Token) -> bool {
        int::$int::is_positive(*self)
      }
    }
  )+};
  (@width $native:ident, $bits:ident) => {
    impl<B> SealedFitsBits<$native> for B
    where
      B: IsLessOrEqual<$bits>,
      LeEq<B, $bits>: NativeBitsFit,
    {
    }

    impl<B> SealedFillsRepr<$native> for B
    where
      B: IsEqual<$bits>,
      <B as IsEqual<$bits>>::Output: NativeReprFilled,
    {
    }
  };
}

native_storage!(
  u8, i8: U8;
  u16, i16: U16;
  u32, i32: U32;
  u64, i64: U64;
  u128, i128: U128;
);

/// Writes the lowest `ceil(bits / 64)` 64-bit words of the integer in `limbs` to `state`, least
/// significant first, without the length prefix of a slice. For limbs of `B <= 64` bits, each word
/// joins `64 / B` limbs, the least significant first, and a limb at index `limbs.len()` or above
/// reads as `fill`. A limb of 128 bits holds two words, the low one first, and `N` such limbs hold
/// every word of `bits <= 128 * N` bits, so they never read `fill`.
///
/// Every storage of a value of `bits` bits writes the same words, so the hash does not depend on the
/// limb type or the number of limbs.
fn hash_words<L: Copy + Into<u128>, H: Hasher>(limbs: &[L], bits: u32, fill: L, state: &mut H) {
  let limb_bits = 8 * size_of::<L>() as u32;
  for word in 0..bits.div_ceil(64) as usize {
    // A 128-bit limb comes first, since `64 / 128` limbs per word would write zero words.
    let value = if limb_bits == 128 {
      limbs[word / 2].into() >> (64 * (word % 2))
    } else {
      let per_word = (u64::BITS / limb_bits) as usize;
      let mut value = 0;
      for k in 0..per_word {
        let limb = limbs.get(word * per_word + k).copied().unwrap_or(fill);
        value |= limb.into() << (limb_bits * k as u32);
      }
      value
    };
    // The low 64 bits: a word of narrower limbs has no others, and the high half of a 128-bit limb
    // is the next word.
    state.write_u64(value as u64);
  }
}

/// Writes `magnitude` in decimal with `write_digits_into`, negative unless `is_nonnegative`,
/// honoring the flags of `f` as integers do. Requires each limb to add at most `DIGITS` digits.
fn fmt_magnitude<L, const N: usize, const DIGITS: usize>(
  magnitude: &[L; N],
  write_digits_into: impl FnOnce(&[L; N], &mut [u8]) -> usize,
  is_nonnegative: bool,
  f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
  let mut buf = [[0; DIGITS]; N];
  let buf = buf.as_flattened_mut();
  let start = write_digits_into(magnitude, buf);
  f.pad_integral(is_nonnegative, "", ascii(&buf[start..])?)
}

/// Views digits produced by the integer core, which are always ASCII, as a string.
const fn ascii(digits: &[u8]) -> Result<&str, fmt::Error> {
  match core::str::from_utf8(digits) {
    Ok(text) => Ok(text),
    Err(_) => Err(fmt::Error),
  }
}
