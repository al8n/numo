//! The method tables of [`Uint`](crate::Uint) and [`Int`](crate::Int).
//!
//! A table writes the documentation, the attributes and the signature of each method once and
//! passes them to a callback: `inherent_items!` emits the inherent `const fn`s of one storage,
//! `trait_items!` the declarations of a trait and `generic_items!` the methods of a trait
//! implementation that is generic over the storage. A row is either derived, with a body written in
//! terms of the methods and constants of `Self`, or computed by the storage, ending in
//! `= prim name(operands);`, where `name` is the function of the storage's layer in `int` and the
//! method of its sealed trait:
//!
//! - `= prim name(self);` returns what `name` returns for the value, as a query does;
//! - `= prim pair name(self, rhs);` and `= prim pair name(self);` also pass the width,
//!   `Self::BITS`, and return the value that `name` computes, with its overflow flag;
//! - `= prim value name(self);` returns the value that `name` computes;
//! - `= prim unsigned name(self);` returns the unsigned integer of the same width that `name`
//!   computes, whose type the inherent callback takes from its arguments.
//!
//! A derived body calls the inherent `const fn`s of the storage in an inherent block, and the trait
//! methods, which must be in scope, in a generic implementation.

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
/// array, which its layer in `int::group` takes by reference, followed for `Int` by the unsigned
/// integer of the same width, as in `[i8, Uint<B, u8>]`, which the `unsigned` rows return.
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
    fn $name:ident($($param:tt)*) -> $ret:ty $body:block
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[must_use]
    fn $name($($param)*) -> $ret;

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
    fn $name:ident($($param:tt)*) -> $ret:ty $body:block
    $($rest:tt)*
  ) => {
    $(#[$meta])*
    #[inline]
    fn $name($($param)*) -> $ret $body

    generic_items!([] $($rest)*);
  };
}
