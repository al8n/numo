//! The bytes of [`Uint`](crate::Uint) and [`Int`](crate::Int): the byte methods of every storage,
//! which `byte_methods!` emits as inherent `const fn`s, and the panics of the methods that index
//! the bytes or copy them to a buffer.
//!
//! Every storage holds the value of `Bits` bits in its low `BYTES` bytes, zero-extended for `Uint`
//! and sign-extended for `Int`, so those bytes, least significant first, are the bytes of the
//! value. The arrays of bytes have their length in their type, `[u8; BYTES]`, through a constant
//! `M` that the bound `u8: LimbArray<B, Array = [u8; M]>` fixes; the slices and the copies to a
//! buffer take any width.

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
