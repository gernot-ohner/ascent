use std::collections::{BTreeMap, BTreeSet, VecDeque};

use ascent::{WhyProvenance, ascent};

type Node = u8;
type Token = u8;
type Witness = BTreeSet<Token>;
type Witnesses = BTreeSet<Witness>;
type Reachability = BTreeMap<(Node, Node), Witnesses>;

ascent! {
   struct LinearReachability;

   #[provenance(Token)] relation edge(Node, Node);
   #[provenance(Token)] relation reachable(Node, Node);
   #[provenance(Token)] relation downstream(Node, Node);

   reachable(x, y) <-- edge(x, y);
   reachable(x, z) <-- edge(x, y), reachable(y, z);
   downstream(x, y) <-- reachable(x, y);
}

ascent! {
   struct NonlinearReachability;

   #[provenance(Token)] relation edge(Node, Node);
   #[provenance(Token)] relation reachable(Node, Node);
   #[provenance(Token)] relation downstream(Node, Node);

   reachable(x, y) <-- edge(x, y);
   reachable(x, z) <-- reachable(x, y), reachable(y, z);
   downstream(x, y) <-- reachable(x, y);
}

ascent! {
   struct MutualReachability;

   #[provenance(Token)] relation edge(Node, Node);
   #[provenance(Token)] relation reachable(Node, Node);
   #[provenance(Token)] relation mirror(Node, Node);
   #[provenance(Token)] relation downstream(Node, Node);

   reachable(x, y) <-- edge(x, y);
   mirror(x, y) <-- reachable(x, y);
   reachable(x, z) <-- edge(x, y), mirror(y, z);
   downstream(x, y) <-- mirror(x, y);
}

fn walk_oracle(edges: &[(Node, Node, Token)]) -> Reachability {
   let mut result = Reachability::new();
   for source in 0..3 {
      let mut visited = BTreeSet::from([(source, Witness::new())]);
      let mut queue = VecDeque::from([(source, Witness::new())]);

      while let Some((node, used_tokens)) = queue.pop_front() {
         for &(edge_source, endpoint, token) in edges {
            if edge_source != node {
               continue;
            }
            let mut next_tokens = used_tokens.clone();
            next_tokens.insert(token);
            result.entry((source, endpoint)).or_default().insert(next_tokens.clone());
            if visited.insert((endpoint, next_tokens.clone())) {
               queue.push_back((endpoint, next_tokens));
            }
         }
      }
   }
   result
}

fn tagged_edges(edges: &[(Node, Node, Token)]) -> Vec<(Node, Node, WhyProvenance<Token>)> {
   edges.iter().map(|&(source, target, token)| (source, target, WhyProvenance::token(token))).collect()
}

fn relation_contents(rows: &[(Node, Node, WhyProvenance<Token>)]) -> Reachability {
   rows.iter().map(|(source, target, provenance)| ((*source, *target), provenance.witnesses().clone())).collect()
}

fn every_three_node_graph() -> impl Iterator<Item = (u16, Vec<(Node, Node, Token)>, Reachability)> {
   let pairs = (0..3).flat_map(|source| (0..3).map(move |target| (source, target))).collect::<Vec<_>>();

   (0..(1_u16 << pairs.len())).map(move |mask| {
      let edges = pairs
         .iter()
         .enumerate()
         .filter(|(token, _)| mask & (1 << token) != 0)
         .map(|(token, &(source, target))| (source, target, token as Token))
         .collect::<Vec<_>>();
      let expected = walk_oracle(&edges);
      (mask, edges, expected)
   })
}

#[test]
fn every_three_node_graph_matches_the_linear_walk_oracle_in_both_input_orders() {
   for (mask, edges, expected) in every_three_node_graph() {
      for input in [edges.clone(), edges.iter().copied().rev().collect()] {
         let mut linear = LinearReachability::default();
         linear.edge = tagged_edges(&input);
         linear.run();
         assert_eq!(relation_contents(&linear.reachable), expected, "linear reachability, graph mask {mask:#05x}");
         assert_eq!(relation_contents(&linear.downstream), expected, "linear downstream, graph mask {mask:#05x}");
      }
   }
}

#[test]
fn every_three_node_graph_with_at_most_six_edges_matches_the_nonlinear_walk_oracle_in_both_input_orders() {
   for (mask, edges, expected) in every_three_node_graph().filter(|(mask, _, _)| mask.count_ones() <= 6) {
      for input in [edges.clone(), edges.iter().copied().rev().collect()] {
         let mut nonlinear = NonlinearReachability::default();
         nonlinear.edge = tagged_edges(&input);
         nonlinear.run();
         assert_eq!(
            relation_contents(&nonlinear.reachable),
            expected,
            "nonlinear reachability, graph mask {mask:#05x}"
         );
         assert_eq!(relation_contents(&nonlinear.downstream), expected, "nonlinear downstream, graph mask {mask:#05x}");
      }
   }
}

#[test]
fn every_three_node_graph_matches_the_mutual_walk_oracle_in_both_input_orders() {
   for (mask, edges, expected) in every_three_node_graph() {
      for input in [edges.clone(), edges.iter().copied().rev().collect()] {
         let mut mutual = MutualReachability::default();
         mutual.edge = tagged_edges(&input);
         mutual.run();
         assert_eq!(relation_contents(&mutual.reachable), expected, "mutual reachability, graph mask {mask:#05x}");
         assert_eq!(relation_contents(&mutual.mirror), expected, "mutual mirror, graph mask {mask:#05x}");
         assert_eq!(relation_contents(&mutual.downstream), expected, "mutual downstream, graph mask {mask:#05x}");
      }
   }
}
