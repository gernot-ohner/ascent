//! Run through `python3 ascent/examples/provsql/compare.py`.
use std::io::{self, BufRead};

use ascent::{BooleanProvenance, WhyProvenance, ascent};

#[path = "why_provenance_shortest_path.rs"]
mod shortest;

// Instantiate the same rules with either annotation algebra.
macro_rules! examples {
   ($module:ident, $annotation:ident, $($mode:tt)*) => {
      mod $module {
         use super::*;
         ascent! {
            $($mode)*
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
            relation recursive_enabled(bool);
            relation destination(u32);
            #[provenance(u32)] relation reach(u32, u32);
            #[provenance(u32)] relation suffix(u32, u32);
            #[provenance(u32)] relation identity(u32, u32);

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
            reach(x, y) <-- edge(x, y), recursive_enabled(true);
            reach(x, z) <-- reach(x, y), edge(y, z);
            identity(x, x) <-- destination(x);
            suffix(x, x) <-- destination(x), recursive_enabled(true);
            suffix(x, z) <-- edge(x, y), suffix(y, z);
         }

         pub fn run(data: &str, recursive: bool, destination: u32) {
            let mut program = Examples::default();
            program.destination = vec![(destination,)];
            if recursive { program.recursive_enabled.push((true,)); }
            for line in data.lines() {
               let values: Vec<u32> = line.split_whitespace().map(|value| value.parse().unwrap()).collect();
               assert_eq!(values.len(), 3, "expected source, target, token");
               program.edge.push((values[0], values[1], $annotation::token(values[2])));
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
               ("reach", program.reach),
               ("suffix", program.suffix),
               ("identity", program.identity),
            ] {
               println!("@{name}");
               for (source, target, why) in rows {
                  let witnesses: Vec<Vec<u32>> = why.witnesses().iter().map(|set| set.iter().copied().collect()).collect();
                  println!("{source}\t{target}\t{witnesses:?}");
               }
            }
         }
      }
   };
}

examples!(why, WhyProvenance,);
examples!(boolean, BooleanProvenance, #![provenance(boolean)]);

fn main() {
   let args: Vec<String> = std::env::args().skip(1).collect();
   assert_eq!(args.len(), 3, "expected mode, recursive flag, destination");
   let data = io::stdin().lock().lines().map(Result::unwrap).collect::<Vec<_>>().join("\n");
   let destination: u32 = args[2].parse().unwrap();
   match args[0].as_str() {
      "why" => why::run(&data, args[1] == "true", destination),
      "boolean" => boolean::run(&data, args[1] == "true", destination),
      "shortest" => {
         // The demo's public interface uses static string labels. These small
         // process-local fixtures live until this invocation exits.
         let intern = |value: u32| -> &'static str { Box::leak(value.to_string().into_boxed_str()) };
         let destination = intern(destination);
         let mut nodes = std::collections::BTreeSet::from([destination, "99"]);
         let edges: Vec<_> = data
            .lines()
            .map(|line| {
               let v: Vec<u32> = line.split_whitespace().map(|v| v.parse().unwrap()).collect();
               assert_eq!(v.len(), 4, "expected source, target, weight, token");
               let (source, target) = (intern(v[0]), intern(v[1]));
               nodes.extend([source, target]);
               (source, target, v[2], intern(v[3]))
            })
            .collect();
         println!("@shortest");
         for source in nodes {
            if let Some((distance, witnesses)) = shortest::shortest_path_with_why(&edges, source, destination) {
               let witnesses: Vec<Vec<u32>> =
                  witnesses.iter().map(|w| w.iter().map(|token| token.parse().unwrap()).collect()).collect();
               println!("{source}\t{distance}\t{witnesses:?}");
            }
         }
      },
      _ => panic!("unknown mode"),
   }
}
