use numo::i256;

// A QUANTITY has no sign, so `quantity` takes the unsigned integers alone.
#[derive(serde_full::Serialize, serde_full::Deserialize)]
#[serde(crate = "serde_full")]
struct Transfer {
  #[serde(with = "numo::serde::quantity::required")]
  value: i256,
}

fn main() {}
