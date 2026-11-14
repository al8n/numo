//! The decimal literal [`dec!`](crate::dec): the macro, the two hidden traits that its expansion
//! names, and the messages of a literal that its type does not hold.
//!
//! The expansion defines a unit type that carries the text of the literal in [`LiteralText`], and
//! reads the value from [`DecimalLiteral`], which every decimal type and every [`Scaled`] over one
//! implements for every such text: its constant is the `parse` of that type. So the type is the one
//! that the context infers, and the text is read when the crate is compiled.
//!
//! [`Scaled`]: crate::Scaled

use super::ParseErrorKind;

/// The text of one literal of [`dec!`](crate::dec), carried by the unit type that the macro
/// defines for it.
///
/// It is public because the expansion names it from another crate. It is not part of the API, and
/// may change in any release.
#[doc(hidden)]
pub trait LiteralText {
  /// The text that the type of the literal parses.
  const TEXT: &'static str;
}

/// The value of the literal `T` of [`dec!`](crate::dec) in this type, implemented by every decimal
/// type and every [`Scaled`](crate::Scaled) over one.
///
/// It is public because the expansion names it from another crate. It is not part of the API, and
/// may change in any release.
#[doc(hidden)]
pub trait DecimalLiteral<T>: Sized {
  /// The `parse` of this type of the text of `T`. A text that it rejects fails to compile, with
  /// the message of its kind.
  const VALUE: Self;
}

/// Fails the evaluation of a literal's constant with the message of `kind`.
///
/// A literal reaches it only while the compiler evaluates the literal's constant, and
/// `#[track_caller]` makes the compiler report that constant, and not this function.
#[track_caller]
pub(super) const fn reject(kind: ParseErrorKind) -> ! {
  match kind {
    // No token gives an empty text, an infinity or a NaN, but the traits are public.
    ParseErrorKind::Empty | ParseErrorKind::InvalidSyntax | ParseErrorKind::NotFinite => {
      panic!("the literal is not a decimal number")
    }
    ParseErrorKind::Negative => panic!("the decimal literal is negative, and the type is unsigned"),
    ParseErrorKind::OutOfRange => panic!("the decimal literal is out of range of the type"),
    ParseErrorKind::Inexact => panic!("the decimal literal has more fraction digits than the type"),
    ParseErrorKind::ScaleTooLarge => {
      panic!("the decimal literal has more than 65535 fraction digits")
    }
  }
}

/// A decimal literal, read and checked when the crate is compiled.
///
/// `dec!(1.25)` is the value that [`parse`](crate::Decimal::parse) of its type reads from the text
/// `1.25`: the grammar, the range and the exactness are those of `parse`, and nothing is rounded.
/// The text is read when the crate is compiled, and the value is a constant, so the macro works in
/// a `const`, in a `static` and in a `const fn`. Its type is the one that the context expects, or
/// the one written after `as`, as in `dec!(0.5 as Price)`. A [`Scaled`](crate::Scaled) keeps the
/// scale of the literal, as its `parse` does: `dec!(1.50)` writes `1.50`.
///
/// ```
/// use numo::{
///   Decimal, Scaled, dec,
///   typenum::{U8, U10},
/// };
///
/// type Price = Decimal<U10, U8>;
///
/// const TICK: Price = dec!(0.01);
/// let mut price: Price = dec!(64_250.5);
/// price += dec!(0.25);
/// let ask = price + dec!(1e-2);
/// assert!(ask == dec!(64_250.76));
/// assert_eq!(dec!(0.5 as Price).checked_add(&TICK), Some(dec!(0.51 as Price)));
/// let quoted: Scaled<Price> = dec!(1.50);
/// assert_eq!(quoted.to_string(), "1.50");
/// ```
///
/// A literal that its type does not hold fails to compile, here one with nine fraction digits in
/// a type of eight:
///
/// ```compile_fail
/// use numo::{
///   Decimal, Scaled, dec,
///   typenum::{U8, U10},
/// };
///
/// type Price = Decimal<U10, U8>;
///
/// const TICK: Price = dec!(0.01);
/// let mut price: Price = dec!(64_250.000_000_005);
/// price += dec!(0.25);
/// let ask = price + dec!(1e-2);
/// assert!(ask == dec!(64_250.76));
/// assert_eq!(dec!(0.5 as Price).checked_add(&TICK), Some(dec!(0.51 as Price)));
/// let quoted: Scaled<Price> = dec!(1.50);
/// assert_eq!(quoted.to_string(), "1.50");
/// ```
///
/// # Literals
///
/// The macro reads one Rust number token, with an optional `-` before it: digits, `_`, a point,
/// and an exponent with `e` or `E` and an optional sign, as in `1_000.000_1`, `5.`, `2.5E+3` and
/// `-1e-8`. It does not read `+1.5` or `.5`, which are not Rust number tokens, and a radix prefix,
/// a suffix or a string is not a decimal number.
///
/// A macro that passes a literal on to `dec!` as one fragment passes its text as it was written, so
/// a minus sign with a space after it is not a decimal number there. Pass the number without its
/// sign, and write the sign in the macro, as in `dec!(-$number)`.
///
/// # The type
///
/// The literal needs a type from its context. It has none with no expected type at all; as the
/// left operand of an operator or of a comparison, and where both operands are literals; as the
/// receiver of a method; beside a reference, as in `&price + dec!(0.5)`; and in code generic over
/// the decimal. Write the literal on the right of a value, or its type after `as`. Code generic
/// over the decimal parses a text.
///
/// The type after `as` is written out: `Self` and a type parameter of the enclosing item do not
/// compile there, because the macro names the type in a constant item of its own. Without `as`, a
/// literal in a function that returns `Self` takes that type.
///
/// ```
/// use numo::{
///   Decimal, dec,
///   typenum::{U8, U10},
/// };
///
/// type Price = Decimal<U10, U8>;
///
/// let price: Price = dec!(64_250.5);
/// assert!(price.is_positive());
/// let fee = dec!(0.5 as Price) + price;
/// assert!(price < fee);
/// ```
///
/// Without the type of the binding, the literal has none:
///
/// ```compile_fail
/// use numo::{
///   Decimal, dec,
///   typenum::{U8, U10},
/// };
///
/// type Price = Decimal<U10, U8>;
///
/// let price = dec!(64_250.5);
/// assert!(price.is_positive());
/// let fee = dec!(0.5 as Price) + price;
/// assert!(price < fee);
/// ```
///
/// Nor has a left operand without `as`:
///
/// ```compile_fail
/// use numo::{
///   Decimal, dec,
///   typenum::{U8, U10},
/// };
///
/// type Price = Decimal<U10, U8>;
///
/// let price: Price = dec!(64_250.5);
/// assert!(price.is_positive());
/// let fee = dec!(0.5) + price;
/// assert!(price < fee);
/// ```
///
/// # Errors
///
/// A literal that its type does not hold is a compile error, with one of these messages:
///
/// - `the literal is not a decimal number`, for a radix prefix, a suffix or a string;
/// - `the decimal literal is negative, and the type is unsigned`;
/// - `the decimal literal is out of range of the type`, for too many integral digits;
/// - `the decimal literal has more fraction digits than the type`;
/// - `the decimal literal has more than 65535 fraction digits`, for a `Scaled`, whose scale is at
///   most 65535.
///
/// When the error is reported depends on where the literal stands:
///
/// - In a `const` or a `static` item, and with `as Type` anywhere, it is reported when the code is
///   checked, so `cargo check` and an editor show it.
/// - In a function body without `as Type`, it is reported when the code of that function is
///   generated: `cargo build`, `cargo test`, `cargo run` and the doctests report it, and
///   `cargo check` and Clippy do not.
/// - In a library, the code of a function that is generated in the crates that call it, such as a
///   generic one, may be generated only there, so the error may be reported only when such a
///   crate is built.
///
/// In library code, and wherever the check should report it, write `as Type` or a `const` item. A
/// literal that its type does not hold is a compile error in every build that produces code, and
/// never reaches run time.
///
/// # Cost
///
/// For the compiler, each literal is a type, an impl and one evaluation of `parse`, and a literal
/// with `as Type` is two of each. A table of thousands of values is better written as one text that
/// is parsed once.
#[macro_export]
macro_rules! dec {
  (- $literal:literal $(as $ty:ty)?) => {
    $crate::dec!(
      @text $crate::__private::concat!("-", $crate::__private::stringify!($literal)) $(, $ty)?
    )
  };
  ($literal:literal $(as $ty:ty)?) => {
    $crate::dec!(@text $crate::__private::stringify!($literal) $(, $ty)?)
  };
  (@text $text:expr) => {{
    struct __Literal;
    impl $crate::__private::LiteralText for __Literal {
      const TEXT: &'static $crate::__private::str = $text;
    }
    <_ as $crate::__private::DecimalLiteral<__Literal>>::VALUE
  }};
  (@text $text:expr, $ty:ty) => {{
    // The items of an expansion are not hygienic, so the type is named only out here, where no
    // named item of the expansion is in scope: inside the inner blocks, a caller's `__Literal`
    // would be the expansion's. A constant item is evaluated when the code is checked, where a
    // constant that a function body names waits until the code of the function is generated:
    // this is what `as Type` is for. The value comes from a struct expression and not from a
    // call, which keeps a borrow of it promotable to `&'static`.
    const _: $ty = $crate::dec!(@text $text);
    $crate::__private::Typed::<$ty> {
      value: $crate::dec!(@text $text),
    }
    .value
  }};
}
