//! Adversarial distribution tests for optimized incremental invalidation.
//!
//! All observations are unique per-walk hits, so each individual (ego, node,
//! segment) count is binomial. Compare independent incremental/fresh samples
//! with a pooled two-sample z test, rejecting at |z| >= 5. Walk RNGs are not
//! seedable through the public API; only fuzzer graph/edit generation is seeded.
//! Written by an adversarial agent (codex GPT-6 Astra); the defects it found
//! (stale `negative_segment_start` after recalculation, cancelling cached sums,
//! panic on a tiny weight to an absent edge) are fixed, so these are regression
//! tests now. Slow: run with `--features expensive_tests`.
#![cfg(feature = "expensive_tests")]
use meritrank_core::{Graph, MeritRank, NodeId, Weight};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::sync::Mutex;

const W: usize = 200_000;
const ALPHA: f64 = 0.85;
const Z_FAIL: f64 = 5.0;
// Bound peak memory even with the default parallel Rust test runner.
static SERIAL: Mutex<()> = Mutex::new(());
type Edge = (NodeId, NodeId, Weight);

#[derive(Clone, Copy, Debug)]
enum Op {
  Edge(NodeId, NodeId, Weight),
  Calc(NodeId),
}

fn build(
  n: usize,
  edges: &[Edge],
) -> MeritRank {
  let mut mr = MeritRank::new(Graph::new(), W);
  mr.alpha = ALPHA;
  for _ in 0..n {
    mr.get_new_nodeid();
  }
  for &(s, d, w) in edges {
    mr.set_edge(s, d, w).unwrap();
  }
  mr
}

fn apply_model(
  edges: &mut Vec<Edge>,
  s: NodeId,
  d: NodeId,
  w: Weight,
) {
  edges.retain(|&(a, b, _)| (a, b) != (s, d));
  if w.abs() > 1e-6 {
    edges.push((s, d, w));
  }
}

fn frequency(
  mr: &MeritRank,
  ego: NodeId,
  node: NodeId,
  negative: bool,
) -> f64 {
  let hits = if negative {
    mr.get_negative_hits()
  } else {
    mr.get_personal_hits()
  };
  hits.get(&ego).map_or(0, |c| c.get_count(&node)) as f64 / W as f64
}

fn z_two(
  a: f64,
  b: f64,
) -> f64 {
  let pooled = (a + b) / 2.0;
  let se = (2.0 * pooled * (1.0 - pooled) / W as f64).sqrt();
  if se == 0.0 {
    if a == b {
      0.0
    } else {
      (a - b).signum() * f64::INFINITY
    }
  } else {
    (a - b) / se
  }
}

#[derive(Debug, Default)]
struct Observation {
  z: f64,
  ego: NodeId,
  node: NodeId,
  negative: bool,
  inc: f64,
  fresh: f64,
}

fn compare(
  name: &str,
  inc: &MeritRank,
  fresh: &MeritRank,
  egos: &[NodeId],
  n: usize,
) -> Observation {
  let mut worst = Observation::default();
  for &ego in egos {
    for node in 0..n {
      for negative in [false, true] {
        let a = frequency(inc, ego, node, negative);
        let b = frequency(fresh, ego, node, negative);
        let z = z_two(a, b);
        if z.abs() > worst.z.abs() {
          worst = Observation {
            z,
            ego,
            node,
            negative,
            inc: a,
            fresh: b,
          };
        }
        if z.abs() >= Z_FAIL {
          println!("{name}: ego={ego} node={node} segment={} inc={a:.6} fresh={b:.6} z={z:+.3} score_inc={:.6} score_fresh={:.6}",
                        if negative { "neg" } else { "pos" },
                        inc.get_node_score(ego, node).unwrap(), fresh.get_node_score(ego, node).unwrap());
        }
      }
    }
  }
  println!(
    "{name}: worst z={:+.3} ego={} node={} segment={} inc={:.6} fresh={:.6}",
    worst.z,
    worst.ego,
    worst.node,
    if worst.negative { "neg" } else { "pos" },
    worst.inc,
    worst.fresh
  );
  worst
}

fn scenario(
  name: &str,
  n: usize,
  initial: &[Edge],
  egos: &[NodeId],
  ops: &[Op],
) -> Observation {
  let mut inc = build(n, initial);
  let mut all_egos = egos.to_vec();
  for &ego in egos {
    inc.calculate(ego).unwrap();
  }
  let mut final_edges = initial.to_vec();
  for (i, &op) in ops.iter().enumerate() {
    let result = match op {
      Op::Edge(s, d, w) => {
        apply_model(&mut final_edges, s, d, w);
        inc.set_edge(s, d, w)
      },
      Op::Calc(ego) => {
        if !all_egos.contains(&ego) {
          all_egos.push(ego);
        }
        inc.calculate(ego)
      },
    };
    assert!(result.is_ok(), "{name}: op[{i}]={op:?}: {result:?}");
  }
  // Rebuild from the canonical final edge list, not a clone of incremental
  // Graph: cloning could carry corrupted sums/caches into the reference.
  let mut fresh = build(n, &final_edges);
  for &ego in &all_egos {
    fresh.calculate(ego).unwrap();
  }
  compare(name, &inc, &fresh, &all_egos, n)
}

fn check(
  name: &str,
  n: usize,
  initial: &[Edge],
  egos: &[NodeId],
  ops: &[Op],
) {
  let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
  let obs = scenario(name, n, initial, egos, ops);
  assert!(obs.z.abs() < Z_FAIL, "{name}: {obs:?}");
}

fn repeated(
  name: &str,
  n: usize,
  initial: &[Edge],
  egos: &[NodeId],
  ops: &[Op],
) {
  let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
  let observations: Vec<_> = (1..=3)
    .map(|trial| {
      scenario(&format!("{name} trial={trial}"), n, initial, egos, ops)
    })
    .collect();
  assert!(
    observations.iter().all(|o| o.z.abs() < Z_FAIL),
    "{name}: {observations:?}"
  );
}

#[test]
fn negative_control_detects_wrong_signed_distribution() {
  let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
  let mut inc = build(3, &[(0, 1, -1.0), (0, 2, 1.0)]);
  let mut deliberately_wrong = build(3, &[(0, 1, -1.3), (0, 2, 1.0)]);
  inc.calculate(0).unwrap();
  deliberately_wrong.calculate(0).unwrap();
  // Require sensitivity separately for both observable segments.
  for (node, negative) in [(1, true), (2, false)] {
    let z = z_two(
      frequency(&inc, 0, node, negative),
      frequency(&deliberately_wrong, 0, node, negative),
    );
    println!("negative control node={node} negative={negative}: z={z:+.3}");
    assert!(z.abs() >= Z_FAIL);
  }
}

#[test]
fn signed_boundary_additions_before_at_after() {
  for (name, op) in [
    ("before negative boundary", Op::Edge(1, 5, -2.0)),
    ("at negative boundary positive", Op::Edge(2, 5, 2.0)),
    ("at negative boundary negative", Op::Edge(2, 5, -2.0)),
    ("after negative boundary positive", Op::Edge(3, 5, 2.0)),
    ("after negative boundary negative", Op::Edge(3, 5, -2.0)),
  ] {
    check(
      name,
      6,
      &[(0, 1, 1.0), (1, 2, -1.0), (2, 3, 1.0), (3, 4, 1.0)],
      &[0],
      &[op],
    );
  }
}

#[test]
fn signed_deletions_force_correct_regime() {
  for (name, initial, op) in [
    (
      "remove entering negative edge",
      vec![(0, 1, -1.0), (0, 2, 1.0), (1, 3, 1.0)],
      Op::Edge(0, 1, 0.0),
    ),
    (
      "force another negative edge",
      vec![(0, 1, -1.0), (0, 2, -1.0), (2, 3, 1.0)],
      Op::Edge(0, 1, 0.0),
    ),
    (
      "negative mode cannot force negative edge",
      vec![(0, 1, -1.0), (1, 2, 1.0), (1, 3, -1.0)],
      Op::Edge(1, 2, 0.0),
    ),
    (
      "force positive edge in negative mode",
      vec![(0, 1, -1.0), (1, 2, 1.0), (1, 3, 1.0)],
      Op::Edge(1, 2, 0.0),
    ),
  ] {
    check(name, 4, &initial, &[0], &[op]);
  }
}

#[test]
fn sign_flips_and_same_weight_on_signed_cycle() {
  check(
    "signed cycle sign flips and no-op weights",
    4,
    &[
      (0, 1, 1.0),
      (1, 2, -1.0),
      (2, 0, 1.0),
      (1, 3, 2.0),
      (3, 1, 1.0),
    ],
    &[0, 2],
    &[
      Op::Edge(1, 2, 1.0),
      Op::Edge(1, 2, -1.0),
      Op::Edge(0, 1, -1.0),
      Op::Edge(0, 1, -1.0),
      Op::Edge(1, 3, 2.0),
      Op::Edge(0, 1, 1.0),
    ],
  );
}

#[test]
fn repeated_nodes_in_both_segments_and_edges_into_egos() {
  check(
    "first-visit index across repeated repairs",
    5,
    &[
      (0, 1, 1.0),
      (1, 0, 1.0),
      (1, 2, -1.0),
      (2, 0, 1.0),
      (2, 3, 1.0),
      (3, 1, 1.0),
    ],
    &[0, 1, 2],
    &[
      Op::Edge(1, 4, -1.0),
      Op::Edge(4, 0, 1.0),
      Op::Edge(2, 0, 0.0),
      Op::Edge(2, 0, 2.0),
      Op::Edge(1, 2, 0.0),
      Op::Edge(1, 2, -2.0),
      Op::Edge(1, 4, 0.0),
      Op::Edge(3, 1, -1.0),
    ],
  );
}

#[test]
fn signed_dead_ends_disappear_and_reappear() {
  check(
    "negative-only outedges are a dead end in negative mode",
    4,
    &[(0, 1, -1.0), (1, 2, -1.0)],
    &[0, 1],
    &[
      Op::Edge(1, 3, 1.0),
      Op::Edge(1, 3, 0.0),
      Op::Edge(1, 2, 1.0),
      Op::Edge(1, 2, -1.0),
      Op::Edge(2, 0, 1.0),
      Op::Edge(1, 3, 2.0),
    ],
  );
}

#[test]
fn calculate_new_ego_between_signed_edits() {
  check(
    "new ego between signed edits",
    4,
    &[(0, 1, -1.0), (1, 2, 1.0), (2, 0, 1.0)],
    &[0],
    &[
      Op::Edge(1, 3, 2.0),
      Op::Calc(2),
      Op::Edge(2, 3, -1.0),
      Op::Edge(1, 2, 0.0),
    ],
  );
}

#[test]
fn epsilon_boundary_on_existing_edges() {
  check(
    "epsilon deletes existing edges; above epsilon is sampleable",
    3,
    &[(0, 1, 2e-6), (0, 2, -2e-6)],
    &[0],
    &[
      Op::Edge(0, 1, 1e-6),
      Op::Edge(0, 1, 1.000001e-6),
      Op::Edge(0, 2, -1e-6),
      Op::Edge(0, 2, -1.000001e-6),
    ],
  );
}

#[test]
fn recalculating_existing_signed_ego_retains_negative_boundary() {
  // Minimal lifecycle reproducer: two nodes, one negative edge, no edit.
  repeated(
    "calculate same signed ego twice",
    2,
    &[(0, 1, -1.0)],
    &[0],
    &[Op::Calc(0)],
  );
}

#[test]
fn sign_flip_repairs_stale_boundary_after_recalculation() {
  // Cutting at position 1 clears even stale markers beyond the old walk end.
  check(
    "recalculate then negative-to-positive flip",
    2,
    &[(0, 1, -1.0)],
    &[0],
    &[Op::Calc(0), Op::Edge(0, 1, 1.0)],
  );
}

#[test]
fn recalculation_suppresses_later_negative_edge() {
  // Old markers force positive-only generation from the ego, suppressing a
  // negative edge even when reached through a positive prefix.
  repeated(
    "recalculate with reachable later negative edge",
    3,
    &[(0, 1, 1.0), (1, 2, -1.0)],
    &[0],
    &[Op::Calc(0)],
  );
}

#[test]
fn recalculation_then_add_return_edge_keeps_bias() {
  // Two-node edit reproducer: stale slots never reach node 1, so adding its
  // return edge cannot repair their suppressed first negative transition.
  repeated(
    "recalculate then add edge into ego",
    2,
    &[(0, 1, -1.0)],
    &[0],
    &[Op::Calc(0), Op::Edge(1, 0, 1.0)],
  );
}

#[test]
fn positive_weight_cancellation_bias_minimal() {
  // 2^54 + 1 rounds to 2^54. Deleting the large edge leaves pos_sum=0
  // although 0->2 of weight 1 remains. The re-addition probability is 1,
  // not 1/2. Only three nodes and ONE public set_edge edit are necessary.
  repeated(
    "positive cached sum cancellation",
    3,
    &[(0, 1, 18_014_398_509_481_984.0), (0, 2, 1.0)],
    &[0],
    &[Op::Edge(0, 1, 1.0)],
  );
}

#[test]
fn negative_weight_cancellation_bias_minimal() {
  repeated(
    "negative cached sum cancellation",
    3,
    &[(0, 1, -18_014_398_509_481_984.0), (0, 2, -1.0)],
    &[0],
    &[Op::Edge(0, 1, -1.0)],
  );
}

#[test]
fn cancellation_in_negative_continuation_positive_only_sum() {
  repeated(
    "positive-only cached sum cancellation",
    4,
    &[(0, 1, -1.0), (1, 2, 18_014_398_509_481_984.0), (1, 3, 1.0)],
    &[0],
    &[Op::Edge(1, 2, 1.0)],
  );
}

#[test]
fn tiny_nonzero_weight_on_absent_edge_is_safe() {
  // A sub-EPSILON assignment should behave like deletion of an absent edge.
  check(
    "absent edge receives tiny weight",
    2,
    &[],
    &[0],
    &[Op::Edge(0, 1, 0.5e-6)],
  );
}

#[test]
fn exact_epsilon_weight_on_absent_edge_is_safe() {
  check(
    "absent edge receives EPSILON",
    2,
    &[],
    &[0],
    &[Op::Edge(0, 1, -1e-6)],
  );
}

#[test]
fn seeded_dense_mixed_sign_edit_fuzzer() {
  let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
  let mut worst = (0.0_f64, String::new());
  // This sweep deliberately excludes calculate(existing ego) and extreme
  // weight ratios, isolating repeated-visit/segment coupling from the
  // separately minimized lifecycle and floating-point regressions.
  for seed in 0..8_u64 {
    let mut rng = StdRng::seed_from_u64(0xade5_0000 + seed);
    let n = 5;
    let weights = [-3.0, -1.0, -0.25, 0.25, 1.0, 3.0];
    let mut edges = Vec::new();
    for s in 0..n {
      for d in 0..n {
        if s != d && rng.random_bool(0.7) {
          edges.push((s, d, weights[rng.random_range(0..weights.len())]));
        }
      }
    }
    let initial = edges.clone();
    let mut inc = build(n, &edges);
    let egos = [0, 2];
    for &ego in &egos {
      inc.calculate(ego).unwrap();
    }
    let mut ops = Vec::new();
    for i in 0..40 {
      let s = rng.random_range(0..n);
      let mut d = rng.random_range(0..n - 1);
      if d >= s {
        d += 1;
      }
      let w = if rng.random_bool(0.25) {
        0.0
      } else {
        weights[rng.random_range(0..weights.len())]
      };
      ops.push(Op::Edge(s, d, w));
      let result = inc.set_edge(s, d, w);
      assert!(
        result.is_ok(),
        "seed={seed} initial={initial:?} ops={ops:?}: {result:?}"
      );
      apply_model(&mut edges, s, d, w);
      if (i + 1) % 10 == 0 {
        let mut fresh = build(n, &edges);
        for &ego in &egos {
          fresh.calculate(ego).unwrap();
        }
        let obs = compare(
          &format!("fuzzer seed={seed} edits={}", i + 1),
          &inc,
          &fresh,
          &egos,
          n,
        );
        if obs.z.abs() > worst.0 {
          worst = (
            obs.z.abs(),
            format!(
              "seed={seed} initial={initial:?} ops={ops:?} worst={obs:?}"
            ),
          );
        }
      }
    }
  }
  println!("FUZZER WORST |z|={:.3}: {}", worst.0, worst.1);
  assert!(worst.0 < Z_FAIL, "{}", worst.1);
}
