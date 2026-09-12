// Use the macro crate directly so both entry points are tested even when the
// ascent crate's optional parallel runtime is disabled.
ascent_macro::ascent_par! {
   #![provenance(ascent::WhyProvenance<String>)]
   relation input(i32);
}

fn main() {
   ascent_macro::ascent_run_par! {
      #![provenance(ascent::WhyProvenance<String>)]
      relation input(i32);
   };
}
