// Each row of the digit table gets a test named after its `typenum` constant.
#![allow(non_snake_case)]

use core::{any::TypeId, fmt::Debug};
use std::{string::ToString, vec, vec::Vec};

use num_bigint::{BigInt, BigUint};
use typenum::{
  B0, B1, U0, U1, U2, U3, U8, U9, U10, U18, U19, U20, U38, U39, U57, U58, U76, U77, U78, U79, U96,
  U97, U98, U115, U116, U117, U134, U135, U153, U154, Unsigned,
};

use super::{
  DigitBuf, DigitCount, DivIntermediate, FitsDigits, InlineCore, Layout, LayoutDigits, Pick,
  StorageCore, Token, WidenRepr,
};
use crate::{bigint, i192, i256, i320, i384, i448, i512, u192, u256, u320, u384, u448, u512};

/// Every signed storage with its number of value bits, smallest storage first.
const SIGNED: [(&str, u32); 11] = [
  ("i8", 7),
  ("i16", 15),
  ("i32", 31),
  ("i64", 63),
  ("i128", 127),
  ("i192", 191),
  ("i256", 255),
  ("i320", 319),
  ("i384", 383),
  ("i448", 447),
  ("i512", 511),
];

/// Every unsigned storage with its number of value bits, smallest storage first.
const UNSIGNED: [(&str, u32); 11] = [
  ("u8", 8),
  ("u16", 16),
  ("u32", 32),
  ("u64", 64),
  ("u128", 128),
  ("u192", 192),
  ("u256", 256),
  ("u320", 320),
  ("u384", 384),
  ("u448", 448),
  ("u512", 512),
];

fn assert_fits<Digits: FitsDigits<Repr>, Repr>() {}

/// Checks the storages that a row lists for one sign class: the first holds `10^digits - 1` while
/// the next smaller storage of the class does not, and the list is every storage of the class that
/// holds it. Each listed storage also holds `10^digits`, which the integer core relies on when it
/// scales by a power of ten.
fn assert_class(storages: &[(&str, u32)], digits: u32, listed: &[&str]) {
  let power = BigUint::from(10u8).pow(digits);
  let max_coeff = &power - 1u8;
  let max = |bits: u32| (BigUint::from(1u8) << bits) - 1u8;
  let minimal = storages
    .iter()
    .position(|&(name, _)| name == listed[0])
    .expect("the table names a known storage");
  assert!(
    max_coeff <= max(storages[minimal].1),
    "the minimal storage holds the digits"
  );
  if let Some(smaller) = minimal.checked_sub(1) {
    assert!(
      max_coeff > max(storages[smaller].1),
      "the next smaller storage does not"
    );
  }
  let holding = storages
    .iter()
    .filter(|&&(_, bits)| max_coeff <= max(bits))
    .map(|&(name, _)| name);
  assert!(listed.iter().copied().eq(holding));
  for &(_, bits) in storages.iter().filter(|&&(name, _)| listed.contains(&name)) {
    assert!(
      power <= max(bits),
      "a storage that holds the digits holds the power"
    );
  }
}

macro_rules! test_digits {
  ($(
    $digits:ident = $value:literal =>
      [$signed:ident $(, $wider_signed:ident)*],
      [$unsigned:ident $(, $wider_unsigned:ident)*]
  ),+ $(,)?) => {$(
    #[test]
    #[cfg_attr(
      all(miri, target_pointer_width = "32"),
      ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
    )]
    fn $digits() {
      type Digits = typenum::consts::$digits;
      assert_eq!(<Digits as Unsigned>::U32, $value);

      assert_class(
        &SIGNED,
        $value,
        &[stringify!($signed) $(, stringify!($wider_signed))*],
      );
      assert_class(
        &UNSIGNED,
        $value,
        &[stringify!($unsigned) $(, stringify!($wider_unsigned))*],
      );
      assert_eq!(
        TypeId::of::<<Digits as DigitCount>::Signed>(),
        TypeId::of::<$signed>()
      );
      assert_eq!(
        TypeId::of::<<Digits as DigitCount>::Unsigned>(),
        TypeId::of::<$unsigned>()
      );
      // `div_round` divides in the same storages.
      assert_eq!(
        TypeId::of::<<Digits as DivIntermediate<B0>>::Storage>(),
        TypeId::of::<$signed>()
      );
      assert_eq!(
        TypeId::of::<<Digits as DivIntermediate<B1>>::Storage>(),
        TypeId::of::<$unsigned>()
      );

      // `FitsDigits` is implemented for every listed storage.
      assert_fits::<Digits, $signed>();
      $(assert_fits::<Digits, $wider_signed>();)*
      assert_fits::<Digits, $unsigned>();
      $(assert_fits::<Digits, $wider_unsigned>();)*
    }
  )+};
}

digit_table!(test_digits);

/// Only a `u512` holds 154 digits, so the table has no row for them.
#[test]
#[cfg_attr(
  all(miri, target_pointer_width = "32"),
  ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
)]
fn U154() {
  assert_eq!(<U154 as Unsigned>::U32, 154);
  assert_class(&UNSIGNED, 154, &["u512"]);
  let max_coeff = bigint::pow10(154) - 1u8;
  assert!(
    SIGNED
      .iter()
      .all(|&(_, bits)| max_coeff > (BigInt::from(1u8) << bits) - 1u8),
    "no signed storage holds the digits"
  );
  assert_fits::<U154, u512>();
  // Only a division of two unsigned storages has an intermediate of 154 digits.
  assert_eq!(
    TypeId::of::<<U154 as DivIntermediate<B1>>::Storage>(),
    TypeId::of::<u512>()
  );
}

fn digits<I, F, R>() -> (u32, u32, u32)
where
  (I, F): Layout<R>,
{
  (
    <(I, F) as LayoutDigits<R>>::INTEGRAL,
    <(I, F) as LayoutDigits<R>>::FRACTION,
    <(I, F) as LayoutDigits<R>>::DIGITS,
  )
}

#[test]
fn layout_digit_counts() {
  assert_eq!(digits::<U1, U1, i8>(), (1, 1, 2));
  assert_eq!(digits::<U0, U2, i8>(), (0, 2, 2));
  assert_eq!(digits::<U3, U0, i16>(), (3, 0, 3));
  assert_eq!(digits::<U10, U8, i64>(), (10, 8, 18));
  assert_eq!(digits::<U10, U8, i128>(), (10, 8, 18));
  assert_eq!(digits::<U18, U20, i128>(), (18, 20, 38));
  assert_eq!(digits::<U19, U19, i128>(), (19, 19, 38));
  assert_eq!(digits::<U0, U38, i128>(), (0, 38, 38));
  assert_eq!(digits::<U38, U0, i128>(), (38, 0, 38));

  assert_eq!(digits::<U1, U1, u8>(), (1, 1, 2));
  assert_eq!(digits::<U10, U9, u64>(), (10, 9, 19));
  assert_eq!(digits::<U19, U0, u64>(), (19, 0, 19));
  assert_eq!(digits::<U10, U9, u128>(), (10, 9, 19));
  assert_eq!(digits::<U38, U0, u128>(), (38, 0, 38));

  assert_eq!(digits::<U39, U18, i192>(), (39, 18, 57));
  assert_eq!(digits::<U0, U57, u192>(), (0, 57, 57));
  assert_eq!(digits::<U58, U18, i256>(), (58, 18, 76));
  assert_eq!(digits::<U77, U0, u256>(), (77, 0, 77));
  assert_eq!(digits::<U78, U18, i320>(), (78, 18, 96));
  assert_eq!(digits::<U96, U0, u320>(), (96, 0, 96));
  assert_eq!(digits::<U0, U96, i320>(), (0, 96, 96));
  assert_eq!(digits::<U79, U18, i384>(), (79, 18, 97));
  assert_eq!(digits::<U115, U0, u384>(), (115, 0, 115));
  assert_eq!(digits::<U0, U115, i384>(), (0, 115, 115));
  assert_eq!(digits::<U98, U18, u448>(), (98, 18, 116));
  assert_eq!(digits::<U134, U0, i448>(), (134, 0, 134));
  assert_eq!(digits::<U117, U18, i512>(), (117, 18, 135));
  assert_eq!(digits::<U0, U153, i512>(), (0, 153, 153));
  assert_eq!(digits::<U154, U0, u512>(), (154, 0, 154));
  assert_eq!(digits::<U0, U154, u512>(), (0, 154, 154));
  // Few digits in a wide storage.
  assert_eq!(digits::<U1, U1, u320>(), (1, 1, 2));
  assert_eq!(digits::<U0, U1, i192>(), (0, 1, 1));
  assert_eq!(digits::<U1, U1, u512>(), (1, 1, 2));
}

#[test]
fn storage_signs() {
  fn signed<R: StorageCore>() -> bool {
    R::SIGNED
  }

  assert!(signed::<i8>() && signed::<i16>() && signed::<i32>());
  assert!(signed::<i64>() && signed::<i128>());
  assert!(signed::<i192>() && signed::<i256>() && signed::<i320>());
  assert!(signed::<i384>() && signed::<i448>() && signed::<i512>());
  assert!(!signed::<u8>() && !signed::<u16>() && !signed::<u32>());
  assert!(!signed::<u64>() && !signed::<u128>());
  assert!(!signed::<u192>() && !signed::<u256>() && !signed::<u320>());
  assert!(!signed::<u384>() && !signed::<u448>() && !signed::<u512>());
  assert_eq!(
    TypeId::of::<<i64 as StorageCore>::IsUnsigned>(),
    TypeId::of::<B0>()
  );
  assert_eq!(
    TypeId::of::<<u64 as StorageCore>::IsUnsigned>(),
    TypeId::of::<B1>()
  );
  assert_eq!(
    TypeId::of::<<i256 as StorageCore>::IsUnsigned>(),
    TypeId::of::<B0>()
  );
  assert_eq!(
    TypeId::of::<<u256 as StorageCore>::IsUnsigned>(),
    TypeId::of::<B1>()
  );
  assert_eq!(
    TypeId::of::<<i512 as StorageCore>::IsUnsigned>(),
    TypeId::of::<B0>()
  );
  assert_eq!(
    TypeId::of::<<u512 as StorageCore>::IsUnsigned>(),
    TypeId::of::<B1>()
  );
}

#[test]
fn pick_by_sign() {
  fn picked<B: Pick<D>, D>() -> TypeId
  where
    B::Storage: 'static,
  {
    TypeId::of::<B::Storage>()
  }

  assert_eq!(picked::<B0, U2>(), TypeId::of::<i8>());
  assert_eq!(picked::<B1, U2>(), TypeId::of::<u8>());
  // 19 digits need an `i128` but fit a `u64`.
  assert_eq!(picked::<B0, U19>(), TypeId::of::<i128>());
  assert_eq!(picked::<B1, U19>(), TypeId::of::<u64>());
  assert_eq!(picked::<B0, U38>(), TypeId::of::<i128>());
  assert_eq!(picked::<B1, U38>(), TypeId::of::<u128>());
  assert_eq!(picked::<B0, U39>(), TypeId::of::<i192>());
  assert_eq!(picked::<B1, U39>(), TypeId::of::<u192>());
  assert_eq!(picked::<B0, U57>(), TypeId::of::<i192>());
  assert_eq!(picked::<B1, U57>(), TypeId::of::<u192>());
  assert_eq!(picked::<B0, U58>(), TypeId::of::<i256>());
  assert_eq!(picked::<B1, U58>(), TypeId::of::<u256>());
  assert_eq!(picked::<B0, U76>(), TypeId::of::<i256>());
  // 77 digits need an `i320` but fit a `u256`.
  assert_eq!(picked::<B0, U77>(), TypeId::of::<i320>());
  assert_eq!(picked::<B1, U77>(), TypeId::of::<u256>());
  assert_eq!(picked::<B1, U78>(), TypeId::of::<u320>());
  assert_eq!(picked::<B0, U96>(), TypeId::of::<i320>());
  assert_eq!(picked::<B1, U96>(), TypeId::of::<u320>());
  assert_eq!(picked::<B0, U97>(), TypeId::of::<i384>());
  assert_eq!(picked::<B1, U97>(), TypeId::of::<u384>());
  assert_eq!(picked::<B0, U115>(), TypeId::of::<i384>());
  assert_eq!(picked::<B1, U115>(), TypeId::of::<u384>());
  assert_eq!(picked::<B0, U116>(), TypeId::of::<i448>());
  assert_eq!(picked::<B1, U116>(), TypeId::of::<u448>());
  assert_eq!(picked::<B0, U134>(), TypeId::of::<i448>());
  assert_eq!(picked::<B1, U134>(), TypeId::of::<u448>());
  assert_eq!(picked::<B0, U135>(), TypeId::of::<i512>());
  assert_eq!(picked::<B1, U135>(), TypeId::of::<u512>());
  assert_eq!(picked::<B0, U153>(), TypeId::of::<i512>());
  assert_eq!(picked::<B1, U153>(), TypeId::of::<u512>());
}

#[test]
fn widen_repr_pairs() {
  fn assert_widens<Source: WidenRepr<Target>, Target>() {}

  macro_rules! assert_pairs {
    ($($source:ident),+ => $targets:tt) => {
      $(assert_pairs!(@source $source => $targets);)+
    };
    (@source $source:ident => [$($target:ident),+]) => {
      $(assert_widens::<$source, $target>();)+
    };
  }

  // Every unsigned storage converts to every storage, and every signed one to every signed one;
  // the remaining pairs fail to compile.
  assert_pairs!(
    u8, u16, u32, u64, u128, u192, u256, u320, u384, u448, u512 => [
      u8, u16, u32, u64, u128, u192, u256, u320, u384, u448, u512,
      i8, i16, i32, i64, i128, i192, i256, i320, i384, i448, i512
    ]
  );
  assert_pairs!(
    i8, i16, i32, i64, i128, i192, i256, i320, i384, i448, i512 => [
      i8, i16, i32, i64, i128, i192, i256, i320, i384, i448, i512
    ]
  );
}

/// A storage whose values the conversion tests compare as `num-bigint` integers.
trait Value: InlineCore + Debug + PartialEq {
  fn to_big(self) -> BigInt;

  /// The value of this storage equal to `value`, if the storage holds it.
  fn from_big(value: &BigInt) -> Option<Self>;
}

macro_rules! native_value {
  ($($repr:ident),+) => {$(
    impl Value for $repr {
      fn to_big(self) -> BigInt {
        BigInt::from(self)
      }

      fn from_big(value: &BigInt) -> Option<Self> {
        <$repr>::try_from(value).ok()
      }
    }
  )+};
}

native_value!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128);

macro_rules! wide_value {
  ($($repr:ident: $limbs:literal),+) => {$(
    impl Value for $repr {
      fn to_big(self) -> BigInt {
        bigint::from_limbs(&self.to_limbs(), Self::SIGNED)
      }

      fn from_big(value: &BigInt) -> Option<Self> {
        bigint::to_limbs::<$limbs>(value, Self::SIGNED).map(Self::from_limbs)
      }
    }
  )+};
}

wide_value!(
  i192: 3, i256: 4, i320: 5, i384: 6, i448: 7, i512: 8,
  u192: 3, u256: 4, u320: 5, u384: 6, u448: 7, u512: 8
);

/// Integers at and around the limits of every storage.
fn boundaries() -> Vec<BigInt> {
  let mut values = vec![BigInt::ZERO, BigInt::from(1u8), BigInt::from(-1)];
  for bits in [8, 16, 32, 64, 128, 192, 256, 320, 384, 448, 512] {
    for limit in [
      bigint::min(bits, true),
      bigint::max(bits, true),
      bigint::max(bits, false),
    ] {
      values.extend([&limit - 1u8, limit.clone(), &limit + 1u8]);
    }
  }
  bigint::thin(values)
}

/// The boundaries that `Source` holds, with their values.
fn samples<Source: Value>() -> Vec<(Source, BigInt)> {
  let mut samples = Vec::new();
  for value in boundaries() {
    if let Some(source) = Source::from_big(&value) {
      assert_eq!(source.to_big(), value);
      samples.push((source, value));
    }
  }
  samples
}

/// Converts each sample to `Target`: the checked conversion succeeds exactly when `Target` holds
/// the value, and then both conversions keep it.
fn assert_conversions<Source: Value, Target: Value>(samples: &[(Source, BigInt)]) {
  for (source, value) in samples {
    let expected = Target::from_big(value);
    assert_eq!(
      source.checked_convert::<Target>(Token::new()),
      expected,
      "{value}"
    );
    if let Some(target) = expected {
      assert_eq!(source.convert::<Target>(Token::new()), target, "{value}");
    }
  }
}

#[test]
#[cfg_attr(
  all(miri, target_pointer_width = "32"),
  ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
)]
fn conversions_between_every_pair() {
  macro_rules! assert_pairs {
    ($($source:ident),+ => $targets:tt) => {
      $(assert_pairs!(@source $source => $targets);)+
    };
    (@source $source:ident => [$($target:ident),+]) => {
      let source_samples = samples::<$source>();
      $(assert_conversions::<$source, $target>(&source_samples);)+
    };
  }

  assert_pairs!(
    i8, i16, i32, i64, i128, i192, i256, i320, i384, i448, i512,
    u8, u16, u32, u64, u128, u192, u256, u320, u384, u448, u512 => [
      i8, i16, i32, i64, i128, i192, i256, i320, i384, i448, i512,
      u8, u16, u32, u64, u128, u192, u256, u320, u384, u448, u512
    ]
  );
}

/// Checks, against `num-bigint`, the rows `read_limbs` and `checked_read_limbs` of `R`, a storage
/// of `bits` bits, called with `M` limbs of either sign, at zero, one, minus one, the limits of `R`
/// and one beyond each, wherever the limbs hold the value: the checked row gives the value exactly
/// where `R` holds it, and the other row then gives it too. Counts each value that the checked row
/// refuses in `refused`, by the sign of `R` and the sign of the limbs.
fn check_read_limbs<R: Value, const M: usize>(bits: u32, refused: &mut [[usize; 2]; 2]) {
  let (min, max) = (bigint::min(bits, R::SIGNED), bigint::max(bits, R::SIGNED));
  let values = [
    BigInt::ZERO,
    BigInt::from(1u8),
    BigInt::from(-1),
    &min - 1u8,
    min,
    max.clone(),
    max + 1u8,
  ];
  for signed in [false, true] {
    for value in &values {
      let Some(limbs) = bigint::to_limbs::<M>(value, signed) else {
        continue;
      };
      let expected = R::from_big(value);
      assert_eq!(
        R::checked_read_limbs(&limbs, signed, Token::new()),
        expected,
        "{value} in {M} limbs, signed {signed}"
      );
      match expected {
        Some(expected) => assert_eq!(
          R::read_limbs(&limbs, signed, Token::new()),
          expected,
          "{value} in {M} limbs, signed {signed}"
        ),
        None => refused[usize::from(R::SIGNED)][usize::from(signed)] += 1,
      }
    }
  }
}

/// Checks [`check_read_limbs`] for `R` with 1, 2, 3 and 8 limbs.
#[inline(never)]
fn read_limbs_of<R: Value>(bits: u32, refused: &mut [[usize; 2]; 2]) {
  check_read_limbs::<R, 1>(bits, refused);
  check_read_limbs::<R, 2>(bits, refused);
  check_read_limbs::<R, 3>(bits, refused);
  check_read_limbs::<R, 8>(bits, refused);
}

/// The rows that read the limbs of a wide source give every value that the target holds, from
/// limbs of either sign and of any count, and the checked row refuses the others: each pair of the
/// signs of the limbs and of the storage has a value that it refuses.
#[test]
#[cfg_attr(
  all(miri, target_pointer_width = "32"),
  ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
)]
fn rows_read_limbs_of_either_sign() {
  let mut refused = [[0; 2]; 2];
  read_limbs_of::<i8>(8, &mut refused);
  read_limbs_of::<i16>(16, &mut refused);
  read_limbs_of::<i32>(32, &mut refused);
  read_limbs_of::<i64>(64, &mut refused);
  read_limbs_of::<i128>(128, &mut refused);
  read_limbs_of::<i192>(192, &mut refused);
  read_limbs_of::<i256>(256, &mut refused);
  read_limbs_of::<i320>(320, &mut refused);
  read_limbs_of::<i384>(384, &mut refused);
  read_limbs_of::<i448>(448, &mut refused);
  read_limbs_of::<i512>(512, &mut refused);
  read_limbs_of::<u8>(8, &mut refused);
  read_limbs_of::<u16>(16, &mut refused);
  read_limbs_of::<u32>(32, &mut refused);
  read_limbs_of::<u64>(64, &mut refused);
  read_limbs_of::<u128>(128, &mut refused);
  read_limbs_of::<u192>(192, &mut refused);
  read_limbs_of::<u256>(256, &mut refused);
  read_limbs_of::<u320>(320, &mut refused);
  read_limbs_of::<u384>(384, &mut refused);
  read_limbs_of::<u448>(448, &mut refused);
  read_limbs_of::<u512>(512, &mut refused);
  assert!(
    refused.iter().flatten().all(|&count| count > 0),
    "refused by the sign of the storage and of the limbs: {refused:?}"
  );
}

/// Checks, against `num-bigint`, that the digit buffer of `R` has as many bytes as its widest
/// magnitude has digits, `|MIN|` for a signed storage of `bits` bits and `MAX` for an unsigned one,
/// that the row writes them from the first byte, and that the text buffer has two bytes more.
#[inline(never)]
fn check_buffer_lengths<R: Value>(bits: u32) {
  let widest = if R::SIGNED {
    bigint::min(bits, true)
  } else {
    bigint::max(bits, false)
  };
  let digits = widest.magnitude().to_string();
  assert_eq!(<R::DigitLen as Unsigned>::USIZE, digits.len(), "{widest}");
  assert_eq!(
    <R::TextLen as Unsigned>::USIZE,
    digits.len() + 2,
    "{widest}"
  );
  let value = R::from_big(&widest).expect("the storage holds its widest value");
  let mut buf = DigitBuf::<R>::default();
  assert_eq!(value.write_digits(&mut buf, Token::new()), 0, "{widest}");
  assert_eq!(&buf[..], digits.as_bytes());
}

#[test]
#[cfg_attr(
  all(miri, target_pointer_width = "32"),
  ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
)]
fn buffer_lengths_of_every_storage() {
  check_buffer_lengths::<i8>(8);
  check_buffer_lengths::<i16>(16);
  check_buffer_lengths::<i32>(32);
  check_buffer_lengths::<i64>(64);
  check_buffer_lengths::<i128>(128);
  check_buffer_lengths::<i192>(192);
  check_buffer_lengths::<i256>(256);
  check_buffer_lengths::<i320>(320);
  check_buffer_lengths::<i384>(384);
  check_buffer_lengths::<i448>(448);
  check_buffer_lengths::<i512>(512);
  check_buffer_lengths::<u8>(8);
  check_buffer_lengths::<u16>(16);
  check_buffer_lengths::<u32>(32);
  check_buffer_lengths::<u64>(64);
  check_buffer_lengths::<u128>(128);
  check_buffer_lengths::<u192>(192);
  check_buffer_lengths::<u256>(256);
  check_buffer_lengths::<u320>(320);
  check_buffer_lengths::<u384>(384);
  check_buffer_lengths::<u448>(448);
  check_buffer_lengths::<u512>(512);
}

/// Checks, against `num-bigint`, that the float scratch of `R`, a storage of `bits` bits whose
/// coefficient takes `limbs` limbs, is the fewest limbs, at least `limbs`, that hold
/// `bits(5^d) + 55` bits, for the `d` digits that `R` holds.
#[inline(never)]
fn check_float_length<R: Value>(bits: u32, limbs: usize) {
  let digits = bigint::max_exp(&bigint::max(bits, R::SIGNED));
  let power_bits = BigInt::from(5u8).pow(digits).bits();
  let fewest = (limbs..)
    .find(|&count| power_bits + 55 <= 64 * count as u64)
    .expect("a count of limbs");
  assert_eq!(<R::FloatLen as Unsigned>::USIZE, fewest, "{digits} digits");
}

#[test]
#[cfg_attr(
  all(miri, target_pointer_width = "32"),
  ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
)]
fn float_lengths_of_every_storage() {
  check_float_length::<i8>(8, 2);
  check_float_length::<i16>(16, 2);
  check_float_length::<i32>(32, 2);
  check_float_length::<i64>(64, 2);
  check_float_length::<i128>(128, 2);
  check_float_length::<i192>(192, 3);
  check_float_length::<i256>(256, 4);
  check_float_length::<i320>(320, 5);
  check_float_length::<i384>(384, 6);
  check_float_length::<i448>(448, 7);
  check_float_length::<i512>(512, 8);
  check_float_length::<u8>(8, 2);
  check_float_length::<u16>(16, 2);
  check_float_length::<u32>(32, 2);
  check_float_length::<u64>(64, 2);
  check_float_length::<u128>(128, 2);
  check_float_length::<u192>(192, 3);
  check_float_length::<u256>(256, 4);
  check_float_length::<u320>(320, 5);
  check_float_length::<u384>(384, 6);
  check_float_length::<u448>(448, 7);
  check_float_length::<u512>(512, 8);
}
