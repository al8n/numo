use numo::u256;

// `required` takes the integer itself; an `Option` field takes `optional`.
#[derive(serde_full::Serialize, serde_full::Deserialize)]
#[serde(crate = "serde_full")]
struct Transaction {
  #[serde(with = "numo::serde::quantity::required")]
  gas: Option<u256>,
}

fn main() {}
