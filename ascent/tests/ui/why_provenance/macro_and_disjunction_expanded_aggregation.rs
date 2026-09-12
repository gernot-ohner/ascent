use ascent::ascent;

ascent! {
   macro aggregate($x: ident) { agg $x = count() in input(_) }

   #[provenance(&'static str)] relation input(i32);
   #[provenance(&'static str)] relation output(i32);

   output(x) <-- (aggregate!(x) | input(x));
}

fn main() {}
