use core::any::TypeId;

use typenum::{
  U1, U2, U8, U9, U16, U17, U24, U32, U33, U64, U65, U128, U129, U200, U1024, U8192, U16384,
  U32768, U65536, U131072,
};

use super::{LimbArray, Limbs};

/// Asserts that `Limbs<$bits, $limb>` is the array `$array` of `$count` limbs.
macro_rules! assert_limbs {
  ($bits:ty, $limb:ty => $array:ty, $count:ty) => {
    assert_eq!(TypeId::of::<Limbs<$bits, $limb>>(), TypeId::of::<$array>());
    assert_eq!(
      TypeId::of::<<$limb as LimbArray<$bits>>::Count>(),
      TypeId::of::<$count>()
    );
  };
}

/// Each limb type of `b` bits holds 1 to `b` bits in one limb, `b + 1` bits in two, and `1024 * b`
/// bits in 1024, the most that has an array.
#[test]
fn limb_array() {
  assert_limbs!(U1, u8 => [u8; 1], U1);
  assert_limbs!(U8, u8 => [u8; 1], U1);
  assert_limbs!(U9, u8 => [u8; 2], U2);
  assert_limbs!(U8192, u8 => [u8; 1024], U1024);
  assert_limbs!(U1, u16 => [u16; 1], U1);
  assert_limbs!(U16, u16 => [u16; 1], U1);
  assert_limbs!(U17, u16 => [u16; 2], U2);
  assert_limbs!(U16384, u16 => [u16; 1024], U1024);
  assert_limbs!(U1, u32 => [u32; 1], U1);
  assert_limbs!(U32, u32 => [u32; 1], U1);
  assert_limbs!(U33, u32 => [u32; 2], U2);
  assert_limbs!(U32768, u32 => [u32; 1024], U1024);
  assert_limbs!(U1, u64 => [u64; 1], U1);
  assert_limbs!(U64, u64 => [u64; 1], U1);
  assert_limbs!(U65, u64 => [u64; 2], U2);
  assert_limbs!(U65536, u64 => [u64; 1024], U1024);
  assert_limbs!(U1, u128 => [u128; 1], U1);
  assert_limbs!(U128, u128 => [u128; 1], U1);
  assert_limbs!(U129, u128 => [u128; 2], U2);
  assert_limbs!(U131072, u128 => [u128; 1024], U1024);

  // The limb type defaults to `u64`.
  assert_eq!(TypeId::of::<Limbs<U200>>(), TypeId::of::<[u64; 4]>());
  assert_eq!(TypeId::of::<Limbs<U24, u8>>(), TypeId::of::<[u8; 3]>());
}
