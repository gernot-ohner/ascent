use ascent::ascent;

ascent! {
   #[provenance(&'static str)]
   relation output(i32);

   output(1);
}

fn main() {}
