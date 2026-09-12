use ascent::ascent;

ascent! {
   #[provenance(&'static str)] relation input(i32);
   #[provenance(&'static str)] relation output(usize);

   output(n) <-- agg n = count() in input(_);
}

fn main() {}
