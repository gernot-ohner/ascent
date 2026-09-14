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

/// Why-provenance retaining only inclusion-minimal witness sets.
///
/// A smaller witness absorbs its supersets. Ordering is logical implication:
/// every witness on the left must contain some witness on the right.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AbsorbingWhyProvenance<T: Ord + Hash>(WhyProvenance<T>);

impl<T: Ord + Hash> Default for AbsorbingWhyProvenance<T> {
   fn default() -> Self { Self(WhyProvenance::default()) }
}

impl<T: Ord + Hash> AbsorbingWhyProvenance<T> {
   /// Returns the inclusion-minimal witness sets.
   pub fn witnesses(&self) -> &BTreeSet<BTreeSet<T>> { self.0.witnesses() }

   /// Constructs provenance for one tagged input fact.
   pub fn token(token: T) -> Self { Self(WhyProvenance::token(token)) }

   #[doc(hidden)]
   pub fn __one() -> Self { Self(WhyProvenance::__one()) }

   fn insert(&mut self, witness: BTreeSet<T>) -> bool {
      if self.witnesses().iter().any(|old| old.is_subset(&witness)) {
         return false;
      }
      self.0.witnesses.0.retain(|old| !witness.is_subset(old));
      self.0.witnesses.0.insert(witness);
      true
   }
}

impl<T: Clone + Ord + Hash> AbsorbingWhyProvenance<T> {
   #[doc(hidden)]
   pub fn __product(&self, other: &Self) -> Self {
      // Normalize candidates too: first insertion of a tuple can bypass join.
      let mut result = Self::default();
      for witness in self.0.__product(&other.0).witnesses.0 {
         result.insert(witness);
      }
      result
   }
}

impl<T: Ord + Hash> PartialOrd for AbsorbingWhyProvenance<T> {
   fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
      let implies =
         |left: &Self, right: &Self| left.witnesses().iter().all(|w| right.witnesses().iter().any(|v| v.is_subset(w)));
      match (implies(self, other), implies(other, self)) {
         (true, true) => Some(Ordering::Equal),
         (true, false) => Some(Ordering::Less),
         (false, true) => Some(Ordering::Greater),
         (false, false) => None,
      }
   }
}

impl<T: Clone + Ord + Hash> Lattice for AbsorbingWhyProvenance<T> {
   fn meet_mut(&mut self, other: Self) -> bool {
      let product = self.__product(&other);
      let changed = *self != product;
      *self = product;
      changed
   }

   fn join_mut(&mut self, other: Self) -> bool {
      let mut changed = false;
      for witness in other.0.witnesses.0 {
         changed |= self.insert(witness);
      }
      changed
   }
}
