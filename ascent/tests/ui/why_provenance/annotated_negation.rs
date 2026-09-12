use ascent::ascent;

ascent! {
   #[provenance(&'static str)] relation input(i32);
   #[provenance(&'static str)] relation output(i32);

   output(x) <-- input(x), !input(x);
}

fn main() {}
