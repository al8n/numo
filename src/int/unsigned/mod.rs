use super::{half_limbs, limb_layer};

#[cfg_attr(
  not(test),
  expect(dead_code, reason = "only the -Dwarnings test build lints it fully")
)]
pub(crate) mod b128;
#[cfg_attr(
  not(test),
  expect(dead_code, reason = "only the -Dwarnings test build lints it fully")
)]
pub(crate) mod b16;
#[cfg_attr(
  not(test),
  expect(dead_code, reason = "only the -Dwarnings test build lints it fully")
)]
pub(crate) mod b32;
pub(crate) mod b64;
#[cfg_attr(
  not(test),
  expect(dead_code, reason = "only the -Dwarnings test build lints it fully")
)]
pub(crate) mod b8;
