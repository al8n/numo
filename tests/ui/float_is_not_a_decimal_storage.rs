use numo::Storage;

fn storage<R: Storage>() {}

fn main() {
  // A float holds no integer coefficient, so it is not a storage of `Decimal`.
  storage::<f64>();
}
