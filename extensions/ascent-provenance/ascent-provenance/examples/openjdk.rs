//! Three-way OpenJDK comparison. See ../../benchmarks/OPENJDK.md.
#[path = "openjdk/facts.rs"]
mod facts;
#[path = "openjdk/analysis.rs"]
mod analysis;

use std::hint::black_box;
use std::path::Path;
use std::time::Duration;

use analysis::{Stats, boolean, compact, compact_check, explicit};
use facts::{FILES, Facts};

fn measure(mode: &str, input: &Facts) -> (Duration, Stats) {
   let (time, pairs, witnesses, max_witnesses) = match mode {
      "eqrel" => {
         let (p, time) = compact(input);
         let pairs = p.pair_count();
         black_box(&p);
         (time, pairs, 0, 0)
      },
      "explicit" => {
         let (p, time) = explicit(input);
         let pairs = p.vpt.len();
         black_box(&p);
         (time, pairs, 0, 0)
      },
      "boolean" => {
         let (p, time) = boolean(input);
         let witnesses = p.vpt.iter().map(|r| r.2.witnesses().len()).sum();
         let max = p.vpt.iter().map(|r| r.2.witnesses().len()).max().unwrap_or(0);
         let pairs = p.vpt.len();
         black_box(&p);
         (time, pairs, witnesses, max)
      },
      _ => panic!("mode must be eqrel, explicit or boolean"),
   };
   (time, Stats { pairs, witnesses, max_witnesses, field_pairs: 0 })
}

fn main() {
   let args: Vec<_> = std::env::args().collect();
   if args.get(1).map(String::as_str) == Some("explain") {
      let input = Facts::read(&facts::default_path()).unwrap();
      let seed = input
         .symbols
         .iter()
         .position(|s| s == "java.lang.ProcessBuilder.command([Ljava/lang/String;)Ljava/lang/ProcessBuilder;|%2")
         .unwrap() as u32;
      let small = input.component(seed);
      let (p, _) = boolean(&small);
      let (a, b, value) = p
         .vpt
         .iter()
         .find(|(a, b, p)| {
            a != b
               && p
                  .witnesses()
                  .iter()
                  .any(|w| w.iter().any(|t| small.rows.iter().any(|f| f.token == *t && f.row.kind() == 2)))
         })
         .unwrap();
      println!("{} aliases {}", small.symbols[*a as usize], small.symbols[*b as usize]);
      for witness in value.witnesses() {
         println!("Witness:");
         for token in witness {
            let f = small.rows.iter().find(|f| f.token == *token).unwrap();
            println!("  {}.facts:{} (token {})", FILES[f.row.kind()], f.line, f.token);
         }
      }
      return;
   }
   assert!(
      args.len() >= 4,
      "usage: openjdk explain | check <facts-dir> <cap> | bench <facts-dir> <cap> <eqrel|explicit|boolean> <samples>; cap 0 is full-data eqrel only"
   );
   let cap: usize = args[3].parse().unwrap();
   let input = Facts::read(Path::new(&args[2])).unwrap().bounded(cap);
   let [alloc, assign, load, store] = input.counts();
   if args[1] == "check" {
      assert_eq!(args.len(), 4);
      let stats = if cap == 0 {
         let (p, _) = compact(&input);
         compact_check(&input, &p);
         Stats { pairs: p.pair_count(), witnesses: 0, max_witnesses: 0, field_pairs: 0 }
      } else {
         analysis::check(&input)
      };
      println!("cap,facts,alloc,assign,load,store,pairs,witnesses,max_witnesses,field_pairs");
      println!(
         "{cap},{},{alloc},{assign},{load},{store},{},{},{},{}",
         input.rows.len(),
         stats.pairs,
         stats.witnesses,
         stats.max_witnesses,
         stats.field_pairs
      );
   } else {
      assert!(args[1] == "bench" && args.len() == 6);
      let mode = &args[4];
      let samples: usize = args[5].parse().unwrap();
      assert!((1..=25).contains(&samples));
      assert!(
         cap != 0 || mode == "eqrel",
         "full-data explicit/Boolean execution is intentionally disabled; select a positive cap"
      );
      let (_, warmup) = measure(mode, &input);
      println!("mode,cap,facts,alloc,assign,load,store,pairs,witnesses,max_witnesses,sample,elapsed_ns");
      for sample in 1..=samples {
         let (elapsed, stats) = measure(mode, &input);
         assert_eq!(stats, warmup);
         println!(
            "{mode},{cap},{},{alloc},{assign},{load},{store},{},{},{},{sample},{}",
            input.rows.len(),
            stats.pairs,
            stats.witnesses,
            stats.max_witnesses,
            elapsed.as_nanos()
         );
      }
   }
}
