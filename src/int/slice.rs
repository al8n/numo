//! The integers in slices of bytes, which the layers of every storage read and write.
//!
//! A slice of `len` bytes holds an integer of `8 * len` bits, unsigned or in two's complement, its
//! byte of significance `k` at index `k`, least significant first, or at index `len - 1 - k`, most
//! significant first, `big_endian`. Beyond its bytes the integer extends by zeros, or by copies of
//! its sign in two's complement, and no bytes hold zero.

/// The index in a slice of `len` bytes of the byte of significance `k`: `k`, or `len - 1 - k` if
/// the bytes are most significant first. Requires `k < len`.
#[inline]
pub(crate) const fn index(len: usize, k: usize, big_endian: bool) -> usize {
  if big_endian { len - 1 - k } else { k }
}

/// The byte of significance `k` of the integer in `bytes`, or from `bytes.len()` up `fill`, the
/// byte that extends it.
#[inline]
pub(crate) const fn byte(bytes: &[u8], k: usize, big_endian: bool, fill: u8) -> u8 {
  if k < bytes.len() {
    bytes[index(bytes.len(), k, big_endian)]
  } else {
    fill
  }
}

/// The byte that extends the integer in `bytes`, read in two's complement if `signed`: `0xff`, the
/// copies of its sign, if the top bit of its most significant byte is set, and zero otherwise, as
/// for an unsigned integer and for no bytes.
#[inline]
pub(crate) const fn fill(bytes: &[u8], big_endian: bool, signed: bool) -> u8 {
  match bytes.len() {
    0 => 0,
    len if signed && byte(bytes, len - 1, big_endian, 0) >= 0x80 => 0xff,
    _ => 0,
  }
}

/// Returns `true` if the integer in `bytes`, read in two's complement if `signed` and unsigned
/// otherwise, is within the range of `bits` bits: every bit from index `bits` up is zero for an
/// unsigned integer, and every bit from index `bits - 1` up copies the sign for a signed one,
/// whose zero bits hold only zero, as for an unsigned one.
pub(crate) const fn fits_bits(bytes: &[u8], big_endian: bool, bits: u32, signed: bool) -> bool {
  // The lowest bit that must equal the bits of `fill`, and `fill`.
  let (low, fill) = if signed && bits > 0 {
    (bits - 1, fill(bytes, big_endian, true))
  } else {
    (bits, 0)
  };
  // The significance of the byte that holds bit `low`, compared before it is narrowed to `usize`,
  // which can have fewer bits than `u32`, and the bits below `low` in that byte.
  let (first, below) = ((low / 8) as u64, low % 8);
  let len = bytes.len();
  let mut k = if first < len as u64 {
    first as usize
  } else {
    len
  };
  while k < len {
    let differ = byte(bytes, k, big_endian, fill) ^ fill;
    let excess = if k as u64 == first {
      differ >> below
    } else {
      differ
    };
    if excess != 0 {
      return false;
    }
    k += 1;
  }
  true
}
