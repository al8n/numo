use core::{
  fmt::{Debug, Display},
  hash::{BuildHasher, Hash, Hasher},
};
use std::{collections::hash_map::RandomState, format, string::ToString, vec, vec::Vec};

use ::proptest::{prelude::*, test_runner::RngSeed};
use num_bigint::{BigInt, Sign};
use typenum::{
  U1, U2, U7, U8, U9, U15, U16, U17, U24, U31, U32, U33, U63, U64, U65, U100, U127, U128, U129,
  U160, U192, U200, U255, U256, U257, U511, U512, U1023, U4096, U8192, U16384, U32768, U65536,
  U131072, U4294967295, Unsigned,
};

use crate::{
  ArrayInteger, BitCount, FixedInteger, Int, Limbs, TryNewError, Uint, bigint, i7, i192, i256,
  i320, i384, i448, i512, i768, i1024, i2048, i4096, int::DIGIT_BUF_LEN, u7, u192, u256, u320,
  u384, u448, u512, u768, u1024, u2048, u4096,
};

/// Records the calls that a `Hash` implementation makes.
#[derive(Default)]
struct Recorder {
  calls: Vec<Call>,
}

#[derive(Debug, PartialEq)]
enum Call {
  U64(u64),
  Usize(usize),
  Bytes(Vec<u8>),
}

impl Hasher for Recorder {
  fn finish(&self) -> u64 {
    0
  }

  fn write(&mut self, bytes: &[u8]) {
    self.calls.push(Call::Bytes(bytes.to_vec()));
  }

  fn write_u64(&mut self, value: u64) {
    self.calls.push(Call::U64(value));
  }

  fn write_usize(&mut self, value: usize) {
    self.calls.push(Call::Usize(value));
  }
}

/// Asserts that `$value` formats as `$reference` with each format string.
macro_rules! assert_same_format {
  ($value:expr, $reference:expr, $($spec:tt),+ $(,)?) => {$(
    assert_eq!(format!($spec, $value), format!($spec, $reference), "format {}", $spec);
  )+};
}

fn recorded<T: Hash>(value: &T) -> Vec<Call> {
  let mut recorder = Recorder::default();
  value.hash(&mut recorder);
  recorder.calls
}

/// The calls that `Hash` makes for `value`, an integer of `bits` bits: one `write_u64` for each of
/// its lowest `ceil(bits / 64)` 64-bit words in two's complement, least significant first.
fn word_calls(value: &BigInt, bits: u32) -> Vec<Call> {
  let count = bits.div_ceil(64) as usize;
  // The shortest two's complement, extended by copies of the sign, or cut to the words when an
  // unsigned value needs a zero byte above them.
  let mut bytes = value.to_signed_bytes_le();
  bytes.resize(
    8 * count,
    if value.sign() == Sign::Minus { 0xff } else { 0 },
  );
  let (words, _) = bytes.as_chunks::<8>();
  words
    .iter()
    .map(|&word| Call::U64(u64::from_le_bytes(word)))
    .collect()
}

#[test]
fn trait_surface_and_layout() {
  fn assert_traits<
    T: Copy + Eq + Ord + Hash + Debug + Display + Default + Send + Sync + Unpin + 'static,
  >() {
  }

  assert_traits::<u192>();
  assert_traits::<u256>();
  assert_traits::<u320>();
  assert_traits::<u384>();
  assert_traits::<u448>();
  assert_traits::<u512>();
  assert_traits::<u768>();
  assert_traits::<u1024>();
  assert_traits::<u2048>();
  assert_traits::<u4096>();
  assert_traits::<i192>();
  assert_traits::<i256>();
  assert_traits::<i320>();
  assert_traits::<i384>();
  assert_traits::<i448>();
  assert_traits::<i512>();
  assert_traits::<i768>();
  assert_traits::<i1024>();
  assert_traits::<i2048>();
  assert_traits::<i4096>();
  // Widths that leave bits or limbs unused.
  assert_traits::<Uint<U200, [u64; 4]>>();
  assert_traits::<Uint<U64, [u64; 2]>>();
  assert_traits::<Int<U7, [u64; 1]>>();
  assert_traits::<Int<U100, [u64; 3]>>();

  assert_eq!(size_of::<u192>(), 24);
  assert_eq!(size_of::<i256>(), 32);
  assert_eq!(size_of::<u320>(), 40);
  assert_eq!(size_of::<i384>(), 48);
  assert_eq!(size_of::<u448>(), 56);
  assert_eq!(size_of::<i512>(), 64);
  assert_eq!(size_of::<u768>(), 96);
  assert_eq!(size_of::<i768>(), 96);
  assert_eq!(size_of::<u1024>(), 128);
  assert_eq!(size_of::<i1024>(), 128);
  assert_eq!(size_of::<u2048>(), 256);
  assert_eq!(size_of::<i2048>(), 256);
  assert_eq!(size_of::<u4096>(), 512);
  assert_eq!(size_of::<i4096>(), 512);
  assert_eq!(size_of::<Uint<U200, [u64; 4]>>(), 32);
  assert_eq!(size_of::<Int<U100, [u64; 3]>>(), 24);
  // The alignment of `u64`, 8 bytes on 64-bit targets.
  assert_eq!(align_of::<i320>(), align_of::<u64>());
  assert_eq!(align_of::<u512>(), align_of::<u64>());
  assert_eq!(align_of::<u4096>(), align_of::<u64>());
  assert_eq!(align_of::<i4096>(), align_of::<u64>());
  assert_eq!(size_of::<Option<u256>>(), size_of::<Option<[u64; 4]>>());

  // The other limb types, whose arrays give the size and the alignment: 1 for bytes, and 2 and 4
  // bytes for `u16` and `u32` on the usual targets.
  assert_traits::<Uint<U24, [u8; 3]>>();
  assert_traits::<Int<U24, [u8; 3]>>();
  assert_traits::<Uint<U100, [u16; 7]>>();
  assert_traits::<Int<U100, [u16; 7]>>();
  assert_traits::<Uint<U100, [u32; 4]>>();
  assert_traits::<Int<U100, [u32; 4]>>();
  assert_eq!(size_of::<Uint<U24, [u8; 3]>>(), 3);
  assert_eq!(size_of::<Int<U24, [u8; 3]>>(), 3);
  assert_eq!(size_of::<Uint<U100, [u16; 7]>>(), 14);
  assert_eq!(size_of::<Int<U100, [u16; 7]>>(), 14);
  assert_eq!(size_of::<Uint<U100, [u32; 4]>>(), 16);
  assert_eq!(size_of::<Int<U100, [u32; 4]>>(), 16);
  assert_eq!(align_of::<Uint<U24, [u8; 3]>>(), 1);
  assert_eq!(align_of::<Int<U24, [u8; 3]>>(), 1);
  assert_eq!(align_of::<Uint<U100, [u16; 7]>>(), align_of::<u16>());
  assert_eq!(align_of::<Int<U100, [u16; 7]>>(), align_of::<u16>());
  assert_eq!(align_of::<Uint<U100, [u32; 4]>>(), align_of::<u32>());
  assert_eq!(align_of::<Int<U100, [u32; 4]>>(), align_of::<u32>());
  assert_eq!(
    size_of::<Option<Uint<U24, [u8; 3]>>>(),
    size_of::<Option<[u8; 3]>>()
  );

  // 128-bit limbs, whose arrays give the size and the alignment: that of `u128` depends on the
  // target, 16 bytes on x86_64 and aarch64.
  assert_traits::<Uint<U256, [u128; 2]>>();
  assert_traits::<Int<U256, [u128; 2]>>();
  assert_traits::<Uint<U100, [u128; 1]>>();
  assert_traits::<Int<U100, [u128; 1]>>();
  assert_eq!(size_of::<Uint<U256, [u128; 2]>>(), 32);
  assert_eq!(size_of::<Int<U256, [u128; 2]>>(), 32);
  assert_eq!(size_of::<Uint<U100, [u128; 1]>>(), 16);
  assert_eq!(size_of::<Int<U100, [u128; 1]>>(), 16);
  assert_eq!(align_of::<Uint<U256, [u128; 2]>>(), align_of::<u128>());
  assert_eq!(align_of::<Int<U100, [u128; 1]>>(), align_of::<u128>());

  // The native integers, which give the size and the alignment, the smallest that holds the width
  // and wider ones.
  assert_traits::<Uint<U7, u8>>();
  assert_traits::<Int<U7, i8>>();
  assert_traits::<Uint<U24, u32>>();
  assert_traits::<Int<U24, i32>>();
  assert_traits::<Uint<U100, u128>>();
  assert_traits::<Int<U127, i128>>();
  assert_traits::<Uint<U8, u8>>();
  assert_traits::<Int<U1, i8>>();
  assert_traits::<Uint<U7, u64>>();
  assert_eq!(size_of::<Uint<U7, u8>>(), 1);
  assert_eq!(size_of::<Int<U7, i8>>(), 1);
  assert_eq!(size_of::<Uint<U24, u32>>(), 4);
  assert_eq!(size_of::<Int<U24, i32>>(), 4);
  assert_eq!(size_of::<Uint<U100, u128>>(), 16);
  assert_eq!(size_of::<Int<U127, i128>>(), 16);
  assert_eq!(size_of::<Uint<U7, u64>>(), 8);
  assert_eq!(align_of::<Uint<U7, u8>>(), 1);
  assert_eq!(align_of::<Int<U24, i32>>(), align_of::<i32>());
  assert_eq!(align_of::<Uint<U100, u128>>(), align_of::<u128>());
  assert_eq!(align_of::<Int<U127, i128>>(), align_of::<i128>());
  // No value is invalid, so `None` takes a byte of its own.
  assert_eq!(size_of::<Option<Uint<U7, u8>>>(), 2);
  assert_eq!(size_of::<Option<Int<U1, i8>>>(), 2);

  // Generic code with the bound `B: BitCount` gets the traits of the default storages, though not
  // the auto traits, which no bound of the storages states.
  fn default_storages<B: BitCount>() {
    fn assert_generic<T: Copy + Eq + Ord + Hash + Debug + Display + Default>() {}
    assert_generic::<Uint<B>>();
    assert_generic::<Int<B>>();
  }
  default_storages::<U7>();
  default_storages::<U200>();
  // The default storage of the width alone.
  assert_traits::<Uint<U7>>();
  assert_traits::<Int<U200>>();
  assert_eq!(size_of::<Uint<U7>>(), 1);
  assert_eq!(size_of::<Int<U24>>(), 4);
  assert_eq!(size_of::<Uint<U200>>(), 32);
}

macro_rules! integer_tests {
  ($($module:ident: $name:ident, $signed:literal, $limbs:literal);+ $(;)?) => {$(
    mod $module {
      use super::*;

      type T = $name;

      fn big(value: T) -> BigInt {
        bigint::from_limbs(&value.to_limbs(), $signed)
      }

      /// Limb patterns at zero, the limits, the limb boundaries and the sign bit.
      fn patterns() -> Vec<[u64; $limbs]> {
        let mut patterns = vec![[0; $limbs], [u64::MAX; $limbs]];
        for k in 0..$limbs {
          for limb in [1, u64::MAX, 1 << 63, (1 << 63) - 1] {
            let mut pattern = [0; $limbs];
            pattern[k] = limb;
            patterns.push(pattern);
            let mut pattern = [u64::MAX; $limbs];
            pattern[k] = limb;
            patterns.push(pattern);
          }
        }
        bigint::thin(patterns)
      }

      #[test]
      fn limbs_round_trip() {
        assert_eq!(T::ZERO.to_limbs(), [0; $limbs]);
        assert_eq!(T::ZERO, T::from_limbs([0; $limbs]));
        for pattern in patterns() {
          assert_eq!(T::from_limbs(pattern).to_limbs(), pattern);
        }
      }

      #[test]
      fn equality_and_order_are_numeric() {
        let values: Vec<(T, BigInt)> = patterns()
          .into_iter()
          .map(|pattern| (T::from_limbs(pattern), bigint::from_limbs(&pattern, $signed)))
          .collect();
        for &(a, ref big_a) in &values {
          for &(b, ref big_b) in &values {
            assert_eq!(a == b, big_a == big_b);
            assert_eq!(a.cmp(&b), big_a.cmp(big_b), "{a} and {b}");
            assert_eq!(a.partial_cmp(&b), Some(big_a.cmp(big_b)));
          }
        }
        // The array order compares the least significant limb first, so it puts 1 above 2^64.
        let mut one = [0; $limbs];
        one[0] = 1;
        let mut two_pow_64 = [0; $limbs];
        two_pow_64[1] = 1;
        assert!(one > two_pow_64);
        assert!(T::from_limbs(one) < T::from_limbs(two_pow_64));

        let mut sorted: Vec<T> = values.iter().map(|&(value, _)| value).collect();
        sorted.sort();
        assert!(sorted.windows(2).all(|pair| big(pair[0]) <= big(pair[1])));
      }

      #[test]
      fn hash_writes_the_limbs() {
        let state = RandomState::new();
        for pattern in patterns() {
          let value = T::from_limbs(pattern);
          // One `write_u64` per limb, least significant first, and no length prefix.
          let expected: Vec<Call> = pattern.iter().map(|&limb| Call::U64(limb)).collect();
          assert_eq!(recorded(&value), expected);
          assert_eq!(state.hash_one(value), state.hash_one(T::from_limbs(pattern)));
        }
        // The array's own `Hash` would write a length prefix first.
        assert_eq!(recorded(&[0u64; $limbs])[0], Call::Usize($limbs));
      }

      #[test]
      fn formatting_matches_big_integers() {
        for pattern in patterns() {
          let value = T::from_limbs(pattern);
          let reference = big(value);
          assert_eq!(value.to_string(), reference.to_string());
          assert_eq!(format!("{value:?}"), reference.to_string());
          assert_same_format!(
            value,
            reference,
            "{}",
            "{:1}",
            "{:120}",
            "{:<120}",
            "{:^121}",
            "{:*>120}",
            "{:+}",
            "{:+120}",
            "{:0120}",
            "{:+0120}",
            "{:<0120}",
            "{:?}",
            "{:>120?}",
            "{:+0120?}",
            "{:160}",
            "{:^161}",
            "{:+0160}",
          );
        }
      }
    }
  )+};
}

integer_tests!(
  unsigned_192: u192, false, 3;
  unsigned_256: u256, false, 4;
  unsigned_320: u320, false, 5;
  unsigned_384: u384, false, 6;
  unsigned_448: u448, false, 7;
  unsigned_512: u512, false, 8;
  signed_192: i192, true, 3;
  signed_256: i256, true, 4;
  signed_320: i320, true, 5;
  signed_384: i384, true, 6;
  signed_448: i448, true, 7;
  signed_512: i512, true, 8;
);

#[test]
fn small_values_format_as_native_integers() {
  for value in [0i64, 1, -1, 42, -42, i64::MAX, i64::MIN] {
    let wide = i256::from_limbs(bigint::to_limbs(&BigInt::from(value), true).expect("fits"));
    for (formatted, expected) in [
      (format!("{wide}"), format!("{value}")),
      (format!("{wide:+}"), format!("{value:+}")),
      (format!("{wide:08}"), format!("{value:08}")),
      (format!("{wide:+08}"), format!("{value:+08}")),
      (format!("{wide:_^9}"), format!("{value:_^9}")),
      (format!("{wide:<6?}"), format!("{value:<6?}")),
    ] {
      assert_eq!(formatted, expected);
    }
  }
  for value in [0u64, 7, u64::MAX] {
    let wide = u192::from_limbs([value, 0, 0]);
    assert_eq!(format!("{wide:+05}"), format!("{value:+05}"));
    assert_eq!(format!("{wide:>25}"), format!("{value:>25}"));
  }
}

#[test]
fn signed_order_runs_from_min_to_max() {
  let min = i192::from_limbs([0, 0, 1 << 63]);
  let minus_one = i192::from_limbs([u64::MAX; 3]);
  let one = i192::from_limbs([1, 0, 0]);
  let max = i192::from_limbs([u64::MAX, u64::MAX, (1 << 63) - 1]);
  assert!(min < minus_one && minus_one < i192::ZERO && i192::ZERO < one && one < max);
  // As unsigned limbs, -1 is the largest value.
  assert!(minus_one.to_limbs() > max.to_limbs());
  assert_eq!(min.to_string(), format!("-{}", BigInt::from(1u8) << 191u32));
  assert_eq!(
    max.to_string(),
    ((BigInt::from(1u8) << 191u32) - 1u8).to_string()
  );

  let unsigned_max = u320::from_limbs([u64::MAX; 5]);
  assert!(u320::ZERO < unsigned_max);
  assert_eq!(unsigned_max.to_string().len(), 97);
  assert_eq!(
    i320::from_limbs([0, 0, 0, 0, 1 << 63]).to_string().len(),
    98
  );
}

#[test]
fn the_widest_values_fill_the_digit_buffer() {
  // `u512::MAX` has 155 digits, and `i512::MIN` has 154 and a sign.
  assert_eq!(
    u512::from_limbs([u64::MAX; 8]).to_string().len(),
    DIGIT_BUF_LEN
  );
  assert_eq!(
    i512::from_limbs([0, 0, 0, 0, 0, 0, 0, 1 << 63])
      .to_string()
      .len(),
    DIGIT_BUF_LEN
  );
}

#[test]
fn constants_in_const_context() {
  const LIMBS: [u64; 4] = [5, 1, 0, 0];
  const VALUE: u256 = u256::from_limbs(LIMBS);
  const BACK: [u64; 4] = VALUE.to_limbs();
  const ZERO: i320 = i320::ZERO;

  assert_eq!(BACK, LIMBS);
  assert_eq!(VALUE.to_string(), "18446744073709551621");
  assert_eq!(ZERO.to_limbs(), [0; 5]);

  const MAX: Uint<U200, [u64; 4]> = Uint::<U200, [u64; 4]>::MAX;
  const MIN: Int<U100, [u64; 2]> = Int::<U100, [u64; 2]>::MIN;
  const WIDEST: u4096 = u4096::from_limbs([u64::MAX; 64]);
  const BITS: u32 = i4096::BITS;
  const ACCEPTED: Option<Uint<U65, [u64; 2]>> = Uint::<U65, [u64; 2]>::checked_from_limbs([1, 1]);
  const REJECTED: Option<Int<U7, [u64; 1]>> = Int::<U7, [u64; 1]>::checked_from_limbs([127]);
  const QUERIES: [bool; 3] = [MIN.is_negative(), MAX.is_zero(), MIN.is_positive()];
  const MAX_LIMBS: [u64; 4] = *MAX.as_limbs();

  assert_eq!(MAX_LIMBS, [u64::MAX, u64::MAX, u64::MAX, 0xff]);
  assert_eq!(MIN.to_string(), format!("-{}", BigInt::from(1u8) << 99u32));
  assert_eq!(WIDEST, u4096::MAX);
  assert_eq!(BITS, 4096);
  assert_eq!(
    ACCEPTED.map(|value| value.to_limbs()),
    Some([1, 1]),
    "2^64 + 1 has 65 bits"
  );
  assert_eq!(REJECTED, None, "127 needs 8 bits");
  assert_eq!(QUERIES, [true, false, false]);

  // 1024 limbs, the most for which `typenum` has every number of limbs, in every limb type.
  const MAX_8: Uint<U8192, [u8; 1024]> = Uint::<U8192, [u8; 1024]>::MAX;
  const MAX_16: Uint<U16384, [u16; 1024]> = Uint::<U16384, [u16; 1024]>::MAX;
  const MAX_32: Uint<U32768, [u32; 1024]> = Uint::<U32768, [u32; 1024]>::MAX;
  const MAX_64: Uint<U65536, [u64; 1024]> = Uint::<U65536, [u64; 1024]>::MAX;
  const MIN_8: Int<U8192, [u8; 1024]> = Int::<U8192, [u8; 1024]>::MIN;
  const MIN_16: Int<U16384, [u16; 1024]> = Int::<U16384, [u16; 1024]>::MIN;
  const MIN_32: Int<U32768, [u32; 1024]> = Int::<U32768, [u32; 1024]>::MIN;
  const MIN_64: Int<U65536, [u64; 1024]> = Int::<U65536, [u64; 1024]>::MIN;
  const MINUS_ONE_8: Option<Int<U8192, [u8; 1024]>> =
    Int::<U8192, [u8; 1024]>::checked_from_limbs([u8::MAX; 1024]);
  const MINUS_ONE_16: Option<Int<U16384, [u16; 1024]>> =
    Int::<U16384, [u16; 1024]>::checked_from_limbs([u16::MAX; 1024]);
  const MINUS_ONE_32: Option<Int<U32768, [u32; 1024]>> =
    Int::<U32768, [u32; 1024]>::checked_from_limbs([u32::MAX; 1024]);
  const MINUS_ONE_64: Option<Int<U65536, [u64; 1024]>> =
    Int::<U65536, [u64; 1024]>::checked_from_limbs([u64::MAX; 1024]);
  const MAX_128: Uint<U131072, [u128; 1024]> = Uint::<U131072, [u128; 1024]>::MAX;
  const MIN_128: Int<U131072, [u128; 1024]> = Int::<U131072, [u128; 1024]>::MIN;
  const MINUS_ONE_128: Option<Int<U131072, [u128; 1024]>> =
    Int::<U131072, [u128; 1024]>::checked_from_limbs([u128::MAX; 1024]);
  const WIDE_QUERIES: [bool; 10] = [
    MAX_8.is_zero(),
    MAX_16.is_zero(),
    MAX_32.is_zero(),
    MAX_64.is_zero(),
    MAX_128.is_zero(),
    MIN_8.is_negative(),
    MIN_16.is_negative(),
    MIN_32.is_negative(),
    MIN_64.is_negative(),
    MIN_128.is_negative(),
  ];

  assert_eq!(MAX_8.to_limbs(), [u8::MAX; 1024]);
  assert_eq!(MAX_16.to_limbs(), [u16::MAX; 1024]);
  assert_eq!(MAX_32.to_limbs(), [u32::MAX; 1024]);
  assert_eq!(MAX_64.to_limbs(), [u64::MAX; 1024]);
  assert_eq!(MAX_128.to_limbs(), [u128::MAX; 1024]);
  assert_eq!(MIN_8.as_limbs()[1023], 1 << 7);
  assert_eq!(MIN_16.as_limbs()[1023], 1 << 15);
  assert_eq!(MIN_32.as_limbs()[1023], 1 << 31);
  assert_eq!(MIN_64.as_limbs()[1023], 1 << 63);
  assert_eq!(MIN_128.as_limbs()[1023], 1 << 127);
  assert!(MINUS_ONE_8.is_some_and(|value| value.is_negative()));
  assert!(MINUS_ONE_16.is_some_and(|value| value.is_negative()));
  assert!(MINUS_ONE_32.is_some_and(|value| value.is_negative()));
  assert!(MINUS_ONE_64.is_some_and(|value| value.is_negative()));
  assert!(MINUS_ONE_128.is_some_and(|value| value.is_negative()));
  assert_eq!(
    WIDE_QUERIES,
    [
      false, false, false, false, false, true, true, true, true, true
    ]
  );

  // The native integers.
  const SEVEN: Uint<U7, u8> = Uint::<U7, u8>::new(100);
  const REJECTED_NATIVE: Result<Int<U7, i8>, TryNewError> = Int::<U7, i8>::try_new(-65);
  const ACCEPTED_NATIVE: Result<Int<U7, i8>, TryNewError> = Int::<U7, i8>::try_new(-64);
  const LARGEST: u128 = Uint::<U100, u128>::MAX.value();
  const SIGN: i8 = Int::<U1, i8>::MIN.value();
  const NATIVE_BITS: [u32; 2] = [Uint::<U24, u32>::BITS, Int::<U127, i128>::BITS];
  const NATIVE_QUERIES: [bool; 4] = [
    SEVEN.is_zero(),
    Uint::<U7, u8>::ZERO.is_zero(),
    Int::<U1, i8>::MIN.is_negative(),
    Int::<U1, i8>::MAX.is_positive(),
  ];

  assert_eq!(SEVEN.value(), 100);
  assert_eq!(REJECTED_NATIVE, Err(TryNewError), "-65 needs 8 bits");
  assert_eq!(ACCEPTED_NATIVE.map(|value| value.value()), Ok(-64));
  assert_eq!(LARGEST, (1 << 100) - 1);
  assert_eq!(SIGN, -1, "one signed bit holds -1 and 0");
  assert_eq!(NATIVE_BITS, [24, 127]);
  assert_eq!(NATIVE_QUERIES, [false, true, true, false]);
}

#[test]
fn the_widest_aliases_format_every_digit() {
  // `2^4096 - 1` has 1234 digits, and `-2^4095` has 1233 and a sign.
  let max = u4096::MAX.to_string();
  assert_eq!(max, bigint::max(4096, false).to_string());
  assert_eq!(max.len(), 1234);
  let min = i4096::MIN.to_string();
  assert_eq!(min, bigint::min(4096, true).to_string());
  assert_eq!(min.len(), 1234);
  assert_eq!(
    format!("{:+}", i4096::MAX),
    format!("+{}", bigint::max(4096, true))
  );
}

#[test]
fn the_largest_width_is_u32_max() {
  // `2^32 - 1` bits in `2^26` limbs of 64 bits, in `2^25` limbs of 128 bits, and in as many bytes
  // in the other limb types. Only constants of the types are used, so no value of half a gibibyte
  // is made.
  assert_eq!(Uint::<U4294967295, [u64; 67108864]>::BITS, u32::MAX);
  assert_eq!(Int::<U4294967295, [u64; 67108864]>::BITS, u32::MAX);
  assert_eq!(Uint::<U4294967295, [u128; 33554432]>::BITS, u32::MAX);
  assert_eq!(Int::<U4294967295, [u128; 33554432]>::BITS, u32::MAX);
  assert_eq!(Uint::<U4294967295, [u32; 134217728]>::BITS, u32::MAX);
  assert_eq!(Int::<U4294967295, [u32; 134217728]>::BITS, u32::MAX);
  assert_eq!(Uint::<U4294967295, [u16; 268435456]>::BITS, u32::MAX);
  assert_eq!(Int::<U4294967295, [u16; 268435456]>::BITS, u32::MAX);
  assert_eq!(Uint::<U4294967295, [u8; 536870912]>::BITS, u32::MAX);
  assert_eq!(Int::<U4294967295, [u8; 536870912]>::BITS, u32::MAX);
}

/// The proptest configuration: the default, or under Miri a few cases from a fixed seed, written
/// to no file.
fn config() -> ProptestConfig {
  if cfg!(miri) {
    ProptestConfig {
      cases: 4,
      failure_persistence: None,
      rng_seed: RngSeed::Fixed(0x5eed),
      ..ProptestConfig::default()
    }
  } else {
    ProptestConfig::default()
  }
}

/// Tests of one width against `num-bigint`, after the limb type of the storages. Each type is
/// listed with its sign, `signed` or `unsigned`, its numbers of bits and limbs, and `full` when the
/// bits fill the limbs.
macro_rules! width_tests {
  ($limb:ident; $(
    $module:ident: $ty:ty, $sign:ident, $bits:literal, $limbs:literal $(, $full:ident)?
  );+ $(;)?) => {$(
    mod $module {
      use super::*;

      type T = $ty;

      /// The type of a limb.
      type Limb = $limb;

      const SIGNED: bool = width_tests!(@signed $sign);
      const BITS: u32 = $bits;
      const LIMBS: usize = $limbs;

      fn big(value: T) -> BigInt {
        bigint::from_limbs(&value.to_limbs(), SIGNED)
      }

      fn min() -> BigInt {
        bigint::min(BITS, SIGNED)
      }

      fn max() -> BigInt {
        bigint::max(BITS, SIGNED)
      }

      /// The limbs of `value`, in `LIMBS` limbs of two's complement for a signed type.
      fn limbs_of(value: &BigInt) -> [Limb; LIMBS] {
        bigint::to_limbs_of(value, SIGNED).expect("the limbs hold the value")
      }

      /// The values at and next to zero, the limits, the limb boundaries and the sign bit.
      fn values() -> Vec<T> {
        let (min, max) = (min(), max());
        let one = BigInt::from(1u8);
        let mut candidates = vec![
          BigInt::ZERO,
          one.clone(),
          BigInt::from(2u8),
          max.clone(),
          &max - 1u8,
          min.clone(),
          &min + 1u8,
        ];
        // The powers of two around the boundaries of 64-bit limbs and the sign bit, and then
        // around the boundaries of the limbs of `Limb`.
        let b = Limb::BITS;
        let exps = [1, 62, 63, 64, 65, 127, 128, BITS.saturating_sub(2), BITS - 1];
        for exp in exps.into_iter().chain([b - 1, b, b + 1, 2 * b - 1, 2 * b]) {
          let power = &one << exp;
          candidates.extend([&power - 1u8, power]);
        }
        let mut values: Vec<BigInt> = Vec::new();
        for candidate in candidates {
          for value in [-&candidate, candidate] {
            if min <= value && value <= max && !values.contains(&value) {
              values.push(value);
            }
          }
        }
        bigint::thin(values)
          .iter()
          .map(|value| T::checked_from_limbs(limbs_of(value)).expect("in range"))
          .collect()
      }

      #[test]
      #[cfg_attr(
        all(miri, target_pointer_width = "32"),
        ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
      )]
      fn constants_match_big_integers() {
        assert_eq!(T::BITS, BITS);
        assert_eq!(big(T::ZERO), BigInt::ZERO);
        assert_eq!(big(T::MIN), min());
        assert_eq!(big(T::MAX), max());
        // The limits hold their values in every limb, as `checked_from_limbs` requires.
        assert_eq!(T::MIN.to_limbs(), limbs_of(&min()));
        assert_eq!(T::MAX.to_limbs(), limbs_of(&max()));
        assert_eq!(T::checked_from_limbs(T::MIN.to_limbs()), Some(T::MIN));
        assert_eq!(T::checked_from_limbs(T::MAX.to_limbs()), Some(T::MAX));
        assert_eq!(T::default(), T::ZERO);
        assert_eq!(T::ZERO.to_limbs(), [0; LIMBS]);
      }

      #[test]
      #[cfg_attr(
        all(miri, target_pointer_width = "32"),
        ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
      )]
      fn checked_from_limbs_accepts_exactly_the_values() {
        let (min, max) = (min(), max());
        for pattern in bigint::bit_patterns_of::<Limb, LIMBS>(&[BITS]) {
          let value = bigint::from_limbs(&pattern, SIGNED);
          let checked = T::checked_from_limbs(pattern);
          assert_eq!(checked.is_some(), min <= value && value <= max, "{pattern:x?}");
          if let Some(checked) = checked {
            assert_eq!(checked.to_limbs(), pattern);
            assert_eq!(checked.as_limbs(), &pattern);
          }
        }
      }

      #[test]
      #[cfg_attr(
        all(miri, target_pointer_width = "32"),
        ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
      )]
      fn equality_order_and_hash_are_numeric() {
        let values: Vec<(T, BigInt)> = values().into_iter().map(|value| (value, big(value))).collect();
        let state = RandomState::new();
        for &(a, ref big_a) in &values {
          // One `write_u64` for each of the lowest `ceil(Bits / 64)` 64-bit words, least
          // significant first, and no length prefix. For `u64` limbs, the words are the limbs.
          assert_eq!(recorded(&a), word_calls(big_a, BITS), "{a}");
          for &(b, ref big_b) in &values {
            assert_eq!(a == b, big_a == big_b);
            assert_eq!(a.cmp(&b), big_a.cmp(big_b), "{a} and {b}");
            assert_eq!(a.partial_cmp(&b), Some(big_a.cmp(big_b)));
            if a == b {
              assert_eq!(state.hash_one(a), state.hash_one(b));
            }
          }
        }
        let mut sorted: Vec<T> = values.iter().map(|&(value, _)| value).collect();
        sorted.sort();
        assert!(sorted.windows(2).all(|pair| big(pair[0]) < big(pair[1])));
      }

      #[test]
      #[cfg_attr(
        all(miri, target_pointer_width = "32"),
        ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
      )]
      fn formatting_matches_big_integers() {
        for value in values() {
          let reference = big(value);
          assert_eq!(value.to_string(), reference.to_string());
          // Under Miri, which divides many limbs slowly, the widest values skip the other formats.
          if LIMBS <= 8 || !cfg!(miri) {
            assert_eq!(format!("{value:?}"), reference.to_string());
            assert_same_format!(
              value,
              reference,
              "{:1}",
              "{:120}",
              "{:<120}",
              "{:^121}",
              "{:*>120}",
              "{:+}",
              "{:+120}",
              "{:0120}",
              "{:+0120}",
              "{:<0120}",
              "{:>120?}",
              "{:+0120?}",
            );
          }
        }
      }

      width_tests!(@sign_tests $sign);

      $(width_tests!(@$full);)?

      proptest! {
        #![proptest_config(config())]

        #[test]
        #[cfg_attr(
          all(miri, target_pointer_width = "32"),
          ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
        )]
        fn checked_from_limbs_matches_big_integers(limbs in any::<[Limb; LIMBS]>()) {
          let value = bigint::from_limbs(&limbs, SIGNED);
          prop_assert_eq!(T::checked_from_limbs(limbs).is_some(), min() <= value && value <= max());
        }

        // Random limbs rarely hold a value of a width that leaves bits unused, so these hold a
        // value of a random width, which `T` holds when the width is at most `Bits`.
        #[test]
        #[cfg_attr(
          all(miri, target_pointer_width = "32"),
          ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
        )]
        fn checked_from_limbs_matches_big_integers_near_the_limits(
          limbs in any::<[Limb; LIMBS]>(),
          width in 1..=Limb::BITS * LIMBS as u32,
        ) {
          let modulus = BigInt::from(1u8) << width;
          let low = bigint::from_limbs(&limbs, false) % &modulus;
          let value = if SIGNED && low >= &modulus >> 1u8 { low - &modulus } else { low };
          let checked = T::checked_from_limbs(limbs_of(&value));
          prop_assert_eq!(checked.is_some(), min() <= value && value <= max());
          if let Some(checked) = checked {
            prop_assert_eq!(big(checked), value);
          }
        }
      }
    }
  )+};
  (@signed signed) => {
    true
  };
  (@signed unsigned) => {
    false
  };
  (@sign_tests signed) => {
    #[test]
    #[cfg_attr(
      all(miri, target_pointer_width = "32"),
      ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
    )]
    fn sign_queries_match_big_integers() {
      for value in values() {
        let sign = big(value).sign();
        assert_eq!(value.is_zero(), sign == Sign::NoSign, "{value}");
        assert_eq!(value.is_negative(), sign == Sign::Minus, "{value}");
        assert_eq!(value.is_positive(), sign == Sign::Plus, "{value}");
      }
      assert!(T::MIN.is_negative() && !T::MAX.is_negative());
    }
  };
  (@sign_tests unsigned) => {
    #[test]
    #[cfg_attr(
      all(miri, target_pointer_width = "32"),
      ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
    )]
    fn zero_query_matches_big_integers() {
      for value in values() {
        assert_eq!(value.is_zero(), big(value).sign() == Sign::NoSign, "{value}");
      }
    }
  };
  (@full) => {
    #[test]
    #[cfg_attr(
      all(miri, target_pointer_width = "32"),
      ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
    )]
    fn from_limbs_takes_every_pattern() {
      for pattern in bigint::bit_patterns_of::<Limb, LIMBS>(&[BITS]) {
        let value = T::from_limbs(pattern);
        assert_eq!(value.to_limbs(), pattern);
        assert_eq!(T::checked_from_limbs(pattern), Some(value));
      }
    }
  };
}

width_tests!(u64;
  // Widths that fill their limbs: every alias, and one and two limbs.
  uint_64_in_1: Uint<U64, [u64; 1]>, unsigned, 64, 1, full;
  uint_128_in_2: Uint<U128, [u64; 2]>, unsigned, 128, 2, full;
  uint_192: u192, unsigned, 192, 3, full;
  uint_256: u256, unsigned, 256, 4, full;
  uint_320: u320, unsigned, 320, 5, full;
  uint_384: u384, unsigned, 384, 6, full;
  uint_448: u448, unsigned, 448, 7, full;
  uint_512: u512, unsigned, 512, 8, full;
  uint_768: u768, unsigned, 768, 12, full;
  uint_1024: u1024, unsigned, 1024, 16, full;
  uint_2048: u2048, unsigned, 2048, 32, full;
  uint_4096: u4096, unsigned, 4096, 64, full;
  int_64_in_1: Int<U64, [u64; 1]>, signed, 64, 1, full;
  int_128_in_2: Int<U128, [u64; 2]>, signed, 128, 2, full;
  int_192: i192, signed, 192, 3, full;
  int_256: i256, signed, 256, 4, full;
  int_320: i320, signed, 320, 5, full;
  int_384: i384, signed, 384, 6, full;
  int_448: i448, signed, 448, 7, full;
  int_512: i512, signed, 512, 8, full;
  int_768: i768, signed, 768, 12, full;
  int_1024: i1024, signed, 1024, 16, full;
  int_2048: i2048, signed, 2048, 32, full;
  int_4096: i4096, signed, 4096, 64, full;
  // Widths that leave bits of the top limb unused.
  uint_1_in_1: Uint<U1, [u64; 1]>, unsigned, 1, 1;
  uint_7_in_1: Uint<U7, [u64; 1]>, unsigned, 7, 1;
  uint_63_in_1: Uint<U63, [u64; 1]>, unsigned, 63, 1;
  uint_65_in_2: Uint<U65, [u64; 2]>, unsigned, 65, 2;
  uint_127_in_2: Uint<U127, [u64; 2]>, unsigned, 127, 2;
  uint_129_in_3: Uint<U129, [u64; 3]>, unsigned, 129, 3;
  uint_160_in_3: Uint<U160, [u64; 3]>, unsigned, 160, 3;
  uint_200_in_4: Uint<U200, [u64; 4]>, unsigned, 200, 4;
  uint_255_in_4: Uint<U255, [u64; 4]>, unsigned, 255, 4;
  uint_257_in_5: Uint<U257, [u64; 5]>, unsigned, 257, 5;
  uint_511_in_8: Uint<U511, [u64; 8]>, unsigned, 511, 8;
  uint_1023_in_16: Uint<U1023, [u64; 16]>, unsigned, 1023, 16;
  int_1_in_1: Int<U1, [u64; 1]>, signed, 1, 1;
  int_2_in_1: Int<U2, [u64; 1]>, signed, 2, 1;
  int_7_in_1: Int<U7, [u64; 1]>, signed, 7, 1;
  int_63_in_1: Int<U63, [u64; 1]>, signed, 63, 1;
  int_65_in_2: Int<U65, [u64; 2]>, signed, 65, 2;
  int_127_in_2: Int<U127, [u64; 2]>, signed, 127, 2;
  int_129_in_3: Int<U129, [u64; 3]>, signed, 129, 3;
  int_160_in_3: Int<U160, [u64; 3]>, signed, 160, 3;
  int_200_in_4: Int<U200, [u64; 4]>, signed, 200, 4;
  int_255_in_4: Int<U255, [u64; 4]>, signed, 255, 4;
  int_257_in_5: Int<U257, [u64; 5]>, signed, 257, 5;
  int_511_in_8: Int<U511, [u64; 8]>, signed, 511, 8;
  int_1023_in_16: Int<U1023, [u64; 16]>, signed, 1023, 16;
  // Widths that leave limbs unused.
  uint_64_in_2: Uint<U64, [u64; 2]>, unsigned, 64, 2;
  uint_100_in_3: Uint<U100, [u64; 3]>, unsigned, 100, 3;
  int_64_in_2: Int<U64, [u64; 2]>, signed, 64, 2;
  int_100_in_3: Int<U100, [u64; 3]>, signed, 100, 3;
);

/// The widths of the narrower limb types: widths that fill one limb, two limbs and 256 bits, widths
/// that leave bits of the top limb unused, and a width that leaves a limb unused.
mod u8_limbs {
  use super::*;

  width_tests!(u8;
    uint_8_in_1: Uint<U8, [u8; 1]>, unsigned, 8, 1, full;
    uint_16_in_2: Uint<U16, [u8; 2]>, unsigned, 16, 2, full;
    uint_256_in_32: Uint<U256, [u8; 32]>, unsigned, 256, 32, full;
    int_8_in_1: Int<U8, [u8; 1]>, signed, 8, 1, full;
    int_16_in_2: Int<U16, [u8; 2]>, signed, 16, 2, full;
    int_256_in_32: Int<U256, [u8; 32]>, signed, 256, 32, full;
    uint_9_in_2: Uint<U9, [u8; 2]>, unsigned, 9, 2;
    uint_100_in_13: Uint<U100, [u8; 13]>, unsigned, 100, 13;
    int_1_in_1: Int<U1, [u8; 1]>, signed, 1, 1;
    int_2_in_1: Int<U2, [u8; 1]>, signed, 2, 1;
    int_7_in_1: Int<U7, [u8; 1]>, signed, 7, 1;
    int_9_in_2: Int<U9, [u8; 2]>, signed, 9, 2;
    int_100_in_13: Int<U100, [u8; 13]>, signed, 100, 13;
    uint_8_in_2: Uint<U8, [u8; 2]>, unsigned, 8, 2;
    int_8_in_2: Int<U8, [u8; 2]>, signed, 8, 2;
  );
}

/// The widths of `u16` limbs, as for `u8` limbs.
mod u16_limbs {
  use super::*;

  width_tests!(u16;
    uint_16_in_1: Uint<U16, [u16; 1]>, unsigned, 16, 1, full;
    uint_32_in_2: Uint<U32, [u16; 2]>, unsigned, 32, 2, full;
    uint_256_in_16: Uint<U256, [u16; 16]>, unsigned, 256, 16, full;
    int_16_in_1: Int<U16, [u16; 1]>, signed, 16, 1, full;
    int_32_in_2: Int<U32, [u16; 2]>, signed, 32, 2, full;
    int_256_in_16: Int<U256, [u16; 16]>, signed, 256, 16, full;
    uint_17_in_2: Uint<U17, [u16; 2]>, unsigned, 17, 2;
    uint_100_in_7: Uint<U100, [u16; 7]>, unsigned, 100, 7;
    int_1_in_1: Int<U1, [u16; 1]>, signed, 1, 1;
    int_2_in_1: Int<U2, [u16; 1]>, signed, 2, 1;
    int_7_in_1: Int<U7, [u16; 1]>, signed, 7, 1;
    int_17_in_2: Int<U17, [u16; 2]>, signed, 17, 2;
    int_100_in_7: Int<U100, [u16; 7]>, signed, 100, 7;
    uint_16_in_2: Uint<U16, [u16; 2]>, unsigned, 16, 2;
    int_16_in_2: Int<U16, [u16; 2]>, signed, 16, 2;
  );
}

/// The widths of `u32` limbs, as for `u8` limbs.
mod u32_limbs {
  use super::*;

  width_tests!(u32;
    uint_32_in_1: Uint<U32, [u32; 1]>, unsigned, 32, 1, full;
    uint_64_in_2: Uint<U64, [u32; 2]>, unsigned, 64, 2, full;
    uint_256_in_8: Uint<U256, [u32; 8]>, unsigned, 256, 8, full;
    int_32_in_1: Int<U32, [u32; 1]>, signed, 32, 1, full;
    int_64_in_2: Int<U64, [u32; 2]>, signed, 64, 2, full;
    int_256_in_8: Int<U256, [u32; 8]>, signed, 256, 8, full;
    uint_33_in_2: Uint<U33, [u32; 2]>, unsigned, 33, 2;
    uint_100_in_4: Uint<U100, [u32; 4]>, unsigned, 100, 4;
    int_1_in_1: Int<U1, [u32; 1]>, signed, 1, 1;
    int_2_in_1: Int<U2, [u32; 1]>, signed, 2, 1;
    int_7_in_1: Int<U7, [u32; 1]>, signed, 7, 1;
    int_33_in_2: Int<U33, [u32; 2]>, signed, 33, 2;
    int_100_in_4: Int<U100, [u32; 4]>, signed, 100, 4;
    uint_32_in_2: Uint<U32, [u32; 2]>, unsigned, 32, 2;
    int_32_in_2: Int<U32, [u32; 2]>, signed, 32, 2;
  );
}

/// The widths of `u128` limbs: widths that fill one, two and four limbs, widths that leave bits of
/// the top limb unused, and a width that leaves a limb unused.
mod u128_limbs {
  use super::*;

  width_tests!(u128;
    uint_128_in_1: Uint<U128, [u128; 1]>, unsigned, 128, 1, full;
    uint_256_in_2: Uint<U256, [u128; 2]>, unsigned, 256, 2, full;
    uint_512_in_4: Uint<U512, [u128; 4]>, unsigned, 512, 4, full;
    int_128_in_1: Int<U128, [u128; 1]>, signed, 128, 1, full;
    int_256_in_2: Int<U256, [u128; 2]>, signed, 256, 2, full;
    int_512_in_4: Int<U512, [u128; 4]>, signed, 512, 4, full;
    uint_100_in_1: Uint<U100, [u128; 1]>, unsigned, 100, 1;
    uint_129_in_2: Uint<U129, [u128; 2]>, unsigned, 129, 2;
    int_1_in_1: Int<U1, [u128; 1]>, signed, 1, 1;
    int_2_in_1: Int<U2, [u128; 1]>, signed, 2, 1;
    int_7_in_1: Int<U7, [u128; 1]>, signed, 7, 1;
    int_65_in_1: Int<U65, [u128; 1]>, signed, 65, 1;
    int_100_in_1: Int<U100, [u128; 1]>, signed, 100, 1;
    int_127_in_1: Int<U127, [u128; 1]>, signed, 127, 1;
    int_129_in_2: Int<U129, [u128; 2]>, signed, 129, 2;
    uint_128_in_2: Uint<U128, [u128; 2]>, unsigned, 128, 2;
    int_128_in_2: Int<U128, [u128; 2]>, signed, 128, 2;
  );
}

/// Asserts that `$narrow` and `$wide`, two storages of one width, make the same `Hash` calls for
/// each of `$values`.
macro_rules! assert_same_hash_calls {
  ($signed:literal, $narrow:ty, $wide:ty, $values:expr) => {
    for value in $values {
      let narrow =
        <$narrow>::checked_from_limbs(bigint::to_limbs_of(&value, $signed).expect("fits"))
          .expect("in range");
      let wide = <$wide>::checked_from_limbs(bigint::to_limbs_of(&value, $signed).expect("fits"))
        .expect("in range");
      assert_eq!(recorded(&narrow), recorded(&wide), "{value}");
    }
  };
}

/// Asserts that `$native`, a native storage, and `$array`, an array storage of the same width, make
/// the same `Hash` calls for each of `$values`.
macro_rules! assert_native_hash_calls {
  ($signed:literal, $native:ty, $array:ty, $values:expr) => {
    for value in $values {
      let native = <$native>::try_new((&value).try_into().expect("fits the native integer"))
        .expect("in range");
      let array = <$array>::checked_from_limbs(bigint::to_limbs_of(&value, $signed).expect("fits"))
        .expect("in range");
      assert_eq!(recorded(&native), recorded(&array), "{value}");
    }
  };
}

/// The limits of `bits` bits, the values next to them, and the values next to zero.
fn edges(bits: u32, signed: bool) -> Vec<BigInt> {
  let (min, max) = (bigint::min(bits, signed), bigint::max(bits, signed));
  let mut values = vec![BigInt::ZERO, BigInt::from(1u8), &max - 1u8, max.clone()];
  if signed {
    values.extend([BigInt::from(-1), &min + 1u8, min.clone()]);
  }
  // One signed bit holds only -1 and 0.
  values.retain(|value| min <= *value && *value <= max);
  values
}

#[test]
#[cfg_attr(
  all(miri, target_pointer_width = "32"),
  ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
)]
fn hash_calls_do_not_depend_on_the_number_of_limbs() {
  assert_same_hash_calls!(false, Uint<U1, [u64; 1]>, Uint<U1, [u64; 2]>, edges(1, false));
  assert_same_hash_calls!(false, Uint<U64, [u64; 1]>, Uint<U64, [u64; 2]>, edges(64, false));
  assert_same_hash_calls!(false, Uint<U100, [u64; 2]>, Uint<U100, [u64; 3]>, edges(100, false));
  assert_same_hash_calls!(false, u256, Uint<U256, [u64; 5]>, edges(256, false));
  assert_same_hash_calls!(true, Int<U2, [u64; 1]>, Int<U2, [u64; 3]>, edges(2, true));
  assert_same_hash_calls!(true, Int<U7, [u64; 1]>, Int<U7, [u64; 2]>, edges(7, true));
  assert_same_hash_calls!(true, Int<U64, [u64; 1]>, Int<U64, [u64; 2]>, edges(64, true));
  assert_same_hash_calls!(true, Int<U100, [u64; 2]>, Int<U100, [u64; 3]>, edges(100, true));
  assert_same_hash_calls!(true, i256, Int<U256, [u64; 5]>, edges(256, true));

  // A negative value writes the copies of its sign bit that its lowest limbs hold, and nothing
  // from the limbs above them.
  let minus_one = Int::<U100, [u64; 3]>::checked_from_limbs([u64::MAX; 3]).expect("-1");
  assert_eq!(
    recorded(&minus_one),
    [Call::U64(u64::MAX), Call::U64(u64::MAX)]
  );
}

#[test]
#[cfg_attr(
  all(miri, target_pointer_width = "32"),
  ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
)]
fn hash_calls_do_not_depend_on_the_limb_type() {
  // Storages of one byte, narrower than a word, and of bytes that end within a word.
  assert_same_hash_calls!(false, Uint<U1, [u8; 1]>, Uint<U1, [u64; 1]>, edges(1, false));
  assert_same_hash_calls!(false, Uint<U7, [u8; 1]>, Uint<U7, [u64; 1]>, edges(7, false));
  assert_same_hash_calls!(false, Uint<U9, [u8; 2]>, Uint<U9, [u64; 1]>, edges(9, false));
  assert_same_hash_calls!(false, Uint<U9, [u16; 1]>, Uint<U9, [u64; 1]>, edges(9, false));
  assert_same_hash_calls!(false, Uint<U64, [u8; 8]>, Uint<U64, [u64; 1]>, edges(64, false));
  assert_same_hash_calls!(false, Uint<U64, [u16; 4]>, Uint<U64, [u64; 1]>, edges(64, false));
  assert_same_hash_calls!(false, Uint<U64, [u32; 2]>, Uint<U64, [u64; 1]>, edges(64, false));
  assert_same_hash_calls!(false, Uint<U100, [u8; 13]>, Uint<U100, [u64; 2]>, edges(100, false));
  assert_same_hash_calls!(false, Uint<U100, [u16; 7]>, Uint<U100, [u64; 2]>, edges(100, false));
  assert_same_hash_calls!(false, Uint<U100, [u32; 4]>, Uint<U100, [u64; 2]>, edges(100, false));
  assert_same_hash_calls!(false, Uint<U256, [u8; 32]>, u256, edges(256, false));
  assert_same_hash_calls!(false, Uint<U256, [u16; 16]>, u256, edges(256, false));
  assert_same_hash_calls!(false, Uint<U256, [u32; 8]>, u256, edges(256, false));
  assert_same_hash_calls!(true, Int<U1, [u8; 1]>, Int<U1, [u64; 1]>, edges(1, true));
  assert_same_hash_calls!(true, Int<U7, [u8; 1]>, Int<U7, [u64; 1]>, edges(7, true));
  assert_same_hash_calls!(true, Int<U9, [u8; 2]>, Int<U9, [u64; 1]>, edges(9, true));
  assert_same_hash_calls!(true, Int<U9, [u16; 1]>, Int<U9, [u64; 1]>, edges(9, true));
  assert_same_hash_calls!(true, Int<U64, [u8; 8]>, Int<U64, [u64; 1]>, edges(64, true));
  assert_same_hash_calls!(true, Int<U64, [u16; 4]>, Int<U64, [u64; 1]>, edges(64, true));
  assert_same_hash_calls!(true, Int<U64, [u32; 2]>, Int<U64, [u64; 1]>, edges(64, true));
  assert_same_hash_calls!(true, Int<U100, [u8; 13]>, Int<U100, [u64; 2]>, edges(100, true));
  assert_same_hash_calls!(true, Int<U100, [u16; 7]>, Int<U100, [u64; 2]>, edges(100, true));
  assert_same_hash_calls!(true, Int<U100, [u32; 4]>, Int<U100, [u64; 2]>, edges(100, true));
  assert_same_hash_calls!(true, Int<U256, [u8; 32]>, i256, edges(256, true));
  assert_same_hash_calls!(true, Int<U256, [u16; 16]>, i256, edges(256, true));
  assert_same_hash_calls!(true, Int<U256, [u32; 8]>, i256, edges(256, true));
  // Storages of 128-bit limbs, each of which holds two words.
  assert_same_hash_calls!(false, Uint<U1, [u128; 1]>, Uint<U1, [u64; 1]>, edges(1, false));
  assert_same_hash_calls!(false, Uint<U7, [u128; 1]>, Uint<U7, [u64; 1]>, edges(7, false));
  assert_same_hash_calls!(false, Uint<U64, [u128; 1]>, Uint<U64, [u64; 1]>, edges(64, false));
  assert_same_hash_calls!(false, Uint<U65, [u128; 1]>, Uint<U65, [u64; 2]>, edges(65, false));
  assert_same_hash_calls!(false, Uint<U100, [u128; 1]>, Uint<U100, [u64; 2]>, edges(100, false));
  assert_same_hash_calls!(false, Uint<U128, [u128; 1]>, Uint<U128, [u64; 2]>, edges(128, false));
  assert_same_hash_calls!(false, Uint<U129, [u128; 2]>, Uint<U129, [u64; 3]>, edges(129, false));
  assert_same_hash_calls!(false, Uint<U192, [u128; 2]>, u192, edges(192, false));
  assert_same_hash_calls!(false, Uint<U256, [u128; 2]>, u256, edges(256, false));
  assert_same_hash_calls!(true, Int<U1, [u128; 1]>, Int<U1, [u64; 1]>, edges(1, true));
  assert_same_hash_calls!(true, Int<U7, [u128; 1]>, Int<U7, [u64; 1]>, edges(7, true));
  assert_same_hash_calls!(true, Int<U64, [u128; 1]>, Int<U64, [u64; 1]>, edges(64, true));
  assert_same_hash_calls!(true, Int<U65, [u128; 1]>, Int<U65, [u64; 2]>, edges(65, true));
  assert_same_hash_calls!(true, Int<U100, [u128; 1]>, Int<U100, [u64; 2]>, edges(100, true));
  assert_same_hash_calls!(true, Int<U128, [u128; 1]>, Int<U128, [u64; 2]>, edges(128, true));
  assert_same_hash_calls!(true, Int<U129, [u128; 2]>, Int<U129, [u64; 3]>, edges(129, true));
  assert_same_hash_calls!(true, Int<U192, [u128; 2]>, i192, edges(192, true));
  assert_same_hash_calls!(true, Int<U256, [u128; 2]>, i256, edges(256, true));

  // 192 bits in two 128-bit limbs: three words, the last of them the low half of the second limb.
  let value = Uint::<U192, [u128; 2]>::checked_from_limbs([1 | 2 << 64, 3]).expect("192 bits");
  assert_eq!(recorded(&value), [Call::U64(1), Call::U64(2), Call::U64(3)]);

  // A negative value writes copies of its sign bit for the limbs that a storage narrower than its
  // words lacks.
  let minus_one = Int::<U7, [u8; 1]>::checked_from_limbs([u8::MAX]).expect("-1");
  assert_eq!(recorded(&minus_one), [Call::U64(u64::MAX)]);
  let minus_one = Int::<U100, [u8; 13]>::checked_from_limbs([u8::MAX; 13]).expect("-1");
  assert_eq!(
    recorded(&minus_one),
    [Call::U64(u64::MAX), Call::U64(u64::MAX)]
  );
  let max = Uint::<U100, [u16; 7]>::MAX;
  assert_eq!(
    recorded(&max),
    [Call::U64(u64::MAX), Call::U64((1 << 36) - 1)]
  );
  // And a 128-bit limb writes the words of the value's width, whatever the bits above them.
  let minus_one = Int::<U64, [u128; 1]>::checked_from_limbs([u128::MAX]).expect("-1");
  assert_eq!(recorded(&minus_one), [Call::U64(u64::MAX)]);
  let minus_one = Int::<U100, [u128; 1]>::checked_from_limbs([u128::MAX]).expect("-1");
  assert_eq!(
    recorded(&minus_one),
    [Call::U64(u64::MAX), Call::U64(u64::MAX)]
  );

  // A native integer writes the words of an array of one limb of its width.
  assert_native_hash_calls!(false, Uint<U1, u8>, Uint<U1, [u64; 1]>, edges(1, false));
  assert_native_hash_calls!(false, Uint<U7, u8>, Uint<U7, [u8; 1]>, edges(7, false));
  assert_native_hash_calls!(false, Uint<U8, u8>, Uint<U8, [u8; 1]>, edges(8, false));
  assert_native_hash_calls!(false, Uint<U8, u8>, Uint<U8, [u64; 1]>, edges(8, false));
  assert_native_hash_calls!(false, Uint<U9, u16>, Uint<U9, [u64; 1]>, edges(9, false));
  assert_native_hash_calls!(false, Uint<U16, u16>, Uint<U16, [u16; 1]>, edges(16, false));
  assert_native_hash_calls!(false, Uint<U17, u32>, Uint<U17, [u64; 1]>, edges(17, false));
  assert_native_hash_calls!(false, Uint<U32, u32>, Uint<U32, [u32; 1]>, edges(32, false));
  assert_native_hash_calls!(false, Uint<U33, u64>, Uint<U33, [u64; 1]>, edges(33, false));
  assert_native_hash_calls!(false, Uint<U64, u64>, Uint<U64, [u64; 1]>, edges(64, false));
  assert_native_hash_calls!(false, Uint<U65, u128>, Uint<U65, [u64; 2]>, edges(65, false));
  assert_native_hash_calls!(false, Uint<U100, u128>, Uint<U100, [u64; 2]>, edges(100, false));
  assert_native_hash_calls!(false, Uint<U127, u128>, Uint<U127, [u128; 1]>, edges(127, false));
  assert_native_hash_calls!(false, Uint<U128, u128>, Uint<U128, [u64; 2]>, edges(128, false));
  assert_native_hash_calls!(true, Int<U1, i8>, Int<U1, [u64; 1]>, edges(1, true));
  assert_native_hash_calls!(true, Int<U7, i8>, Int<U7, [u8; 1]>, edges(7, true));
  assert_native_hash_calls!(true, Int<U8, i8>, Int<U8, [u8; 1]>, edges(8, true));
  assert_native_hash_calls!(true, Int<U8, i8>, Int<U8, [u64; 1]>, edges(8, true));
  assert_native_hash_calls!(true, Int<U9, i16>, Int<U9, [u64; 1]>, edges(9, true));
  assert_native_hash_calls!(true, Int<U16, i16>, Int<U16, [u16; 1]>, edges(16, true));
  assert_native_hash_calls!(true, Int<U17, i32>, Int<U17, [u64; 1]>, edges(17, true));
  assert_native_hash_calls!(true, Int<U32, i32>, Int<U32, [u32; 1]>, edges(32, true));
  assert_native_hash_calls!(true, Int<U33, i64>, Int<U33, [u64; 1]>, edges(33, true));
  assert_native_hash_calls!(true, Int<U64, i64>, Int<U64, [u64; 1]>, edges(64, true));
  assert_native_hash_calls!(true, Int<U65, i128>, Int<U65, [u64; 2]>, edges(65, true));
  assert_native_hash_calls!(true, Int<U100, i128>, Int<U100, [u64; 2]>, edges(100, true));
  assert_native_hash_calls!(true, Int<U127, i128>, Int<U127, [u128; 1]>, edges(127, true));
  assert_native_hash_calls!(true, Int<U128, i128>, Int<U128, [u64; 2]>, edges(128, true));
  // A wider native integer writes the words of the width, not of the native integer.
  assert_native_hash_calls!(false, Uint<U7, u128>, Uint<U7, [u64; 1]>, edges(7, false));
  assert_native_hash_calls!(true, Int<U7, i128>, Int<U7, [u64; 1]>, edges(7, true));
  // A negative value writes copies of its sign bit above the native integer.
  assert_eq!(recorded(&Int::<U7, i8>::new(-1)), [Call::U64(u64::MAX)]);
  assert_eq!(
    recorded(&Int::<U100, i128>::new(-1)),
    [Call::U64(u64::MAX), Call::U64(u64::MAX)]
  );
}

/// A deterministic xorshift generator, so that a failure replays from the seed.
struct Rng(u64);

impl Rng {
  fn next(&mut self) -> u64 {
    self.0 ^= self.0 >> 12;
    self.0 ^= self.0 << 25;
    self.0 ^= self.0 >> 27;
    self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
  }

  /// `N` words, each random, zero or all ones.
  fn words<const N: usize>(&mut self) -> [u64; N] {
    core::array::from_fn(|_| match self.next() % 4 {
      0 => 0,
      1 => u64::MAX,
      _ => self.next(),
    })
  }
}

/// `words` in `M` limbs of type `L`, least significant first. Requires `M` limbs of `L` to have as
/// many bits as `N` words.
fn split<L: TryFrom<u64>, const N: usize, const M: usize>(words: &[u64; N]) -> [L; M] {
  let per_word = M / N;
  let limb_bits = 64 / per_word;
  let mask = u64::MAX >> (64 - limb_bits);
  core::array::from_fn(|i| {
    let limb = words[i / per_word] >> (limb_bits * (i % per_word)) & mask;
    L::try_from(limb).ok().expect("the limb fits")
  })
}

/// The words of `limbs`, `M` limbs of type `L`, least significant first. Requires `M` limbs of
/// `L` to have as many bits as `N` words.
fn join<L: Copy + Into<u64>, const M: usize, const N: usize>(limbs: &[L; M]) -> [u64; N] {
  let per_word = M / N;
  let limb_bits = 64 / per_word;
  core::array::from_fn(|i| {
    (0..per_word).fold(0, |word, k| {
      let limb: u64 = limbs[per_word * i + k].into();
      word | limb << (limb_bits * k)
    })
  })
}

/// `words` in limbs of 128 bits, each joining two words, the low one first. Requires `N == 2 * M`.
fn pair<const N: usize, const M: usize>(words: &[u64; N]) -> [u128; M] {
  core::array::from_fn(|i| u128::from(words[2 * i]) | u128::from(words[2 * i + 1]) << 64)
}

/// The words of `limbs`, two for each limb of 128 bits, the low one first. Requires `N == 2 * M`.
fn unpair<const M: usize, const N: usize>(limbs: &[u128; M]) -> [u64; N] {
  core::array::from_fn(|i| (limbs[i / 2] >> (64 * (i % 2))) as u64)
}

/// `words` with every bit at index `bits` or above cleared, or for a signed value set to bit
/// `bits - 1`.
fn truncate<const N: usize>(words: &[u64; N], bits: u32, signed: bool) -> [u64; N] {
  let negative = signed && words[(bits as usize - 1) / 64] >> ((bits - 1) % 64) & 1 == 1;
  core::array::from_fn(|i| {
    let low = bits.saturating_sub(64 * i as u32).min(64);
    let kept = if low == 64 { u64::MAX } else { (1 << low) - 1 };
    match negative {
      false => words[i] & kept,
      true => words[i] | !kept,
    }
  })
}

/// Checks the same `Uint` or `Int` values in `[u64; N]`, `[u32; 2 * N]`, `[u16; 4 * N]` and
/// `[u8; 8 * N]`, and in `[u128; N / 2]` when it is given last: `$integer` is the type, and
/// `$signed` whether it is `Int`.
macro_rules! storages_agree {
  ($integer:ident, $signed:tt, $bits:ty, $n:literal $(, $halves:ty)?) => {{
    type W = $integer<$bits, [u64; $n]>;
    type N32 = $integer<$bits, [u32; 2 * $n]>;
    type N16 = $integer<$bits, [u16; 4 * $n]>;
    type N8 = $integer<$bits, [u8; 8 * $n]>;
    let bits = <$bits>::U32;

    // The limits in every storage, joined into words, are those of the 64-bit limbs.
    assert_eq!(join(&N32::MAX.to_limbs()), W::MAX.to_limbs());
    assert_eq!(join(&N16::MAX.to_limbs()), W::MAX.to_limbs());
    assert_eq!(join(&N8::MAX.to_limbs()), W::MAX.to_limbs());
    assert_eq!(join(&N32::MIN.to_limbs()), W::MIN.to_limbs());
    assert_eq!(join(&N16::MIN.to_limbs()), W::MIN.to_limbs());
    assert_eq!(join(&N8::MIN.to_limbs()), W::MIN.to_limbs());

    let mut rng = Rng(0x5707_a9e5 ^ u64::from(bits) ^ u64::from($signed));
    // Formatting thousands of bits in narrow limbs is slow under Miri.
    let count = if cfg!(miri) { 2 } else if $n > 8 { 4 } else { 100 };
    let format = $n <= 8 || !cfg!(miri);
    let mut values: Vec<(W, N32, N16, N8)> = Vec::new();
    for _ in 0..count {
      // Random words, which hold a value of `bits` bits only now and then, and the value of
      // `bits` bits in their low bits.
      let raw = rng.words::<$n>();
      let accepted = W::checked_from_limbs(raw).is_some();
      assert_eq!(N32::checked_from_limbs(split(&raw)).is_some(), accepted);
      assert_eq!(N16::checked_from_limbs(split(&raw)).is_some(), accepted);
      assert_eq!(N8::checked_from_limbs(split(&raw)).is_some(), accepted);

      let words = truncate(&raw, bits, $signed);
      let wide = W::checked_from_limbs(words).expect("in range");
      let n32 = N32::checked_from_limbs(split(&words)).expect("in range");
      let n16 = N16::checked_from_limbs(split(&words)).expect("in range");
      let n8 = N8::checked_from_limbs(split(&words)).expect("in range");
      assert_eq!(join(&n32.to_limbs()), words);
      assert_eq!(join(&n16.to_limbs()), words);
      assert_eq!(join(&n8.to_limbs()), words);
      storages_agree!(@queries $signed, wide, n32, n16, n8);
      assert_eq!(recorded(&n32), recorded(&wide));
      assert_eq!(recorded(&n16), recorded(&wide));
      assert_eq!(recorded(&n8), recorded(&wide));
      if format {
        storages_agree!(@formats wide, n32, n16, n8);
      }
      values.push((wide, n32, n16, n8));
    }
    for &(wide, n32, n16, n8) in &values {
      for &(other_wide, other_32, other_16, other_8) in &values {
        let order = wide.cmp(&other_wide);
        assert_eq!(n32.cmp(&other_32), order);
        assert_eq!(n16.cmp(&other_16), order);
        assert_eq!(n8.cmp(&other_8), order);
        assert_eq!(n8 == other_8, wide == other_wide);
      }
    }
    $(storages_agree!(@halves $integer, $signed, $bits, $n, $halves);)?
  }};
  // The same values in `$halves`, an array of 128-bit limbs, each of which holds two of the words.
  (@halves $integer:ident, $signed:tt, $bits:ty, $n:literal, $halves:ty) => {{
    type W = $integer<$bits, [u64; $n]>;
    type H = $integer<$bits, $halves>;
    let bits = <$bits>::U32;

    // The limits, split into words, are those of the 64-bit limbs.
    assert_eq!(unpair(H::MAX.as_limbs()), W::MAX.to_limbs());
    assert_eq!(unpair(H::MIN.as_limbs()), W::MIN.to_limbs());

    // The random words of the narrower limb types, from the same seed.
    let mut rng = Rng(0x5707_a9e5 ^ u64::from(bits) ^ u64::from($signed));
    let count = if cfg!(miri) { 2 } else if $n > 8 { 4 } else { 100 };
    let format = $n <= 8 || !cfg!(miri);
    let mut values: Vec<(W, H)> = Vec::new();
    for _ in 0..count {
      let raw = rng.words::<$n>();
      assert_eq!(
        H::checked_from_limbs(pair(&raw)).is_some(),
        W::checked_from_limbs(raw).is_some()
      );

      let words = truncate(&raw, bits, $signed);
      let wide = W::checked_from_limbs(words).expect("in range");
      let halves = H::checked_from_limbs(pair(&words)).expect("in range");
      assert_eq!(unpair(halves.as_limbs()), words);
      storages_agree!(@queries $signed, wide, halves);
      assert_eq!(recorded(&halves), recorded(&wide));
      if format {
        storages_agree!(@formats wide, halves);
      }
      values.push((wide, halves));
    }
    for &(wide, halves) in &values {
      for &(other_wide, other_halves) in &values {
        assert_eq!(halves.cmp(&other_halves), wide.cmp(&other_wide));
        assert_eq!(halves == other_halves, wide == other_wide);
      }
    }
  }};
  (@formats $wide:ident, $($narrow:ident),+) => {
    $(
      assert_same_format!(
        $narrow,
        $wide,
        "{}",
        "{:120}",
        "{:<120}",
        "{:^121}",
        "{:*>120}",
        "{:+}",
        "{:+0120}",
        "{:<0120}",
        "{:?}",
        "{:>120?}",
      );
    )+
  };
  (@queries false, $wide:ident, $($narrow:ident),+) => {
    $(assert_eq!($narrow.is_zero(), $wide.is_zero());)+
  };
  (@queries true, $wide:ident, $($narrow:ident),+) => {
    $(
      assert_eq!($narrow.is_zero(), $wide.is_zero());
      assert_eq!($narrow.is_negative(), $wide.is_negative());
      assert_eq!($narrow.is_positive(), $wide.is_positive());
    )+
  };
}

/// The same random values of widths that fill and that leave unused bits of `[u64; N]`, stored in
/// every limb type, in `[u128; N / 2]` for an even `N`: the text under each format, the order, the
/// hash calls, the sign queries, the limits and the limbs that `checked_from_limbs` accepts do not
/// depend on the limb type. Under Miri the widest values are not formatted.
#[test]
fn every_limb_type_holds_the_same_values() {
  storages_agree!(Uint, false, U64, 1);
  storages_agree!(Int, true, U64, 1);
  storages_agree!(Uint, false, U7, 1);
  storages_agree!(Int, true, U7, 1);
  storages_agree!(Uint, false, U100, 2, [u128; 1]);
  storages_agree!(Int, true, U100, 2, [u128; 1]);
  storages_agree!(Uint, false, U256, 4, [u128; 2]);
  storages_agree!(Int, true, U256, 4, [u128; 2]);
  storages_agree!(Uint, false, U200, 4, [u128; 2]);
  storages_agree!(Int, true, U200, 4, [u128; 2]);
  storages_agree!(Uint, false, U4096, 64, [u128; 32]);
  storages_agree!(Int, true, U4096, 64, [u128; 32]);
}

/// Tests of one native storage against the native integer and `num-bigint`. Each type is listed with
/// its sign, `signed` or `unsigned`, its number of bits, its native integer, and `partial` when the
/// native integer has more bits than the type.
macro_rules! native_tests {
  ($(
    $module:ident: $ty:ty, $sign:ident, $bits:literal, $native:ident $(, $partial:ident)?
  );+ $(;)?) => {$(
    mod $module {
      use super::*;

      type T = $ty;

      /// The native integer.
      type Native = $native;

      /// The 128-bit integer of the same sign, which holds every value of the native integer.
      type Wide = native_tests!(@wide $sign);

      const SIGNED: bool = native_tests!(@signed $sign);
      const BITS: u32 = $bits;

      /// The largest value of `BITS` bits, computed in 128 bits.
      const HIGH: Wide = native_tests!(@high $sign);

      /// The smallest value of `BITS` bits.
      const LOW: Wide = native_tests!(@low $sign);

      fn in_range(value: Native) -> bool {
        (LOW..=HIGH).contains(&(value as Wide))
      }

      /// Native values next to zero, to the limits of `BITS` bits and to those of the native
      /// integer, the powers of two with their neighbours, and seeded random values.
      fn candidates() -> Vec<Native> {
        let mut wide: Vec<Wide> = Vec::new();
        for limit in [LOW, HIGH, 0, Native::MIN as Wide, Native::MAX as Wide] {
          wide.extend([limit.checked_sub(1), Some(limit), limit.checked_add(1)].into_iter().flatten());
        }
        for exp in 0..Native::BITS - u32::from(SIGNED) {
          let power: Wide = 1 << exp;
          wide.extend([power - 1, power, power + 1]);
          native_tests!(@negatives $sign wide, power);
        }
        let mut rng = Rng(0x7e57_0000 ^ u64::from(BITS) ^ u64::from(SIGNED));
        for _ in 0..if cfg!(miri) { 4 } else { 64 } {
          let random = u128::from(rng.next()) << 64 | u128::from(rng.next());
          wide.push(random as Native as Wide);
        }
        let mut values: Vec<Native> =
          wide.into_iter().filter_map(|value| Native::try_from(value).ok()).collect();
        values.sort_unstable();
        values.dedup();
        values
      }

      /// The candidates that are values of `BITS` bits, or under Miri every third of them.
      fn values() -> Vec<T> {
        let values = candidates().into_iter().filter(|&value| in_range(value)).map(T::new);
        bigint::thin(values.collect())
      }

      #[test]
      #[cfg_attr(
        all(miri, target_pointer_width = "32"),
        ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
      )]
      fn constants_match_big_integers() {
        assert_eq!(T::BITS, BITS);
        assert_eq!(BigInt::from(T::ZERO.value()), BigInt::ZERO);
        assert_eq!(BigInt::from(T::MIN.value()), bigint::min(BITS, SIGNED));
        assert_eq!(BigInt::from(T::MAX.value()), bigint::max(BITS, SIGNED));
        assert_eq!(T::MIN.value() as Wide, LOW);
        assert_eq!(T::MAX.value() as Wide, HIGH);
        assert_eq!(T::default(), T::ZERO);
        assert_eq!(size_of::<T>(), size_of::<Native>());
        assert_eq!(align_of::<T>(), align_of::<Native>());
      }

      // `new` agrees with `try_new` on every value that it accepts.
      #[test]
      fn try_new_accepts_exactly_the_range() {
        let check = |value: Native| match T::try_new(value) {
          Ok(integer) => {
            assert!(in_range(value), "{value}");
            assert_eq!(integer.value(), value);
            assert_eq!(T::new(value), integer);
          }
          Err(error) => {
            assert!(!in_range(value), "{value}");
            assert_eq!(error, TryNewError);
          }
        };
        candidates().into_iter().for_each(check);
        // Every value of a native integer of 8 or 16 bits, or under Miri every 61st of 16 bits.
        if Native::BITS <= 16 {
          let step = if cfg!(miri) && Native::BITS == 16 { 61 } else { 1 };
          (Native::MIN..=Native::MAX).step_by(step).for_each(check);
        }
      }

      #[test]
      #[cfg_attr(
        all(miri, target_pointer_width = "32"),
        ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
      )]
      fn equality_order_and_hash_are_numeric() {
        let values = values();
        let state = RandomState::new();
        for &a in &values {
          // The words of the value in two's complement, as every storage of `BITS` bits writes.
          assert_eq!(recorded(&a), word_calls(&BigInt::from(a.value()), BITS), "{a}");
          for &b in &values {
            assert_eq!(a == b, a.value() == b.value());
            assert_eq!(a.cmp(&b), a.value().cmp(&b.value()), "{a} and {b}");
            assert_eq!(a.partial_cmp(&b), Some(a.value().cmp(&b.value())));
            if a == b {
              assert_eq!(state.hash_one(a), state.hash_one(b));
            }
          }
        }
      }

      #[test]
      fn formatting_matches_the_native_integer() {
        for value in values() {
          let native = value.value();
          assert_eq!(value.to_string(), native.to_string());
          assert_same_format!(
            value,
            native,
            "{}",
            "{:1}",
            "{:120}",
            "{:<120}",
            "{:^121}",
            "{:*>120}",
            "{:+}",
            "{:+120}",
            "{:0120}",
            "{:+0120}",
            "{:<0120}",
            "{:#}",
            "{:.3}",
            "{:?}",
            "{:>120?}",
            "{:+0120?}",
          );
          // `Debug` writes decimal, where the native integer honors the hexadecimal flags.
          assert_eq!(format!("{value:x?}"), native.to_string());
          assert_eq!(format!("{value:X?}"), native.to_string());
        }
      }

      native_tests!(@sign_tests $sign);

      $(native_tests!(@$partial);)?

      proptest! {
        #![proptest_config(config())]

        #[test]
        fn try_new_matches_the_range(value in any::<Native>()) {
          prop_assert_eq!(T::try_new(value).is_ok(), in_range(value));
        }
      }
    }
  )+};
  (@wide signed) => {
    i128
  };
  (@wide unsigned) => {
    u128
  };
  (@signed signed) => {
    true
  };
  (@signed unsigned) => {
    false
  };
  (@high signed) => {
    if BITS == 128 { i128::MAX } else { (1 << (BITS - 1)) - 1 }
  };
  (@high unsigned) => {
    if BITS == 128 { u128::MAX } else { (1 << BITS) - 1 }
  };
  (@low signed) => {
    -HIGH - 1
  };
  (@low unsigned) => {
    0
  };
  (@negatives signed $values:ident, $power:ident) => {
    $values.extend([-$power - 1, -$power, -$power + 1]);
  };
  (@negatives unsigned $values:ident, $power:ident) => {};
  (@sign_tests signed) => {
    #[test]
    fn sign_queries_match_the_native_integer() {
      for value in values() {
        let native = value.value();
        assert_eq!(value.is_zero(), native == 0, "{value}");
        assert_eq!(value.is_negative(), native < 0, "{value}");
        assert_eq!(value.is_positive(), native > 0, "{value}");
      }
    }
  };
  (@sign_tests unsigned) => {
    #[test]
    fn zero_query_matches_the_native_integer() {
      for value in values() {
        assert_eq!(value.is_zero(), value.value() == 0, "{value}");
      }
    }
  };
  (@partial) => {
    #[test]
    #[should_panic(expected = "the value does not fit in `Bits` bits")]
    fn new_panics_outside_the_range() {
      // One more than the largest value, which the native integer holds.
      let _ = T::new((HIGH + 1) as Native);
    }
  };
}

native_tests!(
  // The smallest native integer of each width.
  uint_1_in_u8: Uint<U1, u8>, unsigned, 1, u8, partial;
  uint_2_in_u8: Uint<U2, u8>, unsigned, 2, u8, partial;
  uint_7_in_u8: Uint<U7, u8>, unsigned, 7, u8, partial;
  uint_8_in_u8: Uint<U8, u8>, unsigned, 8, u8;
  uint_9_in_u16: Uint<U9, u16>, unsigned, 9, u16, partial;
  uint_15_in_u16: Uint<U15, u16>, unsigned, 15, u16, partial;
  uint_16_in_u16: Uint<U16, u16>, unsigned, 16, u16;
  uint_17_in_u32: Uint<U17, u32>, unsigned, 17, u32, partial;
  uint_24_in_u32: Uint<U24, u32>, unsigned, 24, u32, partial;
  uint_31_in_u32: Uint<U31, u32>, unsigned, 31, u32, partial;
  uint_32_in_u32: Uint<U32, u32>, unsigned, 32, u32;
  uint_33_in_u64: Uint<U33, u64>, unsigned, 33, u64, partial;
  uint_63_in_u64: Uint<U63, u64>, unsigned, 63, u64, partial;
  uint_64_in_u64: Uint<U64, u64>, unsigned, 64, u64;
  uint_65_in_u128: Uint<U65, u128>, unsigned, 65, u128, partial;
  uint_100_in_u128: Uint<U100, u128>, unsigned, 100, u128, partial;
  uint_127_in_u128: Uint<U127, u128>, unsigned, 127, u128, partial;
  uint_128_in_u128: Uint<U128, u128>, unsigned, 128, u128;
  int_1_in_i8: Int<U1, i8>, signed, 1, i8, partial;
  int_2_in_i8: Int<U2, i8>, signed, 2, i8, partial;
  int_7_in_i8: Int<U7, i8>, signed, 7, i8, partial;
  int_8_in_i8: Int<U8, i8>, signed, 8, i8;
  int_9_in_i16: Int<U9, i16>, signed, 9, i16, partial;
  int_15_in_i16: Int<U15, i16>, signed, 15, i16, partial;
  int_16_in_i16: Int<U16, i16>, signed, 16, i16;
  int_17_in_i32: Int<U17, i32>, signed, 17, i32, partial;
  int_24_in_i32: Int<U24, i32>, signed, 24, i32, partial;
  int_31_in_i32: Int<U31, i32>, signed, 31, i32, partial;
  int_32_in_i32: Int<U32, i32>, signed, 32, i32;
  int_33_in_i64: Int<U33, i64>, signed, 33, i64, partial;
  int_63_in_i64: Int<U63, i64>, signed, 63, i64, partial;
  int_64_in_i64: Int<U64, i64>, signed, 64, i64;
  int_65_in_i128: Int<U65, i128>, signed, 65, i128, partial;
  int_100_in_i128: Int<U100, i128>, signed, 100, i128, partial;
  int_127_in_i128: Int<U127, i128>, signed, 127, i128, partial;
  int_128_in_i128: Int<U128, i128>, signed, 128, i128;
  // Wider native integers.
  uint_7_in_u16: Uint<U7, u16>, unsigned, 7, u16, partial;
  uint_7_in_u128: Uint<U7, u128>, unsigned, 7, u128, partial;
  uint_24_in_u64: Uint<U24, u64>, unsigned, 24, u64, partial;
  uint_33_in_u128: Uint<U33, u128>, unsigned, 33, u128, partial;
  uint_64_in_u128: Uint<U64, u128>, unsigned, 64, u128, partial;
  int_7_in_i16: Int<U7, i16>, signed, 7, i16, partial;
  int_7_in_i128: Int<U7, i128>, signed, 7, i128, partial;
  int_24_in_i64: Int<U24, i64>, signed, 24, i64, partial;
  int_33_in_i128: Int<U33, i128>, signed, 33, i128, partial;
  int_64_in_i128: Int<U64, i128>, signed, 64, i128, partial;
);

/// The values of a native integer from `min` to `max`, in order: every value, or under Miri every
/// 61st value and the values next to zero and to `low` and `high`, the limits of a width.
fn every_native_value(min: i128, max: i128, low: i128, high: i128) -> Vec<i128> {
  if !cfg!(miri) {
    return (min..=max).collect();
  }
  let mut values: Vec<i128> = (min..=max).step_by(61).collect();
  values.extend([low, low + 1, -1, 0, 1, high - 1, high, high + 1]);
  values.retain(|value| (min..=max).contains(value));
  values.sort_unstable();
  values.dedup();
  values
}

/// Every value of every width up to 16 bits, in its smallest native integer, in `u128` or `i128`,
/// in `[u64; 1]` and in the bytes of the native integer: the storages accept the same native
/// values, they write the same text and hash calls, and consecutive values compare in order.
#[test]
fn every_value_of_up_to_16_bits() {
  macro_rules! every_value {
    ($($bits:ident: $uint:ident, $int:ident);+ $(;)?) => {$(
      every_value!(@check Uint, $bits, $uint, u128, |value: $uint| u64::from(value));
      every_value!(@check Int, $bits, $int, i128, |value: $int| value as i64 as u64);
    )+};
    (@check $integer:ident, $bits:ident, $native:ident, $wide:ident, $word:expr) => {{
      type Bits = typenum::consts::$bits;
      type Small = $integer<Bits, $native>;
      type Wide = $integer<Bits, $wide>;
      type Word = $integer<Bits, [u64; 1]>;
      type Bytes = $integer<Bits, [u8; size_of::<$native>()]>;
      let word: fn($native) -> u64 = $word;
      let low = Small::MIN.value() as i128;
      let high = Small::MAX.value() as i128;
      let mut previous: Option<Small> = None;
      for value in every_native_value(<$native>::MIN as i128, <$native>::MAX as i128, low, high) {
        let native = value as $native;
        let small = Small::try_new(native);
        let wide = Wide::try_new(<$wide>::from(native));
        let words = Word::checked_from_limbs([word(native)]);
        let bytes = Bytes::checked_from_limbs(native.to_le_bytes());
        assert_eq!(small.is_ok(), (low..=high).contains(&value), "{value}");
        assert_eq!(wide.is_ok(), small.is_ok(), "{value}");
        assert_eq!(words.is_some(), small.is_ok(), "{value}");
        assert_eq!(bytes.is_some(), small.is_ok(), "{value}");
        if let (Ok(small), Ok(wide), Some(words), Some(bytes)) = (small, wide, words, bytes) {
          let text = native.to_string();
          assert_eq!(small.to_string(), text);
          assert_eq!(wide.to_string(), text);
          assert_eq!(words.to_string(), text);
          assert_eq!(bytes.to_string(), text);
          let calls = recorded(&small);
          assert_eq!(recorded(&wide), calls, "{value}");
          assert_eq!(recorded(&words), calls, "{value}");
          assert_eq!(recorded(&bytes), calls, "{value}");
          if let Some(previous) = previous {
            assert!(previous < small, "{previous} and {small}");
          }
          previous = Some(small);
        }
      }
    }};
  }

  every_value!(
    U1: u8, i8;
    U2: u8, i8;
    U3: u8, i8;
    U4: u8, i8;
    U5: u8, i8;
    U6: u8, i8;
    U7: u8, i8;
    U8: u8, i8;
    U9: u16, i16;
    U10: u16, i16;
    U11: u16, i16;
    U12: u16, i16;
    U13: u16, i16;
    U14: u16, i16;
    U15: u16, i16;
    U16: u16, i16;
  );
}

/// Every pair of values of `u7` and of `i7`: equality and order are those of the native integers
/// and of the same values in `[u64; 1]`. Under Miri, every 7th value against every value.
#[test]
fn every_pair_of_7_bits() {
  let step = if cfg!(miri) { 7 } else { 1 };
  let unsigned: Vec<(u7, Uint<U7, [u64; 1]>)> = (0..=127u8)
    .map(|value| {
      let word = Uint::<U7, [u64; 1]>::checked_from_limbs([u64::from(value)]).expect("7 bits");
      (u7::new(value), word)
    })
    .collect();
  for &(a, word_a) in unsigned.iter().step_by(step) {
    for &(b, word_b) in &unsigned {
      assert_eq!(a == b, a.value() == b.value());
      assert_eq!(a.cmp(&b), a.value().cmp(&b.value()), "{a} and {b}");
      assert_eq!(word_a.cmp(&word_b), a.cmp(&b), "{a} and {b}");
      assert_eq!(word_a == word_b, a == b);
    }
  }
  let signed: Vec<(i7, Int<U7, [u64; 1]>)> = (-64..=63i8)
    .map(|value| {
      let word = Int::<U7, [u64; 1]>::checked_from_limbs([value as i64 as u64]).expect("7 bits");
      (i7::new(value), word)
    })
    .collect();
  for &(a, word_a) in signed.iter().step_by(step) {
    for &(b, word_b) in &signed {
      assert_eq!(a == b, a.value() == b.value());
      assert_eq!(a.cmp(&b), a.value().cmp(&b.value()), "{a} and {b}");
      assert_eq!(word_a.cmp(&word_b), a.cmp(&b), "{a} and {b}");
      assert_eq!(word_a == word_b, a == b);
    }
  }
}

/// Values that write text and debug text, through which formats are compiled once for every type.
trait Text: Display + Debug {}

impl<T: Display + Debug + ?Sized> Text for T {}

/// Asserts that `value` and `reference` write the same text under each format.
fn assert_same_text(value: &dyn Text, reference: &dyn Text) {
  assert_same_format!(
    value, reference, "{}", "{:1}", "{:50}", "{:<50}", "{:^51}", "{:*>50}", "{:+}", "{:+50}",
    "{:050}", "{:+050}", "{:<050}", "{:#}", "{:.3}", "{:?}", "{:>50?}", "{:+050?}",
  );
}

/// Asserts that `natives`, values in a native storage `N`, agree with the same values in the array
/// storage `A` of `K` limbs of type `L`: the limits, the limbs, the text under each format, the hash
/// calls, the queries that `queries` compares, and the order of every pair.
fn assert_array_agrees<N, A, L, const K: usize>(natives: &[N], signed: bool, queries: fn(&N, &A))
where
  N: FixedInteger,
  A: FixedInteger + ArrayInteger<Array = [L; K]>,
  L: bigint::Limb + Debug + PartialEq,
{
  let limbs = |value: &N| -> [L; K] {
    let big: BigInt = value.to_string().parse().expect("decimal text");
    bigint::to_limbs_of(&big, signed).expect("the array holds the value")
  };
  let to_array = |value: &N| A::checked_from_limbs(limbs(value)).expect("in range");
  assert_eq!(A::MIN.to_limbs(), limbs(&N::MIN));
  assert_eq!(A::MAX.to_limbs(), limbs(&N::MAX));
  let arrays: Vec<A> = natives.iter().map(to_array).collect();
  for (native, array) in natives.iter().zip(&arrays) {
    assert_same_text(array, native);
    assert_eq!(recorded(array), recorded(native), "{native}");
    assert_eq!(FixedInteger::is_zero(array), FixedInteger::is_zero(native));
    queries(native, array);
  }
  for (native_a, array_a) in natives.iter().zip(&arrays) {
    for (native_b, array_b) in natives.iter().zip(&arrays) {
      assert_eq!(
        array_a.cmp(array_b),
        native_a.cmp(native_b),
        "{native_a} and {native_b}"
      );
    }
  }
}

/// Seeded random values at every width from 1 to 128 bits, in the default storage, the smallest
/// native integer, and in arrays of each limb type: the limits, the text under each format, the
/// hash calls, the queries and the order agree. Under Miri, one value of each sign at every fourth
/// width.
#[test]
#[cfg_attr(
  all(miri, target_pointer_width = "32"),
  ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
)]
fn native_and_array_storages_agree() {
  macro_rules! agree {
    ($($bits:ident),+ $(,)?) => {$({
      type B = typenum::consts::$bits;
      let bits = <B as Unsigned>::U32;
      let count = if cfg!(miri) { usize::from(bits % 4 == 0) } else { 16 };
      let mut rng = Rng(0xa9ee_0000 ^ u64::from(bits));
      let mut random = || u128::from(rng.next()) << 64 | u128::from(rng.next());

      // `bits` random bits, and the same bits sign-extended from bit `bits - 1`.
      let unsigned: Vec<Uint<B>> = (0..count)
        .map(|_| {
          let value = random() >> (128 - bits);
          Uint::<B>::new(value as <B as BitCount>::Unsigned)
        })
        .collect();
      let signed: Vec<Int<B>> = (0..count)
        .map(|_| {
          let value = ((random() << (128 - bits)) as i128) >> (128 - bits);
          Int::<B>::new(value as <B as BitCount>::Signed)
        })
        .collect();

      assert_array_agrees::<_, Uint<B, Limbs<B, u8>>, _, _>(&unsigned, false, |n, a| {
        assert_eq!(a.is_zero(), n.is_zero())
      });
      assert_array_agrees::<_, Uint<B, Limbs<B, u16>>, _, _>(&unsigned, false, |n, a| {
        assert_eq!(a.is_zero(), n.is_zero())
      });
      assert_array_agrees::<_, Uint<B, Limbs<B, u32>>, _, _>(&unsigned, false, |n, a| {
        assert_eq!(a.is_zero(), n.is_zero())
      });
      assert_array_agrees::<_, Uint<B, Limbs<B, u64>>, _, _>(&unsigned, false, |n, a| {
        assert_eq!(a.is_zero(), n.is_zero())
      });
      assert_array_agrees::<_, Uint<B, [u128; 1]>, _, _>(&unsigned, false, |n, a| {
        assert_eq!(a.is_zero(), n.is_zero())
      });
      assert_array_agrees::<_, Int<B, Limbs<B, u8>>, _, _>(&signed, true, |n, a| {
        assert_eq!([a.is_negative(), a.is_positive()], [n.is_negative(), n.is_positive()])
      });
      assert_array_agrees::<_, Int<B, Limbs<B, u16>>, _, _>(&signed, true, |n, a| {
        assert_eq!([a.is_negative(), a.is_positive()], [n.is_negative(), n.is_positive()])
      });
      assert_array_agrees::<_, Int<B, Limbs<B, u32>>, _, _>(&signed, true, |n, a| {
        assert_eq!([a.is_negative(), a.is_positive()], [n.is_negative(), n.is_positive()])
      });
      assert_array_agrees::<_, Int<B, Limbs<B, u64>>, _, _>(&signed, true, |n, a| {
        assert_eq!([a.is_negative(), a.is_positive()], [n.is_negative(), n.is_positive()])
      });
      assert_array_agrees::<_, Int<B, [u128; 1]>, _, _>(&signed, true, |n, a| {
        assert_eq!([a.is_negative(), a.is_positive()], [n.is_negative(), n.is_positive()])
      });
    })+};
  }

  agree!(
    U1, U2, U3, U4, U5, U6, U7, U8, U9, U10, U11, U12, U13, U14, U15, U16, U17, U18, U19, U20, U21,
    U22, U23, U24, U25, U26, U27, U28, U29, U30, U31, U32, U33, U34, U35, U36, U37, U38, U39, U40,
    U41, U42, U43, U44, U45, U46, U47, U48, U49, U50, U51, U52, U53, U54, U55, U56, U57, U58, U59,
    U60, U61, U62, U63, U64, U65, U66, U67, U68, U69, U70, U71, U72, U73, U74, U75, U76, U77, U78,
    U79, U80, U81, U82, U83, U84, U85, U86, U87, U88, U89, U90, U91, U92, U93, U94, U95, U96, U97,
    U98, U99, U100, U101, U102, U103, U104, U105, U106, U107, U108, U109, U110, U111, U112, U113,
    U114, U115, U116, U117, U118, U119, U120, U121, U122, U123, U124, U125, U126, U127, U128,
  );
}
