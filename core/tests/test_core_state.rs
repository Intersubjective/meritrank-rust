//! Core state bookkeeping of the service consistency track, phase 1
//! (SERVICE_CONSISTENCY_PLAN.md §2.3–2.4): injected RNG, lazy exact distributions, dirty egos,
//! the calculated set, memory release on eviction, deterministic tie order.

use meritrank_core::{Graph, MeritRank, NodeId};
use rand::rngs::StdRng;
use rand::SeedableRng;

const W: usize = 2_000;

fn ring(
  n: usize,
  rng: &mut StdRng,
) -> MeritRank {
  let mut mr = MeritRank::new(Graph::new(), W);
  for _ in 0..n {
    mr.get_new_nodeid();
  }
  for i in 0..n {
    mr.set_edge_with_rng(i, (i + 1) % n, 1.0, rng).unwrap();
    mr.set_edge_with_rng(i, (i + 3) % n, 0.5, rng).unwrap();
  }
  mr
}

fn scores(
  mr: &MeritRank,
  ego: NodeId,
) -> Vec<(NodeId, f64)> {
  mr.get_all_scores(ego, None).unwrap()
}

/// The same seed and the same operations give bit-identical scores; another seed does not.
#[test]
fn seeded_runs_are_identical() {
  let run = |seed: u64| {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut mr = ring(12, &mut rng);
    mr.calculate_with_rng(0, &mut rng).unwrap();
    mr.calculate_with_rng(5, &mut rng).unwrap();
    mr.set_edge_with_rng(3, 9, 2.0, &mut rng).unwrap();
    mr.set_edge_with_rng(0, 1, 0.0, &mut rng).unwrap();
    mr.set_edge_with_rng(7, 2, -1.0, &mut rng).unwrap();
    (scores(&mr, 0), scores(&mr, 5))
  };
  assert_eq!(run(42), run(42));
  assert_ne!(run(42), run(43));
}

/// Equal scores are ordered by node id, so result order does not depend on hash-set iteration.
#[test]
fn score_ties_ordered_by_id() {
  let mut mr = MeritRank::new(Graph::new(), W);
  for _ in 0..6 {
    mr.get_new_nodeid();
  }
  // A chain walked to its dead end on every walk (alpha = 1): all six nodes get the same count.
  mr.alpha = 1.0;
  for i in 1..6 {
    mr.set_edge(i - 1, i, 1.0).unwrap();
  }
  mr.calculate(0).unwrap();
  let s = scores(&mr, 0);
  assert_eq!(s.iter().map(|(id, _)| *id).collect::<Vec<_>>(), vec![0, 1, 2, 3, 4, 5]);
  for pair in s.windows(2) {
    let ((id1, s1), (id2, s2)) = (pair[0], pair[1]);
    assert!(s1 > s2 || (s1 == s2 && id1 < id2), "bad order: {:?}", s);
  }
}

/// A weight 2^54 times larger than another does not cancel the cached sum on removal: the
/// optimizer's invalidation probability for the next edge stays 1/2, not 1.
#[test]
fn sums_are_exact_after_removing_a_huge_weight() {
  let mut mr = MeritRank::new(Graph::new(), W);
  for _ in 0..4 {
    mr.get_new_nodeid();
  }
  let huge = 2f64.powi(54);
  mr.set_edge(0, 1, huge).unwrap();
  mr.set_edge(0, 2, 1.0).unwrap();
  mr.set_edge(0, 1, 0.0).unwrap();
  let data = mr.graph.get_node_data(0).unwrap();
  assert_eq!(data.pos_sum(), 1.0);
  assert_eq!(data.abs_sum(), 1.0);
}

/// Deleting an absent edge, or writing a weight at or below the deletion epsilon to it, is a
/// no-op rather than a panic.
#[test]
fn tiny_weight_on_absent_edge_is_noop() {
  let mut mr = MeritRank::new(Graph::new(), W);
  mr.get_new_nodeid();
  mr.get_new_nodeid();
  mr.set_edge(0, 1, 1e-7).unwrap();
  mr.set_edge(0, 1, -1e-6).unwrap();
  mr.set_edge(0, 1, 0.0).unwrap();
  assert_eq!(mr.graph.edge_weight(0, 1).unwrap(), None);
}

/// `calculate` marks the ego calculated and dirty; an edge change marks the egos whose walks it
/// repaired; a change at a node no walk visits marks nobody; `clear_ego` marks and uncalculates.
#[test]
fn dirty_egos_and_calculated_set() {
  let mut rng = StdRng::seed_from_u64(7);
  let mut mr = ring(12, &mut rng);
  let far = mr.get_new_nodeid(); // unreachable from the ring
  let far2 = mr.get_new_nodeid();

  assert!(!mr.is_calculated(0));
  mr.calculate_with_rng(0, &mut rng).unwrap();
  mr.calculate_with_rng(6, &mut rng).unwrap();
  assert!(mr.is_calculated(0) && mr.is_calculated(6));
  assert_eq!(mr.take_dirty_egos(), vec![0, 6]);
  assert_eq!(mr.take_dirty_egos(), Vec::<NodeId>::new());

  // Nobody walks through `far`.
  mr.set_edge_with_rng(far, far2, 1.0, &mut rng).unwrap();
  assert_eq!(mr.take_dirty_egos(), Vec::<NodeId>::new());

  // Deleting an edge on the ring repairs walks of both egos.
  mr.set_edge_with_rng(4, 5, 0.0, &mut rng).unwrap();
  assert_eq!(mr.take_dirty_egos(), vec![0, 6]);

  mr.clear_ego(6).unwrap();
  assert!(!mr.is_calculated(6));
  assert_eq!(mr.take_dirty_egos(), vec![6]);
  assert!(mr.get_all_scores(6, None).is_err());
}

/// An evicted ego's walk block is reused by the next ego instead of growing the storage.
#[test]
fn eviction_frees_and_reuses_walk_blocks() {
  let mut rng = StdRng::seed_from_u64(9);
  let mut mr = ring(12, &mut rng);
  mr.calculate_with_rng(0, &mut rng).unwrap();
  assert_eq!(mr.allocated_walks(), W);
  mr.clear_ego(0).unwrap();
  mr.calculate_with_rng(1, &mut rng).unwrap();
  assert_eq!(mr.allocated_walks(), W, "the freed block must be reused");
  mr.calculate_with_rng(0, &mut rng).unwrap();
  assert_eq!(mr.allocated_walks(), 2 * W);

  // Recalculating an evicted ego gives a normal frame.
  let s = scores(&mr, 0);
  assert!(s.iter().any(|(id, sc)| *id == 1 && *sc > 0.0));
}
