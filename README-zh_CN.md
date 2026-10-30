<div align="center">
<h1>numo</h1>
</div>
<div align="center">

位数与存储都写在类型里的精确定点十进制数，最多 153 位（显式指定 `u512` 时为 154 位），支持 `no_std`，不需要分配内存。

[<img alt="github" src="https://img.shields.io/badge/github-al8n/numo-8da0cb?style=for-the-badge&logo=Github" height="22">][Github-url]
[<img alt="Build" src="https://img.shields.io/github/actions/workflow/status/al8n/numo/ci.yml?logo=Github-Actions&style=for-the-badge" height="22">][CI-url]
[<img alt="codecov" src="https://img.shields.io/codecov/c/gh/al8n/numo?style=for-the-badge&logo=codecov" height="22">][codecov-url]

[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-numo-66c2a5?style=for-the-badge&labelColor=555555&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20">][doc-url]
[<img alt="crates.io" src="https://img.shields.io/crates/v/numo?style=for-the-badge&logo=data:image/svg+xml;base64,PD94bWwgdmVyc2lvbj0iMS4wIiBlbmNvZGluZz0iaXNvLTg4NTktMSI/Pg0KPCEtLSBHZW5lcmF0b3I6IEFkb2JlIElsbHVzdHJhdG9yIDE5LjAuMCwgU1ZHIEV4cG9ydCBQbHVnLUluIC4gU1ZHIFZlcnNpb246IDYuMDAgQnVpbGQgMCkgIC0tPg0KPHN2ZyB2ZXJzaW9uPSIxLjEiIGlkPSJMYXllcl8xIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHhtbG5zOnhsaW5rPSJodHRwOi8vd3d3LnczLm9yZy8xOTk5L3hsaW5rIiB4PSIwcHgiIHk9IjBweCINCgkgdmlld0JveD0iMCAwIDUxMiA1MTIiIHhtbDpzcGFjZT0icHJlc2VydmUiPg0KPGc+DQoJPGc+DQoJCTxwYXRoIGQ9Ik0yNTYsMEwzMS41MjgsMTEyLjIzNnYyODcuNTI4TDI1Niw1MTJsMjI0LjQ3Mi0xMTIuMjM2VjExMi4yMzZMMjU2LDB6IE0yMzQuMjc3LDQ1Mi41NjRMNzQuOTc0LDM3Mi45MTNWMTYwLjgxDQoJCQlsMTU5LjMwMyw3OS42NTFWNDUyLjU2NHogTTEwMS44MjYsMTI1LjY2MkwyNTYsNDguNTc2bDE1NC4xNzQsNzcuMDg3TDI1NiwyMDIuNzQ5TDEwMS44MjYsMTI1LjY2MnogTTQzNy4wMjYsMzcyLjkxMw0KCQkJbC0xNTkuMzAzLDc5LjY1MVYyNDAuNDYxbDE1OS4zMDMtNzkuNjUxVjM3Mi45MTN6IiBmaWxsPSIjRkZGIi8+DQoJPC9nPg0KPC9nPg0KPGc+DQo8L2c+DQo8Zz4NCjwvZz4NCjxnPg0KPC9nPg0KPGc+DQo8L2c+DQo8Zz4NCjwvZz4NCjxnPg0KPC9nPg0KPGc+DQo8L2c+DQo8Zz4NCjwvZz4NCjxnPg0KPC9nPg0KPGc+DQo8L2c+DQo8Zz4NCjwvZz4NCjxnPg0KPC9nPg0KPGc+DQo8L2c+DQo8Zz4NCjwvZz4NCjxnPg0KPC9nPg0KPC9zdmc+DQo=" height="22">][crates-url]
[<img alt="crates.io" src="https://img.shields.io/crates/d/numo?color=critical&logo=data:image/svg+xml;base64,PD94bWwgdmVyc2lvbj0iMS4wIiBzdGFuZGFsb25lPSJubyI/PjwhRE9DVFlQRSBzdmcgUFVCTElDICItLy9XM0MvL0RURCBTVkcgMS4xLy9FTiIgImh0dHA6Ly93d3cudzMub3JnL0dyYXBoaWNzL1NWRy8xLjEvRFREL3N2ZzExLmR0ZCI+PHN2ZyB0PSIxNjQ1MTE3MzMyOTU5IiBjbGFzcz0iaWNvbiIgdmlld0JveD0iMCAwIDEwMjQgMTAyNCIgdmVyc2lvbj0iMS4xIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHAtaWQ9IjM0MjEiIGRhdGEtc3BtLWFuY2hvci1pZD0iYTMxM3guNzc4MTA2OS4wLmkzIiB3aWR0aD0iNDgiIGhlaWdodD0iNDgiIHhtbG5zOnhsaW5rPSJodHRwOi8vd3d3LnczLm9yZy8xOTk5L3hsaW5rIj48ZGVmcz48c3R5bGUgdHlwZT0idGV4dC9jc3MiPjwvc3R5bGU+PC9kZWZzPjxwYXRoIGQ9Ik00NjkuMzEyIDU3MC4yNHYtMjU2aDg1LjM3NnYyNTZoMTI4TDUxMiA3NTYuMjg4IDM0MS4zMTIgNTcwLjI0aDEyOHpNMTAyNCA2NDAuMTI4QzEwMjQgNzgyLjkxMiA5MTkuODcyIDg5NiA3ODcuNjQ4IDg5NmgtNTEyQzEyMy45MDQgODk2IDAgNzYxLjYgMCA1OTcuNTA0IDAgNDUxLjk2OCA5NC42NTYgMzMxLjUyIDIyNi40MzIgMzAyLjk3NiAyODQuMTYgMTk1LjQ1NiAzOTEuODA4IDEyOCA1MTIgMTI4YzE1Mi4zMiAwIDI4Mi4xMTIgMTA4LjQxNiAzMjMuMzkyIDI2MS4xMkM5NDEuODg4IDQxMy40NCAxMDI0IDUxOS4wNCAxMDI0IDY0MC4xOTJ6IG0tMjU5LjItMjA1LjMxMmMtMjQuNDQ4LTEyOS4wMjQtMTI4Ljg5Ni0yMjIuNzItMjUyLjgtMjIyLjcyLTk3LjI4IDAtMTgzLjA0IDU3LjM0NC0yMjQuNjQgMTQ3LjQ1NmwtOS4yOCAyMC4yMjQtMjAuOTI4IDIuOTQ0Yy0xMDMuMzYgMTQuNC0xNzguMzY4IDEwNC4zMi0xNzguMzY4IDIxNC43MiAwIDExNy45NTIgODguODMyIDIxNC40IDE5Ni45MjggMjE0LjRoNTEyYzg4LjMyIDAgMTU3LjUwNC03NS4xMzYgMTU3LjUwNC0xNzEuNzEyIDAtODguMDY0LTY1LjkyLTE2NC45MjgtMTQ0Ljk2LTE3MS43NzZsLTI5LjUwNC0yLjU2LTUuODg4LTMwLjk3NnoiIGZpbGw9IiNmZmZmZmYiIHAtaWQ9IjM0MjIiIGRhdGEtc3BtLWFuY2hvci1pZD0iYTMxM3guNzc4MTA2OS4wLmkwIiBjbGFzcz0iIj48L3BhdGg+PC9zdmc+&style=for-the-badge" height="22">][crates-url]
<img alt="license" src="https://img.shields.io/badge/License-Apache%202.0/MIT-blue.svg?style=for-the-badge&fontColor=white&logoColor=f5c076&logo=data:image/svg+xml;base64,PCFET0NUWVBFIHN2ZyBQVUJMSUMgIi0vL1czQy8vRFREIFNWRyAxLjEvL0VOIiAiaHR0cDovL3d3dy53My5vcmcvR3JhcGhpY3MvU1ZHLzEuMS9EVEQvc3ZnMTEuZHRkIj4KDTwhLS0gVXBsb2FkZWQgdG86IFNWRyBSZXBvLCB3d3cuc3ZncmVwby5jb20sIFRyYW5zZm9ybWVkIGJ5OiBTVkcgUmVwbyBNaXhlciBUb29scyAtLT4KPHN2ZyBmaWxsPSIjZmZmZmZmIiBoZWlnaHQ9IjgwMHB4IiB3aWR0aD0iODAwcHgiIHZlcnNpb249IjEuMSIgaWQ9IkNhcGFfMSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIiB4bWxuczp4bGluaz0iaHR0cDovL3d3dy53My5vcmcvMTk5OS94bGluayIgdmlld0JveD0iMCAwIDI3Ni43MTUgMjc2LjcxNSIgeG1sOnNwYWNlPSJwcmVzZXJ2ZSIgc3Ryb2tlPSIjZmZmZmZmIj4KDTxnIGlkPSJTVkdSZXBvX2JnQ2FycmllciIgc3Ryb2tlLXdpZHRoPSIwIi8+Cg08ZyBpZD0iU1ZHUmVwb190cmFjZXJDYXJyaWVyIiBzdHJva2UtbGluZWNhcD0icm91bmQiIHN0cm9rZS1saW5lam9pbj0icm91bmQiLz4KDTxnIGlkPSJTVkdSZXBvX2ljb25DYXJyaWVyIj4gPGc+IDxwYXRoIGQ9Ik0xMzguMzU3LDBDNjIuMDY2LDAsMCw2Mi4wNjYsMCwxMzguMzU3czYyLjA2NiwxMzguMzU3LDEzOC4zNTcsMTM4LjM1N3MxMzguMzU3LTYyLjA2NiwxMzguMzU3LTEzOC4zNTcgUzIxNC42NDgsMCwxMzguMzU3LDB6IE0xMzguMzU3LDI1OC43MTVDNzEuOTkyLDI1OC43MTUsMTgsMjA0LjcyMywxOCwxMzguMzU3UzcxLjk5MiwxOCwxMzguMzU3LDE4IHMxMjAuMzU3LDUzLjk5MiwxMjAuMzU3LDEyMC4zNTdTMjA0LjcyMywyNTguNzE1LDEzOC4zNTcsMjU4LjcxNXoiLz4gPHBhdGggZD0iTTE5NC43OTgsMTYwLjkwM2MtNC4xODgtMi42NzctOS43NTMtMS40NTQtMTIuNDMyLDIuNzMyYy04LjY5NCwxMy41OTMtMjMuNTAzLDIxLjcwOC0zOS42MTQsMjEuNzA4IGMtMjUuOTA4LDAtNDYuOTg1LTIxLjA3OC00Ni45ODUtNDYuOTg2czIxLjA3Ny00Ni45ODYsNDYuOTg1LTQ2Ljk4NmMxNS42MzMsMCwzMC4yLDcuNzQ3LDM4Ljk2OCwyMC43MjMgYzIuNzgyLDQuMTE3LDguMzc1LDUuMjAxLDEyLjQ5NiwyLjQxOGM0LjExOC0yLjc4Miw1LjIwMS04LjM3NywyLjQxOC0xMi40OTZjLTEyLjExOC0xNy45MzctMzIuMjYyLTI4LjY0NS01My44ODItMjguNjQ1IGMtMzUuODMzLDAtNjQuOTg1LDI5LjE1Mi02NC45ODUsNjQuOTg2czI5LjE1Miw2NC45ODYsNjQuOTg1LDY0Ljk4NmMyMi4yODEsMCw0Mi43NTktMTEuMjE4LDU0Ljc3OC0zMC4wMDkgQzIwMC4yMDgsMTY5LjE0NywxOTguOTg1LDE2My41ODIsMTk0Ljc5OCwxNjAuOTAzeiIvPiA8L2c+IDwvZz4KDTwvc3ZnPg==" height="22">

[English][en-url] | 简体中文

</div>

## 概览

`numo` 提供一组数字类型：大小是 [`typenum`](https://crates.io/crates/typenum)
的类型层面数字，存储是在类型里指定的整数：原生整数，或者本 crate 的 192、256、320、384、448、512 位整数。容量在类型检查阶段核对，放不下位数的类型无法创建任何值。
算术方法按引用接收 `self` 与操作数，例如 `a.checked_add(&b)`，因此调用时不会复制宽整数；运算符则保持标准库 trait 的签名。

| 类型族 | 状态 |
|---|---|
| `Decimal<Integral, Fraction, Repr>`：精确的定点十进制数 | 已提供 |
| `Uint<Bits, Repr>` 与 `Int<Bits, Repr>`：任意位宽的整数 | 已提供以最多 128 位的原生整数或 8、16、32、64、128 位 limb 存储的任意位宽，包括默认存储、别名 `u0`–`u1024` 与 `i0`–`i1024`、常量、带检查的构造、比较、哈希、格式化，与原生整数各种溢出形式相同的加、减、乘与取负及其运算符，以及泛型 trait；除法、位运算与移位计划中 |
| 标准格式的浮点类型，例如 `f16`、`bf16` 与 `f128` | 计划中 |
| 上述整数类型的原子版本 | 计划中 |

## 安装

```sh
cargo add numo
```

## 示例

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

- 值为 `c / 10^Fraction`，系数 `c` 存放在 `Repr` 中，并且 `|c| <= 10^(Integral + Fraction) - 1`，
  与 SQL 的 `NUMERIC(Integral + Fraction, Fraction)` 对应。没有 NaN、无穷，也没有负零。
- `Repr` 是有符号或无符号的整数，默认取能放下这些位数的最小有符号整数：`i8` 最多 2 位，
  `i16` 最多 4 位，`i32` 最多 9 位，`i64` 最多 18 位，`i128` 最多 38 位，宽整数 `i192` 最多 57 位，
  `i256` 最多 76 位，`i320` 最多 96 位，`i384` 最多 115 位，`i448` 最多 134 位，`i512` 最多 153 位。
  也可以显式指定更宽的 `Repr`，或者无符号的 `Repr`：
  `u8` 最多 2 位，`u16` 最多 4 位，`u32` 最多 9 位，`u64` 最多 19 位，`u128` 最多 38 位，
  `u192` 最多 57 位，`u256` 最多 77 位，`u320` 最多 96 位，`u384` 最多 115 位，`u448` 最多 134 位，
  `u512` 最多 154 位。类型的大小、对齐与 ABI 都与 `Repr` 相同。
- 有符号 `Repr` 的范围对称，并且实现 `Neg`。无符号 `Repr` 的值从零到 `MAX`，不能取负，
  `checked_sub` 的结果低于零时返回 `None`，解析时拒绝开头的 `-`，`-0` 也不例外。
- 位数为 0、超过 154 位或 `Repr` 放不下时，类型检查阶段就会报错。使用默认 `Repr` 时，位数为 0
  或超过 153 位的类型在任何写出它的地方都无法通过编译；显式指定 `Repr` 时，该类型的常量和构造函数
  无法通过编译，因此无法创建它的值。类型本身不带约束，泛型代码存放和传递这些值时不需要任何约束。
- 所有运算要么精确，要么失败：`checked_add` 与 `checked_sub` 超出范围时返回 `None`；`exact_mul`
  返回一定放得下乘积的更宽类型，乘积最多 153 位；`widen` 转换到位数不少于原类型的类型；`try_convert`
  在超出范围或会丢失位数时返回 `OutOfRange` 或 `Inexact`，从不舍入。只有显式给出 `RoundingMode`
  的地方才会舍入：`round_to` 转换到小数位更少的类型，`quantize` 舍入到步长的整数倍，`div_round`
  做除法并把商舍入到任意十进制类型。不提供 `+`、`-`、`*`、`/` 运算符。
- 转换遵循存储的符号：两个无符号存储的乘积用无符号存储，其他乘积用有符号存储；`widen`
  可以把无符号存储转换为任何存储，但有符号存储只能转换为有符号存储；`try_convert`
  把负数转换为无符号存储时返回 `OutOfRange`。
- 只依赖存储的方法都是 `const fn`，常量可以在编译期解析和计算。
- 常量 `INTEGRAL`、`FRACTION` 与 `DIGITS` 给出类型的位数，`SIGNED` 表示存储是否有符号。供泛型代码使用的
  sealed trait 分两层提供类型的各项，每种存储各有一个实现。通用层要求 `Clone`，不要求 `Copy`：
  `FixedDecimal` 提供位数、查询、算术以及 `zero()`、`max_value()` 与 `min_value()`，
  `SignedFixedDecimal` 与 `UnsignedFixedDecimal` 按符号区分，`DecimalOne` 提供 `one()`；这些函数返回
  常量 `ZERO`、`MAX`、`MIN` 与 `ONE`。常量层 `ConstFixedDecimal` 与 `ConstDecimalOne`
  另外要求 `Copy`，并直接提供这些常量，泛型 `const fn` 也能读取它们。`ExactMul`、`WidenTo`、
  `ConvertTo` 与 `DivRound` 提供十进制类型之间的方法。对 `Decimal<I, F, R>` 泛型的代码需要加上约束
  `Decimal<I, F, R>: FixedDecimal`。
- `parse` 接受 ASCII 文本，可以带符号、小数点与指数，并按 Rust 字面量的规则使用 `_` 分隔符：
  即 General Decimal Arithmetic 规范的数值字符串，以及 `f64` 能解析的全部写法，例如 `-1.5`、
  `+.5`、`5.`、`2.5E-3` 与 `1_000`。无穷与 NaN 能被识别，但报告为 `NotFinite`，因为 `Decimal`
  只表示有限值。放不下的值报告为 `OutOfRange` 或 `Inexact`，从不舍入。
- `Display` 输出最短的精确文本，不使用指数；`{:e}` 与 `{:E}` 以科学计数法输出同一个值，例如
  `6.42505e4`。格式化精度只补零，从不舍入。

## 宽存储

超过 38 位的十进制数存放在 `i192`、`i256`、`i320`、`i384`、`i448`、`i512` 或 `u192`、`u256`、`u320`、
`u384`、`u448`、`u512` 中，它们是 `Int` 与 `Uint` 的别名，分别由 3 到 8 个 64 位 limb 组成。任意 uint256
数额（例如代币余额）最多 78 位，放得进 `u320`。第 154 位只有 `u512` 放得下，必须显式指定，例如
`Decimal<U154, U0, u512>`：没有有符号存储能放下 154 位，因此默认存储与 `exact_mul` 的乘积最多到 153 位。
宽系数由 limb 构造，最低位的 limb 在前，再由 `from_repr` 检查范围；带指数的十进制文本解析得到同一个值。

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

## 整数

`Uint<Bits, Repr>` 与 `Int<Bits, Repr>` 可以是任意位宽 `Bits`，存放在至少有 `Bits` 位的存储 `Repr` 中：
原生整数（`Uint` 用 `u8` 到 `u128`，`Int` 用 `i8` 到 `i128`）直接存放值本身；limb 数组则最低位的 limb 在前，
`Int` 使用二进制补码：`[u64; N]`、`[u32; N]`、`[u16; N]`、`[u8; N]` 或 `[u128; N]`。零位宽的整数只能存放零，
任何存储都一样。省略 `Repr` 时，零位宽的存储是零大小的 `[u64; 0]`，1 到 128 位是能放下该位宽的最小原生整数，
129 到 65536 位则是 `[u64; ceil(Bits / 64)]`，因此 `Uint<U7>` 就是 `Uint<U7, u8>`，`Uint<U200>` 就是
`Uint<U200, [u64; 4]>`。
`[u32; N]` 的运算使用原生 64 位整数作中间值，`[u8; N]` 的对齐为 1，在任何目标上都按小端序存放值的字节。
`[u128; N]` 的内存布局与 `u128` 数组相同，适合与存放 128 位字的代码共享内存。没有更宽的原生整数，所以它的运算
在每个 limb 的两个 64 位半部分上进行；它相对 `[u64; N]` 的速度取决于运算、位宽与目标平台（格式化只有几个 limb
的值要慢好几倍），因此请为了布局而不是为了速度选用它。
别名包括 `u0` 到 `u1024` 与 `i0` 到 `i1024`（原生位宽 8、16、32、64、128 除外），使用默认存储：零位宽存放在
`[u64; 0]` 中，因此 `u0` 与 `i0` 不占空间；1 到 127 位存放在最小的原生整数中，例如 `u7` 即 `Uint<U7, u8>`；
129 到 1024 位存放在 `[u64; ceil(N / 64)]` 中，例如 `u144` 即
`Uint<U144, [u64; 3]>`。另有 `u2048`、`u4096`、`i2048` 与 `i4096`。其他位宽或存储要写出类型，例如
`Uint<U8192>` 或 `Uint<U24, [u8; 3]>`；`typenum` 没有常量的位宽要用运算得出，例如 1500 位写作
`Uint<Sum<U1024, U476>>`。

- `const` 常量与方法：`BITS`、`ZERO`、`MIN` 与 `MAX`，以及 `ONE`（`Uint` 至少 1 位、`Int` 至少 2 位时才有）；
  原生整数存储有 `new`（值放不下时 panic）、`try_new`（改为返回错误）与 `value`；limb 数组存储有位宽填满
  limb 时的 `from_limbs`、适用于任意位宽的 `checked_from_limbs`（limb 中的值超出 `Bits` 位的范围时返回
  `None`），以及 `to_limbs` 与 `as_limbs`；`is_zero` 与 `is_one`，`Int` 还有 `is_negative` 与 `is_positive`。
- 与原生整数相同、按位宽 `Bits` 计算的算术，均为 `const fn`：`checked_*`、`wrapping_*`、`overflowing_*`、
  `saturating_*` 与 `strict_*` 各种形式的加、减、乘与取负，`Uint` 的 `carrying_add` 与 `borrowing_sub`，
  以及 `Int` 的 `abs` 及其各种形式、`unsigned_abs` 与 `signum`。加、减、乘（`Int` 还有取负）另有
  `unchecked_*` 形式，均为 `const unsafe fn`，调用者须保证对应的 `checked_*` 方法会返回 `Some`。
  运算符 `+`、`-`、`*` 及其复合赋值、`Int` 的取负 `-`、`Sum` 与 `Product` 溢出时与原生整数一样，由
  `overflow-checks` 决定 panic 还是回绕。
- 按数值的相等、比较与哈希，哈希只取决于值与 `Bits`，与存储无关；十进制格式化；`Default` 为零。
- 供泛型代码使用的 trait 分两层，每类存储各只有一个实现。通用层要求 `Clone`，不要求 `Copy`：
  `FixedInteger` 提供 `zero()`、`min_value()`、`max_value()`、`is_zero`、`is_one` 与运算符，
  `IntegerOne` 提供 `one()`；这些函数返回常量 `ZERO`、`MIN`、`MAX` 与 `ONE`。常量层
  `ConstFixedInteger` 与 `ConstIntegerOne` 另外要求 `Copy`，并直接提供这些常量，泛型 `const fn`
  也能读取它们。`UnsignedFixedInteger` 与 `SignedFixedInteger` 提供按符号区分的方法，
  `NativeInteger`、`ArrayInteger` 与 `FromLimbs` 提供各类存储的构造，算术 trait `OverflowingOps`、
  `CheckedOps`、`WrappingOps`、`SaturatingOps`、`StrictOps` 与 `UncheckedOps` 提供算术；加上
  `B: BitWidth` 约束后，泛型代码可以对 0 到 65536 位的任意位宽使用 `Uint<B>` 与 `Int<B>`。
- 存储放不下的位宽、`2^32` 位及以上的位宽，以及带前导零位的位宽写法（`typenum` 的常量与运算结果都不会这样写），在类型检查阶段就会报错；没有写出存储且超过 65536 位的位宽也一样。

除法与取余、位运算与移位计划中。

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
