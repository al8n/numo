use numo::{Uint, typenum::U16384};

// The bytes helpers take the widths whose array of bytes exists, up to 8192 bits.
#[derive(serde_full::Serialize, serde_full::Deserialize)]
#[serde(crate = "serde_full")]
struct Wide {
  #[serde(with = "numo::serde::be_bytes::required")]
  value: Uint<U16384>,
}

fn main() {}
