use numo::{
  Decimal,
  typenum::{U2, U10},
};

// `Decimal` writes its decimal text already; `decimal` is for the integers.
#[derive(serde_full::Serialize, serde_full::Deserialize)]
#[serde(crate = "serde_full")]
struct Price {
  #[serde(with = "numo::serde::decimal::required")]
  amount: Decimal<U10, U2>,
}

fn main() {}
