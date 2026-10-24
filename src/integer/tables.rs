//! The method tables of [`Uint`](crate::Uint) and [`Int`](crate::Int).
//!
//! A table writes the documentation and the signature of each method once and passes them to a
//! callback, which emits the methods for one storage. A row whose method the storage computes ends
//! in `= prim name(receiver);`, the function of the storage's layer in `int` that it calls.

/// Passes the queries of `Uint` and `Int` to `$callback`, after the tokens of `$args`.
macro_rules! integer_query_table {
  ($callback:ident! $args:tt) => {
    $callback! { $args
      /// Returns `true` if the value is zero.
      fn is_zero(&self) -> bool = prim is_zero(self);
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
/// for a native integer, which its layer in `int` takes by value, or `[layer, &]` for an array,
/// which its layer takes by reference.
// The parameters and the receiver both come from the table, so that the `self` of the body is the
// `self` of the signature: macro hygiene keeps apart a `self` written in this macro.
macro_rules! inherent_items {
  ($args:tt $(
    $(#[doc = $doc:expr])*
    fn $name:ident($($param:tt)*) -> $ret:ty = prim $prim:ident($receiver:ident);
  )*) => {$(
    $(#[doc = $doc])*
    #[must_use]
    #[inline]
    pub const fn $name($($param)*) -> $ret {
      inherent_items!(@prim $args $prim($receiver))
    }
  )*};
  (@prim [$layer:ident] $prim:ident($receiver:ident)) => {
    int::$layer::$prim($receiver.repr)
  };
  (@prim [$layer:ident, &] $prim:ident($receiver:ident)) => {
    int::$layer::$prim(&$receiver.repr)
  };
}
