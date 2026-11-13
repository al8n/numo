//! The wide multiplication of `Uint` and `Int`: `carrying_mul` and `carrying_mul_add` of both, and
//! `carrying_add` and `borrowing_sub` of `Int`, through the traits against `num-bigint` in every
//! storage at the widths of the arithmetic, through the inherent `const fn`s and the traits against
//! `i128` at every value of up to 3 bits, their relations with the multiplication of the families,
//! their edges, and their constants.

use typenum::U3;

use super::*;

/// `exact` split at bit `bits`: the remainder and the quotient of its division by `2^bits`,
/// rounded down.
fn halves(exact: &BigInt, bits: u32) -> (BigInt, BigInt) {
  let high = exact >> bits;
  (exact - (&high << bits), high)
}

/// Checks `carrying_mul` and `carrying_mul_add` of `T`, unsigned, of `bits` bits, through the trait
/// against `num-bigint`, at every pair of `samples`, with the smallest and with the largest sample
/// as the carry and as the addend.
#[inline(never)]
fn check_unsigned_products<T>(bits: u32, samples: &[BigInt])
where
  T: Big + UnsignedFixedInteger,
{
  let values: Vec<T> = samples.iter().map(T::from_big).collect();
  // The smallest and the largest sample, which `big_samples` sorts.
  let ends = [0, samples.len() - 1];
  for (a, x) in samples.iter().zip(&values) {
    for (b, y) in samples.iter().zip(&values) {
      let product = a * b;
      for end in ends {
        let (c, z) = (&samples[end], &values[end]);
        let (low, high) = x.carrying_mul(y, z);
        assert_eq!(
          (low.to_big(), high.to_big()),
          halves(&(&product + c), bits),
          "{a} * {b} + {c}"
        );
        let (low, high) = x.carrying_mul_add(y, z, z);
        assert_eq!(
          (low.to_big(), high.to_big()),
          halves(&(&product + c + c), bits),
          "{a} * {b} + {c} + {c}"
        );
      }
    }
  }
}

/// Checks `carrying_mul` and `carrying_mul_add` of `T`, signed, of `bits` bits, as
/// [`check_unsigned_products`] checks them, the low half read unsigned, and `carrying_add` and
/// `borrowing_sub` at every pair of `samples` with each carry and borrow, through the trait against
/// `num-bigint`.
#[inline(never)]
fn check_signed_products<T>(bits: u32, samples: &[BigInt])
where
  T: Big + SignedFixedInteger,
  T::Unsigned: Big,
{
  let (min, max) = (bigint::min(bits, true), bigint::max(bits, true));
  // `exact` wrapped to the range of `bits` bits, and whether it is outside that range.
  let wrapped = |exact: BigInt| {
    let outside = exact < min || max < exact;
    (bigint::wrap(&exact, bits, &min), outside)
  };
  let values: Vec<T> = samples.iter().map(T::from_big).collect();
  // The smallest and the largest sample, `MIN` and `MAX`, which `big_samples` sorts.
  let ends = [0, samples.len() - 1];
  for (a, x) in samples.iter().zip(&values) {
    for (b, y) in samples.iter().zip(&values) {
      for carry in [false, true] {
        let one = u8::from(carry);
        let (sum, overflow) = x.carrying_add(y, carry);
        assert_eq!(
          (sum.to_big(), overflow),
          wrapped(a + b + one),
          "{a} + {b} + {one}"
        );
        let (difference, overflow) = x.borrowing_sub(y, carry);
        assert_eq!(
          (difference.to_big(), overflow),
          wrapped(a - b - one),
          "{a} - {b} - {one}"
        );
      }
      let product = a * b;
      for end in ends {
        let (c, z) = (&samples[end], &values[end]);
        let (low, high) = x.carrying_mul(y, z);
        assert_eq!(
          (low.to_big(), high.to_big()),
          halves(&(&product + c), bits),
          "{a} * {b} + {c}"
        );
        let (low, high) = x.carrying_mul_add(y, z, z);
        assert_eq!(
          (low.to_big(), high.to_big()),
          halves(&(&product + c + c), bits),
          "{a} * {b} + {c} + {c}"
        );
      }
    }
  }
}

/// Checks the wide multiplication of `Uint` and `Int` of `$bits` bits, the width `$width`, in each
/// pair of storages written `unsigned, signed;`, against `num-bigint`, with `$count` random
/// samples, as `agree!` checks the arithmetic.
macro_rules! wide_agree {
  ($width:ty, $bits:literal, $count:literal; $($uint:ty, $int:ty);+ $(;)?) => {{
    let unsigned = big_samples($bits, false, $count);
    let signed = big_samples($bits, true, $count);
    $(
      check_unsigned_products::<Uint<$width, $uint>>($bits, &unsigned);
      check_signed_products::<Int<$width, $int>>($bits, &signed);
    )+
  }};
}

/// The edge and random values of 1 to 4096 bits of `storages_agree_on_the_arithmetic`, in every
/// storage that holds them, native integers up to 128 bits and arrays of each limb type: the
/// products with a carry and an addend of both signs and the carries of `Int` agree with
/// `num-bigint` through the traits. Under Miri, the widths above 256 bits are left out.
#[test]
#[cfg_attr(
  all(miri, target_pointer_width = "32"),
  ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
)]
fn storages_agree_on_the_wide_products() {
  wide_agree!(U1, 1, 10; u8, i8; u16, i16; u32, i32; u64, i64; u128, i128);
  wide_agree!(U1, 1, 10; [u8; 1], [u8; 1]; [u16; 1], [u16; 1]; [u64; 1], [u64; 1]);
  wide_agree!(U1, 1, 10; [u128; 1], [u128; 1]);
  wide_agree!(U7, 7, 10; u8, i8; u16, i16; u32, i32; u64, i64; u128, i128);
  wide_agree!(U7, 7, 10; [u8; 1], [u8; 1]; [u16; 1], [u16; 1]; [u32; 1], [u32; 1]);
  wide_agree!(U7, 7, 10; [u64; 1], [u64; 1]; [u128; 1], [u128; 1]);
  wide_agree!(U8, 8, 10; u8, i8; u16, i16; u32, i32; u64, i64; u128, i128);
  wide_agree!(U8, 8, 10; [u8; 1], [u8; 1]; [u32; 1], [u32; 1]; [u128; 1], [u128; 1]);
  wide_agree!(U9, 9, 10; u16, i16; u32, i32; u64, i64; u128, i128);
  wide_agree!(U9, 9, 10; [u8; 2], [u8; 2]; [u16; 1], [u16; 1]; [u64; 1], [u64; 1]);
  wide_agree!(U9, 9, 10; [u128; 1], [u128; 1]);
  wide_agree!(U63, 63, 10; u64, i64; u128, i128; [u8; 8], [u8; 8]; [u16; 4], [u16; 4]);
  wide_agree!(U63, 63, 10; [u32; 2], [u32; 2]; [u64; 1], [u64; 1]; [u128; 1], [u128; 1]);
  wide_agree!(U64, 64, 10; u64, i64; u128, i128; [u8; 8], [u8; 8]; [u16; 4], [u16; 4]);
  wide_agree!(U64, 64, 10; [u32; 2], [u32; 2]; [u64; 1], [u64; 1]; [u128; 1], [u128; 1]);
  wide_agree!(U65, 65, 10; u128, i128; [u8; 9], [u8; 9]; [u16; 5], [u16; 5]; [u32; 3], [u32; 3]);
  wide_agree!(U65, 65, 10; [u64; 2], [u64; 2]; [u128; 1], [u128; 1]);
  wide_agree!(U100, 100, 10; u128, i128; [u8; 13], [u8; 13]; [u16; 7], [u16; 7]);
  wide_agree!(U100, 100, 10; [u32; 4], [u32; 4]; [u64; 2], [u64; 2]; [u128; 1], [u128; 1]);
  wide_agree!(U127, 127, 10; u128, i128; [u8; 16], [u8; 16]; [u32; 4], [u32; 4]);
  wide_agree!(U127, 127, 10; [u64; 2], [u64; 2]; [u128; 1], [u128; 1]);
  wide_agree!(U128, 128, 10; u128, i128; [u8; 16], [u8; 16]; [u16; 8], [u16; 8]);
  wide_agree!(U128, 128, 10; [u32; 4], [u32; 4]; [u64; 2], [u64; 2]; [u128; 1], [u128; 1]);
  wide_agree!(U129, 129, 10; [u8; 17], [u8; 17]; [u16; 9], [u16; 9]; [u32; 5], [u32; 5]);
  wide_agree!(U129, 129, 10; [u64; 3], [u64; 3]; [u128; 2], [u128; 2]);
  wide_agree!(U200, 200, 10; [u8; 25], [u8; 25]; [u16; 13], [u16; 13]; [u32; 7], [u32; 7]);
  wide_agree!(U200, 200, 10; [u64; 4], [u64; 4]; [u128; 2], [u128; 2]);
  wide_agree!(U256, 256, 10; [u8; 32], [u8; 32]; [u16; 16], [u16; 16]; [u32; 8], [u32; 8]);
  wide_agree!(U256, 256, 10; [u64; 4], [u64; 4]; [u128; 2], [u128; 2]);
  if !cfg!(miri) {
    wide_agree!(U4096, 4096, 2; [u8; 512], [u8; 512]; [u16; 256], [u16; 256]);
    wide_agree!(U4096, 4096, 2; [u32; 128], [u32; 128]; [u64; 64], [u64; 64]);
    wide_agree!(U4096, 4096, 2; [u128; 32], [u128; 32]);
  }
}

/// Checks every value of `$bits` bits, at most 3, as each operand, the carry and the addend of
/// `carrying_mul` and `carrying_mul_add` of `$uint` and `$int`, which `$unsigned` and `$signed`
/// create from their values, and every pair with each carry and borrow of `carrying_add` and
/// `borrowing_sub` of `$int`, through the inherent `const fn`s and the traits, which must agree,
/// against `i128` arithmetic.
macro_rules! every_wide {
  ($bits:expr, $uint:ty, $int:ty, $unsigned:expr, $signed:expr) => {{
    let unsigned: fn(i128) -> $uint = $unsigned;
    let signed: fn(i128) -> $int = $signed;
    let bits: u32 = $bits;
    let modulus = 1i128 << bits;
    // `exact` split at bit `bits`: the remainder, and the quotient rounded down.
    let split = |exact: i128| (exact.rem_euclid(modulus), exact.div_euclid(modulus));
    let (min, max) = range(bits, false);
    for a in min..=max {
      for b in min..=max {
        for c in min..=max {
          let (x, y, z) = (unsigned(a), unsigned(b), unsigned(c));
          let (low, high) = split(a * b + c);
          assert_eq!(
            both!(UnsignedFixedInteger::carrying_mul(&x, &y, &z)),
            (unsigned(low), unsigned(high)),
            "{a} * {b} + {c} in {bits} bits"
          );
          for d in min..=max {
            let (low, high) = split(a * b + c + d);
            assert_eq!(
              both!(UnsignedFixedInteger::carrying_mul_add(
                &x,
                &y,
                &z,
                &unsigned(d)
              )),
              (unsigned(low), unsigned(high)),
              "{a} * {b} + {c} + {d} in {bits} bits"
            );
          }
        }
      }
    }
    let (min, max) = range(bits, true);
    for a in min..=max {
      for b in min..=max {
        let (x, y) = (signed(a), signed(b));
        for carry in [false, true] {
          let one = i128::from(carry);
          let (sum, overflow) = wrap(a + b + one, bits, true);
          assert_eq!(
            both!(SignedFixedInteger::carrying_add(&x, &y, carry)),
            (signed(sum), overflow),
            "{a} + {b} + {one} in {bits} bits"
          );
          let (difference, overflow) = wrap(a - b - one, bits, true);
          assert_eq!(
            both!(SignedFixedInteger::borrowing_sub(&x, &y, carry)),
            (signed(difference), overflow),
            "{a} - {b} - {one} in {bits} bits"
          );
        }
        for c in min..=max {
          let z = signed(c);
          let (low, high) = split(a * b + c);
          assert_eq!(
            both!(SignedFixedInteger::carrying_mul(&x, &y, &z)),
            (unsigned(low), signed(high)),
            "{a} * {b} + {c} in {bits} bits"
          );
          for d in min..=max {
            let (low, high) = split(a * b + c + d);
            assert_eq!(
              both!(SignedFixedInteger::carrying_mul_add(&x, &y, &z, &signed(d))),
              (unsigned(low), signed(high)),
              "{a} * {b} + {c} + {d} in {bits} bits"
            );
          }
        }
      }
    }
  }};
}

/// Runs [`every_wide!`] at the width `$width` of `$bits` bits in `u8` and `i8`, in arrays of one
/// byte and of one 64-bit limb, and in `u128` and `i128`.
macro_rules! every_wide_storage {
  ($width:ty, $bits:expr) => {{
    every_wide!(
      $bits,
      Uint<$width, u8>,
      Int<$width, i8>,
      |v| Uint::<$width, u8>::new(v as u8),
      |v| Int::<$width, i8>::new(v as i8)
    );
    every_wide!(
      $bits,
      Uint<$width, [u8; 1]>,
      Int<$width, [u8; 1]>,
      |v| Uint::<$width, [u8; 1]>::checked_from_limbs(limbs_of(v)).expect("in range"),
      |v| Int::<$width, [u8; 1]>::checked_from_limbs(limbs_of(v)).expect("in range")
    );
    every_wide!(
      $bits,
      Uint<$width, [u64; 1]>,
      Int<$width, [u64; 1]>,
      |v| Uint::<$width, [u64; 1]>::checked_from_limbs(limbs_of(v)).expect("in range"),
      |v| Int::<$width, [u64; 1]>::checked_from_limbs(limbs_of(v)).expect("in range")
    );
    every_wide!(
      $bits,
      Uint<$width, u128>,
      Int<$width, i128>,
      |v| Uint::<$width, u128>::new(v as u128),
      |v| Int::<$width, i128>::new(v)
    );
  }};
}

/// Every value of 0 to 3 bits, or under Miri of 0 to 2, as each operand, the carry and the addend
/// of the products, and every pair with each carry and borrow of the carries of `Int`, in native
/// and array storages, through the inherent `const fn`s and the traits.
#[test]
fn every_wide_product_of_three_bits() {
  every_wide_storage!(U0, 0);
  every_wide_storage!(U1, 1);
  every_wide_storage!(U2, 2);
  if !tier().is_miri() {
    every_wide_storage!(U3, 3);
  }
}

/// Checks, at every pair of `samples` of `T`, unsigned, that the low half of a product with a zero
/// carry is the wrapped product, and that the high half is zero exactly when the product overflows.
#[inline(never)]
fn unsigned_relations<T>(samples: &[BigInt])
where
  T: Big + FixedInteger + UnsignedFixedInteger + OverflowingOps + WrappingOps,
{
  let values: Vec<T> = samples.iter().map(T::from_big).collect();
  let zero = T::zero();
  for x in &values {
    for y in &values {
      let (low, high) = x.carrying_mul(y, &zero);
      assert_eq!(low, x.wrapping_mul(y), "{x:?} * {y:?}");
      assert_eq!(!high.is_zero(), x.overflowing_mul(y).1, "{x:?} * {y:?}");
    }
  }
}

/// Checks, at every pair of `samples` of `T`, signed, of `bits` bits, that the low half of a
/// product with a zero carry, unsigned, has the bits of the wrapped product.
#[inline(never)]
fn signed_relations<T>(bits: u32, samples: &[BigInt])
where
  T: Big + FixedInteger + SignedFixedInteger + WrappingOps,
  T::Unsigned: Big,
{
  let values: Vec<T> = samples.iter().map(T::from_big).collect();
  let zero = T::zero();
  for x in &values {
    for y in &values {
      let (low, _) = x.carrying_mul(y, &zero);
      // The bits of the wrapped product, read unsigned.
      let bits_of_product = bigint::wrap(&x.wrapping_mul(y).to_big(), bits, &BigInt::ZERO);
      assert_eq!(low.to_big(), bits_of_product, "{x:?} * {y:?}");
    }
  }
}

/// The low half of a product with a zero carry has the bits of the wrapped product, and for `Uint`
/// the high half is zero exactly when the product overflows, at the samples of 7, 64, 65 and 256
/// bits in the default storages.
#[test]
#[cfg_attr(
  all(miri, target_pointer_width = "32"),
  ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
)]
fn products_relate_to_the_multiplication() {
  unsigned_relations::<u7>(&big_samples(7, false, 10));
  signed_relations::<i7>(7, &big_samples(7, true, 10));
  unsigned_relations::<Uint<U64>>(&big_samples(64, false, 10));
  signed_relations::<Int<U64>>(64, &big_samples(64, true, 10));
  unsigned_relations::<Uint<U65>>(&big_samples(65, false, 10));
  signed_relations::<Int<U65>>(65, &big_samples(65, true, 10));
  unsigned_relations::<u256>(&big_samples(256, false, 10));
  signed_relations::<i256>(256, &big_samples(256, true, 10));
}

/// The edges of the products and of the carries: at zero bits every result is zero and a carry or
/// a borrow leaves the range; at one bit `Int` holds -1 and 0; and at 7 bits the limits give the
/// largest and the smallest exact values, and a carry or a borrow whose two steps both overflow
/// stays in range.
#[test]
fn products_and_carries_at_the_edges() {
  {
    type U = Uint<U0, u8>;
    type I = Int<U0, [u64; 0]>;
    type M = Uint<U0, [u64; 0]>;
    assert_eq!(U::ZERO.carrying_mul(&U::ZERO, &U::ZERO), (U::ZERO, U::ZERO));
    assert_eq!(
      U::ZERO.carrying_mul_add(&U::ZERO, &U::ZERO, &U::ZERO),
      (U::ZERO, U::ZERO)
    );
    assert_eq!(I::ZERO.carrying_mul(&I::ZERO, &I::ZERO), (M::ZERO, I::ZERO));
    assert_eq!(
      I::ZERO.carrying_mul_add(&I::ZERO, &I::ZERO, &I::ZERO),
      (M::ZERO, I::ZERO)
    );
    assert_eq!(I::ZERO.carrying_add(&I::ZERO, true), (I::ZERO, true));
    assert_eq!(I::ZERO.borrowing_sub(&I::ZERO, true), (I::ZERO, true));
  }
  {
    type U = Uint<U1, u8>;
    type I = Int<U1, i8>;
    // 1 * 1 + 1 + 1 is 3, which is 1 + 1 * 2.
    assert_eq!(
      U::ONE.carrying_mul_add(&U::ONE, &U::ONE, &U::ONE),
      (U::ONE, U::ONE)
    );
    // -1 * -1 + 0 is 1, which is 1 + 0 * 2, and -1 * 0 - 1 - 1 is -2, which is 0 + -1 * 2.
    assert_eq!(I::MIN.carrying_mul(&I::MIN, &I::ZERO), (U::ONE, I::ZERO));
    assert_eq!(
      I::MIN.carrying_mul_add(&I::ZERO, &I::MIN, &I::MIN),
      (U::ZERO, I::MIN)
    );
  }
  // 127 * 127 + 127 + 127 is 16383, which is 127 + 127 * 128.
  assert_eq!(
    u7::MAX.carrying_mul_add(&u7::MAX, &u7::MAX, &u7::MAX),
    (u7::MAX, u7::MAX)
  );
  // -64 * -64 + 63 + 63 is 4222, which is 126 + 32 * 128, and -64 * 63 - 64 - 64 is -4160, which
  // is 64 + -33 * 128.
  assert_eq!(
    i7::MIN.carrying_mul_add(&i7::MIN, &i7::MAX, &i7::MAX),
    (u7::new(126), i7::new(32))
  );
  assert_eq!(
    i7::MIN.carrying_mul_add(&i7::MAX, &i7::MIN, &i7::MIN),
    (u7::new(64), i7::new(-33))
  );
  // 63 + 0 + 1 is 64, outside the range; -1 + -64 + 1 is -64, inside it, though both steps of the
  // carry overflow. The borrows mirror them.
  assert_eq!(i7::MAX.carrying_add(&i7::ZERO, true), (i7::MIN, true));
  assert_eq!(i7::new(-1).carrying_add(&i7::MIN, true), (i7::MIN, false));
  assert_eq!(i7::MIN.borrowing_sub(&i7::ZERO, true), (i7::MAX, true));
  assert_eq!(i7::ZERO.borrowing_sub(&i7::MIN, true), (i7::MAX, false));
}

/// Each inherent method in a constant.
#[test]
fn wide_products_in_constant_contexts() {
  const PRODUCT: (u7, u7) = u7::MAX.carrying_mul(&u7::MAX, &u7::MAX);
  const SIGNED: (u7, i7) = i7::MIN.carrying_mul(&i7::MIN, &i7::MAX);
  const WIDE: (u4096, u4096) = u4096::MAX.carrying_mul_add(&u4096::MAX, &u4096::MAX, &u4096::MAX);
  const CARRIES: [(i7, bool); 2] = [
    i7::MAX.carrying_add(&i7::ZERO, true),
    i7::MIN.borrowing_sub(&i7::ZERO, true),
  ];
  // 127 * 127 + 127 is 16256, which is 0 + 127 * 128, and -64 * -64 + 63 is 4159, which is
  // 63 + 32 * 128.
  assert_eq!(PRODUCT, (u7::ZERO, u7::MAX));
  assert_eq!(SIGNED, (u7::new(63), i7::new(32)));
  assert_eq!(WIDE, (u4096::MAX, u4096::MAX));
  assert_eq!(CARRIES, [(i7::MIN, true), (i7::MAX, true)]);
}
