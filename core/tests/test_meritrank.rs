#[allow(unused_imports)]
#[cfg(test)]

mod tests {
  use super::*;
  use indexmap::indexmap;
  use meritrank_core::graph::{EdgeId, NodeId};
  use meritrank_core::random_walk::RandomWalk;
  use meritrank_core::walk_storage::WalkStorage;
  use meritrank_core::{assert_approx_eq, Graph, MeritRank, Weight};

  use rand::Rng;
  use std::collections::HashMap;

  // lets write test for get_personal_hits(&self) -> &HashMap<NodeId, Counter>
  #[test]
  fn test_get_personal_hits() {
    let graph = Graph::new();
    let merit_rank = MeritRank::new(graph);
    let result = merit_rank.get_personal_hits();
    assert!(result.is_empty());
  }

  #[test]
  fn test_basic_chain_graph() {
    let mut rank_ref = MeritRank::new(Graph::new());
    rank_ref.get_new_nodeid();
    let walk_count = 10000;

    let mut rank = MeritRank::new(Graph::new());
    rank.get_new_nodeid();
    rank.calculate(0, walk_count).unwrap();
    for n in 1..9 {
      rank_ref.get_new_nodeid();
      rank_ref.set_edge(n - 1, n, 1.0);
      rank.get_new_nodeid();
      rank.set_edge(n - 1, n, 1.0);
    }
    rank_ref.set_edge(8, 1, 1.0);
    rank.set_edge(8, 1, 1.0);
    rank_ref.calculate(0, walk_count).unwrap();
    println!("{:?}", rank_ref.get_all_scores(0, None));
    println!("{:?}", rank.get_all_scores(0, None));
    for n in 1..8 {
      let ref_score = rank_ref.get_node_score(0, n).unwrap() as f64;
      let score = rank.get_node_score(0, n).unwrap() as f64;
      assert_approx_eq!(ref_score, score, 0.1);
    }
  }

  #[test]
  fn test_clone_basic_chain_graph() {
    let mut rank_ref = MeritRank::new(Graph::new());
    rank_ref.get_new_nodeid();
    let walk_count = 10000;

    let mut rank = MeritRank::new(Graph::new());
    rank.get_new_nodeid();
    rank.calculate(0, walk_count).unwrap();
    for n in 1..9 {
      rank_ref.get_new_nodeid();
      rank_ref.set_edge(n - 1, n, 1.0);
      rank.get_new_nodeid();
      rank.set_edge(n - 1, n, 1.0);
    }
    rank_ref.set_edge(8, 1, 1.0);
    rank.set_edge(8, 1, 1.0);
    let cloned = rank.clone();
    rank_ref.calculate(0, walk_count).unwrap();
    println!("{:?}", cloned.get_all_scores(0, None));
    for n in 1..8 {
      let ref_score = rank_ref.get_node_score(0, n).unwrap() as f64;
      let score = cloned.get_node_score(0, n).unwrap() as f64;
      assert_approx_eq!(ref_score, score, 0.1);
    }
  }

  #[test]
  fn test_negative_hits_basic() {
    let walk_count = 1000;
    let mut rank = MeritRank::new(Graph::new());
    rank.get_new_nodeid();
    rank.get_new_nodeid();
    rank.get_new_nodeid();
    rank.get_new_nodeid();

    rank.set_edge(0, 1, -10000.0);
    rank.set_edge(0, 2, 0.0001);
    rank.set_edge(1, 0, 1.0);
    rank.calculate(0, walk_count).unwrap();
  }

  #[ignore]
  #[test]
  fn test_too_early_cut_position_bug() {
    let walk_count = 10000;
    let mut ref_rank = MeritRank::new(Graph::new());
    ref_rank.get_new_nodeid();
    ref_rank.get_new_nodeid();
    ref_rank.get_new_nodeid();
    ref_rank.set_edge(0, 1, -1.0);
    ref_rank.set_edge(0, 2, 1.0);
    ref_rank.set_edge(1, 2, 1.0);
    ref_rank.set_edge(2, 1, 1.0);
    ref_rank.set_edge(2, 0, 1.0);
    ref_rank.calculate(0, walk_count).unwrap();

    let mut rank = MeritRank::new(Graph::new());
    rank.get_new_nodeid();
    rank.get_new_nodeid();
    rank.get_new_nodeid();
    rank.set_edge(0, 1, -1.0);
    rank.set_edge(0, 2, 1.0);
    rank.set_edge(1, 2, 1.0);
    rank.set_edge(2, 1, 1.0);

    rank.calculate(0, walk_count).unwrap();

    //rank.print_walks();
    rank.set_edge(2, 0, 1.0);
    let ref_score = ref_rank.get_node_score(0, 2).unwrap() as f64;
    let score = rank.get_node_score(0, 2).unwrap() as f64;
    assert_approx_eq!(ref_score, score, 0.2);

    println!("{:?}", rank.get_all_scores(0, None));
    println!("{:?}", ref_rank.get_all_scores(0, None));
    //rank.print_walks();
  }

  #[test]
  fn test_too_much_incremental_ego_bug() {
    let walk_count = 10000;
    let mut ref_rank = MeritRank::new(Graph::new());
    ref_rank.get_new_nodeid();
    ref_rank.get_new_nodeid();
    ref_rank.get_new_nodeid();
    ref_rank.set_edge(0, 2, 1.0);
    ref_rank.set_edge(1, 0, 1.0);
    ref_rank.set_edge(2, 1, 1.0);
    ref_rank.calculate(0, walk_count).unwrap();

    let mut rank = MeritRank::new(Graph::new());
    rank.get_new_nodeid();
    rank.get_new_nodeid();
    rank.get_new_nodeid();
    rank.set_edge(0, 1, -1.0);
    rank.set_edge(0, 2, 1.0);
    rank.set_edge(1, 0, 1.0);
    rank.set_edge(2, 1, 1.0);

    rank.calculate(0, walk_count).unwrap();

    //rank.print_walks();
    rank.set_edge(0, 1, 0.0);

    let ref_score = ref_rank.get_node_score(0, 2).unwrap() as f64;
    let score = rank.get_node_score(0, 2).unwrap() as f64;
    assert_approx_eq!(ref_score, score, 0.1);

    println!("{:?}", rank.get_all_scores(0, None));
    println!("{:?}", ref_rank.get_all_scores(0, None));
    //rank.print_walks();
  }

  #[test]
  fn test_return_strictly_negative_scores() {
    let walk_count = 100;
    let mut rank = MeritRank::new(Graph::new());
    rank.get_new_nodeid();
    rank.get_new_nodeid();
    rank.get_new_nodeid();
    rank.set_edge(0, 1, 1.0);
    rank.set_edge(0, 2, -1.0);
    rank.calculate(0, walk_count).unwrap();
    let result = rank.get_all_scores(0, None).unwrap();

    assert_eq!(result.len(), 3);
    assert!(result[2].1 < 0.0);
  }

  #[test]
  fn test_node_data_get_outgoing_edges() {
    let mut graph = Graph::new();

    // Create nodes
    let node0 = graph.get_new_nodeid();
    let node1 = graph.get_new_nodeid();
    let node2 = graph.get_new_nodeid();
    let node3 = graph.get_new_nodeid();

    // Add edges
    graph.set_edge(node0, node1, 1.0).unwrap();
    graph.set_edge(node0, node2, -2.0).unwrap();
    graph.set_edge(node0, node3, 3.0).unwrap();

    // Get the NodeData for node0
    let node_data = graph.get_node_data(node0).unwrap();

    // Collect outgoing edges into a vector
    let outgoing_edges: Vec<(NodeId, f64)> =
      node_data.get_outgoing_edges().collect();

    // Sort the edges for consistent comparison
    let mut sorted_edges = outgoing_edges;
    sorted_edges.sort_by(|a, b| a.0.cmp(&b.0));

    // Expected edges (sorted by NodeId)
    let expected_edges = vec![(node1, 1.0), (node2, -2.0), (node3, 3.0)];

    assert_eq!(sorted_edges, expected_edges);
  }

  #[test]
  fn test_get_inbound_edges_not_affected_by_outbound_changes() {
    let mut graph = Graph::new();
    let mut rng = rand::thread_rng();

    // Create 10 nodes
    let nodes: Vec<NodeId> = (0..10).map(|_| graph.get_new_nodeid()).collect();

    // Perform 50 random edge operations
    for _ in 0..50 {
      let from = nodes[rng.gen_range(0..nodes.len())];
      let to = nodes[rng.gen_range(0..nodes.len())];
      if from != to {
        let weight = rng.gen_range(-5.0..5.0);
        if weight != 0.0 {
          graph.set_edge(from, to, weight).unwrap();
        } else {
          // If weight is 0, remove the edge if it exists
          let _ = graph.remove_edge(from, to);
        }
      }
    }

    // Manually calculate inbound edges
    let mut manual_inbound_edges: HashMap<NodeId, HashMap<NodeId, Weight>> =
      HashMap::new();
    for &from in &nodes {
      let node_data = graph.get_node_data(from).unwrap();
      for (to, weight) in node_data.get_outgoing_edges() {
        manual_inbound_edges
          .entry(to)
          .or_insert_with(HashMap::new)
          .insert(from, weight);
      }
    }

    // Compare manual calculation with get_inbound_edges results
    for &node in &nodes {
      let node_data = graph.get_node_data(node).unwrap();
      let inbound_edges: HashMap<NodeId, Weight> =
        node_data.get_inbound_edges().collect();

      let manual_edges =
        manual_inbound_edges.get(&node).cloned().unwrap_or_default();

      assert_eq!(inbound_edges, manual_edges, "Mismatch for node {}", node);
    }

    // Perform additional random modifications
    for _ in 0..20 {
      let from = nodes[rng.gen_range(0..nodes.len())];
      let to = nodes[rng.gen_range(0..nodes.len())];
      if from != to {
        let weight = rng.gen_range(-5.0..5.0);
        if weight != 0.0 {
          graph.set_edge(from, to, weight).unwrap();
        } else {
          let _ = graph.remove_edge(from, to);
        }
      }
    }

    // Re-check after modifications
    for &node in &nodes {
      let node_data = graph.get_node_data(node).unwrap();
      let inbound_edges: HashMap<NodeId, Weight> =
        node_data.get_inbound_edges().collect();

      let manual_edges: HashMap<NodeId, Weight> = nodes
        .iter()
        .filter_map(|&from| {
          graph
            .edge_weight(from, node)
            .unwrap()
            .map(|weight| (from, weight))
        })
        .collect();

      assert_eq!(
        inbound_edges, manual_edges,
        "Mismatch for node {} after modifications",
        node
      );
    }
  }
}
