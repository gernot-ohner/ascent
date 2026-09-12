use ascent::{HowProvenance, ascent};

ascent! {
   #![provenance(HowProvenance<String>)]
   relation input(i32);
   input(1);
}

fn main() {}
