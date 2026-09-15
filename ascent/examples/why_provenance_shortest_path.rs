//! Find every why-provenance witness for a shortest path.
//!
//! The first Ascent computation finds final distances. The second keeps only
//! distance-tight edges and computes their why-provenance. Keeping these as two
//! computations avoids treating changing distance-lattice values as tracked
//! provenance.

use std::collections::{BTreeMap, BTreeSet};

use ascent::{BooleanProvenance, Dual, ascent};

pub type Node = &'static str;
pub type EdgeToken = &'static str;

ascent! {
   struct DistancesToDestination;

   relation edge(Node, Node, u32);
   relation destination(Node);
   lattice distance(Node, Dual<u32>);

   distance(node, Dual(0)) <-- destination(node);
   distance(source, Dual(total)) <--
      edge(source, target, weight),
      distance(target, ?Dual(suffix)),
      if let Some(total) = weight.checked_add(*suffix);
}

ascent! {
   #![provenance(boolean)]
   struct ShortestPathWitnesses;

   relation destination(Node);
   #[provenance(EdgeToken)] relation tight_edge(Node, Node);
   #[provenance(EdgeToken)] relation reaches_destination(Node);

   reaches_destination(node) <-- destination(node);
   reaches_destination(source) <-- tight_edge(source, target), reaches_destination(target);
}

/// Returns the shortest distance and the minimal token set for every shortest walk.
///
/// Weights are nonnegative. Arithmetic overflow is treated as an unusable
/// edge. A witness is a set, so it records neither traversal order nor repeated
/// uses of the same edge token.
pub fn shortest_path_with_why(
   edges: &[(Node, Node, u32, EdgeToken)], source: Node, destination: Node,
) -> Option<(u32, BTreeSet<BTreeSet<EdgeToken>>)> {
   let mut distances = DistancesToDestination::default();
   distances.edge = edges.iter().map(|&(from, to, weight, _)| (from, to, weight)).collect();
   distances.destination = vec![(destination,)];
   distances.run();

   let distances_by_node =
      distances.distance.iter().map(|(node, Dual(distance))| (*node, *distance)).collect::<BTreeMap<_, _>>();
   let source_distance = *distances_by_node.get(source)?;

   let mut witnesses = ShortestPathWitnesses::default();
   witnesses.destination = vec![(destination,)];
   witnesses.tight_edge = edges
      .iter()
      .filter(|&&(from, to, weight, _)| {
         let Some(&from_distance) = distances_by_node.get(from) else { return false };
         let Some(&to_distance) = distances_by_node.get(to) else { return false };
         weight.checked_add(to_distance) == Some(from_distance)
      })
      .map(|&(from, to, _, token)| (from, to, BooleanProvenance::token(token)))
      .collect();
   witnesses.run();

   let provenance = witnesses
      .reaches_destination
      .iter()
      .find(|(node, _)| *node == source)
      .map(|(_, provenance)| provenance.witnesses().clone())?;
   Some((source_distance, provenance))
}

#[allow(dead_code)]
fn main() {
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

   let result = shortest_path_with_why(&edges, "A", "Z");
   println!("{result:?}");
}
