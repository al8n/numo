//! Integers whose width is a type-level number of bits.

use core::{
  cmp::Ordering,
  fmt,
  hash::{Hash, Hasher},
  marker::PhantomData,
  num::ParseIntError,
};

pub use bytes::{ByteArray, FromByteArray, IntegerByteOrder, IntegerBytes};
pub use cast::{BoundedCastFrom, CheckedCastFrom, TruncateFrom, WidenFrom};
pub use error::{FromFloatError, TryNewError};
pub use float::IntegerFloat;
pub use traits::{
  ArrayInteger, CheckedOps, ConstFixedInteger, ConstIntegerOne, ConstNonZeroLimits, FixedInteger,
  FixedNonZero, FromLimbs, IntegerBits, IntegerMath, IntegerNonZero, IntegerOne, NativeInteger,
  NonZeroLimits, NonZeroOf, OverflowingOps, SaturatingOps, SignedBits, SignedFixedInteger,
  SignedMath, SignedNonZero, StrictOps, UncheckedOps, UnsignedBits, UnsignedFixedInteger,
  UnsignedMath, UnsignedNonZero, WrappingOps,
};

use typenum::{IsGreater, NonZero, True, U1};

use crate::{
  int,
  layout::Token,
  width::{
    BitWidth, FillsRepr, FitsBits, InlineIntRepr, InlineUintRepr, IntRepr, IntWidth, LimbArray,
    UintRepr, UintWidth, WholeBytes,
  },
};

mod error;
#[macro_use]
mod tables;
#[macro_use]
mod arithmetic;
#[macro_use]
mod bits;
#[macro_use]
mod bytes;
#[macro_use]
mod float;
#[macro_use]
mod nonzero;
mod cast;
mod compare;
mod convert;
mod text;
mod traits;

/// An unsigned integer of `Bits` bits, stored in `Repr`.
///
/// The value runs from zero to `2^Bits - 1`, which is zero for zero bits. `Bits` is a [`typenum`]
/// number, and `Repr` holds at least `Bits` bits: a native unsigned integer, `u8`, `u16`, `u32`,
/// `u64` or `u128`, which holds the value itself, or an array of limbs, least significant first,
/// `[u8; N]`, `[u16; N]`, `[u32; N]`, `[u64; N]` or `[u128; N]`, of `8 * N`, `16 * N`, `32 * N`,
/// `64 * N` or `128 * N` bits. The bits above `Bits`, if any, are zero. Without `Repr`, the storage
/// is the default that [`BitWidth`] gives for 0 to 65536 bits: the zero-sized `[u64; 0]` for zero
/// bits, the smallest native integer that holds 1 to 128 bits, and above 128 bits
/// `[u64; ceil(Bits / 64)]`, so `Uint<U7>` is `Uint<U7, u8>`
/// and `Uint<U200>` is `Uint<U200, [u64; 4]>`. The module [`aliases`](crate::aliases) names common
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
/// a width that fills its limbs, [`checked_from_limbs`](Self::checked_from_limbs) from the limbs of
/// any width, checking that they hold a value of `Bits` bits, and
/// [`checked_from_limbs_slice`](Self::checked_from_limbs_slice) and its other forms from a slice of
/// limbs of any length (see [Bytes](Uint#bytes)). Values compare, hash and format by their numeric
/// value: `Ord` is numeric, and so are the comparisons with the native integers (see
/// [Conversions](Uint#conversions)), `Hash` depends only on the value and `Bits`, whatever the
/// storage, and `Display` and `Debug` write decimal.
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
/// `u7::new(100)`, `u256::from_limbs(limbs)`, `Uint::<U7>::new(100)`,
/// `Uint::<U200, [u64; 4]>::checked_from_limbs(limbs)` or `u7::from_str_radix("7f", 16)`.
///
/// # Arithmetic
///
/// Each storage has the arithmetic of the native unsigned integers at the width `Bits`, as
/// `const fn`s: addition, subtraction, multiplication, negation, division, remainder and
/// exponentiation in the `checked_*`, `wrapping_*`, `overflowing_*` and `strict_*` forms, the
/// Euclidean division and remainder in the same four forms, the saturating addition, subtraction,
/// multiplication, division and exponentiation, [`carrying_add`](Self::carrying_add) and
/// [`borrowing_sub`](Self::borrowing_sub), and from one bit the constant `ONE`. The functions
/// without an overflow prefix are [`pow`](Self::pow), [`div_euclid`](Self::div_euclid) and
/// [`rem_euclid`](Self::rem_euclid), [`div_floor`](Self::div_floor) and
/// [`div_ceil`](Self::div_ceil), [`next_multiple_of`](Self::next_multiple_of),
/// [`is_multiple_of`](Self::is_multiple_of), [`div_exact`](Self::div_exact),
/// [`isqrt`](Self::isqrt), the root of any degree [`nth_root`](Self::nth_root), the logarithms
/// [`ilog`](Self::ilog), [`ilog2`](Self::ilog2) and [`ilog10`](Self::ilog10),
/// [`midpoint`](Self::midpoint), [`abs_diff`](Self::abs_diff) and
/// [`next_power_of_two`](Self::next_power_of_two); those that can fail also have a `checked_*`
/// form, and the next power of two a `wrapping_*` form. Addition, subtraction, multiplication,
/// exact division and the shifts also have the `unchecked_*` form, such as
/// [`unchecked_add`](Self::unchecked_add), [`unchecked_div_exact`](Self::unchecked_div_exact) and
/// [`unchecked_shl`](Self::unchecked_shl), as `const unsafe fn`s: the caller guarantees that the
/// `checked_*` method would return `Some`, and the behavior is undefined otherwise. The operators
/// `+`, `-`, `*`, `/` and `%`, their compound assignments, `Sum` and `Product` work for every
/// storage, and so do the traits that give generic code the methods, such as
/// [`CheckedOps`](crate::CheckedOps) and [`IntegerMath`](crate::IntegerMath). A method of a storage
/// is called with method syntax or with a path that names the storage, such as
/// `Uint::<U7>::checked_add(&a, &b)`: a path that leaves the storage to inference, such as
/// `Uint::checked_add(&a, &b)`, is ambiguous (E0034).
///
/// Each storage also has the bit operations of the native unsigned integers at the width `Bits`, as
/// `const fn`s: the counts [`count_ones`](Self::count_ones), [`count_zeros`](Self::count_zeros),
/// [`leading_zeros`](Self::leading_zeros), [`trailing_zeros`](Self::trailing_zeros),
/// [`leading_ones`](Self::leading_ones) and [`trailing_ones`](Self::trailing_ones),
/// [`bit_width`](Self::bit_width), [`highest_one`](Self::highest_one) and
/// [`lowest_one`](Self::lowest_one) and their isolation, such as
/// [`isolate_highest_one`](Self::isolate_highest_one), [`is_power_of_two`](Self::is_power_of_two),
/// [`reverse_bits`](Self::reverse_bits), [`rotate_left`](Self::rotate_left) and
/// [`rotate_right`](Self::rotate_right), the shifts in the `checked_*`, `wrapping_*`,
/// `overflowing_*` and `strict_*` forms, [`unbounded_shl`](Self::unbounded_shl) and
/// [`unbounded_shr`](Self::unbounded_shr), and the exact shifts [`shl_exact`](Self::shl_exact) and
/// [`shr_exact`](Self::shr_exact), which follow the native methods, still unstable
/// ([#144336](https://github.com/rust-lang/rust/issues/144336)). [`bit`](Self::bit) reads a bit and
/// [`set_bit`](Self::set_bit) writes one, and [`cast_signed`](Self::cast_signed) reads the bits as
/// the [`Int`] of the same width in a storage of the same size. Every count, index and rotation
/// stays within the `Bits` bits, whatever the storage holds: `u7::MAX.leading_ones()` is 7, and
/// [`reverse_bits`](Self::reverse_bits) works at a width that is not a whole number of bytes. The
/// operators `!`, `&`, `|`, `^`, `<<` and `>>`, with an amount of any primitive integer type, and
/// their compound assignments work for every storage, and the traits
/// [`IntegerBits`](crate::IntegerBits) and [`UnsignedBits`](crate::UnsignedBits) give generic code
/// the bit methods. The operators cannot be `const`, so [`bitand`](Self::bitand),
/// [`bitor`](Self::bitor), [`bitxor`](Self::bitxor) and [`not`](Self::not) do the bitwise
/// operations as `const fn`s for constant contexts, and other code uses the operators. They take
/// their operands by value, as the methods of the operator traits do, so that method syntax on a
/// value, such as `A.bitand(B)`, calls them also where `core::ops::BitAnd` is in scope.
///
/// ```
/// use numo::{Uint, i7, typenum::U200, u7};
///
/// assert_eq!(u7::MAX.checked_add(&u7::ONE), None);
/// assert_eq!(u7::MAX.wrapping_add(&u7::ONE), u7::ZERO);
/// assert_eq!(u7::MAX.saturating_add(&u7::ONE), u7::MAX);
/// assert_eq!(u7::new(100) + u7::new(27), u7::MAX);
/// // SAFETY: 100 + 27 is at most `MAX`, so `checked_add` would return `Some`.
/// assert_eq!(unsafe { u7::new(100).unchecked_add(&u7::new(27)) }, u7::MAX);
/// // SAFETY: 7 is not zero and divides 126, so `checked_div_exact` would return `Some`.
/// assert_eq!(unsafe { u7::new(126).unchecked_div_exact(&u7::new(7)) }, u7::new(18));
///
/// // Division, which a zero divisor fails, and the functions of the native integers.
/// assert_eq!(u7::new(100) / u7::new(7), u7::new(14));
/// assert_eq!(u7::new(100) % u7::new(7), u7::new(2));
/// assert_eq!(u7::new(100).checked_div(&u7::ZERO), None);
/// assert_eq!(u7::new(100).div_ceil(&u7::new(7)), u7::new(15));
/// assert_eq!(u7::new(2).checked_pow(7), None);
/// assert_eq!(u7::new(100).isqrt(), u7::new(10));
/// assert_eq!(u7::new(100).ilog10(), 2);
///
/// // 2^199 needs every bit of 200, and twice it needs 201.
/// type U200x4 = Uint<U200, [u64; 4]>;
/// let top = U200x4::checked_from_limbs([0, 0, 0, 1 << 7]).unwrap();
/// let two = U200x4::ONE + U200x4::ONE;
/// assert_eq!(top.checked_mul(&two), None);
/// assert_eq!(top.overflowing_mul(&two), (U200x4::ZERO, true));
/// assert_eq!(top / &two, top.wrapping_div(&two));
/// assert_eq!(top.checked_ilog2(), Some(199));
///
/// // The bits of 7 bits: the counts, the rotations and the shifts stay within them.
/// assert_eq!(u7::MAX.count_ones(), 7);
/// assert_eq!(u7::ONE.rotate_right(1), u7::new(64));
/// assert_eq!(u7::new(3) << 5u32, u7::new(96));
/// assert_eq!(u7::new(3).checked_shl(7), None);
/// assert_eq!(u7::new(3).shl_exact(6), None);
/// // SAFETY: 5 is below 7 bits, so `checked_shl` would return `Some`.
/// assert_eq!(unsafe { u7::new(3).unchecked_shl(5) }, u7::new(96));
/// assert_eq!(u7::MAX.cast_signed(), i7::new(-1));
/// assert_eq!(top.trailing_zeros(), 199);
/// const LOW: u7 = u7::MAX.bitand(u7::new(0b11));
/// assert_eq!(LOW, u7::new(3));
/// ```
///
/// # Text
///
/// [`from_str_radix`](Self::from_str_radix), a `const fn` of each storage, and `FromStr`, which
/// reads decimal digits, parse the text of the native `from_str_radix` at the width `Bits`: an
/// optional `+`, then one or more ASCII digits and letters of either case below the radix, from 2
/// to 36, and nothing else, so no whitespace, `_` or prefix such as `0x`. They return the error of
/// the native integers, [`core::num::ParseIntError`], whose
/// [`kind`](core::num::ParseIntError::kind) is `Empty` for an empty string, `InvalidDigit` for a
/// byte that is not a digit of the radix, a lone sign or `-`, which is no sign of `Uint`, and
/// `PosOverflow` for digits above `MAX`. They read from the left, as the native parser does: a byte
/// that is not a digit is `InvalidDigit`, unless the digits before it already exceed `MAX`, which
/// is `PosOverflow`. The range is that of `Bits`, not of the storage, so `"200"` overflows
/// [`u7`](crate::u7) although its `u8` holds 200, and zero bits read every spelling of zero, such
/// as `"0"`, `"+0"` and `"00"`, and nothing else. A radix outside 2 to 36 panics in every build,
/// reporting the caller. In a constant, a width above the default storages, or above about 16384
/// bits in `u8` limbs, may need `#[allow(long_running_const_eval)]` on the constant.
///
/// `Display` and `Debug` write decimal, `Debug` also under `{:x?}` and `{:X?}`, whose flags the
/// standard library does not expose. The radix formats `Binary`, `Octal`, `LowerHex` and
/// `UpperHex` write the digits of the value, and the exponent formats `LowerExp` and `UpperExp` its
/// scientific notation, both with every flag as the native integers have it: in particular, the
/// precision of `{:e}` rounds the digits after the first half to even, which
/// [`Decimal`](crate::Decimal) never does. Nothing allocates: a native storage writes as its native
/// integer does, and an array writes its radix digits through a buffer of 64 bytes and its decimal
/// digits through the buffer of `Display`.
///
/// ```
/// use core::num::IntErrorKind;
///
/// use numo::{
///   Uint,
///   typenum::{U12, U200},
///   u7,
/// };
///
/// assert_eq!(u7::from_str_radix("7f", 16), Ok(u7::MAX));
/// assert_eq!("+100".parse::<u7>(), Ok(u7::new(100)));
/// // 200 needs 8 bits, which the digits before the byte that is not a digit decide first.
/// let kind = |text: &str| *u7::from_str_radix(text, 10).unwrap_err().kind();
/// assert_eq!(kind("200x"), IntErrorKind::PosOverflow);
/// assert_eq!(kind("26x"), IntErrorKind::InvalidDigit);
/// assert_eq!(kind("1_0"), IntErrorKind::InvalidDigit);
/// assert_eq!(kind(""), IntErrorKind::Empty);
/// const HEX: u7 = match u7::from_str_radix("64", 16) {
///   Ok(value) => value,
///   Err(_) => panic!("100 has 7 bits"),
/// };
/// assert_eq!(HEX, u7::new(100));
///
/// assert_eq!(format!("{:#x} {:o} {:010b}", HEX, HEX, u7::new(5)), "0x64 144 0000000101");
/// assert_eq!(format!("{:e} {:.0e}", HEX, u7::new(125)), "1e2 1e2");
/// // The precision rounds half to even, also in an array of limbs.
/// assert_eq!(format!("{:.1e}", Uint::<U12>::new(1250)), "1.2e3");
/// assert_eq!(format!("{:.1e}", Uint::<U12>::new(1350)), "1.4e3");
/// type U200x4 = Uint<U200, [u64; 4]>;
/// let max = U200x4::MAX;
/// assert_eq!(format!("{max:x}"), "ff".repeat(25));
/// assert_eq!(U200x4::from_str_radix(&format!("{max:x}"), 16), Ok(max));
/// assert_eq!(format!("{max:.3e}"), "1.607e60");
/// assert_eq!(format!("{:.1e}", U200x4::from_str_radix("1250", 10)?), "1.2e3");
/// # Ok::<(), core::num::ParseIntError>(())
/// ```
///
/// # Bytes
///
/// The bytes of a value are the low [`BYTES`](Self::BYTES), `ceil(Bits / 8)`, bytes of the
/// storage: least significant first in [`to_le_bytes`](Self::to_le_bytes), most significant first
/// in [`to_be_bytes`](Self::to_be_bytes), and in the order of the target in
/// [`to_ne_bytes`](Self::to_ne_bytes). An array of bytes has its length in its type,
/// `[u8; BYTES]`, which is inferred, so that a wrong length fails to compile, and exists up to 8192
/// bits. At a width that is not a whole number of bytes, the bits of the last byte above the width
/// are zeros: [`checked_from_le_bytes`](Self::checked_from_le_bytes) and the other orders read an
/// array back and return `None` if one of them is set, while at a width of whole bytes, a
/// [`WholeBytes`](crate::WholeBytes), [`from_le_bytes`](Self::from_le_bytes) and the other orders
/// read every array. [`checked_from_le_slice`](Self::checked_from_le_slice) and
/// [`checked_from_be_slice`](Self::checked_from_be_slice) read the value of a slice of any length,
/// zero for an empty one, and return `None` if it is above `MAX`, whatever the length;
/// [`wrapping_from_le_slice`](Self::wrapping_from_le_slice) and
/// [`wrapping_from_be_slice`](Self::wrapping_from_be_slice) read it modulo `2^Bits`,
/// [`overflowing_from_le_slice`](Self::overflowing_from_le_slice) and
/// [`overflowing_from_be_slice`](Self::overflowing_from_be_slice) also return whether it is above
/// `MAX`, [`saturating_from_le_slice`](Self::saturating_from_le_slice) and
/// [`saturating_from_be_slice`](Self::saturating_from_be_slice) return `MAX` there, and
/// [`strict_from_le_slice`](Self::strict_from_le_slice) and
/// [`strict_from_be_slice`](Self::strict_from_be_slice) panic there in every build, reporting the
/// caller. In an array, [`checked_from_limbs_slice`](Self::checked_from_limbs_slice),
/// [`wrapping_from_limbs_slice`](Self::wrapping_from_limbs_slice),
/// [`overflowing_from_limbs_slice`](Self::overflowing_from_limbs_slice),
/// [`saturating_from_limbs_slice`](Self::saturating_from_limbs_slice) and
/// [`strict_from_limbs_slice`](Self::strict_from_limbs_slice) read a slice of limbs of any length,
/// least significant first, in the same way: its value is that of the bytes of its limbs.
/// [`copy_le_bytes_to`](Self::copy_le_bytes_to) and [`copy_be_bytes_to`](Self::copy_be_bytes_to)
/// write the bytes to the start of a buffer at any width, also one above 8192 bits, and panic if it
/// is shorter, where their `checked_*` forms return `None`, and [`byte`](Self::byte) reads the
/// byte at an index and panics from `BYTES` up. The methods that read a value borrow it, as every
/// `to_*` method does, and nothing allocates.
///
/// At a width of whole bytes, [`swap_bytes`](Self::swap_bytes) reverses the bytes, and
/// [`to_be`](Self::to_be), [`to_le`](Self::to_le), [`from_be`](Self::from_be) and
/// [`from_le`](Self::from_le) convert the value to and from an order as the native integers do: on
/// a little-endian target, `to_be` reverses the bytes and `to_le` returns the value. They convert
/// the value, not a layout: an array keeps its limbs least significant first on every target, so
/// on a big-endian target the memory of an array is not its bytes in either order. The same holds
/// for [`to_ne_bytes`](Self::to_ne_bytes), which gives the bytes of the value in the order of the
/// target, not the memory of the storage, and for [`from_ne_bytes`](Self::from_ne_bytes) and
/// [`checked_from_ne_bytes`](Self::checked_from_ne_bytes), which read such bytes as a value. The
/// traits [`IntegerBytes`](crate::IntegerBytes), [`ByteArray`](crate::ByteArray),
/// [`FromByteArray`](crate::FromByteArray) and [`IntegerByteOrder`](crate::IntegerByteOrder) give
/// generic code the methods.
///
/// ```
/// use numo::{
///   Uint,
///   typenum::{U12, U24},
/// };
///
/// let value = Uint::<U24>::new(0x01_0203);
/// assert_eq!(value.to_le_bytes(), [3, 2, 1]);
/// assert_eq!(value.to_be_bytes(), [1, 2, 3]);
/// assert_eq!(Uint::<U24>::from_be_bytes([1, 2, 3]), value);
/// assert_eq!(value.swap_bytes(), Uint::<U24>::new(0x03_0201));
/// assert_eq!(value.byte(2), 1);
///
/// // 12 bits leave four bits of the second byte, which must be zeros.
/// type U12x2 = Uint<U12, [u8; 2]>;
/// assert_eq!(U12x2::MAX.to_le_bytes(), [0xff, 0x0f]);
/// assert_eq!(U12x2::checked_from_le_bytes([0xff, 0x1f]), None);
/// // A slice of any length, whose value decides, and its value modulo 2^12.
/// assert_eq!(U12x2::checked_from_le_slice(&[0xff, 0x0f, 0, 0]), Some(U12x2::MAX));
/// assert_eq!(U12x2::wrapping_from_le_slice(&[0xff, 0xff]), U12x2::MAX);
/// // 4097 passes `MAX`: modulo 2^12 with the overflow, or `MAX`.
/// assert_eq!(U12x2::overflowing_from_be_slice(&[0x10, 0x01]), (U12x2::ONE, true));
/// assert_eq!(U12x2::saturating_from_be_slice(&[0x10, 0x01]), U12x2::MAX);
/// // The same of a slice of the limbs of the array, least significant first.
/// assert_eq!(U12x2::checked_from_limbs_slice(&[0xff, 0x0f, 0]), Some(U12x2::MAX));
/// assert_eq!(U12x2::saturating_from_limbs_slice(&[0x01, 0x10]), U12x2::MAX);
///
/// // A copy to a buffer, which leaves the rest of it.
/// let mut buf = [0; 4];
/// assert_eq!(value.copy_be_bytes_to(&mut buf), 3);
/// assert_eq!(buf, [1, 2, 3, 0]);
/// assert_eq!(value.checked_copy_be_bytes_to(&mut [0; 2]), None);
/// ```
///
/// # Conversions
///
/// The casts convert between integers of any width and sign, as the casts of the native integers
/// do, still unstable ([#157388](https://github.com/rust-lang/rust/issues/157388),
/// [#154330](https://github.com/rust-lang/rust/issues/154330)). The target is a [`Uint`] or an
/// [`Int`] of any width and storage, or a native integer, which method syntax names, as in
/// `x.checked_cast::<u64>()`, or the expected type infers, as in `let y: u256 = x.widen();`.
/// [`checked_cast`](Self::checked_cast) returns `None` for a value outside the range of the target,
/// [`strict_cast`](Self::strict_cast) panics there, the `unsafe`
/// [`unchecked_cast`](Self::unchecked_cast) assumes that the value is in that range,
/// [`wrapping_cast`](Self::wrapping_cast) takes the value modulo `2^Bits` of the target, as `as`
/// casts the native integers, and [`saturating_cast`](Self::saturating_cast) the bound of the range
/// that the value passes.
/// [`widen`](Self::widen) converts into an integer of the same sign that is at least as wide, which
/// the compiler checks, so it never fails, and [`truncate`](Self::truncate),
/// [`checked_truncate`](Self::checked_truncate) and
/// [`saturating_truncate`](Self::saturating_truncate) into one of the same sign that is at most as
/// wide. The traits of the target, [`CheckedCastFrom`](crate::CheckedCastFrom),
/// [`BoundedCastFrom`](crate::BoundedCastFrom), [`WidenFrom`](crate::WidenFrom) and
/// [`TruncateFrom`](crate::TruncateFrom), give generic code the casts and convert from a native
/// integer, as in `u7::checked_cast_from(5u64)`; their documentation lists the pairs. The casts
/// that check, [`checked_cast`](Self::checked_cast) and [`strict_cast`](Self::strict_cast), and
/// `unchecked_cast` also take a non-zero target, a [`NonZeroUint`], a [`NonZeroInt`] or a native
/// `NonZero` integer, which has no value of zero: `checked_cast` returns `None` for zero,
/// `strict_cast` panics with the text of the error of the kind `Zero`, and `unchecked_cast`,
/// whose caller guarantees a value that is not zero, checks it in every build and panics there
/// (see [Conversions](NonZeroUint#conversions)).
/// [`checked_cast_signed`](Self::checked_cast_signed),
/// [`saturating_cast_signed`](Self::saturating_cast_signed) and
/// [`strict_cast_signed`](Self::strict_cast_signed) convert into the [`Int`] of the same width in a
/// storage of the same size, whose bits [`cast_signed`](Self::cast_signed) reads, as the native
/// methods do, still unstable ([#154650](https://github.com/rust-lang/rust/issues/154650)). The
/// casts take the value, as conversions do, and only the casts between the signs are `const fn`s:
/// the others are generic over the target, whose trait functions a constant context cannot call.
///
/// The standard conversions with the native integers go by direction. `From` converts a native
/// unsigned integer, `u8` to `u128`, into a `Uint` whose width holds every value of it, and
/// `TryFrom` converts a `usize` or an `isize` at every width. `TryFrom` converts a `Uint` of any
/// width into each native integer, with the error of the native conversions,
/// [`TryFromIntError`](core::num::TryFromIntError), whose kind is `PosOverflow` above the range of
/// the target and `NegOverflow` below it. No signed integer converts into a `Uint` by `From`, and a
/// literal without a suffix is an `i32`: `u256::from(5)` fails to compile, while `u256::from(5u64)`
/// compiles. `From<bool>` converts `false` and `true` to zero and one, from one bit, and `TryFrom`
/// into `bool` converts zero and one back. Between numo integers, the casts are the conversions:
/// `From` and `TryFrom` there would overlap the standard implementations from a type to itself.
///
/// The values compare with the native integers, both ways: `PartialEq` and `PartialOrd` with each
/// of the twelve native integers compare them as integers, whatever the widths and the signs, so
/// that `u7::new(5) == 5i8`, `u256::MAX > u128::MAX` and `300u16 > u7::MAX`, and a literal without
/// a suffix on either side, as in `x == 5` and `5 == x`, compares as an `i32`. With them in a
/// build, the compiler no longer infers the type of an expression compared with a native integer
/// from that integer, as in `n == m.into()`, `n == x.try_into().unwrap()`,
/// `n == s.parse().unwrap()`, `n == v.iter().sum()`, `n == Default::default()` or a generic call in
/// that place: such a comparison names the type, as in `n == u64::from(m)` or
/// `n == s.parse::<u64>().unwrap()`, and so does one with a numo integer. Integers of two numo
/// types do not compare: a cast converts one to the type of the other, as in
/// `a.widen::<T>() == b`.
///
/// ```
/// use numo::{
///   CheckedCastFrom, NonZeroUint, Uint, i7,
///   typenum::{U16, U200},
///   u7, u256,
/// };
///
/// let x = u7::new(100);
/// assert_eq!(x.checked_cast::<i7>(), None);
/// assert_eq!(x.wrapping_cast::<i7>(), i7::new(-28));
/// assert_eq!(x.saturating_cast::<i7>(), i7::MAX);
/// assert_eq!(x.checked_cast::<u64>(), Some(100));
/// assert_eq!(u7::ZERO.checked_cast::<NonZeroUint<U16>>(), None);
/// let wide: u256 = x.widen();
/// assert_eq!(wide.truncate::<u8>(), 100);
/// assert_eq!(Uint::<U200>::checked_cast_from(-1i8), None);
/// assert_eq!(u7::checked_cast_from(300u16), None);
/// assert_eq!(u7::MAX.checked_cast_signed(), None);
/// assert_eq!(u7::MAX.saturating_cast_signed(), i7::MAX);
///
/// // The standard conversions, by direction.
/// assert_eq!(u256::from(5u64), u256::from_limbs([5, 0, 0, 0]));
/// let n: u64 = wide.try_into()?;
/// assert_eq!(n, 100);
/// let error = u64::try_from(u256::MAX).unwrap_err();
/// assert_eq!(error.to_string(), "number too large to fit in target type");
/// assert!(Uint::<U16>::try_from(70_000usize).is_err());
/// assert_eq!(u7::from(true), u7::ONE);
///
/// // The comparisons with the native integers, both ways.
/// assert!(x < 128u8 && 300u16 > x && x == 100);
/// assert!(u256::MAX > u128::MAX);
/// # Ok::<(), core::num::TryFromIntError>(())
/// ```
///
/// # Overflow
///
/// The `checked_*`, `wrapping_*`, `overflowing_*`, `saturating_*` and `strict_*` methods behave the
/// same in every build. The operators `+`, `-` and `*`, [`pow`](Self::pow),
/// [`next_multiple_of`](Self::next_multiple_of) and [`next_power_of_two`](Self::next_power_of_two)
/// forward the overflow to a native operation: the last two to an addition, which is how the native
/// methods overflow, and `pow` to an exponentiation. With one profile for the build, numo's
/// `overflow-checks` and yours agree and they behave as the native integers do. With a per-package
/// override, which setting applies depends on where the code is generated: an unoptimized build
/// follows numo's setting; an optimized build panics only when both numo's and your crate's checks
/// are on. `pow` follows your crate's setting in an unoptimized build too, as the native `pow`
/// does, since that is the operation it repeats, and in a constant an overflowing `pow` fails to
/// compile in every build. At zero bits, which hold no one, `pow(0)` and the next power of two of
/// zero overflow: `checked_pow(0)` returns `None` there.
///
/// The operators `<<` and `>>` overflow on an amount that is `Bits` or more, or negative, of any
/// primitive integer type, and forward the overflow to a native shift, so that with overflow
/// checks they panic with its message and without them they shift by the amount modulo `Bits`,
/// taken as a mathematical integer: at 7 bits, an amount of 7 shifts by 0 places and an amount of
/// -1 by 6. At a width that is a power of two, this is the mask that the native operators apply.
/// The amount is bounded by `Bits`, not by the storage, so `u7 << 7` overflows although the `u8`
/// holds 8 bits, and at zero bits every amount overflows, so `u0 << 0` panics with overflow checks.
/// A panicking `<<=` or `>>=` leaves its target unchanged, and the bitwise operators cannot
/// overflow.
///
/// A division or a remainder by zero, in the methods and in the operators `/` and `%`, a logarithm
/// of zero or to a base below 2, and a root of degree zero panic in every build, as for the native
/// integers. A bit index of `Bits` or more panics in [`set_bit`](Self::set_bit), in every build.
/// [`div_exact`](Self::div_exact) panics on a zero divisor with the message of the remainder: the
/// native `div_exact` is unstable, and numo follows what it does, which computes the remainder
/// first, rather than its documentation. Every panic reports the caller, where the native methods
/// sometimes report a line in `core`, except that of `pow`, which reports the line in `core` that
/// the native `pow` reports.
#[repr(transparent)]
pub struct Uint<Bits, Repr = <Bits as BitWidth>::Unsigned> {
  // Holds a value of `Bits` bits: every bit of the storage at index `Bits` or above is zero, so the
  // storage, read as an unsigned integer, equals the value. Every constructor and every arithmetic
  // result establishes this, and equality, comparison, hashing, formatting and the arithmetic rely
  // on it. It is a logical invariant, not a safety invariant: no `unsafe` code may rely on it. The
  // `unchecked_*` methods assume only their caller's guarantee, which is stated through
  // `checked_*`, so they do not rely on it either.
  repr: Repr,
  bits: PhantomData<Bits>,
}

/// A signed integer of `Bits` bits in two's complement, stored in `Repr`.
///
/// The value runs from `-2^(Bits - 1)` to `2^(Bits - 1) - 1`, and is zero for zero bits. `Bits` is
/// a [`typenum`] number, and `Repr` holds at least `Bits` bits: a native signed integer, `i8`,
/// `i16`, `i32`, `i64` or `i128`, which holds the value itself, or an array of limbs, least
/// significant first, as for [`Uint`]: `[u8; N]`, `[u16; N]`, `[u32; N]`, `[u64; N]` or
/// `[u128; N]`. The bits above `Bits`, if any, are copies of the sign bit, so the storage holds the
/// value in two's complement. Without `Repr`, the storage is the default that [`BitWidth`] gives,
/// as for [`Uint`]: `Int<U24>` is `Int<U24, i32>` and `Int<U200>` is `Int<U200, [u64; 4]>`. The
/// module [`aliases`](crate::aliases) names common widths, such as [`i7`](crate::i7) for
/// `Int<U7, i8>` and [`i256`](crate::i256) for `Int<U256, [u64; 4]>`. The type has the size,
/// alignment and ABI of `Repr`, and as for [`Uint`], each storage gives a distinct type.
///
/// As for [`Uint`], in a native integer [`new`](Self::new) and [`try_new`](Self::try_new) create a
/// value and [`value`](Self::value) returns it, and in an array [`from_limbs`](Self::from_limbs)
/// creates a value from any bit pattern of a width that fills its limbs, and
/// [`checked_from_limbs`](Self::checked_from_limbs) from the limbs of any width, checking that
/// they hold a value of `Bits` bits. Values compare, hash and format by their numeric value.
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
/// `i7::new(-5)`, `i256::from_limbs(limbs)`, `Int::<U7>::new(-5)`,
/// `Int::<U200, [u64; 4]>::checked_from_limbs(limbs)` or `i7::from_str_radix("-40", 16)`.
///
/// # Arithmetic
///
/// Each storage has the arithmetic of the native signed integers at the width `Bits`, as
/// `const fn`s: addition, subtraction, multiplication, negation, division, remainder and
/// exponentiation in the `checked_*`, `wrapping_*`, `overflowing_*` and `strict_*` forms, the
/// Euclidean division and remainder in the same four forms, the saturating addition, subtraction,
/// multiplication, negation, division and exponentiation, the absolute value in every form,
/// [`abs`](Self::abs) and [`unsigned_abs`](Self::unsigned_abs), which returns the [`Uint`] of the
/// same width and storage size, [`signum`](Self::signum), and from two bits the constant `ONE`,
/// since one bit holds only -1 and 0. The functions without an overflow prefix are those of
/// [`Uint`], rounded toward zero where `Uint` rounds down, but for `is_multiple_of` and the next
/// power of two, and with [`checked_isqrt`](Self::checked_isqrt):
/// [`abs_diff`](Self::abs_diff) returns the [`Uint`] of the same width and storage size, and
/// [`nth_root`](Self::nth_root) takes a negative value for an odd degree. As for [`Uint`],
/// addition, subtraction, multiplication, negation, exact division and the shifts also have the
/// `unchecked_*` form, as `const unsafe fn`s whose caller guarantees that the `checked_*` method
/// would return `Some`: [`unchecked_neg`](Self::unchecked_neg), for example, requires that `self`
/// is not `MIN`, except at zero bits, and [`unchecked_div_exact`](Self::unchecked_div_exact) that
/// `rhs` is not zero and divides `self`, and that `self` is not `MIN` with `rhs` -1. The operators
/// `+`, `-`, `*`, `/` and `%`, their compound assignments, the negation `-`, `Sum` and `Product`
/// work for every storage, and so do the traits that give generic code the methods, such as
/// [`CheckedOps`](crate::CheckedOps) and [`IntegerMath`](crate::IntegerMath). As for [`Uint`], a
/// path to a method names the storage.
///
/// The bit operations are those of [`Uint`] but for `bit_width` and `is_power_of_two`, on the
/// `Bits` bits of the two's complement: the counts leave out the copies of the sign that the
/// storage holds above them, so `i7::new(-1).count_ones()` is 7, the right shifts are arithmetic
/// and copy the sign, and the rotations and [`reverse_bits`](Self::reverse_bits) move the sign bit
/// like any other, so they can change the sign. [`set_bit`](Self::set_bit) at bit `Bits - 1` sets
/// or clears the sign, and [`cast_unsigned`](Self::cast_unsigned) reads the bits as the [`Uint`] of
/// the same width in a storage of the same size. The traits [`IntegerBits`](crate::IntegerBits) and
/// [`SignedBits`](crate::SignedBits) give generic code the bit methods, and the operators and the
/// bitwise `const fn`s are those of [`Uint`].
///
/// ```
/// use numo::{i7, u7};
///
/// assert_eq!(i7::MIN.checked_neg(), None);
/// assert_eq!(i7::MIN.wrapping_abs(), i7::MIN);
/// assert_eq!(i7::MIN.unsigned_abs().value(), 64);
/// assert_eq!(i7::new(-5).signum(), -i7::ONE);
/// assert_eq!(i7::MIN.saturating_sub(&i7::ONE), i7::MIN);
/// assert_eq!(-i7::new(63) - i7::ONE, i7::MIN);
/// // SAFETY: -63 is not `MIN`, so `checked_neg` would return `Some`.
/// assert_eq!(unsafe { i7::new(-63).unchecked_neg() }, i7::MAX);
///
/// // Division rounds toward zero, and the Euclidean remainder is never negative.
/// assert_eq!(i7::new(-7) / i7::new(2), i7::new(-3));
/// assert_eq!(i7::new(-7).div_floor(&i7::new(2)), i7::new(-4));
/// assert_eq!(i7::new(-7).rem_euclid(&i7::new(2)), i7::ONE);
/// assert_eq!(i7::MIN.checked_div(&-i7::ONE), None);
/// assert_eq!(i7::MIN.abs_diff(&i7::MAX), u7::MAX);
/// assert_eq!(i7::new(-27).nth_root(3), i7::new(-3));
///
/// // The bits of the two's complement of 7 bits.
/// assert_eq!(i7::new(-1).count_ones(), 7);
/// assert_eq!(i7::MIN >> 3u32, i7::new(-8));
/// // SAFETY: 6 is below 7 bits, so `checked_shr` would return `Some`.
/// assert_eq!(unsafe { i7::MIN.unchecked_shr(6) }, -i7::ONE);
/// assert_eq!(i7::new(-2).cast_unsigned(), u7::new(126));
/// assert_eq!(i7::new(1).rotate_right(1), i7::MIN);
/// let mut value = i7::ZERO;
/// value.set_bit(6, true);
/// assert_eq!(value, i7::MIN);
/// ```
///
/// # Text
///
/// The text is that of [`Uint`], with `-` as well as `+` before the digits, and `NegOverflow` for
/// digits after `-` below `MIN`; `-0` is zero, also at zero bits. The radix formats write the two's
/// complement of the value in its `Bits` bits, as the native integers write the bits of their
/// width: -1 of 7 bits writes `1111111` and `7f`, whatever the storage holds above the width, so
/// `Int<U7, i8>` and `Int<U7, [u64; 1]>` write the same text. That text reads back as the [`Uint`]
/// of the same width, through [`cast_unsigned`](Self::cast_unsigned); `from_str_radix` of `Int`
/// reads `-` and the digits of the magnitude. The exponent formats write `-` before a negative
/// value.
///
/// ```
/// use core::num::IntErrorKind;
///
/// use numo::{Int, i7, typenum::U7, u7};
///
/// assert_eq!(i7::from_str_radix("-40", 16), Ok(i7::MIN));
/// assert_eq!("-0".parse::<i7>(), Ok(i7::ZERO));
/// assert_eq!(*"-65".parse::<i7>().unwrap_err().kind(), IntErrorKind::NegOverflow);
///
/// // The two's complement of 7 bits, in every storage.
/// let minus_one = i7::new(-1);
/// assert_eq!(format!("{minus_one:b} {minus_one:#x}"), "1111111 0x7f");
/// assert_eq!(format!("{:x}", Int::<U7, [u64; 1]>::MIN), "40");
/// assert_eq!(u7::from_str_radix(&format!("{minus_one:x}"), 16), Ok(minus_one.cast_unsigned()));
/// assert_eq!(format!("{:e} {:.0e}", i7::MIN, i7::new(-35)), "-6.4e1 -4e1");
/// ```
///
/// # Bytes
///
/// The bytes are those of [`Uint`] (see [Bytes](Uint#bytes)), in two's complement: at a width that
/// is not a whole number of bytes, the bits of the last byte above the width copy the sign, as the
/// storage holds them, so that the bytes, read as an integer of `8 * BYTES` bits in two's
/// complement, give the value, and [`checked_from_le_bytes`](Self::checked_from_le_bytes) returns
/// `None` unless they do; [`cast_unsigned`](Self::cast_unsigned) gives the bytes of the `Bits` bits
/// with zeros above them. [`checked_from_le_slice`](Self::checked_from_le_slice) reads a slice as
/// an integer in two's complement of `8 * len` bits, whose sign is the top bit of its most
/// significant byte, and returns `None` if that integer is outside the range of `Bits` bits;
/// [`wrapping_from_le_slice`](Self::wrapping_from_le_slice) reduces it modulo `2^Bits` into the
/// range, so that it returns the value of `checked_from_le_slice` wherever that returns one,
/// [`overflowing_from_le_slice`](Self::overflowing_from_le_slice) also returns whether the integer
/// is outside the range, [`saturating_from_le_slice`](Self::saturating_from_le_slice) returns
/// [`MIN`](Self::MIN) below it, for a negative slice, and [`MAX`](Self::MAX) above it, and
/// [`strict_from_le_slice`](Self::strict_from_le_slice) panics outside it. The slices of limbs of
/// an array, such as [`checked_from_limbs_slice`](Self::checked_from_limbs_slice), read the limbs
/// in two's complement in the same way, the sign being the top bit of the last limb. The byte order
/// is that of [`Uint`], and [`swap_bytes`](Self::swap_bytes) reads the reversed bytes again in
/// two's complement.
///
/// ```
/// use numo::{
///   Int, i7,
///   typenum::{U12, U24},
/// };
///
/// assert_eq!(i7::new(-1).to_le_bytes(), [0xff]);
/// assert_eq!(Int::<U24>::new(-2).to_be_bytes(), [0xff, 0xff, 0xfe]);
/// assert_eq!(Int::<U24>::new(-2).swap_bytes(), Int::<U24>::new(-0x01_0001));
///
/// // 12 bits in two bytes, whose four bits above the width copy the sign.
/// type I12 = Int<U12>;
/// assert_eq!(I12::MIN.to_le_bytes(), [0x00, 0xf8]);
/// assert_eq!(I12::checked_from_le_bytes([0x00, 0x08]), None);
/// // A short slice keeps its sign, and the wrapping form reduces a value into the range.
/// assert_eq!(I12::checked_from_le_slice(&[0x80]), Some(I12::new(-128)));
/// assert_eq!(I12::wrapping_from_le_slice(&[0x80]), I12::new(-128));
/// assert_eq!(I12::checked_from_le_slice(&[0xff, 0x0f]), None);
/// assert_eq!(I12::wrapping_from_le_slice(&[0xff, 0x0f]), I12::new(-1));
/// // 4095 passes `MAX`, and -32768, negative by the top bit of its last byte, passes `MIN`.
/// assert_eq!(I12::overflowing_from_le_slice(&[0xff, 0x0f]), (I12::new(-1), true));
/// assert_eq!(I12::saturating_from_le_slice(&[0x00, 0x80]), I12::MIN);
/// // The limbs of an array, in two's complement.
/// type I12x2 = Int<U12, [u8; 2]>;
/// assert_eq!(I12x2::checked_from_limbs_slice(&[0x80]), I12x2::checked_from_limbs([0x80, 0xff]));
/// ```
///
/// # Conversions
///
/// The casts are those of [`Uint`] (see [Conversions](Uint#conversions)), which convert a negative
/// value too: [`checked_cast`](Self::checked_cast) returns `None` for it into an unsigned target,
/// [`wrapping_cast`](Self::wrapping_cast) takes its two's complement modulo `2^Bits` of the target,
/// and [`saturating_cast`](Self::saturating_cast) gives the `MIN` of the target, zero for an
/// unsigned one. [`checked_cast_unsigned`](Self::checked_cast_unsigned),
/// [`saturating_cast_unsigned`](Self::saturating_cast_unsigned) and
/// [`strict_cast_unsigned`](Self::strict_cast_unsigned) convert into the [`Uint`] of the same width
/// in a storage of the same size, whose bits [`cast_unsigned`](Self::cast_unsigned) reads.
///
/// `From` converts a native signed integer, `i8` to `i128`, into an `Int` whose width holds every
/// value of it, and a native unsigned integer, `u8` to `u128`, into one with a bit more for the
/// sign, so that a literal without a suffix, an `i32`, converts: `i256::from(5)` compiles, where
/// `u256::from(5)` does not. `TryFrom` converts a `usize` or an `isize` at every width, and an `Int`
/// of any width into each native integer, as for [`Uint`], and `From<bool>` converts from two bits,
/// as `ONE` exists. The comparisons with the native integers are those of [`Uint`], with the same
/// cost to inference: a negative value is below every unsigned integer, as in
/// `Int::<U7>::new(-1) < 0u8`.
///
/// ```
/// use numo::{Int, i7, i256, typenum::U12, u7};
///
/// let x = i7::new(-5);
/// assert_eq!(x.checked_cast::<u8>(), None);
/// assert_eq!(x.wrapping_cast::<u8>(), 251);
/// assert_eq!(x.saturating_cast::<u8>(), 0);
/// assert_eq!(x.widen::<i64>(), -5);
/// assert_eq!(Int::<U12>::new(-2000).truncate::<i8>(), 48);
/// assert_eq!(x.checked_cast_unsigned(), None);
/// assert_eq!(i7::MAX.checked_cast_unsigned(), Some(u7::new(63)));
///
/// assert_eq!(i256::from(5), i256::from(5i8));
/// let error = i8::try_from(i256::MIN).unwrap_err();
/// assert_eq!(error.to_string(), "number too small to fit in target type");
///
/// // The comparisons with the native integers, both ways.
/// assert!(x < 0u8 && -5i64 == x && i256::MIN < i128::MIN);
/// ```
///
/// # Overflow
///
/// The `checked_*`, `wrapping_*`, `overflowing_*`, `saturating_*` and `strict_*` methods behave the
/// same in every build. The operators `+`, `-` and `*`, the negation `-`, [`pow`](Self::pow) and
/// [`next_multiple_of`](Self::next_multiple_of) forward the overflow to a native operation, as for
/// [`Uint`]. With one profile for the build, numo's `overflow-checks` and yours agree and they
/// behave as the native integers do. With a per-package override, which setting applies depends on
/// where the code is generated: an unoptimized build follows numo's setting; an optimized build
/// panics only when both numo's and your crate's checks are on; `pow` follows your crate's setting,
/// and fails to compile in a constant, as for [`Uint`]. [`abs`](Self::abs) handles `MIN` in the
/// same way, as the native `abs` does; in a constant, where `abs` is the only method other than
/// `pow` that forwards an overflow, numo's setting applies and the error names the native value
/// (`i8::MIN`). One bit holds -1 and 0 and no one, so `MIN.pow(0)`, which is one, overflows to -1:
/// it panics with overflow checks and returns -1 without them, `saturating_pow(0)` returns `MAX`,
/// which is zero, and `checked_pow(0)` returns `None`, while `MIN.pow(3)` is -1; zero bits overflow
/// on `pow(0)` as for [`Uint`]. The operators `<<` and `>>` overflow as for [`Uint`], on an amount
/// that is `Bits` or more or negative, and every amount overflows zero bits: `i1 >> 1` overflows
/// one bit, and without overflow checks shifts by nothing.
///
/// As for [`Uint`], a division or a remainder by zero, a logarithm of a value that is not positive
/// or to a base below 2, and a root of degree zero panic in every build, and so do `MIN / -1` and
/// `MIN % -1` in the strict methods, the Euclidean and rounded division and the operators `/` and
/// `%`, the square root and a root of even degree of a negative value, as for the native integers.
/// [`div_exact`](Self::div_exact) panics on a zero divisor and on `MIN / -1` with the messages of
/// the remainder, as the unstable native method does, and [`set_bit`](Self::set_bit) on an index of
/// `Bits` or more. Every panic reports the caller, but that of `pow`.
#[repr(transparent)]
pub struct Int<Bits, Repr = <Bits as BitWidth>::Signed> {
  // Holds a value of `Bits` bits: every bit of the storage at index `Bits - 1` or above equals bit
  // `Bits - 1`, the sign bit, so the storage, read as a two's complement integer, equals the value.
  // Zero bits have no sign bit, and every bit of the storage is zero. Every constructor and every
  // arithmetic result establishes this, and equality, comparison, the sign queries, hashing,
  // formatting and the arithmetic rely on it. It is a logical invariant, not a safety invariant: no
  // `unsafe` code may rely on it. The `unchecked_*` methods assume only their caller's guarantee,
  // which is stated through `checked_*`, so they do not rely on it either.
  repr: Repr,
  bits: PhantomData<Bits>,
}

/// A non-zero unsigned integer of `Bits` bits, stored in `Repr`.
///
/// The value runs from one to `2^Bits - 1`. `Repr` is the storage of a value that is not zero:
/// `core::num::NonZero<u8>`, `NonZero<u16>`, `NonZero<u32>`, `NonZero<u64>` or `NonZero<u128>`,
/// whose integer holds at least `Bits` bits, or an array of limbs, least significant first,
/// `[u8; N]`, `[u16; N]`, `[u32; N]`, `[u64; N]` or `[u128; N]`, as for [`Uint`]. Each is the
/// [`NonZero`](crate::UintRepr::NonZero) of a storage of [`Uint`], and without `Repr` the storage
/// is that of the default storage of [`Uint`], which [`BitWidth`] names: `NonZeroUint<U7>` is
/// `NonZeroUint<U7, NonZero<u8>>` and `NonZeroUint<U200>` is `NonZeroUint<U200, [u64; 4]>`.
/// [`NonZeroOf`](crate::NonZeroOf) names the non-zero integer of an integer, such as
/// `NonZeroOf<u7>` for `NonZeroUint<U7>`.
///
/// The type has the size, alignment and ABI of `Repr`. In a native storage, `Option` of the type has
/// the size, alignment and call ABI of the native integer, and `None` is the bit pattern of zero,
/// as for the native `NonZero`: `Option<NonZeroUint<U7>>` takes one byte. An array has no value
/// that it cannot hold, so `Option` of an array storage adds a tag. Each storage gives a distinct
/// type, as for [`Uint`].
///
/// In each storage, [`new`](Self::new) creates a value from the [`Uint`] of the same width and
/// storage, returning `None` for zero, and [`get`](Self::get) returns it, as `From` does.
/// [`new_unchecked`](Self::new_unchecked), a `const unsafe fn`, creates a value without the check:
/// the caller guarantees that it is not zero. The constants are `BITS`, and from one bit `MIN`,
/// which is one, and `MAX`, which is `2^Bits - 1`. Zero bits hold no value but zero, so `new`
/// returns `None` there and the type has no `MIN` or `MAX`; one bit holds only one. Values compare,
/// hash and format as the integers of the same values: `Ord` is numeric, the hash is that of the
/// [`Uint`], `Display` and `Debug` write decimal, and the radix and exponent formats write what the
/// integer writes. There is no `Default`, as no value would be a natural one.
///
/// [`from_str_radix`](Self::from_str_radix), a `const fn` of each storage, and `FromStr` parse a
/// value as the native `NonZero` integers do: as the [`Uint`] of the same width and storage parses
/// it (see [Text](Uint#text)), then the error of the kind `Zero` for zero, after every error of the
/// integer, so that at zero bits every text is an error.
///
/// At a width of whole bytes, [`swap_bytes`](Self::swap_bytes), [`to_be`](Self::to_be),
/// [`to_le`](Self::to_le), [`from_be`](Self::from_be) and [`from_le`](Self::from_le) give the byte
/// order of the [`Uint`] of the same value (see [Bytes](Uint#bytes)), which is not zero, and follow
/// the native methods, still unstable ([#128281](https://github.com/rust-lang/rust/issues/128281)).
/// As for the integers, they convert the value, not a layout, for an array storage.
/// [`IntegerByteOrder`](crate::IntegerByteOrder) gives them to generic code.
///
/// ```
/// use core::num::NonZero;
///
/// use numo::{
///   NonZeroUint, Uint,
///   typenum::{U7, U200},
///   u7,
/// };
///
/// let five = NonZeroUint::<U7>::new(u7::new(5)).unwrap();
/// assert_eq!(five.get(), u7::new(5));
/// assert_eq!(NonZeroUint::<U7>::new(u7::ZERO), None);
/// // SAFETY: 5 is not zero.
/// assert_eq!(unsafe { NonZeroUint::<U7>::new_unchecked(u7::new(5)) }, five);
/// assert_eq!(NonZeroUint::<U7>::MAX.get(), u7::MAX);
/// assert!(NonZeroUint::<U7>::MIN < five);
///
/// // The storage is the native `NonZero` of the storage of `u7`, a `u8`, and `None` takes its niche.
/// let _: NonZeroUint<U7, NonZero<u8>> = five;
/// assert_eq!(size_of::<Option<NonZeroUint<U7>>>(), 1);
///
/// // 200 bits in four limbs.
/// let wide = NonZeroUint::<U200>::new(Uint::<U200>::MAX).unwrap();
/// assert_eq!(wide.count_ones().get(), 200);
/// assert_eq!(Uint::from(wide), Uint::<U200>::MAX);
///
/// // The text, in which zero is an error.
/// assert_eq!(NonZeroUint::<U7>::from_str_radix("5", 10), Ok(five));
/// assert_eq!(format!("{five:#b} {five:e}"), "0b101 5e0");
/// let zero = "00".parse::<NonZeroUint<U7>>().unwrap_err();
/// assert_eq!(*zero.kind(), core::num::IntErrorKind::Zero);
/// assert_eq!(zero, "0".parse::<NonZero<u8>>().unwrap_err());
/// ```
///
/// # Arithmetic
///
/// Each storage has the methods of the native non-zero integers at the width `Bits`, as
/// `const fn`s: [`leading_zeros`](Self::leading_zeros), [`trailing_zeros`](Self::trailing_zeros)
/// and [`count_ones`](Self::count_ones), which returns a `core::num::NonZero<u32>`,
/// [`highest_one`](Self::highest_one) and [`lowest_one`](Self::lowest_one) and their isolation,
/// [`bit_width`](Self::bit_width), [`is_power_of_two`](Self::is_power_of_two),
/// [`reverse_bits`](Self::reverse_bits), [`rotate_left`](Self::rotate_left) and
/// [`rotate_right`](Self::rotate_right), which follow the native methods, still unstable
/// ([#128281](https://github.com/rust-lang/rust/issues/128281)), the checked and saturating
/// multiplication and exponentiation, the checked and saturating addition of a [`Uint`], which may
/// be zero, [`checked_next_power_of_two`](Self::checked_next_power_of_two), the logarithms
/// [`ilog2`](Self::ilog2) and [`ilog10`](Self::ilog10), [`midpoint`](Self::midpoint),
/// [`isqrt`](Self::isqrt), [`div_ceil`](Self::div_ceil), and [`cast_signed`](Self::cast_signed),
/// which reads the bits as the [`NonZeroInt`] of the same width in a storage of the same size. A
/// saturating method returns `MAX` where the result passes it. Every count and index stays within
/// the `Bits` bits, whatever the storage holds, and no method panics: a value that is not zero has
/// a logarithm, and [`div_ceil`](Self::div_ceil) divides by a value that is not zero.
///
/// The division `Uint / NonZeroUint` and the remainder `Uint % NonZeroUint`, with `/=` and `%=`,
/// divide by a value that is not zero, so they cannot panic, as for the native integers. `|`
/// combines two non-zero integers, or one and its [`Uint`] either way, into a non-zero integer, and
/// `|=` assigns it. The operators work on references for every storage, and on values for a
/// storage whose integer holds the value inline. The traits
/// [`FixedNonZero`](crate::FixedNonZero), [`UnsignedNonZero`](crate::UnsignedNonZero),
/// [`NonZeroLimits`](crate::NonZeroLimits) and [`ConstNonZeroLimits`](crate::ConstNonZeroLimits)
/// give generic code the methods and the limits. As for [`Uint`], a path to a method names the
/// storage, as in `NonZeroUint::<U7>::new(..)`.
///
/// ```
/// use numo::{NonZeroInt, NonZeroUint, i7, typenum::U7, u7};
///
/// let ten = NonZeroUint::<U7>::new(u7::new(10)).unwrap();
/// let twenty = ten.checked_add(&u7::new(10)).unwrap();
/// assert_eq!(ten.checked_add(&u7::ZERO), Some(ten));
/// assert_eq!(ten.checked_mul(&twenty), None);
/// assert_eq!(ten.saturating_mul(&twenty), NonZeroUint::<U7>::MAX);
/// assert_eq!((ten.ilog2(), ten.ilog10(), ten.isqrt().get()), (3, 1, u7::new(3)));
/// assert_eq!(ten.rotate_right(1).get(), u7::new(5));
/// assert_eq!(NonZeroUint::<U7>::MAX.cast_signed(), NonZeroInt::<U7>::new(i7::new(-1)).unwrap());
///
/// // The division by a value that is not zero cannot panic.
/// assert_eq!(u7::new(100) / ten, u7::new(10));
/// assert_eq!(u7::new(105) % ten, u7::new(5));
/// assert_eq!((ten | u7::new(1)).get(), u7::new(11));
/// ```
///
/// # Conversions
///
/// `TryFrom` converts the [`Uint`] of the same width and storage into a non-zero integer, or
/// returns the error of the native conversions of the kind `Zero` for zero,
/// [`TryFromIntError`](core::num::TryFromIntError), as the native `NonZero` integers convert from
/// their integers, and `From` converts it back. With the native `NonZero` integers, the
/// conversions go by the directions of the integers with the native integers (see
/// [Conversions](Uint#conversions)): `From` converts `NonZero<u8>` to `NonZero<u128>` into a
/// `NonZeroUint` whose width holds every value of it, and `TryFrom` converts `NonZero<usize>` and
/// `NonZero<isize>` into any width, and a `NonZeroUint` of any width into the `NonZero` of each
/// native integer, with the errors of the kinds `PosOverflow` and `NegOverflow`.
///
/// The casts of [`Uint`] (see [Conversions](Uint#conversions)) convert a non-zero integer as the
/// integer of the same value: [`checked_cast`](Self::checked_cast),
/// [`wrapping_cast`](Self::wrapping_cast), [`saturating_cast`](Self::saturating_cast),
/// [`strict_cast`](Self::strict_cast), the `unsafe` [`unchecked_cast`](Self::unchecked_cast),
/// [`widen`](Self::widen), [`truncate`](Self::truncate),
/// [`checked_truncate`](Self::checked_truncate) and
/// [`saturating_truncate`](Self::saturating_truncate). A non-zero integer, numo's or a native
/// `NonZero`, is also the target of the casts that check, which treat zero apart: `checked_cast`
/// returns `None`, `strict_cast` panics with the text of the error of the kind `Zero`, and
/// `unchecked_cast`, whose caller guarantees a value that is not zero, checks it in every build and
/// panics. It is the target of `widen` from a non-zero integer of the same sign that is at most as
/// wide, as the native `From` converts between the `NonZero` integers, and not of the wrapping,
/// saturating and truncating casts, whose value may be zero. Between numo's non-zero integers, the
/// casts are the conversions, as between the integers, and `widen` converts one losslessly into a
/// native `NonZero`, as in `x.widen::<NonZero<u64>>()`.
///
/// ```
/// use core::num::NonZero;
///
/// use numo::{
///   NonZeroUint,
///   typenum::{U7, U16},
///   u7,
/// };
///
/// let five = NonZeroUint::<U7>::try_from(u7::new(5))?;
/// assert_eq!(u7::from(five), u7::new(5));
/// let zero = NonZeroUint::<U7>::try_from(u7::ZERO).unwrap_err();
/// assert_eq!(zero.to_string(), "number would be zero for non-zero type");
///
/// // The native `NonZero` integers, by direction.
/// let wide = NonZeroUint::<U16>::from(NonZero::<u8>::MAX);
/// assert_eq!(NonZero::<u8>::try_from(wide)?, NonZero::<u8>::MAX);
/// assert!(NonZero::<i8>::try_from(wide).is_err());
///
/// // The casts, from a non-zero integer and into one.
/// assert_eq!(five.checked_cast::<u8>(), Some(5));
/// let widened: NonZeroUint<U16> = five.widen();
/// assert_eq!(widened.checked_cast::<NonZeroUint<U7>>(), Some(five));
/// assert_eq!(wide.checked_cast::<NonZeroUint<U7>>(), None);
/// assert_eq!(u7::ZERO.checked_cast::<NonZeroUint<U7>>(), None);
/// assert_eq!(five.widen::<NonZero<u64>>().get(), 5);
/// # Ok::<(), core::num::TryFromIntError>(())
/// ```
#[repr(transparent)]
pub struct NonZeroUint<Bits, Repr = <Bits as BitWidth>::NonZeroUnsigned> {
  // `Repr` is the storage of the value itself: `core::num::NonZero<u8>` to `NonZero<u128>` for a
  // native storage, whose niche `Option` uses, and the array for an array storage, which the
  // methods lend as a `Uint`. The value has `Bits` bits, as that of `Uint` does, and is not zero.
  // In a native storage the validity of `core::num::NonZero` excludes zero; in an array both are
  // logical invariants, as the width of `Uint` is, which no `unsafe` code may rely on. A `Uint`
  // over a storage that is not one of `Uint`, such as `NonZero<u8>`, has no methods: only `Clone`,
  // `Copy`, `PartialEq`, `Eq` and the seal of the integer traits.
  value: Uint<Bits, Repr>,
}

/// A non-zero signed integer of `Bits` bits in two's complement, stored in `Repr`.
///
/// The value runs from `-2^(Bits - 1)` to `2^(Bits - 1) - 1`, without zero. `Repr` is the storage
/// of a value that is not zero: `core::num::NonZero<i8>`, `NonZero<i16>`, `NonZero<i32>`,
/// `NonZero<i64>` or `NonZero<i128>`, whose integer holds at least `Bits` bits, or an array of
/// limbs, least significant first, in two's complement, as for [`Int`]. Each is the
/// [`NonZero`](crate::IntRepr::NonZero) of a storage of [`Int`], and without `Repr` the storage is
/// that of the default storage of [`Int`], which [`BitWidth`] names: `NonZeroInt<U7>` is
/// `NonZeroInt<U7, NonZero<i8>>` and `NonZeroInt<U200>` is `NonZeroInt<U200, [u64; 4]>`.
///
/// As for [`NonZeroUint`], the type has the size, alignment and ABI of `Repr`, and `Option` of a
/// native storage takes the niche of zero. [`new`](Self::new) creates a value from the [`Int`] of
/// the same width and storage, returning `None` for zero, [`new_unchecked`](Self::new_unchecked)
/// creates it without the check, and [`get`](Self::get) returns it. The constants are `BITS`, and
/// from one bit `MIN` and `MAX`. One bit holds -1 and 0, of which only -1 is not zero, so `MIN` and
/// `MAX` of one bit are both -1; zero bits hold no value but zero. Values compare, hash and format
/// as the integers of the same values, and [`from_str_radix`](Self::from_str_radix) and `FromStr`
/// parse as for [`NonZeroUint`], with `-` as well as `+`: `-0` is the error of the kind `Zero`.
/// The byte order, [`swap_bytes`](Self::swap_bytes), [`to_be`](Self::to_be),
/// [`to_le`](Self::to_le), [`from_be`](Self::from_be) and [`from_le`](Self::from_le), is that of
/// the [`Int`] of the same value at a width of whole bytes, as for [`NonZeroUint`]: a conversion of
/// the value, not a layout, for an array storage.
///
/// # Arithmetic
///
/// The methods are those of [`NonZeroUint`] that the native signed non-zero integers have: the
/// bit counts, the highest and the lowest one and their isolation, the rotations and the bit
/// reversal, which move the sign bit like any other, and the checked and saturating multiplication
/// and exponentiation. To them come [`is_negative`](Self::is_negative) and
/// [`is_positive`](Self::is_positive), the negation and the absolute value in the `checked_*`,
/// `overflowing_*`, `wrapping_*` and `saturating_*` forms, [`abs`](Self::abs),
/// [`unsigned_abs`](Self::unsigned_abs), which returns the [`NonZeroUint`] of the same width and
/// also holds the absolute value of `MIN`, and [`cast_unsigned`](Self::cast_unsigned). A
/// saturating method returns the bound of the range that the result passes, and `MAX` for the
/// negation and the absolute value of `MIN`. At one bit, whose `MAX` is -1, a product of -1 and -1
/// and an even power of -1 overflow, and their saturating forms return -1. `|` and `|=` work as for
/// [`NonZeroUint`], and the negation `-` on a value and on a reference. `Int` has no division by a
/// `NonZeroInt`: `MIN / -1` overflows, so a divisor that is not zero does not make the division
/// safe. The traits
/// [`FixedNonZero`](crate::FixedNonZero), [`SignedNonZero`](crate::SignedNonZero),
/// [`NonZeroLimits`](crate::NonZeroLimits) and [`ConstNonZeroLimits`](crate::ConstNonZeroLimits)
/// give generic code the methods and the limits.
///
/// ```
/// use numo::{NonZeroInt, i7, typenum::U7, u7};
///
/// let min = NonZeroInt::<U7>::MIN;
/// assert!(min.is_negative());
/// assert_eq!(min.checked_neg(), None);
/// assert_eq!(min.saturating_abs(), NonZeroInt::<U7>::MAX);
/// assert_eq!(min.unsigned_abs().get(), u7::new(64));
/// let three = NonZeroInt::<U7>::new(i7::new(3)).unwrap();
/// assert_eq!((-three).get(), i7::new(-3));
/// assert_eq!(min.saturating_mul(&three), min);
///
/// // One bit holds only -1.
/// type I1 = NonZeroInt<numo::typenum::U1>;
/// assert_eq!(I1::MIN, I1::MAX);
/// assert_eq!(I1::MIN.checked_mul(&I1::MIN), None);
/// assert_eq!(I1::MIN.saturating_neg(), I1::MIN);
/// ```
///
/// # Conversions
///
/// The conversions are those of [`NonZeroUint`] (see [Conversions](NonZeroUint#conversions)):
/// `TryFrom` of the [`Int`] of the same width and storage, with the error of the kind `Zero` for
/// zero, and `From` back; `From` of `NonZero<i8>` to `NonZero<i128>` into a `NonZeroInt` whose
/// width holds every value of it, and of `NonZero<u8>` to `NonZero<u128>` into one with a bit more
/// for the sign; `TryFrom` of `NonZero<usize>` and `NonZero<isize>` into any width, and of a
/// `NonZeroInt` of any width into the `NonZero` of each native integer; and the casts, which
/// convert a negative value as those of [`Int`] do.
///
/// ```
/// use core::num::NonZero;
///
/// use numo::{
///   NonZeroInt, NonZeroUint, i7,
///   typenum::{U7, U16},
/// };
///
/// let minus_five = NonZeroInt::<U7>::try_from(i7::new(-5))?;
/// assert_eq!(minus_five.checked_cast::<u8>(), None);
/// assert_eq!(minus_five.saturating_cast::<u8>(), 0);
/// assert_eq!(minus_five.widen::<NonZero<i64>>().get(), -5);
/// let error = NonZero::<u16>::try_from(minus_five).unwrap_err();
/// assert_eq!(error.to_string(), "number too small to fit in target type");
///
/// let wide = NonZeroInt::<U16>::from(NonZero::<u8>::MAX);
/// assert_eq!(wide.checked_cast::<NonZeroUint<U7>>(), None);
/// let positive = wide.checked_cast::<NonZeroUint<U16>>().unwrap();
/// assert_eq!(positive.get().value(), 255);
/// # Ok::<(), core::num::TryFromIntError>(())
/// ```
///
/// # Overflow
///
/// The methods with an overflow prefix behave the same in every build. [`abs`](Self::abs) and the
/// negation `-` of `MIN` overflow, as for [`Int`]: they repeat a native negation, which panics or
/// returns `MIN` as `overflow-checks` decides (see [Overflow](Int#overflow)), and a panic reports
/// the caller. No other method or operator panics.
#[repr(transparent)]
pub struct NonZeroInt<Bits, Repr = <Bits as BitWidth>::NonZeroSigned> {
  // As for `NonZeroUint`: `Repr` is the storage of the value itself, and the value has `Bits` bits
  // in two's complement, as that of `Int` does, and is not zero.
  value: Int<Bits, Repr>,
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
// own constants for each width, which its sealed trait carries. `ONE` also needs a width that
// holds one.
impl<B: FitsBits<R>, R: UintRepr> Uint<B, R> {
  /// The number of bits, `Bits`.
  pub const BITS: u32 = B::U32;

  /// The number of bytes of the value, `ceil(Bits / 8)`: the length of its arrays of bytes, and
  /// of the buffer that it copies its bytes to.
  pub const BYTES: usize = bytes::byte_count(B::U32);
}

impl<B: FitsBits<R>, R: InlineUintRepr> Uint<B, R> {
  /// Zero.
  pub const ZERO: Self = Self::wrap(R::ZERO);

  /// The smallest value, zero.
  pub const MIN: Self = Self::ZERO;

  /// The largest value, `2^Bits - 1`.
  pub const MAX: Self = Self::wrap(<R::Width<B> as UintWidth<R>>::MAX);
}

impl<B: FitsBits<R> + NonZero, R: InlineUintRepr> Uint<B, R> {
  /// One. Zero bits hold only zero, so `Uint` of zero bits has no `ONE`.
  pub const ONE: Self = Self::wrap(R::ONE);
}

impl<B: FitsBits<R>, R: IntRepr> Int<B, R> {
  /// The number of bits, `Bits`, including the sign bit.
  pub const BITS: u32 = B::U32;

  /// The number of bytes of the value, `ceil(Bits / 8)`: the length of its arrays of bytes, and
  /// of the buffer that it copies its bytes to.
  pub const BYTES: usize = bytes::byte_count(B::U32);
}

impl<B: FitsBits<R>, R: InlineIntRepr> Int<B, R> {
  /// Zero.
  pub const ZERO: Self = Self::wrap(R::ZERO);

  /// The smallest value, `-2^(Bits - 1)`, or zero for zero bits.
  pub const MIN: Self = Self::wrap(<R::Width<B> as IntWidth<R>>::MIN);

  /// The largest value, `2^(Bits - 1) - 1`, or zero for zero bits.
  pub const MAX: Self = Self::wrap(<R::Width<B> as IntWidth<R>>::MAX);
}

impl<B: FitsBits<R> + IsGreater<U1, Output = True>, R: InlineIntRepr> Int<B, R> {
  /// One. One bit holds only -1 and 0, and zero bits only 0, so `Int` of fewer than two bits has
  /// no `ONE`.
  pub const ONE: Self = Self::wrap(R::ONE);
}

/// Emits the conversions of [`Uint`] and [`Int`] from slices of limbs of type `$limb` as inherent
/// `const fn`s of the arrays of that limb type, in an impl block whose width relation holds:
/// `[group, layer]` is the layer of the sign of the storage in `int::group`, which reads the limbs
/// by reference.
// They follow the slices of bytes of `byte_methods!`, whose rules they apply to limbs.
macro_rules! limb_slice_methods {
  ($limb:ident, [$group:ident, $layer:ident]) => {
    /// Creates an integer from the integer in `limbs`, least significant limb first, of any
    /// length: unsigned for `Uint`, and for `Int` in two's complement of the bits of the limbs,
    /// whose sign is the top bit of the last limb; no limbs hold zero. Returns `None` if that
    /// integer is outside the range of `Bits` bits, whatever the length, so limbs that only extend
    /// it, zeros or copies of its sign, are accepted. It is the integer that
    /// [`checked_from_le_slice`](Self::checked_from_le_slice) reads from the bytes of the limbs,
    /// least significant first.
    #[must_use]
    #[inline]
    pub const fn checked_from_limbs_slice(limbs: &[$limb]) -> Option<Self> {
      match int::$group::$layer::checked_read_limbs(limbs, Self::BITS) {
        Some(repr) => Some(Self::wrap(repr)),
        None => None,
      }
    }

    /// Creates an integer from the integer in `limbs`, least significant limb first, read as
    /// [`checked_from_limbs_slice`](Self::checked_from_limbs_slice) reads it, modulo `2^Bits`:
    /// for `Int`, reduced into the range of `Bits` bits. Wherever `checked_from_limbs_slice`
    /// returns a value, this returns it too.
    #[must_use]
    #[inline]
    pub const fn wrapping_from_limbs_slice(limbs: &[$limb]) -> Self {
      Self::wrap(int::$group::$layer::wrapping_read_limbs(limbs, Self::BITS))
    }

    /// Creates an integer from the integer in `limbs`, least significant limb first, read as
    /// [`checked_from_limbs_slice`](Self::checked_from_limbs_slice) reads it, modulo `2^Bits`, as
    /// [`wrapping_from_limbs_slice`](Self::wrapping_from_limbs_slice) does, and returns whether
    /// that integer is outside the range of `Bits` bits, where `checked_from_limbs_slice` returns
    /// `None`.
    #[must_use]
    #[inline]
    pub const fn overflowing_from_limbs_slice(limbs: &[$limb]) -> (Self, bool) {
      match Self::checked_from_limbs_slice(limbs) {
        Some(value) => (value, false),
        None => (Self::wrapping_from_limbs_slice(limbs), true),
      }
    }

    /// Creates an integer from the integer in `limbs`, least significant limb first, read as
    /// [`checked_from_limbs_slice`](Self::checked_from_limbs_slice) reads it, or the bound of the
    /// range of `Bits` bits that it passes: [`MAX`](Self::MAX) above the range, and for `Int`
    /// [`MIN`](Self::MIN) below it, which the integer passes when the top bit of its last limb,
    /// its sign, is set.
    #[must_use]
    #[inline]
    pub const fn saturating_from_limbs_slice(limbs: &[$limb]) -> Self {
      Self::wrap(int::$group::$layer::saturating_read_limbs(
        limbs,
        Self::BITS,
      ))
    }

    /// Creates an integer from the integer in `limbs`, least significant limb first, read as
    /// [`checked_from_limbs_slice`](Self::checked_from_limbs_slice) reads it.
    ///
    /// # Panics
    ///
    /// Panics if that integer is outside the range of `Bits` bits, in every build, reporting the
    /// caller; `checked_from_limbs_slice` returns `None` instead.
    #[must_use]
    #[inline]
    #[track_caller]
    pub const fn strict_from_limbs_slice(limbs: &[$limb]) -> Self {
      match Self::checked_from_limbs_slice(limbs) {
        Some(value) => value,
        None => bytes::strict_limbs_slice(),
      }
    }
  };
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

      limb_slice_methods!($limb, [unsigned, $lower]);

      /// Parses an integer from the digits of `radix` after an optional `+`, as the native
      /// `from_str_radix` does at the width `Bits`: the ASCII digits and letters of either case
      /// below `radix`, with no whitespace, `_` or prefix.
      ///
      /// # Errors
      ///
      /// Returns the [`ParseIntError`] of the native integers, whose
      /// [`kind`](ParseIntError::kind) is `Empty` for an empty string, `InvalidDigit` for a byte
      /// that is not a digit of `radix`, a lone sign or `-`, and `PosOverflow` for digits above
      /// `MAX`; an overflow of the digits before a byte that is not a digit comes first.
      ///
      /// # Panics
      ///
      /// Panics if `radix` is not from 2 to 36, reporting the caller, so in a constant such a
      /// radix fails to compile.
      #[inline]
      #[track_caller]
      pub const fn from_str_radix(src: &str, radix: u32) -> Result<Self, ParseIntError> {
        text::check_radix(radix);
        match int::unsigned::$lower::from_str_radix_bits(src.as_bytes(), radix, Self::BITS) {
          Ok(limbs) => Ok(Self::wrap(limbs)),
          Err(error) => Err(error),
        }
      }

      overflowing_table!(inherent_items![unsigned, $lower, &]);
      checked_table!(inherent_items![unsigned, $lower, &]);
      wrapping_table!(inherent_items![unsigned, $lower, &]);
      saturating_table!(Uint, inherent_items![unsigned, $lower, &]);
      strict_table!(inherent_items![unsigned, $lower, &]);
      uint_sign_table!(inherent_items![unsigned, $lower, &]);
      integer_math_table!(inherent_items![unsigned, $lower, &, Uint<B, [$limb; N]>]);
      uint_math_table!(inherent_items![unsigned, $lower, &]);
      integer_bits_table!(inherent_items![unsigned, $lower, &]);
      uint_bits_table!(inherent_items![unsigned, $lower, &, Int<B, [$limb; N]>]);
      bitwise_table!(inherent_items![unsigned, $lower, &]);
      byte_methods!([unsigned, $lower, &]);
      float_methods!([unsigned, $lower, &]);

      /// Unchecked addition: `self + rhs`, assuming it cannot overflow.
      ///
      /// # Safety
      ///
      /// The sum must be at most `MAX`, that is, [`checked_add`](Self::checked_add) would return
      /// `Some`. Otherwise the behavior is undefined.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn unchecked_add(&self, rhs: &Self) -> Self {
        let (sum, overflow) =
          int::unsigned::$lower::overflowing_add_bits(&self.repr, &rhs.repr, Self::BITS);
        // SAFETY: `overflow` is the flag on which `checked_add` returns `None`, and the caller
        // guarantees that it would return `Some`. Nothing else is assumed: `sum` is the value that
        // `wrapping_add` returns.
        unsafe { core::hint::assert_unchecked(!overflow) };
        Self::wrap(sum)
      }

      /// Unchecked subtraction: `self - rhs`, assuming it cannot overflow.
      ///
      /// # Safety
      ///
      /// `rhs` must be at most `self`, that is, [`checked_sub`](Self::checked_sub) would return
      /// `Some`. Otherwise the behavior is undefined.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn unchecked_sub(&self, rhs: &Self) -> Self {
        let (difference, overflow) =
          int::unsigned::$lower::overflowing_sub_bits(&self.repr, &rhs.repr, Self::BITS);
        // SAFETY: `overflow` is the flag on which `checked_sub` returns `None`, and the caller
        // guarantees that it would return `Some`. Nothing else is assumed: `difference` is the
        // value that `wrapping_sub` returns.
        unsafe { core::hint::assert_unchecked(!overflow) };
        Self::wrap(difference)
      }

      /// Unchecked multiplication: `self * rhs`, assuming it cannot overflow.
      ///
      /// # Safety
      ///
      /// The product must be at most `MAX`, that is, [`checked_mul`](Self::checked_mul) would
      /// return `Some`. Otherwise the behavior is undefined.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn unchecked_mul(&self, rhs: &Self) -> Self {
        let (product, overflow) =
          int::unsigned::$lower::overflowing_mul_bits(&self.repr, &rhs.repr, Self::BITS);
        // SAFETY: `overflow` is the flag on which `checked_mul` returns `None`, and the caller
        // guarantees that it would return `Some`. Nothing else is assumed: `product` is the value
        // that `wrapping_mul` returns.
        unsafe { core::hint::assert_unchecked(!overflow) };
        Self::wrap(product)
      }

      /// Unchecked exact division: `self / rhs`, assuming that `rhs` divides `self`.
      ///
      /// # Safety
      ///
      /// `rhs` must not be zero and must divide `self`, leaving no remainder, that is,
      /// [`checked_div_exact`](Self::checked_div_exact) would return `Some`. Otherwise the
      /// behavior is undefined. Zero bits hold no divisor but zero, so no call is sound there.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn unchecked_div_exact(&self, rhs: &Self) -> Self {
        let quotient =
          int::unsigned::$lower::checked_div_exact_bits(&self.repr, &rhs.repr, Self::BITS);
        // SAFETY: `quotient` is the value on which `checked_div_exact` returns `Some`, and the
        // caller guarantees that it would. Nothing else is assumed: the value it holds is the
        // quotient that `checked_div_exact` returns.
        Self::wrap(unsafe { quotient.unwrap_unchecked() })
      }

      /// Unchecked shift left: `self << rhs`, assuming that `rhs` is below `Bits`.
      ///
      /// # Safety
      ///
      /// `rhs` must be below `Bits`, that is, [`checked_shl`](Self::checked_shl) would return
      /// `Some`. Otherwise the behavior is undefined. Zero bits have no amount below `Bits`, so no
      /// call is sound there.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn unchecked_shl(&self, rhs: u32) -> Self {
        let (shifted, overflow) =
          int::unsigned::$lower::overflowing_shl_bits(&self.repr, rhs, Self::BITS);
        // SAFETY: `overflow` is the flag on which `checked_shl` returns `None`, and the caller
        // guarantees that it would return `Some`. Nothing else is assumed: `shifted` is the value
        // that `wrapping_shl` returns.
        unsafe { core::hint::assert_unchecked(!overflow) };
        Self::wrap(shifted)
      }

      /// Unchecked shift right: `self >> rhs`, assuming that `rhs` is below `Bits`.
      ///
      /// # Safety
      ///
      /// `rhs` must be below `Bits`, that is, [`checked_shr`](Self::checked_shr) would return
      /// `Some`. Otherwise the behavior is undefined. Zero bits have no amount below `Bits`, so no
      /// call is sound there.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn unchecked_shr(&self, rhs: u32) -> Self {
        let (shifted, overflow) =
          int::unsigned::$lower::overflowing_shr_bits(&self.repr, rhs, Self::BITS);
        // SAFETY: `overflow` is the flag on which `checked_shr` returns `None`, and the caller
        // guarantees that it would return `Some`. Nothing else is assumed: `shifted` is the value
        // that `wrapping_shr` returns.
        unsafe { core::hint::assert_unchecked(!overflow) };
        Self::wrap(shifted)
      }
    }

    impl<B, const N: usize> Uint<B, [$limb; N]> {
      #[doc = concat!(
        "Returns a copy of the ",
        stringify!($bits),
        "-bit limbs of the value, least significant first. The bits above `Bits`, if any,"
      )]
      /// are zero.
      #[must_use]
      #[inline]
      pub const fn to_limbs(&self) -> [$limb; N] {
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

      limb_slice_methods!($limb, [signed, $upper]);

      /// Parses an integer from the digits of `radix` after an optional `+` or `-`, as the native
      /// `from_str_radix` does at the width `Bits`: the ASCII digits and letters of either case
      /// below `radix`, with no whitespace, `_` or prefix.
      ///
      /// # Errors
      ///
      /// Returns the [`ParseIntError`] of the native integers, whose
      /// [`kind`](ParseIntError::kind) is `Empty` for an empty string, `InvalidDigit` for a byte
      /// that is not a digit of `radix` or a lone sign, `PosOverflow` for digits above `MAX` and
      /// `NegOverflow` for digits after `-` below `MIN`; an overflow of the digits before a byte
      /// that is not a digit comes first.
      ///
      /// # Panics
      ///
      /// Panics if `radix` is not from 2 to 36, reporting the caller, so in a constant such a
      /// radix fails to compile.
      #[inline]
      #[track_caller]
      pub const fn from_str_radix(src: &str, radix: u32) -> Result<Self, ParseIntError> {
        text::check_radix(radix);
        match int::signed::$upper::from_str_radix_bits(src.as_bytes(), radix, Self::BITS) {
          Ok(limbs) => Ok(Self::wrap(limbs)),
          Err(error) => Err(error),
        }
      }

      overflowing_table!(inherent_items![signed, $upper, &]);
      checked_table!(inherent_items![signed, $upper, &]);
      wrapping_table!(inherent_items![signed, $upper, &]);
      saturating_table!(Int, inherent_items![signed, $upper, &]);
      strict_table!(inherent_items![signed, $upper, &]);
      int_sign_table!(inherent_items![signed, $upper, &, Uint<B, [$limb; N]>]);
      integer_math_table!(inherent_items![signed, $upper, &, Uint<B, [$limb; N]>]);
      int_math_table!(inherent_items![signed, $upper, &]);
      integer_bits_table!(inherent_items![signed, $upper, &]);
      int_bits_table!(inherent_items![signed, $upper, &, Uint<B, [$limb; N]>]);
      bitwise_table!(inherent_items![signed, $upper, &]);
      byte_methods!([signed, $upper, &]);
      float_methods!([signed, $upper, &]);

      /// Unchecked addition: `self + rhs`, assuming it cannot overflow.
      ///
      /// # Safety
      ///
      /// The sum must be within `MIN..=MAX`, that is, [`checked_add`](Self::checked_add) would
      /// return `Some`. Otherwise the behavior is undefined.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn unchecked_add(&self, rhs: &Self) -> Self {
        let (sum, overflow) =
          int::signed::$upper::overflowing_add_bits(&self.repr, &rhs.repr, Self::BITS);
        // SAFETY: `overflow` is the flag on which `checked_add` returns `None`, and the caller
        // guarantees that it would return `Some`. Nothing else is assumed: `sum` is the value that
        // `wrapping_add` returns.
        unsafe { core::hint::assert_unchecked(!overflow) };
        Self::wrap(sum)
      }

      /// Unchecked subtraction: `self - rhs`, assuming it cannot overflow.
      ///
      /// # Safety
      ///
      /// The difference must be within `MIN..=MAX`, that is, [`checked_sub`](Self::checked_sub)
      /// would return `Some`. Otherwise the behavior is undefined.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn unchecked_sub(&self, rhs: &Self) -> Self {
        let (difference, overflow) =
          int::signed::$upper::overflowing_sub_bits(&self.repr, &rhs.repr, Self::BITS);
        // SAFETY: `overflow` is the flag on which `checked_sub` returns `None`, and the caller
        // guarantees that it would return `Some`. Nothing else is assumed: `difference` is the
        // value that `wrapping_sub` returns.
        unsafe { core::hint::assert_unchecked(!overflow) };
        Self::wrap(difference)
      }

      /// Unchecked multiplication: `self * rhs`, assuming it cannot overflow.
      ///
      /// # Safety
      ///
      /// The product must be within `MIN..=MAX`, that is, [`checked_mul`](Self::checked_mul)
      /// would return `Some`. Otherwise the behavior is undefined.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn unchecked_mul(&self, rhs: &Self) -> Self {
        let (product, overflow) =
          int::signed::$upper::overflowing_mul_bits(&self.repr, &rhs.repr, Self::BITS);
        // SAFETY: `overflow` is the flag on which `checked_mul` returns `None`, and the caller
        // guarantees that it would return `Some`. Nothing else is assumed: `product` is the value
        // that `wrapping_mul` returns.
        unsafe { core::hint::assert_unchecked(!overflow) };
        Self::wrap(product)
      }

      /// Unchecked negation: `-self`, assuming it cannot overflow.
      ///
      /// # Safety
      ///
      /// The negation must be within `MIN..=MAX`, that is, [`checked_neg`](Self::checked_neg)
      /// would return `Some`: `self` must not be `MIN`, except at zero bits, where `MIN` is zero.
      /// Otherwise the behavior is undefined.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn unchecked_neg(&self) -> Self {
        let (negated, overflow) = int::signed::$upper::overflowing_neg_bits(&self.repr, Self::BITS);
        // SAFETY: `overflow` is the flag on which `checked_neg` returns `None`, and the caller
        // guarantees that it would return `Some`. Nothing else is assumed: `negated` is the value
        // that `wrapping_neg` returns.
        unsafe { core::hint::assert_unchecked(!overflow) };
        Self::wrap(negated)
      }

      /// Unchecked exact division: `self / rhs`, assuming that `rhs` divides `self` and that the
      /// quotient cannot overflow.
      ///
      /// # Safety
      ///
      /// `rhs` must not be zero and must divide `self`, leaving no remainder, and the quotient must
      /// be within `MIN..=MAX`, so `self` must not be `MIN` with `rhs` -1: that is,
      /// [`checked_div_exact`](Self::checked_div_exact) would return `Some`. Otherwise the
      /// behavior is undefined. Zero bits hold no divisor but zero, so no call is sound there.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn unchecked_div_exact(&self, rhs: &Self) -> Self {
        let quotient =
          int::signed::$upper::checked_div_exact_bits(&self.repr, &rhs.repr, Self::BITS);
        // SAFETY: `quotient` is the value on which `checked_div_exact` returns `Some`, and the
        // caller guarantees that it would. Nothing else is assumed: the value it holds is the
        // quotient that `checked_div_exact` returns.
        Self::wrap(unsafe { quotient.unwrap_unchecked() })
      }

      /// Unchecked shift left: `self << rhs`, assuming that `rhs` is below `Bits`.
      ///
      /// # Safety
      ///
      /// `rhs` must be below `Bits`, that is, [`checked_shl`](Self::checked_shl) would return
      /// `Some`. Otherwise the behavior is undefined. Zero bits have no amount below `Bits`, so no
      /// call is sound there.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn unchecked_shl(&self, rhs: u32) -> Self {
        let (shifted, overflow) =
          int::signed::$upper::overflowing_shl_bits(&self.repr, rhs, Self::BITS);
        // SAFETY: `overflow` is the flag on which `checked_shl` returns `None`, and the caller
        // guarantees that it would return `Some`. Nothing else is assumed: `shifted` is the value
        // that `wrapping_shl` returns.
        unsafe { core::hint::assert_unchecked(!overflow) };
        Self::wrap(shifted)
      }

      /// Unchecked shift right: `self >> rhs`, an arithmetic shift, assuming that `rhs` is below
      /// `Bits`.
      ///
      /// # Safety
      ///
      /// `rhs` must be below `Bits`, that is, [`checked_shr`](Self::checked_shr) would return
      /// `Some`. Otherwise the behavior is undefined. Zero bits have no amount below `Bits`, so no
      /// call is sound there.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn unchecked_shr(&self, rhs: u32) -> Self {
        let (shifted, overflow) =
          int::signed::$upper::overflowing_shr_bits(&self.repr, rhs, Self::BITS);
        // SAFETY: `overflow` is the flag on which `checked_shr` returns `None`, and the caller
        // guarantees that it would return `Some`. Nothing else is assumed: `shifted` is the value
        // that `wrapping_shr` returns.
        unsafe { core::hint::assert_unchecked(!overflow) };
        Self::wrap(shifted)
      }
    }

    impl<B, const N: usize> Int<B, [$limb; N]> {
      #[doc = concat!(
        "Returns a copy of the ",
        stringify!($bits),
        "-bit limbs of the value, least significant first, in two's complement. The bits"
      )]
      /// above `Bits`, if any, are copies of the sign bit.
      #[must_use]
      #[inline]
      pub const fn to_limbs(&self) -> [$limb; N] {
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
      #[inline(always)]
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
      #[inline(always)]
      pub const fn try_new(value: $uint) -> Result<Self, TryNewError> {
        if int::$uint::fits_bits(value, Self::BITS) {
          Ok(Self::wrap(value))
        } else {
          Err(TryNewError::new(Self::BITS))
        }
      }

      /// Parses an integer from the digits of `radix` after an optional `+`, as the native
      /// `from_str_radix` does at the width `Bits`: the ASCII digits and letters of either case
      /// below `radix`, with no whitespace, `_` or prefix.
      ///
      /// # Errors
      ///
      /// Returns the [`ParseIntError`] of the native integers, whose
      /// [`kind`](ParseIntError::kind) is `Empty` for an empty string, `InvalidDigit` for a byte
      /// that is not a digit of `radix`, a lone sign or `-`, and `PosOverflow` for digits above
      /// `MAX`; an overflow of the digits before a byte that is not a digit comes first.
      ///
      /// # Panics
      ///
      /// Panics if `radix` is not from 2 to 36, reporting the caller, so in a constant such a
      /// radix fails to compile.
      #[inline]
      #[track_caller]
      pub const fn from_str_radix(src: &str, radix: u32) -> Result<Self, ParseIntError> {
        text::check_radix(radix);
        match int::$uint::from_str_radix_bits(src.as_bytes(), radix, Self::BITS) {
          Ok(value) => Ok(Self::wrap(value)),
          Err(error) => Err(error),
        }
      }

      overflowing_table!(inherent_items![$uint]);
      checked_table!(inherent_items![$uint]);
      wrapping_table!(inherent_items![$uint]);
      saturating_table!(Uint, inherent_items![$uint]);
      strict_table!(inherent_items![$uint]);
      uint_sign_table!(inherent_items![$uint]);
      integer_math_table!(inherent_items![$uint, Uint<B, $uint>]);
      uint_math_table!(inherent_items![$uint]);
      integer_bits_table!(inherent_items![$uint]);
      uint_bits_table!(inherent_items![$uint, Int<B, $int>]);
      bitwise_table!(inherent_items![$uint]);
      byte_methods!([$uint]);
      float_methods!([$uint]);

      /// Unchecked addition: `self + rhs`, assuming it cannot overflow.
      ///
      /// # Safety
      ///
      /// The sum must be at most `MAX`, that is, [`checked_add`](Self::checked_add) would return
      /// `Some`. Otherwise the behavior is undefined.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn unchecked_add(&self, rhs: &Self) -> Self {
        if Self::BITS == $uint::BITS {
          // SAFETY: `Bits` is the width of the native integer, so `checked_add` returns `Some`
          // exactly when the native addition does not overflow, and the caller guarantees that it
          // would: that is the precondition of the native `unchecked_add`.
          Self::wrap(unsafe { $uint::unchecked_add(self.repr, rhs.repr) })
        } else {
          let (sum, overflow) = int::$uint::overflowing_add_bits(self.repr, rhs.repr, Self::BITS);
          // SAFETY: `overflow` is the flag on which `checked_add` returns `None`, and the caller
          // guarantees that it would return `Some`. Nothing else is assumed: `sum` is the value
          // that `wrapping_add` returns.
          unsafe { core::hint::assert_unchecked(!overflow) };
          Self::wrap(sum)
        }
      }

      /// Unchecked subtraction: `self - rhs`, assuming it cannot overflow.
      ///
      /// # Safety
      ///
      /// `rhs` must be at most `self`, that is, [`checked_sub`](Self::checked_sub) would return
      /// `Some`. Otherwise the behavior is undefined.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn unchecked_sub(&self, rhs: &Self) -> Self {
        if Self::BITS == $uint::BITS {
          // SAFETY: `Bits` is the width of the native integer, so `checked_sub` returns `Some`
          // exactly when the native subtraction does not overflow, and the caller guarantees that
          // it would: that is the precondition of the native `unchecked_sub`.
          Self::wrap(unsafe { $uint::unchecked_sub(self.repr, rhs.repr) })
        } else {
          let (difference, overflow) =
            int::$uint::overflowing_sub_bits(self.repr, rhs.repr, Self::BITS);
          // SAFETY: `overflow` is the flag on which `checked_sub` returns `None`, and the caller
          // guarantees that it would return `Some`. Nothing else is assumed: `difference` is the
          // value that `wrapping_sub` returns.
          unsafe { core::hint::assert_unchecked(!overflow) };
          Self::wrap(difference)
        }
      }

      /// Unchecked multiplication: `self * rhs`, assuming it cannot overflow.
      ///
      /// # Safety
      ///
      /// The product must be at most `MAX`, that is, [`checked_mul`](Self::checked_mul) would
      /// return `Some`. Otherwise the behavior is undefined.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn unchecked_mul(&self, rhs: &Self) -> Self {
        if Self::BITS == $uint::BITS {
          // SAFETY: `Bits` is the width of the native integer, so `checked_mul` returns `Some`
          // exactly when the native multiplication does not overflow, and the caller guarantees
          // that it would: that is the precondition of the native `unchecked_mul`.
          Self::wrap(unsafe { $uint::unchecked_mul(self.repr, rhs.repr) })
        } else {
          let (product, overflow) =
            int::$uint::overflowing_mul_bits(self.repr, rhs.repr, Self::BITS);
          // SAFETY: `overflow` is the flag on which `checked_mul` returns `None`, and the caller
          // guarantees that it would return `Some`. Nothing else is assumed: `product` is the
          // value that `wrapping_mul` returns.
          unsafe { core::hint::assert_unchecked(!overflow) };
          Self::wrap(product)
        }
      }

      /// Unchecked exact division: `self / rhs`, assuming that `rhs` divides `self`.
      ///
      /// # Safety
      ///
      /// `rhs` must not be zero and must divide `self`, leaving no remainder, that is,
      /// [`checked_div_exact`](Self::checked_div_exact) would return `Some`. Otherwise the
      /// behavior is undefined. Zero bits hold no divisor but zero, so no call is sound there.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn unchecked_div_exact(&self, rhs: &Self) -> Self {
        // The native `unchecked_div_exact` is unstable, so the full width computes the checked
        // quotient, as every other width does.
        let quotient = int::$uint::checked_div_exact_bits(self.repr, rhs.repr, Self::BITS);
        // SAFETY: `quotient` is the value on which `checked_div_exact` returns `Some`, and the
        // caller guarantees that it would. Nothing else is assumed: the value it holds is the
        // quotient that `checked_div_exact` returns.
        Self::wrap(unsafe { quotient.unwrap_unchecked() })
      }

      /// Unchecked shift left: `self << rhs`, assuming that `rhs` is below `Bits`.
      ///
      /// # Safety
      ///
      /// `rhs` must be below `Bits`, that is, [`checked_shl`](Self::checked_shl) would return
      /// `Some`. Otherwise the behavior is undefined. Zero bits have no amount below `Bits`, so no
      /// call is sound there.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn unchecked_shl(&self, rhs: u32) -> Self {
        if Self::BITS == $uint::BITS {
          // SAFETY: `Bits` is the width of the native integer, so `checked_shl` returns `Some`
          // exactly when `rhs` is below that width, and the caller guarantees that it would: that
          // is the precondition of the native `unchecked_shl`.
          Self::wrap(unsafe { $uint::unchecked_shl(self.repr, rhs) })
        } else {
          let (shifted, overflow) = int::$uint::overflowing_shl_bits(self.repr, rhs, Self::BITS);
          // SAFETY: `overflow` is the flag on which `checked_shl` returns `None`, and the caller
          // guarantees that it would return `Some`. Nothing else is assumed: `shifted` is the value
          // that `wrapping_shl` returns.
          unsafe { core::hint::assert_unchecked(!overflow) };
          Self::wrap(shifted)
        }
      }

      /// Unchecked shift right: `self >> rhs`, assuming that `rhs` is below `Bits`.
      ///
      /// # Safety
      ///
      /// `rhs` must be below `Bits`, that is, [`checked_shr`](Self::checked_shr) would return
      /// `Some`. Otherwise the behavior is undefined. Zero bits have no amount below `Bits`, so no
      /// call is sound there.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn unchecked_shr(&self, rhs: u32) -> Self {
        if Self::BITS == $uint::BITS {
          // SAFETY: `Bits` is the width of the native integer, so `checked_shr` returns `Some`
          // exactly when `rhs` is below that width, and the caller guarantees that it would: that
          // is the precondition of the native `unchecked_shr`.
          Self::wrap(unsafe { $uint::unchecked_shr(self.repr, rhs) })
        } else {
          let (shifted, overflow) = int::$uint::overflowing_shr_bits(self.repr, rhs, Self::BITS);
          // SAFETY: `overflow` is the flag on which `checked_shr` returns `None`, and the caller
          // guarantees that it would return `Some`. Nothing else is assumed: `shifted` is the value
          // that `wrapping_shr` returns.
          unsafe { core::hint::assert_unchecked(!overflow) };
          Self::wrap(shifted)
        }
      }
    }

    impl<B> Uint<B, $uint> {
      /// Returns the value.
      #[must_use]
      #[inline(always)]
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
      #[inline(always)]
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
      #[inline(always)]
      pub const fn try_new(value: $int) -> Result<Self, TryNewError> {
        if int::$int::fits_bits(value, Self::BITS) {
          Ok(Self::wrap(value))
        } else {
          Err(TryNewError::new(Self::BITS))
        }
      }

      /// Parses an integer from the digits of `radix` after an optional `+` or `-`, as the native
      /// `from_str_radix` does at the width `Bits`: the ASCII digits and letters of either case
      /// below `radix`, with no whitespace, `_` or prefix.
      ///
      /// # Errors
      ///
      /// Returns the [`ParseIntError`] of the native integers, whose
      /// [`kind`](ParseIntError::kind) is `Empty` for an empty string, `InvalidDigit` for a byte
      /// that is not a digit of `radix` or a lone sign, `PosOverflow` for digits above `MAX` and
      /// `NegOverflow` for digits after `-` below `MIN`; an overflow of the digits before a byte
      /// that is not a digit comes first.
      ///
      /// # Panics
      ///
      /// Panics if `radix` is not from 2 to 36, reporting the caller, so in a constant such a
      /// radix fails to compile.
      #[inline]
      #[track_caller]
      pub const fn from_str_radix(src: &str, radix: u32) -> Result<Self, ParseIntError> {
        text::check_radix(radix);
        match int::$int::from_str_radix_bits(src.as_bytes(), radix, Self::BITS) {
          Ok(value) => Ok(Self::wrap(value)),
          Err(error) => Err(error),
        }
      }

      overflowing_table!(inherent_items![$int]);
      checked_table!(inherent_items![$int]);
      wrapping_table!(inherent_items![$int]);
      saturating_table!(Int, inherent_items![$int]);
      strict_table!(inherent_items![$int]);
      int_sign_table!(inherent_items![$int, Uint<B, $uint>]);
      integer_math_table!(inherent_items![$int, Uint<B, $uint>]);
      int_math_table!(inherent_items![$int]);
      integer_bits_table!(inherent_items![$int]);
      int_bits_table!(inherent_items![$int, Uint<B, $uint>]);
      bitwise_table!(inherent_items![$int]);
      byte_methods!([$int]);
      float_methods!([$int]);

      /// Unchecked addition: `self + rhs`, assuming it cannot overflow.
      ///
      /// # Safety
      ///
      /// The sum must be within `MIN..=MAX`, that is, [`checked_add`](Self::checked_add) would
      /// return `Some`. Otherwise the behavior is undefined.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn unchecked_add(&self, rhs: &Self) -> Self {
        if Self::BITS == $int::BITS {
          // SAFETY: `Bits` is the width of the native integer, so `checked_add` returns `Some`
          // exactly when the native addition does not overflow, and the caller guarantees that it
          // would: that is the precondition of the native `unchecked_add`.
          Self::wrap(unsafe { $int::unchecked_add(self.repr, rhs.repr) })
        } else {
          let (sum, overflow) = int::$int::overflowing_add_bits(self.repr, rhs.repr, Self::BITS);
          // SAFETY: `overflow` is the flag on which `checked_add` returns `None`, and the caller
          // guarantees that it would return `Some`. Nothing else is assumed: `sum` is the value
          // that `wrapping_add` returns.
          unsafe { core::hint::assert_unchecked(!overflow) };
          Self::wrap(sum)
        }
      }

      /// Unchecked subtraction: `self - rhs`, assuming it cannot overflow.
      ///
      /// # Safety
      ///
      /// The difference must be within `MIN..=MAX`, that is, [`checked_sub`](Self::checked_sub)
      /// would return `Some`. Otherwise the behavior is undefined.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn unchecked_sub(&self, rhs: &Self) -> Self {
        if Self::BITS == $int::BITS {
          // SAFETY: `Bits` is the width of the native integer, so `checked_sub` returns `Some`
          // exactly when the native subtraction does not overflow, and the caller guarantees that
          // it would: that is the precondition of the native `unchecked_sub`.
          Self::wrap(unsafe { $int::unchecked_sub(self.repr, rhs.repr) })
        } else {
          let (difference, overflow) =
            int::$int::overflowing_sub_bits(self.repr, rhs.repr, Self::BITS);
          // SAFETY: `overflow` is the flag on which `checked_sub` returns `None`, and the caller
          // guarantees that it would return `Some`. Nothing else is assumed: `difference` is the
          // value that `wrapping_sub` returns.
          unsafe { core::hint::assert_unchecked(!overflow) };
          Self::wrap(difference)
        }
      }

      /// Unchecked multiplication: `self * rhs`, assuming it cannot overflow.
      ///
      /// # Safety
      ///
      /// The product must be within `MIN..=MAX`, that is, [`checked_mul`](Self::checked_mul)
      /// would return `Some`. Otherwise the behavior is undefined.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn unchecked_mul(&self, rhs: &Self) -> Self {
        if Self::BITS == $int::BITS {
          // SAFETY: `Bits` is the width of the native integer, so `checked_mul` returns `Some`
          // exactly when the native multiplication does not overflow, and the caller guarantees
          // that it would: that is the precondition of the native `unchecked_mul`.
          Self::wrap(unsafe { $int::unchecked_mul(self.repr, rhs.repr) })
        } else {
          let (product, overflow) =
            int::$int::overflowing_mul_bits(self.repr, rhs.repr, Self::BITS);
          // SAFETY: `overflow` is the flag on which `checked_mul` returns `None`, and the caller
          // guarantees that it would return `Some`. Nothing else is assumed: `product` is the
          // value that `wrapping_mul` returns.
          unsafe { core::hint::assert_unchecked(!overflow) };
          Self::wrap(product)
        }
      }

      /// Unchecked negation: `-self`, assuming it cannot overflow.
      ///
      /// # Safety
      ///
      /// The negation must be within `MIN..=MAX`, that is, [`checked_neg`](Self::checked_neg)
      /// would return `Some`: `self` must not be `MIN`, except at zero bits, where `MIN` is zero.
      /// Otherwise the behavior is undefined.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn unchecked_neg(&self) -> Self {
        if Self::BITS == $int::BITS {
          // SAFETY: `Bits` is the width of the native integer, so `checked_neg` returns `Some`
          // exactly when the native negation does not overflow, and the caller guarantees that it
          // would: that is the precondition of the native `unchecked_neg`.
          Self::wrap(unsafe { $int::unchecked_neg(self.repr) })
        } else {
          let (negated, overflow) = int::$int::overflowing_neg_bits(self.repr, Self::BITS);
          // SAFETY: `overflow` is the flag on which `checked_neg` returns `None`, and the caller
          // guarantees that it would return `Some`. Nothing else is assumed: `negated` is the
          // value that `wrapping_neg` returns.
          unsafe { core::hint::assert_unchecked(!overflow) };
          Self::wrap(negated)
        }
      }

      /// Unchecked exact division: `self / rhs`, assuming that `rhs` divides `self` and that the
      /// quotient cannot overflow.
      ///
      /// # Safety
      ///
      /// `rhs` must not be zero and must divide `self`, leaving no remainder, and the quotient must
      /// be within `MIN..=MAX`, so `self` must not be `MIN` with `rhs` -1: that is,
      /// [`checked_div_exact`](Self::checked_div_exact) would return `Some`. Otherwise the
      /// behavior is undefined. Zero bits hold no divisor but zero, so no call is sound there.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn unchecked_div_exact(&self, rhs: &Self) -> Self {
        // The native `unchecked_div_exact` is unstable, so the full width computes the checked
        // quotient, as every other width does.
        let quotient = int::$int::checked_div_exact_bits(self.repr, rhs.repr, Self::BITS);
        // SAFETY: `quotient` is the value on which `checked_div_exact` returns `Some`, and the
        // caller guarantees that it would. Nothing else is assumed: the value it holds is the
        // quotient that `checked_div_exact` returns.
        Self::wrap(unsafe { quotient.unwrap_unchecked() })
      }

      /// Unchecked shift left: `self << rhs`, assuming that `rhs` is below `Bits`.
      ///
      /// # Safety
      ///
      /// `rhs` must be below `Bits`, that is, [`checked_shl`](Self::checked_shl) would return
      /// `Some`. Otherwise the behavior is undefined. Zero bits have no amount below `Bits`, so no
      /// call is sound there.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn unchecked_shl(&self, rhs: u32) -> Self {
        if Self::BITS == $int::BITS {
          // SAFETY: `Bits` is the width of the native integer, so `checked_shl` returns `Some`
          // exactly when `rhs` is below that width, and the caller guarantees that it would: that
          // is the precondition of the native `unchecked_shl`.
          Self::wrap(unsafe { $int::unchecked_shl(self.repr, rhs) })
        } else {
          let (shifted, overflow) = int::$int::overflowing_shl_bits(self.repr, rhs, Self::BITS);
          // SAFETY: `overflow` is the flag on which `checked_shl` returns `None`, and the caller
          // guarantees that it would return `Some`. Nothing else is assumed: `shifted` is the value
          // that `wrapping_shl` returns.
          unsafe { core::hint::assert_unchecked(!overflow) };
          Self::wrap(shifted)
        }
      }

      /// Unchecked shift right: `self >> rhs`, an arithmetic shift, assuming that `rhs` is below
      /// `Bits`.
      ///
      /// # Safety
      ///
      /// `rhs` must be below `Bits`, that is, [`checked_shr`](Self::checked_shr) would return
      /// `Some`. Otherwise the behavior is undefined. Zero bits have no amount below `Bits`, so no
      /// call is sound there.
      #[must_use]
      #[inline]
      #[track_caller]
      pub const unsafe fn unchecked_shr(&self, rhs: u32) -> Self {
        if Self::BITS == $int::BITS {
          // SAFETY: `Bits` is the width of the native integer, so `checked_shr` returns `Some`
          // exactly when `rhs` is below that width, and the caller guarantees that it would: that
          // is the precondition of the native `unchecked_shr`.
          Self::wrap(unsafe { $int::unchecked_shr(self.repr, rhs) })
        } else {
          let (shifted, overflow) = int::$int::overflowing_shr_bits(self.repr, rhs, Self::BITS);
          // SAFETY: `overflow` is the flag on which `checked_shr` returns `None`, and the caller
          // guarantees that it would return `Some`. Nothing else is assumed: `shifted` is the value
          // that `wrapping_shr` returns.
          unsafe { core::hint::assert_unchecked(!overflow) };
          Self::wrap(shifted)
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

      #[inline(always)]
      #[track_caller]
      fn new(value: $native) -> Self {
        Self::new(value)
      }

      #[inline(always)]
      fn try_new(value: $native) -> Result<Self, TryNewError> {
        Self::try_new(value)
      }

      #[inline(always)]
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
