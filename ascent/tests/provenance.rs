#![allow(clippy::field_reassign_with_default)]

use ascent::{HowProvenance, ProvenanceSemiring, WhyProvenance, ascent, ascent_run};

ascent! {
   #![provenance(HowProvenance<String>)]

   struct CopyProgram;

   relation input(i32);
   relation output(i32);

   output(x) <-- input(x);
}

#[test]
fn copy_rule_preserves_input_token() {
   let token = HowProvenance::token("input-7".to_owned());
   let mut program = CopyProgram::default();
   program.input = vec![(7, token.clone())];

   program.run();

   assert_eq!(program.output, vec![(7, token)]);
}

#[test]
fn ascent_run_provenance_normalizes_local_inputs() {
   let rows = vec![
      (1, HowProvenance::token("a".to_owned())),
      (1, HowProvenance::token("a".to_owned())),
      (2, HowProvenance::zero()),
   ];
   let program = ascent_run! {
      #![provenance(HowProvenance<String>)]
      relation input(i32) = rows;
      relation output(i32);
      output(x) <-- input(x);
   };
   assert_eq!(program.input.len(), 1);
   assert_eq!(program.input[0].0, 1);
   assert_eq!(program.input[0].1.to_string(), "2*a");
   assert_eq!(program.output, program.input);
}

mod reusable_source {
   ascent::ascent_source! {
      copy:
      relation included_input(i32);
      relation included_output(i32);
      included_output(x) <-- included_input(x);
   }
}

ascent! {
   #![provenance(HowProvenance<String>)]

   struct IncludedSourceProgram;

   include_source!(reusable_source::copy);
}

#[test]
fn existing_ascent_sources_run_unchanged_with_provenance() {
   let token = HowProvenance::token("included".to_owned());
   let mut program = IncludedSourceProgram::default();
   program.included_input = vec![(9, token.clone())];

   program.run();

   assert_eq!(program.included_output, vec![(9, token)]);
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NaturalProvenance(u64);

impl ProvenanceSemiring for NaturalProvenance {
   fn zero() -> Self { Self(0) }

   fn one() -> Self { Self(1) }

   fn add_assign(&mut self, other: &Self) -> bool {
      let previous = self.0;
      self.0 += other.0;
      self.0 != previous
   }

   fn multiply(&self, other: &Self) -> Self { Self(self.0 * other.0) }
}

ascent! {
   #![provenance(NaturalProvenance)]

   struct CustomSemiringProgram;

   relation left(i32);
   relation right(i32);
   relation fallback(i32);
   relation output(i32);

   output(x) <-- left(x), right(x);
   output(x) <-- fallback(x);
}

#[test]
fn macro_accepts_a_user_defined_semiring() {
   let mut program = CustomSemiringProgram::default();
   program.left = vec![(1, NaturalProvenance(2))];
   program.right = vec![(1, NaturalProvenance(3))];
   program.fallback = vec![(1, NaturalProvenance(5))];

   program.run();

   assert_eq!(program.output, vec![(1, NaturalProvenance(11))]);
}

ascent! {
   #![provenance(HowProvenance<String>)]

   struct DiamondProgram;

   relation edge(i32, i32);
   relation path(i32, i32);

   path(x, z) <-- edge(x, y), edge(y, z);
}

#[test]
fn diamond_join_multiplies_tokens_and_adds_alternative_derivations() {
   let mut program = DiamondProgram::default();
   program.edge = vec![
      (1, 2, HowProvenance::token("x1".to_owned())),
      (1, 4, HowProvenance::token("x2".to_owned())),
      (2, 3, HowProvenance::token("x3".to_owned())),
      (4, 3, HowProvenance::token("x4".to_owned())),
   ];

   program.run();

   let (_, _, provenance) = program.path.iter().find(|(from, to, _)| (*from, *to) == (1, 3)).unwrap();
   assert_eq!(provenance.to_string(), "x1*x3 + x2*x4");
}

#[test]
fn duplicate_inputs_are_coalesced_and_zero_inputs_are_discarded() {
   let mut program = CopyProgram::default();
   program.input = vec![
      (7, HowProvenance::token("x1".to_owned())),
      (7, HowProvenance::token("x1".to_owned())),
      (8, HowProvenance::zero()),
   ];

   program.run();

   assert_eq!(program.input.len(), 1);
   assert_eq!(program.input[0].0, 7);
   assert_eq!(program.input[0].1.to_string(), "2*x1");
   assert_eq!(program.output.len(), 1);
   assert_eq!(program.output[0].0, 7);
   assert_eq!(program.output[0].1.to_string(), "2*x1");
}

ascent! {
   #![provenance(HowProvenance<String>)]

   struct ProjectionProgram;

   relation source(i32, i32);
   relation projected(i32);

   projected(x) <-- source(x, _);
}

#[test]
fn projection_adds_annotations_from_distinct_source_tuples() {
   let mut program = ProjectionProgram::default();
   program.source =
      vec![(1, 10, HowProvenance::token("x1".to_owned())), (1, 20, HowProvenance::token("x2".to_owned()))];

   program.run();

   assert_eq!(program.projected.len(), 1);
   assert_eq!(program.projected[0].0, 1);
   assert_eq!(program.projected[0].1.to_string(), "x1 + x2");
}

ascent! {
   #![provenance(HowProvenance<String>)]

   struct AlternativeRulesProgram;

   relation left(i32);
   relation right(i32);
   relation output(i32);

   output(x) <-- left(x);
   output(x) <-- right(x);
}

#[test]
fn alternative_rules_add_their_derivations() {
   let mut program = AlternativeRulesProgram::default();
   program.left = vec![(1, HowProvenance::token("x1".to_owned()))];
   program.right = vec![(1, HowProvenance::token("x2".to_owned()))];

   program.run();

   assert_eq!(program.output.len(), 1);
   assert_eq!(program.output[0].1.to_string(), "x1 + x2");
}

ascent! {
   #![provenance(HowProvenance<String>)]

   struct DuplicateRulesProgram;

   relation input(i32);
   relation output(i32);

   output(x) <-- input(x);
   output(x) <-- input(x);
}

#[test]
fn duplicate_how_derivations_increase_coefficients() {
   let mut program = DuplicateRulesProgram::default();
   program.input = vec![(1, HowProvenance::token("x1".to_owned()))];

   program.run();

   assert_eq!(program.output.len(), 1);
   assert_eq!(program.output[0].1.to_string(), "2*x1");
}

ascent! {
   #![provenance(WhyProvenance<String>)]

   struct WhySemanticsProgram;

   relation input(i32);
   relation alternative(i32);
   relation output(i32);
   relation self_join(i32);

   output(x) <-- input(x);
   output(x) <-- input(x);
   output(x) <-- alternative(x);
   self_join(x) <-- input(x), input(x);
}

#[test]
fn why_provenance_discards_coefficients_and_exponents_but_keeps_alternatives() {
   let mut program = WhySemanticsProgram::default();
   program.input = vec![(1, WhyProvenance::token("x1".to_owned()))];
   program.alternative = vec![(1, WhyProvenance::token("x2".to_owned()))];

   program.run();

   assert_eq!(program.output.len(), 1);
   assert_eq!(program.output[0].1.to_string(), "{{x1}, {x2}}");
   assert_eq!(program.self_join.len(), 1);
   assert_eq!(program.self_join[0].1.to_string(), "{{x1}}");
}

ascent! {
   #![provenance(WhyProvenance<String>)]

   struct RecursiveWhyProgram;

   relation edge(i32, i32);
   relation path(i32, i32);

   path(x, y) <-- edge(x, y);
   path(x, z) <-- path(x, y), edge(y, z);
}

fn path_provenance(program: &RecursiveWhyProgram, from: i32, to: i32) -> String {
   program.path.iter().find(|row| (row.0, row.1) == (from, to)).unwrap().2.to_string()
}

#[test]
fn recursive_why_provenance_computes_acyclic_transitive_closure() {
   let mut program = RecursiveWhyProgram::default();
   program.edge = vec![
      (1, 2, WhyProvenance::token("a".to_owned())),
      (2, 3, WhyProvenance::token("b".to_owned())),
      (1, 3, WhyProvenance::token("c".to_owned())),
   ];

   program.run();

   assert_eq!(program.path.len(), 3);
   assert_eq!(path_provenance(&program, 1, 2), "{{a}}");
   assert_eq!(path_provenance(&program, 1, 3), "{{a,b}, {c}}");
   assert_eq!(path_provenance(&program, 2, 3), "{{b}}");
}

#[test]
fn recursive_why_provenance_terminates_on_a_cycle() {
   let mut program = RecursiveWhyProgram::default();
   program.edge = vec![(1, 2, WhyProvenance::token("a".to_owned())), (2, 1, WhyProvenance::token("b".to_owned()))];

   program.run();

   assert_eq!(program.path.len(), 4);
   assert_eq!(path_provenance(&program, 1, 1), "{{a,b}}");
   assert_eq!(path_provenance(&program, 1, 2), "{{a}, {a,b}}");
   assert_eq!(path_provenance(&program, 2, 1), "{{a,b}, {b}}");
   assert_eq!(path_provenance(&program, 2, 2), "{{a,b}}");
}

ascent! {
   #![provenance(HowProvenance<String>)]

   struct ClauseFeaturesProgram;

   relation input(i32, Option<i32>);
   relation auxiliary(i32);
   relation head_a(i32);
   relation head_b(i32);
   relation patterned(i32);
   relation transformed(i32);
   relation disjoined(i32);
   relation via_macro(i32);

   macro record($value: expr) {
      via_macro($value)
   }

   { head_a(x), head_b(x) } <-- input(x, _);
   patterned(y) <-- input(_, ?Some(y));
   transformed(z) <-- input(x, _), if *x > 0, let y = *x + 1, for z in [y * 2];
   disjoined(x) <-- (head_a(x) | auxiliary(x));
   record!(*x) <-- input(x, _);
}

#[test]
fn rust_clauses_patterns_disjunction_and_multi_head_preserve_provenance() {
   let mut program = ClauseFeaturesProgram::default();
   program.input = vec![(3, Some(9), HowProvenance::token("p".to_owned()))];
   program.auxiliary = vec![(4, HowProvenance::token("q".to_owned()))];

   program.run();

   assert_eq!(program.head_a[0].0, 3);
   assert_eq!(program.head_a[0].1.to_string(), "p");
   assert_eq!(program.head_b[0].0, 3);
   assert_eq!(program.head_b[0].1.to_string(), "p");
   assert_eq!(program.patterned[0].0, 9);
   assert_eq!(program.patterned[0].1.to_string(), "p");
   assert_eq!(program.transformed[0].0, 8);
   assert_eq!(program.transformed[0].1.to_string(), "p");
   assert_eq!(program.via_macro[0].0, 3);
   assert_eq!(program.via_macro[0].1.to_string(), "p");
   assert_eq!(program.disjoined.len(), 2);
   assert!(program.disjoined.iter().any(|row| row.0 == 3 && row.1.to_string() == "p"));
   assert!(program.disjoined.iter().any(|row| row.0 == 4 && row.1.to_string() == "q"));
}

#[test]
#[should_panic(expected = "`run()` may only be called once")]
fn provenance_program_rejects_a_second_run() {
   let mut program = CopyProgram::default();
   program.input = vec![(7, HowProvenance::token("x".to_owned()))];
   program.run();
   program.run();
}
