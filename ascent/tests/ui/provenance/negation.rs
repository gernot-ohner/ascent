use ascent::{HowProvenance, ascent_provenance};

ascent_provenance! {
   semiring HowProvenance<String>;
   relation input(i32);
   relation excluded(i32);
   relation output(i32);
   output(x) <-- input(x), !excluded(x);
}

fn main() {}
