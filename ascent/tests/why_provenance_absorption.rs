use std::cmp::Ordering;
use std::collections::BTreeSet;

use ascent::{BooleanProvenance as Why, Dual, Lattice, ascent, ascent_run};

fn from_mask(mask: u8) -> Why<u8> {
   (0..3)
      .filter(|bit| mask & (1 << bit) != 0)
      .map(Why::token)
      .fold(Why::__one(), |left, right| Why::__product(&left, &right))
}

fn masks(value: &Why<u8>) -> BTreeSet<u8> {
   value.witnesses().iter().map(|w| w.iter().fold(0, |mask, bit| mask | (1 << bit))).collect()
}

// Independent truth-table model: each bit identifies an available input fact.
fn worlds(witnesses: &BTreeSet<u8>) -> BTreeSet<u8> {
   (0..8).filter(|world| witnesses.iter().any(|w| w & world == *w)).collect()
}

#[test]
fn boolean_lattice_matches_all_three_token_truth_tables() {
   let mut domain = (0..256_u16)
      .map(|alternatives| {
         let raw: BTreeSet<u8> = (0..8).filter(|w| alternatives & (1 << w) != 0).collect();
         let minimal: BTreeSet<u8> =
            raw.iter().copied().filter(|w| !raw.iter().any(|v| v != w && v & w == *v)).collect();
         let value = raw.iter().map(|w| from_mask(*w)).fold(Why::default(), |a, b| a.join(b));
         assert_eq!(masks(&value), minimal);
         value
      })
      .collect::<Vec<_>>();
   domain.sort_by_key(masks);
   domain.dedup();
   assert_eq!(domain.len(), 20);

   for left in &domain {
      for right in &domain {
         let a = worlds(&masks(left));
         let b = worlds(&masks(right));
         let mut joined = left.clone();
         let changed = joined.join_mut(right.clone());
         assert_eq!(worlds(&masks(&joined)), a.union(&b).copied().collect());
         assert_eq!(changed, joined != *left);
         let mut met = left.clone();
         let changed = met.meet_mut(right.clone());
         assert_eq!(worlds(&masks(&met)), a.intersection(&b).copied().collect());
         assert_eq!(changed, met != *left);
         assert_eq!(met, Why::__product(left, right));
         let expected_order = match (a.is_subset(&b), b.is_subset(&a)) {
            (true, true) => Some(Ordering::Equal),
            (true, false) => Some(Ordering::Less),
            (false, true) => Some(Ordering::Greater),
            _ => None,
         };
         assert_eq!(left.partial_cmp(right), expected_order);
         for value in [&joined, &met] {
            let ws = masks(value);
            assert!(ws.iter().all(|w| !ws.iter().any(|v| v != w && v & w == *v)));
         }
      }
   }
}

ascent! {
   #![provenance(boolean)]
   struct Boolean;
   #[provenance(u8)] relation input(u8);
   #[provenance(u8)] relation copied(u8);
   #[provenance(u8)] relation product(u8);
   #[provenance(u8)] relation identity(u8);
   relation background(u8);
   lattice minimum(u8, Dual<u8>);

   copied(x) <-- input(x);
   product(x) <-- input(x), input(x);
   identity(x) <-- background(x);
   minimum(0, Dual(*x)) <-- background(x);
}

#[test]
fn boolean_program_setting_normalizes_inputs_and_first_insertions_without_changing_background() {
   for values in [vec![3, 1, 2], vec![2, 1, 3]] {
      let mut program = Boolean::default();
      program.input = values.into_iter().map(|mask| (0, from_mask(mask))).collect();
      program.input.push((1, Why::default()));
      program.background = vec![(1,), (2,)];
      program.run();
      for rows in [&program.input, &program.copied, &program.product] {
         assert_eq!(rows.len(), 1);
         assert_eq!(masks(&rows[0].1), BTreeSet::from([1, 2]));
      }
      assert!(program.identity.iter().all(|(_, why)| masks(why) == BTreeSet::from([0])));
      assert_eq!(program.identity.len(), 2);
      assert_eq!(program.minimum, vec![(0, Dual(1))]);
      let before = program.product.clone();
      program.run();
      assert_eq!(program.product, before);
   }
}

#[test]
fn inline_boolean_program_absorbs_candidates_from_different_rules() {
   let result = ascent_run! {
      #![provenance(boolean)]
      #[provenance(u8)] relation input(u8) = vec![(0, from_mask(3))];
      #[provenance(u8)] relation smaller(u8) = vec![(0, from_mask(1))];
      #[provenance(u8)] relation output(u8);
      output(x) <-- input(x);
      output(x) <-- smaller(x);
   };
   assert_eq!(result.output.len(), 1);
   assert_eq!(masks(&result.output[0].1), BTreeSet::from([1]));
}
