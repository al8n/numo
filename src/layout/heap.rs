//! The heap storages of [`Decimal`](crate::Decimal): a wide integer of 192 to 1024 bits whose limbs
//! are in a `Box` or an `Rc`, and on a target with atomic pointers in an `Arc`, with the `std` or
//! the `alloc` feature; and on such a target in a `triomphe::Arc`, with the `triomphe` feature.
//!
//! Each is a heap integer of this crate, `Uint<B, P<[u64; N]>>` or `Int<B, P<[u64; N]>>`, whose
//! twin is the integer of the same width over the array itself, an inline storage. The twin
//! computes every coefficient, and a heap storage has the three methods of the protocol and no
//! row, so that generic code builds the storage of a decimal once, for the value that it returns.
//! The three methods are the protocol of the heap integers, which decides where a result lives: in
//! its allocation for a `Box`, and in a new allocation for a shared pointer, which never writes the
//! array that it holds.

#[cfg(all(any(feature = "std", feature = "alloc"), target_has_atomic = "ptr"))]
use std::sync::Arc;
#[cfg(any(feature = "std", feature = "alloc"))]
use std::{boxed::Box, rc::Rc};

#[cfg(all(feature = "triomphe", target_has_atomic = "ptr"))]
use triomphe::Arc as TriompheArc;

use super::{DigitsOf, FitsDigits, InlineStorage, Storage, StorageCore, Token};
#[cfg(any(feature = "std", feature = "alloc"))]
use super::{InPlace, MutableStorage};
use crate::{
  Int, Uint,
  width::{FillsRepr, IntCore, UintCore},
};

/// Implements the storage traits of [`Decimal`](crate::Decimal) for the wide integers whose limbs
/// are in the heap pointer `$pointer`, written `pointer, kind, [cfg]; Integer: Core, ..`, where
/// `kind` is `in_place` for a pointer that writes its array in place and `shared` for one that
/// never writes it, `cfg` is the predicate of the features and the target that give the pointer,
/// which the docs show, and `Core` is the storage core through which the pointer holds the limbs of
/// `Integer`. A storage of each integer whose twin, the integer over the array itself, is an inline
/// storage: its protocol, `Storage`, `MutableStorage` for the kind `in_place`, and the digits that
/// it holds.
macro_rules! decimal_storages {
  ($pointer:ident, $kind:ident, [$cfg:meta]; $($integer:ident: $core:ident),+ $(,)?) => {$(
    // The twin decides which integers are storages: a width that does not fill its array, another
    // limb type and a boxed native integer have no inline storage of a decimal as their twin. The
    // width that fills the array is named too, because `Storage` requires `Hash`, and the hash of
    // the integer needs the width relation of the pointer, which follows from that of the array.
    impl<B, const N: usize> StorageCore for $integer<B, $pointer<[u64; N]>>
    where
      B: FillsRepr<[u64; N]>,
      $integer<B, [u64; N]>: InlineStorage,
    {
      // The sign of the twin, as the bound of `Twin` requires: the compiler cannot match a bit
      // written here with the sign of the twin, an integer of a width and a length that it does not
      // know.
      type IsUnsigned = <$integer<B, [u64; N]> as StorageCore>::IsUnsigned;
      type Twin = $integer<B, [u64; N]>;

      // A pointer implements the storage cores of both signs, so each call names that of the
      // integer. The pointer lends its array, which the twin copies.
      #[inline]
      fn twin(&self, token: Token) -> Self::Twin {
        $integer::wrap(*<$pointer<[u64; N]> as $core>::inline(&self.repr, token))
      }

      #[inline]
      fn from_twin(value: Self::Twin, token: Token) -> Self {
        Self::wrap(<$pointer<[u64; N]> as $core>::from_inline(value.repr, token))
      }

      #[inline]
      fn set_twin(&mut self, value: Self::Twin, token: Token) {
        <$pointer<[u64; N]> as $core>::set_inline(&mut self.repr, value.repr, token);
      }
    }

    #[cfg_attr(docsrs, doc(cfg($cfg)))]
    impl<B, const N: usize> Storage for $integer<B, $pointer<[u64; N]>>
    where
      B: FillsRepr<[u64; N]>,
      $integer<B, [u64; N]>: InlineStorage,
    {
      type Inline = $integer<B, [u64; N]>;
    }

    decimal_storages!(@mutable $kind, $integer, $pointer, [$cfg]);

    // The digits that the integer holds are those of its twin, as its width relation is that of
    // the array: a count that does not fit reports the message of the twin.
    impl<D, B, const N: usize> DigitsOf<$integer<B, $pointer<[u64; N]>>> for D
    where
      D: DigitsOf<$integer<B, [u64; N]>>,
    {
    }

    #[cfg_attr(docsrs, doc(cfg($cfg)))]
    impl<D, B, const N: usize> FitsDigits<$integer<B, $pointer<[u64; N]>>> for D
    where
      D: FitsDigits<$integer<B, [u64; N]>>,
    {
    }
  )+};
  // A pointer that holds its array in place stores a coefficient in its allocation.
  (@mutable in_place, $integer:ident, $pointer:ident, [$cfg:meta]) => {
    impl<B, const N: usize> InPlace for $integer<B, $pointer<[u64; N]>>
    where
      B: FillsRepr<[u64; N]>,
      $integer<B, [u64; N]>: InlineStorage,
    {
    }

    #[cfg_attr(docsrs, doc(cfg($cfg)))]
    impl<B, const N: usize> MutableStorage for $integer<B, $pointer<[u64; N]>>
    where
      B: FillsRepr<[u64; N]>,
      $integer<B, [u64; N]>: InlineStorage,
    {
    }
  };
  // A shared pointer never writes the array that it holds, which another owner may read: it stores
  // a coefficient in a new allocation, which replaces the pointer, so a decimal over it has no
  // assignment.
  (@mutable shared, $integer:ident, $pointer:ident, [$cfg:meta]) => {};
}

#[cfg(any(feature = "std", feature = "alloc"))]
decimal_storages!(
  Box,
  in_place,
  [any(feature = "std", feature = "alloc")];
  Uint: UintCore,
  Int: IntCore
);
#[cfg(any(feature = "std", feature = "alloc"))]
decimal_storages!(
  Rc,
  shared,
  [any(feature = "std", feature = "alloc")];
  Uint: UintCore,
  Int: IntCore
);
#[cfg(all(any(feature = "std", feature = "alloc"), target_has_atomic = "ptr"))]
decimal_storages!(
  Arc,
  shared,
  [all(any(feature = "std", feature = "alloc"), target_has_atomic = "ptr")];
  Uint: UintCore,
  Int: IntCore
);
#[cfg(all(feature = "triomphe", target_has_atomic = "ptr"))]
decimal_storages!(
  TriompheArc,
  shared,
  [all(feature = "triomphe", target_has_atomic = "ptr")];
  Uint: UintCore,
  Int: IntCore
);
