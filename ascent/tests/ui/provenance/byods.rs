use ascent::{HowProvenance, ascent_provenance};

ascent_provenance! {
   semiring HowProvenance<String>;
   #![ds(ascent::rel)]
   relation input(i32);
   relation output(i32);
   output(x) <-- input(x);
}

fn main() {}
