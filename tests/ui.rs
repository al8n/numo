//! Compile-failure diagnostics, compared against the `.stderr` snapshot next to each case.
//!
//! Compiler diagnostics change between releases, and nightly compilers change them first, so the
//! snapshots are compared only on stable compilers. On nightly, beta and development compilers
//! the test checks nothing and passes.
//!
//! Regenerate the snapshots with `TRYBUILD=overwrite cargo +stable test --test ui` after reviewing
//! a change in the compiler output. The cases in `tests/ui/serde` need `--features serde`. The
//! cases in `tests/ui/inline` and `tests/ui/heap` print lists of implementors, which the heap
//! storages of a build join: those in `tests/ui/inline` need a build without the `std`, the `alloc`
//! and the `triomphe` features, as `--no-default-features` gives, and those in `tests/ui/heap` one
//! with `std` or `alloc` and without `triomphe`. A build with `triomphe` takes neither.
//! `tests/ui/heap` also holds the cases that implement a trait whose items the `std` or the `alloc`
//! feature adds, which need those items to print one error.

use std::process::Command;

#[test]
#[cfg_attr(miri, ignore)]
fn ui() {
  if stable_compiler() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
    // trybuild builds the cases with the features of this test build, which it reads from the
    // fingerprint of the build, and a case can read them with `cfg(feature = "..")`, as the impl of
    // `tests/ui/fixed_non_zero_is_sealed.rs` does. Where trybuild cannot read them, it builds the
    // cases with the default features: the serde cases fail on `numo::serde`, which needs `serde`,
    // and a case that reads the features fails as well.
    #[cfg(feature = "serde")]
    cases.compile_fail("tests/ui/serde/*.rs");
    // A list of implementors includes the heap storages in a build that has them.
    #[cfg(not(any(feature = "std", feature = "alloc", feature = "triomphe")))]
    cases.compile_fail("tests/ui/inline/*.rs");
    #[cfg(all(any(feature = "std", feature = "alloc"), not(feature = "triomphe")))]
    cases.compile_fail("tests/ui/heap/*.rs");
  }
}

/// Returns `true` if `rustc --version` reports a stable release, not a nightly, beta or
/// development build.
fn stable_compiler() -> bool {
  let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
  match Command::new(rustc).arg("--version").output() {
    Ok(output) if output.status.success() => {
      let version = String::from_utf8_lossy(&output.stdout);
      version.starts_with("rustc ")
        && !["-nightly", "-beta", "-dev"]
          .iter()
          .any(|channel| version.contains(channel))
    }
    _ => false,
  }
}
