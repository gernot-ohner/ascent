use ascent::{HowProvenance, ascent};

ascent! {
   #![provenance(HowProvenance<String>)]
   relation input(i32);
   relation output(usize);
   output(count) <-- agg count = ascent::aggregators::count() in input(_);
}

fn main() {}
