use numo::{
  Int, Uint,
  typenum::{U25, U26, U54, U55},
};

fn main() {
  // `From` converts into a float a width whose every value it holds exactly, and `to_f64` and
  // `to_f32` round a wider one.
  let _ = f64::from(Uint::<U54>::new(1));
  let _ = f32::from(Uint::<U25>::new(1));
  let _ = f64::from(Int::<U55>::new(1));
  let _ = f32::from(Int::<U26>::new(1));
}
