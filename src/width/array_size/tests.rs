// Each row of the length table gets a test named after its `typenum` constant.
#![allow(non_snake_case)]

use core::any::TypeId;

use typenum::{U, Unsigned};

use super::ArraySize;

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
