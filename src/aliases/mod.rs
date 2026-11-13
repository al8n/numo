//! Aliases of [`Uint`], [`Int`], [`NonZeroUint`] and [`NonZeroInt`] for common widths.
//!
//! An alias is named after its sign and width, as the primitive integers are: `uN` is `Uint<UN>`
//! and `iN` is `Int<UN>`, an integer of `N` bits in its default storage. `u0` to `u1024` and `i0`
//! to `i1024` have an alias, but for the native widths 8, 16, 32, 64 and 128 bits, whose aliases
//! would shadow the primitive types of the same name: such an integer is written `Uint<U8>`, which
//! is `Uint<U8, u8>`. `u0` and `i0` hold only 0, in `[u64; 0]`, the default storage of zero bits,
//! so they are zero-sized. An integer of 1 to 127 bits is stored in the smallest native integer
//! that holds it, as the default storage is: `u7` is `Uint<U7, u8>` and `i24` is `Int<U24, i32>`.
//! An integer of 129 to 1024 bits is stored in `[u64; ceil(N / 64)]`, the limbs that the default
//! storage takes: `u144` is `Uint<U144, [u64; 3]>` and `i256` is `Int<U256, [u64; 4]>`. The widths
//! of the wide storages of [`Decimal`](crate::Decimal), 192 to 1024 bits, every 64 bits, are among
//! them. Beyond 1024 bits only `u2048`, `u4096`, `i2048` and `i4096` have an alias, each of which
//! fills `N / 64` limbs of 64 bits. Any other width is written out, such as `Uint<U8192>` for its
//! default storage, and so is any other storage, such as `Uint<U7, u16>`, `Uint<U256, [u8; 32]>` or
//! `Uint<U256, [u128; 2]>`. A width that `typenum` has no constant for is computed, as in
//! `Uint<Sum<U1024, U476>>` for 1500 bits.
//!
//! The non-zero integers have aliases too: `NonZeroUN` is the non-zero integer of `uN` and
//! `NonZeroIN` that of `iN`, so `NonZeroUN` is exactly [`NonZeroOf<uN>`](crate::NonZeroOf), in the
//! non-zero form of the storage of `uN`: `NonZeroU7` is `NonZeroUint<U7, core::num::NonZero<u8>>`
//! and `NonZeroI200` is `NonZeroInt<U200, [u64; 4]>`. They exist for the widths that have an
//! integer alias, except zero bits, which hold no value that is not zero, and the native widths 8,
//! 16, 32, 64 and 128, whose names `core::num` already uses for its own types: its `NonZeroU8` is
//! `NonZero<u8>`, a different type from `NonZeroUint<U8, NonZero<u8>>`.
//!
//! The crate root re-exports every alias, so `numo::u7` and `numo::aliases::u7` are the same type,
//! and so are `numo::NonZeroU7` and `numo::aliases::NonZeroU7`.

// The aliases follow the names of the primitive integers.
#![allow(non_camel_case_types)]

use typenum::{U, U0, U2048, U4096};

use crate::{Int, NonZeroInt, NonZeroUint, Uint};

/// Passes the aliases of the native storages to `$callback`. Each row is a sign, a native integer
/// and the aliases that it stores, written `name nonzero_name = bits`: the integer alias and the
/// alias of its non-zero integer.
macro_rules! native_alias_table {
  ($callback:ident) => {
    $callback! {
      unsigned u8: u1 NonZeroU1 = 1, u2 NonZeroU2 = 2, u3 NonZeroU3 = 3, u4 NonZeroU4 = 4,
        u5 NonZeroU5 = 5, u6 NonZeroU6 = 6, u7 NonZeroU7 = 7;
      unsigned u16: u9 NonZeroU9 = 9, u10 NonZeroU10 = 10, u11 NonZeroU11 = 11, u12 NonZeroU12 = 12,
        u13 NonZeroU13 = 13, u14 NonZeroU14 = 14, u15 NonZeroU15 = 15;
      unsigned u32: u17 NonZeroU17 = 17, u18 NonZeroU18 = 18, u19 NonZeroU19 = 19,
        u20 NonZeroU20 = 20, u21 NonZeroU21 = 21, u22 NonZeroU22 = 22, u23 NonZeroU23 = 23,
        u24 NonZeroU24 = 24, u25 NonZeroU25 = 25, u26 NonZeroU26 = 26, u27 NonZeroU27 = 27,
        u28 NonZeroU28 = 28, u29 NonZeroU29 = 29, u30 NonZeroU30 = 30, u31 NonZeroU31 = 31;
      unsigned u64: u33 NonZeroU33 = 33, u34 NonZeroU34 = 34, u35 NonZeroU35 = 35,
        u36 NonZeroU36 = 36, u37 NonZeroU37 = 37, u38 NonZeroU38 = 38, u39 NonZeroU39 = 39,
        u40 NonZeroU40 = 40, u41 NonZeroU41 = 41, u42 NonZeroU42 = 42, u43 NonZeroU43 = 43,
        u44 NonZeroU44 = 44, u45 NonZeroU45 = 45, u46 NonZeroU46 = 46, u47 NonZeroU47 = 47,
        u48 NonZeroU48 = 48, u49 NonZeroU49 = 49, u50 NonZeroU50 = 50, u51 NonZeroU51 = 51,
        u52 NonZeroU52 = 52, u53 NonZeroU53 = 53, u54 NonZeroU54 = 54, u55 NonZeroU55 = 55,
        u56 NonZeroU56 = 56, u57 NonZeroU57 = 57, u58 NonZeroU58 = 58, u59 NonZeroU59 = 59,
        u60 NonZeroU60 = 60, u61 NonZeroU61 = 61, u62 NonZeroU62 = 62, u63 NonZeroU63 = 63;
      unsigned u128: u65 NonZeroU65 = 65, u66 NonZeroU66 = 66, u67 NonZeroU67 = 67,
        u68 NonZeroU68 = 68, u69 NonZeroU69 = 69, u70 NonZeroU70 = 70, u71 NonZeroU71 = 71,
        u72 NonZeroU72 = 72, u73 NonZeroU73 = 73, u74 NonZeroU74 = 74, u75 NonZeroU75 = 75,
        u76 NonZeroU76 = 76, u77 NonZeroU77 = 77, u78 NonZeroU78 = 78, u79 NonZeroU79 = 79,
        u80 NonZeroU80 = 80, u81 NonZeroU81 = 81, u82 NonZeroU82 = 82, u83 NonZeroU83 = 83,
        u84 NonZeroU84 = 84, u85 NonZeroU85 = 85, u86 NonZeroU86 = 86, u87 NonZeroU87 = 87,
        u88 NonZeroU88 = 88, u89 NonZeroU89 = 89, u90 NonZeroU90 = 90, u91 NonZeroU91 = 91,
        u92 NonZeroU92 = 92, u93 NonZeroU93 = 93, u94 NonZeroU94 = 94, u95 NonZeroU95 = 95,
        u96 NonZeroU96 = 96, u97 NonZeroU97 = 97, u98 NonZeroU98 = 98, u99 NonZeroU99 = 99,
        u100 NonZeroU100 = 100, u101 NonZeroU101 = 101, u102 NonZeroU102 = 102,
        u103 NonZeroU103 = 103, u104 NonZeroU104 = 104, u105 NonZeroU105 = 105,
        u106 NonZeroU106 = 106, u107 NonZeroU107 = 107, u108 NonZeroU108 = 108,
        u109 NonZeroU109 = 109, u110 NonZeroU110 = 110, u111 NonZeroU111 = 111,
        u112 NonZeroU112 = 112, u113 NonZeroU113 = 113, u114 NonZeroU114 = 114,
        u115 NonZeroU115 = 115, u116 NonZeroU116 = 116, u117 NonZeroU117 = 117,
        u118 NonZeroU118 = 118, u119 NonZeroU119 = 119, u120 NonZeroU120 = 120,
        u121 NonZeroU121 = 121, u122 NonZeroU122 = 122, u123 NonZeroU123 = 123,
        u124 NonZeroU124 = 124, u125 NonZeroU125 = 125, u126 NonZeroU126 = 126,
        u127 NonZeroU127 = 127;
      signed i8: i1 NonZeroI1 = 1, i2 NonZeroI2 = 2, i3 NonZeroI3 = 3, i4 NonZeroI4 = 4,
        i5 NonZeroI5 = 5, i6 NonZeroI6 = 6, i7 NonZeroI7 = 7;
      signed i16: i9 NonZeroI9 = 9, i10 NonZeroI10 = 10, i11 NonZeroI11 = 11, i12 NonZeroI12 = 12,
        i13 NonZeroI13 = 13, i14 NonZeroI14 = 14, i15 NonZeroI15 = 15;
      signed i32: i17 NonZeroI17 = 17, i18 NonZeroI18 = 18, i19 NonZeroI19 = 19,
        i20 NonZeroI20 = 20, i21 NonZeroI21 = 21, i22 NonZeroI22 = 22, i23 NonZeroI23 = 23,
        i24 NonZeroI24 = 24, i25 NonZeroI25 = 25, i26 NonZeroI26 = 26, i27 NonZeroI27 = 27,
        i28 NonZeroI28 = 28, i29 NonZeroI29 = 29, i30 NonZeroI30 = 30, i31 NonZeroI31 = 31;
      signed i64: i33 NonZeroI33 = 33, i34 NonZeroI34 = 34, i35 NonZeroI35 = 35,
        i36 NonZeroI36 = 36, i37 NonZeroI37 = 37, i38 NonZeroI38 = 38, i39 NonZeroI39 = 39,
        i40 NonZeroI40 = 40, i41 NonZeroI41 = 41, i42 NonZeroI42 = 42, i43 NonZeroI43 = 43,
        i44 NonZeroI44 = 44, i45 NonZeroI45 = 45, i46 NonZeroI46 = 46, i47 NonZeroI47 = 47,
        i48 NonZeroI48 = 48, i49 NonZeroI49 = 49, i50 NonZeroI50 = 50, i51 NonZeroI51 = 51,
        i52 NonZeroI52 = 52, i53 NonZeroI53 = 53, i54 NonZeroI54 = 54, i55 NonZeroI55 = 55,
        i56 NonZeroI56 = 56, i57 NonZeroI57 = 57, i58 NonZeroI58 = 58, i59 NonZeroI59 = 59,
        i60 NonZeroI60 = 60, i61 NonZeroI61 = 61, i62 NonZeroI62 = 62, i63 NonZeroI63 = 63;
      signed i128: i65 NonZeroI65 = 65, i66 NonZeroI66 = 66, i67 NonZeroI67 = 67,
        i68 NonZeroI68 = 68, i69 NonZeroI69 = 69, i70 NonZeroI70 = 70, i71 NonZeroI71 = 71,
        i72 NonZeroI72 = 72, i73 NonZeroI73 = 73, i74 NonZeroI74 = 74, i75 NonZeroI75 = 75,
        i76 NonZeroI76 = 76, i77 NonZeroI77 = 77, i78 NonZeroI78 = 78, i79 NonZeroI79 = 79,
        i80 NonZeroI80 = 80, i81 NonZeroI81 = 81, i82 NonZeroI82 = 82, i83 NonZeroI83 = 83,
        i84 NonZeroI84 = 84, i85 NonZeroI85 = 85, i86 NonZeroI86 = 86, i87 NonZeroI87 = 87,
        i88 NonZeroI88 = 88, i89 NonZeroI89 = 89, i90 NonZeroI90 = 90, i91 NonZeroI91 = 91,
        i92 NonZeroI92 = 92, i93 NonZeroI93 = 93, i94 NonZeroI94 = 94, i95 NonZeroI95 = 95,
        i96 NonZeroI96 = 96, i97 NonZeroI97 = 97, i98 NonZeroI98 = 98, i99 NonZeroI99 = 99,
        i100 NonZeroI100 = 100, i101 NonZeroI101 = 101, i102 NonZeroI102 = 102,
        i103 NonZeroI103 = 103, i104 NonZeroI104 = 104, i105 NonZeroI105 = 105,
        i106 NonZeroI106 = 106, i107 NonZeroI107 = 107, i108 NonZeroI108 = 108,
        i109 NonZeroI109 = 109, i110 NonZeroI110 = 110, i111 NonZeroI111 = 111,
        i112 NonZeroI112 = 112, i113 NonZeroI113 = 113, i114 NonZeroI114 = 114,
        i115 NonZeroI115 = 115, i116 NonZeroI116 = 116, i117 NonZeroI117 = 117,
        i118 NonZeroI118 = 118, i119 NonZeroI119 = 119, i120 NonZeroI120 = 120,
        i121 NonZeroI121 = 121, i122 NonZeroI122 = 122, i123 NonZeroI123 = 123,
        i124 NonZeroI124 = 124, i125 NonZeroI125 = 125, i126 NonZeroI126 = 126,
        i127 NonZeroI127 = 127;
    }
  };
}

macro_rules! native_aliases {
  ($($sign:ident $repr:ident: $($name:ident $nonzero:ident = $bits:tt),+;)+) => {$($(
    native_aliases!(@alias $sign $repr $name $nonzero $bits);
  )+)+};
  (@alias unsigned $repr:ident $name:ident $nonzero:ident $bits:tt) => {
    #[doc = concat!(
      "An unsigned ",
      stringify!($bits),
      "-bit integer, stored in a `",
      stringify!($repr),
      "`."
    )]
    pub type $name = Uint<U<$bits>, $repr>;

    #[doc = concat!(
      "A non-zero unsigned ",
      stringify!($bits),
      "-bit integer, stored in a `core::num::NonZero<",
      stringify!($repr),
      ">`."
    )]
    pub type $nonzero = NonZeroUint<U<$bits>, core::num::NonZero<$repr>>;
  };
  // One signed bit holds only the sign.
  (@alias signed $repr:ident $name:ident $nonzero:ident 1) => {
    #[doc = concat!(
      "A signed 1-bit integer, stored in an `",
      stringify!($repr),
      "`: it holds -1 or 0."
    )]
    pub type $name = Int<U<1>, $repr>;

    #[doc = concat!(
      "A non-zero signed 1-bit integer, stored in a `core::num::NonZero<",
      stringify!($repr),
      ">`: it holds only -1."
    )]
    pub type $nonzero = NonZeroInt<U<1>, core::num::NonZero<$repr>>;
  };
  (@alias signed $repr:ident $name:ident $nonzero:ident $bits:tt) => {
    #[doc = concat!(
      "A signed ",
      stringify!($bits),
      "-bit integer, stored in an `",
      stringify!($repr),
      "`."
    )]
    pub type $name = Int<U<$bits>, $repr>;

    #[doc = concat!(
      "A non-zero signed ",
      stringify!($bits),
      "-bit integer, stored in a `core::num::NonZero<",
      stringify!($repr),
      ">`."
    )]
    pub type $nonzero = NonZeroInt<U<$bits>, core::num::NonZero<$repr>>;
  };
}

native_alias_table!(native_aliases);

/// Passes the aliases of the wide storages to `$callback`. Each row is an unsigned alias, a signed
/// alias, the aliases of their non-zero integers, the `typenum` constant of their width, the width
/// and the number of 64-bit limbs that holds it, written
/// `unsigned signed nonzero_unsigned nonzero_signed: Constant, bits, limbs;`, for every width from
/// 129 to 1024 bits.
// The rows name the constants, as `array_size_table!` does: `U192` is the number itself, where
// `U<192>` is an associated type that the compiler must normalize, and a diagnostic that names a
// wide alias is worded differently for the projection.
macro_rules! wide_alias_table {
  ($callback:ident) => {
    $callback! {
      u129 i129 NonZeroU129 NonZeroI129: U129, 129, 3;
      u130 i130 NonZeroU130 NonZeroI130: U130, 130, 3;
      u131 i131 NonZeroU131 NonZeroI131: U131, 131, 3;
      u132 i132 NonZeroU132 NonZeroI132: U132, 132, 3;
      u133 i133 NonZeroU133 NonZeroI133: U133, 133, 3;
      u134 i134 NonZeroU134 NonZeroI134: U134, 134, 3;
      u135 i135 NonZeroU135 NonZeroI135: U135, 135, 3;
      u136 i136 NonZeroU136 NonZeroI136: U136, 136, 3;
      u137 i137 NonZeroU137 NonZeroI137: U137, 137, 3;
      u138 i138 NonZeroU138 NonZeroI138: U138, 138, 3;
      u139 i139 NonZeroU139 NonZeroI139: U139, 139, 3;
      u140 i140 NonZeroU140 NonZeroI140: U140, 140, 3;
      u141 i141 NonZeroU141 NonZeroI141: U141, 141, 3;
      u142 i142 NonZeroU142 NonZeroI142: U142, 142, 3;
      u143 i143 NonZeroU143 NonZeroI143: U143, 143, 3;
      u144 i144 NonZeroU144 NonZeroI144: U144, 144, 3;
      u145 i145 NonZeroU145 NonZeroI145: U145, 145, 3;
      u146 i146 NonZeroU146 NonZeroI146: U146, 146, 3;
      u147 i147 NonZeroU147 NonZeroI147: U147, 147, 3;
      u148 i148 NonZeroU148 NonZeroI148: U148, 148, 3;
      u149 i149 NonZeroU149 NonZeroI149: U149, 149, 3;
      u150 i150 NonZeroU150 NonZeroI150: U150, 150, 3;
      u151 i151 NonZeroU151 NonZeroI151: U151, 151, 3;
      u152 i152 NonZeroU152 NonZeroI152: U152, 152, 3;
      u153 i153 NonZeroU153 NonZeroI153: U153, 153, 3;
      u154 i154 NonZeroU154 NonZeroI154: U154, 154, 3;
      u155 i155 NonZeroU155 NonZeroI155: U155, 155, 3;
      u156 i156 NonZeroU156 NonZeroI156: U156, 156, 3;
      u157 i157 NonZeroU157 NonZeroI157: U157, 157, 3;
      u158 i158 NonZeroU158 NonZeroI158: U158, 158, 3;
      u159 i159 NonZeroU159 NonZeroI159: U159, 159, 3;
      u160 i160 NonZeroU160 NonZeroI160: U160, 160, 3;
      u161 i161 NonZeroU161 NonZeroI161: U161, 161, 3;
      u162 i162 NonZeroU162 NonZeroI162: U162, 162, 3;
      u163 i163 NonZeroU163 NonZeroI163: U163, 163, 3;
      u164 i164 NonZeroU164 NonZeroI164: U164, 164, 3;
      u165 i165 NonZeroU165 NonZeroI165: U165, 165, 3;
      u166 i166 NonZeroU166 NonZeroI166: U166, 166, 3;
      u167 i167 NonZeroU167 NonZeroI167: U167, 167, 3;
      u168 i168 NonZeroU168 NonZeroI168: U168, 168, 3;
      u169 i169 NonZeroU169 NonZeroI169: U169, 169, 3;
      u170 i170 NonZeroU170 NonZeroI170: U170, 170, 3;
      u171 i171 NonZeroU171 NonZeroI171: U171, 171, 3;
      u172 i172 NonZeroU172 NonZeroI172: U172, 172, 3;
      u173 i173 NonZeroU173 NonZeroI173: U173, 173, 3;
      u174 i174 NonZeroU174 NonZeroI174: U174, 174, 3;
      u175 i175 NonZeroU175 NonZeroI175: U175, 175, 3;
      u176 i176 NonZeroU176 NonZeroI176: U176, 176, 3;
      u177 i177 NonZeroU177 NonZeroI177: U177, 177, 3;
      u178 i178 NonZeroU178 NonZeroI178: U178, 178, 3;
      u179 i179 NonZeroU179 NonZeroI179: U179, 179, 3;
      u180 i180 NonZeroU180 NonZeroI180: U180, 180, 3;
      u181 i181 NonZeroU181 NonZeroI181: U181, 181, 3;
      u182 i182 NonZeroU182 NonZeroI182: U182, 182, 3;
      u183 i183 NonZeroU183 NonZeroI183: U183, 183, 3;
      u184 i184 NonZeroU184 NonZeroI184: U184, 184, 3;
      u185 i185 NonZeroU185 NonZeroI185: U185, 185, 3;
      u186 i186 NonZeroU186 NonZeroI186: U186, 186, 3;
      u187 i187 NonZeroU187 NonZeroI187: U187, 187, 3;
      u188 i188 NonZeroU188 NonZeroI188: U188, 188, 3;
      u189 i189 NonZeroU189 NonZeroI189: U189, 189, 3;
      u190 i190 NonZeroU190 NonZeroI190: U190, 190, 3;
      u191 i191 NonZeroU191 NonZeroI191: U191, 191, 3;
      u192 i192 NonZeroU192 NonZeroI192: U192, 192, 3;
      u193 i193 NonZeroU193 NonZeroI193: U193, 193, 4;
      u194 i194 NonZeroU194 NonZeroI194: U194, 194, 4;
      u195 i195 NonZeroU195 NonZeroI195: U195, 195, 4;
      u196 i196 NonZeroU196 NonZeroI196: U196, 196, 4;
      u197 i197 NonZeroU197 NonZeroI197: U197, 197, 4;
      u198 i198 NonZeroU198 NonZeroI198: U198, 198, 4;
      u199 i199 NonZeroU199 NonZeroI199: U199, 199, 4;
      u200 i200 NonZeroU200 NonZeroI200: U200, 200, 4;
      u201 i201 NonZeroU201 NonZeroI201: U201, 201, 4;
      u202 i202 NonZeroU202 NonZeroI202: U202, 202, 4;
      u203 i203 NonZeroU203 NonZeroI203: U203, 203, 4;
      u204 i204 NonZeroU204 NonZeroI204: U204, 204, 4;
      u205 i205 NonZeroU205 NonZeroI205: U205, 205, 4;
      u206 i206 NonZeroU206 NonZeroI206: U206, 206, 4;
      u207 i207 NonZeroU207 NonZeroI207: U207, 207, 4;
      u208 i208 NonZeroU208 NonZeroI208: U208, 208, 4;
      u209 i209 NonZeroU209 NonZeroI209: U209, 209, 4;
      u210 i210 NonZeroU210 NonZeroI210: U210, 210, 4;
      u211 i211 NonZeroU211 NonZeroI211: U211, 211, 4;
      u212 i212 NonZeroU212 NonZeroI212: U212, 212, 4;
      u213 i213 NonZeroU213 NonZeroI213: U213, 213, 4;
      u214 i214 NonZeroU214 NonZeroI214: U214, 214, 4;
      u215 i215 NonZeroU215 NonZeroI215: U215, 215, 4;
      u216 i216 NonZeroU216 NonZeroI216: U216, 216, 4;
      u217 i217 NonZeroU217 NonZeroI217: U217, 217, 4;
      u218 i218 NonZeroU218 NonZeroI218: U218, 218, 4;
      u219 i219 NonZeroU219 NonZeroI219: U219, 219, 4;
      u220 i220 NonZeroU220 NonZeroI220: U220, 220, 4;
      u221 i221 NonZeroU221 NonZeroI221: U221, 221, 4;
      u222 i222 NonZeroU222 NonZeroI222: U222, 222, 4;
      u223 i223 NonZeroU223 NonZeroI223: U223, 223, 4;
      u224 i224 NonZeroU224 NonZeroI224: U224, 224, 4;
      u225 i225 NonZeroU225 NonZeroI225: U225, 225, 4;
      u226 i226 NonZeroU226 NonZeroI226: U226, 226, 4;
      u227 i227 NonZeroU227 NonZeroI227: U227, 227, 4;
      u228 i228 NonZeroU228 NonZeroI228: U228, 228, 4;
      u229 i229 NonZeroU229 NonZeroI229: U229, 229, 4;
      u230 i230 NonZeroU230 NonZeroI230: U230, 230, 4;
      u231 i231 NonZeroU231 NonZeroI231: U231, 231, 4;
      u232 i232 NonZeroU232 NonZeroI232: U232, 232, 4;
      u233 i233 NonZeroU233 NonZeroI233: U233, 233, 4;
      u234 i234 NonZeroU234 NonZeroI234: U234, 234, 4;
      u235 i235 NonZeroU235 NonZeroI235: U235, 235, 4;
      u236 i236 NonZeroU236 NonZeroI236: U236, 236, 4;
      u237 i237 NonZeroU237 NonZeroI237: U237, 237, 4;
      u238 i238 NonZeroU238 NonZeroI238: U238, 238, 4;
      u239 i239 NonZeroU239 NonZeroI239: U239, 239, 4;
      u240 i240 NonZeroU240 NonZeroI240: U240, 240, 4;
      u241 i241 NonZeroU241 NonZeroI241: U241, 241, 4;
      u242 i242 NonZeroU242 NonZeroI242: U242, 242, 4;
      u243 i243 NonZeroU243 NonZeroI243: U243, 243, 4;
      u244 i244 NonZeroU244 NonZeroI244: U244, 244, 4;
      u245 i245 NonZeroU245 NonZeroI245: U245, 245, 4;
      u246 i246 NonZeroU246 NonZeroI246: U246, 246, 4;
      u247 i247 NonZeroU247 NonZeroI247: U247, 247, 4;
      u248 i248 NonZeroU248 NonZeroI248: U248, 248, 4;
      u249 i249 NonZeroU249 NonZeroI249: U249, 249, 4;
      u250 i250 NonZeroU250 NonZeroI250: U250, 250, 4;
      u251 i251 NonZeroU251 NonZeroI251: U251, 251, 4;
      u252 i252 NonZeroU252 NonZeroI252: U252, 252, 4;
      u253 i253 NonZeroU253 NonZeroI253: U253, 253, 4;
      u254 i254 NonZeroU254 NonZeroI254: U254, 254, 4;
      u255 i255 NonZeroU255 NonZeroI255: U255, 255, 4;
      u256 i256 NonZeroU256 NonZeroI256: U256, 256, 4;
      u257 i257 NonZeroU257 NonZeroI257: U257, 257, 5;
      u258 i258 NonZeroU258 NonZeroI258: U258, 258, 5;
      u259 i259 NonZeroU259 NonZeroI259: U259, 259, 5;
      u260 i260 NonZeroU260 NonZeroI260: U260, 260, 5;
      u261 i261 NonZeroU261 NonZeroI261: U261, 261, 5;
      u262 i262 NonZeroU262 NonZeroI262: U262, 262, 5;
      u263 i263 NonZeroU263 NonZeroI263: U263, 263, 5;
      u264 i264 NonZeroU264 NonZeroI264: U264, 264, 5;
      u265 i265 NonZeroU265 NonZeroI265: U265, 265, 5;
      u266 i266 NonZeroU266 NonZeroI266: U266, 266, 5;
      u267 i267 NonZeroU267 NonZeroI267: U267, 267, 5;
      u268 i268 NonZeroU268 NonZeroI268: U268, 268, 5;
      u269 i269 NonZeroU269 NonZeroI269: U269, 269, 5;
      u270 i270 NonZeroU270 NonZeroI270: U270, 270, 5;
      u271 i271 NonZeroU271 NonZeroI271: U271, 271, 5;
      u272 i272 NonZeroU272 NonZeroI272: U272, 272, 5;
      u273 i273 NonZeroU273 NonZeroI273: U273, 273, 5;
      u274 i274 NonZeroU274 NonZeroI274: U274, 274, 5;
      u275 i275 NonZeroU275 NonZeroI275: U275, 275, 5;
      u276 i276 NonZeroU276 NonZeroI276: U276, 276, 5;
      u277 i277 NonZeroU277 NonZeroI277: U277, 277, 5;
      u278 i278 NonZeroU278 NonZeroI278: U278, 278, 5;
      u279 i279 NonZeroU279 NonZeroI279: U279, 279, 5;
      u280 i280 NonZeroU280 NonZeroI280: U280, 280, 5;
      u281 i281 NonZeroU281 NonZeroI281: U281, 281, 5;
      u282 i282 NonZeroU282 NonZeroI282: U282, 282, 5;
      u283 i283 NonZeroU283 NonZeroI283: U283, 283, 5;
      u284 i284 NonZeroU284 NonZeroI284: U284, 284, 5;
      u285 i285 NonZeroU285 NonZeroI285: U285, 285, 5;
      u286 i286 NonZeroU286 NonZeroI286: U286, 286, 5;
      u287 i287 NonZeroU287 NonZeroI287: U287, 287, 5;
      u288 i288 NonZeroU288 NonZeroI288: U288, 288, 5;
      u289 i289 NonZeroU289 NonZeroI289: U289, 289, 5;
      u290 i290 NonZeroU290 NonZeroI290: U290, 290, 5;
      u291 i291 NonZeroU291 NonZeroI291: U291, 291, 5;
      u292 i292 NonZeroU292 NonZeroI292: U292, 292, 5;
      u293 i293 NonZeroU293 NonZeroI293: U293, 293, 5;
      u294 i294 NonZeroU294 NonZeroI294: U294, 294, 5;
      u295 i295 NonZeroU295 NonZeroI295: U295, 295, 5;
      u296 i296 NonZeroU296 NonZeroI296: U296, 296, 5;
      u297 i297 NonZeroU297 NonZeroI297: U297, 297, 5;
      u298 i298 NonZeroU298 NonZeroI298: U298, 298, 5;
      u299 i299 NonZeroU299 NonZeroI299: U299, 299, 5;
      u300 i300 NonZeroU300 NonZeroI300: U300, 300, 5;
      u301 i301 NonZeroU301 NonZeroI301: U301, 301, 5;
      u302 i302 NonZeroU302 NonZeroI302: U302, 302, 5;
      u303 i303 NonZeroU303 NonZeroI303: U303, 303, 5;
      u304 i304 NonZeroU304 NonZeroI304: U304, 304, 5;
      u305 i305 NonZeroU305 NonZeroI305: U305, 305, 5;
      u306 i306 NonZeroU306 NonZeroI306: U306, 306, 5;
      u307 i307 NonZeroU307 NonZeroI307: U307, 307, 5;
      u308 i308 NonZeroU308 NonZeroI308: U308, 308, 5;
      u309 i309 NonZeroU309 NonZeroI309: U309, 309, 5;
      u310 i310 NonZeroU310 NonZeroI310: U310, 310, 5;
      u311 i311 NonZeroU311 NonZeroI311: U311, 311, 5;
      u312 i312 NonZeroU312 NonZeroI312: U312, 312, 5;
      u313 i313 NonZeroU313 NonZeroI313: U313, 313, 5;
      u314 i314 NonZeroU314 NonZeroI314: U314, 314, 5;
      u315 i315 NonZeroU315 NonZeroI315: U315, 315, 5;
      u316 i316 NonZeroU316 NonZeroI316: U316, 316, 5;
      u317 i317 NonZeroU317 NonZeroI317: U317, 317, 5;
      u318 i318 NonZeroU318 NonZeroI318: U318, 318, 5;
      u319 i319 NonZeroU319 NonZeroI319: U319, 319, 5;
      u320 i320 NonZeroU320 NonZeroI320: U320, 320, 5;
      u321 i321 NonZeroU321 NonZeroI321: U321, 321, 6;
      u322 i322 NonZeroU322 NonZeroI322: U322, 322, 6;
      u323 i323 NonZeroU323 NonZeroI323: U323, 323, 6;
      u324 i324 NonZeroU324 NonZeroI324: U324, 324, 6;
      u325 i325 NonZeroU325 NonZeroI325: U325, 325, 6;
      u326 i326 NonZeroU326 NonZeroI326: U326, 326, 6;
      u327 i327 NonZeroU327 NonZeroI327: U327, 327, 6;
      u328 i328 NonZeroU328 NonZeroI328: U328, 328, 6;
      u329 i329 NonZeroU329 NonZeroI329: U329, 329, 6;
      u330 i330 NonZeroU330 NonZeroI330: U330, 330, 6;
      u331 i331 NonZeroU331 NonZeroI331: U331, 331, 6;
      u332 i332 NonZeroU332 NonZeroI332: U332, 332, 6;
      u333 i333 NonZeroU333 NonZeroI333: U333, 333, 6;
      u334 i334 NonZeroU334 NonZeroI334: U334, 334, 6;
      u335 i335 NonZeroU335 NonZeroI335: U335, 335, 6;
      u336 i336 NonZeroU336 NonZeroI336: U336, 336, 6;
      u337 i337 NonZeroU337 NonZeroI337: U337, 337, 6;
      u338 i338 NonZeroU338 NonZeroI338: U338, 338, 6;
      u339 i339 NonZeroU339 NonZeroI339: U339, 339, 6;
      u340 i340 NonZeroU340 NonZeroI340: U340, 340, 6;
      u341 i341 NonZeroU341 NonZeroI341: U341, 341, 6;
      u342 i342 NonZeroU342 NonZeroI342: U342, 342, 6;
      u343 i343 NonZeroU343 NonZeroI343: U343, 343, 6;
      u344 i344 NonZeroU344 NonZeroI344: U344, 344, 6;
      u345 i345 NonZeroU345 NonZeroI345: U345, 345, 6;
      u346 i346 NonZeroU346 NonZeroI346: U346, 346, 6;
      u347 i347 NonZeroU347 NonZeroI347: U347, 347, 6;
      u348 i348 NonZeroU348 NonZeroI348: U348, 348, 6;
      u349 i349 NonZeroU349 NonZeroI349: U349, 349, 6;
      u350 i350 NonZeroU350 NonZeroI350: U350, 350, 6;
      u351 i351 NonZeroU351 NonZeroI351: U351, 351, 6;
      u352 i352 NonZeroU352 NonZeroI352: U352, 352, 6;
      u353 i353 NonZeroU353 NonZeroI353: U353, 353, 6;
      u354 i354 NonZeroU354 NonZeroI354: U354, 354, 6;
      u355 i355 NonZeroU355 NonZeroI355: U355, 355, 6;
      u356 i356 NonZeroU356 NonZeroI356: U356, 356, 6;
      u357 i357 NonZeroU357 NonZeroI357: U357, 357, 6;
      u358 i358 NonZeroU358 NonZeroI358: U358, 358, 6;
      u359 i359 NonZeroU359 NonZeroI359: U359, 359, 6;
      u360 i360 NonZeroU360 NonZeroI360: U360, 360, 6;
      u361 i361 NonZeroU361 NonZeroI361: U361, 361, 6;
      u362 i362 NonZeroU362 NonZeroI362: U362, 362, 6;
      u363 i363 NonZeroU363 NonZeroI363: U363, 363, 6;
      u364 i364 NonZeroU364 NonZeroI364: U364, 364, 6;
      u365 i365 NonZeroU365 NonZeroI365: U365, 365, 6;
      u366 i366 NonZeroU366 NonZeroI366: U366, 366, 6;
      u367 i367 NonZeroU367 NonZeroI367: U367, 367, 6;
      u368 i368 NonZeroU368 NonZeroI368: U368, 368, 6;
      u369 i369 NonZeroU369 NonZeroI369: U369, 369, 6;
      u370 i370 NonZeroU370 NonZeroI370: U370, 370, 6;
      u371 i371 NonZeroU371 NonZeroI371: U371, 371, 6;
      u372 i372 NonZeroU372 NonZeroI372: U372, 372, 6;
      u373 i373 NonZeroU373 NonZeroI373: U373, 373, 6;
      u374 i374 NonZeroU374 NonZeroI374: U374, 374, 6;
      u375 i375 NonZeroU375 NonZeroI375: U375, 375, 6;
      u376 i376 NonZeroU376 NonZeroI376: U376, 376, 6;
      u377 i377 NonZeroU377 NonZeroI377: U377, 377, 6;
      u378 i378 NonZeroU378 NonZeroI378: U378, 378, 6;
      u379 i379 NonZeroU379 NonZeroI379: U379, 379, 6;
      u380 i380 NonZeroU380 NonZeroI380: U380, 380, 6;
      u381 i381 NonZeroU381 NonZeroI381: U381, 381, 6;
      u382 i382 NonZeroU382 NonZeroI382: U382, 382, 6;
      u383 i383 NonZeroU383 NonZeroI383: U383, 383, 6;
      u384 i384 NonZeroU384 NonZeroI384: U384, 384, 6;
      u385 i385 NonZeroU385 NonZeroI385: U385, 385, 7;
      u386 i386 NonZeroU386 NonZeroI386: U386, 386, 7;
      u387 i387 NonZeroU387 NonZeroI387: U387, 387, 7;
      u388 i388 NonZeroU388 NonZeroI388: U388, 388, 7;
      u389 i389 NonZeroU389 NonZeroI389: U389, 389, 7;
      u390 i390 NonZeroU390 NonZeroI390: U390, 390, 7;
      u391 i391 NonZeroU391 NonZeroI391: U391, 391, 7;
      u392 i392 NonZeroU392 NonZeroI392: U392, 392, 7;
      u393 i393 NonZeroU393 NonZeroI393: U393, 393, 7;
      u394 i394 NonZeroU394 NonZeroI394: U394, 394, 7;
      u395 i395 NonZeroU395 NonZeroI395: U395, 395, 7;
      u396 i396 NonZeroU396 NonZeroI396: U396, 396, 7;
      u397 i397 NonZeroU397 NonZeroI397: U397, 397, 7;
      u398 i398 NonZeroU398 NonZeroI398: U398, 398, 7;
      u399 i399 NonZeroU399 NonZeroI399: U399, 399, 7;
      u400 i400 NonZeroU400 NonZeroI400: U400, 400, 7;
      u401 i401 NonZeroU401 NonZeroI401: U401, 401, 7;
      u402 i402 NonZeroU402 NonZeroI402: U402, 402, 7;
      u403 i403 NonZeroU403 NonZeroI403: U403, 403, 7;
      u404 i404 NonZeroU404 NonZeroI404: U404, 404, 7;
      u405 i405 NonZeroU405 NonZeroI405: U405, 405, 7;
      u406 i406 NonZeroU406 NonZeroI406: U406, 406, 7;
      u407 i407 NonZeroU407 NonZeroI407: U407, 407, 7;
      u408 i408 NonZeroU408 NonZeroI408: U408, 408, 7;
      u409 i409 NonZeroU409 NonZeroI409: U409, 409, 7;
      u410 i410 NonZeroU410 NonZeroI410: U410, 410, 7;
      u411 i411 NonZeroU411 NonZeroI411: U411, 411, 7;
      u412 i412 NonZeroU412 NonZeroI412: U412, 412, 7;
      u413 i413 NonZeroU413 NonZeroI413: U413, 413, 7;
      u414 i414 NonZeroU414 NonZeroI414: U414, 414, 7;
      u415 i415 NonZeroU415 NonZeroI415: U415, 415, 7;
      u416 i416 NonZeroU416 NonZeroI416: U416, 416, 7;
      u417 i417 NonZeroU417 NonZeroI417: U417, 417, 7;
      u418 i418 NonZeroU418 NonZeroI418: U418, 418, 7;
      u419 i419 NonZeroU419 NonZeroI419: U419, 419, 7;
      u420 i420 NonZeroU420 NonZeroI420: U420, 420, 7;
      u421 i421 NonZeroU421 NonZeroI421: U421, 421, 7;
      u422 i422 NonZeroU422 NonZeroI422: U422, 422, 7;
      u423 i423 NonZeroU423 NonZeroI423: U423, 423, 7;
      u424 i424 NonZeroU424 NonZeroI424: U424, 424, 7;
      u425 i425 NonZeroU425 NonZeroI425: U425, 425, 7;
      u426 i426 NonZeroU426 NonZeroI426: U426, 426, 7;
      u427 i427 NonZeroU427 NonZeroI427: U427, 427, 7;
      u428 i428 NonZeroU428 NonZeroI428: U428, 428, 7;
      u429 i429 NonZeroU429 NonZeroI429: U429, 429, 7;
      u430 i430 NonZeroU430 NonZeroI430: U430, 430, 7;
      u431 i431 NonZeroU431 NonZeroI431: U431, 431, 7;
      u432 i432 NonZeroU432 NonZeroI432: U432, 432, 7;
      u433 i433 NonZeroU433 NonZeroI433: U433, 433, 7;
      u434 i434 NonZeroU434 NonZeroI434: U434, 434, 7;
      u435 i435 NonZeroU435 NonZeroI435: U435, 435, 7;
      u436 i436 NonZeroU436 NonZeroI436: U436, 436, 7;
      u437 i437 NonZeroU437 NonZeroI437: U437, 437, 7;
      u438 i438 NonZeroU438 NonZeroI438: U438, 438, 7;
      u439 i439 NonZeroU439 NonZeroI439: U439, 439, 7;
      u440 i440 NonZeroU440 NonZeroI440: U440, 440, 7;
      u441 i441 NonZeroU441 NonZeroI441: U441, 441, 7;
      u442 i442 NonZeroU442 NonZeroI442: U442, 442, 7;
      u443 i443 NonZeroU443 NonZeroI443: U443, 443, 7;
      u444 i444 NonZeroU444 NonZeroI444: U444, 444, 7;
      u445 i445 NonZeroU445 NonZeroI445: U445, 445, 7;
      u446 i446 NonZeroU446 NonZeroI446: U446, 446, 7;
      u447 i447 NonZeroU447 NonZeroI447: U447, 447, 7;
      u448 i448 NonZeroU448 NonZeroI448: U448, 448, 7;
      u449 i449 NonZeroU449 NonZeroI449: U449, 449, 8;
      u450 i450 NonZeroU450 NonZeroI450: U450, 450, 8;
      u451 i451 NonZeroU451 NonZeroI451: U451, 451, 8;
      u452 i452 NonZeroU452 NonZeroI452: U452, 452, 8;
      u453 i453 NonZeroU453 NonZeroI453: U453, 453, 8;
      u454 i454 NonZeroU454 NonZeroI454: U454, 454, 8;
      u455 i455 NonZeroU455 NonZeroI455: U455, 455, 8;
      u456 i456 NonZeroU456 NonZeroI456: U456, 456, 8;
      u457 i457 NonZeroU457 NonZeroI457: U457, 457, 8;
      u458 i458 NonZeroU458 NonZeroI458: U458, 458, 8;
      u459 i459 NonZeroU459 NonZeroI459: U459, 459, 8;
      u460 i460 NonZeroU460 NonZeroI460: U460, 460, 8;
      u461 i461 NonZeroU461 NonZeroI461: U461, 461, 8;
      u462 i462 NonZeroU462 NonZeroI462: U462, 462, 8;
      u463 i463 NonZeroU463 NonZeroI463: U463, 463, 8;
      u464 i464 NonZeroU464 NonZeroI464: U464, 464, 8;
      u465 i465 NonZeroU465 NonZeroI465: U465, 465, 8;
      u466 i466 NonZeroU466 NonZeroI466: U466, 466, 8;
      u467 i467 NonZeroU467 NonZeroI467: U467, 467, 8;
      u468 i468 NonZeroU468 NonZeroI468: U468, 468, 8;
      u469 i469 NonZeroU469 NonZeroI469: U469, 469, 8;
      u470 i470 NonZeroU470 NonZeroI470: U470, 470, 8;
      u471 i471 NonZeroU471 NonZeroI471: U471, 471, 8;
      u472 i472 NonZeroU472 NonZeroI472: U472, 472, 8;
      u473 i473 NonZeroU473 NonZeroI473: U473, 473, 8;
      u474 i474 NonZeroU474 NonZeroI474: U474, 474, 8;
      u475 i475 NonZeroU475 NonZeroI475: U475, 475, 8;
      u476 i476 NonZeroU476 NonZeroI476: U476, 476, 8;
      u477 i477 NonZeroU477 NonZeroI477: U477, 477, 8;
      u478 i478 NonZeroU478 NonZeroI478: U478, 478, 8;
      u479 i479 NonZeroU479 NonZeroI479: U479, 479, 8;
      u480 i480 NonZeroU480 NonZeroI480: U480, 480, 8;
      u481 i481 NonZeroU481 NonZeroI481: U481, 481, 8;
      u482 i482 NonZeroU482 NonZeroI482: U482, 482, 8;
      u483 i483 NonZeroU483 NonZeroI483: U483, 483, 8;
      u484 i484 NonZeroU484 NonZeroI484: U484, 484, 8;
      u485 i485 NonZeroU485 NonZeroI485: U485, 485, 8;
      u486 i486 NonZeroU486 NonZeroI486: U486, 486, 8;
      u487 i487 NonZeroU487 NonZeroI487: U487, 487, 8;
      u488 i488 NonZeroU488 NonZeroI488: U488, 488, 8;
      u489 i489 NonZeroU489 NonZeroI489: U489, 489, 8;
      u490 i490 NonZeroU490 NonZeroI490: U490, 490, 8;
      u491 i491 NonZeroU491 NonZeroI491: U491, 491, 8;
      u492 i492 NonZeroU492 NonZeroI492: U492, 492, 8;
      u493 i493 NonZeroU493 NonZeroI493: U493, 493, 8;
      u494 i494 NonZeroU494 NonZeroI494: U494, 494, 8;
      u495 i495 NonZeroU495 NonZeroI495: U495, 495, 8;
      u496 i496 NonZeroU496 NonZeroI496: U496, 496, 8;
      u497 i497 NonZeroU497 NonZeroI497: U497, 497, 8;
      u498 i498 NonZeroU498 NonZeroI498: U498, 498, 8;
      u499 i499 NonZeroU499 NonZeroI499: U499, 499, 8;
      u500 i500 NonZeroU500 NonZeroI500: U500, 500, 8;
      u501 i501 NonZeroU501 NonZeroI501: U501, 501, 8;
      u502 i502 NonZeroU502 NonZeroI502: U502, 502, 8;
      u503 i503 NonZeroU503 NonZeroI503: U503, 503, 8;
      u504 i504 NonZeroU504 NonZeroI504: U504, 504, 8;
      u505 i505 NonZeroU505 NonZeroI505: U505, 505, 8;
      u506 i506 NonZeroU506 NonZeroI506: U506, 506, 8;
      u507 i507 NonZeroU507 NonZeroI507: U507, 507, 8;
      u508 i508 NonZeroU508 NonZeroI508: U508, 508, 8;
      u509 i509 NonZeroU509 NonZeroI509: U509, 509, 8;
      u510 i510 NonZeroU510 NonZeroI510: U510, 510, 8;
      u511 i511 NonZeroU511 NonZeroI511: U511, 511, 8;
      u512 i512 NonZeroU512 NonZeroI512: U512, 512, 8;
      u513 i513 NonZeroU513 NonZeroI513: U513, 513, 9;
      u514 i514 NonZeroU514 NonZeroI514: U514, 514, 9;
      u515 i515 NonZeroU515 NonZeroI515: U515, 515, 9;
      u516 i516 NonZeroU516 NonZeroI516: U516, 516, 9;
      u517 i517 NonZeroU517 NonZeroI517: U517, 517, 9;
      u518 i518 NonZeroU518 NonZeroI518: U518, 518, 9;
      u519 i519 NonZeroU519 NonZeroI519: U519, 519, 9;
      u520 i520 NonZeroU520 NonZeroI520: U520, 520, 9;
      u521 i521 NonZeroU521 NonZeroI521: U521, 521, 9;
      u522 i522 NonZeroU522 NonZeroI522: U522, 522, 9;
      u523 i523 NonZeroU523 NonZeroI523: U523, 523, 9;
      u524 i524 NonZeroU524 NonZeroI524: U524, 524, 9;
      u525 i525 NonZeroU525 NonZeroI525: U525, 525, 9;
      u526 i526 NonZeroU526 NonZeroI526: U526, 526, 9;
      u527 i527 NonZeroU527 NonZeroI527: U527, 527, 9;
      u528 i528 NonZeroU528 NonZeroI528: U528, 528, 9;
      u529 i529 NonZeroU529 NonZeroI529: U529, 529, 9;
      u530 i530 NonZeroU530 NonZeroI530: U530, 530, 9;
      u531 i531 NonZeroU531 NonZeroI531: U531, 531, 9;
      u532 i532 NonZeroU532 NonZeroI532: U532, 532, 9;
      u533 i533 NonZeroU533 NonZeroI533: U533, 533, 9;
      u534 i534 NonZeroU534 NonZeroI534: U534, 534, 9;
      u535 i535 NonZeroU535 NonZeroI535: U535, 535, 9;
      u536 i536 NonZeroU536 NonZeroI536: U536, 536, 9;
      u537 i537 NonZeroU537 NonZeroI537: U537, 537, 9;
      u538 i538 NonZeroU538 NonZeroI538: U538, 538, 9;
      u539 i539 NonZeroU539 NonZeroI539: U539, 539, 9;
      u540 i540 NonZeroU540 NonZeroI540: U540, 540, 9;
      u541 i541 NonZeroU541 NonZeroI541: U541, 541, 9;
      u542 i542 NonZeroU542 NonZeroI542: U542, 542, 9;
      u543 i543 NonZeroU543 NonZeroI543: U543, 543, 9;
      u544 i544 NonZeroU544 NonZeroI544: U544, 544, 9;
      u545 i545 NonZeroU545 NonZeroI545: U545, 545, 9;
      u546 i546 NonZeroU546 NonZeroI546: U546, 546, 9;
      u547 i547 NonZeroU547 NonZeroI547: U547, 547, 9;
      u548 i548 NonZeroU548 NonZeroI548: U548, 548, 9;
      u549 i549 NonZeroU549 NonZeroI549: U549, 549, 9;
      u550 i550 NonZeroU550 NonZeroI550: U550, 550, 9;
      u551 i551 NonZeroU551 NonZeroI551: U551, 551, 9;
      u552 i552 NonZeroU552 NonZeroI552: U552, 552, 9;
      u553 i553 NonZeroU553 NonZeroI553: U553, 553, 9;
      u554 i554 NonZeroU554 NonZeroI554: U554, 554, 9;
      u555 i555 NonZeroU555 NonZeroI555: U555, 555, 9;
      u556 i556 NonZeroU556 NonZeroI556: U556, 556, 9;
      u557 i557 NonZeroU557 NonZeroI557: U557, 557, 9;
      u558 i558 NonZeroU558 NonZeroI558: U558, 558, 9;
      u559 i559 NonZeroU559 NonZeroI559: U559, 559, 9;
      u560 i560 NonZeroU560 NonZeroI560: U560, 560, 9;
      u561 i561 NonZeroU561 NonZeroI561: U561, 561, 9;
      u562 i562 NonZeroU562 NonZeroI562: U562, 562, 9;
      u563 i563 NonZeroU563 NonZeroI563: U563, 563, 9;
      u564 i564 NonZeroU564 NonZeroI564: U564, 564, 9;
      u565 i565 NonZeroU565 NonZeroI565: U565, 565, 9;
      u566 i566 NonZeroU566 NonZeroI566: U566, 566, 9;
      u567 i567 NonZeroU567 NonZeroI567: U567, 567, 9;
      u568 i568 NonZeroU568 NonZeroI568: U568, 568, 9;
      u569 i569 NonZeroU569 NonZeroI569: U569, 569, 9;
      u570 i570 NonZeroU570 NonZeroI570: U570, 570, 9;
      u571 i571 NonZeroU571 NonZeroI571: U571, 571, 9;
      u572 i572 NonZeroU572 NonZeroI572: U572, 572, 9;
      u573 i573 NonZeroU573 NonZeroI573: U573, 573, 9;
      u574 i574 NonZeroU574 NonZeroI574: U574, 574, 9;
      u575 i575 NonZeroU575 NonZeroI575: U575, 575, 9;
      u576 i576 NonZeroU576 NonZeroI576: U576, 576, 9;
      u577 i577 NonZeroU577 NonZeroI577: U577, 577, 10;
      u578 i578 NonZeroU578 NonZeroI578: U578, 578, 10;
      u579 i579 NonZeroU579 NonZeroI579: U579, 579, 10;
      u580 i580 NonZeroU580 NonZeroI580: U580, 580, 10;
      u581 i581 NonZeroU581 NonZeroI581: U581, 581, 10;
      u582 i582 NonZeroU582 NonZeroI582: U582, 582, 10;
      u583 i583 NonZeroU583 NonZeroI583: U583, 583, 10;
      u584 i584 NonZeroU584 NonZeroI584: U584, 584, 10;
      u585 i585 NonZeroU585 NonZeroI585: U585, 585, 10;
      u586 i586 NonZeroU586 NonZeroI586: U586, 586, 10;
      u587 i587 NonZeroU587 NonZeroI587: U587, 587, 10;
      u588 i588 NonZeroU588 NonZeroI588: U588, 588, 10;
      u589 i589 NonZeroU589 NonZeroI589: U589, 589, 10;
      u590 i590 NonZeroU590 NonZeroI590: U590, 590, 10;
      u591 i591 NonZeroU591 NonZeroI591: U591, 591, 10;
      u592 i592 NonZeroU592 NonZeroI592: U592, 592, 10;
      u593 i593 NonZeroU593 NonZeroI593: U593, 593, 10;
      u594 i594 NonZeroU594 NonZeroI594: U594, 594, 10;
      u595 i595 NonZeroU595 NonZeroI595: U595, 595, 10;
      u596 i596 NonZeroU596 NonZeroI596: U596, 596, 10;
      u597 i597 NonZeroU597 NonZeroI597: U597, 597, 10;
      u598 i598 NonZeroU598 NonZeroI598: U598, 598, 10;
      u599 i599 NonZeroU599 NonZeroI599: U599, 599, 10;
      u600 i600 NonZeroU600 NonZeroI600: U600, 600, 10;
      u601 i601 NonZeroU601 NonZeroI601: U601, 601, 10;
      u602 i602 NonZeroU602 NonZeroI602: U602, 602, 10;
      u603 i603 NonZeroU603 NonZeroI603: U603, 603, 10;
      u604 i604 NonZeroU604 NonZeroI604: U604, 604, 10;
      u605 i605 NonZeroU605 NonZeroI605: U605, 605, 10;
      u606 i606 NonZeroU606 NonZeroI606: U606, 606, 10;
      u607 i607 NonZeroU607 NonZeroI607: U607, 607, 10;
      u608 i608 NonZeroU608 NonZeroI608: U608, 608, 10;
      u609 i609 NonZeroU609 NonZeroI609: U609, 609, 10;
      u610 i610 NonZeroU610 NonZeroI610: U610, 610, 10;
      u611 i611 NonZeroU611 NonZeroI611: U611, 611, 10;
      u612 i612 NonZeroU612 NonZeroI612: U612, 612, 10;
      u613 i613 NonZeroU613 NonZeroI613: U613, 613, 10;
      u614 i614 NonZeroU614 NonZeroI614: U614, 614, 10;
      u615 i615 NonZeroU615 NonZeroI615: U615, 615, 10;
      u616 i616 NonZeroU616 NonZeroI616: U616, 616, 10;
      u617 i617 NonZeroU617 NonZeroI617: U617, 617, 10;
      u618 i618 NonZeroU618 NonZeroI618: U618, 618, 10;
      u619 i619 NonZeroU619 NonZeroI619: U619, 619, 10;
      u620 i620 NonZeroU620 NonZeroI620: U620, 620, 10;
      u621 i621 NonZeroU621 NonZeroI621: U621, 621, 10;
      u622 i622 NonZeroU622 NonZeroI622: U622, 622, 10;
      u623 i623 NonZeroU623 NonZeroI623: U623, 623, 10;
      u624 i624 NonZeroU624 NonZeroI624: U624, 624, 10;
      u625 i625 NonZeroU625 NonZeroI625: U625, 625, 10;
      u626 i626 NonZeroU626 NonZeroI626: U626, 626, 10;
      u627 i627 NonZeroU627 NonZeroI627: U627, 627, 10;
      u628 i628 NonZeroU628 NonZeroI628: U628, 628, 10;
      u629 i629 NonZeroU629 NonZeroI629: U629, 629, 10;
      u630 i630 NonZeroU630 NonZeroI630: U630, 630, 10;
      u631 i631 NonZeroU631 NonZeroI631: U631, 631, 10;
      u632 i632 NonZeroU632 NonZeroI632: U632, 632, 10;
      u633 i633 NonZeroU633 NonZeroI633: U633, 633, 10;
      u634 i634 NonZeroU634 NonZeroI634: U634, 634, 10;
      u635 i635 NonZeroU635 NonZeroI635: U635, 635, 10;
      u636 i636 NonZeroU636 NonZeroI636: U636, 636, 10;
      u637 i637 NonZeroU637 NonZeroI637: U637, 637, 10;
      u638 i638 NonZeroU638 NonZeroI638: U638, 638, 10;
      u639 i639 NonZeroU639 NonZeroI639: U639, 639, 10;
      u640 i640 NonZeroU640 NonZeroI640: U640, 640, 10;
      u641 i641 NonZeroU641 NonZeroI641: U641, 641, 11;
      u642 i642 NonZeroU642 NonZeroI642: U642, 642, 11;
      u643 i643 NonZeroU643 NonZeroI643: U643, 643, 11;
      u644 i644 NonZeroU644 NonZeroI644: U644, 644, 11;
      u645 i645 NonZeroU645 NonZeroI645: U645, 645, 11;
      u646 i646 NonZeroU646 NonZeroI646: U646, 646, 11;
      u647 i647 NonZeroU647 NonZeroI647: U647, 647, 11;
      u648 i648 NonZeroU648 NonZeroI648: U648, 648, 11;
      u649 i649 NonZeroU649 NonZeroI649: U649, 649, 11;
      u650 i650 NonZeroU650 NonZeroI650: U650, 650, 11;
      u651 i651 NonZeroU651 NonZeroI651: U651, 651, 11;
      u652 i652 NonZeroU652 NonZeroI652: U652, 652, 11;
      u653 i653 NonZeroU653 NonZeroI653: U653, 653, 11;
      u654 i654 NonZeroU654 NonZeroI654: U654, 654, 11;
      u655 i655 NonZeroU655 NonZeroI655: U655, 655, 11;
      u656 i656 NonZeroU656 NonZeroI656: U656, 656, 11;
      u657 i657 NonZeroU657 NonZeroI657: U657, 657, 11;
      u658 i658 NonZeroU658 NonZeroI658: U658, 658, 11;
      u659 i659 NonZeroU659 NonZeroI659: U659, 659, 11;
      u660 i660 NonZeroU660 NonZeroI660: U660, 660, 11;
      u661 i661 NonZeroU661 NonZeroI661: U661, 661, 11;
      u662 i662 NonZeroU662 NonZeroI662: U662, 662, 11;
      u663 i663 NonZeroU663 NonZeroI663: U663, 663, 11;
      u664 i664 NonZeroU664 NonZeroI664: U664, 664, 11;
      u665 i665 NonZeroU665 NonZeroI665: U665, 665, 11;
      u666 i666 NonZeroU666 NonZeroI666: U666, 666, 11;
      u667 i667 NonZeroU667 NonZeroI667: U667, 667, 11;
      u668 i668 NonZeroU668 NonZeroI668: U668, 668, 11;
      u669 i669 NonZeroU669 NonZeroI669: U669, 669, 11;
      u670 i670 NonZeroU670 NonZeroI670: U670, 670, 11;
      u671 i671 NonZeroU671 NonZeroI671: U671, 671, 11;
      u672 i672 NonZeroU672 NonZeroI672: U672, 672, 11;
      u673 i673 NonZeroU673 NonZeroI673: U673, 673, 11;
      u674 i674 NonZeroU674 NonZeroI674: U674, 674, 11;
      u675 i675 NonZeroU675 NonZeroI675: U675, 675, 11;
      u676 i676 NonZeroU676 NonZeroI676: U676, 676, 11;
      u677 i677 NonZeroU677 NonZeroI677: U677, 677, 11;
      u678 i678 NonZeroU678 NonZeroI678: U678, 678, 11;
      u679 i679 NonZeroU679 NonZeroI679: U679, 679, 11;
      u680 i680 NonZeroU680 NonZeroI680: U680, 680, 11;
      u681 i681 NonZeroU681 NonZeroI681: U681, 681, 11;
      u682 i682 NonZeroU682 NonZeroI682: U682, 682, 11;
      u683 i683 NonZeroU683 NonZeroI683: U683, 683, 11;
      u684 i684 NonZeroU684 NonZeroI684: U684, 684, 11;
      u685 i685 NonZeroU685 NonZeroI685: U685, 685, 11;
      u686 i686 NonZeroU686 NonZeroI686: U686, 686, 11;
      u687 i687 NonZeroU687 NonZeroI687: U687, 687, 11;
      u688 i688 NonZeroU688 NonZeroI688: U688, 688, 11;
      u689 i689 NonZeroU689 NonZeroI689: U689, 689, 11;
      u690 i690 NonZeroU690 NonZeroI690: U690, 690, 11;
      u691 i691 NonZeroU691 NonZeroI691: U691, 691, 11;
      u692 i692 NonZeroU692 NonZeroI692: U692, 692, 11;
      u693 i693 NonZeroU693 NonZeroI693: U693, 693, 11;
      u694 i694 NonZeroU694 NonZeroI694: U694, 694, 11;
      u695 i695 NonZeroU695 NonZeroI695: U695, 695, 11;
      u696 i696 NonZeroU696 NonZeroI696: U696, 696, 11;
      u697 i697 NonZeroU697 NonZeroI697: U697, 697, 11;
      u698 i698 NonZeroU698 NonZeroI698: U698, 698, 11;
      u699 i699 NonZeroU699 NonZeroI699: U699, 699, 11;
      u700 i700 NonZeroU700 NonZeroI700: U700, 700, 11;
      u701 i701 NonZeroU701 NonZeroI701: U701, 701, 11;
      u702 i702 NonZeroU702 NonZeroI702: U702, 702, 11;
      u703 i703 NonZeroU703 NonZeroI703: U703, 703, 11;
      u704 i704 NonZeroU704 NonZeroI704: U704, 704, 11;
      u705 i705 NonZeroU705 NonZeroI705: U705, 705, 12;
      u706 i706 NonZeroU706 NonZeroI706: U706, 706, 12;
      u707 i707 NonZeroU707 NonZeroI707: U707, 707, 12;
      u708 i708 NonZeroU708 NonZeroI708: U708, 708, 12;
      u709 i709 NonZeroU709 NonZeroI709: U709, 709, 12;
      u710 i710 NonZeroU710 NonZeroI710: U710, 710, 12;
      u711 i711 NonZeroU711 NonZeroI711: U711, 711, 12;
      u712 i712 NonZeroU712 NonZeroI712: U712, 712, 12;
      u713 i713 NonZeroU713 NonZeroI713: U713, 713, 12;
      u714 i714 NonZeroU714 NonZeroI714: U714, 714, 12;
      u715 i715 NonZeroU715 NonZeroI715: U715, 715, 12;
      u716 i716 NonZeroU716 NonZeroI716: U716, 716, 12;
      u717 i717 NonZeroU717 NonZeroI717: U717, 717, 12;
      u718 i718 NonZeroU718 NonZeroI718: U718, 718, 12;
      u719 i719 NonZeroU719 NonZeroI719: U719, 719, 12;
      u720 i720 NonZeroU720 NonZeroI720: U720, 720, 12;
      u721 i721 NonZeroU721 NonZeroI721: U721, 721, 12;
      u722 i722 NonZeroU722 NonZeroI722: U722, 722, 12;
      u723 i723 NonZeroU723 NonZeroI723: U723, 723, 12;
      u724 i724 NonZeroU724 NonZeroI724: U724, 724, 12;
      u725 i725 NonZeroU725 NonZeroI725: U725, 725, 12;
      u726 i726 NonZeroU726 NonZeroI726: U726, 726, 12;
      u727 i727 NonZeroU727 NonZeroI727: U727, 727, 12;
      u728 i728 NonZeroU728 NonZeroI728: U728, 728, 12;
      u729 i729 NonZeroU729 NonZeroI729: U729, 729, 12;
      u730 i730 NonZeroU730 NonZeroI730: U730, 730, 12;
      u731 i731 NonZeroU731 NonZeroI731: U731, 731, 12;
      u732 i732 NonZeroU732 NonZeroI732: U732, 732, 12;
      u733 i733 NonZeroU733 NonZeroI733: U733, 733, 12;
      u734 i734 NonZeroU734 NonZeroI734: U734, 734, 12;
      u735 i735 NonZeroU735 NonZeroI735: U735, 735, 12;
      u736 i736 NonZeroU736 NonZeroI736: U736, 736, 12;
      u737 i737 NonZeroU737 NonZeroI737: U737, 737, 12;
      u738 i738 NonZeroU738 NonZeroI738: U738, 738, 12;
      u739 i739 NonZeroU739 NonZeroI739: U739, 739, 12;
      u740 i740 NonZeroU740 NonZeroI740: U740, 740, 12;
      u741 i741 NonZeroU741 NonZeroI741: U741, 741, 12;
      u742 i742 NonZeroU742 NonZeroI742: U742, 742, 12;
      u743 i743 NonZeroU743 NonZeroI743: U743, 743, 12;
      u744 i744 NonZeroU744 NonZeroI744: U744, 744, 12;
      u745 i745 NonZeroU745 NonZeroI745: U745, 745, 12;
      u746 i746 NonZeroU746 NonZeroI746: U746, 746, 12;
      u747 i747 NonZeroU747 NonZeroI747: U747, 747, 12;
      u748 i748 NonZeroU748 NonZeroI748: U748, 748, 12;
      u749 i749 NonZeroU749 NonZeroI749: U749, 749, 12;
      u750 i750 NonZeroU750 NonZeroI750: U750, 750, 12;
      u751 i751 NonZeroU751 NonZeroI751: U751, 751, 12;
      u752 i752 NonZeroU752 NonZeroI752: U752, 752, 12;
      u753 i753 NonZeroU753 NonZeroI753: U753, 753, 12;
      u754 i754 NonZeroU754 NonZeroI754: U754, 754, 12;
      u755 i755 NonZeroU755 NonZeroI755: U755, 755, 12;
      u756 i756 NonZeroU756 NonZeroI756: U756, 756, 12;
      u757 i757 NonZeroU757 NonZeroI757: U757, 757, 12;
      u758 i758 NonZeroU758 NonZeroI758: U758, 758, 12;
      u759 i759 NonZeroU759 NonZeroI759: U759, 759, 12;
      u760 i760 NonZeroU760 NonZeroI760: U760, 760, 12;
      u761 i761 NonZeroU761 NonZeroI761: U761, 761, 12;
      u762 i762 NonZeroU762 NonZeroI762: U762, 762, 12;
      u763 i763 NonZeroU763 NonZeroI763: U763, 763, 12;
      u764 i764 NonZeroU764 NonZeroI764: U764, 764, 12;
      u765 i765 NonZeroU765 NonZeroI765: U765, 765, 12;
      u766 i766 NonZeroU766 NonZeroI766: U766, 766, 12;
      u767 i767 NonZeroU767 NonZeroI767: U767, 767, 12;
      u768 i768 NonZeroU768 NonZeroI768: U768, 768, 12;
      u769 i769 NonZeroU769 NonZeroI769: U769, 769, 13;
      u770 i770 NonZeroU770 NonZeroI770: U770, 770, 13;
      u771 i771 NonZeroU771 NonZeroI771: U771, 771, 13;
      u772 i772 NonZeroU772 NonZeroI772: U772, 772, 13;
      u773 i773 NonZeroU773 NonZeroI773: U773, 773, 13;
      u774 i774 NonZeroU774 NonZeroI774: U774, 774, 13;
      u775 i775 NonZeroU775 NonZeroI775: U775, 775, 13;
      u776 i776 NonZeroU776 NonZeroI776: U776, 776, 13;
      u777 i777 NonZeroU777 NonZeroI777: U777, 777, 13;
      u778 i778 NonZeroU778 NonZeroI778: U778, 778, 13;
      u779 i779 NonZeroU779 NonZeroI779: U779, 779, 13;
      u780 i780 NonZeroU780 NonZeroI780: U780, 780, 13;
      u781 i781 NonZeroU781 NonZeroI781: U781, 781, 13;
      u782 i782 NonZeroU782 NonZeroI782: U782, 782, 13;
      u783 i783 NonZeroU783 NonZeroI783: U783, 783, 13;
      u784 i784 NonZeroU784 NonZeroI784: U784, 784, 13;
      u785 i785 NonZeroU785 NonZeroI785: U785, 785, 13;
      u786 i786 NonZeroU786 NonZeroI786: U786, 786, 13;
      u787 i787 NonZeroU787 NonZeroI787: U787, 787, 13;
      u788 i788 NonZeroU788 NonZeroI788: U788, 788, 13;
      u789 i789 NonZeroU789 NonZeroI789: U789, 789, 13;
      u790 i790 NonZeroU790 NonZeroI790: U790, 790, 13;
      u791 i791 NonZeroU791 NonZeroI791: U791, 791, 13;
      u792 i792 NonZeroU792 NonZeroI792: U792, 792, 13;
      u793 i793 NonZeroU793 NonZeroI793: U793, 793, 13;
      u794 i794 NonZeroU794 NonZeroI794: U794, 794, 13;
      u795 i795 NonZeroU795 NonZeroI795: U795, 795, 13;
      u796 i796 NonZeroU796 NonZeroI796: U796, 796, 13;
      u797 i797 NonZeroU797 NonZeroI797: U797, 797, 13;
      u798 i798 NonZeroU798 NonZeroI798: U798, 798, 13;
      u799 i799 NonZeroU799 NonZeroI799: U799, 799, 13;
      u800 i800 NonZeroU800 NonZeroI800: U800, 800, 13;
      u801 i801 NonZeroU801 NonZeroI801: U801, 801, 13;
      u802 i802 NonZeroU802 NonZeroI802: U802, 802, 13;
      u803 i803 NonZeroU803 NonZeroI803: U803, 803, 13;
      u804 i804 NonZeroU804 NonZeroI804: U804, 804, 13;
      u805 i805 NonZeroU805 NonZeroI805: U805, 805, 13;
      u806 i806 NonZeroU806 NonZeroI806: U806, 806, 13;
      u807 i807 NonZeroU807 NonZeroI807: U807, 807, 13;
      u808 i808 NonZeroU808 NonZeroI808: U808, 808, 13;
      u809 i809 NonZeroU809 NonZeroI809: U809, 809, 13;
      u810 i810 NonZeroU810 NonZeroI810: U810, 810, 13;
      u811 i811 NonZeroU811 NonZeroI811: U811, 811, 13;
      u812 i812 NonZeroU812 NonZeroI812: U812, 812, 13;
      u813 i813 NonZeroU813 NonZeroI813: U813, 813, 13;
      u814 i814 NonZeroU814 NonZeroI814: U814, 814, 13;
      u815 i815 NonZeroU815 NonZeroI815: U815, 815, 13;
      u816 i816 NonZeroU816 NonZeroI816: U816, 816, 13;
      u817 i817 NonZeroU817 NonZeroI817: U817, 817, 13;
      u818 i818 NonZeroU818 NonZeroI818: U818, 818, 13;
      u819 i819 NonZeroU819 NonZeroI819: U819, 819, 13;
      u820 i820 NonZeroU820 NonZeroI820: U820, 820, 13;
      u821 i821 NonZeroU821 NonZeroI821: U821, 821, 13;
      u822 i822 NonZeroU822 NonZeroI822: U822, 822, 13;
      u823 i823 NonZeroU823 NonZeroI823: U823, 823, 13;
      u824 i824 NonZeroU824 NonZeroI824: U824, 824, 13;
      u825 i825 NonZeroU825 NonZeroI825: U825, 825, 13;
      u826 i826 NonZeroU826 NonZeroI826: U826, 826, 13;
      u827 i827 NonZeroU827 NonZeroI827: U827, 827, 13;
      u828 i828 NonZeroU828 NonZeroI828: U828, 828, 13;
      u829 i829 NonZeroU829 NonZeroI829: U829, 829, 13;
      u830 i830 NonZeroU830 NonZeroI830: U830, 830, 13;
      u831 i831 NonZeroU831 NonZeroI831: U831, 831, 13;
      u832 i832 NonZeroU832 NonZeroI832: U832, 832, 13;
      u833 i833 NonZeroU833 NonZeroI833: U833, 833, 14;
      u834 i834 NonZeroU834 NonZeroI834: U834, 834, 14;
      u835 i835 NonZeroU835 NonZeroI835: U835, 835, 14;
      u836 i836 NonZeroU836 NonZeroI836: U836, 836, 14;
      u837 i837 NonZeroU837 NonZeroI837: U837, 837, 14;
      u838 i838 NonZeroU838 NonZeroI838: U838, 838, 14;
      u839 i839 NonZeroU839 NonZeroI839: U839, 839, 14;
      u840 i840 NonZeroU840 NonZeroI840: U840, 840, 14;
      u841 i841 NonZeroU841 NonZeroI841: U841, 841, 14;
      u842 i842 NonZeroU842 NonZeroI842: U842, 842, 14;
      u843 i843 NonZeroU843 NonZeroI843: U843, 843, 14;
      u844 i844 NonZeroU844 NonZeroI844: U844, 844, 14;
      u845 i845 NonZeroU845 NonZeroI845: U845, 845, 14;
      u846 i846 NonZeroU846 NonZeroI846: U846, 846, 14;
      u847 i847 NonZeroU847 NonZeroI847: U847, 847, 14;
      u848 i848 NonZeroU848 NonZeroI848: U848, 848, 14;
      u849 i849 NonZeroU849 NonZeroI849: U849, 849, 14;
      u850 i850 NonZeroU850 NonZeroI850: U850, 850, 14;
      u851 i851 NonZeroU851 NonZeroI851: U851, 851, 14;
      u852 i852 NonZeroU852 NonZeroI852: U852, 852, 14;
      u853 i853 NonZeroU853 NonZeroI853: U853, 853, 14;
      u854 i854 NonZeroU854 NonZeroI854: U854, 854, 14;
      u855 i855 NonZeroU855 NonZeroI855: U855, 855, 14;
      u856 i856 NonZeroU856 NonZeroI856: U856, 856, 14;
      u857 i857 NonZeroU857 NonZeroI857: U857, 857, 14;
      u858 i858 NonZeroU858 NonZeroI858: U858, 858, 14;
      u859 i859 NonZeroU859 NonZeroI859: U859, 859, 14;
      u860 i860 NonZeroU860 NonZeroI860: U860, 860, 14;
      u861 i861 NonZeroU861 NonZeroI861: U861, 861, 14;
      u862 i862 NonZeroU862 NonZeroI862: U862, 862, 14;
      u863 i863 NonZeroU863 NonZeroI863: U863, 863, 14;
      u864 i864 NonZeroU864 NonZeroI864: U864, 864, 14;
      u865 i865 NonZeroU865 NonZeroI865: U865, 865, 14;
      u866 i866 NonZeroU866 NonZeroI866: U866, 866, 14;
      u867 i867 NonZeroU867 NonZeroI867: U867, 867, 14;
      u868 i868 NonZeroU868 NonZeroI868: U868, 868, 14;
      u869 i869 NonZeroU869 NonZeroI869: U869, 869, 14;
      u870 i870 NonZeroU870 NonZeroI870: U870, 870, 14;
      u871 i871 NonZeroU871 NonZeroI871: U871, 871, 14;
      u872 i872 NonZeroU872 NonZeroI872: U872, 872, 14;
      u873 i873 NonZeroU873 NonZeroI873: U873, 873, 14;
      u874 i874 NonZeroU874 NonZeroI874: U874, 874, 14;
      u875 i875 NonZeroU875 NonZeroI875: U875, 875, 14;
      u876 i876 NonZeroU876 NonZeroI876: U876, 876, 14;
      u877 i877 NonZeroU877 NonZeroI877: U877, 877, 14;
      u878 i878 NonZeroU878 NonZeroI878: U878, 878, 14;
      u879 i879 NonZeroU879 NonZeroI879: U879, 879, 14;
      u880 i880 NonZeroU880 NonZeroI880: U880, 880, 14;
      u881 i881 NonZeroU881 NonZeroI881: U881, 881, 14;
      u882 i882 NonZeroU882 NonZeroI882: U882, 882, 14;
      u883 i883 NonZeroU883 NonZeroI883: U883, 883, 14;
      u884 i884 NonZeroU884 NonZeroI884: U884, 884, 14;
      u885 i885 NonZeroU885 NonZeroI885: U885, 885, 14;
      u886 i886 NonZeroU886 NonZeroI886: U886, 886, 14;
      u887 i887 NonZeroU887 NonZeroI887: U887, 887, 14;
      u888 i888 NonZeroU888 NonZeroI888: U888, 888, 14;
      u889 i889 NonZeroU889 NonZeroI889: U889, 889, 14;
      u890 i890 NonZeroU890 NonZeroI890: U890, 890, 14;
      u891 i891 NonZeroU891 NonZeroI891: U891, 891, 14;
      u892 i892 NonZeroU892 NonZeroI892: U892, 892, 14;
      u893 i893 NonZeroU893 NonZeroI893: U893, 893, 14;
      u894 i894 NonZeroU894 NonZeroI894: U894, 894, 14;
      u895 i895 NonZeroU895 NonZeroI895: U895, 895, 14;
      u896 i896 NonZeroU896 NonZeroI896: U896, 896, 14;
      u897 i897 NonZeroU897 NonZeroI897: U897, 897, 15;
      u898 i898 NonZeroU898 NonZeroI898: U898, 898, 15;
      u899 i899 NonZeroU899 NonZeroI899: U899, 899, 15;
      u900 i900 NonZeroU900 NonZeroI900: U900, 900, 15;
      u901 i901 NonZeroU901 NonZeroI901: U901, 901, 15;
      u902 i902 NonZeroU902 NonZeroI902: U902, 902, 15;
      u903 i903 NonZeroU903 NonZeroI903: U903, 903, 15;
      u904 i904 NonZeroU904 NonZeroI904: U904, 904, 15;
      u905 i905 NonZeroU905 NonZeroI905: U905, 905, 15;
      u906 i906 NonZeroU906 NonZeroI906: U906, 906, 15;
      u907 i907 NonZeroU907 NonZeroI907: U907, 907, 15;
      u908 i908 NonZeroU908 NonZeroI908: U908, 908, 15;
      u909 i909 NonZeroU909 NonZeroI909: U909, 909, 15;
      u910 i910 NonZeroU910 NonZeroI910: U910, 910, 15;
      u911 i911 NonZeroU911 NonZeroI911: U911, 911, 15;
      u912 i912 NonZeroU912 NonZeroI912: U912, 912, 15;
      u913 i913 NonZeroU913 NonZeroI913: U913, 913, 15;
      u914 i914 NonZeroU914 NonZeroI914: U914, 914, 15;
      u915 i915 NonZeroU915 NonZeroI915: U915, 915, 15;
      u916 i916 NonZeroU916 NonZeroI916: U916, 916, 15;
      u917 i917 NonZeroU917 NonZeroI917: U917, 917, 15;
      u918 i918 NonZeroU918 NonZeroI918: U918, 918, 15;
      u919 i919 NonZeroU919 NonZeroI919: U919, 919, 15;
      u920 i920 NonZeroU920 NonZeroI920: U920, 920, 15;
      u921 i921 NonZeroU921 NonZeroI921: U921, 921, 15;
      u922 i922 NonZeroU922 NonZeroI922: U922, 922, 15;
      u923 i923 NonZeroU923 NonZeroI923: U923, 923, 15;
      u924 i924 NonZeroU924 NonZeroI924: U924, 924, 15;
      u925 i925 NonZeroU925 NonZeroI925: U925, 925, 15;
      u926 i926 NonZeroU926 NonZeroI926: U926, 926, 15;
      u927 i927 NonZeroU927 NonZeroI927: U927, 927, 15;
      u928 i928 NonZeroU928 NonZeroI928: U928, 928, 15;
      u929 i929 NonZeroU929 NonZeroI929: U929, 929, 15;
      u930 i930 NonZeroU930 NonZeroI930: U930, 930, 15;
      u931 i931 NonZeroU931 NonZeroI931: U931, 931, 15;
      u932 i932 NonZeroU932 NonZeroI932: U932, 932, 15;
      u933 i933 NonZeroU933 NonZeroI933: U933, 933, 15;
      u934 i934 NonZeroU934 NonZeroI934: U934, 934, 15;
      u935 i935 NonZeroU935 NonZeroI935: U935, 935, 15;
      u936 i936 NonZeroU936 NonZeroI936: U936, 936, 15;
      u937 i937 NonZeroU937 NonZeroI937: U937, 937, 15;
      u938 i938 NonZeroU938 NonZeroI938: U938, 938, 15;
      u939 i939 NonZeroU939 NonZeroI939: U939, 939, 15;
      u940 i940 NonZeroU940 NonZeroI940: U940, 940, 15;
      u941 i941 NonZeroU941 NonZeroI941: U941, 941, 15;
      u942 i942 NonZeroU942 NonZeroI942: U942, 942, 15;
      u943 i943 NonZeroU943 NonZeroI943: U943, 943, 15;
      u944 i944 NonZeroU944 NonZeroI944: U944, 944, 15;
      u945 i945 NonZeroU945 NonZeroI945: U945, 945, 15;
      u946 i946 NonZeroU946 NonZeroI946: U946, 946, 15;
      u947 i947 NonZeroU947 NonZeroI947: U947, 947, 15;
      u948 i948 NonZeroU948 NonZeroI948: U948, 948, 15;
      u949 i949 NonZeroU949 NonZeroI949: U949, 949, 15;
      u950 i950 NonZeroU950 NonZeroI950: U950, 950, 15;
      u951 i951 NonZeroU951 NonZeroI951: U951, 951, 15;
      u952 i952 NonZeroU952 NonZeroI952: U952, 952, 15;
      u953 i953 NonZeroU953 NonZeroI953: U953, 953, 15;
      u954 i954 NonZeroU954 NonZeroI954: U954, 954, 15;
      u955 i955 NonZeroU955 NonZeroI955: U955, 955, 15;
      u956 i956 NonZeroU956 NonZeroI956: U956, 956, 15;
      u957 i957 NonZeroU957 NonZeroI957: U957, 957, 15;
      u958 i958 NonZeroU958 NonZeroI958: U958, 958, 15;
      u959 i959 NonZeroU959 NonZeroI959: U959, 959, 15;
      u960 i960 NonZeroU960 NonZeroI960: U960, 960, 15;
      u961 i961 NonZeroU961 NonZeroI961: U961, 961, 16;
      u962 i962 NonZeroU962 NonZeroI962: U962, 962, 16;
      u963 i963 NonZeroU963 NonZeroI963: U963, 963, 16;
      u964 i964 NonZeroU964 NonZeroI964: U964, 964, 16;
      u965 i965 NonZeroU965 NonZeroI965: U965, 965, 16;
      u966 i966 NonZeroU966 NonZeroI966: U966, 966, 16;
      u967 i967 NonZeroU967 NonZeroI967: U967, 967, 16;
      u968 i968 NonZeroU968 NonZeroI968: U968, 968, 16;
      u969 i969 NonZeroU969 NonZeroI969: U969, 969, 16;
      u970 i970 NonZeroU970 NonZeroI970: U970, 970, 16;
      u971 i971 NonZeroU971 NonZeroI971: U971, 971, 16;
      u972 i972 NonZeroU972 NonZeroI972: U972, 972, 16;
      u973 i973 NonZeroU973 NonZeroI973: U973, 973, 16;
      u974 i974 NonZeroU974 NonZeroI974: U974, 974, 16;
      u975 i975 NonZeroU975 NonZeroI975: U975, 975, 16;
      u976 i976 NonZeroU976 NonZeroI976: U976, 976, 16;
      u977 i977 NonZeroU977 NonZeroI977: U977, 977, 16;
      u978 i978 NonZeroU978 NonZeroI978: U978, 978, 16;
      u979 i979 NonZeroU979 NonZeroI979: U979, 979, 16;
      u980 i980 NonZeroU980 NonZeroI980: U980, 980, 16;
      u981 i981 NonZeroU981 NonZeroI981: U981, 981, 16;
      u982 i982 NonZeroU982 NonZeroI982: U982, 982, 16;
      u983 i983 NonZeroU983 NonZeroI983: U983, 983, 16;
      u984 i984 NonZeroU984 NonZeroI984: U984, 984, 16;
      u985 i985 NonZeroU985 NonZeroI985: U985, 985, 16;
      u986 i986 NonZeroU986 NonZeroI986: U986, 986, 16;
      u987 i987 NonZeroU987 NonZeroI987: U987, 987, 16;
      u988 i988 NonZeroU988 NonZeroI988: U988, 988, 16;
      u989 i989 NonZeroU989 NonZeroI989: U989, 989, 16;
      u990 i990 NonZeroU990 NonZeroI990: U990, 990, 16;
      u991 i991 NonZeroU991 NonZeroI991: U991, 991, 16;
      u992 i992 NonZeroU992 NonZeroI992: U992, 992, 16;
      u993 i993 NonZeroU993 NonZeroI993: U993, 993, 16;
      u994 i994 NonZeroU994 NonZeroI994: U994, 994, 16;
      u995 i995 NonZeroU995 NonZeroI995: U995, 995, 16;
      u996 i996 NonZeroU996 NonZeroI996: U996, 996, 16;
      u997 i997 NonZeroU997 NonZeroI997: U997, 997, 16;
      u998 i998 NonZeroU998 NonZeroI998: U998, 998, 16;
      u999 i999 NonZeroU999 NonZeroI999: U999, 999, 16;
      u1000 i1000 NonZeroU1000 NonZeroI1000: U1000, 1000, 16;
      u1001 i1001 NonZeroU1001 NonZeroI1001: U1001, 1001, 16;
      u1002 i1002 NonZeroU1002 NonZeroI1002: U1002, 1002, 16;
      u1003 i1003 NonZeroU1003 NonZeroI1003: U1003, 1003, 16;
      u1004 i1004 NonZeroU1004 NonZeroI1004: U1004, 1004, 16;
      u1005 i1005 NonZeroU1005 NonZeroI1005: U1005, 1005, 16;
      u1006 i1006 NonZeroU1006 NonZeroI1006: U1006, 1006, 16;
      u1007 i1007 NonZeroU1007 NonZeroI1007: U1007, 1007, 16;
      u1008 i1008 NonZeroU1008 NonZeroI1008: U1008, 1008, 16;
      u1009 i1009 NonZeroU1009 NonZeroI1009: U1009, 1009, 16;
      u1010 i1010 NonZeroU1010 NonZeroI1010: U1010, 1010, 16;
      u1011 i1011 NonZeroU1011 NonZeroI1011: U1011, 1011, 16;
      u1012 i1012 NonZeroU1012 NonZeroI1012: U1012, 1012, 16;
      u1013 i1013 NonZeroU1013 NonZeroI1013: U1013, 1013, 16;
      u1014 i1014 NonZeroU1014 NonZeroI1014: U1014, 1014, 16;
      u1015 i1015 NonZeroU1015 NonZeroI1015: U1015, 1015, 16;
      u1016 i1016 NonZeroU1016 NonZeroI1016: U1016, 1016, 16;
      u1017 i1017 NonZeroU1017 NonZeroI1017: U1017, 1017, 16;
      u1018 i1018 NonZeroU1018 NonZeroI1018: U1018, 1018, 16;
      u1019 i1019 NonZeroU1019 NonZeroI1019: U1019, 1019, 16;
      u1020 i1020 NonZeroU1020 NonZeroI1020: U1020, 1020, 16;
      u1021 i1021 NonZeroU1021 NonZeroI1021: U1021, 1021, 16;
      u1022 i1022 NonZeroU1022 NonZeroI1022: U1022, 1022, 16;
      u1023 i1023 NonZeroU1023 NonZeroI1023: U1023, 1023, 16;
      u1024 i1024 NonZeroU1024 NonZeroI1024: U1024, 1024, 16;
    }
  };
}

macro_rules! wide_aliases {
  ($(
    $unsigned:ident $signed:ident $nonzero_unsigned:ident $nonzero_signed:ident:
      $constant:ident, $bits:tt, $limbs:tt;
  )+) => {$(
    #[doc = concat!(
      "An unsigned ",
      stringify!($bits),
      "-bit integer, stored in `[u64; ",
      stringify!($limbs),
      "]`."
    )]
    pub type $unsigned = Uint<typenum::consts::$constant, [u64; $limbs]>;

    #[doc = concat!(
      "A signed ",
      stringify!($bits),
      "-bit integer, stored in `[u64; ",
      stringify!($limbs),
      "]` in two's complement."
    )]
    pub type $signed = Int<typenum::consts::$constant, [u64; $limbs]>;

    #[doc = concat!(
      "A non-zero unsigned ",
      stringify!($bits),
      "-bit integer, stored in `[u64; ",
      stringify!($limbs),
      "]`."
    )]
    pub type $nonzero_unsigned = NonZeroUint<typenum::consts::$constant, [u64; $limbs]>;

    #[doc = concat!(
      "A non-zero signed ",
      stringify!($bits),
      "-bit integer, stored in `[u64; ",
      stringify!($limbs),
      "]` in two's complement."
    )]
    pub type $nonzero_signed = NonZeroInt<typenum::consts::$constant, [u64; $limbs]>;
  )+};
}

wide_alias_table!(wide_aliases);

/// An unsigned 0-bit integer, stored in `[u64; 0]`: it holds only 0 and is zero-sized.
pub type u0 = Uint<U0, [u64; 0]>;

/// A signed 0-bit integer, stored in `[u64; 0]`: it holds only 0 and is zero-sized.
pub type i0 = Int<U0, [u64; 0]>;

/// A 2048-bit unsigned integer, stored in 32 64-bit limbs.
pub type u2048 = Uint<U2048, [u64; 32]>;

/// A 4096-bit unsigned integer, stored in 64 64-bit limbs.
pub type u4096 = Uint<U4096, [u64; 64]>;

/// A 2048-bit signed integer, stored in 32 64-bit limbs in two's complement.
pub type i2048 = Int<U2048, [u64; 32]>;

/// A 4096-bit signed integer, stored in 64 64-bit limbs in two's complement.
pub type i4096 = Int<U4096, [u64; 64]>;

/// A non-zero unsigned 2048-bit integer, stored in `[u64; 32]`.
pub type NonZeroU2048 = NonZeroUint<U2048, [u64; 32]>;

/// A non-zero unsigned 4096-bit integer, stored in `[u64; 64]`.
pub type NonZeroU4096 = NonZeroUint<U4096, [u64; 64]>;

/// A non-zero signed 2048-bit integer, stored in `[u64; 32]` in two's complement.
pub type NonZeroI2048 = NonZeroInt<U2048, [u64; 32]>;

/// A non-zero signed 4096-bit integer, stored in `[u64; 64]` in two's complement.
pub type NonZeroI4096 = NonZeroInt<U4096, [u64; 64]>;

#[cfg(test)]
mod tests;
