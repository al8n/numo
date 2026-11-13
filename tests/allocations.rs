//! The allocations and the frees of the operations of the integers in a heap storage, counted by a
//! global allocator that this test binary alone has.
//!
//! The address of the limbs shows where a result lives, and not an allocation that came and went:
//! an operation may allocate, free and end at the address it started from. So each check counts the
//! calls of the allocator around one operation, on the thread of its test, and compares them with
//! what the heap storages promise: an operation builds only the integers that it returns, none
//! when it returns `None` or an error, and one that takes its integer by value stores its result
//! in that allocation, where the storage holds its value in place, as a `Box` does. A shared
//! storage never writes a value that it holds, so every result of one is a new allocation, and an
//! integer that an operation takes gives up its share, which frees its allocation only where no
//! other owner holds it. The casts between two types are the exception for every storage: they
//! build their target from the words of their source.

#![cfg(any(feature = "std", feature = "alloc"))]

#[cfg(target_has_atomic = "ptr")]
use std::sync::Arc;
use std::{
  alloc::{GlobalAlloc, Layout, System},
  cell::Cell,
  hint::black_box,
  rc::Rc,
  thread::LocalKey,
};

use numo::{
  ArrayInteger, CheckedCastFrom, CheckedOps, FixedInteger, FixedNonZero, FromLimbs, Int,
  IntegerBits, IntegerByteOrder, IntegerFloat, IntegerMath, IntegerOne, NonZeroInt, NonZeroLimits,
  NonZeroUint, SaturatingOps, SignedBits, SignedFixedInteger, SignedNonZero, Uint, UnsignedBits,
  UnsignedFixedInteger, UnsignedNonZero, WidenFrom, WrappingOps, typenum::U256,
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

/// The largest boxed unsigned integer, observed.
fn largest() -> Boxed {
  black_box(<Boxed as FixedInteger>::max_value())
}

/// The smallest boxed signed integer, observed.
fn smallest() -> BoxedInt {
  black_box(<BoxedInt as FixedInteger>::min_value())
}

/// The address of the limbs of `value`: that of its allocation.
fn address<T: ArrayInteger>(value: &T) -> *const T::Array {
  std::ptr::from_ref(value.as_limbs())
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

/// The pointer moves into an integer and out of it without a copy, an array of limbs is put in one
/// allocation, and the limbs of an integer that is taken leave its allocation, which is freed.
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

/// A checked method that returns `None` builds no integer, and one that returns `Some` builds that
/// one.
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

/// The carrying addition and the borrowing subtraction build their result alone, also with a carry
/// or a borrow to take in, whatever flag they return.
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

/// Emits the checks of a shared storage, the pointer `$pointer` of four limbs at 256 bits, in the
/// module `$module`. Every result is a new allocation, also where a `Box` stores in place, and an
/// integer that an operation takes gives up its share: the allocation is freed where the integer
/// was its only owner, and kept, with its value, where another owner holds it.
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
    }
  };
}

shared_storage!(rc, Rc);
#[cfg(target_has_atomic = "ptr")]
shared_storage!(arc, Arc);
#[cfg(all(feature = "triomphe", target_has_atomic = "ptr"))]
shared_storage!(triomphe_arc, TriompheArc);
