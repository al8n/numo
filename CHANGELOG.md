# UNRELEASED

FEATURES

- `Decimal<Integral, Fraction, Repr>`: exact fixed-point decimal numbers with type-level digit
  counts, stored in a native integer from `i8` to `i128` or from `u8` to `u128` (up to 38 digits),
  or in a wide integer from `i192` to `i512` or from `u192` to `u512` (up to 153 digits, and 154
  with an explicit `u512`).
  - `const fn` constants and methods for each storage: `ZERO`, `ONE`, `MIN`, `MAX`, `parse`,
    `from_repr`, `repr`, `checked_add`, `checked_sub`, `is_zero` and `is_positive`, plus `neg`,
    `abs` and `is_negative` for signed storages.
  - `exact_mul`, `widen` and `try_convert` between decimal types.
  - Exact parsing through `FromStr`, and the shortest exact text through `Display` and `Debug`.
  - Parsing accepts the numeric strings of the General Decimal Arithmetic specification, every
    form that `f64` parses (signs, `.5`, `5.`, exponents) and `_` separators as in Rust literals,
    and reports infinity and NaN as the new `ParseErrorKind::NotFinite`. Exponents of any size are
    applied exactly.
  - Scientific notation through `LowerExp` and `UpperExp`, `{:e}` and `{:E}`: the significant
    digits with the point after the first, and the exponent, as in `6.42505e4`, with zero as `0e0`.
    The flags work as for `Display`, and the precision is the minimum number of mantissa fraction
    digits, so it never rounds, unlike the precision of the primitive types.
  - Unsigned storages from `u8` to `u128`, named explicitly (`u64` holds 19 digits, one more than
    `i64`): values are never negative, parsing rejects a leading `-`, and `checked_sub` returns
    `None` below zero. The product of two unsigned storages is unsigned, `widen` never converts a
    signed storage to an unsigned one, and `try_convert` reports a negative value as `OutOfRange`.
  - Wide storages: `i192`, `i256`, `i320`, `i384`, `i448` and `i512` hold 57, 76, 96, 115, 134 and
    153 digits, and `u192`, `u256`, `u320`, `u384`, `u448` and `u512` hold 57, 77, 96, 115, 134 and
    154, so any uint256 amount fits `Decimal<U60, U18, u320>`. They have the same constants,
    `const fn` methods, parsing and formatting as the native storages; products reach up to 153
    digits, and `widen` and `try_convert` convert between native and wide storages. The 154th digit
    is available only with an explicit `u512`, as in `Decimal<U154, U0, u512>`, because no signed
    storage holds it.
- `RoundingMode`, with the modes `Up`, `Down`, `Ceiling`, `Floor`, `HalfUp`, `HalfDown` and
  `HalfEven`. Rounding happens only where a mode is given, for native and wide storages alike:
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
  - `const fn` `is_multiple_of` checks a value against a step without rounding it.
- The `serde` feature: `Decimal` is written as its text and read from a string, never from a number,
  in every format, and `RoundingMode` as the name of its variant, without allocation. The impls use
  `serde_core`, so they need `serde` 1.0.220 or later in the build; the feature enables no feature
  of any other crate.
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
  - A default storage for 1 to 65536 bits, so that `Uint<U7>` is `Uint<U7, u8>` and `Uint<U200>`
    is `Uint<U200, [u64; 4]>`: the smallest native integer up to 128 bits and
    `[u64; ceil(Bits / 64)]` above. `BitCount` names the mapping, and generic code adds
    `B: BitCount`. Zero bits and more than 65536 bits have no default; such a type names its
    storage.
  - The module `aliases`, re-exported at the crate root: `u1` to `u1024` and `i1` to `i1024`, but
    for the native widths 8, 16, 32, 64 and 128, in the default storage: the smallest native
    integer that holds 1 to 127 bits, and `[u64; ceil(N / 64)]` for 129 to 1024 bits, such as
    `u144` for `Uint<U144, [u64; 3]>`; and `u2048`, `u4096`, `i2048` and `i4096`.
  - `ArraySize` maps a `typenum` number of limbs, from 1 to 1024, to an array. `LimbArray` and
    `Limbs<Bits, L = u64>` name the array of limbs `L` that holds `Bits` bits, such as
    `Limbs<U200>` for `[u64; 4]` or `Limbs<U24, u8>` for `[u8; 3]`.
  - `const` items and methods: `BITS`, `ZERO`, `MIN` and `MAX`; for a native integer, `new`, which
    panics if the value does not fit in `Bits` bits, so that such a value fails to compile in a
    constant, `try_new`, which returns the new `TryNewError` instead, and `value`; for an array,
    `from_limbs` for a width that fills its limbs, `checked_from_limbs` for any width, which
    returns `None` unless the limbs hold a value of `Bits` bits, and `to_limbs` and `as_limbs`;
    `is_zero`, plus `is_negative` and `is_positive` for `Int`.
  - Numeric `Eq`, `Ord` and `Hash`, where the hash depends only on the value and `Bits`: every
    storage writes the same 64-bit words, a native integer those of a one-limb array. Decimal
    `Display` and `Debug`, the latter also under the hexadecimal flags, and `Default`, which is
    zero. There is no arithmetic yet.
  - `FitsBits` and `FillsRepr`, which relate a width to the storages that hold it and to the
    storages that it fills, with compile-time diagnostics, the sealed storage traits `UintRepr` and
    `IntRepr`, and `InlineUintRepr` and `InlineIntRepr` for the storages that hold the value inline.
  - The constants `BITS`, `ZERO`, `MIN` and `MAX` are defined once for every storage, so a constant
    whose storage the expected type names resolves: `let x: Uint<U7> = Uint::MAX;`,
    `let y: u256 = Uint::MAX;` and `Uint::MAX` passed for a parameter of a concrete type compile,
    while `Uint::<U7, _>::MAX` with no expected type needs a type annotation (E0283). Each storage
    has constructors of its own, so a constructor that leaves the storage to inference is ambiguous
    (E0034): `Uint::new(5)`, `Uint::from_limbs(..)` and `Uint::checked_from_limbs(..)` with only an
    expected type. Naming the width for the default storage, as in `Uint::<U7>::new(5)` or
    `Uint::<U200>::MAX`, the full type, or an alias, as in `u7::new(5)`, resolves it.
  - The sealed traits `FixedInteger`, with the constants and `is_zero`, `UnsignedFixedInteger`,
    `SignedFixedInteger`, with `is_negative`, `is_positive` and the unsigned integer of the same
    width, `NativeInteger`, with `new`, `try_new` and `value`, `ArrayInteger`, with
    `checked_from_limbs`, `to_limbs` and `as_limbs`, and `FromLimbs`, with `from_limbs`. Each has
    one implementation for every storage of a kind and mirrors the inherent items for generic
    code; the bound `B: BitCount` gives `Uint<B>` and `Int<B>` `FixedInteger` and the sign traits.
    The traits are not `const`; constant contexts use the inherent `const fn`s.
- `Storage`, `DigitCount`, `FitsDigits`, `Layout`, `Widen`, `WidenRepr` and `Pick`: the type-level
  capacity and sign checks, with compile-time diagnostics for invalid digit counts and storages.
