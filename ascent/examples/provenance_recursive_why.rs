use ascent::{WhyProvenance, ascent};

ascent! {
   #![provenance(WhyProvenance<String>)]

   struct CyclicReachability;

   relation edge(i32, i32);
   relation path(i32, i32);

   path(x, y) <-- edge(x, y);
   path(x, z) <-- path(x, y), edge(y, z);
}

fn main() {
   let mut program = CyclicReachability {
      edge: vec![(1, 2, WhyProvenance::token("a".to_owned())), (2, 1, WhyProvenance::token("b".to_owned()))],
      ..Default::default()
   };

   program.run();
   program.path.sort_by_key(|row| (row.0, row.1));

   for (from, to, provenance) in &program.path {
      println!("path({from}, {to}): {provenance}");
   }
}
