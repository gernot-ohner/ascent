use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use ascent::{Lattice, WhyProvenance, ascent};

type Node = u8;
type Token = u8;
type Witness = BTreeSet<Token>;
type Witnesses = BTreeSet<Witness>;

ascent! {
   struct NormalizedCopy;

   #[provenance(Token)] relation input(Node);
   #[provenance(Token)] relation output(Node);

   output(x) <-- input(x);
}

fn single_witness(tokens: &Witness) -> WhyProvenance<Token> {
   tokens
      .iter()
      .copied()
      .map(WhyProvenance::token)
      .fold(WhyProvenance::__one(), |product, token| WhyProvenance::__product(&product, &token))
}

fn finite_domain() -> Vec<(WhyProvenance<Token>, Witnesses)> {
   let possible_witnesses =
      (0..4).map(|mask| (0..2).filter(|token| mask & (1 << token) != 0).collect::<Witness>()).collect::<Vec<_>>();

   (0..(1 << possible_witnesses.len()))
      .map(|outer_mask| {
         let mut value = WhyProvenance::default();
         let mut model = Witnesses::new();
         for (witness_index, witness) in possible_witnesses.iter().enumerate() {
            if outer_mask & (1 << witness_index) != 0 {
               value.join_mut(single_witness(witness));
               model.insert(witness.clone());
            }
         }
         (value, model)
      })
      .collect()
}

fn model_product(left: &Witnesses, right: &Witnesses) -> Witnesses {
   left
      .iter()
      .flat_map(|left_witness| {
         right.iter().map(|right_witness| left_witness.union(right_witness).copied().collect::<Witness>())
      })
      .collect()
}

fn model_order(left: &Witnesses, right: &Witnesses) -> Option<Ordering> {
   match (left.is_subset(right), right.is_subset(left)) {
      (true, true) => Some(Ordering::Equal),
      (true, false) => Some(Ordering::Less),
      (false, true) => Some(Ordering::Greater),
      (false, false) => None,
   }
}

#[test]
fn finite_domain_operations_match_the_independent_set_model() {
   let domain = finite_domain();
   assert_eq!(domain.len(), 16);

   for (value_index, (value, model)) in domain.iter().enumerate() {
      assert_eq!(value.witnesses(), model, "domain value {value_index}");

      for (other_index, (other, other_model)) in domain.iter().enumerate() {
         let expected_join = model.union(other_model).cloned().collect::<Witnesses>();
         let expected_meet = model.intersection(other_model).cloned().collect::<Witnesses>();
         let expected_product = model_product(model, other_model);

         let mut joined = value.clone();
         let join_changed = joined.join_mut(other.clone());
         assert_eq!(joined.witnesses(), &expected_join, "join of values {value_index} and {other_index}");
         assert_eq!(join_changed, &expected_join != model, "join change flag for {value_index} and {other_index}");

         let mut met = value.clone();
         let meet_changed = met.meet_mut(other.clone());
         assert_eq!(met.witnesses(), &expected_meet, "meet of values {value_index} and {other_index}");
         assert_eq!(meet_changed, &expected_meet != model, "meet change flag for {value_index} and {other_index}");

         let product = WhyProvenance::__product(value, other);
         assert_eq!(product.witnesses(), &expected_product, "product of values {value_index} and {other_index}");
         assert_eq!(
            value.partial_cmp(other),
            model_order(model, other_model),
            "order of {value_index} and {other_index}"
         );
      }
   }
}

#[test]
fn every_finite_domain_triple_obeys_lattice_and_product_laws() {
   let domain = finite_domain();

   for (left_index, (left, _)) in domain.iter().enumerate() {
      for (middle_index, (middle, _)) in domain.iter().enumerate() {
         assert_eq!(left.clone().join(middle.clone()), middle.clone().join(left.clone()));
         assert_eq!(left.clone().meet(middle.clone()), middle.clone().meet(left.clone()));
         assert_eq!(
            left.clone().meet(left.clone().join(middle.clone())),
            left.clone(),
            "meet absorption for values {left_index} and {middle_index}"
         );
         assert_eq!(
            left.clone().join(left.clone().meet(middle.clone())),
            left.clone(),
            "join absorption for values {left_index} and {middle_index}"
         );

         for (right_index, (right, _)) in domain.iter().enumerate() {
            assert_eq!(
               left.clone().join(middle.clone()).join(right.clone()),
               left.clone().join(middle.clone().join(right.clone())),
               "join associativity for values {left_index}, {middle_index}, and {right_index}"
            );
            assert_eq!(
               left.clone().meet(middle.clone()).meet(right.clone()),
               left.clone().meet(middle.clone().meet(right.clone())),
               "meet associativity for values {left_index}, {middle_index}, and {right_index}"
            );

            let left_middle = WhyProvenance::__product(left, middle);
            let middle_right = WhyProvenance::__product(middle, right);
            assert_eq!(
               WhyProvenance::__product(&left_middle, right),
               WhyProvenance::__product(left, &middle_right),
               "product associativity for values {left_index}, {middle_index}, and {right_index}"
            );

            let middle_or_right = middle.clone().join(right.clone());
            let distributed_left = WhyProvenance::__product(left, &middle_or_right);
            let distributed_right = WhyProvenance::__product(left, middle).join(WhyProvenance::__product(left, right));
            assert_eq!(
               distributed_left, distributed_right,
               "left distributivity for values {left_index}, {middle_index}, and {right_index}"
            );
         }
      }
   }
}

fn lifecycle_choice(choice: usize) -> Option<(Node, WhyProvenance<Token>)> {
   if choice == 0 {
      return None;
   }

   let choice = choice - 1;
   let node = (choice / 5) as Node;
   let annotation = match choice % 5 {
      0 => WhyProvenance::default(),
      1 => WhyProvenance::__one(),
      2 => WhyProvenance::token(0),
      3 => WhyProvenance::token(1),
      4 => WhyProvenance::__product(&WhyProvenance::token(0), &WhyProvenance::token(1)),
      _ => unreachable!(),
   };
   Some((node, annotation))
}

fn unary_relation_contents(rows: &[(Node, WhyProvenance<Token>)]) -> BTreeMap<Node, Witnesses> {
   rows.iter().map(|(node, provenance)| (*node, provenance.witnesses().clone())).collect()
}

#[test]
fn every_bounded_input_batch_normalizes_and_reruns_idempotently() {
   for first in 0..11 {
      for second in 0..11 {
         for third in 0..11 {
            let input = [first, second, third].into_iter().filter_map(lifecycle_choice).collect::<Vec<_>>();
            let mut expected = BTreeMap::<Node, Witnesses>::new();
            for (node, provenance) in &input {
               expected.entry(*node).or_default().extend(provenance.witnesses().iter().cloned());
            }
            expected.retain(|_, witnesses| !witnesses.is_empty());

            let mut program = NormalizedCopy::default();
            program.input = input;
            program.run();

            let case = format!("input choices [{first}, {second}, {third}]");
            assert_eq!(program.input.len(), expected.len(), "normalized input row count for {case}");
            assert_eq!(unary_relation_contents(&program.input), expected, "normalized input witnesses for {case}");
            assert_eq!(program.output.len(), expected.len(), "output row count for {case}");
            assert_eq!(unary_relation_contents(&program.output), expected, "output witnesses for {case}");

            let before_rerun = (program.input.clone(), program.output.clone());
            program.run();
            assert_eq!((program.input, program.output), before_rerun, "unchanged rerun for {case}");
         }
      }
   }
}
