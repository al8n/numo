//! The bytes of [`Uint`] and [`Int`]: the byte methods of every storage, which `byte_methods!`
//! emits as inherent `const fn`s, the traits that give them to generic code, and the panics of the
//! methods that index the bytes, copy them to a buffer, or read a slice of bytes, or of limbs,
//! strictly.
//!
//! Every storage holds the value of `Bits` bits in its low `BYTES` bytes, zero-extended for `Uint`
//! and sign-extended for `Int`, so those bytes, least significant first, are the bytes of the
//! value. The arrays of bytes have their length in their type, `[u8; BYTES]`, through a constant
//! `M` that the bound `u8: LimbArray<B, Array = [u8; M]>` fixes; the slices and the copies to a
//! buffer take any width. The traits come in one per bound: [`IntegerBytes`] for every width and
//! storage, [`ByteArray`] for the widths that have an array, [`FromByteArray`] for those of whole
//! bytes, and [`IntegerByteOrder`] for every width of whole bytes.

use super::{Int, NonZeroInt, NonZeroUint, Uint, nonzero::Parts, traits::Sealed};
use crate::{
  layout::Token,
  width::{FitsBits, IntRepr, LimbArray, NonZeroIntRepr, NonZeroUintRepr, UintRepr, WholeBytes},
};

/// The number of bytes of a value of `bits` bits, `ceil(bits / 8)`, which `BYTES` holds.
///
/// The count is computed in `u32`, which holds every width, and only then narrowed to `usize`,
/// which can have fewer bits: on a 16-bit target, the `USIZE` of `typenum` is zero for a width of
/// `2^16` bits. A count that `usize` cannot hold, which only a width whose storage does not fit in
/// memory has, fails the evaluation of `BYTES`, and so the build, instead of giving a wrong count.
pub(crate) const fn byte_count(bits: u32) -> usize {
  let bytes = bits.div_ceil(8);
  // Compared in `u64`, which holds both sides on every target.
  assert!(
    bytes as u64 <= usize::MAX as u64,
    "the number of bytes of the width does not fit in `usize`"
  );
  bytes as usize
}

/// Panics on a byte index of `BYTES` or more, which no byte of the value has, as indexing the
/// array of its bytes would.
#[cold]
#[track_caller]
pub(crate) const fn byte_index() -> ! {
  panic!("byte index out of range")
}

/// Panics on a buffer shorter than the bytes of the value in `copy_le_bytes_to`.
#[cold]
#[track_caller]
pub(crate) const fn short_le_buffer() -> ! {
  panic!("copy_le_bytes_to: the buffer is shorter than the bytes of the value")
}

/// Panics on a buffer shorter than the bytes of the value in `copy_be_bytes_to`.
#[cold]
#[track_caller]
pub(crate) const fn short_be_buffer() -> ! {
  panic!("copy_be_bytes_to: the buffer is shorter than the bytes of the value")
}

/// Panics on bytes, least significant first, that hold an integer outside the range of the integer
/// in `strict_from_le_slice`.
#[cold]
#[track_caller]
pub(crate) const fn strict_le_slice() -> ! {
  panic!("strict_from_le_slice: the bytes hold a value outside the range of the integer")
}

/// Panics on bytes, most significant first, that hold an integer outside the range of the integer
/// in `strict_from_be_slice`.
#[cold]
#[track_caller]
pub(crate) const fn strict_be_slice() -> ! {
  panic!("strict_from_be_slice: the bytes hold a value outside the range of the integer")
}

/// Panics on limbs that hold an integer outside the range of the integer in
/// `strict_from_limbs_slice`.
#[cold]
#[track_caller]
pub(crate) const fn strict_limbs_slice() -> ! {
  panic!("strict_from_limbs_slice: the limbs hold a value outside the range of the integer")
}

/// Emits the byte methods of `Uint` and `Int` as inherent `const fn`s of one storage, in an impl
/// block whose width relation holds: `$args` is `[layer]` for a native integer, which its layer in
/// `int` takes by value, or `[group, layer, &]` for an array, which its layers take by reference,
/// as for `inherent_items!`. An array writes its bytes and reads the byte at an index through the
/// lower layer of its limb type, which serves both signs, and reads bytes through the layer of its
/// sign.
// A method bound and a constant of a method do not fit the rows of the tables, so the methods are
// written here once for every storage.
macro_rules! byte_methods {
  // The function `$f` of the layer of the sign of the storage.
  (@layer [$group:ident, $layer:ident, &] $f:ident($($arg:expr),*)) => {
    int::$group::$layer::$f($($arg),*)
  };
  (@layer [$layer:ident] $f:ident($($arg:expr),*)) => {
    int::$layer::$f($($arg),*)
  };
  // The function `$f` of the storage of `$value` in the layer of its sign, or in the lower layer
  // for an array.
  (@own [$group:ident, $layer:ident, &] $f:ident($value:expr $(, $arg:expr)*)) => {
    int::$group::$layer::$f(&$value.repr $(, $arg)*)
  };
  (@own [$layer:ident] $f:ident($value:expr $(, $arg:expr)*)) => {
    int::$layer::$f($value.repr $(, $arg)*)
  };
  (@lower [$group:ident, $layer:ident, &] $f:ident($value:expr $(, $arg:expr)*)) => {
    int::unsigned::$layer::$f(&$value.repr $(, $arg)*)
  };
  (@lower [$layer:ident] $f:ident($value:expr $(, $arg:expr)*)) => {
    int::$layer::$f($value.repr $(, $arg)*)
  };
  ($args:tt) => {
    /// Writes the low `out.len()` bytes of the storage, which hold the value in its low
    /// [`BYTES`](Self::BYTES) bytes, to `out`, least significant first, or most significant first
    /// if `big_endian`. Requires `out.len() <= BYTES`.
    #[inline]
    const fn write_bytes(&self, out: &mut [u8], big_endian: bool) {
      byte_methods!(@lower $args write_bytes(self, out, big_endian));
    }

    /// The integer in `bytes`, least significant byte first, or most significant first if
    /// `big_endian`, unsigned for `Uint` and in two's complement of `8 * bytes.len()` bits for
    /// `Int`, or `None` if it is outside the range of `Bits` bits.
    #[inline]
    const fn checked_read_bytes(bytes: &[u8], big_endian: bool) -> Option<Self> {
      match byte_methods!(@layer $args checked_read_bytes(bytes, big_endian, Self::BITS)) {
        Some(repr) => Some(Self::wrap(repr)),
        None => None,
      }
    }

    /// The integer in `bytes`, read as [`checked_read_bytes`](Self::checked_read_bytes) reads
    /// it, modulo `2^Bits`.
    #[inline]
    const fn wrapping_read_bytes(bytes: &[u8], big_endian: bool) -> Self {
      Self::wrap(byte_methods!(@layer $args wrapping_read_bytes(bytes, big_endian, Self::BITS)))
    }

    /// The integer in `bytes`, read as [`checked_read_bytes`](Self::checked_read_bytes) reads
    /// it, modulo `2^Bits`, and whether it is outside the range of `Bits` bits.
    #[inline]
    const fn overflowing_read_bytes(bytes: &[u8], big_endian: bool) -> (Self, bool) {
      match Self::checked_read_bytes(bytes, big_endian) {
        Some(value) => (value, false),
        None => (Self::wrapping_read_bytes(bytes, big_endian), true),
      }
    }

    /// The integer in `bytes`, read as [`checked_read_bytes`](Self::checked_read_bytes) reads
    /// it, or the bound of the range of `Bits` bits that it passes.
    #[inline]
    const fn saturating_read_bytes(bytes: &[u8], big_endian: bool) -> Self {
      Self::wrap(byte_methods!(@layer $args saturating_read_bytes(bytes, big_endian, Self::BITS)))
    }

    /// Writes the bytes of the value to the start of `buf`, least significant first, or most
    /// significant first if `big_endian`, and returns their number, or returns `None` if `buf`
    /// is shorter and leaves it unchanged.
    #[inline]
    const fn checked_copy_bytes_to(&self, buf: &mut [u8], big_endian: bool) -> Option<usize> {
      match buf.split_at_mut_checked(Self::BYTES) {
        Some((head, _)) => {
          self.write_bytes(head, big_endian);
          Some(Self::BYTES)
        }
        None => None,
      }
    }

    /// The bytes of the value, least significant first: [`BYTES`](Self::BYTES) of them,
    /// `ceil(Bits / 8)`, in two's complement for `Int`. At a width that is not a whole number of
    /// bytes, the bits of the last byte above the width are zeros for `Uint` and copies of the
    /// sign for `Int`, as the storage holds the value, so the bytes, read as an integer of
    /// `8 * BYTES` bits, unsigned or in two's complement, give the value.
    ///
    /// The length is part of the type: the bound fixes `M` at `BYTES`, so the length is inferred,
    /// a wrong one fails to compile, and the array exists up to 8192 bits, as
    /// [`Limbs<Bits, u8>`](crate::Limbs) does. A wider value copies its bytes to a buffer with
    /// [`copy_le_bytes_to`](Self::copy_le_bytes_to).
    #[must_use]
    #[inline]
    pub const fn to_le_bytes<const M: usize>(&self) -> [u8; M]
    where
      u8: LimbArray<B, Array = [u8; M]>,
    {
      let mut bytes = [0; M];
      self.write_bytes(&mut bytes, false);
      bytes
    }

    /// The bytes of the value, most significant first: those of
    /// [`to_le_bytes`](Self::to_le_bytes) in reverse order.
    #[must_use]
    #[inline]
    pub const fn to_be_bytes<const M: usize>(&self) -> [u8; M]
    where
      u8: LimbArray<B, Array = [u8; M]>,
    {
      let mut bytes = [0; M];
      self.write_bytes(&mut bytes, true);
      bytes
    }

    /// The bytes of the value in the byte order of the target: those of
    /// [`to_be_bytes`](Self::to_be_bytes) on a big-endian target, and of
    /// [`to_le_bytes`](Self::to_le_bytes) on a little-endian one. They are the value in that
    /// order, not the memory of the storage: an array keeps its limbs least significant first on
    /// every target.
    #[must_use]
    #[inline]
    pub const fn to_ne_bytes<const M: usize>(&self) -> [u8; M]
    where
      u8: LimbArray<B, Array = [u8; M]>,
    {
      let mut bytes = [0; M];
      self.write_bytes(&mut bytes, cfg!(target_endian = "big"));
      bytes
    }

    /// Creates an integer from its bytes, least significant first, in two's complement for
    /// `Int`, or returns `None` if they hold no value of `Bits` bits: if a bit of the last byte
    /// above the width is not zero for `Uint`, or not a copy of the sign bit, bit `Bits - 1`, for
    /// `Int`. At a width of whole bytes every array holds a value, so this never returns `None`,
    /// and [`from_le_bytes`](Self::from_le_bytes) needs no check.
    #[must_use]
    #[inline]
    pub const fn checked_from_le_bytes<const M: usize>(bytes: [u8; M]) -> Option<Self>
    where
      u8: LimbArray<B, Array = [u8; M]>,
    {
      Self::checked_read_bytes(&bytes, false)
    }

    /// Creates an integer from its bytes, most significant first, or returns `None` if they hold
    /// no value of `Bits` bits, as [`checked_from_le_bytes`](Self::checked_from_le_bytes) does
    /// for them in reverse order.
    #[must_use]
    #[inline]
    pub const fn checked_from_be_bytes<const M: usize>(bytes: [u8; M]) -> Option<Self>
    where
      u8: LimbArray<B, Array = [u8; M]>,
    {
      Self::checked_read_bytes(&bytes, true)
    }

    /// Creates an integer from its bytes in the byte order of the target, or returns `None` if
    /// they hold no value of `Bits` bits, as
    /// [`checked_from_be_bytes`](Self::checked_from_be_bytes) does on a big-endian target and
    /// [`checked_from_le_bytes`](Self::checked_from_le_bytes) on a little-endian one.
    #[must_use]
    #[inline]
    pub const fn checked_from_ne_bytes<const M: usize>(bytes: [u8; M]) -> Option<Self>
    where
      u8: LimbArray<B, Array = [u8; M]>,
    {
      Self::checked_read_bytes(&bytes, cfg!(target_endian = "big"))
    }

    /// Creates an integer from its bytes, least significant first, in two's complement for
    /// `Int`. `Bits` is a whole number of bytes, so every array holds a value and this never
    /// fails; [`checked_from_le_bytes`](Self::checked_from_le_bytes) reads any width.
    #[must_use]
    #[inline]
    pub const fn from_le_bytes<const M: usize>(bytes: [u8; M]) -> Self
    where
      u8: LimbArray<B, Array = [u8; M]>,
      B: WholeBytes,
    {
      Self::wrapping_read_bytes(&bytes, false)
    }

    /// Creates an integer from its bytes, most significant first, as
    /// [`from_le_bytes`](Self::from_le_bytes) does from them in reverse order.
    #[must_use]
    #[inline]
    pub const fn from_be_bytes<const M: usize>(bytes: [u8; M]) -> Self
    where
      u8: LimbArray<B, Array = [u8; M]>,
      B: WholeBytes,
    {
      Self::wrapping_read_bytes(&bytes, true)
    }

    /// Creates an integer from its bytes in the byte order of the target, as
    /// [`from_be_bytes`](Self::from_be_bytes) does on a big-endian target and
    /// [`from_le_bytes`](Self::from_le_bytes) on a little-endian one.
    #[must_use]
    #[inline]
    pub const fn from_ne_bytes<const M: usize>(bytes: [u8; M]) -> Self
    where
      u8: LimbArray<B, Array = [u8; M]>,
      B: WholeBytes,
    {
      Self::wrapping_read_bytes(&bytes, cfg!(target_endian = "big"))
    }

    /// Creates an integer from the integer in `bytes`, least significant byte first, of any
    /// length: unsigned for `Uint`, and for `Int` in two's complement of `8 * bytes.len()` bits,
    /// whose sign is the top bit of the last byte; no bytes hold zero. Returns `None` if that
    /// integer is outside the range of `Bits` bits, whatever the length, so bytes that only
    /// extend it, zeros or copies of its sign, are accepted.
    #[must_use]
    #[inline]
    pub const fn checked_from_le_slice(bytes: &[u8]) -> Option<Self> {
      Self::checked_read_bytes(bytes, false)
    }

    /// Creates an integer from the integer in `bytes`, most significant byte first, as
    /// [`checked_from_le_slice`](Self::checked_from_le_slice) does from them in reverse order:
    /// the sign of `Int` is the top bit of the first byte.
    #[must_use]
    #[inline]
    pub const fn checked_from_be_slice(bytes: &[u8]) -> Option<Self> {
      Self::checked_read_bytes(bytes, true)
    }

    /// Creates an integer from the integer in `bytes`, least significant byte first, read as
    /// [`checked_from_le_slice`](Self::checked_from_le_slice) reads it, modulo `2^Bits`: for
    /// `Int`, reduced into the range of `Bits` bits. Wherever `checked_from_le_slice` returns a
    /// value, this returns it too.
    #[must_use]
    #[inline]
    pub const fn wrapping_from_le_slice(bytes: &[u8]) -> Self {
      Self::wrapping_read_bytes(bytes, false)
    }

    /// Creates an integer from the integer in `bytes`, most significant byte first, as
    /// [`wrapping_from_le_slice`](Self::wrapping_from_le_slice) does from them in reverse order.
    #[must_use]
    #[inline]
    pub const fn wrapping_from_be_slice(bytes: &[u8]) -> Self {
      Self::wrapping_read_bytes(bytes, true)
    }

    /// Creates an integer from the integer in `bytes`, least significant byte first, read as
    /// [`checked_from_le_slice`](Self::checked_from_le_slice) reads it, modulo `2^Bits`, as
    /// [`wrapping_from_le_slice`](Self::wrapping_from_le_slice) does, and returns whether that
    /// integer is outside the range of `Bits` bits, where `checked_from_le_slice` returns `None`.
    #[must_use]
    #[inline]
    pub const fn overflowing_from_le_slice(bytes: &[u8]) -> (Self, bool) {
      Self::overflowing_read_bytes(bytes, false)
    }

    /// Creates an integer from the integer in `bytes`, most significant byte first, as
    /// [`overflowing_from_le_slice`](Self::overflowing_from_le_slice) does from them in reverse
    /// order.
    #[must_use]
    #[inline]
    pub const fn overflowing_from_be_slice(bytes: &[u8]) -> (Self, bool) {
      Self::overflowing_read_bytes(bytes, true)
    }

    /// Creates an integer from the integer in `bytes`, least significant byte first, read as
    /// [`checked_from_le_slice`](Self::checked_from_le_slice) reads it, or the bound of the range
    /// of `Bits` bits that it passes: [`MAX`](Self::MAX) above the range, and for `Int`
    /// [`MIN`](Self::MIN) below it, which the integer passes when the top bit of its last byte, its
    /// sign, is set.
    #[must_use]
    #[inline]
    pub const fn saturating_from_le_slice(bytes: &[u8]) -> Self {
      Self::saturating_read_bytes(bytes, false)
    }

    /// Creates an integer from the integer in `bytes`, most significant byte first, as
    /// [`saturating_from_le_slice`](Self::saturating_from_le_slice) does from them in reverse
    /// order: the sign of `Int` is the top bit of the first byte.
    #[must_use]
    #[inline]
    pub const fn saturating_from_be_slice(bytes: &[u8]) -> Self {
      Self::saturating_read_bytes(bytes, true)
    }

    /// Creates an integer from the integer in `bytes`, least significant byte first, read as
    /// [`checked_from_le_slice`](Self::checked_from_le_slice) reads it.
    ///
    /// # Panics
    ///
    /// Panics if that integer is outside the range of `Bits` bits, in every build, reporting the
    /// caller; `checked_from_le_slice` returns `None` instead.
    #[must_use]
    #[inline]
    #[track_caller]
    pub const fn strict_from_le_slice(bytes: &[u8]) -> Self {
      match Self::checked_read_bytes(bytes, false) {
        Some(value) => value,
        None => bytes::strict_le_slice(),
      }
    }

    /// Creates an integer from the integer in `bytes`, most significant byte first, as
    /// [`strict_from_le_slice`](Self::strict_from_le_slice) does from them in reverse order.
    ///
    /// # Panics
    ///
    /// Panics if that integer is outside the range of `Bits` bits, in every build, reporting the
    /// caller; [`checked_from_be_slice`](Self::checked_from_be_slice) returns `None` instead.
    #[must_use]
    #[inline]
    #[track_caller]
    pub const fn strict_from_be_slice(bytes: &[u8]) -> Self {
      match Self::checked_read_bytes(bytes, true) {
        Some(value) => value,
        None => bytes::strict_be_slice(),
      }
    }

    /// Writes the bytes of the value, least significant first, those of
    /// [`to_le_bytes`](Self::to_le_bytes), to the start of `buf`, and returns their number,
    /// [`BYTES`](Self::BYTES); the rest of `buf` is unchanged. It takes any width, also one above
    /// 8192 bits, which has no array of bytes.
    ///
    /// # Panics
    ///
    /// Panics if `buf` is shorter than `BYTES`, reporting the caller;
    /// [`checked_copy_le_bytes_to`](Self::checked_copy_le_bytes_to) returns `None` instead.
    #[inline]
    #[track_caller]
    pub const fn copy_le_bytes_to(&self, buf: &mut [u8]) -> usize {
      match self.checked_copy_bytes_to(buf, false) {
        Some(written) => written,
        None => bytes::short_le_buffer(),
      }
    }

    /// Writes the bytes of the value, most significant first, those of
    /// [`to_be_bytes`](Self::to_be_bytes), to the start of `buf`, as
    /// [`copy_le_bytes_to`](Self::copy_le_bytes_to) does.
    ///
    /// # Panics
    ///
    /// Panics if `buf` is shorter than [`BYTES`](Self::BYTES), reporting the caller;
    /// [`checked_copy_be_bytes_to`](Self::checked_copy_be_bytes_to) returns `None` instead.
    #[inline]
    #[track_caller]
    pub const fn copy_be_bytes_to(&self, buf: &mut [u8]) -> usize {
      match self.checked_copy_bytes_to(buf, true) {
        Some(written) => written,
        None => bytes::short_be_buffer(),
      }
    }

    /// Writes the bytes of the value, least significant first, to the start of `buf`, and
    /// returns their number, as [`copy_le_bytes_to`](Self::copy_le_bytes_to) does, or returns
    /// `None` if `buf` is shorter than [`BYTES`](Self::BYTES), and leaves it unchanged.
    #[must_use]
    #[inline]
    pub const fn checked_copy_le_bytes_to(&self, buf: &mut [u8]) -> Option<usize> {
      self.checked_copy_bytes_to(buf, false)
    }

    /// Writes the bytes of the value, most significant first, to the start of `buf`, and returns
    /// their number, as [`copy_be_bytes_to`](Self::copy_be_bytes_to) does, or returns `None` if
    /// `buf` is shorter than [`BYTES`](Self::BYTES), and leaves it unchanged.
    #[must_use]
    #[inline]
    pub const fn checked_copy_be_bytes_to(&self, buf: &mut [u8]) -> Option<usize> {
      self.checked_copy_bytes_to(buf, true)
    }

    /// The byte of the value at `index`, its bits from `8 * index` to `8 * index + 7`: the byte
    /// at `index` of [`to_le_bytes`](Self::to_le_bytes), at any width, in two's complement for
    /// `Int`.
    ///
    /// # Panics
    ///
    /// Panics if `index` is [`BYTES`](Self::BYTES) or more, as indexing the bytes would,
    /// reporting the caller, unlike [`bit`](Self::bit), which returns `false` from `Bits` up.
    #[must_use]
    #[inline]
    #[track_caller]
    pub const fn byte(&self, index: usize) -> u8 {
      if index >= Self::BYTES {
        bytes::byte_index();
      }
      byte_methods!(@lower $args byte_at(self, index))
    }

    /// The value with its [`BYTES`](Self::BYTES) bytes in reverse order, at a width of whole
    /// bytes, read again in two's complement for `Int`, as the native `swap_bytes` reverses the
    /// bytes of its width. Reversing them twice gives the value back.
    #[must_use]
    #[inline]
    pub const fn swap_bytes(&self) -> Self
    where
      B: WholeBytes,
    {
      Self::wrap(byte_methods!(@own $args swap_bytes_bits(self, Self::BITS)))
    }

    /// Converts the value to big-endian order, as the native `to_be` does: `self` on a
    /// big-endian target and [`swap_bytes`](Self::swap_bytes) on a little-endian one. It
    /// converts the value, not a layout: an array keeps its limbs least significant first on
    /// every target, and the arrays of bytes, such as [`to_be_bytes`](Self::to_be_bytes), give
    /// the bytes in an order.
    #[must_use]
    #[inline]
    pub const fn to_be(&self) -> Self
    where
      B: WholeBytes,
    {
      if cfg!(target_endian = "big") {
        *self
      } else {
        self.swap_bytes()
      }
    }

    /// Converts the value to little-endian order, as the native `to_le` does: `self` on a
    /// little-endian target and [`swap_bytes`](Self::swap_bytes) on a big-endian one, a
    /// conversion of the value, as for [`to_be`](Self::to_be).
    #[must_use]
    #[inline]
    pub const fn to_le(&self) -> Self
    where
      B: WholeBytes,
    {
      if cfg!(target_endian = "big") {
        self.swap_bytes()
      } else {
        *self
      }
    }

    /// Converts `x` from big-endian order, as the native `from_be` does: `x.to_be()`, a
    /// conversion of the value, as for [`to_be`](Self::to_be).
    #[must_use]
    #[inline]
    pub const fn from_be(x: Self) -> Self
    where
      B: WholeBytes,
    {
      x.to_be()
    }

    /// Converts `x` from little-endian order, as the native `from_le` does: `x.to_le()`, a
    /// conversion of the value, as for [`to_be`](Self::to_be).
    #[must_use]
    #[inline]
    pub const fn from_le(x: Self) -> Self
    where
      B: WholeBytes,
    {
      x.to_le()
    }
  };
}

/// The bytes of an integer through slices, at any width: [`Uint`] and [`Int`] over every storage.
///
/// [`BYTES`](Self::BYTES) and the methods are the inherent items of the same names, which a
/// concrete type reaches without this trait, also in constant contexts: the byte at an index, the
/// copies of the bytes to a buffer, and the conversions from a slice of any length, checked,
/// wrapping, overflowing, saturating and strict, which read it unsigned for `Uint` and in two's
/// complement for `Int`. The trait needs neither `Copy` nor a constant of a value, and every
/// storage implements it, so the bound `B: BitWidth` provides it on `Uint<B>` and `Int<B>`; generic
/// code calls the methods with the trait in scope.
///
/// ```
/// use numo::{Int, IntegerBytes, Uint, i7, typenum::U200};
///
/// // The bytes of a value, most significant first, as hexadecimal digits.
/// fn hex<T: IntegerBytes>(value: &T) -> String {
///   let mut buf = [0; 64];
///   let len = value.copy_be_bytes_to(&mut buf);
///   buf[..len].iter().map(|byte| format!("{byte:02x}")).collect()
/// }
///
/// assert_eq!(hex(&Uint::<numo::typenum::U12>::MAX), "0fff");
/// assert_eq!(hex(&i7::new(-1)), "ff");
/// assert_eq!(hex(&Uint::<U200>::MAX), "ff".repeat(25));
/// // A slice of any length, whose value decides: 0xfff is 4095, which 12 signed bits do not hold.
/// type U12 = Uint<numo::typenum::U12>;
/// type I12 = Int<numo::typenum::U12>;
/// assert_eq!(<U12 as IntegerBytes>::checked_from_le_slice(&[0xff, 0x0f, 0]), Some(U12::MAX));
/// assert_eq!(<I12 as IntegerBytes>::checked_from_le_slice(&[0xff, 0x0f]), None);
/// assert_eq!(<I12 as IntegerBytes>::wrapping_from_le_slice(&[0x80]), I12::new(-128));
/// assert_eq!(<I12 as IntegerBytes>::saturating_from_le_slice(&[0xff, 0x0f]), I12::MAX);
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait IntegerBytes: Sealed + Sized {
  /// The number of bytes of the value, `ceil(Bits / 8)`.
  const BYTES: usize;

  /// The byte of the value at `index`, its bits from `8 * index` to `8 * index + 7`.
  ///
  /// # Panics
  ///
  /// Panics if `index` is `BYTES` or more, reporting the caller.
  #[must_use]
  #[track_caller]
  fn byte(&self, index: usize) -> u8;

  /// Writes the bytes of the value, least significant first, to the start of `buf`, and returns
  /// their number, `BYTES`; the rest of `buf` is unchanged.
  ///
  /// # Panics
  ///
  /// Panics if `buf` is shorter than `BYTES`, reporting the caller.
  #[track_caller]
  fn copy_le_bytes_to(&self, buf: &mut [u8]) -> usize;

  /// Writes the bytes of the value, most significant first, to the start of `buf`, and returns
  /// their number, `BYTES`; the rest of `buf` is unchanged.
  ///
  /// # Panics
  ///
  /// Panics if `buf` is shorter than `BYTES`, reporting the caller.
  #[track_caller]
  fn copy_be_bytes_to(&self, buf: &mut [u8]) -> usize;

  /// Writes the bytes of the value, least significant first, to the start of `buf`, and returns
  /// their number, or returns `None` if `buf` is shorter than `BYTES`, and leaves it unchanged.
  #[must_use]
  fn checked_copy_le_bytes_to(&self, buf: &mut [u8]) -> Option<usize>;

  /// Writes the bytes of the value, most significant first, to the start of `buf`, and returns
  /// their number, or returns `None` if `buf` is shorter than `BYTES`, and leaves it unchanged.
  #[must_use]
  fn checked_copy_be_bytes_to(&self, buf: &mut [u8]) -> Option<usize>;

  /// Creates an integer from the integer in `bytes`, least significant byte first, of any length,
  /// unsigned for `Uint` and in two's complement of `8 * bytes.len()` bits for `Int`, or returns
  /// `None` if it is outside the range of `Bits` bits.
  #[must_use]
  fn checked_from_le_slice(bytes: &[u8]) -> Option<Self>;

  /// Creates an integer from the integer in `bytes`, most significant byte first, as
  /// [`checked_from_le_slice`](Self::checked_from_le_slice) does from them in reverse order.
  #[must_use]
  fn checked_from_be_slice(bytes: &[u8]) -> Option<Self>;

  /// Creates an integer from the integer in `bytes`, least significant byte first, read as
  /// [`checked_from_le_slice`](Self::checked_from_le_slice) reads it, modulo `2^Bits`.
  #[must_use]
  fn wrapping_from_le_slice(bytes: &[u8]) -> Self;

  /// Creates an integer from the integer in `bytes`, most significant byte first, as
  /// [`wrapping_from_le_slice`](Self::wrapping_from_le_slice) does from them in reverse order.
  #[must_use]
  fn wrapping_from_be_slice(bytes: &[u8]) -> Self;

  /// Creates an integer from the integer in `bytes`, least significant byte first, read as
  /// [`checked_from_le_slice`](Self::checked_from_le_slice) reads it, modulo `2^Bits`, and returns
  /// whether that integer is outside the range of `Bits` bits.
  #[must_use]
  fn overflowing_from_le_slice(bytes: &[u8]) -> (Self, bool);

  /// Creates an integer from the integer in `bytes`, most significant byte first, as
  /// [`overflowing_from_le_slice`](Self::overflowing_from_le_slice) does from them in reverse
  /// order.
  #[must_use]
  fn overflowing_from_be_slice(bytes: &[u8]) -> (Self, bool);

  /// Creates an integer from the integer in `bytes`, least significant byte first, read as
  /// [`checked_from_le_slice`](Self::checked_from_le_slice) reads it, or the bound of the range of
  /// `Bits` bits that it passes: `MAX` above the range, and `MIN` below it.
  #[must_use]
  fn saturating_from_le_slice(bytes: &[u8]) -> Self;

  /// Creates an integer from the integer in `bytes`, most significant byte first, as
  /// [`saturating_from_le_slice`](Self::saturating_from_le_slice) does from them in reverse order.
  #[must_use]
  fn saturating_from_be_slice(bytes: &[u8]) -> Self;

  /// Creates an integer from the integer in `bytes`, least significant byte first, read as
  /// [`checked_from_le_slice`](Self::checked_from_le_slice) reads it.
  ///
  /// # Panics
  ///
  /// Panics if that integer is outside the range of `Bits` bits, reporting the caller.
  #[must_use]
  #[track_caller]
  fn strict_from_le_slice(bytes: &[u8]) -> Self;

  /// Creates an integer from the integer in `bytes`, most significant byte first, as
  /// [`strict_from_le_slice`](Self::strict_from_le_slice) does from them in reverse order.
  ///
  /// # Panics
  ///
  /// Panics if that integer is outside the range of `Bits` bits, reporting the caller.
  #[must_use]
  #[track_caller]
  fn strict_from_be_slice(bytes: &[u8]) -> Self;
}

/// The bytes of an integer as an array whose length is part of its type: [`Uint`] and [`Int`]
/// over every storage, up to 8192 bits, where [`Limbs<Bits, u8>`](crate::Limbs) exists.
///
/// [`Bytes`](Self::Bytes) is `[u8; BYTES]`, which generic code reads through `AsRef<[u8]>` and
/// writes through `AsMut<[u8]>`, without a constant for its length. The methods are the inherent
/// items of the same names: the arrays in each order and the checked conversions from them, which
/// at a width that is not a whole number of bytes check the bits of the last byte above the width.
/// Generic code adds the bound `T: ByteArray`, which `B: BitWidth` does not imply, since the arrays
/// stop at 8192 bits.
///
/// ```
/// use numo::{
///   ByteArray, Int, Uint,
///   typenum::{U12, U24},
/// };
///
/// // The most significant byte of a value.
/// fn top<T: ByteArray>(value: &T) -> u8 {
///   value.to_be_bytes().as_ref()[0]
/// }
///
/// assert_eq!(top(&Uint::<U24>::new(0x01_0203)), 0x01);
/// assert_eq!(top(&Int::<U12>::new(-1)), 0xff);
/// // 12 bits leave four bits of the last byte, which must be zeros for `Uint`.
/// assert_eq!(<Uint<U12> as ByteArray>::checked_from_le_bytes([0xff, 0x1f]), None);
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait ByteArray: IntegerBytes {
  /// The array of the bytes, `[u8; BYTES]`.
  type Bytes: Copy + AsRef<[u8]> + AsMut<[u8]>;

  /// The bytes of the value, least significant first, in two's complement for `Int`.
  #[must_use]
  fn to_le_bytes(&self) -> Self::Bytes;

  /// The bytes of the value, most significant first.
  #[must_use]
  fn to_be_bytes(&self) -> Self::Bytes;

  /// The bytes of the value in the byte order of the target.
  #[must_use]
  fn to_ne_bytes(&self) -> Self::Bytes;

  /// Creates an integer from its bytes, least significant first, or returns `None` if they hold no
  /// value of `Bits` bits.
  #[must_use]
  fn checked_from_le_bytes(bytes: Self::Bytes) -> Option<Self>;

  /// Creates an integer from its bytes, most significant first, or returns `None` if they hold no
  /// value of `Bits` bits.
  #[must_use]
  fn checked_from_be_bytes(bytes: Self::Bytes) -> Option<Self>;

  /// Creates an integer from its bytes in the byte order of the target, or returns `None` if they
  /// hold no value of `Bits` bits.
  #[must_use]
  fn checked_from_ne_bytes(bytes: Self::Bytes) -> Option<Self>;
}

/// An integer of whole bytes from its array of bytes, every one of which holds a value: [`Uint`]
/// and [`Int`] whose `Bits` is [`WholeBytes`], up to 8192 bits.
///
/// The methods are the inherent items of the same names, which never fail.
///
/// ```
/// use numo::{FromByteArray, Int, Uint, typenum::U24};
///
/// // A value from the bytes of the network order, most significant first.
/// fn from_network<T: FromByteArray>(bytes: T::Bytes) -> T {
///   T::from_be_bytes(bytes)
/// }
///
/// assert_eq!(from_network::<Int<U24>>([0xff, 0xff, 0xfe]), Int::<U24>::new(-2));
/// assert_eq!(from_network::<Uint<U24>>([1, 2, 3]), Uint::<U24>::new(0x01_0203));
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait FromByteArray: ByteArray {
  /// Creates an integer from its bytes, least significant first, in two's complement for `Int`.
  #[must_use]
  fn from_le_bytes(bytes: Self::Bytes) -> Self;

  /// Creates an integer from its bytes, most significant first.
  #[must_use]
  fn from_be_bytes(bytes: Self::Bytes) -> Self;

  /// Creates an integer from its bytes in the byte order of the target.
  #[must_use]
  fn from_ne_bytes(bytes: Self::Bytes) -> Self;
}

/// The byte order of an integer of whole bytes: [`Uint`] and [`Int`] whose `Bits` is
/// [`WholeBytes`], over every storage, and the non-zero integers [`NonZeroUint`] and
/// [`NonZeroInt`] of such a width.
///
/// The methods are the inherent items of the same names: the reversal of the bytes, and the
/// conversions of the native integers to and from an order, which convert the value, not a layout.
/// A non-zero integer computes them on its integer, whose bytes in any order are not all zeros.
/// The trait needs neither `Copy` nor a constant of a value; generic code adds the bound
/// `B: WholeBytes` to `B: BitWidth`. Its implementations need a storage that is `Clone`, as every
/// storage is, to return the value itself where the order is that of the target.
///
/// ```
/// use numo::{BitWidth, IntegerByteOrder, Uint, WholeBytes, typenum::U24};
///
/// fn reversed<B: BitWidth + WholeBytes>(value: &Uint<B>) -> Uint<B> {
///   value.swap_bytes()
/// }
///
/// let value = Uint::<U24>::new(0x01_0203);
/// assert_eq!(reversed(&value), Uint::<U24>::new(0x03_0201));
/// assert_eq!(IntegerByteOrder::to_be(&value), value.to_be());
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait IntegerByteOrder: Sealed + Sized {
  /// The value with its bytes in reverse order.
  #[must_use]
  fn swap_bytes(&self) -> Self;

  /// Converts the value to big-endian order: `self` on a big-endian target and its bytes reversed
  /// on a little-endian one.
  #[must_use]
  fn to_be(&self) -> Self;

  /// Converts the value to little-endian order: `self` on a little-endian target and its bytes
  /// reversed on a big-endian one.
  #[must_use]
  fn to_le(&self) -> Self;

  /// Converts `x` from big-endian order: `x.to_be()`.
  #[must_use]
  fn from_be(x: Self) -> Self;

  /// Converts `x` from little-endian order: `x.to_le()`.
  #[must_use]
  fn from_le(x: Self) -> Self;
}

/// Implements the byte traits for [`Uint`] and [`Int`], written `Integer: Repr`, with the sealed
/// methods of the storage trait `Repr`, which neither need `Copy` nor read a constant of a value.
/// [`IntegerByteOrder`] also needs `Repr: Clone`, to return the value itself where the order is
/// that of the target.
// Trait methods cannot be `const`.
macro_rules! byte_traits {
  ($($integer:ident: $repr:ident),+ $(,)?) => {$(
    impl<B: FitsBits<R>, R: $repr> IntegerBytes for $integer<B, R> {
      const BYTES: usize = Self::BYTES;

      #[inline]
      #[track_caller]
      fn byte(&self, index: usize) -> u8 {
        if index >= Self::BYTES {
          byte_index();
        }
        self.repr.byte_at(index, Token::new())
      }

      #[inline]
      #[track_caller]
      fn copy_le_bytes_to(&self, buf: &mut [u8]) -> usize {
        match IntegerBytes::checked_copy_le_bytes_to(self, buf) {
          Some(written) => written,
          None => short_le_buffer(),
        }
      }

      #[inline]
      #[track_caller]
      fn copy_be_bytes_to(&self, buf: &mut [u8]) -> usize {
        match IntegerBytes::checked_copy_be_bytes_to(self, buf) {
          Some(written) => written,
          None => short_be_buffer(),
        }
      }

      #[inline]
      fn checked_copy_le_bytes_to(&self, buf: &mut [u8]) -> Option<usize> {
        let (head, _) = buf.split_at_mut_checked(Self::BYTES)?;
        self.repr.write_bytes(head, false, Token::new());
        Some(Self::BYTES)
      }

      #[inline]
      fn checked_copy_be_bytes_to(&self, buf: &mut [u8]) -> Option<usize> {
        let (head, _) = buf.split_at_mut_checked(Self::BYTES)?;
        self.repr.write_bytes(head, true, Token::new());
        Some(Self::BYTES)
      }

      #[inline]
      fn checked_from_le_slice(bytes: &[u8]) -> Option<Self> {
        R::checked_read_bytes(bytes, false, B::U32, Token::new()).map(Self::wrap)
      }

      #[inline]
      fn checked_from_be_slice(bytes: &[u8]) -> Option<Self> {
        R::checked_read_bytes(bytes, true, B::U32, Token::new()).map(Self::wrap)
      }

      #[inline]
      fn wrapping_from_le_slice(bytes: &[u8]) -> Self {
        Self::wrap(R::wrapping_read_bytes(bytes, false, B::U32, Token::new()))
      }

      #[inline]
      fn wrapping_from_be_slice(bytes: &[u8]) -> Self {
        Self::wrap(R::wrapping_read_bytes(bytes, true, B::U32, Token::new()))
      }

      #[inline]
      fn overflowing_from_le_slice(bytes: &[u8]) -> (Self, bool) {
        match IntegerBytes::checked_from_le_slice(bytes) {
          Some(value) => (value, false),
          None => (IntegerBytes::wrapping_from_le_slice(bytes), true),
        }
      }

      #[inline]
      fn overflowing_from_be_slice(bytes: &[u8]) -> (Self, bool) {
        match IntegerBytes::checked_from_be_slice(bytes) {
          Some(value) => (value, false),
          None => (IntegerBytes::wrapping_from_be_slice(bytes), true),
        }
      }

      #[inline]
      fn saturating_from_le_slice(bytes: &[u8]) -> Self {
        Self::wrap(R::saturating_read_bytes(bytes, false, B::U32, Token::new()))
      }

      #[inline]
      fn saturating_from_be_slice(bytes: &[u8]) -> Self {
        Self::wrap(R::saturating_read_bytes(bytes, true, B::U32, Token::new()))
      }

      #[inline]
      #[track_caller]
      fn strict_from_le_slice(bytes: &[u8]) -> Self {
        match IntegerBytes::checked_from_le_slice(bytes) {
          Some(value) => value,
          None => strict_le_slice(),
        }
      }

      #[inline]
      #[track_caller]
      fn strict_from_be_slice(bytes: &[u8]) -> Self {
        match IntegerBytes::checked_from_be_slice(bytes) {
          Some(value) => value,
          None => strict_be_slice(),
        }
      }
    }

    // The bound on `u8` fixes `M` at `BYTES`, so that `Bytes` is the array of the width.
    impl<B: FitsBits<R>, R: $repr, const M: usize> ByteArray for $integer<B, R>
    where
      u8: LimbArray<B, Array = [u8; M]>,
    {
      type Bytes = [u8; M];

      #[inline]
      fn to_le_bytes(&self) -> [u8; M] {
        let mut bytes = [0; M];
        self.repr.write_bytes(&mut bytes, false, Token::new());
        bytes
      }

      #[inline]
      fn to_be_bytes(&self) -> [u8; M] {
        let mut bytes = [0; M];
        self.repr.write_bytes(&mut bytes, true, Token::new());
        bytes
      }

      #[inline]
      fn to_ne_bytes(&self) -> [u8; M] {
        let mut bytes = [0; M];
        self
          .repr
          .write_bytes(&mut bytes, cfg!(target_endian = "big"), Token::new());
        bytes
      }

      #[inline]
      fn checked_from_le_bytes(bytes: [u8; M]) -> Option<Self> {
        IntegerBytes::checked_from_le_slice(&bytes)
      }

      #[inline]
      fn checked_from_be_bytes(bytes: [u8; M]) -> Option<Self> {
        IntegerBytes::checked_from_be_slice(&bytes)
      }

      #[inline]
      fn checked_from_ne_bytes(bytes: [u8; M]) -> Option<Self> {
        let big_endian = cfg!(target_endian = "big");
        R::checked_read_bytes(&bytes, big_endian, B::U32, Token::new()).map(Self::wrap)
      }
    }

    impl<B: FitsBits<R> + WholeBytes, R: $repr, const M: usize> FromByteArray for $integer<B, R>
    where
      u8: LimbArray<B, Array = [u8; M]>,
    {
      #[inline]
      fn from_le_bytes(bytes: [u8; M]) -> Self {
        IntegerBytes::wrapping_from_le_slice(&bytes)
      }

      #[inline]
      fn from_be_bytes(bytes: [u8; M]) -> Self {
        IntegerBytes::wrapping_from_be_slice(&bytes)
      }

      #[inline]
      fn from_ne_bytes(bytes: [u8; M]) -> Self {
        let big_endian = cfg!(target_endian = "big");
        Self::wrap(R::wrapping_read_bytes(&bytes, big_endian, B::U32, Token::new()))
      }
    }

    impl<B: FitsBits<R> + WholeBytes, R: $repr + Clone> IntegerByteOrder for $integer<B, R> {
      #[inline]
      fn swap_bytes(&self) -> Self {
        Self::wrap(self.repr.swap_bytes_bits(B::U32, Token::new()))
      }

      #[inline]
      fn to_be(&self) -> Self {
        if cfg!(target_endian = "big") {
          Self::wrap(self.repr.clone())
        } else {
          IntegerByteOrder::swap_bytes(self)
        }
      }

      #[inline]
      fn to_le(&self) -> Self {
        if cfg!(target_endian = "big") {
          IntegerByteOrder::swap_bytes(self)
        } else {
          Self::wrap(self.repr.clone())
        }
      }

      #[inline]
      fn from_be(x: Self) -> Self {
        if cfg!(target_endian = "big") {
          x
        } else {
          IntegerByteOrder::swap_bytes(&x)
        }
      }

      #[inline]
      fn from_le(x: Self) -> Self {
        if cfg!(target_endian = "big") {
          IntegerByteOrder::swap_bytes(&x)
        } else {
          x
        }
      }
    }
  )+};
}

byte_traits!(Uint: UintRepr, Int: IntRepr);

/// Implements [`IntegerByteOrder`] for the non-zero integers, written `NonZero: Repr`, through the
/// helpers of [`Parts`], over every storage whose integer storage is `Clone`, as the implementation
/// of the integer needs: each method computes the method of the integer on the value and wraps the
/// result, which is not zero.
macro_rules! nonzero_byte_traits {
  ($($nonzero:ident: $repr:ident),+ $(,)?) => {$(
    impl<B: FitsBits<R::Integer> + WholeBytes, R: $repr> IntegerByteOrder for $nonzero<B, R>
    where
      R::Integer: Clone,
    {
      #[inline]
      fn swap_bytes(&self) -> Self {
        Self::known(IntegerByteOrder::swap_bytes(&self.integer()))
      }

      #[inline]
      fn to_be(&self) -> Self {
        Self::known(IntegerByteOrder::to_be(&self.integer()))
      }

      #[inline]
      fn to_le(&self) -> Self {
        Self::known(IntegerByteOrder::to_le(&self.integer()))
      }

      #[inline]
      fn from_be(x: Self) -> Self {
        Self::known(IntegerByteOrder::from_be(x.integer()))
      }

      #[inline]
      fn from_le(x: Self) -> Self {
        Self::known(IntegerByteOrder::from_le(x.integer()))
      }
    }
  )+};
}

nonzero_byte_traits!(NonZeroUint: NonZeroUintRepr, NonZeroInt: NonZeroIntRepr);
