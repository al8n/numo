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
    const fn shr<const N: usize>(value: &[Limb; N], shift: u32) -> [Limb; N] {
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
      (value[N - 1] as SignedLimb) < 0
    }

    #[inline]
    pub(crate) const fn is_zero<const N: usize>(value: &[Limb; N]) -> bool {
      $unsigned::is_zero(value)
    }

    #[inline]
    pub(crate) const fn is_positive<const N: usize>(value: &[Limb; N]) -> bool {
      !is_negative(value) && !is_zero(value)
    }

    /// Compares `lhs` and `rhs`: the top limbs as `SignedLimb`, then the others as `Limb`, most
    /// significant first.
    pub(crate) const fn cmp<const N: usize>(lhs: &[Limb; N], rhs: &[Limb; N]) -> Ordering {
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
    /// complement integer of `bits` bits in the low bits of `value`, sign-extended to `N` limbs.
    /// Requires `1 <= bits <= B * N`.
    pub(crate) const fn sign_extend<const N: usize>(value: &[Limb; N], bits: u32) -> [Limb; N] {
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
    /// `2^(bits - 1) - 1`. Requires `1 <= bits <= B * N`.
    #[inline]
    pub(crate) const fn fits_bits<const N: usize>(value: &[Limb; N], bits: u32) -> bool {
      // Sign-extending from bit `bits - 1` changes only a value outside the range.
      $unsigned::eq(&sign_extend(value, bits), value)
    }

    /// `2^(bits - 1) - 1`, the largest value of `bits` bits. Requires `1 <= bits <= B * N`.
    #[inline]
    pub(crate) const fn max_of_bits<const N: usize>(bits: u32) -> [Limb; N] {
      $unsigned::max_of_bits(bits - 1)
    }

    /// `-2^(bits - 1)`, the smallest value of `bits` bits. Requires `1 <= bits <= B * N`.
    pub(crate) const fn min_of_bits<const N: usize>(bits: u32) -> [Limb; N] {
      // Every bit from index `bits - 1` up is set, and every bit below it is clear.
      let mut min = max_of_bits::<N>(bits);
      let mut i = 0;
      while i < N {
        min[i] = !min[i];
        i += 1;
      }
      min
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
