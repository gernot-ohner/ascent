//! Track alternative explanations for recursive reachability.

use std::collections::BTreeSet;

use ascent_provenance::{WhyProvenance, provenance};

type Node = &'static str;
type FactId = &'static str;

provenance! {
   #[provenance(FactId)] relation edge(Node, Node);
   #[provenance(FactId)] relation reachable(Node, Node);

   reachable(x, y) <-- edge(x, y);
   reachable(x, z) <-- edge(x, y), reachable(y, z);
}

fn main() {
   let mut program = AscentProgram::default();
   program.edge = vec![
      ("A", "B", WhyProvenance::token("ab")),
      ("B", "D", WhyProvenance::token("bd")),
      ("A", "C", WhyProvenance::token("ac")),
      ("C", "D", WhyProvenance::token("cd")),
   ];
   program.run();

   let explanations = program
      .reachable
      .iter()
      .find(|(source, target, _)| *source == "A" && *target == "D")
      .map(|(_, _, provenance)| provenance.witnesses())
      .unwrap();
   assert_eq!(explanations, &BTreeSet::from([BTreeSet::from(["ab", "bd"]), BTreeSet::from(["ac", "cd"])]));
   println!("A reaches D because of {explanations:?}");
}
