use ascent::{HowProvenance, ascent};

ascent! {
   #![provenance(HowProvenance<String>)]
   relation edge(i32, i32);
   relation path(i32, i32);
   path(x, y) <-- edge(x, y);
   path(x, z) <-- path(x, y), edge(y, z);
}

fn main() {}
