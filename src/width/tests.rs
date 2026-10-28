// Each row of the bit table gets a test named after its `typenum` constant.
#![allow(non_snake_case)]

use core::any::TypeId;

use typenum::{
  Add1, Shleft, Sub1, U, U0, U1, U2, U6, U8, U9, U16, U17, U24, U32, U33, U64, U65, U128, U129,
  U192, U193, U200, U256, U320, U384, U448, U512, U768, U1024, U2047, U2048, U3600, U4095, U4096,
  U8191, U8192, U10000, U16383, U16384, U32767, U32768, U65535, U65536, U131072, Unsigned,
};

use super::{BitCount, FitsBits, LimbArray, Limbs};
use crate::{
  Int, Uint, i192, i256, i320, i384, i448, i512, i768, i1024, i2048, i4096, u192, u256, u320, u384,
  u448, u512, u768, u1024, u2048, u4096,
};

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

/// Each limb type of `b` bits holds zero bits in no limbs, 1 to `b` bits in one limb, `b + 1` bits
/// in two, and `1024 * b` bits in 1024, the most that has an array.
#[test]
fn limb_array() {
  assert_limbs!(U0, u8 => [u8; 0], U0);
  assert_limbs!(U1, u8 => [u8; 1], U1);
  assert_limbs!(U8, u8 => [u8; 1], U1);
  assert_limbs!(U9, u8 => [u8; 2], U2);
  assert_limbs!(U8192, u8 => [u8; 1024], U1024);
  assert_limbs!(U0, u16 => [u16; 0], U0);
  assert_limbs!(U1, u16 => [u16; 1], U1);
  assert_limbs!(U16, u16 => [u16; 1], U1);
  assert_limbs!(U17, u16 => [u16; 2], U2);
  assert_limbs!(U16384, u16 => [u16; 1024], U1024);
  assert_limbs!(U0, u32 => [u32; 0], U0);
  assert_limbs!(U1, u32 => [u32; 1], U1);
  assert_limbs!(U32, u32 => [u32; 1], U1);
  assert_limbs!(U33, u32 => [u32; 2], U2);
  assert_limbs!(U32768, u32 => [u32; 1024], U1024);
  assert_limbs!(U0, u64 => [u64; 0], U0);
  assert_limbs!(U1, u64 => [u64; 1], U1);
  assert_limbs!(U64, u64 => [u64; 1], U1);
  assert_limbs!(U65, u64 => [u64; 2], U2);
  assert_limbs!(U65536, u64 => [u64; 1024], U1024);
  assert_limbs!(U0, u128 => [u128; 0], U0);
  assert_limbs!(U1, u128 => [u128; 1], U1);
  assert_limbs!(U128, u128 => [u128; 1], U1);
  assert_limbs!(U129, u128 => [u128; 2], U2);
  assert_limbs!(U131072, u128 => [u128; 1024], U1024);

  // The limb type defaults to `u64`.
  assert_eq!(TypeId::of::<Limbs<U200>>(), TypeId::of::<[u64; 4]>());
  assert_eq!(TypeId::of::<Limbs<U24, u8>>(), TypeId::of::<[u8; 3]>());
}

fn assert_fits<B: FitsBits<R>, R>() {}

/// Checks a row of the bit table: the constant has the value, the default storages are the natives
/// of the row and hold the width, and no smaller native holds it.
macro_rules! test_bits {
  ($($bits:ident = $value:literal => $uint:ident, $int:ident),+ $(,)?) => {$(
    #[test]
    fn $bits() {
      type Bits = typenum::consts::$bits;
      assert_eq!(<Bits as Unsigned>::U32, $value);
      assert_eq!(TypeId::of::<<Bits as BitCount>::Unsigned>(), TypeId::of::<$uint>());
      assert_eq!(TypeId::of::<<Bits as BitCount>::Signed>(), TypeId::of::<$int>());
      assert_fits::<Bits, $uint>();
      assert_fits::<Bits, $int>();
      // The native of half the bits, if any, would not hold the width.
      let (bits, native) = ($value, $uint::BITS);
      assert!(bits <= native && (native == 8 || bits > native / 2));
    }
  )+};
}

mod bit_count {
  use super::*;

  bit_table!(test_bits);

  /// Zero bits default to `[u64; 0]`, which holds them in no space, rather than to a native
  /// integer.
  #[test]
  fn U0() {
    type Bits = typenum::consts::U0;
    assert_eq!(<Bits as Unsigned>::U32, 0);
    assert_eq!(
      TypeId::of::<<Bits as BitCount>::Unsigned>(),
      TypeId::of::<[u64; 0]>()
    );
    assert_eq!(
      TypeId::of::<<Bits as BitCount>::Signed>(),
      TypeId::of::<[u64; 0]>()
    );
    assert_fits::<Bits, [u64; 0]>();
  }
}

/// Asserts that the default storages of `Bits` bits are both `[u64; N]`, the fewest 64-bit limbs
/// that hold the width.
fn assert_default_array<Bits: BitCount + 'static, const N: usize>() {
  assert_eq!(TypeId::of::<Bits::Unsigned>(), TypeId::of::<[u64; N]>());
  assert_eq!(TypeId::of::<Bits::Signed>(), TypeId::of::<[u64; N]>());
  let (bits, limbs) = (Bits::U32, N as u32);
  assert!(64 * (limbs - 1) < bits && bits <= 64 * limbs, "{bits} bits");
}

/// The default above 128 bits at both ends of a sample of limb counts, `64 * N - 63` and `64 * N`
/// bits, and at the widths that `typenum` names above 128 bits.
#[test]
fn wide_default() {
  macro_rules! assert_limb_counts {
    ($($n:literal),+) => {$(
      assert_default_array::<Add1<Shleft<Sub1<U<$n>>, U6>>, $n>();
      assert_default_array::<Shleft<U<$n>, U6>, $n>();
    )+};
  }
  assert_limb_counts!(
    3, 4, 5, 8, 16, 17, 31, 32, 33, 63, 64, 65, 100, 127, 128, 129, 255, 256, 257, 511, 512, 513,
    1000, 1023, 1024
  );

  assert_default_array::<U129, 3>();
  assert_default_array::<U192, 3>();
  assert_default_array::<U193, 4>();
  assert_default_array::<U200, 4>();
  assert_default_array::<U256, 4>();
  assert_default_array::<U1024, 16>();
  assert_default_array::<Add1<U1024>, 17>();
  assert_default_array::<U2047, 32>();
  assert_default_array::<U2048, 32>();
  assert_default_array::<U3600, 57>();
  assert_default_array::<U4095, 64>();
  assert_default_array::<U4096, 64>();
  assert_default_array::<U8191, 128>();
  assert_default_array::<U8192, 128>();
  assert_default_array::<U10000, 157>();
  assert_default_array::<U16383, 256>();
  assert_default_array::<U16384, 256>();
  assert_default_array::<U32767, 512>();
  assert_default_array::<U32768, 512>();
  assert_default_array::<U65535, 1024>();
  assert_default_array::<U65536, 1024>();
}

/// Each wide alias is the type of its width with the default storage.
#[test]
fn aliases_are_defaults() {
  macro_rules! assert_defaults {
    ($($alias:ident = $integer:ident<$bits:ident>),+ $(,)?) => {$(
      assert_eq!(TypeId::of::<$alias>(), TypeId::of::<$integer<$bits>>());
    )+};
  }
  assert_defaults!(
    u192 = Uint<U192>,
    u256 = Uint<U256>,
    u320 = Uint<U320>,
    u384 = Uint<U384>,
    u448 = Uint<U448>,
    u512 = Uint<U512>,
    u768 = Uint<U768>,
    u1024 = Uint<U1024>,
    u2048 = Uint<U2048>,
    u4096 = Uint<U4096>,
    i192 = Int<U192>,
    i256 = Int<U256>,
    i320 = Int<U320>,
    i384 = Int<U384>,
    i448 = Int<U448>,
    i512 = Int<U512>,
    i768 = Int<U768>,
    i1024 = Int<U1024>,
    i2048 = Int<U2048>,
    i4096 = Int<U4096>,
  );
}
