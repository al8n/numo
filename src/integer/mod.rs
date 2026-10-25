//! Integers whose width is a type-level number of bits.

use core::{
  cmp::Ordering,
  fmt,
  hash::{Hash, Hasher},
  marker::PhantomData,
};

pub use error::TryNewError;
pub use traits::{
  ArrayInteger, FixedInteger, FromLimbs, NativeInteger, SignedFixedInteger, UnsignedFixedInteger,
};

use crate::{
  int,
  layout::Token,
  width::{
    BitCount, FillsRepr, FitsBits, InlineIntRepr, InlineUintRepr, IntRepr, IntWidth, UintRepr,
    UintWidth,
  },
};

mod error;
#[macro_use]
mod tables;
mod traits;

/// An unsigned integer of `Bits` bits, stored in `Repr`.
///
/// The value runs from zero to `2^Bits - 1`. `Bits` is a [`typenum`] number of at least 1, and
/// `Repr` holds at least `Bits` bits: a native unsigned integer, `u8`, `u16`, `u32`, `u64` or
/// `u128`, which holds the value itself, or an array of limbs, least significant first, `[u8; N]`,
/// `[u16; N]`, `[u32; N]`, `[u64; N]` or `[u128; N]`, of `8 * N`, `16 * N`, `32 * N`, `64 * N` or
/// `128 * N` bits. The bits above `Bits`, if any, are zero. Without `Repr`, the storage is the
/// default that [`BitCount`] gives for 1 to 65536 bits: the smallest native integer that holds
/// `Bits` bits, and above 128 bits `[u64; ceil(Bits / 64)]`, so `Uint<U7>` is `Uint<U7, u8>` and
/// `Uint<U200>` is `Uint<U200, [u64; 4]>`. The module [`aliases`](crate::aliases) names common
/// widths in their default storages, such as [`u7`](crate::u7) for `Uint<U7, u8>` and
/// [`u256`](crate::u256) for `Uint<U256, [u64; 4]>`. `[u32; N]` computes with native 64-bit
/// intermediates, and `[u8; N]` has alignment 1 and holds the little-endian bytes of the value on
/// every target. `[u128; N]` has the layout of an array of `u128`, for memory shared with code that
/// stores 128-bit words. Its arithmetic works on the 64-bit halves of each limb, as no wider native
/// integer exists; its speed relative to `[u64; N]` depends on the operation, the width and the
/// target (formatting a value of a few limbs is several times slower), so choose it for layout, not
/// for speed. The type has the size, alignment and ABI of `Repr`. Each storage gives a distinct
/// type: `Uint<U8, u8>` and `Uint<U8, [u8; 1]>` have the same layout, order, text and hash, but are
/// not the same type.
///
/// In a native integer, [`new`](Self::new) creates a value, panicking if it does not fit in `Bits`
/// bits, [`try_new`](Self::try_new) returns an error instead, and [`value`](Self::value) returns
/// the value. In an array, [`from_limbs`](Self::from_limbs) creates a value from any bit pattern of
/// a width that fills its limbs, and [`checked_from_limbs`](Self::checked_from_limbs) from the
/// limbs of any width, checking that they hold a value of `Bits` bits. Values compare, hash and
/// format by their numeric value: `Ord` is numeric, `Hash` depends only on the value and `Bits`,
/// whatever the storage, and `Display` and `Debug` write decimal. There is no arithmetic.
///
/// The aliases from 192 to 512 bits are the wide storages of [`Decimal`](crate::Decimal):
///
/// ```
/// use numo::{
///   Decimal, Uint,
///   typenum::{U7, U18, U24, U59, U200},
///   u7, u256,
/// };
///
/// // 2^64 + 5, least significant limb first.
/// let value = u256::from_limbs([5, 1, 0, 0]);
/// assert_eq!(value.to_limbs(), [5, 1, 0, 0]);
/// assert_eq!(value.to_string(), "18446744073709551621");
/// assert!(u256::ZERO < value && value < u256::MAX);
/// assert_eq!(
///   u256::MAX.to_string(),
///   "115792089237316195423570985008687907853269984665640564039457584007913129639935"
/// );
///
/// // 200 bits in four limbs: the top 56 bits of the last limb are zero.
/// type U200x4 = Uint<U200, [u64; 4]>;
/// assert_eq!(U200x4::MAX.to_limbs(), [u64::MAX, u64::MAX, u64::MAX, 0xff]);
/// assert_eq!(U200x4::checked_from_limbs([5, 1, 0, 0]).unwrap().to_string(), value.to_string());
/// assert_eq!(U200x4::checked_from_limbs([0, 0, 0, 0x100]), None);
///
/// // 2^64 + 5 units of 10^-18.
/// let amount = Decimal::<U59, U18, u256>::from_repr(value).unwrap();
/// assert_eq!(amount.to_string(), "18.446744073709551621");
///
/// // 24 bits in three bytes, least significant first: 1 + 2 * 2^8 + 3 * 2^16.
/// let bytes = Uint::<U24, [u8; 3]>::from_limbs([1, 2, 3]);
/// assert_eq!(bytes.to_string(), "197121");
/// assert_eq!(Uint::<U24, [u8; 3]>::MAX.to_limbs(), [u8::MAX; 3]);
///
/// // 7 bits in the default storage, a `u8`, from zero to 127, which the alias `u7` names.
/// assert_eq!(u7::new(100).value(), 100u8);
/// assert_eq!(u7::MAX.value(), 127);
/// assert!(u7::try_new(128).is_err());
/// assert_eq!(Uint::<U7>::MAX, u7::MAX);
/// ```
///
/// The width cannot be inferred from the limbs, nor the storage from the expected type, so
/// construction names the alias, the width for the default storage, or the full type, for example
/// `u7::new(100)`, `u256::from_limbs(limbs)`, `Uint::<U7>::new(100)` or
/// `Uint::<U200, [u64; 4]>::checked_from_limbs(limbs)`.
#[repr(transparent)]
pub struct Uint<Bits, Repr = <Bits as BitCount>::Unsigned> {
  // Holds a value of `Bits` bits: every bit of the storage at index `Bits` or above is zero, so the
  // storage, read as an unsigned integer, equals the value. Every constructor establishes this, and
  // equality, comparison, hashing and formatting rely on it. It is a logical invariant, not a
  // safety invariant: no `unsafe` code may rely on it.
  repr: Repr,
  bits: PhantomData<Bits>,
}

/// A signed integer of `Bits` bits in two's complement, stored in `Repr`.
///
/// The value runs from `-2^(Bits - 1)` to `2^(Bits - 1) - 1`. `Bits` is a [`typenum`] number of at
/// least 1, and `Repr` holds at least `Bits` bits: a native signed integer, `i8`, `i16`, `i32`,
/// `i64` or `i128`, which holds the value itself, or an array of limbs, least significant first, as
/// for [`Uint`]: `[u8; N]`, `[u16; N]`, `[u32; N]`, `[u64; N]` or `[u128; N]`. The bits above
/// `Bits`, if any, are copies of the sign bit, so the storage holds the value in two's complement.
/// Without `Repr`, the storage is the default that [`BitCount`] gives, as for [`Uint`]: `Int<U24>`
/// is `Int<U24, i32>` and `Int<U200>` is `Int<U200, [u64; 4]>`. The module
/// [`aliases`](crate::aliases) names common widths, such as [`i7`](crate::i7) for `Int<U7, i8>`
/// and [`i256`](crate::i256) for `Int<U256, [u64; 4]>`. The type has the size, alignment and ABI of
/// `Repr`, and as for [`Uint`], each storage gives a distinct type.
///
/// As for [`Uint`], in a native integer [`new`](Self::new) and [`try_new`](Self::try_new) create a
/// value and [`value`](Self::value) returns it, and in an array [`from_limbs`](Self::from_limbs)
/// creates a value from any bit pattern of a width that fills its limbs, and
/// [`checked_from_limbs`](Self::checked_from_limbs) from the limbs of any width, checking that
/// they hold a value of `Bits` bits. Values compare, hash and format by their numeric value, and
/// there is no arithmetic.
///
/// ```
/// use numo::{
///   Int, i7, i256,
///   typenum::{U7, U24, U100},
/// };
///
/// let minus_one = i256::from_limbs([u64::MAX; 4]);
/// assert_eq!(minus_one.to_string(), "-1");
/// assert_eq!(format!("{minus_one:>4}"), "  -1");
/// assert!(minus_one.is_negative());
///
/// // Ordering is numeric: the most negative value has only the sign bit set.
/// assert_eq!(i256::MIN.to_limbs(), [0, 0, 0, 1 << 63]);
/// assert!(i256::MIN < minus_one && minus_one < i256::ZERO);
/// assert_eq!(
///   i256::MIN.to_string(),
///   "-57896044618658097711785492504343953926634992332820282019728792003956564819968"
/// );
///
/// // 7 bits in one limb: the value runs from -64 to 63, and -1 has every bit set.
/// type I7 = Int<U7, [u64; 1]>;
/// assert_eq!(I7::MIN.to_string(), "-64");
/// assert_eq!(I7::MAX.to_string(), "63");
/// assert_eq!(I7::checked_from_limbs([u64::MAX]).unwrap().to_string(), "-1");
/// // The limb 0x7f holds 127, which needs 8 bits.
/// assert_eq!(I7::checked_from_limbs([0x7f]), None);
///
/// // 100 bits in two limbs: the 28 bits above the sign bit copy it.
/// assert_eq!(Int::<U100, [u64; 2]>::MIN.to_limbs(), [0, 0xffff_fff8 << 32]);
///
/// // 24 bits in three bytes: -197121 in two's complement, least significant byte first.
/// let bytes = Int::<U24, [u8; 3]>::from_limbs([0xff, 0xfd, 0xfc]);
/// assert_eq!(bytes.to_string(), "-197121");
/// assert!(Int::<U24, [u8; 3]>::MIN < bytes);
///
/// // 7 bits in the default storage, an `i8`, from -64 to 63, which the alias `i7` names.
/// assert_eq!(i7::new(-5).value(), -5i8);
/// assert_eq!(i7::MIN.value(), -64);
/// assert!(i7::try_new(64).is_err());
/// assert_eq!(Int::<U7>::MIN, i7::MIN);
/// ```
///
/// The width cannot be inferred from the limbs, nor the storage from the expected type, so
/// construction names the alias, the width for the default storage, or the full type, for example
/// `i7::new(-5)`, `i256::from_limbs(limbs)`, `Int::<U7>::new(-5)` or
/// `Int::<U200, [u64; 4]>::checked_from_limbs(limbs)`.
#[repr(transparent)]
pub struct Int<Bits, Repr = <Bits as BitCount>::Signed> {
  // Holds a value of `Bits` bits: every bit of the storage at index `Bits - 1` or above equals bit
  // `Bits - 1`, the sign bit, so the storage, read as a two's complement integer, equals the value.
  // Every constructor establishes this, and equality, comparison, the sign queries, hashing and
  // formatting rely on it. It is a logical invariant, not a safety invariant: no `unsafe` code may
  // rely on it.
  repr: Repr,
  bits: PhantomData<Bits>,
}

impl<B, R> Uint<B, R> {
  /// Wraps `repr`. Requires it to hold a value of `Bits` bits.
  #[inline]
  const fn wrap(repr: R) -> Self {
    Self {
      repr,
      bits: PhantomData,
    }
  }
}

impl<B, R> Int<B, R> {
  /// Wraps `repr`. Requires it to hold a value of `Bits` bits.
  #[inline]
  const fn wrap(repr: R) -> Self {
    Self {
      repr,
      bits: PhantomData,
    }
  }
}

// The constants are generic over the storage. `BITS` needs only the width relation, so every
// storage has it; the values need a storage that holds them inline, and come from the storage's
// own constants for each width, which its sealed trait carries.
impl<B: FitsBits<R>, R: UintRepr> Uint<B, R> {
  /// The number of bits, `Bits`.
  pub const BITS: u32 = B::U32;
}

impl<B: FitsBits<R>, R: InlineUintRepr> Uint<B, R> {
  /// Zero.
  pub const ZERO: Self = Self::wrap(R::ZERO);

  /// The smallest value, zero.
  pub const MIN: Self = Self::ZERO;

  /// The largest value, `2^Bits - 1`.
  pub const MAX: Self = Self::wrap(<R::Width<B> as UintWidth<R>>::MAX);
}

impl<B: FitsBits<R>, R: IntRepr> Int<B, R> {
  /// The number of bits, `Bits`, including the sign bit.
  pub const BITS: u32 = B::U32;
}

impl<B: FitsBits<R>, R: InlineIntRepr> Int<B, R> {
  /// Zero.
  pub const ZERO: Self = Self::wrap(R::ZERO);

  /// The smallest value, `-2^(Bits - 1)`.
  pub const MIN: Self = Self::wrap(<R::Width<B> as IntWidth<R>>::MIN);

  /// The largest value, `2^(Bits - 1) - 1`.
  pub const MAX: Self = Self::wrap(<R::Width<B> as IntWidth<R>>::MAX);
}

/// Generates the `const fn`s of [`Uint`] and [`Int`] over the arrays of each limb type, written
/// `limb, bits, lower, upper;`, where `bits` is the number of bits of a limb and `lower` and
/// `upper` are the modules of its limb layers, and followed by the examples of
/// `checked_from_limbs` for `Uint` and for `Int`.
// Trait methods cannot be `const`, so each limb type has inherent impls of its own. Each limb type
// also has an invocation of its own, which a diagnostic that points into the macro shows.
macro_rules! array_integers {
  ($(
    $limb:ident, $bits:literal, $lower:ident, $upper:ident;
    Uint: { $(#[$uint_example:meta])* }
    Int: { $(#[$int_example:meta])* }
  )+) => {$(
    impl<B, const N: usize> Uint<B, [$limb; N]>
    where
      B: FitsBits<[$limb; N]>,
    {
      #[doc = concat!(
        "Creates an integer from its ",
        stringify!($bits),
        "-bit limbs, least significant first."
      )]
      ///
      /// `Bits` fills the limbs, so every bit pattern is a value and this never fails. For a width that
      /// leaves bits unused, [`checked_from_limbs`](Self::checked_from_limbs) checks the limbs instead.
      #[must_use]
      #[inline]
      pub const fn from_limbs(limbs: [$limb; N]) -> Self
      where
        B: FillsRepr<[$limb; N]>,
      {
        Self::wrap(limbs)
      }

      // Each line of this documentation starts with a space, as each line of the examples does, so
      // that rustdoc removes one space from every line.
      #[doc = concat!(
        " Creates an integer from its ",
        stringify!($bits),
        "-bit limbs, least significant first, or returns `None` if they"
      )]
      #[doc = " hold `2^Bits` or more: a bit at index `Bits` or above is set."]
      #[doc = ""]
      #[doc = " For a width that fills its limbs, this never returns `None`."]
      #[doc = ""]
      $(#[$uint_example])*
      #[must_use]
      #[inline]
      pub const fn checked_from_limbs(limbs: [$limb; N]) -> Option<Self> {
        if int::unsigned::$lower::fits_bits(&limbs, Self::BITS) {
          Some(Self::wrap(limbs))
        } else {
          None
        }
      }
    }

    impl<B, const N: usize> Uint<B, [$limb; N]> {
      #[doc = concat!(
        "Returns the ",
        stringify!($bits),
        "-bit limbs of the value, least significant first. The bits above `Bits`, if any,"
      )]
      /// are zero.
      #[must_use]
      #[inline]
      pub const fn to_limbs(self) -> [$limb; N] {
        self.repr
      }

      #[doc = concat!(
        "Borrows the ",
        stringify!($bits),
        "-bit limbs of the value, least significant first. The bits above `Bits`, if any,"
      )]
      /// are zero.
      #[must_use]
      #[inline]
      pub const fn as_limbs(&self) -> &[$limb; N] {
        &self.repr
      }

      integer_query_table!(inherent_items![unsigned, $lower, &]);
    }

    impl<B, const N: usize> Int<B, [$limb; N]>
    where
      B: FitsBits<[$limb; N]>,
    {
      #[doc = concat!(
        "Creates an integer from its ",
        stringify!($bits),
        "-bit limbs, least significant first, in two's complement."
      )]
      ///
      /// `Bits` fills the limbs, so every bit pattern is a value and this never fails. For a width that
      /// leaves bits unused, [`checked_from_limbs`](Self::checked_from_limbs) checks the limbs instead.
      #[must_use]
      #[inline]
      pub const fn from_limbs(limbs: [$limb; N]) -> Self
      where
        B: FillsRepr<[$limb; N]>,
      {
        Self::wrap(limbs)
      }

      // Each line of this documentation starts with a space, as each line of the examples does, so
      // that rustdoc removes one space from every line.
      #[doc = concat!(
        " Creates an integer from its ",
        stringify!($bits),
        "-bit limbs, least significant first, in two's complement, or"
      )]
      #[doc = concat!(
        " returns `None` if the integer they hold is below `-2^(Bits - 1)` or above ",
        "`2^(Bits - 1) - 1`:"
      )]
      #[doc = " a bit at index `Bits - 1` or above differs from the top bit of the limbs."]
      #[doc = ""]
      #[doc = " For a width that fills its limbs, this never returns `None`."]
      #[doc = ""]
      $(#[$int_example])*
      #[must_use]
      #[inline]
      pub const fn checked_from_limbs(limbs: [$limb; N]) -> Option<Self> {
        if int::signed::$upper::fits_bits(&limbs, Self::BITS) {
          Some(Self::wrap(limbs))
        } else {
          None
        }
      }
    }

    impl<B, const N: usize> Int<B, [$limb; N]> {
      #[doc = concat!(
        "Returns the ",
        stringify!($bits),
        "-bit limbs of the value, least significant first, in two's complement. The bits"
      )]
      /// above `Bits`, if any, are copies of the sign bit.
      #[must_use]
      #[inline]
      pub const fn to_limbs(self) -> [$limb; N] {
        self.repr
      }

      #[doc = concat!(
        "Borrows the ",
        stringify!($bits),
        "-bit limbs of the value, least significant first, in two's complement. The bits"
      )]
      /// above `Bits`, if any, are copies of the sign bit.
      #[must_use]
      #[inline]
      pub const fn as_limbs(&self) -> &[$limb; N] {
        &self.repr
      }

      integer_query_table!(inherent_items![signed, $upper, &]);
      signed_query_table!(inherent_items![signed, $upper, &]);
    }
  )+};
}

array_integers! {
  u64, 64, b64, b64;
  Uint: {
    /// ```
    /// use numo::{Uint, typenum::U65};
    ///
    /// type U65x2 = Uint<U65, [u64; 2]>;
    ///
    /// // 2^64 + 1 and 2^65 - 1 have 65 bits, and 2^65 has 66.
    /// assert_eq!(
    ///   U65x2::checked_from_limbs([1, 1]).unwrap().to_string(),
    ///   "18446744073709551617"
    /// );
    /// assert_eq!(U65x2::checked_from_limbs([u64::MAX, 1]), Some(U65x2::MAX));
    /// assert_eq!(U65x2::checked_from_limbs([0, 2]), None);
    /// ```
  }
  Int: {
    /// ```
    /// use numo::{Int, typenum::U7};
    ///
    /// type I7 = Int<U7, [u64; 1]>;
    ///
    /// // -1, every bit set, and 63, the largest value.
    /// assert!(I7::checked_from_limbs([u64::MAX]).unwrap().is_negative());
    /// assert_eq!(I7::checked_from_limbs([63]), Some(I7::MAX));
    /// // 127 and -65 need 8 bits.
    /// assert_eq!(I7::checked_from_limbs([127]), None);
    /// assert_eq!(I7::checked_from_limbs([-65i64 as u64]), None);
    /// ```
  }
}

array_integers! {
  u8, 8, b8, b8;
  Uint: {
    /// ```
    /// use numo::{Uint, typenum::U12};
    ///
    /// type U12x2 = Uint<U12, [u8; 2]>;
    ///
    /// // 0x234 and 0xfff, the largest value, have at most 12 bits, and 0x1000 has 13.
    /// assert_eq!(U12x2::checked_from_limbs([0x34, 0x02]).unwrap().to_string(), "564");
    /// assert_eq!(U12x2::checked_from_limbs([0xff, 0x0f]), Some(U12x2::MAX));
    /// assert_eq!(U12x2::checked_from_limbs([0x00, 0x10]), None);
    /// ```
  }
  Int: {
    /// ```
    /// use numo::{Int, typenum::U12};
    ///
    /// type I12x2 = Int<U12, [u8; 2]>;
    ///
    /// // -1 has every bit set, and -2048, the smallest value, every bit from the sign bit up.
    /// assert_eq!(I12x2::checked_from_limbs([0xff, 0xff]).unwrap().to_string(), "-1");
    /// assert_eq!(I12x2::checked_from_limbs([0x00, 0xf8]), Some(I12x2::MIN));
    /// // 2048 and -2049 need 13 bits.
    /// assert_eq!(I12x2::checked_from_limbs([0x00, 0x08]), None);
    /// assert_eq!(I12x2::checked_from_limbs([0xff, 0xf7]), None);
    /// ```
  }
}

array_integers! {
  u16, 16, b16, b16;
  Uint: {
    /// ```
    /// use numo::{Uint, typenum::U20};
    ///
    /// type U20x2 = Uint<U20, [u16; 2]>;
    ///
    /// // 2^16 + 1 and 2^20 - 1 have at most 20 bits, and 2^20 has 21.
    /// assert_eq!(U20x2::checked_from_limbs([1, 1]).unwrap().to_string(), "65537");
    /// assert_eq!(U20x2::checked_from_limbs([u16::MAX, 0xf]), Some(U20x2::MAX));
    /// assert_eq!(U20x2::checked_from_limbs([0, 0x10]), None);
    /// ```
  }
  Int: {
    /// ```
    /// use numo::{Int, typenum::U20};
    ///
    /// type I20x2 = Int<U20, [u16; 2]>;
    ///
    /// // -1, every bit set, and 2^19 - 1, the largest value.
    /// assert!(I20x2::checked_from_limbs([u16::MAX; 2]).unwrap().is_negative());
    /// assert_eq!(I20x2::checked_from_limbs([u16::MAX, 0x7]), Some(I20x2::MAX));
    /// // 2^19 needs 21 bits.
    /// assert_eq!(I20x2::checked_from_limbs([0, 0x8]), None);
    /// ```
  }
}

array_integers! {
  u32, 32, b32, b32;
  Uint: {
    /// ```
    /// use numo::{Uint, typenum::U33};
    ///
    /// type U33x2 = Uint<U33, [u32; 2]>;
    ///
    /// // 2^32 + 1 and 2^33 - 1 have 33 bits, and 2^33 has 34.
    /// assert_eq!(U33x2::checked_from_limbs([1, 1]).unwrap().to_string(), "4294967297");
    /// assert_eq!(U33x2::checked_from_limbs([u32::MAX, 1]), Some(U33x2::MAX));
    /// assert_eq!(U33x2::checked_from_limbs([0, 2]), None);
    /// ```
  }
  Int: {
    /// ```
    /// use numo::{Int, typenum::U40};
    ///
    /// type I40x2 = Int<U40, [u32; 2]>;
    ///
    /// // -2^39, the smallest value, has every bit from the sign bit up set.
    /// assert_eq!(I40x2::checked_from_limbs([0, 0xffff_ff80]), Some(I40x2::MIN));
    /// assert_eq!(I40x2::MIN.to_string(), "-549755813888");
    /// // 2^39 needs 41 bits.
    /// assert_eq!(I40x2::checked_from_limbs([0, 0x80]), None);
    /// ```
  }
}

array_integers! {
  u128, 128, b128, b128;
  Uint: {
    /// ```
    /// use numo::{Uint, typenum::U129};
    ///
    /// type U129x2 = Uint<U129, [u128; 2]>;
    ///
    /// // 2^128 + 1 and 2^129 - 1 have 129 bits, and 2^129 has 130.
    /// assert_eq!(
    ///   U129x2::checked_from_limbs([1, 1]).unwrap().to_string(),
    ///   "340282366920938463463374607431768211457"
    /// );
    /// assert_eq!(U129x2::checked_from_limbs([u128::MAX, 1]), Some(U129x2::MAX));
    /// assert_eq!(U129x2::checked_from_limbs([0, 2]), None);
    /// ```
  }
  Int: {
    /// ```
    /// use numo::{Int, typenum::U7};
    ///
    /// type I7 = Int<U7, [u128; 1]>;
    ///
    /// // -1, every bit set, and 63, the largest value.
    /// assert!(I7::checked_from_limbs([u128::MAX]).unwrap().is_negative());
    /// assert_eq!(I7::checked_from_limbs([63]), Some(I7::MAX));
    /// // 127 and -65 need 8 bits.
    /// assert_eq!(I7::checked_from_limbs([127]), None);
    /// assert_eq!(I7::checked_from_limbs([-65i128 as u128]), None);
    /// ```
  }
}

/// Generates the `const fn`s of [`Uint`] and [`Int`] over the native integers, and their
/// [`NativeInteger`] implementations, written `unsigned, signed;`.
macro_rules! native_integers {
  ($($uint:ident, $int:ident);+ $(;)?) => {$(
    impl<B> Uint<B, $uint>
    where
      B: FitsBits<$uint>,
    {
      /// Creates an integer of the value `value`.
      ///
      /// # Panics
      ///
      /// Panics if `value` is `2^Bits` or more, so in a constant such a value fails to compile.
      /// [`try_new`](Self::try_new) returns an error instead.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const fn new(value: $uint) -> Self {
        match Self::try_new(value) {
          Ok(integer) => integer,
          Err(_) => panic!("the value does not fit in `Bits` bits"),
        }
      }

      /// Creates an integer of the value `value`.
      ///
      /// # Errors
      ///
      /// Returns [`TryNewError`] if `value` is `2^Bits` or more.
      #[inline]
      pub const fn try_new(value: $uint) -> Result<Self, TryNewError> {
        if int::$uint::fits_bits(value, Self::BITS) {
          Ok(Self::wrap(value))
        } else {
          Err(TryNewError)
        }
      }
    }

    impl<B> Uint<B, $uint> {
      /// Returns the value.
      #[must_use]
      #[inline]
      pub const fn value(self) -> $uint {
        self.repr
      }

      integer_query_table!(inherent_items![$uint]);
    }

    impl<B> Int<B, $int>
    where
      B: FitsBits<$int>,
    {
      /// Creates an integer of the value `value`.
      ///
      /// # Panics
      ///
      /// Panics if `value` is below `-2^(Bits - 1)` or above `2^(Bits - 1) - 1`, so in a constant
      /// such a value fails to compile. [`try_new`](Self::try_new) returns an error instead.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const fn new(value: $int) -> Self {
        match Self::try_new(value) {
          Ok(integer) => integer,
          Err(_) => panic!("the value does not fit in `Bits` bits"),
        }
      }

      /// Creates an integer of the value `value`.
      ///
      /// # Errors
      ///
      /// Returns [`TryNewError`] if `value` is below `-2^(Bits - 1)` or above `2^(Bits - 1) - 1`.
      #[inline]
      pub const fn try_new(value: $int) -> Result<Self, TryNewError> {
        if int::$int::fits_bits(value, Self::BITS) {
          Ok(Self::wrap(value))
        } else {
          Err(TryNewError)
        }
      }
    }

    impl<B> Int<B, $int> {
      /// Returns the value.
      #[must_use]
      #[inline]
      pub const fn value(self) -> $int {
        self.repr
      }

      integer_query_table!(inherent_items![$int]);
      signed_query_table!(inherent_items![$int]);
    }

    native_integers!(@trait Uint, $uint);
    native_integers!(@trait Int, $int);
  )+};
  // The methods call the inherent `const fn`s, which `Self::` names first.
  (@trait $integer:ident, $native:ident) => {
    impl<B: FitsBits<$native>> NativeInteger for $integer<B, $native> {
      type Native = $native;

      #[inline]
      #[track_caller]
      fn new(value: $native) -> Self {
        Self::new(value)
      }

      #[inline]
      fn try_new(value: $native) -> Result<Self, TryNewError> {
        Self::try_new(value)
      }

      #[inline]
      fn value(self) -> $native {
        Self::value(self)
      }
    }
  };
}

native_integers!(
  u8, i8;
  u16, i16;
  u32, i32;
  u64, i64;
  u128, i128;
);

// Trait methods cannot be `const`.
macro_rules! integer_traits {
  ($name:ident, $repr:ident) => {
    impl<B, R> Clone for $name<B, R>
    where
      R: Clone,
    {
      #[inline]
      fn clone(&self) -> Self {
        Self::wrap(self.repr.clone())
      }
    }

    impl<B, R> Copy for $name<B, R> where R: Copy {}

    impl<B, R> PartialEq for $name<B, R>
    where
      R: PartialEq,
    {
      #[inline]
      fn eq(&self, other: &Self) -> bool {
        self.repr == other.repr
      }
    }

    impl<B, R> Eq for $name<B, R> where R: Eq {}

    impl<B, R> PartialOrd for $name<B, R>
    where
      R: $repr,
    {
      #[inline]
      fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
      }
    }

    impl<B, R> Ord for $name<B, R>
    where
      R: $repr,
    {
      /// Compares the values. A native integer compares as its integer type does, and an array
      /// from the most significant limb down, as the array order would compare the least
      /// significant limb first.
      #[inline]
      fn cmp(&self, other: &Self) -> Ordering {
        self.repr.compare(&other.repr, Token::new())
      }
    }

    impl<B, R> Hash for $name<B, R>
    where
      R: $repr,
      B: FitsBits<R>,
    {
      /// Writes the lowest `ceil(Bits / 64)` 64-bit words of the value, in two's complement for
      /// `Int`, least significant first and without the length prefix of an array, so that the hash
      /// depends only on the value and `Bits`, whatever the storage: a native integer writes what
      /// an array of one limb of its width writes, so `Uint<U8, u8>` hashes as `Uint<U8, [u8; 1]>`
      /// does.
      #[inline]
      fn hash<H: Hasher>(&self, state: &mut H) {
        self.repr.hash_limbs(B::U32, state, Token::new());
      }
    }

    impl<B, R> fmt::Display for $name<B, R>
    where
      R: $repr,
    {
      /// Writes the value in decimal, honoring width, fill, alignment, `+` and `0` as integers do.
      ///
      /// A native integer writes its value as the native integer does. In an array, the digits take
      /// a stack buffer of `3 * N`, `5 * N`, `10 * N`, `20 * N` or `39 * N` bytes for limbs of 8,
      /// 16, 32, 64 or 128 bits, and time quadratic in `N`.
      #[inline]
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.repr.fmt_decimal(f, Token::new())
      }
    }

    impl<B, R> fmt::Debug for $name<B, R>
    where
      R: $repr,
    {
      /// Writes the same text as [`Display`](fmt::Display): decimal for every storage, ignoring
      /// the hexadecimal flags of `{:x?}` and `{:X?}`.
      #[inline]
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
      }
    }

    impl<B, R> Default for $name<B, R>
    where
      R: $repr,
      B: FitsBits<R>,
    {
      /// Returns zero.
      #[inline]
      fn default() -> Self {
        Self::wrap(R::zero(Token::new()))
      }
    }
  };
}

integer_traits!(Uint, UintRepr);
integer_traits!(Int, IntRepr);

#[cfg(test)]
mod tests;
