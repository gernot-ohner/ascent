use std::collections::{BTreeMap, BTreeSet, VecDeque};

use ascent::{BooleanProvenance, WhyProvenance, ascent};

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

fn every_three_node_acyclic_graph() -> impl Iterator<Item = (u16, Vec<(Node, Node, Token)>, Reachability)> {
   let pairs = (0..3).flat_map(|source| ((source + 1)..3).map(move |target| (source, target))).collect::<Vec<_>>();

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
fn every_three_node_acyclic_graph_matches_the_linear_walk_oracle_in_both_input_orders() {
   for (mask, edges, expected) in every_three_node_acyclic_graph() {
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
fn every_three_node_acyclic_graph_matches_the_nonlinear_walk_oracle_in_both_input_orders() {
   for (mask, edges, expected) in every_three_node_acyclic_graph() {
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
fn every_three_node_acyclic_graph_matches_the_mutual_walk_oracle_in_both_input_orders() {
   for (mask, edges, expected) in every_three_node_acyclic_graph() {
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

ascent! {
   #![provenance(boolean)]
   struct BooleanReachability;
   #[provenance(Token)] relation edge(Node, Node);
   #[provenance(Token)] relation reachable(Node, Node);
   #[provenance(Token)] relation mirror(Node, Node);
   #[provenance(Token)] relation downstream(Node, Node);

   reachable(x, y) <-- edge(x, y);
   mirror(x, y) <-- reachable(x, y);
   reachable(x, z) <-- mirror(x, y), mirror(y, z);
   downstream(x, y) <-- reachable(x, y);
}

#[test]
fn boolean_matches_minimal_walk_witnesses_on_every_three_node_graph() {
   for (mask, edges, mut expected) in every_three_node_graph() {
      for witnesses in expected.values_mut() {
         let all = witnesses.clone();
         witnesses.retain(|w| !all.iter().any(|v| v != w && v.is_subset(w)));
      }
      for input in [edges.clone(), edges.iter().copied().rev().collect()] {
         let mut program = BooleanReachability::default();
         program.edge = input.iter().map(|&(x, y, t)| (x, y, BooleanProvenance::token(t))).collect();
         program.run();
         for rows in [&program.reachable, &program.mirror, &program.downstream] {
            let actual: Reachability = rows.iter().map(|(x, y, p)| ((*x, *y), p.witnesses().clone())).collect();
            assert_eq!(rows.len(), actual.len());
            assert_eq!(actual, expected, "boolean reachability, graph mask {mask:#05x}");
         }
      }
   }
}
