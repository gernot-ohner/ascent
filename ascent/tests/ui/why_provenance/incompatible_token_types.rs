use ascent::ascent;

ascent! {
   #[provenance(&'static str)] relation input(i32);
   #[provenance(u32)] relation output(i32);

   output(x) <-- input(x);
}

fn main() {}
