//! The traits that [`Uint`] and [`Int`] implement for their storages.
//!
//! Each trait mirrors inherent items of the same names, which concrete types reach without the
//! trait, also in constant contexts. Generic code reaches them through the traits, which have one
//! implementation for every storage of a kind: the bound `B: BitWidth` gives `Uint<B>` and `Int<B>`
//! every trait that their default storages implement.

use core::{
  fmt::{Debug, Display},
  hash::Hash,
};

use sealed::Sealed;

use super::{Int, TryNewError, Uint};
use crate::{
  layout::Token,
  width::{FillsRepr, FitsBits, InlineIntRepr, InlineUintRepr, IntCore, IntRepr, UintCore},
};

mod sealed {
  /// Seals the integer traits: only [`Uint`](crate::Uint) and [`Int`](crate::Int) implement them.
  pub trait Sealed {}

  impl<B, R> Sealed for crate::Uint<B, R> {}

  impl<B, R> Sealed for crate::Int<B, R> {}
}

/// An integer of a fixed number of bits that holds its value inline: [`Uint`] and [`Int`] over a
/// native integer or an array of limbs.
///
/// The constants and methods are the inherent items of the same names, which a concrete type
/// reaches without this trait, also in constant contexts. Generic code reaches them through this
/// trait, which the bound `B: BitWidth` provides on `Uint<B>` and `Int<B>`; a method call needs the
/// trait in scope, while the constants are also inherent, as in `Uint::<B>::MAX`.
///
/// ```
/// use numo::{
///   BitWidth, FixedInteger, Int, Uint, u256,
///   typenum::{U7, U200},
/// };
///
/// fn is_max<T: FixedInteger>(value: T) -> bool {
///   value == T::MAX
/// }
///
/// fn zeros<B: BitWidth>(values: &[Uint<B>]) -> usize {
///   values.iter().filter(|value| value.is_zero()).count()
/// }
///
/// assert!(is_max(u256::MAX) && is_max(Int::<U7>::MAX) && !is_max(Uint::<U7>::ZERO));
/// assert_eq!(zeros(&[Uint::<U200>::ZERO, Uint::<U200>::MAX, Uint::<U200>::MIN]), 2);
/// assert_eq!(<Int<U7> as FixedInteger>::BITS, 7);
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait FixedInteger: Sealed + Copy + Eq + Ord + Hash + Debug + Display + Default {
  /// The number of bits, `Bits`.
  const BITS: u32;

  /// Zero.
  const ZERO: Self;

  /// The smallest value.
  const MIN: Self;

  /// The largest value.
  const MAX: Self;

  integer_query_table!(trait_items![]);
}

/// An unsigned integer of a fixed number of bits: [`Uint`].
///
/// ```
/// use numo::{
///   BitWidth, FixedInteger, Uint, UnsignedFixedInteger,
///   typenum::{U7, U200},
/// };
///
/// // The smallest unsigned value is zero.
/// fn is_min<T: FixedInteger + UnsignedFixedInteger>(value: T) -> bool {
///   value.is_zero()
/// }
///
/// fn smallest<B: BitWidth>() -> Uint<B> {
///   Uint::<B>::MIN
/// }
///
/// assert!(is_min(smallest::<U7>()) && is_min(smallest::<U200>()));
/// assert!(!is_min(numo::u256::MAX));
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait UnsignedFixedInteger: Sealed + Sized {}

/// A signed integer of a fixed number of bits, in two's complement: [`Int`].
///
/// ```
/// use numo::{
///   BitWidth, Int, SignedFixedInteger, Uint, i256,
///   typenum::U7,
/// };
///
/// fn negatives<T: SignedFixedInteger>(values: &[T]) -> usize {
///   values.iter().filter(|value| value.is_negative()).count()
/// }
///
/// // The unsigned integer of `Int<B>` is `Uint<B>`.
/// fn unsigned_max<B: BitWidth>(_: Int<B>) -> <Int<B> as SignedFixedInteger>::Unsigned {
///   Uint::<B>::MAX
/// }
///
/// assert_eq!(negatives(&[i256::MIN, i256::ZERO, i256::MAX]), 1);
/// assert_eq!(unsigned_max(Int::<U7>::MIN), Uint::<U7>::MAX);
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait SignedFixedInteger: Sealed + Sized {
  /// The unsigned integer of the same width in a storage of the same size: `Uint<B, u8>` for
  /// `Int<B, i8>`, and so on, and `Uint<B, [L; N]>` for `Int<B, [L; N]>`.
  type Unsigned;

  signed_query_table!(trait_items![]);
}

/// An integer stored in a native integer: [`Uint`] over `u8`, `u16`, `u32`, `u64` or `u128`, and
/// [`Int`] over `i8`, `i16`, `i32`, `i64` or `i128`.
///
/// ```
/// use numo::{Int, NativeInteger, Uint, typenum::U7};
///
/// fn doubled<T: NativeInteger<Native = u8>>(value: T) -> Option<T> {
///   T::try_new(value.value().checked_mul(2)?).ok()
/// }
///
/// assert_eq!(doubled(Uint::<U7>::new(50)), Some(Uint::<U7>::new(100)));
/// assert_eq!(doubled(Uint::<U7>::new(100)), None);
/// assert_eq!(<Int<U7> as NativeInteger>::new(-64).value(), -64);
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait NativeInteger: FixedInteger {
  /// The native integer that holds the value.
  type Native;

  /// Creates an integer of the value `value`.
  ///
  /// # Panics
  ///
  /// Panics if `value` does not fit in `Bits` bits.
  fn new(value: Self::Native) -> Self;

  /// Creates an integer of the value `value`.
  ///
  /// # Errors
  ///
  /// Returns [`TryNewError`] if `value` does not fit in `Bits` bits.
  fn try_new(value: Self::Native) -> Result<Self, TryNewError>;

  /// Returns the value.
  fn value(self) -> Self::Native;
}

/// An integer stored in an array of limbs, least significant first: [`Uint`] and [`Int`] over
/// `[u8; N]`, `[u16; N]`, `[u32; N]`, `[u64; N]` or `[u128; N]`.
///
/// ```
/// use numo::{
///   ArrayInteger, Int, i256,
///   typenum::{U100, U200},
/// };
///
/// fn low_limb<T: ArrayInteger<Array = [u64; N]>, const N: usize>(value: &T) -> u64 {
///   value.as_limbs()[0]
/// }
///
/// assert_eq!(low_limb(&i256::from_limbs([7, 0, 0, 0])), 7);
/// assert_eq!(low_limb(&Int::<U200>::MIN), 0);
/// // 2^104 needs 106 bits.
/// assert_eq!(<Int<U100, [u64; 2]> as ArrayInteger>::checked_from_limbs([0, 1 << 40]), None);
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait ArrayInteger: Sealed + Sized {
  /// The array of limbs, `Repr`.
  type Array;

  /// Creates an integer from its limbs, least significant first, or returns `None` if they do not
  /// hold a value of `Bits` bits.
  fn checked_from_limbs(limbs: Self::Array) -> Option<Self>;

  /// Returns the limbs of the value, least significant first.
  fn to_limbs(self) -> Self::Array;

  /// Borrows the limbs of the value, least significant first.
  fn as_limbs(&self) -> &Self::Array;
}

/// An integer stored in an array of limbs that its width fills, so that every bit pattern is a
/// value: [`Uint`] and [`Int`] whose `Bits` implements [`FillsRepr`] for the array.
///
/// ```
/// use numo::{FromLimbs, Uint, u256, typenum::U24};
///
/// fn from_bytes<T: FromLimbs<Array = [u8; 3]>>(bytes: [u8; 3]) -> T {
///   T::from_limbs(bytes)
/// }
///
/// let value: Uint<U24, [u8; 3]> = from_bytes([1, 2, 3]);
/// assert_eq!(value.to_string(), "197121");
/// assert_eq!(<u256 as FromLimbs>::from_limbs([u64::MAX; 4]), u256::MAX);
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait FromLimbs: ArrayInteger {
  /// Creates an integer from its limbs, least significant first. Every bit pattern is a value, so
  /// this never fails.
  fn from_limbs(limbs: Self::Array) -> Self;
}

// The constants are the inherent constants, which `Self::` names first.
impl<B: FitsBits<R>, R: InlineUintRepr> FixedInteger for Uint<B, R> {
  const BITS: u32 = Self::BITS;
  const ZERO: Self = Self::ZERO;
  const MIN: Self = Self::MIN;
  const MAX: Self = Self::MAX;

  integer_query_table!(generic_items![]);
}

impl<B: FitsBits<R>, R: InlineIntRepr> FixedInteger for Int<B, R> {
  const BITS: u32 = Self::BITS;
  const ZERO: Self = Self::ZERO;
  const MIN: Self = Self::MIN;
  const MAX: Self = Self::MAX;

  integer_query_table!(generic_items![]);
}

impl<B: FitsBits<R>, R: InlineUintRepr> UnsignedFixedInteger for Uint<B, R> {}

impl<B: FitsBits<R>, R: InlineIntRepr> SignedFixedInteger for Int<B, R> {
  type Unsigned = Uint<B, <R as IntRepr>::Unsigned>;

  signed_query_table!(generic_items![]);
}

impl<B, L, const N: usize> ArrayInteger for Uint<B, [L; N]>
where
  B: FitsBits<[L; N]>,
  [L; N]: InlineUintRepr,
{
  type Array = [L; N];

  #[inline]
  fn checked_from_limbs(limbs: [L; N]) -> Option<Self> {
    if <[L; N] as UintCore>::fits_bits(&limbs, B::U32, Token::new()) {
      Some(Self::wrap(limbs))
    } else {
      None
    }
  }

  #[inline]
  fn to_limbs(self) -> [L; N] {
    self.repr
  }

  #[inline]
  fn as_limbs(&self) -> &[L; N] {
    &self.repr
  }
}

impl<B, L, const N: usize> ArrayInteger for Int<B, [L; N]>
where
  B: FitsBits<[L; N]>,
  [L; N]: InlineIntRepr,
{
  type Array = [L; N];

  #[inline]
  fn checked_from_limbs(limbs: [L; N]) -> Option<Self> {
    if <[L; N] as IntCore>::fits_bits(&limbs, B::U32, Token::new()) {
      Some(Self::wrap(limbs))
    } else {
      None
    }
  }

  #[inline]
  fn to_limbs(self) -> [L; N] {
    self.repr
  }

  #[inline]
  fn as_limbs(&self) -> &[L; N] {
    &self.repr
  }
}

impl<B, L, const N: usize> FromLimbs for Uint<B, [L; N]>
where
  B: FillsRepr<[L; N]>,
  [L; N]: InlineUintRepr,
{
  #[inline]
  fn from_limbs(limbs: [L; N]) -> Self {
    Self::wrap(limbs)
  }
}

impl<B, L, const N: usize> FromLimbs for Int<B, [L; N]>
where
  B: FillsRepr<[L; N]>,
  [L; N]: InlineIntRepr,
{
  #[inline]
  fn from_limbs(limbs: [L; N]) -> Self {
    Self::wrap(limbs)
  }
}

#[cfg(test)]
mod tests;
