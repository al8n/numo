use numo::u7;

fn main() {
  // `From` converts into a width that holds every value of the native integer, and so does the
  // `TryFrom` that the standard library derives from it; a narrower width converts with the casts.
  let _ = u7::from(5u64);
  let _ = u7::try_from(5u64);
}
