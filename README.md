<div align="center">
<h1>numo</h1>
</div>
<div align="center">

Exact fixed-point decimal numbers of up to 153 digits, or 154 with an explicit `u512`, whose digit counts and storage are part of the type, for `no_std` without allocation.

[<img alt="github" src="https://img.shields.io/badge/github-al8n/numo-8da0cb?style=for-the-badge&logo=Github" height="22">][Github-url]
[<img alt="Build" src="https://img.shields.io/github/actions/workflow/status/al8n/numo/ci.yml?logo=Github-Actions&style=for-the-badge" height="22">][CI-url]
[<img alt="codecov" src="https://img.shields.io/codecov/c/gh/al8n/numo?style=for-the-badge&logo=codecov" height="22">][codecov-url]

[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-numo-66c2a5?style=for-the-badge&labelColor=555555&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20">][doc-url]
[<img alt="crates.io" src="https://img.shields.io/crates/v/numo?style=for-the-badge&logo=data:image/svg+xml;base64,PD94bWwgdmVyc2lvbj0iMS4wIiBlbmNvZGluZz0iaXNvLTg4NTktMSI/Pg0KPCEtLSBHZW5lcmF0b3I6IEFkb2JlIElsbHVzdHJhdG9yIDE5LjAuMCwgU1ZHIEV4cG9ydCBQbHVnLUluIC4gU1ZHIFZlcnNpb246IDYuMDAgQnVpbGQgMCkgIC0tPg0KPHN2ZyB2ZXJzaW9uPSIxLjEiIGlkPSJMYXllcl8xIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHhtbG5zOnhsaW5rPSJodHRwOi8vd3d3LnczLm9yZy8xOTk5L3hsaW5rIiB4PSIwcHgiIHk9IjBweCINCgkgdmlld0JveD0iMCAwIDUxMiA1MTIiIHhtbDpzcGFjZT0icHJlc2VydmUiPg0KPGc+DQoJPGc+DQoJCTxwYXRoIGQ9Ik0yNTYsMEwzMS41MjgsMTEyLjIzNnYyODcuNTI4TDI1Niw1MTJsMjI0LjQ3Mi0xMTIuMjM2VjExMi4yMzZMMjU2LDB6IE0yMzQuMjc3LDQ1Mi41NjRMNzQuOTc0LDM3Mi45MTNWMTYwLjgxDQoJCQlsMTU5LjMwMyw3OS42NTFWNDUyLjU2NHogTTEwMS44MjYsMTI1LjY2MkwyNTYsNDguNTc2bDE1NC4xNzQsNzcuMDg3TDI1NiwyMDIuNzQ5TDEwMS44MjYsMTI1LjY2MnogTTQzNy4wMjYsMzcyLjkxMw0KCQkJbC0xNTkuMzAzLDc5LjY1MVYyNDAuNDYxbDE1OS4zMDMtNzkuNjUxVjM3Mi45MTN6IiBmaWxsPSIjRkZGIi8+DQoJPC9nPg0KPC9nPg0KPGc+DQo8L2c+DQo8Zz4NCjwvZz4NCjxnPg0KPC9nPg0KPGc+DQo8L2c+DQo8Zz4NCjwvZz4NCjxnPg0KPC9nPg0KPGc+DQo8L2c+DQo8Zz4NCjwvZz4NCjxnPg0KPC9nPg0KPGc+DQo8L2c+DQo8Zz4NCjwvZz4NCjxnPg0KPC9nPg0KPGc+DQo8L2c+DQo8Zz4NCjwvZz4NCjxnPg0KPC9nPg0KPC9zdmc+DQo=" height="22">][crates-url]
[<img alt="crates.io" src="https://img.shields.io/crates/d/numo?color=critical&logo=data:image/svg+xml;base64,PD94bWwgdmVyc2lvbj0iMS4wIiBzdGFuZGFsb25lPSJubyI/PjwhRE9DVFlQRSBzdmcgUFVCTElDICItLy9XM0MvL0RURCBTVkcgMS4xLy9FTiIgImh0dHA6Ly93d3cudzMub3JnL0dyYXBoaWNzL1NWRy8xLjEvRFREL3N2ZzExLmR0ZCI+PHN2ZyB0PSIxNjQ1MTE3MzMyOTU5IiBjbGFzcz0iaWNvbiIgdmlld0JveD0iMCAwIDEwMjQgMTAyNCIgdmVyc2lvbj0iMS4xIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHAtaWQ9IjM0MjEiIGRhdGEtc3BtLWFuY2hvci1pZD0iYTMxM3guNzc4MTA2OS4wLmkzIiB3aWR0aD0iNDgiIGhlaWdodD0iNDgiIHhtbG5zOnhsaW5rPSJodHRwOi8vd3d3LnczLm9yZy8xOTk5L3hsaW5rIj48ZGVmcz48c3R5bGUgdHlwZT0idGV4dC9jc3MiPjwvc3R5bGU+PC9kZWZzPjxwYXRoIGQ9Ik00NjkuMzEyIDU3MC4yNHYtMjU2aDg1LjM3NnYyNTZoMTI4TDUxMiA3NTYuMjg4IDM0MS4zMTIgNTcwLjI0aDEyOHpNMTAyNCA2NDAuMTI4QzEwMjQgNzgyLjkxMiA5MTkuODcyIDg5NiA3ODcuNjQ4IDg5NmgtNTEyQzEyMy45MDQgODk2IDAgNzYxLjYgMCA1OTcuNTA0IDAgNDUxLjk2OCA5NC42NTYgMzMxLjUyIDIyNi40MzIgMzAyLjk3NiAyODQuMTYgMTk1LjQ1NiAzOTEuODA4IDEyOCA1MTIgMTI4YzE1Mi4zMiAwIDI4Mi4xMTIgMTA4LjQxNiAzMjMuMzkyIDI2MS4xMkM5NDEuODg4IDQxMy40NCAxMDI0IDUxOS4wNCAxMDI0IDY0MC4xOTJ6IG0tMjU5LjItMjA1LjMxMmMtMjQuNDQ4LTEyOS4wMjQtMTI4Ljg5Ni0yMjIuNzItMjUyLjgtMjIyLjcyLTk3LjI4IDAtMTgzLjA0IDU3LjM0NC0yMjQuNjQgMTQ3LjQ1NmwtOS4yOCAyMC4yMjQtMjAuOTI4IDIuOTQ0Yy0xMDMuMzYgMTQuNC0xNzguMzY4IDEwNC4zMi0xNzguMzY4IDIxNC43MiAwIDExNy45NTIgODguODMyIDIxNC40IDE5Ni45MjggMjE0LjRoNTEyYzg4LjMyIDAgMTU3LjUwNC03NS4xMzYgMTU3LjUwNC0xNzEuNzEyIDAtODguMDY0LTY1LjkyLTE2NC45MjgtMTQ0Ljk2LTE3MS43NzZsLTI5LjUwNC0yLjU2LTUuODg4LTMwLjk3NnoiIGZpbGw9IiNmZmZmZmYiIHAtaWQ9IjM0MjIiIGRhdGEtc3BtLWFuY2hvci1pZD0iYTMxM3guNzc4MTA2OS4wLmkwIiBjbGFzcz0iIj48L3BhdGg+PC9zdmc+&style=for-the-badge" height="22">][crates-url]
<img alt="license" src="https://img.shields.io/badge/License-Apache%202.0/MIT-blue.svg?style=for-the-badge&fontColor=white&logoColor=f5c076&logo=data:image/svg+xml;base64,PCFET0NUWVBFIHN2ZyBQVUJMSUMgIi0vL1czQy8vRFREIFNWRyAxLjEvL0VOIiAiaHR0cDovL3d3dy53My5vcmcvR3JhcGhpY3MvU1ZHLzEuMS9EVEQvc3ZnMTEuZHRkIj4KDTwhLS0gVXBsb2FkZWQgdG86IFNWRyBSZXBvLCB3d3cuc3ZncmVwby5jb20sIFRyYW5zZm9ybWVkIGJ5OiBTVkcgUmVwbyBNaXhlciBUb29scyAtLT4KPHN2ZyBmaWxsPSIjZmZmZmZmIiBoZWlnaHQ9IjgwMHB4IiB3aWR0aD0iODAwcHgiIHZlcnNpb249IjEuMSIgaWQ9IkNhcGFfMSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIiB4bWxuczp4bGluaz0iaHR0cDovL3d3dy53My5vcmcvMTk5OS94bGluayIgdmlld0JveD0iMCAwIDI3Ni43MTUgMjc2LjcxNSIgeG1sOnNwYWNlPSJwcmVzZXJ2ZSIgc3Ryb2tlPSIjZmZmZmZmIj4KDTxnIGlkPSJTVkdSZXBvX2JnQ2FycmllciIgc3Ryb2tlLXdpZHRoPSIwIi8+Cg08ZyBpZD0iU1ZHUmVwb190cmFjZXJDYXJyaWVyIiBzdHJva2UtbGluZWNhcD0icm91bmQiIHN0cm9rZS1saW5lam9pbj0icm91bmQiLz4KDTxnIGlkPSJTVkdSZXBvX2ljb25DYXJyaWVyIj4gPGc+IDxwYXRoIGQ9Ik0xMzguMzU3LDBDNjIuMDY2LDAsMCw2Mi4wNjYsMCwxMzguMzU3czYyLjA2NiwxMzguMzU3LDEzOC4zNTcsMTM4LjM1N3MxMzguMzU3LTYyLjA2NiwxMzguMzU3LTEzOC4zNTcgUzIxNC42NDgsMCwxMzguMzU3LDB6IE0xMzguMzU3LDI1OC43MTVDNzEuOTkyLDI1OC43MTUsMTgsMjA0LjcyMywxOCwxMzguMzU3UzcxLjk5MiwxOCwxMzguMzU3LDE4IHMxMjAuMzU3LDUzLjk5MiwxMjAuMzU3LDEyMC4zNTdTMjA0LjcyMywyNTguNzE1LDEzOC4zNTcsMjU4LjcxNXoiLz4gPHBhdGggZD0iTTE5NC43OTgsMTYwLjkwM2MtNC4xODgtMi42NzctOS43NTMtMS40NTQtMTIuNDMyLDIuNzMyYy04LjY5NCwxMy41OTMtMjMuNTAzLDIxLjcwOC0zOS42MTQsMjEuNzA4IGMtMjUuOTA4LDAtNDYuOTg1LTIxLjA3OC00Ni45ODUtNDYuOTg2czIxLjA3Ny00Ni45ODYsNDYuOTg1LTQ2Ljk4NmMxNS42MzMsMCwzMC4yLDcuNzQ3LDM4Ljk2OCwyMC43MjMgYzIuNzgyLDQuMTE3LDguMzc1LDUuMjAxLDEyLjQ5NiwyLjQxOGM0LjExOC0yLjc4Miw1LjIwMS04LjM3NywyLjQxOC0xMi40OTZjLTEyLjExOC0xNy45MzctMzIuMjYyLTI4LjY0NS01My44ODItMjguNjQ1IGMtMzUuODMzLDAtNjQuOTg1LDI5LjE1Mi02NC45ODUsNjQuOTg2czI5LjE1Miw2NC45ODYsNjQuOTg1LDY0Ljk4NmMyMi4yODEsMCw0Mi43NTktMTEuMjE4LDU0Ljc3OC0zMC4wMDkgQzIwMC4yMDgsMTY5LjE0NywxOTguOTg1LDE2My41ODIsMTk0Ljc5OCwxNjAuOTAzeiIvPiA8L2c+IDwvZz4KDTwvc3ZnPg==" height="22">

</div>

## Overview

`numo` provides number types whose sizes are type-level numbers from
[`typenum`](https://crates.io/crates/typenum) and whose storage is an integer named in the type: a
native integer, or a 192-, 256-, 320-, 384-, 448- or 512-bit integer of this crate. Capacity is
checked during type checking, so no value can be created for a type that cannot hold its digits.
The arithmetic methods take the receiver and the operands by reference, as in `a.checked_add(&b)`,
so that a call does not copy a wide value, while the operators keep the signatures of their
standard traits. The casts, such as `checked_cast` and `cast_signed`, and the bitwise `const fn`s,
such as `bitand`, take their operands by value, as the native casts and the methods of the operator
traits do.

| Family | Status |
|---|---|
| `Decimal<Integral, Fraction, Repr>`: exact fixed-point decimal numbers | Available |
| `Uint<Bits, Repr>` and `Int<Bits, Repr>`: integers of any bit width | Any width in a native integer of up to 128 bits or in limbs of 8, 16, 32, 64 or 128 bits, and with the `std` or the `alloc` feature in a `Box`, updated in place, or shared in an `Rc`, an `Arc` or a `triomphe::Arc`, with a default storage, the aliases `u0`–`u1024` and `i0`–`i1024`, constants, checked construction, also from slices of limbs, comparison, also with the native integers both ways, hashing, the text of the native integers, parsed in every radix and written in decimal, binary, octal, hexadecimal and scientific notation, the bytes in either order, in arrays whose length is part of the type or in slices and buffers, with the byte order of whole bytes, the arithmetic of the native integers in every overflow form with the operators, division, powers, roots and logarithms included, the bit operations and shifts with their operators, the casts between any widths and signs and with the native integers, with `From` and `TryFrom` by direction, the conversions with `f64` and `f32`, truncating, saturating, rounded by a mode or exact, with `From` where lossless, serde, as numbers up to 128 bits and decimal strings above, with helpers for decimal strings, Ethereum quantities, fixed-width hexadecimal, reading by type, bytes in either order and ruint's form, and generic traits |
| `NonZeroUint<Bits, Repr>` and `NonZeroInt<Bits, Repr>`: non-zero integers of any bit width | Any width in the native `NonZero` of a native integer, whose niche makes `Option` free, or in limbs, or over a heap storage, whose pointer keeps `Option` free, with the aliases `NonZeroU1`–`NonZeroU1024` and `NonZeroI1`–`NonZeroI1024`, the methods of the native `NonZero` integers, the byte order of whole bytes, the division of `Uint` by `NonZeroUint`, which cannot panic, `TryFrom` of the integer, the conversions with the native `NonZero` integers by direction, the casts into any integer and, checking for zero, from any integer, serde as their integers, and generic traits |
| Floating-point types for standard formats such as `f16`, `bf16` and `f128` | Planned |
| Atomic versions of the integer types | Planned |

## Installation

```sh
cargo add numo
```

## Features

| Feature | Description |
|---|---|
| `std` (default) | Links the standard library, and turns on the default features of `thiserror` and `std` of `triomphe` when that feature is on; gives the heap storages and the forms that return a `Vec` or a `String` |
| `alloc` | Gives the heap storages and the forms that return a `Vec` or a `String` to a `no_std` build, through the `alloc` crate |
| `serde` | `Serialize` and `Deserialize`, through `serde_core`; turns on no feature of another crate |
| `arbitrary` | `Arbitrary` for fuzzing; the `arbitrary` crate uses `std` |
| `proptest` | `Arbitrary` and strategies for property tests, with the `std` feature of `proptest` |
| `quickcheck` | `Arbitrary` for property tests; the `quickcheck` crate uses `std` |
| `triomphe` | `triomphe::Arc<[L; N]>` as a read-only heap storage, on a target with atomic pointers; on another target it turns nothing on |

With no feature the crate is `no_std` and does not allocate. `std` and `alloc` are independent:
either one gives the heap storages `Box<[L; N]>`, `Rc<[L; N]>` and `Arc<[L; N]>` and the forms that
return a `Vec` or a `String`, and neither turns the other on. No feature of an optional dependency
turns on `std` or `alloc`.

## Example

```rust
use numo::{
  Decimal, ParseError,
  typenum::{U8, U10, U12},
};

type Price = Decimal<U10, U8>; // 18 digits, stored in an `i64`
type Quantity = Decimal<U12, U8>; // 20 digits, stored in an `i128`

fn main() -> Result<(), ParseError> {
  let price: Price = "64250.5".parse()?;
  let quantity: Quantity = "0.0015".parse()?;

  // The product type, `Decimal<U22, U16>`, is computed by the compiler; nothing is rounded.
  let notional = price.exact_mul(&quantity);
  assert_eq!(notional.to_string(), "96.37575");

  // Sums are checked against the decimal range, not only the storage range.
  assert_eq!(Price::MAX.checked_add(&price), None);

  Ok(())
}
```

## `Decimal`

- The value is `c / 10^Fraction`, where the coefficient `c` is stored in `Repr` and
  `|c| <= 10^(Integral + Fraction) - 1`, like SQL `NUMERIC(Integral + Fraction, Fraction)`. There
  is no NaN, infinity or negative zero.
- `Repr` is a signed or unsigned integer. It defaults to the smallest signed integer that holds
  the digits: `i8` up to 2 digits, `i16` up to 4, `i32` up to 9, `i64` up to 18, `i128` up to 38,
  and the wide integers `i192` up to 57, `i256` up to 76, `i320` up to 96, `i384` up to 115, `i448`
  up to 134 and `i512` up to 153. A wider `Repr` can be named explicitly, and so can an unsigned
  one: `u8` up to 2 digits, `u16` up to 4, `u32` up to 9, `u64` up to 19, `u128` up to 38, `u192`
  up to 57, `u256` up to 77, `u320` up to 96, `u384` up to 115, `u448` up to 134 and `u512` up to
  154. The type has the size, alignment and ABI of `Repr`.
- With a signed `Repr`, the range is symmetric and `Neg` is implemented. With an unsigned `Repr`,
  values run from zero to `MAX`, there is no negation, `checked_sub` returns `None` below zero,
  and parsing rejects a negative number as `Negative`, even `-0`.
- Zero digits, more than 154 digits, or a `Repr` that is too small are rejected during type
  checking. With the default `Repr`, zero or more than 153 digits fail wherever the type is
  written; with an explicit `Repr`, the type's constants and constructors fail, so no value of it
  can be created. The type itself has no bounds, so generic code can store and pass values
  without any.
- Every operation is exact or fails, unless it is asked to round or to saturate. `checked_add` and
  `checked_sub` return `None` outside the range, `exact_mul` returns a wider type that always holds
  the product, up to 153 digits, `widen` converts to a type with at least as many digits, and
  `try_convert` reports `OutOfRange` or `Inexact` instead of rounding. Rounding happens only where
  it is asked for: `trunc`, `floor` and `ceil` round to an integer in their own direction, `to_f64`
  and `to_f32` round to the nearest float, and every other rounding takes an explicit
  `RoundingMode`: `round_to` converts to fewer fraction digits, `quantize` rounds to a multiple of a
  step, `div_round` divides into any decimal type, and `round_significant` rounds to a number of
  significant digits.
  The modes are `Up`, `Down`, `Ceiling`, `Floor`, `HalfUp`, `HalfDown`, `HalfEven` and `Up05`.
- The operators `+`, `-`, `+=` and `-=` take two values of one type, `+` and `-` also two
  references, and `Sum` adds the values or the references of an iterator. They are exact too: a
  result outside the range panics in every build, whatever `overflow-checks` is, with the message
  of the native operators, where `checked_add` and `checked_sub` return `None` and
  `saturating_add` and `saturating_sub` the limit that it passes. `checked_rem` is the exact
  remainder, with the sign of the dividend. There are no `*`, `/` or `%` operators: `exact_mul`
  and `div_round` are methods.
- The parts and the digits of a value: `trunc`, `fract`, `floor` and `ceil`, in the type of the
  value, where `floor` and `ceil` report a result out of range, and `is_integer`,
  `order_of_magnitude`, `significant_digits` and `fraction_digits`.
- Conversions follow the sign of the storage. The product of two unsigned storages is unsigned,
  and any other product is signed. `widen` converts an unsigned storage to any storage, but a
  signed storage only to a signed one. `try_convert` reports a negative value converted to an
  unsigned storage as `OutOfRange`.
- The conversions with `f64` and `f32` are for interchange and display. `to_f64` and `to_f32` return
  the nearest float, rounded once. `round_from_f64` reads the exact binary value of a float and
  `from_f64_shortest` its shortest text, so that `0.1` reads as `0.1`; each rounds once with a
  `RoundingMode`, and each has an `f32` form. `DecimalFloat` has the six for generic code.
- Methods that depend only on the storage are `const fn`, so constants can be parsed and computed
  at compile time.
- The constants `INTEGRAL`, `FRACTION` and `DIGITS` give the digit counts of a type, and `SIGNED`
  whether its storage is signed. Sealed traits give generic code the items of a type in two
  layers, each implemented for every storage. The general layer requires `Clone` but not `Copy`:
  `FixedDecimal`, with the digit counts, the queries, the arithmetic, the operators `+` and `-` on
  values, `Sum` and `zero()`, `max_value()` and `min_value()`, `SignedFixedDecimal` and
  `UnsignedFixedDecimal` for each sign, and `DecimalOne`, with `one()`; these functions return the
  constants `ZERO`, `MAX`, `MIN` and `ONE`. The const layer, `ConstFixedDecimal` and
  `ConstDecimalOne`, adds `Copy`, the assignments `+=` and `-=` of `DecimalAssign` and the constants
  themselves, which a generic `const fn` can read. `ExactMul`, `WidenTo`, `ConvertTo` and `DivRound`
  have the methods between decimal types, and `DecimalFloat` the conversions with the floats. Code
  generic over `Decimal<I, F, R>` adds the bound `Decimal<I, F, R>: FixedDecimal`.
- `parse` accepts ASCII text with an optional sign, point and exponent, and `_` separators as in
  Rust literals: the numeric strings of the General Decimal Arithmetic specification and every
  form that `f64` parses, such as `-1.5`, `+.5`, `5.`, `2.5E-3` and `1_000`. Infinity and NaN are
  recognized and reported as `NotFinite`, because a `Decimal` is always finite. A value that does
  not fit is `OutOfRange` or `Inexact`, never rounded.
- `dec!` writes a decimal literal in the source, which `parse` reads when the crate is compiled,
  of the type that its context expects or that `as` names: `dec!(0.01)`,
  `dec!(-2.5e-3 as Price)`. A literal that the type does not hold fails to compile: in a `const`
  or a `static` item and with `as` when the code is checked, and elsewhere when the code is
  generated.
- `Display` writes the shortest exact text, without an exponent, and `{:e}` and `{:E}` write it in
  scientific notation, such as `6.42505e4`, or with `#` in engineering notation, such as
  `64.2505e3`. Format precision pads with zeros but never rounds.

```rust
use numo::{
  Decimal, Scaled, dec,
  typenum::{U8, U10},
};

type Price = Decimal<U10, U8>;

fn main() {
  const TICK: Price = dec!(0.01);
  let mut price: Price = dec!(64_250.5);
  price += dec!(0.25);
  let ask = price + dec!(1e-2);
  assert!(ask == dec!(64_250.76));
  assert_eq!(dec!(0.5 as Price).checked_add(&TICK), Some(dec!(0.51 as Price)));
  let quoted: Scaled<Price> = dec!(1.50);
  assert_eq!(quoted.to_string(), "1.50");
}
```

## `Scaled`

A `Decimal` writes the shortest exact text, so `0.00100000` reads as `0.001` and writes back as
`0.001`. `Scaled<D>` keeps the scale of the text beside the decimal, the number of fraction digits
that the text showed, for code that must write a text back as it came. It parses with the grammar
of `Decimal::parse`, in a `const fn` of each storage and through `FromStr`, writes at least as many
fraction digits as its scale, and compares, orders and hashes by value only. Every decimal trait is
implemented for it over any decimal, so generic code takes a `Scaled` and a bare decimal alike: a
sum takes the larger scale, an exact product the sum of the scales, a value of `round_to` or a
quotient the fraction digits of its target type, and a value read from a float those of the type or,
by its shortest text, those of the text. The operators `+`, `-`, `+=` and `-=` and `Sum` work as for
a decimal, and their result takes the larger scale. A scale above 65535 is `ScaleTooLarge`, which
only the `parse` of `Scaled` reports. A literal of `dec!` keeps its scale too: `dec!(1.50)` writes
`1.50`.

```rust
use numo::{
  Decimal, FixedDecimal, ParseError, Scaled,
  typenum::{U8, U10},
};

type Price = Scaled<Decimal<U10, U8>>;

fn main() -> Result<(), ParseError> {
  let price: Price = "64250.50000".parse()?;
  assert_eq!(price.to_string(), "64250.50000");
  assert_eq!(price.scale(), 5);

  // The scale is presentation only, and a sum takes the larger one.
  assert_eq!(price, "64250.5".parse()?);
  let tick: Price = "0.10".parse()?;
  assert_eq!(price.checked_add(&tick).unwrap().to_string(), "64250.60000");

  Ok(())
}
```

## Wide storage

Decimals with more than 38 digits are stored in `i192`, `i256`, `i320`, `i384`, `i448` and `i512`,
or `u192`, `u256`, `u320`, `u384`, `u448` and `u512`: aliases of `Int` and `Uint` with three to
eight 64-bit limbs. Any uint256 amount, such as a token balance, has at most 78 digits and fits a
`u320`. The 154th digit fits only a `u512`, which must be named explicitly, as in
`Decimal<U154, U0, u512>`: no signed storage holds 154 digits, so the default storage and the
product of `exact_mul` stop at 153. A wide coefficient is built from its limbs, least significant
first, and `from_repr` checks its range; decimal text with an exponent parses to the same value.

```rust
use numo::{
  Decimal, ParseError,
  typenum::{U18, U60},
  u320,
};

// Any uint256 amount of a token with 18 decimals.
type TokenAmount = Decimal<U60, U18, u320>;

fn main() -> Result<(), ParseError> {
  // The raw amount 1.5 * 10^18 as the four limbs of a uint256.
  let raw: [u64; 4] = [1_500_000_000_000_000_000, 0, 0, 0];
  let [l0, l1, l2, l3] = raw;
  let amount = TokenAmount::from_repr(u320::from_limbs([l0, l1, l2, l3, 0])).unwrap();
  assert_eq!(amount.to_string(), "1.5");
  assert_eq!("1500000000000000000e-18".parse::<TokenAmount>()?, amount);

  Ok(())
}
```

## Integers

`Uint<Bits, Repr>` and `Int<Bits, Repr>` hold any width `Bits` in a storage `Repr` of at least
`Bits` bits: a native integer, `u8` to `u128` for `Uint` and `i8` to `i128` for `Int`, which holds
the value itself, or an array of limbs, least significant limb first, in two's complement for
`Int`: `[u64; N]`, `[u32; N]`, `[u16; N]`, `[u8; N]` or `[u128; N]`. Zero bits hold only zero, in
every storage. Without `Repr`, the storage is the zero-sized `[u64; 0]` for zero bits, the smallest
native integer that holds the width from 1 to 128 bits, and from 129 to 65536 bits
`[u64; ceil(Bits / 64)]`, so `Uint<U7>` is `Uint<U7, u8>` and `Uint<U200>` is
`Uint<U200, [u64; 4]>`. `[u32; N]` computes with native 64-bit intermediates, and `[u8; N]` has
alignment 1 and holds the little-endian bytes of the value on every target. `[u128; N]` has the
layout of an array of `u128`, for memory shared with code that stores 128-bit words. Its arithmetic
works on the 64-bit halves of each limb, as no wider native integer exists; its speed relative to
`[u64; N]` depends on the operation, the width and the target (formatting a value of a few limbs is
several times slower), so choose it for layout, not for speed. The aliases name `u0` to `u1024` and
`i0` to `i1024`, except the native widths 8, 16, 32, 64 and 128, in the default storage: `[u64; 0]`
for zero bits, so that `u0` and `i0` take no space, the smallest native integer for 1 to 127 bits,
such as `u7` for `Uint<U7, u8>`, and `[u64; ceil(N / 64)]` for 129 to 1024 bits, such as `u144` for
`Uint<U144, [u64; 3]>`. They also name `u2048`, `u4096`, `i2048` and `i4096`. The non-zero integers
have the aliases `NonZeroUN` and `NonZeroIN` for the same widths, such as `NonZeroU7` for
`NonZeroUint<U7, NonZero<u8>>`, except zero bits, which hold no value that is not zero, and the
native widths, whose names `core::num` has. Any other width or storage is written out, such as
`Uint<U8192>` or `Uint<U24, [u8; 3]>`, and a width that `typenum` has no constant for is computed,
as in `Uint<Sum<U1024, U476>>` for 1500 bits.

With the `std` or the `alloc` feature, `Uint`, `Int`, `NonZeroUint` and `NonZeroInt` also take an
array of limbs in a `Box<[L; N]>`, for limbs `L` from `u8` to `u128`, at every width that the array
holds, such as `Uint<U4096, Box<[u64; 64]>>`. Such an integer has the size of a pointer, is not
`Copy`, and has no constants and no `const fn`s: its API is the traits, with `FixedInteger::zero`,
`min_value` and `max_value` and `IntegerOne::one` for the constants. An operator on two references
allocates its result, a compound assignment of `Uint` or `Int` computes in place, and an operator
with an owned operand stores its result in the allocation of that operand, the left one of two. A
method builds only the integers that it returns, none when it returns `None` or an error, and one
that takes its integer by value, as the casts between the signs do, stores its result in that
allocation, while a cast, such as `checked_cast`, builds its target from the words of the source and
frees a source given by value. Each operation computes in arrays on the stack before it stores, so a
heap storage keeps the value off the stack and not the working space. `From` moves a `Box<[L; N]>`
out of an integer, and into one whose width fills the array, without a copy, and `ArrayInteger`
reads and builds the limbs. serde writes and reads a boxed integer as the inline integer of its
width.

```rust
use numo::{FixedInteger, Uint, typenum::U4096};

type U4096x64 = Uint<U4096, Box<[u64; 64]>>;

let (a, b) = (U4096x64::from(3u64), U4096x64::from(5u64));
let mut total = U4096x64::zero();
total += &a; // in place
let product = &a * &b; // a new allocation
let sum = a.clone() + &b; // in the allocation of the clone
assert_eq!(total.to_string(), "3");
assert_eq!(product.to_string(), "15");
assert_eq!(sum.to_string(), "8");
```

- `const` items and methods: `BITS`, `ZERO`, `MIN` and `MAX`, and `ONE` from one bit for `Uint`
  and from two bits for `Int`; for a native integer, `new`, which panics if the value does not fit,
  `try_new`, which returns an error instead, and `value`; for an array, `from_limbs` for a width
  that fills its limbs, `checked_from_limbs` for any width, which returns `None` unless the limbs
  hold a value of `Bits` bits, `checked_from_limbs_slice`, `wrapping_from_limbs_slice`,
  `overflowing_from_limbs_slice`, `saturating_from_limbs_slice` and `strict_from_limbs_slice`,
  which read a slice of limbs of any length as the slices of bytes are read, and `to_limbs`,
  `as_limbs` and `into_limbs`; `is_zero` and `is_one`, plus `is_negative` and `is_positive` for
  `Int`.
- The arithmetic of the native integers at the width `Bits`, as `const fn`s: addition, subtraction,
  multiplication, negation, division, remainder and exponentiation in the `checked_*`,
  `wrapping_*`, `overflowing_*`, `saturating_*` and `strict_*` forms that the native integers have,
  the Euclidean, rounded and exact division, the next multiple, the square root and the root of
  any degree, the logarithms, the midpoint and the absolute difference, `carrying_mul` and
  `carrying_mul_add`, the low and the high half of `self * rhs + carry + add`, whose low half for
  `Int` is the `Uint` of the same width, `carrying_add`, `borrowing_sub`, `is_multiple_of` and the
  next power of two for `Uint`, and `abs` and its forms, `unsigned_abs`, `signum`, `checked_isqrt`,
  and `carrying_add` and `borrowing_sub`, whose flag is an overflow of the range, for `Int`.
  Addition, subtraction, multiplication, exact division and the shifts, and negation for `Int`,
  also have the `unchecked_*` form, as `const unsafe fn`s whose caller guarantees that the
  `checked_*` method would return `Some`. `widening_mul`, for every storage, returns the exact
  product of two integers of any widths and storages whose widths sum to at most 65536 bits, in
  the integer of the sum of the widths, in its default storage: the product of two `u256` is a
  `u512`. The operators `+`, `-` and `*`, their compound assignments, `-` for `Int`, `Sum` and
  `Product` panic or wrap on overflow as `overflow-checks` decides, as for the native integers, and
  so do `pow`, `next_multiple_of` and `next_power_of_two`; `/` and `%` and their compound
  assignments, like the division methods, panic on a zero divisor and on overflow in every build.
  Every panic reports the caller, as the operators of the native integers do, but that of `pow`,
  which reports a line in `core` as the native `pow` does.
- The bit operations of the native integers at the width `Bits`, as `const fn`s: the counts of
  ones and zeros, leading and trailing, the highest and the lowest one and their isolation, the
  bit reversal, the rotations, the shifts in every overflow form, the unbounded shifts and the exact
  shifts of the unstable native API, `bit` and `set_bit`, `bit_width` and `is_power_of_two` for
  `Uint`, and the casts `cast_signed` and `cast_unsigned` between `Uint` and `Int` of the same width
  and storage size. They count and move the `Bits` bits alone, whatever the storage, so
  `u7::MAX.leading_ones()` is 7 and `reverse_bits` works at any width. The operators `!`, `&`,
  `|`, `^`, `<<` and `>>`, with an amount of any primitive integer type, and their compound
  assignments work for every storage; a shift amount that is `Bits` or more, or negative, overflows
  as the native shifts do, and `bitand`, `bitor`, `bitxor` and `not` serve constant contexts.
- Numeric equality, ordering and hashing, where the hash depends only on the value and `Bits`,
  whatever the storage, and `Default`, which is zero. `PartialEq` and `PartialOrd` with each native
  integer, both ways, compare the values as integers, whatever the widths and the signs, as in
  `Int::<U7>::new(-1) < 0u8` and `u256::MAX > u128::MAX`. With them in a build, a comparison of a
  native integer with an expression whose type is inferred from it, such as `n == m.into()`,
  `n == s.parse().unwrap()` or `n == v.iter().sum()`, needs the type named, as in
  `n == u64::from(m)`.
- The text of the native integers at the width `Bits`: `from_str_radix`, a `const fn` for the
  radixes 2 to 36, and `FromStr` read an optional sign and digits, with the error of the native
  integers, `core::num::ParseIntError`, and reject a value outside the range of `Bits`, whatever
  the storage holds. `Display` and `Debug` write decimal, `{:b}`, `{:o}`, `{:x}` and `{:X}` write
  the digits of a `Uint` and the two's complement of the `Bits` bits of an `Int`, and `{:e}` and
  `{:E}` write scientific notation, rounding half to even at a precision, all with the flags of the
  native integers and without allocation. With the `std` or the `alloc` feature, `to_str_radix`
  returns the text that `from_str_radix` reads back, in the radixes 2 to 36, with `-` and the
  digits of the magnitude of a negative `Int`, where `{:x}` writes its bits: `-1` and `7f` for
  `i7` -1.
- The bytes of the native integers at the width `Bits`, the low `BYTES` bytes of the storage, in
  two's complement for `Int`: `to_le_bytes`, `to_be_bytes` and `to_ne_bytes` return an array whose
  length is part of its type, up to 8192 bits, and `checked_from_le_bytes` and the other orders read
  one back, checking the bits of the last byte above the width, while at a width of whole bytes
  `from_le_bytes` and the others read every array. `checked_from_le_slice`,
  `wrapping_from_le_slice`, `overflowing_from_le_slice`, `saturating_from_le_slice` and
  `strict_from_le_slice` read a slice of any length, `copy_le_bytes_to` writes the bytes to a
  buffer at any width, and `byte` reads one byte. At a width of whole bytes, `swap_bytes`,
  `to_be`, `to_le`, `from_be` and `from_le` convert the byte order as the native integers do, by
  value rather than by layout. With the `std` or the `alloc` feature, `to_le_bytes_vec` and
  `to_be_bytes_vec` return the bytes at any width, and `to_le_bytes_trimmed_vec` and
  `to_be_bytes_trimmed_vec` the shortest bytes that the slices read back: none for zero, and the
  shortest two's complement of an `Int`.
- The digits of a `Uint` in any base, through `UnsignedDigits`: `checked_from_radix_be` and
  `checked_from_radix_le`, `const fn`s, read digits in bytes in a radix from 2 to 256, and
  `checked_from_base_be` and `checked_from_base_le` read digits in `u64` from an iterator in any
  base from 2, each with `None` for a digit outside the base or a value outside the width;
  `to_base_le` writes the digits as an iterator without allocation, and with the `std` or the
  `alloc` feature `to_base_be`, `to_radix_be` and `to_radix_le` write them into a `Vec`, as in a
  base-58 text of a `u256`: `u256::checked_from_radix_be(&x.to_radix_be(58), 58) == Some(x)`.
- The casts of the native integers, still unstable, between any widths and signs: `checked_cast`,
  `wrapping_cast`, `saturating_cast`, `strict_cast` and the `unsafe` `unchecked_cast` into a
  `Uint`, an `Int` or a native integer, the checked, strict and unchecked ones also into a non-zero
  integer, `widen` into an integer of the same sign that is at least as wide and `truncate`, with
  its checked and saturating forms, into one that is at most as wide, the widths and the signs
  checked at compile time, and the checked, saturating and strict casts between `Uint` and `Int`
  of the same width. The standard conversions go by direction: `From` converts a native integer
  into a numo integer whose width holds every value of it, as in `u256::from(5u64)`, `TryFrom`
  converts a `usize` or an `isize` into any width and a numo integer into each native integer, with
  the error of the native conversions, and `bool` converts both ways. A literal without a suffix is
  an `i32`, so `u256::from(5)` fails to compile, while `i256::from(5)` compiles.
- The conversions with the floats: `to_f64` and `to_f32` round to the nearest float, ties to even,
  as `as` converts the native integers, and `From` converts a width whose every value the float
  holds, at most 53 bits of `Uint` and 54 of `Int` into `f64`, and 24 and 25 bits into `f32`. From
  a float, as `const fn`s, `checked_from_f64`, `saturating_from_f64` and `strict_from_f64` truncate
  toward zero as the native `to_int_checked`, `as` and `to_int_strict` do, `round_from_f64` rounds
  with a `RoundingMode`, so that `HalfEven` rounds to the nearest integer, and `exact_from_f64`
  takes only an integer of the range. Their error, `FromFloatError`, reports NaN, the infinities, a
  value above the range and one below it apart, and for `exact_from_f64` a fraction. Each has an
  `f32` form.
- With the `serde` feature, `Serialize` and `Deserialize` at every width and in every storage,
  without allocation in numo: a value of up to 128 bits is written as a number, through the method
  of the smallest native integer of its sign that holds the width, and a wider one as its decimal
  text, in every format, so the form depends only on the width, the sign and the value. In a
  human-readable format, a value of up to 64 or of more than 128 bits reads from any integer that
  the format gives and from a decimal string, and one of 65 to 128 bits from a 128-bit integer, as
  `u128` reads; in other formats each class reads through its own method. As for `u128`, a field of
  65 to 128 bits fails in an untagged enum or a flattened struct. No error that numo builds names
  the input. The non-zero integers are written and read as their integers, and zero does not read.
  The module `numo::serde` has helpers for `#[serde(with = "...")]`, each with the leaves
  `required` and `optional`, for numo's integers and the native integers: `decimal`, the decimal
  text at every width; `quantity`, Ethereum's JSON-RPC QUANTITY of the unsigned integers, read as
  go-ethereum's `hexutil` decodes a number; `fixed_hex`, two hexadecimal digits for each byte of
  the width; `typed`, the form of the defaults, read alone and by its type in every format;
  `be_bytes` and `le_bytes`, the bytes of the value in either order, up to 8192 bits; and `ruint`,
  the form of ruint's `Uint`, read as ruint 1.21 reads it.
- Traits with one implementation for every storage of a kind, for generic code, in two layers. The
  general layer requires `Clone` but not `Copy`: `FixedInteger`, with `zero()`, `min_value()`,
  `max_value()`, `is_zero`, `is_one`, the operators, `FromStr`, the formats and
  `from_str_radix()`, and `IntegerOne`, with `one()`; `zero()`, `min_value()`, `max_value()` and
  `one()` return the constants `ZERO`, `MIN`, `MAX` and `ONE`, and `IntegerAssign` adds the compound
  assignments. The const layer, `ConstFixedInteger` and `ConstIntegerOne`, adds `Copy` and the
  constants themselves, which a generic `const fn` can read. `UnsignedFixedInteger` and
  `SignedFixedInteger` have the methods of each sign, `NativeInteger`, `ArrayInteger`, with the
  slices of limbs, and `FromLimbs` the construction of each kind, the arithmetic traits
  `OverflowingOps`, `CheckedOps`, `WrappingOps`, `SaturatingOps`, `StrictOps` and `UncheckedOps` the
  arithmetic, `IntegerMath`, `UnsignedMath` and `SignedMath` the functions without an overflow
  prefix, such as `pow`, `isqrt` and `ilog10`, `IntegerBits`, `UnsignedBits` and `SignedBits` the
  bit operations, `IntegerBytes`, `ByteArray`, `FromByteArray` and `IntegerByteOrder` the bytes,
  `CheckedCastFrom`, `BoundedCastFrom`, `WidenFrom` and `TruncateFrom`, which the target implements,
  the casts, and `IntegerFloat` the conversions with the floats; with the bound `B: BitWidth`, such
  code uses `Uint<B>` and `Int<B>` for any width from 0 to 65536 bits.
- A width that its storage cannot hold, `2^32` bits or more, or a width written with leading zero
  bits, which no `typenum` constant or operator gives, is rejected during type checking, and so is a
  width of more than 65536 bits without an explicit storage.
- `NonZeroUint<Bits, Repr>` and `NonZeroInt<Bits, Repr>` hold the values of `Bits` bits that are
  not zero, in the native `NonZero` of a native storage, such as `core::num::NonZero<u8>`, whose
  niche gives `Option` the size of the integer, or in an array of limbs; without `Repr`, the
  storage is that of the default storage, and `NonZeroOf<T>` names the non-zero integer of an
  integer `T`, as in `NonZeroOf<u7>`. `new` and `get` convert from and to the integer,
  `new_unchecked` converts without the check of zero, `from_str_radix` and `FromStr` parse as the
  integer does, with the error of the native `NonZero` integers for zero, the formats and
  `to_str_radix` write what the integer writes, and the methods of the native `NonZero`
  integers at the width are `const fn`s: the bit counts, the highest and the lowest one, the
  rotations, the byte order of whole bytes, the checked and saturating multiplication, powers and
  addition, the logarithms, the square root, the midpoint, the division rounded up, the casts, and
  the negation and the absolute value of `NonZeroInt`. `Uint / NonZeroUint` and
  `Uint % NonZeroUint` cannot panic. `TryFrom` converts the integer of the same width and storage
  into a non-zero integer, with the error of the kind `Zero` for zero, as the native `NonZero`
  integers convert, and the conversions with the native `NonZero` integers go by the directions of
  the integers with the native integers, as in `NonZeroUint::<U16>::from(NonZero::<u8>::MAX)`. The
  casts take a non-zero integer as a source, into any integer, and as a target that takes no zero:
  of `checked_cast`, `strict_cast` and the `unsafe` `unchecked_cast`, and of `widen` from a
  non-zero integer, as the native `From` converts between the `NonZero` integers. The traits
  `FixedNonZero`, `UnsignedNonZero`, `SignedNonZero`, `NonZeroLimits`, `ConstNonZeroLimits` and
  `IntegerByteOrder` give them to generic code.

#### License

`numo` is dual-licensed under the MIT license or the Apache License (Version 2.0), at your option.

See [LICENSE-APACHE](LICENSE-APACHE), [LICENSE-MIT](LICENSE-MIT) for details.

Copyright (c) 2026 Al Liu.

[Github-url]: https://github.com/al8n/numo/
[CI-url]: https://github.com/al8n/numo/actions/workflows/ci.yml
[doc-url]: https://docs.rs/numo
[crates-url]: https://crates.io/crates/numo
[codecov-url]: https://app.codecov.io/gh/al8n/numo/
