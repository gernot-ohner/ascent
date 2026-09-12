use ascent::ascent;

ascent! {
   #[provenance(&'static str)] relation input(i32);
   relation ordinary(i32);
   #[provenance(&'static str)] relation output(i32);

   ordinary(x), output(x) <-- input(x), !input(x);
}

fn main() {}
