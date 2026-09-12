use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::hash::{DefaultHasher, Hash, Hasher};

use ascent::{Lattice, WhyProvenance, ascent, ascent_run};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct NonDefaultToken(&'static str);

mod provenance_source {
   ascent::ascent_source! { rules:
      #[provenance(&'static str)] relation source_input(i32);
      #[provenance(&'static str)] relation source_output(i32);
      source_output(x) <-- source_input(x);
   }
}

#[test]
fn witness_domain_distinguishes_zero_and_one_and_uses_subset_order() {
   let zero = WhyProvenance::<&str>::default();
   let one = ascent::internal::why_provenance_one();
   let a = WhyProvenance::token("a");
   let b = WhyProvenance::token("b");
   let alternatives = a.clone().join(b.clone());

   assert!(zero.witnesses().is_empty());
   assert_eq!(one.witnesses(), &BTreeSet::from([BTreeSet::new()]));
   assert_ne!(zero, one);
   assert_eq!(a.partial_cmp(&alternatives), Some(Ordering::Less));
   assert_eq!(a.partial_cmp(&b), None);
}

#[test]
fn token_type_does_not_need_to_implement_default() {
   let zero = WhyProvenance::<NonDefaultToken>::default();
   let token = WhyProvenance::token(NonDefaultToken("token"));

   assert!(zero.witnesses().is_empty());
   assert_eq!(token.witnesses(), &BTreeSet::from([BTreeSet::from([NonDefaultToken("token")])]));
}

#[test]
fn witness_domain_lattice_and_product_keep_all_explanations() {
   let a = WhyProvenance::token("a");
   let b = WhyProvenance::token("b");
   let c = WhyProvenance::token("c");
   let left = a.clone().join(b.clone());
   let right = a.clone().join(c.clone());

   assert_eq!(left.clone().meet(right.clone()), a);
   assert_eq!(left.clone().join(right.clone()).witnesses().len(), 3);
   assert_eq!(
      ascent::internal::why_provenance_product(&left, &right).witnesses(),
      &BTreeSet::from([
         BTreeSet::from(["a"]),
         BTreeSet::from(["a", "b"]),
         BTreeSet::from(["a", "c"]),
         BTreeSet::from(["b", "c"]),
      ])
   );
}

#[test]
fn witness_domain_obeys_lattice_and_product_laws() {
   let zero = WhyProvenance::<&str>::default();
   let one = ascent::internal::why_provenance_one();
   let a = WhyProvenance::token("a");
   let b = WhyProvenance::token("b");
   let c = WhyProvenance::token("c");
   let alternatives = a.clone().join(b.clone());

   assert_eq!(a.clone().join(a.clone()), a);
   assert_eq!(a.clone().join(b.clone()), b.clone().join(a.clone()));
   assert_eq!(alternatives.clone().meet(a.clone()), a);
   assert_eq!(ascent::internal::why_provenance_product(&a, &one), a);
   assert_eq!(ascent::internal::why_provenance_product(&alternatives, &zero), zero);
   assert_eq!(
      ascent::internal::why_provenance_product(&alternatives, &c),
      ascent::internal::why_provenance_product(&a, &c).join(ascent::internal::why_provenance_product(&b, &c))
   );

   let mut left_hasher = DefaultHasher::new();
   let mut right_hasher = DefaultHasher::new();
   alternatives.hash(&mut left_hasher);
   b.clone().join(a).hash(&mut right_hasher);
   assert_eq!(left_hasher.finish(), right_hasher.finish());
}

#[test]
fn annotated_copy_normalizes_inputs_and_reruns_idempotently() {
   ascent! {
       #[provenance(&'static str)] relation input(i32);
       #[provenance(&'static str)] relation output(i32);
       output(x) <-- input(x);
   }

   let mut program = AscentProgram::default();
   program.input = vec![(1, WhyProvenance::token("a")), (1, WhyProvenance::token("b")), (2, WhyProvenance::default())];
   program.run();

   assert_eq!(program.input.len(), 1);
   assert_eq!(program.output.len(), 1);
   assert_eq!(program.output[0].0, 1);
   assert_eq!(program.output[0].1.witnesses(), &BTreeSet::from([BTreeSet::from(["a"]), BTreeSet::from(["b"])]));

   let before = program.output.clone();
   program.run();
   assert_eq!(program.output, before);
}

#[test]
fn named_program_default_normalizes_annotated_initializers() {
   ascent! {
      struct Initialized;
      #[provenance(&'static str)] relation input(i32) = vec![
         (1, WhyProvenance::token("a")),
         (1, WhyProvenance::token("b")),
         (2, WhyProvenance::default()),
      ];
      #[provenance(&'static str)] relation output(i32);

      output(x) <-- input(x);
   }

   let mut program = Initialized::default();
   assert_eq!(program.input.len(), 1);
   assert_eq!(program.input[0].0, 1);
   assert_eq!(program.input[0].1.witnesses(), &BTreeSet::from([BTreeSet::from(["a"]), BTreeSet::from(["b"])]));

   program.run();
   assert_eq!(program.output.len(), 1);
   assert_eq!(program.output[0].0, 1);
   assert_eq!(program.output[0].1.witnesses(), &BTreeSet::from([BTreeSet::from(["a"]), BTreeSet::from(["b"])]));
}

#[test]
fn inline_execution_supports_identity_and_join_products() {
   let result = ascent_run! {
      #[provenance(&'static str)] relation left(i32) = vec![(1, WhyProvenance::token("l"))];
      #[provenance(&'static str)] relation right(i32) = vec![(1, WhyProvenance::token("r"))];
      relation enabled(i32) = vec![(1,)];
      #[provenance(&'static str)] relation joined(i32);
      #[provenance(&'static str)] relation background(i32);

      joined(x) <-- left(x), right(x), enabled(x);
      background(x) <-- enabled(x);
   };

   assert_eq!(result.joined[0].1.witnesses(), &BTreeSet::from([BTreeSet::from(["l", "r"])]));
   assert_eq!(result.background[0].1.witnesses(), &BTreeSet::from([BTreeSet::new()]));
}

#[test]
fn lowering_handles_disjunction_macros_multi_head_and_disconnected_token_types() {
   ascent! {
      macro take($x: ident) { left($x) }

      #[provenance(&'static str)] relation left(i32);
      #[provenance(&'static str)] relation right(i32);
      #[provenance(&'static str)] relation either(i32);
      #[provenance(&'static str)] relation from_macro(i32);
      #[provenance(u32)] relation numbers(i32);
      #[provenance(u32)] relation numbers_copy(i32);
      relation ordinary(i32);

      either(x), ordinary(x) <-- (left(x) | right(x));
      from_macro(x) <-- take!(x);
      numbers_copy(x) <-- numbers(x);
   }

   let mut program = AscentProgram::default();
   program.left = vec![(1, WhyProvenance::token("a")), (2, WhyProvenance::token("b"))];
   program.right = vec![(1, WhyProvenance::token("c")), (3, WhyProvenance::token("d"))];
   program.numbers = vec![(9, WhyProvenance::token(42))];
   program.run();

   program.either.sort_by_key(|row| row.0);
   assert_eq!(program.either.iter().map(|row| row.0).collect::<Vec<_>>(), vec![1, 2, 3]);
   assert_eq!(program.either[0].1.witnesses(), &BTreeSet::from([BTreeSet::from(["a"]), BTreeSet::from(["c"])]));
   assert_eq!(program.ordinary, vec![(1,), (2,), (3,)]);
   assert_eq!(program.from_macro.len(), 2);
   assert_eq!(program.numbers_copy[0].1.witnesses(), &BTreeSet::from([BTreeSet::from([42])]));
}

#[test]
fn lowering_handles_patterns_conditions_bindings_generators_and_zero_arity() {
   ascent! {
      #[provenance(&'static str)] relation maybe(Option<i32>);
      #[provenance(&'static str)] relation expanded(i32);
      #[provenance(&'static str)] relation seed();
      #[provenance(&'static str)] relation ready();

      expanded(y) <-- maybe(?Some(x)), let start = *x, for y in start..start + 3, if y % 2 == 0;
      ready() <-- seed();
   }

   let mut program = AscentProgram::default();
   program.maybe = vec![(Some(2), WhyProvenance::token("m"))];
   program.seed = vec![(WhyProvenance::token("s"),)];
   program.run();

   program.expanded.sort_by_key(|row| row.0);
   assert_eq!(program.expanded.iter().map(|row| row.0).collect::<Vec<_>>(), vec![2, 4]);
   assert!(program.expanded.iter().all(|row| row.1 == WhyProvenance::token("m")));
   assert_eq!(program.ready, vec![(WhyProvenance::token("s"),)]);
}

#[test]
fn generated_annotation_names_do_not_capture_user_variables() {
   let result = ascent_run! {
      #[provenance(&'static str)] relation input(i32) = vec![(5, WhyProvenance::token("five"))];
      #[provenance(&'static str)] relation output(i32);

      output(*__ascent_provenance_0) <-- input(__ascent_provenance_0);
   };

   assert_eq!(result.output, vec![(5, WhyProvenance::token("five"))]);
}

#[test]
fn generated_annotation_names_do_not_capture_attached_let_bindings() {
   let mut result = ascent_run! {
      relation ordinary(i32) = vec![(-1,), (1,), (2,)];
      #[provenance(&'static str)] relation tagged(i32) = vec![
         (-1, WhyProvenance::token("negative")),
         (1, WhyProvenance::token("one")),
         (2, WhyProvenance::token("two")),
      ];
      #[provenance(&'static str)] relation output(i32);

      output(x) <-- ordinary(x) let __ascent_provenance_0 = 1_i32,
         if *x > 0, tagged(x);
   };

   result.output.sort_by_key(|row| row.0);
   assert_eq!(result.output, vec![(1, WhyProvenance::token("one")), (2, WhyProvenance::token("two"))]);
}

#[test]
fn generated_annotation_names_do_not_capture_attached_if_let_bindings() {
   let mut result = ascent_run! {
      relation ordinary(i32) = vec![(-1,), (1,), (2,)];
      #[provenance(&'static str)] relation tagged(i32) = vec![
         (-1, WhyProvenance::token("negative")),
         (1, WhyProvenance::token("one")),
         (2, WhyProvenance::token("two")),
      ];
      #[provenance(&'static str)] relation output(i32);

      output(x) <-- ordinary(x) if let Some(__ascent_provenance_0) = Some(*x),
         if __ascent_provenance_0 > 0, tagged(x);
   };

   result.output.sort_by_key(|row| row.0);
   assert_eq!(result.output, vec![(1, WhyProvenance::token("one")), (2, WhyProvenance::token("two"))]);
}

#[test]
fn program_default_storage_provider_still_applies_to_ordinary_relations() {
   let result = ascent_run! {
      #![ds(ascent::rel)]
      #[provenance(&'static str)] relation input(i32) = vec![(1, WhyProvenance::token("one"))];
      relation ordinary(i32);

      ordinary(x) <-- input(x);
   };

   assert_eq!(result.ordinary, vec![(1,)]);
}

#[test]
fn annotated_relations_can_feed_ordinary_aggregation_and_negation() {
   use ascent::aggregators::count;

   let result = ascent_run! {
      #[provenance(&'static str)] relation tagged(i32) = vec![
         (1, WhyProvenance::token("a")),
         (2, WhyProvenance::token("b")),
      ];
      relation candidate(i32) = vec![(1,), (2,), (3,)];
      relation count_tagged(usize);
      relation absent(i32);

      count_tagged(n) <-- agg n = count() in tagged(_);
      absent(x) <-- candidate(x), !tagged(x);
   };

   assert_eq!(result.count_tagged, vec![(2,)]);
   assert_eq!(result.absent, vec![(3,)]);
}

#[test]
fn crossing_an_ordinary_relation_discards_upstream_witnesses() {
   let result = ascent_run! {
      #[provenance(&'static str)] relation tagged(i32) = vec![(1, WhyProvenance::token("upstream"))];
      relation ordinary(i32);
      #[provenance(&'static str)] relation retagged(i32);

      ordinary(x) <-- tagged(x);
      retagged(x) <-- ordinary(x);
   };

   assert_eq!(result.retagged[0].1.witnesses(), &BTreeSet::from([BTreeSet::new()]));
}

#[test]
fn recursive_rules_accumulate_late_alternative_witnesses() {
   ascent! {
      #[provenance(&'static str)] relation edge(i32, i32);
      #[provenance(&'static str)] relation path(i32, i32);

      path(x, y) <-- edge(x, y);
      path(x, z) <-- edge(x, y), path(y, z);
   }

   let mut program = AscentProgram::default();
   program.edge =
      vec![(1, 2, WhyProvenance::token("a")), (2, 3, WhyProvenance::token("b")), (1, 3, WhyProvenance::token("c"))];
   program.run();

   let row = program.path.iter().find(|row| row.0 == 1 && row.1 == 3).unwrap();
   assert_eq!(row.2.witnesses(), &BTreeSet::from([BTreeSet::from(["a", "b"]), BTreeSet::from(["c"])]));
}

#[test]
fn source_inclusion_and_timeout_completion_are_supported() {
   ascent! {
      #![generate_run_timeout]
      struct Included;
      include_source!(provenance_source::rules);
   }

   let mut program = Included::default();
   program.source_input = vec![(7, WhyProvenance::token("included"))];
   assert!(program.run_timeout(std::time::Duration::MAX));
   assert_eq!(program.source_output, vec![(7, WhyProvenance::token("included"))]);
}

#[test]
fn mixed_user_lattice_and_provenance_recursion_remains_permitted() {
   ascent! {
      #[provenance(&'static str)] relation reached(i32);
      relation edge(i32, i32) = vec![(0, 1), (1, 2)];
      lattice enabled(bool);

      enabled(true) <-- reached(_);
      reached(y) <-- reached(x), edge(x, y), enabled(flag), if *flag;
   }

   let mut program = AscentProgram::default();
   program.reached = vec![(0, WhyProvenance::token("seed"))];
   program.run();

   program.reached.sort_by_key(|row| row.0);
   assert_eq!(program.reached.iter().map(|row| row.0).collect::<Vec<_>>(), vec![0, 1, 2]);
   assert!(program.reached.iter().all(|row| row.1 == WhyProvenance::token("seed")));
}
