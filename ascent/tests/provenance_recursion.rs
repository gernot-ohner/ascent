// The generated timeout evaluator clones generic tuple fields, including i32.
#![allow(clippy::clone_on_copy)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::time::Duration;

use ascent::{WhyProvenance, ascent_provenance};

type Edge = (i32, i32, String);
type Row = (i32, i32, WhyProvenance<String>);
type Witnesses = BTreeMap<(i32, i32), BTreeSet<BTreeSet<String>>>;

// Enumerate nonempty walks, remembering (endpoint, used tokens) for each start.
// Revisiting that state cannot produce new witnesses. This oracle uses neither
// Ascent nor the production semiring operations or recursive join scheduler.
fn walk_witnesses(edges: &[Edge]) -> Witnesses {
   let mut result = Witnesses::new();
   let starts: BTreeSet<_> = edges.iter().map(|edge| edge.0).collect();
   for start in starts {
      let mut queue: VecDeque<_> =
         edges.iter().filter(|edge| edge.0 == start).map(|edge| (edge.1, BTreeSet::from([edge.2.clone()]))).collect();
      let mut seen = BTreeSet::new();
      while let Some((node, witness)) = queue.pop_front() {
         if !seen.insert((node, witness.clone())) {
            continue;
         }
         result.entry((start, node)).or_default().insert(witness.clone());
         for (_, next, token) in edges.iter().filter(|edge| edge.0 == node) {
            let mut extended = witness.clone();
            extended.insert(token.clone());
            queue.push_back((*next, extended));
         }
      }
   }
   result
}

fn graph_cases() -> Vec<Vec<Edge>> {
   let mut graphs = Vec::new();
   // Every directed graph on two nodes (including self-loops), and every
   // directed graph on three nodes without self-loops.
   for (nodes, loops) in [(2, true), (3, false)] {
      let slots: Vec<_> =
         (0..nodes).flat_map(|a| (0..nodes).filter(move |&b| loops || a != b).map(move |b| (a, b))).collect();
      for mask in 0..(1usize << slots.len()) {
         graphs.push(
            slots
               .iter()
               .enumerate()
               .filter(|(i, _)| mask & (1 << i) != 0)
               .map(|(i, &(a, b))| (a, b, format!("e{i}")))
               .collect(),
         );
      }
   }
   for edges in [
      vec![(0, 1, "a"), (0, 2, "b"), (1, 3, "c"), (2, 3, "d")],
      vec![(0, 1, "a"), (0, 2, "b"), (1, 3, "c"), (2, 3, "d"), (3, 0, "e")],
      vec![(0, 1, "a"), (0, 1, "b"), (1, 0, "c"), (1, 1, "d")],
   ] {
      graphs.push(edges.into_iter().map(|(a, b, token)| (a, b, token.to_owned())).collect());
   }
   graphs
}

fn check_graphs(evaluate: impl Fn(Vec<Row>) -> Vec<Vec<Row>>) {
   for mut edges in graph_cases() {
      let expected = walk_witnesses(&edges);
      for _ in 0..2 {
         let input = edges.iter().map(|(a, b, token)| (*a, *b, WhyProvenance::token(token.clone()))).collect();
         for rows in evaluate(input) {
            let actual: Witnesses = rows.iter().map(|(a, b, p)| ((*a, *b), p.witnesses().clone())).collect();
            assert_eq!(rows.len(), actual.len(), "duplicate logical rows for {edges:?}");
            assert_eq!(actual, expected, "wrong witnesses for {edges:?}");
         }
         edges.reverse();
      }
   }
}

ascent_provenance! {
   semiring WhyProvenance<String>;
   #![generate_run_timeout]
   struct Nonlinear;
   relation edge(i32, i32);
   relation path(i32, i32);
   relation output(i32, i32);

   path(x, y) <-- edge(x, y);
   path(x, z) <-- path(x, y), path(y, z);
   output(x, y) <-- path(x, y);
}

#[test]
fn nonlinear_recursion_matches_walk_reference() {
   check_graphs(|edge| {
      let mut program = Nonlinear { edge, ..Default::default() };
      assert!(program.run_timeout(Duration::from_secs(5)), "nonlinear recursion did not converge");
      vec![program.path, program.output]
   });
}

ascent_provenance! {
   semiring WhyProvenance<String>;
   #![generate_run_timeout]
   struct Mutual;
   relation edge(i32, i32);
   relation left(i32, i32);
   relation right(i32, i32);
   relation output(i32, i32);

   left(x, y) <-- edge(x, y);
   left(x, z) <-- right(x, y), edge(y, z);
   right(x, y) <-- left(x, y);
   output(x, y) <-- right(x, y);
}

#[test]
fn mutual_recursion_matches_walk_reference() {
   check_graphs(|edge| {
      let mut program = Mutual { edge, ..Default::default() };
      assert!(program.run_timeout(Duration::from_secs(5)), "mutual recursion did not converge");
      vec![program.left, program.right, program.output]
   });
}
