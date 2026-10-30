//! Exact fixed-point decimal numbers whose digit counts are part of the type.
//!
//! [`Decimal<Integral, Fraction, Repr>`](Decimal) stores the value `c / 10^Fraction` as an integer
//! coefficient `c` of type `Repr`, with `|c| <= 10^(Integral + Fraction) - 1`. The digit counts
//! are [`typenum`] numbers, so capacity is checked during type checking: no value can be created
//! for a type with more than 154 digits or with a `Repr` that cannot hold its digits. `Repr`
//! defaults to the smallest signed integer that holds the digits, up to 153 digits; an unsigned
//! `Repr`, such as `u64` for up to 19 digits, gives a type whose values are never negative.
//!
//! ```
//! use numo::{
//!   Decimal,
//!   typenum::{U8, U10, U12},
//! };
//!
//! type Price = Decimal<U10, U8>; // 18 digits, stored in an `i64`
//! type Quantity = Decimal<U12, U8>; // 20 digits, stored in an `i128`
//!
//! let price: Price = "64250.5".parse()?;
//! let quantity: Quantity = "0.0015".parse()?;
//!
//! // The product type, `Decimal<U22, U16>`, is computed by the compiler; nothing is rounded.
//! let notional = price.exact_mul(&quantity);
//! assert_eq!(notional.to_string(), "96.37575");
//!
//! // Sums are checked against the decimal range, not only the storage range.
//! assert_eq!(Price::MAX.checked_add(&price), None);
//! # Ok::<(), numo::ParseError>(())
//! ```
//!
//! Every operation is exact: it returns the exact result or reports failure, and nothing rounds,
//! truncates or wraps unless asked to. Rounding takes an explicit [`RoundingMode`]:
//! [`round_to`](Decimal::round_to) converts to fewer fraction digits,
//! [`quantize`](Decimal::quantize) rounds to a multiple of a step and
//! [`div_round`](Decimal::div_round) divides into any decimal type, while
//! [`is_multiple_of`](Decimal::is_multiple_of) checks a value against a step without rounding it.
//! Parsing accepts decimal and exponent notation with `_` separators, including every form that
//! `f64` parses, and rejects infinity, NaN and digits that do not fit; formatting writes the
//! shortest exact text. See [`Decimal`] for the details.
//!
//! Every decimal type has the constants `INTEGRAL`, `FRACTION` and `DIGITS`, its digit counts, and
//! `SIGNED`, whether its storage is signed. Generic code reaches the constants and methods through
//! traits in two layers, each with one implementation for every storage. The general layer,
//! [`FixedDecimal`], with the digit counts, the queries, the arithmetic and the functions `zero`,
//! `max_value` and `min_value`, [`SignedFixedDecimal`] and [`UnsignedFixedDecimal`], for each sign,
//! and [`DecimalOne`], with `one`, requires `Clone` but not `Copy`; its functions return the
//! inherent constants `ZERO`, `MAX`, `MIN` and `ONE`. The const layer, [`ConstFixedDecimal`] and
//! [`ConstDecimalOne`], adds `Copy` and these constants, which a generic `const fn` can read.
//! [`ExactMul`], [`WidenTo`], [`ConvertTo`] and [`DivRound`] have the methods between decimal
//! types. Code generic over the storage adds the bound `Decimal<I, F, R>: FixedDecimal`. The trait
//! methods are not `const`, so constant contexts call the inherent `const fn`s.
//!
//! # Wide storage
//!
//! Up to 38 digits, `Repr` is a native integer. Beyond that, up to 153 digits, it is one of the
//! wide integers of this crate, aliases of [`Uint`] and [`Int`] that store 64-bit limbs: the signed
//! [`i192`], [`i256`], [`i320`], [`i384`], [`i448`] and [`i512`] hold 57, 76, 96, 115, 134 and 153
//! digits, and the unsigned [`u192`], [`u256`], [`u320`], [`u384`], [`u448`] and [`u512`] hold 57,
//! 77, 96, 115, 134 and 154. So any uint256 amount, which has at most 78 digits, fits a `u320`. The
//! 154th digit is available only with an explicit `u512`, as in `Decimal<U154, U0, u512>`; the
//! default storage and the product of [`exact_mul`](Decimal::exact_mul) stop at 153 digits, and a
//! product with more digits fails to compile.
//!
//! A wide coefficient is built from its limbs, least significant first, and
//! [`from_repr`](Decimal::from_repr) returns `None` when it is out of range. Parsing decimal text
//! with an exponent gives the same value:
//!
//! ```
//! use numo::{
//!   Decimal,
//!   typenum::{U18, U60},
//!   u320,
//! };
//!
//! // Any uint256 amount of a token with 18 decimals.
//! type TokenAmount = Decimal<U60, U18, u320>;
//!
//! // The raw amount 2^64 + 5 as the four limbs of a uint256, with a fifth limb of zero.
//! let raw: [u64; 4] = [5, 1, 0, 0];
//! let [l0, l1, l2, l3] = raw;
//! let amount = TokenAmount::from_repr(u320::from_limbs([l0, l1, l2, l3, 0])).unwrap();
//! assert_eq!(amount.to_string(), "18.446744073709551621");
//! assert_eq!("18446744073709551621e-18".parse::<TokenAmount>()?, amount);
//! # Ok::<(), numo::ParseError>(())
//! ```
//!
//! # Integers
//!
//! [`Uint<Bits, Repr>`](Uint) and [`Int<Bits, Repr>`](Int) are integers of `Bits` bits, unsigned
//! and in two's complement, stored in `Repr`, which holds at least `Bits` bits: a native integer,
//! `u8` to `u128` for `Uint` and `i8` to `i128` for `Int`, which holds the value itself, or an
//! array of limbs, least significant first, `[u64; N]`, `[u32; N]`, `[u16; N]`, `[u8; N]` or
//! `[u128; N]`. Zero bits hold only zero, in every storage. Without `Repr`, the storage is the
//! default that [`BitWidth`] gives for 0 to 65536 bits: the zero-sized `[u64; 0]` for zero bits,
//! the smallest native integer that holds 1 to 128 bits, and above 128 bits
//! [`Limbs<Bits>`](Limbs), `[u64; ceil(Bits / 64)]`, so `Uint<U7>` is `Uint<U7, u8>` and
//! `Uint<U200>` is `Uint<U200, [u64; 4]>`. `[u64; N]` is the storage of the wide aliases,
//! `[u32; N]` computes with native 64-bit intermediates, and `[u8; N]` has alignment 1 and holds
//! the little-endian bytes of the value on every target. `[u128; N]` has the layout of an array of
//! `u128`, for memory shared with code that stores 128-bit words. Its arithmetic works on the
//! 64-bit halves of each limb, as no wider native integer exists; its speed relative to `[u64; N]`
//! depends on the operation, the width and the target (formatting a value of a few limbs is several
//! times slower), so choose it for layout, not for speed. The module [`aliases`] names `u0` to
//! `u1024` and `i0` to `i1024`, but for the native widths 8, 16, 32, 64 and 128, in the default
//! storage: `[u64; 0]` for zero bits, so that [`u0`] and [`i0`] take no space, the smallest native
//! integer that holds 1 to 127 bits, such as [`u7`] for `Uint<U7, u8>`, and `[u64; ceil(N / 64)]`
//! for 129 to 1024 bits, such as [`u144`] for `Uint<U144, [u64; 3]>`. It
//! also names `u2048`, `u4096`, `i2048` and `i4096`, which fill their limbs. Any other width or
//! storage is written out, such as `Uint<U8192>` or `Uint<U24, [u8; 3]>`, and a width that
//! `typenum` has no constant for is computed, as in `Uint<Sum<U1024, U476>>` for 1500 bits.
//! [`FitsBits`] relates a width to the storages that hold it, [`FillsRepr`] to the storages that it
//! fills, and [`Limbs<Bits, L>`](Limbs) names the array of limbs of type `L` that holds it.
//!
//! Each type has the constants `BITS`, `ZERO`, `MIN` and `MAX`, and `ONE` from one bit for `Uint`
//! and from two bits for `Int`. In a native integer, its `const fn` constructors are `new`, which
//! panics if the value does not fit in `Bits` bits, and `try_new`, which returns [`TryNewError`]
//! instead, and `value` returns the value. In an array, they are `from_limbs`, for a width that
//! fills its limbs, and `checked_from_limbs`, which checks that the limbs hold a value of `Bits`
//! bits, and `to_limbs` and `as_limbs` return the limbs. Its `const fn` queries are `is_zero` and
//! `is_one`, and for `Int` also `is_negative` and `is_positive`. Values compare, hash and format by
//! their numeric value, alike in every storage.
//!
//! Each type also has the arithmetic of the native integers at its width, as `const fn`s:
//! addition, subtraction, multiplication and negation in the `checked_*`, `wrapping_*`,
//! `overflowing_*`, `saturating_*` and `strict_*` forms, `carrying_add` and `borrowing_sub` for
//! `Uint`, `abs` and its forms, `unsigned_abs` and `signum` for `Int`, and the operators `+`, `-`
//! and `*`, which panic or wrap on overflow as the native integers do (see
//! [Overflow](Uint#overflow)). Addition, subtraction and multiplication, and negation for `Int`,
//! also have the `unchecked_*` form, as `const unsafe fn`s whose caller guarantees that the
//! `checked_*` method would return `Some`.
//!
//! Generic code reaches the same items through traits, each with one implementation for every
//! storage of a kind: [`FixedInteger`], with the constants, `is_zero`, `is_one` and the operators,
//! [`UnsignedFixedInteger`] and [`SignedFixedInteger`], with the methods of each sign, [`HasOne`],
//! with `ONE`, the families [`OverflowingOps`], [`CheckedOps`], [`WrappingOps`], [`SaturatingOps`],
//! [`StrictOps`] and [`UncheckedOps`], and for the construction of each kind [`NativeInteger`],
//! [`ArrayInteger`] and [`FromLimbs`]. The bound `B: BitWidth` gives `Uint<B>` and `Int<B>`
//! [`FixedInteger`], the sign traits and the families, and `HasOne` with a width that holds one.
//! The traits are not `const`, so constant contexts use the inherent `const fn`s, which a concrete
//! type reaches without importing a trait.
//!
//! ```
//! use numo::{
//!   BitWidth, CheckedOps, FixedInteger, Int, Uint, i256,
//!   typenum::{U7, U24, U200},
//!   u7,
//! };
//!
//! // A width that fills its limbs takes any bit pattern: -1 has every bit set.
//! let minus_one = i256::from_limbs([u64::MAX; 4]);
//! assert!(minus_one.is_negative() && i256::MIN < minus_one);
//!
//! // Another width checks the limbs.
//! type U200x4 = Uint<U200, [u64; 4]>;
//! assert_eq!(U200x4::MAX.to_limbs(), [u64::MAX, u64::MAX, u64::MAX, 0xff]);
//! assert_eq!(U200x4::checked_from_limbs([0, 0, 0, 0x100]), None);
//! assert_eq!(Int::<U7, [u64; 1]>::MIN.to_string(), "-64");
//!
//! // Three bytes, least significant first, hold the same value as one 64-bit limb.
//! let bytes = Uint::<U24, [u8; 3]>::from_limbs([1, 2, 3]);
//! let word = Uint::<U24, [u64; 1]>::checked_from_limbs([0x03_02_01]).unwrap();
//! assert_eq!(bytes.to_string(), word.to_string());
//!
//! // 7 bits in the default storage, a `u8`, which the alias `u7` names, and 200 bits in four
//! // 64-bit limbs.
//! const SMALL: u7 = u7::new(100);
//! assert_eq!(SMALL.value(), 100);
//! assert!(Uint::<U7>::try_new(128).is_err());
//! assert_eq!(Uint::<U200>::MAX, U200x4::MAX);
//!
//! // The arithmetic of 7 bits, which wraps at 128.
//! assert_eq!(SMALL + u7::new(27), u7::MAX);
//! assert_eq!(SMALL.checked_add(&u7::new(28)), None);
//! assert_eq!(SMALL.wrapping_add(&u7::new(28)), u7::ZERO);
//!
//! // Generic code over the width alone.
//! fn count_zeros<B: BitWidth>(values: &[Int<B>]) -> usize {
//!   values.iter().filter(|value| value.is_zero()).count()
//! }
//! assert_eq!(count_zeros(&[Int::<U7>::ZERO, Int::<U7>::MIN]), 1);
//! assert_eq!(count_zeros(&[Int::<U200>::ZERO; 3]), 3);
//!
//! fn total<B: BitWidth>(values: &[Uint<B>]) -> Option<Uint<B>> {
//!   values.iter().try_fold(Uint::<B>::ZERO, |sum, value| sum.checked_add(value))
//! }
//! assert_eq!(total(&[u7::new(100), u7::new(27)]), Some(u7::MAX));
//! assert_eq!(total(&[Uint::<U200>::MAX, Uint::<U200>::MAX]), None);
//! ```
//!
//! # `typenum`
//!
//! The crate re-exports [`typenum`], so no direct dependency is needed. Digit counts are written
//! as its constants, such as `typenum::U8`, or as `typenum::U<8>`. The constants are not
//! re-exported at the crate root, which keeps the root namespace to this crate's own items.
//!
//! # `no_std`
//!
//! The crate is `no_std` and never allocates, except for the boxed iterator that the `quickcheck`
//! feature returns from shrinking, as the `quickcheck` trait requires.
//!
//! # `serde`
//!
//! The `serde` feature implements `Serialize` and `Deserialize` for [`Decimal`] and
//! [`RoundingMode`]. The impls are written against `serde_core`, whose traits `serde` re-exports
//! from version 1.0.220 on, so they need a build whose `serde` is 1.0.220 or later; an older
//! `serde` defines traits of its own. The feature enables `serde_core` without its default
//! features and nothing else, so it turns on no feature in any other crate of a build, and it
//! needs no allocation.
//!
//! A decimal is written as a string, the text that `Display` writes, and read from a string in the
//! syntax of [`parse`](Decimal::parse), in every format, human-readable or binary. Reading
//! rejects a number, so a decimal never passes through a binary floating-point value, which would
//! change `16.7` to the nearest `f64`. Nor is the coefficient written alone: it depends on the
//! number of fraction digits, and read back into a type with another number it would be off by a
//! power of ten. A rounding mode is written as its lowercase name, such as `"halfeven"`, and
//! read from a name of any case, with the words of a half mode optionally separated.
//!
//! # `arbitrary`, `proptest` and `quickcheck`
//!
//! The `arbitrary`, `proptest` and `quickcheck` features implement the `Arbitrary` trait of each
//! crate for [`Decimal`] and [`RoundingMode`], for fuzzing and property tests. All three draw a
//! decimal the same way. With probability 1/8 it is an edge value: zero, one unit in the last
//! place, one when `Integral` is nonzero, `MAX`, or a coefficient of `10^k`, `10^k - 1`, `5 * 10^k`
//! or `15 * 10^k` for a uniform `k`, the last two being rounding ties. Otherwise the number of
//! digits of its coefficient is uniform between one and `Integral + Fraction`, and the coefficient
//! is uniform among the numbers with that many digits, so small amounts, values with few fraction
//! digits and values of full width are all common. With probability 1/4, a uniform number of its
//! trailing digits, fewer than all of them, is then zero, which makes values on a coarser step,
//! such as `64250.50`, frequent. A signed storage gives a nonzero value either sign with equal
//! probability.
//!
//! The `arbitrary` feature reads one byte for each choice through `Unstructured::int_in_range`,
//! which makes the choices only nearly uniform. As for `arbitrary` itself, which bytes give which
//! value is not stable and may change in any release.
//!
//! ```
//! # #[cfg(feature = "arbitrary")]
//! # {
//! use arbitrary::{Arbitrary, Unstructured};
//! use numo::{
//!   Decimal,
//!   typenum::{U8, U10},
//! };
//!
//! type Price = Decimal<U10, U8>;
//!
//! // Bytes from a fuzzer.
//! let bytes = [3, 17, 4, 2, 9, 9, 1, 3, 5, 8, 0, 6];
//! let price = Price::arbitrary(&mut Unstructured::new(&bytes)).unwrap();
//! assert_eq!(price.to_string().parse(), Ok(price));
//! # }
//! ```
//!
//! The `proptest` and `quickcheck` features shrink a failing decimal by decimal truncation: they
//! try zero, then the magnitude of a negative value, then the value truncated toward zero to its
//! first digit, its first two digits, and so on, and continue from the first candidate that fails.
//! They stop at a failing value none of whose candidates fail, which is not necessarily the
//! smallest failing value. A rounding mode shrinks toward [`RoundingMode::Down`], which truncates.
//! The `proptest` feature enables the `std` feature of `proptest` and adds the module
//! `numo::proptest`, with the strategy and value tree of a decimal.
//!
//! ```
//! # #[cfg(feature = "proptest")]
//! # {
//! use numo::{
//!   Decimal, RoundingMode,
//!   typenum::{U8, U10},
//! };
//! use proptest::prelude::*;
//!
//! type Price = Decimal<U10, U8>;
//!
//! proptest!(|(price in any::<Price>(), mode in any::<RoundingMode>())| {
//!   let tick: Price = "0.01".parse().unwrap();
//!   if let Ok(rounded) = price.quantize(&tick, mode) {
//!     prop_assert!(rounded.is_multiple_of(&tick));
//!   }
//! });
//! # }
//! ```
//!
//! The `quickcheck` feature boxes the iterator that shrinking returns, as the `quickcheck` trait
//! requires.
//!
//! ```
//! # #[cfg(feature = "quickcheck")]
//! # {
//! use numo::{
//!   Decimal,
//!   typenum::{U8, U10},
//! };
//! use quickcheck::QuickCheck;
//!
//! type Price = Decimal<U10, U8>;
//!
//! fn cancels(price: Price) -> bool {
//!   price.checked_add(&-price) == Some(Price::ZERO)
//! }
//!
//! QuickCheck::new().quickcheck(cancels as fn(Price) -> bool);
//! # }
//! ```
//!
//! The three frameworks use `std`, so these features need a target that has it; the code of this
//! crate stays `no_std`.
#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(docsrs, allow(unused_attributes))]
#![deny(missing_docs)]

#[cfg(feature = "quickcheck")]
extern crate alloc;

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod bigint;

pub use typenum;

pub use aliases::*;
pub use decimal::{
  ConstDecimalOne, ConstFixedDecimal, ConvertError, ConvertTo, Decimal, DecimalOne, DivRound,
  DivideError, ExactMul, FixedDecimal, ParseError, ParseErrorKind, QuantizeError,
  SignedFixedDecimal, UnsignedFixedDecimal, WidenTo,
};
pub use integer::{
  ArrayInteger, CheckedOps, FixedInteger, FromLimbs, HasOne, Int, NativeInteger, OverflowingOps,
  SaturatingOps, SignedFixedInteger, StrictOps, TryNewError, Uint, UncheckedOps,
  UnsignedFixedInteger, WrappingOps,
};
pub use layout::{
  DigitCount, DivIntermediate, FitsDigits, Layout, Pick, Storage, Widen, WidenRepr,
};
pub use rounding::RoundingMode;
pub use width::{
  ArraySize, BitWidth, FillsRepr, FitsBits, InlineIntRepr, InlineUintRepr, IntRepr, IntReprMut,
  LimbArray, Limbs, UintRepr, UintReprMut,
};

pub mod aliases;
#[cfg(feature = "arbitrary")]
#[cfg_attr(docsrs, doc(cfg(feature = "arbitrary")))]
mod arbitrary;
mod decimal;
mod int;
mod integer;
mod layout;
#[cfg(feature = "proptest")]
#[cfg_attr(docsrs, doc(cfg(feature = "proptest")))]
pub mod proptest;
#[cfg(feature = "quickcheck")]
mod quickcheck;
mod rounding;
#[cfg(any(feature = "arbitrary", feature = "proptest", feature = "quickcheck"))]
mod sample;
#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
mod serde;
mod width;
