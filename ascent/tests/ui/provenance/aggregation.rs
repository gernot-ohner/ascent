use ascent::{HowProvenance, ascent_provenance};

ascent_provenance! {
   semiring HowProvenance<String>;
   relation input(i32);
   relation output(usize);
   output(count) <-- agg count = ascent::aggregators::count() in input(_);
}

fn main() {}
