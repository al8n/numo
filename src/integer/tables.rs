//! The method tables of [`Uint`](crate::Uint) and [`Int`](crate::Int).
//!
//! A table writes the documentation, the attributes and the signature of each method once and
//! passes them to a callback: `inherent_items!` emits the inherent `const fn`s of one storage,
//! `trait_items!` the declarations of a trait and `generic_items!` the methods of a trait
//! implementation that is generic over the storage. A row is either derived, with a body written in
//! terms of the methods and constants of `Self`, or computed by the storage, ending in
//! `= prim name(operands);`, where `name` is the function of the storage's layer in `int` and the
//! method of its sealed trait, and the operands, the receiver first, are passed by reference to an
//! array and by value to a native integer:
//!
//! - `= prim name(self);` returns what `name` returns for the value, as a query does,
//!   `= prim name(self, rhs);` what it returns for several operands, and
//!   `= prim name(self; arguments);` what it returns with the scalar arguments after the `;`, such
//!   as an index or `Self::BITS`, which are passed as written;
//! - `= prim pair name(self, rhs);` and `= prim pair name(self);` also pass the width,
//!   `Self::BITS`, and return the value that `name` computes, with its overflow flag; the scalar
//!   arguments after a `;`, as in `= prim pair name(self; exp);`, come before the width;
//! - `= prim value name(self);` returns the value that `name` computes, also with several operands
//!   or with scalar arguments;
//! - `= prim unsigned name(self);` returns the unsigned integer of the same width that `name`
//!   computes, also with several operands or with scalar arguments, and
//!   `= prim signed name(self; arguments);` the signed integer of the same width, with scalar
//!   arguments; the inherent callback takes the type from its arguments;
//! - `= prim name(operands; arguments) as result { body }` binds what `name` returns for the
//!   operands and the scalar arguments, such as `Self::BITS`, to `result`, and returns `body`. It
//!   is the one form whose body sees the raw storages, which it wraps with `Self::of` and tests for
//!   zero with `Self::holds_zero`, for the results that a derived body, generic over the storage,
//!   cannot compute: it can neither test a divisor for zero nor read the constant one. A generic
//!   implementation computes `name` on the inline storages that the operands lend, so that the row
//!   builds only the storage that it returns. Without a return type, as in
//!   `fn set_bit(&mut self, ..) = prim name(self; ..) as result { body }`, the method returns
//!   nothing, and its body may store a value with `self.store`.
//! - `= prim storage name(operands; arguments) as result { body }` binds what the row `name` of
//!   the storage itself returns for the storages of the operands, where the form above computes on
//!   the inline storages that they lend: for a row whose operands or results are of both signs,
//!   whose inline storages no bound relates. Such a row returns the inline storage for a value of
//!   its own sign, which the body wraps with `Self::of`, and a storage for a value of the other
//!   sign, which the body wraps with `Uint::wrap` or `Int::wrap`, so that a heap storage builds
//!   only what the method returns.
//!
//! A derived body calls the inherent `const fn`s of the storage in an inherent block, and the trait
//! methods, which must be in scope, in a generic implementation. A row writes `Self::bound(max)`
//! for the largest value if `max` and the smallest otherwise, and `self.duplicate()` for a copy of
//! the value: an inherent block defines these, `of`, `store` and `holds_zero` as `const fn`s that
//! read the constants, copy the value, write the field and test a storage for zero, and the generic
//! implementations reach them through the helper trait `Rows` of `integer::traits`, which computes
//! the limits from their words, clones the value and tests the inline storage.
//!
//! An unchecked row, `unsafe fn name(..) -> Self = unchecked method(operands), "message";`,
//! declares an unsafe method whose caller guarantees that `method`, the overflowing method of the
//! same operation, does not overflow. Only `trait_items!` and `generic_items!` take it: the generic
//! body returns the value of `method` and checks its flag with a debug assertion that panics with
//! `message`; it has no unsafe code. A derived unsafe row, `unsafe fn name(..) -> Self { body }`,
//! serves the traits in the same way, with a body that has no unsafe code either. Each storage
//! writes out its inherent `unchecked_*` methods.
//!
//! The tables of the non-zero integers, in `integer::nonzero`, have derived rows only, and the
//! derived arm of `inherent_items!` ignores their `$args`. In an inherent block `$args` is
//! `[integer, other]`, the integer of the same width and storage and the non-zero integer of the
//! other sign, which the rows name through `nonzero_integer!` and `nonzero_other!`, and in a trait
//! it is `[]`, for the associated types. A row calls `self.integer()`, which lends the value,
//! `self.into_integer()`, which takes it, `self.magnitude()`, the absolute value of a `NonZeroInt`
//! as the unsigned integer, the rows of the integer, `Self::known`, `Self::saturated`,
//! `Self::duplicate` and other rows: an inherent block defines these as `const fn`s, and the
//! generic implementations reach them through the helper traits of `integer::nonzero`.

/// Passes the queries of `Uint` and `Int` to `$callback`, after the tokens of `$args`.
macro_rules! integer_query_table {
  ($callback:ident! $args:tt) => {
    $callback! { $args
      /// Returns `true` if the value is zero.
      fn is_zero(&self) -> bool = prim is_zero(self);

      /// Returns `true` if the value is one.
      fn is_one(&self) -> bool = prim is_one(self);
    }
  };
}

/// Passes the sign queries of `Int` to `$callback`, after the tokens of `$args`.
macro_rules! signed_query_table {
  ($callback:ident! $args:tt) => {
    $callback! { $args
      /// Returns `true` if the value is negative.
      fn is_negative(&self) -> bool = prim is_negative(self);

      /// Returns `true` if the value is positive.
      fn is_positive(&self) -> bool = prim is_positive(self);
    }
  };
}

/// Emits the rows of a method table as inherent `const fn`s of one storage. `$args` is `[layer]`
/// for a native integer, which its layer in `int` takes by value, or `[group, layer, &]` for an
/// array, which its layer in `int::group` takes by reference, followed, for a table that has rows
/// of the other sign, by the integer of the same width that they return: the unsigned one, as in
/// `[i8, Uint<B, u8>]`, for the `unsigned` rows, and the signed one, as in `[u8, Int<B, i8>]`, for
/// the `signed` rows. The last element thus names the unsigned type in some tables and the signed
/// type in others, and no table has both `unsigned` and `signed` rows.
// The parameters and the operands both come from the table: macro hygiene would keep a `self`
// written in this macro apart from the `self` parameter of a signature written in the table.
macro_rules! inherent_items {
  // The call of the layer function `$prim` on the storages of the operands, followed by the
  // arguments in `$extra`.
  (
    @call [$group:ident, $layer:ident, & $(, $unsigned:ty)?]
    $prim:ident($($operand:ident),+) $($extra:tt)*
  ) => {
    int::$group::$layer::$prim($(&$operand.repr),+ $($extra)*)
  };
  (@call [$layer:ident $(, $unsigned:ty)?] $prim:ident($($operand:ident),+) $($extra:tt)*) => {
    int::$layer::$prim($($operand.repr),+ $($extra)*)
  };
  // The unsigned integer of the same width.
  (@unsigned [$group:ident, $layer:ident, &, $unsigned:ty]) => {
    $unsigned
  };
  (@unsigned [$layer:ident, $unsigned:ty]) => {
    $unsigned
  };
  // The signed integer of the same width.
  (@signed [$group:ident, $layer:ident, &, $signed:ty]) => {
    $signed
  };
  (@signed [$layer:ident, $signed:ty]) => {
    $signed
  };
  ($args:tt) => {};
  (
    $args:tt
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty = prim $prim:ident($receiver:ident);
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[must_use]
    #[inline]
    pub const fn $name($($param)*) -> $ret {
      inherent_items!(@call $args $prim($receiver))
    }

    inherent_items!($args $($rest)*);
  };
  (
    $args:tt
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty = prim pair $prim:ident($($operand:ident),+);
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[must_use]
    #[inline]
    pub const fn $name($($param)*) -> $ret {
      let (value, overflow) = inherent_items!(@call $args $prim($($operand),+), Self::BITS);
      (Self::wrap(value), overflow)
    }

    inherent_items!($args $($rest)*);
  };
  (
    $args:tt
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty = prim value $prim:ident($receiver:ident);
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[must_use]
    #[inline]
    pub const fn $name($($param)*) -> $ret {
      Self::wrap(inherent_items!(@call $args $prim($receiver)))
    }

    inherent_items!($args $($rest)*);
  };
  (
    $args:tt
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty = prim unsigned $prim:ident($receiver:ident);
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[must_use]
    #[inline]
    pub const fn $name($($param)*) -> inherent_items!(@unsigned $args) {
      Uint::wrap(inherent_items!(@call $args $prim($receiver)))
    }

    inherent_items!($args $($rest)*);
  };
  (
    $args:tt
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty
      = prim $prim:ident($receiver:ident, $($operand:ident),+);
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[must_use]
    #[inline]
    pub const fn $name($($param)*) -> $ret {
      inherent_items!(@call $args $prim($receiver, $($operand),+))
    }

    inherent_items!($args $($rest)*);
  };
  (
    $args:tt
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty
      = prim pair $prim:ident($($operand:ident),+; $($arg:expr),+);
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[must_use]
    #[inline]
    pub const fn $name($($param)*) -> $ret {
      let (value, overflow) =
        inherent_items!(@call $args $prim($($operand),+), $($arg),+, Self::BITS);
      (Self::wrap(value), overflow)
    }

    inherent_items!($args $($rest)*);
  };
  (
    $args:tt
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty
      = prim value $prim:ident($receiver:ident, $($operand:ident),+);
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[must_use]
    #[inline]
    pub const fn $name($($param)*) -> $ret {
      Self::wrap(inherent_items!(@call $args $prim($receiver, $($operand),+)))
    }

    inherent_items!($args $($rest)*);
  };
  (
    $args:tt
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty
      = prim unsigned $prim:ident($receiver:ident, $($operand:ident),+);
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[must_use]
    #[inline]
    pub const fn $name($($param)*) -> inherent_items!(@unsigned $args) {
      Uint::wrap(inherent_items!(@call $args $prim($receiver, $($operand),+)))
    }

    inherent_items!($args $($rest)*);
  };
  (
    $args:tt
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty
      = prim $prim:ident($receiver:ident $(, $operand:ident)*; $($arg:expr),+);
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[must_use]
    #[inline]
    pub const fn $name($($param)*) -> $ret {
      inherent_items!(@call $args $prim($receiver $(, $operand)*), $($arg),+)
    }

    inherent_items!($args $($rest)*);
  };
  (
    $args:tt
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty
      = prim value $prim:ident($receiver:ident $(, $operand:ident)*; $($arg:expr),+);
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[must_use]
    #[inline]
    pub const fn $name($($param)*) -> $ret {
      Self::wrap(inherent_items!(@call $args $prim($receiver $(, $operand)*), $($arg),+))
    }

    inherent_items!($args $($rest)*);
  };
  (
    $args:tt
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty
      = prim unsigned $prim:ident($receiver:ident; $($arg:expr),+);
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[must_use]
    #[inline]
    pub const fn $name($($param)*) -> inherent_items!(@unsigned $args) {
      Uint::wrap(inherent_items!(@call $args $prim($receiver), $($arg),+))
    }

    inherent_items!($args $($rest)*);
  };
  (
    $args:tt
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty
      = prim signed $prim:ident($receiver:ident; $($arg:expr),+);
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[must_use]
    #[inline]
    pub const fn $name($($param)*) -> inherent_items!(@signed $args) {
      Int::wrap(inherent_items!(@call $args $prim($receiver), $($arg),+))
    }

    inherent_items!($args $($rest)*);
  };
  (
    $args:tt
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty
      = prim $prim:ident($($operand:ident),+ $(; $($arg:expr),+)?) as $result:ident $body:block
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[must_use]
    #[inline]
    pub const fn $name($($param)*) -> $ret {
      let $result = inherent_items!(@call $args $prim($($operand),+) $(, $($arg),+)?);
      $body
    }

    inherent_items!($args $($rest)*);
  };
  // A row of the storage itself: in an inline storage, the row of the storage and that of the
  // inline storage are one function of the layer, which the arm above calls.
  (
    $args:tt
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty
      = prim storage $prim:ident($($operand:ident),+ $(; $($arg:expr),+)?)
      as $result:ident $body:block
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[must_use]
    #[inline]
    pub const fn $name($($param)*) -> $ret {
      let $result = inherent_items!(@call $args $prim($($operand),+) $(, $($arg),+)?);
      $body
    }

    inherent_items!($args $($rest)*);
  };
  // A method that returns nothing, such as one that changes the value through `&mut self`.
  (
    $args:tt
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*)
      = prim $prim:ident($($operand:ident),+ $(; $($arg:expr),+)?) as $result:ident $body:block
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[inline]
    pub const fn $name($($param)*) {
      let $result = inherent_items!(@call $args $prim($($operand),+) $(, $($arg),+)?);
      $body
    }

    inherent_items!($args $($rest)*);
  };
  (
    $args:tt
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty $body:block
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[must_use]
    #[inline]
    pub const fn $name($($param)*) -> $ret $body

    inherent_items!($args $($rest)*);
  };
}

/// Emits the rows of a method table as the declarations of a trait.
macro_rules! trait_items {
  ([]) => {};
  (
    []
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty = prim $($prim:ident)+($($operand:ident),+);
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[must_use]
    fn $name($($param)*) -> $ret;

    trait_items!([] $($rest)*);
  };
  (
    []
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty
      = prim $($prim:ident)+($($operand:ident),+; $($arg:expr),+);
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[must_use]
    fn $name($($param)*) -> $ret;

    trait_items!([] $($rest)*);
  };
  (
    []
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty
      = prim $prim:ident($($operand:ident),+ $(; $($arg:expr),+)?) as $result:ident $body:block
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[must_use]
    fn $name($($param)*) -> $ret;

    trait_items!([] $($rest)*);
  };
  (
    []
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty
      = prim storage $prim:ident($($operand:ident),+ $(; $($arg:expr),+)?)
      as $result:ident $body:block
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[must_use]
    fn $name($($param)*) -> $ret;

    trait_items!([] $($rest)*);
  };
  (
    []
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*)
      = prim $prim:ident($($operand:ident),+ $(; $($arg:expr),+)?) as $result:ident $body:block
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    fn $name($($param)*);

    trait_items!([] $($rest)*);
  };
  (
    []
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty $body:block
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[must_use]
    fn $name($($param)*) -> $ret;

    trait_items!([] $($rest)*);
  };
  (
    []
    $(#[$meta:meta])*
    unsafe fn $name:ident($($param:tt)*) -> $ret:ty
      = unchecked $method:ident($($operand:ident),+), $message:literal;
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[must_use]
    unsafe fn $name($($param)*) -> $ret;

    trait_items!([] $($rest)*);
  };
  (
    []
    $(#[$meta:meta])*
    unsafe fn $name:ident($($param:tt)*) -> $ret:ty $body:block
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[must_use]
    unsafe fn $name($($param)*) -> $ret;

    trait_items!([] $($rest)*);
  };
}

/// Emits the rows of a method table as the methods of a trait implementation that is generic over
/// the storage, which call the storage through its sealed trait.
macro_rules! generic_items {
  ([]) => {};
  (
    []
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty = prim $prim:ident($receiver:ident);
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[inline]
    fn $name($($param)*) -> $ret {
      $receiver.repr.$prim(Token::new())
    }

    generic_items!([] $($rest)*);
  };
  (
    []
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty
      = prim pair $prim:ident($receiver:ident $(, $operand:ident)*);
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[inline]
    fn $name($($param)*) -> $ret {
      let (value, overflow) = $receiver.repr.$prim($(&$operand.repr,)* Self::BITS, Token::new());
      (Self::wrap(value), overflow)
    }

    generic_items!([] $($rest)*);
  };
  (
    []
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty = prim value $prim:ident($receiver:ident);
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[inline]
    fn $name($($param)*) -> $ret {
      Self::wrap($receiver.repr.$prim(Token::new()))
    }

    generic_items!([] $($rest)*);
  };
  (
    []
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty = prim unsigned $prim:ident($receiver:ident);
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[inline]
    fn $name($($param)*) -> $ret {
      Uint::wrap($receiver.repr.$prim(Token::new()))
    }

    generic_items!([] $($rest)*);
  };
  (
    []
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty
      = prim $prim:ident($receiver:ident, $($operand:ident),+);
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[inline]
    fn $name($($param)*) -> $ret {
      $receiver.repr.$prim($(&$operand.repr,)+ Token::new())
    }

    generic_items!([] $($rest)*);
  };
  (
    []
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty
      = prim pair $prim:ident($receiver:ident $(, $operand:ident)*; $($arg:expr),+);
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[inline]
    fn $name($($param)*) -> $ret {
      let (value, overflow) =
        $receiver.repr.$prim($(&$operand.repr,)* $($arg,)+ Self::BITS, Token::new());
      (Self::wrap(value), overflow)
    }

    generic_items!([] $($rest)*);
  };
  (
    []
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty
      = prim value $prim:ident($receiver:ident, $($operand:ident),+);
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[inline]
    fn $name($($param)*) -> $ret {
      Self::wrap($receiver.repr.$prim($(&$operand.repr,)+ Token::new()))
    }

    generic_items!([] $($rest)*);
  };
  (
    []
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty
      = prim unsigned $prim:ident($receiver:ident, $($operand:ident),+);
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[inline]
    fn $name($($param)*) -> $ret {
      Uint::wrap($receiver.repr.$prim($(&$operand.repr,)+ Token::new()))
    }

    generic_items!([] $($rest)*);
  };
  (
    []
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty
      = prim $prim:ident($receiver:ident $(, $operand:ident)*; $($arg:expr),+);
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[inline]
    fn $name($($param)*) -> $ret {
      $receiver.repr.$prim($(&$operand.repr,)* $($arg,)+ Token::new())
    }

    generic_items!([] $($rest)*);
  };
  (
    []
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty
      = prim value $prim:ident($receiver:ident $(, $operand:ident)*; $($arg:expr),+);
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[inline]
    fn $name($($param)*) -> $ret {
      Self::wrap($receiver.repr.$prim($(&$operand.repr,)* $($arg,)+ Token::new()))
    }

    generic_items!([] $($rest)*);
  };
  (
    []
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty
      = prim unsigned $prim:ident($receiver:ident; $($arg:expr),+);
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[inline]
    fn $name($($param)*) -> $ret {
      Uint::wrap($receiver.repr.$prim($($arg,)+ Token::new()))
    }

    generic_items!([] $($rest)*);
  };
  (
    []
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty
      = prim signed $prim:ident($receiver:ident; $($arg:expr),+);
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[inline]
    fn $name($($param)*) -> $ret {
      Int::wrap($receiver.repr.$prim($($arg,)+ Token::new()))
    }

    generic_items!([] $($rest)*);
  };
  // A row that sees raw storages computes on the inline storages that its operands lend.
  (
    []
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty
      = prim $prim:ident($receiver:ident $(, $operand:ident)* $(; $($arg:expr),+)?)
      as $result:ident $body:block
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[inline]
    fn $name($($param)*) -> $ret {
      let $result = $receiver.repr.inline(Token::new()).$prim(
        $($operand.repr.inline(Token::new()),)*
        $($($arg,)+)?
        Token::new()
      );
      $body
    }

    generic_items!([] $($rest)*);
  };
  // `storage` calls the row on the storages themselves, not on the inline storages that they lend.
  (
    []
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty
      = prim storage $prim:ident($receiver:ident $(, $operand:ident)* $(; $($arg:expr),+)?)
      as $result:ident $body:block
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[inline]
    fn $name($($param)*) -> $ret {
      let $result = $receiver.repr.$prim($(&$operand.repr,)* $($($arg,)+)? Token::new());
      $body
    }

    generic_items!([] $($rest)*);
  };
  // A method that returns nothing, such as one that changes the value through `&mut self`.
  (
    []
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*)
      = prim $prim:ident($receiver:ident $(, $operand:ident)* $(; $($arg:expr),+)?)
      as $result:ident $body:block
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[inline]
    fn $name($($param)*) {
      let $result = $receiver.repr.inline(Token::new()).$prim(
        $($operand.repr.inline(Token::new()),)*
        $($($arg,)+)?
        Token::new()
      );
      $body
    }

    generic_items!([] $($rest)*);
  };
  (
    []
    $(#[$meta:meta])*
    fn $name:ident($($param:tt)*) -> $ret:ty $body:block
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[inline]
    fn $name($($param)*) -> $ret $body

    generic_items!([] $($rest)*);
  };
  // The body is safe: it returns the value of the overflowing method, which is the exact result
  // when the caller's guarantee holds, and never calls the inherent `unchecked_*` methods.
  (
    []
    $(#[$meta:meta])*
    unsafe fn $name:ident($($param:tt)*) -> $ret:ty
      = unchecked $method:ident($receiver:ident $(, $operand:ident)*), $message:literal;
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[inline]
    unsafe fn $name($($param)*) -> $ret {
      let (value, overflow) = $receiver.$method($($operand),*);
      debug_assert!(!overflow, $message);
      value
    }

    generic_items!([] $($rest)*);
  };
  // The body has no unsafe code either, as the table's documentation requires, and never calls the
  // inherent `unchecked_*` methods.
  (
    []
    $(#[$meta:meta])*
    unsafe fn $name:ident($($param:tt)*) -> $ret:ty $body:block
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[inline]
    unsafe fn $name($($param)*) -> $ret $body

    generic_items!([] $($rest)*);
  };
}
