//! The literal `dec!`: its forms and its sign, a named type, the contexts that give it a type, the
//! limits of every storage as literals, a caller's own `concat!` and `stringify!`, and the
//! messages of a literal that its type does not hold.
//!
//! A literal that its type does not hold fails the build only where its constant is evaluated, and
//! a check evaluates the constant of a literal in a `const` or a `static` item, or with `as Type`,
//! but not that of a literal written bare in a function body. So each function that writes bare
//! literals lists them first in a constant of the same type, and a comparison that a constant can
//! make is made there too.

use std::{format, string::ToString, vec, vec::Vec};

use num_bigint::BigInt;
use typenum::{
  U0, U1, U2, U4, U5, U8, U9, U10, U18, U19, U39, U58, U77, U78, U96, U97, U116, U135, U154,
};

use crate::{
  Decimal, FixedDecimal, ParseErrorKind, Scaled, bigint::tier, dec, decimal::literal::reject, i192,
  i256, i320, i384, i448, i512, u192, u256, u320, u384, u448, u512,
};

/// A type with room for the literals of the forms and of the contexts.
type Price = Decimal<U10, U8>;

// One type of each storage at the digit limit of the storage, among them types without integral
// digits and types without fraction digits, and the type with the most fraction digits.
type Di8 = Decimal<U1, U1, i8>;
type Di16 = Decimal<U0, U4, i16>;
type Di32 = Decimal<U5, U4, i32>;
type Di64 = Decimal<U10, U8, i64>;
type Di128 = Decimal<U19, U19, i128>;
type Di192 = Decimal<U39, U18, i192>;
type Di256 = Decimal<U58, U18, i256>;
type Di320 = Decimal<U96, U0, i320>;
type Di384 = Decimal<U97, U18, i384>;
type Di448 = Decimal<U116, U18, i448>;
type Di512 = Decimal<U135, U18, i512>;
type Du8 = Decimal<U1, U1, u8>;
type Du16 = Decimal<U2, U2, u16>;
type Du32 = Decimal<U0, U9, u32>;
type Du64 = Decimal<U10, U9, u64>;
type Du128 = Decimal<U19, U19, u128>;
type Du192 = Decimal<U39, U18, u192>;
type Du256 = Decimal<U77, U0, u256>;
type Du320 = Decimal<U78, U18, u320>;
type Du384 = Decimal<U97, U18, u384>;
type Du448 = Decimal<U116, U18, u448>;
type Du512 = Decimal<U154, U0, u512>;
type Du0x154 = Decimal<U0, U154, u512>;

/// Whether two values of one decimal type are equal, in a constant.
macro_rules! same {
  ($a:expr, $b:expr) => {
    matches!($a.checked_sub(&$b), Some(difference) if difference.is_zero())
  };
}

fn parse<T: FixedDecimal>(text: &str) -> T {
  text.parse().unwrap_or_else(|err| panic!("{text:?}: {err}"))
}

/// The coefficient of `value`: the integer that its text with every fraction digit of the type
/// writes, without the point.
fn coefficient<T: FixedDecimal>(value: &T) -> BigInt {
  let digits = T::FRACTION as usize;
  format!("{value:.digits$}")
    .replace('.', "")
    .parse()
    .expect("an integer")
}

#[test]
fn forms_read_as_parse_reads_them() {
  const SEPARATED: Price = dec!(1_000.000_1);
  const EXPONENT: Price = dec!(1e-8);
  const UPPER: Price = dec!(2.5E+3);
  const POINT: Price = dec!(5.);
  const ZEROS: Price = dec!(007.10);
  const _: () = assert!(SEPARATED.repr() == 100_000_010_000);
  const _: () = assert!(EXPONENT.repr() == 1);
  const _: () = assert!(UPPER.repr() == 250_000_000_000);
  const _: () = assert!(POINT.repr() == 500_000_000);
  const _: () = assert!(ZEROS.repr() == 710_000_000);
  // A storage that is not the default one.
  const UNSIGNED: Decimal<U2, U2, u16> = dec!(1_2.3e-1);
  const _: () = assert!(UNSIGNED.repr() == 123);

  let cases = [
    (SEPARATED, 100_000_010_000, "1_000.000_1"),
    (EXPONENT, 1, "1e-8"),
    (UPPER, 250_000_000_000, "2.5E+3"),
    (POINT, 500_000_000, "5."),
    (ZEROS, 710_000_000, "007.10"),
  ];
  for (literal, repr, text) in cases {
    assert_eq!(Some(literal), Price::from_repr(repr), "{text}");
    assert_eq!(literal, parse::<Price>(text), "{text}");
  }
  assert_eq!(Decimal::<U2, U2, u16>::from_repr(123), Some(UNSIGNED));
  assert_eq!(UNSIGNED, parse::<Decimal<U2, U2, u16>>("1_2.3e-1"));
}

/// The macro writes the sign itself, so a space after the `-`, which `stringify!` would keep,
/// changes nothing; `rustfmt` would remove the spaces, and with them the test.
#[test]
fn the_sign_reads_with_or_without_a_space() {
  #[rustfmt::skip]
  const SPELLINGS: [Price; 6] = [
    dec!(-1.5),
    dec!(- 1.5),
    dec!(-   1_5.0e-1),
    dec!(-1.5 as Price),
    dec!(- 1.5 as Price),
    dec!(-   1_5.0e-1 as Price),
  ];
  const _: () = {
    let mut index = 0;
    while index < SPELLINGS.len() {
      assert!(SPELLINGS[index].repr() == -150_000_000);
      index += 1;
    }
  };
  for spelling in SPELLINGS {
    assert_eq!(Some(spelling), Price::from_repr(-150_000_000));
    assert_eq!(spelling, parse::<Price>("-1.5"));
  }
}

const fn half() -> Price {
  dec!(0.5 as Price)
}

const fn quarter() -> Price {
  dec!(0.25)
}

trait Tenth {
  fn tenth() -> Self;
}

impl Tenth for Price {
  /// Without `as`, the literal takes the return type, `Self`.
  fn tenth() -> Self {
    dec!(0.1)
  }
}

#[test]
fn a_named_type_and_constant_functions() {
  const _: Price = dec!(0.1);
  const HALF: Price = half();
  const QUARTER: Price = quarter();
  const SUM: Option<Price> = dec!(0.5 as Price).checked_add(&dec!(0.25));
  const CENTS: Decimal<U2, U2, u16> = dec!(12.34 as Decimal<U2, U2, u16>);
  const QUOTED: Scaled<Decimal<U2, U2>> = dec!(1.50 as Scaled<Decimal<U2, U2>>);
  const _: () = assert!(HALF.repr() == 50_000_000 && QUARTER.repr() == 25_000_000);
  const _: () = assert!(matches!(SUM, Some(sum) if sum.repr() == 75_000_000));
  const _: () = assert!(CENTS.repr() == 1234 && QUOTED.scale() == 2);

  assert_eq!(HALF, parse::<Price>("0.5"));
  assert_eq!(QUARTER, parse::<Price>("0.25"));
  assert_eq!(SUM, Some(parse::<Price>("0.75")));
  assert!(dec!(0.5 as Price).is_positive());
  assert_eq!(dec!(0.5 as Price).checked_sub(&HALF), Some(Price::ZERO));
  assert_eq!(CENTS, parse::<Decimal<U2, U2, u16>>("12.34"));
  assert_eq!(QUOTED.to_string(), "1.50");
  assert_eq!(Price::tenth(), parse::<Price>("0.1"));
  let promoted: &'static Price = &dec!(0.5 as Price);
  assert_eq!(*promoted, HALF);
}

/// A literal without a type reads where `str` names another type: the expansion names the
/// primitive by its path.
#[test]
fn a_literal_where_str_names_another_type() {
  #[allow(
    non_camel_case_types,
    reason = "the alias shadows the primitive under test"
  )]
  type str = Decimal<U1, U2>;
  const INFERRED: str = dec!(1.25);
  const _: () = assert!(INFERRED.repr() == 125);
  assert_eq!(INFERRED, parse::<str>("1.25"));
}

/// A literal with `as Type` reads where `str` names another type, also when the type after `as`
/// is that one.
#[test]
fn a_literal_with_as_where_str_names_another_type() {
  #[allow(
    non_camel_case_types,
    reason = "the alias shadows the primitive under test"
  )]
  type str = Decimal<U1, U2>;
  const NAMED: str = dec!(1.25 as str);
  const _: () = assert!(NAMED.repr() == 125);
  assert_eq!(NAMED, parse::<str>("1.25"));
}

/// A type whose name in the test holds a constant.
trait Pick {
  type Type;
}

impl Pick for [u8; 2] {
  type Type = Price;
}

/// The type after `as` resolves where the macro is called, even when it names `__Literal`, the
/// name of a type that the expansion defines.
#[test]
fn a_type_after_as_named_like_the_literal_of_the_expansion() {
  type __Literal = Price;
  const LITERAL: Price = dec!(1.50 as __Literal);
  const _: () = assert!(LITERAL.repr() == 150_000_000);
  assert_eq!(LITERAL, parse::<Price>("1.50"));
}

/// The type after `as` resolves where the macro is called, even when a constant argument of it
/// names `__VALUE`.
#[test]
fn a_type_after_as_with_a_constant_named_like_a_constant_of_the_expansion() {
  const __VALUE: usize = 2;
  const PICKED: Price = dec!(1.50 as <[u8; __VALUE] as Pick>::Type);
  const _: () = assert!(PICKED.repr() == 150_000_000);
  assert_eq!(PICKED, parse::<Price>("1.50"));
}

struct Holder {
  price: Price,
}

fn identity(price: Price) -> Price {
  price
}

/// Each context that gives a bare literal its type, and each operand on the right of a value.
#[test]
#[allow(clippy::op_ref, reason = "the operators on references are under test")]
fn literals_take_the_type_of_their_context() {
  // Every literal that this function writes bare, so that a check evaluates each for `Price`.
  const _: [Price; 24] = [
    dec!(2.5),
    dec!(3.5),
    dec!(4.5),
    dec!(5.5),
    dec!(6.5),
    dec!(7.5),
    dec!(8.5),
    dec!(9.5),
    dec!(10.5),
    dec!(11.5),
    dec!(12.5),
    dec!(13.5),
    dec!(100),
    dec!(0.5),
    dec!(1.25),
    dec!(0.25),
    dec!(101),
    dec!(100.000_000_01),
    dec!(99.999_999_99),
    dec!(1),
    dec!(2),
    dec!(14.5),
    dec!(-0.5),
    dec!(0),
  ];
  const IN_CONST: Price = dec!(0.5);
  static IN_STATIC: Price = dec!(1.5);

  let array: [Price; 2] = [dec!(2.5), dec!(3.5)];
  let typed: Price = dec!(4.5);
  let argument = identity(dec!(5.5));
  let field = Holder { price: dec!(6.5) };
  let tuple: (Price, u8) = (dec!(7.5), 1);
  let mut vector: Vec<Price> = vec![dec!(8.5)];
  vector.push(dec!(14.5));
  let some: Option<Price> = Some(dec!(9.5));
  let closure = || -> Price { dec!(10.5) };
  let matched: Price = match tuple.1 {
    1 => dec!(11.5),
    _ => dec!(12.5),
  };
  let promoted: &'static Price = &dec!(13.5);
  let returned = Price::tenth();
  let contexts = [
    (IN_CONST, "0.5"),
    (IN_STATIC, "1.5"),
    (array[0], "2.5"),
    (array[1], "3.5"),
    (typed, "4.5"),
    (argument, "5.5"),
    (field.price, "6.5"),
    (tuple.0, "7.5"),
    (vector[0], "8.5"),
    (vector[1], "14.5"),
    (closure(), "10.5"),
    (matched, "11.5"),
    (*promoted, "13.5"),
    (returned, "0.1"),
  ];
  for (value, text) in contexts {
    assert_eq!(value, parse::<Price>(text), "{text}");
  }
  assert_eq!(some, Some(parse::<Price>("9.5")));

  let a: Price = dec!(100);
  assert_eq!(a + dec!(0.5), parse::<Price>("100.5"));
  assert_eq!(a - dec!(0.5), parse::<Price>("99.5"));
  assert_eq!(a + dec!(-0.5), parse::<Price>("99.5"));
  let mut b = a;
  b += dec!(1.25);
  b -= dec!(0.25);
  assert!(b == dec!(101));
  assert!(a < dec!(100.000_000_01));
  assert!(a > dec!(99.999_999_99));
  assert!(a > dec!(0));
  assert_eq!(a.checked_add(&dec!(1)), Some(parse::<Price>("101")));
  assert_eq!(a + dec!(1) + dec!(2), parse::<Price>("103"));
  assert_eq!(&a + &dec!(0.5), parse::<Price>("100.5"));
}

/// The statements of the first example in the docs of `dec!`.
#[test]
fn the_first_example_of_the_docs() {
  const _: [Price; 4] = [dec!(64_250.5), dec!(0.25), dec!(1e-2), dec!(64_250.76)];
  const _: Scaled<Price> = dec!(1.50);

  const TICK: Price = dec!(0.01);
  let mut price: Price = dec!(64_250.5);
  price += dec!(0.25);
  let ask = price + dec!(1e-2);
  assert!(ask == dec!(64_250.76));
  assert_eq!(
    dec!(0.5 as Price).checked_add(&TICK),
    Some(dec!(0.51 as Price))
  );
  let quoted: Scaled<Price> = dec!(1.50);
  assert_eq!(quoted.to_string(), "1.50");
}

/// The statements of the example of the type in the docs of `dec!`.
#[test]
fn the_example_of_the_type_in_the_docs() {
  const _: Price = dec!(64_250.5);

  let price: Price = dec!(64_250.5);
  assert!(price.is_positive());
  let fee = dec!(0.5 as Price) + price;
  assert!(price < fee);
}

/// Reads the limits of one type of each storage, and one unit of its last digit, as literals: each
/// inferred in a constant and named with `as`, and compared with `MAX` and `MIN` in a constant and
/// in the test. The limit is forwarded without its sign, and the arm writes the `-`.
macro_rules! limits_read {
  (signed $ty:ty, $max:literal, $unit:literal) => {
    limits_read!(@check $ty, [dec!($max), dec!(-$max), dec!($unit)], [
      dec!($max as T),
      dec!(-$max as T),
      dec!($unit as T),
    ], [
      stringify!($max),
      concat!("-", stringify!($max)),
      stringify!($unit),
    ])
  };
  (unsigned $ty:ty, $max:literal, $unit:literal) => {
    limits_read!(@check $ty, [dec!($max), dec!(0), dec!($unit)], [
      dec!($max as T),
      dec!(0 as T),
      dec!($unit as T),
    ], [stringify!($max), "0", stringify!($unit)])
  };
  (@check $ty:ty, $inferred:expr, $named:expr, $texts:expr) => {{
    #[inline(never)]
    fn storage() {
      type T = $ty;
      const INFERRED: [T; 3] = $inferred;
      const NAMED: [T; 3] = $named;
      const _: () = assert!(same!(INFERRED[0], T::MAX) && same!(INFERRED[1], T::MIN));
      const _: () = assert!(same!(NAMED[0], T::MAX) && same!(NAMED[1], T::MIN));
      const _: () = assert!(same!(INFERRED[2], NAMED[2]) && INFERRED[2].is_positive());
      limits_agree(&INFERRED, &NAMED, $texts);
    }
    storage();
  }};
}

/// Checks the limits that [`limits_read`] read against `MAX` and `MIN`, the unit against its
/// coefficient, and each value against the `parse` of its text.
#[inline(never)]
fn limits_agree<T: FixedDecimal>(inferred: &[T; 3], named: &[T; 3], texts: [&str; 3]) {
  assert_eq!(inferred, named);
  assert_eq!(inferred[0], T::max_value());
  assert_eq!(inferred[1], T::min_value());
  assert_eq!(coefficient(&inferred[2]), BigInt::from(1));
  for (value, text) in inferred.iter().zip(texts) {
    assert_eq!(*value, parse::<T>(text), "{text}");
  }
}

/// The limits of one type of each storage as literals; under Miri, of a signed native, an unsigned
/// native and a wide storage.
#[test]
fn limits_of_every_storage_as_literals() {
  limits_read!(signed Di8, 9.9, 0.1);
  limits_read!(signed Di256, 9999999999999999999999999999999999999999999999999999999999.999999999999999999, 0.000000000000000001);
  limits_read!(unsigned Du64, 9999999999.999999999, 0.000000001);
  if tier().is_miri() {
    return;
  }
  limits_read!(signed Di16, 0.9999, 0.0001);
  limits_read!(signed Di32, 99999.9999, 0.0001);
  limits_read!(signed Di64, 9999999999.99999999, 0.00000001);
  limits_read!(signed Di128, 9999999999999999999.9999999999999999999, 0.0000000000000000001);
  limits_read!(signed Di192, 999999999999999999999999999999999999999.999999999999999999, 0.000000000000000001);
  limits_read!(signed Di320, 999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999, 1);
  limits_read!(signed Di384, 9999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999.999999999999999999, 0.000000000000000001);
  limits_read!(signed Di448, 99999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999.999999999999999999, 0.000000000000000001);
  limits_read!(signed Di512, 999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999.999999999999999999, 0.000000000000000001);
  limits_read!(unsigned Du8, 9.9, 0.1);
  limits_read!(unsigned Du16, 99.99, 0.01);
  limits_read!(unsigned Du32, 0.999999999, 0.000000001);
  limits_read!(unsigned Du128, 9999999999999999999.9999999999999999999, 0.0000000000000000001);
  limits_read!(unsigned Du192, 999999999999999999999999999999999999999.999999999999999999, 0.000000000000000001);
  limits_read!(unsigned Du256, 99999999999999999999999999999999999999999999999999999999999999999999999999999, 1);
  limits_read!(unsigned Du320, 999999999999999999999999999999999999999999999999999999999999999999999999999999.999999999999999999, 0.000000000000000001);
  limits_read!(unsigned Du384, 9999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999.999999999999999999, 0.000000000000000001);
  limits_read!(unsigned Du448, 99999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999.999999999999999999, 0.000000000000000001);
  limits_read!(unsigned Du512, 9999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999, 1);
  limits_read!(unsigned Du0x154, 0.9999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999, 0.0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001);
}

/// A caller's own `concat!` and `stringify!` change nothing: the expansion names those of `core`.
#[test]
fn a_literal_ignores_the_callers_macros() {
  macro_rules! concat {
    ($($token:tt)*) => {
      "7"
    };
  }
  macro_rules! stringify {
    ($($token:tt)*) => {
      "7"
    };
  }
  const HALF: Price = dec!(0.5);
  const MINUS_HALF: Price = dec!(-0.5);
  const _: () = assert!(HALF.repr() == 50_000_000 && MINUS_HALF.repr() == -50_000_000);
  assert_eq!((concat!(-0.5), stringify!(0.5)), ("7", "7"));
  assert_eq!(HALF, parse::<Price>("0.5"));
  assert_eq!(MINUS_HALF, parse::<Price>("-0.5"));
}

#[test]
#[should_panic(expected = "the literal is not a decimal number")]
fn an_empty_text_is_not_a_decimal_number() {
  reject(ParseErrorKind::Empty);
}

#[test]
#[should_panic(expected = "the literal is not a decimal number")]
fn invalid_syntax_is_not_a_decimal_number() {
  reject(ParseErrorKind::InvalidSyntax);
}

#[test]
#[should_panic(expected = "the literal is not a decimal number")]
fn a_text_that_is_not_finite_is_not_a_decimal_number() {
  reject(ParseErrorKind::NotFinite);
}

#[test]
#[should_panic(expected = "the decimal literal is negative, and the type is unsigned")]
fn a_negative_literal_of_an_unsigned_type() {
  reject(ParseErrorKind::Negative);
}

#[test]
#[should_panic(expected = "the decimal literal is out of range of the type")]
fn a_literal_out_of_range() {
  reject(ParseErrorKind::OutOfRange);
}

#[test]
#[should_panic(expected = "the decimal literal has more fraction digits than the type")]
fn a_literal_with_too_many_fraction_digits() {
  reject(ParseErrorKind::Inexact);
}

#[test]
#[should_panic(expected = "the decimal literal has more than 255 fraction digits")]
fn a_literal_with_a_scale_above_255() {
  reject(ParseErrorKind::ScaleTooLarge);
}
