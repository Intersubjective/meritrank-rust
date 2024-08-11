
#[allow(unused_imports)]
#[cfg(test)]


mod tests {
  use super::*;
  use indexmap::indexmap;
  use meritrank::graph::{NodeId, EdgeId};
  use meritrank::random_walk::RandomWalk;
  use meritrank::walk_storage::WalkStorage;
  use meritrank::{MeritRank, Graph, assert_approx_eq};

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
    for n in 1..9
    {
      rank_ref.get_new_nodeid();
      rank_ref.set_edge(n-1, n, 1.0);
      rank.get_new_nodeid();
      rank.set_edge(n-1, n, 1.0);

    }
    rank_ref.set_edge(8,1, 1.0);
    rank.set_edge(8,1, 1.0);
    rank_ref.calculate(0, walk_count).unwrap();
    println! ("{:?}", rank.get_ranks(0, None));
    for n in 1..8
    {
      let ref_score = rank_ref.get_node_score(0, n).unwrap() as f64;
      let score = rank.get_node_score(0, n).unwrap() as f64;
      assert_approx_eq!(ref_score, score , 0.1);
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
    for n in 1..9
    {
      rank_ref.get_new_nodeid();
      rank_ref.set_edge(n-1, n, 1.0);
      rank.get_new_nodeid();
      rank.set_edge(n-1, n, 1.0);

    }
    rank_ref.set_edge(8,1, 1.0);
    rank.set_edge(8,1, 1.0);
    let cloned = rank.clone();
    rank_ref.calculate(0, walk_count).unwrap();
    println! ("{:?}", cloned.get_ranks(0, None));
    for n in 1..8
    {
      let ref_score = rank_ref.get_node_score(0, n).unwrap() as f64;
      let score = cloned.get_node_score(0, n).unwrap() as f64;
      assert_approx_eq!(ref_score, score , 0.1);
    }
  }

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
    rank.set_edge(2, 1, 1.0);

    rank.calculate(0, walk_count).unwrap();


    //rank.print_walks();
    rank.set_edge(2, 0, 1.0);
    let ref_score = ref_rank.get_node_score(0, 2).unwrap() as f64;
    let score = rank.get_node_score(0, 2).unwrap() as f64;
    assert_approx_eq!(ref_score, score , 0.1);

    println! ("{:?}", rank.get_ranks(0, None));
    println! ("{:?}", ref_rank.get_ranks(0, None));
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
    assert_approx_eq!(ref_score, score , 0.1);

    println! ("{:?}", rank.get_ranks(0, None));
    println! ("{:?}", ref_rank.get_ranks(0, None));
    //rank.print_walks();
  }

}
