use ascent::{HowProvenance, ascent};

ascent! {
   #![provenance(HowProvenance<String>)]
   #![ds(ascent::rel)]
   relation input(i32);
   relation output(i32);
   output(x) <-- input(x);
}

fn main() {}
