//! The integer core.
//!
//! `Decimal` performs every operation on its coefficient through this module. The core has two
//! layers. The lower layer works on magnitudes: it reads and writes decimal digits, computes powers
//! of ten and scales by them, multiplies, divides and rounds quotients, and adds and subtracts with
//! overflow checks. The upper layer handles the sign and passes magnitude work to the lower layer
//! through `unsigned_abs`. A function whose implementation does not depend on the sign has the same
//! implementation in both layers.
//!
//! Each storage has a module of the same name. For the native storages, the modules for `u8`
//! through `u128` form the lower layer and the modules for `i8` through `i128` the upper layer.
//! They also give `Uint` and `Int` over the native integers the limits of a width, `max_of_bits`
//! and `min_of_bits`, its range check, `fits_bits`, and its arithmetic and bit operations, such as
//! `overflowing_add_bits` and `rotate_left_bits`, named as in the limb layers: the functions that
//! take `bits` serve `Uint` and `Int`, at the width `bits`. The wide storages `u192` through `u512`
//! and `i192` through `i512` hold arrays of 64-bit limbs, least significant first. Their modules
//! apply `unsigned::b64`, the lower layer for `[u64; N]`, and `signed::b64`, the upper layer, which
//! reads the array as two's complement. `Uint` and `Int` also store arrays of the other limb types,
//! through `unsigned::b8`, `unsigned::b16`, `unsigned::b32` and `unsigned::b128` and the upper
//! layers over them, `signed::b8`, `signed::b16`, `signed::b32` and `signed::b128`, which
//! `limb_layer!` generates from the same code as `unsigned::b64` and `signed::b64`. No native
//! integer has 256 bits, so `half_limbs!` computes the two primitives of the layer for `u128` from
//! the 64-bit halves of each limb.
//!
//! Storage-specific code calls the `const fn`s of the storage modules in constant contexts; a
//! function has the same name and contract in every module that provides it. A native module takes
//! its values by value, as the native integers do, and a wide module takes them by reference, so
//! that a call does not copy the limbs. Storage-generic code reaches the core through the sealed
//! storage trait, which calls the module of a native storage and lends the limbs of a wide storage
//! to its limb layer. A native module also converts its values to and from two limbs, and a wide
//! module from its limbs: conversions between storages of different widths go through limbs, which
//! a wide value lends.
//!
//! Preconditions are stated per function. Callers establish them from the coefficient bound
//! `|c| <= 10^digits - 1` that every `Decimal` maintains. A violated precondition is a bug in this
//! crate: native arithmetic then overflows as native arithmetic does, multi-limb arithmetic wraps
//! and fails a debug assertion, and narrowing truncates.

/// Length of a buffer that holds the decimal digits of any magnitude: `u512::MAX` has 155 and
/// `|i512::MIN|` has 154.
pub(crate) const DIGIT_BUF_LEN: usize = 155;

/// Decimal digits of a magnitude, written right-aligned by `write_digits`.
// The modules below shadow the primitive type names in this module.
pub(crate) type DigitBuf = [core::primitive::u8; DIGIT_BUF_LEN];

mod half_limbs;
mod limb_layer;

pub(crate) mod parse_errors;
pub(crate) mod signed;
pub(crate) mod slice;
pub(crate) mod unsigned;

macro_rules! native {
  (unsigned $uint:ident => $int:ident) => {
    #[doc = concat!("Lower layer for `", stringify!($uint), "`.")]
    pub(crate) mod $uint {
      use core::{cmp::Ordering, num::ParseIntError};

      use super::{DIGIT_BUF_LEN, DigitBuf, parse_errors, slice};
      use crate::rounding::{self, RoundingMode};

      native!(@shared $uint);

      /// Writes the decimal digits of `value` to the end of `buf`, without leading zeros (zero is
      /// a single `0`), and returns the index of the first digit.
      pub(crate) const fn write_digits(mut value: $uint, buf: &mut DigitBuf) -> usize {
        let mut start = DIGIT_BUF_LEN;
        loop {
          start -= 1;
          buf[start] = b'0' + (value % 10) as u8;
          value /= 10;
          if value == 0 {
            return start;
          }
        }
      }

      /// `value * 10^exp`, or `None` if the product is not representable. Requires `10^exp` to be
      /// representable.
      #[inline]
      pub(crate) const fn checked_mul_pow10(value: $uint, exp: u32) -> Option<$uint> {
        value.checked_mul(pow10(exp))
      }

      /// Quotient and remainder of `value / 10^exp`. Requires `10^exp` to be representable.
      #[inline]
      pub(crate) const fn div_rem_pow10(value: $uint, exp: u32) -> ($uint, $uint) {
        let divisor = pow10(exp);
        (value / divisor, value % divisor)
      }

      /// Returns `true` if `mode` rounds a quotient one step away from zero, given the magnitudes
      /// of the quotient and the remainder of a division rounded toward zero and of the divisor,
      /// and whether the dividend is negative. Requires `remainder < divisor`.
      pub(crate) const fn rounds_away(
        quotient: $uint,
        remainder: $uint,
        divisor: $uint,
        negative: bool,
        mode: RoundingMode,
      ) -> bool {
        // `remainder < divisor`, so this does not overflow.
        let rest = divisor - remainder;
        let half = if remainder < rest {
          Ordering::Less
        } else if remainder > rest {
          Ordering::Greater
        } else {
          Ordering::Equal
        };
        rounding::increments(mode, negative, remainder == 0, half, quotient % 2 == 1)
      }

      /// `value / divisor` rounded to an integer with `mode`, the remainder of the division
      /// rounded toward zero, and whether `mode` rounded the quotient away from zero. Requires
      /// `divisor != 0`.
      pub(crate) const fn round_div(
        value: $uint,
        divisor: $uint,
        mode: RoundingMode,
      ) -> ($uint, $uint, bool) {
        let (quotient, remainder) = div_rem(value, divisor);
        let away = rounds_away(quotient, remainder, divisor, false, mode);
        // Rounding away needs a nonzero remainder, so the divisor is at least 2 and the quotient
        // at most half of `value`: adding one does not overflow.
        let rounded = if away { quotient + 1 } else { quotient };
        (rounded, remainder, away)
      }

      /// Zero-extends `value` to two limbs.
      #[inline]
      pub(crate) const fn to_limbs(value: $uint) -> [u64; 2] {
        let wide = value as u128;
        [wide as u64, (wide >> 64) as u64]
      }

      /// The unsigned integer in `limbs`, or `None` if it is not representable.
      #[inline]
      pub(crate) const fn checked_from_limbs(limbs: [u64; 2]) -> Option<$uint> {
        let wide = native!(@u128 limbs);
        if wide <= $uint::MAX as u128 {
          Some(wide as $uint)
        } else {
          None
        }
      }

      /// `2^bits - 1`, the largest value of `bits` bits. Requires `bits <= BITS`.
      #[inline]
      pub(crate) const fn max_of_bits(bits: u32) -> $uint {
        // The shift below would be by `BITS` bits, which overflows.
        if bits == 0 {
          0
        } else {
          $uint::MAX >> ($uint::BITS - bits)
        }
      }

      /// Returns `true` if `value < 2^bits`: every bit at index `bits` or above is zero. Requires
      /// `bits <= BITS`.
      #[inline]
      pub(crate) const fn fits_bits(value: $uint, bits: u32) -> bool {
        value <= max_of_bits(bits)
      }

      /// `value` modulo `2^bits`: every bit at index `bits` or above cleared. Requires
      /// `bits <= BITS`.
      #[inline]
      pub(crate) const fn zero_extend(value: $uint, bits: u32) -> $uint {
        value & max_of_bits(bits)
      }

      /// The quotient and the remainder of `lhs / rhs`, and `false`, which no unsigned quotient
      /// overflows, or `None` if `rhs` is zero. The quotient and the remainder are at most `lhs`,
      /// so `bits` does not change them; it gives every width the signature of the upper layer.
      #[inline]
      pub(crate) const fn div_rem_bits(
        lhs: $uint,
        rhs: $uint,
        _bits: u32,
      ) -> Option<($uint, $uint, bool)> {
        if rhs == 0 {
          None
        } else {
          Some((lhs / rhs, lhs % rhs, false))
        }
      }

      /// The Euclidean quotient and remainder of `lhs / rhs`, those of the division rounded toward
      /// zero for unsigned values, as [`div_rem_bits`] gives them.
      #[inline]
      pub(crate) const fn div_rem_euclid_bits(
        lhs: $uint,
        rhs: $uint,
        bits: u32,
      ) -> Option<($uint, $uint, bool)> {
        div_rem_bits(lhs, rhs, bits)
      }

      /// `lhs / rhs` rounded down, and `false`, or `None` if `rhs` is zero.
      #[inline]
      pub(crate) const fn div_floor_bits(
        lhs: $uint,
        rhs: $uint,
        _bits: u32,
      ) -> Option<($uint, bool)> {
        match lhs.checked_div(rhs) {
          Some(quotient) => Some((quotient, false)),
          None => None,
        }
      }

      /// `lhs / rhs` rounded up, and `false`, or `None` if `rhs` is zero.
      #[inline]
      pub(crate) const fn div_ceil_bits(
        lhs: $uint,
        rhs: $uint,
        _bits: u32,
      ) -> Option<($uint, bool)> {
        if rhs == 0 {
          None
        } else {
          Some((lhs.div_ceil(rhs), false))
        }
      }

      /// The smallest multiple of `rhs` at least `lhs`, wrapped to `bits` bits, and whether it is
      /// `2^bits` or more, or `None` if `rhs` is zero. Requires `bits <= BITS` and both values
      /// below `2^bits`.
      pub(crate) const fn next_multiple_of_bits(
        lhs: $uint,
        rhs: $uint,
        bits: u32,
      ) -> Option<($uint, bool)> {
        if rhs == 0 {
          return None;
        }
        match lhs % rhs {
          0 => Some((lhs, false)),
          // `rhs - remainder` is below `rhs`, so it has `bits` bits.
          remainder => Some(overflowing_add_bits(lhs, rhs - remainder, bits)),
        }
      }

      /// Returns `true` if `lhs` is a multiple of `rhs`: if `lhs / rhs` leaves no remainder, or if
      /// both are zero.
      #[inline]
      pub(crate) const fn is_multiple_of(lhs: $uint, rhs: $uint) -> bool {
        lhs.is_multiple_of(rhs)
      }

      /// The square root of `value`, rounded down, which every unsigned value has.
      #[inline]
      pub(crate) const fn checked_isqrt(value: $uint) -> Option<$uint> {
        Some(value.isqrt())
      }

      /// The `n`th root of `value`, rounded down. Requires `n >= 1`.
      pub(crate) const fn nth_root(value: $uint, n: u32) -> $uint {
        if value == 0 || n == 1 {
          return value;
        }
        let log2 = value.ilog2();
        // `value < 2^(log2 + 1) <= 2^n`.
        if n > log2 {
          return 1;
        }
        // Newton's method from above the root, as in the limb layers: `x0^n >= 2^(log2 + 1)`, and
        // each step is `floor(((n - 1) * x + q) / n)` for `q = floor(value / x^(n - 1))`.
        let mut x: $uint = 1 << (log2 / n + 1);
        loop {
          // A power that overflows is above `value`.
          let quotient = match x.checked_pow(n - 1) {
            Some(power) => value / power,
            None => 0,
          };
          if quotient >= x {
            return x;
          }
          // `n <= log2 < BITS`, so `n` fits.
          x -= (x - quotient - 1) / n as $uint + 1;
        }
      }

      /// The `n`th root of `value`, rounded down, or `None` if `n` is zero.
      #[inline]
      pub(crate) const fn checked_nth_root(value: $uint, n: u32) -> Option<$uint> {
        if n == 0 { None } else { Some(nth_root(value, n)) }
      }

      /// `|lhs - rhs|`.
      #[inline]
      pub(crate) const fn abs_diff(lhs: $uint, rhs: $uint) -> $uint {
        lhs.abs_diff(rhs)
      }

      /// The smallest power of two at least `value`, wrapped to `bits` bits, and whether it is
      /// `2^bits` or more, as it is at zero bits, where the power of two of zero is one. Requires
      /// `bits <= BITS` and `value < 2^bits`.
      #[inline]
      pub(crate) const fn next_power_of_two_bits(value: $uint, bits: u32) -> ($uint, bool) {
        match value.checked_next_power_of_two() {
          Some(power) if fits_bits(power, bits) => (power, false),
          // A power of two of `bits` bits or more wraps to zero.
          _ => (0, true),
        }
      }

      /// `!value` modulo `2^bits`: every bit below index `bits` flipped. Requires `bits <= BITS`
      /// and `value < 2^bits`.
      #[inline]
      pub(crate) const fn not_bits(value: $uint, bits: u32) -> $uint {
        value ^ max_of_bits(bits)
      }

      /// The number of ones of `value`, a value of `bits` bits, which has none above them.
      #[inline]
      pub(crate) const fn count_ones_bits(value: $uint, _bits: u32) -> u32 {
        value.count_ones()
      }

      /// The number of zeros of `value` from bit `bits - 1` down to its highest one, or `bits` for
      /// zero: those of the native integer, less the `BITS - bits` zeros above the width. Requires
      /// `bits <= BITS` and `value < 2^bits`.
      #[inline]
      pub(crate) const fn leading_zeros_bits(value: $uint, bits: u32) -> u32 {
        value.leading_zeros() - ($uint::BITS - bits)
      }

      /// The number of zeros of `value` below its lowest one, or `bits` for zero.
      #[inline]
      pub(crate) const fn trailing_zeros_bits(value: $uint, bits: u32) -> u32 {
        if value == 0 { bits } else { value.trailing_zeros() }
      }

      /// The number of ones of `value` from bit `bits - 1` down, the leading zeros of its
      /// complement in `bits` bits. Requires `bits <= BITS` and `value < 2^bits`.
      #[inline]
      pub(crate) const fn leading_ones_bits(value: $uint, bits: u32) -> u32 {
        leading_zeros_bits(not_bits(value, bits), bits)
      }

      /// The number of ones of `value` from bit 0 up, the trailing zeros of its complement in
      /// `bits` bits. Requires `bits <= BITS` and `value < 2^bits`.
      #[inline]
      pub(crate) const fn trailing_ones_bits(value: $uint, bits: u32) -> u32 {
        trailing_zeros_bits(not_bits(value, bits), bits)
      }

      /// The index of the highest one of `value`, or `None` for zero: a value of `bits` bits has
      /// none above them, so the width does not change it.
      #[inline]
      pub(crate) const fn highest_one_bits(value: $uint, _bits: u32) -> Option<u32> {
        value.checked_ilog2()
      }

      /// The index of the lowest one of `value`, or `None` for zero.
      #[inline]
      pub(crate) const fn lowest_one(value: $uint) -> Option<u32> {
        if value == 0 {
          None
        } else {
          Some(value.trailing_zeros())
        }
      }

      /// `value` with only its highest one, or zero for zero.
      #[inline]
      pub(crate) const fn isolate_highest_one_bits(value: $uint, _bits: u32) -> $uint {
        match value.checked_ilog2() {
          Some(log) => 1 << log,
          None => 0,
        }
      }

      /// `value` with only its lowest one, `value & -value`, or zero for zero.
      #[inline]
      pub(crate) const fn isolate_lowest_one_bits(value: $uint, _bits: u32) -> $uint {
        value & value.wrapping_neg()
      }

      /// The bits of `value` below index `bits` in reverse order: the reversal of the native
      /// integer, shifted down by the `BITS - bits` zeros that it moves from above the width to
      /// below it. Requires `bits <= BITS` and `value < 2^bits`.
      #[inline]
      pub(crate) const fn reverse_bits_bits(value: $uint, bits: u32) -> $uint {
        // The shift below would be by `BITS` bits at zero bits, which overflows.
        if bits == 0 {
          0
        } else {
          value.reverse_bits() >> ($uint::BITS - bits)
        }
      }

      /// `value` rotated left by `n` places within `bits` bits: the bits shifted out past bit
      /// `bits - 1` come back at bit 0. Requires `bits <= BITS` and `value < 2^bits`.
      pub(crate) const fn rotate_left_bits(value: $uint, n: u32, bits: u32) -> $uint {
        // Zero bits hold only zero, which no rotation changes, and take no remainder by zero.
        if bits == 0 {
          return value;
        }
        match n % bits {
          0 => value,
          // `0 < amount < bits`, so neither shift is by `BITS` bits or more.
          amount => ((value << amount) | (value >> (bits - amount))) & max_of_bits(bits),
        }
      }

      /// `value` rotated right by `n` places within `bits` bits, the left rotation by the rest of
      /// the width. Requires `bits <= BITS` and `value < 2^bits`.
      pub(crate) const fn rotate_right_bits(value: $uint, n: u32, bits: u32) -> $uint {
        if bits == 0 {
          return value;
        }
        match n % bits {
          0 => value,
          amount => rotate_left_bits(value, bits - amount, bits),
        }
      }

      /// `value << (rhs % bits)` modulo `2^bits`, and whether `rhs >= bits`, as the native
      /// `overflowing_shl` gives them for its width; zero bits overflow on every amount. Requires
      /// `bits <= BITS` and `value < 2^bits`.
      #[inline]
      pub(crate) const fn overflowing_shl_bits(value: $uint, rhs: u32, bits: u32) -> ($uint, bool) {
        // Zero bits hold only zero, and take no remainder by zero.
        if bits == 0 {
          return (0, true);
        }
        let amount = if rhs < bits { rhs } else { rhs % bits };
        ((value << amount) & max_of_bits(bits), rhs >= bits)
      }

      /// `value >> (rhs % bits)`, and whether `rhs >= bits`, as [`overflowing_shl_bits`] gives
      /// them. Requires `bits <= BITS` and `value < 2^bits`.
      #[inline]
      pub(crate) const fn overflowing_shr_bits(value: $uint, rhs: u32, bits: u32) -> ($uint, bool) {
        if bits == 0 {
          return (0, true);
        }
        let amount = if rhs < bits { rhs } else { rhs % bits };
        (value >> amount, rhs >= bits)
      }

      /// `value << rhs` modulo `2^bits`, which is zero for `rhs >= bits`. Requires `bits <= BITS`.
      #[inline]
      pub(crate) const fn unbounded_shl_bits(value: $uint, rhs: u32, bits: u32) -> $uint {
        value.unbounded_shl(rhs) & max_of_bits(bits)
      }

      /// `value >> rhs`, which is zero for `rhs >= bits`: a value of `bits` bits has no one above
      /// them, so the width does not change it.
      #[inline]
      pub(crate) const fn unbounded_shr_bits(value: $uint, rhs: u32, _bits: u32) -> $uint {
        value.unbounded_shr(rhs)
      }

      /// `value << rhs` if `rhs < bits` and the shift moves no one out of `bits` bits, so that
      /// `>> rhs` gives `value` back, or `None` otherwise. Requires `bits <= BITS` and
      /// `value < 2^bits`.
      #[inline]
      pub(crate) const fn shl_exact_bits(value: $uint, rhs: u32, bits: u32) -> Option<$uint> {
        if rhs < bits && rhs <= leading_zeros_bits(value, bits) {
          Some(value << rhs)
        } else {
          None
        }
      }

      /// `value >> rhs` if `rhs < bits` and the shift moves no one out, so that `<< rhs` gives
      /// `value` back, or `None` otherwise. Requires `value < 2^bits`.
      #[inline]
      pub(crate) const fn shr_exact_bits(value: $uint, rhs: u32, bits: u32) -> Option<$uint> {
        if rhs < bits && rhs <= trailing_zeros_bits(value, bits) {
          Some(value >> rhs)
        } else {
          None
        }
      }

      /// `value`, a pattern of `bits` bits, read in two's complement: the signed integer of the
      /// same size, sign-extended from bit `bits - 1`, or zero for zero bits. Requires
      /// `bits <= BITS`.
      #[inline]
      pub(crate) const fn cast_signed_bits(value: $uint, bits: u32) -> $int {
        super::$int::sign_extend(value.cast_signed(), bits)
      }

      /// The bit at `index` of `value`, or `false` for `index >= bits`.
      #[inline]
      pub(crate) const fn bit(value: $uint, index: u32, bits: u32) -> bool {
        index < bits && (value >> index) & 1 == 1
      }

      /// The value of the digit `byte` in `radix`, an ASCII digit or letter of either case below
      /// `radix`, as `char::to_digit` reads it, or `None` for every other byte. Requires
      /// `2 <= radix <= 36`.
      #[inline]
      pub(crate) const fn digit_value(byte: u8, radix: u32) -> Option<u32> {
        (byte as char).to_digit(radix)
      }

      /// Parses `src`, an optional `+` and the digits of `radix`, as a value of `bits` bits, with
      /// the errors of the native `from_str_radix` from the left: `Empty` for no bytes and
      /// `InvalidDigit` for a lone sign, and at each digit, as the native parser reads it, the
      /// product by `radix` first, then `InvalidDigit` for a byte that is not a digit, then
      /// `PosOverflow` once the product or the sum is above `2^bits - 1`. A `-` is a byte that is
      /// not a digit. Requires `2 <= radix <= 36` and `bits <= BITS`.
      pub(crate) const fn from_str_radix_bits(
        src: &[u8],
        radix: u32,
        bits: u32,
      ) -> Result<$uint, ParseIntError> {
        let digits = match src {
          [] => return Err(parse_errors::EMPTY),
          [b'+' | b'-'] => return Err(parse_errors::INVALID_DIGIT),
          [b'+', digits @ ..] => digits,
          digits => digits,
        };
        let max = max_of_bits(bits);
        let mut value: $uint = 0;
        let mut i = 0;
        while i < digits.len() {
          // `radix` is at most 36, which every native integer holds.
          let product = value.checked_mul(radix as $uint);
          let digit = match digit_value(digits[i], radix) {
            Some(digit) => digit as $uint,
            None => return Err(parse_errors::INVALID_DIGIT),
          };
          value = match product {
            Some(product) => match product.checked_add(digit) {
              Some(sum) if sum <= max => sum,
              _ => return Err(parse_errors::POS_OVERFLOW),
            },
            None => return Err(parse_errors::POS_OVERFLOW),
          };
          i += 1;
        }
        Ok(value)
      }

      /// `value` with the bit at `index` set if `set`, and cleared otherwise, or `None` for
      /// `index >= bits`. Requires `bits <= BITS`.
      #[inline]
      pub(crate) const fn with_bit(
        value: $uint,
        index: u32,
        set: bool,
        bits: u32,
      ) -> Option<$uint> {
        if index >= bits {
          return None;
        }
        let mask: $uint = 1 << index;
        Some(if set { value | mask } else { value & !mask })
      }

      /// The byte of `value` at `index`: its bits from `8 * index` to `8 * index + 7`. Requires
      /// `index < BITS / 8`.
      #[inline]
      pub(crate) const fn byte_at(value: $uint, index: usize) -> u8 {
        (value >> (8 * index) as u32) as u8
      }

      /// The 64-bit word of `value` at `index`, its bits from `64 * index` to `64 * index + 63`:
      /// zero beyond the integer.
      #[inline]
      pub(crate) const fn word(value: $uint, index: usize) -> u64 {
        // Every native integer has at most the two words of a `u128`.
        let wide = value as u128;
        match index {
          0 => wide as u64,
          1 => (wide >> 64) as u64,
          _ => 0,
        }
      }

      /// Writes the low `out.len()` bytes of `value` to `out`, least significant first, or most
      /// significant first if `big_endian`. Requires `out.len() <= BITS / 8`.
      pub(crate) const fn write_bytes(value: $uint, out: &mut [u8], big_endian: bool) {
        let len = out.len();
        let mut k = 0;
        while k < len {
          out[slice::index(len, k, big_endian)] = byte_at(value, k);
          k += 1;
        }
      }

      /// The low `BITS` bits of the integer in `bytes`, least significant byte first, or most
      /// significant first if `big_endian`, extended beyond its bytes by copies of `fill`: zero
      /// for an unsigned integer, and the bits of its sign for one in two's complement.
      pub(crate) const fn read_bytes(bytes: &[u8], big_endian: bool, fill: u8) -> $uint {
        let mut value: $uint = 0;
        let mut k = 0;
        while k < size_of::<$uint>() {
          value |= (slice::byte(bytes, k, big_endian, fill) as $uint) << (8 * k) as u32;
          k += 1;
        }
        value
      }

      /// The integer in `bytes`, least significant byte first, or most significant first if
      /// `big_endian`, unsigned, as a value of `bits` bits, or `None` if it is `2^bits` or more,
      /// whatever the number of bytes. No bytes are zero. Requires `bits <= BITS`.
      pub(crate) const fn checked_read_bytes(
        bytes: &[u8],
        big_endian: bool,
        bits: u32,
      ) -> Option<$uint> {
        if slice::fits_bits(bytes, big_endian, bits, false) {
          Some(read_bytes(bytes, big_endian, 0))
        } else {
          None
        }
      }

      /// The integer in `bytes`, read as [`checked_read_bytes`] reads it, modulo `2^bits`.
      /// Requires `bits <= BITS`.
      #[inline]
      pub(crate) const fn wrapping_read_bytes(bytes: &[u8], big_endian: bool, bits: u32) -> $uint {
        zero_extend(read_bytes(bytes, big_endian, 0), bits)
      }

      /// The integer in `bytes`, read as [`checked_read_bytes`] reads it, or `2^bits - 1`, the
      /// largest value of `bits` bits, if it is above that. Requires `bits <= BITS`.
      #[inline]
      pub(crate) const fn saturating_read_bytes(
        bytes: &[u8],
        big_endian: bool,
        bits: u32,
      ) -> $uint {
        match checked_read_bytes(bytes, big_endian, bits) {
          Some(value) => value,
          None => max_of_bits(bits),
        }
      }

      /// The low `bits / 8` bytes of `value` in reverse order: the reversal of the bytes of the
      /// native integer, shifted down by the `BITS - bits` zeros that it moves from above the
      /// width to below it. Requires `bits` to be a multiple of 8, `bits <= BITS` and
      /// `value < 2^bits`.
      #[inline]
      pub(crate) const fn swap_bytes_bits(value: $uint, bits: u32) -> $uint {
        // The shift below would be by `BITS` bits at zero bits, which overflows.
        if bits == 0 {
          0
        } else {
          value.swap_bytes() >> ($uint::BITS - bits)
        }
      }

      native!(@bits $uint, zero_extend);
    }
  };
  (signed $int:ident => $uint:ident) => {
    #[doc = concat!(
      "Upper layer for `",
      stringify!($int),
      "`, over the lower layer for `",
      stringify!($uint),
      "`."
    )]
    pub(crate) mod $int {
      use core::num::ParseIntError;

      use super::{DigitBuf, parse_errors, slice};
      use crate::rounding::RoundingMode;

      native!(@shared $int);

      #[inline]
      pub(crate) const fn is_negative(value: $int) -> bool {
        value < 0
      }

      /// `-value`. Requires `value != MIN`.
      #[inline]
      pub(crate) const fn neg(value: $int) -> $int {
        -value
      }

      /// `|value|`. Requires `value != MIN`.
      #[inline]
      pub(crate) const fn abs(value: $int) -> $int {
        value.abs()
      }

      /// `|value|` as an unsigned integer, which also holds `|MIN|`.
      #[inline]
      pub(crate) const fn unsigned_abs(value: $int) -> $uint {
        value.unsigned_abs()
      }

      /// `-1`, `0` or `1` as the sign of `value`.
      #[inline]
      pub(crate) const fn signum(value: $int) -> $int {
        value.signum()
      }

      /// Writes the decimal digits of `|value|` to the end of `buf`, without leading zeros (zero
      /// is a single `0`), and returns the index of the first digit.
      #[inline]
      pub(crate) const fn write_digits(value: $int, buf: &mut DigitBuf) -> usize {
        super::$uint::write_digits(value.unsigned_abs(), buf)
      }

      /// `value * 10^exp`, or `None` if the product is not representable. Requires `10^exp` to be
      /// representable.
      #[inline]
      pub(crate) const fn checked_mul_pow10(value: $int, exp: u32) -> Option<$int> {
        match super::$uint::checked_mul_pow10(value.unsigned_abs(), exp) {
          Some(magnitude) if is_negative(value) => ZERO.checked_sub_unsigned(magnitude),
          Some(magnitude) => ZERO.checked_add_unsigned(magnitude),
          None => None,
        }
      }

      /// Quotient and remainder of `value / 10^exp`, rounding toward zero. As with `/` and `%`,
      /// both have the sign of `value`. Requires `10^exp` to be representable.
      #[inline]
      pub(crate) const fn div_rem_pow10(value: $int, exp: u32) -> ($int, $int) {
        let (quotient, remainder) = super::$uint::div_rem_pow10(value.unsigned_abs(), exp);
        // Both magnitudes are at most `|value|`, so each is exact with the sign of `value`.
        if is_negative(value) {
          (
            ZERO.wrapping_sub_unsigned(quotient),
            ZERO.wrapping_sub_unsigned(remainder),
          )
        } else {
          (quotient as $int, remainder as $int)
        }
      }

      /// `value / divisor` rounded to an integer with `mode`, the remainder of the division
      /// rounded toward zero, which has the sign of `value`, and whether `mode` rounded the
      /// quotient away from zero. Requires `divisor > 0`.
      pub(crate) const fn round_div(
        value: $int,
        divisor: $int,
        mode: RoundingMode,
      ) -> ($int, $int, bool) {
        let (quotient, remainder) = div_rem(value, divisor);
        // The sign of the value, which the quotient loses when it is zero.
        let negative = is_negative(value);
        let away = super::$uint::rounds_away(
          quotient.unsigned_abs(),
          remainder.unsigned_abs(),
          divisor.unsigned_abs(),
          negative,
          mode,
        );
        // Rounding away needs a nonzero remainder, so the divisor is at least 2 and `|quotient|`
        // at most half of `|value|`: one step further from zero does not overflow.
        let rounded = match (away, negative) {
          (false, _) => quotient,
          (true, false) => quotient + 1,
          (true, true) => quotient - 1,
        };
        (rounded, remainder, away)
      }

      /// Sign-extends `value` to two limbs.
      #[inline]
      pub(crate) const fn to_limbs(value: $int) -> [u64; 2] {
        let wide = value as i128;
        [wide as u64, (wide >> 64) as u64]
      }

      /// The two's complement integer in `limbs`, or `None` if it is not representable.
      #[inline]
      pub(crate) const fn checked_from_limbs(limbs: [u64; 2]) -> Option<$int> {
        let wide = native!(@u128 limbs) as i128;
        if $int::MIN as i128 <= wide && wide <= $int::MAX as i128 {
          Some(wide as $int)
        } else {
          None
        }
      }

      /// `2^(bits - 1) - 1`, the largest value of `bits` bits, or zero for zero bits. Requires
      /// `bits <= BITS`.
      #[inline]
      pub(crate) const fn max_of_bits(bits: u32) -> $int {
        // The shift below would be by `BITS` bits, which overflows.
        if bits == 0 {
          0
        } else {
          $int::MAX >> ($int::BITS - bits)
        }
      }

      /// `-2^(bits - 1)`, the smallest value of `bits` bits, or zero for zero bits. Requires
      /// `bits <= BITS`.
      #[inline]
      pub(crate) const fn min_of_bits(bits: u32) -> $int {
        // The arithmetic shift fills the vacated bits with copies of the sign bit. At zero bits it
        // would be by `BITS` bits, which overflows.
        if bits == 0 {
          0
        } else {
          $int::MIN >> ($int::BITS - bits)
        }
      }

      /// Returns `true` if `value` is within the range of `bits` bits, from `-2^(bits - 1)` to
      /// `2^(bits - 1) - 1`, or is zero for zero bits. Requires `bits <= BITS`.
      #[inline]
      pub(crate) const fn fits_bits(value: $int, bits: u32) -> bool {
        min_of_bits(bits) <= value && value <= max_of_bits(bits)
      }

      /// `value` with every bit at index `bits - 1` or above set to bit `bits - 1`: the two's
      /// complement integer of `bits` bits in the low bits of `value`, or zero for zero bits.
      /// Requires `bits <= BITS`.
      #[inline]
      pub(crate) const fn sign_extend(value: $int, bits: u32) -> $int {
        // Zero bits hold only zero, and have no sign bit to copy. The shifts below would be by
        // `BITS` bits, which overflows.
        if bits == 0 {
          0
        } else {
          // The arithmetic shift right copies bit `bits - 1`, shifted to the top, into the bits
          // above it.
          (value << ($int::BITS - bits)) >> ($int::BITS - bits)
        }
      }

      /// The quotient and the remainder of `lhs / rhs` rounded toward zero, the remainder with the
      /// sign of `lhs`, and whether the quotient is outside the range of `bits` bits, as it is only
      /// for `MIN / -1`, whose quotient wraps to `MIN` with the remainder zero; or `None` if `rhs`
      /// is zero. Requires `bits <= BITS` and both values within that range.
      pub(crate) const fn div_rem_bits(
        lhs: $int,
        rhs: $int,
        bits: u32,
      ) -> Option<($int, $int, bool)> {
        match rhs {
          0 => None,
          // `lhs / -1` is `-lhs`. At a partial width `MIN / -1` does not overflow the native
          // integer, so the negation at the width detects it; at the full width the native
          // division would panic.
          -1 => {
            let (quotient, overflow) = overflowing_neg_bits(lhs, bits);
            Some((quotient, 0, overflow))
          }
          // With `|rhs| >= 2`, the native division cannot overflow.
          _ => Some((lhs / rhs, lhs % rhs, false)),
        }
      }

      /// The Euclidean quotient and remainder of `lhs / rhs`, the remainder never negative, and
      /// whether the quotient is outside the range of `bits` bits, as [`div_rem_bits`] gives them.
      pub(crate) const fn div_rem_euclid_bits(
        lhs: $int,
        rhs: $int,
        bits: u32,
      ) -> Option<($int, $int, bool)> {
        match rhs {
          // The remainder of a division by -1 is zero, so both divisions agree there.
          0 | -1 => div_rem_bits(lhs, rhs, bits),
          _ => Some((lhs.div_euclid(rhs), lhs.rem_euclid(rhs), false)),
        }
      }

      /// `lhs / rhs` rounded down, and whether it is outside the range of `bits` bits, as it is
      /// only for `MIN / -1`; or `None` if `rhs` is zero. Requires `bits <= BITS` and both values
      /// within that range.
      pub(crate) const fn div_floor_bits(lhs: $int, rhs: $int, bits: u32) -> Option<($int, bool)> {
        match div_rem_bits(lhs, rhs, bits) {
          // A remainder of the other sign than `rhs` needs `|rhs| >= 2`, and one less is in range.
          Some((quotient, remainder, _)) if remainder != 0 && (remainder < 0) != (rhs < 0) => {
            Some((quotient - 1, false))
          }
          Some((quotient, _, overflow)) => Some((quotient, overflow)),
          None => None,
        }
      }

      /// `lhs / rhs` rounded up, and whether it is outside the range of `bits` bits, as it is only
      /// for `MIN / -1`; or `None` if `rhs` is zero. Requires `bits <= BITS` and both values within
      /// that range.
      pub(crate) const fn div_ceil_bits(lhs: $int, rhs: $int, bits: u32) -> Option<($int, bool)> {
        match div_rem_bits(lhs, rhs, bits) {
          // A remainder of the sign of `rhs` needs `|rhs| >= 2`, and one more is in range.
          Some((quotient, remainder, _)) if remainder != 0 && (remainder < 0) == (rhs < 0) => {
            Some((quotient + 1, false))
          }
          Some((quotient, _, overflow)) => Some((quotient, overflow)),
          None => None,
        }
      }

      /// For a positive `rhs` the smallest multiple of `rhs` at least `lhs`, and for a negative
      /// `rhs` the largest at most `lhs`, wrapped to `bits` bits, and whether it is outside the
      /// range of `bits` bits; or `None` if `rhs` is zero. Requires `bits <= BITS` and both values
      /// within that range.
      pub(crate) const fn next_multiple_of_bits(
        lhs: $int,
        rhs: $int,
        bits: u32,
      ) -> Option<($int, bool)> {
        match rhs {
          0 => None,
          // Every value is a multiple of -1, and `MIN % -1` would overflow the native remainder.
          -1 => Some((lhs, false)),
          _ => {
            let remainder = lhs % rhs;
            // The remainder of the division rounded toward negative infinity, which has the sign
            // of `rhs`: a remainder of the other sign plus `rhs`, which then differ in sign.
            let rounded = if remainder != 0 && (remainder < 0) != (rhs < 0) {
              remainder + rhs
            } else {
              remainder
            };
            if rounded == 0 {
              Some((lhs, false))
            } else {
              // `rounded` has the sign of `rhs` and a smaller magnitude, so `rhs - rounded` is
              // within the range of `bits` bits.
              Some(overflowing_add_bits(lhs, rhs - rounded, bits))
            }
          }
        }
      }

      /// The square root of `value`, rounded down, or `None` if `value` is negative.
      #[inline]
      pub(crate) const fn checked_isqrt(value: $int) -> Option<$int> {
        value.checked_isqrt()
      }

      /// The `n`th root of `value`, rounded toward zero, or `None` if `n` is zero or `value` is
      /// negative and `n` even.
      pub(crate) const fn checked_nth_root(value: $int, n: u32) -> Option<$int> {
        if n == 0 || (value < 0 && n % 2 == 0) {
          return None;
        }
        // The root of the magnitude, which also holds that of `MIN`, is at most the magnitude, so
        // its negation is in range.
        let root = super::$uint::nth_root(value.unsigned_abs(), n);
        Some(if value < 0 {
          ZERO.wrapping_sub_unsigned(root)
        } else {
          root as $int
        })
      }

      /// `|lhs - rhs|` as an unsigned integer, which holds it.
      #[inline]
      pub(crate) const fn abs_diff(lhs: $int, rhs: $int) -> $uint {
        lhs.abs_diff(rhs)
      }

      /// `!value`: every bit flipped, which keeps the copies of the sign, or zero for zero bits.
      #[inline]
      pub(crate) const fn not_bits(value: $int, bits: u32) -> $int {
        // Zero bits hold only zero, whose complement would be -1.
        if bits == 0 { 0 } else { !value }
      }

      /// The number of ones of `value` in `bits` bits, without the copies of the sign above them.
      /// Requires `bits <= BITS`.
      #[inline]
      pub(crate) const fn count_ones_bits(value: $int, bits: u32) -> u32 {
        super::$uint::zero_extend(value.cast_unsigned(), bits).count_ones()
      }

      /// The number of zeros of `value` from bit `bits - 1` down to its highest one: none for a
      /// negative value, and for a value at least zero those of the native integer less the
      /// `BITS - bits` zeros above the width. Requires `bits <= BITS` and `value` within the range
      /// of `bits` bits.
      #[inline]
      pub(crate) const fn leading_zeros_bits(value: $int, bits: u32) -> u32 {
        if value < 0 {
          0
        } else {
          value.leading_zeros() - ($int::BITS - bits)
        }
      }

      /// The number of zeros of `value` below its lowest one, or `bits` for zero: a value other
      /// than zero has a one below the copies of its sign.
      #[inline]
      pub(crate) const fn trailing_zeros_bits(value: $int, bits: u32) -> u32 {
        if value == 0 { bits } else { value.trailing_zeros() }
      }

      /// The number of ones of `value` from bit `bits - 1` down: for a negative value those of the
      /// native integer less the `BITS - bits` copies of the sign above the width, and none for a
      /// value at least zero. Requires `bits <= BITS` and `value` within the range of `bits` bits.
      #[inline]
      pub(crate) const fn leading_ones_bits(value: $int, bits: u32) -> u32 {
        if value < 0 {
          value.leading_ones() - ($int::BITS - bits)
        } else {
          0
        }
      }

      /// The number of ones of `value` from bit 0 up: `bits` for -1, and otherwise those of the
      /// native integer, since a value other than -1 has a zero below the copies of its sign.
      #[inline]
      pub(crate) const fn trailing_ones_bits(value: $int, bits: u32) -> u32 {
        if value == -1 { bits } else { value.trailing_ones() }
      }

      /// The index of the highest one of `value` in `bits` bits: the sign bit, `bits - 1`, for a
      /// negative value, or `None` for zero.
      #[inline]
      pub(crate) const fn highest_one_bits(value: $int, bits: u32) -> Option<u32> {
        if value < 0 {
          Some(bits - 1)
        } else {
          value.checked_ilog2()
        }
      }

      /// The index of the lowest one of `value`, or `None` for zero.
      #[inline]
      pub(crate) const fn lowest_one(value: $int) -> Option<u32> {
        if value == 0 {
          None
        } else {
          Some(value.trailing_zeros())
        }
      }

      /// `value` with only its highest one in `bits` bits: `MIN` for a negative value, whose
      /// highest one is the sign bit, or zero for zero. Requires `bits <= BITS` and `value` within
      /// the range of `bits` bits.
      #[inline]
      pub(crate) const fn isolate_highest_one_bits(value: $int, bits: u32) -> $int {
        // The logarithm is `None` for a value that is not positive.
        match value.checked_ilog2() {
          Some(log) => 1 << log,
          None if value < 0 => min_of_bits(bits),
          None => 0,
        }
      }

      /// `value` with only its lowest one, sign-extended from `bits` bits: `MIN` for `MIN`, whose
      /// lowest one is the sign bit, or zero for zero. Requires `bits <= BITS`.
      #[inline]
      pub(crate) const fn isolate_lowest_one_bits(value: $int, bits: u32) -> $int {
        // The lowest one of `MIN` of a partial width is positive in the native integer.
        sign_extend(value & value.wrapping_neg(), bits)
      }

      /// The `bits` bits of `value` in reverse order, read in two's complement: old bit 0 is the
      /// new sign bit. Requires `bits <= BITS` and `value` within the range of `bits` bits.
      #[inline]
      pub(crate) const fn reverse_bits_bits(value: $int, bits: u32) -> $int {
        // The arithmetic shift copies the new sign bit, the old bit 0, into the bits above the
        // width. At zero bits it would be by `BITS` bits, which overflows.
        if bits == 0 {
          0
        } else {
          value.reverse_bits() >> ($int::BITS - bits)
        }
      }

      /// The `bits` bits of `value` rotated left by `n` places, read in two's complement, so that
      /// the rotation can change the sign. Requires `bits <= BITS`.
      #[inline]
      pub(crate) const fn rotate_left_bits(value: $int, n: u32, bits: u32) -> $int {
        let pattern = super::$uint::zero_extend(value.cast_unsigned(), bits);
        sign_extend(
          super::$uint::rotate_left_bits(pattern, n, bits).cast_signed(),
          bits,
        )
      }

      /// The `bits` bits of `value` rotated right by `n` places, read in two's complement, as
      /// [`rotate_left_bits`] reads them.
      #[inline]
      pub(crate) const fn rotate_right_bits(value: $int, n: u32, bits: u32) -> $int {
        let pattern = super::$uint::zero_extend(value.cast_unsigned(), bits);
        sign_extend(
          super::$uint::rotate_right_bits(pattern, n, bits).cast_signed(),
          bits,
        )
      }

      /// `value << (rhs % bits)` wrapped to `bits` bits, whose bit `bits - 1` is the new sign, and
      /// whether `rhs >= bits`; zero bits overflow on every amount. Requires `bits <= BITS`.
      #[inline]
      pub(crate) const fn overflowing_shl_bits(value: $int, rhs: u32, bits: u32) -> ($int, bool) {
        // Zero bits hold only zero, and take no remainder by zero.
        if bits == 0 {
          return (0, true);
        }
        let amount = if rhs < bits { rhs } else { rhs % bits };
        (sign_extend(value << amount, bits), rhs >= bits)
      }

      /// `value >> (rhs % bits)`, an arithmetic shift, and whether `rhs >= bits`, as
      /// [`overflowing_shl_bits`] gives them. Requires `value` within the range of `bits` bits.
      #[inline]
      pub(crate) const fn overflowing_shr_bits(value: $int, rhs: u32, bits: u32) -> ($int, bool) {
        if bits == 0 {
          return (0, true);
        }
        let amount = if rhs < bits { rhs } else { rhs % bits };
        (value >> amount, rhs >= bits)
      }

      /// `value << rhs` wrapped to `bits` bits, which is zero for `rhs >= bits`. Requires
      /// `bits <= BITS`.
      #[inline]
      pub(crate) const fn unbounded_shl_bits(value: $int, rhs: u32, bits: u32) -> $int {
        sign_extend(value.unbounded_shl(rhs), bits)
      }

      /// `value >> rhs`, an arithmetic shift, which is 0 or -1 for `rhs >= bits`: the bits of
      /// `value` from bit `bits - 1` up already copy the sign, so the width does not change it.
      #[inline]
      pub(crate) const fn unbounded_shr_bits(value: $int, rhs: u32, _bits: u32) -> $int {
        value.unbounded_shr(rhs)
      }

      /// `value << rhs` if every bit that the shift moves out of `bits` bits, and the new bit
      /// `bits - 1`, equal the sign, so that `>> rhs` gives `value` back, or `None` otherwise.
      /// Requires `bits <= BITS` and `value` within the range of `bits` bits.
      #[inline]
      pub(crate) const fn shl_exact_bits(value: $int, rhs: u32, bits: u32) -> Option<$int> {
        // `rhs` below the leading zeros or ones keeps the sign and every bit above it, and is
        // below `bits`.
        if rhs < leading_zeros_bits(value, bits) || rhs < leading_ones_bits(value, bits) {
          Some(value << rhs)
        } else {
          None
        }
      }

      /// `value >> rhs`, an arithmetic shift, if `rhs < bits` and the shift moves no one out, so
      /// that `<< rhs` gives `value` back, or `None` otherwise.
      #[inline]
      pub(crate) const fn shr_exact_bits(value: $int, rhs: u32, bits: u32) -> Option<$int> {
        if rhs < bits && rhs <= trailing_zeros_bits(value, bits) {
          Some(value >> rhs)
        } else {
          None
        }
      }

      /// The `bits` bits of `value`, its two's complement, as the unsigned integer of the same
      /// size. Requires `bits <= BITS`.
      #[inline]
      pub(crate) const fn cast_unsigned_bits(value: $int, bits: u32) -> $uint {
        super::$uint::zero_extend(value.cast_unsigned(), bits)
      }

      /// The bit at `index` of `value`, or `false` for `index >= bits`, which reads no copy of
      /// the sign.
      #[inline]
      pub(crate) const fn bit(value: $int, index: u32, bits: u32) -> bool {
        index < bits && (value >> index) & 1 == 1
      }

      /// Parses `src`, an optional `+` or `-` and the digits of `radix`, as a value of `bits` bits
      /// in two's complement, with the errors of the native `from_str_radix` from the left:
      /// `Empty` for no bytes and `InvalidDigit` for a lone sign, and at each digit, as the native
      /// parser reads it, the product by `radix` first, then `InvalidDigit` for a byte that is not
      /// a digit, then `PosOverflow` once the product or the sum is above `2^(bits - 1) - 1`, or
      /// after `-`, which subtracts the digits, `NegOverflow` once the product or the difference is
      /// below `-2^(bits - 1)`. Zero bits hold only zero. Requires `2 <= radix <= 36` and
      /// `bits <= BITS`.
      pub(crate) const fn from_str_radix_bits(
        src: &[u8],
        radix: u32,
        bits: u32,
      ) -> Result<$int, ParseIntError> {
        let (negative, digits) = match src {
          [] => return Err(parse_errors::EMPTY),
          [b'+' | b'-'] => return Err(parse_errors::INVALID_DIGIT),
          [b'+', digits @ ..] => (false, digits),
          [b'-', digits @ ..] => (true, digits),
          digits => (false, digits),
        };
        let (min, max) = (min_of_bits(bits), max_of_bits(bits));
        let mut value: $int = 0;
        let mut i = 0;
        while i < digits.len() {
          // `radix` is at most 36, which every native integer holds.
          let product = value.checked_mul(radix as $int);
          let digit = match super::$uint::digit_value(digits[i], radix) {
            Some(digit) => digit as $int,
            None => return Err(parse_errors::INVALID_DIGIT),
          };
          value = match (product, negative) {
            (Some(product), false) => match product.checked_add(digit) {
              Some(sum) if sum <= max => sum,
              _ => return Err(parse_errors::POS_OVERFLOW),
            },
            (Some(product), true) => match product.checked_sub(digit) {
              Some(difference) if difference >= min => difference,
              _ => return Err(parse_errors::NEG_OVERFLOW),
            },
            (None, _) => return Err(parse_errors::overflow(negative)),
          };
          i += 1;
        }
        Ok(value)
      }

      /// `value` with the bit at `index` set if `set`, and cleared otherwise, sign-extended from
      /// `bits` bits, so that a change of the sign bit changes its copies, or `None` for
      /// `index >= bits`. Requires `bits <= BITS`.
      #[inline]
      pub(crate) const fn with_bit(value: $int, index: u32, set: bool, bits: u32) -> Option<$int> {
        if index >= bits {
          return None;
        }
        let mask: $int = 1 << index;
        let changed = if set { value | mask } else { value & !mask };
        Some(sign_extend(changed, bits))
      }

      /// The byte of the two's complement of `value` at `index`, that of the unsigned integer of
      /// its bits. Requires `index < BITS / 8`.
      #[inline]
      pub(crate) const fn byte_at(value: $int, index: usize) -> u8 {
        super::$uint::byte_at(value.cast_unsigned(), index)
      }

      /// The 64-bit word of `value` at `index` in two's complement, its bits from `64 * index` to
      /// `64 * index + 63`: copies of the sign beyond the integer.
      #[inline]
      pub(crate) const fn word(value: $int, index: usize) -> u64 {
        // Every native integer has at most the two words of an `i128`.
        let wide = value as i128;
        match index {
          0 => wide as u64,
          1 => (wide >> 64) as u64,
          // The arithmetic shift copies the sign into every bit.
          _ => (wide >> 127) as u64,
        }
      }

      /// Writes the low `out.len()` bytes of the two's complement of `value` to `out`, least
      /// significant first, or most significant first if `big_endian`: those of the unsigned
      /// integer of its bits. Requires `out.len() <= BITS / 8`.
      #[inline]
      pub(crate) const fn write_bytes(value: $int, out: &mut [u8], big_endian: bool) {
        super::$uint::write_bytes(value.cast_unsigned(), out, big_endian);
      }

      /// The integer in `bytes`, least significant byte first, or most significant first if
      /// `big_endian`, in two's complement of `8 * bytes.len()` bits, as a value of `bits` bits,
      /// or `None` if it is outside their range, whatever the number of bytes. No bytes are zero,
      /// and zero bits hold only zero. Requires `bits <= BITS`.
      pub(crate) const fn checked_read_bytes(
        bytes: &[u8],
        big_endian: bool,
        bits: u32,
      ) -> Option<$int> {
        if slice::fits_bits(bytes, big_endian, bits, true) {
          let fill = slice::fill(bytes, big_endian, true);
          Some(super::$uint::read_bytes(bytes, big_endian, fill).cast_signed())
        } else {
          None
        }
      }

      /// The integer in `bytes`, read as [`checked_read_bytes`] reads it, reduced modulo
      /// `2^bits` into the range of `bits` bits, or zero for zero bits. Requires `bits <= BITS`.
      #[inline]
      pub(crate) const fn wrapping_read_bytes(bytes: &[u8], big_endian: bool, bits: u32) -> $int {
        let fill = slice::fill(bytes, big_endian, true);
        sign_extend(
          super::$uint::read_bytes(bytes, big_endian, fill).cast_signed(),
          bits,
        )
      }

      /// The integer in `bytes`, read as [`checked_read_bytes`] reads it, or the bound of the
      /// range of `bits` bits that it passes: the smallest value below the range, which only a
      /// negative integer is, and the largest above it. Requires `bits <= BITS`.
      pub(crate) const fn saturating_read_bytes(
        bytes: &[u8],
        big_endian: bool,
        bits: u32,
      ) -> $int {
        match checked_read_bytes(bytes, big_endian, bits) {
          Some(value) => value,
          None if slice::fill(bytes, big_endian, true) != 0 => min_of_bits(bits),
          None => max_of_bits(bits),
        }
      }

      /// The low `bits / 8` bytes of `value` in reverse order, read in two's complement: the
      /// reversal of the bytes of the native integer, which moves the copies of the sign from
      /// above the width to below it, shifted down by them with the arithmetic shift, which
      /// copies the new sign bit, the top bit of the old low byte, above the width. Requires
      /// `bits` to be a multiple of 8, `bits <= BITS` and `value` within the range of `bits` bits.
      #[inline]
      pub(crate) const fn swap_bytes_bits(value: $int, bits: u32) -> $int {
        // The shift below would be by `BITS` bits at zero bits, which overflows.
        if bits == 0 {
          0
        } else {
          value.swap_bytes() >> ($int::BITS - bits)
        }
      }

      native!(@bits $int, sign_extend);
    }
  };
  (@shared $int:ident) => {
    pub(crate) const ZERO: $int = 0;

    #[inline]
    pub(crate) const fn is_zero(value: $int) -> bool {
      value == 0
    }

    /// The non-zero storage of `value`, which is not zero. Panics if it is zero: the non-zero
    /// integers pass only a value that is not zero, such as a product of values that are not.
    #[inline]
    pub(crate) const fn known_nonzero(value: $int) -> core::num::NonZero<$int> {
      match core::num::NonZero::new(value) {
        Some(value) => value,
        None => panic!("a non-zero result is zero"),
      }
    }

    #[inline]
    pub(crate) const fn is_positive(value: $int) -> bool {
      value > 0
    }

    /// Returns `true` if `value` is one.
    #[inline]
    pub(crate) const fn is_one(value: $int) -> bool {
      value == 1
    }

    #[inline]
    pub(crate) const fn le(lhs: $int, rhs: $int) -> bool {
      lhs <= rhs
    }

    #[inline]
    pub(crate) const fn checked_add(lhs: $int, rhs: $int) -> Option<$int> {
      lhs.checked_add(rhs)
    }

    #[inline]
    pub(crate) const fn checked_sub(lhs: $int, rhs: $int) -> Option<$int> {
      lhs.checked_sub(rhs)
    }

    /// `lhs * rhs`. Requires the product to be representable.
    #[inline]
    pub(crate) const fn mul(lhs: $int, rhs: $int) -> $int {
      lhs * rhs
    }

    /// Quotient and remainder of `lhs / rhs`, rounding toward zero. For a signed type, as with `/`
    /// and `%`, the remainder has the sign of `lhs`. Requires `rhs != 0`, and for a signed type
    /// not `lhs == MIN` with `rhs == -1`.
    #[inline]
    pub(crate) const fn div_rem(lhs: $int, rhs: $int) -> ($int, $int) {
      (lhs / rhs, lhs % rhs)
    }

    /// `10^exp`. Requires `10^exp` to be representable.
    #[inline]
    pub(crate) const fn pow10(exp: u32) -> $int {
      (10 as $int).pow(exp)
    }

    /// `10^digits - 1`, the largest magnitude with `digits` decimal digits. Requires `10^digits`
    /// to be representable.
    #[inline]
    pub(crate) const fn max_magnitude(digits: u32) -> $int {
      pow10(digits) - 1
    }

    /// `value * 10^exp`. Requires the product to be representable.
    #[inline]
    pub(crate) const fn mul_pow10(value: $int, exp: u32) -> $int {
      value * pow10(exp)
    }

    /// Appends the ASCII digits in `bytes[start..end]` to `value`, computing `value * 10 + d` for
    /// each digit `d` and skipping every other byte, such as the separators `_` and `.`. Requires
    /// a representable result.
    pub(crate) const fn push_digits(
      mut value: $int,
      bytes: &[u8],
      start: usize,
      end: usize,
    ) -> $int {
      let mut i = start;
      while i < end {
        let byte = bytes[i];
        if byte.is_ascii_digit() {
          value = value * 10 + (byte - b'0') as $int;
        }
        i += 1;
      }
      value
    }

    /// Truncates `limbs`, a 128-bit integer, to this type. Requires its value, read as two's
    /// complement for a signed type, to be representable.
    #[inline]
    pub(crate) const fn from_limbs(limbs: [u64; 2]) -> $int {
      native!(@u128 limbs) as $int
    }

    /// `(lhs + rhs) / 2` rounded toward zero, which cannot overflow.
    #[inline]
    pub(crate) const fn midpoint(lhs: $int, rhs: $int) -> $int {
      lhs.midpoint(rhs)
    }

    /// The logarithm of `value` to the base `base`, rounded down, or `None` if `value` is not
    /// positive or `base` is below 2.
    #[inline]
    pub(crate) const fn checked_ilog(value: $int, base: $int) -> Option<u32> {
      value.checked_ilog(base)
    }

    /// The base-2 logarithm of `value` rounded down, or `None` if `value` is not positive.
    #[inline]
    pub(crate) const fn checked_ilog2(value: $int) -> Option<u32> {
      value.checked_ilog2()
    }

    /// The base-10 logarithm of `value` rounded down, or `None` if `value` is not positive.
    #[inline]
    pub(crate) const fn checked_ilog10(value: $int) -> Option<u32> {
      value.checked_ilog10()
    }

    /// `lhs & rhs`.
    #[inline]
    pub(crate) const fn and(lhs: $int, rhs: $int) -> $int {
      lhs & rhs
    }

    /// `lhs | rhs`.
    #[inline]
    pub(crate) const fn or(lhs: $int, rhs: $int) -> $int {
      lhs | rhs
    }

    /// `lhs ^ rhs`.
    #[inline]
    pub(crate) const fn xor(lhs: $int, rhs: $int) -> $int {
      lhs ^ rhs
    }
  };
  (@u128 $limbs:ident) => {
    $limbs[0] as u128 | ($limbs[1] as u128) << 64
  };
  // The arithmetic at a width of `bits` bits, which wraps the result of the native operation
  // with `$extend`, `zero_extend` or `sign_extend`. The operands have `bits` bits, so the result
  // overflows `bits` bits when the native operation overflows or when wrapping changes it.
  (@bits $int:ident, $extend:ident) => {
    /// `lhs + rhs` wrapped to `bits` bits, and whether the exact sum is outside the range of
    /// `bits` bits. Requires `bits <= BITS` and both values within that range.
    #[inline]
    pub(crate) const fn overflowing_add_bits(lhs: $int, rhs: $int, bits: u32) -> ($int, bool) {
      let (raw, overflow) = lhs.overflowing_add(rhs);
      let wrapped = $extend(raw, bits);
      (wrapped, overflow || wrapped != raw)
    }

    /// `lhs - rhs` wrapped to `bits` bits, and whether the exact difference is outside the range
    /// of `bits` bits. Requires `bits <= BITS` and both values within that range.
    #[inline]
    pub(crate) const fn overflowing_sub_bits(lhs: $int, rhs: $int, bits: u32) -> ($int, bool) {
      let (raw, overflow) = lhs.overflowing_sub(rhs);
      let wrapped = $extend(raw, bits);
      (wrapped, overflow || wrapped != raw)
    }

    /// `lhs * rhs` wrapped to `bits` bits, and whether the exact product is outside the range of
    /// `bits` bits. Requires `bits <= BITS` and both values within that range.
    #[inline]
    pub(crate) const fn overflowing_mul_bits(lhs: $int, rhs: $int, bits: u32) -> ($int, bool) {
      let (raw, overflow) = lhs.overflowing_mul(rhs);
      let wrapped = $extend(raw, bits);
      (wrapped, overflow || wrapped != raw)
    }

    /// `-value` wrapped to `bits` bits, and whether the exact negation is outside the range of
    /// `bits` bits. Requires `bits <= BITS` and `value` within that range.
    #[inline]
    pub(crate) const fn overflowing_neg_bits(value: $int, bits: u32) -> ($int, bool) {
      let (raw, overflow) = value.overflowing_neg();
      let wrapped = $extend(raw, bits);
      (wrapped, overflow || wrapped != raw)
    }

    /// `base^exp` wrapped to `bits` bits, and whether the exact power is outside the range of
    /// `bits` bits. Requires `bits <= BITS` and `base` within that range. The native integer holds
    /// one, so `base^0` overflows only a width that does not: zero bits, and one signed bit.
    #[inline]
    pub(crate) const fn overflowing_pow_bits(base: $int, exp: u32, bits: u32) -> ($int, bool) {
      let (raw, overflow) = base.overflowing_pow(exp);
      let wrapped = $extend(raw, bits);
      (wrapped, overflow || wrapped != raw)
    }

    /// `lhs / rhs` if `rhs` divides `lhs` and the quotient is within the range of `bits` bits, or
    /// `None` if `rhs` is zero, the division leaves a remainder, or the quotient is outside that
    /// range, as only that of `MIN / -1` is. Requires `bits <= BITS` and both values within that
    /// range.
    #[inline]
    pub(crate) const fn checked_div_exact_bits(lhs: $int, rhs: $int, bits: u32) -> Option<$int> {
      match div_rem_bits(lhs, rhs, bits) {
        Some((quotient, 0, false)) => Some(quotient),
        _ => None,
      }
    }
  };
}

native!(unsigned u8 => i8);
native!(unsigned u16 => i16);
native!(unsigned u32 => i32);
native!(unsigned u64 => i64);
native!(unsigned u128 => i128);

native!(signed i8 => u8);
native!(signed i16 => u16);
native!(signed i32 => u32);
native!(signed i64 => u64);
native!(signed i128 => u128);

macro_rules! wide {
  (unsigned $uint:ident: $limbs:literal) => {
    #[doc = concat!(
      "Lower layer for `",
      stringify!($uint),
      "`, over `[u64; ",
      stringify!($limbs),
      "]`."
    )]
    pub(crate) mod $uint {
      use super::unsigned::b64;
      use crate::{$uint, rounding::RoundingMode};

      wide!(@shared $uint, b64, $limbs);
    }
  };
  (signed $int:ident: $limbs:literal) => {
    #[doc = concat!(
      "Upper layer for `",
      stringify!($int),
      "`, over `[u64; ",
      stringify!($limbs),
      "]` in two's complement."
    )]
    pub(crate) mod $int {
      use super::signed::b64;
      use crate::{$int, rounding::RoundingMode};

      wide!(@shared $int, b64, $limbs);

      #[inline]
      pub(crate) const fn is_negative(value: &$int) -> bool {
        b64::is_negative(value.as_limbs())
      }

      /// `-value`. Requires `value != MIN`.
      #[inline]
      pub(crate) const fn neg(value: &$int) -> $int {
        $int::from_limbs(b64::neg(value.as_limbs()))
      }

      /// `|value|`. Requires `value != MIN`.
      #[inline]
      pub(crate) const fn abs(value: &$int) -> $int {
        $int::from_limbs(b64::abs(value.as_limbs()))
      }
    }
  };
  (@shared $ty:ident, $layer:ident, $limbs:literal) => {
    pub(crate) const ZERO: $ty = $ty::ZERO;

    #[inline]
    pub(crate) const fn is_zero(value: &$ty) -> bool {
      $layer::is_zero(value.as_limbs())
    }

    #[inline]
    pub(crate) const fn is_positive(value: &$ty) -> bool {
      $layer::is_positive(value.as_limbs())
    }

    #[inline]
    pub(crate) const fn le(lhs: &$ty, rhs: &$ty) -> bool {
      $layer::le(lhs.as_limbs(), rhs.as_limbs())
    }

    #[inline]
    pub(crate) const fn checked_add(lhs: &$ty, rhs: &$ty) -> Option<$ty> {
      match $layer::checked_add(lhs.as_limbs(), rhs.as_limbs()) {
        Some(sum) => Some($ty::from_limbs(sum)),
        None => None,
      }
    }

    #[inline]
    pub(crate) const fn checked_sub(lhs: &$ty, rhs: &$ty) -> Option<$ty> {
      match $layer::checked_sub(lhs.as_limbs(), rhs.as_limbs()) {
        Some(difference) => Some($ty::from_limbs(difference)),
        None => None,
      }
    }

    /// `10^exp`. Requires `10^exp` to be representable.
    #[inline]
    pub(crate) const fn pow10(exp: u32) -> $ty {
      $ty::from_limbs($layer::pow10(exp))
    }

    /// `10^digits - 1`, the largest magnitude with `digits` decimal digits. Requires `10^digits`
    /// to be representable.
    #[inline]
    pub(crate) const fn max_magnitude(digits: u32) -> $ty {
      $ty::from_limbs($layer::max_magnitude(digits))
    }

    /// `value * 10^exp`. Requires the product to be representable.
    #[inline]
    pub(crate) const fn mul_pow10(value: &$ty, exp: u32) -> $ty {
      $ty::from_limbs($layer::mul_pow10(value.as_limbs(), exp))
    }

    /// Quotient and remainder of `lhs / rhs`, rounding toward zero. For a signed type, as with `/`
    /// and `%`, the remainder has the sign of `lhs`. Requires `rhs != 0`, and for a signed type
    /// not `lhs == MIN` with `rhs == -1`.
    #[inline]
    pub(crate) const fn div_rem(lhs: &$ty, rhs: &$ty) -> ($ty, $ty) {
      let (quotient, remainder) = $layer::div_rem(lhs.as_limbs(), rhs.as_limbs());
      ($ty::from_limbs(quotient), $ty::from_limbs(remainder))
    }

    /// `value / divisor` rounded to an integer with `mode`, the remainder of the division rounded
    /// toward zero, which for a signed type has the sign of `value`, and whether `mode` rounded the
    /// quotient away from zero. Requires `divisor > 0`.
    #[inline]
    pub(crate) const fn round_div(
      value: &$ty,
      divisor: &$ty,
      mode: RoundingMode,
    ) -> ($ty, $ty, bool) {
      let (rounded, remainder, away) =
        $layer::round_div(value.as_limbs(), divisor.as_limbs(), mode);
      ($ty::from_limbs(rounded), $ty::from_limbs(remainder), away)
    }

    /// Appends the ASCII digits in `bytes[start..end]` to `value`, computing `value * 10 + d` for
    /// each digit `d` and skipping every other byte, such as the separators `_` and `.`. Requires
    /// `value` not to be negative and a representable result.
    #[inline]
    pub(crate) const fn push_digits(value: &$ty, bytes: &[u8], start: usize, end: usize) -> $ty {
      $ty::from_limbs($layer::push_digits(value.as_limbs(), bytes, start, end))
    }

    #[inline]
    pub(crate) const fn from_limbs(value: [u64; $limbs]) -> $ty {
      $ty::from_limbs(value)
    }

    /// Every bit pattern is a value, so this always succeeds.
    #[inline]
    pub(crate) const fn checked_from_limbs(value: [u64; $limbs]) -> Option<$ty> {
      Some($ty::from_limbs(value))
    }
  };
}

wide!(unsigned u192: 3);
wide!(unsigned u256: 4);
wide!(unsigned u320: 5);
wide!(unsigned u384: 6);
wide!(unsigned u448: 7);
wide!(unsigned u512: 8);

wide!(signed i192: 3);
wide!(signed i256: 4);
wide!(signed i320: 5);
wide!(signed i384: 6);
wide!(signed i448: 7);
wide!(signed i512: 8);

#[cfg(test)]
mod tests;
