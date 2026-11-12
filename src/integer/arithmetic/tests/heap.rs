//! The heap storage `Box<[L; N]>` of `Uint`, `Int`, `NonZeroUint` and `NonZeroInt`, against its
//! inline twin, the array that it holds, through the checks of every heap storage and its own:
//! every row of the storage cores on a box against the row of the same name on its array, with the
//! pointers of the rows that never allocate; the layout and the auto traits of the boxed integers;
//! each way of building one; every trait and operator, compared through the limbs of the results,
//! at sampled operands and at the limits, where the methods return `None`, saturate or wrap; the
//! folds; the byte order; the allocations that the operations keep; the overflow of the operators
//! and of the assignments, and the panics at the limits; the non-zero integers, with the
//! constructors that reject zero; code generic over the storage traits; the widths of 4096 and
//! 65536 bits; and serde.

use typenum::{U20, U300, U65536};

use super::{shared::*, *};
use crate::{
  ArrayInteger, ByteArray, CheckedCastFrom, FixedNonZero, FromByteArray, IntRepr, IntegerByteOrder,
  NonZeroInt, NonZeroIntRepr, NonZeroUint, NonZeroUintRepr, SignedNonZero, UintRepr,
  UnsignedNonZero, WholeBytes, WidenFrom,
  int::parse_errors,
  integer::cast::try_cast,
  width::{ArrayLimb, ArrayRepr},
};

/// Every row of `UintCore` and `IntCore` on `Box<[L; N]>` gives what the row of the same name gives
/// on `[L; N]`, so that a row that forwards to the wrong row, or passes the wrong operand, changes a
/// result here, whatever calls it: `[u64; N]` at 0 bits in no limbs, at 1, 63 and 64 bits in one,
/// at 65 and 128 in two, at 200 and 256 in four and, in an optimized build, at 1024 in sixteen;
/// three bytes at 20 bits, two 16-bit limbs at 24, three 32-bit limbs at 65, two 128-bit limbs at
/// 129 and 256, and 25 bytes at 200. Under Miri, no limbs, two 64-bit limbs at 65 bits and three
/// bytes at 20.
#[test]
fn box_rows_agree_with_the_array_rows() {
  uint_rows::<Box<[u64; 0]>, u64, 0, 1>(0);
  int_rows::<Box<[u64; 0]>, u64, 0, 1>(0);
  uint_rows::<Box<[u64; 2]>, u64, 2, 2>(65);
  int_rows::<Box<[u64; 2]>, u64, 2, 2>(65);
  uint_rows::<Box<[u8; 3]>, u8, 3, 1>(20);
  int_rows::<Box<[u8; 3]>, u8, 3, 1>(20);
  if tier().is_miri() {
    return;
  }
  for bits in [1, 63, 64] {
    uint_rows::<Box<[u64; 1]>, u64, 1, 1>(bits);
    int_rows::<Box<[u64; 1]>, u64, 1, 1>(bits);
  }
  uint_rows::<Box<[u64; 2]>, u64, 2, 2>(128);
  int_rows::<Box<[u64; 2]>, u64, 2, 2>(128);
  for bits in [200, 256] {
    uint_rows::<Box<[u64; 4]>, u64, 4, 4>(bits);
    int_rows::<Box<[u64; 4]>, u64, 4, 4>(bits);
  }
  uint_rows::<Box<[u16; 2]>, u16, 2, 1>(24);
  int_rows::<Box<[u16; 2]>, u16, 2, 1>(24);
  uint_rows::<Box<[u32; 3]>, u32, 3, 2>(65);
  int_rows::<Box<[u32; 3]>, u32, 3, 2>(65);
  for bits in [129, 256] {
    uint_rows::<Box<[u128; 2]>, u128, 2, 4>(bits);
    int_rows::<Box<[u128; 2]>, u128, 2, 4>(bits);
  }
  uint_rows::<Box<[u8; 25]>, u8, 25, 4>(200);
  int_rows::<Box<[u8; 25]>, u8, 25, 4>(200);
  if tier().is_full() {
    uint_rows::<Box<[u64; 16]>, u64, 16, 16>(1024);
    int_rows::<Box<[u64; 16]>, u64, 16, 16>(1024);
  }
}

/// A boxed integer has the size and the alignment of a pointer, and so has its `Option`, which
/// takes the niche of the pointer, in limbs of each type and in no limbs, and so have the non-zero
/// integers; and each is `Send` and `Sync`, as the box is.
#[test]
fn boxed_integers_have_the_layout_of_a_pointer() {
  let (size, align) = (size_of::<usize>(), align_of::<usize>());
  let pointer = [size, align, size, align];
  assert_eq!(layout::<Uint<U256, Box<[u64; 4]>>>(), pointer);
  assert_eq!(layout::<Int<U256, Box<[u64; 4]>>>(), pointer);
  assert_eq!(layout::<Uint<U20, Box<[u8; 3]>>>(), pointer);
  assert_eq!(layout::<Int<U20, Box<[u8; 3]>>>(), pointer);
  assert_eq!(layout::<Uint<U129, Box<[u128; 2]>>>(), pointer);
  assert_eq!(layout::<Uint<U0, Box<[u64; 0]>>>(), pointer);
  assert_eq!(layout::<Int<U0, Box<[u64; 0]>>>(), pointer);
  assert_eq!(layout::<NonZeroUint<U256, Box<[u64; 4]>>>(), pointer);
  assert_eq!(layout::<NonZeroInt<U256, Box<[u64; 4]>>>(), pointer);
  assert_eq!(layout::<NonZeroUint<U20, Box<[u8; 3]>>>(), pointer);
  assert_eq!(layout::<NonZeroInt<U0, Box<[u64; 0]>>>(), pointer);
  send_and_sync::<Uint<U256, Box<[u64; 4]>>>();
  send_and_sync::<Int<U20, Box<[u8; 3]>>>();
  send_and_sync::<NonZeroUint<U256, Box<[u64; 4]>>>();
  send_and_sync::<NonZeroInt<U20, Box<[u8; 3]>>>();
}

/// Each way of building a boxed integer gives the limbs of its inline twin: zero, the limits, the
/// limbs, the slices of limbs and of bytes, the text, the casts from an inline integer, also
/// borrowed and into a native integer, the native integers and the floats, in the storages of
/// [`box_rows_agree_with_the_array_rows`], fewer under Miri; `FromLimbs` and `From` of the pointer
/// at widths that fill the array; and, at 256 bits in four 64-bit limbs, `From` of the native
/// integers, `widen` both ways, `IntegerOne`, and the arrays of bytes.
#[test]
fn construction_of_boxed_integers() {
  uint_construction::<Box<[u64; 0]>, U0, u64, 0, 1>();
  int_construction::<Box<[u64; 0]>, U0, u64, 0, 1>();
  uint_construction::<Box<[u64; 2]>, U65, u64, 2, 2>();
  int_construction::<Box<[u64; 2]>, U65, u64, 2, 2>();
  uint_construction::<Box<[u8; 3]>, U20, u8, 3, 1>();
  int_construction::<Box<[u8; 3]>, U20, u8, 3, 1>();
  filled_construction::<Box<[u64; 0]>, U0, u64, 0, 1>();
  filled_construction::<Box<[u64; 4]>, U256, u64, 4, 4>();
  if !tier().is_miri() {
    uint_construction::<Box<[u64; 1]>, U1, u64, 1, 1>();
    int_construction::<Box<[u64; 1]>, U1, u64, 1, 1>();
    uint_construction::<Box<[u64; 4]>, U200, u64, 4, 4>();
    int_construction::<Box<[u64; 4]>, U200, u64, 4, 4>();
    uint_construction::<Box<[u16; 2]>, U24, u16, 2, 1>();
    int_construction::<Box<[u16; 2]>, U24, u16, 2, 1>();
    uint_construction::<Box<[u128; 2]>, U129, u128, 2, 4>();
    int_construction::<Box<[u128; 2]>, U129, u128, 2, 4>();
    filled_construction::<Box<[u64; 1]>, U64, u64, 1, 1>();
    filled_construction::<Box<[u8; 3]>, U24, u8, 3, 1>();
    filled_construction::<Box<[u128; 2]>, U256, u128, 2, 4>();
    filled_construction::<Box<[u8; 25]>, U200, u8, 25, 4>();
  }

  type U = Uint<U256, Box<[u64; 4]>>;
  type I = Int<U256, Box<[u64; 4]>>;
  assert_eq!(
    U::from(u64::MAX).into_limbs(),
    u256::from(u64::MAX).into_limbs()
  );
  assert_eq!(U::from(true).into_limbs(), u256::ONE.into_limbs());
  assert_eq!(
    I::from(i64::MIN).into_limbs(),
    i256::from(i64::MIN).into_limbs()
  );
  assert_eq!(
    I::from(u128::MAX).into_limbs(),
    i256::from(u128::MAX).into_limbs()
  );
  assert_eq!(
    <U as IntegerOne>::one().into_limbs(),
    u256::ONE.into_limbs()
  );
  assert_eq!(
    <I as IntegerOne>::one().into_limbs(),
    i256::ONE.into_limbs()
  );
  let widened: U = u7::new(100).widen();
  assert_eq!(widened.into_limbs(), [100, 0, 0, 0]);
  let widened: I = i7::new(-50).widen();
  assert_eq!(widened.into_limbs(), i256::from(-50i8).into_limbs());
  let boxed = U::from(u64::MAX);
  let wide = Uint::<U300, [u64; 5]>::widen_from(&boxed);
  assert_eq!(wide.to_limbs(), [u64::MAX, 0, 0, 0, 0]);
  assert_eq!(
    boxed.widen::<Uint<U300, Box<[u64; 5]>>>().into_limbs(),
    wide.to_limbs()
  );
  let bytes: [u8; 32] = core::array::from_fn(|i| i as u8 * 7);
  assert_eq!(
    <U as FromByteArray>::from_le_bytes(bytes).into_limbs(),
    u256::from_le_bytes(bytes).into_limbs()
  );
  assert_eq!(
    <I as FromByteArray>::from_be_bytes(bytes).into_limbs(),
    i256::from_be_bytes(bytes).into_limbs()
  );
  assert_eq!(
    U::from_le_bytes(bytes).to_be_bytes(),
    u256::from_le_bytes(bytes).to_be_bytes()
  );
}

/// The ten compound assignments of `IntegerAssign` on a clone of `a` by a clone of `b`, and by `n`
/// for the shifts, as the limbs of the targets, each where the checked method of the operation
/// returns a value.
#[inline(never)]
fn assignments<T>(a: &T, b: &T, n: u32) -> Vec<String>
where
  T: IntegerAssign + CheckedOps + ArrayInteger<Array: Debug>,
{
  let assign = |op: fn(&mut T, T)| {
    let mut target = a.clone();
    op(&mut target, b.clone());
    limbs(target)
  };
  let shift = |op: fn(&mut T, u32)| {
    let mut target = a.clone();
    op(&mut target, n);
    limbs(target)
  };
  vec![format!(
    "{:?}",
    [
      a.checked_add(b).map(|_| assign(|t, b| *t += b)),
      a.checked_sub(b).map(|_| assign(|t, b| *t -= b)),
      a.checked_mul(b).map(|_| assign(|t, b| *t *= b)),
      a.checked_div(b).map(|_| assign(|t, b| *t /= b)),
      a.checked_rem(b).map(|_| assign(|t, b| *t %= b)),
      Some(assign(|t, b| *t &= b)),
      Some(assign(|t, b| *t |= b)),
      Some(assign(|t, b| *t ^= b)),
      a.checked_shl(n).map(|_| shift(|t, n| *t <<= n)),
      a.checked_shr(n).map(|_| shift(|t, n| *t >>= n)),
    ]
  )]
}

/// The limbs of the compound assignments of the binary operator `$op` on clones of the reference
/// `$a`, with a value and with a reference `$b`, where `$a.$checked($b)` returns a value: they
/// panic or wrap on overflow as `overflow-checks` decides.
macro_rules! assigning_forms {
  ($a:expr, $b:expr, $checked:ident, $assign:tt) => {{
    let (a, b) = ($a, $b);
    let forms = a.$checked(b).map(|_| {
      let (mut by_value, mut by_reference) = (a.clone(), a.clone());
      by_value $assign b.clone();
      by_reference $assign b;
      [limbs(by_value), limbs(by_reference)]
    });
    format!("{forms:?}")
  }};
}

/// Emits a function that gives the results of the compound assignments of `$integer<B, R>` on
/// clones of `a` by `b`, in the forms of [`assigning_forms!`], and by the amount `n` for the
/// shifts, as the limbs of the targets, each where the checked method of the operation returns a
/// value.
macro_rules! assigning_operators {
  ($name:ident, $integer:ident, $repr_mut:ident) => {
    #[inline(never)]
    fn $name<B, R>(a: &$integer<B, R>, b: &$integer<B, R>, n: u32) -> Vec<String>
    where
      B: FitsBits<R>,
      R: $repr_mut + Clone,
      $integer<B, R>: ArrayInteger<Array: Debug>,
    {
      let (mut and, mut or, mut xor) = (a.clone(), a.clone(), a.clone());
      and &= b;
      or |= b.clone();
      xor ^= b;
      let shifts = |shl: bool| {
        let mut target = a.clone();
        if shl {
          a.checked_shl(n).map(|_| {
            target <<= n;
            limbs(target)
          })
        } else {
          a.checked_shr(n).map(|_| {
            target >>= n;
            limbs(target)
          })
        }
      };
      vec![
        assigning_forms!(a, b, checked_add, +=),
        assigning_forms!(a, b, checked_sub, -=),
        assigning_forms!(a, b, checked_mul, *=),
        assigning_forms!(a, b, checked_div, /=),
        assigning_forms!(a, b, checked_rem, %=),
        format!("{:?}", [limbs(and), limbs(or), limbs(xor)]),
        format!("{:?}", (shifts(true), shifts(false))),
      ]
    }
  };
}

assigning_operators!(uint_assigning_operators, Uint, UintReprMut);
assigning_operators!(int_assigning_operators, Int, IntReprMut);

/// The results of every trait and operator of a boxed `Uint` on `$a` and `$b`, with the amount
/// `$n` in the storage `$repr`: those of [`uint_results!`], and the compound assignments.
macro_rules! boxed_uint_results {
  ($a:expr, $b:expr, $n:expr, $repr:ty) => {{
    let (a, b, n) = ($a, $b, $n);
    let mut results = uint_results!(a, b, n, $repr);
    results.extend(uint_assigning_operators::<_, $repr>(a, b, n));
    results.extend(assignments(a, b, n));
    results
  }};
}

/// The results of every trait and operator of a boxed `Int` on `$a` and `$b`, with the amount
/// `$n` in the storage `$repr`: those of [`int_results!`], and the compound assignments.
macro_rules! boxed_int_results {
  ($a:expr, $b:expr, $n:expr, $repr:ty) => {{
    let (a, b, n) = ($a, $b, $n);
    let mut results = int_results!(a, b, n, $repr);
    results.extend(int_assigning_operators::<_, $repr>(a, b, n));
    results.extend(assignments(a, b, n));
    results
  }};
}

traits_agree!(
  boxed_uint_traits_agree,
  Uint,
  false,
  boxed_uint_results,
  UintReprMut,
  IntReprMut
);
traits_agree!(
  boxed_int_traits_agree,
  Int,
  true,
  boxed_int_results,
  UintReprMut,
  IntReprMut
);

/// Every trait and operator of `Uint` and `Int` in `Box<[L; N]>` gives what it gives on the inline
/// twin in `[L; N]`, compared through the limbs of the results: the families of the overflowing,
/// checked, wrapping, saturating and strict arithmetic, the math and bit traits, the methods of
/// each sign, the text, the bytes, the casts and the conversions with the native integers and the
/// floats, the comparisons with the native integers, `Ord`, `Hash` through one hasher, the
/// operators in their four forms, the compound assignments, `IntegerAssign`, `Sum`, `Product` and
/// `IntegerOne`, in the storages of [`box_rows_agree_with_the_array_rows`]; fewer under Miri. The
/// `unchecked_*` methods are left out: they are `unsafe`.
#[test]
fn boxed_integers_agree_with_inline_integers() {
  boxed_uint_traits_agree::<Box<[u64; 0]>, U0, u64, 0, 1>();
  boxed_int_traits_agree::<Box<[u64; 0]>, U0, u64, 0, 1>();
  boxed_uint_traits_agree::<Box<[u64; 2]>, U65, u64, 2, 2>();
  boxed_int_traits_agree::<Box<[u64; 2]>, U65, u64, 2, 2>();
  products_agree::<Box<[u64; 2]>, U65, u64, 2, 2>();
  boxed_uint_traits_agree::<Box<[u8; 3]>, U20, u8, 3, 1>();
  boxed_int_traits_agree::<Box<[u8; 3]>, U20, u8, 3, 1>();
  products_agree::<Box<[u8; 3]>, U20, u8, 3, 1>();
  if tier().is_miri() {
    return;
  }
  boxed_uint_traits_agree::<Box<[u64; 1]>, U1, u64, 1, 1>();
  boxed_int_traits_agree::<Box<[u64; 1]>, U1, u64, 1, 1>();
  boxed_uint_traits_agree::<Box<[u64; 1]>, U63, u64, 1, 1>();
  boxed_int_traits_agree::<Box<[u64; 1]>, U63, u64, 1, 1>();
  products_agree::<Box<[u64; 1]>, U63, u64, 1, 1>();
  boxed_uint_traits_agree::<Box<[u64; 1]>, U64, u64, 1, 1>();
  boxed_int_traits_agree::<Box<[u64; 1]>, U64, u64, 1, 1>();
  boxed_uint_traits_agree::<Box<[u64; 2]>, U128, u64, 2, 2>();
  boxed_int_traits_agree::<Box<[u64; 2]>, U128, u64, 2, 2>();
  boxed_uint_traits_agree::<Box<[u64; 4]>, U200, u64, 4, 4>();
  boxed_int_traits_agree::<Box<[u64; 4]>, U200, u64, 4, 4>();
  products_agree::<Box<[u64; 4]>, U200, u64, 4, 4>();
  boxed_uint_traits_agree::<Box<[u64; 4]>, U256, u64, 4, 4>();
  boxed_int_traits_agree::<Box<[u64; 4]>, U256, u64, 4, 4>();
  boxed_uint_traits_agree::<Box<[u16; 2]>, U24, u16, 2, 1>();
  boxed_int_traits_agree::<Box<[u16; 2]>, U24, u16, 2, 1>();
  boxed_uint_traits_agree::<Box<[u32; 3]>, U65, u32, 3, 2>();
  boxed_int_traits_agree::<Box<[u32; 3]>, U65, u32, 3, 2>();
  products_agree::<Box<[u32; 3]>, U65, u32, 3, 2>();
  boxed_uint_traits_agree::<Box<[u128; 2]>, U129, u128, 2, 4>();
  boxed_int_traits_agree::<Box<[u128; 2]>, U129, u128, 2, 4>();
  products_agree::<Box<[u128; 2]>, U129, u128, 2, 4>();
  boxed_uint_traits_agree::<Box<[u128; 2]>, U256, u128, 2, 4>();
  boxed_int_traits_agree::<Box<[u128; 2]>, U256, u128, 2, 4>();
  boxed_uint_traits_agree::<Box<[u8; 25]>, U200, u8, 25, 4>();
  boxed_int_traits_agree::<Box<[u8; 25]>, U200, u8, 25, 4>();
  if tier().is_full() {
    boxed_uint_traits_agree::<Box<[u64; 16]>, U1024, u64, 16, 16>();
    boxed_int_traits_agree::<Box<[u64; 16]>, U1024, u64, 16, 16>();
    products_agree::<Box<[u64; 16]>, U1024, u64, 16, 16>();
  }
}

/// The results of the arithmetic of `Uint` on `$a` and `$b`, with the amount `$n`: the families,
/// the math functions and the methods of `Uint` alone, as the functions above give them.
macro_rules! uint_limits {
  ($a:expr, $b:expr, $n:expr) => {{
    let (a, b, n) = ($a, $b, $n);
    let mut results = arithmetic_families(a, b, n);
    results.extend(math(a, b, n));
    results.extend(unsigned_methods(a, b));
    results
  }};
}

/// The results of the arithmetic of `Int` on `$a` and `$b`, with the amount `$n`: the families,
/// the math functions and the methods of `Int` alone, as the functions above give them.
macro_rules! int_limits {
  ($a:expr, $b:expr, $n:expr) => {{
    let (a, b, n) = ($a, $b, $n);
    let mut results = arithmetic_families(a, b, n);
    results.extend(math(a, b, n));
    results.extend(signed_methods(a));
    results
  }};
}

/// Emits a function that checks `$integer` of `B` bits in `Box<[L; N]>` against its twin in
/// `[L; N]`, read in two's complement if `$signed`, through the arithmetic that `$results!` gives,
/// at every pair of the [`pattern_ends`], the limits of both signs among them, with every amount
/// of [`row_amounts`]: the operands at which a method returns `None`, saturates or wraps, whatever
/// [`row_pairs`] takes. The function is `#[inline(never)]`, so that its frame holds the values of
/// this storage alone.
macro_rules! limits_agree {
  ($name:ident, $integer:ident, $signed:literal, $results:ident) => {
    #[inline(never)]
    fn $name<B, L, const N: usize>()
    where
      B: FitsBits<[L; N]>,
      L: ArrayLimb + bigint::Limb + Debug,
      [L; N]: ArrayRepr,
    {
      let bits = B::U32;
      let values: Vec<[L; N]> = pattern_ends(bits)
        .iter()
        .map(|pattern| bigint::limbs_of_pattern(pattern, $signed))
        .collect();
      let twin = |array| <$integer<B, [L; N]> as ArrayInteger>::checked_from_limbs(array);
      let boxed = |array| <$integer<B, Box<[L; N]>> as ArrayInteger>::checked_from_limbs(array);
      for &x in &values {
        for &y in &values {
          let (a, b) = (twin(x).expect("in range"), twin(y).expect("in range"));
          let (c, d) = (boxed(x).expect("in range"), boxed(y).expect("in range"));
          for n in row_amounts(bits) {
            assert_eq!(
              $results!(&c, &d, n),
              $results!(&a, &b, n),
              "{x:?} and {y:?} at {bits} bits with {n}"
            );
          }
        }
      }
    }
  };
}

limits_agree!(uint_limits_agree, Uint, false, uint_limits);
limits_agree!(int_limits_agree, Int, true, int_limits);

/// At the limits, the arithmetic of `Uint` and `Int` in `Box<[L; N]>` gives what it gives on the
/// inline twin: at every pair of zero, one, the top bit alone, every bit but the top one and every
/// bit, with every amount of [`row_amounts`], through the families of the arithmetic, the math
/// functions and the methods of each sign. There each checked method returns `None`, each
/// saturating one saturates, and the carry, the borrow, `div_exact` and the saturating casts
/// between the signs take each of their paths; at zero bits, at one bit, and at partial and full
/// widths of the limbs. Under Miri, zero bits and one bit.
#[test]
fn boxed_integers_agree_at_the_limits() {
  uint_limits_agree::<U0, u64, 0>();
  int_limits_agree::<U0, u64, 0>();
  uint_limits_agree::<U1, u64, 1>();
  int_limits_agree::<U1, u64, 1>();
  if tier().is_miri() {
    return;
  }
  uint_limits_agree::<U65, u64, 2>();
  int_limits_agree::<U65, u64, 2>();
  uint_limits_agree::<U20, u8, 3>();
  int_limits_agree::<U20, u8, 3>();
  uint_limits_agree::<U64, u64, 1>();
  int_limits_agree::<U64, u64, 1>();
  uint_limits_agree::<U256, u64, 4>();
  int_limits_agree::<U256, u64, 4>();
  uint_limits_agree::<U129, u128, 2>();
  int_limits_agree::<U129, u128, 2>();
}

/// The windows of limbs at which the folds of `bits` bits are checked, read in two's complement if
/// `signed`: no item, and every window of one, two and three of the [`pattern_ends`], the limits
/// among them; or under Miri every fifth of those, with every window of at most one item.
fn fold_windows<L: bigint::Limb, const N: usize>(bits: u32, signed: bool) -> Vec<Vec<[L; N]>> {
  let values: Vec<[L; N]> = pattern_ends(bits)
    .iter()
    .map(|pattern| bigint::limbs_of_pattern(pattern, signed))
    .collect();
  let mut windows = vec![Vec::new()];
  // The windows of the length before, from which those of the next length grow.
  let mut start = 0;
  for _ in 0..3 {
    let end = windows.len();
    for k in start..end {
      for &value in &values {
        let mut window = windows[k].clone();
        window.push(value);
        windows.push(window);
      }
    }
    start = end;
  }
  bigint::thin_by(windows, 5, |window| window.len() < 2)
}

/// What a fold gives: the limbs of its result, or the message of its panic.
fn fold_outcome<T: ArrayInteger>(fold: impl FnOnce() -> T) -> Result<T::Array, String> {
  let mut result = None;
  match panic_of(|| result = Some(fold())) {
    Some((message, _, _)) => Err(message),
    None => Ok(limbs(result.expect("a result"))),
  }
}

/// What `Sum` over `items` gives, of values and of references, as [`fold_outcome`] gives it.
#[inline(never)]
fn window_sums<T>(items: &[T]) -> String
where
  T: Clone + Sum + for<'x> Sum<&'x T> + ArrayInteger<Array: Debug>,
{
  format!(
    "{:?}",
    [
      fold_outcome(|| items.iter().cloned().sum::<T>()),
      fold_outcome(|| items.iter().sum::<T>()),
    ]
  )
}

/// What `Product` over `items` gives, of values and of references, as [`fold_outcome`] gives it.
#[inline(never)]
fn window_products<T>(items: &[T]) -> String
where
  T: Clone + Product + for<'x> Product<&'x T> + ArrayInteger<Array: Debug>,
{
  format!(
    "{:?}",
    [
      fold_outcome(|| items.iter().cloned().product::<T>()),
      fold_outcome(|| items.iter().product::<T>()),
    ]
  )
}

/// What a fold of `items` must give, of values and of references alike, from the overflowing
/// method `step` of each of its steps, on `start`, what no item gives: the total, or, where a step
/// overflows, the panic `message` of the native operator if the build `checks` for overflow, and
/// the wrapped total if it does not.
fn expected_fold<T>(
  items: &[T],
  start: T,
  step: impl Fn(&T, &T) -> (T, bool),
  message: &str,
  checks: bool,
) -> String
where
  T: ArrayInteger<Array: Debug>,
{
  let mut overflow = false;
  let total = items.iter().fold(start, |total, item| {
    let (total, flag) = step(&total, item);
    overflow |= flag;
    total
  });
  let outcome: Result<T::Array, String> = if overflow && checks {
    Err(message.to_string())
  } else {
    Ok(limbs(total))
  };
  format!("{:?}", [&outcome, &outcome])
}

/// Emits two functions that check the folds of `$integer` of `B` bits in `Box<[L; N]>` and of its
/// twin in `[L; N]`, read in two's complement if `$signed`, of values and of references, at the
/// windows of [`fold_windows`]: `$sums` checks `Sum`, and `$products` checks `Product`, at a width
/// of two bits or more, which `Int` needs for one. Each fold gives what [`expected_fold`] gives
/// from the overflowing addition or multiplication of the twin. Each function is
/// `#[inline(never)]`, so that its frame holds the values of this storage alone.
macro_rules! folds_agree {
  ($sums:ident, $products:ident, $integer:ident, $signed:literal) => {
    #[inline(never)]
    fn $sums<B, L, const N: usize>()
    where
      B: FitsBits<[L; N]>,
      L: ArrayLimb + bigint::Limb + Debug,
      [L; N]: ArrayRepr,
    {
      let checks = overflow_checks();
      let twin = |array| <$integer<B, [L; N]> as ArrayInteger>::checked_from_limbs(array);
      let boxed = |array| <$integer<B, Box<[L; N]>> as ArrayInteger>::checked_from_limbs(array);
      for window in fold_windows::<L, N>(B::U32, $signed) {
        let twins: Vec<_> = window.iter().map(|&x| twin(x).expect("in range")).collect();
        let boxes: Vec<_> = window
          .iter()
          .map(|&x| boxed(x).expect("in range"))
          .collect();
        let expected = expected_fold(
          &twins,
          FixedInteger::zero(),
          |total, item| total.overflowing_add(item),
          "attempt to add with overflow",
          checks,
        );
        assert_eq!(
          (window_sums(&boxes), window_sums(&twins)),
          (expected.clone(), expected),
          "the sum of {window:?}"
        );
      }
    }

    #[inline(never)]
    fn $products<B, L, const N: usize>()
    where
      B: FitsBits<[L; N]> + NonZero + IsGreater<U1, Output = True>,
      L: ArrayLimb + bigint::Limb + Debug,
      [L; N]: ArrayRepr,
    {
      let checks = overflow_checks();
      let twin = |array| <$integer<B, [L; N]> as ArrayInteger>::checked_from_limbs(array);
      let boxed = |array| <$integer<B, Box<[L; N]>> as ArrayInteger>::checked_from_limbs(array);
      for window in fold_windows::<L, N>(B::U32, $signed) {
        let twins: Vec<_> = window.iter().map(|&x| twin(x).expect("in range")).collect();
        let boxes: Vec<_> = window
          .iter()
          .map(|&x| boxed(x).expect("in range"))
          .collect();
        let expected = expected_fold(
          &twins,
          IntegerOne::one(),
          |total, item| total.overflowing_mul(item),
          "attempt to multiply with overflow",
          checks,
        );
        assert_eq!(
          (window_products(&boxes), window_products(&twins)),
          (expected.clone(), expected),
          "the product of {window:?}"
        );
      }
    }
  };
}

folds_agree!(uint_sums_agree, uint_products_agree, Uint, false);
folds_agree!(int_sums_agree, int_products_agree, Int, true);

/// `Sum` and `Product` of boxed integers and of their inline twins, of values and of references,
/// give what the overflowing addition and multiplication give step by step: over no item, zero or
/// one, and over every window of one, two and three of zero, one, the top bit alone, every bit but
/// the top one and every bit. Where a step overflows, a build that checks for overflow panics with
/// the message of the native operator, and another returns the wrapped total. The sums at zero
/// bits, at one bit and at wider widths, and the products from two bits, in both signs; under
/// Miri, the sums at zero bits and at one bit and both folds at 65 bits.
#[test]
fn boxed_folds_agree_with_inline_folds() {
  uint_sums_agree::<U0, u64, 0>();
  int_sums_agree::<U0, u64, 0>();
  uint_sums_agree::<U1, u64, 1>();
  int_sums_agree::<U1, u64, 1>();
  uint_sums_agree::<U65, u64, 2>();
  int_sums_agree::<U65, u64, 2>();
  uint_products_agree::<U65, u64, 2>();
  int_products_agree::<U65, u64, 2>();
  if tier().is_miri() {
    return;
  }
  uint_products_agree::<U2, u8, 1>();
  int_products_agree::<U2, u8, 1>();
  uint_sums_agree::<U20, u8, 3>();
  int_sums_agree::<U20, u8, 3>();
  uint_products_agree::<U20, u8, 3>();
  int_products_agree::<U20, u8, 3>();
  uint_sums_agree::<U256, u64, 4>();
  int_sums_agree::<U256, u64, 4>();
  uint_products_agree::<U256, u64, 4>();
  int_products_agree::<U256, u64, 4>();
}

/// The byte order of `a` through `IntegerByteOrder`, as the limbs of the integers that it returns.
#[inline(never)]
fn byte_order<T>(a: &T) -> String
where
  T: IntegerByteOrder + Clone + ArrayInteger<Array: Debug>,
{
  format!(
    "{:?}",
    [
      limbs(a.swap_bytes()),
      limbs(a.to_be()),
      limbs(a.to_le()),
      limbs(T::from_be(a.clone())),
      limbs(T::from_le(a.clone())),
    ]
  )
}

/// Checks the byte order of `Uint` and `Int` of `B` bits, a width of whole bytes, in `Box<[L; N]>`
/// against that of their twins in `[L; N]`, at the patterns of [`row_patterns`] in `K` words, and
/// that `from_be` and `from_le` store their result in the allocation of the integer that they
/// take, whichever of them swaps the bytes on the target. The function is `#[inline(never)]`, so
/// that its frame holds the values of this storage alone.
#[inline(never)]
fn byte_order_agrees<B, L, const N: usize, const K: usize>()
where
  B: FitsBits<[L; N]> + WholeBytes,
  L: ArrayLimb + bigint::Limb + Debug,
  [L; N]: ArrayRepr,
{
  for signed in [false, true] {
    for pattern in row_patterns::<K>(B::U32, signed) {
      let array: [L; N] = bigint::limbs_of_pattern(&pattern, signed);
      if signed {
        let twin = <Int<B, [L; N]> as ArrayInteger>::checked_from_limbs(array);
        let boxed = <Int<B, Box<[L; N]>> as ArrayInteger>::checked_from_limbs(array);
        let (twin, boxed) = (twin.expect("in range"), boxed.expect("in range"));
        assert_eq!(byte_order(&boxed), byte_order(&twin), "{array:?}");
        let before = place(&boxed);
        let stored = IntegerByteOrder::from_le(IntegerByteOrder::from_be(boxed));
        assert_eq!(place(&stored), before, "{array:?}");
      } else {
        let twin = <Uint<B, [L; N]> as ArrayInteger>::checked_from_limbs(array);
        let boxed = <Uint<B, Box<[L; N]>> as ArrayInteger>::checked_from_limbs(array);
        let (twin, boxed) = (twin.expect("in range"), boxed.expect("in range"));
        assert_eq!(byte_order(&boxed), byte_order(&twin), "{array:?}");
        let before = place(&boxed);
        let stored = IntegerByteOrder::from_le(IntegerByteOrder::from_be(boxed));
        assert_eq!(place(&stored), before, "{array:?}");
      }
    }
  }
}

/// The byte order of a boxed integer through `IntegerByteOrder`, `swap_bytes`, `to_be`, `to_le`,
/// `from_be` and `from_le`, is that of its inline twin, and `from_be` and `from_le` store their
/// result in the allocation of the integer that they take: at zero bits, and at widths of whole
/// bytes that fill their limbs or leave a part of the top one, in limbs of each size. Under Miri,
/// zero bits and 24 bits in two 16-bit limbs.
#[test]
fn boxed_byte_order_is_that_of_inline_integers() {
  byte_order_agrees::<U0, u64, 0, 1>();
  byte_order_agrees::<U24, u16, 2, 1>();
  if tier().is_miri() {
    return;
  }
  byte_order_agrees::<U64, u64, 1, 1>();
  byte_order_agrees::<U128, u64, 2, 2>();
  byte_order_agrees::<U200, u64, 4, 4>();
  byte_order_agrees::<U256, u128, 2, 4>();
  byte_order_agrees::<U200, u8, 25, 4>();
}

/// Asserts that `$target $op $rhs`, a compound assignment or a call that writes its target, keeps
/// the allocation of `$target`.
macro_rules! keeps {
  ($target:ident $op:tt $rhs:expr) => {{
    let before = place(&$target);
    $target $op $rhs;
    assert_eq!(place(&$target), before, "{}", stringify!($target $op $rhs));
  }};
}

/// Asserts that `$expr` stores its result in the allocation of `$operand`, which it consumes.
macro_rules! stored_in {
  ($operand:ident, $expr:expr) => {{
    let before = place(&$operand);
    let result = $expr;
    assert_eq!(place(&result), before, "{}", stringify!($expr));
  }};
}

/// Each compound assignment of a boxed integer, with a value and with a reference, and by a
/// non-zero divisor, `set_bit` and `clone_from` store in the allocation of their target; each
/// operator with an owned operand stores its result in the allocation of that operand, the left
/// one when both are owned, and `&a / nz` and `&a % nz` in that of the divisor; the casts between
/// the signs, the non-zero `new`, `get`, `TryFrom`, `cast_signed` and `cast_unsigned`, and `From`
/// into and out of the pointer keep it; the saturating casts between the signs, also where they
/// saturate, `from_be` and `from_le` store in the allocation of the integer that they take, and
/// `Sum` and `Product` of values in that of their first item; and an assignment that panics, on
/// a zero divisor or on overflow with overflow checks, leaves the value of its target.
#[test]
fn boxed_operations_keep_their_allocation() {
  type U = Uint<U256, Box<[u64; 4]>>;
  type I = Int<U200, Box<[u64; 4]>>;
  type Nz = NonZeroUint<U256, Box<[u64; 4]>>;
  let (seven, three) = (U::from(7u64), U::from(3u64));
  let divisor = Nz::new(U::from(3u64)).expect("not zero");
  let mut a = U::from(1_000_000u64);
  keeps!(a += seven.clone());
  keeps!(a += &seven);
  keeps!(a -= seven.clone());
  keeps!(a -= &seven);
  keeps!(a *= three.clone());
  keeps!(a *= &three);
  keeps!(a /= three.clone());
  keeps!(a /= &three);
  keeps!(a %= U::from(1_000u64));
  keeps!(a %= &seven);
  keeps!(a &= seven.clone());
  keeps!(a |= &three);
  keeps!(a ^= seven.clone());
  keeps!(a <<= 9u32);
  keeps!(a >>= 2i64);
  keeps!(a /= divisor.clone());
  keeps!(a /= &divisor);
  keeps!(a %= divisor.clone());
  keeps!(a %= &divisor);
  let before = place(&a);
  a.set_bit(100, true);
  a.clone_from(&seven);
  assert_eq!((place(&a), a.to_limbs()), (before, [7, 0, 0, 0]));
  let mut b = I::from(-1_000_000i64);
  keeps!(b -= I::from(5i64));
  keeps!(b *= &I::from(-3i64));
  keeps!(b /= I::from(7i64));
  keeps!(b %= &I::from(1_000i64));
  keeps!(b <<= 3u8);

  let fresh = || (U::from(1_000_000u64), U::from(7u64));
  let (x, y) = fresh();
  stored_in!(x, x + y);
  let (x, y) = fresh();
  stored_in!(x, x - &y);
  let (x, y) = fresh();
  stored_in!(y, &x * y);
  let (x, y) = fresh();
  stored_in!(x, x / y);
  let (x, y) = fresh();
  stored_in!(y, &x % y);
  let (x, y) = fresh();
  stored_in!(x, x & y);
  let (x, y) = fresh();
  stored_in!(y, &x | y);
  let (x, y) = fresh();
  stored_in!(x, x ^ &y);
  let (x, _) = fresh();
  stored_in!(x, !x);
  let (x, _) = fresh();
  stored_in!(x, x << 3u32);
  let (x, _) = fresh();
  stored_in!(x, x >> 3u32);
  let (x, _) = fresh();
  stored_in!(x, x / divisor.clone());
  let (x, _) = fresh();
  stored_in!(x, x % &divisor);
  let x = I::from(-1_000_000i64);
  stored_in!(x, -x);
  let integer = U::from(3u64);
  let before = place(&integer);
  let nonzero = Nz::new(integer).expect("not zero");
  let (x, _) = fresh();
  assert_eq!(place(&(&x / nonzero)), before, "&a / nz");
  let integer = U::from(3u64);
  let before = place(&integer);
  let nonzero = Nz::new(integer).expect("not zero");
  assert_eq!(place(&(&x % nonzero)), before, "&a % nz");

  // The casts and the conversions with the non-zero integers move the pointer.
  let (x, _) = fresh();
  let before = place(&x);
  let signed = x.cast_signed();
  assert_eq!(place(&signed), before);
  let unsigned = signed.cast_unsigned();
  assert_eq!(place(&unsigned), before);
  let nonzero = Nz::try_from(unsigned).expect("not zero");
  let signed = nonzero.cast_signed();
  let unsigned = signed.cast_unsigned().get();
  assert_eq!(place(&unsigned), before);
  let nonzero = Nz::new(unsigned).expect("not zero");
  assert_eq!(place(&nonzero.get()), before);
  let pointer = Box::new([1, 2, 3, 4]);
  let before: *const [u64; 4] = &*pointer;
  let integer = U::from(pointer);
  assert_eq!(place(&integer), before);
  let pointer = Box::<[u64; 4]>::from(integer);
  assert_eq!(&*pointer as *const [u64; 4], before);

  // The saturating casts store what they return, the bound of the other sign too, in the
  // allocation that they take.
  let x = U::max_value();
  let before = place(&x);
  let signed = x.saturating_cast_signed();
  assert_eq!(
    (place(&signed), signed.to_limbs()),
    (before, i256::MAX.to_limbs())
  );
  let negative = !signed;
  let before = place(&negative);
  let unsigned = negative.saturating_cast_unsigned();
  assert_eq!((place(&unsigned), unsigned.to_limbs()), (before, [0; 4]));
  let signed = U::from(7u64).saturating_cast_signed();
  let before = place(&signed);
  let unsigned = signed.saturating_cast_unsigned();
  assert_eq!(
    (place(&unsigned), unsigned.to_limbs()),
    (before, [7, 0, 0, 0])
  );

  // One of `from_be` and `from_le` swaps the bytes on every target, in the allocation that it
  // takes, and the other returns its integer.
  let x = U::from(0x0102u64);
  let before = place(&x);
  let swapped = U::from_le(U::from_be(x));
  assert_eq!(
    (place(&swapped), swapped.to_limbs()),
    (before, u256::from(0x0102u64).swap_bytes().to_limbs())
  );

  // The folds of values store their total in the first item.
  let items = [U::from(5u64), U::from(6u64), U::from(7u64)];
  let before = place(&items[0]);
  let sum: U = items.into_iter().sum();
  assert_eq!((place(&sum), sum.to_limbs()), (before, [18, 0, 0, 0]));
  let items = [U::from(5u64), U::from(6u64), U::from(7u64)];
  let before = place(&items[0]);
  let product: U = items.into_iter().product();
  assert_eq!(
    (place(&product), product.to_limbs()),
    (before, [210, 0, 0, 0])
  );
  let items = [I::from(-5i64), I::from(6i64)];
  let before = place(&items[0]);
  let sum: I = items.into_iter().sum();
  assert_eq!((place(&sum), sum.to_limbs()), (before, [1, 0, 0, 0]));
  let items = [I::from(-5i64), I::from(6i64)];
  let before = place(&items[0]);
  let product: I = items.into_iter().product();
  assert_eq!((place(&product), product), (before, I::from(-30i64)));

  // A panicking assignment leaves its target.
  let checks = overflow_checks();
  let mut target = U::from(1_000u64);
  assert!(panic_of(|| target /= U::from(0u64)).is_some());
  assert!(panic_of(|| target %= &U::from(0u64)).is_some());
  let panic = panic_of(|| target += <U as FixedInteger>::max_value());
  assert_eq!(panic.is_some(), checks);
  let expected = if checks { 1_000u64 } else { 999 };
  assert_eq!(target.to_limbs(), [expected, 0, 0, 0]);
  let mut target = <I as FixedInteger>::min_value();
  let panic = panic_of(|| target -= &I::from(1i64));
  assert_eq!(panic.is_some(), checks);
  let expected = if checks {
    <I as FixedInteger>::min_value()
  } else {
    <I as FixedInteger>::max_value()
  };
  assert_eq!(target, expected);
}

/// The operators with an owned operand and the compound assignments of a boxed integer overflow as
/// those of its inline twin: with overflow checks they panic with the messages of the native
/// integers and report the line of the caller, a panicking assignment leaving its target, and
/// without them they wrap; `/` and `%` panic on a zero divisor in every build.
#[test]
fn boxed_overflow_follows_the_checks() {
  type U = Uint<U256, Box<[u64; 4]>>;
  type I = Int<U200, Box<[u64; 4]>>;
  let add = "attempt to add with overflow".to_string();
  let sub = "attempt to subtract with overflow".to_string();
  let mul = "attempt to multiply with overflow".to_string();
  let neg = "attempt to negate with overflow".to_string();
  let div = "attempt to divide by zero".to_string();
  let rem = "attempt to calculate the remainder with a divisor of zero".to_string();
  let (m, o, z) = (U::max_value(), U::from(1u64), U::from(0u64));
  let two = U::from(2u64);
  overflows_here!(m.clone() + o.clone(), add, z.clone());
  overflows_here!(m.clone() + &o, add, z.clone());
  overflows_here!(&m + o.clone(), add, z.clone());
  overflows_here!(z.clone() - o.clone(), sub, m.clone());
  overflows_here!(&z - o.clone(), sub, m.clone());
  overflows_here!(m.clone() * two.clone(), mul, m.clone() - &o);
  overflows_here!(&m * two.clone(), mul, m.clone() - &o);
  panics_here!(m.clone() / z.clone(), div);
  panics_here!(&m / z.clone(), div);
  panics_here!(m.clone() % &z, rem);
  let (min, minus_one) = (I::min_value(), I::from(-1i64));
  overflows_here!(-min.clone(), neg, min.clone());
  overflows_here!(min.clone() - &I::from(1i64), sub, I::max_value());
  overflows_here!(min.clone() * minus_one.clone(), mul, min.clone());

  let checks = overflow_checks();
  // `$target $op $rhs` panics with `$message` and reports the line of the invocation with overflow
  // checks, and leaves `$target`, and without them it stores `$wrapped`.
  macro_rules! assigns_here {
    ($target:ident $op:tt $rhs:expr, $message:expr, $wrapped:expr) => {{
      let before = $target.clone();
      match panic_of(|| $target $op $rhs) {
        Some(panic) => {
          assert!(checks, "{} panics without overflow checks", stringify!($op));
          assert_eq!(panic, ($message.clone(), file!().to_string(), line!()));
          assert_eq!($target, before);
        }
        None => {
          assert!(!checks, "{} returns with overflow checks", stringify!($op));
          assert_eq!($target, $wrapped);
        }
      }
    }};
  }
  let mut t = m.clone();
  assigns_here!(t += o.clone(), add, z.clone());
  let mut t = z.clone();
  assigns_here!(t -= &o, sub, m.clone());
  let mut t = m.clone();
  assigns_here!(t *= &two, mul, m.clone() - &o);
  let mut t = min.clone();
  assigns_here!(t *= minus_one.clone(), mul, min.clone());
  let mut t = U::from(1u128 << 64);
  panics_here!(t /= z.clone(), div);
  panics_here!(t %= &z, rem);
  assert_eq!(t.to_limbs(), [0, 1, 0, 0]);
}

/// The methods of a boxed integer that panic at the limits panic as those of its inline twin do,
/// with the messages of the native integers, and report the line of the caller. In every build:
/// the strict methods on overflow and on a zero divisor, `saturating_div` on a zero divisor,
/// `div_exact` on a zero divisor and on `MIN / -1`, and `/` and `%` on two references. With
/// overflow checks, without which they return the wrapped value: `abs`, `pow`, which reports a
/// line of `core`, `+`, `-` and `*` on two references, and the negation of a reference, of an
/// integer and of a non-zero integer. At a full and at a partial width of the limbs, at one signed
/// bit, where -1 is `MIN`, and at zero bits, where `0^0` overflows and every divisor is zero.
#[test]
fn boxed_methods_panic_at_the_limits() {
  type U = Uint<U256, Box<[u64; 4]>>;
  type I = Int<U200, Box<[u64; 4]>>;
  let add = "attempt to add with overflow".to_string();
  let sub = "attempt to subtract with overflow".to_string();
  let mul = "attempt to multiply with overflow".to_string();
  let neg = "attempt to negate with overflow".to_string();
  let power = "attempt to exponentiate with overflow".to_string();
  let shl = "attempt to shift left with overflow".to_string();
  let shr = "attempt to shift right with overflow".to_string();
  let div = "attempt to divide with overflow".to_string();
  let rem = "attempt to calculate the remainder with overflow".to_string();
  let by_zero = "attempt to divide by zero".to_string();
  let rem_by_zero = "attempt to calculate the remainder with a divisor of zero".to_string();
  // The message of the native `pow` that overflows, which only a build with checks has.
  let pow = panic_of(|| black_box(u8::MAX).pow(2)).map_or_else(String::new, |panic| panic.0);

  let (m, o, z) = (U::max_value(), U::from(1u64), U::from(0u64));
  let two = U::from(2u64);
  panics_here!(m.strict_add(&o), add);
  panics_here!(z.strict_sub(&o), sub);
  panics_here!(m.strict_mul(&two), mul);
  panics_here!(o.strict_neg(), neg);
  panics_here!(m.strict_pow(2), power);
  panics_here!(m.strict_shl(256), shl);
  panics_here!(m.strict_shr(256), shr);
  panics_here!(m.strict_div(&z), by_zero);
  panics_here!(m.strict_rem(&z), rem_by_zero);
  panics_here!(m.strict_div_euclid(&z), by_zero);
  panics_here!(m.strict_rem_euclid(&z), rem_by_zero);
  panics_here!(m.saturating_div(&z), by_zero);
  panics_here!(m.div_exact(&z), rem_by_zero);
  overflows_here!(m.pow(2), pow, o.clone(), in core);
  overflows_here!(&m + &o, add, z.clone());
  overflows_here!(&z - &o, sub, m.clone());
  overflows_here!(&m * &two, mul, m.clone() - &o);
  panics_here!(&m / &z, by_zero);
  panics_here!(&m % &z, rem_by_zero);

  let (min, max) = (I::min_value(), I::max_value());
  let (zero, one, minus_one) = (I::from(0i64), I::from(1i64), I::from(-1i64));
  panics_here!(max.strict_add(&one), add);
  panics_here!(min.strict_sub(&one), sub);
  panics_here!(min.strict_mul(&minus_one), mul);
  panics_here!(min.strict_neg(), neg);
  panics_here!(min.strict_abs(), neg);
  panics_here!(min.strict_pow(2), power);
  panics_here!(min.strict_shl(200), shl);
  panics_here!(min.strict_shr(200), shr);
  panics_here!(min.strict_div(&minus_one), div);
  panics_here!(min.strict_rem(&minus_one), rem);
  panics_here!(min.strict_div_euclid(&minus_one), div);
  panics_here!(min.strict_rem_euclid(&minus_one), rem);
  panics_here!(min.div_exact(&minus_one), rem);
  panics_here!(min.strict_div(&zero), by_zero);
  panics_here!(min.strict_rem_euclid(&zero), rem_by_zero);
  panics_here!(min.saturating_div(&zero), by_zero);
  panics_here!(min.div_exact(&zero), rem_by_zero);
  assert_eq!(min.saturating_div(&minus_one), max);
  overflows_here!(min.abs(), neg, min.clone());
  overflows_here!(min.pow(2), pow, zero.clone(), in core);
  overflows_here!(-&min, neg, min.clone());
  overflows_here!(&max + &one, add, min.clone());
  overflows_here!(&min - &one, sub, max.clone());
  overflows_here!(&min * &minus_one, mul, min.clone());
  panics_here!(&min / &minus_one, div);
  panics_here!(&min % &minus_one, rem);
  panics_here!(&min / &zero, by_zero);
  panics_here!(&min % &zero, rem_by_zero);
  let nonzero = NonZeroInt::<U200, Box<[u64; 4]>>::new(min.clone()).expect("not zero");
  overflows_here!(nonzero.abs(), neg, nonzero.clone());
  overflows_here!(-&nonzero, neg, nonzero.clone());
  overflows_here!(-nonzero.clone(), neg, nonzero.clone());

  // One bit holds only -1 and 0, so -1 is `MIN`, and its square and its quotient by itself, both
  // one, overflow.
  type Bit = Int<U1, Box<[u64; 1]>>;
  let bit = Bit::min_value();
  panics_here!(bit.strict_add(&bit), add);
  panics_here!(bit.strict_mul(&bit), mul);
  panics_here!(bit.strict_neg(), neg);
  panics_here!(bit.strict_abs(), neg);
  panics_here!(bit.strict_pow(2), power);
  panics_here!(bit.strict_div(&bit), div);
  panics_here!(bit.div_exact(&bit), rem);
  overflows_here!(bit.abs(), neg, bit.clone());
  overflows_here!(-&bit, neg, bit.clone());
  overflows_here!(&bit + &bit, add, Bit::zero());
  overflows_here!(&bit * &bit, mul, bit.clone());
  let nonzero = NonZeroInt::<U1, Box<[u64; 1]>>::new(bit.clone()).expect("not zero");
  overflows_here!(nonzero.abs(), neg, nonzero.clone());
  overflows_here!(-&nonzero, neg, nonzero.clone());

  // Zero bits hold only zero.
  type Empty = Uint<U0, Box<[u64; 0]>>;
  let (none, nothing) = (Empty::zero(), Empty::zero());
  panics_here!(none.strict_pow(0), power);
  panics_here!(none.strict_shl(0), shl);
  panics_here!(none.strict_shr(0), shr);
  panics_here!(none.strict_div(&nothing), by_zero);
  panics_here!(none.div_exact(&nothing), rem_by_zero);
  panics_here!(&none / &nothing, by_zero);
  overflows_here!(none.pow(0), pow, nothing.clone(), in core);
}

/// The non-zero integers over a box give what their twins over the array give: through their
/// traits, `TryFrom` of the integer and the limits, in storages of `[u64; N]` and of bytes, fewer
/// under Miri; and the operators, the division and the remainder of the integer by a non-zero
/// integer and `|` in every form, with the assignments, and the negation, at 256 and 200 bits; and
/// `Option` of a non-zero integer has the size of a pointer.
#[test]
fn boxed_nonzero_integers() {
  nonzero_uint_agree::<Box<[u64; 2]>, U65, u64, 2, 2>();
  nonzero_int_agree::<Box<[u64; 2]>, U65, u64, 2, 2>();
  nonzero_uint_agree::<Box<[u8; 3]>, U20, u8, 3, 1>();
  nonzero_int_agree::<Box<[u8; 3]>, U20, u8, 3, 1>();
  if !tier().is_miri() {
    nonzero_uint_agree::<Box<[u64; 1]>, U1, u64, 1, 1>();
    nonzero_int_agree::<Box<[u64; 1]>, U1, u64, 1, 1>();
    nonzero_uint_agree::<Box<[u64; 4]>, U200, u64, 4, 4>();
    nonzero_int_agree::<Box<[u64; 4]>, U200, u64, 4, 4>();
    nonzero_uint_agree::<Box<[u128; 2]>, U129, u128, 2, 4>();
    nonzero_int_agree::<Box<[u128; 2]>, U129, u128, 2, 4>();
  }

  type U = Uint<U256, Box<[u64; 4]>>;
  type Nz = NonZeroUint<U256, Box<[u64; 4]>>;
  type Twin = NonZeroUint<U256, [u64; 4]>;
  let (x, twin_x) = (U::from(1_000_003u64), u256::from(1_000_003u64));
  let a = Nz::new(U::from(7u64)).expect("not zero");
  let b = Nz::new(U::from(0x10u64)).expect("not zero");
  let (twin_a, twin_b) = (Twin::new(u256::from(7u64)), Twin::new(u256::from(0x10u64)));
  let (twin_a, twin_b) = (twin_a.expect("not zero"), twin_b.expect("not zero"));
  let mut quotient = x.clone();
  quotient /= a.clone();
  quotient /= &a;
  let mut remainder = x.clone();
  remainder %= &b;
  remainder %= a.clone();
  let mut or = a.clone();
  or |= b.clone();
  or |= &x;
  assert_eq!(
    [
      limbs(x.clone() / a.clone()),
      limbs(x.clone() / &a),
      limbs(&x / a.clone()),
      limbs(&x / &a),
      limbs(x.clone() % b.clone()),
      limbs(&x % &b),
      limbs(quotient),
      limbs(remainder),
    ],
    [
      (twin_x / twin_a).to_limbs(),
      (twin_x / twin_a).to_limbs(),
      (twin_x / twin_a).to_limbs(),
      (twin_x / twin_a).to_limbs(),
      (twin_x % twin_b).to_limbs(),
      (twin_x % twin_b).to_limbs(),
      (twin_x / twin_a / twin_a).to_limbs(),
      (twin_x % twin_b % twin_a).to_limbs(),
    ]
  );
  assert_eq!(
    [
      limbs((a.clone() | b.clone()).get()),
      limbs((&a | &b).get()),
      limbs((a.clone() | &x).get()),
      limbs((&x | a.clone()).get()),
      limbs(or.get()),
    ],
    [
      (twin_a | twin_b).get().to_limbs(),
      (twin_a | twin_b).get().to_limbs(),
      (twin_a | twin_x).get().to_limbs(),
      (twin_x | twin_a).get().to_limbs(),
      (twin_a | twin_b | twin_x).get().to_limbs(),
    ]
  );
  type I = Int<U200, Box<[u64; 4]>>;
  let minus_seven = NonZeroInt::<U200, Box<[u64; 4]>>::new(I::from(-7i64)).expect("not zero");
  assert_eq!(limbs((-&minus_seven).get()), I::from(7i64).into_limbs());
  assert_eq!(limbs((-minus_seven).get()), I::from(7i64).into_limbs());
  assert_eq!(size_of::<Option<Nz>>(), size_of::<usize>());
  assert_eq!(
    size_of::<Option<NonZeroInt<U200, Box<[u64; 4]>>>>(),
    size_of::<usize>()
  );
}

/// Emits a function that checks `$nonzero` of `B` bits in `Box<[L; N]>` against its twin in
/// `[L; N]`, read in two's complement if `$signed`, as [`limits_agree!`] checks the integers:
/// through its traits, `$sign` giving the methods of its sign, at every pair of the
/// [`pattern_ends`] that are not zero, the limits among them, with every amount of
/// [`row_amounts`]. The function is `#[inline(never)]`, so that its frame holds the values of this
/// storage alone.
macro_rules! nonzero_limits_agree {
  ($name:ident, $nonzero:ident, $integer:ident, $signed:literal, $sign:ident) => {
    #[inline(never)]
    fn $name<B, L, const N: usize>()
    where
      B: FitsBits<[L; N]> + NonZero,
      L: ArrayLimb + bigint::Limb + Debug,
      [L; N]: ArrayRepr
        + NonZeroUintRepr<Integer = [L; N], Signed = [L; N]>
        + NonZeroIntRepr<Integer = [L; N], Unsigned = [L; N]>,
    {
      let bits = B::U32;
      let values: Vec<[L; N]> = pattern_ends(bits)
        .iter()
        .map(|pattern| bigint::limbs_of_pattern(pattern, $signed))
        .collect();
      let twin = |array| <$integer<B, [L; N]> as ArrayInteger>::checked_from_limbs(array);
      let boxed = |array| <$integer<B, Box<[L; N]>> as ArrayInteger>::checked_from_limbs(array);
      let nonzero = |array| twin(array).and_then(<$nonzero<B, [L; N]> as FixedNonZero>::new);
      let boxed_nonzero =
        |array| boxed(array).and_then(<$nonzero<B, Box<[L; N]>> as FixedNonZero>::new);
      for &x in &values {
        for &y in &values {
          let (Some(a), Some(b)) = (nonzero(x), nonzero(y)) else {
            continue;
          };
          let (c, d) = (
            boxed_nonzero(x).expect("not zero"),
            boxed_nonzero(y).expect("not zero"),
          );
          for n in row_amounts(bits) {
            let mut results = nonzero_methods(&c, &d, n);
            results.extend($sign!(&c, &d));
            let mut twin_results = nonzero_methods(&a, &b, n);
            twin_results.extend($sign!(&a, &b));
            assert_eq!(
              results, twin_results,
              "{x:?} and {y:?} at {bits} bits with {n}"
            );
          }
        }
      }
    }
  };
}

nonzero_limits_agree!(
  nonzero_uint_limits_agree,
  NonZeroUint,
  Uint,
  false,
  unsigned_nonzero
);
nonzero_limits_agree!(
  nonzero_int_limits_agree,
  NonZeroInt,
  Int,
  true,
  signed_nonzero
);

/// At the limits, the non-zero integers over a box give what their twins over the array give, as
/// [`boxed_integers_agree_at_the_limits`] checks the integers: at every pair of one, the top bit
/// alone, every bit but the top one and every bit, where they are not zero, with every amount of
/// [`row_amounts`], through the traits of the non-zero integers. There the checked product, power
/// and sum return `None` and the saturating ones saturate, and at `MIN` the negation and the
/// absolute value of `NonZeroInt` return `None`, saturate and wrap; at one bit, where `NonZeroInt`
/// holds only -1, and at partial and full widths of the limbs. Under Miri, one bit.
#[test]
fn boxed_nonzero_integers_agree_at_the_limits() {
  nonzero_uint_limits_agree::<U1, u64, 1>();
  nonzero_int_limits_agree::<U1, u64, 1>();
  if tier().is_miri() {
    return;
  }
  nonzero_uint_limits_agree::<U65, u64, 2>();
  nonzero_int_limits_agree::<U65, u64, 2>();
  nonzero_uint_limits_agree::<U20, u8, 3>();
  nonzero_int_limits_agree::<U20, u8, 3>();
  nonzero_uint_limits_agree::<U256, u64, 4>();
  nonzero_int_limits_agree::<U256, u64, 4>();
}

/// What the parsers of the non-zero integer `T` give for `text`, `FromStr` and `from_str_radix` in
/// hexadecimal, as the limbs of the integers.
fn nonzero_reads<T>(text: &str) -> String
where
  T: FixedNonZero<Integer: ArrayInteger<Array: Debug>>,
{
  let value = |x: T| limbs(x.get());
  format!(
    "{:?}",
    (
      text.parse::<T>().map(value),
      T::from_str_radix(text, 16).map(value),
    )
  )
}

/// Checks that the parsers of the non-zero integer `T`, over a box, give what those of `U`, its
/// twin over the array, give, at the spellings of zero, at texts that are no number, and at
/// numbers inside and outside the width.
#[inline(never)]
fn nonzero_reads_agree<T, U>()
where
  T: FixedNonZero<Integer: ArrayInteger<Array: Debug>>,
  U: FixedNonZero<Integer: ArrayInteger<Array: Debug>>,
{
  for text in [
    "0", "+0", "00", "-0", "", "-", "x", "1", "-1", "7f", "-80", "10000", "-fffff",
  ] {
    assert_eq!(nonzero_reads::<T>(text), nonzero_reads::<U>(text), "{text}");
  }
}

/// The constructors of a boxed non-zero integer that take a value which may be zero reject zero
/// as those of its inline twin do, and build every other value of the width: the parsers, with
/// the error of the kind `Zero` after every error of the integer; the checked cast, which returns
/// `None`, and `try_cast`, with the error of the kind `Zero`, from native integers, from an
/// inline integer and from a borrowed boxed one; and the strict cast, which panics in every build
/// and reports the caller. In both signs, at one bit, and at zero bits, which hold no value that
/// is not zero.
#[test]
fn boxed_nonzero_constructors_reject_zero() {
  nonzero_reads_agree::<NonZeroUint<U65, Box<[u64; 2]>>, NonZeroUint<U65, [u64; 2]>>();
  nonzero_reads_agree::<NonZeroInt<U65, Box<[u64; 2]>>, NonZeroInt<U65, [u64; 2]>>();
  nonzero_reads_agree::<NonZeroUint<U20, Box<[u8; 3]>>, NonZeroUint<U20, [u8; 3]>>();
  nonzero_reads_agree::<NonZeroInt<U20, Box<[u8; 3]>>, NonZeroInt<U20, [u8; 3]>>();
  nonzero_reads_agree::<NonZeroUint<U1, Box<[u64; 1]>>, NonZeroUint<U1, [u64; 1]>>();
  nonzero_reads_agree::<NonZeroInt<U1, Box<[u64; 1]>>, NonZeroInt<U1, [u64; 1]>>();
  nonzero_reads_agree::<NonZeroUint<U0, Box<[u64; 0]>>, NonZeroUint<U0, [u64; 0]>>();
  nonzero_reads_agree::<NonZeroInt<U0, Box<[u64; 0]>>, NonZeroInt<U0, [u64; 0]>>();
  type Nz = NonZeroUint<U65, Box<[u64; 2]>>;
  let read = |text: &str| text.parse::<Nz>().map(|x| limbs(x.get()));
  assert_eq!(read("0"), Err(parse_errors::ZERO));
  assert_eq!(read("36893488147419103231"), Ok([u64::MAX, 1]));

  // The checked cast and `try_cast` of each of `$sources` into `$nonzero` of `$width` in a box of
  // `$array` and into its twin in the array, as the limbs of the integers.
  macro_rules! casts_agree {
    ($nonzero:ident<$width:ty, $array:ty>: $($source:expr),+ $(,)?) => {{
      type Boxed = $nonzero<$width, Box<$array>>;
      type Twin = $nonzero<$width, $array>;
      $(
        assert_eq!(
          Boxed::checked_cast_from($source).map(|x| limbs(x.get())),
          Twin::checked_cast_from($source).map(|x| limbs(x.get())),
          "{} into {}",
          stringify!($source),
          stringify!($nonzero<$width, $array>)
        );
        assert_eq!(
          try_cast::<_, Boxed>($source).map(|x| limbs(x.get())),
          try_cast::<_, Twin>($source).map(|x| limbs(x.get())),
          "{} into {}",
          stringify!($source),
          stringify!($nonzero<$width, $array>)
        );
      )+
    }};
  }
  let (inline, boxed) = (u256::ZERO, Uint::<U256, Box<[u64; 4]>>::zero());
  let negative = Int::<U200, Box<[u64; 4]>>::from(-7i64);
  casts_agree!(NonZeroUint<U256, [u64; 4]>: 0u8, 7u64, u128::MAX, -1i8, inline, &boxed, &negative);
  casts_agree!(NonZeroInt<U200, [u64; 4]>: 0i64, -7i64, i128::MIN, u128::MAX, &boxed, &negative);
  casts_agree!(NonZeroUint<U65, [u64; 2]>: 0u64, u64::MAX, u128::MAX, i128::MIN, &boxed, &negative);
  casts_agree!(NonZeroInt<U20, [u8; 3]>: 0u8, -1i8, 1u32 << 19, -(1i32 << 19), inline, &negative);
  casts_agree!(NonZeroInt<U1, [u64; 1]>: 0i8, -1i8, 1i8, -2i8, &boxed);
  casts_agree!(NonZeroUint<U0, [u64; 0]>: 0u8, 1u8, &boxed);
  type Wide = NonZeroUint<U256, Box<[u64; 4]>>;
  type Signed = NonZeroInt<U200, Box<[u64; 4]>>;
  assert_eq!(Wide::checked_cast_from(0u8).map(|x| limbs(x.get())), None);
  assert_eq!(
    Wide::checked_cast_from(7u8).map(|x| limbs(x.get())),
    Some([7, 0, 0, 0])
  );
  assert_eq!(
    Signed::checked_cast_from(&negative).map(|x| limbs(x.get())),
    Some(negative.to_limbs())
  );
  let zero = "number would be zero for non-zero type".to_string();
  panics_here!(Wide::strict_cast_from(0u64), zero);
  panics_here!(Wide::strict_cast_from(&boxed), zero);
  panics_here!(Signed::strict_cast_from(0i8), zero);
  assert_eq!(limbs(Wide::strict_cast_from(7u8).get()), [7, 0, 0, 0]);
}

/// The sum of `values`, in code generic over the storage trait alone, through `Sum` of references.
fn total<B: FitsBits<R>, R: UintRepr>(values: &[Uint<B, R>]) -> Uint<B, R> {
  values.iter().sum()
}

/// The difference of `a` and `b`, in code generic over the storage trait alone, through `-` on
/// references.
fn difference<B: FitsBits<R>, R: IntRepr>(a: &Int<B, R>, b: &Int<B, R>) -> Int<B, R> {
  a - b
}

/// `value` plus `other` and shifted left by one place, in place, in code generic over the storage
/// trait of the storages that hold the value in place.
fn shifted_sum<B: FitsBits<R>, R: UintReprMut>(value: &mut Uint<B, R>, other: &Uint<B, R>) {
  *value += other;
  *value <<= 1u32;
}

/// The square of `value`, assigned in place, through `IntegerAssign`.
fn squared<T: IntegerAssign>(mut value: T) -> T {
  value *= value.clone();
  value
}

/// The low limb of `value`, through `ArrayInteger` with 64-bit limbs.
fn low_limb<T: ArrayInteger<Array = [u64; N]>, const N: usize>(value: &T) -> u64 {
  value.as_limbs()[0]
}

/// Code generic over `R: UintRepr`, `R: IntRepr`, `R: UintReprMut`, `T: IntegerAssign` and
/// `T: ArrayInteger<Array = [u64; N]>` takes an integer in an array and one in a box alike, and
/// gives the same value for each.
#[test]
fn generic_code_takes_arrays_and_boxes() {
  type U = Uint<U256, Box<[u64; 4]>>;
  type I = Int<U200, Box<[u64; 4]>>;
  let arrays = [u256::from(100u64), u256::from(27u64)];
  let boxes = [U::from(100u64), U::from(27u64)];
  assert_eq!(total(&boxes).into_limbs(), total(&arrays).to_limbs());
  let signed = (I::from(-5i64), I::from(12i64));
  let twins = (Int::<U200>::from(-5i64), Int::<U200>::from(12i64));
  assert_eq!(
    difference(&signed.0, &signed.1).into_limbs(),
    difference(&twins.0, &twins.1).to_limbs()
  );
  let (mut array, mut boxed) = (arrays[0], boxes[0].clone());
  shifted_sum(&mut array, &arrays[1]);
  shifted_sum(&mut boxed, &boxes[1]);
  assert_eq!(boxed.to_limbs(), array.to_limbs());
  assert_eq!(
    squared(boxes[1].clone()).into_limbs(),
    squared(arrays[1]).to_limbs()
  );
  assert_eq!(low_limb(&boxes[0]), low_limb(&arrays[0]));
  assert_eq!(low_limb(&signed.0), low_limb(&twins.0));
}

/// The limbs of a value of `N` 64-bit limbs in a box: words from `seed` in the low quarter of the
/// limbs and zeros above, so that the sum and the product of two such values stay in the width.
fn wide_limbs<const N: usize>(seed: u64) -> Box<[u64; N]> {
  let limbs: Box<[u64]> = (0..N)
    .map(|i| {
      if i < N / 4 {
        seed.rotate_left(7 * i as u32) ^ i as u64
      } else {
        0
      }
    })
    .collect();
  limbs.try_into().expect("N limbs")
}

/// A boxed integer of `B` bits in `N` 64-bit limbs and its inline twin.
type WidePair<B, const N: usize> = (Uint<B, Box<[u64; N]>>, Uint<B, [u64; N]>);

/// The [`WidePair`] of the limbs `limbs`.
fn wide_pair<B, const N: usize>(limbs: &[u64; N]) -> WidePair<B, N>
where
  B: FitsBits<[u64; N]>,
{
  let boxed = <Uint<B, Box<[u64; N]>> as ArrayInteger>::checked_from_limbs(*limbs);
  let twin = <Uint<B, [u64; N]> as ArrayInteger>::checked_from_limbs(*limbs);
  (boxed.expect("in range"), twin.expect("in range"))
}

/// `a + b` on two boxes and on two twins, each operation in a frame of its own.
#[inline(never)]
fn wide_sum<B: FitsBits<[u64; N]>, const N: usize>(a: &[u64; N], b: &[u64; N]) {
  let ((x, twin_x), (y, twin_y)) = (wide_pair::<B, N>(a), wide_pair::<B, N>(b));
  assert_eq!((x + &y).into_limbs(), (twin_x + twin_y).into_limbs());
}

/// `a * b` on two borrowed boxes and on two twins.
#[inline(never)]
fn wide_product<B: FitsBits<[u64; N]>, const N: usize>(a: &[u64; N], b: &[u64; N]) {
  let ((x, twin_x), (y, twin_y)) = (wide_pair::<B, N>(a), wide_pair::<B, N>(b));
  assert_eq!((&x * &y).into_limbs(), (twin_x * twin_y).into_limbs());
}

/// `a / b` on two boxes and on two twins.
#[inline(never)]
fn wide_quotient<B: FitsBits<[u64; N]>, const N: usize>(a: &[u64; N], b: &[u64; N]) {
  let ((x, twin_x), (y, twin_y)) = (wide_pair::<B, N>(a), wide_pair::<B, N>(b));
  assert_eq!((x / y).into_limbs(), (twin_x / twin_y).into_limbs());
}

/// `a << 7` on a box and on its twin.
#[inline(never)]
fn wide_shift<B: FitsBits<[u64; N]>, const N: usize>(a: &[u64; N]) {
  let (x, twin_x) = wide_pair::<B, N>(a);
  assert_eq!((x << 7u32).into_limbs(), (twin_x << 7u32).into_limbs());
}

/// The decimal text of a box and of its twin, and the box read back from it.
#[inline(never)]
fn wide_text<B: FitsBits<[u64; N]>, const N: usize>(a: &[u64; N]) {
  let (x, twin_x) = wide_pair::<B, N>(a);
  let text = x.to_string();
  assert_eq!(text, twin_x.to_string());
  let parsed = text.parse::<Uint<B, Box<[u64; N]>>>();
  assert_eq!(parsed.map(ArrayInteger::into_limbs), Ok(*a));
}

/// The sum of `a`, `b` and `a` again, of references to boxes and to twins.
#[inline(never)]
fn wide_total<B: FitsBits<[u64; N]>, const N: usize>(a: &[u64; N], b: &[u64; N]) {
  let ((x, twin_x), (y, twin_y)) = (wide_pair::<B, N>(a), wide_pair::<B, N>(b));
  let sum: Uint<B, Box<[u64; N]>> = [&x, &y, &x].into_iter().sum();
  let twin_sum: Uint<B, [u64; N]> = [twin_x, twin_y, twin_x].iter().sum();
  assert_eq!(sum.into_limbs(), twin_sum.into_limbs());
}

/// Checks the wide operations of `B` bits in `N` 64-bit limbs, each in a function of its own.
fn wide_operations<B: FitsBits<[u64; N]>, const N: usize>() {
  let (a, b) = (
    wide_limbs::<N>(0x9e37_79b9_7f4a_7c15),
    wide_limbs::<N>(0x5851_f42d_4c95_7f2d),
  );
  wide_sum::<B, N>(&a, &b);
  wide_product::<B, N>(&a, &b);
  wide_quotient::<B, N>(&a, &b);
  wide_shift::<B, N>(&a);
  wide_text::<B, N>(&a);
  wide_total::<B, N>(&a, &b);
}

/// At 4096 bits, and in an optimized build also at 65536, addition, multiplication, division, a
/// shift, the text and back, and `Sum` of boxed integers give what the inline twins give, each
/// operation in a frame of its own. Not under Miri, which would take minutes at 4096 bits.
#[test]
fn wide_boxed_integers() {
  if tier().is_miri() {
    return;
  }
  wide_operations::<U4096, 64>();
  if tier().is_full() {
    wide_operations::<U65536, 1024>();
  }
}

/// A boxed integer writes in JSON and in postcard what its inline twin writes, and reads back from
/// both, at 7, 64, 100, 128, 200 and 4096 bits and at zero bits, in both signs, and so do the
/// non-zero integers, which read zero as their twins do, with an error.
#[cfg(feature = "serde")]
#[test]
fn boxed_integers_serde_as_inline_integers() {
  uint_serde::<Box<[u64; 0]>, U0, u64, 0, 1>();
  int_serde::<Box<[u64; 0]>, U0, u64, 0, 1>();
  uint_serde::<Box<[u8; 1]>, U7, u8, 1, 1>();
  int_serde::<Box<[u8; 1]>, U7, u8, 1, 1>();
  uint_serde::<Box<[u64; 1]>, U64, u64, 1, 1>();
  int_serde::<Box<[u64; 1]>, U64, u64, 1, 1>();
  uint_serde::<Box<[u64; 2]>, U100, u64, 2, 2>();
  int_serde::<Box<[u64; 2]>, U100, u64, 2, 2>();
  uint_serde::<Box<[u64; 2]>, U128, u64, 2, 2>();
  int_serde::<Box<[u64; 2]>, U128, u64, 2, 2>();
  uint_serde::<Box<[u64; 4]>, U200, u64, 4, 4>();
  int_serde::<Box<[u64; 4]>, U200, u64, 4, 4>();
  uint_serde::<Box<[u64; 64]>, U4096, u64, 64, 64>();
  int_serde::<Box<[u64; 64]>, U4096, u64, 64, 64>();

  // Zero is no value of a non-zero integer, in a box as in the array.
  type Nz = NonZeroInt<U200, Box<[u64; 4]>>;
  type Twin = NonZeroInt<U200, [u64; 4]>;
  for json in ["0", "\"0\"", "\"-0\""] {
    let boxed = serde_json::from_str::<Nz>(json).map(|x| limbs(x.get()));
    let twin = serde_json::from_str::<Twin>(json).map(|x| limbs(x.get()));
    assert!(boxed.is_err(), "{json}");
    assert_eq!(
      boxed.map_err(|error| error.to_string()),
      twin.map_err(|error| error.to_string()),
      "{json}"
    );
  }
}
