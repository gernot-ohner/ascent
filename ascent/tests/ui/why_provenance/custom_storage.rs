use ascent::ascent;

ascent! {
   #[provenance(&'static str)]
   #[ds(ascent::rel)]
   relation input(i32);
}

fn main() {}
