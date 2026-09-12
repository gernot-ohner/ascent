use std::collections::BTreeSet;

#[path = "../examples/why_provenance_shortest_path.rs"]
mod example;

use example::shortest_path_with_why;

#[test]
fn returns_every_tied_shortest_walk_including_zero_cost_cycles_and_excludes_longer_routes() {
   let edges = [
      ("A", "B", 2, "ab"),
      ("B", "Z", 3, "bz"),
      ("A", "C", 1, "ac"),
      ("C", "Z", 4, "cz"),
      ("A", "D", 4, "ad"),
      ("D", "Z", 5, "dz"),
      ("B", "U", 0, "bu"),
      ("U", "B", 0, "ub"),
   ];

   let (distance, actual) = shortest_path_with_why(&edges, "A", "Z").unwrap();

   assert_eq!(distance, 5);
   assert_eq!(
      actual,
      BTreeSet::from([
         BTreeSet::from(["ab", "bz"]),
         BTreeSet::from(["ac", "cz"]),
         BTreeSet::from(["ab", "bu", "bz", "ub"]),
      ])
   );
}

#[test]
fn destination_seed_is_the_empty_witness_and_a_tight_zero_cost_loop_is_an_alternative() {
   let edges = [("Z", "Z", 0, "zz")];

   assert_eq!(
      shortest_path_with_why(&edges, "Z", "Z"),
      Some((0, BTreeSet::from([BTreeSet::new(), BTreeSet::from(["zz"])])))
   );
}

#[test]
fn reports_unreachable_sources_and_does_not_treat_overflow_as_a_path() {
   let overflow = [("A", "B", u32::MAX, "ab"), ("B", "Z", 1, "bz")];

   assert_eq!(shortest_path_with_why(&overflow, "A", "Z"), None);
   assert_eq!(shortest_path_with_why(&[], "A", "Z"), None);
}

#[test]
fn source_equal_to_destination_has_an_empty_shortest_suffix_even_without_edges() {
   assert_eq!(shortest_path_with_why(&[], "Z", "Z"), Some((0, BTreeSet::from([BTreeSet::new()]))));
}
