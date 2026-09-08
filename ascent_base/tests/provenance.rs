use ascent_base::{ConvergentProvenanceSemiring, HowProvenance, ProvenanceSemiring, WhyProvenance};

fn token_how(name: &str) -> HowProvenance<String> { HowProvenance::token(name.to_owned()) }

fn token_why(name: &str) -> WhyProvenance<String> { WhyProvenance::token(name.to_owned()) }

fn add<S: ProvenanceSemiring>(mut left: S, right: &S) -> S {
   left.add_assign(right);
   left
}

fn assert_convergent<S: ConvergentProvenanceSemiring>() {}

#[test]
fn how_provenance_obeys_identities_and_reports_changes() {
   let x = token_how("x");
   let zero = HowProvenance::zero();
   let one = HowProvenance::one();

   assert_eq!(add(zero.clone(), &x), x);
   assert_eq!(x.multiply(&one), x);
   assert_eq!(x.multiply(&zero), zero);

   let mut unchanged = x.clone();
   assert!(!unchanged.add_assign(&zero));

   let mut changed = x.clone();
   assert!(changed.add_assign(&x));
}

#[test]
fn how_provenance_is_a_canonical_polynomial_semiring() {
   let x1 = token_how("x1");
   let x2 = token_how("x2");
   let x3 = token_how("x3");
   let x4 = token_how("x4");

   let diamond = add(x1.multiply(&x3), &x2.multiply(&x4));
   assert_eq!(diamond.to_string(), "x1*x3 + x2*x4");

   let duplicate = add(x1.multiply(&x3), &x1.multiply(&x3));
   assert_eq!(duplicate.to_string(), "2*x1*x3");
   assert_eq!(x1.multiply(&x1).to_string(), "x1^2");

   assert_eq!(x1.multiply(&x2), x2.multiply(&x1));
   assert_eq!(x1.multiply(&x2).multiply(&x3), x1.multiply(&x2.multiply(&x3)));
   assert_eq!(add(x1.clone(), &x2), add(x2.clone(), &x1));
   assert_eq!(add(add(x1.clone(), &x2), &x3), add(x1.clone(), &add(x2.clone(), &x3)));
   assert_eq!(x1.multiply(&add(x2.clone(), &x3)), add(x1.multiply(&x2), &x1.multiply(&x3)));
}

#[test]
fn why_provenance_is_convergent_without_absorption() {
   assert_convergent::<WhyProvenance<String>>();

   let x1 = token_why("x1");
   let x2 = token_why("x2");
   let x3 = token_why("x3");
   let zero = WhyProvenance::zero();
   let one = WhyProvenance::one();

   assert_eq!(add(zero.clone(), &x1), x1);
   assert_eq!(x1.multiply(&one), x1);
   assert_eq!(x1.multiply(&zero), zero);
   assert_eq!(add(x1.clone(), &x1), x1);
   assert_eq!(x1.multiply(&x1), x1);
   assert_eq!(x1.multiply(&x2).multiply(&x3), x1.multiply(&x2.multiply(&x3)));
   assert_eq!(add(x1.clone(), &x2), add(x2.clone(), &x1));
   assert_eq!(add(add(x1.clone(), &x2), &x3), add(x1.clone(), &add(x2.clone(), &x3)));

   let alternatives = add(x1.multiply(&x2), &x1.multiply(&x3));
   assert_eq!(alternatives.to_string(), "{{x1,x2}, {x1,x3}}");

   let no_absorption = add(x1.clone(), &x1.multiply(&x2));
   assert_eq!(no_absorption.to_string(), "{{x1}, {x1,x2}}");

   let mut unchanged = x1.clone();
   assert!(!unchanged.add_assign(&x1));
   let mut changed = x1.clone();
   assert!(changed.add_assign(&x2));
}
