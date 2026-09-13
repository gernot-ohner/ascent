use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::hash::Hash;

use crate::Lattice;
use crate::lattice::set::Set;

/// Why-provenance represented as alternative sets of jointly required input tokens.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WhyProvenance<T: Ord + Hash> {
   witnesses: Set<BTreeSet<T>>,
}

impl<T: Ord + Hash> Default for WhyProvenance<T> {
   fn default() -> Self { Self { witnesses: Set::default() } }
}

impl<T: Ord + Hash> WhyProvenance<T> {
   /// Returns all distinct witness sets.
   pub fn witnesses(&self) -> &BTreeSet<BTreeSet<T>> { &self.witnesses }
   /// Constructs provenance for one tagged input fact.
   pub fn token(token: T) -> Self { Self { witnesses: Set::singleton(BTreeSet::from([token])) } }

   #[doc(hidden)]
   pub fn __one() -> Self { Self { witnesses: Set::singleton(BTreeSet::new()) } }
}

impl<T: Clone + Ord + Hash> WhyProvenance<T> {
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
      Self { witnesses: Set(witnesses) }
   }
}

impl<T: Ord + Hash> PartialOrd for WhyProvenance<T> {
   fn partial_cmp(&self, other: &Self) -> Option<Ordering> { self.witnesses.partial_cmp(&other.witnesses) }
}

impl<T: Ord + Hash> Lattice for WhyProvenance<T> {
   fn meet_mut(&mut self, other: Self) -> bool { self.witnesses.meet_mut(other.witnesses) }

   fn join_mut(&mut self, other: Self) -> bool { self.witnesses.join_mut(other.witnesses) }
}
