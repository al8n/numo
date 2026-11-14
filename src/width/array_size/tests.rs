// Each row of the length table gets a test named after its `typenum` constant.
#![allow(non_snake_case)]

use core::any::TypeId;

use typenum::{U, Unsigned};

use super::{Array, ArrayLength, ArraySize};

macro_rules! test_array_size {
  ($($count:ident = $len:literal),+ $(,)?) => {$(
    #[test]
    #[cfg_attr(miri, ignore = "checks types only, which no target changes")]
    fn $count() {
      type Count = typenum::consts::$count;
      assert_eq!(<Count as Unsigned>::USIZE, $len);
      assert_eq!(
        TypeId::of::<<Count as ArraySize>::ArrayType<u8>>(),
        TypeId::of::<[u8; $len]>()
      );
      // The other direction, which the width relations of the arrays take.
      assert_eq!(TypeId::of::<U<$len>>(), TypeId::of::<Count>());
    }
  )+};
}

array_size_table!(test_array_size);

/// Checks that the default `Array` of `N` bytes holds `N` zeros, and that it reads and writes as a
/// slice of `N` bytes.
#[inline(never)]
fn check_array<N: ArrayLength>() {
  let mut array = Array::<u8, N>::default();
  assert_eq!(array.len(), N::USIZE);
  assert!(array.iter().all(|&byte| byte == 0));
  let slice: &mut [u8] = &mut array;
  assert_eq!(slice.len(), N::USIZE);
  slice.fill(7);
  assert!(array.iter().all(|&byte| byte == 7));
}

#[test]
fn an_array_is_a_slice_of_its_length() {
  check_array::<typenum::consts::U0>();
  check_array::<typenum::consts::U1>();
  check_array::<typenum::consts::U19>();
  check_array::<typenum::consts::U1024>();
}

#[test]
fn array_from_fn_gives_the_indexes_in_order() {
  let mut next = 0;
  let indexes = <typenum::consts::U19 as ArrayLength>::array_from_fn(|index| {
    assert_eq!(index, next);
    next += 1;
    index
  });
  assert_eq!(next, 19);
  assert!(indexes.iter().copied().eq(0..19));
}
