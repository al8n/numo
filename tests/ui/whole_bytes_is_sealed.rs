use numo::WholeBytes;

// The seal of `WholeBytes` holds for a local type, which is no `typenum` number; the orphan rules
// keep other crates from implementing it for the numbers of `typenum`.
struct Local;

impl WholeBytes for Local {}

fn main() {}
