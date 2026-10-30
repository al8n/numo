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

English | [简体中文][zh-cn-url]

</div>

## Overview

`numo` provides number types whose sizes are type-level numbers from
[`typenum`](https://crates.io/crates/typenum) and whose storage is an integer named in the type: a
native integer, or a 192-, 256-, 320-, 384-, 448- or 512-bit integer of this crate. Capacity is
checked during type checking, so no value can be created for a type that cannot hold its digits.
The arithmetic methods take the receiver and the operands by reference, as in `a.checked_add(&b)`,
so that a call does not copy a wide value, while the operators keep the signatures of their
standard traits.

| Family | Status |
|---|---|
| `Decimal<Integral, Fraction, Repr>`: exact fixed-point decimal numbers | Available |
| `Uint<Bits, Repr>` and `Int<Bits, Repr>`: integers of any bit width | Any width in a native integer of up to 128 bits or in limbs of 8, 16, 32, 64 or 128 bits, with a default storage, the aliases `u0`–`u1024` and `i0`–`i1024`, constants, checked construction, comparison, hashing, formatting, addition, subtraction, multiplication and negation in every overflow form of the native integers with the operators, and generic traits; division, bit operations and shifts planned |
| Floating-point types for standard formats such as `f16`, `bf16` and `f128` | Planned |
| Atomic versions of the integer types | Planned |

## Installation

```sh
cargo add numo
```

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
  and parsing rejects a leading `-`, even in `-0`.
- Zero digits, more than 154 digits, or a `Repr` that is too small are rejected during type
  checking. With the default `Repr`, zero or more than 153 digits fail wherever the type is
  written; with an explicit `Repr`, the type's constants and constructors fail, so no value of it
  can be created. The type itself has no bounds, so generic code can store and pass values
  without any.
- Every operation is exact or fails. `checked_add` and `checked_sub` return `None` outside the
  range, `exact_mul` returns a wider type that always holds the product, up to 153 digits, `widen`
  converts to a type with at least as many digits, and `try_convert` reports `OutOfRange` or
  `Inexact` instead of rounding. Rounding happens only where an explicit `RoundingMode` is given:
  `round_to` converts to fewer fraction digits, `quantize` rounds to a multiple of a step, and
  `div_round` divides into any decimal type. There are no `+`, `-`, `*` or `/` operators.
- Conversions follow the sign of the storage. The product of two unsigned storages is unsigned,
  and any other product is signed. `widen` converts an unsigned storage to any storage, but a
  signed storage only to a signed one. `try_convert` reports a negative value converted to an
  unsigned storage as `OutOfRange`.
- Methods that depend only on the storage are `const fn`, so constants can be parsed and computed
  at compile time.
- The constants `INTEGRAL`, `FRACTION` and `DIGITS` give the digit counts of a type, and `SIGNED`
  whether its storage is signed. Sealed traits give generic code the items of a type in two
  layers, each implemented for every storage. The general layer requires `Clone` but not `Copy`:
  `FixedDecimal`, with the digit counts, the queries, the arithmetic and `zero()`, `max_value()`
  and `min_value()`, `SignedFixedDecimal` and `UnsignedFixedDecimal` for each sign, and
  `DecimalOne`, with `one()`; these functions return the constants `ZERO`, `MAX`, `MIN` and `ONE`.
  The const layer, `ConstFixedDecimal` and `ConstDecimalOne`, adds `Copy` and the constants
  themselves, which a generic `const fn` can read. `ExactMul`, `WidenTo`, `ConvertTo` and
  `DivRound` have the methods between decimal types. Code generic over `Decimal<I, F, R>` adds the
  bound `Decimal<I, F, R>: FixedDecimal`.
- `parse` accepts ASCII text with an optional sign, point and exponent, and `_` separators as in
  Rust literals: the numeric strings of the General Decimal Arithmetic specification and every
  form that `f64` parses, such as `-1.5`, `+.5`, `5.`, `2.5E-3` and `1_000`. Infinity and NaN are
  recognized and reported as `NotFinite`, because a `Decimal` is always finite. A value that does
  not fit is `OutOfRange` or `Inexact`, never rounded.
- `Display` writes the shortest exact text, without an exponent, and `{:e}` and `{:E}` write it in
  scientific notation, such as `6.42505e4`. Format precision pads with zeros but never rounds.

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
`Uint<U144, [u64; 3]>`. They also name `u2048`, `u4096`, `i2048` and `i4096`. Any other width or
storage is written out, such as `Uint<U8192>` or `Uint<U24, [u8; 3]>`, and a width that `typenum`
has no constant for is computed, as in `Uint<Sum<U1024, U476>>` for 1500 bits.

- `const` items and methods: `BITS`, `ZERO`, `MIN` and `MAX`, and `ONE` from one bit for `Uint`
  and from two bits for `Int`; for a native integer, `new`, which panics if the value does not fit,
  `try_new`, which returns an error instead, and `value`; for an array, `from_limbs` for a width
  that fills its limbs, `checked_from_limbs` for any width, which returns `None` unless the limbs
  hold a value of `Bits` bits, and `to_limbs` and `as_limbs`; `is_zero` and `is_one`, plus
  `is_negative` and `is_positive` for `Int`.
- The arithmetic of the native integers at the width `Bits`, as `const fn`s: addition, subtraction,
  multiplication and negation in the `checked_*`, `wrapping_*`, `overflowing_*`, `saturating_*` and
  `strict_*` forms, `carrying_add` and `borrowing_sub` for `Uint`, and `abs` and its forms,
  `unsigned_abs` and `signum` for `Int`. Addition, subtraction and multiplication, and negation for
  `Int`, also have the `unchecked_*` form, as `const unsafe fn`s whose caller guarantees that the
  `checked_*` method would return `Some`. The operators `+`, `-` and `*`, their compound
  assignments, `-` for `Int`, `Sum` and `Product` panic or wrap on overflow as `overflow-checks`
  decides, as for the native integers.
- Numeric equality, ordering and hashing, where the hash depends only on the value and `Bits`,
  whatever the storage, decimal formatting, and `Default`, which is zero.
- Traits with one implementation for every storage of a kind, for generic code, in two layers. The
  general layer requires `Clone` but not `Copy`: `FixedInteger`, with `zero()`, `min_value()`,
  `max_value()`, `is_zero`, `is_one` and the operators, and `IntegerOne`, with `one()`; these
  functions return the constants `ZERO`, `MIN`, `MAX` and `ONE`. The const layer,
  `ConstFixedInteger` and `ConstIntegerOne`, adds `Copy` and the constants themselves, which a
  generic `const fn` can read. `UnsignedFixedInteger` and `SignedFixedInteger` have the methods of
  each sign, `NativeInteger`, `ArrayInteger` and `FromLimbs` the construction of each kind, and the
  arithmetic traits `OverflowingOps`, `CheckedOps`, `WrappingOps`, `SaturatingOps`, `StrictOps`
  and `UncheckedOps` the arithmetic; with the bound `B: BitWidth`, such code uses `Uint<B>` and
  `Int<B>` for any width from 0 to 65536 bits.
- A width that its storage cannot hold, `2^32` bits or more, or a width written with leading zero
  bits, which no `typenum` constant or operator gives, is rejected during type checking, and so is a
  width of more than 65536 bits without an explicit storage.

Division and remainder, bit operations and shifts are planned.

#### License

`numo` is dual-licensed under the MIT license or the Apache License (Version 2.0), at your option.

See [LICENSE-APACHE](LICENSE-APACHE), [LICENSE-MIT](LICENSE-MIT) for details.

Copyright (c) 2026 Al Liu.

[Github-url]: https://github.com/al8n/numo/
[CI-url]: https://github.com/al8n/numo/actions/workflows/ci.yml
[doc-url]: https://docs.rs/numo
[crates-url]: https://crates.io/crates/numo
[codecov-url]: https://app.codecov.io/gh/al8n/numo/
[zh-cn-url]: https://github.com/al8n/numo/tree/main/README-zh_CN.md
[en-url]: https://github.com/al8n/numo/tree/main/README.md
