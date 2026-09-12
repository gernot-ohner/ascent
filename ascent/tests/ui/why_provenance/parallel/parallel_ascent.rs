use ascent::ascent_par;

ascent_par! {
   #[provenance(&'static str)]
   relation input(i32);
}

fn main() {}
