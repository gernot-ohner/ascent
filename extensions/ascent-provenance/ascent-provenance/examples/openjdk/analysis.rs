use std::collections::{BTreeSet, HashMap};
use std::time::{Duration, Instant};

use ascent::ascent;
use ascent::internal::RelIndexRead;
use ascent_byods_rels::eqrel;
use ascent_provenance::{BooleanProvenance as B, provenance};

use super::facts::{Facts, Id, Row, reference};

ascent! {
   pub struct Compact;
   relation alloc(Id,Id); relation assign(Id,Id);
   relation load(Id,u32,Id); relation store(Id,u32,Id);
   #[ds(eqrel)] relation vpt(Id,Id);
   vpt(x,y) <-- alloc(x,y);
   vpt(x,y) <-- assign(x,y);
   vpt(y,p) <-- store(x,f,y), load(p,f,q), vpt(x,q);
}
impl Compact {
   pub fn pair_count(&self) -> usize { self.__vpt_ind_common.count_exact() }
}

ascent! {
   pub struct Explicit;
   relation alloc(Id,Id); relation assign(Id,Id);
   relation load(Id,u32,Id); relation store(Id,u32,Id);
   relation vpt(Id,Id);
   vpt(x,y) <-- alloc(x,y);
   vpt(x,y) <-- assign(x,y);
   vpt(y,p) <-- store(x,f,y), load(p,f,q), vpt(x,q);
   vpt(y,x), vpt(x,x) <-- vpt(x,y);
   vpt(x,z) <-- vpt(x,y), vpt(y,z);
}
provenance! {
   #![provenance(boolean)]
   pub struct Explained;
   #[provenance(usize)] relation alloc(Id,Id);
   #[provenance(usize)] relation assign(Id,Id);
   #[provenance(usize)] relation load(Id,u32,Id);
   #[provenance(usize)] relation store(Id,u32,Id);
   #[provenance(usize)] relation vpt(Id,Id);
   vpt(x,y) <-- alloc(x,y);
   vpt(x,y) <-- assign(x,y);
   vpt(y,p) <-- store(x,f,y), load(p,f,q), vpt(x,q);
   vpt(y,x), vpt(x,x) <-- vpt(x,y);
   vpt(x,z) <-- vpt(x,y), vpt(y,z);
}

// Identical loading for each engine; only the last column differs.
macro_rules! runner {
   ($name:ident, $program:ty $(, $annotation:ident)?) => {
      pub fn $name(facts: &Facts) -> ($program, Duration) {
         let mut p = <$program>::default();
         for f in &facts.rows {
            match f.row {
               Row::Alloc(a,b) => p.alloc.push((a,b $(, $annotation::token(f.token))?)),
               Row::Assign(a,b) => p.assign.push((a,b $(, $annotation::token(f.token))?)),
               Row::Load(a,fld,b) => p.load.push((a,fld,b $(, $annotation::token(f.token))?)),
               Row::Store(a,fld,b) => p.store.push((a,fld,b $(, $annotation::token(f.token))?)),
            }
         }
         let start = Instant::now(); p.run(); let elapsed = start.elapsed();
         (p,elapsed)
      }
   };
}
runner!(compact, Compact);
runner!(explicit, Explicit);
runner!(boolean, Explained, B);

#[derive(Debug, PartialEq, Eq)]
pub struct Stats {
   pub pairs: usize,
   pub witnesses: usize,
   pub max_witnesses: usize,
   pub field_pairs: usize,
}

pub fn compact_check(facts: &Facts, p: &Compact) {
   let expected = reference(&facts.rows);
   assert_eq!(p.pair_count(), expected.count());
   // Count plus every member-to-root edge identifies the complete partition;
   // this avoids enumerating hundreds of millions of full-data pairs.
   for (root, members) in expected.classes() {
      for member in members {
         assert!(p.__vpt_ind_common.index_get(&(member, root)).is_some());
      }
   }
}

pub fn check_mode(facts: &Facts, mode: &str) -> Stats {
   let expected = reference(&facts.rows);
   let seeds = reference(facts.rows.iter().filter(|f| f.row.kind() < 2)).count();
   let mut stats =
      Stats { pairs: expected.count(), witnesses: 0, max_witnesses: 0, field_pairs: expected.count() - seeds };
   eprintln!("running {mode}: {} facts, {} expected pairs", facts.rows.len(), stats.pairs);
   match mode {
      "eqrel" => {
         let (p, elapsed) = compact(facts);
         eprintln!("eqrel evaluation finished: {elapsed:?}; checking partition");
         compact_check(facts, &p);
      },
      "explicit" => {
         let (p, elapsed) = explicit(facts);
         eprintln!("explicit evaluation finished: {elapsed:?}; checking pairs");
         assert_eq!(p.vpt.iter().copied().collect::<BTreeSet<_>>(), expected.pairs());
      },
      "boolean" => {
         let (annotated, elapsed) = boolean(facts);
         eprintln!("boolean evaluation finished: {elapsed:?}; checking pairs and replaying witnesses");
         assert_eq!(annotated.vpt.iter().map(|(a, b, _)| (*a, *b)).collect::<BTreeSet<_>>(), expected.pairs());
         let by_token: HashMap<_, _> = facts.rows.iter().map(|f| (f.token, f)).collect();
         let mut witnesses = 0;
         let mut max_witnesses = 0;
         for (a, b, value) in &annotated.vpt {
            max_witnesses = max_witnesses.max(value.witnesses().len());
            for witness in value.witnesses() {
               let rows: Vec<_> = witness.iter().map(|t| by_token[t]).collect();
               let support = reference(rows.iter().copied());
               assert!(
                  support.root(*a).is_some() && support.root(*a) == support.root(*b),
                  "insufficient witness for ({a},{b}): {witness:?}"
               );
               for removed in witness {
                  let smaller = reference(rows.iter().copied().filter(|f| f.token != *removed));
                  assert!(
                     smaller.root(*a).is_none() || smaller.root(*a) != smaller.root(*b),
                     "non-minimal witness for ({a},{b}): {witness:?}"
                  );
               }
               witnesses += 1;
            }
         }
         stats.witnesses = witnesses;
         stats.max_witnesses = max_witnesses;
      },
      _ => panic!("mode must be eqrel, explicit or boolean"),
   }
   stats
}

pub fn check(facts: &Facts) -> Stats {
   let compact = check_mode(facts, "eqrel");
   let plain = check_mode(facts, "explicit");
   let boolean = check_mode(facts, "boolean");
   assert_eq!(compact.pairs, plain.pairs);
   assert_eq!(plain.pairs, boolean.pairs);
   boolean
}
