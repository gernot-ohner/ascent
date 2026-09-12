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

#[test]
fn linear_recursion_matches_the_walk_oracle_in_both_input_orders() {
   let edges = vec![(0, 1, 0), (1, 2, 1), (0, 2, 2), (2, 1, 3), (1, 1, 4)];
   let expected = walk_oracle(&edges);

   for input in [edges.clone(), edges.iter().copied().rev().collect()] {
      let mut program = LinearReachability::default();
      program.edge = tagged_edges(&input);
      program.run();

      assert_eq!(relation_contents(&program.reachable), expected);
      assert_eq!(relation_contents(&program.downstream), expected);
   }
}

#[test]
fn nonlinear_recursion_propagates_late_alternatives_to_downstream_consumers() {
   let edges = vec![(0, 1, 0), (1, 2, 1), (0, 2, 2), (1, 1, 3)];
   let expected = walk_oracle(&edges);

   for input in [edges.clone(), edges.iter().copied().rev().collect()] {
      let mut program = NonlinearReachability::default();
      program.edge = tagged_edges(&input);
      program.run();

      assert_eq!(relation_contents(&program.reachable), expected);
      assert_eq!(relation_contents(&program.downstream), expected);
   }
}

#[test]
fn mutual_recursion_propagates_late_alternatives_to_every_member_and_consumer() {
   let edges = vec![(0, 1, 0), (1, 2, 1), (0, 2, 2), (2, 1, 3)];
   let expected = walk_oracle(&edges);

   for input in [edges.clone(), edges.iter().copied().rev().collect()] {
      let mut program = MutualReachability::default();
      program.edge = tagged_edges(&input);
      program.run();

      assert_eq!(relation_contents(&program.reachable), expected);
      assert_eq!(relation_contents(&program.mirror), expected);
      assert_eq!(relation_contents(&program.downstream), expected);
   }
}
