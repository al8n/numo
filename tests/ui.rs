//! Compile-failure diagnostics, compared against the `.stderr` snapshot next to each case.
//!
//! Compiler diagnostics change between releases, and nightly compilers change them first, so the
//! snapshots are compared only on stable compilers. On nightly, beta and development compilers
//! the test checks nothing and passes.
//!
//! Regenerate the snapshots with `TRYBUILD=overwrite cargo +stable test --test ui` after reviewing
//! a change in the compiler output. The cases in `tests/ui/serde` need `--features serde`. The
//! cases in `tests/ui/inline` and `tests/ui/heap` print lists of implementors, which the heap
//! storages of a build join: those in `tests/ui/inline` need a build without the `std` and the
//! `alloc` features, as `--no-default-features` gives, and those in `tests/ui/heap` one with
//! either of them.

use std::process::Command;

#[test]
#[cfg_attr(miri, ignore)]
fn ui() {
  if stable_compiler() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
    // trybuild builds the cases with the features of this test build, which it reads from the
    // fingerprint of the build. Where it cannot read them, it builds the cases with the default
    // features, and the serde cases fail on `numo::serde`, which needs `serde`.
    #[cfg(feature = "serde")]
    cases.compile_fail("tests/ui/serde/*.rs");
    // A list of implementors includes the heap storages in a build that has them.
    #[cfg(not(any(feature = "std", feature = "alloc")))]
    cases.compile_fail("tests/ui/inline/*.rs");
    #[cfg(any(feature = "std", feature = "alloc"))]
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
