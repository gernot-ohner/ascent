use ascent::{HowProvenance, ascent};

ascent! {
   #![provenance(HowProvenance<String>)]
   relation input(i32);
   relation excluded(i32);
   relation output(i32);
   output(x) <-- input(x), !excluded(x);
}

fn main() {}
