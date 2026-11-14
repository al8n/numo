use numo::Storage;

// Code generic over the storage of a decimal.
fn storage<R: Storage>() {}

fn main() {
  // A heap storage of a decimal boxes the limbs of a wide integer, not a native integer.
  storage::<Box<i64>>();
}
