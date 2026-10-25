use core::any::TypeId;

use typenum::U;

use super::*;
use crate::TryNewError;

/// Checks the aliases of one native integer: each is `Uint<U<bits>>` or `Int<U<bits>>`, the default
/// storage of its width, stored in the native integer, with the limits of its width.
macro_rules! test_aliases {
  ($($sign:ident $repr:ident: $($name:ident = $bits:tt),+;)+) => {$(
    #[test]
    fn $repr() {
      $(test_aliases!(@alias $sign $repr $name $bits);)+
    }
  )+};
  (@alias unsigned $repr:ident $name:ident $bits:tt) => {
    assert_eq!(TypeId::of::<$name>(), TypeId::of::<Uint<U<$bits>>>());
    assert_eq!(TypeId::of::<$name>(), TypeId::of::<Uint<U<$bits>, $repr>>());
    assert_eq!(<$name>::BITS, $bits);
    assert_eq!(size_of::<$name>(), size_of::<$repr>());
    assert_eq!(<$name>::MIN.value(), 0);
    assert_eq!(<$name>::MAX.value() as u128, (1 << $bits) - 1);
  };
  (@alias signed $repr:ident $name:ident $bits:tt) => {
    assert_eq!(TypeId::of::<$name>(), TypeId::of::<Int<U<$bits>>>());
    assert_eq!(TypeId::of::<$name>(), TypeId::of::<Int<U<$bits>, $repr>>());
    assert_eq!(<$name>::BITS, $bits);
    assert_eq!(size_of::<$name>(), size_of::<$repr>());
    assert_eq!(<$name>::MIN.value() as i128, -(1 << ($bits - 1)));
    assert_eq!(<$name>::MAX.value() as i128, (1 << ($bits - 1)) - 1);
  };
}

native_alias_table!(test_aliases);

/// The aliases in constant contexts, with the layout of their native integers.
#[test]
fn native_aliases_are_const_and_native_sized() {
  const SEVEN: u7 = u7::new(100);
  const REJECTED: Result<i7, TryNewError> = i7::try_new(-65);
  const LARGEST: u128 = u100::MAX.value();
  const SIGN: i8 = i1::MIN.value();
  const QUERIES: [bool; 3] = [
    SEVEN.is_zero(),
    i1::MIN.is_negative(),
    i127::MAX.is_positive(),
  ];

  assert_eq!(SEVEN.value(), 100);
  assert_eq!(REJECTED, Err(TryNewError::new(8)), "-65 needs 8 bits");
  assert_eq!(LARGEST, (1 << 100) - 1);
  assert_eq!(SIGN, -1);
  assert_eq!(QUERIES, [false, true, true]);
  assert_eq!(size_of::<Option<u7>>(), 2);
  assert_eq!(size_of::<i24>(), size_of::<i32>());
  assert_eq!(align_of::<u100>(), align_of::<u128>());
}

/// The most limbs of an alias in `wide_alias_table!`.
const MOST_LIMBS: usize = 16;

/// The limbs that have exactly the bits below `count` set, least significant limb first.
fn limbs_with_low_bits(count: usize, limbs: usize) -> [u64; MOST_LIMBS] {
  let mut out = [0; MOST_LIMBS];
  for bit in 0..count.min(64 * limbs) {
    out[bit / 64] |= 1 << (bit % 64);
  }
  out
}

/// The limbs that have exactly the bits from `first` up set, within `limbs` limbs.
fn limbs_with_high_bits(first: usize, limbs: usize) -> [u64; MOST_LIMBS] {
  let mut out = [0; MOST_LIMBS];
  for bit in first..64 * limbs {
    out[bit / 64] |= 1 << (bit % 64);
  }
  out
}

/// Checks the limits of an unsigned alias of `bits` bits: zero, and the `bits` low bits set.
fn check_unsigned_limits(min: &[u64], max: &[u64], bits: usize) {
  let limbs = min.len();
  assert_eq!(max.len(), limbs);
  assert_eq!(min, &[0; MOST_LIMBS][..limbs], "{bits} bits: MIN");
  assert_eq!(
    max,
    &limbs_with_low_bits(bits, limbs)[..limbs],
    "{bits} bits: MAX"
  );
}

/// Checks the limits of a signed alias of `bits` bits in two's complement: `MAX` has the bits below
/// the sign bit set and `MIN` has the sign bit and every bit above it set.
fn check_signed_limits(min: &[u64], max: &[u64], bits: usize) {
  let limbs = min.len();
  assert_eq!(max.len(), limbs);
  assert_eq!(
    min,
    &limbs_with_high_bits(bits - 1, limbs)[..limbs],
    "{bits} bits: MIN"
  );
  assert_eq!(
    max,
    &limbs_with_low_bits(bits - 1, limbs)[..limbs],
    "{bits} bits: MAX"
  );
}

/// Checks the aliases from 129 to 1024 bits: each is `Uint<U<bits>>` or `Int<U<bits>>`, the default
/// storage of its width, stored in `ceil(bits / 64)` limbs of 64 bits, with the limits of its width.
macro_rules! test_wide_aliases {
  ($($unsigned:ident $signed:ident: $constant:ident, $bits:tt, $limbs:tt;)+) => {
    #[test]
    fn wide_unsigned_aliases() {$(
      assert_eq!(TypeId::of::<$unsigned>(), TypeId::of::<Uint<U<$bits>>>());
      assert_eq!(TypeId::of::<$unsigned>(), TypeId::of::<Uint<U<$bits>, [u64; $limbs]>>());
      assert_eq!(<$unsigned>::BITS, $bits);
      assert_eq!(size_of::<$unsigned>(), 8 * $limbs);
      check_unsigned_limits(&<$unsigned>::MIN.to_limbs(), &<$unsigned>::MAX.to_limbs(), $bits);
    )+}

    #[test]
    fn wide_signed_aliases() {$(
      assert_eq!(TypeId::of::<$signed>(), TypeId::of::<Int<U<$bits>>>());
      assert_eq!(TypeId::of::<$signed>(), TypeId::of::<Int<U<$bits>, [u64; $limbs]>>());
      assert_eq!(<$signed>::BITS, $bits);
      assert_eq!(size_of::<$signed>(), 8 * $limbs);
      check_signed_limits(&<$signed>::MIN.to_limbs(), &<$signed>::MAX.to_limbs(), $bits);
    )+}
  };
}

wide_alias_table!(test_wide_aliases);

/// The wide aliases in constant contexts, with the layout of their limbs.
#[test]
fn wide_aliases_are_const_and_limb_sized() {
  const TOP: [u64; 3] = u129::MAX.to_limbs();
  const ALL: [u64; 3] = u144::MAX.to_limbs();
  const FULL: [u64; 3] = u192::MAX.to_limbs();
  const ZERO: bool = u160::MIN.is_zero();
  const SIGN: [u64; 16] = i1023::MIN.to_limbs();
  const NEGATIVE: bool = i129::MIN.is_negative();

  assert_eq!(TOP, [u64::MAX, u64::MAX, 1]);
  assert_eq!(ALL, [u64::MAX, u64::MAX, 0xffff]);
  assert_eq!(FULL, [u64::MAX; 3]);
  assert_eq!(SIGN[..15], [0; 15]);
  assert_eq!(SIGN[15], 3 << 62);
  assert_eq!([ZERO, NEGATIVE], [true, true]);
  assert_eq!(size_of::<u129>(), 24);
  assert_eq!(size_of::<i1023>(), 128);
  assert_eq!(align_of::<u144>(), align_of::<u64>());
}
