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
//! truncates, saturates or wraps unless asked to. The operators `+`, `-`, `+=` and `-=` between two
//! values of one type, and `Sum`, are exact too: where [`checked_add`](Decimal::checked_add) and
//! [`checked_sub`](Decimal::checked_sub) return `None`, for a result outside the range of the type,
//! they panic, in every build, and [`saturating_add`](Decimal::saturating_add) and
//! [`saturating_sub`](Decimal::saturating_sub) return the limit that the result passes. Rounding
//! takes an explicit [`RoundingMode`]: [`round_to`](Decimal::round_to) converts to fewer fraction
//! digits, [`quantize`](Decimal::quantize) rounds to a multiple of a step and
//! [`div_round`](Decimal::div_round) divides into any decimal type, while
//! [`is_multiple_of`](Decimal::is_multiple_of) checks a value against a step without rounding it.
//! Parsing accepts decimal and exponent notation with `_` separators, including every form that
//! `f64` parses, and rejects infinity, NaN, a negative number for an unsigned storage and digits
//! that do not fit; formatting writes the shortest exact text. See [`Decimal`] for the details.
//!
//! Every decimal type has the constants `INTEGRAL`, `FRACTION` and `DIGITS`, its digit counts, and
//! `SIGNED`, whether its storage is signed. Generic code reaches the constants and methods through
//! traits in two layers, each with one implementation for every storage. The general layer,
//! [`FixedDecimal`], with the digit counts, the queries, the arithmetic, the operators `+` and `-`
//! on values, `Sum` and the functions `zero`, `max_value` and `min_value`, [`SignedFixedDecimal`]
//! and [`UnsignedFixedDecimal`], for each sign, and [`DecimalOne`], with `one`, requires `Clone`
//! but not `Copy`; its functions return the inherent constants `ZERO`, `MAX`, `MIN` and `ONE`. The
//! const layer, [`ConstFixedDecimal`] and [`ConstDecimalOne`], adds `Copy`, the assignments `+=`
//! and `-=` and these constants, which a generic `const fn` can read. [`ExactMul`], [`WidenTo`],
//! [`ConvertTo`] and [`DivRound`] have the methods between decimal types. Code generic over the
//! storage adds the bound `Decimal<I, F, R>: FixedDecimal`. The trait methods are not `const`, so
//! constant contexts call the inherent `const fn`s.
//!
//! # The scale of a text
//!
//! A decimal's text depends only on its value, so `0.00100000` reads as `0.001` and writes back
//! as `0.001`. [`Scaled<D>`](Scaled) keeps the scale of the text beside the decimal, the number of
//! fraction digits that the text showed, for code that must write a text back as it came. It reads
//! the grammar of [`parse`](Decimal::parse), writes at least as many fraction digits as its scale,
//! and compares, orders and hashes by value only. Every decimal trait is implemented for it over
//! any decimal, so generic code takes a `Scaled` and a bare decimal alike: a sum takes the larger
//! scale, an exact product the sum of the scales, and a rounded value or a quotient the fraction
//! digits of its target type. Code generic over the decimal `D` adds the bound
//! `Scaled<D>: FixedDecimal`.
//!
//! ```
//! use numo::{
//!   Decimal, FixedDecimal, Scaled,
//!   typenum::{U8, U10},
//! };
//!
//! type Price = Scaled<Decimal<U10, U8>>;
//!
//! let price: Price = "64250.50000".parse()?;
//! assert_eq!(price.to_string(), "64250.50000");
//! assert_eq!(price.scale(), 5);
//!
//! // The scale is presentation only, and a sum takes the larger one.
//! assert_eq!(price, "64250.5".parse()?);
//! let tick: Price = "0.10".parse()?;
//! assert_eq!(price.checked_add(&tick).unwrap().to_string(), "64250.60000");
//! # Ok::<(), numo::ParseError>(())
//! ```
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
//! for 129 to 1024 bits, such as [`u144`] for `Uint<U144, [u64; 3]>`. It also names `u2048`,
//! `u4096`, `i2048` and `i4096`, which fill their limbs. The non-zero integers have the aliases
//! `NonZeroUN` and `NonZeroIN` for the same widths, such as [`NonZeroU7`] for
//! `NonZeroUint<U7, NonZero<u8>>`, except zero bits, which hold no value that is not zero, and the
//! native widths, whose names `core::num` has. Any other width or storage is written out, such as
//! `Uint<U8192>` or `Uint<U24, [u8; 3]>`, and a width that `typenum` has no constant for is
//! computed, as in `Uint<Sum<U1024, U476>>` for 1500 bits.
//! [`FitsBits`] relates a width to the storages that hold it, [`FillsRepr`] to the storages that it
//! fills, and [`Limbs<Bits, L>`](Limbs) names the array of limbs of type `L` that holds it.
//!
//! With the `std` or the `alloc` feature, the storage can also be an array of limbs in a `Box`,
//! `Box<[L; N]>`, such as `Uint<U4096, Box<[u64; 64]>>`, which keeps the value off the stack. Such
//! an integer is not `Copy`, and has the traits of the general layer below but not the constants
//! and the `const fn`s (see [Heap storages](Uint#heap-storages)).
//!
//! Each type has the constants `BITS`, `ZERO`, `MIN` and `MAX`, and `ONE` from one bit for `Uint`
//! and from two bits for `Int`. In a native integer, its `const fn` constructors are `new`, which
//! panics if the value does not fit in `Bits` bits, and `try_new`, which returns [`TryNewError`]
//! instead, and `value` returns the value. In an array, they are `from_limbs`, for a width that
//! fills its limbs, `checked_from_limbs`, which checks that the limbs hold a value of `Bits` bits,
//! and `checked_from_limbs_slice` with its wrapping, overflowing, saturating and strict forms,
//! which read a slice of limbs of any length, and `to_limbs` and `as_limbs` return the limbs. Its
//! `const fn` queries are `is_zero` and `is_one`, and for `Int` also `is_negative` and
//! `is_positive`. Values compare, hash and format by their numeric value, alike in every storage.
//! Their text is that of the native integers at the width: `from_str_radix`, a `const fn` for the
//! radixes 2 to 36, and `FromStr` parse it with the native error, [`core::num::ParseIntError`],
//! `Display` writes decimal, the radix formats `{:b}`, `{:o}`, `{:x}` and `{:X}` write a negative
//! `Int` in the two's complement of its `Bits` bits, and the exponent formats `{:e}` and `{:E}`
//! round half to even at a precision (see [Text](Uint#text)). Their bytes are the low `BYTES` bytes
//! of the storage, in two's complement for `Int`: `to_le_bytes`, `to_be_bytes` and `to_ne_bytes`
//! return an array whose length is part of its type, up to 8192 bits, which `checked_from_le_bytes`
//! and the other orders read back, and at a width of whole bytes, a [`WholeBytes`], `from_le_bytes`
//! and the others; slices of any length convert back in the checked, wrapping, overflowing,
//! saturating and strict forms, `copy_le_bytes_to` and `copy_be_bytes_to` write the bytes to a
//! buffer at any width, and at a width of whole bytes `swap_bytes`, `to_be`, `to_le`, `from_be` and
//! `from_le` convert the byte order as the native integers do, by value rather than by layout (see
//! [Bytes](Uint#bytes)).
//!
//! Each type also has the arithmetic of the native integers at its width, as `const fn`s:
//! addition, subtraction, multiplication, negation, division, remainder and exponentiation in the
//! `checked_*`, `wrapping_*`, `overflowing_*`, `saturating_*` and `strict_*` forms that the native
//! integers have, the Euclidean, rounded and exact division, the next multiple, the square root
//! and the root of any degree, the logarithms, the midpoint and the absolute difference,
//! `carrying_add`, `borrowing_sub`, `is_multiple_of` and the next power of two for `Uint`, `abs`
//! and its forms, `unsigned_abs`, `signum` and `checked_isqrt` for `Int`, and the operators `+`,
//! `-`, `*`, `/` and `%`, which panic or wrap on overflow as the native integers do (see
//! [Overflow](Uint#overflow)): `/` and `%` panic on a zero divisor and on overflow in every build.
//! Addition, subtraction, multiplication, exact division and the shifts, and negation for `Int`,
//! also have the `unchecked_*` form, as `const unsafe fn`s whose caller guarantees that the
//! `checked_*` method would return `Some`. The bit operations of the native integers work at the
//! width too: the counts of ones and zeros, the highest and the lowest one, the bit reversal, the
//! rotations, the shifts in every form, the unbounded and the exact shifts, a bit read and written
//! by its index, and the casts between `Uint` and `Int` of the same width, with the operators `!`,
//! `&`, `|`, `^`, `<<` and `>>`; a shift amount is bounded by `Bits`, and `bitand`, `bitor`,
//! `bitxor` and `not` do the bitwise operations as `const fn`s for constant contexts.
//!
//! The integers convert between any widths and signs with the casts of the native integers, still
//! unstable: `checked_cast`, `wrapping_cast`, `saturating_cast`, `strict_cast` and the `unsafe`
//! `unchecked_cast` into a `Uint`, an `Int` or a native integer, the checked, strict and unchecked
//! ones also into a non-zero integer, `widen` into an integer of the same sign that is at least as
//! wide and `truncate`, with its checked and saturating forms, into one that is at most as wide,
//! the widths compared at compile time, and the checked, saturating and strict casts between
//! `Uint` and `Int` of the same width (see [Conversions](Uint#conversions)). With the native
//! integers, `From` converts into a numo integer whose width holds every value of the native
//! integer, and `TryFrom` converts a `usize` or an `isize` into any width and a numo integer into
//! each native integer, with the error of the native conversions, [`core::num::TryFromIntError`];
//! `bool` converts both ways. `PartialEq` and `PartialOrd` compare the integers with each native
//! integer, both ways, as integers, whatever the widths and the signs; a comparison of a native
//! integer with an expression whose type is inferred from it, such as `n == m.into()`, then needs
//! the type named. The integers convert to `f64` and `f32` as `as` converts the native integers,
//! to the nearest float, and by `From` where the float holds every value of the width. From a
//! float, `const fn`s truncate toward zero, checked, saturating or strict, as the unstable native
//! conversions do, round with a [`RoundingMode`], or take only an integer, with the error
//! [`FromFloatError`] (see [Floats](Uint#floats)).
//!
//! Generic code reaches the same items through traits, each with one implementation for every
//! storage of a kind, in two layers. The general layer, [`FixedInteger`], with `BITS`, `is_zero`,
//! `is_one`, the operators, the text and the functions `zero`, `min_value`, `max_value` and
//! `from_str_radix`, and [`IntegerOne`], with `one`, requires `Clone` but not `Copy`, and needs of
//! the storage only its storage trait and `Clone`: its functions `zero`, `min_value`, `max_value`
//! and `one` compute the values of the inherent constants `ZERO`, `MIN`, `MAX` and `ONE`.
//! [`IntegerAssign`] adds the compound assignments, for a storage that holds the value in place.
//! The const layer, [`ConstFixedInteger`] and [`ConstIntegerOne`], adds `Copy` and these constants,
//! which a generic `const fn` can read.
//! [`UnsignedFixedInteger`] and [`SignedFixedInteger`] have the methods of each sign, the families
//! [`OverflowingOps`], [`CheckedOps`], [`WrappingOps`], [`SaturatingOps`], [`StrictOps`] and
//! [`UncheckedOps`] the arithmetic, [`IntegerMath`], [`UnsignedMath`] and [`SignedMath`] the
//! functions without an overflow prefix, [`IntegerBits`], [`UnsignedBits`] and [`SignedBits`] the
//! bit operations, [`IntegerBytes`], [`ByteArray`], [`FromByteArray`] and [`IntegerByteOrder`] the
//! bytes, [`CheckedCastFrom`], [`BoundedCastFrom`], [`WidenFrom`] and [`TruncateFrom`], which the
//! target implements, the casts, also those with the native integers and the non-zero integers,
//! [`IntegerFloat`] the conversions with the floats, and [`NativeInteger`], [`ArrayInteger`] and
//! [`FromLimbs`] the construction of each kind. The bound `B: BitWidth` gives `Uint<B>` and
//! `Int<B>` [`FixedInteger`], [`IntegerAssign`] and [`ConstFixedInteger`], the sign traits, the
//! families, the math traits, the bit traits, [`IntegerBytes`], the casts and [`IntegerFloat`];
//! [`IntegerOne`] and [`ConstIntegerOne`] with a width that holds one; and [`IntegerByteOrder`]
//! with `B: WholeBytes`. The trait methods are not `const`, so constant contexts call the inherent
//! `const fn`s, which a concrete type reaches without importing a trait.
//!
//! ```
//! use numo::{
//!   BitWidth, CheckedCastFrom, CheckedOps, FixedInteger, Int, RoundingMode, Uint, i256,
//!   typenum::{U7, U24, U200},
//!   u7,
//! };
//!
//! // A width that fills its limbs takes any bit pattern: -1 has every bit set.
//! let minus_one = i256::from_limbs([u64::MAX; 4]);
//! assert!(minus_one.is_negative() && i256::MIN < minus_one);
//!
//! // Another width checks the limbs, which a slice of any length also holds.
//! type U200x4 = Uint<U200, [u64; 4]>;
//! assert_eq!(U200x4::MAX.to_limbs(), [u64::MAX, u64::MAX, u64::MAX, 0xff]);
//! assert_eq!(U200x4::checked_from_limbs([0, 0, 0, 0x100]), None);
//! assert_eq!(U200x4::saturating_from_limbs_slice(&[u64::MAX; 5]), U200x4::MAX);
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
//! assert_eq!(SMALL / u7::new(7), u7::new(14));
//! assert_eq!(SMALL.checked_rem(&u7::ZERO), None);
//! assert_eq!(SMALL.isqrt(), u7::new(10));
//!
//! // The bits of 7 bits: 100 is `0b110_0100`.
//! assert_eq!(SMALL.count_ones(), 3);
//! assert_eq!(SMALL << 1u32, u7::new(72));
//! assert_eq!(SMALL.checked_shl(7), None);
//!
//! // The text of 7 bits, in which 128 overflows.
//! assert_eq!(format!("{SMALL:#b} {SMALL:x} {SMALL:e}"), "0b1100100 64 1e2");
//! assert_eq!(u7::from_str_radix("64", 16), Ok(SMALL));
//! assert!("128".parse::<u7>().is_err());
//!
//! // The bytes of 7 bits, and of 24, a whole number of bytes, in reverse order.
//! assert_eq!(SMALL.to_le_bytes(), [100]);
//! assert_eq!(Uint::<U24>::new(0x01_0203).swap_bytes(), Uint::<U24>::new(0x03_0201));
//!
//! // The casts of 7 bits, the conversions with the native integers and the comparisons with them.
//! assert_eq!(SMALL.widen::<u64>(), 100);
//! assert_eq!(SMALL.checked_cast::<Int<U7>>(), None);
//! assert_eq!(u7::checked_cast_from(200u64), None);
//! assert_eq!(u64::try_from(SMALL), Ok(100));
//! assert!(SMALL < 128u8 && 300u16 > SMALL && SMALL == 100);
//!
//! // The conversions of 7 bits with the floats.
//! assert_eq!(SMALL.to_f64(), 100.0);
//! assert_eq!(u7::round_from_f64(99.5, RoundingMode::HalfEven), Ok(SMALL));
//! assert_eq!(u7::checked_from_f32(128.5), None);
//!
//! // Generic code over the width alone.
//! fn zero_values<B: BitWidth>(values: &[Int<B>]) -> usize {
//!   values.iter().filter(|value| value.is_zero()).count()
//! }
//! assert_eq!(zero_values(&[Int::<U7>::ZERO, Int::<U7>::MIN]), 1);
//! assert_eq!(zero_values(&[Int::<U200>::ZERO; 3]), 3);
//!
//! fn total<B: BitWidth>(values: &[Uint<B>]) -> Option<Uint<B>> {
//!   values.iter().try_fold(Uint::<B>::ZERO, |sum, value| sum.checked_add(value))
//! }
//! assert_eq!(total(&[u7::new(100), u7::new(27)]), Some(u7::MAX));
//! assert_eq!(total(&[Uint::<U200>::MAX, Uint::<U200>::MAX]), None);
//! ```
//!
//! [`NonZeroUint<Bits, Repr>`](NonZeroUint) and [`NonZeroInt<Bits, Repr>`](NonZeroInt) are the
//! integers of `Bits` bits that are not zero, as the native `NonZero` integers are at the native
//! widths. `Repr` is the storage of a value that is not zero, the [`NonZero`](UintRepr::NonZero)
//! of a storage of the integer: the native `NonZero` of a native storage, such as
//! `core::num::NonZero<u8>`, whose niche gives `Option` the size of the integer, or the array
//! itself. Without `Repr`, the storage is that of the default storage, so `NonZeroUint<U7>` holds a
//! `NonZero<u8>`, and [`NonZeroOf<T>`](NonZeroOf) names the non-zero integer of the integer `T`, as
//! in `NonZeroOf<u7>`. In each storage, `new` makes a value of the integer of the same width and
//! storage, returning `None` for zero, `new_unchecked`, a `const unsafe fn`, leaves that check to
//! the caller, and `get` returns it; `from_str_radix` and `FromStr` parse a value as the integer
//! does, with the error of the kind `Zero` for zero, as the native `NonZero` integers do, the
//! formats write what the integer writes, and at a width of whole bytes the byte order is that of
//! the integer. The non-zero integers have the methods of the native `NonZero` integers at the
//! width as `const fn`s, such as `checked_mul`, `ilog2` and `cast_signed`, the division and the
//! remainder of a [`Uint`] by a [`NonZeroUint`], which cannot panic, `|`, and the negation of a
//! [`NonZeroInt`]. They convert as the native `NonZero` integers do: `TryFrom` converts the integer
//! of the same width and storage into a non-zero integer, with the error of the kind `Zero` for
//! zero, and the conversions with the native `NonZero` integers go by the directions of the
//! integers with the native integers. The casts take them as sources, into any integer, and as
//! targets of the casts that check, which return `None` for zero, and of `widen` from a non-zero
//! integer (see [Conversions](NonZeroUint#conversions)). Generic code reaches them through traits
//! in two layers, as for the integers: [`FixedNonZero`], [`UnsignedNonZero`], [`SignedNonZero`]
//! and [`NonZeroLimits`], which every storage implements, and [`ConstNonZeroLimits`], with the
//! constants `MIN` and `MAX`; [`IntegerNonZero`] relates an integer to its non-zero integer, and
//! [`IntegerByteOrder`] gives them the byte order, as it gives the integers.
//!
//! ```
//! use numo::{NonZeroOf, NonZeroUint, typenum::U7, u7};
//!
//! let parts = NonZeroUint::<U7>::new(u7::new(4)).unwrap();
//! assert_eq!(u7::new(100) / parts, u7::new(25));
//! assert_eq!(parts.checked_mul(&parts).map(|square| square.get()), Some(u7::new(16)));
//! assert_eq!(NonZeroOf::<u7>::new(u7::ZERO), None);
//! assert_eq!(size_of::<Option<NonZeroOf<u7>>>(), 1);
//!
//! // The conversions and the casts, into and from a non-zero integer.
//! assert_eq!(NonZeroOf::<u7>::try_from(u7::new(4)), Ok(parts));
//! assert_eq!(u7::ZERO.checked_cast::<NonZeroOf<u7>>(), None);
//! assert_eq!(parts.widen::<u64>(), 4);
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
//! The crate is `no_std` without the `std` feature, which is a default feature. The `std` and the
//! `alloc` features are independent: either one gives the heap storages, and `alloc` gives them to
//! a `no_std` build, through the `alloc` crate. The crate allocates only in the heap storages and
//! in the boxed iterator that the `quickcheck` feature returns from shrinking, as the `quickcheck`
//! trait requires, so with no feature it never allocates.
//!
//! # `serde`
//!
//! The `serde` feature implements `Serialize` and `Deserialize` for [`Decimal`], [`Scaled`],
//! [`RoundingMode`] and the integers [`Uint`], [`Int`], [`NonZeroUint`] and [`NonZeroInt`]. The
//! impls are written against `serde_core`, whose traits `serde` re-exports from version 1.0.220 on,
//! so they need a build whose `serde` is 1.0.220 or later; an older `serde` defines traits of its
//! own. The feature enables `serde_core` without its default features and nothing else, so it turns
//! on no feature in any other crate of a build, and it needs no allocation.
//!
//! A decimal is written as a string, the text that `Display` writes, and read from a string in the
//! syntax of [`parse`](Decimal::parse), in every format, human-readable or binary. Reading
//! rejects a number, so a decimal never passes through a binary floating-point value, which would
//! change `16.7` to the nearest `f64`. Nor is the coefficient written alone: it depends on the
//! number of fraction digits, and read back into a type with another number it would be off by a
//! power of ten. A `Scaled` is written as its text with its scale and read with the scale of the
//! text, in the same way. A rounding mode is written as its lowercase name, such as `"halfeven"`,
//! and read from a name of any case, with the words of a half mode, and `up` and `05` of `up05`,
//! optionally separated.
//!
//! An integer is written in a form that depends only on its width, its sign and its value, in every
//! format and every storage: up to 128 bits as a number, through the method of the smallest native
//! integer of its sign that holds the width, and above 128 bits as its decimal text. In a
//! human-readable format, a value of up to 64 or of more than 128 bits reads from any integer that
//! the format gives and from a decimal string, and one of 65 to 128 bits from a 128-bit integer, as
//! `u128` reads; in other formats each class reads through its own method. A non-zero integer is
//! written and read as its integer, and zero does not read. The forms, the formats and the limits
//! are in [`serde`](Uint#serde).
//!
//! No error that numo builds names the rejected input. When it reads a decimal or a `Scaled`, every
//! input but a string, such as a number, gets the fixed message
//! `expected a decimal value as a string`, and when it reads an integer, every input but an integer
//! or a decimal string gets `expected an integer`; the fixed message also replaces an error that
//! the format builds from numo's description of the expected input. An error that a format builds
//! without that description, such as ciborium's type errors, passes through as the format wrote it,
//! and so does every other error of the format, such as the end of the input, an I/O error or a
//! missing field. A string that does not parse as a decimal reports its [`ParseError`], and an
//! integer that does not fit reports std's error. This covers numbers only: reading a rounding mode
//! reports the rejected name and the accepted ones.
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
//! The three frameworks use `std`, so these features need a target that has it, and none of them
//! turns on the `std` or the `alloc` feature of this crate.
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(docsrs, allow(unused_attributes))]
#![deny(missing_docs)]

// `std` names the standard library with the `std` feature, and the `alloc` crate in a `no_std`
// build with the `alloc` feature, so that heap code spells its paths through `std` in every build
// that has a heap. A unit-test build always has the standard library, which the tests use.
#[cfg(all(not(feature = "std"), not(test), feature = "alloc"))]
extern crate alloc as std;

#[cfg(any(feature = "std", test))]
extern crate std;

#[cfg(test)]
mod bigint;

pub use typenum;

pub use aliases::*;
pub use decimal::{
  ConstDecimalOne, ConstFixedDecimal, ConvertError, ConvertTo, Decimal, DecimalOne, DivRound,
  DivideError, ExactMul, FixedDecimal, ParseError, ParseErrorKind, QuantizeError, Scaled,
  SignedFixedDecimal, UnsignedFixedDecimal, WidenTo,
};
pub use integer::{
  ArrayInteger, BoundedCastFrom, ByteArray, CheckedCastFrom, CheckedOps, ConstFixedInteger,
  ConstIntegerOne, ConstNonZeroLimits, FixedInteger, FixedNonZero, FromByteArray, FromFloatError,
  FromLimbs, Int, IntegerAssign, IntegerBits, IntegerByteOrder, IntegerBytes, IntegerFloat,
  IntegerMath, IntegerNonZero, IntegerOne, NativeInteger, NonZeroInt, NonZeroLimits, NonZeroOf,
  NonZeroUint, OverflowingOps, SaturatingOps, SignedBits, SignedFixedInteger, SignedMath,
  SignedNonZero, StrictOps, TruncateFrom, TryNewError, Uint, UncheckedOps, UnsignedBits,
  UnsignedFixedInteger, UnsignedMath, UnsignedNonZero, WidenFrom, WrappingOps,
};
pub use layout::{
  DigitCount, DivIntermediate, FitsDigits, Layout, Pick, Storage, Widen, WidenRepr,
};
pub use rounding::RoundingMode;
pub use width::{
  ArraySize, BitWidth, FillsRepr, FitsBits, InlineIntRepr, InlineUintRepr, IntRepr, IntReprMut,
  LimbArray, Limbs, NonZeroIntRepr, NonZeroUintRepr, UintRepr, UintReprMut, WholeBytes,
};

pub mod aliases;
#[cfg(feature = "arbitrary")]
#[cfg_attr(docsrs, doc(cfg(feature = "arbitrary")))]
mod arbitrary;
mod decimal;
mod format;
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
pub mod serde;
mod width;

/// The items that the expansion of [`dec!`] names from the crate that calls it. They are not part
/// of the API, and may change in any release.
#[doc(hidden)]
pub mod __private {
  pub use core::{concat, stringify};

  pub use crate::decimal::{DecimalLiteral, LiteralText};
}
