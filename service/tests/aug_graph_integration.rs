//! Integration tests for AugGraph: full-flow tests using shared helpers.

use meritrank_service::aug_graph::AugGraph;
use meritrank_service::data::{
  FilterOptions, GraphResult, OpReadGraph, OpReadMutualScores,
  OpReadNeighbors, OpReadNodeScore, OpReadScores, ScoreResult,
  NEIGHBORS_ALL, NEIGHBORS_INBOUND, NEIGHBORS_OUTBOUND,
};
use meritrank_service::node_registry::node_kind_from_prefix;
use meritrank_service::settings::Settings;

// ================================================================
// Shared test helpers (ported legacy tests)
// ================================================================

fn default_graph() -> AugGraph {
  AugGraph::new(Settings {
    num_walks:              50,
    zero_opinion_factor:    0.0,
    ..Settings::default()
  })
}

fn default_graph_zero() -> AugGraph {
  AugGraph::new(Settings {
    num_walks:              50,
    ..Settings::default()
  })
}

fn read_scores(
  graph: &AugGraph,
  ego: &str,
  kind_prefix: &str,
  hide_personal: bool,
  score_lt: f64,
  score_lte: bool,
  score_gt: f64,
  score_gte: bool,
  index: u32,
  count: u32,
) -> Vec<ScoreResult> {
  let node_kind = node_kind_from_prefix(kind_prefix);
  graph.read_scores(OpReadScores {
    ego:           ego.into(),
    score_options: FilterOptions {
      node_kind,
      hide_personal,
      score_lt,
      score_lte,
      score_gt,
      score_gte,
      index,
      count,
    },
  })
}

fn read_node_score_helper(
  graph: &AugGraph,
  ego: &str,
  target: &str,
) -> Vec<ScoreResult> {
  graph.read_node_score(OpReadNodeScore {
    ego:    ego.into(),
    target: target.into(),
  })
}

fn read_graph_helper(
  graph: &AugGraph,
  ego: &str,
  focus: &str,
  positive_only: bool,
  index: u64,
  count: u64,
) -> Vec<GraphResult> {
  graph.read_graph(OpReadGraph {
    ego: ego.into(),
    focus: focus.into(),
    positive_only,
    index,
    count,
  })
}

fn read_mutual_scores_helper(
  graph: &AugGraph,
  ego: &str,
) -> Vec<ScoreResult> {
  graph.read_mutual_scores(OpReadMutualScores {
    ego: ego.into(),
  })
}

fn read_neighbors_helper(
  graph: &AugGraph,
  ego: &str,
  focus: &str,
  direction: i64,
  kind_str: &str,
  hide_personal: bool,
  lt: f64,
  lte: bool,
  gt: f64,
  gte: bool,
  index: u32,
  count: u32,
) -> Vec<ScoreResult> {
  graph.read_neighbors(OpReadNeighbors {
    ego: ego.into(),
    focus: focus.into(),
    direction,
    kind: node_kind_from_prefix(kind_str),
    hide_personal,
    lt,
    lte,
    gt,
    gte,
    index,
    count,
  })
}

// --- Score tests ---

#[test]
fn scores_uncontexted() {
  let mut graph = AugGraph::new(Settings {
    num_walks:              500,
    zero_opinion_factor:    0.0,
    ..Settings::default()
  });

  graph.set_edge("U1".into(), "U2".into(), 2.0, 0);
  graph.set_edge("U1".into(), "U3".into(), 1.0, 0);
  graph.set_edge("U2".into(), "U3".into(), 3.0, 0);

  graph.calculate("U1".into());

  let res = read_scores(&graph, "U1", "U", false, 10.0, false, 0.0, false, 0, u32::MAX);

  assert_eq!(res.len(), 3);

  for x in &res {
    assert_eq!(x.ego, "U1");
    match x.target.as_str() {
      "U1" => {
        assert!(x.score > 0.2);
        assert!(x.score < 0.5);
      },
      "U2" => {
        assert!(x.score > 0.18);
        assert!(x.score < 0.5);
      },
      "U3" => {
        assert!(x.score > 0.2);
        assert!(x.score < 0.5);
      },
      _ => panic!("Unexpected target: {}", x.target),
    }
  }
}

#[test]
fn scores_reversed() {
  let mut graph = default_graph();

  graph.set_edge("U1".into(), "U2".into(), 2.0, 0);
  graph.set_edge("U1".into(), "U3".into(), 1.0, 0);
  graph.set_edge("U2".into(), "U3".into(), 3.0, 0);
  graph.set_edge("U2".into(), "U1".into(), 4.0, 0);
  graph.set_edge("U3".into(), "U1".into(), -5.0, 0);

  graph.calculate("U1".into());
  graph.calculate("U2".into());
  graph.calculate("U3".into());

  let res = read_scores(&graph, "U1", "U", false, 10.0, false, 0.0, false, 0, u32::MAX);

  assert!(res.len() >= 2);
  assert!(res.len() <= 3);

  for x in &res {
    assert_eq!(x.ego, "U1");
    match x.target.as_str() {
      "U1" => {
        assert!(x.score > 0.0);
        assert!(x.score < 0.4);
        assert!(x.reverse_score > 0.0);
        assert!(x.reverse_score < 0.4);
      },
      "U2" => {
        assert!(x.score > -0.1);
        assert!(x.score < 0.3);
        assert!(x.reverse_score > -0.3);
        assert!(x.reverse_score < 0.1);
      },
      "U3" => {
        assert!(x.score > -0.1);
        assert!(x.score < 0.3);
        assert!(x.reverse_score > -0.6);
        assert!(x.reverse_score < 0.0);
      },
      _ => panic!("Unexpected target: {}", x.target),
    }
  }
}

#[test]
fn scores_sort_order() {
  let mut graph = default_graph();

  graph.set_edge("U1".into(), "U2".into(), 2.0, 0);
  graph.set_edge("U1".into(), "U3".into(), 1.0, 0);
  graph.set_edge("U2".into(), "U3".into(), 3.0, 0);

  graph.calculate("U1".into());

  let res = read_scores(&graph, "U1", "U", false, 10.0, false, 0.0, false, 0, u32::MAX);

  assert!(res.len() > 1);
  for n in 1..res.len() {
    assert!(res[n - 1].score.abs() >= res[n].score.abs());
  }
}

#[test]
fn scores_without_recalculate() {
  let mut graph = default_graph();

  graph.set_edge("U1".into(), "U2".into(), 1.0, 0);
  graph.set_edge("U1".into(), "U0".into(), 1.0, 0);

  graph.calculate("U1".into());

  let res = read_scores(&graph, "U1", "U", true, 100.0, false, -100.0, false, 0, u32::MAX);
  let n = res.len();
  assert_eq!(n, 3);
}

#[test]
fn scores_self() {
  let mut graph = default_graph();

  graph.set_edge("B1".into(), "U1".into(), 3.0, 0);

  graph.calculate("U1".into());

  let res = read_scores(&graph, "U1", "U", false, 10.0, false, 0.0, false, 0, u32::MAX);

  assert_eq!(res.len(), 1);
  assert_eq!(res[0].ego, "U1");
  assert_eq!(res[0].target, "U1");
  assert!(res[0].score > 0.999);
  assert!(res[0].score < 1.001);
}

// --- Node score tests ---

#[test]
fn node_score_uncontexted() {
  let mut graph = default_graph();

  graph.set_edge("U1".into(), "U2".into(), 2.0, 0);
  graph.set_edge("U1".into(), "U3".into(), 1.0, 0);
  graph.set_edge("U3".into(), "U2".into(), 3.0, 0);

  graph.calculate("U1".into());

  let res = read_node_score_helper(&graph, "U1", "U2");

  assert_eq!(res.len(), 1);
  assert_eq!(res[0].ego, "U1");
  assert_eq!(res[0].target, "U2");
  assert!(res[0].score > 0.3);
  assert!(res[0].score < 0.45);
}

#[test]
fn node_score_reversed() {
  let mut graph = default_graph_zero();

  graph.set_edge("U1".into(), "U2".into(), 2.0, 0);
  graph.set_edge("U1".into(), "U3".into(), 1.0, 0);
  graph.set_edge("U3".into(), "U2".into(), 3.0, 0);
  graph.set_edge("U2".into(), "U1".into(), 4.0, 0);

  graph.calculate("U1".into());
  graph.calculate("U2".into());

  let res = read_node_score_helper(&graph, "U1", "U2");

  assert_eq!(res.len(), 1);
  assert_eq!(res[0].ego, "U1");
  assert_eq!(res[0].target, "U2");
  assert!(res[0].score > 0.2);
  assert!(res[0].score < 0.4);
  assert!(res[0].reverse_score > 0.2);
  assert!(res[0].reverse_score < 0.4);
}

// --- Mutual score tests ---

#[test]
fn mutual_scores_uncontexted() {
  let mut graph = default_graph_zero();

  graph.set_edge("U1".into(), "U2".into(), 3.0, 0);
  graph.set_edge("U1".into(), "U3".into(), 1.0, 0);
  graph.set_edge("U2".into(), "U1".into(), 2.0, 0);
  graph.set_edge("U2".into(), "U3".into(), 4.0, 0);
  graph.set_edge("U3".into(), "U1".into(), 3.0, 0);
  graph.set_edge("U3".into(), "U2".into(), 2.0, 0);

  graph.calculate("U1".into());
  graph.calculate("U2".into());
  graph.calculate("U3".into());

  let res = read_mutual_scores_helper(&graph, "U1");

  assert_eq!(res.len(), 3);

  let mut u1 = true;
  let mut u2 = true;
  let mut u3 = true;

  for x in &res {
    assert_eq!(x.ego, "U1");
    match x.target.as_str() {
      "U1" => {
        assert!(x.score > 0.25);
        assert!(x.score < 0.5);
        assert!(x.reverse_score > 0.25);
        assert!(x.reverse_score < 0.5);
        assert!(u1);
        u1 = false;
      },
      "U2" => {
        assert!(x.score > 0.15);
        assert!(x.score < 0.35);
        assert!(x.reverse_score > 0.15);
        assert!(x.reverse_score < 0.35);
        assert!(u2);
        u2 = false;
      },
      "U3" => {
        assert!(x.score > 0.15);
        assert!(x.score < 0.35);
        assert!(x.reverse_score > 0.15);
        assert!(x.reverse_score < 0.35);
        assert!(u3);
        u3 = false;
      },
      _ => panic!("Unexpected target"),
    }
  }
}

#[test]
fn mutual_scores_self() {
  let mut graph = default_graph();

  graph.set_edge("U1".into(), "U2".into(), 3.0, 0);

  let ego_id = graph.nodes.get_by_name("U1").unwrap().id;
  let dst_id = graph.nodes.get_by_name("U2").unwrap().id;
  graph.mr.set_edge(ego_id, dst_id, 0.0).unwrap();

  graph.calculate("U1".into());

  let res = read_mutual_scores_helper(&graph, "U1");

  assert_eq!(res.len(), 1);
  assert_eq!(res[0].ego, "U1");
  assert_eq!(res[0].target, "U1");
  assert!(res[0].score > 0.99);
  assert!(res[0].score < 1.01);
  assert!(res[0].reverse_score > 0.99);
  assert!(res[0].reverse_score < 1.01);
}

#[test]
fn mutual_scores_cluster_single_score_uncontexted() {
  let mut graph = default_graph();

  graph.set_edge("U1".into(), "U2".into(), 10.0, 0);

  graph.calculate("U1".into());

  let res = read_mutual_scores_helper(&graph, "U1");

  assert_eq!(res.len(), 2);
  assert_eq!(res[0].cluster, 100);
  assert_eq!(res[1].cluster, 1);
}

// --- Graph tests ---

#[test]
fn graph_uncontexted() {
  let mut graph = default_graph();

  graph.set_edge("U1".into(), "U2".into(), 2.0, 0);
  graph.set_edge("U1".into(), "U3".into(), 1.0, 0);
  graph.set_edge("U2".into(), "U3".into(), 3.0, 0);

  let res = read_graph_helper(&graph, "U1", "U2", false, 0, 10000);

  assert_eq!(res.len(), 2);

  let mut has_u1 = false;
  let mut has_u2 = false;

  for x in &res {
    match x.src.as_str() {
      "U1" => {
        assert_eq!(x.dst, "U2");
        assert!(x.weight > 0.65);
        assert!(x.weight < 0.67);
        has_u1 = true;
      },
      "U2" => {
        assert_eq!(x.dst, "U3");
        assert!(x.weight > 0.99);
        assert!(x.weight < 1.01);
        has_u2 = true;
      },
      _ => panic!("Unexpected src: {}", x.src),
    }
  }

  assert!(has_u1);
  assert!(has_u2);
}

#[test]
fn graph_reversed() {
  let mut graph = default_graph_zero();

  graph.set_edge("U1".into(), "U2".into(), 2.0, 0);
  graph.set_edge("U1".into(), "U3".into(), 1.0, 0);
  graph.set_edge("U2".into(), "U3".into(), 3.0, 0);
  graph.set_edge("U2".into(), "U1".into(), 4.0, 0);

  graph.calculate("U1".into());

  let res = read_graph_helper(&graph, "U1", "U2", false, 0, 10000);

  assert_eq!(res.len(), 3);

  for x in &res {
    match x.src.as_str() {
      "U1" => {
        assert_eq!(x.dst, "U2");
        assert!(x.weight > 0.6);
        assert!(x.weight < 0.7);
        assert!(x.score > 0.05);
        assert!(x.score < 0.4);
      },
      "U2" => {
        if x.dst == "U1" {
          assert!(x.weight > 0.5);
          assert!(x.weight < 0.6);
          assert!(x.score > 0.2);
          assert!(x.score < 0.5);
        }
        if x.dst == "U3" {
          assert!(x.weight > 0.39);
          assert!(x.weight < 0.49);
          assert!(x.score > 0.16);
          assert!(x.score < 0.4);
        }
      },
      _ => panic!("Unexpected src: {}", x.src),
    }
  }
}

#[test]
fn graph_sort_order() {
  let mut graph = default_graph();

  graph.set_edge("U1".into(), "U2".into(), 2.0, 0);
  graph.set_edge("U1".into(), "U3".into(), 1.0, 0);
  graph.set_edge("U2".into(), "U3".into(), 3.0, 0);

  let res = read_graph_helper(&graph, "U1", "U2", false, 0, 10000);

  assert!(res.len() > 1);
  assert_eq!(res[0].src, "U1");
  assert_eq!(res[0].dst, "U2");
  for n in 2..res.len() {
    assert!(
      res[n - 1].weight.abs() >= res[n].weight.abs(),
      "focus neighbors must be sorted by weight descending"
    );
  }
}

#[test]
fn graph_path_visible_with_low_count_and_focus_neighbors_sorted() {
  let mut graph = default_graph();

  graph.set_edge("U0".into(), "U1".into(), 5.0, 0);
  graph.set_edge("U1".into(), "U2".into(), 4.0, 0);
  graph.set_edge("U2".into(), "U3".into(), 3.0, 0);
  graph.set_edge("U3".into(), "U4".into(), 2.0, 0);
  graph.set_edge("U3".into(), "U5".into(), 4.0, 0);
  graph.set_edge("U3".into(), "U6".into(), 8.0, 0);

  let res = read_graph_helper(&graph, "U0", "U3", false, 0, 2);

  assert_eq!(res.len(), 3, "full path must appear even with count=2");
  assert_eq!(res[0].src, "U0");
  assert_eq!(res[0].dst, "U1");
  assert_eq!(res[1].src, "U1");
  assert_eq!(res[1].dst, "U2");
  assert_eq!(res[2].src, "U2");
  assert_eq!(res[2].dst, "U3");

  let res = read_graph_helper(&graph, "U0", "U3", false, 0, 10);

  let focus_idx = res.iter().position(|x| x.src == "U3").unwrap();
  for i in focus_idx + 1..res.len().saturating_sub(1) {
    assert!(
      res[i].weight >= res[i + 1].weight,
      "focus neighbors must be sorted by weight descending: {} -> {} ({}), {} -> {} ({})",
      res[i].src,
      res[i].dst,
      res[i].weight,
      res[i + 1].src,
      res[i + 1].dst,
      res[i + 1].weight
    );
  }
}

#[test]
fn graph_empty() {
  let mut graph = default_graph();

  graph.set_edge("U1".into(), "U2".into(), 2.0, 0);
  graph.set_edge("U1".into(), "U3".into(), 1.0, 0);
  graph.set_edge("U2".into(), "U3".into(), 3.0, 0);

  let ego_id = graph.nodes.get_by_name("U1").unwrap().id;
  let u2_id = graph.nodes.get_by_name("U2").unwrap().id;
  let u3_id = graph.nodes.get_by_name("U3").unwrap().id;
  graph.mr.set_edge(ego_id, u2_id, 0.0).unwrap();
  graph.mr.set_edge(ego_id, u3_id, 0.0).unwrap();
  graph.mr.set_edge(u2_id, u3_id, 0.0).unwrap();

  let res = read_graph_helper(&graph, "U1", "U2", false, 0, 10000);
  assert_eq!(res.len(), 0);
}

#[test]
fn graph_no_direct_connectivity() {
  let mut graph = default_graph();

  graph.set_edge("U1".into(), "B1".into(), 1.0, 0);
  graph.set_edge("U2".into(), "U3".into(), 1.0, 0);
  graph.set_edge("U2".into(), "B2".into(), 1.0, 0);

  let res = read_graph_helper(&graph, "U1", "U2", false, 0, 10000);

  assert_eq!(res.len(), 1);
  assert_eq!(res[0].src, "U2");
  assert_eq!(res[0].dst, "U3");
}

#[test]
fn graph_force_connectivity() {
  let mut graph = AugGraph::new(Settings {
    num_walks:              100,
    force_read_graph_conn:  true,
    ..Settings::default()
  });

  graph.set_edge("U1".into(), "B1".into(), 1.0, 0);
  graph.set_edge("U2".into(), "U3".into(), 1.0, 0);
  graph.set_edge("U2".into(), "B2".into(), 1.0, 0);

  let res = read_graph_helper(&graph, "U1", "U2", false, 0, 10000);

  assert_eq!(res.len(), 2);
  assert_eq!(res[0].src, "U1");
  assert_eq!(res[0].dst, "U2");
  assert_eq!(res[1].src, "U2");
  assert_eq!(res[1].dst, "U3");
}

// --- Clustering tests ---

#[test]
fn five_user_scores_clustering() {
  let mut graph = default_graph();

  graph.set_edge("U1".into(), "U2".into(), 5.0, 0);
  graph.set_edge("U1".into(), "U3".into(), 2.5, 0);
  graph.set_edge("U1".into(), "U4".into(), 2.0, 0);
  graph.set_edge("U1".into(), "U5".into(), 3.0, 0);
  graph.set_edge("U2".into(), "U1".into(), 4.0, 0);

  graph.calculate("U1".into());

  let res = read_scores(&graph, "U1", "", true, 100.0, false, -100.0, false, 0, u32::MAX);

  assert_eq!(res.len(), 5);

  assert!(res[0].cluster <= 100);
  assert!(res[0].cluster >= 40);

  assert!(res[1].cluster <= 100);
  assert!(res[1].cluster >= 20);

  assert!(res[2].cluster <= 100);
  assert!(res[2].cluster >= 1);

  assert!(res[3].cluster <= 80);
  assert!(res[3].cluster >= 1);

  assert!(res[4].cluster <= 60);
  assert!(res[4].cluster >= 1);
}

#[test]
fn five_beacon_scores_clustering() {
  let mut graph = AugGraph::new(Settings {
    num_walks: 500,
    ..Settings::default()
  });

  graph.set_edge("U1".into(), "B2".into(), 5.0, 0);
  graph.set_edge("U1".into(), "B3".into(), 1.0, 0);
  graph.set_edge("U1".into(), "B4".into(), 2.0, 0);
  graph.set_edge("U1".into(), "B5".into(), 3.0, 0);
  graph.set_edge("U1".into(), "B6".into(), 4.0, 0);

  graph.calculate("U1".into());

  let res = read_scores(&graph, "U1", "B", true, 100.0, false, -100.0, false, 0, u32::MAX);

  assert_eq!(res.len(), 5);

  assert!(res[0].cluster <= 100);
  assert!(res[0].cluster >= 40);

  assert!(res[1].cluster <= 100);
  assert!(res[1].cluster >= 20);

  assert!(res[2].cluster <= 100);
  assert!(res[2].cluster >= 1);

  assert!(res[3].cluster <= 80);
  assert!(res[3].cluster >= 1);

  assert!(res[4].cluster <= 60);
  assert!(res[4].cluster >= 1);
}

#[test]
fn three_scores_chain_clustering() {
  let mut graph = default_graph();

  graph.set_edge("U1".into(), "U2".into(), 2.0, 0);
  graph.set_edge("U2".into(), "U3".into(), 3.0, 0);
  graph.set_edge("U3".into(), "U1".into(), 4.0, 0);

  graph.calculate("U1".into());

  let res = read_scores(&graph, "U1", "", true, 100.0, false, -100.0, false, 0, u32::MAX);

  assert_eq!(res.len(), 3);

  assert!(res[0].cluster <= 100);
  assert!(res[0].cluster >= 40);

  assert!(res[1].cluster <= 80);
  assert!(res[1].cluster >= 20);

  assert!(res[2].cluster <= 60);
  assert!(res[2].cluster >= 1);
}

#[test]
fn separate_clusters_without_users() {
  let mut graph = default_graph();

  graph.set_edge("U1".into(), "B1".into(), 3.0, 0);
  graph.set_edge("U1".into(), "C1".into(), 4.0, 0);

  graph.calculate("U1".into());

  let res = read_scores(&graph, "U1", "", true, 100.0, false, -100.0, false, 0, u32::MAX);

  assert_eq!(res.len(), 3);

  assert_eq!(res[0].cluster, 100);
  assert_eq!(res[1].cluster, 100);
  assert_eq!(res[2].cluster, 100);
}

#[test]
fn separate_clusters_self_score() {
  let mut graph = default_graph();

  graph.set_edge("U1".into(), "U2".into(), 2.0, 0);
  graph.set_edge("U1".into(), "B1".into(), 3.0, 0);
  graph.set_edge("U1".into(), "C1".into(), 4.0, 0);

  graph.calculate("U1".into());

  let res = read_scores(&graph, "U1", "U", true, 100.0, false, -100.0, false, 0, u32::MAX);

  assert_eq!(res.len(), 2);

  assert_eq!(res[0].cluster, 100);
  assert_eq!(res[1].cluster, 1);
}

// --- Neighbor tests ---

#[test]
fn neighbors_all() {
  let mut graph = default_graph();
  graph.set_edge("U1".into(), "U2".into(), 1.0, 0);
  graph.set_edge("U2".into(), "U3".into(), 2.0, 0);
  graph.set_edge("U3".into(), "U1".into(), 3.0, 0);

  let neighbors = read_neighbors_helper(
    &graph, "U1", "U2", NEIGHBORS_ALL, "", false, 100.0, false, -100.0,
    false, 0, 100,
  );

  assert_eq!(neighbors.len(), 2);
  let targets: Vec<&str> = neighbors.iter().map(|n| n.target.as_str()).collect();
  assert!(targets.contains(&"U1"));
  assert!(targets.contains(&"U3"));
}

#[test]
fn neighbors_inbound() {
  let mut graph = default_graph();
  graph.set_edge("U1".into(), "U2".into(), 1.0, 0);
  graph.set_edge("U2".into(), "U3".into(), 2.0, 0);
  graph.set_edge("U3".into(), "U1".into(), 3.0, 0);

  let neighbors = read_neighbors_helper(
    &graph, "U1", "U2", NEIGHBORS_INBOUND, "", false, 100.0, false, -100.0,
    false, 0, 100,
  );

  assert_eq!(neighbors.len(), 1);
  assert_eq!(neighbors[0].target, "U1");
}

#[test]
fn neighbors_outbound() {
  let mut graph = default_graph();
  graph.set_edge("U1".into(), "U2".into(), 1.0, 0);
  graph.set_edge("U2".into(), "U3".into(), 2.0, 0);
  graph.set_edge("U3".into(), "U1".into(), 3.0, 0);

  let neighbors = read_neighbors_helper(
    &graph, "U1", "U2", NEIGHBORS_OUTBOUND, "", false, 100.0, false, -100.0,
    false, 0, 100,
  );

  assert_eq!(neighbors.len(), 1);
  assert_eq!(neighbors[0].target, "U3");
}

#[test]
fn neighbors_non_ego_score() {
  let mut graph = AugGraph::new(Settings {
    num_walks:              500,
    zero_opinion_factor:    0.0,
    ..Settings::default()
  });

  graph.set_edge("U1".into(), "U2".into(), 1.0, 0);
  graph.set_edge("U2".into(), "U3".into(), 1.0, 0);
  graph.set_edge("U1".into(), "U4".into(), 1.0, 0);
  graph.set_edge("U4".into(), "U3".into(), 1.0, 0);
  graph.set_edge("U3".into(), "U4".into(), 1.0, 0);

  let neighbors = read_neighbors_helper(
    &graph, "U1", "U3", NEIGHBORS_INBOUND, "", false, 100.0, false, -100.0,
    false, 0, 100,
  );

  assert_eq!(neighbors.len(), 2);
  let targets: Vec<&str> = neighbors.iter().map(|n| n.target.as_str()).collect();
  assert!(targets.contains(&"U2"));
  assert!(targets.contains(&"U4"));
  for n in &neighbors {
    assert_eq!(n.ego, "U1");
    assert!(n.score > 0.0, "Scores should be calculated from ego's standpoint");
  }
}

#[test]
fn neighbors_prioritize_ego_owned_objects() {
  let mut graph = default_graph();

  graph.set_edge("O1".into(), "U1".into(), 1.0, 0);
  graph.set_edge("U1".into(), "U2".into(), 100.0, 0);
  graph.set_edge("U2".into(), "U3".into(), 1.0, 0);
  graph.set_edge("U1".into(), "O1".into(), 1.0, 0);
  graph.set_edge("O1".into(), "U3".into(), 1.0, 0);

  let neighbors = read_neighbors_helper(
    &graph, "U1", "U3", NEIGHBORS_INBOUND, "", false, 100.0, false, -100.0,
    false, 0, 100,
  );

  assert_eq!(neighbors[0].ego, "U1");
  assert_eq!(neighbors[0].target, "O1");
  assert_eq!(neighbors[1].ego, "U1");
  assert_eq!(neighbors[1].target, "U2");
}

#[test]
fn neighbors_omit_opinions_from_self_to_focus() {
  let mut graph = default_graph();

  graph.set_edge("U1".into(), "U2".into(), 1.0, 0);

  graph.set_edge("O2".into(), "U2".into(), 1.0, 0);
  graph.set_edge("U2".into(), "O2".into(), 1.0, 0);
  graph.set_edge("O2".into(), "U3".into(), 1.0, 0);

  graph.set_edge("O3".into(), "U3".into(), 1.0, 0);
  graph.set_edge("U3".into(), "O3".into(), 1.0, 0);
  graph.set_edge("O3".into(), "U1".into(), 1.0, 0);

  let neighbors = read_neighbors_helper(
    &graph, "U1", "U3", NEIGHBORS_INBOUND, "O", false, 100.0, false, -100.0,
    false, 0, 100,
  );

  assert_eq!(neighbors[0].ego, "U1");
  assert_eq!(neighbors[0].target, "O2");
  assert_eq!(neighbors.len(), 1);
}

#[test]
fn neighbors_opinions_on_ego() {
  let mut graph = default_graph();

  graph.set_edge("U1".into(), "O12".into(), 1.0, 0);
  graph.set_edge("O12".into(), "U1".into(), 1.0, 0);
  graph.set_edge("O12".into(), "U2".into(), 1.0, 0);

  graph.set_edge("U1".into(), "O13".into(), 1.0, 0);
  graph.set_edge("O13".into(), "U1".into(), 1.0, 0);
  graph.set_edge("O13".into(), "U3".into(), 1.0, 0);

  graph.set_edge("U3".into(), "O31".into(), 1.0, 0);
  graph.set_edge("O31".into(), "U3".into(), 1.0, 0);
  graph.set_edge("O31".into(), "U1".into(), 1.0, 0);

  let neighbors = read_neighbors_helper(
    &graph,
    "U1",
    "U1",
    NEIGHBORS_INBOUND,
    "O",
    false,
    100.0,
    false,
    -100.0,
    false,
    0,
    100,
  );

  assert_eq!(neighbors[0].ego, "U1");
  assert_eq!(neighbors[0].target, "O31");
  assert_eq!(neighbors.len(), 1);
}

// --- Zero opinion tests ---

#[test]
fn set_zero_opinion_uncontexted() {
  let mut graph = default_graph_zero();
  graph.set_edge("U1".into(), "U2".into(), -5.0, 0);
  graph.calculate("U1".into());
  let s0 = read_node_score_helper(&graph, "U1", "U2")[0].score;

  let u2_id = graph.nodes.get_by_name("U2").unwrap().id;
  if u2_id >= graph.zero_opinion.len() {
    graph.zero_opinion.resize(u2_id + 1, 0.0);
  }
  graph.zero_opinion[u2_id] = 10.0;

  graph.calculate("U1".into());

  let s1 = read_node_score_helper(&graph, "U1", "U2")[0].score;

  assert_ne!(s0, s1);
}

// --- VSIDS tests ---

#[test]
fn vsids_write_edge() {
  let mut graph = AugGraph::new(Settings {
    num_walks: 500,
    ..Settings::default()
  });

  graph.set_edge("U1".into(), "U4".into(), 3.0, 0);
  graph.set_edge("U1".into(), "U2".into(), 3.0, 0);
  graph.set_edge("U1".into(), "U3".into(), 1.0, 20);

  graph.calculate("U1".into());

  let u12 = read_node_score_helper(&graph, "U1", "U2");
  let u13 = read_node_score_helper(&graph, "U1", "U3");

  assert!(
    u12[0].score < u13[0].score,
    "Assert that thanks to magnitude, U3 has a higher score than U2"
  );

  graph.set_edge("U1".into(), "U4".into(), 1.0, 200);
  graph.calculate("U1".into());
  let u12_final = read_node_score_helper(&graph, "U1", "U2");
  let u13_final = read_node_score_helper(&graph, "U1", "U3");
  assert!(
    u12_final.is_empty() || u12_final[0].score == 0.0,
    "U1->U2 edge should not exist"
  );
  assert!(
    u13_final.is_empty() || u13_final[0].score == 0.0,
    "U1->U3 edge should not exist"
  );
}

// --- omit_neg_edges_scores setting ---

#[test]
fn omit_neg_edges_scores_setting() {
  let mut graph_omit = AugGraph::new(Settings {
    num_walks:              50,
    omit_neg_edges_scores:  true,
    ..Settings::default()
  });

  let mut graph_include = AugGraph::new(Settings {
    num_walks:              50,
    omit_neg_edges_scores:  false,
    ..Settings::default()
  });

  let edges = vec![
    ("U1", "U2", -1.0),
    ("U1", "U3", 10.0),
    ("U3", "U2", 1.0),
  ];

  // Set zero opinion for U2
  {
    graph_include.set_edge("U1".into(), "U2".into(), 1.0, 0);
    let u2_id = graph_include.nodes.get_by_name("U2").unwrap().id;
    if u2_id >= graph_include.zero_opinion.len() {
      graph_include.zero_opinion.resize(u2_id + 1, 0.0);
    }
    graph_include.zero_opinion[u2_id] = 10.0;
    graph_include.mr.set_edge(
      graph_include.nodes.get_by_name("U1").unwrap().id,
      u2_id,
      0.0,
    ).unwrap();
  }
  {
    graph_omit.set_edge("U1".into(), "U2".into(), 1.0, 0);
    let u2_id = graph_omit.nodes.get_by_name("U2").unwrap().id;
    if u2_id >= graph_omit.zero_opinion.len() {
      graph_omit.zero_opinion.resize(u2_id + 1, 0.0);
    }
    graph_omit.zero_opinion[u2_id] = 10.0;
    graph_omit.mr.set_edge(
      graph_omit.nodes.get_by_name("U1").unwrap().id,
      u2_id,
      0.0,
    ).unwrap();
  }

  for (src, dst, weight) in edges {
    graph_omit.set_edge(src.into(), dst.into(), weight, 0);
    graph_include.set_edge(src.into(), dst.into(), weight, 0);
  }

  graph_omit.calculate("U1".into());
  graph_include.calculate("U1".into());

  let scores_omit = read_scores(
    &graph_omit, "U1", "U", false, 100.0, false, -100.0, false, 0, u32::MAX,
  );

  let scores_include = read_scores(
    &graph_include, "U1", "U", false, 100.0, false, -100.0, false, 0, u32::MAX,
  );

  let find_node_score = |scores: &[ScoreResult], node: &str| -> Option<f64> {
    scores.iter().find(|s| s.target == node).map(|s| s.score)
  };

  let u2_score_include = find_node_score(&scores_include, "U2");
  let u2_score_omit = find_node_score(&scores_omit, "U2");

  assert!(
    u2_score_include.is_some(),
    "U2 should have a score when negative edges are included"
  );
  assert!(
    u2_score_omit.is_none(),
    "U2 should not have a score when negative edges are omitted"
  );
}

/// omit_neg_edges_scores with read_mutual_scores: nodes with a direct negative
/// edge from ego must be excluded when the setting is true.
#[test]
fn omit_neg_edges_scores_mutual_scores() {
  let mut graph_omit = AugGraph::new(Settings {
    num_walks:              200,
    omit_neg_edges_scores:  true,
    ..Settings::default()
  });
  let mut graph_include = AugGraph::new(Settings {
    num_walks:              200,
    omit_neg_edges_scores:  false,
    ..Settings::default()
  });

  // Ego U1: negative edge to U2, positive to U3. U3->U2 so U2 is reachable.
  let edges = vec![
    ("U1", "U2", -1.0),
    ("U1", "U3", 1.0),
    ("U3", "U2", 1.0),
  ];
  for (src, dst, weight) in &edges {
    graph_omit.set_edge((*src).into(), (*dst).into(), *weight, 0);
    graph_include.set_edge((*src).into(), (*dst).into(), *weight, 0);
  }

  graph_omit.calculate("U1".into());
  graph_include.calculate("U1".into());

  let mutual_omit = read_mutual_scores_helper(&graph_omit, "U1");
  let mutual_include = read_mutual_scores_helper(&graph_include, "U1");

  let targets_omit: std::collections::HashSet<_> =
    mutual_omit.iter().map(|s| s.target.as_str()).collect();
  let targets_include: std::collections::HashSet<_> =
    mutual_include.iter().map(|s| s.target.as_str()).collect();

  assert!(
    !targets_omit.contains("U2"),
    "With omit_neg_edges_scores=true, U2 (negative edge from ego) must not appear in mutual_scores"
  );
  assert!(
    targets_omit.contains("U3"),
    "With omit_neg_edges_scores=true, U3 (positive edge from ego) must appear in mutual_scores"
  );
  assert!(
    targets_include.contains("U3"),
    "With omit_neg_edges_scores=false, U3 must appear in mutual_scores"
  );
  // With omit_neg=false, U2 may still not appear if its score is <= 0 (we only show positive forward scores)
}
