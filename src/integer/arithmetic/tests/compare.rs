//! The comparisons of `Uint` and `Int` with the native integers: the twelve numo integers of the
//! casts against the twelve native integers, both ways, with every comparison operator and
//! `partial_cmp`, against the bit model; literals, mixed signs and widths; and code generic over
//! the width. Each pair checks in an instantiation of a generic function of its own, whose frame
//! holds the values of that pair alone.

use core::{any::type_name, cmp::Ordering};

use typenum::{U12, U33};

use super::{
  casts::{Model, Native, samples},
  *,
};

/// The values of the native integer `N` with which every value of a numo integer is compared: the
/// smallest and the largest, the values next to them, and -1, 0 and 1 where `N` holds them.
fn native_ends<N: Native>() -> Vec<N> {
  let (min, max) = (N::MIN.as_u128(), N::MAX.as_u128());
  let mut ends: Vec<N> = Vec::new();
  for bits in [
    min,
    min.wrapping_add(1),
    u128::MAX,
    0,
    1,
    max.wrapping_sub(1),
    max,
  ] {
    let end = N::from_u128(bits);
    if !ends.contains(&end) {
      ends.push(end);
    }
  }
  ends
}

/// The values of the numo integer `T` with which every value of a native integer is compared:
/// zero, one where `T` holds it, every bit, which is -1 for a signed `T`, and the smallest and the
/// largest value.
fn numo_ends<T: Model>() -> Vec<T> {
  let (width, signed) = (T::WIDTH, T::SIGNED);
  let mut patterns = vec![
    model::pattern(width, |_| false),
    model::pattern(width, |_| true),
    bound_pattern(width, signed, true),
    bound_pattern(width, signed, false),
  ];
  // Zero bits and one signed bit hold no one.
  if width > u32::from(signed) {
    patterns.push(model::pattern(width, |i| i == 0));
  }
  let mut ends: Vec<T> = Vec::new();
  for pattern in &patterns {
    let end = T::of_pattern(pattern);
    if !ends.contains(&end) {
      ends.push(end);
    }
  }
  ends
}

/// Checks the comparisons of the numo integer `T` with the native integer `N`, both ways, against
/// the bit model: each value of `numos` with each value of `native_ends`, and each value of
/// `numo_ends` with each value of `natives`, with `==`, `!=`, `<`, `>`, `<=`, `>=` and
/// `partial_cmp`.
fn check_compare<T, N>(numos: &[T], numo_ends: &[T], natives: &[N], native_ends: &[N])
where
  T: Model + PartialEq<N> + PartialOrd<N>,
  N: Native + PartialEq<T> + PartialOrd<T>,
{
  let pairs = numos
    .iter()
    .flat_map(|x| native_ends.iter().map(move |n| (x, n)))
    .chain(
      numo_ends
        .iter()
        .flat_map(|x| natives.iter().map(move |n| (x, n))),
    );
  for (&x, &n) in pairs {
    let order = model::cmp(&x.pattern(), T::SIGNED, &n.pattern(), N::SIGNED);
    let name = || format!("{x:?} and {n:?} of {}", type_name::<N>());
    assert_eq!(
      (
        x.partial_cmp(&n),
        x == n,
        x != n,
        x < n,
        x > n,
        x <= n,
        x >= n
      ),
      (
        Some(order),
        order.is_eq(),
        order.is_ne(),
        order.is_lt(),
        order.is_gt(),
        order.is_le(),
        order.is_ge(),
      ),
      "{}",
      name()
    );
    assert_eq!(
      (
        n.partial_cmp(&x),
        n == x,
        n != x,
        n < x,
        n > x,
        n <= x,
        n >= x
      ),
      (
        Some(order.reverse()),
        order.is_eq(),
        order.is_ne(),
        order.is_gt(),
        order.is_lt(),
        order.is_ge(),
        order.is_le(),
      ),
      "{}",
      name()
    );
  }
}

/// Checks the comparisons of each type of `$numos` with each native integer of `$natives`, at the
/// values of [`samples`] and the ends of both, each pair in an instantiation of [`check_compare`]
/// of its own.
macro_rules! compare_pairs {
  ([$($numo:ty),+ $(,)?] with $natives:tt) => {$(
    compare_pairs!(@numo $numo, $natives);
  )+};
  (@numo $numo:ty, [$($native:ty),+ $(,)?]) => {{
    let (numos, ends) = (samples::<$numo>(), numo_ends::<$numo>());
    $(
      check_compare::<$numo, $native>(&numos, &ends, &samples::<$native>(), &native_ends());
    )+
  }};
}

/// The comparisons of the twelve numo integers of the casts, of widths from zero bits to 200 and of
/// every kind of storage, with every native integer, both ways, against the bit model: every value
/// of [`samples`] of the numo integer with the ends of the native integer, and the ends of the numo
/// integer with every value of [`samples`] of the native integer.
#[test]
fn numo_integers_compare_with_native_integers() {
  compare_pairs!(
    [
      Uint<U0>,
      Uint<U7, u8>,
      Int<U7, i8>,
      Uint<U8, [u8; 1]>,
      Int<U12, [u16; 1]>,
      Uint<U33, [u32; 2]>,
      Int<U64, i64>,
      Uint<U65, [u64; 2]>,
      Int<U100, [u128; 1]>,
      Uint<U128, u128>,
      Int<U129, [u64; 3]>,
      Uint<U200, [u8; 25]>,
    ] with [u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize]
  );
}

/// A literal on either side compares, as an `i32` where nothing else fixes its type, and mixed
/// signs and widths compare as integers: a negative value is below every unsigned integer, and a
/// value outside the range of the native integer is beyond each of its values.
#[test]
fn comparisons_take_literals_and_mixed_signs_and_widths() {
  assert!(Int::<U7>::new(-1) < 0u8);
  assert_eq!(u7::new(5), 5i8);
  assert!(u256::MAX > u128::MAX);
  assert!(u7::new(5) == 5 && 5 == u7::new(5));
  assert!(i7::MIN < -63 && -64 == i7::MIN && i7::MIN >= -64);
  assert!(i256::MIN < i128::MIN && i128::MAX < i256::MAX);
  assert_eq!(u256::from_limbs([u64::MAX, u64::MAX, 0, 0]), u128::MAX);
  assert_ne!(u256::from_limbs([0, 0, 1, 0]), 0u128);
  assert!(i7::new(-1) < 0usize && 0isize > i7::new(-1));
  assert_eq!(u7::new(100).partial_cmp(&200u8), Some(Ordering::Less));
  assert_eq!(300u16.partial_cmp(&u7::MAX), Some(Ordering::Greater));
  assert_eq!((-1i128).partial_cmp(&u256::ZERO), Some(Ordering::Less));
  assert_eq!(Uint::<U0>::ZERO, 0u8);
  assert_eq!(Int::<U1, i8>::MIN, -1i64);
}

/// Whether `x` is above `n`, in code generic over the width alone.
fn above<B: BitWidth>(x: Uint<B>, n: u64) -> bool {
  x > n
}

/// Whether `n` is below `x`, in code generic over the width alone.
fn below<B: BitWidth>(n: i64, x: Int<B>) -> bool {
  n < x
}

/// The comparisons in code generic over the width, whose bound `B: BitWidth` alone gives `Uint<B>`
/// and `Int<B>` the comparisons with the native integers, at widths of each kind of default
/// storage.
#[test]
fn comparisons_in_code_generic_over_the_width() {
  assert!(above(Uint::<U65>::MAX, u64::MAX));
  assert!(!above(Uint::<U64>::MAX, u64::MAX));
  assert!(!above(Uint::<U7>::MAX, 127));
  assert!(above(Uint::<U200>::MAX, 0));
  assert!(!above(Uint::<U0>::ZERO, 0));
  assert!(below(i64::MIN, Int::<U7>::MIN));
  assert!(!below(i64::MIN, Int::<U64>::MIN));
  assert!(below(i64::MIN, Int::<U200>::MAX));
  assert!(!below(-1, Int::<U200>::MIN));
}
