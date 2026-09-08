//! Semiring domains for tracking how and why Datalog tuples were derived.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Debug, Display, Formatter};

use num_bigint::BigUint;

/// An annotation algebra used by provenance-aware Ascent programs.
///
/// Implementations must obey the laws of a commutative semiring. Rust's trait
/// system cannot express those algebraic laws, so implementations should test
/// them directly.
pub trait ProvenanceSemiring: Clone + Eq {
   /// The annotation for a tuple with no derivations.
   fn zero() -> Self;

   /// The annotation at the start of a rule derivation.
   fn one() -> Self;

   /// Adds an alternative derivation, returning whether the annotation changed.
   fn add_assign(&mut self, other: &Self) -> bool;

   /// Combines annotations for facts jointly used by one derivation.
   fn multiply(&self, other: &Self) -> Self;
}

/// Marks semirings that are safe for recursive cumulative re-evaluation.
///
/// In addition to the [`ProvenanceSemiring`] laws, addition must be idempotent
/// and every ascending annotation chain reachable from a finite input must
/// terminate. The evaluator re-enqueues a changed tuple with its full
/// accumulated annotation, so idempotence is required to avoid recounting old
/// derivations.
pub trait ConvergentProvenanceSemiring: ProvenanceSemiring {}

/// Canonical polynomials over tokens in `N[X]`.
///
/// A monomial is stored as a sorted token vector. A map entry records its
/// arbitrary-precision natural-number coefficient.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HowProvenance<T> {
   terms: BTreeMap<Vec<T>, BigUint>,
}

impl<T> HowProvenance<T>
where T: Ord
{
   /// Creates the polynomial consisting of one input token.
   pub fn token(token: T) -> Self { Self { terms: BTreeMap::from([(vec![token], BigUint::from(1u8))]) } }

   /// Returns the canonical polynomial terms.
   pub fn terms(&self) -> &BTreeMap<Vec<T>, BigUint> { &self.terms }
}

impl<T> Default for HowProvenance<T> {
   fn default() -> Self { Self { terms: BTreeMap::new() } }
}

impl<T> ProvenanceSemiring for HowProvenance<T>
where T: Clone + Ord
{
   fn zero() -> Self { Self::default() }

   fn one() -> Self { Self { terms: BTreeMap::from([(Vec::new(), BigUint::from(1u8))]) } }

   fn add_assign(&mut self, other: &Self) -> bool {
      if other.terms.is_empty() {
         return false
      }
      for (monomial, coefficient) in &other.terms {
         *self.terms.entry(monomial.clone()).or_default() += coefficient;
      }
      true
   }

   fn multiply(&self, other: &Self) -> Self {
      let mut result = Self::zero();
      for (left_monomial, left_coefficient) in &self.terms {
         for (right_monomial, right_coefficient) in &other.terms {
            let mut monomial = left_monomial.clone();
            monomial.extend(right_monomial.iter().cloned());
            monomial.sort_unstable();
            *result.terms.entry(monomial).or_default() += left_coefficient * right_coefficient;
         }
      }
      result
   }
}

impl<T> Debug for HowProvenance<T>
where T: Debug
{
   fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
      formatter.debug_tuple("HowProvenance").field(&self.terms).finish()
   }
}

impl<T> Display for HowProvenance<T>
where T: Display + Ord
{
   fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
      if self.terms.is_empty() {
         return formatter.write_str("0")
      }

      for (term_index, (monomial, coefficient)) in self.terms.iter().enumerate() {
         if term_index > 0 {
            formatter.write_str(" + ")?;
         }
         let coefficient_is_one = coefficient == &BigUint::from(1u8);
         if monomial.is_empty() || !coefficient_is_one {
            write!(formatter, "{coefficient}")?;
            if !monomial.is_empty() {
               formatter.write_str("*")?;
            }
         }

         let mut variable_index = 0;
         let mut first_variable = true;
         while variable_index < monomial.len() {
            let token = &monomial[variable_index];
            let mut next_index = variable_index + 1;
            while next_index < monomial.len() && &monomial[next_index] == token {
               next_index += 1;
            }
            if !first_variable {
               formatter.write_str("*")?;
            }
            write!(formatter, "{token}")?;
            let exponent = next_index - variable_index;
            if exponent > 1 {
               write!(formatter, "^{exponent}")?;
            }
            first_variable = false;
            variable_index = next_index;
         }
      }
      Ok(())
   }
}

/// Sets of witness sets for why-provenance.
///
/// Alternative derivations union witness sets. Joint use unions the tokens in
/// each pair of witnesses. No absorption or minimal-witness simplification is
/// applied.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WhyProvenance<T> {
   witnesses: BTreeSet<BTreeSet<T>>,
}

impl<T> WhyProvenance<T>
where T: Ord
{
   /// Creates why-provenance containing one singleton witness.
   pub fn token(token: T) -> Self { Self { witnesses: BTreeSet::from([BTreeSet::from([token])]) } }

   /// Returns all witness sets without absorption or minimization.
   pub fn witnesses(&self) -> &BTreeSet<BTreeSet<T>> { &self.witnesses }
}

impl<T> Default for WhyProvenance<T> {
   fn default() -> Self { Self { witnesses: BTreeSet::new() } }
}

impl<T> ProvenanceSemiring for WhyProvenance<T>
where T: Clone + Ord
{
   fn zero() -> Self { Self::default() }

   fn one() -> Self { Self { witnesses: BTreeSet::from([BTreeSet::new()]) } }

   fn add_assign(&mut self, other: &Self) -> bool {
      let old_len = self.witnesses.len();
      self.witnesses.extend(other.witnesses.iter().cloned());
      old_len != self.witnesses.len()
   }

   fn multiply(&self, other: &Self) -> Self {
      let witnesses = self
         .witnesses
         .iter()
         .flat_map(|left| {
            other.witnesses.iter().map(move |right| {
               let mut witness = left.clone();
               witness.extend(right.iter().cloned());
               witness
            })
         })
         .collect();
      Self { witnesses }
   }
}

impl<T> ConvergentProvenanceSemiring for WhyProvenance<T> where T: Clone + Ord {}

impl<T> Debug for WhyProvenance<T>
where T: Debug
{
   fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
      formatter.debug_tuple("WhyProvenance").field(&self.witnesses).finish()
   }
}

impl<T> Display for WhyProvenance<T>
where T: Display
{
   fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
      if self.witnesses.is_empty() {
         return formatter.write_str("∅")
      }
      formatter.write_str("{")?;
      for (witness_index, witness) in self.witnesses.iter().enumerate() {
         if witness_index > 0 {
            formatter.write_str(", ")?;
         }
         formatter.write_str("{")?;
         for (token_index, token) in witness.iter().enumerate() {
            if token_index > 0 {
               formatter.write_str(",")?;
            }
            write!(formatter, "{token}")?;
         }
         formatter.write_str("}")?;
      }
      formatter.write_str("}")
   }
}
