use ascent::{HowProvenance, ascent_provenance};

ascent_provenance! {
   semiring HowProvenance<String>;
   relation input(i32);
   input(1);
}

fn main() {}
