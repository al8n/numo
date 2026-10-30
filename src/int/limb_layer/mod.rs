//! The layers for arrays of limbs, written once for every limb type.
//!
//! `limb_layer!` generates, in the module that invokes it, the lower layer for `[Limb; N]`, which
//! reads the array as an unsigned integer, or the upper layer, which reads it as two's complement.
//! `B` denotes the number of bits of a limb, `Limb::BITS`.
//!
//! The lower layer takes, as paths to functions, the only two operations that need integers of
//! `2B` bits:
//!
//! - `carrying_mul_add(a, b, carry, add)` returns `a * b + carry + add` as two limbs, the low one
//!   first. It is at most `2^(2B) - 1`, so it does not overflow.
//! - `div_rem_wide(high, low, divisor)` returns the quotient and the remainder of
//!   `high * 2^B + low` by `divisor`. It requires `high < divisor`, so that the quotient fits a
//!   limb, and fails on a zero divisor as native division does.
//!
//! Everything else uses the methods of the limb type. The upper layer takes the signed integer of
//! `B` bits and the module of the lower layer for the same limb type.

/// Generates the lower layer (`unsigned`) or the upper layer (`signed`) for arrays of one limb
/// type in the module that invokes it, as the module documentation describes.
macro_rules! limb_layer {
  (
    unsigned limb: $limb:ident,
    carrying_mul_add: $carrying_mul_add:path,
    div_rem_wide: $div_rem_wide:path
  ) => {
    use ::core::cmp::Ordering;

    use $crate::{
      int::DigitBuf,
      rounding::{self, RoundingMode},
    };

    /// The type of a limb: an unsigned integer of `B` bits.
    type Limb = $limb;

    /// The exponent of `TEN_POW`: one limb operation scales by at most this many decimal digits.
    const MAX_STEP: u32 = Limb::MAX.ilog10();

    /// `10^MAX_STEP`, the largest power of ten in a limb.
    const TEN_POW: Limb = Limb::pow(10, MAX_STEP);

    /// The most decimal digits that one limb adds to a value: `2^B` is not a power of ten, so it is
    /// below `10^(MAX_STEP + 1)`.
    pub(crate) const DIGITS_PER_LIMB: usize = MAX_STEP as usize + 1;

    /// One.
    pub(crate) const fn one<const N: usize>() -> [Limb; N] {
      let mut one = [0; N];
      one[0] = 1;
      one
    }

    /// Returns `true` if `lhs` and `rhs` are equal.
    pub(crate) const fn eq<const N: usize>(lhs: &[Limb; N], rhs: &[Limb; N]) -> bool {
      let mut i = 0;
      while i < N {
        if lhs[i] != rhs[i] {
          return false;
        }
        i += 1;
      }
      true
    }

    #[inline]
    pub(crate) const fn is_zero<const N: usize>(value: &[Limb; N]) -> bool {
      eq(value, &[0; N])
    }

    #[inline]
    pub(crate) const fn is_positive<const N: usize>(value: &[Limb; N]) -> bool {
      !is_zero(value)
    }

    /// Returns `true` if `value` is one, which no limbs hold.
    #[inline]
    pub(crate) const fn is_one<const N: usize>(value: &[Limb; N]) -> bool {
      // `one` needs a limb to set.
      N > 0 && eq(value, &one())
    }

    /// Compares the unsigned integers formed by the limbs below index `top`. Requires `top <= N`.
    pub(crate) const fn cmp_below<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
      top: usize,
    ) -> Ordering {
      let mut i = top;
      while i > 0 {
        i -= 1;
        if lhs[i] != rhs[i] {
          return if lhs[i] < rhs[i] {
            Ordering::Less
          } else {
            Ordering::Greater
          };
        }
      }
      Ordering::Equal
    }

    /// Compares `lhs` and `rhs` from the most significant limb down.
    #[inline]
    pub(crate) const fn cmp<const N: usize>(lhs: &[Limb; N], rhs: &[Limb; N]) -> Ordering {
      cmp_below(lhs, rhs, N)
    }

    #[inline]
    pub(crate) const fn le<const N: usize>(lhs: &[Limb; N], rhs: &[Limb; N]) -> bool {
      !matches!(cmp(lhs, rhs), Ordering::Greater)
    }

    /// `lhs + rhs` modulo `2^(B * N)`, and whether the sum carried out of the top limb.
    pub(crate) const fn overflowing_add<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
    ) -> ([Limb; N], bool) {
      let mut sum = [0; N];
      let mut carry = false;
      let mut i = 0;
      while i < N {
        let (partial, first) = lhs[i].overflowing_add(rhs[i]);
        let (limb, second) = partial.overflowing_add(carry as Limb);
        sum[i] = limb;
        carry = first || second;
        i += 1;
      }
      (sum, carry)
    }

    /// `lhs + rhs`, or `None` if the sum carries out of the top limb.
    #[inline]
    pub(crate) const fn checked_add<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
    ) -> Option<[Limb; N]> {
      match overflowing_add(lhs, rhs) {
        (sum, false) => Some(sum),
        (_, true) => None,
      }
    }

    /// `lhs - rhs` modulo `2^(B * N)`, and whether the difference borrowed past the top limb.
    pub(crate) const fn overflowing_sub<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
    ) -> ([Limb; N], bool) {
      let mut difference = [0; N];
      let mut borrow = false;
      let mut i = 0;
      while i < N {
        let (partial, first) = lhs[i].overflowing_sub(rhs[i]);
        let (limb, second) = partial.overflowing_sub(borrow as Limb);
        difference[i] = limb;
        borrow = first || second;
        i += 1;
      }
      (difference, borrow)
    }

    /// `lhs - rhs`, or `None` if `rhs > lhs`.
    #[inline]
    pub(crate) const fn checked_sub<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
    ) -> Option<[Limb; N]> {
      match overflowing_sub(lhs, rhs) {
        (difference, false) => Some(difference),
        (_, true) => None,
      }
    }

    /// `value * factor + addend` modulo `2^(B * N)`, and the limb that carried out of the top limb.
    const fn mul_add_limb<const N: usize>(
      value: &[Limb; N],
      factor: Limb,
      addend: Limb,
    ) -> ([Limb; N], Limb) {
      let mut product = [0; N];
      let mut carry = addend;
      let mut i = 0;
      while i < N {
        (product[i], carry) = $carrying_mul_add(value[i], factor, carry, 0);
        i += 1;
      }
      (product, carry)
    }

    /// `lhs * rhs` modulo `2^(B * N)`. Requires the product to be representable.
    pub(crate) const fn mul<const N: usize>(lhs: &[Limb; N], rhs: &[Limb; N]) -> [Limb; N] {
      let mut product = [0; N];
      let mut i = 0;
      while i < N {
        let mut carry: Limb = 0;
        let mut j = 0;
        while i + j < N {
          (product[i + j], carry) = $carrying_mul_add(lhs[i], rhs[j], carry, product[i + j]);
          j += 1;
        }
        // A representable product carries nothing past the top limb, and the partial products that
        // would land there are zero.
        debug_assert!(carry == 0, "multiplication overflowed");
        while j < N {
          debug_assert!(lhs[i] == 0 || rhs[j] == 0, "multiplication overflowed");
          j += 1;
        }
        i += 1;
      }
      product
    }

    /// The index of the most significant nonzero limb, or `None` for zero.
    const fn top_limb<const N: usize>(value: &[Limb; N]) -> Option<usize> {
      let mut i = N;
      while i > 0 {
        i -= 1;
        if value[i] != 0 {
          return Some(i);
        }
      }
      None
    }

    /// `lhs * rhs` modulo `2^(B * N)`, and whether the product is `2^(B * N)` or more.
    pub(crate) const fn overflowing_mul<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
    ) -> ([Limb; N], bool) {
      let mut product = [0; N];
      // A partial product `lhs[i] * rhs[j]` with `i + j >= N` lands past the top limb unless a
      // factor is zero. A zero factor, as every value of no limbs is, has no top limb.
      let mut overflow = match (top_limb(lhs), top_limb(rhs)) {
        (Some(i), Some(j)) => i + j >= N,
        _ => false,
      };
      let mut i = 0;
      while i < N {
        let mut carry: Limb = 0;
        let mut j = 0;
        while i + j < N {
          (product[i + j], carry) = $carrying_mul_add(lhs[i], rhs[j], carry, product[i + j]);
          j += 1;
        }
        // The partial products that would land past the top limb are zero, so the rows hold the
        // whole product, and it reaches `2^(B * N)` exactly when a row carries out of the top
        // limb.
        overflow |= carry != 0;
        i += 1;
      }
      (product, overflow)
    }

    /// `value * 10^exp`, or `None` if the product is not representable.
    pub(crate) const fn checked_mul_pow10<const N: usize>(
      value: &[Limb; N],
      mut exp: u32,
    ) -> Option<[Limb; N]> {
      let mut value = *value;
      // Each partial product is at most the full product, so the first carry decides overflow.
      while exp > 0 {
        let step = if exp < MAX_STEP { exp } else { MAX_STEP };
        match mul_add_limb(&value, Limb::pow(10, step), 0) {
          (product, 0) => value = product,
          _ => return None,
        }
        exp -= step;
      }
      Some(value)
    }

    /// `value * 10^exp`. Requires the product to be representable.
    pub(crate) const fn mul_pow10<const N: usize>(value: &[Limb; N], mut exp: u32) -> [Limb; N] {
      let mut value = *value;
      while exp > 0 {
        let step = if exp < MAX_STEP { exp } else { MAX_STEP };
        let (product, carry) = mul_add_limb(&value, Limb::pow(10, step), 0);
        debug_assert!(carry == 0, "multiplication by a power of ten overflowed");
        value = product;
        exp -= step;
      }
      value
    }

    /// `10^exp`. Requires `10^exp` to be representable.
    #[inline]
    pub(crate) const fn pow10<const N: usize>(exp: u32) -> [Limb; N] {
      mul_pow10(&one(), exp)
    }

    /// `10^digits - 1`, the largest magnitude with `digits` decimal digits. Requires `10^digits`
    /// to be representable.
    #[inline]
    pub(crate) const fn max_magnitude<const N: usize>(digits: u32) -> [Limb; N] {
      // `10^digits >= 1`, so nothing borrows.
      overflowing_sub(&pow10(digits), &one()).0
    }

    /// Quotient and remainder of `value / divisor`. Requires `divisor != 0`.
    pub(crate) const fn div_rem_limb<const N: usize>(
      value: &[Limb; N],
      divisor: Limb,
    ) -> ([Limb; N], Limb) {
      let mut quotient = [0; N];
      let mut remainder: Limb = 0;
      let mut i = N;
      while i > 0 {
        i -= 1;
        // `remainder < divisor`, as `div_rem_wide` requires.
        (quotient[i], remainder) = $div_rem_wide(remainder, value[i], divisor);
      }
      (quotient, remainder)
    }

    /// Quotient and remainder of `value / 10^exp`.
    pub(crate) const fn div_rem_pow10<const N: usize>(
      value: &[Limb; N],
      exp: u32,
    ) -> ([Limb; N], [Limb; N]) {
      // Dividing by the factors of `10^exp` in turn truncates once, as dividing by `10^exp` does.
      let mut quotient = *value;
      let mut rest = exp;
      while rest > 0 {
        let step = if rest < MAX_STEP { rest } else { MAX_STEP };
        quotient = div_rem_limb(&quotient, Limb::pow(10, step)).0;
        rest -= step;
      }
      // `quotient * 10^exp <= value`, so neither the product nor the difference overflows.
      let (remainder, borrow) = overflowing_sub(value, &mul_pow10(&quotient, exp));
      debug_assert!(!borrow, "remainder of a division by a power of ten");
      (quotient, remainder)
    }

    /// The dividend of `div_rem` once normalized: `N` limbs, and at index `N` the limb that
    /// normalization shifts out of the top one. It stands for a `[Limb; N + 1]`, which stable
    /// Rust cannot name for a generic `N`. The division reduces it to the normalized remainder.
    struct Dividend<const N: usize> {
      limbs: [Limb; N],
      top: Limb,
    }

    impl<const N: usize> Dividend<N> {
      /// The limb at index `i`. Requires `i <= N`.
      const fn get(&self, i: usize) -> Limb {
        if i == N { self.top } else { self.limbs[i] }
      }

      /// Sets the limb at index `i`. Requires `i <= N`.
      const fn set(&mut self, i: usize, limb: Limb) {
        if i == N {
          self.top = limb;
        } else {
          self.limbs[i] = limb;
        }
      }
    }

    /// `value << shift` modulo `2^(B * N)`, and the bits shifted out of the top limb. Requires
    /// `shift < B`.
    const fn shl<const N: usize>(value: &[Limb; N], shift: u32) -> ([Limb; N], Limb) {
      // The carry below would shift by `B` bits, which overflows.
      if shift == 0 {
        return (*value, 0);
      }
      let mut shifted = [0; N];
      let mut carry = 0;
      let mut i = 0;
      while i < N {
        shifted[i] = value[i] << shift | carry;
        carry = value[i] >> (Limb::BITS - shift);
        i += 1;
      }
      (shifted, carry)
    }

    /// `value >> shift`. Requires `shift < B`.
    pub(crate) const fn shr<const N: usize>(value: &[Limb; N], shift: u32) -> [Limb; N] {
      // The bits taken from the next limb below would be shifted by `B` bits, which overflows.
      if shift == 0 {
        return *value;
      }
      let mut shifted = [0; N];
      let mut i = 0;
      while i < N {
        shifted[i] = value[i] >> shift;
        if i + 1 < N {
          shifted[i] |= value[i + 1] << (Limb::BITS - shift);
        }
        i += 1;
      }
      shifted
    }

    /// Quotient and remainder of `lhs / rhs`. Requires `rhs != 0`.
    ///
    /// A divisor of one significant limb goes through `div_rem_limb`, and a dividend below the
    /// divisor has the quotient zero. Otherwise this is Algorithm D of Knuth's The Art of Computer
    /// Programming, volume 2, section 4.3.1: both operands are shifted left until the high bit of
    /// the divisor's top limb is set, each quotient limb is estimated from the top limbs of the
    /// remaining dividend and corrected, and the remainder is shifted back.
    pub(crate) const fn div_rem<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
    ) -> ([Limb; N], [Limb; N]) {
      // The number of significant limbs of the divisor, at least one.
      let mut n = N;
      while n > 1 && rhs[n - 1] == 0 {
        n -= 1;
      }
      if n == 1 {
        // Algorithm D needs two divisor limbs. A zero divisor fails here as native division does.
        let (quotient, remainder) = div_rem_limb(lhs, rhs[0]);
        let mut wide = [0; N];
        wide[0] = remainder;
        return (quotient, wide);
      }
      if matches!(cmp(lhs, rhs), Ordering::Less) {
        return ([0; N], *lhs);
      }

      // D1: normalize. The shift drops only zero bits of the divisor, while the dividend's top
      // bits move to an extra limb. With the high bit of the divisor's top limb set, each estimate
      // below exceeds its quotient limb by at most two.
      let shift = rhs[n - 1].leading_zeros();
      let (divisor, lost) = shl(rhs, shift);
      debug_assert!(lost == 0, "normalization dropped divisor bits");
      let (limbs, top) = shl(lhs, shift);
      let mut rem = Dividend { limbs, top };
      let high = divisor[n - 1];
      let next = divisor[n - 2];
      let mut quotient = [0; N];
      // D2: one step per quotient limb, from the most significant down. Step `j` divides the
      // window `rem[j..=j + n]`, whose top `n` limbs are below the divisor; only the first window
      // reaches the extra limb, which that step clears.
      let mut j = N - n + 1;
      while j > 0 {
        j -= 1;
        // D3: estimate the quotient limb, which is below `2^B`, from the window's top two limbs,
        // and correct the estimate with the next divisor limb. The top limb is at most `high`. If
        // it equals `high`, the quotient of the top two limbs by `high` is at least `2^B`, so the
        // estimate starts at `2^B - 1`, and `rest`, the remainder that goes with it, at
        // `rem[j + n - 1] + high`, which may carry out of the limb. Once `rest` reaches `2^B`, the
        // product test cannot succeed.
        let top = rem.get(j + n);
        let (mut estimate, mut rest, mut rest_overflowed) = if top == high {
          let (rest, overflowed) = rem.get(j + n - 1).overflowing_add(high);
          (Limb::MAX, rest, overflowed)
        } else {
          let (estimate, rest) = $div_rem_wide(top, rem.get(j + n - 1), high);
          (estimate, rest, false)
        };
        // Corrects while `estimate * next > rest * 2^B + rem[j + n - 2]`, compared limb by limb.
        while !rest_overflowed {
          let (product_low, product_high) = $carrying_mul_add(estimate, next, 0, 0);
          if product_high < rest || (product_high == rest && product_low <= rem.get(j + n - 2)) {
            break;
          }
          estimate -= 1;
          (rest, rest_overflowed) = rest.overflowing_add(high);
        }

        // D4: subtract `estimate * divisor` from the window.
        let mut carry: Limb = 0;
        let mut borrow = false;
        let mut i = 0;
        while i < n {
          let (product_low, product_high) = $carrying_mul_add(estimate, divisor[i], carry, 0);
          carry = product_high;
          let (limb, first) = rem.get(j + i).overflowing_sub(product_low);
          let (limb, second) = limb.overflowing_sub(borrow as Limb);
          rem.set(j + i, limb);
          borrow = first || second;
          i += 1;
        }
        let (limb, first) = rem.get(j + n).overflowing_sub(carry);
        let (limb, second) = limb.overflowing_sub(borrow as Limb);
        rem.set(j + n, limb);

        // D5, D6: a borrow out of the window means that the estimate was one too large, which the
        // divisor added back corrects. The carry out of the top limb cancels the borrow.
        if first || second {
          estimate -= 1;
          let mut carry = false;
          let mut i = 0;
          while i < n {
            let (limb, first) = rem.get(j + i).overflowing_add(divisor[i]);
            let (limb, second) = limb.overflowing_add(carry as Limb);
            rem.set(j + i, limb);
            carry = first || second;
            i += 1;
          }
          rem.set(j + n, rem.get(j + n).wrapping_add(carry as Limb));
        }
        quotient[j] = estimate;
      }
      debug_assert!(rem.top == 0, "the remainder exceeds the divisor");
      // D8: unnormalize the remainder, which is below the normalized divisor.
      (quotient, shr(&rem.limbs, shift))
    }

    /// Returns `true` if `mode` rounds a quotient one step away from zero, given the magnitudes of
    /// the quotient and the remainder of a division rounded toward zero and of the divisor, and
    /// whether the dividend is negative. Requires `remainder < divisor`.
    pub(crate) const fn rounds_away<const N: usize>(
      quotient: &[Limb; N],
      remainder: &[Limb; N],
      divisor: &[Limb; N],
      negative: bool,
      mode: RoundingMode,
    ) -> bool {
      // `remainder < divisor`, so nothing borrows.
      let (rest, _) = overflowing_sub(divisor, remainder);
      rounding::increments(
        mode,
        negative,
        is_zero(remainder),
        cmp(remainder, &rest),
        quotient[0] % 2 == 1,
      )
    }

    /// `value / divisor` rounded to an integer with `mode`, the remainder of the division rounded
    /// toward zero, and whether `mode` rounded the quotient away from zero. Requires
    /// `divisor != 0`.
    pub(crate) const fn round_div<const N: usize>(
      value: &[Limb; N],
      divisor: &[Limb; N],
      mode: RoundingMode,
    ) -> ([Limb; N], [Limb; N], bool) {
      let (quotient, remainder) = div_rem(value, divisor);
      let away = rounds_away(&quotient, &remainder, divisor, false, mode);
      // Rounding away needs a nonzero remainder, so the divisor is at least 2 and the quotient at
      // most half of `value`: adding one does not overflow.
      let rounded = if away {
        overflowing_add(&quotient, &one()).0
      } else {
        quotient
      };
      (rounded, remainder, away)
    }

    /// Writes the decimal digits of `value` to the end of `buf`, without leading zeros (zero is a
    /// single `0`), and returns the index of the first digit. Requires `value` to have at most
    /// `DIGIT_BUF_LEN` digits, which holds for `B * N <= 512`.
    #[inline]
    pub(crate) const fn write_digits<const N: usize>(
      value: &[Limb; N],
      buf: &mut DigitBuf,
    ) -> usize {
      write_digits_into(value, buf)
    }

    /// Writes the decimal digits of `value` to the end of `buf`, without leading zeros (zero is a
    /// single `0`), and returns the index of the first digit. Requires `buf` to hold every digit
    /// of `value`, which `DIGITS_PER_LIMB * N` bytes do.
    pub(crate) const fn write_digits_into<const N: usize>(
      value: &[Limb; N],
      buf: &mut [u8],
    ) -> usize {
      let mut value = *value;
      let mut start = buf.len();
      // Divides off `MAX_STEP` digits at a time, least significant first.
      loop {
        let (quotient, mut chunk) = div_rem_limb(&value, TEN_POW);
        value = quotient;
        if is_zero(&value) {
          // The most significant chunk, without leading zeros.
          loop {
            start -= 1;
            buf[start] = b'0' + (chunk % 10) as u8;
            chunk /= 10;
            if chunk == 0 {
              return start;
            }
          }
        }
        // A less significant chunk has all `MAX_STEP` digits, including its leading zeros.
        let mut written = 0;
        while written < MAX_STEP {
          start -= 1;
          buf[start] = b'0' + (chunk % 10) as u8;
          chunk /= 10;
          written += 1;
        }
      }
    }

    /// `value * scale + chunk`. Requires a representable result.
    const fn append<const N: usize>(value: &[Limb; N], scale: Limb, chunk: Limb) -> [Limb; N] {
      let (appended, carry) = mul_add_limb(value, scale, chunk);
      debug_assert!(carry == 0, "appending digits overflowed");
      appended
    }

    /// Appends the ASCII digits in `bytes[start..end]` to `value`, computing `value * 10 + d` for
    /// each digit `d` and skipping every other byte, such as the separators `_` and `.`. Requires a
    /// representable result.
    pub(crate) const fn push_digits<const N: usize>(
      value: &[Limb; N],
      bytes: &[u8],
      start: usize,
      end: usize,
    ) -> [Limb; N] {
      let mut value = *value;
      // Gathers up to `MAX_STEP` digits in a limb and appends them with one multi-limb
      // multiplication. Each intermediate value is a prefix of the result, so it is representable
      // as well.
      let mut chunk: Limb = 0;
      let mut scale: Limb = 1;
      let mut i = start;
      while i < end {
        let byte = bytes[i];
        if byte.is_ascii_digit() {
          chunk = chunk * 10 + (byte - b'0') as Limb;
          scale *= 10;
          if scale == TEN_POW {
            value = append(&value, scale, chunk);
            chunk = 0;
            scale = 1;
          }
        }
        i += 1;
      }
      append(&value, scale, chunk)
    }

    /// Returns `true` if `value < 2^bits`: every bit at index `bits` or above is zero.
    pub(crate) const fn fits_bits<const N: usize>(value: &[Limb; N], bits: u32) -> bool {
      let mut i = (bits / Limb::BITS) as usize;
      if i >= N {
        return true;
      }
      // In the limb at index `bits / B`, only the bits below index `bits` may be set, and in the
      // limbs above it, none.
      if value[i] >> (bits % Limb::BITS) != 0 {
        return false;
      }
      i += 1;
      while i < N {
        if value[i] != 0 {
          return false;
        }
        i += 1;
      }
      true
    }

    /// `2^bits - 1`, the largest value of `bits` bits. Requires `bits <= B * N`.
    pub(crate) const fn max_of_bits<const N: usize>(bits: u32) -> [Limb; N] {
      let mut max = [0; N];
      let full = (bits / Limb::BITS) as usize;
      let mut i = 0;
      while i < full {
        max[i] = Limb::MAX;
        i += 1;
      }
      if full < N {
        // The shift is below `B`, so it does not overflow, and a shift of zero leaves the
        // limb zero.
        max[full] = (1 << (bits % Limb::BITS)) - 1;
      }
      max
    }

    /// `value` modulo `2^bits`: every bit at index `bits` or above cleared. Requires
    /// `bits <= B * N`.
    pub(crate) const fn zero_extend<const N: usize>(value: &[Limb; N], bits: u32) -> [Limb; N] {
      let mut extended = *value;
      let mut i = (bits / Limb::BITS) as usize;
      if i < N {
        // The shift is below `B`, so it does not overflow, and a shift of zero clears the limb.
        extended[i] &= (1 << (bits % Limb::BITS)) - 1;
        i += 1;
        while i < N {
          extended[i] = 0;
          i += 1;
        }
      }
      extended
    }

    /// `lhs + rhs` modulo `2^bits`, and whether the sum is `2^bits` or more. Requires
    /// `bits <= B * N` and both values below `2^bits`.
    pub(crate) const fn overflowing_add_bits<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
      bits: u32,
    ) -> ([Limb; N], bool) {
      let (raw, carry) = overflowing_add(lhs, rhs);
      (zero_extend(&raw, bits), carry || !fits_bits(&raw, bits))
    }

    /// `lhs - rhs` modulo `2^bits`, and whether `rhs > lhs`. Requires `bits <= B * N` and both
    /// values below `2^bits`.
    pub(crate) const fn overflowing_sub_bits<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
      bits: u32,
    ) -> ([Limb; N], bool) {
      let (raw, borrow) = overflowing_sub(lhs, rhs);
      (zero_extend(&raw, bits), borrow || !fits_bits(&raw, bits))
    }

    /// `lhs * rhs` modulo `2^bits`, and whether the product is `2^bits` or more. Requires
    /// `bits <= B * N` and both values below `2^bits`.
    pub(crate) const fn overflowing_mul_bits<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
      bits: u32,
    ) -> ([Limb; N], bool) {
      let (raw, overflow) = overflowing_mul(lhs, rhs);
      (zero_extend(&raw, bits), overflow || !fits_bits(&raw, bits))
    }

    /// `-value` modulo `2^bits`, and whether `value` is not zero. Requires `bits <= B * N` and
    /// `value` below `2^bits`.
    pub(crate) const fn overflowing_neg_bits<const N: usize>(
      value: &[Limb; N],
      bits: u32,
    ) -> ([Limb; N], bool) {
      let (raw, borrow) = overflowing_sub(&[0; N], value);
      (zero_extend(&raw, bits), borrow || !fits_bits(&raw, bits))
    }

    /// `lhs & rhs`.
    pub(crate) const fn and<const N: usize>(lhs: &[Limb; N], rhs: &[Limb; N]) -> [Limb; N] {
      let mut result = [0; N];
      let mut i = 0;
      while i < N {
        result[i] = lhs[i] & rhs[i];
        i += 1;
      }
      result
    }

    /// `lhs ^ rhs`.
    pub(crate) const fn xor<const N: usize>(lhs: &[Limb; N], rhs: &[Limb; N]) -> [Limb; N] {
      let mut result = [0; N];
      let mut i = 0;
      while i < N {
        result[i] = lhs[i] ^ rhs[i];
        i += 1;
      }
      result
    }

    /// `2^exp`. Requires `exp < B * N`.
    pub(crate) const fn pow2<const N: usize>(exp: u32) -> [Limb; N] {
      let mut power = [0; N];
      power[(exp / Limb::BITS) as usize] = 1 << (exp % Limb::BITS);
      power
    }

    /// `value` in limbs. Requires `value < 2^(B * N)`.
    const fn from_u32<const N: usize>(value: u32) -> [Limb; N] {
      let mut limbs = [0; N];
      let mut rest = value;
      let mut i = 0;
      while rest != 0 {
        limbs[i] = rest as Limb;
        // A limb of 32 bits or more takes all of `value`, which a shift by the width of the limb
        // would overflow.
        rest = match rest.checked_shr(Limb::BITS) {
          Some(rest) => rest,
          None => 0,
        };
        i += 1;
      }
      limbs
    }

    /// The base-2 logarithm of `value` rounded down, the index of its highest set bit, or `None`
    /// for zero. Requires that index to be below `2^32`, as it is in every value of a width.
    pub(crate) const fn checked_ilog2<const N: usize>(value: &[Limb; N]) -> Option<u32> {
      match top_limb(value) {
        Some(i) => Some(i as u32 * Limb::BITS + value[i].ilog2()),
        None => None,
      }
    }

    /// `floor(log10(2) * 2^64)`.
    const LOG10_2: u128 = 0x4d10_4d42_7de7_fbcc;

    /// The base-10 logarithm of `value` rounded down, or `None` for zero.
    pub(crate) const fn checked_ilog10<const N: usize>(value: &[Limb; N]) -> Option<u32> {
      let log2 = match checked_ilog2(value) {
        Some(log2) => log2,
        None => return None,
      };
      // With `e = floor(log2 * log10(2))`, `10^e <= 2^log2 <= value < 2^(log2 + 1) < 10^(e + 2)`,
      // so the logarithm is `e` or `e + 1`. The estimate is `e` or `e - 1`: the error of `LOG10_2`
      // times `log2 < 2^32` is below `2^-32`. No more than two products by powers of ten correct
      // it, and none divides.
      let mut log = ((log2 as u128 * LOG10_2) >> 64) as u32;
      while matches!(checked_mul_pow10(&one(), log + 1), Some(power) if le(&power, value)) {
        log += 1;
      }
      Some(log)
    }

    /// The logarithm of `value` to the base `base`, rounded down, or `None` if `value` is zero or
    /// `base` is below 2.
    pub(crate) const fn checked_ilog<const N: usize>(
      value: &[Limb; N],
      base: &[Limb; N],
    ) -> Option<u32> {
      let (log2, base_log2) = match (checked_ilog2(value), checked_ilog2(base)) {
        (Some(log2), Some(base_log2)) if base_log2 >= 1 => (log2, base_log2),
        _ => return None,
      };
      if matches!(cmp(value, base), Ordering::Less) {
        return Some(0);
      }
      // A power of two, `2^k`: the logarithm `m` has `k * m <= log2 < k * (m + 1)`.
      if eq(base, &pow2(base_log2)) {
        return Some(log2 / base_log2);
      }
      let mut ten = [0; N];
      ten[0] = 10;
      if eq(base, &ten) {
        return checked_ilog10(value);
      }
      // The power of the base reached, and its exponent, the logarithm once no further product
      // stays at most `value`.
      let mut power = one::<N>();
      let mut log = 0;
      if base_log2 < Limb::BITS {
        // A base of one limb, `b`, which is neither 2 nor 10 here: products by `b^m`, the largest
        // power of `b` in a limb, and then by `b`, each a product by one limb.
        let b = base[0];
        let m = Limb::MAX.ilog(b);
        let (mut factor, mut exp) = (b.pow(m), m);
        loop {
          match mul_add_limb(&power, factor, 0) {
            (product, 0) if le(&product, value) => {
              power = product;
              log += exp;
            }
            _ if exp > 1 => (factor, exp) = (b, 1),
            _ => return Some(log),
          }
        }
      }
      // A wider base, of at least `B` bits: at most `N` products.
      loop {
        match overflowing_mul(&power, base) {
          (product, false) if le(&product, value) => {
            power = product;
            log += 1;
          }
          _ => return Some(log),
        }
      }
    }

    /// `(lhs + rhs) / 2` rounded down, which cannot overflow.
    pub(crate) const fn midpoint<const N: usize>(lhs: &[Limb; N], rhs: &[Limb; N]) -> [Limb; N] {
      // `lhs + rhs = 2 * (lhs & rhs) + (lhs ^ rhs)`.
      overflowing_add(&and(lhs, rhs), &shr(&xor(lhs, rhs), 1)).0
    }

    /// `|lhs - rhs|`.
    pub(crate) const fn abs_diff<const N: usize>(lhs: &[Limb; N], rhs: &[Limb; N]) -> [Limb; N] {
      if matches!(cmp(lhs, rhs), Ordering::Less) {
        overflowing_sub(rhs, lhs).0
      } else {
        overflowing_sub(lhs, rhs).0
      }
    }

    /// The quotient and the remainder of `lhs / rhs`, and `false`, which no unsigned quotient
    /// overflows, or `None` if `rhs` is zero. The quotient and the remainder are at most `lhs`, so
    /// `bits` does not change them; it gives every width the signature of the upper layer.
    pub(crate) const fn div_rem_bits<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
      _bits: u32,
    ) -> Option<([Limb; N], [Limb; N], bool)> {
      // A zero divisor never reaches `div_rem`, which would index below zero with no limbs.
      if is_zero(rhs) {
        return None;
      }
      let (quotient, remainder) = div_rem(lhs, rhs);
      Some((quotient, remainder, false))
    }

    /// The Euclidean quotient and remainder of `lhs / rhs`, those of the division rounded toward
    /// zero for unsigned values, as [`div_rem_bits`] gives them.
    #[inline]
    pub(crate) const fn div_rem_euclid_bits<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
      bits: u32,
    ) -> Option<([Limb; N], [Limb; N], bool)> {
      div_rem_bits(lhs, rhs, bits)
    }

    /// `lhs / rhs` rounded down, and `false`, or `None` if `rhs` is zero.
    pub(crate) const fn div_floor_bits<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
      bits: u32,
    ) -> Option<([Limb; N], bool)> {
      match div_rem_bits(lhs, rhs, bits) {
        Some((quotient, _, overflow)) => Some((quotient, overflow)),
        None => None,
      }
    }

    /// `lhs / rhs` rounded up, and `false`, or `None` if `rhs` is zero.
    pub(crate) const fn div_ceil_bits<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
      bits: u32,
    ) -> Option<([Limb; N], bool)> {
      match div_rem_bits(lhs, rhs, bits) {
        // A nonzero remainder needs `rhs >= 2`, so the quotient is at most half of `lhs` and one
        // more does not overflow.
        Some((quotient, remainder, _)) if !is_zero(&remainder) => {
          Some((overflowing_add(&quotient, &one()).0, false))
        }
        Some((quotient, _, overflow)) => Some((quotient, overflow)),
        None => None,
      }
    }

    /// The smallest multiple of `rhs` at least `lhs`, wrapped to `bits` bits, and whether it is
    /// `2^bits` or more, or `None` if `rhs` is zero. Requires `bits <= B * N` and both values below
    /// `2^bits`.
    pub(crate) const fn next_multiple_of_bits<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
      bits: u32,
    ) -> Option<([Limb; N], bool)> {
      if is_zero(rhs) {
        return None;
      }
      let remainder = div_rem(lhs, rhs).1;
      if is_zero(&remainder) {
        return Some((*lhs, false));
      }
      // `rhs - remainder` is below `rhs`, so it does not borrow and has `bits` bits.
      Some(overflowing_add_bits(
        lhs,
        &overflowing_sub(rhs, &remainder).0,
        bits,
      ))
    }

    /// Returns `true` if `lhs` is a multiple of `rhs`: if `lhs / rhs` leaves no remainder, or if
    /// both are zero.
    pub(crate) const fn is_multiple_of<const N: usize>(lhs: &[Limb; N], rhs: &[Limb; N]) -> bool {
      if is_zero(rhs) {
        is_zero(lhs)
      } else {
        is_zero(&div_rem(lhs, rhs).1)
      }
    }

    /// `lhs / rhs` if `rhs` divides `lhs`, or `None` if `rhs` is zero or the division leaves a
    /// remainder.
    pub(crate) const fn checked_div_exact_bits<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
      bits: u32,
    ) -> Option<[Limb; N]> {
      match div_rem_bits(lhs, rhs, bits) {
        Some((quotient, remainder, false)) if is_zero(&remainder) => Some(quotient),
        _ => None,
      }
    }

    /// `base^exp` modulo `2^(B * N)`, and whether it is `2^(B * N)` or more. Requires `N >= 1`.
    pub(crate) const fn overflowing_pow<const N: usize>(
      base: &[Limb; N],
      mut exp: u32,
    ) -> ([Limb; N], bool) {
      if exp == 0 {
        return (one(), false);
      }
      // Square-and-multiply from the lowest bit of `exp`. Every square is a factor of the power,
      // as the last bit needs none after it, so for a base of at least 2 an overflowing square
      // overflows the power, and the flag is exact; smaller bases never overflow.
      let mut base = *base;
      let mut power = one::<N>();
      let mut overflow = false;
      loop {
        if exp & 1 == 1 {
          let (product, carried) = overflowing_mul(&power, &base);
          power = product;
          overflow |= carried;
          if exp == 1 {
            return (power, overflow);
          }
        }
        exp >>= 1;
        let (square, carried) = overflowing_mul(&base, &base);
        base = square;
        overflow |= carried;
      }
    }

    /// `base^exp` modulo `2^bits`, and whether it is `2^bits` or more. Requires `bits <= B * N`
    /// and `base < 2^bits`.
    pub(crate) const fn overflowing_pow_bits<const N: usize>(
      base: &[Limb; N],
      exp: u32,
      bits: u32,
    ) -> ([Limb; N], bool) {
      // No limbs hold zero bits, where `base^0 = 1` overflows.
      if N == 0 {
        return ([0; N], exp == 0);
      }
      // The full width holds one, so the power wraps to the width at the end, as for the native
      // integers.
      let (raw, overflow) = overflowing_pow(base, exp);
      (zero_extend(&raw, bits), overflow || !fits_bits(&raw, bits))
    }

    /// The square root of `value`, rounded down.
    pub(crate) const fn isqrt<const N: usize>(value: &[Limb; N]) -> [Limb; N] {
      let log2 = match checked_ilog2(value) {
        Some(log2) => log2,
        None => return [0; N],
      };
      if log2 < Limb::BITS {
        let mut root = [0; N];
        root[0] = value[0].isqrt();
        return root;
      }
      // Newton's method from above the root: `(2^(log2 / 2 + 1))^2 >= 2^(log2 + 1) > value`. Each
      // step `x - ceil((x - q) / 2)`, which is `floor((x + q) / 2)` for `q = floor(value / x)`,
      // leaves `x` at or above the root and decreases it while it is above, so the first `x` with
      // `q >= x` is the root.
      let mut x = pow2::<N>(log2 / 2 + 1);
      loop {
        let quotient = div_rem(value, &x).0;
        if le(&x, &quotient) {
          return x;
        }
        let gap = overflowing_sub(&overflowing_sub(&x, &quotient).0, &one()).0;
        x = overflowing_sub(&overflowing_sub(&x, &shr(&gap, 1)).0, &one()).0;
      }
    }

    /// The square root of `value`, rounded down, which every unsigned value has.
    #[inline]
    pub(crate) const fn checked_isqrt<const N: usize>(value: &[Limb; N]) -> Option<[Limb; N]> {
      Some(isqrt(value))
    }

    /// The `n`th root of `value`, rounded down. Requires `n >= 1`.
    pub(crate) const fn nth_root<const N: usize>(value: &[Limb; N], n: u32) -> [Limb; N] {
      let log2 = match checked_ilog2(value) {
        Some(log2) => log2,
        None => return [0; N],
      };
      if n == 1 {
        return *value;
      }
      // `value < 2^(log2 + 1) <= 2^n`.
      if n > log2 {
        return one();
      }
      // `n <= log2 < B * N`.
      let degree = from_u32::<N>(n);
      // Newton's method from above the root, as in `isqrt`: `x0^n >= 2^(log2 + 1) > value`, and
      // each step is `floor(((n - 1) * x + q) / n)`, written as `x - (x - q - 1) / n - 1`.
      let mut x = pow2::<N>(log2 / n + 1);
      loop {
        // `value / x^(n - 1)`, which is zero when the power has more bits than `value`. With
        // `2 <= n <= log2`, `x <= 2^log2 < 2^(log2 + 1)`, the width that the power takes.
        let (power, overflow) = overflowing_pow_bits(&x, n - 1, log2 + 1);
        let quotient = if overflow {
          [0; N]
        } else {
          div_rem(value, &power).0
        };
        if le(&x, &quotient) {
          return x;
        }
        let gap = overflowing_sub(&overflowing_sub(&x, &quotient).0, &one()).0;
        x = overflowing_sub(&overflowing_sub(&x, &div_rem(&gap, &degree).0).0, &one()).0;
      }
    }

    /// The `n`th root of `value`, rounded down, or `None` if `n` is zero.
    pub(crate) const fn checked_nth_root<const N: usize>(
      value: &[Limb; N],
      n: u32,
    ) -> Option<[Limb; N]> {
      if n == 0 {
        None
      } else {
        Some(nth_root(value, n))
      }
    }

    /// The smallest power of two at least `value`, wrapped to `bits` bits, and whether it is
    /// `2^bits` or more, as it is at zero bits, where the power of two of zero is one. Requires
    /// `bits <= B * N` and `value < 2^bits`.
    pub(crate) const fn next_power_of_two_bits<const N: usize>(
      value: &[Limb; N],
      bits: u32,
    ) -> ([Limb; N], bool) {
      let exp = match checked_ilog2(value) {
        // `2^log2 <= value < 2^(log2 + 1)`.
        Some(log2) if eq(value, &pow2(log2)) => log2,
        Some(log2) => log2 + 1,
        None => 0,
      };
      // `2^exp` wraps to zero at `bits <= exp`.
      if exp < bits {
        (pow2(exp), false)
      } else {
        ([0; N], true)
      }
    }

    /// Zero-extends or truncates `value` to `M` limbs.
    pub(crate) const fn resize<const N: usize, const M: usize>(value: &[Limb; N]) -> [Limb; M] {
      let mut resized = [0; M];
      let mut i = 0;
      while i < N && i < M {
        resized[i] = value[i];
        i += 1;
      }
      resized
    }

    /// `value` in `M` limbs, or `None` if it does not fit: a limb that truncation drops is not
    /// zero.
    pub(crate) const fn checked_resize<const N: usize, const M: usize>(
      value: &[Limb; N],
    ) -> Option<[Limb; M]> {
      let resized = resize::<N, M>(value);
      // Extending back restores every limb exactly when no dropped limb was nonzero.
      if eq(&resize::<M, N>(&resized), value) {
        Some(resized)
      } else {
        None
      }
    }
  };
  (signed limb: $limb:ident, signed: $signed:ident, unsigned: $unsigned:ident) => {
    use ::core::cmp::Ordering;

    use $crate::int::unsigned::$unsigned;
    use $crate::{int::DigitBuf, rounding::RoundingMode};

    /// The type of a limb: an unsigned integer of `B` bits.
    type Limb = $limb;

    /// The signed integer of `B` bits, which reads the top limb with its sign.
    type SignedLimb = $signed;

    #[inline]
    pub(crate) const fn is_negative<const N: usize>(value: &[Limb; N]) -> bool {
      // No limbs hold only zero, and have no top limb.
      N > 0 && (value[N - 1] as SignedLimb) < 0
    }

    #[inline]
    pub(crate) const fn is_zero<const N: usize>(value: &[Limb; N]) -> bool {
      $unsigned::is_zero(value)
    }

    #[inline]
    pub(crate) const fn is_positive<const N: usize>(value: &[Limb; N]) -> bool {
      !is_negative(value) && !is_zero(value)
    }

    /// Returns `true` if `value` is one, which no limbs hold.
    #[inline]
    pub(crate) const fn is_one<const N: usize>(value: &[Limb; N]) -> bool {
      $unsigned::is_one(value)
    }

    /// `-1`, `0` or `1` as the sign of `value`.
    pub(crate) const fn signum<const N: usize>(value: &[Limb; N]) -> [Limb; N] {
      if is_negative(value) {
        [Limb::MAX; N]
      } else if $unsigned::is_zero(value) {
        [0; N]
      } else {
        // A positive value has a limb, which `one` sets.
        $unsigned::one()
      }
    }

    /// Compares `lhs` and `rhs`: the top limbs as `SignedLimb`, then the others as `Limb`, most
    /// significant first.
    pub(crate) const fn cmp<const N: usize>(lhs: &[Limb; N], rhs: &[Limb; N]) -> Ordering {
      // No limbs hold only zero, and have no top limb.
      if N == 0 {
        return Ordering::Equal;
      }
      let lhs_top = lhs[N - 1] as SignedLimb;
      let rhs_top = rhs[N - 1] as SignedLimb;
      if lhs_top < rhs_top {
        Ordering::Less
      } else if lhs_top > rhs_top {
        Ordering::Greater
      } else {
        $unsigned::cmp_below(lhs, rhs, N - 1)
      }
    }

    #[inline]
    pub(crate) const fn le<const N: usize>(lhs: &[Limb; N], rhs: &[Limb; N]) -> bool {
      !matches!(cmp(lhs, rhs), Ordering::Greater)
    }

    /// `lhs + rhs`, or `None` if the sum is not representable.
    pub(crate) const fn checked_add<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
    ) -> Option<[Limb; N]> {
      let (sum, _) = $unsigned::overflowing_add(lhs, rhs);
      // The sum overflows exactly when the operands have the same sign and the wrapped sum
      // the other.
      if is_negative(lhs) == is_negative(rhs) && is_negative(&sum) != is_negative(lhs) {
        None
      } else {
        Some(sum)
      }
    }

    /// `lhs - rhs`, or `None` if the difference is not representable.
    pub(crate) const fn checked_sub<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
    ) -> Option<[Limb; N]> {
      let (difference, _) = $unsigned::overflowing_sub(lhs, rhs);
      // The difference overflows exactly when the operands have different signs and the wrapped
      // difference has the sign of `rhs`.
      if is_negative(lhs) != is_negative(rhs) && is_negative(&difference) != is_negative(lhs) {
        None
      } else {
        Some(difference)
      }
    }

    /// `-value` modulo `2^(B * N)`: `!value + 1`, which maps `MIN` to itself.
    const fn wrapping_neg<const N: usize>(value: &[Limb; N]) -> [Limb; N] {
      let mut negated = [0; N];
      let mut carry = true;
      let mut i = 0;
      while i < N {
        let (limb, overflow) = (!value[i]).overflowing_add(carry as Limb);
        negated[i] = limb;
        carry = overflow;
        i += 1;
      }
      negated
    }

    /// `-value`. Requires `value != MIN`.
    #[inline]
    pub(crate) const fn neg<const N: usize>(value: &[Limb; N]) -> [Limb; N] {
      let negated = wrapping_neg(value);
      debug_assert!(
        $unsigned::is_zero(value) || is_negative(value) != is_negative(&negated),
        "negation overflowed"
      );
      negated
    }

    /// `|value|`. Requires `value != MIN`.
    #[inline]
    pub(crate) const fn abs<const N: usize>(value: &[Limb; N]) -> [Limb; N] {
      if is_negative(value) {
        neg(value)
      } else {
        *value
      }
    }

    /// `|value|` as an unsigned integer, which also holds `|MIN| = 2^(B * N - 1)`.
    #[inline]
    pub(crate) const fn unsigned_abs<const N: usize>(value: &[Limb; N]) -> [Limb; N] {
      if is_negative(value) {
        wrapping_neg(value)
      } else {
        *value
      }
    }

    /// The value with the magnitude `magnitude`, negated if `negative`, or `None` if it is not
    /// representable.
    const fn checked_from_magnitude<const N: usize>(
      magnitude: &[Limb; N],
      negative: bool,
    ) -> Option<[Limb; N]> {
      if !negative {
        return if is_negative(magnitude) {
          None
        } else {
          Some(*magnitude)
        };
      }
      // A magnitude from 1 to `2^(B * N - 1)` negates to a value with the sign bit set.
      let negated = wrapping_neg(magnitude);
      if $unsigned::is_zero(magnitude) || is_negative(&negated) {
        Some(negated)
      } else {
        None
      }
    }

    /// The value with the magnitude `magnitude`, negated if `negative`. Requires it to be
    /// representable.
    #[inline]
    const fn from_magnitude<const N: usize>(magnitude: &[Limb; N], negative: bool) -> [Limb; N] {
      debug_assert!(
        checked_from_magnitude(magnitude, negative).is_some(),
        "signed result overflowed"
      );
      if negative {
        wrapping_neg(magnitude)
      } else {
        *magnitude
      }
    }

    /// `10^exp`. Requires `10^exp` to be representable.
    #[inline]
    pub(crate) const fn pow10<const N: usize>(exp: u32) -> [Limb; N] {
      from_magnitude(&$unsigned::pow10(exp), false)
    }

    /// `10^digits - 1`, the largest magnitude with `digits` decimal digits. Requires `10^digits`
    /// to be representable.
    #[inline]
    pub(crate) const fn max_magnitude<const N: usize>(digits: u32) -> [Limb; N] {
      from_magnitude(&$unsigned::max_magnitude(digits), false)
    }

    /// `lhs * rhs`. Requires the product to be representable.
    #[inline]
    pub(crate) const fn mul<const N: usize>(lhs: &[Limb; N], rhs: &[Limb; N]) -> [Limb; N] {
      let magnitude = $unsigned::mul(&unsigned_abs(lhs), &unsigned_abs(rhs));
      from_magnitude(&magnitude, is_negative(lhs) != is_negative(rhs))
    }

    /// `value * 10^exp`. Requires the product to be representable.
    #[inline]
    pub(crate) const fn mul_pow10<const N: usize>(value: &[Limb; N], exp: u32) -> [Limb; N] {
      from_magnitude(
        &$unsigned::mul_pow10(&unsigned_abs(value), exp),
        is_negative(value),
      )
    }

    /// `value * 10^exp`, or `None` if the product is not representable.
    pub(crate) const fn checked_mul_pow10<const N: usize>(
      value: &[Limb; N],
      exp: u32,
    ) -> Option<[Limb; N]> {
      match $unsigned::checked_mul_pow10(&unsigned_abs(value), exp) {
        Some(magnitude) => checked_from_magnitude(&magnitude, is_negative(value)),
        None => None,
      }
    }

    /// Quotient and remainder of `value / 10^exp`, rounding toward zero. As with `/` and `%`, both
    /// have the sign of `value`.
    pub(crate) const fn div_rem_pow10<const N: usize>(
      value: &[Limb; N],
      exp: u32,
    ) -> ([Limb; N], [Limb; N]) {
      let (quotient, remainder) = $unsigned::div_rem_pow10(&unsigned_abs(value), exp);
      // Both magnitudes are at most `|value|`, so each is exact with the sign of `value`.
      let negative = is_negative(value);
      (
        from_magnitude(&quotient, negative),
        from_magnitude(&remainder, negative),
      )
    }

    /// Quotient and remainder of `lhs / rhs`, rounding toward zero. As with `/` and `%`, the
    /// remainder has the sign of `lhs`. Requires `rhs != 0`, and `rhs != -1` if `lhs == MIN`.
    pub(crate) const fn div_rem<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
    ) -> ([Limb; N], [Limb; N]) {
      // The magnitudes are unsigned, which also holds `|MIN|`. The quotient's magnitude is at
      // most `|lhs|`, and the remainder's below `|rhs|`.
      let (quotient, remainder) = $unsigned::div_rem(&unsigned_abs(lhs), &unsigned_abs(rhs));
      (
        from_magnitude(&quotient, is_negative(lhs) != is_negative(rhs)),
        from_magnitude(&remainder, is_negative(lhs)),
      )
    }

    /// `value / divisor` rounded to an integer with `mode`, the remainder of the division rounded
    /// toward zero, which has the sign of `value`, and whether `mode` rounded the quotient away
    /// from zero. Requires `divisor > 0`.
    pub(crate) const fn round_div<const N: usize>(
      value: &[Limb; N],
      divisor: &[Limb; N],
      mode: RoundingMode,
    ) -> ([Limb; N], [Limb; N], bool) {
      let (quotient, remainder) = div_rem(value, divisor);
      // The sign of the value, which the quotient loses when it is zero. A positive divisor is its
      // own magnitude.
      let negative = is_negative(value);
      let away = $unsigned::rounds_away(
        &unsigned_abs(&quotient),
        &unsigned_abs(&remainder),
        divisor,
        negative,
        mode,
      );
      // Rounding away needs a nonzero remainder, so the divisor is at least 2 and `|quotient|` at
      // most half of `|value|`: one step further from zero is representable, so the wrapping
      // arithmetic is exact.
      let rounded = if !away {
        quotient
      } else if negative {
        $unsigned::overflowing_sub(&quotient, &$unsigned::one()).0
      } else {
        $unsigned::overflowing_add(&quotient, &$unsigned::one()).0
      };
      (rounded, remainder, away)
    }

    /// Writes the decimal digits of `|value|` to the end of `buf`, without leading zeros (zero is a
    /// single `0`), and returns the index of the first digit. Requires `|value|` to have at most
    /// `DIGIT_BUF_LEN` digits, which holds for `B * N <= 512`.
    #[inline]
    pub(crate) const fn write_digits<const N: usize>(
      value: &[Limb; N],
      buf: &mut DigitBuf,
    ) -> usize {
      $unsigned::write_digits(&unsigned_abs(value), buf)
    }

    /// Appends the ASCII digits in `bytes[start..end]` to `value`, computing `value * 10 + d` for
    /// each digit `d` and skipping every other byte, such as the separators `_` and `.`. Requires
    /// `value` not to be negative and a representable result.
    #[inline]
    pub(crate) const fn push_digits<const N: usize>(
      value: &[Limb; N],
      bytes: &[u8],
      start: usize,
      end: usize,
    ) -> [Limb; N] {
      from_magnitude(&$unsigned::push_digits(value, bytes, start, end), false)
    }

    /// Sign-extends or truncates `value` to `M` limbs.
    pub(crate) const fn resize<const N: usize, const M: usize>(value: &[Limb; N]) -> [Limb; M] {
      let fill = if is_negative(value) { Limb::MAX } else { 0 };
      let mut resized = [fill; M];
      let mut i = 0;
      while i < N && i < M {
        resized[i] = value[i];
        i += 1;
      }
      resized
    }

    /// `value` in `M` limbs, or `None` if it is not representable in `M` limbs.
    pub(crate) const fn checked_resize<const N: usize, const M: usize>(
      value: &[Limb; N],
    ) -> Option<[Limb; M]> {
      let resized = resize::<N, M>(value);
      // Sign-extending back restores every limb exactly when truncation dropped only copies of the
      // sign bit that remains.
      if $unsigned::eq(&resize::<M, N>(&resized), value) {
        Some(resized)
      } else {
        None
      }
    }

    /// `value` with every bit at index `bits - 1` or above set to bit `bits - 1`: the two's
    /// complement integer of `bits` bits in the low bits of `value`, sign-extended to `N` limbs, or
    /// zero for zero bits. Requires `bits <= B * N`.
    pub(crate) const fn sign_extend<const N: usize>(value: &[Limb; N], bits: u32) -> [Limb; N] {
      // Zero bits hold only zero, and have no sign bit to copy.
      if bits == 0 {
        return [0; N];
      }
      let top = ((bits - 1) / Limb::BITS) as usize;
      // Shifting bit `bits - 1` to the top of its limb and back with an arithmetic shift copies it
      // into the bits above.
      let shift = Limb::BITS - 1 - (bits - 1) % Limb::BITS;
      let mut extended = *value;
      extended[top] = (((value[top] << shift) as SignedLimb) >> shift) as Limb;
      let fill = if (extended[top] as SignedLimb) < 0 {
        Limb::MAX
      } else {
        0
      };
      let mut i = top + 1;
      while i < N {
        extended[i] = fill;
        i += 1;
      }
      extended
    }

    /// Returns `true` if `value` is within the range of `bits` bits, from `-2^(bits - 1)` to
    /// `2^(bits - 1) - 1`, or is zero for zero bits. Requires `bits <= B * N`.
    #[inline]
    pub(crate) const fn fits_bits<const N: usize>(value: &[Limb; N], bits: u32) -> bool {
      // Sign-extending from bit `bits - 1` changes only a value outside the range.
      $unsigned::eq(&sign_extend(value, bits), value)
    }

    /// `2^(bits - 1) - 1`, the largest value of `bits` bits, or zero for zero bits. Requires
    /// `bits <= B * N`.
    #[inline]
    pub(crate) const fn max_of_bits<const N: usize>(bits: u32) -> [Limb; N] {
      if bits == 0 {
        [0; N]
      } else {
        $unsigned::max_of_bits(bits - 1)
      }
    }

    /// `-2^(bits - 1)`, the smallest value of `bits` bits, or zero for zero bits. Requires
    /// `bits <= B * N`.
    pub(crate) const fn min_of_bits<const N: usize>(bits: u32) -> [Limb; N] {
      // Complementing the largest value, which is zero at zero bits, would give -1.
      if bits == 0 {
        return [0; N];
      }
      // Every bit from index `bits - 1` up is set, and every bit below it is clear.
      let mut min = max_of_bits::<N>(bits);
      let mut i = 0;
      while i < N {
        min[i] = !min[i];
        i += 1;
      }
      min
    }

    /// `lhs + rhs` wrapped to `bits` bits, and whether the sum is outside the range of `bits`
    /// bits. Requires `bits <= B * N` and both values within that range.
    pub(crate) const fn overflowing_add_bits<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
      bits: u32,
    ) -> ([Limb; N], bool) {
      let (raw, _) = $unsigned::overflowing_add(lhs, rhs);
      // The sum overflows `B * N` bits when the operands have the same sign and the wrapped sum
      // the other.
      let overflow = is_negative(lhs) == is_negative(rhs) && is_negative(&raw) != is_negative(lhs);
      let wrapped = sign_extend(&raw, bits);
      (wrapped, overflow || !$unsigned::eq(&wrapped, &raw))
    }

    /// `lhs - rhs` wrapped to `bits` bits, and whether the difference is outside the range of
    /// `bits` bits. Requires `bits <= B * N` and both values within that range.
    pub(crate) const fn overflowing_sub_bits<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
      bits: u32,
    ) -> ([Limb; N], bool) {
      let (raw, _) = $unsigned::overflowing_sub(lhs, rhs);
      // The difference overflows `B * N` bits when the operands have different signs and the
      // wrapped difference has the sign of `rhs`.
      let overflow = is_negative(lhs) != is_negative(rhs) && is_negative(&raw) != is_negative(lhs);
      let wrapped = sign_extend(&raw, bits);
      (wrapped, overflow || !$unsigned::eq(&wrapped, &raw))
    }

    /// `lhs * rhs` wrapped to `bits` bits, and whether the product is outside the range of `bits`
    /// bits. Requires `bits <= B * N` and both values within that range.
    pub(crate) const fn overflowing_mul_bits<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
      bits: u32,
    ) -> ([Limb; N], bool) {
      let negative = is_negative(lhs) != is_negative(rhs);
      let (magnitude, carried) = $unsigned::overflowing_mul(&unsigned_abs(lhs), &unsigned_abs(rhs));
      // The wrapped product is the magnitude modulo `2^(B * N)` with the sign of the product, and
      // it overflows `B * N` bits when the magnitude does not fit them with that sign.
      let raw = if negative {
        wrapping_neg(&magnitude)
      } else {
        magnitude
      };
      let overflow = carried || checked_from_magnitude(&magnitude, negative).is_none();
      let wrapped = sign_extend(&raw, bits);
      (wrapped, overflow || !$unsigned::eq(&wrapped, &raw))
    }

    /// `-value` wrapped to `bits` bits, and whether `value` is the smallest value of `bits` bits,
    /// whose negation is outside the range. Requires `bits <= B * N` and `value` within that
    /// range.
    pub(crate) const fn overflowing_neg_bits<const N: usize>(
      value: &[Limb; N],
      bits: u32,
    ) -> ([Limb; N], bool) {
      let raw = wrapping_neg(value);
      // Only `MIN` of `B * N` bits, whose sign the negation keeps, negates out of range. Zero
      // keeps its sign as well, and does not overflow.
      let overflow = !$unsigned::is_zero(value) && is_negative(&raw) == is_negative(value);
      let wrapped = sign_extend(&raw, bits);
      (wrapped, overflow || !$unsigned::eq(&wrapped, &raw))
    }

    /// Returns `true` if `value` is -1, which has every bit set. No limbs hold only zero.
    const fn is_minus_one<const N: usize>(value: &[Limb; N]) -> bool {
      N > 0 && $unsigned::eq(value, &[Limb::MAX; N])
    }

    /// The quotient and the remainder of `lhs / rhs` rounded toward zero, the remainder with the
    /// sign of `lhs`, and whether the quotient is outside the range of `bits` bits, as it is only
    /// for `MIN / -1`, whose quotient wraps to `MIN` with the remainder zero; or `None` if `rhs` is
    /// zero. Requires `bits <= B * N` and both values within that range.
    pub(crate) const fn div_rem_bits<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
      bits: u32,
    ) -> Option<([Limb; N], [Limb; N], bool)> {
      if $unsigned::is_zero(rhs) {
        return None;
      }
      // `lhs / -1` is `-lhs`. At a partial width `MIN / -1` does not overflow the storage, so the
      // negation at the width detects it; at the full width `div_rem` would not accept it.
      if is_minus_one(rhs) {
        let (quotient, overflow) = overflowing_neg_bits(lhs, bits);
        return Some((quotient, [0; N], overflow));
      }
      let (quotient, remainder) = div_rem(lhs, rhs);
      Some((quotient, remainder, false))
    }

    /// The Euclidean quotient and remainder of `lhs / rhs`, the remainder never negative, and
    /// whether the quotient is outside the range of `bits` bits, as [`div_rem_bits`] gives them.
    pub(crate) const fn div_rem_euclid_bits<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
      bits: u32,
    ) -> Option<([Limb; N], [Limb; N], bool)> {
      match div_rem_bits(lhs, rhs, bits) {
        // A negative remainder needs `|rhs| >= 2`, so the quotient one step further from zero and
        // the remainder moved by `|rhs|`, between zero and `|rhs|`, are in range, and the wrapping
        // arithmetic is exact.
        Some((quotient, remainder, _)) if is_negative(&remainder) => Some(if is_negative(rhs) {
          (
            $unsigned::overflowing_add(&quotient, &$unsigned::one()).0,
            $unsigned::overflowing_sub(&remainder, rhs).0,
            false,
          )
        } else {
          (
            $unsigned::overflowing_sub(&quotient, &$unsigned::one()).0,
            $unsigned::overflowing_add(&remainder, rhs).0,
            false,
          )
        }),
        division => division,
      }
    }

    /// `lhs / rhs` rounded down, and whether it is outside the range of `bits` bits, as it is only
    /// for `MIN / -1`; or `None` if `rhs` is zero. Requires `bits <= B * N` and both values within
    /// that range.
    pub(crate) const fn div_floor_bits<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
      bits: u32,
    ) -> Option<([Limb; N], bool)> {
      match div_rem_bits(lhs, rhs, bits) {
        // The quotient rounded toward zero is one above the floor when the remainder, of the sign
        // of `lhs`, differs in sign from `rhs`; then `|rhs| >= 2`, and one less is in range.
        Some((quotient, remainder, _))
          if !$unsigned::is_zero(&remainder) && is_negative(&remainder) != is_negative(rhs) =>
        {
          Some(($unsigned::overflowing_sub(&quotient, &$unsigned::one()).0, false))
        }
        Some((quotient, _, overflow)) => Some((quotient, overflow)),
        None => None,
      }
    }

    /// `lhs / rhs` rounded up, and whether it is outside the range of `bits` bits, as it is only
    /// for `MIN / -1`; or `None` if `rhs` is zero. Requires `bits <= B * N` and both values within
    /// that range.
    pub(crate) const fn div_ceil_bits<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
      bits: u32,
    ) -> Option<([Limb; N], bool)> {
      match div_rem_bits(lhs, rhs, bits) {
        // The quotient rounded toward zero is one below the ceiling when the remainder has the
        // sign of `rhs`; then `|rhs| >= 2`, and one more is in range.
        Some((quotient, remainder, _))
          if !$unsigned::is_zero(&remainder) && is_negative(&remainder) == is_negative(rhs) =>
        {
          Some(($unsigned::overflowing_add(&quotient, &$unsigned::one()).0, false))
        }
        Some((quotient, _, overflow)) => Some((quotient, overflow)),
        None => None,
      }
    }

    /// For a positive `rhs` the smallest multiple of `rhs` at least `lhs`, and for a negative `rhs`
    /// the largest at most `lhs`, wrapped to `bits` bits, and whether it is outside the range of
    /// `bits` bits; or `None` if `rhs` is zero. Requires `bits <= B * N` and both values within
    /// that range.
    pub(crate) const fn next_multiple_of_bits<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
      bits: u32,
    ) -> Option<([Limb; N], bool)> {
      if $unsigned::is_zero(rhs) {
        return None;
      }
      // Every value is a multiple of -1, and `MIN / -1` would overflow the division below.
      if is_minus_one(rhs) {
        return Some((*lhs, false));
      }
      let remainder = div_rem(lhs, rhs).1;
      // The remainder of the division rounded toward negative infinity, which has the sign of
      // `rhs`: a remainder of the other sign plus `rhs`, which then differ in sign.
      let differs = !$unsigned::is_zero(&remainder) && is_negative(&remainder) != is_negative(rhs);
      let rounded = if differs {
        $unsigned::overflowing_add(&remainder, rhs).0
      } else {
        remainder
      };
      if $unsigned::is_zero(&rounded) {
        return Some((*lhs, false));
      }
      // `rounded` has the sign of `rhs` and a smaller magnitude, so `rhs - rounded` is exact and
      // within the range of `bits` bits.
      Some(overflowing_add_bits(
        lhs,
        &$unsigned::overflowing_sub(rhs, &rounded).0,
        bits,
      ))
    }

    /// `lhs / rhs` if `rhs` divides `lhs` and the quotient is within the range of `bits` bits, or
    /// `None` if `rhs` is zero, the division leaves a remainder, or `lhs` is `MIN` and `rhs` is -1.
    /// Requires `bits <= B * N` and both values within that range.
    pub(crate) const fn checked_div_exact_bits<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
      bits: u32,
    ) -> Option<[Limb; N]> {
      match div_rem_bits(lhs, rhs, bits) {
        Some((quotient, remainder, false)) if $unsigned::is_zero(&remainder) => Some(quotient),
        _ => None,
      }
    }

    /// `value >> 1`, an arithmetic shift: the sign bit stays.
    const fn sar1<const N: usize>(value: &[Limb; N]) -> [Limb; N] {
      let mut shifted = $unsigned::shr(value, 1);
      if N > 0 {
        shifted[N - 1] |= value[N - 1] & (1 << (Limb::BITS - 1));
      }
      shifted
    }

    /// `(lhs + rhs) / 2` rounded toward zero, which cannot overflow.
    pub(crate) const fn midpoint<const N: usize>(lhs: &[Limb; N], rhs: &[Limb; N]) -> [Limb; N] {
      // `lhs + rhs = 2 * (lhs & rhs) + (lhs ^ rhs)`, so this is the floor of the half sum. On
      // values sign-extended from a width, every step stays sign-extended.
      let odd = $unsigned::xor(lhs, rhs);
      let floor = $unsigned::overflowing_add(&$unsigned::and(lhs, rhs), &sar1(&odd)).0;
      // The floor of an odd negative sum is one below the quotient rounded toward zero.
      if is_negative(&floor) && odd[0] & 1 == 1 {
        $unsigned::overflowing_add(&floor, &$unsigned::one()).0
      } else {
        floor
      }
    }

    /// `|lhs - rhs|` as an unsigned integer, which holds it: the difference of two values within
    /// the range of `bits` bits is below `2^bits`.
    pub(crate) const fn abs_diff<const N: usize>(lhs: &[Limb; N], rhs: &[Limb; N]) -> [Limb; N] {
      if matches!(cmp(lhs, rhs), Ordering::Less) {
        $unsigned::overflowing_sub(rhs, lhs).0
      } else {
        $unsigned::overflowing_sub(lhs, rhs).0
      }
    }

    /// `base^exp` wrapped to `bits` bits, and whether it is outside the range of `bits` bits.
    /// Requires `bits <= B * N` and `base` within that range.
    pub(crate) const fn overflowing_pow_bits<const N: usize>(
      base: &[Limb; N],
      exp: u32,
      bits: u32,
    ) -> ([Limb; N], bool) {
      // No limbs hold zero bits, where `base^0 = 1` overflows.
      if N == 0 {
        return ([0; N], exp == 0);
      }
      // The full width holds one, so a base of magnitude one never overflows on its own square,
      // and the width decides at the end, as for the native integers: one bit, which holds no one,
      // overflows on `(-1)^0` and `(-1)^2` but not on `(-1)^3`.
      let negative = is_negative(base) && exp % 2 == 1;
      let (magnitude, carried) = $unsigned::overflowing_pow(&unsigned_abs(base), exp);
      let raw = if negative {
        wrapping_neg(&magnitude)
      } else {
        magnitude
      };
      let overflow = carried || checked_from_magnitude(&magnitude, negative).is_none();
      let wrapped = sign_extend(&raw, bits);
      (wrapped, overflow || !$unsigned::eq(&wrapped, &raw))
    }

    /// The square root of `value`, rounded down, or `None` if `value` is negative.
    pub(crate) const fn checked_isqrt<const N: usize>(value: &[Limb; N]) -> Option<[Limb; N]> {
      if is_negative(value) {
        None
      } else {
        Some($unsigned::isqrt(value))
      }
    }

    /// The `n`th root of `value`, rounded toward zero, or `None` if `n` is zero or `value` is
    /// negative and `n` even.
    pub(crate) const fn checked_nth_root<const N: usize>(
      value: &[Limb; N],
      n: u32,
    ) -> Option<[Limb; N]> {
      let negative = is_negative(value);
      if n == 0 || (negative && n % 2 == 0) {
        return None;
      }
      // The root of the magnitude, which also holds that of `MIN`, is at most the magnitude, so its
      // negation is in range.
      let root = $unsigned::nth_root(&unsigned_abs(value), n);
      Some(if negative { wrapping_neg(&root) } else { root })
    }

    /// The logarithm of `value` to the base `base`, rounded down, or `None` if `value` is not
    /// positive or `base` is below 2.
    pub(crate) const fn checked_ilog<const N: usize>(
      value: &[Limb; N],
      base: &[Limb; N],
    ) -> Option<u32> {
      if is_negative(value) || is_negative(base) {
        None
      } else {
        $unsigned::checked_ilog(value, base)
      }
    }

    /// The base-2 logarithm of `value` rounded down, or `None` if `value` is not positive.
    pub(crate) const fn checked_ilog2<const N: usize>(value: &[Limb; N]) -> Option<u32> {
      if is_negative(value) {
        None
      } else {
        $unsigned::checked_ilog2(value)
      }
    }

    /// The base-10 logarithm of `value` rounded down, or `None` if `value` is not positive.
    pub(crate) const fn checked_ilog10<const N: usize>(value: &[Limb; N]) -> Option<u32> {
      if is_negative(value) {
        None
      } else {
        $unsigned::checked_ilog10(value)
      }
    }

    /// The unsigned integer `value` in `M` limbs of two's complement, or `None` if it is not
    /// representable.
    pub(crate) const fn checked_from_unsigned<const N: usize, const M: usize>(
      value: &[Limb; N],
    ) -> Option<[Limb; M]> {
      match $unsigned::checked_resize::<N, M>(value) {
        Some(resized) if !is_negative(&resized) => Some(resized),
        _ => None,
      }
    }

    /// `value` as an unsigned integer of `M` limbs, or `None` if it is negative or does not fit.
    pub(crate) const fn checked_to_unsigned<const N: usize, const M: usize>(
      value: &[Limb; N],
    ) -> Option<[Limb; M]> {
      if is_negative(value) {
        None
      } else {
        $unsigned::checked_resize(value)
      }
    }
  };
}

pub(crate) use limb_layer;

#[cfg(test)]
pub(super) mod tests;
