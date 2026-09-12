use ascent::{Dual, HowProvenance, ascent};

ascent! {
   #![provenance(HowProvenance<String>)]
   relation input(i32);
   lattice output(i32, Dual<i32>);
   output(x, Dual(*x)) <-- input(x);
}

fn main() {}
