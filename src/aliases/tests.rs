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
  assert_eq!(REJECTED, Err(TryNewError), "-65 needs 8 bits");
  assert_eq!(LARGEST, (1 << 100) - 1);
  assert_eq!(SIGN, -1);
  assert_eq!(QUERIES, [false, true, true]);
  assert_eq!(size_of::<Option<u7>>(), 2);
  assert_eq!(size_of::<i24>(), size_of::<i32>());
  assert_eq!(align_of::<u100>(), align_of::<u128>());
}
