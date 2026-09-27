//! "Calculate mode" vs incremental: random dense graphs with walls, random edit sequences
//! (walls of every strength, sign transitions, deletions, trust edits, recalculation and eviction
//! of egos between edits), several egos, discredit on. After every sequence the incrementally
//! maintained frames are compared with frames generated from scratch on the final graph, per ego
//! and node, for credits and blame (|z| < 5 over all comparisons of a case).
//!
//! Default: 12 cases. `--features expensive_tests`: 60.

use meritrank_core::{BlameRadius, Graph, MeritRank, NodeId, Weight};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

const W: usize = 20_000;
const Z_FAIL: f64 = 5.0;
const N: usize = 7;
const EGOS: [NodeId; 3] = [0, 1, 2];

#[cfg(feature = "expensive_tests")]
const CASES: u64 = 60;
#[cfg(not(feature = "expensive_tests"))]
const CASES: u64 = 12;

#[derive(Clone, Copy, Debug)]
enum Op {
  Edge(NodeId, NodeId, Weight),
  Calc(NodeId),
  Evict(NodeId),
}

fn random_weight(rng: &mut StdRng) -> Weight {
  match rng.random_range(0..10) {
    0..=4 => rng.random_range(0.2..3.0),
    5..=7 => -rng.random_range(0.05..1.0), // soft wall
    8 => -1.0,                             // hard wall
    _ => -rng.random_range(1.0..3.0),      // |w| > 1: hard
  }
}

fn case(seed: u64) -> (Vec<(NodeId, NodeId, Weight)>, Vec<Op>, f64, BlameRadius) {
  let mut rng = StdRng::seed_from_u64(seed);
  let mut edges = vec![];
  for s in 0..N {
    for d in 0..N {
      if s != d && rng.random_bool(0.35) {
        edges.push((s, d, random_weight(&mut rng)));
      }
    }
  }
  let mut ops = vec![];
  for _ in 0..30 {
    let op = match rng.random_range(0..20) {
      0 => Op::Calc(EGOS[rng.random_range(0..EGOS.len())]),
      1 => Op::Evict(EGOS[rng.random_range(0..EGOS.len())]),
      2..=4 => {
        let s = rng.random_range(0..N);
        let d = (s + rng.random_range(1..N)) % N;
        Op::Edge(s, d, 0.0)
      },
      _ => {
        let s = rng.random_range(0..N);
        let d = (s + rng.random_range(1..N)) % N;
        Op::Edge(s, d, random_weight(&mut rng))
      },
    };
    ops.push(op);
  }
  let lambda = [0.0, 0.5, 1.0][rng.random_range(0..3)];
  let radius = if rng.random_bool(0.25) {
    BlameRadius::Voucher
  } else {
    BlameRadius::Prefix
  };
  (edges, ops, lambda, radius)
}

fn build(
  edges: &[(NodeId, NodeId, Weight)],
  seed: u64,
  lambda: f64,
  radius: BlameRadius,
) -> MeritRank {
  let mut mr = MeritRank::new(Graph::new(), W);
  mr.reseed(seed);
  mr.discredit = lambda;
  mr.blame_radius = radius;
  for _ in 0..N {
    mr.get_new_nodeid();
  }
  for &(s, d, w) in edges {
    mr.set_edge(s, d, w).unwrap();
  }
  mr
}

fn stats(
  mr: &MeritRank,
  ego: NodeId,
  node: NodeId,
) -> (f64, f64) {
  let c = mr
    .get_personal_hits()
    .get(&ego)
    .map_or(0, |h| h.get_count(&node)) as f64
    / W as f64;
  let b = mr.blame_of(ego, node) / W as f64;
  (c, b)
}

fn z(
  a: f64,
  b: f64,
) -> f64 {
  let p = ((a + b) / 2.0).clamp(0.0, 1.0);
  let se = (2.0 * p * (1.0 - p) / W as f64).sqrt();
  if se == 0.0 {
    if (a - b).abs() < 1e-12 { 0.0 } else { f64::INFINITY }
  } else {
    (a - b) / se
  }
}

#[test]
fn incremental_matches_calculate_mode() {
  let mut worst_overall: (f64, u64, String) = (0.0, 0, String::new());
  for seed in 0..CASES {
    let (edges, ops, lambda, radius) = case(seed);
    let mut inc = build(&edges, 1_000 + seed, lambda, radius);
    for &e in &EGOS {
      inc.calculate(e).unwrap();
    }
    let mut final_edges = edges.clone();
    for op in &ops {
      match *op {
        Op::Edge(s, d, w) => {
          inc.set_edge(s, d, w).unwrap();
          final_edges.retain(|&(s2, d2, _)| !(s2 == s && d2 == d));
          if w != 0.0 {
            final_edges.push((s, d, w));
          }
        },
        Op::Calc(e) => inc.calculate(e).unwrap(),
        Op::Evict(e) => {
          inc.clear_ego(e).unwrap();
          inc.calculate(e).unwrap();
        },
      }
    }
    let mut fresh = build(&final_edges, 2_000 + seed, lambda, radius);
    for &e in &EGOS {
      fresh.calculate(e).unwrap();
    }

    let mut worst: (f64, String) = (0.0, String::new());
    for &e in &EGOS {
      for node in 0..N {
        let (ci, bi) = stats(&inc, e, node);
        let (cf, bf) = stats(&fresh, e, node);
        for (what, a, b) in [("credits", ci, cf), ("blame", bi, bf)] {
          let zz = z(a, b).abs();
          if zz > worst.0 {
            worst = (
              zz,
              format!("ego {e} node {node} {what}: incremental {a:.4} fresh {b:.4}"),
            );
          }
        }
      }
    }
    println!(
      "case {seed}: lambda {lambda} {radius:?}, {} edges, max |z| = {:.2} ({})",
      final_edges.len(),
      worst.0,
      worst.1
    );
    if worst.0 > worst_overall.0 {
      worst_overall = (worst.0, seed, worst.1.clone());
    }
    assert!(
      worst.0 < Z_FAIL,
      "case {seed}: {} (ops {:?})",
      worst.1,
      ops
    );
  }
  println!("worst case {}: |z| = {:.2} ({})", worst_overall.1, worst_overall.0, worst_overall.2);
}
