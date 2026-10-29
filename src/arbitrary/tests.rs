//! Tests of the `arbitrary` impls: values read from bytes, the bytes they read against
//! `size_hint`, and short and empty data.

use std::{string::String, vec, vec::Vec};

use ::arbitrary::{Arbitrary, Unstructured};
use num_bigint::BigInt;
use typenum::{U0, U1};

use crate::{
  Decimal, RoundingMode,
  layout::{Layout, LayoutDigits, Storage},
  sample::{
    MODES,
    tests::{SAMPLES, SplitMix, big, check_values, for_each_type},
  },
};

/// Bytes that take the longest path for `digits` digits, then bytes that stay unread: the
/// digit-wise branch, all the digits, the first `5` and the others `7`, the last of them zeroed,
/// and a minus sign, which only a signed storage reads.
fn longest(digits: u32) -> Vec<u8> {
  let several = digits >= 2;
  let mut bytes = vec![1];
  if several {
    bytes.push((digits - 1) as u8);
  }
  bytes.push(4);
  bytes.extend(core::iter::repeat_n(7, digits as usize - 1));
  bytes.push(0);
  if several {
    bytes.push(1);
  }
  bytes.extend([1, 0xa5, 0x5a, 0xff]);
  bytes
}

/// The value that [`longest`] gives for `digits` digits.
fn longest_value(digits: u32, signed: bool) -> BigInt {
  let text = if digits >= 2 {
    let mut text = String::from("5");
    text.extend(core::iter::repeat_n('7', digits as usize - 2));
    text.push('0');
    text
  } else {
    String::from("4")
  };
  let magnitude: BigInt = text.parse().expect("digits");
  if signed { -magnitude } else { magnitude }
}

/// Reads a decimal from `bytes` and returns it with the number of bytes it read.
fn read<I, F, R>(bytes: &[u8]) -> (Decimal<I, F, R>, usize)
where
  R: Storage,
  (I, F): Layout<R>,
{
  let mut data = Unstructured::new(bytes);
  let value = <Decimal<I, F, R> as Arbitrary<'_>>::arbitrary(&mut data).expect("reading succeeds");
  (value, bytes.len() - data.len())
}

/// Checks the impl for `Decimal<I, F, R>`: the bounds of `size_hint`, which the shortest and the
/// longest paths read exactly; every prefix of the longest path and empty data; and values read
/// from random bytes, each within the bounds.
fn check_bytes<I, F, R>(
  name: &str,
  repr: fn(Decimal<I, F, R>) -> R,
  from_repr: fn(R) -> Option<Decimal<I, F, R>>,
) where
  R: Storage,
  (I, F): Layout<R>,
{
  let digits = <(I, F) as LayoutDigits<R>>::DIGITS;
  let several = usize::from(digits >= 2);
  let (lower, upper) = <Decimal<I, F, R> as Arbitrary<'_>>::size_hint(0);
  let upper = upper.expect("an upper bound");
  assert_eq!(lower, 2, "{name}");
  assert_eq!(
    upper,
    digits as usize + 2 + 2 * several + usize::from(R::SIGNED),
    "{name}"
  );

  let pattern = longest(digits);
  let (value, count) = read::<I, F, R>(&pattern);
  assert_eq!(
    count, upper,
    "{name}: the longest path reads the upper bound"
  );
  assert_eq!(big(repr(value)), longest_value(digits, R::SIGNED), "{name}");

  let (value, count) = read::<I, F, R>(&[0, 0, 0xff, 0xff]);
  assert_eq!(
    count, lower,
    "{name}: the edge value zero reads the lower bound"
  );
  assert_eq!(big(repr(value)), BigInt::ZERO, "{name}");

  for len in 0..=upper {
    let (value, count) = read::<I, F, R>(&pattern[..len]);
    assert_eq!(from_repr(repr(value)), Some(value), "{name}: {len} bytes");
    assert!(count <= len, "{name}: {len} bytes");
  }
  let (value, count) = read::<I, F, R>(&[]);
  assert_eq!((big(repr(value)), count), (BigInt::ZERO, 0), "{name}");

  // Enough bytes that every value reads all it needs.
  let mut source = SplitMix(0xa4b1_0001);
  let bytes: Vec<u8> = (0..SAMPLES * upper)
    .map(|_| source.next_u64() as u8)
    .collect();
  let mut data = Unstructured::new(&bytes);
  let values: Vec<Decimal<I, F, R>> = (0..SAMPLES)
    .map(|_| {
      let before = data.len();
      let value =
        <Decimal<I, F, R> as Arbitrary<'_>>::arbitrary(&mut data).expect("reading succeeds");
      let count = before - data.len();
      assert!(
        lower <= count && count <= upper,
        "{name}: {count} bytes for {value:?}"
      );
      value
    })
    .collect();
  check_values(name, &values, repr, from_repr);
}

macro_rules! check_type {
  ($t:ty) => {
    check_bytes(stringify!($t), <$t>::repr, <$t>::from_repr)
  };
}

#[test]
fn decimals_from_bytes() {
  for_each_type!(check_type);
}

/// With a single digit, the number of digits and the number of trailing zeros read no byte.
#[test]
fn single_digits_from_bytes() {
  for (bytes, signed, unsigned) in [
    (&[1, 4, 0, 1, 0xff][..], -4, 4),
    (&[1, 9, 2, 0, 0xff][..], 9, 9),
    (&[0, 3, 1, 0xff][..], -9, 9),
  ] {
    let (value, _) = read::<U1, U0, i8>(bytes);
    assert_eq!(value.repr(), signed, "{bytes:?}");
    let (value, _) = read::<U1, U0, u8>(bytes);
    assert_eq!(value.repr(), unsigned, "{bytes:?}");
  }
  check_size_of_single_digits::<i8>(4);
  check_size_of_single_digits::<u8>(3);
}

fn check_size_of_single_digits<R>(upper: usize)
where
  R: Storage,
  (U1, U0): Layout<R>,
{
  assert_eq!(
    <Decimal<U1, U0, R> as Arbitrary<'_>>::size_hint(0),
    (2, Some(upper))
  );
  let (_, count) = read::<U1, U0, R>(&longest(1));
  assert_eq!(count, upper);
}

#[test]
fn modes_from_bytes() {
  assert_eq!(<RoundingMode as Arbitrary<'_>>::size_hint(0), (1, Some(1)));
  for (index, &mode) in MODES.iter().enumerate() {
    let bytes = [index as u8, 0xff];
    let mut data = Unstructured::new(&bytes);
    assert_eq!(
      <RoundingMode as Arbitrary<'_>>::arbitrary(&mut data).unwrap(),
      mode
    );
    assert_eq!(data.len(), 1, "one byte for {mode:?}");
  }
  let mut empty = Unstructured::new(&[]);
  assert_eq!(
    <RoundingMode as Arbitrary<'_>>::arbitrary(&mut empty).unwrap(),
    RoundingMode::Down
  );
  let mut source = SplitMix(0xa4b1_0002);
  let bytes: Vec<u8> = (0..SAMPLES).map(|_| source.next_u64() as u8).collect();
  let mut data = Unstructured::new(&bytes);
  let modes: Vec<RoundingMode> = (0..SAMPLES)
    .map(|_| <RoundingMode as Arbitrary<'_>>::arbitrary(&mut data).unwrap())
    .collect();
  assert!(MODES.iter().all(|mode| modes.contains(mode)));
}
