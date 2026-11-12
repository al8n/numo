//! The allocations and the frees of the operations of the integers in a heap storage, counted by a
//! global allocator that this test binary alone has.
//!
//! The address of the limbs shows where a result lives, and not an allocation that came and went:
//! an operation may allocate, free and end at the address it started from. So each check counts the
//! calls of the allocator around one operation, on the thread of its test, and compares them with
//! the documentation of the heap storages.

#![cfg(any(feature = "std", feature = "alloc"))]

use std::{
  alloc::{GlobalAlloc, Layout, System},
  cell::Cell,
  hint::black_box,
  thread::LocalKey,
};

use numo::{
  CheckedCastFrom, CheckedOps, FixedInteger, FixedNonZero, Int, IntegerBits, IntegerMath,
  IntegerOne, NonZeroInt, NonZeroLimits, NonZeroUint, SignedNonZero, Uint, UnsignedNonZero,
  WidenFrom, WrappingOps, typenum::U256,
};

/// The system allocator, with the calls of each thread counted.
struct Counting;

thread_local! {
  /// The calls of `alloc`, `alloc_zeroed` and `realloc` on this thread.
  static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };

  /// The calls of `dealloc` on this thread.
  static FREES: Cell<usize> = const { Cell::new(0) };
}

/// Adds one to `counter` on this thread.
///
/// The cells have a constant initializer and no destructor, so reading and writing one neither
/// allocates nor registers anything, where thread-locals are native, as on the targets that run
/// this test: the allocator is not entered again from here. `try_with` leaves a call uncounted that
/// arrives while the locals of its thread are being destroyed, and the addition wraps, so that
/// nothing here can panic, which an allocator must not do.
fn count(counter: &'static LocalKey<Cell<usize>>) {
  let _ = counter.try_with(|calls| calls.set(calls.get().wrapping_add(1)));
}

// SAFETY: each method gives its arguments unchanged to the method of the same name of `System`
// and returns what that returns. So what a caller must guarantee to this allocator is what
// `System` requires, the blocks of this allocator are those of `System`, and `System` keeps the
// contract of `GlobalAlloc` towards the caller. Counting touches no block and cannot unwind.
unsafe impl GlobalAlloc for Counting {
  unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
    count(&ALLOCATIONS);
    // SAFETY: the caller guarantees that `layout` has a size that is not zero.
    unsafe { System.alloc(layout) }
  }

  unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
    count(&ALLOCATIONS);
    // SAFETY: the caller guarantees that `layout` has a size that is not zero.
    unsafe { System.alloc_zeroed(layout) }
  }

  unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
    count(&ALLOCATIONS);
    // SAFETY: the caller guarantees that `ptr` is a block of this allocator, and so of `System`,
    // that `layout` is the layout it was allocated with, and that `new_size` is not zero and does
    // not overflow `isize` when it is rounded up to the alignment of `layout`.
    unsafe { System.realloc(ptr, layout, new_size) }
  }

  unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
    count(&FREES);
    // SAFETY: the caller guarantees that `ptr` is a block of this allocator, and so of `System`,
    // and that `layout` is the layout it was allocated with.
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
  (result, [after[0] - before[0], after[1] - before[1]])
}

/// Asserts that `$operation` allocates `$allocations` times and frees `$frees` times, and gives its
/// result. The operation borrows what it only reads and takes what it consumes, so an operand that
/// it consumes is freed inside the count, and one that it borrows is not.
macro_rules! allocates {
  ($allocations:literal, $frees:literal; $operation:expr) => {{
    let (result, counts) = counted(|| $operation);
    assert_eq!(counts, [$allocations, $frees], "{}", stringify!($operation));
    result
  }};
}

/// An unsigned integer of 256 bits in a box of four limbs.
type Boxed = Uint<U256, Box<[u64; 4]>>;

/// The signed integer of the same width and storage.
type BoxedInt = Int<U256, Box<[u64; 4]>>;

/// The non-zero integer over `Boxed`.
type BoxedNonZero = NonZeroUint<U256, Box<[u64; 4]>>;

/// The non-zero integer over `BoxedInt`.
type BoxedNonZeroInt = NonZeroInt<U256, Box<[u64; 4]>>;

/// The inline twin of `Boxed`.
type Inline = Uint<U256, [u64; 4]>;

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

/// An operator with an owned operand stores its result in the allocation of that operand, the left
/// one of two, and frees the other operand that it owns.
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

/// `Sum` and `Product` allocate their start, and store each step in the running value. An item
/// that the fold owns is freed.
#[test]
fn folds_allocate_their_start() {
  let values = [boxed(3), boxed(5), boxed(7)];
  allocates!(1, 0; values.iter().sum::<Boxed>());
  allocates!(1, 0; values.iter().product::<Boxed>());
  allocates!(1, 3; values.into_iter().sum::<Boxed>());
}

/// Each way of building an integer allocates once: zero in zeroed memory, and every other value
/// from its array. `clone` allocates the copy.
#[test]
fn construction_allocates_once() {
  allocates!(1, 0; <Boxed as FixedInteger>::zero());
  allocates!(1, 0; Boxed::default());
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
fn text_allocates_as_for_the_inline_twin() {
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
