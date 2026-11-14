use numo::{Decimal, InlineStorage, Storage};

// `Storage` does not require `Copy`, so a decimal over a storage that it bounds moves where it is
// used.
fn pair<I, F, R: Storage>(value: Decimal<I, F, R>) -> [Decimal<I, F, R>; 2] {
  [value, value]
}

// The fix: `InlineStorage` requires `Copy`, so the decimal is copied.
fn inline_pair<I, F, R: InlineStorage>(value: Decimal<I, F, R>) -> [Decimal<I, F, R>; 2] {
  [value, value]
}

fn main() {}
