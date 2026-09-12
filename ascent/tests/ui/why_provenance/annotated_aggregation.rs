use ascent::ascent;

ascent! {
   #[provenance(&'static str)] relation input(i32);
   relation ordinary(usize);
   #[provenance(&'static str)] relation output(usize);

   ordinary(n), output(n) <-- agg n = count() in input(_);
}

fn main() {}
