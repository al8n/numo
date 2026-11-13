//! The allocations and the frees of the operations of the integers in a heap storage, counted by a
//! global allocator that this test binary alone has.
//!
//! The address of the limbs shows where a result lives, and not an allocation that came and went:
//! an operation may allocate, free and end at the address it started from. So each check counts the
//! calls of the allocator around one operation, on the thread of its test, and compares them with
//! what the heap storages promise: an operation builds only the integers that it returns, none
//! when it returns `None` or an error, and one that takes its integer by value stores its result
//! in that allocation, where the storage holds its value in place, as a `Box` does. The casts of
//! the cast traits, such as `checked_cast` and `widen`, are the exception there: they build their
//! target from the words of their source. A shared storage never writes a value that it holds: a
//! result is a new allocation, unless it is the allocation of an operand, which it then shares or
//! takes over, as the absolute value of a value that is not negative and the conversions into a
//! non-zero integer do, and an integer that an operation takes gives up its share unless the
//! result is its allocation, which frees the allocation only where no other owner holds it.
//!
//! The digits and the forms that return a `Vec` or a `String` are counted too, in a heap storage
//! and in the inline twin: a constructor from digits builds the integer that it returns and
//! nothing for `None`, the iterator of the digits, least significant first, allocates nothing, and
//! a form that returns a `Vec` or a `String` allocates it once, with a capacity that it does not
//! outgrow, and not at all where it is empty.
//!
//! The decimals over a heap storage are counted last, by the same rules but one, which their docs
//! give: a decimal stores the value that it computed, so in a shared storage a result that equals
//! an operand is a new allocation, and not a share of the operand.

#![cfg(any(
  feature = "std",
  feature = "alloc",
  all(feature = "triomphe", target_has_atomic = "ptr")
))]

#[cfg(any(feature = "std", feature = "alloc"))]
use std::rc::Rc;
#[cfg(all(any(feature = "std", feature = "alloc"), target_has_atomic = "ptr"))]
use std::sync::Arc;
use std::{
  alloc::{GlobalAlloc, Layout, System},
  cell::Cell,
  hint::black_box,
  thread::LocalKey,
};

use numo::{
  ArrayInteger, CheckedCastFrom, CheckedOps, FixedInteger, FixedNonZero, FromLimbs, Int,
  IntegerBits, IntegerByteOrder, IntegerFloat, IntegerMath, IntegerOne, NonZeroInt, NonZeroLimits,
  NonZeroUint, SaturatingOps, SignedBits, SignedFixedInteger, SignedNonZero, Uint, UnsignedBits,
  UnsignedDigits, UnsignedFixedInteger, UnsignedNonZero, WidenFrom, WrappingOps, typenum::U256,
};
#[cfg(all(feature = "triomphe", target_has_atomic = "ptr"))]
use triomphe::Arc as TriompheArc;

/// The system allocator, with the calls of each thread counted.
struct Counting;

thread_local! {
  /// The calls of `alloc`, `alloc_zeroed` and `realloc` on this thread.
  static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };

  /// The calls of `dealloc` on this thread.
  static FREES: Cell<usize> = const { Cell::new(0) };

  /// The calls of `alloc_zeroed` on this thread, which `ALLOCATIONS` counts too.
  static ZEROED: Cell<usize> = const { Cell::new(0) };
}

/// Adds one to `counter` on this thread.
///
/// The standard library guarantees that its thread-locals never allocate with the global
/// allocator, also on a target that allocates their storage on first use, and these cells have a
/// constant initializer and no destructor, so an access registers nothing: the allocator is not
/// entered again from here. `try_with` leaves a call uncounted that arrives while the locals of its
/// thread are being destroyed, and the addition wraps, so that nothing here can panic, which an
/// allocator must not do.
fn count(counter: &'static LocalKey<Cell<usize>>) {
  let _ = counter.try_with(|calls| calls.set(calls.get().wrapping_add(1)));
}

// SAFETY: each method gives its arguments unchanged to the method of the same name of `System`
// and returns what that returns. So what a caller must guarantee to this allocator is what
// `System` requires, the blocks of this allocator are those of `System`, and `System` keeps the
// contract of `GlobalAlloc` towards the caller. The counting touches no block and cannot unwind.
unsafe impl GlobalAlloc for Counting {
  unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
    count(&ALLOCATIONS);
    // SAFETY: the caller guarantees that `layout` has a size that is not zero.
    unsafe { System.alloc(layout) }
  }

  unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
    count(&ALLOCATIONS);
    count(&ZEROED);
    // SAFETY: the caller guarantees that `layout` has a size that is not zero.
    unsafe { System.alloc_zeroed(layout) }
  }

  unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
    count(&ALLOCATIONS);
    // SAFETY: the caller guarantees that `ptr` is a block that this allocator, and so `System`,
    // has allocated and not freed, that `layout` is the layout it was allocated with, and that
    // `new_size` is not zero and does not overflow `isize` when it is rounded up to the alignment
    // of `layout`.
    unsafe { System.realloc(ptr, layout, new_size) }
  }

  unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
    count(&FREES);
    // SAFETY: the caller guarantees that `ptr` is a block that this allocator, and so `System`,
    // has allocated and not freed, and that `layout` is the layout it was allocated with.
    unsafe { System.dealloc(ptr, layout) }
  }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// What `operation` returns, with the allocations and the frees of this thread while it ran. The
/// result is still alive when the counts are read, so its own allocation is not freed in them.
///
/// The result goes through `black_box`, and so do the operands that the tests build: an optimizing
/// compiler may remove an allocation that nothing observes, with its free, and the counts are
/// those of the operations as written only while every value in them is observed.
fn counted<T>(operation: impl FnOnce() -> T) -> (T, [usize; 2]) {
  let before = [ALLOCATIONS.get(), FREES.get()];
  let result = black_box(operation());
  let after = [ALLOCATIONS.get(), FREES.get()];
  (
    result,
    [
      after[0].wrapping_sub(before[0]),
      after[1].wrapping_sub(before[1]),
    ],
  )
}

/// Asserts that `$operation` allocates `$allocations` times and frees `$frees` times, and gives its
/// result. The operation borrows what it only reads and takes what it consumes, so a consumed
/// operand that the operation does not return is freed inside the count, and a borrowed one is
/// not.
macro_rules! allocates {
  ($allocations:literal, $frees:literal; $operation:expr) => {{
    let (result, counts) = counted(|| $operation);
    assert_eq!(counts, [$allocations, $frees], "{}", stringify!($operation));
    result
  }};
}

/// The inline twin of `Boxed` and of each `Shared`.
type Inline = Uint<U256, [u64; 4]>;

/// The address of the limbs of `value`: that of its allocation.
fn address<T: ArrayInteger>(value: &T) -> *const T::Array {
  std::ptr::from_ref(value.as_limbs())
}

/// The digits of `2^256` in the radix 256, most significant first: one more than the largest value
/// of 256 bits.
const OVER_BE: [u8; 33] = {
  let mut digits = [0; 33];
  digits[0] = 1;
  digits
};

/// The same digits, least significant first.
const OVER_LE: [u8; 33] = {
  let mut digits = [0; 33];
  digits[32] = 1;
  digits
};

/// The digits of `2^256` in the base `2^32`, most significant first.
const OVER_WORDS_BE: [u64; 9] = [1, 0, 0, 0, 0, 0, 0, 0, 0];

/// The same digits, least significant first.
const OVER_WORDS_LE: [u64; 9] = [0, 0, 0, 0, 0, 0, 0, 0, 1];

/// Asserts what the unsigned integer `$uint`, of 256 bits, allocates when it reads digits and when
/// it writes them through its iterator. A constructor allocates `$built` times for the integer
/// that it returns, once in a heap storage and never inline, and not at all for `None`, whether a
/// digit is not below the base or the value is `2^256`: a reader that built a value and dropped it
/// would return the same `None`, and only the count tells the two apart. The iterator of the
/// digits allocates nothing.
macro_rules! reads_digits {
  ($uint:ty, $built:literal) => {{
    let radix_be = <$uint as UnsignedDigits>::checked_from_radix_be;
    let radix_le = <$uint as UnsignedDigits>::checked_from_radix_le;

    assert!(allocates!(0, 0; radix_be(&[1, 10], 10)).is_none());
    assert!(allocates!(0, 0; radix_le(&[10, 1], 10)).is_none());
    assert!(allocates!(0, 0; <$uint>::checked_from_base_be([1, 10], 10)).is_none());
    assert!(allocates!(0, 0; <$uint>::checked_from_base_le([10, 1], 10)).is_none());

    assert!(allocates!(0, 0; radix_be(&OVER_BE, 256)).is_none());
    assert!(allocates!(0, 0; radix_le(&OVER_LE, 256)).is_none());
    assert!(allocates!(0, 0; <$uint>::checked_from_base_be(OVER_WORDS_BE, 1 << 32)).is_none());
    assert!(allocates!(0, 0; <$uint>::checked_from_base_le(OVER_WORDS_LE, 1 << 32)).is_none());

    // In the base 256 a reader adds the last digits after its loop, and builds nothing there for
    // `None` either.
    let (over_be, over_le) = (OVER_BE.map(u64::from), OVER_LE.map(u64::from));
    assert!(allocates!(0, 0; <$uint>::checked_from_base_be(over_be, 256)).is_none());
    assert!(allocates!(0, 0; <$uint>::checked_from_base_le(over_le, 256)).is_none());

    assert!(allocates!($built, 0; radix_be(&[1, 2, 7], 10)).is_some());
    assert!(allocates!($built, 0; radix_le(&[7, 2, 1], 10)).is_some());
    assert!(allocates!($built, 0; <$uint>::checked_from_base_be([1, 2, 7], 10)).is_some());
    assert!(allocates!($built, 0; <$uint>::checked_from_base_le([7, 2, 1], 10)).is_some());

    let largest = black_box(<$uint as FixedInteger>::max_value());
    assert_eq!(allocates!(0, 0; largest.to_base_le(10).count()), 78);
  }};
}

/// Asserts that each form of the integers `$uint` and `$int`, of 256 bits, and of their non-zero
/// integers `$nonzero` and `$nonzero_int`, that returns a `Vec` or a `String` allocates what it
/// returns, once, and nothing else, in a heap storage as in an inline one, and that the digits of a
/// base, most significant first, are collected in one `Vec`, which their iterator frees. Each
/// capacity is a bound of what is written, which the largest value meets exactly in the bases and
/// the radixes 2 and 256, and the smallest signed one in the radix 2 with its sign: a writer that
/// reserved one less would reallocate, which the allocator counts. What is empty is not allocated:
/// the digits of zero in a base, and its trimmed bytes. The text of a signed integer builds no
/// magnitude in the heap, which would count a second allocation and a free.
#[cfg(any(feature = "std", feature = "alloc"))]
macro_rules! writes_vectors_and_text {
  ($uint:ty, $int:ty, $nonzero:ty, $nonzero_int:ty) => {{
    let most = black_box(<$uint as FixedInteger>::max_value());
    let zero = black_box(<$uint as FixedInteger>::min_value());
    let least = black_box(<$int as FixedInteger>::min_value());
    let greatest = black_box(<$int as FixedInteger>::max_value());
    let minus_one = black_box(<$int>::from(-1i64));
    let nothing = black_box(<$int>::from(0i64));
    let nonzero = black_box(<$nonzero as NonZeroLimits>::max_value());
    let negative = black_box(<$nonzero_int as NonZeroLimits>::min_value());

    // The digits of a base, most significant first, which fill the capacity in the bases 2 and
    // 256. The iterator is observed before it is counted, so that the `Vec` that it holds is one
    // that the harness sees, and counting the digits frees it.
    let digits = allocates!(1, 0; most.to_base_be(2));
    assert_eq!(allocates!(0, 1; digits.count()), 256);
    let digits = allocates!(1, 0; most.to_base_be(10));
    assert_eq!(allocates!(0, 1; digits.count()), 78);
    let digits = allocates!(1, 0; most.to_base_be(256));
    assert_eq!(allocates!(0, 1; digits.count()), 32);
    let digits = allocates!(0, 0; zero.to_base_be(10));
    assert_eq!(allocates!(0, 0; digits.count()), 0);

    // The digits of a radix, which fill the capacity in the radixes 2 and 256.
    assert_eq!(allocates!(1, 0; most.to_radix_le(2)).len(), 256);
    assert_eq!(allocates!(1, 0; most.to_radix_be(2)).len(), 256);
    assert_eq!(allocates!(1, 0; most.to_radix_le(3)).len(), 162);
    assert_eq!(allocates!(1, 0; most.to_radix_be(3)).len(), 162);
    assert_eq!(allocates!(1, 0; most.to_radix_le(10)).len(), 78);
    assert_eq!(allocates!(1, 0; most.to_radix_be(10)).len(), 78);
    assert_eq!(allocates!(1, 0; most.to_radix_le(256)).len(), 32);
    assert_eq!(allocates!(1, 0; most.to_radix_be(256)).len(), 32);
    assert_eq!(allocates!(1, 0; zero.to_radix_le(2)), [0]);
    assert_eq!(allocates!(1, 0; zero.to_radix_be(2)), [0]);
    assert_eq!(allocates!(1, 0; zero.to_radix_le(3)), [0]);
    assert_eq!(allocates!(1, 0; zero.to_radix_be(3)), [0]);
    assert_eq!(allocates!(1, 0; zero.to_radix_le(10)), [0]);
    assert_eq!(allocates!(1, 0; zero.to_radix_be(10)), [0]);
    assert_eq!(allocates!(1, 0; zero.to_radix_le(256)), [0]);
    assert_eq!(allocates!(1, 0; zero.to_radix_be(256)), [0]);

    // The text, which fills the capacity in the radix 2, with the sign of a negative value.
    assert_eq!(allocates!(1, 0; most.to_str_radix(2)).len(), 256);
    assert_eq!(allocates!(1, 0; most.to_str_radix(36)).len(), 50);
    assert_eq!(allocates!(1, 0; zero.to_str_radix(2)), "0");
    assert_eq!(allocates!(1, 0; zero.to_str_radix(36)), "0");
    assert_eq!(allocates!(1, 0; least.to_str_radix(2)).len(), 257);
    assert_eq!(allocates!(1, 0; least.to_str_radix(36)).len(), 51);
    assert_eq!(allocates!(1, 0; minus_one.to_str_radix(2)), "-1");
    assert_eq!(allocates!(1, 0; minus_one.to_str_radix(36)), "-1");
    assert_eq!(allocates!(1, 0; greatest.to_str_radix(2)).len(), 255);
    assert_eq!(allocates!(1, 0; nothing.to_str_radix(2)), "0");
    assert_eq!(allocates!(1, 0; nonzero.to_str_radix(2)).len(), 256);
    assert_eq!(allocates!(1, 0; nonzero.to_str_radix(36)).len(), 50);
    assert_eq!(allocates!(1, 0; negative.to_str_radix(2)).len(), 257);
    assert_eq!(allocates!(1, 0; negative.to_str_radix(36)).len(), 51);

    // The bytes: the 32 of the width, and the shortest ones, of which zero has none.
    assert_eq!(allocates!(1, 0; most.to_le_bytes_vec()).len(), 32);
    assert_eq!(allocates!(1, 0; most.to_be_bytes_vec()).len(), 32);
    assert_eq!(allocates!(1, 0; most.to_le_bytes_trimmed_vec()).len(), 32);
    assert_eq!(allocates!(1, 0; most.to_be_bytes_trimmed_vec()).len(), 32);
    assert_eq!(allocates!(1, 0; zero.to_le_bytes_vec()).len(), 32);
    assert_eq!(allocates!(1, 0; zero.to_be_bytes_vec()).len(), 32);
    assert!(allocates!(0, 0; zero.to_le_bytes_trimmed_vec()).is_empty());
    assert!(allocates!(0, 0; zero.to_be_bytes_trimmed_vec()).is_empty());
    assert_eq!(allocates!(1, 0; minus_one.to_le_bytes_vec()).len(), 32);
    assert_eq!(allocates!(1, 0; minus_one.to_be_bytes_vec()).len(), 32);
    assert_eq!(allocates!(1, 0; minus_one.to_le_bytes_trimmed_vec()), [0xff]);
    assert_eq!(allocates!(1, 0; minus_one.to_be_bytes_trimmed_vec()), [0xff]);
    assert_eq!(allocates!(1, 0; least.to_le_bytes_vec()).len(), 32);
    assert_eq!(allocates!(1, 0; least.to_be_bytes_vec()).len(), 32);
    assert_eq!(allocates!(1, 0; least.to_le_bytes_trimmed_vec()).len(), 32);
    assert_eq!(allocates!(1, 0; least.to_be_bytes_trimmed_vec()).len(), 32);
    assert_eq!(allocates!(1, 0; greatest.to_le_bytes_vec()).len(), 32);
    assert_eq!(allocates!(1, 0; greatest.to_be_bytes_vec()).len(), 32);
    assert_eq!(allocates!(1, 0; greatest.to_le_bytes_trimmed_vec()).len(), 32);
    assert_eq!(allocates!(1, 0; greatest.to_be_bytes_trimmed_vec()).len(), 32);
    assert_eq!(allocates!(1, 0; nothing.to_le_bytes_vec()).len(), 32);
    assert_eq!(allocates!(1, 0; nothing.to_be_bytes_vec()).len(), 32);
    assert!(allocates!(0, 0; nothing.to_le_bytes_trimmed_vec()).is_empty());
    assert!(allocates!(0, 0; nothing.to_be_bytes_trimmed_vec()).is_empty());
  }};
}

/// The inline twin reads digits, and writes them through its iterator, without allocating.
#[test]
fn the_inline_twin_reads_digits_without_allocating() {
  reads_digits!(Inline, 0);
}

/// On the inline twin, each form that returns a `Vec` or a `String` allocates what it returns and
/// nothing else, as on a heap storage.
#[cfg(any(feature = "std", feature = "alloc"))]
#[test]
fn the_inline_twin_allocates_what_a_writer_returns() {
  writes_vectors_and_text!(
    Inline,
    Int<U256, [u64; 4]>,
    NonZeroUint<U256, [u64; 4]>,
    NonZeroInt<U256, [u64; 4]>
  );
}

/// The checks of the storage that holds its value in place, `Box<[u64; 4]>` at 256 bits.
#[cfg(any(feature = "std", feature = "alloc"))]
mod boxed {
  use super::*;

  /// An unsigned integer of 256 bits in a box of four limbs.
  type Boxed = Uint<U256, Box<[u64; 4]>>;

  /// The signed integer of the same width and storage.
  type BoxedInt = Int<U256, Box<[u64; 4]>>;

  /// The non-zero integer over `Boxed`.
  type BoxedNonZero = NonZeroUint<U256, Box<[u64; 4]>>;

  /// The non-zero integer over `BoxedInt`.
  type BoxedNonZeroInt = NonZeroInt<U256, Box<[u64; 4]>>;

  /// The boxed unsigned integer of `value`, observed.
  fn boxed(value: u64) -> Boxed {
    black_box(Boxed::from(value))
  }

  /// The boxed signed integer of `value`, observed.
  fn boxed_int(value: i64) -> BoxedInt {
    black_box(BoxedInt::from(value))
  }

  /// Two boxed integers, a larger and a smaller one.
  fn pair() -> (Boxed, Boxed) {
    (boxed(1_000_000), boxed(7))
  }

  /// A boxed non-zero integer.
  fn divisor() -> BoxedNonZero {
    black_box(BoxedNonZero::new(boxed(3)).expect("not zero"))
  }

  /// The largest boxed unsigned integer, observed.
  fn largest() -> Boxed {
    black_box(<Boxed as FixedInteger>::max_value())
  }

  /// The smallest boxed signed integer, observed.
  fn smallest() -> BoxedInt {
    black_box(<BoxedInt as FixedInteger>::min_value())
  }

  /// An operation on references that returns an integer allocates that integer, and nothing else.
  #[test]
  fn operations_on_references_allocate_their_result() {
    let (a, b) = pair();
    allocates!(1, 0; &a + &b);
    allocates!(1, 0; &a - &b);
    allocates!(1, 0; &a * &b);
    allocates!(1, 0; &a / &b);
    allocates!(1, 0; &a % &b);
    allocates!(1, 0; &a & &b);
    allocates!(1, 0; &a | &b);
    allocates!(1, 0; &a ^ &b);
    allocates!(1, 0; !&a);
    allocates!(1, 0; &a << 7u32);
    allocates!(1, 0; &a >> 7u32);
    allocates!(1, 0; a.isqrt());
    allocates!(1, 0; a.wrapping_pow(3));
    allocates!(1, 0; a.checked_add(&b));
    allocates!(1, 0; a.wrapping_mul(&b));
    let divisor = divisor();
    allocates!(1, 0; &a / &divisor);
    allocates!(1, 0; &a % &divisor);
    let negative = boxed_int(-1_000_000);
    allocates!(1, 0; -&negative);
  }

  /// An operator with an owned operand stores its result in the allocation of that operand, the
  /// left one of two, and frees the other operand that it owns.
  #[test]
  fn operators_with_an_owned_operand_reuse_its_allocation() {
    let (a, b) = pair();
    allocates!(0, 1; a + b);
    let (a, b) = pair();
    allocates!(0, 0; a + &b);
    let (a, b) = pair();
    allocates!(0, 0; &a - b);
    let (a, b) = pair();
    allocates!(0, 1; a * b);
    let (a, b) = pair();
    allocates!(0, 1; a / b);
    let (a, b) = pair();
    allocates!(0, 0; &a % b);
    let (a, b) = pair();
    allocates!(0, 0; a ^ &b);
    let (a, _) = pair();
    allocates!(0, 0; !a);
    let (a, _) = pair();
    allocates!(0, 0; a << 3u32);
    let (a, _) = pair();
    allocates!(0, 0; a >> 3u32);
    let negative = boxed_int(-1_000_000);
    allocates!(0, 0; -negative);

    // By a non-zero divisor: the dividend that the operator owns holds the result, or else the
    // divisor that it owns.
    let (a, _) = pair();
    let by = divisor();
    allocates!(0, 1; a / by);
    let (a, _) = pair();
    let by = divisor();
    allocates!(0, 0; a % &by);
    let (a, _) = pair();
    let by = divisor();
    allocates!(0, 0; &a / by);
  }

  /// A compound assignment, `set_bit` and `clone_from` store in the allocation of their target. An
  /// operand that the assignment owns is freed.
  #[test]
  fn assignments_store_in_their_target() {
    let (mut a, b) = pair();
    allocates!(0, 0; a += &b);
    allocates!(0, 0; a -= &b);
    allocates!(0, 0; a *= &b);
    allocates!(0, 0; a /= &b);
    allocates!(0, 0; a %= &b);
    allocates!(0, 0; a &= &b);
    allocates!(0, 0; a |= &b);
    allocates!(0, 0; a ^= &b);
    allocates!(0, 0; a <<= 9u32);
    allocates!(0, 0; a >>= 2u32);
    allocates!(0, 0; a.set_bit(100, true));
    allocates!(0, 0; a.clone_from(&b));
    let by = divisor();
    allocates!(0, 0; a /= &by);
    allocates!(0, 0; a %= &by);
    let (_, owned) = pair();
    allocates!(0, 1; a += owned);
    let owned = divisor();
    allocates!(0, 1; a /= owned);
  }

  /// `Sum` and `Product` compute on the stack and store once: of values in the allocation of the
  /// first item, with the other items freed, and of references, or of no item, in a new allocation.
  #[test]
  fn folds_store_once() {
    let values = [boxed(3), boxed(5), boxed(7)];
    allocates!(1, 0; values.iter().sum::<Boxed>());
    allocates!(1, 0; values.iter().product::<Boxed>());
    let first = address(&values[0]);
    let sum = allocates!(0, 2; values.into_iter().sum::<Boxed>());
    assert_eq!(address(&sum), first);

    let values = [boxed(3), boxed(5), boxed(7)];
    let first = address(&values[0]);
    let product = allocates!(0, 2; values.into_iter().product::<Boxed>());
    assert_eq!(address(&product), first);

    allocates!(1, 0; std::iter::empty::<Boxed>().sum::<Boxed>());
    allocates!(1, 0; std::iter::empty::<Boxed>().product::<Boxed>());
    allocates!(1, 0; std::iter::empty::<&Boxed>().sum::<Boxed>());
    allocates!(1, 0; std::iter::empty::<&Boxed>().product::<Boxed>());
  }

  /// Each way of building an integer allocates once: zero in zeroed memory, and every other value
  /// from its array. `clone` allocates the copy.
  #[test]
  fn construction_allocates_once() {
    let zeroed = ZEROED.get();
    allocates!(1, 0; <Boxed as FixedInteger>::zero());
    allocates!(1, 0; Boxed::default());
    assert_eq!(
      ZEROED.get().wrapping_sub(zeroed),
      2,
      "zero takes zeroed memory"
    );
    allocates!(1, 0; <Boxed as IntegerOne>::one());
    allocates!(1, 0; <Boxed as FixedInteger>::min_value());
    allocates!(1, 0; <Boxed as FixedInteger>::max_value());
    allocates!(1, 0; <BoxedInt as FixedInteger>::min_value());
    allocates!(1, 0; Boxed::from(5u64));
    allocates!(1, 0; Boxed::checked_cast_from(5u64));
    let parsed = allocates!(1, 0; "340282366920938463463374607431768211456".parse::<Boxed>());
    assert!(parsed.is_ok());
    let inline = black_box(Inline::from(5u64));
    allocates!(1, 0; Boxed::widen_from(&inline));
    let (a, _) = pair();
    allocates!(1, 0; a.clone());
  }

  /// The pointer moves into an integer and out of it without a copy, an array of limbs is put in
  /// one allocation, and the limbs of an integer that is taken leave its allocation, which is
  /// freed.
  #[test]
  fn pointers_move_and_limbs_are_stored_once() {
    let pointer = black_box(Box::new([1u64, 2, 3, 4]));
    let limbs = std::ptr::from_ref(&*pointer);
    let integer = allocates!(0, 0; Boxed::from(pointer));
    assert_eq!(address(&integer), limbs);
    let pointer = allocates!(0, 0; Box::<[u64; 4]>::from(integer));
    assert_eq!(std::ptr::from_ref(&*pointer), limbs);
    allocates!(1, 0; Boxed::from_limbs(*pointer));
    let integer = allocates!(1, 0; Boxed::checked_from_limbs(*pointer)).expect("in range");
    allocates!(0, 1; integer.into_limbs());
  }

  /// What reads a value without returning an integer in a heap storage does not allocate: the casts
  /// into an inline integer, the comparisons and the queries of the bits.
  #[test]
  fn reads_do_not_allocate() {
    let (a, b) = pair();
    allocates!(0, 0; u64::checked_cast_from(&a));
    allocates!(0, 0; Inline::widen_from(&a));
    allocates!(0, 0; a == b);
    allocates!(0, 0; a < b);
    allocates!(0, 0; a.cmp(&b));
    allocates!(0, 0; a.count_ones());
    allocates!(0, 0; a.leading_zeros());
    allocates!(0, 0; a.bit(3));
  }

  /// The text of a boxed integer allocates what the text of its inline twin does: the `String`.
  #[test]
  fn the_text_allocates_as_for_the_inline_twin() {
    let (a, _) = pair();
    let twin = black_box(Inline::from(1_000_000u64));
    let (inline, expected) = counted(|| twin.to_string());
    let (boxed, counts) = counted(|| a.to_string());
    assert_eq!((boxed, counts), (inline, expected));
    let (inline, expected) = counted(|| format!("{twin:x}"));
    let (boxed, counts) = counted(|| format!("{a:x}"));
    assert_eq!((boxed, counts), (inline, expected));
  }

  /// The pointer is its own non-zero storage: the conversions between an integer and its non-zero
  /// integer and the casts between the signs move it. A row that returns no integer lends it, and
  /// one that returns an integer allocates that integer.
  #[test]
  fn nonzero_integers_move_and_lend_their_pointer() {
    let (a, b) = pair();
    let nonzero = allocates!(0, 0; BoxedNonZero::new(a)).expect("not zero");
    let integer = allocates!(0, 0; nonzero.get());
    let nonzero = allocates!(0, 0; BoxedNonZero::try_from(integer)).expect("not zero");
    let signed = allocates!(0, 0; nonzero.cast_signed());
    let nonzero = allocates!(0, 0; signed.cast_unsigned());
    allocates!(0, 0; nonzero.leading_zeros());
    allocates!(0, 0; nonzero.count_ones());
    let other = BoxedNonZero::new(b).expect("not zero");
    allocates!(1, 0; nonzero.checked_mul(&other));
    let negative = black_box(BoxedNonZeroInt::new(boxed_int(-5)).expect("not zero"));
    allocates!(1, 0; negative.unsigned_abs());
    allocates!(1, 0; <BoxedNonZero as NonZeroLimits>::min_value());
    allocates!(1, 0; <BoxedNonZero as NonZeroLimits>::max_value());
    allocates!(1, 0; <BoxedNonZeroInt as NonZeroLimits>::min_value());
  }

  /// A checked method that returns `None` builds no integer, and one that returns `Some` builds
  /// that one.
  #[test]
  fn checked_methods_build_nothing_for_none() {
    let (a, b) = pair();
    let most = largest();
    assert!(allocates!(0, 0; most.checked_add(&b)).is_none());
    assert!(allocates!(0, 0; b.checked_sub(&a)).is_none());
    assert!(allocates!(0, 0; most.checked_mul(&a)).is_none());
    assert!(allocates!(0, 0; a.checked_neg()).is_none());
    assert!(allocates!(0, 0; most.checked_pow(2)).is_none());
    assert!(allocates!(0, 0; a.checked_shl(256)).is_none());
    assert!(allocates!(0, 0; a.checked_shr(256)).is_none());
    assert!(allocates!(1, 0; a.checked_sub(&b)).is_some());
    let least = smallest();
    assert!(allocates!(0, 0; least.checked_abs()).is_none());
    assert!(allocates!(0, 0; least.checked_neg()).is_none());
  }

  /// A saturating method that saturates builds its bound, and no wrapped result beside it.
  #[test]
  fn saturating_methods_build_their_bound_alone() {
    let (a, b) = pair();
    let most = largest();
    allocates!(1, 0; most.saturating_add(&b));
    allocates!(1, 0; b.saturating_sub(&a));
    allocates!(1, 0; most.saturating_mul(&a));
    allocates!(1, 0; most.saturating_pow(2));
    let least = smallest();
    let one = boxed_int(1);
    let minus_one = boxed_int(-1);
    allocates!(1, 0; least.saturating_add(&minus_one));
    allocates!(1, 0; least.saturating_sub(&one));
    allocates!(1, 0; least.saturating_mul(&minus_one));
    allocates!(1, 0; least.saturating_div(&minus_one));
    allocates!(1, 0; least.saturating_pow(3));
    allocates!(1, 0; least.saturating_neg());
    allocates!(1, 0; least.saturating_abs());
  }

  /// The carrying addition and the borrowing subtraction build their result alone, also with a
  /// carry or a borrow to take in, whatever flag they return.
  #[test]
  fn carries_and_borrows_build_one_integer() {
    let (a, b) = pair();
    let most = largest();
    let (_, carry) = allocates!(1, 0; a.carrying_add(&b, true));
    assert!(!carry);
    let (_, carry) = allocates!(1, 0; most.carrying_add(&b, true));
    assert!(carry);
    let (_, borrow) = allocates!(1, 0; a.borrowing_sub(&b, true));
    assert!(!borrow);
    let (_, borrow) = allocates!(1, 0; b.borrowing_sub(&a, true));
    assert!(borrow);
  }

  /// The carrying addition and the borrowing subtraction of `Int` build their result alone, with a
  /// carry or a borrow to take in or without, whether the exact value overflows the range or not.
  #[test]
  fn signed_carries_and_borrows_build_one_integer() {
    let (a, b) = (boxed_int(-1_000_000), boxed_int(7));
    let least = smallest();
    let greatest = black_box(<BoxedInt as FixedInteger>::max_value());
    let (_, overflow) = allocates!(1, 0; a.carrying_add(&b, true));
    assert!(!overflow);
    let (_, overflow) = allocates!(1, 0; greatest.carrying_add(&b, true));
    assert!(overflow);
    let (_, overflow) = allocates!(1, 0; a.carrying_add(&b, false));
    assert!(!overflow);
    let (_, overflow) = allocates!(1, 0; least.carrying_add(&a, false));
    assert!(overflow);
    let (_, overflow) = allocates!(1, 0; a.borrowing_sub(&b, true));
    assert!(!overflow);
    let (_, overflow) = allocates!(1, 0; least.borrowing_sub(&b, true));
    assert!(overflow);
    let (_, overflow) = allocates!(1, 0; a.borrowing_sub(&b, false));
    assert!(!overflow);
    let (_, overflow) = allocates!(1, 0; greatest.borrowing_sub(&a, false));
    assert!(overflow);
  }

  /// The products with a carry build the two halves that they return, of both signs: the low half
  /// of a signed product is the unsigned integer in a box.
  #[test]
  fn carrying_products_build_both_halves() {
    let (a, b) = pair();
    let halves = allocates!(2, 0; a.carrying_mul(&b, &b));
    assert_eq!(halves, (boxed(7_000_007), boxed(0)));
    let halves = allocates!(2, 0; a.carrying_mul_add(&b, &b, &a));
    assert_eq!(halves, (boxed(8_000_007), boxed(0)));
    let (c, d, e) = (boxed_int(-1_000_000), boxed_int(7), boxed_int(-3));
    let (_, high) = allocates!(2, 0; c.carrying_mul(&d, &e));
    assert_eq!(high, boxed_int(-1));
    let (_, high) = allocates!(2, 0; c.carrying_mul_add(&d, &e, &d));
    assert_eq!(high, boxed_int(-1));
  }

  /// The widening product is in the default storage of its width, which is inline, so it allocates
  /// nothing, of two boxed operands, of a boxed operand by an inline one and the reverse, of both
  /// signs.
  #[test]
  fn widening_products_allocate_nothing() {
    let (a, b) = pair();
    let twin = black_box(Inline::from(5u64));
    allocates!(0, 0; a.widening_mul(&b));
    allocates!(0, 0; a.widening_mul(&twin));
    allocates!(0, 0; twin.widening_mul(&a));
    let (c, d) = (boxed_int(-1_000_000), boxed_int(7));
    let signed_twin = black_box(Int::<U256, [u64; 4]>::from(-5i64));
    allocates!(0, 0; c.widening_mul(&d));
    allocates!(0, 0; c.widening_mul(&signed_twin));
    allocates!(0, 0; signed_twin.widening_mul(&c));
  }

  /// The exact division builds its quotient, and nothing for a division that leaves a remainder.
  #[test]
  fn exact_division_builds_only_a_quotient() {
    let (a, b) = pair();
    assert!(allocates!(0, 0; a.div_exact(&b)).is_none());
    let by = boxed(1_000);
    assert!(allocates!(1, 0; a.div_exact(&by)).is_some());
  }

  /// The casts between the signs, the saturating ones that change the value too, and the
  /// conversions from a byte order keep the allocation of the integer that they take.
  #[test]
  fn casts_and_byte_orders_keep_the_allocation() {
    let (a, b) = pair();
    let limbs = address(&a);
    let signed = allocates!(0, 0; a.cast_signed());
    assert_eq!(address(&signed), limbs);

    // The largest unsigned value is above every signed one, and saturates.
    let most = largest();
    let limbs = address(&most);
    let signed = allocates!(0, 0; most.saturating_cast_signed());
    assert_eq!(address(&signed), limbs);
    assert_eq!(signed, <BoxedInt as FixedInteger>::max_value());

    // A negative value saturates to zero.
    let negative = boxed_int(-5);
    let limbs = address(&negative);
    let unsigned = allocates!(0, 0; negative.saturating_cast_unsigned());
    assert_eq!(address(&unsigned), limbs);
    assert_eq!(unsigned, <Boxed as FixedInteger>::zero());

    // One of the two reverses the bytes, on every target.
    let limbs = address(&b);
    let b = allocates!(0, 0; <Boxed as IntegerByteOrder>::from_be(b));
    assert_eq!(address(&b), limbs);
    let b = allocates!(0, 0; <Boxed as IntegerByteOrder>::from_le(b));
    assert_eq!(address(&b), limbs);

    // The conversions to a byte order borrow their integer, and each builds one of its own.
    allocates!(2, 0; (b.to_be(), b.to_le()));
  }

  /// The rows of the non-zero integers build what they return: nothing for `None`, and the bound
  /// alone where they saturate. `|=` builds its result and frees the value that it replaces.
  #[test]
  fn nonzero_methods_build_only_what_they_return() {
    let least = black_box(<BoxedNonZeroInt as NonZeroLimits>::min_value());
    assert!(allocates!(0, 0; least.checked_neg()).is_none());
    allocates!(1, 0; least.saturating_neg());
    assert!(allocates!(0, 0; least.checked_abs()).is_none());
    allocates!(1, 0; least.saturating_abs());
    allocates!(1, 0; least.unsigned_abs());

    let most = black_box(<BoxedNonZero as NonZeroLimits>::max_value());
    let one = boxed(1);
    assert!(allocates!(0, 0; most.checked_mul(&most)).is_none());
    allocates!(1, 0; most.saturating_mul(&most));
    assert!(allocates!(0, 0; most.checked_pow(2)).is_none());
    allocates!(1, 0; most.saturating_pow(2));
    assert!(allocates!(0, 0; most.checked_add(&one)).is_none());
    allocates!(1, 0; most.saturating_add(&one));

    let mut target = divisor();
    let other = divisor();
    allocates!(1, 1; target |= &other);

    // The absolute value of a value that is not negative is a copy of it.
    let positive = black_box(BoxedNonZeroInt::new(boxed_int(5)).expect("not zero"));
    assert!(allocates!(1, 0; positive.checked_abs()).is_some());

    // An operator that takes its non-zero integer stores its result in it.
    allocates!(0, 0; target | &other);
    let negative = black_box(BoxedNonZeroInt::new(boxed_int(-5)).expect("not zero"));
    allocates!(0, 0; -negative);
  }

  /// A conversion that fails on its last check builds no integer: the text of zero and the cast of
  /// zero into a non-zero integer, and the exact conversion of a float with a fraction.
  #[test]
  fn errors_build_nothing() {
    assert!(allocates!(0, 0; "0".parse::<BoxedNonZero>()).is_err());
    assert!(allocates!(1, 0; "5".parse::<BoxedNonZero>()).is_ok());
    assert!(allocates!(0, 0; BoxedNonZero::checked_cast_from(0u64)).is_none());
    assert!(allocates!(1, 0; BoxedNonZero::checked_cast_from(5u64)).is_some());
    assert!(allocates!(0, 0; Boxed::exact_from_f64(0.5)).is_err());
    assert!(allocates!(1, 0; Boxed::exact_from_f64(5.0)).is_ok());
  }

  /// The casts of the cast traits build their target from the words of their source, also where
  /// both are one type: a cast allocates its target, and frees a source given by value.
  #[test]
  fn casts_build_their_target() {
    let (a, b) = pair();
    assert!(allocates!(1, 1; Boxed::checked_cast_from(a)).is_some());
    assert!(allocates!(1, 0; Boxed::checked_cast_from(&b)).is_some());
    assert!(allocates!(1, 1; BoxedInt::checked_cast_from(b)).is_some());
  }

  /// A constructor from digits builds the integer that it returns and nothing for `None`, and the
  /// iterator of the digits allocates nothing.
  #[test]
  fn digits_are_read_into_the_integer_alone() {
    reads_digits!(Boxed, 1);
  }

  /// Each form that returns a `Vec` or a `String` allocates what it returns, once, and nothing
  /// else: the limbs stay in their box.
  #[test]
  fn vectors_and_texts_are_allocated_once() {
    writes_vectors_and_text!(Boxed, BoxedInt, BoxedNonZero, BoxedNonZeroInt);
  }
}

/// Emits the checks of a shared storage, the pointer `$pointer` of four limbs at 256 bits, in the
/// module `$module`. A result is a new allocation, also where a `Box` stores in place, unless it is
/// the allocation of an operand, and an integer that an operation takes gives up its share unless
/// the result is its allocation: the allocation is freed where the integer was its only owner, and
/// kept, with its value, where another owner holds it.
macro_rules! shared_storage {
  ($module:ident, $pointer:ident) => {
    mod $module {
      use super::*;

      /// An unsigned integer of 256 bits in a shared pointer to four limbs.
      type Shared = Uint<U256, $pointer<[u64; 4]>>;

      /// The signed integer of the same width and storage.
      type SharedInt = Int<U256, $pointer<[u64; 4]>>;

      /// The non-zero integer over `Shared`.
      type SharedNonZero = NonZeroUint<U256, $pointer<[u64; 4]>>;

      /// The non-zero integer over `SharedInt`.
      type SharedNonZeroInt = NonZeroInt<U256, $pointer<[u64; 4]>>;

      /// The shared unsigned integer of `value`, with one owner, observed.
      fn shared(value: u64) -> Shared {
        black_box(Shared::from(value))
      }

      /// The shared signed integer of `value`, with one owner, observed.
      fn shared_int(value: i64) -> SharedInt {
        black_box(SharedInt::from(value))
      }

      /// Two shared integers, a larger and a smaller one, each with one owner.
      fn pair() -> (Shared, Shared) {
        (shared(1_000_000), shared(7))
      }

      /// A shared non-zero integer, with one owner.
      fn divisor() -> SharedNonZero {
        black_box(SharedNonZero::new(shared(3)).expect("not zero"))
      }

      /// The largest shared unsigned integer, observed.
      fn largest() -> Shared {
        black_box(<Shared as FixedInteger>::max_value())
      }

      /// The smallest shared signed integer, observed.
      fn smallest() -> SharedInt {
        black_box(<SharedInt as FixedInteger>::min_value())
      }

      /// An operation on references that returns an integer allocates that integer, and nothing
      /// else.
      #[test]
      fn results_are_new_allocations() {
        let (a, b) = pair();
        allocates!(1, 0; &a + &b);
        allocates!(1, 0; &a - &b);
        allocates!(1, 0; &a * &b);
        allocates!(1, 0; &a / &b);
        allocates!(1, 0; &a % &b);
        allocates!(1, 0; &a & &b);
        allocates!(1, 0; &a | &b);
        allocates!(1, 0; &a ^ &b);
        allocates!(1, 0; !&a);
        allocates!(1, 0; &a << 7u32);
        allocates!(1, 0; &a >> 7u32);
        allocates!(1, 0; a.isqrt());
        allocates!(1, 0; a.wrapping_pow(3));
        allocates!(1, 0; a.checked_add(&b));
        allocates!(1, 0; a.wrapping_mul(&b));
        let divisor = divisor();
        allocates!(1, 0; &a / &divisor);
        allocates!(1, 0; &a % &divisor);
        let negative = shared_int(-1_000_000);
        allocates!(1, 0; -&negative);
      }

      /// An operator with an owned operand allocates its result, where a `Box` stores it in that
      /// operand, and frees each operand that it owns alone.
      #[test]
      fn owned_operands_give_up_their_share() {
        let (a, b) = pair();
        allocates!(1, 2; a + b);
        let (a, b) = pair();
        allocates!(1, 1; a + &b);
        let (a, b) = pair();
        allocates!(1, 1; &a - b);
        let (a, b) = pair();
        allocates!(1, 2; a * b);
        let (a, b) = pair();
        allocates!(1, 2; a / b);
        let (a, b) = pair();
        allocates!(1, 1; &a % b);
        let (a, b) = pair();
        allocates!(1, 1; a ^ &b);
        let (a, _) = pair();
        allocates!(1, 1; !a);
        let (a, _) = pair();
        allocates!(1, 1; a << 3u32);
        let (a, _) = pair();
        allocates!(1, 1; a >> 3u32);
        let negative = shared_int(-1_000_000);
        allocates!(1, 1; -negative);

        // By a non-zero divisor.
        let (a, _) = pair();
        let by = divisor();
        allocates!(1, 2; a / by);
        let (a, _) = pair();
        let by = divisor();
        allocates!(1, 1; a % &by);
        let (a, _) = pair();
        let by = divisor();
        allocates!(1, 1; &a / by);

        // An operand that another owner holds too is not freed, and keeps its value where it is.
        let (a, b) = pair();
        let (first, second) = (a.clone(), b.clone());
        let (at_first, at_second) = (address(&first), address(&second));
        let sum = allocates!(1, 0; a + b);
        assert_eq!(sum.to_string(), "1000007");
        assert_eq!((address(&first), address(&second)), (at_first, at_second));
        assert_eq!(first.to_string(), "1000000");
        assert_eq!(second.to_string(), "7");
        assert_ne!(address(&sum), at_first);
        assert_ne!(address(&sum), at_second);
      }

      /// `clone` shares the allocation, `clone_from` takes a share of the source's, and `set_bit`
      /// puts the changed value in a new allocation, which leaves the other owners their value.
      #[test]
      fn shares_and_writes() {
        let (mut a, b) = pair();
        let other = allocates!(0, 0; a.clone());
        assert_eq!(address(&other), address(&a));

        // `a` has another owner: its old allocation stays, with its value.
        allocates!(1, 0; a.set_bit(100, true));
        assert_ne!(address(&other), address(&a));
        assert!(a.bit(100));
        assert!(!other.bit(100));

        // `a` is now the only owner of its allocation, which a second write frees.
        allocates!(1, 1; a.set_bit(101, true));
        assert!(a.bit(100) && a.bit(101));

        // The target gives up its allocation and shares that of the source.
        allocates!(0, 1; a.clone_from(&b));
        assert_eq!(address(&a), address(&b));
      }

      /// `Sum` and `Product` compute on the stack and allocate their result once. The items that
      /// the fold owns alone are freed, and those that another owner holds are kept.
      #[test]
      fn folds_allocate_once() {
        let values = [shared(3), shared(5), shared(7)];
        allocates!(1, 0; values.iter().sum::<Shared>());
        allocates!(1, 0; values.iter().product::<Shared>());
        let sum = allocates!(1, 3; values.into_iter().sum::<Shared>());
        assert_eq!(sum.to_string(), "15");

        let values = [shared(3), shared(5), shared(7)];
        let product = allocates!(1, 3; values.into_iter().product::<Shared>());
        assert_eq!(product.to_string(), "105");

        let values = [shared(3), shared(5), shared(7)];
        let others = values.clone();
        let first = address(&others[0]);
        let sum = allocates!(1, 0; values.into_iter().sum::<Shared>());
        assert_ne!(address(&sum), first);
        assert_eq!(address(&others[0]), first);
        assert_eq!(others[0].to_string(), "3");

        allocates!(1, 0; std::iter::empty::<Shared>().sum::<Shared>());
        allocates!(1, 0; std::iter::empty::<Shared>().product::<Shared>());
        allocates!(1, 0; std::iter::empty::<&Shared>().sum::<Shared>());
        allocates!(1, 0; std::iter::empty::<&Shared>().product::<Shared>());
      }

      /// Each way of building an integer allocates once.
      #[test]
      fn construction_allocates_once() {
        allocates!(1, 0; <Shared as FixedInteger>::zero());
        allocates!(1, 0; Shared::default());
        allocates!(1, 0; <Shared as IntegerOne>::one());
        allocates!(1, 0; <Shared as FixedInteger>::min_value());
        allocates!(1, 0; <Shared as FixedInteger>::max_value());
        allocates!(1, 0; <SharedInt as FixedInteger>::min_value());
        allocates!(1, 0; Shared::from(5u64));
        allocates!(1, 0; Shared::checked_cast_from(5u64));
        let parsed = allocates!(1, 0; "340282366920938463463374607431768211456".parse::<Shared>());
        assert!(parsed.is_ok());
        let inline = black_box(Inline::from(5u64));
        allocates!(1, 0; Shared::widen_from(&inline));
      }

      /// The pointer moves into an integer and out of it without a copy, an array of limbs is put
      /// in one allocation, and the limbs of an integer that is taken leave its allocation, which
      /// is freed.
      #[test]
      fn pointers_move_and_limbs_are_stored_once() {
        let pointer = black_box($pointer::new([1u64, 2, 3, 4]));
        let limbs = std::ptr::from_ref(&*pointer);
        let integer = allocates!(0, 0; Shared::from(pointer));
        assert_eq!(address(&integer), limbs);
        let pointer = allocates!(0, 0; <$pointer<[u64; 4]>>::from(integer));
        assert_eq!(std::ptr::from_ref(&*pointer), limbs);
        allocates!(1, 0; Shared::from_limbs(*pointer));
        let integer = allocates!(1, 0; Shared::checked_from_limbs(*pointer)).expect("in range");
        allocates!(0, 1; integer.into_limbs());
      }

      /// What reads a value without returning an integer in a heap storage does not allocate.
      #[test]
      fn reads_do_not_allocate() {
        let (a, b) = pair();
        allocates!(0, 0; u64::checked_cast_from(&a));
        allocates!(0, 0; Inline::widen_from(&a));
        allocates!(0, 0; a == b);
        allocates!(0, 0; a < b);
        allocates!(0, 0; a.cmp(&b));
        allocates!(0, 0; a.count_ones());
        allocates!(0, 0; a.leading_zeros());
        allocates!(0, 0; a.bit(3));
      }

      /// The text of a shared integer allocates what the text of its inline twin does.
      #[test]
      fn the_text_allocates_as_for_the_inline_twin() {
        let (a, _) = pair();
        let twin = black_box(Inline::from(1_000_000u64));
        let (inline, expected) = counted(|| twin.to_string());
        let (text, counts) = counted(|| a.to_string());
        assert_eq!((text, counts), (inline, expected));
        let (inline, expected) = counted(|| format!("{twin:x}"));
        let (text, counts) = counted(|| format!("{a:x}"));
        assert_eq!((text, counts), (inline, expected));
      }

      /// The pointer is its own non-zero storage: the conversions between an integer and its
      /// non-zero integer move it, and a row that returns no integer lends it. A cast between the
      /// signs, which a `Box` makes in place, allocates its result.
      #[test]
      fn nonzero_integers_move_and_lend_their_pointer() {
        let (a, b) = pair();
        let nonzero = allocates!(0, 0; SharedNonZero::new(a)).expect("not zero");
        let integer = allocates!(0, 0; nonzero.get());
        let nonzero = allocates!(0, 0; SharedNonZero::try_from(integer)).expect("not zero");
        let signed = allocates!(1, 1; nonzero.cast_signed());
        let nonzero = allocates!(1, 1; signed.cast_unsigned());
        allocates!(0, 0; nonzero.leading_zeros());
        allocates!(0, 0; nonzero.count_ones());
        let other = SharedNonZero::new(b).expect("not zero");
        allocates!(1, 0; nonzero.checked_mul(&other));
        let negative = black_box(SharedNonZeroInt::new(shared_int(-5)).expect("not zero"));
        allocates!(1, 0; negative.unsigned_abs());
        allocates!(1, 0; <SharedNonZero as NonZeroLimits>::min_value());
        allocates!(1, 0; <SharedNonZero as NonZeroLimits>::max_value());
        allocates!(1, 0; <SharedNonZeroInt as NonZeroLimits>::min_value());
      }

      /// A checked method that returns `None` builds no integer, and one that returns `Some`
      /// builds that one, or shares the value of its operand where that is the result.
      #[test]
      fn checked_methods_build_nothing_for_none() {
        let (a, b) = pair();
        let most = largest();
        assert!(allocates!(0, 0; most.checked_add(&b)).is_none());
        assert!(allocates!(0, 0; b.checked_sub(&a)).is_none());
        assert!(allocates!(0, 0; most.checked_mul(&a)).is_none());
        assert!(allocates!(0, 0; a.checked_neg()).is_none());
        assert!(allocates!(0, 0; most.checked_pow(2)).is_none());
        assert!(allocates!(0, 0; a.checked_shl(256)).is_none());
        assert!(allocates!(0, 0; a.checked_shr(256)).is_none());
        assert!(allocates!(1, 0; a.checked_sub(&b)).is_some());
        let least = smallest();
        assert!(allocates!(0, 0; least.checked_abs()).is_none());
        assert!(allocates!(0, 0; least.checked_neg()).is_none());

        // The absolute value of a value that is not negative is that value, which it shares.
        let positive = shared_int(5);
        let same = allocates!(0, 0; positive.checked_abs()).expect("in range");
        assert_eq!(address(&same), address(&positive));
      }

      /// A saturating method that saturates builds its bound, and no wrapped result beside it.
      #[test]
      fn saturating_methods_build_their_bound_alone() {
        let (a, b) = pair();
        let most = largest();
        allocates!(1, 0; most.saturating_add(&b));
        allocates!(1, 0; b.saturating_sub(&a));
        allocates!(1, 0; most.saturating_mul(&a));
        allocates!(1, 0; most.saturating_pow(2));
        let least = smallest();
        let one = shared_int(1);
        let minus_one = shared_int(-1);
        allocates!(1, 0; least.saturating_add(&minus_one));
        allocates!(1, 0; least.saturating_sub(&one));
        allocates!(1, 0; least.saturating_mul(&minus_one));
        allocates!(1, 0; least.saturating_div(&minus_one));
        allocates!(1, 0; least.saturating_pow(3));
        allocates!(1, 0; least.saturating_neg());
        allocates!(1, 0; least.saturating_abs());
      }

      /// The carrying addition and the borrowing subtraction build their result alone, also with
      /// a carry or a borrow to take in, whatever flag they return.
      #[test]
      fn carries_and_borrows_build_one_integer() {
        let (a, b) = pair();
        let most = largest();
        let (_, carry) = allocates!(1, 0; a.carrying_add(&b, true));
        assert!(!carry);
        let (_, carry) = allocates!(1, 0; most.carrying_add(&b, true));
        assert!(carry);
        let (_, borrow) = allocates!(1, 0; a.borrowing_sub(&b, true));
        assert!(!borrow);
        let (_, borrow) = allocates!(1, 0; b.borrowing_sub(&a, true));
        assert!(borrow);
      }

      /// The carrying addition and the borrowing subtraction of `Int` build their result alone,
      /// with a carry or a borrow to take in or without, whether the exact value overflows the
      /// range or not. No result is the value of an operand, which it would share.
      #[test]
      fn signed_carries_and_borrows_build_one_integer() {
        let (a, b) = (shared_int(-1_000_000), shared_int(7));
        let least = smallest();
        let greatest = black_box(<SharedInt as FixedInteger>::max_value());
        let (_, overflow) = allocates!(1, 0; a.carrying_add(&b, true));
        assert!(!overflow);
        let (_, overflow) = allocates!(1, 0; greatest.carrying_add(&b, true));
        assert!(overflow);
        let (_, overflow) = allocates!(1, 0; a.carrying_add(&b, false));
        assert!(!overflow);
        let (_, overflow) = allocates!(1, 0; least.carrying_add(&a, false));
        assert!(overflow);
        let (_, overflow) = allocates!(1, 0; a.borrowing_sub(&b, true));
        assert!(!overflow);
        let (_, overflow) = allocates!(1, 0; least.borrowing_sub(&b, true));
        assert!(overflow);
        let (_, overflow) = allocates!(1, 0; a.borrowing_sub(&b, false));
        assert!(!overflow);
        let (_, overflow) = allocates!(1, 0; greatest.borrowing_sub(&a, false));
        assert!(overflow);
      }

      /// The products with a carry build the two halves that they return, of both signs, and no
      /// half is the value of an operand, which it would share: the low half of a signed product
      /// is the unsigned integer in a pointer of the same kind.
      #[test]
      fn carrying_products_build_both_halves() {
        let (a, b) = pair();
        let halves = allocates!(2, 0; a.carrying_mul(&b, &b));
        assert_eq!(halves, (shared(7_000_007), shared(0)));
        let halves = allocates!(2, 0; a.carrying_mul_add(&b, &b, &a));
        assert_eq!(halves, (shared(8_000_007), shared(0)));
        let (c, d, e) = (shared_int(-1_000_000), shared_int(7), shared_int(-3));
        let (_, high) = allocates!(2, 0; c.carrying_mul(&d, &e));
        assert_eq!(high, shared_int(-1));
        let (_, high) = allocates!(2, 0; c.carrying_mul_add(&d, &e, &d));
        assert_eq!(high, shared_int(-1));
      }

      /// The widening product is in the default storage of its width, which is inline, so it
      /// allocates nothing, of two shared operands, of a shared operand by an inline one and the
      /// reverse, of both signs.
      #[test]
      fn widening_products_allocate_nothing() {
        let (a, b) = pair();
        let twin = black_box(Inline::from(5u64));
        allocates!(0, 0; a.widening_mul(&b));
        allocates!(0, 0; a.widening_mul(&twin));
        allocates!(0, 0; twin.widening_mul(&a));
        let (c, d) = (shared_int(-1_000_000), shared_int(7));
        let signed_twin = black_box(Int::<U256, [u64; 4]>::from(-5i64));
        allocates!(0, 0; c.widening_mul(&d));
        allocates!(0, 0; c.widening_mul(&signed_twin));
        allocates!(0, 0; signed_twin.widening_mul(&c));
      }

      /// The exact division builds its quotient, and nothing for a division that leaves a
      /// remainder.
      #[test]
      fn exact_division_builds_only_a_quotient() {
        let (a, b) = pair();
        assert!(allocates!(0, 0; a.div_exact(&b)).is_none());
        let by = shared(1_000);
        assert!(allocates!(1, 0; a.div_exact(&by)).is_some());
      }

      /// The casts between the signs and the conversion from a byte order that reverses the bytes
      /// allocate their result, where a `Box` keeps its allocation, and free the integer that they
      /// take where it was the only owner.
      #[test]
      fn casts_and_byte_orders_allocate_their_result() {
        let (a, b) = pair();
        let other = a.clone();
        let signed = allocates!(1, 0; a.cast_signed());
        assert_ne!(address(&signed), address(&other));
        assert_eq!(other.to_string(), "1000000");
        let unsigned = allocates!(1, 1; signed.cast_unsigned());
        assert_eq!(unsigned, other);

        // The largest unsigned value is above every signed one, and saturates.
        let most = largest();
        let signed = allocates!(1, 1; most.saturating_cast_signed());
        assert_eq!(signed, <SharedInt as FixedInteger>::max_value());

        // A negative value saturates to zero.
        let negative = shared_int(-5);
        let unsigned = allocates!(1, 1; negative.saturating_cast_unsigned());
        assert_eq!(unsigned, <Shared as FixedInteger>::zero());

        // One of the two reverses the bytes, on every target, and the other returns its integer.
        let twice = allocates!(1, 1; Shared::from_le(Shared::from_be(b)));
        assert_eq!(twice, shared(7).swap_bytes());

        // The conversions to a byte order borrow their integer: the one that reverses the bytes
        // builds an integer, and the other shares the value.
        allocates!(1, 0; (twice.to_be(), twice.to_le()));
      }

      /// The rows of the non-zero integers build what they return: nothing for `None`, and the
      /// bound alone where they saturate.
      #[test]
      fn nonzero_methods_build_only_what_they_return() {
        let least = black_box(<SharedNonZeroInt as NonZeroLimits>::min_value());
        assert!(allocates!(0, 0; least.checked_neg()).is_none());
        allocates!(1, 0; least.saturating_neg());
        assert!(allocates!(0, 0; least.checked_abs()).is_none());
        allocates!(1, 0; least.saturating_abs());
        allocates!(1, 0; least.unsigned_abs());

        let most = black_box(<SharedNonZero as NonZeroLimits>::max_value());
        let one = shared(1);
        assert!(allocates!(0, 0; most.checked_mul(&most)).is_none());
        allocates!(1, 0; most.saturating_mul(&most));
        assert!(allocates!(0, 0; most.checked_pow(2)).is_none());
        allocates!(1, 0; most.saturating_pow(2));
        assert!(allocates!(0, 0; most.checked_add(&one)).is_none());
        allocates!(1, 0; most.saturating_add(&one));

        // The absolute value of a value that is not negative is that value, which it shares.
        let positive = black_box(SharedNonZeroInt::new(shared_int(5)).expect("not zero"));
        let same = allocates!(0, 0; positive.checked_abs()).expect("in range");
        assert_eq!(address(&same.get()), address(&positive.get()));

        // An operator that takes its non-zero integer allocates its result and gives up its share.
        let (taken, other) = (divisor(), divisor());
        allocates!(1, 1; taken | &other);
        let negative = black_box(SharedNonZeroInt::new(shared_int(-5)).expect("not zero"));
        allocates!(1, 1; -negative);
      }

      /// A conversion that fails on its last check builds no integer.
      #[test]
      fn errors_build_nothing() {
        assert!(allocates!(0, 0; "0".parse::<SharedNonZero>()).is_err());
        assert!(allocates!(1, 0; "5".parse::<SharedNonZero>()).is_ok());
        assert!(allocates!(0, 0; SharedNonZero::checked_cast_from(0u64)).is_none());
        assert!(allocates!(1, 0; SharedNonZero::checked_cast_from(5u64)).is_some());
        assert!(allocates!(0, 0; Shared::exact_from_f64(0.5)).is_err());
        assert!(allocates!(1, 0; Shared::exact_from_f64(5.0)).is_ok());
      }

      /// The casts of the cast traits build their target from the words of their source, also
      /// where both are one type: a cast allocates its target, and a source given by value gives up
      /// its share.
      #[test]
      fn casts_build_their_target() {
        let (a, b) = pair();
        assert!(allocates!(1, 1; Shared::checked_cast_from(a)).is_some());
        assert!(allocates!(1, 0; Shared::checked_cast_from(&b)).is_some());
        let other = b.clone();
        assert!(allocates!(1, 0; SharedInt::checked_cast_from(b)).is_some());
        assert_eq!(other.to_string(), "7");
      }

      /// A constructor from digits builds the integer that it returns and nothing for `None`, and
      /// the iterator of the digits allocates nothing.
      #[test]
      fn digits_are_read_into_the_integer_alone() {
        reads_digits!(Shared, 1);
      }

      /// Each form that returns a `Vec` or a `String` allocates what it returns, once, and nothing
      /// else. The forms need the `std` or the `alloc` feature, which a build with `triomphe`
      /// alone does not have.
      #[cfg(any(feature = "std", feature = "alloc"))]
      #[test]
      fn vectors_and_texts_are_allocated_once() {
        writes_vectors_and_text!(Shared, SharedInt, SharedNonZero, SharedNonZeroInt);
      }
    }
  };
}

#[cfg(any(feature = "std", feature = "alloc"))]
shared_storage!(rc, Rc);
#[cfg(all(any(feature = "std", feature = "alloc"), target_has_atomic = "ptr"))]
shared_storage!(arc, Arc);
#[cfg(all(feature = "triomphe", target_has_atomic = "ptr"))]
shared_storage!(triomphe_arc, TriompheArc);

/// The checks that the decimals over every heap storage share, at 256 bits: the operations whose
/// counts do not depend on the pointer.
///
/// A decimal over a heap storage computes in its inline twin, the decimal over `Int<U256, [u64;
/// 4]>`, and builds its storage once, for the value that it returns, after every check: nothing
/// for `None` or an error, and nothing before a panic. Here the decimals differ from the integers
/// above: a decimal stores the twin that it computed and does not compare it with the value that
/// it had, so in a shared storage too a result that equals an operand is a new allocation, and
/// never a share of the operand.
macro_rules! heap_decimal {
  ($pointer:ident) => {
    use std::{
      fmt::{self, Write as _},
      hash::{DefaultHasher, Hash, Hasher},
      num::NonZero,
    };

    use numo::{
      Decimal, DecimalFloat, DecimalOne, FixedDecimal, RoundingMode, Scaled, SignedFixedDecimal,
      typenum::{U8, U10, U18, U58, U60, U320},
    };

    /// A signed decimal of 58 integral and 18 fraction digits over a heap storage of 256 bits.
    type Heap = Decimal<U58, U18, Int<U256, $pointer<[u64; 4]>>>;

    /// A decimal of two integral digits more over a heap storage of 320 bits: a wider target.
    type Wider = Decimal<U60, U18, Int<U320, $pointer<[u64; 5]>>>;

    /// A decimal of 10 integral and 8 fraction digits over the same heap storage as `Heap`.
    type HeapNarrow = Decimal<U10, U8, Int<U256, $pointer<[u64; 4]>>>;

    /// The inline twin of `Heap`.
    type Twin = Decimal<U58, U18, Int<U256, [u64; 4]>>;

    /// An inline decimal too narrow for the largest values of `Heap`.
    type Narrow = Decimal<U10, U8, i64>;

    /// A writer that keeps nothing, so that a format allocates only what the decimal allocates.
    struct Sink;

    impl fmt::Write for Sink {
      fn write_str(&mut self, _: &str) -> fmt::Result {
        Ok(())
      }
    }

    /// The decimal of `text`, with one owner, observed.
    fn heap(text: &str) -> Heap {
      black_box(text.parse().expect("a value of the type"))
    }

    /// Two decimals, a positive and a negative one, each with one owner.
    fn pair() -> (Heap, Heap) {
      (heap("1234.5"), heap("-0.25"))
    }

    /// The largest decimal of the type, observed.
    fn largest() -> Heap {
      black_box(<Heap as FixedDecimal>::max_value())
    }

    /// The `Scaled` over `Heap` of `text`, with one owner, observed.
    fn scaled(text: &str) -> Scaled<Heap> {
      black_box(text.parse().expect("a value of the type"))
    }

    /// Reading a decimal allocates nothing: its twin, its predicates and counts, a comparison, a
    /// hash, a format into a sink, its floats, and the exact product, which is inline.
    #[test]
    fn decimal_reads_allocate_nothing() {
      let (a, b) = pair();
      allocates!(0, 0; a.to_inline());
      allocates!(0, 0; a.is_zero());
      allocates!(0, 0; a.is_positive());
      allocates!(0, 0; b.is_negative());
      allocates!(0, 0; a.is_integer());
      allocates!(0, 0; a.is_multiple_of(&b));
      allocates!(0, 0; a.significant_digits());
      allocates!(0, 0; a.fraction_digits());
      allocates!(0, 0; a.order_of_magnitude());
      allocates!(0, 0; a == b);
      allocates!(0, 0; a.cmp(&b));
      allocates!(0, 0; {
        let mut hasher = DefaultHasher::new();
        a.hash(&mut hasher);
        hasher.finish()
      });
      allocates!(0, 0; write!(Sink, "{a} {b:e} {a:E} {b:?}")).expect("a sink never fails");
      allocates!(0, 0; a.to_f64());
      allocates!(0, 0; a.to_f32());
      allocates!(0, 0; a.exact_mul(&b));
    }

    /// `repr` and `from_repr` in range move the storage, and `from_repr` out of range drops the
    /// storage that it was given.
    #[test]
    fn decimal_storages_move() {
      let (a, _) = pair();
      let repr = allocates!(0, 0; a.repr());
      allocates!(0, 0; Heap::from_repr(repr)).expect("in range");
      let beyond = black_box(<Int<U256, $pointer<[u64; 4]>> as FixedInteger>::max_value());
      assert!(allocates!(0, 1; Heap::from_repr(beyond)).is_none());
    }

    /// A constructor allocates the decimal that it returns, and nothing for an error.
    #[test]
    fn decimal_builders_allocate_their_result() {
      allocates!(1, 0; Heap::from_inline(Twin::ZERO));
      allocates!(1, 0; <Heap as FixedDecimal>::zero());
      allocates!(1, 0; <Heap as FixedDecimal>::max_value());
      allocates!(1, 0; <Heap as FixedDecimal>::min_value());
      allocates!(1, 0; <Heap as DecimalOne>::one());
      allocates!(1, 0; Heap::default());
      allocates!(1, 0; "1234.5".parse::<Heap>()).expect("a value of the type");
      assert!(allocates!(0, 0; "12x".parse::<Heap>()).is_err());
      let mode = RoundingMode::HalfEven;
      allocates!(1, 0; Heap::round_from_f64(1.5, mode)).expect("in range");
      allocates!(1, 0; Heap::round_from_f32(1.5, mode)).expect("in range");
      allocates!(1, 0; Heap::from_f64_shortest(1.5, mode)).expect("in range");
      allocates!(1, 0; Heap::from_f32_shortest(1.5, mode)).expect("in range");
      assert!(allocates!(0, 0; Heap::round_from_f64(f64::NAN, mode)).is_err());
      assert!(allocates!(0, 0; Heap::round_from_f32(f32::INFINITY, mode)).is_err());
      assert!(allocates!(0, 0; Heap::from_f64_shortest(1e300, mode)).is_err());
      // Every finite `f32` is below `10^39`, which 58 integral digits hold: only a value that is not
      // finite fails here.
      assert!(allocates!(0, 0; Heap::from_f32_shortest(f32::NEG_INFINITY, mode)).is_err());
    }

    /// `Deserialize` allocates the decimal, here with a deserializer that lends the string, and
    /// `Serialize` nothing, with a serializer whose `collect_str` does not allocate: the text of
    /// a `Scaled` whose scale is above its fraction digits goes through `collect_str`.
    #[cfg(feature = "serde")]
    #[test]
    fn decimal_serde_allocates_the_value_that_it_reads() {
      allocates!(1, 0; serde_json::from_str::<Heap>("\"1234.5\"")).expect("a value of the type");
      let (a, _) = pair();
      let mut buffer = [0u8; 512];
      allocates!(0, 0; postcard::to_slice(&a, &mut buffer).map(|bytes| bytes.len()))
        .expect("the buffer holds the text");
      let below = Scaled::new(a, 3);
      allocates!(0, 0; postcard::to_slice(&below, &mut buffer).map(|bytes| bytes.len()))
        .expect("the buffer holds the text");
      let above = Scaled::new(heap("1.5"), 30);
      allocates!(0, 0; postcard::to_slice(&above, &mut buffer).map(|bytes| bytes.len()))
        .expect("the buffer holds the text");
    }

    /// A generated decimal is one allocation.
    #[cfg(feature = "arbitrary")]
    #[test]
    fn decimal_generation_allocates_the_value() {
      let bytes = [0x5a; 64];
      allocates!(1, 0; <Heap as arbitrary::Arbitrary>::arbitrary(
        &mut arbitrary::Unstructured::new(&bytes)
      ))
      .expect("enough bytes");
    }

    /// A checked or fallible method allocates the decimal that it returns, and nothing for `None`
    /// or an error.
    #[test]
    fn decimal_fallible_methods_allocate_their_result() {
      let (a, b) = pair();
      let big = largest();
      let zero = heap("0");
      let mode = RoundingMode::HalfEven;
      allocates!(1, 0; a.checked_add(&b)).expect("in range");
      allocates!(1, 0; a.checked_sub(&b)).expect("in range");
      allocates!(1, 0; a.checked_rem(&b)).expect("not zero");
      assert!(allocates!(0, 0; big.checked_add(&a)).is_none());
      assert!(allocates!(0, 0; big.checked_sub(&b)).is_none());
      assert!(allocates!(0, 0; a.checked_rem(&zero)).is_none());
      let step = heap("0.5");
      allocates!(1, 0; a.quantize(&step, mode)).expect("in range");
      assert!(allocates!(0, 0; big.quantize(&step, RoundingMode::Up)).is_err());
      allocates!(1, 0; a.floor()).expect("in range");
      allocates!(1, 0; a.ceil()).expect("in range");
      assert!(allocates!(0, 0; big.ceil()).is_err());
      let digits = NonZero::new(3).expect("not zero");
      allocates!(1, 0; a.round_significant(digits, mode)).expect("in range");
      let one = NonZero::new(1).expect("not zero");
      assert!(allocates!(0, 0; big.round_significant(one, RoundingMode::Up)).is_err());
    }

    /// An infallible method allocates the decimal that it returns, also where its value is that of
    /// the operand.
    #[test]
    fn decimal_methods_allocate_their_result() {
      let (a, b) = pair();
      let big = largest();
      let whole = heap("12");
      let positive = heap("5");
      allocates!(1, 0; a.saturating_add(&b));
      allocates!(1, 0; big.saturating_add(&a));
      allocates!(1, 0; a.saturating_sub(&b));
      allocates!(1, 0; a.trunc());
      allocates!(1, 0; whole.trunc());
      allocates!(1, 0; a.fract());
      allocates!(1, 0; SignedFixedDecimal::neg(&a));
      allocates!(1, 0; SignedFixedDecimal::abs(&b));
      allocates!(1, 0; SignedFixedDecimal::abs(&positive));
      allocates!(1, 0; &a + &b);
      allocates!(1, 0; &a - &b);
    }

    /// A change of type allocates the target where it is a heap decimal, and frees the source
    /// that it takes where that is one.
    #[test]
    fn decimal_changes_of_type_allocate_the_target() {
      let mode = RoundingMode::HalfEven;
      let (a, _) = pair();
      let _: Wider = allocates!(1, 1; a.widen());
      let (a, _) = pair();
      let _: Wider = allocates!(1, 1; a.try_convert()).expect("fits");
      let (a, _) = pair();
      let _: Wider = allocates!(1, 1; a.round_to(mode)).expect("fits");
      let big = largest();
      let failed: Result<HeapNarrow, _> = allocates!(0, 1; big.try_convert());
      assert!(failed.is_err());
      let big = largest();
      let failed: Result<HeapNarrow, _> = allocates!(0, 1; big.round_to(mode));
      assert!(failed.is_err());
      let (a, _) = pair();
      let _: Narrow = allocates!(0, 1; a.try_convert()).expect("fits");
      let (a, _) = pair();
      let _: Narrow = allocates!(0, 1; a.round_to(mode)).expect("fits");
      let inline: Narrow = black_box("1234.5".parse().expect("a value of the type"));
      let _: Heap = allocates!(1, 0; inline.widen());
      let _: Heap = allocates!(1, 0; inline.try_convert()).expect("fits");
    }

    /// `div_round` allocates the quotient where it is a heap decimal, and nothing for an error or
    /// for an inline quotient.
    #[test]
    fn decimal_quotients_allocate_their_result() {
      let mode = RoundingMode::HalfEven;
      let (a, b) = pair();
      let zero = heap("0");
      allocates!(1, 0; a.div_round::<U58, U18, Int<U256, $pointer<[u64; 4]>>, _, _, _>(&b, mode))
        .expect("in range");
      assert!(
        allocates!(0, 0; a.div_round::<U58, U18, Int<U256, $pointer<[u64; 4]>>, _, _, _>(&zero, mode))
          .is_err()
      );
      allocates!(0, 0; a.div_round::<U58, U18, Int<U256, [u64; 4]>, _, _, _>(&b, mode))
        .expect("in range");
    }

    /// A `Scaled` over a heap decimal allocates as its decimal, but for its sums of values, which
    /// fold in the inline twin and free every value.
    #[test]
    fn scaled_heap_decimals_allocate_as_their_decimal() {
      let s = scaled("1234.50");
      let t = scaled("-0.250");
      allocates!(1, 0; "1234.50".parse::<Scaled<Heap>>()).expect("a value of the type");
      allocates!(1, 0; s.checked_add(&t)).expect("in range");
      allocates!(1, 0; &s + &t);
      let values = [scaled("1"), scaled("2.0"), scaled("3.00")];
      allocates!(1, 3; values.into_iter().sum::<Scaled<Heap>>());
      allocates!(1, 0; [&s, &t, &s].into_iter().sum::<Scaled<Heap>>());
    }
  };
}

/// The decimals over a `Box`, which holds its coefficient in place: an operator, a sum or an
/// assignment that takes its decimal by value stores the result in that allocation.
#[cfg(any(feature = "std", feature = "alloc"))]
mod boxed_decimal {
  use super::*;

  heap_decimal!(Box);

  /// An operator with an owned operand stores its result in the allocation of that operand, the
  /// left one of two, and frees the other operand that it owns.
  #[test]
  fn decimal_operators_with_an_owned_operand_reuse_its_allocation() {
    let (a, b) = pair();
    allocates!(0, 1; a + b);
    let (a, b) = pair();
    allocates!(0, 1; a - b);
    let (a, b) = pair();
    allocates!(0, 0; a + &b);
    let (a, b) = pair();
    allocates!(0, 0; a - &b);
    let (a, b) = pair();
    allocates!(0, 0; &a + b);
    let (a, b) = pair();
    allocates!(0, 0; &a - b);
    let (a, _) = pair();
    allocates!(0, 0; -a);
    let (mut a, b) = pair();
    allocates!(0, 1; a += b);
    let (mut a, b) = pair();
    allocates!(0, 0; a += &b);
    let (mut a, b) = pair();
    allocates!(0, 1; a -= b);
    let (mut a, b) = pair();
    allocates!(0, 0; a -= &b);
  }

  /// A sum of values stores the total in the first value and frees the others; a sum of
  /// references, or of nothing, allocates the total.
  #[test]
  fn decimal_sums_reuse_the_first_value() {
    let values = [heap("1"), heap("2"), heap("3")];
    allocates!(0, 2; values.into_iter().sum::<Heap>());
    let one = [heap("1")];
    allocates!(0, 0; one.into_iter().sum::<Heap>());
    allocates!(1, 0; std::iter::empty::<Heap>().sum::<Heap>());
    let (a, b) = pair();
    allocates!(1, 0; [&a, &b, &a].into_iter().sum::<Heap>());
    allocates!(1, 0; std::iter::empty::<&Heap>().sum::<Heap>());
  }

  /// A clone allocates its value; `clone_from` writes into the allocation that it has.
  #[test]
  fn decimal_clones_copy_the_value() {
    let (a, mut b) = pair();
    allocates!(1, 0; a.clone());
    allocates!(0, 0; b.clone_from(&a));
    let s = scaled("1.50");
    allocates!(1, 0; s.clone());
  }

  /// A `Scaled` over a boxed decimal stores as its decimal: its operator with owned operands
  /// keeps the left allocation, and its assignment by reference writes in place.
  #[test]
  fn scaled_boxed_decimals_store_in_place() {
    let s = scaled("1234.50");
    let t = scaled("-0.250");
    allocates!(0, 1; s + t);
    let mut s = scaled("1234.50");
    let t = scaled("-0.250");
    allocates!(0, 0; s += &t);
  }
}

/// The checks of the decimals over the shared pointer `$pointer`, in the module `$module`. A shared
/// pointer never writes the coefficient that it holds: every result is a new allocation, the
/// operands that an operation takes give up their shares, which frees an allocation where the
/// decimal was its only owner, and a clone shares.
macro_rules! shared_decimal {
  ($module:ident, $pointer:ident) => {
    mod $module {
      use super::*;

      heap_decimal!($pointer);

      /// An operator with an owned operand allocates its result and gives up the shares of the
      /// operands that it takes; with a second owner of an operand, that allocation stays.
      #[test]
      fn decimal_operators_with_an_owned_operand_allocate_their_result() {
        let (a, b) = pair();
        allocates!(1, 2; a + b);
        let (a, b) = pair();
        allocates!(1, 2; a - b);
        let (a, b) = pair();
        allocates!(1, 1; a + &b);
        let (a, b) = pair();
        allocates!(1, 1; a - &b);
        let (a, b) = pair();
        allocates!(1, 1; &a + b);
        let (a, b) = pair();
        allocates!(1, 1; &a - b);
        let (a, _) = pair();
        allocates!(1, 1; -a);
        let (a, b) = pair();
        let _owner = black_box(a.clone());
        allocates!(1, 1; a + b);
      }

      /// A sum allocates its total and gives up the shares of the values that it takes.
      #[test]
      fn decimal_sums_allocate_their_total() {
        let values = [heap("1"), heap("2"), heap("3")];
        allocates!(1, 3; values.into_iter().sum::<Heap>());
        let one = [heap("1")];
        allocates!(1, 1; one.into_iter().sum::<Heap>());
        allocates!(1, 0; std::iter::empty::<Heap>().sum::<Heap>());
        let (a, b) = pair();
        allocates!(1, 0; [&a, &b, &a].into_iter().sum::<Heap>());
        allocates!(1, 0; std::iter::empty::<&Heap>().sum::<Heap>());
      }

      /// A clone shares the allocation; `clone_from` gives up the share that it had.
      #[test]
      fn decimal_clones_share_the_value() {
        let (a, mut b) = pair();
        allocates!(0, 0; a.clone());
        allocates!(0, 1; b.clone_from(&a));
        let s = scaled("1.50");
        allocates!(0, 0; s.clone());
      }

      /// A `Scaled` over a shared decimal allocates as its decimal.
      #[test]
      fn scaled_shared_decimals_allocate_their_result() {
        let s = scaled("1234.50");
        let t = scaled("-0.250");
        allocates!(1, 2; s + t);
      }
    }
  };
}

#[cfg(any(feature = "std", feature = "alloc"))]
shared_decimal!(rc_decimal, Rc);
#[cfg(all(any(feature = "std", feature = "alloc"), target_has_atomic = "ptr"))]
shared_decimal!(arc_decimal, Arc);
#[cfg(all(feature = "triomphe", target_has_atomic = "ptr"))]
shared_decimal!(triomphe_arc_decimal, TriompheArc);
