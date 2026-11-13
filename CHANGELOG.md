# UNRELEASED

FEATURES

- `Decimal<Integral, Fraction, Repr>`: exact fixed-point decimal numbers with type-level digit
  counts, stored in a native integer from `i8` to `i128` or from `u8` to `u128` (up to 38 digits),
  or in a wide integer from `i192` to `i512` or from `u192` to `u512` (up to 153 digits, and 154
  with an explicit `u512`).
  - `const fn` constants and methods for each storage: `ZERO`, `ONE`, `MIN`, `MAX`, `parse`,
    `from_repr`, `repr`, `checked_add`, `checked_sub`, `is_zero` and `is_positive`, plus `neg`,
    `abs` and `is_negative` for signed storages.
  - The operators `+`, `-`, `+=` and `-=` between two values of one type, `+` and `-` also between
    two references, and `Sum` for values and references. A result outside the range of the type
    panics in every build, whatever `overflow-checks` is, with the message of the native
    operators; `checked_add` and `checked_sub` return `None` instead. `*` and `/` are the methods
    `exact_mul` and `div_round`.
  - `const fn` `checked_rem`, the exact remainder with the sign of the dividend, and
    `saturating_add` and `saturating_sub`, which return the limit that the result passes.
  - `const fn` `trunc`, `fract`, `floor` and `ceil`, the parts of a value in its type, where
    `floor` and `ceil` report a result out of range as `ConvertError::OutOfRange`, and
    `is_integer`, `order_of_magnitude`, `significant_digits` and `fraction_digits`, the digits of a
    value as `{:e}` and `Display` write them.
  - `exact_mul`, `widen` and `try_convert` between decimal types.
  - The conversions with the floats, for interchange and display, not for arithmetic that must be
    exact. `to_f64` and `to_f32`, `const fn`s of every storage, return the nearest float, ties to
    even, rounded once, the float that parsing the text of the decimal gives; like `trunc`, `floor`
    and `ceil`, they round without a `RoundingMode`. `to_f32` gives the infinity of the sign of the
    value for a magnitude that rounds to `2^128` or more, a subnormal float for one that rounds
    below `2^-126`, and a zero with the sign of the value from `2^-150` down. From a float, each
    reading is rounded once with a `RoundingMode` to the fraction digits of the type:
    `round_from_f64` and `round_from_f32`, `const fn`s, read the exact binary value, and
    `from_f64_shortest` and `from_f32_shortest` the shortest text that parses back to the float, so
    that `0.1` reads as `0.1`. NaN and the infinities are reported by `FromFloatError`, and `-0.0`
    is zero. The sealed trait `DecimalFloat` gives the six to generic code.
  - Exact parsing through `FromStr`, and the shortest exact text through `Display` and `Debug`.
  - Parsing accepts the numeric strings of the General Decimal Arithmetic specification, every
    form that `f64` parses (signs, `.5`, `5.`, exponents) and `_` separators as in Rust literals,
    and reports infinity and NaN as the new `ParseErrorKind::NotFinite`. Exponents of any size are
    applied exactly.
  - `dec!`, a decimal literal checked at compile time, of the type that its context expects or
    names: `dec!(0.01)`, `dec!(-2.5e-3 as Price)`.
  - Scientific notation through `LowerExp` and `UpperExp`, `{:e}` and `{:E}`: the significant
    digits with the point after the first, and the exponent, as in `6.42505e4`, with zero as `0e0`.
    The flags work as for `Display`, but for `#`: `{:#e}` and `{:#E}` write engineering notation,
    as in `12.345678e6`, with an exponent that is a multiple of three and one to three digits
    before the point. The alternate flag has this meaning for a decimal alone: the exponent formats
    of the floats of the standard library and of `Uint` and `Int` ignore it. The precision is the
    minimum number of mantissa fraction digits, so it never rounds, unlike the precision of the
    primitive types and of `Uint` and `Int`.
  - Unsigned storages from `u8` to `u128`, named explicitly (`u64` holds 19 digits, one more than
    `i64`): values are never negative, parsing reports a negative number as
    `ParseErrorKind::Negative`, and `checked_sub` returns `None` below zero. The product of two
    unsigned storages is unsigned, `widen` never converts a signed storage to an unsigned one, and
    `try_convert` reports a negative value as `OutOfRange`.
  - `ParseErrorKind::Negative`, for `-` followed by a finite number read into an unsigned storage,
    whatever its magnitude, `-0` included. `-` alone, a second sign and other invalid text after
    the sign are `InvalidSyntax`, and `-inf` is `NotFinite`. The kind comes after `NotFinite` and
    before `OutOfRange`, as the sign is decided after the syntax and before the value.
  - Wide storages: `i192`, `i256`, `i320`, `i384`, `i448` and `i512` hold 57, 76, 96, 115, 134 and
    153 digits, and `u192`, `u256`, `u320`, `u384`, `u448` and `u512` hold 57, 77, 96, 115, 134 and
    154, so any uint256 amount fits `Decimal<U60, U18, u320>`. They have the same constants,
    `const fn` methods, parsing and formatting as the native storages; products reach up to 153
    digits, and `widen` and `try_convert` convert between native and wide storages. The 154th digit
    is available only with an explicit `u512`, as in `Decimal<U154, U0, u512>`, because no signed
    storage holds it.
  - The errors `ParseErrorKind`, `ConvertError`, `QuantizeError` and `DivideError` have a
    `const fn` predicate for each variant, such as `is_out_of_range`.
  - The constants `INTEGRAL`, `FRACTION` and `DIGITS`, the digit counts of the type, and `SIGNED`,
    `true` for a signed storage and `false` for an unsigned one, so that generic code can derive a
    display scale or a step from the fraction digits of a type.
  - The sealed traits of a decimal type in two layers, each with one implementation for every
    storage. The general layer requires `Clone` but not `Copy`: `FixedDecimal`, with the digit
    counts, `SIGNED`, the storage as `Repr`, `repr`, `from_repr`, `zero`, `max_value`,
    `min_value`, `is_zero`, `is_positive`, `checked_add`, `checked_sub`, `checked_rem`,
    `saturating_add`, `saturating_sub`, `quantize`, `is_multiple_of`, `trunc`, `fract`, `floor`,
    `ceil`, `is_integer`, `order_of_magnitude`, `significant_digits`, `fraction_digits` and
    `round_significant`, `SignedFixedDecimal`,
    which also requires `Neg` and has `is_negative`, `neg` and `abs`, `UnsignedFixedDecimal`, and
    `DecimalOne`, with `one` where `Integral` is nonzero. Its functions `zero`, `max_value`,
    `min_value` and `one` return the inherent constants `ZERO`, `MAX`, `MIN` and `ONE`. The const
    layer also requires `Copy` and has these constants, which a generic `const fn` can read:
    `ConstFixedDecimal`, with `ZERO`, `MAX` and `MIN`, and `ConstDecimalOne`, with `ONE`.
    `FixedDecimal` requires the operators `+` and `-` on values and `Sum`, and `ConstFixedDecimal`
    the assignments `+=` and `-=`. The sealed traits `ExactMul`, `WidenTo`, `ConvertTo` and
    `DivRound` each have one implementation for the decimal types that the inherent method accepts,
    so that generic code does not repeat its bounds. `ExactMul` has the product type as `Output`,
    which implements `FixedDecimal`. Because the implementations are per storage, code generic over
    `Decimal<I, F, R>` adds the bound `Decimal<I, F, R>: FixedDecimal`. The trait methods are not
    `const`; constant contexts use the inherent `const fn`s.
- `Scaled<D>`: a decimal with the scale of its text, the number of fraction digits that the text
  showed, for code that must write a text back as it came.
  - A `const fn` `parse` for each storage and `FromStr` read the grammar of `Decimal::parse` and
    keep the scale: the fraction digits written, not counting `_`, minus the exponent, and at least
    0, which may exceed the fraction digits of the decimal. A scale above 65535 is the new
    `ParseErrorKind::ScaleTooLarge`, which only a text that `Decimal::parse` accepts can give, and
    which `Decimal::parse` never reports. `dec!` reads a literal of a `Scaled` and keeps its scale.
  - `Display` writes at least as many fraction digits as the scale, or the precision where it is
    larger, never rounded and without allocation; with scale 0 it writes what the decimal writes.
    `Debug` shows the scale, while equality, ordering and hashing compare the value only.
  - `const fn` `new`, `value` and `scale`, and `into_value` and `into_parts`. `From` and `Default`
    give scale 0.
  - The operators `+`, `-`, `+=` and `-=` and `Sum`, as for `Decimal`: their result takes the
    larger scale of its operands, and a sum over an iterator the largest scale of its items.
  - Every decimal trait, implemented once for every decimal that implements it, so that generic
    code takes a `Scaled` and a bare decimal alike: `FixedDecimal`, `ConstFixedDecimal`, the sign
    traits, `DecimalOne`, `ConstDecimalOne` and `DecimalFloat`, and `ExactMul`, `WidenTo`,
    `ConvertTo` and `DivRound` between `Scaled` types, with no mixed implementations. Sums,
    differences and remainders take the larger scale, also where they saturate to a limit, exact
    products the sum of the scales, saturating at 65535, quantized values the scale of the step,
    widened and exactly converted values their own, values of `round_to` and quotients the fraction
    digits of their target type, and zero, the limits, one and a value made from a coefficient
    scale 0. `trunc`, `floor` and `ceil` give scale 0, `fract` keeps the scale, `round_significant`
    gives the scale that the significant digits of its result need, at most 65535, and `is_integer`
    and the digits of a value are those of the value, whatever the scale. A value read from the
    binary value of a float takes the fraction digits of the type, and one read from its shortest
    text the fraction digits of that text, at most those of the type. A concrete
    `Scaled<Decimal<I, F, R>>` needs no bound; code generic over the decimal `D` adds the bound
    `Scaled<D>: FixedDecimal`.
- `RoundingMode`, with the modes `Up`, `Down`, `Ceiling`, `Floor`, `HalfUp`, `HalfDown`,
  `HalfEven` and `Up05`. Rounding happens only where a mode is given, but for `trunc`, `floor` and
  `ceil`, which round in their own direction, and for `to_f64` and `to_f32`, which round to the
  nearest float, for native and wide storages alike:
  - `round_to` converts to any decimal type as `try_convert` does, but rounds dropped fraction
    digits with the mode instead of failing. It reports `OutOfRange` if the rounded value does not
    fit, so a negative value converts to an unsigned storage only when it rounds to zero.
  - `const fn` `quantize` rounds to a multiple of a step of the same type with the mode, and reports
    the new `QuantizeError`: `InvalidStep` for a step that is zero or negative, and `OutOfRange` for
    a multiple outside the range.
  - `div_round` divides by a decimal of any type into any decimal type, rounding the quotient once
    with the mode, and reports the new `DivideError`: `DivisionByZero` for a zero divisor, which
    never gives zero, and `OutOfRange` for a quotient outside the target's range. The intermediate
    is sized at the type level; one of more than 153 digits fails to compile, and 154 is allowed
    only when both storages are unsigned.
  - `const fn` `round_significant` rounds to a number of significant digits with the mode, in the
    type of the value, and reports `OutOfRange` where a carry leaves the range. The count is a
    `NonZero<u32>`, so a count of zero cannot be written.
  - `round_from_f64` and `from_f64_shortest`, and their `f32` forms, round a float once with the
    mode to the fraction digits of a type.
  - `const fn` `is_multiple_of` checks a value against a step without rounding it.
  - `Up05` is the `ROUND_05UP` of the General Decimal Arithmetic: away from zero if the last digit
    of the truncated result is 0 or 5. Its name is `up05` and its predicate `is_up05`.
- The `serde` feature: `Decimal` is written as its text and read from a string, never from a number,
  in every format, `Scaled` likewise with the scale of its text, and `RoundingMode` as its lowercase
  name, read leniently, without allocation in numo. A `Scaled` whose scale is above the fraction
  digits of its decimal is written through `collect_str`, as an integer above 128 bits is, and any
  other through a buffer on the stack, as a `Decimal` is. No error that numo builds names the input:
  every input but a string gets the fixed message `expected a decimal value as a string`, which also
  replaces a format error built from numo's description of the expected input; an error that a
  format builds without numo's description of the expected input passes through as the format wrote
  it, and every other format error keeps its category. The impls use `serde_core`, so they need
  `serde` 1.0.220 or later in the build; the feature enables no feature of any other crate.
- The `serde` feature implements `Serialize` and `Deserialize` for `Uint`, `Int`, `NonZeroUint` and
  `NonZeroInt`, at every width and in every storage, without allocation in numo. A value of up to
  128 bits is written as a number through the method of the smallest native integer of its sign
  that holds the width, `u8` or `i8` from zero to 8 bits up to `u128` or `i128` from 65 to 128, and
  a wider one as its decimal text, in every format; the form depends only on the width, the sign and
  the value. In a human-readable format, a value of up to 64 or of more than 128 bits reads from any
  integer that the format gives and from a decimal string in the syntax of `FromStr`, through the
  format's own description of its values, and a value of 65 to 128 bits from a 128-bit integer, as
  `u128` reads; in other formats each class reads through its own method. As for `u128`, a field of
  65 to 128 bits fails in an untagged enum or a flattened struct. A type above 128 bits reads a JSON
  number outside the 64-bit range only in a build whose serde_json has `arbitrary_precision`; even
  then, not from a `serde_json::Value` when the number is outside the 128-bit range and a 64-bit
  float prints the same digits. Floats never read. A non-zero integer is written as its integer, and
  zero does not read. No error that numo builds names the input; an error that a format builds
  without numo's description of the expected input passes through as the format wrote it.
- The public module `numo::serde`, with helpers for `#[serde(with = "...")]`, each a module with the
  leaves `required`, for a field of the type, and `optional`, for an `Option` of it, which take
  numo's integers and the native integers through a sealed trait: `decimal` writes and reads the
  decimal text at every width; `quantity` Ethereum's JSON-RPC QUANTITY, `0x` and the lowercase
  hexadecimal digits without leading zeros, for unsigned integers, read as go-ethereum's `hexutil`
  decodes a number; `fixed_hex` `0x` and two hexadecimal digits for each byte of the width, a
  negative `Int` as the pattern of its `Bits` bits; `typed` writes the default form and reads only
  that form, by the type in every format: for formats that cannot describe their values, and for
  fields that must not take the other form.
- `numo::serde::be_bytes` and `le_bytes` write and read the bytes of a value, most or least
  significant first, in two's complement for `Int`, up to 8192 bits; `numo::serde::ruint` writes
  the form of ruint's `Uint`, its hexadecimal text in a human-readable format and its big-endian
  bytes in another, and reads everything that ruint 1.21 reads.
- Heap storages, with the `std` feature or the `alloc` feature: `Uint`, `Int`, `NonZeroUint` and
  `NonZeroInt` in a `Box<[L; N]>`, for limbs `L` from `u8` to `u128`, at every width that the array
  `[L; N]` holds, such as `Uint<U4096, Box<[u64; 64]>>`. The integer has the size of a pointer, is
  not `Copy`, and has no constants and no `const fn`s: its API is the traits, with
  `FixedInteger::zero`, `min_value` and `max_value` and `IntegerOne::one` for the constants. An
  operator on two references allocates its result, a compound assignment of `Uint` or `Int` computes
  in place, and an operator with an owned operand stores its result in the allocation of that
  operand, the left one of two. A method builds only the integers that it returns, none when it
  returns `None` or an error, and one that takes its integer by value, as the casts between the
  signs do, stores its result in that allocation, while a cast, such as `checked_cast`, builds its
  target from the words of the source and frees a source given by value. Each operation computes in
  arrays on the stack before it stores, so a heap storage keeps the value off the stack and not the
  working space. `From` moves a `Box<[L; N]>` out of an integer, and into one whose width fills the
  array, without a copy, and `ArrayInteger` reads and builds the limbs. serde writes and reads a
  boxed integer as the inline integer of its width. `Rc<[L; N]>` and, on a target with atomic
  pointers, `Arc<[L; N]>` are storages too, read-only: a clone shares the value, a method makes at
  most one allocation for each integer that it returns and a fold one, a method that takes a shared
  integer by value allocates its result and gives up its share, and there is no compound assignment.
  The `triomphe` feature adds `triomphe::Arc<[L; N]>`, a storage of the same kind without a weak
  count, on a target with atomic pointers. An integer in an `Arc` or a `triomphe::Arc` is `Send` and
  `Sync`, and one in an `Rc` is neither.
- The `std` and `alloc` features are independent, and either one gives the heap storages and the
  forms that return a `Vec` or a `String`: `alloc` gives them to a `no_std` build. With no feature
  the crate is `no_std` and does not allocate.
- The `arbitrary`, `proptest` and `quickcheck` features: the `Arbitrary` trait of each crate for
  `Decimal` and `RoundingMode`, for fuzzing and property tests on a target with `std`. One generator
  serves all three: an edge value one time in eight (zero, one unit in the last place, one, `MAX`,
  powers of ten, runs of nines and the rounding ties `5 * 10^k` and `15 * 10^k`), and otherwise a
  number of digits chosen uniformly and then uniform digits, with trailing zeros one time in four,
  so that every magnitude is common. `proptest` and `quickcheck` shrink a decimal by truncating its
  digits and a rounding mode toward `Down`. The `proptest` feature adds `numo::proptest`, with
  `DecimalStrategy` and `DecimalValueTree`, and the `quickcheck` feature allocates only the boxed
  iterator that its trait requires shrinking to return.
- `Uint<Bits, Repr>` and `Int<Bits, Repr>`: integers of any width `Bits`, stored in a native
  integer, `u8` to `u128` for `Uint` and `i8` to `i128` for `Int`, which holds the value itself, or
  in an array of limbs that has at least `Bits` bits, least significant limb first, in two's
  complement for `Int`: `[u64; N]`, `[u32; N]`, `[u16; N]`, `[u8; N]` or `[u128; N]`. `[u8; N]` has
  alignment 1 and holds the little-endian bytes of the value on every target, and `[u128; N]` has
  the layout of an array of `u128` and works on the 64-bit halves of each limb.
  - A default storage for 0 to 65536 bits, so that `Uint<U7>` is `Uint<U7, u8>` and `Uint<U200>` is
    `Uint<U200, [u64; 4]>`: the zero-sized `[u64; 0]` for 0 bits, the smallest native integer from 1
    to 128 bits and `[u64; ceil(Bits / 64)]` above. `BitWidth` names the mapping, and generic code
    adds `B: BitWidth`. More than 65536 bits have no default; such a type names its storage.
  - The module `aliases`, re-exported at the crate root: `u0` to `u1024` and `i0` to `i1024`, but
    for the native widths 8, 16, 32, 64 and 128, in the default storage: `[u64; 0]` for `u0` and
    `i0`, which hold only 0 and take no space, the smallest native integer that holds 1 to 127 bits,
    and `[u64; ceil(N / 64)]` for 129 to 1024 bits, such as `u144` for `Uint<U144, [u64; 3]>`; and
    `u2048`, `u4096`, `i2048` and `i4096`.
  - `ArraySize` maps a `typenum` number of limbs, from 0 to 1024, to an array. `LimbArray` and
    `Limbs<Bits, L = u64>` name the array of limbs `L` that holds `Bits` bits, such as
    `Limbs<U200>` for `[u64; 4]` or `Limbs<U24, u8>` for `[u8; 3]`.
  - `const` items and methods: `BITS`, `ZERO`, `MIN` and `MAX`; for a native integer, `new`, which
    panics if the value does not fit in `Bits` bits, so that such a value fails to compile in a
    constant, `try_new`, which returns the new `TryNewError` instead, and `value`; for an array,
    `from_limbs` for a width that fills its limbs, `checked_from_limbs` for any width, which
    returns `None` unless the limbs hold a value of `Bits` bits, `to_limbs`, which takes `&self`
    and returns a copy of the limbs, and `as_limbs`, which borrows them; `is_zero`, plus
    `is_negative` and `is_positive` for `Int`.
  - Zero bits: `Uint<U0, R>` and `Int<U0, R>` hold only 0, in every storage, so `ZERO`, `MIN` and
    `MAX` are 0, `try_new` and `checked_from_limbs` accept only 0, and a value writes no word to a
    hasher and formats as 0. Zero bits fill the arrays of no limbs, `[L; 0]`, which take no space.
  - Numeric `Eq`, `Ord` and `Hash`, where the hash depends only on the value and `Bits`: every
    storage writes the same 64-bit words, a native integer those of a one-limb array. Decimal
    `Display` and `Debug`, the latter also under the hexadecimal flags, and `Default`, which is
    zero.
  - `PartialEq` and `PartialOrd` with each of the twelve native integers, both ways, for every
    storage: `x == n`, `n == x`, `x < n` and `n < x` compare the values as integers, whatever the
    widths and the signs, so that `Int::<U7>::new(-1) < 0u8`, `u7::new(5) == 5i8` and
    `u256::MAX > u128::MAX`, and a literal without a suffix on either side compares as an `i32`.
    They cost inference: in every crate whose build loads numo, one that uses it and one that
    depends on a crate that uses it, a comparison `n == e`, `n < e` or `assert_eq!(n, e)` of a
    native integer `n` with an expression `e` whose type is inferred from `n`, such as
    `n == m.into()`, `n == x.try_into().unwrap()`, `n == s.parse().unwrap()`,
    `n == v.iter().sum()`, `n == Default::default()` or a generic call in that place, no longer
    infers (E0283) and needs the type named, as in `n == u64::from(m)` or
    `n == s.parse::<u64>().unwrap()`, and so does such a comparison of a numo integer. Literals,
    `as`, `From` and the other forms compare as before. Integers of two numo types do not compare:
    a cast converts one to the type of the other.
  - The text of the native integers at the width `Bits`. `from_str_radix`, a `const fn` of every
    storage for the radixes 2 to 36, and `FromStr`, for decimal, read an optional `+`, or `-` for
    `Int`, then ASCII digits and letters of either case below the radix, with no whitespace, `_`
    or prefix, and at zero bits every spelling of zero. They return the error of the native
    integers, `core::num::ParseIntError`, with its kinds in the native order: a byte that is not a
    digit is `InvalidDigit`, unless the digits before it already pass the range of `Bits`, which is
    `PosOverflow` or `NegOverflow`, whatever the storage holds. A radix outside 2 to 36 panics in
    every build, reporting the caller. In a constant, the work grows with the digits and with the
    limbs of the storage: a text or a digit sequence that fills 65536 bits in `[u64; 1024]`, or
    16384 bits in `[u8; 2048]`, can exceed the compiler's default limit for evaluating a constant,
    which may then need `#[allow(long_running_const_eval)]`. Far beyond that limit, as for a text
    that fills 65536 bits, the compiler still warns that the evaluation is taking a long time,
    whatever the level of the lint.
  - The radix formats `Binary`, `Octal`, `LowerHex` and `UpperHex`, which write the digits of a
    `Uint` and the two's complement of the `Bits` bits of an `Int`, whatever the storage holds
    above them, and the exponent formats `LowerExp` and `UpperExp`, whose precision rounds the
    digits after the first half to even, as the native integers do. Every flag works as for the
    native integers, and no format allocates: a native storage writes as its native integer, and
    an array writes its radix digits through a buffer of 64 bytes.
  - The bytes of the native integers at the width `Bits`, as `const fn`s of every storage: `BYTES`,
    `ceil(Bits / 8)`; `to_le_bytes`, `to_be_bytes` and `to_ne_bytes`, which return an array whose
    length is part of its type, `[u8; BYTES]`, inferred so that a wrong length fails to compile, up
    to 8192 bits, where the bits of the last byte above the width are zeros for `Uint` and copies of
    the sign for `Int`, as the storage holds them; `checked_from_le_bytes`, `checked_from_be_bytes`
    and `checked_from_ne_bytes`, which return `None` unless those bits are, and at a width of whole
    bytes, which the new sealed marker `WholeBytes` names, `from_le_bytes`, `from_be_bytes` and
    `from_ne_bytes`, which read every array; `checked_from_le_slice` and `checked_from_be_slice`,
    which read a slice of any length as an unsigned integer for `Uint` and in two's complement for
    `Int`, whose sign is the top bit of its most significant byte, and return `None` if that integer
    is outside the range of `Bits` bits, and `wrapping_from_le_slice` and `wrapping_from_be_slice`,
    which reduce it modulo `2^Bits` into the range; `overflowing_from_le_slice` and
    `overflowing_from_be_slice`, which also return whether the integer is outside the range,
    `saturating_from_le_slice` and `saturating_from_be_slice`, which return `MIN` below the range,
    for a negative slice of `Int`, and `MAX` above it, and `strict_from_le_slice` and
    `strict_from_be_slice`, which panic outside the range in every build, reporting the caller;
    `copy_le_bytes_to` and `copy_be_bytes_to`, which write the bytes to the start of a buffer at any
    width and panic if it is shorter, reporting the caller, and their `checked_*` forms, which
    return `None` and leave it unchanged; and `byte`, which reads the byte at an index and panics
    from `BYTES` up, reporting the caller. The methods that read a value borrow it, and nothing
    allocates.
  - The conversions from slices of limbs, as `const fn`s of every array storage, which read a slice
    of limbs of any length, least significant first, as the slices of bytes read the bytes of its
    limbs: `checked_from_limbs_slice`, unsigned for `Uint` and in two's complement for `Int`,
    whose sign is the top bit of the last limb, which returns `None` if the integer is outside the
    range of `Bits` bits, whatever the length; `wrapping_from_limbs_slice`, which reduces it modulo
    `2^Bits` into the range; `overflowing_from_limbs_slice`, which also returns whether it is
    outside the range; `saturating_from_limbs_slice`, which returns `MIN` below the range and `MAX`
    above it; and `strict_from_limbs_slice`, which panics outside the range in every build,
    reporting the caller.
  - The byte order of the native integers at a width of whole bytes, as `const fn`s of every
    storage: `swap_bytes`, which reverses the bytes, and `to_be`, `to_le`, `from_be` and
    `from_le`, which convert the value to and from an order as the native integers do. For an
    array storage they, `to_ne_bytes` and `from_ne_bytes` convert the value, not a layout: the
    limbs stay least significant first on every target. At another width, a call of the byte order
    or of `from_le_bytes` fails to compile with the diagnostic of `WholeBytes`.
  - The sealed traits of the bytes: `IntegerBytes`, with `BYTES`, `byte`, the copies to a buffer
    and the conversions from a slice, which every storage implements, `ByteArray`, with the arrays
    as the associated type `Bytes`, which generic code reads through `AsRef<[u8]>`, up to 8192
    bits, `FromByteArray`, with the conversions from the arrays of whole bytes, and
    `IntegerByteOrder`, with the byte order of whole bytes. The bound `B: BitWidth` gives `Uint<B>`
    and `Int<B>` `IntegerBytes`, and with `B: WholeBytes` `IntegerByteOrder`.
  - Digits in any base, for `Uint` in every storage, through the new trait `UnsignedDigits`.
    `checked_from_radix_be` and `checked_from_radix_le`, `const fn`s of every inline storage, read
    digits in bytes, most or least significant first, in a radix from 2 to 256, and return `None`
    for a digit that is not below the radix and for a value outside the width; no digits are zero,
    and zeros above the value are read. `checked_from_base_be` and `checked_from_base_le` read
    digits in `u64` from an iterator, in any base from 2. `to_base_le` returns the digits of a
    value as an iterator, least significant first, which divides the value once for each run of
    digits that a `u64` holds and allocates nothing; zero has no digits there. A radix or a base
    outside its range panics, reporting the caller. `Int` has no digit sequences: `unsigned_abs`
    gives those of its magnitude.
  - With the `std` or the `alloc` feature, the forms that return a `Vec` or a `String`.
    `to_radix_be` and `to_radix_le` return the digits of a `Uint` in a radix from 2 to 256 as
    bytes, where zero is the one digit 0, and `to_base_be` its digits in `u64`, most significant
    first. `to_str_radix`, of `Uint`, `Int`, `NonZeroUint` and `NonZeroInt` and in `FixedInteger`
    and `FixedNonZero`, returns the text that `from_str_radix` reads back, in a radix from 2 to 36
    with lowercase letters, with `-` before the digits of a negative value, where the radix formats
    write its bits. `to_le_bytes_vec` and `to_be_bytes_vec`, of `Uint` and `Int` and in
    `IntegerBytes`, return the bytes at any width, and `to_le_bytes_trimmed_vec` and
    `to_be_bytes_trimmed_vec` the shortest bytes that `checked_from_le_slice` and
    `checked_from_be_slice` read back as the value: none for zero, and the shortest two's
    complement of a negative value. Each allocates what it returns and nothing else, in every
    storage.
  - The arithmetic of the native integers at the width `Bits`, as `const fn`s of every storage:
    `overflowing_*`, `checked_*`, `wrapping_*` and `strict_*` addition, subtraction, multiplication
    and negation, and `saturating_*` addition, subtraction and multiplication; `carrying_add` and
    `borrowing_sub` for `Uint`; `saturating_neg`, the absolute value as `overflowing_abs`,
    `checked_abs`, `wrapping_abs`, `saturating_abs`, `strict_abs` and `abs`, `unsigned_abs`, which
    returns the `Uint` of the same width, and `signum` for `Int`; `is_one`; and the constant `ONE`
    from one bit for `Uint` and from two bits for `Int`, so that `u0`, `i0` and `i1` have none. The
    strict methods panic on overflow in every build, with the messages of the native integers.
  - The division and the remainder, rounded toward zero and Euclidean, in the `overflowing_*`,
    `checked_*`, `wrapping_*` and `strict_*` forms, and `saturating_div`, the exponentiation in
    every form, and the integer functions of the native integers without an overflow prefix, as
    `const fn`s of every storage: `pow`, `div_euclid`, `rem_euclid`, `div_floor`, `div_ceil`,
    `next_multiple_of` and `checked_next_multiple_of`, `div_exact` and `checked_div_exact`, `isqrt`,
    the root of any degree `nth_root` and `checked_nth_root`, rounded toward zero, `ilog`, `ilog2`
    and `ilog10` and their checked forms, `midpoint`, rounded toward zero, and `abs_diff`, which for
    `Int` returns the `Uint` of the same width; `is_multiple_of` and `next_power_of_two` with its
    checked and wrapping forms for `Uint`, and `checked_isqrt` for `Int`. They compute the results
    of the native integers at the width `Bits`, including `MIN / -1`, which overflows at every
    width, and the powers at zero bits and at one signed bit, which hold no one, so that `pow(0)`
    overflows there. A zero divisor, `MIN / -1` in the strict, Euclidean and rounded division, the
    square root or a root of even degree of a negative value, a root of degree zero, and a logarithm
    of a value that is not positive or to a base below 2 panic in every build with the messages of
    the native integers, and every panic reports the caller. `pow`, `next_multiple_of` and
    `next_power_of_two` repeat an overflow in a native operation, as the operators do: `pow` in a
    native exponentiation, so it follows the `overflow-checks` setting of the calling crate also in
    an unoptimized build, its panic reports a line in `core`, and an overflowing `pow` in a constant
    fails to compile in every build. `div_exact` follows the behavior of the native method, which is
    unstable and computes the remainder first: a zero divisor and `MIN / -1` panic with the messages
    of the remainder.
  - `unchecked_add`, `unchecked_sub`, `unchecked_mul`, `unchecked_div_exact`, `unchecked_shl` and
    `unchecked_shr`, and `unchecked_neg` for `Int`, as `const unsafe fn`s of every storage: the
    caller guarantees that the `checked_*` method of the same operation would return `Some`, and the
    behavior is undefined otherwise. `unchecked_div_exact` computes the checked quotient at every
    width, as the native method is unstable.
  - The operators `+`, `-` and `*` in every form of values and references, `+=`, `-=` and `*=` for
    a value and a reference, `-` for `Int`, `Sum`, and `Product` where `ONE` exists, for every
    storage. On overflow they repeat a native overflow, so that `overflow-checks` decides whether
    they panic or wrap, as for the native integers: with a per-package override, an unoptimized
    build follows the setting of this crate, and an optimized build panics only when both crates
    check. A panic reports the expression of the caller, and a panicking compound assignment leaves
    its target unchanged. `abs` handles `MIN` in the same way.
  - The operators `/` and `%` in every form of values and references, and `/=` and `%=` for a value
    and a reference, for every storage: as the native operators do, they panic on a zero divisor
    and on `MIN / -1` in every build, report the expression of the caller, and leave the target of a
    panicking compound assignment unchanged. `FixedInteger` requires them.
  - The bit operations of the native integers at the width `Bits`, as `const fn`s of every storage:
    `count_ones`, `count_zeros`, `leading_zeros`, `trailing_zeros`, `leading_ones` and
    `trailing_ones`, `highest_one` and `lowest_one`, `isolate_highest_one` and
    `isolate_lowest_one`, `reverse_bits`, `rotate_left` and `rotate_right`, the shifts in the
    `overflowing_*`, `checked_*`, `wrapping_*` and `strict_*` forms, `unbounded_shl` and
    `unbounded_shr`, and `shl_exact` and `shr_exact`, which follow the native methods, still
    unstable (#144336); `bit`, which reads the bit at an index, and `set_bit`, which writes one;
    `bit_width`, `is_power_of_two` and `cast_signed` for `Uint`, and `cast_unsigned` for `Int`, the
    casts reading the bits as the integer of the same width and the other sign in a storage of the
    same size. They compute the results of the native integers at the width `Bits`: the counts and
    indexes ignore the bits of the storage above the width, so the ones of -1 are `Bits`, the
    reversal and the rotations move the `Bits` bits alone, also at a width that is not a whole
    number of bytes, the right shifts of `Int` are arithmetic, a shift amount is a `u32` that
    overflows from `Bits` up, the wrapping shifts take it modulo `Bits`, and at zero bits every
    amount overflows and no rotation takes a remainder by zero. `set_bit` on the sign bit of `Int`
    changes the sign. The strict shifts panic in every build with the messages of the native ones,
    `set_bit` panics on an index of `Bits` or more, and every panic reports the caller.
  - The operators `!`, `&`, `|` and `^` in every form of values and references, and `&=`, `|=` and
    `^=` for a value and a reference, for every storage; they cannot overflow. The operators `<<`
    and `>>`, arithmetic for `Int`, with every primitive integer as the amount, by value and by
    reference, on values and references, and `<<=` and `>>=` with an amount and a reference to one,
    for every storage. An amount that is negative or `Bits` or more, as every amount is at zero
    bits, repeats the overflow of a native shift, so that `overflow-checks` decides whether the
    operator panics, with the message of the native one and reporting the caller, or shifts by the
    amount modulo `Bits`, taken as a mathematical integer, which at a width that is a power of two
    is the mask of the native operators; a panicking compound assignment leaves its target
    unchanged. `FixedInteger` requires the bitwise operators and the shifts by a `u32`, with their
    compound assignments, so that `x << 3` shifts by a `u32` in code bounded by it alone. `bitand`,
    `bitor`, `bitxor` and `not` do the bitwise operations as `const fn`s for constant contexts; they
    take their operands by value, as the methods of the operator traits do, so that method syntax on
    a value calls them also where those traits are in scope.
  - The casts of the native integers, still unstable (#157388, #154330, #154650), between integers
    of any width and sign, as methods of every storage that take the value: `checked_cast`,
    `wrapping_cast`, `saturating_cast`, `strict_cast` and the `unsafe` `unchecked_cast`, whose
    target is a `Uint` or an `Int` of any width and storage or a native integer, and which follow
    `as` and the native casts at every width, so that -1 of 7 bits wraps to 255 in a `u8` and 300 to
    44 in 7 bits; `widen` into an integer of the same sign that is at least as wide, and `truncate`,
    `checked_truncate` and `saturating_truncate` into one that is at most as wide, whose widths and
    signs are compared at compile time, with `usize` and `isize` as in the native tables: of at
    least 16 bits, and no fixed upper bound; and the `const fn`s `checked_cast_signed`,
    `saturating_cast_signed` and `strict_cast_signed` of `Uint` and `checked_cast_unsigned`,
    `saturating_cast_unsigned` and `strict_cast_unsigned` of `Int`, into the integer of the same
    width and the other sign. The strict casts panic in every build with the message of the native
    ones, reporting the caller. The caller of `unchecked_cast` guarantees that `checked_cast` would
    return `Some`, and the behavior is undefined otherwise. Only the casts between the signs are
    `const`: the others are generic over the target, whose trait functions a constant context
    cannot call.
  - The sealed traits `CheckedCastFrom`, `BoundedCastFrom`, `WidenFrom` and `TruncateFrom`, which
    the target implements, with the functions of the casts, such as `checked_cast_from`, for every
    pair of `Uint`, `Int` and the native integers that the casts take, each through one blanket
    implementation: generic code bounded by `T: CheckedCastFrom<S>` takes every pair, and a native
    integer is a target, as in `u8::checked_cast_from(x)`. Their documentation lists the pairs.
    `UnsignedBits` and `SignedBits` have the casts between the signs.
  - The standard conversions with the native integers, by direction: `From` from `u8` to `u128`
    into a `Uint` of at least their width and into an `Int` of more, and from `i8` to `i128` into an
    `Int` of at least their width, which a sealed marker checks with a diagnostic, with the
    `TryFrom` that the standard library derives from it; `TryFrom` from `usize` and `isize` at every
    width; `TryFrom` into each of the twelve native integers at every width, with the error of the
    native conversions, `core::num::TryFromIntError`, of the kind `PosOverflow` above the range and
    `NegOverflow` below it; and `From<bool>` from one bit for `Uint` and from two for `Int`, with
    `TryFrom` into `bool`. A literal without a suffix is an `i32`, so `u256::from(5)` does not
    compile and `u256::from(5u64)` does, while `i256::from(5)` compiles. Between numo integers the
    casts are the conversions: `From` and `TryFrom` there would overlap the standard
    implementations from a type to itself.
  - The conversions with the floats, as `const fn`s of every storage: `to_f64` and `to_f32`, which
    take `&self` and round to the nearest float, ties to even, as `as` converts the native integers,
    with an infinity of the sign of the value for a magnitude that rounds beyond the largest float
    and `+0.0` for zero; `checked_from_f64`, `saturating_from_f64` and `strict_from_f64`, which
    truncate a float toward zero and follow the native `to_int_checked`, `to_int_saturating`, which
    `as` is, and `to_int_strict`, still unstable (#159913): `None` for NaN, an infinity or a
    truncated value outside the range of `Bits` bits, zero for NaN and the bound of the range
    otherwise, and a panic in every build with the message of the native method, reporting the
    caller; `round_from_f64`, which rounds the value of the float to an integer with a
    `RoundingMode`, at every magnitude, so that `HalfEven` rounds to the nearest integer and `0.5`
    to zero, and a negative value that rounds to zero converts; and `exact_from_f64`, which converts
    a float only if it is an integer of the range, `-0.0` included. Each has an `f32` form, which
    converts as the `f64` of the same value does. The new error `FromFloatError`, for the integers
    and the decimals, a `#[non_exhaustive]` enum with a predicate for each kind, reports NaN, the
    infinities, a value above `MAX` and one below `MIN` apart, the rounded value for
    `round_from_f64`, so that `-0.4` converts to zero with `HalfEven`, and a fraction, `Inexact`,
    which only `exact_from_f64` reports, after the range, which it judges on the value itself:
    `255.5` is above 8 unsigned bits and `-0.5` below them. The sealed trait `IntegerFloat` gives
    the conversions to generic code, for every storage, and `B: BitWidth` gives it on `Uint<B>` and
    `Int<B>`. `From` converts a `Uint` of at most 53 bits and an `Int` of at most 54 into `f64`, and
    one of at most 24 and 25 bits into `f32`, which hold every value of such a width, with a
    diagnostic for a wider one, and with the `TryFrom` that the standard library derives from it,
    whose error is `Infallible`; no other `TryFrom` converts between the integers and the floats.
    The conversions compute with the bits of the floats, so they evaluate in constant contexts.
  - The sealed traits `OverflowingOps`, `CheckedOps`, `WrappingOps`, `SaturatingOps`, `StrictOps`,
    `UncheckedOps` and `IntegerOne`, each with one implementation for every storage; `is_one`, the
    operators, their compound assignments and `Sum` on `FixedInteger`, the methods of each sign on
    `UnsignedFixedInteger` and `SignedFixedInteger`, which also requires `Neg` and has
    `unchecked_neg`, and `Product` on `IntegerOne`. The families have the division, the remainder,
    the power and the shifts in their forms, `CheckedOps` also `checked_next_multiple_of` and
    `checked_div_exact`, and `UncheckedOps` also `unchecked_div_exact`, `unchecked_shl` and
    `unchecked_shr`. The sealed traits `IntegerMath`, with the functions without an overflow prefix
    and the associated type `Magnitude`, the type of `abs_diff`: `Self` for `Uint`, and for `Int`
    the `Uint` of the same width, the type that `SignedFixedInteger::Unsigned` also names;
    `UnsignedMath`, with `is_multiple_of`, `is_power_of_two` and the next power of two; and
    `SignedMath`, with `checked_isqrt`. The sealed traits `IntegerBits`, with the bit methods of
    both kinds, `UnsignedBits`, with `bit_width`, `cast_signed` and the associated type `Signed`,
    the `Int` of the same width in a storage of the same size, and `SignedBits`, with
    `cast_unsigned`, which returns the `Magnitude` of `IntegerMath`. These need only the storage
    traits `UintRepr` and `IntRepr`. The unsafe methods of the traits have the
    contract of the inherent ones. The bound `B: BitWidth` gives `Uint<B>` and `Int<B>` the
    families and the operators, and `IntegerOne` and `ConstIntegerOne` with `B: NonZero` for
    `Uint<B>` or `B: IsGreater<U1, Output = True>` for `Int<B>`. The sealed storage traits
    `UintReprMut` and `IntReprMut`, which every native integer and array implements and
    `InlineUintRepr` and `InlineIntRepr` now require, bound the compound assignments.
  - Each storage has its own arithmetic methods, so a path to a method that leaves the storage to
    inference, such as `Uint::checked_add(&a, &b)`, is ambiguous (E0034); method syntax or a path
    with the storage, such as `Uint::<U7>::checked_add(&a, &b)`, resolves it. Generic code bounded
    by these traits and by traits of other crates with the same names, such as `is_zero`, `is_one`,
    `zero`, `one`, `min_value`, `max_value`, `ZERO` or `ONE`, names the trait in the path.
  - `FitsBits` and `FillsRepr`, which relate a width to the storages that hold it and to the
    storages that it fills, with compile-time diagnostics, the sealed storage traits `UintRepr` and
    `IntRepr`, and `InlineUintRepr` and `InlineIntRepr` for the storages that hold the value inline.
    `FitsBits` requires the width in canonical form, as every `typenum` constant and operator result
    is, instead of `NonZero`: zero bits fit every storage, and a width written with leading zero
    bits, such as `UInt<UTerm, B0>`, which `typenum` compares by its length rather than its value,
    is rejected. `UintRepr` has the associated type `Signed`, the storage of `Int` of the same size,
    whose `IntRepr::Unsigned` is the storage again, and `BitWidth` states the pair for the default
    storages, so that `cast_signed` of `Uint<B>` is `Int<B>` in code over `B: BitWidth`.
  - The constants `BITS`, `ZERO`, `MIN` and `MAX` are defined once for every storage, so a constant
    whose storage the expected type names resolves: `let x: Uint<U7> = Uint::MAX;`,
    `let y: u256 = Uint::MAX;` and `Uint::MAX` passed for a parameter of a concrete type compile,
    while `Uint::<U7, _>::MAX` with no expected type needs a type annotation (E0283). Each storage
    has constructors of its own, so a constructor that leaves the storage to inference is ambiguous
    (E0034): `Uint::new(5)`, `Uint::from_limbs(..)`, `Uint::checked_from_limbs(..)`,
    `Uint::from_str_radix(..)` and `Uint::from_le_bytes(..)` with only an expected type. Naming
    the width for the default storage, as in `Uint::<U7>::new(5)` or `Uint::<U200>::MAX`, the full
    type, or an alias, as in `u7::new(5)`, resolves it.
  - The sealed traits of an integer type in two layers, each with one implementation for every
    storage of a kind. The general layer requires `Clone` but not `Copy`: `FixedInteger`, with
    `BITS`, `zero`, `min_value`, `max_value`, `is_zero` and `from_str_radix`, and with
    `FromStr<Err = ParseIntError>`, the radix formats and the exponent formats as supertraits, so
    that code bounded by `FromStr<Err = ParseIntError>` takes the native integers and these alike,
    and `IntegerOne`, with `one`. Its functions `zero`, `min_value`, `max_value` and `one` return
    the inherent constants `ZERO`, `MIN`, `MAX` and `ONE`. The const layer also requires `Copy` and
    has these constants, which a generic `const fn` can read: `ConstFixedInteger`, with `ZERO`,
    `MIN` and `MAX`, and `ConstIntegerOne`, with `ONE`. The other traits are `UnsignedFixedInteger`
    and `SignedFixedInteger`, with `is_negative`, `is_positive` and the unsigned integer of the same
    width, `NativeInteger`, which requires `ConstFixedInteger` and has `new`, `try_new` and `value`,
    `ArrayInteger`, with `checked_from_limbs`, `to_limbs` by reference, `as_limbs`, the type of a
    limb as `Limb` and the conversions from slices of limbs, and `FromLimbs`, with `from_limbs`.
    Each mirrors the inherent items for generic code; the bound `B: BitWidth` gives `Uint<B>` and
    `Int<B>` `FixedInteger`, `ConstFixedInteger` and the sign traits. The trait methods are not
    `const`; constant contexts use the inherent `const fn`s.
  - The operators with an owned operand, `Sum`, `Product`, `SaturatingOps`, `UnsignedFixedInteger`,
    `SignedFixedInteger`, `FixedInteger` and `IntegerOne` need only the storage trait, `UintRepr` or
    `IntRepr`, where they needed an inline storage; `FixedInteger`, `IntegerOne` and
    `SignedFixedInteger` also need the storage to be `Clone`. The compound assignments are no longer
    supertraits of `FixedInteger`. They are those of the new `IntegerAssign`, which the integers
    over a storage that holds its value in place implement and which `ConstFixedInteger` requires;
    code bounded by `FixedInteger` or `IntegerOne` alone that assigns adds `IntegerAssign`. In code
    generic over the storage trait, a method call of an operator trait on a value, such as
    `x.not()`, now takes the value, as it does for a concrete integer. `into_limbs` consumes an
    integer in an array and returns its limbs, inherent and in `ArrayInteger`. The functions of the
    cast traits take a borrowed source, as in `u64::checked_cast_from(&x)`, and `clone_from` copies
    into the storage. An array of a type that is not a limb, as in `Uint<U64, [f32; 4]>`, is
    reported as not being a storage, where the compiler overflowed.
- `NonZeroUint<Bits, Repr>` and `NonZeroInt<Bits, Repr>`: the integers of `Bits` bits that are not
  zero, as the native `NonZero` integers are at the native widths.
  - The storage is the native `NonZero` of a native storage, `core::num::NonZero<u8>` to
    `NonZero<u128>` for `NonZeroUint` and `NonZero<i8>` to `NonZero<i128>` for `NonZeroInt`, or an
    array of limbs as for `Uint` and `Int`. The sealed storage traits `NonZeroUintRepr` and
    `NonZeroIntRepr` name them, with the storage of the integer as `Integer`, and
    `UintRepr::NonZero` and `IntRepr::NonZero` name the non-zero storage of each storage. Without
    `Repr`, the storage is that of the default storage of the width, which
    `BitWidth::NonZeroUnsigned` and `BitWidth::NonZeroSigned` name, so `NonZeroUint<U7>` holds a
    `NonZero<u8>` and `NonZeroUint<U200>` a `[u64; 4]`.
  - The size, alignment and ABI of the storage; in a native storage, `Option` takes the niche of
    zero, so `Option<NonZeroUint<U7>>` takes one byte.
  - `const fn` `new`, which takes the `Uint` or the `Int` of the same width and storage and returns
    `None` for zero, `const unsafe fn` `new_unchecked`, which leaves that check to the caller, and
    `get`, and `From` to the integer. The constants `BITS`, and from one bit `MIN` and `MAX`: one
    for `NonZeroUint<U1>`, and -1 for `NonZeroInt<U1>`. Zero bits hold no value that is not zero.
  - The methods of the native `NonZero` integers at the width, as `const fn`s of every storage:
    `leading_zeros`, `trailing_zeros`, `count_ones`, which returns a `core::num::NonZero<u32>`,
    `highest_one`, `lowest_one`, `isolate_highest_one`, `isolate_lowest_one`, `reverse_bits`,
    `rotate_left` and `rotate_right`, which follow the native methods, still unstable (#128281),
    `checked_mul`, `saturating_mul`, `checked_pow` and `saturating_pow`; for `NonZeroUint`
    `checked_add` and `saturating_add` of a `Uint`, which may be zero, `checked_next_power_of_two`,
    `ilog2`, `ilog10`, `midpoint`, `is_power_of_two`, `isqrt`, `div_ceil`, `bit_width` and
    `cast_signed`; and for `NonZeroInt` `is_negative`, `is_positive`, the negation and the
    absolute value in the `checked_*`, `overflowing_*`, `wrapping_*` and `saturating_*` forms,
    `abs`, `unsigned_abs`, which returns a `NonZeroUint`, and `cast_unsigned`. A saturating method
    returns the bound of the range that the result passes, which at one signed bit is -1. `abs` of
    `MIN` repeats a native negation, as the operator `-` does.
  - `Clone`, `Copy`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`, `Display` and `Debug` as for
    the integer of the same value, and no `Default`.
  - The text of the native `NonZero` integers: `from_str_radix`, a `const fn` of every storage,
    and `FromStr` parse as the integer of the same width and storage does, then return the error
    of the kind `Zero` for zero, after every error of the integer, so that at zero bits every text
    is an error; the radix and exponent formats write what the integer of the same value writes.
  - The byte order of the native `NonZero` integers at a width of whole bytes, as `const fn`s of
    every storage: `swap_bytes`, `to_be`, `to_le`, `from_be` and `from_le`, which compute the byte
    order of the integer of the same value, which is not zero, and follow the native methods, still
    unstable (#128281); for an array storage they convert the value, not a layout.
    `IntegerByteOrder` gives them to generic code.
  - `Uint / NonZeroUint` and `Uint % NonZeroUint`, with `/=` and `%=`, which cannot panic; `|` of
    two non-zero integers and of a non-zero integer and its integer either way, with `|=`; and the
    negation `-` of `NonZeroInt`, which overflows at `MIN` as the native one does. The operators
    work on values and on references for every storage. `Int` has no division by a `NonZeroInt`, as
    `MIN / -1` overflows.
  - `TryFrom` of the `Uint` or the `Int` of the same width and storage, which returns the error of
    the native conversions, `core::num::TryFromIntError`, of the kind `Zero` for zero, as the
    native `NonZero` integers convert from their integers.
  - The standard conversions with the native `NonZero` integers, by the directions of the integers
    with the native integers: `From` from `NonZero<u8>` to `NonZero<u128>` into a `NonZeroUint` of
    at least their width and into a `NonZeroInt` of more, and from `NonZero<i8>` to `NonZero<i128>`
    into a `NonZeroInt` of at least their width, which the marker of the integers checks, with the
    `TryFrom` that the standard library derives from it; `TryFrom` from `NonZero<usize>` and
    `NonZero<isize>` at every width; and `TryFrom` into the `NonZero` of each of the twelve native
    integers at every width, with the errors of the kinds `PosOverflow` and `NegOverflow`. Between
    numo's non-zero integers, the casts are the conversions, as between the integers.
  - The casts with the non-zero integers: `NonZeroUint`, `NonZeroInt` and the native `NonZero`
    integers are sources of every cast, which converts them as the integers of the same values, and
    targets of the casts that check, those of `CheckedCastFrom`, which return `None` for zero,
    where the strict cast panics in every build with the text of the error of the kind `Zero`,
    reporting the caller, and of `WidenFrom` from a non-zero integer of the same sign that is at
    most as wide, as the native `From` converts between the `NonZero` integers. They are not
    targets of the wrapping, saturating and truncating casts, whose value may be zero, nor of
    `widen` from an integer: these fail to compile, with a diagnostic. `NonZero<usize>` and
    `NonZero<isize>` count as 16 bits, as `usize` and `isize` do, and widen into themselves and
    into their integers, which they also truncate into. `NonZeroUint` and `NonZeroInt` have the
    inherent `checked_cast`, `wrapping_cast`, `saturating_cast`, `strict_cast`, the `unsafe`
    `unchecked_cast`, `widen`, `truncate`, `checked_truncate` and `saturating_truncate`, generic
    over the target, as `Uint` and `Int` have them. The caller of `unchecked_cast` into a non-zero
    integer guarantees a value that is not zero, that is, `checked_cast` would return `Some`; the
    cast checks it in every build and panics there, reporting the caller, as no non-zero integer
    of zero exists to return.
  - The sealed traits `FixedNonZero`, with `Integer`, `BITS`, `new`, `get`, `new_unchecked`, which
    checks the value in every build, `from_str_radix` and the methods of both signs, and with
    `FromStr<Err = ParseIntError>`, the radix formats and the exponent formats as supertraits,
    `UnsignedNonZero` and
    `SignedNonZero`, with the methods of each sign, and `NonZeroLimits`, with `min_value` and
    `max_value` from one bit, which need only the storage traits, and `ConstNonZeroLimits`, with
    `MIN` and `MAX`, for a storage whose integer holds the value inline. `IntegerNonZero` and the
    alias `NonZeroOf<T>` name the non-zero integer of an integer, as in `NonZeroOf<u7>`.
  - The module `aliases` names the non-zero integer of each integer alias but `u0` and `i0`, which
    hold no value that is not zero, in the same storage: `NonZeroU1` to `NonZeroU1024` and
    `NonZeroI1` to `NonZeroI1024`, but for the native widths 8, 16, 32, 64 and 128, whose names
    `core::num` has, and `NonZeroU2048`, `NonZeroU4096`, `NonZeroI2048` and `NonZeroI4096`, so that
    `NonZeroU7` is `NonZeroOf<u7>`.
- The arithmetic methods of `Decimal`, `Uint` and `Int`, inherent and in the traits, take the
  receiver and the operands by reference, as in `a.checked_add(&b)`, so that a call does not copy a
  wide value, while the operators keep the signatures of their standard traits. The casts, between
  any widths and signs, and the bitwise `const fn`s take their operands by value, as the native
  casts and the methods of the operator traits do; a shift amount or a bit index is a `u32`.
- `Storage`, `DigitCount`, `FitsDigits`, `Layout`, `Widen`, `WidenRepr` and `Pick`: the type-level
  capacity and sign checks, with compile-time diagnostics for invalid digit counts and storages.
