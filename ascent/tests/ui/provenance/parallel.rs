use ascent::{HowProvenance, ascent_provenance_par};

ascent_provenance_par! {
   semiring HowProvenance<String>;
   relation input(i32);
   relation output(i32);
   output(x) <-- input(x);
}

fn main() {}
