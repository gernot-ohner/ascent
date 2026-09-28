#[path = "../examples/openjdk/facts.rs"]
mod facts;
#[path = "../examples/openjdk/analysis.rs"]
mod analysis;

use std::collections::{BTreeMap, BTreeSet};

use facts::{Fact, Facts, Row};

fn fixture(rows: Vec<Row>) -> Facts {
   Facts {
      rows: rows.into_iter().enumerate().map(|(token, row)| Fact { token, row, line: token + 1 }).collect(),
      symbols: vec![],
   }
}

fn exhaustive(facts: &Facts) {
   let mut expected = BTreeMap::<_, BTreeSet<BTreeSet<usize>>>::new();
   for mask in 0..1_usize << facts.rows.len() {
      let chosen: Vec<_> =
         facts.rows.iter().enumerate().filter(|(i, _)| mask & (1 << i) != 0).map(|(_, f)| f).collect();
      let tokens: BTreeSet<_> = chosen.iter().map(|f| f.token).collect();
      for pair in facts::reference(chosen).pairs() {
         expected.entry(pair).or_default().insert(tokens.clone());
      }
   }
   for alternatives in expected.values_mut() {
      let all = alternatives.clone();
      alternatives.retain(|w| !all.iter().any(|other| other != w && other.is_subset(w)));
   }
   let (program, _) = analysis::boolean(facts);
   let actual = program.vpt.iter().map(|(x, y, p)| ((*x, *y), p.witnesses().clone())).collect::<BTreeMap<_, _>>();
   assert_eq!(actual, expected);
}

#[test]
fn field_sensitive_witnesses_match_every_input_world() {
   exhaustive(&fixture(vec![
      Row::Alloc(0, 1),
      Row::Alloc(0, 1), // Duplicate source fact, distinct tokens.
      Row::Store(0, 7, 2),
      Row::Load(3, 7, 1),
      Row::Load(4, 8, 1), // Different field must not join.
      Row::Assign(2, 3),  // Alternative direct explanation.
      Row::Assign(3, 3),
   ]));
   exhaustive(&fixture(vec![Row::Store(0, 7, 2), Row::Load(3, 7, 0)])); // No seed: no reflexive premise.
   exhaustive(&fixture(vec![]));
}

#[test]
fn real_processbuilder_component_matches_every_input_world() {
   let all = Facts::read(&facts::default_path()).unwrap();
   let seed = all
      .symbols
      .iter()
      .position(|s| s == "java.lang.ProcessBuilder.command([Ljava/lang/String;)Ljava/lang/ProcessBuilder;|%2")
      .unwrap() as u32;
   let component = all.component(seed);
   assert_eq!(component.counts(), [1, 2, 1, 1]);
   exhaustive(&component);
   assert!(analysis::check(&component).field_pairs > 0);
}

#[test]
fn openjdk_closed_subset_agrees_in_all_three_engines_and_replays_every_witness() {
   let all = Facts::read(&facts::default_path()).unwrap();
   assert_eq!(all.counts(), [4261, 39607, 2692, 509]);
   let subset = all.bounded(8);
   assert_eq!(subset.counts(), [957, 12194, 1601, 11]);
   let stats = analysis::check(&subset);
   assert!(stats.field_pairs > 0);
   assert!(stats.witnesses > stats.pairs);
   let vars: BTreeSet<_> = subset
      .rows
      .iter()
      .flat_map(|f| {
         let (a, b) = f.row.vars();
         [a, b]
      })
      .collect();
   let tokens: BTreeSet<_> = subset.rows.iter().map(|f| f.token).collect();
   for fact in &all.rows {
      let (a, b) = fact.row.vars();
      if vars.contains(&a) || vars.contains(&b) {
         assert!(tokens.contains(&fact.token));
      }
   }
}

#[test]
fn component_selection_is_order_independent_and_excludes_fields() {
   let mut data = fixture(vec![Row::Alloc(0, 1), Row::Load(1, 7, 2), Row::Store(3, 7, 4)]);
   let selected = data.bounded(1);
   assert_eq!(selected.rows.iter().map(|f| f.token).collect::<Vec<_>>(), vec![2]);
   data.rows.reverse();
   assert_eq!(data.bounded(1).rows.iter().map(|f| f.token).collect::<Vec<_>>(), vec![2]);
   assert_eq!(data.component(0).rows.len(), 2);
}

#[test]
fn tsv_load_column_order_and_bad_rows_are_checked() {
   let data = Facts::parse(["base\tobject\n", "", "result\t7\tbase\n", "base\t7\tstored\n"]).unwrap();
   assert_eq!(data.counts(), [1, 0, 1, 1]);
   assert!(data.rows.iter().all(|f| f.line == 1));
   let stats = analysis::check(&data);
   assert_eq!(stats.field_pairs, 4);
   for bad in ["result\tbase\t7\n", "result\t7\n", "result\t7\t\n"] {
      assert!(Facts::parse(["", "", bad, ""]).is_err());
   }
}
