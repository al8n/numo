//! Aliases of [`Uint`] and [`Int`] for common widths.
//!
//! An alias is named after its sign and width, as the primitive integers are: `uN` is `Uint<UN>`
//! and `iN` is `Int<UN>`, an integer of `N` bits in its default storage. `u1` to `u127` and `i1` to
//! `i127` are stored in the smallest native integer that holds them, as the default storage is:
//! `u7` is `Uint<U7, u8>` and `i24` is `Int<U24, i32>`. A native width, 8, 16, 32, 64 or 128 bits,
//! has no alias, which would shadow the primitive type of the same name: such an integer is written
//! `Uint<U8>`, which is `Uint<U8, u8>`. The wide aliases cover 192, 256, 320, 384, 448 and 512
//! bits, the wide storages of [`Decimal`](crate::Decimal), and 768, 1024, 2048 and 4096 bits, each
//! of which fills `N / 64` limbs of 64 bits: `u256` is `Uint<U256>`, which is
//! `Uint<U256, [u64; 4]>`. Any other width is written out, such as `Uint<U200>` for its default
//! storage, and so is any other storage, such as `Uint<U7, u16>`, `Uint<U256, [u8; 32]>` or
//! `Uint<U256, [u128; 2]>`.
//!
//! The crate root re-exports every alias, so `numo::u7` and `numo::aliases::u7` are the same type.

// The aliases follow the names of the primitive integers.
#![allow(non_camel_case_types)]

use typenum::{U, U192, U256, U320, U384, U448, U512, U768, U1024, U2048, U4096};

use crate::{Int, Uint};

/// Passes the aliases of the native storages to `$callback`. Each row is a sign, a native integer
/// and the aliases that it stores, written `name = bits`.
macro_rules! native_alias_table {
  ($callback:ident) => {
    $callback! {
      unsigned u8: u1 = 1, u2 = 2, u3 = 3, u4 = 4, u5 = 5, u6 = 6, u7 = 7;
      unsigned u16: u9 = 9, u10 = 10, u11 = 11, u12 = 12, u13 = 13, u14 = 14, u15 = 15;
      unsigned u32: u17 = 17, u18 = 18, u19 = 19, u20 = 20, u21 = 21, u22 = 22, u23 = 23, u24 = 24,
        u25 = 25, u26 = 26, u27 = 27, u28 = 28, u29 = 29, u30 = 30, u31 = 31;
      unsigned u64: u33 = 33, u34 = 34, u35 = 35, u36 = 36, u37 = 37, u38 = 38, u39 = 39, u40 = 40,
        u41 = 41, u42 = 42, u43 = 43, u44 = 44, u45 = 45, u46 = 46, u47 = 47, u48 = 48, u49 = 49,
        u50 = 50, u51 = 51, u52 = 52, u53 = 53, u54 = 54, u55 = 55, u56 = 56, u57 = 57, u58 = 58,
        u59 = 59, u60 = 60, u61 = 61, u62 = 62, u63 = 63;
      unsigned u128: u65 = 65, u66 = 66, u67 = 67, u68 = 68, u69 = 69, u70 = 70, u71 = 71, u72 = 72,
        u73 = 73, u74 = 74, u75 = 75, u76 = 76, u77 = 77, u78 = 78, u79 = 79, u80 = 80, u81 = 81,
        u82 = 82, u83 = 83, u84 = 84, u85 = 85, u86 = 86, u87 = 87, u88 = 88, u89 = 89, u90 = 90,
        u91 = 91, u92 = 92, u93 = 93, u94 = 94, u95 = 95, u96 = 96, u97 = 97, u98 = 98, u99 = 99,
        u100 = 100, u101 = 101, u102 = 102, u103 = 103, u104 = 104, u105 = 105, u106 = 106,
        u107 = 107, u108 = 108, u109 = 109, u110 = 110, u111 = 111, u112 = 112, u113 = 113,
        u114 = 114, u115 = 115, u116 = 116, u117 = 117, u118 = 118, u119 = 119, u120 = 120,
        u121 = 121, u122 = 122, u123 = 123, u124 = 124, u125 = 125, u126 = 126, u127 = 127;
      signed i8: i1 = 1, i2 = 2, i3 = 3, i4 = 4, i5 = 5, i6 = 6, i7 = 7;
      signed i16: i9 = 9, i10 = 10, i11 = 11, i12 = 12, i13 = 13, i14 = 14, i15 = 15;
      signed i32: i17 = 17, i18 = 18, i19 = 19, i20 = 20, i21 = 21, i22 = 22, i23 = 23, i24 = 24,
        i25 = 25, i26 = 26, i27 = 27, i28 = 28, i29 = 29, i30 = 30, i31 = 31;
      signed i64: i33 = 33, i34 = 34, i35 = 35, i36 = 36, i37 = 37, i38 = 38, i39 = 39, i40 = 40,
        i41 = 41, i42 = 42, i43 = 43, i44 = 44, i45 = 45, i46 = 46, i47 = 47, i48 = 48, i49 = 49,
        i50 = 50, i51 = 51, i52 = 52, i53 = 53, i54 = 54, i55 = 55, i56 = 56, i57 = 57, i58 = 58,
        i59 = 59, i60 = 60, i61 = 61, i62 = 62, i63 = 63;
      signed i128: i65 = 65, i66 = 66, i67 = 67, i68 = 68, i69 = 69, i70 = 70, i71 = 71, i72 = 72,
        i73 = 73, i74 = 74, i75 = 75, i76 = 76, i77 = 77, i78 = 78, i79 = 79, i80 = 80, i81 = 81,
        i82 = 82, i83 = 83, i84 = 84, i85 = 85, i86 = 86, i87 = 87, i88 = 88, i89 = 89, i90 = 90,
        i91 = 91, i92 = 92, i93 = 93, i94 = 94, i95 = 95, i96 = 96, i97 = 97, i98 = 98, i99 = 99,
        i100 = 100, i101 = 101, i102 = 102, i103 = 103, i104 = 104, i105 = 105, i106 = 106,
        i107 = 107, i108 = 108, i109 = 109, i110 = 110, i111 = 111, i112 = 112, i113 = 113,
        i114 = 114, i115 = 115, i116 = 116, i117 = 117, i118 = 118, i119 = 119, i120 = 120,
        i121 = 121, i122 = 122, i123 = 123, i124 = 124, i125 = 125, i126 = 126, i127 = 127;
    }
  };
}

macro_rules! native_aliases {
  ($($sign:ident $repr:ident: $($name:ident = $bits:tt),+;)+) => {$($(
    native_aliases!(@alias $sign $repr $name $bits);
  )+)+};
  (@alias unsigned $repr:ident $name:ident $bits:tt) => {
    #[doc = concat!(
      "An unsigned ",
      stringify!($bits),
      "-bit integer, stored in a `",
      stringify!($repr),
      "`."
    )]
    pub type $name = Uint<U<$bits>, $repr>;
  };
  // One signed bit holds only the sign.
  (@alias signed $repr:ident $name:ident 1) => {
    #[doc = concat!(
      "A signed 1-bit integer, stored in an `",
      stringify!($repr),
      "`: it holds -1 or 0."
    )]
    pub type $name = Int<U<1>, $repr>;
  };
  (@alias signed $repr:ident $name:ident $bits:tt) => {
    #[doc = concat!(
      "A signed ",
      stringify!($bits),
      "-bit integer, stored in an `",
      stringify!($repr),
      "`."
    )]
    pub type $name = Int<U<$bits>, $repr>;
  };
}

native_alias_table!(native_aliases);

/// A 192-bit unsigned integer, stored in three 64-bit limbs.
pub type u192 = Uint<U192, [u64; 3]>;

/// A 256-bit unsigned integer, stored in four 64-bit limbs.
pub type u256 = Uint<U256, [u64; 4]>;

/// A 320-bit unsigned integer, stored in five 64-bit limbs.
pub type u320 = Uint<U320, [u64; 5]>;

/// A 384-bit unsigned integer, stored in six 64-bit limbs.
pub type u384 = Uint<U384, [u64; 6]>;

/// A 448-bit unsigned integer, stored in seven 64-bit limbs.
pub type u448 = Uint<U448, [u64; 7]>;

/// A 512-bit unsigned integer, stored in eight 64-bit limbs.
pub type u512 = Uint<U512, [u64; 8]>;

/// A 768-bit unsigned integer, stored in twelve 64-bit limbs.
pub type u768 = Uint<U768, [u64; 12]>;

/// A 1024-bit unsigned integer, stored in sixteen 64-bit limbs.
pub type u1024 = Uint<U1024, [u64; 16]>;

/// A 2048-bit unsigned integer, stored in 32 64-bit limbs.
pub type u2048 = Uint<U2048, [u64; 32]>;

/// A 4096-bit unsigned integer, stored in 64 64-bit limbs.
pub type u4096 = Uint<U4096, [u64; 64]>;

/// A 192-bit signed integer, stored in three 64-bit limbs in two's complement.
pub type i192 = Int<U192, [u64; 3]>;

/// A 256-bit signed integer, stored in four 64-bit limbs in two's complement.
pub type i256 = Int<U256, [u64; 4]>;

/// A 320-bit signed integer, stored in five 64-bit limbs in two's complement.
pub type i320 = Int<U320, [u64; 5]>;

/// A 384-bit signed integer, stored in six 64-bit limbs in two's complement.
pub type i384 = Int<U384, [u64; 6]>;

/// A 448-bit signed integer, stored in seven 64-bit limbs in two's complement.
pub type i448 = Int<U448, [u64; 7]>;

/// A 512-bit signed integer, stored in eight 64-bit limbs in two's complement.
pub type i512 = Int<U512, [u64; 8]>;

/// A 768-bit signed integer, stored in twelve 64-bit limbs in two's complement.
pub type i768 = Int<U768, [u64; 12]>;

/// A 1024-bit signed integer, stored in sixteen 64-bit limbs in two's complement.
pub type i1024 = Int<U1024, [u64; 16]>;

/// A 2048-bit signed integer, stored in 32 64-bit limbs in two's complement.
pub type i2048 = Int<U2048, [u64; 32]>;

/// A 4096-bit signed integer, stored in 64 64-bit limbs in two's complement.
pub type i4096 = Int<U4096, [u64; 64]>;

#[cfg(test)]
mod tests;
