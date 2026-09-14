//! Run through `python3 ascent/examples/provsql/compare.py`.
use std::io::{self, BufRead};

use ascent::{WhyProvenance, ascent};

ascent! {
   struct Examples;
   #[provenance(u32)] relation edge(u32, u32);
   #[provenance(u32)] relation copy(u32, u32);
   #[provenance(u32)] relation join(u32, u32);
   #[provenance(u32)] relation projection(u32, u32);
   #[provenance(u32)] relation alternatives(u32, u32);
   #[provenance(u32)] relation self_join(u32, u32);
   #[provenance(u32)] relation non_absorption(u32, u32);
   #[provenance(u32)] relation alternative_product(u32, u32);
   #[provenance(u32)] relation no_match(u32, u32);

   copy(x, y) <-- edge(x, y);
   join(x, z) <-- edge(x, y), edge(y, z);
   projection(x, 0) <-- edge(x, _);
   alternatives(x, y) <-- edge(x, y), if *y == 1;
   alternatives(x, y) <-- edge(x, y), if *x == 0;
   self_join(x, y) <-- edge(x, y), edge(x, y);
   non_absorption(x, y) <-- edge(x, y);
   non_absorption(x, z) <-- edge(x, y), edge(y, z);
   alternative_product(x, y) <-- join(x, y), join(x, y);
   no_match(x, y) <-- edge(x, y), if *x == 99;
}

fn main() {
   let mut program = Examples::default();
   for line in io::stdin().lock().lines() {
      let values: Vec<u32> = line.unwrap().split_whitespace().map(|value| value.parse().unwrap()).collect();
      assert_eq!(values.len(), 3, "expected source, target, token");
      program.edge.push((values[0], values[1], WhyProvenance::token(values[2])));
   }
   program.run();
   for (name, rows) in [
      ("copy", program.copy),
      ("join", program.join),
      ("projection", program.projection),
      ("alternatives", program.alternatives),
      ("self_join", program.self_join),
      ("non_absorption", program.non_absorption),
      ("alternative_product", program.alternative_product),
      ("no_match", program.no_match),
   ] {
      println!("@{name}");
      for (source, target, why) in rows {
         let witnesses: Vec<Vec<u32>> = why.witnesses().iter().map(|set| set.iter().copied().collect()).collect();
         println!("{source}\t{target}\t{witnesses:?}");
      }
   }
}
