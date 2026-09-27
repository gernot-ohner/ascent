//! Reproducible, bounded scaling probe. See benchmarks/README.md.
use std::hint::black_box;
use std::time::{Duration, Instant};

use ascent_provenance::{BooleanProvenance as B, WhyProvenance as W, provenance};

provenance! {
    struct Inputs;
    #[provenance(usize)] relation input(usize);
}
provenance! {
    struct WhyPaths;
    #[provenance(usize)] relation edge(usize, usize);
    #[provenance(usize)] relation path(usize);
    path(y) <-- path(x), edge(x, y);
}
provenance! {
    #![provenance(boolean)]
    struct BooleanPaths;
    #[provenance(usize)] relation edge(usize, usize);
    #[provenance(usize)] relation path(usize);
    path(y) <-- path(x), edge(x, y);
}

fn measure(case: &str, size: usize) -> (Duration, usize, usize) {
   match case {
      "inputs" => {
         let mut p = Inputs::default();
         p.input = (0..size).map(|i| (i, W::token(i))).collect();
         let start = Instant::now();
         p.run();
         let elapsed = start.elapsed();
         assert_eq!(p.input.len(), size);
         assert!(p.input.iter().enumerate().all(|(i, row)| row == &(i, W::token(i))));
         black_box(&p);
         (elapsed, p.input.len(), size)
      },
      "why" => {
         let mut p = WhyPaths::default();
         p.path = vec![(0, W::token(usize::MAX))];
         p.edge = (0..size).flat_map(|i| [(i, i + 1, W::token(2 * i)), (i, i + 1, W::token(2 * i + 1))]).collect();
         let start = Instant::now();
         p.run();
         let elapsed = start.elapsed();
         let witnesses = p.path.iter().find(|row| row.0 == size).unwrap().1.witnesses();
         assert_eq!(witnesses.len(), 1 << size);
         assert!(witnesses.iter().all(|w| w.len() == size + 1));
         assert_eq!(p.path.len(), size + 1);
         black_box(&p);
         (elapsed, p.path.len(), witnesses.len())
      },
      "boolean" => {
         let mut p = BooleanPaths::default();
         p.path = vec![(0, B::token(usize::MAX))];
         p.edge = (0..size).flat_map(|i| [(i, i + 1, B::token(2 * i)), (i, i + 1, B::token(2 * i + 1))]).collect();
         let start = Instant::now();
         p.run();
         let elapsed = start.elapsed();
         let witnesses = p.path.iter().find(|row| row.0 == size).unwrap().1.witnesses();
         assert_eq!(witnesses.len(), 1 << size);
         assert!(witnesses.iter().all(|w| w.len() == size + 1));
         assert_eq!(p.path.len(), size + 1);
         black_box(&p);
         (elapsed, p.path.len(), witnesses.len())
      },
      _ => panic!("case must be inputs, why or boolean"),
   }
}

fn main() {
   let args: Vec<_> = std::env::args().collect();
   assert_eq!(args.len(), 4, "usage: scaling <inputs|why|boolean> <size> <samples>");
   let case = &args[1];
   let size: usize = args[2].parse().unwrap();
   let samples: usize = args[3].parse().unwrap();
   assert!((1..=25).contains(&samples));
   assert!(size > 0 && if case == "inputs" { size <= 1_000_000 } else { size <= 16 });
   measure(case, size); // Warmup is excluded from reported execution times.
   let mut durations = Vec::with_capacity(samples);
   let mut counts = (0, 0);
   for _ in 0..samples {
      let (duration, tuples, witnesses) = measure(case, size);
      durations.push(duration.as_nanos());
      counts = (tuples, witnesses);
   }
   durations.sort_unstable();
   println!(
      "{case},{size},{samples},{},{},{},{},{}",
      counts.0,
      counts.1,
      durations[samples / 2],
      durations[0],
      durations[samples - 1]
   );
}
