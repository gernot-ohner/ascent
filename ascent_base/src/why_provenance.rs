use std::cmp::Ordering;
use std::collections::BTreeSet;

use crate::Lattice;

/// Why-provenance represented as alternative sets of jointly required input tokens.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WhyProvenance<T> {
   witnesses: BTreeSet<BTreeSet<T>>,
}

impl<T> Default for WhyProvenance<T> {
   fn default() -> Self { Self { witnesses: BTreeSet::new() } }
}

impl<T> WhyProvenance<T> {
   /// Returns all distinct witness sets.
   pub fn witnesses(&self) -> &BTreeSet<BTreeSet<T>> { &self.witnesses }
}

impl<T: Ord> WhyProvenance<T> {
   /// Constructs provenance for one tagged input fact.
   pub fn token(token: T) -> Self { Self { witnesses: BTreeSet::from([BTreeSet::from([token])]) } }

   #[doc(hidden)]
   pub fn __one() -> Self { Self { witnesses: BTreeSet::from([BTreeSet::new()]) } }
}

impl<T: Clone + Ord> WhyProvenance<T> {
   #[doc(hidden)]
   pub fn __product(&self, other: &Self) -> Self {
      let witnesses = self
         .witnesses
         .iter()
         .flat_map(|left| {
            other.witnesses.iter().map(|right| {
               let mut witness = left.clone();
               witness.extend(right.iter().cloned());
               witness
            })
         })
         .collect();
      Self { witnesses }
   }
}

impl<T: Ord> PartialOrd for WhyProvenance<T> {
   fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
      let self_subset = self.witnesses.is_subset(&other.witnesses);
      let other_subset = other.witnesses.is_subset(&self.witnesses);
      match (self_subset, other_subset) {
         (true, true) => Some(Ordering::Equal),
         (true, false) => Some(Ordering::Less),
         (false, true) => Some(Ordering::Greater),
         (false, false) => None,
      }
   }
}

impl<T: Ord> Lattice for WhyProvenance<T> {
   fn meet_mut(&mut self, other: Self) -> bool {
      let old_len = self.witnesses.len();
      self.witnesses.retain(|witness| other.witnesses.contains(witness));
      self.witnesses.len() != old_len
   }

   fn join_mut(&mut self, other: Self) -> bool {
      let old_len = self.witnesses.len();
      self.witnesses.extend(other.witnesses);
      self.witnesses.len() != old_len
   }
}
