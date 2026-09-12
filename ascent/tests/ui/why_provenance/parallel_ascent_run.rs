use ascent::ascent_run_par;

fn main() {
   let _ = ascent_run_par! {
      #[provenance(&'static str)]
      relation input(i32);
   };
}
