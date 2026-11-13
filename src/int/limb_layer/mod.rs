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
    use ::core::{cmp::Ordering, num::ParseIntError};

    use $crate::{
      FromFloatError,
      int::{
        DigitBuf,
        float::{self, FloatFormat, Rounded},
        order, parse_errors, slice,
      },
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

    /// Quotient and remainder of `value / divisor`, a divisor of up to 64 bits, which a limb
    /// narrower than 64 bits does not hold. Requires `divisor != 0`.
    #[inline]
    pub(crate) const fn div_rem_chunk<const N: usize>(
      value: &[Limb; N],
      divisor: u64,
    ) -> ([Limb; N], u64) {
      let mut quotient = [0; N];
      let mut remainder: u64 = 0;
      let mut i = N;
      while i > 0 {
        i -= 1;
        // `remainder < divisor`, so each quotient is below `2^B`, a limb.
        if Limb::BITS >= 64 {
          // A limb of 64 bits or more holds the divisor and the remainder.
          let (limb, rest) = $div_rem_wide(remainder as Limb, value[i], divisor as Limb);
          quotient[i] = limb;
          remainder = rest as u64;
        } else {
          // The shift by the bits of a narrow limb does not overflow; a wider one took the other
          // branch.
          let high = match (remainder as u128).checked_shl(Limb::BITS) {
            Some(high) => high,
            None => 0,
          };
          let wide = high | value[i] as u128;
          quotient[i] = (wide / divisor as u128) as Limb;
          remainder = (wide % divisor as u128) as u64;
        }
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
        matches!(mode, RoundingMode::Up05) && div_rem_limb(quotient, 5).1 == 0,
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

    /// The number of digits of `radix` that a `u64` gathers before they are appended: the largest
    /// `k` with `radix^k <= u64::MAX`, such as 63 for binary, 19 for decimal, 12 for radix 36 and 7
    /// for radix 256. Requires `2 <= radix <= 256`.
    #[inline]
    const fn chunk_digits(radix: u32) -> u32 {
      u64::MAX.ilog(radix as u64)
    }

    /// The value of the digit `byte` in `radix`, an ASCII digit or letter of either case below
    /// `radix`, as `char::to_digit` reads it, or `None` for every other byte. Requires
    /// `2 <= radix <= 36`.
    #[inline]
    const fn digit_value(byte: u8, radix: u32) -> Option<u32> {
      (byte as char).to_digit(radix)
    }

    /// `value * scale + add` modulo `2^(B * N)`, and what carried out of the top limb, which is
    /// below `2^64`.
    const fn mul_add_chunk<const N: usize>(
      value: &[Limb; N],
      scale: u64,
      add: u64,
    ) -> ([Limb; N], u64) {
      // A limb of 128 bits holds the factor and the addend.
      if Limb::BITS > 64 {
        let (product, carry) = mul_add_limb(value, scale as Limb, add as Limb);
        return (product, carry as u64);
      }
      let mut product = [0; N];
      let mut carry = add;
      let mut i = 0;
      while i < N {
        // At most `(2^B - 1) * (2^64 - 1) + 2^64 - 1 < 2^B * 2^64`, so it fits in a `u128`, and
        // the bits above the limb fit in a `u64`.
        let wide = value[i] as u128 * scale as u128 + carry as u128;
        product[i] = wide as Limb;
        // The shift by the 128 bits of the widest limb would overflow; that limb returned above.
        carry = match wide.checked_shr(Limb::BITS) {
          Some(high) => high as u64,
          None => 0,
        };
        i += 1;
      }
      (product, carry)
    }

    /// `value * scale + chunk` if it is at most `2^limit - 1`, or `2^limit` if `power`, and `None`
    /// otherwise. Requires `limit <= B * N`, and `limit < B * N` if `power`.
    pub(crate) const fn append_chunk<const N: usize>(
      value: &[Limb; N],
      scale: u64,
      chunk: u64,
      limit: u32,
      power: bool,
    ) -> Option<[Limb; N]> {
      match mul_add_chunk(value, scale, chunk) {
        (next, 0) if fits_bits(&next, limit) || (power && eq(&next, &pow2(limit))) => Some(next),
        _ => None,
      }
    }

    /// Parses `digits`, the digits of `radix` without a sign, as a magnitude of at most
    /// `2^limit - 1`, or of at most `2^limit` if `power`, with the errors of the native
    /// `from_str_radix` from the left: a byte that is not a digit of `radix` is `InvalidDigit`,
    /// unless the digits before it already exceed the limit, which is `PosOverflow`, or
    /// `NegOverflow` if `negative`. No digits are zero. Requires `2 <= radix <= 36` and
    /// `limit <= B * N`, and `limit < B * N` if `power`.
    ///
    /// The digits are gathered in a `u64` and appended `chunk_digits(radix)` at a time, each chunk
    /// with one pass over the limbs. The value only grows with each digit, so the limit checked
    /// after each chunk and before a byte that is not a digit gives the error that a check after
    /// each digit gives.
    pub(crate) const fn parse_magnitude<const N: usize>(
      digits: &[u8],
      radix: u32,
      limit: u32,
      power: bool,
      negative: bool,
    ) -> Result<[Limb; N], ParseIntError> {
      let per_chunk = chunk_digits(radix);
      let mut value = [0; N];
      // The digits since the last append, `radix` to their number, and their number: the first
      // two stay at most `radix^per_chunk <= u64::MAX`.
      let (mut chunk, mut scale, mut count) = (0, 1, 0);
      let mut i = 0;
      while i < digits.len() {
        let digit = match digit_value(digits[i], radix) {
          Some(digit) => digit,
          // The digits before the byte decide whether an overflow came first.
          None => {
            return match append_chunk(&value, scale, chunk, limit, power) {
              Some(_) => Err(parse_errors::INVALID_DIGIT),
              None => Err(parse_errors::overflow(negative)),
            };
          }
        };
        chunk = chunk * radix as u64 + digit as u64;
        scale *= radix as u64;
        count += 1;
        if count == per_chunk {
          value = match append_chunk(&value, scale, chunk, limit, power) {
            Some(value) => value,
            None => return Err(parse_errors::overflow(negative)),
          };
          (chunk, scale, count) = (0, 1, 0);
        }
        i += 1;
      }
      match append_chunk(&value, scale, chunk, limit, power) {
        Some(value) => Ok(value),
        None => Err(parse_errors::overflow(negative)),
      }
    }

    /// Parses `src`, an optional `+` and the digits of `radix`, as a value of `bits` bits, with the
    /// errors of the native `from_str_radix`: `Empty` for no bytes, `InvalidDigit` for a lone sign,
    /// and those of [`parse_magnitude`] for the digits. A `-` is a byte that is not a digit, as for
    /// the native unsigned integers. Requires `2 <= radix <= 36` and `bits <= B * N`.
    pub(crate) const fn from_str_radix_bits<const N: usize>(
      src: &[u8],
      radix: u32,
      bits: u32,
    ) -> Result<[Limb; N], ParseIntError> {
      match src {
        [] => Err(parse_errors::EMPTY),
        [b'+' | b'-'] => Err(parse_errors::INVALID_DIGIT),
        [b'+', digits @ ..] => parse_magnitude(digits, radix, bits, false, false),
        digits => parse_magnitude(digits, radix, bits, false, false),
      }
    }

    /// The integer whose digits in `radix`, a byte each, are `digits`, most significant first if
    /// `big_endian` and least significant first otherwise, as a value of `bits` bits, or `None` if
    /// a digit is not below `radix` or the value is `2^bits` or more. No digits are zero. Requires
    /// `2 <= radix <= 256` and `bits <= B * N`.
    ///
    /// The digits are read from the most significant down, whatever the order of the slice, and
    /// appended `chunk_digits(radix)` at a time, as [`parse_magnitude`] appends them.
    pub(crate) const fn checked_read_digits<const N: usize>(
      digits: &[u8],
      big_endian: bool,
      radix: u32,
      bits: u32,
    ) -> Option<[Limb; N]> {
      let per_chunk = chunk_digits(radix);
      let mut value = [0; N];
      // The digits since the last append, `radix` to their number, and their number: the first
      // two stay at most `radix^per_chunk <= u64::MAX`.
      let (mut chunk, mut scale, mut count) = (0, 1, 0);
      let mut i = 0;
      while i < digits.len() {
        let digit = if big_endian {
          digits[i]
        } else {
          digits[digits.len() - 1 - i]
        };
        if digit as u32 >= radix {
          return None;
        }
        chunk = chunk * radix as u64 + digit as u64;
        scale *= radix as u64;
        count += 1;
        if count == per_chunk {
          value = match append_chunk(&value, scale, chunk, bits, false) {
            Some(value) => value,
            None => return None,
          };
          (chunk, scale, count) = (0, 1, 0);
        }
        i += 1;
      }
      append_chunk(&value, scale, chunk, bits, false)
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

    /// `lhs + rhs + carry` modulo `2^bits`, and whether the sum is `2^bits` or more. Requires
    /// `bits <= B * N` and both values below `2^bits`.
    pub(crate) const fn carrying_add_bits<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
      carry: bool,
      bits: u32,
    ) -> ([Limb; N], bool) {
      let (sum, overflow) = overflowing_add_bits(lhs, rhs, bits);
      if carry {
        // Adding one subtracts `2^bits - 1`, which is `-1` modulo `2^bits`, and overflows exactly
        // when the sum is `2^bits - 1`, which is when the subtraction does not borrow. At zero
        // bits, which hold no one, a carry always overflows.
        let (sum, borrow) = overflowing_sub_bits(&sum, &max_of_bits(bits), bits);
        (sum, overflow || !borrow)
      } else {
        (sum, overflow)
      }
    }

    /// `lhs - rhs - borrow` modulo `2^bits`, and whether the difference is negative. Requires
    /// `bits <= B * N` and both values below `2^bits`.
    pub(crate) const fn borrowing_sub_bits<const N: usize>(
      lhs: &[Limb; N],
      rhs: &[Limb; N],
      borrow: bool,
      bits: u32,
    ) -> ([Limb; N], bool) {
      let (difference, overflow) = overflowing_sub_bits(lhs, rhs, bits);
      if borrow {
        // Subtracting one adds `2^bits - 1`, which is `-1` modulo `2^bits`, and overflows exactly
        // when the difference is zero, which is when the addition does not carry. At zero bits,
        // which hold no one, a borrow always overflows.
        let (difference, carry) = overflowing_add_bits(&difference, &max_of_bits(bits), bits);
        (difference, overflow || !carry)
      } else {
        (difference, overflow)
      }
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

    /// `lhs | rhs`.
    pub(crate) const fn or<const N: usize>(lhs: &[Limb; N], rhs: &[Limb; N]) -> [Limb; N] {
      let mut result = [0; N];
      let mut i = 0;
      while i < N {
        result[i] = lhs[i] | rhs[i];
        i += 1;
      }
      result
    }

    /// `!value` modulo `2^bits`: every bit below index `bits` flipped. Requires `bits <= B * N` and
    /// `value < 2^bits`.
    #[inline]
    pub(crate) const fn not_bits<const N: usize>(value: &[Limb; N], bits: u32) -> [Limb; N] {
      xor(value, &max_of_bits(bits))
    }

    /// `value << shift` modulo `2^(B * N)`, zero for `shift >= B * N`.
    pub(crate) const fn shift_left<const N: usize>(value: &[Limb; N], shift: u32) -> [Limb; N] {
      let whole = shift / Limb::BITS;
      // The offset is compared before it is narrowed to `usize`, which can have fewer bits than
      // `u32`.
      if whole as u64 >= N as u64 {
        return [0; N];
      }
      let limbs = whole as usize;
      let bits = shift % Limb::BITS;
      let mut shifted = [0; N];
      let mut i = N;
      while i > limbs {
        i -= 1;
        let source = i - limbs;
        shifted[i] = value[source] << bits;
        // The top `bits` bits of the limb below: a shift by zero bits takes none, and the shift by
        // `B - bits` would overflow there.
        if bits > 0 && source > 0 {
          shifted[i] |= value[source - 1] >> (Limb::BITS - bits);
        }
      }
      shifted
    }

    /// `value >> shift`, zero for `shift >= B * N`.
    pub(crate) const fn shift_right<const N: usize>(value: &[Limb; N], shift: u32) -> [Limb; N] {
      let whole = shift / Limb::BITS;
      // The offset is compared before it is narrowed to `usize`, which can have fewer bits than
      // `u32`.
      if whole as u64 >= N as u64 {
        return [0; N];
      }
      let limbs = whole as usize;
      let bits = shift % Limb::BITS;
      let mut shifted = [0; N];
      let mut i = 0;
      while i + limbs < N {
        let source = i + limbs;
        shifted[i] = value[source] >> bits;
        // The low bits of the limb above, as in `shift_left`.
        if bits > 0 && source + 1 < N {
          shifted[i] |= value[source + 1] << (Limb::BITS - bits);
        }
        i += 1;
      }
      shifted
    }

    /// The number of ones in `value`. Requires it to be below `2^32`, as it is in every value of a
    /// width.
    pub(crate) const fn count_ones<const N: usize>(value: &[Limb; N]) -> u32 {
      let mut count = 0;
      let mut i = 0;
      while i < N {
        count += value[i].count_ones();
        i += 1;
      }
      count
    }

    /// The number of ones in `value`, a value of `bits` bits, which has none above them.
    #[inline]
    pub(crate) const fn count_ones_bits<const N: usize>(value: &[Limb; N], _bits: u32) -> u32 {
      count_ones(value)
    }

    /// The index of the lowest one of `value`, or `None` for zero. Requires that index to be below
    /// `2^32`, as it is in every value of a width.
    pub(crate) const fn lowest_one<const N: usize>(value: &[Limb; N]) -> Option<u32> {
      let mut i = 0;
      while i < N {
        if value[i] != 0 {
          return Some(i as u32 * Limb::BITS + value[i].trailing_zeros());
        }
        i += 1;
      }
      None
    }

    /// The index of the highest one of `value`, or `None` for zero: a value of `bits` bits has none
    /// above them, so the width does not change it.
    #[inline]
    pub(crate) const fn highest_one_bits<const N: usize>(
      value: &[Limb; N],
      _bits: u32,
    ) -> Option<u32> {
      checked_ilog2(value)
    }

    /// The number of zeros of `value` from bit `bits - 1` down to its highest one, or `bits` for
    /// zero. Requires `bits <= B * N` and `value < 2^bits`.
    pub(crate) const fn leading_zeros_bits<const N: usize>(value: &[Limb; N], bits: u32) -> u32 {
      match checked_ilog2(value) {
        Some(log) => bits - 1 - log,
        None => bits,
      }
    }

    /// The number of zeros of `value` below its lowest one, or `bits` for zero. Requires
    /// `value < 2^bits`.
    pub(crate) const fn trailing_zeros_bits<const N: usize>(value: &[Limb; N], bits: u32) -> u32 {
      match lowest_one(value) {
        Some(index) => index,
        None => bits,
      }
    }

    /// The number of ones of `value` from bit `bits - 1` down, the leading zeros of its complement
    /// in `bits` bits. Requires `bits <= B * N` and `value < 2^bits`.
    #[inline]
    pub(crate) const fn leading_ones_bits<const N: usize>(value: &[Limb; N], bits: u32) -> u32 {
      leading_zeros_bits(&not_bits(value, bits), bits)
    }

    /// The number of ones of `value` from bit 0 up, the trailing zeros of its complement in `bits`
    /// bits. Requires `bits <= B * N` and `value < 2^bits`.
    #[inline]
    pub(crate) const fn trailing_ones_bits<const N: usize>(value: &[Limb; N], bits: u32) -> u32 {
      trailing_zeros_bits(&not_bits(value, bits), bits)
    }

    /// `value` with only its highest one, or zero for zero.
    pub(crate) const fn isolate_highest_one_bits<const N: usize>(
      value: &[Limb; N],
      _bits: u32,
    ) -> [Limb; N] {
      match checked_ilog2(value) {
        Some(log) => pow2(log),
        None => [0; N],
      }
    }

    /// `value` with only its lowest one, or zero for zero.
    pub(crate) const fn isolate_lowest_one_bits<const N: usize>(
      value: &[Limb; N],
      _bits: u32,
    ) -> [Limb; N] {
      match lowest_one(value) {
        Some(index) => pow2(index),
        None => [0; N],
      }
    }

    /// The bits of `value` below index `bits` in reverse order: bit `i` moves to bit
    /// `bits - 1 - i`. Requires `bits <= B * N` and `value < 2^bits`.
    pub(crate) const fn reverse_bits_bits<const N: usize>(
      value: &[Limb; N],
      bits: u32,
    ) -> [Limb; N] {
      // The limbs that hold the width, reversed limb by limb and bit by bit, hold the pattern
      // reversed above `gap` zero bits, those that the top limb holds above the width.
      let limbs = bits.div_ceil(Limb::BITS) as usize;
      let mut reversed = [0; N];
      let mut i = 0;
      while i < limbs {
        reversed[i] = value[limbs - 1 - i].reverse_bits();
        i += 1;
      }
      let gap = (Limb::BITS - bits % Limb::BITS) % Limb::BITS;
      shr(&reversed, gap)
    }

    /// `value` rotated left by `n` places within `bits` bits: the bits shifted out past bit
    /// `bits - 1` come back at bit 0. Requires `bits <= B * N` and `value < 2^bits`.
    pub(crate) const fn rotate_left_bits<const N: usize>(
      value: &[Limb; N],
      n: u32,
      bits: u32,
    ) -> [Limb; N] {
      // Zero bits hold only zero, which no rotation changes, and take no remainder by zero.
      if bits == 0 {
        return *value;
      }
      let amount = n % bits;
      if amount == 0 {
        return *value;
      }
      let rotated = or(&shift_left(value, amount), &shift_right(value, bits - amount));
      zero_extend(&rotated, bits)
    }

    /// `value` rotated right by `n` places within `bits` bits, the left rotation by the rest of the
    /// width. Requires `bits <= B * N` and `value < 2^bits`.
    pub(crate) const fn rotate_right_bits<const N: usize>(
      value: &[Limb; N],
      n: u32,
      bits: u32,
    ) -> [Limb; N] {
      if bits == 0 {
        return *value;
      }
      match n % bits {
        0 => *value,
        amount => rotate_left_bits(value, bits - amount, bits),
      }
    }

    /// `value << (rhs % bits)` modulo `2^bits`, and whether `rhs >= bits`, as the native
    /// `overflowing_shl` gives them for its width; zero bits overflow on every amount. Requires
    /// `bits <= B * N` and `value < 2^bits`.
    pub(crate) const fn overflowing_shl_bits<const N: usize>(
      value: &[Limb; N],
      rhs: u32,
      bits: u32,
    ) -> ([Limb; N], bool) {
      // Zero bits hold only zero, and take no remainder by zero.
      if bits == 0 {
        return ([0; N], true);
      }
      let amount = if rhs < bits { rhs } else { rhs % bits };
      (zero_extend(&shift_left(value, amount), bits), rhs >= bits)
    }

    /// `value >> (rhs % bits)`, and whether `rhs >= bits`, as [`overflowing_shl_bits`] gives them.
    /// Requires `bits <= B * N` and `value < 2^bits`.
    pub(crate) const fn overflowing_shr_bits<const N: usize>(
      value: &[Limb; N],
      rhs: u32,
      bits: u32,
    ) -> ([Limb; N], bool) {
      if bits == 0 {
        return ([0; N], true);
      }
      let amount = if rhs < bits { rhs } else { rhs % bits };
      (shift_right(value, amount), rhs >= bits)
    }

    /// `value << rhs` modulo `2^bits`, which is zero for `rhs >= bits`. Requires `bits <= B * N`.
    #[inline]
    pub(crate) const fn unbounded_shl_bits<const N: usize>(
      value: &[Limb; N],
      rhs: u32,
      bits: u32,
    ) -> [Limb; N] {
      zero_extend(&shift_left(value, rhs), bits)
    }

    /// `value >> rhs`, which is zero for `rhs >= bits`: a value of `bits` bits has no one above
    /// them, so the width does not change it.
    #[inline]
    pub(crate) const fn unbounded_shr_bits<const N: usize>(
      value: &[Limb; N],
      rhs: u32,
      _bits: u32,
    ) -> [Limb; N] {
      shift_right(value, rhs)
    }

    /// `value << rhs` if `rhs < bits` and the shift moves no one out of `bits` bits, so that
    /// `>> rhs` gives `value` back, or `None` otherwise. Requires `bits <= B * N` and
    /// `value < 2^bits`.
    pub(crate) const fn shl_exact_bits<const N: usize>(
      value: &[Limb; N],
      rhs: u32,
      bits: u32,
    ) -> Option<[Limb; N]> {
      if rhs < bits && rhs <= leading_zeros_bits(value, bits) {
        Some(shift_left(value, rhs))
      } else {
        None
      }
    }

    /// `value >> rhs` if `rhs < bits` and the shift moves no one out, so that `<< rhs` gives
    /// `value` back, or `None` otherwise. Requires `value < 2^bits`.
    pub(crate) const fn shr_exact_bits<const N: usize>(
      value: &[Limb; N],
      rhs: u32,
      bits: u32,
    ) -> Option<[Limb; N]> {
      if rhs < bits && rhs <= trailing_zeros_bits(value, bits) {
        Some(shift_right(value, rhs))
      } else {
        None
      }
    }

    /// `value`, a pattern of `bits` bits, read in two's complement: every bit at index `bits` or
    /// above set to bit `bits - 1`, or zero for zero bits. Requires `bits <= B * N` and
    /// `value < 2^bits`.
    pub(crate) const fn cast_signed_bits<const N: usize>(
      value: &[Limb; N],
      bits: u32,
    ) -> [Limb; N] {
      if bits == 0 {
        return [0; N];
      }
      let top = bits - 1;
      if (value[(top / Limb::BITS) as usize] >> (top % Limb::BITS)) & 1 == 1 {
        // The bits from index `bits` up, every bit that `max_of_bits` leaves clear.
        or(value, &xor(&max_of_bits(bits), &[Limb::MAX; N]))
      } else {
        *value
      }
    }

    /// `value` as the two's complement integer of `bits` bits, or the largest one,
    /// `2^(bits - 1) - 1`, if `value` is above it, which is when bit `bits - 1` is set; zero bits
    /// hold only zero. Requires `bits <= B * N` and `value < 2^bits`.
    pub(crate) const fn saturating_cast_signed_bits<const N: usize>(
      value: &[Limb; N],
      bits: u32,
    ) -> [Limb; N] {
      if bit(value, bits.wrapping_sub(1), bits) {
        // The largest value in two's complement has every bit below the sign bit: the complement
        // of the sign bit alone, which is the highest one of `value`.
        cast_signed_bits(&not_bits(&isolate_highest_one_bits(value, bits), bits), bits)
      } else {
        cast_signed_bits(value, bits)
      }
    }

    /// The bit at `index` of `value`, or `false` for `index >= bits`. Requires `bits <= B * N`.
    pub(crate) const fn bit<const N: usize>(value: &[Limb; N], index: u32, bits: u32) -> bool {
      index < bits && (value[(index / Limb::BITS) as usize] >> (index % Limb::BITS)) & 1 == 1
    }

    /// `value` with the bit at `index` set if `set`, and cleared otherwise, or `None` for
    /// `index >= bits`. Requires `bits <= B * N`.
    pub(crate) const fn with_bit<const N: usize>(
      value: &[Limb; N],
      index: u32,
      set: bool,
      bits: u32,
    ) -> Option<[Limb; N]> {
      if index >= bits {
        return None;
      }
      let mut changed = *value;
      let limb = (index / Limb::BITS) as usize;
      let mask: Limb = 1 << (index % Limb::BITS);
      if set {
        changed[limb] |= mask;
      } else {
        changed[limb] &= !mask;
      }
      Some(changed)
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

    /// The number of bytes of a limb.
    const LIMB_BYTES: usize = (Limb::BITS / 8) as usize;

    /// The byte of `value` at `index`: its bits from `8 * index` to `8 * index + 7`. Requires
    /// `index < B * N / 8`.
    #[inline]
    pub(crate) const fn byte_at<const N: usize>(value: &[Limb; N], index: usize) -> u8 {
      // The offset of the byte in bits, which a `u64` holds for every index of the limbs.
      let bit = 8 * index as u64;
      (value[(bit / Limb::BITS as u64) as usize] >> (bit % Limb::BITS as u64) as u32) as u8
    }

    /// The 64-bit word at `index` of the integer in `value`, its bits from `64 * index` to
    /// `64 * index + 63`, extended beyond the limbs by copies of `fill`: zero for an unsigned
    /// integer, and the bits of its sign for one in two's complement. A word joins `64 / B` limbs,
    /// the least significant first, or is the low or the high half of a limb of 128 bits.
    pub(crate) const fn extended_word<const N: usize>(
      value: &[Limb; N],
      index: usize,
      fill: Limb,
    ) -> u64 {
      if Limb::BITS == 128 {
        let limb = if index / 2 < N { value[index / 2] } else { fill };
        return (limb as u128 >> (64 * (index % 2))) as u64;
      }
      let per_word = (u64::BITS / Limb::BITS) as usize;
      // The index of the first limb of the word, which saturates far beyond the limbs, where every
      // limb is the fill.
      let first = index.saturating_mul(per_word);
      let mut word = 0;
      let mut k = 0;
      while k < per_word {
        let i = first.saturating_add(k);
        let limb = if i < N { value[i] } else { fill };
        word |= (limb as u64) << (Limb::BITS * k as u32);
        k += 1;
      }
      word
    }

    /// The 64-bit word at `index` of `value`, as [`extended_word`] gives it: zero beyond the limbs.
    #[inline]
    pub(crate) const fn word<const N: usize>(value: &[Limb; N], index: usize) -> u64 {
      extended_word(value, index, 0)
    }

    /// The low 128 bits of `value`, extended beyond its limbs by copies of `fill`, and whether
    /// every limb above them is `fill`, so that those bits, extended by `fill`, hold the value.
    pub(crate) const fn low_u128<const N: usize>(value: &[Limb; N], fill: Limb) -> (u128, bool) {
      let high = extended_word(value, 1, fill) as u128;
      let low = high << 64 | extended_word(value, 0, fill) as u128;
      // 128 is a multiple of the bits of every limb type, so a limb is below the low 128 bits or
      // above them.
      let mut i = (128 / Limb::BITS) as usize;
      while i < N {
        if value[i] != fill {
          return (low, false);
        }
        i += 1;
      }
      (low, true)
    }

    /// Compares `value` with `rhs`, as integers: a value with a bit set from index 128 up is the
    /// greater, and another compares its low 128 bits.
    pub(crate) const fn compare_u128<const N: usize>(value: &[Limb; N], rhs: u128) -> Ordering {
      match low_u128(value, 0) {
        (low, true) => order::cmp_u128(low, rhs),
        (_, false) => Ordering::Greater,
      }
    }

    /// Compares `value` with `rhs`, as integers: `value` is above every negative `rhs`.
    #[inline]
    pub(crate) const fn compare_i128<const N: usize>(value: &[Limb; N], rhs: i128) -> Ordering {
      if rhs < 0 {
        Ordering::Greater
      } else {
        compare_u128(value, rhs as u128)
      }
    }

    /// `value` in limbs. Requires `value < 2^(B * N)`.
    pub(crate) const fn from_u64<const N: usize>(value: u64) -> [Limb; N] {
      let mut limbs = [0; N];
      let mut rest = value;
      let mut i = 0;
      while rest != 0 {
        limbs[i] = rest as Limb;
        // A limb of 64 bits or more takes all of `value`, which a shift by the width of the limb
        // would overflow.
        rest = match rest.checked_shr(Limb::BITS) {
          Some(rest) => rest,
          None => 0,
        };
        i += 1;
      }
      limbs
    }

    /// The bits of the float of `format` nearest to `magnitude`, negated if `negative`, ties to
    /// even, or of the infinity of that sign if the magnitude rounds to `2^(2^(e - 1))` or more,
    /// for `e` bits of exponent. A magnitude of at most `p` bits, the precision of the format,
    /// converts exactly. A longer one keeps its top `p` bits, which round up when the bit below
    /// them, the guard bit, is set and either a lower bit is set or the top bits are odd; a carry
    /// out of them makes one bit more. Zero is `+0.0`, whatever `negative`.
    pub(crate) const fn magnitude_float_bits<const N: usize>(
      magnitude: &[Limb; N],
      negative: bool,
      format: FloatFormat,
    ) -> u64 {
      // The index of the highest one of the magnitude, the exponent of the float.
      let mut log = match checked_ilog2(magnitude) {
        Some(log) => log,
        None => return 0,
      };
      let precision = format.precision();
      // The significand, with its top bit at `precision - 1`, the implicit bit of the format.
      let significand = if log < precision {
        // At most `precision` bits, which the low word holds.
        word(magnitude, 0) << (precision - 1 - log)
      } else {
        let dropped = log + 1 - precision;
        let top = word(&shift_right(magnitude, dropped), 0);
        let guard = dropped - 1;
        let guard_set = (word(magnitude, (guard / 64) as usize) >> (guard % 64)) & 1 == 1;
        let sticky = match lowest_one(magnitude) {
          Some(lowest) => lowest < guard,
          None => false,
        };
        if !(guard_set && (sticky || top & 1 == 1)) {
          top
        } else if top + 1 == 1 << precision {
          log += 1;
          1 << (precision - 1)
        } else {
          top + 1
        }
      };
      let exponent_bits = format.exponent_bits();
      let fraction_bits = precision - 1;
      let sign = (negative as u64) << (fraction_bits + exponent_bits);
      // The biased exponent of every bit set marks the infinities.
      let infinite = (1 << exponent_bits) - 1;
      let biased = log as u64 + (1 << (exponent_bits - 1)) - 1;
      if biased >= infinite {
        sign | infinite << fraction_bits
      } else {
        sign | biased << fraction_bits | (significand & ((1 << fraction_bits) - 1))
      }
    }

    /// The bits of the float of `format` nearest to `value`, ties to even, or of the infinity
    /// above its range, as [`magnitude_float_bits`] gives them for a value that is not negative.
    #[inline]
    pub(crate) const fn float_bits<const N: usize>(value: &[Limb; N], format: FloatFormat) -> u64 {
      magnitude_float_bits(value, false, format)
    }

    /// `rounded`, a float rounded to an integer, as a value of `bits` bits, or the error of a value
    /// outside their range: `NegOverflow` for a negative value but zero, and `PosOverflow` for a
    /// magnitude of more than `bits` bits. Requires `bits <= B * N`.
    pub(crate) const fn place_magnitude<const N: usize>(
      rounded: Rounded,
      bits: u32,
    ) -> Result<[Limb; N], FromFloatError> {
      match float::check_range(rounded, bits, false) {
        // The magnitude is below `2^bits`, so its significand is below `2^(B * N)`.
        Ok(()) => Ok(shift_left(&from_u64(rounded.significand), rounded.shift)),
        Err(error) => Err(error),
      }
    }

    /// Writes the low `out.len()` bytes of `value` to `out`, least significant first, or most
    /// significant first if `big_endian`. The upper layer writes with it too: the limbs hold the
    /// bits of a value of either sign, and for one in two's complement the copies of its sign above
    /// them. Requires `out.len() <= B * N / 8`.
    pub(crate) const fn write_bytes<const N: usize>(
      value: &[Limb; N],
      out: &mut [u8],
      big_endian: bool,
    ) {
      let len = out.len();
      let mut k = 0;
      while k < len {
        out[slice::index(len, k, big_endian)] = byte_at(value, k);
        k += 1;
      }
    }

    /// The low `B * N` bits of the integer in `bytes`, least significant byte first, or most
    /// significant first if `big_endian`, extended beyond its bytes by copies of `fill`: zero for
    /// an unsigned integer, and the bits of its sign for one in two's complement.
    pub(crate) const fn read_bytes<const N: usize>(
      bytes: &[u8],
      big_endian: bool,
      fill: u8,
    ) -> [Limb; N] {
      let mut value = [0; N];
      let mut i = 0;
      while i < N {
        let mut j = 0;
        while j < LIMB_BYTES {
          let byte = slice::byte(bytes, LIMB_BYTES * i + j, big_endian, fill) as Limb;
          value[i] |= byte << (8 * j) as u32;
          j += 1;
        }
        i += 1;
      }
      value
    }

    /// The integer in `bytes`, least significant byte first, or most significant first if
    /// `big_endian`, unsigned, as a value of `bits` bits, or `None` if it is `2^bits` or more,
    /// whatever the number of bytes. No bytes are zero. Requires `bits <= B * N`.
    pub(crate) const fn checked_read_bytes<const N: usize>(
      bytes: &[u8],
      big_endian: bool,
      bits: u32,
    ) -> Option<[Limb; N]> {
      if slice::fits_bits(bytes, big_endian, bits, false) {
        Some(read_bytes(bytes, big_endian, 0))
      } else {
        None
      }
    }

    /// The integer in `bytes`, read as [`checked_read_bytes`] reads it, modulo `2^bits`. Requires
    /// `bits <= B * N`.
    #[inline]
    pub(crate) const fn wrapping_read_bytes<const N: usize>(
      bytes: &[u8],
      big_endian: bool,
      bits: u32,
    ) -> [Limb; N] {
      zero_extend(&read_bytes(bytes, big_endian, 0), bits)
    }

    /// The integer in `bytes`, read as [`checked_read_bytes`] reads it, or `2^bits - 1`, the
    /// largest value of `bits` bits, if it is above that. Requires `bits <= B * N`.
    #[inline]
    pub(crate) const fn saturating_read_bytes<const N: usize>(
      bytes: &[u8],
      big_endian: bool,
      bits: u32,
    ) -> [Limb; N] {
      match checked_read_bytes(bytes, big_endian, bits) {
        Some(value) => value,
        None => max_of_bits(bits),
      }
    }

    /// The low `bits / 8` bytes of `value` in reverse order: the limbs that hold the width,
    /// reversed limb by limb and byte by byte, hold them above the bytes of the top limb that are
    /// above the width, which a shift removes. Requires `bits` to be a multiple of 8,
    /// `bits <= B * N` and `value < 2^bits`.
    pub(crate) const fn swap_bytes_bits<const N: usize>(value: &[Limb; N], bits: u32) -> [Limb; N] {
      let limbs = bits.div_ceil(Limb::BITS) as usize;
      let mut swapped = [0; N];
      let mut i = 0;
      while i < limbs {
        swapped[i] = value[limbs - 1 - i].swap_bytes();
        i += 1;
      }
      let gap = (Limb::BITS - bits % Limb::BITS) % Limb::BITS;
      shr(&swapped, gap)
    }

    /// The limb that extends the integer in `limbs`, least significant limb first, read in two's
    /// complement if `signed`: `Limb::MAX`, the copies of its sign, if the top bit of its last limb
    /// is set, and zero otherwise, as for an unsigned integer and for no limbs.
    #[inline]
    pub(crate) const fn limbs_fill(limbs: &[Limb], signed: bool) -> Limb {
      match limbs.last() {
        Some(&top) if signed && top >> (Limb::BITS - 1) != 0 => Limb::MAX,
        _ => 0,
      }
    }

    /// Returns `true` if the integer in `limbs`, least significant limb first, read in two's
    /// complement if `signed` and unsigned otherwise, is within the range of `bits` bits: every bit
    /// from index `bits` up is zero for an unsigned integer, and every bit from index `bits - 1` up
    /// copies the sign for a signed one, whose zero bits hold only zero, as for an unsigned one.
    pub(crate) const fn limbs_fit_bits(limbs: &[Limb], bits: u32, signed: bool) -> bool {
      // The lowest bit that must equal the bits of `fill`, and `fill`.
      let (low, fill) = if signed && bits > 0 {
        (bits - 1, limbs_fill(limbs, true))
      } else {
        (bits, 0)
      };
      // The index of the limb that holds bit `low`, compared before it is narrowed to `usize`,
      // which can have fewer bits than `u32`, and the bits below `low` in that limb.
      let (first, below) = ((low / Limb::BITS) as u64, low % Limb::BITS);
      let len = limbs.len();
      let mut k = if first < len as u64 {
        first as usize
      } else {
        len
      };
      while k < len {
        let differ = limbs[k] ^ fill;
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

    /// The low `B * N` bits of the integer in `limbs`, least significant limb first, extended
    /// beyond its limbs by copies of `fill`: zero for an unsigned integer, and the bits of its sign
    /// for one in two's complement.
    pub(crate) const fn read_limbs<const N: usize>(limbs: &[Limb], fill: Limb) -> [Limb; N] {
      let mut value = [fill; N];
      let mut i = 0;
      while i < N && i < limbs.len() {
        value[i] = limbs[i];
        i += 1;
      }
      value
    }

    /// The integer in `limbs`, least significant limb first, unsigned, as a value of `bits` bits,
    /// or `None` if it is `2^bits` or more, whatever the number of limbs. No limbs are zero.
    /// Requires `bits <= B * N`.
    pub(crate) const fn checked_read_limbs<const N: usize>(
      limbs: &[Limb],
      bits: u32,
    ) -> Option<[Limb; N]> {
      if limbs_fit_bits(limbs, bits, false) {
        Some(read_limbs(limbs, 0))
      } else {
        None
      }
    }

    /// The integer in `limbs`, read as [`checked_read_limbs`] reads it, modulo `2^bits`. Requires
    /// `bits <= B * N`.
    #[inline]
    pub(crate) const fn wrapping_read_limbs<const N: usize>(
      limbs: &[Limb],
      bits: u32,
    ) -> [Limb; N] {
      zero_extend(&read_limbs(limbs, 0), bits)
    }

    /// The integer in `limbs`, read as [`checked_read_limbs`] reads it, or `2^bits - 1`, the
    /// largest value of `bits` bits, if it is above that. Requires `bits <= B * N`.
    #[inline]
    pub(crate) const fn saturating_read_limbs<const N: usize>(
      limbs: &[Limb],
      bits: u32,
    ) -> [Limb; N] {
      match checked_read_limbs(limbs, bits) {
        Some(value) => value,
        None => max_of_bits(bits),
      }
    }
  };
  (signed limb: $limb:ident, signed: $signed:ident, unsigned: $unsigned:ident) => {
    use ::core::{cmp::Ordering, num::ParseIntError};

    use $crate::int::unsigned::$unsigned;
    use $crate::{
      FromFloatError,
      int::{
        DigitBuf,
        float::{self, FloatFormat, Rounded},
        order, parse_errors, slice,
      },
      rounding::RoundingMode,
    };

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

    /// Parses `src`, an optional `+` or `-` and the digits of `radix`, as a value of `bits` bits in
    /// two's complement, with the errors of the native `from_str_radix`: `Empty` for no bytes,
    /// `InvalidDigit` for a lone sign, and those of `parse_magnitude` for the magnitude, which is
    /// at most `2^(bits - 1) - 1`, or `2^(bits - 1)` after `-`, and zero at zero bits. Requires
    /// `2 <= radix <= 36` and `bits <= B * N`.
    pub(crate) const fn from_str_radix_bits<const N: usize>(
      src: &[u8],
      radix: u32,
      bits: u32,
    ) -> Result<[Limb; N], ParseIntError> {
      let (negative, digits) = match src {
        [] => return Err(parse_errors::EMPTY),
        [b'+' | b'-'] => return Err(parse_errors::INVALID_DIGIT),
        [b'+', digits @ ..] => (false, digits),
        [b'-', digits @ ..] => (true, digits),
        digits => (false, digits),
      };
      // Zero bits hold only zero, the one magnitude below `2^0`.
      let (limit, power) = if bits == 0 {
        (0, false)
      } else {
        (bits - 1, negative)
      };
      match $unsigned::parse_magnitude(digits, radix, limit, power, negative) {
        // A magnitude of at most `2^(bits - 1)` negates in `N` limbs to a value whose bits above
        // the width copy its sign.
        Ok(magnitude) if negative => Ok(wrapping_neg(&magnitude)),
        result => result,
      }
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

    /// `lhs & rhs`, limb by limb, as the lower layer computes it: the copies of the signs combine
    /// as the signs do.
    #[inline]
    pub(crate) const fn and<const N: usize>(lhs: &[Limb; N], rhs: &[Limb; N]) -> [Limb; N] {
      $unsigned::and(lhs, rhs)
    }

    /// `lhs | rhs`, as [`and`] computes it.
    #[inline]
    pub(crate) const fn or<const N: usize>(lhs: &[Limb; N], rhs: &[Limb; N]) -> [Limb; N] {
      $unsigned::or(lhs, rhs)
    }

    /// `lhs ^ rhs`, as [`and`] computes it.
    #[inline]
    pub(crate) const fn xor<const N: usize>(lhs: &[Limb; N], rhs: &[Limb; N]) -> [Limb; N] {
      $unsigned::xor(lhs, rhs)
    }

    /// `!value`: every bit flipped, which keeps the copies of the sign, or zero for zero bits.
    /// Requires `bits <= B * N` and `value` within the range of `bits` bits.
    pub(crate) const fn not_bits<const N: usize>(value: &[Limb; N], bits: u32) -> [Limb; N] {
      // Zero bits hold only zero, whose flipped limbs would read as -1.
      if bits == 0 {
        return [0; N];
      }
      $unsigned::xor(value, &[Limb::MAX; N])
    }

    /// `value >> shift`, an arithmetic shift: the bits shifted in copy the sign, and a shift of
    /// `B * N` bits or more leaves the sign alone, 0 or -1.
    pub(crate) const fn sar<const N: usize>(value: &[Limb; N], shift: u32) -> [Limb; N] {
      let fill = if is_negative(value) { Limb::MAX } else { 0 };
      let whole = shift / Limb::BITS;
      // The offset is compared before it is narrowed to `usize`, which can have fewer bits than
      // `u32`.
      if whole as u64 >= N as u64 {
        return [fill; N];
      }
      let limbs = whole as usize;
      let bits = shift % Limb::BITS;
      let mut shifted = [fill; N];
      let mut i = 0;
      while i + limbs < N {
        let source = i + limbs;
        // The limb above the top one reads as a copy of the sign.
        let next = if source + 1 < N { value[source + 1] } else { fill };
        // The shift by `B - bits` would overflow at zero bits, which take nothing from `next`.
        shifted[i] = if bits == 0 {
          value[source]
        } else {
          (value[source] >> bits) | (next << (Limb::BITS - bits))
        };
        i += 1;
      }
      shifted
    }

    /// The number of ones of `value` in `bits` bits, without the copies of the sign above them.
    /// Requires `bits <= B * N`.
    #[inline]
    pub(crate) const fn count_ones_bits<const N: usize>(value: &[Limb; N], bits: u32) -> u32 {
      $unsigned::count_ones(&$unsigned::zero_extend(value, bits))
    }

    /// The number of zeros of `value` from bit `bits - 1` down to its highest one: none for a
    /// negative value, whose bit `bits - 1` is one, and `bits` for zero. Requires `bits <= B * N`
    /// and `value` within the range of `bits` bits.
    pub(crate) const fn leading_zeros_bits<const N: usize>(value: &[Limb; N], bits: u32) -> u32 {
      if is_negative(value) {
        0
      } else {
        $unsigned::leading_zeros_bits(value, bits)
      }
    }

    /// The number of zeros of `value` below its lowest one, or `bits` for zero: a value other than
    /// zero has a one below index `bits`, so the lower layer counts them.
    #[inline]
    pub(crate) const fn trailing_zeros_bits<const N: usize>(value: &[Limb; N], bits: u32) -> u32 {
      $unsigned::trailing_zeros_bits(value, bits)
    }

    /// The number of ones of `value` from bit `bits - 1` down: the leading zeros of the complement
    /// for a negative value, and none for a value at least zero. Requires `bits <= B * N` and
    /// `value` within the range of `bits` bits.
    pub(crate) const fn leading_ones_bits<const N: usize>(value: &[Limb; N], bits: u32) -> u32 {
      if is_negative(value) {
        $unsigned::leading_zeros_bits(&not_bits(value, bits), bits)
      } else {
        0
      }
    }

    /// The number of ones of `value` from bit 0 up, the trailing zeros of its complement: `bits`
    /// for -1. Requires `bits <= B * N` and `value` within the range of `bits` bits.
    #[inline]
    pub(crate) const fn trailing_ones_bits<const N: usize>(value: &[Limb; N], bits: u32) -> u32 {
      $unsigned::trailing_zeros_bits(&not_bits(value, bits), bits)
    }

    /// The index of the highest one of `value` in `bits` bits: the sign bit, `bits - 1`, for a
    /// negative value, or `None` for zero. Requires `value` within the range of `bits` bits.
    pub(crate) const fn highest_one_bits<const N: usize>(
      value: &[Limb; N],
      bits: u32,
    ) -> Option<u32> {
      if is_negative(value) {
        Some(bits - 1)
      } else {
        $unsigned::checked_ilog2(value)
      }
    }

    /// The index of the lowest one of `value`, or `None` for zero, as the lower layer finds it: a
    /// value other than zero has a one below the copies of its sign.
    #[inline]
    pub(crate) const fn lowest_one<const N: usize>(value: &[Limb; N]) -> Option<u32> {
      $unsigned::lowest_one(value)
    }

    /// `value` with only its highest one in `bits` bits: `MIN` for a negative value, whose highest
    /// one is the sign bit, or zero for zero. Requires `bits <= B * N` and `value` within the range
    /// of `bits` bits.
    pub(crate) const fn isolate_highest_one_bits<const N: usize>(
      value: &[Limb; N],
      bits: u32,
    ) -> [Limb; N] {
      if is_negative(value) {
        min_of_bits(bits)
      } else {
        $unsigned::isolate_highest_one_bits(value, bits)
      }
    }

    /// `value` with only its lowest one, sign-extended from `bits` bits: `MIN` for `MIN`, whose
    /// lowest one is the sign bit, or zero for zero. Requires `bits <= B * N` and `value` within
    /// the range of `bits` bits.
    pub(crate) const fn isolate_lowest_one_bits<const N: usize>(
      value: &[Limb; N],
      bits: u32,
    ) -> [Limb; N] {
      match $unsigned::lowest_one(value) {
        Some(index) => sign_extend(&$unsigned::pow2(index), bits),
        None => [0; N],
      }
    }

    /// The `bits` bits of `value` in reverse order, read in two's complement: old bit 0 is the new
    /// sign bit. Requires `bits <= B * N` and `value` within the range of `bits` bits.
    pub(crate) const fn reverse_bits_bits<const N: usize>(
      value: &[Limb; N],
      bits: u32,
    ) -> [Limb; N] {
      let pattern = $unsigned::zero_extend(value, bits);
      sign_extend(&$unsigned::reverse_bits_bits(&pattern, bits), bits)
    }

    /// The `bits` bits of `value` rotated left by `n` places, read in two's complement, so that
    /// the rotation can change the sign. Requires `bits <= B * N` and `value` within the range of
    /// `bits` bits.
    pub(crate) const fn rotate_left_bits<const N: usize>(
      value: &[Limb; N],
      n: u32,
      bits: u32,
    ) -> [Limb; N] {
      let pattern = $unsigned::zero_extend(value, bits);
      sign_extend(&$unsigned::rotate_left_bits(&pattern, n, bits), bits)
    }

    /// The `bits` bits of `value` rotated right by `n` places, read in two's complement, as
    /// [`rotate_left_bits`] reads them.
    pub(crate) const fn rotate_right_bits<const N: usize>(
      value: &[Limb; N],
      n: u32,
      bits: u32,
    ) -> [Limb; N] {
      let pattern = $unsigned::zero_extend(value, bits);
      sign_extend(&$unsigned::rotate_right_bits(&pattern, n, bits), bits)
    }

    /// `value << (rhs % bits)` wrapped to `bits` bits, whose bit `bits - 1` is the new sign, and
    /// whether `rhs >= bits`; zero bits overflow on every amount. Requires `bits <= B * N` and
    /// `value` within the range of `bits` bits.
    pub(crate) const fn overflowing_shl_bits<const N: usize>(
      value: &[Limb; N],
      rhs: u32,
      bits: u32,
    ) -> ([Limb; N], bool) {
      // Zero bits hold only zero, and take no remainder by zero.
      if bits == 0 {
        return ([0; N], true);
      }
      let amount = if rhs < bits { rhs } else { rhs % bits };
      (sign_extend(&$unsigned::shift_left(value, amount), bits), rhs >= bits)
    }

    /// `value >> (rhs % bits)`, an arithmetic shift, and whether `rhs >= bits`, as
    /// [`overflowing_shl_bits`] gives them. Requires `value` within the range of `bits` bits.
    pub(crate) const fn overflowing_shr_bits<const N: usize>(
      value: &[Limb; N],
      rhs: u32,
      bits: u32,
    ) -> ([Limb; N], bool) {
      if bits == 0 {
        return ([0; N], true);
      }
      let amount = if rhs < bits { rhs } else { rhs % bits };
      (sar(value, amount), rhs >= bits)
    }

    /// `value << rhs` wrapped to `bits` bits, which is zero for `rhs >= bits`. Requires
    /// `bits <= B * N`.
    #[inline]
    pub(crate) const fn unbounded_shl_bits<const N: usize>(
      value: &[Limb; N],
      rhs: u32,
      bits: u32,
    ) -> [Limb; N] {
      sign_extend(&$unsigned::shift_left(value, rhs), bits)
    }

    /// `value >> rhs`, an arithmetic shift, which is 0 or -1 for `rhs >= bits`: the bits of
    /// `value` from bit `bits - 1` up already copy the sign, so the width does not change it.
    #[inline]
    pub(crate) const fn unbounded_shr_bits<const N: usize>(
      value: &[Limb; N],
      rhs: u32,
      _bits: u32,
    ) -> [Limb; N] {
      sar(value, rhs)
    }

    /// `value << rhs` if every bit that the shift moves out of `bits` bits, and the new bit
    /// `bits - 1`, equal the sign, so that `>> rhs` gives `value` back, or `None` otherwise.
    /// Requires `bits <= B * N` and `value` within the range of `bits` bits.
    pub(crate) const fn shl_exact_bits<const N: usize>(
      value: &[Limb; N],
      rhs: u32,
      bits: u32,
    ) -> Option<[Limb; N]> {
      // `rhs` below the leading zeros or ones keeps the sign and every bit above it, so the shifted
      // limbs hold the copies of the sign above the width; and `rhs < bits`.
      if rhs < leading_zeros_bits(value, bits) || rhs < leading_ones_bits(value, bits) {
        Some($unsigned::shift_left(value, rhs))
      } else {
        None
      }
    }

    /// `value >> rhs`, an arithmetic shift, if `rhs < bits` and the shift moves no one out, so
    /// that `<< rhs` gives `value` back, or `None` otherwise. Requires `value` within the range of
    /// `bits` bits.
    pub(crate) const fn shr_exact_bits<const N: usize>(
      value: &[Limb; N],
      rhs: u32,
      bits: u32,
    ) -> Option<[Limb; N]> {
      if rhs < bits && rhs <= trailing_zeros_bits(value, bits) {
        Some(sar(value, rhs))
      } else {
        None
      }
    }

    /// The `bits` bits of `value`, its two's complement, as an unsigned pattern of `bits` bits.
    /// Requires `bits <= B * N`.
    #[inline]
    pub(crate) const fn cast_unsigned_bits<const N: usize>(
      value: &[Limb; N],
      bits: u32,
    ) -> [Limb; N] {
      $unsigned::zero_extend(value, bits)
    }

    /// `value` as an unsigned integer of `bits` bits, or zero if `value` is negative, which is
    /// when bit `bits - 1` is set. Requires `bits <= B * N` and `value` within the range of `bits`
    /// bits.
    pub(crate) const fn saturating_cast_unsigned_bits<const N: usize>(
      value: &[Limb; N],
      bits: u32,
    ) -> [Limb; N] {
      if bit(value, bits.wrapping_sub(1), bits) {
        // A negative value saturates to zero.
        [0; N]
      } else {
        cast_unsigned_bits(value, bits)
      }
    }

    /// The bit at `index` of `value`, or `false` for `index >= bits`, which reads no copy of the
    /// sign. Requires `bits <= B * N`.
    #[inline]
    pub(crate) const fn bit<const N: usize>(value: &[Limb; N], index: u32, bits: u32) -> bool {
      $unsigned::bit(value, index, bits)
    }

    /// `value` with the bit at `index` set if `set`, and cleared otherwise, sign-extended from
    /// `bits` bits, so that a change of the sign bit changes its copies, or `None` for
    /// `index >= bits`. Requires `bits <= B * N` and `value` within the range of `bits` bits.
    pub(crate) const fn with_bit<const N: usize>(
      value: &[Limb; N],
      index: u32,
      set: bool,
      bits: u32,
    ) -> Option<[Limb; N]> {
      match $unsigned::with_bit(value, index, set, bits) {
        Some(changed) => Some(sign_extend(&changed, bits)),
        None => None,
      }
    }

    /// The 64-bit word at `index` of `value` in two's complement, its bits from `64 * index` to
    /// `64 * index + 63`, as the lower layer's `extended_word` gives it: copies of the sign beyond
    /// the limbs.
    #[inline]
    pub(crate) const fn word<const N: usize>(value: &[Limb; N], index: usize) -> u64 {
      let fill = if is_negative(value) { Limb::MAX } else { 0 };
      $unsigned::extended_word(value, index, fill)
    }

    /// Compares `value` with `rhs`, as integers: a negative `value` is below every `rhs`.
    #[inline]
    pub(crate) const fn compare_u128<const N: usize>(value: &[Limb; N], rhs: u128) -> Ordering {
      if is_negative(value) {
        Ordering::Less
      } else {
        $unsigned::compare_u128(value, rhs)
      }
    }

    /// Compares `value` with `rhs`, as integers: `value` compares as an `i128` if its bits from
    /// index 127 up copy its sign, and is otherwise below every `rhs` if negative and above every
    /// `rhs` if not.
    pub(crate) const fn compare_i128<const N: usize>(value: &[Limb; N], rhs: i128) -> Ordering {
      let negative = is_negative(value);
      let fill = if negative { Limb::MAX } else { 0 };
      match $unsigned::low_u128(value, fill) {
        // The limbs above the low 128 bits and bit 127 copy the sign: an `i128` holds the value.
        (low, true) if ((low as i128) < 0) == negative => order::cmp_i128(low as i128, rhs),
        _ if negative => Ordering::Less,
        _ => Ordering::Greater,
      }
    }

    /// The bits of the float of `format` nearest to `value`, ties to even, or of the infinity of
    /// its sign beyond the range of the float, as the lower layer's `magnitude_float_bits` gives
    /// them for the magnitude and the sign of `value`: zero is `+0.0`.
    #[inline]
    pub(crate) const fn float_bits<const N: usize>(value: &[Limb; N], format: FloatFormat) -> u64 {
      $unsigned::magnitude_float_bits(&unsigned_abs(value), is_negative(value), format)
    }

    /// `rounded`, a float rounded to an integer, as a value of `bits` bits in two's complement, or
    /// the error of a value outside their range: `NegOverflow` below `-2^(bits - 1)`, and
    /// `PosOverflow` above `2^(bits - 1) - 1`. Requires `bits <= B * N`.
    pub(crate) const fn place_magnitude<const N: usize>(
      rounded: Rounded,
      bits: u32,
    ) -> Result<[Limb; N], FromFloatError> {
      match float::check_range(rounded, bits, true) {
        Ok(()) => {
          // The magnitude is at most `2^(bits - 1)`, so its negation holds copies of its sign from
          // bit `bits - 1` up, as a value of `bits` bits does.
          let magnitude =
            $unsigned::shift_left(&$unsigned::from_u64(rounded.significand), rounded.shift);
          Ok(if rounded.negative {
            wrapping_neg(&magnitude)
          } else {
            magnitude
          })
        }
        Err(error) => Err(error),
      }
    }

    /// The integer in `bytes`, least significant byte first, or most significant first if
    /// `big_endian`, in two's complement of `8 * bytes.len()` bits, as a value of `bits` bits, or
    /// `None` if it is outside their range, whatever the number of bytes. No bytes are zero, and
    /// zero bits hold only zero. Requires `bits <= B * N`.
    pub(crate) const fn checked_read_bytes<const N: usize>(
      bytes: &[u8],
      big_endian: bool,
      bits: u32,
    ) -> Option<[Limb; N]> {
      if slice::fits_bits(bytes, big_endian, bits, true) {
        let fill = slice::fill(bytes, big_endian, true);
        Some($unsigned::read_bytes(bytes, big_endian, fill))
      } else {
        None
      }
    }

    /// The integer in `bytes`, read as [`checked_read_bytes`] reads it, reduced modulo `2^bits`
    /// into the range of `bits` bits, or zero for zero bits. Requires `bits <= B * N`.
    #[inline]
    pub(crate) const fn wrapping_read_bytes<const N: usize>(
      bytes: &[u8],
      big_endian: bool,
      bits: u32,
    ) -> [Limb; N] {
      let fill = slice::fill(bytes, big_endian, true);
      sign_extend(&$unsigned::read_bytes(bytes, big_endian, fill), bits)
    }

    /// The integer in `bytes`, read as [`checked_read_bytes`] reads it, or the bound of the range
    /// of `bits` bits that it passes: the smallest value below the range, which only a negative
    /// integer is, and the largest above it. Requires `bits <= B * N`.
    pub(crate) const fn saturating_read_bytes<const N: usize>(
      bytes: &[u8],
      big_endian: bool,
      bits: u32,
    ) -> [Limb; N] {
      match checked_read_bytes(bytes, big_endian, bits) {
        Some(value) => value,
        None if slice::fill(bytes, big_endian, true) != 0 => min_of_bits(bits),
        None => max_of_bits(bits),
      }
    }

    /// The low `bits / 8` bytes of `value` in reverse order, read in two's complement: the old
    /// low byte holds the new sign bit. Requires `bits` to be a multiple of 8, `bits <= B * N` and
    /// `value` within the range of `bits` bits.
    #[inline]
    pub(crate) const fn swap_bytes_bits<const N: usize>(value: &[Limb; N], bits: u32) -> [Limb; N] {
      let pattern = $unsigned::zero_extend(value, bits);
      sign_extend(&$unsigned::swap_bytes_bits(&pattern, bits), bits)
    }

    /// The integer in `limbs`, least significant limb first, in two's complement of the bits of
    /// its limbs, as a value of `bits` bits, or `None` if it is outside their range, whatever the
    /// number of limbs. No limbs are zero, and zero bits hold only zero. Requires `bits <= B * N`.
    pub(crate) const fn checked_read_limbs<const N: usize>(
      limbs: &[Limb],
      bits: u32,
    ) -> Option<[Limb; N]> {
      if $unsigned::limbs_fit_bits(limbs, bits, true) {
        Some($unsigned::read_limbs(limbs, $unsigned::limbs_fill(limbs, true)))
      } else {
        None
      }
    }

    /// The integer in `limbs`, read as [`checked_read_limbs`] reads it, reduced modulo `2^bits`
    /// into the range of `bits` bits, or zero for zero bits. Requires `bits <= B * N`.
    #[inline]
    pub(crate) const fn wrapping_read_limbs<const N: usize>(
      limbs: &[Limb],
      bits: u32,
    ) -> [Limb; N] {
      let fill = $unsigned::limbs_fill(limbs, true);
      sign_extend(&$unsigned::read_limbs(limbs, fill), bits)
    }

    /// The integer in `limbs`, read as [`checked_read_limbs`] reads it, or the bound of the range
    /// of `bits` bits that it passes: the smallest value below the range, which only a negative
    /// integer is, and the largest above it. Requires `bits <= B * N`.
    pub(crate) const fn saturating_read_limbs<const N: usize>(
      limbs: &[Limb],
      bits: u32,
    ) -> [Limb; N] {
      match checked_read_limbs(limbs, bits) {
        Some(value) => value,
        None if $unsigned::limbs_fill(limbs, true) != 0 => min_of_bits(bits),
        None => max_of_bits(bits),
      }
    }
  };
}

pub(crate) use limb_layer;

#[cfg(test)]
pub(super) mod tests;
