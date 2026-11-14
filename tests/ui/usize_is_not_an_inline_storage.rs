use numo::InlineStorage;

fn inline_storage<R: InlineStorage>() {}

fn main() {
  // `usize`, whose width depends on the target, is not one of the native integers that a decimal
  // stores, so it is not an inline storage of `Decimal`.
  inline_storage::<usize>();
}
